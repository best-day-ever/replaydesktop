---
quick_id: 260730-ruu
mode: quick-full
status: superseded
date: 2026-07-30
iteration: 1
superseded_by:
  - 260730-swz
  - macos-native-decoder-telemetry
  - client-evidence-and-gui
  - live-4k60-qualification
depends_on:
  - 260730-j91
  - 260730-mqr
scope_budget:
  declared_source_paths: 21
  new_parent_patch_artifacts: 2
prototype_exceptions:
  - The existing internal LAN/Tailscale TLS-verification exception remains unchanged; telemetry must not broaden it or record credentials.
  - Ad-hoc signing remains acceptable only for the private Apple Silicon test package; the package must state that it is not notarized or production-qualified.
must_haves:
  truths:
    - Every metric shown in the technical client is backed by an ownership-point observation or is shown as Unknown with a machine-readable reason.
    - Requested codec/chroma is visibly separate from actual bitstream codec/chroma, selected decoder implementation, live VideoToolbox hardware property, decoded fourcc, and IOSurface presence.
    - Legacy scalar stage callbacks and raw keys remain accepted and emitted; legacy displayed is preserved for evidence but normalized only as client_presentation_enqueued, never physical presentation or scanout.
    - Acquired, encoded, received, decoder-output, and presentation-enqueued FPS plus encoded/received payload bitrate come from observed events and byte counts rather than configured targets.
    - The slice reports typed drops and scoped queue gauges only at instrumented owners; uninstrumented drop sites and unavailable queue depths remain Unknown with exact reasons.
    - Every aggregate carries its window, sequence range, observed count, loss count, loss sources, and coverage state; relevant loss makes the aggregate null with measurement_lost.
    - Latency is calculated only between stages sharing one monotonic clock; cross-host one-way, end-to-end, and physical-present latency remain Unknown with exact reasons.
    - Each launch creates restrictive append-only raw evidence with session/source/generation/frame correlation, sequence numbers, requested settings, actual path status, and source/binary provenance.
    - The existing AppKit technical GUI remains responsive and exposes Video path, Throughput, Drops and queues, Latency, and Evidence sections without replacing the pinned VLC player.
    - Clean pinned replay, automated tests, Linux live probes, remote Apple Silicon package qualification, and human 4K60 UAT are recorded as separate evidence lanes before any publication decision.
  artifacts:
    - patches/kyber/0008-native-telemetry-ownership-points.patch
    - patches/kyber/0009-client-telemetry-evidence.patch
    - prototype/macos/ReplayDesktopLauncher.swift
    - scripts/verify-stream-telemetry.sh
    - scripts/package-macos-gui.sh
    - scripts/verify-gui-spike.sh
    - prototype/macos/engine-baseline.lock
    - ReplayDesktop-arm64-telemetry-spike.zip
  key_links:
    - NvFBC, encoder, Kymedia handoff, Kymux access, decoder selection, and VideoToolbox ownership points emit additive scalar facts through the existing callbacks.
    - The existing Kymux metrics packet remains backward compatible while new scalar keys, payload bytes, gauges, typed drops, producer sequence, and loss counts traverse the same path.
    - Kyctl maps native and host scalar facts into one additive typed Metric surface without changing existing stage discriminants.
    - The Rust normalizer correlates frames by session_id, source_id, generation, and pts_us and never joins host/client clock domains to manufacture latency.
    - Every rolling or per-frame aggregate is gated by overlapping callback/channel/sequence/writer loss provenance before AppKit can display a number.
    - The AppKit model consumes append-only events.jsonl off the main thread and renders only explicit values or null-plus-reason states.
    - Exact recursive pins and path-prefixed patch sections make both parent-owned patch artifacts replayable without nested commits.
---

# Truthful streaming telemetry vertical slice

> Superseded as an executable plan: validation showed that this cross-platform
> slice exceeded the quick-task scope budget. Its research and truth contract
> remain the shared design input for four independently planned and verified
> slices. Linux host telemetry is implemented by `260730-swz`; the remaining
> client-native, evidence/GUI, and live-qualification slices are intentionally
> sequenced after it.

Instrument the narrow ownership points needed to prove actual media/decode
facts, carry them through the existing scalar telemetry path, preserve raw
evidence, expose honest technical GUI values, and qualify the exact Apple
Silicon package. Facts outside this bounded slice remain explicit Unknowns.

## Task 1: Trace one frame through native ownership points using the existing scalar callbacks

**Files**

- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/iosys_nvfbc.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/encode.c`
- `upstream/kyber-desktop/kysdk/kymedia/kyavservice/src/metrics.rs`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc/modules/access/kymux.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc/src/input/decoder_helpers.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc/modules/codec/videotoolbox/decoder.c`
- `patches/kyber/0008-native-telemetry-ownership-points.patch`

**Action**

- Preserve the existing `key + int64` txproto/VLC callbacks, Kymux metrics
  packet, all legacy keys, and callback threading. Add scalar keys only; older
  consumers must continue ignoring unknown keys without a media-path change.
- Define and carry these exact stage semantics:

  | Native fact | Normalized stage | Exact meaning |
  |---|---|---|
  | `pts` | `frame_pts` | Correlation value only; never a latency clock |
  | new `capture_begin` | `host_capture_begin` | Immediately before the NvFBC grab call |
  | `acquired` | `host_capture_ready` | NvFBC grab and the existing same-GPU device copy completed |
  | `encoding` | `host_encode_submit` | Immediately before FFmpeg/NVENC submission |
  | `encoded` | `host_encode_output` | FFmpeg returned the encoded packet |
  | `sent` | `host_local_transport_write_complete` | Existing callback after local Kymux socket writes; not wire egress or remote receipt |
  | `received` | `client_payload_receive_complete` | VLC read the complete Kymux media payload |
  | `decoding` | `client_decoder_dispatch_begin` | Existing callback immediately before decoder dispatch |
  | new `decode_output` | `client_decode_output` | The real VideoToolbox output callback produced a frame or terminal drop/error |
  | `decoded` | `client_vout_picture_queued` | Existing callback after `vout_PutPicture`; not decoder completion |
  | `prepared` | `client_renderer_prepare_complete` | Existing vout prepare callback returned |
  | `displayed` | `client_presentation_enqueued` | Existing zero-latency path returned after prepare/sample enqueue and its no-op display callback; not physical presentation |
  | `skipped` | drop `client_vlc_pic_buf_superseded` | Existing one-slot VLC buffer replaced its prior picture |

- At NvFBC ownership, emit `capture_begin`, the existing `acquired`, and
  `host_capture_fifo` depth/capacity immediately around the existing push.
  Emit typed drop `host_capture_fifo_rejected` only when that push actually
  fails, retaining the bounded native error code.
- At encoder ownership, emit `encoded_payload_bytes` from the returned
  `AVPacket::size`, `host_encoded_packet_fifo` depth/capacity around the
  existing push, and typed drop `host_encoded_packet_fifo_rejected` only when
  that push fails. The configured target bitrate is never a measurement.
- Keep the Kymedia callback nonblocking. Give each successfully forwarded host
  batch a monotonic `producer_seq`; when the bounded channel is full, count
  dropped batches/entries and record their first/last affected producer
  sequence and callback-time interval. Piggyback the accumulated loss on the
  next successful batch and attempt a final bounded flush at shutdown. An
  unflushed terminal loss leaves host coverage Unknown rather than zero-loss.
- In `modules/access/kymux.c`, emit `received_payload_bytes` from the actual
  media payload length, actual codec from the received Kymux codec header, and
  actual chroma only after pinned VLC H.264/HEVC/AV1 sequence parsing accepts
  real configuration data. Until then use
  `bitstream_config_unavailable`; malformed data uses
  `bitstream_config_invalid`. Requested codec/chroma is never copied into
  actual fields.
- After decoder module load, map the selected module object to stable scalar
  enum values for the known pinned modules. Preserve the actual known module;
  an unmapped module is Unknown with `unmapped_decoder_module`, not guessed
  from codec or launch flags.
- After every real VideoToolbox session creation/restart, query
  `kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder` and emit
  true, false, or Unknown with `property_query_failed`. In the output callback,
  emit `decode_output`, decoded CoreVideo fourcc,
  `CVPixelBufferGetIOSurface(imageBuffer) != NULL`, and the typed reasons
  `client_videotoolbox_frame_dropped` or
  `client_videotoolbox_decode_error` from their real flags/status.
- Do not modify the broad VLC renderer, CoreAnimation, sample-layer, clock-sync,
  or packet-loss paths. The existing `prepared`/`displayed` callbacks prove
  renderer preparation and enqueue-return semantics only. Renderer module,
  sample-layer readiness/error, presentation queue depth, and physical
  presentation remain Unknown with
  `renderer_status_not_exported`, `no_queue_depth_api`, or
  `no_physical_presentation_callback`.
- Generate one parent-owned, binary/full-index, path-prefixed composite patch.
  Build its sections from the exact txproto, Kymedia, and VLC roots with
  prefixes relative to recursive `upstream/kyber-desktop`, and verify each
  section's root/pin/path allowlist. Do not create nested commits or touch the
  user's untracked `patches/kyber/0003-linux-zlib-pic.patch`.

**Verify**

- Add focused native/Rust tests beside the changed code for actual
  codec/chroma success/unavailable/invalid, decoder enum known/unknown,
  VideoToolbox hardware true/false/query-failure, decoded fourcc/IOSurface,
  VT drop/error, exact encoded/received bytes, both FIFO gauges/drop branches,
  host channel saturation, producer-sequence gaps, loss piggyback, and
  terminal unflushed-loss coverage.
- From `upstream/kyber-desktop/kysdk/kymedia`, run focused Meson/Ninja tests,
  `cargo test --locked -p kyavservice`, formatting, and task-scoped clippy with
  warnings denied for changed Rust code.
- Replay 0008 with `git apply --check` from a recursively initialized scratch
  Kyber checkout whose Kyber/Kysdk/Kymedia/txproto/VLC pins match the lock;
  assert the patch contains only the six declared source paths.

**Done**

- One H.264 frame and HEVC/invalid configuration fixtures traverse the legacy
  scalar path plus additive actual path/bytes/gauge/drop/loss facts; native
  media threads remain nonblocking and every renderer/presentation fact not
  owned by this slice is explicitly Unknown.

## Task 2: Map, normalize, and persist loss-qualified client evidence

**Files**

- `upstream/kyber-desktop/kysdk/kyctl/kyclient-common/src/metrics.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyvlcplayer/src/player.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyclient/src/platform/desktop/message.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyclient/src/kymux_backend/metrics.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyclient/src/metrics.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyclient/src/capi.rs`
- `upstream/kyber-desktop/kysdk/kyctl/kyclient-rs/src/lib.rs`
- `upstream/kyber-desktop/kyclient/src/metrics.rs`
- `upstream/kyber-desktop/kyclient/src/event_loop.rs`
- `patches/kyber/0009-client-telemetry-evidence.patch`

**Action**

- Extend the existing metric model additively with bounded scalar facts for
  stage, path status, payload bytes, queue gauge, typed drop, producer
  sequence, and measurement loss. Keep existing stage variants and C-ABI
  discriminant order byte-compatible; validate enum/range/conversion failures
  into Unknown/loss instead of `unwrap`, `expect`, zero, or `-1`.
- Preserve the legacy raw `metrics.json` path and record shapes, including raw
  `displayed`, for backward compatibility. Add a separate schema-1 evidence
  stream rather than using the current global-offset consolidator to create
  cross-clock values.
- Correlate frames only by
  `{session_id, source_id, generation, pts_us}`. Generate one session UUID per
  process launch, increment generation on reconnect/source restart, assign a
  strictly increasing client `seq` to every received/derived record, retain
  producer clock and client receipt time, and never merge reused PTS values
  across generations.
- Persist `session_start`, `frame_stage`, `payload_bytes`,
  `video_path_status`, `queue_gauge`, `drop`, `measurement_loss`, `summary`,
  and `session_end` JSONL records. Missing facts are null plus one bounded
  reason: `not_observed`, `producer_unsupported`,
  `bitstream_config_unavailable`, `bitstream_config_invalid`,
  `unmapped_decoder_module`, `property_query_failed`,
  `renderer_status_not_exported`, `measurement_lost`,
  `unsynchronized_clocks`, `no_queue_depth_api`, or
  `no_physical_presentation_callback`.
- Account for the Kyctl client channel with its real depth/capacity. On full
  channel, count dropped records and affected sequence/time interval; on
  producer-sequence discontinuity, emit the exact missing range. Treat
  invalid scalar groups and C-bridge rejections as scoped measurement loss.
- Give every rolling/per-frame aggregate a mandatory coverage object:
  `{window_start_us, window_end_us, first_seq, last_seq, observed_count,
  loss_count, loss_sources, complete}`. Track loss intervals from native
  callbacks when reported, host/client channel saturation, producer/client
  sequence gaps, invalid bridge records, and evidence writer failures.
- If any relevant loss interval overlaps an FPS, bitrate, or latency window,
  write and render `value: null`, `reason: measurement_lost`, exact
  `loss_count`, `loss_sources`, and the incomplete coverage object. Do not
  display a lower bound, extrapolation, cached prior value, or zero. A clean
  window may show a number only with `loss_count: 0` and `complete: true`.
- Derive acquired, encoded, received, decoder-output, and
  presentation-enqueued FPS over a monotonic 1-second window. Derive host
  encoded-payload and client received-payload Mbps only from observed bytes,
  labeling them as payload rates without QUIC/TLS overhead.
- Derive only same-clock deltas: host capture, capture-to-encode-submit,
  encode, encode-output-to-local-write, and host pipeline; client
  receive-to-decode-dispatch, VideoToolbox decode, decode-output-to-vout,
  vout-to-enqueue-return, and client pipeline. Cross-host send-to-receive,
  end-to-end, and physical-present latency are always null with
  `unsynchronized_clocks` or `no_physical_presentation_callback`.
- Create one
  `Application Support/ReplayDesktop/telemetry/<UTC>-<session-id>/` directory
  per launch with mode 0700 and mode-0600 `events.jsonl`, child/native log
  targets, and `manifest.json`. Use create-new/append semantics, complete
  newline-delimited records, bounded lines, and explicit per-record flush.
  Never record credentials, private keys, JWTs, clipboard content, or inferred
  facts.
- Treat an evidence writer failure as a terminal loss interval for that
  session: preserve already-written lines, notify the GUI through the in-memory
  callback and stderr, set current/subsequent aggregates null, and never claim
  a complete manifest/session end. Direct CLI and GUI launches use the same
  evidence contract.
- Generate 0009 as one path-prefixed composite patch from exact Kyctl and
  top-level Kyber roots. It applies after existing 0006/0007 and 0008, contains
  only the nine declared source paths, and creates no nested commit.

**Verify**

- Add focused tests for old-key-only packets, additive keys, enum evolution,
  negative/oversize/invalid groups, missing/out-of-order/duplicate/reconnect
  frames, PTS reuse, host producer gaps, client sequence gaps, client-channel
  saturation, and C-bridge rejection.
- Add deterministic aggregate tests proving:

  - clean windows emit numeric FPS/bitrate/same-clock latency with complete
    zero-loss coverage;
  - each callback/channel/sequence loss source overlapping a window emits null
    plus `measurement_lost`, exact loss count/sources, and incomplete coverage;
  - loss outside a later clean window does not poison it;
  - writer failure preserves prior lines and nulls the affected and subsequent
    windows;
  - missing physical-presentation and cross-clock facts can never become
    numeric.

- From `upstream/kyber-desktop/kysdk/kyctl`, run
  `cargo test --locked -p kyclient-common -p kyvlcplayer -p kyclient`,
  formatting, and task-scoped clippy. From `upstream/kyber-desktop`, run the
  focused top-level kyclient metrics/evidence tests.
- Replay 0009 with `git apply --check` after the existing product patches and
  0008 in the same exact-pin recursive scratch checkout; assert it contains
  only the nine declared source paths.

**Done**

- Legacy metrics remain readable while the schema-1 evidence stream preserves
  correlated raw facts and refuses every aggregate whose declared window has
  incomplete measurement coverage.

## Task 3: Expose the bounded slice in AppKit and qualify the exact two-machine package

**Files**

- `prototype/macos/ReplayDesktopLauncher.swift`
- `prototype/macos/engine-baseline.lock`
- `scripts/verify-stream-telemetry.sh`
- `scripts/package-macos-gui.sh`
- `scripts/verify-gui-spike.sh`
- `README.md`
- `ReplayDesktop-arm64-telemetry-spike.zip` (generated output, not committed source)
- `.planning/quick/260730-ruu-implement-truthful-linux-to-macos-stream/260730-ruu-SUMMARY.md` (execution output)

**Action**

- Let the launcher create and pass the restrictive session evidence directory
  before child start. Capture stdout/stderr/native log without mixing them
  into parsed telemetry. Parse bounded complete JSONL lines on a background
  queue, retain partial trailing lines, count rejected/oversize/sequence-gap
  input, and update AppKit only on the main thread.
- Add five compact sections to the existing technical window:

  1. **Video path:** requested versus actual codec/chroma, decoder
     implementation, hardware `Yes / No / Unknown (reason)`, decoded fourcc,
     IOSurface, renderer/export status, and enqueue-return semantics.
  2. **Throughput:** stage-specific FPS and encoded/received payload Mbps, each
     with window, observed count, loss count, and coverage state.
  3. **Drops and queues:** typed counters for the instrumented owners, scoped
     gauges/capacities, and explicit Unknown rows for uninstrumented owners.
  4. **Latency:** loss-qualified same-clock stage deltas and visibly Unknown
     cross-host/end-to-end/physical-present values with reasons.
  5. **Evidence:** session path, schema/session/generation/sequence health,
     writer/manifest state, and the existing bounded raw transcript.

- The GUI must render aggregate null on overlapping measurement loss and clear
  a previously shown numeric value when the next window becomes incomplete.
  Actual path status becomes Unknown after relevant loss until a fresh
  ownership-point status record arrives.
- Extend launcher fixtures for requested/actual disagreement, every Unknown
  reason, clean/incomplete coverage transitions, host/client saturation,
  producer/client sequence gaps, writer failure, stale-status clearing, and
  the exact `displayed → client_presentation_enqueued` label.
- Extend `engine-baseline.lock` and source verification with exact Kymedia,
  txproto, VLC, vlc-rs, and Kyctl pins plus rebuilt engine/payload digests.
  Verify the two composite patch artifacts by their exact section prefixes and
  declared path allowlists; leave 0003 and all unrelated/untracked files
  untouched and unstaged.
- **Automated lane:** `scripts/verify-stream-telemetry.sh --automated` runs all
  Task 1/2 tests and launcher fixtures, legacy compatibility, evidence
  mode/append/flush tests, saturation/gap/writer-failure cases, source-boundary
  checks, secret scans, arm64/minimum-OS/signing gates, and exact app/ZIP
  manifest comparison.
- **Linux live lane:** rebuild only from the exact recursive pins and ordered
  patches, preserve the host configuration digest, replace only the scoped
  user service, and prove X11/NvFBC/NVENC/Kymux plus actual codec/chroma,
  payload bytes, selected native drop/gauge/loss facts, and unchanged input/
  clipboard behavior. Restore an active clean service and the original config.
- **Apple Silicon package lane:** explicitly select Xcode 26.6, deployment
  target 15.0, and clean native/Rust/package outputs. Run H.264 4:2:0 and HEVC
  4:4:4 against the Linux host; verify actual path values or exact Unknown
  reasons, clean-window aggregates, deliberately saturated null aggregates,
  responsive GUI, restrictive evidence, arm64-only inventory, deep ad-hoc
  signature, raw CLI, and app/ZIP equivalence. If Xcode 26.6 or the remote Mac
  is unavailable, mark this lane unavailable and make no inherited claim.
- Produce `ReplayDesktop-arm64-telemetry-spike.zip` only from the qualified
  app and record its SHA-256/manifest locally. Do not publish or replace any
  release in this quick execution; publication is a separate explicit action
  after automated, Linux-live, Apple-package, and requested human verification
  results are reviewed.
- **Human 4K60 UAT lane:** record `human_needed` for a 10-minute
  3840×2160@60, zero-configured-buffer Finder-launched session. The operator
  checks rendered control, stable visible labels, responsive GUI, H.264 and
  HEVC 4:4:4 actual/requested behavior, intelligible drops/queues, and evidence
  access. Human smoothness cannot create latency or scanout evidence.
- README and SUMMARY report four independent verdicts: `automated`,
  `linux_live`, `apple_silicon_package`, and `human_4k60_uat`, each with
  `pass`, `fail`, `unavailable`, or `human_needed`. Never collapse them into a
  misleading overall PASS.

**Verify**

- Run `scripts/verify-stream-telemetry.sh --automated`; the saturation,
  sequence-gap, and writer-failure matrix must prove that no affected aggregate
  remains numeric in Rust output or AppKit fixtures.
- Run `scripts/verify-gui-spike.sh --source-boundary --base <base> --head
  <head>`; it verifies exact pins, ordered patches 0001–0009, the 21 declared
  source paths, no nested commits, and no unrelated/untracked/private/runtime
  material in the source range.
- Run `scripts/verify-stream-telemetry.sh --linux-live` on the NVIDIA X11 host
  and `scripts/verify-stream-telemetry.sh --macos-package
  ReplayDesktop-arm64-telemetry-spike.zip` on the Xcode-26.6 Apple Silicon
  builder. The latter repeats signing, metadata, evidence, raw CLI, archive
  manifest, and both live codec-mode checks.
- Confirm the summary leaves `human_4k60_uat: human_needed` until the operator
  performs it and records no publication URL/tag.

**Done**

- The bounded GUI slice, exact-pin Linux runtime, and local Apple Silicon test
  archive pass their available lanes; every aggregate is coverage-qualified,
  unavailable path/queue/presentation facts remain explicit Unknowns, and
  publication waits for a separate reviewed action.

## Explicitly deferred after this slice

- Physical presentation/scanout callbacks and physical-present latency.
- Cross-host one-way and end-to-end latency derived from full uncertainty-bound
  clock synchronization.
- AVSampleBufferDisplayLayer/CoreAnimation queue depth, which the current path
  does not expose, and broad renderer-module/readiness/error instrumentation.
- Exhaustive FFmpeg, VLC, sample-layer, network-packet, and operating-system
  drop-site enumeration beyond the six instrumented/legacy owner reasons.
- QUIC/TLS wire bitrate and protocol-overhead accounting; this slice reports
  encoded and received media payload bytes only.
- A native VideoToolbox/Metal player rewrite or any replacement of pinned VLC.

These are not silently omitted GUI categories: their rows remain visible as
Unknown with the reasons defined above.

## Threat model

| Threat ID | STRIDE | Boundary/component | Severity | Disposition | Mitigation |
|---|---|---|---|---|---|
| T-RUU-01 | Tampering / Elevation | Native scalar callbacks → Rust | high | mitigate | Stable enums, numeric bounds, checked conversions, null-plus-reason fallback, and adversarial tests |
| T-RUU-02 | Spoofing / Tampering | Linux telemetry → authenticated Kymux → macOS | high | mitigate | Preserve authenticated transport, bind session/source/generation/PTS, producer sequence/loss ranges, and never weaken TLS |
| T-RUU-03 | Denial of service | Media callbacks, bounded channels, writer, AppKit parser | high | mitigate | Nonblocking producers, bounded records/windows, loss invalidation, off-main-thread parsing, and saturation tests |
| T-RUU-04 | Information disclosure | Evidence directory/package | high | mitigate | 0700 directory/0600 files, no credentials/clipboard, secret scans, release exclusion, and exact archive manifest |
| T-RUU-05 | Repudiation | Metrics/package claims | medium | mitigate | Window coverage provenance, append-only sequence, pin/binary/toolchain manifest, hashes, and separate verification lanes |
| T-RUU-06 | Tampering | Composite patch/build supply chain | high | mitigate | Exact recursive pins, path-prefixed allowlisted patch sections, clean replay/build, source-boundary check, and no nested commits |

## Source coverage audit

| Source | Item | Coverage | Status |
|---|---|---|---|
| GOAL | Actual path, FPS/bitrate, drops/queues, latency, GUI, evidence, Apple package | Tasks 1–3 | COVERED |
| REQ | No ROADMAP requirement IDs are assigned to quick task 260730-ruu | — | N/A |
| RESEARCH | Native owner instrumentation through existing callbacks | Task 1 | COVERED |
| RESEARCH | Null-safe, loss-qualified normalization and durable evidence | Task 2 | COVERED |
| RESEARCH | Technical GUI and two-machine package qualification | Task 3 | COVERED |
| CONTEXT | No quick-task CONTEXT.md or D-numbered locked decisions exist | Caller/checker constraints are explicit throughout | N/A |

## Success criteria

- Exactly 21 declared source paths and two parent-owned patch artifacts deliver
  the slice; generated evidence/archive/summary outputs are listed separately.
- Every requested category is observed or visibly Unknown with reason.
- Any relevant loss overlapping an aggregation window produces null
  `measurement_lost` output with exact loss and coverage provenance.
- Legacy telemetry remains readable; no broad player/clock/renderer rewrite,
  dependency addition, pin drift, nested commit, publication, or unrelated
  staging occurs.
