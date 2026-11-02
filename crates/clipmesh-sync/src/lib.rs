//! ClipMesh Synchronization Engine
//!
//! Event-driven synchronization coordinator with monotonic sequence tracking,
//! deduplication, multi-stage acknowledgements, offline queues, and deterministic ordering.

use chrono::Utc;
use clipmesh_crypto::{
    construct_nonce, decrypt_aead, encrypt_aead, AeadKey, CryptoError,
};
use clipmesh_protocol::{
    AckEvent, AckStage, ClipboardPayload, EnvelopeStatus, InnerMessage, RelayEnvelope, PROTOCOL_VERSION,
};
use clipmesh_storage::LocalStore;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] clipmesh_protocol::ProtocolError),

    #[error("Storage error: {0}")]
    Storage(#[from] clipmesh_storage::StorageError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Duplicate event ignored: {0}")]
    DuplicateEvent(Uuid),

    #[error("Recipient device not in trust store: {0}")]
    UntrustedRecipient(Uuid),

    #[error("Invalid sequence number: expected > {expected}, got {received}")]
    InvalidSequence { expected: u64, received: u64 },
}

/// Sync Coordinator managing peer sessions and sequence numbers
pub struct SyncEngine {
    device_id: Uuid,
    device_prefix: u32,
    sequence_counter: AtomicU64,
    seen_events: parking_lot::RwLock<HashSet<Uuid>>,
    peer_sessions: parking_lot::RwLock<HashMap<Uuid, PeerSessionState>>,
    store: Arc<LocalStore>,
}

#[derive(Clone)]
pub struct PeerSessionState {
    pub peer_device_id: Uuid,
    pub epoch_id: u64,
    pub session_key: [u8; 32],
    pub last_received_sequence: u64,
}

impl SyncEngine {
    pub fn new(device_id: Uuid, store: Arc<LocalStore>) -> Self {
        // Derive 32-bit device prefix from device UUID bytes
        let bytes = device_id.as_bytes();
        let prefix = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        Self {
            device_id,
            device_prefix: prefix,
            sequence_counter: AtomicU64::new(1),
            seen_events: parking_lot::RwLock::new(HashSet::new()),
            peer_sessions: parking_lot::RwLock::new(HashMap::new()),
            store,
        }
    }

    /// Register or update an active pairwise encrypted session with a trusted peer
    pub fn add_peer_session(&self, peer_id: Uuid, epoch_id: u64, session_key: [u8; 32]) {
        let mut map = self.peer_sessions.write();
        map.insert(
            peer_id,
            PeerSessionState {
                peer_device_id: peer_id,
                epoch_id,
                session_key,
                last_received_sequence: 0,
            },
        );
    }

    /// Prepare an outbound encrypted clipboard event for a trusted peer
    pub fn create_clipboard_envelope(
        &self,
        recipient_id: Uuid,
        payload: &ClipboardPayload,
    ) -> Result<RelayEnvelope, SyncError> {
        let peer_state = {
            let map = self.peer_sessions.read();
            map.get(&recipient_id)
                .cloned()
                .ok_or(SyncError::UntrustedRecipient(recipient_id))?
        };

        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let envelope_id = Uuid::new_v4();
        let nonce = construct_nonce(self.device_prefix, seq);

        let inner_msg = InnerMessage::ClipboardSync(payload.clone());
        let plaintext_json = serde_json::to_vec(&inner_msg)?;

        let ad = RelayEnvelope::build_associated_data(
            PROTOCOL_VERSION,
            &self.device_id,
            &recipient_id,
            peer_state.epoch_id,
            seq,
        );

        let aead_key = AeadKey::from_bytes(peer_state.session_key);
        let ciphertext = encrypt_aead(&aead_key, &nonce, &plaintext_json, &ad)?;

        let envelope = RelayEnvelope {
            id: envelope_id,
            protocol_version: PROTOCOL_VERSION,
            sender_device_id: self.device_id,
            recipient_device_id: recipient_id,
            sequence_number: seq,
            epoch_id: peer_state.epoch_id,
            nonce,
            encrypted_payload: ciphertext,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            status: EnvelopeStatus::Pending,
        };

        // Persist locally in outbound queue for offline resilience
        self.store.enqueue_outbound(&envelope)?;

        Ok(envelope)
    }

    /// Process and decrypt an incoming envelope from a peer
    pub fn process_inbound_envelope(
        &self,
        envelope: &RelayEnvelope,
    ) -> Result<(InnerMessage, AckEvent), SyncError> {
        envelope.validate()?;

        // 1. Deduplication check
        {
            let mut seen = self.seen_events.write();
            if seen.contains(&envelope.id) {
                return Err(SyncError::DuplicateEvent(envelope.id));
            }
            seen.insert(envelope.id);
        }

        // 2. Fetch peer session
        let peer_state = {
            let map = self.peer_sessions.read();
            map.get(&envelope.sender_device_id)
                .cloned()
                .ok_or(SyncError::UntrustedRecipient(envelope.sender_device_id))?
        };

        // 3. Verify sequence number monotonic increment
        if envelope.sequence_number <= peer_state.last_received_sequence {
            return Err(SyncError::InvalidSequence {
                expected: peer_state.last_received_sequence,
                received: envelope.sequence_number,
            });
        }

        // 4. Authenticate & Decrypt with AEAD Associated Data
        let ad = envelope.associated_data();
        let aead_key = AeadKey::from_bytes(peer_state.session_key);
        let plaintext_bytes = decrypt_aead(
            &aead_key,
            &envelope.nonce,
            &envelope.encrypted_payload,
            &ad,
        )?;

        // 5. Parse decrypted inner message
        let inner_msg: InnerMessage = serde_json::from_slice(&plaintext_bytes)?;

        // 6. If clipboard item, save to local storage
        if let InnerMessage::ClipboardSync(ref clip) = inner_msg {
            self.store.insert_clip(envelope.id, clip)?;
        }

        // Update sequence state
        {
            let mut map = self.peer_sessions.write();
            if let Some(state) = map.get_mut(&envelope.sender_device_id) {
                state.last_received_sequence = envelope.sequence_number;
            }
        }

        // Generate recipient applied acknowledgement
        let ack = AckEvent {
            envelope_id: envelope.id,
            stage: AckStage::RecipientApplied,
            device_id: self.device_id,
            timestamp: Utc::now(),
            error_detail: None,
        };

        Ok((inner_msg, ack))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipmesh_protocol::ClipKind;

    #[test]
    fn test_sync_encrypt_decrypt_roundtrip() {
        let store_a = Arc::new(LocalStore::in_memory().unwrap());
        let store_b = Arc::new(LocalStore::in_memory().unwrap());

        let dev_a = Uuid::new_v4();
        let dev_b = Uuid::new_v4();

        let engine_a = SyncEngine::new(dev_a, store_a);
        let engine_b = SyncEngine::new(dev_b, store_b.clone());

        let shared_session_key = [0x77; 32];
        engine_a.add_peer_session(dev_b, 1, shared_session_key);
        engine_b.add_peer_session(dev_a, 1, shared_session_key);

        let clip = ClipboardPayload {
            mime_type: "text/plain".into(),
            kind: ClipKind::Text,
            title: "Meeting notes".into(),
            text: "Sync protocol review".into(),
            pinned: false,
            origin_device_name: "Windows PC".into(),
            timestamp_ms: 1000,
        };

        // Sender encrypts
        let envelope = engine_a.create_clipboard_envelope(dev_b, &clip).unwrap();

        // Recipient decrypts and applies
        let (received_msg, ack) = engine_b.process_inbound_envelope(&envelope).unwrap();

        match received_msg {
            InnerMessage::ClipboardSync(c) => {
                assert_eq!(c.text, "Sync protocol review");
            }
            _ => panic!("Expected ClipboardSync"),
        }

        assert_eq!(ack.stage, AckStage::RecipientApplied);

        // Verify stored in recipient's local store
        let clips = store_b.list_clips(None, None, 10).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].body, "Sync protocol review");

        // Duplicate rejection
        assert!(engine_b.process_inbound_envelope(&envelope).is_err());
    }
}
