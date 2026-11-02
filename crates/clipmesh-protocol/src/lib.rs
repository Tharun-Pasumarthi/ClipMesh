//! ClipMesh Wire Protocol (v1)
//!
//! Strongly-typed wire envelopes, canonical binary serializers,
//! cryptographic associated data builders, and payload validators.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Serialization / Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Protocol version mismatch: expected {expected}, got {received}")]
    VersionMismatch { expected: u32, received: u32 },

    #[error("Invalid payload format or schema: {0}")]
    InvalidPayload(String),

    #[error("Associated data mismatch or corrupted envelope")]
    CorruptedEnvelope,

    #[error("Payload exceeds maximum permitted size: {size} bytes")]
    PayloadTooLarge { size: usize },
}

/// Status of an envelope in the relay system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvelopeStatus {
    Pending,
    Delivered,
    Acknowledged,
    Failed,
}

/// Multi-stage acknowledgement levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AckStage {
    /// 1. Server accepted envelope into queue
    ServerAccepted,
    /// 2. Recipient client network stack received envelope
    RecipientReceived,
    /// 3. Recipient authenticated & decrypted envelope successfully
    RecipientDecrypted,
    /// 4. Recipient applied event to local OS clipboard / state
    RecipientApplied,
}

/// Explicit delivery and application acknowledgement event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckEvent {
    pub envelope_id: Uuid,
    pub stage: AckStage,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub error_detail: Option<String>,
}

/// Outer Blind Relay Envelope
///
/// Plaintext routing metadata inspected by the relay server.
/// The relay server NEVER possesses keys to read `encrypted_payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayEnvelope {
    pub id: Uuid,
    pub protocol_version: u32,
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub sequence_number: u64,
    pub epoch_id: u64,
    pub nonce: [u8; 12],
    pub encrypted_payload: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: EnvelopeStatus,
}

impl RelayEnvelope {
    /// Max allowable clipboard envelope size: 1 MB
    pub const MAX_ENVELOPE_SIZE: usize = 1024 * 1024;

    /// Build canonical Associated Data (AD) for ChaCha20-Poly1305 AEAD.
    ///
    /// Layout:
    /// [4 bytes version LE] + [16 bytes sender UUID] + [16 bytes recipient UUID] +
    /// [8 bytes epoch LE] + [8 bytes sequence LE]
    pub fn build_associated_data(
        protocol_version: u32,
        sender_device_id: &Uuid,
        recipient_device_id: &Uuid,
        epoch_id: u64,
        sequence_number: u64,
    ) -> [u8; 52] {
        let mut ad = [0u8; 52];
        ad[0..4].copy_from_slice(&protocol_version.to_le_bytes());
        ad[4..20].copy_from_slice(sender_device_id.as_bytes());
        ad[20..36].copy_from_slice(recipient_device_id.as_bytes());
        ad[36..44].copy_from_slice(&epoch_id.to_le_bytes());
        ad[44..52].copy_from_slice(&sequence_number.to_le_bytes());
        ad
    }

    /// Extract associated data for this envelope
    pub fn associated_data(&self) -> [u8; 52] {
        Self::build_associated_data(
            self.protocol_version,
            &self.sender_device_id,
            &self.recipient_device_id,
            self.epoch_id,
            self.sequence_number,
        )
    }

    /// Validate envelope boundary parameters
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                received: self.protocol_version,
            });
        }
        if self.encrypted_payload.is_empty() {
            return Err(ProtocolError::InvalidPayload("Empty payload".to_string()));
        }
        if self.encrypted_payload.len() > Self::MAX_ENVELOPE_SIZE {
            return Err(ProtocolError::PayloadTooLarge {
                size: self.encrypted_payload.len(),
            });
        }
        Ok(())
    }
}

/// Content kind for clipboard items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind {
    Text,
    Link,
    Code,
}

/// Decrypted Clipboard Item Body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub mime_type: String,
    pub kind: ClipKind,
    pub title: String,
    pub text: String,
    pub pinned: bool,
    pub origin_device_name: String,
    pub timestamp_ms: i64,
}

/// Inner Decrypted Protocol Message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InnerMessage {
    ClipboardSync(ClipboardPayload),
    DeviceRevoked {
        revoked_device_id: Uuid,
        reason: String,
    },
    FileTransferOffer {
        transfer_id: Uuid,
        file_name: String,
        file_size: u64,
        total_chunks: u32,
        sha256_hash: String,
    },
}

/// WebRTC P2P Direct Connection Signaling Payload
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum P2pSignalingMessage {
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    },
}

/// P2P Signaling Envelope routed through the server signaling channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pSignalingEnvelope {
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub message: P2pSignalingMessage,
    pub timestamp: DateTime<Utc>,
}

/// QR Code Pairing Payload Format (RFC Compliant URI or JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingQrPayload {
    pub v: u32,
    pub session_id: Uuid,
    pub token: String,
    pub initiator_device_id: Uuid,
    pub initiator_name: String,
    pub ephemeral_public_key: String, // hex encoded
    pub fingerprint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_associated_data_invariance() {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let ad1 = RelayEnvelope::build_associated_data(1, &sender, &recipient, 5, 42);
        let ad2 = RelayEnvelope::build_associated_data(1, &sender, &recipient, 5, 42);
        assert_eq!(ad1, ad2);

        // Different sequence creates different AD
        let ad3 = RelayEnvelope::build_associated_data(1, &sender, &recipient, 5, 43);
        assert_ne!(ad1, ad3);
    }

    #[test]
    fn test_envelope_validation() {
        let env = RelayEnvelope {
            id: Uuid::new_v4(),
            protocol_version: 1,
            sender_device_id: Uuid::new_v4(),
            recipient_device_id: Uuid::new_v4(),
            sequence_number: 1,
            epoch_id: 1,
            nonce: [0u8; 12],
            encrypted_payload: vec![1, 2, 3, 4],
            created_at: Utc::now(),
            expires_at: Utc::now(),
            status: EnvelopeStatus::Pending,
        };
        assert!(env.validate().is_ok());

        let invalid_version = RelayEnvelope {
            protocol_version: 99,
            ..env.clone()
        };
        assert!(invalid_version.validate().is_err());
    }

    #[test]
    fn test_p2p_signaling_serialization() {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();

        let offer_env = P2pSignalingEnvelope {
            sender_device_id: sender,
            recipient_device_id: recipient,
            message: P2pSignalingMessage::Offer {
                sdp: "v=0\r\no=- 12345 2 IN IP4 127.0.0.1\r\n".to_string(),
            },
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&offer_env).expect("failed to serialize offer");
        let decoded: P2pSignalingEnvelope =
            serde_json::from_str(&json).expect("failed to deserialize offer");

        assert_eq!(decoded.sender_device_id, sender);
        assert_eq!(decoded.recipient_device_id, recipient);
        match decoded.message {
            P2pSignalingMessage::Offer { sdp } => {
                assert!(sdp.contains("v=0"));
            }
            _ => panic!("Expected Offer variant"),
        }

        let ice_env = P2pSignalingEnvelope {
            sender_device_id: sender,
            recipient_device_id: recipient,
            message: P2pSignalingMessage::IceCandidate {
                candidate: "candidate:1 1 UDP 2130706431 192.168.1.50 50000 typ host".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_mline_index: Some(0),
            },
            timestamp: Utc::now(),
        };

        let ice_json = serde_json::to_string(&ice_env).expect("failed to serialize ice");
        let decoded_ice: P2pSignalingEnvelope =
            serde_json::from_str(&ice_json).expect("failed to deserialize ice");
        assert_eq!(decoded_ice.message, ice_env.message);
    }
}

