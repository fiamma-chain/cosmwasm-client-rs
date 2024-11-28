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
            tracing::info!("Received latest height: {}", latest_height);
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
        tracing::info!("Parsing event: {:?}", event);
        if event.kind != "wasm" {
            tracing::info!("Ignoring non-wasm event: {:?}", event);
            return Ok(None);
        }

        let mut token_event = TokenEvent::default();

        // scan attributes
        let mut contract_address = String::new();
        let mut data = String::new();
        let mut hash = String::new();

        for attr in &event.attributes {
            if let (Ok(key), Ok(value)) = (attr.key_str(), attr.value_str()) {
                match key {
                    "_contract_address" => {
                        contract_address = value.to_string();
                        tracing::info!("Contract address: {}", value);
                    }
                    "data" => {
                        data = value.to_string();
                        tracing::info!("Data: {}", value);
                    }
                    "hash_string" => {
                        hash = value.to_string();
                        tracing::info!("Hash: {}", value);
                    }
                    _ => {
                        tracing::info!("Ignoring attribute: {}={}", key, value);
                    }
                }
            }
        }

        // if data is not empty, it's a token transfer event
        if !data.is_empty() {
            token_event.event_type = "token_transfer".to_string(); // 或其他适当的事件类型
            token_event.amount = data; // data字段可能需要解码，具体取决于合约的实现
            token_event.from = Some(contract_address.clone());
            // token_event.to = Some(contract_address.clone());

            tracing::info!("Parsed token event: {:?}", token_event);
            return Ok(Some(token_event));
        }

        tracing::info!("No valid token event data found");
        Ok(None)
    }
}
