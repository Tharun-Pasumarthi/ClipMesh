use crate::error::ApiError;
use crate::models::{
    CompletePairingRequest, InitiatePairingRequest, InitiatePairingResponse,
    PairingStatusResponse,
};
use crate::state::{AppState, PairingSessionRecord};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Initiate an authenticated QR pairing session
pub async fn initiate_pairing(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InitiatePairingRequest>,
) -> Result<Json<InitiatePairingResponse>, ApiError> {
    state.verify_active_device(&payload.initiator_device_id)?;

    let session_id = Uuid::new_v4();
    let eph_bytes = hex::decode(&payload.ephemeral_public_key)
        .map_err(|_| ApiError::BadRequest("Invalid ephemeral_public_key hex".to_string()))?;
    if eph_bytes.len() != 32 {
        return Err(ApiError::BadRequest("ephemeral_public_key must be 32 bytes".to_string()));
    }
    let mut eph_arr = [0u8; 32];
    eph_arr.copy_from_slice(&eph_bytes);

    // Generate random 16-character alphanumeric challenge
    let mut rng = rand::thread_rng();
    let part1: String = (0..4).map(|_| rng.gen_range(b'A'..=b'Z') as char).collect();
    let part2: String = (0..4).map(|_| rng.gen_range(b'A'..=b'Z') as char).collect();
    let pairing_code = format!("CM-{}-{}", part1, part2);

    let mut hasher = Sha256::new();
    hasher.update(pairing_code.as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    let user_id = state
        .users
        .read()
        .keys()
        .next()
        .copied()
        .unwrap_or_else(Uuid::new_v4);

    let expires_at = Utc::now() + Duration::seconds(120);

    let record = PairingSessionRecord {
        id: session_id,
        user_id,
        initiator_device_id: payload.initiator_device_id,
        responder_device_id: None,
        token_hash: token_hash.clone(),
        ephemeral_public_key: eph_arr,
        status: "PENDING".to_string(),
        confirmation_code: None,
        expires_at,
        created_at: Utc::now(),
    };

    let qr_payload = format!(
        "clipmesh://pair?v=1&sid={}&code={}&pk={}",
        session_id, pairing_code, payload.ephemeral_public_key
    );

    {
        let mut sessions = state.pairing_sessions.write();
        sessions.insert(session_id, record);
    }
    {
        let mut tokens = state.pairing_tokens.write();
        tokens.insert(token_hash, session_id);
    }

    Ok(Json(InitiatePairingResponse {
        session_id,
        pairing_code,
        qr_payload,
        expires_in_seconds: 120,
    }))
}

/// Responder scans QR and submits pairing request
pub async fn complete_pairing(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CompletePairingRequest>,
) -> Result<Json<PairingStatusResponse>, ApiError> {
    state.verify_active_device(&payload.responder_device_id)?;

    let mut hasher = Sha256::new();
    hasher.update(payload.token.trim().as_bytes());
    let token_hash = hex::encode(hasher.finalize());

    let session_id = {
        let tokens = state.pairing_tokens.read();
        tokens
            .get(&token_hash)
            .copied()
            .ok_or_else(|| ApiError::NotFound("Invalid or expired pairing code".to_string()))?
    };

    let mut sessions = state.pairing_sessions.write();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| ApiError::NotFound("Pairing session not found".to_string()))?;

    if Utc::now() > session.expires_at {
        session.status = "EXPIRED".to_string();
        return Err(ApiError::BadRequest("Pairing session expired".to_string()));
    }

    // Generate 6-digit confirmation code (SAS)
    let confirmation_code = format!("{:06}", rand::thread_rng().gen_range(100000..999999));

    session.responder_device_id = Some(payload.responder_device_id);
    session.confirmation_code = Some(confirmation_code);
    session.status = "AWAITING_APPROVAL".to_string();

    Ok(Json(session.to_response()))
}

/// Initiator approves the pairing
pub async fn approve_pairing(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<PairingStatusResponse>, ApiError> {
    let mut sessions = state.pairing_sessions.write();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    if session.status != "AWAITING_APPROVAL" {
        return Err(ApiError::BadRequest("Session is not awaiting approval".to_string()));
    }

    session.status = "COMPLETED".to_string();
    Ok(Json(session.to_response()))
}

/// Reject pairing
pub async fn reject_pairing(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<PairingStatusResponse>, ApiError> {
    let mut sessions = state.pairing_sessions.write();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;

    session.status = "REJECTED".to_string();
    Ok(Json(session.to_response()))
}

/// Get pairing session status
pub async fn get_pairing_status(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<PairingStatusResponse>, ApiError> {
    let sessions = state.pairing_sessions.read();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| ApiError::NotFound("Session not found".to_string()))?;
    Ok(Json(session.to_response()))
}

