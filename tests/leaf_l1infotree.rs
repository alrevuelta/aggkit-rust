#[cfg(test)]
mod leaf_tests {
    use aggkit_rust::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTree;
    use aggkit_rust::contracts::PolygonZkEVMGlobalExitRootV2::UpdateL1InfoTreeV2;
    use aggkit_rust::leaf_l1infotree::L2EventOrBlock;
    use aggkit_rust::leaf_l1infotree::LeafL1InfoTree;
    use alloy::hex::FromHex;
    use alloy::primitives::B256;
    use alloy::primitives::FixedBytes;
    use alloy::rpc::types::Log;

    // Util macro to avoid boilerplate code
    macro_rules! hex {
        ($hex_str:expr) => {
            FixedBytes::from_hex($hex_str).unwrap()
        };
    }

    #[test]
    fn test_to_ger() {
        // Tests the GER is calculated correctly from the MER and RER
        let root = LeafL1InfoTree::to_ger(
            &hex!("0xd2ee691debbb8a5cf4ccab16bd80fab5063c415e605576ede10d587dcdf98edf"),
            &hex!("0x21251408a11f26cb9b1bcec3155b857073b8c270ea72b4d377680fe831e50047"),
        );

        assert_eq!(
            root,
            hex!("0x42e89bec7b54efea505793e0ce21fb405c3ea3e7d9cc5e725e659b75e421b49b")
        );
    }
    #[test]
    fn test_info_root_leaf() {
        // Tests that the L1 info root leaf is calculated correctly using the GER, previous
        // block hash and timestamp.
        let leaf = LeafL1InfoTree::new(
            UpdateL1InfoTree {
                mainnetExitRoot: hex!(
                    "0x0af758850c3a010370afa0f780d091a9e72007f43e9f147505ca709e0f7d9b1c"
                ),
                rollupExitRoot: hex!(
                    "0xdbf6a41b961855c5c76e0fa2264fb104706925d2b73f6f5261ded3ff6cb1798f"
                ),
            },
            L2EventOrBlock::V2Event(UpdateL1InfoTreeV2 {
                blockhash: hex!(
                    "0x40ce3a02825dc9bd7aacb530d64071f91d4f50fcad523bd5779d81d535420060"
                )
                .into(),
                leafCount: 1,
                minTimestamp: 1707911747,
                currentL1InfoRoot: B256::ZERO,
            }),
            Log::default(),
        );

        println!("ger: {:?}", leaf.ger());

        assert_eq!(
            leaf.info_root_leaf(),
            hex!("0x53876e8afa7a663aa40a380be957c481841f080b5a4ac17f0873b64f39cb66f9")
        );
    }
}
