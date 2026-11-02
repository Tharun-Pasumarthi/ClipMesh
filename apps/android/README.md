# ClipMesh Android Client

## 1. Overview
The ClipMesh Android application provides secure clipboard synchronisation and encrypted file transfer on mobile devices powered by Android 8.0+ (API level 26+).

## 2. Architecture & Components
- **Language & Framework**: Kotlin 2.0+ & Jetpack Compose.
- **Shared Core**: Integrates `clipmesh-core` via UniFFI Rust bindings.
- **Hardware Keystore**: Hardware-isolated identity keys generated and preserved inside Android Keystore (`KeystoreManager.kt`).
- **Foreground Synchronization**: Foreground service (`ClipboardSyncService.kt`) with persistent notification complying with Android 10+ background clipboard restrictions.
- **Pairing**: In-app CameraX QR scanner + `clipmesh://pair` deep linking.

