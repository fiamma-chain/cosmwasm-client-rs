use anyhow;
use cosmwasm_client_rs::{chain::ChainConfig, CosmWasmClient};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Initialize client with empty contract address (we don't have it yet)
    let rpc_url = "https://grpc-euphrates.devnet.babylonlabs.io:443";
    // get private key from env
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");

    let contract = "bbn18rlp8ewpqsfmd8ur9sp7ml5tzs2d76cc3zafmpdypcuqr4lqx4xss7yc3s";

    let babylon_chain_config = ChainConfig {
        account_prefix: "bbn".to_string(),
        denom: "ubbn".to_string(),
        fee_amount: 10000,
        gas_limit: 1000000,
        chain_id: "euphrates-0.5.0".to_string(),
    };

    let client = CosmWasmClient::new(rpc_url, &private_key, contract, babylon_chain_config)
        .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

    // Test 1: Peg-in some tokens
    // let recipient = "bbn18rlp8ewpqsfmd8ur9sp7ml5tzs2d76cc3zafmpdypcuqr4lqx4xss7yc3s";
    // let amount = 3300000000;
    // println!("Performing peg-in...");

    // let tx_hash = client.peg_in(recipient, amount, "", "", 0, vec![]).await?;
    // println!("Peg-in completed. Tx hash: {}", tx_hash);

    // // Wait for 3 seconds
    // time::sleep(time::Duration::from_secs(10)).await;

    // // // Test 2: Peg-out some tokens
    let btc_address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
    let amount = 3;
    let operator_btc_pk = "test";
    println!("Performing peg-out...");
    let tx_hash = client.peg_out(btc_address, amount, operator_btc_pk).await?;
    println!("Peg-out completed. Tx hash: {}", tx_hash);

    Ok(())
}
