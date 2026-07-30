---
phase: quick-260730-w4b
verified: 2026-07-30T23:12:37Z
status: human_needed
score: 4/9 must-haves verified
behavior_unverified: 5
overrides_applied: 0
behavior_unverified_items:
  - truth: "The selected VLC decoder module is emitted only after module selection succeeds and is copied into a bounded record before callback return."
    test: "Run real H.264, HEVC, and AV1 sessions with the additive callback enabled and correlate module-load success with decoder-module events."
    expected: "Exactly one bounded non-empty module name is emitted after each successful video decoder selection, and none is emitted for a failed selection."
    why_human: "The production callsite is correctly ordered and compiled, but no registered test invokes decoder_LoadModule and observes its callback event."
  - truth: "VideoToolbox hardware use comes from the live post-create session property, separately from enable/require requests, with typed Unknown on query failure."
    test: "Create real VideoToolbox sessions for supported and forced/unavailable hardware cases and compare the emitted session record with an independent VTSessionCopyProperty query."
    expected: "hardware_state reflects UsingHardwareAcceleratedVideoDecoder, policy_flags only reflect enable/require requests, and unreadable/non-boolean properties remain Unknown."
    why_human: "The Apple smoke compiles the real plugin and tests the pure normalization seam, but does not create a decode session or execute the production property query."
  - truth: "Actual decoded CVPixelBuffer fourcc and IOSurface presence are observed from real output buffers and remain distinct from compressed chroma."
    test: "Decode real buffers across an output-format or IOSurface-state change and compare callback records with the CVPixelBuffer values at the decoder output callback."
    expected: "decoded_fourcc and iosurface_state match the actual buffer, emit on first observation/change, and never overwrite the compressed-stream chroma fact."
    why_human: "The Darwin seam creates a real IOSurface-backed CVPixelBuffer, but it does not invoke the production DecoderCallback event path."
  - truth: "Every correlated VideoToolbox callback produces exactly one truthful terminal outcome, with no external callback under the decoder mutex."
    test: "Fault-inject each VideoToolbox terminal branch and use a re-entrant callback while counting outcomes per correlated sourceFrameRefCon."
    expected: "Exactly one terminal frame event appears per correlated callback, with matching status/flags/PTS, and re-entry does not deadlock."
    why_human: "Pure classifiers cover every reason and source inspection places publication after unlock, but no behavioral test invokes the real callback under forced interleavings."
  - truth: "Renderer early returns cannot emit enqueue success; backpressure is not a drop; legacy displayed is not enqueue; physical presentation, scanout, and layer depth remain Unknown."
    test: "Fault-inject layer-absent, conversion, format-description, sample-buffer, and backpressure paths in the real sample-buffer renderer, then exercise a successful enqueue."
    expected: "Early returns emit only their typed failure, backpressure may precede a later enqueue without claiming a drop, enqueue success appears only after enqueueSampleBuffer, legacy displayed remains separate, and unavailable facts stay Unknown."
    why_human: "The classifier is tested and the production plugin compiles, but no test drives RenderPicture through the real early-return/enqueue branches."
human_verification:
  - test: "Observe real decoder-module event ordering for H.264, HEVC, and AV1."
    expected: "A bounded module name appears only after successful module selection."
    why_human: "No behavioral test invokes the production decoder selection callsite."
  - test: "Compare a real VideoToolbox session event with the live hardware-use property."
    expected: "Authoritative hardware state, request flags, and Unknown behavior agree with the session."
    why_human: "The smoke is compile/link plus seam testing, not a real decode session."
  - test: "Compare decoded-surface events with real CVPixelBuffers, including a surface change."
    expected: "Fourcc/IOSurface facts match the buffers and remain separate from compressed chroma."
    why_human: "The production output callback is not executed by the registered test."
  - test: "Fault-inject correlated VideoToolbox terminal branches with a re-entrant callback."
    expected: "One truthful outcome per correlation and no decoder-mutex deadlock."
    why_human: "The registered test exercises classifiers, not DecoderCallback."
  - test: "Fault-inject renderer early returns, backpressure, and successful enqueue."
    expected: "Only successful enqueue emits renderer_sample_enqueued; no drop/presentation/depth is invented."
    why_human: "The registered test exercises classifiers, not RenderPicture."
---

# Quick 260730-w4b: Truthful macOS Native Decoder Telemetry Verification

**Task Goal:** Expose truthful macOS native decoder telemetry: actual decoder
module, VideoToolbox hardware-use property, codec/chroma and decoded pixel
format, IOSurface backing, typed decode/render drops, and honest
presentation/queue unknowns.

**Verified:** 2026-07-30T23:12:37Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Verdict

No observable implementation gap was found. All source, registered-test,
stress, exact-pin replay, commit-boundary, and non-deploying Apple smoke gates
passed independently.

The phase is not marked `passed` because five truths depend on runtime decoder
or renderer behavior that the current tests do not exercise. The Mac lane
compiles the real plugins and runs the Darwin seam test; it does not start a
real VideoToolbox decode/render session. Those truths remain
`PRESENT_BEHAVIOR_UNVERIFIED` rather than being inferred from symbol presence.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Legacy scalar callback/keys/meanings remain ABI-compatible; video-path facts use an additive bounded v1 record. | ✓ VERIFIED | The original `libvlc_metrics_entry` and callback signature are byte-for-byte unchanged at the start of `include/vlc/libvlc_metrics.h`; the new fixed record is appended at lines 16-136. `nm` finds each legacy/additive setter exactly once. Scalar producer files are unchanged except the additive Kymux config publication. |
| 2 | Events have producer sequence and coherent cumulative rejection provenance without allocation, payload logging, or callback retry. | ✓ VERIFIED | `src/misc/metrics.c:19-114,180-247` assigns atomic sequences, takes a one-attempt coherent snapshot, calls the external callback once, and records rejection. Forced in-progress, delayed min/max, and 4×500 concurrent rejection tests passed, including 50 direct stress repetitions. |
| 3 | Real H.264/HEVC/AV1 config parsing uses the production helper and malformed input becomes typed Unknown rather than inferred chroma. | ✓ VERIFIED | `modules/access/kymux.c:241-262,336` passes the received config block to `kymux_video_config_Inspect`; `kymux_video_config.c` uses VLC H.264/HEVC SPS and AV1 sequence-header helpers. Known and malformed fixtures pass. Meson and Ninja show the test and production Kymux plugin both link `libkymux_video_config.a` plus `hxxxhelper`. |
| 4 | Selected decoder name is emitted only after successful module selection and bounded before callback return. | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `src/input/decoder_helpers.c:91-104` gates on non-null `p_module`, obtains `module_get_object`, and copies through the 64-byte `SetText` helper. The production callsite compiles, but no test observes it during real module selection. |
| 5 | Hardware use is the live post-create VideoToolbox property, separate from enable/require requests, with typed Unknown. | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `decoder.c:1090-1120,1219-1238,1336-1340` queries `UsingHardwareAcceleratedVideoDecoder` only after successful session creation and records policy separately. Apple compilation and normalization seams pass; the real session query is not executed. |
| 6 | Decoded fourcc and IOSurface presence come from the real CVPixelBuffer and stay separate from compressed chroma. | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `decoder.c:1122-1152,2295-2296` reads both values from `imageBuffer`; no chroma field is assigned. The Darwin seam validates real CoreVideo/IOSurface APIs, but does not drive the production callback. |
| 7 | VideoToolbox branches produce one typed terminal outcome per correlated callback, outside the decoder mutex. | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | All classifier branches are unit-tested. `decoder.c:2235-2250,2387-2395` records correlation once, unlocks before publication, then emits one frame outcome. No test invokes the real callback under forced interleavings. |
| 8 | Renderer early returns cannot emit enqueue success; legacy displayed remains separate; backpressure does not invent a drop; unavailable presentation facts are Unknown. | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `VLCSampleBufferDisplay.m:860-959` publishes early failures before returning, publishes backpressure separately, and emits enqueue success only after `enqueueSampleBuffer`. Scalar `displayed` remains at `video_output.c:2071-2077`. Observation-limit flags cover physical presentation, scanout, and layer depth. The real renderer branches are not executed by tests. |
| 9 | Clean replay, registered native tests, Linux ABI tests, and non-deploying Apple smoke pass. | ✓ VERIFIED | Independent `--source`, `--tests`, `--replay`, and `--mac-smoke` runs all exited 0. The Apple manifest reports arm64, macOS 15.7.7, Xcode 26.2 (17C52), SDK 26.2, deployment target 15.0, ten native test runs, `APP_ACTION=NONE`, and `COMPATIBILITY_ONLY_XCODE_26_2`. |

**Score:** 4/9 truths behaviorally verified (5 present and wired, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `patches/kyber/0010-vlc-video-path-telemetry-abi.patch` | Additive ABI, publisher, production config parser, decoder-module event, tests | ✓ VERIFIED | 1,535 substantive lines; exact 16-path allowlist; applies from VLC `dd2db547...`; replay matches current source byte-for-byte. |
| `patches/kyber/0011-vlc-macos-decoder-telemetry.patch` | VideoToolbox, decoded-surface, frame, and renderer telemetry | ✓ VERIFIED | 680 substantive lines; exact four-path allowlist; applies after 0010 and real Apple targets compile. |
| `scripts/verify-macos-native-telemetry.sh` | Source/tests/replay/scratch-only Mac gates | ✓ VERIFIED | Shell syntax and every mode passed; remote path validation, mode-0700 scratch, app snapshots, and validated cleanup are wired. |
| `README.md` | Patch order and honest fact semantics | ✓ VERIFIED | Applies 0010 before 0011; distinguishes compressed chroma, decoded fourcc, enqueue, legacy displayed, unavailable presentation, and Xcode 26.2 compatibility evidence. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| Public callback setters | One player-owned publisher | Player lock + immutable callback slots + inherited `kyber-metrics` | ✓ VERIFIED | Reverse setter order succeeded; duplicate setters failed; eight concurrent setters produced exactly one winner per slot; active-playback installation returned `-EINVAL`; player release while active completed safely. Teardown frees metrics only after input resources are released. |
| Received Kymux config / selected module | Additive callback | Production parser helper / post-`module_need` bounded copy | ⚠️ WIRED, runtime module event unverified | Config parsing is behavior-tested; selected-module event ordering is source-proven but not exercised. |
| Live VT session / CVPixelBuffer / renderer branch | Additive callback | Production property query, surface preparation, pure classifiers | ⚠️ WIRED, runtime behavior unverified | Real plugins compile; pure seams pass; no real decode/render callback run exists. |
| Registered test | Production parser and classifiers | Meson target links libVLC/core, `kymux_video_config_lib`, `hxxxhelper_lib` | ✓ VERIFIED | Meson test passed; Ninja query and symbol inspection confirm shared production linkage. |

### Data-Flow Trace

| Fact | Source | Publication path | Status |
|---|---|---|---|
| Compressed codec/chroma/profile/depth | Received Kymux config block | `Demux` → `kymux_video_config_Inspect` → `PushVideoPath` | ✓ FLOWING |
| Decoder name | Successful `module_need[_var]` result | `module_get_object` → bounded `SetText` → callback | ⚠️ FLOWING, runtime event unverified |
| Hardware use | Real created VT session | `VTSessionCopyProperty(UsingHardware...)` → normalized truth/Unknown → callback | ⚠️ FLOWING, live query unverified |
| Decoded surface | Real decoder output `CVPixelBufferRef` | fourcc/IOSurface getters → change detector → callback | ⚠️ FLOWING, live output unverified |
| Frame terminal outcome | Real `DecoderCallback` state | one pure classification → unlock → callback | ⚠️ FLOWING, callsite behavior unverified |
| Renderer outcome | Real sample-buffer renderer branch | pure classification → callback; enqueue event after enqueue | ⚠️ FLOWING, callsite behavior unverified |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Shell syntax | `bash -n scripts/verify-macos-native-telemetry.sh` | exit 0 | ✓ PASS |
| Static source/pins/ABI/log safety | `scripts/verify-macos-native-telemetry.sh --source` | `SOURCE=PASS` | ✓ PASS |
| Registered test and stress | `scripts/verify-macos-native-telemetry.sh --tests` | Meson 1/1 pass, 50 stress runs, symbol/parser gates pass | ✓ PASS |
| Exact-pin replay | `scripts/verify-macos-native-telemetry.sh --replay` | ordered/reverse replay and byte comparison pass | ✓ PASS |
| Non-deploying Apple smoke | `scripts/verify-macos-native-telemetry.sh --mac-smoke finn@100.119.34.79` | `MAC_SMOKE=PASS`, app action none, compatibility-only Xcode 26.2 | ✓ PASS |
| Reverse setter order and duplicates | Read-only ctypes call against built libVLC | video-first=0, scalar-second=0, both duplicates rejected | ✓ PASS |
| Concurrent setter replacement | Eight concurrent ctypes calls per setter | exactly one success and seven failures per slot | ✓ PASS |
| Active-playback replacement | Mock demux with dummy audio/video outputs | playing state 3; unused scalar install rejected with `-22` | ✓ PASS |

## Probe Execution

| Probe | Result | Status |
|---|---|---|
| `--source` | Exact pins, ABI, wiring, allowlists, and log-safety assertions passed | PASS |
| `--tests` | Registered production-linked native test and stress passed | PASS |
| `--replay` | Both patches replay and reverse cleanly at the exact VLC pin | PASS |
| `--mac-smoke` | Real Apple production plugins built in scratch; app unchanged; scratch removed | PASS |

## Requirements Coverage

No project requirement IDs were declared for this quick task
(`requirements-completed: []`). All nine PLAN frontmatter truths and all four
key links were evaluated directly.

## Commit and State Boundaries

- The chain is exactly
  `23fa1c31 → 7f67aa2 → 029be40 → 8ba945d`.
- The combined range changes exactly four authorized parent paths:
  `README.md`, patches 0010/0011, and the verifier.
- `7f67aa2` contains only patch 0010; `029be40` only patch 0011;
  `8ba945d` only README plus verifier.
- Parent Kyber gitlink remains `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`;
  recursive nested heads match the documented pins, including VLC
  `dd2db54794384591684c1c86ce70eb64eb9eab15`.
- The Phase 10 base commit is preserved and no `.planning`, `log/`, patch 0003,
  service/config, or application artifact entered the task commits.
- The Git index was empty before this report was written. Existing unrelated
  untracked/dirty paths were preserved.
- The Mac smoke used only a validated `/tmp` scratch tree, reported
  `MANIFEST_APP_ACTION=NONE`, and left no matching scratch directory behind.
  It did not install, replace, launch, or restart ReplayDesktop.

## Anti-Patterns and Disconfirmation

| Finding | Severity | Assessment |
|---|---|---|
| TODO/FIXME/XXX/placeholder in added task hunks | None | No matches. Existing upstream markers in untouched portions of large VLC files predate the exact pin and are not task debt. |
| Payload, secret, hostname, or metric-content logging | None | Added patch lines contain no such logging; verifier gate passed. |
| Callback executed under VideoToolbox decoder mutex | None | Surface and terminal events are prepared under the lock but published only after line 2389 unlocks it. |
| Callback-loss writer implementation uses CAS loops | ℹ️ Info | The external callback is attempted exactly once and rejection is not retried. Accounting is lock-free but not formally wait-free because saturating min/max updates use compare-exchange loops. If “writers never wait” was intended as a strict wait-free complexity guarantee, that contract needs a maintainer decision. |
| Passing tests overstate production-callsite coverage | ⚠️ Warning | Classifier tests prove reason mapping, not actual `DecoderCallback` or `RenderPicture` event cardinality/ordering. This is why five truths remain behavior-unverified. |
| Error-path coverage | ⚠️ Warning | Real VT property failure/non-boolean results, correlated output failures, and renderer early-return branches are not executed by the test suite. |

The disconfirmation pass found no false parser linkage or patch-replay claim:
the production plugin and test truly share the same helper target. It did find
that the successful Apple smoke is a build/seam smoke, not end-to-end decoder
telemetry evidence.

## Human Verification Required

### 1. Decoder module event ordering

**Test:** Run real H.264, HEVC, and AV1 sessions and correlate module selection
with decoder-module callback records.

**Expected:** A bounded actual module name appears only after successful video
decoder selection; failed selection emits none.

**Why human:** No registered test invokes this production ordering path.

### 2. Live VideoToolbox hardware truth

**Test:** Compare emitted session state against the live
`UsingHardwareAcceleratedVideoDecoder` property for hardware, software, and
unavailable-property cases.

**Expected:** Hardware state follows only the live property; enable/require
remain policy flags; unreadable/non-boolean values are Unknown.

**Why human:** The smoke compiles but does not create a decode session.

### 3. Real decoded-surface facts

**Test:** Decode through first output and a surface-format/IOSurface change.

**Expected:** Fourcc and IOSurface state match the real buffers, emit on
first/change, and remain separate from compressed chroma.

**Why human:** The current seam test does not invoke the production output
callback.

### 4. Decoder terminal cardinality and re-entry

**Test:** Fault-inject every correlated VideoToolbox branch with a re-entrant
consumer callback.

**Expected:** Exactly one truthful terminal event per correlation and no
deadlock.

**Why human:** Pure classifiers and source ordering do not exercise the
runtime invariant.

### 5. Renderer early-return and enqueue semantics

**Test:** Fault-inject each renderer early return, backpressure, and successful
enqueue.

**Expected:** Early returns never emit enqueue success; backpressure is not a
drop and can be followed by enqueue; legacy displayed is never normalized to
enqueue; physical presentation, scanout, and layer depth remain Unknown.

**Why human:** `RenderPicture` is compiled but not behaviorally driven.

## Gaps Summary

There are no confirmed implementation gaps and no blocker. Five runtime truths
need production-callsite testing before the strict verifier can certify a 9/9
behavioral score. Xcode 26.2 evidence is correctly compatibility-only; Xcode
26.6 release qualification is intentionally outside this quick task.

---

_Verified: 2026-07-30T23:12:37Z_
_Verifier: gsd-verifier_
