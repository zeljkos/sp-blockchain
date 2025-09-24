use crate::hash::Blake2bHash;
use std::path::Path;
use rocksdb::{DB, Options, ColumnFamily, ColumnFamilyDescriptor};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// RocksDB-based blockchain storage using proper column families
pub struct RocksChainStore {
    db: Arc<DB>,
}

#[derive(Debug, thiserror::Error)]
pub enum RocksError {
    #[error("RocksDB error: {0}")]
    Rocks(#[from] rocksdb::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainState {
    pub height: u64,
    pub head_hash: Blake2bHash,
    pub total_blocks: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_blocks: usize,
    pub total_records: usize,
    pub total_settlements: usize,
    pub database_size: u64,
}

impl RocksChainStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, RocksError> {
        let data_dir = path.as_ref().to_path_buf();

        // RocksDB options
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families for different data types
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new("blocks", Options::default()),
            ColumnFamilyDescriptor::new("records", Options::default()),
            ColumnFamilyDescriptor::new("chain_state", Options::default()),
            ColumnFamilyDescriptor::new("settlements", Options::default()),
            ColumnFamilyDescriptor::new("block_index", Options::default()),
        ];

        // Open database with column families
        let db = DB::open_cf_descriptors(&opts, &data_dir, cf_descriptors)?;

        println!("🗄️  RocksDB Chain Store initialized at: {}", data_dir.display());

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Store block data using proper RocksDB key-value pattern
    pub fn store_block(&self, hash: &Blake2bHash, data: &[u8]) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("blocks").ok_or_else(|| {
            RocksError::Other("blocks column family not found".to_string())
        })?;

        let key = format!("block:{}", hex::encode(hash.as_bytes()));
        self.db.put_cf(cf, key.as_bytes(), data)?;

        // Update block index
        let index_cf = self.db.cf_handle("block_index").ok_or_else(|| {
            RocksError::Other("block_index column family not found".to_string())
        })?;

        let index_key = format!("idx:{}", self.get_next_block_number()?);
        self.db.put_cf(index_cf, index_key.as_bytes(), hash.as_bytes())?;

        println!("💾 RocksDB stored block: {}", hex::encode(hash.as_bytes()));
        Ok(())
    }

    /// Retrieve block data
    pub fn get_block(&self, hash: &Blake2bHash) -> Result<Option<Vec<u8>>, RocksError> {
        let cf = self.db.cf_handle("blocks").ok_or_else(|| {
            RocksError::Other("blocks column family not found".to_string())
        })?;

        let key = format!("block:{}", hex::encode(hash.as_bytes()));
        Ok(self.db.get_cf(cf, key.as_bytes())?)
    }

    /// Store BCE record with RocksDB key-value pattern
    pub fn store_bce_record(&self, record_id: &str, data: &[u8]) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("records").ok_or_else(|| {
            RocksError::Other("records column family not found".to_string())
        })?;

        let key = format!("record:{}", record_id);
        self.db.put_cf(cf, key.as_bytes(), data)?;

        println!("📝 RocksDB stored BCE record: {}", record_id);
        Ok(())
    }

    /// Retrieve BCE record
    pub fn get_bce_record(&self, record_id: &str) -> Result<Option<Vec<u8>>, RocksError> {
        let cf = self.db.cf_handle("records").ok_or_else(|| {
            RocksError::Other("records column family not found".to_string())
        })?;

        let key = format!("record:{}", record_id);
        Ok(self.db.get_cf(cf, key.as_bytes())?)
    }

    /// Update chain state
    pub fn update_chain_state(&self, height: u64, head_hash: Blake2bHash) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("chain_state").ok_or_else(|| {
            RocksError::Other("chain_state column family not found".to_string())
        })?;

        let state = ChainState {
            height,
            head_hash,
            total_blocks: height + 1,
        };

        let serialized = serde_json::to_vec(&state)?;
        self.db.put_cf(cf, b"current", serialized)?;

        println!("🔗 RocksDB updated chain state - Height: {}, Head: {}",
                 height, hex::encode(head_hash.as_bytes()));

        Ok(())
    }

    /// Get current chain state
    pub fn get_chain_state(&self) -> Result<Option<ChainState>, RocksError> {
        let cf = self.db.cf_handle("chain_state").ok_or_else(|| {
            RocksError::Other("chain_state column family not found".to_string())
        })?;

        if let Some(data) = self.db.get_cf(cf, b"current")? {
            let state: ChainState = serde_json::from_slice(&data)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Store settlement data
    pub fn store_settlement(&self, settlement_id: &str, data: &[u8]) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("settlements").ok_or_else(|| {
            RocksError::Other("settlements column family not found".to_string())
        })?;

        let key = format!("settlement:{}", settlement_id);
        self.db.put_cf(cf, key.as_bytes(), data)?;

        println!("💰 RocksDB stored settlement: {}", settlement_id);
        Ok(())
    }

    /// Get all block hashes in order
    pub fn get_all_block_hashes(&self) -> Result<Vec<Blake2bHash>, RocksError> {
        let cf = self.db.cf_handle("block_index").ok_or_else(|| {
            RocksError::Other("block_index column family not found".to_string())
        })?;

        let mut hashes = Vec::new();
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, value) = item?;
            if key.starts_with(b"idx:") {
                if value.len() == 32 {
                    let mut hash_bytes = [0u8; 32];
                    hash_bytes.copy_from_slice(&value);
                    hashes.push(Blake2bHash::from(hash_bytes));
                }
            }
        }

        // Sort by index number
        hashes.sort_by_key(|_| {
            // In a real implementation, we'd parse the index from the key
            // For now, maintain insertion order
            0
        });

        Ok(hashes)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats, RocksError> {
        let blocks_count = self.count_keys_in_cf("blocks", "block:")?;
        let records_count = self.count_keys_in_cf("records", "record:")?;
        let settlements_count = self.count_keys_in_cf("settlements", "settlement:")?;

        // Get approximate database size
        let mut database_size = 0u64;
        if let Ok(Some(size_str)) = self.db.property_value("rocksdb.total-sst-files-size") {
            database_size = size_str.parse().unwrap_or(0);
        }

        Ok(StorageStats {
            total_blocks: blocks_count,
            total_records: records_count,
            total_settlements: settlements_count,
            database_size,
        })
    }

    /// Helper to count keys with prefix in column family
    fn count_keys_in_cf(&self, cf_name: &str, prefix: &str) -> Result<usize, RocksError> {
        let cf = self.db.cf_handle(cf_name).ok_or_else(|| {
            RocksError::Other(format!("{} column family not found", cf_name))
        })?;

        let mut count = 0;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, _) = item?;
            if key.starts_with(prefix.as_bytes()) {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get next block number for indexing
    fn get_next_block_number(&self) -> Result<u64, RocksError> {
        if let Some(state) = self.get_chain_state()? {
            Ok(state.height + 1)
        } else {
            Ok(0)
        }
    }

    /// Sync and compact database
    pub fn sync(&self) -> Result<(), RocksError> {
        // Force a flush of memtables to SST files
        self.db.flush()?;

        // Compact all column families
        for cf_name in &["blocks", "records", "chain_state", "settlements", "block_index"] {
            if let Some(cf) = self.db.cf_handle(cf_name) {
                self.db.compact_range_cf(cf, None::<&[u8]>, None::<&[u8]>);
            }
        }

        println!("🔧 RocksDB Chain Store sync completed");
        Ok(())
    }
}

/// Type alias for compatibility
pub type SettlementStore = RocksChainStore;