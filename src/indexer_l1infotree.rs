use crate::contracts::PolygonZkEVMGlobalExitRootV2::{UpdateL1InfoTree, UpdateL1InfoTreeV2};
use crate::indexer::EventProcessor;
use crate::leaf_l1infotree::{L2EventOrBlock, LeafL1InfoTree};
use crate::merkle_tree::{MerkleForest, TreeType};
use alloy::primitives::B256;
use alloy::providers::Provider;
use alloy::rpc::types::{BlockNumberOrTag, Log};
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::sync::Arc;

pub struct L1InfoTreeEventProcessor {
    pub tree: Arc<MerkleForest>,
    pub provider: Arc<dyn Provider>,
}

#[async_trait]
impl EventProcessor for L1InfoTreeEventProcessor {
    fn latest_processed_block(&self) -> Result<Option<u64>, eyre::Error> {
        self.tree.get_latest_block(TreeType::L1InfoTree)
    }

    async fn process_events(&self, events: &[Log]) -> Result<(), eyre::Error> {
        let mut v1_events: Vec<Log<UpdateL1InfoTree>> = Vec::new();
        let mut v2_events_by_tx: HashMap<B256, Log<UpdateL1InfoTreeV2>> = HashMap::new();

        // The event V2 was introduced to avoid fetching the full block. But before that when only
        // the V1 event was present, we need to fetch the full block.
        for log in events {
            match log.topic0() {
                Some(&UpdateL1InfoTree::SIGNATURE_HASH) => {
                    let event = log.log_decode::<UpdateL1InfoTree>()?;
                    v1_events.push(event);
                }
                Some(&UpdateL1InfoTreeV2::SIGNATURE_HASH) => {
                    let event = log.log_decode::<UpdateL1InfoTreeV2>()?;
                    v2_events_by_tx.insert(
                        log.transaction_hash
                            .ok_or(eyre::eyre!("Transaction hash is None"))?,
                        event,
                    );
                }
                _ => {}
            }
        }

        let v2_events_by_tx = Arc::new(v2_events_by_tx);

        // TODO: Move this somewhere
        let parallel_fetches = 15;

        // Iterate all V1 events. This event is always present.
        let leaves: Vec<LeafL1InfoTree> = stream::iter(v1_events)
            .map(|v1_event| {
                let provider = Arc::clone(&self.provider);
                let v2_events_by_tx = Arc::clone(&v2_events_by_tx);

                async move {
                    // Check if we already have the V2 counterpart.
                    if let Some(v2_event) = v2_events_by_tx.get(
                        &v1_event
                            .transaction_hash
                            .ok_or(eyre::eyre!("Transaction hash is None"))?,
                    ) {
                        // Found the V2 counterpart, build the leaf directly.
                        let leaf = LeafL1InfoTree::new(
                            v1_event.data().clone(),
                            L2EventOrBlock::V2Event(v2_event.data().clone()),
                            v1_event.reserialize(),
                        );
                        Ok::<LeafL1InfoTree, eyre::Error>(leaf)
                    } else {
                        // No V2 event was found. We need to fetch the full block.
                        let block_number = v1_event
                            .block_number
                            .ok_or(eyre::eyre!("Block number is None"))?;

                        // This is the slow part.
                        let current_block = provider
                            .get_block_by_number(BlockNumberOrTag::Number(block_number))
                            .await?
                            .ok_or(eyre::eyre!("Current block is None"))?;

                        let leaf = LeafL1InfoTree::new(
                            v1_event.data().clone(),
                            L2EventOrBlock::Block(current_block),
                            v1_event.reserialize(),
                        );
                        Ok(leaf)
                    }
                }
            })
            .buffered(parallel_fetches)
            .try_collect()
            .await?;

        // Now that we have constructed all the leaves, we can process them.
        for leaf in leaves {
            self.tree.append_l1info_leaf(
                &leaf.info_root_leaf(),
                leaf.log
                    .block_number
                    .ok_or(eyre::eyre!("Block number is None"))?,
            )?;
        }
        Ok(())
    }
}
