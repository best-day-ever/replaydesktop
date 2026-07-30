---
quick_id: 260730-iie
status: complete
completed: 2026-07-30
subsystem: live-prototype-host
---

# Quick Task 260730-iie Summary

The live ReplayDesktop prototype now accepts Kyber's implicit stock Basic
credentials on the LAN/VPN path, so the macOS tester supplies only the host
address. JWT and OIDC are disabled. The existing generated TLS certificate,
private key, and `webtransport_gen_certificate = true` setting remain in place
as transport requirements.

## Changes

- Enabled Basic authentication with username `Default user` and the supplied
  SHA-256 hash for `Default password`.
- Disabled JWT and OIDC.
- Preserved the NvFBC/NVENC host settings, generated TLS identity paths,
  WebTransport certificate generation, port 8080, input, and clipboard.
- Restarted `replaydesktop-kyber-spike.service`.

## Verification

- User service is `active/running`; HTTPS listens on `0.0.0.0:8080`.
- An actual certificate-verification-bypassed curl
  `POST /session/login` using `Default user:Default password` returned HTTP
  200 with a valid session response.
- The temporary cookie jar and response files were removed after a successful
  HTTP 200 logout, leaving the mono-client host available for the tester.
- `GET /enumerate_displays` reports `DP-0.3` at index 0 with dimensions
  3840x2160.
- The pinned `kyclient` parser exposes every flag used below. Kymux's UDP data
  plane remains configured for port 8080 and binds when the client starts the
  Kymux session.

## macOS Launch

```bash
/Applications/ReplayDesktop.app/Contents/MacOS/kyclient --protocol=kymux --tls-skip-verification --video-codec=h264 --display-idx=0 --audio=false --inputs=true --clipboard=true --keyboard-grab=true --video-buffer=0 --metrics=true 100.85.187.73
```

No CA, fingerprint, token, username, password, or other authentication flag is
required. H.264 is requested by the client while NVENC remains explicitly
configured on the host. This verification proves configuration, authentication,
display selection, CLI parsing, and reachability; it does not claim that the
Apple Silicon client has rendered a frame yet.

## Deviations from Plan

None.
