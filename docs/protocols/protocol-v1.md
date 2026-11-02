# ClipMesh Wire Protocol Specification (v1)

## 1. Framing & Envelope Format

All messages transmitted across the relay WebSocket or HTTP endpoints use the canonical JSON or binary envelope format defined below.

### 1.1 Outer Relay Envelope (Plaintext / Routing Layer)

The relay server inspects only the outer envelope required to route and store messages:

```json
{
  "id": "urn:uuid:f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
  "protocol_version": 1,
  "sender_device_id": "018f4a21-9922-791b-83cf-58728d1544a0",
  "recipient_device_id": "018f4a21-9922-791b-83cf-58728d1544a1",
  "encrypted_payload": "<base64-encoded-chacha20-poly1305-ciphertext>",
  "nonce": "<base64-encoded-12-byte-nonce>",
  "sequence_number": 42,
  "epoch_id": 1,
  "created_at": 1730590000,
  "expires_at": 1730676400
}
```

### 1.2 Inner Payload (Encrypted Layer)

Decrypted only by the recipient device using the pairwise session key:

```json
{
  "message_type": "CLIPBOARD_SYNC",
  "event_id": "urn:uuid:f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
  "timestamp_ms": 1730590000123,
  "content": {
    "mime_type": "text/plain; charset=utf-8",
    "kind": "text",
    "text": "Secret text string copied by user",
    "pinned": false
  },
  "sender_signature": "<base64-encoded-ed25519-signature>"
}
```

### 1.3 Associated Data (AD) for AEAD

The ChaCha20-Poly1305 authentication tag is computed over the following canonical associated data bytes:

$$\text{AD} = \text{protocol\_version (u32 LE)} \parallel \text{sender\_device\_id (16 bytes)} \parallel \text{recipient\_device\_id (16 bytes)} \parallel \text{epoch\_id (u64 LE)} \parallel \text{sequence\_number (u64 LE)}$$

Any tampering of routing parameters by an intermediate server immediately breaks decryption at the client.

## 2. Acknowledgement Lifecycle

ClipMesh strictly separates delivery and local application states:

```
Sender                     Relay Server                  Recipient
  |                             |                            |
  |--- Encrypted Envelope ----->|                            |
  |<-- 1. Server Accepted ------|                            |
  |                             |--- Encrypted Envelope ---->|
  |                             |<-- 2. Recipient Received --|
  |<-- Forward Ack(Received) ---|                            |
  |                             |                            | (Recipient decrypts,
  |                             |                            |  validates signature,
  |                             |                            |  applies to OS clipboard)
  |                             |<-- 3. Recipient Applied ---|
  |<-- Forward Ack(Applied) ----|                            |
```

Status Enum:
- `PENDING`: Stored at relay, awaiting recipient connection.
- `DELIVERED`: Received by recipient network stack.
- `ACKNOWLEDGED`: Cryptographically authenticated, decrypted, and applied to recipient local state.
- `FAILED`: Decryption failure, signature failure, or rejected by recipient policy.

