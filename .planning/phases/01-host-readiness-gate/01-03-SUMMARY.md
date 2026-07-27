---
phase: 01-host-readiness-gate
plan: 03
subsystem: host-readiness
status: complete
tags:
  - rust
  - x11
  - nvml
  - ffi
  - nvidia
  - bounded-workers
  - tdd
dependency-graph:
  requires:
    - "01-02: bounded workers, parent-owned evaluation, atomic evidence, and exact-run readback"
  provides:
    - "Conjunctive local-Xorg proof using logind, AF_UNIX peer identity, X11 setup, and RandR"
    - "Official-header-gated NVML ABI oracle and complete source/runtime admission sequence"
    - "Strict host-foundation.v1 semantic readback validation"
    - "Current Wayland and NVIDIA mismatch fixture plus adversarial native failure matrix"
  affects:
    - "01-04 pre-reboot immutable FAIL archive"
    - "01-05 and 01-06 selected-output/GPU correlation"
    - "01-07 through 01-10 NvFBC, NVENC, and terminal G0 proof"
tech-stack:
  added: []
  patterns:
    - "Operator-controlled public ABI source compiled through a narrow C oracle"
    - "Fixed-SONAME dynamic loading only after source digest and ABI verification"
    - "Typed, ordered, path-free native failure evidence"
    - "Known-extension semantic validation layered above the generic V1 decoder"
key-files:
  created:
    - build.rs
    - native/nvml_abi_oracle.c
    - src/local_xorg.rs
    - src/native_nvml.rs
    - tests/fixtures/current-wayland-driver-mismatch.json
    - tests/fixtures/host01-session-spoofing.json
  modified:
    - src/lib.rs
    - src/probe.rs
    - tests/host_doctor_cli.rs
decisions:
  - "Local Xorg passes only when one active local logind session, its AF_UNIX peer credentials/process, X11 setup, and RandR facts all agree."
  - "NVML admission requires the authenticated API 13 source digest, C-oracle ABI agreement, fixed libnvidia-ml.so.1 symbols, positive device identity, and successful shutdown."
  - "host-foundation.v1 semantics are validated on application readback without changing the generic V1 base decoder."
  - "Selected-output correlation remains explicitly UNPROVEN and prevents G0 PASS even when the local-Xorg and NVML foundation subchecks pass."
requirements-completed:
  - HOST-01
coverage:
  - id: D1
    description: "The host doctor rejects non-local, non-Xorg, nested, proxy, malformed, and environment-only session claims through one conjunctive production predicate."
    requirement: HOST-01
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#local_xorg_joined_predicate_accepts_only_the_complete_shape"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#bounded_probe_session_clears_environment_only_xorg_claims"
        status: pass
    human_judgment: false
  - id: D2
    description: "NVML readiness is authenticated against the operator-confirmed official API 13 header and requires the complete bounded runtime sequence."
    requirement: HOST-01
    verification:
      - kind: unit
        ref: "cargo test --locked nvml_source_abi_"
        status: pass
      - kind: unit
        ref: "cargo test --locked nvml_runtime_"
        status: pass
    human_judgment: false
  - id: D3
    description: "The current Wayland host and exact 610.43.02/610.43.03 mismatch produce deterministic named failures, while every partial native path remains failed."
    requirement: HOST-01
    verification:
      - kind: e2e
        ref: "tests/host_doctor_cli.rs#host01_native_current_wayland_driver_mismatch_is_specific_and_deterministic"
        status: pass
      - kind: e2e
        ref: "tests/host_doctor_cli.rs#host01_native_session_and_nvml_partial_matrix_never_clears_later_blockers"
        status: pass
    human_judgment: false
  - id: D4
    description: "HOST-01 evidence round-trips strictly without admitting injected status/path fields or clearing selected-output, NvFBC, NVENC, and G0 blockers."
    requirement: HOST-01
    verification:
      - kind: e2e
        ref: "tests/host_doctor_cli.rs#host01_native_known_extension_payload_is_strict_at_verify_readback"
        status: pass
      - kind: e2e
        ref: "tests/host_doctor_cli.rs#host01_native_complete_foundation_still_cannot_clear_g0"
        status: pass
    human_judgment: false
metrics:
  duration: "33m 28s"
  tasks-completed: 3
  files-changed: 9
  lines-added: 3983
  lines-removed: 6
  completed-date: "2026-07-27"
---

# Phase 1 Plan 3: Native Local-Xorg and NVML Proof Summary

Conjunctive physical-Xorg admission and official-header-backed NVML ABI/runtime
proof with strict, deterministic, path-free HOST-01 evidence.

## Accomplishments

- Joined the active local logind session to its real AF_UNIX X peer, verified
  peer credentials and executable identity, and required successful X11
  setup/RandR facts before local-Xorg admission.
- Added read-only `/dev/uinput`, DRM render-node, connected physical-output,
  and loaded NVIDIA kernel-version observations with stable ordered
  remediation reasons.
- Authenticated the operator-confirmed NVML API 13 header by public identity
  and SHA-256, compiled a project-owned C ABI oracle, and mapped every unsafe
  Rust declaration to an official symbol and oracle comparison.
- Exercised only `libnvidia-ml.so.1` through verified load, initialize,
  userspace-version, positive bounded device count, UUID, canonical PCI BDF,
  and exactly-once shutdown stages.
- Added current-host, spoofing, native-stage, cleanup, timeout, sentinel, and
  injected-payload process coverage through the production binary, evaluator,
  atomic store, and exact-run readback.
- Kept selected-output correlation, NvFBC, and NVENC explicitly unproven, so no
  partial host foundation can clear G0.

## Task Commits

Each task followed a committed RED/GREEN cycle:

| Task | RED | GREEN | Result |
|---|---|---|---|
| 1. Prove one local-Xorg-shaped production path | `f8192b5` | `32a97ce` | The joined logind/socket/process/protocol predicate rejects spoofed and nonphysical X servers while source-neutral device checks remain fail-closed. |
| 2. Authenticate the NVML ABI and runtime | `6db2193` | `bae4627` | The official source digest and C oracle gate the complete fixed-SONAME NVML runtime sequence. |
| 3. Lock native failures into process assertions | `12e24a9` | `c8a0672` | Current-host and adversarial cases produce strict ordered evidence without admitting paths, native errors, or partial success. |

## Files Created or Modified

- `build.rs` — authenticates the configured official header, hashes it, and
  compiles the narrow C ABI oracle without copying source into the repository.
- `native/nvml_abi_oracle.c` — includes the official header and exports only
  project-owned size, alignment, offset, constant, status, and signature facts.
- `src/local_xorg.rs` — collects and evaluates the conjunctive physical-Xorg
  and source-neutral HOST-01 foundation facts.
- `src/native_nvml.rs` — source map, authenticated ABI metadata, fixed-symbol
  dynamic provider, complete runtime lifecycle, and typed evidence validation.
- `src/probe.rs` — runs live HOST-01 collection in the existing bounded,
  environment-cleared worker and passes only the explicit source-root variable.
- `src/lib.rs` — populates and strictly validates `host-foundation.v1` during
  persistence/readback and explicit evidence verification.
- `tests/fixtures/current-wayland-driver-mismatch.json` — deterministic current
  Wayland/Xwayland and 610.43.02/610.43.03 negative observation.
- `tests/fixtures/host01-session-spoofing.json` — physical-Xorg spoofing,
  source/ABI/runtime, device, and cleanup failure matrix.
- `tests/host_doctor_cli.rs` — end-to-end production process assertions for
  current-host, partial-success, bounded-fault, and tamper cases.

## Decisions Made

- Environment strings and X socket existence are never admission evidence by
  themselves; every identity layer must agree on one active local real-Xorg
  session.
- The external header remains operator-controlled. Evidence stores only the
  public `nvidia-nvml-api-13` identity and SHA-256
  `31a26e3ce6f0b98a76cea38a3cf28aa112a20cfb37e9057786c768712d6a487f`,
  never the source root or proprietary content.
- Native runtime presence is insufficient. The provider loads only the
  verified SONAME and symbols after source/ABI proof and admits no initialized
  path that misses bounded device identity or successful cleanup.
- Strict HOST-01 payload validation stays above the generic envelope decoder,
  preserving Plan 01-01's forward-compatible unknown-extension contract and
  unchanged V1 base fields.

## Deviations from Plan

None - plan executed exactly as written.

## Authentication and Operator Gates

The blocking source gate was satisfied before Task 2. The operator confirmed
lawful access to an applicable official NVIDIA CUDA 13.3 / NVML API 13 header
and acceptance of applicable terms outside this workflow. The executor did not
download a header, install a package, accept terms, or persist the
operator-controlled source location.

## Known Stubs

None. Selected-output correlation, NvFBC, and NVENC remain intentional
fail-closed ownership boundaries for Plans 01-05 through 01-09, not stubs in
this plan's HOST-01 goal.

## Validation Results

| Gate | Result |
|---|---|
| Task 1 exact local-Xorg/source-neutral/bounded-session gate | Passed |
| Task 2 authenticated `nvml_source_abi_` and `nvml_runtime_` gates | Passed |
| Task 3 fmt, clippy `-D warnings`, `host01_`, and `host01_native_` gates | Passed |
| Full locked Rust suite | 46 unit tests + 22 process tests passed; 0 failed, ignored, or skipped |
| No-header provider build/check | Passed; source absence remains a typed fail-closed path |
| Live production run with authenticated source | Returned expected G0 exit 2 with local-Xorg and NVML failures and later blockers unproven |
| Immediate exact-run `verify-evidence` readback | Returned exit 0 and verified the persisted FAIL |
| Stub/skipped-test/unrun-verify scan | No stubs, skipped tests, or unrun verification |

## TDD Gate Compliance

- Task 1 RED `f8192b5` precedes GREEN `32a97ce`.
- Task 2 RED `6db2193` precedes GREEN `bae4627`.
- Task 3 RED `12e24a9` precedes GREEN `c8a0672`.
- Every GREEN commit passed its task verification before commit.

## Requirement Coverage

HOST-01 is complete: the production doctor specifically reports and blocks
non-Xorg sessions, the known NVIDIA kernel/userspace mismatch, failed or
partial NVML, missing physical output, missing `/dev/uinput`, and missing DRM
render access. Selected-output identity, capture, and encode remain assigned to
HOST-02 through HOST-04.

## Next Plan Readiness

Plan 01-04 can archive the current authenticated pre-reboot FAIL without
inventing native facts or modifying the host. The live host remains on a
non-Xorg session and its NVML initialization fails under the current mismatched
driver installation; the immutable archive should preserve those facts before
operator correction. Later output/GPU correlation, NvFBC, and NVENC plans
remain blocked from G0 PASS exactly as intended.

## Self-Check: PASSED

- All nine implementation/test/fixture artifacts exist.
- All six RED/GREEN task commits exist in git history.
- The exact task gates, full locked suite, no-header check, live run, and exact
  readback verification passed.
- `01-03-SUMMARY.md` is present with `status: complete`.
