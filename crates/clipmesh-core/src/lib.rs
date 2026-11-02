//! ClipMesh Core Client Engine
//!
//! Orchestrates cryptography, identity, local storage, event synchronisation,
//! and chunked file transfer into a cohesive, high-level client service.

use clipmesh_identity::{DeviceIdentity, MemoryKeyStore, Platform, SecureKeyStore};
use clipmesh_protocol::{AckEvent, ClipKind, ClipboardPayload, RelayEnvelope};
use clipmesh_storage::{LocalStore, StoredClip};
use clipmesh_sync::SyncEngine;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub mod ffi;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Identity error: {0}")]
    Identity(#[from] clipmesh_identity::IdentityError),

    #[error("Sync error: {0}")]
    Sync(#[from] clipmesh_sync::SyncError),

    #[error("Storage error: {0}")]
    Storage(#[from] clipmesh_storage::StorageError),

    #[error("Transfer error: {0}")]
    Transfer(#[from] clipmesh_transfer::TransferError),

    #[error("Core error: {0}")]
    General(String),
}

/// ClipMesh High-Level Application Client Controller
pub struct ClipMeshClient {
    pub identity: Arc<DeviceIdentity>,
    pub store: Arc<LocalStore>,
    pub sync: Arc<SyncEngine>,
}

impl ClipMeshClient {
    /// Initialize client with persistent local storage and secure keystore
    pub fn init<P: AsRef<Path>>(
        device_name: String,
        platform: Platform,
        storage_path: P,
        keystore: Box<dyn SecureKeyStore>,
    ) -> Result<Self, CoreError> {
        let store = Arc::new(LocalStore::open(storage_path)?);
        let identity = Arc::new(DeviceIdentity::generate_and_persist(
            device_name,
            platform,
            keystore.as_ref(),
        )?);
        let sync = Arc::new(SyncEngine::new(identity.id, store.clone()));

        Ok(Self {
            identity,
            store,
            sync,
        })
    }

    /// Initialize an ephemeral/in-memory client (for testing & demo modes)
    pub fn init_ephemeral(name: String, platform: Platform) -> Result<Self, CoreError> {
        let keystore = MemoryKeyStore::default();
        let store = Arc::new(LocalStore::in_memory()?);
        let identity = Arc::new(DeviceIdentity::generate_and_persist(name, platform, &keystore)?);
        let sync = Arc::new(SyncEngine::new(identity.id, store.clone()));

        Ok(Self {
            identity,
            store,
            sync,
        })
    }

    /// Add a trusted peer device session
    pub fn establish_peer_trust(&self, peer_device_id: Uuid, epoch_id: u64, shared_session_key: [u8; 32]) {
        self.sync.add_peer_session(peer_device_id, epoch_id, shared_session_key);
    }

    /// Copy / record a new clipboard text and produce encrypted envelopes for peers
    pub fn publish_clipboard(
        &self,
        kind: ClipKind,
        title: String,
        text: String,
        pinned: bool,
        recipient_device_ids: &[Uuid],
    ) -> Result<(StoredClip, Vec<RelayEnvelope>), CoreError> {
        let payload = ClipboardPayload {
            mime_type: "text/plain; charset=utf-8".to_string(),
            kind,
            title,
            text,
            pinned,
            origin_device_name: self.identity.name.clone(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        let event_id = Uuid::new_v4();
        let stored = self.store.insert_clip(event_id, &payload)?;

        let mut envelopes = Vec::new();
        for recipient in recipient_device_ids {
            let env = self.sync.create_clipboard_envelope(*recipient, &payload)?;
            envelopes.push(env);
        }

        Ok((stored, envelopes))
    }

    /// Handle incoming envelope from relay
    pub fn receive_envelope(
        &self,
        envelope: &RelayEnvelope,
    ) -> Result<AckEvent, CoreError> {
        let (_msg, ack) = self.sync.process_inbound_envelope(envelope)?;
        Ok(ack)
    }

    /// Retrieve clipboard history
    pub fn get_history(
        &self,
        query: Option<&str>,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StoredClip>, CoreError> {
        Ok(self.store.list_clips(query, filter, limit)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_client_e2e() {
        let client_a = ClipMeshClient::init_ephemeral("Laptop".into(), Platform::Windows).unwrap();
        let client_b = ClipMeshClient::init_ephemeral("Phone".into(), Platform::Android).unwrap();

        let shared_key = [0x55; 32];
        client_a.establish_peer_trust(client_b.identity.id, 1, shared_key);
        client_b.establish_peer_trust(client_a.identity.id, 1, shared_key);

        let (_stored, envelopes) = client_a
            .publish_clipboard(
                ClipKind::Text,
                "API endpoint".into(),
                "POST /api/v1/relay/events".into(),
                false,
                &[client_b.identity.id],
            )
            .unwrap();

        assert_eq!(envelopes.len(), 1);
        let envelope = &envelopes[0];

        let ack = client_b.receive_envelope(envelope).unwrap();
        assert_eq!(ack.stage, clipmesh_protocol::AckStage::RecipientApplied);

        let b_history = client_b.get_history(None, None, 10).unwrap();
        assert_eq!(b_history.len(), 1);
        assert_eq!(b_history[0].body, "POST /api/v1/relay/events");
    }
}
