# Phase 01 Plan Outline — Host Readiness Gate

| Plan ID | Objective | Wave | Depends On | Requirements | Files |
|---|---|---:|---|---|---:|
| 01-01 | Freeze the audited Rust/dependency foundation, shared digest API, and final forward-compatible `G0EvidenceEnvelopeV1` base plus bounded raw-preserving extensions (D-01, D-02, D-09). | 1 | — | HOST-01 | 9 |
| 01-02 | Ship the doctor CLI, bounded workers, diagnostic admission policy, currentness, atomic evidence persistence/readback, process fixtures, and valid development command. | 2 | 01-01 | HOST-01 | 9 |
| 01-03 | Prove local Xorg through logind/AF_UNIX/SO_PEERCRED/real-Xorg/x11rb and source-gate the complete live NVML sequence (D-01, D-02, D-06–D-08). | 3 | 01-02 | HOST-01 | 9 |
| 01-04 | Archive the fresh current-host FAIL with the exact `/proc/self/exe` copy, exact V1 evidence, contained manifest, create-once integrity, and permanent compatibility regressions (D-06, D-09). | 4 | 01-03 | HOST-01 | 8 |
| 01-05 | Create the output/GPU mapping spike first, then implement the pure unique XRandR/EDID/DRM/PCI/NVML relation and ambiguity fixtures (D-02, D-03). | 5 | 01-04 | HOST-02 | 5 |
| 01-06 | Integrate explicit `--output`, durable `selected-output.v1`, process tests, original-archive compatibility, and corrected-host live HOST-01/HOST-02 proof; G0 remains FAIL. | 6 | 01-05 | HOST-02 | 8 |
| 01-07 | Build the NvFBC/CUDA source/ABI gate, no-SDK and fixture providers, frame/lease/copy/cursor/driver models, strict extension, and cleanup matrix (D-02, D-05, D-08). | 7 | 01-06 | HOST-03 | 9 |
| 01-08 | Verify immutable history and separate operator source assertion, then prove one selected-output shared-CUDA frame and `nvfbc-capture.v1`; G0 remains FAIL for HOST-04. | 8 | 01-07 | HOST-03 | 7 |
| 01-09 | Create the NVENC copy-boundary spike first, then freeze H.264 High/HEVC/Ampere-Ada policy, bounded bitstream parsers, and diagnostic fixtures (D-04, D-05). | 9 | 01-08 | HOST-04 | 7 |
| 01-10 | Authenticate SDK 13.1, exact-probe eligible tuples on the live capture lease, verify original history, produce final current G0, and archive current binary/evidence separately. | 10 | 01-09 | HOST-04 | 9 |

All plans are intentionally sequential because each consumes the preceding
contract or live gate. Plans 01-03, 01-06, 01-08, and 01-10 are
non-autonomous due to explicit operator source or physical-host actions.

`G0EvidenceEnvelopeV1` is final in Plan 01-01. Later capabilities populate
`host-foundation.v1`, `selected-output.v1`, `nvfbc-capture.v1`, and
`nvenc-tuples.v1` records; they never add V1 base fields. Generic archive
verification validates the V1 base, raw extension preservation/digests,
manifest, and contained files. Owned gate validation is additional.

The pre-reboot archive copies and hashes the exact executing `/proc/self/exe`
inside a create-once run directory. Later verification never relies on a
rebuilt `target/.../replay-host-doctor`. The post-repair archive is separate
and cannot overwrite original history.

The standalone SDK 13.1 HOST-04 proof remains explicitly separate from the
project-research `nv-codec-headers n12.1.14.0` Kyber/Kymedia integration pin;
this phase does not mutate or claim compatibility for the later pinned tree.

No phase-specific `01-RESEARCH.md` is created. Deferred streaming, transport,
macOS, input, clipboard, Wayland, virtual-display, audio, UI, and product scope
remain outside Phase 1.

## OUTLINE COMPLETE
