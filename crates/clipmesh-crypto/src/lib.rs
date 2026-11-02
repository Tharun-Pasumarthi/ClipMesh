//! ClipMesh Cryptographic Primitives
//!
//! Provides reviewed, standard cryptographic implementations:
//! - Ed25519 for identity and signing
//! - X25519 for Diffie-Hellman key agreement
//! - HKDF-SHA256 for key expansion
//! - ChaCha20-Poly1305 for authenticated encryption (AEAD)
//! - Zeroization of memory holding secret keys on drop

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Nonce,
};
use ed25519_dalek::{Signer, Verifier};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;
use std::fmt;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption or authentication failed")]
    DecryptionFailed,

    #[error("Invalid key length or format: {0}")]
    InvalidKey(String),

    #[error("Signature verification failed")]
    InvalidSignature,

    #[error("Key derivation failed: {0}")]
    DerivationFailed(String),
}

/// Device Signing Keypair (Ed25519) with guaranteed zeroization on drop
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SigningKeyPair {
    secret_bytes: [u8; 32],
}

impl SigningKeyPair {
    /// Generate a fresh random Ed25519 keypair using OS CSPRNG
    pub fn generate() -> Self {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        Self {
            secret_bytes: signing_key.to_bytes(),
        }
    }

    /// Import from 32-byte seed
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            secret_bytes: *bytes,
        }
    }

    /// Export raw secret bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret_bytes
    }

    /// Get corresponding public key
    pub fn public_key(&self) -> SigningPublicKey {
        let key = ed25519_dalek::SigningKey::from_bytes(&self.secret_bytes);
        SigningPublicKey(key.verifying_key())
    }

    /// Sign an arbitrary message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let key = ed25519_dalek::SigningKey::from_bytes(&self.secret_bytes);
        key.sign(message).to_bytes()
    }
}

/// Ed25519 Verifying (Public) Key
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SigningPublicKey(pub ed25519_dalek::VerifyingKey);

impl SigningPublicKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
        Ok(Self(verifying_key))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn verify(&self, message: &[u8], signature_bytes: &[u8; 64]) -> Result<(), CryptoError> {
        let signature = ed25519_dalek::Signature::from_bytes(signature_bytes);
        self.0
            .verify(message, &signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    pub fn fingerprint(&self) -> String {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(self.to_bytes());
        hex::encode(hasher.finalize())
    }
}

impl fmt::Debug for SigningPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SigningPublicKey({})", &self.fingerprint()[..8])
    }
}

/// Diffie-Hellman Agreement Keypair (X25519) with guaranteed zeroization on drop
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AgreementKeyPair {
    secret_bytes: [u8; 32],
}

impl AgreementKeyPair {
    /// Generate a fresh random X25519 keypair
    pub fn generate() -> Self {
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(&mut OsRng);
        Self {
            secret_bytes: secret_key.to_bytes(),
        }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            secret_bytes: *bytes,
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret_bytes
    }

    pub fn public_key(&self) -> AgreementPublicKey {
        let secret = x25519_dalek::StaticSecret::from(self.secret_bytes);
        AgreementPublicKey(x25519_dalek::PublicKey::from(&secret))
    }

    /// Perform X25519 Diffie-Hellman with a peer's public key
    pub fn diffie_hellman(&self, peer_public: &AgreementPublicKey) -> [u8; 32] {
        let secret = x25519_dalek::StaticSecret::from(self.secret_bytes);
        let shared = secret.diffie_hellman(&peer_public.0);
        *shared.as_bytes()
    }
}

/// X25519 Agreement Public Key
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AgreementPublicKey(pub x25519_dalek::PublicKey);

impl AgreementPublicKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(x25519_dalek::PublicKey::from(*bytes))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        *self.0.as_bytes()
    }

    pub fn fingerprint(&self) -> String {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(self.to_bytes());
        hex::encode(hasher.finalize())
    }
}

impl fmt::Debug for AgreementPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AgreementPublicKey({})", &self.fingerprint()[..8])
    }
}

/// Symmetric Key for ChaCha20-Poly1305 AEAD
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AeadKey {
    key: [u8; 32],
}

impl AeadKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Derive symmetric key material using HKDF-SHA256
pub fn derive_key(
    shared_secret: &[u8; 32],
    salt: &[u8],
    info: &[u8],
) -> Result<AeadKey, CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(salt), shared_secret);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .map_err(|e| CryptoError::DerivationFailed(e.to_string()))?;
    Ok(AeadKey::from_bytes(okm))
}

/// Construct a unique 96-bit (12-byte) AEAD Nonce:
/// [4 bytes device prefix] + [8 bytes monotonic sequence counter]
pub fn construct_nonce(device_prefix: u32, counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[0..4].copy_from_slice(&device_prefix.to_be_bytes());
    nonce[4..12].copy_from_slice(&counter.to_be_bytes());
    nonce
}

/// Encrypt plaintext with ChaCha20-Poly1305 and bind associated data
pub fn encrypt_aead(
    key: &AeadKey,
    nonce_bytes: &[u8; 12],
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let payload = Payload {
        msg: plaintext,
        aad: associated_data,
    };
    cipher
        .encrypt(nonce, payload)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))
}

/// Decrypt ciphertext with ChaCha20-Poly1305 and verify associated data
pub fn decrypt_aead(
    key: &AeadKey,
    nonce_bytes: &[u8; 12],
    ciphertext_and_tag: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key.as_bytes())
        .map_err(|_| CryptoError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let payload = Payload {
        msg: ciphertext_and_tag,
        aad: associated_data,
    };
    cipher
        .decrypt(nonce, payload)
        .map_err(|_| CryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ed25519_sign_and_verify() {
        let keypair = SigningKeyPair::generate();
        let pubkey = keypair.public_key();
        let message = b"ClipMesh Secure Signature Verification Test";

        let signature = keypair.sign(message);
        assert!(pubkey.verify(message, &signature).is_ok());

        let corrupted_message = b"ClipMesh Corrupted Signature Verification Test";
        assert!(pubkey.verify(corrupted_message, &signature).is_err());
    }

    #[test]
    fn test_x25519_diffie_hellman_agreement() {
        let alice = AgreementKeyPair::generate();
        let bob = AgreementKeyPair::generate();

        let alice_shared = alice.diffie_hellman(&bob.public_key());
        let bob_shared = bob.diffie_hellman(&alice.public_key());

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_chacha20_poly1305_roundtrip() {
        let key = AeadKey::from_bytes([0x42; 32]);
        let nonce = construct_nonce(1, 100);
        let plaintext = b"Sensitive clipboard payload contents.";
        let aad = b"sender_123|recipient_456|seq_100";

        let ciphertext = encrypt_aead(&key, &nonce, plaintext, aad).expect("encryption failed");
        assert_ne!(&ciphertext[..], plaintext);

        let decrypted = decrypt_aead(&key, &nonce, &ciphertext, aad).expect("decryption failed");
        assert_eq!(decrypted, plaintext);

        // Verify tampering detection
        let tampered_aad = b"sender_123|recipient_456|seq_101";
        assert!(decrypt_aead(&key, &nonce, &ciphertext, tampered_aad).is_err());
    }
}
