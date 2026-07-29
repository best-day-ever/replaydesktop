use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn project_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn spike_text() -> String {
    fs::read_to_string(project_path(
        ".planning/phases/01-host-readiness-gate/01-NVENC-COPY-BOUNDARY-SPIKE.md",
    ))
    .expect("Plan 01-09 must publish the NVENC copy-boundary spike before native work")
}

fn fixture() -> Value {
    let bytes = fs::read(project_path("tests/fixtures/host04-nvenc-tuples.json"))
        .expect("Plan 01-09 must publish HOST-04 diagnostic fixtures");
    serde_json::from_slice(&bytes).expect("HOST-04 fixture must be valid JSON")
}

#[test]
fn host04_copy_spike_source_matrix_and_graph_are_executable() {
    let spike = spike_text();

    for required in [
        "## Source and observability matrix",
        "## Resource and ownership graph",
        "source-guaranteed",
        "runtime-queryable",
        "optional-read-only-tooling",
        "unknown",
        "CaptureFrameLeaseV1",
        "NvEncRegisterResource",
        "NvEncMapInputResource",
        "NvEncEncodePicture",
        "NvEncLockBitstream",
        "CopyBoundaryProofV1",
    ] {
        assert!(
            spike.contains(required),
            "copy spike is missing executable contract token {required:?}"
        );
    }

    let fixture = fixture();
    assert_eq!(
        fixture["schema"],
        "replaydesktop.host04-nvenc-fixtures.v1"
    );
    assert_eq!(fixture["provenance"], "diagnostic");
    assert!(
        fixture["copy_boundary_cases"]
            .as_array()
            .is_some_and(|cases| !cases.is_empty()),
        "fixture must carry executable copy-boundary cases"
    );
}

#[test]
fn host04_copy_spike_unknown_edges_block() {
    let spike = spike_text();

    for required in [
        "PASS",
        "BLOCKED_UNKNOWN",
        "host staging",
        "cross-GPU",
        "unobserved application edge",
        "encoder-internal conversion",
        "encoder-internal copy",
    ] {
        assert!(
            spike.contains(required),
            "copy predicate is missing blocking condition {required:?}"
        );
    }

    let cases = fixture()["copy_boundary_cases"]
        .as_array()
        .expect("copy_boundary_cases must be an array")
        .clone();
    for id in [
        "closed-device-pointer-path",
        "host-staging-blocked",
        "mismatched-resource-blocked",
        "unknown-application-edge-blocked",
        "unknown-encoder-internal-edge-blocked",
    ] {
        assert!(
            cases.iter().any(|case| case["id"] == id),
            "fixture is missing {id}"
        );
    }
    assert!(
        cases
            .iter()
            .filter(|case| case["id"] != "closed-device-pointer-path")
            .all(|case| case["expected_copy_status"] == "blocked-unknown"),
        "every incomplete or unsafe copy case must stay BLOCKED_UNKNOWN"
    );
}

#[test]
fn host04_copy_spike_sdk_13_1_isolated_from_kyber_12_1() {
    let spike = spike_text();

    for required in [
        "Video Codec SDK 13.1",
        "nv-codec-headers n12.1.14.0",
        "standalone proof",
        "does not modify",
        "Plan 01-10",
        "cargo run --locked -- run",
    ] {
        assert!(
            spike.contains(required),
            "SDK separation or live handoff is missing {required:?}"
        );
    }
}
