---
phase: 01-host-readiness-gate
plan: 01
subsystem: evidence-foundation
tags: [rust, serde, sha2, cargo-deny, strict-json, supply-chain]

requires: []
provides:
  - Locked Rust 1.89.0 host-doctor package with six reviewed direct dependencies
  - Shared lowercase 32-byte SHA-256 identity contract
  - Strict version-dispatched G0EvidenceEnvelopeV1 with bounded raw extensions
  - Complete locked dependency inventory and deny-by-reviewed-graph policy
affects:
  - 01-02-doctor-cli
  - host-readiness-evidence
  - immutable-archives
  - dependency-policy

tech-stack:
  added:
    - Rust 1.89.0
    - serde 1.0.229
    - serde_json 1.0.151
    - sha2 0.11.0
    - libloading 0.9.0
    - rustix 1.1.4
    - x11rb 0.14.0
  patterns:
    - Strict final base schema with forward-compatible bounded raw extensions
    - Raw-payload SHA-256 verification before extension admission
    - Exact locked-package and direct-feature allowlists

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - DEPENDENCIES.md
    - deny.toml
    - src/lib.rs
    - src/model.rs
    - src/digest.rs
    - tests/fixtures/g0-envelope-v1-foundation.json
  modified:
    - .gitignore

key-decisions:
  - "Freeze V1 base fields now; later host facts live in independently validated extension payloads."
  - "Require all four known G0 records and derive overall status/reasons from their ordered terminal states."
  - "Use sha2 behind the sole 32-byte lowercase SHA-256 identity type and byte/reader/file helpers."
  - "Deny dependency graph drift with an exact package allowlist and exact direct-feature policies."

patterns-established:
  - "Strict admission: duplicate keys, unknown base fields, malformed Unicode/numbers, oversized data, and digest mismatches fail closed."
  - "Forward compatibility: unknown well-formed extension payload bytes remain opaque, preserved, bounded, and digest-verified."
  - "Supply-chain evidence: Cargo.lock, metadata, tree, checksums, licenses, provenance, features, and build scripts are reviewed together."

requirements-completed: [HOST-01]

coverage:
  - id: D1
    description: "Locked Rust 1.89.0 package with exactly six reviewed direct dependencies and a complete resolved inventory."
    requirement: HOST-01
    verification:
      - kind: other
        ref: "cargo tree --locked --target all --edges normal,build,dev"
        status: pass
      - kind: other
        ref: "cargo metadata --locked --format-version 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "Shared SHA-256 contract and strict forward-compatible G0EvidenceEnvelopeV1 admission."
    requirement: HOST-01
    verification:
      - kind: unit
        ref: "cargo test --locked (22 tests)"
        status: pass
      - kind: other
        ref: "cargo clippy --locked --all-targets -- -D warnings"
        status: pass
    human_judgment: false
  - id: D3
    description: "Exact dependency denial policy covering packages, sources, features, licenses, duplicates, and advisories."
    requirement: HOST-01
    verification:
      - kind: other
        ref: "Python tomllib parse plus metadata/allowlist/checksum equality audits"
        status: pass
    human_judgment: true
    rationale: "The policy is structurally and semantically audited, but cargo-deny 0.20.2 was unavailable so its conditional check did not run."

duration: 33 min
completed: 2026-07-27
status: complete
---

# Phase 01 Plan 01: Evidence Foundation Summary

**Locked Rust evidence foundation with a strict raw-preserving G0 V1 envelope, one SHA-256 contract, and an exact reviewed dependency-denial policy**

## Performance

- **Duration:** 33 min
- **Started:** 2026-07-26T23:56:59Z
- **Completed:** 2026-07-27T00:30:21Z
- **Tasks:** 3
- **Files modified:** 10

## Accomplishments

- Froze the final V1 base schema and public version dispatcher while preserving bounded unknown extension payload bytes and verifying their raw SHA-256 identities.
- Hardened digest and JSON admission with 22 locked tests covering known vectors, arbitrary reader chunks, I/O failures, duplicate keys, exact integers, Unicode, size limits, status consistency, and semantic separation.
- Audited all 30 resolved registry packages, eight build scripts, licenses, provenance, selected features, checksums, and parent edges, then encoded that review as an exact `deny.toml` policy.

## Task Commits

Each task was committed atomically:

1. **Task 1: Round-trip the final evidence envelope through the locked foundation**
   - `621c931` — RED: failing foundation/forward-compatibility tests
   - `e93e717` — GREEN: strict V1 evidence foundation
2. **Task 2: Harden shared digest and recursive JSON admission**
   - `d9f2848` — RED: failing digest/admission integrity tests
   - `68f119b` — GREEN: known gate summary integrity
3. **Task 3: Finalize the reproducible dependency evidence and denial policy**
   - `d23f67b` — complete dependency inventory and denial policy

Additional verified deviation fix:

- `c2e1a12` — removed the stale unreachable decoder stub found by the final scan

**Plan metadata:** committed with this summary and the sequential state updates.

## Files Created/Modified

- `.gitignore` — excludes Cargo build output from repository status.
- `Cargo.toml` — pins the package and six direct dependency feature surfaces.
- `Cargo.lock` — records the complete exact registry resolution and checksums.
- `rust-toolchain.toml` — fixes Rust 1.89.0 with rustfmt and clippy.
- `DEPENDENCIES.md` — records direct legitimacy and the complete transitive review.
- `deny.toml` — denies unreviewed packages, sources, features, licenses, wildcards, and duplicate versions.
- `src/lib.rs` — exports the evidence and digest contracts and retains the foundation regression tests.
- `src/model.rs` — implements bounded, strict, version-dispatched V1 admission and hardening tests.
- `src/digest.rs` — implements the sole SHA-256 type/helpers and deterministic boundary tests.
- `tests/fixtures/g0-envelope-v1-foundation.json` — preserves the original V1 compatibility fixture.

## Decisions Made

- The V1 base is final. Output, NvFBC, NVENC, and later facts remain extension records so future payload evolution does not mutate the base decoder.
- All four stable known records are mandatory, and the base status/reasons must exactly match their ordered non-PASS terminal states.
- Known-extension payload schemas remain outside generic base decoding; the base owns only JSON integrity, bounds, identity, and record consistency.
- The reviewed dependency graph is an exact allowlist. Any new package or direct-feature drift requires updating both the evidence inventory and denial policy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added Cargo build-output exclusion**

- **Found during:** Task 1 (foundation scaffold)
- **Issue:** Locked Cargo verification creates `target/`; leaving it unignored would violate the executor's generated-file hygiene requirement.
- **Fix:** Added `/target/` to `.gitignore`.
- **Files modified:** `.gitignore`
- **Verification:** Repeated Cargo checks leave `git status --short` clean.
- **Committed in:** `621c931`

**2. [Rule 1 - Bug] Removed stale decoder stub**

- **Found during:** Final plan stub scan
- **Issue:** The completed decoder still exposed an unreachable `NotImplemented` error variant and message from the RED scaffold.
- **Fix:** Removed the stale variant and display arm so the public error surface matches shipped behavior.
- **Files modified:** `src/model.rs`
- **Verification:** `cargo clippy --locked --all-targets -- -D warnings`, all 22 tests, and the repeated stub scan pass.
- **Committed in:** `c2e1a12`

---

**Total deviations:** 2 auto-fixed (1 blocking hygiene issue, 1 stale-code bug).
**Impact on plan:** Both changes preserve correctness and repository hygiene without expanding the evidence contract.

## Issues Encountered

- `cargo-deny 0.20.2` is not installed on this host. Per the plan, no tool was installed or substituted; the conditional `cargo deny check` was not executed and is not represented as a pass. This unrun conditional verification is recorded in `.planning/WINDOWS.md`.

## Authentication Gates

None.

## Human Verification

- The Task 1 tracer gate was approved after its locked round-trip and bounds tests passed. The continuation re-ran those tests before Tasks 2–3.

## Verification Results

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS |
| `cargo tree --locked --edges normal,build,dev` | PASS |
| `cargo metadata --locked --format-version 1` | PASS |
| `cargo test --locked` | PASS — 22 unit tests, 0 failures |
| Inventory/package/checksum/source equality audits | PASS |
| `cargo deny check` with exact `cargo-deny 0.20.2` | NOT RUN — pinned executable unavailable |

## TDD Gate Compliance

- Task 1: RED `621c931` precedes GREEN `e93e717`.
- Task 2: RED `d9f2848` precedes GREEN `68f119b`.
- Both GREEN states pass the full locked suite.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for `01-02-PLAN.md` to add the doctor CLI, bounded workers, diagnostic admission, and atomic evidence readback without changing the V1 base decoder.
- The conditional cargo-deny check remains open in the broken-windows ledger until the exact pinned executable is available; it does not invalidate the locked Cargo/test evidence completed here.

## Self-Check: PASSED

- All 11 claimed output/tracking files exist.
- All six production/deviation commits are present in repository history.
- Coverage metadata classifies without schema errors: two automated deliverables and one explicit human-judgment item.

---
*Phase: 01-host-readiness-gate*
*Completed: 2026-07-27*
