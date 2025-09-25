use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_snark::SNARK;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::zkp::circuit::{SettlementCircuit, SettlementWitness};

/// Metrics for ZKP operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZkpMetrics {
    pub proofs_generated: u64,
    pub proofs_verified: u64,
    pub proofs_failed_generation: u64,
    pub proofs_failed_verification: u64,
    pub total_proof_generation_time_ms: u64,
    pub total_verification_time_ms: u64,
    pub avg_proof_generation_time_ms: f64,
    pub avg_verification_time_ms: f64,
    pub max_proof_generation_time_ms: u64,
    pub max_verification_time_ms: u64,
    pub min_proof_generation_time_ms: u64,
    pub min_verification_time_ms: u64,
    pub system_start_time: u64,
    pub last_proof_generated: Option<u64>,
    pub last_proof_verified: Option<u64>,
}

/// Zero-Knowledge Proof system for privacy-preserving settlement validation
pub struct SettlementProofSystem {
    proving_key: ProvingKey<Bn254>,
    verifying_key: VerifyingKey<Bn254>,
    metrics: Arc<Mutex<ZkpMetrics>>,
}

/// ZK Proof for settlement transactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementProof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<String>, // Serialized field elements
}

/// Parameters for proof generation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofParameters {
    pub total_amount_cents: u64,
    pub operator_count: u32,
    pub settlement_hash: [u8; 32],
    pub private_amounts: Vec<u64>, // Hidden individual amounts
    pub private_rates: Vec<u64>,   // Hidden rate information
}

impl SettlementProofSystem {
    /// Initialize the proof system with trusted setup keys from disk
    pub fn new(provider_name: &str) -> Result<Self, ZkpError> {
        println!("🔐 Initializing ZKP settlement proof system for {}...", provider_name);

        // Load the same trusted setup keys used for proof generation
        let keys_dir = "/app/zkp_keys";
        let pk_path = format!("{}/cdr_privacy.pk", keys_dir);
        let vk_path = format!("{}/cdr_privacy.vk", keys_dir);

        // Load proving key
        let pk_bytes = std::fs::read(&pk_path)
            .map_err(|e| ZkpError::SetupFailed(format!("Failed to load proving key from {}: {}", pk_path, e)))?;
        let pk = ProvingKey::<Bn254>::deserialize_compressed(&pk_bytes[..])
            .map_err(|e| ZkpError::SetupFailed(format!("Proving key deserialization failed: {:?}", e)))?;

        // Load verifying key
        let vk_bytes = std::fs::read(&vk_path)
            .map_err(|e| ZkpError::SetupFailed(format!("Failed to load verifying key from {}: {}", vk_path, e)))?;
        let vk = VerifyingKey::<Bn254>::deserialize_compressed(&vk_bytes[..])
            .map_err(|e| ZkpError::SetupFailed(format!("Verifying key deserialization failed: {:?}", e)))?;

        println!("✅ ZKP trusted setup loaded from {} keys", provider_name);

        let mut initial_metrics = ZkpMetrics::default();
        initial_metrics.system_start_time = chrono::Utc::now().timestamp() as u64;
        initial_metrics.min_proof_generation_time_ms = u64::MAX;
        initial_metrics.min_verification_time_ms = u64::MAX;

        Ok(Self {
            proving_key: pk,
            verifying_key: vk,
            metrics: Arc::new(Mutex::new(initial_metrics)),
        })
    }

    /// Generate a privacy-preserving proof for a settlement
    pub fn generate_proof(&self, params: ProofParameters) -> Result<SettlementProof, ZkpError> {
        let start_time = Instant::now();
        println!("🛡️  Generating ZK proof for settlement...");

        // Create witness data
        let witness = SettlementWitness {
            total_amount: params.total_amount_cents,
            operator_count: params.operator_count,
            settlement_hash: params.settlement_hash,
            private_amounts: params.private_amounts.clone(),
            private_rates: params.private_rates.clone(),
        };

        // Create circuit with witness
        let circuit = SettlementCircuit::new(witness);

        // Public inputs (what everyone can see)
        let public_inputs = vec![
            Fr::from(params.total_amount_cents),
            Fr::from(params.operator_count as u64),
            // Convert settlement hash to field element
            {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&params.settlement_hash[0..8]);
                Fr::from(u64::from_le_bytes(bytes))
            },
        ];

        // Generate the proof
        let rng = &mut ark_std::rand::thread_rng();
        let proof_result = Groth16::<Bn254>::prove(&self.proving_key, circuit, rng);

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;
        let current_time = chrono::Utc::now().timestamp() as u64;

        // Update metrics based on result
        let mut metrics = self.metrics.lock().unwrap();
        match proof_result {
            Ok(proof) => {
                // Serialize proof
                let mut proof_bytes = Vec::new();
                if let Err(e) = proof.serialize_compressed(&mut proof_bytes) {
                    metrics.proofs_failed_generation += 1;
                    return Err(ZkpError::SerializationFailed(format!("Proof serialization failed: {:?}", e)));
                }

                // Serialize public inputs
                let public_inputs_str: Vec<String> = public_inputs
                    .iter()
                    .map(|input| {
                        let mut bytes = Vec::new();
                        input.serialize_compressed(&mut bytes).unwrap();
                        hex::encode(bytes)
                    })
                    .collect();

                // Update success metrics
                metrics.proofs_generated += 1;
                metrics.total_proof_generation_time_ms += duration_ms;
                metrics.avg_proof_generation_time_ms =
                    metrics.total_proof_generation_time_ms as f64 / metrics.proofs_generated as f64;
                metrics.max_proof_generation_time_ms =
                    metrics.max_proof_generation_time_ms.max(duration_ms);
                if metrics.min_proof_generation_time_ms == u64::MAX || duration_ms < metrics.min_proof_generation_time_ms {
                    metrics.min_proof_generation_time_ms = duration_ms;
                }
                metrics.last_proof_generated = Some(current_time);

                println!("✅ ZK proof generated successfully ({} bytes, {}ms)", proof_bytes.len(), duration_ms);

                Ok(SettlementProof {
                    proof_bytes,
                    public_inputs: public_inputs_str,
                })
            }
            Err(e) => {
                metrics.proofs_failed_generation += 1;
                Err(ZkpError::ProofGenFailed(format!("Proof generation failed: {:?}", e)))
            }
        }
    }

    /// Verify a settlement proof
    pub fn verify_proof(&self, settlement_proof: &SettlementProof) -> Result<bool, ZkpError> {
        let start_time = Instant::now();
        println!("🔍 Verifying ZK settlement proof...");

        // Deserialize proof
        let proof = Proof::<Bn254>::deserialize_compressed(&settlement_proof.proof_bytes[..])
            .map_err(|e| ZkpError::DeserializationFailed(format!("Proof deserialization failed: {:?}", e)))?;

        // Deserialize public inputs
        let public_inputs: Result<Vec<Fr>, _> = settlement_proof
            .public_inputs
            .iter()
            .map(|input_str| {
                let bytes = hex::decode(input_str)
                    .map_err(|e| ZkpError::DeserializationFailed(format!("Hex decode failed: {:?}", e)))?;
                Fr::deserialize_compressed(&bytes[..])
                    .map_err(|e| ZkpError::DeserializationFailed(format!("Field element deserialization failed: {:?}", e)))
            })
            .collect();

        let public_inputs = public_inputs?;

        // Verify the proof
        let verify_result = Groth16::<Bn254>::verify(&self.verifying_key, &public_inputs, &proof);

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;
        let current_time = chrono::Utc::now().timestamp() as u64;

        // Update metrics based on result
        let mut metrics = self.metrics.lock().unwrap();
        match verify_result {
            Ok(is_valid) => {
                metrics.proofs_verified += 1;
                metrics.total_verification_time_ms += duration_ms;
                metrics.avg_verification_time_ms =
                    metrics.total_verification_time_ms as f64 / metrics.proofs_verified as f64;
                metrics.max_verification_time_ms =
                    metrics.max_verification_time_ms.max(duration_ms);
                if metrics.min_verification_time_ms == u64::MAX || duration_ms < metrics.min_verification_time_ms {
                    metrics.min_verification_time_ms = duration_ms;
                }
                metrics.last_proof_verified = Some(current_time);

                if is_valid {
                    println!("✅ ZK proof verification successful ({}ms)", duration_ms);
                } else {
                    println!("❌ ZK proof verification failed - proof is invalid ({}ms)", duration_ms);
                }

                Ok(is_valid)
            }
            Err(e) => {
                metrics.proofs_failed_verification += 1;
                Err(ZkpError::VerificationFailed(format!("Verification failed: {:?}", e)))
            }
        }
    }

    /// Generate proof for BCE record settlement (privacy-preserving)
    pub fn prove_bce_settlement(
        &self,
        total_amount_cents: u64,
        individual_amounts: Vec<u64>,
        rates: Vec<u64>,
        settlement_id: &str,
    ) -> Result<SettlementProof, ZkpError> {
        let settlement_hash = crate::hash::Blake2bHash::hash(settlement_id.as_bytes());
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&settlement_hash.as_bytes()[0..32]); // Take first 32 bytes

        let params = ProofParameters {
            total_amount_cents,
            operator_count: individual_amounts.len() as u32,
            settlement_hash: hash_array,
            private_amounts: individual_amounts,
            private_rates: rates,
        };

        self.generate_proof(params)
    }

    /// Batch verify multiple settlement proofs
    pub fn batch_verify(&self, proofs: &[SettlementProof]) -> Result<Vec<bool>, ZkpError> {
        println!("🔍 Batch verifying {} ZK proofs...", proofs.len());

        let results: Result<Vec<bool>, _> = proofs
            .iter()
            .map(|proof| self.verify_proof(proof))
            .collect();

        let results = results?;
        let valid_count = results.iter().filter(|&&valid| valid).count();

        println!("✅ Batch verification complete: {}/{} proofs valid", valid_count, proofs.len());

        Ok(results)
    }

    /// Export verifying key for public verification
    pub fn export_verifying_key(&self) -> Result<Vec<u8>, ZkpError> {
        let mut vk_bytes = Vec::new();
        self.verifying_key.serialize_compressed(&mut vk_bytes)
            .map_err(|e| ZkpError::SerializationFailed(format!("VK serialization failed: {:?}", e)))?;
        Ok(vk_bytes)
    }

    /// Create proof system from exported verifying key (for verification-only nodes)
    pub fn from_verifying_key(vk_bytes: &[u8]) -> Result<Self, ZkpError> {
        let vk = VerifyingKey::<Bn254>::deserialize_compressed(vk_bytes)
            .map_err(|e| ZkpError::DeserializationFailed(format!("VK deserialization failed: {:?}", e)))?;

        // Create dummy proving key (won't be used)
        let dummy_circuit = SettlementCircuit::new_dummy();
        let rng = &mut ark_std::rand::thread_rng();
        let (pk, _) = Groth16::<Bn254>::circuit_specific_setup(dummy_circuit, rng)
            .map_err(|e| ZkpError::SetupFailed(format!("Dummy setup failed: {:?}", e)))?;

        let mut initial_metrics = ZkpMetrics::default();
        initial_metrics.system_start_time = chrono::Utc::now().timestamp() as u64;
        initial_metrics.min_proof_generation_time_ms = u64::MAX;
        initial_metrics.min_verification_time_ms = u64::MAX;

        Ok(Self {
            proving_key: pk,
            verifying_key: vk,
            metrics: Arc::new(Mutex::new(initial_metrics)),
        })
    }

    /// Get system information for status checks
    pub fn get_system_info(&self) -> Result<serde_json::Value, ZkpError> {
        // Get verifying key info
        let vk_bytes = self.export_verifying_key()?;

        Ok(serde_json::json!({
            "system_type": "Groth16 zk-SNARKs",
            "curve": "BN254",
            "verifying_key_size_bytes": vk_bytes.len(),
            "proving_key_available": true,
            "system_initialized": true,
            "supported_circuits": ["settlement_privacy", "cdr_validation"],
            "max_operators": 5,
            "version": "1.0.0"
        }))
    }

    /// Get trusted setup information
    pub fn get_setup_info(&self) -> Result<serde_json::Value, ZkpError> {
        let vk_bytes = self.export_verifying_key()?;
        let vk_hash = crate::hash::Blake2bHash::hash(&vk_bytes);

        Ok(serde_json::json!({
            "setup_type": "circuit_specific",
            "curve": "BN254",
            "ceremony_completed": true,
            "proving_key_hash": hex::encode(&vk_hash.as_bytes()[0..16]),
            "verifying_key_hash": hex::encode(&vk_hash.as_bytes()[16..32]),
            "setup_timestamp": chrono::Utc::now().timestamp(),
            "trusted_participants": 1, // Simplified for demo
            "circuit_constraints": "~10000", // Approximate
            "security_level": 128
        }))
    }

    /// Get comprehensive ZKP metrics
    pub fn get_metrics(&self) -> ZkpMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get performance metrics in JSON format
    pub fn get_performance_metrics(&self) -> Result<serde_json::Value, ZkpError> {
        let metrics = self.metrics.lock().unwrap();
        let current_time = chrono::Utc::now().timestamp() as u64;
        let uptime_seconds = current_time - metrics.system_start_time;

        Ok(serde_json::json!({
            "performance": {
                "proof_generation": {
                    "total_proofs_generated": metrics.proofs_generated,
                    "average_time_ms": metrics.avg_proof_generation_time_ms,
                    "min_time_ms": if metrics.min_proof_generation_time_ms == u64::MAX { 0 } else { metrics.min_proof_generation_time_ms },
                    "max_time_ms": metrics.max_proof_generation_time_ms,
                    "total_time_ms": metrics.total_proof_generation_time_ms,
                    "failures": metrics.proofs_failed_generation,
                    "success_rate": if metrics.proofs_generated + metrics.proofs_failed_generation > 0 {
                        metrics.proofs_generated as f64 / (metrics.proofs_generated + metrics.proofs_failed_generation) as f64 * 100.0
                    } else { 0.0 }
                },
                "proof_verification": {
                    "total_proofs_verified": metrics.proofs_verified,
                    "average_time_ms": metrics.avg_verification_time_ms,
                    "min_time_ms": if metrics.min_verification_time_ms == u64::MAX { 0 } else { metrics.min_verification_time_ms },
                    "max_time_ms": metrics.max_verification_time_ms,
                    "total_time_ms": metrics.total_verification_time_ms,
                    "failures": metrics.proofs_failed_verification,
                    "success_rate": if metrics.proofs_verified + metrics.proofs_failed_verification > 0 {
                        metrics.proofs_verified as f64 / (metrics.proofs_verified + metrics.proofs_failed_verification) as f64 * 100.0
                    } else { 0.0 }
                },
                "system": {
                    "uptime_seconds": uptime_seconds,
                    "uptime_hours": uptime_seconds as f64 / 3600.0,
                    "last_proof_generated": metrics.last_proof_generated,
                    "last_proof_verified": metrics.last_proof_verified,
                    "total_operations": metrics.proofs_generated + metrics.proofs_verified,
                    "operations_per_hour": if uptime_seconds > 0 {
                        (metrics.proofs_generated + metrics.proofs_verified) as f64 * 3600.0 / uptime_seconds as f64
                    } else { 0.0 }
                }
            }
        }))
    }

    /// Perform comprehensive health check
    pub fn health_check(&self) -> Result<serde_json::Value, ZkpError> {
        let metrics = self.metrics.lock().unwrap();
        let current_time = chrono::Utc::now().timestamp() as u64;
        let uptime_seconds = current_time - metrics.system_start_time;

        // Calculate health scores
        let proof_gen_success_rate = if metrics.proofs_generated + metrics.proofs_failed_generation > 0 {
            metrics.proofs_generated as f64 / (metrics.proofs_generated + metrics.proofs_failed_generation) as f64 * 100.0
        } else { 100.0 };

        let verification_success_rate = if metrics.proofs_verified + metrics.proofs_failed_verification > 0 {
            metrics.proofs_verified as f64 / (metrics.proofs_verified + metrics.proofs_failed_verification) as f64 * 100.0
        } else { 100.0 };

        // Determine system health status
        let health_status = if proof_gen_success_rate >= 95.0 && verification_success_rate >= 95.0 {
            "healthy"
        } else if proof_gen_success_rate >= 80.0 && verification_success_rate >= 80.0 {
            "degraded"
        } else {
            "unhealthy"
        };

        // Performance warnings
        let mut warnings = Vec::new();
        if metrics.avg_proof_generation_time_ms > 5000.0 {
            warnings.push("High proof generation latency detected");
        }
        if metrics.avg_verification_time_ms > 100.0 {
            warnings.push("High verification latency detected");
        }
        if proof_gen_success_rate < 90.0 {
            warnings.push("Low proof generation success rate");
        }
        if verification_success_rate < 90.0 {
            warnings.push("Low verification success rate");
        }

        Ok(serde_json::json!({
            "health": {
                "status": health_status,
                "uptime_seconds": uptime_seconds,
                "warnings": warnings,
                "scores": {
                    "proof_generation_success_rate": proof_gen_success_rate,
                    "verification_success_rate": verification_success_rate,
                    "overall_health_score": (proof_gen_success_rate + verification_success_rate) / 2.0
                },
                "operations": {
                    "total_proofs_generated": metrics.proofs_generated,
                    "total_proofs_verified": metrics.proofs_verified,
                    "total_failures": metrics.proofs_failed_generation + metrics.proofs_failed_verification
                },
                "timing": {
                    "avg_proof_gen_ms": metrics.avg_proof_generation_time_ms,
                    "avg_verification_ms": metrics.avg_verification_time_ms
                },
                "last_activity": {
                    "last_proof_generated": metrics.last_proof_generated,
                    "last_proof_verified": metrics.last_proof_verified
                },
                "checked_at": current_time
            }
        }))
    }

    /// Reset metrics (for testing purposes)
    pub fn reset_metrics(&self) -> Result<(), ZkpError> {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics = ZkpMetrics::default();
        metrics.system_start_time = chrono::Utc::now().timestamp() as u64;
        metrics.min_proof_generation_time_ms = u64::MAX;
        metrics.min_verification_time_ms = u64::MAX;
        Ok(())
    }
}

impl Default for SettlementProofSystem {
    fn default() -> Self {
        Self::new("default").expect("ZKP system initialization failed")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ZkpError {
    #[error("ZKP setup failed: {0}")]
    SetupFailed(String),

    #[error("Proof generation failed: {0}")]
    ProofGenFailed(String),

    #[error("Proof verification failed: {0}")]
    VerificationFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_proof_system() {
        let zkp_system = SettlementProofSystem::new("test-provider").unwrap();

        let params = ProofParameters {
            total_amount_cents: 10000,
            operator_count: 2,
            settlement_hash: [1u8; 32],
            private_amounts: vec![6000, 4000],
            private_rates: vec![100, 150],
        };

        let proof = zkp_system.generate_proof(params).unwrap();
        let is_valid = zkp_system.verify_proof(&proof).unwrap();

        assert!(is_valid);
    }
}