---
title: Brokered direct access architecture
date: 2026-07-30
context: Single-organization public access after the direct ReplayDesktop proof
---

# Brokered Direct Access Architecture

ReplayDesktop will use a public control service for identity and policy while
keeping the high-bandwidth Kymux path direct:

```text
Mac -- HTTPS/UDP control --> Replay Control + Gateway adapter
Mac -- Kymux/QUIC -------> UDM Pro SE -- static DNAT --> workstation
```

## Locked decisions

- One organization; no public registration or tenant model.
- A server-local admin creates local users, workstations, and grants.
- Local passwords now; provider identities are separate records so Google/OIDC
  can be linked later without changing grants.
- Replay Control owns authorization and the Ed25519 ticket-signing key.
- The gateway adapter owns UDP source proof and narrowly scoped UniFi policy
  leases. It does not own passwords or broad authorization.
- The UDM Pro SE has a public IPv4 address and performs the actual data-path
  NAT/firewall work. There is no WireGuard or media relay.
- Each workstation has a stable LAN address, an enrolled certificate
  fingerprint, and a statically provisioned unique public UDP port.
- A temporary source-IP rule reduces exposure but is not authentication.
  Workstation TLS pinning and the Kymux connection ticket remain mandatory.

## Session flow

1. The Mac logs in and requests an assigned workstation.
2. Control creates a short pending session and returns a random UDP knock.
3. The Mac sends the exact binary knock to the gateway adapter.
4. The adapter consumes the knock, records the UDP source IPv4, and asks UniFi
   to allow only that source to the selected internal address and UDP port.
5. Control returns the public endpoint, workstation certificate fingerprint,
   lease expiry, and signed Kymux ticket.
6. The Mac connects directly through the UDM; the workstation validates the
   certificate/ticket and consumes the ticket ID before opening endpoints.
7. Disconnect, expiry, revocation, or reconciliation removes the UniFi policy.

## Trust boundaries

- Compromising the gateway adapter may alter firewall leases, but it cannot
  mint workstation tickets when the signer remains isolated in Control.
- Compromising Control can authorize access by design; it must never gain the
  workstation private key or inspect Kymux content.
- A peer sharing the allowed public IPv4 can reach the port but cannot pass
  workstation TLS and ticket validation.
- The UniFi API key is never accepted from a public request and the adapter
  only reconciles policy IDs it created.

## Deployment

The first implementation may run Control and Gateway in one binary on one
server. The code keeps the admission adapter behind an interface so the
components can later run separately over a narrow mTLS API. HTTPS terminates at
a configured reverse proxy; the application binds HTTP to loopback by default.

Static DNAT and baseline deny/rule ordering are operator-provisioned. The
official UniFi integration API is used only for temporary firewall policies;
automated tests use a memory backend and cannot mutate the real UDM.
