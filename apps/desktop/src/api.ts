/**
 * ClipMesh Backend API Client
 */

const getApiBase = () => {
  if ((import.meta as any).env?.VITE_API_BASE_URL) {
    return (import.meta as any).env.VITE_API_BASE_URL;
  }
  const host = typeof window !== "undefined" && window.location.hostname ? window.location.hostname : "localhost";
  return `http://${host}:8085/api/v1`;
};

const API_BASE = getApiBase();

export interface DeviceDto {
  id: string;
  user_id: string;
  name: string;
  platform: string;
  signing_public_key: string;
  agreement_public_key: string;
  key_fingerprint: string;
  is_revoked: boolean;
  created_at: string;
  last_seen_at: string;
}

export interface PairingInitResponse {
  session_id: string;
  pairing_code: string;
  qr_payload: string;
  expires_in_seconds: number;
}

export interface TransferDto {
  id: string;
  sender_device_id: string;
  recipient_device_id: string;
  file_name: string;
  file_size: number;
  total_chunks: number;
  received_chunks: number;
  status: string;
  created_at: string;
}

export const api = {
  async getHealth(): Promise<{ status: string }> {
    const res = await fetch(`${API_BASE.replace("/api/v1", "")}/health`);
    return res.json();
  },

  async listDevices(): Promise<DeviceDto[]> {
    try {
      const res = await fetch(`${API_BASE}/devices`);
      if (!res.ok) throw new Error("Failed to fetch devices");
      return await res.json();
    } catch {
      return [];
    }
  },

  async revokeDevice(deviceId: string): Promise<void> {
    await fetch(`${API_BASE}/devices/${deviceId}`, { method: "DELETE" });
  },

  async initiatePairing(initiatorDeviceId: string, ephemeralPublicKeyHex: string): Promise<PairingInitResponse> {
    const res = await fetch(`${API_BASE}/pairing/initiate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        initiator_device_id: initiatorDeviceId,
        ephemeral_public_key: ephemeralPublicKeyHex,
      }),
    });
    if (!res.ok) throw new Error("Failed to initiate pairing");
    return res.json();
  },

  async completePairing(token: string, responderDeviceId: string, responderEphemeralKey: string) {
    const res = await fetch(`${API_BASE}/pairing/complete`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        token,
        responder_device_id: responderDeviceId,
        responder_ephemeral_key: responderEphemeralKey,
      }),
    });
    return res.json();
  },

  async approvePairing(sessionId: string) {
    const res = await fetch(`${API_BASE}/pairing/${sessionId}/approve`, {
      method: "POST",
    });
    return res.json();
  },

  async initiateTransfer(transfer: {
    recipient_device_id: string;
    file_name: string;
    file_size: number;
    mime_type: string;
    total_chunks: number;
    whole_file_hash: string;
  }): Promise<TransferDto> {
    const res = await fetch(`${API_BASE}/transfers/initiate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(transfer),
    });
    return res.json();
  },
};
