---
schema_version: 1
open_count: 3
waived_count: 0
fixed_count: 7
total_count: 10
last_updated: 2026-07-27T08:22:29.832Z
---

# Broken Windows Ledger

> Cross-phase defect register. `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | src/model.rs |  | Removed the stale unreachable NotImplemented decoder error variant found by the plan stub scan | fixed |  | 2026-07-27T00:29:48.042Z | 2026-07-27T00:30:09.403Z |
| 2 | 01 | unrun-verify | deny.toml |  | cargo-deny 0.20.2 conditional policy check was not run because the pinned executable is unavailable | open |  | 2026-07-27T00:31:18.358Z |  |
| 3 | 01 | deviation | .gitignore |  | Added Cargo target output exclusion so locked verification artifacts do not remain untracked | fixed |  | 2026-07-27T00:31:31.464Z | 2026-07-27T00:31:31.540Z |
| 4 | 01 | stub | src/probe.rs | 429 | Live probe worker deliberately returns native-probe-not-implemented until later Phase 1 plans supply native proofs. | open |  | 2026-07-27T01:06:43.669Z |  |
| 5 | 01 | deviation | src/probe.rs |  | Moved deadline validation before spawn and terminate/reap children when piped handles are unavailable. | fixed |  | 2026-07-27T01:07:37.629Z | 2026-07-27T01:07:52.178Z |
| 6 | 01 | deviation | src/currentness.rs |  | Expanded default run-duration currentness bound to contain all four maximum worker deadlines. | fixed |  | 2026-07-27T01:07:37.703Z | 2026-07-27T01:07:52.258Z |
| 7 | 01 | deviation | src/native_nvml.rs |  | Retained the exact loaded NVML userspace version when initialization fails so live kernel/userspace mismatch evidence remains complete | fixed |  | 2026-07-27T07:47:42.911Z | 2026-07-27T07:47:43.071Z |
| 8 | 01 | deviation | src/archive.rs |  | Accepted Cargo-style hard-linked executable descriptors while retaining single-link evidence input and descriptor-only copying | fixed |  | 2026-07-27T07:47:42.986Z | 2026-07-27T07:47:43.146Z |
| 9 | 01 | deviation | tests/host_doctor_cli.rs | 112 | Default-parallel full-suite run intermittently hit ETXTBSY launching a copied archive test binary; exact and serial-suite reruns passed. | fixed |  | 2026-07-27T08:22:00.694Z | 2026-07-27T08:22:29.832Z |
| 10 | 01 | deviation | tests/host_doctor_cli.rs | 655 | Default-parallel full-suite run intermittently hit ETXTBSY launching a copied archive test binary; exact and serial-suite reruns passed. | open |  | 2026-07-27T08:22:24.528Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "01",
    "file": "src/model.rs",
    "line": null,
    "description": "Removed the stale unreachable NotImplemented decoder error variant found by the plan stub scan",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T00:29:48.042Z",
    "resolved_at": "2026-07-27T00:30:09.403Z"
  },
  {
    "id": 2,
    "kind": "unrun-verify",
    "phase": "01",
    "file": "deny.toml",
    "line": null,
    "description": "cargo-deny 0.20.2 conditional policy check was not run because the pinned executable is unavailable",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-27T00:31:18.358Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "01",
    "file": ".gitignore",
    "line": null,
    "description": "Added Cargo target output exclusion so locked verification artifacts do not remain untracked",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T00:31:31.464Z",
    "resolved_at": "2026-07-27T00:31:31.540Z"
  },
  {
    "id": 4,
    "kind": "stub",
    "phase": "01",
    "file": "src/probe.rs",
    "line": 429,
    "description": "Live probe worker deliberately returns native-probe-not-implemented until later Phase 1 plans supply native proofs.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-27T01:06:43.669Z",
    "resolved_at": null
  },
  {
    "id": 5,
    "kind": "deviation",
    "phase": "01",
    "file": "src/probe.rs",
    "line": null,
    "description": "Moved deadline validation before spawn and terminate/reap children when piped handles are unavailable.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T01:07:37.629Z",
    "resolved_at": "2026-07-27T01:07:52.178Z"
  },
  {
    "id": 6,
    "kind": "deviation",
    "phase": "01",
    "file": "src/currentness.rs",
    "line": null,
    "description": "Expanded default run-duration currentness bound to contain all four maximum worker deadlines.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T01:07:37.703Z",
    "resolved_at": "2026-07-27T01:07:52.258Z"
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "01",
    "file": "src/native_nvml.rs",
    "line": null,
    "description": "Retained the exact loaded NVML userspace version when initialization fails so live kernel/userspace mismatch evidence remains complete",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T07:47:42.911Z",
    "resolved_at": "2026-07-27T07:47:43.071Z"
  },
  {
    "id": 8,
    "kind": "deviation",
    "phase": "01",
    "file": "src/archive.rs",
    "line": null,
    "description": "Accepted Cargo-style hard-linked executable descriptors while retaining single-link evidence input and descriptor-only copying",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T07:47:42.986Z",
    "resolved_at": "2026-07-27T07:47:43.146Z"
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "01",
    "file": "tests/host_doctor_cli.rs",
    "line": 112,
    "description": "Default-parallel full-suite run intermittently hit ETXTBSY launching a copied archive test binary; exact and serial-suite reruns passed.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-07-27T08:22:00.694Z",
    "resolved_at": "2026-07-27T08:22:29.832Z"
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "01",
    "file": "tests/host_doctor_cli.rs",
    "line": 655,
    "description": "Default-parallel full-suite run intermittently hit ETXTBSY launching a copied archive test binary; exact and serial-suite reruns passed.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-27T08:22:24.528Z",
    "resolved_at": null
  }
]
````
