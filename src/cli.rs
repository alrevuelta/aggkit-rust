use alloy::transports::http::reqwest::Url;
use clap::Parser;
use std::str::FromStr;

// TODO:
// - Add starting block for each contract
// - Add parameters to index multiple aggchains L2.
// - Add presets for different aggchains so one doesnt have to set all the parameters.

// Custom type to hold Aggchain ID and RPC URL
#[derive(Clone, Debug)]
pub struct L2Rpc {
    pub aggchain_id: u32,
    pub rpc_url: Url,
}

impl FromStr for L2Rpc {
    type Err = String;

    // TODO: Do some proper pattern matching here.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ':');
        let aggchain_id_str = parts.next().ok_or("Missing Aggchain ID")?;
        let rpc_url_str = parts.next().ok_or("Missing URL")?;

        let aggchain_id = aggchain_id_str
            .parse::<u32>()
            .map_err(|_| "Invalid Aggchain ID".to_string())?;
        let rpc_url = rpc_url_str
            .parse::<Url>()
            .map_err(|_| "Invalid URL".to_string())?;

        Ok(L2Rpc {
            aggchain_id,
            rpc_url,
        })
    }
}

#[derive(Parser)]
#[command(name = "aggkit-rust")]
#[command(about = "TODO", long_about = None)]
pub struct Cli {
    /// RPC URL for the Ethereum L1 network.
    /// https://mainnet.infura.io/v3/xxx
    #[arg(long)]
    pub l1_rpc_url: String,

    /// RPC URLs for the Aggchain L2 networks. chain-id:rpc-url.
    /// --l2-rpc-url=0:http://someurl
    #[arg(long = "l2-rpc-url", value_parser = clap::value_parser!(L2Rpc))]
    pub l2_rpcs: Vec<L2Rpc>,

    /// Path for the key-value store storing the merkle tree.
    /// Example: db
    #[arg(long, default_value = "db")]
    pub key_value_store: String,

    /// Contract address of the PolygonZkEVMGlobalExitRootV2.
    /// Example: 0x580bda1e7A0CFAe92Fa7F6c20A3794F169CE3CFb
    #[arg(long, default_value = "0x580bda1e7A0CFAe92Fa7F6c20A3794F169CE3CFb")]
    pub ger_address: String,

    /// Contract address of the PolygonZkEVMBridgeV2.
    /// Example: 0x2a3DD3EB832aF982ec71669E178424b10Dca2EDe
    #[arg(long, default_value = "0x2a3DD3EB832aF982ec71669E178424b10Dca2EDe")]
    pub bridge_address: String,

    /// Contract address of the PolygonRollupManager.
    /// Example: 0x5132A183E9F3CB7C848b0AAC5Ae0c4f0491B7aB2
    #[arg(long, default_value = "0x5132A183E9F3CB7C848b0AAC5Ae0c4f0491B7aB2")]
    pub rollup_manager_address: String,

    /// Number of blocks to query in a single request.
    /// Lower values help avoid RPC timeouts but may slow down indexing.
    #[arg(long, default_value = "10000")]
    pub block_range: u64,
}
