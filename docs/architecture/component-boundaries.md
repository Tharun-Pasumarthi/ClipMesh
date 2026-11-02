# ClipMesh Component Boundaries & Interfaces

## 1. Boundary Separation Principles

To prevent security leakages and preserve clean software design:
1. **Cryptographic Core vs Platform Adapters**:
   - `clipmesh-crypto` contains zero platform-specific UI or clipboard code. It operates exclusively on raw byte slices and strong types (`PublicKey`, `SecretKey`, `Nonce`, `Ciphertext`).
   - Platform clipboard access (Windows `user32`/`ole32`, Android `ClipboardManager`, Wayland/X11 protocols, macOS `NSPasteboard`) lives strictly in dedicated client shell adapters.
2. **Untrusted Relay vs Trusted Endpoints**:
   - The relay server NEVER possesses encryption or decryption keys for user payloads.
   - The server inspects only envelope routing headers: `id`, `sender_device_id`, `recipient_device_id`, `protocol_version`, `status`, `expires_at`.
   - The payload `encrypted_payload` is treated as an opaque byte sequence.
3. **Storage Security Boundaries**:
   - Local database (`clipmesh-storage`) stores local history, pending outbound messages, and local configuration.
   - Long-term cryptographic private keys are NEVER stored in plaintext in the database or config files. They are protected via OS-level hardware-backed keychains (Windows DPAPI, macOS Keychain, Linux Secret Service, Android Keystore).
4. **Desktop IPC Boundary (Tauri 2)**:
   - Frontend React UI communicates with the Tauri core via strongly-typed commands (`tauri::command`) and event listeners.
   - Private keys are never serialized across the Tauri IPC boundary to the webview.
   - Only user-facing decrypted clipboard items, device statuses, and transfer progress are exposed to the UI layer.

## 2. Crate Dependencies & Boundaries

```
[clipmesh-crypto]
  ^
  | depends on
[clipmesh-identity]
  ^
  | depends on
[clipmesh-protocol]
  ^
  | depends on
[clipmesh-sync] <----+
  ^                  |
  | depends on       | depends on
[clipmesh-transfer]  |
  ^                  |
  |                  |
[clipmesh-storage] --+
  ^
  |
[clipmesh-core] (Orchestrator)
  ^
  +---------------+---------------+
  |                               |
[apps/desktop (Tauri)]    [apps/android (UniFFI)]
```

### 2.1 Crate Contract Matrix

| Crate | Responsibility | Prohibited Dependencies |
| :--- | :--- | :--- |
| `clipmesh-crypto` | Key generation, DH exchange, HKDF, ChaCha20-Poly1305, zeroize | Network, UI, Filesystem |
| `clipmesh-identity`| Hardware keystore abstractions, device fingerprinting, proof-of-possession | Direct network calls, UI |
| `clipmesh-protocol`| Wire format, envelopes, validation, serialization, test vectors | Direct hardware access, UI |
| `clipmesh-sync` | Event ordering, sequence numbers, vector clocks, deduplication, retry queues | Platform clipboards |
| `clipmesh-transfer`| Chunking, reassembly, Merkle tree hashing, resume logic | Plaintext transfer |
| `clipmesh-storage` | Local SQLite / RocksDB caching, query pagination, cleanup | Network sockets |
| `clipmesh-core` | High-level orchestrator connecting sync, identity, and storage | Direct OS GUI widgets |

