use anyhow::{anyhow, Result, Context};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tendermint::abci;
use tendermint::block::Height;
use tendermint_rpc::SubscriptionClient;
use tendermint_rpc::{WebSocketClient, Client, event::EventData};
use tendermint_rpc::query::{EventType, Query};
use tokio::sync::{broadcast, mpsc};
use tracing;

const EVENT_PROCESSOR_SIZE: usize = 1000;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegInEvent {
    pub block_height: u64,
    pub msg_index: u32,
    pub receiver: String,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegOutEvent {
    pub block_height: u64,
    pub msg_index: u32,
    pub sender: String,
    pub btc_address: String,
    pub operator_btc_pk: String,
    pub amount: u128,
}

#[derive(Debug, Clone)]
pub enum ContractEvent {
    PegIn(PegInEvent),
    PegOut(PegOutEvent),
}

pub struct EventListener {
    ws_client: WebSocketClient,
    latest_height_tx: broadcast::Sender<u64>,
    latest_height_rx: broadcast::Receiver<u64>,
    last_processed_height: u64,
    event_sender: mpsc::Sender<ContractEvent>,
    contract_address: String,
}

impl EventListener {
    /// Creates a new EventListener instance
    pub async fn new(
        ws_url: &str,
        event_sender: mpsc::Sender<ContractEvent>,
        contract_address: String,
        last_processed_height: u64,
    ) -> anyhow::Result<Self> {
        let ws_client = Self::connect(ws_url).await?;
        let (latest_height_tx, latest_height_rx) = broadcast::channel(EVENT_PROCESSOR_SIZE);
        
        Ok(Self {
            ws_client,
            latest_height_tx,
            latest_height_rx,
            last_processed_height,
            event_sender,
            contract_address,
        })
    }

    /// Connect to WebSocket endpoint
    async fn connect(ws_url: &str) -> anyhow::Result<WebSocketClient> {
        let (ws_client, driver) = WebSocketClient::new(ws_url)
            .await
            .context("Failed to create WebSocket client")?;

        // Spawn WebSocket driver in a separate task
        tokio::spawn(async move {
            if let Err(e) = driver.run().await {
                tracing::error!("WebSocket client driver error: {}", e);
            }
        });

        Ok(ws_client)
    }

    /// Get events for a specific block height
    pub async fn get_block_events(&self, height: u64) -> anyhow::Result<Vec<Vec<abci::Event>>> {
        let height = Height::try_from(height).context("Failed to convert height to u64")?;

        let block_results = self
            .ws_client
            .block_results(height)
            .await
            .context("Failed to get block results")?;

        let events = block_results
            .txs_results
            .unwrap_or_default()
            .into_iter()
            .map(|tx_result| tx_result.events)
            .collect();

        Ok(events)
    }

    /// Starts the event subscription and processing
    pub async fn start(&mut self) -> anyhow::Result<()> {
        // Start WebSocket subscription in a separate task
        let height_tx = self.latest_height_tx.clone();
        let ws_client = self.ws_client.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::subscribe_to_events(ws_client, height_tx).await {
                tracing::error!("Event subscription error: {}", e);
            }
        });

        // Start block processing
        self.process_blocks_sequentially().await
    }

    /// Subscribe to new block events via WebSocket
    async fn subscribe_to_events(
        ws_client: WebSocketClient,
        height_tx: broadcast::Sender<u64>,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting WebSocket subscription for new events");
        let query = Query::from(EventType::NewBlock);
        let mut event_stream = ws_client.subscribe(query).await?;

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
    async fn process_blocks_sequentially(&mut self) -> anyhow::Result<()> {
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
    async fn process_block(&self, height: u64) -> anyhow::Result<()> {
        let block_events = self.get_block_events(height).await?;
        for events in block_events {
            for event in events {
                if let Some(contract_event) = self.parse_contract_event(height, &event)? {
                    self.event_sender
                        .send(contract_event)
                        .await
                        .map_err(|e| anyhow!("Failed to send event to channel: {}", e))?;
                }
            }
        }

        Ok(())
    }

    /// Parse blockchain events into ContractEvent
    fn parse_contract_event(
        &self,
        block_height: u64,
        event: &abci::Event,
    ) -> Result<Option<ContractEvent>> {
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
            .ok_or_else(|| anyhow!("Missing amount"))?
            .parse::<u128>()
            .map_err(|e| anyhow!("Failed to parse amount: {}", e))?;

        let msg_index = attrs
            .get("msg_index")
            .ok_or_else(|| anyhow!("Missing msg_index"))?
            .parse::<u32>()
            .map_err(|e| anyhow!("Failed to parse msg_index: {}", e))?;

        match attrs.get("action").map(String::as_str) {
            Some("peg_in") => {
                let receiver = attrs
                    .get("receiver")
                    .ok_or_else(|| anyhow!("Missing receiver"))?
                    .clone();
                Ok(Some(ContractEvent::PegIn(PegInEvent {
                    block_height,
                    msg_index,
                    receiver,
                    amount,
                })))
            }
            Some("peg_out") => {
                let sender = attrs
                    .get("sender")
                    .ok_or_else(|| anyhow!("Missing sender"))?
                    .clone();
                let btc_address = attrs
                    .get("btc_address")
                    .ok_or_else(|| anyhow!("Missing btc_address"))?
                    .clone();
                let operator_btc_pk = attrs
                    .get("operator_btc_pk")
                    .ok_or_else(|| anyhow!("Missing operator_btc_pk"))?
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
