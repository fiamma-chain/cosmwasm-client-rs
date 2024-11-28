use cosmrs::AccountId;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tendermint::abci;
use tendermint::block::Height;
use tendermint_rpc::Subscription;
use tendermint_rpc::{query::Query, Client, SubscriptionClient, WebSocketClient};
use tokio::sync::RwLock;

use cosmos_sdk_proto::cosmos::{
    auth::v1beta1::{query_client::QueryClient, BaseAccount, QueryAccountRequest},
    tx::v1beta1::{
        service_client::ServiceClient, BroadcastMode, BroadcastTxRequest, BroadcastTxResponse,
        GetTxRequest, GetTxResponse,
    },
};

use crate::{
    error::{ClientError, Result},
    wallet::Wallet,
};

#[derive(Clone)]
pub struct CosmWasmClient {
    grpc_channel: tonic::transport::Channel,
    ws_client: Arc<RwLock<Option<WebSocketClient>>>,
    pub wallet: Wallet,
    pub contract: Option<AccountId>,
}

impl CosmWasmClient {
    pub async fn new(
        grpc_url: &str,
        ws_url: &str,
        private_key: &str,
        contract: Option<AccountId>,
    ) -> Result<Self> {
        let grpc_channel = tonic::transport::Channel::from_shared(grpc_url.to_string())
            .map_err(|e| ClientError::ConfigError(format!("Invalid gRPC URL: {}", e)))?
            .connect()
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to connect to gRPC: {}", e)))?;

        let (ws_client, driver) = WebSocketClient::new(ws_url).await.map_err(|e| {
            ClientError::WebSocketError(format!("Failed to create WebSocket client: {}", e))
        })?;

        tokio::spawn(async move {
            if let Err(e) = driver.run().await {
                tracing::error!("WebSocket client driver error: {}", e);
            }
        });
        let wallet = Wallet::new(private_key);

        Ok(Self {
            grpc_channel,
            ws_client: Arc::new(RwLock::new(Some(ws_client))),
            wallet,
            contract,
        })
    }

    pub async fn broadcast_tx(&self, tx_bytes: Vec<u8>) -> Result<BroadcastTxResponse> {
        let mut client = ServiceClient::new(self.grpc_channel.clone());

        let request = tonic::Request::new(BroadcastTxRequest {
            tx_bytes,
            mode: BroadcastMode::Sync as i32,
        });

        let response = client.broadcast_tx(request).await.map_err(|e| {
            ClientError::GrpcError(format!("Failed to broadcast transaction: {}", e))
        })?;

        Ok(response.into_inner())
    }

    pub async fn subscribe_events(&self, query: Query) -> Result<Subscription> {
        let mut ws_client = self.ws_client.write().await;
        let client = ws_client.as_mut().ok_or_else(|| {
            ClientError::WebSocketError("WebSocket client not initialized".to_string())
        })?;

        let subscription = client
            .subscribe(query)
            .await
            .map_err(|e| ClientError::WebSocketError(format!("Failed to subscribe: {}", e)))?;

        Ok(subscription)
    }

    // pub async fn get_latest_block_height(&self) -> Result<u64> {
    //     let mut ws_client = self.ws_client.write().await;
    //     let client = ws_client.as_mut().ok_or_else(|| {
    //         ClientError::WebSocketError("WebSocket client not initialized".to_string())
    //     })?;

    //     let info = client
    //         .status()
    //         .await
    //         .map_err(|e| ClientError::WebSocketError(format!("Failed to get status: {}", e)))?;

    //     Ok(info.sync_info.latest_block_height.value())
    // }

    pub async fn get_block_txs(&self, height: Height) -> Result<Vec<CustomTxResult>> {
        let mut ws_client = self.ws_client.write().await;
        let client = ws_client.as_mut().ok_or_else(|| {
            ClientError::WebSocketError("WebSocket client not initialized".to_string())
        })?;

        let block_results = client.block_results(height).await.map_err(|e| {
            ClientError::WebSocketError(format!("Failed to get block results: {}", e))
        })?;

        let txs = if let Some(txs_results) = block_results.txs_results {
            txs_results
                .into_iter()
                .map(|tx_result| CustomTxResult {
                    log: tx_result.log,
                    gas_wanted: tx_result.gas_wanted,
                    gas_used: tx_result.gas_used,
                    events: tx_result.events,
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(txs)
    }

    pub async fn get_account_info(&self, address: String) -> Result<BaseAccount> {
        let mut client = QueryClient::new(self.grpc_channel.clone());
        let response = client
            .account(QueryAccountRequest { address })
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to get account: {}", e)))?;

        let account = response
            .into_inner()
            .account
            .ok_or_else(|| ClientError::ParseError("No account data found".to_string()))?;

        BaseAccount::decode(account.value.as_slice())
            .map_err(|e| ClientError::ParseError(format!("Failed to decode account: {}", e)))
    }

    pub async fn get_tx(&self, hash: &str) -> Result<GetTxResponse> {
        let mut client = ServiceClient::new(self.grpc_channel.clone());
        let response = client
            .get_tx(GetTxRequest {
                hash: hash.to_string(),
            })
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to get transaction: {}", e)))?
            .into_inner();

        Ok(response)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContractResponse {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomTxResult {
    pub log: String,
    pub gas_wanted: i64,
    pub gas_used: i64,
    pub events: Vec<abci::Event>,
}
