# Project Research Summary

**Project:** LinuxRemote (ReplayDesktop prototype)
**Domain:** Direct, high-performance Linux X11/NVIDIA to Apple Silicon macOS remote desktop
**Researched:** 2026-07-26
**Confidence:** MEDIUM

## Executive Summary

LinuxRemote is a proof-oriented remote desktop, not a general remote-access product. The first milestone must prove that one physical NVIDIA-backed X11 desktop can be securely controlled from one directly reachable Apple Silicon Mac at a sustained, measurable 3840×2160@60 Hz. Experts minimize variables in this kind of work: preserve a known media and transport stack, probe the exact runtime path instead of trusting model tables, bound every queue, and make capture, conversion, encode, transport, decode, presentation, and input timing independently observable.

The recommended implementation is a thin fork of Kyber Desktop 0.27.0 at `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`, with every recursive gitlink and coordinated FFmpeg/VLC dependency pinned. Preserve Kyber/Kymux transport, session, input, and initial libVLC behavior; add narrow seams for versioned media capabilities, host and client probes, deterministic mode selection, macOS input and clipboard adapters, and correlated telemetry. Establish a secure H.264 4:2:0 baseline first, then add formats one exact tuple at a time.

There are two deliberately separate video outcomes. The latency path is a fully hardware-proven 4:2:0 path: normally HEVC 4:2:0, with H.264 4:2:0 as the compatibility fallback and AV1 4:2:0 experimental until an Ada-or-newer host and the actual Mac playback path both pass exact-bitstream and 4K60 gates. The fidelity path is manual HEVC 4:4:4, with bitstream and output-chroma evidence and an explicit hardware/software decoder label. A software-decoded HEVC 4:4:4 experiment may demonstrate fidelity but does not satisfy the hardware latency gate. AV1 4:4:4 must not exist in the contract or UI. The largest risks are the current host's invalid Wayland/driver state, hidden copy or software fallback, unproven Apple decode paths, tail latency hidden by buffering, and premature divergence from Kyber; each is a stop/go gate below.

## Reconciled Decisions and Hard Feasibility Gates

### Reconciled Research Decisions

| Question | Roadmap decision |
|----------|------------------|
| Pinned libVLC or an immediate native VideoToolbox/Metal player? | Keep pinned Kyber libVLC as the mandatory diagnostic baseline. Add a player-adapter seam, exact VideoToolbox probe, and libVLC decoder/queue/presentation instrumentation. Build the native `AVPacket -> VideoToolbox -> CVPixelBuffer/IOSurface -> Metal` adapter only if measurement shows libVLC cannot expose or meet decoder-path, copy, queue, presentation, or required-codec gates. Experimental AV1 may independently require that adapter. |
| 60 KiB or 1 MiB clipboard payloads? | Use the existing 60 KiB wire-class limit for the first prototype, reject oversize updates without altering the destination, and never truncate UTF-8 silently. The proposed 1 MiB limit is a later versioned contract change only after Kynput framing, allocation, abuse, and latency tests prove it safe. |
| Ten-minute or thirty-minute performance gate? | Use ten minutes for engineering smoke runs and thirty minutes after warm-up for the milestone PASS/FAIL acceptance report. |
| Which transport mode is the baseline? | Start with reliable Kymux and zero configured video buffering. Test unreliable/FEC modes only after the deterministic reliable 4K60 baseline and promote one only with better measured tail latency and recovery. |
| How is direct-host trust established? | Provision an explicit certificate fingerprint or private CA and fail closed. Do not ship sample identities, trust-all, silent TOFU, or silent re-pinning. |

### Stop/Go Gates

| Gate | Required evidence | Failure disposition |
|------|-------------------|---------------------|
| **G0 — Host readiness** | Real Xorg session; matching loaded NVIDIA kernel and userspace versions; NVML, NvFBC shared-CUDA capture, one-frame exact NVENC, `/dev/uinput`, render access, and selected physical output all pass a host-doctor check | Stop media work. The current development host is on Wayland with an NVIDIA kernel/userspace mismatch and must be rebooted into a matching kernel and X11 session first. |
| **G1 — Reproducible secure baseline** | Exact Kyber tree rebuilds; unique host identity and authentication work; H.264 4:2:0 pixels and input cross the real network with certificate verification | Stop feature work. Do not debug performance on a moving dependency tree or insecure session. |
| **G2 — Honest NVIDIA path** | Telemetry proves NvFBC capture, every copy/conversion boundary, NVENC for the accepted tuple, bounded queues, and no XCB/CPU/software fallback | Fail the performance claim. XCB remains diagnostic-only. |
| **G3 — Exact client decode and presentation** | A representative 4K bitstream with the negotiated profile/chroma creates and decodes successfully; the actual decoder path and output surface are reported; queue depth and presentation timestamps are visible | Reject that tuple from `Auto`. A codec-wide API or Mac model name is insufficient. Decide whether native player work is required from this evidence. |
| **G4 — Codec identity** | Accepted tuple, first codec packet, bitstream profile/chroma, decoder output, and on-screen label agree | Abort the session, invalidate cached evidence, and re-probe. Never substitute in place. |
| **G5 — 4K60 acceptance** | Thirty-minute mixed-motion run presents at least 59 fps in every rolling ten-second window, drops no more than 0.1%, keeps queue depth at most one, meets documented p95/p99 stage budgets, and includes an external input-to-photon audit | No 4K60 viability claim. Fix the identified stage before adding breadth or higher-refresh goals. |
| **G6 — Platform claims** | Each named distro/macOS row has its own build, launch, connection, video, input, clipboard, security, and probe evidence | Claim only the rows that passed; one Arch host or one Mac generation does not establish the matrix. |

## Key Findings

### Recommended Stack

The stack research strongly favors preserving Kyber's coordinated multimedia graph instead of upgrading or replacing individual parts. The exact recursive tree, source hashes, toolchains, runtime hardware state, and local patches are part of every benchmark result. System FFmpeg/VLC, moving Rust, floating submodules, bundled NVIDIA libraries, and runtime containers would make failures ambiguous.

**Core technologies:**

- **Kyber Desktop 0.27.0** at `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f`: superproject, macOS client, and closest working foundation.
- **Kyber SDK and exact recursive gitlinks:** preserve the inspected control, media, multiplexing, and input graph; verify it in `UPSTREAM.lock` and CI.
- **Kymux** at `831d3120bea004505dc69426eebbb9b07f10e443`: retain the existing QUIC/TLS endpoints and framing; do not add capability policy to the data plane.
- **Kymedia/txproto** at `e80eb6bb347ed0e378ae46a86f67ada2aa6079da` / `82694c38fb7d662ad364071382166edf8731db93`: retain NvFBC, GPU conversion, FFmpeg, and encoded packet flow.
- **FFmpeg n8.1 plus Kyber's 15 patches** and **`nv-codec-headers` n12.1.14.0:** use the tested NVENC plumbing rather than system packages or SDK 13.1 churn.
- **Kyber VLC fork** at `dd2db54794384591684c1c86ce70eb64eb9eab15`: first playback and diagnostic baseline, instrumented to disclose decoder, surface format, queue, and present behavior.
- **NvFBC shared-CUDA plus NVENC:** required host path; certify NVIDIA display driver `>=570.86.16` with exact kernel/userspace match and never bundle driver libraries.
- **VideoToolbox/CoreMedia/CoreVideo and, conditionally, Metal/QuartzCore:** exact decode probes are mandatory; native rendering is a measurement-triggered adapter, not a prerequisite rewrite.
- **CoreGraphics event taps and AppKit NSPasteboard:** narrow native macOS bridges for immersive input and text/HTML clipboard.
- **Rust 1.89.0, Cargo lockfiles, Meson 1.10.0:** pin the upstream build toolchain and build with locked inputs.
- **Xcode 26.6 release lane, Xcode 27 beta 4 compatibility lane, macOS deployment target 15.0, arm64 only:** keep beta qualification separate from shipping.
- **Native distro qualification artifacts:** Arch/CachyOS, Ubuntu 24.04, and Rocky 9 builds; containers may build but must not be the latency runtime.

Critical version detail lives in [STACK.md](./STACK.md); the roadmap should create `UPSTREAM.lock`, `PATCHES.md`, build manifests, source-offer/license materials, and an SPDX SBOM before benchmark evidence is considered reproducible.

### Expected Features

**Must have (prototype table stakes):**

- Secure direct hostname/IP connection with distinct resolve, timeout, refusal, TLS, authentication, probe, and session failures.
- Host and client preflight that blocks unsupported session, driver, capture, encoder, decoder, permission, renderer, and display states before a black window.
- One explicitly selected physical X11 output through measured NvFBC/GPU conversion/NVENC; no software encoder or automatic XCB fallback.
- Atomic `Auto` and manual media tuples, not independent codec/chroma toggles.
- Exact decoder-path proof and actual-path reporting; `Auto` considers only hardware-proven tuples.
- Sustained 3840×2160@60 with stage telemetry, queue/drop evidence, exported JSONL/report, and external latency validation.
- Correct SDR presentation, aspect/input-coordinate mapping, reported range/matrix/chroma, and no implied HDR support.
- Immersive relative and absolute mouse plus eligible keyboard forwarding, permission preflight, release chord, focus/disconnect cleanup, and an empirically tested shortcut matrix.
- Authenticated, loop-free bidirectional X11 `CLIPBOARD` and NSPasteboard synchronization for UTF-8 text and HTML within the resolved explicit limit.
- Deterministic teardown, stuck-input prevention, clean failure/recovery behavior, and compatibility evidence tied to exact host/client builds.

**Should have (differentiators):**

- Evidence-backed negotiation showing every candidate and rejection reason.
- Manual HEVC 4:4:4 fidelity mode with SPS/output-plane verification and a one-pixel chroma stress fixture.
- Honest AV1 4:2:0 experimentation that stays unavailable until the full host-to-present path passes.
- Per-frame latency provenance and bounded-queue/drop classification instead of a single average number.
- Reproducible proof bundles and a truthful, row-by-row compatibility matrix.

**Defer until after the core proof:**

- Faithful Wacom pressure/tilt/eraser/tool/pad transport.
- 1440p120 and 4K120 gates; AV1 promotion into `Auto`.
- Audio, Wayland, virtual/headless displays, HDR/10-bit acceptance, adaptive quality, multi-monitor UX, file/image clipboard, and automatic reconnect.
- Discovery, relay/NAT traversal, accounts, fleet management, polished UI, installers/updaters, and additional host/client platforms.

**Never offer:** AV1 4:4:4, silent codec/chroma substitution, silent software encode/decode, unauthenticated clipboard/input, or an acceptance result produced by lowering the requested raster/fps.

### Architecture Approach

Keep policy and evidence outside the media transport. The authenticated control/session plane exchanges a bounded, versioned capability contract, a pure selector intersects exact host/client tuples, and the host revalidates before allocating media endpoints. Kymux continues to carry existing typed video, input, clipboard, and low-priority metrics traffic. New platform behavior lives behind adapters, leaving the known-good Kyber path available for differential diagnosis.

**Major components:**

1. **Kyber controller/session orchestrator** — identity, authentication, direct session lifecycle, capability response, validation, and child-process cleanup.
2. **HostProbe** — actual Xorg/NvFBC/NVENC/display/permission probes and short-lived evidence keyed to GPU, driver, and display state.
3. **Kyber media service and txproto** — capture, GPU conversion, NVENC configuration, and encoded `AVPacket` production.
4. **Kymux data plane** — unchanged authenticated QUIC/TLS endpoints, packet sequencing, and IPC bridge.
5. **Client orchestrator and pure `ModeSelector`** — exact client probe, deterministic tuple intersection/ranking, structured rejection, session start, and re-negotiation.
6. **Player adapter** — pinned libVLC diagnostic implementation first; conditional native VideoToolbox/IOSurface/Metal implementation behind the same boundary.
7. **MacInputAdapter and existing Kynput service** — event-tap/fullscreen state machine, canonical coordinate transform, reliable transport, and Linux `/dev/uinput` injection.
8. **MacClipboardAdapter and X11 clipboard service** — authenticated text/HTML lifecycle, origin/hash/sequence dedupe, explicit size policy, and no binary transfer.
9. **TelemetryAggregator** — non-blocking per-frame and input/clipboard correlation, clock-offset uncertainty, raw JSONL, percentiles, and acceptance reports.

**Key patterns:**

- Represent codec, chroma, bit depth, profile, resolution, fps, encoder path, and decoder path as one versioned tuple with evidence.
- Cache probes only against exact hardware, driver, OS, display, app, and bitstream identities; revalidate at session creation.
- Treat manual selection as strict. `Auto` may move to a lower ranked, already-proven hardware tuple only through visible teardown and re-negotiation.
- Bound presentation and telemetry queues; telemetry may drop and count itself but must never backpressure video or input.
- Keep one logical patch per owned seam and retain the baseline for differential testing.
- Authenticate before returning detailed capabilities or opening input/clipboard channels; validate all peer lengths, codec parameters, HTML, and reason strings.

See [ARCHITECTURE.md](./ARCHITECTURE.md) for component boundaries, data flows, failure behavior, and test seams.

### Critical Pitfalls

1. **Measuring a fallback path as hardware accelerated** — require one-frame exact probes and runtime evidence for capture backend, copy boundaries, encoder, decoder, surfaces, and renderer; fail unapproved fallback.
2. **Benchmarking Wayland or a mismatched NVIDIA stack** — make the host-doctor gate a prerequisite, not an error discovered during tuning.
3. **Treating codec and chroma independently** — negotiate only exact tuples; reject AV1 4:4:4 before allocation and never silently alter a manual choice.
4. **Optimizing averages while hiding queues and tails** — correlate monotonic stage timestamps, p50/p95/p99, drops, clock uncertainty, and presentation; run the full mixed-motion soak.
5. **Assuming a Mac model or codec-wide API proves hardware decode** — decode the exact negotiated bitstream with hardware required and inspect the actual session property.
6. **Shipping prototype security defaults** — generate per-host identity/credential material, verify pins/private CA, remove skip-verification and sample secrets, and gate control-bearing channels on authentication.
7. **Calling input/clipboard complete without state and abuse tests** — preflight permissions, release held state on every lifecycle transition, dedupe clipboard origins, bound payloads, and keep clipboard content out of logs.
8. **Forking or tuning too broadly** — preserve Kymux/framing and the multimedia baseline; postpone transport mode tuning and dependency upgrades until measured evidence identifies the need.

## Implications for Roadmap

The roadmap should contain eight implementation phases plus a non-negotiable execution prerequisite. Each phase must leave a runnable vertical slice and archive evidence for its gate.

### Execution Prerequisite: Repair and Prove the Host

**Rationale:** The current Wayland session and NVIDIA kernel/userspace mismatch make all capture/encode results invalid.
**Delivers:** A repeatable host-doctor result on real Xorg with matching driver libraries, physical-output identity, NvFBC shared-CUDA creation, exact H.264/HEVC NVENC probes, `/dev/uinput`, and render access.
**Gate:** G0 must pass before the roadmap treats any media result as evidence.

### Phase 1: Pinned, Secure Kyber Baseline

**Rationale:** Reproducibility and authenticated control are dependencies of every later measurement and feature.
**Delivers:** Exact recursive Kyber checkout, locked toolchains, `UPSTREAM.lock`, patch ledger, unique host identity/credential, direct hostname/IP connection, arm64 macOS package, and upstream H.264 4:2:0/libVLC pixels plus basic input over the real network.
**Addresses:** Direct connection, authenticated transport, reproducible proof bundle.
**Avoids:** Moving dependencies, test certificates, trust-all modes, and premature media rewrites.
**Gate:** G1.

### Phase 2: Instrumented NVIDIA 4K60 Baseline

**Rationale:** Prove the existing capture/encode/transport path before changing codecs or the player.
**Delivers:** Mandatory X11/NvFBC/NVENC preflight, copy/conversion evidence, correlated Kyber stage telemetry, bounded queues, H.264 4:2:0 reliable-transport smoke and soak reports, and explicit failure for XCB/software paths.
**Addresses:** Physical capture, NVIDIA encode, actionable diagnostics, first 4K60 evidence.
**Avoids:** Hidden CPU conversion, unbounded queues, static-demo bias, and aggregate-only latency.
**Gate:** G2; collect a ten-minute engineering run here, with final G5 reserved for Phase 5.

### Phase 3: Versioned Capability Contract and Strict Selector

**Rationale:** Runtime evidence must constrain the menu before additional tuples multiply failure modes.
**Delivers:** `MediaCapabilitiesV1`, `MediaTuple`, host/client probe IDs, pure exhaustive `ModeSelector`, structured stable errors, stale-probe handling, host revalidation, and first-packet/bitstream conformance checks. Advertise only already-proven H.264 4:2:0 initially.
**Addresses:** Evidence-backed negotiation, `Auto`, exact manual modes, actionable unsupported-mode failures.
**Avoids:** Independent codec/chroma flags, late encoder failure, silent fallback, and unversioned schema ambiguity.
**Gate:** Impossible tuples—including AV1 4:4:4—fail deterministically before endpoint allocation.

### Phase 4: macOS Decoder/Renderer Evidence and Player Decision

**Rationale:** The research disagrees only on when to replace libVLC, not on the need for an observable adapter seam and exact client evidence.
**Delivers:** Player interface, exact H.264 sample probe, hardware-required VideoToolbox verification, libVLC decoder/output/queue/present instrumentation, recorded packet fixtures, and a written keep/replace decision. If libVLC fails the evidence or latency/copy gate, implement the narrow native VideoToolbox/IOSurface/Metal H.264 adapter and retain libVLC as a diagnostic feature.
**Addresses:** Runtime decoder proof, presentation identity, decode/render telemetry.
**Avoids:** A speculative player rewrite, unproven hardware claims, CPU frame copies, and VLC patch sprawl.
**Gate:** G3 for H.264 4:2:0 and 4K60 parity with the Phase 2 baseline.

### Phase 5: Negotiated Latency and Fidelity Matrix

**Rationale:** Add formats only after one complete, measurable tuple and strict selection behavior are stable.
**Delivers:** HEVC 4:2:0 hardware latency candidate, H.264 4:2:0 fallback, manual HEVC 4:4:4 fidelity mode with actual hardware/software label, and AV1 4:2:0 experimental mode only where an Ada-or-newer host and exact client playback path pass. Includes bitstream/profile/chroma verification, one-pixel chroma fixture, mixed-motion workload, external latency audit, and final thirty-minute report.
**Addresses:** Fully hardware `Auto`, HEVC fidelity, honest AV1, sustained 4K60.
**Avoids:** Conflating fidelity with latency, Apple model-name assumptions, software fallback, AV1 4:4:4, and results combined across changed configurations.
**Gates:** G3–G5. A measured software HEVC 4:4:4 run may pass the fidelity identity check but cannot pass the hardware latency gate.

### Phase 6: Native Immersive Input

**Rationale:** Build on the now-stable authenticated session and telemetry identifiers without mixing input lifecycle bugs into media bring-up.
**Delivers:** CoreGraphics/NSEvent permission preflight, focused/fullscreen/suspended state machine, relative and absolute mouse, canonical display transform, buttons/wheel, physical keys/modifiers/repeat, release chord, release-all cleanup, latency telemetry, and macOS 15/current-beta shortcut matrix.
**Addresses:** Immersive mouse and eligible keyboard control.
**Avoids:** Equating fullscreen with keyboard capture, stuck modifiers/buttons, pointer trapping, and unsupported shortcut claims.
**Gate:** Permission, focus-loss, disconnect, sleep, modifier, repeat, pointer-lock, and release-path tests pass.

### Phase 7: Authenticated Bidirectional Clipboard

**Rationale:** Clipboard reuses the trusted input endpoint but needs its own bounded state machine and abuse tests.
**Delivers:** NSPasteboard and X11 `CLIPBOARD` text/HTML adapters, explicit per-session enablement, origin/hash/sequence suppression, 60 KiB initial rejection limit, authentication lifecycle, and rapid/malformed/oversize/reconnect tests. A later 1 MiB extension requires a versioned, independently gated contract.
**Addresses:** Bidirectional text/HTML copy/paste without loops or binary scope.
**Avoids:** Clipboard before authentication, content in logs, echo storms, busy polling, silent truncation, and arbitrary MIME transfer.
**Gate:** Both directions work within the limit; rejected updates leave the destination unchanged and never impair input/video.

### Phase 8: Compatibility, Packaging, and Fault Soak

**Rationale:** Broader claims become meaningful only after the reference vertical slice passes.
**Delivers:** Arch/CachyOS, Ubuntu 24.04, and Rocky 9 build/runtime rows; macOS 15 and current macOS 27-beta lanes; app signing/notarization checks as applicable; source/license/SBOM bundle; clean launch/preflight; network impairment and recovery matrix; security/fault injection; repeatable final acceptance archive.
**Addresses:** Explicit compatibility matrix and internal transferability without product-control-plane scope.
**Avoids:** Calling one successful demo universal support, weakening stable behavior for a beta, and selecting lossy transport by name.
**Gate:** G6. Reliable Kymux remains default unless a candidate mode wins p95/p99 and recovery tests without moving input/clipboard off reliable ordering.

### Phase Ordering Rationale

- Host readiness, immutable inputs, and security precede measurement because invalid hardware state, moving code, or unauthenticated control makes every downstream result unusable.
- Existing H.264/libVLC provides a differential baseline before capability contracts, player changes, or new codecs.
- Instrumentation precedes optimization and format expansion so each regression is attributable to one stage.
- The selector precedes the codec matrix so impossible pairs never become runtime experiments.
- The player decision is evidence-driven; it preserves the smallest patch until a concrete decoder, copy, queue, presentation, or AV1 limitation requires native code.
- Input and clipboard follow the stable authenticated data plane but precede compatibility claims because they are part of the core value.
- Packaging and transport tuning follow the reference proof to avoid multiplying environment variables before viability is known.

### Research Flags

Phases likely needing deeper research during planning:

- **Execution prerequisite / Phase 2:** NvFBC-to-NVENC surface ownership, copy boundaries, cursor behavior, and driver-specific capture behavior require live host investigation.
- **Phase 4:** Direct endpoint ownership, codec parameter-set conversion, libVLC observability, VideoToolbox session properties, IOSurface formats, and Metal presentation semantics need a focused macOS player spike.
- **Phase 5:** Exact HEVC 4:4:4 behavior on each target Apple Silicon generation and AV1 4:2:0 through the packaged client require real-device experiments; public capability tables are insufficient.
- **Phase 6:** macOS 15 and macOS 27-beta event-tap permissions and OS-reserved shortcut behavior require empirical research.
- **Phase 8:** Rocky runtime integration and current beta signing/permission regressions need platform-specific qualification.

Phases with well-documented patterns (skip a separate research phase):

- **Phase 1:** Exact git pinning, certificate/private-CA verification, build manifests, and locked-toolchain practices are established.
- **Phase 3:** Versioned bounded schemas, pure tuple selection, typed errors, and exhaustive contract tests are standard and well specified by the architecture research.
- **Phase 7:** Pasteboard/selection observation and origin/hash dedupe are established patterns; use an implementation spike only if expanding beyond the recommended 60 KiB upstream limit.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Exact Kyber pins and NVIDIA capability limits were inspected from primary sources; macOS tuple support, VLC behavior, native player need, and current hardware performance remain unproven. |
| Features | MEDIUM | Scope and observable contracts are coherent and benchmarked against mature products, but acceptance budgets and some compatibility expectations are project hypotheses until live runs pass. |
| Architecture | HIGH for upstream / MEDIUM for additions | Pinned Kyber boundaries and data paths were inspected directly. Proposed capability, telemetry, and adapter seams are strong; the native player and exact platform behavior require spikes. |
| Pitfalls | HIGH for known constraints / MEDIUM for thresholds | Kyber/NVIDIA constraints and common failure modes are well supported. Apple decode, input, beta behavior, and actual latency/thermal thresholds need target hardware. |

**Overall confidence:** MEDIUM

### Gaps to Address

- **Current host is not benchmark-ready:** repair the kernel/userspace mismatch and use real Xorg before any live capture claim.
- **Zero/minimal-copy is unverified:** account for every NvFBC, conversion, encoder, decode, and presentation boundary with runtime surface evidence.
- **HEVC 4:4:4 on Apple Silicon is unknown by tuple:** run the exact Kyber bitstream; label hardware or software from the actual session and do not extrapolate across generations.
- **AV1 4:2:0 is unproven end to end:** Ada encode support and M3-or-newer marketing are only candidate filters; the packaged player and exact bitstream must pass. AV1 4:4:4 remains unsupported regardless.
- **Player implementation is intentionally unresolved:** instrumentation decides whether pinned libVLC remains adequate or a native adapter is required. Do not commit the roadmap to a wholesale rewrite before this gate.
- **Clipboard research specified conflicting limits:** the roadmap recommendation is 60 KiB reject-without-truncation for v1; a 1 MiB requirement must be explicitly re-approved and versioned after load/abuse testing.
- **macOS shortcut and beta behavior is empirical:** build a per-version matrix and describe reserved actions honestly.
- **Performance budgets are proposed, not achieved:** calibrate thresholds only through archived runs; never weaken them silently or combine runs with different tuples/settings.
- **Compatibility is not transitive:** real NVIDIA/X11 hardware smoke tests are still required beyond container builds, and at least an older Apple Silicon Mac plus an M3-or-newer Mac are needed for generation claims.
- **Legal/package implementation needs review:** AGPL source obligations are accepted for the prototype, but any proprietary distribution and codec licensing need a fresh decision.

## Sources

The conclusions above synthesize [STACK.md](./STACK.md), [FEATURES.md](./FEATURES.md), [ARCHITECTURE.md](./ARCHITECTURE.md), and [PITFALLS.md](./PITFALLS.md). Source links establish API and upstream facts; they do not replace the runtime gates.

### Primary (HIGH confidence)

- [Kyber Desktop 0.27.0 pinned tree](https://gitlab.com/kyber/apps/kyber-desktop/-/tree/6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f) — release, build, package, and superproject structure.
- [Kyber control tree](https://gitlab.com/kyber/core/kyctl/-/tree/e46dda2825d6e8c432521cfcb4c5726b5bba8bc4) — controller/client session boundaries.
- [Kyber media tree](https://gitlab.com/kyber/core/kymedia/-/tree/e80eb6bb347ed0e378ae46a86f67ada2aa6079da) and [Kymux tree](https://gitlab.com/kyber/core/kymux/-/tree/831d3120bea004505dc69426eebbb9b07f10e443) — media graph, endpoint model, and transport.
- [Kyber input tree](https://gitlab.com/kyber/core/kynput/-/tree/5595478f3606c5624197360e2dcbf31bd60e8163) — canonical input and X11 clipboard foundation.
- [NVIDIA NVENC Application Note](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-application-note/index.html) — generation/profile/chroma support, including HEVC 4:4:4 and Ada AV1 Main 4:2:0.
- [NVIDIA NVENC Programming Guide](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-video-encoder-api-prog-guide/index.html) — capability queries and API behavior.
- [NVIDIA Capture SDK](https://developer.nvidia.com/capture-sdk) — NvFBC and driver requirements.
- [Apple VideoToolbox hardware decode APIs](https://developer.apple.com/documentation/videotoolbox/vtishardwaredecodesupported%28_%3A%29) — coarse support query; research also uses Apple's require-hardware and actual-decoder session properties.
- [Apple M3 announcement](https://www.apple.com/newsroom/2023/10/apple-unveils-m3-m3-pro-and-m3-max-the-most-advanced-chips-for-a-personal-computer/) — first advertised Apple Silicon AV1 decode, used only as a candidate filter.
- [Apple CGEvent](https://developer.apple.com/documentation/coregraphics/cgevent) and [NSPasteboard changeCount](https://developer.apple.com/documentation/appkit/nspasteboard/changecount) — native input and clipboard APIs.
- [Linux uinput documentation](https://docs.kernel.org/input/uinput.html) — host input injection.
- [RFC 9001](https://www.rfc-editor.org/rfc/rfc9001.html) and [RFC 9525](https://www.rfc-editor.org/rfc/rfc9525.html) — QUIC/TLS security and service identity.
- [GNU AGPLv3](https://www.gnu.org/licenses/agpl-3.0.html) — license text; implementation-specific legal review remains separate.

### Secondary (MEDIUM confidence)

- [Moonlight performance definitions](https://github.com/moonlight-stream/moonlight-docs/wiki/Frequently-Asked-Questions) and [Moonlight client baseline](https://github.com/moonlight-stream/moonlight-qt) — comparison for latency reporting and mature remote-input expectations.
- [Parsec stream statistics](https://support.parsec.app/hc/en-us/articles/32381603663636-Stream-Overlay-Stats-and-Logging) and [Parsec compatibility](https://support.parsec.app/hc/en-us/articles/32381568346644-Hardware-and-Software-Compatibility) — comparison for diagnostic and 4:4:4 positioning.

### Unresolved Runtime Evidence

- Target-host NvFBC/NVENC probes and 4K60 reports.
- Exact Apple Silicon HEVC 4:4:4 and AV1 4:2:0 decode/session evidence.
- libVLC versus native player copy, queue, and presentation measurements.
- macOS 15/current-beta input, clipboard, signing, and compatibility runs.

---
*Research completed: 2026-07-26*
*Ready for roadmap: yes, subject to execution prerequisite G0*
