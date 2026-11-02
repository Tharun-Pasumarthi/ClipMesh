//! ClipMesh Secure Chunked File Transfer Engine
//!
//! Provides chunked streaming, authenticated encryption per chunk,
//! whole-file SHA-256 integrity verification, safe temporary file handling,
//! path-traversal prevention, and resume negotiation.

use chrono::{DateTime, Utc};
use clipmesh_crypto::{
    construct_nonce, decrypt_aead, encrypt_aead, AeadKey, CryptoError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KB per chunk

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Invalid filename or path traversal detected: {0}")]
    UnsafeFilename(String),

    #[error("Chunk integrity check failed for chunk {index}")]
    ChunkCorruption { index: u32 },

    #[error("Whole file SHA-256 mismatch: expected {expected}, got {actual}")]
    FileChecksumMismatch { expected: String, actual: String },

    #[error("Transfer already finished or cancelled")]
    TransferInactive,

    #[error("Exceeded maximum file size: {0} bytes")]
    FileTooLarge(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Metadata describing a file transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferManifest {
    pub transfer_id: Uuid,
    pub sender_device_id: Uuid,
    pub recipient_device_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub total_chunks: u32,
    pub whole_file_sha256: String,
    pub created_at: DateTime<Utc>,
}

/// Individual encrypted file chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedChunk {
    pub transfer_id: Uuid,
    pub chunk_index: u32,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

/// Sanitize filename to prevent directory traversal
pub fn sanitize_filename(input: &str) -> Result<String, TransferError> {
    let path = Path::new(input);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| TransferError::UnsafeFilename(input.to_string()))?;

    // Disallow relative traversal characters
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(TransferError::UnsafeFilename(input.to_string()));
    }
    Ok(file_name.to_string())
}

/// Outbound file transfer sender
pub struct FileSender {
    manifest: TransferManifest,
    file: File,
    key: AeadKey,
    device_prefix: u32,
}

impl FileSender {
    pub fn open<P: AsRef<Path>>(
        file_path: P,
        sender_device_id: Uuid,
        recipient_device_id: Uuid,
        session_key: [u8; 32],
    ) -> Result<Self, TransferError> {
        let mut file = File::open(file_path.as_ref())?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        let raw_name = file_path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed.dat");
        let safe_name = sanitize_filename(raw_name)?;

        // Compute whole file SHA-256
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 32 * 1024];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let whole_file_sha256 = hex::encode(hasher.finalize());
        file.seek(SeekFrom::Start(0))?;

        let total_chunks = ((file_size + (CHUNK_SIZE as u64) - 1) / (CHUNK_SIZE as u64)) as u32;
        let prefix = u32::from_be_bytes([
            sender_device_id.as_bytes()[0],
            sender_device_id.as_bytes()[1],
            sender_device_id.as_bytes()[2],
            sender_device_id.as_bytes()[3],
        ]);

        let manifest = TransferManifest {
            transfer_id: Uuid::new_v4(),
            sender_device_id,
            recipient_device_id,
            file_name: safe_name,
            file_size,
            mime_type: "application/octet-stream".to_string(),
            total_chunks,
            whole_file_sha256,
            created_at: Utc::now(),
        };

        Ok(Self {
            manifest,
            file,
            key: AeadKey::from_bytes(session_key),
            device_prefix: prefix,
        })
    }

    pub fn manifest(&self) -> &TransferManifest {
        &self.manifest
    }

    /// Read and encrypt a specific chunk index
    pub fn read_and_encrypt_chunk(&mut self, chunk_index: u32) -> Result<EncryptedChunk, TransferError> {
        let offset = (chunk_index as u64) * (CHUNK_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;

        let mut buf = vec![0u8; CHUNK_SIZE];
        let bytes_read = self.file.read(&mut buf)?;
        buf.truncate(bytes_read);

        let nonce = construct_nonce(self.device_prefix, chunk_index as u64);
        let mut ad = [0u8; 20];
        ad[0..16].copy_from_slice(self.manifest.transfer_id.as_bytes());
        ad[16..20].copy_from_slice(&chunk_index.to_le_bytes());

        let ciphertext = encrypt_aead(&self.key, &nonce, &buf, &ad)?;

        Ok(EncryptedChunk {
            transfer_id: self.manifest.transfer_id,
            chunk_index,
            nonce,
            ciphertext,
        })
    }
}

/// Inbound file transfer receiver
pub struct FileReceiver {
    manifest: TransferManifest,
    temp_file_path: PathBuf,
    file: Option<File>,
    key: AeadKey,
    received_chunks: std::collections::BTreeSet<u32>,
    status: TransferStatus,
}

impl FileReceiver {
    pub fn create(
        manifest: TransferManifest,
        destination_dir: &Path,
        session_key: [u8; 32],
    ) -> Result<Self, TransferError> {
        let safe_name = sanitize_filename(&manifest.file_name)?;
        let temp_filename = format!(".clipmesh_part_{}_{}", manifest.transfer_id, safe_name);
        let temp_file_path = destination_dir.join(temp_filename);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&temp_file_path)?;

        file.set_len(manifest.file_size)?;

        Ok(Self {
            manifest,
            temp_file_path,
            file: Some(file),
            key: AeadKey::from_bytes(session_key),
            received_chunks: std::collections::BTreeSet::new(),
            status: TransferStatus::InProgress,
        })
    }

    /// Decrypt and write an incoming chunk to the file
    pub fn write_chunk(&mut self, chunk: &EncryptedChunk) -> Result<(), TransferError> {
        if self.status != TransferStatus::InProgress {
            return Err(TransferError::TransferInactive);
        }

        let file = self.file.as_mut().ok_or(TransferError::TransferInactive)?;

        let mut ad = [0u8; 20];
        ad[0..16].copy_from_slice(self.manifest.transfer_id.as_bytes());
        ad[16..20].copy_from_slice(&chunk.chunk_index.to_le_bytes());

        let plaintext = decrypt_aead(&self.key, &chunk.nonce, &chunk.ciphertext, &ad)?;

        let offset = (chunk.chunk_index as u64) * (CHUNK_SIZE as u64);
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&plaintext)?;

        self.received_chunks.insert(chunk.chunk_index);
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.received_chunks.len() == (self.manifest.total_chunks as usize)
    }

    /// Finalize, verify SHA-256 checksum, and rename to target path
    pub fn finalize(&mut self, final_dir: &Path) -> Result<PathBuf, TransferError> {
        if !self.is_complete() {
            return Err(TransferError::TransferInactive);
        }

        let mut file = self.file.take().ok_or(TransferError::TransferInactive)?;
        file.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 32 * 1024];
        let mut remaining = self.manifest.file_size;

        while remaining > 0 {
            let to_read = (buffer.len() as u64).min(remaining) as usize;
            let n = file.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            remaining -= n as u64;
        }

        let actual_hash = hex::encode(hasher.finalize());
        if actual_hash != self.manifest.whole_file_sha256 {
            self.status = TransferStatus::Failed;
            let _ = std::fs::remove_file(&self.temp_file_path);
            return Err(TransferError::FileChecksumMismatch {
                expected: self.manifest.whole_file_sha256.clone(),
                actual: actual_hash,
            });
        }

        drop(file); // Explicitly close Windows file handle before rename

        let target_path = final_dir.join(&self.manifest.file_name);
        std::fs::rename(&self.temp_file_path, &target_path)?;
        self.status = TransferStatus::Completed;

        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_chunked_file_transfer_e2e() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source_document.txt");

        let test_data = b"ClipMesh secure chunked transfer validation payload. Repeated data for testing chunks. ".repeat(1000);
        std::fs::write(&source_path, &test_data).unwrap();

        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();
        let shared_key = [0x99; 32];

        let mut sender = FileSender::open(&source_path, sender_id, recipient_id, shared_key).unwrap();
        let manifest = sender.manifest().clone();

        let mut receiver = FileReceiver::create(manifest.clone(), dir.path(), shared_key).unwrap();

        for i in 0..manifest.total_chunks {
            let chunk = sender.read_and_encrypt_chunk(i).unwrap();
            receiver.write_chunk(&chunk).unwrap();
        }

        assert!(receiver.is_complete());
        let final_path = receiver.finalize(dir.path()).unwrap();
        let received_data = std::fs::read(&final_path).unwrap();

        assert_eq!(received_data, test_data);
    }
}
