use axum::{
    routing::{get, post},
    Router,
};

use crate::{event_handlers, state::AppState};

pub fn event_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/events",
            get(event_handlers::get_contract_events),
        )
        .route(
            "/api/contracts/:id/events/stats",
            get(event_handlers::get_event_stats),
        )
        .route(
            "/api/contracts/:id/events/export",
            get(event_handlers::export_events_csv),
        )
        .route("/api/events", post(event_handlers::index_event))
        .route(
            "/api/events/batch",
            post(event_handlers::index_events_batch),
        )
}
