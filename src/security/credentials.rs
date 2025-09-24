// SP Credential management for consortium
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpCredentials {
    pub provider_id: String,
    pub api_key: String,
    pub public_key_pem: String,
}

pub struct CredentialManager {
    credentials: HashMap<String, SpCredentials>,
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    pub fn load_credentials(&mut self, credentials: Vec<SpCredentials>) {
        for cred in credentials {
            self.credentials.insert(cred.provider_id.clone(), cred);
        }
    }

    pub fn get_credentials(&self, provider_id: &str) -> Option<&SpCredentials> {
        self.credentials.get(provider_id)
    }
}