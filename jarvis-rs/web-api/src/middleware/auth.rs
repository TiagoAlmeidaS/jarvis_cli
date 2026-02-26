//! Authentication middleware.

use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use tracing::warn;

use crate::state::AppState;

/// Authentication middleware that validates Bearer tokens.
pub async fn validate_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for health check
    if request.uri().path() == "/api/health" {
        return Ok(next.run(request).await);
    }

    // Skip auth for static files
    if !request.uri().path().starts_with("/api/") {
        return Ok(next.run(request).await);
    }

    // Skip auth for WebSocket (it validates token in query string)
    if request.uri().path() == "/ws/daemon" {
        return Ok(next.run(request).await);
    }

    // Get API config
    let api_config = state.config.api.as_ref().ok_or_else(|| {
        warn!("API not configured");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    // Extract Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        warn!("Invalid Authorization header format");
        StatusCode::UNAUTHORIZED
    })?;

    // Validate token against configured API key
    if token != api_config.api_key {
        warn!("Invalid API key");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_app_state_with_api_key;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use axum_test::TestServer;

    async fn test_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_token() {
        let api_key = "test-api-key-123".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());

        let app = Router::new()
            .route("/api/test", get(test_handler))
            .layer(from_fn_with_state(app_state.clone(), validate_auth))
            .with_state(app_state);

        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/test")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), "ok");
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_token() {
        let api_key = "test-api-key-123".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);

        let app = Router::new()
            .route("/api/test", get(test_handler))
            .layer(from_fn_with_state(app_state.clone(), validate_auth))
            .with_state(app_state);

        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/test")
            .add_header("Authorization", "Bearer wrong-key")
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_header() {
        let api_key = "test-api-key-123".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);

        let app = Router::new()
            .route("/api/test", get(test_handler))
            .layer(from_fn_with_state(app_state.clone(), validate_auth))
            .with_state(app_state);

        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/test").await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_skips_health() {
        let api_key = "test-api-key-123".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);

        let app = Router::new()
            .route("/api/health", get(|| async { "healthy" }))
            .layer(from_fn_with_state(app_state.clone(), validate_auth))
            .with_state(app_state);

        let server = TestServer::new(app).unwrap();

        // Should work without auth
        let response = server.get("/api/health").await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), "healthy");
    }

    #[tokio::test]
    async fn test_auth_middleware_skips_static_files() {
        let api_key = "test-api-key-123".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);

        let app = Router::new()
            .route("/static/test", get(|| async { "static content" }))
            .layer(from_fn_with_state(app_state.clone(), validate_auth))
            .with_state(app_state);

        let server = TestServer::new(app).unwrap();

        // Should work without auth
        let response = server.get("/static/test").await;

        response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), "static content");
    }
}
