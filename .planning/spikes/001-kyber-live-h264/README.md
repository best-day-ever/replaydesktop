---
spike: 001
name: kyber-live-h264
type: standard
validates: "Given the pinned Kyber stack, the Linux X11/NVIDIA host, and the Apple Silicon Mac, when the client connects directly with verified TLS and requests H.264 4:2:0, then the Mac displays the live DP-0.3 desktop"
verdict: PENDING
related: []
tags: [kyber, kymux, nvfbc, nvenc, h264, macos]
---

# Spike 001: Kyber Live H.264

## What This Validates

Given Kyber Desktop 0.27.0 with its recursive dependency lock, the current
Linux X11/NVIDIA host, and the Apple Silicon Mac at `100.119.34.79`, when the
macOS client connects directly using verified TLS and requests hardware H.264
4:2:0, then it displays the physical `DP-0.3` desktop as a live stream.

## Research

The official Kyber Desktop 0.27.0 repository already provides the relevant
vertical slice: an X11/Linux controller, FFmpeg/NVENC media server, Kymux data
plane, macOS arm64 client, and libVLC playback integration. Its documented
entry points are `build-linux.sh`, `run_kycontroller.sh`,
`build-macos.sh -p -a arm64`, and the packaged macOS application.

| Approach | Tool/library | Pros | Cons | Status |
|----------|--------------|------|------|--------|
| Build pinned Kyber Desktop | Kyber 0.27.0, Kymux, txproto, FFmpeg, libVLC | Existing end-to-end host, transport, input, clipboard, and macOS client | Large native dependency build; upstream defaults must be security-audited | Chosen |
| Extend `replay-host-doctor` into a streamer | Custom Rust/CUDA/QUIC work | Full control over every stage | Reimplements transport, packetization, decoder, client, input, and clipboard before proving Kyber | Rejected for spike |
| Switch to Sunshine/Moonlight | Sunshine, Moonlight | Mature and fast route to pixels | Does not validate the user-selected Kyber foundation | Rejected |

**Chosen approach:** build the exact Kyber 0.27.0 superproject and use its
existing direct Linux-to-macOS flow. ReplayDesktop's host doctor remains a
diagnostic preflight; it is not promoted into a second streaming stack.

Primary references:

- Official Kyber Desktop repository and 0.27.0 build instructions:
  <https://gitlab.com/kyber/apps/kyber-desktop>
- Official Kyber project overview and platform support:
  <https://gitlab.com/kyber/kyber>
- Kyber Desktop 0.27.0 changelog:
  <https://gitlab.com/kyber/apps/kyber-desktop/-/blob/6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f/CHANGELOG.md>

## How to Run

The current client-only artifact is available from the private
`v0.1.0-spike.1` GitHub prerelease as
`ReplayDesktop-arm64-spike.zip`. It is Apple Silicon-only, requires macOS 15
or newer, is ad-hoc signed for internal testing, and is not notarized.

The full operator commands will be finalized after the Linux build passes.
The intended operator flow is:

1. Build the Linux rootfs from the pinned superproject.
2. Provision a per-host certificate and explicit client trust.
3. Launch the Linux controller against X11 output `DP-0.3`, with audio off and
   H.264 4:2:0 selected.
4. Build/package the arm64 macOS client remotely.
5. Launch the client against the host's direct VPN/LAN address.

## What to Expect

- The Linux controller remains running and reports one authenticated client.
- The Mac opens a native Kyber client window.
- The window shows the live `DP-0.3` desktop and visibly updates when the
  Linux desktop changes.
- Logs identify H.264/NVENC on the host and the selected client decoder.
- No certificate-verification override or test identity is used.

## Observability

The spike retains timestamped Linux controller, media server, Kymux, and
macOS client logs. The final evidence records exact source commits, build
toolchains, certificate fingerprints (never private keys), negotiated codec,
decoder path, first-frame time, and any failure point.

## Investigation Trail

1. The existing repository was audited before starting. It contains a real
   one-frame NvFBC/CUDA/NVENC host probe, but no continuous packet sender,
   Kyber/Kymux checkout, macOS source, decoder, or display path.
2. The first-pixels scope was reduced to H.264 NV12. HEVC, 4:4:4, AV1, Wacom,
   and exhaustive host-admission hardening are explicitly outside the blocking
   path.
3. Environment audits and the exact recursive Kyber checkout are in progress.
4. The patched macOS build completed successfully with Rust/Cargo 1.89.0 and
   `MACOSX_DEPLOYMENT_TARGET=15.0`. All 311 Mach-O files are arm64, the main
   executable records a 15.0 minimum OS, the ad-hoc signature verifies
   strictly, and the package contains no `kybertest` files.
5. The published client archive is 39,925,915 bytes with SHA-256
   `c7c277115e86ab29b8a6abd3120c45e74d0a36f48c973fa0bb19b58f7ad57f9d`.
6. The first Linux build reached the final native link graph but failed
   because the staged static `libz.a` was not position-independent. This is a
   host-build blocker, not a macOS client failure, and the live-stream verdict
   therefore remains pending.

## Results

**Verdict: PENDING**

The spike is complete only after live pixels are visually verified on the Mac
or a concrete upstream/environment limitation invalidates the selected path.
