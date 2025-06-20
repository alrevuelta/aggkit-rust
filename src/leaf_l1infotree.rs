use crate::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTree;
use crate::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTreeV2;
use alloy::primitives::B256;
use alloy::primitives::FixedBytes;
use alloy::primitives::keccak256;
use alloy::rpc::types::Block;
use alloy::rpc::types::Log;

// This enum is used to enforce an invariant. With it we can guarantee at
// compile time that either the V2Event or the Block is present. Depending
// on which one is present, we will get the timestamp and prev_l1_block_hash.
pub enum L2EventOrBlock {
    V2Event(UpdateL1InfoTreeV2),
    Block(Block),
}

pub struct LeafL1InfoTree {
    pub v1_event: UpdateL1InfoTree,
    pub v2_event_or_block: L2EventOrBlock,
    pub log: Log,
    ger: B256,
}

impl LeafL1InfoTree {
    pub fn new(v1_event: UpdateL1InfoTree, v2_event_or_block: L2EventOrBlock, log: Log) -> Self {
        let ger = Self::to_ger(&v1_event.mainnetExitRoot, &v1_event.rollupExitRoot);
        Self {
            v1_event,
            v2_event_or_block,
            log,
            ger,
        }
    }

    pub fn timestamp(&self) -> u64 {
        match &self.v2_event_or_block {
            L2EventOrBlock::V2Event(v2_event) => v2_event.minTimestamp,
            L2EventOrBlock::Block(block) => block.header.timestamp,
        }
    }

    pub fn prev_l1_block_hash(&self) -> B256 {
        match &self.v2_event_or_block {
            L2EventOrBlock::V2Event(v2_event) => v2_event.blockhash.into(),
            L2EventOrBlock::Block(block) => block.header.parent_hash,
        }
    }

    pub fn leaf_count(&self) -> Option<u32> {
        match &self.v2_event_or_block {
            L2EventOrBlock::V2Event(v2_event) => Some(v2_event.leafCount),
            L2EventOrBlock::Block(_) => None,
        }
    }

    pub fn onchain_info_root(&self) -> Option<FixedBytes<32>> {
        match &self.v2_event_or_block {
            L2EventOrBlock::V2Event(v2_event) => Some(v2_event.currentL1InfoRoot),
            L2EventOrBlock::Block(_) => None,
        }
    }

    pub fn info_root_leaf(&self) -> FixedBytes<32> {
        let mut buf = [0u8; 72];
        buf[0..32].copy_from_slice(self.ger.as_slice());
        buf[32..64].copy_from_slice(self.prev_l1_block_hash().as_slice());
        buf[64..72].copy_from_slice(&self.timestamp().to_be_bytes());
        keccak256(&buf)
    }

    pub fn to_ger(mer: &FixedBytes<32>, rer: &FixedBytes<32>) -> B256 {
        let mut buf = [0u8; 64];
        buf[0..32].copy_from_slice(mer.as_slice());
        buf[32..64].copy_from_slice(rer.as_slice());
        keccak256(&buf)
    }

    pub fn ger(&self) -> B256 {
        self.ger
    }

    pub fn rer(&self) -> B256 {
        self.v1_event.rollupExitRoot
    }

    pub fn mer(&self) -> B256 {
        self.v1_event.mainnetExitRoot
    }

    // TODO: Add getter for the index? Only preset in V2 event
    // but maybe a nice to have to ensure we dont try to insert events out of order.
}
