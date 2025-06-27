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
    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (checkpoint_tx, mut checkpoint_rx) = mpsc::channel(100);

    // Initialize event listener
    let rpc_url = "https://babylon-testnet-rpc.nodes.guru";
    let contract_address = "bbn17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgs6spw0g";

    let mut event_listener = EventListener::new(
        rpc_url,
        event_tx,
        checkpoint_tx,
        contract_address,
        1329500, // Start from block height 1329500
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
                    fee_rate,
                    operator_btc_pk,
                    amount,
                }) => {
                    tracing::info!(
                        "Received PegOut event tx_hash: {} msg_index: {} sender: {} btc_address: {} fee_rate: {} operator_btc_pk: {} amount: {}",
                        tx_hash,
                        msg_index,
                        sender,
                        btc_address,
                        fee_rate,
                        operator_btc_pk,
                        amount
                    );
                }
            }
        }
    }

    Ok(())
}
