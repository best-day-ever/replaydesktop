---
phase: 01-host-readiness-gate
plan: 06
subsystem: host-readiness
tags:
  - rust
  - xrandr
  - nv-control
  - nvml
  - x11
  - mst
  - tdd
requires:
  - phase: 01-05
    provides: "Typed fail-closed output/GPU predicate, exact timing model, and adversarial topology fixtures"
  - phase: 01-04
    provides: "Immutable pre-reboot failure archive and permanent V1 compatibility gates"
provides:
  - "Explicit no-default XRandR output discovery and selected-output.v1 evidence"
  - "Fixed-SONAME XRandR → NV-CONTROL display target → enabled X screen → GPU target → exact NVML BDF+UUID proof"
  - "Source-proven MST support with DRM connector state retained only as optional diagnostics"
  - "Live HOST-01/HOST-02 PASS for DP-0.3 while HOST-03/HOST-04 honestly remain UNPROVEN"
affects:
  - 01-07-nvfbc-capture
  - 01-08-live-capture
  - capture-geometry
  - coordinate-mapping
tech-stack:
  added: []
  patterns:
    - "Fixed system SONAME loading with recorded resolved library identities"
    - "Direct source-defined cross-API identity edges; enumeration order and supplemental metadata never establish ownership"
    - "Opening/closing RandR, NV-CONTROL, and NVML digests plus a zero-RandR-event guard"
key-files:
  created:
    - target/g0-post-reboot-output.json
    - .planning/phases/01-host-readiness-gate/01-06-SUMMARY.md
  modified:
    - src/model.rs
    - src/output_mapping.rs
    - src/local_xorg.rs
    - src/probe.rs
    - src/evidence.rs
    - src/cli.rs
    - tests/fixtures/host02-output-topologies.json
    - tests/output_mapping.rs
    - tests/host_doctor_cli.rs
    - README.md
    - .planning/phases/01-host-readiness-gate/01-OUTPUT-GPU-MAPPING-SPIKE.md
key-decisions:
  - "HOST-02 ownership follows XRandR output XID to an NV-CONTROL display target, enabled X-screen membership, one owning NV-CONTROL GPU, then one NVML device matching both BDF and UUID."
  - "NV-CONTROL display and GPU target ID zero are valid and must not be rejected by XID-style validators."
  - "MST is accepted when the direct source relation is unique; DRM connector state is optional diagnostic data and never enters HOST-01/HOST-02 admission or the authoritative token."
  - "Display targets are enumerated through binary attribute 14; XNVCTRLQueryTargetCount is used for GPU targets only because display-target count raises BadValue on the live driver."
patterns-established:
  - "Every Xmalloc-owned NV-CONTROL string/binary result is bounded, copied, and released with XFree on success and malformed/failure paths."
  - "The selected-output readback validator repeats all cross-field equalities rather than trusting payload construction."
requirements-completed:
  - HOST-02
coverage:
  - id: D1
    description: "The production CLI discovers exact XRandR names without choosing a default and persists one strict selected-output.v1 relation."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host02_discovery_and_nvcontrol_mst_selection_use_the_production_path"
        status: pass
    human_judgment: false
  - id: D2
    description: "The pure predicate requires the direct NV-CONTROL display/GPU ownership chain and exact NVML BDF+UUID while rejecting every missing, duplicate, mismatched, malformed, or changing edge."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/output_mapping.rs#host02_mapping_fixture_matrix_is_deterministic_and_fail_closed"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host02_nvcontrol_cardinality_and_topology_relations_fail_closed_in_process"
        status: pass
    human_judgment: false
  - id: D3
    description: "The corrected real Xorg host proves exact MST output DP-0.3 through target 0/GPU 0 to one matching NVML device while public DRM reports zero connectors."
    requirement: HOST-02
    verification:
      - kind: e2e
        ref: "target/g0-post-reboot-output.json + verify-evidence --require-host01 pass --require-host02 pass --require-host03 unproven --require-host04 unproven --validate-extension selected-output.v1"
        status: pass
    human_judgment: false
  - id: D4
    description: "The original V1 fixture and immutable pre-reboot failure archive remain byte-compatible with the extended doctor."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#g0_v1_foundation_compat"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#original_pre_reboot_archive_compat"
        status: pass
    human_judgment: false
duration: "46m 23s active continuation; operator gate spanned two calendar days"
completed: 2026-07-29
status: complete
---

# Phase 1 Plan 6: Live Selected-Output Proof Summary

**Exact XRandR output `DP-0.3` proven through NV-CONTROL display/GPU target 0
to one matching NVML BDF+UUID, with MST accepted and DRM scanout state
diagnostic only**

## Performance

- **Active continuation duration:** 46m 23s
- **Continuation started:** 2026-07-29T15:03:39Z
- **Completed:** 2026-07-29T15:50:02Z
- **Tasks:** 3/3
- **Files created, modified, or retired:** 23
- **Operator pause:** Plan execution paused across the reboot/Xorg checkpoint;
  no repository work substituted for the physical-host transition.

## Accomplishments

- Carried one explicit `--output` through bounded workers, evaluation, atomic
  V1 persistence, exact-run readback, and strict known-extension validation;
  omission remains discovery-only and never chooses a default.
- Replaced the disproven DRM-mediated identity hypothesis with direct
  fixed-SONAME NV-CONTROL collection and exact XRandR display-target/GPU
  ownership, including target ID zero and complete Xmalloc cleanup.
- Added a zero-event topology guard and deterministic opening/closing RandR,
  NV-CONTROL, and NVML digests. Optional DRM snapshots may be absent, empty, or
  changing without affecting admission.
- Proved live `DP-0.3` at 3840×2160, origin `(0,0)`, XRandR output XID `540`,
  NV-CONTROL display target `0`, GPU target `0`, MST `true`, canonical BDF
  `00000000:01:00.0`, and UUID
  `GPU-becdbf04-4151-31a1-a69e-8d877a1e26b0`.
- Preserved overall G0 FAIL: HOST-01 and HOST-02 pass, while NvFBC HOST-03 and
  NVENC HOST-04 remain unproven.

## Task Commits

### Task 1: Carry one selected output through the production command and V1 extension

- `cb32ed0` — TDD RED: failing selected-output integration tests.
- `c717603` — TDD GREEN: explicit CLI, worker/evaluator path, persistence, and
  strict readback.

### Task 2: Close mapping/process boundaries and extension compatibility

- `f8cf2b9` — TDD RED: failing production process regressions.
- `242b36f` — TDD GREEN: mapping/race/cache/secrecy/compatibility boundaries.

### Task 3: Correct the host contract and prove live HOST-01/HOST-02

- `7359cea` — admitted only bounded authenticated Xorg environment locators.
- `f8fac5f` — TDD RED: failing NV-CONTROL, target-zero, MST, and optional-DRM
  regressions.
- `5190a18` — TDD GREEN: direct NV-CONTROL collector, predicate, token, and
  HOST-01 XRandR output admission.
- `9a573fa` — corrected the spike, plans, state, requirements, and operator
  documentation.
- `99a2045` — aligned strict foundation readback with XRandR physical-output
  remediation.
- `2e42da4` — aligned the preserved Wayland diagnostic fixture's ordered reason
  expectation.

Workflow-only pause/resume commits `dc943ce`, `850b66e`, and `8ef9b24`
preserved and consumed the operator handoff without changing the proof result.

## Live Evidence

The final evidence is `target/g0-post-reboot-output.json`:

| Fact | Persisted value |
| --- | --- |
| Evidence SHA-256 | `d1009f6ea9543b4fbe80955ddba1e01f1559222ad6e3bacc59e6b815806c92a8` |
| Mode / size | `0600` / `5398` bytes |
| Run ID | `run-3d4ba9afa63fed3b2ecc30e2e730d8551f611102f98aa7dc493d6478542c0ddf` |
| Boot / session | `7ee3d78c-c589-4a3a-9bb7-040d5e659fcf` / `sid-280557` |
| Executable SHA-256 | `74250afc04241672ca68676d56869e0a75f790c00afb51d99b38bd67a7b36e55` |
| Linux / NVIDIA | `7.1.4-1-cachyos` / kernel+userspace `610.43.03` |
| Loaded X libraries | `libX11.so.6.4.0`, `libXNVCtrl.so.0.0.0` |
| NV-CONTROL protocol | `1.29` |
| Authoritative token | RandR `b54c8b7e…`, NV-CONTROL `3f4bb8ca…`, NVML `2a858d10…`, RandR events `0` |
| Optional DRM diagnostic | opening/closing connector count `0`, active count `0` |
| Final gate | HOST-01 PASS, HOST-02 PASS, HOST-03 UNPROVEN, HOST-04 UNPROVEN, overall FAIL/exit `2` |

The boot and session differ from the immutable archive's
`be28f2a4-b236-4fbf-b0e2-5a90d0e9641e` / `sid-1968565`. The original archive
still verifies with run
`run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`
and archived executable digest
`9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e`.

## Files Created or Modified

- `src/output_mapping.rs` — fixed-SONAME Xlib/XNVCTRL collector, binary target
  decoder, event-aware topology recheck, optional DRM diagnostics, and exact
  proof/validator.
- `src/model.rs` — typed display/GPU target IDs, NV-CONTROL snapshots, corrected
  selected-output evidence, and authoritative token.
- `src/local_xorg.rs` — authenticated display/screen retention and connected
  active XRandR output admission for HOST-01.
- `src/probe.rs`, `src/cli.rs`, `src/evidence.rs`, `src/lib.rs` — production
  command, worker, extension, readback, and public API integration.
- `tests/fixtures/host02-output-topologies.json`,
  `tests/output_mapping.rs`, `tests/host_doctor_cli.rs` — pure and subprocess
  matrices covering source identity, MST, target zero, optional DRM, topology
  events, timeouts, secrecy, and compatibility.
- `README.md`, `01-VALIDATION.md`, and
  `01-OUTPUT-GPU-MAPPING-SPIKE.md` — exact discovery/selection/readback and
  corrected ownership contract.
- `.continue-here.md` — removed after the resumed checkpoint was fully
  consumed. `.planning/HANDOFF.json` remains absent.

## Decisions Made

- Provider membership proves which provider exposes the XRandR output; it does
  not identify the GPU. The GPU edge comes only from the NV-CONTROL
  connected-display target list.
- `RRGetOutputInfo.clones` is clone capability metadata. Active single-output
  admission is based on the selected CRTC's actual output list.
- XRandR primary state, connector number, EDID, type name, DP GUID, EDID hash,
  and DRM state are recorded only as supplemental evidence.
- The fixed system SONAMEs and resolved `/proc/self/maps` basenames are evidence
  fields; caller-controlled filenames never reach `dlopen`.
- Exact BDF and UUID must match the same NVML record. Matching either field
  alone cannot clear HOST-02.

## Deviations from Plan

### User-Directed Contract Correction

The operator established that public DRM scanout content is irrelevant to this
NVIDIA X11 topology. The corrected live source showed a valid `DP-0.3` MST
branch even though DRM exposed zero active connectors. The original
RandR/EDID/DRM/sysfs ownership hypothesis was therefore replaced with the
direct NV-CONTROL relation. This was an explicit user decision, not an
executor-selected architectural change.

### Auto-fixed Issues

**1. [Rule 1 - Bug] Strict foundation readback still expected the old DRM remediation**

- **Found during:** Full permanent suite after Task 3 GREEN.
- **Issue:** Evaluation emitted `physical XRandR output`, but strict readback
  reconstructed `physical DRM output`, causing valid diagnostic evidence to
  return exit 74.
- **Fix:** Unified evaluation and readback on the XRandR remediation.
- **Files modified:** `src/local_xorg.rs`
- **Verification:** The six previously failing diagnostic/readback tests and
  full default-parallel suite pass.
- **Committed in:** `99a2045`

**2. [Rule 1 - Bug] Preserved Wayland fixture omitted the new XRandR absence reason**

- **Found during:** Full permanent suite after the readback fix.
- **Issue:** The exact ordered reason assertion still reflected the
  DRM-derived coarse-output result.
- **Fix:** Added `PHYSICAL_OUTPUT_REQUIRED` to the fixture expectation.
- **Files modified:** `tests/host_doctor_cli.rs`
- **Verification:** Exact fixture test and full default-parallel suite pass.
- **Committed in:** `2e42da4`

**Total deviations:** One user-directed contract correction and two Rule 1
auto-fixes. All were necessary for truthful identity/readback behavior; no
media, transport, UI, or system-remediation scope was added.

## Authentication and Operator Gates

No authentication gate occurred. The physical reboot/real-Xorg checkpoint was
satisfied before continuation. The executor only performed bounded read/query
operations using the provided Xauthority locator and official NVML header root;
it did not reboot, modeset, edit X configuration, contact the Mac, install a
package, or persist authorization bytes.

## Known Stubs

None. No skipped tests, TODO/FIXME implementation paths, empty UI data sources,
or unrun verification commands remain in plan-owned files.

## Validation Results

| Gate | Result |
| --- | --- |
| `cargo test --locked` with official NVML header root | 52 unit + 39 process + 9 mapping tests passed; 0 failed, ignored, or skipped |
| `cargo test --locked --test host_doctor_cli host02_` | 6 passed |
| `g0_v1_foundation_compat --exact` | passed |
| `original_pre_reboot_archive_compat --exact` | passed |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | passed |
| immutable `verify-archive` | `status=verified`, original run/digest unchanged |
| `git diff --exit-code 8ef9b24 -- artifacts/validation/g0/pre-reboot` | passed |
| exact live doctor | exit `2`; fresh evidence written mode `0600` |
| exact live persisted readback | HOST-01 PASS, HOST-02 PASS, HOST-03/HOST-04 UNPROVEN, selected-output strict validation passed |
| stale DRM-authority/MST-rejection documentation scan | no current-contract claim remains |

## TDD Gate Compliance

- Task 1 RED `cb32ed0` precedes GREEN `c717603`.
- Task 2 RED `f8cf2b9` precedes GREEN `242b36f`.
- The user-directed live-contract correction also preserved RED
  `f8fac5f` before GREEN `5190a18`.
- All three RED commits failed on the deliberately absent behavior/API, and
  every corresponding GREEN plus final quality gate passes.

## Threat Surface

The new local Xlib/XNVCTRL FFI surface is covered by the plan's
XRandR/NV-CONTROL/NVML trust boundary and spoofing mitigation: fixed SONAMEs,
authenticated exact screen, minimum protocol 1.27, bounded strings/lists,
paired `XFree`, source-defined target edges, duplicate rejection, exact
BDF+UUID, strict readback, and opening/closing/event stability. No unregistered
network, authentication, file-write, or schema trust boundary was introduced.

## Next Phase Readiness

Plan 01-07 can consume one durable, live-proven selected output/GPU identity.
The active blockers are intentionally HOST-03 NvFBC shared-CUDA capture and
HOST-04 NVENC tuples; no HOST-01/HOST-02 or DRM-scanout remediation remains.

## Self-Check: PASSED

- Every documented source, fixture, contract, summary, and live evidence file
  exists.
- Every Task 1/2 and corrected Task 3 commit exists in git history.
- The pre-reboot archive is byte-unchanged from `8ef9b24`.
- `.continue-here.md` is consumed and `.planning/HANDOFF.json` is absent.
- No current document asserts DRM ownership authority or rejects
  source-proven MST.
