use crate::contracts::PolygonZkEVMBridgeV2::BridgeEvent;
use alloy::primitives::Log as LogPrimitive;
use alloy::primitives::keccak256;
use alloy::rpc::types::Log;
use hex;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
use std::sync::Arc;

#[derive(Clone)]
pub struct Database {
    pool: Arc<SqlitePool>,
}

impl Database {
    // Create a new SQLite database (file) and ensure the required table exists.
    pub async fn new(database_url: &str) -> Result<Self, eyre::Error> {
        if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
            println!("Creating database {}", database_url);
            match Sqlite::create_database(database_url).await {
                Ok(_) => println!("Create db success"),
                Err(error) => panic!("error: {}", error),
            }
        } else {
            println!("Database already exists");
        }

        let db = SqlitePool::connect(database_url).await.unwrap();

        /* TODO: Have a look into this.
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;*/

        Ok(Self { pool: Arc::new(db) })
    }

    // TODO: Unoptimized, create INDEX, etc.
    pub async fn create_tables(&self) -> Result<(), eyre::Error> {
        // Create table if it doesn't exist yet. Adjust types as needed.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bridge_events (
                id TEXT PRIMARY KEY,
                leaf_type INTEGER,
                orig_net INTEGER,
                orig_addr TEXT,
                amount TEXT,
                dest_net INTEGER,
                dest_addr TEXT,
                block_num INTEGER,
                deposit_cnt INTEGER,
                network_id INTEGER,
                tx_hash TEXT,
                claim_tx_hash TEXT,
                metadata TEXT,
                ready_for_claim INTEGER,
                global_index INTEGER
            );
            "#,
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_bridge_event(
        &self,
        event: &Log<BridgeEvent>,
        network_id: u32,
    ) -> Result<(), eyre::Error> {
        sqlx::query(
            r#"
            INSERT INTO bridge_events (
                id,
                leaf_type,
                orig_net,
                orig_addr,
                amount,
                dest_net,
                dest_addr,
                block_num,
                deposit_cnt,
                network_id,
                tx_hash,
                claim_tx_hash,
                metadata,
                ready_for_claim,
                global_index
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(calculate_id(network_id, event.data().depositCount))
        .bind(&event.data().leafType)
        .bind(&event.data().originNetwork)
        .bind(&event.data().originAddress.to_string())
        .bind(&event.data().amount.to_string())
        .bind(&event.data().destinationNetwork)
        .bind(&event.data().destinationAddress.to_string())
        .bind(
            event
                .block_number
                .ok_or_else(|| eyre::eyre!("Block number is None"))? as i64,
        )
        .bind(&event.data().depositCount)
        .bind(network_id)
        .bind(
            event
                .transaction_hash
                .ok_or_else(|| eyre::eyre!("Transaction hash is None"))?
                .to_string(),
        )
        .bind("0xnot_implemented") // TODO: Not implemented.
        .bind(&event.data().metadata.to_string())
        .bind(0) // TODO: Not implemented.
        .bind(0) // TODO: Not implemented
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_bridges(
        &self,
        filters: BridgeFilters,
    ) -> Result<(Vec<BridgeRecord>, i64), eyre::Error> {
        // Default pagination values
        let page_size = filters.page_size.unwrap_or(50).clamp(1, 1000);
        let page_number = filters.page_number.unwrap_or(1).max(1);
        let offset = (page_number - 1) * page_size;

        // Always filter by `network_id` first; additional filters are appended.
        let mut qb: QueryBuilder<sqlx::Sqlite> =
            QueryBuilder::new("SELECT * FROM bridge_events WHERE network_id = ");

        // Always filter by network_id first.
        qb.push_bind(filters.network_id as i64);

        // Append any optional filters.
        apply_filters(&mut qb, &filters);

        // ORDER BY deposit_cnt ascending
        qb.push(" ORDER BY deposit_cnt ASC");

        // Apply pagination
        qb.push(" LIMIT ")
            .push_bind(page_size)
            .push(" OFFSET ")
            .push_bind(offset);

        let bridges: Vec<BridgeRecord> = qb.build_query_as().fetch_all(&*self.pool).await?;

        // Count query. Same query to count total number of rows. For pagination.
        let mut qb_cnt: QueryBuilder<sqlx::Sqlite> =
            QueryBuilder::new("SELECT COUNT(*) as cnt FROM bridge_events WHERE network_id = ");
        qb_cnt.push_bind(filters.network_id as i64);

        // Re-use the same optional filters for the count query.
        apply_filters(&mut qb_cnt, &filters);

        let count: (i64,) = qb_cnt.build_query_as().fetch_one(&*self.pool).await?;

        Ok((bridges, count.0))
    }
}

// The id field of the bridge_events table is a unique identifier for the bridge event.
// It is a hash of the network_id and deposit_cnt.
// TODO: Unsure if there are performance penalties if using a STRING as primary key.
fn calculate_id(network_id: u32, deposit_cnt: u32) -> String {
    let mut buf = [0u8; 8];
    buf[..4].copy_from_slice(&network_id.to_be_bytes());
    buf[4..].copy_from_slice(&deposit_cnt.to_be_bytes());

    let hash = keccak256(&buf);
    format!("0x{}", hex::encode(hash.as_slice()))
}

fn apply_filters<'qb>(qb: &mut QueryBuilder<'qb, Sqlite>, f: &'qb BridgeFilters) {
    macro_rules! add {
        ($opt:expr, $col:literal) => {
            if let Some(ref v) = $opt {
                qb.push(" AND ").push($col).push(" = ").push_bind(v);
            }
        };
    }

    add!(f.leaf_type, "leaf_type");
    add!(f.orig_net, "orig_net");
    add!(f.orig_addr, "orig_addr");
    add!(f.amount, "amount");
    add!(f.dest_net, "dest_net");
    add!(f.dest_addr, "dest_addr");
    add!(f.block_num, "block_num");
    add!(f.deposit_cnt, "deposit_cnt");
    add!(f.tx_hash, "tx_hash");
    add!(f.claim_tx_hash, "claim_tx_hash");
    add!(f.metadata, "metadata");
    if let Some(b) = f.ready_for_claim {
        qb.push(" AND ready_for_claim = ").push_bind(b as i32);
    }
    add!(f.global_index, "global_index");
}

#[derive(Serialize, Clone, sqlx::FromRow, Debug)]
pub struct BridgeRecord {
    pub leaf_type: i64,
    pub orig_net: i64,
    pub orig_addr: String,
    pub amount: String,
    pub dest_net: i64,
    pub dest_addr: String,
    pub block_num: i64,
    pub deposit_cnt: i64,
    pub network_id: u32,
    pub tx_hash: String,
    pub claim_tx_hash: String,
    pub metadata: String,
    pub ready_for_claim: bool,
    pub global_index: i64,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct BridgeFilters {
    // Pagination
    pub page_number: Option<i64>,
    pub page_size: Option<i64>,

    // Mandatory filter
    pub network_id: u32,

    // Optional filters. All correspond to column names
    pub leaf_type: Option<i64>,
    pub orig_net: Option<i64>,
    pub orig_addr: Option<String>,
    pub amount: Option<String>,
    pub dest_net: Option<i64>,
    pub dest_addr: Option<String>,
    pub block_num: Option<i64>,
    pub deposit_cnt: Option<i64>,
    pub tx_hash: Option<String>,
    pub claim_tx_hash: Option<String>,
    pub metadata: Option<String>,
    pub ready_for_claim: Option<bool>,
    pub global_index: Option<i64>,
}

impl Default for BridgeFilters {
    fn default() -> Self {
        Self {
            page_number: None,
            page_size: None,
            network_id: 0,
            leaf_type: None,
            orig_net: None,
            orig_addr: None,
            amount: None,
            dest_net: None,
            dest_addr: None,
            block_num: None,
            deposit_cnt: None,
            tx_hash: None,
            claim_tx_hash: None,
            metadata: None,
            ready_for_claim: None,
            global_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::B256;
    use alloy::primitives::Bytes;
    use alloy::primitives::address;
    use alloy::uint;

    use super::*;

    #[tokio::test]
    async fn test_get_bridges() {
        // Create a new in-memory database with the required tables.
        let db = Database::new("sqlite::memory:").await.unwrap();
        db.create_tables().await.unwrap();

        // Insert a bunch of bridge events (see depositCount)
        let network_id = 0u32;
        for i in 0..10 {
            let log = Log::<BridgeEvent> {
                inner: LogPrimitive {
                    address: address!("0x1111111111111111111111111111111111111111"),
                    data: BridgeEvent {
                        leafType: 1,
                        originNetwork: 0,
                        originAddress: address!("0x1111111111111111111111111111111111111111"),
                        destinationNetwork: 2,
                        destinationAddress: address!("0x2222222222222222222222222222222222222222"),
                        amount: uint!(6_666_666_U256),
                        metadata: Bytes::from_static(b"some metadata"),
                        // -> Set an increasing deposit count.
                        depositCount: i,
                    },
                },
                block_hash: Some(B256::ZERO),
                block_number: Some(0),
                block_timestamp: None,
                transaction_hash: Some(B256::ZERO),
                transaction_index: Some(0),
                log_index: Some(0),
                removed: false,
            };
            db.insert_bridge_event(&log, network_id).await.unwrap();
        }

        // Test 1. Get a single bridge event by deposit count.
        {
            let filters = BridgeFilters {
                // -> filter by deposit count.
                deposit_cnt: Some(5),
                network_id: network_id,
                // Defaults to None.
                ..Default::default()
            };

            let (bridges, count) = db.get_bridges(filters).await.unwrap();
            assert_eq!(bridges.len(), 1);
            assert_eq!(count, 1);
            assert_eq!(bridges[0].deposit_cnt, 5);
        }

        // Test 2. Get all bridge events using pagination. First page. 5/10
        {
            let filters = BridgeFilters {
                // -> page 1
                page_number: Some(1),
                page_size: Some(5),
                network_id: network_id,
                // Defaults to None.
                ..Default::default()
            };
            let (bridges, count) = db.get_bridges(filters).await.unwrap();
            assert_eq!(bridges.len(), 5);
            assert_eq!(count, 10);
            assert_eq!(bridges[0].deposit_cnt, 0);
            assert_eq!(bridges[1].deposit_cnt, 1);
            assert_eq!(bridges[2].deposit_cnt, 2);
            assert_eq!(bridges[3].deposit_cnt, 3);
            assert_eq!(bridges[4].deposit_cnt, 4);
        }

        // Test 3. Get all bridge events using pagination. Second page. 5/10
        {
            let filters = BridgeFilters {
                // -> page 2
                page_number: Some(2),
                page_size: Some(5),
                network_id: network_id,
                // Defaults to None.
                ..Default::default()
            };
            let (bridges, count) = db.get_bridges(filters).await.unwrap();
            assert_eq!(bridges.len(), 5);
            assert_eq!(count, 10);
            assert_eq!(bridges[0].deposit_cnt, 5);
            assert_eq!(bridges[1].deposit_cnt, 6);
            assert_eq!(bridges[2].deposit_cnt, 7);
            assert_eq!(bridges[3].deposit_cnt, 8);
            assert_eq!(bridges[4].deposit_cnt, 9);
        }
    }
}
