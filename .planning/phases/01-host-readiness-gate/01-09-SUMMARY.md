---
phase: 01-host-readiness-gate
plan: 09
subsystem: host-readiness
tags:
  - rust
  - nvenc
  - h264
  - hevc
  - av1
  - cuda
  - evidence
requires:
  - phase: 01-08
    provides: "Authenticated one-frame NvFBC/CUDA capture lease and explicit copy ledger"
provides:
  - "Executable NVENC ownership/copy predicate with explicit PASS and BLOCKED_UNKNOWN outcomes"
  - "Closed seven-position 3840x2160@60 NVENC policy with Ada-or-newer AV1 filtering"
  - "Bounded H.264, HEVC, and AV1 first-keyframe inspectors over actual provider bytes"
  - "Typed diagnostic and live-unavailable evidence that cannot advertise or pass HOST-04"
affects:
  - 01-10-native-nvenc-probes
  - host04-evidence
  - g0-readiness
tech-stack:
  added: []
  patterns:
    - "Advertisements are derived only from terminal attempt, copy, parsed stream, and reverse-cleanup proof"
    - "Generation-ineligible policy positions terminate without invoking a provider"
    - "Diagnostic API semantics exercise policy compatibility without conferring live authority"
key-files:
  created:
    - .planning/phases/01-host-readiness-gate/01-NVENC-COPY-BOUNDARY-SPIKE.md
    - src/native_nvenc.rs
    - src/nvenc_bitstream.rs
    - tests/host04_copy_spike.rs
    - tests/host04_policy.rs
    - tests/host04_bitstream.rs
  modified:
    - src/model.rs
    - src/evidence.rs
    - src/probe.rs
    - src/lib.rs
    - tests/fixtures/host04-nvenc-tuples.json
    - tests/host_doctor_cli.rs
key-decisions:
  - "Treat the documented same-GPU pitch-linear to block-linear NVENC preprocessing copy as a known encoder-internal edge; any host, peer, or unknown edge blocks admission."
  - "Keep the policy closed to exactly seven 4K60 positions and prefilter AV1 before provider invocation on pre-Ada or unknown generations."
  - "Let codec bytes prove codec/profile/chroma/depth/dimensions/keyframe only; buffer format and exact 60/1 remain separately proven resource/config facts."
  - "Keep SDK 12.x, 13.0, 13.1, and incompatible-version fixtures diagnostic-only and keep the live provider unavailable until Plan 01-10."
patterns-established:
  - "Strict first-stream inspection accepts H.264/HEVC Annex-B and AV1 low-overhead sized OBUs only, with explicit byte, unit, bit, Exp-Golomb, LEB128, and allocation limits."
  - "Known extension validation is additive: typed HOST-04 evidence is strict while the exact legacy unproven placeholder remains readable."
requirements-completed:
  - HOST-04
coverage:
  - id: D1
    description: "The D-05 copy predicate names the NvFBC lease, CUDA registration/map/submit edges, the known NVENC preprocessing copy, and every blocking unknown edge."
    requirement: HOST-04
    verification:
      - kind: unit
        ref: "tests/host04_copy_spike.rs#host04_copy_spike_"
        status: pass
      - kind: other
        ref: ".planning/phases/01-host-readiness-gate/01-NVENC-COPY-BOUNDARY-SPIKE.md"
        status: pass
    human_judgment: false
  - id: D2
    description: "Exactly seven policy positions exist, with H.264 High first, no AV1 4:4:4 representation, and no AV1 provider calls before Ada."
    requirement: HOST-04
    verification:
      - kind: unit
        ref: "tests/host04_policy.rs#host04_policy_ and host04_generation_"
        status: pass
    human_judgment: false
  - id: D3
    description: "H.264, HEVC, and AV1 identity and keyframe facts are parsed from bounded provider bytes and must match the exact closed position."
    requirement: HOST-04
    verification:
      - kind: unit
        ref: "tests/host04_bitstream.rs#host04_bitstream_"
        status: pass
    human_judgment: false
  - id: D4
    description: "Diagnostic API/generation cases persist seven terminal attempts but never advertisements or HOST-04 authority."
    requirement: HOST-04
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host04_diagnostic_"
        status: pass
    human_judgment: false
  - id: D5
    description: "Unavailable native source, timeout, malformed input, or injected advertisement/copy/cleanup/version claims keep HOST-04 unproven and G0 failed."
    requirement: HOST-04
    verification:
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host04_diagnostic_timeout_and_invalid_fixture_fail_closed"
        status: pass
      - kind: integration
        ref: "tests/host_doctor_cli.rs#host04_diagnostic_injected_advertisement_copy_cleanup_and_version_are_rejected"
        status: pass
    human_judgment: false
duration: 40m
completed: 2026-07-30
status: complete
---

# Phase 1 Plan 9: NVENC Copy-Boundary and Diagnostic Contract Summary

**Closed seven-position 4K60 NVENC policy with an executable copy predicate, bounded real-bitstream identity inspection, and diagnostic evidence that cannot claim live authority**

## Performance

- **Duration:** 40m
- **Started:** 2026-07-29T22:19:48Z
- **Completed:** 2026-07-29T23:00:00Z
- **Tasks:** 3/3
- **Files created or modified:** 12

## Accomplishments

- Froze the D-05 ownership and copy graph before native NVENC work, including
  the documented SDK 13.1 same-GPU pitch-linear preprocessing copy and explicit
  blockers for host staging, peer transfer, incomplete cleanup, or unknown
  application/encoder edges.
- Modeled exactly seven 3840x2160@60 policy positions. H.264 High is first,
  HEVC covers 4:2:0 and 4:4:4 at 8/10-bit, AV1 is Main 4:2:0 only, and AV1 is
  terminated without a provider call before Ada.
- Added panic-free, bounded H.264 SPS/IDR, HEVC SPS/IDR, and AV1 sequence/frame
  inspection over bytes, with strict Annex-B or sized low-overhead OBU framing,
  shared SHA-256 identity, and exact tuple matching.
- Persisted typed diagnostic evidence through the production worker and
  extension path. SDK 12.2, 13.0, 13.1, and 14.0 cases all produce seven
  terminal attempts, zero advertisements, HOST-04 UNPROVEN, and overall G0
  FAIL without any live NVENC call.
- Kept the exact legacy unproven HOST-04 placeholder readable while rejecting
  injected advertisements, copy proof, cleanup, or API-version claims.

## Task Commits

### Task 1: Create the NVENC copy-boundary spike before native policy work

- `c39a8d8` — TDD RED: failing copy-boundary contract tests.
- `37becf6` — TDD GREEN: executable ownership/copy matrix and diagnostic fixture.

### Task 2: Implement the closed tuple policy and provider seam

- `2b793a1` — TDD RED: failing closed-policy, generation, and extension tests.
- `e88a25c` — TDD GREEN: seven positions, terminal attempts, advertisement
  derivation, strict typed persistence, and diagnostic/live-unavailable
  providers.

### Task 3: Bound real-bitstream inspection and diagnostic process behavior

- `6408d8d` — TDD RED: failing parser and production-process tests.
- `cefb75b` — TDD GREEN: bounded H.264/HEVC/AV1 inspectors, typed worker
  transport, diagnostic fixture matrix, and fail-closed persistence behavior.

## Verification

- `cargo fmt --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- All plan filters passed: `host04_copy_spike_`, `host04_policy_`,
  `host04_generation_`, `nvenc_extension_`, `host04_bitstream_`, and
  `host04_diagnostic_`.
- Source-enabled `cargo test --locked --all-targets` passed: 79 library tests,
  5 bitstream tests, 3 copy tests, 7 policy tests, 47 CLI tests, 9 mapping
  tests, and the tracer. The existing operator-only live NvFBC test remained
  ignored as designed.

## Files Created or Modified

- `.planning/phases/01-host-readiness-gate/01-NVENC-COPY-BOUNDARY-SPIKE.md` —
  exact resource graph, observability matrix, PASS/BLOCKED predicate, and
  Plan 01-10 live command.
- `src/model.rs`, `src/native_nvenc.rs` — closed tuple types, generation gates,
  terminal attempt/cleanup/copy/stream proof, providers, and advertisement
  derivation.
- `src/nvenc_bitstream.rs` — bounded H.264/HEVC Annex-B and AV1 low-overhead
  OBU inspectors over actual bytes.
- `src/evidence.rs`, `src/probe.rs`, `src/lib.rs` — strict typed persistence,
  request-bound worker transport, and additive legacy compatibility.
- `tests/fixtures/host04-nvenc-tuples.json` and `tests/host04_*.rs` — copy,
  policy, generation, API, bitstream, timeout, cleanup, and injection matrices.
- `tests/host_doctor_cli.rs` — full-binary diagnostic authority, persistence,
  timeout, and adversarial-claim regressions.

## Decisions Made

- A known NVENC-internal pitch-linear to block-linear conversion is recorded,
  not hidden. It is acceptable only on the same GPU; unknown, peer, or host
  edges block the tuple.
- Codec bytes cannot prove the CUDA input buffer enum or an exact 60/1 rate.
  The parser proves only observable codec/profile/chroma/depth/dimensions and
  keyframe identity; native resource/config evidence must prove the remaining
  tuple fields in Plan 01-10.
- Diagnostic SDK versions exercise semantic policy branches only. They carry
  neither authenticated headers/runtime identity nor authority to advertise.
- The standalone SDK 13.1 copy experiment stays isolated from Kyber's pinned
  `nv-codec-headers n12.1.14.0` build inputs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added strict typed persistence validation**

- **Found during:** Task 2
- **Issue:** The planned model/provider files could construct typed evidence,
  but the persisted `nvenc-tuples.v1` record would still accept only the legacy
  placeholder and could not safely enforce advertisement derivation.
- **Fix:** Added additive typed validation in `src/evidence.rs`, retaining the
  exact legacy unproven shape while rejecting injected or inconsistent typed
  claims.
- **Committed in:** `e88a25c`

**2. [Rule 2 - Missing Critical] Added a request-bound NVENC worker seam**

- **Found during:** Task 3
- **Issue:** Full-binary diagnostic coverage required typed evidence to cross
  the existing bounded worker protocol, but `src/probe.rs` was absent from the
  task file list and had no NVENC observation channel.
- **Fix:** Added a strictly probe-bound optional NVENC observation, HOST-04
  fixture dispatch, diagnostic/live-unavailable providers, and typed extension
  persistence without adding a native session.
- **Committed in:** `cefb75b`

## Deferred Issues

- Best-effort `.planning/WINDOWS.md` deviation appends were rejected by a
  pre-existing ledger count mismatch (`3/0/7/10` frontmatter versus
  `2/0/8/10` parsed entries). The issue is recorded in the phase
  `deferred-items.md`; it does not affect code or test verification.
- HOST-04 is intentionally not ready to mark complete: the readiness query
  returned `0/1` because Plan 01-10 still owns the authenticated native
  one-frame proof.

## TDD Gate Compliance

- RED commits precede GREEN commits for all three tasks.
- Every RED gate failed for the intended missing behavior, and every GREEN
  gate passed the exact task filters.

## Authentication Gates

None.

## Self-Check: PASSED

- All seven created artifacts exist on disk.
- All six task commits are present in repository history.
