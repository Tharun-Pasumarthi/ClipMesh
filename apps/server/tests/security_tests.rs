use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use clipmesh_identity::{DeviceIdentity, MemoryKeyStore, Platform};
use clipmesh_server::{
    build_app,
    models::*,
    state::{AppState, DeviceRecord},
};
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app() -> (axum::Router, Arc<AppState>) {
    let state = AppState::new("test-jwt-secret-key-32-chars-long!".to_string());
    let app = build_app(state.clone());
    (app, state)
}

#[tokio::test]
async fn test_auth_invalid_credentials() {
    let (app, _) = setup_app().await;

    // Register user
    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "alice@clipmesh.test",
                "password": "correct_password_123"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(reg_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Try login with incorrect password
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "alice@clipmesh.test",
                "password": "WRONG_PASSWORD"
            })
            .to_string(),
        ))
        .unwrap();

    let login_res = app.oneshot(login_req).await.unwrap();
    assert_eq!(login_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_token_rotation_and_reuse_defense() {
    let (app, _) = setup_app().await;

    // 1. Register
    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": "bob@clipmesh.test",
                "password": "bob_secure_password"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(reg_req).await.unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let auth_res: AuthResponse = serde_json::from_slice(&body).unwrap();
    let old_refresh_token = auth_res.refresh_token;

    // 2. First refresh (valid)
    let ref_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": old_refresh_token }).to_string(),
        ))
        .unwrap();

    let ref_res = app.clone().oneshot(ref_req).await.unwrap();
    assert_eq!(ref_res.status(), StatusCode::OK);

    // 3. Second refresh with the OLD token (must be rejected!)
    let replay_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "refresh_token": old_refresh_token }).to_string(),
        ))
        .unwrap();

    let replay_res = app.oneshot(replay_req).await.unwrap();
    assert_eq!(replay_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_proof_of_possession_verification() {
    let (app, _) = setup_app().await;
    let keystore = MemoryKeyStore::default();
    let identity =
        DeviceIdentity::generate_and_persist("Windows PC".into(), Platform::Windows, &keystore)
            .unwrap();

    let challenge = [0x42; 32];
    let pop = identity.create_proof_of_possession(&challenge);

    // Valid registration
    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/devices/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": identity.name,
                "platform": "windows",
                "signing_public_key": hex::encode(identity.signing_keypair.public_key().to_bytes()),
                "agreement_public_key": hex::encode(identity.agreement_keypair.public_key().to_bytes()),
                "proof_of_possession": {
                    "challenge": hex::encode(challenge),
                    "signature": pop.signature_hex
                }
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(reg_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Invalid signature registration attempt
    let mut bad_sig = hex::decode(&pop.signature_hex).unwrap();
    bad_sig[0] ^= 0xff; // corrupt signature
    let bad_req = Request::builder()
        .method("POST")
        .uri("/api/v1/devices/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Attacker Device",
                "platform": "linux",
                "signing_public_key": hex::encode(identity.signing_keypair.public_key().to_bytes()),
                "agreement_public_key": hex::encode(identity.agreement_keypair.public_key().to_bytes()),
                "proof_of_possession": {
                    "challenge": hex::encode(challenge),
                    "signature": hex::encode(bad_sig)
                }
            })
            .to_string(),
        ))
        .unwrap();

    let bad_res = app.oneshot(bad_req).await.unwrap();
    assert_eq!(bad_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_device_revocation_prevents_relay() {
    let (app, state) = setup_app().await;
    let user_id = Uuid::new_v4();

    // Register two devices
    let dev1_id = Uuid::new_v4();
    let dev2_id = Uuid::new_v4();

    let record1 = DeviceRecord {
        id: dev1_id,
        user_id,
        name: "PC 1".into(),
        platform: "windows".into(),
        signing_public_key: [1; 32],
        agreement_public_key: [2; 32],
        key_fingerprint: "fp1".into(),
        is_revoked: false,
        created_at: chrono::Utc::now(),
        last_seen_at: chrono::Utc::now(),
    };
    let record2 = DeviceRecord {
        id: dev2_id,
        user_id,
        name: "PC 2".into(),
        platform: "windows".into(),
        signing_public_key: [3; 32],
        agreement_public_key: [4; 32],
        key_fingerprint: "fp2".into(),
        is_revoked: false,
        created_at: chrono::Utc::now(),
        last_seen_at: chrono::Utc::now(),
    };

    {
        let mut devs = state.devices.write();
        devs.insert(dev1_id, record1);
        devs.insert(dev2_id, record2);
    }

    // Revoke device 2
    let revoke_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/devices/{}", dev2_id))
        .body(Body::empty())
        .unwrap();

    let revoke_res = app.clone().oneshot(revoke_req).await.unwrap();
    assert_eq!(revoke_res.status(), StatusCode::OK);

    // Attempt to route event to revoked device 2
    let post_req = Request::builder()
        .method("POST")
        .uri("/api/v1/relay/events")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "recipient_device_id": dev2_id,
                "encrypted_payload": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"secret"),
                "nonce": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 12]),
                "sequence_number": 1,
                "epoch_id": 1,
                "protocol_version": 1
            })
            .to_string(),
        ))
        .unwrap();

    let post_res = app.oneshot(post_req).await.unwrap();
    assert_eq!(post_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_idor_cross_account_isolation() {
    let (app, state) = setup_app().await;

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let dev_a = Uuid::new_v4();
    let dev_b = Uuid::new_v4();

    {
        let mut devs = state.devices.write();
        devs.insert(
            dev_a,
            DeviceRecord {
                id: dev_a,
                user_id: user_a,
                name: "User A Phone".into(),
                platform: "android".into(),
                signing_public_key: [1; 32],
                agreement_public_key: [2; 32],
                key_fingerprint: "fpa".into(),
                is_revoked: false,
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
        );
        devs.insert(
            dev_b,
            DeviceRecord {
                id: dev_b,
                user_id: user_b, // DIFFERENT USER ACCOUNT
                name: "User B Laptop".into(),
                platform: "windows".into(),
                signing_public_key: [3; 32],
                agreement_public_key: [4; 32],
                key_fingerprint: "fpb".into(),
                is_revoked: false,
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
        );
    }

    // Device A attempts to route to Device B across account boundary
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/relay/events")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "sender_device_id": dev_a,
                "recipient_device_id": dev_b,
                "encrypted_payload": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"cross_account_exploit"),
                "nonce": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 12]),
                "sequence_number": 1,
                "epoch_id": 1,
                "protocol_version": 1
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    // Must be rejected with 403 FORBIDDEN!
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_qr_pairing_full_lifecycle() {
    let (app, state) = setup_app().await;

    let dev_a = Uuid::new_v4();
    let dev_b = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    {
        let mut devs = state.devices.write();
        devs.insert(
            dev_a,
            DeviceRecord {
                id: dev_a,
                user_id,
                name: "Existing Device".into(),
                platform: "windows".into(),
                signing_public_key: [1; 32],
                agreement_public_key: [2; 32],
                key_fingerprint: "fpa".into(),
                is_revoked: false,
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
        );
        devs.insert(
            dev_b,
            DeviceRecord {
                id: dev_b,
                user_id,
                name: "New Device".into(),
                platform: "android".into(),
                signing_public_key: [3; 32],
                agreement_public_key: [4; 32],
                key_fingerprint: "fpb".into(),
                is_revoked: false,
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
        );
    }

    // 1. Device A initiates pairing
    let init_req = Request::builder()
        .method("POST")
        .uri("/api/v1/pairing/initiate")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "initiator_device_id": dev_a,
                "ephemeral_public_key": hex::encode([0xaa; 32])
            })
            .to_string(),
        ))
        .unwrap();

    let init_res = app.clone().oneshot(init_req).await.unwrap();
    assert_eq!(init_res.status(), StatusCode::OK);
    let init_body = init_res.into_body().collect().await.unwrap().to_bytes();
    let init_data: InitiatePairingResponse = serde_json::from_slice(&init_body).unwrap();

    // 2. Device B scans QR and submits code
    let scan_req = Request::builder()
        .method("POST")
        .uri("/api/v1/pairing/complete")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "token": init_data.pairing_code,
                "responder_device_id": dev_b,
                "responder_ephemeral_key": hex::encode([0xbb; 32])
            })
            .to_string(),
        ))
        .unwrap();

    let scan_res = app.clone().oneshot(scan_req).await.unwrap();
    assert_eq!(scan_res.status(), StatusCode::OK);
    let scan_body = scan_res.into_body().collect().await.unwrap().to_bytes();
    let scan_data: PairingStatusResponse = serde_json::from_slice(&scan_body).unwrap();
    assert_eq!(scan_data.status, "AWAITING_APPROVAL");
    assert!(scan_data.confirmation_code.is_some());

    // 3. Device A approves
    let approve_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/pairing/{}/approve", init_data.session_id))
        .body(Body::empty())
        .unwrap();

    let approve_res = app.oneshot(approve_req).await.unwrap();
    assert_eq!(approve_res.status(), StatusCode::OK);
    let final_body = approve_res.into_body().collect().await.unwrap().to_bytes();
    let final_data: PairingStatusResponse = serde_json::from_slice(&final_body).unwrap();
    assert_eq!(final_data.status, "COMPLETED");
}
