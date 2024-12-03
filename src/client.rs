use std::str::FromStr;

use anyhow::Context;
use cosmrs::AccountId;
use prost::Message;

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
    pub wallet: Wallet,
    pub contract: Option<AccountId>,
}

impl CosmWasmClient {
    pub async fn new(
        grpc_url: &str,
        private_key: &str,
        contract: &str,
    ) -> anyhow::Result<Self> {
        let wallet = Wallet::new(private_key)?;
        let contract = AccountId::from_str(contract).map_err(|e| anyhow::anyhow!(e));

        Ok(Self {
            grpc_url: grpc_url.to_string(),
            wallet,
            contract: Some(contract?),
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
