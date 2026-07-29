use crate::model::{
    CopyBoundaryProofV1, CopyBoundaryStatusV1, MAX_NVENC_COPY_EDGES_V1,
    MAX_NVENC_RESOURCE_EVENTS_V1, NVENC_POLICY_POSITION_COUNT_V1, NVENC_TUPLES_SCHEMA_V1,
    NvencAdmissionV1, NvencAdvertisementV1, NvencApiVersionV1, NvencAttemptOutcomeV1,
    NvencCleanupProofV1, NvencCopyEdgeV1, NvencGpuGenerationV1, NvencPolicyPositionV1,
    NvencProviderKindV1, NvencResourceEventV1, NvencSourceEvidenceV1, NvencTupleAttemptV1,
    NvencTupleV1, NvencTuplesEvidenceV1,
};

pub trait NvencProvider {
    fn kind(&self) -> NvencProviderKindV1;

    fn source(&self) -> &NvencSourceEvidenceV1;

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1;
}

#[derive(Debug, Clone)]
pub struct LiveUnavailableNvencProvider {
    source: NvencSourceEvidenceV1,
}

impl LiveUnavailableNvencProvider {
    pub fn new(api_version: NvencApiVersionV1) -> Self {
        Self {
            source: NvencSourceEvidenceV1::unavailable(api_version),
        }
    }
}

impl NvencProvider for LiveUnavailableNvencProvider {
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
        NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticNvencProvider {
    source: NvencSourceEvidenceV1,
}

impl DiagnosticNvencProvider {
    pub fn new(api_version: NvencApiVersionV1) -> Self {
        Self {
            source: NvencSourceEvidenceV1::diagnostic(api_version),
        }
    }
}

impl NvencProvider for DiagnosticNvencProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        NvencProviderKindV1::DiagnosticFixture
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
    }
}

pub const fn nvenc_policy_positions()
-> &'static [NvencPolicyPositionV1; NVENC_POLICY_POSITION_COUNT_V1] {
    &NvencPolicyPositionV1::ALL
}

pub fn evaluate_nvenc_policy<P: NvencProvider>(
    generation: NvencGpuGenerationV1,
    provider: &mut P,
) -> NvencTuplesEvidenceV1 {
    let provider_kind = provider.kind();
    let source = provider.source().clone();
    let mut attempts = Vec::with_capacity(NVENC_POLICY_POSITION_COUNT_V1);

    for position in NvencPolicyPositionV1::ALL {
        let tuple = position.tuple();
        let attempt = if position.requires_ada_or_newer() && !generation.supports_av1_encode() {
            NvencTupleAttemptV1::generation_ineligible(position, tuple, source.api_version)
        } else {
            provider.attempt(position, tuple)
        };
        attempts.push(attempt);
    }

    let mut evidence = NvencTuplesEvidenceV1 {
        schema: NVENC_TUPLES_SCHEMA_V1.to_owned(),
        provider: provider_kind,
        source,
        admission: NvencAdmissionV1::Unproven,
        gpu_generation: generation,
        attempts,
        advertised: Vec::new(),
    };

    if evidence.provider == NvencProviderKindV1::SourceAuthenticated {
        evidence.advertised = evidence
            .attempts
            .iter()
            .filter_map(advertisement_from_attempt)
            .collect();
        evidence.admission = if evidence.advertised.is_empty() {
            NvencAdmissionV1::Rejected
        } else {
            NvencAdmissionV1::Pass
        };
    }
    evidence
}

pub fn advertisement_from_attempt(attempt: &NvencTupleAttemptV1) -> Option<NvencAdvertisementV1> {
    if !attempt.terminal
        || !attempt.provider_invoked
        || attempt.outcome != NvencAttemptOutcomeV1::Success
        || !valid_copy_proof(attempt.copy_proof.as_ref()?)
        || !valid_cleanup(&attempt.cleanup)
    {
        return None;
    }

    let stream = attempt.stream_proof.as_ref()?;
    if !stream.keyframe
        || stream.byte_len == 0
        || stream.parsed_tuple != attempt.tuple
        || attempt.position.tuple() != attempt.tuple
    {
        return None;
    }

    Some(NvencAdvertisementV1 {
        position: attempt.position,
        tuple: attempt.tuple,
        bitstream_sha256: stream.bitstream_sha256,
    })
}

pub fn validate_nvenc_tuples_evidence(evidence: &NvencTuplesEvidenceV1) -> bool {
    if evidence.schema != NVENC_TUPLES_SCHEMA_V1
        || evidence.attempts.len() != NVENC_POLICY_POSITION_COUNT_V1
        || evidence.advertised.len() > NVENC_POLICY_POSITION_COUNT_V1
        || !valid_source(evidence)
    {
        return false;
    }

    for (expected, attempt) in NvencPolicyPositionV1::ALL.iter().zip(&evidence.attempts) {
        if attempt.position != *expected
            || attempt.tuple != expected.tuple()
            || attempt.api_version != evidence.source.api_version
            || !valid_attempt(evidence.gpu_generation, attempt)
        {
            return false;
        }
    }

    let derived = evidence
        .attempts
        .iter()
        .filter_map(advertisement_from_attempt)
        .collect::<Vec<_>>();

    match evidence.provider {
        NvencProviderKindV1::DiagnosticFixture | NvencProviderKindV1::LiveUnavailable => {
            evidence.admission == NvencAdmissionV1::Unproven && evidence.advertised.is_empty()
        }
        NvencProviderKindV1::SourceAuthenticated => {
            if !evidence.source.api_version.is_sdk_13_1() {
                return false;
            }
            match evidence.admission {
                NvencAdmissionV1::Pass => !derived.is_empty() && evidence.advertised == derived,
                NvencAdmissionV1::Rejected => derived.is_empty() && evidence.advertised.is_empty(),
                NvencAdmissionV1::Unproven => false,
            }
        }
    }
}

fn valid_source(evidence: &NvencTuplesEvidenceV1) -> bool {
    let source = &evidence.source;
    if source.api_version.major == 0 {
        return false;
    }
    match evidence.provider {
        NvencProviderKindV1::DiagnosticFixture | NvencProviderKindV1::LiveUnavailable => {
            source.source_identity.is_none()
                && source.header_sha256.is_none()
                && source.runtime_library.is_none()
        }
        NvencProviderKindV1::SourceAuthenticated => {
            source
                .source_identity
                .as_deref()
                .is_some_and(valid_source_identifier)
                && source.header_sha256.is_some()
                && source
                    .runtime_library
                    .as_deref()
                    .is_some_and(valid_runtime_library_identifier)
        }
    }
}

fn valid_attempt(generation: NvencGpuGenerationV1, attempt: &NvencTupleAttemptV1) -> bool {
    if !attempt.terminal
        || attempt.cleanup.acquired.len() > MAX_NVENC_RESOURCE_EVENTS_V1
        || attempt.cleanup.released.len() > MAX_NVENC_RESOURCE_EVENTS_V1
    {
        return false;
    }

    if attempt.position.requires_ada_or_newer() && !generation.supports_av1_encode() {
        return !attempt.provider_invoked
            && attempt.outcome == NvencAttemptOutcomeV1::GenerationIneligible
            && attempt.copy_proof.is_none()
            && attempt.stream_proof.is_none()
            && valid_cleanup(&attempt.cleanup);
    }

    match attempt.outcome {
        NvencAttemptOutcomeV1::Success => advertisement_from_attempt(attempt).is_some(),
        NvencAttemptOutcomeV1::GenerationIneligible => false,
        NvencAttemptOutcomeV1::Unsupported
        | NvencAttemptOutcomeV1::ProviderUnavailable
        | NvencAttemptOutcomeV1::Rejected
        | NvencAttemptOutcomeV1::Timeout
        | NvencAttemptOutcomeV1::InvalidStream => {
            attempt.provider_invoked
                && attempt.stream_proof.is_none()
                && valid_cleanup(&attempt.cleanup)
                && attempt
                    .copy_proof
                    .as_ref()
                    .is_none_or(|proof| proof.status == CopyBoundaryStatusV1::BlockedUnknown)
        }
    }
}

fn valid_copy_proof(proof: &CopyBoundaryProofV1) -> bool {
    proof.application_edges.len() <= MAX_NVENC_COPY_EDGES_V1
        && proof.encoder_internal_edges.len() <= MAX_NVENC_COPY_EDGES_V1
        && proof.status == CopyBoundaryStatusV1::Pass
        && proof.application_edges
            == [
                NvencCopyEdgeV1::RegisterNoCopy,
                NvencCopyEdgeV1::MapNoCopy,
                NvencCopyEdgeV1::SubmitNoCopy,
            ]
        && proof.encoder_internal_edges == [NvencCopyEdgeV1::SameGpuPitchLinearToBlockLinearCopy]
        && proof.host_staging_edges == 0
        && proof.cross_gpu_edges == 0
        && proof.unknown_application_edges == 0
        && proof.unknown_encoder_internal_edges == 0
}

fn valid_cleanup(cleanup: &NvencCleanupProofV1) -> bool {
    cleanup.complete
        && cleanup.acquired.len() <= MAX_NVENC_RESOURCE_EVENTS_V1
        && cleanup.released.len() <= MAX_NVENC_RESOURCE_EVENTS_V1
        && cleanup.released
            == cleanup
                .acquired
                .iter()
                .rev()
                .copied()
                .collect::<Vec<NvencResourceEventV1>>()
}

fn valid_source_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn valid_runtime_library_identifier(value: &str) -> bool {
    valid_source_identifier(value)
        && !value.contains("..")
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}
