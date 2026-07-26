pub mod digest;
pub mod model;

pub use digest::{Sha256DigestV1, sha256_bytes, sha256_file, sha256_reader};
pub use model::{
    DecodedG0Evidence, G0DecodeError, G0EvidenceBaseV1, G0EvidenceEnvelopeV1, G0ExtensionRecordV1,
    decode_g0_evidence,
};

#[cfg(test)]
fn foundation_fixture() -> &'static [u8] {
    include_bytes!("../tests/fixtures/g0-envelope-v1-foundation.json")
}

#[cfg(test)]
fn fixture_value() -> serde_json::Value {
    serde_json::from_slice(foundation_fixture()).expect("foundation fixture must be JSON")
}

#[cfg(test)]
#[test]
fn g0_envelope_foundation_roundtrip() {
    use model::{
        G0_BASE_SCHEMA_V1, G0_ENVELOPE_SCHEMA_V1, HOST_FOUNDATION_EXTENSION_ID,
        NVENC_TUPLES_EXTENSION_ID, NVFBC_CAPTURE_EXTENSION_ID, SELECTED_OUTPUT_EXTENSION_ID,
    };

    const UNKNOWN_ID: &str = "future-display-proof.v2";
    const UNKNOWN_PAYLOAD: &str = r#"{ "future": [1, 2, 3], "note": "preserve me" }"#;

    let decoded = decode_g0_evidence(foundation_fixture()).expect("V1 fixture must decode");
    let envelope = decoded.as_v1();

    assert_eq!(envelope.schema, G0_ENVELOPE_SCHEMA_V1);
    assert_eq!(envelope.version, 1);
    assert_eq!(envelope.base.schema, G0_BASE_SCHEMA_V1);

    let identifiers: Vec<_> = envelope
        .extensions
        .iter()
        .map(|extension| extension.id.as_str())
        .collect();
    assert_eq!(
        identifiers,
        [
            HOST_FOUNDATION_EXTENSION_ID,
            SELECTED_OUTPUT_EXTENSION_ID,
            NVFBC_CAPTURE_EXTENSION_ID,
            NVENC_TUPLES_EXTENSION_ID,
            UNKNOWN_ID,
        ]
    );

    let unknown = envelope
        .extensions
        .iter()
        .find(|extension| extension.id == UNKNOWN_ID)
        .expect("unknown extension must be preserved");
    assert_eq!(unknown.payload.get(), UNKNOWN_PAYLOAD);

    let base_before = envelope.base.clone();
    let reemitted = serde_json::to_vec(envelope).expect("V1 envelope must serialize");
    let decoded_again = decode_g0_evidence(&reemitted).expect("re-emitted V1 must decode");
    let envelope_again = decoded_again.as_v1();
    assert_eq!(envelope_again.base, base_before);
    assert_eq!(
        envelope_again
            .extensions
            .iter()
            .find(|extension| extension.id == UNKNOWN_ID)
            .expect("unknown extension must survive re-emission")
            .payload
            .get(),
        UNKNOWN_PAYLOAD
    );
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_total_size() {
    let oversized = vec![b' '; model::MAX_G0_ENVELOPE_BYTES + 1];
    assert!(matches!(
        decode_g0_evidence(&oversized),
        Err(G0DecodeError::InputTooLarge { .. })
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_extension_count() {
    let mut value = fixture_value();
    let extensions = value["extensions"]
        .as_array_mut()
        .expect("extensions array");
    let template = extensions[4].clone();
    while extensions.len() <= model::MAX_G0_EXTENSIONS {
        let mut extension = template.clone();
        extension["id"] = format!("future-extension-{}.v1", extensions.len()).into();
        extensions.push(extension);
    }
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::TooManyExtensions { .. })
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_payload_size() {
    let mut value = fixture_value();
    value["extensions"][4]["payload"] =
        "x".repeat(model::MAX_G0_EXTENSION_PAYLOAD_BYTES + 1).into();
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::PayloadTooLarge { .. })
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_duplicate_extension_identifier() {
    let mut value = fixture_value();
    let duplicate = value["extensions"][0].clone();
    value["extensions"]
        .as_array_mut()
        .expect("extensions array")
        .push(duplicate);
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::DuplicateExtensionIdentifier(_))
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_invalid_payload_digest() {
    let mut value = fixture_value();
    value["extensions"][4]["payload_sha256"] = "0".repeat(64).into();
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::PayloadDigestMismatch { .. })
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_unknown_base_field() {
    let mut value = fixture_value();
    value["base"]["future"] = true.into();
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::InvalidEnvelope(_))
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_duplicate_top_level_field() {
    let fixture = String::from_utf8(foundation_fixture().to_vec()).expect("UTF-8 fixture");
    let duplicate = fixture.replacen(
        r#"  "version": 1,"#,
        "  \"version\": 1,\n  \"version\": 1,",
        1,
    );
    assert!(decode_g0_evidence(duplicate.as_bytes()).is_err());
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_unsupported_version() {
    let mut value = fixture_value();
    value["version"] = 2.into();
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::UnsupportedVersion(2))
    ));
}

#[cfg(test)]
#[test]
fn g0_envelope_bounds_invalid_extension_identifier() {
    let mut value = fixture_value();
    value["extensions"][4]["id"] = "future-\u{2603}.v1".into();
    let bytes = serde_json::to_vec(&value).expect("mutated fixture");
    assert!(matches!(
        decode_g0_evidence(&bytes),
        Err(G0DecodeError::InvalidExtensionIdentifier(_))
    ));
}
