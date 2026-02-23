use axum::{routing::get, routing::put, Router};

use crate::{maturity_handlers, state::AppState};

pub fn maturity_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/maturity",
            put(maturity_handlers::update_maturity),
        )
        .route(
            "/api/contracts/:id/maturity/history",
            get(maturity_handlers::get_maturity_history),
        )
        .route(
            "/api/contracts/:id/maturity/requirements",
            get(maturity_handlers::check_maturity_requirements),
        )
}
