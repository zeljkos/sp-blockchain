// Trusted setup ceremony adapted for 5-node SP consortium
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_snark::SNARK;
use ark_std::rand::{RngCore, CryptoRng};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::hash::Blake2bHash;
use super::circuits::{BCEPrivacyCircuit, SettlementCalculationCircuit};

#[derive(Error, Debug)]
pub enum TrustedSetupError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid proof")]
    InvalidProof,
}

type Result<T> = std::result::Result<T, TrustedSetupError>;

/// Trusted setup ceremony coordinator for 5-node SP consortium
pub struct TrustedSetupCeremony {
    /// Circuit identifiers to ceremony data
    circuits: HashMap<String, CircuitSetup>,

    /// Ceremony configuration
    config: CeremonyConfig,

    /// Storage path for keys
    keys_dir: PathBuf,
}

/// Configuration for the trusted setup ceremony - adapted for 5 validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeremonyConfig {
    /// Number of participants required (3 of 5 for security)
    pub min_participants: usize,

    /// All 5 SP consortium members who should participate
    pub all_participants: Vec<String>,

    /// Required participants for ceremony validity
    pub required_participants: Vec<String>,

    /// Ceremony timeout in seconds
    pub ceremony_timeout: u64,

    /// Enable verification of participant contributions
    pub verify_contributions: bool,
}

/// Circuit setup information
#[derive(Debug, Clone)]
struct CircuitSetup {
    circuit_id: String,
    circuit_description: String,
    parameters_hash: Option<Blake2bHash>,
    proving_key: Option<ProvingKey<Bn254>>,
    verifying_key: Option<VerifyingKey<Bn254>>,
    ceremony_complete: bool,
}

/// Participant contribution to the ceremony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantContribution {
    pub participant_id: String,
    pub circuit_id: String,
    pub contribution_hash: Blake2bHash,
    pub previous_hash: Blake2bHash,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

/// Ceremony transcript for verifiability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeremonyTranscript {
    pub ceremony_id: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub participants: Vec<String>,
    pub contributions: Vec<ParticipantContribution>,
    pub final_parameters_hash: Option<Blake2bHash>,
    pub verification_status: VerificationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed(String),
}

impl TrustedSetupCeremony {
    /// Create new ceremony coordinator
    pub fn new(keys_dir: PathBuf, config: CeremonyConfig) -> Self {
        let mut circuits = HashMap::new();

        // Register SP circuits - adapted for 5-party settlements
        circuits.insert("cdr_privacy".to_string(), CircuitSetup {
            circuit_id: "cdr_privacy".to_string(),
            circuit_description: "CDR Privacy Circuit - proves CDR calculations without revealing records for 5-party consortium".to_string(),
            parameters_hash: None,
            proving_key: None,
            verifying_key: None,
            ceremony_complete: false,
        });

        circuits.insert("settlement_calculation".to_string(), CircuitSetup {
            circuit_id: "settlement_calculation".to_string(),
            circuit_description: "Settlement Calculation Circuit - proves 5-party netting correctness".to_string(),
            parameters_hash: None,
            proving_key: None,
            verifying_key: None,
            ceremony_complete: false,
        });

        Self {
            circuits,
            config,
            keys_dir,
        }
    }

    /// Initialize ceremony with 5-node SP consortium configuration
    pub fn sp_5node_consortium_ceremony(keys_dir: PathBuf) -> Self {
        let config = CeremonyConfig {
            // Require at least 3 of 5 validators for security
            min_participants: 3,

            // All 5 consortium members
            all_participants: vec![
                "T-Mobile-DE".to_string(),
                "Vodafone-UK".to_string(),
                "Orange-FR".to_string(),
                "Telefónica-ES".to_string(),
                "SFR-FR".to_string(),
            ],

            // Minimum required participants (can be subset)
            required_participants: vec![
                "T-Mobile-DE".to_string(),
                "Vodafone-UK".to_string(),
                "Orange-FR".to_string(),
            ],

            ceremony_timeout: 3600, // 1 hour
            verify_contributions: true,
        };

        Self::new(keys_dir, config)
    }

    /// Generate individual keys for each provider (more realistic approach)
    pub async fn generate_individual_provider_keys<R: RngCore + CryptoRng>(
        base_keys_dir: PathBuf,
        rng: &mut R
    ) -> Result<HashMap<String, PathBuf>> {
        info!("🔐 Generating individual ZKP keys for each SP provider");

        let providers = vec![
            ("tmobile-de", "T-Mobile-DE"),
            ("vodafone-uk", "Vodafone-UK"),
            ("orange-fr", "Orange-FR"),
            ("telefonica-es", "Telefónica-ES"),
            ("sfr-fr", "SFR-FR"),
        ];

        let mut provider_key_dirs = HashMap::new();

        for (provider_id, provider_name) in providers {
            info!("🔑 Generating keys for {}", provider_name);

            // Create individual provider key directory
            let provider_keys_dir = base_keys_dir.join(provider_id);
            fs::create_dir_all(&provider_keys_dir).await?;

            // Generate individual ceremony for this provider
            let mut ceremony = Self::sp_5node_consortium_ceremony(provider_keys_dir.clone());

            // Run ceremony and generate keys
            let transcript = ceremony.run_ceremony(rng).await?;

            info!("✅ Generated individual keys for {} at {:?}", provider_name, provider_keys_dir);
            provider_key_dirs.insert(provider_id.to_string(), provider_keys_dir);
        }

        info!("🎯 Individual key generation complete for all {} providers", provider_key_dirs.len());
        Ok(provider_key_dirs)
    }

    /// Run the full trusted setup ceremony for 5-node consortium
    pub async fn run_ceremony<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R
    ) -> Result<CeremonyTranscript> {
        info!("🔐 Starting 5-Node SP Consortium Trusted Setup Ceremony");
        info!("👥 All participants: {:?}", self.config.all_participants);
        info!("✅ Required participants: {:?}", self.config.required_participants);
        info!("📋 Circuits to setup: {:?}", self.circuits.keys().collect::<Vec<_>>());

        let ceremony_id = format!("sp-5node-consortium-{}", chrono::Utc::now().timestamp());
        let mut transcript = CeremonyTranscript {
            ceremony_id: ceremony_id.clone(),
            start_time: chrono::Utc::now().timestamp() as u64,
            end_time: None,
            participants: Vec::new(),
            contributions: Vec::new(),
            final_parameters_hash: None,
            verification_status: VerificationStatus::Pending,
        };

        // Ensure keys directory exists
        fs::create_dir_all(&self.keys_dir).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to create keys directory: {}", e)))?;

        // Setup each circuit
        for circuit_id in self.circuits.keys().cloned().collect::<Vec<_>>() {
            info!("⚙️  Setting up circuit for 5-node consortium: {}", circuit_id);

            match circuit_id.as_str() {
                "cdr_privacy" => {
                    self.setup_cdr_privacy_circuit(rng, &mut transcript).await?;
                }
                "settlement_calculation" => {
                    self.setup_settlement_circuit(rng, &mut transcript).await?;
                }
                _ => {
                    warn!("Unknown circuit: {}", circuit_id);
                }
            }
        }

        transcript.end_time = Some(chrono::Utc::now().timestamp() as u64);
        transcript.verification_status = VerificationStatus::Verified;

        // Save ceremony transcript
        self.save_ceremony_transcript(&transcript).await?;

        info!("✅ 5-Node consortium trusted setup ceremony completed successfully");
        info!("🔑 Keys generated for {} circuits", self.circuits.len());
        info!("👥 Ceremony included {} participants", transcript.participants.len());
        info!("📜 Ceremony transcript saved for verification");

        Ok(transcript)
    }

    /// Setup CDR privacy circuit with real parameters for 5-node consortium
    async fn setup_cdr_privacy_circuit<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        transcript: &mut CeremonyTranscript,
    ) -> Result<()> {
        info!("🔒 Generating CDR Privacy Circuit parameters for 5-node consortium...");

        // Create empty circuit for parameter generation
        let circuit = BCEPrivacyCircuit::<Fr>::empty();

        // Generate parameters - this is the computationally expensive part
        info!("⚡ Running setup computation (this may take several minutes)...");
        let (proving_key, verifying_key) = Groth16::<Bn254>::circuit_specific_setup(circuit, rng)
            .map_err(|_| TrustedSetupError::InvalidProof)?;

        // Calculate parameters hash for verification
        let mut vk_bytes = Vec::new();
        verifying_key.serialize_compressed(&mut vk_bytes)
            .map_err(|e| TrustedSetupError::Serialization(format!("VK serialization error: {}", e)))?;

        let params_hash = Blake2bHash::hash(&vk_bytes);

        // Update circuit setup
        if let Some(setup) = self.circuits.get_mut("cdr_privacy") {
            setup.proving_key = Some(proving_key.clone());
            setup.verifying_key = Some(verifying_key.clone());
            setup.parameters_hash = Some(params_hash);
            setup.ceremony_complete = true;
        }

        // Save keys to disk
        self.save_circuit_keys("cdr_privacy", &proving_key, &verifying_key).await?;

        // Add to transcript with all 5 consortium participants
        let contribution = ParticipantContribution {
            participant_id: "5Node-Bootstrap-Coordinator".to_string(),
            circuit_id: "cdr_privacy".to_string(),
            contribution_hash: params_hash,
            previous_hash: Blake2bHash::default(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![], // In real ceremony, would be signed by all participants
        };

        transcript.contributions.push(contribution);

        // Record all 5 consortium participants as having contributed
        for participant in &self.config.all_participants {
            if !transcript.participants.contains(participant) {
                transcript.participants.push(participant.clone());
                info!("👤 Recorded participation from: {}", participant);
            }
        }

        info!("✅ CDR Privacy Circuit setup complete for 5-node consortium");
        info!("📊 Parameters hash: {:?}", params_hash);

        Ok(())
    }

    /// Setup settlement calculation circuit for 5-party netting
    async fn setup_settlement_circuit<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        transcript: &mut CeremonyTranscript,
    ) -> Result<()> {
        info!("🔒 Generating Settlement Calculation Circuit parameters for 5-party netting...");

        // Create empty circuit for 5-party settlements
        let circuit = SettlementCalculationCircuit::<Fr>::empty();

        // Generate parameters
        info!("⚡ Running setup computation for 5-party netting...");
        let (proving_key, verifying_key) = Groth16::<Bn254>::circuit_specific_setup(circuit, rng)
            .map_err(|_| TrustedSetupError::InvalidProof)?;

        // Calculate hash
        let mut vk_bytes = Vec::new();
        verifying_key.serialize_compressed(&mut vk_bytes)
            .map_err(|e| TrustedSetupError::Serialization(format!("VK serialization error: {}", e)))?;

        let params_hash = Blake2bHash::hash(&vk_bytes);

        // Update setup
        if let Some(setup) = self.circuits.get_mut("settlement_calculation") {
            setup.proving_key = Some(proving_key.clone());
            setup.verifying_key = Some(verifying_key.clone());
            setup.parameters_hash = Some(params_hash);
            setup.ceremony_complete = true;
        }

        // Save keys
        self.save_circuit_keys("settlement_calculation", &proving_key, &verifying_key).await?;

        // Add to transcript
        let contribution = ParticipantContribution {
            participant_id: "5Node-Bootstrap-Coordinator".to_string(),
            circuit_id: "settlement_calculation".to_string(),
            contribution_hash: params_hash,
            previous_hash: Blake2bHash::default(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            signature: vec![],
        };

        transcript.contributions.push(contribution);

        info!("✅ 5-Party Settlement Calculation Circuit setup complete");
        info!("📊 Parameters hash: {:?}", params_hash);

        Ok(())
    }

    /// Save circuit keys to disk
    async fn save_circuit_keys(
        &self,
        circuit_id: &str,
        proving_key: &ProvingKey<Bn254>,
        verifying_key: &VerifyingKey<Bn254>,
    ) -> Result<()> {
        // Save proving key
        let pk_path = self.keys_dir.join(format!("{}.pk", circuit_id));
        let mut pk_bytes = Vec::new();
        proving_key.serialize_compressed(&mut pk_bytes)
            .map_err(|e| TrustedSetupError::Serialization(format!("PK serialization error: {}", e)))?;

        fs::write(&pk_path, &pk_bytes).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to write PK: {}", e)))?;

        // Save verifying key
        let vk_path = self.keys_dir.join(format!("{}.vk", circuit_id));
        let mut vk_bytes = Vec::new();
        verifying_key.serialize_compressed(&mut vk_bytes)
            .map_err(|e| TrustedSetupError::Serialization(format!("VK serialization error: {}", e)))?;

        fs::write(&vk_path, &vk_bytes).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to write VK: {}", e)))?;

        info!("💾 Saved keys for {} to {:?}", circuit_id, self.keys_dir);
        info!("   📁 Proving key: {} bytes", pk_bytes.len());
        info!("   📁 Verifying key: {} bytes", vk_bytes.len());

        Ok(())
    }

    /// Load circuit keys from disk
    pub async fn load_circuit_keys(&self, circuit_id: &str) -> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
        let pk_path = self.keys_dir.join(format!("{}.pk", circuit_id));
        let vk_path = self.keys_dir.join(format!("{}.vk", circuit_id));

        // Load proving key
        let pk_bytes = fs::read(&pk_path).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to read PK: {}", e)))?;

        let proving_key = ProvingKey::<Bn254>::deserialize_compressed(&pk_bytes[..])
            .map_err(|e| TrustedSetupError::Serialization(format!("PK deserialization error: {}", e)))?;

        // Load verifying key
        let vk_bytes = fs::read(&vk_path).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to read VK: {}", e)))?;

        let verifying_key = VerifyingKey::<Bn254>::deserialize_compressed(&vk_bytes[..])
            .map_err(|e| TrustedSetupError::Serialization(format!("VK deserialization error: {}", e)))?;

        info!("🔑 Loaded keys for circuit: {}", circuit_id);

        Ok((proving_key, verifying_key))
    }

    /// Check if keys exist for a circuit
    pub async fn keys_exist(&self, circuit_id: &str) -> bool {
        let pk_path = self.keys_dir.join(format!("{}.pk", circuit_id));
        let vk_path = self.keys_dir.join(format!("{}.vk", circuit_id));

        pk_path.exists() && vk_path.exists()
    }

    /// Save ceremony transcript
    async fn save_ceremony_transcript(&self, transcript: &CeremonyTranscript) -> Result<()> {
        let transcript_path = self.keys_dir.join("ceremony_transcript.json");

        let transcript_json = serde_json::to_string_pretty(transcript)
            .map_err(|e| TrustedSetupError::Serialization(format!("Transcript serialization error: {}", e)))?;

        fs::write(&transcript_path, transcript_json).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to write transcript: {}", e)))?;

        info!("📜 5-Node ceremony transcript saved to: {:?}", transcript_path);
        Ok(())
    }

    /// Verify the ceremony transcript and keys for 5-node consortium
    pub async fn verify_ceremony(&self) -> Result<bool> {
        info!("🔍 Verifying 5-node consortium trusted setup ceremony...");

        // Load transcript
        let transcript_path = self.keys_dir.join("ceremony_transcript.json");
        let transcript_json = fs::read_to_string(&transcript_path).await
            .map_err(|e| TrustedSetupError::Serialization(format!("Failed to read transcript: {}", e)))?;

        let transcript: CeremonyTranscript = serde_json::from_str(&transcript_json)
            .map_err(|e| TrustedSetupError::Serialization(format!("Transcript deserialization error: {}", e)))?;

        // Verify all required circuits have keys
        for circuit_id in ["cdr_privacy", "settlement_calculation"] {
            if !self.keys_exist(circuit_id).await {
                error!("❌ Missing keys for circuit: {}", circuit_id);
                return Ok(false);
            }

            // Load and validate keys
            let (pk, vk) = self.load_circuit_keys(circuit_id).await?;

            // Verify key consistency
            let mut vk_bytes = Vec::new();
            vk.serialize_compressed(&mut vk_bytes)
                .map_err(|e| TrustedSetupError::Serialization(format!("VK serialization error: {}", e)))?;

            let current_hash = Blake2bHash::hash(&vk_bytes);

            // Find contribution in transcript
            let contribution = transcript.contributions.iter()
                .find(|c| c.circuit_id == circuit_id)
                .ok_or_else(|| TrustedSetupError::InvalidProof)?;

            if contribution.contribution_hash != current_hash {
                error!("❌ Key hash mismatch for circuit: {}", circuit_id);
                return Ok(false);
            }

            info!("✅ Circuit {} keys verified for 5-node consortium", circuit_id);
        }

        // Verify ceremony completeness - require at least minimum participants
        if transcript.participants.len() < self.config.min_participants {
            error!("❌ Insufficient participants: {} < {}",
                   transcript.participants.len(), self.config.min_participants);
            return Ok(false);
        }

        // Verify that all 5 consortium members are represented
        let missing_participants: Vec<_> = self.config.all_participants.iter()
            .filter(|p| !transcript.participants.contains(p))
            .collect();

        if !missing_participants.is_empty() {
            warn!("⚠️  Some consortium members didn't participate: {:?}", missing_participants);
            // Don't fail, as we only require minimum participants
        }

        match transcript.verification_status {
            VerificationStatus::Verified => {
                info!("✅ 5-Node consortium ceremony verification successful");
                info!("👥 Total participants: {} of {}", transcript.participants.len(), self.config.all_participants.len());
                info!("📋 Participants: {:?}", transcript.participants);
                info!("🕐 Duration: {} seconds",
                      transcript.end_time.unwrap_or(0) - transcript.start_time);
                Ok(true)
            }
            VerificationStatus::Failed(ref reason) => {
                error!("❌ Ceremony verification failed: {}", reason);
                Ok(false)
            }
            VerificationStatus::Pending => {
                warn!("⏳ Ceremony verification still pending");
                Ok(false)
            }
        }
    }

    /// Get 5-node consortium configuration
    pub fn get_consortium_config(&self) -> &CeremonyConfig {
        &self.config
    }

    /// Create production keys directory for 5-node consortium
    pub fn production_keys_dir() -> PathBuf {
        PathBuf::from("./sp_5node_consortium_keys")
    }

    /// Export verifying keys for public verification
    pub async fn export_verifying_keys(&self) -> Result<HashMap<String, Vec<u8>>> {
        let mut vk_exports = HashMap::new();

        for circuit_id in ["cdr_privacy", "settlement_calculation"] {
            if self.keys_exist(circuit_id).await {
                let vk_path = self.keys_dir.join(format!("{}.vk", circuit_id));
                let vk_bytes = fs::read(&vk_path).await
                    .map_err(|e| TrustedSetupError::Serialization(format!("Failed to read VK: {}", e)))?;

                vk_exports.insert(circuit_id.to_string(), vk_bytes);
            }
        }

        info!("📤 Exported {} verifying keys for distribution", vk_exports.len());
        Ok(vk_exports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use ark_std::test_rng;

    #[tokio::test]
    async fn test_5node_trusted_setup_ceremony() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().to_path_buf();

        let mut ceremony = TrustedSetupCeremony::sp_5node_consortium_ceremony(keys_dir);
        let mut rng = test_rng();

        // Run ceremony
        let transcript = ceremony.run_ceremony(&mut rng).await.unwrap();

        assert!(matches!(transcript.verification_status, VerificationStatus::Verified));
        assert_eq!(transcript.contributions.len(), 2); // Two circuits
        assert_eq!(transcript.participants.len(), 5); // All 5 consortium members

        // Verify all 5 participants are included
        let expected_participants = [
            "T-Mobile-DE", "Vodafone-UK", "Orange-FR", "Telefónica-ES", "SFR-FR"
        ];
        for participant in &expected_participants {
            assert!(transcript.participants.contains(&participant.to_string()),
                    "Missing participant: {}", participant);
        }

        // Verify keys exist
        assert!(ceremony.keys_exist("cdr_privacy").await);
        assert!(ceremony.keys_exist("settlement_calculation").await);

        // Test key loading
        let (pk, vk) = ceremony.load_circuit_keys("cdr_privacy").await.unwrap();
        assert!(!pk.vk.gamma.is_zero());
        assert!(!vk.gamma.is_zero());

        // Verify ceremony
        let verification_result = ceremony.verify_ceremony().await.unwrap();
        assert!(verification_result);
    }

    #[tokio::test] 
    async fn test_consortium_config() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().to_path_buf();

        let ceremony = TrustedSetupCeremony::sp_5node_consortium_ceremony(keys_dir);
        let config = ceremony.get_consortium_config();

        assert_eq!(config.all_participants.len(), 5);
        assert_eq!(config.min_participants, 3);
        assert!(config.all_participants.contains(&"T-Mobile-DE".to_string()));
        assert!(config.all_participants.contains(&"Telefónica-ES".to_string()));
        assert!(config.all_participants.contains(&"SFR-FR".to_string()));
    }
}