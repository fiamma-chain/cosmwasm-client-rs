use crate::chain::ACCOUNT_PREFIX;
use crate::error::{ClientError, Result};
use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    tx::{Raw, SignDoc},
    AccountId,
};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub private_key: Vec<u8>,
    pub public_key: PublicKey,
    pub account_id: AccountId,
}

impl Wallet {
    pub fn new(private_key: &str) -> Result<Self> {
        let private_key = hex::decode(private_key)
            .map_err(|e| ClientError::SigningError(format!("Invalid private key hex format: {}", e)))?;

        let signing_key = SigningKey::from_slice(&private_key)
            .map_err(|e| ClientError::SigningError(format!("Invalid private key: {}", e)))?;

        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(ACCOUNT_PREFIX)
            .map_err(|e| ClientError::SigningError(format!("Failed to derive account ID: {}", e)))?;

        Ok(Self {
            private_key,
            public_key,
            account_id,
        })
    }

    pub fn sign(&self, sign_doc: SignDoc) -> Result<Raw> {
        let signing_key = SigningKey::from_slice(&self.private_key)
            .map_err(|e| ClientError::SigningError(format!("Failed to parse signing key: {}", e)))?;
        
        sign_doc
            .sign(&signing_key)
            .map_err(|e| ClientError::SigningError(format!("Signing failed: {}", e)))
    }
}
