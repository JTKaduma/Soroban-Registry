#![allow(dead_code, unused)]

mod aggregation;
mod analytics;
mod breaking_changes;
mod cache;
mod compatibility_testing_handlers;
mod custom_metrics_handlers;
mod migration_handlers;
mod dependency;
mod deprecation_handlers;
mod error;
mod handlers;
mod health;
pub mod health_monitor;
#[cfg(test)]
mod health_tests;
mod metrics;
mod metrics_handler;
mod rate_limit;
pub mod request_tracing;
mod routes;
mod release_notes_handlers;
mod release_notes_routes;
pub mod signing_handlers;
mod state;
mod type_safety;
mod validation;
// mod auth;
// mod auth_handlers;
// mod resource_handlers;
// mod resource_tracking;

use anyhow::Result;
use axum::http::{header, HeaderValue, Method};
use axum::{middleware, Router};
use dotenv::dotenv;
use prometheus::Registry;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::rate_limit::RateLimitState;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize structured JSON tracing (ELK/Splunk compatible)
    request_tracing::init_json_tracing();

    // Database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("../../database/migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database connected and migrations applied");

    // Check migration versioning state on startup (Issue #252)
    migration_handlers::check_migrations_on_startup(&pool).await;

    // Spawn the hourly analytics aggregation background task
    aggregation::spawn_aggregation_task(pool.clone());

    // Create prometheus registry for metrics
    let registry = Registry::new();
    if let Err(e) = crate::metrics::register_all(&registry) {
        tracing::error!("Failed to register metrics: {}", e);
    }

    // Create app state
    let is_shutting_down = Arc::new(AtomicBool::new(false));
    let state = AppState::new(pool.clone(), registry, is_shutting_down.clone());
    
    // Warm up the cache
    state.cache.clone().warm_up(pool.clone());

    let rate_limit_state = RateLimitState::from_env();

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("https://soroban-registry.vercel.app"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    // Build router
    let app = Router::new()
        .merge(routes::contract_routes())
        .merge(routes::publisher_routes())
        .merge(routes::health_routes())
        .merge(routes::migration_routes())
        .merge(routes::compatibility_dashboard_routes())
        .merge(release_notes_routes::release_notes_routes())
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(request_tracing::tracing_middleware))
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit::rate_limit_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!(
            "SIGTERM/SIGINT received. Failing health checks and stopping new requests..."
        );
        is_shutting_down.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = tx.send(());
    });

    tokio::select! {
        res = server => {
            if let Err(e) = res {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = async {
            let _ = rx.await;
            tracing::info!("Draining active requests (timeout: 30s)...");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            tracing::warn!("Drain timeout reached. Forcing shutdown...");
        } => {}
    }

    tracing::info!("Closing database connections...");
    pool.close().await;
    tracing::info!("Shutdown complete");

    Ok(())
}


