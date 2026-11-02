use chrono::Utc;
use clipmesh_protocol::{
    EnvelopeStatus, P2pSignalingEnvelope, RelayEnvelope, PROTOCOL_VERSION,
};
use proptest::prelude::*;
use uuid::Uuid;

proptest! {
    /// Fuzz test: Arbitrary random string inputs to envelope deserializers must never panic
    #[test]
    fn test_arbitrary_json_deserialization_never_panics(raw_input in "\\PC*") {
        let _ = serde_json::from_str::<RelayEnvelope>(&raw_input);
        let _ = serde_json::from_str::<P2pSignalingEnvelope>(&raw_input);
    }

    /// Invariant: Valid RelayEnvelopes must round-trip cleanly and preserve Associated Data
    #[test]
    fn test_envelope_roundtrip_and_ad_invariance(
        seq in any::<u64>(),
        epoch in any::<u64>(),
        payload in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();
        let nonce = [42u8; 12];
        let now = Utc::now();

        let original = RelayEnvelope {
            id: Uuid::new_v4(),
            protocol_version: PROTOCOL_VERSION,
            sender_device_id: sender,
            recipient_device_id: recipient,
            sequence_number: seq,
            epoch_id: epoch,
            nonce,
            encrypted_payload: payload,
            created_at: now,
            expires_at: now,
            status: EnvelopeStatus::Pending,
        };

        // Canonical AD must match before and after serialization
        let ad_before = original.associated_data();

        let serialized = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: RelayEnvelope = serde_json::from_str(&serialized).expect("deserialization failed");

        let ad_after = deserialized.associated_data();
        prop_assert_eq!(ad_before, ad_after);
        prop_assert_eq!(original.id, deserialized.id);
        prop_assert_eq!(original.sequence_number, deserialized.sequence_number);
        prop_assert_eq!(original.epoch_id, deserialized.epoch_id);
    }

    /// Invariant: Associated Data must be strictly unique when any input parameter differs
    #[test]
    fn test_associated_data_avoids_collisions(
        seq1 in any::<u64>(),
        seq2 in any::<u64>(),
        epoch1 in any::<u64>(),
        epoch2 in any::<u64>(),
    ) {
        let sender = Uuid::new_v4();
        let recipient = Uuid::new_v4();

        let ad1 = RelayEnvelope::build_associated_data(1, &sender, &recipient, epoch1, seq1);
        let ad2 = RelayEnvelope::build_associated_data(1, &sender, &recipient, epoch2, seq2);

        if seq1 != seq2 || epoch1 != epoch2 {
            prop_assert_ne!(ad1, ad2);
        } else {
            prop_assert_eq!(ad1, ad2);
        }
    }
}
