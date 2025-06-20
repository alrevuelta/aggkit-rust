use alloy::primitives::{Address, B256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::rpc::types::{BlockNumberOrTag, Log};
use alloy::transports::http::reqwest::Url;
use alloy::{
    providers::ProviderBuilder, rpc::client::RpcClient, transports::layers::RetryBackoffLayer,
};
use async_trait::async_trait;
use eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::sleep;

// TODO:
// - Handle sigterm gracefully.

#[async_trait]
pub trait EventProcessor: Send + Sync + 'static {
    async fn process_events(&self, events: &[Log]) -> Result<(), eyre::Error>;
    fn latest_processed_block(&self) -> Result<Option<u64>, eyre::Error>;
}

pub struct Indexer<P: EventProcessor + Send + Sync + 'static> {
    name: String,
    provider: Arc<dyn Provider + Send + Sync>,
    contract_address: Address,
    block_range: u64,
    parallel_queries: usize,
    max_queue_size: usize,
    starting_block: u64,
    poll_interval: u64,
    sync_mode: BlockNumberOrTag,
    event_processor: P,
}

impl<P: EventProcessor + Send + Sync + 'static> Indexer<P> {
    pub fn new(
        rpc_url: Url,
        name: String,
        contract_address: Address,
        sync_mode: BlockNumberOrTag,
        event_processor: P,
    ) -> Result<Self, eyre::Report> {
        // TODO: Maybe pass the provider instead of the url to each type?
        let max_retry: u32 = 100;
        let backoff: u64 = 2000;
        let cups: u64 = 100;

        let block_range: u64 = 10_000;
        let parallel_queries: usize = 5;
        let max_queue_size: usize = 100;
        let poll_interval: u64 = 3;

        let latest_processed_block = event_processor.latest_processed_block()?;

        println!(
            "[{}] latest_processed_block: {:?}",
            name, latest_processed_block
        );

        let starting_block = match latest_processed_block {
            Some(block) => block + 1,
            None => 0,
        };

        // Uses middleware to retry on rate limit errors.
        let provider = Arc::new(
            ProviderBuilder::new().connect_client(
                RpcClient::builder()
                    .layer(RetryBackoffLayer::new(max_retry, backoff, cups))
                    .http(rpc_url),
            ),
        );

        Ok(Self {
            name,
            provider,
            contract_address,
            block_range,
            parallel_queries,
            max_queue_size,
            poll_interval,
            starting_block,
            sync_mode,
            event_processor,
        })
    }

    async fn event_producer(
        &self,
        tx: mpsc::Sender<(u64, u64, Vec<Log>)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut processed_to = self.starting_block.saturating_sub(1);
        let contract_address = self.contract_address;

        loop {
            let provider = Arc::clone(&self.provider);
            let finalized_block = provider
                .get_block_by_number(self.sync_mode)
                .await?
                .ok_or(eyre::eyre!("Finalized block is None"))
                .unwrap()
                .header
                .number;
            println!(
                "[{}] {} block: {:?}",
                self.name, self.sync_mode, finalized_block
            );
            let remaining = finalized_block.saturating_sub(processed_to);
            if remaining == 0 {
                sleep(Duration::from_secs(self.poll_interval)).await;
                continue;
            }

            let chunk_starts: Vec<u64> = ((processed_to + 1)..=finalized_block)
                .step_by(self.block_range as usize)
                .collect();

            stream::iter(
                chunk_starts
                    .into_iter()
                    .map(Ok::<u64, Box<dyn Error + Send + Sync>>),
            )
            .try_for_each_concurrent(self.parallel_queries, |chunk_start| {
                let tx = tx.clone();
                let provider = Arc::clone(&provider);
                async move {
                    let chunk_end =
                        std::cmp::min(chunk_start + self.block_range - 1, finalized_block);

                    println!(
                        "[{}] Fetching events for blocks [{:?}-{:?}] contract address: {:?}",
                        self.name, chunk_start, chunk_end, contract_address
                    );
                    let events = provider
                        .get_logs(
                            &Filter::new()
                                .from_block(chunk_start)
                                .to_block(chunk_end)
                                .address(contract_address),
                        )
                        .await?;

                    tx.send((chunk_start, chunk_end, events)).await?;
                    Ok(())
                }
            })
            .await?;

            // All blocks up to `finalized_block` have been queued for processing.
            processed_to = finalized_block;
        }
    }

    async fn event_consumer(
        &self,
        mut rx: mpsc::Receiver<(u64, u64, Vec<Log>)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // `last_processed` tracks the last block that has been fully processed. We
        // initialise it to one block before `starting_block` so that the next
        // expected chunk will start exactly at `starting_block`.
        let mut last_processed = self.starting_block.saturating_sub(1);

        // Buffer keyed by `chunk_start` so we can easily fetch the contiguous
        // range that must come next (i.e. `last_processed + 1`).
        let mut buffer: BTreeMap<u64, (u64, Vec<Log>)> = BTreeMap::new();
        // TODO: confusion between events (whats recevied) and ev (what i will process)
        while let Some((chunk_start, chunk_end, events)) = rx.recv().await {
            // Store the range in the buffer.  The key is the `chunk_start` so we
            // can determine when the next contiguous segment is ready.
            buffer.insert(chunk_start, (chunk_end, events));

            // Try to process as many contiguous chunks as possible.  The next
            // expected chunk must start exactly at `last_processed + 1`.
            while let Some((end, ev)) = buffer.remove(&(last_processed + 1)) {
                println!(
                    "[{}] Processing [{}-{}] events for contract address: {:?} fetched events: {:?}",
                    self.name,
                    last_processed + 1,
                    end,
                    self.contract_address,
                    ev.len()
                );
                // TODO: Maybe add the start and end block chunks.
                self.event_processor.process_events(&ev).await?;
                // Update the cursor so that the next expected start is directly
                // after the `end` we just processed.
                last_processed = end;
            }
        }
        Ok(())
    }

    // Runs the indexer in the background. The function consumes `self` so it can be
    // moved into an `Arc`, allowing the spawned tasks to hold an owned clone that
    // lives for the entire `'static` lifetime required by `tokio::spawn`.
    pub async fn run(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // TODO: handle graceful shutdown.
        // Bounded channel to avoid un-controlled memory growth and to provide
        // back-pressure between producer and consumer.
        let (tx, rx) = mpsc::channel::<(u64, u64, Vec<Log>)>(self.max_queue_size);

        // Move `self` into an `Arc` so we can share it between the two async tasks we are about to spawn.
        let this = Arc::new(self);

        // Spawn producer.
        let producer_handle = {
            let idx = Arc::clone(&this);
            let tx = tx.clone();
            task::spawn(async move { idx.event_producer(tx).await })
        };

        // Spawn consumer.
        let consumer_handle = {
            let idx = Arc::clone(&this);
            task::spawn(async move { idx.event_consumer(rx).await })
        };

        // Await both tasks.
        producer_handle.await??;
        consumer_handle.await??;

        Ok(())
    }
}
