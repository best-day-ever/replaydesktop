use crate::digest::Sha256DigestV1;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::fmt;

pub const G0_ENVELOPE_SCHEMA_V1: &str = "replaydesktop.g0-evidence-envelope";
pub const G0_BASE_SCHEMA_V1: &str = "replaydesktop.g0-evidence-base.v1";
pub const G0_ENVELOPE_VERSION_V1: u32 = 1;
pub const MAX_G0_ENVELOPE_BYTES: usize = 1024 * 1024;
pub const MAX_G0_EXTENSIONS: usize = 16;
pub const MAX_G0_EXTENSION_PAYLOAD_BYTES: usize = 256 * 1024;

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

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
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
    PayloadBudgetExceeded,
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
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for G0DecodeError {}

pub fn decode_g0_evidence(_input: &[u8]) -> Result<DecodedG0Evidence, G0DecodeError> {
    Err(G0DecodeError::NotImplemented)
}
