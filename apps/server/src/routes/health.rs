use crate::state::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "clipmesh-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn readiness_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let active_devices = state.devices.read().len();
    let ws_connections = state.ws_subscribers.read().len();

    Json(json!({
        "status": "ready",
        "database": "connected",
        "redis": "connected",
        "active_devices": active_devices,
        "active_ws_connections": ws_connections,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> Response {
    let devices_count = state.devices.read().len();
    let events_count = state.relay_events.read().len();
    let ws_clients_count = state.ws_subscribers.read().len();
    let transfers_count = state.transfers.read().len();

    let body = format!(
        "# HELP clipmesh_registered_devices_total Total number of registered devices\n\
         # TYPE clipmesh_registered_devices_total gauge\n\
         clipmesh_registered_devices_total {}\n\
         # HELP clipmesh_relay_events_total Total number of relay events processed\n\
         # TYPE clipmesh_relay_events_total counter\n\
         clipmesh_relay_events_total {}\n\
         # HELP clipmesh_connected_ws_clients Currently connected WebSocket clients\n\
         # TYPE clipmesh_connected_ws_clients gauge\n\
         clipmesh_connected_ws_clients {}\n\
         # HELP clipmesh_active_transfers Currently active or staged file transfers\n\
         # TYPE clipmesh_active_transfers gauge\n\
         clipmesh_active_transfers {}\n",
        devices_count, events_count, ws_clients_count, transfers_count
    );

    axum::response::Response::builder()
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(body.into())
        .unwrap()
}
