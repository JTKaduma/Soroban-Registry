use axum::{routing::post, Router};

use crate::{cost_handlers, state::AppState};

pub fn cost_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/cost-estimate",
            post(cost_handlers::estimate_cost),
        )
        .route(
            "/api/contracts/:id/cost-estimate/batch",
            post(cost_handlers::batch_estimate),
        )
        .route(
            "/api/contracts/:id/cost-estimate/optimize",
            post(cost_handlers::optimize_costs),
        )
        .route(
            "/api/contracts/:id/cost-estimate/forecast",
            post(cost_handlers::forecast_costs),
        )
}
