---
quick_id: 260801-ozj
phase: quick
plan: 260801-ozj
subsystem: virtual-display
tags: [nvidia, xorg, nvfbc, macos, multi-monitor]

requires:
  - quick: 260730-j91
    provides: Existing Kyber multi-display launcher and independent macOS windows
provides:
  - Persistent NVIDIA Xorg configuration for one or two fixed UHD60 virtual outputs
  - Headless-one, headless-two, and physical-plus-virtual host layouts
  - Exactly-two-display macOS client contract with independent native windows
  - Recoverable enable/disable workflow that never couples displays to client lifetime
affects: [linux-host, nvidia-xorg, nvfbc, macos-launcher, macos-packaging]

key-files:
  created:
    - scripts/replay-virtual-display
    - scripts/verify-virtual-display.sh
    - prototype/linux/virtual-display/edid-4k60-1.hex
    - prototype/linux/virtual-display/edid-4k60-2.hex
    - .planning/quick/260801-ozj-prototype-persistent-x11-virtual-display/260801-ozj-SUMMARY.md
  modified:
    - prototype/macos/ReplayDesktopLauncher.swift

key-decisions:
  - "Create virtual outputs in persistent NVIDIA Xorg configuration, never in the ReplayDesktop client session lifecycle."
  - "Use real scanout heads backed by fixed 3840x2160@60 EDIDs so X11 and NvFBC can enumerate them; XRandR logical monitors are insufficient."
  - "Keep the prototype at exactly two host streams and two independent macOS windows; native per-window fullscreen already supplies the requested behavior."
  - "Never restart X automatically; activation and rollback are explicit disruptive operator checkpoints."

requirements-completed: []

coverage:
  - id: VD1
    description: Host can persistently configure one or two UHD60 virtual NVIDIA X11 outputs
    verification:
      - kind: integration
        ref: scripts/verify-virtual-display.sh
        status: pass
      - kind: manual_procedural
        ref: live display-manager restart and NvFBC enumeration
        status: unknown
    human_judgment: true
    rationale: "The live X restart is deliberately pending explicit approval because it terminates the current graphical session."
  - id: VD2
    description: Mac client accepts exactly two display streams in separate independently fullscreenable windows
    verification:
      - kind: integration
        ref: ReplayDesktopLauncher --self-test and arm64 macOS compile
        status: pass
      - kind: manual_procedural
        ref: live two-window stream/fullscreen smoke
        status: unknown
    human_judgment: true
    rationale: "Kyber's independent windows are retained; a live dual-stream visual smoke remains after host activation."
  - id: VD3
    description: Virtual displays survive client disconnect and remain until explicitly disabled
    verification:
      - kind: unit
        ref: managed Xorg block insertion, idempotence, byte-preserving removal, and rollback fixtures
        status: pass
      - kind: manual_procedural
        ref: disconnect/reconnect against the activated Xorg layout
        status: unknown
    human_judgment: true
    rationale: "Persistence is structurally independent of Kyber, but the final live disconnect proof follows activation."

completed: 2026-08-01
status: partial
---

# Quick Task 260801-ozj: Persistent Dual-UHD Virtual Displays

ReplayDesktop now has the complete prototype host configuration path and a
two-display macOS client package. Source, static verification, root-level
configuration rehearsal, packaging, and live controller refresh are complete.
The only intentionally pending step is restarting this workstation's X session
and proving the new synthetic output through NvFBC and two live Mac windows.

## Accomplishments

- Added `scripts/replay-virtual-display` with `status`, `validate-assets`,
  `render`, `enable`, and `disable` commands for `headless-one`,
  `headless-two`, and `add-one`.
- Added two checksum-valid, uniquely identified 256-byte UHD60 EDIDs. Both
  advertise a preferred 3840x2160 at 60 Hz timing.
- Implemented NVIDIA `ConnectedMonitor`, `CustomEDID`, `UseDisplayDevice`,
  `MetaModes`, and `nvidiaXineramaInfoOrder` configuration without bypassing
  mode validation.
- Preserved the existing Xorg file through a first-enable backup and a clearly
  delimited managed block. Disable removes only ReplayDesktop-managed state.
- Kept virtual outputs independent from client connections. They are Xorg
  state and remain present until the operator disables them and restarts X.
- Capped the macOS technical launcher at exactly two displays. The existing
  Kyber path creates one stream/player/window per host display, and each native
  macOS window can enter fullscreen independently using its green button.
- Restarted only the Kyber controller, not X, so it now advertises the current
  physical outputs `DP-0` (ID 569) and `DP-5.3` (ID 694), both 3840x2160.

## Task Commits

1. Host configuration tool, EDIDs, and verifier: `708adf7`
2. Exactly-two-display macOS launcher: `551d468`

## Verification Evidence

- `scripts/verify-virtual-display.sh` passed EDID decoding, all three layouts,
  idempotence, rollback, unrelated-config preservation, connector rejection,
  and the two-display bound.
- `bash -n` and `git diff --check` passed.
- A root-level rehearsal enabled `headless-two` against a temporary copy of
  the live `/etc/X11/xorg.conf`, installed and decoded both EDIDs, then disabled
  the feature and restored that temporary config byte-for-byte. The live Xorg
  configuration was untouched.
- The Apple Silicon builder compiled the launcher for macOS 15 and its
  self-test passed all 192 argument cases, including rejection of a third
  display.
- The packaged app passed 312 arm64 Mach-O checks, minimum macOS 15.0,
  launcher self-test, deep ad-hoc signature verification, and exact archive
  manifest verification.
- The live controller remains active on `0.0.0.0:8080` after its service-only
  refresh.

## macOS Package

- Version: `v0.3.0-spike.5`
- Builder app:
  `/Users/finn/Developer/replaydesktop-virtual-v0.3.0-spike.5.ZDfepF/output/ReplayDesktop.app`
- Builder archive:
  `/Users/finn/Developer/replaydesktop-virtual-v0.3.0-spike.5.ZDfepF/output/ReplayDesktop-arm64-clipboard-spike.zip`
- Builder download copy:
  `/Users/finn/Downloads/ReplayDesktop-v0.3.0-spike.5-arm64.zip`
- Local download copy:
  `/home/finn/Downloads/ReplayDesktop-v0.3.0-spike.5-arm64.zip`
- Private GitHub prerelease:
  `https://github.com/best-day-ever/replaydesktop/releases/tag/v0.3.0-spike.5`
- Archive size: 40,157,492 bytes
- SHA-256:
  `f94ac3a1fbdb0db1513e528ec2e12beaddc66fdcc6646cf2a54a5996ecb240a5`
- Source commit:
  `551d46831bccb538a56a6ef72875e4aa2e765288`
- Source-boundary SHA-256:
  `e2b3394243769e89b19a7096f6e62759aa7a75d6b34eb0256fa946fd948a0e78`
- Signing: ad-hoc internal prototype; not notarized.

## Prepared Live Layout on This Laptop

The non-mutating preview is `/tmp/replaydesktop-add-one-xorg.conf`. It adds one
managed block to the existing NVIDIA Device section:

- Keep physical NVIDIA head `DFP-6.3` (`DP-5.3`, the left Dell) at `+0+0`.
- Add virtual NVIDIA head `DFP-2` at `+3840+0` using the first ReplayDesktop
  3840x2160@60 EDID.
- Omit currently active physical `DFP-0`, leaving exactly two target displays.

Preparation command, not yet run against the live configuration:

```bash
sudo /home/finn/Documents/linuxremote/scripts/replay-virtual-display enable add-one --physical DFP-6.3
```

Activation command, deliberately pending explicit approval:

```bash
sudo systemctl restart display-manager
```

Rollback prepares the pre-ReplayDesktop Xorg configuration for the next X
restart:

```bash
sudo /home/finn/Documents/linuxremote/scripts/replay-virtual-display disable
sudo systemctl restart display-manager
```

## Remaining Live Proof

1. Enable the prepared `add-one` configuration and restart the display manager.
2. Verify XRandR and NvFBC enumerate the physical and synthetic UHD60 outputs.
3. Connect with `v0.3.0-spike.5`, choose Multiple, and prove two simultaneous
   3840x2160@60 streams in two independent windows.
4. Fullscreen each window independently, disconnect, confirm the virtual panel
   remains, and reconnect.

The display-manager restart will terminate the current graphical session,
including this Codex UI and any unsaved GUI applications. No live activation
or virtual-NvFBC claim has been made before that checkpoint.

## Deviations from Plan

- The live two-window smoke and virtual-output activation were intentionally
  deferred to the explicit disruptive checkpoint.
- The package used the installed Xcode 26.2, SDK 26.2, and Swift 6.2.3 rather
  than the preferred Xcode 26.6 qualification lane. All prototype gates passed.

## Self-Check: PARTIAL

All non-disruptive source, configuration, packaging, and controller checks
passed. Live X activation, NvFBC capture, per-window fullscreen, and
disconnect-persistence remain the final human-visible prototype proof.

---
*Quick task: 260801-ozj*
*Source and package completed: 2026-08-01*
