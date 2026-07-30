---
quick_id: 260730-mqr
phase: quick
plan: 260730-mqr
subsystem: clipboard
tags: [appkit, nspasteboard, x11, xfixes, kyber, kymux, macos-arm64, nvfbc, nvenc]

requires:
  - phase: quick-260730-j91
    provides: Pinned Kyber 0.27.0 Linux host, Apple Silicon macOS launcher, high-resolution input, deterministic packaging, and working Kymux media baseline
provides:
  - Native bidirectional Text and HTML clipboard synchronization between macOS and Linux X11
  - Independent negotiated copy/paste permissions with zero pasteboard work when disabled or fully refused
  - Strict 60 KiB combined bounds, immutable lazy snapshots, atomic remote commits, and loop/stale-generation suppression
  - Private v0.3.0-spike.1 Apple Silicon prerelease verified after a fresh GitHub download
affects: [macos-client, linux-host, clipboard, kymux-input, packaging, macos-host-port]

tech-stack:
  added: [objc2, objc2-app-kit, objc2-foundation]
  patterns:
    - Single named macOS clipboard worker owns every NSPasteboard operation
    - Requested state and independently negotiated directions gate the worker before pasteboard access
    - Local clipboard data is snapshotted immutably and served lazily over existing reliable Kymux input events
    - Remote representations are fetched sequentially and committed atomically because the current wire has no request ID
    - Parent-owned patches reproduce nested Kyber changes without nested commits

key-files:
  created:
    - patches/kyber/0005-kynput-macos-clipboard.patch
    - patches/kyber/0006-kyctl-clipboard-negotiation.patch
    - patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch
  modified:
    - README.md
    - prototype/macos/ReplayDesktopLauncher.swift
    - prototype/macos/engine-baseline.lock
    - scripts/package-macos-gui.sh
    - scripts/verify-gui-spike.sh

key-decisions:
  - "Map host allow_paste to Mac local-read/send and host allow_copy to Mac remote-write; never collapse the two directions."
  - "Default clipboard off in the GUI and create no pasteboard worker when the user disables it or both host directions are refused."
  - "Keep the existing ClipboardEvent wire for the prototype, serialize remote requests, and defer deterministic simultaneous-peer conflict resolution to a versioned request-ID update."
  - "Treat matched new-controller/new-service positional MessagePack IPC as the prototype boundary; mixed internal versions are unsupported and fail closed."
  - "Kyber has reusable macOS control/client scaffolding but no working macOS capture/encode/input host; macOS hosting is a platform port, not a configuration switch."

patterns-established:
  - "Privacy gate: permissions are applied before pipeline start, not merely filtered during packet routing."
  - "Clipboard integrity: immutable local snapshots, sequential remote fetches, atomic writes, local-wins races, self-write suppression, and stale-response draining."
  - "Capability honesty: GUI, logs, release notes, and tests distinguish requested, negotiated, disabled, refused, automated, and human-needed states."
  - "Release integrity: qualify the local archive first, publish second, then compare the downloaded asset byte-for-byte."

requirements-completed: []

coverage:
  - id: D1
    description: Native macOS Text and HTML clipboard pipeline is bounded, lazy, atomic, loop-safe, and performs zero operations for denied directions
    verification:
      - kind: unit
        ref: "Kynput clipboard core tests: 10 passed"
        status: pass
      - kind: integration
        ref: "macOS named-NSPasteboard integration tests: 11 passed; the general pasteboard was not used"
        status: pass
      - kind: other
        ref: "Pinned cargo fmt, task-scoped clippy, and stop/join tests"
        status: pass
    human_judgment: false
  - id: D2
    description: Linux bounds, independent host policy negotiation, and the opt-in GUI control are wired through the existing Kyber/Kymux input path
    verification:
      - kind: unit
        ref: "Linux Kynput library suite: 65 passed; focused malformed/oversize/wire suite: 10 passed"
        status: pass
      - kind: integration
        ref: "Kyctl negotiation tests and launcher self-test across 192 option combinations"
        status: pass
      - kind: integration
        ref: "Clean exact-pin replay of patches 0001, 0004, 0005, 0006, and 0007"
        status: pass
    human_judgment: false
  - id: D3
    description: The exact packaged app synchronizes both directions and obeys every negotiated or disabled policy without clipping, stale reconnect writes, or content logging
    verification:
      - kind: e2e
        ref: "Exact-archive Mac-to-Linux Text+HTML and Linux-to-Mac Text+HTML checks"
        status: pass
      - kind: e2e
        ref: "Mac-send-only, Mac-receive-only, both-refused, and client-disabled policy checks"
        status: pass
      - kind: e2e
        ref: "Oversize rejection, rapid changes, reconnect/no-stale, and ReplayDesktop product-log content scan"
        status: pass
    human_judgment: false
  - id: D4
    description: Apple Silicon macOS 15 clipboard package is provenance-bound, signed, published privately, and identical after re-download
    verification:
      - kind: integration
        ref: "312 thin arm64 Mach-O files, minimum macOS 15.0, deep codesign, raw CLI, source boundary, identity scan, and exact archive manifest"
        status: pass
      - kind: e2e
        ref: "GitHub v0.3.0-spike.1 fresh download byte comparison and SHA-256 verification"
        status: pass
    human_judgment: false
  - id: D5
    description: Physical cross-application clipboard behavior and subjective transfer latency are acceptable on the operator's Mac
    verification: []
    human_judgment: true
    rationale: Real application pasteboards, macOS privacy prompts, simultaneous human changes, and perceived sub-500 ms latency require the operator at the Mac

duration: 48m
completed: 2026-07-30
status: complete
---

# Quick Task 260730-mqr: Bidirectional macOS Clipboard Summary

**Privacy-gated Text and HTML clipboard synchronization over Kyber/Kymux, qualified in both directions and every permission mode, shipped as a byte-verified private Apple Silicon prerelease**

## Performance

- **Duration:** 48m
- **Started:** 2026-07-30T14:59:08Z
- **Completed:** 2026-07-30T15:46:34Z
- **Tasks:** 3
- **Parent-repository implementation files created/modified:** 8

## Accomplishments

- Added a native macOS `NSPasteboard` pipeline for plain text and HTML over
  Kyber's existing reliable Kymux input channel. Local values are captured as
  immutable snapshots, served lazily, and never replaced by a later unrelated
  pasteboard value.
- Added sequential remote representation fetching and one atomic pasteboard
  commit, plus self-write, same-content, stale-generation, reconnect, and
  local-wins suppression.
- Enforced a strict 60 KiB combined Text+HTML boundary on both platforms.
  Oversized or invalid X11 properties are rejected rather than clipped.
- Negotiated host copy and paste permissions independently. Clipboard remains
  opt-in and starts no pasteboard worker when disabled or when both host
  directions are refused.
- Replaced the unavailable GUI label with
  `Clipboard sync — Text + HTML, 60 KiB`, default off and independent from
  mouse/keyboard input.
- Rebuilt the Linux and Apple Silicon runtimes from exact pins, preserved the
  working NvFBC/NVENC/Kymux/VideoToolbox baseline, and left the Linux service
  active with its original configuration byte-for-byte.
- Published private prerelease
  [v0.3.0-spike.1](https://github.com/best-day-ever/replaydesktop/releases/tag/v0.3.0-spike.1)
  with `ReplayDesktop-arm64-clipboard-spike.zip`. The freshly downloaded asset
  is byte-identical to the qualified archive with SHA-256
  `87da42b2df9512fbce6c692496735d164e64ce0f9f7057fb1e0a782bce188265`.

## Task Commits

Each source task was committed atomically:

1. **Task 1: Native macOS clipboard pipeline** - `d9909b9` (`feat`)
2. **Task 2: Directional negotiation, Linux bounds, and GUI control** -
   `1504085` (`feat`)
3. **Task 3: Clean-build and package provenance lock** - `c1f48ad` (`chore`)

No nested Kyber repository commit was created. Patches 0005, 0006, and 0007
are the reproducible parent-owned source of the nested changes. The unrelated
local `patches/kyber/0003-linux-zlib-pic.patch` was not staged, committed,
replayed, or included in any qualification claim.

## Files Created/Modified

- `patches/kyber/0005-kynput-macos-clipboard.patch` - Native pasteboard
  adapter, worker state machine, strict wire bounds, Linux X11 hardening, and
  deterministic tests.
- `patches/kyber/0006-kyctl-clipboard-negotiation.patch` - Independent
  requested/negotiated copy and paste capabilities across kyctl.
- `patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch` -
  Top-level Kyber Desktop wiring that keeps clipboard independent from general
  input and preserves patch 0001 byte-for-byte.
- `prototype/macos/ReplayDesktopLauncher.swift` - Opt-in Text+HTML clipboard
  control, exact `--clipboard=true|false` argv mapping, and expanded
  self-test matrix.
- `prototype/macos/engine-baseline.lock` - Updated raw kyclient and executable
  payload provenance for the clean Rust 1.89 build.
- `scripts/package-macos-gui.sh` - Clipboard-spike archive naming and locked
  build/package inputs.
- `scripts/verify-gui-spike.sh` - Exact clipboard source boundary, Kyctl pin,
  archive, engine, and identity verification.
- `README.md` - Clipboard behavior, policy direction law, current release,
  limits, prototype wire boundary, and human-UAT status.

## Automated Evidence

### Deterministic clipboard behavior

- Linux Kynput library suite: 65 passed, 0 failed.
- Clipboard state-machine core: 10 passed, 0 failed.
- Focused malformed, truncated, strict-UTF-8, oversize, combined-limit, and
  sequential-wire suite: 10 passed, 0 failed.
- macOS integration: 11 passed using a uniquely named pasteboard; tests never
  touched `NSPasteboard.general`.
- Kyctl legacy/default and directional-negotiation tests passed.
- Pinned formatting and task-scoped clippy passed. The only allowed clippy
  exception was the known dummy-backend dead-code warning in the pinned tree.
- Launcher self-test passed 192 combinations and proved exactly one clipboard
  flag, default-off behavior, and independence from the Input toggle.

### Clean source and package boundary

- Source base:
  `4d8e39d2202fcdaa26abebeb37de7acf245741d0`.
- Qualified source commit:
  `c1f48adb28192bf5e3f25275bdb549fd16399520`.
- Deterministic source-boundary SHA-256:
  `ca9f7e4f5066d30f07d4f051d4db9a71aa458627d7b7a8b997589a32858b548d`.
- Patch 0001 remained byte-identical with SHA-256
  `bf5035d6636d00ccf57b999375a84608bdaa350b253e52735766252ac4a81c28`.
- Patches 0001 through 0007 replayed from exact pins in their documented
  repositories. Private/test identity material and nested build artifacts
  were absent.
- The authoritative macOS engine was rebuilt from a discarded target state
  with Cargo/Rust 1.89.0 and GNU Bison 3.8.2. Its raw SHA-256 is
  `4c09bb6fffb732cde74642fb72cfa46cbdf1291f4f75456cef621fe98295e5f4`,
  and its locked executable-payload SHA-256 is
  `b8bb23983c8da3d679319938a9a19c493300df7a8178eefd0d686d032b677310`.
- Built on Apple Silicon macOS 15.7.7 with Xcode 26.2 (`17C52`), SDK 26.2
  (`25C57`), Swift 6.2.3, and deployment target 15.0. The package does not
  claim testing on macOS 27.
- The package verifier inspected 312 Mach-O files; all are thin arm64, all
  declare a supported minimum OS, deep ad-hoc signature verification passed,
  and the exact archive manifest matched the qualified app.

### Exact-archive end-to-end behavior

The app was extracted from the exact qualified ZIP and connected to the live
Linux X11 host. The following all passed without printing clipboard contents:

- Mac-to-Linux plain text plus HTML.
- Linux-to-Mac plain text and HTML.
- Rapid consecutive changes in both directions; the final generation won.
- Oversized local and remote properties were rejected and never clipped or
  transferred.
- Disconnect/reconnect preserved pre-session baselines, produced no stale
  write, and transferred the first new post-reconnect generation.
- Host policy `copy=false, paste=true`: Mac local-read/send worked and remote
  writes were refused.
- Host policy `copy=true, paste=false`: Linux-to-Mac writes worked and Mac
  local-read/send was refused.
- Both host policies false: the client logged `macOS clipboard disabled`, no
  direction transferred, and no pasteboard worker started.
- Client `--clipboard=false` with both host policies available: neither
  direction transferred and the pipeline remained disabled.
- Synthetic clipboard markers were absent from ReplayDesktop product logs on
  both machines.

After testing, both test clipboards were cleared, the client was stopped, and
the original host config was restored to SHA-256
`3ad53c4455b886077be87618457869c3d5017189f4bc43bf114d240457ffd12e`.
`replaydesktop-kyber-spike.service`, TCP 8080, DP-0.3/DP-5, NvFBC, NVENC, and
the Kynput input path all passed the final readiness check.

### Publication

- `best-day-ever/replaydesktop` remains private.
- Existing releases were preserved.
- `v0.3.0-spike.1` is a prerelease targeting
  `codex/publish-kyber-spike` at source commit `c1f48ad`.
- The sole new asset is
  `ReplayDesktop-arm64-clipboard-spike.zip`.
- A fresh `gh release download` passed SHA-256, byte comparison, ZIP
  integrity, and complete entry-manifest comparison against the qualified
  local archive.

## Decisions Made

- Host `clipboard_allow_paste` authorizes Mac local clipboard reads and sends;
  host `clipboard_allow_copy` authorizes remote data to write the Mac
  pasteboard.
- The GUI checkbox expresses requested behavior only. Actual pasteboard access
  is the intersection of that request and the two negotiated host directions.
- The initial pasteboard generation is baselined without a content read or
  advertisement. Enabling a session never uploads pre-existing clipboard
  contents.
- Text and HTML are the only prototype formats. Image/file clipboard remains
  out of scope.
- Current remote requests remain serialized because `ClipboardEvent` responses
  have no request ID. A future versioned wire update is required for fully
  deterministic simultaneous-peer conflict resolution.
- Internal controller/service positional MessagePack layouts must match for
  this prototype. External start responses are backward-tolerant and legacy
  aggregate behavior fails closed.
- Ad-hoc signing and TLS verification bypass remain explicitly accepted only
  for this private LAN/Tailscale prototype.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Kept clipboard independent from input**
- **Found during:** Task 2
- **Issue:** The existing top-level pipeline treated clipboard as an aggregate
  input capability, which could suppress clipboard-only operation or enable it
  through the wrong gate.
- **Fix:** Added patch 0007 so clipboard and general input are negotiated and
  started independently without modifying immutable packaging patch 0001.
- **Files modified:** `patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch`
- **Verification:** Launcher clipboard-without-input matrix and live policy
  sessions passed.
- **Committed in:** `1504085`

**2. [Rule 1 - Bug] Made the legacy aggregate fail closed**
- **Found during:** Task 2
- **Issue:** Combining independent copy/paste directions with logical OR could
  authorize a legacy aggregate when only one direction was permitted.
- **Fix:** Legacy aggregate compatibility now requires both directions; new
  clients consume the separate fields.
- **Files modified:** `patches/kyber/0006-kyctl-clipboard-negotiation.patch`
- **Verification:** Kyctl legacy/default and directional tests passed.
- **Committed in:** `1504085`

**3. [Rule 3 - Blocking] Discarded compiler-drift artifacts**
- **Found during:** Task 3
- **Issue:** An early macOS build root contained Rust 1.94 artifacts and could
  not support the pinned Rust 1.89 provenance claim.
- **Fix:** Discarded every affected target/native/rootfs/app artifact and
  rebuilt the engine and dependencies from clean source with Rust/Cargo 1.89.
- **Files modified:** `prototype/macos/engine-baseline.lock`
- **Verification:** Raw-engine, payload-boundary, package, Mach-O, and archive
  provenance checks passed.
- **Committed in:** `c1f48ad`

**4. [Rule 3 - Blocking] Selected a modern Bison for the clean VLC build**
- **Found during:** Task 3
- **Issue:** Apple's Bison 2.3 could not parse the pinned VLC grammar during
  the from-scratch build.
- **Fix:** Rebuilt with Homebrew GNU Bison 3.8.2 explicitly first in `PATH`;
  no dependency pin changed.
- **Files modified:** None; build-environment correction only.
- **Verification:** Clean native dependency and final app builds completed.
- **Committed in:** Reflected in `c1f48ad` package provenance.

**5. [Rule 3 - Blocking] Recreated the live service around locked binaries**
- **Found during:** Task 3
- **Issue:** Replacing a running executable produced `ETXTBSY`.
- **Fix:** Stopped and recreated the transient user service around the rebuilt
  runtime, preserving and later re-verifying the exact host configuration.
- **Files modified:** None; live-runtime operation only.
- **Verification:** Repeated stream, policy, logout, port, process, display,
  NvFBC, NVENC, input, and final config-SHA checks passed.
- **Committed in:** Runtime-only; no source commit required.

---

**Total deviations:** 5 auto-fixed (1 Rule 1, 1 Rule 2, 3 Rule 3)
**Impact on plan:** Every deviation was required for correct gating or a clean,
reproducible build. No new product scope was added.

## Issues Encountered

- The first oversize end-to-end assertion expected the previous Linux
  selection to remain owned. The intended protocol behavior instead advertises
  no supported formats for an invalid local generation, so the host selection
  becomes empty. The corrected gate proves rejection and absence of clipping;
  the opposite direction proves an invalid remote property does not overwrite
  the Mac baseline.
- The available builder is macOS 15.7.7 with Xcode 26.2, not the recommended
  Xcode 26.6 lane. The complete package passed on that environment, but macOS
  27 beta compatibility remains unclaimed.
- The best-effort `.planning/WINDOWS.md` append for the human-only UAT item was
  skipped because that pre-existing ledger's frontmatter counts disagree with
  its entries. The UAT remains recorded in this summary and coverage block;
  the unrelated ledger was not rewritten.

## Known Stubs

None. UI placeholder strings and empty parser/runtime initializers found by the
stub scan are functional initialization states, not disconnected deliverables.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: clipboard-ingress-egress | `patches/kyber/0005-kynput-macos-clipboard.patch` | Adds a system-pasteboard/X11-selection trust boundary and carries bounded Text/HTML over the network. |
| threat_flag: capability-negotiation | `patches/kyber/0006-kyctl-clipboard-negotiation.patch` | Host copy/paste policy now authorizes two independent data directions before pipeline startup. |

## User Setup Required

None for the existing live host. Download the private prerelease, expand it,
move `ReplayDesktop.app` to Applications, launch it, enter the host, enable the
clipboard checkbox, and connect. The package is ad-hoc signed and not notarized
by explicit prototype approval.

## Human UAT Remaining

`human_needed`:

- Copy plain and rich text between real macOS and Linux applications in both
  directions.
- Observe whether macOS presents any pasteboard privacy prompt.
- Change both clipboards simultaneously and confirm the operator-preferred
  result.
- Judge whether normal transfers feel comfortably below 500 ms.

These are the only remaining clipboard acceptance checks; every deterministic
automated gate is complete.

## macOS Hosting Assessment

The pinned Kyber tree does **not** contain a working macOS sender/host.
`kycontroller` and `kynputserver` have macOS build scaffolding, but the actual
`kyavserver` sender is disabled on macOS, macOS host input returns no
configuration, txproto has no ScreenCaptureKit/IOSurface capture backend, and
the app packages only `kyclient`.

A future macOS-host phase can reuse Kymux, controller/session/TLS logic, AV
routing/metrics, and most clipboard protocol machinery. It still needs a
ScreenCaptureKit-to-IOSurface/CVPixelBuffer capture path, low-latency
VideoToolbox H.264/HEVC encoding and capability reporting, CGEvent-based host
input with Accessibility permission, cursor/display-hotplug handling, Screen
Recording permission UX, and signed host/LaunchAgent packaging. Hardware
4:4:4 support must be measured rather than assumed.

## Next Phase Readiness

- Clipboard is ready for the four named physical UAT checks.
- The next Linux-to-Mac technical priorities can return to measured media
  telemetry, audio path qualification, multi-monitor presentation, and
  hardware decode/fidelity evidence without reopening clipboard architecture.
- macOS hosting should be planned as its own platform-port spike rather than
  mixed into Linux-host prototype tuning.

## Self-Check: PASSED

- All three source commits exist.
- All parent-owned clipboard patches and package/provenance files exist.
- The private prerelease and expected asset exist.
- Fresh-download SHA-256 and byte identity passed.
- Host config, service, port, displays, NvFBC, NVENC, and input checks passed.

---
*Quick task: 260730-mqr*
*Completed: 2026-07-30*
