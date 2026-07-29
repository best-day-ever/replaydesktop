# HOST-02 Output-to-GPU Mapping Contract

**Status:** `PROVEN_LIVE_XORG` on 2026-07-29
**Contract correction:** Plan 01-06 supersedes the original DRM-authoritative
Plan 01-05 join after the corrected Xorg host showed that public DRM connector
scanout is not an ownership oracle for NVIDIA's X11 display-target topology.

## Outcome

The mapping succeeds only when one stable, query-only observation proves:

1. exactly one XRandR output selected by bounded raw name bytes;
2. exactly one XRandR provider membership for that output;
3. exactly one NV-CONTROL display target whose
   `NV_CTRL_DISPLAY_RANDR_OUTPUT_ID` equals the XRandR output XID and whose raw
   RandR name agrees;
4. exactly one enabled-on-X-screen membership for that display target;
5. exactly one owning GPU target (an NV-CONTROL GPU target) through
   `NV_CTRL_BINARY_DATA_DISPLAYS_CONNECTED_TO_GPU`; and
6. exactly one current NVML device whose canonical PCI BDF and GPU UUID both
   equal the NV-CONTROL GPU identity.

This is the authoritative relation:

```text
requested raw XRandR name
  -> XRandR output XID
  -> NV-CONTROL display target
  -> enabled on authenticated NVIDIA X screen
  -> NV-CONTROL GPU target
  -> exact PCI BDF + GPU UUID
  -> current NVML device
```

Enumeration order, primary-output state, connector number, EDID equality, DRM
connector state, and matching numeric values across unrelated namespaces never
supply an ownership edge. DRM is an optional diagnostic and is excluded from
HOST-01/HOST-02 admission and the authoritative topology token.

MST is supported when the source-defined XRandR-to-NV-CONTROL relation is
unique. The live accepted output `DP-0.3` is an MST branch:
XRandR output XID `540` mapped to NV-CONTROL display target `0`, enabled on X
screen `0`, owned by NV-CONTROL GPU target `0`, and matched one NVML device by
both `00000000:01:00.0` and
`GPU-becdbf04-4151-31a1-a69e-8d877a1e26b0`.

## Authoritative Source Ledger

| Source | Facts used |
| --- | --- |
| X Resize, Rotate and Reflect Extension 1.4 protocol | `RRGetScreenResourcesCurrent`, output raw names, active CRTC/mode, exact timing/origin, provider membership, primary state, and resource/config timestamps. |
| `x11rb-protocol 0.14.0` generated RandR definitions | Exact Rust field widths, reply shapes, notify masks, and separate XID namespaces. |
| Installed `/usr/include/NVCtrl/NVCtrlLib.h` and `/usr/include/NVCtrl/NVCtrl.h` | Fixed XNVCTRL function ABI, target types, attribute IDs, Xmalloc ownership, and protocol-version query. |
| Installed NVIDIA `nv-control-targets.c` sample | Display targets are enumerated through binary attribute lists; GPU target count is the only target-count query used. |
| NV-CONTROL protocol 1.27+ | Display target type and `NV_CTRL_DISPLAY_RANDR_OUTPUT_ID` provide the direct X output edge. The live server reports 1.29. |
| Authenticated NVML API 13 source plus fixed `libnvidia-ml.so.1` runtime | Current device PCI bus ID and UUID, including initialization, bounded enumeration, and shutdown. |
| Linux DRM KMS/sysfs | Optional diagnostic connector count/digest only; no ownership or readiness authority. |

The production collector loads only fixed system SONAMEs:
`libX11.so.6`, `libXNVCtrl.so.0`, and the existing fixed NVML SONAME. Selected
evidence records the resolved loaded filenames. It never loads a
caller-controlled library path.

## Namespace, Unit, and Cardinality Inventory

| Field | Representation | Admission rule |
| --- | --- | --- |
| requested output name | 1–256 raw bytes, lowercase hex plus optional exact UTF-8 display | exactly one raw-name match |
| XRandR output/CRTC/mode/provider | separate `u32` X resource newtypes | nonzero where active; exactly one provider contains the output |
| actual CRTC outputs | bounded XRandR output-XID list | selected output occurs exactly once and is the only active output on its CRTC |
| output geometry/origin | `u16` dimensions and signed `i16` origin | dimensions equal exact mode display fields |
| exact refresh | checked timing reduced to rational Hz | no float or approximate comparison |
| primary/connector number/EDID | supplemental XRandR fields | recorded only; never ownership |
| NV-CONTROL display target | typed target ID; zero is valid | exactly one target has the selected XRandR output XID |
| display target RandR name | raw XNVCTRL string encoded as `OutputNameV1` | byte-for-byte equality with the XRandR name |
| display enabled state | `NV_CTRL_DISPLAY_ENABLED` | present and true |
| enabled-on-X-screen list | native `int[]` binary attribute 17 | selected target occurs exactly once |
| NV-CONTROL GPU target | typed target ID; zero is valid | exactly one GPU's attribute 15 list contains the display target once |
| NV-CONTROL PCI identity | domain/bus/device/function attributes | canonical lowercase `dddddddd:bb:dd.f` |
| NV-CONTROL GPU UUID | bounded `GPU-...` string attribute | exact equality with the same NVML device |
| NVML PCI BDF + UUID | bounded current enumeration | exactly one record matches both fields |
| raw EDID | transient XRandR property bytes | hashed in memory; raw EDID is never persisted |
| DRM diagnostic | optional digest and counts | never part of the pass predicate or topology token |

Target ID `0` is valid for both an NV-CONTROL display target and an NV-CONTROL
GPU target. Validators therefore do not apply XID-style nonzero rules to those
types.

## Bounded Collection Procedure

1. Authenticate the current local Xorg session through logind, the AF_UNIX
   socket peer, Xorg process identity, Xauthority, and X11 setup.
2. Subscribe to all relevant RandR topology notifications before the opening
   snapshot, complete an X11 round trip, and drain the initial queue.
3. Collect bounded XRandR outputs, actual CRTC output membership, modes,
   providers, properties, timestamps, and a deterministic sorted digest.
4. Open a second authenticated Xlib display for the exact display and screen.
   Require NV-CONTROL extension version at least 1.27 and
   `XNVCTRLIsNvScreen(screen)`.
5. Enumerate display targets through binary attribute 14. Do **not** call
   `XNVCTRLQueryTargetCount` for display targets: driver 610.43.03 returns an
   X `BadValue` for that request.
6. Decode every binary target list as native signed `int[]`:
   `[count, target_0, ...]`. Require exact byte length, count at most 64,
   nonnegative IDs at most `i32::MAX`, and no duplicates. Always release the
   Xmalloc result with `XFree`.
7. Query each display target's XRandR output ID. Record the exact RandR name,
   enabled state, target-index/type names, DP GUID, EDID hash, and MST state
   when the attribute exists.
8. Query enabled-on-X-screen attribute 17 for the authenticated screen.
9. Query the bounded NV-CONTROL GPU count, each GPU's connected-display list
   (attribute 15), PCI components, and GPU UUID.
10. Collect current bounded NVML BDF+UUID identities.
11. Recollect XRandR, NV-CONTROL, and NVML, force an X11 round trip, and count
    queued RandR topology events.
12. Admit only equal opening/closing RandR, NV-CONTROL, and NVML digests and
    zero RandR events. Optional DRM diagnostic observations may be absent,
    empty, unequal, or changing without affecting admission.

All lists and snapshots are sorted before hashing. Every native string,
collection, and binary result is bounded. The doctor never modesets, changes
X configuration, or writes to the server.

## Exact Predicate

Let `R` be the selected XRandR output, `P` an XRandR provider, `D` an
NV-CONTROL display target, `G` an NV-CONTROL GPU target, and `N` an NVML
device. Selection succeeds iff:

```text
count(R where R.raw_name == requested_raw_name) == 1
R.connected && R.physical && !R.non_desktop
R.crtc_xid != 0 && R.mode_xid != 0
R.crtc_output_xids == [R.output_xid]

count(P where R.output_xid occurs once in P.output_xids) == 1

NV-CONTROL extension present
NV-CONTROL version >= 1.27
authenticated X screen is NVIDIA-controlled

count(D where D.randr_output_xid == R.output_xid) == 1
D.randr_name.raw_bytes == R.raw_name
D.enabled == true
count(enabled_on_xscreen where target_id == D.target_id) == 1

count(G where D.target_id occurs once in G.connected_display_target_ids) == 1
G.canonical_pci_bdf is exactly derived from G PCI fields

count(N where N.pci_bdf == G.canonical_pci_bdf
          && N.uuid == G.uuid) == 1

opening_authoritative_token == closing_authoritative_token
closing_randr_event_count == 0
```

The proof stores all six relation cardinalities. It returns
`BLOCKED_AMBIGUOUS` for zero/multiple ownership edges,
`BLOCKED_CONFLICTING_FACTS` for disagreeing direct identities,
`BLOCKED_TOPOLOGY_CHANGED` for mixed generations, and
`BLOCKED_NVCONTROL_UNAVAILABLE` when the direct display-target source is not
available at the required version. Malformed or oversized observations return
`BLOCKED_INVALID_OBSERVATION`.

## Adversarial Matrix

| Condition | Result |
| --- | --- |
| Unique XRandR output → display target → enabled screen → GPU target → exact NVML BDF+UUID | pass |
| Two displays share EDID, connector number, type name, or primary ordering but direct target IDs remain unique | pass |
| Selected source-defined display target reports MST | pass |
| DRM diagnostic absent, zero, inactive, ambiguous, or changing | pass |
| NV-CONTROL extension/version missing or older than 1.27 | `BLOCKED_NVCONTROL_UNAVAILABLE` |
| Authenticated X screen is not NVIDIA-controlled | `BLOCKED_UNSUPPORTED_TOPOLOGY` |
| Zero/multiple display targets for the XRandR XID | `BLOCKED_AMBIGUOUS` |
| Display-target RandR name differs from the selected raw name | `BLOCKED_CONFLICTING_FACTS` |
| Display target disabled or not enabled on the screen | blocked; no fallback |
| Zero/multiple owning GPU targets or duplicate list membership | `BLOCKED_AMBIGUOUS` or invalid observation |
| GPU PCI fields malformed, or NVML BDF/UUID differs | invalid or `BLOCKED_AMBIGUOUS` |
| Provider membership ambiguous | `BLOCKED_AMBIGUOUS` |
| Actual CRTC scanout contains more than the selected output | `BLOCKED_UNSUPPORTED_TOPOLOGY` |
| Authoritative digest/timestamp changes or a RandR event arrives | `BLOCKED_TOPOLOGY_CHANGED` |

## Live Reference Result

The corrected reference host ran Linux
`7.1.4-1-cachyos` with NVIDIA kernel/userspace `610.43.03`, one real local
Xorg session on `:0.0`, NV-CONTROL 1.29, and one current NVML device. The exact
operator selection `DP-0.3` passed HOST-02. Public DRM returned zero active
connectors in both optional diagnostic snapshots; that observation neither
blocked HOST-01 nor entered the HOST-02 token.

The preserved pre-reboot Wayland/NVML-mismatch archive remains immutable and
verifiable. NvFBC capture and NVENC tuple proof remain unproven, so the overall
G0 result remains FAIL.
