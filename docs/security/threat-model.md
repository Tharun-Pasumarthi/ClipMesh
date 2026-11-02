# ClipMesh Threat Model & Security Posture

## 1. Scope & System Definition

ClipMesh facilitates cross-device clipboard synchronization and secure file transfers across untrusted networks via a centralized relay server. This threat model applies STRIDE and formal security analysis to identify assets, adversaries, threat vectors, and mitigations.

## 2. Protected Assets

1. **Clipboard Payloads**: Passwords, API tokens, personal communication, PII copied by the user.
2. **File Contents**: Documents, binaries, and sensitive media transferred across devices.
3. **Cryptographic Identity Keys**: Ed25519 signing private keys, X25519 static/ephemeral agreement keys.
4. **Session Credentials**: Short-lived JWTs, rotating refresh token hashes, QR pairing ephemeral keys.
5. **Metadata Privacy**: Communication graph, device presence, synchronization frequency.

## 3. Adversary Profiles

| Adversary | Description & Capabilities | Trust Level |
| :--- | :--- | :--- |
| **Untrusted Relay / Cloud Operator** | Controls the relay infrastructure, can observe/modify traffic, inspect database. | Completely untrusted with data contents. Authenticated router only. |
| **Network Man-in-the-Middle (MitM)** | Active or passive attacker on public Wi-Fi or transit ISP. | Can intercept, drop, delay, or replay packets. |
| **Malicious Peer / Compromised Device** | Compromised user device or attacker attempting unauthorized registration. | Zero trust until authenticated and approved. |
| **Local OS Process (Unprivileged)** | Other unprivileged applications running on the user's host OS. | Sandboxed; prevented from reading protected keystores. |

## 4. Threat Analysis (STRIDE)

### 4.1 Spoofing
- **Threat**: Attacker impersonates an existing user device to receive clipboard updates.
- **Mitigation**: Every device must generate an Ed25519 keypair and register by signing a server-issued nonce (proof of possession). Relay authenticates each request using signature-verified JWT device tokens.

### 4.2 Tampering
- **Threat**: Relay or MitM modifies encrypted clipboard payloads or routing headers.
- **Mitigation**: Authenticated Encryption with Associated Data (AEAD - ChaCha20-Poly1305) cryptographically binds ciphertext with associated data: `protocol_version`, `sender_device_id`, `recipient_device_id`, `sequence_number`, `epoch_id`. Tampering triggers authentication failure; corrupted envelopes are rejected immediately.

### 4.3 Repudiation
- **Threat**: Compromised device denies having sent a payload or forged an event.
- **Mitigation**: Payloads are individually signed by the sender device's Ed25519 private key or MAC'd using a ratchet-derived key. Monotonic per-device sequence numbers ensure provable ordering.

### 4.4 Information Disclosure
- **Threat**: Relay database dump or server memory inspection leaks user clipboard text or files.
- **Mitigation**: End-to-end encryption. Server only receives opaque `encrypted_payload` bytes. Server possesses no decryption keys. Private keys never leave the origin device's secure enclave / OS Keystore.

### 4.5 Denial of Service
- **Threat**: Attacker floods relay with massive envelopes or attempts to exhaust victim device storage.
- **Mitigation**: 
  - Max clipboard payload size enforced at server and client (e.g., 512 KB for text; files must use chunked transfer).
  - Rate limiting on API endpoints and WebSocket messages using Redis token-bucket.
  - Disk quota and temporary file isolation for chunked transfers.
  - Automatic expiration and purging of unacknowledged relay events (24-hour default TTL).

### 4.6 Elevation of Privilege
- **Threat**: Insecure Direct Object Reference (IDOR) allows Device A to fetch events addressed to Device B (different user or revoked device).
- **Mitigation**: Server-side authorization queries enforce:
  1. `sender_device.user_id == recipient_device.user_id` (or active pairwise trust).
  2. `recipient_device.is_revoked == false`.
  3. Relayed envelope retrieval requires authenticated JWT matching `recipient_device.id`.

## 5. Explicit Non-Goals & Out-of-Scope Risks

1. **Compromised Host OS / Root Malware**: If an attacker has kernel-level or root access on the client device (e.g., memory dumper, keylogger), client-side memory security cannot be guaranteed.
2. **Side-Channel Hardware Attacks**: Physical probing of CPU registers or cold boot memory extraction is out of scope.

