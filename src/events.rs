use crate::client::CosmWasmClient;
use crate::error::{ClientError, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tendermint::abci;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::{EventType, Query};
use tokio::sync::{broadcast, mpsc};
use tracing;

const EVENT_PROCESSOR_SIZE: usize = 5000;

/// Represents a token-related event with type, amount, and optional from/to addresses
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegOutEvent {
    pub block_height: u64,
    pub msg_index: u32,
    pub sender: String,
    pub btc_address: String,
    pub operator_btc_pk: String,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegInEvent {
    pub block_height: u64,
    pub msg_index: u32,
    pub receiver: String,
    pub amount: u128,
}

#[derive(Debug, Clone)]
pub enum ContractEvent {
    PegIn(PegInEvent),
    PegOut(PegOutEvent),
}

/// Event listener that processes blockchain events sequentially
pub struct EventListener {
    client: CosmWasmClient,
    latest_height_tx: broadcast::Sender<u64>,
    latest_height_rx: broadcast::Receiver<u64>,
    last_processed_height: u64,
    event_sender: mpsc::Sender<ContractEvent>,
    contract_address: String,
}

impl EventListener {
    /// Creates a new EventListener instance
    pub fn new(
        client: CosmWasmClient,
        event_sender: mpsc::Sender<ContractEvent>,
        contract_address: String,
    ) -> Self {
        let (latest_height_tx, latest_height_rx) = broadcast::channel(EVENT_PROCESSOR_SIZE);
        Self {
            client,
            latest_height_tx,
            latest_height_rx,
            last_processed_height: 0,
            event_sender,
            contract_address,
        }
    }

    /// Starts the event subscription and processing
    pub async fn start(&mut self) -> Result<()> {
        // Start WebSocket subscription in a separate task
        let height_tx = self.latest_height_tx.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::subscribe_to_events(client, height_tx).await {
                tracing::error!("Event subscription error: {}", e);
            }
        });

        // Start block processing
        self.process_blocks_sequentially().await
    }

    /// Subscribe to new block events via WebSocket
    async fn subscribe_to_events(
        client: CosmWasmClient,
        height_tx: broadcast::Sender<u64>,
    ) -> Result<()> {
        tracing::info!("Starting WebSocket subscription for new events");
        let query = Query::from(EventType::NewBlock);
        let mut event_stream = client.subscribe_events(query).await?;

        while let Some(event_result) = event_stream.next().await {
            match event_result {
                Ok(event) => {
                    // Extract height from new block event
                    let height = match &event.data {
                        EventData::NewBlock { block, .. } => block
                            .as_ref()
                            .map(|b| b.header.height.value())
                            .unwrap_or_default(),
                        EventData::LegacyNewBlock { block, .. } => block
                            .as_ref()
                            .map(|b| b.header.height.value())
                            .unwrap_or_default(),
                        _ => 0u64,
                    };

                    if height > 0 {
                        if let Err(e) = height_tx.send(height) {
                            tracing::error!("Failed to send height: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error in event stream: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process blocks sequentially, ensuring order
    async fn process_blocks_sequentially(&mut self) -> Result<()> {
        let mut height_rx = self.latest_height_rx.resubscribe();

        while let Ok(latest_height) = height_rx.recv().await {
            if latest_height <= self.last_processed_height {
                tracing::info!(
                    "Latest height {} is not greater than last processed height {}",
                    latest_height,
                    self.last_processed_height
                );
                continue;
            }
            // Process all blocks from last_processed_height + 1 to latest_height
            for height in self.last_processed_height + 1..=latest_height {
                if let Err(e) = self.process_block(height).await {
                    tracing::error!("Error processing block {}: {}", height, e);
                    continue;
                }
                self.last_processed_height = height;
            }
        }

        Ok(())
    }

    /// Process events in a single block
    async fn process_block(&self, height: u64) -> Result<()> {
        let block_events = self.client.get_block_events(height).await?;
        for events in block_events {
            for event in events {       
                if let Some(contract_event) = self.parse_contract_event(height, &event)? {
                    self.event_sender.send(contract_event).await.map_err(|e| {
                        ClientError::EventError(format!("Failed to send event to channel: {}", e))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Parse blockchain events into ContractEvent
    fn parse_contract_event(&self, block_height: u64, event: &abci::Event) -> Result<Option<ContractEvent>> {
        if event.kind != "wasm" {
            return Ok(None);
        }

        // Convert attributes to a HashMap for easier access
        let attrs: std::collections::HashMap<_, _> = event
            .attributes
            .iter()
            .filter_map(|attr| {
                attr.key_str()
                    .ok()
                    .zip(attr.value_str().ok())
                    .map(|(k, v)| (k, v.to_string()))
            })
            .collect();

        // Check if this is our target contract
        if attrs.get("_contract_address") != Some(&self.contract_address) {
            return Ok(None);
        }

        // Parse amount first as it's common for both events
        let amount = attrs
            .get("amount")
            .ok_or_else(|| ClientError::EventError("Missing amount".to_string()))?
            .parse::<u128>()
            .map_err(|e| ClientError::EventError(format!("Failed to parse amount: {}", e)))?;

        let msg_index = attrs
            .get("msg_index")
            .ok_or_else(|| ClientError::EventError("Missing msg_index".to_string()))?
            .parse::<u32>()
            .map_err(|e| ClientError::EventError(format!("Failed to parse msg_index: {}", e)))?;

        match attrs.get("action").map(String::as_str) {
            Some("peg_in") => {
                let receiver = attrs
                    .get("receiver")
                    .ok_or_else(|| ClientError::EventError("Missing receiver".to_string()))?
                    .clone();
                Ok(Some(ContractEvent::PegIn(PegInEvent { block_height, msg_index, receiver, amount })))
            }
            Some("peg_out") => {
                let sender = attrs
                    .get("sender")
                    .ok_or_else(|| ClientError::EventError("Missing sender".to_string()))?
                    .clone();
                let btc_address = attrs
                    .get("btc_address")
                    .ok_or_else(|| ClientError::EventError("Missing btc_address".to_string()))?
                    .clone();
                let operator_btc_pk = attrs
                    .get("operator_btc_pk")
                    .ok_or_else(|| ClientError::EventError("Missing operator_btc_pk".to_string()))?
                    .clone();
                Ok(Some(ContractEvent::PegOut(PegOutEvent {
                    block_height,
                    msg_index,
                    sender,
                    btc_address,
                    operator_btc_pk,
                    amount,
                })))
            }
            _ => Ok(None),
        }
    }
}
