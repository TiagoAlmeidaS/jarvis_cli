//! Health check endpoint.

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: &'static str,
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::create_router;
    use crate::test_utils::create_test_app_state;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_check() {
        let app_state = create_test_app_state();
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/health").await;

        response.assert_status(StatusCode::OK);
        let body: HealthResponse = response.json();
        assert_eq!(body.status, "ok");
        assert!(!body.version.is_empty());
    }

    #[tokio::test]
    async fn test_health_check_no_auth_required() {
        let app_state = create_test_app_state();
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        // Should work without Authorization header
        let response = server.get("/api/health").await;

        response.assert_status(StatusCode::OK);
    }
}
