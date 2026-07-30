# Spike Manifest

## Idea

Prove the shortest honest ReplayDesktop vertical slice: stream the physical
Linux X11 output `DP-0.3` from the NVIDIA host to an Apple Silicon Mac through
the pinned Kyber/Kymux stack, with authenticated direct connectivity and
hardware H.264 4:2:0 as the first visible-pixels codec.

## Requirements

- A successful spike ends with a live Mac window displaying the Linux
  `DP-0.3` desktop. A local encoded frame is not success.
- Use Kyber Desktop `0.27.0` at
  `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f` and its recorded recursive
  submodule commits.
- Linux is the only host platform; the client is Apple Silicon macOS 15 or
  newer.
- The connection is direct hostname/IP on the internal LAN or VPN.
- TLS certificate verification stays enabled. Test identities, default
  credentials, and verification bypasses are forbidden.
- Start with hardware H.264 4:2:0 NV12. HEVC 4:2:0 is the next codec after
  first pixels.
- No audio, virtual display, Wayland, Wacom fidelity, control server, or
  polished UI is required for this spike.
- HEVC 4:4:4, P010, and AV1 stay unavailable until independently proven and
  must not block the first-pixels result.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | kyber-live-h264 | standard | Given the pinned Kyber stack, the Linux X11/NVIDIA host, and the Apple Silicon Mac, when the client connects directly with verified TLS and requests H.264 4:2:0, then the Mac displays the live `DP-0.3` desktop | PENDING | kyber, kymux, nvfbc, nvenc, h264, macos |
