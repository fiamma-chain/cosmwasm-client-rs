use anyhow;
use cosmwasm_client_rs::{
    events::{BlockEvents, ContractEvent, PegInEvent, PegOutEvent},
    EventListener,
};
use tokio::sync::mpsc;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    fmt::init();

    // Create event channel with sufficient buffer
    let (tx, mut rx) = mpsc::channel::<BlockEvents>(1000);

    // Initialize event listener
    let ws_url = "ws://localhost:26657/websocket";
    let contract_address = "fiamma1xsmqvl8lqr2uwl50aetu0572rss9hrza5kddpfj9ky3jq80fv2tsk3g4ux";

    let mut event_listener = EventListener::new(
        ws_url,
        tx,
        contract_address,
        0, // Start from block height 0
    )
    .await?;

    // Start event listening in background task
    tokio::spawn(async move {
        if let Err(e) = event_listener.start().await {
            tracing::error!("Error in event listener: {}", e);
        }
    });

    // Process events in main task
    while let Some(block_events) = rx.recv().await {
        tracing::info!("Processing events from block {}", block_events.height);

        for (tx_hash, event) in block_events.events {
            match event {
                ContractEvent::PegIn(PegInEvent {
                    msg_index,
                    receiver,
                    amount,
                }) => {
                    tracing::info!(
                        "Received PegIn event tx_hash: {} msg_index: {} receiver: {} amount: {}",
                        tx_hash,
                        msg_index,
                        receiver,
                        amount
                    );
                }
                ContractEvent::PegOut(PegOutEvent {
                    msg_index,
                    sender,
                    btc_address,
                    operator_btc_pk,
                    amount,
                }) => {
                    tracing::info!(
                        "Received PegOut event tx_hash: {} msg_index: {} sender: {} btc_address: {} operator_btc_pk: {} amount: {}",
                        tx_hash,
                        msg_index,
                        sender,
                        btc_address,
                        operator_btc_pk,
                        amount
                    );
                }
            }
        }
    }

    Ok(())
}
