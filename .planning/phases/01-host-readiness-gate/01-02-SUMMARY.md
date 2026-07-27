---
phase: 01-host-readiness-gate
plan: 02
subsystem: host-readiness
status: complete
tags:
  - rust
  - cli
  - bounded-workers
  - atomic-evidence
  - currentness
  - tdd
dependency-graph:
  requires:
    - "01-01: strict G0EvidenceEnvelopeV1, version dispatcher, extension IDs, and SHA-256 APIs"
  provides:
    - "Production replay-host-doctor run/diagnose/verify-evidence command surface"
    - "Parent-owned run identity and current boot/session/binary/time verification"
    - "Bounded self-worker protocol with diagnostic-only fixtures"
    - "Mode-0600 atomic evidence persistence and exact-run readback"
  affects:
    - "01-03 through 01-10 native host proof providers"
    - "Phase 1 immutable evidence archives and final G0 decision"
tech-stack:
  added: []
  patterns:
    - "Explicit no-dependency CLI grammar with sysexits-style stable codes"
    - "Parent-owned authority with child-only primitive observations"
    - "Direct-argv self-workers with cleared environments and bounded streams"
    - "Same-directory create-new/fsync/rename/fsync evidence commits"
key-files:
  created:
    - README.md
    - src/main.rs
    - src/cli.rs
    - src/currentness.rs
    - src/evidence.rs
    - src/probe.rs
    - tests/host_doctor_cli.rs
    - tests/fixtures/host01-edge-cases.json
  modified:
    - src/lib.rs
decisions:
  - "Only the parent may assign run identity, provenance, extension status, currentness, or the G0 verdict."
  - "Fixture observations traverse production evaluation and persistence but can only produce diagnostic FAIL/UNPROVEN evidence."
  - "Evidence is accepted only after strict V1 decoding, byte-identical readback, exact run/binary identity, and currentness verification."
  - "Live native facts remain explicitly unavailable until their owning Phase 1 plans implement source-backed probes."
metrics:
  duration: "28m 32s"
  tasks-completed: 3
  files-changed: 9
  lines-added: 3162
  completed-date: "2026-07-27"
coverage:
  requirements:
    - id: HOST-01
      contribution: "Establishes the fail-closed command, isolation, currentness, and evidence path; native Xorg/NVML proof remains pending Plan 01-03."
      completion: pending-native-proof
  must-haves:
    - "Fresh production runs persist and read back one current V1 envelope with stable exit 2 while native facts are unavailable."
    - "Diagnostic fixtures cannot acquire live provenance or parent-owned admission authority."
    - "Evidence writes are mode 0600, same-directory, create-new, fsynced, atomically renamed, and exact-run verified."
    - "Workers have direct argv, cleared environments, per-probe deadlines, bounded streams, strict responses, and kill/wait cleanup."
    - "README documents the exact command and makes no native readiness or G0 PASS claim."
---

# Phase 1 Plan 2: Host Doctor Walking Skeleton Summary

Fail-closed Rust host doctor with parent-owned currentness, bounded self-workers,
and mode-0600 atomic V1 evidence readback.

## Accomplishments

- Shipped `run`, `diagnose`, and `verify-evidence` through one explicit command
  grammar with stable exits 0, 2, 64, 70, and 74.
- Bound every fresh run to the current boot, process session, exact argv,
  executing-binary SHA-256, and wall/monotonic start and finish times.
- Implemented strict same-directory evidence persistence with create-new,
  no-follow, mode `0600`, file fsync, atomic rename, directory fsync, bounded
  decode, byte-digest comparison, and exact-run readback.
- Isolated live and fixture probes behind direct-argv self-workers with cleared
  environments, nonces, strict request/response envelopes, independent
  deadlines, bounded stdout/stderr, and kill/wait cleanup.
- Added a diagnostic fixture matrix and process tests for timeouts, malformed
  and oversized output, child crashes, duplicate observations/responses,
  injected authority, stale claims, Unicode, secret sentinels, filesystem
  failures, stale destinations, and unknown V1 extensions.
- Published the exact locked development command, verification flow, exit
  taxonomy, evidence guarantees, diagnostic policy, read-only boundary, and
  open native blockers.

## Task Commits

Each task followed a committed RED/GREEN cycle:

| Task | RED | GREEN | Result |
|---|---|---|---|
| 1. Production command through decision, persistence, and readback | `86a712a` | `d80e299` | Fresh live execution writes current FAIL/UNPROVEN V1 evidence and reads back the exact run. |
| 2. Bounded workers and fixture admission | `e4fe824` | `c4ac7ab` | Primitive-only child protocol fails closed under every bounded fault and never grants fixtures live authority. |
| 3. Process contract and honest operator skeleton | `e250306` | `5431cc6` | All commands/exits, filesystem boundaries, currentness, unknown extensions, and documentation are process-tested. |

## Files Created or Modified

- `src/main.rs` — thin UTF-8-aware binary entry returning `DoctorExit`.
- `src/cli.rs` — explicit command grammar, public usage, and stable exit/error
  envelopes.
- `src/lib.rs` — parent orchestration, one G0 evaluator, ordered reasons,
  persistence/readback, and public result JSON.
- `src/currentness.rs` — run identity capture and current
  boot/session/executable/time verification.
- `src/evidence.rs` — strict mode-0600 atomic store and exact-run readback.
- `src/probe.rs` — live/fixture backends, bounded self-worker runner, strict
  protocol, and sanitized typed failures.
- `tests/host_doctor_cli.rs` — production process tests and diagnostic
  admission/fault coverage.
- `tests/fixtures/host01-edge-cases.json` — bounded diagnostic-only fixture
  matrix.
- `README.md` — operator commands, guarantees, boundaries, and open blockers.

## Decisions Made

- Parent authority is non-delegable: child output contains only a nonce-bound
  primitive observation. The parent derives provenance, statuses, reasons,
  currentness, identity, and the overall verdict.
- Diagnostic mode uses the production worker/evaluator/store/readback path but
  is permanently inadmissible for live G0 PASS.
- Unknown bounded V1 extensions remain byte-preserved and verifiable without
  changing the Plan 01-01 base schema or making known-extension validation
  implicit.
- Native Xorg, NVML, selected-output, NvFBC, and NVENC proofs stay honest
  `UNPROVEN` facts until their source-backed providers arrive in later plans.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Guaranteed cleanup on worker setup errors**

- **Found during:** Task 3 process-contract review
- **Issue:** After spawning, missing piped handles could return through `?`
  without an explicit kill/wait, and deadline overflow was checked after spawn.
- **Fix:** Validate the deadline before spawn and terminate/reap the child on
  either missing-pipe setup path; timeout and wait-error paths share the same
  cleanup helper.
- **Files modified:** `src/probe.rs`
- **Commit:** `5431cc6`

**2. [Rule 1 - Bug] Made run currentness contain the advertised probe bounds**

- **Found during:** Task 3 process-contract review
- **Issue:** Four sequential maximum 60-second probe deadlines could exceed the
  prior 60-second whole-run currentness limit and turn an otherwise valid
  fail-closed run into a persistence error.
- **Fix:** Set the default whole-run duration to five minutes and added a unit
  test proving it contains all four maximum probe deadlines without exceeding
  the evidence age window.
- **Files modified:** `src/currentness.rs`
- **Commit:** `5431cc6`

## Authentication Gates

None.

## Known Stubs

| File | Line | Stub | Reason |
|---|---:|---|---|
| `src/probe.rs` | 429 | Live workers return `native-probe-not-implemented`. | Intentional Plan 01-02 boundary: Plans 01-03 through 01-10 replace this with source-backed Xorg/NVML/output/NvFBC/NVENC proof. It does not prevent this plan's fail-closed skeleton goal. |

The stub is recorded as open entry 4 in `.planning/WINDOWS.md`.

## Validation Results

| Gate | Result |
|---|---|
| Task 1 tracer: `cargo test --locked tracer_path -- --exact`, `evidence_atomic_`, `cli_exit_` | Passed |
| Task 2: `bounded_probe_`, `fixture_admission_`, `currentness_` | Passed |
| Task 3 exact gate: fmt check, clippy `-D warnings`, filtered process suite, full suite | Passed |
| Full Rust suite | 37 unit tests + 14 process tests passed; 0 failed, ignored, or skipped |
| Documented `cargo run ... run --evidence target/g0-evidence.json` | Returned expected exit 2 and ordered live FAIL/UNPROVEN result |
| Immediate `verify-evidence` against emitted run ID | Returned exit 0 and verified the current exact run |
| Stub/skipped-test/unrun-verify scan for plan files | One intentional native stub; no skipped tests or unrun plan verification |

## TDD Gate Compliance

- Task 1 RED `86a712a` precedes GREEN `d80e299`.
- Task 2 RED `e4fe824` precedes GREEN `c4ac7ab`.
- Task 3 RED `e250306` precedes GREEN `5431cc6`.
- Every GREEN commit passed its task verification before commit.

## Requirement Coverage

HOST-01 is not marked complete by this plan. The command now fails closed with
durable current evidence while local Xorg and live NVML remain unavailable;
Plan 01-03 owns those native proofs.

## Next Plan Readiness

Plan 01-03 can implement local-Xorg and NVML observations behind the existing
primitive worker seam without changing the V1 base, CLI, parent-owned
authority, persistence, or process-test contracts.

## Self-Check: PASSED

- All nine implementation/test/documentation artifacts exist.
- All six RED/GREEN task commits exist in git history.
- The exact plan verification and documented run/readback commands passed.
- `01-02-SUMMARY.md` is present with `status: complete`.
