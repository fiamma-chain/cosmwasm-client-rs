use anyhow::{anyhow, Context, Result};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tendermint::abci;
use tendermint::block::Height;
use tendermint_rpc::{Client, HttpClient};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegInEvent {
    pub msg_index: u32,
    pub receiver: String,
    pub amount: u128,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PegOutEvent {
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

#[derive(Debug)]
pub struct BlockEvents {
    pub height: u64,
    pub events: Vec<(String, ContractEvent)>, // (tx_hash, event)
}

pub struct EventListener {
    rpc_client: HttpClient,
    event_sender: mpsc::Sender<BlockEvents>,
    contract_address: String,
    last_processed_height: u64,
}

impl EventListener {
    pub async fn new(
        rpc_url: &str,
        event_sender: mpsc::Sender<BlockEvents>,
        contract_address: &str,
        last_processed_height: u64,
    ) -> anyhow::Result<Self> {
        let rpc_client = HttpClient::new(rpc_url).context("Failed to create HTTP client")?;

        Ok(Self {
            rpc_client,
            event_sender,
            contract_address: contract_address.to_string(),
            last_processed_height,
        })
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let mut interval_timer = interval(Duration::from_millis(100));

        loop {
            interval_timer.tick().await;

            // get latest block height
            let status = self.rpc_client.status().await?;
            let latest_height = status.sync_info.latest_block_height.value();

            if latest_height <= self.last_processed_height {
                // if already caught up to latest block, use longer interval
                interval_timer = interval(Duration::from_secs(5));
                continue;
            }

            // if lagging behind more blocks, use shorter interval
            if latest_height - self.last_processed_height > 10 {
                interval_timer = interval(Duration::from_millis(100));
            }

            // process all blocks from last processed height to latest height
            for height in (self.last_processed_height + 1)..=latest_height {
                if let Err(e) = self.process_block(height).await {
                    tracing::error!("Error processing block {}: {}", height, e);
                    continue;
                }
                self.last_processed_height = height;
                tracing::debug!("Successfully processed block: {}", height);
            }
        }
    }

    async fn get_block_events(
        &self,
        height: u64,
    ) -> anyhow::Result<Vec<(String, Vec<abci::Event>)>> {
        let height = Height::try_from(height).context("Failed to convert height")?;

        // get block and block results
        let block = self.rpc_client.block(height).await?;
        let block_results = self.rpc_client.block_results(height).await?;

        let mut tx_events = Vec::new();

        if let Some(tx_results) = block_results.txs_results {
            let txs = block.block.data;

            if txs.len() == tx_results.len() {
                for (i, tx) in txs.iter().enumerate() {
                    if let Some(result) = tx_results.get(i) {
                        let tx_hash = calculate_tx_hash(tx);
                        tx_events.push((tx_hash, result.events.clone()));
                    }
                }
            }
        }

        Ok(tx_events)
    }

    async fn process_block(&self, height: u64) -> anyhow::Result<()> {
        tracing::debug!("Processing block at height: {}", height);
        let tx_events = self.get_block_events(height).await?;
        let mut contract_events = Vec::new();

        // Collect all contract events from this block
        for (tx_hash, events) in tx_events {
            for event in events {
                if let Some(contract_event) = self.parse_contract_event(&event)? {
                    contract_events.push((tx_hash.clone(), contract_event));
                }
            }
        }

        // If we have any events, send them as a batch
        if !contract_events.is_empty() {
            let block_events = BlockEvents {
                height,
                events: contract_events,
            };
            self.event_sender
                .send(block_events)
                .await
                .map_err(|e| anyhow!("Failed to send block events to channel: {}", e))?;
        }

        Ok(())
    }

    /// Parse blockchain events into ContractEvent
    fn parse_contract_event(&self, event: &abci::Event) -> Result<Option<ContractEvent>> {
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

        // Skip if not our contract or not a relevant action
        if attrs.get("_contract_address") != Some(&self.contract_address)
            || (attrs.get("action") != Some(&"peg_out".to_string())
                && attrs.get("action") != Some(&"peg_in".to_string()))
        {
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

// Calculate transaction hash
fn calculate_tx_hash(tx: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tx);
    let hash = hasher.finalize();
    let tx_hash = hex::encode(hash);
    tx_hash
}
