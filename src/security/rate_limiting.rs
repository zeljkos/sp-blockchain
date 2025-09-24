// Rate limiting for SP API endpoints
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            burst_limit: 20,
        }
    }
}

pub struct RateLimiter {
    config: RateLimitConfig,
    client_requests: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            client_requests: HashMap::new(),
        }
    }

    pub fn check_rate_limit(&mut self, client_id: &str) -> bool {
        let now = Instant::now();
        let minute_ago = now - Duration::from_secs(60);

        let requests = self.client_requests.entry(client_id.to_string()).or_insert_with(Vec::new);

        // Remove old requests
        requests.retain(|&time| time > minute_ago);

        // Check limits
        if requests.len() as u32 >= self.config.requests_per_minute {
            return false;
        }

        requests.push(now);
        true
    }
}