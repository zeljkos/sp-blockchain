use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use crate::hash::Blake2bHash;
use super::NetworkMessage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Vote {
    pub validator_id: String,
    pub block_hash: Blake2bHash,
    pub approve: bool,
    pub signature: Vec<u8>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ConsensusRound {
    pub block_hash: Blake2bHash,
    pub votes: HashMap<String, Vote>,
    pub started_at: SystemTime,
    pub finalized: bool,
    pub result: Option<bool>, // Some(true) = approved, Some(false) = rejected
}

pub struct SimpleConsensus {
    // Known SP validators in the consortium
    validators: HashMap<String, ValidatorInfo>,
    // Active consensus rounds
    active_rounds: HashMap<Blake2bHash, ConsensusRound>,
    // Configuration
    config: ConsensusConfig,
}

#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub node_id: String,
    pub stake_weight: u64, // For weighted voting (could be based on network size)
    pub public_key: Vec<u8>, // For signature verification
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    pub min_validators: usize,
    pub approval_threshold: f64, // 0.67 = 67% approval needed
    pub timeout_duration: Duration,
    pub max_concurrent_rounds: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_validators: 3, // At least 3 of 5 SP nodes
            approval_threshold: 0.67, // 67% approval (4/5 or 3/4)
            timeout_duration: Duration::from_secs(30),
            max_concurrent_rounds: 10,
        }
    }
}

impl SimpleConsensus {
    pub fn new(config: ConsensusConfig) -> Self {
        let mut validators = HashMap::new();

        // Initialize known SP consortium validators
        let sp_operators = vec![
            ("tmobile-de", 100),
            ("vodafone-uk", 100),
            ("orange-fr", 100),
            ("telenor-no", 100),
            ("sfr-fr", 100),
        ];

        for (node_id, stake) in sp_operators {
            validators.insert(
                node_id.to_string(),
                ValidatorInfo {
                    node_id: node_id.to_string(),
                    stake_weight: stake,
                    public_key: vec![0u8; 32], // TODO: Real public keys
                    is_active: true,
                },
            );
        }

        println!("🏛️  Consensus initialized with {} validators", validators.len());

        Self {
            validators,
            active_rounds: HashMap::new(),
            config,
        }
    }

    /// Start a new consensus round for a block
    pub fn start_consensus(&mut self, block_hash: Blake2bHash) -> Result<(), ConsensusError> {
        if self.active_rounds.contains_key(&block_hash) {
            return Err(ConsensusError::RoundAlreadyExists);
        }

        if self.active_rounds.len() >= self.config.max_concurrent_rounds {
            return Err(ConsensusError::TooManyActiveRounds);
        }

        let round = ConsensusRound {
            block_hash,
            votes: HashMap::new(),
            started_at: SystemTime::now(),
            finalized: false,
            result: None,
        };

        self.active_rounds.insert(block_hash, round);
        println!("🗳️  Started consensus round for block: {}", hex::encode(block_hash.as_bytes()));

        Ok(())
    }

    /// Process a vote from a validator
    pub fn process_vote(&mut self, vote: Vote) -> Result<ConsensusResult, ConsensusError> {
        // Validate the validator
        if !self.validators.contains_key(&vote.validator_id) {
            return Err(ConsensusError::UnknownValidator(vote.validator_id));
        }

        let validator = &self.validators[&vote.validator_id];
        if !validator.is_active {
            return Err(ConsensusError::InactiveValidator(vote.validator_id));
        }

        // TODO: Verify signature
        // if !self.verify_signature(&vote) {
        //     return Err(ConsensusError::InvalidSignature);
        // }

        // Get the consensus round
        let round = self.active_rounds.get_mut(&vote.block_hash)
            .ok_or(ConsensusError::NoActiveRound)?;

        if round.finalized {
            return Err(ConsensusError::RoundAlreadyFinalized);
        }

        // Record the vote
        println!("📝 Recording vote from {}: {} for block {}",
                 vote.validator_id,
                 if vote.approve { "APPROVE" } else { "REJECT" },
                 hex::encode(vote.block_hash.as_bytes()));

        let block_hash = vote.block_hash;
        round.votes.insert(vote.validator_id.clone(), vote);

        // Check if we have reached consensus
        self.check_consensus(block_hash)
    }

    /// Check if consensus has been reached for a block
    fn check_consensus(&mut self, block_hash: Blake2bHash) -> Result<ConsensusResult, ConsensusError> {
        let round = self.active_rounds.get_mut(&block_hash)
            .ok_or(ConsensusError::NoActiveRound)?;

        if round.finalized {
            return Ok(ConsensusResult::AlreadyFinalized(round.result));
        }

        let total_validators = self.validators.len();
        let active_validators = self.validators.values().filter(|v| v.is_active).count();

        // Check if we have minimum participation
        if round.votes.len() < self.config.min_validators {
            return Ok(ConsensusResult::InProgress {
                votes_received: round.votes.len(),
                votes_needed: self.config.min_validators,
            });
        }

        // Calculate approval rate
        let approvals = round.votes.values().filter(|v| v.approve).count();
        let rejections = round.votes.values().filter(|v| !v.approve).count();
        let total_votes = approvals + rejections;

        let approval_rate = approvals as f64 / total_votes as f64;

        println!("📊 Consensus check: {}/{} approvals ({}%)",
                 approvals, total_votes, (approval_rate * 100.0) as u32);

        // Check if we have enough votes and meet threshold
        if total_votes >= active_validators || approval_rate >= self.config.approval_threshold {
            let approved = approval_rate >= self.config.approval_threshold;

            round.finalized = true;
            round.result = Some(approved);

            println!("✅ Consensus reached for block {}: {}",
                     hex::encode(block_hash.as_bytes()),
                     if approved { "APPROVED" } else { "REJECTED" });

            Ok(ConsensusResult::Finalized { approved })
        } else {
            Ok(ConsensusResult::InProgress {
                votes_received: total_votes,
                votes_needed: active_validators,
            })
        }
    }

    /// Check for expired consensus rounds
    pub fn cleanup_expired_rounds(&mut self) {
        let now = SystemTime::now();
        let mut expired_rounds = Vec::new();

        for (block_hash, round) in &self.active_rounds {
            if let Ok(elapsed) = now.duration_since(round.started_at) {
                if elapsed > self.config.timeout_duration && !round.finalized {
                    expired_rounds.push(*block_hash);
                }
            }
        }

        for block_hash in expired_rounds {
            println!("⏰ Consensus round expired for block: {}", hex::encode(block_hash.as_bytes()));
            if let Some(mut round) = self.active_rounds.remove(&block_hash) {
                round.finalized = true;
                round.result = Some(false); // Timeout = rejection
            }
        }
    }

    /// Get the status of all active consensus rounds
    pub fn get_active_rounds(&self) -> Vec<(&Blake2bHash, &ConsensusRound)> {
        self.active_rounds.iter().collect()
    }

    /// Get validator information
    pub fn get_validators(&self) -> &HashMap<String, ValidatorInfo> {
        &self.validators
    }
}

#[derive(Debug)]
pub enum ConsensusResult {
    InProgress {
        votes_received: usize,
        votes_needed: usize,
    },
    Finalized {
        approved: bool,
    },
    AlreadyFinalized(Option<bool>),
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Consensus round already exists for this block")]
    RoundAlreadyExists,

    #[error("Too many active consensus rounds")]
    TooManyActiveRounds,

    #[error("Unknown validator: {0}")]
    UnknownValidator(String),

    #[error("Inactive validator: {0}")]
    InactiveValidator(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("No active consensus round for this block")]
    NoActiveRound,

    #[error("Consensus round already finalized")]
    RoundAlreadyFinalized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_consensus() {
        let mut consensus = SimpleConsensus::new(ConsensusConfig::default());
        let block_hash = Blake2bHash::hash(b"test_block");

        // Start consensus
        consensus.start_consensus(block_hash).unwrap();

        // Create votes
        let vote1 = Vote {
            validator_id: "tmobile-de".to_string(),
            block_hash,
            approve: true,
            signature: vec![],
            timestamp: SystemTime::now(),
        };

        let vote2 = Vote {
            validator_id: "vodafone-uk".to_string(),
            block_hash,
            approve: true,
            signature: vec![],
            timestamp: SystemTime::now(),
        };

        let vote3 = Vote {
            validator_id: "orange-fr".to_string(),
            block_hash,
            approve: true,
            signature: vec![],
            timestamp: SystemTime::now(),
        };

        // Process votes
        consensus.process_vote(vote1).unwrap();
        consensus.process_vote(vote2).unwrap();
        let result = consensus.process_vote(vote3).unwrap();

        // Should reach consensus with 3/3 approval
        match result {
            ConsensusResult::Finalized { approved } => assert!(approved),
            _ => panic!("Expected consensus to be finalized"),
        }
    }
}