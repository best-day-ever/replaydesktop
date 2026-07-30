---
phase: 01-host-readiness-gate
reviewed: 2026-07-30T00:50:11Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - artifacts/validation/g0/post-repair/index.json
  - artifacts/validation/g0/post-repair/run-73c9e241c73e6672cf8e6e3169b5d7dc8f77f7d809037c84cd46dbf28d8f93e3-2dc751130f3322db0b73a16fe0cb997d30a03a79be657498c3a73ad8fc58cfcc/manifest.json
  - artifacts/validation/g0/pre-reboot/index.json
  - artifacts/validation/g0/pre-reboot/run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/g0-evidence.json
  - artifacts/validation/g0/pre-reboot/run-a0d9c6cc52b55dfaa6f15a9a176be98ed7ac1691cca76913115aa0870cb2127f-5ee84e7214944d041b550319be80f85732609200ed0897eb71e3c3117f8b5b61/manifest.json
  - native/nvenc_abi_oracle.c
  - native/nvfbc_abi_oracle.c
  - native/nvml_abi_oracle.c
  - src/archive.rs
  - src/cli.rs
  - src/currentness.rs
  - src/digest.rs
  - src/evidence.rs
  - src/lib.rs
  - src/local_xorg.rs
  - src/main.rs
  - src/model.rs
  - src/native_nvenc.rs
  - src/native_nvfbc.rs
  - src/native_nvml.rs
  - src/nvenc_bitstream.rs
  - src/output_mapping.rs
  - src/probe.rs
  - tests/fixtures/current-wayland-driver-mismatch.json
  - tests/fixtures/g0-envelope-v1-foundation.json
  - tests/fixtures/host01-edge-cases.json
  - tests/fixtures/host01-session-spoofing.json
  - tests/fixtures/host02-output-topologies.json
  - tests/fixtures/host03-nvfbc-capture.json
  - tests/fixtures/host04-nvenc-tuples.json
  - tests/host04_bitstream.rs
  - tests/host04_copy_spike.rs
  - tests/host04_policy.rs
  - tests/host_doctor_cli.rs
  - tests/output_mapping.rs
  - tests/z_host03_capture_cli.rs
findings:
  critical: 8
  warning: 4
  info: 0
  total: 12
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-07-30T00:50:11Z  
**Depth:** standard  
**Files Reviewed:** 36  
**Status:** issues_found

## Summary

The readiness gate is not safe to ship. The live NVENC path can declare a complete policy while skipping AV1 and four non-NV12 positions, and its stream proof accepts undecodable header fragments. The display discovery path rejects valid target distributions and contains two memory-safety violations. Capture cleanup failures can also be hidden from the caller.

The submitted quality gates are red:

- `cargo fmt --check` passed.
- `cargo clippy --locked --all-targets -- -D warnings` failed on three dead-code errors.
- `cargo test --locked --all-targets` failed in `native_nvml::tests::nvml_source_abi_authorized_header_matches_complete_rust_contract`.
- `cargo test --locked --test host_doctor_cli` failed six tests because diagnostic runs exited `74` (`EVIDENCE_PERSISTENCE`) instead of producing durable G0 FAIL evidence.
- The focused HOST-02, HOST-03, HOST-04 policy, copy, and parser integration tests passed, but several of those positives encode the incorrect behavior identified below.

The supplied `target/g0-post-reboot-output.json` was excluded because `/target/` is ignored by `.gitignore`.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Build-dependent NVML validation makes diagnostic evidence impossible to persist

**Classification:** BLOCKER  
**File:** `src/native_nvml.rs:274-283`  
**Also affected:** `src/native_nvml.rs:605-615`, `tests/host_doctor_cli.rs:1804-1861`, `tests/host_doctor_cli.rs:1919-1951`, `src/native_nvml.rs:1121-1129`

**Issue:** `NvmlEvidenceV1::is_valid` accepts populated source identity and digest fields only when the current binary was compiled with `replay_nvml_source` and those values equal that build's metadata. Normal no-SDK builds return `None` from `compiled_source_metadata`, so the project's diagnostic fixtures produce a semantically rejected host-foundation record. `persist_and_readback` consequently returns `EVIDENCE_PERSISTENCE` before writing evidence. This is directly reproducible: six `host_doctor_cli` cases exit `74`, and the unconditional no-SDK unit test also panics.

Diagnostic fixture authority and live source authority need distinct types. A diagnostic command must remain able to record a bounded, non-authoritative mismatch without requiring an operator SDK at build time.

**Fix:** Add an explicit NVML provider/provenance field (for example `DiagnosticFixture`, `LiveUnavailable`, `SourceAuthenticated`) and validate each form separately. Permit only the fixed fixture source contract for diagnostic envelopes, while requiring compiled metadata for live authenticated evidence and never letting fixture evidence produce live PASS. Gate the source-ABI positive test with `#[cfg(replay_nvml_source)]` and add a no-source diagnostic persistence test.

### CR-02: The live policy always marks AV1 generation-ineligible without probing it

**Classification:** BLOCKER  
**File:** `src/native_nvenc.rs:203-208`  
**Also affected:** `src/native_nvenc.rs:742-749`, `src/native_nvenc.rs:927-938`, `src/model.rs:1202-1213`

**Issue:** The only live entry point hardcodes `NvencGpuGenerationV1::Unknown`. `Unknown.supports_av1_encode()` is false, so both AV1 policy positions become `GenerationIneligible` and the native provider is never invoked—even on Ada or Blackwell. `GenerationIneligible` is also non-blocking, so final admission can PASS while those hardware-eligible tuples were never attempted.

**Fix:**

```rust
let generation = authenticated_nvml_generation_for_selected_gpu()?;
evaluate_nvenc_policy(generation, &mut provider)
```

If generation cannot be proven, do not equate unknown with ineligible. Either attempt the authoritative NVENC codec/open probe or keep admission non-passing until generation is known. Add a live-entry regression asserting that Ada invokes both AV1 positions.

### CR-03: Four of seven codec positions are rejected before any native attempt

**Classification:** BLOCKER  
**File:** `src/native_nvenc.rs:259-273`

**Issue:** `probe_nvenc_tuple_with_runtime` immediately returns `Unsupported` for every buffer format other than NV12. HEVC Main10 4:2:0, HEVC 4:4:4 8-bit, HEVC 4:4:4 10-bit, and AV1 Main 10-bit therefore can never be advertised on any GPU. This is a limitation of the implementation, not an observed hardware capability, yet `Unsupported` is terminal and non-blocking. In combination with CR-02, a live run can exercise only H.264 and HEVC 8-bit 4:2:0 and still claim the closed policy completed.

**Fix:** Acquire or produce the exact same-GPU input format for each position, including NvFBC YUV444P and bounded device-only conversions for 10-bit surfaces; verify allocation/pitch/copy edges; then call the native bridge. Only return `Unsupported` after an actual capability/open result from the target GPU. Unknown or unimplemented format plumbing must block admission.

### CR-04: Header-only, undecodable streams count as successful encoded keyframes

**Classification:** BLOCKER  
**File:** `src/nvenc_bitstream.rs:134-177`  
**Also affected:** `src/nvenc_bitstream.rs:308-315`, `src/nvenc_bitstream.rs:317-363`, `src/nvenc_bitstream.rs:487-498`, `src/nvenc_bitstream.rs:500-577`, `tests/host04_bitstream.rs:91-93`, `tests/host04_bitstream.rs:123-125`, `tests/host04_bitstream.rs:168-172`

**Issue:** H.264 admission requires only an SPS and three Exp-Golomb values from an IDR slice; it never requires a PPS, a matching PPS/SPS reference, or slice payload. HEVC similarly accepts SPS plus a few IDR header bits without VPS/PPS or payload. AV1 accepts a sequence header and the first frame-header bits without any tile/frame data. The positive tests deliberately construct these truncated byte sequences and pass them. `run_nvenc_on_capture_lease` turns that result into `NvencAttemptOutcomeV1::Success`, so a truncated, non-decodable output can be advertised.

**Fix:** Validate a complete independently decodable access unit. At minimum require and cross-reference all mandatory parameter sets and prove non-empty complete slice/tile payload; preferably pass the bounded one-frame output through the pinned FFmpeg parser/decoder. Replace the synthetic positives with real encoder-produced fixtures and add truncation tests after every parameter-set, slice-header, and tile boundary.

### CR-05: Capture cleanup/containment failure is ignored after the consumer returns

**Classification:** BLOCKER  
**File:** `src/native_nvfbc.rs:239-267`  
**Also affected:** `src/native_nvfbc.rs:2244-2270`, `src/native_nvenc.rs:274-280`

**Issue:** `with_live_selected_capture_lease` stores `consumed = Some(value)` inside the callback before NvFBC/CUDA cleanup. It then returns that value solely because it is present, without checking `observation.failure`. If NVENC requests containment, `capture_with_nvfbc` reports `CleanupUncertain` and deliberately leaks/contains native state, but the wrapper still returns `Ok(run)`. Cleanup errors occurring after the callback are hidden the same way. The policy loop can continue issuing native calls in a contaminated worker.

**Fix:**

```rust
if let Some(failure) = observation.failure {
    return Err(failure);
}
if !cleanup_is_complete_live(&observation.lifecycle) {
    return Err(CaptureFailureV1::CleanupUncertain);
}
consumed.ok_or(CaptureFailureV1::WorkerRejected)
```

Propagate containment as a distinct terminal result and abort the remaining policy attempts in that worker. Add fault injection after every post-callback cleanup operation and assert no subsequent native attempt occurs.

### CR-06: Zero-connector DRM responses invoke undefined behavior

**Classification:** BLOCKER  
**File:** `src/output_mapping.rs:1963-1968`  
**Also affected:** `src/output_mapping.rs:2064-2071`

**Issue:** The code permits null C array pointers when the corresponding count is zero, then passes those null pointers to `std::slice::from_raw_parts`. Rust requires a non-null, aligned pointer even for a zero-length slice. A valid DRM card with zero connectors or a connector with zero properties can therefore trigger undefined behavior in a normal diagnostic collection path.

**Fix:**

```rust
let connector_ids = if count == 0 {
    Vec::new()
} else {
    if resources.connectors.is_null() {
        return Err(());
    }
    unsafe { std::slice::from_raw_parts(resources.connectors, count) }.to_vec()
};
```

Apply the same zero-count branch to both `props` and `prop_values`, and cover null-plus-zero fake-libdrm responses under Miri or a native fixture.

### CR-07: Valid Ubuntu and Rocky Xorg servers are rejected by one Arch-specific path

**Classification:** BLOCKER  
**File:** `src/local_xorg.rs:957-977`

**Issue:** A peer is classified as Xorg only when `/proc/<pid>/exe` equals exactly `/usr/lib/Xorg`. The project's required qualification targets include Ubuntu 24.04 and Rocky 9, whose packaged Xorg executable locations differ (commonly `/usr/lib/xorg/Xorg` and `/usr/libexec/Xorg`). Those valid physical X11 sessions are classified `Other`, causing `X11_PEER_NOT_XORG` before capture can start.

**Fix:** Resolve trusted executable identities from the supported distributions' package-owned paths, then compare the open executable's `(device, inode)` and ownership/mode against those trusted candidates. Keep the root-owned/non-writable checks and reject basename-only matches. Add one fixture/integration case per supported distribution path.

### CR-08: Absence of the optional RandR `non-desktop` property means “non-desktop”

**Classification:** BLOCKER  
**File:** `src/output_mapping.rs:835-839`  
**Also affected:** `src/output_mapping.rs:1021-1035`

**Issue:** `output_property_u32` correctly returns `None` when the optional property is absent, but the caller evaluates `None != Some(0)` as true. A normal physical output on a driver that does not expose `non-desktop` is therefore discarded as non-desktop, producing a false HOST-02 failure. The DRM implementation in the same file correctly defaults an absent property to false, so the two collectors disagree.

**Fix:**

```rust
let non_desktop =
    output_property_u32(connection, output_xid, atoms.non_desktop)?
        .unwrap_or(0) != 0;
```

Add collector tests for absent, zero, one, malformed, and multi-value property replies.

## Warnings

### WR-01: Archive commit failures strand an immutable run that cannot be retried

**Classification:** WARNING  
**File:** `src/archive.rs:381-445`

**Issue:** The run directory is renamed and fsynced before index creation begins. Any failure creating, writing, sealing, renaming, or fsyncing the index returns an error but leaves the committed mode-0500 run directory behind. Retrying the same evidence reaches the run-directory `NOREPLACE` collision, while no authoritative index exists. A process crash in the same window has the same result.

**Fix:** Make this state recoverable: create and fsync the index staging file before committing the run, then on retry detect an exact digest-matching committed run and finish its index atomically. Alternatively keep a durable transaction marker and implement narrowly scoped recovery. Test injected failure/crash at every boundary between run rename and index fsync.

### WR-02: The safe lease callback can return a dangling raw native lease and is not panic-safe

**Classification:** WARNING  
**File:** `src/native_nvfbc.rs:19-56`  
**Also affected:** `src/native_nvfbc.rs:219-225`

**Issue:** The callback receives `NativeCaptureLease` by value, and generic `R` may itself be that lease. Safe crate code can therefore return/store raw CUDA context and device-pointer getters after the callback's allocation and context have been freed. A panic in the callback also bypasses the manual cleanup sequence.

**Fix:** Pass a lifetime-bound borrowed lease through a higher-ranked callback whose return type cannot contain the borrow, expose narrowly scoped operations instead of raw handles where possible, and put every native acquisition behind RAII cleanup guards. Contain callback panics at the worker boundary while guards unwind.

### WR-03: BGRA byte stride is validated as a pixel stride

**Classification:** WARNING  
**File:** `src/native_nvfbc.rs:3016-3061`

**Issue:** `pitch_bytes` and `stride_bytes` are byte counts, but BGRA validation requires only `pitch >= width` and computes plane size as `stride * height * 4`. A tightly packed 3840-wide BGRA row should have a 15360-byte stride and plane size `stride * height`; the current validator rejects that valid shape while accepting a 3840-byte row that cannot hold the pixels.

**Fix:** Compute the per-format minimum row bytes with checked arithmetic (`width * 4` for BGRA), require `stride >= minimum_row_bytes`, and calculate every packed plane as `stride_bytes * plane_height` without multiplying the byte stride by bytes-per-pixel again.

### WR-04: The required warnings-denied quality gate cannot pass in a no-SDK build

**Classification:** WARNING  
**File:** `src/native_nvenc.rs:121-123`  
**Also affected:** `src/native_nvenc.rs:238-247`, `src/native_nvfbc.rs:270-278`

**Issue:** `cargo clippy --locked --all-targets -- -D warnings` fails because `requested_output`, `unsupported_attempt`, and the no-source `with_live_selected_capture_lease` stub are dead in the default configuration. This is the exact phase verification command, so the submitted implementation does not satisfy its own static gate.

**Fix:** Apply precise `#[cfg(replay_nvenc_source)]` / `#[cfg(replay_nvfbc_source)]` gating to fields, helpers, and implementations that exist only in authenticated-source builds. Keep both source-enabled and source-disabled clippy lanes in CI; do not suppress the warnings globally.

---

_Reviewed: 2026-07-30T00:50:11Z_  
_Reviewer: the agent (gsd-code-reviewer)_  
_Depth: standard_
