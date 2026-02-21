use axum::{
    routing::{get, post},
    Router,
};
use crate::{formal_verification_handlers, state::AppState};

pub fn formal_verification_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/contracts/:id/formal-verification",
            get(formal_verification_handlers::get_formal_verification_history)
                .post(formal_verification_handlers::run_formal_verification),
        )
}
