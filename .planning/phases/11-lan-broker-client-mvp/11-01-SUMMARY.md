---
phase: 11
plan: 01
status: complete
completed: 2026-08-01
---

# Phase 11 Plan 01 Summary

Implemented the complete single-organization LAN broker tracer without placing
Kymux traffic on the broker or calling UniFi.

## Delivered

- Persistent host registration and heartbeat-backed online/offline presence.
- Automatic one-organization grants between active users and registered hosts.
- Authenticated LAN session creation returning the registered direct endpoint,
  enrolled certificate SHA-256, and a bounded RS256 Kyber JWT.
- Explicitly gated LAN HTTP and `finn / 1337` bootstrap behavior; production
  defaults still require loopback/TLS and default to the UniFi admission path.
- Docker broker and registrar services with persistent generated secrets and
  no firewall/router configuration.
- A broker-first AppKit launcher with remembered broker URL, in-memory login
  credentials/tokens, online workstation picker, per-workstation settings,
  fingerprint-pinned Kyber launch, and persistent menu-bar settings/statistics
  and disconnect controls.

## Verification result

- Rust format, clippy, 12 unit/integration tests, and doc tests passed.
- Docker broker health, host heartbeat, login, discovery, direct session,
  Kyber JWT acceptance, logout, and cleanup passed against the live laptop.
- The launcher compiled on the real arm64 Mac with Xcode 26.2 and all 192
  built-in combinations plus broker/JWT/fingerprint self-tests passed.
- From that Mac, the live flow reached `192.168.33.42:8090`, discovered
  `archbae1337`, received `192.168.33.42:8080`, and authenticated to Kyber with
  the broker-issued JWT.
- The final Mac source compiles with the port written as an ASCII decimal
  string, preventing German locale grouping from turning `8080` into `8.080`.

## Operational boundary

The running LAN profile is intentionally cleartext on the broker hop and uses
the requested weak test password. It is not safe for public exposure. Public
TLS termination and the already-separated UniFi direct-admission gate remain a
later deployment step.
