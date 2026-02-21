// api/src/feature_flag_routes.rs
//
// Mount in main.rs:
//   mod feature_flag_handlers;
//   mod feature_flag_routes;
//
//   let app = Router::new()
//       .merge(feature_flag_routes::feature_flag_router())
//       .with_state(state);

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    feature_flag_handlers::{
        check_enabled, configure_ab_test, create_flag, disable_flag,
        enable_flag, get_ab_test, get_analytics, get_flag, list_flags,
        sunset_flag, sweep_expired, update_rollout,
    },
    state::AppState,
};

pub fn feature_flag_router() -> Router<AppState> {
    Router::new()
        // Collection: create / list
        .route(
            "/contracts/:id/feature-flags",
            post(create_flag).get(list_flags),
        )
        // Batch: sweep all expired flags for a contract
        .route(
            "/contracts/:id/feature-flags/sweep",
            post(sweep_expired),
        )
        // Single flag: get
        .route(
            "/contracts/:id/feature-flags/:name",
            get(get_flag),
        )
        // Lifecycle transitions
        .route(
            "/contracts/:id/feature-flags/:name/enable",
            patch(enable_flag),
        )
        .route(
            "/contracts/:id/feature-flags/:name/disable",
            patch(disable_flag),
        )
        .route(
            "/contracts/:id/feature-flags/:name/sunset",
            patch(sunset_flag),
        )
        // Rollout percentage adjustment
        .route(
            "/contracts/:id/feature-flags/:name/rollout",
            patch(update_rollout),
        )
        // Analytics
        .route(
            "/contracts/:id/feature-flags/:name/analytics",
            get(get_analytics),
        )
        // A/B test config
        .route(
            "/contracts/:id/feature-flags/:name/ab-test",
            post(configure_ab_test).get(get_ab_test),
        )
        // Per-user flag check (used by SDK / clients before calling contract)
        .route(
            "/contracts/:id/feature-flags/:name/check",
            get(check_enabled),
        )
}