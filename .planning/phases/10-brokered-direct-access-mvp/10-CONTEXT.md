# Phase 10 Context — Brokered Direct Access MVP

## User story

As a user, I want to open an authorized direct workstation session, so that I
can connect without relaying media.

## Decisions

- Single organization.
- Admin operations are server-local CLI commands.
- Local username/password is the first identity provider.
- Persist identity, grants, sessions, and audit metadata in a local database
  for the one-server MVP; keep repository and admission boundaries replaceable.
- Replay Control signs Kymux tickets. The gateway adapter returns them only
  after a successful UDP source proof and UniFi lease.
- Use static per-workstation DNAT and dynamic source-IP firewall policies.
- Use the official UniFi integration firewall-policy API. Do not depend on
  undocumented dynamic port-forward endpoints.
- Require verified HTTPS for UniFi; no skip-verification option.
- Use a fake admission backend for all automated tests.
- Direct media remains the existing Kymux QUIC/TLS connection from Mac to
  workstation through UDM DNAT.

## MVP scope

- Persistent local users, password hashes, API sessions, workstations, grants,
  pending connections, leases, and audit events.
- Login, refresh, logout, workstation list, session creation/status/close.
- Exact binary UDP knock and source IPv4 binding.
- UniFi create/delete policy adapter.
- Ed25519 Kymux ticket issue and standalone verification library/CLI.
- Server-local admin CLI and operational documentation.

## Deferred

- Public admin UI, registration, password reset, email, MFA, Google/OIDC,
  multi-organization tenancy, horizontal gateway scaling, relay fallback,
  IPv6 admission, QUIC migration across source-address changes, and automatic
  static DNAT creation.
