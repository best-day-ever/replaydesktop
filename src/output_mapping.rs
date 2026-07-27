use std::collections::HashSet;
use std::ffi::{c_char, c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::Path;

use libloading::Library;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use x11rb::connection::RequestConnection as _;
use x11rb::protocol::randr::{Connection as RandrConnection, ConnectionExt as _, SetConfig};
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _};

use crate::digest::sha256_bytes;
use crate::model::{
    DrmConnectorObservationV1, ExactModeTimingV1, MAX_CONNECTOR_NAME_BYTES_V1,
    MAX_NVML_UUID_BYTES_V1, MAX_OUTPUT_MAPPING_ITEMS_V1, MAX_OUTPUT_NAME_BYTES_V1,
    OutputMappingProofCardinalitiesV1, OutputNameV1, OutputTopologyObservationV1,
    OutputTopologyTokenV1, PhysicalConnectorKindV1, RandrOutputObservationV1, RefreshRateV1,
    SELECTED_OUTPUT_SCHEMA_V1, SelectedOutputV1,
};

pub const SELECTED_OUTPUT_DISCOVERY_SCHEMA_V1: &str = "replaydesktop.selected-output-discovery.v1";
pub const SELECTED_OUTPUT_FAILURE_SCHEMA_V1: &str = "replaydesktop.selected-output-failure.v1";

const RANDR_PROVIDER_SOURCE_OUTPUT: u32 = 1;
const RANDR_PROVIDER_KNOWN_CAPABILITIES: u32 = 0x0f;
const RANDR_MODE_INTERLACE: u32 = 1 << 4;
const RANDR_MODE_DOUBLE_SCAN: u32 = 1 << 5;
const RANDR_MODE_EXACT_REFRESH_FLAGS: u32 = 0x07ff;
const HOST02_FIXTURE_SCHEMA_V1: &str = "replaydesktop.host02-output-topologies.v1";
const MAX_HOST02_FIXTURE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum OutputMappingReasonV1 {
    #[serde(rename = "BLOCKED_AMBIGUOUS")]
    Ambiguous,
    #[serde(rename = "BLOCKED_CONFLICTING_FACTS")]
    ConflictingFacts,
    #[serde(rename = "BLOCKED_TOPOLOGY_CHANGED")]
    TopologyChanged,
    #[serde(rename = "BLOCKED_UNSUPPORTED_TOPOLOGY")]
    UnsupportedTopology,
    #[serde(rename = "BLOCKED_INVALID_OBSERVATION")]
    InvalidObservation,
}

impl OutputMappingReasonV1 {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::Ambiguous => "BLOCKED_AMBIGUOUS",
            Self::ConflictingFacts => "BLOCKED_CONFLICTING_FACTS",
            Self::TopologyChanged => "BLOCKED_TOPOLOGY_CHANGED",
            Self::UnsupportedTopology => "BLOCKED_UNSUPPORTED_TOPOLOGY",
            Self::InvalidObservation => "BLOCKED_INVALID_OBSERVATION",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMappingRelationV1 {
    Observation,
    TopologyToken,
    RequestedOutput,
    RandrProvider,
    DrmConnector,
    CanonicalPciBdf,
    NvmlDevice,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputMappingFailureV1 {
    pub reason: OutputMappingReasonV1,
    pub relation: OutputMappingRelationV1,
    pub observed_cardinality: Option<u32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputCollectionFailureV1 {
    XorgUnavailable,
    RandrUnavailable,
    DrmUnavailable,
    NvmlUnavailable,
    InvalidObservation,
}

impl OutputCollectionFailureV1 {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::XorgUnavailable => "xorg-unavailable",
            Self::RandrUnavailable => "randr-unavailable",
            Self::DrmUnavailable => "drm-unavailable",
            Self::NvmlUnavailable => "nvml-unavailable",
            Self::InvalidObservation => "invalid-observation",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputCollectorObservationV1 {
    pub requested_output: Option<OutputNameV1>,
    pub candidates: Vec<OutputNameV1>,
    pub topology: Option<OutputTopologyObservationV1>,
    pub collection_failure: Option<OutputCollectionFailureV1>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedOutputDiscoveryV1 {
    pub schema: String,
    pub candidates: Vec<OutputNameV1>,
    pub collection_failure: Option<OutputCollectionFailureV1>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedOutputFailureEvidenceV1 {
    pub schema: String,
    pub requested_output: OutputNameV1,
    pub candidates: Vec<OutputNameV1>,
    pub mapping_failure: Option<OutputMappingFailureV1>,
    pub collection_failure: Option<OutputCollectionFailureV1>,
}

pub fn output_name_from_cli(value: &str) -> Option<OutputNameV1> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_OUTPUT_NAME_BYTES_V1
        || value.chars().any(char::is_control)
    {
        return None;
    }
    Some(OutputNameV1 {
        hex: lower_hex_encode(bytes),
        display: Some(value.to_owned()),
    })
}

pub fn selected_output_discovery(
    observation: &OutputCollectorObservationV1,
) -> Option<SelectedOutputDiscoveryV1> {
    validate_output_collection_observation(observation).then(|| SelectedOutputDiscoveryV1 {
        schema: SELECTED_OUTPUT_DISCOVERY_SCHEMA_V1.to_owned(),
        candidates: observation.candidates.clone(),
        collection_failure: observation.collection_failure,
    })
}

pub fn selected_output_failure(
    observation: &OutputCollectorObservationV1,
    mapping_failure: Option<OutputMappingFailureV1>,
) -> Option<SelectedOutputFailureEvidenceV1> {
    let requested_output = observation.requested_output.clone()?;
    if !validate_output_collection_observation(observation)
        || (mapping_failure.is_some() == observation.collection_failure.is_some())
    {
        return None;
    }
    Some(SelectedOutputFailureEvidenceV1 {
        schema: SELECTED_OUTPUT_FAILURE_SCHEMA_V1.to_owned(),
        requested_output,
        candidates: observation.candidates.clone(),
        mapping_failure,
        collection_failure: observation.collection_failure,
    })
}

pub fn prove_output_gpu_mapping(
    observation: &OutputTopologyObservationV1,
) -> Result<SelectedOutputV1, OutputMappingFailureV1> {
    validate_observation(observation)?;

    if observation.token_before != observation.token_after {
        return Err(failure(
            OutputMappingReasonV1::TopologyChanged,
            OutputMappingRelationV1::TopologyToken,
            None,
        ));
    }

    let output_matches = observation
        .randr_outputs
        .iter()
        .filter(|output| output.name.hex == observation.requested_output_name.hex)
        .collect::<Vec<_>>();
    if output_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::RequestedOutput,
            output_matches.len(),
        ));
    }
    let output = output_matches[0];
    validate_selected_output(output)?;
    let edid_sha256 = output
        .edid_sha256
        .ok_or_else(|| cardinality_failure(OutputMappingRelationV1::DrmConnector, 0))?;
    let refresh_hz = exact_refresh_rate(&output.timing).ok_or_else(invalid_observation)?;

    if observation.randr_providers.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::RandrProvider,
            observation.randr_providers.len(),
        ));
    }
    let provider_matches = observation
        .randr_providers
        .iter()
        .filter(|provider| {
            provider
                .output_xids
                .iter()
                .filter(|candidate| **candidate == output.output_xid)
                .count()
                == 1
                && provider
                    .crtc_xids
                    .iter()
                    .filter(|candidate| **candidate == output.crtc_xid)
                    .count()
                    == 1
        })
        .collect::<Vec<_>>();
    if provider_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::RandrProvider,
            provider_matches.len(),
        ));
    }
    let provider = provider_matches[0];
    if provider.capabilities != RANDR_PROVIDER_SOURCE_OUTPUT
        || !provider.associated_provider_xids.is_empty()
    {
        return Err(failure(
            OutputMappingReasonV1::UnsupportedTopology,
            OutputMappingRelationV1::RandrProvider,
            Some(1),
        ));
    }

    let metadata_candidates = observation
        .drm_connectors
        .iter()
        .filter(|connector| {
            connector.edid_sha256 == Some(edid_sha256)
                && connector.connector_kind == output.connector_kind
        })
        .collect::<Vec<_>>();
    let drm_matches = metadata_candidates
        .iter()
        .copied()
        .filter(|connector| connector.active_timing == output.timing)
        .collect::<Vec<_>>();
    if drm_matches.is_empty() && !metadata_candidates.is_empty() {
        return Err(failure(
            OutputMappingReasonV1::ConflictingFacts,
            OutputMappingRelationV1::DrmConnector,
            Some(0),
        ));
    }
    if drm_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::DrmConnector,
            drm_matches.len(),
        ));
    }
    let connector = drm_matches[0];
    validate_selected_connector(connector)?;

    if connector.canonical_pci_bdfs.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::CanonicalPciBdf,
            connector.canonical_pci_bdfs.len(),
        ));
    }
    let canonical_pci_bdf = &connector.canonical_pci_bdfs[0];

    let nvml_matches = observation
        .nvml_devices
        .iter()
        .filter(|device| device.nvml_pci_bdf == *canonical_pci_bdf)
        .collect::<Vec<_>>();
    if nvml_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvmlDevice,
            nvml_matches.len(),
        ));
    }
    let nvml_device = nvml_matches[0];
    let uuid_matches = observation
        .nvml_devices
        .iter()
        .filter(|device| device.nvml_uuid == nvml_device.nvml_uuid)
        .count();
    if uuid_matches != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvmlDevice,
            uuid_matches,
        ));
    }

    Ok(SelectedOutputV1 {
        schema: SELECTED_OUTPUT_SCHEMA_V1.to_owned(),
        output_name: output.name.clone(),
        randr_output_xid: output.output_xid,
        randr_crtc_xid: output.crtc_xid,
        randr_mode_xid: output.mode_xid,
        randr_provider_xid: provider.provider_xid,
        randr_timestamp: observation.token_before.randr_timestamp,
        randr_config_timestamp: observation.token_before.randr_config_timestamp,
        width_px: output.width_px,
        height_px: output.height_px,
        origin_x: output.origin_x,
        origin_y: output.origin_y,
        refresh_hz,
        exact_timing: output.timing.clone(),
        edid_sha256,
        randr_connector_kind: output.connector_kind,
        randr_connector_number: output.connector_number,
        drm_connector_id: connector.connector_id,
        drm_connector_kind: connector.connector_kind,
        drm_connector_type_id: connector.connector_type_id,
        drm_connector_name: connector.connector_name.clone(),
        drm_canonical_pci_bdf: canonical_pci_bdf.clone(),
        nvml_pci_bdf: nvml_device.nvml_pci_bdf.clone(),
        nvml_uuid: nvml_device.nvml_uuid.clone(),
        topology_token: observation.token_before.clone(),
        proof: OutputMappingProofCardinalitiesV1 {
            requested_output_matches: 1,
            provider_matches: 1,
            drm_connector_matches: 1,
            canonical_pci_bdf_matches: 1,
            nvml_device_matches: 1,
        },
    })
}

pub fn validate_selected_output_evidence(selected: &SelectedOutputV1) -> bool {
    selected.schema == SELECTED_OUTPUT_SCHEMA_V1
        && valid_output_name(&selected.output_name)
        && selected.randr_output_xid.get() != 0
        && selected.randr_crtc_xid.get() != 0
        && selected.randr_mode_xid.get() != 0
        && selected.randr_provider_xid.get() != 0
        && selected.width_px == selected.exact_timing.hdisplay
        && selected.height_px == selected.exact_timing.vdisplay
        && exact_refresh_rate(&selected.exact_timing) == Some(selected.refresh_hz)
        && selected.randr_connector_kind == selected.drm_connector_kind
        && selected.drm_connector_id.get() != 0
        && selected.drm_connector_type_id != 0
        && valid_visible_ascii(&selected.drm_connector_name, MAX_CONNECTOR_NAME_BYTES_V1)
        && valid_canonical_pci_bdf(&selected.drm_canonical_pci_bdf)
        && selected.drm_canonical_pci_bdf == selected.nvml_pci_bdf
        && valid_nvml_uuid(&selected.nvml_uuid)
        && selected.randr_timestamp == selected.topology_token.randr_timestamp
        && selected.randr_config_timestamp == selected.topology_token.randr_config_timestamp
        && selected.proof.requested_output_matches == 1
        && selected.proof.provider_matches == 1
        && selected.proof.drm_connector_matches == 1
        && selected.proof.canonical_pci_bdf_matches == 1
        && selected.proof.nvml_device_matches == 1
}

pub fn validate_selected_output_discovery(discovery: &SelectedOutputDiscoveryV1) -> bool {
    discovery.schema == SELECTED_OUTPUT_DISCOVERY_SCHEMA_V1
        && valid_candidates(&discovery.candidates)
}

pub fn validate_selected_output_failure(failure: &SelectedOutputFailureEvidenceV1) -> bool {
    failure.schema == SELECTED_OUTPUT_FAILURE_SCHEMA_V1
        && valid_output_name(&failure.requested_output)
        && valid_candidates(&failure.candidates)
        && (failure.mapping_failure.is_some() != failure.collection_failure.is_some())
}

pub fn collect_fixture_output_topology(
    path: &Path,
    case_id: &str,
    requested_output: Option<&str>,
) -> Result<OutputCollectorObservationV1, OutputCollectionFailureV1> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| OutputCollectionFailureV1::InvalidObservation)?
        .take((MAX_HOST02_FIXTURE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| OutputCollectionFailureV1::InvalidObservation)?;
    if bytes.len() > MAX_HOST02_FIXTURE_BYTES {
        return Err(OutputCollectionFailureV1::InvalidObservation);
    }
    let fixture: Host02FixtureDocumentV1 = serde_json::from_slice(&bytes)
        .map_err(|_| OutputCollectionFailureV1::InvalidObservation)?;
    if fixture.schema != HOST02_FIXTURE_SCHEMA_V1
        || fixture.fixture_semantics.is_empty()
        || !unique_case_ids(&fixture.cases)
    {
        return Err(OutputCollectionFailureV1::InvalidObservation);
    }
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(OutputCollectionFailureV1::InvalidObservation)?;
    if !matches!(case.expected.status.as_str(), "pass" | "blocked")
        || (case.expected.status == "pass" && case.expected.reason.is_some())
        || (case.expected.status == "blocked"
            && !case
                .expected
                .reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("BLOCKED_")))
    {
        return Err(OutputCollectionFailureV1::InvalidObservation);
    }
    let mut topology = fixture.base_topology;
    overlay_json(&mut topology, &case.topology_patch);
    let mut topology: OutputTopologyObservationV1 = serde_json::from_value(topology)
        .map_err(|_| OutputCollectionFailureV1::InvalidObservation)?;
    let candidates = candidates_from_outputs(&topology.randr_outputs);
    let requested_output = match requested_output {
        Some(value) => {
            Some(output_name_from_cli(value).ok_or(OutputCollectionFailureV1::InvalidObservation)?)
        }
        None => None,
    };
    if let Some(requested) = &requested_output {
        topology.requested_output_name = requested.clone();
    }
    let has_requested_output = requested_output.is_some();
    let observation = OutputCollectorObservationV1 {
        requested_output,
        candidates,
        topology: has_requested_output.then_some(topology),
        collection_failure: None,
    };
    validate_output_collection_observation(&observation)
        .then_some(observation)
        .ok_or(OutputCollectionFailureV1::InvalidObservation)
}

pub fn collect_live_output_topology(
    requested_output: Option<OutputNameV1>,
) -> OutputCollectorObservationV1 {
    let failure = |failure| OutputCollectorObservationV1 {
        requested_output: requested_output.clone(),
        candidates: Vec::new(),
        topology: None,
        collection_failure: Some(failure),
    };
    let Some(xorg) = crate::local_xorg::connect_authenticated_local_xorg() else {
        return failure(OutputCollectionFailureV1::XorgUnavailable);
    };
    let Ok(opening_randr) = collect_randr_snapshot(&xorg.connection, xorg.root) else {
        return failure(OutputCollectionFailureV1::RandrUnavailable);
    };
    let candidates = candidates_from_outputs(&opening_randr.outputs);
    if requested_output.is_none() {
        let observation = OutputCollectorObservationV1 {
            requested_output: None,
            candidates,
            topology: None,
            collection_failure: None,
        };
        return if validate_output_collection_observation(&observation) {
            observation
        } else {
            failure(OutputCollectionFailureV1::InvalidObservation)
        };
    }

    let Ok(opening_drm) = collect_drm_snapshot() else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::DrmUnavailable),
        };
    };
    let Ok(opening_nvml) = collect_nvml_snapshot() else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::NvmlUnavailable),
        };
    };
    let Ok(closing_randr) = collect_randr_snapshot(&xorg.connection, xorg.root) else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::RandrUnavailable),
        };
    };
    let Ok(closing_drm) = collect_drm_snapshot() else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::DrmUnavailable),
        };
    };
    let Ok(closing_nvml) = collect_nvml_snapshot() else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::NvmlUnavailable),
        };
    };
    let requested = requested_output
        .clone()
        .expect("explicit-output branch must carry the request");
    let topology = OutputTopologyObservationV1 {
        requested_output_name: requested,
        token_before: OutputTopologyTokenV1 {
            randr_timestamp: opening_randr.timestamp,
            randr_config_timestamp: opening_randr.config_timestamp,
            randr_snapshot_sha256: opening_randr.digest,
            drm_snapshot_sha256: opening_drm.digest,
            nvml_snapshot_sha256: opening_nvml.digest,
        },
        token_after: OutputTopologyTokenV1 {
            randr_timestamp: closing_randr.timestamp,
            randr_config_timestamp: closing_randr.config_timestamp,
            randr_snapshot_sha256: closing_randr.digest,
            drm_snapshot_sha256: closing_drm.digest,
            nvml_snapshot_sha256: closing_nvml.digest,
        },
        randr_outputs: opening_randr.outputs,
        randr_providers: opening_randr.providers,
        drm_connectors: opening_drm.connectors,
        nvml_devices: opening_nvml.devices,
    };
    let observation = OutputCollectorObservationV1 {
        requested_output,
        candidates,
        topology: Some(topology),
        collection_failure: None,
    };
    if validate_output_collection_observation(&observation) {
        observation
    } else {
        OutputCollectorObservationV1 {
            requested_output: observation.requested_output,
            candidates: observation.candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::InvalidObservation),
        }
    }
}

#[derive(Debug)]
struct RandrSnapshotV1 {
    timestamp: u32,
    config_timestamp: u32,
    outputs: Vec<RandrOutputObservationV1>,
    providers: Vec<crate::model::RandrProviderObservationV1>,
    digest: crate::Sha256DigestV1,
}

fn collect_randr_snapshot(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    root: u32,
) -> Result<RandrSnapshotV1, ()> {
    if connection
        .extension_information(x11rb::protocol::randr::X11_EXTENSION_NAME)
        .map_err(|_| ())?
        .is_none()
    {
        return Err(());
    }
    let version = connection
        .randr_query_version(1, 6)
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    if (version.major_version, version.minor_version) < (1, 4) {
        return Err(());
    }
    let resources = connection
        .randr_get_screen_resources_current(root)
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    let provider_resources = connection
        .randr_get_providers(root)
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    if resources.outputs.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
        || resources.crtcs.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
        || resources.modes.len() > MAX_OUTPUT_MAPPING_ITEMS_V1 * 4
        || provider_resources.providers.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
    {
        return Err(());
    }

    let atoms = RandrPropertyAtoms::intern(connection)?;
    let mut outputs = Vec::new();
    for output_xid in resources.outputs.iter().copied() {
        let info = connection
            .randr_get_output_info(output_xid, resources.config_timestamp)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?;
        if info.status != SetConfig::SUCCESS
            || info.connection != RandrConnection::CONNECTED
            || info.crtc == 0
            || info.name.is_empty()
            || info.name.len() > MAX_OUTPUT_NAME_BYTES_V1
            || info.clones.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
        {
            continue;
        }
        let crtc = connection
            .randr_get_crtc_info(info.crtc, resources.config_timestamp)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?;
        if crtc.status != SetConfig::SUCCESS || crtc.mode == 0 {
            continue;
        }
        let mode = resources
            .modes
            .iter()
            .find(|mode| mode.id == crtc.mode)
            .ok_or(())?;
        let connector_kind = output_connector_kind(connection, output_xid, atoms.connector_type)?;
        let connector_number = output_property_u32(connection, output_xid, atoms.connector_number)?;
        let non_desktop =
            output_property_u32(connection, output_xid, atoms.non_desktop)? != Some(0);
        let edid = output_property_bytes(connection, output_xid, atoms.edid, 4096)?;
        let name = output_name_from_bytes(&info.name).ok_or(())?;
        outputs.push(RandrOutputObservationV1 {
            output_xid: crate::model::XrandrOutputXidV1::new(output_xid),
            crtc_xid: crate::model::XrandrCrtcXidV1::new(info.crtc),
            mode_xid: crate::model::XrandrModeXidV1::new(crtc.mode),
            name,
            connected: true,
            physical: true,
            non_desktop,
            clone_output_xids: info
                .clones
                .iter()
                .copied()
                .map(crate::model::XrandrOutputXidV1::new)
                .collect(),
            connector_kind,
            connector_number,
            width_px: crtc.width,
            height_px: crtc.height,
            origin_x: crtc.x,
            origin_y: crtc.y,
            edid_sha256: edid.map(|bytes| sha256_bytes(&bytes)),
            timing: ExactModeTimingV1 {
                pixel_clock_hz: u64::from(mode.dot_clock),
                hdisplay: mode.width,
                hsync_start: mode.hsync_start,
                hsync_end: mode.hsync_end,
                htotal: mode.htotal,
                vdisplay: mode.height,
                vsync_start: mode.vsync_start,
                vsync_end: mode.vsync_end,
                vtotal: mode.vtotal,
                flags: u32::from(mode.mode_flags),
            },
        });
    }
    outputs.sort_by_key(|output| output.output_xid.get());

    let mut providers = Vec::new();
    for provider_xid in provider_resources.providers.iter().copied() {
        let info = connection
            .randr_get_provider_info(provider_xid, resources.config_timestamp)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?;
        if info.status != 0
            || info.name.is_empty()
            || info.name.len() > MAX_OUTPUT_NAME_BYTES_V1
            || info.crtcs.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
            || info.outputs.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
            || info.associated_providers.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
        {
            return Err(());
        }
        providers.push(crate::model::RandrProviderObservationV1 {
            provider_xid: crate::model::XrandrProviderXidV1::new(provider_xid),
            name: output_name_from_bytes(&info.name).ok_or(())?,
            capabilities: u32::from(info.capabilities),
            crtc_xids: info
                .crtcs
                .into_iter()
                .map(crate::model::XrandrCrtcXidV1::new)
                .collect(),
            output_xids: info
                .outputs
                .into_iter()
                .map(crate::model::XrandrOutputXidV1::new)
                .collect(),
            associated_provider_xids: info
                .associated_providers
                .into_iter()
                .map(crate::model::XrandrProviderXidV1::new)
                .collect(),
        });
    }
    providers.sort_by_key(|provider| provider.provider_xid.get());
    let digest = sha256_bytes(
        &serde_json::to_vec(&(
            resources.timestamp,
            resources.config_timestamp,
            &outputs,
            &providers,
        ))
        .map_err(|_| ())?,
    );
    Ok(RandrSnapshotV1 {
        timestamp: resources.timestamp,
        config_timestamp: resources.config_timestamp,
        outputs,
        providers,
        digest,
    })
}

#[derive(Clone, Copy)]
struct RandrPropertyAtoms {
    edid: Atom,
    connector_type: Atom,
    connector_number: Atom,
    non_desktop: Atom,
}

impl RandrPropertyAtoms {
    fn intern(
        connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    ) -> Result<Self, ()> {
        let atom = |name: &[u8]| {
            connection
                .intern_atom(false, name)
                .map_err(|_| ())?
                .reply()
                .map(|reply| reply.atom)
                .map_err(|_| ())
        };
        Ok(Self {
            edid: atom(b"EDID")?,
            connector_type: atom(b"ConnectorType")?,
            connector_number: atom(b"ConnectorNumber")?,
            non_desktop: atom(b"non-desktop")?,
        })
    }
}

fn output_property_reply(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    output: u32,
    property: Atom,
    long_length: u32,
) -> Result<x11rb::protocol::randr::GetOutputPropertyReply, ()> {
    let reply = connection
        .randr_get_output_property(
            output,
            property,
            AtomEnum::ANY,
            0,
            long_length,
            false,
            false,
        )
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    if reply.bytes_after != 0 {
        return Err(());
    }
    Ok(reply)
}

fn output_property_bytes(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    output: u32,
    property: Atom,
    maximum: usize,
) -> Result<Option<Vec<u8>>, ()> {
    let reply = output_property_reply(connection, output, property, 1024)?;
    if reply.type_ == u32::from(AtomEnum::NONE) || reply.num_items == 0 {
        return Ok(None);
    }
    if reply.format != 8 || reply.data.is_empty() || reply.data.len() > maximum {
        return Err(());
    }
    Ok(Some(reply.data))
}

fn output_property_u32(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    output: u32,
    property: Atom,
) -> Result<Option<u32>, ()> {
    let reply = output_property_reply(connection, output, property, 1)?;
    if reply.type_ == u32::from(AtomEnum::NONE) || reply.num_items == 0 {
        return Ok(None);
    }
    if reply.format != 32 || reply.num_items != 1 || reply.data.len() != 4 {
        return Err(());
    }
    Ok(Some(u32::from_ne_bytes(
        reply.data.as_slice().try_into().map_err(|_| ())?,
    )))
}

fn output_connector_kind(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    output: u32,
    property: Atom,
) -> Result<PhysicalConnectorKindV1, ()> {
    let reply = output_property_reply(connection, output, property, 64)?;
    if reply.type_ == u32::from(AtomEnum::NONE) || reply.num_items == 0 {
        return Err(());
    }
    let value = if reply.format == 8 {
        String::from_utf8(reply.data).map_err(|_| ())?
    } else if reply.format == 32 && reply.num_items == 1 && reply.data.len() == 4 {
        let atom = u32::from_ne_bytes(reply.data.as_slice().try_into().map_err(|_| ())?);
        let name = connection
            .get_atom_name(atom)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?
            .name;
        String::from_utf8(name).map_err(|_| ())?
    } else {
        return Err(());
    };
    connector_kind_from_text(value.trim_matches(char::from(0)).trim()).ok_or(())
}

fn connector_kind_from_text(value: &str) -> Option<PhysicalConnectorKindV1> {
    match value {
        "DisplayPort" | "DP" => Some(PhysicalConnectorKindV1::DisplayPort),
        "HDMI" | "HDMI-A" => Some(PhysicalConnectorKindV1::HdmiA),
        "HDMI-B" => Some(PhysicalConnectorKindV1::HdmiB),
        "DVI-D" => Some(PhysicalConnectorKindV1::DviD),
        "DVI-I" => Some(PhysicalConnectorKindV1::DviI),
        "DVI-A" => Some(PhysicalConnectorKindV1::DviA),
        "eDP" | "Panel" => Some(PhysicalConnectorKindV1::Edp),
        "LVDS" => Some(PhysicalConnectorKindV1::Lvds),
        "VGA" => Some(PhysicalConnectorKindV1::Vga),
        _ => None,
    }
}

fn output_name_from_bytes(bytes: &[u8]) -> Option<OutputNameV1> {
    if bytes.is_empty() || bytes.len() > MAX_OUTPUT_NAME_BYTES_V1 {
        return None;
    }
    Some(OutputNameV1 {
        hex: lower_hex_encode(bytes),
        display: std::str::from_utf8(bytes).ok().map(str::to_owned),
    })
}

#[derive(Debug)]
struct DrmSnapshotV1 {
    connectors: Vec<DrmConnectorObservationV1>,
    digest: crate::Sha256DigestV1,
}

#[derive(Debug)]
struct NvmlIdentitySnapshotV1 {
    devices: Vec<crate::model::NvmlDeviceIdentityObservationV1>,
    digest: crate::Sha256DigestV1,
}

fn collect_nvml_snapshot() -> Result<NvmlIdentitySnapshotV1, ()> {
    let observation = crate::native_nvml::observe_live_nvml();
    if observation.source_failure.is_some()
        || !observation.abi_verified
        || !observation.runtime.loaded
        || !observation.runtime.initialized
        || observation.runtime.failure.is_some()
        || !observation.runtime.shutdown_attempted
        || !observation.runtime.shutdown_succeeded
        || observation.runtime.devices.is_empty()
        || observation.runtime.devices.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
    {
        return Err(());
    }
    let mut devices = observation
        .runtime
        .devices
        .into_iter()
        .map(|device| crate::model::NvmlDeviceIdentityObservationV1 {
            nvml_pci_bdf: device.pci_bdf,
            nvml_uuid: device.uuid,
        })
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| {
        (&left.nvml_pci_bdf, &left.nvml_uuid).cmp(&(&right.nvml_pci_bdf, &right.nvml_uuid))
    });
    let digest = sha256_bytes(&serde_json::to_vec(&devices).map_err(|_| ())?);
    Ok(NvmlIdentitySnapshotV1 { devices, digest })
}

#[repr(C)]
struct DrmModeResources {
    count_fbs: c_int,
    fbs: *mut u32,
    count_crtcs: c_int,
    crtcs: *mut u32,
    count_connectors: c_int,
    connectors: *mut u32,
    count_encoders: c_int,
    encoders: *mut u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DrmModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    mode_type: u32,
    name: [c_char; 32],
}

#[repr(C)]
struct DrmModeConnector {
    connector_id: u32,
    encoder_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: c_int,
    mm_width: u32,
    mm_height: u32,
    subpixel: c_int,
    count_modes: c_int,
    modes: *mut DrmModeInfo,
    count_props: c_int,
    props: *mut u32,
    prop_values: *mut u64,
    count_encoders: c_int,
    encoders: *mut u32,
}

#[repr(C)]
struct DrmModeEncoder {
    encoder_id: u32,
    encoder_type: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    possible_clones: u32,
}

#[repr(C)]
struct DrmModeCrtc {
    crtc_id: u32,
    buffer_id: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    mode_valid: c_int,
    mode: DrmModeInfo,
    gamma_size: c_int,
}

#[repr(C)]
struct DrmModeProperty {
    prop_id: u32,
    flags: u32,
    name: [c_char; 32],
    count_values: c_int,
    values: *mut u64,
    count_enums: c_int,
    enums: *mut c_void,
    count_blobs: c_int,
    blob_ids: *mut u32,
}

#[repr(C)]
struct DrmModePropertyBlob {
    id: u32,
    length: u32,
    data: *mut c_void,
}

type DrmGetResourcesFn = unsafe extern "C" fn(c_int) -> *mut DrmModeResources;
type DrmFreeResourcesFn = unsafe extern "C" fn(*mut DrmModeResources);
type DrmGetConnectorFn = unsafe extern "C" fn(c_int, u32) -> *mut DrmModeConnector;
type DrmFreeConnectorFn = unsafe extern "C" fn(*mut DrmModeConnector);
type DrmGetEncoderFn = unsafe extern "C" fn(c_int, u32) -> *mut DrmModeEncoder;
type DrmFreeEncoderFn = unsafe extern "C" fn(*mut DrmModeEncoder);
type DrmGetCrtcFn = unsafe extern "C" fn(c_int, u32) -> *mut DrmModeCrtc;
type DrmFreeCrtcFn = unsafe extern "C" fn(*mut DrmModeCrtc);
type DrmGetPropertyFn = unsafe extern "C" fn(c_int, u32) -> *mut DrmModeProperty;
type DrmFreePropertyFn = unsafe extern "C" fn(*mut DrmModeProperty);
type DrmGetPropertyBlobFn = unsafe extern "C" fn(c_int, u32) -> *mut DrmModePropertyBlob;
type DrmFreePropertyBlobFn = unsafe extern "C" fn(*mut DrmModePropertyBlob);

struct DrmApi {
    _library: Library,
    get_resources: DrmGetResourcesFn,
    free_resources: DrmFreeResourcesFn,
    get_connector_current: DrmGetConnectorFn,
    free_connector: DrmFreeConnectorFn,
    get_encoder: DrmGetEncoderFn,
    free_encoder: DrmFreeEncoderFn,
    get_crtc: DrmGetCrtcFn,
    free_crtc: DrmFreeCrtcFn,
    get_property: DrmGetPropertyFn,
    free_property: DrmFreePropertyFn,
    get_property_blob: DrmGetPropertyBlobFn,
    free_property_blob: DrmFreePropertyBlobFn,
}

impl DrmApi {
    fn load() -> Result<Self, ()> {
        // SAFETY: The fixed SONAME is loaded without executing project-controlled paths.
        let library = unsafe { Library::new("libdrm.so.2") }.map_err(|_| ())?;
        // SAFETY: Every symbol name and declaration is copied from the installed
        // xf86drmMode.h ABI. Function pointers remain valid while `_library` lives.
        unsafe {
            let get_resources = *library
                .get::<DrmGetResourcesFn>(b"drmModeGetResources\0")
                .map_err(|_| ())?;
            let free_resources = *library
                .get::<DrmFreeResourcesFn>(b"drmModeFreeResources\0")
                .map_err(|_| ())?;
            let get_connector_current = *library
                .get::<DrmGetConnectorFn>(b"drmModeGetConnectorCurrent\0")
                .map_err(|_| ())?;
            let free_connector = *library
                .get::<DrmFreeConnectorFn>(b"drmModeFreeConnector\0")
                .map_err(|_| ())?;
            let get_encoder = *library
                .get::<DrmGetEncoderFn>(b"drmModeGetEncoder\0")
                .map_err(|_| ())?;
            let free_encoder = *library
                .get::<DrmFreeEncoderFn>(b"drmModeFreeEncoder\0")
                .map_err(|_| ())?;
            let get_crtc = *library
                .get::<DrmGetCrtcFn>(b"drmModeGetCrtc\0")
                .map_err(|_| ())?;
            let free_crtc = *library
                .get::<DrmFreeCrtcFn>(b"drmModeFreeCrtc\0")
                .map_err(|_| ())?;
            let get_property = *library
                .get::<DrmGetPropertyFn>(b"drmModeGetProperty\0")
                .map_err(|_| ())?;
            let free_property = *library
                .get::<DrmFreePropertyFn>(b"drmModeFreeProperty\0")
                .map_err(|_| ())?;
            let get_property_blob = *library
                .get::<DrmGetPropertyBlobFn>(b"drmModeGetPropertyBlob\0")
                .map_err(|_| ())?;
            let free_property_blob = *library
                .get::<DrmFreePropertyBlobFn>(b"drmModeFreePropertyBlob\0")
                .map_err(|_| ())?;
            Ok(Self {
                _library: library,
                get_resources,
                free_resources,
                get_connector_current,
                free_connector,
                get_encoder,
                free_encoder,
                get_crtc,
                free_crtc,
                get_property,
                free_property,
                get_property_blob,
                free_property_blob,
            })
        }
    }
}

fn collect_drm_snapshot() -> Result<DrmSnapshotV1, ()> {
    let api = DrmApi::load()?;
    let mut card_paths = std::fs::read_dir("/dev/dri")
        .map_err(|_| ())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("card").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                    })
                })
        })
        .collect::<Vec<_>>();
    card_paths.sort();
    if card_paths.is_empty() || card_paths.len() > MAX_OUTPUT_MAPPING_ITEMS_V1 {
        return Err(());
    }

    let mut connectors = Vec::new();
    for card_path in card_paths {
        let card = OpenOptions::new()
            .read(true)
            .open(&card_path)
            .map_err(|_| ())?;
        collect_drm_card(&api, &card, &mut connectors)?;
    }
    if connectors.len() > MAX_OUTPUT_MAPPING_ITEMS_V1 {
        return Err(());
    }
    connectors.sort_by_key(|connector| connector.connector_id.get());
    let mut identifiers = HashSet::with_capacity(connectors.len());
    if connectors
        .iter()
        .any(|connector| !identifiers.insert(connector.connector_id))
    {
        return Err(());
    }
    let digest = sha256_bytes(&serde_json::to_vec(&connectors).map_err(|_| ())?);
    Ok(DrmSnapshotV1 { connectors, digest })
}

fn collect_drm_card(
    api: &DrmApi,
    card: &File,
    output: &mut Vec<DrmConnectorObservationV1>,
) -> Result<(), ()> {
    // SAFETY: `card` is an open DRM node and the returned pointer is owned by
    // libdrm until the paired free call below.
    let resources_ptr = unsafe { (api.get_resources)(card.as_raw_fd()) };
    if resources_ptr.is_null() {
        return Err(());
    }
    // SAFETY: Non-null libdrm resource pointer remains live through this scope.
    let result = (|| {
        // SAFETY: Non-null libdrm resource pointer remains live through this closure.
        unsafe {
            let resources = &*resources_ptr;
            let count = bounded_c_count(resources.count_connectors)?;
            if count > 0 && resources.connectors.is_null() {
                return Err(());
            }
            let connector_ids = std::slice::from_raw_parts(resources.connectors, count).to_vec();
            for connector_id in connector_ids {
                if let Some(connector) = collect_drm_connector(api, card, connector_id)? {
                    output.push(connector);
                }
            }
            Ok(())
        }
    })();
    // SAFETY: Exactly pairs the successful `drmModeGetResources` call.
    unsafe { (api.free_resources)(resources_ptr) };
    result
}

unsafe fn collect_drm_connector(
    api: &DrmApi,
    card: &File,
    connector_id: u32,
) -> Result<Option<DrmConnectorObservationV1>, ()> {
    // SAFETY: The caller holds a valid DRM fd and resource-derived connector ID.
    let connector_ptr = unsafe { (api.get_connector_current)(card.as_raw_fd(), connector_id) };
    if connector_ptr.is_null() {
        return Err(());
    }
    let result = (|| {
        // SAFETY: Non-null connector pointer remains owned until the paired free.
        let connector = unsafe { &*connector_ptr };
        if connector.connection != 1 || connector.encoder_id == 0 {
            return Ok(None);
        }
        let kind = match drm_connector_kind(connector.connector_type) {
            Some(kind) => kind,
            None => return Ok(None),
        };
        let mode = unsafe { active_drm_mode(api, card, connector.encoder_id)? };
        let (connector_name, canonical_pci_bdfs, mst) =
            sysfs_connector_identity(connector.connector_id, kind, connector.connector_type_id)?;
        let properties = unsafe { read_drm_connector_properties(api, card, connector)? };
        if properties.non_desktop {
            return Ok(None);
        }
        Ok(Some(DrmConnectorObservationV1 {
            connector_id: crate::model::DrmConnectorIdV1::new(connector.connector_id),
            connector_kind: kind,
            connector_type_id: connector.connector_type_id,
            connector_name,
            connected: true,
            enabled: true,
            physical: true,
            mst,
            leased: false,
            edid_sha256: properties.edid_sha256,
            canonical_pci_bdfs,
            active_timing: drm_exact_timing(mode)?,
        }))
    })();
    // SAFETY: Exactly pairs the successful connector allocation.
    unsafe { (api.free_connector)(connector_ptr) };
    result
}

unsafe fn active_drm_mode(api: &DrmApi, card: &File, encoder_id: u32) -> Result<DrmModeInfo, ()> {
    // SAFETY: `encoder_id` came from a live connector on this fd.
    let encoder_ptr = unsafe { (api.get_encoder)(card.as_raw_fd(), encoder_id) };
    if encoder_ptr.is_null() {
        return Err(());
    }
    // SAFETY: Non-null encoder pointer is live until the paired free call.
    let crtc_id = unsafe { (*encoder_ptr).crtc_id };
    // SAFETY: Exactly pairs the successful encoder allocation.
    unsafe { (api.free_encoder)(encoder_ptr) };
    if crtc_id == 0 {
        return Err(());
    }
    // SAFETY: `crtc_id` came from the live encoder.
    let crtc_ptr = unsafe { (api.get_crtc)(card.as_raw_fd(), crtc_id) };
    if crtc_ptr.is_null() {
        return Err(());
    }
    // SAFETY: Copying fixed-width C fields before freeing the libdrm object.
    let (valid, mode) = unsafe { ((*crtc_ptr).mode_valid != 0, (*crtc_ptr).mode) };
    // SAFETY: Exactly pairs the successful CRTC allocation.
    unsafe { (api.free_crtc)(crtc_ptr) };
    valid.then_some(mode).ok_or(())
}

#[derive(Debug)]
struct DrmConnectorPropertiesV1 {
    edid_sha256: Option<crate::Sha256DigestV1>,
    non_desktop: bool,
}

unsafe fn read_drm_connector_properties(
    api: &DrmApi,
    card: &File,
    connector: &DrmModeConnector,
) -> Result<DrmConnectorPropertiesV1, ()> {
    let count = bounded_c_count(connector.count_props)?;
    if count > 0 && (connector.props.is_null() || connector.prop_values.is_null()) {
        return Err(());
    }
    // SAFETY: libdrm reports arrays with exactly `count_props` elements.
    let ids = unsafe { std::slice::from_raw_parts(connector.props, count) };
    // SAFETY: Same ownership and count as the property ID array.
    let values = unsafe { std::slice::from_raw_parts(connector.prop_values, count) };
    let mut edid_sha256 = None;
    let mut non_desktop = false;
    for (&property_id, &value) in ids.iter().zip(values) {
        // SAFETY: Property IDs originate from this live connector.
        let property_ptr = unsafe { (api.get_property)(card.as_raw_fd(), property_id) };
        if property_ptr.is_null() {
            return Err(());
        }
        // SAFETY: Property pointer is non-null and its name is a fixed C array.
        let name = unsafe { bounded_c_name(&(*property_ptr).name) };
        let property_result = match name.as_deref() {
            Some("EDID") if value != 0 => {
                // SAFETY: The property value is the blob ID for the EDID property.
                let blob_ptr = unsafe { (api.get_property_blob)(card.as_raw_fd(), value as u32) };
                if blob_ptr.is_null() {
                    Err(())
                } else {
                    // SAFETY: Non-null blob pointer is live until the paired free.
                    let blob = unsafe { &*blob_ptr };
                    let length = usize::try_from(blob.length).map_err(|_| ())?;
                    let digest = if length == 0 || length > 4096 || blob.data.is_null() {
                        Err(())
                    } else {
                        // SAFETY: libdrm supplies `length` readable blob bytes.
                        let bytes =
                            unsafe { std::slice::from_raw_parts(blob.data.cast::<u8>(), length) };
                        Ok(sha256_bytes(bytes))
                    };
                    // SAFETY: Exactly pairs the successful blob allocation.
                    unsafe { (api.free_property_blob)(blob_ptr) };
                    digest.map(|digest| edid_sha256 = Some(digest))
                }
            }
            Some("non-desktop") => {
                non_desktop = value != 0;
                Ok(())
            }
            _ => Ok(()),
        };
        // SAFETY: Exactly pairs the successful property allocation.
        unsafe { (api.free_property)(property_ptr) };
        property_result?;
    }
    Ok(DrmConnectorPropertiesV1 {
        edid_sha256,
        non_desktop,
    })
}

fn bounded_c_count(value: c_int) -> Result<usize, ()> {
    let value = usize::try_from(value).map_err(|_| ())?;
    (value <= MAX_OUTPUT_MAPPING_ITEMS_V1 * 4)
        .then_some(value)
        .ok_or(())
}

fn bounded_c_name(value: &[c_char; 32]) -> Option<String> {
    let bytes = value
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();
    (!bytes.is_empty())
        .then(|| String::from_utf8(bytes).ok())
        .flatten()
}

fn drm_exact_timing(mode: DrmModeInfo) -> Result<ExactModeTimingV1, ()> {
    Ok(ExactModeTimingV1 {
        pixel_clock_hz: u64::from(mode.clock).checked_mul(1000).ok_or(())?,
        hdisplay: mode.hdisplay,
        hsync_start: mode.hsync_start,
        hsync_end: mode.hsync_end,
        htotal: mode.htotal,
        vdisplay: mode.vdisplay,
        vsync_start: mode.vsync_start,
        vsync_end: mode.vsync_end,
        vtotal: mode.vtotal,
        flags: mode.flags,
    })
}

fn drm_connector_kind(value: u32) -> Option<PhysicalConnectorKindV1> {
    match value {
        1 => Some(PhysicalConnectorKindV1::Vga),
        2 => Some(PhysicalConnectorKindV1::DviI),
        3 => Some(PhysicalConnectorKindV1::DviD),
        4 => Some(PhysicalConnectorKindV1::DviA),
        7 => Some(PhysicalConnectorKindV1::Lvds),
        10 => Some(PhysicalConnectorKindV1::DisplayPort),
        11 => Some(PhysicalConnectorKindV1::HdmiA),
        12 => Some(PhysicalConnectorKindV1::HdmiB),
        14 => Some(PhysicalConnectorKindV1::Edp),
        _ => None,
    }
}

fn sysfs_connector_identity(
    connector_id: u32,
    kind: PhysicalConnectorKindV1,
    type_id: u32,
) -> Result<(String, Vec<String>, bool), ()> {
    let mut matches = Vec::new();
    for entry in std::fs::read_dir("/sys/class/drm")
        .map_err(|_| ())?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("card") || !file_name.contains('-') {
            continue;
        }
        let Some(observed_id) = read_small_text(&path.join("connector_id"))
            .and_then(|value| value.trim().parse::<u32>().ok())
        else {
            continue;
        };
        if observed_id == connector_id {
            matches.push(path);
        }
    }
    if matches.len() != 1 {
        return Err(());
    }
    let path = matches.pop().expect("one checked sysfs match");
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or(())?;
    let dash = file_name.find('-').ok_or(())?;
    let connector_name = file_name.get(dash + 1..).ok_or(())?.to_owned();
    if !valid_visible_ascii(&connector_name, MAX_CONNECTOR_NAME_BYTES_V1) {
        return Err(());
    }
    let expected_name = canonical_connector_name(kind, type_id);
    let mst = connector_name != expected_name;
    if mst && !connector_name.starts_with(&(expected_name + "-")) {
        return Err(());
    }

    let device = std::fs::canonicalize(path.join("device")).map_err(|_| ())?;
    let pci_ancestor = device
        .ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(valid_sysfs_pci_bdf)
        })
        .ok_or(())?;
    let bdf = pci_ancestor
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(())?;
    let bus_device =
        std::fs::canonicalize(Path::new("/sys/bus/pci/devices").join(bdf)).map_err(|_| ())?;
    if bus_device != pci_ancestor {
        return Err(());
    }
    Ok((
        connector_name,
        vec![canonicalize_sysfs_pci_bdf(bdf).ok_or(())?],
        mst,
    ))
}

fn canonical_connector_name(kind: PhysicalConnectorKindV1, type_id: u32) -> String {
    let prefix = match kind {
        PhysicalConnectorKindV1::DisplayPort => "DP",
        PhysicalConnectorKindV1::HdmiA => "HDMI-A",
        PhysicalConnectorKindV1::HdmiB => "HDMI-B",
        PhysicalConnectorKindV1::DviD => "DVI-D",
        PhysicalConnectorKindV1::DviI => "DVI-I",
        PhysicalConnectorKindV1::DviA => "DVI-A",
        PhysicalConnectorKindV1::Edp => "eDP",
        PhysicalConnectorKindV1::Lvds => "LVDS",
        PhysicalConnectorKindV1::Vga => "VGA",
    };
    format!("{prefix}-{type_id}")
}

fn read_small_text(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(4097)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn valid_sysfs_pci_bdf(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 12
        && bytes[4] == b':'
        && bytes[7] == b':'
        && bytes[10] == b'.'
        && matches!(bytes[11], b'0'..=b'7')
        && bytes[..4].iter().all(|byte| lower_hex(*byte))
        && bytes[5..7].iter().all(|byte| lower_hex(*byte))
        && bytes[8..10].iter().all(|byte| lower_hex(*byte))
}

fn canonicalize_sysfs_pci_bdf(value: &str) -> Option<String> {
    valid_sysfs_pci_bdf(value).then(|| format!("0000{value}"))
}

fn validate_observation(
    observation: &OutputTopologyObservationV1,
) -> Result<(), OutputMappingFailureV1> {
    if !bounded(&observation.randr_outputs)
        || !bounded(&observation.randr_providers)
        || !bounded(&observation.drm_connectors)
        || !bounded(&observation.nvml_devices)
        || !valid_output_name(&observation.requested_output_name)
    {
        return Err(invalid_observation());
    }

    let mut output_ids = HashSet::with_capacity(observation.randr_outputs.len());
    for output in &observation.randr_outputs {
        if output.output_xid.get() == 0
            || !output_ids.insert(output.output_xid)
            || !valid_output_name(&output.name)
            || !bounded(&output.clone_output_xids)
            || output
                .clone_output_xids
                .iter()
                .any(|candidate| candidate.get() == 0)
        {
            return Err(invalid_observation());
        }
    }

    let mut provider_ids = HashSet::with_capacity(observation.randr_providers.len());
    for provider in &observation.randr_providers {
        if provider.provider_xid.get() == 0
            || !provider_ids.insert(provider.provider_xid)
            || !valid_output_name(&provider.name)
            || provider.capabilities & !RANDR_PROVIDER_KNOWN_CAPABILITIES != 0
            || !bounded(&provider.crtc_xids)
            || !bounded(&provider.output_xids)
            || !bounded(&provider.associated_provider_xids)
            || provider.crtc_xids.iter().any(|xid| xid.get() == 0)
            || provider.output_xids.iter().any(|xid| xid.get() == 0)
            || provider
                .associated_provider_xids
                .iter()
                .any(|xid| xid.get() == 0)
        {
            return Err(invalid_observation());
        }
    }

    let mut connector_ids = HashSet::with_capacity(observation.drm_connectors.len());
    for connector in &observation.drm_connectors {
        if connector.connector_id.get() == 0
            || !connector_ids.insert(connector.connector_id)
            || connector.connector_type_id == 0
            || !valid_visible_ascii(&connector.connector_name, MAX_CONNECTOR_NAME_BYTES_V1)
            || !bounded(&connector.canonical_pci_bdfs)
            || connector
                .canonical_pci_bdfs
                .iter()
                .any(|bdf| !valid_canonical_pci_bdf(bdf))
            || connector.enabled && !valid_timing(&connector.active_timing)
        {
            return Err(invalid_observation());
        }
    }

    if observation.nvml_devices.iter().any(|device| {
        !valid_canonical_pci_bdf(&device.nvml_pci_bdf) || !valid_nvml_uuid(&device.nvml_uuid)
    }) {
        return Err(invalid_observation());
    }

    Ok(())
}

fn validate_selected_output(
    output: &RandrOutputObservationV1,
) -> Result<(), OutputMappingFailureV1> {
    if !output.connected || !output.physical || output.non_desktop {
        return Err(failure(
            OutputMappingReasonV1::UnsupportedTopology,
            OutputMappingRelationV1::RequestedOutput,
            Some(1),
        ));
    }
    if !output.clone_output_xids.is_empty() {
        return Err(failure(
            OutputMappingReasonV1::UnsupportedTopology,
            OutputMappingRelationV1::RequestedOutput,
            Some(1),
        ));
    }
    if output.crtc_xid.get() == 0 || output.mode_xid.get() == 0 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::RequestedOutput,
            0,
        ));
    }
    if !valid_timing(&output.timing) {
        return Err(invalid_observation());
    }
    if output.width_px != output.timing.hdisplay || output.height_px != output.timing.vdisplay {
        return Err(failure(
            OutputMappingReasonV1::ConflictingFacts,
            OutputMappingRelationV1::RequestedOutput,
            Some(1),
        ));
    }
    Ok(())
}

fn validate_selected_connector(
    connector: &DrmConnectorObservationV1,
) -> Result<(), OutputMappingFailureV1> {
    if !connector.connected
        || !connector.enabled
        || !connector.physical
        || connector.mst
        || connector.leased
    {
        return Err(failure(
            OutputMappingReasonV1::UnsupportedTopology,
            OutputMappingRelationV1::DrmConnector,
            Some(1),
        ));
    }
    Ok(())
}

fn exact_refresh_rate(timing: &ExactModeTimingV1) -> Option<RefreshRateV1> {
    if !valid_timing(timing) {
        return None;
    }
    let mut numerator = timing.pixel_clock_hz;
    let mut denominator = u64::from(timing.htotal).checked_mul(u64::from(timing.vtotal))?;
    if timing.flags & RANDR_MODE_INTERLACE != 0 {
        numerator = numerator.checked_mul(2)?;
    }
    if timing.flags & RANDR_MODE_DOUBLE_SCAN != 0 {
        denominator = denominator.checked_mul(2)?;
    }
    let divisor = greatest_common_divisor(numerator, denominator);
    Some(RefreshRateV1 {
        numerator: numerator / divisor,
        denominator: denominator / divisor,
    })
}

fn valid_timing(timing: &ExactModeTimingV1) -> bool {
    timing.pixel_clock_hz > 0
        && timing.pixel_clock_hz <= u64::from(u32::MAX)
        && timing.hdisplay > 0
        && timing.hdisplay <= timing.hsync_start
        && timing.hsync_start < timing.hsync_end
        && timing.hsync_end <= timing.htotal
        && timing.vdisplay > 0
        && timing.vdisplay <= timing.vsync_start
        && timing.vsync_start < timing.vsync_end
        && timing.vsync_end <= timing.vtotal
        && timing.flags & !RANDR_MODE_EXACT_REFRESH_FLAGS == 0
}

fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn valid_output_name(name: &OutputNameV1) -> bool {
    if name.hex.is_empty()
        || name.hex.len() > MAX_OUTPUT_NAME_BYTES_V1 * 2
        || !name.hex.len().is_multiple_of(2)
        || !name.hex.bytes().all(lower_hex)
    {
        return false;
    }
    let Some(bytes) = decode_lower_hex(&name.hex) else {
        return false;
    };
    if bytes.is_empty() || bytes.len() > MAX_OUTPUT_NAME_BYTES_V1 {
        return false;
    }
    match &name.display {
        None => true,
        Some(display) => std::str::from_utf8(&bytes).is_ok_and(|decoded| decoded == display),
    }
}

pub fn validate_output_collection_observation(observation: &OutputCollectorObservationV1) -> bool {
    if !valid_candidates(&observation.candidates)
        || observation
            .requested_output
            .as_ref()
            .is_some_and(|name| !valid_output_name(name))
    {
        return false;
    }
    match (&observation.requested_output, &observation.topology) {
        (None, None) => true,
        (Some(requested), Some(topology)) => {
            observation.collection_failure.is_none()
                && topology.requested_output_name == *requested
                && validate_observation(topology).is_ok()
        }
        (Some(_), None) => observation.collection_failure.is_some(),
        (None, Some(_)) => false,
    }
}

pub fn validate_output_name_evidence(name: &OutputNameV1) -> bool {
    valid_output_name(name)
}

fn valid_candidates(candidates: &[OutputNameV1]) -> bool {
    if candidates.len() > MAX_OUTPUT_MAPPING_ITEMS_V1 {
        return false;
    }
    let mut previous = None;
    let mut identifiers = HashSet::with_capacity(candidates.len());
    for candidate in candidates {
        if !valid_output_name(candidate)
            || !identifiers.insert(candidate.hex.as_str())
            || previous.is_some_and(|value: &str| value >= candidate.hex.as_str())
        {
            return false;
        }
        previous = Some(candidate.hex.as_str());
    }
    true
}

fn candidates_from_outputs(outputs: &[RandrOutputObservationV1]) -> Vec<OutputNameV1> {
    let mut candidates = outputs
        .iter()
        .filter(|output| {
            output.connected
                && output.physical
                && !output.non_desktop
                && output.crtc_xid.get() != 0
                && output.mode_xid.get() != 0
                && output.clone_output_xids.is_empty()
        })
        .map(|output| output.name.clone())
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.hex.cmp(&right.hex));
    candidates.dedup_by(|left, right| left.hex == right.hex);
    candidates
}

fn lower_hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn decode_lower_hex(value: &str) -> Option<Vec<u8>> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = lower_hex_value(pair[0])?;
            let low = lower_hex_value(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

const fn lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

const fn lower_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn valid_visible_ascii(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_nvml_uuid(value: &str) -> bool {
    value.starts_with("GPU-")
        && value.len() < MAX_NVML_UUID_BYTES_V1
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_canonical_pci_bdf(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 16
        && bytes[8] == b':'
        && bytes[11] == b':'
        && bytes[14] == b'.'
        && matches!(bytes[15], b'0'..=b'7')
        && bytes[..8].iter().all(|byte| lower_hex(*byte))
        && bytes[9..11].iter().all(|byte| lower_hex(*byte))
        && bytes[12..14].iter().all(|byte| lower_hex(*byte))
}

fn bounded<T>(items: &[T]) -> bool {
    items.len() <= MAX_OUTPUT_MAPPING_ITEMS_V1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host02FixtureDocumentV1 {
    schema: String,
    fixture_semantics: String,
    base_topology: Value,
    cases: Vec<Host02FixtureCaseV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host02FixtureCaseV1 {
    id: String,
    topology_patch: Value,
    expected: Host02FixtureExpectedV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host02FixtureExpectedV1 {
    status: String,
    reason: Option<String>,
}

fn unique_case_ids(cases: &[Host02FixtureCaseV1]) -> bool {
    let mut identifiers = HashSet::with_capacity(cases.len());
    cases.iter().all(|case| {
        !case.id.is_empty()
            && case.id.len() <= 64
            && case.id.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.')
            })
            && identifiers.insert(case.id.as_str())
    })
}

fn overlay_json(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, patch_value) in patch {
                if let Some(target_value) = target.get_mut(key) {
                    overlay_json(target_value, patch_value);
                } else {
                    target.insert(key.clone(), patch_value.clone());
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

fn cardinality_failure(relation: OutputMappingRelationV1, actual: usize) -> OutputMappingFailureV1 {
    failure(
        OutputMappingReasonV1::Ambiguous,
        relation,
        Some(u32::try_from(actual).unwrap_or(u32::MAX)),
    )
}

fn invalid_observation() -> OutputMappingFailureV1 {
    failure(
        OutputMappingReasonV1::InvalidObservation,
        OutputMappingRelationV1::Observation,
        None,
    )
}

const fn failure(
    reason: OutputMappingReasonV1,
    relation: OutputMappingRelationV1,
    observed_cardinality: Option<u32>,
) -> OutputMappingFailureV1 {
    OutputMappingFailureV1 {
        reason,
        relation,
        observed_cardinality,
    }
}

#[cfg(test)]
mod collector_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn host02_live_collector_fixture_rechecks_topology_before_admission() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/host02-output-topologies.json");

        let stable =
            collect_fixture_output_topology(&fixture, "namespace-disjoint-unique", Some("DP-0"))
                .expect("stable fixture collection must succeed");
        let selected = stable
            .topology
            .as_ref()
            .map(prove_output_gpu_mapping)
            .expect("an explicit selection must carry a topology")
            .expect("the full unequal-ID relation must pass");
        assert_eq!(selected.output_name.display.as_deref(), Some("DP-0"));
        assert_ne!(
            selected.randr_output_xid.get(),
            selected.drm_connector_id.get()
        );

        let raced = collect_fixture_output_topology(&fixture, "topology-changed", Some("DP-0"))
            .expect("the collector must preserve a changed closing token");
        let failure = raced
            .topology
            .as_ref()
            .map(prove_output_gpu_mapping)
            .expect("an explicit selection must carry a topology")
            .expect_err("a topology race must remove selection");
        assert_eq!(failure.reason, OutputMappingReasonV1::TopologyChanged);
    }
}
