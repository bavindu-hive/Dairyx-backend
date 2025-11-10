// src/main.rs
mod routes;
mod handlers;
mod models;
mod database;
mod middleware;
mod state;
mod dtos; // expose DTO modules
mod error;
mod auth; // expose auth module

use axum::{routing::get, Router};
use tracing_subscriber::fmt::init as tracing_init;
use tokio::net::TcpListener;
use dotenvy::dotenv;
use std::net::{SocketAddr, IpAddr};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_init();
    
    // Load environment variables
    dotenv().ok();
    
    // Create database pool
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let db_pool = database::create_pool(&database_url).await
        .expect("Failed to create database pool");
    
    // Create application state
    let app_state = state::AppState::new(db_pool);
    
    // Build application under /DairyX base path
    let api = routes::create_router()
        .route("/", get(|| async { "DairyX API" }))
        .route("/health", get(health_check));

    let app = Router::new()
        .nest("/DairyX", api)
        .with_state(app_state);
    
    // Start server (axum 0.8 style) with HOST/PORT env and graceful port selection
    let host_str = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let host: IpAddr = host_str.parse().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
    let base_port = std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(3000);

    // Try base_port..base_port+20 to avoid crash when address is in use
    let listener = {
        let mut bound = None;
        for offset in 0u16..=20 {
            let port = base_port.saturating_add(offset);
            let addr = SocketAddr::from((host, port));
            match TcpListener::bind(addr).await {
                Ok(l) => { bound = Some((l, addr)); break; }
                Err(e) => {
                    if offset == 0 { tracing::warn!(%addr, error=%e, "Port in use, trying next"); }
                }
            }
        }
        match bound {
            Some((l, addr)) => {
                tracing::info!("Server running on {}", addr);
                l
            }
            None => {
                tracing::error!("Failed to bind to any port starting at {} on {}", base_port, host);
                return;
            }
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error=%e, "Server error");
    }
}

async fn health_check() -> &'static str {
    "OK"
}