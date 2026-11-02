# ClipMesh UI

A responsive React + TypeScript + Tailwind CSS v4 UI starter for ClipMesh.

## Included screens
- Overview dashboard
- Clipboard history (search, filters, pin/unpin, delete, copy)
- Trusted device management (demo revoke/restore states)
- QR pairing (generated QR code + one-time code preview)
- File transfers (history, filters, new-transfer modal)
- Security & privacy settings (theme, sync, clipboard monitoring, retention controls)

## Run locally

Requirements: Node.js 20.19+ or 22.12+.

```bash
npm install
npm run dev
```

Open the local URL printed by Vite.

## Build

```bash
npm run build
npm run preview
```

## Notes before production integration
This is a frontend prototype with local sample data. It does not connect to the Rust/Axum API, PostgreSQL, Redis, native clipboard APIs, device identity, or a real pairing service.

Recommended next integration points:
- Replace sample state in `src/App.tsx` with typed API client calls.
- Fetch device and relay state from `/api/v1`.
- Replace demo pairing code with short-lived, server-issued, authenticated pairing sessions.
- Implement device revocation and key rotation in the backend.
- Connect file selection and transfer progress to the Tauri native layer.
- Use the Android app's explicit, OS-compliant clipboard interaction flow; do not promise unrestricted background clipboard monitoring.
- Implement actual E2EE in the shared Rust client core using reviewed protocol libraries. The UI's encryption labels are illustrative and must not be treated as proof that encryption is implemented.

## Theme
Use the theme buttons in the top bar to select light, system, or dark mode. The preference is stored in localStorage.
