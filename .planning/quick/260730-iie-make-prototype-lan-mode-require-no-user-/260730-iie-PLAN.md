---
quick_id: 260730-iie
mode: quick
status: in_progress
date: 2026-07-30
---

# Prototype LAN connection without user-managed trust

Use Kyber's stock development Basic login and TLS verification bypass so an
internal tester only supplies the host address. Kymux still uses encrypted TLS
internally, but the tester does not copy a CA, JWT, username, or password.

## Task 1: Switch the live host to prototype LAN authentication

- Change `/home/finn/.local/share/replaydesktop/kyber-spike.toml` to enable the
  stock `Default user` login, disable JWT and OIDC, and retain the generated
  server and WebTransport certificates as invisible transport details.
- Restart `replaydesktop-kyber-spike.service`.
- Verify the host listens on TCP/UDP 8080 and accepts the client's implicit
  stock Basic credentials over certificate-verification-bypassed HTTPS.

## Task 2: Document and verify the one-command macOS launch

- Provide the exact Apple Silicon client invocation using
  `--tls-skip-verification`, with no trust or auth flags.
- Preserve H.264 NVENC, display index 0 (`DP-0.3`), audio off, input,
  clipboard, keyboard grab, zero video buffer, and metrics.
- Verify CLI parsing and reachability without claiming that a rendered frame
  has been proven before the tester runs the client.
