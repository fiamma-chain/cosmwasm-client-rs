use anyhow;
use cosmwasm_client_rs::{
    events::{ContractEvent, PegInEvent, PegOutEvent},
    EventListener,
};
use tokio::sync::mpsc;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Create event channel with sufficient buffer
    let (tx, mut rx) = mpsc::channel::<ContractEvent>(1000);

    // Initialize WebSocket client and event listener
    let ws_url = "ws://localhost:26657/websocket";
    let contract_address = "fiamma1xsmqvl8lqr2uwl50aetu0572rss9hrza5kddpfj9ky3jq80fv2tsk3g4ux";
    
    let mut event_listener = EventListener::new(
        ws_url,
        tx,
        contract_address.to_string(),
        0, // Start from block height 0
    ).await?;

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
