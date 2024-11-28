use crate::chain::{CHAIN_ID, DENOM, FEE_AMOUNT, GAS_LIMIT};
use crate::client::CosmWasmClient;
use crate::error::{ClientError, Result};
use cosmos_sdk_proto::traits::Message;
use cosmrs::cosmwasm::{MsgExecuteContract, MsgInstantiateContract};
use cosmrs::tx::{BodyBuilder, Fee, Msg, Raw, SignDoc, SignerInfo};
use cosmrs::{Any, Coin, Denom};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cosmwasm_std::Uint128;
use serde::Serialize;
use std::str::FromStr;

#[cw_serde]
pub struct InstantiateMsg {
    /// denom is the denomination of the bridged asset
    pub denom: String,
    /// operators is the list of operators
    pub operators: Vec<Operator>,
}
#[cw_serde]
pub struct Operator {
    /// btc_pk is the BTC PK of the operator
    pub btc_pk: String,
    /// address is the Cosmos address of the operator
    pub address: String,
    // TODO: self-stake
    // TODO: more fields
}

#[cw_serde]
pub enum ExecuteMsg {
    PegIn {
        receiver_address: Addr,
        amount: Uint128,
    },
    PegOut {
        btc_address: String,
        amount: Uint128,
        operator_btc_pk: String,
    },
}

impl CosmWasmClient {
    /// Instantiates a new contract with the given code ID
    pub async fn instantiate(
        &self,
        code_id: u64,
        denom: &str,
        operators: Vec<Operator>,
        label: &str,
    ) -> Result<String> {
        let msg = InstantiateMsg {
            denom: denom.to_string(),
            operators,
        };

        self.initiate_contract(code_id, &msg, label).await
    }

    /// Mints tokens to the specified recipient
    pub async fn peg_in(&self, recipient: &str, amount: u128) -> Result<String> {
        let msg = ExecuteMsg::PegIn {
            receiver_address: Addr::unchecked(recipient),
            amount: Uint128::from(amount),
        };

        self.execute_contract(&msg).await
    }

    /// Burns the specified amount of tokens
    pub async fn peg_out(
        &self,
        btc_address: &str,
        amount: u128,
        operator_btc_pk: &str,
    ) -> Result<String> {
        let msg = ExecuteMsg::PegOut {
            btc_address: btc_address.to_string(),
            amount: Uint128::from(amount),
            operator_btc_pk: operator_btc_pk.to_string(),
        };

        self.execute_contract(&msg).await
    }

    pub async fn initiate_contract<T: Serialize>(
        &self,
        code_id: u64,
        msg: &T,
        label: &str,
    ) -> Result<String> {
        // Serialize contract init message
        let msg_bytes = serde_json::to_vec(&msg).map_err(|e| {
            ClientError::EncodingError(format!("Failed to serialize message: {}", e))
        })?;

        // Create instantiate contract message
        let instantiate_msg = MsgInstantiateContract {
            sender: self.wallet.account_id.clone(),
            admin: Some(self.wallet.account_id.clone()),
            code_id,
            label: Some(label.to_string()),
            msg: msg_bytes,
            funds: vec![],
        };

        self.build_and_broadcast_tx(
            instantiate_msg
                .to_any()
                .map_err(|e| ClientError::Other(e.to_string()))?,
        )
        .await
    }

    /// Build and broadcasts a transaction with the given message
    pub async fn execute_contract<T: Serialize>(&self, msg: &T) -> Result<String> {
        // Serialize contract message
        let msg_bytes = serde_json::to_vec(msg).map_err(|e| {
            ClientError::EncodingError(format!("Failed to serialize message: {}", e))
        })?;

        let contract = self
            .contract
            .clone()
            .ok_or(ClientError::Other("No contract address found".to_string()))?;

        // Create execute contract message
        let execute_msg = MsgExecuteContract {
            sender: self.wallet.account_id.clone(),
            contract: contract,
            msg: msg_bytes,
            funds: vec![],
        };

        self.build_and_broadcast_tx(
            execute_msg
                .to_any()
                .map_err(|e| ClientError::Other(e.to_string()))?,
        )
        .await
    }

    async fn build_and_broadcast_tx<M>(&self, msg: M) -> Result<String>
    where
        M: Message + Into<Any>,
    {
        let tx_raw = self.build_tx(msg).await?;

        let tx_bytes = tx_raw
            .to_bytes()
            .map_err(|e| ClientError::Other(e.to_string()))?;

        let response = self.broadcast_tx(tx_bytes).await?;
        let tx_response = response.tx_response.unwrap();

        if tx_response.code != 0 {
            return Err(ClientError::Other(tx_response.raw_log));
        }

        Ok(tx_response.txhash)
    }

    /// Builds and signs a transaction with the given message
    pub async fn build_tx<M>(&self, msg: M) -> Result<Raw>
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
}
