use crate::contracts::PolygonRollupManager::AddExistingRollup;
use crate::contracts::PolygonRollupManager::CreateNewRollup;
use crate::contracts::PolygonRollupManager::VerifyBatchesTrustedAggregator;
use crate::contracts::PolygonRollupManager::VerifyPessimisticStateTransition;
use crate::contracts::PolygonRollupManagerOld::AddExistingRollup as AddExistingRollupOld;
use crate::indexer::EventProcessor;
use crate::merkle_tree::MerkleForest;
use crate::merkle_tree::TreeType;
use alloy::primitives::B256;
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;
use async_trait::async_trait;
use eyre::Result;
use std::sync::Arc;

pub struct RollupManagerEventProcessor {
    pub tree: Arc<MerkleForest>,
}

#[async_trait]
impl EventProcessor for RollupManagerEventProcessor {
    fn latest_processed_block(&self) -> Result<Option<u64>, eyre::Error> {
        self.tree.get_latest_block(TreeType::RollupExitTree)
    }

    // TODO: Maybe not the most efficient thing. The exit root keep changing constantly
    // so when indexing, you are continuisly overriding the prev leaf.
    async fn process_events(&self, events: &[Log]) -> Result<(), eyre::Error> {
        for event in events {
            match event.topic0() {
                Some(&CreateNewRollup::SIGNATURE_HASH) => {
                    let event = event.log_decode::<CreateNewRollup>()?;
                    self.tree.set_rollup_leaf(
                        event.data().rollupID,
                        &B256::default(),
                        event
                            .block_number
                            .ok_or(eyre::eyre!("Block number not found"))?,
                    )?;
                }
                Some(&AddExistingRollup::SIGNATURE_HASH) => {
                    let event = event.log_decode::<AddExistingRollup>()?;
                    self.tree.set_rollup_leaf(
                        event.data().rollupID,
                        &B256::default(),
                        event
                            .block_number
                            .ok_or(eyre::eyre!("Block number not found"))?,
                    )?;
                }
                Some(&AddExistingRollupOld::SIGNATURE_HASH) => {
                    let event = event.log_decode::<AddExistingRollupOld>()?;
                    self.tree.set_rollup_leaf(
                        event.data().rollupID,
                        &B256::default(),
                        event
                            .block_number
                            .ok_or(eyre::eyre!("Block number not found"))?,
                    )?;
                }
                Some(&VerifyBatchesTrustedAggregator::SIGNATURE_HASH) => {
                    let event = event.log_decode::<VerifyBatchesTrustedAggregator>()?;
                    self.tree.set_rollup_leaf(
                        event.data().rollupID,
                        &event.data().exitRoot,
                        event
                            .block_number
                            .ok_or(eyre::eyre!("Block number not found"))?,
                    )?;
                }
                Some(&VerifyPessimisticStateTransition::SIGNATURE_HASH) => {}
                _ => {}
            }
        }
        Ok(())
    }
}
