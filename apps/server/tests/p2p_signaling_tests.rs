use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use clipmesh_identity::{DeviceIdentity, MemoryKeyStore, Platform};
use clipmesh_protocol::{P2pSignalingEnvelope, P2pSignalingMessage};
use clipmesh_server::{
    build_app,
    state::{AppState, DeviceRecord},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app() -> (axum::Router, Arc<AppState>) {
    let state = AppState::new("test-jwt-secret-key-32-chars-long!".to_string());
    let app = build_app(state.clone());
    (app, state)
}

fn create_device(user_id: Uuid, name: &str) -> (DeviceIdentity, DeviceRecord) {
    let ks = MemoryKeyStore::default();
    let id = DeviceIdentity::generate_and_persist(name.to_string(), Platform::Windows, &ks).unwrap();
    let rec = DeviceRecord {
        id: id.id,
        user_id,
        name: name.to_string(),
        platform: "windows".to_string(),
        signing_public_key: id.signing_keypair.public_key().to_bytes(),
        agreement_public_key: id.agreement_keypair.public_key().to_bytes(),
        key_fingerprint: id.signing_keypair.public_key().fingerprint(),
        is_revoked: false,
        created_at: Utc::now(),
        last_seen_at: Utc::now(),
    };
    (id, rec)
}

#[tokio::test]
async fn test_p2p_signaling_exchange() {
    let (app, state) = setup_app().await;
    let user_id = Uuid::new_v4();

    let (_, dev_a) = create_device(user_id, "Device A");
    let (_, dev_b) = create_device(user_id, "Device B");

    {
        let mut devs = state.devices.write();
        devs.insert(dev_a.id, dev_a.clone());
        devs.insert(dev_b.id, dev_b.clone());
    }

    // 1. Device A sends WebRTC Offer to Device B
    let offer_envelope = P2pSignalingEnvelope {
        sender_device_id: dev_a.id,
        recipient_device_id: dev_b.id,
        message: P2pSignalingMessage::Offer {
            sdp: "v=0\r\no=alice 100 2 IN IP4 127.0.0.1\r\ns=-\r\n".to_string(),
        },
        timestamp: Utc::now(),
    };

    let post_req = Request::builder()
        .method("POST")
        .uri("/api/v1/relay/signal")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&offer_envelope).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(post_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Device B retrieves pending signals
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/relay/signals?device_id={}", dev_b.id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let signals: Vec<P2pSignalingEnvelope> = serde_json::from_slice(&body).unwrap();
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sender_device_id, dev_a.id);
    match &signals[0].message {
        P2pSignalingMessage::Offer { sdp } => {
            assert!(sdp.contains("o=alice"));
        }
        _ => panic!("Expected Offer message"),
    }

    // 3. Device B sends Answer back to Device A
    let answer_envelope = P2pSignalingEnvelope {
        sender_device_id: dev_b.id,
        recipient_device_id: dev_a.id,
        message: P2pSignalingMessage::Answer {
            sdp: "v=0\r\no=bob 200 2 IN IP4 127.0.0.1\r\ns=-\r\n".to_string(),
        },
        timestamp: Utc::now(),
    };

    let post_answer_req = Request::builder()
        .method("POST")
        .uri("/api/v1/relay/signal")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&answer_envelope).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(post_answer_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Device A retrieves pending signals
    let get_answer_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/relay/signals?device_id={}", dev_a.id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(get_answer_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let signals: Vec<P2pSignalingEnvelope> = serde_json::from_slice(&body).unwrap();
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].sender_device_id, dev_b.id);
    match &signals[0].message {
        P2pSignalingMessage::Answer { sdp } => {
            assert!(sdp.contains("o=bob"));
        }
        _ => panic!("Expected Answer message"),
    }
}

#[tokio::test]
async fn test_cross_account_signaling_prohibited() {
    let (app, state) = setup_app().await;

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let (_, dev_a) = create_device(user_a, "User A Device");
    let (_, dev_b) = create_device(user_b, "User B Device");

    {
        let mut devs = state.devices.write();
        devs.insert(dev_a.id, dev_a.clone());
        devs.insert(dev_b.id, dev_b.clone());
    }

    // Device A attempts to signal Device B across account boundary
    let cross_envelope = P2pSignalingEnvelope {
        sender_device_id: dev_a.id,
        recipient_device_id: dev_b.id,
        message: P2pSignalingMessage::Offer {
            sdp: "malicious sdp".to_string(),
        },
        timestamp: Utc::now(),
    };

    let post_req = Request::builder()
        .method("POST")
        .uri("/api/v1/relay/signal")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&cross_envelope).unwrap()))
        .unwrap();

    let res = app.oneshot(post_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_prometheus_metrics_and_readiness() {
    let (app, _) = setup_app().await;

    // Check /ready
    let ready_req = Request::builder()
        .method("GET")
        .uri("/ready")
        .body(Body::empty())
        .unwrap();

    let ready_res = app.clone().oneshot(ready_req).await.unwrap();
    assert_eq!(ready_res.status(), StatusCode::OK);

    // Check /metrics
    let metrics_req = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let metrics_res = app.oneshot(metrics_req).await.unwrap();
    assert_eq!(metrics_res.status(), StatusCode::OK);

    let body = metrics_res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("clipmesh_registered_devices_total"));
    assert!(text.contains("clipmesh_relay_events_total"));
    assert!(text.contains("clipmesh_connected_ws_clients"));
}
