use alloy::sol;

// ABI taken from https://github.com/agglayer/agglayer-contracts/tree/main/compiled-contracts
// See ["abi"] field of the json of each file.

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PolygonZkEVMGlobalExitRootV2,
    "abi/PolygonZkEVMGlobalExitRootV2.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PolygonZkEVMBridge,
    "abi/PolygonZkEVMBridge.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PolygonZkEVMBridgeV2,
    "abi/PolygonZkEVMBridgeV2.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PolygonRollupManager,
    "abi/PolygonRollupManager.json"
);

// This is an old version of the event. Onchain it has the same address but the ABI
// had a breaking chang in the AddExistingRollup event.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract PolygonRollupManagerOld {
        event AddExistingRollup(
            uint32 indexed rollupID,
            uint64 forkID,
            address rollupAddress,
            uint64 chainID,
            uint8 rollupCompatibilityID,
            uint64 lastVerifiedBatchBeforeUpgrade
        );
    }
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    BridgeL2SovereignChain,
    "abi/BridgeL2SovereignChain.json"
);

// TODO: Unused by now
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    GlobalExitRootManagerL2SovereignChain,
    "abi/GlobalExitRootManagerL2SovereignChain.json"
);

// TODO: Unused by now
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PolygonZkEVMGlobalExitRootL2,
    "abi/PolygonZkEVMGlobalExitRootL2.json"
);
