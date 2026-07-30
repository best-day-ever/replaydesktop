---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 01
current_phase_name: host-readiness-gate
status: verifying
stopped_at: Completed 01-10-PLAN.md
last_updated: "2026-07-30T16:21:30+02:00"
last_activity: 2026-07-30
last_activity_desc: "Completed quick task 260730-j91: macOS technical GUI and high-resolution scrolling"
progress:
  total_phases: 1
  completed_phases: 1
  total_plans: 10
  completed_plans: 10
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-26)

**Core value:** Prove that a Linux-to-macOS Kyber pipeline can deliver a visually excellent, consistently low-latency 4K60 physical-desktop session with immersive control and working clipboard synchronization.
**Current focus:** Phase 01 — host-readiness-gate

## Current Position

Phase: 01 (host-readiness-gate) — EXECUTING
Plan: 10 of 10
Status: Phase complete — ready for verification
Last activity: 2026-07-30 — Completed quick task 260730-j91: macOS technical GUI and high-resolution scrolling

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: 9
- Average duration: 44m
- Total execution time: 6h 32m

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| Phase 01 | 9 | 6h 32m | 44m |

**Recent Trend:**

- Last 5 plans: 30m 16s, 46m 23s, 50m 30s, 1h 39m 25s, 40m
- Trend: Longer recent plans reflect increasingly hardware-bound proof, authenticated native ABI work, and adversarial evidence review.

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P01 | 33 min | 3 tasks | 10 files |
| Phase 01 P02 | 29 min | 3 tasks | 9 files |
| Phase 01-host-readiness-gate P03 | 33m 28s | 3 tasks | 9 files |
| Phase 01-host-readiness-gate P04 | 29m 58s | 3 tasks | 11 files |
| Phase 01-host-readiness-gate P05 | 30m 16s | 2 tasks | 8 files |
| Phase 01-host-readiness-gate P06 | 46m 23s | 3 tasks | 23 files |
| Phase 01-host-readiness-gate P07 | 50m 30s | 3 tasks | 11 files |
| Phase 01-host-readiness-gate P08 | 1h 39m 25s | 3 tasks | 10 files |
| Phase 01 P09 | 40m | 3 tasks | 12 files |
| Phase 01 P10 | 58m | 3 tasks | 22 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Phase 1]: G0 host readiness is an executable stop/go prerequisite; no media result from the preserved pre-reboot Wayland/NVIDIA-mismatch state is admissible.
- [Phase 2]: Reproducibility and authenticated direct transport precede all performance evidence.
- [Phase 5]: Keep pinned libVLC unless exact decoder/copy/queue/presentation evidence requires the narrow native adapter.
- [Phase 6]: Hardware 4:2:0 latency proof and HEVC 4:4:4 fidelity proof remain distinct; AV1 4:4:4 is always invalid.
- [Phase 01]: Freeze V1 base fields; later host facts remain independently validated extension payloads.
- [Phase 01]: Require all four known G0 records and derive overall status/reasons from their ordered terminal states.
- [Phase 01]: Use one sha2-backed 32-byte lowercase SHA-256 identity contract for bytes, readers, and files.
- [Phase 01]: Deny dependency drift with exact locked-package and direct-feature allowlists.
- [Phase 01]: Keep run identity, provenance, currentness, extension status, and G0 verdict exclusively parent-owned; workers return nonce-bound primitive observations only.
- [Phase 01]: Fixture observations traverse production evaluation and persistence but remain diagnostic FAIL/UNPROVEN regardless of supplied positives.
- [Phase 01]: Accept evidence only after mode-0600 atomic persistence, strict V1 decoding, byte-identical exact-run readback, and current boot/session/binary/time verification.
- [Phase 01]: Local Xorg admission requires one active local logind session whose AF_UNIX peer identity, real Xorg executable, X11 setup, and RandR facts all agree.
- [Phase 01]: NVML admission requires authenticated API 13 source/ABI metadata and the complete fixed-SONAME runtime/device/shutdown sequence.
- [Phase 01]: Strict host-foundation.v1 semantics are enforced on application readback without changing the generic V1 base decoder.
- [Phase 01]: Selected-output correlation is proven for exact XRandR output DP-0.3; NvFBC and NVENC remain UNPROVEN and prevent G0 PASS.
- [Phase 01]: The executable source is trusted only through an open /proc/self/exe descriptor; Cargo-style hard links are safe because path identity is never reopened or resolved.
- [Phase 01]: The loaded libnvidia-ml.so exact filename version is recorded before NVML initialization so initialization failure cannot hide a real kernel/userspace mismatch.
- [Phase 01]: Archive verification is offline and never executes the archived binary or consults target/ or the current executable.
- [Phase 01]: Plans 01-06, 01-08, and 01-10 and every base-decoder/archive-verifier change must run both permanent compatibility tests.
- [Phase 01]: HOST-02 ownership follows XRandR output XID to an NV-CONTROL display target, enabled X-screen membership, one owning NV-CONTROL GPU, then one NVML device matching both BDF and UUID.
- [Phase 01]: NV-CONTROL display and GPU target ID zero are valid and must not be rejected by XID-style validators.
- [Phase 01]: MST is accepted when the direct source relation is unique; DRM connector state is optional diagnostic data and never enters HOST-01/HOST-02 admission or the authoritative token.
- [Phase 01]: Display targets are enumerated through binary attribute 14; XNVCTRLQueryTargetCount is used for GPU targets only because display-target count raises BadValue on the live driver.
- [Phase 01]: Retain ProbeId::NvfbcCapture as the single HOST-03 probe identifier.
- [Phase 01]: Require both authenticated NvFBC and CUDA roots before enabling the conditional ABI oracle.
- [Phase 01]: Derive capture leases, copy totals, cleanup, and verdicts in the parent; workers supply primitives only.
- [Phase 01]: Diagnostic capture fixtures remain UNPROVEN; Plan 01-08 owns live HOST-03 proof.
- [Phase 01]: Authenticate the operator-supplied NvFBC 1.9 and installed CUDA Driver API 13.3 declarations by exact digest without vendoring proprietary sources.
- [Phase 01]: Bind live NvFBC capture to exact XRandR XID, NV-CONTROL GPU, NVML BDF/UUID, and the source identity compiled into the current executable.
- [Phase 01]: Admit HOST-03 only after one fresh selected-output NV12 frame, one same-GPU device copy, zero host staging, and complete truthful cleanup.
- [Phase 01]: Treat NvFBC cursor visibility and composition as independent facts; hidden-but-composited is valid.
- [Phase 01]: HOST-03 ends at the proven application-owned capture surface; HOST-04 owns NVENC registration, mapping, input, internal-copy, and tuple proof.
- [Phase 01]: Treat the documented same-GPU pitch-linear to block-linear NVENC preprocessing copy as known; host, peer, or unknown edges block admission.
- [Phase 01]: Keep NVENC policy closed to seven 4K60 positions and prefilter AV1 before provider invocation on pre-Ada or unknown generations.
- [Phase 01]: Codec bytes prove codec/profile/chroma/depth/dimensions/keyframe only; buffer format and exact 60/1 require native resource/config evidence.
- [Phase 01]: Diagnostic SDK versions never authorize tuple advertisement; live NVENC stays unavailable until Plan 01-10.
- [Phase 01]: Keep the official SDK 13.1 standalone NVENC proof isolated from Kyber/Kymedia's pinned nv-codec-headers n12.1.14.0; later integration compatibility remains unclaimed.
- [Phase 01]: Advertise only tuples proven from the exact current NV12 lease; incompatible HEVC inputs and Ampere-ineligible AV1 stay terminal and non-advertised.
- [Phase 01]: Use distinct post-repair archive schemas and dispatch verification by exact matching index/manifest contract so PASS support cannot loosen pre-reboot history.
- [Phase 01]: Parse only a bounded IDR slice-header prefix while hashing and clearing the complete capped NVENC bitstream.

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 5]: Exact Apple Silicon HEVC 4:4:4, AV1 4:2:0, and libVLC/native-player behavior remain runtime questions.
- [Phase 9]: Real pre-M3/M3+ Mac and non-reference Linux distro hardware are required before their compatibility rows can be claimed.

### Quick Tasks Completed

| # | Description | Date | Commit | Status | Directory |
|---|-------------|------|--------|--------|-----------|
| 260730-iie | Prototype LAN connection without user-managed trust | 2026-07-30 | runtime-only |  | [260730-iie-make-prototype-lan-mode-require-no-user-](./quick/260730-iie-make-prototype-lan-mode-require-no-user-/) |
| 260730-j91 | macOS technical GUI, live telemetry panel, audio/multi-monitor controls, and high-resolution scrolling | 2026-07-30 | ecdff71 | Needs Review | [260730-j91-build-and-publish-a-macos-technical-gui-](./quick/260730-j91-build-and-publish-a-macos-technical-gui-/) |

## Deferred Items

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| v2 | Tablet fidelity, higher refresh, audio/HDR, product connectivity, additional platforms, Wayland, and virtual desktops | Deferred | Project definition |

## Session Continuity

Last session: 2026-07-30T00:18:29.330Z
Stopped at: Completed 01-10-PLAN.md
Resume file: None
