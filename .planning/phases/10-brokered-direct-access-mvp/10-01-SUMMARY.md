---
phase: 10-brokered-direct-access-mvp
plan: 01
status: complete
completed: 2026-07-30
requirements: [AUTH-01, AUTH-02, AUTH-03, AUTH-04, GATE-01, GATE-02, GATE-03, GATE-04, GATE-05]
---

# Phase 10 Plan 01 Summary

Implemented a standalone Rust control server under `server/` without adding a
media relay to ReplayDesktop.

## Delivered

- SQLite identity, future OIDC-link, workstation, grant, API-session,
  connection-session, lease, and audit-ready schema.
- Server-local CLI for Ed25519 key generation, user create/disable,
  workstation registration, and grant/revoke.
- Bounded Argon2id passwords; timing-equalized, per-user and global
  rate-limited login; short opaque access tokens; rotating opaque refresh
  tokens; digest-only token storage.
- Exact 52-byte, 30-second, one-use UDP source proof.
- Fakeable admission boundary plus verified-HTTPS UniFi Network integration
  API firewall-policy create/delete adapter.
- Immediate policy-ID persistence, explicit disconnect, revoked-principal
  cleanup, lease expiry, interrupted-session cleanup, and startup
  reconciliation for recorded leases.
- Direct endpoint and enrolled certificate fingerprint response. No Kymux
  media/input/clipboard dependency or listener exists in the server crate.
- Ed25519 Kymux ticket issue/verify contract with exact algorithm, type, key
  ID, issuer, audience, UUID, peer IPv4, time-window, 1024-byte, and replay
  checks.
- Deployment runbook with static DNAT, TLS reverse proxy, key/database
  permissions, environment, API, host-verifier, and recovery guidance.

## Verification

```text
cargo fmt --manifest-path server/Cargo.toml --check
cargo clippy --manifest-path server/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path server/Cargo.toml --locked
```

All pass. The test suite contains 6 unit tests and 5 integration tests,
including the complete fake-gateway login-to-ready tracer, grant revocation,
disabled-user denial, refresh replay rejection, wrong/replayed knock denial,
ticket failure modes, and idempotent close.

The CLI was also smoke-tested against an isolated SQLite database with
generated keys, an admin, one workstation, one grant, and a bounded
in-memory-backend server start.

## Human gates retained

- No automated test contacts the real UDM Pro SE. The target firmware still
  needs the documented policy-order, DNAT-match, source-preservation, and
  activation/deactivation latency spike.
- The pinned Kyber controller currently maps a locally generated pending token
  to an in-process session. Its accept path must be adapted to the supplied
  verifier and external session identity before a public workstation port is
  safe or an end-to-end remote desktop can be claimed.
- A crash between the UDM accepting a create and returning/persisting its UUID
  requires operator policy-prefix cleanup; inventory reconciliation is future
  work.
