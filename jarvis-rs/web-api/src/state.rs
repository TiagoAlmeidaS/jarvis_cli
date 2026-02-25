//! Application state shared across handlers.

use jarvis_core::config::Config;
use jarvis_core::auth::AuthManager;
use jarvis_core::models_manager::manager::ModelsManager;
use std::sync::Arc;

/// Application state containing core services.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub auth_manager: Arc<AuthManager>,
    pub models_manager: Arc<ModelsManager>,
}
