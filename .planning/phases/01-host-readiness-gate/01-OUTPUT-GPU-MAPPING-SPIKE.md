# HOST-02 Output-to-GPU Mapping Feasibility Spike

**Status:** `BLOCKED_PRE_REBOOT_XORG` on the current reference machine  
**Contract outcome:** feasible as a fail-closed, observation-only proof  
**Scope boundary:** this document freezes the pure mapping contract for Plan
01-05. Live collection and selected-output evidence integration belong to Plan
01-06.

## Outcome

The mapping is accepted only when one stable observation relates:

1. exactly one XRandR output selected by its bounded raw name;
2. exactly one XRandR provider that owns that output and its active CRTC;
3. exactly one enabled DRM connector with the same SHA-256 EDID digest,
   source-verified connector kind, and exact active timing;
4. that DRM connector's device link to one canonical PCI BDF; and
5. exactly one current NVML device with that canonical PCI BDF and one unique
   NVML UUID.

All five relations must hold in the same stable topology token. The matcher
does not rank candidates, compare approximate refresh rates, use enumeration
order, or guess when displays are identical. Missing facts, duplicate
candidates, conflicting observations, cloned outputs, MST, PRIME, offload, and
topology changes are blocking evidence.

`XRandR output XID` and `DRM connector_id` occupy disjoint namespaces.
Numeric equality between them has no meaning and is never a predicate. XRandR
`ConnectorNumber` and DRM `connector_type_id` are disjoint too. A value that
happens to be equal does not prove identity.

## Authoritative Source Ledger

| Source | Version or interface | Facts used by the contract |
| --- | --- | --- |
| [X Resize, Rotate and Reflect Extension Version 1.4](https://www.x.org/releases/current/doc/randrproto/randrproto.txt) | Current published RandR protocol specification | `RRGetScreenResourcesCurrent` supplies resource/config timestamps and XID lists; `RRGetOutputInfo` supplies raw output name bytes, current CRTC, connection state, modes, and clone XIDs; `RRGetOutputProperty` supplies bounded typed property bytes and `bytes-after`; `RRGetCrtcInfo` supplies signed origin, dimensions, mode, outputs, and config timestamp; provider requests supply provider XIDs, capability bits, outputs, CRTCs, and associated providers. |
| `x11rb-protocol` source | Cargo lock resolves `x11rb-protocol 0.14.0`; exact generated definitions in `src/protocol/randr.rs` | Rust field widths and collection shapes: XIDs and timestamps are `u32`, CRTC origins are `i16`, dimensions are `u16`, mode dot clock is `u32`, names/properties are byte vectors, and provider capability flags are `SOURCE_OUTPUT=1`, `SINK_OUTPUT=2`, `SOURCE_OFFLOAD=4`, `SINK_OFFLOAD=8`. |
| [Linux DRM mode UAPI](https://codebrowser.dev/linux/linux/include/uapi/drm/drm_mode.h.html) | `DRM_IOCTL_MODE_GETRESOURCES`, `GETCONNECTOR`, `GETENCODER`, and `GETCRTC` | `connector_id` is a DRM mode-object ID; `connector_type_id` is only the per-type instance number. Connector array sizes can change during hotplug and therefore require bounded retry. The active chain is connector → encoder → CRTC. DRM mode clocks are kHz and must be checked before conversion to Hz. |
| [Linux DRM KMS properties](https://docs.kernel.org/gpu/drm-kms.html) | Connector `EDID` blob and atomic connector properties | A connected sink's EDID is a connector property. Only its in-memory SHA-256 digest crosses the collection boundary. Active/connected state and current mode must come from DRM state rather than a connector filename. |
| [Linux sysfs rules](https://docs.kernel.org/admin-guide/sysfs-rules.html) | `/sys/class/drm/*/device` symlink resolution | Class links are resolved to the canonical `/sys/devices` hierarchy. The PCI ancestor is normalized to lowercase `dddd:bb:dd.f` and cross-checked through `/sys/bus/pci/devices/<BDF>`. |
| [NVML device queries](https://docs.nvidia.com/deploy/nvml-api/group__nvmlDeviceQueries.html) | `nvmlDeviceGetCount_v2`, `nvmlDeviceGetHandleByIndex_v2`, `nvmlDeviceGetPciInfo_v3`, `nvmlDeviceGetUUID` | Enumeration index is not stable and is never identity. `busId` is the PCI domain:bus:device.function identity; UUID is the immutable device identity recorded after the exact BDF match. |
| [NVIDIA RandR 1.4 offload documentation](https://download.nvidia.com/XFree86/Linux-x86_64/570.169/README/randr14.html) | NVIDIA source-output and PRIME/offload provider layouts | Multi-provider source/sink and offload associations are real supported topologies but are outside this prototype proof. Their presence is rejected rather than simplified. |

`NV-CONTROL`: **NOT_REQUIRED**. RandR protocol observations, DRM KMS state,
canonical sysfs device ancestry, and NVML BDF/UUID identity close the required
relation. Introducing NV-CONTROL would add another namespace without filling a
missing edge. If Plan 01-06 discovers that a deployed NVIDIA Xorg configuration
does not expose source-verified connector metadata through RandR, that is a new
spike result, not permission to guess.

## Namespace, Unit, and Cardinality Inventory

| Field | Namespace and representation | Unit / normalization | Required cardinality |
| --- | --- | --- | --- |
| requested output name | Bounded XRandR raw byte string, persisted as lowercase even-length hex plus optional exact UTF-8 display | bytes; 1–256 bytes | one request |
| XRandR output XID | X server resource ID (`CARD32`) | opaque integer | exactly one raw-name match |
| XRandR CRTC XID | X server resource ID (`CARD32`) | opaque integer | exactly one nonzero active CRTC on the selected output |
| XRandR mode XID | X server resource ID (`CARD32`) | opaque integer | exactly one nonzero active mode |
| XRandR provider XID | X server resource ID (`CARD32`) | opaque integer | exactly one provider containing both selected output and CRTC |
| RandR timestamp and config timestamp | X server time (`TIMESTAMP`, represented as `u32`) | server milliseconds modulo 2³²; compare for equality only | one before and one after token |
| CRTC origin | RandR signed coordinates | pixels; preserve negative values | one `(x,y)` pair |
| active dimensions | RandR CRTC and mode dimensions | pixels | CRTC dimensions must equal mode display dimensions |
| RandR dot clock | `ModeInfo.dot_clock` | Hz | one checked nonzero value |
| RandR timing totals | `ModeInfo` horizontal/vertical fields | pixels/lines | one internally ordered exact tuple |
| refresh | Derived only from the exact timing tuple | reduced rational Hz; no float | exactly one checked rational |
| RandR connector kind | Source-verified bounded property value | normalized enum, not inferred from output name | exactly one supported physical kind |
| RandR `ConnectorNumber` | RandR output-property namespace | optional `u32`; never compared with a DRM object ID | zero or one |
| EDID | RandR/DRM property payload | raw bytes are hashed in memory; persist only lowercase SHA-256 | exactly one digest on each side |
| DRM connector ID | DRM mode-object namespace (`u32`) | opaque integer | exactly one matching enabled connector |
| DRM connector kind | DRM `connector_type` | normalized enum | exactly equal to the source-verified RandR kind |
| DRM type instance | DRM `connector_type_id` | per-kind number, not globally stable | recorded, never treated as X identity |
| DRM connector name | Kernel display label such as `DP-1` | bounded visible ASCII | recorded, never matched to XRandR name |
| DRM mode clock | `drm_mode_modeinfo.clock` | kHz; checked multiply by 1000 to Hz | exact equality after normalization |
| PCI BDF | Canonical PCI ancestor identity | lowercase `dddd:bb:dd.f` | exactly one canonical BDF |
| NVML PCI BDF | NVML `busId` | canonical lowercase `dddd:bb:dd.f` | exactly one current device match |
| NVML UUID | NVML UUID byte string | bounded visible ASCII, `GPU-` prefix | exactly one UUID and no duplicate UUID observation |
| proof cardinalities | Counts after every relation filter | unsigned bounded counts | every relation count equals one |

The persisted result names XRandR and DRM fields separately. It never serializes
a generic `output_id`, `connector_id`, or `device_id` whose namespace would be
ambiguous.

## Bounded Observation Procedure

The production collector in Plan 01-06 must use the protocol/UAPI equivalents
of the commands below. These commands are the reproducible spike surface, not a
license to parse localized CLI output in production.

### XRandR

```text
xrandr --version
xrandr --current --query
xrandr --listproviders
xrandr --current --listproviders
xrandr --current --prop
```

Collection rules:

- Take `RRGetScreenResourcesCurrent` and `RRGetProviders` snapshots before
  output/property reads.
- Select the requested output by exact raw name bytes, not lossy UTF-8.
- Require connected, active output/CRTC/mode and an empty clone list.
- Require one provider with capability exactly `SOURCE_OUTPUT`, no offload or
  sink bits, no associated providers, and membership of both output and CRTC.
- Bound every reply collection. For properties, bound `num-items`, request no
  more than the limit, and require `bytes-after == 0`.
- Hash the EDID property immediately; raw EDID is never persisted or logged.
- Read resources/providers again and require timestamps/config timestamps and a
  digest of all identity-relevant observations to match the opening token.

### DRM and sysfs

```text
for connector in /sys/class/drm/card*-*; do
  test -e "$connector/status" || continue
  readlink -f "$connector/device"
  sed -n '1p' "$connector/status"
  test -r "$connector/enabled" && sed -n '1p' "$connector/enabled"
done
lspci -Dnn
```

Collection rules:

- Enumerate card/connector resources through libdrm or the DRM mode ioctls.
- Retry the two-call `GETCONNECTOR` sizing sequence only a bounded number of
  times; changing counts exhaust the snapshot with
  `BLOCKED_TOPOLOGY_CHANGED`.
- Require connected, enabled, physical, non-leased, non-MST state.
- Follow current encoder and CRTC and require an active mode.
- Normalize DRM kHz to Hz with checked arithmetic, then compare every timing
  field and relevant flag exactly.
- Resolve the connector's `device` link and its PCI ancestor; require the
  canonical `/sys/devices` target and `/sys/bus/pci/devices/<BDF>` target to
  identify the same device.

### NVML

```text
nvidia-smi --query-gpu=pci.bus_id,uuid --format=csv,noheader
```

The production collector calls NVML directly and performs bounded current
enumeration. The CLI line is retained so an operator can reproduce the spike.
Indices and display-active flags are not identity. BDF equality is exact after
canonicalization; UUID is then recorded and required to be unique.

## Exact Mapping Predicate

Let `R` be the selected XRandR output, `P` a provider, `D` a DRM connector, and
`N` an NVML device. Selection succeeds iff all of the following are true in one
stable token:

```text
count(R where R.raw_name == requested_raw_name) == 1
R.connected && R.physical && !R.non_desktop
R.crtc_xid != 0 && R.mode_xid != 0 && count(R.clone_output_xids) == 0

count(P where R.output_xid in P.output_xids
              && R.crtc_xid in P.crtc_xids) == 1
count(all providers) == 1
P.capabilities == SOURCE_OUTPUT
count(P.associated_provider_xids) == 0

R.edid_sha256 exists
R.connector_kind is source-verified and supported
R.crtc_dimensions == R.mode_display_dimensions
R.refresh == reduce(exact_checked_timing)

count(D where D.connected && D.enabled && D.physical
              && !D.mst && !D.leased
              && D.edid_sha256 == R.edid_sha256
              && D.connector_kind == R.connector_kind
              && D.active_timing == R.exact_timing) == 1

count(canonical PCI BDF reached from D.device) == 1
count(N where canonical(N.pci_bus_id) == D.canonical_pci_bdf) == 1
count(NVML observations with N.uuid) == 1

opening_topology_token == closing_topology_token
```

The proof stores the five relation cardinalities. Any count other than one is
`BLOCKED_AMBIGUOUS`; it does not select the first item. Facts that identify the
same object but disagree are `BLOCKED_CONFLICTING_FACTS`. Changed opening and
closing tokens are `BLOCKED_TOPOLOGY_CHANGED`.

## Topology and Failure Matrix

| Condition | Stable result | Rationale |
| --- | --- | --- |
| Unique XRandR → provider → EDID/kind/timing → DRM → PCI BDF → NVML relation | pass | Every edge and cardinality is explicit. |
| Two physically identical displays produce two valid DRM candidates | `BLOCKED_AMBIGUOUS` | Identical EDID and timing do not authorize connector guessing. |
| Duplicate EDID with missing discriminating metadata | `BLOCKED_AMBIGUOUS` | Absence of proof is not evidence for a preferred connector. |
| Missing output, provider, EDID, active CRTC, DRM connector, PCI ancestor, NVML BDF, or UUID | `BLOCKED_AMBIGUOUS` | A required edge has cardinality zero. |
| Duplicate provider, DRM candidate, PCI ancestor, NVML BDF, or UUID | `BLOCKED_AMBIGUOUS` | A required edge has cardinality greater than one. |
| Same observation identity with incompatible dimensions/timing/kind/BDF | `BLOCKED_CONFLICTING_FACTS` | Internally inconsistent evidence cannot be ranked. |
| Resource/config/snapshot token changes during collection | `BLOCKED_TOPOLOGY_CHANGED` | Mixed-generation facts cannot prove one topology. |
| XRandR clone list is nonempty | `BLOCKED_UNSUPPORTED_TOPOLOGY` | A single X output does not uniquely identify one physical scanout. |
| Selected DRM connector participates in MST | `BLOCKED_UNSUPPORTED_TOPOLOGY` | The prototype has not established a stable branch-to-X mapping. |
| Provider has PRIME/offload/sink bits or associations | `BLOCKED_UNSUPPORTED_TOPOLOGY` | Multi-GPU presentation/render relations need a separate contract. |
| Virtual/non-desktop output, lease, or nonphysical connector | `BLOCKED_UNSUPPORTED_TOPOLOGY` | HOST-02 proves a physical NVIDIA X11 desktop only. |
| Oversized collection, string, or property; malformed BDF; arithmetic overflow | `BLOCKED_INVALID_OBSERVATION` | Unbounded or malformed evidence is not accepted. |

## Current Reference-Machine Observation

Observed on 2026-07-27 before the required reboot into the Xorg session:

```text
$ printf '%s\n' "$XDG_SESSION_TYPE"
XDG_SESSION_TYPE=wayland

$ xrandr --version
xrandr program version 1.5.4
Server reports RandR version 1.6
warning: running xrandr against an Xwayland server

$ xrandr --current --listproviders
Providers: number : 0

$ xrandr --current --query
eDP-1 connected 1920x1080+0+0

$ xrandr --current --prop
RANDR Emulation: 1

$ nvidia-smi --query-gpu=pci.bus_id,uuid --format=csv,noheader
Failed to initialize NVML: Driver/library version mismatch
NVML library version: 610.43
```

The connected sysfs connector was `card1-eDP-1`, with DRM
`connector_id=836`, status `connected`, state `enabled`, and a canonical device
ancestor ending in PCI BDF `0000:01:00.0`. Its EDID property was not readable in
this environment. Those facts do not bridge the Xwayland-emulated output to an
NVML device. In normalized diagnostic wording this is an
`NVML driver/library version mismatch`.

Therefore the reference-machine result is:

```text
CURRENT_MACHINE_MAPPING=BLOCKED_PRE_REBOOT_XORG
```

This is an environment gate, not a mapping success and not a fixture pass. A
post-reboot native Xorg run is required before Plan 01-06 may emit selected
output/GPU evidence.

## Persistence and Security Boundary

- Persist output names as bounded lowercase raw-byte hex. Add an optional
  display string only when strict UTF-8 decoding round-trips to the same bytes.
- Persist only the SHA-256 digest of EDID. Raw EDID is ephemeral and raw EDID
  bytes must never appear in evidence, fixtures, logs, errors, or command
  transcripts.
- Persist no Xauthority cookie, environment secret, socket credential, or full
  process environment.
- Bound outputs, providers, CRTCs, modes, properties, connectors, NVML devices,
  names, UUIDs, and topology retries.
- Normalize clocks and refresh using checked integer arithmetic. Store the
  refresh as a reduced rational, never a floating-point approximation.
- Preserve XRandR XIDs, DRM IDs, PCI BDF, and NVML UUID in separately named
  fields so downstream consumers cannot compare unlike namespaces by accident.

## Plan 01-06 Handoff

Plan 01-06 may implement live collectors and encode the proven result in the
selected-output extension. It must retain these exact fail-closed predicates,
stable reasons, bounds, and namespace separation. This plan does not alter the
G0 V1 envelope decoder, archive verifier, CLI, or live evidence integration.
