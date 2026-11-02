use clipmesh_core::ClipMeshClient;
use clipmesh_identity::Platform;
use clipmesh_protocol::{ClipKind, RelayEnvelope};
use std::sync::Arc;

#[test]
fn test_ciphertext_tampering_fails_aead_authentication() {
    let client_a = Arc::new(
        ClipMeshClient::init_ephemeral("Device A".to_string(), Platform::Windows).unwrap(),
    );
    let client_b = Arc::new(
        ClipMeshClient::init_ephemeral("Device B".to_string(), Platform::Android).unwrap(),
    );

    let session_key = [0x99u8; 32];
    client_a.establish_peer_trust(client_b.identity.id, 1, session_key);
    client_b.establish_peer_trust(client_a.identity.id, 1, session_key);

    let (_, envelopes) = client_a
        .publish_clipboard(
            ClipKind::Text,
            "Secret Note".to_string(),
            "Confidential payload text".to_string(),
            false,
            &[client_b.identity.id],
        )
        .unwrap();

    let mut tampered_envelope = envelopes[0].clone();

    // Attacker flips one bit in the middle of ciphertext
    let mid = tampered_envelope.encrypted_payload.len() / 2;
    tampered_envelope.encrypted_payload[mid] ^= 0x01;

    // Recipient must reject tampered envelope
    let result = client_b.receive_envelope(&tampered_envelope);
    assert!(result.is_err(), "Decryption must fail when ciphertext is modified");
}

#[test]
fn test_associated_data_tampering_fails_authentication() {
    let client_a = Arc::new(
        ClipMeshClient::init_ephemeral("Device A".to_string(), Platform::Windows).unwrap(),
    );
    let client_b = Arc::new(
        ClipMeshClient::init_ephemeral("Device B".to_string(), Platform::Android).unwrap(),
    );

    let session_key = [0x99u8; 32];
    client_a.establish_peer_trust(client_b.identity.id, 1, session_key);
    client_b.establish_peer_trust(client_a.identity.id, 1, session_key);

    let (_, envelopes) = client_a
        .publish_clipboard(
            ClipKind::Text,
            "Secret Note".to_string(),
            "Confidential payload text".to_string(),
            false,
            &[client_b.identity.id],
        )
        .unwrap();

    // Attacker attempts to change sequence number in plaintext routing header
    let tampered_seq = RelayEnvelope {
        sequence_number: 9999, // tampered sequence
        ..envelopes[0].clone()
    };

    let result = client_b.receive_envelope(&tampered_seq);
    assert!(result.is_err(), "Decryption must fail when associated data is tampered");
}

#[test]
fn test_replay_attack_prevention() {
    let client_a = Arc::new(
        ClipMeshClient::init_ephemeral("Device A".to_string(), Platform::Windows).unwrap(),
    );
    let client_b = Arc::new(
        ClipMeshClient::init_ephemeral("Device B".to_string(), Platform::Android).unwrap(),
    );

    let session_key = [0x99u8; 32];
    client_a.establish_peer_trust(client_b.identity.id, 1, session_key);
    client_b.establish_peer_trust(client_a.identity.id, 1, session_key);

    let (_, envelopes) = client_a
        .publish_clipboard(
            ClipKind::Text,
            "Transfer Funds".to_string(),
            "$10,000 to Account XYZ".to_string(),
            false,
            &[client_b.identity.id],
        )
        .unwrap();

    let envelope = &envelopes[0];

    // First arrival succeeds
    let first_ack = client_b.receive_envelope(envelope);
    assert!(first_ack.is_ok());

    // Attacker intercepts and replays the same envelope
    let replay_result = client_b.receive_envelope(envelope);
    assert!(
        replay_result.is_err(),
        "Replayed sequence number must be detected and rejected"
    );
}
