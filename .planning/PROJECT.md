# LinuxRemote

## What This Is

LinuxRemote is a prototype-first, high-performance remote desktop system for
controlling a physical Linux X11 desktop from an Apple Silicon Mac. It uses
Kyber/Kymux as the direct peer-to-peer media and input foundation, with NVIDIA
capture and NVENC on the host, and prioritizes proving a sharp, responsive
4K60 experience before investing in product UI, account infrastructure, or
cross-platform hosting.

The prototype is for internal use on directly reachable LAN or VPN hosts. It
must expose honest codec choices and measured behavior rather than silently
falling back to lower-quality or software paths.

## Core Value

Prove that a Linux-to-macOS Kyber pipeline can deliver a visually excellent,
consistently low-latency 4K60 physical-desktop session with immersive control
and working clipboard synchronization.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] A user can start a Linux X11 host and connect from an Apple Silicon Mac
      by entering a directly reachable IP address or hostname.
- [ ] The prototype sustains a measurable 3840×2160 at 60 Hz streaming session
      on supported hardware without audio or a virtual desktop.
- [ ] The Linux host uses an NVIDIA zero/minimal-copy capture path and NVENC;
      capture, conversion, encode, transport, decode, render, and input timing
      are instrumented independently.
- [ ] The macOS client provides an `Auto` codec policy plus explicit codec and
      chroma choices. Initial choices include HEVC 4:4:4, HEVC 4:2:0, AV1
      4:2:0 where both endpoints prove support, and H.264 4:2:0 fallback.
- [ ] Unsupported codec/chroma combinations fail with a clear explanation.
      AV1 4:4:4 must never be offered on Ada NVENC hardware.
- [ ] `Auto` prioritizes fully hardware-accelerated end-to-end operation and
      selects only combinations proven by runtime encoder and decoder probes.
- [ ] A fidelity mode exposes HEVC 4:4:4, while reporting whether macOS decode
      is hardware or software and refusing an unmeasured silent fallback.
- [ ] Fullscreen immersive mode locks the pointer, forwards relative and
      absolute mouse input, captures eligible keyboard input through native
      macOS APIs, and clearly identifies shortcuts macOS does not permit the
      app to intercept.
- [ ] Bidirectional clipboard synchronization works between macOS NSPasteboard
      and the Linux X11 clipboard for text and HTML within an explicit size
      limit.
- [ ] The prototype uses authenticated TLS with certificate
      verification/pinning and does not ship Kyber's test certificate or
      default credentials.
- [ ] The macOS client is arm64-only, supports macOS 15 Sequoia and newer, and
      is continuously checked against the current macOS 27 Golden Gate beta.
- [ ] The Linux prototype is usable on X11 installations derived from Arch,
      Ubuntu, and Rocky Linux, with the first live proof allowed on the current
      CachyOS/Arch-class development host.
- [ ] The Kyber base and every local patch are pinned and reproducible, with
      licenses and source obligations documented for internal use and possible
      later open-source publication.

### Out of Scope

- Wayland hosting — explicitly unnecessary for the prototype; use a real Xorg
  session.
- Audio capture or playback — deferred until the video/control proof succeeds.
- Virtual desktops or headless virtual monitors — the prototype controls an
  already-running physical desktop.
- Control server, discovery service, relay, STUN/TURN, or automatic NAT
  traversal — direct LAN/VPN reachability is sufficient.
- Windows or Linux clients and macOS/Windows hosts — future product scope, not
  part of this proof.
- A polished GUI, accounts, device lists, installers, auto-update, or fleet
  management — a CLI-launched host and minimal client are acceptable.
- Wacom pressure, tilt, eraser, tool identity, and pad controls — ordinary
  pointer fallback is acceptable until the core stream is proven; faithful
  tablet transport is the next input milestone.
- Clipboard file transfer, images, and arbitrary binary formats — prototype
  clipboard scope is text and HTML.
- Support for Intel Macs or macOS releases older than macOS 15 Sequoia.
- Claiming HEVC 4:4:4 hardware decode on Apple Silicon without a successful
  VideoToolbox runtime probe on the actual client.

## Context

- Kyber Desktop 0.27.0 is open source and already provides the closest viable
  base: a Rust control/input stack, Kymux over QUIC/TLS, FFmpeg-based capture
  and encoding, libVLC-based playback, Linux host support, macOS arm64
  packaging, direct hostname connections, mouse/keyboard forwarding, and
  Linux X11 clipboard endpoints.
- The prototype should pin the Kyber Desktop 0.27.0 superproject at commit
  `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f` and preserve its exact submodule
  revisions before applying narrowly scoped patches.
- Upstream currently has three relevant gaps:
  1. its macOS input pipeline does not bridge NSPasteboard clipboard events;
  2. its macOS window layer reports keyboard grab as unsupported, leaving
     immersive keyboard behavior incomplete; and
  3. its input protocol has no tablet/Wacom event model.
- NVIDIA's current capability matrix allows HEVC 4:4:4 on Ampere and Ada but
  only AV1 Main-profile 4:2:0 on Ada. Codec and chroma therefore must be
  negotiated as a pair rather than exposed as independent unchecked toggles.
- Apple Silicon generations differ: advertised AV1 hardware decode starts with
  M3. VideoToolbox does not publicly guarantee HEVC Main 4:4:4 hardware
  decode, so the client must probe the exact bitstream and report the actual
  decode path.
- Upstream Kyber's pinned libVLC currently lacks a proven VideoToolbox AV1
  decode path on macOS. AV1 remains an experimental selectable path until
  patched and measured, not the default solely because the host is Ada.
- The first performance gate is 4K60. Higher-refresh targets such as 1440p120
  and 4K120 are future measurement gates and must not compromise the first
  reproducible proof.
- The current development host has an RTX A5000 Laptop GPU (Ampere), two
  physical 4K displays including one 120 Hz display, and adequate development
  tooling. It is presently running a Wayland session and has a loaded
  NVIDIA-kernel/userspace version mismatch; a reboot into a matching installed
  kernel followed by an X11 login is required before live NVENC/NvFBC tests.
- `/dev/uinput` and DRM render access are already available to the current user
  through ACLs. NVENC must be re-probed after the driver/session correction.
- Internal use and possible later open-source release make AGPLv3 acceptable
  for the prototype. A future proprietary distribution would require a Kyber
  commercial license plus a fresh third-party codec and dependency review.

## Constraints

- **Protocol**: Build on current Kyber/Kymux rather than inventing a new media
  transport — the prototype exists to validate and tune this stack.
- **Topology**: One directly reachable Linux host and one macOS client — no
  service-mediated discovery or NAT traversal.
- **Host OS**: Linux X11 only for the prototype — Wayland is deliberately
  excluded.
- **Host GPU**: NVIDIA NVENC-capable hardware — AV1 requires Ada or newer;
  older supported NVIDIA generations use HEVC/H.264 according to queried caps.
- **Client**: Apple Silicon and macOS 15 Sequoia or newer — no x86_64 build.
- **Performance**: First proof at 4K60 with zero configured video buffering and
  explicit stage-level latency telemetry — subjective smoothness alone is not
  acceptance evidence.
- **Quality**: HEVC 4:4:4 is a first-class manual fidelity choice, but a fully
  hardware-decoded 4:2:0 mode may be the latency default.
- **Security**: Authenticated TLS and certificate verification from the first
  networked build — test certificates and default credentials are prohibited.
- **License**: Prototype changes may comply with AGPLv3 network-source
  obligations — keep patches, notices, and build inputs reproducible.
- **Scope**: No audio, virtual display, polished UI, control server, or faithful
  Wacom transport in the first streaming proof.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Fork Kyber Desktop 0.27.0 and preserve its pinned components | It already implements the high-risk media, QUIC, host, and client foundations; narrow patches are lower risk than a rewrite | — Pending |
| Use adaptive codec negotiation with a visible manual override | NVIDIA and Apple capabilities vary by generation, and unchecked codec/chroma flags can request impossible modes | — Pending |
| Treat HEVC 4:4:4 as a selectable fidelity mode | It satisfies the requested desktop sharpness on NVIDIA hosts while keeping hardware-decoded 4:2:0 available for latency | — Pending |
| Reject AV1 4:4:4 and offer AV1 only as 4:2:0 | Ada NVENC implements AV1 Main profile only | — Pending |
| Make 4K60 the first performance gate | It is demanding enough to prove viability while remaining a realistic first optimization target | — Pending |
| Target macOS 15 through macOS 27 beta, arm64 only | This matches the internal client fleet and removes legacy Intel/macOS compatibility work | — Pending |
| Keep topology to LAN/VPN direct IP or hostname | It proves the data plane without prematurely adding control-plane or NAT infrastructure | — Pending |
| Defer faithful Wacom support until after the core proof | Pressure/tilt requires new capture, protocol, and Linux virtual-tablet work; mouse fallback is acceptable initially | — Pending |
| Accept AGPLv3 for the internal prototype | Internal use and potential open sourcing align with the available Kyber license path | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `$gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `$gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-07-26 after initialization*
