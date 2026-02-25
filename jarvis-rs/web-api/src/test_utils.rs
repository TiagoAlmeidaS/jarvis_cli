//! Test utilities for web-api tests.

use crate::state::AppState;
use jarvis_core::config::Config;
use jarvis_core::config::types::ApiConfig;
use jarvis_core::auth::AuthManager;
use jarvis_core::models_manager::manager::ModelsManager;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test AppState with a given API key.
pub fn create_test_app_state_with_api_key(api_key: String) -> AppState {
    let temp_dir = TempDir::new().unwrap();
    let jarvis_home = temp_dir.path().to_path_buf();
    
    let mut config = Config::load_from_base_config_with_overrides(
        jarvis_core::config::ConfigToml::default(),
        jarvis_core::config::ConfigOverrides::default(),
        jarvis_home.clone(),
    ).unwrap();
    
    config.api = Some(ApiConfig {
        api_key: api_key.clone(),
        port: 3000,
        bind_address: "0.0.0.0".to_string(),
        enable_cors: false,
    });
    
    let config = Arc::new(config);
    
    let auth_manager = Arc::new(AuthManager::new(
        jarvis_home.clone(),
        true,
        jarvis_core::auth::AuthCredentialsStoreMode::Auto,
    ));
    
    let models_manager = Arc::new(ModelsManager::new(
        jarvis_home,
        auth_manager.clone(),
    ));
    
    AppState {
        config,
        auth_manager,
        models_manager,
    }
}

/// Create a test AppState with default API key.
pub fn create_test_app_state() -> AppState {
    create_test_app_state_with_api_key("test-api-key".to_string())
}
