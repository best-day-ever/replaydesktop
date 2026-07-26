<!-- GSD:project-start source:PROJECT.md -->

## Project

**ReplayDesktop**

ReplayDesktop is a prototype-first, high-performance remote desktop system for
controlling a physical Linux X11 desktop from an Apple Silicon Mac. It uses
Kyber/Kymux as the direct peer-to-peer media and input foundation, with NVIDIA
capture and NVENC on the host, and prioritizes proving a sharp, responsive
4K60 experience before investing in product UI, account infrastructure, or
cross-platform hosting.

The prototype is for internal use on directly reachable LAN or VPN hosts. It
must expose honest codec choices and measured behavior rather than silently
falling back to lower-quality or software paths.

**Core Value:** Prove that a Linux-to-macOS Kyber pipeline can deliver a visually excellent,
consistently low-latency 4K60 physical-desktop session with immersive control
and working clipboard synchronization.

### Constraints

- **Protocol**: Build on current Kyber/Kymux rather than inventing a new media
  transport — the prototype exists to validate and tune this stack.

- **Topology**: One directly reachable Linux host and one macOS client — no
  service-mediated discovery or NAT traversal.

- **Host OS**: Linux X11 only for the prototype — Wayland is deliberately
  excluded.

- **Host GPU**: NVIDIA NVENC-capable hardware — AV1 requires Ada or newer;
  older supported NVIDIA generations use HEVC/H.264 according to queried caps.

- **Client**: Apple Silicon and macOS 15 Sequoia or newer — no x86_64 build.
- **Performance**: First proof at 4K60 with zero configured video buffering and
  explicit stage-level latency telemetry — subjective smoothness alone is not
  acceptance evidence.

- **Quality**: HEVC 4:4:4 is a first-class manual fidelity choice, but a fully
  hardware-decoded 4:2:0 mode may be the latency default.

- **Security**: Authenticated TLS and certificate verification from the first
  networked build — test certificates and default credentials are prohibited.

- **License**: Prototype changes may comply with AGPLv3 network-source
  obligations — keep patches, notices, and build inputs reproducible.

- **Scope**: No audio, virtual display, polished UI, control server, or faithful
  Wacom transport in the first streaming proof.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Recommendation

### Delivery status

| Status | Stack decision |
|--------|----------------|
| **Proven upstream** | Direct hostname/IP, Kymux over QUIC/TLS, NvFBC→CUDA→NVENC, H.264/HEVC/AV1 encoder names, HEVC/H.264 YUV444 switches, macOS arm64 bundle, X11 `/dev/uinput`, XFixes clipboard protocol |
| **Must spike before 4K60 gate** | End-to-end HEVC 4:4:4 on every target Mac; AV1 4:2:0 through pinned libVLC; actual zero/minimal-copy surface flow; native keyboard suppression limits; stage telemetry |
| **Deferred** | AV1 4:4:4, Wayland, audio, virtual display, control/relay services, image/file clipboard, Wacom protocol, SwiftUI rewrite, dependency upgrades |

## Recommended Stack

### Core technologies

| Technology | Exact version / pin | Purpose | Why |
|------------|---------------------|---------|-----|
| Kyber Desktop | `0.27.0`, `6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f` | Superproject and client | Closest working foundation; released 2026-06-26 and inspected at the required commit. AGPL-3.0 is accepted. |
| Kyber SDK | `5836202aa4654cf64b5ca9fd204005fe4916be99` | Aggregates Kyber core components | Preserve the exact gitlink rather than following `main`. |
| Kymux | `831d3120bea004505dc69426eebbb9b07f10e443` | QUIC/TLS media and input data plane | Avoids inventing transport; lockfile resolves `quinn 0.11.9`, `wtransport 0.7.0`, `rustls 0.23.x`. |
| Kymedia | `e80eb6bb347ed0e378ae46a86f67ada2aa6079da` | Capture, conversion, encode, playback dependencies | Contains the coordinated FFmpeg/VLC/txproto patch set. |
| FFmpeg | `n8.1`, source SHA-256 `dd308201bb1239a1b73185f80c6b4121f4efdfa424a009ce544fd00bf736bb2e` plus Kyber's 15 patches | NVENC and codec plumbing | This is the Kyber-tested build, not a system FFmpeg. It includes AV1 VideoToolbox support in libavcodec but that path remains to be proven through VLC. |
| Kyber VLC fork | `dd2db54794384591684c1c86ce70eb64eb9eab15` (`kyber-260626`), based on VLC commit `4962271021860d1a7d0b9c6b00b4070af788537c` | Decode, scheduling, macOS video output | Retains Kyber’s zero-buffer playback integration and custom low-latency modules. Do not substitute Homebrew VLC. |
| txproto fork | `82694c38fb7d662ad364071382166edf8731db93` | NvFBC capture and FFmpeg graph | Implements the actual NvFBC/CUDA capture path used by Kyber. |
| NVIDIA codec headers | `nv-codec-headers n12.1.14.0` | Build FFmpeg NVENC/NVDEC bindings | Keep Kyber's tested header/API level. NVIDIA guarantees NVENC binary backward compatibility; SDK 13.1 would force driver 610/CUDA 13.1 and is unnecessary for Ampere/Ada features in scope. |
| NVIDIA display driver | Runtime dependency; certify `>=570.86.16`, exact kernel/userspace match | Provides `libnvidia-fbc.so`, `libnvidia-encode.so`, CUDA interop | 570.86.16 is the current Capture SDK 9 minimum. Do not bundle it. Record the full driver/kernel pair in every benchmark. Prefer distro packages. |
| Rust | `1.89.0` via `rustup` | Kyber control/client/input code | Exact upstream toolchain. Add a checked-in `rust-toolchain.toml`; do not use moving `stable`. |
| Cargo | shipped with Rust `1.89.0`; all existing `Cargo.lock` files committed | Reproducible Rust resolution | Use `cargo build --locked` and `cargo install --locked cargo-c@0.10.15`. |
| Meson | `1.10.0` | Kymedia/dependency build | Exact minimum required by Kyber 0.27.0; pin the Python package/tool, not distro Meson. |
| Linux build images | Kyber image revision `a3d1e282885b1ef5fd5fdbbf7a7d8e0b3a33de15` (`ubuntu-noble` and `debian-trixie`) | Reproduce upstream baseline | Mirror these registry images by immutable OCI digest before relying on them; derive Rocky/Arch qualification builders from documented lockfiles rather than floating distro tags. |
| macOS SDK/toolchain | Release: Xcode `26.6`; compatibility: Xcode `27 beta 4`; deployment target `15.0`; `arm64` only | Native frameworks and packaging | Xcode 26.6 is the current stable lane; 27 beta 4 is isolated to a non-shipping compatibility lane. Set `MACOSX_DEPLOYMENT_TARGET=15.0` and `DEVELOPER_DIR` explicitly. |

### Exact Kyber gitlink lock

| Component | Commit |
|-----------|--------|
| `external/winit` | `73cc88800dffb4e385e853c553fbe8341a2c6771` |
| `kysdk/kyctl` | `e46dda2825d6e8c432521cfcb4c5726b5bba8bc4` |
| `kysdk/kymedia` | `e80eb6bb347ed0e378ae46a86f67ada2aa6079da` |
| `kysdk/kymux` | `831d3120bea004505dc69426eebbb9b07f10e443` |
| `kysdk/kynput` | `5595478f3606c5624197360e2dcbf31bd60e8163` |
| `kysdk/kyutil` | `5704c85dc53e50b16341916aca29cf26cba327c4` |
| `kymedia/libvlcjni` | `6145355a14b8888ccbfbe37bb34f1685799992fe` |
| `kymedia/txproto` | `82694c38fb7d662ad364071382166edf8731db93` |
| `kymedia/vlc` | `dd2db54794384591684c1c86ce70eb64eb9eab15` |
| `kymedia/vlc-rs` | `7cbfc51313b4bb3ab07be51505b7054e4e2c366b` |
| `kynput/keycode` | `e3c17908f610d1a5485873850f7184b291146731` |
| `kynput/libudev-sys` | `ce4f263551bc0c3903fc6a2ac416ec71510aacb5` |
| `kynput/rust-sdl2` | `38ad0da9ff7b4b211e151570ab9510f3e8882f93` |
| `kynput/vigem-client` | `874ab2aed14d413c6432414b75469784d26a0048` |

### Platform APIs and supporting libraries

| API/library | Version | Use |
|-------------|---------|-----|
| NvFBC shared CUDA API | Kyber-bundled header ABI `1.8`; runtime library from driver | Primary X11 physical-display capture. Request `NVFBC_CAPTURE_SHARED_CUDA`; use NV12 for 4:2:0 and YUV444P for 4:4:4. Log `bRequiredPostProcessing`, timestamp, dimensions, and capture status. |
| NVENC through FFmpeg | FFmpeg 8.1 + headers 12.1.14.0 | `h264_nvenc`, `hevc_nvenc`, `av1_nvenc`; preserve Kyber `p2`, `tune=ull`, `zerolatency=1`, `bf=0`, FIFO 1 baseline. |
| XCB/XFixes/XTest/XKB | Existing Kyber `xcb` crates/system libraries | Host clipboard ownership/selection monitoring and input support. Preserve the existing 60 KiB clipboard protocol limit for MVP. |
| `/dev/uinput` + libevdev/libudev | Distro system ABI | Host input injection. Ship a narrowly scoped udev rule/group policy; never run the controller as root. |
| VideoToolbox/CoreMedia/CoreVideo | macOS 15 SDK ABI, compiled by Xcode 26.6 | Preflight and actual decode-path reporting. Use `VTIsHardwareDecodeSupported`, then create a session for the exact real bitstream. Query `kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder`; use the `RequireHardware...` key for Auto hardware-only candidates. |
| libVLC | Pinned Kyber fork | Default H.264/HEVC decoder/render path. Patch a small status callback exposing selected decoder, output pixel format, queue depth, and presentation timestamps. |
| Metal + QuartzCore | macOS 15 SDK ABI | Only add a native `CVPixelBuffer` → `CVMetalTextureCache` → `CAMetalLayer` output if libVLC measurement proves an extra copy or unbounded presentation queue. Do not begin with a wholesale player rewrite. |
| CoreGraphics event taps | macOS 15 SDK ABI | Immersive keyboard/mouse bridge: `CGEventTapCreate`, `CGAssociateMouseAndMouseCursorPosition`, cursor hide/warp, focus-loss cleanup. Requires Accessibility consent. Document OS-reserved shortcuts that cannot be captured. |
| AppKit `NSPasteboard` | macOS 15 SDK ABI | Poll/change-count bridge for `public.utf8-plain-text` and `public.html`, mapped to Kyber `ClipboardEvent`; deduplicate origin and enforce 60 KiB UTF-8-safe truncation. |
| Security / trust | Existing rustls stack; Security.framework only for local identity storage if needed | Replace test certificates. Pin an explicitly imported CA/certificate fingerprint; fail closed on mismatch. Do not disable certificate verification. |

## Capability probes (required, not optional)

| Probe | Implementation | Pass condition |
|-------|----------------|----------------|
| Host capture | Load NvFBC, `NvFBCGetStatus`, enumerate outputs, create shared-CUDA session | Correct physical output, new-frame timestamps, no CPU buffer path; otherwise fail (XCB is diagnostic only). |
| Host codec/chroma | Enumerate FFmpeg encoder and pixel formats, then open a 3840×2160@60 test encode for each pair; optionally call `NvEncGetEncodeCaps` directly | H.264/HEVC/AV1 GUID present, dimensions accepted, `NV_ENC_CAPS_SUPPORT_YUV444_ENCODE` when 4:4:4 requested. AV1 is offered only on Ada-or-newer and only 4:2:0. |
| Client coarse decode | `VTIsHardwareDecodeSupported(codec)` | Necessary but not sufficient; it is codec-wide, not profile/chroma-specific. |
| Client exact decode | Feed an encoded IDR/keyframe plus parameter sets to a hardware-required `VTDecompressionSession` | Session creation and first decoded 4K frame succeed; queried hardware property is true. This is authoritative for Auto. |
| VLC path | Start pinned libVLC with zero network/video caching and new instrumentation | Selected module/path, pixel format, queue depth, and first-frame time are reported. No silent software fallback. |
| 4K60 | 10-minute instrumented run with monotonic timestamps at capture, conversion, encode submit/complete, transport send/receive, decode submit/complete, present | 3840×2160 at 60 Hz, no hidden buffering, bounded drops/latency per acceptance thresholds. |

### Codec policy

| Mode | Offer when | Default status |
|------|------------|----------------|
| HEVC 4:2:0 | NVENC open succeeds and exact VT session succeeds in hardware | Preferred Auto candidate on all Apple Silicon generations |
| H.264 4:2:0 | NVENC and exact VT session succeed | Hardware fallback |
| AV1 4:2:0 | Host is Ada-or-newer and exact AV1 VideoToolbox path succeeds on the client (Apple advertises AV1 media decode beginning with M3) | Experimental/manual until pinned VLC path passes 4K60 |
| HEVC 4:4:4 | Host YUV444 capability and exact client session pass | Manual fidelity mode; label hardware/software explicitly |
| AV1 4:4:4 | Never | Unsupported by Ada NVENC Main profile; reject before session start |

## Development and test tools

| Tool | Pin / policy | Purpose |
|------|--------------|---------|
| `cargo fmt`, `clippy`, `cargo test` | Rust 1.89.0; always `--locked`; warnings denied for changed crates | Rust quality gate |
| Meson test + Ninja | Meson 1.10.0; use generated Ninja from the chosen build image | Native unit/build gate |
| ASan/UBSan | Linux Clang lane and Xcode 26.6 lane | C/C++/ObjC memory/UB checks; do not benchmark sanitizer builds |
| ThreadSanitizer | Separate targeted tests | Clipboard/input/session state; Xcode 26.4 notes sanitizer hangs with older Xcode, hence use 26.6 |
| `ffmpeg`/`ffprobe` | The built pinned FFmpeg 8.1 binaries | Encoder listing, pixel formats, stream profile/chroma verification, frame timestamps, framemd5 |
| NVIDIA tools | `nvidia-smi`, driver logs, NvFBC logs, Video Codec SDK sample probe built separately | Record GPU/driver state and independently verify encode capabilities |
| Network tools | distro `iperf3`, `tc netem`, `ss`, packet capture; record exact versions in test manifest | Baseline bandwidth and deterministic loss/jitter/RTT tests |
| macOS instruments | `xctrace` from Xcode 26.6; Time Profiler, System Trace, Metal System Trace | CPU wakeups, copies, scheduling, GPU/present timing |
| Quality fixtures | `libvmaf` filter from a separate analysis FFmpeg build, plus OCR/checkerboard/text fixtures | Offline fidelity comparison; do not add libvmaf to shipping binaries |
| Integration harness | Rust tests + shell launcher, fixed 4K60 fixture, JSONL telemetry | Reproducible start/connect/probe/stream/input/clipboard/failure tests |
| Supply chain | `cargo-deny` and `cargo-audit` pinned in `tools.lock`; Syft SPDX JSON pinned by container digest | License/advisory checks and SBOM. Findings are reviewed; do not auto-upgrade lockfiles. |

## Build and packaging

### Linux host

- Build **three native qualification artifacts**, not one “universal Linux” binary: Ubuntu 24.04 x86_64, Rocky 9 x86_64, and Arch/CachyOS x86_64.
- Use Kyber's staged rootfs/tar layout initially. Bundle the pinned Kyber-built FFmpeg/VLC/private `.so` files, but dynamically use the host's glibc, X11, kernel, and NVIDIA driver libraries.
- Do not use AppImage, Flatpak, Snap, Docker, or static glibc for the latency prototype. GPU/X11/device/driver integration and sandbox permissions obscure failures. Containers are acceptable only as builders.
- Package a systemd **user** service template, configuration, udev guidance, certificates/public trust material, `SOURCE-OFFER.md`, `UPSTREAM.lock`, `PATCHES.md`, and SPDX SBOM. Never include Kyber's `kybertest_*` keys/certificates.

### macOS client

- Build `aarch64-apple-darwin` only, `.app` + zipped bundle, `LSMinimumSystemVersion=15.0`.
- Release lane: Xcode 26.6. Compatibility lane: Xcode 27 beta 4 on a separate runner, non-shipping.
- For internal same-machine testing, ad-hoc signing is acceptable. For transfer to other Macs, use Developer ID Application signing, hardened runtime, `notarytool`, stapling, and verify with `codesign --verify --deep --strict` and `spctl`.
- Do not instruct users to remove quarantine with `xattr`; that is an upstream development shortcut, not a packaging strategy.

## Dependency strategy

## Alternatives considered

| Recommended | Alternative | Why not now |
|-------------|-------------|-------------|
| Kyber 0.27.0 fork | Sunshine/Moonlight, RustDesk, custom QUIC | Violates the chosen base or discards working Kyber integration; useful only if feasibility fails. |
| Pinned VLC first | Immediate native VideoToolbox/Metal player | Larger rewrite before the baseline; use only if instrumentation proves VLC cannot meet buffering/path requirements. |
| NvFBC shared CUDA | XCB capture | XCB is CPU RGB capture and cannot satisfy the zero/minimal-copy requirement; retain only for diagnosis. |
| Headers 12.1.14.0 | Video Codec SDK 13.1 | SDK 13.1 requires driver 610/CUDA 13.1 and adds mostly Blackwell features outside scope. |
| Native distro packages/tar | AppImage/Flatpak/Snap/container runtime | Driver/device/display integration and sandboxing complicate latency proof. |
| Rust 1.89.0 | Current stable Rust | Upstream reproducibility is more valuable than unrelated compiler churn. |

## What not to use

| Avoid | Reason | Use instead |
|-------|--------|-------------|
| Kyber `main` or floating submodules | Non-reproducible coordinated dependency changes | Exact lock above |
| System/Homebrew FFmpeg or VLC | Omits Kyber patches and changes codec/module behavior | Pinned Kyber builds |
| Kyber test certificate/default credentials | No authenticated trust and explicitly prohibited | Generated per-host certificate plus explicit CA/fingerprint import |
| Silent software decode | Invalidates Auto latency claims | Hardware-required exact-bitstream probe; manual software mode clearly labeled |
| AV1 based only on GPU/Mac model names | Driver, profile, chroma, and player path still vary | Runtime encode + exact decode probes |
| XCB as automatic fallback | Hides loss of zero-copy and can pass superficially at low load | Fail clearly; expose XCB only as diagnostic |
| SwiftUI/Tauri/Electron UI rewrite | Adds scope and latency/build variables | Existing winit client plus minimal native bridges |
| Unpinned beta Xcode for shipping | Beta behavior and SDK drift | Xcode 26.6 release lane; 27 beta 4 qualification lane |

## Sources

- [Kyber Desktop 0.27.0 repository and README](https://gitlab.com/kyber/apps/kyber-desktop) — release, build requirements, packaging (HIGH; repository inspected at exact commit)
- [Kyber Desktop changelog](https://gitlab.com/kyber/apps/kyber-desktop/-/blob/6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f/CHANGELOG.md) — FFmpeg 8.1 and release history (HIGH)
- [NVIDIA Video Codec SDK 13.1 index](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/index.html) and [system requirements](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/read-me/index.html) — current SDK and driver/CUDA requirements (HIGH)
- [NVIDIA NVENC application note](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-application-note/index.html) — Ampere/Ada HEVC 4:4:4 and Ada AV1 Main 4:2:0 matrix (HIGH)
- [NVIDIA NVENC programming guide](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-video-encoder-api-prog-guide/index.html) — capability queries and API compatibility (HIGH)
- [NVIDIA Capture SDK 9.0](https://developer.nvidia.com/capture-sdk) — NvFBC characteristics and driver minimum (HIGH)
- [Apple VideoToolbox hardware enable key](https://developer.apple.com/documentation/videotoolbox/kvtvideodecoderspecification_enablehardwareacceleratedvideodecoder), [require key](https://developer.apple.com/documentation/videotoolbox/kvtvideodecoderspecification_requirehardwareacceleratedvideodecoder), and [selected-decoder property](https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_usinghardwareacceleratedvideodecoder) — authoritative runtime probe mechanics (HIGH)
- [Apple `VTIsHardwareDecodeSupported`](https://developer.apple.com/documentation/videotoolbox/vtishardwaredecodesupported%28_%3A%29) — coarse codec probe (HIGH)
- [Apple M3 announcement](https://www.apple.com/newsroom/2023/10/apple-unveils-m3-m3-pro-and-m3-max-the-most-advanced-chips-for-a-personal-computer/) — first advertised Apple Silicon AV1 decode (HIGH)
- [Apple Xcode system requirements](https://developer.apple.com/xcode/system-requirements) — Xcode 26.6 and Xcode 27 beta 4 lanes (HIGH)
- [GNU AGPLv3](https://www.gnu.org/licenses/agpl-3.0.html) — license text (HIGH; legal implementation should still be reviewed)

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `$gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `$gsd-debug` for investigation and bug fixing
- `$gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `$gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
