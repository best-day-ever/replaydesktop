use std::fs;
use std::path::{Path, PathBuf};

use replay_host_doctor::{
    DrmConnectorIdV1, OutputMappingReasonV1, OutputTopologyObservationV1, RefreshRateV1,
    SelectedOutputV1, XrandrOutputXidV1, prove_output_gpu_mapping,
};
use serde_json::Value;

const SPIKE_PATH: &str = ".planning/phases/01-host-readiness-gate/01-OUTPUT-GPU-MAPPING-SPIKE.md";
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

fn overlay_json(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, patch_value) in patch {
                if let Some(target_value) = target.get_mut(key) {
                    overlay_json(target_value, patch_value);
                } else {
                    target.insert(key.clone(), patch_value.clone());
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

fn mapping_fixture() -> Value {
    serde_json::from_str(&read_text(FIXTURE_PATH)).expect("mapping fixture must be strict JSON")
}

fn topology_case(case_id: &str) -> (OutputTopologyObservationV1, Value) {
    let fixture = mapping_fixture();
    let case = fixture["cases"]
        .as_array()
        .expect("fixture cases must be an array")
        .iter()
        .find(|case| case["id"] == case_id)
        .unwrap_or_else(|| panic!("missing topology case {case_id}"))
        .clone();
    let mut topology = fixture["base_topology"].clone();
    overlay_json(&mut topology, &case["topology_patch"]);
    (
        serde_json::from_value(topology)
            .unwrap_or_else(|error| panic!("case {case_id} must deserialize: {error}")),
        case,
    )
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
        ("multiple-pci-bdfs", "blocked"),
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

#[test]
fn host02_mapping_fixture_matrix_is_deterministic_and_fail_closed() {
    let fixture = mapping_fixture();
    for case in fixture["cases"].as_array().expect("fixture cases") {
        let case_id = case["id"].as_str().expect("case id");
        let (topology, expected_case) = topology_case(case_id);
        match expected_case["expected"]["status"]
            .as_str()
            .expect("expected status")
        {
            "pass" => {
                let selected = prove_output_gpu_mapping(&topology)
                    .unwrap_or_else(|failure| panic!("case {case_id} blocked: {failure:?}"));
                assert_eq!(selected.schema, "replaydesktop.selected-output.v1");
                assert_eq!(selected.proof.requested_output_matches, 1);
                assert_eq!(selected.proof.provider_matches, 1);
                assert_eq!(selected.proof.drm_connector_matches, 1);
                assert_eq!(selected.proof.canonical_pci_bdf_matches, 1);
                assert_eq!(selected.proof.nvml_device_matches, 1);
            }
            "blocked" => {
                let failure = match prove_output_gpu_mapping(&topology) {
                    Ok(_) => panic!("case {case_id} guessed a selection"),
                    Err(failure) => failure,
                };
                assert_eq!(
                    failure.reason.as_code(),
                    expected_case["expected"]["reason"]
                        .as_str()
                        .expect("expected reason"),
                    "case {case_id}"
                );
                assert_eq!(
                    serde_json::to_value(&failure).expect("failure must encode")["reason"],
                    expected_case["expected"]["reason"],
                    "case {case_id} must persist the stable reason code"
                );
            }
            status => panic!("unknown expected status {status:?}"),
        }
    }
}

#[test]
fn host02_mapping_exact_name_origin_and_refresh_round_trip() {
    let (mut topology, _) = topology_case("namespace-disjoint-unique");
    topology.requested_output_name.hex = "44502dce94".to_owned();
    topology.requested_output_name.display = Some("DP-Δ".to_owned());
    topology.randr_outputs[0].name = topology.requested_output_name.clone();
    topology.randr_outputs[0].origin_x = -7680;
    topology.randr_outputs[0].origin_y = -2160;

    let selected = prove_output_gpu_mapping(&topology).expect("complete relation must pass");
    assert_eq!(selected.output_name.hex, "44502dce94");
    assert_eq!(selected.output_name.display.as_deref(), Some("DP-Δ"));
    assert_eq!(selected.origin_x, -7680);
    assert_eq!(selected.origin_y, -2160);
    assert_eq!(
        selected.refresh_hz,
        RefreshRateV1 {
            numerator: 60,
            denominator: 1,
        }
    );

    let encoded = serde_json::to_vec(&selected).expect("selected output must encode");
    let decoded: SelectedOutputV1 =
        serde_json::from_slice(&encoded).expect("selected output must decode");
    assert_eq!(decoded, selected);
}

#[test]
fn host02_mapping_bounds_and_exact_arithmetic_reject_invalid_observations() {
    let (topology, _) = topology_case("namespace-disjoint-unique");

    let mut oversized = topology.clone();
    oversized.randr_outputs = vec![topology.randr_outputs[0].clone(); 65];
    assert_eq!(
        prove_output_gpu_mapping(&oversized)
            .expect_err("oversized output collection must fail")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );

    let mut invalid_name = topology.clone();
    invalid_name.requested_output_name.hex = "44502D30".to_owned();
    assert_eq!(
        prove_output_gpu_mapping(&invalid_name)
            .expect_err("uppercase output hex must fail")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );

    let mut overflowing_refresh = topology.clone();
    overflowing_refresh.randr_outputs[0].timing.pixel_clock_hz = u64::MAX;
    overflowing_refresh.randr_outputs[0].timing.flags = 16;
    assert_eq!(
        prove_output_gpu_mapping(&overflowing_refresh)
            .expect_err("refresh arithmetic overflow must fail")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );

    let mut unsupported_clock_flags = topology.clone();
    unsupported_clock_flags.randr_outputs[0].timing.flags = 0x1000;
    assert_eq!(
        prove_output_gpu_mapping(&unsupported_clock_flags)
            .expect_err("unimplemented clock modifiers must fail closed")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );

    let mut malformed_bdf = topology;
    malformed_bdf.drm_connectors[0].canonical_pci_bdfs[0] = "00000000:GG:00.0".to_owned();
    assert_eq!(
        prove_output_gpu_mapping(&malformed_bdf)
            .expect_err("malformed canonical PCI BDF must fail")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );
}

#[test]
fn host02_namespace_disjoint_full_relation_passes_without_id_equality() {
    let (topology, _) = topology_case("namespace-disjoint-unique");
    let selected = prove_output_gpu_mapping(&topology).expect("unique full relation must pass");

    let output_xid: XrandrOutputXidV1 = selected.randr_output_xid;
    let connector_id: DrmConnectorIdV1 = selected.drm_connector_id;
    assert_ne!(output_xid.get(), connector_id.get());
    assert_eq!(output_xid.get(), 73);
    assert_eq!(connector_id.get(), 911);
    assert_ne!(
        selected.randr_connector_number,
        Some(selected.drm_connector_type_id)
    );
    assert_ne!(selected.output_name.display.as_deref(), Some("DP-1"));
    assert_eq!(selected.drm_canonical_pci_bdf, selected.nvml_pci_bdf);
}

#[test]
fn host02_topology_token_change_returns_no_selection() {
    let (topology, _) = topology_case("topology-changed");
    let failure =
        prove_output_gpu_mapping(&topology).expect_err("mixed topology generations must fail");
    assert_eq!(failure.reason, OutputMappingReasonV1::TopologyChanged);
    assert_eq!(failure.reason.as_code(), "BLOCKED_TOPOLOGY_CHANGED");
}
