// SP Authentication system for 5-party consortium
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use log::{info, warn, error};

#[derive(Error, Debug)]
pub enum AuthenticationError {
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Expired timestamp")]
    ExpiredTimestamp,
    #[error("Unknown SP provider: {0}")]
    UnknownProvider(String),
    #[error("Authorization denied for SP: {0}")]
    AuthorizationDenied(String),
    #[error("Missing authentication header")]
    MissingAuthHeader,
    #[error("Malformed authentication data")]
    MalformedAuth,
}

/// Represents an authenticated SP provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedSp {
    pub provider_id: String,
    pub provider_name: String,
    pub api_key: String,
    pub public_key_bytes: [u8; 32],
    pub permissions: Vec<SpPermission>,
}

impl AuthenticatedSp {
    pub fn get_public_key(&self) -> Result<VerifyingKey, ed25519_dalek::SignatureError> {
        VerifyingKey::from_bytes(&self.public_key_bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpPermission {
    SubmitBceRecords,
    ViewAllRecords,
    ExecuteSettlements,
    ManageContracts,
    ViewStats,
}

/// SP Authentication system for the consortium
pub struct SpAuthentication {
    /// Known SP providers with their credentials
    providers: HashMap<String, SpProvider>,

    /// API keys mapping to providers
    api_keys: HashMap<String, String>, // api_key -> provider_id

    /// Request signature validation window (seconds)
    signature_window: u64,
}

#[derive(Debug, Clone)]
struct SpProvider {
    provider_id: String,
    provider_name: String,
    api_key: String,
    public_key: VerifyingKey,
    permissions: Vec<SpPermission>,
    active: bool,
}

impl SpAuthentication {
    /// Initialize authentication for 5-party consortium
    pub fn new_consortium() -> Self {
        let mut auth = Self {
            providers: HashMap::new(),
            api_keys: HashMap::new(),
            signature_window: 300, // 5 minutes
        };

        // Initialize consortium providers
        auth.initialize_consortium_providers();
        auth
    }

    /// Initialize the 5 known consortium providers
    fn initialize_consortium_providers(&mut self) {
        let consortium_providers = vec![
            ("tmobile-de", "T-Mobile-DE", "tmobile_api_key_2024_secure"),
            ("vodafone-uk", "Vodafone-UK", "vodafone_api_key_2024_secure"),
            ("orange-fr", "Orange-FR", "orange_api_key_2024_secure"),
            ("telenor-no", "Telenor-NO", "telenor_api_key_2024_secure"),
            ("sfr-fr", "SFR-FR", "sfr_api_key_2024_secure"),
        ];

        for (provider_id, provider_name, api_key) in consortium_providers {
            // Generate demo public key (in production, these would be loaded from secure storage)
            let demo_public_key = self.generate_demo_public_key(provider_id);

            let provider = SpProvider {
                provider_id: provider_id.to_string(),
                provider_name: provider_name.to_string(),
                api_key: api_key.to_string(),
                public_key: demo_public_key,
                permissions: vec![
                    SpPermission::SubmitBceRecords,
                    SpPermission::ViewStats,
                    SpPermission::ExecuteSettlements,
                ],
                active: true,
            };

            self.providers.insert(provider_id.to_string(), provider.clone());
            self.api_keys.insert(api_key.to_string(), provider_id.to_string());
        }

        info!("✅ Initialized authentication for {} consortium providers", self.providers.len());
    }

    /// Generate demo public key (in production, load from HSM/secure storage)
    fn generate_demo_public_key(&self, provider_id: &str) -> VerifyingKey {
        // Create deterministic but demo public key based on provider ID
        let mut key_bytes = [0u8; 32];
        let provider_bytes = provider_id.as_bytes();

        // Fill with provider ID bytes (padded/repeated as needed)
        for (i, &byte) in provider_bytes.iter().cycle().take(32).enumerate() {
            key_bytes[i] = byte;
        }

        // Ensure valid Ed25519 public key format
        key_bytes[31] &= 0x7F; // Clear top bit for valid point

        VerifyingKey::from_bytes(&key_bytes).unwrap_or_else(|_| {
            // Fallback to a known valid key if generation fails
            VerifyingKey::from_bytes(&[
                215, 90, 152, 1, 130, 177, 10, 183, 213, 75, 254, 211, 201, 100, 7, 58,
                14, 225, 114, 243, 218, 166, 35, 37, 175, 2, 26, 104, 247, 7, 81, 26
            ]).unwrap()
        })
    }

    /// Authenticate SP by API key
    pub fn authenticate_by_api_key(&self, api_key: &str) -> Result<AuthenticatedSp, AuthenticationError> {
        let provider_id = self.api_keys.get(api_key)
            .ok_or_else(|| AuthenticationError::InvalidApiKey)?;

        let provider = self.providers.get(provider_id)
            .ok_or_else(|| AuthenticationError::UnknownProvider(provider_id.clone()))?;

        if !provider.active {
            return Err(AuthenticationError::AuthorizationDenied(provider_id.clone()));
        }

        info!("🔐 Authenticated SP: {}", provider.provider_name);

        Ok(AuthenticatedSp {
            provider_id: provider.provider_id.clone(),
            provider_name: provider.provider_name.clone(),
            api_key: api_key.to_string(),
            public_key_bytes: provider.public_key.to_bytes(),
            permissions: provider.permissions.clone(),
        })
    }

    /// Authenticate SP by signature
    pub fn authenticate_by_signature(
        &self,
        provider_id: &str,
        message: &[u8],
        signature: &Signature,
        timestamp: u64,
    ) -> Result<AuthenticatedSp, AuthenticationError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| AuthenticationError::UnknownProvider(provider_id.to_string()))?;

        if !provider.active {
            return Err(AuthenticationError::AuthorizationDenied(provider_id.to_string()));
        }

        // Check timestamp window
        let now = chrono::Utc::now().timestamp() as u64;
        if now.saturating_sub(timestamp) > self.signature_window {
            warn!("⚠️  Signature timestamp expired for {}: {} seconds old",
                  provider.provider_name, now.saturating_sub(timestamp));
            return Err(AuthenticationError::ExpiredTimestamp);
        }

        // Verify signature
        if let Err(_) = provider.public_key.verify(message, signature) {
            error!("❌ Signature verification failed for {}", provider.provider_name);
            return Err(AuthenticationError::InvalidSignature);
        }

        info!("🔐 Authenticated SP by signature: {}", provider.provider_name);

        Ok(AuthenticatedSp {
            provider_id: provider.provider_id.clone(),
            provider_name: provider.provider_name.clone(),
            api_key: provider.api_key.clone(),
            public_key_bytes: provider.public_key.to_bytes(),
            permissions: provider.permissions.clone(),
        })
    }

    /// Check if SP has specific permission
    pub fn check_permission(&self, sp: &AuthenticatedSp, permission: &SpPermission) -> bool {
        sp.permissions.contains(permission)
    }

    /// Authorize SP to submit records for specific network
    pub fn authorize_bce_submission(&self, sp: &AuthenticatedSp, home_network: &str) -> Result<(), AuthenticationError> {
        if !self.check_permission(sp, &SpPermission::SubmitBceRecords) {
            return Err(AuthenticationError::AuthorizationDenied(
                format!("SP {} lacks SubmitBceRecords permission", sp.provider_id)
            ));
        }

        // SP can only submit records where they are the home network
        let expected_home_network = match sp.provider_id.as_str() {
            "tmobile-de" => "T-Mobile-DE",
            "vodafone-uk" => "Vodafone-UK",
            "orange-fr" => "Orange-FR",
            "telenor-no" => "Telenor-NO",
            "sfr-fr" => "SFR-FR",
            _ => return Err(AuthenticationError::UnknownProvider(sp.provider_id.clone())),
        };

        if home_network != expected_home_network {
            warn!("⚠️  SP {} attempted to submit records for {}, but can only submit for {}",
                  sp.provider_id, home_network, expected_home_network);
            return Err(AuthenticationError::AuthorizationDenied(
                format!("SP {} can only submit records for {}", sp.provider_id, expected_home_network)
            ));
        }

        info!("✅ Authorized {} to submit BCE records for {}", sp.provider_id, home_network);
        Ok(())
    }

    /// Get all active consortium providers
    pub fn get_active_providers(&self) -> Vec<String> {
        self.providers.values()
            .filter(|p| p.active)
            .map(|p| p.provider_name.clone())
            .collect()
    }

    /// Revoke API key (security incident response)
    pub fn revoke_api_key(&mut self, api_key: &str) -> bool {
        if let Some(provider_id) = self.api_keys.remove(api_key) {
            if let Some(provider) = self.providers.get_mut(&provider_id) {
                provider.active = false;
                warn!("🚨 Revoked API key for SP: {}", provider.provider_name);
                return true;
            }
        }
        false
    }

    /// Generate authentication statistics
    pub fn get_auth_stats(&self) -> AuthenticationStats {
        let total_providers = self.providers.len();
        let active_providers = self.providers.values().filter(|p| p.active).count();
        let inactive_providers = total_providers - active_providers;

        AuthenticationStats {
            total_providers,
            active_providers,
            inactive_providers,
            signature_window_seconds: self.signature_window,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthenticationStats {
    pub total_providers: usize,
    pub active_providers: usize,
    pub inactive_providers: usize,
    pub signature_window_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consortium_initialization() {
        let auth = SpAuthentication::new_consortium();
        assert_eq!(auth.providers.len(), 5);
        assert_eq!(auth.api_keys.len(), 5);
        assert!(auth.providers.contains_key("tmobile-de"));
        assert!(auth.providers.contains_key("vodafone-uk"));
    }

    #[test]
    fn test_api_key_authentication() {
        let auth = SpAuthentication::new_consortium();

        let result = auth.authenticate_by_api_key("tmobile_api_key_2024_secure");
        assert!(result.is_ok());

        let sp = result.unwrap();
        assert_eq!(sp.provider_id, "tmobile-de");
        assert_eq!(sp.provider_name, "T-Mobile-DE");
        assert!(sp.permissions.contains(&SpPermission::SubmitBceRecords));
    }

    #[test]
    fn test_invalid_api_key() {
        let auth = SpAuthentication::new_consortium();
        let result = auth.authenticate_by_api_key("invalid_key");
        assert!(matches!(result, Err(AuthenticationError::InvalidApiKey)));
    }

    #[test]
    fn test_bce_authorization() {
        let auth = SpAuthentication::new_consortium();
        let sp = auth.authenticate_by_api_key("tmobile_api_key_2024_secure").unwrap();

        // Should authorize T-Mobile to submit for T-Mobile-DE
        let result = auth.authorize_bce_submission(&sp, "T-Mobile-DE");
        assert!(result.is_ok());

        // Should deny T-Mobile submitting for Vodafone
        let result = auth.authorize_bce_submission(&sp, "Vodafone-UK");
        assert!(matches!(result, Err(AuthenticationError::AuthorizationDenied(_))));
    }
}