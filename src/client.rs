use anyhow::Context;
use cosmrs::AccountId;
use prost::Message;
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

use crate::wallet::Wallet;

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
    ) -> anyhow::Result<Self> {
        let (ws_client, driver) = WebSocketClient::new(ws_url)
            .await
            .context("Failed to create WebSocket client")?;

        // Spawn WebSocket driver in a separate task
        tokio::spawn(async move {
            if let Err(e) = driver.run().await {
                tracing::error!("WebSocket client driver error: {}", e);
            }
        });

        let wallet = Wallet::new(private_key)?;

        Ok(Self {
            grpc_url: grpc_url.to_string(),
            ws_client,
            wallet,
            contract,
        })
    }

    pub async fn broadcast_tx(&self, tx_bytes: Vec<u8>) -> anyhow::Result<BroadcastTxResponse> {
        let mut client = ServiceClient::connect(self.grpc_url.clone())
            .await
            .context("Failed to connect to gRPC service")?;

        let request = tonic::Request::new(BroadcastTxRequest {
            tx_bytes,
            mode: BroadcastMode::Sync as i32,
        });

        let response = client
            .broadcast_tx(request)
            .await
            .context("Failed to broadcast transaction")?;

        Ok(response.into_inner())
    }

    pub async fn subscribe_events(&self, query: Query) -> anyhow::Result<Subscription> {
        self.ws_client
            .clone()
            .subscribe(query)
            .await
            .context("Failed to subscribe to events")
    }

    pub async fn get_block_events(&self, height: u64) -> anyhow::Result<Vec<Vec<abci::Event>>> {
        let height = Height::try_from(height).context("Failed to convert height to u64")?;

        let block_results = self
            .ws_client
            .block_results(height)
            .await
            .context("Failed to get block results")?;

        let events = block_results
            .txs_results
            .unwrap_or_default()
            .into_iter()
            .map(|tx_result| tx_result.events)
            .collect();

        Ok(events)
    }

    pub async fn get_account_info(&self, address: String) -> anyhow::Result<BaseAccount> {
        let mut client = QueryClient::connect(self.grpc_url.clone())
            .await
            .context("Failed to connect to gRPC service")?;

        let response = client
            .account(QueryAccountRequest { address })
            .await
            .context("Failed to get account")?;

        let account = response
            .into_inner()
            .account
            .ok_or_else(|| anyhow::anyhow!("No account data found"))?;

        BaseAccount::decode(account.value.as_slice()).context("Failed to decode account info")
    }

    pub async fn get_tx(&self, hash: &str) -> anyhow::Result<GetTxResponse> {
        let mut client = ServiceClient::connect(self.grpc_url.clone())
            .await
            .context("Failed to connect to gRPC service")?;

        let response = client
            .get_tx(GetTxRequest {
                hash: hash.to_string(),
            })
            .await
            .context("Failed to get transaction")?
            .into_inner();

        Ok(response)
    }
}
