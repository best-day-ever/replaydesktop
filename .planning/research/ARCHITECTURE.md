# Architecture Research

**Domain:** Direct, low-latency Linux X11/NVIDIA to Apple Silicon macOS remote desktop
**Researched:** 2026-07-26
**Confidence:** HIGH for the pinned Kyber source structure and existing data paths; MEDIUM for the proposed native macOS player until it is proven on target hardware

## Recommendation

Build LinuxRemote as a **thin, pinned Kyber distribution**, not as a new remote-desktop stack. Preserve Kyber's controller/process model, Kymux endpoint model, QUIC/TLS implementation, and input event protocol. Add four explicit seams around it:

1. A versioned control-plane capability contract and pure mode selector.
2. A runtime probe layer on both peers.
3. A macOS-native player/input/clipboard overlay.
4. A telemetry aggregator that extends Kyber's existing per-frame metrics.

The first runnable proof should use upstream's H.264 4:2:0/libVLC path. The target macOS path should then replace only the desktop **player adapter** with `AVPacket -> VideoToolbox -> CVPixelBuffer/IOSurface -> Metal -> CAMetalLayer`. This keeps a known-good diagnostic baseline while avoiding a VLC/Kymux rewrite. The pinned VLC tree does contain a VideoToolbox decoder, but the pinned decoder has no AV1 mapping and its Apple output is not an application-owned, directly measurable Metal pipeline; do not describe that baseline as the target native renderer.

## Standard Architecture

### System Overview

```text
                     Authenticated control plane (HTTPS/TLS)
┌──────────────────────── Linux X11 host ────────────────────────┐
│ kycontroller                                                     │
│  ├─ identity/auth + session state                                │
│  ├─ host capability probe/cache                                  │
│  ├─ selection validator                                          │
│  └─ subprocess lifecycle                                         │
│       │                                                           │
│       ├────────── spawn/configure ──────────┐                     │
│       v                                      v                     │
│  kyavserver / kyavservice               kynputservice             │
│  X11 display -> NvFBC -> GPU format      X11 clipboard            │
│  conversion -> NVENC -> AVPacket         uinput injection         │
│       │                                      ^                     │
│       │ video endpoint                       │ input endpoint       │
│       v                                      │                     │
│  kycom IPC -> kyproto -> kynet/Quinn QUIC/TLS                     │
└───────────────────────┬──────────────────────┬────────────────────┘
                        │ video + metrics       │ input + clipboard
                        │ host -> client         │ bidirectional
                        v                       v
┌──────────────────── Apple Silicon macOS client ──────────────────┐
│ kyclient session orchestrator                                     │
│  ├─ TLS pin/private-CA verification + authentication              │
│  ├─ client decoder probe + ModeSelector                           │
│  ├─ stage telemetry/clock correlation                             │
│  └─ lifecycle and explicit reconnect/re-negotiate                 │
│       │                                      ^                     │
│       v                                      │                     │
│ VideoClientEndpoint                      InputEndpoint             │
│       │                                      │                     │
│ AVPacket parser                         native input adapter        │
│       │                                  NSEvent/CGEvent tap        │
│ codec config + access units             pointer confinement        │
│       v                                      │                     │
│ VTDecompressionSession                      │                     │
│       │ CVPixelBuffer / IOSurface             │                     │
│       v                                      v                     │
│ Metal texture import -> color shader     NSPasteboard adapter      │
│ -> drawable present on CAMetalLayer      text/html, loop guard     │
└───────────────────────────────────────────────────────────────────┘
```

The control plane chooses and validates a media tuple before allocating Kymux endpoints. The data plane remains Kyber's existing typed endpoints over one authenticated QUIC/TLS connection. Media never travels through the new capability code, and capability JSON never changes Kymux framing.

### Component Responsibilities

| Component | Owns | Must not own |
|-----------|------|--------------|
| `kycontroller` | Authentication, direct session lifecycle, capability response, authoritative selection validation, subprocess start/stop | Codec ranking policy, decoding, packet framing |
| `HostProbe` (new) | Actual X11/NvFBC/NVENC runtime probes; tuple availability and rejection evidence; short-lived cache keyed by GPU/driver/display | Client assumptions based on GPU model names |
| `kyavservice` + `txproto` | Display enumeration, capture, conversion/filter graph, FFmpeg/NVENC encoder, encoded packet production | Client policy or transport changes |
| `kymux` (`kyproto`, `kynet`, `kycom`, `kymux-types`) | Typed endpoints, authentication token, QUIC/TLS streams/datagrams, packet sequencing and IPC bridge | Capability negotiation or fallback policy |
| `kyclient` orchestrator | TLS identity, authentication, probe execution, mode selection, session start, reconnect/re-negotiate, UI/event-loop integration | Host capability fabrication |
| `ModeSelector` (new, pure library) | Intersection and deterministic ranking of host encoder tuples and client decoder tuples | Performing probes or silently changing a manual selection |
| `MacNativePlayer` (new) | `AVPacket` receive, codec configuration, VideoToolbox session, CVPixelBuffer lifetime, Metal rendering, decoder-path evidence | QUIC internals or session negotiation |
| `kynput` | Canonical input events, Kymux input endpoint, Linux injection, X11 clipboard type | macOS window policy |
| `MacInputAdapter` (new) | Focus/fullscreen state, pointer lock, relative/absolute translation, event tap, permitted/unpermitted shortcut reporting, release-all on focus loss | Linux injection details |
| `MacClipboardAdapter` (new) | NSPasteboard watch/read/write, text/HTML mapping, byte limit, origin/hash loop suppression | File/image/binary transfer |
| `TelemetryAggregator` (new) | Per-frame stage correlation, clock-domain normalization, JSONL output, percentiles and acceptance report | Blocking the media path |

## Upstream Pin and Fork Strategy

Use exact source commits, not version strings alone:

| Repository | Pinned commit |
|------------|---------------|
| `kyber/apps/kyber-desktop` | `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f` |
| `kyber/deps/winit` | `73cc88800dffb4e385e853c553fbe8341a2c6771` |
| `kyber/core/kysdk` | `5836202aa4654cf64b5ca9fd204005fe4916be99` |
| `kyber/core/kyctl` | `e46dda2825d6e8c432521cfcb4c5726b5bba8bc4` |
| `kyber/core/kymedia` | `e80eb6bb347ed0e378ae46a86f67ada2aa6079da` |
| `kyber/core/kymux` | `831d3120bea004505dc69426eebbb9b07f10e443` |
| `kyber/core/kynput` | `5595478f3606c5624197360e2dcbf31bd60e8163` |
| `kyber/core/kyutil` | `5704c85dc53e50b16341916aca29cf26cba327c4` |
| `kyber/deps/txproto` | `82694c38fb7d662ad364071382166edf8731db93` |
| `kyber/deps/vlc` | `dd2db54794384591684c1c86ce70eb64eb9eab15` |

Recommended Git policy:

- Mirror/fork the desktop superproject and keep an immutable `upstream/0.27.0` ref at the exact commit.
- Fork only subrepositories that receive code changes: initially `kyctl`, `kymedia` only if the probe/extra timestamp cannot live outside it, and `kynput`. Each LinuxRemote branch starts directly at the pinned SHA.
- Update the corresponding gitlinks through `kysdk`, then update the `kysdk` gitlink in the desktop fork. Never rely on dirty submodules.
- Keep new independent code in overlay crates where possible. A small Kyber patch should select/invoke the overlay, not absorb its implementation.
- Generate `UPSTREAM.lock` with every upstream SHA, fork SHA, applied patch ID, Rust toolchain, Meson version, FFmpeg archive hash, and VLC SHA. Verify it in CI.
- Keep one logical change per commit/patch so the delta can be rebased or offered upstream.

### Recommended Project Structure

```text
linuxremote/
├── upstream/
│   └── kyber-desktop/              # fork/submodule, recursively pinned
├── crates/
│   ├── lr-capabilities/            # versioned contracts + ModeSelector
│   ├── lr-host-probe/              # X11/NvFBC/NVENC runtime probes
│   ├── lr-macos-player/            # AVPacket -> VideoToolbox -> Metal
│   ├── lr-macos-input/             # event tap, pointer lock, translation
│   ├── lr-macos-clipboard/         # NSPasteboard bridge
│   └── lr-telemetry/               # normalized stage records/reports
├── config/
│   ├── host.example.toml
│   └── codec-policy.toml
├── fixtures/
│   ├── bitstreams/                 # tiny H.264/HEVC/AV1 probe vectors
│   ├── avpacket/                   # deterministic packet replays
│   └── certificates/               # test-only, never packaged
├── tests/
│   ├── contract/
│   ├── loopback/
│   ├── macos/
│   └── hardware/
├── packaging/
│   ├── linux/
│   └── macos/
├── scripts/
│   ├── verify-upstream-lock
│   ├── probe-host
│   ├── probe-client
│   └── run-4k60-gate
└── UPSTREAM.lock
```

## Modification Policy

### Intended narrow patch areas

| Upstream area | Permitted change |
|---------------|------------------|
| Desktop `kyclient` | CLI/config, macOS event-loop integration, player selection, user-visible failures |
| `kyctl/kyclient` | Versioned capability structs, session request/response, player trait/adapter seam, telemetry forwarding |
| `kyctl/kycontroller` | Authenticated media-capability endpoint, validation, typed failure response, single-client policy |
| `kymedia/kyavservice` | Invoke host probe and pass an already validated encoder configuration |
| `txproto` | Only a narrowly documented timestamp/surface hook if conversion timing cannot be observed externally |
| `kynput` macOS pipeline | Add clipboard producer/consumer and native input producer; reuse existing event types |

### Do not modify for this milestone

- `kymux`, `kyproto`, `kynet`, `kycom`, and `kymux-types`: their existing video, metrics, and bidirectional input endpoints are sufficient.
- QUIC congestion/stream framing or TLS cryptography.
- `kyutil/libkypc` process IPC.
- FFmpeg patches, NVENC codec implementation, or the VLC fork unless a minimal, independently reproduced upstream defect blocks the native-player path.
- The winit fork: macOS event taps and pasteboard integration belong in an adapter beside winit.
- Windows/web/Android/iOS paths, audio code, Wayland code, virtual display/Kidd, gamepad, or Wacom event models.

This boundary is important: a native player may consume `VideoClientEndpoint` directly and eliminate the client-side Kycom/TCP hop, but that is a `kyclient` adapter change, not a Kymux protocol change. Keep the existing Kycom + libVLC path behind a diagnostic build feature until the native path passes parity tests.

## Capability and Session Contract

Treat codec, chroma, bit depth, profile, resolution, frame rate, encoder path, and decoder path as **one tuple**:

```rust
struct MediaTuple {
    codec: Codec,              // h264 | hevc | av1
    chroma: Chroma,            // c420 | c444
    bit_depth: u8,
    profile: String,
    width: u32,
    height: u32,
    fps: u32,
}

struct ModeEvidence {
    tuple: MediaTuple,
    available: bool,
    path: AccelerationPath,    // hardware | software
    backend: String,           // nvenc, videotoolbox, ffmpeg-software
    probe_id: String,
    reason: Option<ReasonCode>,
}
```

Protocol sequence:

1. Establish HTTPS/TLS, verify the hostname/private CA or pinned SHA-256 fingerprint, and authenticate.
2. Fetch authenticated `MediaCapabilitiesV1`. Host evidence comes from real NvFBC/NVENC session probes, not a GPU-name table.
3. Run/retrieve the client VideoToolbox probe. `VTIsHardwareDecodeSupported` is only a coarse first filter; create a session with representative codec configuration, require hardware for hardware claims, decode a sample, and record the actual path.
4. `ModeSelector` intersects exact tuples. `Auto` ranks: proven hardware end-to-end first, then configured fidelity/latency preference. It never considers AV1 4:4:4 and never invents an unprobed tuple.
5. POST `StartSessionV1 { requested_policy, selected_tuple, host_probe_id, client_probe_id }`.
6. The host revalidates current capability, then allocates endpoints/spawns the encoder. Return the accepted tuple and evidence. Unsupported requests return a structured `422` reason before media allocation, not a bare `403`.
7. The first `AVPacket::Codec` and codec parameter sets must agree with the accepted tuple. Mismatch is a protocol/session error: stop, report, and re-probe.

Manual selection is strict. `Auto` may try the next ranked tuple only by visibly tearing down and starting a new negotiated session. A hardware-to-software change is never an in-place fallback. HEVC 4:4:4 software decode may be offered only when a measured client probe says it works and the user explicitly allows software decode.

Suggested stable rejection codes include `HOST_CAPTURE_UNAVAILABLE`, `HOST_ENCODER_UNAVAILABLE`, `CLIENT_DECODER_UNAVAILABLE`, `PROFILE_UNSUPPORTED`, `CHROMA_UNSUPPORTED`, `RESOLUTION_OR_FPS_UNSUPPORTED`, `HARDWARE_REQUIRED`, `PROBE_STALE`, and `BITSTREAM_MISMATCH`.

## Data Flow

### Forward Video: Capture to Present

```text
X11 physical display
  -> NvFBC capture surface
     [frame id, capture-start/acquired timestamps]
  -> GPU color conversion / scale, only if selected tuple requires it
     [conversion-start/end, copy count, input/output format]
  -> FFmpeg NVENC context configured from accepted MediaTuple
     [encode-start/end, key/config flags, bytes]
  -> kymux_types::AVPacket
     Codec packet once/config change; Media packets with PTS/key/config
  -> Kycom/kyproto video endpoint
  -> Quinn QUIC/TLS
     [sent/received, RTT, loss/drop counters]
  -> Mac VideoClientEndpoint
  -> access-unit assembler + codec parameter-set parser
  -> CMVideoFormatDescription + VTDecompressionSession
     [decode-submit/callback, actual acceleration path]
  -> IOSurface-backed CVPixelBuffer
  -> CVMetalTextureCache plane textures
  -> Metal YUV/RGB shader, color metadata honored
  -> CAMetalLayer drawable present
     [prepared/command-buffer-complete/presented timestamps]
```

Do not claim zero-copy from configuration alone. The 4K60 report must state each copy/import boundary. NvFBC failure or an unexpected XCB/CPU path fails the performance gate unless the operator explicitly runs a non-acceptance diagnostic fallback.

### Reverse Input

```text
macOS focused/fullscreen window
  -> native NSEvent/CGEvent tap
  -> MacInputAdapter
       keyboard usage/modifiers
       relative delta while locked
       absolute normalized position while unlocked
       buttons/wheel
  -> existing Kynput canonical event
  -> existing bidirectional InputEndpoint over Kymux
  -> Linux kynputservice
  -> /dev/uinput virtual keyboard/mouse
  -> X11 applications
```

The adapter owns an explicit state machine: `unfocused`, `focused-windowed`, `focused-fullscreen`, `suspended`. On focus loss, Command-Tab, disconnect, sleep, or crash recovery, it releases the pointer, disables the event tap, sends release-all/reset state, and reports which macOS shortcuts cannot be intercepted. Absolute and relative events retain one canonical desktop-coordinate transform derived from the selected X11 display.

### Bidirectional Clipboard

```text
NSPasteboard.general changeCount
  -> read UTF-8 text and/or HTML
  -> normalize + enforce byte limit
  -> {origin_id, sequence, content_hash, mime, bytes}
  -> existing Kynput clipboard InputPacket / InputEndpoint
  -> X11 CLIPBOARD selection owner

X11 CLIPBOARD owner change follows the same path in reverse.
```

Keep a recently-applied `(origin_id, content_hash)` window so a remote write observed locally is not echoed back. Last accepted update wins. Oversize/unsupported content is rejected with a metric and visible log but does not close the session. File URLs, images, and arbitrary binary types are ignored.

### Telemetry

Kyber 0.27.0 already emits `acquired`, `encoding`, `encoded`, `sent`, `received`, `decoding`, `decoded`, `prepared`, `displayed/skipped`, plus network RTT/loss and clock-offset samples. Extend rather than replace this chain:

- Add conversion start/end and surface/copy metadata on the host.
- Add VideoToolbox submit/callback and Metal command-buffer/present timestamps on macOS.
- Add input capture, Kymux send/receive, Linux inject, clipboard observe/apply timestamps.
- Correlate video by `(source_id, pts)` and input/clipboard by a monotonic sequence ID.
- Keep metrics on their existing low-priority endpoint/channel; a full telemetry queue drops telemetry and increments `telemetry_dropped`, never backpressures video or input.
- Write raw append-only JSONL plus a derived run summary with p50/p95/p99, achieved frame cadence, dropped/skipped frames, selected tuple, exact probe evidence, and copy path.

## Failure and Fallback Model

| Failure | Required behavior |
|---------|-------------------|
| Certificate mismatch/unknown strict host | Fail before credentials/session data; show expected and actual identity; no bypass in packaged build |
| Authentication failure | Close control-plane attempt; never open Kymux |
| Wayland session, NvFBC unavailable, driver/userspace mismatch | Preflight fails with actionable reason; XCB may be an explicit diagnostic mode but cannot satisfy the NVIDIA 4K60 gate |
| Explicit tuple unsupported | `422` with tuple and reason; no encoder/player spawned |
| `Auto` first choice fails during creation | Tear down partial resources, record failure, select next already-probed hardware tuple, and announce the change |
| Manual choice fails | Stop; never change codec/chroma/acceleration path |
| Decoder reports software when hardware was selected | Abort the session as evidence mismatch; do not continue silently |
| Codec packet/parameter sets disagree with handshake | Abort and invalidate probe/cache |
| QUIC loss/disconnect | Release input immediately; reconnect control plane, repeat capability validation, create a fresh session, request a fresh keyframe |
| Metrics clock sync missing | Continue video but mark cross-host latency unavailable; never fabricate end-to-end latency |
| Event tap disabled or accessibility permission absent | Exit immersive state, release keys/pointer, explain permission/shortcut limitation |
| Clipboard oversize/unsupported/invalid HTML | Drop that update, count/report it, preserve session |
| Host helper crash | Controller reaps all endpoints/processes; a restart always creates a new negotiated session |

## Security Boundaries

- Ship a unique host certificate and key with restrictive file permissions, or provision a small private CA. Never package `test/tls/kybertest_*`.
- Default client policy should be explicit fingerprint/private-CA verification. System roots are acceptable only when the host certificate has a valid name chain. TOFU auto-accept is not an acceptable unattended default.
- Remove or compile-gate `--tls-skip-verification` from the production app surface.
- Replace Kyber's sample Basic login and development JWT key. For the one-user prototype, a high-entropy per-host credential stored outside the TOML file is sufficient; a control server/OIDC deployment is not.
- Bind only the configured direct interface/port; use one-client mode; reject a second active session.
- Authenticate before returning detailed device capabilities and before creating QUIC tokens.
- Put explicit maximum sizes on capability JSON, AV access-unit assembly, clipboard payloads, and metric queues.
- Treat all peer-provided lengths, codec parameters, HTML, and reason strings as untrusted. Validate before allocation/rendering/logging.
- Kymux already uses QUIC/TLS; do not add a second custom encryption layer.

## Configuration and CLI Surface

Keep the surface small and make effective choices printable as JSON.

Host configuration:

```toml
[linuxremote]
listen = "0.0.0.0:8080"
single_client = true
display = 0
capture = "nvfbc"
allow_capture_fallback = false
video_buffer_ms = 0
codec_allowlist = ["h264-420", "hevc-420", "hevc-444", "av1-420"]
clipboard = true
clipboard_max_bytes = 1048576
metrics = true

[linuxremote.tls]
cert = "/etc/linuxremote/host.pem"
key = "/etc/linuxremote/host-key.pem"
```

Client surface:

```text
linuxremote connect HOST
  --tls-host NAME --tls-fingerprint-file PATH
  --codec auto|h264|hevc|av1
  --chroma auto|420|444
  --allow-software-decode=false
  --display 0 --video-buffer-ms 0
  --fullscreen --clipboard=true
  --metrics-out RUN.jsonl

linuxremote probe HOST --json
linuxremote known-hosts add|remove|list
```

`probe --json`, the startup banner, and every metrics run must show the requested policy, selected tuple, capture backend, encoder backend, decoder backend, whether each side is hardware accelerated, and every rejected higher-ranked candidate. Audio, virtual display, discovery/relay, Wayland, and Wacom flags should not appear.

## Packaging Boundary

Linux:

- Build only the X11/NVIDIA host/controller and required Kyber runtime into a versioned rootfs/tar package.
- Add a preflight command for X11, matching NVIDIA driver/userspace, NvFBC/NVENC availability, `/dev/uinput`, render-device access, certificate/key permissions, and UDP reachability.
- Keep distro-specific dependency installation outside the core package; verify Arch/CachyOS first, then Ubuntu and Rocky in containers/VM build jobs plus real-hardware smoke tests.

macOS:

- Use `build-macos.sh -p -a arm64` as the upstream packaging base, set the deployment target to macOS 15, and produce arm64 only.
- Bundle the native player library and compiled Metal shaders. Keep libVLC only while the diagnostic fallback is useful.
- Add required Info.plist usage text and a first-run accessibility/input-monitoring check for the event tap. Losing permission must degrade safely.
- Codesign the whole app consistently; run CI against the supported macOS baseline and a current macOS 27 beta runner when available.

Both packages should embed `UPSTREAM.lock`, license notices, source/patch retrieval instructions, a machine-readable build manifest, and an SBOM. Packaging must fail if a test certificate/default credential or an uncommitted submodule is detected.

## Test Seams

| Seam | Test |
|------|------|
| Capability contract | Golden JSON for schema versioning and unknown-field compatibility |
| `ModeSelector` | Exhaustive tuple table, especially AV1 4:4:4 rejection, hardware-first Auto, and no manual fallback |
| Probe adapters | Fake NVENC/VideoToolbox providers; stale evidence and creation/decode failure injection |
| Encoder/player boundary | Recorded `AVPacket` codec/config/media fixtures for H.264, HEVC, and AV1 |
| Native player | Decode tiny vectors; assert actual acceleration property, pixel format, frame order, and Metal present callback |
| Kymux | Existing loopback tests plus loss/disconnect/reconnect; no wire-format delta |
| Input state machine | Focus/fullscreen transitions, modifier release, absolute/relative transform, forbidden shortcut reporting |
| Clipboard | Text/HTML round trip, loop suppression, size limit, malformed HTML, rapid ownership changes |
| Telemetry | Out-of-order events, missing clock sync, dropped frames, queue saturation, percentile calculation |
| Security | Wrong pin/name/CA, changed certificate, replayed/expired token, second-client rejection, test-artifact packaging scan |
| Hardware acceptance | Static high-detail desktop pattern at 3840x2160/60, 10+ minute run, stage percentiles, frame cadence/drop rate, exact selected tuple and copy path |

Use three increasingly expensive environments:

1. Hermetic unit/fixture tests on every commit.
2. Linux/macOS loopback and packet replay in CI.
3. Scheduled/phase-gate tests on the real X11 NVIDIA host and each relevant Apple Silicon generation.

## Dependency-Driven Vertical Build Order

Each phase must leave one runnable end-to-end slice.

1. **Pinned, secure upstream baseline**
   - Materialize every exact gitlink, build Linux and arm64 macOS packages, remove test identity/default credentials, and connect by direct hostname/IP.
   - Run upstream H.264 4:2:0, no audio, existing mouse/keyboard, zero configured video buffering.
   - Gate: authenticated pixels and control cross the real LAN/VPN; build is reproducible from `UPSTREAM.lock`.

2. **Measured NVIDIA 4K60 baseline**
   - Correct the development host's driver/session state, require X11 + NvFBC + NVENC, enable Kyber's existing stage metrics, and add the missing conversion/copy evidence.
   - Keep H.264 4:2:0 and libVLC to isolate capture/encode/transport throughput from new codec/player work.
   - Gate: 3840x2160/60 reaches the client with an honest stage report; no XCB/software fallback is accepted.

3. **Capability contract and strict selector**
   - Add host/client probe interfaces, `MediaCapabilitiesV1`, `ModeSelector`, structured rejection, and first-packet conformance check.
   - Initially advertise only the already-proven H.264 tuple; then add tuples as probes pass.
   - Gate: every impossible tuple fails before endpoint allocation and Auto produces a deterministic, explainable result.

4. **Native macOS H.264 player**
   - Introduce the player adapter seam and direct `VideoClientEndpoint` consumer. Implement H.264 4:2:0 VideoToolbox decode, IOSurface/CVMetalTexture import, Metal color conversion, and present telemetry.
   - Retain libVLC as a diagnostic feature, not a silent runtime fallback.
   - Gate: native path matches baseline stability, proves the reported decoder path, and passes 4K60.

5. **Negotiated codec/fidelity matrix**
   - Add HEVC 4:2:0, HEVC 4:4:4, AV1 4:2:0, and H.264 fallback one tuple at a time. AV1 requires an Ada host and a client probe that actually decodes it. AV1 4:4:4 remains unrepresentable.
   - Gate: matrix tests cover selection, bitstream conformance, hardware/software labeling, explicit failure, image fidelity, and latency. Auto defaults only to fully hardware-proven tuples.

6. **Native immersive input**
   - Add the macOS event-tap adapter and focus/fullscreen state machine around existing Kynput events; instrument capture-to-inject latency.
   - Gate: relative/absolute mouse, buttons/wheel, keyboard/modifiers, pointer release, disconnect cleanup, and shortcut limitations are reproducible.

7. **Native bidirectional clipboard**
   - Add NSPasteboard and X11 text/HTML adapters on the existing input endpoint with limits and loop suppression.
   - Gate: both directions, rapid updates, malformed/oversize payloads, reconnect, and no echo loop.

8. **Cross-version packaging and soak**
   - Harden preflight, effective-config output, source/license bundle, arm64 app signing, distro builds, macOS 15/current 27-beta checks, fault injection, and long 4K60 runs.
   - Gate: one-command host/client launch and an archived acceptance report tied to exact binaries and hardware.

The ordering deliberately proves the existing transport and NVIDIA data path before introducing a decoder rewrite, then proves one native codec before multiplying the codec matrix. Input and clipboard reuse the already-stable authenticated session and therefore follow the data-plane proof.

## Anti-Patterns

### Editing Kymux to Carry Capabilities

**Why it fails:** It couples policy to the data plane, creates a new wire compatibility burden, and is unnecessary because authenticated HTTP session negotiation already exists.

**Instead:** Negotiate/validate tuples in `kyctl`; keep existing typed Kymux endpoints unchanged.

### Independent Codec and Chroma Toggles

**Why it fails:** It creates invalid combinations such as AV1 4:4:4 on Ada and cannot express profile/bit-depth/acceleration evidence.

**Instead:** Select an atomic `MediaTuple`.

### Silent Software or Capture Fallback

**Why it fails:** The session may look functional while invalidating the latency/quality claim.

**Instead:** Change paths only through a new visible negotiation with measured evidence.

### Patching VLC Before Isolating the Player Boundary

**Why it fails:** It mixes a large downstream multimedia fork with product-specific macOS behavior and makes AV1/render telemetry hard to reason about.

**Instead:** Keep libVLC as the baseline and implement a small native player against the existing `AVPacket` endpoint.

### One Monolithic “Latency” Timer

**Why it fails:** It cannot distinguish capture, conversion, encode, queueing, network, decode, and display regressions or clock-domain errors.

**Instead:** Correlate stage events and report missing evidence explicitly.

## Sources

Primary source inspection used the exact pinned commits listed above.

- [Kyber Desktop 0.27.0 pinned tree](https://gitlab.com/kyber/apps/kyber-desktop/-/tree/6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f) — HIGH
- [Kyber control plane/client tree](https://gitlab.com/kyber/core/kyctl/-/tree/e46dda2825d6e8c432521cfcb4c5726b5bba8bc4) — HIGH
- [Kyber media tree](https://gitlab.com/kyber/core/kymedia/-/tree/e80eb6bb347ed0e378ae46a86f67ada2aa6079da) — HIGH
- [Kymux pinned tree and architecture](https://gitlab.com/kyber/core/kymux/-/tree/831d3120bea004505dc69426eebbb9b07f10e443) — HIGH
- [Kynput pinned tree](https://gitlab.com/kyber/core/kynput/-/tree/5595478f3606c5624197360e2dcbf31bd60e8163) — HIGH
- [NVIDIA NVENC Application Note](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-application-note/index.html) — HIGH
- [NVIDIA NVENC API Programming Guide](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-video-encoder-api-prog-guide/index.html) — HIGH
- [Apple `VTIsHardwareDecodeSupported`](https://developer.apple.com/documentation/videotoolbox/vtishardwaredecodesupported%28_%3A%29) — HIGH
- [Apple `NSPasteboard.changeCount`](https://developer.apple.com/documentation/appkit/nspasteboard/changecount) — HIGH
- [Apple `CGEvent`](https://developer.apple.com/documentation/coregraphics/cgevent) — HIGH

## Open Research Flags

- Phase 2 must verify whether NvFBC-to-NVENC is truly zero-copy in this exact Kyber/driver build and identify where conversion occurs; source configuration alone is insufficient.
- Phase 4 needs a focused native-player spike to validate direct endpoint ownership, parameter-set conversion, drawable timing semantics, and 4:4:4 CVPixelBuffer/Metal formats.
- Phase 5 needs real device tests for HEVC 4:4:4 and AV1 on each Apple Silicon generation. Public codec-level hardware support is not enough to claim tuple-level hardware decode.
- Phase 6 must enumerate macOS 15 and macOS 27-beta shortcut interception behavior and permissions empirically.

---
*Architecture research for: LinuxRemote*
*Researched: 2026-07-26*
