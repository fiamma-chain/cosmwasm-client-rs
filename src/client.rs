use std::str::FromStr;

use crate::generated::babylon::btclightclient;
use anyhow::Context;
use cosmos_sdk_proto::cosmos::{
    auth::v1beta1::{query_client::QueryClient, BaseAccount, QueryAccountRequest},
    tx::v1beta1::{
        service_client::ServiceClient, BroadcastMode, BroadcastTxRequest, BroadcastTxResponse,
        GetTxRequest, GetTxResponse,
    },
};
use cosmrs::AccountId;

use crate::chain::ChainConfig;
use crate::wallet::Wallet;

#[derive(Clone)]
pub struct CosmWasmClient {
    grpc_url: String,
    pub wallet: Wallet,
    pub contract: Option<AccountId>,
    pub config: ChainConfig,
}

impl CosmWasmClient {
    pub fn new(
        grpc_url: &str,
        private_key: &str,
        contract: &str,
        config: ChainConfig,
    ) -> anyhow::Result<Self> {
        let wallet = Wallet::new(private_key, &config.account_prefix)?;
        let contract = AccountId::from_str(contract).map_err(|e| anyhow::anyhow!(e));

        Ok(Self {
            grpc_url: grpc_url.to_string(),
            wallet,
            contract: Some(contract?),
            config,
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

    pub async fn get_account_info(&self, address: String) -> anyhow::Result<BaseAccount> {
        let mut client = QueryClient::connect(self.grpc_url.clone())
            .await
            .context("Failed to connect to gRPC service")?;

        let resp = client
            .account(QueryAccountRequest { address })
            .await
            .context("Failed to query account information")?;

        let account_info = resp
            .get_ref()
            .clone()
            .account
            .ok_or_else(|| anyhow::anyhow!("No account data found"))?;

        let account = account_info
            .to_msg::<BaseAccount>()
            .context("Failed to convert account info to BaseAccount")?;

        Ok(account)
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

    pub async fn query_header_contains(&self, block_hash: &str) -> anyhow::Result<bool> {
        let mut client =
            btclightclient::v1::query_client::QueryClient::connect(self.grpc_url.clone())
                .await
                .context("Failed to connect to gRPC service")?;
        let mut hash_bytes =
            hex::decode(block_hash).context("Failed to decode block hash from hex")?;
        hash_bytes.reverse();

        let resp = client
            .contains_bytes(btclightclient::v1::QueryContainsBytesRequest { hash: hash_bytes })
            .await
            .context("Failed to query header contains")?;

        Ok(resp.into_inner().contains)
    }
}
