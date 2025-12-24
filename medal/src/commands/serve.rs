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
use crate::commands::decompile_no_io;

#[derive(Deserialize, Clone)]
pub struct ServeConfig {
    pub port: u16,
    pub luau: bool,
    pub lua51: bool,
}

#[derive(Deserialize)]
struct LuauQuery {
    #[serde(default = "default_encode_key")]
    encode_key: u8,
}

pub const fn default_encode_key() -> u8 { 203 }

pub async fn serve(config: ServeConfig) -> Result<(), std::io::Error> {
    tracing_subscriber::fmt::init();

    // 1. Define the static file handler
    // This serves files from the "public" directory.
    // The fallback ensures that visiting the root "/" sends the index.html.
    let serve_dir = ServeDir::new("public")
        .not_found_service(ServeFile::new("public/index.html"));

    let mut app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "OK") }));

    // 2. Register Decompiler API Routes
    if config.luau {
        info!("✅ Luau endpoint enabled");
        app = app.route("/luau/decompile", post(decompile_luau));
    }

    if config.lua51 {
        info!("✅ Lua 5.1 endpoint enabled");
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }

    // 3. Layering: Add CORS and the Static File Fallback
    let app = app
        .fallback_service(serve_dir)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🚀 Server running on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// --- Handlers ---

async fn decompile_luau(Query(query): Query<LuauQuery>, body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode provided".into())); }
    
    // Call the internal decompiler logic
    let result = decompile_no_io(body, query.encode_key, false)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(result)
}

async fn decompile_lua51(body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode provided".into())); }
    
    let result = decompile_no_io(body, default_encode_key(), true)
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(result)
}

// --- Error Types ---

#[derive(Debug)]
enum AppError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::InternalError(m) => {
                error!("Decompile Error: {}", m);
                (StatusCode::INTERNAL_SERVER_ERROR, m)
            }
        };
        (status, msg).into_response()
    }
}
