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

fn current_host_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/current-wayland-driver-mismatch.json")
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

fn copied_executable(directory: &Path, label: &str) -> PathBuf {
    let path = directory.join(label);
    std::fs::copy(binary(), &path).expect("doctor executable must be copyable");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
        .expect("copied executable mode must be settable");
    path
}

fn run_with_executable(executable: &Path, evidence: &Path) -> Output {
    Command::new(executable)
        .args([
            "run",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--probe-timeout-ms",
            "1000",
        ])
        .output()
        .expect("copied doctor process must launch")
}

fn archive_with_executable(executable: &Path, evidence: &Path, archive_root: &Path) -> Output {
    Command::new(executable)
        .args([
            "archive-pre-reboot",
            "--evidence",
            evidence.to_str().expect("evidence path must be UTF-8"),
            "--archive-root",
            archive_root
                .to_str()
                .expect("archive root path must be UTF-8"),
        ])
        .output()
        .expect("archive process must launch")
}

fn verify_archive(index: &Path) -> Output {
    Command::new(binary())
        .args([
            "verify-archive",
            "--index",
            index.to_str().expect("index path must be UTF-8"),
        ])
        .output()
        .expect("archive verification process must launch")
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("JSON file must be readable"))
        .expect("file must contain JSON")
}

fn archived_manifest_path(archive_root: &Path) -> PathBuf {
    let index = read_json(&archive_root.join("index.json"));
    let relative = index["manifest_path"]
        .as_str()
        .expect("index must carry a relative manifest path");
    assert!(!Path::new(relative).is_absolute());
    archive_root.join(relative)
}

fn rewrite_manifest_and_index(
    archive_root: &Path,
    mutate: impl FnOnce(&mut Value, &Path),
) -> PathBuf {
    let index_path = archive_root.join("index.json");
    let mut index = read_json(&index_path);
    let manifest_path = archived_manifest_path(archive_root);
    let mut manifest = read_json(&manifest_path);
    mutate(&mut manifest, &manifest_path);
    let manifest_bytes = serde_json::to_vec(&manifest).expect("manifest must serialize");
    std::fs::set_permissions(&manifest_path, std::fs::Permissions::from_mode(0o600))
        .expect("manifest must become writable for adversarial test");
    std::fs::write(&manifest_path, &manifest_bytes).expect("manifest mutation must write");
    std::fs::set_permissions(&manifest_path, std::fs::Permissions::from_mode(0o400))
        .expect("manifest mode must be restored");
    index["manifest_sha256"] = json!(replay_host_doctor::sha256_bytes(&manifest_bytes));
    let index_bytes = serde_json::to_vec(&index).expect("index must serialize");
    std::fs::set_permissions(&index_path, std::fs::Permissions::from_mode(0o600))
        .expect("index must become writable for adversarial test");
    std::fs::write(&index_path, index_bytes).expect("index mutation must write");
    std::fs::set_permissions(&index_path, std::fs::Permissions::from_mode(0o400))
        .expect("index mode must be restored");
    manifest_path
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
fn archive_proc_self_exe_magic_link_source_open() {
    let directory = temp_dir("archive-proc-self-exe");
    let executable = copied_executable(&directory, "running-doctor");
    let executable_digest =
        replay_host_doctor::sha256_file(&executable).expect("copied executable must hash");
    let evidence = directory.join("fresh-live.json");
    let run = run_with_executable(&executable, &evidence);
    assert_eq!(run.status.code(), Some(2));
    let run_id = stdout_json(&run)["run_id"]
        .as_str()
        .expect("live result must expose a run ID")
        .to_owned();

    let archive_root = directory.join("nested/pre-reboot");
    let archived = archive_with_executable(&executable, &evidence, &archive_root);
    assert_eq!(archived.status.code(), Some(0));
    let result = stdout_json(&archived);
    assert_eq!(result["command"], "archive-pre-reboot");
    assert_eq!(result["run_id"], run_id);

    let index_path = archive_root.join("index.json");
    let manifest_path = archived_manifest_path(&archive_root);
    let manifest = read_json(&manifest_path);
    assert_eq!(
        manifest["schema"],
        "replaydesktop.g0-pre-reboot-archive-manifest.v1"
    );
    assert_eq!(
        manifest["source_executable"]["kind"],
        "proc-self-exe-magic-link"
    );
    assert_eq!(manifest["source_executable"]["file_type"], "regular");
    assert_eq!(
        manifest["source_executable"]["size_bytes"],
        std::fs::metadata(&executable)
            .expect("copied executable metadata")
            .len()
    );
    for field in ["mode", "device", "inode", "size_bytes"] {
        assert!(
            manifest["source_executable"][field].as_u64().is_some(),
            "source descriptor metadata missing {field}"
        );
    }

    let run_directory = manifest_path.parent().expect("manifest must have a parent");
    let archived_binary = run_directory.join(
        manifest["archived_binary"]["path"]
            .as_str()
            .expect("binary path"),
    );
    assert_eq!(
        replay_host_doctor::sha256_file(&archived_binary).expect("contained executable must hash"),
        executable_digest
    );
    assert_eq!(
        std::fs::metadata(&archived_binary)
            .expect("archived executable metadata")
            .permissions()
            .mode()
            & 0o777,
        0o500
    );
    assert_eq!(
        std::fs::metadata(run_directory.join("g0-evidence.json"))
            .expect("archived evidence metadata")
            .permissions()
            .mode()
            & 0o777,
        0o400
    );

    std::fs::write(&executable, b"replacement build output")
        .expect("future build replacement must be writable");
    let verified = verify_archive(&index_path);
    assert_eq!(verified.status.code(), Some(0));
    assert_eq!(stdout_json(&verified)["status"], "verified");

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn archive_create_once_rejects_collision_and_symlink_root() {
    let directory = temp_dir("archive-create-once");
    let executable = copied_executable(&directory, "running-doctor");
    let evidence = directory.join("fresh-live.json");
    assert_eq!(
        run_with_executable(&executable, &evidence).status.code(),
        Some(2)
    );
    let archive_root = directory.join("pre-reboot");
    assert_eq!(
        archive_with_executable(&executable, &evidence, &archive_root)
            .status
            .code(),
        Some(0)
    );
    let index_before =
        std::fs::read(archive_root.join("index.json")).expect("index must be readable");
    let manifest_path = archived_manifest_path(&archive_root);
    let manifest_before = std::fs::read(&manifest_path).expect("manifest must be readable");

    assert_private_error(
        &archive_with_executable(&executable, &evidence, &archive_root),
        74,
        "ARCHIVE_PERSISTENCE",
    );
    assert_eq!(
        std::fs::read(archive_root.join("index.json")).expect("index must remain readable"),
        index_before
    );
    assert_eq!(
        std::fs::read(&manifest_path).expect("manifest must remain readable"),
        manifest_before
    );

    let real_root = directory.join("real-root");
    std::fs::create_dir(&real_root).expect("real root must be creatable");
    let symlink_root = directory.join("symlink-root");
    symlink(&real_root, &symlink_root).expect("root symlink must be creatable");
    assert_private_error(
        &archive_with_executable(&executable, &evidence, &symlink_root),
        74,
        "ARCHIVE_PERSISTENCE",
    );
    assert!(
        std::fs::read_dir(&real_root)
            .expect("real root must remain readable")
            .next()
            .is_none()
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn archive_archived_binary_tamper_and_path_escape_fail_closed() {
    let directory = temp_dir("archive-adversarial");
    let executable = copied_executable(&directory, "running-doctor");
    let evidence = directory.join("fresh-live.json");
    assert_eq!(
        run_with_executable(&executable, &evidence).status.code(),
        Some(2)
    );

    let tamper_root = directory.join("tamper-root");
    assert_eq!(
        archive_with_executable(&executable, &evidence, &tamper_root)
            .status
            .code(),
        Some(0)
    );
    let tamper_manifest_path = archived_manifest_path(&tamper_root);
    let tamper_manifest = read_json(&tamper_manifest_path);
    let archived_binary = tamper_manifest_path.parent().unwrap().join(
        tamper_manifest["archived_binary"]["path"]
            .as_str()
            .expect("archived binary path"),
    );
    let mut bytes = std::fs::read(&archived_binary).expect("archived binary must be readable");
    bytes[0] ^= 0xff;
    std::fs::set_permissions(&archived_binary, std::fs::Permissions::from_mode(0o700))
        .expect("archived binary must become writable for adversarial test");
    std::fs::write(&archived_binary, bytes).expect("archived binary mutation must write");
    std::fs::set_permissions(&archived_binary, std::fs::Permissions::from_mode(0o500))
        .expect("archived binary mode must be restored");
    assert_private_error(
        &verify_archive(&tamper_root.join("index.json")),
        74,
        "ARCHIVE_PERSISTENCE",
    );

    let escape_root = directory.join("escape-root");
    assert_eq!(
        archive_with_executable(&executable, &evidence, &escape_root)
            .status
            .code(),
        Some(0)
    );
    let outside = directory.join("outside-sentinel");
    std::fs::write(&outside, b"must not be read as archived binary")
        .expect("outside sentinel must write");
    rewrite_manifest_and_index(&escape_root, |manifest, _| {
        manifest["archived_binary"]["path"] = json!("../outside-sentinel");
    });
    assert_private_error(
        &verify_archive(&escape_root.join("index.json")),
        74,
        "ARCHIVE_PERSISTENCE",
    );
    assert_eq!(
        std::fs::read(&outside).expect("outside sentinel must remain readable"),
        b"must not be read as archived binary"
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn archive_v1_compat_rejects_malformed_evidence_after_outer_digests_match() {
    let directory = temp_dir("archive-malformed-v1");
    let executable = copied_executable(&directory, "running-doctor");
    let evidence = directory.join("fresh-live.json");
    assert_eq!(
        run_with_executable(&executable, &evidence).status.code(),
        Some(2)
    );
    let archive_root = directory.join("pre-reboot");
    assert_eq!(
        archive_with_executable(&executable, &evidence, &archive_root)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        verify_archive(&archive_root.join("index.json"))
            .status
            .code(),
        Some(0)
    );

    rewrite_manifest_and_index(&archive_root, |manifest, manifest_path| {
        let archived_evidence = manifest_path.parent().unwrap().join(
            manifest["evidence"]["path"]
                .as_str()
                .expect("archived evidence path"),
        );
        let malformed = b"{\"schema\":\"replaydesktop.g0-evidence-envelope\",\"version\":1";
        std::fs::set_permissions(&archived_evidence, std::fs::Permissions::from_mode(0o600))
            .expect("evidence must become writable for adversarial test");
        std::fs::write(&archived_evidence, malformed).expect("malformed evidence must write");
        std::fs::set_permissions(&archived_evidence, std::fs::Permissions::from_mode(0o400))
            .expect("evidence mode must be restored");
        manifest["evidence"]["size_bytes"] = json!(malformed.len());
        manifest["evidence"]["sha256"] = json!(replay_host_doctor::sha256_bytes(malformed));
    });
    assert_private_error(
        &verify_archive(&archive_root.join("index.json")),
        74,
        "ARCHIVE_PERSISTENCE",
    );

    let diagnostic = directory.join("diagnostic.json");
    assert_eq!(
        diagnose_with_fixture(
            &current_host_fixture(),
            "current-wayland-driver-mismatch",
            &diagnostic,
            500,
        )
        .status
        .code(),
        Some(2)
    );
    assert_private_error(
        &archive_with_executable(&executable, &diagnostic, &directory.join("diagnostic-root")),
        74,
        "ARCHIVE_PERSISTENCE",
    );

    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
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
    assert!(
        matches!(reasons[0]["status"].as_str(), Some("fail" | "unproven")),
        "host foundation must never pass before authenticated NVML evidence"
    );
    assert!(
        reasons[1..]
            .iter()
            .all(|reason| reason["status"] == "unproven")
    );

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

#[test]
fn host01_native_current_wayland_driver_mismatch_is_specific_and_deterministic() {
    let directory = temp_dir("current-wayland-driver-mismatch");
    let evidence = directory.join("evidence.json");
    let output = diagnose_with_fixture(
        &current_host_fixture(),
        "current-wayland-driver-mismatch",
        &evidence,
        500,
    );
    assert_eq!(output.status.code(), Some(2));
    let result = stdout_json(&output);
    assert_eq!(result["status"], "fail");
    let envelope = read_envelope(&evidence);
    assert_eq!(envelope.base.provenance, G0EvidenceProvenanceV1::Diagnostic);
    assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
    let payload = host_foundation_payload(&envelope);
    assert_eq!(payload["local_xorg"]["status"], "fail");
    assert_eq!(payload["local_xorg"]["reason"]["code"], "SESSION_NOT_XORG");
    assert_eq!(payload["nvidia_kernel_version"], "610.43.02");
    assert_eq!(payload["nvml"]["status"], "fail");
    assert_eq!(payload["nvml"]["source_identity"], "nvidia-nvml-api-13");
    assert_eq!(
        payload["nvml"]["source_sha256"],
        "31a26e3ce6f0b98a76cea38a3cf28aa112a20cfb37e9057786c768712d6a487f"
    );
    assert_eq!(payload["nvml"]["kernel_driver_version"], "610.43.02");
    assert_eq!(payload["nvml"]["userspace_driver_version"], "610.43.03");
    assert_eq!(
        payload["reasons"]
            .as_array()
            .expect("foundation reasons")
            .iter()
            .map(|reason| reason["code"].as_str().expect("reason code"))
            .collect::<Vec<_>>(),
        [
            "SESSION_NOT_XORG",
            "NVIDIA_VERSION_MISMATCH",
            "SELECTED_OUTPUT_CORRELATION_UNPROVEN",
        ]
    );
    assert!(
        envelope
            .extensions
            .iter()
            .skip(1)
            .all(|extension| extension.status == G0ExtensionStatusV1::Unproven)
    );
    assert_eq!(
        verify(
            &evidence,
            result["run_id"].as_str().expect("diagnostic run ID")
        )
        .status
        .code(),
        Some(0)
    );
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_native_session_and_nvml_partial_matrix_never_clears_later_blockers() {
    for (case, expected_reason) in [
        ("multiple-sessions", "SESSION_NOT_XORG"),
        ("inactive-session", "SESSION_NOT_XORG"),
        ("missing-peer-credentials", "X11_PEER_CREDENTIALS_INVALID"),
        ("xnest-peer", "X11_PEER_NOT_XORG"),
        ("xvnc-peer", "X11_PEER_NOT_XORG"),
        ("xdummy-peer", "X11_PEER_NOT_XORG"),
        ("malformed-setup", "X11_SETUP_FAILED"),
        ("nvml-source-unavailable", "NVML_SOURCE_UNAVAILABLE"),
        ("nvml-abi-mismatch", "NVML_SOURCE_ABI_MISMATCH"),
        ("nvml-symbol-missing", "NVML_SYMBOL_MISSING"),
        ("nvml-init-failed", "NVML_INITIALIZATION_FAILED"),
        ("nvml-zero-devices", "NVML_NO_DEVICES"),
        ("nvml-shutdown-failed", "NVML_SHUTDOWN_FAILED"),
    ] {
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
        assert_eq!(foundation.status, G0ExtensionStatusV1::Fail, "case {case}");
        let payload = host_foundation_payload(&envelope);
        assert!(
            payload["reasons"]
                .as_array()
                .expect("foundation reasons")
                .iter()
                .any(|reason| reason["code"] == expected_reason),
            "case {case}"
        );
        assert_eq!(
            payload["selected_output_correlation"]["status"], "unproven",
            "case {case}"
        );
        assert!(
            envelope
                .extensions
                .iter()
                .skip(1)
                .all(|extension| extension.status == G0ExtensionStatusV1::Unproven),
            "case {case}"
        );
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn host01_native_complete_foundation_still_cannot_clear_g0() {
    let directory = temp_dir("nvml-complete-matching");
    let evidence = directory.join("evidence.json");
    let output =
        diagnose_with_fixture(&session_fixture(), "nvml-complete-matching", &evidence, 500);
    assert_eq!(output.status.code(), Some(2));
    let envelope = read_envelope(&evidence);
    let foundation = envelope
        .extensions
        .iter()
        .find(|extension| extension.id == "host-foundation.v1")
        .expect("host-foundation extension");
    assert_eq!(foundation.status, G0ExtensionStatusV1::Unproven);
    let payload = host_foundation_payload(&envelope);
    assert_eq!(payload["local_xorg"]["status"], "pass");
    assert_eq!(payload["nvml"]["status"], "pass");
    assert_eq!(payload["selected_output_correlation"]["status"], "unproven");
    assert_eq!(envelope.base.status, G0GateStatusV1::Fail);
    assert_eq!(
        envelope
            .extensions
            .iter()
            .map(|extension| extension.status)
            .collect::<Vec<_>>(),
        [
            G0ExtensionStatusV1::Unproven,
            G0ExtensionStatusV1::Unproven,
            G0ExtensionStatusV1::Unproven,
            G0ExtensionStatusV1::Unproven,
        ]
    );
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}

#[test]
fn host01_native_worker_faults_are_bounded_and_sanitized() {
    for (case, timeout_ms) in [
        ("timeout", 40),
        ("child-crash", 500),
        ("duplicate-response", 500),
        ("path-sentinel", 500),
        ("native-error-sentinel", 500),
        ("injected-pass", 500),
        ("stale-cache", 500),
    ] {
        let directory = temp_dir(case);
        let evidence = directory.join("evidence.json");
        let started = Instant::now();
        let output = diagnose(case, &evidence, timeout_ms);
        assert_eq!(output.status.code(), Some(2), "case {case}");
        assert!(started.elapsed() < Duration::from_secs(2), "case {case}");
        let persisted = std::fs::read_to_string(&evidence).expect("evidence must be UTF-8");
        for forbidden in [
            "/private/operator/diagnostic/native-driver-secret-path",
            "NVML_PRIVATE_NATIVE_ERROR_0xDEADBEEF",
            "fixture-owned-run",
            "stale-boot",
            "0xDEADBEEF",
            "EDID_PRIVATE_SENTINEL",
        ] {
            assert!(!persisted.contains(forbidden), "case {case}: {forbidden}");
        }
        assert_eq!(read_envelope(&evidence).base.status, G0GateStatusV1::Fail);
        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }
}

#[test]
fn host01_native_known_extension_payload_is_strict_at_verify_readback() {
    let directory = temp_dir("strict-known-extension");
    let evidence = directory.join("evidence.json");
    let output = diagnose_with_fixture(
        &current_host_fixture(),
        "current-wayland-driver-mismatch",
        &evidence,
        500,
    );
    assert_eq!(output.status.code(), Some(2));
    let run_id = stdout_json(&output)["run_id"]
        .as_str()
        .expect("run ID")
        .to_owned();
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&evidence).expect("evidence must exist"))
            .expect("evidence must be JSON");
    let extension = document["extensions"]
        .as_array_mut()
        .expect("extensions array")
        .iter_mut()
        .find(|extension| extension["id"] == "host-foundation.v1")
        .expect("host-foundation extension");
    extension["payload"]["injected_status"] = json!("pass");
    extension["payload"]["source_path"] = json!("/private/operator/nvml.h");
    let payload =
        serde_json::to_vec(&extension["payload"]).expect("payload mutation must serialize");
    extension["payload_sha256"] = json!(replay_host_doctor::sha256_bytes(&payload));
    std::fs::write(
        &evidence,
        serde_json::to_vec(&document).expect("document must serialize"),
    )
    .expect("mutated evidence must be writable");

    assert_private_error(&verify(&evidence, &run_id), 74, "EVIDENCE_PERSISTENCE");
    std::fs::remove_dir_all(&directory).expect("test directory must be removable");
}
