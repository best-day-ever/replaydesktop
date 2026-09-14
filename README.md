# ReplayDesktop

> [!WARNING]
> **Experimental development and testing only.** This is work-in-progress
> prototype software, not a production-ready remote desktop solution. Features
> may be incomplete, unstable, or change without notice. Use only in controlled
> development and test environments.

ReplayDesktop is an experimental Linux-to-macOS remote-desktop
prototype built on the pinned Kyber/Kymux stack.

## Direct-admission control server

The repository now includes a standalone single-organization control server in
[`server/`](server/README.md). It provides local user/workstation
administration, login and grants, one-use UDP source proof, ephemeral UDM Pro
SE firewall policies, and source-bound Ed25519 Kymux tickets. Kymux media stays
on the direct client-to-workstation path; the server is not a relay.

The fake-gateway tracer is implemented and tested. Public deployment remains
blocked on the documented live UDM policy-order/DNAT spike and on wiring the
included strict ticket verifier into the pinned Kyber workstation accept path.

## Apple Silicon prototype client

The native technical clipboard spike is published as an experimental GitHub
prerelease:

<https://github.com/best-day-ever/replaydesktop/releases/tag/v0.3.0-spike.1>

Download `ReplayDesktop-arm64-clipboard-spike.zip`, expand it, and move
`ReplayDesktop.app` to `/Applications`. Finder launch opens an AppKit control
panel; `ReplayDesktop.app/Contents/MacOS/kyclient` remains the direct CLI
fallback. The panel:

- always launches the established Kymux/TLS-bypass/zero-video-buffer/metrics
  baseline before adding visible operator choices;
- labels H.264 4:2:0 as proven and HEVC 4:2:0, HEVC 4:4:4, and AV1 4:2:0 as
  experimental/manual; AV1 4:4:4 is never offered;
- maps audio and multi-monitor controls only to existing Kyber flags, with
  total bitrate shared across active displays;
- makes the Input control govern mouse plus focused-window keyboard
  forwarding, keeps immersive system-shortcut suppression visibly unavailable
  on macOS, and forces `--keyboard-grab=false`;
- exposes an opt-in `Clipboard sync — Text + HTML, 60 KiB` control, independent
  of general input, and negotiates host copy and paste policy separately before
  touching the Mac pasteboard;
- displays a bounded live tail of child output, Kyber logs, and
  `metrics.json`; and
- writes runtime logs and metrics under the user's Application Support
  directory rather than inside the app.

Current host qualification finds PulseAudio-on-PipeWire monitor sources and
the GUI request starts Kyber's single Kymux audio service. The preserved
four-channel M-Audio default monitor currently fails capture initialization
with `Invalid argument` before Opus encoding begins, so this prerelease makes
no working-audio claim. Choosing or adapting a compatible monitor remains
follow-up work.

This prototype is intended for controlled LAN/Tailscale testing. The operator
explicitly approved fixed development credentials, TLS certificate-verification bypass, and
ad-hoc signing without notarization for this spike. Those exceptions are not
production-safe; the public repository and experimental prerelease builds are
for development and testing only.

The tracked `prototype/macos/engine-baseline.lock` binds packaging to the exact
known raw-input `kyclient` SHA-256, executable-payload SHA-256, code-signature
boundary, and pinned Kyber/Kysdk commits. Packaging fails if those engine bytes
or source pins differ. The copied executable payload is verified unchanged,
while its mutable signature envelope is regenerated so the finished app has a
valid ad-hoc deep signature. The bundle also records and verifies its explicit
package version, ReplayDesktop source range and digest, Xcode version/build,
Swift version, SDK version/build, macOS deployment target, and raw-engine
provenance.

The original known-good CLI-only prerelease remains available and is not
replaced:

<https://github.com/best-day-ever/replaydesktop/releases/tag/v0.1.0-spike.1>

Both packages:

- support Apple Silicon only;
- require macOS 15 Sequoia or newer;
- are ad-hoc signed for internal testing, but are not notarized; and
- contain no Kyber test certificates or private trust material.

Because this build is not notarized, macOS may require the operator to approve
the first launch through **System Settings → Privacy & Security → Open
Anyway**. Do not disable Gatekeeper or strip quarantine metadata.

The clipboard package has passed automated argument-matrix, engine-provenance,
build-metadata, architecture, deployment-target, archive, raw-CLI, and
code-signature checks. Physical cross-application Text/HTML copy and paste,
macOS pasteboard privacy prompts, simultaneous real-user changes, Finder launch
and GUI connection, rendered pixels/input, short trackpad scrolling, system
audio, two-screen mode, and visibly changing telemetry remain explicit human
UAT.
The package was compiled with Xcode 26.2, the newest Xcode installed on the
builder, rather than the planned Xcode 26.6 qualification lane.
The prior `v0.2.0-spike.1`, `v0.2.0-spike.2`, and `v0.2.0-spike.3` assets
remain immutable for audit.

## Pinned Kyber source

The official Kyber Desktop 0.27.0 source is recorded as a recursive submodule:

```console
git submodule update --init --recursive
git -C upstream/kyber-desktop apply \
  ../../patches/kyber/0001-secure-prototype-packaging.patch
git -C upstream/kyber-desktop/kysdk/kymedia apply \
  ../../../../patches/kyber/0002-linux-disable-ffmpeg-vulkan.patch
git -C upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto apply \
  ../../../../../../patches/kyber/0008-txproto-host-telemetry.patch
git -C upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto apply \
  ../../../../../../patches/kyber/0012-txproto-capture-begin.patch
git -C upstream/kyber-desktop/kysdk/kymedia apply \
  ../../../../patches/kyber/0009-kymedia-host-telemetry-forwarding.patch
git -C upstream/kyber-desktop/kysdk/kymedia apply \
  ../../../../patches/kyber/0013-kymedia-host-evidence-sequence.patch
git -C upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc apply \
  ../../../../../../patches/kyber/0010-vlc-video-path-telemetry-abi.patch
git -C upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc apply \
  ../../../../../../patches/kyber/0011-vlc-macos-decoder-telemetry.patch
git -C upstream/kyber-desktop/kysdk/kymedia/subprojects/vlc-rs apply \
  ../../../../../../patches/kyber/0014-vlc-rs-video-path-bridge.patch
git -C upstream/kyber-desktop/kysdk/kynput apply \
  ../../../../patches/kyber/0004-linux-hires-wheel.patch
git -C upstream/kyber-desktop/kysdk/kynput apply \
  ../../../../patches/kyber/0005-kynput-macos-clipboard.patch
git -C upstream/kyber-desktop/kysdk/kyctl apply \
  ../../../../patches/kyber/0006-kyctl-clipboard-negotiation.patch
git -C upstream/kyber-desktop apply \
  ../../patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch
```

The first patch removes upstream test identities from Linux and macOS
packages and supplies ReplayDesktop bundle metadata. The second keeps the
Linux FFmpeg build on the CUDA/NVENC path while disabling its unused,
currently incompatible Vulkan codec path. The fourth preserves the established
macOS scroll conversion while emitting Linux `REL_WHEEL_HI_RES` and
`REL_HWHEEL_HI_RES` events immediately, with accumulated legacy detents for
compatibility. Patch 0005 adds the bounded native macOS pasteboard handler and
strict Linux/wire clipboard validation. Patch 0006 carries the independently
negotiated host copy/paste permissions into that handler before the client
pipeline starts. Patch 0007 lets clipboard start that pipeline independently
of mouse and keyboard input while leaving interactive handlers disabled.
Patch 0008 fixes and instruments the bounded txproto host queues at the native
capture, encode, and packet-sink ownership points. Apply it in txproto before
patch 0009, which carries callback rejection and cumulative telemetry-loss
facts through the Kymedia Rust forwarder. These facts remain host-local raw
measurements; they do not claim cross-machine latency or scanout timing.

Patch 0012 adds the missing real NvFBC begin observation. `capture_begin` is
sampled on the host monotonic clock immediately before
`nvFBCToCudaGrabFrame`; the same PTS-qualified batch is published only after
the frame reaches the application-owned CUDA surface through the checked
device-to-device copy. The resulting `capture_begin` to `acquired` interval is
`host_capture_to_cuda_surface_ready`: it includes the NvFBC grab and the
same-GPU copy. It is not scanout age or a claim about pure GPU capture time,
and failed grabs, allocations, or copies do not publish orphan begin records.

Patch 0013 gives every host forward attempt one monotonic producer sequence.
Accepted batches preserve every native txproto entry in its original order
and append `telemetry_forwarder_sequence`; rejected attempts use that same
identity in coherent cumulative first/last-sequence and batch/entry loss
state. Native callback rejection and the Rust forwarder's bounded-channel
rejection are separate, correlated loss layers rather than interchangeable
counts.

After the callback-owned channel closes, accepted batches drain before one
direct terminal record is sent through the still-open KyCom endpoint. The
record carries the final attempted sequence, final accepted sequence (`0`
means none), and the cumulative forwarder-loss snapshot. Only a terminal
record actually observed by the consumer confirms producer completion. A
disconnect without it must be classified as
`producer_terminal_unconfirmed`; local send success is not remote receipt
evidence.

Verify this host evidence split without deploying, restarting the Linux user
service, connecting a client, or changing an installed macOS application:

```console
scripts/verify-host-evidence-sequence.sh --source
scripts/verify-host-evidence-sequence.sh --tests
scripts/verify-host-evidence-sequence.sh --replay
```

Patch 0014 independently carries the native video-path callback through
`vlc-rs`; patches 0015 and 0016 remain the Kyctl evidence ABI and client
writer/reducer splits. Those layers consume this contract; they do not change
the host meanings frozen here.

Run `scripts/verify-host-telemetry.sh --source`, `--tests`, and `--replay` to
check the patch allowlists, focused native/Rust gates, and clean pinned replay.
The separate `--live` mode is an explicit, bounded host-service probe: it
records the current service/config state, checks deployed NvFBC, NVENC, and
telemetry evidence, and restores the original active state without printing
configuration contents.

Patch 0010 adds a separate, versioned, fixed-capacity video-path callback
without changing the existing scalar metrics callback, keys, or meanings. It
reports the compressed H.264/HEVC/AV1 codec, profile, chroma, and bit depth
parsed from the received Kymux configuration, followed by the decoder module
that VLC actually selected. Callback rejection is observable through coherent
cumulative loss counts and producer sequence bounds; the native publisher
does not serialize media or log event payloads.

Patch 0011 adds the macOS-owned facts. Hardware use comes from
`kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder` on the
created VideoToolbox session; enable/require requests remain separate policy
flags, and an unreadable or non-boolean property is explicitly Unknown.
Compressed chroma is not inferred from the decoded surface:
`decoded_fourcc` is the real `CVPixelBuffer` pixel format and IOSurface
presence is observed independently. Decoder callbacks produce one typed
terminal outcome for each correlated frame. The sample-buffer renderer
separately reports failures, observable backpressure, and successful enqueue.

The existing scalar `displayed` value remains unchanged, but means only
`legacy_display_callback_returned`: VLC core can emit it after renderer
early-return paths. It is not evidence of enqueue, physical presentation, or
scanout. `renderer_sample_enqueued` is the distinct enqueue fact. Physical
presentation, scanout timestamp, and layer queue depth remain explicitly
Unknown because the current API path exposes no authoritative observation.

Verify the native boundary in order:

```console
scripts/verify-macos-native-telemetry.sh --source
scripts/verify-macos-native-telemetry.sh --tests
scripts/verify-macos-native-telemetry.sh --replay
scripts/verify-macos-native-telemetry.sh --mac-smoke finn@100.119.34.79
```

The Mac smoke copies only the verifier and exact patches, works under a
mode-0700 temporary directory, builds/tests the pinned source without
installing or replacing `ReplayDesktop.app`, returns a versioned manifest, and
removes only its validated scratch directory. Its recorded Xcode version must
be interpreted literally: Xcode 26.2 is compatibility evidence, not the
project's Xcode 26.6 release qualification.

Patch 0014 must be applied to `vlc-rs` at
`7cbfc51313b4bb3ab07be51505b7054e4e2c366b` only after the exact VLC
`0010` then `0011` boundary exists. Its callback accepts at most 64 records,
1024 bytes per record, and 64 KiB per native call. It validates the complete
version-one batch and copies the known 192-byte prefix plus bounded UTF-8 text
into owned Rust values before invoking consumer code; no callback-owned
pointer, slice, or text escapes the FFI call.

The safe surface is
`MediaPlayer::set_video_path_callback(&mut self, callback)` with
`VideoPathCallbackEvent::{Records(Vec<VideoPathEvent>),
ProducerTerminalUnconfirmed(VideoPathTerminal)}` and an explicit
`VideoPathCallbackDisposition::{Accept, Reject}` result. Known enum values are
typed, future discriminants and flag bits retain their raw values, and
malformed sentinels are rejected. `VideoPathCallbackLoss` remains the native
cumulative callback-loss tuple carried by accepted records; saturating
bridge-rejected callback/record counts and their readable sequence bounds stay
separate.

Validation failure, consumer `Reject`, or consumer panic returns one bounded
nonzero rejection to native libVLC without retry, waiting, payload logging, or
unwinding through C. Native userdata is a monotonic, never-reused opaque ID,
not a dereferenced Rust allocation. Callback entry resolves that ID through a
bounded-lifetime registry and clones an `Arc` invocation guard before consumer
dispatch, so callback-triggered owner drop cannot invalidate in-flight state.

Rust calls `libvlc_media_player_release` before unregistering the ID. Calls
that arrive during release can still resolve the guard; calls arriving after
unregistration fail closed without touching freed memory. This deliberately
does not treat the reference-count decrement as proof that native destruction
completed. After unregistration Rust emits one local
`ProducerTerminalUnconfirmed` control event, and the state is freed only after
all in-flight guards finish. That event is not a native clean-completion
acknowledgement, a final native-loss snapshot, or physical-presentation
evidence. Kyctl consumption, GUI state, evidence reduction, and packaging
remain outside patch 0014.

Verify the Rust bridge locally and offline without deployment:

```console
scripts/verify-vlc-rs-video-path-bridge.sh --source
scripts/verify-vlc-rs-video-path-bridge.sh --tests
scripts/verify-vlc-rs-video-path-bridge.sh --replay
```

The prototype retains Kyber's existing clipboard event set. Requests are
serialized and local Mac changes win over an in-flight remote fetch, but fully
deterministic simultaneous cross-peer conflict resolution requires a future
versioned generation/request ID.

The rebuilt controller and `kynputservice` are a matched prototype pair:
mixed-version internal MessagePack IPC is not supported. The external Kymux
HTTP response remains tolerant of missing directional fields, and its legacy
aggregate is enabled only when both host directions are authorized.

## macOS hosting status

Pinned Kyber 0.27.0 does not contain a working macOS sender. Its controller and
Kymux pieces can compile on macOS, but the media server is disabled there and
there is no ScreenCaptureKit capture backend, wired VideoToolbox host encoder,
system-audio capture path, or safe macOS host-input injector. The narrowest
host port is ScreenCaptureKit NV12 to H.264 VideoToolbox over the existing
Kymux transport, followed by CoreGraphics input and reuse of this pasteboard
adapter. That is a platform port, not a hidden build option in the current
prototype.

## Host readiness doctor

`replay-host-doctor` is the first ReplayDesktop walking skeleton. It performs
bounded, read-only host observations, derives one fail-closed G0 decision,
writes one versioned evidence envelope, reads those exact bytes back, and
verifies that the evidence belongs to the current run.

The host-foundation probe proves or rejects the local Xorg session and exact
NVIDIA kernel/NVML userspace identity. Selected output mapping is available
only through explicit, measured selection; omission enters discovery and never
chooses a default. With separately authenticated NVIDIA Capture SDK and CUDA
Driver API headers, the live path now proves one selected-output NvFBC
shared-CUDA frame. NVENC tuples remain `UNPROVEN`, so the current command still
cannot produce G0 PASS.

## Run the doctor

From the repository root, use the locked toolchain and lockfile:

```console
cargo run --locked --bin replay-host-doctor -- run --evidence target/g0-evidence.json
```

Until later host probes are implemented, a valid run prints one
`replaydesktop.host-doctor-result.v1` JSON object to stdout, persists
`target/g0-evidence.json`, and returns exit 2. The JSON includes its fresh
`run_id`, live provenance, the overall `fail` status, and stable ordered
extension reasons.

To verify the exact evidence immediately, copy the `run_id` from that result:

```console
cargo run --locked --bin replay-host-doctor -- verify-evidence --evidence target/g0-evidence.json --run-id <RUN_ID>
```

Verification is read-only. It strictly decodes V1, checks extension payload
digests, requires the requested run ID, and rechecks the current boot, process
session, executable digest, and time bounds. A successful verification prints
one result object and returns exit 0; it verifies integrity and currentness,
not readiness.

## Discover and select one physical output

Output discovery uses the same bounded production worker as an explicit run.
It queries XRandR but makes no selection, writes a `selected-output.v1`
discovery payload, and exits 2:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo run --locked --bin replay-host-doctor -- run \
    --evidence target/g0-post-reboot-output-discovery.json
```

Enumerate the exact UTF-8 XRandR candidate names from persisted evidence:

```console
jq -r '.extensions[]
  | select(.id == "selected-output.v1")
  | .payload.candidates[]
  | select(.display != null)
  | .display' target/g0-post-reboot-output-discovery.json
```

The operator must choose one exact candidate; the doctor never selects by
position, display order, or an inferred default:

```console
export REPLAY_HOST_OUTPUT='DP-0.3'
```

Run the full read-only relation with that exact name. A successful HOST-02
proof still exits 2 because later media gates remain open:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo run --locked --bin replay-host-doctor -- run \
    --output "$REPLAY_HOST_OUTPUT" \
    --evidence target/g0-post-reboot-output.json
doctor_status=$?
set -e
test "$doctor_status" -eq 2

cargo run --locked --bin replay-host-doctor -- verify-evidence \
  --evidence target/g0-post-reboot-output.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 unproven \
  --require-host04 unproven \
  --validate-extension selected-output.v1
```

The admitted extension records the exact XRandR output/CRTC/mode/provider and
active timing/origin, then follows
`NV_CTRL_DISPLAY_RANDR_OUTPUT_ID` to one NV-CONTROL display target, one
enabled-on-X-screen membership, one owning NV-CONTROL GPU target, and one NVML
device matching both canonical PCI BDF and NVML UUID. MST is accepted when
that source-defined chain is unique; `DP-0.3` is the live proven MST example.
DRM is optional diagnostic data only and never affects HOST-01, HOST-02, or
the authoritative topology token.

Missing or ambiguous direct relations, mismatched names/BDFs/UUIDs, malformed
observations, an actual multi-output CRTC scanout, or a RandR/topology change
produce a typed HOST-02 failure. No raw EDID bytes, X authority material,
native errors, SDK paths, or inherited secrets are persisted.

## Prove one selected-output NvFBC frame

The current host already has everything needed for the CUDA side of this
prototype. No additional CUDA runtime or full Toolkit download is required:
the build consumes only the CUDA Driver API declarations in `cuda.h` and
`cudaTypedefs.h` from
`/opt/cuda/targets/x86_64-linux/include`, and the program dynamically loads
`libcuda.so.1` from the installed NVIDIA display driver. The later NVENC plan
uses its separately pinned Video Codec SDK headers.

The qualified Capture SDK 9.0 source was supplied by the operator after a
separate lawful-source and terms assertion. This run used these explicit
source roots:

```console
export REPLAY_NVFBC_SDK_ROOT=/home/finn/.local/share/replaydesktop/nvidia-capture-sdk/capture-linux-v9.0.0-31e0d8f2e2fe94fa/include
export REPLAY_CUDA_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include
export REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include
export REPLAY_HOST_OUTPUT=DP-0.3
```

Source-enabled builds fail closed unless the authenticated NvFBC 1.9 and CUDA
Driver API 13.3 declarations match the C ABI oracle and remain stable
throughout the build. Exercise the source, fixture, cleanup, and process
contracts before touching hardware:

```console
cargo test --locked host03_source_abi_
cargo test --locked host03_fixture_
cargo test --locked host03_cleanup_
cargo test --locked --test host_doctor_cli host03_process_
```

The independent live test has a 180-second process bound and an inner
750-millisecond one-grab deadline:

```console
timeout 180s cargo test --locked --test host_doctor_cli \
  host03_live_one_frame_current_selected_output \
  -- --ignored --exact --nocapture
```

After that test passes, produce and strictly read back the current production
evidence. Exit 2 is required because HOST-04 remains open:

```console
set +e
cargo run --locked --bin replay-host-doctor -- run \
  --output "$REPLAY_HOST_OUTPUT" \
  --evidence target/g0-host03-live.json
doctor_status=$?
set -e
test "$doctor_status" -eq 2

cargo run --locked --bin replay-host-doctor -- verify-evidence \
  --evidence target/g0-host03-live.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 pass \
  --require-host04 unproven \
  --validate-extension nvfbc-capture.v1
```

The qualified `DP-0.3` readback binds XRandR output XID `540` at
3840×2160 to NVIDIA GPU `00000000:01:00.0` /
`GPU-becdbf04-4151-31a1-a69e-8d877a1e26b0`, with driver, CUDA, and NvFBC
runtime libraries at `610.43.03`. It records one genuinely new NV12 frame,
12,441,600 bytes with two validated planes, required BGRA→NV12
post-processing, a requested/included/NvFBC-composited cursor whose independent
visibility flag was false, one same-GPU device copy into application-owned
memory, no host or peer copy, and complete reverse cleanup. No pixel data or
native pointer is persisted. Protected/DRM content is outside prototype scope;
DRM remains diagnostic and no protected-content meaning is inferred from the
raw NvFBC status.

## Preserved pre-reboot failure

The original Wayland and NVIDIA version-mismatch state is preserved under
`artifacts/validation/g0/pre-reboot/`. Its create-once index points only to a
contained manifest, which in turn binds the exact archived executable,
evidence bytes, V1 identity, and raw extension digests.

Before any reboot, session change, driver remediation, or later live gate,
verify that immutable archive:

```console
cargo run --locked --bin replay-host-doctor -- verify-archive \
  --index artifacts/validation/g0/pre-reboot/index.json
```

Verification must report `status: "verified"`, the preserved run
`run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`,
and archived binary digest
`9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e`.
It reads the immutable index and contained archive files only; it neither
executes the archived binary nor consults `target/`.

The exact capture transcript, expected exit status, containment checks, and
recorded host facts are in
`.planning/phases/01-host-readiness-gate/01-VALIDATION.md`.

### Permanent compatibility gates

The original decoder fixture and the original live archive are permanent
compatibility inputs:

```console
cargo test --locked g0_v1_foundation_compat -- --exact
cargo test --locked original_pre_reboot_archive_compat -- --exact
```

Plans 01-06, 01-08, and 01-10 must run both named tests at their
integration/live checkpoints. Any plan that changes the base V1 decoder or
the archive verifier must also run both. Plans 01-05, 01-07, and 01-09 add
isolated pure/fixture modules without changing those surfaces, so they do not
rerun this compatibility pair.

Known-extension validators are additional checks; they may not replace the
generic V1 base/raw-extension regression or the original offline archive
verification.

## Diagnostic fixtures

Fixtures exercise the same worker, evaluator, persistence, and readback path:

```console
cargo run --locked --bin replay-host-doctor -- diagnose \
  --fixture tests/fixtures/host01-edge-cases.json \
  --fixture-case positive \
  --evidence target/g0-diagnostic-evidence.json
```

Fixture observations always have `diagnostic` provenance. Even an all-positive
fixture remains `UNPROVEN`, returns exit 2, and cannot be admitted as live G0
evidence. The fixture matrix also covers empty and duplicate observations,
timeouts, malformed or oversized responses, abnormal workers, injected
authority, stale-cache claims, Unicode, and secret/path/native-error
sentinels.

## Exit taxonomy

| Code | Meaning |
|---:|---|
| exit 0 | Command or exact-run verification succeeded. |
| exit 2 | The command completed normally and G0 is `FAIL`. |
| exit 64 | Command-line usage was invalid; no probe or evidence write occurred. |
| exit 70 | An internal hidden worker request failed. |
| exit 74 | Evidence persistence, strict decoding, identity, digest, or currentness verification failed. |

Failures outside a normal G0 decision use bounded JSON error envelopes on
stderr. Raw child stderr, inherited environment values, native error strings,
and operator paths are not copied into public output or persisted evidence.

## Evidence and currentness guarantees

Every run creates its identity in the parent before examining the destination.
The identity binds the current boot and process session, exact argv,
wall-clock and monotonic start/finish times, and SHA-256 of the executing
binary. Runs have bounded per-probe deadlines and a five-minute total
currentness window.

Evidence is serialized and strictly decoded before writing. The store creates
a mode `0600` temporary sibling with create-new and no-follow flags, flushes
and fsyncs it, atomically renames it over the requested regular destination,
fsyncs the parent directory, then reopens without following symlinks and
requires the exact run and byte digest. Parent directories are never created
implicitly.

The foundation/output probes and every verification command retain a
read-only/no-remediation contract. They do not install packages, change
permissions, alter sessions, load modules, reboot, download sources, accept
licenses, invoke encoders, or contact a network. The explicit HOST-03 live
probe additionally creates one bounded NvFBC/CUDA session, captures and copies
exactly one frame on the GPU, and releases every acquired resource in reverse
order. It never persists frame pixels. The requested evidence file and its
owned same-directory temporary sibling are the only durable writes.

## Qualification boundary

HOST-01 through HOST-04 now pass on the qualified current host, and the final
G0 decision is PASS. The standalone HOST-04 probe does not claim that the
pinned Kyber/Kymedia `nv-codec-headers n12.1.14.0` integration has been
rebuilt or qualified; that compatibility boundary remains explicitly
unclaimed. Diagnostic fixtures, cached evidence, and child-supplied claims can
never substitute for a live gate.

## Final HOST-04 and G0 qualification

The operator separately asserted that the supplied official NVIDIA Video
Codec SDK 13.1 source was authentic, lawfully obtained, and accepted outside
this workflow. The authenticated `nvEncodeAPI.h` digest is
`75939d1b11cc3cbe7123c922903f2123644ea167b006207e5965c9fe2eab241a`.
The source-derived oracle required Linux driver major 610 and CUDA Driver API
13010; the live run observed driver `610.43.03`, CUDA 13030, and NVENC runtime
API 13.1.

Run the static, fixture, and live gates separately:

```console
cargo test --locked host04_source_abi_
cargo test --locked host04_policy_
cargo test --locked host04_bitstream_
cargo test --locked --test host_doctor_cli host04_fixture_ -- --nocapture

timeout 900s cargo test --locked --test host_doctor_cli \
  host04_live_g0_current_output -- --ignored --exact --nocapture
```

The live gate produced seven terminal positions. H.264 High 4:2:0 8-bit and
HEVC Main 4:2:0 8-bit were advertised from exact 3840×2160@60 P2/ULL
one-frame attempts using the selected-output application-owned NV12 CUDA
lease. Each advertisement has exact config/resource identity, a closed
same-GPU copy predicate with zero host staging, a parsed forced keyframe
digest, and complete reverse cleanup. The unavailable input formats and
Ampere-ineligible AV1 positions are terminal and non-advertised.

The final evidence was verified with all known extension validators, then
archived through the distinct create-once command:

```console
target/debug/replay-host-doctor archive-post-repair \
  --evidence target/g0-host04-final.json \
  --archive-root artifacts/validation/g0/post-repair

target/debug/replay-host-doctor verify-archive \
  --index artifacts/validation/g0/post-repair/index.json
```

The archived run is
`run-73c9e241c73e6672cf8e6e3169b5d7dc8f77f7d809037c84cd46dbf28d8f93e3`.
Its contained executable digest is
`865632ea0d0d7eecb2954bfa33647765c56cad7f616903e6264ce2f35caca14f`,
evidence digest is
`2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc`,
manifest digest is
`e661f12c9fd6c4e7dbf2a01ece9f6fd52be2fa72c60b5006702364d1f45d9a5f`,
and index digest is
`09ad18c811f8bd169ff9aae6eaa76097425a0ff7c20b8266cb3f2899d64f33c8`.
The generic archive verifier dispatches on the exact pre-reboot or post-repair
schema and does not loosen the original live-FAIL decoder.
