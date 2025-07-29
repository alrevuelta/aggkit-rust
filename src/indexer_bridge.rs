use crate::contracts::PolygonZkEVMBridgeV2::{BridgeEvent, ClaimEvent, NewWrappedToken};
use crate::db::Database;
use crate::indexer::EventProcessor;
use crate::leaf_bridge::LeafBridge;
use crate::merkle_tree::{MerkleForest, TreeType};
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use eyre::Result;

use std::sync::Arc;

pub struct BridgeEventProcessor {
    pub tree: Arc<MerkleForest>,
    pub aggchain_id: u32,
    pub db: Database,
}

#[async_trait]
impl EventProcessor for BridgeEventProcessor {
    fn latest_processed_block(&self) -> Result<Option<u64>, eyre::Error> {
        self.tree
            .get_latest_block(TreeType::LocalExitTree(self.aggchain_id))
    }

    async fn process_events(&self, events: &[Log]) -> Result<(), eyre::Error> {
        for event in events {
            match event.topic0() {
                Some(&BridgeEvent::SIGNATURE_HASH) => {
                    let event = event.log_decode::<BridgeEvent>()?;
                    let leaf_bridge = LeafBridge::new(event.data().clone());

                    // TODO: Ugly assumption. We assume bridge exits here are inserted
                    // atomically in the tree (key-value) and in the sqlite database. The one ruling
                    // up to which block we processed, is the key-value store. Fix this.

                    // TODO: Ensure the block number is updated only when there are no more events from that block.
                    // TODO: Process in batches instead of one by one.
                    self.tree.append_events(
                        self.aggchain_id,
                        &[leaf_bridge],
                        event
                            .block_number
                            .ok_or(eyre::eyre!("Block number is None"))?,
                    )?;

                    // TODO: Do batch inserts as well.
                    self.db
                        .insert_bridge_event(&event, self.aggchain_id)
                        .await?;
                }
                Some(&ClaimEvent::SIGNATURE_HASH) => {
                    let event = event.log_decode::<ClaimEvent>()?;
                    // TODO:
                }
                Some(&NewWrappedToken::SIGNATURE_HASH) => {
                    let event = event.log_decode::<NewWrappedToken>()?;
                    // TODO:
                }

                _ => {}
            }
        }
        // TODO: Sovereign chain events are missing.
        // -SetSovereignTokenAddress
        // -MigrateLegacyToken
        // -RemoveLegacySovereignTokenAddress
        // -SetSovereignWETHAddress
        // -UpdatedClaimedGlobalIndexHashChain
        // -UpdatedUnsetGlobalIndexHashChain
        // -SetInitialLocalBalanceTreeAmount
        Ok(())
    }
}
