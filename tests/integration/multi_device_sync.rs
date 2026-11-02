use clipmesh_core::ClipMeshClient;
use clipmesh_identity::Platform;
use clipmesh_protocol::{AckStage, ClipKind, EnvelopeStatus};
use clipmesh_server::state::{AppState, DeviceRecord};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
#[test]
async fn test_multi_device_sync_e2e_flow() {
    // 1. Initialize Relay Hub State
    let state = AppState::new("integration-test-secret-key-32-chars!".to_string());
    let user_id = Uuid::new_v4();

    // 2. Initialize Device A (Windows Desktop) and Device B (Android Mobile)
    let client_a = Arc::new(
        ClipMeshClient::init_ephemeral("ThinkPad X1".to_string(), Platform::Windows)
            .expect("Client A init failed"),
    );
    let client_b = Arc::new(
        ClipMeshClient::init_ephemeral("Pixel 8 Pro".to_string(), Platform::Android)
            .expect("Client B init failed"),
    );

    // 3. Register both devices in server state under the same authorized account
    {
        let mut devs = state.devices.write();
        devs.insert(
            client_a.identity.id,
            DeviceRecord {
                id: client_a.identity.id,
                user_id,
                name: client_a.identity.name.clone(),
                platform: "windows".to_string(),
                signing_public_key: client_a.identity.signing_keypair.public_key().to_bytes(),
                agreement_public_key: client_a.identity.agreement_keypair.public_key().to_bytes(),
                key_fingerprint: client_a.identity.signing_keypair.public_key().fingerprint(),
                is_revoked: false,
                created_at: Utc::now(),
                last_seen_at: Utc::now(),
            },
        );
        devs.insert(
            client_b.identity.id,
            DeviceRecord {
                id: client_b.identity.id,
                user_id,
                name: client_b.identity.name.clone(),
                platform: "android".to_string(),
                signing_public_key: client_b.identity.signing_keypair.public_key().to_bytes(),
                agreement_public_key: client_b.identity.agreement_keypair.public_key().to_bytes(),
                key_fingerprint: client_b.identity.signing_keypair.public_key().fingerprint(),
                is_revoked: false,
                created_at: Utc::now(),
                last_seen_at: Utc::now(),
            },
        );
    }

    // 4. Perform X25519 Diffie-Hellman mutual trust establishment
    let shared_session_key = [0x42u8; 32];
    client_a.establish_peer_trust(client_b.identity.id, 1, shared_session_key);
    client_b.establish_peer_trust(client_a.identity.id, 1, shared_session_key);

    // 5. Device A copies secret content and publishes to Device B
    let secret_clipboard_text = "https://internal.vault.company/credentials/token-xyz-12345";
    let (stored_a, envelopes) = client_a
        .publish_clipboard(
            ClipKind::Link,
            "Internal Vault Access".to_string(),
            secret_clipboard_text.to_string(),
            true, // pinned
            &[client_b.identity.id],
        )
        .expect("Publish clipboard failed");

    assert_eq!(stored_a.title, "Internal Vault Access");
    assert_eq!(stored_a.body, secret_clipboard_text);
    assert_eq!(envelopes.len(), 1);

    let envelope = envelopes[0].clone();
    assert_eq!(envelope.sender_device_id, client_a.identity.id);
    assert_eq!(envelope.recipient_device_id, client_b.identity.id);
    assert_eq!(envelope.sequence_number, 1);
    assert_eq!(envelope.status, EnvelopeStatus::Pending);

    // 6. Relay routes envelope through zero-knowledge blind queue
    state.route_envelope(envelope.clone()).expect("Relay routing failed");

    // 7. Device B polls pending envelopes from Relay
    let pending_ids = {
        let queues = state.pending_queues.read();
        queues.get(&client_b.identity.id).cloned().unwrap_or_default()
    };
    assert_eq!(pending_ids.len(), 1);

    let queued_envelope = {
        let events = state.relay_events.read();
        events.get(&pending_ids[0]).cloned().expect("Envelope not found")
    };

    // 8. Device B decrypts, verifies AEAD MAC, and applies to local store
    let ack = client_b
        .receive_envelope(&queued_envelope)
        .expect("Client B decryption/receipt failed");

    assert_eq!(ack.stage, AckStage::RecipientApplied);
    assert_eq!(ack.device_id, client_b.identity.id);

    // 9. Verify Device B local storage contains decrypted clipboard content
    let b_history = client_b
        .get_history(None, None, 10)
        .expect("History query failed");

    assert_eq!(b_history.len(), 1);
    assert_eq!(b_history[0].title, "Internal Vault Access");
    assert_eq!(b_history[0].body, secret_clipboard_text);
    assert!(b_history[0].pinned);
}
