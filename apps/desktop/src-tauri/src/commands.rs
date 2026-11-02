//! Desktop Service & IPC Commands
//!
//! Provides the typed bridge between the React frontend and `clipmesh-core`.

use crate::clipboard::ClipboardManager;
use clipmesh_core::ClipMeshClient;
use clipmesh_protocol::ClipKind;
use clipmesh_storage::StoredClip;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalDeviceProfile {
    pub id: Uuid,
    pub name: String,
    pub platform: String,
    pub fingerprint: String,
    pub is_online: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishClipRequest {
    pub kind: String,
    pub title: String,
    pub text: String,
    pub pinned: bool,
    pub target_recipients: Vec<Uuid>,
}

pub struct DesktopAppState {
    pub client: Arc<ClipMeshClient>,
    pub clipboard_mgr: Arc<ClipboardManager>,
}

pub struct DesktopController {
    state: Arc<DesktopAppState>,
}

impl DesktopController {
    pub fn new(client: Arc<ClipMeshClient>, clipboard_mgr: Arc<ClipboardManager>) -> Self {
        Self {
            state: Arc::new(DesktopAppState {
                client,
                clipboard_mgr,
            }),
        }
    }

    /// Retrieve local device profile
    pub fn get_device_profile(&self) -> LocalDeviceProfile {
        LocalDeviceProfile {
            id: self.state.client.identity.id,
            name: self.state.client.identity.name.clone(),
            platform: "windows".to_string(),
            fingerprint: self.state.client.identity.signing_keypair.public_key().fingerprint(),
            is_online: true,
        }
    }

    /// Publish a new clipboard item locally and to recipient peers
    pub fn publish_clip(&self, req: PublishClipRequest) -> Result<StoredClip, String> {
        let kind = match req.kind.to_lowercase().as_str() {
            "link" => ClipKind::Link,
            "code" => ClipKind::Code,
            _ => ClipKind::Text,
        };

        let (stored, _envelopes) = self
            .state
            .client
            .publish_clipboard(kind, req.title, req.text, req.pinned, &req.target_recipients)
            .map_err(|e| e.to_string())?;

        Ok(stored)
    }

    /// Retrieve clipboard history
    pub fn get_history(
        &self,
        query: Option<&str>,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StoredClip>, String> {
        self.state
            .client
            .get_history(query, filter, limit)
            .map_err(|e| e.to_string())
    }

    /// Toggle pin status
    pub fn toggle_pin(&self, clip_id: i64) -> Result<bool, String> {
        self.state
            .client
            .store
            .toggle_pin(clip_id)
            .map_err(|e| e.to_string())
    }

    /// Delete clip
    pub fn delete_clip(&self, clip_id: i64) -> Result<(), String> {
        self.state
            .client
            .store
            .delete_clip(clip_id)
            .map_err(|e| e.to_string())
    }

    /// Clear unpinned clips
    pub fn clear_unpinned(&self) -> Result<usize, String> {
        self.state
            .client
            .store
            .clear_unpinned()
            .map_err(|e| e.to_string())
    }

    /// Apply retention period cleanup
    pub fn cleanup_retention(&self, days: i64) -> Result<usize, String> {
        self.state
            .client
            .store
            .cleanup_retention(days)
            .map_err(|e| e.to_string())
    }

    /// Copy text directly to the native OS clipboard
    pub fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        self.state
            .clipboard_mgr
            .write_text(text)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipmesh_identity::Platform;

    #[test]
    fn test_desktop_controller_lifecycle() {
        let client = Arc::new(
            ClipMeshClient::init_ephemeral("Test Desktop".to_string(), Platform::Windows).unwrap(),
        );
        let mgr = ClipboardManager::new();
        let controller = DesktopController::new(client, mgr);

        let profile = controller.get_device_profile();
        assert_eq!(profile.name, "Test Desktop");
        assert_eq!(profile.platform, "windows");
        assert!(profile.is_online);

        // Publish clip
        let req = PublishClipRequest {
            kind: "text".to_string(),
            title: "Test Note".to_string(),
            text: "Secret Content".to_string(),
            pinned: false,
            target_recipients: vec![],
        };
        let stored = controller.publish_clip(req).expect("publish failed");
        assert_eq!(stored.title, "Test Note");
        assert_eq!(stored.body, "Secret Content");

        // History retrieval
        let history = controller.get_history(None, None, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, stored.id);

        // Toggle pin
        let is_pinned = controller.toggle_pin(stored.id).unwrap();
        assert!(is_pinned);

        let is_unpinned = controller.toggle_pin(stored.id).unwrap();
        assert!(!is_unpinned);

        // Clear unpinned
        let cleared = controller.clear_unpinned().unwrap();
        assert_eq!(cleared, 1);

        let history_after = controller.get_history(None, None, 10).unwrap();
        assert!(history_after.is_empty());
    }
}
