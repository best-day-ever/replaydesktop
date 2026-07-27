---
phase: 01-host-readiness-gate
plan: 04
subsystem: host-readiness
status: complete
tags:
  - rust
  - immutable-archive
  - provenance
  - nvml
  - compatibility
  - tdd
dependency-graph:
  requires:
    - "01-03: live local-Xorg and exact NVIDIA kernel/NVML userspace evidence"
  provides:
    - "Create-once self-contained pre-reboot archive with descriptor-copied executable and exact live evidence"
    - "Offline archive verifier for contained paths, modes, sizes, file digests, V1 identity, and raw extension digests"
    - "Immutable original Wayland and 610.43.02/610.43.03 mismatch evidence"
    - "Permanent foundation-envelope and original-archive compatibility gates"
  affects:
    - "01-05 through 01-10 host-readiness expansions"
    - "Any future base V1 decoder or archive-verifier change"
tech-stack:
  added: []
  patterns:
    - "Descriptor-only source copy from /proc/self/exe with immediate fstat identity"
    - "Component-by-component no-follow destination traversal and rename-noreplace commit"
    - "Manifest-after-data and index-last fsync ordering"
    - "Offline compatibility tests pinned to immutable historical digests"
key-files:
  created:
    - src/archive.rs
    - .planning/phases/01-host-readiness-gate/01-VALIDATION.md
    - artifacts/validation/g0/pre-reboot/index.json
    - artifacts/validation/g0/pre-reboot/run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/replay-host-doctor
    - artifacts/validation/g0/pre-reboot/run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/g0-evidence.json
    - artifacts/validation/g0/pre-reboot/run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/manifest.json
  modified:
    - src/cli.rs
    - src/lib.rs
    - src/native_nvml.rs
    - tests/host_doctor_cli.rs
    - README.md
decisions:
  - "The executable source is trusted only through an open /proc/self/exe descriptor; Cargo-style hard links are safe because path identity is never reopened or resolved."
  - "The loaded libnvidia-ml.so exact filename version is recorded before NVML initialization so initialization failure cannot hide a real kernel/userspace mismatch."
  - "Archive verification is offline and never executes the archived binary or consults target/ or the current executable."
  - "Plans 01-06, 01-08, and 01-10 and every base-decoder/archive-verifier change must run both permanent compatibility tests."
requirements-completed:
  - HOST-01
coverage:
  - id: D1
    description: "The archive commits complete executable/evidence bytes before a relative contained manifest and create-once index become authoritative."
    requirement: HOST-01
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#archive_proc_self_exe_magic_link_source_open"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#archive_create_once_rejects_collision_and_symlink_root"
        status: pass
    human_judgment: false
  - id: D2
    description: "The preserved original-host evidence is a fresh live FAIL containing SESSION_NOT_XORG and exact 610.43.02/610.43.03 NVIDIA_VERSION_MISMATCH facts."
    requirement: HOST-01
    verification:
      - kind: live
        ref: ".planning/phases/01-host-readiness-gate/01-VALIDATION.md#build-and-controlled-live-capture"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#pre_reboot_archive_manifest_path_digest_schema"
        status: pass
    human_judgment: false
  - id: D3
    description: "The verifier reads only immutable contained bytes and checks fixed binary/evidence/manifest digests, V1 base identity, and every raw extension digest."
    requirement: HOST-01
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#original_pre_reboot_archive_compat"
        status: pass
      - kind: cli
        ref: "replay-host-doctor verify-archive --index artifacts/validation/g0/pre-reboot/index.json"
        status: pass
    human_judgment: false
  - id: D4
    description: "The original Plan 01-01 fixture retains its unchanged V1 base and unknown raw extension across re-encoding."
    requirement: HOST-01
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#g0_v1_foundation_compat"
        status: pass
    human_judgment: false
metrics:
  duration: "29m 58s"
  tasks-completed: 3
  files-changed: 11
  lines-added: 2228
  lines-removed: 23
  completed-date: "2026-07-27"
---

# Phase 1 Plan 4: Immutable Pre-Reboot Failure Archive Summary

Descriptor-copied executable and exact live Wayland/NVIDIA-mismatch evidence
sealed into a create-once archive with offline V1 and raw-extension
verification.

## Accomplishments

- Added `archive-pre-reboot` and `verify-archive` with no-follow,
  create-new, descriptor-relative traversal, restrictive modes, file and
  directory fsyncs, rename-noreplace commit, and index-last publication.
- Copied and hashed the exact `/proc/self/exe` bytes through one immediately
  metadata-checked descriptor and proved later target replacement cannot affect
  archive verification.
- Preserved live run
  `run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`
  with `SESSION_NOT_XORG`, loaded kernel `610.43.02`, NVML userspace
  `610.43.03`, and `NVIDIA_VERSION_MISMATCH`.
- Bound the archived executable, evidence, manifest, base identity, argv,
  boot/session/times, provenance, schema, and all raw extension payload
  digests under one contained run directory.
- Added stable original-fixture and original-archive compatibility gates and
  assigned them to Plans 01-06, 01-08, and 01-10 plus every future decoder or
  verifier change.

## Task Commits

| Task | Commit | Result |
|---|---|---|
| 1 RED: immutable archive process contract | `82bbd86` | Failing process tests covered procfs magic-link copy, create-once semantics, target replacement, malformed V1, and filesystem attacks. |
| 1 GREEN: archive writer/verifier | `b9945f8` | Self-contained descriptor-copied archive and offline verifier passed the tracer gate. |
| 2: fresh current-host archive | `019d997` | The final live FAIL and exact observing binary were archived and immediately verified. |
| 3 RED: permanent original compatibility gates | `3d325c8` | Both stable tests failed on the missing later-plan/documentation obligation after their byte-level checks passed. |
| 3 GREEN: compatibility obligation | `27d74b9` | README and validation now require the named gates at all owning later checkpoints. |

## Files Created or Modified

- `src/archive.rs` — archive manifest/index schemas, descriptor-only writer,
  containment rules, transaction ordering, and offline verifier.
- `src/cli.rs` and `src/lib.rs` — public archive/verify commands and bounded
  result envelopes.
- `src/native_nvml.rs` — loaded-library version extraction before
  initialization and exact kernel/userspace mismatch precedence.
- `tests/host_doctor_cli.rs` — archive adversarial coverage, immutable index
  schema/digest check, and permanent original-fixture/archive regressions.
- `artifacts/validation/g0/pre-reboot/` — create-once original live archive.
- `README.md` and `01-VALIDATION.md` — exact capture, verification, and
  downstream compatibility obligations.

## Decisions Made

- A regular executable may have multiple hard links, as Cargo artifacts do.
  The security boundary is the opened `/proc/self/exe` descriptor and its
  immediate metadata, not source-path link count. Evidence input remains a
  single-link mode-0600 file.
- The exact loaded NVML shared-object filename is a useful userspace identity
  before initialization. The official runtime driver-version query must agree
  with it when initialization succeeds.
- The immutable archive verifier remains deliberately offline: it hashes
  contained archived bytes, strictly decodes their evidence, and never runs
  historical code.
- Known-extension semantic validators are additive and can never replace the
  base V1/raw-extension or original archive compatibility gates.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Retained userspace identity when NVML initialization fails**

- **Found during:** Task 2 live acceptance
- **Issue:** The original provider queried the userspace driver version only
  after successful NVML initialization, so the real mismatched host reported
  `NVML_INITIALIZATION_FAILED` but could not preserve or emit
  `NVIDIA_VERSION_MISMATCH`.
- **Fix:** Read the exact loaded `libnvidia-ml.so.610.43.03` identity from the
  already-loaded mapping before initialization, require an unambiguous exact
  version, retain it on initialization failure, and cross-check it against the
  official query after successful initialization.
- **Files modified:** `src/native_nvml.rs`
- **Commit:** `019d997`

**2. [Rule 1 - Bug] Accepted Cargo-style hard-linked executable sources**

- **Found during:** Task 2 fail-closed archive preflight
- **Issue:** Cargo hard-linked `target/debug/replay-host-doctor` to its hashed
  artifact, so `/proc/self/exe` correctly described a regular file with link
  count two. A shared source check rejected it before any index or run
  directory was committed.
- **Fix:** Keep the single-link requirement for evidence, but allow regular
  executable descriptors with multiple links because all bytes and metadata
  are consumed only through the already-open descriptor. The procfs regression
  now creates and accepts a Cargo-style hard link.
- **Files modified:** `src/archive.rs`, `tests/host_doctor_cli.rs`,
  `.planning/phases/01-host-readiness-gate/01-VALIDATION.md`
- **Commit:** `019d997`

## Authentication and Operator Gates

No authentication gate occurred. The preconfigured
`REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include` source gate
remained satisfied; the executor did not download sources, install packages,
accept terms, or persist that operator-controlled path in evidence.

## Known Stubs

None. Selected-output, NvFBC, and NVENC remain explicit later-plan
fail-closed boundaries, not missing functionality in this archive goal.

## Validation Results

| Gate | Result |
|---|---|
| Task 1 archive tracer and post-commit feedback gate | Passed |
| Live capture | Expected exit 2; live FAIL with `SESSION_NOT_XORG`, `610.43.02` kernel, `610.43.03` userspace, and `NVIDIA_VERSION_MISMATCH` |
| Immediate archive verification | Passed; archived executable SHA-256 `9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e` |
| Manifest/index containment and fixed digests | Passed; manifest SHA-256 `d4d16fc2bee5960cedd01881e3fcc8cd1496ed014ceadb88fe67f288490ee136` |
| Permanent `g0_v1_foundation_compat` | Passed |
| Permanent `original_pre_reboot_archive_compat` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| Full locked Rust suite | 47 unit tests + 31 process tests passed; 0 failed, ignored, or skipped |
| Stub/skipped-test/unrun-verify scan | No plan-introduced stubs, skipped tests, or unrun verification |

## TDD Gate Compliance

- Task 1 RED `82bbd86` precedes GREEN `b9945f8`; the tracer verification was
  rerun successfully before expansion.
- Task 3 RED `3d325c8` precedes GREEN `27d74b9`.
- Both RED phases failed for their intended missing behavior/documentation,
  and every GREEN gate passed before commit.

## Requirement Coverage

HOST-01's original negative state is now durable and non-repudiable. The
archive contains the exact binary and live evidence that observed the current
Wayland session and NVIDIA kernel/userspace mismatch, while later host gates
remain unable to substitute fixture evidence or bypass the preserved failure.

## Next Plan Readiness

Plan 01-05 may add its isolated selected-output/GPU correlation logic without
rerunning the permanent compatibility pair. Before any reboot, remediation, or
owning later live gate, the immutable archive must continue to pass
`verify-archive`; Plans 01-06, 01-08, and 01-10 must additionally run both
named permanent compatibility tests.

## Self-Check: PASSED

- All six created archive/validation artifacts exist at their documented
  paths.
- All five Task 1 through Task 3 commits exist in git history.
- The immutable index, contained manifest, executable, evidence, schemas,
  modes, and fixed digests passed offline verification.
- Both plan deviations are recorded as fixed in `.planning/WINDOWS.md`.
- `01-04-SUMMARY.md` is present with `status: complete`.
