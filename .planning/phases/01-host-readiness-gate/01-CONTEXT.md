# Phase 1: Host Readiness Gate - Context

**Gathered:** 2026-07-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers a repeatable, fail-closed G0 host-readiness check for the real
Linux machine. It identifies one physical X11 output and proves the NVIDIA
driver, device access, NvFBC shared-CUDA capture path, copy boundaries, and
one-frame NVENC tuples before any later phase is allowed to treat media results
as evidence.

This phase does not vendor or customize the full Kyber application, establish a
network session, optimize sustained streaming, implement codec negotiation, or
touch macOS input/clipboard. Those capabilities begin in later phases.

</domain>

<decisions>
## Implementation Decisions

### G0 Admission Contract

- **D-01:** G0 is fail-closed. A real Xorg session, matching loaded NVIDIA
  kernel/userspace stack, working NVML, one selected physical output,
  `/dev/uinput` and DRM/render access, NvFBC shared-CUDA capture creation, and
  successful exact one-frame NVENC probes are all mandatory.
- **D-02:** The doctor must distinguish “API or library exists” from “the exact
  runtime path succeeded.” FFmpeg encoder enumeration, CUDA device discovery,
  or an NvFBC shared object alone never counts as a passing probe.
- **D-03:** The operator selects exactly one physical X11 output. Multi-monitor
  panorama, hot switching, Wayland/Xwayland capture, XCB fallback, software
  encoding, and automatic virtual displays are outside G0.
- **D-04:** Every offered NVENC capability is an atomic codec/profile/chroma/
  bit-depth tuple successfully probed on the actual GPU and driver. In
  particular, Ampere cannot advertise AV1 encode and Ada cannot advertise AV1
  4:4:4.
- **D-05:** Capture-to-encoder evidence must identify every GPU-to-GPU,
  GPU-to-CPU, cross-GPU, conversion, and staging boundary. An unaccounted copy
  makes the capture-path check fail or remain explicitly unproven.
- **D-06:** The current development machine's Wayland session and NVIDIA
  kernel/userspace mismatch are a known failing fixture. The implementation can
  be built and its failure diagnostics tested now; a live G0 PASS requires a
  reboot into a matching installed kernel and a real Xorg login.

### Safety and Evidence

- **D-07:** The readiness tool diagnoses and explains remediation but does not
  reboot, change sessions, install drivers, alter permissions, or mutate system
  configuration automatically.
- **D-08:** Probe failures are bounded by timeouts and leave no capture session,
  encoder session, virtual input state, or temporary display state behind.
- **D-09:** A durable G0 result records exact OS/kernel/session, NVIDIA
  kernel/userspace/API versions, GPU identity, output mode, device permissions,
  capture backend, copy evidence, tested tuples, timestamps, and PASS/FAIL
  reasons. Later benchmark reports reference this evidence rather than
  reinterpreting host state.

### the agent's Discretion

- Choose the Phase 1 implementation language, crate/module layout, stable
  machine-readable schema, human-readable presentation, CLI flag names, and
  exit-code taxonomy.
- Choose direct library/API probes versus narrowly scoped helper-process probes,
  provided the exact runtime path is exercised and time-bounded.
- Choose the test fixture strategy for driver/session/device permutations.
- Choose whether the first implementation embeds low-level NvFBC/NVENC probes
  or delegates them to pinned minimal helpers, provided output is structured
  and reproducible and no mutable system FFmpeg claim is treated as sufficient.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Contract

- `.planning/ROADMAP.md` §Phase 1 — phase boundary, G0 gate, dependencies, and
  success criteria.
- `.planning/REQUIREMENTS.md` §Linux Host Readiness and Capture — HOST-01
  through HOST-04, the exact requirements owned by this phase.
- `.planning/PROJECT.md` — prototype constraints, current host state, accepted
  exclusions, and key architecture decisions.

### Research and Risk

- `.planning/research/SUMMARY.md` §Stop/Go Gates and Execution Prerequisite —
  reconciled G0 evidence and failure disposition.
- `.planning/research/STACK.md` §Capability probes — pinned platform and
  NVIDIA probe expectations.
- `.planning/research/ARCHITECTURE.md` §HostProbe and Test Seams — component
  boundary and downstream integration contract.
- `.planning/research/PITFALLS.md` §Pitfall 1 and Pitfall 4 — hidden-fallback
  and wrong-session/driver failure modes.

No external project-specific spec or ADR exists beyond these planning
artifacts. Upstream NVIDIA and Kyber references are catalogued in the research
files above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- No product source exists yet. The repository currently contains only the GSD
  project contract, requirements, research, roadmap, state, and project guide.
- The local machine itself provides the first negative fixture: Wayland,
  mismatched NVIDIA kernel/userspace components, an RTX A5000 Laptop GPU, two
  enabled 4K outputs, and already-granted `/dev/uinput`/DRM ACLs.

### Established Patterns

- Reproducibility, explicit failure, exact runtime evidence, and no silent
  fallback are project-wide invariants.
- Planning uses vertical MVP phases and atomic commits; Phase 1 must leave a
  runnable doctor and archived G0 result even though the live PASS depends on a
  user reboot/session change.

### Integration Points

- Phase 2 will consume the G0 result before it vendors/builds and starts the
  pinned Kyber baseline.
- Phase 3 will reuse the host identity, selected output, capture/encoder
  evidence, tuple schema, and stable error vocabulary for continuous streaming
  telemetry.
- Phase 4 will evolve the probed tuples into the authenticated media-capability
  contract; Phase 1 must not invent a competing transport or negotiation model.

</code_context>

<specifics>
## Specific Ideas

- Treat the current bad environment as a first-class test case: it should
  report a real Xorg requirement and the `610.43.02` loaded-kernel versus
  `610.43.03` userspace mismatch rather than reaching a generic NVENC error.
- The current reference GPU is an NVIDIA RTX A5000 Laptop GPU (Ampere). Its
  expected codec set is useful for proving that advertised FFmpeg AV1 support
  is filtered out by the actual hardware probe.
- The current physical outputs include 3840×2160 near 60 Hz and 3840×2160 near
  120 Hz. Phase 1 selects and reports one; it does not attempt the 4K60 soak.

</specifics>

<deferred>
## Deferred Ideas

- Secure direct network streaming and the pinned Kyber source tree — Phase 2.
- Continuous copy/queue/latency telemetry and 4K60 engineering baseline —
  Phase 3.
- Capability negotiation, macOS decode/presentation, codec/fidelity matrix,
  immersive input, clipboard, and platform qualification — Phases 4–9.
- Wacom fidelity, audio, higher-refresh acceptance, Wayland, virtual displays,
  control service/NAT traversal, and additional platforms — v2.

</deferred>

---

*Phase: 1-Host Readiness Gate*
*Context gathered: 2026-07-26*
