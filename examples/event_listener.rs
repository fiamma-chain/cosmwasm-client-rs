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
    let rpc_url = "https://rpc-euphrates.devnet.babylonlabs.io:443";
    let contract_address = "bbn18rlp8ewpqsfmd8ur9sp7ml5tzs2d76cc3zafmpdypcuqr4lqx4xss7yc3s";

    let mut event_listener = EventListener::new(
        rpc_url,
        tx,
        contract_address,
        164000, // Start from block height 0
    )
    .await?;

    // Start event listening in background task
    tokio::spawn(async move {
        tracing::info!("Starting event listener task...");
        if let Err(e) = event_listener.start().await {
            tracing::error!("Event listener error: {}", e);
        }
    });

    // Process events in main task
    tracing::info!("Starting event processing loop...");
    while let Some(block_events) = rx.recv().await {
        tracing::info!(
            "Received block {} with {} events",
            block_events.height,
            block_events.events.len()
        );
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
