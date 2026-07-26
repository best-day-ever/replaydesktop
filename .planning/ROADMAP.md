# Roadmap: ReplayDesktop

## Overview

ReplayDesktop advances through evidence-gated vertical slices: first make the
real Linux host trustworthy, then establish a pinned and authenticated Kyber
session, prove an observable NVIDIA H.264 baseline, constrain all media choices
with live capability evidence, validate the exact macOS decode/presentation
path, and only then expand the latency/fidelity matrix. Native immersive input
and bounded clipboard synchronization complete the reference workflow before
the final compatibility and transferability gate. A failed stop/go gate blocks
dependent evidence; it is never bypassed by a silent software path, fallback
tuple, or weakened platform claim.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): planned milestone work
- Decimal phases (2.1, 2.2): urgent insertions, ordered numerically

- [ ] **Phase 1: Host Readiness Gate** - Repair and prove the real Xorg/NVIDIA host before any media result is admissible.
- [ ] **Phase 2: Pinned Secure Kyber Baseline** - Rebuild the exact fork and run an authenticated direct H.264 session from an arm64 Mac.
- [ ] **Phase 3: Instrumented NVIDIA 4K60 Baseline** - Prove an honest, bounded, measurable NvFBC/NVENC H.264 path.
- [ ] **Phase 4: Evidence-Backed Media Contract** - Negotiate one versioned live-proven tuple and reject stale or impossible choices before allocation.
- [ ] **Phase 5: Proven macOS Decode and Presentation** - Identify the exact H.264 decoder, surface, copy, queue, and presentation path.
- [ ] **Phase 6: Negotiated Latency and Fidelity Matrix** - Add strict codec modes and pass the final 4K60 latency and fidelity gates.
- [ ] **Phase 7: Native Immersive Input** - Deliver permission-aware, lifecycle-safe fullscreen mouse and eligible keyboard control.
- [ ] **Phase 8: Authenticated Bidirectional Clipboard** - Synchronize bounded text and HTML between NSPasteboard and X11 without loops or leakage.
- [ ] **Phase 9: Compatibility and Transferable Proof** - Qualify only real platform rows and archive a reproducible, legally complete acceptance bundle.

## Phase Details

### Phase 1: Host Readiness Gate

**Goal:** An operator has a real Xorg/NVIDIA host whose selected physical-output capture and encode prerequisites are proven; no later media evidence is admissible until they pass.
**Mode:** mvp
**Depends on:** Nothing (mandatory execution prerequisite)
**Requirements:** HOST-01, HOST-02, HOST-03, HOST-04
**Success Criteria** (what must be TRUE):

  1. The host-doctor identifies and blocks a non-Xorg session, NVIDIA kernel/userspace mismatch, failed NVML, missing physical output, missing `/dev/uinput`, or missing DRM/render access with a specific remediation reason.
  2. An operator can select exactly one physical X11 output and see its XRandR name, dimensions, refresh rate, desktop origin, and owning GPU before capture starts.
  3. The doctor creates the selected output's NvFBC shared-CUDA path, accounts for every GPU/CPU or cross-GPU copy to the encoder, and successfully runs one-frame probes for every NVENC tuple it advertises.
  4. A durable G0 result is PASS only on the real Xorg host with matching driver components, working input/render access, a physical output, NvFBC, and exact NVENC probes; any failed check stops dependent media work.

**Gate:** G0 — Host readiness. Failure stops all media implementation and invalidates capture/encode evidence.
**Research flag:** Live NvFBC surface ownership, copy boundaries, cursor behavior, and driver-specific capture behavior require host investigation during planning.
**Plans:** 10 plans

Plans:
**Wave 1**

- [ ] 01-01-PLAN.md — Foundation, audited dependencies, digest, and final forward-compatible evidence envelope

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 01-02-PLAN.md — Doctor CLI, bounded workers, diagnostic admission, and atomic evidence readback

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 01-03-PLAN.md — Native local-Xorg and source/runtime NVML proof

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 01-04-PLAN.md — Immutable self-contained pre-reboot FAIL archive

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 01-05-PLAN.md — Output/GPU mapping spike and pure unique-correlation contract

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 01-06-PLAN.md — Explicit selected-output integration and corrected-host proof

**Wave 7** *(blocked on Wave 6 completion)*

- [ ] 01-07-PLAN.md — NvFBC source/ABI, fixture, frame/ledger, and cleanup contracts

**Wave 8** *(blocked on Wave 7 completion)*

- [ ] 01-08-PLAN.md — Operator gate and real selected-output one-frame NvFBC proof

**Wave 9** *(blocked on Wave 8 completion)*

- [ ] 01-09-PLAN.md — NVENC copy-boundary spike, closed policy, and bounded parsers

**Wave 10** *(blocked on Wave 9 completion)*

- [ ] 01-10-PLAN.md — Authenticated SDK 13.1 exact tuple probes, final G0, and current archive

### Phase 2: Pinned Secure Kyber Baseline

**Goal:** A user can reproducibly start the pinned Linux host and securely reach a basic H.264 4:2:0 desktop session from the arm64 macOS client by direct address.
**Mode:** mvp
**Depends on:** Phase 1 (G0 passed)
**Requirements:** BASE-01, BASE-02, BASE-03, BASE-04, CONN-01, CONN-02, CONN-03, CONN-04, CONN-05, COMP-01
**Success Criteria** (what must be TRUE):

  1. Another operator can recreate the exact Kyber 0.27.0 recursive tree with locked tools and hashes, inspect every narrow local patch in its ledger, tie the G1 smoke report to the complete source/build/runtime manifest, and obtain the license notices, corresponding-source instructions, and SPDX-compatible SBOM.
  2. The macOS 15+ arm64-only client can connect to a directly reachable hostname or IP plus optional port and receive basic H.264 4:2:0 pixels and input over the real network without discovery, relay, or a control service.
  3. A user can cancel the resolving, connecting, authenticating, or probing flow and can distinguish DNS failure, timeout, refusal, TLS identity failure, authentication rejection, and successful streaming.
  4. Each host uses a unique non-sample identity and secret, while the client verifies an explicitly provisioned fingerprint or private CA and hard-fails any identity mismatch without trust-all, silent TOFU, or silent repinning.
  5. Media capabilities, media endpoints, input, and clipboard remain unavailable before authentication; explicit disconnect closes them with a stable reason and does not reconnect automatically.

**Gate:** G1 — Reproducible secure baseline. Failure stops feature and performance work on a moving or unauthenticated session.
**Plans:** TBD

### Phase 3: Instrumented NVIDIA 4K60 Baseline

**Goal:** An operator can run a ten-minute engineering baseline whose evidence proves the existing reliable-Kymux H.264 path is NVIDIA-backed, bounded, and independently measurable at every stage.
**Mode:** mvp
**Depends on:** Phase 2 (G1 passed)
**Requirements:** HOST-05, HOST-06, PERF-01, PERF-02, PERF-03, PERF-08
**Success Criteria** (what must be TRUE):

  1. Host startup reports the selected capture backend, GPU PCI identity, driver/API and NVENC generation, input format, conversion route, codec controls, and bitrate expectations, and the session aborts rather than silently using XCB or software encoding.
  2. Every video frame can be followed by one identity through capture, conversion, encode, send, receive, decode, presentation queue, and render submission, while input timing reports client-send, Linux-injection, and clock-offset uncertainty.
  3. Video runs with no configured multi-frame buffer, a presentation queue of at most one, and explicit acquired/dropped/encoded/sent/received/decoded/presented counters and drop reasons.
  4. A ten-minute mixed-content 3840×2160@60 engineering run exports non-blocking raw JSONL and a readable PASS/FAIL report with latency distributions, queue depth, network statistics, requested/actual tuple, hardware paths, platform identities, source revisions, and session ID.
  5. G2 passes only when the report proves NvFBC, all copy/conversion boundaries, NVENC, bounded queues, and absence of an XCB/CPU/software fallback; otherwise no performance claim is made.

**Gate:** G2 — Honest NVIDIA path. Failure blocks optimization and invalidates the baseline rather than relaxing the path.
**Research flag:** NvFBC-to-NVENC ownership and copy telemetry must be validated against the corrected live driver/session.
**Plans:** TBD

### Phase 4: Evidence-Backed Media Contract

**Goal:** The authenticated peers represent media selection as one bounded, versioned tuple and reject impossible choices before allocating media resources.
**Mode:** mvp
**Depends on:** Phase 3
**Requirements:** CODE-01, CODE-04
**Success Criteria** (what must be TRUE):

  1. The control plane treats codec, profile, chroma, bit depth, raster, frame rate, encoder path, and decoder path as one versioned media tuple rather than independently combinable flags.
  2. AV1 4:4:4 and any other impossible tuple fail deterministically with a stable structured reason such as `CODEC_PAIR_INVALID` before media endpoints or encoder/decoder resources are created.

**Gate:** Contract gate — impossible, stale, oversized, or version-incompatible capability data must fail deterministically before endpoint allocation.
**Plans:** TBD

### Phase 5: Proven macOS Decode and Presentation

**Goal:** An operator can prove the exact H.264 client decode-to-presentation path and use that evidence to retain libVLC or activate only the narrow native player adapter that is necessary.
**Mode:** mvp
**Depends on:** Phase 4
**Requirements:** CODE-05, CODE-06, CODE-09
**Success Criteria** (what must be TRUE):

  1. Session creation invalidates stale evidence and re-probes against the exact GPU, driver, OS, display, application build, and representative bitstream before accepting a client tuple.
  2. An exact representative 4K H.264 bitstream reports the actual decoder implementation and hardware/software status, decoded chroma/pixel format, output surface, every copy, and final renderer.
  3. Presented output preserves the exact 3840×2160 SDR raster, aspect ratio, declared range/matrix/chroma, and the content rectangle later used for input mapping, while G3 exposes queue/presentation timing and makes no HDR or wide-gamut claim.
  4. The recorded packet fixture produces an archived keep-libVLC or activate-native-adapter decision; failure of exact decode/presentation evidence removes that tuple from `Auto`, and any required native path keeps libVLC as a diagnostic comparison.

**Gate:** G3 — Exact client decode and presentation. Codec-wide APIs or Mac model names are not sufficient evidence.
**Research flag:** Direct endpoint ownership, parameter-set conversion, libVLC observability, VideoToolbox session properties, IOSurface formats, and Metal semantics need a focused player spike.
**Plans:** TBD

### Phase 6: Negotiated Latency and Fidelity Matrix

**Goal:** A user can run honest exact codec modes, demonstrate HEVC 4:4:4 fidelity, and pass the complete 4K60 acceptance gate on at least one fully hardware-accelerated tuple.
**Mode:** mvp
**Depends on:** Phase 5 (G3 passed for the baseline)
**Requirements:** CODE-02, CODE-03, CODE-07, CODE-08, PERF-04, PERF-05, PERF-06, PERF-07
**Success Criteria** (what must be TRUE):

  1. `Auto` reports every candidate and rejection reason and selects only the fully hardware-proven intersection of live host encode, exact client decode, renderer, raster, and frame-rate evidence.
  2. A manual request for HEVC 4:4:4, HEVC 4:2:0, AV1 4:2:0, or H.264 4:2:0 either runs that exact tuple or fails without substitution; AV1 runs only when the actual host and client path prove it.
  3. Performance modes require a hardware decoder, while manual HEVC 4:4:4 warns before any measured software decode and records `chroma_format_idc=3`, full-resolution chroma planes, and the one-pixel stress-chart result.
  4. G4 passes only when the accepted tuple, first packet, bitstream identity, decoded output, and session report agree; any mismatch aborts the session, invalidates the cached evidence, and re-probes rather than substituting.
  5. After warm-up, the 30-minute wired-LAN workload meets the specified rolling frame/drop and p95/p99 stage/end-to-end/input budgets, and the external input-to-photon audit records its method and uncertainty; G5 failure blocks the 4K60 claim without weakening the workload, raster, frame rate, queue, or tuple.

**Gate:** G4 — Codec identity, followed by G5 — 4K60 acceptance. A software HEVC 4:4:4 result can pass fidelity identity but cannot satisfy the hardware latency gate.
**Research flag:** Exact HEVC 4:4:4 and AV1 4:2:0 behavior must be measured on the target Apple Silicon generations and packaged client.
**Plans:** TBD

### Phase 7: Native Immersive Input

**Goal:** A user can control the selected Linux desktop immersively with native macOS mouse and eligible keyboard input, with visible permission state and deterministic local recovery.
**Mode:** mvp
**Depends on:** Phase 6
**Requirements:** INPT-01, INPT-02, INPT-03, INPT-04, INPT-05, INPT-06
**Success Criteria** (what must be TRUE):

  1. Before immersive capture, the arm64 client explains required Accessibility/Input Monitoring permissions and leaves remote input visibly disabled when either permission is denied.
  2. Fullscreen immersive mode hides and locks the local pointer, forwards unclipped relative deltas, buttons, and wheel events, and always exposes a tested local release chord.
  3. Absolute events map through the displayed content rectangle to the selected output, ignore letterbox bars, clamp correctly, and remain accurate under Retina and window scaling.
  4. Physical key down/up, repeat, left/right modifiers, navigation keys, and function keys arrive reliably and in order, while the macOS 15/current macOS 27 beta matrix labels each relevant shortcut as remote, local-only, or release chord.
  5. Focus loss, release chord, disconnect, termination, or host-session loss restores the local pointer and releases every remotely held key and mouse button.

**Gate:** Immersive-input gate — permission, focus-loss, disconnect, sleep, modifier, repeat, pointer-lock, and every release path pass on the target macOS lanes.
**Research flag:** macOS 15 and current macOS 27 beta event-tap permissions and reserved shortcut behavior require empirical testing.
**Plans:** TBD
**UI hint:** yes

### Phase 8: Authenticated Bidirectional Clipboard

**Goal:** A connected user can copy bounded text and HTML in either direction between macOS and X11 without loops, data leakage, or interference with video and input.
**Mode:** mvp
**Depends on:** Phase 7
**Requirements:** CLIP-01, CLIP-02, CLIP-03, CLIP-04, CLIP-05
**Success Criteria** (what must be TRUE):

  1. During an authenticated session, UTF-8 plain text and HTML synchronize both ways between general NSPasteboard and X11 `CLIPBOARD`, while X11 `PRIMARY` and binary/image/file formats remain untouched.
  2. A normal valid change reaches the other endpoint within 500 ms and preserves both text and HTML representations when both were supplied.
  3. An update above 60 KiB, malformed data, or an unsupported MIME type is rejected without truncation or changing the destination clipboard.
  4. Origin, hash, and sequence/version handling suppresses echoes and resolves simultaneous observations deterministically without starving video or input.
  5. Clipboard bridging starts only after authentication, logs no clipboard content, stops on disconnect, and leaves both endpoint clipboards intact.

**Gate:** Clipboard gate — both directions pass lifecycle, rapid-update, simultaneous-change, malformed, oversize, reconnect, and no-content-in-logs tests without impairing media or input.
**Plans:** TBD

### Phase 9: Compatibility and Transferable Proof

**Goal:** Another internal developer can reproduce the accepted prototype and rely only on platform rows, errors, and legal artifacts proven by the exact packaged builds on real hardware.
**Mode:** mvp
**Depends on:** Phase 8
**Requirements:** COMP-02, COMP-03, COMP-04, COMP-05, COMP-06
**Success Criteria** (what must be TRUE):

  1. The matrix records build, launch, connection, video, input, clipboard, security, and capability-probe results for macOS 15 Sequoia, the current stable supported macOS, and the current macOS 27 beta without weakening stable behavior for the beta.
  2. Decoder claims are tied to exact Apple Silicon generations and real runs on at least one pre-M3 and one M3-or-newer Mac; AV1 is not described as broadly available until those rows prove it.
  3. Current Arch/CachyOS-, Ubuntu 24.04-, and Rocky 9-derived X11/NVIDIA systems each have reproducible build/start/smoke instructions, and runtime support is claimed only for rows that passed on real hardware.
  4. Clean launch and fault/impairment tests produce stable actionable platform, X11, NVIDIA, capture, encoder, decoder, permission, clipboard, TLS, and authentication errors without exposing credentials or clipboard contents.
  5. The final acceptance archive combines each passing platform row and session report with the Phase 2 source/patch/build/runtime manifests, license notices, corresponding-source instructions, and SPDX-compatible SBOM so another operator can reproduce the proof.

**Gate:** G6 — Platform claims. Only independently evidenced real-hardware rows pass; container builds or the reference host never imply another row.
**Research flag:** Rocky runtime integration and current-beta signing, permission, and packaging regressions need platform-specific qualification.
**Plans:** TBD

## Progress

**Execution Order:**
Phases execute in numeric order. Every named gate is stop/go for its dependent
phases.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Host Readiness Gate | 0/10 | Not started | - |
| 2. Pinned Secure Kyber Baseline | 0/TBD | Not started | - |
| 3. Instrumented NVIDIA 4K60 Baseline | 0/TBD | Not started | - |
| 4. Evidence-Backed Media Contract | 0/TBD | Not started | - |
| 5. Proven macOS Decode and Presentation | 0/TBD | Not started | - |
| 6. Negotiated Latency and Fidelity Matrix | 0/TBD | Not started | - |
| 7. Native Immersive Input | 0/TBD | Not started | - |
| 8. Authenticated Bidirectional Clipboard | 0/TBD | Not started | - |
| 9. Compatibility and Transferable Proof | 0/TBD | Not started | - |
