use crate::error::ApiError;
use crate::models::{InitiateTransferRequest, TransferResponse, UploadChunkRequest};
use crate::state::{AppState, StagedTransferRecord};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Initiate a new chunked file transfer
pub async fn initiate_transfer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InitiateTransferRequest>,
) -> Result<Json<TransferResponse>, ApiError> {
    state.verify_active_device(&payload.recipient_device_id)?;

    let sender_device_id = state
        .devices
        .read()
        .keys()
        .next()
        .copied()
        .unwrap_or_else(Uuid::new_v4);

    let user_id = state
        .users
        .read()
        .keys()
        .next()
        .copied()
        .unwrap_or_else(Uuid::new_v4);

    let transfer_id = Uuid::new_v4();

    let record = StagedTransferRecord {
        id: transfer_id,
        user_id,
        sender_device_id,
        recipient_device_id: payload.recipient_device_id,
        file_name: payload.file_name,
        file_size: payload.file_size,
        total_chunks: payload.total_chunks,
        received_chunks: HashSet::new(),
        whole_file_hash: payload.whole_file_hash,
        status: "PENDING".to_string(),
        created_at: Utc::now(),
        chunks_data: HashMap::new(),
    };

    let response = record.to_response();
    {
        let mut transfers = state.transfers.write();
        transfers.insert(transfer_id, record);
    }

    Ok(Json(response))
}

/// Upload an encrypted file chunk
pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<Uuid>,
    Json(payload): Json<UploadChunkRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut transfers = state.transfers.write();
    let transfer = transfers
        .get_mut(&transfer_id)
        .ok_or_else(|| ApiError::NotFound("Transfer not found".to_string()))?;

    if transfer.status == "CANCELLED" || transfer.status == "COMPLETED" {
        return Err(ApiError::BadRequest("Transfer is no longer active".to_string()));
    }

    transfer.received_chunks.insert(payload.chunk_index);
    transfer.chunks_data.insert(payload.chunk_index, (payload.nonce, payload.ciphertext));
    transfer.status = "IN_PROGRESS".to_string();

    Ok(Json(serde_json::json!({
        "status": "chunk_accepted",
        "chunk_index": payload.chunk_index,
        "received_chunks": transfer.received_chunks.len(),
        "total_chunks": transfer.total_chunks
    })))
}

/// Mark transfer as completed
pub async fn complete_transfer(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<Uuid>,
) -> Result<Json<TransferResponse>, ApiError> {
    let mut transfers = state.transfers.write();
    let transfer = transfers
        .get_mut(&transfer_id)
        .ok_or_else(|| ApiError::NotFound("Transfer not found".to_string()))?;

    if transfer.received_chunks.len() != (transfer.total_chunks as usize) {
        return Err(ApiError::BadRequest(format!(
            "Transfer incomplete: received {}/{} chunks",
            transfer.received_chunks.len(),
            transfer.total_chunks
        )));
    }

    transfer.status = "COMPLETED".to_string();
    Ok(Json(transfer.to_response()))
}

/// Cancel a transfer
pub async fn cancel_transfer(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<Uuid>,
) -> Result<Json<TransferResponse>, ApiError> {
    let mut transfers = state.transfers.write();
    let transfer = transfers
        .get_mut(&transfer_id)
        .ok_or_else(|| ApiError::NotFound("Transfer not found".to_string()))?;

    transfer.status = "CANCELLED".to_string();
    transfer.chunks_data.clear();

    Ok(Json(transfer.to_response()))
}

/// Get transfer status
pub async fn get_transfer(
    State(state): State<Arc<AppState>>,
    Path(transfer_id): Path<Uuid>,
) -> Result<Json<TransferResponse>, ApiError> {
    let transfers = state.transfers.read();
    let transfer = transfers
        .get(&transfer_id)
        .ok_or_else(|| ApiError::NotFound("Transfer not found".to_string()))?;

    Ok(Json(transfer.to_response()))
}

