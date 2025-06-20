use crate::contracts::PolygonZkEVMBridgeV2::{
    BridgeEvent, ClaimEvent, EmergencyStateActivated, EmergencyStateDeactivated, Initialized,
    NewWrappedToken,
};
use alloy::primitives::B256;
use alloy::primitives::Uint;
use alloy::primitives::keccak256;

#[derive(Clone)]
pub struct LeafBridge {
    // TODO: Not the most efficient thing. Will involve a lot of copies.
    pub bridge_event: BridgeEvent,
}

impl LeafBridge {
    pub fn new(bridge_event: BridgeEvent) -> Self {
        Self { bridge_event }
    }

    pub fn hashed_leaf(&self) -> B256 {
        let mut buf = [0u8; 113];
        let amount_be = Uint::<256, 4>::from(self.bridge_event.amount).to_be_bytes::<32>();
        let metadata_hash = keccak256(self.bridge_event.metadata.as_ref());

        buf[0] = self.bridge_event.leafType;
        buf[1..5].copy_from_slice(&self.bridge_event.originNetwork.to_be_bytes());
        buf[5..25].copy_from_slice(self.bridge_event.originAddress.as_slice());
        buf[25..29].copy_from_slice(&self.bridge_event.destinationNetwork.to_be_bytes());
        buf[29..49].copy_from_slice(self.bridge_event.destinationAddress.as_slice());
        buf[49..81].copy_from_slice(&amount_be);
        buf[81..113].copy_from_slice(metadata_hash.as_slice());

        keccak256(&buf)
    }
}
