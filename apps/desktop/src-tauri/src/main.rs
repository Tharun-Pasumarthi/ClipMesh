use clipmesh_core::ClipMeshClient;
use clipmesh_desktop::{ClipboardManager, DesktopController};
use clipmesh_identity::Platform;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,clipmesh_desktop=debug".into()),
        )
        .init();

    info!("Starting ClipMesh Desktop application service...");

    let client = Arc::new(ClipMeshClient::init_ephemeral(
        "Windows Desktop".to_string(),
        Platform::Windows,
    )?);

    let clipboard_mgr = ClipboardManager::new();
    let controller = Arc::new(DesktopController::new(client.clone(), clipboard_mgr.clone()));

    let profile = controller.get_device_profile();
    info!(
        device_id = %profile.id,
        device_name = %profile.name,
        fingerprint = %profile.fingerprint,
        "Device profile loaded successfully"
    );

    // Background clipboard polling worker
    let clipboard_worker = {
        let mgr = clipboard_mgr.clone();
        let client = client.clone();
        tokio::spawn(async move {
            info!("Clipboard background monitor started");
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;
                match mgr.poll_change() {
                    Ok(Some(text)) => {
                        let preview = if text.len() > 32 {
                            format!("{}...", &text[..32])
                        } else {
                            text.clone()
                        };
                        info!(preview = %preview, "Detected local clipboard change");

                        // Store in local history
                        if let Err(e) = client.publish_clipboard(
                            clipmesh_protocol::ClipKind::Text,
                            "Clipboard Item".to_string(),
                            text,
                            false,
                            &[],
                        ) {
                            error!(error = %e, "Failed to store captured clipboard entry");
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!(error = %e, "Clipboard polling error");
                    }
                }
            }
        })
    };

    info!("ClipMesh Desktop is operational. Press Ctrl+C to terminate.");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received. Exiting ClipMesh Desktop.");
        }
        _ = clipboard_worker => {
            info!("Clipboard worker exited.");
        }
    }

    Ok(())
}
