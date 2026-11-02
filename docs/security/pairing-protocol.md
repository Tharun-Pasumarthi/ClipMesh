# ClipMesh Authenticated QR Pairing Protocol

## 1. Objectives

1. Enable secure introduction between an existing trusted device ($D_A$) and an unauthenticated new device ($D_B$).
2. Guarantee Mutual Authentication: $D_A$ and $D_B$ authenticate each other's Ed25519 public keys without trusting the relay server.
3. Mitigate QR Code substitution and relay MitM attacks via Out-of-Band (OOB) optical confirmation and Short Authentication String (SAS) verification.
4. Prevent replay attacks via single-use challenges and strict timeouts (120 seconds).

## 2. Cryptographic Transcript & Protocol Steps

### Phase 1: Initiation on Existing Device ($D_A$)
1. $D_A$ generates an ephemeral X25519 keypair: $(sk_{eph}^A, pk_{eph}^A)$.
2. $D_A$ generates a random 256-bit pairing challenge $C_{pair}$.
3. $D_A$ submits a pairing session to the server:
   - Server computes $H_{token} = \text{SHA-256}(C_{pair})$ and stores session record with `expires_at = now + 120s`.
4. $D_A$ renders the QR code containing:
   ```json
   {
     "v": 1,
     "sid": "session-uuid-v4",
     "pk_eph": "<hex-encoded-pk_eph_A>",
     "fp_a": "<hex-encoded-fingerprint-of-D_A-identity-key>",
     "token": "<hex-encoded-C_pair>"
   }
   ```

### Phase 2: Scanning & Key Agreement on Joining Device ($D_B$)
1. $D_B$ scans the QR code and parses the payload.
2. $D_B$ generates its own ephemeral X25519 keypair: $(sk_{eph}^B, pk_{eph}^B)$.
3. $D_B$ computes Diffie-Hellman shared secret:
   $$SS_{eph} = \text{X25519}(sk_{eph}^B, pk_{eph}^A)$$
4. $D_B$ derives the session keys using HKDF-SHA256:
   $$(K_{enc}, K_{mac}, \text{SAS}) = \text{HKDF-Expand}(\text{HKDF-Extract}(\text{salt}=C_{pair}, SS_{eph}), \text{"clipmesh-pairing-v1"}, 72)$$
   - $\text{SAS}$ is formatted as a 6-digit confirmation code or short word pair for visual inspection.
5. $D_B$ creates an authenticated pairing request containing its identity keys ($PK_{sign}^B$, $PK_{agree}^B$), device metadata, and an Ed25519 signature over $(sid, pk_{eph}^A, pk_{eph}^B, C_{pair})$.
6. $D_B$ encrypts this payload with $K_{enc}$ and transmits it to the server relay with $H_{token}$.

### Phase 3: Approval & Transcript Confirmation on $D_A$
1. Server validates that the token hash matches the pending session and notifies $D_A$.
2. $D_A$ computes $SS_{eph} = \text{X25519}(sk_{eph}^A, pk_{eph}^B)$ and derives the identical $(K_{enc}, K_{mac}, \text{SAS})$.
3. $D_A$ displays the SAS and device name to the user.
4. The user verifies the SAS matches on both screens and clicks "Approve".
5. $D_A$ signs the confirmation transcript and sends its own encrypted identity payload to $D_B$.
6. Both devices add each other's static identity keys to their local trusted devices database.
7. The pairing session is marked `completed` and the token is invalidated.

