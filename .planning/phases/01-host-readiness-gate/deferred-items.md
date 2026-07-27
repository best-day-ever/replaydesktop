# Deferred Items

## Parallel archive-test `ETXTBSY`

- **Discovered during:** Plan 01-05 extra full-suite verification
- **Scope:** Pre-existing archive integration test; no archive source or test was
  changed by Plan 01-05.
- **Observation:** A default-parallel `cargo test --locked` run intermittently
  failed `archive_archived_binary_tamper_and_path_escape_fail_closed` while
  launching a copied executable with Linux `ETXTBSY` (`Text file busy`).
- **Recheck:** The exact test passed in isolation, and the complete suite passed
  with `--test-threads=1` (86 tests total).
- **Deferred action:** Make copied-binary creation/launch synchronization robust
  under parallel integration-test execution in a plan that owns the archive
  test harness.
