use std::collections::HashSet;
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::Path;

use libloading::Library;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use x11rb::connection::{Connection as _, RequestConnection as _};
use x11rb::protocol::Event;
use x11rb::protocol::randr::{
    Connection as RandrConnection, ConnectionExt as _, NotifyMask, SetConfig,
};
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _};

use crate::digest::sha256_bytes;
use crate::model::{
    DrmConnectorObservationV1, DrmDiagnosticSnapshotV1, ExactModeTimingV1,
    MAX_CONNECTOR_NAME_BYTES_V1, MAX_NVCONTROL_STRING_BYTES_V1, MAX_NVML_UUID_BYTES_V1,
    MAX_OUTPUT_MAPPING_ITEMS_V1, MAX_OUTPUT_NAME_BYTES_V1, NvControlDisplayTargetIdV1,
    NvControlDisplayTargetObservationV1, NvControlExtensionVersionV1, NvControlGpuTargetIdV1,
    NvControlGpuTargetObservationV1, NvControlSnapshotV1, OutputMappingProofCardinalitiesV1,
    OutputNameV1, OutputTopologyObservationV1, OutputTopologyTokenV1, PhysicalConnectorKindV1,
    RandrOutputObservationV1, RefreshRateV1, SELECTED_OUTPUT_SCHEMA_V1, SelectedOutputV1,
};

pub const SELECTED_OUTPUT_DISCOVERY_SCHEMA_V1: &str = "replaydesktop.selected-output-discovery.v1";
pub const SELECTED_OUTPUT_FAILURE_SCHEMA_V1: &str = "replaydesktop.selected-output-failure.v1";

const RANDR_PROVIDER_KNOWN_CAPABILITIES: u32 = 0x0f;
const RANDR_MODE_INTERLACE: u32 = 1 << 4;
const RANDR_MODE_DOUBLE_SCAN: u32 = 1 << 5;
const RANDR_MODE_EXACT_REFRESH_FLAGS: u32 = 0x07ff;
const HOST02_FIXTURE_SCHEMA_V1: &str = "replaydesktop.host02-output-topologies.v1";
const MAX_HOST02_FIXTURE_BYTES: usize = 1024 * 1024;
const MIN_NVCONTROL_VERSION: (u32, u32) = (1, 27);
const X11_LIBRARY_NAME: &str = "libX11.so.6";
const XNVCTRL_LIBRARY_NAME: &str = "libXNVCtrl.so.0";
const NV_CTRL_TARGET_TYPE_X_SCREEN: c_int = 0;
const NV_CTRL_TARGET_TYPE_GPU: c_int = 1;
const NV_CTRL_TARGET_TYPE_DISPLAY: c_int = 8;
const NV_CTRL_BINARY_DATA_DISPLAY_TARGETS: c_uint = 14;
const NV_CTRL_BINARY_DATA_DISPLAYS_CONNECTED_TO_GPU: c_uint = 15;
const NV_CTRL_BINARY_DATA_DISPLAYS_ENABLED_ON_XSCREEN: c_uint = 17;
const NV_CTRL_PCI_BUS: c_uint = 239;
const NV_CTRL_PCI_DEVICE: c_uint = 240;
const NV_CTRL_PCI_FUNCTION: c_uint = 241;
const NV_CTRL_PCI_DOMAIN: c_uint = 306;
const NV_CTRL_DISPLAY_ENABLED: c_uint = 388;
const NV_CTRL_DISPLAY_RANDR_OUTPUT_ID: c_uint = 391;
const NV_CTRL_DISPLAYPORT_IS_MULTISTREAM: c_uint = 422;
const NV_CTRL_STRING_DISPLAY_NAME_TYPE_ID: c_uint = 47;
const NV_CTRL_STRING_DISPLAY_NAME_DP_GUID: c_uint = 48;
const NV_CTRL_STRING_DISPLAY_NAME_EDID_HASH: c_uint = 49;
const NV_CTRL_STRING_DISPLAY_NAME_TARGET_INDEX: c_uint = 50;
const NV_CTRL_STRING_DISPLAY_NAME_RANDR: c_uint = 51;
const NV_CTRL_STRING_GPU_UUID: c_uint = 52;
const MAX_RANDR_EVENTS: u32 = 64;

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
    #[serde(rename = "BLOCKED_NVCONTROL_UNAVAILABLE")]
    NvControlUnavailable,
}

impl OutputMappingReasonV1 {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::Ambiguous => "BLOCKED_AMBIGUOUS",
            Self::ConflictingFacts => "BLOCKED_CONFLICTING_FACTS",
            Self::TopologyChanged => "BLOCKED_TOPOLOGY_CHANGED",
            Self::UnsupportedTopology => "BLOCKED_UNSUPPORTED_TOPOLOGY",
            Self::InvalidObservation => "BLOCKED_INVALID_OBSERVATION",
            Self::NvControlUnavailable => "BLOCKED_NVCONTROL_UNAVAILABLE",
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
    NvControlExtension,
    NvControlDisplayTarget,
    NvControlEnabledOnXscreen,
    NvControlGpuOwner,
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
    X11Unavailable,
    NvControlUnavailable,
    NvmlUnavailable,
    InvalidObservation,
}

impl OutputCollectionFailureV1 {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::XorgUnavailable => "xorg-unavailable",
            Self::RandrUnavailable => "randr-unavailable",
            Self::X11Unavailable => "x11-unavailable",
            Self::NvControlUnavailable => "nvcontrol-unavailable",
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

    if observation.token_before != observation.token_after
        || observation.token_before.randr_event_count != 0
    {
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
    let refresh_hz = exact_refresh_rate(&output.timing).ok_or_else(invalid_observation)?;

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
        })
        .collect::<Vec<_>>();
    if provider_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::RandrProvider,
            provider_matches.len(),
        ));
    }
    let provider = provider_matches[0];

    let Some(extension_version) = observation.nvcontrol.extension_version else {
        return Err(failure(
            OutputMappingReasonV1::NvControlUnavailable,
            OutputMappingRelationV1::NvControlExtension,
            None,
        ));
    };
    if !observation.nvcontrol.extension_present
        || (extension_version.major, extension_version.minor) < MIN_NVCONTROL_VERSION
    {
        return Err(failure(
            OutputMappingReasonV1::NvControlUnavailable,
            OutputMappingRelationV1::NvControlExtension,
            None,
        ));
    }
    if !observation.nvcontrol.x_screen_is_nvidia {
        return Err(failure(
            OutputMappingReasonV1::UnsupportedTopology,
            OutputMappingRelationV1::NvControlExtension,
            Some(1),
        ));
    }

    let display_matches = observation
        .nvcontrol
        .display_targets
        .iter()
        .filter(|target| target.randr_output_xid == output.output_xid)
        .collect::<Vec<_>>();
    if display_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvControlDisplayTarget,
            display_matches.len(),
        ));
    }
    let display_target = display_matches[0];
    let Some(display_randr_name) = display_target.randr_name.as_ref() else {
        return Err(failure(
            OutputMappingReasonV1::NvControlUnavailable,
            OutputMappingRelationV1::NvControlDisplayTarget,
            None,
        ));
    };
    if display_randr_name.hex != output.name.hex {
        return Err(failure(
            OutputMappingReasonV1::ConflictingFacts,
            OutputMappingRelationV1::NvControlDisplayTarget,
            Some(1),
        ));
    }
    match display_target.enabled {
        Some(true) => {}
        Some(false) => {
            return Err(failure(
                OutputMappingReasonV1::UnsupportedTopology,
                OutputMappingRelationV1::NvControlDisplayTarget,
                Some(1),
            ));
        }
        None => {
            return Err(failure(
                OutputMappingReasonV1::NvControlUnavailable,
                OutputMappingRelationV1::NvControlDisplayTarget,
                None,
            ));
        }
    }
    let target_index_name = display_target
        .target_index_name
        .as_ref()
        .ok_or_else(invalid_observation)?;
    let type_id_name = display_target
        .type_id_name
        .as_ref()
        .ok_or_else(invalid_observation)?;
    let displayport_is_multistream = display_target
        .displayport_is_multistream
        .ok_or_else(invalid_observation)?;

    let enabled_matches = observation
        .nvcontrol
        .enabled_display_target_ids
        .iter()
        .filter(|candidate| **candidate == display_target.target_id)
        .count();
    if enabled_matches != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvControlEnabledOnXscreen,
            enabled_matches,
        ));
    }

    let gpu_matches = observation
        .nvcontrol
        .gpu_targets
        .iter()
        .filter(|gpu| {
            gpu.connected_display_target_ids
                .iter()
                .filter(|candidate| **candidate == display_target.target_id)
                .count()
                == 1
        })
        .collect::<Vec<_>>();
    if gpu_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvControlGpuOwner,
            gpu_matches.len(),
        ));
    }
    let gpu = gpu_matches[0];
    let nvml_matches = observation
        .nvml_devices
        .iter()
        .filter(|device| {
            device.nvml_pci_bdf == gpu.canonical_pci_bdf && device.nvml_uuid == gpu.uuid
        })
        .collect::<Vec<_>>();
    if nvml_matches.len() != 1 {
        return Err(cardinality_failure(
            OutputMappingRelationV1::NvmlDevice,
            nvml_matches.len(),
        ));
    }
    let nvml_device = nvml_matches[0];

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
        randr_primary: output.primary,
        refresh_hz,
        exact_timing: output.timing.clone(),
        edid_sha256: output.edid_sha256,
        randr_connector_kind: output.connector_kind,
        randr_connector_number: output.connector_number,
        randr_provider_name: provider.name.clone(),
        randr_provider_capabilities: provider.capabilities,
        x11_library: observation.nvcontrol.x11_library.clone(),
        xnvctrl_library: observation.nvcontrol.xnvctrl_library.clone(),
        nvcontrol_extension_version: extension_version,
        nvcontrol_x_screen: observation.nvcontrol.x_screen,
        nvcontrol_display_target_id: display_target.target_id,
        nvcontrol_display_randr_name: display_randr_name.clone(),
        nvcontrol_display_target_index_name: target_index_name.clone(),
        nvcontrol_display_type_id_name: type_id_name.clone(),
        nvcontrol_display_dp_guid: display_target.dp_guid.clone(),
        nvcontrol_display_edid_hash: display_target.edid_hash.clone(),
        nvcontrol_display_enabled: true,
        nvcontrol_displayport_is_multistream: displayport_is_multistream,
        nvcontrol_gpu_target_id: gpu.target_id,
        nvcontrol_gpu_pci_domain: gpu.pci_domain,
        nvcontrol_gpu_pci_bus: gpu.pci_bus,
        nvcontrol_gpu_pci_device: gpu.pci_device,
        nvcontrol_gpu_pci_function: gpu.pci_function,
        nvcontrol_gpu_pci_bdf: gpu.canonical_pci_bdf.clone(),
        nvcontrol_gpu_uuid: gpu.uuid.clone(),
        nvml_pci_bdf: nvml_device.nvml_pci_bdf.clone(),
        nvml_uuid: nvml_device.nvml_uuid.clone(),
        drm_diagnostic_before: observation.drm_diagnostic_before.clone(),
        drm_diagnostic_after: observation.drm_diagnostic_after.clone(),
        topology_token: observation.token_before.clone(),
        proof: OutputMappingProofCardinalitiesV1 {
            requested_output_matches: 1,
            provider_matches: 1,
            nvcontrol_display_target_matches: 1,
            enabled_on_xscreen_matches: 1,
            nvcontrol_gpu_owner_matches: 1,
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
        && valid_output_name(&selected.randr_provider_name)
        && selected.randr_provider_capabilities & !RANDR_PROVIDER_KNOWN_CAPABILITIES == 0
        && valid_library_identity(&selected.x11_library, "libX11.so.")
        && valid_library_identity(&selected.xnvctrl_library, "libXNVCtrl.so.")
        && (
            selected.nvcontrol_extension_version.major,
            selected.nvcontrol_extension_version.minor,
        ) >= MIN_NVCONTROL_VERSION
        && selected.nvcontrol_display_randr_name == selected.output_name
        && valid_nvcontrol_string(&selected.nvcontrol_display_target_index_name)
        && valid_nvcontrol_string(&selected.nvcontrol_display_type_id_name)
        && selected
            .nvcontrol_display_dp_guid
            .as_deref()
            .is_none_or(valid_nvcontrol_string)
        && selected
            .nvcontrol_display_edid_hash
            .as_deref()
            .is_none_or(valid_nvcontrol_string)
        && selected.nvcontrol_display_enabled
        && canonical_nvcontrol_pci_bdf(
            selected.nvcontrol_gpu_pci_domain,
            selected.nvcontrol_gpu_pci_bus,
            selected.nvcontrol_gpu_pci_device,
            selected.nvcontrol_gpu_pci_function,
        )
        .as_deref()
            == Some(selected.nvcontrol_gpu_pci_bdf.as_str())
        && valid_nvml_uuid(&selected.nvcontrol_gpu_uuid)
        && valid_canonical_pci_bdf(&selected.nvml_pci_bdf)
        && valid_nvml_uuid(&selected.nvml_uuid)
        && selected.nvcontrol_gpu_pci_bdf == selected.nvml_pci_bdf
        && selected.nvcontrol_gpu_uuid == selected.nvml_uuid
        && selected.randr_timestamp == selected.topology_token.randr_timestamp
        && selected.randr_config_timestamp == selected.topology_token.randr_config_timestamp
        && selected.topology_token.randr_event_count == 0
        && selected.proof.requested_output_matches == 1
        && selected.proof.provider_matches == 1
        && selected.proof.nvcontrol_display_target_matches == 1
        && selected.proof.enabled_on_xscreen_matches == 1
        && selected.proof.nvcontrol_gpu_owner_matches == 1
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
    if subscribe_randr_topology_events(&xorg.connection, xorg.root).is_err() {
        return failure(OutputCollectionFailureV1::X11Unavailable);
    }
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

    let opening_drm = collect_drm_snapshot().ok().map(drm_diagnostic_snapshot);
    let Ok(opening_nvcontrol) = collect_nvcontrol_snapshot(xorg.display, xorg.screen) else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::NvControlUnavailable),
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
    let Ok(closing_nvcontrol) = collect_nvcontrol_snapshot(xorg.display, xorg.screen) else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::NvControlUnavailable),
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
    let closing_drm = collect_drm_snapshot().ok().map(drm_diagnostic_snapshot);
    let Ok(randr_event_count) = collect_randr_event_count(&xorg.connection) else {
        return OutputCollectorObservationV1 {
            requested_output,
            candidates,
            topology: None,
            collection_failure: Some(OutputCollectionFailureV1::X11Unavailable),
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
            randr_event_count: 0,
            randr_snapshot_sha256: opening_randr.digest,
            nvcontrol_snapshot_sha256: opening_nvcontrol.digest,
            nvml_snapshot_sha256: opening_nvml.digest,
        },
        token_after: OutputTopologyTokenV1 {
            randr_timestamp: closing_randr.timestamp,
            randr_config_timestamp: closing_randr.config_timestamp,
            randr_event_count,
            randr_snapshot_sha256: closing_randr.digest,
            nvcontrol_snapshot_sha256: closing_nvcontrol.digest,
            nvml_snapshot_sha256: closing_nvml.digest,
        },
        randr_outputs: opening_randr.outputs,
        randr_providers: opening_randr.providers,
        nvcontrol: opening_nvcontrol.observation,
        drm_diagnostic_before: opening_drm,
        drm_diagnostic_after: closing_drm,
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

fn subscribe_randr_topology_events(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
    root: u32,
) -> Result<(), ()> {
    let mask = NotifyMask::SCREEN_CHANGE
        | NotifyMask::CRTC_CHANGE
        | NotifyMask::OUTPUT_CHANGE
        | NotifyMask::OUTPUT_PROPERTY
        | NotifyMask::PROVIDER_CHANGE
        | NotifyMask::PROVIDER_PROPERTY
        | NotifyMask::RESOURCE_CHANGE
        | NotifyMask::LEASE;
    connection
        .randr_select_input(root, mask)
        .map_err(|_| ())?
        .check()
        .map_err(|_| ())?;
    connection
        .get_input_focus()
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    while connection.poll_for_event().map_err(|_| ())?.is_some() {}
    Ok(())
}

fn collect_randr_event_count(
    connection: &x11rb::rust_connection::RustConnection<x11rb::rust_connection::DefaultStream>,
) -> Result<u32, ()> {
    connection
        .get_input_focus()
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?;
    let mut relevant = 0_u32;
    let mut total = 0_u32;
    while let Some(event) = connection.poll_for_event().map_err(|_| ())? {
        total = total.saturating_add(1);
        if total > MAX_RANDR_EVENTS.saturating_mul(4) {
            return Err(());
        }
        if matches!(
            event,
            Event::RandrNotify(_) | Event::RandrScreenChangeNotify(_)
        ) {
            relevant = relevant.saturating_add(1).min(MAX_RANDR_EVENTS);
        }
    }
    Ok(relevant)
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
    let primary_output = connection
        .randr_get_output_primary(root)
        .map_err(|_| ())?
        .reply()
        .map_err(|_| ())?
        .output;
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
        if crtc.status != SetConfig::SUCCESS
            || crtc.mode == 0
            || crtc.outputs.len() > MAX_OUTPUT_MAPPING_ITEMS_V1
        {
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
        let mut clone_output_xids = info
            .clones
            .iter()
            .copied()
            .map(crate::model::XrandrOutputXidV1::new)
            .collect::<Vec<_>>();
        clone_output_xids.sort_by_key(|xid| xid.get());
        let mut crtc_output_xids = crtc
            .outputs
            .iter()
            .copied()
            .map(crate::model::XrandrOutputXidV1::new)
            .collect::<Vec<_>>();
        crtc_output_xids.sort_by_key(|xid| xid.get());
        outputs.push(RandrOutputObservationV1 {
            output_xid: crate::model::XrandrOutputXidV1::new(output_xid),
            crtc_xid: crate::model::XrandrCrtcXidV1::new(info.crtc),
            mode_xid: crate::model::XrandrModeXidV1::new(crtc.mode),
            name,
            connected: true,
            physical: true,
            non_desktop,
            primary: output_xid == primary_output,
            clone_output_xids,
            crtc_output_xids,
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
        let mut crtc_xids = info
            .crtcs
            .into_iter()
            .map(crate::model::XrandrCrtcXidV1::new)
            .collect::<Vec<_>>();
        crtc_xids.sort_by_key(|xid| xid.get());
        let mut output_xids = info
            .outputs
            .into_iter()
            .map(crate::model::XrandrOutputXidV1::new)
            .collect::<Vec<_>>();
        output_xids.sort_by_key(|xid| xid.get());
        let mut associated_provider_xids = info
            .associated_providers
            .into_iter()
            .map(crate::model::XrandrProviderXidV1::new)
            .collect::<Vec<_>>();
        associated_provider_xids.sort_by_key(|xid| xid.get());
        providers.push(crate::model::RandrProviderObservationV1 {
            provider_xid: crate::model::XrandrProviderXidV1::new(provider_xid),
            name: output_name_from_bytes(&info.name).ok_or(())?,
            capabilities: u32::from(info.capabilities),
            crtc_xids,
            output_xids,
            associated_provider_xids,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NvControlTargetListErrorV1 {
    Malformed,
    TooMany,
    Duplicate,
    TargetOutOfRange,
}

pub fn decode_nvcontrol_target_list(bytes: &[u8]) -> Result<Vec<u32>, NvControlTargetListErrorV1> {
    if bytes.len() < size_of::<c_int>() || !bytes.len().is_multiple_of(size_of::<c_int>()) {
        return Err(NvControlTargetListErrorV1::Malformed);
    }
    let count = c_int::from_ne_bytes(
        bytes[..size_of::<c_int>()]
            .try_into()
            .map_err(|_| NvControlTargetListErrorV1::Malformed)?,
    );
    let count = usize::try_from(count).map_err(|_| NvControlTargetListErrorV1::TargetOutOfRange)?;
    if count > MAX_OUTPUT_MAPPING_ITEMS_V1 {
        return Err(NvControlTargetListErrorV1::TooMany);
    }
    let expected_length = count
        .checked_add(1)
        .and_then(|items| items.checked_mul(size_of::<c_int>()))
        .ok_or(NvControlTargetListErrorV1::Malformed)?;
    if bytes.len() != expected_length {
        return Err(NvControlTargetListErrorV1::Malformed);
    }
    let mut identifiers = HashSet::with_capacity(count);
    let mut targets = Vec::with_capacity(count);
    for chunk in bytes[size_of::<c_int>()..].chunks_exact(size_of::<c_int>()) {
        let target = c_int::from_ne_bytes(
            chunk
                .try_into()
                .map_err(|_| NvControlTargetListErrorV1::Malformed)?,
        );
        let target =
            u32::try_from(target).map_err(|_| NvControlTargetListErrorV1::TargetOutOfRange)?;
        if !identifiers.insert(target) {
            return Err(NvControlTargetListErrorV1::Duplicate);
        }
        targets.push(target);
    }
    Ok(targets)
}

type XOpenDisplayFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type XCloseDisplayFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type XFreeFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type XNvCtrlQueryExtensionFn = unsafe extern "C" fn(*mut c_void, *mut c_int, *mut c_int) -> c_int;
type XNvCtrlQueryVersionFn = unsafe extern "C" fn(*mut c_void, *mut c_int, *mut c_int) -> c_int;
type XNvCtrlIsNvScreenFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type XNvCtrlQueryTargetCountFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> c_int;
type XNvCtrlQueryTargetAttributeFn =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, *mut c_int) -> c_int;
type XNvCtrlQueryTargetStringAttributeFn =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, *mut *mut c_char) -> c_int;
type XNvCtrlQueryTargetBinaryDataFn = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    c_int,
    c_uint,
    c_uint,
    *mut *mut u8,
    *mut c_int,
) -> c_int;

struct X11Api {
    _library: Library,
    open_display: XOpenDisplayFn,
    close_display: XCloseDisplayFn,
    free: XFreeFn,
}

impl X11Api {
    fn load() -> Result<Self, ()> {
        // SAFETY: This is a fixed system SONAME; no caller-controlled path reaches dlopen.
        let library = unsafe { Library::new(X11_LIBRARY_NAME) }.map_err(|_| ())?;
        // SAFETY: Symbol names and signatures are copied from the Xlib ABI, and
        // the owning library handle remains live for the lifetime of the pointers.
        unsafe {
            Ok(Self {
                open_display: *library.get(b"XOpenDisplay\0").map_err(|_| ())?,
                close_display: *library.get(b"XCloseDisplay\0").map_err(|_| ())?,
                free: *library.get(b"XFree\0").map_err(|_| ())?,
                _library: library,
            })
        }
    }
}

struct XNvCtrlApi {
    _library: Library,
    query_extension: XNvCtrlQueryExtensionFn,
    query_version: XNvCtrlQueryVersionFn,
    is_nv_screen: XNvCtrlIsNvScreenFn,
    query_target_count: XNvCtrlQueryTargetCountFn,
    query_target_attribute: XNvCtrlQueryTargetAttributeFn,
    query_target_string_attribute: XNvCtrlQueryTargetStringAttributeFn,
    query_target_binary_data: XNvCtrlQueryTargetBinaryDataFn,
}

impl XNvCtrlApi {
    fn load() -> Result<Self, ()> {
        // SAFETY: This is a fixed system SONAME; no caller-controlled path reaches dlopen.
        let library = unsafe { Library::new(XNVCTRL_LIBRARY_NAME) }.map_err(|_| ())?;
        // SAFETY: Symbol names and signatures are copied from NVCtrlLib.h, and
        // the owning library handle remains live for the lifetime of the pointers.
        unsafe {
            Ok(Self {
                query_extension: *library.get(b"XNVCTRLQueryExtension\0").map_err(|_| ())?,
                query_version: *library.get(b"XNVCTRLQueryVersion\0").map_err(|_| ())?,
                is_nv_screen: *library.get(b"XNVCTRLIsNvScreen\0").map_err(|_| ())?,
                query_target_count: *library.get(b"XNVCTRLQueryTargetCount\0").map_err(|_| ())?,
                query_target_attribute: *library
                    .get(b"XNVCTRLQueryTargetAttribute\0")
                    .map_err(|_| ())?,
                query_target_string_attribute: *library
                    .get(b"XNVCTRLQueryTargetStringAttribute\0")
                    .map_err(|_| ())?,
                query_target_binary_data: *library
                    .get(b"XNVCTRLQueryTargetBinaryData\0")
                    .map_err(|_| ())?,
                _library: library,
            })
        }
    }
}

#[derive(Debug)]
struct NvControlLiveSnapshotV1 {
    observation: NvControlSnapshotV1,
    digest: crate::Sha256DigestV1,
}

fn collect_nvcontrol_snapshot(
    display_number: u16,
    screen: u16,
) -> Result<NvControlLiveSnapshotV1, ()> {
    let x11 = X11Api::load()?;
    let nvcontrol = XNvCtrlApi::load()?;
    let x11_library = loaded_library_identity("libX11.so.")?;
    let xnvctrl_library = loaded_library_identity("libXNVCtrl.so.")?;
    let display_name = CString::new(format!(":{display_number}.{screen}")).map_err(|_| ())?;
    // SAFETY: `display_name` is a live, NUL-terminated string and Xlib owns the result.
    let display = unsafe { (x11.open_display)(display_name.as_ptr()) };
    if display.is_null() {
        return Err(());
    }
    let result = collect_nvcontrol_from_display(
        &x11,
        &nvcontrol,
        display,
        screen,
        x11_library,
        xnvctrl_library,
    );
    // SAFETY: Exactly pairs the successful XOpenDisplay call above.
    let close_result = unsafe { (x11.close_display)(display) };
    if close_result != 0 {
        return Err(());
    }
    result
}

fn collect_nvcontrol_from_display(
    x11: &X11Api,
    api: &XNvCtrlApi,
    display: *mut c_void,
    screen: u16,
    x11_library: String,
    xnvctrl_library: String,
) -> Result<NvControlLiveSnapshotV1, ()> {
    let screen_i32 = c_int::from(screen);
    let mut event_base = 0;
    let mut error_base = 0;
    // SAFETY: `display` is a live Xlib Display and all output pointers are valid.
    let extension_present =
        unsafe { (api.query_extension)(display, &mut event_base, &mut error_base) } != 0;
    let mut major = 0;
    let mut minor = 0;
    let extension_version = if extension_present {
        // SAFETY: Same live Display and valid output pointers as above.
        let succeeded = unsafe { (api.query_version)(display, &mut major, &mut minor) } != 0;
        if !succeeded || major < 0 || minor < 0 {
            None
        } else {
            Some(NvControlExtensionVersionV1 {
                major: u32::try_from(major).map_err(|_| ())?,
                minor: u32::try_from(minor).map_err(|_| ())?,
            })
        }
    } else {
        None
    };
    // SAFETY: The authenticated screen index came from the same X server setup.
    let x_screen_is_nvidia =
        extension_present && unsafe { (api.is_nv_screen)(display, screen_i32) } != 0;

    let mut snapshot = NvControlSnapshotV1 {
        x11_library,
        xnvctrl_library,
        extension_present,
        extension_version,
        x_screen: u32::from(screen),
        x_screen_is_nvidia,
        display_targets: Vec::new(),
        enabled_display_target_ids: Vec::new(),
        gpu_targets: Vec::new(),
    };
    if !extension_present
        || !x_screen_is_nvidia
        || extension_version
            .is_none_or(|version| (version.major, version.minor) < MIN_NVCONTROL_VERSION)
    {
        let digest = sha256_bytes(&serde_json::to_vec(&snapshot).map_err(|_| ())?);
        return Ok(NvControlLiveSnapshotV1 {
            observation: snapshot,
            digest,
        });
    }

    let display_target_ids = query_nvcontrol_target_list(
        x11,
        api,
        display,
        NV_CTRL_TARGET_TYPE_X_SCREEN,
        screen_i32,
        NV_CTRL_BINARY_DATA_DISPLAY_TARGETS,
    )?;
    let enabled_display_target_ids = query_nvcontrol_target_list(
        x11,
        api,
        display,
        NV_CTRL_TARGET_TYPE_X_SCREEN,
        screen_i32,
        NV_CTRL_BINARY_DATA_DISPLAYS_ENABLED_ON_XSCREEN,
    )?;
    snapshot.enabled_display_target_ids = enabled_display_target_ids
        .into_iter()
        .map(NvControlDisplayTargetIdV1::new)
        .collect();
    snapshot
        .enabled_display_target_ids
        .sort_by_key(|target| target.get());

    for target_id in display_target_ids {
        let target_i32 = c_int::try_from(target_id).map_err(|_| ())?;
        let randr_output_xid = query_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_DISPLAY,
            target_i32,
            NV_CTRL_DISPLAY_RANDR_OUTPUT_ID,
        )?
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
        let randr_name = query_nvcontrol_string_bytes(
            x11,
            api,
            display,
            NV_CTRL_TARGET_TYPE_DISPLAY,
            target_i32,
            NV_CTRL_STRING_DISPLAY_NAME_RANDR,
        )?
        .as_deref()
        .and_then(output_name_from_bytes);
        let enabled = query_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_DISPLAY,
            target_i32,
            NV_CTRL_DISPLAY_ENABLED,
        )?
        .and_then(|value| match value {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        });
        let displayport_is_multistream = query_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_DISPLAY,
            target_i32,
            NV_CTRL_DISPLAYPORT_IS_MULTISTREAM,
        )?
        .and_then(|value| match value {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        });
        snapshot
            .display_targets
            .push(NvControlDisplayTargetObservationV1 {
                target_id: NvControlDisplayTargetIdV1::new(target_id),
                randr_output_xid: crate::model::XrandrOutputXidV1::new(randr_output_xid),
                randr_name,
                enabled,
                target_index_name: query_nvcontrol_string(
                    x11,
                    api,
                    display,
                    NV_CTRL_TARGET_TYPE_DISPLAY,
                    target_i32,
                    NV_CTRL_STRING_DISPLAY_NAME_TARGET_INDEX,
                )?,
                type_id_name: query_nvcontrol_string(
                    x11,
                    api,
                    display,
                    NV_CTRL_TARGET_TYPE_DISPLAY,
                    target_i32,
                    NV_CTRL_STRING_DISPLAY_NAME_TYPE_ID,
                )?,
                dp_guid: query_nvcontrol_string(
                    x11,
                    api,
                    display,
                    NV_CTRL_TARGET_TYPE_DISPLAY,
                    target_i32,
                    NV_CTRL_STRING_DISPLAY_NAME_DP_GUID,
                )?,
                edid_hash: query_nvcontrol_string(
                    x11,
                    api,
                    display,
                    NV_CTRL_TARGET_TYPE_DISPLAY,
                    target_i32,
                    NV_CTRL_STRING_DISPLAY_NAME_EDID_HASH,
                )?,
                displayport_is_multistream,
            });
    }
    snapshot
        .display_targets
        .sort_by_key(|target| target.target_id.get());

    let mut gpu_count = 0;
    // SAFETY: GPU is the only target type queried by count; the live Display
    // and output pointer satisfy XNVCTRLQueryTargetCount's ABI.
    if unsafe { (api.query_target_count)(display, NV_CTRL_TARGET_TYPE_GPU, &mut gpu_count) } == 0 {
        return Err(());
    }
    let gpu_count = usize::try_from(gpu_count).map_err(|_| ())?;
    if gpu_count > MAX_OUTPUT_MAPPING_ITEMS_V1 {
        return Err(());
    }
    for gpu_id in 0..gpu_count {
        let gpu_i32 = c_int::try_from(gpu_id).map_err(|_| ())?;
        let connected_display_target_ids = query_nvcontrol_target_list(
            x11,
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_BINARY_DATA_DISPLAYS_CONNECTED_TO_GPU,
        )?;
        let pci_domain = required_nonnegative_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_PCI_DOMAIN,
        )?;
        let pci_bus = required_nonnegative_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_PCI_BUS,
        )?;
        let pci_device = required_nonnegative_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_PCI_DEVICE,
        )?;
        let pci_function = required_nonnegative_nvcontrol_int(
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_PCI_FUNCTION,
        )?;
        let canonical_pci_bdf =
            canonical_nvcontrol_pci_bdf(pci_domain, pci_bus, pci_device, pci_function).ok_or(())?;
        let uuid = query_nvcontrol_string(
            x11,
            api,
            display,
            NV_CTRL_TARGET_TYPE_GPU,
            gpu_i32,
            NV_CTRL_STRING_GPU_UUID,
        )?
        .filter(|value| valid_nvml_uuid(value))
        .ok_or(())?;
        let mut connected_display_target_ids = connected_display_target_ids
            .into_iter()
            .map(NvControlDisplayTargetIdV1::new)
            .collect::<Vec<_>>();
        connected_display_target_ids.sort_by_key(|target| target.get());
        snapshot.gpu_targets.push(NvControlGpuTargetObservationV1 {
            target_id: NvControlGpuTargetIdV1::new(u32::try_from(gpu_id).map_err(|_| ())?),
            connected_display_target_ids,
            pci_domain,
            pci_bus,
            pci_device,
            pci_function,
            canonical_pci_bdf,
            uuid,
        });
    }
    snapshot.gpu_targets.sort_by_key(|gpu| gpu.target_id.get());
    let digest = sha256_bytes(&serde_json::to_vec(&snapshot).map_err(|_| ())?);
    Ok(NvControlLiveSnapshotV1 {
        observation: snapshot,
        digest,
    })
}

fn query_nvcontrol_int(
    api: &XNvCtrlApi,
    display: *mut c_void,
    target_type: c_int,
    target_id: c_int,
    attribute: c_uint,
) -> Result<Option<c_int>, ()> {
    let mut value = 0;
    // SAFETY: The Display is live, target IDs are bounded signed values, the
    // display mask is unused for target attributes, and `value` is writable.
    let succeeded = unsafe {
        (api.query_target_attribute)(display, target_type, target_id, 0, attribute, &mut value)
    } != 0;
    Ok(succeeded.then_some(value))
}

fn required_nonnegative_nvcontrol_int(
    api: &XNvCtrlApi,
    display: *mut c_void,
    target_type: c_int,
    target_id: c_int,
    attribute: c_uint,
) -> Result<u32, ()> {
    query_nvcontrol_int(api, display, target_type, target_id, attribute)?
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(())
}

fn query_nvcontrol_string(
    x11: &X11Api,
    api: &XNvCtrlApi,
    display: *mut c_void,
    target_type: c_int,
    target_id: c_int,
    attribute: c_uint,
) -> Result<Option<String>, ()> {
    query_nvcontrol_string_bytes(x11, api, display, target_type, target_id, attribute)?
        .map(|bytes| {
            if !bytes.iter().all(|byte| byte.is_ascii_graphic()) {
                return Err(());
            }
            String::from_utf8(bytes).map_err(|_| ())
        })
        .transpose()
}

fn query_nvcontrol_string_bytes(
    x11: &X11Api,
    api: &XNvCtrlApi,
    display: *mut c_void,
    target_type: c_int,
    target_id: c_int,
    attribute: c_uint,
) -> Result<Option<Vec<u8>>, ()> {
    let mut pointer = std::ptr::null_mut();
    // SAFETY: The Display is live and `pointer` is writable. Successful calls
    // return Xmalloc-owned storage released with XFree below.
    let succeeded = unsafe {
        (api.query_target_string_attribute)(
            display,
            target_type,
            target_id,
            0,
            attribute,
            &mut pointer,
        )
    } != 0;
    if !succeeded {
        if !pointer.is_null() {
            // SAFETY: Defensive release of any Xmalloc-owned pointer returned on failure.
            unsafe { (x11.free)(pointer.cast()) };
        }
        return Ok(None);
    }
    if pointer.is_null() {
        return Err(());
    }
    // SAFETY: XNVCTRL promises a NUL-terminated Xmalloc-owned string on success.
    let bytes = unsafe { CStr::from_ptr(pointer) }.to_bytes();
    let value = if bytes.is_empty() || bytes.len() > MAX_NVCONTROL_STRING_BYTES_V1 {
        Err(())
    } else {
        Ok(bytes.to_vec())
    };
    // SAFETY: Exactly releases the Xmalloc-owned string returned above.
    unsafe { (x11.free)(pointer.cast()) };
    value.map(Some)
}

fn query_nvcontrol_target_list(
    x11: &X11Api,
    api: &XNvCtrlApi,
    display: *mut c_void,
    target_type: c_int,
    target_id: c_int,
    attribute: c_uint,
) -> Result<Vec<u32>, ()> {
    let mut pointer = std::ptr::null_mut();
    let mut length = 0;
    // SAFETY: The Display is live and both output pointers are writable.
    // Successful calls return Xmalloc-owned bytes released with XFree below.
    let succeeded = unsafe {
        (api.query_target_binary_data)(
            display,
            target_type,
            target_id,
            0,
            attribute,
            &mut pointer,
            &mut length,
        )
    } != 0;
    if !succeeded || pointer.is_null() {
        if !pointer.is_null() {
            // SAFETY: Defensive release of any Xmalloc-owned pointer returned on failure.
            unsafe { (x11.free)(pointer.cast()) };
        }
        return Err(());
    }
    let maximum = (MAX_OUTPUT_MAPPING_ITEMS_V1 + 1) * size_of::<c_int>();
    let decoded = match usize::try_from(length) {
        Ok(length) if length >= size_of::<c_int>() && length <= maximum => {
            // SAFETY: XNVCTRL reports exactly `length` readable bytes for this allocation.
            let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
            decode_nvcontrol_target_list(bytes).map_err(|_| ())
        }
        _ => Err(()),
    };
    // SAFETY: Exactly releases the Xmalloc-owned binary result returned above.
    unsafe { (x11.free)(pointer.cast()) };
    decoded
}

fn loaded_library_identity(prefix: &str) -> Result<String, ()> {
    let maps = std::fs::read_to_string("/proc/self/maps").map_err(|_| ())?;
    let identities = maps
        .lines()
        .filter_map(|line| line.split_ascii_whitespace().last())
        .filter_map(|path| Path::new(path).file_name()?.to_str())
        .filter(|name| name.starts_with(prefix) && valid_visible_ascii(name, 96))
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    (identities.len() == 1)
        .then(|| identities.into_iter().next())
        .flatten()
        .ok_or(())
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

fn drm_diagnostic_snapshot(snapshot: DrmSnapshotV1) -> DrmDiagnosticSnapshotV1 {
    DrmDiagnosticSnapshotV1 {
        snapshot_sha256: snapshot.digest,
        connector_count: u32::try_from(snapshot.connectors.len()).unwrap_or(u32::MAX),
        active_connector_count: u32::try_from(
            snapshot
                .connectors
                .iter()
                .filter(|connector| connector.connected && connector.enabled)
                .count(),
        )
        .unwrap_or(u32::MAX),
    }
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
        || !bounded(&observation.nvml_devices)
        || !valid_output_name(&observation.requested_output_name)
        || observation.token_before.randr_event_count > MAX_RANDR_EVENTS
        || observation.token_after.randr_event_count > MAX_RANDR_EVENTS
        || !valid_nvcontrol_snapshot(&observation.nvcontrol)
        || !valid_drm_diagnostic(observation.drm_diagnostic_before.as_ref())
        || !valid_drm_diagnostic(observation.drm_diagnostic_after.as_ref())
    {
        return Err(invalid_observation());
    }

    let mut output_ids = HashSet::with_capacity(observation.randr_outputs.len());
    for output in &observation.randr_outputs {
        if output.output_xid.get() == 0
            || !output_ids.insert(output.output_xid)
            || !valid_output_name(&output.name)
            || !bounded(&output.clone_output_xids)
            || !bounded(&output.crtc_output_xids)
            || output
                .clone_output_xids
                .iter()
                .any(|candidate| candidate.get() == 0)
            || output
                .crtc_output_xids
                .iter()
                .any(|candidate| candidate.get() == 0)
            || !unique_xrandr_outputs(&output.clone_output_xids)
            || !unique_xrandr_outputs(&output.crtc_output_xids)
            || output.crtc_xid.get() == 0
            || output.mode_xid.get() == 0
            || !valid_timing(&output.timing)
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
            || !unique_xrandr_crtcs(&provider.crtc_xids)
            || !unique_xrandr_outputs(&provider.output_xids)
            || !unique_xrandr_providers(&provider.associated_provider_xids)
        {
            return Err(invalid_observation());
        }
    }

    let mut nvml_pairs = HashSet::with_capacity(observation.nvml_devices.len());
    let mut nvml_bdfs = HashSet::with_capacity(observation.nvml_devices.len());
    let mut nvml_uuids = HashSet::with_capacity(observation.nvml_devices.len());
    for device in &observation.nvml_devices {
        if !valid_canonical_pci_bdf(&device.nvml_pci_bdf)
            || !valid_nvml_uuid(&device.nvml_uuid)
            || !nvml_pairs.insert((&device.nvml_pci_bdf, &device.nvml_uuid))
            || !nvml_bdfs.insert(&device.nvml_pci_bdf)
            || !nvml_uuids.insert(&device.nvml_uuid)
        {
            return Err(invalid_observation());
        }
    }

    Ok(())
}

fn valid_nvcontrol_snapshot(snapshot: &NvControlSnapshotV1) -> bool {
    if !valid_library_identity(&snapshot.x11_library, "libX11.so.")
        || !valid_library_identity(&snapshot.xnvctrl_library, "libXNVCtrl.so.")
        || snapshot.x_screen > u32::from(u16::MAX)
        || !bounded(&snapshot.display_targets)
        || !bounded(&snapshot.enabled_display_target_ids)
        || !bounded(&snapshot.gpu_targets)
    {
        return false;
    }

    let mut display_ids = HashSet::with_capacity(snapshot.display_targets.len());
    for target in &snapshot.display_targets {
        if !display_ids.insert(target.target_id)
            || target
                .randr_name
                .as_ref()
                .is_some_and(|name| !valid_output_name(name))
            || target
                .target_index_name
                .as_deref()
                .is_some_and(|value| !valid_nvcontrol_string(value))
            || target
                .type_id_name
                .as_deref()
                .is_some_and(|value| !valid_nvcontrol_string(value))
            || target
                .dp_guid
                .as_deref()
                .is_some_and(|value| !valid_nvcontrol_string(value))
            || target
                .edid_hash
                .as_deref()
                .is_some_and(|value| !valid_nvcontrol_string(value))
        {
            return false;
        }
    }
    let mut enabled_ids = HashSet::with_capacity(snapshot.enabled_display_target_ids.len());
    if snapshot
        .enabled_display_target_ids
        .iter()
        .any(|target| !enabled_ids.insert(*target))
    {
        return false;
    }

    let mut gpu_ids = HashSet::with_capacity(snapshot.gpu_targets.len());
    for gpu in &snapshot.gpu_targets {
        let mut connected_ids = HashSet::with_capacity(gpu.connected_display_target_ids.len());
        if !gpu_ids.insert(gpu.target_id)
            || !bounded(&gpu.connected_display_target_ids)
            || gpu
                .connected_display_target_ids
                .iter()
                .any(|target| !connected_ids.insert(*target))
            || canonical_nvcontrol_pci_bdf(
                gpu.pci_domain,
                gpu.pci_bus,
                gpu.pci_device,
                gpu.pci_function,
            )
            .as_deref()
                != Some(gpu.canonical_pci_bdf.as_str())
            || !valid_nvml_uuid(&gpu.uuid)
        {
            return false;
        }
    }
    true
}

fn valid_drm_diagnostic(diagnostic: Option<&DrmDiagnosticSnapshotV1>) -> bool {
    diagnostic.is_none_or(|diagnostic| {
        usize::try_from(diagnostic.connector_count).is_ok_and(|count| {
            count <= MAX_OUTPUT_MAPPING_ITEMS_V1
                && diagnostic.active_connector_count <= diagnostic.connector_count
        })
    })
}

fn unique_xrandr_outputs(items: &[crate::model::XrandrOutputXidV1]) -> bool {
    let mut identifiers = HashSet::with_capacity(items.len());
    items.iter().all(|item| identifiers.insert(*item))
}

fn unique_xrandr_crtcs(items: &[crate::model::XrandrCrtcXidV1]) -> bool {
    let mut identifiers = HashSet::with_capacity(items.len());
    items.iter().all(|item| identifiers.insert(*item))
}

fn unique_xrandr_providers(items: &[crate::model::XrandrProviderXidV1]) -> bool {
    let mut identifiers = HashSet::with_capacity(items.len());
    items.iter().all(|item| identifiers.insert(*item))
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
    if output.crtc_output_xids.len() != 1
        || output
            .crtc_output_xids
            .iter()
            .filter(|candidate| **candidate == output.output_xid)
            .count()
            != 1
    {
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
                && output.crtc_output_xids.len() == 1
                && output.crtc_output_xids[0] == output.output_xid
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

fn valid_library_identity(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() <= 96
        && value.strip_prefix(prefix).is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .split('.')
                    .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

fn valid_nvcontrol_string(value: &str) -> bool {
    valid_visible_ascii(value, MAX_NVCONTROL_STRING_BYTES_V1)
}

fn valid_nvml_uuid(value: &str) -> bool {
    value.starts_with("GPU-")
        && value.len() < MAX_NVML_UUID_BYTES_V1
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn canonical_nvcontrol_pci_bdf(
    domain: u32,
    bus: u32,
    device: u32,
    function: u32,
) -> Option<String> {
    (bus <= u32::from(u8::MAX) && device <= u32::from(u8::MAX) && function <= 7)
        .then(|| format!("{domain:08x}:{bus:02x}:{device:02x}.{function:x}"))
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
            collect_fixture_output_topology(&fixture, "nvcontrol-mst-dp-0-3", Some("DP-0.3"))
                .expect("stable fixture collection must succeed");
        let selected = stable
            .topology
            .as_ref()
            .map(prove_output_gpu_mapping)
            .expect("an explicit selection must carry a topology")
            .expect("the full unequal-ID relation must pass");
        assert_eq!(selected.output_name.display.as_deref(), Some("DP-0.3"));
        assert_ne!(
            selected.randr_output_xid.get(),
            selected.nvcontrol_display_target_id.get()
        );

        let raced = collect_fixture_output_topology(&fixture, "topology-changed", Some("DP-0.3"))
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
