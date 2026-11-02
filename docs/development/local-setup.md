# ClipMesh Local Development & Engineering Guide

## 1. Prerequisites

- **Rust**: 1.85+ (Stable)
- **Node.js**: 20+ / 24+ & npm
- **Docker & Docker Compose** (Optional for local PostgreSQL and Redis orchestration)
- **Android Studio** (for Android client development)

## 2. Directory Structure

```
clipmesh/
  apps/
    server/          # Axum + Tokio backend relay & API server
    desktop/         # Tauri 2 + React + TypeScript + Tailwind v4 desktop app
    android/         # Kotlin + Jetpack Compose Android client
  crates/
    clipmesh-crypto/   # Ed25519, X25519, HKDF-SHA256, ChaCha20-Poly1305, zeroize
    clipmesh-identity/ # Device keypairs, hardware keystores, proof of possession
    clipmesh-protocol/ # Wire format envelopes, canonical AD builder, validation
    clipmesh-sync/     # Monotonic sequences, deduplication, vector clocks, multi-stage receipts
    clipmesh-transfer/ # Chunked resumable file transfer engine with whole-file SHA-256
    clipmesh-storage/  # Local SQLite storage engine, history, offline queue, retention
    clipmesh-core/     # High-level client facade orchestrating all primitives
  protocol/
    schemas/         # JSON schemas for wire messages
    test-vectors/    # Cryptographic test vectors
  migrations/        # Versioned PostgreSQL schema migrations
  infrastructure/
    docker/          # Dockerfiles & container configs
  docs/              # Architectural, security, and protocol specifications
  tests/             # Integration and security test suites
```

## 3. Building and Running

### 3.1 Run All Rust Unit & Integration Tests
```bash
cargo test --workspace
```

### 3.2 Run Backend Security Tests
```bash
cargo test -p clipmesh-server --test security_tests
```

### 3.3 Run the Backend Server
```bash
cargo run -p clipmesh-server
```
The server will bind to `http://0.0.0.0:8080`.
Health endpoint: `http://localhost:8080/health`.

### 3.4 Run the Desktop Application Frontend
```bash
npm install --prefix apps/desktop
npm run dev --prefix apps/desktop
```
Vite dev server will start at `http://localhost:5173`.
Build for production:
```bash
npm run build --prefix apps/desktop
```

### 3.5 Local Infrastructure with Docker Compose
```bash
docker compose up -d
```
Starts `postgres:17-alpine`, `redis:7-alpine`, and `clipmesh-server`.

