use crate::chain::ACCOUNT_PREFIX;
use anyhow::Context;
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
    pub fn new(private_key: &str) -> anyhow::Result<Self> {
        let private_key = hex::decode(private_key).context("Invalid private key hex format")?;

        let signing_key = SigningKey::from_slice(&private_key)
            .map_err(|e| anyhow::anyhow!("Failed to parse signing key: {e}"))?;

        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(ACCOUNT_PREFIX)
            .map_err(|e| anyhow::anyhow!("Failed to generate account ID: {e}"))?;

        Ok(Self {
            private_key,
            public_key,
            account_id,
        })
    }

    pub fn sign(&self, sign_doc: SignDoc) -> anyhow::Result<Raw> {
        let signing_key = SigningKey::from_slice(&self.private_key)
            .map_err(|e| anyhow::anyhow!("Failed to parse signing key: {e}"))?;

        sign_doc
            .sign(&signing_key)
            .map_err(|e| anyhow::anyhow!("Failed to sign transaction: {e}"))
    }
}
