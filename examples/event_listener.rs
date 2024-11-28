use anyhow;
use cosmrs::AccountId;
use cosmwasm_client_rs::{
    events::{ContractEvent, PegInEvent, PegOutEvent},
    CosmWasmClient, EventListener,
};
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Create event channel with sufficient buffer
    let (tx, mut rx) = mpsc::channel::<ContractEvent>(1000);

    // Initialize client
    let grpc_url = "http://localhost:9090";
    let ws_url = "ws://localhost:26657/websocket";
    let contract_address = "fiamma1xsmqvl8lqr2uwl50aetu0572rss9hrza5kddpfj9ky3jq80fv2tsk3g4ux";
    let private_key = "7ae58f95b0f15c999f77488fa0fbebbd4acbe2d12948dcd1729b07ee8f3051e8";

    let contract = AccountId::from_str(contract_address)
        .map_err(|e| anyhow::anyhow!("Failed to parse contract address: {}", e))?;

    let client = CosmWasmClient::new(grpc_url, ws_url, private_key, Some(contract))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create client: {}", e))?;

    // Create and start event listener
    let mut event_listener = EventListener::new(client.clone(), tx, contract_address.to_string());

    // Start event listening in background task
    tokio::spawn(async move {
        if let Err(e) = event_listener.start().await {
            tracing::error!("Error in event listener: {}", e);
        }
    });

    // Process events in main task
    while let Some(event) = rx.recv().await {
        match event {
            ContractEvent::PegIn(PegInEvent {
                block_height,
                msg_index,
                receiver,
                amount,
            }) => {
                tracing::info!(
                    "Received PegIn event block_height: {} msg_index: {} receiver: {} amount: {}",
                    block_height,
                    msg_index,
                    receiver,
                    amount
                );
            }
            ContractEvent::PegOut(PegOutEvent {
                block_height,
                msg_index,
                sender,
                btc_address,
                operator_btc_pk,
                amount,
            }) => {
                tracing::info!(
                    "Received PegOut event block_height: {} msg_index: {} sender: {} btc_address: {} operator_btc_pk: {} amount: {}",
                    block_height,
                    msg_index,
                    sender,
                    btc_address,
                    operator_btc_pk,
                    amount
                );
            }
        }
    }

    Ok(())
}
