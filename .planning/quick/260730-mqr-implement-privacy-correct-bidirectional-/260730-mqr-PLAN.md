---
quick_id: 260730-mqr
mode: quick-full
status: ready
date: 2026-07-30
prototype_exceptions:
  - Internal LAN/Tailscale development credentials and TLS verification bypass remain explicitly approved for this prototype.
  - Ad-hoc signing without notarization remains explicitly approved for internal testing.
must_haves:
  truths:
    - macOS-to-Linux and Linux-to-macOS clipboard synchronization works for plain text and HTML over the existing Kyber clipboard events.
    - Clipboard is opt-in and performs zero NSPasteboard reads, writes, polling, or network sends when disabled by the user or refused by the host.
    - Local clipboard snapshots are transferred lazily and never replaced by a later unrelated pasteboard value.
    - Remote Text and HTML representations are fetched sequentially and committed atomically to the Mac pasteboard.
    - Self-writes, same-content re-ownership, stale generations, and disconnect/reconnect do not create echo loops or stale writes.
    - Text plus HTML is bounded to 60 KiB combined, invalid UTF-8 and oversize X11 properties are rejected rather than silently clipped.
    - The technical GUI exposes an honest clipboard toggle and the rebuilt app preserves the proven Kymux media/input baseline.
  artifacts:
    - patches/kyber/0005-kynput-macos-clipboard.patch
    - patches/kyber/0006-kyctl-clipboard-negotiation.patch
    - patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch
    - prototype/macos/ReplayDesktopLauncher.swift
    - prototype/macos/engine-baseline.lock
    - ReplayDesktop-arm64-clipboard-spike.zip
  key_links:
    - NSPasteboard changeCount polling maps to ClipboardEvent::Change and immutable local snapshots answer DataRequest.
    - Requested GUI state plus separately negotiated host copy/paste permissions gate local-read/send and remote-write behavior before any pasteboard access.
    - Existing reliable Kymux input transport carries ClipboardEvent without a new media transport.
    - GUI --clipboard value maps to the negotiated native pipeline and never bypasses host copy/paste policy.
---

# Bidirectional macOS clipboard prototype

## Task 1: Add a privacy-correct native macOS clipboard pipeline

**Files**

- `upstream/kyber-desktop/kysdk/kynput/kynput/src/macos/clipboard.rs`
- `upstream/kyber-desktop/kysdk/kynput/kynput/src/macos/mod.rs`
- `upstream/kyber-desktop/kysdk/kynput/kynput/src/macos/pipeline_config.rs`
- `upstream/kyber-desktop/kysdk/kynput/kynput/Cargo.toml`
- `upstream/kyber-desktop/kysdk/kynput/Cargo.lock`
- Narrow directional capability plumbing in `kysdk/kynput`; the matching
  `kysdk/kyctl` transport changes are owned by Task 2 and patch 0006.

**Action**

- Implement an `NSPasteboard` adapter with pinned `objc2` AppKit/Foundation
  crates and a single named worker that owns all pasteboard operations.
- Poll `changeCount` every 100–250 ms, baseline without advertising existing
  content, snapshot exact Text/HTML values, and advertise only canonical
  formats whose combined UTF-8 size is at most 60 KiB.
- Answer peer `DataRequest` from the immutable advertised snapshot.
- Fetch remote formats sequentially because current responses have no request
  ID; atomically write Text and HTML only after all advertised representations
  validate.
- Implement self-write `changeCount` suppression, exact same-content
  suppression, stale-generation draining, local-copy-wins during a remote
  fetch, and clean worker shutdown.
- Add explicit requested-and-negotiated enablement before pipeline start.
  Carry the two host policies independently: host `allow_paste` permits
  local Mac pasteboard reads/network sends, while host `allow_copy` permits
  remote data to write the Mac pasteboard. A denied direction performs zero
  pasteboard operations and sends zero clipboard packets for that direction;
  when both are denied (or the GUI toggle is off), no worker starts. Routing
  filters alone are insufficient.
- Never log clipboard contents.

**Verify**

- Unit-test with an injected pasteboard backend and fake consumer: initial
  baseline, local Text+HTML, immutable lazy response, remote sequential fetch,
  atomic commit, self/same-content suppression, stale/unsolicited data,
  local-wins race, exact/oversize limits, disabled/refused zero operations, and
  stop/join behavior.
- On macOS, use a uniquely named pasteboard for integration tests rather than
  the user's general pasteboard.
- Run pinned Rust formatting, focused unit tests, and task-scoped clippy.

**Done**

- Native macOS clipboard capability is advertised only when enabled and both
  clipboard directions pass deterministic automated tests without reading the
  user's real pasteboard.

## Task 2: Harden Linux bounds and expose the real GUI control

**Files**

- `upstream/kyber-desktop/kysdk/kynput/kynput/src/linux/clipboard.rs`
- `upstream/kyber-desktop/kysdk/kynput/kynput/src/types/clipboard.rs`
- Directional negotiation in `upstream/kyber-desktop/kysdk/kynput/kynputservice-types`
- Directional response propagation in `upstream/kyber-desktop/kysdk/kyctl`
- `prototype/macos/ReplayDesktopLauncher.swift`
- `patches/kyber/0005-kynput-macos-clipboard.patch`
- `patches/kyber/0006-kyctl-clipboard-negotiation.patch`
- `patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch`
- `README.md`

**Action**

- Reject X11 properties with nonzero `bytes_after` instead of silently clipping
  them at 60 KiB, require strict UTF-8, enforce deserialization bounds, and
  apply a 60 KiB combined Text+HTML policy where the current wire permits it.
- Keep the existing wire event set for this prototype. Serialize remote
  requests and document that deterministic simultaneous cross-peer conflict
  resolution needs a future versioned update/request ID.
- Replace the aggregate negotiated clipboard boolean with copy and paste
  booleans in the matched controller/kynputservice pair and backward-tolerant
  defaulted fields in the external Kymux start response. Mixed-version internal
  positional MessagePack IPC is outside the prototype contract. Preserve the
  requested GUI boolean, map host `allow_copy` to Mac
  remote-write permission and host `allow_paste` to Mac local-read/send
  permission, and apply both before `pipeline.start()`.
- Replace the GUI's unavailable label with an opt-in experimental checkbox:
  `Clipboard sync — Text + HTML, 60 KiB`. Emit exactly one
  `--clipboard=true|false`; default off until the operator opts in.
- Preserve all fixed Kymux, buffering, trust, input, codec, audio, and display
  arguments. Do not conflate clipboard with general input enablement.
- Create three repository-owned patches without committing or pushing upstream
  submodules, plus a separate top-level Kyber Desktop patch so the immutable
  security/package patch 0001 remains byte-for-byte unchanged:
  `0005-kynput-macos-clipboard.patch` is applied from
  `upstream/kyber-desktop/kysdk/kynput`, and
  `0006-kyctl-clipboard-negotiation.patch` is applied from
  `upstream/kyber-desktop/kysdk/kyctl`, while
  `0007-kyber-desktop-clipboard-input-pipeline.patch` is applied from
  `upstream/kyber-desktop` after patch 0001.

**Verify**

- Add malformed, truncated, oversize, strict UTF-8, combined-limit, and
  sequential-request tests.
- Expand the launcher self-test matrix to assert exactly one clipboard flag,
  default-off behavior, and independence from the Input toggle.
- From an exact pinned clean checkout, replay with:
  `git -C upstream/kyber-desktop apply ../../patches/kyber/0001-secure-prototype-packaging.patch`,
  `git -C upstream/kyber-desktop/kysdk/kymedia apply ../../../../patches/kyber/0002-linux-disable-ffmpeg-vulkan.patch`,
  `git -C upstream/kyber-desktop/kysdk/kynput apply ../../../../patches/kyber/0004-linux-hires-wheel.patch`,
  `git -C upstream/kyber-desktop/kysdk/kynput apply ../../../../patches/kyber/0005-kynput-macos-clipboard.patch`, and
  `git -C upstream/kyber-desktop/kysdk/kyctl apply ../../../../patches/kyber/0006-kyctl-clipboard-negotiation.patch`.
  Then replay
  `git -C upstream/kyber-desktop apply ../../patches/kyber/0007-kyber-desktop-clipboard-input-pipeline.patch`.
  Assert no nested source, private identity, logs, or build artifacts enter
  the parent repository. Patch 0003 remains an uncommitted local/user change
  and is not part of this clean replay claim.

**Done**

- Linux no longer silently accepts truncated clipboard data, the GUI toggles
  the genuinely negotiated native pipeline, and source changes are replayable.

## Task 3: Rebuild both sides, publish, and perform clipboard UAT

**Files**

- Rebuilt Linux Kyber runtime
- Rebuilt Apple Silicon `ReplayDesktop.app`
- `prototype/macos/engine-baseline.lock` and package provenance metadata
- `ReplayDesktop-arm64-clipboard-spike.zip`
- `.planning/quick/260730-mqr-implement-privacy-correct-bidirectional-/260730-mqr-SUMMARY.md`

**Action**

- Build the patched Linux runtime from clean pins, preserve the live host
  configuration, restart safely, and re-run login/display/NvFBC/NVENC/input
  regression checks.
- Rebuild the raw macOS Kyber client and native libraries from clean pinned
  source plus the unchanged 0001 packaging patch and repository-owned patches
  0005, 0006, and 0007; update the locked engine/payload provenance, then
  package the technical launcher from a clean parent source range.
- Compile on Apple Silicon with macOS 15 deployment target, run clipboard
  integration tests, the complete launcher matrix, signing, Mach-O/minimum-OS,
  identity scan, source-boundary, exact archive-manifest, raw CLI, raw H.264,
  and Finder-launch gates.
- Before publication, run automated end-to-end tests against the exact local
  archive and runtime to prove both clipboard directions with Text+HTML, each
  host directional policy, disabled/refused zero access, size rejection,
  rapid changes, reconnect, and absence of clipboard contents in logs.
- Only after the exact archive passes every automated gate, publish a new
  private prerelease `v0.3.0-spike.1` with
  `ReplayDesktop-arm64-clipboard-spike.zip`; preserve all prior releases.

**Verify**

- Re-download the GitHub asset solely to prove its SHA-256 and complete
  manifest are identical to the already-qualified local archive; do not use
  publication as a prerequisite for qualification.
- Record `human_needed` for physical cross-application copy/paste on the user's
  Mac, macOS pasteboard privacy prompts, simultaneous real-user changes, and
  subjective transfer latency under 500 ms.

**Done**

- A downloadable private clipboard prerelease passes automated gates and the
  host remains ready; only the named physical pasteboard checks remain.
