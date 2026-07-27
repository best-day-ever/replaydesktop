# ReplayDesktop host readiness doctor

`replay-host-doctor` is the first ReplayDesktop walking skeleton. It performs
bounded, read-only host observations, derives one fail-closed G0 decision,
writes one versioned evidence envelope, reads those exact bytes back, and
verifies that the evidence belongs to the current run.

The host-foundation probe proves or rejects the local Xorg session and exact
NVIDIA kernel/NVML userspace identity. Selected output mapping is available
only through explicit, measured selection; omission enters discovery and never
chooses a default. NvFBC capture and NVENC tuples remain `UNPROVEN`, so the
current command still cannot produce G0 PASS.

## Run the doctor

From the repository root, use the locked toolchain and lockfile:

```console
cargo run --locked --bin replay-host-doctor -- run --evidence target/g0-evidence.json
```

Until later host probes are implemented, a valid run prints one
`replaydesktop.host-doctor-result.v1` JSON object to stdout, persists
`target/g0-evidence.json`, and returns exit 2. The JSON includes its fresh
`run_id`, live provenance, the overall `fail` status, and stable ordered
extension reasons.

To verify the exact evidence immediately, copy the `run_id` from that result:

```console
cargo run --locked --bin replay-host-doctor -- verify-evidence --evidence target/g0-evidence.json --run-id <RUN_ID>
```

Verification is read-only. It strictly decodes V1, checks extension payload
digests, requires the requested run ID, and rechecks the current boot, process
session, executable digest, and time bounds. A successful verification prints
one result object and returns exit 0; it verifies integrity and currentness,
not readiness.

## Discover and select one physical output

Output discovery uses the same bounded production worker as an explicit run.
It queries XRandR but makes no selection, writes a `selected-output.v1`
discovery payload, and exits 2:

```console
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo run --locked --bin replay-host-doctor -- run \
    --evidence target/g0-post-reboot-output-discovery.json
```

Enumerate the exact UTF-8 XRandR candidate names from persisted evidence:

```console
jq -r '.extensions[]
  | select(.id == "selected-output.v1")
  | .payload.candidates[]
  | select(.display != null)
  | .display' target/g0-post-reboot-output-discovery.json
```

The operator must choose one exact candidate; the doctor never selects by
position, display order, or an inferred default:

```console
export REPLAY_HOST_OUTPUT='DP-0'
```

Run the full read-only relation with that exact name. A successful HOST-02
proof still exits 2 because later media gates remain open:

```console
set +e
REPLAY_NVML_SDK_ROOT=/opt/cuda/targets/x86_64-linux/include \
  cargo run --locked --bin replay-host-doctor -- run \
    --output "$REPLAY_HOST_OUTPUT" \
    --evidence target/g0-post-reboot-output.json
doctor_status=$?
set -e
test "$doctor_status" -eq 2

cargo run --locked --bin replay-host-doctor -- verify-evidence \
  --evidence target/g0-post-reboot-output.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 unproven \
  --require-host04 unproven \
  --validate-extension selected-output.v1
```

The admitted extension records the exact XRandR output/CRTC/mode/provider,
active timing and origin, an in-memory EDID SHA-256, the unique DRM connector,
canonical PCI BDF, matching NVML UUID, and before/after topology digests.
Missing or ambiguous relations, malformed observations, unsupported
clone/MST/PRIME layouts, or a topology change produce a typed HOST-02 failure.
No raw EDID bytes, native errors, SDK paths, or inherited secrets are
persisted.

## Preserved pre-reboot failure

The original Wayland and NVIDIA version-mismatch state is preserved under
`artifacts/validation/g0/pre-reboot/`. Its create-once index points only to a
contained manifest, which in turn binds the exact archived executable,
evidence bytes, V1 identity, and raw extension digests.

Before any reboot, session change, driver remediation, or later live gate,
verify that immutable archive:

```console
cargo run --locked --bin replay-host-doctor -- verify-archive \
  --index artifacts/validation/g0/pre-reboot/index.json
```

Verification must report `status: "verified"`, the preserved run
`run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f`,
and archived binary digest
`9662e8ee6a8196ee81f2011ce60fe6aba9712db7960e3640c19ba22fe01e677e`.
It reads the immutable index and contained archive files only; it neither
executes the archived binary nor consults `target/`.

The exact capture transcript, expected exit status, containment checks, and
recorded host facts are in
`.planning/phases/01-host-readiness-gate/01-VALIDATION.md`.

### Permanent compatibility gates

The original decoder fixture and the original live archive are permanent
compatibility inputs:

```console
cargo test --locked g0_v1_foundation_compat -- --exact
cargo test --locked original_pre_reboot_archive_compat -- --exact
```

Plans 01-06, 01-08, and 01-10 must run both named tests at their
integration/live checkpoints. Any plan that changes the base V1 decoder or
the archive verifier must also run both. Plans 01-05, 01-07, and 01-09 add
isolated pure/fixture modules without changing those surfaces, so they do not
rerun this compatibility pair.

Known-extension validators are additional checks; they may not replace the
generic V1 base/raw-extension regression or the original offline archive
verification.

## Diagnostic fixtures

Fixtures exercise the same worker, evaluator, persistence, and readback path:

```console
cargo run --locked --bin replay-host-doctor -- diagnose \
  --fixture tests/fixtures/host01-edge-cases.json \
  --fixture-case positive \
  --evidence target/g0-diagnostic-evidence.json
```

Fixture observations always have `diagnostic` provenance. Even an all-positive
fixture remains `UNPROVEN`, returns exit 2, and cannot be admitted as live G0
evidence. The fixture matrix also covers empty and duplicate observations,
timeouts, malformed or oversized responses, abnormal workers, injected
authority, stale-cache claims, Unicode, and secret/path/native-error
sentinels.

## Exit taxonomy

| Code | Meaning |
|---:|---|
| exit 0 | Command or exact-run verification succeeded. |
| exit 2 | The command completed normally and G0 is `FAIL`. |
| exit 64 | Command-line usage was invalid; no probe or evidence write occurred. |
| exit 70 | An internal hidden worker request failed. |
| exit 74 | Evidence persistence, strict decoding, identity, digest, or currentness verification failed. |

Failures outside a normal G0 decision use bounded JSON error envelopes on
stderr. Raw child stderr, inherited environment values, native error strings,
and operator paths are not copied into public output or persisted evidence.

## Evidence and currentness guarantees

Every run creates its identity in the parent before examining the destination.
The identity binds the current boot and process session, exact argv,
wall-clock and monotonic start/finish times, and SHA-256 of the executing
binary. Runs have bounded per-probe deadlines and a five-minute total
currentness window.

Evidence is serialized and strictly decoded before writing. The store creates
a mode `0600` temporary sibling with create-new and no-follow flags, flushes
and fsyncs it, atomically renames it over the requested regular destination,
fsyncs the parent directory, then reopens without following symlinks and
requires the exact run and byte digest. Parent directories are never created
implicitly.

The probes have a read-only/no-remediation contract. They do not install
packages, change permissions, alter sessions, load modules, reboot, download
sources, accept licenses, capture frames, invoke encoders, or contact a
network. The requested evidence file and its owned same-directory temporary
sibling are the only writes.

## Open native blockers

This skeleton reports rather than hides the remaining work:

- remediate the rejected Wayland session and exact NVIDIA
  `610.43.02`/`610.43.03` kernel/userspace mismatch, then prove the repaired
  local physical Xorg/NVML foundation;
- on that corrected host, explicitly select and prove the physical output to
  clear HOST-02;
- prove one NvFBC shared-CUDA capture with the required format and cleanup for
  HOST-03;
- open the required NVENC codec/chroma tuples for HOST-04 and record their exact
  capabilities.

Even after HOST-01 and HOST-02 pass, HOST-03 and HOST-04 remain `UNPROVEN`, so
the overall G0 remains FAIL. Those media proofs belong to later Phase 1 plans.
Diagnostic fixtures, cached evidence, and child-supplied claims can never
substitute for them.
