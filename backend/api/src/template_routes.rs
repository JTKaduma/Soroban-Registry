use axum::{routing::{get, post}, Router};
use crate::{state::AppState, template_handlers};

pub fn template_routes() -> Router<AppState> {
    Router::new()
        .route("/api/templates", get(template_handlers::list_templates))
        .route("/api/templates/:slug", get(template_handlers::get_template))
        .route("/api/templates/:slug/clone", post(template_handlers::clone_template))
}
