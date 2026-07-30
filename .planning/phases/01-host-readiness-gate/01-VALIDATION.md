# Phase 01 Host Readiness Validation

This guide records the immutable pre-reboot HOST-01 failure captured on
2026-07-27. The capture was read-only: it did not reboot, change sessions,
repair drivers, load modules, alter permissions, install software, or contact
a network. The only durable writes were the evidence file and the archive
root.

## Original-host precondition

Immediately before capture, read-only checks reported:

- active local session type: `wayland`;
- loaded NVIDIA kernel module: `610.43.02`;
- resolved NVML userspace library:
  `/usr/lib/libnvidia-ml.so.610.43.03`;
- `nvidia-smi` exit `18`, reporting a driver/library mismatch.

No fixture evidence was substituted for these live facts.

## Fail-closed preflight correction

An initial preflight capture exposed an archive validation bug before any
archive index or run directory was committed: Cargo had hard-linked
`target/debug/replay-host-doctor` to its hashed build artifact, while the
archive source check incorrectly required `/proc/self/exe` to have exactly one
link. The empty archive root was confirmed before continuing.

The source check was corrected to accept a regular executable reached through
the already-open `/proc/self/exe` descriptor regardless of hard-link count.
The evidence source still requires exactly one link. A process regression now
creates a Cargo-style executable hard link and proves descriptor-only
copying/hashing succeeds. The doctor was rebuilt after that correction, and a
new final live run was used for the archive below; the rejected preflight
evidence was not archived.

## Build and controlled live capture

The doctor was built once with the operator-provided official NVML header
root:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo build --locked --bin replay-host-doctor
```

The exact built executable had SHA-256
`9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e`.
The live run used a fresh path and captured the expected nonzero result before
any archive operation:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor run \
    --evidence target/g0-pre-reboot-live-01-04-archived.json \
    --probe-timeout-ms 2000
live_status=$?
set -e
test "$live_status" -eq 2
```

Result: exit `2`, provenance `live`, overall status `fail`, run ID
`run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`,
boot ID `be28f2a4-b236-4fbf-b0e2-5a90d0e9641e`, and process session
`sid-1968565`.

The `host-foundation.v1` payload was independently checked before archival:

| Fact | Preserved value |
|---|---|
| local session reason | `SESSION_NOT_XORG` |
| NVIDIA kernel version | `610.43.02` |
| NVML userspace version | `610.43.03` |
| NVIDIA mismatch reason | `NVIDIA_VERSION_MISMATCH` |
| NVML initialization | failed closed, `NVML_INITIALIZATION_FAILED` |

## Create-once archive

No build or source change occurred between the live run and the successful
archive copy. The archive command was then invoked against the live evidence:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor archive-pre-reboot \
    --evidence target/g0-pre-reboot-live-01-04-archived.json \
    --archive-root artifacts/validation/g0/pre-reboot
```

It reported `status: "archived"` and wrote the create-once index
`artifacts/validation/g0/pre-reboot/index.json`. The index contains the
relative manifest path:

```text
run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/manifest.json
```

Recorded digests:

| Contained object | SHA-256 |
|---|---|
| archived executable | `9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e` |
| archived evidence | `5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61` |
| manifest | `d4d16fc2bee5960cedd01881e3fcc8cd1496ed014ceadb88fe67f288490ee136` |
| index | `66d6fbf707f39781bed50c21eaf201be7d3a63f64bb1e96e0cf320ca14435c3c` |

The committed run directory is mode `0500`; its executable is mode `0500`;
the evidence, manifest, and index are each mode `0400`. All manifest file paths
are relative single components inside that run directory.

## Mandatory verification before remediation

Run this before every reboot, session change, driver repair, or later live
gate:

```console
cargo run --locked --bin replay-host-doctor -- verify-archive \
  --index artifacts/validation/g0/pre-reboot/index.json
```

The immediate post-archive result was exit `0`, `status: "verified"`,
`evidence_status: "fail"`, and the archived executable digest shown above.
The verifier reads and hashes only the immutable index and contained archived
files. It does not execute the archived binary and does not read the current
build target.

The structural regression is:

```console
cargo test --locked pre_reboot_archive_manifest_path_digest_schema -- --exact
```

It verifies the relative two-component manifest pointer, index-to-manifest
digest, schemas, provenance, FAIL status, and the complete offline archive
verification path.

## Corrected-host HOST-01/HOST-02 procedure

The immutable archive verification above must succeed first. The only manual
host action is for the operator to reboot into an installed kernel matching
the NVIDIA userspace driver and log into one active local physical Xorg
session. The doctor does not repair the driver, select a session, reboot,
install packages, change permissions, or alter display state.

After that operator transition, build with the approved official NVML header
root:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo build --locked --bin replay-host-doctor
```

First run discovery with no `--output`. Exit 2 is required; omission never
selects a default:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor run \
    --evidence target/g0-post-reboot-output-discovery.json
discovery_status=$?
set -e
test "$discovery_status" -eq 2
```

Enumerate exact candidate names from the persisted `selected-output.v1`
payload:

```console
jq -r '.extensions[]
  | select(.id == "selected-output.v1")
  | .payload.candidates[]
  | select(.display != null)
  | .display' target/g0-post-reboot-output-discovery.json
```

The operator chooses one exact listed XRandR name. Do not select by array
position or connector numbering:

```console
export REPLAY_HOST_OUTPUT='DP-0.3'
```

Run the selected relation and require the honest nonzero result:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor run \
    --output "$REPLAY_HOST_OUTPUT" \
    --evidence target/g0-post-reboot-output.json
selected_status=$?
set -e
test "$selected_status" -eq 2
```

Verify only the persisted readback. HOST-01 and HOST-02 must pass, while
HOST-03 and HOST-04 remain unproven and the overall G0 remains FAIL:

```console
cargo run --locked --bin replay-host-doctor -- verify-evidence \
  --evidence target/g0-post-reboot-output.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 unproven \
  --require-host04 unproven \
  --validate-extension selected-output.v1
```

The selected proof joins the exact XRandR output, CRTC, mode, provider,
geometry and timing through `NV_CTRL_DISPLAY_RANDR_OUTPUT_ID` to one
NV-CONTROL display target, its enabled X-screen membership, one owning
NV-CONTROL GPU target, and one current NVML device matching both canonical PCI
BDF and NVML UUID. MST is admitted when this source-defined chain is unique.
DRM is optional diagnostic data and is excluded from HOST-01/HOST-02
admission and the authoritative topology token.

The collector rechecks XRandR, NV-CONTROL, NVML, and queued RandR events before
admission. Missing, ambiguous, malformed, cached, raced, actual multi-output
CRTC, timeout, or secret-bearing observations fail closed. Raw EDID, authority
files, SDK paths, and native error strings are never evidence.

The corrected evidence must also identify a different boot and session from
the preserved archive (`be28f2a4-b236-4fbf-b0e2-5a90d0e9641e` and
`sid-1968565`). A repeated original identity is not post-reboot proof.

## Authenticated sources and live HOST-03 proof

The operator separately asserted that the official NVIDIA Capture SDK source
was lawfully available and its applicable terms had been accepted. Automated
path, digest, and ABI checks did not substitute for that assertion. The source
archive and authenticated declarations used for this run were:

| Object | Location / identity | SHA-256 |
|---|---|---|
| Capture SDK archive | `/home/finn/Downloads/SW_36244718.0_SW-apps_Release_Linux_AMD64_CaptureSDK.tgz` | `31e0d8f2e2fe94fab1f9cdb76b85f5f343692a6323edd732a9aad761814697f5` |
| NvFBC 1.9 header | `capture-linux-v9.0.0-31e0d8f2e2fe94fa/include/NvFBC.h` | `b079b8d672e9ef34e358ded5e40592971ef290a972ccb831f46b38c691ed9ee1` |
| CUDA Driver API 13.3 header | `/opt/cuda/targets/x86_64-linux/include/cuda.h` | `31df84e16179b6d97db4b3c0bae7697392a370b41983f4a8962f0e5a8069b577` |
| CUDA typedef header | `/opt/cuda/targets/x86_64-linux/include/cudaTypedefs.h` | `30d517cfa051f7a498e432eb1a1964abb11c1cd32098125bba3dfef0b059381f` |

No additional CUDA library download was required. ReplayDesktop uses the CUDA
Driver API declarations in `cuda.h` and `cudaTypedefs.h`; at runtime it loads
`libcuda.so.1` from the installed NVIDIA display driver. It does not link
`libcudart`, use CUDA samples, or require a full Toolkit installation for this
capture proof. NVENC headers are a separate, pinned Plan 01-09/01-10 input.

The authenticated build and test environment was:

```console
export REPLAY_NVFBC_SDK_ROOT=/home/finn/.local/share/replaydesktop/nvidia-capture-sdk/capture-linux-v9.0.0-31e0d8f2e2fe94fa/include
export REPLAY_CUDA_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include
export REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include
export REPLAY_HOST_OUTPUT=DP-0.3
export DISPLAY=:0
export XAUTHORITY=/run/user/1000/xauth_LAgRuP
export XDG_SESSION_ID=3
export XDG_SESSION_TYPE=x11
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
```

The X authority and session values are observations from this qualification
run, not stable configuration; rediscover them from the active local Xorg
session after a logout or reboot.

The source/ABI, fixture, cleanup, and process gates passed before live capture:

```console
cargo test --locked host03_source_abi_
cargo test --locked host03_fixture_
cargo test --locked host03_cleanup_
cargo test --locked --test host_doctor_cli host03_process_
```

The independent hardware test then obtained one selected-output frame under
both an outer process timeout and the provider's shorter native deadline:

```console
timeout 180s cargo test --locked --test host_doctor_cli \
  host03_live_one_frame_current_selected_output \
  -- --ignored --exact --nocapture
```

Result: one test passed in 2.20 seconds. The provider performed exactly one
fresh-frame grab with a 750 ms native timeout and no retry or fallback.

The final production readback was created with:

```console
set +e
cargo run --locked --bin replay-host-doctor -- run \
  --output "$REPLAY_HOST_OUTPUT" \
  --evidence target/g0-host03-live.json
live_status=$?
set -e
test "$live_status" -eq 2

cargo run --locked --bin replay-host-doctor -- verify-evidence \
  --evidence target/g0-host03-live.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 pass \
  --require-host04 unproven \
  --validate-extension nvfbc-capture.v1
```

The persisted evidence has SHA-256
`fe6514a6ef9b6d698f00a0d747e4b5768af3acb380bca7f91ea4efa405707db1`
and run ID
`run-2ac1feff88632a37bf16f31832172bccfdae24c07555c8ae3a37921312610146`.
Its live `nvfbc-capture.v1` extension records:

| Fact | Current qualified value |
|---|---|
| source identity | `nvidia-nvfbc-api-1.9-cuda-driver-api-13.3` |
| runtime libraries | `libnvidia-fbc.so.610.43.03`, `libcuda.so.610.43.03` |
| selected output | `DP-0.3`, XRandR XID `540`, 3840×2160 at origin 0,0 |
| selected GPU | `00000000:01:00.0`, `GPU-becdbf04-4151-31a1-a69e-8d877a1e26b0` |
| frame | sequence 1, fresh, NV12, 3840×2160, 12,441,600 bytes, two validated planes |
| capture processing | selected scanout BGRA → NvFBC shared-CUDA NV12; `required_post_processing=true`, `direct_capture=false` |
| cursor | requested, included, and NvFBC-composited; the independent visibility flag was `false` |
| application-visible copy | one same-GPU device-to-device copy to application-owned NV12; zero host-staged or peer-copy edges |
| cleanup | frame release, application buffer free, session/handle destruction, library unload, CUDA context/library release; complete in reverse order |
| encoder boundary | `unproven` |

The evidence carries typed identities, sizes, formats, counts, timestamps,
digests, and lifecycle events only. It persists no frame bytes, native pointer,
operator source path, native error string, or inferred protected-content
meaning. Protected/DRM content is outside the prototype requirement; DRM
remains optional diagnostic data and cannot affect admission.

The overall run intentionally remained `fail` with one reason only:
`nvenc-tuples.v1 unproven`. HOST-03 is therefore complete without making an
NVENC or G0 PASS claim.

## Permanent compatibility obligation

The two stable regressions are:

```console
cargo test --locked g0_v1_foundation_compat -- --exact
cargo test --locked original_pre_reboot_archive_compat -- --exact
```

`g0_v1_foundation_compat` decodes the original Plan 01-01 fixture, preserves
its unchanged V1 base, and proves the unknown `future-display-proof.v2` raw
payload and payload digest survive re-encoding. The original-archive test
reads only the immutable index, contained manifest, archived executable, and
archived evidence. It verifies their fixed digests, strict V1 base identity,
and every raw extension payload digest without executing the archived binary
or consulting `target/`.

Plans 01-06, 01-08, and 01-10 must run both named tests before their
integration/live acceptance. Any plan that changes the base V1 decoder or
archive verifier has the same obligation. Plans 01-05, 01-07, and 01-09 are
isolated pure/fixture expansions that do not change those surfaces and do not
rerun this pair.

Known-extension validators are additional checks; they may not replace either
the generic base/raw-extension compatibility test or the immutable archive
verification.

## Final HOST-04 and G0 qualification

The operator separately asserted that the supplied NVIDIA Video Codec SDK
13.1 source was authentic official NVIDIA material, lawfully obtained, and
accepted outside this workflow. Automated path, digest, and ABI checks did not
substitute for that assertion. The authenticated `nvEncodeAPI.h` SHA-256 is
`75939d1b11cc3cbe7123c922903f2123644ea167b006207e5965c9fe2eab241a`.
The standalone oracle derives every used native type/layout/GUID/enum,
structure version, API encoding, and compatibility fact from that header.

The static and fixture gates are separate from the live hardware timeout:

```console
export REPLAY_NVENC_SDK_ROOT=<official-sdk-13.1-include-root>
export REPLAY_NVFBC_SDK_ROOT=<authorized-capture-sdk-include-root>
export REPLAY_CUDA_SDK_ROOT=<cuda-driver-api-include-root>
export REPLAY_NVML_SDK_ROOT=<nvml-include-root>
export REPLAY_HOST_OUTPUT=DP-0.3

cargo test --locked host04_source_abi_
cargo test --locked host04_policy_
cargo test --locked host04_bitstream_
cargo test --locked --test host_doctor_cli host04_fixture_ -- --nocapture
```

The independent live gate is:

```console
timeout 900s cargo test --locked --test host_doctor_cli \
  host04_live_g0_current_output -- --ignored --exact --nocapture
```

Result: one test passed. It re-proved HOST-01, selected-output HOST-02, and
NvFBC/CUDA HOST-03 before allocating NVENC resources. All seven policy
positions were terminal. H.264 High 4:2:0 8-bit and HEVC Main 4:2:0 8-bit
succeeded and were advertised. The three lease-incompatible HEVC 10-bit/4:4:4
positions were unsupported, and both AV1 positions were generation-ineligible
on the Ampere host. No tuple silently substituted another input format,
output, GPU, context, codec, profile, or software path.

Each advertised attempt records exact 3840×2160 at 60/1, P2, ultra-low-latency
tuning, synchronous Linux encode, PTD, forced IDR plus parameter sets, zero B
frames/lookahead/reorder, one-frame VBV, the exact application-owned NV12
lease and topology/GPU identity, closed copy edges with no host staging, a
bounded parsed keyframe digest, and complete reverse cleanup. The parser
accepts valid Annex-B trailing zero bytes and reads only a bounded slice-header
prefix from large NVENC IDRs; the full bitstream remains capped, hashed, and
cleared without persistence.

The final production run and same-binary readback were:

```console
target/debug/replay-host-doctor run \
  --output "$REPLAY_HOST_OUTPUT" \
  --evidence target/g0-host04-final.json \
  --probe-timeout-ms 60000

target/debug/replay-host-doctor verify-evidence \
  --evidence target/g0-host04-final.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 pass \
  --require-host04 pass \
  --validate-extension host-foundation.v1 \
  --validate-extension selected-output.v1 \
  --validate-extension nvfbc-capture.v1 \
  --validate-extension nvenc-tuples.v1
```

Both commands exited 0. The final run ID is
`run-73c9e241c73e6672cf8e6e3169b5d7dc8f77f7d809037c84cd46dbf28d8f93e3`
and the evidence SHA-256 is
`2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc`.

The current result was archived separately without modifying
`artifacts/validation/g0/pre-reboot/`:

```console
target/debug/replay-host-doctor archive-post-repair \
  --evidence target/g0-host04-final.json \
  --archive-root artifacts/validation/g0/post-repair

target/debug/replay-host-doctor verify-archive \
  --index artifacts/validation/g0/post-repair/index.json
```

The post-repair index, manifest, evidence, and contained executable use modes
0400, 0400, 0400, and 0500 respectively; the run directory is 0500. Recorded
digests:

| Contained object | SHA-256 |
|---|---|
| archived executable | `865632ea0d0d7eecb2954bfa33647765c56cad7f616903e6264ce2f35caca14f` |
| archived evidence | `2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc` |
| post-repair manifest | `e661f12c9fd6c4e7dbf2a01ece9f6fd52be2fa72c60b5006702364d1f45d9a5f` |
| post-repair index | `09ad18c811f8bd169ff9aae6eaa76097425a0ff7c20b8266cb3f2899d64f33c8` |

The original foundation fixture and immutable pre-reboot archive both verified
with the final code. The verifier dispatches on the exact index schema and
requires the matching manifest schema, so post-repair PASS support cannot
loosen the original pre-reboot live-FAIL-only decoder.

This plan proves the standalone official SDK 13.1 HOST-04 path only. The
Kyber/Kymedia dependency remains pinned to
`nv-codec-headers n12.1.14.0`; compatibility of that pinned integration with
the standalone probe is unclaimed and no Kyber tree or lockfile was changed.
