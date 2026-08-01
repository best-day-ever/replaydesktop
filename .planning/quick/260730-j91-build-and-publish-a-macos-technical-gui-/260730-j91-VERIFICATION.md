---
quick_id: 260730-j91
status: human_needed
verified: 2026-07-30
release: v0.2.0-spike.3
release_sha256: b2895483fe9859aac300e98d262e3dc3ca0de3c4abe018cea22a0c9dd0edbbb5
---

# Verification: ReplayDesktop technical macOS GUI spike

## Goal verdict

The automated implementation and publication goal is achieved. The final
result remains `human_needed` because physical Mac interaction, audible audio,
two-screen presentation, and visibly changing telemetry cannot be established
from packaging and remote process evidence alone.

## Automated evidence

- `ReplayDesktopLauncher.swift` retains `Contents/MacOS/kyclient` as the raw
  engine and builds one argument per setting from the proven Kymux baseline.
- The input control maps to `--inputs`; `--keyboard-grab=false` is emitted
  exactly once. The GUI states that macOS system-shortcut suppression is
  unavailable and rejects an explicit grab request.
- Clipboard is visibly unavailable in this build and no clipboard argument is
  emitted.
- H.264 4:2:0 is labeled proven; HEVC, HEVC 4:4:4, and AV1 4:2:0 are labeled
  experimental. AV1 4:4:4 is unavailable and rejected.
- Single-display and multi-display controls map to `--display-idx` and bounded
  Kymux `--display-count`; the GUI identifies bitrate as a shared total.
- Audio controls map only to the existing `--audio`, `--audio-buffer`, and
  `--kymux-audio` settings.
- Child output, `metrics.json`, and `log/kyclient.log` are polled every 0.5
  seconds into a bounded display. Visible update cadence remains human UAT.
- The launcher self-test passed 96 input/codec/display/audio combinations.
- The high-resolution wheel patch passed five focused tests and preserves
  legacy wheel detents while emitting immediate 120-unit high-resolution
  events.
- The package binds the pinned Kyber/Kysdk/Kynput commits, raw input engine
  digest, immutable Mach-O payload digest, signature boundary, source commit
  range/digest, compiler/SDK identity, and deployment target.
- The qualified archive contains 312 arm64 Mach-O files, declares supported
  deployment targets with launcher and raw client at macOS 15.0, passes deep
  strict ad-hoc signature verification, contains no test identities, and is
  manifest-identical after extraction.
- GitHub's downloaded `ReplayDesktop-arm64-gui-spike.zip` reports and matches
  SHA-256
  `b2895483fe9859aac300e98d262e3dc3ca0de3c4abe018cea22a0c9dd0edbbb5`.
- A raw H.264 smoke reached the Linux host, enumerated both 4K displays,
  started Kymux video/input, used NVENC on the host, and selected VideoToolbox
  on the client.
- The Linux user service was restored after final packaging and is currently
  active on TCP 8080 with the rebuilt scroll runtime.

## Human evidence received

- The operator reports that HEVC 4:4:4 appears to work and feels responsive.
  This proves useful functional behavior but does not establish the exact
  hardware/software decode module or chroma path.

## Remaining human checks

1. Finder-launch the final Spike 3 app and connect through the GUI.
2. Confirm rendered pixels plus focused mouse/keyboard input.
3. Confirm short physical trackpad gestures scroll, including direction and
   subjective feel.
4. Confirm the metrics/log panel visibly changes during a live session.
5. Test two-display mode on a Mac with two physical screens.
6. Test system audio after selecting or adapting a compatible host monitor.
   The current four-channel M-Audio monitor fails capture initialization with
   `Invalid argument`, so no working-audio claim is made.

## Known qualification deviations

- Xcode 26.2 was the newest installed builder rather than the planned 26.6
  lane; the actual version is embedded in package provenance.
- Full all-target kynput clippy still reports pre-existing pinned upstream lint
  debt outside the changed wheel code. Focused tests, formatting, task-scoped
  lint, clean patch replay, and the release build pass.
- GitHub repository release immutability is disabled. Spike 1, 2, and 3 were
  preserved without overwriting tags or assets; no destructive release-policy
  change was authorized.
