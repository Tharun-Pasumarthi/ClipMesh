# ClipMesh

> Production-oriented, privacy-first, cross-platform clipboard synchronisation and secure file-transfer platform.

ClipMesh enables users to copy content on one device and seamlessly access it on other trusted personal devices without exposing plaintext clipboard data or private keys to cloud infrastructure.

---

## Key Capabilities

- **Zero-Knowledge Cloud Relay**: End-to-end encryption using ChaCha20-Poly1305 AEAD. The relay server routes opaque ciphertext envelopes without access to encryption keys.
- **Dual-Key Asymmetric Identity**: Strict algorithmic separation between signing keys (Ed25519) and key agreement (X25519).
- **Proof-of-Possession Authentication**: Device registration and login require cryptographic signature over server nonces, preventing device spoofing.
- **Hardware-Isolated Keystores**: Private keys are maintained in OS-native secure enclaves (Windows DPAPI, macOS Keychain, Linux Secret Service, Android Keystore TEE).
- **Authenticated QR Pairing**: Ephemeral Diffie-Hellman key agreement with Short Authentication String (SAS) confirmation and 120-second timeout.
- **Multi-Stage Delivery Receipts**: Distinguishes server acceptance, network receipt, cryptographic decryption, and local clipboard application.
- **Resumable Chunked File Transfers**: 64 KB authenticated chunks with per-chunk nonces, out-of-order reassembly, directory traversal protection, and whole-file SHA-256 verification.
- **Local-First & Offline Resilience**: Local SQLite database for offline clipboard history, search, pin/unpin, and outbound retry queues.

---

## Architectural Layout

```
clipmesh/
  apps/
    server/              # Axum + Tokio WebSocket & REST relay backend
    desktop/             # Tauri 2 + React + TypeScript + Tailwind v4 desktop app
    android/             # Kotlin + Jetpack Compose Android client with Keystore & Foreground Service
  crates/
    clipmesh-crypto/     # Ed25519, X25519, HKDF-SHA256, ChaCha20-Poly1305, zeroize
    clipmesh-identity/   # Device identity, keystore abstraction, proof of possession
    clipmesh-protocol/   # Wire envelopes, canonical associated data builders, validation
    clipmesh-sync/       # Monotonic sequences, deduplication, vector clocks, multi-stage receipts
    clipmesh-transfer/   # Chunked file streaming, nonce rotation, whole-file verification
    clipmesh-storage/    # Local SQLite storage engine, history, retention cleanup
    clipmesh-core/       # Unified client facade orchestrating all subsystems
  protocol/
    schemas/             # JSON schemas for wire messages
    test-vectors/        # Cryptographic test vectors
  migrations/            # Versioned SQL migrations (PostgreSQL 17)
  infrastructure/
    docker/              # Dockerfiles and compose definitions
  docs/                  # Complete architecture, threat model, key management & ADRs
  tests/                 # Workspace test suites & security verification
```

---

## Quickstart & Verification

### Prerequisites
- Rust stable (1.85+)
- Node.js 20+ & npm

### Run Rust Workspace Tests
```bash
cargo test --workspace
```

### Run Backend Security Integration Tests
```bash
cargo test -p clipmesh-server --test security_tests
```

### Run the Relay Backend Server
```bash
cargo run -p clipmesh-server
```
Runs at `http://localhost:8080`. Check health at `http://localhost:8080/health`.

### Run the Desktop Frontend
```bash
npm install --prefix apps/desktop
npm run dev --prefix apps/desktop
```

---

## Security Architecture & Design Guarantees

Detailed security specifications are located in `/docs`:
- [System Overview](docs/architecture/system-overview.md)
- [Component Boundaries](docs/architecture/component-boundaries.md)
- [Threat Model (STRIDE Analysis)](docs/security/threat-model.md)
- [Key Management Lifecycle](docs/security/key-management.md)
- [Authenticated QR Pairing Protocol](docs/security/pairing-protocol.md)
- [Wire Protocol Specification (v1)](docs/protocols/protocol-v1.md)
- [Architecture Decisions (ADRs)](docs/decisions/architecture-decisions.md)

---

## License

Licensed under Apache-2.0 or MIT.

