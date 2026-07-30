# ReplayDesktop control server

`replay-control` is the single-organization authentication and direct-admission
server for ReplayDesktop. It handles identities and short control messages. It
does not proxy Kymux video, input, or clipboard traffic.

The deployed flow is:

1. The client logs in over HTTPS and requests an allowed workstation.
2. The server returns a 30-second, one-use UDP knock.
3. The client sends the exact 52-byte knock to the gateway service. The UDP
   source address, not an HTTP forwarding header, becomes the admitted address.
4. The server asks the UDM Pro SE to create one source-IP-to-workstation UDP
   allow policy.
5. The client receives the workstation's direct WAN endpoint, its enrolled TLS
   certificate fingerprint, and a 60-second Ed25519 Kymux ticket bound to that
   source address and workstation.
6. Media travels directly between the client and workstation through the UDM.

This gives one public ingress IP, but not one port: HTTPS and the knock UDP port
reach this service, while every workstation has a unique direct UDP WAN port.

## Current status

The isolated login-to-ready tracer, denial cases, UniFi request serialization,
ticket validation, and cleanup paths are automated. The real UniFi backend is
opt-in. Do not publish workstation ports until the live procedure in
`../.planning/phases/10-brokered-direct-access-mvp/10-UNIFI-DIRECT-ADMISSION-SPIKE.md`
passes on the exact Network and UDM firmware versions being deployed.

The crate includes the strict host-side `TicketVerifier` contract. The pinned
Kyber workstation still needs to call that verifier before
`accept_authentication`; until that integration is deployed, a workstation is
not safe to expose publicly.

## Build and test

Rust 1.89 is pinned by the repository:

```console
cargo fmt --manifest-path server/Cargo.toml --check
cargo clippy --locked --manifest-path server/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path server/Cargo.toml
cargo build --release --locked --manifest-path server/Cargo.toml
```

## Initial local administration

There is deliberately no public administrative mutation API. Run these
commands on the server. Files are created with mode `0600` and existing key
files are never overwritten.

```console
cd server
cargo run --locked -- init-keys
cargo run --locked -- create-user admin --role admin
cargo run --locked -- create-user finn
cargo run --locked -- create-workstation render-01 \
  --lan-ipv4 192.168.20.41 \
  --kymux-port 47990 \
  --wan-port 47990 \
  --certificate-sha256 <64_HEX_CHARACTERS>
cargo run --locked -- grant finn render-01
```

`create-user --password-stdin` is available for non-interactive provisioning;
pass it through a protected pipe, not a command-line argument or shell history.
Use `disable-user`, `grant`, and `revoke` for later changes. Disabling a user
also revokes every active API session.

## UDM Pro SE preparation

Use the supported UniFi Network integration API. Create a narrowly scoped API
key and record the site, External zone, and Internal/workstation-zone UUIDs.
The adapter accepts only an `https://` base URL and uses the system trust store;
there is no insecure certificate switch.

For each workstation, manually create one static UDP destination-NAT mapping:

```text
PUBLIC_IPV4:workstation.wan_port
    -> workstation.lan_ipv4:workstation.kymux_port
```

The current official Network integration API exposes firewall policy CRUD but
not port-forward CRUD, so ReplayDesktop does not create DNAT rules. Configure
the port forward so it does not create a broad automatic allow rule. A
default-deny External-to-Internal policy must remain effective; the dynamic
ReplayDesktop rule is the only temporary source-specific allow.

Verify policy ordering and whether the destination match is evaluated before
or after DNAT with the live spike. The implementation currently targets the
post-DNAT workstation IPv4 and Kymux port.

## Runtime configuration

Put secrets in a root/service-account-readable environment file:

```dotenv
REPLAY_DATABASE_URL=sqlite:///var/lib/replay-control/replay-control.db
REPLAY_HTTP_BIND=127.0.0.1:8080
REPLAY_KNOCK_BIND=0.0.0.0:8444
REPLAY_PUBLIC_HOST=desktop.example.com
REPLAY_KNOCK_ENDPOINT=desktop.example.com:8444
REPLAY_TICKET_PRIVATE_KEY=/var/lib/replay-control/replay-ticket-private.pem
REPLAY_TICKET_ISSUER=https://desktop.example.com
REPLAY_TICKET_KEY_ID=replay-v1
REPLAY_ADMISSION_BACKEND=unifi
REPLAY_UNIFI_BASE_URL=https://udm.example.internal/proxy/network/integration/
REPLAY_UNIFI_SITE_ID=<UUID>
REPLAY_UNIFI_API_KEY=<SECRET>
REPLAY_UNIFI_EXTERNAL_ZONE_ID=<UUID>
REPLAY_UNIFI_INTERNAL_ZONE_ID=<UUID>
```

Run:

```console
server/target/release/replay-control serve
```

The process refuses a non-loopback HTTP bind. Terminate TLS in a reverse proxy
that publishes only the loopback API on TCP 443. Forward UDP 8444 to this
server. Do not place API keys, database files, signing keys, or the HTTP port
on a public interface.

For a no-router local test, set
`REPLAY_ADMISSION_BACKEND=memory`. The process emits a warning and never opens
a real firewall rule.

## Client API

The small client-facing surface is:

```text
POST   /v1/auth/login
POST   /v1/auth/refresh
POST   /v1/auth/logout
GET    /v1/workstations
POST   /v1/workstations/{workstation_id}/sessions
GET    /v1/sessions/{session_id}
DELETE /v1/sessions/{session_id}
```

Access tokens live for 10 minutes. Refresh tokens live for 30 days and rotate
on every use. Only SHA-256 token digests are stored. Login returns the same
error for a missing user and a wrong password, and repeated failures for a
username are rate-limited.

The session creation response contains `session_id`, `knock_endpoint`, and a
base64url `knock_token`. The client constructs:

```text
4 bytes  ASCII "RDK1"
16 bytes session UUID in network byte order
32 bytes decoded knock token
```

No extra bytes are accepted. The client then polls the session URL until
`ready`, `failed`, `closed`, or `expired`. A ready response includes the
direct endpoint, certificate fingerprint, Kymux token, and lease expiry.

## Ticket verification at the workstation

Before accepting Kymux authentication, load only the Ed25519 public key and
construct `TicketVerifier` with the exact issuer and key ID. Call `verify` with
the selected workstation UUID and the QUIC peer IPv4. It rejects:

- a non-EdDSA algorithm, wrong key ID, token type, signature, issuer, or
  workstation audience;
- the wrong peer source, invalid UUID claims, future/not-yet-valid tickets, or
  tickets with a lifetime over 60 seconds;
- tokens over Kymux's 1024-byte bound; and
- a repeated JTI within its validity window.

The in-process replay set is intentionally small and time-bounded. Production
host integration must share it across concurrent Kymux accepts and must not
call `accept_authentication` if verification fails.

The source-IP rule is only an admission optimization. Kymux must still verify
its TLS identity and ticket. A changing client public IP requires a new
session; carrier-grade NAT admits other subscribers sharing that public IPv4
to the firewall rule, but they still cannot pass the signed Kymux ticket and
workstation certificate checks.

## Recovery and operational limits

- Explicit disconnect deletes the owned UniFi policy. Ready leases expire
  after 12 hours. The cleanup loop retries failed deletions.
- On startup, recorded policies left in `opening_lease` are removed before
  listeners start. The returned UniFi policy UUID is persisted immediately
  after creation.
- A process or power failure in the tiny interval between UniFi accepting a
  create request and returning/persisting its UUID cannot be reconciled from
  SQLite alone. Operators must search for the `ReplayDesktop <session UUID>`
  policy prefix after an unclean gateway/API failure. Automated live-policy
  inventory is follow-up work.
- IPv4 is required. This MVP has no NAT traversal, relay, Google SSO, public
  admin UI, multi-organization tenancy, or high-availability database.
- SQLite serializes the MVP control plane. It is not on the media path.
