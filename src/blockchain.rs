use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::hash::Blake2bHash;
use crate::storage::rocks_store::RocksSettlementStore;
use crate::network::p2p::{P2PNetwork, BlockchainNetwork};
use crate::network::consensus::{SimpleConsensus, Vote, ConsensusConfig};
use crate::zkp::{SettlementProofSystem, SettlementProof};
use crate::smart_contracts::contract_api::{ContractAPI, SettlementRequest};

/// Core blockchain for SP settlement records
pub struct SPBlockchain {
    pub storage: Arc<RocksSettlementStore>,
    pub p2p_network: Arc<RwLock<P2PNetwork>>,
    pub consensus: Arc<RwLock<SimpleConsensus>>,
    pub zkp_system: SettlementProofSystem,
    pub smart_contracts: ContractAPI,
    pub node_id: String,
    pub pending_records: Arc<RwLock<HashMap<String, BceRecord>>>,
    pub blocks: Arc<RwLock<Vec<SettlementBlock>>>,
}

/// BCE record structure for telecom settlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BceRecord {
    pub record_id: String,
    pub imsi: String,
    pub home_operator: String,
    pub visited_operator: String,
    pub call_minutes: u32,
    pub data_mb: u32,
    pub sms_count: u32,
    pub call_rate_cents: u32,
    pub data_rate_cents: u32,
    pub sms_rate_cents: u32,
    pub wholesale_charge_cents: u32,
    pub timestamp: u64,
    pub zkp_proof: Option<SettlementProof>,
}

/// Settlement block containing multiple BCE records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBlock {
    pub block_hash: Blake2bHash,
    pub previous_hash: Blake2bHash,
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
    pub records: Vec<BceRecord>,
    pub settlement_summary: SettlementSummary,
    pub validator_signatures: Vec<ValidatorSignature>,
}

/// Summary of settlement totals in a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementSummary {
    pub total_records: u32,
    pub total_amount_cents: u64,
    pub operator_balances: HashMap<String, i64>,
    pub zkp_proof: Option<SettlementProof>,
}

/// Validator signature for block consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_id: String,
    pub signature: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

impl SPBlockchain {
    /// Create new SP blockchain instance
    pub async fn new(
        data_dir: &str,
        node_id: String,
        p2p_port: u16,
        bootstrap_peers: Vec<String>,
    ) -> Result<Self, BlockchainError> {
        println!("🔗 Initializing SP blockchain: {}", node_id);

        // Initialize RocksDB storage
        let storage = Arc::new(RocksSettlementStore::new(data_dir)?);
        println!("💾 RocksDB storage initialized at: {}", data_dir);

        // Initialize P2P network
        let p2p_network = Arc::new(RwLock::new(
            P2PNetwork::new(p2p_port, bootstrap_peers).await?
        ));
        println!("🌐 P2P network initialized on port: {}", p2p_port);

        // Initialize consensus system
        let validators = vec![
            "tmobile-de".to_string(),
            "vodafone-uk".to_string(),
            "orange-fr".to_string(),
            "telenor-no".to_string(),
            "sfr-fr".to_string(),
        ];

        let consensus_config = ConsensusConfig {
            validators: validators.clone(),
            approval_threshold: 0.67, // 67% approval required
            timeout_seconds: 30,
        };

        let consensus = Arc::new(RwLock::new(
            SimpleConsensus::new(node_id.clone(), consensus_config)
        ));
        println!("⚖️  Consensus system initialized with {} validators", validators.len());

        // Initialize ZKP system
        let zkp_system = SettlementProofSystem::new(&node_id)?;
        println!("🛡️  ZKP system initialized");

        // Initialize smart contracts
        let smart_contracts = ContractAPI::new()?;
        println!("📋 Smart contract system initialized");

        Ok(Self {
            storage,
            p2p_network,
            consensus,
            zkp_system,
            smart_contracts,
            node_id,
            pending_records: Arc::new(RwLock::new(HashMap::new())),
            blocks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Submit BCE record to blockchain
    pub async fn submit_bce_record(&self, mut record: BceRecord) -> Result<String, BlockchainError> {
        println!("📝 Processing BCE record: {}", record.record_id);

        // Validate record
        self.validate_bce_record(&record)?;

        // Generate ZKP proof for privacy
        let zkp_proof = self.zkp_system.prove_bce_settlement(
            record.wholesale_charge_cents as u64,
            vec![record.call_minutes as u64, record.data_mb as u64, record.sms_count as u64],
            vec![record.call_rate_cents as u64, record.data_rate_cents as u64, record.sms_rate_cents as u64],
            &record.record_id,
        )?;

        record.zkp_proof = Some(zkp_proof);
        println!("🔐 ZKP proof generated for record");

        // Store in RocksDB database
        self.storage.store_bce_record(&record)?;
        println!("💾 Record stored in RocksDB database");

        // Add to pending records for consensus
        {
            let mut pending = self.pending_records.write().await;
            pending.insert(record.record_id.clone(), record.clone());
        }

        // BCE records are local ingestion data - do NOT broadcast
        // Only settlement blocks should be broadcast to the consortium
        println!("💾 Record stored locally (BCE records are not broadcast)");

        Ok(record.record_id)
    }

    /// Broadcast BCE record to all validators
    async fn broadcast_bce_record(&self, record: BceRecord) -> Result<(), BlockchainError> {
        let mut network = self.p2p_network.write().await;
        let message = serde_json::to_string(&record)?;

        network.broadcast_message("bce_record".to_string(), message.into_bytes()).await?;
        println!("📡 BCE record broadcasted to {} peers", network.connected_peers().await);

        Ok(())
    }

    /// Process incoming BCE record from peer
    pub async fn process_peer_record(&self, record: BceRecord) -> Result<(), BlockchainError> {
        println!("📥 Received BCE record from peer: {}", record.record_id);

        // Validate record and ZKP proof
        self.validate_bce_record(&record)?;

        if let Some(ref proof) = record.zkp_proof {
            if !self.zkp_system.verify_proof(proof)? {
                return Err(BlockchainError::InvalidZKPProof);
            }
        }

        // Store in RocksDB database
        self.storage.store_bce_record(&record)?;

        // Add to pending records
        {
            let mut pending = self.pending_records.write().await;
            pending.insert(record.record_id.clone(), record);
        }

        // Check if we can create a settlement block
        self.try_create_settlement_block().await?;

        Ok(())
    }

    /// Try to create settlement block when threshold reached
    async fn try_create_settlement_block(&self) -> Result<(), BlockchainError> {
        let pending_count = {
            let pending = self.pending_records.read().await;
            pending.len()
        };

        // Create block when we have 10+ records or after timeout
        if pending_count >= 10 {
            self.create_settlement_block().await?;
        }

        Ok(())
    }

    /// Create settlement block with consensus
    async fn create_settlement_block(&self) -> Result<SettlementBlock, BlockchainError> {
        println!("🔨 Creating settlement block");

        // Collect pending records
        let records = {
            let mut pending = self.pending_records.write().await;
            let records: Vec<BceRecord> = pending.values().cloned().collect();
            pending.clear();
            records
        };

        if records.is_empty() {
            return Err(BlockchainError::NoPendingRecords);
        }

        // Calculate settlement summary
        let settlement_summary = self.calculate_settlement_summary(&records)?;

        // Get previous block hash
        let previous_hash = {
            let blocks = self.blocks.read().await;
            blocks.last()
                .map(|b| b.block_hash)
                .unwrap_or_else(|| Blake2bHash::hash(b"genesis"))
        };

        let block_number = {
            let blocks = self.blocks.read().await;
            blocks.len() as u64
        };

        // Create block
        let mut block = SettlementBlock {
            block_hash: Blake2bHash::hash(b"placeholder"), // Will be calculated
            previous_hash,
            block_number,
            timestamp: Utc::now(),
            records,
            settlement_summary,
            validator_signatures: Vec::new(),
        };

        // Calculate actual block hash
        let block_data = serde_json::to_vec(&block)?;
        block.block_hash = Blake2bHash::hash(&block_data);

        // Start consensus voting
        let vote = Vote {
            block_hash: block.block_hash,
            validator_id: self.node_id.clone(),
            approve: true,
            signature: vec![], // Simplified signature for SP consortium
            timestamp: SystemTime::now(),
        };

        // Submit vote and broadcast
        {
            let mut consensus = self.consensus.write().await;
            consensus.submit_vote(vote.clone())?;
        }

        self.broadcast_vote(vote).await?;

        // Store block in RocksDB
        self.storage.store_settlement_block(&block)?;

        // Add to local blocks
        {
            let mut blocks = self.blocks.write().await;
            blocks.push(block.clone());
        }

        println!("✅ Settlement block {} created with {} records",
                 block.block_number, block.records.len());

        Ok(block)
    }

    /// Calculate settlement summary for block
    fn calculate_settlement_summary(&self, records: &[BceRecord]) -> Result<SettlementSummary, BlockchainError> {
        let mut operator_balances: HashMap<String, i64> = HashMap::new();
        let mut total_amount_cents = 0u64;

        for record in records {
            total_amount_cents += record.wholesale_charge_cents as u64;

            // Home operator owes money (negative)
            let home_balance = operator_balances.entry(record.home_operator.clone()).or_insert(0);
            *home_balance -= record.wholesale_charge_cents as i64;

            // Visited operator receives money (positive)
            let visited_balance = operator_balances.entry(record.visited_operator.clone()).or_insert(0);
            *visited_balance += record.wholesale_charge_cents as i64;
        }

        // Generate ZKP proof for settlement totals
        let amounts: Vec<u64> = records.iter().map(|r| r.wholesale_charge_cents as u64).collect();
        let rates: Vec<u64> = vec![1; amounts.len()]; // Simplified for settlement proof

        let zkp_proof = self.zkp_system.prove_bce_settlement(
            total_amount_cents,
            amounts,
            rates,
            &format!("settlement_block_{}", chrono::Utc::now().timestamp()),
        ).ok();

        Ok(SettlementSummary {
            total_records: records.len() as u32,
            total_amount_cents,
            operator_balances,
            zkp_proof,
        })
    }

    /// Broadcast consensus vote
    async fn broadcast_vote(&self, vote: Vote) -> Result<(), BlockchainError> {
        let mut network = self.p2p_network.write().await;
        let message = serde_json::to_string(&vote)?;

        network.broadcast_message("consensus_vote".to_string(), message.into_bytes()).await?;

        Ok(())
    }

    /// Validate BCE record
    fn validate_bce_record(&self, record: &BceRecord) -> Result<(), BlockchainError> {
        if record.record_id.is_empty() {
            return Err(BlockchainError::InvalidRecord("Missing record ID".to_string()));
        }

        if record.imsi.is_empty() {
            return Err(BlockchainError::InvalidRecord("Missing IMSI".to_string()));
        }

        // Validate rate calculation
        let calculated_charge = record.call_minutes * record.call_rate_cents
            + record.data_mb * record.data_rate_cents
            + record.sms_count * record.sms_rate_cents;

        let variance = if calculated_charge > record.wholesale_charge_cents {
            calculated_charge - record.wholesale_charge_cents
        } else {
            record.wholesale_charge_cents - calculated_charge
        };

        // Allow small variance for realistic billing
        if variance > 50 { // 50 cents tolerance
            return Err(BlockchainError::InvalidRecord(
                format!("Charge mismatch: calculated {}, actual {}", calculated_charge, record.wholesale_charge_cents)
            ));
        }

        Ok(())
    }

    /// Get blockchain statistics
    pub async fn get_stats(&self) -> BlockchainStats {
        let blocks = self.blocks.read().await;
        let pending = self.pending_records.read().await;

        let total_records: u32 = blocks.iter().map(|b| b.records.len() as u32).sum();
        let total_amount: u64 = blocks.iter().map(|b| b.settlement_summary.total_amount_cents).sum();

        BlockchainStats {
            total_blocks: blocks.len(),
            total_records,
            pending_records: pending.len(),
            total_settlement_amount_cents: total_amount,
            last_block_time: blocks.last().map(|b| b.timestamp),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BlockchainStats {
    pub total_blocks: usize,
    pub total_records: u32,
    pub pending_records: usize,
    pub total_settlement_amount_cents: u64,
    pub last_block_time: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Storage error: {0}")]
    Storage(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Invalid ZKP proof")]
    InvalidZKPProof,

    #[error("Invalid record: {0}")]
    InvalidRecord(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("ZKP error: {0}")]
    ZKP(String),

    #[error("No pending records to create block")]
    NoPendingRecords,
}

// Convert ZKP errors
impl From<crate::zkp::ZkpError> for BlockchainError {
    fn from(err: crate::zkp::ZkpError) -> Self {
        BlockchainError::ZKP(err.to_string())
    }
}