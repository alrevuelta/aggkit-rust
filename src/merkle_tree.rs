use crate::leaf_bridge::LeafBridge;
use alloy::primitives::{FixedBytes, keccak256};
use eyre::{Result, eyre};
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DB, DBCompressionType, Options, WriteBatch, WriteOptions,
};
use std::{convert::TryInto, path::Path, process::exit};

// This file contains an implementation of all Merkle trees existing in the Agglayer. These are:
// - Local Exit Trees: Merkle trees for each Aggchain, storing the bridge exits as leafs.
// - Rollup Exit Tree: A Merkle tree made of all the Local Exit Roots of all Aggchains (except of the Main Exit Tree).
// - L1 Info Tree: A Merkle tree storing all the Global Exit Roots + some extra information at different points in time.
// It is implemented using a RocksDB key-value store. This allows for very fast retrieval of the Merkle tree roots
// and proofs of any leaf in any of the mentioned trees.

// Each tree is identified by its TreeType. There are only one RollupExitTree and one L1InfoTree trees
// and an arbitrary number of LocalExitTree identified by their AggchainId.
// Every tree stores some metadata like the latest processed block number and the number of leaves inserted so far.
// Note that LocalExitTree and L1InfoTree are append only, and RollupExitTree is not, since it allow
// insertion/modification of new leaves at any index.

pub type AggchainId = u32;
pub type BlockNum = u64;

// Depth of the merkle tree and maximum number of leaves that
// can be stored. Calculated as 2^DEPTH - 1.
const DEPTH: usize = 32;
const MAX_LEAVES: u32 = ((1u64 << DEPTH) - 1) as u32;

pub enum TreeType {
    /// Append-only Local Exit Tree of a specific AggChain.
    /// When 0 it is the Main Exit Tree. This type requires a TreeId.
    LocalExitTree(AggchainId),
    /// Tree made of all rollup Local Exit Roots. A single tree of this type exists.
    /// Not append-only, leafs can be modified at any index.
    RollupExitTree,
    /// Append-only L1 Info tree storing the Global Exit Root + some
    /// extra information at different points in time. Just a single tree of this type exists.
    L1InfoTree,
}

impl TreeType {
    pub fn tree_type(&self) -> u8 {
        match self {
            Self::LocalExitTree(_) => 1,
            Self::RollupExitTree => 2,
            Self::L1InfoTree => 3,
        }
    }

    pub fn aggchain_id(&self) -> AggchainId {
        match *self {
            Self::LocalExitTree(id) => id,
            _ => 0,
        }
    }
}

const CF_RAW_BRIDGE_LEAF: &str = "raw_bridge_leaf";
const CF_METADATA: &str = "metadata";

// TODO: Unsure if I will need this.
const CF_TREE_LEVELS: &str = "tree_levels";

#[repr(u8)]
enum ColumnType {
    HashedNode = 0,
    Metadata = 1,
}

#[repr(u8)]
enum MetaTag {
    /// Number of leaves inserted so far in the Merkle tree.
    LeafCount = 0,
    /// Latest fully processed block number.
    LatestProcessedBlock = 1,
}

fn node_key(tree: &TreeType, level: u8, index: u32) -> [u8; 11] {
    let mut k = [0u8; 11];
    k[..4].copy_from_slice(&tree.aggchain_id().to_be_bytes());
    k[4] = ColumnType::HashedNode as u8;
    k[5] = tree.tree_type();
    k[6] = level;
    k[7..11].copy_from_slice(&index.to_be_bytes());
    k
}

fn meta_key(tree: &TreeType, tag: MetaTag) -> [u8; 7] {
    let mut k = [0u8; 7];
    k[..4].copy_from_slice(&tree.aggchain_id().to_be_bytes());
    k[4] = ColumnType::Metadata as u8;
    k[5] = tree.tree_type();
    k[6] = tag as u8;
    k
}
pub struct MerkleForest {
    db: DB,
    zero: [FixedBytes<32>; DEPTH + 1],
}

impl MerkleForest {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // TODO: Tune this. https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide
        let mut tree_opts = Options::default();
        let mut raw_opts = Options::default();
        let mut meta_opts = Options::default();

        tree_opts.set_compression_type(DBCompressionType::Zstd);
        raw_opts.set_compression_type(DBCompressionType::Zstd);
        meta_opts.set_compression_type(DBCompressionType::Zstd);

        let db = DB::open_cf_descriptors(
            &opts,
            path,
            vec![
                ColumnFamilyDescriptor::new(CF_TREE_LEVELS, tree_opts),
                ColumnFamilyDescriptor::new(CF_RAW_BRIDGE_LEAF, raw_opts),
                ColumnFamilyDescriptor::new(CF_METADATA, meta_opts),
            ],
        )?;

        // Default leaf is the zero hash.
        let mut zero = [FixedBytes::<32>::from([0u8; 32]); DEPTH + 1];

        // Precompute the zero hashes for each level.
        // Start at [1] because [0] is the default leaf.
        for i in 1..=DEPTH {
            zero[i] = hash(&zero[i - 1], &zero[i - 1]);
        }

        Ok(Self { db, zero })
    }

    fn cf_trees(&self) -> Result<&ColumnFamily> {
        self.db
            .cf_handle(CF_TREE_LEVELS)
            .ok_or_else(|| eyre!("CF 'trees' not found"))
    }

    fn cf_meta(&self) -> Result<&ColumnFamily> {
        self.db
            .cf_handle(CF_METADATA)
            .ok_or_else(|| eyre!("CF 'meta' not found"))
    }

    // TODO: This should be u64?
    pub fn get_leaf_count(&self, tree_type: &TreeType) -> Result<u32> {
        let index = self
            .db
            .get_cf(self.cf_meta()?, &meta_key(&tree_type, MetaTag::LeafCount))?
            .map(|v| u32::from_be_bytes(v[..4].try_into().unwrap()))
            .unwrap_or(0);
        Ok(index)
    }

    fn get_hash(
        &self,
        tree_type: TreeType,
        level: u8,
        index: u32,
    ) -> Result<Option<FixedBytes<32>>> {
        // Get the hash of this tree at this level and index.
        let opt = self
            .db
            .get_cf(self.cf_trees()?, &node_key(&tree_type, level, index))?;

        // Parse bytes to hash
        Ok(opt.map(|bytes| {
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .expect("stored hash must be exactly 32 bytes");
            FixedBytes::from(arr)
        }))
    }

    /// Get the latest block number for this tree
    pub fn get_latest_block(&self, tree_type: TreeType) -> Result<Option<BlockNum>> {
        Ok(self
            .db
            .get_cf(
                self.cf_meta()?,
                &meta_key(&tree_type, MetaTag::LatestProcessedBlock),
            )?
            .map(|v| u64::from_be_bytes(v[..8].try_into().unwrap())))
    }

    pub fn get_root(&self, tree_type: &TreeType) -> Result<Option<FixedBytes<32>>> {
        // If the tree is empty return None. Otherwise can be missleading.
        let leaf_count = self.get_leaf_count(tree_type)?;
        if leaf_count == 0 {
            return Ok(None);
        }

        // The root is always at index 0 at the last level
        if let Some(v) = self
            .db
            .get_cf(self.cf_trees()?, &node_key(&tree_type, DEPTH as u8, 0))?
        {
            Ok(Some(FixedBytes::<32>::from(
                <[u8; 32]>::try_from(&v[..]).unwrap(),
            )))
        } else {
            Ok(Some(self.zero[DEPTH]))
        }
    }

    fn put_deposit_count(
        &self,
        batch: &mut WriteBatch,
        tree_type: TreeType,
        num_leaves: u32,
    ) -> Result<()> {
        batch.put_cf(
            self.cf_meta()?,
            &meta_key(&tree_type, MetaTag::LeafCount),
            &num_leaves.to_be_bytes(),
        );
        Ok(())
    }

    fn put_level(
        &self,
        batch: &mut WriteBatch,
        tree_type: TreeType,
        level: usize,
        index: u32,
        node: &FixedBytes<32>,
    ) -> Result<()> {
        // TODO: use batches
        self.db.put_cf(
            self.cf_trees()?,
            node_key(&tree_type, level as u8, index),
            node.as_slice(),
        )?;
        Ok(())
    }

    fn put_block_number(
        &self,
        batch: &mut WriteBatch,
        tree_type: TreeType,
        block_number: BlockNum,
    ) -> Result<()> {
        batch.put_cf(
            self.cf_meta()?,
            &meta_key(&tree_type, MetaTag::LatestProcessedBlock),
            &block_number.to_be_bytes(),
        );
        Ok(())
    }

    // Append events to the Local Exit Tree of a specific Aggchain.
    // TODO: Allow for batch insertions.
    // TODO: Perform some extra checks.
    pub fn append_events(
        &self,
        aggchain_id: AggchainId,
        leaves: &[LeafBridge],
        block: BlockNum,
    ) -> eyre::Result<()> {
        if !leaves
            .windows(2)
            .all(|w| w[0].bridge_event.depositCount + 1 == w[1].bridge_event.depositCount)
        {
            return Err(eyre!("depositCount is not increasing monotonically"));
        }

        // Ensure we don't store more than possible. Both are u32, should never happen.
        let mut index = self.get_leaf_count(&TreeType::LocalExitTree(aggchain_id))?;
        if index >= MAX_LEAVES {
            return Err(eyre!(
                "Deposit count exceeds maximum limit {} >= {}",
                index,
                MAX_LEAVES
            ));
        }

        // The local index shall match the first leaf's deposit count.
        if index
            != (leaves
                .first()
                .ok_or(eyre!("No leaves provided"))?
                .bridge_event
                .depositCount)
        {
            return Err(eyre!(
                "Deposit count mismatch {} != {}",
                index,
                leaves.first().unwrap().bridge_event.depositCount
            ));
        }

        if leaves.len() > 1 {
            return Err(eyre!("Only one leaf can be inserted at a time by now"));
        }

        // TODO: Write in batches is blocked by this: https://github.com/rust-rocksdb/rust-rocksdb/pull/921
        // By now you can't read your own writes within a batch.
        let mut batch = WriteBatch::default();

        // TODO: Only one leaf is supported by now
        for leaf in leaves {
            let mut node = leaf.hashed_leaf();
            self.put_level(
                &mut batch,
                TreeType::LocalExitTree(aggchain_id),
                0,
                index,
                &node,
            )?;

            // Hash all the way up to the root.
            for level in 0..DEPTH {
                let left;
                let right;

                if index % 2 == 0 {
                    // left node
                    left = node;
                    right = self
                        .get_hash(TreeType::LocalExitTree(aggchain_id), level as u8, index + 1)?
                        .unwrap_or(self.zero[level]);
                } else {
                    // right node
                    left = self
                        .get_hash(TreeType::LocalExitTree(aggchain_id), level as u8, index - 1)?
                        .unwrap_or(self.zero[level]);
                    right = node;
                };

                node = hash(&left, &right);
                index /= 2;
                self.put_level(
                    &mut batch,
                    TreeType::LocalExitTree(aggchain_id),
                    level + 1,
                    index,
                    &node,
                )?;
            }

            index += 1;
        }

        // Persist meta: number of leaves + latest block
        self.put_deposit_count(
            &mut batch,
            TreeType::LocalExitTree(aggchain_id),
            leaves
                .last()
                .ok_or(eyre!("No leaves provided"))?
                .bridge_event
                .depositCount
                + 1,
        )?;
        self.put_block_number(&mut batch, TreeType::LocalExitTree(aggchain_id), block)?;

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);
        self.db.write_opt(batch, &write_opts)?;
        Ok(())
    }

    // Stores a Rollup's Local Exit Root at a specific index in the Rollup Exit Tree.
    pub fn set_rollup_leaf(
        &self,
        index: u32,
        leaf: &FixedBytes<32>,
        block: BlockNum,
    ) -> Result<()> {
        // Aggchain index is shifted by 1. Meaning Aggchain 1 is placed at index 0, etc.
        let index = index
            .checked_sub(1)
            .ok_or_else(|| eyre!("index cannot be 0"))?;

        if index >= MAX_LEAVES {
            return Err(eyre!(
                "Index {} exceeds the maximum number of leaves ({})",
                index,
                MAX_LEAVES
            ));
        }

        // Retrieve the current leaf count from the metadata
        let current_count = self.get_leaf_count(&TreeType::RollupExitTree)?;

        // Update the leaf count only if the new index is higher
        let new_count = (index + 1).max(current_count);

        // TODO: Use batches
        let mut batch = WriteBatch::default();

        // Level-0 write (the leaf itself)
        let mut node = *leaf;
        self.put_level(&mut batch, TreeType::RollupExitTree, 0, index, &node)?;

        let mut idx = index;
        for level in 0..DEPTH {
            let (left, right) = if idx % 2 == 0 {
                (
                    node,
                    self.get_hash(TreeType::RollupExitTree, level as u8, idx + 1)?
                        .unwrap_or(self.zero[level]),
                )
            } else {
                (
                    self.get_hash(TreeType::RollupExitTree, level as u8, idx - 1)?
                        .unwrap_or(self.zero[level]),
                    node,
                )
            };

            node = hash(&left, &right);
            idx /= 2;
            self.put_level(&mut batch, TreeType::RollupExitTree, level + 1, idx, &node)?;
        }

        // Update the leaf count in the metadata
        self.put_deposit_count(&mut batch, TreeType::RollupExitTree, new_count)?;
        self.put_block_number(&mut batch, TreeType::RollupExitTree, block)?;

        // TODO: Ensure when updating block number there are no future transactions
        // with the same one. Otherwise it won't be really in sync until that block.

        let mut wo = WriteOptions::default();
        wo.set_sync(false);
        self.db.write_opt(batch, &wo)?;
        Ok(())
    }

    // TODO: Dirty function, lots of copy pasted logic from append_events. Generalize.
    pub fn append_l1info_leaf(&self, leaf: &FixedBytes<32>, block: BlockNum) -> eyre::Result<()> {
        let mut index = self.get_leaf_count(&TreeType::L1InfoTree)?;
        let mut batch = WriteBatch::default();
        let mut node = leaf.clone();

        self.put_level(&mut batch, TreeType::L1InfoTree, 0, index, &node)?;

        for level in 0..DEPTH {
            let left;
            let right;

            if index % 2 == 0 {
                // left node
                left = node;
                right = self
                    .get_hash(TreeType::L1InfoTree, level as u8, index + 1)?
                    .unwrap_or(self.zero[level]);
            } else {
                // right node
                left = self
                    .get_hash(TreeType::L1InfoTree, level as u8, index - 1)?
                    .unwrap_or(self.zero[level]);
                right = node;
            };

            node = hash(&left, &right);
            index /= 2;
            self.put_level(&mut batch, TreeType::L1InfoTree, level + 1, index, &node)?;
        }

        // Persist meta: number of leaves + latest block
        self.put_deposit_count(
            &mut batch,
            TreeType::L1InfoTree,
            self.get_leaf_count(&TreeType::L1InfoTree)? + 1,
        )?;

        // TODO: Ensure when updating block number there are no future transactions
        // with the same one. Otherwise it won't be really in sync until that block.
        self.put_block_number(&mut batch, TreeType::L1InfoTree, block)?;

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);
        self.db.write_opt(batch, &write_opts)?;
        Ok(())
    }

    pub fn merkle_proof(&self, tree_type: TreeType, index: u64) -> Result<[FixedBytes<32>; DEPTH]> {
        let mut index = index;
        match tree_type {
            TreeType::RollupExitTree => {
                // TODO: Maybe dirty. In the Rollup Exit Tree, aggchain 1 is placed at index 0, etc.
                index = index - 1;
            }
            _ => {}
        }
        let mut proof = [FixedBytes::<32>::default(); DEPTH];
        let mut idx = index;
        for level in 0..DEPTH {
            let sib = idx ^ 1;
            proof[level] = match self.db.get_cf(
                self.cf_trees()?,
                &node_key(&tree_type, level as u8, sib as u32),
            )? {
                Some(v) => FixedBytes::<32>::from(<[u8; 32]>::try_from(&v[..]).unwrap()),
                None => self.zero[level],
            };
            idx >>= 1;
        }
        Ok(proof)
    }
}

#[inline(always)]
fn hash(l: &FixedBytes<32>, r: &FixedBytes<32>) -> FixedBytes<32> {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(l.as_slice());
    buf[32..].copy_from_slice(r.as_slice());
    keccak256(buf)
}

pub fn calculate_merkle_root(
    leaf: &FixedBytes<32>,
    proof: &[FixedBytes<32>; DEPTH],
    index: u64,
) -> FixedBytes<32> {
    let mut current_hash = *leaf;
    let mut idx = index;

    for sibling_hash in proof {
        if idx % 2 == 0 {
            current_hash = hash(&current_hash, sibling_hash);
        } else {
            current_hash = hash(sibling_hash, &current_hash);
        }
        idx >>= 1;
    }

    current_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hash() -> Result<(), eyre::Error> {
        let _ = std::fs::remove_dir_all("db_test");
        let t = MerkleForest::open("db_test").unwrap();
        let hash = t.get_hash(TreeType::RollupExitTree, 0, 0).unwrap();
        assert_eq!(hash, None);
        Ok(())
    }
}
