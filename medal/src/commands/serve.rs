use axum::{
    body::Bytes,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

// Configuration
#[derive(Deserialize, Clone)]
pub struct ServeConfig {
    pub port: u16,
    pub luau: bool,
    pub lua51: bool,
}

// Query parameters for Luau encode key
#[derive(Deserialize)]
struct LuauQuery {
    #[serde(default = "default_encode_key")]
    encode_key: u8,
}

pub const fn default_encode_key() -> u8 { 203 }

// Main server function
pub async fn serve(config: ServeConfig) -> Result<(), std::io::Error> {
    tracing_subscriber::fmt::init();

    // Setup Static File Service
    // ServeDir looks into the "public" folder. 
    // ServeFile is the fallback so visiting "/" returns index.html.
    let serve_dir = ServeDir::new("public")
        .not_found_service(ServeFile::new("public/index.html"));

    let mut app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "OK") }));

    // Register API endpoints based on flags
    if config.luau {
        info!("✅ Luau endpoint: POST /luau/decompile");
        app = app.route("/luau/decompile", post(decompile_luau));
    }

    if config.lua51 {
        info!("✅ Lua 5.1 endpoint: POST /lua51/decompile");
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }

    // Apply CORS and Static Files
    let app = app
        .fallback_service(serve_dir)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🚀 Server starting on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// Handlers
async fn decompile_luau(Query(query): Query<LuauQuery>, body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode".into())); }
    // Call your actual logic here:
    // crate::commands::decompile_no_io(body, query.encode_key, false)
    Ok("-- Luau Decompiled Code".to_string())
}

async fn decompile_lua51(body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode".into())); }
    Ok("-- Lua 5.1 Decompiled Code".to_string())
}

// Error Handling
#[derive(Debug)]
enum AppError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::InternalError(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, msg).into_response()
    }
}
