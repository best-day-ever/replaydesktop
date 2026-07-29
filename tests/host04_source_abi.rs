use std::fs;
use std::path::PathBuf;

fn project_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[test]
fn host04_source_abi_build_gate_authenticates_sdk_13_1() {
    let build = fs::read_to_string(project_path("build.rs")).expect("build script");
    for contract in [
        "REPLAY_NVENC_SDK_ROOT",
        "nvEncodeAPI.h",
        "replay_nvenc_source",
        "REPLAY_NVENC_SOURCE_SHA256",
        "native/nvenc_abi_oracle.c",
    ] {
        assert!(build.contains(contract), "build gate missing {contract}");
    }
}

#[test]
fn host04_source_abi_oracle_owns_every_native_boundary() {
    let oracle = fs::read_to_string(project_path("native/nvenc_abi_oracle.c"))
        .expect("authenticated NVENC oracle");
    for contract in [
        "NV_ENCODE_API_FUNCTION_LIST",
        "NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS",
        "NV_ENC_INITIALIZE_PARAMS",
        "NV_ENC_CONFIG",
        "NV_ENC_PRESET_CONFIG",
        "NV_ENC_REGISTER_RESOURCE",
        "NV_ENC_MAP_INPUT_RESOURCE",
        "NV_ENC_CREATE_BITSTREAM_BUFFER",
        "NV_ENC_PIC_PARAMS",
        "NV_ENC_LOCK_BITSTREAM",
        "NV_ENC_CODEC_H264_GUID",
        "NV_ENC_CODEC_HEVC_GUID",
        "NV_ENC_CODEC_AV1_GUID",
        "NV_ENC_PRESET_P2_GUID",
        "NV_ENC_TUNING_INFO_ULTRA_LOW_LATENCY",
        "NVENCAPI_VERSION",
        "NVENCAPI_MAJOR_VERSION",
        "NVENCAPI_MINOR_VERSION",
    ] {
        assert!(oracle.contains(contract), "ABI oracle missing {contract}");
    }
}
