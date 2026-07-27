use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{
    DrmConnectorObservationV1, ExactModeTimingV1, MAX_CONNECTOR_NAME_BYTES_V1,
    MAX_NVML_UUID_BYTES_V1, MAX_OUTPUT_MAPPING_ITEMS_V1, MAX_OUTPUT_NAME_BYTES_V1,
    OutputMappingProofCardinalitiesV1, OutputNameV1, OutputTopologyObservationV1,
    RandrOutputObservationV1, RefreshRateV1, SELECTED_OUTPUT_SCHEMA_V1, SelectedOutputV1,
};

const RANDR_PROVIDER_SOURCE_OUTPUT: u32 = 1;
const RANDR_PROVIDER_KNOWN_CAPABILITIES: u32 = 0x0f;
const RANDR_MODE_INTERLACE: u32 = 1 << 4;
const RANDR_MODE_DOUBLE_SCAN: u32 = 1 << 5;
const RANDR_MODE_EXACT_REFRESH_FLAGS: u32 = 0x07ff;

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

        let stable = collect_fixture_output_topology(
            &fixture,
            "namespace-disjoint-unique",
            Some("DP-0"),
        )
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

        let raced =
            collect_fixture_output_topology(&fixture, "topology-changed", Some("DP-0"))
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
