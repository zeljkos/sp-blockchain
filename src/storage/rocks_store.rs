use crate::simple_blockchain::{BceRecord, SettlementBlock};
use std::path::Path;
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::sync::Arc;

/// Persistent storage for SP blockchain using RocksDB
pub struct RocksSettlementStore {
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

impl RocksSettlementStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, RocksError> {
        let data_dir = path.as_ref().to_path_buf();

        // RocksDB options
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new("bce_records", Options::default()),
            ColumnFamilyDescriptor::new("settlement_blocks", Options::default()),
        ];

        // Open database with column families
        let db = DB::open_cf_descriptors(&opts, &data_dir, cf_descriptors)?;

        println!("🗄️  RocksDB persistent storage initialized at: {}", data_dir.display());

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Store BCE record persistently
    pub fn store_bce_record(&self, record: &BceRecord) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("bce_records").ok_or_else(|| {
            RocksError::Other("bce_records column family not found".to_string())
        })?;

        let key = record.record_id.as_bytes();
        let value = serde_json::to_vec(record)?;

        self.db.put_cf(&cf, key, value)?;

        println!("💾 Stored BCE record: {} in RocksDB", record.record_id);
        Ok(())
    }

    /// Store settlement block persistently
    pub fn store_settlement_block(&self, block: &SettlementBlock) -> Result<(), RocksError> {
        let cf = self.db.cf_handle("settlement_blocks").ok_or_else(|| {
            RocksError::Other("settlement_blocks column family not found".to_string())
        })?;

        let key = format!("block_{:08}", block.block_number);
        let value = serde_json::to_vec(block)?;

        self.db.put_cf(&cf, key.as_bytes(), value)?;

        println!("🧱 Stored settlement block #{} in RocksDB", block.block_number);
        Ok(())
    }

    /// Get BCE record from persistent storage
    pub fn get_bce_record(&self, record_id: &str) -> Result<Option<BceRecord>, RocksError> {
        let cf = self.db.cf_handle("bce_records").ok_or_else(|| {
            RocksError::Other("bce_records column family not found".to_string())
        })?;

        let key = record_id.as_bytes();

        match self.db.get_cf(&cf, key)? {
            Some(data) => {
                let record: BceRecord = serde_json::from_slice(&data)?;
                Ok(Some(record))
            },
            None => Ok(None),
        }
    }

    /// Get settlement block from persistent storage
    pub fn get_settlement_block(&self, block_number: u64) -> Result<Option<SettlementBlock>, RocksError> {
        let cf = self.db.cf_handle("settlement_blocks").ok_or_else(|| {
            RocksError::Other("settlement_blocks column family not found".to_string())
        })?;

        let key = format!("block_{:08}", block_number);

        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(data) => {
                let block: SettlementBlock = serde_json::from_slice(&data)?;
                Ok(Some(block))
            },
            None => Ok(None),
        }
    }

    /// Get all settlement blocks from persistent storage
    pub fn get_all_blocks(&self) -> Result<Vec<SettlementBlock>, RocksError> {
        let cf = self.db.cf_handle("settlement_blocks").ok_or_else(|| {
            RocksError::Other("settlement_blocks column family not found".to_string())
        })?;

        let mut blocks = Vec::new();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let block: SettlementBlock = serde_json::from_slice(&value)?;
            blocks.push(block);
        }

        // Sort by block number
        blocks.sort_by_key(|b| b.block_number);
        Ok(blocks)
    }

    /// Get all BCE records from persistent storage
    pub fn get_all_bce_records(&self) -> Result<Vec<BceRecord>, RocksError> {
        let cf = self.db.cf_handle("bce_records").ok_or_else(|| {
            RocksError::Other("bce_records column family not found".to_string())
        })?;

        let mut records = Vec::new();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let record: BceRecord = serde_json::from_slice(&value)?;
            records.push(record);
        }

        Ok(records)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<(usize, usize), RocksError> {
        let records = self.get_all_bce_records()?;
        let blocks = self.get_all_blocks()?;

        Ok((records.len(), blocks.len()))
    }

    /// List all stored data for debugging
    pub fn list_files(&self) -> Result<(), RocksError> {
        println!("📁 RocksDB storage contents:");

        let bce_cf = self.db.cf_handle("bce_records").ok_or_else(|| {
            RocksError::Other("bce_records column family not found".to_string())
        })?;

        let blocks_cf = self.db.cf_handle("settlement_blocks").ok_or_else(|| {
            RocksError::Other("settlement_blocks column family not found".to_string())
        })?;

        println!("  BCE Records:");
        let iter = self.db.iterator_cf(&bce_cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            println!("    - {}", key_str);
        }

        println!("  Settlement Blocks:");
        let iter = self.db.iterator_cf(&blocks_cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            println!("    - {}", key_str);
        }

        Ok(())
    }
}