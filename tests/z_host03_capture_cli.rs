// Named last so the legacy timeout-sensitive process suite runs before this
// additional end-to-end HOST-03 binary under Cargo's default test ordering.
use replay_host_doctor::model::{G0EvidenceProvenanceV1, G0ExtensionStatusV1, G0GateStatusV1};
use replay_host_doctor::{G0EvidenceEnvelopeV1, G0ExtensionRecordV1, decode_g0_evidence};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_replay-host-doctor")
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host03-nvfbc-capture.json")
}

fn temp_dir(label: &str) -> PathBuf {
    let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "replay-host03-process-{label}-{}-{timestamp}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&path).expect("test directory must be creatable");
    path
}

fn diagnose(case: &str, evidence: &Path) -> Output {
    Command::new(binary())
        .args([
            "diagnose",
            "--fixture",
            fixture().to_str().expect("fixture path must be UTF-8"),
            "--fixture-case",
            case,
            "--output",
            "DP-0.3",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--probe-timeout-ms",
            "500",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("doctor process must launch")
}

fn envelope(path: &Path) -> G0EvidenceEnvelopeV1 {
    decode_g0_evidence(&std::fs::read(path).expect("evidence must be readable"))
        .expect("evidence must strictly decode")
        .into_v1()
}

fn extension<'a>(envelope: &'a G0EvidenceEnvelopeV1, identifier: &str) -> &'a G0ExtensionRecordV1 {
    envelope
        .extensions
        .iter()
        .find(|extension| extension.id == identifier)
        .expect("known extension")
}

fn payload(record: &G0ExtensionRecordV1) -> Value {
    serde_json::from_str(record.payload.get()).expect("extension payload must be JSON")
}

#[test]
fn host03_fixture_tracer_process_and_rejection_cross_full_binary() {
    let directory = temp_dir("tracer");
    let valid_path = directory.join("valid.json");
    let valid = diagnose("valid-zero-copy", &valid_path);
    assert_eq!(valid.status.code(), Some(2));
    assert!(valid.stderr.is_empty());
    let valid_envelope = envelope(&valid_path);
    assert_eq!(
        valid_envelope.base.provenance,
        G0EvidenceProvenanceV1::Diagnostic
    );
    assert_eq!(valid_envelope.base.status, G0GateStatusV1::Fail);
    assert_eq!(
        extension(&valid_envelope, "selected-output.v1").status,
        G0ExtensionStatusV1::Pass
    );
    let valid_capture = extension(&valid_envelope, "nvfbc-capture.v1");
    assert_eq!(valid_capture.status, G0ExtensionStatusV1::Unproven);
    assert!(replay_host_doctor::evidence::validate_nvfbc_capture_record(
        valid_capture
    ));
    let valid_payload = payload(valid_capture);
    assert_eq!(valid_payload["admission"], "unproven");
    assert_eq!(valid_payload["nvenc_boundary"], "unproven");

    let rejected_path = directory.join("host-staged.json");
    let rejected = diagnose("host-staged", &rejected_path);
    assert_eq!(rejected.status.code(), Some(2));
    assert!(rejected.stderr.is_empty());
    let rejected_envelope = envelope(&rejected_path);
    assert_eq!(rejected_envelope.base.status, G0GateStatusV1::Fail);
    assert_eq!(
        extension(&rejected_envelope, "selected-output.v1").status,
        G0ExtensionStatusV1::Pass
    );
    let rejected_capture = extension(&rejected_envelope, "nvfbc-capture.v1");
    assert_ne!(rejected_capture.status, G0ExtensionStatusV1::Pass);
    let rejected_payload = payload(rejected_capture);
    assert_eq!(rejected_payload["admission"], "rejected");
    assert_eq!(rejected_payload["failure"], "INVALID_COPY_LEDGER");
    assert!(rejected_payload["lease"].is_null());
    assert!(rejected_payload["copy_ledger"].is_null());

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}
