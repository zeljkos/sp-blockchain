use crate::hash::Blake2bHash;
use std::path::Path;

pub struct SettlementStore {
    data_dir: std::path::PathBuf,
}

impl SettlementStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("blocks"))?;
        std::fs::create_dir_all(data_dir.join("records"))?;

        Ok(Self { data_dir })
    }

    pub fn store_block(&self, hash: &Blake2bHash, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.data_dir.join("blocks").join(format!("{}.block", hex::encode(hash.as_bytes())));
        std::fs::write(file_path, block_data)?;
        Ok(())
    }

    pub fn get_block(&self, hash: &Blake2bHash) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let file_path = self.data_dir.join("blocks").join(format!("{}.block", hex::encode(hash.as_bytes())));
        match std::fs::read(file_path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn store_bce_record(&self, record_id: &str, record_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.data_dir.join("records").join(format!("{}.json", record_id));
        std::fs::write(file_path, record_data)?;
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChainStateData {
    pub height: u32,
    pub head_hash: Blake2bHash,
    pub total_settlements: u64,
    pub total_value_cents: u64,
    pub last_settlement_time: u64,
}