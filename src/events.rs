use crate::client::CosmWasmClient;
use crate::error::{ClientError, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tendermint::abci;
use tendermint::block::Height;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::{EventType, Query};
use tokio::sync::{broadcast, mpsc};
use tracing;

const EVENT_PROCESSOR_SIZE: usize = 5000;

/// Represents a token-related event with type, amount, and optional from/to addresses
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TokenEvent {
    pub event_type: String,
    pub amount: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Event listener that processes blockchain events sequentially
pub struct EventListener {
    client: CosmWasmClient,
    latest_height_tx: broadcast::Sender<u64>,
    latest_height_rx: broadcast::Receiver<u64>,
    last_processed_height: u64,
    event_sender: mpsc::Sender<TokenEvent>,
}

impl EventListener {
    /// Creates a new EventListener instance
    pub fn new(client: CosmWasmClient, event_sender: mpsc::Sender<TokenEvent>) -> Self {
        let (latest_height_tx, latest_height_rx) = broadcast::channel(EVENT_PROCESSOR_SIZE);
        Self {
            client,
            latest_height_tx,
            latest_height_rx,
            last_processed_height: 0,
            event_sender,
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
                continue;
            }

            tracing::debug!(
                "Processing blocks from {} to {}",
                self.last_processed_height + 1,
                latest_height
            );

            // Process all blocks from last_processed_height + 1 to latest_height
            for height in self.last_processed_height + 1..=latest_height {
                if let Err(e) = self.process_block(height).await {
                    tracing::error!("Error processing block {}: {}", height, e);
                    continue;
                }
                self.last_processed_height = height;
            }

            tracing::info!(
                "Processed blocks up to height {}",
                self.last_processed_height
            );
        }

        Ok(())
    }

    /// Process events in a single block
    async fn process_block(&self, height: u64) -> Result<()> {
        let height = Height::try_from(height).expect("Failed to convert height to u64");
        let block_txs = self.client.get_block_txs(height).await?;

        for tx in block_txs {
            for event in tx.events {
                if let Some(token_event) = self.parse_token_event(&event)? {
                    self.event_sender.send(token_event).await.map_err(|e| {
                        ClientError::EventError(format!("Failed to send event to channel: {}", e))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Parse a blockchain event into a TokenEvent
    fn parse_token_event(&self, event: &abci::Event) -> Result<Option<TokenEvent>> {
        if event.kind != "wasm" {
            return Ok(None);
        }

        let mut token_event = TokenEvent::default();
        for attr in &event.attributes {
            if let (Ok(key), Ok(value)) = (attr.key_str(), attr.value_str()) {
                match key {
                    "action" => token_event.event_type = value.to_string(),
                    "amount" => token_event.amount = value.to_string(),
                    "from" => token_event.from = Some(value.to_string()),
                    "to" => token_event.to = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(Some(token_event))
    }
}
