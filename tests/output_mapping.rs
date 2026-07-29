use std::fs;
use std::path::{Path, PathBuf};

use replay_host_doctor::{
    NvControlDisplayTargetIdV1, NvControlGpuTargetIdV1, OutputMappingReasonV1,
    OutputTopologyObservationV1, RefreshRateV1, SelectedOutputV1, XrandrOutputXidV1,
    decode_nvcontrol_target_list, prove_output_gpu_mapping,
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
                assert_no_raw_edid(child, &format!("{path}.{key}"));
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

fn target_list_bytes(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((values.len() + 1) * 4);
    bytes.extend_from_slice(
        &u32::try_from(values.len())
            .expect("test count")
            .to_ne_bytes(),
    );
    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    bytes
}

#[test]
fn host02_mapping_spike_contract_is_explicit_and_fail_closed() {
    let spike = read_text(SPIKE_PATH);

    for required in [
        "BLOCKED_AMBIGUOUS",
        "BLOCKED_CONFLICTING_FACTS",
        "BLOCKED_TOPOLOGY_CHANGED",
        "BLOCKED_NVCONTROL_UNAVAILABLE",
        "XRandR output XID",
        "NV-CONTROL display target",
        "NV-CONTROL GPU target",
        "NV_CTRL_DISPLAY_RANDR_OUTPUT_ID",
        "NV_CTRL_BINARY_DATA_DISPLAYS_CONNECTED_TO_GPU",
        "exactly one XRandR output",
        "exactly one XRandR provider membership",
        "exactly one NV-CONTROL display target",
        "exactly one owning GPU target",
        "exactly one current NVML device",
        "canonical PCI BDF",
        "raw EDID",
        "MST",
        "DRM",
        "optional diagnostic",
    ] {
        assert!(
            spike.contains(required),
            "spike is missing required contract marker {required:?}"
        );
    }

    assert!(
        !spike.contains("NV-CONTROL`: **NOT_REQUIRED**")
            && !spike.contains("Selected DRM connector participates in MST"),
        "the superseded DRM-authoritative/MST-rejection contract must be removed"
    );
}

#[test]
fn host02_mapping_fixture_covers_nvcontrol_and_drm_diagnostic_boundaries() {
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
        ("nvcontrol-mst-dp-0-3", "pass"),
        ("identical-edid-shared-mst-name", "pass"),
        ("drm-absent", "pass"),
        ("drm-zero-inactive-ambiguous-changing", "pass"),
        ("order-primary-number-not-ownership", "pass"),
        ("missing-nvcontrol-extension", "blocked"),
        ("missing-nvcontrol-version", "blocked"),
        ("old-nvcontrol-version", "blocked"),
        ("non-nvidia-screen", "blocked"),
        ("zero-display-targets-for-xid", "blocked"),
        ("multiple-display-targets-for-xid", "blocked"),
        ("display-name-mismatch", "blocked"),
        ("display-disabled", "blocked"),
        ("not-enabled-on-xscreen", "blocked"),
        ("duplicate-enabled-membership", "blocked"),
        ("zero-owning-gpus", "blocked"),
        ("multiple-owning-gpus", "blocked"),
        ("duplicate-gpu-membership", "blocked"),
        ("invalid-pci-fields", "blocked"),
        ("nvml-bdf-mismatch", "blocked"),
        ("nvml-uuid-mismatch", "blocked"),
        ("provider-ambiguity", "blocked"),
        ("topology-changed", "blocked"),
        ("randr-event-churn", "blocked"),
        ("invalid-timing", "blocked"),
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
fn host02_nvcontrol_binary_target_lists_are_exact_bounded_and_duplicate_free() {
    assert_eq!(
        decode_nvcontrol_target_list(&target_list_bytes(&[0, 6, 7])).expect("valid list"),
        vec![0, 6, 7],
        "NV-CONTROL display and GPU target ID zero is valid"
    );

    let malformed = [
        Vec::new(),
        vec![0, 0, 0],
        target_list_bytes(&[17, 17]),
        {
            let mut bytes = target_list_bytes(&[17]);
            bytes.extend_from_slice(&18_u32.to_ne_bytes());
            bytes
        },
        {
            let mut bytes = target_list_bytes(&[17]);
            bytes[..4].copy_from_slice(&2_u32.to_ne_bytes());
            bytes
        },
        {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&65_u32.to_ne_bytes());
            bytes.resize((65 + 1) * 4, 0);
            bytes
        },
        target_list_bytes(&[u32::MAX]),
    ];
    for bytes in malformed {
        assert!(
            decode_nvcontrol_target_list(&bytes).is_err(),
            "malformed binary target list must fail closed: {bytes:?}"
        );
    }
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
                assert_eq!(selected.proof.nvcontrol_display_target_matches, 1);
                assert_eq!(selected.proof.enabled_on_xscreen_matches, 1);
                assert_eq!(selected.proof.nvcontrol_gpu_owner_matches, 1);
                assert_eq!(selected.proof.nvml_device_matches, 1);
            }
            "blocked" => {
                let failure = prove_output_gpu_mapping(&topology)
                    .expect_err("blocked case must not guess a selection");
                assert_eq!(
                    failure.reason.as_code(),
                    expected_case["expected"]["reason"]
                        .as_str()
                        .expect("expected reason"),
                    "case {case_id}"
                );
            }
            status => panic!("unknown expected status {status:?}"),
        }
    }
}

#[test]
fn host02_mapping_exact_name_origin_timing_and_mst_round_trip() {
    let (mut topology, _) = topology_case("nvcontrol-mst-dp-0-3");
    topology.requested_output_name.hex = "44502d302ece94".to_owned();
    topology.requested_output_name.display = Some("DP-0.Δ".to_owned());
    topology.randr_outputs[0].name = topology.requested_output_name.clone();
    topology.randr_outputs[0].origin_x = -7680;
    topology.randr_outputs[0].origin_y = -2160;
    topology.nvcontrol.display_targets[0].randr_name =
        Some(topology.requested_output_name.clone());

    let selected = prove_output_gpu_mapping(&topology).expect("complete relation must pass");
    assert_eq!(selected.output_name.hex, "44502d302ece94");
    assert_eq!(selected.output_name.display.as_deref(), Some("DP-0.Δ"));
    assert_eq!(selected.origin_x, -7680);
    assert_eq!(selected.origin_y, -2160);
    assert_eq!(
        selected.refresh_hz,
        RefreshRateV1 {
            numerator: 60,
            denominator: 1,
        }
    );
    assert!(selected.nvcontrol_displayport_is_multistream);

    let encoded = serde_json::to_vec(&selected).expect("selected output must encode");
    let decoded: SelectedOutputV1 =
        serde_json::from_slice(&encoded).expect("selected output must decode");
    assert_eq!(decoded, selected);
}

#[test]
fn host02_mapping_bounds_and_exact_arithmetic_reject_invalid_observations() {
    let (topology, _) = topology_case("nvcontrol-mst-dp-0-3");

    let mut oversized = topology.clone();
    oversized.randr_outputs = vec![topology.randr_outputs[0].clone(); 65];
    assert_eq!(
        prove_output_gpu_mapping(&oversized)
            .expect_err("oversized output collection must fail")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );

    let mut invalid_name = topology.clone();
    invalid_name.requested_output_name.hex = "44502D302e33".to_owned();
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

    let mut unsupported_clock_flags = topology;
    unsupported_clock_flags.randr_outputs[0].timing.flags = 0x1000;
    assert_eq!(
        prove_output_gpu_mapping(&unsupported_clock_flags)
            .expect_err("unimplemented clock modifiers must fail closed")
            .reason,
        OutputMappingReasonV1::InvalidObservation
    );
}

#[test]
fn host02_source_defined_xid_target_gpu_nvml_relation_ignores_shortcuts() {
    let (topology, _) = topology_case("identical-edid-shared-mst-name");
    let selected = prove_output_gpu_mapping(&topology).expect("unique source relation must pass");

    let output_xid: XrandrOutputXidV1 = selected.randr_output_xid;
    let display_target: NvControlDisplayTargetIdV1 = selected.nvcontrol_display_target_id;
    let gpu_target: NvControlGpuTargetIdV1 = selected.nvcontrol_gpu_target_id;
    assert_eq!(output_xid.get(), 73);
    assert_eq!(display_target.get(), 17);
    assert_eq!(gpu_target.get(), 0);
    assert_ne!(output_xid.get(), display_target.get());
    assert_eq!(
        selected.nvcontrol_gpu_pci_bdf, selected.nvml_pci_bdf,
        "PCI BDF must match exactly"
    );
    assert_eq!(
        selected.nvcontrol_gpu_uuid, selected.nvml_uuid,
        "GPU UUID must match exactly"
    );
    assert!(selected.nvcontrol_displayport_is_multistream);
}

#[test]
fn host02_drm_diagnostics_never_enter_the_authoritative_topology_token() {
    let (topology, _) = topology_case("drm-zero-inactive-ambiguous-changing");
    assert_ne!(
        topology.drm_diagnostic_before,
        topology.drm_diagnostic_after
    );
    let selected =
        prove_output_gpu_mapping(&topology).expect("changing DRM diagnostics must not block");
    assert_ne!(
        selected.drm_diagnostic_before,
        selected.drm_diagnostic_after
    );
    let token = serde_json::to_value(&selected.topology_token).expect("token JSON");
    assert!(token.get("drm_snapshot_sha256").is_none());
    assert!(token.get("nvcontrol_snapshot_sha256").is_some());
}

#[test]
fn host02_topology_or_randr_event_change_returns_no_selection() {
    for case in ["topology-changed", "randr-event-churn"] {
        let (topology, _) = topology_case(case);
        let failure =
            prove_output_gpu_mapping(&topology).expect_err("mixed topology generations must fail");
        assert_eq!(failure.reason, OutputMappingReasonV1::TopologyChanged);
        assert_eq!(failure.reason.as_code(), "BLOCKED_TOPOLOGY_CHANGED");
    }
}
