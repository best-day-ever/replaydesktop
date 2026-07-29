# ReplayDesktop host readiness doctor

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

## Remaining native blockers

This skeleton reports rather than hides the remaining work:

- retain the now-proven real Xorg/NVML HOST-01 foundation and exact
  `DP-0.3` NV-CONTROL/NVML HOST-02 identity;
- retain the now-proven one-frame NvFBC shared-CUDA HOST-03 path, including
  its exact output/GPU binding, conversion/copy ledger, and cleanup;
- open the required NVENC codec/chroma tuples for HOST-04 and record their exact
  capabilities.

HOST-01, HOST-02, and HOST-03 now pass on the current host. HOST-04 remains
`UNPROVEN`, so overall G0 remains FAIL. The encoder proof belongs to the next
Phase 1 plans. Diagnostic fixtures, cached evidence, and child-supplied claims
can never substitute for a live gate.
