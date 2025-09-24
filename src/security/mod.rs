// Security module for Phase 3 hardening
pub mod auth;
pub mod middleware;
pub mod credentials;
pub mod rate_limiting;

pub use auth::{SpAuthentication, AuthenticationError, AuthenticatedSp};
pub use middleware::{auth_middleware, signature_middleware, security_headers_middleware};
pub use credentials::{SpCredentials, CredentialManager};
pub use rate_limiting::{RateLimiter, RateLimitConfig};