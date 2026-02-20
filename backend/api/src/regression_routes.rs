// api/src/regression_routes.rs
// Route definitions for regression testing endpoints

use axum::{
    routing::{get, post},
    Router,
};

use crate::{regression_handlers, state::AppState};

pub fn regression_routes() -> Router<AppState> {
    Router::new()
        // Baseline management
        .route(
            "/api/contracts/:id/regression/baseline",
            post(regression_handlers::establish_baseline),
        )
        .route(
            "/api/contracts/:id/regression/baselines",
            get(regression_handlers::get_baselines),
        )
        // Test execution
        .route(
            "/api/contracts/:id/regression/test",
            post(regression_handlers::run_regression_test),
        )
        .route(
            "/api/contracts/:id/regression/suite",
            post(regression_handlers::run_test_suite),
        )
        .route(
            "/api/contracts/:id/regression/runs",
            get(regression_handlers::get_test_runs),
        )
        // Test suites
        .route(
            "/api/contracts/:id/regression/suites",
            get(regression_handlers::get_test_suites)
                .post(regression_handlers::create_test_suite),
        )
        // Alerts
        .route(
            "/api/contracts/:id/regression/alerts",
            get(regression_handlers::get_alerts),
        )
        .route(
            "/api/contracts/:id/regression/alerts/:alert_id/acknowledge",
            post(regression_handlers::acknowledge_alert),
        )
        .route(
            "/api/contracts/:id/regression/alerts/:alert_id/resolve",
            post(regression_handlers::resolve_alert),
        )
        // Statistics
        .route(
            "/api/contracts/:id/regression/statistics",
            get(regression_handlers::get_statistics),
        )
}
