---
phase: quick
plan: 260730-w4b
subsystem: native-media-telemetry
tags: [libvlc, videotoolbox, corevideo, iosurface, kymux, macos]

requires:
  - phase: 260730-swz
    provides: bounded callback-loss provenance and native telemetry verification patterns
provides:
  - additive versioned libVLC video-path callback with coherent loss provenance
  - actual Kymux codec/chroma parsing and selected-decoder reporting
  - authoritative VideoToolbox session, decoded-surface, frame, and renderer outcomes
  - clean-replay verifier with Linux native gates and a non-deploying Apple Silicon smoke lane
affects: [native-evidence-aggregation, macos-gui-telemetry, 4k60-qualification]

tech-stack:
  added: []
  patterns:
    - fixed-capacity additive native telemetry ABI
    - pure outcome classifiers fed by framework-owning callsites
    - exact-pin patch replay with scratch-only Apple compatibility builds

key-files:
  created:
    - patches/kyber/0010-vlc-video-path-telemetry-abi.patch
    - patches/kyber/0011-vlc-macos-decoder-telemetry.patch
    - scripts/verify-macos-native-telemetry.sh
  modified:
    - README.md

key-decisions:
  - "Keep legacy scalar metrics ABI and meanings unchanged; expose video-path truth through a separate immutable callback."
  - "Treat requested hardware policy separately from the live VideoToolbox hardware-use property."
  - "Report sample-buffer enqueue separately from legacy displayed, and explicitly mark physical presentation facts unavailable."
  - "Classify Xcode 26.2 evidence as compatibility-only; Xcode 26.6 remains the release qualification lane."

patterns-established:
  - "Native truth boundary: compressed config, decoded surface, enqueue, and physical presentation are separate fact domains."
  - "Remote verification: copy exact inputs into mode-0700 scratch, never deploy, emit a manifest, and validate cleanup scope."

requirements-completed: []

coverage:
  - id: D1
    description: Additive loss-aware libVLC ABI reports parsed compressed-path and selected-decoder truth without changing scalar metrics.
    verification:
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --tests"
        status: pass
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --replay"
        status: pass
    human_judgment: false
  - id: D2
    description: VideoToolbox and sample-buffer callsites report authoritative hardware, surface, frame, enqueue, and observation-limit facts.
    verification:
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --mac-smoke finn@100.119.34.79"
        status: pass
      - kind: unit
        ref: "test/libvlc/video_path_metrics.c#VideoToolbox, renderer, CoreFoundation, and IOSurface seams"
        status: pass
    human_judgment: false
  - id: D3
    description: Exact pins, patch boundaries, native tests, clean replay, and scratch-only Apple builds are reproducibly verified and documented.
    verification:
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --source"
        status: pass
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --replay"
        status: pass
      - kind: integration
        ref: "scripts/verify-macos-native-telemetry.sh --mac-smoke finn@100.119.34.79"
        status: pass
    human_judgment: false

duration: 57min
completed: 2026-07-30
status: complete
---

# Quick 260730-w4b: Truthful macOS Native Decoder Telemetry Summary

**A bounded additive libVLC ABI now carries parsed Kymux, actual VideoToolbox, decoded-surface, and sample-buffer enqueue facts through clean exact-pin patches and a non-deploying Apple Silicon verification lane.**

## Performance

- **Duration:** 57 minutes
- **Started:** 2026-07-30T21:58:08Z
- **Completed:** 2026-07-30T22:55:17Z
- **Tasks:** 3
- **Parent files created/modified:** 4
- **Nested VLC paths replayed:** 18

## Accomplishments

- Preserved the scalar metrics ABI while adding one immutable, fixed-capacity, versioned video-path callback with producer sequence and coherent cumulative callback-loss provenance.
- Parsed the actual received H.264, HEVC, and AV1 configuration through production VLC helpers, and exposed the real selected decoder module without changing demux or selection behavior.
- Queried the live VideoToolbox hardware-use property, observed actual CVPixelBuffer fourcc/IOSurface state, and classified decoder plus renderer branches through production pure helpers.
- Kept legacy `displayed` semantics unchanged while adding a distinct successful-enqueue event and explicit Unknown facts for presentation, scanout, and layer queue depth.
- Added deterministic source, Linux test, clean replay, and scratch-only remote Mac modes; the Apple lane compiled real production plugins and ran the Darwin seam test ten times.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add the compatible event ABI and publish compressed-path truth** - `7f67aa2` (feat)
2. **Task 2: Publish real VideoToolbox and sample-buffer renderer outcomes** - `029be40` (feat)
3. **Task 3: Freeze reproducible verification and run non-deploying Mac smoke** - `8ba945d` (test)

Planning metadata remains uncommitted as required by the quick-task handoff.

## Files Created/Modified

- `patches/kyber/0010-vlc-video-path-telemetry-abi.patch` - Additive ABI, loss-aware publisher, production config parser, selected decoder event, and registered native tests.
- `patches/kyber/0011-vlc-macos-decoder-telemetry.patch` - Live VideoToolbox session/surface/frame facts and typed sample-buffer renderer outcomes.
- `scripts/verify-macos-native-telemetry.sh` - Exact-pin source, test, replay, and scratch-only Apple Silicon smoke gates.
- `README.md` - Patch order, fact-domain semantics, verification commands, and Xcode qualification boundary.

## Verification Evidence

- `bash -n scripts/verify-macos-native-telemetry.sh` — passed.
- `scripts/verify-macos-native-telemetry.sh --source` — passed exact pins, allowlists, ABI coexistence, event wiring, observation limits, and log safety.
- `scripts/verify-macos-native-telemetry.sh --tests` — passed the registered Linux test, 50 stress repetitions, production Kymux parser linkage, and built libVLC symbol checks.
- `scripts/verify-macos-native-telemetry.sh --replay` — passed ordered apply, byte equivalence, reverse apply, and clean exact-pin restoration.
- `scripts/verify-macos-native-telemetry.sh --mac-smoke finn@100.119.34.79` — passed on arm64 macOS 15.7.7 with Xcode 26.2 (17C52), SDK 26.2, deployment target 15.0, Meson 1.10.2, and Ninja 1.13.2.
- The remote lane built `libvideotoolbox_plugin.dylib`, `libsamplebufferdisplay_plugin.dylib`, patched libVLC/core, and ran the CoreFoundation/CoreVideo/IOSurface seam test ten times.
- The remote manifest recorded `MANIFEST_APP_ACTION=NONE` and `MANIFEST_QUALIFICATION=COMPATIBILITY_ONLY_XCODE_26_2`.

## Decisions Made

- Retained the existing scalar callback exactly and installed the additive callback in the same player-owned publisher lifetime.
- Used production H.264/HEVC/AV1 parsers rather than test-only parsers or requested codec labels.
- Preserved requested enable/require flags, but made only `VTSessionCopyProperty(...UsingHardware...)` authoritative for hardware-use state.
- Emitted decoded surface facts only on first observation or fourcc/IOSurface-state change.
- Published external decoder callbacks after releasing the decoder mutex to prevent callback re-entry deadlock.
- Preserved sample-buffer enqueue policy when observable backpressure occurs; backpressure and actual enqueue are separate events.
- Used the pre-generated exact-pin native Mac build in an isolated copy, with explicit SDK root and deployment target, rather than modifying the source checkout or installed app.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Completed native test/build integration details**
- **Found during:** Task 1
- **Issue:** Apple/AV1 include ordering, release-mode assertions, internal classifier visibility, and one copied HEVC fixture expectation blocked truthful production-linked tests.
- **Fix:** Included VLC ES declarations before the AV1 helper, enabled assertions for the test target, exported internal test helpers from libvlccore, and corrected the real HEVC fixture to profile 4 / 4:4:4.
- **Files modified:** Included in `0010-vlc-video-path-telemetry-abi.patch`.
- **Verification:** Registered test, production Kymux link, symbol inspection, and 50 stress repetitions passed.
- **Committed in:** `7f67aa2`

**2. [Rule 1 - Bug] Removed external callback publication from the decoder critical section**
- **Found during:** Task 2 final source review
- **Issue:** Publishing surface/frame events while holding the decoder mutex allowed consumer callback re-entry to deadlock the decode path.
- **Fix:** Prepared immutable event data under the mutex, then published surface and terminal outcome events after unlocking.
- **Files modified:** Included in `0011-vlc-macos-decoder-telemetry.patch`.
- **Verification:** Apple production plugin rebuild, Darwin test, Linux registered test, and clean replay passed.
- **Committed in:** `029be40`

**3. [Rule 1/3 - Verifier correctness] Fixed three evidence-lane assumptions**
- **Found during:** Task 3 source, test, and Mac smoke runs
- **Issue:** The verifier initially expected semicolon-terminated VLC symbols, used `rg -q` behind `pipefail` and misread `nm` SIGPIPE as a missing parser, and omitted the explicit SDK root from the manually linked Darwin seam test.
- **Fix:** Counted exact bare exports with `awk`, consumed complete `nm` streams, and passed the Xcode SDK root to compile and link commands.
- **Files modified:** `scripts/verify-macos-native-telemetry.sh`
- **Verification:** All four final verifier modes passed.
- **Committed in:** `8ba945d`

---

**Total deviations:** 3 auto-fixed (2 correctness, 1 blocking integration group)
**Impact on plan:** All fixes were necessary for deadlock safety or trustworthy build/test evidence; no product scope was added.

## Issues Encountered

- The remote Mac has Xcode 26.2 rather than the planned 26.6 release lane. Its successful result is recorded only as compatibility evidence.
- The copied native build emitted 84 existing upstream deprecation/unused warnings while rebuilding broad libvlccore dependencies; the changed Apple objects had no new API compile errors.
- `shellcheck` is not installed locally. The planned `bash -n` gate and every executable verifier mode passed.

## Known Stubs

None. Added/modified artifacts contain no TODO, FIXME, placeholder, skipped-test, or unwired empty-data stubs.

## Authentication Gates

None. Existing batch SSH authentication to the test Mac worked; no credentials or application trust state were changed.

## User Setup Required

None.

## Next Phase Readiness

- The next Rust/GUI evidence slice can deep-copy the bounded v1 records, aggregate them into JSONL, and expose honest client telemetry without reverse-engineering native state.
- A real streamed session should collect the new callback events end-to-end before making user-visible performance claims.
- Xcode 26.6 release qualification remains required; Xcode 26.2 evidence must not be promoted.
- No blocker remains for the planned native consumer work.

## Self-Check: PASSED

- All four parent deliverables exist, and the verifier is executable.
- Task commits `7f67aa2`, `029be40`, and `8ba945d` exist with their exact staged allowlists.
- The summary exists at the required quick-task path.

---
*Quick task: 260730-w4b*
*Completed: 2026-07-30*
