---
quick_id: 260801-ozj
mode: quick
status: partial
date: 2026-08-01
files_modified:
  - scripts/replay-virtual-display
  - scripts/verify-virtual-display.sh
  - prototype/linux/virtual-display/edid-4k60-1.hex
  - prototype/linux/virtual-display/edid-4k60-2.hex
  - prototype/macos/ReplayDesktopLauncher.swift
must_haves:
  truths:
    - The host can explicitly prepare one or two persistent NVIDIA X11 UHD60 virtual outputs for headless or physical-plus-virtual layouts.
    - Virtual outputs are Xorg configuration, not client state, and remain present across ReplayDesktop disconnects until explicitly disabled.
    - Enable and disable preserve the existing user-owned xorg.conf through a recoverable backup and change only a marked ReplayDesktop option block.
    - The Mac client requests at most two displays and uses Kyber's existing independent stream/window path with native per-window fullscreen.
    - No automated command restarts X or the display manager on the live workstation.
---

# Quick Plan 260801-ozj: Persistent dual-UHD virtual displays

Build the smallest host-side NVIDIA/Xorg virtual-display command, prove the
existing dual-window client path with the two real UHD60 displays, and stop at
the explicit X-restart checkpoint before activating a fake panel on this live
desktop.

## Scope boundary

- X11 plus NVIDIA only; maximum two active ReplayDesktop target displays.
- Fixed 3840x2160@60 EDIDs; no client-resolution matching.
- No daemon, control server, host GUI, Wayland, XRandR logical monitor, or
  client-lifecycle creation/removal.
- Preserve unrelated dirty work and the current X session. Never restart the
  display manager automatically because that would terminate this Codex UI.

## Task 1: Add the persistent host enable/status/disable command

**Files**

- `scripts/replay-virtual-display`
- `prototype/linux/virtual-display/edid-4k60-1.hex`
- `prototype/linux/virtual-display/edid-4k60-2.hex`

**Action**

- Implement `status`, `render`, `enable`, and `disable` for layouts
  `headless-one`, `headless-two`, and `add-one`.
- Use NVIDIA `ConnectedMonitor`, `CustomEDID`, `UseDisplayDevice`, and
  `MetaModes`; use DFP-2 first and DFP-4 second, with explicit overrides for
  other hosts. Position heads horizontally at `+0+0` and `+3840+0`.
- Decode and validate two fixed, checksum-valid, uniquely identified 4K60
  EDIDs. Reject invalid EDID, connector aliases, more than two outputs, missing
  NVIDIA/Xorg prerequisites, or a physical-plus-virtual request without an
  explicit retained physical DFP.
- Preserve `/etc/X11/xorg.conf`: create a root-owned recoverable backup on
  first enable, inject/remove only a clearly delimited ReplayDesktop block in
  the selected active NVIDIA Device section, validate the rendered file with
  bounded syntax/contract checks, and install atomically through `sudo`.
- Never restart X. Print the exact display-manager restart, post-restart
  verification, and emergency rollback commands. `disable` restores the
  pre-ReplayDesktop configuration and likewise requires a later X restart.

**Verify**

- Shell syntax passes; `render` is deterministic for all three layouts.
- EDIDs pass `edid-decode` and advertise 3840x2160 at 60 Hz.
- Fixture configs prove marked-block insertion/removal, preservation of all
  other bytes, idempotence, max-two enforcement, and safe rejection paths.

**Done**

- The host tool can safely prepare or remove persistent virtual-output config,
  while activation remains an explicit user-controlled X restart.

## Task 2: Freeze the host and client prototype contract

**Files**

- `scripts/verify-virtual-display.sh`
- `prototype/macos/ReplayDesktopLauncher.swift` only if needed to cap the GUI
  to the supported maximum of two

**Action**

- Add a focused verifier for the host CLI, EDID, render, idempotence, and
  rollback fixtures.
- Cap the technical GUI's multiple-display count at exactly two while keeping
  the existing `--display-count=2` mapping. Do not rewrite Kyber windows: its
  current per-display Kymux/libVLC windows and macOS green-button fullscreen
  already satisfy the prototype.
- Restart only `replaydesktop-kyber-spike.service` so the controller replaces
  stale MST output IDs, then prove it advertises the current DP-5.3 and DP-0
  3840x2160@60 outputs.
- Run one dual-stream session against those two physical outputs if the Mac is
  interactively available; otherwise leave only the visual/fullscreen check
  pending without building UI automation.

**Verify**

- Host verifier passes on Linux.
- Launcher self-test proves `--display-count=2` and rejects counts above two.
- Controller logs/listing use current XRandR IDs rather than stale DP-0.3/DP-5
  IDs after its service-only restart.

**Done**

- The existing two-window path is bounded to the requested prototype and the
  live host's current two-UHD topology is fresh.

## Task 3: Package if the client changed, then stop at activation checkpoint

**Action**

- If Task 2 changes Swift, build/package the next arm64 macOS 15 prerelease
  through the existing clean package pipeline. Otherwise reuse spike.4.
- Run `replay-virtual-display render add-one --physical <current DFP>` on this
  host and report the exact diff, chosen virtual connector, backup path, and
  rollback command.
- Do not run `enable` or restart X until the user explicitly confirms the
  disruptive activation window.

**Done**

- Code and package are ready; the only remaining step is the approved Xorg
  restart and live virtual-output/NvFBC/dual-window proof.
