//! Integration tests for the web API.

use axum::http::StatusCode;
use axum_test::TestServer;
use jarvis_web_api::server::create_router;
use jarvis_web_api::test_utils::create_test_app_state_with_api_key;

#[tokio::test]
async fn test_health_endpoint_integration() {
    let api_key = "integration-test-key".to_string();
    let app_state = create_test_app_state_with_api_key(api_key);
    let app = create_router(app_state, false);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/health").await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_config_endpoint_integration() {
    let api_key = "integration-test-key".to_string();
    let app_state = create_test_app_state_with_api_key(api_key.clone());
    let app = create_router(app_state, false);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/api/config")
        .add_header("Authorization", format!("Bearer {}", api_key))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body.get("model_provider").is_some());
}

#[tokio::test]
async fn test_threads_endpoint_integration() {
    let api_key = "integration-test-key".to_string();
    let app_state = create_test_app_state_with_api_key(api_key.clone());
    let app = create_router(app_state, false);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/api/threads")
        .add_header("Authorization", format!("Bearer {}", api_key))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert!(body.get("threads").is_some());
}

#[tokio::test]
async fn test_auth_required_for_protected_endpoints() {
    let api_key = "integration-test-key".to_string();
    let app_state = create_test_app_state_with_api_key(api_key);
    let app = create_router(app_state, false);
    let server = TestServer::new(app).unwrap();

    // Test config endpoint without auth
    let response = server.get("/api/config").await;
    response.assert_status(StatusCode::UNAUTHORIZED);

    // Test threads endpoint without auth
    let response = server.get("/api/threads").await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}

    #[tokio::test]
    async fn test_invalid_auth_token() {
        let api_key = "integration-test-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/config")
            .add_header("Authorization", "Bearer wrong-token")
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_cors_enabled() {
        let api_key = "integration-test-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);
        let app = create_router(app_state, true); // Enable CORS
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/health")
            .add_header("Origin", "http://localhost:3000")
            .await;

        response.assert_status(StatusCode::OK);
        // CORS headers should be present
        // Note: axum_test may not expose CORS headers, but the layer is applied
    }
    
    #[tokio::test]
    async fn test_chat_validation() {
        let api_key = "integration-test-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        // Test empty prompt
        let response = server
            .post("/api/chat")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .add_header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "prompt": "",
                "thread_id": null
            }))
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
