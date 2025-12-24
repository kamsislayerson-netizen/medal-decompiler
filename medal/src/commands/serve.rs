use axum::{
    body::Bytes,
    extract::Query,
    http::{StatusCode, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tracing::{info, error, warn};
use crate::commands::decompile_no_io;

#[derive(Deserialize)]
pub struct ServeConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub luau: bool,
    #[serde(default)]
    pub lua51: bool,
}

fn default_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000)
}

#[derive(Deserialize)]
pub struct LuauQuery {
    #[serde(default = "default_encode_key")]
    encode_key: u8,
}

fn default_encode_key() -> u8 {
    203
}

pub async fn serve(config: ServeConfig) -> Result<(), std::io::Error> {
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(root));

    // Enable CORS for web browser access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(3600));

    // Add decompile endpoints based on configuration
    if config.luau {
        info!("Luau decompile endpoint enabled at /luau/decompile");
        app = app.route("/luau/decompile", post(decompile_luau));
    }

    if config.lua51 {
        info!("Lua 5.1 decompile endpoint enabled at /lua51/decompile");
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }

    let app = app.layer(cors);

    // Bind to 0.0.0.0 for Render compatibility
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🚀 Server listening on {}", addr);
    info!("📍 API endpoint: http://{}{}", addr, if config.luau { "/luau/decompile" } else { "" });

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn root() -> impl IntoResponse {
    "🎯 Luau Decompiler API is running! Use POST /luau/decompile or /lua51/decompile"
}

async fn decompile_luau(
    Query(query): Query<LuauQuery>,
    body: Bytes,
) -> Result<String, AppError> {
    if body.is_empty() {
        return Err(AppError::BadRequest("No bytecode provided".to_string()));
    }

    if body.len() < 4 {
        return Err(AppError::BadRequest("Invalid bytecode: too short (minimum 4 bytes)".to_string()));
    }

    info!("Decompiling Luau bytecode ({} bytes, encode_key: {})", body.len(), query.encode_key);
    
    let result = decompile_no_io(body, query.encode_key, false)
        .map_err(|e| AppError::InternalError(format!("Decompilation failed: {}", e)))?;

    if result.trim().is_empty() {
        return Err(AppError::InternalError("Decompilation returned empty output".to_string()));
    }

    Ok(result)
}

async fn decompile_lua51(body: Bytes) -> Result<String, AppError> {
    if body.is_empty() {
        return Err(AppError::BadRequest("No bytecode provided".to_string()));
    }

    if body.len() < 4 {
        return Err(AppError::BadRequest("Invalid bytecode: too short (minimum 4 bytes)".to_string()));
    }

    info!("Decompiling Lua 5.1 bytecode ({} bytes)", body.len());
    
    let result = decompile_no_io(body, default_encode_key(), true)
        .map_err(|e| AppError::InternalError(format!("Decompilation failed: {}", e)))?;

    if result.trim().is_empty() {
        return Err(AppError::InternalError("Decompilation returned empty output".to_string()));
    }

    Ok(result)
}

// Error handling
#[derive(Debug)]
enum AppError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };
        
        (status, message).into_response()
    }
}

// CLI integration (called from main.rs)
pub async fn serve_command(port: u16, luau: bool, lua51: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = ServeConfig { port, luau, lua51 };
    serve(config).await?;
    Ok(())
}
