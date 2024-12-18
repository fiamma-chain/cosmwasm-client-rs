use std::time::Duration;

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
    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (checkpoint_tx, mut checkpoint_rx) = mpsc::channel(100);

    // Initialize event listener
    let rpc_url = "https://rpc-euphrates.devnet.babylonlabs.io:443";
    let contract_address = "bbn1sdq3gyl9cuad9d3jx8f23yzu0fz0wazlj2t5vgpksxxr2ehlnpgsvvg2qd";

    let mut event_listener = EventListener::new(
        rpc_url,
        event_tx,
        checkpoint_tx,
        contract_address,
        170000, // Start from block height 170000
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = event_listener.start().await {
            tracing::error!("Event listener error: {}", e);
        }
    });

    // Process checkpoint in background task
    tokio::spawn(async move {
        while let Some(height) = checkpoint_rx.recv().await {
            println!("Received checkpoint: {}", height);
        }
    });

    // Process events in main task
    tracing::info!("Starting event processing loop...");
    while let Some(block_events) = event_rx.recv().await {
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
