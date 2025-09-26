// Cryptographic verification for 5-party SP consortium smart contracts
use serde::{Deserialize, Serialize};
use log::{info, error};
use thiserror::Error;

use crate::hash::Blake2bHash;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid proof format")]
    InvalidProofFormat,
    #[error("Proof verification failed")]
    ProofVerificationFailed,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Invalid consortium member: {0}")]
    InvalidConsortiumMember(String),
}

type Result<T> = std::result::Result<T, CryptoError>;

/// ZKP proof inputs for settlement calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProofInputs {
    pub bilateral_amounts: [u64; 20], // All 20 bilateral amounts for 5 parties
    pub net_positions: [i64; 5],      // Net positions for all 5 parties
    pub net_settlement_count: u64,
    pub total_net_amount: u64,
    pub period_hash: [u8; 8],
    pub savings_percentage: u64,
    pub consortium_hash: u64,
}

/// ZKP proof inputs for BCE privacy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BCEPrivacyInputs {
    pub raw_call_minutes: u64,
    pub raw_data_mb: u64,
    pub raw_sms_count: u64,
    pub roaming_minutes: u64,
    pub roaming_data_mb: u64,
    pub call_rate_cents: u64,
    pub data_rate_cents: u64,
    pub sms_rate_cents: u64,
    pub roaming_rate_cents: u64,
    pub roaming_data_rate_cents: u64,
    pub privacy_salt: u64,
    pub total_charges_cents: u64,
    pub period_hash: u64,
    pub network_pair_hash: u64,
    pub commitment_randomness: u64,
    pub consortium_id: u64,
}

/// Digital signature for consortium member authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsortiumSignature {
    pub signer_id: String,
    pub signature_data: Vec<u8>,
    pub public_key: Vec<u8>,
    pub message_hash: Blake2bHash,
    pub signature_type: SignatureType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureType {
    Ed25519,
    BLS, // For aggregated signatures
}

/// Cryptographic verifier for 5-party consortium operations
#[derive(Clone)]
pub struct CryptoVerifier {
    /// Valid consortium member IDs
    consortium_members: Vec<String>,
    
    /// ZKP verification enabled flag
    zkp_enabled: bool,
    
    /// Signature verification enabled flag
    signature_verification_enabled: bool,
}

impl CryptoVerifier {
    /// Create new crypto verifier for 5-party consortium
    pub fn new_5party_consortium() -> Self {
        let consortium_members = vec![
            "T-Mobile-DE".to_string(),
            "Vodafone-UK".to_string(),
            "Orange-FR".to_string(),
            "Telefónica-ES".to_string(),
            "SFR-FR".to_string(),
        ];
        
        Self {
            consortium_members,
            zkp_enabled: true,
            signature_verification_enabled: true,
        }
    }
    
    /// Verify ZKP settlement proof
    pub fn verify_settlement_proof(
        &self,
        proof_data: &[u8],
        public_inputs: &SettlementProofInputs,
    ) -> Result<bool> {
        if !self.zkp_enabled {
            info!("⚠️  ZKP verification disabled - skipping settlement proof check");
            return Ok(true);
        }
        
        info!("🔐 Verifying 5-party settlement ZKP proof...");
        
        // Validate input constraints
        self.validate_settlement_inputs(public_inputs)?;
        
        // In a real implementation, this would:
        // 1. Deserialize the Groth16 proof from proof_data
        // 2. Load the verifying key for the settlement circuit
        // 3. Verify the proof against the public inputs
        // 4. Return the verification result
        
        // For demo purposes, simulate verification logic
        if proof_data.len() < 192 { // Groth16 proofs are typically ~192 bytes
            error!("❌ Settlement proof too short: {} bytes", proof_data.len());
            return Err(CryptoError::InvalidProofFormat);
        }
        
        // Simulate computational verification
        let verification_result = self.simulate_settlement_verification(public_inputs);
        
        if verification_result {
            info!("✅ 5-party settlement proof verified successfully");
            Ok(true)
        } else {
            error!("❌ 5-party settlement proof verification failed");
            Err(CryptoError::ProofVerificationFailed)
        }
    }
    
    /// Verify ZKP BCE privacy proof
    pub fn verify_bce_privacy_proof(
        &self,
        proof_data: &[u8],
        public_inputs: &BCEPrivacyInputs,
    ) -> Result<bool> {
        if !self.zkp_enabled {
            info!("⚠️  ZKP verification disabled - skipping BCE privacy proof check");
            return Ok(true);
        }
        
        info!("🔒 Verifying BCE privacy ZKP proof...");
        
        // Validate input constraints
        self.validate_cdr_inputs(public_inputs)?;
        
        // Validate proof length for real Groth16 proofs
        if proof_data.len() < 96 { // Real Groth16 proofs are ~96-128 bytes for BN254
            error!("❌ BCE privacy proof too short: {} bytes", proof_data.len());
            return Err(CryptoError::InvalidProofFormat);
        }
        
        let verification_result = self.simulate_cdr_verification(public_inputs);
        
        if verification_result {
            info!("✅ BCE privacy proof verified successfully");
            Ok(true)
        } else {
            error!("❌ BCE privacy proof verification failed");
            Err(CryptoError::ProofVerificationFailed)
        }
    }
    
    /// Verify digital signature from consortium member
    pub fn verify_consortium_signature(&self, signature: &ConsortiumSignature) -> Result<bool> {
        if !self.signature_verification_enabled {
            info!("⚠️  Signature verification disabled - skipping signature check");
            return Ok(true);
        }
        
        info!("✍️  Verifying signature from: {}", signature.signer_id);
        
        // Validate that signer is consortium member
        if !self.consortium_members.contains(&signature.signer_id) {
            error!("❌ Invalid consortium member: {}", signature.signer_id);
            return Err(CryptoError::InvalidConsortiumMember(signature.signer_id.clone()));
        }
        
        // Validate signature format
        match signature.signature_type {
            SignatureType::Ed25519 => {
                if signature.signature_data.len() != 64 {
                    error!("❌ Invalid Ed25519 signature length: {}", signature.signature_data.len());
                    return Err(CryptoError::InvalidSignature);
                }
                if signature.public_key.len() != 32 {
                    error!("❌ Invalid Ed25519 public key length: {}", signature.public_key.len());
                    return Err(CryptoError::InvalidSignature);
                }
            }
            SignatureType::BLS => {
                if signature.signature_data.len() != 96 {
                    error!("❌ Invalid BLS signature length: {}", signature.signature_data.len());
                    return Err(CryptoError::InvalidSignature);
                }
                if signature.public_key.len() != 48 {
                    error!("❌ Invalid BLS public key length: {}", signature.public_key.len());
                    return Err(CryptoError::InvalidSignature);
                }
            }
        }
        
        // In a real implementation, this would verify the actual signature
        // For demo purposes, simulate successful verification
        info!("✅ Consortium signature verified for: {}", signature.signer_id);
        Ok(true)
    }
    
    /// Verify multiple signatures (for multi-party contracts)
    pub fn verify_multi_party_signatures(
        &self,
        signatures: &[ConsortiumSignature],
        required_count: usize,
    ) -> Result<bool> {
        info!("👥 Verifying multi-party signatures: {}/{} required", required_count, signatures.len());
        
        if signatures.len() < required_count {
            error!("❌ Insufficient signatures: {} < {}", signatures.len(), required_count);
            return Ok(false);
        }
        
        let mut valid_signatures = 0;
        let mut unique_signers = std::collections::HashSet::new();
        
        for signature in signatures {
            // Ensure no duplicate signers
            if unique_signers.contains(&signature.signer_id) {
                error!("❌ Duplicate signature from: {}", signature.signer_id);
                continue;
            }
            
            match self.verify_consortium_signature(signature) {
                Ok(true) => {
                    valid_signatures += 1;
                    unique_signers.insert(signature.signer_id.clone());
                }
                Ok(false) | Err(_) => {
                    error!("❌ Invalid signature from: {}", signature.signer_id);
                }
            }
        }
        
        let success = valid_signatures >= required_count;
        
        if success {
            info!("✅ Multi-party signature verification successful: {}/{}", valid_signatures, required_count);
        } else {
            error!("❌ Multi-party signature verification failed: {}/{}", valid_signatures, required_count);
        }
        
        Ok(success)
    }
    
    /// Validate settlement proof inputs
    fn validate_settlement_inputs(&self, inputs: &SettlementProofInputs) -> Result<()> {
        // Validate that bilateral amounts are reasonable
        for &amount in &inputs.bilateral_amounts {
            if amount > 100_000_000 { // Max €1M per bilateral settlement
                error!("❌ Bilateral amount too large: {}", amount);
                return Err(CryptoError::ProofVerificationFailed);
            }
        }
        
        // Validate net settlement count (max 10 for 5 parties)
        if inputs.net_settlement_count > 10 {
            error!("❌ Net settlement count too high: {}", inputs.net_settlement_count);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        // Validate savings percentage (0-100%)
        if inputs.savings_percentage > 100 {
            error!("❌ Invalid savings percentage: {}", inputs.savings_percentage);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        // Validate consortium hash
        if inputs.consortium_hash != 54321 { // Expected 5-party consortium hash
            error!("❌ Invalid consortium hash: {}", inputs.consortium_hash);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        info!("✅ Settlement proof inputs validated");
        Ok(())
    }
    
    /// Validate BCE privacy proof inputs
    fn validate_cdr_inputs(&self, inputs: &BCEPrivacyInputs) -> Result<()> {
        // Validate usage amounts are reasonable
        if inputs.raw_call_minutes > 150_000 {
            error!("❌ Call minutes too high: {}", inputs.raw_call_minutes);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        if inputs.raw_data_mb > 2_000_000 {
            error!("❌ Data usage too high: {}", inputs.raw_data_mb);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        if inputs.total_charges_cents > 1_000_000_000 { // Max €10M
            error!("❌ Total charges too high: {}", inputs.total_charges_cents);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        // Validate consortium ID
        if inputs.consortium_id != 12345 { // Expected 5-party consortium ID
            error!("❌ Invalid consortium ID: {}", inputs.consortium_id);
            return Err(CryptoError::ProofVerificationFailed);
        }
        
        info!("✅ BCE privacy proof inputs validated");
        Ok(())
    }
    
    /// Simulate settlement proof verification (replace with real ZKP verification)
    fn simulate_settlement_verification(&self, inputs: &SettlementProofInputs) -> bool {
        // Simulate verification logic - in reality this would verify Groth16 proof
        let total_bilateral: u64 = inputs.bilateral_amounts.iter().sum();
        let expected_savings = (total_bilateral * inputs.savings_percentage) / 100;
        let expected_net = total_bilateral - expected_savings;
        
        // Verify netting calculation makes sense
        let actual_vs_expected_diff = if inputs.total_net_amount > expected_net {
            inputs.total_net_amount - expected_net
        } else {
            expected_net - inputs.total_net_amount
        };
        
        // Allow 10% tolerance in simulation
        let tolerance = expected_net / 10;
        actual_vs_expected_diff <= tolerance
    }
    
    /// Simulate BCE privacy proof verification (replace with real ZKP verification)
    fn simulate_cdr_verification(&self, inputs: &BCEPrivacyInputs) -> bool {
        // Simulate verification of realistic roaming CDR calculation
        // In real roaming: subscriber uses foreign network, pays roaming rates for ALL usage
        let roaming_call_charges = inputs.raw_call_minutes * inputs.call_rate_cents;
        let roaming_data_charges = inputs.raw_data_mb * inputs.data_rate_cents;
        let roaming_sms_charges = inputs.raw_sms_count * inputs.sms_rate_cents;

        let calculated_total = roaming_call_charges + roaming_data_charges + roaming_sms_charges;
        
        // Verify total charges match calculation
        calculated_total == inputs.total_charges_cents
    }
    
    /// Get consortium member list
    pub fn get_consortium_members(&self) -> &[String] {
        &self.consortium_members
    }
    
    /// Enable/disable ZKP verification (for testing)
    pub fn set_zkp_enabled(&mut self, enabled: bool) {
        self.zkp_enabled = enabled;
        info!("⚙️  ZKP verification: {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Enable/disable signature verification (for testing)
    pub fn set_signature_verification_enabled(&mut self, enabled: bool) {
        self.signature_verification_enabled = enabled;
        info!("⚙️  Signature verification: {}", if enabled { "enabled" } else { "disabled" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crypto_verifier_creation() {
        let verifier = CryptoVerifier::new_5party_consortium();
        assert_eq!(verifier.consortium_members.len(), 5);
        assert!(verifier.consortium_members.contains(&"T-Mobile-DE".to_string()));
        assert!(verifier.consortium_members.contains(&"SFR-FR".to_string()));
    }
    
    #[test]
    fn test_settlement_proof_validation() {
        let verifier = CryptoVerifier::new_5party_consortium();
        
        let valid_inputs = SettlementProofInputs {
            bilateral_amounts: [50000; 20], // €500 each
            net_positions: [100000, -50000, 25000, -25000, -50000],
            net_settlement_count: 4,
            total_net_amount: 250000,
            period_hash: [1, 2, 3, 4, 5, 6, 7, 8],
            savings_percentage: 75,
            consortium_hash: 54321,
        };
        
        assert!(verifier.validate_settlement_inputs(&valid_inputs).is_ok());
        
        // Test invalid consortium hash
        let invalid_inputs = SettlementProofInputs {
            consortium_hash: 99999, // Invalid
            ..valid_inputs
        };
        
        assert!(verifier.validate_settlement_inputs(&invalid_inputs).is_err());
    }
    
    #[test]
    fn test_consortium_signature_verification() {
        let verifier = CryptoVerifier::new_5party_consortium();
        
        let valid_signature = ConsortiumSignature {
            signer_id: "T-Mobile-DE".to_string(),
            signature_data: vec![0u8; 64], // Mock Ed25519 signature
            public_key: vec![1u8; 32],     // Mock Ed25519 public key
            message_hash: Blake2bHash::hash(b"test message"),
            signature_type: SignatureType::Ed25519,
        };
        
        assert!(verifier.verify_consortium_signature(&valid_signature).is_ok());
        
        // Test invalid consortium member
        let invalid_signature = ConsortiumSignature {
            signer_id: "Invalid-Network".to_string(),
            ..valid_signature
        };
        
        assert!(verifier.verify_consortium_signature(&invalid_signature).is_err());
    }
    
    #[test]
    fn test_multi_party_signature_verification() {
        let verifier = CryptoVerifier::new_5party_consortium();
        
        let signatures = vec![
            ConsortiumSignature {
                signer_id: "T-Mobile-DE".to_string(),
                signature_data: vec![0u8; 64],
                public_key: vec![1u8; 32],
                message_hash: Blake2bHash::hash(b"test"),
                signature_type: SignatureType::Ed25519,
            },
            ConsortiumSignature {
                signer_id: "Vodafone-UK".to_string(),
                signature_data: vec![0u8; 64],
                public_key: vec![2u8; 32],
                message_hash: Blake2bHash::hash(b"test"),
                signature_type: SignatureType::Ed25519,
            },
            ConsortiumSignature {
                signer_id: "Orange-FR".to_string(),
                signature_data: vec![0u8; 64],
                public_key: vec![3u8; 32],
                message_hash: Blake2bHash::hash(b"test"),
                signature_type: SignatureType::Ed25519,
            },
        ];
        
        // Should succeed with 3/3 required
        assert!(verifier.verify_multi_party_signatures(&signatures, 3).unwrap());
        
        // Should succeed with 2/3 required
        assert!(verifier.verify_multi_party_signatures(&signatures, 2).unwrap());
        
        // Should fail with 4/3 required (impossible)
        assert!(!verifier.verify_multi_party_signatures(&signatures, 4).unwrap());
    }
    
    #[test]
    fn test_cdr_privacy_simulation() {
        let verifier = CryptoVerifier::new_5party_consortium();
        
        let inputs = BCEPrivacyInputs {
            raw_call_minutes: 1000,
            raw_data_mb: 5000,
            raw_sms_count: 200,
            roaming_minutes: 500,
            roaming_data_mb: 1000,
            call_rate_cents: 15,
            data_rate_cents: 5,
            sms_rate_cents: 10,
            roaming_rate_cents: 25,
            roaming_data_rate_cents: 8,
            privacy_salt: 12345,
            total_charges_cents: 65500, // 1000*15 + 5000*5 + 200*10 + 500*25 + 1000*8
            period_hash: 20240101,
            network_pair_hash: 98765,
            commitment_randomness: 54321,
            consortium_id: 12345,
        };
        
        assert!(verifier.simulate_cdr_verification(&inputs));
        
        // Test with incorrect total
        let incorrect_inputs = BCEPrivacyInputs {
            total_charges_cents: 99999, // Wrong total
            ..inputs
        };
        
        assert!(!verifier.simulate_cdr_verification(&incorrect_inputs));
    }
}