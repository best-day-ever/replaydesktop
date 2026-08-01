---
phase: quick-260730-swz
verified: 2026-07-30T21:02:08Z
status: human_needed
score: 7/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 6/7
  gaps_closed:
    - "Native callback and Rust forwarding-channel loss tuples are now published coherently; Rust sequence bounds use minimum/maximum affected sequence, and deterministic overlap plus concurrent exact-total tests pass."
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "During an approved maintenance window, build/install the patched native and Rust artifacts, then run scripts/verify-host-telemetry.sh --live."
    expected: "The bounded NvFBC/NVENC probe observes the deployed telemetry schema and restores the original service state plus configuration content and metadata."
    why_human: "The opt-in probe restarts the active user service, and the currently deployed rootfs binaries do not yet contain the new telemetry keys."
---

# Quick Task 260730-swz: Linux Host Loss-Aware Native Telemetry Verification Report

**Task Goal:** Implement the Linux-host native telemetry slice for ReplayDesktop: preserve existing NvFBC/NVENC stage timings, add actual encoded payload byte counts, bounded FIFO depth and occupancy, typed frame-drop reasons, and callback/channel telemetry-loss accounting through txproto and kyavservice; add exact native/Rust automated tests and reproducible parent patch artifacts, without client decoder, GUI, packaging, or cross-host latency claims.

**Verified:** 2026-07-30T21:02:08Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure in `fd3a11f`

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Existing NvFBC `acquired`, NVENC `encoding`/`encoded`, and Kymux-local `sent` timestamps retain their meanings and remain backward compatible. | ✓ VERIFIED | The legacy C entry/callback layout remains `const char *key`, `int64_t value`, and integer-return `tx_metrics_cb` in `metrics.h:27-34`. `acquired` remains after NvFBC grab and CUDA copy (`iosys_nvfbc.c:726-774`); `encoding` remains immediately before `avcodec_send_frame` (`encode.c:704-714`); `encoded` remains after `avcodec_receive_packet` (`encode.c:741-779`); `sent` remains after both socket writes (`packet_sink.c:195-210`). The legacy Rust `set_metrics_cb(FnMut)` remains at `txproto-rs/src/lib.rs:661-683`. |
| 2 | Every successfully encoded/sent frame reports its real `AVPacket::size`; configured bitrate is never presented as measured payload throughput. | ✓ VERIFIED | `encode.c:773-779` snapshots `out_pkt->size` and emits `encoded_payload_bytes`; `packet_sink.c:201-210` emits `in_pkt->size` only after payload write success. No added metric derives payload bytes from configured bitrate. |
| 3 | A configured FIFO limit is an enforced maximum, and observed gauges contain the exact current depth and configured limit from the owning FIFO. | ✓ VERIFIED | `fifo_template.c:164-176` and `:304-324` use the same `num_data_queued >= max_queued` boundary and a capacity-rechecking `while`. `:204-258` safely references exactly one effective bounded owner and rechecks linkage. `host_metrics.c:201-330` covers limit 1, pop/recovery, exact gauges, pass-through ownership, zero/multiple/unbounded owners, null/unlimited/pass-through behavior, two blocked producers, and deliberate broadcasts. The native test passed 25 consecutive direct repetitions plus Meson. |
| 4 | Capture and encoded-packet FIFO rejection emit stable stage/reason, PTS, timestamp, depth/limit when available, and cumulative count. | ✓ VERIFIED | NvFBC handles `ENOBUFS` at `iosys_nvfbc.c:780-800`; encoder handles it at `encode.c:784-805`. Both route exact owner gauges or a typed owner-unavailable result through the tested helpers at `metrics.c:131-246`. The production objects compiled with the existing build flags and `host_metrics` verified the exact drop batch. |
| 5 | Native callback and Rust full/closed-channel rejection produce truthful explicit telemetry-loss accounting. | ✓ VERIFIED | Native writers bracket both cumulative fields with one active-writer/generation word (`metrics.h:55-89`, `metrics.c:48-88`); a reader makes one attempt and emits the pair only when no writer overlaps. `host_metrics.c:160-173,421-513` deterministically pauses after the first field and then stress-checks concurrent exact totals. Rust applies the same protocol with `SeqCst` operations, initializes the first bound to `u64::MAX`, and uses `fetch_min`/`fetch_max` (`kyavservice/src/metrics.rs:50-175`). Its forced-overlap, delayed-minimum, and concurrent exact-total tests at `:370-535` all passed. Full and closed channel tests also passed. |
| 6 | Metrics callbacks remain bounded, nonblocking, and content-free. | ✓ VERIFIED | Native batches are fixed at 32 stack entries (`metrics.h:25`, `metrics.c:98-128`); no callback-delivery retry, sleep, or producer-value logging exists. Rust validates at most 32 entries and uses a bounded 32-batch `mpsc` with `try_send` (`txproto-rs/src/lib.rs:164-258`, `kyavservice/src/metrics.rs:231-275`). `--source` found no payload-content, credential, clipboard, or metric-value logging additions. |
| 7 | Changes replay from exact pinned txproto/Kymedia commits through parent-owned patches with no nested commit or gitlink update. | ✓ VERIFIED | `--replay` cloned txproto `82694c3` and Kymedia `e80eb6b`, applied 0008 then 0009 with exact allowlists, and preserved every nested HEAD. Parent commits keep `upstream/kyber-desktop` at `6c75cc2`; current nested HEADs also remain pinned. |

**Score:** 7/7 truths verified

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `patches/kyber/0008-txproto-host-telemetry.patch` | Exact-pin native implementation/test patch | ✓ VERIFIED | 1,312 lines; exactly 10 allowed paths; applies cleanly at txproto `82694c3`; reverse-check confirms the current applied native source matches it. |
| `patches/kyber/0009-kymedia-host-telemetry-forwarding.patch` | Exact-pin Rust bridge/forwarder patch | ✓ VERIFIED | 987 lines; exactly two allowed paths; applies cleanly at Kymedia `e80eb6b`; reverse-check confirms the current applied Rust source matches it. |
| `scripts/verify-host-telemetry.sh` | Source, test, replay, and opt-in live verifier | ✓ VERIFIED | 589 lines, executable, `bash -n` clean. `--source`, `--tests`, and `--replay` independently passed. `--live` was intentionally not run. |

## Commit and Repository Integrity

| Check | Result | Evidence |
|---|---|---|
| `499bb2a` scope | ✓ PASS | Adds only `patches/kyber/0008-txproto-host-telemetry.patch`. |
| `0e7542d` scope | ✓ PASS | Modifies 0008 and adds only `patches/kyber/0009-kymedia-host-telemetry-forwarding.patch`. |
| `09caad9` scope | ✓ PASS | Modifies `README.md` and adds only `scripts/verify-host-telemetry.sh`. |
| `fd3a11f` gap-closure scope | ✓ PASS | Modifies only patch artifacts 0008 and 0009; no source tree, gitlink, service, configuration, or unrelated path enters the commit. Its parent is exactly `09caad9c16018f54e2604b1fc63a6f6e19161122`. |
| Linear ancestry | ✓ PASS | Exact chain is `f9abdfd → 499bb2a → 0e7542d → 09caad9 → fd3a11f`. |
| Parent gitlink | ✓ PASS | `upstream/kyber-desktop` is `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f` at both `09caad9` and `fd3a11f`. |
| Nested HEADs | ✓ PASS | Kyber Desktop `6c75cc2`, Kysdk `5836202`, Kymedia `e80eb6b`, txproto `82694c3`. No nested commit was created or referenced. |
| Staging | ✓ PASS | `git diff --cached --name-status` was empty after verification. Existing unrelated untracked/planning files and the user-owned `0003-linux-zlib-pic.patch` were untouched. |

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| NvFBC/encoder/packet sink | `tx_metrics_cb` | Fixed scalar helper batches | ✓ WIRED | Ownership sites call `tx_metrics_push` / payload / queue / drop helpers with PTS and real owned values. |
| txproto-rs | Native callback return | `set_metrics_result_cb(Fn -> bool + Send + Sync)` | ✓ WIRED | Acceptance maps to C return 0; rejection/panic/invalid input maps to nonzero. Legacy `FnMut` API remains present. |
| kyavservice | Existing KyCom metrics endpoint | Bounded `try_send`, MessagePack, `MetricsPacket` | ✓ WIRED | `try_forward` takes only a coherent published snapshot, appends all four loss fields together, uses bounded `try_send`, and advances the reported generation only after acceptance (`kyavservice/src/metrics.rs:190-285`). |
| Native production sites | `host_metrics` target | Shared FIFO/metrics helpers | ✓ WIRED | Meson links the built txproto library; boundary, ownership, drop helper, and callback-loss tests execute the same implementations used by production. |

## Data-Flow Trace

| Fact | Ownership Source | Forwarding Path | Status |
|---|---|---|---|
| Encoded payload bytes | `out_pkt->size` after FFmpeg returns a packet | C helper → native callback → validated Rust bridge → bounded kyavservice channel → KyCom | ✓ FLOWING |
| Sent payload bytes | `in_pkt->size` after both local writes complete | Same path | ✓ FLOWING |
| Queue depth/limit | Owning bounded FIFO under its lock | C helper → callback → Rust/KyCom | ✓ FLOWING |
| Typed drop | `ENOBUFS` branch plus owner query and cumulative producer counter | C helper → callback → Rust/KyCom | ✓ FLOWING |
| Callback/channel loss | Versioned native/Rust atomic tuples | Coherent cumulative snapshot piggybacked on a later accepted batch | ✓ FLOWING |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Static ABI/path/content contract | `scripts/verify-host-telemetry.sh --source` | `SOURCE=PASS` | ✓ PASS |
| Native FIFO/metrics behavior | `scripts/verify-host-telemetry.sh --tests` | Named Meson `host_metrics` 1/1 passed, including deterministic partial-publication and concurrent exact-total subchecks; full txproto build passed | ✓ PASS |
| Concurrent capacity stress | Prior direct `host_metrics` repetitions plus current named Meson test | Previous 25/25 repetition check and current 1/1 compiled test pass; no regression | ✓ PASS |
| Rust bridge/forwarder behavior | `scripts/verify-host-telemetry.sh --tests` | txproto-rs 3/3 and kyavservice 8/8 passed. Output names confirm forced in-progress omission/recovery, delayed-minimum min/max, concurrent coherent exact totals, full/closed channel behavior, and no-loss behavior; fmt/build passed | ✓ PASS |
| Changed-path lint | Exact strict Clippy then scoped rerun | Exact command fails only at pre-existing `kyavservice/src/lib.rs:361`; rerun allowing that one lint passes; no changed Rust path appears in the strict failure | ✓ PASS |
| Exact-pin replay | `scripts/verify-host-telemetry.sh --replay` | `REPLAY=PASS` | ✓ PASS |
| Deployed live session | `scripts/verify-host-telemetry.sh --live` | Not run because it restarts the active host service | ? HUMAN |

## Probe Execution

| Probe | Result | Status |
|---|---|---|
| `--source` | Exact pins, allowlists, ABI/key/content checks passed | PASS |
| `--tests` | Native/Rust tests, production-object compiles, format, scoped warning gate, and kyavserver link passed | PASS |
| `--replay` | Both patches replayed at exact pins with no HEAD/gitlink drift | PASS |
| `--live` | Deliberately skipped; see Human Verification | HUMAN NEEDED |

## Coverage Assessment

No formal `REQUIREMENTS.md` IDs are assigned to this quick task.

| Plan Coverage Item | Status | Evidence |
|---|---|---|
| D1 — bounded FIFO and native primitives | ✓ SATISFIED | Native tests/build and repeated concurrent-capacity run pass. |
| D2 — real payload/queue/drop/loss facts through Rust | ✓ SATISFIED | Payload, queue, and drop data flow remains present; native and Rust loss snapshots are now coherently published and behaviorally exercised under forced and concurrent interleavings. |
| D3 — exact-pin parent patch replay | ✓ SATISFIED | `--replay` passes and commit/gitlink scopes are clean. |
| D4 — explicit deployed live probe | ? NEEDS HUMAN | The opt-in probe restarts the service and was not authorized during this verification. |

## Anti-Patterns and Disconfirmation Pass

| File | Line | Finding | Severity | Impact |
|---|---|---|---|---|
| `kyavservice/src/lib.rs` | 361 | `let_underscore_future` strict Clippy failure | ℹ️ PRE-EXISTING | Present at pinned Kymedia commit `e80eb6b`; not introduced by 0009. |
| `subprojects/txproto/src/packet_sink.c` | 422 | Unreferenced TODO about interrupting blocking socket I/O | ℹ️ PRE-EXISTING | Present at pinned txproto commit `82694c3`; unrelated to the telemetry patch. |

No added debt marker, placeholder, skipped test, payload-content log, credential log, clipboard content, client decoder, GUI, packaging, scanout, or cross-host latency claim was found. The verifier script's `XXXXXX` strings are `mktemp` templates, not debt markers.

### Required Disconfirmation Checks

- **Prior mixed-snapshot counterexample:** no longer succeeds. Both implementations expose an active writer in the publication word before changing related fields and reject a snapshot if the word changes or has an active writer.
- **Test-strength check:** native code deterministically stops between batch and entry publication; Rust deterministically stops after batch publication, delays the minimum sequence until all other writers complete, and separately stresses six writers plus two readers with exact final totals.
- **Boundedness check:** readers do not spin or retry; an overlapping snapshot is omitted and retried only by a future ordinary telemetry batch. Writers use atomic operations only and never wait for readers.

## Human Verification Required

### Deployed Host Telemetry Probe

**Test:** During an approved maintenance window, build/install the patched native and Rust artifacts, then run `scripts/verify-host-telemetry.sh --live`.

**Expected:** A bounded NvFBC/NVENC session is observed, deployed artifacts contain the telemetry schema, and the original service active state plus config content/metadata are restored.

**Why human:** The probe explicitly restarts the active user service. This verification did not restart or mutate it. The current service remained active/running with MainPID `3206561`, start timestamp `2026-07-30 17:43:08 CEST`, and config SHA-256 `3ad53c4455b886077be87618457869c3d5017189f4bc43bf114d240457ffd12e`, mode `600`, before and after all automated checks. Read-only binary inspection also shows the currently deployed rootfs artifacts do not yet contain the new telemetry keys, so deployment must precede this probe.

## Gaps Summary

The prior blocker is closed. Source inspection, deterministic overlap tests, concurrent exact-total tests, the full native/Rust build gate, static contract checks, exact-pin replay, commit boundaries, and service/config preservation all pass. All seven codebase must-haves are verified.

Overall status remains `human_needed`, not `passed`, because the plan's explicit `--live` gate restarts the active user service. Read-only inspection confirms the currently deployed rootfs artifacts still lack the new telemetry keys, so an approved build/install must precede that probe. This is a human/deployment gate, not an implementation gap.

---

_Verified: 2026-07-30T21:02:08Z_
_Verifier: gsd-verifier_
