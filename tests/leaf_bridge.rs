#[cfg(test)]
mod leaf_tests {
    use aggkit_rust::contracts::PolygonZkEVMBridgeV2::{
        BridgeEvent, ClaimEvent, EmergencyStateActivated, EmergencyStateDeactivated, Initialized,
        NewWrappedToken,
    };
    use aggkit_rust::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTree;
    use aggkit_rust::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTreeV2;
    use aggkit_rust::leaf_bridge::LeafBridge;
    use alloy::hex::FromHex;
    use alloy::primitives::B256;
    use alloy::primitives::Bytes;
    use alloy::primitives::FixedBytes;
    use alloy::primitives::Uint;
    use alloy::primitives::address;
    use alloy::uint;

    // Util macro to avoid boilerplate code
    macro_rules! hex {
        ($hex_str:expr) => {
            FixedBytes::from_hex($hex_str).unwrap()
        };
    }

    #[test]
    fn test_leaf_bridge() {
        // TODO: This is taken from the python specs. unsure if its correct xd
        let leaf_bridge = BridgeEvent {
            leafType: 1,
            originNetwork: 0,
            originAddress: address!("0x1111111111111111111111111111111111111111"),
            destinationNetwork: 2,
            destinationAddress: address!("0x2222222222222222222222222222222222222222"),
            amount: uint!(6_666_666_U256),
            metadata: Bytes::from_static(b"some metadata"),
            depositCount: 1,
        };

        let leaf_bridge = LeafBridge::new(leaf_bridge);
        assert_eq!(
            leaf_bridge.hashed_leaf(),
            hex!("0x350216a4120cc1547aa7dabd5a7f5428f74cf70930efd6f76bee6a36b5e39f34")
        );
    }
}
