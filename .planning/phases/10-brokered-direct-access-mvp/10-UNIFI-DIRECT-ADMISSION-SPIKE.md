# UniFi Direct-Admission Spike

## Question

Can the UDM Pro SE activate and remove an External-to-Internal UDP policy fast
enough for interactive login while preserving the direct Kymux path and exact
source address?

## Automated proof

- Serialize the request schema from the official Network integration OpenAPI
  contract.
- Refuse HTTP base URLs, invalid UUIDs, non-IPv4 sources, and missing API
  configuration.
- Use a fake backend for session tracer tests.
- Verify cleanup is idempotent and only stored ReplayDesktop policy IDs are
  deleted.

## Live proof on the UDM Pro SE

1. Manually provision one unique static UDP DNAT mapping to a test workstation.
2. Confirm the baseline External-to-Internal policy blocks the mapping.
3. Run five create-policy calls through the configured local integration API.
4. From the exact authorized external IPv4, measure API response-to-first-Kymux
   Initial acceptance.
5. Confirm another IPv4 remains blocked.
6. Delete each policy and measure last-packet-to-block activation.
7. Repeat with IDS/IPS enabled exactly as production will run.
8. Capture rule ordering, whether destination matching is pre- or post-DNAT,
   policy IDs, Network/firmware versions, p50/p95 activation, and cleanup.

## Pass condition

- The official API schema is accepted without an undocumented endpoint.
- Correct source reaches only the selected UDP mapping.
- Wrong source remains blocked.
- Kymux sees the original public source IPv4.
- Create/delete activation is bounded and suitable for a visible
  `authorizing network` connection state.
- No packet traverses the Replay server.

Until this live spike passes, the real UniFi backend is implemented but
deployment remains opt-in; automated success proves only the fake boundary.
