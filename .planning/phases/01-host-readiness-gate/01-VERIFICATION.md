---
phase: 01-host-readiness-gate
verified: 2026-07-30T01:08:58Z
status: gaps_found
score: 1/4 must-haves verified
behavior_unverified: 0
overrides_applied: 0
mvp_user_story_valid: false
audit_only_override: true
gaps:
  - truth: "The MVP phase has a canonical user-story goal whose outcome can be verified."
    status: failed
    reason: "ROADMAP mode is mvp, but the phase goal does not match the required 'As a ..., I want ..., so that ....' contract. user-story.validate returned valid=false. This report is an audit-only verification produced under an explicit orchestrator override; it does not waive the format guard."
    artifacts:
      - path: ".planning/ROADMAP.md"
        issue: "Phase 1 goal has no actor, capability, or outcome slots in canonical user-story form."
    missing:
      - "Run `$gsd-mvp-phase 1` and replace the Phase 1 goal with a canonical user story without reducing its existing host-readiness contract."
  - truth: "Doctor reliably identifies and blocks every required invalid-host condition with specific remediation."
    status: failed
    reason: "Normal diagnostic builds can lose the build-time NVML source identity required by the evidence envelope, causing a diagnostic result to fail persistence with exit 74 instead of the intended diagnostic exit 2. The live output collector also rejects a valid output when the optional RandR non-desktop property is absent, and zero-count DRM arrays can invoke from_raw_parts with a null pointer."
    artifacts:
      - path: "src/native_nvml.rs"
        issue: "compiled_source_metadata() returns None without replay_nvml_source; evidence persistence still requires the identity."
      - path: "src/output_mapping.rs"
        issue: "Absent non-desktop property is treated as true; null zero-length DRM arrays are passed to from_raw_parts."
      - path: "tests/host_doctor_cli.rs"
        issue: "Named Wayland/mismatch diagnostic test exits 74 rather than the asserted exit 2."
    missing:
      - "Make authorized NVML source identity/digest available to every build that can emit diagnostic evidence."
      - "Treat an absent optional RandR non-desktop property as desktop unless other evidence says otherwise."
      - "Represent zero-count native arrays without constructing slices from null pointers."
      - "Restore passing named tests for specific invalid-host diagnostics."
  - truth: "The selected output is proven through a live NvFBC shared-CUDA capture with complete cleanup and containment accounting."
    status: failed
    reason: "The live lease wrapper returns a successful consumer result even when post-consumer cleanup or source-containment observation records failure. Durable evidence can therefore claim cleanup_complete=true without the wrapper making cleanup failure admission-blocking."
    artifacts:
      - path: "src/native_nvfbc.rs"
        issue: "with_live_selected_capture_lease() prefers consumed over observation.failure after cleanup."
      - path: "src/probe.rs"
        issue: "The probe consumes the wrapper result and has no independent authoritative cleanup oracle."
    missing:
      - "Return cleanup/containment failure even when the frame consumer succeeded."
      - "Add and pass one named test in which consumer success is followed by cleanup or containment failure."
  - truth: "Every advertised eligible NVENC codec/profile/chroma/bit-depth tuple receives an exact live 3840x2160@60 one-frame attempt on the actual GPU and driver."
    status: failed
    reason: "The live policy entry hardcodes GPU generation to Unknown, which skips AV1 as generation-ineligible. Any requested output format other than NV12 is returned as Unsupported before native capture/encode setup. Both outcomes are non-blocking, so several policy positions receive no hardware attempt while final G0 can still pass."
    artifacts:
      - path: "src/native_nvenc.rs"
        issue: "observe_live_nvenc_policy() evaluates Unknown generation; non-NV12 requests short-circuit before the native provider; GenerationIneligible and Unsupported are non-blocking."
      - path: "src/model.rs"
        issue: "Unknown generation reports AV1 encode unsupported."
      - path: "artifacts/validation/g0/post-repair"
        issue: "The durable PASS records gpu_generation=unknown, AV1 provider_invoked=false, and non-NV12 provider_invoked=true without native config/resource/copy/stream proof."
    missing:
      - "Derive and persist the actual GPU generation from authoritative runtime capability evidence."
      - "Invoke the native provider for every eligible policy tuple, including supported 4:4:4 and 10-bit formats."
      - "Make skipped/unsupported positions admission-blocking unless an authoritative native capability result proves them ineligible."
  - truth: "A successful one-frame probe proves a complete, parseable keyframe/access unit for the exact advertised tuple."
    status: failed
    reason: "The H.264, HEVC, and AV1 inspectors accept short header fragments. H.264 proof omits PPS and coded payload, HEVC proof omits VPS/PPS and coded payload, and AV1 proof accepts sequence/frame-header bits without tile/frame payload. The passing test constructs these fragments, so it does not establish decodable one-frame output."
    artifacts:
      - path: "src/nvenc_bitstream.rs"
        issue: "Codec inspectors validate selected headers, not a complete decodable access unit."
      - path: "tests/host04_bitstream.rs"
        issue: "The all-seven-position test passes synthetic header fragments as keyframes."
      - path: "artifacts/validation/g0/post-repair"
        issue: "Archive stores hashes and claimed proof fields, but no independently decodable access-unit fixture."
    missing:
      - "Require all codec parameter sets and actual coded frame/tile payload appropriate to each codec."
      - "Persist or independently decode the exact encoded access unit used for admission."
      - "Add negative tests proving the current header-only fragments are rejected."
  - truth: "Durable G0 PASS is emitted only after complete HOST-01 through HOST-04 proof and before later media evidence is admitted."
    status: failed
    reason: "The final archive is internally hash-valid, but the evaluator accepts semantically incomplete HOST-03/HOST-04 records described above. The production run path also does not enforce the Plan 10 link that verifies the immutable foundation/original archive before collecting final live evidence."
    artifacts:
      - path: "src/lib.rs"
        issue: "G0 evaluation trusts extension Pass statuses whose producers can omit required hardware attempts or cleanup failures."
      - path: "src/evidence.rs"
        issue: "HOST-04 binding checks advertised attempts but does not reject unknown-generation skips, pre-native unsupported tuples, or header-only bitstream proof."
      - path: "src/archive.rs"
        issue: "Archive verification proves integrity and envelope semantics, not the missing hardware semantics."
    missing:
      - "Strengthen extension validation so every required cleanup, tuple attempt, actual generation, and complete bitstream proof is admission-blocking."
      - "Wire foundation/original-archive verification into the production final run path before live probe collection."
      - "Regenerate G0 evidence only after the strengthened verifier rejects the current post-repair archive."
  - truth: "Changed Rust targets satisfy the phase warnings-denied quality gate."
    status: failed
    reason: "cargo clippy --locked --all-targets -- -D warnings exits 101 on dead code in native_nvenc.rs and native_nvfbc.rs."
    artifacts:
      - path: "src/native_nvenc.rs"
        issue: "requested_output and unsupported_attempt are dead code in the active build."
      - path: "src/native_nvfbc.rs"
        issue: "The no-source with_live_selected_capture_lease implementation is dead code in the active build."
    missing:
      - "Remove, correctly feature-gate, or use the dead code and rerun the single workspace clippy gate."
deferred:
  - truth: "Host doctor recognizes valid distro-specific Xorg executable locations beyond /usr/lib/Xorg."
    addressed_in: "Phase 9"
    evidence: "Phase 9 explicitly qualifies Arch/CachyOS, Ubuntu 24.04, and Rocky 9 real X11/NVIDIA hosts; COMP-04 owns those distro rows."
unverified_prohibition_flags:
  - statement: "PLAN prohibitions use legacy free-text verification descriptions instead of the canonical test/judgment tiers."
    flagged: true
    disposition: "Non-authoritative manual review recommended; no prohibition was silently counted as passed."
---

# Phase 1: Host Readiness Gate Verification Report

**Phase Goal:** An operator has a real Xorg/NVIDIA host whose selected physical-output capture and encode prerequisites are proven; no later media evidence is admissible until they pass.
**Verified:** 2026-07-30T01:08:58Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## User Flow Coverage

The phase is marked `mode: mvp`, but its goal is not a valid user story. The canonical validator reported all three slots missing: `As a ...`, `I want ...`, and `so that ...`. Per orchestrator direction, the table below is an audit-only flow derived from the existing goal; it is not a waiver of the MVP format guard.

| Step | Expected | Codebase evidence | Status |
| --- | --- | --- | --- |
| 1. Run the host doctor | Every required invalid-host condition is blocked with specific remediation | The collectors and typed failures exist, but a named diagnostic test exits 74 during persistence rather than its expected diagnostic exit 2; valid-host collection also has optional-property and zero-count FFI defects | ✗ FAILED |
| 2. Select one physical X11 output | Exactly one output is identified with XRandR name, dimensions, refresh, origin, and owning GPU | The durable archive records `DP-0.3`, 3840x2160, exact rational refresh, origin, target identity, and GPU/NVML identity | ✓ VERIFIED |
| 3. Prove live capture and encode | NvFBC shared-CUDA cleanup and every eligible NVENC tuple are proven on the actual GPU | Cleanup failure can be dropped; generation is `unknown`; AV1 is skipped; non-NV12 positions never receive a native attempt; header fragments count as keyframes | ✗ FAILED |
| 4. Admit later work only after G0 | Durable G0 PASS is impossible unless all prerequisites are complete | The hash-valid post-repair archive is marked PASS despite the incomplete capture/encode semantics | ✗ FAILED |

**Outcome:** The selected output is observable, but the host is not admission-safely proven. The phase goal is not achieved.

## Goal Achievement

### Observable Truths

| # | Roadmap contract truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Doctor identifies and blocks non-Xorg, NVIDIA mismatch/NVML failure, missing physical output, `/dev/uinput`, and DRM/render access with specific remediation | ✗ FAILED — BLOCKER | The typed checks exist, but `native_nvml::compiled_source_metadata()` can make diagnostic evidence non-persistable. `host01_native_current_wayland_driver_mismatch_is_specific_and_deterministic` exits 74 instead of 2. `output_mapping.rs` also mishandles absent optional properties and null zero-length arrays. |
| 2 | Operator selects exactly one physical X11 output and sees its name, dimensions, refresh, origin, and owning GPU | ✓ VERIFIED | Post-repair evidence contains one selected `DP-0.3` 3840x2160 output with exact refresh/origin/target/GPU binding. Archive hashes and semantic envelope verify independently. |
| 3 | Doctor creates selected-output NvFBC shared-CUDA capture, accounts every copy, and performs one-frame probes for every advertised NVENC tuple | ✗ FAILED — BLOCKER | Cleanup failure is not authoritative after consumer success. Live NVENC uses `Unknown` generation, skips AV1, short-circuits non-NV12 requests, and accepts incomplete header fragments as one-frame proof. |
| 4 | Durable G0 PASS occurs only on a completely proven real host and blocks dependent work on any failed check | ✗ FAILED — BLOCKER | The final archive verifies cryptographically but records G0 PASS with `gpu_generation=unknown`, uninvoked AV1 providers, non-native unsupported positions, and non-decodable proof semantics. |

**Score:** 1/4 truths verified (0 present-but-behavior-unverified)

### Plan-Level Must-Have Audit

All ten PLAN files and all 51 plan-frontmatter truths were read. They were deduplicated under the four roadmap contract truths above; plan truths add detail but do not reduce roadmap scope.

| Plan | Focus | Resolution |
| --- | --- | --- |
| 01-01 | Evidence schema, digests, dependency discipline | ✓ VERIFIED — substantive strict envelope/digest code and tests exist |
| 01-02 | CLI, worker isolation, atomic evidence | ✗ FAILED — diagnostic evidence persistence can replace the intended diagnostic result with infrastructure exit 74 |
| 01-03 | Local Xorg and NVML proof | ✓ VERIFIED for the archived current host; cross-distro executable portability is deferred to Phase 9 |
| 01-04 | Immutable pre-reboot evidence | ✓ VERIFIED — index/manifest/evidence/binary hashes verify and the pre-reboot archive remains a durable FAIL |
| 01-05 | Pure output mapping/selection contract | ✓ VERIFIED for the modeled contract |
| 01-06 | Live output collection and selected-output evidence | ✗ FAILED — absent optional `non-desktop` is classified as non-desktop, and zero-count DRM arrays have an unsafe null-slice path |
| 01-07 | NvFBC source/model contract | ✗ FAILED — fixture/diagnostic operation remains coupled to unavailable build-time source identity |
| 01-08 | Live NvFBC capture and cleanup | ✗ FAILED — post-consumer cleanup/containment failure can be ignored |
| 01-09 | NVENC policy and bitstream proof | ✗ FAILED — policy is structurally closed but the bitstream inspectors accept incomplete fragments |
| 01-10 | Final live G0 and post-repair archive | ✗ FAILED — archive integrity passes, but required tuple attempts and production preflight wiring are incomplete |

## Required Artifacts

All file artifacts declared by PLAN frontmatter exist and are substantive. The artifact query reported `EISDIR` for the two archive-root directory declarations; those roots and their immutable files were checked manually.

| Artifact group | Expected | Status | Details |
| --- | --- | --- | --- |
| `src/model.rs`, `src/evidence.rs`, `src/digest.rs`, fixtures | Strict evidence foundation | ✓ VERIFIED | Versioned records, canonical digests, extension validation, and negative fixtures are substantive and wired |
| `src/cli.rs`, `src/lib.rs`, `src/probe.rs`, `src/currentness.rs` | Fresh-run CLI/worker/evidence path | ⚠️ PARTIAL | The path is wired, but diagnostic persistence can fail because the authorized source identity is absent |
| `src/local_xorg.rs`, `src/native_nvml.rs` | Xorg/NVML truth | ⚠️ PARTIAL | Current archived host passes; normal diagnostic/no-source operation is not reliable |
| `artifacts/validation/g0/pre-reboot`, `artifacts/validation/g0/post-repair` | Immutable G0 evidence | ✓ VERIFIED for integrity | Both archives verify; pre-reboot remains FAIL and post-repair remains PASS |
| `src/output_mapping.rs` and output tests | Exact physical output mapping | ✗ FAILED | Current selected output is proven, but valid-output and native-memory edge paths are defective |
| `src/native_nvfbc.rs` and HOST-03 tests | Live shared-CUDA capture and cleanup | ✗ FAILED | Live data flows, but cleanup/containment failure is not authoritative |
| `src/native_nvenc.rs`, `src/nvenc_bitstream.rs`, HOST-04 tests | Exact capability/encode/keyframe proof | ✗ FAILED | Hardware positions can be skipped and incomplete streams accepted |
| `src/archive.rs`, `docs/validation-g0.md` | Durable archive verification and runbook | ⚠️ PARTIAL | Archive integrity works; production final-run ordering is procedural rather than enforced |

## Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `src/lib.rs` | `src/probe.rs` | `run` / `diagnose` fresh-run backend | ✓ WIRED | CLI dispatch creates a run, invokes probes, evaluates, persists, and reads back evidence |
| `src/probe.rs` | Xorg/NVML/output collectors | HOST-01/HOST-02 probe dispatch | ✓ WIRED | Real collectors feed the evidence model |
| `src/probe.rs` | `src/native_nvfbc.rs` | `ProbeId::NvfbcCapture` and `observe_live_selected_capture` | ✓ WIRED | The PLAN regex used stale naming, but the actual link exists |
| `src/native_nvfbc.rs` | `src/native_nvenc.rs` | Live lease consumed by encoder probe | ⚠️ PARTIAL | NV12 flows, but cleanup failure can be discarded and non-NV12 requests do not reach native setup |
| `src/native_nvenc.rs` | `src/nvenc_bitstream.rs` | Successful encode result inspection | ⚠️ PARTIAL | Inspector is invoked, but its acceptance contract is too weak |
| `src/lib.rs` | `src/evidence.rs` | Extension binding and G0 evaluation | ⚠️ PARTIAL | Mechanically wired; semantic validation permits incomplete proof |
| Production final run | Immutable foundation/original archive | Required preflight verification | ✗ NOT_WIRED | No production link enforces this Plan 10 ordering before live collection |
| `src/archive.rs` | archive index/manifest/evidence/binary | SHA-256 and envelope verification | ✓ WIRED | Independent CLI checks pass for both durable archives |

## Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Produces real data | Status |
| --- | --- | --- | --- | --- |
| Selected-output evidence | Output identity, mode, origin, GPU | Live XRandR/NVML collection | Yes — persisted as `DP-0.3` 3840x2160 | ✓ FLOWING |
| NvFBC evidence | Source identity, lease, frame, copy ledger, cleanup | Live NvFBC native provider | Partly — live data is present, but cleanup failure can be hidden | ✗ UNSOUND |
| NVENC evidence | Generation, tuple attempts, config/resources/copies/stream | Live NVENC policy/provider | No for all positions — generation is hardcoded unknown and multiple formats short-circuit | ✗ HOLLOW |
| G0 status | Four extension statuses | `src/lib.rs` evaluator | Mechanically yes, semantically incomplete | ✗ UNSOUND |
| Archive verification | Index, manifest, evidence, binary hashes | Immutable post-repair/pre-reboot directories | Yes | ✓ FLOWING |

## Behavioral Spot-Checks

Only named checks and immutable archive reads were run; no server, external service, state mutation, or repeated full-suite filtering was used.

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Verify post-repair archive independently | Archived binary `verify-archive --index artifacts/validation/g0/post-repair/index.json` | Exit 0; `verified`; evidence status `pass` | ✓ PASS (integrity only) |
| Verify original pre-reboot archive remains readable | Archived binary `verify-archive --index artifacts/validation/g0/pre-reboot/index.json` | Exit 0; `verified`; evidence status `fail` | ✓ PASS |
| Authorized NVML source identity exists in normal build | `cargo test --locked native_nvml::tests::nvml_source_abi_authorized_header_matches_complete_rust_contract -- --exact` | Exit 101; panic: authorized source must be compiled | ✗ FAIL |
| Specific Wayland/driver diagnostic survives persistence | `cargo test --locked --test host_doctor_cli host01_native_current_wayland_driver_mismatch_is_specific_and_deterministic -- --exact` | Exit 101; actual exit 74, expected 2 | ✗ FAIL |
| Claimed all-position bitstream proof test | `cargo test --locked --test host04_bitstream host04_bitstream_all_seven_policy_positions_prove_from_keyframes -- --exact` | Exit 0, but fixtures are header fragments; this is evidence of a weak oracle, not goal proof | ✗ FAIL |
| Formatting | `cargo fmt --all -- --check` | Exit 0 | ✓ PASS |
| Warnings-denied quality gate | `cargo clippy --locked --all-targets -- -D warnings` | Exit 101; three dead-code errors | ✗ FAIL |

## Probe Execution

No conventional `scripts/**/probe-*.sh` files or PLAN-declared shell probe paths exist. Phase probes are Rust/native doctor paths and were assessed through immutable archive verification plus the named behavioral checks above.

## Requirements Coverage

| Requirement | Source plans | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| HOST-01 | 01-01 through 01-04 | Block invalid Xorg/NVIDIA/input/render/output hosts with specific remediation | ✗ BLOCKED | Diagnostic persistence and live output-collector defects prevent reliable fulfillment |
| HOST-02 | 01-05, 01-06 | Select exactly one physical output and report exact mode/origin/GPU | ⚠️ PARTIAL | Current `DP-0.3` proof is complete, but valid outputs can be rejected when the optional property is absent |
| HOST-03 | 01-07, 01-08 | Prove shared-CUDA capture and every conversion/copy through cleanup | ✗ BLOCKED | Cleanup/containment failure can be lost after consumer success |
| HOST-04 | 01-09, 01-10 | Account capture-to-NVENC registration/mapping/input/copy and exact tuple probes | ✗ BLOCKED | Unknown generation, skipped/pre-native positions, and incomplete stream proof |

All Phase 1 requirement IDs declared by plans match the ROADMAP/REQUIREMENTS mapping. No Phase 1 requirement is orphaned.

## Adversarial Review Finding Validation

SUMMARY claims were not used as evidence. The eight blocker-tier findings in `01-REVIEW.md` were checked against the implementation and durable evidence:

| Finding | Independent resolution | Goal impact |
| --- | --- | --- |
| CR-01 build-dependent NVML evidence | ✓ CONFIRMED in `native_nvml.rs` and two failing named tests | Blocks HOST-01 reliability |
| CR-02 live NVENC generation hardcoded unknown | ✓ CONFIRMED in `native_nvenc.rs`; durable archive says `unknown` and skips AV1 | Blocks HOST-04 |
| CR-03 non-NV12 positions short-circuit | ✓ CONFIRMED before native resource/config setup; durable archive lacks those proofs | Blocks HOST-04 |
| CR-04 bitstream oracle accepts fragments | ✓ CONFIRMED in parser and synthetic passing fixtures | Blocks HOST-04 |
| CR-05 cleanup/containment failure ignored | ✓ CONFIRMED in live lease result precedence | Blocks HOST-03 |
| CR-06 null zero-length native slices | ✓ CONFIRMED at DRM connector/property slice construction | Blocks safe HOST-01/HOST-02 collection |
| CR-07 Xorg path hardcoded to `/usr/lib/Xorg` | ✓ CONFIRMED | Deferred only to Phase 9's explicit distro qualification contract |
| CR-08 absent optional `non-desktop` property treated as true | ✓ CONFIRMED | Blocks valid HOST-02 output discovery |

## Anti-Patterns Found

| File | Line/area | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| `src/native_nvenc.rs` | Live policy entry | Hardcoded `GpuGeneration::Unknown` | 🛑 BLOCKER | AV1 can be skipped without actual GPU proof |
| `src/native_nvenc.rs` | Requested-output branch | Static `Unsupported` before native provider | 🛑 BLOCKER | Eligible tuples can receive no real attempt |
| `src/nvenc_bitstream.rs` | Codec inspectors | Header presence used as complete one-frame proof | 🛑 BLOCKER | Invalid/incomplete streams can pass G0 |
| `src/native_nvfbc.rs` | Live lease wrapper | Consumer success overrides cleanup failure | 🛑 BLOCKER | Incomplete containment can pass |
| `src/output_mapping.rs` | RandR property handling | Missing optional value treated as disqualifying true | 🛑 BLOCKER | Valid physical outputs can disappear |
| `src/output_mapping.rs` | DRM FFI slices | `from_raw_parts(null, 0)` path | 🛑 BLOCKER | Undefined behavior in a valid empty-array case |
| `src/local_xorg.rs` | Executable identity | Exact `/usr/lib/Xorg` allow-list | ⚠️ DEFERRED | Other supported distros are Phase 9 scope |
| Changed Rust targets | Clippy gate | Dead code under `-D warnings` | 🛑 BLOCKER | Required quality gate fails |

No unreferenced `TBD`, `FIXME`, or `XXX` debt marker was found in the phase source/test set. Matches for legacy “placeholder” terminology in `evidence.rs` are typed compatibility handling, not user-visible stubs.

## Deferred Items

| # | Item | Addressed In | Evidence |
| --- | --- | --- | --- |
| 1 | Recognize valid distro-specific Xorg executable locations beyond `/usr/lib/Xorg` | Phase 9 | Phase 9 and COMP-04 explicitly own real-host qualification for Arch/CachyOS, Ubuntu 24.04, and Rocky 9 |

Deferred scope does not excuse or mask the current cleanup, capability, tuple-attempt, stream-proof, or G0-admission failures.

## Human Verification Required

None before remediation. Automated/code-level evidence already demonstrates blockers, so manual testing cannot turn this phase green. PLAN prohibitions use noncanonical legacy verification descriptions and remain prominently flagged for later human review rather than being silently passed.

## Gaps Summary

The immutable evidence system is real and its archive hashes verify. The selected physical output is also genuinely recorded. Those facts do not establish the phase goal: the final G0 evaluator can admit a run whose cleanup failed, whose actual GPU generation was never established, whose AV1 and non-NV12 policy positions were not natively attempted, and whose encoded “frame” may be only a header fragment. The current post-repair PASS is therefore integrity-valid but goal-invalid.

Close the structured gaps in frontmatter, restore the warnings-denied gate, regenerate the live archive under the stronger semantics, and then re-run phase verification. The MVP goal must also be converted to a canonical user story before a normal MVP verification can pass.

---

_Verified: 2026-07-30T01:08:58Z_
_Verifier: the agent (gsd-verifier)_
