#![allow(unused)]
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

    // dev net bridge contract address
    let dev_bridge_contract = "bbn1c9fkszt5lq34vvvlat3fxj6yv7ejtqapz04e97vtc9m5z9cwnamq26r0jx";
    // dev net cw20 contract address
    let dev_cw20_contract = "bbn1ejhxn5vmue89ghk2phq6mlfr7pvsflydull0j8tavszm4fzv8mfqpmv9r7";

    // local net bridge contract address
    let local_bridge_contract = "bbn1sdq3gyl9cuad9d3jx8f23yzu0fz0wazlj2t5vgpksxxr2ehlnpgsvvg2qd";
    // local net cw20 contract address
    let local_cw20_contract = "bbn1ts37gqnqn4t555pgqt2hw8c4pge2d2u90ceujja33jeaxsz2c8kqxxdexa";

    let regtest_btc_receiver_address =
        "bcrt1phcnl4zcl2fu047pv4wx6y058v8u0n02at6lthvm7pcf2wrvjm5tqatn90k";

    let signer_btc_receiver_address =
        "tb1pgx9vzuplwk87w587ekyh4tqecew0gxhttpfqk4jrz6euqgz3xpdsuzdp6g";

    let babylon_chain_config = ChainConfig {
        account_prefix: "bbn".to_string(),
        denom: "ubbn".to_string(),
        fee_amount: 10000,
        gas_limit: 1000000,
        chain_id: "euphrates-0.5.0".to_string(),
    };

    let local_client = CosmWasmClient::new(
        rpc_url,
        &private_key,
        local_bridge_contract,
        babylon_chain_config.clone(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;
    let recipient = "bbn1zyn8k5d0heyafjz0fx0frrelpr00hesvkhx88q";

    // Test 1: Peg-in some tokens
    // let amount = 3000;
    // let pegin_tx = "02000000018e11b41490ade753423c9b293327f17f07fec806054d1558e6e0b07680bb47650400000000ffffffff028813000000000000220020001f05369d0d7ce4712508e9b0f52bce0baab6b0e750059d90e7ba1e52aa433bc509820000000000225120418ac1703f758fe750fecd897aac19c65cf41aeb58520b564316b3c02051305b00000000";
    // let pegin_tx_idx = 73;
    // let sender_btc_pk = "03cb4bf65f02d17a51fe788d196d8c62750e346ae22142f7bb92df010e2f52f81f";
    // let btc_block_hash = "000000205a6c440dc4e8ce93b516b41912d65fa32928885049a5274ba07928e8cd000000a5de2fdb036761620fafeb4cb7b870481586da8f8a836b6bb774514aceb99cb28cba5d679448011ee2783b00";
    // let pegin_tx_merkle_proof = vec![
    //     "acbfbb318c0a988169e3cffa201809a75167c7f55103d830887f7f96ba849c98".to_string(),
    //     "80427ca6ccae930c64e7b0a1a93df88348febe7469e9baf873b57af18c51b3fb".to_string(),
    //     "fbdce76c0e1ee8d8c63b39dfc21ae7504b026193853c4500563a5bce3b26febc".to_string(),
    //     "2a68d1bf8a4a86d08a23503c96e1221d0d29c54cf179ee17c21f88511003a34e".to_string(),
    //     "871fbf2319087c98f5c4a22eb8aa14054a4d9d1ac49a9900d91cebf29e1bed19".to_string(),
    //     "cc03ad7d2edddc5721dd8f40c5457435b36809c45f9756009f639aa4de26b2d3".to_string(),
    //     "2dfc6026eaa436058f4aade6d3cf4962d556ac719d728094bd79d3fbc04bad77".to_string(),
    //     "f5732256ebb464cdbcced32951ade5200329e38e1ecd84dc03915684ffe6fc63".to_string(),
    //     "c734dea8e9cc835c5b27d9a86856aaa0bccb7c91704b6dc9729a5bba401bbcc6".to_string(),
    //     "7fdb69aaaa2081e4fedb2fdb69617c87b8cdb2f4c860bb701429ce1fc9ca7c15".to_string(),
    //     "1d97156d2c29a24cbaa0e3e27ab97cbdde1e8b7f0ac803a1af696267ea92ba7e".to_string(),
    // ];

    // let tx_hash = client
    //     .peg_in(
    //         sender_btc_pk,
    //         recipient,
    //         amount,
    //         btc_block_hash,
    //         pegin_tx,
    //         pegin_tx_idx,
    //         pegin_tx_merkle_proof,
    //     )
    //     .await?;
    // println!("Peg-in completed. Tx hash: {}", tx_hash);

    // Before peg-out we need to approve the bridge contract to spend the cw20tokens
    // let cw20_client =
    //     CosmWasmClient::new(rpc_url, &private_key, cw20_contract, babylon_chain_config)
    //         .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;
    // let increase_allowance_msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
    //     spender: local_bridge_contract.to_string(),
    //     amount: Uint128::from(10000u128),
    //     expires: None,
    // };

    // let tx_hash = cw20_client
    //     .execute_contract(&increase_allowance_msg)
    //     .await?;

    // println!("Allowance increased. Tx hash: {}", tx_hash);

    // // // // Test 2: Peg-out some tokens
    let amount = 5000;
    let operator_btc_pk = "1";
    println!("Performing peg-out...");
    let tx_hash = local_client
        .peg_out(regtest_btc_receiver_address, amount, operator_btc_pk)
        .await?;
    println!("Peg-out completed. Tx hash: {}", tx_hash);

    // query cw20 balance

    let balance = query_cw20_balance(rpc_url, local_cw20_contract, &recipient).await?;

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
