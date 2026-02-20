mod aggregation;
mod analytics;
mod cache;
mod error;
mod handlers;
mod metrics;
mod metrics_handler;
mod resource_handlers;
mod resource_tracking;
mod routes;
mod state;

use anyhow::Result;
use axum::http::{header, HeaderValue, Method};
use axum::{middleware, Router};
use dotenv::dotenv;
use prometheus::Registry;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../../database/migrations")
        .run(&pool)
        .await?;

    popularity::spawn_popularity_task(pool.clone());
    aggregation::spawn_aggregation_task(pool.clone());

    let registry = Registry::new_custom(Some("api".into()), None)?;
    metrics::register_all(&registry)?;
    let state = AppState::new(pool, registry);

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("https://soroban-registry.vercel.app"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .merge(routes::contract_routes())
        .merge(routes::publisher_routes())
        .merge(routes::health_routes())
        .merge(routes::migration_routes())
        .merge(routes::resource_routes())
        .merge(routes::observability_routes())
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(request_logger))
        .layer(CorsLayer::permissive())
        .layer(cors)
        .with_state(state);

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

mod popularity {
    pub fn spawn_popularity_task(_pool: sqlx::PgPool) {}
}
