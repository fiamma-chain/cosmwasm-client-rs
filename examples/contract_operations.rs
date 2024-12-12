use anyhow;
use cosmwasm_client_rs::{chain::ChainConfig, CosmWasmClient};
use cosmwasm_std::Uint128;
use dotenv;
use std::path::Path;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Load .env file from examples directory
    if let Err(e) = dotenv::from_path(Path::new("examples/.env")) {
        eprintln!("Failed to load .env file: {}", e);
    }

    // Initialize client with empty contract address (we don't have it yet)
    let rpc_url = "https://grpc-euphrates.devnet.babylonlabs.io:443";
    // get private key from env
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");

    let bridge_contract = "bbn18rlp8ewpqsfmd8ur9sp7ml5tzs2d76cc3zafmpdypcuqr4lqx4xss7yc3s";

    let cw20_contract = "bbn154gczdhr0swgssr44y4uq9r42gvtaacgq6s44unf24ntqx4zsfsq2ywct2";

    let babylon_chain_config = ChainConfig {
        account_prefix: "bbn".to_string(),
        denom: "ubbn".to_string(),
        fee_amount: 10000,
        gas_limit: 1000000,
        chain_id: "euphrates-0.5.0".to_string(),
    };

    let client = CosmWasmClient::new(
        rpc_url,
        &private_key,
        bridge_contract,
        babylon_chain_config.clone(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

    // Test 1: Peg-in some tokens
    // let recipient = "bbn18rlp8ewpqsfmd8ur9sp7ml5tzs2d76cc3zafmpdypcuqr4lqx4xss7yc3s";
    // let amount = 3300000000;
    // println!("Performing peg-in...");

    // let tx_hash = client.peg_in(recipient, amount, "", "", 0, vec![]).await?;
    // println!("Peg-in completed. Tx hash: {}", tx_hash);

    // // Wait for 3 seconds
    // time::sleep(time::Duration::from_secs(10)).await;

    // Before peg-out we need to approve the bridge contract to spend the cw20tokens
    let cw20_client =
        CosmWasmClient::new(rpc_url, &private_key, cw20_contract, babylon_chain_config)
            .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;
    let increase_allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
        spender: bridge_contract.to_string(),
        amount: Uint128::from(200000u128),
        expires: None,
    };

    let tx_hash = cw20_client
        .execute_contract(&increase_allowance_msg)
        .await?;

    println!("Allowance increased. Tx hash: {}", tx_hash);

    // // // // Test 2: Peg-out some tokens
    let btc_address = "bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k";
    let amount = 2000;
    let operator_btc_pk = "1";
    println!("Performing peg-out...");
    let tx_hash = client.peg_out(btc_address, amount, operator_btc_pk).await?;
    println!("Peg-out completed. Tx hash: {}", tx_hash);

    // query cw20 balance todo fix
    // let cw20_client = CosmWasmClient::new(rpc_url, &private_key, cw20_contract, babylon_chain_config)
    //     .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

    // println!("Cw20 balance: {}", cw20_balance);

    Ok(())
}
