use clipmesh_crypto::{decrypt_aead, encrypt_aead, AeadKey, AgreementKeyPair};
use clipmesh_protocol::RelayEnvelope;
use uuid::Uuid;

#[test]
fn test_wire_compatibility_crypto_vectors() {
    let vector_json = include_str!("../../protocol/test-vectors/crypto-vectors-v1.json");
    let parsed: serde_json::Value = serde_json::from_str(vector_json).expect("Invalid test vectors JSON");

    let vectors = parsed["vectors"].as_array().expect("Expected vectors array");

    // Test 1: ChaCha20-Poly1305 AEAD round-trip with Associated Data
    let aead_vec = &vectors[0];
    let key_bytes: [u8; 32] = hex::decode(aead_vec["key_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
    let nonce_bytes: [u8; 12] = hex::decode(aead_vec["nonce_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
    let plaintext = aead_vec["plaintext_utf8"].as_str().unwrap().as_bytes();
    let ad = aead_vec["associated_data_utf8"].as_str().unwrap().as_bytes();

    let key = AeadKey::from_bytes(key_bytes);
    let ciphertext = encrypt_aead(&key, &nonce_bytes, plaintext, ad)
        .expect("AEAD encryption with test vector failed");

    let decrypted = decrypt_aead(&key, &nonce_bytes, &ciphertext, ad)
        .expect("AEAD decryption with test vector failed");

    assert_eq!(decrypted, plaintext);

    // Test 2: X25519 DH mutual agreement RFC 7748 test vectors
    let dh_vec = &vectors[1];
    let alice_priv: [u8; 32] = hex::decode(dh_vec["alice_private_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
    let bob_priv: [u8; 32] = hex::decode(dh_vec["bob_private_hex"].as_str().unwrap()).unwrap().try_into().unwrap();

    let alice_kp = AgreementKeyPair::from_bytes(&alice_priv);
    let bob_kp = AgreementKeyPair::from_bytes(&bob_priv);

    // Alice public key matches RFC test vector
    assert_eq!(
        hex::encode(alice_kp.public_key().to_bytes()),
        dh_vec["alice_public_hex"].as_str().unwrap()
    );

    // Bob public key matches RFC test vector
    assert_eq!(
        hex::encode(bob_kp.public_key().to_bytes()),
        dh_vec["bob_public_hex"].as_str().unwrap()
    );

    // Mutual DH agreement produces identical shared secrets
    let secret_alice = alice_kp.diffie_hellman(&bob_kp.public_key());
    let secret_bob = bob_kp.diffie_hellman(&alice_kp.public_key());
    assert_eq!(secret_alice, secret_bob);
}

#[test]
fn test_canonical_associated_data_binary_layout() {
    let sender = Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap();
    let recipient = Uuid::parse_str("87654321-4321-4321-4321-cba987654321").unwrap();

    let ad = RelayEnvelope::build_associated_data(1, &sender, &recipient, 10, 42);

    // Layout check: [4B version LE] + [16B sender] + [16B recipient] + [8B epoch LE] + [8B seq LE] = 52 Bytes
    assert_eq!(ad.len(), 52);

    // Protocol version = 1 in Little Endian
    assert_eq!(&ad[0..4], &[1, 0, 0, 0]);

    // Sender bytes
    assert_eq!(&ad[4..20], sender.as_bytes());

    // Recipient bytes
    assert_eq!(&ad[20..36], recipient.as_bytes());

    // Epoch = 10 in Little Endian
    assert_eq!(&ad[36..44], &10u64.to_le_bytes());

    // Sequence = 42 in Little Endian
    assert_eq!(&ad[44..52], &42u64.to_le_bytes());
}
