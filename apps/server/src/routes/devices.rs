use crate::error::ApiError;
use crate::models::{DeviceResponse, RegisterDeviceRequest};
use crate::state::{AppState, DeviceRecord};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use clipmesh_crypto::SigningPublicKey;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Register a new device with proof of possession
pub async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<DeviceResponse>, ApiError> {
    let sign_pk_bytes = hex::decode(&payload.signing_public_key)
        .map_err(|_| ApiError::BadRequest("Invalid signing_public_key hex".to_string()))?;
    if sign_pk_bytes.len() != 32 {
        return Err(ApiError::BadRequest("signing_public_key must be 32 bytes".to_string()));
    }
    let mut sign_pk_arr = [0u8; 32];
    sign_pk_arr.copy_from_slice(&sign_pk_bytes);

    let agree_pk_bytes = hex::decode(&payload.agreement_public_key)
        .map_err(|_| ApiError::BadRequest("Invalid agreement_public_key hex".to_string()))?;
    if agree_pk_bytes.len() != 32 {
        return Err(ApiError::BadRequest("agreement_public_key must be 32 bytes".to_string()));
    }
    let mut agree_pk_arr = [0u8; 32];
    agree_pk_arr.copy_from_slice(&agree_pk_bytes);

    // Verify proof of possession
    let challenge_bytes = hex::decode(&payload.proof_of_possession.challenge)
        .map_err(|_| ApiError::BadRequest("Invalid challenge hex".to_string()))?;
    let sig_bytes = hex::decode(&payload.proof_of_possession.signature)
        .map_err(|_| ApiError::BadRequest("Invalid signature hex".to_string()))?;
    if sig_bytes.len() != 64 {
        return Err(ApiError::BadRequest("signature must be 64 bytes".to_string()));
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);

    let pubkey = SigningPublicKey::from_bytes(&sign_pk_arr)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 public key".to_string()))?;
    pubkey
        .verify(&challenge_bytes, &sig_arr)
        .map_err(|_| ApiError::Unauthorized("Proof of possession failed".to_string()))?;

    // Compute key fingerprint
    let mut hasher = Sha256::new();
    hasher.update(&sign_pk_arr);
    let key_fingerprint = hex::encode(hasher.finalize());

    let device_id = Uuid::new_v4();
    // Default account user ID (or extract from bearer JWT)
    let user_id = state
        .users
        .read()
        .keys()
        .next()
        .copied()
        .unwrap_or_else(Uuid::new_v4);

    let record = DeviceRecord {
        id: device_id,
        user_id,
        name: payload.name,
        platform: payload.platform,
        signing_public_key: sign_pk_arr,
        agreement_public_key: agree_pk_arr,
        key_fingerprint,
        is_revoked: false,
        created_at: Utc::now(),
        last_seen_at: Utc::now(),
    };

    let response = record.to_response();
    {
        let mut devices = state.devices.write();
        devices.insert(device_id, record);
    }

    Ok(Json(response))
}

/// List all devices
pub async fn list_devices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeviceResponse>>, ApiError> {
    let devices = state.devices.read();
    let res: Vec<DeviceResponse> = devices.values().map(|d| d.to_response()).collect();
    Ok(Json(res))
}

/// Get a specific device
pub async fn get_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, ApiError> {
    let devices = state.devices.read();
    let dev = devices
        .get(&device_id)
        .ok_or_else(|| ApiError::NotFound("Device not found".to_string()))?;
    Ok(Json(dev.to_response()))
}

/// Revoke a device
pub async fn revoke_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = {
        let devs = state.devices.read();
        devs.get(&device_id)
            .map(|d| d.user_id)
            .ok_or_else(|| ApiError::NotFound("Device not found".to_string()))?
    };

    state.revoke_device(&device_id, &user_id)?;
    Ok(Json(serde_json::json!({
        "status": "revoked",
        "device_id": device_id
    })))
}

