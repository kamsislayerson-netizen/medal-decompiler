use ax_um::{
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

// Configuration for Render and Feature Flags
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

    // Setup the Static File Service
    // ServeDir handles assets; ServeFile ensures the root / returns index.html
    let serve_dir = ServeDir::new("public")
        .not_found_service(ServeFile::new("public/index.html"));

    let mut app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "OK") }));

    // API Routes
    if config.luau {
        info!("✅ Luau endpoint active: /luau/decompile");
        app = app.route("/luau/decompile", post(decompile_luau));
    }

    if config.lua51 {
        info!("✅ Lua 5.1 endpoint active: /lua51/decompile");
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }

    // Combine API with Static Files and CORS
    let app = app
        .fallback_service(serve_dir) 
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🚀 Decompiler active at http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

// Handlers for decompilation
async fn decompile_luau(Query(query): Query<LuauQuery>, body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode provided".into())); }
    // Call your actual logic: crate::commands::decompile_no_io(body, query.encode_key, false)
    Ok("-- Result from Luau Decompiler".to_string())
}

async fn decompile_lua51(body: Bytes) -> Result<String, AppError> {
    if body.is_empty() { return Err(AppError::BadRequest("No bytecode provided".into())); }
    Ok("-- Result from Lua 5.1 Decompiler".to_string())
}

// Error Management
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
