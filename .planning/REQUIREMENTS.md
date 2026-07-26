# Requirements: ReplayDesktop

**Defined:** 2026-07-26
**Core Value:** Prove that a Linux-to-macOS Kyber pipeline can deliver a
visually excellent, consistently low-latency 4K60 physical-desktop session
with immersive control and working clipboard synchronization.

## v1 Prototype Requirements

### Reproducible Foundation

- [ ] **BASE-01**: An operator can recreate the prototype from Kyber Desktop
  0.27.0 commit `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`,
  its exact recursive gitlinks, locked build tools, and recorded source hashes.
- [ ] **BASE-02**: Every local Kyber modification is recorded as a narrow,
  independently reviewable patch with its owning component, purpose, and
  upstream-base revision.
- [ ] **BASE-03**: Every benchmark report identifies the complete source,
  patch, compiler, dependency, build-option, and runtime-configuration inputs
  that produced the tested binaries.
- [ ] **BASE-04**: The repository provides license notices, corresponding-source
  instructions, and an SPDX-compatible dependency/SBOM artifact sufficient for
  the accepted AGPL internal-use and possible open-source path.

### Direct Connection and Security

- [ ] **CONN-01**: A user can connect by entering a directly reachable hostname
  or IP address and an optional port, without discovery, relay, or a control
  service.
- [ ] **CONN-02**: The client displays cancellable `resolving`, `connecting`,
  `authenticating`, `probing`, and `streaming` states and distinguishes DNS
  failure, timeout, refusal, TLS identity failure, and authentication rejection.
- [ ] **CONN-03**: Each host uses a unique non-test TLS identity and a unique
  authentication secret; no Kyber sample key, certificate, or default
  credential is present in a build or package.
- [ ] **CONN-04**: The client verifies an explicitly provisioned certificate
  fingerprint or private CA and hard-fails an identity mismatch without a
  trust-all, continue-anyway, silent-TOFU, or silent-repin path.
- [ ] **CONN-05**: Input, clipboard, media-capability details, and media
  endpoints become available only after authentication, and explicit
  disconnect closes them with a stable reason and no automatic reconnect.

### Linux Host Readiness and Capture

- [ ] **HOST-01**: An operator can run a host-doctor command that reports and
  blocks on a non-Xorg session, NVIDIA kernel/userspace mismatch, failed NVML,
  missing physical output, missing `/dev/uinput`, or missing DRM/render access.
- [ ] **HOST-02**: An operator can select exactly one physical X11 output, and
  startup reports its XRandR name, dimensions, refresh rate, GPU, and desktop
  origin.
- [ ] **HOST-03**: The host proves creation of the selected NvFBC shared-CUDA
  capture path and reports every GPU-to-GPU, GPU-to-CPU, and cross-GPU copy
  between scanout capture and the encoder.
- [ ] **HOST-04**: The host queries and successfully probes each advertised
  NVENC codec/profile/chroma/bit-depth tuple on the actual GPU and driver before
  offering it to a client.
- [ ] **HOST-05**: A session aborts if NvFBC or NVENC cannot run the negotiated
  path; XCB capture and software encoding are diagnostic-only and never silent
  performance fallbacks.
- [ ] **HOST-06**: Host startup identifies the selected capture backend, GPU PCI
  identity, driver/API versions, NVENC generation, input pixel format,
  conversion path, codec settings, and expected bitrate controls.

### Codec Negotiation and Presentation

- [ ] **CODE-01**: The control plane represents codec, profile, chroma,
  bit depth, resolution, frame rate, encoder path, and decoder path as one
  bounded, versioned media tuple rather than independent unchecked flags.
- [ ] **CODE-02**: `Auto` evaluates the intersection of live host encode,
  exact client decode, renderer, and resolution/frame-rate probes and selects
  only a fully hardware-proven tuple while reporting every rejected candidate.
- [ ] **CODE-03**: A user can manually request HEVC 4:4:4, HEVC 4:2:0,
  AV1 4:2:0, or H.264 4:2:0, and the session either runs that exact tuple or
  fails without substitution.
- [ ] **CODE-04**: AV1 4:4:4 is never advertised or accepted and returns a
  stable `CODEC_PAIR_INVALID` explanation before media allocation.
- [ ] **CODE-05**: The host and client revalidate cached capability evidence
  against the current GPU, driver, OS, display, application build, and exact
  representative bitstream when a session is created.
- [ ] **CODE-06**: The client reports the actual decoder implementation,
  hardware/software status, decoded pixel format/chroma, output surface type,
  copy path, and presentation renderer for the negotiated stream.
- [ ] **CODE-07**: Performance modes require a hardware decoder; manual HEVC
  4:4:4 may use a measured software decoder only after an explicit warning and
  can satisfy fidelity evidence but not the hardware-latency gate.
- [ ] **CODE-08**: HEVC 4:4:4 fidelity evidence includes bitstream
  `chroma_format_idc=3`, full-resolution decoded chroma planes, and a recorded
  one-pixel chroma stress-chart result.
- [ ] **CODE-09**: Presentation preserves the exact 3840×2160 SDR raster,
  aspect ratio, declared range/matrix/chroma, and the content rectangle used
  for input mapping without claiming HDR or wide-gamut behavior.

### Performance Evidence and Diagnostics

- [ ] **PERF-01**: Every video frame carries a correlated identity and
  monotonic timestamps for capture, conversion, encode, send, receive, decode,
  presentation-queue, and render-submit boundaries.
- [ ] **PERF-02**: Input events carry correlated client-send and Linux-injection
  timing, and reports include clock-offset uncertainty rather than presenting
  unsynchronized timestamps as exact end-to-end latency.
- [ ] **PERF-03**: Video runs with no configured multi-frame buffer, a
  presentation queue depth of at most one, and explicit counters for every
  acquired, dropped, encoded, sent, received, decoded, and presented frame.
- [ ] **PERF-04**: After 60 seconds of warm-up, a 30-minute wired-LAN mixed
  motion/text/chroma workload presents at least 59.0 fps in every rolling
  10-second window and drops or presents late no more than 0.1% of frames.
- [ ] **PERF-05**: The 4K60 gate reports capture+convert+encode p95 below
  16.67 ms and p99 below 25 ms, decode p95 below 16.67 ms, and decode-to-present
  queue p95 below 16.67 ms.
- [ ] **PERF-06**: The instrumented capture-ready-to-render-submit path reports
  p95 at or below 50 ms and p99 at or below 67 ms, while client-input-to-Linux-
  injection reports p95 at or below 10 ms on the reference LAN.
- [ ] **PERF-07**: A high-speed-camera or equivalent external input-to-photon
  audit reports methodology and uncertainty and targets p95 at or below 67 ms;
  any unmeasured compositor, display, or device delay remains explicitly
  unmeasured.
- [ ] **PERF-08**: Each session exports a non-blocking raw JSONL trace and
  human-readable PASS/FAIL report containing percentiles, queue depths, drop
  reasons, bitrate, RTT/jitter/loss, requested/actual media tuple, actual
  hardware paths, platform identities, source revisions, and a session ID.

### Immersive Mouse and Keyboard

- [ ] **INPT-01**: Before immersive capture, the arm64 macOS client preflights
  and explains every required Accessibility/Input Monitoring permission and
  leaves input visibly disabled when permission is denied.
- [ ] **INPT-02**: Fullscreen immersive mode visibly locks and hides the local
  pointer, forwards relative deltas without edge clipping, and forwards
  buttons and wheel events with a documented local release chord.
- [ ] **INPT-03**: Absolute pointer events map through the displayed content
  rectangle to the selected host output, exclude letterbox bars, clamp to the
  output, and remain correct under Retina/window scaling.
- [ ] **INPT-04**: Native macOS acquisition forwards physical key down/up,
  repeat, modifiers with left/right variants, navigation keys, and function
  keys over a reliable ordered channel.
- [ ] **INPT-05**: A tested matrix for macOS 15 and the current macOS 27 beta
  labels relevant shortcuts as `remote`, `local-only`, or `release chord`; the
  client never claims interception macOS reserves.
- [ ] **INPT-06**: Focus loss, release chord, disconnect, client termination,
  or host-session loss restores local pointer state and releases every remotely
  held key and mouse button.

### Bidirectional Clipboard

- [ ] **CLIP-01**: While an authenticated session is connected, macOS general
  NSPasteboard and X11 `CLIPBOARD` synchronize UTF-8 plain text and HTML in both
  directions without synchronizing X11 `PRIMARY`.
- [ ] **CLIP-02**: A valid clipboard change normally reaches the opposite
  endpoint within 500 ms, and both text and HTML representations are preserved
  when both are present.
- [ ] **CLIP-03**: The initial protocol accepts at most 60 KiB of combined
  serialized clipboard content per update and rejects oversize, malformed, or
  unsupported MIME data without truncation or changing the destination.
- [ ] **CLIP-04**: Origin identity, content hash, and sequence/version handling
  suppress echo loops and deterministically resolve simultaneous observations
  without starving video or input.
- [ ] **CLIP-05**: Clipboard bridging starts only after authentication, never
  writes clipboard contents to logs or reports, stops cleanly on disconnect,
  and leaves both local clipboards intact.

### Platform Compatibility and Packaging

- [ ] **COMP-01**: The client builds and packages only for arm64 with a macOS
  15.0 deployment target; no x86_64 client artifact is produced.
- [ ] **COMP-02**: Build, launch, connection, video, input, clipboard, security,
  and capability-probe results are recorded for macOS 15 Sequoia, current
  stable supported macOS, and the current macOS 27 Golden Gate beta.
- [ ] **COMP-03**: Decoder claims are recorded per Apple Silicon generation,
  including at least one pre-M3 Mac and one M3-or-newer Mac before AV1 is
  considered broadly available.
- [ ] **COMP-04**: Reproducible host build/start/session-smoke instructions and
  results exist for current Arch/CachyOS-, Ubuntu 24.04-, and Rocky 9-derived
  X11/NVIDIA systems.
- [ ] **COMP-05**: The compatibility matrix claims only rows that passed on
  real hardware; container builds and the reference Arch-class 4K60 result do
  not imply runtime support for another distro or Mac generation.
- [ ] **COMP-06**: Packages and logs expose stable actionable errors for
  unsupported platform, X11 requirement, NVIDIA mismatch, capture/encoder/
  decoder failure, input permission, clipboard limit/type, and TLS/auth
  failure without exposing credentials or clipboard contents.

## Definition of Done

The v1 prototype is complete only when:

1. Every v1 requirement is implemented, tested, committed, and mapped to
   verification evidence.
2. Host readiness gate G0 and secure-baseline gate G1 pass before performance
   evidence is collected.
3. At least one fully hardware-accelerated tuple passes the complete 30-minute
   4K60 gate and external latency audit.
4. Manual HEVC 4:4:4 truthfully demonstrates fidelity and labels its actual
   macOS hardware/software decode path.
5. Immersive input and bidirectional clipboard pass lifecycle, permission,
   malformed-input, and disconnect tests.
6. The supported platform matrix and proof bundle are reproducible by another
   internal developer.
7. Automated checks pass, required manual hardware checks are archived, and no
   open high-severity security finding remains.

## v2 Requirements

Deferred until the v1 protocol and 4K60 proof pass.

### Tablet Input

- **TABL-01**: User can forward Wacom pen proximity, absolute coordinates,
  pressure, tilt, eraser/tool identity, barrel buttons, and pad controls from
  macOS to a correctly classified Linux virtual tablet.
- **TABL-02**: Tablet transport preserves ordering and latency telemetry and is
  verified in representative creative applications without duplicate mouse
  events.

### Higher Refresh and Media

- **RATE-01**: User can run separately measured 2560×1440@120 and
  3840×2160@120 performance gates on proven hardware.
- **RATE-02**: `Auto` can promote AV1 4:2:0 only after repeated end-to-end
  hardware results beat or materially reduce bandwidth versus HEVC without
  worse latency/recovery.
- **AUDI-01**: User can capture and play synchronized low-latency audio with
  explicit device selection and latency telemetry.
- **COLR-01**: User can select a separately verified HDR/10-bit/wide-gamut
  pipeline without weakening the v1 SDR behavior.

### Product Connectivity and Platforms

- **CTRL-01**: User can discover and authorize hosts through a control service
  without placing video/input traffic on the service data plane.
- **NAT-01**: User can establish peer-to-peer sessions across NAT with explicit
  relay fallback and equivalent authentication.
- **PLAT-01**: User can host on macOS and Windows with native hardware capture,
  encoding, and input injection.
- **PLAT-02**: User can connect from Windows and Linux clients with equivalent
  decoder, renderer, immersive-input, and clipboard behavior.
- **WAYL-01**: User can host a real Wayland desktop through compositor-approved
  capture and input APIs with a separately measured performance contract.
- **VDES-01**: User can create and reconnect to a headless or virtual desktop.
- **PROD-01**: User can use a polished signed installer, connection UI, device
  list, updater, and fleet-management controls.

## Out of Scope

Explicit exclusions for the v1 prototype.

| Feature | Reason |
|---------|--------|
| AV1 4:4:4 | Ada NVENC does not implement it; the tuple must be rejected |
| Intel Macs or macOS older than 15 | User selected Apple Silicon and Sequoia as the minimum |
| Software encoding | Invalidates the NVIDIA high-performance premise |
| Silent codec, chroma, capture, encode, or decode fallback | Makes quality and latency evidence untrustworthy |
| Wayland hosting in v1 | User explicitly accepts X11-only; Wayland is a separate future backend |
| Audio, virtual displays, control service, NAT traversal, or polished product UI in v1 | They do not prove the requested core data plane |
| Binary/file/image clipboard in v1 | Larger parsing and data-exposure surface; text/HTML is sufficient |
| X11 `PRIMARY` clipboard synchronization | Selection changes would unexpectedly overwrite the Mac clipboard |
| Multi-monitor panorama or hot switching | One configured physical output keeps capture and coordinates deterministic |
| Automatic reconnect | Can hide crashes and complicate input/clipboard ownership during proof collection |
| Wacom fidelity in v1 | Requires a new end-to-end tablet protocol; ordinary pointer fallback is accepted |

## Traceability

Populated during roadmap creation. Every v1 requirement must map to exactly one
phase.

| Requirement | Phase | Status |
|-------------|-------|--------|
| (Pending roadmap generation) | — | Pending |

**Coverage:**
- v1 requirements: 49 total
- Mapped to phases: 0
- Unmapped: 49 ⚠️

---
*Requirements defined: 2026-07-26*
*Last updated: 2026-07-26 after initial definition*
