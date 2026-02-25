//! Server setup and configuration.

use anyhow::Result;
use axum::Router;
use jarvis_common::CliConfigOverrides;
use jarvis_core::config::Config;
use jarvis_core::config::ConfigBuilder;
use jarvis_core::config::find_jarvis_home;
use jarvis_core::config::types::ApiConfig;
use jarvis_core::auth::AuthManager;
use jarvis_core::models_manager::manager::ModelsManager;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::middleware::auth;
use crate::state::AppState;

pub fn create_router(app_state: AppState, enable_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/api/health", axum::routing::get(handlers::health::health_check))
        .route("/api/chat", axum::routing::post(handlers::chat::handle_chat))
        .route("/api/threads", axum::routing::get(handlers::threads::list_threads))
        .route("/api/config", axum::routing::get(handlers::config::get_config))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn_with_state(
                    app_state.clone(),
                    auth::validate_auth,
                ))
        )
        .with_state(app_state);

        // Add CORS layer if enabled
        if enable_cors {
            router = router.layer(
                CorsLayer::permissive()
                    .allow_origin(tower_http::cors::Any)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(tower_http::cors::Any)
            );
        }

    // Serve static files from the web-api/static directory
    // In production, files should be embedded or served via reverse proxy
    // Try multiple paths for different deployment scenarios
    let static_paths = vec![
        "jarvis-rs/web-api/static",
        "web-api/static",
        "static",
        "./static",
    ];
    
    // Find the first existing static directory
    let mut static_dir_path: Option<String> = None;
    for static_path in static_paths {
        let static_dir = std::path::Path::new(static_path);
        if static_dir.exists() {
            static_dir_path = Some(static_path.to_string());
            tracing::info!("Serving static files from: {}", static_path);
            break;
        }
    }
    
    // Use fallback_service instead of nest_service for root path
    if let Some(static_path) = static_dir_path {
        router = router.fallback_service(
            tower_http::services::ServeDir::new(static_path)
        );
    }

    router
}

pub async fn run_server(config_overrides: CliConfigOverrides) -> Result<()> {
    // Load configuration
    let jarvis_home = find_jarvis_home().map_err(|e| anyhow::anyhow!("Failed to find jarvis home: {}", e))?;
    let cli_kv_overrides = config_overrides.parse_overrides().map_err(|e| anyhow::anyhow!("Failed to parse config overrides: {}", e))?;
    
    let config = ConfigBuilder::default()
        .jarvis_home(jarvis_home.clone())
        .cli_overrides(cli_kv_overrides)
        .build()
        .await?;
    let config = Arc::new(config);

    // Initialize core services
    let auth_manager = Arc::new(AuthManager::new(
        jarvis_home.clone(),
        true, // enable_codex_api_key_env
        jarvis_core::auth::AuthCredentialsStoreMode::Auto,
    ));

    let models_manager = Arc::new(ModelsManager::new(
        jarvis_home.clone(),
        auth_manager.clone(),
    ));

    // Create app state
    let app_state = AppState {
        config: config.clone(),
        auth_manager,
        models_manager,
    };

    // Get API configuration
    // Port can be overridden by PORT environment variable (for cloud services)
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or_else(|| {
            let default_api_config = ApiConfig {
                port: 3000,
                api_key: String::new(),
                bind_address: "0.0.0.0".to_string(),
                enable_cors: false,
            };
            config.api.as_ref().unwrap_or(&default_api_config).port
        });
    
    let default_api_config = ApiConfig {
        port,
        api_key: String::new(),
        bind_address: "0.0.0.0".to_string(),
        enable_cors: false,
    };
    let api_config = config.api.as_ref().unwrap_or(&default_api_config);
    let bind_address = api_config.bind_address.clone();
    let enable_cors = api_config.enable_cors;

    // Build application
    let app = create_router(app_state, enable_cors);

    // Start server
    let addr = format!("{}:{}", bind_address, port);

    tracing::info!("Jarvis API server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
