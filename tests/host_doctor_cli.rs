use replay_host_doctor::decode_g0_evidence;
use replay_host_doctor::model::{G0EvidenceProvenanceV1, G0ExtensionStatusV1, G0GateStatusV1};
use serde_json::{Value, json};
use std::os::unix::fs::{PermissionsExt, symlink};
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

fn session_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host01-session-spoofing.json")
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
    diagnose_with_fixture(&fixture(), case, evidence, timeout_ms)
}

fn diagnose_with_fixture(
    fixture_path: &Path,
    case: &str,
    evidence: &Path,
    timeout_ms: u64,
) -> Output {
    Command::new(binary())
        .args([
            "diagnose",
            "--fixture",
            fixture_path.to_str().expect("fixture path must be UTF-8"),
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

fn run(evidence: &Path, timeout_ms: u64) -> Output {
    Command::new(binary())
        .args([
            "run",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--probe-timeout-ms",
            &timeout_ms.to_string(),
        ])
        .output()
        .expect("doctor process must launch")
}

fn verify(evidence: &Path, run_id: &str) -> Output {
    Command::new(binary())
        .args([
            "verify-evidence",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--run-id",
            run_id,
        ])
        .output()
        .expect("verify process must launch")
}

fn stdout_json(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("stdout must contain one JSON object")
}

fn assert_private_error(output: &Output, exit: i32, code: &str) {
    assert_eq!(output.status.code(), Some(exit));
    assert!(output.stdout.is_empty());
    let error: Value =
        serde_json::from_slice(&output.stderr).expect("stderr must contain one error object");
    assert_eq!(
        error,
        json!({
            "schema": "replaydesktop.host-doctor-error.v1",
            "code": code,
        })
    );
}

fn read_envelope(path: &Path) -> replay_host_doctor::G0EvidenceEnvelopeV1 {
    decode_g0_evidence(&std::fs::read(path).expect("evidence must be readable"))
        .expect("evidence must decode")
        .into_v1()
}

#[test]
fn host01_skeleton_run_writes_mode_0600_and_verifies_the_exact_run() {
    let directory = temp_dir("skeleton-live");
    let evidence = directory.join("evidence.json");

    let output = run(&evidence, 500);
    assert_eq!(output.status.code(), Some(2));
    let result = stdout_json(&output);
    assert_eq!(result["schema"], "replaydesktop.host-doctor-result.v1");
    assert_eq!(result["command"], "run");
    assert_eq!(result["provenance"], "live");
    assert_eq!(result["status"], "fail");
    let reasons = result["reasons"]
        .as_array()
        .expect("reasons must be an array");
    assert_eq!(
        reasons
            .iter()
            .map(|reason| reason["extension"].as_str().expect("extension ID"))
            .collect::<Vec<_>>(),
        [
            "host-foundation.v1",
            "selected-output.v1",
            "nvfbc-capture.v1",
            "nvenc-tuples.v1",
        ]
    );
    assert!(reasons.iter().all(|reason| reason["status"] == "unproven"));

    let mode = std::fs::metadata(&evidence)
        .expect("evidence metadata must exist")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let envelope = read_envelope(&evidence);
    let run_id = result["run_id"].as_str().expect("run ID");
    assert_eq!(envelope.base.run_id, run_id);

    let verified = verify(&evidence, run_id);
    assert_eq!(verified.status.code(), Some(0));
    let verification = stdout_json(&verified);
    assert_eq!(verification["command"], "verify-evidence");
    assert_eq!(verification["status"], "verified");
    assert_eq!(verification["evidence_status"], "fail");

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_skeleton_exercises_every_command_and_stable_exit() {
    let directory = temp_dir("skeleton-exits");

    let help = Command::new(binary())
        .arg("--help")
        .output()
        .expect("help process must launch");
    assert_eq!(help.status.code(), Some(0));
    assert!(help.stderr.is_empty());
    assert!(
        String::from_utf8(help.stdout)
            .expect("help must be UTF-8")
            .contains("replay-host-doctor run")
    );

    let usage = Command::new(binary())
        .arg("run")
        .output()
        .expect("usage process must launch");
    assert_eq!(usage.status.code(), Some(64));
    assert!(usage.stdout.is_empty());
    assert!(
        String::from_utf8(usage.stderr)
            .expect("usage must be UTF-8")
            .starts_with("replay-host-doctor:")
    );

    let missing_parent = run(&directory.join("missing/evidence.json"), 500);
    assert_private_error(&missing_parent, 74, "EVIDENCE_PERSISTENCE");

    let internal = Command::new(binary())
        .args(["__probe-worker", "--request-json", "{}"])
        .output()
        .expect("worker process must launch");
    assert_eq!(internal.status.code(), Some(70));
    assert!(internal.stdout.is_empty());
    let internal_error: Value =
        serde_json::from_slice(&internal.stderr).expect("worker stderr must be one JSON object");
    assert_eq!(
        internal_error,
        json!({
            "schema": "replaydesktop.host-probe-worker-error.v1",
            "code": "worker-request-invalid",
        })
    );

    let diagnostic_path = directory.join("diagnostic.json");
    let diagnostic = diagnose("positive", &diagnostic_path, 500);
    assert_eq!(diagnostic.status.code(), Some(2));
    let diagnostic_result = stdout_json(&diagnostic);
    assert_eq!(diagnostic_result["command"], "diagnose");
    assert_eq!(diagnostic_result["provenance"], "diagnostic");
    assert_eq!(diagnostic_result["status"], "fail");

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_skeleton_rejects_symlinks_and_preserves_their_targets() {
    let directory = temp_dir("skeleton-symlink");
    let target = directory.join("target.json");
    let evidence = directory.join("evidence.json");
    std::fs::write(&target, b"operator-owned-target").expect("target must be writable");
    symlink(&target, &evidence).expect("evidence symlink must be creatable");

    let output = run(&evidence, 500);
    assert_private_error(&output, 74, "EVIDENCE_PERSISTENCE");
    assert_eq!(
        std::fs::read(&target).expect("target must remain readable"),
        b"operator-owned-target"
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_skeleton_timeout_and_oversize_diagnostics_are_bounded() {
    for (case, timeout_ms) in [("timeout", 40), ("oversize-output", 500)] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let started = Instant::now();
        let output = diagnose(case, &evidence, timeout_ms);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        assert!(started.elapsed() < Duration::from_secs(2), "case {case}");
        let result = stdout_json(&output);
        assert_eq!(result["provenance"], "diagnostic");
        assert_eq!(result["status"], "fail");
        assert!(output.stdout.len() < 4096, "case {case}");
        assert!(std::fs::metadata(&evidence).expect("evidence").len() < 64 * 1024);
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn host01_skeleton_fresh_runs_replace_stale_decisions_without_reusing_identity() {
    let directory = temp_dir("skeleton-freshness");
    let evidence = directory.join("evidence.json");

    let first = run(&evidence, 500);
    assert_eq!(first.status.code(), Some(2));
    let first_id = stdout_json(&first)["run_id"]
        .as_str()
        .expect("first run ID")
        .to_owned();

    let second = run(&evidence, 500);
    assert_eq!(second.status.code(), Some(2));
    let second_id = stdout_json(&second)["run_id"]
        .as_str()
        .expect("second run ID")
        .to_owned();
    assert_ne!(first_id, second_id);
    assert_eq!(read_envelope(&evidence).base.run_id, second_id);

    assert_private_error(&verify(&evidence, &first_id), 74, "EVIDENCE_PERSISTENCE");
    assert_eq!(verify(&evidence, &second_id).status.code(), Some(0));

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_skeleton_unknown_extension_survives_strict_readback() {
    let directory = temp_dir("skeleton-unknown-extension");
    let evidence = directory.join("evidence.json");
    let output = run(&evidence, 500);
    assert_eq!(output.status.code(), Some(2));
    let run_id = stdout_json(&output)["run_id"]
        .as_str()
        .expect("run ID")
        .to_owned();

    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&evidence).expect("evidence must exist"))
            .expect("evidence must be JSON");
    let extensions = document["extensions"]
        .as_array_mut()
        .expect("extensions must be an array");
    let mut future = extensions[0].clone();
    let payload = json!({"future_observation":"preserved","values":[1,2,3]});
    let payload_bytes = serde_json::to_vec(&payload).expect("payload must serialize");
    future["id"] = json!("future-observation.v1");
    future["version"] = json!(1);
    future["status"] = json!("unproven");
    future["payload"] = payload;
    future["payload_sha256"] = json!(replay_host_doctor::sha256_bytes(&payload_bytes));
    extensions.push(future);
    std::fs::write(
        &evidence,
        serde_json::to_vec(&document).expect("document must serialize"),
    )
    .expect("mutated evidence must be writable");

    let verified = verify(&evidence, &run_id);
    assert_eq!(verified.status.code(), Some(0));
    let readback = read_envelope(&evidence);
    let unknown = readback
        .extensions
        .iter()
        .find(|extension| extension.id == "future-observation.v1")
        .expect("unknown extension must survive");
    assert_eq!(
        unknown.payload.get(),
        String::from_utf8(payload_bytes).unwrap()
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_skeleton_readme_documents_the_honest_operator_contract() {
    let readme = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("README.md must document the walking skeleton");
    assert!(readme.contains(
        "cargo run --locked --bin replay-host-doctor -- run --evidence target/g0-evidence.json"
    ));
    for required in [
        "verify-evidence",
        "diagnose",
        "exit 2",
        "exit 64",
        "exit 70",
        "exit 74",
        "0600",
        "read-only",
        "Xorg",
        "NVML",
        "output",
        "NvFBC",
        "NVENC",
        "UNPROVEN",
        "cannot produce G0 PASS",
    ] {
        assert!(readme.contains(required), "README missing {required}");
    }
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

fn host_foundation_payload(envelope: &replay_host_doctor::G0EvidenceEnvelopeV1) -> Value {
    let extension = envelope
        .extensions
        .iter()
        .find(|extension| extension.id == "host-foundation.v1")
        .expect("host-foundation.v1 must exist");
    serde_json::from_str(extension.payload.get()).expect("host-foundation payload must be JSON")
}

#[test]
fn local_xorg_joined_predicate_accepts_only_the_complete_shape() {
    let cases = [
        ("local-xorg-shaped", "pass", None),
        ("fake-environment-only", "fail", Some("SESSION_NOT_XORG")),
        ("remote-session", "fail", Some("SESSION_REMOTE")),
        ("tcp-transport", "fail", Some("X11_TRANSPORT_NOT_UNIX")),
        ("wrong-peer", "fail", Some("X11_PEER_CREDENTIALS_INVALID")),
        ("xwayland-peer", "fail", Some("X11_PEER_NOT_XORG")),
        ("nested-xephyr-peer", "fail", Some("X11_PEER_NOT_XORG")),
        ("malformed-randr", "fail", Some("XRANDR_QUERY_FAILED")),
    ];

    for (case, local_xorg_status, local_xorg_reason) in cases {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = diagnose_with_fixture(&session_fixture(), case, &evidence, 500);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let envelope = read_envelope(&evidence);
        assert_eq!(envelope.base.status, G0GateStatusV1::Fail, "case {case}");
        let foundation = envelope
            .extensions
            .iter()
            .find(|extension| extension.id == "host-foundation.v1")
            .expect("host-foundation extension");
        assert_eq!(
            foundation.status,
            if local_xorg_status == "pass" {
                G0ExtensionStatusV1::Unproven
            } else {
                G0ExtensionStatusV1::Fail
            },
            "case {case}"
        );
        let payload = host_foundation_payload(&envelope);
        assert_eq!(
            payload["schema"], "replaydesktop.host-foundation.v1",
            "case {case}"
        );
        assert_eq!(
            payload["local_xorg"]["status"], local_xorg_status,
            "case {case}"
        );
        match local_xorg_reason {
            Some(reason) => assert_eq!(
                payload["local_xorg"]["reason"]["code"], reason,
                "case {case}"
            ),
            None => assert!(payload["local_xorg"]["reason"].is_null(), "case {case}"),
        }
        assert_eq!(payload["nvml"]["status"], "unproven", "case {case}");
        assert_eq!(
            payload["selected_output_correlation"]["status"], "unproven",
            "case {case}"
        );
        assert!(
            envelope.extensions[1..]
                .iter()
                .all(|extension| extension.status == G0ExtensionStatusV1::Unproven),
            "case {case}"
        );
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn host01_source_neutral_device_failures_are_stable_and_actionable() {
    for (case, reason) in [
        ("missing-uinput", "UINPUT_ACCESS_REQUIRED"),
        ("missing-render", "DRM_RENDER_ACCESS_REQUIRED"),
        ("missing-physical-output", "PHYSICAL_OUTPUT_REQUIRED"),
    ] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let output = diagnose_with_fixture(&session_fixture(), case, &evidence, 500);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        let envelope = read_envelope(&evidence);
        let foundation = envelope
            .extensions
            .iter()
            .find(|extension| extension.id == "host-foundation.v1")
            .expect("host-foundation extension");
        assert_eq!(foundation.status, G0ExtensionStatusV1::Fail, "case {case}");
        let payload = host_foundation_payload(&envelope);
        assert_eq!(payload["local_xorg"]["status"], "pass", "case {case}");
        assert!(
            payload["reasons"]
                .as_array()
                .expect("reasons array")
                .iter()
                .any(|entry| entry["code"] == reason
                    && entry["remediation"]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())),
            "case {case}"
        );
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn bounded_probe_session_clears_environment_only_xorg_claims() {
    let directory = temp_dir("session-environment");
    let evidence = directory.join("evidence.json");
    let display_sentinel = "remote.example.invalid:123";
    let output = Command::new(binary())
        .args([
            "run",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--probe-timeout-ms",
            "1000",
        ])
        .env("DISPLAY", display_sentinel)
        .env("XDG_SESSION_TYPE", "x11")
        .output()
        .expect("doctor process must launch");
    assert_eq!(output.status.code(), Some(2));
    let envelope = read_envelope(&evidence);
    assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
    let foundation = host_foundation_payload(&envelope);
    assert_eq!(foundation["schema"], "replaydesktop.host-foundation.v1");
    assert!(matches!(
        foundation["local_xorg"]["status"].as_str(),
        Some("fail" | "unproven")
    ));
    let persisted = std::fs::read_to_string(&evidence).expect("evidence must be UTF-8");
    assert!(!persisted.contains(display_sentinel));
    assert!(!persisted.contains("XDG_SESSION_TYPE"));
    assert!(
        envelope
            .extensions
            .iter()
            .skip(1)
            .all(|extension| extension.status == G0ExtensionStatusV1::Unproven)
    );
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}
