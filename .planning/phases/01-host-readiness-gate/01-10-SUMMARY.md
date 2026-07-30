---
phase: 01-host-readiness-gate
plan: 10
subsystem: host-readiness
tags: [rust, nvenc, nvfbc, cuda, x11, g0, archive]

# Dependency graph
requires:
  - phase: 01-host-readiness-gate
    provides: Current HOST-01/HOST-02/HOST-03 selected-output NvFBC/CUDA proof and the closed HOST-04 policy/parser contract
provides:
  - Authenticated SDK 13.1 C oracle and bounded native NVENC exact-tuple provider
  - Current live HOST-01 through HOST-04 G0 PASS with two hardware-advertised 4K60 tuples
  - Separate immutable post-repair binary, evidence, manifest, and schema-dispatched archive index
affects: [kyber-integration, macos-client-decode, 4k60-streaming-gate, benchmark-harness]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Official-header C oracle owns every native NVENC ABI and version value
    - Advertisements derive only from exact live config/resource/copy/stream/cleanup success
    - Archive verification dispatches on exact index/manifest schema without loosening history

key-files:
  created:
    - native/nvenc_abi_oracle.c
    - artifacts/validation/g0/post-repair/index.json
    - artifacts/validation/g0/post-repair/run-73c9e241c73e6672cf8e6e3169b5d7dc8f77f7d809037c84cd46dbf28d8f93e3-2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc/manifest.json
  modified:
    - build.rs
    - src/native_nvenc.rs
    - src/native_nvfbc.rs
    - src/evidence.rs
    - src/archive.rs
    - src/nvenc_bitstream.rs
    - tests/host_doctor_cli.rs
    - .planning/phases/01-host-readiness-gate/01-VALIDATION.md

key-decisions:
  - "Keep the official SDK 13.1 standalone probe isolated from Kyber/Kymedia's pinned nv-codec-headers n12.1.14.0; compatibility of that later integration remains unclaimed."
  - "Advertise only H.264 High 4:2:0 8-bit and HEVC Main 4:2:0 8-bit on the current Ampere/NV12 lease; incompatible HEVC inputs and AV1 remain terminal and non-advertised."
  - "Use distinct post-repair schemas while retaining one verifier that dispatches fail-closed by exact index and manifest contract."
  - "Parse only a bounded decoded IDR slice-header prefix while hashing the complete capped bitstream, allowing real large NVENC frames without weakening parser bounds."

patterns-established:
  - "Native proof pattern: authenticate headers at build time, compare source-derived ABI facts, load only fixed bootstrap symbols, and persist no native pointers or source paths."
  - "Live advertisement pattern: config + exact resource identity + copy predicate + parsed keyframe + reverse cleanup are all mandatory."
  - "Historical archive pattern: new result classes receive distinct schemas; legacy admission semantics remain byte-compatible and contract-specific."

requirements-completed: [HOST-04]

coverage:
  - id: D1
    description: Authenticated SDK 13.1 ABI and exact native one-frame NVENC tuple provider
    requirement: HOST-04
    verification:
      - kind: integration
        ref: cargo test --locked host04_source_abi_
        status: pass
      - kind: integration
        ref: cargo test --locked host04_policy_
        status: pass
      - kind: integration
        ref: cargo test --locked host04_bitstream_
        status: pass
    human_judgment: false
  - id: D2
    description: Current selected-output G0 PASS with seven terminal policy positions and exact proof for every advertisement
    requirement: HOST-04
    verification:
      - kind: e2e
        ref: cargo test --locked --test host_doctor_cli host04_live_g0_current_output -- --ignored --exact --nocapture
        status: pass
      - kind: e2e
        ref: replay-host-doctor verify-evidence with HOST-01 through HOST-04 pass and all four known-extension validators
        status: pass
    human_judgment: false
  - id: D3
    description: Separate create-once post-repair PASS archive with unchanged original pre-reboot compatibility
    requirement: HOST-04
    verification:
      - kind: integration
        ref: tests/host_doctor_cli.rs#post_repair_archive_is_distinct_create_once_and_verifiable
        status: pass
      - kind: integration
        ref: tests/host_doctor_cli.rs#original_pre_reboot_archive_compat
        status: pass
      - kind: e2e
        ref: replay-host-doctor verify-archive --index artifacts/validation/g0/post-repair/index.json
        status: pass
    human_judgment: false

# Metrics
duration: 58min
completed: 2026-07-30
status: complete
---

# Phase 1 Plan 10: Native NVENC and Final G0 Summary

**Authenticated SDK 13.1 NVENC probes now encode and validate exact live 4K60 H.264/HEVC keyframes, producing a durable current G0 PASS archive without loosening the original failure history**

## Performance

- **Duration:** 58 min
- **Started:** 2026-07-29T23:18:35Z
- **Completed:** 2026-07-30T00:17:22Z
- **Tasks:** 3
- **Files modified:** 22

## Accomplishments

- Derived every used NVENC ABI, GUID, enum, structure version, and runtime compatibility fact from the separately asserted official SDK 13.1 header and C oracle.
- Re-proved HOST-01 through HOST-03, encoded the actual selected-output NV12 CUDA lease, and emitted seven terminal HOST-04 positions with exact proof for both advertised 4K60 hardware tuples.
- Persisted and verified a separate post-repair G0 PASS archive containing the exact current executable, evidence, manifest, and index while retaining the original pre-reboot decoder and fixed archive digests.

## Task Commits

Each TDD task was committed atomically:

1. **Task 1: Assert SDK authenticity and verify original/current prerequisites** — read-only checkpoint accepted; no repository commit
2. **Task 2 RED: Add failing authenticated NVENC contracts** — `2eb2a41` (test)
3. **Task 2 GREEN: Implement authenticated native NVENC tuple probes** — `8b50bb0` (feat)
4. **Task 3 RED: Add failing final G0 archive contracts** — `50f703a` (test)
5. **Task 3 GREEN: Qualify and archive final live G0 PASS** — `db46355` (feat)

## Files Created/Modified

- `native/nvenc_abi_oracle.c` - Header-owned ABI checks, policy facts, exact encoder configuration, live CUDA resource registration, one-frame submission, and reverse cleanup.
- `build.rs` - Authenticates the canonical SDK 13.1 header snapshot and compiles the isolated native oracle.
- `src/native_nvenc.rs` - Runs seven closed policy positions against the live selected-output capture lease and derives advertisements from complete proof only.
- `src/native_nvfbc.rs` - Exposes the scoped in-process CUDA lease/context consumer used by the NVENC worker.
- `src/model.rs`, `src/evidence.rs`, `src/probe.rs`, `src/lib.rs` - Persist and cross-validate source, config, resource, copy, stream, cleanup, topology, and G0 admission evidence.
- `src/nvenc_bitstream.rs` - Accepts valid Annex-B framing zeros and parses a bounded slice-header prefix from large real keyframes.
- `src/archive.rs`, `src/cli.rs` - Add the distinct create-once post-repair archive contract and schema-dispatched verification.
- `tests/host04_source_abi.rs`, `tests/host04_policy.rs`, `tests/host04_bitstream.rs`, `tests/host_doctor_cli.rs` - Cover source/ABI, closed policy, parser bounds, process containment, live G0, archive immutability, and history compatibility.
- `artifacts/validation/g0/post-repair/` - Contains the exact current PASS executable, evidence, manifest, and index.
- `README.md`, `.planning/phases/01-host-readiness-gate/01-VALIDATION.md` - Separate static, fixture, live, compatibility, and archive procedures and document the unclaimed Kyber boundary.

## Final Qualified Result

- **Run ID:** `run-73c9e241c73e6672cf8e6e3169b5d7dc8f77f7d809037c84cd46dbf28d8f93e3`
- **Advertised tuples:** H.264 High YUV420 8-bit and HEVC Main YUV420 8-bit, each 3840×2160 at 60/1
- **Archived executable SHA-256:** `865632ea0d0d7eecb2954bfa33647765c56cad7f616903e6264ce2f35caca14f`
- **Evidence SHA-256:** `2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc`
- **Manifest SHA-256:** `e661f12c9fd6c4e7dbf2a01ece9f6fd52be2fa72c60b5006702364d1f45d9a5f`
- **Index SHA-256:** `09ad18c811f8bd169ff9aae6eaa76097425a0ff7c20b8266cb3f2899d64f33c8`

## Decisions Made

- Kept the standalone official SDK 13.1 proof separate from the Kyber `n12.1.14.0` header pin to avoid implying an integration qualification that was not run.
- Treated the current application-owned NV12 capture lease as authoritative: unsupported input formats stay terminal rather than triggering conversion, alternate capture, or software fallback.
- Added post-repair archive schemas instead of broadening the semantically named pre-reboot live-FAIL contract.
- Retained strict total bitstream and parameter-set bounds while reading only the small slice-header prefix needed to prove a real large keyframe.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Carried the actual NvFBC CUDA lease into the native NVENC worker**
- **Found during:** Task 2 (native exact-tuple attempts)
- **Issue:** The task file list did not include the model/NvFBC seam needed to prove the exact live allocation, CUDA context, GPU, output, and topology across registration and submission.
- **Fix:** Added a scoped same-process capture-lease consumer plus typed config/resource proof and cross-record validation.
- **Files modified:** `src/model.rs`, `src/native_nvfbc.rs`, `src/lib.rs`
- **Verification:** Full test suite and exact ignored live G0 test pass; persisted resource proofs match HOST-02/HOST-03.
- **Committed in:** `8b50bb0`

**2. [Rule 1 - Bug] Preserved authenticated runtime facts across later unsupported policy positions**
- **Found during:** Task 2 (provider evaluation)
- **Issue:** An expected unsupported tuple could overwrite runtime API/CUDA facts learned from an earlier real attempt with absent values and reject an otherwise complete source proof.
- **Fix:** Made source evidence accumulation monotonic and associated the runtime library only with an actual loaded bridge result.
- **Files modified:** `src/native_nvenc.rs`
- **Verification:** Policy tests, full suite, and live seven-position evaluation pass.
- **Committed in:** `8b50bb0`

**3. [Rule 1 - Bug] Accepted valid large NVENC IDR slices without weakening parser bounds**
- **Found during:** Task 3 (first live G0 run)
- **Issue:** Both real keyframes were 157–158 KiB, but the parser incorrectly applied its 64 KiB parameter-set limit to the entire IDR slice and returned `invalid-stream`.
- **Fix:** Kept the 32 MiB stream cap and 64 KiB parameter-set cap, accepted Annex-B trailing zeros, and decoded only a fixed 64-byte slice-header prefix for IDR proof.
- **Files modified:** `src/nvenc_bitstream.rs`, `tests/host04_bitstream.rs`
- **Verification:** Large-IDR and trailing-zero regressions pass; the exact live G0 test advertises both hardware tuples.
- **Committed in:** `db46355`

**4. [Rule 2 - Missing Critical] Added a PASS-capable archive contract without loosening pre-reboot history**
- **Found during:** Task 3 (final archive)
- **Issue:** The existing writer was intentionally named and validated as pre-reboot live FAIL only, so reusing it directly could not honestly archive the final PASS.
- **Fix:** Added distinct post-repair index/manifest schemas and strict known-extension validation, with generic verification dispatching by exact matching contract.
- **Files modified:** `src/archive.rs`, `src/cli.rs`, `src/lib.rs`, `tests/host_doctor_cli.rs`
- **Verification:** Post-repair create-once, diagnostic rejection, cross-schema substitution, full archive attack suite, and original fixed-digest compatibility tests pass.
- **Committed in:** `db46355`

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bugs, 2 Rule 2 missing critical requirements)
**Impact on plan:** All changes were necessary to make the planned native proof and durable archive correct and fail-closed; no product-scope features were added.

## Issues Encountered

- The first real encoder run reached config, registration, mapping, copy, submission, lock, and cleanup successfully but exposed the slice-size parser bug above. The bounded fix was regression-tested before repeating the full live gate.
- A concurrent test rebuild briefly invalidated one diagnostic same-binary check during development; the isolated rerun passed. The final evidence, readback, archive copy, and archive verification were then executed sequentially with one exact binary.

## TDD Gate Compliance

- Task 2 RED `2eb2a41` precedes GREEN `8b50bb0`.
- Task 3 RED `50f703a` precedes GREEN `db46355`.
- Both RED gates failed for the intended missing behavior before implementation, and every GREEN gate now passes.

## Known Stubs

None. The two ignored hardware tests are explicit opt-in live gates; `host04_live_g0_current_output` was run and passed during this plan.

## User Setup Required

None remaining. Repeating the qualification requires the already asserted official SDK 13.1 source and the documented current X11/NVIDIA environment; the workflow did not download, install, vendor, or accept gated material.

## Next Phase Readiness

- Phase 1 now has a current immutable G0 PASS proving physical X11 capture, exact GPU/output identity, same-GPU NVENC input, H.264/HEVC 4K60 keyframes, and complete cleanup.
- The next work can begin Kyber/Kymux streaming integration and macOS exact-bitstream decode probes.
- The Kyber/Kymedia `nv-codec-headers n12.1.14.0` integration remains deliberately unclaimed and must be qualified without replacing the standalone 13.1 proof.

## Self-Check: PASSED

- All important created/modified files exist.
- Task commits `2eb2a41`, `8b50bb0`, `50f703a`, and `db46355` exist in repository history.
- Both the fixed original pre-reboot archive and the new post-repair G0 PASS archive verify from contained bytes.

---
*Phase: 01-host-readiness-gate*
*Completed: 2026-07-30*
