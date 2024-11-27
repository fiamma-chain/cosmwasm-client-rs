use cosmrs::AccountId;
use cosmwasm_client_rs::{events::TokenEvent, CosmWasmClient, EventListener, Result};
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    fmt::init();

    // Create event channel with sufficient buffer
    let (tx, mut rx) = mpsc::channel::<TokenEvent>(1000);

    // Initialize client
    let grpc_url = "http://localhost:9090";
    let ws_url = "ws://localhost:26657/websocket";
    let contract_address = "contract-address";

    let private_key = "";

    let contract = AccountId::from_str(contract_address).unwrap();

    let client = CosmWasmClient::new(grpc_url, ws_url, private_key, contract).await?;

    // Create and start event listener
    let mut event_listener = EventListener::new(client.clone(), tx);

    // Start event listening in background task
    tokio::spawn(async move {
        if let Err(e) = event_listener.start().await {
            tracing::error!("Error in event listener: {}", e);
        }
    });

    // Process events in main task
    while let Some(event) = rx.recv().await {
        match event.event_type.as_str() {
            "mint" => {
                tracing::info!(
                    "Mint event: amount={}, to={}",
                    event.amount,
                    event.to.unwrap_or_default()
                );
            }
            "burn" => {
                tracing::info!(
                    "Burn event: amount={}, from={}",
                    event.amount,
                    event.from.unwrap_or_default()
                );
            }
            "transfer" => {
                tracing::info!(
                    "Transfer event: amount={}, from={}, to={}",
                    event.amount,
                    event.from.unwrap_or_default(),
                    event.to.unwrap_or_default()
                );
            }
            _ => {
                tracing::debug!("Unknown event type: {}", event.event_type);
            }
        }
    }

    Ok(())
}
