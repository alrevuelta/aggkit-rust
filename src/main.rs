use aggkit_rust::api::{AppState, ProviderStack, run_server};
use aggkit_rust::cli::Cli;
use aggkit_rust::contracts::PolygonZkEVMBridgeV2::{self, PolygonZkEVMBridgeV2Instance};
use aggkit_rust::contracts::{PolygonRollupManager, PolygonZkEVMGlobalExitRootV2};
use aggkit_rust::indexer::Indexer;
use aggkit_rust::indexer_bridge::BridgeEventProcessor;
use aggkit_rust::indexer_l1infotree::L1InfoTreeEventProcessor;
use aggkit_rust::indexer_rollupmanager::RollupManagerEventProcessor;
use aggkit_rust::merkle_tree::MerkleForest;
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::rpc::client::RpcClient;
use alloy::rpc::types::BlockNumberOrTag;
use alloy::transports::http::reqwest::Url;
use alloy::transports::layers::RetryBackoffLayer;
use clap::Parser;
use eyre::Result;
use futures::future;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::{signal, task};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    let l1_rpc_url: Url = cli.l1_rpc_url.parse()?;
    let l2_rpc_urls: Vec<Url> = cli
        .l2_rpcs
        .iter()
        .map(|l2_rpc| l2_rpc.rpc_url.clone())
        .collect();
    let ger_address: Address = cli.ger_address.parse()?;
    let bridge_address: Address = cli.bridge_address.parse()?;
    let rollup_manager_address: Address = cli.rollup_manager_address.parse()?;
    let key_value_store: String = cli.key_value_store;

    let max_retry: u32 = 100;
    let backoff: u64 = 2000;
    let cups: u64 = 100;

    // TODO: Organize this better.
    let l1_provider = Arc::new(
        ProviderBuilder::new().connect_client(
            RpcClient::builder()
                .layer(RetryBackoffLayer::new(max_retry, backoff, cups))
                .http(l1_rpc_url.clone()),
        ),
    );

    // Build a provider per configured L2 RPC
    let l2_providers: Vec<ProviderStack> = l2_rpc_urls
        .iter()
        .map(|l2_rpc_url| {
            Arc::new(
                ProviderBuilder::new().connect_client(
                    RpcClient::builder()
                        .layer(RetryBackoffLayer::new(max_retry, backoff, cups))
                        .http(l2_rpc_url.clone()),
                ),
            )
        })
        .collect();

    println!("l1 rpc url: {:?}", l1_rpc_url.as_str());
    println!("l2 rpc urls: {:?}", l2_rpc_urls);
    println!("key value store: {:?}", key_value_store);
    println!("Ger contract address: {:?}", ger_address);
    println!("Bridge contract address: {:?}", bridge_address);
    println!(
        "Rollup manager contract address: {:?}",
        rollup_manager_address
    );

    let trees = Arc::new(MerkleForest::open(key_value_store)?);

    let l1_bridge_indexer = Indexer::new(
        l1_rpc_url.clone(),
        "l1-bridge-indexer".to_string(),
        bridge_address,
        BlockNumberOrTag::Latest,
        BridgeEventProcessor {
            tree: Arc::clone(&trees),
            aggchain_id: 0,
        },
        cli.block_range,
    )?;

    // Create one indexer per configured L2 RPC, using aggchain_id in the name
    let l2_bridge_indexers: Vec<Indexer<BridgeEventProcessor>> = cli
        .l2_rpcs
        .iter()
        .map(|l2_rpc| {
            Indexer::new(
                l2_rpc.rpc_url.clone(),
                format!("l2-bridge-indexer-aggchain-{}", l2_rpc.aggchain_id),
                bridge_address,
                BlockNumberOrTag::Latest,
                BridgeEventProcessor {
                    tree: Arc::clone(&trees),
                    aggchain_id: l2_rpc.aggchain_id,
                },
                cli.block_range,
            )
        })
        .collect::<Result<_, _>>()?;

    let rollup_manager_indexer = Indexer::new(
        l1_rpc_url.clone(),
        "rollup-manager-indexer".to_string(),
        rollup_manager_address,
        BlockNumberOrTag::Latest,
        RollupManagerEventProcessor {
            tree: Arc::clone(&trees),
        },
        cli.block_range,
    )?;

    let l1infotree_indexer = Indexer::new(
        l1_rpc_url.clone(),
        "l1infotree-indexer".to_string(),
        ger_address,
        BlockNumberOrTag::Latest,
        L1InfoTreeEventProcessor {
            tree: Arc::clone(&trees),
            provider: l1_provider.clone(),
        },
        cli.block_range,
    )?;

    let handle_l1_bridge_indexer = task::spawn(l1_bridge_indexer.run());

    // Spawn a task per L2 bridge indexer
    let l2_bridge_tasks: Vec<_> = l2_bridge_indexers
        .into_iter()
        .map(|indexer| task::spawn(indexer.run()))
        .collect();

    // Future that resolves once all L2 bridge indexer tasks finish
    let handle_l2_bridge_indexers = future::join_all(l2_bridge_tasks);

    let handle_l1infotree_indexer = task::spawn(l1infotree_indexer.run());
    let handle_rollup_manager_indexer = task::spawn(rollup_manager_indexer.run());

    let l1_bridge = PolygonZkEVMBridgeV2::new(bridge_address, l1_provider.clone());
    let l1_infotree = PolygonZkEVMGlobalExitRootV2::new(ger_address, l1_provider.clone());
    let rollup_manager = PolygonRollupManager::new(rollup_manager_address, l1_provider.clone());

    let l2_bridges: HashMap<u32, PolygonZkEVMBridgeV2Instance<ProviderStack>> = cli
        .l2_rpcs
        .iter()
        .map(|l2_rpc| {
            let provider = Arc::new(
                ProviderBuilder::new().connect_client(
                    RpcClient::builder()
                        .layer(RetryBackoffLayer::new(max_retry, backoff, cups))
                        .http(l2_rpc.rpc_url.clone()),
                ),
            );
            (
                l2_rpc.aggchain_id,
                PolygonZkEVMBridgeV2::new(bridge_address, provider),
            )
        })
        .collect();

    let state = AppState {
        tree: Arc::clone(&trees),
        l1_bridge: l1_bridge,
        l2_bridges: l2_bridges,
        l1_infotree: l1_infotree,
        //rollup_manager: rollup_manager,
    };
    let handle_api = task::spawn(run_server(state));

    // TODO: Do proper error handling

    tokio::select! {
        res = handle_l1_bridge_indexer => {
            println!("Index task l1 completed: {:?}", res);
        }
        res = handle_l2_bridge_indexers => {
            println!("Index tasks l2 completed: {:?}", res);
        }

        res = handle_l1infotree_indexer => {
            println!("Index task l1infotree completed: {:?}", res);
        }

        res = handle_rollup_manager_indexer => {
            println!("Index task rollup manager completed: {:?}", res);
        }

        res = handle_api => {
            println!("Server task completed: {:?}", res);
        }

        res = signal::ctrl_c() => {
            println!("Received shutdown signal: {:?}", res);
        }
    }

    // TODO: Handle sigterm gracefully.

    Ok(())
}
