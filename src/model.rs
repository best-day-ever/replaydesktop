use crate::digest::Sha256DigestV1;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashSet;
use std::fmt;

pub const G0_ENVELOPE_SCHEMA_V1: &str = "replaydesktop.g0-evidence-envelope";
pub const G0_BASE_SCHEMA_V1: &str = "replaydesktop.g0-evidence-base.v1";
pub const G0_ENVELOPE_VERSION_V1: u32 = 1;
pub const MAX_G0_ENVELOPE_BYTES: usize = 1024 * 1024;
pub const MAX_G0_EXTENSIONS: usize = 16;
pub const MAX_G0_EXTENSION_PAYLOAD_BYTES: usize = 256 * 1024;
pub const MAX_G0_EXTENSION_IDENTIFIER_BYTES: usize = 64;
pub const MAX_G0_IDENTITY_BYTES: usize = 128;
pub const MAX_G0_ARGV_ITEMS: usize = 64;
pub const MAX_G0_ARG_BYTES: usize = 4096;

pub const HOST_FOUNDATION_EXTENSION_ID: &str = "host-foundation.v1";
pub const SELECTED_OUTPUT_EXTENSION_ID: &str = "selected-output.v1";
pub const NVFBC_CAPTURE_EXTENSION_ID: &str = "nvfbc-capture.v1";
pub const NVENC_TUPLES_EXTENSION_ID: &str = "nvenc-tuples.v1";

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum G0EvidenceProvenanceV1 {
    Live,
    Diagnostic,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum G0GateStatusV1 {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum G0ExtensionStatusV1 {
    Pass,
    Fail,
    Unproven,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum G0KnownExtensionV1 {
    #[serde(rename = "host-foundation.v1")]
    HostFoundation,
    #[serde(rename = "selected-output.v1")]
    SelectedOutput,
    #[serde(rename = "nvfbc-capture.v1")]
    NvfbcCapture,
    #[serde(rename = "nvenc-tuples.v1")]
    NvencTuples,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ReasonV1 {
    pub extension: G0KnownExtensionV1,
    pub status: G0ExtensionStatusV1,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0EvidenceBaseV1 {
    pub schema: String,
    pub provenance: G0EvidenceProvenanceV1,
    pub run_id: String,
    pub boot_id: String,
    pub session_id: String,
    pub argv: Vec<String>,
    pub executable_sha256: Sha256DigestV1,
    pub wall_started_unix_ns: u64,
    pub wall_finished_unix_ns: u64,
    pub monotonic_started_ns: u64,
    pub monotonic_finished_ns: u64,
    pub status: G0GateStatusV1,
    pub reasons: Vec<G0ReasonV1>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ExtensionRecordV1 {
    pub id: String,
    pub version: u32,
    pub status: G0ExtensionStatusV1,
    pub payload: Box<RawValue>,
    pub payload_sha256: Sha256DigestV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0EvidenceEnvelopeV1 {
    pub schema: String,
    pub version: u32,
    pub base: G0EvidenceBaseV1,
    pub extensions: Vec<G0ExtensionRecordV1>,
}

#[derive(Debug)]
pub enum DecodedG0Evidence {
    V1(G0EvidenceEnvelopeV1),
}

impl DecodedG0Evidence {
    pub fn as_v1(&self) -> &G0EvidenceEnvelopeV1 {
        match self {
            Self::V1(envelope) => envelope,
        }
    }

    pub fn into_v1(self) -> G0EvidenceEnvelopeV1 {
        match self {
            Self::V1(envelope) => envelope,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum G0DecodeError {
    InputTooLarge {
        actual: usize,
        maximum: usize,
    },
    InvalidJson(String),
    WrongSchema(String),
    UnsupportedVersion(u64),
    InvalidEnvelope(String),
    TooManyExtensions {
        actual: usize,
        maximum: usize,
    },
    PayloadTooLarge {
        id: String,
        actual: usize,
        maximum: usize,
    },
    PayloadBudgetExceeded {
        actual: usize,
        maximum: usize,
    },
    InvalidExtensionIdentifier(String),
    DuplicateExtensionIdentifier(String),
    InvalidPayloadJson {
        id: String,
        message: String,
    },
    PayloadDigestMismatch {
        id: String,
    },
}

impl fmt::Display for G0DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "G0 evidence is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidJson(message) => write!(formatter, "invalid G0 JSON: {message}"),
            Self::WrongSchema(schema) => write!(formatter, "unsupported G0 schema: {schema}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported G0 evidence version: {version}")
            }
            Self::InvalidEnvelope(message) => write!(formatter, "invalid V1 envelope: {message}"),
            Self::TooManyExtensions { actual, maximum } => {
                write!(
                    formatter,
                    "V1 envelope has {actual} extensions; maximum is {maximum}"
                )
            }
            Self::PayloadTooLarge {
                id,
                actual,
                maximum,
            } => {
                write!(
                    formatter,
                    "extension {id} payload is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::PayloadBudgetExceeded { actual, maximum } => {
                write!(
                    formatter,
                    "extension payloads total {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidExtensionIdentifier(id) => {
                write!(formatter, "invalid extension identifier: {id}")
            }
            Self::DuplicateExtensionIdentifier(id) => {
                write!(formatter, "duplicate extension identifier: {id}")
            }
            Self::InvalidPayloadJson { id, message } => {
                write!(
                    formatter,
                    "invalid payload JSON for extension {id}: {message}"
                )
            }
            Self::PayloadDigestMismatch { id } => {
                write!(formatter, "payload SHA-256 mismatch for extension {id}")
            }
        }
    }
}

impl std::error::Error for G0DecodeError {}

pub fn decode_g0_evidence(input: &[u8]) -> Result<DecodedG0Evidence, G0DecodeError> {
    if input.len() > MAX_G0_ENVELOPE_BYTES {
        return Err(G0DecodeError::InputTooLarge {
            actual: input.len(),
            maximum: MAX_G0_ENVELOPE_BYTES,
        });
    }

    let dispatch: DispatchHeader = serde_json::from_slice(input)
        .map_err(|error| G0DecodeError::InvalidJson(error.to_string()))?;
    if dispatch.schema != G0_ENVELOPE_SCHEMA_V1 {
        return Err(G0DecodeError::WrongSchema(dispatch.schema));
    }
    if dispatch.version != u64::from(G0_ENVELOPE_VERSION_V1) {
        return Err(G0DecodeError::UnsupportedVersion(dispatch.version));
    }

    let envelope = decode_v1(input)?;
    Ok(DecodedG0Evidence::V1(envelope))
}

#[derive(Deserialize)]
struct DispatchHeader {
    schema: String,
    version: u64,
}

fn decode_v1(input: &[u8]) -> Result<G0EvidenceEnvelopeV1, G0DecodeError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let envelope = G0EvidenceEnvelopeV1::deserialize(&mut deserializer)
        .map_err(|error| G0DecodeError::InvalidEnvelope(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| G0DecodeError::InvalidEnvelope(error.to_string()))?;

    if envelope.schema != G0_ENVELOPE_SCHEMA_V1 {
        return Err(G0DecodeError::WrongSchema(envelope.schema));
    }
    if envelope.version != G0_ENVELOPE_VERSION_V1 {
        return Err(G0DecodeError::UnsupportedVersion(u64::from(
            envelope.version,
        )));
    }

    validate_base(&envelope.base)?;
    validate_extensions(&envelope.extensions)?;
    validate_gate_summary(&envelope.base, &envelope.extensions)?;
    Ok(envelope)
}

fn validate_base(base: &G0EvidenceBaseV1) -> Result<(), G0DecodeError> {
    if base.schema != G0_BASE_SCHEMA_V1 {
        return Err(G0DecodeError::InvalidEnvelope(format!(
            "unsupported base schema: {}",
            base.schema
        )));
    }

    validate_identity("run_id", &base.run_id)?;
    validate_identity("boot_id", &base.boot_id)?;
    validate_identity("session_id", &base.session_id)?;

    if base.argv.is_empty() || base.argv.len() > MAX_G0_ARGV_ITEMS {
        return Err(G0DecodeError::InvalidEnvelope(format!(
            "argv must contain 1..={MAX_G0_ARGV_ITEMS} items"
        )));
    }
    for (index, argument) in base.argv.iter().enumerate() {
        if argument.len() > MAX_G0_ARG_BYTES || argument.as_bytes().contains(&0) {
            return Err(G0DecodeError::InvalidEnvelope(format!(
                "argv[{index}] exceeds its bound or contains NUL"
            )));
        }
    }

    if base.wall_started_unix_ns > base.wall_finished_unix_ns {
        return Err(G0DecodeError::InvalidEnvelope(
            "wall-clock finish precedes start".to_owned(),
        ));
    }
    if base.monotonic_started_ns > base.monotonic_finished_ns {
        return Err(G0DecodeError::InvalidEnvelope(
            "monotonic finish precedes start".to_owned(),
        ));
    }

    if matches!(base.provenance, G0EvidenceProvenanceV1::Diagnostic)
        && matches!(base.status, G0GateStatusV1::Pass)
    {
        return Err(G0DecodeError::InvalidEnvelope(
            "diagnostic evidence cannot carry PASS".to_owned(),
        ));
    }

    match base.status {
        G0GateStatusV1::Pass if !base.reasons.is_empty() => {
            return Err(G0DecodeError::InvalidEnvelope(
                "PASS evidence cannot carry failure reasons".to_owned(),
            ));
        }
        G0GateStatusV1::Fail if base.reasons.is_empty() => {
            return Err(G0DecodeError::InvalidEnvelope(
                "FAIL evidence must carry at least one reason".to_owned(),
            ));
        }
        _ => {}
    }

    if base.reasons.len() > 4 {
        return Err(G0DecodeError::InvalidEnvelope(
            "base reasons exceed the four stable gates".to_owned(),
        ));
    }

    let mut previous_rank = None;
    for reason in &base.reasons {
        if matches!(reason.status, G0ExtensionStatusV1::Pass) {
            return Err(G0DecodeError::InvalidEnvelope(
                "a PASS extension is not a gate failure reason".to_owned(),
            ));
        }
        let rank = reason.extension.rank();
        if previous_rank.is_some_and(|previous| previous >= rank) {
            return Err(G0DecodeError::InvalidEnvelope(
                "base reasons are duplicated or out of stable order".to_owned(),
            ));
        }
        previous_rank = Some(rank);
    }

    Ok(())
}

fn validate_identity(field: &str, value: &str) -> Result<(), G0DecodeError> {
    if value.is_empty()
        || value.len() > MAX_G0_IDENTITY_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(G0DecodeError::InvalidEnvelope(format!(
            "{field} must be 1..={MAX_G0_IDENTITY_BYTES} visible ASCII bytes"
        )));
    }
    Ok(())
}

fn validate_extensions(extensions: &[G0ExtensionRecordV1]) -> Result<(), G0DecodeError> {
    if extensions.len() > MAX_G0_EXTENSIONS {
        return Err(G0DecodeError::TooManyExtensions {
            actual: extensions.len(),
            maximum: MAX_G0_EXTENSIONS,
        });
    }

    let mut identifiers = HashSet::with_capacity(extensions.len());
    let mut payload_total = 0_usize;
    for extension in extensions {
        if !valid_extension_identifier(&extension.id) {
            return Err(G0DecodeError::InvalidExtensionIdentifier(
                extension.id.clone(),
            ));
        }
        if !identifiers.insert(extension.id.as_str()) {
            return Err(G0DecodeError::DuplicateExtensionIdentifier(
                extension.id.clone(),
            ));
        }
        if extension.version == 0 {
            return Err(G0DecodeError::InvalidEnvelope(format!(
                "extension {} has zero schema version",
                extension.id
            )));
        }

        let payload_bytes = extension.payload.get().as_bytes();
        if payload_bytes.len() > MAX_G0_EXTENSION_PAYLOAD_BYTES {
            return Err(G0DecodeError::PayloadTooLarge {
                id: extension.id.clone(),
                actual: payload_bytes.len(),
                maximum: MAX_G0_EXTENSION_PAYLOAD_BYTES,
            });
        }
        payload_total = payload_total.checked_add(payload_bytes.len()).ok_or(
            G0DecodeError::PayloadBudgetExceeded {
                actual: usize::MAX,
                maximum: MAX_G0_ENVELOPE_BYTES,
            },
        )?;
        if payload_total > MAX_G0_ENVELOPE_BYTES {
            return Err(G0DecodeError::PayloadBudgetExceeded {
                actual: payload_total,
                maximum: MAX_G0_ENVELOPE_BYTES,
            });
        }

        validate_payload_json(&extension.id, extension.payload.as_ref())?;
        if crate::digest::sha256_bytes(payload_bytes) != extension.payload_sha256 {
            return Err(G0DecodeError::PayloadDigestMismatch {
                id: extension.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_gate_summary(
    base: &G0EvidenceBaseV1,
    extensions: &[G0ExtensionRecordV1],
) -> Result<(), G0DecodeError> {
    let mut expected_reasons = Vec::with_capacity(G0KnownExtensionV1::ALL.len());

    for known in G0KnownExtensionV1::ALL {
        let extension = extensions
            .iter()
            .find(|extension| extension.id == known.identifier())
            .ok_or_else(|| {
                G0DecodeError::InvalidEnvelope(format!(
                    "missing required extension record: {}",
                    known.identifier()
                ))
            })?;

        if !matches!(extension.status, G0ExtensionStatusV1::Pass) {
            expected_reasons.push(G0ReasonV1 {
                extension: known,
                status: extension.status,
            });
        }
    }

    if base.reasons != expected_reasons {
        return Err(G0DecodeError::InvalidEnvelope(
            "base reasons do not exactly match the ordered non-PASS known extensions".to_owned(),
        ));
    }

    let expected_status = if expected_reasons.is_empty() {
        G0GateStatusV1::Pass
    } else {
        G0GateStatusV1::Fail
    };
    if base.status != expected_status {
        return Err(G0DecodeError::InvalidEnvelope(
            "base status does not match the known extension terminal states".to_owned(),
        ));
    }

    Ok(())
}

fn valid_extension_identifier(identifier: &str) -> bool {
    let bytes = identifier.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_G0_EXTENSION_IDENTIFIER_BYTES {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    if !bytes[bytes.len() - 1].is_ascii_lowercase() && !bytes[bytes.len() - 1].is_ascii_digit() {
        return false;
    }
    bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
    })
}

impl G0KnownExtensionV1 {
    const ALL: [Self; 4] = [
        Self::HostFoundation,
        Self::SelectedOutput,
        Self::NvfbcCapture,
        Self::NvencTuples,
    ];

    const fn identifier(self) -> &'static str {
        match self {
            Self::HostFoundation => HOST_FOUNDATION_EXTENSION_ID,
            Self::SelectedOutput => SELECTED_OUTPUT_EXTENSION_ID,
            Self::NvfbcCapture => NVFBC_CAPTURE_EXTENSION_ID,
            Self::NvencTuples => NVENC_TUPLES_EXTENSION_ID,
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::HostFoundation => 0,
            Self::SelectedOutput => 1,
            Self::NvfbcCapture => 2,
            Self::NvencTuples => 3,
        }
    }
}

fn validate_payload_json(id: &str, payload: &RawValue) -> Result<(), G0DecodeError> {
    let mut deserializer = serde_json::Deserializer::from_str(payload.get());
    DuplicateFreeSeed
        .deserialize(&mut deserializer)
        .and_then(|()| deserializer.end())
        .map_err(|error| G0DecodeError::InvalidPayloadJson {
            id: id.to_owned(),
            message: error.to_string(),
        })
}

struct DuplicateFreeSeed;

impl<'de> DeserializeSeed<'de> for DuplicateFreeSeed {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateFreeVisitor)
    }
}

struct DuplicateFreeVisitor;

impl<'de> Visitor<'de> for DuplicateFreeVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DuplicateFreeSeed.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(DuplicateFreeSeed)?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate object key: {key}")));
            }
            map.next_value_seed(DuplicateFreeSeed)?;
        }
        Ok(())
    }
}

pub const SELECTED_OUTPUT_SCHEMA_V1: &str = "replaydesktop.selected-output.v1";
pub const MAX_OUTPUT_MAPPING_ITEMS_V1: usize = 64;
pub const MAX_OUTPUT_NAME_BYTES_V1: usize = 256;
pub const MAX_CONNECTOR_NAME_BYTES_V1: usize = 64;
pub const MAX_NVCONTROL_STRING_BYTES_V1: usize = 256;
pub const MAX_NVML_UUID_BYTES_V1: usize = 96;

macro_rules! typed_xid {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

typed_xid!(XrandrOutputXidV1);
typed_xid!(XrandrCrtcXidV1);
typed_xid!(XrandrModeXidV1);
typed_xid!(XrandrProviderXidV1);
typed_xid!(DrmConnectorIdV1);
typed_xid!(NvControlDisplayTargetIdV1);
typed_xid!(NvControlGpuTargetIdV1);

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputNameV1 {
    pub hex: String,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhysicalConnectorKindV1 {
    DisplayPort,
    HdmiA,
    HdmiB,
    DviD,
    DviI,
    DviA,
    Edp,
    Lvds,
    Vga,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactModeTimingV1 {
    pub pixel_clock_hz: u64,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRateV1 {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputTopologyTokenV1 {
    pub randr_timestamp: u32,
    pub randr_config_timestamp: u32,
    pub randr_event_count: u32,
    pub randr_snapshot_sha256: Sha256DigestV1,
    pub nvcontrol_snapshot_sha256: Sha256DigestV1,
    pub nvml_snapshot_sha256: Sha256DigestV1,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RandrOutputObservationV1 {
    pub output_xid: XrandrOutputXidV1,
    pub crtc_xid: XrandrCrtcXidV1,
    pub mode_xid: XrandrModeXidV1,
    pub name: OutputNameV1,
    pub connected: bool,
    pub physical: bool,
    pub non_desktop: bool,
    pub primary: bool,
    pub clone_output_xids: Vec<XrandrOutputXidV1>,
    pub crtc_output_xids: Vec<XrandrOutputXidV1>,
    pub connector_kind: PhysicalConnectorKindV1,
    pub connector_number: Option<u32>,
    pub width_px: u16,
    pub height_px: u16,
    pub origin_x: i16,
    pub origin_y: i16,
    pub edid_sha256: Option<Sha256DigestV1>,
    pub timing: ExactModeTimingV1,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RandrProviderObservationV1 {
    pub provider_xid: XrandrProviderXidV1,
    pub name: OutputNameV1,
    pub capabilities: u32,
    pub crtc_xids: Vec<XrandrCrtcXidV1>,
    pub output_xids: Vec<XrandrOutputXidV1>,
    pub associated_provider_xids: Vec<XrandrProviderXidV1>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrmConnectorObservationV1 {
    pub connector_id: DrmConnectorIdV1,
    pub connector_kind: PhysicalConnectorKindV1,
    pub connector_type_id: u32,
    pub connector_name: String,
    pub connected: bool,
    pub enabled: bool,
    pub physical: bool,
    pub mst: bool,
    pub leased: bool,
    pub edid_sha256: Option<Sha256DigestV1>,
    pub canonical_pci_bdfs: Vec<String>,
    pub active_timing: ExactModeTimingV1,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlDeviceIdentityObservationV1 {
    pub nvml_pci_bdf: String,
    pub nvml_uuid: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvControlExtensionVersionV1 {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvControlDisplayTargetObservationV1 {
    pub target_id: NvControlDisplayTargetIdV1,
    pub randr_output_xid: XrandrOutputXidV1,
    pub randr_name: Option<OutputNameV1>,
    pub enabled: Option<bool>,
    pub target_index_name: Option<String>,
    pub type_id_name: Option<String>,
    pub dp_guid: Option<String>,
    pub edid_hash: Option<String>,
    pub displayport_is_multistream: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvControlGpuTargetObservationV1 {
    pub target_id: NvControlGpuTargetIdV1,
    pub connected_display_target_ids: Vec<NvControlDisplayTargetIdV1>,
    pub pci_domain: u32,
    pub pci_bus: u32,
    pub pci_device: u32,
    pub pci_function: u32,
    pub canonical_pci_bdf: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvControlSnapshotV1 {
    pub x11_library: String,
    pub xnvctrl_library: String,
    pub extension_present: bool,
    pub extension_version: Option<NvControlExtensionVersionV1>,
    pub x_screen: u32,
    pub x_screen_is_nvidia: bool,
    pub display_targets: Vec<NvControlDisplayTargetObservationV1>,
    pub enabled_display_target_ids: Vec<NvControlDisplayTargetIdV1>,
    pub gpu_targets: Vec<NvControlGpuTargetObservationV1>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrmDiagnosticSnapshotV1 {
    pub snapshot_sha256: Sha256DigestV1,
    pub connector_count: u32,
    pub active_connector_count: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputTopologyObservationV1 {
    pub requested_output_name: OutputNameV1,
    pub token_before: OutputTopologyTokenV1,
    pub token_after: OutputTopologyTokenV1,
    pub randr_outputs: Vec<RandrOutputObservationV1>,
    pub randr_providers: Vec<RandrProviderObservationV1>,
    pub nvcontrol: NvControlSnapshotV1,
    pub drm_diagnostic_before: Option<DrmDiagnosticSnapshotV1>,
    pub drm_diagnostic_after: Option<DrmDiagnosticSnapshotV1>,
    pub nvml_devices: Vec<NvmlDeviceIdentityObservationV1>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputMappingProofCardinalitiesV1 {
    pub requested_output_matches: u32,
    pub provider_matches: u32,
    pub nvcontrol_display_target_matches: u32,
    pub enabled_on_xscreen_matches: u32,
    pub nvcontrol_gpu_owner_matches: u32,
    pub nvml_device_matches: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedOutputV1 {
    pub schema: String,
    pub output_name: OutputNameV1,
    pub randr_output_xid: XrandrOutputXidV1,
    pub randr_crtc_xid: XrandrCrtcXidV1,
    pub randr_mode_xid: XrandrModeXidV1,
    pub randr_provider_xid: XrandrProviderXidV1,
    pub randr_timestamp: u32,
    pub randr_config_timestamp: u32,
    pub width_px: u16,
    pub height_px: u16,
    pub origin_x: i16,
    pub origin_y: i16,
    pub randr_primary: bool,
    pub refresh_hz: RefreshRateV1,
    pub exact_timing: ExactModeTimingV1,
    pub edid_sha256: Option<Sha256DigestV1>,
    pub randr_connector_kind: PhysicalConnectorKindV1,
    pub randr_connector_number: Option<u32>,
    pub randr_provider_name: OutputNameV1,
    pub randr_provider_capabilities: u32,
    pub x11_library: String,
    pub xnvctrl_library: String,
    pub nvcontrol_extension_version: NvControlExtensionVersionV1,
    pub nvcontrol_x_screen: u32,
    pub nvcontrol_display_target_id: NvControlDisplayTargetIdV1,
    pub nvcontrol_display_randr_name: OutputNameV1,
    pub nvcontrol_display_target_index_name: String,
    pub nvcontrol_display_type_id_name: String,
    pub nvcontrol_display_dp_guid: Option<String>,
    pub nvcontrol_display_edid_hash: Option<String>,
    pub nvcontrol_display_enabled: bool,
    pub nvcontrol_displayport_is_multistream: bool,
    pub nvcontrol_gpu_target_id: NvControlGpuTargetIdV1,
    pub nvcontrol_gpu_pci_domain: u32,
    pub nvcontrol_gpu_pci_bus: u32,
    pub nvcontrol_gpu_pci_device: u32,
    pub nvcontrol_gpu_pci_function: u32,
    pub nvcontrol_gpu_pci_bdf: String,
    pub nvcontrol_gpu_uuid: String,
    pub nvml_pci_bdf: String,
    pub nvml_uuid: String,
    pub drm_diagnostic_before: Option<DrmDiagnosticSnapshotV1>,
    pub drm_diagnostic_after: Option<DrmDiagnosticSnapshotV1>,
    pub topology_token: OutputTopologyTokenV1,
    pub proof: OutputMappingProofCardinalitiesV1,
}

pub const NVFBC_CAPTURE_SCHEMA_V1: &str = "replaydesktop.nvfbc-capture.v1";
pub const NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1: &str = "replaydesktop.nvfbc-capture-primitives.v1";
pub const MAX_CAPTURE_PLANES_V1: usize = 3;
pub const MAX_CAPTURE_COPY_EDGES_V1: usize = 16;
pub const MAX_CAPTURE_LIFECYCLE_EVENTS_V1: usize = 32;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureProviderKindV1 {
    Fixture,
    LiveUnavailable,
    SourceAuthenticated,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureSourceStatusV1 {
    Fixture,
    Unavailable,
    Authenticated,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureSourceEvidenceV1 {
    pub status: CaptureSourceStatusV1,
    pub identity: Option<String>,
    pub api_version: Option<u32>,
    pub nvfbc_header_sha256: Option<Sha256DigestV1>,
    pub cuda_header_sha256: Option<Sha256DigestV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cuda_typedefs_header_sha256: Option<Sha256DigestV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nvfbc_runtime_library: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cuda_runtime_library: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cuda_driver_version: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGpuIdentityV1 {
    pub pci_bdf: String,
    pub gpu_uuid: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureOutputBindingV1 {
    pub output_name: OutputNameV1,
    pub randr_output_xid: XrandrOutputXidV1,
    pub topology_token: OutputTopologyTokenV1,
    pub gpu_pci_bdf: String,
    pub gpu_uuid: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureGrabStatusV1 {
    Success,
    NoNewFrame,
    AccessDenied,
    ProtectedContent,
    Busy,
    DriverError,
    ApiMismatch,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapturePixelFormatV1 {
    Bgra,
    Nv12,
    Yuv444p,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePlaneV1 {
    pub index: u8,
    pub offset_bytes: u64,
    pub stride_bytes: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureCursorModeV1 {
    NvfbcComposited,
    ClientComposited,
    Excluded,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CopyEdgeKindV1 {
    ZeroCopy,
    SameGpuDeviceCopy,
    PeerCopy,
    HostStaged,
    Conversion,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CopyLedgerEdgeV1 {
    pub sequence: u16,
    pub kind: CopyEdgeKindV1,
    pub from_surface: String,
    pub to_surface: String,
    pub from_gpu: CaptureGpuIdentityV1,
    pub to_gpu: CaptureGpuIdentityV1,
    pub input_format: CapturePixelFormatV1,
    pub output_format: CapturePixelFormatV1,
    pub peer_access: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureFrameObservationV1 {
    pub grab_status: CaptureGrabStatusV1,
    pub frame_sequence: u64,
    pub timestamp_us: u64,
    pub is_new_frame: bool,
    pub width_px: u16,
    pub height_px: u16,
    pub pixel_format: CapturePixelFormatV1,
    pub pitch_bytes: u32,
    pub planes: Vec<CapturePlaneV1>,
    pub required_post_processing: bool,
    pub cursor_included: bool,
    pub cursor_mode: CaptureCursorModeV1,
    pub source_surface: String,
    pub lease_surface: String,
    pub edges: Vec<CopyLedgerEdgeV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_pixel_format: Option<CapturePixelFormatV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed_frames: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_capture: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_requested: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_composited: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_flags: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_timeout_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_elapsed_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureLifecycleEventV1 {
    CudaLibraryLoaded,
    CudaContextCreated,
    LibraryLoaded,
    HandleCreated,
    StatusQueried,
    ContextBound,
    SessionCreated,
    ToCudaSetup,
    ApplicationBufferAllocated,
    FrameGrabbed,
    FrameReleased,
    ApplicationBufferFreed,
    SessionDestroyed,
    HandleDestroyed,
    ContextReleased,
    LibraryUnloaded,
    CudaContextDestroyed,
    CudaLibraryUnloaded,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaptureFailureV1 {
    SourceUnavailable,
    SourceMismatch,
    BindingMismatch,
    ApiMismatch,
    AccessDenied,
    ProtectedContent,
    Busy,
    NoNewFrame,
    StaleFrame,
    InvalidFrame,
    InvalidCopyLedger,
    CleanupUncertain,
    WorkerRejected,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePrimitiveObservationV1 {
    pub schema: String,
    pub provider: CaptureProviderKindV1,
    pub source: CaptureSourceEvidenceV1,
    pub binding: Option<CaptureOutputBindingV1>,
    pub frame: Option<CaptureFrameObservationV1>,
    pub lifecycle: Vec<CaptureLifecycleEventV1>,
    pub failure: Option<CaptureFailureV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nvfbc_status_raw: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureFrameLeaseV1 {
    pub frame_sequence: u64,
    pub timestamp_us: u64,
    pub new_frame: bool,
    pub width_px: u16,
    pub height_px: u16,
    pub pixel_format: CapturePixelFormatV1,
    pub pitch_bytes: u32,
    pub planes: Vec<CapturePlaneV1>,
    pub required_post_processing: bool,
    pub cursor_included: bool,
    pub cursor_mode: CaptureCursorModeV1,
    pub source_surface: String,
    pub lease_surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_pixel_format: Option<CapturePixelFormatV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed_frames: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_capture: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_requested: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_composited: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_flags: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_timeout_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grab_elapsed_ns: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CopyLedgerV1 {
    pub edges: Vec<CopyLedgerEdgeV1>,
    pub zero_copy_edges: u16,
    pub device_copy_edges: u16,
    pub peer_copy_edges: u16,
    pub conversion_edges: u16,
    pub host_staged_edges: u16,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureCleanupLedgerV1 {
    pub acquired: Vec<CaptureLifecycleEventV1>,
    pub released: Vec<CaptureLifecycleEventV1>,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureAdmissionV1 {
    Pass,
    Unproven,
    Rejected,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureBoundaryStatusV1 {
    Unproven,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePathEvidenceV1 {
    pub schema: String,
    pub provider: CaptureProviderKindV1,
    pub source: CaptureSourceEvidenceV1,
    pub admission: CaptureAdmissionV1,
    pub binding: Option<CaptureOutputBindingV1>,
    pub lease: Option<CaptureFrameLeaseV1>,
    pub copy_ledger: Option<CopyLedgerV1>,
    pub cleanup: CaptureCleanupLedgerV1,
    pub failure: Option<CaptureFailureV1>,
    pub nvenc_boundary: CaptureBoundaryStatusV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nvfbc_status_raw: Option<i32>,
}

pub const NVENC_TUPLES_SCHEMA_V1: &str = "replaydesktop.nvenc-tuples.v1";
pub const NVENC_POLICY_POSITION_COUNT_V1: usize = 7;
pub const MAX_NVENC_COPY_EDGES_V1: usize = 16;
pub const MAX_NVENC_RESOURCE_EVENTS_V1: usize = 16;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencApiVersionV1 {
    pub major: u16,
    pub minor: u16,
}

impl NvencApiVersionV1 {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn is_sdk_13_1(self) -> bool {
        self.major == 13 && self.minor == 1
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencGpuGenerationV1 {
    TuringOrOlder,
    Ampere,
    Ada,
    BlackwellOrNewer,
    Unknown,
}

impl NvencGpuGenerationV1 {
    pub const fn supports_av1_encode(self) -> bool {
        matches!(self, Self::Ada | Self::BlackwellOrNewer)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencCodecV1 {
    H264,
    Hevc,
    Av1,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencProfileV1 {
    H264High,
    HevcMain,
    HevcMain10,
    HevcFrext,
    Av1Main,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencChromaV1 {
    Yuv420,
    Yuv444,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencBufferFormatV1 {
    Nv12,
    Yuv420TenBit,
    Yuv444,
    Yuv444TenBit,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencPolicyPositionV1 {
    H264HighYuv420EightBit,
    HevcMainYuv420EightBit,
    HevcMain10Yuv420TenBit,
    HevcFrextYuv444EightBit,
    HevcFrextYuv444TenBit,
    Av1MainYuv420EightBit,
    Av1MainYuv420TenBit,
}

impl NvencPolicyPositionV1 {
    pub const ALL: [Self; NVENC_POLICY_POSITION_COUNT_V1] = [
        Self::H264HighYuv420EightBit,
        Self::HevcMainYuv420EightBit,
        Self::HevcMain10Yuv420TenBit,
        Self::HevcFrextYuv444EightBit,
        Self::HevcFrextYuv444TenBit,
        Self::Av1MainYuv420EightBit,
        Self::Av1MainYuv420TenBit,
    ];

    pub const fn tuple(self) -> NvencTupleV1 {
        let (codec, profile, chroma, bit_depth, buffer_format) = match self {
            Self::H264HighYuv420EightBit => (
                NvencCodecV1::H264,
                NvencProfileV1::H264High,
                NvencChromaV1::Yuv420,
                8,
                NvencBufferFormatV1::Nv12,
            ),
            Self::HevcMainYuv420EightBit => (
                NvencCodecV1::Hevc,
                NvencProfileV1::HevcMain,
                NvencChromaV1::Yuv420,
                8,
                NvencBufferFormatV1::Nv12,
            ),
            Self::HevcMain10Yuv420TenBit => (
                NvencCodecV1::Hevc,
                NvencProfileV1::HevcMain10,
                NvencChromaV1::Yuv420,
                10,
                NvencBufferFormatV1::Yuv420TenBit,
            ),
            Self::HevcFrextYuv444EightBit => (
                NvencCodecV1::Hevc,
                NvencProfileV1::HevcFrext,
                NvencChromaV1::Yuv444,
                8,
                NvencBufferFormatV1::Yuv444,
            ),
            Self::HevcFrextYuv444TenBit => (
                NvencCodecV1::Hevc,
                NvencProfileV1::HevcFrext,
                NvencChromaV1::Yuv444,
                10,
                NvencBufferFormatV1::Yuv444TenBit,
            ),
            Self::Av1MainYuv420EightBit => (
                NvencCodecV1::Av1,
                NvencProfileV1::Av1Main,
                NvencChromaV1::Yuv420,
                8,
                NvencBufferFormatV1::Nv12,
            ),
            Self::Av1MainYuv420TenBit => (
                NvencCodecV1::Av1,
                NvencProfileV1::Av1Main,
                NvencChromaV1::Yuv420,
                10,
                NvencBufferFormatV1::Yuv420TenBit,
            ),
        };
        NvencTupleV1 {
            codec,
            profile,
            chroma,
            bit_depth,
            buffer_format,
            width_px: 3840,
            height_px: 2160,
            frame_rate: RefreshRateV1 {
                numerator: 60,
                denominator: 1,
            },
        }
    }

    pub const fn requires_ada_or_newer(self) -> bool {
        matches!(
            self,
            Self::Av1MainYuv420EightBit | Self::Av1MainYuv420TenBit
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "NvencTupleWireV1", into = "NvencTupleWireV1")]
pub struct NvencTupleV1 {
    codec: NvencCodecV1,
    profile: NvencProfileV1,
    chroma: NvencChromaV1,
    bit_depth: u8,
    buffer_format: NvencBufferFormatV1,
    width_px: u16,
    height_px: u16,
    frame_rate: RefreshRateV1,
}

impl NvencTupleV1 {
    pub const fn codec(self) -> NvencCodecV1 {
        self.codec
    }

    pub const fn profile(self) -> NvencProfileV1 {
        self.profile
    }

    pub const fn chroma(self) -> NvencChromaV1 {
        self.chroma
    }

    pub const fn bit_depth(self) -> u8 {
        self.bit_depth
    }

    pub const fn buffer_format(self) -> NvencBufferFormatV1 {
        self.buffer_format
    }

    pub const fn width_px(self) -> u16 {
        self.width_px
    }

    pub const fn height_px(self) -> u16 {
        self.height_px
    }

    pub const fn frame_rate(self) -> RefreshRateV1 {
        self.frame_rate
    }

    pub fn position(self) -> NvencPolicyPositionV1 {
        NvencPolicyPositionV1::ALL
            .into_iter()
            .find(|position| position.tuple() == self)
            .expect("NvencTupleV1 construction is closed to policy positions")
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct NvencTupleWireV1 {
    codec: NvencCodecV1,
    profile: NvencProfileV1,
    chroma: NvencChromaV1,
    bit_depth: u8,
    buffer_format: NvencBufferFormatV1,
    width_px: u16,
    height_px: u16,
    frame_rate: RefreshRateV1,
}

impl TryFrom<NvencTupleWireV1> for NvencTupleV1 {
    type Error = &'static str;

    fn try_from(wire: NvencTupleWireV1) -> Result<Self, Self::Error> {
        NvencPolicyPositionV1::ALL
            .into_iter()
            .map(NvencPolicyPositionV1::tuple)
            .find(|candidate| {
                candidate.codec == wire.codec
                    && candidate.profile == wire.profile
                    && candidate.chroma == wire.chroma
                    && candidate.bit_depth == wire.bit_depth
                    && candidate.buffer_format == wire.buffer_format
                    && candidate.width_px == wire.width_px
                    && candidate.height_px == wire.height_px
                    && candidate.frame_rate == wire.frame_rate
            })
            .ok_or("NVENC tuple is outside the closed seven-position policy")
    }
}

impl From<NvencTupleV1> for NvencTupleWireV1 {
    fn from(tuple: NvencTupleV1) -> Self {
        Self {
            codec: tuple.codec,
            profile: tuple.profile,
            chroma: tuple.chroma,
            bit_depth: tuple.bit_depth,
            buffer_format: tuple.buffer_format,
            width_px: tuple.width_px,
            height_px: tuple.height_px,
            frame_rate: tuple.frame_rate,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CopyBoundaryStatusV1 {
    Pass,
    BlockedUnknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencCopyEdgeV1 {
    RegisterNoCopy,
    MapNoCopy,
    SubmitNoCopy,
    SameGpuPitchLinearToBlockLinearCopy,
    DeviceToHost,
    HostToDevice,
    CrossGpu,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CopyBoundaryProofV1 {
    pub status: CopyBoundaryStatusV1,
    pub application_edges: Vec<NvencCopyEdgeV1>,
    pub encoder_internal_edges: Vec<NvencCopyEdgeV1>,
    pub host_staging_edges: u16,
    pub cross_gpu_edges: u16,
    pub unknown_application_edges: u16,
    pub unknown_encoder_internal_edges: u16,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencStreamProofV1 {
    pub bitstream_sha256: Sha256DigestV1,
    pub byte_len: u32,
    pub keyframe: bool,
    pub parsed_tuple: NvencTupleV1,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencResourceEventV1 {
    EncoderSession,
    RegisteredResource,
    MappedResource,
    BitstreamBuffer,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencCleanupProofV1 {
    pub acquired: Vec<NvencResourceEventV1>,
    pub released: Vec<NvencResourceEventV1>,
    pub complete: bool,
}

impl NvencCleanupProofV1 {
    pub fn no_resources() -> Self {
        Self {
            acquired: Vec::new(),
            released: Vec::new(),
            complete: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencAttemptOutcomeV1 {
    Success,
    GenerationIneligible,
    Unsupported,
    ProviderUnavailable,
    Rejected,
    Timeout,
    InvalidStream,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencTupleAttemptV1 {
    pub position: NvencPolicyPositionV1,
    pub tuple: NvencTupleV1,
    pub api_version: NvencApiVersionV1,
    pub provider_invoked: bool,
    pub terminal: bool,
    pub outcome: NvencAttemptOutcomeV1,
    pub copy_proof: Option<CopyBoundaryProofV1>,
    pub stream_proof: Option<NvencStreamProofV1>,
    pub cleanup: NvencCleanupProofV1,
}

impl NvencTupleAttemptV1 {
    pub fn generation_ineligible(
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
        api_version: NvencApiVersionV1,
    ) -> Self {
        Self {
            position,
            tuple,
            api_version,
            provider_invoked: false,
            terminal: true,
            outcome: NvencAttemptOutcomeV1::GenerationIneligible,
            copy_proof: None,
            stream_proof: None,
            cleanup: NvencCleanupProofV1::no_resources(),
        }
    }

    pub fn provider_unavailable(
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
        api_version: NvencApiVersionV1,
    ) -> Self {
        Self {
            position,
            tuple,
            api_version,
            provider_invoked: true,
            terminal: true,
            outcome: NvencAttemptOutcomeV1::ProviderUnavailable,
            copy_proof: None,
            stream_proof: None,
            cleanup: NvencCleanupProofV1::no_resources(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencAdvertisementV1 {
    pub position: NvencPolicyPositionV1,
    pub tuple: NvencTupleV1,
    pub bitstream_sha256: Sha256DigestV1,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencProviderKindV1 {
    DiagnosticFixture,
    LiveUnavailable,
    SourceAuthenticated,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencSourceEvidenceV1 {
    pub api_version: NvencApiVersionV1,
    pub source_identity: Option<String>,
    pub header_sha256: Option<Sha256DigestV1>,
    pub runtime_library: Option<String>,
}

impl NvencSourceEvidenceV1 {
    pub fn unavailable(api_version: NvencApiVersionV1) -> Self {
        Self {
            api_version,
            source_identity: None,
            header_sha256: None,
            runtime_library: None,
        }
    }

    pub fn diagnostic(api_version: NvencApiVersionV1) -> Self {
        Self::unavailable(api_version)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvencAdmissionV1 {
    Pass,
    Unproven,
    Rejected,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvencTuplesEvidenceV1 {
    pub schema: String,
    pub provider: NvencProviderKindV1,
    pub source: NvencSourceEvidenceV1,
    pub admission: NvencAdmissionV1,
    pub gpu_generation: NvencGpuGenerationV1,
    pub attempts: Vec<NvencTupleAttemptV1>,
    pub advertised: Vec<NvencAdvertisementV1>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digest::sha256_bytes;
    use serde_json::Value;

    fn fixture_bytes() -> Vec<u8> {
        include_bytes!("../tests/fixtures/g0-envelope-v1-foundation.json").to_vec()
    }

    fn fixture_value() -> Value {
        serde_json::from_slice(&fixture_bytes()).expect("foundation fixture must be JSON")
    }

    fn encode_value(value: &Value) -> Vec<u8> {
        let mut value = value.clone();
        for extension in value["extensions"]
            .as_array_mut()
            .expect("extensions array")
        {
            let payload =
                serde_json::to_string(&extension["payload"]).expect("test payload must serialize");
            extension["payload_sha256"] = sha256_bytes(payload.as_bytes()).to_string().into();
        }
        serde_json::to_vec(&value).expect("test mutation must serialize")
    }

    fn set_raw_payload(value: &mut Value, extension_index: usize, payload: &str) {
        value["extensions"][extension_index]["payload"] =
            serde_json::from_str(payload).expect("test payload must be JSON");
        value["extensions"][extension_index]["payload_sha256"] =
            sha256_bytes(payload.as_bytes()).to_string().into();
    }

    #[test]
    fn g0_envelope_strict_rejects_duplicate_base_and_reason_keys() {
        let fixture = String::from_utf8(fixture_bytes()).expect("fixture must be UTF-8");
        let duplicate_base = fixture.replacen(
            r#"    "run_id": "foundation-fixture-run","#,
            "    \"run_id\": \"foundation-fixture-run\",\n    \"run_id\": \"duplicate\",",
            1,
        );
        assert!(decode_g0_evidence(duplicate_base.as_bytes()).is_err());

        let duplicate_reason = fixture.replacen(
            r#"        "status": "unproven""#,
            "        \"status\": \"unproven\",\n        \"status\": \"fail\"",
            1,
        );
        assert!(decode_g0_evidence(duplicate_reason.as_bytes()).is_err());
    }

    #[test]
    fn g0_envelope_strict_rejects_duplicate_extension_record_and_payload_keys() {
        let fixture = String::from_utf8(fixture_bytes()).expect("fixture must be UTF-8");
        let duplicate_record = fixture.replacen(
            r#"      "version": 1,"#,
            "      \"version\": 1,\n      \"version\": 2,",
            1,
        );
        assert!(decode_g0_evidence(duplicate_record.as_bytes()).is_err());

        let duplicate_payload = fixture.replacen(
            r#"      "payload": {},"#,
            "      \"payload\": {\"duplicate\": 1, \"duplicate\": 2},",
            1,
        );
        assert!(matches!(
            decode_g0_evidence(duplicate_payload.as_bytes()),
            Err(G0DecodeError::InvalidPayloadJson { .. })
        ));
    }

    #[test]
    fn g0_envelope_strict_rejects_invalid_unicode_and_integer_forms() {
        let mut invalid_utf8 = fixture_bytes();
        let run_id_byte = invalid_utf8
            .windows("foundation-fixture-run".len())
            .position(|window| window == b"foundation-fixture-run")
            .expect("fixture run id must exist");
        invalid_utf8[run_id_byte] = 0xff;
        assert!(decode_g0_evidence(&invalid_utf8).is_err());

        let fixture = String::from_utf8(fixture_bytes()).expect("fixture must be UTF-8");
        let fractional = fixture.replacen(
            r#"    "wall_started_unix_ns": 1785100000000000000,"#,
            r#"    "wall_started_unix_ns": 1.5,"#,
            1,
        );
        assert!(decode_g0_evidence(fractional.as_bytes()).is_err());

        let overflow = fixture.replacen(
            r#"    "wall_started_unix_ns": 1785100000000000000,"#,
            r#"    "wall_started_unix_ns": 18446744073709551616,"#,
            1,
        );
        assert!(decode_g0_evidence(overflow.as_bytes()).is_err());
    }

    #[test]
    fn g0_envelope_strict_enforces_payload_and_envelope_boundaries() {
        let mut value = fixture_value();
        let exact_payload = format!("\"{}\"", "x".repeat(MAX_G0_EXTENSION_PAYLOAD_BYTES - 2));
        set_raw_payload(&mut value, 4, &exact_payload);
        let exact_payload_bytes = encode_value(&value);
        assert!(decode_g0_evidence(&exact_payload_bytes).is_ok());

        let oversized_payload = format!("\"{}\"", "x".repeat(MAX_G0_EXTENSION_PAYLOAD_BYTES - 1));
        set_raw_payload(&mut value, 4, &oversized_payload);
        assert!(matches!(
            decode_g0_evidence(&encode_value(&value)),
            Err(G0DecodeError::PayloadTooLarge { .. })
        ));

        let mut exact_envelope = fixture_bytes();
        exact_envelope.resize(MAX_G0_ENVELOPE_BYTES, b' ');
        assert!(decode_g0_evidence(&exact_envelope).is_ok());
        exact_envelope.push(b' ');
        assert!(matches!(
            decode_g0_evidence(&exact_envelope),
            Err(G0DecodeError::InputTooLarge { .. })
        ));
    }

    #[test]
    fn g0_envelope_strict_rejects_malformed_payload_and_gate_summary_mismatch() {
        let fixture = String::from_utf8(fixture_bytes()).expect("fixture must be UTF-8");
        let malformed = fixture.replacen(
            r#"      "payload": {},"#,
            "      \"payload\": {\"unterminated\": true,",
            1,
        );
        assert!(decode_g0_evidence(malformed.as_bytes()).is_err());

        let mut inconsistent = fixture_value();
        inconsistent["base"]["provenance"] = "live".into();
        inconsistent["base"]["status"] = "pass".into();
        inconsistent["base"]["reasons"] = Value::Array(Vec::new());
        assert!(decode_g0_evidence(&encode_value(&inconsistent)).is_err());

        let mut missing_known = fixture_value();
        missing_known["extensions"]
            .as_array_mut()
            .expect("extensions array")
            .remove(0);
        missing_known["base"]["reasons"]
            .as_array_mut()
            .expect("reasons array")
            .remove(0);
        assert!(decode_g0_evidence(&encode_value(&missing_known)).is_err());
    }

    #[test]
    fn g0_unknown_extension_preservation() {
        const UNKNOWN_ID: &str = "future-display-proof.v2";
        const UNKNOWN_PAYLOAD: &str = r#"{ "future": [1, 2, 3], "note": "preserve me" }"#;

        let decoded = decode_g0_evidence(&fixture_bytes()).expect("fixture must decode");
        let unknown = decoded
            .as_v1()
            .extensions
            .iter()
            .find(|extension| extension.id == UNKNOWN_ID)
            .expect("unknown extension must remain present");
        assert_eq!(unknown.payload.get(), UNKNOWN_PAYLOAD);
        assert_eq!(
            sha256_bytes(unknown.payload.get().as_bytes()),
            unknown.payload_sha256
        );

        let encoded = serde_json::to_vec(decoded.as_v1()).expect("envelope must re-encode");
        let decoded_again = decode_g0_evidence(&encoded).expect("re-encoded envelope must decode");
        let unknown_again = decoded_again
            .as_v1()
            .extensions
            .iter()
            .find(|extension| extension.id == UNKNOWN_ID)
            .expect("unknown extension must survive re-encoding");
        assert_eq!(unknown_again.payload.get(), UNKNOWN_PAYLOAD);
    }

    #[test]
    fn g0_envelope_strict_keeps_known_payload_semantics_separate() {
        let mut value = fixture_value();
        let payload = r#"{"future_known_schema":{"nested":[true,42,"opaque"]}}"#;
        set_raw_payload(&mut value, 0, payload);
        let decoded = decode_g0_evidence(&encode_value(&value))
            .expect("base decoder must not own known extension payload semantics");
        assert_eq!(decoded.as_v1().extensions[0].payload.get(), payload);
    }
}
