---
phase: 11
name: lan-broker-client-mvp
mode: mvp
date: 2026-08-01
---

# Phase 11 Context: LAN Broker Client MVP

<decisions>

## Locked decisions

- **D-01 — Topology:** One organization. The broker is authentication,
  discovery, presence, and session authorization only. Kymux media/input stays
  direct from the Mac to the registered LAN host.
- **D-02 — Local runtime:** Run the broker in Docker on this Linux laptop and
  publish it on TCP 8090 because the active Kyber host already owns port 8080.
- **D-03 — Firewall:** The current profile must not call UniFi or open any
  firewall rule. The existing UniFi path remains available for the later
  public deployment and its safe defaults must remain fail-closed.
- **D-04 — Development identity:** The local-only profile seeds `finn` with
  password `1337`. This short password is accepted only behind an explicit
  insecure-development bootstrap flag and never through the normal user CLI.
- **D-05 — Host presence:** A registrar sends a stable host ID, name, LAN IPv4,
  Kymux port, certificate fingerprint, and heartbeat. Stale registrations are
  listed offline and cannot create new sessions.
- **D-06 — One login:** The broker signs a short-lived RS256 JWT accepted by
  Kyber's existing JWT backend. The launcher passes it using `--auth-token`, so
  the user is not asked for separate host credentials.
- **D-07 — First-run client:** The Mac asks for a broker hostname/IP before
  login, remembers the normalized URL in UserDefaults, and always provides a
  route to change it later.
- **D-08 — Machine UX:** After login the client shows authorized workstations
  with online/offline state and a connect action. Offline connect is disabled.
- **D-09 — Settings and telemetry:** Connection choices are keyed by
  workstation ID. They remain editable while connected but apply on the next
  connection unless the current Kyber API exposes live renegotiation. The menu
  exposes settings/statistics and disconnect while the child is running.
- **D-10 — Trust:** The broker returns the enrolled host SHA-256 fingerprint;
  the client passes `--tls-fingerprint` instead of adding another trust-all
  path. Broker HTTP is explicitly LAN-development-only until TLS termination is
  added.

## the agent's Discretion

- Exact AppKit layout, colors, card dimensions, polling intervals, and wording
  may follow the existing technical launcher as long as the locked flow and
  honest capability labels remain visible.
- The host registration shared-secret transport and Docker initialization may
  use file-mounted generated secrets as long as no production default secret is
  committed.

</decisions>

## Deferred ideas

- Google or other OIDC SSO.
- Public TLS termination, UniFi admission, and WAN port forwarding.
- Relay/NAT traversal, multi-organization tenancy, and a public admin UI.
- True live codec/display renegotiation inside an already-running Kyber child.
