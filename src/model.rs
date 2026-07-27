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
    NotImplemented,
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
            Self::NotImplemented => formatter.write_str("G0 evidence decoding is not implemented"),
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
