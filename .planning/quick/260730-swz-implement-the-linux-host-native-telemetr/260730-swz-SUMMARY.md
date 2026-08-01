---
phase: quick
plan: 260730-swz
subsystem: linux-host-telemetry
tags: [c11-atomics, meson, rust, tokio, nvfbc, nvenc, kymux]

requires:
  - phase: 260730-j91
    provides: pinned Kyber desktop source and reproducible Linux host baseline
  - phase: 260730-mqr
    provides: working NvFBC, NVENC, and Kymux-local sender path
  - phase: 260730-ruu-research
    provides: ownership-point and telemetry semantics research
provides:
  - bounded native FIFO and callback-loss telemetry primitives
  - real host payload, queue, drop, and telemetry-loss facts
  - loss-aware Rust forwarding with legacy callback compatibility
  - exact-pin source, test, replay, and explicit live verification modes
affects: [host-readiness-gate, 4k60-qualification, client-metrics-aggregation]

tech-stack:
  added: []
  patterns:
    - fixed-size stack telemetry batches with coherent versioned atomic loss snapshots
    - exact owning-FIFO gauges with typed unavailable results
    - parent-owned patches for all nested upstream changes

key-files:
  created:
    - patches/kyber/0009-kymedia-host-telemetry-forwarding.patch
    - scripts/verify-host-telemetry.sh
  modified:
    - patches/kyber/0008-txproto-host-telemetry.patch
    - README.md

key-decisions:
  - "Queue gauges come only from one safely referenced effective bounded owner; ambiguity is reported as unavailable instead of 0/0."
  - "Native callback rejection and Rust channel rejection use one atomic active-writer/generation word so one bounded snapshot can never publish a partial cumulative loss tuple."
  - "Rust loss bounds use the minimum first sequence and maximum last sequence across the accumulated epoch, independent of callback completion order."
  - "Legacy stage keys and the FnMut callback API remain source-compatible beside the additive result-aware callback."
  - "The deployed-service probe is explicit opt-in so normal verification cannot disrupt an active desktop session."

patterns-established:
  - "Bounded hot path: fixed-capacity scalar batches, nonblocking callbacks, and single-attempt coherent cumulative-loss snapshots."
  - "Truthful ownership: report payload size and queue state only where the owning object has the exact fact."
  - "Nested-source discipline: replay changes from parent patches without nested commits or gitlink movement."

requirements-completed: []

coverage:
  - id: D1
    description: Bounded native FIFO and callback-loss primitives with executable race and boundary tests
    verification:
      - kind: unit
        ref: scripts/verify-host-telemetry.sh --tests (Meson host_metrics)
        status: pass
    human_judgment: false
  - id: D2
    description: Real NvFBC/NVENC/packet-sink payload, queue, drop, and loss facts forwarded through Rust
    verification:
      - kind: integration
        ref: scripts/verify-host-telemetry.sh --tests
        status: pass
      - kind: other
        ref: scripts/verify-host-telemetry.sh --source
        status: pass
    human_judgment: false
  - id: D3
    description: Exact-pin parent patch series replays without nested commits or gitlink drift
    verification:
      - kind: integration
        ref: scripts/verify-host-telemetry.sh --replay
        status: pass
    human_judgment: false
  - id: D4
    description: Explicit bounded live probe validates deployed NvFBC, NVENC, and telemetry evidence while restoring service state
    verification:
      - kind: manual_procedural
        ref: scripts/verify-host-telemetry.sh --live
        status: unknown
    human_judgment: true
    rationale: The host service was already carrying a working session, so the restart-based opt-in probe was deliberately not run.

duration: 1h24min
completed: 2026-07-30
status: complete
---

# Quick Plan 260730-swz: Linux Host Loss-Aware Native Telemetry Summary

**Bounded C/Rust telemetry now reports real host timestamps, encoded payload sizes, owning-queue state, typed drops, and coherent loss tuples without nested commits or cross-machine latency claims.**

## Performance

- **Duration:** 1h 24 min
- **Started:** 2026-07-30T19:30:23Z
- **Completed:** 2026-07-30T20:54:40Z
- **Tasks:** 3 plus 1 verifier-directed gap closure
- **Files modified:** 4 parent artifacts representing 12 permitted nested source/test/build paths

## Accomplishments

- Corrected finite FIFO boundary and blocking semantics, added safe effective-owner queries, and covered boundary, mirror, race, helper, and callback-loss behavior in the native `host_metrics` test.
- Emitted truthful NvFBC/NVENC/packet-sink timing, real encoded payload bytes, exact post-push queue gauges, typed drops, and cumulative callback/forwarder loss through a bounded thread-safe Rust bridge.
- Closed the verification-discovered publication race with a nonblocking active-writer/generation protocol, deterministic sequence min/max, forced interleavings, and concurrent exact-total stress.
- Froze the nested work into exact-path parent patches and added a verifier for static contracts, focused native/Rust builds, exact-pin replay, and an explicit state-restoring live probe.

## Task Commits

Each task was committed atomically in the parent repository:

1. **Task 1: Enforce and test bounded FIFO and loss-aware metric primitives** - `499bb2a` (feat)
2. **Task 2: Emit real host ownership-point facts and propagate rejection** - `0e7542d` (feat)
3. **Task 3: Freeze reproducible patches and a clean replay verifier** - `09caad9` (chore)
4. **Verification gap closure: Publish coherent telemetry-loss snapshots** - `fd3a11f` (fix)

No nested txproto, Kymedia, Kysdk, or Kyber Desktop commit was created, and no gitlink moved.

## Files Created/Modified

- `patches/kyber/0008-txproto-host-telemetry.patch` - Native bounded FIFO, metric helpers, producer ownership-point instrumentation, and `host_metrics` tests.
- `patches/kyber/0009-kymedia-host-telemetry-forwarding.patch` - Thread-safe result callback, validation, and bounded loss-aware KyCom forwarding.
- `scripts/verify-host-telemetry.sh` - Source, test, replay, and explicit live verification modes.
- `README.md` - Exact application roots, ordering, semantics, and verifier usage.

## Verification

| Gate | Result | Evidence |
| --- | --- | --- |
| Source contracts | PASS | Exact pins/path allowlists, legacy/additive ABI, schema keys, FIFO checks, and no payload/secret logging |
| Native tests/build | PASS | `host_metrics`, forced partial publication, concurrent exact totals, full isolated txproto build, and 50/50 direct stress repetitions |
| Production native ownership sites | PASS | NvFBC, encoder, and packet-sink objects compiled with production Meson flags |
| Rust tests/format | PASS | 3 `txproto-rs` and 8 `kyavservice` tests, forced interleaving/min-max checks, workspace format, and 25/25 concurrent stress repetitions |
| Changed-path warnings | PASS | Strict warning gate after allowing only the confirmed unrelated pre-existing Clippy lint |
| Linux server link | PASS | `kyavserver` linked against the changed uninstalled txproto build |
| Exact-pin replay | PASS | `0008` then `0009`, exact allowlists, unchanged nested HEADs/gitlinks |
| Live deployed-host probe | HUMAN_NEEDED | Not run because it intentionally restarts the active host service |

The user service remained `active/running` at PID `3206561`, and its configuration SHA-256 remained `3ad53c4455b886077be87618457869c3d5017189f4bc43bf114d240457ffd12e`. Verification did not stop, restart, or modify it.

## Decisions Made

- Native and Rust loss writers increment one atomic active-writer count before touching tuple fields and atomically decrement it while advancing a completed generation afterward. Readers make one attempt and emit only when the publication word is unchanged with zero active writers.
- The low 16 publication bits cover the fixed producer-thread set; the upper 48-bit generation cannot wrap within a practical session. No mutex, heap allocation, sleep, delivery retry, or reader retry was added to the media path.
- The native callback retains a fixed 32-entry stack batch, while Rust initializes the first affected sequence to `u64::MAX` and uses atomic minimum/maximum operations so completion order cannot corrupt the bounds.
- A pass-through FIFO reports a gauge only through one safely held bounded mirror owner. Zero, multiple, unbounded, or changed owners return a typed unavailable reason.
- The Rust result callback owns immutable `Send + Sync` userdata, validates a whole batch before dispatch, and retains callback boxes until after native teardown.
- Kymedia reports cumulative rejected batch/entry totals and first/last local sequence on recovery; terminal unflushed totals remain explicit shutdown diagnostics.
- Host-local facts are not labeled as wire bitrate, cross-machine latency, decode time, or scanout time.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Made related telemetry-loss fields publish as one coherent snapshot**

- **Found during:** Post-plan verification
- **Issue:** Separate relaxed atomics allowed a concurrent reader to observe mixed generations, and Rust recorded the first completing sequence instead of the minimum affected sequence.
- **Fix:** Added a lock-free active-writer/generation publication word in C and Rust, a bounded single-attempt snapshot, atomic sequence min/max, and generation-based reported-loss tracking.
- **Files modified:** `patches/kyber/0008-txproto-host-telemetry.patch`, `patches/kyber/0009-kymedia-host-telemetry-forwarding.patch`
- **Verification:** Forced partial-update tests, concurrent exact-total/min-max tests, 50 native and 25 Rust stress repetitions, full `--tests`, `--source`, and `--replay`
- **Committed in:** `fd3a11f`

---

**Total deviations:** 1 auto-fixed bug
**Impact on plan:** The fix restores the original truthful-loss requirement without expanding paths, transport, client scope, or runtime-service activity.

## Issues Encountered

- Post-plan verification correctly found that individually atomic fields did not form a coherent tuple. The gap is closed by `fd3a11f`; no partial nonzero loss tuple can now pass a snapshot, and sequence bounds are deterministic under reordered completion.
- The exact all-target Clippy command is blocked by the pre-existing unrelated `kyavservice/src/lib.rs:361` `let_underscore_future` test lint. The verifier proves no changed Rust path appears in that failure and reruns with only that lint allowed; the scoped gate passes.
- The isolated rootfs pkg-config view needed the pinned `libavdevice` and Lua aliases plus system shared zlib metadata. The verifier now constructs that view deterministically without changing dependencies.
- Rust ThreadSanitizer was unavailable because only stable and pinned Rust 1.89.0 toolchains are installed. The concurrent callback test still passed normally; the verifier reports `THREAD_SANITIZER=UNAVAILABLE_NO_NIGHTLY`.
- Existing FFmpeg deprecation and NvFBC typedef warnings appeared while production objects compiled; no new changed-path warning failed the gate.

## Known Stubs

None. Added code and patch artifacts contain no TODO/FIXME, skipped test, placeholder data source, or goal-blocking hardcoded empty value.

## Authentication Gates

None.

## User Setup Required

None. The optional deployed-host check is run explicitly with `scripts/verify-host-telemetry.sh --live` only when restarting the current host session is acceptable.

## Next Phase Readiness

- Native and Rust telemetry implementation, coherent loss publication, focused/stress tests, static contracts, and exact-pin replay are ready for the host-readiness and 4K60 qualification work.
- The explicit `--live` probe remains a human-timed operational check because it restarts the active host service.
- Client clock correlation, decode/presentation timing, scanout, GUI, and release work remain deliberately outside this slice.

## Self-Check: PASSED

All four parent artifacts and this summary exist, task commits `499bb2a`, `0e7542d`, `09caad9`, and `fd3a11f` resolve, and the host service/config state still matches the pre-verification snapshot.

---
*Quick plan: 260730-swz*
*Completed: 2026-07-30*
