-- ClipMesh Initial Schema Migration

-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. Users Table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Devices Table
CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    platform VARCHAR(32) NOT NULL, -- 'windows', 'android', 'linux', 'macos'
    signing_public_key BYTEA NOT NULL, -- 32-byte Ed25519 public key
    agreement_public_key BYTEA NOT NULL, -- 32-byte X25519 public key
    key_fingerprint VARCHAR(64) NOT NULL, -- SHA-256 hex string of signing key
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    CONSTRAINT unique_device_fingerprint UNIQUE(user_id, key_fingerprint)
);

CREATE INDEX IF NOT EXISTS idx_devices_user_id ON devices(user_id);
CREATE INDEX IF NOT EXISTS idx_devices_revoked ON devices(is_revoked);

-- 3. Pairing Sessions Table
CREATE TABLE IF NOT EXISTS pairing_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    initiator_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    responder_device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    token_hash VARCHAR(64) NOT NULL, -- SHA-256 of one-time pairing challenge
    ephemeral_public_key BYTEA NOT NULL, -- 32-byte X25519 ephemeral key of initiator
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING', -- PENDING, APPROVED, REJECTED, EXPIRED, COMPLETED
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_pairing_token_hash ON pairing_sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_pairing_status_expires ON pairing_sessions(status, expires_at);

-- 4. Relay Events Table
CREATE TABLE IF NOT EXISTS relay_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    recipient_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_payload BYTEA NOT NULL, -- Opaque ciphertext (ChaCha20-Poly1305)
    nonce BYTEA NOT NULL, -- 12-byte unique nonce
    sequence_number BIGINT NOT NULL,
    epoch_id BIGINT NOT NULL DEFAULT 1,
    protocol_version INT NOT NULL DEFAULT 1,
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING', -- PENDING, DELIVERED, ACKNOWLEDGED, FAILED
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    acknowledged_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_relay_recipient_status ON relay_events(recipient_device_id, status);
CREATE INDEX IF NOT EXISTS idx_relay_expires ON relay_events(expires_at);

-- 5. Refresh Sessions Table
CREATE TABLE IF NOT EXISTS refresh_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL UNIQUE, -- SHA-256 hex of refresh token
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refresh_token_hash ON refresh_sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_user_device ON refresh_sessions(user_id, device_id);

-- 6. File Transfers Table
CREATE TABLE IF NOT EXISTS transfers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    recipient_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(128) NOT NULL,
    total_chunks INT NOT NULL,
    received_chunks INT NOT NULL DEFAULT 0,
    whole_file_hash VARCHAR(64) NOT NULL, -- SHA-256 hex string
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING', -- PENDING, IN_PROGRESS, COMPLETED, FAILED, CANCELLED
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- 7. Security Audit Log
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,
    event_type VARCHAR(64) NOT NULL, -- 'DEVICE_REGISTERED', 'DEVICE_REVOKED', 'PAIRING_SUCCESS', etc.
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    ip_address VARCHAR(45),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_logs(user_id);

