//! ClipMesh Device Identity & Key Lifecycle
//!
//! Manages cryptographic device identity, hardware-backed/OS-level keystore abstraction,
//! and proof-of-possession challenges for registration and authentication.

use clipmesh_crypto::{
    AgreementKeyPair, CryptoError, SigningKeyPair, SigningPublicKey,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[cfg(windows)]
pub mod dpapi;
#[cfg(windows)]
pub use dpapi::DpapiSecureKeyStore;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Keystore error: {0}")]
    KeystoreError(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Invalid challenge response or signature")]
    InvalidProofOfPossession,

    #[error("Device revoked or unrecognized")]
    DeviceRevoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Android,
    Linux,
    Macos,
}

/// Abstract Hardware / OS Secure Keystore
pub trait SecureKeyStore: Send + Sync {
    fn load_signing_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError>;
    fn save_signing_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError>;
    fn load_agreement_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError>;
    fn save_agreement_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError>;
    fn delete_keys(&self, device_id: &Uuid) -> Result<(), IdentityError>;
}

/// In-Memory Secure Storage (for testing, volatile ephemeral environments)
#[derive(Default)]
pub struct MemoryKeyStore {
    inner: std::sync::RwLock<std::collections::HashMap<Uuid, ([u8; 32], [u8; 32])>>,
}

impl SecureKeyStore for MemoryKeyStore {
    fn load_signing_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError> {
        let map = self.inner.read().unwrap();
        Ok(map.get(device_id).map(|(s, _)| *s))
    }

    fn save_signing_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError> {
        let mut map = self.inner.write().unwrap();
        let entry = map.entry(*device_id).or_insert(([0u8; 32], [0u8; 32]));
        entry.0 = *secret_bytes;
        Ok(())
    }

    fn load_agreement_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError> {
        let map = self.inner.read().unwrap();
        Ok(map.get(device_id).map(|(_, a)| *a))
    }

    fn save_agreement_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError> {
        let mut map = self.inner.write().unwrap();
        let entry = map.entry(*device_id).or_insert(([0u8; 32], [0u8; 32]));
        entry.1 = *secret_bytes;
        Ok(())
    }

    fn delete_keys(&self, device_id: &Uuid) -> Result<(), IdentityError> {
        let mut map = self.inner.write().unwrap();
        map.remove(device_id);
        Ok(())
    }
}

/// Cryptographic Device Identity instance
pub struct DeviceIdentity {
    pub id: Uuid,
    pub name: String,
    pub platform: Platform,
    pub signing_keypair: SigningKeyPair,
    pub agreement_keypair: AgreementKeyPair,
}

impl DeviceIdentity {
    /// Generate a brand new cryptographic device identity and persist keys in keystore
    pub fn generate_and_persist(
        name: String,
        platform: Platform,
        keystore: &dyn SecureKeyStore,
    ) -> Result<Self, IdentityError> {
        let id = Uuid::new_v4();
        let signing = SigningKeyPair::generate();
        let agreement = AgreementKeyPair::generate();

        keystore.save_signing_key(&id, &signing.to_bytes())?;
        keystore.save_agreement_key(&id, &agreement.to_bytes())?;

        Ok(Self {
            id,
            name,
            platform,
            signing_keypair: signing,
            agreement_keypair: agreement,
        })
    }

    /// Load existing device identity from keystore
    pub fn load_from_keystore(
        id: Uuid,
        name: String,
        platform: Platform,
        keystore: &dyn SecureKeyStore,
    ) -> Result<Option<Self>, IdentityError> {
        let sign_bytes = keystore.load_signing_key(&id)?;
        let agree_bytes = keystore.load_agreement_key(&id)?;

        match (sign_bytes, agree_bytes) {
            (Some(s), Some(a)) => Ok(Some(Self {
                id,
                name,
                platform,
                signing_keypair: SigningKeyPair::from_bytes(&s),
                agreement_keypair: AgreementKeyPair::from_bytes(&a),
            })),
            _ => Ok(None),
        }
    }

    /// Public verification identity for registration
    pub fn public_registration(&self) -> DevicePublicDescriptor {
        DevicePublicDescriptor {
            id: self.id,
            name: self.name.clone(),
            platform: self.platform,
            signing_public_key: hex::encode(self.signing_keypair.public_key().to_bytes()),
            agreement_public_key: hex::encode(self.agreement_keypair.public_key().to_bytes()),
            key_fingerprint: self.signing_keypair.public_key().fingerprint(),
        }
    }

    /// Respond to server-issued proof of possession challenge
    pub fn create_proof_of_possession(&self, challenge: &[u8]) -> ProofOfPossession {
        let signature = self.signing_keypair.sign(challenge);

        ProofOfPossession {
            device_id: self.id,
            challenge: challenge.to_vec(),
            signature_hex: hex::encode(signature),
        }
    }
}

/// Public descriptor transmitted during device registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePublicDescriptor {
    pub id: Uuid,
    pub name: String,
    pub platform: Platform,
    pub signing_public_key: String,
    pub agreement_public_key: String,
    pub key_fingerprint: String,
}

/// Proof of possession payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfPossession {
    pub device_id: Uuid,
    pub challenge: Vec<u8>,
    pub signature_hex: String,
}

impl ProofOfPossession {
    /// Server-side validation of proof of possession
    pub fn verify(&self, public_key_bytes: &[u8; 32]) -> Result<(), IdentityError> {
        let pubkey = SigningPublicKey::from_bytes(public_key_bytes)?;
        let sig_bytes = hex::decode(&self.signature_hex)
            .map_err(|_| IdentityError::InvalidProofOfPossession)?;
        if sig_bytes.len() != 64 {
            return Err(IdentityError::InvalidProofOfPossession);
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);

        pubkey.verify(&self.challenge, &sig_arr).map_err(|_| IdentityError::InvalidProofOfPossession)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_identity_generation_and_keystore() {
        let keystore = MemoryKeyStore::default();
        let identity = DeviceIdentity::generate_and_persist(
            "Test Laptop".to_string(),
            Platform::Windows,
            &keystore,
        )
        .expect("Failed to generate identity");

        let loaded = DeviceIdentity::load_from_keystore(
            identity.id,
            identity.name.clone(),
            identity.platform,
            &keystore,
        )
        .expect("Keystore load failed")
        .expect("Device not found");

        assert_eq!(loaded.id, identity.id);
        assert_eq!(
            loaded.signing_keypair.public_key().to_bytes(),
            identity.signing_keypair.public_key().to_bytes()
        );
    }

    #[test]
    fn test_proof_of_possession_flow() {
        let keystore = MemoryKeyStore::default();
        let identity = DeviceIdentity::generate_and_persist(
            "Test Phone".to_string(),
            Platform::Android,
            &keystore,
        )
        .unwrap();

        let server_challenge = [0x55; 32];
        let pop = identity.create_proof_of_possession(&server_challenge);

        // Verification succeeds with real public key
        assert!(pop.verify(&identity.signing_keypair.public_key().to_bytes()).is_ok());

        // Verification fails with corrupted public key
        let mut corrupted_key = identity.signing_keypair.public_key().to_bytes();
        corrupted_key[0] ^= 0xff;
        assert!(pop.verify(&corrupted_key).is_err());
    }
}
