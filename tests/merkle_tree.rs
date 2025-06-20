#[cfg(test)]
mod tests {

    use aggkit_rust::contracts::PolygonZkEVMBridgeV2::{
        BridgeEvent, ClaimEvent, EmergencyStateActivated, EmergencyStateDeactivated, Initialized,
        NewWrappedToken,
    };

    use aggkit_rust::leaf_bridge::LeafBridge;

    use aggkit_rust::merkle_tree::calculate_merkle_root;
    use aggkit_rust::merkle_tree::{MerkleForest, TreeType};
    use alloy::hex::FromHex;
    use alloy::primitives::B256;
    use alloy::primitives::Bytes;
    use alloy::primitives::FixedBytes;
    use alloy::primitives::Uint;
    use alloy::primitives::address;
    use rocksdb::{
        ColumnFamily, ColumnFamilyDescriptor, DB, DBCompressionType, Options, WriteBatch,
    };

    use alloy::uint;

    // Util macro to avoid boilerplate code
    macro_rules! hex {
        ($hex_str:expr) => {
            FixedBytes::from_hex($hex_str).unwrap()
        };
    }

    #[test]
    fn test_playing() {
        let leaves = vec![1, 2, 3, 7];
        if !leaves.windows(2).all(|w| w[0] + 1 == w[1]) {
            println!("not increasing");
        }

        // TODO: test if i can read my own write.
    }

    #[test]
    fn test_merkle_proofs() {
        let _ = std::fs::remove_dir_all("db_test");
        let mut t = MerkleForest::open("db_test").unwrap();
        // TODO:;
    }

    #[test]
    fn test_1() {
        let _ = std::fs::remove_dir_all("db_test");
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // To tune: https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide
        let mut tree_opts = Options::default();
        tree_opts.set_compression_type(DBCompressionType::Zstd); // TODO: tune this

        let mut raw_opts = Options::default();
        raw_opts.set_compression_type(DBCompressionType::Zstd); // TODO: tune this

        let db = DB::open(
            &opts, "db_test",
            //vec![ColumnFamilyDescriptor::new(CF_TREE_LEVELS, tree_opts)],
        )
        .unwrap();

        let mut batch = WriteBatch::default();
        batch.put("a", "3");
        let got_value = db.get("a").unwrap();
        db.write(batch).unwrap();

        println!("gotvalue {:?}", got_value);
    }

    #[test]
    fn test_todo() {
        // TODO: test that one by one it works. and also that both at the same time.
        let _ = std::fs::remove_dir_all("db_test");
        let mut t = MerkleForest::open("db_test").unwrap();

        let leaf1 = LeafBridge::new(BridgeEvent {
            leafType: 1,
            originNetwork: 0,
            originAddress: address!("0x1111111111111111111111111111111111111111"),
            destinationNetwork: 2,
            destinationAddress: address!("0x2222222222222222222222222222222222222222"),
            amount: uint!(6_666_666_U256),
            metadata: Bytes::from_static(b"some metadata"),
            depositCount: 0, // its not really used.
        });

        let leaf2 = LeafBridge::new(BridgeEvent {
            leafType: 1,
            originNetwork: 0,
            originAddress: address!("0x3333333333333333333333333333333333333333"),
            destinationNetwork: 2,
            destinationAddress: address!("0x4444444444444444444444444444444444444444"),
            amount: uint!(888_888_U256),
            metadata: Bytes::from_static(b"more metadata"),
            depositCount: 1, // its not really used.
        });

        t.append_events(1, &[leaf1.clone()], 1).unwrap();
        assert_eq!(
            t.get_root(&TreeType::LocalExitTree(1)).unwrap(),
            Some(hex!(
                "0xda166a2aea3989c951077f31eee9d4535a0ab449b82f3557f7335bc79033d121"
            ))
        );

        t.append_events(1, &[leaf2.clone()], 2).unwrap();
        assert_eq!(
            t.get_root(&TreeType::LocalExitTree(1)).unwrap(),
            Some(hex!(
                "0x75322539144af420787b8e59d9bf5051a8fbd19de9302ddc773c0eba884c54d3"
            ))
        );
    }

    #[test]
    fn test_get_root_empty_tree() -> Result<(), eyre::Error> {
        let _ = std::fs::remove_dir_all("db_test");
        let t = MerkleForest::open("db_test").unwrap();
        println!("root: {:?}", t.get_root(&TreeType::RollupExitTree).unwrap());
        Ok(())
    }

    #[test]
    fn test_todo_testingsometuff() -> Result<(), eyre::Error> {
        let _ = std::fs::remove_dir_all("db_test");
        let mut t = MerkleForest::open("db_test").unwrap();

        let rollup_exit_roots = vec![
            (
                1,
                hex!("0x2eec493df61d778cb2a5d02b73445ea758a64543b540a1b8111d0fb47274221f"),
            ),
            (
                2,
                hex!("0x5e5d1aa128d94a3c164b3f76cb54b02fec1387d247b4f500fc562272c717424d"),
            ),
            (
                3,
                hex!("0xb218b61c22d70f2d59bab9cf4964fe9f7ef73afabb776aa54fba30c97fd89b4b"),
            ),
            (
                4,
                hex!("0x0000000000000000000000000000000000000000000000000000000000000000"),
            ),
            (
                5,
                hex!("0x386c1f907fe8768b23a17cf2ebf449e679ae8ab38d17131cd603d6655cc6770b"),
            ),
            (
                6,
                hex!("0xb52156faceb1557001af394dae8fd5b0de31ce9a321f63e813c115abf9ddadb2"),
            ),
            (
                7,
                hex!("0xac9a61e84eb4347c58a8dc22949ef17a593c0662da75d06936ff124f6aac86b6"),
            ),
            (
                8,
                hex!("0x5afa8f38e955e222e440ee61ca4b8f455c41b542e7f8b0f7d276f2ef0026106d"),
            ),
            (
                9,
                hex!("0x2c22e60e9e5ebcce3885897ecebffec1aaf7e141f1241dee04d4b47f3444d665"),
            ),
            (
                10,
                hex!("0x97998af9b58859d6ec3fa77a0613e504fb8e8f051885e8f3a4a13d3bf5eaedae"),
            ),
            (
                11,
                hex!("0xdb1bef18121aed0ec6da77297897bd2ee34b9da0ba3f3766b1fe9cd7446af5d5"),
            ),
            (
                12,
                hex!("0xdb1bef18121aed0ec6da77297897bd2ee34b9da0ba3f3766b1fe9cd7446af5d5"),
            ),
            (
                13,
                hex!("0xe364a474900343072eb2f4234169079ecdb96660901f63f0eaf7df2d7a34243e"),
            ),
            (
                14,
                hex!("0x78c2c6a7aeec799425ca459a970dd859e663af7fcda42d90bfa59c886f46a5da"),
            ),
            (
                15,
                hex!("0x0000000000000000000000000000000000000000000000000000000000000000"),
            ),
            (
                16,
                hex!("0x05fdb6446f26abaf23892e5da409edeb2a125bec3af72eeb9b2ef037449975c4"),
            ),
            (
                17,
                hex!("0x4e5b0deba6495eff975e8389f220520d6c25363cfd1b55e30cbe3667ebdf545a"),
            ),
            (
                18,
                hex!("0xa3a26da5b9c197a458570a50487c7c44f1df73f5ce4024ef913af55f0102ae55"),
            ),
            (
                19,
                hex!("0xcc7e59609197f085caadb64e436cf248f40f4158324daf334ac8099fd1cbe613"),
            ),
            (
                20,
                hex!("0x656db1fd488f456faa9766f4948fc5d8602371602b66e7874080bcc6621f82bc"),
            ),
        ];

        for (rollup_id, exit_root) in rollup_exit_roots {
            // note that rollup 1 is placed at 0, etc.
            t.set_rollup_leaf(rollup_id, &exit_root, 1).unwrap();
        }
        println!("root: {:?}", t.get_root(&TreeType::RollupExitTree).unwrap());
        assert_eq!(
            t.get_root(&TreeType::RollupExitTree).unwrap(),
            Some(hex!(
                "0xcfc0867b45230182da671566501ec406ec5a27cade099444de6e34562d36ea40"
            ))
        );

        let rollup_id = 1;
        let rer_proof = t
            .merkle_proof(TreeType::RollupExitTree, rollup_id as u64)
            .unwrap();
        for i in rer_proof.iter() {
            println!("i: {:?}", i);
        }

        let expected_hashes = vec![
            hex!("0x5e5d1aa128d94a3c164b3f76cb54b02fec1387d247b4f500fc562272c717424d"),
            hex!("0x108031021fc01678da05870a2a6d7d50b12aaf0f2c2cf095da93ed882a77dd84"),
            hex!("0xd0e306aa24e72a56666e842ef4612037aa4e077a21c4bac9a3dc532d7f22f249"),
            hex!("0x908f89438281b585a80788d31016a8622c3afbba8db6f94f8b4762e9f891e9b7"),
            hex!("0x5fc736a2c94307be78da58ffb63d16ec28b7c5799f52cc38f839324a2f7c0614"),
            hex!("0x0eb01ebfc9ed27500cd4dfc979272d1f0913cc9f66540d7e8005811109e1cf2d"),
            hex!("0x887c22bd8750d34016ac3c66b5ff102dacdd73f6b014e710b51e8022af9a1968"),
            hex!("0xffd70157e48063fc33c97a050f7f640233bf646cc98d9524c6b92bcf3ab56f83"),
            hex!("0x9867cc5f7f196b93bae1e27e6320742445d290f2263827498b54fec539f756af"),
            hex!("0xcefad4e508c098b9a7e1d8feb19955fb02ba9675585078710969d3440f5054e0"),
            hex!("0xf9dc3e7fe016e050eff260334f18a5d4fe391d82092319f5964f2e2eb7c1c3a5"),
            hex!("0xf8b13a49e282f609c317a833fb8d976d11517c571d1221a265d25af778ecf892"),
            hex!("0x3490c6ceeb450aecdc82e28293031d10c7d73bf85e57bf041a97360aa2c5d99c"),
            hex!("0xc1df82d9c4b87413eae2ef048f94b4d3554cea73d92b0f7af96e0271c691e2bb"),
            hex!("0x5c67add7c6caf302256adedf7ab114da0acfe870d449a3a489f781d659e8becc"),
            hex!("0xda7bce9f4e8618b6bd2f4132ce798cdc7a60e7e1460a7299e3c6342a579626d2"),
            hex!("0x2733e50f526ec2fa19a22b31e8ed50f23cd1fdf94c9154ed3a7609a2f1ff981f"),
            hex!("0xe1d3b5c807b281e4683cc6d6315cf95b9ade8641defcb32372f1c126e398ef7a"),
            hex!("0x5a2dce0a8a7f68bb74560f8f71837c2c2ebbcbf7fffb42ae1896f13f7c7479a0"),
            hex!("0xb46a28b6f55540f89444f63de0378e3d121be09e06cc9ded1c20e65876d36aa0"),
            hex!("0xc65e9645644786b620e2dd2ad648ddfcbf4a7e5b1a3a4ecfe7f64667a3f0b7e2"),
            hex!("0xf4418588ed35a2458cffeb39b93d26f18d2ab13bdce6aee58e7b99359ec2dfd9"),
            hex!("0x5a9c16dc00d6ef18b7933a6f8dc65ccb55667138776f7dea101070dc8796e377"),
            hex!("0x4df84f40ae0c8229d0d6069e5c8f39a7c299677a09d367fc7b05e3bc380ee652"),
            hex!("0xcdc72595f74c7b1043d0e1ffbab734648c838dfb0527d971b602bc216c9619ef"),
            hex!("0x0abf5ac974a1ed57f4050aa510dd9c74f508277b39d7973bb2dfccc5eeb0618d"),
            hex!("0xb8cd74046ff337f0a7bf2c8e03e10f642c1886798d71806ab1e888d9e5ee87d0"),
            hex!("0x838c5655cb21c6cb83313b5a631175dff4963772cce9108188b34ac87c81c41e"),
            hex!("0x662ee4dd2dd7b2bc707961b1e646c4047669dcb6584f0d8d770daf5d7e7deb2e"),
            hex!("0x388ab20e2573d171a88108e79d820e98f26c0b84aa8b2f4aa4968dbb818ea322"),
            hex!("0x93237c50ba75ee485f4c22adf2f741400bdf8d6a9cc7df7ecae576221665d735"),
            hex!("0x8448818bb4ae4562849e949e17ac16e0be16688e156b5cf15e098c627c0056a9"),
        ];

        for (i, hash) in rer_proof.iter().enumerate() {
            assert_eq!(
                hash, &expected_hashes[i],
                "Hash at index {} does not match",
                i
            );
        }

        let rollup_id = 3;
        let rer_proof = t
            .merkle_proof(TreeType::RollupExitTree, rollup_id as u64)
            .unwrap();

        let additional_hashes = vec![
            hex!("0x0000000000000000000000000000000000000000000000000000000000000000"),
            hex!("0x8ce8ab8e7a04af77dba02e86078e29084981e7e08d88395863d0b0d3c88ea871"),
            hex!("0xd0e306aa24e72a56666e842ef4612037aa4e077a21c4bac9a3dc532d7f22f249"),
            hex!("0x908f89438281b585a80788d31016a8622c3afbba8db6f94f8b4762e9f891e9b7"),
            hex!("0x5fc736a2c94307be78da58ffb63d16ec28b7c5799f52cc38f839324a2f7c0614"),
            hex!("0x0eb01ebfc9ed27500cd4dfc979272d1f0913cc9f66540d7e8005811109e1cf2d"),
            hex!("0x887c22bd8750d34016ac3c66b5ff102dacdd73f6b014e710b51e8022af9a1968"),
            hex!("0xffd70157e48063fc33c97a050f7f640233bf646cc98d9524c6b92bcf3ab56f83"),
            hex!("0x9867cc5f7f196b93bae1e27e6320742445d290f2263827498b54fec539f756af"),
            hex!("0xcefad4e508c098b9a7e1d8feb19955fb02ba9675585078710969d3440f5054e0"),
            hex!("0xf9dc3e7fe016e050eff260334f18a5d4fe391d82092319f5964f2e2eb7c1c3a5"),
            hex!("0xf8b13a49e282f609c317a833fb8d976d11517c571d1221a265d25af778ecf892"),
            hex!("0x3490c6ceeb450aecdc82e28293031d10c7d73bf85e57bf041a97360aa2c5d99c"),
            hex!("0xc1df82d9c4b87413eae2ef048f94b4d3554cea73d92b0f7af96e0271c691e2bb"),
            hex!("0x5c67add7c6caf302256adedf7ab114da0acfe870d449a3a489f781d659e8becc"),
            hex!("0xda7bce9f4e8618b6bd2f4132ce798cdc7a60e7e1460a7299e3c6342a579626d2"),
            hex!("0x2733e50f526ec2fa19a22b31e8ed50f23cd1fdf94c9154ed3a7609a2f1ff981f"),
            hex!("0xe1d3b5c807b281e4683cc6d6315cf95b9ade8641defcb32372f1c126e398ef7a"),
            hex!("0x5a2dce0a8a7f68bb74560f8f71837c2c2ebbcbf7fffb42ae1896f13f7c7479a0"),
            hex!("0xb46a28b6f55540f89444f63de0378e3d121be09e06cc9ded1c20e65876d36aa0"),
            hex!("0xc65e9645644786b620e2dd2ad648ddfcbf4a7e5b1a3a4ecfe7f64667a3f0b7e2"),
            hex!("0xf4418588ed35a2458cffeb39b93d26f18d2ab13bdce6aee58e7b99359ec2dfd9"),
            hex!("0x5a9c16dc00d6ef18b7933a6f8dc65ccb55667138776f7dea101070dc8796e377"),
            hex!("0x4df84f40ae0c8229d0d6069e5c8f39a7c299677a09d367fc7b05e3bc380ee652"),
            hex!("0xcdc72595f74c7b1043d0e1ffbab734648c838dfb0527d971b602bc216c9619ef"),
            hex!("0x0abf5ac974a1ed57f4050aa510dd9c74f508277b39d7973bb2dfccc5eeb0618d"),
            hex!("0xb8cd74046ff337f0a7bf2c8e03e10f642c1886798d71806ab1e888d9e5ee87d0"),
            hex!("0x838c5655cb21c6cb83313b5a631175dff4963772cce9108188b34ac87c81c41e"),
            hex!("0x662ee4dd2dd7b2bc707961b1e646c4047669dcb6584f0d8d770daf5d7e7deb2e"),
            hex!("0x388ab20e2573d171a88108e79d820e98f26c0b84aa8b2f4aa4968dbb818ea322"),
            hex!("0x93237c50ba75ee485f4c22adf2f741400bdf8d6a9cc7df7ecae576221665d735"),
            hex!("0x8448818bb4ae4562849e949e17ac16e0be16688e156b5cf15e098c627c0056a9"),
        ];

        for (i, hash) in rer_proof.iter().enumerate() {
            assert_eq!(
                hash, &additional_hashes[i],
                "Hash at index {} does not match",
                i
            );
        }

        Ok(())
    }

    #[test]
    fn test_anothertodo() -> Result<(), eyre::Error> {
        // test also that leafs can be modified ok.
        let _ = std::fs::remove_dir_all("db_test");
        let mut t = MerkleForest::open("db_test").unwrap();

        let rollup_exit_roots = vec![
            (
                1,
                hex!("0x2eec493df61d778cb2a5d02b73445ea758a64543b540a1b8111d0fb47274221f"),
            ),
            (
                2,
                hex!("0x5e5d1aa128d94a3c164b3f76cb54b02fec1387d247b4f500fc562272c717424d"),
            ),
            (
                3,
                hex!("0xb218b61c22d70f2d59bab9cf4964fe9f7ef73afabb776aa54fba30c97fd89b4b"),
            ),
            (
                4,
                hex!("0x0000000000000000000000000000000000000000000000000000000000000000"),
            ),
            (
                5,
                hex!("0x386c1f907fe8768b23a17cf2ebf449e679ae8ab38d17131cd603d6655cc6770b"),
            ),
            (
                6,
                hex!("0xb52156faceb1557001af394dae8fd5b0de31ce9a321f63e813c115abf9ddadb2"),
            ),
            (
                7,
                hex!("0xac9a61e84eb4347c58a8dc22949ef17a593c0662da75d06936ff124f6aac86b6"),
            ),
            (
                8,
                hex!("0x5afa8f38e955e222e440ee61ca4b8f455c41b542e7f8b0f7d276f2ef0026106d"),
            ),
            (
                9,
                hex!("0x2c22e60e9e5ebcce3885897ecebffec1aaf7e141f1241dee04d4b47f3444d665"),
            ),
            (
                10,
                hex!("0x97998af9b58859d6ec3fa77a0613e504fb8e8f051885e8f3a4a13d3bf5eaedae"),
            ),
            (
                11,
                hex!("0xdb1bef18121aed0ec6da77297897bd2ee34b9da0ba3f3766b1fe9cd7446af5d5"),
            ),
            (
                12,
                hex!("0xdb1bef18121aed0ec6da77297897bd2ee34b9da0ba3f3766b1fe9cd7446af5d5"),
            ),
            (
                13,
                hex!("0xe364a474900343072eb2f4234169079ecdb96660901f63f0eaf7df2d7a34243e"),
            ),
            (
                14,
                hex!("0x78c2c6a7aeec799425ca459a970dd859e663af7fcda42d90bfa59c886f46a5da"),
            ),
            (
                15,
                hex!("0x0000000000000000000000000000000000000000000000000000000000000000"),
            ),
            (
                16,
                hex!("0x05fdb6446f26abaf23892e5da409edeb2a125bec3af72eeb9b2ef037449975c4"),
            ),
            (
                17,
                hex!("0x4e5b0deba6495eff975e8389f220520d6c25363cfd1b55e30cbe3667ebdf545a"),
            ),
            (
                18,
                hex!("0xa3a26da5b9c197a458570a50487c7c44f1df73f5ce4024ef913af55f0102ae55"),
            ),
            (
                19,
                hex!("0xcc7e59609197f085caadb64e436cf248f40f4158324daf334ac8099fd1cbe613"),
            ),
            (
                20,
                hex!("0x656db1fd488f456faa9766f4948fc5d8602371602b66e7874080bcc6621f82bc"),
            ),
        ];

        for (rollup_id, exit_root) in &rollup_exit_roots {
            // note that rollup 1 is placed at 0, etc.
            t.set_rollup_leaf(*rollup_id, &exit_root, 1).unwrap();
        }

        for i in [1, 3, 5, 6] {
            // modify some random leafes
            t.set_rollup_leaf(i, &B256::default(), 1).unwrap();
        }

        // Reset all leafs to the same value
        for (rollup_id, exit_root) in &rollup_exit_roots {
            // note that rollup 1 is placed at 0, etc.
            t.set_rollup_leaf(*rollup_id, &exit_root, 1).unwrap();
        }
        println!("root: {:?}", t.get_root(&TreeType::RollupExitTree).unwrap());
        assert_eq!(
            t.get_root(&TreeType::RollupExitTree).unwrap(),
            Some(hex!(
                "0xcfc0867b45230182da671566501ec406ec5a27cade099444de6e34562d36ea40"
            ))
        );

        Ok(())
    }

    // TODO: test if tree is full what happens.

    #[test]
    fn persistent_tree_roundtrip() -> Result<(), eyre::Error> {
        let _ = std::fs::remove_dir_all("db_test");

        {
            let mut t = MerkleForest::open("db_test").unwrap();

            // Simple deterministic "random" number generator
            fn simple_random(seed: u64) -> u64 {
                (seed.wrapping_mul(6364136223846793005).wrapping_add(1)) % (1 << 32)
            }

            let mut seed = 0;

            for i in 0..100 {
                seed = simple_random(seed);
                let random_metadata = format!("metadata_{}", seed);

                let leaf = LeafBridge::new(BridgeEvent {
                    leafType: 1,
                    originNetwork: 0,
                    originAddress: address!("0x1111111111111111111111111111111111111111"),
                    destinationNetwork: 2,
                    destinationAddress: address!("0x2222222222222222222222222222222222222222"),
                    amount: uint!(888_888_U256),
                    metadata: Bytes::from(random_metadata.clone().into_bytes()),
                    depositCount: i,
                });

                t.append_events(1, &[leaf.clone()], 1).unwrap();

                let hashed_leaf = leaf.hashed_leaf();

                let root_t = t.get_root(&TreeType::LocalExitTree(1)).unwrap();

                let m_proof = t
                    .merkle_proof(TreeType::LocalExitTree(1), i as u64)
                    .unwrap();
                let calculated_root = calculate_merkle_root(&hashed_leaf, &m_proof, i as u64);

                assert_eq!(calculated_root, root_t.unwrap(), "Roots do not match");
            }
        }

        // 2) reopen, the state is still there
        {
            //let t = MerkleForest::open("db").unwrap();
            //assert_eq!(t.leaf_count, 5);
            //let p = t.get_merkle_proof(2)?;
            //assert!(t.verify_merkle_proof([2u8; 32].into(), &p, 2));
        }
        Ok(())
    }
}
