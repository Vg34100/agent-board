Agent Board LAN Mode (Tauri + Axum)

Overview
- Single exe runs both the desktop app and an embedded HTTP server.
- The same Leptos UI is served from the embedded server and accessible on LAN.
- Mobile/other devices control the same operations on the PC via HTTP.

What Works (Current)
- Desktop window and browsers on the LAN load the UI from `http://<PC_IP>:17872`.
- All HTTP calls hit `/api/invoke` and execute the same Tauri command functions in‑process.
- Data saved via `tauri-plugin-store` is read/written by the HTTP handlers on the PC.

What Did Not Work (Initial Attempts)
- Creating the Tokio listener outside a runtime led to a panic (“no reactor running”).
- Sharing Tauri’s runtime sometimes stalled the HTTP server accept loop in dev.
- Expecting Trunk’s in‑memory dev server output to be available in `dist/` (blank page).
- Enabling Tauri 2’s remote-domain IPC in config (not supported by the current crates).

What Fixed It
- Run Axum on a dedicated thread with its own Tokio runtime.
- Serve embedded `dist/` in release; in dev, fall back to the on‑disk `dist/` folder.
- Ensure `dist/` exists before dev (`trunk build && trunk serve`).
- Add a small JS shim so `window.__TAURI__.core.invoke()` maps to POST `/api/invoke` for remote browsers.

Ports and URLs
- Fixed preferred port: `17872` (falls back to a random port if occupied).
- The app title shows the final LAN URL (e.g., `agent-board — http://192.168.1.23:17872`).
- Health check: `GET /health` returns `ok`.

Dev vs Build
- Dev (`cargo tauri dev`):
  - Trunk runs at 1420 but the window navigates to the embedded Axum server.
  - Assets are served from `dist/` on disk.
- Build (`cargo tauri build`):
  - `dist/` is embedded in the exe via `rust-embed` and served directly from memory.

Security Notes
- No auth token is enabled (LAN only). Anyone on the same network can connect.
- Consider adding a random token + QR code for production use.

How Invoke Works Over HTTP
- Frontend calls `window.__TAURI__.core.invoke(cmd, args)`.
- In desktop webview, Tauri API is not injected for `http://` pages, so the shim defines it to POST `/api/invoke`.
- The server maps `cmd` to the corresponding Rust Tauri command and returns JSON.

Troubleshooting
- Blank page in dev:
  - Ensure `dist/index.html` exists (`trunk build` runs before dev).
  - Check server logs: you should see `GET / -> index.html` then JS/WASM requests.
- No response on 17872:
  - Check for `Axum server: starting accept loop` and the self‑test line `Self-test attempt …`.
  - Windows Firewall may block; allow the app, or test `curl http://127.0.0.1:17872/health`.
- Phone connects but actions don’t work:
  - Watch POST logs for `/api/invoke cmd=...` and inspect any error strings in responses.

Known Gaps / Oddities
- Remote pages do not receive Tauri event streams; they interact via HTTP only.
- Desktop webview and browsers both use the HTTP shim, not native `invoke()`.

Next Improvements (Optional)
- Switch dev to `trunk build --watch` for faster updates without running the dev server.
- Show the LAN URL and a QR code inside the UI.
- Add opt‑in token gating for safety.

