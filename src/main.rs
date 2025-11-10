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
use std::net::SocketAddr;

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
    
    // Start server (axum 0.8 style)
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server running on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}