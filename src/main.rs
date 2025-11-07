// src/main.rs
mod routes;
mod handlers;
mod models;
mod database;
mod middleware;
mod state;
mod error;

use axum::{routing::get, Router};
use dotenvy::dotenv;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::init();
    
    // Load environment variables
    dotenv().ok();
    
    // Create database pool
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let db_pool = database::create_pool(&database_url).await
        .expect("Failed to create database pool");
    
    // Create application state
    let app_state = state::AppState::new(db_pool);
    
    // Build application
    let app = Router::new()
        .route("/", get(|| async { "DairyX API" }))
        .route("/health", get(health_check))
        .with_state(app_state);
    
    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server running on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}