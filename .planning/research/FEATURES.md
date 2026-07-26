# Feature Research

**Domain:** High-performance Linux X11-to-macOS remote desktop prototype
**Researched:** 2026-07-26
**Confidence:** MEDIUM

## Scope Boundary

This is a proof-oriented prototype, not a general remote-access product. “Launch”
below means the first internally usable build that proves one physical NVIDIA/X11
host can be controlled from one arm64 Mac over a directly reachable LAN or VPN
address. A feature is only complete when its actual behavior is observable in the
client or exported session report; a configured value is not evidence that the
corresponding path ran.

## Feature Landscape

### Table Stakes (Required for the Prototype)

| Feature | Why Expected | Complexity | Exact Observable Behavior |
|---------|--------------|------------|---------------------------|
| Direct IP/hostname connection | The requested topology has no discovery or control service | MEDIUM | Client accepts a hostname or IP plus optional port, shows `resolving → connecting → authenticating → probing → streaming`, permits cancellation, and distinguishes DNS failure, timeout, refusal, TLS failure, and authentication rejection. |
| Authenticated, pinned transport | Keyboard and clipboard control make unauthenticated access unacceptable | HIGH | QUIC/TLS verifies the entered hostname/IP against a trusted certificate or an explicitly provisioned certificate/public-key pin. A mismatch hard-fails before input or clipboard channels open. No trust-all mode, Kyber test certificate, embedded default password, or silent re-pin is allowed. |
| Host/client preflight | Unsupported hardware and sessions must fail before a misleading black window | HIGH | Host reports X11 session, selected physical output and mode, NVIDIA device/driver, capture backend, NVENC capabilities, `/dev/uinput`, and render access. Client reports arm64/macOS version, decoder candidates, renderer, and Input Monitoring status. Each failed check names the component and remediation. |
| Physical X11 desktop capture | Controlling the already-running desktop is the product premise | HIGH | Host captures one explicitly configured physical X11 output. Startup reports its XRandR name, dimensions, refresh, capture backend, and whether any GPU→CPU or cross-GPU copy occurs. Wayland, missing output, capture denial, or an NVIDIA driver/userspace mismatch stops startup. |
| NVIDIA hardware encode | A software encoder would invalidate the latency proof | HIGH | Every session reports GPU PCI/device identity, NVENC generation, encoder name, input pixel format, copy count, codec/profile/chroma/bit depth, rate-control settings, and actual average bitrate. Failure to create or run NVENC aborts; there is no CPU encode fallback. |
| `Auto` plus exact manual codec policy | Hardware varies across Ampere/Ada and Apple Silicon | HIGH | Choices are `Auto`, HEVC 4:4:4, HEVC 4:2:0, AV1 4:2:0 when proven, and H.264 4:2:0. Codec and chroma are one coupled choice. A manual request either starts that exact format or fails; it never substitutes another format. |
| Runtime decoder-path proof | Model-name capability tables do not prove a specific bitstream path | HIGH | Client creates and decodes a representative stream matching resolution/profile/chroma/bit depth, then reports `VideoToolbox hardware`, another named hardware path, or `software`. `Auto` accepts only a proven hardware path. Manual HEVC 4:4:4 may use software only after a measured probe and an explicit warning; it does not satisfy the hardware 4K60 gate. |
| Sustained 3840×2160 at 60 fps | This is the first viability gate | HIGH | A fixed moving workload runs after warm-up for 30 minutes. Captured, encoded, received, decoded, and presented counts, pacing, drops, and latency distributions must meet the gate below. The result is an exported PASS/FAIL report, not a subjective “looks smooth” judgment. |
| Correct desktop presentation | A stream that is cropped, stretched, or color-mis-signaled is not usable | MEDIUM | Decoded raster is exactly 3840×2160; 1:1 presentation is used for the fidelity gate on a 4K60 display. Windowed scaling preserves aspect ratio and maps input through the displayed content rectangle. SDR range/matrix and actual chroma are reported; no HDR or gamut conversion is implied. |
| Immersive mouse control | Pointer escape or incorrect coordinates breaks remote control | HIGH | Fullscreen visibly enters captured state, hides/confines the local cursor, forwards relative deltas without edge clipping, supports absolute mapping, buttons and wheel, and has a documented local release chord. Focus loss or disconnect releases all buttons. Absolute coordinates exclude letterbox bars and clamp to the configured host output. |
| Immersive keyboard control | A desktop cannot be operated reliably with partial key forwarding | HIGH | Native macOS acquisition sends physical key down/up, repeat, modifiers, left/right variants, navigation and function keys. Input permission denial is detected before capture. Focus loss/disconnect synthesizes releases for all held keys. A tested per-macOS shortcut matrix labels each chord `remote`, `local-only`, or `release chord`; the client never claims interception it cannot perform. |
| Bidirectional text/HTML clipboard | Copy/paste is core desktop behavior | HIGH | macOS general NSPasteboard and X11 `CLIPBOARD` (not `PRIMARY`) synchronize UTF-8 plain text and HTML in both directions while connected. A change normally arrives within 500 ms. Both representations are preserved when present; no other MIME types cross. |
| Bounded, loop-free clipboard behavior | Clipboard ownership is asynchronous and naive mirroring can bounce forever | MEDIUM | Maximum combined serialized payload is **1 MiB** per update. Oversize or malformed content is rejected without truncation or changing the destination clipboard. Origin ID, content hash, and host-assigned sequence suppress echoes; simultaneous changes resolve by host observation order. Disconnect leaves both local clipboards intact. |
| Deterministic teardown and recovery | Stuck input and hidden failures make a prototype unsafe to use | MEDIUM | User can disconnect explicitly. Teardown restores pointer state, releases remote keys/buttons, closes clipboard ownership cleanly, and writes the final report. Decoder loss requests/restarts from a clean intra frame and either recovers within 1 second or terminates with a reason. Reconnect is manual for the first proof. |
| Actionable diagnostics | Optimization is impossible without knowing which stage failed | HIGH | Live overlay and JSON report show requested/actual resolution and fps; codec/profile/chroma/bit depth; encoder/decoder and hardware/software status; copy path; bitrate; queue depth; capture/convert/encode/network/decode/render timings; RTT/jitter/loss; counters and drop reasons; host/client versions, OS, driver, GPU, display mode, Kyber commit and patch ID. |
| Explicit compatibility matrix | “Linux” and “Mac” are otherwise untestably broad | HIGH | arm64-only client launches on macOS 15 and each supported major through the current macOS 27 beta. Host build/start/session smoke tests cover current representatives of Arch-, Ubuntu-, and Rocky-derived X11 systems with NVIDIA. The full performance gate may use the reference CachyOS/Arch host first; compatibility claims remain unmade until their row passes. |

### Differentiators (Why This Prototype Is Worth Building)

| Feature | Value Proposition | Complexity | Exact Observable Behavior |
|---------|-------------------|------------|---------------------------|
| Evidence-backed negotiation | Prevents the common “requested codec” versus “actual fallback” ambiguity | HIGH | Selection is the intersection of a live NVENC encode probe, an exact client decode probe, renderer readiness, and the policy. The selection explanation and rejected candidates appear in the session report. |
| First-class HEVC 4:4:4 fidelity mode | Preserves colored text, fine UI edges, and design-workstation detail lost by 4:2:0 | HIGH | Bitstream inspection confirms HEVC `chroma_format_idc=3`; decoded planes retain full chroma resolution; a 1-pixel chroma stress chart is recorded. The overlay always says HEVC 4:4:4 and hardware/software decode explicitly. |
| Honest experimental AV1 | Uses Ada/M3-class hardware only where the complete path is real | HIGH | AV1 is offered only as 4:2:0 after host encode, transport, client decode, render, and 4K60 probes pass. On Ampere or an unproven client it is disabled/fails with the exact failed probe. It is never chosen merely because a model family advertises AV1. |
| Stage-level latency provenance | Turns optimization into an engineering result rather than a feeling | HIGH | Correlated frame IDs and monotonic timestamps separate capture, conversion, encode, transport, decode, queue, render-submit, and input injection. Percentiles and clock-offset uncertainty are exported. Unmeasured compositor/display/device delay is labeled rather than folded into a misleading total. |
| Reproducible proof bundle | Makes performance regressions and AGPL obligations auditable | MEDIUM | Each report identifies Kyber Desktop 0.27.0 commit `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`, exact submodules, local patch set, build inputs, runtime configuration, licenses, and test workload. Another machine can rebuild the same binaries/configuration. |
| Compatibility without product bloat | Proves the requested OS range while avoiding an installer/account detour | MEDIUM | A compact matrix records build, launch, connection, video, input, clipboard and security results per supported host family and macOS release. Failures stay visible; there is no claim of universal Linux/macOS support. |

### Anti-Features (Explicitly Excluded)

| Feature | Why Tempting | Why Problematic Now | Prototype Alternative |
|---------|--------------|---------------------|-----------------------|
| Wayland host | Modern Linux default | Adds portal/compositor/capture and input-injection problems unrelated to this X11 proof | Refuse startup without a real Xorg session and explain how to select X11. |
| Audio | Makes sessions feel complete | Adds capture, synchronization, buffering and device-routing work before video is proven | Run video/input/clipboard only; keep audio disabled in configuration and UI. |
| Virtual/headless desktop or display driver | Convenient unattended access | Changes capture, EDID, session lifecycle and privilege scope | Control one already-running physical X11 output. |
| Discovery, control server, relay, STUN/TURN, NAT traversal | Easier connections | Creates a separate control plane and security/operations product | Operator enters a directly reachable LAN/VPN IP or hostname. |
| Accounts, device lists, fleet/admin UI | Familiar remote-access UX | Does not validate the media/data plane | Provision one host certificate/pin and unique credential out of band. |
| Polished settings UI, installer, updater | Easier distribution | Consumes prototype effort and obscures reproducibility | CLI-launched host and minimal native client with a small connection/policy surface. |
| Windows/Linux clients, non-NVIDIA hosts, macOS/Windows hosts, Intel Macs | Larger market | Multiplies backend and packaging matrices | Enforce NVIDIA Linux X11 host and arm64 macOS 15+ client. |
| Wacom pressure/tilt/eraser/tool/pad fidelity | Valuable creative-workstation feature | Requires new macOS acquisition, protocol and Linux virtual-tablet models | Send ordinary pointer movement/buttons; schedule Wacom only after the core proof. |
| Clipboard files, images or arbitrary binary types | Expected in mature products | Increases attack surface, memory use and platform conversion complexity | Transfer only UTF-8 text and HTML, bounded to 1 MiB total. |
| X11 `PRIMARY` synchronization | Feels like fuller Linux integration | Selection changes would overwrite the Mac clipboard unexpectedly | Synchronize explicit-copy `CLIPBOARD` only. |
| Silent codec/chroma/software fallback | Avoids session-start errors | Invalidates every latency/quality claim and may request impossible combinations | Exact manual mode or clear failure; `Auto` explains its proven choice. |
| Independent unchecked codec and chroma toggles | Looks flexible | Permits impossible requests such as AV1 4:4:4 on Ada | Expose only valid coupled combinations. |
| AV1 4:4:4 | Sounds like maximum quality/efficiency | Ada NVENC exposes AV1 4:2:0, not AV1 4:4:4 | Use HEVC 4:4:4 for fidelity or AV1 4:2:0 where proven. |
| Adaptive resolution/bitrate or opaque “quality” mode | Smooths variable networks | Hides whether the fixed 4K60 target was actually sustained | Use a recorded fixed test profile; expose bitrate as an explicit diagnostic/test parameter. |
| HDR, wide gamut, 10-bit acceptance, 1440p120 or 4K120 | Attractive next-step quality/performance claims | Adds color-management and higher-refresh variables before the 4K60 SDR baseline exists | Prove 8-bit SDR 4K60 first; retain these as later measurement gates. |
| Multi-monitor panorama or hot switching | Host has multiple displays | Expands coordinate, resolution and capture lifecycle complexity | Choose one physical X11 output in host configuration before connecting. |
| Automatic reconnect | Seems resilient | Can hide crashes, repeat authentication, and complicate input/clipboard ownership during proof collection | End cleanly with a reason and let the operator reconnect manually. |
| Software encode or silent software decode | Broadens compatibility | Violates the NVIDIA hardware-path premise and can mask an unusable latency path | Never software-encode; allow measured software decode only for an explicitly chosen HEVC 4:4:4 experiment. |

## Observable Acceptance Contracts

### 4K60 and Latency Gate

These are recommended prototype acceptance budgets, not validated current
performance. Run on a wired direct LAN with a 4K60-capable client display,
one configured 3840×2160 X11 output, 60 seconds of warm-up, then 30 minutes of
continuous motion, scrolling text, fine chroma patterns, and pointer input.
VPN observations are recorded separately and do not replace the LAN baseline.

| Measure | PASS Threshold | Failure Meaning |
|---------|----------------|-----------------|
| Actual raster and mode | Host mode, captured frames and decoded frames are exactly 3840×2160; source and client presentation are 60 Hz or better | Scaling or configuration masquerading as 4K60 |
| Sustained throughput | Every rolling 10-second window presents at least 59.0 fps after warm-up | Pipeline cannot consistently sustain the target |
| Total dropped/late frames | ≤0.1% after warm-up, with every drop assigned to capture, encode, transport, decode, or presentation | Smoothness claim lacks evidence or a stage is unstable |
| Capture + convert + encode | p95 <16.67 ms and p99 <25 ms | Host misses the 60 fps frame budget |
| Decode | p95 <16.67 ms; actual hardware/software path recorded | Client cannot keep pace or is using an unexpected decoder |
| Decode-to-present queue | No configured multi-frame video buffer; depth ≤1 and p95 wait <16.67 ms | Latency is being hidden in buffering |
| App-instrumented capture-ready → render-submit | p95 ≤50 ms, p99 ≤67 ms | End-to-end software pipeline is not consistently low latency |
| Client input event → Linux injection | p95 ≤10 ms on the reference LAN | Control path is perceptibly delayed before host application response |
| External input-to-photon audit | p95 ≤67 ms with high-speed camera or equivalent; methodology and uncertainty recorded | Internal clocks omit meaningful compositor/display/device latency |
| Fidelity mode identity | HEVC SPS/decoder output proves 4:4:4 and the 1-pixel chroma stress chart passes without 4:2:0 substitution | “4:4:4” label is false or conversion lost chroma |

Any missed threshold produces FAIL plus the offending percentile/counter.
Changing resolution, fps, codec, chroma, bitrate, buffering, or hardware creates
a distinct test run; results cannot be combined.

### Codec Negotiation Contract

| Policy | Required Result |
|--------|-----------------|
| `Auto` | Probe all valid pairs, reject software decode/encode, and choose a fully hardware path that passed the exact 4K sample. Until equivalent measurements justify another order, use HEVC 4:2:0, then proven AV1 4:2:0, then H.264 4:2:0. Report every candidate and rejection reason. |
| Manual HEVC 4:4:4 | Require host HEVC 4:4:4 encode. Probe the exact client stream and label hardware or measured software. Never substitute 4:2:0. |
| Manual HEVC 4:2:0 | Require exact hardware NVENC and exact client decode; otherwise fail. |
| Manual AV1 4:2:0 | Require Ada-or-newer AV1 NVENC plus a working, measured macOS playback path; Apple family advertising alone is insufficient. Mark experimental until its 4K60 gate passes. |
| Manual H.264 4:2:0 | Exact hardware fallback for endpoints without a proven HEVC/AV1 path. |
| AV1 4:4:4 | Never offer or accept. Return `CODEC_PAIR_INVALID` and state that supported AV1 NVENC is 4:2:0. |

### Minimum Failure Vocabulary

All fatal messages contain a stable code, failed stage, requested versus
observed values, one actionable next step, and a session ID. Logs may include
diagnostic details but must not include credentials or clipboard contents.

| Code | User-Visible Message Requirement |
|------|----------------------------------|
| `CONNECTION_RESOLVE_FAILED` / `CONNECTION_TIMEOUT` / `CONNECTION_REFUSED` | Name the address/port and distinguish resolver, timeout, and refusal. |
| `TLS_IDENTITY_MISMATCH` | Show expected identity/pin and observed fingerprint; do not offer “continue anyway.” |
| `AUTHENTICATION_REJECTED` | State that transport succeeded but authorization failed; never echo the secret. |
| `HOST_X11_REQUIRED` | Report detected session/display state and require a real Xorg login. |
| `HOST_NVIDIA_DRIVER_MISMATCH` / `HOST_NVENC_UNAVAILABLE` / `HOST_CAPTURE_UNAVAILABLE` | Name the GPU, driver/API versions and failed probe; suggest the specific reboot, permission, or configuration check. |
| `CODEC_PAIR_INVALID` / `CODEC_HOST_UNSUPPORTED` / `CODEC_CLIENT_UNSUPPORTED` | Show the exact requested codec/chroma/profile and which endpoint rejected it. |
| `HARDWARE_DECODE_UNAVAILABLE` | State the decoder that was attempted and whether software was found; `Auto` must stop or choose another proven pair, never silently use it. |
| `INPUT_PERMISSION_REQUIRED` | Name the missing macOS privacy permission, give the System Settings location, and leave input visibly disabled. |
| `CLIPBOARD_TOO_LARGE` / `CLIPBOARD_TYPE_UNSUPPORTED` | State byte limit or allowed types; leave destination clipboard unchanged. |
| `PLATFORM_UNSUPPORTED` | Show architecture/OS/distro/session and the supported prototype matrix. |

## Feature Dependencies

```text
Pinned Kyber 0.27.0 + exact submodules
    └──> reproducible build/patch manifest
            └──> trustworthy compatibility and performance reports

Host X11/NVIDIA/capture/NVENC probes
Client macOS/decoder/render probes
    └──> codec+chroma capability intersection
            ├──> Auto policy
            └──> exact manual modes
                    └──> 4K60/latency/fidelity gate

Frame IDs + monotonic stage timestamps
    └──> live diagnostics + JSON evidence
            └──> optimization and PASS/FAIL decision

macOS input permission + native event capture
    └──> pointer lock + keyboard forwarding
            └──> stuck-state cleanup and shortcut matrix

NSPasteboard change tracking + X11 CLIPBOARD ownership
    └──> bounded text/HTML wire format
            └──> bidirectional loop-free synchronization
```

### Dependency Notes

- **Security precedes input and clipboard:** no control-bearing channel opens
  until certificate/pin and client authorization succeed.
- **Runtime probes precede codec menus:** static GPU/Mac model tables can seed
  candidates but cannot mark a pair usable.
- **Instrumentation precedes optimization:** without correlated counters and
  timestamps, a 4K60 claim cannot identify the limiting stage.
- **HEVC 4:2:0/H.264 establish the baseline before AV1:** the pinned macOS
  libVLC path is not yet proven for AV1.
- **Core input precedes Wacom:** tablet fidelity depends on a new acquisition,
  protocol, and Linux virtual-device model, not a small mouse extension.

## MVP Definition

### Launch With (First Validated Prototype)

- [ ] Pinned/reproducible Kyber fork, unique credentials, verified certificate/pin.
- [ ] Direct hostname/IP connection and explicit host/client preflight.
- [ ] One physical NVIDIA/X11 output through measured capture, conversion and NVENC.
- [ ] Exact `Auto`, HEVC 4:4:4, HEVC 4:2:0, proven AV1 4:2:0, and H.264 4:2:0 behavior.
- [ ] 30-minute 4K60 gate with stage telemetry, JSON report, fidelity chart and external latency audit.
- [ ] Fullscreen relative/absolute mouse and keyboard forwarding with permission/shortcut disclosure and clean release.
- [ ] Bidirectional, loop-free 1 MiB text/HTML clipboard synchronization.
- [ ] Reference Arch-class proof plus recorded compatibility rows for Ubuntu-, Rocky-, and supported macOS versions.

### Add After Core Validation (Next Milestones)

- [ ] Faithful Wacom pressure/tilt/eraser/tool/pad transport — only after video,
  ordinary input and clipboard pass.
- [ ] 1440p120 and 4K120 measurement gates — only after the same 4K60 report is
  reproducible on more than one run.
- [ ] AV1 promotion within `Auto` — only if end-to-end hardware AV1 repeatedly
  beats or materially reduces bandwidth versus HEVC without worse gate results.

### Future Product Consideration, Not Prototype Backlog

- Audio, virtual displays, Wayland, discovery/relay/NAT traversal, accounts,
  device/fleet management, installers/updaters, additional host/client
  platforms, arbitrary clipboard/file transfer, multi-monitor UX, HDR and
  adaptive quality.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Secure direct connection + preflight | HIGH | HIGH | P1 |
| NVIDIA/X11 capture and NVENC proof | HIGH | HIGH | P1 |
| Stage instrumentation and report | HIGH | HIGH | P1 |
| HEVC 4:2:0 + H.264 baseline | HIGH | MEDIUM | P1 |
| Sustained 4K60/latency gate | HIGH | HIGH | P1 |
| Immersive mouse/keyboard | HIGH | HIGH | P1 |
| Text/HTML clipboard | HIGH | HIGH | P1 |
| HEVC 4:4:4 fidelity mode | HIGH | HIGH | P1 |
| AV1 4:2:0 experimental path | MEDIUM | HIGH | P1 when proven, otherwise visibly unavailable |
| Cross-distro/cross-macOS matrix | MEDIUM | HIGH | P1 before claiming compatibility |
| Wacom fidelity | HIGH for creative work | HIGH | P2 |
| Higher-refresh gates | MEDIUM | HIGH | P2 |
| Product/control-plane features | LOW for this proof | HIGH | P3 / excluded |

**Priority key:** P1 validates this prototype; P2 follows a successful core
proof; P3 is future product work and must not enter the first roadmap.

## Reference Product Analysis

| Capability | Moonlight/Sunshine | Parsec | LinuxRemote Prototype |
|------------|--------------------|--------|-----------------------|
| Hardware codecs | H.264, HEVC, AV1; hardware decode; 4:4:4 with Sunshine | H.264/HEVC; 4:4:4 on supported paid configurations | Kyber/Kymux with exact coupled modes and per-session proof |
| Input modes | Pointer capture and direct mouse control; shortcut forwarding | Mature remote keyboard/mouse | Native macOS immersive input, but only tested shortcut claims |
| Diagnostics | Network/decode/queue/render latency and classified drops | Encode/decode/network, bitrate, resolution, chroma, decoder | Adds capture/conversion/encode/transport/decode/render correlation and reproducible report |
| 4:4:4 positioning | Available capability | Creative-workstation differentiator | First-class HEVC fidelity gate with bitstream/output verification |
| Fallback behavior | Broad compatibility focus | May revert codec/decode paths by endpoint | Exact manual choice or explicit failure; `Auto` only selects proven hardware |
| Product breadth | Discovery/pairing, games, many clients | Accounts, teams/admin, multi-screen, Wacom | Deliberately one direct host/client proof |

## Sources

All web findings are **MEDIUM confidence**: the research seam classified
`websearch` as LOW and raised cross-checked primary-source findings to MEDIUM.
Project scope and acceptance budgets are local design decisions and still
require live validation.

- [Kyber Desktop repository and 0.27.0 documentation](https://gitlab.com/kyber/apps/kyber-desktop)
- [NVIDIA Video Encode and Decode Support Matrix](https://developer.nvidia.com/video-encode-decode-support-matrix)
- [NVIDIA Video Codec SDK](https://developer.nvidia.com/video-codec-sdk)
- [Apple: Require hardware-accelerated VideoToolbox decoding](https://developer.apple.com/documentation/videotoolbox/kvtvideodecoderspecification_requirehardwareacceleratedvideodecoder)
- [Apple: Query whether VideoToolbox is using hardware decode](https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_usinghardwareacceleratedvideodecoder)
- [Apple M3 announcement: first Mac media-engine AV1 decode claim](https://www.apple.com/newsroom/2023/10/apple-unveils-m3-m3-pro-and-m3-max-the-most-advanced-chips-for-a-personal-computer/)
- [Apple CGEvent and event taps](https://developer.apple.com/documentation/coregraphics/cgevent)
- [Apple Input Monitoring privacy control](https://support.apple.com/guide/mac-help/control-access-to-input-monitoring-on-mac-mchl4cedafb6/mac)
- [Apple NSPasteboard](https://developer.apple.com/documentation/appkit/nspasteboard)
- [freedesktop.org X clipboard behavior](https://specifications.freedesktop.org/clipboard/latest/)
- [X.Org ICCCM selection protocol](https://www.x.org/releases/current/doc/xorg-docs/icccm/icccm.pdf)
- [RFC 9001: TLS security and peer authentication for QUIC](https://www.rfc-editor.org/rfc/rfc9001.html)
- [RFC 9525: Service identity verification](https://www.rfc-editor.org/rfc/rfc9525.html)
- [Moonlight performance-stat definitions](https://github.com/moonlight-stream/moonlight-docs/wiki/Frequently-Asked-Questions)
- [Moonlight PC feature baseline](https://github.com/moonlight-stream/moonlight-qt)
- [Parsec stream overlay, stats and logging](https://support.parsec.app/hc/en-us/articles/32381603663636-Stream-Overlay-Stats-and-Logging)
- [Parsec hardware/software and 4:4:4 compatibility](https://support.parsec.app/hc/en-us/articles/32381568346644-Hardware-and-Software-Compatibility)

---
*Feature research for: LinuxRemote*
*Researched: 2026-07-26*
