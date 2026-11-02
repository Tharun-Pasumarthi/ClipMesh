use crate::error::ApiError;
use crate::models::{DeviceResponse, PairingStatusResponse, TransferResponse};
use axum::extract::ws::Message;
use chrono::{DateTime, Utc};
use clipmesh_protocol::RelayEnvelope;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct DeviceRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub platform: String,
    pub signing_public_key: [u8; 32],
    pub agreement_public_key: [u8; 32],
    pub key_fingerprint: String,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

impl DeviceRecord {
    pub fn to_response(&self) -> DeviceResponse {
        DeviceResponse {
            id: self.id,
            user_id: self.user_id,
            name: self.name.clone(),
            platform: self.platform.clone(),
            signing_public_key: hex::encode(self.signing_public_key),
            agreement_public_key: hex::encode(self.agreement_public_key),
            key_fingerprint: self.key_fingerprint.clone(),
            is_revoked: self.is_revoked,
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
        }
    }
}

#[derive(Clone)]
pub struct PairingSessionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub initiator_device_id: Uuid,
    pub responder_device_id: Option<Uuid>,
    pub token_hash: String,
    pub ephemeral_public_key: [u8; 32],
    pub status: String,
    pub confirmation_code: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl PairingSessionRecord {
    pub fn to_response(&self) -> PairingStatusResponse {
        PairingStatusResponse {
            session_id: self.id,
            status: self.status.clone(),
            initiator_device_id: self.initiator_device_id,
            responder_device_id: self.responder_device_id,
            confirmation_code: self.confirmation_code.clone(),
            expires_at: self.expires_at,
        }
    }
}

#[derive(Clone)]
pub struct StagedTransferRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub total_chunks: u32,
    pub received_chunks: HashSet<u32>,
    pub whole_file_hash: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub chunks_data: HashMap<u32, (String, String)>, // chunk_index -> (nonce_b64, ciphertext_b64)
}

impl StagedTransferRecord {
    pub fn to_response(&self) -> TransferResponse {
        TransferResponse {
            id: self.id,
            sender_device_id: self.sender_device_id,
            recipient_device_id: self.recipient_device_id,
            file_name: self.file_name.clone(),
            file_size: self.file_size,
            total_chunks: self.total_chunks,
            received_chunks: self.received_chunks.len() as u32,
            status: self.status.clone(),
            created_at: self.created_at,
        }
    }
}

pub struct AppState {
    pub users: RwLock<HashMap<Uuid, UserRecord>>,
    pub refresh_sessions: RwLock<HashMap<String, (Uuid, DateTime<Utc>)>>, // token_hash -> (user_id, expires_at)
    pub devices: RwLock<HashMap<Uuid, DeviceRecord>>,
    pub pairing_sessions: RwLock<HashMap<Uuid, PairingSessionRecord>>,
    pub pairing_tokens: RwLock<HashMap<String, Uuid>>, // token_hash -> session_id
    pub relay_events: RwLock<HashMap<Uuid, RelayEnvelope>>,
    pub pending_queues: RwLock<HashMap<Uuid, Vec<Uuid>>>, // recipient_device_id -> Vec<envelope_id>
    pub transfers: RwLock<HashMap<Uuid, StagedTransferRecord>>,
    pub p2p_signaling_queues: RwLock<HashMap<Uuid, Vec<clipmesh_protocol::P2pSignalingEnvelope>>>,
    pub ws_subscribers: RwLock<HashMap<Uuid, UnboundedSender<Message>>>, // device_id -> ws sender
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(jwt_secret: String) -> Arc<Self> {
        Arc::new(Self {
            users: RwLock::new(HashMap::new()),
            refresh_sessions: RwLock::new(HashMap::new()),
            devices: RwLock::new(HashMap::new()),
            pairing_sessions: RwLock::new(HashMap::new()),
            pairing_tokens: RwLock::new(HashMap::new()),
            relay_events: RwLock::new(HashMap::new()),
            pending_queues: RwLock::new(HashMap::new()),
            transfers: RwLock::new(HashMap::new()),
            p2p_signaling_queues: RwLock::new(HashMap::new()),
            ws_subscribers: RwLock::new(HashMap::new()),
            jwt_secret,
        })
    }

    /// Verify device is active and not revoked
    pub fn verify_active_device(&self, device_id: &Uuid) -> Result<DeviceRecord, ApiError> {
        let devs = self.devices.read();
        let dev = devs
            .get(device_id)
            .ok_or_else(|| ApiError::NotFound(format!("Device {} not found", device_id)))?;
        if dev.is_revoked {
            return Err(ApiError::Forbidden("Device access has been revoked".to_string()));
        }
        Ok(dev.clone())
    }

    /// Enqueue an encrypted envelope for delivery
    pub fn route_envelope(&self, envelope: RelayEnvelope) -> Result<(), ApiError> {
        // Enforce authorization: both devices must exist and not be revoked
        let sender = self.verify_active_device(&envelope.sender_device_id)?;
        let recipient = self.verify_active_device(&envelope.recipient_device_id)?;

        // Enforce cross-account boundary (IDOR mitigation)
        if sender.user_id != recipient.user_id {
            return Err(ApiError::Forbidden(
                "Cross-account event delivery is prohibited".to_string(),
            ));
        }

        let envelope_id = envelope.id;
        let recipient_id = envelope.recipient_device_id;

        // Save in global storage
        {
            let mut events = self.relay_events.write();
            events.insert(envelope_id, envelope.clone());
        }

        // Add to recipient's pending queue
        {
            let mut queues = self.pending_queues.write();
            queues.entry(recipient_id).or_default().push(envelope_id);
        }

        // If recipient is actively connected via WebSocket, push event immediately
        {
            let subs = self.ws_subscribers.read();
            if let Some(sender) = subs.get(&recipient_id) {
                if let Ok(json_str) = serde_json::to_string(&envelope) {
                    let _ = sender.send(Message::Text(json_str.into()));
                }
            }
        }

        Ok(())
    }

    /// Route a WebRTC P2P direct connection signaling message
    pub fn route_signaling(
        &self,
        envelope: clipmesh_protocol::P2pSignalingEnvelope,
    ) -> Result<(), ApiError> {
        let sender = self.verify_active_device(&envelope.sender_device_id)?;
        let recipient = self.verify_active_device(&envelope.recipient_device_id)?;

        if sender.user_id != recipient.user_id {
            return Err(ApiError::Forbidden(
                "Cross-account signaling is prohibited".to_string(),
            ));
        }

        let recipient_id = envelope.recipient_device_id;

        // Attempt push over active WebSocket
        let delivered = {
            let subs = self.ws_subscribers.read();
            if let Some(ws_tx) = subs.get(&recipient_id) {
                if let Ok(json_str) = serde_json::to_string(&serde_json::json!({
                    "type": "p2p_signal",
                    "envelope": envelope
                })) {
                    ws_tx.send(Message::Text(json_str.into())).is_ok()
                } else {
                    false
                }
            } else {
                false
            }
        };

        if !delivered {
            let mut queues = self.p2p_signaling_queues.write();
            queues.entry(recipient_id).or_default().push(envelope);
        }

        Ok(())
    }

    /// Revoke a device
    pub fn revoke_device(&self, device_id: &Uuid, user_id: &Uuid) -> Result<(), ApiError> {
        let mut devs = self.devices.write();
        let dev = devs
            .get_mut(device_id)
            .ok_or_else(|| ApiError::NotFound("Device not found".to_string()))?;

        if dev.user_id != *user_id {
            return Err(ApiError::Forbidden("Unauthorized device access".to_string()));
        }

        dev.is_revoked = true;

        // Disconnect active WS session
        let mut subs = self.ws_subscribers.write();
        subs.remove(device_id);

        Ok(())
    }
}
