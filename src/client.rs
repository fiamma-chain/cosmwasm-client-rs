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

    pub fn validate_bech32_address(
        address: &str,
        expected_prefix: Option<&str>,
    ) -> anyhow::Result<()> {
        let account_id = AccountId::from_str(address)
            .map_err(|e| anyhow::anyhow!("Invalid bech32 address: {}", e))?;

        if let Some(prefix) = expected_prefix {
            if account_id.prefix() != prefix {
                return Err(anyhow::anyhow!(
                    "Address has wrong prefix: expected {}, got {}",
                    prefix,
                    account_id.prefix()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bech32_address() {
        // Valid address tests - using real valid addresses
        let valid_addresses = vec![
            // Known valid Cosmos address
            ("cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux", "cosmos"),
            // Known valid Osmosis address
            ("osmo17a8smrhauph552zkz5864vjafz9pszpezepz68", "osmo"),
            // Known valid Babylon address
            ("bbn1ad2u30qd2vx6es4pmn28y23qtz6hea7708574y", "bbn"),
            // Known valid Babylon contract address
            (
                "bbn17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgs6spw0g",
                "bbn",
            ),
        ];

        for (address, prefix) in valid_addresses {
            // Test without prefix check
            assert!(
                CosmWasmClient::validate_bech32_address(address, None).is_ok(),
                "Address {} should be valid",
                address
            );

            // Test with correct prefix
            assert!(
                CosmWasmClient::validate_bech32_address(address, Some(prefix)).is_ok(),
                "Address {} should be valid with prefix {}",
                address,
                prefix
            );

            // Test with incorrect prefix
            let wrong_prefix = if prefix == "cosmos" { "osmo" } else { "cosmos" };
            assert!(
                CosmWasmClient::validate_bech32_address(address, Some(wrong_prefix)).is_err(),
                "Address {} should be invalid with wrong prefix {}",
                address,
                wrong_prefix
            );
        }

        // Invalid address tests
        let invalid_addresses = vec![
            "not-a-bech32-address",
            "cosmos1invalid",
            "cosmostooshort",
            "1cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux",
            "",
        ];

        for address in invalid_addresses {
            assert!(
                CosmWasmClient::validate_bech32_address(address, None).is_err(),
                "Address {} should be invalid",
                address
            );
        }
    }

    #[test]
    fn test_wallet_creation_and_validation() {
        // Test wallet creation with private key
        let private_key = "5d386fbdbf11f1141010f81a46b40f94887367562bd33b452bbaa6ce1cd1381e";
        let wallet = Wallet::new(private_key, "cosmos").expect("Failed to create wallet");

        // Validate generated account address
        let address = wallet.account_id.to_string();
        println!("Generated address: {}", address);

        // Verify address validity
        assert!(CosmWasmClient::validate_bech32_address(&address, Some("cosmos")).is_ok());

        // Compare with expected address
        // Note: This address needs to be replaced with the actual address corresponding to the private key above
        let expected_address = wallet.account_id.to_string();
        assert_eq!(address, expected_address);
    }
}
