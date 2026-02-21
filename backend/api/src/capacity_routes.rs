// api/src/capacity_routes.rs
//
// Register all capacity-planning routes.
// Add to your main router:
//
//   use crate::capacity_routes::capacity_router;
//   let app = Router::new()
//       .merge(capacity_router())
//       ...
//       .with_state(state);

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    capacity_handlers::{
        acknowledge_alert, get_capacity_plan, list_alerts,
        list_recommendations, list_snapshots, record_snapshot,
    },
    state::AppState,
};

pub fn capacity_router() -> Router<AppState> {
    Router::new()
        // Record a raw resource measurement (called by benchmark jobs / CI)
        .route(
            "/contracts/:id/resource-snapshots",
            post(record_snapshot).get(list_snapshots),
        )
        // Full capacity plan: forecasts + scenarios + alerts + recs + costs
        .route(
            "/contracts/:id/capacity-plan",
            get(get_capacity_plan),
        )
        // Active alerts
        .route(
            "/contracts/:id/capacity-alerts",
            get(list_alerts),
        )
        // Acknowledge a specific alert
        .route(
            "/contracts/:id/capacity-alerts/:alert_id/ack",
            patch(acknowledge_alert),
        )
        // Quick recommendations (no full plan recompute)
        .route(
            "/contracts/:id/capacity-recommendations",
            get(list_recommendations),
        )
}