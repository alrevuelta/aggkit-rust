use crate::contracts::PolygonZkEVMBridgeV2::PolygonZkEVMBridgeV2Instance;
use crate::contracts::PolygonZkEVMGlobalExitRootV2::PolygonZkEVMGlobalExitRootV2Instance;
use crate::merkle_tree::MerkleForest;
use crate::merkle_tree::TreeType;
use alloy::primitives::B256;
use alloy::providers::fillers::BlobGasFiller;
use alloy::providers::fillers::ChainIdFiller;
use alloy::providers::fillers::FillProvider;
use alloy::providers::fillers::GasFiller;
use alloy::providers::fillers::JoinFill;
use alloy::providers::fillers::NonceFiller;
use alloy::providers::{Identity, Provider, RootProvider};
use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

// TODO: take this from somewhere else
const DEPTH: usize = 32;

// TODO: Ugly
pub type ProviderStack = Arc<
    FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
>;

// TODO: Add multiple L2 sync status
#[derive(Serialize)]
struct SyncStatus {
    l1_bridge: TreeSyncStatus,
    l1_info_tree: TreeSyncStatus,
    l2_bridge: Vec<TreeSyncStatus>,
}

#[derive(Serialize)]
struct TreeSyncStatus {
    local_leaf_count: u64,
    contract_leaf_count: u64,
    is_synced: bool,
}

#[derive(Serialize)]
struct ClaimProofResponse {
    proof: Proof,
}

#[derive(Serialize)]
struct Proof {
    merkle_proof: Vec<String>, // TODO: Use proper types
    rollup_merkle_proof: Vec<String>,
    main_exit_root: String,
    rollup_exit_root: String,
}

#[derive(Clone)]
pub struct AppState {
    pub tree: Arc<MerkleForest>,
    // Consider passing a configuration struct for better organization
    pub l1_bridge: PolygonZkEVMBridgeV2Instance<ProviderStack>,
    pub l2_bridges: HashMap<u32, PolygonZkEVMBridgeV2Instance<ProviderStack>>,
    pub l1_infotree: PolygonZkEVMGlobalExitRootV2Instance<ProviderStack>,
    //pub rollup_manager: PolygonRollupManagerInstance<ProviderStack>, Not needed?
}

async fn sync_status(State(state): State<AppState>) -> impl IntoResponse {
    let l1_contract_deposit_count: u64 = state
        .l1_bridge
        .depositCount()
        .call()
        .await
        .unwrap()
        .try_into()
        .unwrap(); // TODO: handle error properly

    let l1_contract_l1infotree: u64 = state
        .l1_infotree
        .depositCount()
        .call()
        .await
        .unwrap()
        .try_into()
        .unwrap(); // TODO: handle error properly
    // TODO: use u64??
    let l1_bridge_deposit_count = state
        .tree
        .get_leaf_count(&TreeType::LocalExitTree(0))
        .unwrap();

    let l1_local_deposit_count = state.tree.get_leaf_count(&TreeType::L1InfoTree).unwrap();

    // Build status for each L2 bridge.
    let mut l2_bridge_status = Vec::new();
    for (aggchain_id, l2_bridge_instance) in state.l2_bridges.iter() {
        // Fetch on-chain deposit count for this L2 bridge
        let contract_deposit_count: u64 = l2_bridge_instance
            .depositCount()
            .call()
            .await
            .unwrap()
            .try_into()
            .unwrap();

        // Fetch local leaf count for the corresponding Local Exit Tree
        let local_deposit_count = state
            .tree
            .get_leaf_count(&TreeType::LocalExitTree(*aggchain_id))
            .unwrap();

        l2_bridge_status.push(TreeSyncStatus {
            local_leaf_count: local_deposit_count as u64,
            contract_leaf_count: contract_deposit_count,
            is_synced: local_deposit_count as u64 == contract_deposit_count,
        });
    }

    // TODO:
    // - Do parsing properly
    // - Handle errors properly
    let response = SyncStatus {
        l1_bridge: TreeSyncStatus {
            local_leaf_count: l1_bridge_deposit_count as u64,
            contract_leaf_count: l1_contract_deposit_count,
            is_synced: l1_bridge_deposit_count as u64 == l1_contract_deposit_count as u64,
        },

        l1_info_tree: TreeSyncStatus {
            local_leaf_count: l1_local_deposit_count as u64,
            contract_leaf_count: l1_contract_l1infotree,
            is_synced: l1_local_deposit_count as u64 == l1_contract_l1infotree as u64,
        },

        l2_bridge: l2_bridge_status,
    };

    axum::Json(response)
}

#[derive(Deserialize, Debug)]
struct ClaimProofParams {
    #[serde(rename = "net_id")] // Updated query parameter name
    network_id: u32,
    #[serde(rename = "deposit_cnt")] // Updated query parameter name
    deposit_count: u64,
}

async fn claim_proof(
    State(state): State<AppState>,
    Query(params): Query<ClaimProofParams>,
) -> impl IntoResponse {
    // TODO: Do proper error handling
    let mer = state.tree.get_root(&TreeType::LocalExitTree(0)).unwrap();
    let rer = state.tree.get_root(&TreeType::RollupExitTree).unwrap();

    let ler_proof = state
        .tree
        .merkle_proof(
            TreeType::LocalExitTree(params.network_id),
            params.deposit_count,
        )
        .unwrap();

    // The RER only makes sense for non-mainnet.
    let rer_proof = if params.network_id == 0 {
        [B256::default(); DEPTH]
    } else {
        state
            .tree
            .merkle_proof(TreeType::RollupExitTree, params.network_id as u64)
            .unwrap()
    };

    if mer.is_none() || rer.is_none() {
        // TODO: Handle this
        //return axum::Json("Error: TODO: Handle this better");
    }

    let response = ClaimProofResponse {
        proof: Proof {
            merkle_proof: ler_proof.map(|byte| format!("0x{:02x}", byte)).to_vec(),
            rollup_merkle_proof: rer_proof.map(|byte| format!("0x{:02x}", byte)).to_vec(),
            main_exit_root: format!("0x{:02x}", mer.unwrap()),
            rollup_exit_root: format!("0x{:02x}", rer.unwrap()),
        },
    };

    axum::Json(response)
}

pub async fn run_server(state: AppState) -> Result<(), Box<dyn Error + Send + Sync>> {
    let server_task = tokio::spawn(async move {
        let app = Router::new()
            .route("/sync-status", get(sync_status))
            .route("/merkle-proof", get(claim_proof))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

        axum::serve(listener, app).await
    });

    server_task.await??;

    Ok(())
}
