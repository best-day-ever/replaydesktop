---
phase: 01-host-readiness-gate
plan: 07
subsystem: host-readiness
tags:
  - rust
  - nvfbc
  - cuda
  - x11
  - evidence
  - tdd
requires:
  - phase: 01-06
    provides: "Exact selected-output proof bound to one XRandR XID, topology, NVIDIA BDF, and UUID"
provides:
  - "Source-gated NvFBC/CUDA ABI oracle with a complete no-source provider"
  - "Derived one-frame capture lease, copy ledger, cursor/post-processing facts, and reverse cleanup evidence"
  - "Strict diagnostic HOST-03 fixture path that cannot claim live capture or G0 PASS"
affects:
  - 01-08-live-nvfbc-capture
  - 01-09-nvenc-copy-boundary
  - host03-evidence
tech-stack:
  added: []
  patterns:
    - "External native source is enabled only by two explicit absolute roots, authenticated headers, stable digests, and a successful C ABI oracle"
    - "Workers return primitive capture observations; the parent derives leases, copy totals, cleanup, extension status, and the final verdict"
    - "Legacy and no-source paths synthesize an explicit unavailable capture observation without adding a worker deadline"
key-files:
  created:
    - native/nvfbc_abi_oracle.c
    - src/native_nvfbc.rs
    - tests/fixtures/host03-nvfbc-capture.json
    - tests/z_host03_capture_cli.rs
  modified:
    - build.rs
    - src/lib.rs
    - src/model.rs
    - src/probe.rs
    - src/evidence.rs
    - src/currentness.rs
    - tests/host_doctor_cli.rs
key-decisions:
  - "Retain the established ProbeId::NvfbcCapture identifier instead of adding a duplicate Capture variant."
  - "Require both authenticated NvFBC and CUDA roots before compiling the conditional oracle; partial or absent source keeps the provider unavailable."
  - "Diagnostic fixtures may prove the derivation machinery but never produce HOST-03 PASS; the NVENC boundary and overall G0 remain unproven."
  - "Only genuine HOST-03 fixture traces cross the new bounded worker in this plan; live capture remains unavailable until Plan 01-08."
patterns-established:
  - "Opaque surface IDs and typed copy edges replace native pointers or aggregate fixture-owned copy claims."
  - "Every acquired lifecycle resource has one inverse cleanup edge in exact reverse order."
  - "Strict known-extension validation accepts the prior unproven observation payload and the new typed capture evidence without changing the V1 base."
requirements-completed: []
coverage:
  - id: D1
    description: "A scripted selected-output shared-CUDA frame traverses the bounded worker, parent derivation, atomic V1 persistence, exact readback, and strict nvfbc-capture.v1 validation."
    requirement: HOST-03
    verification:
      - kind: e2e
        ref: "tests/z_host03_capture_cli.rs#host03_fixture_tracer_process_and_rejection_cross_full_binary"
        status: pass
    human_judgment: false
  - id: D2
    description: "The lease and copy-ledger evaluator rejects alternate identity, false alias, peer-without-access, host-stage, unknown, disconnected, malformed, stale, cursor, and cleanup failures."
    requirement: HOST-03
    verification:
      - kind: unit
        ref: "src/probe.rs#host03_contract_and_host03_fixture_matrix_exhaust_boundaries"
        status: pass
    human_judgment: false
  - id: D3
    description: "The no-source provider, conditional ABI inventory, and partial-acquisition cleanup state machine are deterministic without proprietary source."
    requirement: HOST-03
    verification:
      - kind: unit
        ref: "src/native_nvfbc.rs#host03_no_source_provider_reports_compiled_gate_unavailable"
        status: pass
      - kind: unit
        ref: "src/native_nvfbc.rs#host03_source_abi_fixture_names_every_future_native_boundary"
        status: pass
      - kind: unit
        ref: "src/native_nvfbc.rs#host03_cleanup_partial_acquisitions_unwind_once_in_reverse_order"
        status: pass
    human_judgment: false
  - id: D4
    description: "Diagnostic provenance, injected authority, original V1 evidence, and the immutable pre-reboot archive remain fail-closed and compatible."
    requirement: HOST-03
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host03_diagnostic_no_source_provider_cannot_claim_capture"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#g0_v1_foundation_compat"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#original_pre_reboot_archive_compat"
        status: pass
    human_judgment: false
duration: 50m 30s
completed: 2026-07-29
status: complete
---

# Phase 1 Plan 7: NvFBC Capture Contract Summary

**Source-gated NvFBC/CUDA ABI contracts and a parent-derived one-frame
lease/copy/cleanup evidence path, exhaustively fixture-proven without claiming
live HOST-03 readiness**

## Performance

- **Duration:** 50m 30s
- **Started:** 2026-07-29T16:00:19Z
- **Completed:** 2026-07-29T16:50:49Z
- **Tasks:** 3/3
- **Files created or modified:** 11

## Accomplishments

- Added an explicit NvFBC source/provider boundary. A no-SDK build remains
  fully fixture-capable and emits typed `SourceUnavailable` evidence without
  loading a native library or making a native call.
- Carried a scripted selected-output capture observation through a bounded
  worker, exact XRandR/GPU identity checks, one-frame lease and copy-ledger
  derivation, atomic evidence persistence, exact-run readback, and strict
  `nvfbc-capture.v1` validation.
- Modeled lifecycle acquisition and reverse exactly-once cleanup, cursor and
  post-processing facts, opaque surface identity, frame freshness and layout,
  and explicit zero-copy/device-copy/peer/host-stage/conversion/unknown edges.
- Exhausted source, binding, copy graph, frame, cursor, timeout, abnormal exit,
  cleanup, injected-authority, pointer/path/secret, and forbidden-fallback
  boundaries while keeping diagnostic HOST-03, HOST-04, and G0 non-PASS.
- Preserved original V1 evidence/archive compatibility and the full
  default-parallel timing suite without weakening any deadline.

## Task Commits

### Task 1: Carry one scripted shared-CUDA frame through production evidence

- `724d160` — TDD RED: failing tracer, evidence, and authority-boundary
  contracts.
- `bdfd0a4` — TDD GREEN: production capture observation, lease/ledger
  derivation, persistence, and strict readback.

### Task 2: Add the conditional source/ABI gate and cleanup state machine

- `44fc4b1` — TDD RED: failing no-source, ABI inventory, and reverse-cleanup
  gates.
- `5469f1e` — TDD GREEN: independent NvFBC/CUDA build gate, C oracle, source
  metadata, and lifecycle evaluator.

### Task 3: Exhaust fixture admission, copy, timeout, and secrecy boundaries

- `32cd8bc` — TDD RED: failing adversarial HOST-03 fixture/process matrix.
- `4a0034c` — TDD GREEN: exhaustive fixture cases and production-binary
  rejection coverage.

### Final verification fix

- `8d5ff0d` — Restored default-parallel process timing while preserving
  exhaustive HOST-03 coverage and all original timeout limits.

## Files Created or Modified

- `native/nvfbc_abi_oracle.c` — official-header layout, constant, function
  pointer, API, and CUDA pointer comparisons for the future native provider.
- `src/native_nvfbc.rs` — provider seam, fixture/no-source providers,
  lifecycle cleanup derivation, and capture lease/copy graph evaluator.
- `src/model.rs` — bounded source, binding, frame, plane, cursor, surface,
  copy-edge, lifecycle, cleanup, and capture-evidence types.
- `build.rs` — independent authenticated NVML and NvFBC/CUDA source gates,
  header digests, conditional oracle compilation, and source metadata.
- `src/probe.rs` — permanent HOST-03 worker protocol and fixture dispatch,
  strict primitive validation, legacy/no-source short path, and direct
  adversarial matrix.
- `src/evidence.rs`, `src/lib.rs` — additive strict extension validation,
  selected-output-bound parent derivation, and diagnostic status policy.
- `tests/fixtures/host03-nvfbc-capture.json` — valid zero/device-copy,
  conversion/post-processing, source/runtime, identity, graph, frame, cursor,
  timeout, cleanup, injection, secrecy, and fallback cases.
- `tests/z_host03_capture_cli.rs` — isolated end-to-end production-binary
  tracer and rejection test.
- `tests/host_doctor_cli.rs` — lightweight diagnostic no-source assertion while
  retaining the legacy process timing suite.
- `src/currentness.rs` — one immutable executable digest per process, reused
  for immediate exact-run readback.

## Decisions Made

- The existing `ProbeId::NvfbcCapture` is the canonical identifier. Adding the
  plan prose's `ProbeId::Capture` would duplicate the HOST-03 concept and break
  the established ordered known-extension contract.
- NvFBC source authentication is all-or-nothing: both explicit absolute roots,
  bounded regular headers, expected declaration markers, stable SHA-256
  digests, and a successful C oracle are required. No guessed or alternate
  header is accepted.
- Primitive traces never carry a verdict, run identity, aggregate copy result,
  cleanup result, native pointer, raw frame, or operator source path. The
  parent derives all authoritative evidence.
- A valid fixture proves only the evaluator and persistence path. Diagnostic
  provenance forces HOST-03 to remain UNPROVEN, and the absent NVENC boundary
  prevents overall G0 PASS.
- `HOST-03` remains pending in `REQUIREMENTS.md`: this plan supplies the
  deterministic pre-live contract, while Plan 01-08 owns actual selected-output
  NvFBC creation and live proof.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Retained the established capture probe identifier**

- **Found during:** Task 1
- **Issue:** The action text requested a new `ProbeId::Capture`, but Plan 01-06
  had already established `ProbeId::NvfbcCapture` in the ordered probe and
  known-extension contracts.
- **Fix:** Extended `NvfbcCapture` in place, avoiding a duplicate probe,
  extension-order drift, and compatibility break.
- **Files modified:** `src/probe.rs`, `src/lib.rs`
- **Verification:** Full suite plus both exact V1/archive compatibility tests.
- **Committed in:** `bdfd0a4`

**2. [Rule 3 - Blocking] Restored default-parallel process timing**

- **Found during:** Final overall verification
- **Issue:** The larger debug executable and an added fourth worker on legacy
  fixtures pushed four existing two-second process checks over their deadline
  under default concurrency.
- **Fix:** Non-HOST-03 fixtures and live/no-source capture now produce the
  deliberate unavailable observation without another worker deadline; the
  immutable `/proc/self/exe` digest is computed once per process; exhaustive
  HOST-03 E2E coverage runs in a separate integration binary.
- **Files modified:** `src/probe.rs`, `src/currentness.rs`,
  `src/evidence.rs`, `tests/host_doctor_cli.rs`,
  `tests/z_host03_capture_cli.rs`
- **Verification:** Default-parallel suite passed with 59 unit, 40 legacy
  process, 9 output-mapping, and 1 HOST-03 E2E test; all original timeout
  limits remain unchanged.
- **Committed in:** `8d5ff0d`

**Total deviations:** Two auto-fixed issues (one Rule 1, one Rule 3). Both
preserve existing compatibility and bounded execution; no live capture,
encoder, archive, documentation, or system-remediation scope was added.

## Authentication and Operator Gates

None. Per operator direction, `REPLAY_NVFBC_SDK_ROOT` and
`REPLAY_CUDA_SDK_ROOT` remained unset. The executor used only the supplied
public NVML header root, did not request proprietary source, did not contact a
Mac, and performed no live NvFBC call or host configuration change.

## Known Stubs

None. `LiveUnavailableCaptureProvider` is the complete, intentional Plan 01-07
no-source behavior, not a placeholder. Plan 01-08 is explicitly responsible
for the source-authenticated native provider and real one-frame proof.

## Validation Results

| Gate | Result |
| --- | --- |
| `cargo test --locked` under default parallelism | 59 unit + 40 legacy process + 9 mapping + 1 HOST-03 E2E passed; 0 failed, ignored, or skipped |
| `cargo test --locked host03_contract_` | 2 passed |
| `cargo test --locked host03_fixture_tracer_` | 1 passed |
| `cargo test --locked nvfbc_extension_` | 1 passed |
| `cargo test --locked host03_no_source_` | 1 passed |
| `cargo test --locked host03_source_abi_fixture_` | 1 passed |
| `cargo test --locked host03_cleanup_` | 2 passed |
| `cargo test --locked host03_fixture_` | 2 passed |
| `cargo test --locked --test host_doctor_cli host03_diagnostic_` | 1 passed |
| `g0_v1_foundation_compat --exact` | passed |
| `original_pre_reboot_archive_compat --exact` | passed |
| `cargo fmt --check` | passed |
| `cargo clippy --locked --all-targets -- -D warnings` | passed |

The proprietary NvFBC source lane was deliberately not compiled because its
root was unavailable and the operator required it to remain unset. This does
not leave a plan verification unrun: the no-source build, declaration
inventory, source-gate logic, and fixture ABI contract are the Plan 01-07
acceptance surface. Actual source compilation and hardware execution belong to
Plan 01-08.

## TDD Gate Compliance

- Task 1 RED `724d160` precedes GREEN `bdfd0a4`.
- Task 2 RED `44fc4b1` precedes GREEN `5469f1e`.
- Task 3 RED `32cd8bc` precedes GREEN `4a0034c`.
- The final regression was reproduced by the failing default-parallel suite
  before fix commit `8d5ff0d`; the same unchanged suite then passed.

## Next Phase Readiness

Plan 01-08 can replace the unavailable provider behind the authenticated
source/ABI boundary and use the same lifecycle, lease, copy ledger, strict
evidence, and process-test contracts for a real selected-output frame.

`HOST-03`, `HOST-04`, and overall G0 intentionally remain non-PASS. A legitimate
NvFBC source root, matching CUDA headers/runtime, and the real Xorg/NVIDIA host
are still required before live proof.

---
*Phase: 01-host-readiness-gate*
*Completed: 2026-07-29*

## Self-Check: PASSED

All four created implementation/test artifacts, the canonical summary, and all
seven task/fix commits were found.
