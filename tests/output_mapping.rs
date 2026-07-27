use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const SPIKE_PATH: &str =
    ".planning/phases/01-host-readiness-gate/01-OUTPUT-GPU-MAPPING-SPIKE.md";
const FIXTURE_PATH: &str = "tests/fixtures/host02-output-topologies.json";

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_text(relative: &str) -> String {
    fs::read_to_string(repository_path(relative))
        .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"))
}

fn assert_no_raw_edid(value: &Value, path: &str) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                assert!(
                    !matches!(
                        key.as_str(),
                        "edid" | "raw_edid" | "edid_bytes" | "edid_hex"
                    ),
                    "raw EDID field {path}.{key} must never be persisted"
                );
                let child_path = format!("{path}.{key}");
                assert_no_raw_edid(child, &child_path);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                assert_no_raw_edid(child, &format!("{path}[{index}]"));
            }
        }
        _ => {}
    }
}

#[test]
fn host02_mapping_spike_contract_is_explicit_and_fail_closed() {
    let spike = read_text(SPIKE_PATH);

    for required in [
        "BLOCKED_AMBIGUOUS",
        "BLOCKED_CONFLICTING_FACTS",
        "BLOCKED_TOPOLOGY_CHANGED",
        "BLOCKED_UNSUPPORTED_TOPOLOGY",
        "XRandR output XID",
        "DRM connector_id",
        "disjoint namespaces",
        "exactly one XRandR output",
        "exactly one XRandR provider",
        "exactly one enabled DRM connector",
        "exactly one current NVML device",
        "canonical PCI BDF",
        "SHA-256",
        "raw EDID",
        "NV-CONTROL",
        "NOT_REQUIRED",
    ] {
        assert!(
            spike.contains(required),
            "spike is missing required contract marker {required:?}"
        );
    }

    assert!(
        spike.contains("xrandr --listproviders")
            && spike.contains("/sys/class/drm")
            && spike.contains("nvidia-smi"),
        "spike must preserve reproducible collection commands"
    );
}

#[test]
fn host02_mapping_spike_fixture_covers_adversarial_topologies() {
    let fixture: Value =
        serde_json::from_str(&read_text(FIXTURE_PATH)).expect("fixture must be strict JSON");
    assert_eq!(
        fixture["schema"],
        "replaydesktop.host02-output-topologies.v1"
    );

    let cases = fixture["cases"]
        .as_array()
        .expect("fixture cases must be an array");
    let expected_cases = [
        ("namespace-disjoint-unique", "pass"),
        ("identical-displays-ambiguous", "blocked"),
        ("duplicate-edid-missing-metadata", "blocked"),
        ("multiple-providers", "blocked"),
        ("multiple-drm-connectors", "blocked"),
        ("multiple-nvml-matches", "blocked"),
        ("topology-changed", "blocked"),
        ("cloned-output", "blocked"),
        ("mst-topology", "blocked"),
        ("prime-offload-topology", "blocked"),
        ("missing-edid", "blocked"),
        ("conflicting-facts", "blocked"),
    ];

    for (id, status) in expected_cases {
        let case = cases
            .iter()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("missing adversarial fixture case {id}"));
        assert_eq!(case["expected"]["status"], status, "case {id}");
        if status == "blocked" {
            let reason = case["expected"]["reason"]
                .as_str()
                .unwrap_or_else(|| panic!("blocked case {id} needs a stable reason"));
            assert!(
                reason.starts_with("BLOCKED_"),
                "blocked case {id} has non-stable reason {reason:?}"
            );
        }
    }

    assert_no_raw_edid(&fixture, "$");
}

#[test]
fn host02_mapping_spike_current_machine_is_blocked_before_xorg_reboot() {
    let spike = read_text(SPIKE_PATH);

    for required in [
        "BLOCKED_PRE_REBOOT_XORG",
        "XDG_SESSION_TYPE=wayland",
        "Providers: number : 0",
        "NVML driver/library version mismatch",
        "RANDR Emulation",
    ] {
        assert!(
            spike.contains(required),
            "current-machine observation is missing {required:?}"
        );
    }

    assert!(
        !spike.contains("CURRENT_MACHINE_MAPPING=PASS"),
        "a Wayland/Xwayland observation must not be recorded as a successful physical mapping"
    );
}
