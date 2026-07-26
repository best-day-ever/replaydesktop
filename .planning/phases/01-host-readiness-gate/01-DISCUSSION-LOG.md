# Phase 1: Host Readiness Gate - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-26
**Phase:** 1-Host Readiness Gate
**Areas discussed:** No additional user-facing gray areas

---

## Clear-Cut Infrastructure Path

The project definition, 49-requirement contract, four research studies, and
roadmap already lock Phase 1's observable behavior. The remaining ambiguities
are implementation mechanics rather than product choices a user should be
asked to decide. The workflow therefore did not manufacture a menu of
questions.

| Candidate gray area | Why no user decision was needed |
|---------------------|---------------------------------|
| G0 pass conditions | Locked by HOST-01 through HOST-04 and roadmap gate G0 |
| Physical output scope | User already accepted one physical X11 desktop and no virtual/multi-monitor prototype |
| Fallback policy | Project-wide decision forbids silent XCB/software/unsupported codec fallback |
| Current machine remediation | Reboot/Xorg is externally required; the tool must diagnose rather than mutate the machine |
| Probe/report implementation | Engineering choice delegated to the planner |

## the agent's Discretion

- Phase 1 implementation language and module layout
- CLI and structured-report schema
- Probe isolation, timeout, and fixture design
- Human-readable remediation format
- Direct API probes versus pinned helper processes

## Deferred Ideas

- All network, sustained-streaming, macOS, input, clipboard, Wacom, and
  cross-platform capabilities remain in their assigned later phases.
