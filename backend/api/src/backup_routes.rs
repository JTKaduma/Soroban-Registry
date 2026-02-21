use axum::{routing::get, routing::post, Router};

use crate::{backup_handlers, state::AppState};

pub fn backup_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/backups",
            post(backup_handlers::create_backup).get(backup_handlers::list_backups),
        )
        .route(
            "/api/contracts/:id/backups/restore",
            post(backup_handlers::restore_backup),
        )
        .route(
            "/api/contracts/:id/backups/:date/verify",
            post(backup_handlers::verify_backup),
        )
        .route(
            "/api/contracts/:id/backups/stats",
            get(backup_handlers::get_backup_stats),
        )
}
