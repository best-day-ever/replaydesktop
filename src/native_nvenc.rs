use crate::model::{
    CopyBoundaryProofV1, CopyBoundaryStatusV1, MAX_NVENC_COPY_EDGES_V1,
    MAX_NVENC_RESOURCE_EVENTS_V1, NVENC_POLICY_POSITION_COUNT_V1, NVENC_TUPLES_SCHEMA_V1,
    NvencAdmissionV1, NvencAdvertisementV1, NvencApiVersionV1, NvencAttemptOutcomeV1,
    NvencBufferFormatV1, NvencCleanupProofV1, NvencConfigProofV1, NvencCopyEdgeV1,
    NvencGpuGenerationV1, NvencPolicyPositionV1, NvencPresetV1, NvencProviderKindV1,
    NvencResourceEventV1, NvencResourceProofV1, NvencSourceEvidenceV1, NvencTuningV1,
    NvencTupleAttemptV1, NvencTupleV1, NvencTuplesEvidenceV1,
};
#[cfg(replay_nvenc_source)]
use crate::nvenc_bitstream::{MAX_NVENC_BITSTREAM_BYTES_V1, inspect_nvenc_bitstream};
#[cfg(replay_nvenc_source)]
use std::ffi::{c_int, c_void};
#[cfg(replay_nvenc_source)]
use std::path::{Path, PathBuf};

const LIVE_NVENC_SOURCE_IDENTITY: &str = "nvidia-video-codec-sdk-13.1";
#[cfg(replay_nvenc_source)]
const NVENC_RUNTIME_LIBRARY: &str = "libnvidia-encode.so.1";

#[cfg(replay_nvenc_source)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct NativeNvencProbeResult {
    struct_size: u32,
    status_class: u32,
    nvenc_status: u32,
    runtime_max_api: u32,
    acquired_mask: u32,
    released_mask: u32,
    config_mask: u32,
    resource_mask: u32,
    bitstream_size: u32,
    mapped_format: u32,
    picture_type: u32,
    cleanup_status: u32,
}

#[cfg(replay_nvenc_source)]
type NvEncodeApiCreateInstanceFn = unsafe extern "C" fn(*mut c_void) -> c_int;
#[cfg(replay_nvenc_source)]
type NvEncodeApiGetMaxSupportedVersionFn = unsafe extern "C" fn(*mut u32) -> c_int;

#[cfg(replay_nvenc_source)]
#[link(name = "replay_nvenc_abi_oracle", kind = "static")]
unsafe extern "C" {
    fn replay_nvenc_header_api_major() -> u32;
    fn replay_nvenc_header_api_minor() -> u32;
    fn replay_nvenc_header_api_version() -> u32;
    fn replay_nvenc_runtime_api_major(raw: u32) -> u32;
    fn replay_nvenc_runtime_api_minor(raw: u32) -> u32;
    fn replay_nvenc_min_linux_driver_major() -> u32;
    fn replay_nvenc_min_cuda_driver_api() -> u32;
    fn replay_nvenc_native_status_success() -> u32;
    fn replay_nvenc_status_success_class() -> u32;
    fn replay_nvenc_status_unsupported_class() -> u32;
    fn replay_nvenc_status_operational_reject_class() -> u32;
    fn replay_nvenc_status_invalid_input_class() -> u32;
    fn replay_nvenc_config_complete_mask() -> u32;
    fn replay_nvenc_resource_complete_mask() -> u32;
    fn replay_nvenc_sizeof_probe_result() -> usize;
    fn replay_nvenc_alignof_probe_result() -> usize;
    fn replay_nvenc_signature_mask() -> u32;
    fn replay_nvenc_probe_one_frame(
        create_instance_fn: *mut c_void,
        get_max_version_fn: *mut c_void,
        cuda_context: *mut c_void,
        cuda_device_pointer: u64,
        allocation_size: u64,
        width: u32,
        height: u32,
        pitch: u32,
        policy_position: u32,
        bitstream_out: *mut u8,
        capacity: u32,
        result: *mut NativeNvencProbeResult,
    ) -> c_int;
}

#[cfg(not(replay_nvenc_source))]
pub fn compiled_nvenc_source() -> NvencSourceEvidenceV1 {
    NvencSourceEvidenceV1::unavailable(NvencApiVersionV1::new(13, 1))
}

#[cfg(replay_nvenc_source)]
pub fn compiled_nvenc_source() -> NvencSourceEvidenceV1 {
    // SAFETY: These zero-argument project-owned oracle exports return only
    // compile-time facts from the authenticated header snapshot.
    let (major, minor, raw, minimum_driver, minimum_cuda) = unsafe {
        (
            replay_nvenc_header_api_major(),
            replay_nvenc_header_api_minor(),
            replay_nvenc_header_api_version(),
            replay_nvenc_min_linux_driver_major(),
            replay_nvenc_min_cuda_driver_api(),
        )
    };
    NvencSourceEvidenceV1 {
        api_version: NvencApiVersionV1::new(
            u16::try_from(major).expect("authenticated API major fits u16"),
            u16::try_from(minor).expect("authenticated API minor fits u16"),
        ),
        api_version_raw: Some(raw),
        source_identity: Some(env!("REPLAY_NVENC_SOURCE_IDENTITY").to_owned()),
        header_sha256: Some(
            env!("REPLAY_NVENC_SOURCE_SHA256")
                .parse()
                .expect("build script emits a checked NVENC SHA-256"),
        ),
        runtime_library: None,
        runtime_max_api_version: None,
        runtime_max_api_version_raw: None,
        minimum_linux_driver_major: u16::try_from(minimum_driver).ok(),
        nvidia_driver_major: None,
        minimum_cuda_driver_version: Some(minimum_cuda),
        cuda_driver_version: None,
    }
}

#[derive(Debug, Clone)]
pub struct NativeNvencProbeProvider {
    requested_output: crate::model::OutputNameV1,
    source: NvencSourceEvidenceV1,
    runtime_source_ready: bool,
}

impl NativeNvencProbeProvider {
    pub fn new(requested_output: crate::model::OutputNameV1) -> Self {
        let source = compiled_nvenc_source();
        #[cfg(replay_nvenc_source)]
        let runtime_source_ready =
            authenticate_runtime_nvenc_header(&source) && verify_nvenc_bridge_abi();
        #[cfg(not(replay_nvenc_source))]
        let runtime_source_ready = false;
        Self {
            requested_output,
            source,
            runtime_source_ready,
        }
    }
}

impl NvencProvider for NativeNvencProbeProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        if self.source.source_identity.is_some() {
            NvencProviderKindV1::SourceAuthenticated
        } else {
            NvencProviderKindV1::LiveUnavailable
        }
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        if self.kind() == NvencProviderKindV1::LiveUnavailable {
            return NvencTupleAttemptV1::provider_unavailable(
                position,
                tuple,
                self.source.api_version,
            );
        }
        if !self.runtime_source_ready {
            return rejected_attempt(position, tuple, self.source.api_version);
        }
        #[cfg(replay_nvenc_source)]
        {
            let output = self.requested_output.clone();
            match probe_nvenc_tuple_with_runtime(&output, position, tuple) {
                Ok(run) => {
                    if run.runtime_max_api_version_raw.is_some() {
                        self.source.runtime_library = Some(NVENC_RUNTIME_LIBRARY.to_owned());
                    }
                    if let Some(version) = run.runtime_max_api_version {
                        self.source.runtime_max_api_version = Some(version);
                    }
                    if let Some(version) = run.runtime_max_api_version_raw {
                        self.source.runtime_max_api_version_raw = Some(version);
                    }
                    if let Some(driver) = run.nvidia_driver_major {
                        self.source.nvidia_driver_major = Some(driver);
                    }
                    if let Some(driver) = run.cuda_driver_version {
                        self.source.cuda_driver_version = Some(driver);
                    }
                    run.attempt
                }
                Err(_) => rejected_attempt(position, tuple, self.source.api_version),
            }
        }
        #[cfg(not(replay_nvenc_source))]
        {
            NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
        }
    }
}

pub fn observe_live_nvenc_policy(
    requested_output: &crate::model::OutputNameV1,
) -> NvencTuplesEvidenceV1 {
    let mut provider = NativeNvencProbeProvider::new(requested_output.clone());
    evaluate_nvenc_policy(NvencGpuGenerationV1::Unknown, &mut provider)
}

pub fn probe_nvenc_tuple_one_frame(
    requested_output: &crate::model::OutputNameV1,
    position: NvencPolicyPositionV1,
) -> NvencTupleAttemptV1 {
    let mut provider = NativeNvencProbeProvider::new(requested_output.clone());
    provider.attempt(position, position.tuple())
}

fn rejected_attempt(
    position: NvencPolicyPositionV1,
    tuple: NvencTupleV1,
    api_version: NvencApiVersionV1,
) -> NvencTupleAttemptV1 {
    NvencTupleAttemptV1 {
        position,
        tuple,
        api_version,
        provider_invoked: true,
        terminal: true,
        outcome: NvencAttemptOutcomeV1::Rejected,
        config_proof: None,
        resource_proof: None,
        copy_proof: None,
        stream_proof: None,
        cleanup: NvencCleanupProofV1::no_resources(),
    }
}

fn unsupported_attempt(
    position: NvencPolicyPositionV1,
    tuple: NvencTupleV1,
    api_version: NvencApiVersionV1,
) -> NvencTupleAttemptV1 {
    NvencTupleAttemptV1 {
        outcome: NvencAttemptOutcomeV1::Unsupported,
        ..rejected_attempt(position, tuple, api_version)
    }
}

#[cfg(replay_nvenc_source)]
struct NativeNvencRun {
    attempt: NvencTupleAttemptV1,
    runtime_max_api_version: Option<NvencApiVersionV1>,
    runtime_max_api_version_raw: Option<u32>,
    nvidia_driver_major: Option<u16>,
    cuda_driver_version: Option<u32>,
}

#[cfg(replay_nvenc_source)]
fn probe_nvenc_tuple_with_runtime(
    requested_output: &crate::model::OutputNameV1,
    position: NvencPolicyPositionV1,
    tuple: NvencTupleV1,
) -> Result<NativeNvencRun, ()> {
    let source = compiled_nvenc_source();
    if tuple.buffer_format() != NvencBufferFormatV1::Nv12 {
        return Ok(NativeNvencRun {
            attempt: unsupported_attempt(position, tuple, source.api_version),
            runtime_max_api_version: None,
            runtime_max_api_version_raw: None,
            nvidia_driver_major: read_nvidia_driver_major(),
            cuda_driver_version: None,
        });
    }
    crate::native_nvfbc::with_live_selected_capture_lease(requested_output, |lease| {
        let run = run_nvenc_on_capture_lease(&lease, position, tuple, source.api_version);
        let containment_required =
            !run.attempt.cleanup.complete && !run.attempt.cleanup.acquired.is_empty();
        (run, containment_required)
    })
    .map_err(|_| ())
}

#[cfg(replay_nvenc_source)]
struct DynamicNvencApi {
    _library: libloading::os::unix::Library,
    create_instance: NvEncodeApiCreateInstanceFn,
    get_max_supported_version: NvEncodeApiGetMaxSupportedVersionFn,
}

#[cfg(replay_nvenc_source)]
impl DynamicNvencApi {
    fn load() -> Result<Self, ()> {
        let library = unsafe {
            // SAFETY: This is a fixed driver SONAME in a worker whose loader
            // environment was cleared before exec.
            libloading::os::unix::Library::open(
                Some(NVENC_RUNTIME_LIBRARY),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .map_err(|_| ())?;
        let create_instance = unsafe {
            // SAFETY: The bootstrap signature is checked by the C source oracle.
            *library
                .get::<NvEncodeApiCreateInstanceFn>(b"NvEncodeAPICreateInstance\0")
                .map_err(|_| ())?
        };
        let get_max_supported_version = unsafe {
            // SAFETY: The bootstrap signature is checked by the C source oracle.
            *library
                .get::<NvEncodeApiGetMaxSupportedVersionFn>(b"NvEncodeAPIGetMaxSupportedVersion\0")
                .map_err(|_| ())?
        };
        if !loaded_nvenc_library_is_unique() {
            return Err(());
        }
        Ok(Self {
            _library: library,
            create_instance,
            get_max_supported_version,
        })
    }
}

#[cfg(replay_nvenc_source)]
fn run_nvenc_on_capture_lease(
    lease: &crate::native_nvfbc::NativeCaptureLease,
    position: NvencPolicyPositionV1,
    tuple: NvencTupleV1,
    api_version: NvencApiVersionV1,
) -> NativeNvencRun {
    let cuda_driver_version = Some(lease.cuda_driver_version());
    let nvidia_driver_major = read_nvidia_driver_major();
    let rejected = || NativeNvencRun {
        attempt: rejected_attempt(position, tuple, api_version),
        runtime_max_api_version: None,
        runtime_max_api_version_raw: None,
        nvidia_driver_major,
        cuda_driver_version,
    };
    let frame = lease.frame();
    if frame.width_px != tuple.width_px()
        || frame.height_px != tuple.height_px()
        || frame.pixel_format != crate::model::CapturePixelFormatV1::Nv12
        || frame.requested_pixel_format != Some(crate::model::CapturePixelFormatV1::Nv12)
        || frame.pitch_bytes < u32::from(tuple.width_px())
        || frame.byte_size != Some(lease.allocation_size())
        || !frame.is_new_frame
    {
        return rejected();
    }
    let api = match DynamicNvencApi::load() {
        Ok(api) => api,
        Err(()) => return rejected(),
    };
    let mut bytes = vec![0_u8; MAX_NVENC_BITSTREAM_BYTES_V1];
    let mut result = NativeNvencProbeResult::default();
    let bridge_status = unsafe {
        // SAFETY: The C bridge owns every native declaration. The CUDA context
        // and allocation remain live for this callback, and the byte buffer is
        // writable for its exact bounded capacity.
        replay_nvenc_probe_one_frame(
            api.create_instance as *const () as *mut c_void,
            api.get_max_supported_version as *const () as *mut c_void,
            lease.context(),
            lease.device_pointer(),
            lease.allocation_size(),
            u32::from(tuple.width_px()),
            u32::from(tuple.height_px()),
            frame.pitch_bytes,
            position_index(position),
            bytes.as_mut_ptr(),
            u32::try_from(bytes.len()).expect("bitstream bound fits u32"),
            &mut result,
        )
    };
    let expected_result_size = unsafe { replay_nvenc_sizeof_probe_result() };
    if bridge_status != 0
        || result.struct_size as usize != expected_result_size
        || result.bitstream_size as usize > bytes.len()
    {
        bytes.fill(0);
        return rejected();
    }

    let runtime_max_api_version = runtime_api_version(result.runtime_max_api);
    let runtime_max_api_version_raw = Some(result.runtime_max_api);
    let cleanup = cleanup_from_native_result(&result);
    let config_complete = result.config_mask == unsafe { replay_nvenc_config_complete_mask() };
    let resource_complete =
        result.resource_mask == unsafe { replay_nvenc_resource_complete_mask() };
    let config_proof = config_complete.then(|| exact_config_proof(tuple));
    let resource_proof = resource_complete.then(|| exact_resource_proof(lease, tuple, &result));
    let copy_proof = resource_complete.then(known_copy_proof);
    let success_class = unsafe { replay_nvenc_status_success_class() };
    let unsupported_class = unsafe { replay_nvenc_status_unsupported_class() };
    let cleanup_success = result.cleanup_status == unsafe { replay_nvenc_native_status_success() };

    let (outcome, stream_proof) = if result.status_class == success_class
        && config_complete
        && resource_complete
        && cleanup.complete
        && cleanup_success
    {
        let stream_len = result.bitstream_size as usize;
        let parsed = inspect_nvenc_bitstream(position, &bytes[..stream_len]);
        bytes.fill(0);
        match parsed {
            Ok(proof) => (NvencAttemptOutcomeV1::Success, Some(proof)),
            Err(_) => (NvencAttemptOutcomeV1::InvalidStream, None),
        }
    } else {
        bytes.fill(0);
        if result.status_class == unsupported_class {
            (NvencAttemptOutcomeV1::Unsupported, None)
        } else {
            (NvencAttemptOutcomeV1::Rejected, None)
        }
    };

    NativeNvencRun {
        attempt: NvencTupleAttemptV1 {
            position,
            tuple,
            api_version,
            provider_invoked: true,
            terminal: true,
            outcome,
            config_proof,
            resource_proof,
            copy_proof,
            stream_proof,
            cleanup,
        },
        runtime_max_api_version,
        runtime_max_api_version_raw,
        nvidia_driver_major,
        cuda_driver_version,
    }
}

#[cfg(replay_nvenc_source)]
fn position_index(position: NvencPolicyPositionV1) -> u32 {
    NvencPolicyPositionV1::ALL
        .iter()
        .position(|candidate| *candidate == position)
        .and_then(|index| u32::try_from(index).ok())
        .expect("closed NVENC position has a stable index")
}

#[cfg(replay_nvenc_source)]
fn runtime_api_version(raw: u32) -> Option<NvencApiVersionV1> {
    let major = unsafe { replay_nvenc_runtime_api_major(raw) };
    let minor = unsafe { replay_nvenc_runtime_api_minor(raw) };
    Some(NvencApiVersionV1::new(
        u16::try_from(major).ok()?,
        u16::try_from(minor).ok()?,
    ))
}

#[cfg(replay_nvenc_source)]
fn cleanup_from_native_result(result: &NativeNvencProbeResult) -> NvencCleanupProofV1 {
    const SESSION: u32 = 1 << 1;
    const REGISTERED: u32 = 1 << 3;
    const MAPPED: u32 = 1 << 4;
    const BITSTREAM: u32 = 1 << 5;
    const LOCKED: u32 = 1 << 7;
    const UNLOCKED: u32 = 1 << 0;
    const BITSTREAM_RELEASED: u32 = 1 << 1;
    const UNMAPPED: u32 = 1 << 2;
    const UNREGISTERED: u32 = 1 << 3;
    const SESSION_RELEASED: u32 = 1 << 4;

    let mut acquired = Vec::new();
    if result.acquired_mask & SESSION != 0 {
        acquired.push(NvencResourceEventV1::EncoderSession);
    }
    if result.acquired_mask & REGISTERED != 0 {
        acquired.push(NvencResourceEventV1::RegisteredResource);
    }
    if result.acquired_mask & MAPPED != 0 {
        acquired.push(NvencResourceEventV1::MappedResource);
    }
    if result.acquired_mask & BITSTREAM != 0 {
        acquired.push(NvencResourceEventV1::BitstreamBuffer);
    }
    let mut released = Vec::new();
    if result.released_mask & BITSTREAM_RELEASED != 0 {
        released.push(NvencResourceEventV1::BitstreamBuffer);
    }
    if result.released_mask & UNMAPPED != 0 {
        released.push(NvencResourceEventV1::MappedResource);
    }
    if result.released_mask & UNREGISTERED != 0 {
        released.push(NvencResourceEventV1::RegisteredResource);
    }
    if result.released_mask & SESSION_RELEASED != 0 {
        released.push(NvencResourceEventV1::EncoderSession);
    }
    let cleanup_success = result.cleanup_status == unsafe { replay_nvenc_native_status_success() };
    let lock_released = result.acquired_mask & LOCKED == 0 || result.released_mask & UNLOCKED != 0;
    let complete = cleanup_success
        && lock_released
        && released == acquired.iter().rev().copied().collect::<Vec<_>>();
    NvencCleanupProofV1 {
        acquired,
        released,
        complete,
    }
}

#[cfg(replay_nvenc_source)]
fn exact_config_proof(tuple: NvencTupleV1) -> NvencConfigProofV1 {
    NvencConfigProofV1 {
        width_px: tuple.width_px(),
        height_px: tuple.height_px(),
        frame_rate: tuple.frame_rate(),
        preset: NvencPresetV1::P2,
        tuning: NvencTuningV1::UltraLowLatency,
        synchronous: true,
        picture_type_decision: true,
        forced_keyframe: true,
        output_parameter_sets: true,
        b_frames: 0,
        lookahead_depth: 0,
        reorder_delay: 0,
        one_frame_vbv: true,
    }
}

#[cfg(replay_nvenc_source)]
fn exact_resource_proof(
    lease: &crate::native_nvfbc::NativeCaptureLease,
    tuple: NvencTupleV1,
    result: &NativeNvencProbeResult,
) -> NvencResourceProofV1 {
    let frame = lease.frame();
    let selected = lease.selected();
    NvencResourceProofV1 {
        output_name: selected.output_name.clone(),
        topology_token: selected.topology_token.clone(),
        gpu_pci_bdf: selected.nvml_pci_bdf.clone(),
        gpu_uuid: selected.nvml_uuid.clone(),
        lease_surface: frame.lease_surface.clone(),
        width_px: frame.width_px,
        height_px: frame.height_px,
        pitch_bytes: frame.pitch_bytes,
        buffer_format: tuple.buffer_format(),
        allocation_byte_size: lease.allocation_size(),
        allocation_base_verified: true,
        allocation_range_verified: true,
        same_cuda_context: true,
        same_gpu: true,
        registered: true,
        mapped: true,
        submitted: true,
        bitstream_locked: result.bitstream_size > 0,
        bitstream_unlocked: result.released_mask & 1 != 0,
    }
}

#[cfg(replay_nvenc_source)]
fn known_copy_proof() -> CopyBoundaryProofV1 {
    CopyBoundaryProofV1 {
        status: CopyBoundaryStatusV1::Pass,
        application_edges: vec![
            NvencCopyEdgeV1::RegisterNoCopy,
            NvencCopyEdgeV1::MapNoCopy,
            NvencCopyEdgeV1::SubmitNoCopy,
        ],
        encoder_internal_edges: vec![NvencCopyEdgeV1::SameGpuPitchLinearToBlockLinearCopy],
        host_staging_edges: 0,
        cross_gpu_edges: 0,
        unknown_application_edges: 0,
        unknown_encoder_internal_edges: 0,
    }
}

#[cfg(replay_nvenc_source)]
fn verify_nvenc_bridge_abi() -> bool {
    unsafe {
        replay_nvenc_header_api_major() == 13
            && replay_nvenc_header_api_minor() == 1
            && replay_nvenc_sizeof_probe_result() == std::mem::size_of::<NativeNvencProbeResult>()
            && replay_nvenc_alignof_probe_result() == std::mem::align_of::<NativeNvencProbeResult>()
            && replay_nvenc_signature_mask() != 0
            && replay_nvenc_min_linux_driver_major() == 610
            && replay_nvenc_min_cuda_driver_api() == 13_010
            && replay_nvenc_status_success_class() != replay_nvenc_status_unsupported_class()
            && replay_nvenc_status_success_class() != replay_nvenc_status_operational_reject_class()
            && replay_nvenc_status_success_class() != replay_nvenc_status_invalid_input_class()
    }
}

#[cfg(replay_nvenc_source)]
fn authenticate_runtime_nvenc_header(source: &NvencSourceEvidenceV1) -> bool {
    let Some(root) = std::env::var_os("REPLAY_NVENC_SDK_ROOT").map(PathBuf::from) else {
        return false;
    };
    if !root.is_absolute() {
        return false;
    }
    let Ok(canonical_root) = std::fs::canonicalize(&root) else {
        return false;
    };
    if canonical_root != root {
        return false;
    }
    let header = root.join("nvEncodeAPI.h");
    let Ok(link_metadata) = std::fs::symlink_metadata(&header) else {
        return false;
    };
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return false;
    }
    let Ok(canonical_header) = std::fs::canonicalize(&header) else {
        return false;
    };
    if canonical_header != header || !canonical_header.starts_with(&canonical_root) {
        return false;
    }
    crate::sha256_file(&canonical_header)
        .ok()
        .is_some_and(|digest| Some(digest) == source.header_sha256)
}

#[cfg(replay_nvenc_source)]
fn loaded_nvenc_library_is_unique() -> bool {
    let Ok(maps) = std::fs::read_to_string("/proc/self/maps") else {
        return false;
    };
    let names = maps
        .lines()
        .filter_map(|line| line.split_ascii_whitespace().last())
        .filter(|path| path.starts_with('/') && !path.ends_with(" (deleted)"))
        .filter_map(|path| Path::new(path).file_name()?.to_str())
        .filter(|name| name.starts_with("libnvidia-encode.so."))
        .collect::<std::collections::HashSet<_>>();
    names.len() == 1
}

#[cfg(replay_nvenc_source)]
fn read_nvidia_driver_major() -> Option<u16> {
    let version = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    version
        .split_ascii_whitespace()
        .find(|token| {
            token.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && token.bytes().filter(|byte| *byte == b'.').count() >= 2
        })
        .and_then(|token| token.split('.').next())
        .and_then(|major| major.parse().ok())
}

pub trait NvencProvider {
    fn kind(&self) -> NvencProviderKindV1;

    fn source(&self) -> &NvencSourceEvidenceV1;

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1;
}

#[derive(Debug, Clone)]
pub struct LiveUnavailableNvencProvider {
    source: NvencSourceEvidenceV1,
}

impl LiveUnavailableNvencProvider {
    pub fn new(api_version: NvencApiVersionV1) -> Self {
        Self {
            source: NvencSourceEvidenceV1::unavailable(api_version),
        }
    }
}

impl NvencProvider for LiveUnavailableNvencProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        NvencProviderKindV1::LiveUnavailable
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticNvencProvider {
    source: NvencSourceEvidenceV1,
}

impl DiagnosticNvencProvider {
    pub fn new(api_version: NvencApiVersionV1) -> Self {
        Self {
            source: NvencSourceEvidenceV1::diagnostic(api_version),
        }
    }
}

impl NvencProvider for DiagnosticNvencProvider {
    fn kind(&self) -> NvencProviderKindV1 {
        NvencProviderKindV1::DiagnosticFixture
    }

    fn source(&self) -> &NvencSourceEvidenceV1 {
        &self.source
    }

    fn attempt(
        &mut self,
        position: NvencPolicyPositionV1,
        tuple: NvencTupleV1,
    ) -> NvencTupleAttemptV1 {
        NvencTupleAttemptV1::provider_unavailable(position, tuple, self.source.api_version)
    }
}

pub const fn nvenc_policy_positions()
-> &'static [NvencPolicyPositionV1; NVENC_POLICY_POSITION_COUNT_V1] {
    &NvencPolicyPositionV1::ALL
}

pub fn evaluate_nvenc_policy<P: NvencProvider>(
    generation: NvencGpuGenerationV1,
    provider: &mut P,
) -> NvencTuplesEvidenceV1 {
    let provider_kind = provider.kind();
    let initial_source = provider.source().clone();
    let mut attempts = Vec::with_capacity(NVENC_POLICY_POSITION_COUNT_V1);

    for position in NvencPolicyPositionV1::ALL {
        let tuple = position.tuple();
        let attempt = if position.requires_ada_or_newer() && !generation.supports_av1_encode() {
            NvencTupleAttemptV1::generation_ineligible(position, tuple, initial_source.api_version)
        } else {
            provider.attempt(position, tuple)
        };
        attempts.push(attempt);
    }

    let mut evidence = NvencTuplesEvidenceV1 {
        schema: NVENC_TUPLES_SCHEMA_V1.to_owned(),
        provider: provider_kind,
        source: provider.source().clone(),
        admission: NvencAdmissionV1::Unproven,
        gpu_generation: generation,
        attempts,
        advertised: Vec::new(),
    };

    if evidence.provider == NvencProviderKindV1::SourceAuthenticated {
        let derived = evidence
            .attempts
            .iter()
            .filter_map(advertisement_from_attempt)
            .collect::<Vec<_>>();
        let blocking_failure = evidence.attempts.iter().any(attempt_blocks_admission);
        if !blocking_failure {
            evidence.advertised = derived;
        }
        evidence.admission = if blocking_failure || evidence.advertised.is_empty() {
            NvencAdmissionV1::Rejected
        } else {
            NvencAdmissionV1::Pass
        };
    }
    evidence
}

pub fn advertisement_from_attempt(attempt: &NvencTupleAttemptV1) -> Option<NvencAdvertisementV1> {
    if !attempt.terminal
        || !attempt.provider_invoked
        || attempt.outcome != NvencAttemptOutcomeV1::Success
        || !valid_config_proof(attempt.tuple, attempt.config_proof.as_ref()?)
        || !valid_resource_proof(attempt.tuple, attempt.resource_proof.as_ref()?)
        || !valid_copy_proof(attempt.copy_proof.as_ref()?)
        || !valid_cleanup(&attempt.cleanup)
    {
        return None;
    }

    let stream = attempt.stream_proof.as_ref()?;
    if !stream.keyframe
        || stream.byte_len == 0
        || stream.parsed_tuple != attempt.tuple
        || attempt.position.tuple() != attempt.tuple
    {
        return None;
    }

    Some(NvencAdvertisementV1 {
        position: attempt.position,
        tuple: attempt.tuple,
        bitstream_sha256: stream.bitstream_sha256,
    })
}

pub fn validate_nvenc_tuples_evidence(evidence: &NvencTuplesEvidenceV1) -> bool {
    if evidence.schema != NVENC_TUPLES_SCHEMA_V1
        || evidence.attempts.len() != NVENC_POLICY_POSITION_COUNT_V1
        || evidence.advertised.len() > NVENC_POLICY_POSITION_COUNT_V1
        || !valid_source(evidence)
    {
        return false;
    }

    for (expected, attempt) in NvencPolicyPositionV1::ALL.iter().zip(&evidence.attempts) {
        if attempt.position != *expected
            || attempt.tuple != expected.tuple()
            || attempt.api_version != evidence.source.api_version
            || !valid_attempt(evidence.gpu_generation, attempt)
        {
            return false;
        }
    }

    let derived = evidence
        .attempts
        .iter()
        .filter_map(advertisement_from_attempt)
        .collect::<Vec<_>>();

    match evidence.provider {
        NvencProviderKindV1::DiagnosticFixture | NvencProviderKindV1::LiveUnavailable => {
            evidence.admission == NvencAdmissionV1::Unproven
                && evidence.advertised.is_empty()
                && evidence.attempts.iter().all(|attempt| {
                    attempt.config_proof.is_none()
                        && attempt.resource_proof.is_none()
                        && attempt.copy_proof.is_none()
                        && attempt.stream_proof.is_none()
                })
        }
        NvencProviderKindV1::SourceAuthenticated => {
            if !source_compiled(&evidence.source)
                || (evidence.admission == NvencAdmissionV1::Pass
                    && !source_authenticated(&evidence.source))
            {
                return false;
            }
            match evidence.admission {
                NvencAdmissionV1::Pass => {
                    !derived.is_empty()
                        && evidence.advertised == derived
                        && !evidence.attempts.iter().any(attempt_blocks_admission)
                }
                NvencAdmissionV1::Rejected => {
                    evidence.advertised.is_empty()
                        && (derived.is_empty()
                            || evidence.attempts.iter().any(attempt_blocks_admission))
                }
                NvencAdmissionV1::Unproven => false,
            }
        }
    }
}

fn valid_source(evidence: &NvencTuplesEvidenceV1) -> bool {
    let source = &evidence.source;
    if source.api_version.major == 0 {
        return false;
    }
    match evidence.provider {
        NvencProviderKindV1::DiagnosticFixture | NvencProviderKindV1::LiveUnavailable => {
            source.api_version_raw.is_none()
                && source.runtime_max_api_version.is_none()
                && source.runtime_max_api_version_raw.is_none()
                && source.minimum_linux_driver_major.is_none()
                && source.nvidia_driver_major.is_none()
                && source.minimum_cuda_driver_version.is_none()
                && source.cuda_driver_version.is_none()
                && source.source_identity.is_none()
                && source.header_sha256.is_none()
                && source.runtime_library.is_none()
        }
        NvencProviderKindV1::SourceAuthenticated => source_compiled(source),
    }
}

fn valid_attempt(generation: NvencGpuGenerationV1, attempt: &NvencTupleAttemptV1) -> bool {
    if !attempt.terminal
        || attempt.cleanup.acquired.len() > MAX_NVENC_RESOURCE_EVENTS_V1
        || attempt.cleanup.released.len() > MAX_NVENC_RESOURCE_EVENTS_V1
    {
        return false;
    }

    if attempt.position.requires_ada_or_newer() && !generation.supports_av1_encode() {
        return !attempt.provider_invoked
            && attempt.outcome == NvencAttemptOutcomeV1::GenerationIneligible
            && attempt.config_proof.is_none()
            && attempt.resource_proof.is_none()
            && attempt.copy_proof.is_none()
            && attempt.stream_proof.is_none()
            && valid_cleanup(&attempt.cleanup);
    }

    match attempt.outcome {
        NvencAttemptOutcomeV1::Success => advertisement_from_attempt(attempt).is_some(),
        NvencAttemptOutcomeV1::GenerationIneligible => false,
        NvencAttemptOutcomeV1::Unsupported => {
            attempt.provider_invoked
                && attempt.config_proof.is_none()
                && attempt.resource_proof.is_none()
                && attempt.copy_proof.is_none()
                && attempt.stream_proof.is_none()
                && valid_cleanup(&attempt.cleanup)
        }
        NvencAttemptOutcomeV1::ProviderUnavailable
        | NvencAttemptOutcomeV1::Rejected
        | NvencAttemptOutcomeV1::Timeout
        | NvencAttemptOutcomeV1::InvalidStream => valid_failed_attempt(attempt),
    }
}

fn attempt_blocks_admission(attempt: &NvencTupleAttemptV1) -> bool {
    if !attempt.terminal {
        return true;
    }
    match attempt.outcome {
        NvencAttemptOutcomeV1::Success => advertisement_from_attempt(attempt).is_none(),
        NvencAttemptOutcomeV1::GenerationIneligible | NvencAttemptOutcomeV1::Unsupported => false,
        NvencAttemptOutcomeV1::ProviderUnavailable
        | NvencAttemptOutcomeV1::Rejected
        | NvencAttemptOutcomeV1::Timeout
        | NvencAttemptOutcomeV1::InvalidStream => true,
    }
}

fn valid_failed_attempt(attempt: &NvencTupleAttemptV1) -> bool {
    attempt.provider_invoked
        && attempt.stream_proof.is_none()
        && valid_cleanup(&attempt.cleanup)
        && attempt
            .config_proof
            .as_ref()
            .is_none_or(|proof| valid_config_proof(attempt.tuple, proof))
        && attempt
            .resource_proof
            .as_ref()
            .is_none_or(|proof| valid_resource_shape(attempt.tuple, proof))
        && attempt.copy_proof.as_ref().is_none_or(|proof| {
            proof.application_edges.len() <= MAX_NVENC_COPY_EDGES_V1
                && proof.encoder_internal_edges.len() <= MAX_NVENC_COPY_EDGES_V1
                && (proof.status == CopyBoundaryStatusV1::BlockedUnknown || valid_copy_proof(proof))
        })
}

fn valid_config_proof(tuple: NvencTupleV1, proof: &NvencConfigProofV1) -> bool {
    proof.width_px == tuple.width_px()
        && proof.height_px == tuple.height_px()
        && proof.frame_rate == tuple.frame_rate()
        && proof.preset == NvencPresetV1::P2
        && proof.tuning == NvencTuningV1::UltraLowLatency
        && proof.synchronous
        && proof.picture_type_decision
        && proof.forced_keyframe
        && proof.output_parameter_sets
        && proof.b_frames == 0
        && proof.lookahead_depth == 0
        && proof.reorder_delay == 0
        && proof.one_frame_vbv
}

fn valid_resource_proof(tuple: NvencTupleV1, proof: &NvencResourceProofV1) -> bool {
    valid_resource_shape(tuple, proof)
        && proof.allocation_base_verified
        && proof.allocation_range_verified
        && proof.same_cuda_context
        && proof.same_gpu
        && proof.registered
        && proof.mapped
        && proof.submitted
        && proof.bitstream_locked
        && proof.bitstream_unlocked
}

fn valid_resource_shape(tuple: NvencTupleV1, proof: &NvencResourceProofV1) -> bool {
    let minimum_pitch = match tuple.buffer_format() {
        NvencBufferFormatV1::Nv12 | NvencBufferFormatV1::Yuv444 => u32::from(tuple.width_px()),
        NvencBufferFormatV1::Yuv420TenBit | NvencBufferFormatV1::Yuv444TenBit => {
            u32::from(tuple.width_px()) * 2
        }
    };
    let minimum_size = match tuple.buffer_format() {
        NvencBufferFormatV1::Nv12 => {
            u64::from(proof.pitch_bytes) * u64::from(tuple.height_px()) * 3 / 2
        }
        NvencBufferFormatV1::Yuv420TenBit => {
            u64::from(proof.pitch_bytes) * u64::from(tuple.height_px()) * 3 / 2
        }
        NvencBufferFormatV1::Yuv444 | NvencBufferFormatV1::Yuv444TenBit => {
            u64::from(proof.pitch_bytes) * u64::from(tuple.height_px()) * 3
        }
    };
    crate::output_mapping::validate_output_name_evidence(&proof.output_name)
        && valid_resource_identifier(&proof.gpu_pci_bdf)
        && valid_resource_identifier(&proof.gpu_uuid)
        && valid_source_identifier(&proof.lease_surface)
        && proof.width_px == tuple.width_px()
        && proof.height_px == tuple.height_px()
        && proof.pitch_bytes >= minimum_pitch
        && proof.buffer_format == tuple.buffer_format()
        && proof.allocation_byte_size >= minimum_size
}

fn valid_resource_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
}

fn source_authenticated(source: &NvencSourceEvidenceV1) -> bool {
    source_compiled(source)
        && source.runtime_library.as_deref() == Some("libnvidia-encode.so.1")
        && source.runtime_max_api_version.is_some_and(|runtime| {
            runtime.major > source.api_version.major
                || (runtime.major == source.api_version.major
                    && runtime.minor >= source.api_version.minor)
        })
        && source.runtime_max_api_version_raw.is_some()
        && source.minimum_linux_driver_major == Some(610)
        && source.nvidia_driver_major.is_some_and(|major| major >= 610)
        && source.minimum_cuda_driver_version == Some(13_010)
        && source
            .cuda_driver_version
            .is_some_and(|version| version >= 13_010)
}

fn source_compiled(source: &NvencSourceEvidenceV1) -> bool {
    source.api_version.is_sdk_13_1()
        && source.api_version_raw.is_some()
        && source.source_identity.as_deref() == Some(LIVE_NVENC_SOURCE_IDENTITY)
        && source.header_sha256.is_some()
        && source.minimum_linux_driver_major == Some(610)
        && source.minimum_cuda_driver_version == Some(13_010)
}

fn valid_copy_proof(proof: &CopyBoundaryProofV1) -> bool {
    proof.application_edges.len() <= MAX_NVENC_COPY_EDGES_V1
        && proof.encoder_internal_edges.len() <= MAX_NVENC_COPY_EDGES_V1
        && proof.status == CopyBoundaryStatusV1::Pass
        && proof.application_edges
            == [
                NvencCopyEdgeV1::RegisterNoCopy,
                NvencCopyEdgeV1::MapNoCopy,
                NvencCopyEdgeV1::SubmitNoCopy,
            ]
        && proof.encoder_internal_edges == [NvencCopyEdgeV1::SameGpuPitchLinearToBlockLinearCopy]
        && proof.host_staging_edges == 0
        && proof.cross_gpu_edges == 0
        && proof.unknown_application_edges == 0
        && proof.unknown_encoder_internal_edges == 0
}

fn valid_cleanup(cleanup: &NvencCleanupProofV1) -> bool {
    cleanup.complete
        && cleanup.acquired.len() <= MAX_NVENC_RESOURCE_EVENTS_V1
        && cleanup.released.len() <= MAX_NVENC_RESOURCE_EVENTS_V1
        && cleanup.released
            == cleanup
                .acquired
                .iter()
                .rev()
                .copied()
                .collect::<Vec<NvencResourceEventV1>>()
}

fn valid_source_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}
