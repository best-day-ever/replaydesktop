use replay_host_doctor::digest::sha256_bytes;
use replay_host_doctor::evidence::validate_nvenc_tuples_record;
use replay_host_doctor::model::{
    CopyBoundaryStatusV1, G0ExtensionRecordV1, G0ExtensionStatusV1, NVENC_TUPLES_EXTENSION_ID,
    NvencAdmissionV1, NvencApiVersionV1, NvencAttemptOutcomeV1, NvencGpuGenerationV1,
    NvencPolicyPositionV1, NvencProviderKindV1, NvencSourceEvidenceV1, NvencTupleAttemptV1,
    NvencTupleV1,
};
use replay_host_doctor::native_nvenc::{
    DiagnosticNvencProvider, LiveUnavailableNvencProvider, NvencProvider,
    advertisement_from_attempt, evaluate_nvenc_policy, nvenc_policy_positions,
    validate_nvenc_tuples_evidence,
};
use serde_json::value::RawValue;

#[test]
fn host04_policy_exact_seven_positions_and_h264_high_first() {
    let positions = nvenc_policy_positions();
    assert_eq!(positions.len(), 7);
    assert_eq!(positions[0], NvencPolicyPositionV1::H264HighYuv420EightBit);
    assert_eq!(
        positions,
        &[
            NvencPolicyPositionV1::H264HighYuv420EightBit,
            NvencPolicyPositionV1::HevcMainYuv420EightBit,
            NvencPolicyPositionV1::HevcMain10Yuv420TenBit,
            NvencPolicyPositionV1::HevcFrextYuv444EightBit,
            NvencPolicyPositionV1::HevcFrextYuv444TenBit,
            NvencPolicyPositionV1::Av1MainYuv420EightBit,
            NvencPolicyPositionV1::Av1MainYuv420TenBit,
        ]
    );

    for position in positions {
        let tuple = position.tuple();
        assert_eq!(tuple.width_px(), 3840);
        assert_eq!(tuple.height_px(), 2160);
        assert_eq!(tuple.frame_rate().numerator, 60);
        assert_eq!(tuple.frame_rate().denominator, 1);
    }
}

#[test]
fn host04_policy_av1_444_cannot_deserialize() {
    let impossible = serde_json::json!({
        "codec": "av1",
        "profile": "av1-main",
        "chroma": "yuv444",
        "bit_depth": 8,
        "buffer_format": "yuv444",
        "width_px": 3840,
        "height_px": 2160,
        "frame_rate": { "numerator": 60, "denominator": 1 }
    });
    assert!(serde_json::from_value::<NvencTupleV1>(impossible).is_err());
}

#[test]
fn host04_policy_advertisement_requires_complete_exact_attempt() {
    let fixture = include_str!("fixtures/host04-nvenc-tuples.json");
    let root: serde_json::Value = serde_json::from_str(fixture).expect("fixture JSON");
    let mut attempt: NvencTupleAttemptV1 =
        serde_json::from_value(root["complete_attempt"].clone()).expect("complete attempt");
    assert!(advertisement_from_attempt(&attempt).is_some());

    attempt.copy_proof.as_mut().expect("copy proof").status = CopyBoundaryStatusV1::BlockedUnknown;
    assert!(advertisement_from_attempt(&attempt).is_none());

    let mut attempt: NvencTupleAttemptV1 =
        serde_json::from_value(root["complete_attempt"].clone()).expect("complete attempt");
    attempt.cleanup.complete = false;
    assert!(advertisement_from_attempt(&attempt).is_none());

    let mut attempt: NvencTupleAttemptV1 =
        serde_json::from_value(root["complete_attempt"].clone()).expect("complete attempt");
    attempt
        .config_proof
        .as_mut()
        .expect("configuration proof")
        .synchronous = false;
    assert!(advertisement_from_attempt(&attempt).is_none());

    let mut attempt: NvencTupleAttemptV1 =
        serde_json::from_value(root["complete_attempt"].clone()).expect("complete attempt");
    attempt
        .resource_proof
        .as_mut()
        .expect("resource proof")
        .same_cuda_context = false;
    assert!(advertisement_from_attempt(&attempt).is_none());
}

#[derive(Debug)]
struct SuccessThenTimeoutProvider {
    source: NvencSourceEvidenceV1,
    complete: NvencTupleAttemptV1,
    calls: usize,
}

impl SuccessThenTimeoutProvider {
    fn new() -> Self {
        let root: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/host04-nvenc-tuples.json"))
                .expect("fixture JSON");
        Self {
            source: serde_json::from_value(serde_json::json!({
                "api_version": { "major": 13, "minor": 1 },
                "source_identity": "nvidia-video-codec-sdk-13.1",
                "header_sha256":
                    "1111111111111111111111111111111111111111111111111111111111111111",
                "runtime_library": "libnvidia-encode.so.1"
            }))
            .expect("source fixture"),
            complete: serde_json::from_value(root["complete_attempt"].clone())
                .expect("complete attempt"),
            calls: 0,
        }
    }
}

impl NvencProvider for SuccessThenTimeoutProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        NvencProviderKindV1::SourceAuthenticated
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        self.calls += 1;
        let mut attempt = self.complete.clone();
        attempt.position = position;
        attempt.tuple = tuple;
        if self.calls == 2 {
            attempt.outcome = NvencAttemptOutcomeV1::Timeout;
            attempt.config_proof = None;
            attempt.resource_proof = None;
            attempt.copy_proof = None;
            attempt.stream_proof = None;
            attempt.cleanup = replay_host_doctor::model::NvencCleanupProofV1::no_resources();
        } else if self.calls > 2 {
            attempt.outcome = NvencAttemptOutcomeV1::Unsupported;
            attempt.config_proof = None;
            attempt.resource_proof = None;
            attempt.copy_proof = None;
            attempt.stream_proof = None;
            attempt.cleanup = replay_host_doctor::model::NvencCleanupProofV1::no_resources();
        }
        attempt
    }
}

#[test]
fn host04_policy_one_success_cannot_mask_a_later_timeout() {
    let mut provider = SuccessThenTimeoutProvider::new();
    let evidence = evaluate_nvenc_policy(NvencGpuGenerationV1::Ampere, &mut provider);

    assert_eq!(evidence.attempts.len(), 7);
    assert_eq!(evidence.admission, NvencAdmissionV1::Rejected);
    assert!(evidence.advertised.is_empty());
}

#[derive(Debug)]
struct RecordingUnavailableProvider {
    calls: Vec<NvencPolicyPositionV1>,
    source: NvencSourceEvidenceV1,
}

impl RecordingUnavailableProvider {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            source: NvencSourceEvidenceV1::unavailable(NvencApiVersionV1::new(13, 1)),
        }
    }
}

impl NvencProvider for RecordingUnavailableProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        NvencProviderKindV1::LiveUnavailable
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        self.calls.push(position);
        NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
    }
}

#[test]
fn host04_generation_ampere_marks_av1_terminal_without_provider_call() {
    let mut provider = RecordingUnavailableProvider::new();
    let evidence = evaluate_nvenc_policy(NvencGpuGenerationV1::Ampere, &mut provider);

    assert_eq!(evidence.attempts.len(), 7);
    assert_eq!(provider.calls.len(), 5);
    assert!(
        provider
            .calls
            .iter()
            .all(|position| !position.requires_ada_or_newer())
    );
    for attempt in &evidence.attempts[5..] {
        assert!(attempt.terminal);
        assert!(!attempt.provider_invoked);
        assert_eq!(
            attempt.outcome,
            replay_host_doctor::model::NvencAttemptOutcomeV1::GenerationIneligible
        );
    }
}

#[test]
fn host04_generation_ada_attempts_only_av1_main_420() {
    let mut provider = RecordingUnavailableProvider::new();
    let evidence = evaluate_nvenc_policy(NvencGpuGenerationV1::Ada, &mut provider);

    assert_eq!(provider.calls.len(), 7);
    assert_eq!(evidence.attempts.len(), 7);
    assert_eq!(
        provider.calls[5..],
        [
            NvencPolicyPositionV1::Av1MainYuv420EightBit,
            NvencPolicyPositionV1::Av1MainYuv420TenBit,
        ]
    );
    assert!(evidence.advertised.is_empty());
    assert_eq!(evidence.admission, NvencAdmissionV1::Unproven);
}

fn record(
    status: G0ExtensionStatusV1,
    evidence: &replay_host_doctor::model::NvencTuplesEvidenceV1,
) -> G0ExtensionRecordV1 {
    let payload = serde_json::to_string(evidence).expect("evidence serializes");
    G0ExtensionRecordV1 {
        id: NVENC_TUPLES_EXTENSION_ID.to_owned(),
        version: 1,
        status,
        payload: RawValue::from_string(payload.clone()).expect("raw payload"),
        payload_sha256: sha256_bytes(payload.as_bytes()),
    }
}

#[test]
fn nvenc_extension_live_unavailable_is_strict_unproven() {
    let mut provider = LiveUnavailableNvencProvider::new(NvencApiVersionV1::new(13, 1));
    let evidence = evaluate_nvenc_policy(NvencGpuGenerationV1::Ampere, &mut provider);

    assert!(validate_nvenc_tuples_evidence(&evidence));
    assert!(validate_nvenc_tuples_record(&record(
        G0ExtensionStatusV1::Unproven,
        &evidence
    )));
    assert!(evidence.advertised.is_empty());
}

#[test]
fn nvenc_extension_diagnostic_api_presence_never_advertises() {
    let root: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/host04-nvenc-tuples.json"))
            .expect("fixture JSON");
    for case in root["api_version_cases"].as_array().expect("API cases") {
        let version: NvencApiVersionV1 =
            serde_json::from_value(case["api_version"].clone()).expect("semantic API version");
        let mut provider = DiagnosticNvencProvider::new(version);
        let mut evidence = evaluate_nvenc_policy(NvencGpuGenerationV1::Ada, &mut provider);
        assert_eq!(evidence.provider, NvencProviderKindV1::DiagnosticFixture);
        assert!(evidence.advertised.is_empty());
        assert_eq!(evidence.admission, NvencAdmissionV1::Unproven);
        assert!(validate_nvenc_tuples_evidence(&evidence));

        evidence.advertised.push(
            advertisement_from_attempt(
                &serde_json::from_value(root["complete_attempt"].clone())
                    .expect("complete attempt"),
            )
            .expect("complete attempt can advertise"),
        );
        assert!(!validate_nvenc_tuples_evidence(&evidence));
    }
}
