---
quick_id: 260801-ou3
status: complete
completed: 2026-08-01
subsystem: host-firewall
tags: [ufw, nftables, lan, kyber]
key-files:
  modified:
    - /etc/ufw/user.rules
requirements-completed: []
duration: 3m
---

# Quick Task 260801-ou3 Summary

ReplayDesktop now has persistent, source-unrestricted IPv4 UFW allowances for
TCP and UDP port 8080, while the existing `kycontroller` listener remains
unchanged on `0.0.0.0:8080`. The Mac builder proved direct TCP reachability to
both host LAN addresses.

## Change

- Added exactly `allow proto tcp from 0.0.0.0/0 to any port 8080` with comment
  `ReplayDesktop prototype` through the UFW CLI.
- Added exactly `allow proto udp from 0.0.0.0/0 to any port 8080` with the same
  comment through the UFW CLI.
- Added no interface qualifier, IPv6 counterpart, broader port allowance,
  route, NAT, service, Kyber configuration, or repository source change.
- No repository commit was created; this was a runtime-only quick task.

## Before Evidence

- `sudo ufw status verbose`: `Status: active` and
  `Default: deny (incoming), allow (outgoing), deny (routed)`.
- `sudo ufw status numbered` and `sudo ufw show added`: no TCP or UDP 8080
  rule existed.
- `sudo nft -a list chain ip filter ufw-user-input`: no destination-port 8080
  rule existed.
- `sudo ss -H -ltnp '( sport = :8080 )'`:
  `LISTEN 0 64 0.0.0.0:8080 0.0.0.0:* users:(("kycontroller",pid=3206570,fd=10))`.
- `sudo ss -H -lunp '( sport = :8080 )'` returned no UDP listener.

## After Evidence

- `sudo ufw status numbered` reports exactly one rule for each protocol:
  `[10] 8080/tcp ALLOW IN Anywhere # ReplayDesktop prototype` and
  `[11] 8080/udp ALLOW IN Anywhere # ReplayDesktop prototype`.
- `sudo ufw show added` reports exactly one normalized persistent command for
  each rule:
  `ufw allow 8080/tcp comment 'ReplayDesktop prototype'` and
  `ufw allow 8080/udp comment 'ReplayDesktop prototype'`.
- `/etc/ufw/user.rules` lines 47-51 contain the two exact IPv4 tuples, each
  declaring source and destination as `0.0.0.0/0`, followed by only
  `-A ufw-user-input -p tcp --dport 8080 -j ACCEPT` and its UDP equivalent.
  `/etc/ufw/user6.rules` has no `ReplayDesktop prototype` entry.
- `sudo nft -a list chain ip filter ufw-user-input` reports
  `tcp dport 8080 ... accept # handle 219` and
  `udp dport 8080 ... accept # handle 223`. After the remote probes, the TCP
  rule counter was `2 packets 128 bytes`; the UDP rule counter was zero.
- The live IPv4 `INPUT` chain retains `policy drop`; traffic reaches
  `ufw-user-input` from `ufw-before-input` before returning to the later
  after/log/reject/track path.
- The final TCP listener is byte-for-byte the same `kycontroller` PID/socket
  evidence as before: `pid=3206570`, fd 10, `0.0.0.0:8080`. No UDP 8080 socket
  is currently exposed, and Kyber was not changed to create one.

## Mac Builder Reachability

From `finn@100.119.34.79`:

- `/sbin/route -n get 192.168.33.42` returned an active direct host route on
  `en31` (`UP,HOST,DONE,LLINFO,WASCLONED,IFSCOPE,IFREF`, MTU 1500).
- `/sbin/route -n get 192.168.33.64` returned the same active direct `en31`
  route characteristics.
- `/usr/bin/nc -vz -G 3 192.168.33.42 8080` exited 0:
  `Connection to 192.168.33.42 port 8080 [tcp/http-alt] succeeded!`
- `/usr/bin/nc -vz -G 3 192.168.33.64 8080` exited 0:
  `Connection to 192.168.33.64 port 8080 [tcp/http-alt] succeeded!`

## Deviations from Plan

None - the runtime firewall task was executed exactly as planned.

## Self-Check: PASSED

Both persistent UFW rules, both compiled nftables accepts, the unchanged
all-interface listener, and both routed Mac-to-host TCP connections were
verified successfully.
