# NVENC Copy-Boundary Spike

**Plan:** 01-09  
**Status:** contract frozen; native execution remains owned by Plan 01-10  
**Scope:** one application-owned CUDA surface from the proven HOST-03 lease,
through NVENC registration, mapping, input, preprocessing, encode completion,
and cleanup

## Decision

`CopyBoundaryProofV1` is the only D-05 admission predicate for the
capture-to-encoder boundary. It is fail-closed:

- `PASS` means every required application and encoder-internal edge has one
  closed typed classification, every application-visible identity/format/
  extent/pitch/plane fact agrees, the encoder session is on the selected CUDA
  device and context, no host staging or cross-GPU transfer occurred, and
  cleanup completed in reverse ownership order.
- `BLOCKED_UNKNOWN` means at least one required edge is missing, mismatched,
  host-staged, cross-GPU, described only by opaque-handle equality, or depends
  on an undocumented or unobserved encoder-internal conversion/copy.

Matching opaque handles or matching configuration values are not surface
identity evidence. `NV_ENC_REGISTERED_PTR`, `NV_ENC_INPUT_PTR`, and bitstream
buffer values are encoder-owned capabilities. Their association is proven by
the documented call chain and successful statuses, never by comparing their
numeric bits to the CUDA allocation address.

## Source separation

This spike uses **Video Codec SDK 13.1** semantics as a standalone proof
contract. It does not modify, vendor into, or claim compatibility for Kyber's
pinned `nv-codec-headers n12.1.14.0` / FFmpeg 8.1 tree. The two inputs have
different jobs:

| Input | Purpose | Authority |
| --- | --- | --- |
| NVIDIA Video Codec SDK 13.1 `nvEncodeAPI.h` and programming guide | Standalone HOST-04 native ABI, resource, copy, and capability proof in Plan 01-10 | Authenticated operator-supplied source plus compiled ABI oracle |
| Kyber/Kymedia `nv-codec-headers n12.1.14.0` | Later reproducible Kyber integration | Existing pinned project stack; untouched by this spike |

Plan 01-10 may translate the proven result into a narrow Kyber patch only
after it authenticates both source identities independently. A passing SDK
13.1 probe does not rewrite the Kyber pin.

## Source and observability matrix

Classification vocabulary:

- `source-guaranteed`: the authenticated SDK 13.1 source or official guide
  defines the relation.
- `runtime-queryable`: the application can check the fact in the live call
  sequence or through the CUDA Driver API.
- `optional-read-only-tooling`: a profiler may corroborate the fact, but the
  tool is not an admission dependency.
- `unknown`: the required relation is neither guaranteed nor observed and
  therefore produces `BLOCKED_UNKNOWN`.

| Fact or edge | Required evidence | Classification | Admission rule |
| --- | --- | --- | --- |
| HOST-03 application allocation | Current `CaptureFrameLeaseV1` surface token, CUDA allocation identity, device, context, format, extent, pitch, planes, and completed capture copy | `runtime-queryable` | Exact lease must be current and owned by the application |
| CUDA pointer ownership | `CU_POINTER_ATTRIBUTE_DEVICE_POINTER`, device ordinal, context, memory type, and allocation range for the registered pointer | `runtime-queryable` | Pointer range must contain every declared plane and belong to the selected GPU/context |
| Encoder session device | `NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS` uses the same current CUDA context and CUDA device type | `source-guaranteed` + `runtime-queryable` | Session creation must succeed on the lease context; ordinal/name matching alone is insufficient |
| Application surface registration | `NV_ENC_REGISTER_RESOURCE` is populated with `CUDADEVICEPTR`, the exact lease pointer, width, height, pitch, and one closed `NV_ENC_BUFFER_FORMAT`, then passed to `NvEncRegisterResource` | `source-guaranteed` + `runtime-queryable` | Successful call plus exact submitted values closes the no-copy registration edge |
| Registered resource association | Successful registration returns an opaque `registeredResource` associated with the submitted external buffer | `source-guaranteed` | Opaque value is recorded only as acquired/not-acquired, never as pointer identity |
| Registered-to-mapped resource | `NvEncMapInputResource` consumes that acquired resource and returns one opaque `mappedResource`; returned `mappedBufferFmt` equals the requested tuple format | `source-guaranteed` + `runtime-queryable` | Successful map and exact format are mandatory |
| Mapped resource used for input | `NV_ENC_PIC_PARAMS::inputBuffer` is the mapped resource from the same attempt; width/height/pitch/format remain exact | `source-guaranteed` + `runtime-queryable` | The application records one nonce-bound call edge |
| Pitch-linear preprocessing | SDK 13.1 documents traditional `CUDADEVICEPTR` input as a pitch-linear to block-linear preprocessing copy performed by an SM kernel | `source-guaranteed` | Record one same-GPU encoder-internal copy; claiming zero internal copies is invalid |
| Preprocessing corroboration | Nsight Systems NVENC/CUDA events for the same process and attempt | `optional-read-only-tooling` | Useful corroboration only; never substitutes for source/runtime proof |
| Unlisted internal conversion/copy | Any conversion or copy required by mismatched profile/chroma/depth/format, or any driver behavior outside the authenticated contract | `unknown` | `BLOCKED_UNKNOWN` |
| Encode input consumption | One synchronous Linux `NvEncEncodePicture` followed by blocking `NvEncLockBitstream` yields a non-empty keyframe bitstream for the attempt | `source-guaranteed` + `runtime-queryable` | Parsed bytes, not status or provider claims, establish the tuple |
| Host staging | CPU-visible input or intermediate staging, DtoH/HtoD transfer, or system-memory registration | `runtime-queryable` | Always `BLOCKED_UNKNOWN`; HOST-04 has no host-staged success mode |
| Cross-GPU transfer | Registration/session devices differ or a peer edge appears | `runtime-queryable` | Always `BLOCKED_UNKNOWN` |
| Cleanup | bitstream unlock/destroy, input unmap/unregister, encoder destroy, and application allocation ownership returned exactly once | `runtime-queryable` | Every acquired resource receives one successful inverse release |

Primary source:
[NVIDIA Video Codec SDK 13.1 NVENC programming guide](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.1/nvenc-video-encoder-api-prog-guide/index.html),
especially “Input buffers allocated externally,” “Synchronous Mode,” and the
SDK 13.1 “Traditional vs CUDA Array Input” comparison. The latter explicitly
classifies the pitch-linear `CUdeviceptr` preprocessing copy; this project
therefore does not call that edge zero-copy.

## Resource and ownership graph

The graph is one-attempt and nonce-bound. Every node is either an
application-owned resource, a borrowed provider capability, or an
encoder-internal typed surface whose concrete handle is deliberately opaque.

```text
selected CUDA device/context
  owns
CaptureFrameLeaseV1 application CUDA allocation (pitch-linear)
  -- E1 register/no copy -->
NV_ENC_REGISTERED_PTR (borrowed opaque capability)
  -- E2 map/no copy -->
NV_ENC_INPUT_PTR (borrowed opaque capability)
  -- E3 submit exact mapped input -->
NVENC pitch-linear input
  -- E4 source-guaranteed same-GPU SM preprocessing copy -->
NVENC internal block-linear surface (opaque identity)
  -- E5 hardware read -->
NVENC codec engine
  -- E6 encoded output -->
NV_ENC_OUTPUT_PTR
  -- E7 blocking lock/copy-to-parser -->
bounded application bitstream bytes + SHA-256
```

Required reverse release:

```text
NvEncUnlockBitstream
NvEncDestroyBitstreamBuffer
NvEncUnmapInputResource
NvEncUnregisterResource
NvEncDestroyEncoder
return CaptureFrameLeaseV1 application allocation ownership
```

No failure path may invent release success. A worker-process timeout contains
native state, but the attempt remains rejected with incomplete cleanup.

## Executable `CopyBoundaryProofV1` predicate

The future model evaluates these booleans and typed edges; provider text and
free-form claims are not inputs:

```text
PASS =
  current_live_capture_lease
  AND lease_allocation_range_verified
  AND selected_gpu_and_context_match
  AND registration_exact_and_successful
  AND mapping_exact_and_successful
  AND mapped_input_submitted_once
  AND application_edges == [
        register-no-copy,
        map-no-copy,
        submit-no-copy
      ]
  AND encoder_internal_edges == [
        same-gpu-pitch-linear-to-block-linear-copy
      ]
  AND host_staging_edges == 0
  AND cross_gpu_edges == 0
  AND no_unknown_application_edge
  AND no_undocumented_or_unobserved_encoder_internal_conversion_or_copy
  AND nonempty_keyframe_bitstream
  AND parsed_tuple_equals_attempt_tuple
  AND cleanup_complete

otherwise BLOCKED_UNKNOWN
```

The following are terminal `BLOCKED_UNKNOWN` cases:

1. host staging is observed or cannot be excluded;
2. CUDA allocation/device/context/range, format, extent, pitch, or planes
   mismatch;
3. an unobserved application edge exists between lease, registration, mapping,
   and encode input;
4. a cross-GPU or peer edge exists;
5. the application supplies pitch-linear input but claims no internal copy;
6. an encoder-internal conversion/copy is undocumented or unobserved;
7. encoded bytes are empty, non-keyframe, malformed, or parse to another tuple;
8. any acquired native resource lacks one truthful successful inverse release.

## Fixture status

`tests/fixtures/host04-nvenc-tuples.json` is diagnostic-only. Its closed case
tests the predicate shape; its unsafe cases ensure host staging, mismatches,
unknown application edges, and unknown encoder-internal edges block. Fixture
data never authorizes a live native session, advertisement, HOST-04 PASS, or G0
PASS.

## Exact Plan 01-10 live command

Plan 01-10 must first authenticate the operator-supplied SDK 13.1 header root
and preserve the already authenticated NvFBC/CUDA/NVML roots. The operator sets
`REPLAY_NVENC_SDK_ROOT` to the absolute directory containing
`nvEncodeAPI.h`; the executable and evidence record its exact digest.

```bash
test -f "${REPLAY_NVENC_SDK_ROOT:?set absolute SDK 13.1 Interface path}/nvEncodeAPI.h"
test -f "${REPLAY_NVFBC_SDK_ROOT:?set authenticated Capture SDK include path}/NvFBC.h"
test -f "${REPLAY_CUDA_SDK_ROOT:?set CUDA Driver API include path}/cuda.h"
test -f "${REPLAY_NVML_SDK_ROOT:?set NVML include path}/nvml.h"

cargo run --locked -- run \
  --output DP-0.3 \
  --evidence target/g0-host04-live.json \
  --probe-timeout-ms 15000

cargo run --locked -- verify-evidence \
  --evidence target/g0-host04-live.json \
  --require-host01 pass \
  --require-host02 pass \
  --require-host03 pass \
  --require-host04 pass \
  --validate-extension nvenc-tuples.v1
```

Until Plan 01-10 supplies the authenticated live provider, the same production
binary must report HOST-04 `UNPROVEN`, overall G0 `FAIL`, and no advertised
tuple.
