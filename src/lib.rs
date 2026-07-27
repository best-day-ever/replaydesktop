pub mod cli;
pub mod currentness;
pub mod digest;
pub mod evidence;
pub mod local_xorg;
pub mod model;
pub mod native_nvml;
pub mod probe;

pub use cli::{DoctorCommand, DoctorExit, DoctorOptions};
pub use currentness::{CurrentnessPolicy, RunIdentityV1, verify_current_run};
pub use digest::{Sha256DigestV1, sha256_bytes, sha256_file, sha256_reader};
pub use evidence::{EvidenceStore, JsonFileEvidenceStore};
pub use local_xorg::{
    HostFoundationEvidenceV1, HostFoundationObservationV1, LocalXorgEvidenceV1, prove_local_xorg,
};
pub use model::{
    DecodedG0Evidence, G0DecodeError, G0EvidenceBaseV1, G0EvidenceEnvelopeV1, G0ExtensionRecordV1,
    decode_g0_evidence,
};
pub use native_nvml::{
    LiveUnavailableNvmlProvider, NativeNvmlProvider, NvmlDeviceObservationV1, NvmlEvidenceV1,
    NvmlObservationV1, NvmlProvider, NvmlRuntimeFailureV1, NvmlSourceFailureV1,
};
pub use probe::{BoundedProbeRunner, FixtureProbeBackend, LiveProbeBackend, ProbeBackend, ProbeId};

use cli::{DoctorOutput, public_usage};
use currentness::CurrentnessError;
use evidence::EvidenceError;
use model::{
    G0_BASE_SCHEMA_V1, G0_ENVELOPE_SCHEMA_V1, G0_ENVELOPE_VERSION_V1, G0EvidenceProvenanceV1,
    G0ExtensionStatusV1, G0GateStatusV1, G0KnownExtensionV1, G0ReasonV1,
    HOST_FOUNDATION_EXTENSION_ID, NVENC_TUPLES_EXTENSION_ID, NVFBC_CAPTURE_EXTENSION_ID,
    SELECTED_OUTPUT_EXTENSION_ID,
};
use serde_json::value::RawValue;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
enum DoctorError {
    Internal,
    Persistence,
}

impl From<CurrentnessError> for DoctorError {
    fn from(_: CurrentnessError) -> Self {
        Self::Internal
    }
}

impl From<EvidenceError> for DoctorError {
    fn from(_: EvidenceError) -> Self {
        Self::Persistence
    }
}

pub fn main_entry(argv: Vec<String>) -> DoctorExit {
    let output = cli::dispatch(argv);
    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }
    output.exit
}

pub(crate) fn execute_doctor(options: DoctorOptions) -> DoctorOutput {
    match execute_doctor_inner(&options) {
        Ok(output) => output,
        Err(DoctorError::Internal) => {
            DoctorOutput::failure(DoctorExit::Internal, "DOCTOR_INTERNAL")
        }
        Err(DoctorError::Persistence) => {
            DoctorOutput::failure(DoctorExit::Persistence, "EVIDENCE_PERSISTENCE")
        }
    }
}

fn execute_doctor_inner(options: &DoctorOptions) -> Result<DoctorOutput, DoctorError> {
    match &options.command {
        DoctorCommand::Run {
            evidence,
            probe_timeout_ms,
        } => {
            let backend = LiveProbeBackend::new(BoundedProbeRunner::current(
                Duration::from_millis(*probe_timeout_ms),
            ));
            execute_fresh_run(
                evidence,
                &options.argv,
                G0EvidenceProvenanceV1::Live,
                "run",
                &backend,
            )
        }
        DoctorCommand::Diagnose {
            fixture,
            fixture_case,
            evidence,
            probe_timeout_ms,
        } => {
            let backend = FixtureProbeBackend::new(
                BoundedProbeRunner::current(Duration::from_millis(*probe_timeout_ms)),
                fixture,
                fixture_case,
            );
            execute_fresh_run(
                evidence,
                &options.argv,
                G0EvidenceProvenanceV1::Diagnostic,
                "diagnose",
                &backend,
            )
        }
        DoctorCommand::VerifyEvidence { evidence, run_id } => {
            let store = JsonFileEvidenceStore::new(evidence);
            let envelope = store.read_exact(run_id)?;
            verify_current_run(&envelope, run_id, &CurrentnessPolicy::default())
                .map_err(|_| DoctorError::Persistence)?;
            let value = serde_json::json!({
                "schema": "replaydesktop.host-doctor-result.v1",
                "command": "verify-evidence",
                "run_id": run_id,
                "status": "verified",
                "evidence_status": gate_status_name(envelope.base.status),
            });
            Ok(DoctorOutput {
                exit: DoctorExit::Success,
                stdout: format!(
                    "{}\n",
                    serde_json::to_string(&value).expect("result JSON must serialize")
                ),
                stderr: String::new(),
            })
        }
        DoctorCommand::ProbeWorker { request_json } => {
            let worker = probe::worker_output(request_json);
            Ok(DoctorOutput {
                exit: if worker.exit_code == 0 {
                    DoctorExit::Success
                } else {
                    DoctorExit::Internal
                },
                stdout: worker.stdout,
                stderr: worker.stderr,
            })
        }
        DoctorCommand::Help => Ok(DoctorOutput {
            exit: DoctorExit::Success,
            stdout: public_usage().to_owned(),
            stderr: String::new(),
        }),
    }
}

fn execute_fresh_run(
    evidence_path: &Path,
    argv: &[String],
    provenance: G0EvidenceProvenanceV1,
    command: &'static str,
    backend: &dyn ProbeBackend,
) -> Result<DoctorOutput, DoctorError> {
    let identity = RunIdentityV1::capture(argv.to_vec())?;
    let outcomes = probe::collect_probes(backend, identity.run_id());
    let extensions = probe_extension_records(&outcomes)?;
    let envelope = evaluate_g0(&identity, provenance, extensions)?;
    let run_id = envelope.base.run_id.clone();
    let store = JsonFileEvidenceStore::new(evidence_path);
    let readback = store.persist_and_readback(&envelope)?;
    verify_current_run(&readback, &run_id, &CurrentnessPolicy::default())
        .map_err(|_| DoctorError::Persistence)?;

    let reasons = readback
        .base
        .reasons
        .iter()
        .map(|reason| {
            serde_json::json!({
                "extension": known_extension_id(reason.extension),
                "status": extension_status_name(reason.status),
            })
        })
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "schema": "replaydesktop.host-doctor-result.v1",
        "command": command,
        "run_id": run_id,
        "provenance": provenance_name(readback.base.provenance),
        "status": gate_status_name(readback.base.status),
        "reasons": reasons,
    });
    Ok(DoctorOutput {
        exit: match readback.base.status {
            G0GateStatusV1::Pass => DoctorExit::Success,
            G0GateStatusV1::Fail => DoctorExit::G0Fail,
        },
        stdout: format!(
            "{}\n",
            serde_json::to_string(&value).expect("result JSON must serialize")
        ),
        stderr: String::new(),
    })
}

#[derive(Debug)]
pub enum EvaluationError {
    Clock,
    Payload,
    DiagnosticPass,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clock => formatter.write_str("cannot finish run clocks"),
            Self::Payload => formatter.write_str("cannot encode extension payload"),
            Self::DiagnosticPass => {
                formatter.write_str("diagnostic observations cannot produce PASS")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}

impl From<EvaluationError> for DoctorError {
    fn from(_: EvaluationError) -> Self {
        Self::Internal
    }
}

pub fn evaluate_g0(
    identity: &RunIdentityV1,
    provenance: G0EvidenceProvenanceV1,
    extensions: Vec<G0ExtensionRecordV1>,
) -> Result<G0EvidenceEnvelopeV1, EvaluationError> {
    let mut reasons = Vec::new();
    for (known, identifier) in [
        (
            G0KnownExtensionV1::HostFoundation,
            HOST_FOUNDATION_EXTENSION_ID,
        ),
        (
            G0KnownExtensionV1::SelectedOutput,
            SELECTED_OUTPUT_EXTENSION_ID,
        ),
        (G0KnownExtensionV1::NvfbcCapture, NVFBC_CAPTURE_EXTENSION_ID),
        (G0KnownExtensionV1::NvencTuples, NVENC_TUPLES_EXTENSION_ID),
    ] {
        let record = extensions
            .iter()
            .find(|extension| extension.id == identifier)
            .ok_or(EvaluationError::Payload)?;
        if record.status != G0ExtensionStatusV1::Pass {
            reasons.push(G0ReasonV1 {
                extension: known,
                status: record.status,
            });
        }
    }
    let status = if reasons.is_empty() {
        G0GateStatusV1::Pass
    } else {
        G0GateStatusV1::Fail
    };
    if provenance == G0EvidenceProvenanceV1::Diagnostic && status == G0GateStatusV1::Pass {
        return Err(EvaluationError::DiagnosticPass);
    }

    let (wall_finished_unix_ns, monotonic_finished_ns) = identity
        .finish_times()
        .map_err(|_| EvaluationError::Clock)?;
    Ok(G0EvidenceEnvelopeV1 {
        schema: G0_ENVELOPE_SCHEMA_V1.to_owned(),
        version: G0_ENVELOPE_VERSION_V1,
        base: G0EvidenceBaseV1 {
            schema: G0_BASE_SCHEMA_V1.to_owned(),
            provenance,
            run_id: identity.run_id().to_owned(),
            boot_id: identity.boot_id().to_owned(),
            session_id: identity.session_id().to_owned(),
            argv: identity.argv().to_vec(),
            executable_sha256: identity.executable_sha256(),
            wall_started_unix_ns: identity.wall_started_unix_ns(),
            wall_finished_unix_ns,
            monotonic_started_ns: identity.monotonic_started_ns(),
            monotonic_finished_ns,
            status,
            reasons,
        },
        extensions,
    })
}

fn probe_extension_records(
    outcomes: &[probe::ProbeOutcome],
) -> Result<Vec<G0ExtensionRecordV1>, DoctorError> {
    outcomes
        .iter()
        .map(|outcome| {
            if outcome.probe == ProbeId::HostFoundation
                && let Some(observation) = outcome
                    .observation
                    .as_ref()
                    .and_then(|observation| observation.host_foundation.as_ref())
            {
                let evidence = local_xorg::evaluate_host_foundation(observation);
                let status = if evidence.has_failure() {
                    G0ExtensionStatusV1::Fail
                } else {
                    G0ExtensionStatusV1::Unproven
                };
                let payload = serde_json::to_value(evidence).map_err(|_| DoctorError::Internal)?;
                return extension_record(HOST_FOUNDATION_EXTENSION_ID, status, payload)
                    .map_err(|_| DoctorError::Internal);
            }
            let (id, schema) = match outcome.probe {
                ProbeId::HostFoundation => (
                    HOST_FOUNDATION_EXTENSION_ID,
                    "replaydesktop.host-foundation-observation.v1",
                ),
                ProbeId::SelectedOutput => (
                    SELECTED_OUTPUT_EXTENSION_ID,
                    "replaydesktop.selected-output-observation.v1",
                ),
                ProbeId::NvfbcCapture => (
                    NVFBC_CAPTURE_EXTENSION_ID,
                    "replaydesktop.nvfbc-capture-observation.v1",
                ),
                ProbeId::NvencTuples => (
                    NVENC_TUPLES_EXTENSION_ID,
                    "replaydesktop.nvenc-tuples-observation.v1",
                ),
            };
            let payload = if let Some(observation) = &outcome.observation {
                serde_json::json!({
                    "schema": schema,
                    "probe": outcome.probe.as_str(),
                    "admission": "unproven",
                    "worker_status": "observed",
                    "primitive_available": observation.available,
                    "observation_class": if observation.available {
                        "primitive-available"
                    } else {
                        "primitive-unavailable"
                    }
                })
            } else {
                let failure = outcome
                    .failure
                    .expect("failed probe outcome must carry a typed failure");
                serde_json::json!({
                    "schema": schema,
                    "probe": outcome.probe.as_str(),
                    "admission": "unproven",
                    "worker_status": "rejected",
                    "reason": failure.code()
                })
            };
            extension_record(id, G0ExtensionStatusV1::Unproven, payload)
                .map_err(|_| DoctorError::Internal)
        })
        .collect()
}

pub(crate) fn extension_record(
    id: &str,
    status: G0ExtensionStatusV1,
    payload: serde_json::Value,
) -> Result<G0ExtensionRecordV1, EvaluationError> {
    let payload = serde_json::to_string(&payload).map_err(|_| EvaluationError::Payload)?;
    let payload_sha256 = sha256_bytes(payload.as_bytes());
    let payload = RawValue::from_string(payload).map_err(|_| EvaluationError::Payload)?;
    Ok(G0ExtensionRecordV1 {
        id: id.to_owned(),
        version: 1,
        status,
        payload,
        payload_sha256,
    })
}

fn known_extension_id(extension: G0KnownExtensionV1) -> &'static str {
    match extension {
        G0KnownExtensionV1::HostFoundation => HOST_FOUNDATION_EXTENSION_ID,
        G0KnownExtensionV1::SelectedOutput => SELECTED_OUTPUT_EXTENSION_ID,
        G0KnownExtensionV1::NvfbcCapture => NVFBC_CAPTURE_EXTENSION_ID,
        G0KnownExtensionV1::NvencTuples => NVENC_TUPLES_EXTENSION_ID,
    }
}

fn provenance_name(provenance: G0EvidenceProvenanceV1) -> &'static str {
    match provenance {
        G0EvidenceProvenanceV1::Live => "live",
        G0EvidenceProvenanceV1::Diagnostic => "diagnostic",
    }
}

fn gate_status_name(status: G0GateStatusV1) -> &'static str {
    match status {
        G0GateStatusV1::Pass => "pass",
        G0GateStatusV1::Fail => "fail",
    }
}

fn extension_status_name(status: G0ExtensionStatusV1) -> &'static str {
    match status {
        G0ExtensionStatusV1::Pass => "pass",
        G0ExtensionStatusV1::Fail => "fail",
        G0ExtensionStatusV1::Unproven => "unproven",
    }
}

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
        .insert(1, duplicate);
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

#[cfg(test)]
fn plan_01_02_temp_path(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "replay-host-doctor-{label}-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
}

#[cfg(test)]
#[test]
fn tracer_path() {
    use crate::cli::{DoctorExit, dispatch};
    use crate::model::{G0EvidenceProvenanceV1, G0GateStatusV1};
    use std::os::unix::fs::PermissionsExt;

    let directory = plan_01_02_temp_path("tracer");
    std::fs::create_dir(&directory).expect("test directory must be creatable");
    let evidence = directory.join("g0-evidence.json");
    let argv = vec![
        "replay-host-doctor".to_owned(),
        "run".to_owned(),
        "--evidence".to_owned(),
        evidence.display().to_string(),
    ];

    let output = dispatch(argv.clone());
    assert_eq!(output.exit, DoctorExit::G0Fail);
    assert!(output.stderr.is_empty());

    let public: serde_json::Value =
        serde_json::from_str(&output.stdout).expect("stdout must be one JSON object");
    assert_eq!(public["schema"], "replaydesktop.host-doctor-result.v1");
    assert_eq!(public["status"], "fail");
    let run_id = public["run_id"]
        .as_str()
        .expect("result must expose its run identity");

    let bytes = std::fs::read(&evidence).expect("live FAIL must still persist evidence");
    let decoded = decode_g0_evidence(&bytes).expect("persisted evidence must decode as V1");
    let envelope = decoded.as_v1();
    assert_eq!(envelope.base.run_id, run_id);
    assert_eq!(envelope.base.argv, argv);
    assert_eq!(envelope.base.provenance, G0EvidenceProvenanceV1::Live);
    assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
    assert_eq!(envelope.base.reasons.len(), 4);
    assert_eq!(
        std::fs::metadata(&evidence)
            .expect("evidence metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    crate::currentness::verify_current_run(
        envelope,
        run_id,
        &crate::currentness::CurrentnessPolicy::default(),
    )
    .expect("fresh exact-run evidence must verify");

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[cfg(test)]
#[test]
fn cli_exit_usage_does_not_write() {
    use crate::cli::{DoctorExit, dispatch};

    let directory = plan_01_02_temp_path("usage");
    std::fs::create_dir(&directory).expect("test directory must be creatable");
    let evidence = directory.join("must-not-exist.json");
    let output = dispatch(vec![
        "replay-host-doctor".to_owned(),
        "run".to_owned(),
        "--evidence".to_owned(),
    ]);

    assert_eq!(output.exit, DoctorExit::Usage);
    assert!(!output.stderr.is_empty());
    assert!(!evidence.exists());
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[cfg(test)]
#[test]
fn evidence_atomic_rejects_symlink_destination() {
    use crate::cli::{DoctorExit, dispatch};
    use std::os::unix::fs::symlink;

    let directory = plan_01_02_temp_path("symlink");
    std::fs::create_dir(&directory).expect("test directory must be creatable");
    let target = directory.join("target");
    let evidence = directory.join("g0-evidence.json");
    std::fs::write(&target, b"do not replace").expect("sentinel target must be writable");
    symlink(&target, &evidence).expect("test symlink must be creatable");

    let output = dispatch(vec![
        "replay-host-doctor".to_owned(),
        "run".to_owned(),
        "--evidence".to_owned(),
        evidence.display().to_string(),
    ]);
    assert_eq!(output.exit, DoctorExit::Persistence);
    assert_eq!(
        std::fs::read(&target).expect("sentinel target must remain"),
        b"do not replace"
    );
    assert!(
        std::fs::symlink_metadata(&evidence)
            .expect("destination symlink must remain")
            .file_type()
            .is_symlink()
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[cfg(test)]
#[test]
fn cli_exit_taxonomy_is_stable() {
    use crate::cli::DoctorExit;

    assert_eq!(DoctorExit::Success.code(), 0);
    assert_eq!(DoctorExit::G0Fail.code(), 2);
    assert_eq!(DoctorExit::Usage.code(), 64);
    assert_eq!(DoctorExit::Internal.code(), 70);
    assert_eq!(DoctorExit::Persistence.code(), 74);
}

#[cfg(test)]
#[test]
fn cli_exit_wrong_run_readback_is_persistence_failure() {
    use crate::cli::{DoctorExit, dispatch};

    let directory = plan_01_02_temp_path("wrong-readback");
    std::fs::create_dir(&directory).expect("test directory must be creatable");
    let evidence = directory.join("g0-evidence.json");
    let run = dispatch(vec![
        "replay-host-doctor".to_owned(),
        "run".to_owned(),
        "--evidence".to_owned(),
        evidence.display().to_string(),
    ]);
    assert_eq!(run.exit, DoctorExit::G0Fail);

    let verify = dispatch(vec![
        "replay-host-doctor".to_owned(),
        "verify-evidence".to_owned(),
        "--evidence".to_owned(),
        evidence.display().to_string(),
        "--run-id".to_owned(),
        "wrong-run".to_owned(),
    ]);
    assert_eq!(verify.exit, DoctorExit::Persistence);

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[cfg(test)]
#[test]
fn currentness_expired_evidence_is_rejected_after_identity_checks() {
    use crate::cli::{DoctorExit, dispatch};
    use crate::currentness::{CurrentnessError, CurrentnessPolicy, verify_current_run};

    let directory = plan_01_02_temp_path("expired");
    std::fs::create_dir(&directory).expect("test directory must be creatable");
    let evidence = directory.join("g0-evidence.json");
    let output = dispatch(vec![
        "replay-host-doctor".to_owned(),
        "run".to_owned(),
        "--evidence".to_owned(),
        evidence.display().to_string(),
    ]);
    assert_eq!(output.exit, DoctorExit::G0Fail);
    let mut envelope = decode_g0_evidence(&std::fs::read(&evidence).expect("evidence must exist"))
        .expect("evidence must decode")
        .into_v1();
    let shift = 10_000_000_000;
    envelope.base.wall_started_unix_ns = envelope.base.wall_started_unix_ns.saturating_sub(shift);
    envelope.base.wall_finished_unix_ns = envelope.base.wall_finished_unix_ns.saturating_sub(shift);
    envelope.base.monotonic_started_ns = envelope.base.monotonic_started_ns.saturating_sub(shift);
    envelope.base.monotonic_finished_ns = envelope.base.monotonic_finished_ns.saturating_sub(shift);
    let run_id = envelope.base.run_id.clone();

    assert!(matches!(
        verify_current_run(
            &envelope,
            &run_id,
            &CurrentnessPolicy {
                max_age_ns: 1_000_000_000,
                ..CurrentnessPolicy::default()
            }
        ),
        Err(CurrentnessError::EvidenceExpired)
    ));

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}
