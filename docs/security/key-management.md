# ClipMesh Key Management Architecture

## 1. Cryptographic Primitive Selection

ClipMesh adheres to conservative, audited, and modern standard primitives:

1. **Identity & Signatures: Ed25519 (RFC 8032)**
   - Used for device identification, authenticating pairing handshakes, and signing protocol envelopes.
   - 256-bit security level, resistant to side-channel attacks, small 32-byte public keys and 64-byte signatures.
2. **Key Agreement: X25519 (RFC 7748)**
   - Used for Diffie-Hellman key exchange between trusted devices during pairing and channel establishment.
   - Dual-key architecture: signing keys (Ed25519) and agreement keys (X25519) are strictly separated. Cross-protocol attacks are prevented by prohibiting key reuse between algorithms.
3. **Key Derivation: HKDF-SHA256 (RFC 5869)**
   - Used to expand shared secrets into directional encryption and MAC keys with domain-specific info strings.
   - `HKDF-Extract(salt, IKM)` followed by `HKDF-Expand(PRK, info, L)`.
4. **Symmetric Encryption: ChaCha20-Poly1305 (RFC 8439)**
   - Authenticated Encryption with Associated Data (AEAD).
   - High performance across all architectures (including mobile/Android without hardware AES instructions).
   - Nonce construction: 96-bit unique nonces composed of a 32-bit device prefix and a 64-bit monotonic counter.

## 2. Key Hierarchy & Lifecycle

```
[Device Identity]
  ├── Ed25519 Keypair (Static, generated at registration)
  └── X25519 Keypair  (Static, stored in secure keystore)

[Pairing Handshake]
  ├── Ephemeral X25519 Keypair (Generated per QR pairing session)
  └── Ephemeral Shared Secret  = X25519(sk_ephemeral_A, pk_ephemeral_B)
         │
         ▼ HKDF-SHA256 (info = "clipmesh-pairing-v1")
      Pairing Key + Confirmation Verification Code

[Pairwise Mesh Channel]
  ├── Master Shared Secret     = X25519(sk_agreement_A, pk_agreement_B)
         │
         ▼ HKDF-SHA256 (info = "clipmesh-epoch-v1", epoch_id)
      Epoch Encryption Key + Monotonic Nonce Counter
```

## 3. Secure Storage Implementations

Under no circumstances are private keys stored in plaintext on disk, in browser localStorage, in unencrypted databases, or transmitted over network links.

| Platform | Keystore Implementation | Key Isolation Strategy |
| :--- | :--- | :--- |
| **Windows** | Windows Data Protection API (DPAPI) / Windows Credential Manager | Keys encrypted with DPAPI tied to the logged-in Windows user account (`CryptProtectData`). |
| **macOS** | Apple Keychain Services | Stored in `kSecClassGenericPassword` with access control flags. |
| **Linux** | Secret Service API (via DBus / freedesktop) / fallback encrypted ring | Protected using user session keyring. |
| **Android** | Android Keystore Provider | Keys generated inside TEE (Hardware-backed Trusted Execution Environment / StrongBox). |

## 4. Key Rotation & Device Revocation

1. **Epoch Advancement**:
   - Each device pair maintains an `epoch_id` (u64).
   - Epoch rotates after $2^{32}$ messages or 30 days, or immediately upon user request.
2. **Device Revocation**:
   - When a device is marked `revoked`:
     a. Server marks `is_revoked = true` in PostgreSQL and blacklists all active JWTs.
     b. Relay broadcasts a signed `RevocationNotification` envelope to all active devices.
     c. All peer devices remove the revoked device's public keys from their local trust stores and securely zeroize pairwise keys.
     d. Future messages encrypted for or by the revoked device are dropped by both relay and clients.

