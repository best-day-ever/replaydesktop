---
quick_id: 260801-wx7
status: complete
completed: 2026-08-01
commit: f31b95e
---

# Quick Task 260801-wx7 Summary

Added a compact nginx download dashboard to the LAN Docker stack. It serves setup instructions and the verified `ReplayDesktop-v0.3.0-spike.6-arm64.zip` directly from an ignored runtime directory.

## Delivered

- Dashboard compose profile with a configurable `0.0.0.0:80` default and health check.
- Static responsive page with broker details, test credentials, macOS first-launch guidance, security warning, release checksum, and direct download.
- Launcher that rejects non-integer ports, canonicalizes `080` to `80`, validates the exact release SHA-256, stages it read-only, and waits for dashboard health.
- Live instances on wired LAN, Wi-Fi, and Tailscale port 80. Per-interface bindings preserve an unrelated Caddy service already using `127.0.0.1:80`.

## Verification

- `bash -n scripts/run-lan-dashboard.sh`
- Docker Compose configuration validation
- `nginx -t` inside the running container
- HTTP 200 page and ZIP attachment headers
- Downloaded ZIP SHA-256 `a5679ae4156752119af814b90141c173c3c382facff6a83b38405dc12242393f`
- Dashboard container healthy
- Health HTTP 204 on `100.85.187.73`, `192.168.33.42`, and `192.168.33.64`
- Broker health HTTP 204, Kyber service active, and complete broker login/discovery/session/JWT verifier passed
