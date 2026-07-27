use replay_host_doctor::decode_g0_evidence;
use replay_host_doctor::model::{G0EvidenceProvenanceV1, G0ExtensionStatusV1, G0GateStatusV1};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_replay-host-doctor")
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host01-edge-cases.json")
}

fn temp_dir(label: &str) -> PathBuf {
    let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "replay-host-doctor-process-{label}-{}-{timestamp}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&path).expect("test directory must be creatable");
    path
}

fn diagnose(case: &str, evidence: &Path, timeout_ms: u64) -> Output {
    Command::new(binary())
        .args([
            "diagnose",
            "--fixture",
            fixture().to_str().expect("fixture path must be UTF-8"),
            "--fixture-case",
            case,
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--probe-timeout-ms",
            &timeout_ms.to_string(),
        ])
        .output()
        .expect("doctor process must launch")
}

fn read_envelope(path: &Path) -> replay_host_doctor::G0EvidenceEnvelopeV1 {
    decode_g0_evidence(&std::fs::read(path).expect("evidence must be readable"))
        .expect("evidence must decode")
        .into_v1()
}

#[test]
fn bounded_probe_timeout_is_killed_and_returns_promptly() {
    let directory = temp_dir("timeout");
    let evidence = directory.join("evidence.json");
    let started = Instant::now();
    let output = diagnose("timeout", &evidence, 40);
    assert_eq!(output.status.code(), Some(2));
    assert!(started.elapsed() < Duration::from_secs(2));
    let bytes = std::fs::read_to_string(&evidence).expect("timeout evidence must exist");
    assert!(bytes.contains("worker-timeout"));
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn bounded_probe_malformed_oversize_crash_and_duplicate_fail_closed() {
    for case in [
        "malformed-response",
        "oversize-output",
        "oversize-stderr",
        "child-crash",
        "duplicate-response",
    ] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = diagnose(case, &evidence, 200);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let envelope = read_envelope(&evidence);
        assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
        assert!(
            envelope
                .extensions
                .iter()
                .all(|record| record.status == G0ExtensionStatusV1::Unproven)
        );
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn bounded_probe_clears_secrets_and_redacts_child_stderr() {
    let secret = "TOP_SECRET_HOST_DOCTOR_SENTINEL_7f129d";
    for (case, forbidden) in [
        ("secret-sentinel", secret),
        (
            "path-sentinel",
            "/private/operator/diagnostic/native-driver-secret-path",
        ),
        (
            "native-error-sentinel",
            "NVML_PRIVATE_NATIVE_ERROR_0xDEADBEEF",
        ),
    ] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = Command::new(binary())
            .args([
                "diagnose",
                "--fixture",
                fixture().to_str().expect("fixture path must be UTF-8"),
                "--fixture-case",
                case,
                "--evidence",
                evidence.to_str().expect("evidence path must be UTF-8"),
                "--probe-timeout-ms",
                "500",
            ])
            .env("REPLAY_HOST_DOCTOR_SECRET_SENTINEL", secret)
            .output()
            .expect("doctor process must launch");
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let public = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
        let diagnostics = String::from_utf8(output.stderr).expect("stderr must be UTF-8");
        let persisted = std::fs::read_to_string(&evidence).expect("evidence must be UTF-8");
        assert!(!public.contains(forbidden), "case {case}");
        assert!(!diagnostics.contains(forbidden), "case {case}");
        assert!(!persisted.contains(forbidden), "case {case}");
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn fixture_admission_positive_is_diagnostic_and_never_passes() {
    let directory = temp_dir("positive");
    let evidence = directory.join("evidence.json");
    let output = diagnose("positive", &evidence, 500);
    assert_eq!(output.status.code(), Some(2));
    let envelope = read_envelope(&evidence);
    assert_eq!(envelope.base.provenance, G0EvidenceProvenanceV1::Diagnostic);
    assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
    assert!(
        envelope
            .extensions
            .iter()
            .all(|record| record.status == G0ExtensionStatusV1::Unproven)
    );
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn fixture_admission_empty_permuted_duplicate_and_unicode_are_bounded() {
    for case in ["empty", "permuted", "duplicate-observations", "unicode"] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = diagnose(case, &evidence, 500);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let envelope = read_envelope(&evidence);
        assert_eq!(envelope.base.provenance, G0EvidenceProvenanceV1::Diagnostic);
        assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
        assert!(
            envelope
                .extensions
                .iter()
                .all(|record| record.payload.get().len() < 4096)
        );
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn fixture_admission_rejects_identity_verdict_digest_and_cache_claims() {
    for case in [
        "injected-pass",
        "injected-currentness",
        "injected-digest",
        "injected-run-id",
        "stale-cache",
    ] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = diagnose(case, &evidence, 500);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let envelope = read_envelope(&evidence);
        assert_eq!(envelope.base.provenance, G0EvidenceProvenanceV1::Diagnostic);
        assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
        let bytes = std::fs::read_to_string(&evidence).expect("evidence must be UTF-8");
        assert!(!bytes.contains("fixture-owned-run"));
        assert!(!bytes.contains("stale-boot"));
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn currentness_repeated_diagnostic_runs_have_unique_parent_identities() {
    let directory = temp_dir("repeat");
    let evidence = directory.join("evidence.json");
    let first = diagnose("positive", &evidence, 500);
    assert_eq!(first.status.code(), Some(2));
    let first_id = read_envelope(&evidence).base.run_id;
    let second = diagnose("positive", &evidence, 500);
    assert_eq!(second.status.code(), Some(2));
    let second_id = read_envelope(&evidence).base.run_id;
    assert_ne!(first_id, second_id);

    let verify_old = Command::new(binary())
        .args([
            "verify-evidence",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--run-id",
            &first_id,
        ])
        .output()
        .expect("verify process must launch");
    assert_eq!(verify_old.status.code(), Some(74));
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}
