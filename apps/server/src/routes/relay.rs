use crate::error::ApiError;
use crate::models::{AckEventRequest, PostRelayEventRequest, RelayEventResponse};
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use clipmesh_protocol::{AckStage, EnvelopeStatus, RelayEnvelope};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Uuid,
}

/// Enqueue an encrypted envelope for routing
pub async fn post_relay_event(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PostRelayEventRequest>,
) -> Result<Json<RelayEventResponse>, ApiError> {
    // Decode base64 payload & nonce
    let encrypted_payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload.encrypted_payload,
    )
    .map_err(|_| ApiError::BadRequest("Invalid base64 encrypted_payload".to_string()))?;

    let nonce_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload.nonce,
    )
    .map_err(|_| ApiError::BadRequest("Invalid base64 nonce".to_string()))?;

    if nonce_bytes.len() != 12 {
        return Err(ApiError::BadRequest("Nonce must be 12 bytes".to_string()));
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);

    let sender_device_id = payload.sender_device_id.unwrap_or_else(|| {
        state
            .devices
            .read()
            .keys()
            .next()
            .copied()
            .unwrap_or_else(Uuid::new_v4)
    });

    let envelope_id = Uuid::new_v4();
    let envelope = RelayEnvelope {
        id: envelope_id,
        protocol_version: payload.protocol_version,
        sender_device_id,
        recipient_device_id: payload.recipient_device_id,
        sequence_number: payload.sequence_number,
        epoch_id: payload.epoch_id,
        nonce,
        encrypted_payload,
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(24),
        status: EnvelopeStatus::Pending,
    };

    state.route_envelope(envelope.clone())?;

    Ok(Json(RelayEventResponse {
        id: envelope_id,
        sender_device_id: envelope.sender_device_id,
        recipient_device_id: envelope.recipient_device_id,
        encrypted_payload: payload.encrypted_payload,
        nonce: payload.nonce,
        sequence_number: envelope.sequence_number,
        epoch_id: envelope.epoch_id,
        protocol_version: envelope.protocol_version,
        created_at: envelope.created_at,
    }))
}

/// Retrieve pending envelopes for a device (HTTP polling fallback)
pub async fn get_pending_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceQuery>,
) -> Result<Json<Vec<RelayEventResponse>>, ApiError> {
    state.verify_active_device(&query.device_id)?;

    let envelope_ids = {
        let queues = state.pending_queues.read();
        queues.get(&query.device_id).cloned().unwrap_or_default()
    };

    let events_map = state.relay_events.read();
    let mut results = Vec::new();

    for id in envelope_ids {
        if let Some(env) = events_map.get(&id) {
            results.push(RelayEventResponse {
                id: env.id,
                sender_device_id: env.sender_device_id,
                recipient_device_id: env.recipient_device_id,
                encrypted_payload: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &env.encrypted_payload,
                ),
                nonce: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &env.nonce,
                ),
                sequence_number: env.sequence_number,
                epoch_id: env.epoch_id,
                protocol_version: env.protocol_version,
                created_at: env.created_at,
            });
        }
    }

    Ok(Json(results))
}

/// Acknowledge an event
pub async fn ack_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<AckEventRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut events = state.relay_events.write();
    let event = events
        .get_mut(&event_id)
        .ok_or_else(|| ApiError::NotFound("Event not found".to_string()))?;

    match payload.stage {
        AckStage::RecipientApplied => {
            event.status = EnvelopeStatus::Acknowledged;
            // Remove from recipient queue
            let mut queues = state.pending_queues.write();
            if let Some(q) = queues.get_mut(&event.recipient_device_id) {
                q.retain(|id| *id != event_id);
            }
        }
        AckStage::RecipientReceived => {
            event.status = EnvelopeStatus::Delivered;
        }
        _ => {}
    }

    Ok(Json(serde_json::json!({
        "status": "ack_recorded",
        "stage": payload.stage,
        "event_id": event_id
    })))
}

/// Send a WebRTC P2P Direct Signaling Message (Offer/Answer/IceCandidate)
pub async fn post_p2p_signal(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<clipmesh_protocol::P2pSignalingEnvelope>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.route_signaling(payload)?;
    Ok(Json(serde_json::json!({ "status": "signal_routed" })))
}

/// Retrieve pending P2P signals for a device (HTTP polling fallback)
pub async fn get_pending_signals(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceQuery>,
) -> Result<Json<Vec<clipmesh_protocol::P2pSignalingEnvelope>>, ApiError> {
    state.verify_active_device(&query.device_id)?;

    let signals = {
        let mut queues = state.p2p_signaling_queues.write();
        queues.remove(&query.device_id).unwrap_or_default()
    };

    Ok(Json(signals))
}

/// WebSocket Relay Hub
pub async fn ws_relay_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<DeviceQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_socket(socket, query.device_id, state))
}

async fn handle_ws_socket(socket: WebSocket, device_id: Uuid, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Register active device connection
    {
        let mut subs = state.ws_subscribers.write();
        subs.insert(device_id, tx);
    }

    // Flush pending queue over WebSocket upon initial connect
    let pending_messages: Vec<String> = {
        let queues = state.pending_queues.read();
        let pending_ids = queues.get(&device_id).cloned().unwrap_or_default();
        let events = state.relay_events.read();
        let mut list = Vec::new();
        for id in pending_ids {
            if let Some(env) = events.get(&id) {
                if let Ok(json_str) = serde_json::to_string(env) {
                    list.push(json_str);
                }
            }
        }
        list
    };

    for json_str in pending_messages {
        let _ = ws_sender.send(Message::Text(json_str.into())).await;
    }

    // Spawn task to forward outgoing channel messages to client socket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming client messages (e.g. keepalive Ping/Pong or Acks)
    let _state = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_) => {}
                Message::Text(_t) => {
                    // Client can submit acks over WS
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
    };

    // Cleanup on disconnect
    {
        let mut subs = state.ws_subscribers.write();
        subs.remove(&device_id);
    }
}
