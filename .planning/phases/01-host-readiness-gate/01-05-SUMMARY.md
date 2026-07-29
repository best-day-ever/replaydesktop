---
phase: 01-host-readiness-gate
plan: 05
subsystem: host-readiness
status: complete
tags:
  - rust
  - xrandr
  - nv-control
  - nvml
  - pci-identity
  - tdd
dependency-graph:
  requires:
    - "01-04: exact local-Xorg/NVML contracts and immutable pre-reboot failure archive"
  provides:
    - "Primary-source output-to-GPU mapping predicate with explicit namespaces, units, and cardinalities"
    - "Pure typed XRandR-to-NV-CONTROL-display/GPU-to-NVML unique-correlation contract, corrected by Plan 01-06 live evidence"
    - "Bounded exact SelectedOutputV1 model with stable topology token and proof cardinalities"
    - "Adversarial fixtures for target ID zero, ambiguity, duplication, topology changes, MST, providers, and optional DRM diagnostics"
  affects:
    - "01-06 live selected-output collection and evidence integration"
    - "Later capture geometry, coordinate mapping, and latency telemetry"
tech-stack:
  added: []
  patterns:
    - "Typed disjoint XRandR and NV-CONTROL identifier newtypes"
    - "Supplemental SHA-256-only EDID digest with bounded raw-name hex"
    - "Opening/closing RandR, NV-CONTROL, and NVML topology snapshot token plus RandR event count"
    - "Checked reduced rational refresh without floating-point admission"
key-files:
  created:
    - .planning/phases/01-host-readiness-gate/01-OUTPUT-GPU-MAPPING-SPIKE.md
    - src/output_mapping.rs
    - tests/fixtures/host02-output-topologies.json
    - tests/output_mapping.rs
    - .planning/phases/01-host-readiness-gate/deferred-items.md
  modified:
    - src/lib.rs
    - src/model.rs
    - .planning/WINDOWS.md
decisions:
  - "Plan 01-06 corrected the live ownership edge to XRandR output XID → NV-CONTROL display target → enabled X screen → NV-CONTROL GPU → exact NVML BDF+UUID."
  - "The canonical PCI representation is lowercase NVML-compatible dddddddd:bb:dd.f and is derived directly from NV-CONTROL GPU PCI components."
  - "Topology stability includes complete RandR, NV-CONTROL, and NVML digests plus RandR timestamps and a zero-event guard."
  - "DRM connector observations are optional diagnostics only; MST is accepted when the direct source-defined target relation is unique."
  - "The preserved pre-reboot Wayland/NVML-mismatch run remains immutable history; Plan 01-06 separately proved the corrected live Xorg/NV-CONTROL relation."
requirements-completed:
  - HOST-02
coverage:
  - id: D1
    description: "The feasibility spike records authoritative field semantics, namespaces, units, commands, exact cardinalities, and a no-guess pass predicate."
    requirement: HOST-02
    verification:
      - kind: contract
        ref: "tests/output_mapping.rs#host02_mapping_spike_contract_is_explicit_and_fail_closed"
        status: pass
      - kind: environment
        ref: "tests/output_mapping.rs#host02_mapping_fixture_covers_nvcontrol_and_drm_diagnostic_boundaries"
        status: pass
    human_judgment: false
  - id: D2
    description: "A namespace-disjoint XRandR output maps only through one provider, one NV-CONTROL display target, one owning GPU target, and one exact NVML BDF+UUID."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/output_mapping.rs#host02_source_defined_xid_target_gpu_nvml_relation_ignores_shortcuts"
        status: pass
      - kind: integration
        ref: "tests/output_mapping.rs#host02_mapping_fixture_matrix_is_deterministic_and_fail_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "Exact raw-name bytes/Unicode display, negative origin, reduced refresh, bounded fields, and stable blocker codes round-trip without raw EDID."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/output_mapping.rs#host02_mapping_exact_name_origin_timing_and_mst_round_trip"
        status: pass
      - kind: integration
        ref: "tests/output_mapping.rs#host02_mapping_bounds_and_exact_arithmetic_reject_invalid_observations"
        status: pass
    human_judgment: false
  - id: D4
    description: "Changing any opening/closing topology token fact returns no selection, while the generic V1 decoder and original archive remain compatible."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/output_mapping.rs#host02_topology_or_randr_event_change_returns_no_selection"
        status: pass
      - kind: compatibility
        ref: "tests/host_doctor_cli.rs#g0_v1_foundation_compat"
        status: pass
      - kind: compatibility
        ref: "tests/host_doctor_cli.rs#original_pre_reboot_archive_compat"
        status: pass
    human_judgment: false
metrics:
  duration: "30m 16s"
  tasks-completed: 2
  files-changed: 8
  lines-added: 2012
  lines-removed: 6
  completed-date: "2026-07-27"
---

# Phase 1 Plan 5: Output-to-GPU Mapping Contract Summary

Typed fail-closed XRandR-to-NV-CONTROL-to-NVML correlation with exact timing,
rational refresh, stable authoritative snapshots, valid MST support, and
adversarial ambiguity rejection.

## Plan 01-06 Live Correction

The initial Plan 01-05 implementation tested a DRM-mediated hypothesis. The
corrected Xorg host disproved DRM scanout as an NVIDIA X11 ownership oracle:
the exact selected MST output passed the direct NV-CONTROL relation while the
optional DRM diagnostic contained zero active connectors. Plan 01-06 replaced
the model, fixtures, predicate, and live collector accordingly. This summary
describes the current corrected contract; the historical RED/GREEN commit
table below remains an audit record of the original plan execution.

## Accomplishments

- Recorded and then corrected the authoritative RandR, NV-CONTROL, and NVML
  field semantics, namespaces, units, collection bounds, reproducible
  commands, and exact cardinality predicate.
- Preserved the current reference machine honestly as
  `BLOCKED_PRE_REBOOT_XORG`: Xwayland exposes zero providers and emulated RandR,
  while NVML reports the already archived driver/library mismatch.
- Added distinct Rust newtypes for XRandR output/CRTC/mode/provider XIDs and
  NV-CONTROL display/GPU target IDs, preventing accidental cross-namespace
  equality while allowing valid target ID zero.
- Added bounded observation and `SelectedOutputV1` models for exact name bytes,
  optional round-tripped Unicode, signed origin, exact timing, reduced rational
  refresh, supplemental EDID digest, direct display/GPU target identities,
  canonical PCI BDF/UUID, optional DRM diagnostics, and proof cardinalities.
- Implemented `prove_output_gpu_mapping` as a pure function with no native
  query, enumeration-order preference, approximate refresh, or fallback.
- Replayed a data-driven matrix covering unequal and zero target IDs,
  identical displays, duplicate EDID, multiple providers/display targets/GPU
  owners/NVML matches, conflicting facts, token/event changes, MST, actual
  CRTC sharing, and optional/unstable DRM diagnostics.

## Task Commits

| Task | Commit | Result |
|---|---|---|
| 1 RED: spike evidence contract | `599bcb3` | Three tests failed because the source-grounded spike and fixture matrix did not exist. |
| 1 GREEN: feasibility spike | `451424c` | Exact predicate, pre-Xorg blocker, and adversarial observation fixtures passed the tracer and post-commit feedback gate. |
| 2 RED: pure mapping contract | `0d63240` | Tests failed at the missing typed IDs, topology model, selected-output model, stable blockers, and proof function. |
| 2 GREEN: pure mapper | `6509389` | Typed bounded models and full unique correlation passed all mapping, namespace, topology, compatibility, fmt, and clippy gates. |

## Files Created or Modified

- `01-OUTPUT-GPU-MAPPING-SPIKE.md` — source ledger, current-host result,
  namespace/unit inventory, collection procedure, exact predicate, and failure
  matrix.
- `src/model.rs` — additive HOST-02 types appended without changing the V1 base
  or generic envelope decoder.
- `src/output_mapping.rs` — bounded validation, exact refresh reduction, stable
  failure reasons, and unique full-relation proof.
- `src/lib.rs` — exports the pure mapping contract only; no CLI or live
  collector integration.
- `tests/fixtures/host02-output-topologies.json` — base observation plus
  deterministic adversarial overlays.
- `tests/output_mapping.rs` — spike, round-trip, bounds, namespace, topology,
  and complete matrix tests.
- `deferred-items.md` and `.planning/WINDOWS.md` — record the unrelated
  parallel archive-test `ETXTBSY` race found by the extra full-suite run.

## Decisions Made

- Numeric equality between an XRandR XID, an NV-CONTROL target ID, or a DRM
  diagnostic ID is meaningless. The direct edge is attribute-defined, and
  target ID zero is valid.
- XRandR primary state, `ConnectorNumber`, EDID, DRM connector state, and
  enumeration order remain supplemental evidence. None is treated as an
  ownership identity.
- Raw EDID exists only inside the live collector long enough to hash it.
  Models, fixtures, errors, and persisted selections carry only
  `Sha256DigestV1`.
- Canonical PCI BDF uses NVML's eight-digit domain representation and is
  derived with checked formatting from NV-CONTROL GPU PCI components.
- Clock-modifying mode flags not supported by the rational derivation fail
  closed rather than being silently ignored.
- Plan 01-06 owns the completed fixed-SONAME live collection, second topology
  read, RandR event guard, and selected-output extension integration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added the executable mapping test harness**

- **Found during:** Task 1 RED
- **Issue:** The plan named three HOST-02 test filters but did not include a
  Rust integration-test file in either task's file list. The spike and pure
  contract could not be verified without permanent executable assertions.
- **Fix:** Added `tests/output_mapping.rs` with TDD tests for spike completeness,
  raw-EDID exclusion, exact round trips, bounds, every adversarial fixture,
  typed unequal IDs, and topology-token changes.
- **Files modified:** `tests/output_mapping.rs`
- **Commits:** `599bcb3`, `0d63240`, `6509389`

## Authentication and Operator Gates

No authentication gate occurred. All Rust build and test commands used the
preconfigured operator-controlled official NVML header root
`REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include`. No dependency was
installed, no source was downloaded, and no operator path or Xauthority secret
was persisted.

## Known Stubs

None in files created or modified by this plan. Live output collection and G0
selected-output evidence are explicit Plan 01-06 scope, not incomplete code in
the pure Plan 01-05 contract.

## Post-Wave Issue Resolution

- The extra default-parallel full-suite run intermittently hit Linux
  `ETXTBSY` in the pre-existing
  `archive_archived_binary_tamper_and_path_escape_fail_closed` test. The exact
  test passed immediately in isolation and all 86 tests passed serially.
  Post-wave commit `164c365` fixed the test-harness race by serializing copied
  executable creation through the child `exec` handshake. The exact test then
  passed 10 consecutive runs, the parallel archive cluster passed 20 runs, and
  the default-parallel 86-test suite passed three times. The resolution is
  recorded in `deferred-items.md` and `.planning/WINDOWS.md` entry 10.

## Validation Results

| Gate | Result |
|---|---|
| Task 1 `host02_mapping_spike_` tracer and post-commit feedback gate | 3 passed |
| Task 2 `host02_mapping_` | 6 passed |
| `host02_namespace_disjoint_` | 1 passed |
| `host02_topology_token_` | 1 passed |
| Permanent `g0_v1_foundation_compat` | Passed |
| Permanent `original_pre_reboot_archive_compat` | Passed |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| Full locked Rust suite with `--test-threads=1` | 47 unit + 31 process + 8 mapping tests passed; 0 failed, ignored, or skipped |
| Raw EDID / namespace-equality / stub / skipped-test scan | No plan-introduced finding |

## TDD Gate Compliance

- Task 1 RED `599bcb3` precedes GREEN `451424c`; all three tests failed only on
  the absent spike artifacts, and the committed tracer was reverified before
  expansion.
- Task 2 RED `0d63240` precedes GREEN `6509389`; compilation failed only at the
  deliberately missing public mapping API, then every behavior and quality
  gate passed.

## Requirement Coverage

HOST-02's mapping and evidence-shape contract is complete: one explicitly
named physical output can produce a deterministic direct-target proof or a
stable blocker without guessing. Plan 01-06 supplies the fixed-SONAME native
Xorg/NV-CONTROL/NVML collector, event-aware closing-token recheck, G0
integration, and corrected-host live proof.

## Next Plan Readiness

Plan 01-06 implemented the corrected bounded collector and proved the exact
`DP-0.3` MST relation on real Xorg with matching NVIDIA kernel/userspace
`610.43.03`. Fixture success did not substitute for that live proof.

## Self-Check: PASSED

- The spike, pure mapper, observation fixture, integration tests, deferred-item
  record, and this summary exist at their documented paths.
- All four Task 1 and Task 2 RED/GREEN commits exist in git history.
- The summary carries `status: complete`, and all plan-introduced source files
  pass the documented verification gates.
- The generic V1 decoder, local-Xorg collector, native NVML provider, and
  archive verifier are unchanged by Plan 01-05.
- The one out-of-scope parallel-test race is resolved and recorded in both the
  phase deferred-items file and `.planning/WINDOWS.md`.
