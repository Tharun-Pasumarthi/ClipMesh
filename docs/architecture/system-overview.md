# ClipMesh System Overview

## 1. Executive Summary

**ClipMesh** is a production-grade, privacy-first, cross-platform clipboard synchronisation and secure data/file-transfer platform. It allows users to copy arbitrary text, links, code, and structured content on one personal device and securely access it on other authorized personal devices without exposing plaintext data to cloud infrastructure or intermediate relays.

## 2. Core Principles & Philosophy

1. **Privacy-First & Zero-Knowledge Relay**: Plaintext clipboard data, transfer metadata (non-routing), and file chunks are encrypted at rest on the client and in transit end-to-end. The relay server acts solely as an authenticated, blind router for opaque encrypted envelopes.
2. **Local-First Architecture**: Clipboard history, keys, and session records reside on local device storage. If network connectivity is lost, local functionality remains operational and queues changes until reconnection.
3. **Cryptographically Enforced Device Trust**: Device identity is bound to asymmetric key pairs. Registration requires cryptographic proof of possession. Revocation immediately invalidates cryptographic relationships and authentication tokens.
4. **Resilient Synchronization**: Event delivery tracks monotonic sequences per device, provides explicit delivery/application receipts, handles concurrent edits deterministically, and preserves history.
5. **No Vendor Lock-In & Cloud Agnostic**: Built on standard open cryptographic primitives (Ed25519, X25519, HKDF-SHA256, ChaCha20-Poly1305), open protocols (WebSockets, HTTPS, WebRTC), and open-source persistence (PostgreSQL, Redis).

## 3. High-Level Architecture Topology

```
+-----------------------------------------------------------------------------------+
|                                 USER TRUST ZONE                                   |
|                                                                                   |
|  +-------------------------+                           +-----------------------+  |
|  |     Desktop Client      |                           |     Android Client    |  |
|  |  (Windows / macOS / Lnx)|                           |  (Kotlin / Compose)   |  |
|  |                         |                           |                       |  |
|  |  +-------------------+  |                           |  +-----------------+  |  |
|  |  | React/TS Frontend |  |                           |  | Compose UI      |  |  |
|  |  +--------+----------+  |                           |  +--------+--------+  |  |
|  |           | (IPC)       |                           |           | (UniFFI)  |  |
|  |  +--------v----------+  |                           |  +--------v--------+  |  |
|  |  | Tauri 2 App Core  |  |                           |  | Android Keystore|  |  |
|  |  +--------+----------+  |                           |  +--------+--------+  |  |
|  |           |             |                           |           |           |  |
|  |  +--------v-----------------------------------------------------v--------+  |  |
|  |  |                  Shared Rust Client Core Crates                       |  |  |
|  |  |  clipmesh-crypto | clipmesh-identity | clipmesh-protocol | clipmesh-sync |  |  |
|  |  |  clipmesh-transfer | clipmesh-storage | clipmesh-core                   |  |  |
|  |  +--------------------------------+--------------------------------------+  |  |
|  +-----------------------------------|-----------------------------------------+  |
+--------------------------------------|--------------------------------------------+
                                       | TLS / E2EE Encrypted Envelopes
                                       v
+-----------------------------------------------------------------------------------+
|                             UNTRUSTED RELAY ZONE                                  |
|                                                                                   |
|                      +-------------------------------+                            |
|                      |   ClipMesh Relay & Auth API   |                            |
|                      |        (Axum / Tokio)         |                            |
|                      +---------------+---------------+                            |
|                                      |                                            |
|                    +-----------------+-----------------+                          |
|                    |                                   |                          |
|         +----------v-----------+             +---------v----------+               |
|         |  PostgreSQL 17 DB    |             |  Redis Cache / PubSub              |
|         | (Encrypted Envelopes,|             | (Presence, Transient               |
|         |  Devices, Sessions)  |             |  Coordination, Rate Limiting)      |
|         +----------------------+             +--------------------+               |
+-----------------------------------------------------------------------------------+
```

## 4. Key Subsystems

### 4.1 Shared Client Core (Rust)
- **clipmesh-crypto**: Ed25519 signing, X25519 DH key agreement, HKDF key derivation, ChaCha20-Poly1305 AEAD, zeroization of secrets.
- **clipmesh-identity**: Device keypair management, hardware/OS keystore integration (DPAPI/Keychain/SecretService/Android Keystore), proof-of-possession challenges.
- **clipmesh-protocol**: Versioned envelope schemas, serialization (Protobuf/bincode/JSON), deterministic canonical byte encoding, message validation.
- **clipmesh-sync**: Monotonic sequence tracking, offline queueing, event deduplication, vector clocks, multi-stage state acknowledgement.
- **clipmesh-transfer**: Chunked file pipeline, per-chunk nonces, Merkle/Blake3 chunk verification, resume negotiation, temporary file security.
- **clipmesh-storage**: Local SQLite / encrypted key-value storage for offline clipboard history and sync state.
- **clipmesh-core**: High-level application facade orchestrating events, adapters, and background tasks.

### 4.2 Backend Relay & API (Axum)
- **Authentication**: Argon2id password hashing (for optional user accounts), rotating refresh tokens with SHA-256 server-side hashes, short-lived JWT device tokens.
- **Device Management**: Device registration with cryptographic challenge-response, device enumeration, revocation cascade.
- **QR Pairing Service**: Short-lived, single-use, rate-limited pairing session coordinator with hash-committed transcript verification.
- **Relay & Queue**: WebSocket hub and HTTP polling fallback for asynchronous envelope delivery with persistent PostgreSQL store-and-forward.
- **File Transfer Staging**: Temporary encrypted chunk storage with strict size, disk quota, and expiration policies.

## 5. Deployment Topology

- Local development: Docker Compose orchestrating `clipmesh-server`, `postgres:17`, and `redis:7-alpine`.
- Production: Containerized deployment on Kubernetes or managed container platforms (Fly.io, AWS ECS, GCP Cloud Run), managed Postgres, managed Redis with TLS enabled everywhere.

