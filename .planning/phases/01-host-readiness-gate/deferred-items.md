# Deferred Items

## Parallel archive-test `ETXTBSY`

- **Status:** resolved
- **Discovered during:** Plan 01-05 extra full-suite verification
- **Scope:** Pre-existing archive integration test; no archive source or test was
  changed by Plan 01-05.
- **Observation:** A default-parallel `cargo test --locked` run intermittently
  failed `archive_archived_binary_tamper_and_path_escape_fail_closed` while
  launching a copied executable with Linux `ETXTBSY` (`Text file busy`).
- **Recheck:** The exact test passed in isolation, and the complete suite passed
  with `--test-threads=1` (86 tests total).
- **Resolution:** Commit `164c365` serializes copied-executable creation through
  the child `exec` handshake while leaving child waits parallel. The exact test
  passed 10 consecutive runs, the parallel archive cluster passed 20 runs, and
  the default-parallel 86-test suite passed three times total (two worker runs
  plus the independent post-wave gate).

## Broken-windows ledger count mismatch

- **Status:** open
- **Discovered during:** Plan 01-09 summary closeout
- **Scope:** Pre-existing `.planning/WINDOWS.md` metadata; Plan 01-09 did not
  create or resolve the conflicting entries.
- **Observation:** Best-effort deviation appends were rejected because the
  ledger frontmatter reports `3/0/7/10` open/waived/fixed/total while its JSON
  entries resolve to `2/0/8/10`.
- **Next action:** Reconcile the table, JSON block, and frontmatter counts with
  `$gsd-health` before the next ship gate.
