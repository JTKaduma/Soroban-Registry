use axum::{routing::get, routing::post, Router};

use crate::{maintenance_handlers, state::AppState};

pub fn maintenance_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/maintenance",
            post(maintenance_handlers::start_maintenance)
                .delete(maintenance_handlers::end_maintenance)
                .get(maintenance_handlers::get_maintenance_status),
        )
        .route(
            "/api/contracts/:id/maintenance/history",
            get(maintenance_handlers::get_maintenance_history),
        )
}
