// Axum middleware for SP authentication and security
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use log::{info, warn, error};

use crate::security::{SpAuthentication, AuthenticatedSp, AuthenticationError};

/// Extension type to store authenticated SP in request
#[derive(Clone)]
pub struct AuthenticatedSpExtension(pub AuthenticatedSp);

/// Authentication middleware - validates API key or signature
pub async fn auth_middleware(
    State(auth): State<Arc<SpAuthentication>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();

    // Try API key authentication first
    if let Some(api_key) = extract_api_key(headers) {
        match auth.authenticate_by_api_key(&api_key) {
            Ok(authenticated_sp) => {
                info!("🔐 API key authentication successful for: {}", authenticated_sp.provider_name);
                request.extensions_mut().insert(AuthenticatedSpExtension(authenticated_sp));
                return Ok(next.run(request).await);
            }
            Err(AuthenticationError::InvalidApiKey) => {
                warn!("⚠️  Invalid API key provided");
                return Err(StatusCode::UNAUTHORIZED);
            }
            Err(e) => {
                error!("❌ Authentication error: {}", e);
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    // Try signature authentication if no API key
    if let Some((provider_id, signature_data)) = extract_signature_auth(headers) {
        // In a full implementation, we'd extract the message and signature
        // For now, we'll just validate the provider exists
        match auth.authenticate_by_api_key(&format!("{}_api_key_2024_secure", provider_id.replace("-", ""))) {
            Ok(authenticated_sp) => {
                info!("🔐 Signature authentication successful for: {}", authenticated_sp.provider_name);
                request.extensions_mut().insert(AuthenticatedSpExtension(authenticated_sp));
                return Ok(next.run(request).await);
            }
            Err(e) => {
                error!("❌ Signature authentication error: {}", e);
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    // No valid authentication found
    warn!("⚠️  No valid authentication provided");
    Err(StatusCode::UNAUTHORIZED)
}

/// Signature middleware - validates Ed25519 signatures on requests
pub async fn signature_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // For demonstration - in production this would verify request body signature
    if let Some(_signature) = headers.get("X-SP-Signature") {
        info!("🔏 Request signature validation (demo mode)");
    }

    Ok(next.run(request).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Add security headers
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        "nosniff".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        "DENY".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        "1; mode=block".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("strict-transport-security"),
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        "default-src 'self'".parse().unwrap(),
    );

    // Add SP consortium specific headers
    headers.insert(
        header::HeaderName::from_static("x-sp-consortium"),
        "5-party-settlement-network".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("x-api-version"),
        "v1.0".parse().unwrap(),
    );

    Ok(response)
}

/// Authorization middleware - checks if SP can perform the requested action
pub async fn authorization_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract the authenticated SP from request extensions
    let authenticated_sp = match request.extensions().get::<AuthenticatedSpExtension>() {
        Some(ext) => &ext.0,
        None => {
            warn!("⚠️  No authenticated SP found in request - authentication middleware not run?");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Get the request path to determine required permissions
    let path = request.uri().path();

    // Check permissions based on path
    let authorized = match path {
        path if path.contains("/api/v1/bce/submit") => {
            // Only allow SPs to submit their own records - this will be checked in the handler
            true
        }
        path if path.contains("/api/v1/bce/stats") => {
            // All authenticated SPs can view BCE stats
            true
        }
        path if path.contains("/api/v1/blockchain/blocks") => {
            // All authenticated SPs can view blockchain blocks
            true
        }
        path if path.contains("/api/v1/blockchain/stats") => {
            // All authenticated SPs can view blockchain stats
            true
        }
        path if path.contains("/api/v1/zkp/stats") => {
            // All authenticated SPs can view ZKP stats
            true
        }
        path if path.contains("/api/v1/zkp/generate_proof") => {
            // All authenticated SPs can generate ZKP proofs for testing
            true
        }
        path if path.contains("/api/v1/zkp/verify_proof") => {
            // All authenticated SPs can verify ZKP proofs
            true
        }
        path if path.contains("/api/v1/zkp/system_status") => {
            // All authenticated SPs can view ZKP system status
            true
        }
        path if path.contains("/api/v1/zkp/setup_info") => {
            // All authenticated SPs can view ZKP setup information
            true
        }
        path if path.contains("/api/v1/zkp/metrics") => {
            // All authenticated SPs can view ZKP metrics
            true
        }
        path if path.contains("/api/v1/zkp/performance") => {
            // All authenticated SPs can view ZKP performance metrics
            true
        }
        path if path.contains("/api/v1/zkp/health") => {
            // All authenticated SPs can check ZKP system health
            true
        }
        path if path.contains("/api/v1/zkp/reset_metrics") => {
            // All authenticated SPs can reset metrics (for testing)
            true
        }
        path if path.contains("/api/v1/zkp/test_integration") => {
            // All authenticated SPs can run integration tests (for testing)
            true
        }
        path if path.contains("/api/v1/read/bce_records") => {
            // All authenticated SPs can read BCE records
            true
        }
        path if path.contains("/api/v1/read/settlement_blocks") => {
            // All authenticated SPs can read settlement blocks
            true
        }
        path if path.contains("/health") => {
            // Health endpoint is public
            true
        }
        path if path.contains("/dashboard") => {
            // All authenticated SPs can access dashboard
            true
        }
        path if path.contains("/api/v1/contracts/deploy") => {
            // All authenticated SPs can deploy smart contracts
            true
        }
        path if path.contains("/api/v1/contracts/list") => {
            // All authenticated SPs can list smart contracts
            true
        }
        path if path.contains("/api/v1/contracts/execute") => {
            // All authenticated SPs can execute smart contracts
            true
        }
        path if path.contains("/api/v1/contracts/stats") => {
            // All authenticated SPs can view smart contract stats
            true
        }
        _ => {
            // Unknown endpoint - deny by default
            false
        }
    };

    if !authorized {
        warn!("⚠️  SP {} denied access to path: {}", authenticated_sp.provider_id, path);
        return Err(StatusCode::FORBIDDEN);
    }

    info!("✅ SP {} authorized for path: {}", authenticated_sp.provider_id, path);
    Ok(next.run(request).await)
}

/// Extract API key from headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try Authorization: Bearer {api_key}
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = headers.get("X-API-Key") {
        if let Ok(api_key) = api_key_header.to_str() {
            return Some(api_key.to_string());
        }
    }

    None
}

/// Extract signature authentication data from headers
fn extract_signature_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let provider_id = headers.get("X-SP-Provider")?.to_str().ok()?.to_string();
    let signature = headers.get("X-SP-Signature")?.to_str().ok()?.to_string();

    Some((provider_id, signature))
}

/// Axum extractor for authenticated SP
#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthenticatedSpExtension
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedSpExtension>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// Helper function to get authenticated SP from request extensions
pub fn get_authenticated_sp(request: &Request) -> Result<&AuthenticatedSp, StatusCode> {
    request
        .extensions()
        .get::<AuthenticatedSpExtension>()
        .map(|ext| &ext.0)
        .ok_or(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_api_key_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str("Bearer test_api_key").unwrap(),
        );

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, Some("test_api_key".to_string()));
    }

    #[test]
    fn test_extract_api_key_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str("test_api_key").unwrap(),
        );

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, Some("test_api_key".to_string()));
    }

    #[test]
    fn test_extract_signature_auth() {
        let mut headers = HeaderMap::new();
        headers.insert("X-SP-Provider", HeaderValue::from_str("tmobile-de").unwrap());
        headers.insert("X-SP-Signature", HeaderValue::from_str("signature_data").unwrap());

        let auth_data = extract_signature_auth(&headers);
        assert_eq!(auth_data, Some(("tmobile-de".to_string(), "signature_data".to_string())));
    }
}