#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub account_prefix: String,
    pub chain_id: String,
    pub denom: String,
    pub gas_limit: u64,
    pub fee_amount: u128,
}

impl ChainConfig {
    pub fn new(
        account_prefix: String,
        chain_id: String,
        denom: String,
        gas_limit: u64,
        fee_amount: u128,
    ) -> Self {
        Self {
            account_prefix,
            chain_id,
            denom,
            gas_limit,
            fee_amount,
        }
    }
}
