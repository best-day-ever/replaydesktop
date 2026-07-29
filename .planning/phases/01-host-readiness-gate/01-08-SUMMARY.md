---
phase: 01-host-readiness-gate
plan: 08
subsystem: host-readiness
tags:
  - rust
  - nvfbc
  - cuda-driver-api
  - x11
  - nvidia
  - ffi
  - evidence
requires:
  - phase: 01-07
    provides: "Source-gated capture contracts, bounded worker protocol, strict extension schema, and cleanup/copy evaluators"
  - phase: 01-06
    provides: "Current DP-0.3 XRandR/NV-CONTROL/NVML selected-output proof"
provides:
  - "Authenticated NvFBC 1.9 and CUDA Driver API 13.3 ABI/runtime boundary"
  - "One real 3840x2160 selected-output shared-CUDA frame with explicit conversion, device-copy, cursor, and cleanup evidence"
  - "Strict current-executable HOST-03 admission while HOST-04 and overall G0 remain open"
affects:
  - 01-09-nvenc-copy-boundary
  - 01-10-nvenc-tuple-probes
  - host03-evidence
tech-stack:
  added: []
  patterns:
    - "Proprietary headers remain operator-controlled; builds record exact digests and compare persisted evidence to the source identity compiled into the executable"
    - "Native success status establishes resource ownership; cleanup success events are emitted only after successful native release"
    - "One bounded worker owns CUDA/NvFBC calls and persists metadata only, never frame bytes, pointers, source paths, or native error strings"
key-files:
  created: []
  modified:
    - build.rs
    - native/nvfbc_abi_oracle.c
    - src/native_nvfbc.rs
    - src/probe.rs
    - src/evidence.rs
    - src/lib.rs
    - src/model.rs
    - tests/host_doctor_cli.rs
    - README.md
    - .planning/phases/01-host-readiness-gate/01-VALIDATION.md
key-decisions:
  - "Use CUDA Driver API headers and dynamically loaded libcuda.so.1; libcudart and a new full CUDA Toolkit download are not required on the qualified host."
  - "Treat NvFBC cursor visibility and composition as independent API 1.9 facts; hidden-but-composited is a valid observed state."
  - "Accept a successful opaque NvFBC handle regardless of its numeric bits; the API defines no numeric invalid-handle sentinel."
  - "HOST-03 PASS requires live current provenance, exact output/GPU/topology cross-binding, compiled source digest equality, complete cleanup, and strict readback."
patterns-established:
  - "Authenticated native evidence is executable-bound through exact header digests and the executable SHA-256."
  - "Ambiguous native cleanup remains fail-closed and relies on worker-process containment rather than invented release success."
requirements-completed:
  - HOST-03
coverage:
  - id: D1
    description: "The current DP-0.3 physical X11 output yields exactly one genuine 3840x2160 NV12 NvFBC shared-CUDA frame on the selected NVIDIA GPU."
    requirement: HOST-03
    verification:
      - kind: e2e
        ref: "tests/host_doctor_cli.rs#host03_live_one_frame_current_selected_output"
        status: pass
      - kind: integration
        ref: "target/g0-host03-live.json run-2ac1feff88632a37bf16f31832172bccfdae24c07555c8ae3a37921312610146"
        status: pass
    human_judgment: false
  - id: D2
    description: "The frame records the BGRA-to-NV12 conversion, one same-GPU copy into application-owned memory, independent cursor facts, and complete reverse cleanup without host staging."
    requirement: HOST-03
    verification:
      - kind: unit
        ref: "src/native_nvfbc.rs#host03_cleanup_ and host03_source_abi_frame_layout_freshness_and_cursor_truth_table"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host03_process_"
        status: pass
    human_judgment: false
  - id: D3
    description: "Strict readback rejects fixture authority, substituted source digests, alternate output/GPU/topology identity, native identifiers, and incomplete cleanup."
    requirement: HOST-03
    verification:
      - kind: unit
        ref: "src/native_nvfbc.rs#host03_source_abi_persisted_digests_match_the_current_executable"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host03_process_fixture_cannot_spoof_live_provider_or_leak_native_identifiers"
        status: pass
    human_judgment: false
  - id: D4
    description: "The original foundation fixture and immutable pre-reboot archive remain readable while HOST-04 stays unproven and overall G0 stays FAIL."
    requirement: HOST-03
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#g0_v1_foundation_compat"
        status: pass
      - kind: other
        ref: "replay-host-doctor verify-archive artifacts/validation/g0/pre-reboot/index.json"
        status: pass
    human_judgment: false
duration: 1h 39m 25s
completed: 2026-07-29
status: complete
---

# Phase 1 Plan 8: Live NvFBC Capture Summary

**Authenticated NvFBC 1.9/CUDA Driver API capture of one real DP-0.3 4K frame, with exact source, output, copy, cursor, cleanup, and readback evidence**

## Performance

- **Duration:** 1h 39m 25s
- **Started:** 2026-07-29T20:24:11Z
- **Completed:** 2026-07-29T22:03:36Z
- **Tasks:** 3/3
- **Files created or modified:** 10

## Accomplishments

- Compiled the operator-supplied Capture SDK 9.0 declarations through a complete
  NvFBC 1.9 x86_64 C ABI oracle and bound them to the CUDA Driver API 13.3
  headers already present under `/opt/cuda`.
- Opened the exact `DP-0.3` / XRandR XID 540 / NVIDIA BDF and UUID path, captured
  exactly one fresh 3840x2160 NV12 frame, and copied it once on the same GPU into
  application-owned memory with zero host-staged or peer-copy edges.
- Persisted independent cursor facts (`requested=true`, `included=true`,
  `composited=true`, `visible=false`), required BGRA-to-NV12 post-processing,
  frame dimensions/layout/timing, and complete reverse cleanup.
- Strictly verified live evidence as HOST-01/02/03 PASS and HOST-04 UNPROVEN.
  Overall G0 intentionally remains FAIL until the NVENC boundary is proven.
- Preserved the original foundation fixture and pre-reboot archive contract.

## Task Commits

### Task 1: Verify history, sources, and current HOST-01/HOST-02

- Operator source assertion and read-only preconditions completed; no repository
  commit was required for the checkpoint.

### Task 2: Acquire, validate, and clean up one real selected-output frame

- `4b949a0` — TDD RED: failing NvFBC 1.9 source/ABI gates.
- `fbc816e` — TDD GREEN: authenticated NvFBC 1.9 and CUDA 13.3 ABI.
- `3d42d6f`, `b50739b` — hardened the authenticated header oracle and snapshot
  mode guard.
- `0c869c0` — implemented the real one-frame shared-CUDA provider.
- `16ec41e` — corrected resource ownership and cleanup truth.
- `188e473` — bound live authority to current provenance and exact selected
  output/GPU/topology evidence.
- `f9fa6d8` — modeled cursor visibility and composition independently.
- `0a096f4` — bound persisted source digests to the current executable.
- `30677ef` — retained a warning-free no-source build.

### Task 3: Publish live readback and rerun compatibility

- `dae8cc5` — documented source inputs, commands, and live HOST-03 readback.
- `fd1fd79` — refreshed the final binary-bound evidence identity.

## Live Evidence

| Fact | Qualified result |
| --- | --- |
| Run ID | `run-2ac1feff88632a37bf16f31832172bccfdae24c07555c8ae3a37921312610146` |
| Evidence SHA-256 | `fe6514a6ef9b6d698f00a0d747e4b5768af3acb380bca7f91ea4efa405707db1` |
| Executable SHA-256 | `21840e4f4331bb0022d1ddd2a0b9aad71446e93d23b04fcdecaf7477aef8f5e3` |
| Source identity | `nvidia-nvfbc-api-1.9-cuda-driver-api-13.3` |
| Runtime | `libnvidia-fbc.so.610.43.03`, `libcuda.so.610.43.03` |
| Frame | fresh sequence 1, 3840x2160 NV12, 12,441,600 bytes, two planes, zero missed frames |
| Copy path | one BGRA-to-NV12 conversion plus one same-GPU device copy; zero host/peer copies |
| Cursor | requested/included/composited true; independent visibility false |
| Cleanup | complete reverse release of frame lease, app buffer, session, handle, libraries, and CUDA context |
| Remaining gate | `nvenc-tuples.v1` / HOST-04 is `unproven`; G0 remains FAIL |

## Files Created or Modified

- `build.rs`, `native/nvfbc_abi_oracle.c` — exact source identity, digest,
  declaration, ABI, version, constant, structure, and function-table checks.
- `src/native_nvfbc.rs` — CUDA/NvFBC loading, selected-GPU context, one-frame
  capture/copy, source verification, typed failures, and cleanup state machine.
- `src/probe.rs` — capture-specific bounded worker path and sanitized protocol.
- `src/evidence.rs`, `src/lib.rs`, `src/model.rs` — strict live extension
  admission, cross-binding, schema, cursor/copy/cleanup, and gate derivation.
- `tests/host_doctor_cli.rs` — process, live-hardware, authority, currentness,
  fallback, and persistence regressions.
- `README.md`, `.planning/phases/01-host-readiness-gate/01-VALIDATION.md` —
  exact operator commands, source identity, CUDA requirement, and live readback.

## Decisions Made

- The capture path uses the CUDA Driver API only. On this host no additional
  CUDA download is needed: `cuda.h`/`cudaTypedefs.h` are present and
  `libcuda.so.1` comes from the NVIDIA driver. `libcudart` is not used.
- Successful NvFBC calls establish ownership even if an opaque handle's numeric
  representation is zero. Cleanup follows actual acquired state, never guessed
  sentinel values.
- Cursor visibility does not gate cursor composition. NvFBC 1.9 can report a
  hidden cursor while the configured composition path is active.
- Persisted live source identity is not merely self-described: its three header
  digests must equal those compiled into the currently verified executable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated the fixture-era ABI to official NvFBC 1.9**

- **Found during:** Task 2 source compilation
- **Issue:** Plan 01-07's non-live contract retained API 1.8 assumptions that
  could not safely call Capture SDK 9.0.
- **Fix:** Rebuilt the oracle and Rust FFI around official API `0x109`, exact
  layouts/versions/constants, function table, CUDA pointer types, and runtime
  checks.
- **Committed in:** `fbc816e`, `3d42d6f`, `b50739b`

**2. [Rule 1 - Bug] Removed an invented zero-valued handle rejection**

- **Found during:** First real NvFBC handle creation
- **Issue:** The successful API call returned opaque handle bits of zero; the
  official contract defines no numeric invalid sentinel.
- **Fix:** Ownership now follows successful status and cleanup destroys the
  acquired handle once.
- **Committed in:** `0c869c0`

**3. [Rule 1/2 - Correctness] Preserved native cleanup truth**

- **Found during:** Independent resource-lifecycle review
- **Issue:** Slow successful calls and several failure paths could lose
  ownership, emit release events after failed cleanup, or claim a borrowed
  frame was released after failed synchronization.
- **Fix:** Recorded ownership before deadline rejection, emitted lifecycle
  success only after native success, retained resources for unwind, and kept
  ambiguous failure fail-closed under process containment.
- **Committed in:** `16ec41e`

**4. [Rule 2 - Security] Hardened evidence authority and source binding**

- **Found during:** Adversarial evidence review
- **Issue:** Record-local claims and well-formed substituted header digests
  could otherwise describe a live PASS without matching the current executable.
- **Fix:** Required live envelope provenance, exact selected-output/GPU/topology
  cross-binding, source-authenticated provider authority, and equality with all
  compiled header digests.
- **Committed in:** `188e473`, `0a096f4`

**5. [Rule 1 - Bug] Corrected cursor semantics from live API evidence**

- **Found during:** First frame validation
- **Issue:** The validator invented `composited => visible`, while the real
  API returned `visible=false`, `composited=true`.
- **Fix:** Preserved both booleans independently and added the full truth table
  plus live hidden-but-composited regression.
- **Committed in:** `f9fa6d8`

**6. [Rule 3 - Blocking] Kept the conditional no-source lane warning-free**

- **Found during:** Final strict Clippy run
- **Issue:** A live-only API constant remained compiled but unused without the
  proprietary source roots.
- **Fix:** Gated the constant with the same authenticated-source configuration.
- **Committed in:** `30677ef`

**Total deviations:** Six auto-fixed issues (three Rule 1 correctness bugs,
two Rule 2 security/correctness hardenings, and two Rule 3 blocking fixes; one
issue spans Rule 1/2). All were required to make the native proof honest and
fail-closed; no product-scope feature was added.

## Issues Encountered

- Running the source and no-source full suites concurrently caused four
  pre-existing wall-clock timeout assertions to exceed their bounds under CPU
  contention. The unaffected tests passed, and all four timeout cases passed
  serially with their limits unchanged.
- Strict evidence currentness correctly rejected readback whenever another
  build replaced a shared executable. The reported run was regenerated after
  the final code commits and immediately passed strict executable-bound
  verification.

## Authentication and Operator Gate

The operator explicitly asserted lawful access to the supplied NVIDIA Capture
SDK and acceptance of its applicable terms. The build did not download,
install, vendor, or accept gated material.

## User Setup Required

None on the qualified host. Future build hosts need official `NvFBC.h` plus the
CUDA Driver API headers `cuda.h` and `cudaTypedefs.h`; the runtime uses
`libcuda.so.1` supplied by the installed NVIDIA driver, not `libcudart`.

## Known Stubs

None. HOST-04 is an explicit unproven gate owned by Plans 01-09/01-10, not a
stub or fallback.

## Validation Results

| Gate | Result |
| --- | --- |
| Source-enabled `cargo test --locked --all-targets` | 79 unit tests passed; 40 integration tests passed in the loaded run and all 3 contention-sensitive cases passed serially; mapping/E2E regressions passed |
| No-source `cargo test --locked --all-targets` | 69 unit tests passed; 39 integration tests passed in the loaded run and all 4 contention-sensitive cases passed serially; mapping/E2E regressions passed |
| `host03_source_abi_` | 9 passed |
| `host03_fixture_` | unit fixture matrix and full-binary tracer passed |
| `host03_cleanup_` | 7 passed |
| `host03_process_` | 2 passed |
| ignored live `host03_live_one_frame_current_selected_output` | 1 passed in 2.20 seconds |
| strict production evidence readback | HOST-01/02/03 PASS; HOST-04 UNPROVEN; expected exit 2 |
| `g0_v1_foundation_compat --exact` | passed |
| preserved pre-reboot archive verification | passed; run `run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f` |
| source and no-source Clippy `-D warnings` | passed |
| `cargo fmt --all -- --check` and `git diff --check` | passed |

## TDD Gate Compliance

- RED commit `4b949a0` introduced failing NvFBC 1.9 source gates.
- GREEN commits `fbc816e` and `0c869c0` implemented the authenticated ABI and
  one-frame hardware path.
- All review-discovered regressions were reproduced by targeted tests before
  their fix commits.

## Next Phase Readiness

Plan 01-09 can now start at the proven application-owned NV12 surface and
measure the exact NVENC copy boundary without revisiting X11 output selection,
NvFBC source identity, capture freshness, cursor semantics, or cleanup.

HOST-04 and overall G0 remain intentionally open. No codec tuple or encoder
hardware claim is made by this plan.

---
*Phase: 01-host-readiness-gate*
*Completed: 2026-07-29*

## Self-Check: PASSED

- Every implementation, test, validation, and summary path documented above
  exists, and every listed Plan 01-08 commit resolves in git history.
- The canonical live evidence is
  `run-2ac1feff88632a37bf16f31832172bccfdae24c07555c8ae3a37921312610146`
  with SHA-256
  `fe6514a6ef9b6d698f00a0d747e4b5768af3acb380bca7f91ea4efa405707db1`;
  strict current-executable readback passed.
- ROADMAP, STATE, and REQUIREMENTS consistently advance to Plan 01-09 with
  HOST-03 complete and HOST-04 still pending.
