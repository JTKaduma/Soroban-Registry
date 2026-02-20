mod aggregation;
mod analytics;
mod audit_handlers;
mod audit_routes;
mod benchmark_engine;
mod benchmark_handlers;
mod benchmark_routes;
mod cache;
mod cache_benchmark;
mod checklist;
mod contract_history_handlers;
mod contract_history_routes;
mod detector;
mod error;
mod handlers;
mod metrics;
mod metrics_handler;
mod models;
mod multisig_handlers;
mod multisig_routes;
mod observability;
mod popularity;
mod rate_limit;
mod residency_handlers;
mod residency_routes;
mod routes;
mod state;
mod template_handlers;
mod template_routes;
mod trust;
mod health_monitor;
mod migration_cli;

use anyhow::Result;
use axum::http::{header, HeaderValue, Method};
use axum::{middleware, Router};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::observability::Observability;
use crate::rate_limit::RateLimitState;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    let obs = Observability::init()?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let migration_command = migration_cli::parse_command(&args)?;

    // Database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations by default, or execute migration subcommands.
    if let Some(command) = migration_command {
        migration_cli::execute(command, &pool).await?;
        return Ok(());
    }

    sqlx::migrate!("../../database/migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database connected and migrations applied");

    // Spawn background popularity scoring job (runs hourly)
    popularity::spawn_popularity_task(pool.clone());
    // Spawn the hourly analytics aggregation background task
    aggregation::spawn_aggregation_task(pool.clone());

    // Create app state
    let state = AppState::new(pool, obs.registry);
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
        .merge(routes::canary_routes())
        .merge(routes::ab_test_routes())
        .merge(routes::performance_routes())
        .merge(multisig_routes::multisig_routes())
        .merge(audit_routes::audit_routes())
        .merge(benchmark_routes::benchmark_routes())
        .merge(contract_history_routes::contract_history_routes())
        .merge(template_routes::template_routes())
        .merge(routes::observability_routes())
        .merge(residency_routes::residency_routes())
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(request_logger))
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn request_logger(
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let elapsed = start.elapsed().as_millis();
    let status = response.status().as_u16();

    tracing::info!("{method} {uri} {status} {elapsed}ms");

    response
}
        .merge(routes::publisher_routes())
