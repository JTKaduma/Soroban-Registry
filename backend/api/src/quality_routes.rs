// backend/api/src/quality_routes.rs

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    quality_handlers::{
        compute_contract_quality, get_contract_quality, get_quality_benchmark,
        get_quality_trend, set_quality_threshold,
    },
    state::AppState,
};

pub fn quality_routes() -> Router<AppState> {
    Router::new()
        // Core quality endpoints
        .route(
            "/contracts/:id/quality",
            get(get_contract_quality).post(compute_contract_quality),
        )
        // Historical trend for charting
        .route("/contracts/:id/quality/trend", get(get_quality_trend))
        // Category benchmark comparison
        .route("/contracts/:id/quality/benchmark", get(get_quality_benchmark))
        // Set / update quality thresholds (gates)
        .route("/contracts/:id/quality/threshold", post(set_quality_threshold))
}