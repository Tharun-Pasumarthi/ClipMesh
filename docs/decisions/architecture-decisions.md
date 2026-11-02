# Architecture Decision Records (ADRs)

## ADR-001: Modular Monolith vs Microservices
- **Status**: Approved
- **Context**: The product requires cross-platform clipboard synchronization, authenticated pairing, and chunked file transfer.
- **Decision**: Implement a modular Rust workspace monolith for the server (`apps/server`) and client libraries (`crates/*`). Avoid premature microservices.
- **Consequences**: Fast compile cycles, atomic transactions across postgres models, zero distributed network overhead for internal services, straightforward debugging and testing.

## ADR-002: Dual-Key Asymmetric Identity (Ed25519 + X25519)
- **Status**: Approved
- **Context**: Device identification and end-to-end encryption require asymmetric cryptography. Using the same key for signing and Diffie-Hellman exchange is unsafe and violates modern cryptographic best practices.
- **Decision**: Assign each device two independently generated keypairs: Ed25519 for signing/proof of possession and X25519 for Diffie-Hellman key agreement.
- **Consequences**: Clean separation of security proofs; prevents cross-protocol signature forgery.

## ADR-003: ChaCha20-Poly1305 over AES-256-GCM for Cross-Platform AEAD
- **Status**: Approved
- **Context**: Android devices and low-power ARM Linux devices may not possess hardware AES instructions (ARM Cryptography Extensions), leading to side-channel cache timing attacks or sluggish performance.
- **Decision**: Use ChaCha20-Poly1305 as the default symmetric AEAD algorithm across all platforms.
- **Consequences**: Constant-time execution, uniform cross-platform performance, strong 256-bit security.

## ADR-004: Blind Envelope Relay Architecture
- **Status**: Approved
- **Context**: Users transmit highly sensitive data (passwords, tokens, personal notes). The cloud backend must be zero-knowledge.
- **Decision**: The backend acts strictly as an authenticated blind router. Server receives only ciphertext envelopes and routing UUIDs.
- **Consequences**: Cloud database breach yields zero plaintext data. Enforces rigorous client-side key management.

## ADR-005: Android Background Execution Privacy Baseline
- **Status**: Approved
- **Context**: Android 10+ restricts background clipboard monitoring (`ClipboardManager.addPrimaryClipChangedListener` is blocked when an app does not have foreground focus or the default IME status).
- **Decision**: Respect platform limitations. Support user-initiated manual clip pulls, foreground service synchronization, and Quick Settings tile / notification actions. Never make false promises of unrestricted background clipboard scraping.
- **Consequences**: Full compliance with Google Play Store policies and modern Android privacy guarantees.

