# Pitfalls Research

**Domain:** Ultra-low-latency Linux-to-macOS remote desktop over Kyber/Kymux
**Researched:** 2026-07-26
**Confidence:** HIGH for current Kyber/NVIDIA limits; MEDIUM where an actual
Apple Silicon client or corrected X11 host must be probed

## Critical Pitfalls

### Pitfall 1: Measuring a fallback path and calling it hardware accelerated

**What goes wrong:**
The demo renders video, but capture copied through system memory, NVENC failed
over to software, or macOS decoded in software. Latency, CPU use, thermals, and
frame pacing then collapse under a real 4K60 soak.

**Why it happens:**
API availability and encoder names are mistaken for runtime use. FFmpeg can
list `*_nvenc` on an unsupported GPU, VideoToolbox session creation can accept
software fallback, and an Xwayland image is not the physical X11 scanout.

**How to avoid:**
Make capture backend, copy count, encoder GUID/profile/pixel format, decoder
implementation, output surface type, and renderer visible in the session
handshake and telemetry. Require NVENC capability queries and
`kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder`; verify
`UsingHardwareAcceleratedVideoDecoder`. Fail the performance gate if any
unapproved fallback occurs.

**Warning signs:**
High CPU, unexplained format-conversion time, `software_decode`, missing
IOSurface/CUDA surface identifiers, FFmpeg encoder names without a successful
one-frame probe, or VideoToolbox logs that never confirm hardware use.

**Phase to address:**
Foundation/capability-probe phase, before the first network stream.

---

### Pitfall 2: Treating codec and chroma as independent switches

**What goes wrong:**
The client requests AV1 4:4:4 from an RTX 4090, HEVC 4:4:4 from hardware that
cannot encode it, or a profile the Mac cannot decode in hardware. Encoder
initialization fails late or the client silently changes modes.

**Why it happens:**
Upstream Kyber currently exposes codec and `--444` separately and does not
negotiate a complete codec/profile/chroma/bit-depth tuple.

**How to avoid:**
Represent modes as validated tuples. The host advertises queried NVENC output
tuples; the client advertises bitstream-probed VideoToolbox tuples; selection is
the intersection. Ada AV1 is 4:2:0-only. HEVC 4:4:4 remains a manual fidelity
mode until the actual Mac proves its decode path. Unsupported manual selections
return a stable, actionable error without fallback.

**Warning signs:**
Independent `codec` and `444` booleans, profile choice after session start,
generic “encoder failed,” or negotiated settings differing from requested
settings without user-visible consent.

**Phase to address:**
Codec contract phase before optimization.

---

### Pitfall 3: Optimizing average latency while hiding tail latency

**What goes wrong:**
A short demo feels fast, but input or frames periodically stall. Mean latency
looks excellent while p95/p99, frame drops, queue depth, and recovery are poor.

**Why it happens:**
One end-to-end timer cannot identify capture, conversion, encode, network,
decode, render, display, or input queues. Unsynchronized clocks and host-monitor
observation can even create apparent “negative latency.”

**How to avoid:**
Timestamp every boundary with monotonic clocks, estimate clock offset, preserve
frame/session IDs, and report p50/p95/p99 plus maximums, drops, duplicates,
queue depth, bitrate, RTT, congestion, and recovery. Run a 30-minute 4K60 soak
with motion, text, packet impairment, clipboard activity, and input. Use
high-speed camera or photodiode/LED validation for externally observed
input-to-photon numbers.

**Warning signs:**
Only FPS and average RTT are reported; telemetry stops at decode instead of
presentation; no frame IDs; numbers are derived by comparing two unsynchronized
screens; or test duration is under a few minutes.

**Phase to address:**
Telemetry phase, before claiming protocol viability.

---

### Pitfall 4: Benchmarking on the wrong Linux graphics session

**What goes wrong:**
The prototype captures only Xwayland applications, sees no NVIDIA X screen, or
uses a slow copy path while the team believes it is testing NvFBC/X11.

**Why it happens:**
`DISPLAY` can exist inside a Wayland session. That does not make the physical
desktop an Xorg desktop. Driver/library mismatches can also leave CUDA partly
functional while NVML and NVENC fail.

**How to avoid:**
Gate startup on an actual Xorg session, a matching loaded NVIDIA
kernel/userspace stack, successful NVML and one-frame NVENC probes, an NVIDIA
X screen/provider, working `/dev/uinput`, and a proven capture backend. Record
the checks in a repeatable host doctor command. Reboot the current development
host into a matching installed kernel and X11 before live proof work.

**Warning signs:**
`XDG_SESSION_TYPE=wayland`, `kwin_wayland`, zero XRandR providers, NVML
driver/library mismatch, or CUDA enumeration succeeding while
`OpenEncodeSessionEx` fails.

**Phase to address:**
Host readiness phase zero.

---

### Pitfall 5: Assuming Kymux mode names guarantee lowest latency

**What goes wrong:**
An unreliable or FEC transport is selected because it sounds faster, but loss,
reordering, congestion, or intra-refresh behavior causes worse tail latency and
visual recovery than the reliable baseline.

**Why it happens:**
Transport mode, codec recovery, bitrate control, GOP/intra-refresh, and network
conditions are coupled. A clean LAN test does not exercise the failure path.

**How to avoid:**
Begin with Kymux reliable and zero configured video buffer. Benchmark reliable,
gopstream, unreliable, and unreliable-FEC under controlled latency, jitter,
loss, reordering, and bandwidth changes. Keep control, input, authentication,
and clipboard on reliable ordered channels. Promote a transport mode only when
its p95/p99 and recovery beat the baseline.

**Warning signs:**
Mode chosen by name, no impairment matrix, no congestion telemetry, no visual
recovery metric, or input sharing a lossy channel with video.

**Phase to address:**
Network tuning phase after the deterministic 4K60 baseline.

---

### Pitfall 6: Calling macOS input “immersive” without testing OS boundaries

**What goes wrong:**
Pointer lock works, but Command-Tab, system gestures, key-up events, focus
transitions, or accessibility permissions leave keys local or stuck remotely.

**Why it happens:**
Upstream Kyber's pinned macOS window layer reports keyboard grab as unsupported.
Native event taps require Accessibility permission and macOS intentionally
reserves some secure/system actions.

**How to avoid:**
Implement the macOS capture layer explicitly with permission preflight, a clear
immersive-state indicator, focus-loss flush, deterministic escape chord, key
repeat and modifier-state tests, and a documented list of OS-reserved
shortcuts. Never claim interception macOS does not permit.

**Warning signs:**
Fullscreen is equated with keyboard capture, no Accessibility preflight,
stuck modifiers after Command-Tab, or no emergency release chord.

**Phase to address:**
Input/clipboard vertical slice.

---

### Pitfall 7: Creating clipboard feedback loops or leaking data

**What goes wrong:**
The same clipboard value bounces indefinitely, rapid updates starve input, rich
content expands unexpectedly, or an unauthenticated peer reads sensitive local
clipboard contents.

**Why it happens:**
Upstream Linux X11 clipboard support exists, but the macOS NSPasteboard bridge
does not. Bidirectional state needs origin/version suppression and policy, not
just two polling loops.

**How to avoid:**
Add message IDs, origin, monotonic version, MIME allowlist, size limit,
deduplication, bounded polling/backoff, and explicit session enablement. Scope
the prototype to UTF-8 text and sanitized HTML. Start clipboard only after the
authenticated session is established and stop/clear session state on
disconnect.

**Warning signs:**
Repeated identical clipboard packets, pasteboard change count driving a busy
loop, arbitrary binary MIME transfer, no size cap, or clipboard starting before
authentication completes.

**Phase to address:**
Input/clipboard vertical slice with loop and abuse tests.

---

### Pitfall 8: Shipping Kyber test security defaults

**What goes wrong:**
Anyone with network reachability can control or observe the desktop, or a
client accepts a man-in-the-middle because TLS verification was skipped.

**Why it happens:**
Direct-IP prototypes are often treated as “trusted LAN” demos. Upstream
supports test certificates, default credentials, skip-verification options,
and multiple trust modes that are safe only when configured deliberately.

**How to avoid:**
Generate per-host credentials and certificates, pin the certificate or use an
explicit trust-on-first-use store with visible fingerprint confirmation, bind
only intended interfaces, require authentication before data-plane setup, and
never package test secrets. Add replay, downgrade, session-ID, and failed-login
tests.

**Warning signs:**
Bundled private keys, default passwords, `--tls-skip-verification`, wildcard
listening without firewall documentation, or media/input streams opening before
authentication.

**Phase to address:**
Foundation/security phase; block release on high-severity failures.

---

### Pitfall 9: Forking too much of Kyber too early

**What goes wrong:**
Local changes spread into Kymux, scheduling, FFmpeg, VLC, and build scripts,
making upstream updates, debugging, and license accounting unmanageable before
the protocol is even proven.

**Why it happens:**
Cross-cutting prototype features are easiest to hack directly into whichever
module currently has the data, and upstream boundaries are still evolving.

**How to avoid:**
Pin the exact 0.27.0 superproject and gitlinks. Keep patches narrow: control
contract/capability selection, macOS player boundary, macOS input/clipboard,
telemetry, and packaging wrappers. Preserve upstream Kymux framing and media
scheduling. Maintain a patch ledger, upstream-base manifest, and automated
rebase/build smoke test.

**Warning signs:**
Unpinned submodules, edits across every Kyber core repository, protocol changes
without a version field, or builds that depend on mutable `main`.

**Phase to address:**
Repository/vendor foundation and every dependency update.

---

### Pitfall 10: Confusing a successful demo with supported platforms

**What goes wrong:**
The Arch development machine works, while Ubuntu/Rocky packaging or macOS 27
beta fails. A new beta breaks permissions, event taps, VideoToolbox, or
packaging after the prototype was declared complete.

**Why it happens:**
Upstream documents Debian/Ubuntu and Arch but not Rocky. Apple beta APIs and
security behavior can change, and one Apple Silicon generation does not
represent all arm64 Macs.

**How to avoid:**
Use reproducible CI/build containers for Arch-, Ubuntu-, and Rocky-family
dependencies; keep runtime hardware tests separate. Set the deployment target
to macOS 15, compile/test with stable and current beta Xcode, and run capability
probes on at least an older Apple Silicon Mac plus an M3-or-newer AV1-capable
Mac. Treat beta regressions as explicit compatibility findings, not reasons to
weaken stable behavior.

**Warning signs:**
Only the developer machine builds, package instructions require untracked
manual state, `main`-branch dependencies, or AV1 tests run on just one Mac
generation.

**Phase to address:**
Packaging/compatibility phase after the core vertical slice.

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Patch upstream `main` instead of pinned 0.27.0 | Fast access to fixes | Non-reproducible results and moving failures | Never for benchmark evidence |
| Use software decode for HEVC 4:4:4 | Fidelity demo may render | Misrepresents latency and thermal viability | Diagnostic/manual fidelity mode only, clearly labeled |
| Report one aggregate latency timer | Simple dashboard | Cannot identify queues or regressions | Early smoke test only |
| Poll NSPasteboard without origin IDs | Quick clipboard demo | Feedback loops and lost updates | Never |
| Run host with broad root permissions | Avoid uinput/DRM setup | Unsafe and hides packaging requirements | Never; use narrow ACL/udev policy |
| Add Wacom packets during core proof | Attractive demo feature | New capture, protocol, virtual-device, and app-compat matrix | Only after core streaming requirements pass |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| NVENC | Trusting FFmpeg encoder enumeration | Query NVENC caps and successfully encode the exact tuple |
| NvFBC/X11 | Treating Xwayland as Xorg | Require actual Xorg and prove the selected capture backend |
| VideoToolbox | Allowing implicit software fallback | Require hardware decode for performance modes and inspect the session property |
| Metal presentation | Copying decoded frames through CPU memory | Keep IOSurface/CVPixelBuffer-backed surfaces and measure present/display time |
| Kymux/QUIC | Tuning loss mode before a baseline | Establish reliable zero-buffer baseline, then use netem-style impairment tests |
| macOS event tap | Requesting permission after fullscreen capture | Preflight/explain Accessibility access and retain a safe escape path |
| NSPasteboard/X11 | Mirroring every observed update | Carry origin/version IDs, dedupe, MIME/size policy, and disconnect state |
| Kyber auth/TLS | Reusing sample certificates | Generate per-host identity and explicitly verify/pin it |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Hidden RGB→YUV CPU conversion | High CPU and encode jitter | GPU-native conversion with per-stage telemetry | 4K60 motion/content changes |
| Unbounded queues | Smooth video several seconds behind | Queue depth of one/latest-frame policy and drop metrics | Network or decoder slowdown |
| B-frames/lookahead/default quality preset | Excellent compression but sluggish input | ULL preset, no B-frames/lookahead, measure every encoder option | Interactive use immediately |
| Testing static desktop only | Great bitrate/FPS, motion artifacts later | Motion/text/scroll/cursor workload corpus | First real creative or 3D workload |
| Display-rate mismatch | Judder or periodic bursts | Pace against captured timestamps and client display, report cadence | 59.94/60/120 Hz combinations |
| Software HEVC 4:4:4 on Mac | Thermal rise and frame drops | Hardware-probe gate; fallback to hardware 4:2:0 for performance | Commonly at 4K60; exact threshold is a required spike |
| Measuring decode completion as photon time | Unrealistically low numbers | Track Metal submit/present and externally validate | Every end-to-end claim |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Test certificate/default credentials | Unauthorized desktop control | Per-host identity, strong credential, secure storage, no packaged secrets |
| TLS skip verification | Man-in-the-middle video/input compromise | Pin/TOFU fingerprint with explicit confirmation |
| Clipboard before authentication | Sensitive data disclosure | Start bridge only inside authenticated session |
| Unlimited clipboard payload | Memory/CPU denial of service | MIME allowlist, 60 KiB-class explicit cap, bounded parsing |
| Input remains active after focus/disconnect | Remote unintended actions | Session/focus state machine and flush all pressed inputs |
| Unversioned capability fields | Downgrade or parser ambiguity | Versioned, bounded control-plane schema and reject unknown critical values |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `Auto` silently changes codec/chroma | User cannot trust quality or diagnose latency | Show requested, negotiated, and actual runtime tuple |
| HEVC 4:4:4 offered as universally “fast” | Mac may software-decode and stutter | Label fidelity mode and display hardware/software result before starting |
| Fullscreen without reliable release | User feels trapped | Visible immersive state and tested emergency escape chord |
| Generic connection failure | Setup becomes guesswork | Stable errors for reachability, TLS, auth, encoder, decoder, capture, and permissions |
| Hidden host readiness problems | User blames protocol | `replay-host doctor` with actionable kernel/session/GPU/input checks |
| Clipboard mirroring without status | Sensitive clipboard surprises | Explicit per-session toggle and transfer indicator |

## "Looks Done But Isn't" Checklist

- [ ] **4K60:** Verify a 30-minute mixed-motion soak, not a short static demo.
- [ ] **Hardware encode:** Verify the exact NVENC codec/profile/chroma tuple at runtime.
- [ ] **Hardware decode:** Require and confirm VideoToolbox hardware use on each tested Mac generation.
- [ ] **Zero/minimal copy:** Account for every capture, conversion, decode, and presentation surface transition.
- [ ] **Latency:** Report stage p50/p95/p99/max plus externally validated input-to-photon evidence.
- [ ] **Codec choice:** Show requested, negotiated, and actual settings; reject impossible combinations.
- [ ] **Immersive input:** Test permissions, focus loss, modifiers, repeat, relative mouse, and emergency release.
- [ ] **Clipboard:** Test both directions, duplicate suppression, disconnect, oversize, malformed HTML, and rapid updates.
- [ ] **Security:** Confirm unique credentials/certificate, verification enabled, and no sample secrets in artifacts.
- [ ] **Compatibility:** Build on every target distro family and test macOS 15 plus current macOS 27 beta.
- [ ] **Reproducibility:** Record all Kyber gitlinks, patches, compiler versions, and package manifests.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Driver/session mismatch | LOW | Reboot matching kernel, login to Xorg, rerun host doctor and exact NVENC probe |
| Unsupported codec tuple | LOW | Renegotiate to a proven hardware tuple; retain explicit error for manual request |
| VideoToolbox software fallback | MEDIUM | Fall back to hardware HEVC/H.264 4:2:0 for performance; isolate decoder work behind player seam |
| Latency queues discovered late | MEDIUM | Add stage IDs/timestamps, bound queues, retune from reliable baseline |
| Broad upstream fork divergence | HIGH | Rebase to pinned 0.27.0, split patch ledger by owned seam, remove Kymux/media-core changes |
| Clipboard loop/data leak | MEDIUM | Disable bridge, add origin/version policy and authenticated lifecycle, replay abuse tests |
| Test credentials published | HIGH | Rotate credentials/certificates, purge packaged secrets, audit access and history |
| macOS beta regression | MEDIUM | Preserve stable deployment target, isolate beta workaround, file upstream/Apple report, keep version-gated test |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Wrong host session/driver | Host readiness | Host doctor passes on real Xorg; NVML/NVENC/capture/uinput probes pass |
| Impossible codec/chroma | Capability contract | Tuple matrix tests reject AV1 4:4:4 and every unsupported pair deterministically |
| Silent software/copy fallback | Media vertical slice | Telemetry proves exact encoder/decoder/surface path |
| Misleading latency | Telemetry baseline | 30-minute 4K60 p50/p95/p99 and external input-to-photon evidence |
| Bad transport assumption | Network tuning | Impairment matrix compares every candidate to reliable baseline |
| Incomplete immersive input | Input/clipboard | Permission, shortcut, focus-loss, stuck-key, mouse-lock, and escape tests pass |
| Clipboard loop/leak | Input/clipboard | Bidirectional dedupe, limit, auth lifecycle, malformed/rapid input tests pass |
| Insecure defaults | Security foundation | No sample secrets; pin/auth/replay/downgrade checks pass |
| Upstream divergence | Repository foundation | Manifest recreates exact tree; patch ledger remains within approved seams |
| Distro/macOS drift | Packaging/compatibility | Clean builds and smoke tests across target distro families and macOS lanes |
| Premature Wacom/UI scope | Roadmap governance | Deferred requirements remain outside first proof and do not block it |

## Spike Gates

1. **HEVC 4:4:4 on Apple Silicon:** create a real VideoToolbox session with
   required hardware acceleration using the exact Kyber bitstream; record
   result, output format, 4K60 decode time, drops, and thermals.
2. **AV1 macOS path:** prove Kyber's packaged client can invoke a hardware
   VideoToolbox AV1 decoder on M3-or-newer; upstream's pinned VLC path is not
   sufficient evidence.
3. **NvFBC/X11 on each host class:** prove capture session creation, copy path,
   cursor semantics, rate, and permissions after host driver/session repair.
4. **4K60 throughput and tail latency:** measure the entire pipeline on the
   RTX A5000/Ampere baseline and at least one Ada host; extrapolation from
   NVIDIA encoder throughput tables is not acceptance.
5. **macOS 27 beta permissions:** rerun event-tap, fullscreen, clipboard,
   VideoToolbox, signing, and notarization smoke tests for every supported beta
   used internally.

## Sources

- NVIDIA Video Codec SDK 13.x NVENC capabilities:
  https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-application-note/index.html
- NVIDIA NVENC API programming guide:
  https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-video-encoder-api-prog-guide/index.html
- Apple VideoToolbox hardware decode APIs:
  https://developer.apple.com/documentation/videotoolbox/vtishardwaredecodesupported(_:)
- Apple tablet and event model:
  https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/
- Linux uinput:
  https://docs.kernel.org/input/uinput.html
- Kyber Desktop 0.27.0:
  https://gitlab.com/kyber/apps/kyber-desktop/-/tree/6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f
- Kyber kymedia:
  https://gitlab.com/kyber/core/kymedia/-/tree/e80eb6bb347ed0e378ae46a86f67ada2aa6079da
- Kyber kynput:
  https://gitlab.com/kyber/core/kynput/-/tree/5595478f3606c5624197360e2dcbf31bd60e8163
- Apple macOS 27 release notes:
  https://developer.apple.com/documentation/macos-release-notes/macos-27-release-notes
- Project stack, feature, and architecture research in this directory.

---
*Pitfalls research for: LinuxRemote / ReplayDesktop prototype*
*Researched: 2026-07-26*
