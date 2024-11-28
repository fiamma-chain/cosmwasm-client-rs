use cosmrs::AccountId;
use prost::Message;
use serde::{Deserialize, Serialize};
use tendermint::abci;
use tendermint::block::Height;
use tendermint_rpc::Subscription;
use tendermint_rpc::{query::Query, Client, SubscriptionClient, WebSocketClient};

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
    grpc_url: String,
    ws_client: WebSocketClient,
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
        let (ws_client, driver) = WebSocketClient::new(ws_url).await.map_err(|e| {
            ClientError::WebSocketError(format!("Failed to create WebSocket client: {}", e))
        })?;

        // Spawn WebSocket driver in a separate task
        tokio::spawn(async move {
            if let Err(e) = driver.run().await {
                tracing::error!("WebSocket client driver error: {}", e);
            }
        });

        let wallet = Wallet::new(private_key);

        Ok(Self {
            grpc_url: grpc_url.to_string(),
            ws_client,
            wallet,
            contract,
        })
    }

    pub async fn broadcast_tx(&self, tx_bytes: Vec<u8>) -> Result<BroadcastTxResponse> {
        let mut client = ServiceClient::connect(self.grpc_url.clone())
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to connect: {}", e)))?;

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
        self.ws_client
            .clone()
            .subscribe(query)
            .await
            .map_err(|e| ClientError::WebSocketError(format!("Failed to subscribe: {}", e)))
    }

    pub async fn get_block_events(&self, height: u64) -> Result<Vec<Vec<abci::Event>>> {

        let height = Height::try_from(height).map_err(|e| {
            ClientError::WebSocketError(format!("Failed to convert height to u64: {}", e))
        })?;

        let block_results = self.ws_client.block_results(height).await.map_err(|e| {
            ClientError::WebSocketError(format!("Failed to get block results: {}", e))
        })?;

        let events = block_results
            .txs_results
            .unwrap_or_default()
            .into_iter()
            .map(|tx_result| {
                tx_result.events
            })
            .collect();

        Ok(events)
    }

    pub async fn get_account_info(&self, address: String) -> Result<BaseAccount> {
        let mut client = QueryClient::connect(self.grpc_url.clone())
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to connect: {}", e)))?;

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
        let mut client = ServiceClient::connect(self.grpc_url.clone())
            .await
            .map_err(|e| ClientError::GrpcError(format!("Failed to connect: {}", e)))?;

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
pub struct CustomTxResult {
    pub block_height: u64,
    pub events: Vec<abci::Event>,
}
