use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{residency_handlers, state::AppState};

pub fn residency_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/residency/policies",
            post(residency_handlers::create_policy).get(residency_handlers::list_policies),
        )
        .route(
            "/api/residency/policies/:id",
            get(residency_handlers::get_policy).put(residency_handlers::update_policy),
        )
        .route("/api/residency/check", post(residency_handlers::check_residency))
        .route("/api/residency/logs", get(residency_handlers::get_audit_logs))
        .route("/api/residency/violations", get(residency_handlers::list_violations))
}
