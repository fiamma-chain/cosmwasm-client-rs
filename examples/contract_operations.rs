use anyhow;
use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    query_client::QueryClient as WasmQueryClient, QuerySmartContractStateRequest,
};
use cosmwasm_client_rs::{chain::ChainConfig, CosmWasmClient};
use cosmwasm_std::{from_json, to_json_binary, Uint128};
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

    let bridge_contract = "bbn1sdq3gyl9cuad9d3jx8f23yzu0fz0wazlj2t5vgpksxxr2ehlnpgsvvg2qd";

    let cw20_contract = "bbn1ts37gqnqn4t555pgqt2hw8c4pge2d2u90ceujja33jeaxsz2c8kqxxdexa";

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
    let recipient = "bbn1zyn8k5d0heyafjz0fx0frrelpr00hesvkhx88q";

    // Test 1: Peg-in some tokens

    // let amount = 100000;
    // let tx_hash = client.peg_in(recipient, amount, "", "", 0, vec![]).await?;
    // println!("Peg-in completed. Tx hash: {}", tx_hash);

    // Before peg-out we need to approve the bridge contract to spend the cw20tokens
    // let cw20_client =
    //     CosmWasmClient::new(rpc_url, &private_key, cw20_contract, babylon_chain_config)
    //         .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;
    // let increase_allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
    //     spender: bridge_contract.to_string(),
    //     amount: Uint128::from(200000u128),
    //     expires: None,
    // };

    // let tx_hash = cw20_client
    //     .execute_contract(&increase_allowance_msg)
    //     .await?;

    // println!("Allowance increased. Tx hash: {}", tx_hash);

    // // // // Test 2: Peg-out some tokens
    // let btc_address = "bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k";
    // let amount = 2000;
    // let operator_btc_pk = "1";
    // println!("Performing peg-out...");
    // let tx_hash = client.peg_out(btc_address, amount, operator_btc_pk).await?;
    // println!("Peg-out completed. Tx hash: {}", tx_hash);

    // query cw20 balance

    let balance = query_cw20_balance(rpc_url, cw20_contract, &recipient).await?;

    println!("Cw20 balance: {:?}", balance);

    Ok(())
}

async fn query_cw20_balance(
    grpc_url: &str,
    contract_address: &str,
    account_address: &str,
) -> anyhow::Result<cw20::BalanceResponse> {
    let mut client = WasmQueryClient::connect(grpc_url.to_string()).await?;

    let balance_query = cw20::Cw20QueryMsg::Balance {
        address: account_address.to_string(),
    };

    let query_msg_binary = to_json_binary(&balance_query)?;

    let request = QuerySmartContractStateRequest {
        address: contract_address.to_string(),
        query_data: query_msg_binary.into(),
    };

    let response = client.smart_contract_state(request).await?;

    from_json(&response.into_inner().data).map_err(Into::into)
}
