use crate::chain::ACCOUNT_PREFIX;
use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    tx::{Raw, SignDoc},
    AccountId,
};

#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: Vec<u8>,
    pub public_key: PublicKey,
    pub account_id: AccountId,
}

impl Wallet {
    pub fn new(private_key: &str) -> Self {
        let private_key =
            hex::decode(private_key).expect("private key should be hex format string");
        let signing_key = SigningKey::from_slice(&private_key).expect("Parse signing_key failed");
        let public_key = signing_key.public_key();
        let account_id = public_key
            .account_id(ACCOUNT_PREFIX)
            .expect("Obtain account id from public key failed");
        Self {
            private_key,
            public_key,
            account_id,
        }
    }

    pub fn sign(&self, sign_doc: SignDoc) -> crate::error::Result<Raw> {
        let signing_key = SigningKey::from_slice(&self.private_key).map_err(|e| {
            crate::error::ClientError::SigningError(format!("Failed to parse signing key: {}", e))
        })?;
        sign_doc
            .sign(&signing_key)
            .map_err(|e| crate::error::ClientError::SigningError(format!("Signing failed: {}", e)))
    }
}
