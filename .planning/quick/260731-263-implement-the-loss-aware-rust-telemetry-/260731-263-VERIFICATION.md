---
phase: quick-260731-263-host-evidence-sequence
verified: 2026-07-31T03:30:23Z
status: passed
score: 7/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Quick 260731-263: Host Evidence Sequence Verification Report

**Quick-task goal:** Close host source gaps with PTS-correlated NvFBC
`capture_begin`, stable accepted host-batch producer sequence, coherent
rejection loss, and a direct terminal record after drain, using exact
prerequisite replay and without deployment.

**Verified:** 2026-07-31T03:30:23Z  
**Status:** passed  
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | The Linux NvFBC path samples `capture_begin` immediately before the real grab and publishes it with matching PTS/acquired only after a successful application-surface copy. | ✓ VERIFIED | `iosys_nvfbc.c:726-782` samples `av_gettime_relative()` immediately before `nvFBCToCudaGrabFrame`, exits/continues on grab, resolution, allocation, or copy errors, samples `acquired` after the checked `cuMemcpyDtoD`, and then publishes `{pts,capture_begin,acquired}`. The production NvFBC object compiled with the existing production arguments; the registered native test verifies one ordered begin per successful synthetic batch and unchanged callback-loss behavior. |
| 2 | Every host forward attempt has one stable monotonic sequence, and accepted batches preserve the native txproto prefix. | ✓ VERIFIED | `metrics.rs:284-309` allocates once before `try_send`, copies source entries in order, appends the same sequence, and records acceptance only after enqueue. `sequential_accepts_preserve_native_prefix_and_append_identity`, `simultaneous_writers_get_unique_sequences_and_nonregressing_acceptance`, and the saturation test passed. |
| 3 | Rejections stay nonblocking and retain coherent cumulative batch/entry/range loss that later accepted batches carry. | ✓ VERIFIED | The callback path uses bounded `try_send`; the atomic publication protocol in `metrics.rs:85-205` excludes torn snapshots. Recovery, monotonic totals, forced in-progress publication, min/max completion-order, and concurrent 12,000-rejection tests all passed. |
| 4 | Callback close drains accepted batches, then emits one direct terminal record with final attempted/accepted sequences and coherent cumulative loss; failure remains unconfirmed. | ✓ VERIFIED | `forward_stream` drains `rx` at `metrics.rs:325-339`, snapshots state, sends the terminal directly with the KyCom sender, and marks reported only after success. The drain/order/final-loss and direct-send-failure async tests passed. README lines 163-170 require a consumer-observed terminal and classify absence as `producer_terminal_unconfirmed`. |
| 5 | The terminal bypasses the bounded callback channel, and a final rejection with no later callback is disclosed when transport remains available. | ✓ VERIFIED | The terminal is constructed only after `while let Some(...)` completes and is passed directly to `send_metrics`, never to `try_send`. `accepted_batches_drain_before_terminal_and_final_loss_is_disclosed` covers an accepted batch followed by the final rejected attempt; `terminal_reports_no_accepted_batches_explicitly` covers rejection-only termination. |
| 6 | Existing metric keys, callback ABI, capture queue/encoder path, and MessagePack transport remain compatible. | ✓ VERIFIED | Patch 0012 touches only the NvFBC producer and registered test; patch 0013 touches only `kyavservice/src/metrics.rs`. Native entries are prefix-preserved, txproto-rs still exposes the result callback, `kymux.rs:43` still wires `metrics::forward`, and `send_metrics` preserves the existing `rmp_serde`/`MetricsPacket` KyCom envelope. Targeted native and Rust builds/tests pass. |
| 7 | Exact prerequisites replay before the new patches; registered tests pass; verification performs no deployment, service restart, or macOS-app mutation. | ✓ VERIFIED | Independent runs of `--source`, `--tests`, and `--replay` all exited 0. Replay proved `0008→0012` and `0009→0013` ordering and reverse byte identity. The verifier contains no deployment, service-control, remote-shell, or installed-app commands. |

**Score:** 7/7 truths verified (0 present-but-behavior-unverified)

The live NVIDIA streaming path was intentionally not started. That is outside
this quick-task boundary and is not needed to prove the source/control-flow
contract: exact source ordering, all failure branches, production compilation,
registered native behavior, Rust state transitions, MessagePack ordering, and
clean replay are deterministic here.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `patches/kyber/0012-txproto-capture-begin.patch` | Prerequisite-relative NvFBC begin/copy patch | ✓ VERIFIED | 127 lines; only `src/iosys_nvfbc.c` and `test/host_metrics.c`; applied in the working source and cleanly replayed/reversed at txproto `82694c3…`. |
| `patches/kyber/0013-kymedia-host-evidence-sequence.patch` | Sequence/loss/terminal Rust patch | ✓ VERIFIED | 569 lines; only `kyavservice/src/metrics.rs`; applied in the working source and cleanly replayed/reversed at Kymedia `e80eb6b…`. |
| `scripts/verify-host-evidence-sequence.sh` | Local source/test/replay verifier | ✓ VERIFIED | 587 lines; substantive exact-pin, allowlist, order, test, replay, and safe-scratch checks. Syntax and all three modes pass. |
| `README.md` | Application order and evidence semantics | ✓ VERIFIED | Lines 108-114 document prerequisite order; lines 146-177 define capture interval, sequence/loss layers, direct terminal, unconfirmed disconnect, and local-only verification. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `iosys_nvfbc.c` capture | `tx_metrics_push` | Same local `capture_begin`, post-copy `ts`, and `frame_pts` | ✓ WIRED | Lines 726, 727, 756-764, and 775-782 are strictly ordered; failure branches precede publication. |
| `MetricsForwarder::try_forward` | bounded channel/loss state | One pre-attempt sequence reused for accepted identity or rejection accounting | ✓ WIRED | Lines 284-309; sequential, rejection/recovery, concurrent writer, and saturation tests pass. |
| callback closure state | `forward_stream` terminal | Shared `Arc<ForwarderState>` and direct KyCom send after receiver drain | ✓ WIRED | Lines 325-368; terminal never enters the bounded callback channel; async order/failure tests pass. |
| exact pinned sources | patches 0012/0013 | `0008→0012` and `0008+0009→0013`, then reverse identity | ✓ WIRED | `--replay` applied both prerequisite-relative chains, compared resulting sources to the working tree, and restored prerequisite hashes exactly. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Downstream path | Status |
|---|---|---|---|---|
| `0012` / `iosys_nvfbc.c` | `capture_begin`, PTS, `acquired` | Host monotonic clock plus the successfully copied NvFBC frame | `tx_metrics_push` → txproto result callback → `MetricsForwarder` | ✓ FLOWING |
| `0013` / `metrics.rs` accepted batch | producer sequence and native entries | Shared atomic sequence plus txproto callback slice | ordered `Vec<Metrics>` → bounded mpsc → MessagePack `MetricsPacket` → KyCom | ✓ FLOWING |
| `0013` / terminal | final attempted/accepted sequence and cumulative loss | shared `ForwarderState` after all senders close | receiver drain → coherent snapshot → direct `send_metrics` | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Verifier parses | `bash -n scripts/verify-host-evidence-sequence.sh` | exit 0 | ✓ PASS |
| Native and Rust behavior | `scripts/verify-host-evidence-sequence.sh --tests` | registered native test + 5 stress runs passed; production NvFBC object compiled; 14/14 targeted Rust tests passed; fmt passed; changed-path clippy gate passed | ✓ PASS |
| Exact replay/reversal | `scripts/verify-host-evidence-sequence.sh --replay` | both chains applied at exact pins and reversed byte-for-byte | ✓ PASS |

### Probe Execution

| Probe | Command | Result | Status |
|---|---|---|---|
| Source contract | `scripts/verify-host-evidence-sequence.sh --source` | `SOURCE=PASS` | PASS |
| Registered tests | `scripts/verify-host-evidence-sequence.sh --tests` | `TESTS=PASS` | PASS |
| Prerequisite replay | `scripts/verify-host-evidence-sequence.sh --replay` | `REPLAY=PASS` | PASS |

### Commit and Nested-State Verification

| Check | Status | Evidence |
|---|---|---|
| Task 1 commit boundary | ✓ VERIFIED | `3f3e9b5` adds only patch 0012. |
| Task 2 commit boundary | ✓ VERIFIED | `5a57e0f` adds only patch 0013. |
| Task 3 commit boundary | ✓ VERIFIED | `ba5b985` changes only README and adds the verifier. |
| Parent gitlink | ✓ VERIFIED | `upstream/kyber-desktop` remains pinned at `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`. |
| Recursive nested pins | ✓ VERIFIED | Every recursive `git submodule status` entry has the expected clean-prefix pin; no `+`, `-`, or conflict marker was present. |
| Nested indexes | ✓ VERIFIED | Recursive staged-file inspection returned no staged paths. Existing dirty/untracked working material was preserved. |
| Scratch cleanup | ✓ VERIFIED | No verifier scratch directories remained after test/replay execution. |

### Requirements Coverage

No requirement IDs are declared by this quick plan (`requirements-completed`
is empty), so there is no phase requirement mapping to assess.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---:|---|---|---|
| `kyavservice/src/lib.rs` | 361 | `clippy::let-underscore-future` | ℹ️ Info | Pre-existing and outside patches 0012/0013. Strict clippy identifies only this line; the plan-authorized single-lint allow rerun passes with changed `metrics.rs` warning-free. |
| `scripts/verify-host-evidence-sequence.sh` | 393, 511 | `XXXXXX` | ℹ️ Info | Safe `mktemp` templates, not `XXX` debt markers. |

No added TODO/FIXME/XXX debt markers, placeholders, stubs, payload logging,
secret logging, or hollow wiring were found.

### Human Verification Required

None. All in-scope behavior is covered by deterministic source checks,
production compilation, registered native/Rust tests, in-memory KyCom protocol
tests, and exact replay. Live host deployment/streaming is explicitly outside
this split.

### Gaps Summary

No blocking or warning-level gaps found. The task goal is achieved and the
host-side contract is ready for the separately planned client bridge.

---

_Verified: 2026-07-31T03:30:23Z_  
_Verifier: gsd-verifier_
