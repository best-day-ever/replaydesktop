---
quick_id: 260730-swz
mode: quick-full
status: ready
date: 2026-07-30
depends_on:
  - 260730-j91
  - 260730-mqr
  - 260730-ruu-research
scope_budget:
  declared_source_test_build_paths: 12
  parent_patch_artifacts: 2
must_haves:
  truths:
    - Existing NvFBC `acquired`, NVENC `encoding`/`encoded`, and Kymux-local `sent` timestamps retain their current meanings and remain backward compatible.
    - Every successfully encoded/sent frame reports its real `AVPacket::size`; configured bitrate is never presented as measured payload throughput.
    - A configured FIFO limit is an enforced maximum, and every observed gauge contains the exact current depth and configured limit from the owning FIFO.
    - Capture and encoded-packet FIFO rejection emit a stable stage/reason code, PTS, timestamp, current depth, limit, and cumulative count.
    - A rejected native callback or full/closed Rust forwarding channel increments explicit telemetry-loss accounting; loss is never represented as zero or silently logged away.
    - Metrics callbacks remain bounded and nonblocking and never contain encoded payload bytes, credentials, clipboard contents, or other user data.
    - The changes replay from the exact pinned txproto and Kymedia commits through parent-owned patches; no nested commit or gitlink update is created.
  artifacts:
    - patches/kyber/0008-txproto-host-telemetry.patch
    - patches/kyber/0009-kymedia-host-telemetry-forwarding.patch
    - scripts/verify-host-telemetry.sh
  key_links:
    - txproto ownership sites emit bounded scalar batches through the existing `tx_metrics_cb`.
    - A new additive thread-safe txproto-rs callback returns acceptance to C, allowing native loss accounting to observe a full kyavservice channel while the legacy callback API remains source-compatible.
    - kyavservice forwards accepted batches over the existing KyCom metrics endpoint and exposes rejected-batch/entry totals on the next accepted batch.
    - The native test target exercises the same metrics and FIFO helpers used by NvFBC, NVENC, and packet-sink production sites.
---

# Linux host loss-aware native telemetry

<objective>
Make the working Linux sender report truthful host-side timing, payload, queue,
drop, and telemetry-loss facts without changing transport, adding client code,
or claiming cross-machine latency. Keep the hot path bounded and preserve all
legacy metric keys.
</objective>

## Task 1: Enforce and test bounded FIFO and loss-aware metric primitives

**Files**

- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/include/libtxproto/metrics.h`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/metrics.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/fifo_template.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/include/libtxproto/fifo_frame.h`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/include/libtxproto/fifo_packet.h`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/test/host_metrics.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/meson.build`

**Action**

- Keep `tx_metrics_entry { key, value }` and `tx_metrics_cb` ABI-compatible.
  Add stable numeric enums for host queue/drop stages and reasons plus bounded
  helpers which emit:

  - `pts`
  - the legacy stage timestamp
  - `payload_bytes` where applicable
  - `queue_stage`, `queue_depth`, and `queue_limit`
  - `dropped`, `drop_stage`, `drop_reason`, and `drop_total`
  - `telemetry_callback_lost_batches_total` and
    `telemetry_callback_lost_entries_total`

- Make `tx_metrics_push()` consume the callback's existing integer return. A
  non-zero callback result atomically increments lost-batch and lost-entry
  totals. On a later callback attempt, prepend the current cumulative totals
  using a fixed-size stack batch; do not allocate, block, retry, sleep, or log
  metric values on the producer thread. Use thread-safe primitives supported
  by the pinned Linux/Windows toolchains and keep concurrent increments
  monotonic.
- Correct the FIFO template so finite `max_queued = N` means at most `N`
  queued non-null entries. `fifo_is_full()` and `fifo_push()` must agree at
  the exact boundary. A blocking producer must wait in a `while` loop which
  rechecks capacity after every wake, including spurious wakes and races with
  other producers. Preserve `-1` as unlimited, `0` as pass-through/no queue,
  null/EOS propagation, mirror behavior, and configured blocking flags.
- Add frame/packet FIFO queries for the unique effective bounded mirror owner.
  A pass-through source FIFO may report a depth/limit only by holding a safe
  reference to exactly one bounded destination and reading that destination's
  accessors outside the source lock. Zero destinations, multiple destinations,
  an unbounded destination, or a changed/unlinked mirror return a typed
  unavailable result; they never become `0/0`. Do not recursively guess
  through an ambiguous graph.
- Add a Meson test executable `host_metrics` in the existing root
  `meson.build`. It links the built txproto dependency and uses public/internal
  headers intentionally. Test:

  - limit 1 accepts exactly one packet and rejects the next with `ENOBUFS`;
  - pop restores capacity and depth/limit accessors are exact;
  - a pass-through FIFO mirrored to one bounded FIFO reports that bounded
    owner's exact depth/limit, while zero/multiple/unbounded owners return the
    documented unavailable result;
  - unlimited, zero, null, and blocking-flag semantics do not regress;
  - with a limit of one, multiple concurrent blocking producers and deliberate
    condition broadcasts never let observed depth exceed one; each producer
    resumes only after a consumer pop creates capacity;
  - a rejecting callback produces monotonic lost-batch/entry totals on the
    next accepted batch;
  - accepted callbacks do not fabricate loss;
  - payload, queue, and typed-drop helpers emit the exact bounded key/value
    group and reject impossible negative byte/depth values.

**Verify**

From `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto`, with the
pinned Kymedia rootfs pkg-config/library environment:

```bash
meson setup build-host-metrics --buildtype=debugoptimized --wrap-mode=nodownload
meson compile -C build-host-metrics host_metrics
meson test -C build-host-metrics host_metrics --print-errorlogs
```

Also run `meson compile -C build-host-metrics` and
`git diff --check` in the txproto worktree.

**Done**

The FIFO bound and loss-aware metric helpers have executable native tests and
cannot silently overfill or discard a callback result.

## Task 2: Emit real host ownership-point facts and propagate rejection

**Files**

- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/iosys_nvfbc.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/encode.c`
- `upstream/kyber-desktop/kysdk/kymedia/subprojects/txproto/src/packet_sink.c`
- `upstream/kyber-desktop/kysdk/kymedia/txproto-rs/src/lib.rs`
- `upstream/kyber-desktop/kysdk/kymedia/kyavservice/src/metrics.rs`

**Action**

- Preserve exact existing timestamp semantics:

  | Raw key | Exact host meaning |
  |---|---|
  | `acquired` | NvFBC grab plus the existing CUDA device-to-device copy completed |
  | `encoding` | immediately before FFmpeg/NVENC frame submission |
  | `encoded` | FFmpeg returned one encoded packet |
  | `sent` | both local packet-sink socket writes completed |

- Around the NvFBC frame FIFO push, snapshot the frame PTS. After the mirrored
  push succeeds, query the unique effective bounded destination (the linked
  encoder `src_frames`, not NvFBC's pass-through `entry->frames`) and emit its
  exact depth/limit. On `ENOBUFS`, emit typed `capture_fifo_full` facts. If the
  owner query is unavailable, emit a bounded `queue_owner_unavailable` reason
  and no numeric gauge.
- At encoder output, emit `encoded_payload_bytes = out_pkt->size`. Check the
  mirrored push result, then query the unique effective bounded destination
  (the linked packet-sink `src_packets`, not encoder's pass-through
  `dst_packets`) and emit its post-push depth/limit. Emit typed
  `encoded_packet_fifo_full` on `ENOBUFS`; owner ambiguity produces no gauge.
  Keep other errors terminal as before.
- After the packet sink writes the header and payload successfully, emit
  `sent_payload_bytes = in_pkt->size` beside legacy `sent`, plus the source
  packet FIFO depth/limit. This is encoded media payload only—not QUIC/TLS/IP
  wire bitrate.
- Preserve the existing `set_metrics_cb(FnMut)` API and behavior for source
  compatibility, but stop using it in kyavservice. Add an additive
  `set_metrics_result_cb` whose callback is `Fn(&[Metrics]) -> bool + Send +
  Sync + 'static`. Its userdata is immutable/thread-safe, the trampoline may
  be called concurrently by capture/encode/sink threads, and it returns C
  success or rejection without a shared mutable reference. Avoid panics on
  null pointers, invalid UTF-8 keys, oversized batches, or impossible values;
  reject the entire batch and let native loss accounting record it.
- Refactor kyavservice forwarding into a small, unit-testable bounded
  forwarder. `try_send` success returns accepted; full/closed returns rejected
  without blocking. Track rejected batch/entry totals, first/last affected
  local sequence, and attempt to piggyback those cumulative values on the next
  accepted batch. A still-unreported terminal loss remains explicitly
  unflushed in shutdown diagnostics; it is never reported as zero.
- Add Rust unit tests inline in the two changed Rust files for callback return
  propagation, invalid keys/counts, full and closed channels, multiple
  consecutive losses, recovery/piggyback, monotonic totals, and no-loss
  success. A concurrent trampoline test must invoke the same registered
  result callback from multiple threads and prove race-free exact acceptance/
  rejection totals under ThreadSanitizer where available.

**Verify**

```bash
cd upstream/kyber-desktop/kysdk/kymedia
cargo +1.89.0 test --locked -p txproto-rs -p kyavservice
cargo +1.89.0 fmt --all -- --check
cargo +1.89.0 clippy --locked -p txproto-rs -p kyavservice --all-targets -- -D warnings
```

Re-run the native `host_metrics` test and compile the Linux `kyavserver`
against the changed txproto. Scan produced logs to prove only keys, counts,
stages, sizes, and queue facts appear—never payload content.

**Done**

One real Linux host frame can produce backward-compatible timing plus actual
encoded/sent bytes, exact bounded queue gauges, typed rejection facts, and
observable telemetry delivery loss.

## Task 3: Freeze reproducible patches and a clean replay verifier

**Files**

- `patches/kyber/0008-txproto-host-telemetry.patch`
- `patches/kyber/0009-kymedia-host-telemetry-forwarding.patch`
- `scripts/verify-host-telemetry.sh`
- `README.md`

**Action**

- Generate `0008` from the exact pinned txproto commit and allow only the ten
  txproto production/test/build paths declared in Tasks 1–2.
- Generate `0009` from the exact pinned Kymedia commit and allow only
  `txproto-rs/src/lib.rs` and `kyavservice/src/metrics.rs`.
- Do not commit inside txproto, Kymedia, Kysdk, or Kyber Desktop and do not
  update any gitlink. Preserve all existing product patches byte-for-byte,
  including the user's untracked `0003-linux-zlib-pic.patch`.
- Add the two application roots and ordering to README:

  1. apply `0008` in `kysdk/kymedia/subprojects/txproto`;
  2. apply `0009` in `kysdk/kymedia`;
  3. retain existing top-level/Kynput/Kyctl product patch order.

- Create `scripts/verify-host-telemetry.sh` with:

  - `--source` static path/key/ABI and secret/content-log checks;
  - `--tests` exact Meson and Rust commands above;
  - `--replay` a temporary recursive checkout at the recorded pins, clean
    `git apply --check`, exact path allowlists, and nested-HEAD assertions;
  - `--live` a non-destructive host probe that records existing service/config
    state, runs a bounded local session only when explicitly selected, checks
    NvFBC/NVENC/metrics facts, and restores the original active state.

**Verify**

```bash
scripts/verify-host-telemetry.sh --source
scripts/verify-host-telemetry.sh --tests
scripts/verify-host-telemetry.sh --replay
```

Before every parent commit, stage only the four Task 3 artifacts or the exact
task-owned source artifact being committed and inspect `git diff --cached`.

**Done**

The host telemetry slice is independently replayable and tested from parent
artifacts without nested commits, gitlink drift, unrelated staging, or an
unrestored host service.

<verification>
- All legacy stage keys still compile and flow.
- Native FIFO/loss tests and focused Rust suites pass with warnings denied.
- Real payload size and queue/drop sites are tied to their owning objects.
- Callback/channel rejection produces explicit monotonic loss facts.
- Patch replay verifies exact pins and allowlists with no nested commit.
- No client, GUI, packaging, release, cross-clock, or scanout claim enters this slice.
</verification>

<success_criteria>
- The Linux sender exposes sufficient raw facts for a later client aggregator
  to compute loss-qualified host FPS, payload bitrate, queue/drop status, and
  same-clock capture/encode/local-write latency.
- Instrumentation remains bounded, nonblocking, backward compatible, and
  content-free.
- Automated native/Rust/replay gates pass and the live service is preserved.
</success_criteria>
