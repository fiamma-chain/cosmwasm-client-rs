use std::str::FromStr;

use crate::chain::{CHAIN_ID, DENOM, FEE_AMOUNT, GAS_LIMIT};
use crate::client::CosmWasmClient;
use crate::error::{ClientError, Result};
use cosmos_sdk_proto::traits::Message;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::{BodyBuilder, Fee, Msg, Raw, SignDoc, SignerInfo};
use cosmrs::{Any, Coin, Denom};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MintMsg {
    pub mint: MintParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MintParams {
    pub recipient: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BurnMsg {
    pub burn: BurnParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BurnParams {
    pub amount: String,
}

impl CosmWasmClient {
    /// Constructs and broadcasts a transaction with the given message
    pub async fn construct_broadcast_tx<T: Serialize>(&self, msg: &T) -> Result<String> {
        // Serialize contract message
        let msg_bytes = serde_json::to_vec(msg).map_err(|e| {
            ClientError::EncodingError(format!("Failed to serialize message: {}", e))
        })?;

        // Create execute contract message
        let execute_msg = MsgExecuteContract {
            sender: self.wallet.account_id.clone(),
            contract: self.contract.clone(),
            msg: msg_bytes,
            funds: vec![],
        };

        // Build and broadcast transaction
        let tx_raw = self
            .build_and_sign_tx(
                execute_msg
                    .to_any()
                    .map_err(|e| ClientError::Other(e.to_string()))?,
            )
            .await?;
        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|e| ClientError::Other(e.to_string()))?;
        let response = self.broadcast_tx(tx_bytes).await?;

        response
            .tx_response
            .ok_or_else(|| ClientError::Other("Missing tx_response".to_string()))
            .map(|resp| resp.txhash)
    }

    /// Builds and signs a transaction with the given message
    async fn build_and_sign_tx<M>(&self, msg: M) -> Result<Raw>
    where
        M: Message + Into<Any>,
    {
        let account = self
            .get_account_info(self.wallet.account_id.to_string())
            .await?;
        let account_number = account.account_number;
        let sequence = account.sequence;

        let chain_id = CHAIN_ID
            .parse()
            .map_err(|e| ClientError::ParseError(format!("Invalid chain ID: {}", e)))?;

        let fee = Coin {
            amount: FEE_AMOUNT,
            denom: Denom::from_str(DENOM)
                .map_err(|e| ClientError::ParseError(format!("Invalid denom: {}", e)))?,
        };
        let fee = Fee::from_amount_and_gas(fee, GAS_LIMIT);

        let tx_body = BodyBuilder::new().msg(msg).finish();
        let auth_info = SignerInfo::single_direct(Some(self.wallet.public_key.clone()), sequence)
            .auth_info(fee);

        let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account_number)
            .map_err(|e| ClientError::SigningError(format!("Failed to create sign doc: {}", e)))?;

        self.wallet.sign(sign_doc)
    }

    /// Mints tokens to the specified recipient
    pub async fn mint(&mut self, recipient: &str, amount: u128) -> Result<String> {
        let msg = MintMsg {
            mint: MintParams {
                recipient: recipient.to_string(),
                amount: amount.to_string(),
            },
        };

        self.construct_broadcast_tx(&msg).await
    }

    /// Burns the specified amount of tokens
    pub async fn burn(&mut self, amount: u128) -> Result<String> {
        let msg = BurnMsg {
            burn: BurnParams {
                amount: amount.to_string(),
            },
        };

        self.construct_broadcast_tx(&msg).await
    }
}
