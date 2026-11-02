use chrono::{DateTime, Utc};
use clipmesh_protocol::AckStage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Auth DTOs ---

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

// --- Device DTOs ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofOfPossessionDto {
    pub challenge: String, // hex encoded
    pub signature: String, // hex encoded 64-byte signature
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub platform: String,
    pub signing_public_key: String,   // hex encoded 32 bytes
    pub agreement_public_key: String, // hex encoded 32 bytes
    pub proof_of_possession: ProofOfPossessionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub platform: String,
    pub signing_public_key: String,
    pub agreement_public_key: String,
    pub key_fingerprint: String,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

// --- Pairing DTOs ---

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiatePairingRequest {
    pub initiator_device_id: Uuid,
    pub ephemeral_public_key: String, // hex encoded 32 bytes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiatePairingResponse {
    pub session_id: Uuid,
    pub pairing_code: String,
    pub qr_payload: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletePairingRequest {
    pub token: String,
    pub responder_device_id: Uuid,
    pub responder_ephemeral_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingStatusResponse {
    pub session_id: Uuid,
    pub status: String,
    pub initiator_device_id: Uuid,
    pub responder_device_id: Option<Uuid>,
    pub confirmation_code: Option<String>,
    pub expires_at: DateTime<Utc>,
}

// --- Relay DTOs ---

#[derive(Debug, Serialize, Deserialize)]
pub struct PostRelayEventRequest {
    pub sender_device_id: Option<Uuid>,
    pub recipient_device_id: Uuid,
    pub encrypted_payload: String, // base64
    pub nonce: String,             // base64 (12 bytes)
    pub sequence_number: u64,
    pub epoch_id: u64,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayEventResponse {
    pub id: Uuid,
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub encrypted_payload: String,
    pub nonce: String,
    pub sequence_number: u64,
    pub epoch_id: u64,
    pub protocol_version: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AckEventRequest {
    pub stage: AckStage,
    pub error_detail: Option<String>,
}

// --- Transfer DTOs ---

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateTransferRequest {
    pub recipient_device_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub total_chunks: u32,
    pub whole_file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    pub id: Uuid,
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub total_chunks: u32,
    pub received_chunks: u32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadChunkRequest {
    pub chunk_index: u32,
    pub nonce: String,
    pub ciphertext: String, // base64
}
