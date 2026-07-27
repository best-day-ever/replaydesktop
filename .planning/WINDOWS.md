---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 2
total_count: 3
last_updated: 2026-07-27T00:31:31.540Z
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
  }
]
````
