use anyhow;
use cosmrs::AccountId;
use cosmwasm_client_rs::CosmWasmClient;
use std::str::FromStr;
use tokio::time;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Initialize client with empty contract address (we don't have it yet)
    let grpc_url = "http://localhost:9090";
    let ws_url = "ws://localhost:26657/websocket";
    let private_key = "7ae58f95b0f15c999f77488fa0fbebbd4acbe2d12948dcd1729b07ee8f3051e8";
    // let private_key = "e18716358226488b7e49e3fb8f1af9a2bbbc16be57db54cbd4cd6e17a69f97c7";

    // For initial deployment, we use a dummy contract address
    // let dummy_contract = AccountId::from_str("fiamma13k3wqnp4zcrlwtph6xk7l6feunu5ae2k6pqnaw").unwrap();
    // let client = CosmWasmClient::new(grpc_url, ws_url, private_key, None).await?;

    // // Test 1: Instantiate the contract
    // let code_id = 4; // Replace with your actual code ID
    // let denom = "ufia";

    // let label = "Bitcoin Bridge v1"; // A human readable label for the contract
    // let operators = vec![
    //     Operator {
    //         btc_pk: "02a8513d9931896d5d3afc8063148db75d8851fd1fc41b1098ba2a6a766db563d4".to_string(),
    //         address: "fiamma1ufs3tlq4umljk0qfe8k5ya0x6hpavn897u2cnf".to_string(),
    //     },
    //     Operator {
    //         btc_pk: "0252d7a1e20c3d51531c1c9e8519f3a37874f0c63ebd4f0eb05af73853db5dd68f".to_string(),
    //         address: "fiamma1d7x5er87rldnxh0sa2nuqe9w82w4ddr4qn2tp6".to_string(),
    //     },
    // ];

    // println!("Instantiating contract...");
    // let tx_hash = client.instantiate(code_id, denom, operators, label).await?;
    // println!("Contract instantiated. Tx hash: {}", tx_hash);

    // TODO: Get the contract address from the instantiate event
    // After getting the contract address, create a new client with the correct contract address
    let contract =
        AccountId::from_str("fiamma1xsmqvl8lqr2uwl50aetu0572rss9hrza5kddpfj9ky3jq80fv2tsk3g4ux")
            .map_err(|e| anyhow::anyhow!("Failed to parse contract address: {}", e))?;

    let client = CosmWasmClient::new(grpc_url, ws_url, private_key, Some(contract))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

    // Test 2: Peg-in some tokens
    let recipient = "fiamma1ufs3tlq4umljk0qfe8k5ya0x6hpavn89g6kfpy";
    let amount = 33_000_000;
    println!("Performing peg-in...");
    let tx_hash = client.peg_in(recipient, amount).await?;
    println!("Peg-in completed. Tx hash: {}", tx_hash);

    // Wait for 3 seconds
    time::sleep(time::Duration::from_secs(3)).await;

    // Test 3: Peg-out some tokens
    let btc_address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
    let amount = 3300_000;
    let operator_btc_pk = "02a8513d9931896d5d3afc8063148db75d8851fd1fc41b1098ba2a6a766db563d4";
    println!("Performing peg-out...");
    let tx_hash = client.peg_out(btc_address, amount, operator_btc_pk).await?;
    println!("Peg-out completed. Tx hash: {}", tx_hash);

    Ok(())
}
