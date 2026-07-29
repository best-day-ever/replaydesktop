use crate::model::{
    CaptureAdmissionV1, CaptureBoundaryStatusV1, CaptureCleanupLedgerV1, CaptureFailureV1,
    CaptureFrameLeaseV1, CaptureFrameObservationV1, CaptureGpuIdentityV1, CaptureLifecycleEventV1,
    CapturePathEvidenceV1, CapturePixelFormatV1, CapturePrimitiveObservationV1,
    CaptureProviderKindV1, CaptureSourceEvidenceV1, CaptureSourceStatusV1, CopyEdgeKindV1,
    CopyLedgerEdgeV1, CopyLedgerV1, MAX_CAPTURE_COPY_EDGES_V1, MAX_CAPTURE_LIFECYCLE_EVENTS_V1,
    MAX_CAPTURE_PLANES_V1, NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1, NVFBC_CAPTURE_SCHEMA_V1,
    SelectedOutputV1,
};
use std::collections::HashSet;
#[cfg(replay_nvfbc_source)]
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
#[cfg(replay_nvfbc_source)]
use std::path::{Path, PathBuf};
#[cfg(replay_nvfbc_source)]
use std::time::Instant;

const LIVE_SOURCE_IDENTITY: &str = "nvidia-nvfbc-api-1.9-cuda-driver-api-13.3";
const LIVE_NVFBC_API_VERSION: u32 = 0x109;
#[cfg(replay_nvfbc_source)]
const NVFBC_CREATE_HANDLE_PARAMS_VERSION: u32 = 0x0903_0070;
#[cfg(replay_nvfbc_source)]
const NVFBC_DESTROY_HANDLE_PARAMS_VERSION: u32 = 0x0901_0004;
#[cfg(replay_nvfbc_source)]
const NVFBC_GET_STATUS_PARAMS_VERSION: u32 = 0x0904_03a8;
#[cfg(replay_nvfbc_source)]
const NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VERSION: u32 = 0x0907_004c;
#[cfg(replay_nvfbc_source)]
const NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VERSION: u32 = 0x0901_0004;
#[cfg(replay_nvfbc_source)]
const NVFBC_TOCUDA_SETUP_PARAMS_VERSION: u32 = 0x0901_0008;
#[cfg(replay_nvfbc_source)]
const NVFBC_TOCUDA_GRAB_FRAME_PARAMS_VERSION: u32 = 0x0902_0020;
const FIXTURE_SOURCE_IDENTITY: &str = "fixture-nvfbc-api-1.8";
const FIXTURE_API_VERSION: u32 = 18;

#[cfg(test)]
const FULL_LIFECYCLE: [CaptureLifecycleEventV1; 9] = [
    CaptureLifecycleEventV1::LibraryLoaded,
    CaptureLifecycleEventV1::StatusQueried,
    CaptureLifecycleEventV1::ContextBound,
    CaptureLifecycleEventV1::SessionCreated,
    CaptureLifecycleEventV1::FrameGrabbed,
    CaptureLifecycleEventV1::FrameReleased,
    CaptureLifecycleEventV1::SessionDestroyed,
    CaptureLifecycleEventV1::ContextReleased,
    CaptureLifecycleEventV1::LibraryUnloaded,
];

const NVFBC_ABI_CONTRACT_NAMES: [&str; 15] = [
    "NVFBC_API_FUNCTION_LIST",
    "NVFBC_CREATE_HANDLE_PARAMS",
    "NVFBC_GET_STATUS_PARAMS",
    "NVFBC_BIND_CONTEXT_PARAMS",
    "NVFBC_CREATE_CAPTURE_SESSION_PARAMS",
    "NVFBC_TOCUDA_SETUP_PARAMS",
    "NVFBC_TOCUDA_GRAB_FRAME_PARAMS",
    "NVFBC_FRAME_GRAB_INFO",
    "NVFBC_DESTROY_CAPTURE_SESSION_PARAMS",
    "NVFBC_RELEASE_CONTEXT_PARAMS",
    "NVFBC_DESTROY_HANDLE_PARAMS",
    "CUcontext",
    "CUdeviceptr",
    "CUuuid",
    "cudaTypedefs.h",
];

pub const fn nvfbc_abi_contract_names() -> &'static [&'static str] {
    &NVFBC_ABI_CONTRACT_NAMES
}

#[cfg(not(replay_nvfbc_source))]
pub fn compiled_capture_source() -> CaptureSourceEvidenceV1 {
    CaptureSourceEvidenceV1 {
        status: CaptureSourceStatusV1::Unavailable,
        identity: None,
        api_version: None,
        nvfbc_header_sha256: None,
        cuda_header_sha256: None,
        cuda_typedefs_header_sha256: None,
        nvfbc_runtime_library: None,
        cuda_runtime_library: None,
        cuda_driver_version: None,
    }
}

#[cfg(replay_nvfbc_source)]
pub fn compiled_capture_source() -> CaptureSourceEvidenceV1 {
    CaptureSourceEvidenceV1 {
        status: CaptureSourceStatusV1::Authenticated,
        identity: Some(env!("REPLAY_NVFBC_SOURCE_IDENTITY").to_owned()),
        api_version: Some(LIVE_NVFBC_API_VERSION),
        nvfbc_header_sha256: Some(
            env!("REPLAY_NVFBC_SOURCE_SHA256")
                .parse()
                .expect("build script emits a checked NvFBC SHA-256"),
        ),
        cuda_header_sha256: Some(
            env!("REPLAY_CUDA_SOURCE_SHA256")
                .parse()
                .expect("build script emits a checked CUDA SHA-256"),
        ),
        cuda_typedefs_header_sha256: Some(
            env!("REPLAY_CUDA_TYPEDEFS_SOURCE_SHA256")
                .parse()
                .expect("build script emits a checked CUDA typedefs SHA-256"),
        ),
        nvfbc_runtime_library: None,
        cuda_runtime_library: None,
        cuda_driver_version: None,
    }
}

pub trait CaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1;
}

#[derive(Debug, Clone)]
pub struct FixtureCaptureProvider {
    observation: CapturePrimitiveObservationV1,
}

impl FixtureCaptureProvider {
    pub fn new(observation: CapturePrimitiveObservationV1) -> Self {
        Self { observation }
    }
}

impl CaptureProvider for FixtureCaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1 {
        self.observation.clone()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LiveUnavailableCaptureProvider;

impl CaptureProvider for LiveUnavailableCaptureProvider {
    fn observe(&self) -> CapturePrimitiveObservationV1 {
        CapturePrimitiveObservationV1 {
            schema: NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1.to_owned(),
            provider: CaptureProviderKindV1::LiveUnavailable,
            source: CaptureSourceEvidenceV1 {
                status: CaptureSourceStatusV1::Unavailable,
                identity: None,
                api_version: None,
                nvfbc_header_sha256: None,
                cuda_header_sha256: None,
                cuda_typedefs_header_sha256: None,
                nvfbc_runtime_library: None,
                cuda_runtime_library: None,
                cuda_driver_version: None,
            },
            binding: None,
            frame: None,
            lifecycle: Vec::new(),
            failure: Some(CaptureFailureV1::SourceUnavailable),
            nvfbc_status_raw: None,
        }
    }
}

pub fn observe_live_selected_capture(
    requested_output: &crate::model::OutputNameV1,
) -> CapturePrimitiveObservationV1 {
    #[cfg(not(replay_nvfbc_source))]
    {
        let _ = requested_output;
        LiveUnavailableCaptureProvider.observe()
    }
    #[cfg(replay_nvfbc_source)]
    {
        observe_source_authenticated_capture(requested_output)
    }
}

#[cfg(replay_nvfbc_source)]
fn observe_source_authenticated_capture(
    requested_output: &crate::model::OutputNameV1,
) -> CapturePrimitiveObservationV1 {
    let mut source = compiled_capture_source();
    if authenticate_runtime_sources(&source).is_err() || verify_compiled_abi().is_err() {
        return rejected_live_observation(
            source,
            CaptureFailureV1::SourceMismatch,
            None,
            Vec::new(),
            None,
        );
    }
    let before_observation =
        crate::output_mapping::collect_live_output_topology(Some(requested_output.clone()));
    let before = match before_observation
        .topology
        .as_ref()
        .and_then(|topology| crate::output_mapping::prove_output_gpu_mapping(topology).ok())
    {
        Some(selected) if before_observation.collection_failure.is_none() => selected,
        _ => {
            return rejected_live_observation(
                source,
                CaptureFailureV1::BindingMismatch,
                None,
                Vec::new(),
                None,
            );
        }
    };

    let mut observation = capture_native_one_frame(&before, source.clone());
    source = observation.source.clone();
    let after_observation =
        crate::output_mapping::collect_live_output_topology(Some(requested_output.clone()));
    let after = after_observation
        .topology
        .as_ref()
        .and_then(|topology| crate::output_mapping::prove_output_gpu_mapping(topology).ok());
    if after_observation.collection_failure.is_some()
        || after
            .as_ref()
            .is_none_or(|after| !same_selected_capture_identity(&before, after))
    {
        observation.failure = Some(CaptureFailureV1::BindingMismatch);
        observation.frame = None;
        observation.source = source;
    }
    observation
}

#[cfg(replay_nvfbc_source)]
fn same_selected_capture_identity(before: &SelectedOutputV1, after: &SelectedOutputV1) -> bool {
    before.output_name == after.output_name
        && before.randr_output_xid == after.randr_output_xid
        && before.randr_crtc_xid == after.randr_crtc_xid
        && before.randr_mode_xid == after.randr_mode_xid
        && before.randr_provider_xid == after.randr_provider_xid
        && before.randr_timestamp == after.randr_timestamp
        && before.randr_config_timestamp == after.randr_config_timestamp
        && before.width_px == after.width_px
        && before.height_px == after.height_px
        && before.origin_x == after.origin_x
        && before.origin_y == after.origin_y
        && before.exact_timing == after.exact_timing
        && before.nvcontrol_display_target_id == after.nvcontrol_display_target_id
        && before.nvcontrol_gpu_target_id == after.nvcontrol_gpu_target_id
        && before.nvml_pci_bdf == after.nvml_pci_bdf
        && before.nvml_uuid == after.nvml_uuid
        && before.topology_token == after.topology_token
}

#[cfg(replay_nvfbc_source)]
fn rejected_live_observation(
    source: CaptureSourceEvidenceV1,
    failure: CaptureFailureV1,
    binding: Option<crate::model::CaptureOutputBindingV1>,
    lifecycle: Vec<CaptureLifecycleEventV1>,
    nvfbc_status_raw: Option<i32>,
) -> CapturePrimitiveObservationV1 {
    CapturePrimitiveObservationV1 {
        schema: NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1.to_owned(),
        provider: CaptureProviderKindV1::SourceAuthenticated,
        source,
        binding,
        frame: None,
        lifecycle,
        failure: Some(failure),
        nvfbc_status_raw,
    }
}

#[cfg(replay_nvfbc_source)]
fn authenticate_runtime_sources(source: &CaptureSourceEvidenceV1) -> Result<(), ()> {
    authenticate_runtime_header(
        "REPLAY_NVFBC_SDK_ROOT",
        "NvFBC.h",
        source.nvfbc_header_sha256,
    )?;
    authenticate_runtime_header("REPLAY_CUDA_SDK_ROOT", "cuda.h", source.cuda_header_sha256)?;
    authenticate_runtime_header(
        "REPLAY_CUDA_SDK_ROOT",
        "cudaTypedefs.h",
        source.cuda_typedefs_header_sha256,
    )
}

#[cfg(replay_nvfbc_source)]
fn authenticate_runtime_header(
    root_variable: &str,
    name: &str,
    expected: Option<crate::Sha256DigestV1>,
) -> Result<(), ()> {
    let root = std::env::var_os(root_variable)
        .map(PathBuf::from)
        .ok_or(())?;
    if !root.is_absolute() {
        return Err(());
    }
    let root = std::fs::canonicalize(root).map_err(|_| ())?;
    let header = root.join(name);
    let link_metadata = std::fs::symlink_metadata(&header).map_err(|_| ())?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err(());
    }
    let header = std::fs::canonicalize(header).map_err(|_| ())?;
    let metadata = std::fs::metadata(&header).map_err(|_| ())?;
    if !header.starts_with(&root)
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > 4 * 1024 * 1024
    {
        return Err(());
    }
    let observed = crate::sha256_file(&header).map_err(|_| ())?;
    (Some(observed) == expected).then_some(()).ok_or(())
}

#[cfg(replay_nvfbc_source)]
const CUDA_LIBRARY_NAME: &str = "libcuda.so.1";
#[cfg(replay_nvfbc_source)]
const NVFBC_LIBRARY_NAME: &str = "libnvidia-fbc.so.1";
#[cfg(replay_nvfbc_source)]
const CUDA_SUCCESS: c_int = 0;
#[cfg(replay_nvfbc_source)]
const NVFBC_SUCCESS: c_int = 0;
#[cfg(replay_nvfbc_source)]
const NVFBC_CAPTURE_SHARED_CUDA: c_int = 1;
#[cfg(replay_nvfbc_source)]
const NVFBC_TRACKING_OUTPUT: c_int = 1;
#[cfg(replay_nvfbc_source)]
const NVFBC_BUFFER_FORMAT_NV12: c_int = 2;
#[cfg(replay_nvfbc_source)]
const NVFBC_BACKEND_X11: c_int = 1;
#[cfg(replay_nvfbc_source)]
const NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY: u32 = 4;
#[cfg(replay_nvfbc_source)]
const NVFBC_GRAB_TIMEOUT_MS: u32 = 750;
#[cfg(replay_nvfbc_source)]
const CUDA_GET_PROC_ADDRESS_LEGACY_STREAM: u64 = 1;
#[cfg(replay_nvfbc_source)]
const CUDA_POINTER_ATTRIBUTE_CONTEXT: c_int = 1;
#[cfg(replay_nvfbc_source)]
const CUDA_POINTER_ATTRIBUTE_MEMORY_TYPE: c_int = 2;
#[cfg(replay_nvfbc_source)]
const CUDA_POINTER_ATTRIBUTE_DEVICE_ORDINAL: c_int = 9;
#[cfg(replay_nvfbc_source)]
const CUDA_MEMORYTYPE_DEVICE: c_uint = 2;
#[cfg(replay_nvfbc_source)]
const MAX_NATIVE_CALL_ELAPSED_NS: u128 = 500_000_000;
#[cfg(replay_nvfbc_source)]
const PCI_BUS_ID_BUFFER_LEN: usize = 32;

#[cfg(replay_nvfbc_source)]
type CuResult = c_int;
#[cfg(replay_nvfbc_source)]
type CuDevice = c_int;
#[cfg(replay_nvfbc_source)]
type CuContext = *mut c_void;
#[cfg(replay_nvfbc_source)]
type CuDevicePtr = u64;
#[cfg(replay_nvfbc_source)]
type NvFbcSessionHandle = u64;

#[cfg(replay_nvfbc_source)]
type CuGetProcAddressFn =
    unsafe extern "C" fn(*const c_char, *mut *mut c_void, c_int, u64, *mut c_int) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuInitFn = unsafe extern "C" fn(c_uint) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuDriverGetVersionFn = unsafe extern "C" fn(*mut c_int) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuDeviceGetByPciBusIdFn = unsafe extern "C" fn(*mut CuDevice, *const c_char) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuDeviceGetPciBusIdFn = unsafe extern "C" fn(*mut c_char, c_int, CuDevice) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuDeviceGetUuidFn = unsafe extern "C" fn(*mut CuUuid, CuDevice) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuCtxCreateFn = unsafe extern "C" fn(*mut CuContext, c_uint, CuDevice) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuCtxGetCurrentFn = unsafe extern "C" fn(*mut CuContext) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuCtxGetDeviceFn = unsafe extern "C" fn(*mut CuDevice) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuPointerGetAttributeFn = unsafe extern "C" fn(*mut c_void, c_int, CuDevicePtr) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuCtxDestroyFn = unsafe extern "C" fn(CuContext) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuMemAllocFn = unsafe extern "C" fn(*mut CuDevicePtr, usize) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuMemcpyDtoDFn = unsafe extern "C" fn(CuDevicePtr, CuDevicePtr, usize) -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuCtxSynchronizeFn = unsafe extern "C" fn() -> CuResult;
#[cfg(replay_nvfbc_source)]
type CuMemFreeFn = unsafe extern "C" fn(CuDevicePtr) -> CuResult;

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CuUuid {
    bytes: [c_char; 16],
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcBox {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcSize {
    w: u32,
    h: u32,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NvFbcRandrOutputInfo {
    id: u32,
    name: [c_char; 128],
    tracked_box: NvFbcBox,
}

#[cfg(replay_nvfbc_source)]
impl Default for NvFbcRandrOutputInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: [0; 128],
            tracked_box: NvFbcBox::default(),
        }
    }
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NvFbcCreateHandleParams {
    version: u32,
    private_data: *const c_void,
    private_data_size: u32,
    externally_managed_context: c_int,
    glx_context: *mut c_void,
    glx_fb_config: *mut c_void,
    use_egl: c_int,
    backend: c_int,
    portal_restore_token: [c_char; 64],
}

#[cfg(replay_nvfbc_source)]
impl Default for NvFbcCreateHandleParams {
    fn default() -> Self {
        Self {
            version: 0,
            private_data: std::ptr::null(),
            private_data_size: 0,
            externally_managed_context: 0,
            glx_context: std::ptr::null_mut(),
            glx_fb_config: std::ptr::null_mut(),
            use_egl: 0,
            backend: NVFBC_BACKEND_X11,
            portal_restore_token: [0; 64],
        }
    }
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NvFbcGetStatusParams {
    version: u32,
    is_capture_possible: c_int,
    currently_capturing: c_int,
    can_create_now: c_int,
    screen_size: NvFbcSize,
    xrandr_available: c_int,
    outputs: [NvFbcRandrOutputInfo; 5],
    output_count: u32,
    nvfbc_version: u32,
    in_modeset: c_int,
    portal_restore_token: [c_char; 64],
    pid: u32,
    dbus_timeout_ms: c_int,
    capture_target_count: u32,
    capture_target_sizes: [NvFbcSize; 10],
}

#[cfg(replay_nvfbc_source)]
impl Default for NvFbcGetStatusParams {
    fn default() -> Self {
        Self {
            version: 0,
            is_capture_possible: 0,
            currently_capturing: 0,
            can_create_now: 0,
            screen_size: NvFbcSize::default(),
            xrandr_available: 0,
            outputs: [NvFbcRandrOutputInfo::default(); 5],
            output_count: 0,
            nvfbc_version: 0,
            in_modeset: 0,
            portal_restore_token: [0; 64],
            pid: 0,
            dbus_timeout_ms: 500,
            capture_target_count: 0,
            capture_target_sizes: [NvFbcSize::default(); 10],
        }
    }
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcCreateCaptureSessionParams {
    version: u32,
    capture_type: c_int,
    tracking_type: c_int,
    output_id: u32,
    capture_box: NvFbcBox,
    frame_size: NvFbcSize,
    with_cursor: c_int,
    disable_auto_modeset_recovery: c_int,
    round_frame_size: c_int,
    sampling_rate_ms: u32,
    push_model: c_int,
    allow_direct_capture: c_int,
    pid: u32,
    dbus_timeout_ms: c_int,
    capture_target: u32,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcToCudaSetupParams {
    version: u32,
    buffer_format: c_int,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcFrameGrabInfo {
    width: u32,
    height: u32,
    byte_size: u32,
    current_frame: u32,
    is_new_frame: c_int,
    timestamp_us: u64,
    missed_frames: u32,
    required_post_processing: c_int,
    direct_capture: c_int,
    cursor_visible: c_int,
    cursor_composited: c_int,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NvFbcToCudaGrabFrameParams {
    version: u32,
    flags: u32,
    cuda_device_buffer: *mut c_void,
    frame_info: *mut NvFbcFrameGrabInfo,
    timeout_ms: u32,
}

#[cfg(replay_nvfbc_source)]
impl Default for NvFbcToCudaGrabFrameParams {
    fn default() -> Self {
        Self {
            version: 0,
            flags: 0,
            cuda_device_buffer: std::ptr::null_mut(),
            frame_info: std::ptr::null_mut(),
            timeout_ms: 0,
        }
    }
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvFbcVersionOnlyParams {
    version: u32,
}

#[cfg(replay_nvfbc_source)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NvFbcApiFunctionList {
    version: u32,
    alignment_padding: u32,
    entries: [*mut c_void; 22],
}

#[cfg(replay_nvfbc_source)]
impl Default for NvFbcApiFunctionList {
    fn default() -> Self {
        Self {
            version: LIVE_NVFBC_API_VERSION,
            alignment_padding: 0,
            entries: [std::ptr::null_mut(); 22],
        }
    }
}

#[cfg(replay_nvfbc_source)]
fn validate_nvfbc_function_list(
    functions: &NvFbcApiFunctionList,
) -> Result<(), NativeCaptureError> {
    if functions.version != LIVE_NVFBC_API_VERSION
        || [1_usize, 2, 3, 4, 5, 8, 9]
            .into_iter()
            .any(|index| functions.entries[index].is_null())
    {
        return Err(NativeCaptureError::nvfbc(CaptureFailureV1::ApiMismatch, 1));
    }
    Ok(())
}

#[cfg(replay_nvfbc_source)]
macro_rules! declare_oracle_usize_functions {
    ($($name:ident),+ $(,)?) => {
        $(fn $name() -> usize;)+
    };
}

#[cfg(replay_nvfbc_source)]
macro_rules! declare_oracle_u32_functions {
    ($($name:ident),+ $(,)?) => {
        $(fn $name() -> c_uint;)+
    };
}

#[cfg(replay_nvfbc_source)]
#[link(name = "replay_nvfbc_abi_oracle", kind = "static")]
unsafe extern "C" {
    declare_oracle_usize_functions!(
        replay_nvfbc_sizeof_box,
        replay_nvfbc_alignof_box,
        replay_nvfbc_offsetof_box_x,
        replay_nvfbc_offsetof_box_y,
        replay_nvfbc_offsetof_box_w,
        replay_nvfbc_offsetof_box_h,
        replay_nvfbc_sizeof_size,
        replay_nvfbc_alignof_size,
        replay_nvfbc_offsetof_size_w,
        replay_nvfbc_offsetof_size_h,
        replay_nvfbc_sizeof_randr_output_info,
        replay_nvfbc_alignof_randr_output_info,
        replay_nvfbc_offsetof_randr_output_info_dwId,
        replay_nvfbc_offsetof_randr_output_info_name,
        replay_nvfbc_offsetof_randr_output_info_trackedBox,
        replay_nvfbc_sizeof_frame_grab_info,
        replay_nvfbc_alignof_frame_grab_info,
        replay_nvfbc_offsetof_frame_grab_info_dwWidth,
        replay_nvfbc_offsetof_frame_grab_info_dwHeight,
        replay_nvfbc_offsetof_frame_grab_info_dwByteSize,
        replay_nvfbc_offsetof_frame_grab_info_dwCurrentFrame,
        replay_nvfbc_offsetof_frame_grab_info_bIsNewFrame,
        replay_nvfbc_offsetof_frame_grab_info_ulTimestampUs,
        replay_nvfbc_offsetof_frame_grab_info_dwMissedFrames,
        replay_nvfbc_offsetof_frame_grab_info_bRequiredPostProcessing,
        replay_nvfbc_offsetof_frame_grab_info_bDirectCapture,
        replay_nvfbc_offsetof_frame_grab_info_bCursorVisible,
        replay_nvfbc_offsetof_frame_grab_info_bCursorComposited,
        replay_nvfbc_sizeof_create_handle_params,
        replay_nvfbc_alignof_create_handle_params,
        replay_nvfbc_offsetof_create_handle_params_dwVersion,
        replay_nvfbc_offsetof_create_handle_params_privateData,
        replay_nvfbc_offsetof_create_handle_params_privateDataSize,
        replay_nvfbc_offsetof_create_handle_params_bExternallyManagedContext,
        replay_nvfbc_offsetof_create_handle_params_glxCtx,
        replay_nvfbc_offsetof_create_handle_params_glxFBConfig,
        replay_nvfbc_offsetof_create_handle_params_bUseEGL,
        replay_nvfbc_offsetof_create_handle_params_eBackend,
        replay_nvfbc_offsetof_create_handle_params_portalRestoreToken,
        replay_nvfbc_sizeof_get_status_params,
        replay_nvfbc_alignof_get_status_params,
        replay_nvfbc_offsetof_get_status_params_dwVersion,
        replay_nvfbc_offsetof_get_status_params_bIsCapturePossible,
        replay_nvfbc_offsetof_get_status_params_bCurrentlyCapturing,
        replay_nvfbc_offsetof_get_status_params_bCanCreateNow,
        replay_nvfbc_offsetof_get_status_params_screenSize,
        replay_nvfbc_offsetof_get_status_params_bXRandRAvailable,
        replay_nvfbc_offsetof_get_status_params_outputs,
        replay_nvfbc_offsetof_get_status_params_dwOutputNum,
        replay_nvfbc_offsetof_get_status_params_dwNvFBCVersion,
        replay_nvfbc_offsetof_get_status_params_bInModeset,
        replay_nvfbc_offsetof_get_status_params_portalRestoreToken,
        replay_nvfbc_offsetof_get_status_params_dwPid,
        replay_nvfbc_offsetof_get_status_params_dwDbusTimeoutMs,
        replay_nvfbc_offsetof_get_status_params_dwCaptureTargetCount,
        replay_nvfbc_offsetof_get_status_params_captureTargetSizes,
        replay_nvfbc_sizeof_create_capture_session_params,
        replay_nvfbc_alignof_create_capture_session_params,
        replay_nvfbc_offsetof_create_capture_session_params_dwVersion,
        replay_nvfbc_offsetof_create_capture_session_params_eCaptureType,
        replay_nvfbc_offsetof_create_capture_session_params_eTrackingType,
        replay_nvfbc_offsetof_create_capture_session_params_dwOutputId,
        replay_nvfbc_offsetof_create_capture_session_params_captureBox,
        replay_nvfbc_offsetof_create_capture_session_params_frameSize,
        replay_nvfbc_offsetof_create_capture_session_params_bWithCursor,
        replay_nvfbc_offsetof_create_capture_session_params_bDisableAutoModesetRecovery,
        replay_nvfbc_offsetof_create_capture_session_params_bRoundFrameSize,
        replay_nvfbc_offsetof_create_capture_session_params_dwSamplingRateMs,
        replay_nvfbc_offsetof_create_capture_session_params_bPushModel,
        replay_nvfbc_offsetof_create_capture_session_params_bAllowDirectCapture,
        replay_nvfbc_offsetof_create_capture_session_params_dwPid,
        replay_nvfbc_offsetof_create_capture_session_params_dwDbusTimeoutMs,
        replay_nvfbc_offsetof_create_capture_session_params_dwCaptureTarget,
        replay_nvfbc_sizeof_tocuda_setup_params,
        replay_nvfbc_alignof_tocuda_setup_params,
        replay_nvfbc_offsetof_tocuda_setup_params_dwVersion,
        replay_nvfbc_offsetof_tocuda_setup_params_eBufferFormat,
        replay_nvfbc_sizeof_tocuda_grab_frame_params,
        replay_nvfbc_alignof_tocuda_grab_frame_params,
        replay_nvfbc_offsetof_tocuda_grab_frame_params_dwVersion,
        replay_nvfbc_offsetof_tocuda_grab_frame_params_dwFlags,
        replay_nvfbc_offsetof_tocuda_grab_frame_params_pCUDADeviceBuffer,
        replay_nvfbc_offsetof_tocuda_grab_frame_params_pFrameGrabInfo,
        replay_nvfbc_offsetof_tocuda_grab_frame_params_dwTimeoutMs,
        replay_nvfbc_sizeof_destroy_handle_params,
        replay_nvfbc_alignof_destroy_handle_params,
        replay_nvfbc_offsetof_destroy_handle_params_dwVersion,
        replay_nvfbc_sizeof_destroy_capture_session_params,
        replay_nvfbc_alignof_destroy_capture_session_params,
        replay_nvfbc_offsetof_destroy_capture_session_params_dwVersion,
        replay_nvfbc_sizeof_api_function_list,
        replay_nvfbc_alignof_api_function_list,
        replay_nvfbc_offsetof_api_function_list_dwVersion,
        replay_nvfbc_offsetof_api_function_list_nvFBCGetLastErrorStr,
        replay_nvfbc_offsetof_api_function_list_nvFBCCreateHandle,
        replay_nvfbc_offsetof_api_function_list_nvFBCDestroyHandle,
        replay_nvfbc_offsetof_api_function_list_nvFBCGetStatus,
        replay_nvfbc_offsetof_api_function_list_nvFBCCreateCaptureSession,
        replay_nvfbc_offsetof_api_function_list_nvFBCDestroyCaptureSession,
        replay_nvfbc_offsetof_api_function_list_nvFBCToSysSetUp,
        replay_nvfbc_offsetof_api_function_list_nvFBCToSysGrabFrame,
        replay_nvfbc_offsetof_api_function_list_nvFBCToCudaSetUp,
        replay_nvfbc_offsetof_api_function_list_nvFBCToCudaGrabFrame,
        replay_nvfbc_offsetof_api_function_list_pad1,
        replay_nvfbc_offsetof_api_function_list_pad2,
        replay_nvfbc_offsetof_api_function_list_pad3,
        replay_nvfbc_offsetof_api_function_list_nvFBCBindContext,
        replay_nvfbc_offsetof_api_function_list_nvFBCReleaseContext,
        replay_nvfbc_offsetof_api_function_list_pad4,
        replay_nvfbc_offsetof_api_function_list_pad5,
        replay_nvfbc_offsetof_api_function_list_pad6,
        replay_nvfbc_offsetof_api_function_list_pad7,
        replay_nvfbc_offsetof_api_function_list_nvFBCToGLSetUp,
        replay_nvfbc_offsetof_api_function_list_nvFBCToGLGrabFrame,
        replay_nvfbc_offsetof_api_function_list_nvFBCCompositeCursor,
        replay_nvfbc_offset_api_composite_cursor,
        replay_nvfbc_sizeof_session_handle,
        replay_nvfbc_alignof_session_handle,
        replay_nvfbc_sizeof_status,
        replay_nvfbc_alignof_status,
        replay_nvfbc_sizeof_bool,
        replay_nvfbc_alignof_bool,
        replay_nvfbc_sizeof_capture_type,
        replay_nvfbc_alignof_capture_type,
        replay_nvfbc_sizeof_tracking_type,
        replay_nvfbc_alignof_tracking_type,
        replay_nvfbc_sizeof_buffer_format,
        replay_nvfbc_alignof_buffer_format,
        replay_nvfbc_sizeof_backend,
        replay_nvfbc_alignof_backend,
        replay_nvfbc_sizeof_tocuda_flags,
        replay_nvfbc_alignof_tocuda_flags,
        replay_nvfbc_sizeof_cuda_context,
        replay_nvfbc_alignof_cuda_context,
        replay_nvfbc_sizeof_cuda_device_pointer,
        replay_nvfbc_alignof_cuda_device_pointer,
        replay_nvfbc_sizeof_cuda_device,
        replay_nvfbc_alignof_cuda_device,
        replay_nvfbc_sizeof_cuda_uuid,
        replay_nvfbc_alignof_cuda_uuid,
        replay_nvfbc_sizeof_cuda_result,
        replay_nvfbc_alignof_cuda_result,
    );
    declare_oracle_u32_functions!(
        replay_nvfbc_api_version_major,
        replay_nvfbc_api_version_minor,
        replay_nvfbc_api_version,
        replay_nvfbc_ver_create_handle_params,
        replay_nvfbc_ver_destroy_handle_params,
        replay_nvfbc_ver_get_status_params,
        replay_nvfbc_ver_create_capture_session_params,
        replay_nvfbc_ver_destroy_capture_session_params,
        replay_nvfbc_ver_tocuda_setup_params,
        replay_nvfbc_ver_tocuda_grab_frame_params,
        replay_nvfbc_bool_false,
        replay_nvfbc_bool_true,
        replay_nvfbc_status_success,
        replay_nvfbc_status_err_api_version,
        replay_nvfbc_status_err_internal,
        replay_nvfbc_status_err_invalid_param,
        replay_nvfbc_status_err_invalid_ptr,
        replay_nvfbc_status_err_invalid_handle,
        replay_nvfbc_status_err_max_clients,
        replay_nvfbc_status_err_unsupported,
        replay_nvfbc_status_err_out_of_memory,
        replay_nvfbc_status_err_bad_request,
        replay_nvfbc_status_err_x,
        replay_nvfbc_status_err_glx,
        replay_nvfbc_status_err_gl,
        replay_nvfbc_status_err_cuda,
        replay_nvfbc_status_err_encoder,
        replay_nvfbc_status_err_context,
        replay_nvfbc_status_err_must_recreate,
        replay_nvfbc_status_err_vulkan,
        replay_nvfbc_status_err_egl,
        replay_nvfbc_status_err_dbus,
        replay_nvfbc_status_err_pipewire,
        replay_nvfbc_status_err_drm,
        replay_nvfbc_capture_to_sys,
        replay_nvfbc_capture_shared_cuda,
        replay_nvfbc_capture_to_gl,
        replay_nvfbc_tracking_default,
        replay_nvfbc_tracking_output,
        replay_nvfbc_tracking_screen,
        replay_nvfbc_buffer_format_argb,
        replay_nvfbc_buffer_format_rgb,
        replay_nvfbc_buffer_format_nv12,
        replay_nvfbc_buffer_format_yuv444p,
        replay_nvfbc_buffer_format_rgba,
        replay_nvfbc_buffer_format_bgra,
        replay_nvfbc_backend_auto,
        replay_nvfbc_backend_x11,
        replay_nvfbc_backend_pipewire,
        replay_nvfbc_backend_direct,
        replay_nvfbc_tocuda_grab_flags_noflags,
        replay_nvfbc_tocuda_grab_flags_nowait,
        replay_nvfbc_tocuda_grab_flags_force_refresh,
        replay_nvfbc_tocuda_grab_flags_nowait_if_new_frame_ready,
        replay_nvfbc_output_max,
        replay_nvfbc_output_name_len,
        replay_nvfbc_portal_restore_token_len,
        replay_nvfbc_direct_max_capture_targets,
        replay_nvfbc_contract_mask,
        replay_nvfbc_cuda_signature_mask,
        replay_cuda_header_version,
        replay_cuda_result_success,
        replay_cuda_get_proc_address_legacy_stream,
        replay_cuda_pointer_attribute_context,
        replay_cuda_pointer_attribute_memory_type,
        replay_cuda_pointer_attribute_device_ordinal,
        replay_cuda_memory_type_device,
        replay_cuda_abi_version_get_proc_address,
        replay_cuda_abi_version_init,
        replay_cuda_abi_version_driver_get_version,
        replay_cuda_abi_version_device_get_by_pci_bus_id,
        replay_cuda_abi_version_device_get_pci_bus_id,
        replay_cuda_abi_version_device_get_uuid_v2,
        replay_cuda_abi_version_ctx_create_v2,
        replay_cuda_abi_version_ctx_get_current,
        replay_cuda_abi_version_ctx_get_device,
        replay_cuda_abi_version_pointer_get_attribute,
        replay_cuda_abi_version_ctx_destroy_v2,
        replay_cuda_abi_version_mem_alloc_v2,
        replay_cuda_abi_version_memcpy_dtod_v2,
        replay_cuda_abi_version_ctx_synchronize,
        replay_cuda_abi_version_mem_free_v2,
        replay_cuda_signature_mask,
    );
}

#[cfg(replay_nvfbc_source)]
type NvFbcCreateInstanceFn = unsafe extern "C" fn(*mut NvFbcApiFunctionList) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcCreateHandleFn =
    unsafe extern "C" fn(*mut NvFbcSessionHandle, *mut NvFbcCreateHandleParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcDestroyHandleFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcVersionOnlyParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcGetStatusFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcGetStatusParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcCreateCaptureSessionFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcCreateCaptureSessionParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcDestroyCaptureSessionFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcVersionOnlyParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcToCudaSetupFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcToCudaSetupParams) -> c_int;
#[cfg(replay_nvfbc_source)]
type NvFbcToCudaGrabFrameFn =
    unsafe extern "C" fn(NvFbcSessionHandle, *mut NvFbcToCudaGrabFrameParams) -> c_int;

#[cfg(replay_nvfbc_source)]
fn verify_compiled_abi() -> Result<(), ()> {
    macro_rules! layout_matches {
        (
            $type:ty,
            $size_fn:ident,
            $align_fn:ident,
            $( $field:ident => $offset_fn:ident ),+ $(,)?
        ) => {
            $size_fn() == std::mem::size_of::<$type>()
                && $align_fn() == std::mem::align_of::<$type>()
                $(
                    && $offset_fn() == std::mem::offset_of!($type, $field)
                )+
        };
    }

    let matches = unsafe {
        // SAFETY: Every function below is a project-owned, zero-argument oracle
        // linked from native/nvfbc_abi_oracle.c. It returns only a source-header
        // compile-time integer and performs no native driver calls.
        let structure_layouts = layout_matches!(
            NvFbcBox,
            replay_nvfbc_sizeof_box,
            replay_nvfbc_alignof_box,
            x => replay_nvfbc_offsetof_box_x,
            y => replay_nvfbc_offsetof_box_y,
            w => replay_nvfbc_offsetof_box_w,
            h => replay_nvfbc_offsetof_box_h,
        ) && layout_matches!(
            NvFbcSize,
            replay_nvfbc_sizeof_size,
            replay_nvfbc_alignof_size,
            w => replay_nvfbc_offsetof_size_w,
            h => replay_nvfbc_offsetof_size_h,
        ) && layout_matches!(
            NvFbcRandrOutputInfo,
            replay_nvfbc_sizeof_randr_output_info,
            replay_nvfbc_alignof_randr_output_info,
            id => replay_nvfbc_offsetof_randr_output_info_dwId,
            name => replay_nvfbc_offsetof_randr_output_info_name,
            tracked_box => replay_nvfbc_offsetof_randr_output_info_trackedBox,
        ) && layout_matches!(
            NvFbcFrameGrabInfo,
            replay_nvfbc_sizeof_frame_grab_info,
            replay_nvfbc_alignof_frame_grab_info,
            width => replay_nvfbc_offsetof_frame_grab_info_dwWidth,
            height => replay_nvfbc_offsetof_frame_grab_info_dwHeight,
            byte_size => replay_nvfbc_offsetof_frame_grab_info_dwByteSize,
            current_frame => replay_nvfbc_offsetof_frame_grab_info_dwCurrentFrame,
            is_new_frame => replay_nvfbc_offsetof_frame_grab_info_bIsNewFrame,
            timestamp_us => replay_nvfbc_offsetof_frame_grab_info_ulTimestampUs,
            missed_frames => replay_nvfbc_offsetof_frame_grab_info_dwMissedFrames,
            required_post_processing =>
                replay_nvfbc_offsetof_frame_grab_info_bRequiredPostProcessing,
            direct_capture => replay_nvfbc_offsetof_frame_grab_info_bDirectCapture,
            cursor_visible => replay_nvfbc_offsetof_frame_grab_info_bCursorVisible,
            cursor_composited => replay_nvfbc_offsetof_frame_grab_info_bCursorComposited,
        ) && layout_matches!(
            NvFbcCreateHandleParams,
            replay_nvfbc_sizeof_create_handle_params,
            replay_nvfbc_alignof_create_handle_params,
            version => replay_nvfbc_offsetof_create_handle_params_dwVersion,
            private_data => replay_nvfbc_offsetof_create_handle_params_privateData,
            private_data_size => replay_nvfbc_offsetof_create_handle_params_privateDataSize,
            externally_managed_context =>
                replay_nvfbc_offsetof_create_handle_params_bExternallyManagedContext,
            glx_context => replay_nvfbc_offsetof_create_handle_params_glxCtx,
            glx_fb_config => replay_nvfbc_offsetof_create_handle_params_glxFBConfig,
            use_egl => replay_nvfbc_offsetof_create_handle_params_bUseEGL,
            backend => replay_nvfbc_offsetof_create_handle_params_eBackend,
            portal_restore_token =>
                replay_nvfbc_offsetof_create_handle_params_portalRestoreToken,
        ) && layout_matches!(
            NvFbcGetStatusParams,
            replay_nvfbc_sizeof_get_status_params,
            replay_nvfbc_alignof_get_status_params,
            version => replay_nvfbc_offsetof_get_status_params_dwVersion,
            is_capture_possible =>
                replay_nvfbc_offsetof_get_status_params_bIsCapturePossible,
            currently_capturing =>
                replay_nvfbc_offsetof_get_status_params_bCurrentlyCapturing,
            can_create_now => replay_nvfbc_offsetof_get_status_params_bCanCreateNow,
            screen_size => replay_nvfbc_offsetof_get_status_params_screenSize,
            xrandr_available => replay_nvfbc_offsetof_get_status_params_bXRandRAvailable,
            outputs => replay_nvfbc_offsetof_get_status_params_outputs,
            output_count => replay_nvfbc_offsetof_get_status_params_dwOutputNum,
            nvfbc_version => replay_nvfbc_offsetof_get_status_params_dwNvFBCVersion,
            in_modeset => replay_nvfbc_offsetof_get_status_params_bInModeset,
            portal_restore_token =>
                replay_nvfbc_offsetof_get_status_params_portalRestoreToken,
            pid => replay_nvfbc_offsetof_get_status_params_dwPid,
            dbus_timeout_ms => replay_nvfbc_offsetof_get_status_params_dwDbusTimeoutMs,
            capture_target_count =>
                replay_nvfbc_offsetof_get_status_params_dwCaptureTargetCount,
            capture_target_sizes =>
                replay_nvfbc_offsetof_get_status_params_captureTargetSizes,
        ) && layout_matches!(
            NvFbcCreateCaptureSessionParams,
            replay_nvfbc_sizeof_create_capture_session_params,
            replay_nvfbc_alignof_create_capture_session_params,
            version => replay_nvfbc_offsetof_create_capture_session_params_dwVersion,
            capture_type => replay_nvfbc_offsetof_create_capture_session_params_eCaptureType,
            tracking_type => replay_nvfbc_offsetof_create_capture_session_params_eTrackingType,
            output_id => replay_nvfbc_offsetof_create_capture_session_params_dwOutputId,
            capture_box => replay_nvfbc_offsetof_create_capture_session_params_captureBox,
            frame_size => replay_nvfbc_offsetof_create_capture_session_params_frameSize,
            with_cursor => replay_nvfbc_offsetof_create_capture_session_params_bWithCursor,
            disable_auto_modeset_recovery =>
                replay_nvfbc_offsetof_create_capture_session_params_bDisableAutoModesetRecovery,
            round_frame_size =>
                replay_nvfbc_offsetof_create_capture_session_params_bRoundFrameSize,
            sampling_rate_ms =>
                replay_nvfbc_offsetof_create_capture_session_params_dwSamplingRateMs,
            push_model => replay_nvfbc_offsetof_create_capture_session_params_bPushModel,
            allow_direct_capture =>
                replay_nvfbc_offsetof_create_capture_session_params_bAllowDirectCapture,
            pid => replay_nvfbc_offsetof_create_capture_session_params_dwPid,
            dbus_timeout_ms =>
                replay_nvfbc_offsetof_create_capture_session_params_dwDbusTimeoutMs,
            capture_target =>
                replay_nvfbc_offsetof_create_capture_session_params_dwCaptureTarget,
        ) && layout_matches!(
            NvFbcToCudaSetupParams,
            replay_nvfbc_sizeof_tocuda_setup_params,
            replay_nvfbc_alignof_tocuda_setup_params,
            version => replay_nvfbc_offsetof_tocuda_setup_params_dwVersion,
            buffer_format => replay_nvfbc_offsetof_tocuda_setup_params_eBufferFormat,
        ) && layout_matches!(
            NvFbcToCudaGrabFrameParams,
            replay_nvfbc_sizeof_tocuda_grab_frame_params,
            replay_nvfbc_alignof_tocuda_grab_frame_params,
            version => replay_nvfbc_offsetof_tocuda_grab_frame_params_dwVersion,
            flags => replay_nvfbc_offsetof_tocuda_grab_frame_params_dwFlags,
            cuda_device_buffer =>
                replay_nvfbc_offsetof_tocuda_grab_frame_params_pCUDADeviceBuffer,
            frame_info => replay_nvfbc_offsetof_tocuda_grab_frame_params_pFrameGrabInfo,
            timeout_ms => replay_nvfbc_offsetof_tocuda_grab_frame_params_dwTimeoutMs,
        ) && replay_nvfbc_sizeof_destroy_handle_params()
            == std::mem::size_of::<NvFbcVersionOnlyParams>()
            && replay_nvfbc_alignof_destroy_handle_params()
                == std::mem::align_of::<NvFbcVersionOnlyParams>()
            && replay_nvfbc_offsetof_destroy_handle_params_dwVersion()
                == std::mem::offset_of!(NvFbcVersionOnlyParams, version)
            && replay_nvfbc_sizeof_destroy_capture_session_params()
                == std::mem::size_of::<NvFbcVersionOnlyParams>()
            && replay_nvfbc_alignof_destroy_capture_session_params()
                == std::mem::align_of::<NvFbcVersionOnlyParams>()
            && replay_nvfbc_offsetof_destroy_capture_session_params_dwVersion()
                == std::mem::offset_of!(NvFbcVersionOnlyParams, version);

        let scalar_layouts = [
            (
                replay_nvfbc_sizeof_session_handle(),
                std::mem::size_of::<NvFbcSessionHandle>(),
                replay_nvfbc_alignof_session_handle(),
                std::mem::align_of::<NvFbcSessionHandle>(),
            ),
            (
                replay_nvfbc_sizeof_status(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_status(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_bool(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_bool(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_capture_type(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_capture_type(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_tracking_type(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_tracking_type(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_buffer_format(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_buffer_format(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_backend(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_backend(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_tocuda_flags(),
                std::mem::size_of::<c_int>(),
                replay_nvfbc_alignof_tocuda_flags(),
                std::mem::align_of::<c_int>(),
            ),
            (
                replay_nvfbc_sizeof_cuda_context(),
                std::mem::size_of::<CuContext>(),
                replay_nvfbc_alignof_cuda_context(),
                std::mem::align_of::<CuContext>(),
            ),
            (
                replay_nvfbc_sizeof_cuda_device_pointer(),
                std::mem::size_of::<CuDevicePtr>(),
                replay_nvfbc_alignof_cuda_device_pointer(),
                std::mem::align_of::<CuDevicePtr>(),
            ),
            (
                replay_nvfbc_sizeof_cuda_device(),
                std::mem::size_of::<CuDevice>(),
                replay_nvfbc_alignof_cuda_device(),
                std::mem::align_of::<CuDevice>(),
            ),
            (
                replay_nvfbc_sizeof_cuda_uuid(),
                std::mem::size_of::<CuUuid>(),
                replay_nvfbc_alignof_cuda_uuid(),
                std::mem::align_of::<CuUuid>(),
            ),
            (
                replay_nvfbc_sizeof_cuda_result(),
                std::mem::size_of::<CuResult>(),
                replay_nvfbc_alignof_cuda_result(),
                std::mem::align_of::<CuResult>(),
            ),
        ]
        .into_iter()
        .all(|(source_size, rust_size, source_align, rust_align)| {
            source_size == rust_size && source_align == rust_align
        });

        let function_list_offsets = [
            replay_nvfbc_offsetof_api_function_list_nvFBCGetLastErrorStr(),
            replay_nvfbc_offsetof_api_function_list_nvFBCCreateHandle(),
            replay_nvfbc_offsetof_api_function_list_nvFBCDestroyHandle(),
            replay_nvfbc_offsetof_api_function_list_nvFBCGetStatus(),
            replay_nvfbc_offsetof_api_function_list_nvFBCCreateCaptureSession(),
            replay_nvfbc_offsetof_api_function_list_nvFBCDestroyCaptureSession(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToSysSetUp(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToSysGrabFrame(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToCudaSetUp(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToCudaGrabFrame(),
            replay_nvfbc_offsetof_api_function_list_pad1(),
            replay_nvfbc_offsetof_api_function_list_pad2(),
            replay_nvfbc_offsetof_api_function_list_pad3(),
            replay_nvfbc_offsetof_api_function_list_nvFBCBindContext(),
            replay_nvfbc_offsetof_api_function_list_nvFBCReleaseContext(),
            replay_nvfbc_offsetof_api_function_list_pad4(),
            replay_nvfbc_offsetof_api_function_list_pad5(),
            replay_nvfbc_offsetof_api_function_list_pad6(),
            replay_nvfbc_offsetof_api_function_list_pad7(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToGLSetUp(),
            replay_nvfbc_offsetof_api_function_list_nvFBCToGLGrabFrame(),
            replay_nvfbc_offsetof_api_function_list_nvFBCCompositeCursor(),
        ];
        let entries_offset = std::mem::offset_of!(NvFbcApiFunctionList, entries);
        let pointer_size = std::mem::size_of::<*mut c_void>();
        let function_list_layout = replay_nvfbc_sizeof_api_function_list()
            == std::mem::size_of::<NvFbcApiFunctionList>()
            && replay_nvfbc_alignof_api_function_list()
                == std::mem::align_of::<NvFbcApiFunctionList>()
            && replay_nvfbc_offsetof_api_function_list_dwVersion()
                == std::mem::offset_of!(NvFbcApiFunctionList, version)
            && function_list_offsets
                .into_iter()
                .enumerate()
                .all(|(index, offset)| offset == entries_offset + index * pointer_size)
            && replay_nvfbc_offset_api_composite_cursor() == entries_offset + 21 * pointer_size;

        let status_values = [
            replay_nvfbc_status_success(),
            replay_nvfbc_status_err_api_version(),
            replay_nvfbc_status_err_internal(),
            replay_nvfbc_status_err_invalid_param(),
            replay_nvfbc_status_err_invalid_ptr(),
            replay_nvfbc_status_err_invalid_handle(),
            replay_nvfbc_status_err_max_clients(),
            replay_nvfbc_status_err_unsupported(),
            replay_nvfbc_status_err_out_of_memory(),
            replay_nvfbc_status_err_bad_request(),
            replay_nvfbc_status_err_x(),
            replay_nvfbc_status_err_glx(),
            replay_nvfbc_status_err_gl(),
            replay_nvfbc_status_err_cuda(),
            replay_nvfbc_status_err_encoder(),
            replay_nvfbc_status_err_context(),
            replay_nvfbc_status_err_must_recreate(),
            replay_nvfbc_status_err_vulkan(),
            replay_nvfbc_status_err_egl(),
            replay_nvfbc_status_err_dbus(),
            replay_nvfbc_status_err_pipewire(),
            replay_nvfbc_status_err_drm(),
        ];
        let values = replay_nvfbc_api_version_major() == 1
            && replay_nvfbc_api_version_minor() == 9
            && replay_nvfbc_api_version() == LIVE_NVFBC_API_VERSION
            && replay_nvfbc_ver_create_handle_params() == NVFBC_CREATE_HANDLE_PARAMS_VERSION
            && replay_nvfbc_ver_destroy_handle_params() == NVFBC_DESTROY_HANDLE_PARAMS_VERSION
            && replay_nvfbc_ver_get_status_params() == NVFBC_GET_STATUS_PARAMS_VERSION
            && replay_nvfbc_ver_create_capture_session_params()
                == NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VERSION
            && replay_nvfbc_ver_destroy_capture_session_params()
                == NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VERSION
            && replay_nvfbc_ver_tocuda_setup_params() == NVFBC_TOCUDA_SETUP_PARAMS_VERSION
            && replay_nvfbc_ver_tocuda_grab_frame_params()
                == NVFBC_TOCUDA_GRAB_FRAME_PARAMS_VERSION
            && replay_nvfbc_bool_false() == 0
            && replay_nvfbc_bool_true() == 1
            && status_values
                .into_iter()
                .enumerate()
                .all(|(value, status)| status == value as u32)
            && replay_nvfbc_capture_to_sys() == 0
            && replay_nvfbc_capture_shared_cuda() as c_int == NVFBC_CAPTURE_SHARED_CUDA
            && replay_nvfbc_capture_to_gl() == 3
            && replay_nvfbc_tracking_default() == 0
            && replay_nvfbc_tracking_output() as c_int == NVFBC_TRACKING_OUTPUT
            && replay_nvfbc_tracking_screen() == 2
            && replay_nvfbc_buffer_format_argb() == 0
            && replay_nvfbc_buffer_format_rgb() == 1
            && replay_nvfbc_buffer_format_nv12() as c_int == NVFBC_BUFFER_FORMAT_NV12
            && replay_nvfbc_buffer_format_yuv444p() == 3
            && replay_nvfbc_buffer_format_rgba() == 4
            && replay_nvfbc_buffer_format_bgra() == 5
            && replay_nvfbc_backend_auto() == 0
            && replay_nvfbc_backend_x11() as c_int == NVFBC_BACKEND_X11
            && replay_nvfbc_backend_pipewire() == 2
            && replay_nvfbc_backend_direct() == 3
            && replay_nvfbc_tocuda_grab_flags_noflags() == 0
            && replay_nvfbc_tocuda_grab_flags_nowait() == 1
            && replay_nvfbc_tocuda_grab_flags_force_refresh() == 2
            && replay_nvfbc_tocuda_grab_flags_nowait_if_new_frame_ready()
                == NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY
            && replay_nvfbc_output_max() == 5
            && replay_nvfbc_output_name_len() == 128
            && replay_nvfbc_portal_restore_token_len() == 64
            && replay_nvfbc_direct_max_capture_targets() == 10
            && replay_nvfbc_contract_mask() == 0x1fff
            && replay_nvfbc_cuda_signature_mask() == 0x7fff
            && replay_cuda_header_version() == 13_030
            && replay_cuda_result_success() as c_int == CUDA_SUCCESS
            && replay_cuda_get_proc_address_legacy_stream()
                == CUDA_GET_PROC_ADDRESS_LEGACY_STREAM as u32
            && replay_cuda_pointer_attribute_context() as c_int == CUDA_POINTER_ATTRIBUTE_CONTEXT
            && replay_cuda_pointer_attribute_memory_type() as c_int
                == CUDA_POINTER_ATTRIBUTE_MEMORY_TYPE
            && replay_cuda_pointer_attribute_device_ordinal() as c_int
                == CUDA_POINTER_ATTRIBUTE_DEVICE_ORDINAL
            && replay_cuda_memory_type_device() == CUDA_MEMORYTYPE_DEVICE
            && replay_cuda_abi_version_get_proc_address() == 12_000
            && replay_cuda_abi_version_init() == 2_000
            && replay_cuda_abi_version_driver_get_version() == 2_020
            && replay_cuda_abi_version_device_get_by_pci_bus_id() == 4_010
            && replay_cuda_abi_version_device_get_pci_bus_id() == 4_010
            && replay_cuda_abi_version_device_get_uuid_v2() == 11_040
            && replay_cuda_abi_version_ctx_create_v2() == 3_020
            && replay_cuda_abi_version_ctx_get_current() == 4_000
            && replay_cuda_abi_version_ctx_get_device() == 2_000
            && replay_cuda_abi_version_pointer_get_attribute() == 4_000
            && replay_cuda_abi_version_ctx_destroy_v2() == 4_000
            && replay_cuda_abi_version_mem_alloc_v2() == 3_020
            && replay_cuda_abi_version_memcpy_dtod_v2() == 3_020
            && replay_cuda_abi_version_ctx_synchronize() == 2_000
            && replay_cuda_abi_version_mem_free_v2() == 3_020
            && replay_cuda_signature_mask() == 0x7fff;

        let native_function_size = std::mem::size_of::<*mut c_void>();
        let function_pointer_representations = [
            std::mem::size_of::<CuGetProcAddressFn>(),
            std::mem::size_of::<CuInitFn>(),
            std::mem::size_of::<CuDriverGetVersionFn>(),
            std::mem::size_of::<CuDeviceGetByPciBusIdFn>(),
            std::mem::size_of::<CuDeviceGetPciBusIdFn>(),
            std::mem::size_of::<CuDeviceGetUuidFn>(),
            std::mem::size_of::<CuCtxCreateFn>(),
            std::mem::size_of::<CuCtxGetCurrentFn>(),
            std::mem::size_of::<CuCtxGetDeviceFn>(),
            std::mem::size_of::<CuPointerGetAttributeFn>(),
            std::mem::size_of::<CuCtxDestroyFn>(),
            std::mem::size_of::<CuMemAllocFn>(),
            std::mem::size_of::<CuMemcpyDtoDFn>(),
            std::mem::size_of::<CuCtxSynchronizeFn>(),
            std::mem::size_of::<CuMemFreeFn>(),
            std::mem::size_of::<NvFbcCreateInstanceFn>(),
            std::mem::size_of::<NvFbcCreateHandleFn>(),
            std::mem::size_of::<NvFbcDestroyHandleFn>(),
            std::mem::size_of::<NvFbcGetStatusFn>(),
            std::mem::size_of::<NvFbcCreateCaptureSessionFn>(),
            std::mem::size_of::<NvFbcDestroyCaptureSessionFn>(),
            std::mem::size_of::<NvFbcToCudaSetupFn>(),
            std::mem::size_of::<NvFbcToCudaGrabFrameFn>(),
        ]
        .into_iter()
        .all(|size| size == native_function_size);

        structure_layouts
            && scalar_layouts
            && function_list_layout
            && values
            && function_pointer_representations
    };

    matches.then_some(()).ok_or(())
}

#[cfg(replay_nvfbc_source)]
struct DynamicCudaApi {
    _library: libloading::os::unix::Library,
    runtime_library: String,
    init: CuInitFn,
    driver_get_version: CuDriverGetVersionFn,
    device_get_by_pci_bus_id: CuDeviceGetByPciBusIdFn,
    device_get_pci_bus_id: CuDeviceGetPciBusIdFn,
    device_get_uuid: CuDeviceGetUuidFn,
    context_create: CuCtxCreateFn,
    context_get_current: CuCtxGetCurrentFn,
    context_get_device: CuCtxGetDeviceFn,
    pointer_get_attribute: CuPointerGetAttributeFn,
    context_destroy: CuCtxDestroyFn,
    memory_allocate: CuMemAllocFn,
    memory_copy_device_to_device: CuMemcpyDtoDFn,
    context_synchronize: CuCtxSynchronizeFn,
    memory_free: CuMemFreeFn,
}

#[cfg(replay_nvfbc_source)]
impl DynamicCudaApi {
    fn load() -> Result<Self, ()> {
        let library = unsafe {
            // SAFETY: The capture worker starts with a cleared loader environment and
            // this is the fixed CUDA driver SONAME, never an operator-provided path.
            libloading::os::unix::Library::open(
                Some(CUDA_LIBRARY_NAME),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .map_err(|_| ())?;
        let get_proc_address: CuGetProcAddressFn = unsafe {
            // SAFETY: The bootstrap symbol is ABI-checked against cudaTypedefs.h.
            *library
                .get::<CuGetProcAddressFn>(b"cuGetProcAddress_v2\0")
                .map_err(|_| ())?
        };
        let runtime_library = loaded_runtime_library_basename("libcuda.so.").ok_or(())?;
        Ok(Self {
            init: resolve_cuda_symbol(get_proc_address, b"cuInit\0", 2_000)?,
            driver_get_version: resolve_cuda_symbol(
                get_proc_address,
                b"cuDriverGetVersion\0",
                2_020,
            )?,
            device_get_by_pci_bus_id: resolve_cuda_symbol(
                get_proc_address,
                b"cuDeviceGetByPCIBusId\0",
                4_010,
            )?,
            device_get_pci_bus_id: resolve_cuda_symbol(
                get_proc_address,
                b"cuDeviceGetPCIBusId\0",
                4_010,
            )?,
            device_get_uuid: resolve_cuda_symbol(get_proc_address, b"cuDeviceGetUuid\0", 11_040)?,
            context_create: resolve_cuda_symbol(get_proc_address, b"cuCtxCreate\0", 3_020)?,
            context_get_current: resolve_cuda_symbol(
                get_proc_address,
                b"cuCtxGetCurrent\0",
                4_000,
            )?,
            context_get_device: resolve_cuda_symbol(get_proc_address, b"cuCtxGetDevice\0", 2_000)?,
            pointer_get_attribute: resolve_cuda_symbol(
                get_proc_address,
                b"cuPointerGetAttribute\0",
                4_000,
            )?,
            context_destroy: resolve_cuda_symbol(get_proc_address, b"cuCtxDestroy\0", 4_000)?,
            memory_allocate: resolve_cuda_symbol(get_proc_address, b"cuMemAlloc\0", 3_020)?,
            memory_copy_device_to_device: resolve_cuda_symbol(
                get_proc_address,
                b"cuMemcpyDtoD\0",
                3_020,
            )?,
            context_synchronize: resolve_cuda_symbol(
                get_proc_address,
                b"cuCtxSynchronize\0",
                2_000,
            )?,
            memory_free: resolve_cuda_symbol(get_proc_address, b"cuMemFree\0", 3_020)?,
            _library: library,
            runtime_library,
        })
    }
}

#[cfg(replay_nvfbc_source)]
fn resolve_cuda_symbol<T: Copy>(
    get_proc_address: CuGetProcAddressFn,
    name: &'static [u8],
    version: c_int,
) -> Result<T, ()> {
    let name = CStr::from_bytes_with_nul(name).map_err(|_| ())?;
    let mut pointer = std::ptr::null_mut();
    let mut query_result = -1;
    let started = Instant::now();
    let status = unsafe {
        // SAFETY: get_proc_address is the fixed bootstrap symbol and all output slots
        // are valid. The exact historical signature is checked by the C oracle.
        get_proc_address(
            name.as_ptr(),
            &mut pointer,
            version,
            CUDA_GET_PROC_ADDRESS_LEGACY_STREAM,
            &mut query_result,
        )
    };
    if started.elapsed().as_nanos() > MAX_NATIVE_CALL_ELAPSED_NS
        || status != CUDA_SUCCESS
        || query_result != 0
        || pointer.is_null()
        || std::mem::size_of::<T>() != std::mem::size_of::<*mut c_void>()
    {
        return Err(());
    }
    Ok(unsafe {
        // SAFETY: The C oracle checked the requested historical function type,
        // and CUDA returned a non-null ABI-compatible function pointer.
        std::mem::transmute_copy::<*mut c_void, T>(&pointer)
    })
}

#[cfg(replay_nvfbc_source)]
struct DynamicNvfbcApi {
    _library: libloading::os::unix::Library,
    runtime_library: String,
    create_handle: NvFbcCreateHandleFn,
    destroy_handle: NvFbcDestroyHandleFn,
    get_status: NvFbcGetStatusFn,
    create_capture_session: NvFbcCreateCaptureSessionFn,
    destroy_capture_session: NvFbcDestroyCaptureSessionFn,
    to_cuda_setup: NvFbcToCudaSetupFn,
    to_cuda_grab_frame: NvFbcToCudaGrabFrameFn,
}

#[cfg(replay_nvfbc_source)]
impl DynamicNvfbcApi {
    fn load() -> Result<Self, NativeCaptureError> {
        let library = unsafe {
            // SAFETY: The worker loader environment is cleared and only the fixed
            // NVIDIA NvFBC SONAME reaches dlopen.
            libloading::os::unix::Library::open(
                Some(NVFBC_LIBRARY_NAME),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .map_err(|_| NativeCaptureError::cuda(CaptureFailureV1::SourceMismatch))?;
        let create_instance = unsafe {
            // SAFETY: This one bootstrap signature is checked by the source oracle.
            *library
                .get::<NvFbcCreateInstanceFn>(b"NvFBCCreateInstance\0")
                .map_err(|_| NativeCaptureError::cuda(CaptureFailureV1::ApiMismatch))?
        };
        let mut functions = NvFbcApiFunctionList::default();
        let status = timed_nvfbc_call(|| unsafe {
            // SAFETY: functions is zero-initialized, carries API 1.9, and passed the
            // exact source-derived layout oracle before this allocation boundary.
            create_instance(&mut functions)
        })?;
        if status != NVFBC_SUCCESS {
            return Err(NativeCaptureError::nvfbc(map_nvfbc_failure(status), status));
        }
        validate_nvfbc_function_list(&functions)?;
        let runtime_library = loaded_runtime_library_basename("libnvidia-fbc.so.")
            .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::SourceMismatch))?;
        Ok(Self {
            create_handle: cast_function_pointer(functions.entries[1])?,
            destroy_handle: cast_function_pointer(functions.entries[2])?,
            get_status: cast_function_pointer(functions.entries[3])?,
            create_capture_session: cast_function_pointer(functions.entries[4])?,
            destroy_capture_session: cast_function_pointer(functions.entries[5])?,
            to_cuda_setup: cast_function_pointer(functions.entries[8])?,
            to_cuda_grab_frame: cast_function_pointer(functions.entries[9])?,
            _library: library,
            runtime_library,
        })
    }
}

#[cfg(replay_nvfbc_source)]
fn cast_function_pointer<T: Copy>(pointer: *mut c_void) -> Result<T, NativeCaptureError> {
    if pointer.is_null() || std::mem::size_of::<T>() != std::mem::size_of::<*mut c_void>() {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::ApiMismatch));
    }
    Ok(unsafe {
        // SAFETY: Function list offsets and signatures are source-oracle checked and
        // the corresponding required entry was verified non-null.
        std::mem::transmute_copy::<*mut c_void, T>(&pointer)
    })
}

#[cfg(replay_nvfbc_source)]
#[derive(Debug, Clone, Copy)]
struct NativeCaptureError {
    failure: CaptureFailureV1,
    nvfbc_status: Option<i32>,
    containment_required: bool,
}

#[cfg(replay_nvfbc_source)]
impl NativeCaptureError {
    const fn cuda(failure: CaptureFailureV1) -> Self {
        Self {
            failure,
            nvfbc_status: None,
            containment_required: false,
        }
    }

    const fn nvfbc(failure: CaptureFailureV1, status: i32) -> Self {
        Self {
            failure,
            nvfbc_status: Some(status),
            containment_required: false,
        }
    }

    const fn cuda_containment(failure: CaptureFailureV1) -> Self {
        Self {
            failure,
            nvfbc_status: None,
            containment_required: true,
        }
    }

    const fn nvfbc_containment(failure: CaptureFailureV1, status: i32) -> Self {
        Self {
            failure,
            nvfbc_status: Some(status),
            containment_required: true,
        }
    }
}

#[cfg(replay_nvfbc_source)]
#[derive(Debug, Clone, Copy)]
struct NativeCallOutcome<T> {
    status: T,
    elapsed_ns: u128,
}

#[cfg(replay_nvfbc_source)]
impl<T> NativeCallOutcome<T> {
    fn exceeded_elapsed_policy(&self) -> bool {
        self.elapsed_ns > MAX_NATIVE_CALL_ELAPSED_NS
    }
}

#[cfg(replay_nvfbc_source)]
fn observe_nvfbc_call(call: impl FnOnce() -> c_int) -> NativeCallOutcome<c_int> {
    let started = Instant::now();
    let status = call();
    NativeCallOutcome {
        status,
        elapsed_ns: started.elapsed().as_nanos(),
    }
}

#[cfg(replay_nvfbc_source)]
fn enforce_nvfbc_elapsed(outcome: NativeCallOutcome<c_int>) -> Result<c_int, NativeCaptureError> {
    if outcome.exceeded_elapsed_policy() {
        Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::WorkerRejected,
            outcome.status,
        ))
    } else {
        Ok(outcome.status)
    }
}

#[cfg(replay_nvfbc_source)]
fn timed_nvfbc_call(call: impl FnOnce() -> c_int) -> Result<c_int, NativeCaptureError> {
    enforce_nvfbc_elapsed(observe_nvfbc_call(call))
}

#[cfg(replay_nvfbc_source)]
fn loaded_runtime_library_basename(prefix: &str) -> Option<String> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    let mut names = HashSet::new();
    for line in maps.lines() {
        let path = line.split_ascii_whitespace().last()?;
        if !path.starts_with('/') || path.ends_with(" (deleted)") {
            continue;
        }
        let name = Path::new(path).file_name()?.to_str()?;
        if name.starts_with(prefix) && valid_runtime_library(name, prefix) {
            names.insert(name.to_owned());
        }
    }
    (names.len() == 1)
        .then(|| names.into_iter().next())
        .flatten()
}

#[cfg(replay_nvfbc_source)]
fn map_nvfbc_failure(status: i32) -> CaptureFailureV1 {
    match status {
        1 => CaptureFailureV1::ApiMismatch,
        6 => CaptureFailureV1::Busy,
        16 => CaptureFailureV1::BindingMismatch,
        _ => CaptureFailureV1::InvalidFrame,
    }
}

#[cfg(replay_nvfbc_source)]
fn observe_cuda_call(call: impl FnOnce() -> CuResult) -> NativeCallOutcome<CuResult> {
    let started = Instant::now();
    let status = call();
    NativeCallOutcome {
        status,
        elapsed_ns: started.elapsed().as_nanos(),
    }
}

#[cfg(replay_nvfbc_source)]
fn enforce_cuda_outcome(outcome: NativeCallOutcome<CuResult>) -> Result<(), NativeCaptureError> {
    if outcome.status != CUDA_SUCCESS {
        Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame))
    } else if outcome.exceeded_elapsed_policy() {
        Err(NativeCaptureError::cuda(CaptureFailureV1::WorkerRejected))
    } else {
        Ok(())
    }
}

#[cfg(replay_nvfbc_source)]
fn timed_cuda_call(call: impl FnOnce() -> CuResult) -> Result<(), NativeCaptureError> {
    enforce_cuda_outcome(observe_cuda_call(call))
}

#[cfg(replay_nvfbc_source)]
fn record_cuda_acquisition(
    outcome: NativeCallOutcome<CuResult>,
    owned: &mut bool,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
    event: CaptureLifecycleEventV1,
) -> Result<(), NativeCaptureError> {
    if outcome.status != CUDA_SUCCESS {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
    }
    *owned = true;
    lifecycle.push(event);
    enforce_cuda_outcome(outcome)
}

#[cfg(replay_nvfbc_source)]
fn record_nvfbc_acquisition(
    outcome: NativeCallOutcome<c_int>,
    owned: &mut bool,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
    event: CaptureLifecycleEventV1,
) -> Result<(), NativeCaptureError> {
    require_nvfbc_success(outcome.status)?;
    *owned = true;
    lifecycle.push(event);
    enforce_nvfbc_elapsed(outcome).map(|_| ())
}

#[cfg(replay_nvfbc_source)]
fn record_cuda_release(
    outcome: NativeCallOutcome<CuResult>,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
    event: CaptureLifecycleEventV1,
) -> Result<(), NativeCaptureError> {
    if outcome.status != CUDA_SUCCESS {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::CleanupUncertain));
    }
    lifecycle.push(event);
    enforce_cuda_outcome(outcome)
}

#[cfg(replay_nvfbc_source)]
fn record_nvfbc_release(
    outcome: NativeCallOutcome<c_int>,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
    event: CaptureLifecycleEventV1,
) -> Result<(), NativeCaptureError> {
    if outcome.status != NVFBC_SUCCESS {
        return Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::CleanupUncertain,
            outcome.status,
        ));
    }
    lifecycle.push(event);
    enforce_nvfbc_elapsed(outcome).map(|_| ())
}

#[cfg(replay_nvfbc_source)]
fn record_frame_sync(
    outcome: NativeCallOutcome<CuResult>,
    frame_borrowed: &mut bool,
    copy_pending: &mut bool,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
) -> Result<(), NativeCaptureError> {
    if outcome.status != CUDA_SUCCESS {
        return Err(NativeCaptureError::cuda_containment(
            CaptureFailureV1::CleanupUncertain,
        ));
    }
    *copy_pending = false;
    *frame_borrowed = false;
    lifecycle.push(CaptureLifecycleEventV1::FrameReleased);
    enforce_cuda_outcome(outcome)
}

#[cfg(replay_nvfbc_source)]
fn validate_created_handle_backend(
    backend: c_int,
    status: c_int,
) -> Result<(), NativeCaptureError> {
    if backend == NVFBC_BACKEND_X11 {
        Ok(())
    } else {
        Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::BindingMismatch,
            status,
        ))
    }
}

#[cfg(replay_nvfbc_source)]
fn capture_native_one_frame(
    selected: &SelectedOutputV1,
    mut source: CaptureSourceEvidenceV1,
) -> CapturePrimitiveObservationV1 {
    let binding = crate::model::CaptureOutputBindingV1 {
        output_name: selected.output_name.clone(),
        randr_output_xid: selected.randr_output_xid,
        topology_token: selected.topology_token.clone(),
        gpu_pci_bdf: selected.nvml_pci_bdf.clone(),
        gpu_uuid: selected.nvml_uuid.clone(),
    };
    let mut lifecycle = Vec::new();
    let cuda = match DynamicCudaApi::load() {
        Ok(cuda) => cuda,
        Err(()) => {
            return rejected_live_observation(
                source,
                CaptureFailureV1::SourceMismatch,
                Some(binding),
                lifecycle,
                None,
            );
        }
    };
    source.cuda_runtime_library = Some(cuda.runtime_library.clone());
    lifecycle.push(CaptureLifecycleEventV1::CudaLibraryLoaded);

    let mut context = std::ptr::null_mut();
    let mut context_created = false;
    let body = (|| {
        timed_cuda_call(|| unsafe {
            // SAFETY: The historical cuInit signature was source-oracle checked.
            (cuda.init)(0)
        })?;
        let mut driver_version = 0;
        timed_cuda_call(|| unsafe {
            // SAFETY: driver_version is a writable c_int output slot.
            (cuda.driver_get_version)(&mut driver_version)
        })?;
        if driver_version < 12_000 {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::ApiMismatch));
        }
        source.cuda_driver_version = u32::try_from(driver_version).ok();

        let selected_bdf = normalize_pci_bdf(&selected.nvml_pci_bdf)
            .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch))?;
        let cuda_bdf = CString::new(selected_bdf.as_str())
            .map_err(|_| NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch))?;
        let mut device = -1;
        timed_cuda_call(|| unsafe {
            // SAFETY: cuda_bdf is NUL-terminated and device is a writable output.
            (cuda.device_get_by_pci_bus_id)(&mut device, cuda_bdf.as_ptr())
        })?;
        if device < 0 {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
        }
        verify_cuda_device_identity(&cuda, device, &selected_bdf, &selected.nvml_uuid)?;

        let mut prior_context = std::ptr::null_mut();
        timed_cuda_call(|| unsafe {
            // SAFETY: prior_context is a writable opaque-context slot.
            (cuda.context_get_current)(&mut prior_context)
        })?;
        if !prior_context.is_null() {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
        }
        let context_outcome = observe_cuda_call(|| unsafe {
            // SAFETY: context is writable and device was selected by exact PCI BDF.
            (cuda.context_create)(&mut context, 0, device)
        });
        if context_outcome.status != CUDA_SUCCESS {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
        }
        if context.is_null() {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
        }
        record_cuda_acquisition(
            context_outcome,
            &mut context_created,
            &mut lifecycle,
            CaptureLifecycleEventV1::CudaContextCreated,
        )?;
        verify_current_cuda_context(&cuda, context, device)?;
        capture_with_nvfbc(
            &cuda,
            context,
            device,
            selected,
            &binding,
            &mut source,
            &mut lifecycle,
        )
    })();

    let containment_required = body
        .as_ref()
        .err()
        .is_some_and(|error| error.containment_required);
    let mut cleanup_error = None;
    if context_created && !containment_required {
        let outcome = observe_cuda_call(|| unsafe {
            // SAFETY: context was created exactly once by this worker and all
            // application allocations were released by capture_with_nvfbc.
            (cuda.context_destroy)(context)
        });
        if outcome.status == CUDA_SUCCESS {
            context_created = false;
        }
        if let Err(error) = record_cuda_release(
            outcome,
            &mut lifecycle,
            CaptureLifecycleEventV1::CudaContextDestroyed,
        ) {
            cleanup_error = Some(error);
        }
    }
    if context_created {
        // A live/uncertain context must remain loaded until the bounded worker
        // process exits; unloading the driver underneath it would invent cleanup.
        std::mem::forget(cuda);
    } else {
        drop(cuda);
        lifecycle.push(CaptureLifecycleEventV1::CudaLibraryUnloaded);
    }

    match (body, cleanup_error) {
        (Ok((frame, nvfbc_status_raw)), None) => CapturePrimitiveObservationV1 {
            schema: NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1.to_owned(),
            provider: CaptureProviderKindV1::SourceAuthenticated,
            source,
            binding: Some(binding),
            frame: Some(frame),
            lifecycle,
            failure: None,
            nvfbc_status_raw: Some(nvfbc_status_raw),
        },
        (Ok((_, nvfbc_status_raw)), Some(error)) => rejected_live_observation(
            source,
            error.failure,
            Some(binding),
            lifecycle,
            Some(nvfbc_status_raw),
        ),
        (Err(error), cleanup_error) => rejected_live_observation(
            source,
            cleanup_error.map_or(error.failure, |cleanup| cleanup.failure),
            Some(binding),
            lifecycle,
            error.nvfbc_status,
        ),
    }
}

#[cfg(replay_nvfbc_source)]
fn capture_with_nvfbc(
    cuda: &DynamicCudaApi,
    context: CuContext,
    device: CuDevice,
    selected: &SelectedOutputV1,
    binding: &crate::model::CaptureOutputBindingV1,
    source: &mut CaptureSourceEvidenceV1,
    lifecycle: &mut Vec<CaptureLifecycleEventV1>,
) -> Result<(CaptureFrameObservationV1, i32), NativeCaptureError> {
    let nvfbc = DynamicNvfbcApi::load()?;
    source.nvfbc_runtime_library = Some(nvfbc.runtime_library.clone());
    lifecycle.push(CaptureLifecycleEventV1::LibraryLoaded);

    let mut handle = 0;
    let mut handle_created = false;
    let mut session_created = false;
    let mut application_buffer = 0;
    let mut buffer_allocated = false;
    let mut frame_borrowed = false;
    let mut copy_pending = false;
    let mut body_status = NVFBC_SUCCESS;
    let body = (|| {
        let mut params = NvFbcCreateHandleParams {
            version: NVFBC_CREATE_HANDLE_PARAMS_VERSION,
            ..NvFbcCreateHandleParams::default()
        };
        let handle_outcome = observe_nvfbc_call(|| unsafe {
            // SAFETY: handle and params use the exact source-oracle checked layout.
            (nvfbc.create_handle)(&mut handle, &mut params)
        });
        body_status = handle_outcome.status;
        record_nvfbc_acquisition(
            handle_outcome,
            &mut handle_created,
            lifecycle,
            CaptureLifecycleEventV1::HandleCreated,
        )?;
        // NvFBC defines no invalid numeric sentinel for its opaque u64 handle;
        // successful creation is authoritative even when the returned bits are zero.
        validate_created_handle_backend(params.backend, body_status)?;

        let mut status = NvFbcGetStatusParams {
            version: NVFBC_GET_STATUS_PARAMS_VERSION,
            ..NvFbcGetStatusParams::default()
        };
        body_status = timed_nvfbc_call(|| unsafe {
            // SAFETY: handle is live and status uses the checked API 1.9 layout.
            (nvfbc.get_status)(handle, &mut status)
        })?;
        require_nvfbc_success(body_status)?;
        validate_nvfbc_status(&status, selected)?;
        lifecycle.push(CaptureLifecycleEventV1::StatusQueried);

        let mut session = NvFbcCreateCaptureSessionParams {
            version: NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VERSION,
            capture_type: NVFBC_CAPTURE_SHARED_CUDA,
            tracking_type: NVFBC_TRACKING_OUTPUT,
            output_id: selected.randr_output_xid.get(),
            with_cursor: 1,
            disable_auto_modeset_recovery: 1,
            round_frame_size: 0,
            sampling_rate_ms: 16,
            push_model: 1,
            allow_direct_capture: 0,
            dbus_timeout_ms: 500,
            ..NvFbcCreateCaptureSessionParams::default()
        };
        let session_outcome = observe_nvfbc_call(|| unsafe {
            // SAFETY: The session is bound to the exact status-proven RandR XID.
            (nvfbc.create_capture_session)(handle, &mut session)
        });
        body_status = session_outcome.status;
        record_nvfbc_acquisition(
            session_outcome,
            &mut session_created,
            lifecycle,
            CaptureLifecycleEventV1::SessionCreated,
        )?;

        let mut setup = NvFbcToCudaSetupParams {
            version: NVFBC_TOCUDA_SETUP_PARAMS_VERSION,
            buffer_format: NVFBC_BUFFER_FORMAT_NV12,
        };
        let setup_outcome = observe_nvfbc_call(|| unsafe {
            // SAFETY: The live session requested SHARED_CUDA and setup is API checked.
            (nvfbc.to_cuda_setup)(handle, &mut setup)
        });
        body_status = setup_outcome.status;
        require_nvfbc_success(body_status)?;
        lifecycle.push(CaptureLifecycleEventV1::ToCudaSetup);
        enforce_nvfbc_elapsed(setup_outcome)?;

        let byte_size = exact_frame_byte_size(
            selected.width_px,
            selected.height_px,
            CapturePixelFormatV1::Nv12,
        )
        .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame))?;
        let allocation_outcome = observe_cuda_call(|| unsafe {
            // SAFETY: application_buffer is a writable CUdeviceptr and byte_size is bounded.
            (cuda.memory_allocate)(
                &mut application_buffer,
                usize::try_from(byte_size).unwrap_or(usize::MAX),
            )
        });
        if allocation_outcome.status != CUDA_SUCCESS {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
        }
        if application_buffer == 0 {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
        }
        record_cuda_acquisition(
            allocation_outcome,
            &mut buffer_allocated,
            lifecycle,
            CaptureLifecycleEventV1::ApplicationBufferAllocated,
        )?;
        verify_cuda_pointer(cuda, application_buffer, context, device)?;

        let mut nvfbc_buffer: CuDevicePtr = 0;
        let mut frame_info = NvFbcFrameGrabInfo::default();
        let mut grab = NvFbcToCudaGrabFrameParams {
            version: NVFBC_TOCUDA_GRAB_FRAME_PARAMS_VERSION,
            flags: NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY,
            cuda_device_buffer: (&mut nvfbc_buffer as *mut CuDevicePtr).cast(),
            frame_info: &mut frame_info,
            timeout_ms: NVFBC_GRAB_TIMEOUT_MS,
        };
        let grab_started = Instant::now();
        body_status = unsafe {
            // SAFETY: The output pointer slots remain live for the full bounded call.
            (nvfbc.to_cuda_grab_frame)(handle, &mut grab)
        };
        let grab_elapsed_ns = u64::try_from(grab_started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        require_nvfbc_success(body_status)?;
        if nvfbc_buffer == 0 {
            return Err(NativeCaptureError::nvfbc(
                CaptureFailureV1::InvalidFrame,
                body_status,
            ));
        }
        frame_borrowed = true;
        lifecycle.push(CaptureLifecycleEventV1::FrameGrabbed);
        if grab_elapsed_ns > 2_000_000_000 {
            return Err(NativeCaptureError::nvfbc(
                CaptureFailureV1::WorkerRejected,
                body_status,
            ));
        }
        validate_frame_info(&frame_info, selected, byte_size)?;
        verify_current_cuda_context(cuda, context, device)?;
        verify_cuda_pointer(cuda, nvfbc_buffer, context, device)?;
        let copy_outcome = observe_cuda_call(|| unsafe {
            // SAFETY: Both pointers belong to this exact context/device and byte_size
            // is the status-validated tightly packed NV12 allocation length.
            (cuda.memory_copy_device_to_device)(
                application_buffer,
                nvfbc_buffer,
                usize::try_from(byte_size).unwrap_or(usize::MAX),
            )
        });
        if copy_outcome.status != CUDA_SUCCESS {
            return Err(NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame));
        }
        copy_pending = true;
        let sync_outcome = observe_cuda_call(|| unsafe {
            // SAFETY: Synchronizes the current isolated context before the borrowed
            // NvFBC buffer is logically released or reused.
            (cuda.context_synchronize)()
        });
        record_frame_sync(
            sync_outcome,
            &mut frame_borrowed,
            &mut copy_pending,
            lifecycle,
        )?;
        enforce_cuda_outcome(copy_outcome)?;

        let gpu = CaptureGpuIdentityV1 {
            pci_bdf: binding.gpu_pci_bdf.clone(),
            gpu_uuid: binding.gpu_uuid.clone(),
        };
        let planes = nv12_planes(selected.width_px, selected.height_px)?;
        Ok(CaptureFrameObservationV1 {
            grab_status: crate::model::CaptureGrabStatusV1::Success,
            frame_sequence: u64::from(frame_info.current_frame),
            timestamp_us: frame_info.timestamp_us,
            is_new_frame: parse_nvfbc_bool(frame_info.is_new_frame)?,
            width_px: selected.width_px,
            height_px: selected.height_px,
            pixel_format: CapturePixelFormatV1::Nv12,
            pitch_bytes: u32::from(selected.width_px),
            planes,
            required_post_processing: parse_nvfbc_bool(frame_info.required_post_processing)?,
            cursor_included: parse_nvfbc_bool(frame_info.cursor_composited)?,
            cursor_mode: if parse_nvfbc_bool(frame_info.cursor_composited)? {
                crate::model::CaptureCursorModeV1::NvfbcComposited
            } else {
                crate::model::CaptureCursorModeV1::Excluded
            },
            source_surface: "selected-scanout-bgra".to_owned(),
            lease_surface: "application-owned-nv12".to_owned(),
            edges: vec![
                CopyLedgerEdgeV1 {
                    sequence: 0,
                    kind: CopyEdgeKindV1::Conversion,
                    from_surface: "selected-scanout-bgra".to_owned(),
                    to_surface: "nvfbc-shared-cuda-nv12".to_owned(),
                    from_gpu: gpu.clone(),
                    to_gpu: gpu.clone(),
                    input_format: CapturePixelFormatV1::Bgra,
                    output_format: CapturePixelFormatV1::Nv12,
                    peer_access: None,
                },
                CopyLedgerEdgeV1 {
                    sequence: 1,
                    kind: CopyEdgeKindV1::SameGpuDeviceCopy,
                    from_surface: "nvfbc-shared-cuda-nv12".to_owned(),
                    to_surface: "application-owned-nv12".to_owned(),
                    from_gpu: gpu.clone(),
                    to_gpu: gpu,
                    input_format: CapturePixelFormatV1::Nv12,
                    output_format: CapturePixelFormatV1::Nv12,
                    peer_access: None,
                },
            ],
            requested_pixel_format: Some(CapturePixelFormatV1::Nv12),
            byte_size: Some(byte_size),
            missed_frames: Some(frame_info.missed_frames),
            direct_capture: Some(parse_nvfbc_bool(frame_info.direct_capture)?),
            cursor_requested: Some(true),
            cursor_visible: Some(parse_nvfbc_bool(frame_info.cursor_visible)?),
            cursor_composited: Some(parse_nvfbc_bool(frame_info.cursor_composited)?),
            grab_flags: Some(NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY),
            grab_timeout_ms: Some(NVFBC_GRAB_TIMEOUT_MS),
            grab_elapsed_ns: Some(grab_elapsed_ns),
        })
    })();

    if frame_borrowed && !copy_pending {
        frame_borrowed = false;
        lifecycle.push(CaptureLifecycleEventV1::FrameReleased);
    }
    if copy_pending {
        // A failed synchronization leaves ownership of both GPU buffers
        // uncertain. Keep the native library loaded and let worker-process
        // termination contain the resources without claiming a release.
        std::mem::forget(nvfbc);
        return Err(NativeCaptureError::cuda_containment(
            CaptureFailureV1::CleanupUncertain,
        ));
    }
    let mut cleanup_error = None;
    if buffer_allocated {
        let outcome = observe_cuda_call(|| unsafe {
            // SAFETY: application_buffer was allocated once by this CUDA context.
            (cuda.memory_free)(application_buffer)
        });
        if outcome.status == CUDA_SUCCESS {
            buffer_allocated = false;
        }
        if let Err(error) = record_cuda_release(
            outcome,
            lifecycle,
            CaptureLifecycleEventV1::ApplicationBufferFreed,
        ) {
            cleanup_error = Some(error);
        }
    }
    if session_created {
        let mut destroy = NvFbcVersionOnlyParams {
            version: NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VERSION,
        };
        let outcome = observe_nvfbc_call(|| unsafe {
            // SAFETY: This destroys the one live capture session exactly once.
            (nvfbc.destroy_capture_session)(handle, &mut destroy)
        });
        if outcome.status == NVFBC_SUCCESS {
            session_created = false;
        }
        if let Err(error) = record_nvfbc_release(
            outcome,
            lifecycle,
            CaptureLifecycleEventV1::SessionDestroyed,
        ) {
            cleanup_error = Some(error);
        }
    }
    if handle_created {
        let mut destroy = NvFbcVersionOnlyParams {
            version: NVFBC_DESTROY_HANDLE_PARAMS_VERSION,
        };
        let outcome = observe_nvfbc_call(|| unsafe {
            // SAFETY: DestroyHandle implicitly releases NvFBC's same-thread context.
            (nvfbc.destroy_handle)(handle, &mut destroy)
        });
        if outcome.status == NVFBC_SUCCESS {
            handle_created = false;
        }
        if let Err(error) =
            record_nvfbc_release(outcome, lifecycle, CaptureLifecycleEventV1::HandleDestroyed)
        {
            cleanup_error = Some(error);
        }
    }
    if handle_created {
        std::mem::forget(nvfbc);
        return Err(NativeCaptureError::nvfbc_containment(
            CaptureFailureV1::CleanupUncertain,
            body_status,
        ));
    }
    drop(nvfbc);
    lifecycle.push(CaptureLifecycleEventV1::LibraryUnloaded);
    let _ = (frame_borrowed, buffer_allocated, session_created);

    if let Some(error) = cleanup_error {
        return Err(error);
    }
    body.map(|frame| (frame, body_status))
}

#[cfg(replay_nvfbc_source)]
fn require_nvfbc_success(status: i32) -> Result<(), NativeCaptureError> {
    if status == NVFBC_SUCCESS {
        Ok(())
    } else {
        Err(NativeCaptureError::nvfbc(map_nvfbc_failure(status), status))
    }
}

#[cfg(replay_nvfbc_source)]
fn parse_nvfbc_bool(value: c_int) -> Result<bool, NativeCaptureError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::InvalidFrame,
            NVFBC_SUCCESS,
        )),
    }
}

#[cfg(replay_nvfbc_source)]
fn validate_nvfbc_status(
    status: &NvFbcGetStatusParams,
    selected: &SelectedOutputV1,
) -> Result<(), NativeCaptureError> {
    validate_nvfbc_status_readiness(status)?;
    let expected_name = selected
        .output_name
        .display
        .as_deref()
        .ok_or_else(|| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    let expected_x = u32::try_from(selected.origin_x)
        .map_err(|_| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    let expected_y = u32::try_from(selected.origin_y)
        .map_err(|_| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    expected_x
        .checked_add(u32::from(selected.width_px))
        .filter(|right| *right <= status.screen_size.w)
        .ok_or_else(|| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    expected_y
        .checked_add(u32::from(selected.height_px))
        .filter(|bottom| *bottom <= status.screen_size.h)
        .ok_or_else(|| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    validate_nvfbc_output_binding(
        &status.outputs[..status.output_count as usize],
        selected.randr_output_xid.get(),
        expected_name,
        expected_x,
        expected_y,
        u32::from(selected.width_px),
        u32::from(selected.height_px),
    )
}

#[cfg(replay_nvfbc_source)]
fn validate_nvfbc_status_readiness(
    status: &NvFbcGetStatusParams,
) -> Result<(), NativeCaptureError> {
    if status.nvfbc_version != LIVE_NVFBC_API_VERSION
        || !parse_nvfbc_bool(status.is_capture_possible)?
        || parse_nvfbc_bool(status.currently_capturing)?
        || !parse_nvfbc_bool(status.can_create_now)?
        || !parse_nvfbc_bool(status.xrandr_available)?
        || parse_nvfbc_bool(status.in_modeset)?
        || status.screen_size.w == 0
        || status.screen_size.h == 0
        || status.output_count == 0
        || status.output_count > status.outputs.len() as u32
    {
        return Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::BindingMismatch,
            NVFBC_SUCCESS,
        ));
    }
    Ok(())
}

#[cfg(replay_nvfbc_source)]
fn validate_nvfbc_output_binding(
    outputs: &[NvFbcRandrOutputInfo],
    expected_id: u32,
    expected_name: &str,
    expected_x: u32,
    expected_y: u32,
    expected_width: u32,
    expected_height: u32,
) -> Result<(), NativeCaptureError> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut exact_matches = 0;
    for output in outputs {
        let name = bounded_c_name(&output.name)?;
        if output.id == 0 || !ids.insert(output.id) || !names.insert(name.clone()) {
            return Err(NativeCaptureError::nvfbc(
                CaptureFailureV1::BindingMismatch,
                NVFBC_SUCCESS,
            ));
        }
        if output.id == expected_id
            && name == expected_name
            && output.tracked_box.x == expected_x
            && output.tracked_box.y == expected_y
            && output.tracked_box.w == expected_width
            && output.tracked_box.h == expected_height
        {
            exact_matches += 1;
        }
        if (output.id == expected_id) != (name == expected_name) {
            return Err(NativeCaptureError::nvfbc(
                CaptureFailureV1::BindingMismatch,
                NVFBC_SUCCESS,
            ));
        }
    }
    if exact_matches != 1 {
        return Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::BindingMismatch,
            NVFBC_SUCCESS,
        ));
    }
    Ok(())
}

#[cfg(replay_nvfbc_source)]
fn bounded_c_name(value: &[c_char; 128]) -> Result<String, NativeCaptureError> {
    let bytes = unsafe {
        // SAFETY: value is a live fixed-size C array; reinterpretation preserves
        // the exact 128-byte bounds and does not read beyond it.
        std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len())
    };
    let nul = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))?;
    if nul == 0 || bytes[nul + 1..].iter().any(|byte| *byte != 0) {
        return Err(NativeCaptureError::nvfbc(
            CaptureFailureV1::BindingMismatch,
            0,
        ));
    }
    std::str::from_utf8(&bytes[..nul])
        .ok()
        .filter(|name| {
            !name.contains('/')
                && !name.contains('\\')
                && name.bytes().all(|byte| byte.is_ascii_graphic())
        })
        .map(str::to_owned)
        .ok_or_else(|| NativeCaptureError::nvfbc(CaptureFailureV1::BindingMismatch, 0))
}

#[cfg(replay_nvfbc_source)]
fn validate_frame_info(
    frame: &NvFbcFrameGrabInfo,
    selected: &SelectedOutputV1,
    expected_byte_size: u64,
) -> Result<(), NativeCaptureError> {
    validate_frame_contract(
        frame,
        u32::from(selected.width_px),
        u32::from(selected.height_px),
        expected_byte_size,
    )
}

#[cfg(replay_nvfbc_source)]
fn validate_frame_contract(
    frame: &NvFbcFrameGrabInfo,
    expected_width: u32,
    expected_height: u32,
    expected_byte_size: u64,
) -> Result<(), NativeCaptureError> {
    if !parse_nvfbc_bool(frame.is_new_frame)?
        || frame.width != expected_width
        || frame.height != expected_height
        || u64::from(frame.byte_size) != expected_byte_size
        || !parse_nvfbc_bool(frame.required_post_processing)?
        || parse_nvfbc_bool(frame.direct_capture)?
    {
        return Err(NativeCaptureError::nvfbc(
            if parse_nvfbc_bool(frame.is_new_frame).unwrap_or(false) {
                CaptureFailureV1::InvalidFrame
            } else {
                CaptureFailureV1::NoNewFrame
            },
            NVFBC_SUCCESS,
        ));
    }
    // API 1.9 reports visibility and composition as independent output facts.
    // A hidden hardware cursor can still traverse the configured composition
    // path, so validate both booleans without inventing an implication.
    parse_nvfbc_bool(frame.cursor_visible)?;
    parse_nvfbc_bool(frame.cursor_composited)?;
    Ok(())
}

#[cfg(replay_nvfbc_source)]
fn exact_frame_byte_size(
    width: u16,
    height: u16,
    pixel_format: CapturePixelFormatV1,
) -> Option<u64> {
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    match pixel_format {
        CapturePixelFormatV1::Bgra => pixels.checked_mul(4),
        CapturePixelFormatV1::Nv12 if width % 4 == 0 && height % 2 == 0 => {
            pixels.checked_mul(3)?.checked_div(2)
        }
        CapturePixelFormatV1::Yuv444p if width % 4 == 0 && height % 2 == 0 => pixels.checked_mul(3),
        CapturePixelFormatV1::Nv12 | CapturePixelFormatV1::Yuv444p => None,
    }
}

#[cfg(replay_nvfbc_source)]
fn nv12_planes(
    width: u16,
    height: u16,
) -> Result<Vec<crate::model::CapturePlaneV1>, NativeCaptureError> {
    let y_size = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::InvalidFrame))?;
    let uv_size = y_size / 2;
    Ok(vec![
        crate::model::CapturePlaneV1 {
            index: 0,
            offset_bytes: 0,
            stride_bytes: u32::from(width),
            size_bytes: y_size,
        },
        crate::model::CapturePlaneV1 {
            index: 1,
            offset_bytes: y_size,
            stride_bytes: u32::from(width),
            size_bytes: uv_size,
        },
    ])
}

#[cfg(replay_nvfbc_source)]
fn normalize_pci_bdf(value: &str) -> Option<String> {
    let (domain, rest) = value.split_once(':')?;
    let (bus, rest) = rest.split_once(':')?;
    let (device, function) = rest.split_once('.')?;
    if !(domain.len() == 4 || domain.len() == 8)
        || bus.len() != 2
        || device.len() != 2
        || function.len() != 1
        || [domain, bus, device, function]
            .iter()
            .any(|part| !part.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return None;
    }
    let domain = u32::from_str_radix(domain, 16).ok()?;
    let bus = u8::from_str_radix(bus, 16).ok()?;
    let device = u8::from_str_radix(device, 16).ok()?;
    let function = u8::from_str_radix(function, 16).ok()?;
    (device <= 0x1f && function <= 7)
        .then(|| format!("{domain:08x}:{bus:02x}:{device:02x}.{function:x}"))
}

#[cfg(replay_nvfbc_source)]
fn verify_cuda_device_identity(
    cuda: &DynamicCudaApi,
    device: CuDevice,
    expected_bdf: &str,
    expected_uuid: &str,
) -> Result<(), NativeCaptureError> {
    let mut bdf = [0_i8; PCI_BUS_ID_BUFFER_LEN];
    timed_cuda_call(|| unsafe {
        // SAFETY: bdf is writable for the supplied exact length.
        (cuda.device_get_pci_bus_id)(bdf.as_mut_ptr(), PCI_BUS_ID_BUFFER_LEN as c_int, device)
    })?;
    let reported_bdf = unsafe {
        // SAFETY: CUDA promises NUL-termination for a successful bounded query.
        CStr::from_ptr(bdf.as_ptr())
    }
    .to_str()
    .ok()
    .and_then(normalize_pci_bdf)
    .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch))?;
    if reported_bdf != expected_bdf {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
    }
    let mut uuid = CuUuid::default();
    timed_cuda_call(|| unsafe {
        // SAFETY: uuid is an exact writable CUuuid and device was BDF-selected.
        (cuda.device_get_uuid)(&mut uuid, device)
    })?;
    let expected_uuid = parse_gpu_uuid(expected_uuid)
        .ok_or_else(|| NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch))?;
    let observed_uuid = uuid.bytes.map(|byte| byte as u8);
    if observed_uuid != expected_uuid {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
    }
    Ok(())
}

#[cfg(replay_nvfbc_source)]
fn parse_gpu_uuid(value: &str) -> Option<[u8; 16]> {
    let compact = value.strip_prefix("GPU-")?.replace('-', "");
    if compact.len() != 32 || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

#[cfg(replay_nvfbc_source)]
fn verify_current_cuda_context(
    cuda: &DynamicCudaApi,
    expected_context: CuContext,
    expected_device: CuDevice,
) -> Result<(), NativeCaptureError> {
    let mut context = std::ptr::null_mut();
    timed_cuda_call(|| unsafe {
        // SAFETY: context is a writable opaque-context slot.
        (cuda.context_get_current)(&mut context)
    })?;
    let mut device = -1;
    timed_cuda_call(|| unsafe {
        // SAFETY: device is writable and an isolated current context is required.
        (cuda.context_get_device)(&mut device)
    })?;
    if context != expected_context || device != expected_device {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
    }
    Ok(())
}

#[cfg(replay_nvfbc_source)]
fn verify_cuda_pointer(
    cuda: &DynamicCudaApi,
    pointer: CuDevicePtr,
    expected_context: CuContext,
    expected_device: CuDevice,
) -> Result<(), NativeCaptureError> {
    let mut context = std::ptr::null_mut();
    timed_cuda_call(|| unsafe {
        // SAFETY: context is a writable attribute output and pointer is nonzero.
        (cuda.pointer_get_attribute)(
            (&mut context as *mut CuContext).cast(),
            CUDA_POINTER_ATTRIBUTE_CONTEXT,
            pointer,
        )
    })?;
    let mut memory_type = 0_u32;
    timed_cuda_call(|| unsafe {
        // SAFETY: memory_type is a writable CUmemorytype output.
        (cuda.pointer_get_attribute)(
            (&mut memory_type as *mut c_uint).cast(),
            CUDA_POINTER_ATTRIBUTE_MEMORY_TYPE,
            pointer,
        )
    })?;
    let mut device = -1;
    timed_cuda_call(|| unsafe {
        // SAFETY: device is a writable ordinal output.
        (cuda.pointer_get_attribute)(
            (&mut device as *mut CuDevice).cast(),
            CUDA_POINTER_ATTRIBUTE_DEVICE_ORDINAL,
            pointer,
        )
    })?;
    if context != expected_context
        || memory_type != CUDA_MEMORYTYPE_DEVICE
        || device != expected_device
    {
        return Err(NativeCaptureError::cuda(CaptureFailureV1::BindingMismatch));
    }
    Ok(())
}

pub fn capture_one_frame(
    provider: &dyn CaptureProvider,
    selected: Option<&SelectedOutputV1>,
) -> CapturePathEvidenceV1 {
    evaluate_capture_observation(provider.observe(), selected)
}

pub fn evaluate_capture_observation(
    observation: CapturePrimitiveObservationV1,
    selected: Option<&SelectedOutputV1>,
) -> CapturePathEvidenceV1 {
    let cleanup = derive_cleanup(&observation.lifecycle);
    let base = |failure| CapturePathEvidenceV1 {
        schema: NVFBC_CAPTURE_SCHEMA_V1.to_owned(),
        provider: observation.provider,
        source: observation.source.clone(),
        admission: CaptureAdmissionV1::Rejected,
        binding: observation.binding.clone(),
        lease: None,
        copy_ledger: None,
        cleanup: cleanup.clone(),
        failure: Some(failure),
        nvenc_boundary: CaptureBoundaryStatusV1::Unproven,
        nvfbc_status_raw: observation.nvfbc_status_raw,
    };

    if observation.schema != NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1
        || observation.lifecycle.len() > MAX_CAPTURE_LIFECYCLE_EVENTS_V1
        || !valid_provider_source(&observation)
    {
        let mut rejected = base(CaptureFailureV1::SourceMismatch);
        rejected.provider = CaptureProviderKindV1::Fixture;
        rejected.source = CaptureSourceEvidenceV1 {
            status: CaptureSourceStatusV1::Fixture,
            identity: Some(FIXTURE_SOURCE_IDENTITY.to_owned()),
            api_version: Some(FIXTURE_API_VERSION),
            nvfbc_header_sha256: None,
            cuda_header_sha256: None,
            cuda_typedefs_header_sha256: None,
            nvfbc_runtime_library: None,
            cuda_runtime_library: None,
            cuda_driver_version: None,
        };
        return rejected;
    }
    if observation.failure.is_some() {
        return base(
            observation
                .failure
                .expect("checked capture failure must remain present"),
        );
    }
    let Some(selected) = selected else {
        return base(CaptureFailureV1::BindingMismatch);
    };
    let Some(binding) = observation.binding.as_ref() else {
        return base(CaptureFailureV1::BindingMismatch);
    };
    if !binding_matches_selected(binding, selected) {
        return base(CaptureFailureV1::BindingMismatch);
    }
    if !cleanup.complete {
        return base(CaptureFailureV1::CleanupUncertain);
    }
    let Some(frame) = observation.frame.as_ref() else {
        return base(CaptureFailureV1::InvalidFrame);
    };
    if frame.grab_status != crate::model::CaptureGrabStatusV1::Success {
        return base(grab_failure(frame.grab_status));
    }
    if !frame.is_new_frame {
        return base(CaptureFailureV1::StaleFrame);
    }
    if frame.width_px != selected.width_px
        || frame.height_px != selected.height_px
        || !valid_frame_geometry(frame)
    {
        return base(CaptureFailureV1::InvalidFrame);
    }
    let Some(copy_ledger) = derive_copy_ledger(frame) else {
        return base(CaptureFailureV1::InvalidCopyLedger);
    };
    let Some(first_edge) = copy_ledger.edges.first() else {
        return base(CaptureFailureV1::InvalidCopyLedger);
    };
    if first_edge.from_gpu.pci_bdf != binding.gpu_pci_bdf
        || first_edge.from_gpu.gpu_uuid != binding.gpu_uuid
    {
        return base(CaptureFailureV1::BindingMismatch);
    }

    let admission = if observation.provider == CaptureProviderKindV1::SourceAuthenticated {
        CaptureAdmissionV1::Pass
    } else {
        CaptureAdmissionV1::Unproven
    };
    CapturePathEvidenceV1 {
        schema: NVFBC_CAPTURE_SCHEMA_V1.to_owned(),
        provider: observation.provider,
        source: observation.source,
        admission,
        binding: observation.binding,
        lease: Some(CaptureFrameLeaseV1 {
            frame_sequence: frame.frame_sequence,
            timestamp_us: frame.timestamp_us,
            new_frame: frame.is_new_frame,
            width_px: frame.width_px,
            height_px: frame.height_px,
            pixel_format: frame.pixel_format,
            pitch_bytes: frame.pitch_bytes,
            planes: frame.planes.clone(),
            required_post_processing: frame.required_post_processing,
            cursor_included: frame.cursor_included,
            cursor_mode: frame.cursor_mode,
            source_surface: frame.source_surface.clone(),
            lease_surface: frame.lease_surface.clone(),
            requested_pixel_format: frame.requested_pixel_format,
            byte_size: frame.byte_size,
            missed_frames: frame.missed_frames,
            direct_capture: frame.direct_capture,
            cursor_requested: frame.cursor_requested,
            cursor_visible: frame.cursor_visible,
            cursor_composited: frame.cursor_composited,
            grab_flags: frame.grab_flags,
            grab_timeout_ms: frame.grab_timeout_ms,
            grab_elapsed_ns: frame.grab_elapsed_ns,
        }),
        copy_ledger: Some(copy_ledger),
        cleanup,
        failure: None,
        nvenc_boundary: CaptureBoundaryStatusV1::Unproven,
        nvfbc_status_raw: observation.nvfbc_status_raw,
    }
}

pub fn validate_capture_path_evidence(evidence: &CapturePathEvidenceV1) -> bool {
    if evidence.schema != NVFBC_CAPTURE_SCHEMA_V1
        || !valid_source(&evidence.source)
        || !provider_matches_source(evidence.provider, evidence.source.status)
        || !valid_cleanup_ledger(&evidence.cleanup)
    {
        return false;
    }
    match evidence.admission {
        CaptureAdmissionV1::Pass | CaptureAdmissionV1::Unproven => {
            let (Some(binding), Some(lease), Some(ledger)) = (
                evidence.binding.as_ref(),
                evidence.lease.as_ref(),
                evidence.copy_ledger.as_ref(),
            ) else {
                return false;
            };
            let common = evidence.failure.is_none()
                && evidence.cleanup.complete
                && valid_gpu_identity(&CaptureGpuIdentityV1 {
                    pci_bdf: binding.gpu_pci_bdf.clone(),
                    gpu_uuid: binding.gpu_uuid.clone(),
                })
                && valid_lease(lease)
                && validate_persisted_ledger(lease, ledger);
            match evidence.admission {
                CaptureAdmissionV1::Pass => {
                    common
                        && evidence.provider == CaptureProviderKindV1::SourceAuthenticated
                        && evidence.source.status == CaptureSourceStatusV1::Authenticated
                        && evidence.nvfbc_status_raw == Some(0)
                        && valid_live_source(&evidence.source)
                        && valid_live_lease(lease)
                        && valid_live_lifecycle(&evidence.cleanup)
                }
                CaptureAdmissionV1::Unproven => {
                    common && evidence.provider == CaptureProviderKindV1::Fixture
                }
                CaptureAdmissionV1::Rejected => false,
            }
        }
        CaptureAdmissionV1::Rejected => {
            evidence.failure.is_some() && evidence.lease.is_none() && evidence.copy_ledger.is_none()
        }
    }
}

fn valid_provider_source(observation: &CapturePrimitiveObservationV1) -> bool {
    match (observation.provider, observation.source.status) {
        (CaptureProviderKindV1::Fixture, CaptureSourceStatusV1::Fixture) => {
            observation.source.identity.as_deref() == Some(FIXTURE_SOURCE_IDENTITY)
                && observation.source.api_version == Some(FIXTURE_API_VERSION)
                && observation.source.nvfbc_header_sha256.is_none()
                && observation.source.cuda_header_sha256.is_none()
                && observation.source.cuda_typedefs_header_sha256.is_none()
                && observation.source.nvfbc_runtime_library.is_none()
                && observation.source.cuda_runtime_library.is_none()
                && observation.source.cuda_driver_version.is_none()
                && observation.nvfbc_status_raw.is_none()
        }
        (CaptureProviderKindV1::LiveUnavailable, CaptureSourceStatusV1::Unavailable) => {
            observation.source.identity.is_none()
                && observation.source.api_version.is_none()
                && observation.source.nvfbc_header_sha256.is_none()
                && observation.source.cuda_header_sha256.is_none()
                && observation.source.cuda_typedefs_header_sha256.is_none()
                && observation.source.nvfbc_runtime_library.is_none()
                && observation.source.cuda_runtime_library.is_none()
                && observation.source.cuda_driver_version.is_none()
                && observation.binding.is_none()
                && observation.frame.is_none()
                && observation.lifecycle.is_empty()
                && observation.failure == Some(CaptureFailureV1::SourceUnavailable)
                && observation.nvfbc_status_raw.is_none()
        }
        (CaptureProviderKindV1::SourceAuthenticated, CaptureSourceStatusV1::Authenticated) => {
            source_matches_compiled_headers(&observation.source)
        }
        _ => false,
    }
}

fn source_matches_compiled_headers(source: &CaptureSourceEvidenceV1) -> bool {
    let compiled = compiled_capture_source();
    compiled.status == CaptureSourceStatusV1::Authenticated
        && source.status == compiled.status
        && source.identity.as_deref() == Some(LIVE_SOURCE_IDENTITY)
        && source.identity.as_deref() == compiled.identity.as_deref()
        && source.api_version == compiled.api_version
        && source.nvfbc_header_sha256 == compiled.nvfbc_header_sha256
        && source.cuda_header_sha256 == compiled.cuda_header_sha256
        && source.cuda_typedefs_header_sha256 == compiled.cuda_typedefs_header_sha256
}

fn valid_source(source: &CaptureSourceEvidenceV1) -> bool {
    match source.status {
        CaptureSourceStatusV1::Fixture => {
            source.identity.as_deref() == Some(FIXTURE_SOURCE_IDENTITY)
                && source.api_version == Some(FIXTURE_API_VERSION)
                && source.nvfbc_header_sha256.is_none()
                && source.cuda_header_sha256.is_none()
                && source.cuda_typedefs_header_sha256.is_none()
                && source.nvfbc_runtime_library.is_none()
                && source.cuda_runtime_library.is_none()
                && source.cuda_driver_version.is_none()
        }
        CaptureSourceStatusV1::Unavailable => {
            source.identity.is_none()
                && source.api_version.is_none()
                && source.nvfbc_header_sha256.is_none()
                && source.cuda_header_sha256.is_none()
                && source.cuda_typedefs_header_sha256.is_none()
                && source.nvfbc_runtime_library.is_none()
                && source.cuda_runtime_library.is_none()
                && source.cuda_driver_version.is_none()
        }
        CaptureSourceStatusV1::Authenticated => source_matches_compiled_headers(source),
    }
}

fn provider_matches_source(provider: CaptureProviderKindV1, source: CaptureSourceStatusV1) -> bool {
    matches!(
        (provider, source),
        (
            CaptureProviderKindV1::Fixture,
            CaptureSourceStatusV1::Fixture
        ) | (
            CaptureProviderKindV1::LiveUnavailable,
            CaptureSourceStatusV1::Unavailable
        ) | (
            CaptureProviderKindV1::SourceAuthenticated,
            CaptureSourceStatusV1::Authenticated
        )
    )
}

fn binding_matches_selected(
    binding: &crate::model::CaptureOutputBindingV1,
    selected: &SelectedOutputV1,
) -> bool {
    binding.output_name == selected.output_name
        && binding.randr_output_xid == selected.randr_output_xid
        && binding.topology_token == selected.topology_token
        && binding.gpu_pci_bdf == selected.nvml_pci_bdf
        && binding.gpu_uuid == selected.nvml_uuid
        && selected.nvcontrol_gpu_pci_bdf == selected.nvml_pci_bdf
        && selected.nvcontrol_gpu_uuid == selected.nvml_uuid
}

fn valid_frame_geometry(frame: &CaptureFrameObservationV1) -> bool {
    if frame.width_px == 0
        || frame.height_px == 0
        || frame.pitch_bytes < u32::from(frame.width_px)
        || frame.planes.is_empty()
        || frame.planes.len() > MAX_CAPTURE_PLANES_V1
        || !valid_surface_id(&frame.source_surface)
        || !valid_surface_id(&frame.lease_surface)
        || (frame.cursor_included
            != matches!(
                frame.cursor_mode,
                crate::model::CaptureCursorModeV1::NvfbcComposited
            ))
        || frame
            .cursor_composited
            .is_some_and(|composited| composited != frame.cursor_included)
    {
        return false;
    }
    let width = u64::from(frame.width_px);
    let height = u64::from(frame.height_px);
    let stride = u64::from(frame.pitch_bytes);
    let expected_sizes: &[u64] = match frame.pixel_format {
        CapturePixelFormatV1::Bgra => &[stride * height * 4],
        CapturePixelFormatV1::Nv12 => &[stride * height, stride * height / 2],
        CapturePixelFormatV1::Yuv444p => &[stride * height, stride * height, stride * height],
    };
    if frame.planes.len() != expected_sizes.len() {
        return false;
    }
    let mut offset = 0_u64;
    frame
        .planes
        .iter()
        .zip(expected_sizes)
        .enumerate()
        .all(|(index, (plane, expected_size))| {
            let valid = plane.index == index as u8
                && plane.offset_bytes == offset
                && plane.stride_bytes == frame.pitch_bytes
                && plane.size_bytes == *expected_size
                && width <= u64::from(plane.stride_bytes);
            offset = offset.saturating_add(plane.size_bytes);
            valid
        })
        && frame.byte_size.is_none_or(|byte_size| byte_size == offset)
}

fn valid_lease(lease: &CaptureFrameLeaseV1) -> bool {
    valid_frame_geometry(&CaptureFrameObservationV1 {
        grab_status: crate::model::CaptureGrabStatusV1::Success,
        frame_sequence: lease.frame_sequence,
        timestamp_us: lease.timestamp_us,
        is_new_frame: lease.new_frame,
        width_px: lease.width_px,
        height_px: lease.height_px,
        pixel_format: lease.pixel_format,
        pitch_bytes: lease.pitch_bytes,
        planes: lease.planes.clone(),
        required_post_processing: lease.required_post_processing,
        cursor_included: lease.cursor_included,
        cursor_mode: lease.cursor_mode,
        source_surface: lease.source_surface.clone(),
        lease_surface: lease.lease_surface.clone(),
        edges: Vec::new(),
        requested_pixel_format: lease.requested_pixel_format,
        byte_size: lease.byte_size,
        missed_frames: lease.missed_frames,
        direct_capture: lease.direct_capture,
        cursor_requested: lease.cursor_requested,
        cursor_visible: lease.cursor_visible,
        cursor_composited: lease.cursor_composited,
        grab_flags: lease.grab_flags,
        grab_timeout_ms: lease.grab_timeout_ms,
        grab_elapsed_ns: lease.grab_elapsed_ns,
    }) && lease.new_frame
}

fn valid_live_source(source: &CaptureSourceEvidenceV1) -> bool {
    source_matches_compiled_headers(source)
        && source
            .nvfbc_runtime_library
            .as_deref()
            .is_some_and(|name| valid_runtime_library(name, "libnvidia-fbc.so."))
        && source
            .cuda_runtime_library
            .as_deref()
            .is_some_and(|name| valid_runtime_library(name, "libcuda.so."))
        && source
            .cuda_driver_version
            .is_some_and(|version| version >= 12_000)
}

fn valid_runtime_library(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|version| {
        !version.is_empty()
            && version
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
            && version.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 6
                    && part.bytes().all(|byte| byte.is_ascii_digit())
            })
    })
}

fn valid_live_lease(lease: &CaptureFrameLeaseV1) -> bool {
    let exact_byte_size = match lease.pixel_format {
        CapturePixelFormatV1::Bgra => return false,
        CapturePixelFormatV1::Nv12 => {
            u64::from(lease.width_px) * u64::from(lease.height_px) * 3 / 2
        }
        CapturePixelFormatV1::Yuv444p => u64::from(lease.width_px) * u64::from(lease.height_px) * 3,
    };
    lease.requested_pixel_format == Some(lease.pixel_format)
        && lease.byte_size == Some(exact_byte_size)
        && lease.pitch_bytes == u32::from(lease.width_px)
        && lease.required_post_processing
        && lease.direct_capture == Some(false)
        && lease.cursor_requested == Some(true)
        && lease.cursor_visible.is_some()
        && lease.cursor_composited.is_some()
        && lease.missed_frames.is_some()
        && lease.grab_flags == Some(4)
        && lease
            .grab_timeout_ms
            .is_some_and(|timeout_ms| timeout_ms > 0 && timeout_ms <= 1_000)
        && lease
            .grab_elapsed_ns
            .is_some_and(|elapsed_ns| elapsed_ns <= 2_000_000_000)
}

fn valid_surface_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with("0x")
        && !value.contains("pointer")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_gpu_identity(identity: &CaptureGpuIdentityV1) -> bool {
    valid_pci_bdf(&identity.pci_bdf) && valid_gpu_uuid(&identity.gpu_uuid)
}

fn valid_pci_bdf(value: &str) -> bool {
    let Some((domain, remainder)) = value.split_once(':') else {
        return false;
    };
    let Some((bus, remainder)) = remainder.split_once(':') else {
        return false;
    };
    let Some((device, function)) = remainder.split_once('.') else {
        return false;
    };
    domain.len() == 8
        && bus.len() == 2
        && device.len() == 2
        && function.len() == 1
        && [domain, bus, device, function]
            .into_iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_gpu_uuid(value: &str) -> bool {
    value.strip_prefix("GPU-").is_some_and(|uuid| {
        let groups = uuid.split('-').collect::<Vec<_>>();
        groups.len() == 5
            && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
                group.len() == length && group.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    })
}

fn derive_copy_ledger(frame: &CaptureFrameObservationV1) -> Option<CopyLedgerV1> {
    if frame.edges.is_empty() || frame.edges.len() > MAX_CAPTURE_COPY_EDGES_V1 {
        return None;
    }
    let mut seen = HashSet::with_capacity(frame.edges.len());
    let mut visited_surfaces = HashSet::with_capacity(frame.edges.len() + 1);
    visited_surfaces.insert(frame.source_surface.as_str());
    let mut previous_surface = frame.source_surface.as_str();
    let mut previous_format = frame
        .edges
        .first()
        .map(|edge| edge.input_format)
        .expect("non-empty copy ledger has a first edge");
    let mut zero_copy_edges = 0_u16;
    let mut device_copy_edges = 0_u16;
    let mut peer_copy_edges = 0_u16;
    let mut conversion_edges = 0_u16;
    for (index, edge) in frame.edges.iter().enumerate() {
        if edge.sequence != index as u16
            || edge.from_surface != previous_surface
            || edge.input_format != previous_format
            || !valid_edge(edge)
            || !seen.insert((
                edge.sequence,
                edge.from_surface.as_str(),
                edge.to_surface.as_str(),
            ))
        {
            return None;
        }
        if edge.kind != CopyEdgeKindV1::ZeroCopy
            && !visited_surfaces.insert(edge.to_surface.as_str())
        {
            return None;
        }
        match edge.kind {
            CopyEdgeKindV1::ZeroCopy => zero_copy_edges += 1,
            CopyEdgeKindV1::SameGpuDeviceCopy => device_copy_edges += 1,
            CopyEdgeKindV1::PeerCopy => peer_copy_edges += 1,
            CopyEdgeKindV1::Conversion => conversion_edges += 1,
            CopyEdgeKindV1::HostStaged | CopyEdgeKindV1::Unknown => return None,
        }
        previous_surface = &edge.to_surface;
        previous_format = edge.output_format;
    }
    if previous_surface != frame.lease_surface || previous_format != frame.pixel_format {
        return None;
    }
    Some(CopyLedgerV1 {
        edges: frame.edges.clone(),
        zero_copy_edges,
        device_copy_edges,
        peer_copy_edges,
        conversion_edges,
        host_staged_edges: 0,
    })
}

fn valid_edge(edge: &CopyLedgerEdgeV1) -> bool {
    if !valid_surface_id(&edge.from_surface)
        || !valid_surface_id(&edge.to_surface)
        || !valid_gpu_identity(&edge.from_gpu)
        || !valid_gpu_identity(&edge.to_gpu)
    {
        return false;
    }
    let same_gpu = edge.from_gpu == edge.to_gpu;
    match edge.kind {
        CopyEdgeKindV1::ZeroCopy => {
            same_gpu
                && edge.from_surface == edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::SameGpuDeviceCopy => {
            same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::PeerCopy => {
            !same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format == edge.output_format
                && edge.peer_access == Some(true)
        }
        CopyEdgeKindV1::Conversion => {
            same_gpu
                && edge.from_surface != edge.to_surface
                && edge.input_format != edge.output_format
                && edge.peer_access.is_none()
        }
        CopyEdgeKindV1::HostStaged | CopyEdgeKindV1::Unknown => false,
    }
}

fn validate_persisted_ledger(lease: &CaptureFrameLeaseV1, ledger: &CopyLedgerV1) -> bool {
    if ledger.host_staged_edges != 0 {
        return false;
    }
    let frame = CaptureFrameObservationV1 {
        grab_status: crate::model::CaptureGrabStatusV1::Success,
        frame_sequence: lease.frame_sequence,
        timestamp_us: lease.timestamp_us,
        is_new_frame: lease.new_frame,
        width_px: lease.width_px,
        height_px: lease.height_px,
        pixel_format: lease.pixel_format,
        pitch_bytes: lease.pitch_bytes,
        planes: lease.planes.clone(),
        required_post_processing: lease.required_post_processing,
        cursor_included: lease.cursor_included,
        cursor_mode: lease.cursor_mode,
        source_surface: lease.source_surface.clone(),
        lease_surface: lease.lease_surface.clone(),
        edges: ledger.edges.clone(),
        requested_pixel_format: lease.requested_pixel_format,
        byte_size: lease.byte_size,
        missed_frames: lease.missed_frames,
        direct_capture: lease.direct_capture,
        cursor_requested: lease.cursor_requested,
        cursor_visible: lease.cursor_visible,
        cursor_composited: lease.cursor_composited,
        grab_flags: lease.grab_flags,
        grab_timeout_ms: lease.grab_timeout_ms,
        grab_elapsed_ns: lease.grab_elapsed_ns,
    };
    derive_copy_ledger(&frame).is_some_and(|derived| derived == *ledger)
}

fn derive_cleanup(events: &[CaptureLifecycleEventV1]) -> CaptureCleanupLedgerV1 {
    let split = events
        .iter()
        .position(|event| {
            matches!(
                event,
                CaptureLifecycleEventV1::FrameReleased
                    | CaptureLifecycleEventV1::ApplicationBufferFreed
                    | CaptureLifecycleEventV1::SessionDestroyed
                    | CaptureLifecycleEventV1::HandleDestroyed
                    | CaptureLifecycleEventV1::ContextReleased
                    | CaptureLifecycleEventV1::LibraryUnloaded
                    | CaptureLifecycleEventV1::CudaContextDestroyed
                    | CaptureLifecycleEventV1::CudaLibraryUnloaded
            )
        })
        .unwrap_or(events.len());
    CaptureCleanupLedgerV1 {
        acquired: events[..split].to_vec(),
        released: events[split..].to_vec(),
        complete: cleanup_sequence_complete(events),
    }
}

fn valid_cleanup_ledger(cleanup: &CaptureCleanupLedgerV1) -> bool {
    let mut combined = cleanup.acquired.clone();
    combined.extend_from_slice(&cleanup.released);
    combined.len() <= MAX_CAPTURE_LIFECYCLE_EVENTS_V1 && derive_cleanup(&combined) == *cleanup
}

fn cleanup_sequence_complete(events: &[CaptureLifecycleEventV1]) -> bool {
    events.is_empty()
        || legacy_cleanup_sequence_complete(events)
        || live_cleanup_sequence_complete(events)
}

fn legacy_cleanup_sequence_complete(events: &[CaptureLifecycleEventV1]) -> bool {
    use CaptureLifecycleEventV1::{
        ContextBound, ContextReleased, FrameGrabbed, FrameReleased, LibraryLoaded, LibraryUnloaded,
        SessionCreated, SessionDestroyed, StatusQueried,
    };

    let mut resources = Vec::new();
    let mut status_queried = false;
    let mut cleanup_started = false;
    for event in events {
        match event {
            LibraryLoaded if resources.is_empty() && !cleanup_started => {
                resources.push(LibraryLoaded);
            }
            StatusQueried
                if resources == [LibraryLoaded] && !status_queried && !cleanup_started =>
            {
                status_queried = true;
            }
            ContextBound if resources == [LibraryLoaded] && status_queried && !cleanup_started => {
                resources.push(ContextBound);
            }
            SessionCreated if resources == [LibraryLoaded, ContextBound] && !cleanup_started => {
                resources.push(SessionCreated);
            }
            FrameGrabbed
                if resources == [LibraryLoaded, ContextBound, SessionCreated]
                    && !cleanup_started =>
            {
                resources.push(FrameGrabbed);
            }
            FrameReleased if resources.last() == Some(&FrameGrabbed) => {
                cleanup_started = true;
                resources.pop();
            }
            SessionDestroyed if resources.last() == Some(&SessionCreated) => {
                cleanup_started = true;
                resources.pop();
            }
            ContextReleased if resources.last() == Some(&ContextBound) => {
                cleanup_started = true;
                resources.pop();
            }
            LibraryUnloaded if resources == [LibraryLoaded] => {
                cleanup_started = true;
                resources.pop();
            }
            _ => return false,
        }
    }
    cleanup_started && resources.is_empty()
}

fn live_cleanup_sequence_complete(events: &[CaptureLifecycleEventV1]) -> bool {
    use CaptureLifecycleEventV1::{
        ApplicationBufferAllocated, ApplicationBufferFreed, CudaContextCreated,
        CudaContextDestroyed, CudaLibraryLoaded, CudaLibraryUnloaded, FrameGrabbed, FrameReleased,
        HandleCreated, HandleDestroyed, LibraryLoaded, LibraryUnloaded, SessionCreated,
        SessionDestroyed, StatusQueried, ToCudaSetup,
    };

    const ACQUIRE_ORDER: [CaptureLifecycleEventV1; 9] = [
        CudaLibraryLoaded,
        CudaContextCreated,
        LibraryLoaded,
        HandleCreated,
        StatusQueried,
        SessionCreated,
        ToCudaSetup,
        ApplicationBufferAllocated,
        FrameGrabbed,
    ];
    let split = events
        .iter()
        .position(|event| {
            matches!(
                event,
                FrameReleased
                    | ApplicationBufferFreed
                    | SessionDestroyed
                    | HandleDestroyed
                    | LibraryUnloaded
                    | CudaContextDestroyed
                    | CudaLibraryUnloaded
            )
        })
        .unwrap_or(events.len());
    let acquired = &events[..split];
    if acquired.is_empty()
        || acquired.len() > ACQUIRE_ORDER.len()
        || acquired != &ACQUIRE_ORDER[..acquired.len()]
    {
        return false;
    }
    let has = |event| acquired.contains(&event);
    let mut expected = Vec::new();
    if has(FrameGrabbed) {
        expected.push(FrameReleased);
    }
    if has(ApplicationBufferAllocated) {
        expected.push(ApplicationBufferFreed);
    }
    if has(SessionCreated) {
        expected.push(SessionDestroyed);
    }
    if has(HandleCreated) {
        expected.push(HandleDestroyed);
    }
    if has(LibraryLoaded) {
        expected.push(LibraryUnloaded);
    }
    if has(CudaContextCreated) {
        expected.push(CudaContextDestroyed);
    }
    if has(CudaLibraryLoaded) {
        expected.push(CudaLibraryUnloaded);
    }
    events[split..] == expected
}

fn valid_live_lifecycle(cleanup: &CaptureCleanupLedgerV1) -> bool {
    let mut events = cleanup.acquired.clone();
    events.extend_from_slice(&cleanup.released);
    cleanup.complete
        && events
            == [
                CaptureLifecycleEventV1::CudaLibraryLoaded,
                CaptureLifecycleEventV1::CudaContextCreated,
                CaptureLifecycleEventV1::LibraryLoaded,
                CaptureLifecycleEventV1::HandleCreated,
                CaptureLifecycleEventV1::StatusQueried,
                CaptureLifecycleEventV1::SessionCreated,
                CaptureLifecycleEventV1::ToCudaSetup,
                CaptureLifecycleEventV1::ApplicationBufferAllocated,
                CaptureLifecycleEventV1::FrameGrabbed,
                CaptureLifecycleEventV1::FrameReleased,
                CaptureLifecycleEventV1::ApplicationBufferFreed,
                CaptureLifecycleEventV1::SessionDestroyed,
                CaptureLifecycleEventV1::HandleDestroyed,
                CaptureLifecycleEventV1::LibraryUnloaded,
                CaptureLifecycleEventV1::CudaContextDestroyed,
                CaptureLifecycleEventV1::CudaLibraryUnloaded,
            ]
}

fn grab_failure(status: crate::model::CaptureGrabStatusV1) -> CaptureFailureV1 {
    match status {
        crate::model::CaptureGrabStatusV1::Success => CaptureFailureV1::InvalidFrame,
        crate::model::CaptureGrabStatusV1::NoNewFrame => CaptureFailureV1::NoNewFrame,
        crate::model::CaptureGrabStatusV1::AccessDenied => CaptureFailureV1::AccessDenied,
        crate::model::CaptureGrabStatusV1::ProtectedContent => CaptureFailureV1::ProtectedContent,
        crate::model::CaptureGrabStatusV1::Busy => CaptureFailureV1::Busy,
        crate::model::CaptureGrabStatusV1::DriverError => CaptureFailureV1::InvalidFrame,
        crate::model::CaptureGrabStatusV1::ApiMismatch => CaptureFailureV1::ApiMismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(replay_nvfbc_source))]
    #[test]
    fn host03_no_source_provider_reports_compiled_gate_unavailable() {
        let source = compiled_capture_source();
        assert_eq!(source.status, CaptureSourceStatusV1::Unavailable);
        assert_eq!(source.identity, None);
        assert_eq!(source.api_version, None);
        assert_eq!(source.nvfbc_header_sha256, None);
        assert_eq!(source.cuda_header_sha256, None);
        assert_eq!(
            LiveUnavailableCaptureProvider.observe().failure,
            Some(CaptureFailureV1::SourceUnavailable)
        );
    }

    #[test]
    fn host03_source_abi_fixture_names_every_future_native_boundary() {
        assert_eq!(
            nvfbc_abi_contract_names(),
            &[
                "NVFBC_API_FUNCTION_LIST",
                "NVFBC_CREATE_HANDLE_PARAMS",
                "NVFBC_GET_STATUS_PARAMS",
                "NVFBC_BIND_CONTEXT_PARAMS",
                "NVFBC_CREATE_CAPTURE_SESSION_PARAMS",
                "NVFBC_TOCUDA_SETUP_PARAMS",
                "NVFBC_TOCUDA_GRAB_FRAME_PARAMS",
                "NVFBC_FRAME_GRAB_INFO",
                "NVFBC_DESTROY_CAPTURE_SESSION_PARAMS",
                "NVFBC_RELEASE_CONTEXT_PARAMS",
                "NVFBC_DESTROY_HANDLE_PARAMS",
                "CUcontext",
                "CUdeviceptr",
                "CUuuid",
                "cudaTypedefs.h",
            ]
        );
        let oracle = include_str!("../native/nvfbc_abi_oracle.c");
        for declaration in nvfbc_abi_contract_names() {
            assert!(
                oracle.contains(declaration),
                "ABI oracle missing {declaration}"
            );
        }
        for export in [
            "replay_nvfbc_sizeof_api_function_list",
            "replay_nvfbc_sizeof_tocuda_grab_frame_params",
            "replay_nvfbc_sizeof_frame_grab_info",
            "replay_nvfbc_sizeof_cuda_context",
            "replay_nvfbc_sizeof_cuda_device_pointer",
            "replay_nvfbc_api_version",
            "replay_nvfbc_api_version_major",
            "replay_nvfbc_api_version_minor",
            "replay_nvfbc_offset_api_composite_cursor",
            "replay_nvfbc_cuda_signature_mask",
            "replay_nvfbc_contract_mask",
        ] {
            assert!(oracle.contains(export), "ABI oracle missing {export}");
        }
    }

    #[test]
    fn host03_source_abi_rejects_legacy_18_as_live() {
        let legacy = CapturePrimitiveObservationV1 {
            schema: NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1.to_owned(),
            provider: CaptureProviderKindV1::SourceAuthenticated,
            source: CaptureSourceEvidenceV1 {
                status: CaptureSourceStatusV1::Authenticated,
                identity: Some("nvidia-nvfbc-api-1.8-cuda-driver-api".to_owned()),
                api_version: Some(18),
                nvfbc_header_sha256: Some(
                    "1111111111111111111111111111111111111111111111111111111111111111"
                        .parse()
                        .expect("test digest"),
                ),
                cuda_header_sha256: Some(
                    "2222222222222222222222222222222222222222222222222222222222222222"
                        .parse()
                        .expect("test digest"),
                ),
                cuda_typedefs_header_sha256: Some(
                    "3333333333333333333333333333333333333333333333333333333333333333"
                        .parse()
                        .expect("test digest"),
                ),
                nvfbc_runtime_library: None,
                cuda_runtime_library: None,
                cuda_driver_version: None,
            },
            binding: None,
            frame: None,
            lifecycle: Vec::new(),
            failure: Some(CaptureFailureV1::SourceMismatch),
            nvfbc_status_raw: Some(0),
        };

        assert!(!valid_provider_source(&legacy));
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_consumes_authenticated_oracle_and_exact_versions() {
        let source = compiled_capture_source();
        assert_eq!(source.status, CaptureSourceStatusV1::Authenticated);
        assert_eq!(source.identity.as_deref(), Some(LIVE_SOURCE_IDENTITY));
        assert_eq!(source.api_version, Some(0x109));
        assert!(source.nvfbc_header_sha256.is_some());
        assert!(source.cuda_header_sha256.is_some());
        assert!(source.cuda_typedefs_header_sha256.is_some());
        assert!(authenticate_runtime_sources(&source).is_ok());
        assert!(verify_compiled_abi().is_ok());

        let oracle = include_str!("../native/nvfbc_abi_oracle.c");
        for contract in [
            "PNVFBCCREATEINSTANCE",
            "PNVFBCCREATEHANDLE",
            "PNVFBCDESTROYHANDLE",
            "PNVFBCGETSTATUS",
            "PNVFBCCREATECAPTURESESSION",
            "PNVFBCDESTROYCAPTURESESSION",
            "PNVFBCTOCUDASETUP",
            "PNVFBCTOCUDAGRABFRAME",
            "CU_GET_PROC_ADDRESS_LEGACY_STREAM",
        ] {
            assert!(oracle.contains(contract), "oracle missing {contract}");
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_persisted_digests_match_the_current_executable() {
        let mut source = compiled_capture_source();
        source.nvfbc_runtime_library = Some("libnvidia-fbc.so.610.43.03".to_owned());
        source.cuda_runtime_library = Some("libcuda.so.610.43.03".to_owned());
        source.cuda_driver_version = Some(13_030);
        assert!(valid_source(&source));
        assert!(valid_live_source(&source));

        let wrong: crate::Sha256DigestV1 =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .expect("test digest");
        for mutated in [
            CaptureSourceEvidenceV1 {
                nvfbc_header_sha256: Some(wrong),
                ..source.clone()
            },
            CaptureSourceEvidenceV1 {
                cuda_header_sha256: Some(wrong),
                ..source.clone()
            },
            CaptureSourceEvidenceV1 {
                cuda_typedefs_header_sha256: Some(wrong),
                ..source.clone()
            },
        ] {
            assert!(!valid_source(&mutated));
            assert!(!valid_live_source(&mutated));
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_function_table_rejects_wrong_version_and_nulls() {
        let mut functions = NvFbcApiFunctionList::default();
        for index in [1_usize, 2, 3, 4, 5, 8, 9] {
            functions.entries[index] = std::ptr::dangling_mut::<c_void>();
        }
        assert!(validate_nvfbc_function_list(&functions).is_ok());

        let mut wrong_version = functions;
        wrong_version.version = 18;
        assert_eq!(
            validate_nvfbc_function_list(&wrong_version)
                .expect_err("legacy API must fail")
                .failure,
            CaptureFailureV1::ApiMismatch
        );
        for index in [1_usize, 2, 3, 4, 5, 8, 9] {
            let mut missing = functions;
            missing.entries[index] = std::ptr::null_mut();
            assert_eq!(
                validate_nvfbc_function_list(&missing)
                    .expect_err("required function must be present")
                    .failure,
                CaptureFailureV1::ApiMismatch,
                "function-list index {index}"
            );
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_status_codes_map_without_invented_drm_meanings() {
        for status in 0..=21 {
            let expected = match status {
                1 => CaptureFailureV1::ApiMismatch,
                6 => CaptureFailureV1::Busy,
                16 => CaptureFailureV1::BindingMismatch,
                _ => CaptureFailureV1::InvalidFrame,
            };
            assert_eq!(map_nvfbc_failure(status), expected, "status {status}");
        }
        assert_eq!(map_nvfbc_failure(22), CaptureFailureV1::InvalidFrame);
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_exact_output_binding_rejects_aliases_and_conflicts() {
        fn output(id: u32, name: &[u8], tracked_box: NvFbcBox) -> NvFbcRandrOutputInfo {
            let mut output = NvFbcRandrOutputInfo {
                id,
                tracked_box,
                ..NvFbcRandrOutputInfo::default()
            };
            for (destination, source) in output.name.iter_mut().zip(name.iter().copied()) {
                *destination = source as c_char;
            }
            output
        }

        let exact_box = NvFbcBox {
            x: 0,
            y: 0,
            w: 3840,
            h: 2160,
        };
        let exact = output(540, b"DP-0.3", exact_box);
        assert!(validate_nvfbc_output_binding(&[exact], 540, "DP-0.3", 0, 0, 3840, 2160).is_ok());

        let mut non_terminated = exact;
        non_terminated.name = [b'A' as c_char; 128];
        for (case_index, invalid) in [
            vec![output(540, b"dp-0.3", exact_box)],
            vec![output(541, b"DP-0.3", exact_box)],
            vec![output(540, b"DP-0.4", exact_box)],
            vec![output(0, b"DP-0.3", exact_box)],
            vec![output(
                540,
                b"DP-0.3",
                NvFbcBox {
                    w: 1920,
                    ..exact_box
                },
            )],
            vec![output(541, b"DP-0.4", exact_box)],
            vec![exact, exact],
            vec![non_terminated],
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                validate_nvfbc_output_binding(&invalid, 540, "DP-0.3", 0, 0, 3840, 2160).is_err(),
                "invalid output case {case_index}"
            );
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_status_readiness_is_fail_closed() {
        let ready = NvFbcGetStatusParams {
            version: NVFBC_GET_STATUS_PARAMS_VERSION,
            is_capture_possible: 1,
            currently_capturing: 0,
            can_create_now: 1,
            screen_size: NvFbcSize { w: 3840, h: 2160 },
            xrandr_available: 1,
            output_count: 1,
            nvfbc_version: LIVE_NVFBC_API_VERSION,
            in_modeset: 0,
            ..NvFbcGetStatusParams::default()
        };
        assert!(validate_nvfbc_status_readiness(&ready).is_ok());

        for invalid in [
            NvFbcGetStatusParams {
                currently_capturing: 1,
                ..ready
            },
            NvFbcGetStatusParams {
                is_capture_possible: 0,
                ..ready
            },
            NvFbcGetStatusParams {
                can_create_now: 0,
                ..ready
            },
            NvFbcGetStatusParams {
                xrandr_available: 0,
                ..ready
            },
            NvFbcGetStatusParams {
                in_modeset: 1,
                ..ready
            },
            NvFbcGetStatusParams {
                output_count: 6,
                ..ready
            },
            NvFbcGetStatusParams {
                nvfbc_version: 18,
                ..ready
            },
            NvFbcGetStatusParams {
                currently_capturing: 2,
                ..ready
            },
        ] {
            assert!(validate_nvfbc_status_readiness(&invalid).is_err());
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_source_abi_frame_layout_freshness_and_cursor_truth_table() {
        assert_eq!(
            exact_frame_byte_size(3840, 2160, CapturePixelFormatV1::Nv12),
            Some(12_441_600)
        );
        assert_eq!(
            exact_frame_byte_size(3840, 2160, CapturePixelFormatV1::Yuv444p),
            Some(24_883_200)
        );
        assert_eq!(
            exact_frame_byte_size(3840, 2160, CapturePixelFormatV1::Bgra),
            Some(33_177_600)
        );
        assert_eq!(
            exact_frame_byte_size(3839, 2160, CapturePixelFormatV1::Nv12),
            None
        );
        assert_eq!(
            exact_frame_byte_size(3840, 2159, CapturePixelFormatV1::Yuv444p),
            None
        );
        assert_eq!(
            nv12_planes(3840, 2160).expect("exact NV12 planes"),
            vec![
                crate::model::CapturePlaneV1 {
                    index: 0,
                    offset_bytes: 0,
                    stride_bytes: 3840,
                    size_bytes: 8_294_400,
                },
                crate::model::CapturePlaneV1 {
                    index: 1,
                    offset_bytes: 8_294_400,
                    stride_bytes: 3840,
                    size_bytes: 4_147_200,
                },
            ]
        );

        let frame = NvFbcFrameGrabInfo {
            width: 3840,
            height: 2160,
            byte_size: 12_441_600,
            current_frame: 0,
            is_new_frame: 1,
            timestamp_us: 0,
            missed_frames: 0,
            required_post_processing: 1,
            direct_capture: 0,
            cursor_visible: 0,
            cursor_composited: 0,
        };
        for (visible, composited) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
            let candidate = NvFbcFrameGrabInfo {
                cursor_visible: visible,
                cursor_composited: composited,
                ..frame
            };
            assert!(
                validate_frame_contract(&candidate, 3840, 2160, 12_441_600).is_ok(),
                "visibility and composition are independent API facts"
            );
        }
        let hidden_composited = CaptureFrameObservationV1 {
            grab_status: crate::model::CaptureGrabStatusV1::Success,
            frame_sequence: 1,
            timestamp_us: 1,
            is_new_frame: true,
            width_px: 3840,
            height_px: 2160,
            pixel_format: CapturePixelFormatV1::Nv12,
            pitch_bytes: 3840,
            planes: nv12_planes(3840, 2160).expect("exact NV12 planes"),
            required_post_processing: true,
            cursor_included: true,
            cursor_mode: crate::model::CaptureCursorModeV1::NvfbcComposited,
            source_surface: "selected-scanout-bgra".to_owned(),
            lease_surface: "application-owned-nv12".to_owned(),
            edges: Vec::new(),
            requested_pixel_format: Some(CapturePixelFormatV1::Nv12),
            byte_size: Some(12_441_600),
            missed_frames: Some(0),
            direct_capture: Some(false),
            cursor_requested: Some(true),
            cursor_visible: Some(false),
            cursor_composited: Some(true),
            grab_flags: Some(NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT_IF_NEW_FRAME_READY),
            grab_timeout_ms: Some(NVFBC_GRAB_TIMEOUT_MS),
            grab_elapsed_ns: Some(1),
        };
        assert!(
            valid_frame_geometry(&hidden_composited),
            "live API 1.9 can report a hidden cursor through the composition path"
        );
        assert!(valid_live_lease(&CaptureFrameLeaseV1 {
            frame_sequence: hidden_composited.frame_sequence,
            timestamp_us: hidden_composited.timestamp_us,
            new_frame: hidden_composited.is_new_frame,
            width_px: hidden_composited.width_px,
            height_px: hidden_composited.height_px,
            pixel_format: hidden_composited.pixel_format,
            pitch_bytes: hidden_composited.pitch_bytes,
            planes: hidden_composited.planes.clone(),
            required_post_processing: hidden_composited.required_post_processing,
            cursor_included: hidden_composited.cursor_included,
            cursor_mode: hidden_composited.cursor_mode,
            source_surface: hidden_composited.source_surface.clone(),
            lease_surface: hidden_composited.lease_surface.clone(),
            requested_pixel_format: hidden_composited.requested_pixel_format,
            byte_size: hidden_composited.byte_size,
            missed_frames: hidden_composited.missed_frames,
            direct_capture: hidden_composited.direct_capture,
            cursor_requested: hidden_composited.cursor_requested,
            cursor_visible: hidden_composited.cursor_visible,
            cursor_composited: hidden_composited.cursor_composited,
            grab_flags: hidden_composited.grab_flags,
            grab_timeout_ms: hidden_composited.grab_timeout_ms,
            grab_elapsed_ns: hidden_composited.grab_elapsed_ns,
        }));

        let stale = NvFbcFrameGrabInfo {
            is_new_frame: 0,
            ..frame
        };
        assert_eq!(
            validate_frame_contract(&stale, 3840, 2160, 12_441_600)
                .expect_err("old frame must fail")
                .failure,
            CaptureFailureV1::NoNewFrame
        );
        for invalid in [
            NvFbcFrameGrabInfo {
                required_post_processing: 0,
                ..frame
            },
            NvFbcFrameGrabInfo {
                direct_capture: 1,
                ..frame
            },
            NvFbcFrameGrabInfo {
                byte_size: 12_441_599,
                ..frame
            },
            NvFbcFrameGrabInfo {
                is_new_frame: 2,
                ..frame
            },
        ] {
            assert!(validate_frame_contract(&invalid, 3840, 2160, 12_441_600).is_err());
        }
    }

    #[test]
    fn host03_cleanup_live_partial_acquisitions_unwind_once_in_reverse_order() {
        use CaptureLifecycleEventV1::*;
        for events in [
            vec![CudaLibraryLoaded, CudaLibraryUnloaded],
            vec![
                CudaLibraryLoaded,
                CudaContextCreated,
                CudaContextDestroyed,
                CudaLibraryUnloaded,
            ],
            vec![
                CudaLibraryLoaded,
                CudaContextCreated,
                LibraryLoaded,
                LibraryUnloaded,
                CudaContextDestroyed,
                CudaLibraryUnloaded,
            ],
            vec![
                CudaLibraryLoaded,
                CudaContextCreated,
                LibraryLoaded,
                HandleCreated,
                HandleDestroyed,
                LibraryUnloaded,
                CudaContextDestroyed,
                CudaLibraryUnloaded,
            ],
            vec![
                CudaLibraryLoaded,
                CudaContextCreated,
                LibraryLoaded,
                HandleCreated,
                StatusQueried,
                SessionCreated,
                ToCudaSetup,
                ApplicationBufferAllocated,
                FrameGrabbed,
                FrameReleased,
                ApplicationBufferFreed,
                SessionDestroyed,
                HandleDestroyed,
                LibraryUnloaded,
                CudaContextDestroyed,
                CudaLibraryUnloaded,
            ],
        ] {
            let cleanup = derive_cleanup(&events);
            assert!(cleanup.complete, "proper live reverse cleanup: {events:?}");
            assert!(valid_cleanup_ledger(&cleanup));
        }
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_cleanup_slow_success_records_context_and_handle_before_unwind() {
        use CaptureLifecycleEventV1::*;
        let slow_cuda = NativeCallOutcome {
            status: CUDA_SUCCESS,
            elapsed_ns: MAX_NATIVE_CALL_ELAPSED_NS + 1,
        };
        let mut context_owned = false;
        let mut context_events = vec![CudaLibraryLoaded];
        let error = record_cuda_acquisition(
            slow_cuda,
            &mut context_owned,
            &mut context_events,
            CudaContextCreated,
        )
        .expect_err("slow successful context creation must reject after ownership");
        assert_eq!(error.failure, CaptureFailureV1::WorkerRejected);
        assert!(context_owned);
        assert_eq!(context_events, [CudaLibraryLoaded, CudaContextCreated]);
        record_cuda_release(
            NativeCallOutcome {
                status: CUDA_SUCCESS,
                elapsed_ns: 0,
            },
            &mut context_events,
            CudaContextDestroyed,
        )
        .expect("owned context must unwind");
        context_events.push(CudaLibraryUnloaded);
        assert!(derive_cleanup(&context_events).complete);

        let slow_nvfbc = NativeCallOutcome {
            status: NVFBC_SUCCESS,
            elapsed_ns: MAX_NATIVE_CALL_ELAPSED_NS + 1,
        };
        let mut handle_owned = false;
        let mut handle_events = vec![CudaLibraryLoaded, CudaContextCreated, LibraryLoaded];
        let error = record_nvfbc_acquisition(
            slow_nvfbc,
            &mut handle_owned,
            &mut handle_events,
            HandleCreated,
        )
        .expect_err("slow successful handle creation must reject after ownership");
        assert_eq!(error.failure, CaptureFailureV1::WorkerRejected);
        assert!(handle_owned);
        record_nvfbc_release(
            NativeCallOutcome {
                status: NVFBC_SUCCESS,
                elapsed_ns: 0,
            },
            &mut handle_events,
            HandleDestroyed,
        )
        .expect("owned handle must unwind");
        handle_events.extend([LibraryUnloaded, CudaContextDestroyed, CudaLibraryUnloaded]);
        assert!(derive_cleanup(&handle_events).complete);
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_cleanup_backend_mismatch_preserves_handle_for_destroy() {
        use CaptureLifecycleEventV1::*;
        let mut handle_owned = false;
        let mut events = vec![CudaLibraryLoaded, CudaContextCreated, LibraryLoaded];
        record_nvfbc_acquisition(
            NativeCallOutcome {
                status: NVFBC_SUCCESS,
                elapsed_ns: 0,
            },
            &mut handle_owned,
            &mut events,
            HandleCreated,
        )
        .expect("successful create must establish ownership");
        let error = validate_created_handle_backend(2, NVFBC_SUCCESS)
            .expect_err("non-X11 backend postcondition must reject");
        assert_eq!(error.failure, CaptureFailureV1::BindingMismatch);
        assert!(handle_owned);
        record_nvfbc_release(
            NativeCallOutcome {
                status: NVFBC_SUCCESS,
                elapsed_ns: 0,
            },
            &mut events,
            HandleDestroyed,
        )
        .expect("backend mismatch must still destroy the owned handle");
        handle_owned = false;
        events.extend([LibraryUnloaded, CudaContextDestroyed, CudaLibraryUnloaded]);
        assert!(!handle_owned);
        assert!(derive_cleanup(&events).complete);
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_cleanup_failed_sync_never_releases_borrowed_frame() {
        use CaptureLifecycleEventV1::*;
        let mut events = vec![
            CudaLibraryLoaded,
            CudaContextCreated,
            LibraryLoaded,
            HandleCreated,
            StatusQueried,
            SessionCreated,
            ToCudaSetup,
            ApplicationBufferAllocated,
            FrameGrabbed,
        ];
        let mut frame_borrowed = true;
        let mut copy_pending = true;
        let error = record_frame_sync(
            NativeCallOutcome {
                status: 1,
                elapsed_ns: 0,
            },
            &mut frame_borrowed,
            &mut copy_pending,
            &mut events,
        )
        .expect_err("failed synchronization must require process containment");
        assert_eq!(error.failure, CaptureFailureV1::CleanupUncertain);
        assert!(error.containment_required);
        assert!(frame_borrowed);
        assert!(copy_pending);
        assert!(!events.contains(&FrameReleased));
        assert!(!derive_cleanup(&events).complete);
    }

    #[cfg(replay_nvfbc_source)]
    #[test]
    fn host03_cleanup_failed_releases_never_emit_success_events() {
        use CaptureLifecycleEventV1::*;
        let acquired = vec![
            CudaLibraryLoaded,
            CudaContextCreated,
            LibraryLoaded,
            HandleCreated,
            StatusQueried,
            SessionCreated,
            ToCudaSetup,
            ApplicationBufferAllocated,
            FrameGrabbed,
            FrameReleased,
        ];

        let mut failed_free = acquired.clone();
        assert!(
            record_cuda_release(
                NativeCallOutcome {
                    status: 1,
                    elapsed_ns: 0,
                },
                &mut failed_free,
                ApplicationBufferFreed,
            )
            .is_err()
        );
        assert!(!failed_free.contains(&ApplicationBufferFreed));
        assert!(!derive_cleanup(&failed_free).complete);

        let mut failed_session = acquired.clone();
        failed_session.push(ApplicationBufferFreed);
        assert!(
            record_nvfbc_release(
                NativeCallOutcome {
                    status: 2,
                    elapsed_ns: 0,
                },
                &mut failed_session,
                SessionDestroyed,
            )
            .is_err()
        );
        assert!(!failed_session.contains(&SessionDestroyed));
        assert!(!derive_cleanup(&failed_session).complete);

        let mut failed_handle = acquired.clone();
        failed_handle.extend([ApplicationBufferFreed, SessionDestroyed]);
        assert!(
            record_nvfbc_release(
                NativeCallOutcome {
                    status: 2,
                    elapsed_ns: 0,
                },
                &mut failed_handle,
                HandleDestroyed,
            )
            .is_err()
        );
        assert!(!failed_handle.contains(&HandleDestroyed));
        assert!(!derive_cleanup(&failed_handle).complete);

        let mut failed_context = acquired;
        failed_context.extend([
            ApplicationBufferFreed,
            SessionDestroyed,
            HandleDestroyed,
            LibraryUnloaded,
        ]);
        assert!(
            record_cuda_release(
                NativeCallOutcome {
                    status: 1,
                    elapsed_ns: 0,
                },
                &mut failed_context,
                CudaContextDestroyed,
            )
            .is_err()
        );
        assert!(!failed_context.contains(&CudaContextDestroyed));
        assert!(!derive_cleanup(&failed_context).complete);
    }

    #[test]
    fn host03_cleanup_partial_acquisitions_unwind_once_in_reverse_order() {
        use CaptureLifecycleEventV1::*;
        for events in [
            vec![LibraryLoaded, LibraryUnloaded],
            vec![LibraryLoaded, StatusQueried, LibraryUnloaded],
            vec![
                LibraryLoaded,
                StatusQueried,
                ContextBound,
                ContextReleased,
                LibraryUnloaded,
            ],
            vec![
                LibraryLoaded,
                StatusQueried,
                ContextBound,
                SessionCreated,
                SessionDestroyed,
                ContextReleased,
                LibraryUnloaded,
            ],
            FULL_LIFECYCLE.to_vec(),
        ] {
            let cleanup = derive_cleanup(&events);
            assert!(cleanup.complete, "proper reverse cleanup: {events:?}");
            assert!(valid_cleanup_ledger(&cleanup));
        }
    }

    #[test]
    fn host03_cleanup_duplicate_or_out_of_order_release_is_uncertain() {
        use CaptureLifecycleEventV1::*;
        for events in [
            vec![LibraryLoaded, LibraryUnloaded, LibraryUnloaded],
            vec![
                LibraryLoaded,
                ContextBound,
                LibraryUnloaded,
                ContextReleased,
            ],
            vec![
                LibraryLoaded,
                ContextBound,
                ContextReleased,
                ContextReleased,
                LibraryUnloaded,
            ],
        ] {
            let cleanup = derive_cleanup(&events);
            assert!(!cleanup.complete, "invalid cleanup: {events:?}");
        }
    }
}
