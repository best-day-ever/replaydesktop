# Quick Task 260730-ruu: Truthful Linux-to-macOS Streaming Metrics — Research

**Researched:** 2026-07-30
**Domain:** Kyber/Kymux, NvFBC/NVENC, patched VLC/VideoToolbox, AppKit technical telemetry
**Confidence:** HIGH for the current code path and implementation seams; MEDIUM for final macOS behavior until exercised on the target Apple Silicon machine.

## Summary

The existing pipeline already transports correlated scalar timestamps from Linux capture through macOS display submission, but it does not yet prove actual chroma, decoder implementation, hardware-decode use, encoded bytes, scoped queue depth, or most drop reasons. Its last `displayed` event is only the return of VLC’s display callback; on the macOS sample-buffer output, the picture has been enqueued rather than proven visible or scanned out. The current GUI tails child output, `metrics.json`, and `kyclient.log` into one text view, while `metrics.json` is truncated at launch and written through a buffered writer without an explicit per-record flush. [VERIFIED: codebase]

Implement an additive telemetry contract at the native ownership points, normalize it once in the Rust client, and render only measured values in the existing AppKit launcher. Same-clock stage deltas are valid; cross-host one-way and end-to-end latency, downstream CoreAnimation queue depth, and physical presentation time must remain `null` with an explicit reason. Actual hardware decode must come from the real VideoToolbox session property, not the requested mode, codec, Apple model, or selected module name. [VERIFIED: codebase] [CITED: https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_usinghardwareacceleratedvideodecoder]

**Primary recommendation:** extend the current scalar metric callbacks for frame stages, gauges, counters, and stable enums; add one versioned native path-status callback for exact module/surface strings; persist append-only per-session JSONL; show requested versus actual state separately in the current technical GUI.

## Project Constraints (from AGENTS.md)

- Preserve the exact Kyber/Kymux/Kymedia/VLC/txproto pins and patch-based, reproducible build; do not substitute system FFmpeg/VLC or invent transport. [VERIFIED: AGENTS.md]
- Scope remains one directly reachable NVIDIA Linux X11 host and one Apple Silicon macOS 15+ client; Wayland, x86_64, audio, virtual display, control services, and a UI rewrite are out of scope. [VERIFIED: AGENTS.md]
- The proof target is 3840×2160 at 60 Hz with zero configured video buffering and explicit stage telemetry; silent software/capture/codec fallback is forbidden. [VERIFIED: AGENTS.md]
- HEVC 4:4:4 remains a manual fidelity mode; a hardware-decoded 4:2:0 path may be the latency default, but both must report what actually ran. [VERIFIED: AGENTS.md]
- Authenticated TLS and certificate verification remain mandatory for the networked product path, and AGPL source/build inputs must remain reproducible. Telemetry work must not weaken either boundary. [VERIFIED: AGENTS.md]
- Release packaging is arm64-only with deployment target 15.0, pinned Kyber payloads, and Xcode 26.6 for the shipping qualification lane; an existing artifact built with another Xcode version cannot inherit that qualification. [VERIFIED: AGENTS.md]

## Architectural Responsibility Map

| Capability | Primary tier | Secondary tier | Rationale |
|---|---|---|---|
| Capture/encode/send facts | Linux native media backend | Kymux metrics stream | txproto owns the actual NvFBC, FFmpeg, FIFO, and socket events. [VERIFIED: codebase] |
| Codec/chroma/decode/render facts | macOS VLC/native modules | Rust VLC bridge | VLC sees the received bitstream, chosen decoder, VideoToolbox session, decoded surface, and vout. [VERIFIED: codebase] |
| Correlation, rolling rates, evidence | macOS Rust client | AppKit launcher | One client-side normalizer can preserve raw inputs and expose a stable JSONL contract without changing transport semantics. [VERIFIED: codebase] |
| Technical presentation | Browser/client equivalent: AppKit launcher | Raw evidence viewer | `ReplayDesktopLauncher.swift` already owns the technical GUI and child/file tailing. [VERIFIED: codebase] |
| Build qualification | macOS package scripts plus Linux host build | Patch replay verifier | Native and Rust changes span nested pinned repositories and both platform artifacts. [VERIFIED: codebase] |

## Existing Metric Path and Exact Seams

The current path is:

`txproto key/i64 callback → txproto-rs → kyavservice MessagePack → Kymux MetricsPacket → kyclient parser → kyctl JSON producer → top-level metrics.json → AppKit tailer`. Client VLC metrics independently enter at `libvlc_metrics_cb → vlc-rs → kyvlcplayer → kyclient-common::Metric` before joining the same JSON producer. [VERIFIED: codebase]

| Fact/event | Current producer | Current consumer and truth boundary |
|---|---|---|
| `pts`, `acquired` | `kymedia/subprojects/txproto/src/iosys_nvfbc.c:694-788` | Emitted after NvFBC grab and CUDA device copy; no capture-start timestamp. FIFO-full only increments generic stats/logging. [VERIFIED: codebase] |
| `encoding`, `encoded` | `kymedia/subprojects/txproto/src/encode.c:703-778` | Brackets FFmpeg send/receive. Packet FIFO push is not checked here, so an encoded-frame drop reason is absent. [VERIFIED: codebase] |
| `sent` | `kymedia/subprojects/txproto/src/packet_sink.c:175-213` | Emitted after two blocking writes to the local Kymux socket; it is not remote receipt or physical network egress. [VERIFIED: codebase] |
| Host scalar transport | `txproto/.../metrics.h:24-35`, `txproto-rs/src/lib.rs:506-548`, `kyavservice/src/metrics.rs:27-76` | `kyclient/src/kymux_backend/metrics.rs:137-205` recognizes only `pts/acquired/encoding/encoded/sent`; a full 32-slot host metric channel drops batches without a loss counter. [VERIFIED: codebase] |
| `received` | `kymedia/subprojects/vlc/modules/access/kymux.c:245-310,392-407` | Timestamp is after the complete payload read. The same access module sees the actual Kymux codec header and bitstream. [VERIFIED: codebase] |
| `decoding`, `decoded` | `vlc/src/input/decoder.c:1548-1575,1784-1815` | `decoding` is before `pf_decode`; `decoded` is after `vout_PutPicture`, not the VideoToolbox output callback. [VERIFIED: codebase] |
| `skipped` | `vlc/src/video_output/pic_buf.c:52-85` | Means the prior picture in VLC’s one-slot zero-latency buffer was superseded; no reason or queue scope crosses the ABI. [VERIFIED: codebase] |
| `prepared`, `displayed` | `vlc/src/video_output/video_output.c:2032-2082` | Both follow vout callbacks. On `VLCSampleBufferDisplay.m:817-887,1043-1046`, rendering enqueues a sample and `display` is a no-op, so normalize `displayed` as `presentation_enqueued`. [VERIFIED: codebase] |
| Network/clock | `kyclient/src/kymux_backend/metrics.rs:34-135`; `kymux/kyproto/src/protocol/clock_sync.rs:32-180` | Reports RTT/loss/drop counters and averages three NTP-like offset/delay samples. Samples are not bound to frames and carry no dispersion/age threshold. [VERIFIED: codebase] |
| JSON/evidence | `kysdk/kyctl/kyclient/src/metrics.rs:23-242`; top-level `kyclient/src/metrics.rs:28-49` | Raw events exist; the consolidator is not wired by the top-level client. The active file is truncated and buffered. Unknown network values are encoded as `-1`. [VERIFIED: codebase] |
| GUI | `prototype/macos/ReplayDesktopLauncher.swift:643-666,1042-1118` | A 0.5-second timer tails stdout/stderr plus two files into one bounded transcript; there is no parsed metric model. [VERIFIED: codebase] |

## What Is Truthfully Measurable

| Requested fact | Implement now | Must remain unknown until added/proven |
|---|---|---|
| Actual codec/chroma | Emit codec from the received Kymux header and chroma only after H.264/HEVC/AV1 sequence data is parsed in `modules/access/kymux.c`; distinguish requested from actual. Existing VLC parsers expose the needed sequence-header data. [VERIFIED: codebase] | Chroma before a valid sequence header; never copy the launch selection into the “actual” field. |
| Decoder implementation/hardware | Emit selected decoder after module load; in `modules/codec/videotoolbox/decoder.c`, query `kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder` after the real session is created. Also emit actual decoded output fourcc/IOSurface presence from the VideoToolbox output callback. [VERIFIED: codebase] [CITED: https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_usinghardwareacceleratedvideodecoder] | Hardware status on property/query failure; do not infer it from `--hw-dec`, the decoder module, codec support, or Mac generation. |
| FPS/bitrate | Derive separate acquired/encoded/received/decoded/enqueued FPS over monotonic rolling windows. Add encoded payload bytes at `encode.c` or `packet_sink.c`, then sum `bytes × 8 / window`. [VERIFIED: codebase] | Wire bitrate including QUIC/TLS overhead; the configured target bitrate is not a measurement. |
| Drops with reasons | Emit stable reason codes at ownership points: capture FIFO full, encoded-packet FIFO failure, VideoToolbox flagged drop/error, VLC picture superseded, sample-layer-not-ready, and telemetry-channel loss. [VERIFIED: codebase] | Never reinterpret Kymux packet-loss counters as frame-drop reasons. |
| Queue depth | Emit scoped gauges from txproto FIFO size/capacity APIs and VLC `pic_buf` state; label them, for example, `host_capture_fifo`, `host_packet_fifo`, and `client_vlc_pic_buf`. [VERIFIED: codebase] | AVSampleBufferDisplayLayer/CoreAnimation queue depth: no exposed counter exists in this path. VideoToolbox’s pacer is bypassed in zero-latency mode and is not a presentation queue. [VERIFIED: codebase] |
| Latency | Report only same-clock deltas: host `acquired→encoding→encoded→sent`, client `received→decoding→decode_output→presentation_enqueued`; add `capture_begin` if capture duration is required. [VERIFIED: codebase] | `sent→received`, end-to-end, and physical scanout latency. The current clock sync is diagnostic evidence, not a bounded one-way-time proof. |

## Prescriptive End-to-End Contract

Keep native media callbacks nonblocking. Retain the scalar `key + int64` ABI for timestamps, byte counts, gauges, counters, and versioned enums. Add a separate versioned `video_path_status` callback for exact decoder/vout module names and decoded-surface descriptions because the scalar ABI cannot carry those strings safely; deep-copy native strings in Rust before returning from the callback. [VERIFIED: codebase]

Normalize both host and client inputs to append-only JSONL at one Rust boundary:

```json
{"schema":1,"session_id":"uuid","seq":41,"kind":"frame_stage","source_id":0,"generation":1,"pts_us":928341,"stage":"encoded","ts":{"us":81234567,"clock":"host_monotonic"},"frame_bytes":47622}
{"schema":1,"session_id":"uuid","seq":42,"kind":"video_path_status","actual":{"codec":"hevc","chroma":"4:4:4","decoder_module":"videotoolbox","hardware_decode":true,"decoded_fourcc":"x420"}}
{"schema":1,"session_id":"uuid","seq":43,"kind":"summary","network_one_way_us":null,"physical_present_us":null,"unknown_reason":{"network_one_way_us":"unsynchronized_clocks","physical_present_us":"no_presentation_callback"}}
```

Use `{session_id, source_id, generation, pts}` as the frame key so reconnects and PTS reuse cannot merge frames. Every event gets a monotonically increasing client-side `seq`, producer clock domain, and receipt time. Missing values are `null` plus a machine-readable reason; never use zero or `-1`. Keep raw events even when a summary cannot be completed, and make queue/drop records carry `{stage, reason, queue_id, depth, capacity, pts?}`.

Write a new evidence directory per launch, for example `telemetry/<UTC>-<session-id>/`, containing `events.jsonl`, unchanged native/child logs, and a manifest with binary/source pins and requested settings. Open with restrictive permissions, append rather than truncate, and line-flush or explicitly flush each complete record. Count and emit telemetry callback/channel/ring loss so “no drop events” cannot mean “the telemetry dropped them.” These are implementation requirements, not optional polish.

In the existing AppKit window, add compact technical sections rather than a new UI framework:

1. **Video path:** Requested and Actual columns for codec/chroma; decoder module; hardware `Yes / No / Unknown`; decoded surface format.
2. **Throughput:** stage-specific FPS and encoded-payload Mbps over labeled windows.
3. **Drops/queues:** counters grouped by exact reason and gauges grouped by queue scope.
4. **Latency:** host and client same-clock stage cards; cross-host one-way, physical present, and end-to-end visibly `Unknown` with their reason.
5. **Evidence:** session path plus the existing raw transcript, preserving diagnostic access.

## Common Pitfalls and “Do Not Hand-Roll”

- Do not feed the current consolidator until it is null-safe: it applies the latest clock offset globally and calls `expect` for missing synchronization, so incomplete/out-of-order frames can be misrepresented or panic. [VERIFIED: codebase]
- Do not divide RTT by two or call offset-corrected subtraction “one-way latency.” A future estimate would need sample time, age, count, delay selection, dispersion, and an uncertainty bound attached to the frame. [VERIFIED: codebase]
- Do not call `displayed` physical presentation. Rename the normalized stage to `presentation_enqueued`; retain the legacy raw event for auditability. [VERIFIED: codebase]
- Do not combine independent queue depths or drop counters into one unlabeled number. A one-slot VLC buffer, VideoToolbox async work, sample-layer readiness, host FIFO, metric channel, and network loss have different owners and meanings. [VERIFIED: codebase]
- Do not block capture/decode/display threads while serializing or updating AppKit. Use bounded handoff, expose its loss counter, and perform GUI aggregation off the native callbacks. [VERIFIED: codebase]
- Do not add a metrics dependency. The pinned native APIs, Rust serde path, and AppKit are sufficient. [VERIFIED: codebase]

## Validation Architecture

| Gate | Required proof |
|---|---|
| Contract/unit | Fixture tests for missing/out-of-order/duplicate/reconnect events, enum evolution, `null + reason`, rolling windows, byte-derived bitrate, and per-source correlation. Saturate every telemetry queue and assert a loss record. |
| Native | Tests or probes for each drop ownership point, FIFO depth/capacity, actual bitstream codec/chroma, VideoToolbox property success/failure, decoded fourcc, and the `presentation_enqueued` semantic. |
| Linux host | Clean locked rebuild on the pinned txproto/Kymedia/Kyber commits; confirm NvFBC physical capture, NVENC, Kymux metrics delivery, and unchanged controller service behavior on the NVIDIA X11 host. |
| Apple Silicon client | Clean arm64/macOS 15.0 build with Xcode 26.6; run H.264 4:2:0 and HEVC 4:4:4 sessions; verify requested/actual divergence is visible, hardware status comes from the live session, unknown cross-host/present values stay unknown, and the GUI remains responsive at 4K60. |
| Evidence/package | Replay patches from clean pins, run the launcher self-test and full verifier, update `engine-baseline.lock` only from the qualified binary, package/sign/archive, redownload, compare digest and complete ZIP manifest, then preserve the live session evidence directory with the package manifest. |

The current Linux environment has Rust/Cargo 1.89.0, NVIDIA RTX A5000 Laptop GPU driver 610.43.03, and pinned FFmpeg n8.1.2 available; Meson is 1.11.2 rather than the prescribed 1.10.0. `xcrun` and `codesign` are absent, so macOS qualification must run on the Apple Silicon builder. [VERIFIED: environment probe]

## Security Domain

| ASVS category | Applies | Control |
|---|---|---|
| V2 Authentication / V6 Cryptography | Yes, unchanged | Preserve verified Kymux TLS/certificate behavior; metrics must not introduce a bypass or record secret material. [VERIFIED: AGENTS.md] |
| V4 Access Control | Local evidence | Restrictive evidence-file permissions; never expose the telemetry directory over a new service in this task. |
| V5 Input Validation | Yes | Treat native strings/enums and JSONL as bounded, versioned, untrusted input; reject invalid lengths/values and retain an explicit unknown value. |

## Proposed Three-Task File Breakdown

### Task 1 — Instrument native owners and extend the Rust metric model

Own the native producers and bridges in `kymedia/subprojects/txproto/src/{iosys_nvfbc.c,encode.c,packet_sink.c,...metrics...}`, `kymedia/subprojects/vlc/{modules/access/kymux.c,src/input/decoder.c,modules/codec/videotoolbox/decoder.c,src/video_output/pic_buf.c,src/video_output/video_output.c,modules/video_output/apple/VLCSampleBufferDisplay.m,include/vlc/...metrics...}`, `kymedia/vlc-rs`, and `kysdk/kyctl/{kyvlcplayer,kyclient-common}`. Add the v1 enums/status callback, exact ownership-point facts, scoped queue/drop events, bytes, and tests; keep unavailable facts tri-state/unknown.

### Task 2 — Normalize, preserve evidence, and expose the technical GUI

Own `kysdk/kyctl/kyclient/src/metrics.rs`, top-level `kyclient/src/{metrics.rs,event_loop.rs,kymux_backend/metrics.rs}`, and `prototype/macos/ReplayDesktopLauncher.swift`. Produce append-only per-session JSONL, eliminate truncation/buffer ambiguity, make aggregation null-safe, derive scoped rates/same-clock deltas, add the five GUI sections, and retain the existing raw evidence viewer.

### Task 3 — Rebuild, replay, qualify, and publish the Apple Silicon test package

Own new parent patch files under `patches/kyber/`, `prototype/macos/engine-baseline.lock`, `scripts/{verify-gui-spike.sh,package-macos-gui.sh}`, and release documentation. Replay every nested change from the exact pins, rebuild/verify Linux host compatibility, build the arm64 client on Xcode 26.6, run the live two-mode qualification matrix, then package and verify the downloadable archive without committing nested dirty worktrees or generated artifacts.

## Assumptions Log

All implementation-path claims above were verified in the checked-out pinned sources or official Apple documentation. No unverified package or external dependency is recommended.

## Sources

### Primary (HIGH confidence)

- Checked-out Kyber Desktop, Kymux, Kymedia, txproto, VLC, vlc-rs, kyctl, and launcher/package sources at the exact gitlinks recorded in `AGENTS.md`. [VERIFIED: codebase]
- `.planning/quick/260730-j91-qualify-and-package-the-native-replaydesktop/260730-j91-{PLAN,SUMMARY}.md` and `.planning/quick/260730-mqr-implement-bidirectional-clipboard-sync-wit/260730-mqr-SUMMARY.md` for the current build, replay, and package boundary. [VERIFIED: codebase]

### Secondary (MEDIUM confidence)

- Apple, `kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder`: query the real decompression session with `VTSessionCopyProperty` to determine whether a hardware decoder was selected. [CITED: https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_usinghardwareacceleratedvideodecoder]

## Metadata

- **Standard stack:** HIGH — no new dependency; all changes stay inside the pinned stack.
- **Architecture:** HIGH — producer/consumer ownership was traced end to end in source.
- **Hardware truth:** MEDIUM until the patched property/status callbacks are exercised on the target Apple Silicon Macs.
- **Packaging:** HIGH for required mechanics; MEDIUM for the new artifact until clean macOS and Linux qualification completes.
- **Valid until:** 2026-08-29, or any Kyber/Kymedia/VLC pin change, whichever comes first.
