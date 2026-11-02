pub mod error;
pub mod models;
pub mod routes {
    pub mod auth;
    pub mod devices;
    pub mod health;
    pub mod pairing;
    pub mod relay;
    pub mod transfers;
}
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};
use state::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/ready", get(routes::health::readiness_check))
        .route("/metrics", get(routes::health::prometheus_metrics))
        // Auth API
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/auth/logout", post(routes::auth::logout))
        // Devices API
        .route("/api/v1/devices/register", post(routes::devices::register_device))
        .route("/api/v1/devices", get(routes::devices::list_devices))
        .route(
            "/api/v1/devices/{id}",
            get(routes::devices::get_device).delete(routes::devices::revoke_device),
        )
        // Pairing API
        .route("/api/v1/pairing/initiate", post(routes::pairing::initiate_pairing))
        .route("/api/v1/pairing/complete", post(routes::pairing::complete_pairing))
        .route("/api/v1/pairing/{id}", get(routes::pairing::get_pairing_status))
        .route("/api/v1/pairing/{id}/approve", post(routes::pairing::approve_pairing))
        .route("/api/v1/pairing/{id}/reject", post(routes::pairing::reject_pairing))
        // Relay API & WebSocket & WebRTC P2P Signaling
        .route("/api/v1/relay/events", post(routes::relay::post_relay_event))
        .route("/api/v1/relay/pending", get(routes::relay::get_pending_events))
        .route("/api/v1/relay/events/{id}/ack", post(routes::relay::ack_event))
        .route("/api/v1/relay/signal", post(routes::relay::post_p2p_signal))
        .route("/api/v1/relay/signals", get(routes::relay::get_pending_signals))
        .route("/api/v1/relay/ws", get(routes::relay::ws_relay_handler))
        // File Transfers API
        .route("/api/v1/transfers/initiate", post(routes::transfers::initiate_transfer))
        .route("/api/v1/transfers/{id}", get(routes::transfers::get_transfer))
        .route("/api/v1/transfers/{id}/chunks", post(routes::transfers::upload_chunk))
        .route("/api/v1/transfers/{id}/complete", post(routes::transfers::complete_transfer))
        .route("/api/v1/transfers/{id}/cancel", post(routes::transfers::cancel_transfer))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

