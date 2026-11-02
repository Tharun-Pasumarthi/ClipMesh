//! Foreign Function Interface (FFI) for Android and Mobile Integration
//!
//! Exposes a memory-safe C ABI that allows Kotlin / JNI or Swift / Objective-C
//! runtimes to interact with `clipmesh-core`.

use crate::ClipMeshClient;
use clipmesh_identity::Platform;
use clipmesh_protocol::ClipKind;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

pub struct ClipMeshClientHandle {
    pub client: Arc<ClipMeshClient>,
}

/// Initialize an ephemeral or in-memory ClipMesh client
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_create(
    device_name: *const c_char,
    platform_code: u8,
) -> *mut ClipMeshClientHandle {
    if device_name.is_null() {
        return std::ptr::null_mut();
    }

    let name = match CStr::from_ptr(device_name).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    let platform = match platform_code {
        1 => Platform::Android,
        2 => Platform::Linux,
        3 => Platform::Macos,
        _ => Platform::Windows,
    };

    match ClipMeshClient::init_ephemeral(name, platform) {
        Ok(client) => {
            let handle = Box::new(ClipMeshClientHandle {
                client: Arc::new(client),
            });
            Box::into_raw(handle)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free client handle
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_destroy(handle: *mut ClipMeshClientHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get Device ID as UUID string
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_device_id(
    handle: *const ClipMeshClientHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle = &*handle;
    let id_str = handle.client.identity.id.to_string();
    CString::new(id_str).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Get Device Public Key Fingerprint
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_fingerprint(
    handle: *const ClipMeshClientHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle = &*handle;
    let fp = handle.client.identity.signing_keypair.public_key().fingerprint();
    CString::new(fp).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Publish clipboard item, returning stored clip JSON
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_publish_clip(
    handle: *mut ClipMeshClientHandle,
    kind: *const c_char,
    title: *const c_char,
    body: *const c_char,
    pinned: bool,
) -> *mut c_char {
    if handle.is_null() || kind.is_null() || title.is_null() || body.is_null() {
        return std::ptr::null_mut();
    }
    let handle = &*handle;

    let kind_str = match CStr::from_ptr(kind).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let title_str = match CStr::from_ptr(title).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    let body_str = match CStr::from_ptr(body).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    let clip_kind = match kind_str.to_lowercase().as_str() {
        "link" => ClipKind::Link,
        "code" => ClipKind::Code,
        _ => ClipKind::Text,
    };

    match handle
        .client
        .publish_clipboard(clip_kind, title_str, body_str, pinned, &[])
    {
        Ok((stored, _)) => {
            if let Ok(json) = serde_json::to_string(&stored) {
                CString::new(json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
            } else {
                std::ptr::null_mut()
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Retrieve history as JSON array
#[no_mangle]
pub unsafe extern "C" fn clipmesh_client_get_history(
    handle: *const ClipMeshClientHandle,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle = &*handle;

    match handle.client.get_history(None, None, limit) {
        Ok(clips) => {
            if let Ok(json) = serde_json::to_string(&clips) {
                CString::new(json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
            } else {
                std::ptr::null_mut()
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a CString allocated by clipmesh FFI functions
#[no_mangle]
pub unsafe extern "C" fn clipmesh_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_lifecycle() {
        unsafe {
            let name = CString::new("Pixel 8 Pro").unwrap();
            let handle = clipmesh_client_create(name.as_ptr(), 1); // 1 = Android
            assert!(!handle.is_null());

            let id_ptr = clipmesh_client_device_id(handle);
            assert!(!id_ptr.is_null());
            let id_str = CStr::from_ptr(id_ptr).to_str().unwrap();
            assert_eq!(id_str.len(), 36);
            clipmesh_free_string(id_ptr);

            let fp_ptr = clipmesh_client_fingerprint(handle);
            assert!(!fp_ptr.is_null());
            let fp_str = CStr::from_ptr(fp_ptr).to_str().unwrap();
            assert!(!fp_str.is_empty());
            clipmesh_free_string(fp_ptr);

            let kind = CString::new("text").unwrap();
            let title = CString::new("Note").unwrap();
            let body = CString::new("Hello from FFI").unwrap();
            let clip_ptr = clipmesh_client_publish_clip(
                handle,
                kind.as_ptr(),
                title.as_ptr(),
                body.as_ptr(),
                false,
            );
            assert!(!clip_ptr.is_null());
            let clip_json = CStr::from_ptr(clip_ptr).to_str().unwrap();
            assert!(clip_json.contains("Hello from FFI"));
            clipmesh_free_string(clip_ptr);

            let history_ptr = clipmesh_client_get_history(handle, 10);
            assert!(!history_ptr.is_null());
            let history_json = CStr::from_ptr(history_ptr).to_str().unwrap();
            assert!(history_json.contains("Hello from FFI"));
            clipmesh_free_string(history_ptr);

            clipmesh_client_destroy(handle);
        }
    }
}
