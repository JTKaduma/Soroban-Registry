use axum::{
    routing::{get, post},
    Router,
};

use crate::{signing_handlers, state::AppState};

pub fn signing_routes() -> Router<AppState> {
    Router::new()
        .route("/api/signatures", post(signing_handlers::sign_package))
        .route(
            "/api/signatures/verify",
            post(signing_handlers::verify_signature),
        )
        .route(
            "/api/signatures/lookup",
            get(signing_handlers::lookup_signatures),
        )
        .route(
            "/api/signatures/:signature_id/revoke",
            post(signing_handlers::revoke_signature),
        )
        .route(
            "/api/signatures/custody/:contract_id",
            get(signing_handlers::get_chain_of_custody),
        )
        .route(
            "/api/signatures/transparency",
            get(signing_handlers::get_transparency_log),
        )
}
