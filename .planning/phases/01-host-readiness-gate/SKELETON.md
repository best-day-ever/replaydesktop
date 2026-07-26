# Walking Skeleton — ReplayDesktop

**Phase:** 1  
**Generated:** 2026-07-27

## Capability Proven End-to-End

An operator can run `replay-host-doctor`, collect bounded read-only host
observations, receive one fail-closed G0 decision, atomically persist a
versioned envelope, read back that exact run, and verify immutable evidence
without trusting a rebuilt target binary.

## Systems-CLI Walking-Skeleton Adaptation

ReplayDesktop’s first interaction is a systems CLI. The command is the
interaction layer, bounded source-backed Linux probes are the backend,
`G0EvidenceEnvelopeV1` plus SHA-256 identities are persistence, and the locked
Cargo command is development deployment. The ten Phase 1 plans grow this one
path without changing its V1 base decoder.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Toolchain | Rust `1.89.0`, committed `Cargo.lock`, and `cargo ... --locked` | Matches upstream and makes builds/review reproducible. |
| Audited dependencies | `serde 1.0.229` derive, `serde_json 1.0.151` raw-value support, `sha2 0.11.0`, `libloading 0.9.0`, `rustix 1.1.4` fs/process/net, `x11rb 0.14.0` RandR plus demonstrated features | One reviewed direct set supports strict JSON, exact unknown-extension preservation, hashing, native loading, OS primitives, and X11 queries. |
| Evidence envelope | Final strict `G0EvidenceEnvelopeV1` base plus bounded digest-bearing raw extension records | Later facts are forward-compatible records, never new required base fields. |
| Extension ownership | `host-foundation.v1`, `selected-output.v1`, `nvfbc-capture.v1`, `nvenc-tuples.v1` | Generic decoding preserves unknown records; each gate strictly validates only its known payload. |
| Local-Xorg authority | Active local non-remote logind session + AF_UNIX + SO_PEERCRED + real `/proc/<pid>/exe` Xorg + x11rb setup/RandR | Environment variables and Xwayland cannot prove the physical Xorg session. |
| Native-source boundary | Explicit operator roots, separate lawful-source assertion, narrow C oracles, then `libloading` runtime sequences | Runtime symbols cannot define unsafe declarations or imply license/source authenticity. |
| Evidence identity | `Sha256DigestV1`, `sha256_bytes`, `sha256_reader`, `sha256_file` backed by `sha2` | Every binary, evidence, source, payload, EDID, manifest, and archive identity shares one tested API. |
| Persistence | Strict decoding, duplicate-key rejection, bounded raw payloads, mode-0600 same-directory atomic write/fsync/rename/dir-fsync, exact-run readback | Malformed, partial, stale, or ambiguous evidence fails closed. |
| Immutable history | Create-once run directory containing exact `/proc/self/exe` copy, exact evidence, and completed-last manifest | Verification reads archived bytes and survives later rebuilds. |
| Time bounds | Per-native-call deadlines below bounded worker deadlines; separate realistic live workflow timeout | One driver call cannot hang the doctor and compilation/fixtures do not consume live budget. |
| Trust model | Live PASS requires current boot/session, sources, binary, output/GPU, capture, copy, tuples, cleanup, and readback; fixtures remain diagnostic | Cached/injected/partial facts cannot become G0 evidence. |
| Development deployment | `cargo run --locked --bin replay-host-doctor -- run --evidence target/g0-evidence.json` | Exercises CLI → bounded observations → decision → atomic V1 JSON → readback. |

## Walking-Skeleton Artifacts

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `DEPENDENCIES.md`,
  `deny.toml`
- `src/main.rs`, `src/cli.rs`, `src/lib.rs`, `src/model.rs`,
  `src/digest.rs`, `src/evidence.rs`, `src/currentness.rs`, `src/probe.rs`
- final V1 foundation fixture and HOST-01 through HOST-04 diagnostic fixtures
- local-Xorg/NVML, output mapping, NvFBC/CUDA, and NVENC provider modules
- output/GPU and NVENC copy-boundary spike artifacts
- immutable pre-reboot and separate post-repair contained archives
- README and `01-VALIDATION.md`

## Phase 1 Slices

1. Plan 01-01 freezes dependencies, digest, final V1 base, bounded extensions,
   and the permanent foundation fixture.
2. Plan 01-02 ships the CLI, workers, fixtures, atomic store/readback, and
   correct development command.
3. Plan 01-03 adds local-Xorg and source/runtime NVML proof.
4. Plan 01-04 archives the exact current failing binary/evidence before repair.
5. Plan 01-05 creates the mapping spike and pure output/GPU contract.
6. Plan 01-06 adds `--output`, `selected-output.v1`, and real post-reboot
   HOST-01/HOST-02 proof.
7. Plan 01-07 creates NvFBC/CUDA source/ABI, fixture, frame/ledger, cursor,
   driver, and cleanup contracts.
8. Plan 01-08 crosses the operator gate and proves one live selected-output
   frame; G0 remains failed for HOST-04.
9. Plan 01-09 creates the copy spike and closed H.264 High/HEVC/AV1 policy plus
   bounded bitstream fixtures.
10. Plan 01-10 authenticates SDK 13.1, runs live exact tuple probes, verifies
    original history, decides final G0, and archives current evidence.

## Explicit Boundaries

- No database, GUI, service, remote transport, TLS session, or Kyber media
  session is part of this local readiness command.
- No fallback capture backend, software encoder, silent tuple downgrade, or
  cached/fixture evidence can satisfy a live gate.
- The doctor does not reboot, change login/session, install drivers/packages,
  alter permissions, load modules, download gated sources, or accept licenses.
- The standalone SDK 13.1 HOST-04 probe does not rewrite or claim compatibility
  for Kyber’s later pinned `nv-codec-headers n12.1.14.0` integration.
- Streaming, sustained 4K60 telemetry, macOS behavior, input, and clipboard
  begin only after archived live G0 PASS.

## Validation Status

This file records the architecture and commands; it does not claim G0 PASS.
The original live FAIL must be archived before repair. Final PASS remains
blocked until current HOST-01 through HOST-04, extension/base integrity,
source, copy, stream, cleanup, readback, and separate current archive checks
all succeed.
