//! Configuration endpoint.

use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConfigResponse {
    pub model_provider: String,
    pub model: Option<String>,
    pub port: Option<u16>,
    pub bind_address: Option<String>,
}

pub async fn get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let api_config = state.config.api.as_ref();

    Json(ConfigResponse {
        model_provider: state.config.model_provider_id.clone(),
        model: state.config.model.clone(),
        port: api_config.map(|c| c.port),
        bind_address: api_config.map(|c| c.bind_address.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::create_router;
    use crate::test_utils::create_test_app_state_with_api_key;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_get_config_with_auth() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/config")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .await;

        response.assert_status(StatusCode::OK);
        let body: ConfigResponse = response.json();
        assert_eq!(body.port, Some(3000));
        assert_eq!(body.bind_address, Some("0.0.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_get_config_requires_auth() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/config").await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}
