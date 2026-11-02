//! Native Windows Clipboard Adapter
//!
//! Provides safe abstraction over Win32 clipboard APIs (`user32.dll`):
//! - Format handling (`CF_UNICODETEXT`)
//! - Retry with backoff if clipboard is locked by another process
//! - Echo-prevention (ignoring updates originating from ClipMesh itself)

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Failed to open clipboard (likely locked by another process)")]
    OpenFailed,

    #[error("Global allocation error")]
    AllocFailed,

    #[error("Failed to lock global memory handle")]
    LockFailed,

    #[error("Failed to set clipboard data")]
    SetDataFailed,

    #[error("Clipboard data is not UTF-16 text")]
    InvalidFormat,
}

#[cfg(windows)]
pub struct WindowsClipboard;

#[cfg(windows)]
impl WindowsClipboard {
    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    /// Read UTF-16 string from native Windows clipboard
    pub fn get_text() -> Result<Option<String>, ClipboardError> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        };
        use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

        // Retry up to 5 times with exponential backoff if another process holds the clipboard lock
        let mut attempts = 0;
        let mut opened = false;
        while attempts < 5 {
            unsafe {
                if OpenClipboard(0 as HWND) != 0 {
                    opened = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(15 * (1 << attempts)));
            attempts += 1;
        }

        if !opened {
            return Err(ClipboardError::OpenFailed);
        }

        struct ClipboardGuard;
        impl Drop for ClipboardGuard {
            fn drop(&mut self) {
                unsafe {
                    CloseClipboard();
                }
            }
        }
        let _guard = ClipboardGuard;

        unsafe {
            if IsClipboardFormatAvailable(Self::CF_UNICODETEXT) == 0 {
                return Ok(None);
            }

            let handle = GetClipboardData(Self::CF_UNICODETEXT);
            if handle.is_null() {
                return Ok(None);
            }

            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                return Err(ClipboardError::LockFailed);
            }

            // Find null terminator length
            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice);

            GlobalUnlock(handle);

            Ok(Some(text))
        }
    }

    /// Write UTF-16 text into native Windows clipboard
    pub fn set_text(text: &str) -> Result<(), ClipboardError> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};

        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes_len = utf16.len() * std::mem::size_of::<u16>();

        unsafe {
            let mut attempts = 0;
            let mut opened = false;
            while attempts < 5 {
                if OpenClipboard(0 as HWND) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(15 * (1 << attempts)));
                attempts += 1;
            }

            if !opened {
                return Err(ClipboardError::OpenFailed);
            }

            struct ClipboardGuard;
            impl Drop for ClipboardGuard {
                fn drop(&mut self) {
                    unsafe {
                        CloseClipboard();
                    }
                }
            }
            let _guard = ClipboardGuard;

            EmptyClipboard();

            let handle = GlobalAlloc(Self::GMEM_MOVEABLE, bytes_len);
            if handle.is_null() {
                return Err(ClipboardError::AllocFailed);
            }

            let ptr = GlobalLock(handle) as *mut u16;
            if ptr.is_null() {
                return Err(ClipboardError::LockFailed);
            }

            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
            GlobalUnlock(handle);

            if SetClipboardData(Self::CF_UNICODETEXT, handle).is_null() {
                return Err(ClipboardError::SetDataFailed);
            }

            Ok(())
        }
    }

    /// Query sequence number to detect changes
    pub fn sequence_number() -> u32 {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber()
        }
        #[cfg(not(windows))]
        0
    }
}

/// Cross-platform Clipboard Manager with Echo-Prevention
pub struct ClipboardManager {
    last_seq: AtomicU32,
    last_hash: parking_lot::RwLock<Option<String>>,
}

impl ClipboardManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            last_seq: AtomicU32::new(WindowsClipboard::sequence_number()),
            last_hash: parking_lot::RwLock::new(None),
        })
    }

    /// Read text if clipboard content has changed
    pub fn poll_change(&self) -> Result<Option<String>, ClipboardError> {
        let current_seq = WindowsClipboard::sequence_number();
        let prev_seq = self.last_seq.swap(current_seq, Ordering::SeqCst);

        if current_seq == prev_seq {
            return Ok(None);
        }

        if let Some(text) = WindowsClipboard::get_text()? {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            let hash = hex::encode(hasher.finalize());

            // Check if this matches our last locally set content (echo prevention)
            let mut last_hash = self.last_hash.write();
            if last_hash.as_ref() == Some(&hash) {
                return Ok(None);
            }

            *last_hash = Some(hash);
            return Ok(Some(text));
        }

        Ok(None)
    }

    /// Write text and mark its hash to prevent echoing back into sync
    pub fn write_text(&self, text: &str) -> Result<(), ClipboardError> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hex::encode(hasher.finalize());

        {
            let mut last_hash = self.last_hash.write();
            *last_hash = Some(hash);
        }

        WindowsClipboard::set_text(text)?;
        self.last_seq.store(WindowsClipboard::sequence_number(), Ordering::SeqCst);
        Ok(())
    }
}
