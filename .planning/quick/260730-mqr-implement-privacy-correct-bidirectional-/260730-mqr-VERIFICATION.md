---
quick_id: 260730-mqr
verified: 2026-07-30T15:59:23Z
status: human_needed
score: 5/7 must-haves verified
behavior_unverified: 2
overrides_applied: 0
behavior_unverified_items:
  - truth: "Real macOS-to-Linux and Linux-to-macOS Text plus HTML clipboard synchronization works across user applications."
    test: "Use the released app against the live Linux X11 host and copy plain text and rich HTML in both directions between real applications."
    expected: "Both representations arrive intact in each direction without an echo or unrelated clipboard replacement."
    why_human: "The verifier proved the exact archive, compiled native pipeline, transport wiring, and state-machine tests, but did not mutate the operator's real general pasteboard during independent verification."
  - truth: "Disconnect and reconnect cannot replay a stale clipboard generation."
    test: "Begin a clipboard transfer, disconnect before completion, reconnect, then make a fresh clipboard change."
    expected: "No pre-disconnect value is written; only the first new post-reconnect generation transfers."
    why_human: "Unit tests cover stale generations, local-wins, stop/join, and pipeline reconstruction separately; no verifier-run test exercises the complete live reconnect transition."
human_verification:
  - test: "Copy plain and rich text between real macOS and Linux applications in both directions."
    expected: "Text and HTML arrive intact, with no loop or unrelated replacement."
    why_human: "Requires the operator's general pasteboard and live applications."
  - test: "Interrupt a transfer with disconnect/reconnect, then copy a new value."
    expected: "No stale pre-disconnect value appears after reconnect."
    why_human: "Requires a live session transition and real pasteboards."
  - test: "Observe macOS pasteboard privacy behavior and make simultaneous changes on both machines."
    expected: "Prompts are acceptable and the documented local-wins behavior is understandable; the current no-request-ID conflict limitation remains visible."
    why_human: "OS privacy UI and simultaneous operator timing cannot be established by source inspection."
  - test: "Judge normal clipboard transfer latency."
    expected: "Ordinary transfers feel comfortably below 500 ms."
    why_human: "Subjective latency needs physical use."
  - test: "Run the package on macOS 27 beta."
    expected: "The app launches and the same clipboard flow works."
    why_human: "The qualified builder was macOS 15.7.7; macOS 27 compatibility is explicitly unclaimed."
---

# Quick Task 260730-mqr Verification Report

**Goal:** Ship a privacy-correct, bidirectional Text/HTML clipboard prototype between the Apple Silicon macOS client and Linux X11 host, with negotiated directional gates, strict bounds, reproducible patches, and an exact private prerelease.

**Status:** `human_needed` — no implementation or release blocker found. Five truths have direct behavioral evidence; two live integration invariants remain for operator UAT.

## Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Bidirectional Text+HTML synchronization works over existing Kyber events | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Native adapter and Kymux wiring are substantive; the shipped `libkynput` links AppKit and contains the clipboard worker. The verifier did not touch the real general pasteboard. |
| 2 | Disabled/refused directions perform zero unauthorized pasteboard work or clipboard sends | ✓ VERIFIED | `both_denied_creates_no_worker...` and `denied_directions_perform_no_directional_operations` passed; negotiation fails closed and is applied before pipeline start. |
| 3 | Local snapshots are lazy and immutable | ✓ VERIFIED | `baseline_then_local_snapshot_is_lazy_and_immutable` passed in the independently run 10-test state-machine binary. |
| 4 | Remote Text/HTML fetches are sequential and commit as one pasteboard item | ✓ VERIFIED | `remote_formats_are_fetched_sequentially_and_committed_once` passed; native code writes one `NSPasteboardItem` containing both representations. |
| 5 | Echo/stale/reconnect suppression holds | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Self-write, same-content, stale-generation, local-wins, stop/join tests passed; the complete live disconnect/reconnect transition was not independently exercised. |
| 6 | Combined 60 KiB, strict UTF-8, packet and X11 truncation bounds are enforced | ✓ VERIFIED | Verifier ran 10 wire tests and 4 Linux budget tests. Source rejects `bytes_after != 0`, invalid UTF-8, oversize declarations, trailing bytes, and combined overflow. |
| 7 | GUI is honest/default-off/independent and the rebuilt app preserves the baseline | ✓ VERIFIED | Shipped launcher self-test passed 192 combinations; exact app/archive verifier passed 312 arm64 Mach-O, macOS 15.0 minimum, deep signing, source boundary, raw-engine hashes, and archive manifest. |

**Score:** 5/7 verified; 2 present but behavior-unverified.

## Artifacts and Wiring

| Artifact / link | Status | Evidence |
|---|---|---|
| `0005-kynput-macos-clipboard.patch` | ✓ VERIFIED | 2,286-line substantive patch; cleanly replays after 0004; adds native worker, Linux/wire hardening, C/Rust API plumbing, and tests. |
| `0006-kyctl-clipboard-negotiation.patch` | ✓ VERIFIED | Clean replay; host `allow_paste → local read/send` and `allow_copy → remote write`; absent/legacy response fails closed. |
| `0007-kyber-desktop-clipboard-input-pipeline.patch` | ✓ VERIFIED | Clean replay after immutable 0001; clipboard can start without interactive input and remains independently gated. |
| Launcher and packaging scripts | ✓ VERIFIED | Checkbox label is exact, defaults off, emits one `--clipboard=true|false`, and is covered by the shipped self-test. |
| Existing Kymux input transport | ✓ WIRED | Negotiation configures the pipeline before `pipeline.start()`; `ClipboardEvent` remains on the reliable input path with no new media transport. |
| Private release asset | ✓ VERIFIED | Private prerelease, tag pinned to `c1f48ad`, sole 40,127,902-byte asset; fresh download and qualified builder archive are byte-identical at SHA-256 `87da42b2df9512fbce6c692496735d164e64ce0f9f7057fb1e0a782bce188265`. |

No clipboard content is interpolated into new logs; logging is limited to status, direction booleans, formats, generations, and byte counts.

## Behavioral and Probe Evidence

| Check | Result |
|---|---|
| Rust 1.89 macOS clipboard state-machine test binary | 10 passed |
| Rust 1.89 clipboard wire/bounds tests | 10 passed |
| Rust 1.89 Linux combined-transfer-budget tests | 4 passed |
| Kycontroller legacy one-way fail-closed test | 1 passed |
| Shipped macOS launcher `--self-test` | PASS, 192 combinations |
| Clean committed source-boundary replay | PASS; digest `ca9f7e4f5066d30f07d4f051d4db9a71aa458627d7b7a8b997589a32858b548d` |
| Exact macOS app/archive qualification | PASS; 312 thin arm64 Mach-O, min macOS 15.0, deep ad-hoc signature, exact manifest |
| Fresh GitHub download vs qualified builder archive | PASS; byte-for-byte, ZIP integrity, 331 entries and normalized manifest match |

The local Kyclient unit-test command without its full native build environment failed on missing `kynput.pc`; this is an environment limitation, not release evidence. The exact packaged native client and full archive verifier passed on the Apple Silicon builder.

## Live Host and Repository Boundary

- `replaydesktop-kyber-spike.service` is active; `kycontroller` listens on TCP `0.0.0.0:8080`.
- Config SHA-256 is the restored `3ad53c4455b886077be87618457869c3d5017189f4bc43bf114d240457ffd12e`, mode 0600, with NVENC, NvFBC, and both clipboard directions enabled.
- XRandR reports `DP-0.3` primary and `DP-5`, both 3840×2160. The current service instance's journal proves NvFBC, `h264_nvenc`, Kynput, keyboard, and mouse startup during its successful prior session; the service is presently idle.
- Commits `d9909b9 → 1504085 → c1f48ad` contain exactly eight task-related parent paths. No nested gitlink, nested commit, patch 0003, ZIP, log, private identity, or planning artifact entered those commits.
- Patch 0001 remains byte-identical at SHA-256 `bf5035d6636d00ccf57b999375a84608bdaa350b253e52735766252ac4a81c28`.

## Anti-Patterns and Limitations

- No `TBD`, `FIXME`, or `XXX` blocker exists in task-modified files.
- Generated patch files contain whitespace-only added lines; this is non-functional patch formatting debt and does not affect replay.
- The wire still has no request/update ID, so deterministic simultaneous-peer conflict resolution is intentionally not claimed.
- The release was qualified on macOS 15.7.7 with Xcode 26.2, not macOS 27 beta.

## Conclusion

The code, tests, packaging, release, and live-host baseline support a credible clipboard prototype with no observed blocker. Final acceptance requires the five operator checks in frontmatter, chiefly real cross-application copy/paste and reconnect behavior.

---

_Verifier: gsd-verifier (independent goal-backward review)_
