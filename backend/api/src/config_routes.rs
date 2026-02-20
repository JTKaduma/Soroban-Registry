use axum::{
    routing::{get, post},
    Router,
};

use crate::{config_handlers, state::AppState};

pub fn config_routes() -> Router<AppState> {
    Router::new()
        .route("/api/contracts/:id/config", get(config_handlers::get_contract_config).post(config_handlers::create_contract_config))
        .route("/api/contracts/:id/config/history", get(config_handlers::get_config_history))
        .route("/api/contracts/:id/config/rollback", post(config_handlers::rollback_config))
}
