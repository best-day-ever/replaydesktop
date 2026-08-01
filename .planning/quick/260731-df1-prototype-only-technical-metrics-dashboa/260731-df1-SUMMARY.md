---
quick_id: 260731-df1
phase: quick
plan: 260731-df1
subsystem: ui
tags: [appkit, swift, jsonl, telemetry, macos]

requires:
  - quick: 260730-j91
    provides: AppKit launcher, raw telemetry tail, and deterministic arm64 packaging
provides:
  - Bounded in-memory reduction of existing Kyber raw_metric JSONL events
  - Honest Host, Network, and Client cards with literal Unknown fallbacks
  - Clean v0.3.0-spike.4 arm64 package bound to the launcher commit
affects: [macos-launcher, prototype-telemetry, macos-packaging]

tech-stack:
  added: []
  patterns:
    - Bounded same-frame JSONL correlation without changing the telemetry producer
    - Raw transcript fan-out to presentation and reduction while retaining original bytes

key-files:
  created:
    - .planning/quick/260731-df1-prototype-only-technical-metrics-dashboa/260731-df1-SUMMARY.md
  modified:
    - prototype/macos/ReplayDesktopLauncher.swift

key-decisions:
  - "Treat only existing log_type=raw_metric records as evidence; malformed, unsupported, missing, sentinel, and negative values never become measurements."
  - "Correlate video stages only by source_id and pts, and derive only nonnegative host-side or client-side same-clock deltas."
  - "Reset cards and reducer state at child start and termination while leaving Clear scoped to the raw transcript."

patterns-established:
  - "Unknown-first telemetry UI: every card row renders the literal Unknown until its exact source evidence arrives."
  - "Bounded live correlation: retain at most 256 recently observed frame keys and no persisted dashboard state."

requirements-completed: []

coverage:
  - id: D1
    description: Host, Network, and Client cards reduce only existing metrics.json raw events while retaining the raw transcript
    verification:
      - kind: unit
        ref: "ReplayDesktopLauncher --self-test metrics JSONL fixtures"
        status: pass
      - kind: integration
        ref: "xcrun swiftc arm64-apple-macos15.0 compile"
        status: pass
    human_judgment: true
    rationale: "A live card-update and visual inspection could not run because the Mac automation process lacks Accessibility permission."
  - id: D2
    description: Clean v0.3.0-spike.4 Apple Silicon package passes the existing bundle and archive gates
    verification:
      - kind: integration
        ref: "scripts/package-macos-gui.sh and scripts/verify-gui-spike.sh"
        status: pass
    human_judgment: false
  - id: D3
    description: One live GUI session connects to the Linux host and shows only evidenced card values
    verification:
      - kind: manual_procedural
        ref: "Mac15,13 to 100.85.187.73 live GUI smoke"
        status: unknown
    human_judgment: true
    rationale: "AXIsProcessTrusted() returned false before launch, so the agent could not enter the host, click Connect, or inspect cards."

duration: 14m 22s
completed: 2026-07-31
status: partial
---

# Quick Task 260731-df1: Prototype Technical Metrics Cards Summary

**An Unknown-first AppKit dashboard now reduces existing Kyber video and network JSONL into bounded current Host, Network, and Client cards, with a clean arm64 package ready for the pending live GUI smoke.**

## Performance

- **Duration:** 14m 22s
- **Started:** 2026-07-31T07:47:16Z
- **Completed:** 2026-07-31T08:01:38Z
- **Tasks:** 1 complete; 1 package-complete with live smoke pending
- **Source files modified:** 1

## Accomplishments

- Added a 256-frame in-memory JSONL reducer that accepts only current `raw_metric` video, network-local, network-remote, and network-ping records.
- Added compact Host, Network, and Client AppKit cards while preserving the existing 0.5-second tails, raw paths, transcript behavior, child lifecycle, and Clear behavior.
- Proved Unknown/reset behavior, chunk-split lines, out-of-order same-frame events, valid host/client deltas, `-1` sentinels, malformed input, skipped outcomes, and negative-delta rejection in the launcher self-test.
- Built and verified the clean `v0.3.0-spike.4` arm64 app and archive from exact commit `eeeb5c652578d59ade2c2159702825c0865a1872`.

## Task Commit

1. **Task 1: Parse existing JSONL events into three live cards** — `eeeb5c6` (`feat`)
2. **Task 2: Package clean arm64 app and run one live smoke** — package complete; no source commit; live GUI smoke pending

## Files Created/Modified

- `prototype/macos/ReplayDesktopLauncher.swift` — bounded reducer, three cards, child lifecycle resets, metrics tail fan-out, and fixture assertions.
- `.planning/quick/260731-df1-prototype-only-technical-metrics-dashboa/260731-df1-SUMMARY.md` — execution evidence and pending live-smoke blocker.

## Package Artifacts

- App: `/Users/finn/Developer/replaydesktop-metrics-v0.3.0-spike.4.FxyYKF/output/ReplayDesktop.app`
- Archive: `/Users/finn/Developer/replaydesktop-metrics-v0.3.0-spike.4.FxyYKF/output/ReplayDesktop-arm64-clipboard-spike.zip`
- Archive SHA-256: `b4d5a6feb6cdd11c9caf5088821ea14266263eb01ae48a36d0c96f7efbee6d07`
- Archive size: 40,156,805 bytes
- Source base: `4e4a5d7542c2d8b7057c85a2198a87b820357bfe`
- Source commit: `eeeb5c652578d59ade2c2159702825c0865a1872`
- Source-boundary SHA-256: `7d25f3df52194391095aa2801535b0b14b468d835acd893416314be3ee3b66bf`

## Verification Evidence

- Exact scratch compile on Apple Silicon:
  `xcrun --sdk macosx swiftc -parse-as-library -target arm64-apple-macos15.0 ReplayDesktopLauncher.swift -framework AppKit`.
- Launcher self-test passed all 192 existing argument combinations and the new metrics JSONL fixtures.
- `scripts/package-macos-gui.sh` completed and its `scripts/verify-gui-spike.sh` invocation passed.
- The verifier inspected 312 Mach-O files; both launcher and `kyclient` are thin arm64.
- `LSMinimumSystemVersion` is `15.0`.
- `codesign --verify --deep --strict ReplayDesktop.app` passed.
- Clean package checkout remained at exact commit `eeeb5c652578d59ade2c2159702825c0865a1872` with no worktree changes.

## Decisions Made

- A missing or `-1` network field replaces that row with `Unknown`; it does not retain a stale value or coerce the sentinel to a measurement.
- Negative or overflowing stage deltas are rejected. A duration is updated only after both exact same-frame endpoints provide a nonnegative same-clock delta.
- A newly observed `displayed` or `skipped` terminal event updates the latest-frame outcome; unsupported event types do nothing.

## Deviations from Plan

- The package was built with the installed Xcode 26.2 (`17C52`), Swift 6.2.3, and SDK 26.2 rather than the project stack's preferred Xcode 26.6 release lane. All existing package gates passed, but this is not claimed as Xcode 26.6 qualification.
- The planned live GUI smoke was not launched. This avoided consuming the one allowed attempt when the required UI interaction could not be performed.

## Live Smoke

**Result: blocked before launch; zero smoke launches were performed.**

- Mac: `Mac15,13`, arm64, macOS 15.7.7.
- Linux host: `100.85.187.73`; ping passed and `kycontroller` was listening on port 8080.
- Blocker: the macOS authorization probe `AXIsProcessTrusted()` returned `false`. System Events could not be used to enter the host, click Connect, disconnect, or inspect card rows.
- Known versus Unknown card rows: not observed. No runtime metric claims are made.
- Remaining action: grant the automation runner Accessibility permission, then perform exactly one live GUI connection and record raw-scroll continuity plus each known and Unknown card row.

## Known Stubs

None. Literal `Unknown` values are the required evidence state, not placeholders.

## Issues Encountered

- The Linux workspace has no Swift/macOS SDK. The exact compile, self-test, and package gates therefore ran on the established Apple Silicon builder.
- UI automation authorization was unavailable on the builder. No alternate launcher mode, replay path, native helper, or telemetry architecture was added.

## Self-Check: PASSED

- Source commit `eeeb5c6` exists and contains only `prototype/macos/ReplayDesktopLauncher.swift`.
- The remote app and archive exist at the paths above.
- Package source, source-boundary digest, architecture, minimum OS, signature, self-test, and archive SHA-256 claims were rechecked after packaging.
- Unrelated parent-checkout, nested Kyber, verifier, patch, README, and `log/` changes were not staged or modified.

---
*Quick task: 260731-df1*
*Completed source/package work: 2026-07-31*
