//! Windows DPAPI Hardware / User Account Bound Keystore
//!
//! Encrypts private identity keys using the Windows Data Protection API (`CryptProtectData`)
//! tied to the user logon session and TPM/DPAPI master key.

#[cfg(windows)]
use crate::{IdentityError, SecureKeyStore};
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use uuid::Uuid;
#[cfg(windows)]
use zeroize::Zeroize;

#[cfg(windows)]
pub struct DpapiSecureKeyStore {
    base_dir: PathBuf,
}

#[cfg(windows)]
impl DpapiSecureKeyStore {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, IdentityError> {
        let path = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        Ok(Self { base_dir: path })
    }

    fn encrypt_bytes(data: &[u8]) -> Result<Vec<u8>, IdentityError> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptProtectData, CRYPT_INTEGER_BLOB,
        };

        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        unsafe {
            let success = CryptProtectData(
                &mut in_blob,
                std::ptr::null(),     // no description
                std::ptr::null_mut(), // no optional entropy
                std::ptr::null_mut(), // reserved
                std::ptr::null_mut(), // prompt struct
                0,                    // flags
                &mut out_blob,
            );

            if success == 0 || out_blob.pbData.is_null() {
                return Err(IdentityError::Storage("CryptProtectData failed".into()));
            }

            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let encrypted = slice.to_vec();
            LocalFree(out_blob.pbData as _);

            Ok(encrypted)
        }
    }

    fn decrypt_bytes(encrypted: &[u8]) -> Result<Vec<u8>, IdentityError> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::{
            CryptUnprotectData, CRYPT_INTEGER_BLOB,
        };

        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        unsafe {
            let success = CryptUnprotectData(
                &mut in_blob,
                std::ptr::null_mut(), // description out
                std::ptr::null_mut(), // optional entropy
                std::ptr::null_mut(), // reserved
                std::ptr::null_mut(), // prompt struct
                0,                    // flags
                &mut out_blob,
            );

            if success == 0 || out_blob.pbData.is_null() {
                return Err(IdentityError::Storage("CryptUnprotectData failed".into()));
            }

            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let decrypted = slice.to_vec();
            LocalFree(out_blob.pbData as _);

            Ok(decrypted)
        }
    }

    fn file_path(&self, device_id: &Uuid, key_type: &str) -> PathBuf {
        self.base_dir.join(format!("{}_{}.dpapi", device_id, key_type))
    }
}

#[cfg(windows)]
impl SecureKeyStore for DpapiSecureKeyStore {
    fn load_signing_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError> {
        let path = self.file_path(device_id, "signing");
        if !path.exists() {
            return Ok(None);
        }
        let ciphertext = std::fs::read(&path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        let mut plaintext = Self::decrypt_bytes(&ciphertext)?;
        if plaintext.len() != 32 {
            return Err(IdentityError::Storage("Invalid key length after DPAPI decryption".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&plaintext);
        plaintext.zeroize();
        Ok(Some(key))
    }

    fn save_signing_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError> {
        let path = self.file_path(device_id, "signing");
        let ciphertext = Self::encrypt_bytes(secret_bytes)?;
        std::fs::write(&path, ciphertext).map_err(|e| IdentityError::Storage(e.to_string()))?;
        Ok(())
    }

    fn load_agreement_key(&self, device_id: &Uuid) -> Result<Option<[u8; 32]>, IdentityError> {
        let path = self.file_path(device_id, "agreement");
        if !path.exists() {
            return Ok(None);
        }
        let ciphertext = std::fs::read(&path).map_err(|e| IdentityError::Storage(e.to_string()))?;
        let mut plaintext = Self::decrypt_bytes(&ciphertext)?;
        if plaintext.len() != 32 {
            return Err(IdentityError::Storage("Invalid key length after DPAPI decryption".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&plaintext);
        plaintext.zeroize();
        Ok(Some(key))
    }

    fn save_agreement_key(&self, device_id: &Uuid, secret_bytes: &[u8; 32]) -> Result<(), IdentityError> {
        let path = self.file_path(device_id, "agreement");
        let ciphertext = Self::encrypt_bytes(secret_bytes)?;
        std::fs::write(&path, ciphertext).map_err(|e| IdentityError::Storage(e.to_string()))?;
        Ok(())
    }

    fn delete_keys(&self, device_id: &Uuid) -> Result<(), IdentityError> {
        let _ = std::fs::remove_file(self.file_path(device_id, "signing"));
        let _ = std::fs::remove_file(self.file_path(device_id, "agreement"));
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn test_dpapi_keystore_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let keystore = DpapiSecureKeyStore::new(temp_dir.path()).unwrap();
        let device_id = Uuid::new_v4();

        let signing_secret = [0x77u8; 32];
        let agreement_secret = [0x88u8; 32];

        keystore.save_signing_key(&device_id, &signing_secret).unwrap();
        keystore.save_agreement_key(&device_id, &agreement_secret).unwrap();

        let loaded_signing = keystore.load_signing_key(&device_id).unwrap().unwrap();
        let loaded_agreement = keystore.load_agreement_key(&device_id).unwrap().unwrap();

        assert_eq!(loaded_signing, signing_secret);
        assert_eq!(loaded_agreement, agreement_secret);

        // Delete keys
        keystore.delete_keys(&device_id).unwrap();
        assert!(keystore.load_signing_key(&device_id).unwrap().is_none());
        assert!(keystore.load_agreement_key(&device_id).unwrap().is_none());
    }
}
