---
phase: 01-host-readiness-gate
plan: 05
subsystem: host-readiness
status: complete
tags:
  - rust
  - xrandr
  - drm-kms
  - nvml
  - pci-identity
  - tdd
dependency-graph:
  requires:
    - "01-04: exact local-Xorg/NVML contracts and immutable pre-reboot failure archive"
  provides:
    - "Primary-source output-to-GPU mapping predicate with explicit namespaces, units, and cardinalities"
    - "Pure typed XRandR-to-DRM-to-PCI-to-NVML unique-correlation contract"
    - "Bounded exact SelectedOutputV1 model with stable topology token and proof cardinalities"
    - "Adversarial fixtures for ambiguity, duplication, topology changes, clones, MST, PRIME, and offload"
  affects:
    - "01-06 live selected-output collection and evidence integration"
    - "Later capture geometry, coordinate mapping, and latency telemetry"
tech-stack:
  added: []
  patterns:
    - "Typed disjoint XRandR and DRM identifier newtypes"
    - "SHA-256-only EDID identity with bounded raw-name hex"
    - "Opening/closing RandR, DRM, and NVML topology snapshot token"
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
  - "XRandR XIDs and DRM connector IDs are typed disjoint namespaces and are never compared; identity is proved only through provider membership, EDID digest, connector kind, exact timing, PCI BDF, and NVML UUID."
  - "The canonical PCI representation is lowercase NVML-compatible dddddddd:bb:dd.f, with sysfs domains zero-extended before comparison."
  - "Topology stability includes a complete RandR observation digest in addition to server/config timestamps and DRM/NVML snapshot digests."
  - "NV-CONTROL remains NOT_REQUIRED because RandR, DRM, canonical sysfs ancestry, and NVML close the relation without another identity namespace."
  - "The current Wayland/Xwayland and mismatched-NVML host remains BLOCKED_PRE_REBOOT_XORG; only Plan 01-06 may perform live integration."
requirements-completed: []
coverage:
  - id: D1
    description: "The feasibility spike records authoritative field semantics, namespaces, units, commands, exact cardinalities, and a no-guess pass predicate."
    requirement: HOST-02
    verification:
      - kind: contract
        ref: "tests/output_mapping.rs#host02_mapping_spike_contract_is_explicit_and_fail_closed"
        status: pass
      - kind: environment
        ref: "tests/output_mapping.rs#host02_mapping_spike_current_machine_is_blocked_before_xorg_reboot"
        status: pass
    human_judgment: false
  - id: D2
    description: "A namespace-disjoint XRandR output maps only through one provider, matching EDID/kind/timing, one DRM connector/BDF, and one NVML BDF/UUID."
    requirement: HOST-02
    verification:
      - kind: integration
        ref: "tests/output_mapping.rs#host02_namespace_disjoint_full_relation_passes_without_id_equality"
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
        ref: "tests/output_mapping.rs#host02_mapping_exact_name_origin_and_refresh_round_trip"
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
        ref: "tests/output_mapping.rs#host02_topology_token_change_returns_no_selection"
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

Typed fail-closed XRandR-to-DRM-to-PCI-to-NVML correlation with SHA-256
display identity, exact timing and rational refresh, stable topology snapshots,
and adversarial ambiguity rejection.

## Accomplishments

- Recorded the authoritative RandR, DRM KMS, sysfs, and NVML field semantics,
  namespaces, units, collection bounds, reproducible commands, and exact
  cardinality predicate before implementation.
- Preserved the current reference machine honestly as
  `BLOCKED_PRE_REBOOT_XORG`: Xwayland exposes zero providers and emulated RandR,
  while NVML reports the already archived driver/library mismatch.
- Added distinct Rust newtypes for XRandR output/CRTC/mode/provider XIDs and
  DRM connector IDs, preventing accidental cross-namespace equality.
- Added bounded observation and `SelectedOutputV1` models for exact name bytes,
  optional round-tripped Unicode, signed origin, exact timing, reduced rational
  refresh, EDID digest, separately named DRM/PCI/NVML identity, and proof
  cardinalities.
- Implemented `prove_output_gpu_mapping` as a pure function with no native
  query, enumeration-order preference, approximate refresh, or fallback.
- Replayed a data-driven matrix covering a unique unequal-ID relation,
  identical displays, duplicate EDID, multiple providers/connectors/BDFs/NVML
  matches, missing EDID, conflicting facts, token changes, clones, MST, and
  PRIME/offload.

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

- Numeric equality between an XRandR XID and a DRM connector ID is meaningless.
  The Rust types are incompatible, and the positive fixture deliberately uses
  `73` and `911`.
- XRandR `ConnectorNumber`, DRM `connector_type_id`, and connector display names
  remain separately named evidence. None is treated as a cross-API identity.
- Raw EDID exists only inside the future collector long enough to hash it.
  Models, fixtures, errors, and persisted selections carry only
  `Sha256DigestV1`.
- Canonical PCI BDF uses NVML's eight-digit domain representation. Linux's
  common four-digit sysfs domain is zero-extended before exact comparison.
- Clock-modifying mode flags not supported by the rational derivation fail
  closed rather than being silently ignored.
- Live collection, a second topology read, and selected-output extension
  integration remain solely owned by Plan 01-06.

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

## Deferred Issues

- The extra default-parallel full-suite run intermittently hit Linux
  `ETXTBSY` in the pre-existing
  `archive_archived_binary_tamper_and_path_escape_fail_closed` test. The exact
  test passed immediately in isolation and all 86 tests passed serially.
  Archive code and tests were unchanged. The archive-test harness race is
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

HOST-02's pure mapping and evidence-shape contract is complete: one explicitly
named physical output can produce a deterministic selected-output proof or a
stable blocker without guessing. HOST-02 is not marked complete yet because
Plan 01-06 still owns native Xorg/DRM collection, closing-token recheck, and G0
selected-output evidence integration.

## Next Plan Readiness

Plan 01-06 can implement bounded live collectors directly against this pure
contract. Its live acceptance remains blocked until the host is rebooted into
the intended native Xorg session and the NVIDIA kernel/userspace mismatch is
remediated; fixture success cannot substitute for that live proof.

## Self-Check: PASSED

- The spike, pure mapper, observation fixture, integration tests, deferred-item
  record, and this summary exist at their documented paths.
- All four Task 1 and Task 2 RED/GREEN commits exist in git history.
- The summary carries `status: complete`, and all plan-introduced source files
  pass the documented verification gates.
- The generic V1 decoder, local-Xorg collector, native NVML provider, and
  archive verifier are unchanged by Plan 01-05.
- The one out-of-scope parallel-test race is recorded in both the phase
  deferred-items file and `.planning/WINDOWS.md`.
