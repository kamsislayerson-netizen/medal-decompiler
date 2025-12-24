use axum::{
    body::Bytes,
    extract::Query,
    http::{StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use tracing::{info, error, warn};
use crate::commands::decompile_no_io;

// Configuration
#[derive(Deserialize, Clone)]
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

// Query parameters for encode key
#[derive(Deserialize)]
struct LuauQuery {
    #[serde(default = "default_encode_key")]
    encode_key: u8,
}

#[inline]
pub const fn default_encode_key() -> u8 {
    203
}

// Main server function
pub async fn serve(config: ServeConfig) -> Result<(), std::io::Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let mut app = Router::new()
        .route("/health", get(health_check));

    // Serve static files from public directory
    // This will serve index.html at /, and other files like /style.css, /script.js, etc.
    app = app.nest_service("/", ServeDir::new("public"));

    // Add CORS for browser frontend access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(3600));

    // Register endpoints based on feature flags
    if config.luau {
        info!("✅ Luau endpoint: POST /luau/decompile?encode_key=<0-255>");
        app = app.route("/luau/decompile", post(decompile_luau));
    }

    if config.lua51 {
        info!("✅ Lua 5.1 endpoint: POST /lua51/decompile");
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }

    let app = app.layer(cors);

    // Bind to 0.0.0.0:PORT for Render compatibility
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🚀 Starting server on {}", addr);
    info!("💡 Health check: http://{}/health", addr);
    info!("📁 Serving static files from: ./public");
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
}

// Health check (required by Render)
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// Luau decompilation handler
async fn decompile_luau(
    Query(query): Query<LuauQuery>,
    body: Bytes,
) -> Result<String, AppError> {
    validate_body(&body)?;
    
    info!("Decompiling Luau: {} bytes, encode_key={}", body.len(), query.encode_key);
    
    let result = decompile_no_io(body, query.encode_key, false)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    if result.trim().is_empty() {
        return Err(AppError::InternalError("Empty decompilation result".to_string()));
    }

    Ok(result)
}

// Lua 5.1 decompilation handler
async fn decompile_lua51(body: Bytes) -> Result<String, AppError> {
    validate_body(&body)?;
    
    info!("Decompiling Lua 5.1: {} bytes", body.len());
    
    let result = decompile_no_io(body, default_encode_key(), true)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    if result.trim().is_empty() {
        return Err(AppError::InternalError("Empty decompilation result".to_string()));
    }

    Ok(result)
}

// Input validation
fn validate_body(body: &Bytes) -> Result<(), AppError> {
    if body.is_empty() {
        return Err(AppError::BadRequest("No bytecode provided".to_string()));
    }
    if body.len() < 4 {
        return Err(AppError::BadRequest("Bytecode too short (minimum 4 bytes)".to_string()));
    }
    Ok(())
}

// Error types
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

// CLI integration
pub async fn serve_command(port: u16, luau: bool, lua51: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !luau && !lua51 {
        return Err("❌ At least one of --luau or --lua51 must be enabled".into());
    }
    serve(ServeConfig { port, luau, lua51 }).await?;
    Ok(())
}
