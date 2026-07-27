# Phase 01 Host Readiness Validation

This guide records the immutable pre-reboot HOST-01 failure captured on
2026-07-27. The capture was read-only: it did not reboot, change sessions,
repair drivers, load modules, alter permissions, install software, or contact
a network. The only durable writes were the evidence file and the archive
root.

## Original-host precondition

Immediately before capture, read-only checks reported:

- active local session type: `wayland`;
- loaded NVIDIA kernel module: `610.43.02`;
- resolved NVML userspace library:
  `/usr/lib/libnvidia-ml.so.610.43.03`;
- `nvidia-smi` exit `18`, reporting a driver/library mismatch.

No fixture evidence was substituted for these live facts.

## Fail-closed preflight correction

An initial preflight capture exposed an archive validation bug before any
archive index or run directory was committed: Cargo had hard-linked
`target/debug/replay-host-doctor` to its hashed build artifact, while the
archive source check incorrectly required `/proc/self/exe` to have exactly one
link. The empty archive root was confirmed before continuing.

The source check was corrected to accept a regular executable reached through
the already-open `/proc/self/exe` descriptor regardless of hard-link count.
The evidence source still requires exactly one link. A process regression now
creates a Cargo-style executable hard link and proves descriptor-only
copying/hashing succeeds. The doctor was rebuilt after that correction, and a
new final live run was used for the archive below; the rejected preflight
evidence was not archived.

## Build and controlled live capture

The doctor was built once with the operator-provided official NVML header
root:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo build --locked --bin replay-host-doctor
```

The exact built executable had SHA-256
`9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e`.
The live run used a fresh path and captured the expected nonzero result before
any archive operation:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor run \
    --evidence target/g0-pre-reboot-live-01-04-archived.json \
    --probe-timeout-ms 2000
live_status=$?
set -e
test "$live_status" -eq 2
```

Result: exit `2`, provenance `live`, overall status `fail`, run ID
`run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`,
boot ID `be28f2a4-b236-4fbf-b0e2-5a90d0e9641e`, and process session
`sid-1968565`.

The `host-foundation.v1` payload was independently checked before archival:

| Fact | Preserved value |
|---|---|
| local session reason | `SESSION_NOT_XORG` |
| NVIDIA kernel version | `610.43.02` |
| NVML userspace version | `610.43.03` |
| NVIDIA mismatch reason | `NVIDIA_VERSION_MISMATCH` |
| NVML initialization | failed closed, `NVML_INITIALIZATION_FAILED` |

## Create-once archive

No build or source change occurred between the live run and the successful
archive copy. The archive command was then invoked against the live evidence:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  target/debug/replay-host-doctor archive-pre-reboot \
    --evidence target/g0-pre-reboot-live-01-04-archived.json \
    --archive-root artifacts/validation/g0/pre-reboot
```

It reported `status: "archived"` and wrote the create-once index
`artifacts/validation/g0/pre-reboot/index.json`. The index contains the
relative manifest path:

```text
run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/manifest.json
```

Recorded digests:

| Contained object | SHA-256 |
|---|---|
| archived executable | `9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e` |
| archived evidence | `5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61` |
| manifest | `d4d16fc2bee5960cedd01881e3fcc8cd1496ed014ceadb88fe67f288490ee136` |
| index | `66d6fbf707f39781bed50c21eaf201be7d3a63f64bb1e96e0cf320ca14435c3c` |

The committed run directory is mode `0500`; its executable is mode `0500`;
the evidence, manifest, and index are each mode `0400`. All manifest file paths
are relative single components inside that run directory.

## Mandatory verification before remediation

Run this before every reboot, session change, driver repair, or later live
gate:

```console
cargo run --locked --bin replay-host-doctor -- verify-archive \
  --index artifacts/validation/g0/pre-reboot/index.json
```

The immediate post-archive result was exit `0`, `status: "verified"`,
`evidence_status: "fail"`, and the archived executable digest shown above.
The verifier reads and hashes only the immutable index and contained archived
files. It does not execute the archived binary and does not read the current
build target.

The structural regression is:

```console
cargo test --locked pre_reboot_archive_manifest_path_digest_schema -- --exact
```

It verifies the relative two-component manifest pointer, index-to-manifest
digest, schemas, provenance, FAIL status, and the complete offline archive
verification path.

## Permanent compatibility obligation

The two stable regressions are:

```console
cargo test --locked g0_v1_foundation_compat -- --exact
cargo test --locked original_pre_reboot_archive_compat -- --exact
```

`g0_v1_foundation_compat` decodes the original Plan 01-01 fixture, preserves
its unchanged V1 base, and proves the unknown `future-display-proof.v2` raw
payload and payload digest survive re-encoding. The original-archive test
reads only the immutable index, contained manifest, archived executable, and
archived evidence. It verifies their fixed digests, strict V1 base identity,
and every raw extension payload digest without executing the archived binary
or consulting `target/`.

Plans 01-06, 01-08, and 01-10 must run both named tests before their
integration/live acceptance. Any plan that changes the base V1 decoder or
archive verifier has the same obligation. Plans 01-05, 01-07, and 01-09 are
isolated pure/fixture expansions that do not change those surfaces and do not
rerun this pair.

Known-extension validators are additional checks; they may not replace either
the generic base/raw-extension compatibility test or the immutable archive
verification.
