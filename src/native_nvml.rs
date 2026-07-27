use crate::{Sha256DigestV1, sha256_file};
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::{CStr, c_char, c_uint, c_void};
use std::path::PathBuf;
#[cfg(replay_nvml_source)]
use std::str::FromStr;

const NVML_LIBRARY_NAME: &str = "libnvidia-ml.so.1";
const NVML_SOURCE_ROOT_ENV: &str = "REPLAY_NVML_SDK_ROOT";
const NVML_HEADER_NAME: &str = "nvml.h";
const NVML_SOURCE_IDENTITY: &str = "nvidia-nvml-api-13";
const NVML_SUCCESS: NvmlReturn = 0;
#[cfg(replay_nvml_source)]
const NVML_API_VERSION: usize = 13;
const NVML_PCI_LEGACY_BUFFER_SIZE: usize = 16;
const NVML_PCI_BUFFER_SIZE: usize = 32;
const NVML_UUID_BUFFER_SIZE: usize = 96;
const NVML_DRIVER_VERSION_BUFFER_SIZE: usize = 80;
#[cfg(replay_nvml_source)]
const NVML_SIGNATURE_MASK: usize = 0x7f;
const MAX_NVML_DEVICES: u32 = 64;
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;

type NvmlReturn = c_uint;
type NvmlDevice = *mut c_void;
type NvmlInitFn = unsafe extern "C" fn() -> NvmlReturn;
type NvmlShutdownFn = unsafe extern "C" fn() -> NvmlReturn;
type NvmlDriverVersionFn = unsafe extern "C" fn(*mut c_char, c_uint) -> NvmlReturn;
type NvmlDeviceCountFn = unsafe extern "C" fn(*mut c_uint) -> NvmlReturn;
type NvmlDeviceHandleFn = unsafe extern "C" fn(c_uint, *mut NvmlDevice) -> NvmlReturn;
type NvmlDeviceUuidFn = unsafe extern "C" fn(NvmlDevice, *mut c_char, c_uint) -> NvmlReturn;
type NvmlDevicePciFn = unsafe extern "C" fn(NvmlDevice, *mut NvmlPciInfo) -> NvmlReturn;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct NvmlSourceMapEntryV1 {
    pub rust_declaration: &'static str,
    pub official_declaration: &'static str,
    pub oracle_check: &'static str,
}

pub const NVML_SOURCE_MAP_V1: &[NvmlSourceMapEntryV1] = &[
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlInitFn",
        official_declaration: "nvmlInit_v2",
        oracle_check: "replay_nvml_signature_mask bit 0",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlShutdownFn",
        official_declaration: "nvmlShutdown",
        oracle_check: "replay_nvml_signature_mask bit 1",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlDriverVersionFn",
        official_declaration: "nvmlSystemGetDriverVersion",
        oracle_check: "replay_nvml_signature_mask bit 2",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlDeviceCountFn",
        official_declaration: "nvmlDeviceGetCount_v2",
        oracle_check: "replay_nvml_signature_mask bit 3",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlDeviceHandleFn",
        official_declaration: "nvmlDeviceGetHandleByIndex_v2",
        oracle_check: "replay_nvml_signature_mask bit 4",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlDeviceUuidFn",
        official_declaration: "nvmlDeviceGetUUID",
        oracle_check: "replay_nvml_signature_mask bit 5",
    },
    NvmlSourceMapEntryV1 {
        rust_declaration: "NvmlDevicePciFn",
        official_declaration: "nvmlDeviceGetPciInfo_v3",
        oracle_check: "replay_nvml_signature_mask bit 6",
    },
];

#[repr(C)]
#[derive(Clone, Copy)]
struct NvmlPciInfo {
    bus_id_legacy: [c_char; NVML_PCI_LEGACY_BUFFER_SIZE],
    domain: c_uint,
    bus: c_uint,
    device: c_uint,
    pci_device_id: c_uint,
    pci_subsystem_id: c_uint,
    bus_id: [c_char; NVML_PCI_BUFFER_SIZE],
}

impl Default for NvmlPciInfo {
    fn default() -> Self {
        Self {
            bus_id_legacy: [0; NVML_PCI_LEGACY_BUFFER_SIZE],
            domain: 0,
            bus: 0,
            device: 0,
            pci_device_id: 0,
            pci_subsystem_id: 0,
            bus_id: [0; NVML_PCI_BUFFER_SIZE],
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvmlSourceFailureV1 {
    Unavailable,
    Ambiguous,
    DigestMismatch,
    AbiMismatch,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NvmlRuntimeFailureV1 {
    LibraryLoad,
    SymbolMissing,
    Initialize,
    DriverVersion,
    DeviceCount,
    ZeroDevices,
    DeviceHandle,
    DeviceUuid,
    DevicePci,
    DeviceIdentity,
    Shutdown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlDeviceObservationV1 {
    pub uuid: String,
    pub pci_bdf: String,
}

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlRuntimeObservationV1 {
    pub loaded: bool,
    pub initialized: bool,
    pub userspace_driver_version: Option<String>,
    pub devices: Vec<NvmlDeviceObservationV1>,
    pub shutdown_attempted: bool,
    pub shutdown_succeeded: bool,
    pub failure: Option<NvmlRuntimeFailureV1>,
}

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlObservationV1 {
    pub source_attempted: bool,
    pub source_identity: Option<String>,
    pub source_sha256: Option<Sha256DigestV1>,
    pub abi_verified: bool,
    pub source_failure: Option<NvmlSourceFailureV1>,
    pub runtime: NvmlRuntimeObservationV1,
}

impl NvmlObservationV1 {
    pub(crate) fn is_valid(&self) -> bool {
        if !self.source_attempted {
            return self.source_identity.is_none()
                && self.source_sha256.is_none()
                && !self.abi_verified
                && self.source_failure.is_none()
                && self.runtime == NvmlRuntimeObservationV1::default();
        }
        if self
            .source_identity
            .as_deref()
            .is_some_and(|identity| identity != NVML_SOURCE_IDENTITY)
            || self.runtime.devices.len() > MAX_NVML_DEVICES as usize
            || self
                .runtime
                .userspace_driver_version
                .as_deref()
                .is_some_and(|version| !valid_version(version))
        {
            return false;
        }
        let mut uuids = HashSet::new();
        let mut pci_ids = HashSet::new();
        if self.runtime.devices.iter().any(|device| {
            !valid_uuid(&device.uuid)
                || !valid_pci_bdf(&device.pci_bdf)
                || !uuids.insert(device.uuid.as_str())
                || !pci_ids.insert(device.pci_bdf.as_str())
        }) {
            return false;
        }
        if self.runtime.shutdown_succeeded && !self.runtime.shutdown_attempted {
            return false;
        }
        if self.runtime.initialized && !self.runtime.loaded {
            return false;
        }
        if self.runtime.shutdown_attempted && !self.runtime.initialized {
            return false;
        }
        if !self.runtime.initialized
            && (self.runtime.userspace_driver_version.is_some() || !self.runtime.devices.is_empty())
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NvmlEvidenceStatusV1 {
    Pass,
    Fail,
    Unproven,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlReasonV1 {
    pub code: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NvmlEvidenceV1 {
    pub status: NvmlEvidenceStatusV1,
    pub source_identity: Option<String>,
    pub source_sha256: Option<Sha256DigestV1>,
    pub abi_verified: bool,
    pub runtime_loaded: bool,
    pub initialized: bool,
    pub kernel_driver_version: Option<String>,
    pub userspace_driver_version: Option<String>,
    pub device_count: u32,
    pub devices: Vec<NvmlDeviceObservationV1>,
    pub shutdown_attempted: bool,
    pub shutdown_succeeded: bool,
    pub reasons: Vec<NvmlReasonV1>,
}

impl NvmlEvidenceV1 {
    pub(crate) fn is_valid(&self) -> bool {
        if self.device_count as usize != self.devices.len()
            || self.devices.len() > MAX_NVML_DEVICES as usize
            || self
                .kernel_driver_version
                .as_deref()
                .is_some_and(|version| !valid_version(version))
            || self
                .userspace_driver_version
                .as_deref()
                .is_some_and(|version| !valid_version(version))
            || self.shutdown_succeeded && !self.shutdown_attempted
            || self.initialized && !self.runtime_loaded
            || self.shutdown_attempted && !self.initialized
        {
            return false;
        }
        let mut uuids = HashSet::new();
        let mut pci_ids = HashSet::new();
        if self.devices.iter().any(|device| {
            !valid_uuid(&device.uuid)
                || !valid_pci_bdf(&device.pci_bdf)
                || !uuids.insert(device.uuid.as_str())
                || !pci_ids.insert(device.pci_bdf.as_str())
        }) {
            return false;
        }
        match (
            self.source_identity.as_deref(),
            self.source_sha256,
            compiled_source_metadata(),
        ) {
            (None, None, _) => {}
            (Some(identity), Some(digest), Some(compiled))
                if identity == compiled.identity && digest == compiled.sha256 => {}
            _ => return false,
        }
        let allowed = allowed_nvml_reasons();
        let mut reason_codes = HashSet::new();
        if self
            .reasons
            .iter()
            .any(|reason| !allowed.contains(reason) || !reason_codes.insert(reason.code.as_str()))
        {
            return false;
        }

        let complete_runtime = self.abi_verified
            && self.runtime_loaded
            && self.initialized
            && self.device_count > 0
            && self.shutdown_attempted
            && self.shutdown_succeeded
            && self.kernel_driver_version.is_some()
            && self.userspace_driver_version.is_some();
        match self.status {
            NvmlEvidenceStatusV1::Pass => {
                self.reasons.is_empty()
                    && complete_runtime
                    && self.source_identity.is_some()
                    && self.source_sha256.is_some()
                    && self.kernel_driver_version == self.userspace_driver_version
            }
            NvmlEvidenceStatusV1::Fail => {
                if self.reasons.is_empty() {
                    return false;
                }
                let mismatch = self.kernel_driver_version.is_some()
                    && self.userspace_driver_version.is_some()
                    && self.kernel_driver_version != self.userspace_driver_version;
                mismatch
                    == self
                        .reasons
                        .iter()
                        .any(|reason| reason.code == "NVIDIA_VERSION_MISMATCH")
                    && (self.shutdown_succeeded
                        || self
                            .reasons
                            .iter()
                            .any(|reason| reason.code == "NVML_SHUTDOWN_FAILED")
                        || !self.initialized)
            }
            NvmlEvidenceStatusV1::Unproven => {
                !self.abi_verified
                    && !self.runtime_loaded
                    && !self.initialized
                    && self.source_identity.is_none()
                    && self.source_sha256.is_none()
                    && self.device_count == 0
                    && !self.shutdown_attempted
                    && !self.shutdown_succeeded
                    && self.reasons.len() == 1
                    && self.reasons[0].code == "NVML_SOURCE_UNAVAILABLE"
            }
        }
    }
}

pub trait NvmlProvider {
    fn observe(&self) -> NvmlObservationV1;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeNvmlProvider;

#[derive(Debug, Clone, Copy)]
pub struct LiveUnavailableNvmlProvider {
    failure: NvmlSourceFailureV1,
}

impl LiveUnavailableNvmlProvider {
    pub const fn new(failure: NvmlSourceFailureV1) -> Self {
        Self { failure }
    }
}

impl NvmlProvider for LiveUnavailableNvmlProvider {
    fn observe(&self) -> NvmlObservationV1 {
        NvmlObservationV1 {
            source_attempted: true,
            source_failure: Some(self.failure),
            ..NvmlObservationV1::default()
        }
    }
}

impl NvmlProvider for NativeNvmlProvider {
    fn observe(&self) -> NvmlObservationV1 {
        let source = match authenticate_runtime_source() {
            Ok(source) => source,
            Err(failure) => return LiveUnavailableNvmlProvider::new(failure).observe(),
        };
        let mut observation = NvmlObservationV1 {
            source_attempted: true,
            source_identity: Some(source.identity.to_owned()),
            source_sha256: Some(source.sha256),
            ..NvmlObservationV1::default()
        };
        if let Err(failure) = verify_compiled_abi() {
            observation.source_failure = Some(failure);
            return observation;
        }
        observation.abi_verified = true;
        let mut api = match DynamicNvmlApi::load() {
            Ok(api) => api,
            Err(failure) => {
                observation.runtime.failure = Some(failure);
                return observation;
            }
        };
        observation.runtime = exercise_runtime(&mut api);
        observation
    }
}

pub fn observe_live_nvml() -> NvmlObservationV1 {
    NativeNvmlProvider.observe()
}

pub fn evaluate_nvml(
    observation: &NvmlObservationV1,
    kernel_driver_version: Option<&str>,
) -> NvmlEvidenceV1 {
    let mut reasons = Vec::new();
    if !observation.source_attempted {
        reasons.push(nvml_reason(
            "NVML_SOURCE_UNAVAILABLE",
            "Provide an operator-controlled official nvml.h source root before NVML admission.",
        ));
    } else if let Some(failure) = observation.source_failure {
        reasons.push(source_failure_reason(failure));
    } else if !observation.abi_verified {
        reasons.push(source_failure_reason(NvmlSourceFailureV1::AbiMismatch));
    }

    if let Some(failure) = observation.runtime.failure {
        reasons.push(runtime_failure_reason(failure));
    }
    if observation.runtime.initialized
        && (!observation.runtime.shutdown_attempted || !observation.runtime.shutdown_succeeded)
        && observation.runtime.failure != Some(NvmlRuntimeFailureV1::Shutdown)
    {
        reasons.push(runtime_failure_reason(NvmlRuntimeFailureV1::Shutdown));
    }

    let userspace = observation.runtime.userspace_driver_version.as_deref();
    if observation.source_attempted
        && observation.source_failure.is_none()
        && observation.abi_verified
        && observation.runtime.failure.is_none()
        && observation.runtime.shutdown_succeeded
    {
        match (kernel_driver_version, userspace) {
            (Some(kernel), Some(user)) if kernel != user => reasons.push(nvml_reason(
                "NVIDIA_VERSION_MISMATCH",
                "Boot a kernel and NVIDIA userspace from the same exact driver release, then rerun the doctor.",
            )),
            (None, _) => reasons.push(nvml_reason(
                "NVIDIA_KERNEL_VERSION_UNAVAILABLE",
                "Load the matching NVIDIA kernel module before rerunning the doctor.",
            )),
            (_, None) => reasons.push(runtime_failure_reason(
                NvmlRuntimeFailureV1::DriverVersion,
            )),
            _ => {}
        }
    }

    let status = if !observation.source_attempted {
        NvmlEvidenceStatusV1::Unproven
    } else if reasons.is_empty() {
        NvmlEvidenceStatusV1::Pass
    } else {
        NvmlEvidenceStatusV1::Fail
    };
    NvmlEvidenceV1 {
        status,
        source_identity: observation.source_identity.clone(),
        source_sha256: observation.source_sha256,
        abi_verified: observation.abi_verified,
        runtime_loaded: observation.runtime.loaded,
        initialized: observation.runtime.initialized,
        kernel_driver_version: kernel_driver_version.map(str::to_owned),
        userspace_driver_version: observation.runtime.userspace_driver_version.clone(),
        device_count: u32::try_from(observation.runtime.devices.len()).unwrap_or(u32::MAX),
        devices: observation.runtime.devices.clone(),
        shutdown_attempted: observation.runtime.shutdown_attempted,
        shutdown_succeeded: observation.runtime.shutdown_succeeded,
        reasons,
    }
}

fn source_failure_reason(failure: NvmlSourceFailureV1) -> NvmlReasonV1 {
    match failure {
        NvmlSourceFailureV1::Unavailable => nvml_reason(
            "NVML_SOURCE_UNAVAILABLE",
            "Provide the confirmed operator-controlled official nvml.h source root, then rebuild and rerun.",
        ),
        NvmlSourceFailureV1::Ambiguous => nvml_reason(
            "NVML_SOURCE_AMBIGUOUS",
            "Use one absolute readable official nvml.h source root, then rebuild and rerun.",
        ),
        NvmlSourceFailureV1::DigestMismatch => nvml_reason(
            "NVML_SOURCE_DIGEST_MISMATCH",
            "Rebuild and run against the same confirmed official nvml.h source file.",
        ),
        NvmlSourceFailureV1::AbiMismatch => nvml_reason(
            "NVML_SOURCE_ABI_MISMATCH",
            "Use the authenticated NVML API 13 header whose declarations match this provider.",
        ),
    }
}

fn runtime_failure_reason(failure: NvmlRuntimeFailureV1) -> NvmlReasonV1 {
    match failure {
        NvmlRuntimeFailureV1::LibraryLoad => nvml_reason(
            "NVML_LIBRARY_UNAVAILABLE",
            "Install the matching NVIDIA userspace runtime exposing libnvidia-ml.so.1, then rerun.",
        ),
        NvmlRuntimeFailureV1::SymbolMissing => nvml_reason(
            "NVML_SYMBOL_MISSING",
            "Install an NVIDIA userspace runtime with the complete verified NVML API, then rerun.",
        ),
        NvmlRuntimeFailureV1::Initialize => nvml_reason(
            "NVML_INITIALIZATION_FAILED",
            "Repair the NVIDIA kernel/userspace installation and permissions, then rerun.",
        ),
        NvmlRuntimeFailureV1::DriverVersion => nvml_reason(
            "NVML_DRIVER_VERSION_QUERY_FAILED",
            "Repair the NVIDIA userspace driver version query, then rerun.",
        ),
        NvmlRuntimeFailureV1::DeviceCount => nvml_reason(
            "NVML_DEVICE_COUNT_FAILED",
            "Repair NVIDIA device enumeration, then rerun.",
        ),
        NvmlRuntimeFailureV1::ZeroDevices => nvml_reason(
            "NVML_NO_DEVICES",
            "Expose at least one supported NVIDIA GPU to NVML, then rerun.",
        ),
        NvmlRuntimeFailureV1::DeviceHandle => nvml_reason(
            "NVML_DEVICE_HANDLE_FAILED",
            "Repair access to every enumerated NVIDIA device, then rerun.",
        ),
        NvmlRuntimeFailureV1::DeviceUuid => nvml_reason(
            "NVML_DEVICE_UUID_FAILED",
            "Repair stable NVIDIA UUID queries for every device, then rerun.",
        ),
        NvmlRuntimeFailureV1::DevicePci => nvml_reason(
            "NVML_DEVICE_PCI_FAILED",
            "Repair canonical NVIDIA PCI identity queries for every device, then rerun.",
        ),
        NvmlRuntimeFailureV1::DeviceIdentity => nvml_reason(
            "NVML_DEVICE_IDENTITY_INVALID",
            "Resolve malformed or duplicate NVIDIA UUID/PCI identities, then rerun.",
        ),
        NvmlRuntimeFailureV1::Shutdown => nvml_reason(
            "NVML_SHUTDOWN_FAILED",
            "Repair NVML cleanup so every initialized probe shuts down exactly once.",
        ),
    }
}

fn nvml_reason(code: &str, remediation: &str) -> NvmlReasonV1 {
    NvmlReasonV1 {
        code: code.to_owned(),
        remediation: remediation.to_owned(),
    }
}

fn allowed_nvml_reasons() -> Vec<NvmlReasonV1> {
    let mut reasons = vec![
        nvml_reason(
            "NVML_SOURCE_UNAVAILABLE",
            "Provide an operator-controlled official nvml.h source root before NVML admission.",
        ),
        nvml_reason(
            "NVIDIA_VERSION_MISMATCH",
            "Boot a kernel and NVIDIA userspace from the same exact driver release, then rerun the doctor.",
        ),
        nvml_reason(
            "NVIDIA_KERNEL_VERSION_UNAVAILABLE",
            "Load the matching NVIDIA kernel module before rerunning the doctor.",
        ),
    ];
    reasons.extend(
        [
            NvmlSourceFailureV1::Unavailable,
            NvmlSourceFailureV1::Ambiguous,
            NvmlSourceFailureV1::DigestMismatch,
            NvmlSourceFailureV1::AbiMismatch,
        ]
        .into_iter()
        .map(source_failure_reason),
    );
    reasons.extend(
        [
            NvmlRuntimeFailureV1::LibraryLoad,
            NvmlRuntimeFailureV1::SymbolMissing,
            NvmlRuntimeFailureV1::Initialize,
            NvmlRuntimeFailureV1::DriverVersion,
            NvmlRuntimeFailureV1::DeviceCount,
            NvmlRuntimeFailureV1::ZeroDevices,
            NvmlRuntimeFailureV1::DeviceHandle,
            NvmlRuntimeFailureV1::DeviceUuid,
            NvmlRuntimeFailureV1::DevicePci,
            NvmlRuntimeFailureV1::DeviceIdentity,
            NvmlRuntimeFailureV1::Shutdown,
        ]
        .into_iter()
        .map(runtime_failure_reason),
    );
    reasons
}

#[derive(Debug, Clone, Copy)]
struct CompiledSourceMetadata {
    identity: &'static str,
    sha256: Sha256DigestV1,
}

fn compiled_source_metadata() -> Option<CompiledSourceMetadata> {
    #[cfg(replay_nvml_source)]
    {
        let identity = env!("REPLAY_NVML_SOURCE_IDENTITY");
        let sha256 = Sha256DigestV1::from_str(env!("REPLAY_NVML_SOURCE_SHA256")).ok()?;
        Some(CompiledSourceMetadata { identity, sha256 })
    }
    #[cfg(not(replay_nvml_source))]
    {
        None
    }
}

fn authenticate_runtime_source() -> Result<CompiledSourceMetadata, NvmlSourceFailureV1> {
    let compiled = compiled_source_metadata().ok_or(NvmlSourceFailureV1::Unavailable)?;
    let root = std::env::var_os(NVML_SOURCE_ROOT_ENV)
        .map(PathBuf::from)
        .ok_or(NvmlSourceFailureV1::Unavailable)?;
    if !root.is_absolute() {
        return Err(NvmlSourceFailureV1::Ambiguous);
    }
    let header = root.join(NVML_HEADER_NAME);
    let metadata = std::fs::metadata(&header).map_err(|_| NvmlSourceFailureV1::Unavailable)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_SOURCE_BYTES {
        return Err(NvmlSourceFailureV1::Ambiguous);
    }
    let observed = sha256_file(&header).map_err(|_| NvmlSourceFailureV1::Unavailable)?;
    if observed != compiled.sha256 {
        return Err(NvmlSourceFailureV1::DigestMismatch);
    }
    Ok(compiled)
}

#[cfg(replay_nvml_source)]
#[link(name = "replay_nvml_abi_oracle", kind = "static")]
unsafe extern "C" {
    fn replay_nvml_sizeof_return() -> usize;
    fn replay_nvml_alignof_return() -> usize;
    fn replay_nvml_sizeof_device() -> usize;
    fn replay_nvml_alignof_device() -> usize;
    fn replay_nvml_sizeof_pci_info() -> usize;
    fn replay_nvml_alignof_pci_info() -> usize;
    fn replay_nvml_offset_pci_bus_id_legacy() -> usize;
    fn replay_nvml_offset_pci_domain() -> usize;
    fn replay_nvml_offset_pci_bus() -> usize;
    fn replay_nvml_offset_pci_device() -> usize;
    fn replay_nvml_offset_pci_device_id() -> usize;
    fn replay_nvml_offset_pci_subsystem_id() -> usize;
    fn replay_nvml_offset_pci_bus_id() -> usize;
    fn replay_nvml_api_version() -> c_uint;
    fn replay_nvml_status_success() -> c_uint;
    fn replay_nvml_status_version_mismatch() -> c_uint;
    fn replay_nvml_pci_legacy_buffer_size() -> c_uint;
    fn replay_nvml_pci_buffer_size() -> c_uint;
    fn replay_nvml_uuid_buffer_size() -> c_uint;
    fn replay_nvml_driver_version_buffer_size() -> c_uint;
    fn replay_nvml_signature_mask() -> c_uint;
}

fn verify_compiled_abi() -> Result<(), NvmlSourceFailureV1> {
    #[cfg(replay_nvml_source)]
    {
        let matches = unsafe {
            // SAFETY: These project-owned, zero-argument oracle functions are linked from
            // native/nvml_abi_oracle.c and return only compile-time values from nvml.h.
            replay_nvml_sizeof_return() == std::mem::size_of::<NvmlReturn>()
                && replay_nvml_alignof_return() == std::mem::align_of::<NvmlReturn>()
                && replay_nvml_sizeof_device() == std::mem::size_of::<NvmlDevice>()
                && replay_nvml_alignof_device() == std::mem::align_of::<NvmlDevice>()
                && replay_nvml_sizeof_pci_info() == std::mem::size_of::<NvmlPciInfo>()
                && replay_nvml_alignof_pci_info() == std::mem::align_of::<NvmlPciInfo>()
                && replay_nvml_offset_pci_bus_id_legacy()
                    == std::mem::offset_of!(NvmlPciInfo, bus_id_legacy)
                && replay_nvml_offset_pci_domain() == std::mem::offset_of!(NvmlPciInfo, domain)
                && replay_nvml_offset_pci_bus() == std::mem::offset_of!(NvmlPciInfo, bus)
                && replay_nvml_offset_pci_device() == std::mem::offset_of!(NvmlPciInfo, device)
                && replay_nvml_offset_pci_device_id()
                    == std::mem::offset_of!(NvmlPciInfo, pci_device_id)
                && replay_nvml_offset_pci_subsystem_id()
                    == std::mem::offset_of!(NvmlPciInfo, pci_subsystem_id)
                && replay_nvml_offset_pci_bus_id() == std::mem::offset_of!(NvmlPciInfo, bus_id)
                && replay_nvml_api_version() as usize == NVML_API_VERSION
                && replay_nvml_status_success() == NVML_SUCCESS
                && replay_nvml_status_version_mismatch() == 18
                && replay_nvml_pci_legacy_buffer_size() as usize == NVML_PCI_LEGACY_BUFFER_SIZE
                && replay_nvml_pci_buffer_size() as usize == NVML_PCI_BUFFER_SIZE
                && replay_nvml_uuid_buffer_size() as usize == NVML_UUID_BUFFER_SIZE
                && replay_nvml_driver_version_buffer_size() as usize
                    == NVML_DRIVER_VERSION_BUFFER_SIZE
                && replay_nvml_signature_mask() as usize == NVML_SIGNATURE_MASK
                && std::mem::size_of::<NvmlInitFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlShutdownFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlDriverVersionFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlDeviceCountFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlDeviceHandleFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlDeviceUuidFn>() == std::mem::size_of::<NvmlDevice>()
                && std::mem::size_of::<NvmlDevicePciFn>() == std::mem::size_of::<NvmlDevice>()
        };
        matches
            .then_some(())
            .ok_or(NvmlSourceFailureV1::AbiMismatch)
    }
    #[cfg(not(replay_nvml_source))]
    {
        Err(NvmlSourceFailureV1::Unavailable)
    }
}

trait NvmlApi {
    type Device: Copy;

    fn initialize(&mut self) -> Result<(), NvmlRuntimeFailureV1>;
    fn driver_version(&mut self) -> Result<String, NvmlRuntimeFailureV1>;
    fn device_count(&mut self) -> Result<u32, NvmlRuntimeFailureV1>;
    fn device_handle(&mut self, index: u32) -> Result<Self::Device, NvmlRuntimeFailureV1>;
    fn device_uuid(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1>;
    fn device_pci_bdf(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1>;
    fn shutdown(&mut self) -> Result<(), NvmlRuntimeFailureV1>;
}

struct DynamicNvmlApi {
    _library: Library,
    initialize: NvmlInitFn,
    shutdown: NvmlShutdownFn,
    driver_version: NvmlDriverVersionFn,
    device_count: NvmlDeviceCountFn,
    device_handle: NvmlDeviceHandleFn,
    device_uuid: NvmlDeviceUuidFn,
    device_pci: NvmlDevicePciFn,
}

impl DynamicNvmlApi {
    fn load() -> Result<Self, NvmlRuntimeFailureV1> {
        let library = unsafe {
            // SAFETY: The worker has a cleared loader environment and opens only the
            // fixed NVIDIA SONAME. No caller-controlled filename reaches the loader.
            Library::new(NVML_LIBRARY_NAME)
        }
        .map_err(|_| NvmlRuntimeFailureV1::LibraryLoad)?;
        let initialize = load_symbol(&library, b"nvmlInit_v2\0")?;
        let shutdown = load_symbol(&library, b"nvmlShutdown\0")?;
        let driver_version = load_symbol(&library, b"nvmlSystemGetDriverVersion\0")?;
        let device_count = load_symbol(&library, b"nvmlDeviceGetCount_v2\0")?;
        let device_handle = load_symbol(&library, b"nvmlDeviceGetHandleByIndex_v2\0")?;
        let device_uuid = load_symbol(&library, b"nvmlDeviceGetUUID\0")?;
        let device_pci = load_symbol(&library, b"nvmlDeviceGetPciInfo_v3\0")?;
        Ok(Self {
            _library: library,
            initialize,
            shutdown,
            driver_version,
            device_count,
            device_handle,
            device_uuid,
            device_pci,
        })
    }
}

fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, NvmlRuntimeFailureV1> {
    let symbol = unsafe {
        // SAFETY: Every requested symbol and its exact function-pointer type is listed
        // in NVML_SOURCE_MAP_V1 and compile-checked by native/nvml_abi_oracle.c.
        library.get::<T>(name)
    }
    .map_err(|_| NvmlRuntimeFailureV1::SymbolMissing)?;
    Ok(*symbol)
}

impl NvmlApi for DynamicNvmlApi {
    type Device = NvmlDevice;

    fn initialize(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
        let status = unsafe {
            // SAFETY: The source-gated ABI oracle verified this zero-argument symbol.
            (self.initialize)()
        };
        status_result(status, NvmlRuntimeFailureV1::Initialize)
    }

    fn driver_version(&mut self) -> Result<String, NvmlRuntimeFailureV1> {
        let mut buffer = [0; NVML_DRIVER_VERSION_BUFFER_SIZE];
        let status = unsafe {
            // SAFETY: The buffer is writable for the exact oracle-verified size.
            (self.driver_version)(
                buffer.as_mut_ptr(),
                NVML_DRIVER_VERSION_BUFFER_SIZE as c_uint,
            )
        };
        status_result(status, NvmlRuntimeFailureV1::DriverVersion)?;
        let value = c_buffer_to_string(&buffer)
            .filter(|version| valid_version(version))
            .ok_or(NvmlRuntimeFailureV1::DriverVersion)?;
        Ok(value)
    }

    fn device_count(&mut self) -> Result<u32, NvmlRuntimeFailureV1> {
        let mut count = 0;
        let status = unsafe {
            // SAFETY: count is a valid writable c_uint for the verified symbol.
            (self.device_count)(&mut count)
        };
        status_result(status, NvmlRuntimeFailureV1::DeviceCount)?;
        Ok(count)
    }

    fn device_handle(&mut self, index: u32) -> Result<Self::Device, NvmlRuntimeFailureV1> {
        let mut device = std::ptr::null_mut();
        let status = unsafe {
            // SAFETY: device is a writable opaque-handle slot and index was enumerated.
            (self.device_handle)(index, &mut device)
        };
        status_result(status, NvmlRuntimeFailureV1::DeviceHandle)?;
        (!device.is_null())
            .then_some(device)
            .ok_or(NvmlRuntimeFailureV1::DeviceHandle)
    }

    fn device_uuid(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1> {
        let mut buffer = [0; NVML_UUID_BUFFER_SIZE];
        let status = unsafe {
            // SAFETY: device came from NVML and the output buffer has oracle-verified size.
            (self.device_uuid)(device, buffer.as_mut_ptr(), NVML_UUID_BUFFER_SIZE as c_uint)
        };
        status_result(status, NvmlRuntimeFailureV1::DeviceUuid)?;
        c_buffer_to_string(&buffer)
            .filter(|uuid| valid_uuid(uuid))
            .ok_or(NvmlRuntimeFailureV1::DeviceUuid)
    }

    fn device_pci_bdf(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1> {
        let mut pci = NvmlPciInfo::default();
        let status = unsafe {
            // SAFETY: device came from NVML and NvmlPciInfo passed every oracle layout check.
            (self.device_pci)(device, &mut pci)
        };
        status_result(status, NvmlRuntimeFailureV1::DevicePci)?;
        if pci.bus > 0xff || pci.device > 0x1f {
            return Err(NvmlRuntimeFailureV1::DevicePci);
        }
        let canonical = format!("{:08x}:{:02x}:{:02x}.0", pci.domain, pci.bus, pci.device);
        let reported = c_buffer_to_string(&pci.bus_id).ok_or(NvmlRuntimeFailureV1::DevicePci)?;
        if !reported.eq_ignore_ascii_case(&canonical) || !valid_pci_bdf(&canonical) {
            return Err(NvmlRuntimeFailureV1::DevicePci);
        }
        Ok(canonical)
    }

    fn shutdown(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
        let status = unsafe {
            // SAFETY: Called exactly once after a successful initialization.
            (self.shutdown)()
        };
        status_result(status, NvmlRuntimeFailureV1::Shutdown)
    }
}

fn status_result(
    status: NvmlReturn,
    failure: NvmlRuntimeFailureV1,
) -> Result<(), NvmlRuntimeFailureV1> {
    if status == NVML_SUCCESS {
        Ok(())
    } else {
        Err(failure)
    }
}

fn exercise_runtime<A: NvmlApi>(api: &mut A) -> NvmlRuntimeObservationV1 {
    let mut observation = NvmlRuntimeObservationV1 {
        loaded: true,
        ..NvmlRuntimeObservationV1::default()
    };
    if let Err(failure) = api.initialize() {
        observation.failure = Some(failure);
        return observation;
    }
    observation.initialized = true;

    let body = collect_runtime_identity(api, &mut observation);
    observation.shutdown_attempted = true;
    let shutdown = api.shutdown();
    observation.shutdown_succeeded = shutdown.is_ok();
    observation.failure = body.err().or_else(|| shutdown.err());
    observation
}

fn collect_runtime_identity<A: NvmlApi>(
    api: &mut A,
    observation: &mut NvmlRuntimeObservationV1,
) -> Result<(), NvmlRuntimeFailureV1> {
    observation.userspace_driver_version = Some(api.driver_version()?);
    let count = api.device_count()?;
    if count == 0 {
        return Err(NvmlRuntimeFailureV1::ZeroDevices);
    }
    if count > MAX_NVML_DEVICES {
        return Err(NvmlRuntimeFailureV1::DeviceCount);
    }
    let mut uuids = HashSet::new();
    let mut pci_ids = HashSet::new();
    for index in 0..count {
        let handle = api.device_handle(index)?;
        let uuid = api.device_uuid(handle)?;
        let pci_bdf = api.device_pci_bdf(handle)?;
        if !valid_uuid(&uuid)
            || !valid_pci_bdf(&pci_bdf)
            || !uuids.insert(uuid.clone())
            || !pci_ids.insert(pci_bdf.clone())
        {
            return Err(NvmlRuntimeFailureV1::DeviceIdentity);
        }
        observation
            .devices
            .push(NvmlDeviceObservationV1 { uuid, pci_bdf });
    }
    Ok(())
}

fn c_buffer_to_string(buffer: &[c_char]) -> Option<String> {
    let bytes = unsafe {
        // SAFETY: c_char has byte alignment and buffer is valid for its full length.
        std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), buffer.len())
    };
    let value = CStr::from_bytes_until_nul(bytes).ok()?.to_bytes();
    if value.is_empty() || !value.iter().all(|byte| byte.is_ascii_graphic()) {
        return None;
    }
    std::str::from_utf8(value).ok().map(str::to_owned)
}

fn valid_version(value: &str) -> bool {
    value.len() < NVML_DRIVER_VERSION_BUFFER_SIZE
        && value.split('.').count() >= 3
        && value
            .split('.')
            .all(|component| !component.is_empty() && component.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_uuid(value: &str) -> bool {
    (value.starts_with("GPU-") || value.starts_with("MIG-"))
        && value.len() < NVML_UUID_BUFFER_SIZE
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_pci_bdf(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 16
        && bytes[8] == b':'
        && bytes[11] == b':'
        && bytes[14] == b'.'
        && bytes[15].is_ascii_digit()
        && bytes[15] <= b'7'
        && bytes[..8].iter().all(|byte| lower_hex(*byte))
        && bytes[9..11].iter().all(|byte| lower_hex(*byte))
        && bytes[12..14].iter().all(|byte| lower_hex(*byte))
}

fn lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct ScriptedApi {
        calls: Vec<String>,
        failure: Option<NvmlRuntimeFailureV1>,
        devices: Vec<NvmlDeviceObservationV1>,
    }

    impl ScriptedApi {
        fn passing() -> Self {
            Self {
                calls: Vec::new(),
                failure: None,
                devices: vec![NvmlDeviceObservationV1 {
                    uuid: "GPU-00000000-1111-2222-3333-444444444444".to_owned(),
                    pci_bdf: "00000000:01:00.0".to_owned(),
                }],
            }
        }

        fn failing(failure: NvmlRuntimeFailureV1) -> Self {
            Self {
                calls: Vec::new(),
                failure: Some(failure),
                devices: Self::passing().devices,
            }
        }

        fn should_fail(&self, failure: NvmlRuntimeFailureV1) -> bool {
            self.failure == Some(failure)
        }
    }

    impl NvmlApi for ScriptedApi {
        type Device = usize;

        fn initialize(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
            self.calls.push("initialize".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::Initialize) {
                Err(NvmlRuntimeFailureV1::Initialize)
            } else {
                Ok(())
            }
        }

        fn driver_version(&mut self) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push("driver-version".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::DriverVersion) {
                Err(NvmlRuntimeFailureV1::DriverVersion)
            } else {
                Ok("610.43.03".to_owned())
            }
        }

        fn device_count(&mut self) -> Result<u32, NvmlRuntimeFailureV1> {
            self.calls.push("device-count".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::DeviceCount) {
                Err(NvmlRuntimeFailureV1::DeviceCount)
            } else if self.should_fail(NvmlRuntimeFailureV1::ZeroDevices) {
                Ok(0)
            } else {
                Ok(u32::try_from(self.devices.len()).expect("fixture device count"))
            }
        }

        fn device_handle(&mut self, index: u32) -> Result<Self::Device, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-handle:{index}"));
            if self.should_fail(NvmlRuntimeFailureV1::DeviceHandle) {
                Err(NvmlRuntimeFailureV1::DeviceHandle)
            } else {
                Ok(usize::try_from(index).expect("fixture device index"))
            }
        }

        fn device_uuid(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-uuid:{device}"));
            if self.should_fail(NvmlRuntimeFailureV1::DeviceUuid) {
                Err(NvmlRuntimeFailureV1::DeviceUuid)
            } else {
                Ok(self.devices[device].uuid.clone())
            }
        }

        fn device_pci_bdf(&mut self, device: Self::Device) -> Result<String, NvmlRuntimeFailureV1> {
            self.calls.push(format!("device-pci:{device}"));
            if self.should_fail(NvmlRuntimeFailureV1::DevicePci) {
                Err(NvmlRuntimeFailureV1::DevicePci)
            } else {
                Ok(self.devices[device].pci_bdf.clone())
            }
        }

        fn shutdown(&mut self) -> Result<(), NvmlRuntimeFailureV1> {
            self.calls.push("shutdown".to_owned());
            if self.should_fail(NvmlRuntimeFailureV1::Shutdown) {
                Err(NvmlRuntimeFailureV1::Shutdown)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn nvml_source_abi_authorized_header_matches_complete_rust_contract() {
        let source = compiled_source_metadata().expect("authorized source must be compiled");
        assert_eq!(source.identity, "nvidia-nvml-api-13");
        assert_eq!(
            source.sha256.to_string(),
            "31a26e3ce6f0b98a76cea38a3cf28aa112a20cfb37e9057786c768712d6a487f"
        );
        verify_compiled_abi().expect("C oracle and Rust declarations must agree");
        assert_eq!(NVML_SOURCE_MAP_V1.len(), 7);
        assert!(
            NVML_SOURCE_MAP_V1
                .iter()
                .all(|entry| !entry.rust_declaration.is_empty()
                    && !entry.official_declaration.is_empty()
                    && !entry.oracle_check.is_empty())
        );
    }

    #[test]
    fn nvml_source_abi_failures_are_typed_and_never_expose_source_paths() {
        for (failure, reason) in [
            (NvmlSourceFailureV1::Unavailable, "NVML_SOURCE_UNAVAILABLE"),
            (NvmlSourceFailureV1::Ambiguous, "NVML_SOURCE_AMBIGUOUS"),
            (
                NvmlSourceFailureV1::DigestMismatch,
                "NVML_SOURCE_DIGEST_MISMATCH",
            ),
            (NvmlSourceFailureV1::AbiMismatch, "NVML_SOURCE_ABI_MISMATCH"),
        ] {
            let observation = LiveUnavailableNvmlProvider::new(failure).observe();
            let evidence = evaluate_nvml(&observation, Some("610.43.02"));
            assert_eq!(evidence.status, NvmlEvidenceStatusV1::Fail);
            assert_eq!(evidence.reasons[0].code, reason);
            let encoded = serde_json::to_string(&evidence).expect("evidence must serialize");
            assert!(!encoded.contains("REPLAY_NVML_SDK_ROOT"));
            assert!(!encoded.contains("/opt/"));
            assert!(!encoded.contains("NVML_PRIVATE_NATIVE_ERROR"));
        }
    }

    #[test]
    fn nvml_source_abi_exact_versions_remain_strings_and_mismatch_fails() {
        let observation = NvmlObservationV1 {
            source_attempted: true,
            source_identity: Some("nvidia-nvml-api-13".to_owned()),
            source_sha256: compiled_source_metadata().map(|source| source.sha256),
            abi_verified: true,
            source_failure: None,
            runtime: NvmlRuntimeObservationV1 {
                loaded: true,
                initialized: true,
                userspace_driver_version: Some("610.43.03".to_owned()),
                devices: ScriptedApi::passing().devices,
                shutdown_attempted: true,
                shutdown_succeeded: true,
                failure: None,
            },
        };
        let evidence = evaluate_nvml(&observation, Some("610.43.02"));
        assert_eq!(evidence.status, NvmlEvidenceStatusV1::Fail);
        assert_eq!(evidence.kernel_driver_version.as_deref(), Some("610.43.02"));
        assert_eq!(
            evidence.userspace_driver_version.as_deref(),
            Some("610.43.03")
        );
        assert!(
            evidence
                .reasons
                .iter()
                .any(|reason| reason.code == "NVIDIA_VERSION_MISMATCH")
        );
    }

    #[test]
    fn nvml_runtime_complete_sequence_collects_identity_and_shutdown() {
        let mut api = ScriptedApi::passing();
        let runtime = exercise_runtime(&mut api);

        assert_eq!(runtime.failure, None);
        assert_eq!(
            runtime.userspace_driver_version.as_deref(),
            Some("610.43.03")
        );
        assert_eq!(runtime.devices, api.devices);
        assert!(runtime.shutdown_attempted);
        assert!(runtime.shutdown_succeeded);
        assert_eq!(
            api.calls,
            [
                "initialize",
                "driver-version",
                "device-count",
                "device-handle:0",
                "device-uuid:0",
                "device-pci:0",
                "shutdown",
            ]
        );
    }

    #[test]
    fn nvml_runtime_every_initialized_failure_shuts_down_exactly_once() {
        for failure in [
            NvmlRuntimeFailureV1::DriverVersion,
            NvmlRuntimeFailureV1::DeviceCount,
            NvmlRuntimeFailureV1::ZeroDevices,
            NvmlRuntimeFailureV1::DeviceHandle,
            NvmlRuntimeFailureV1::DeviceUuid,
            NvmlRuntimeFailureV1::DevicePci,
            NvmlRuntimeFailureV1::Shutdown,
        ] {
            let mut api = ScriptedApi::failing(failure);
            let runtime = exercise_runtime(&mut api);
            assert_eq!(runtime.failure, Some(failure), "failure {failure:?}");
            assert!(runtime.shutdown_attempted, "failure {failure:?}");
            assert_eq!(
                api.calls
                    .iter()
                    .filter(|call| call.as_str() == "shutdown")
                    .count(),
                1,
                "failure {failure:?}"
            );
        }
    }

    #[test]
    fn nvml_runtime_initialization_failure_never_calls_shutdown() {
        let mut api = ScriptedApi::failing(NvmlRuntimeFailureV1::Initialize);
        let runtime = exercise_runtime(&mut api);
        assert_eq!(runtime.failure, Some(NvmlRuntimeFailureV1::Initialize));
        assert!(!runtime.shutdown_attempted);
        assert_eq!(api.calls, ["initialize"]);
    }

    #[test]
    fn nvml_runtime_library_and_symbol_presence_never_claim_readiness() {
        for (failure, reason) in [
            (
                NvmlRuntimeFailureV1::LibraryLoad,
                "NVML_LIBRARY_UNAVAILABLE",
            ),
            (NvmlRuntimeFailureV1::SymbolMissing, "NVML_SYMBOL_MISSING"),
        ] {
            let observation = NvmlObservationV1 {
                source_attempted: true,
                source_identity: Some("nvidia-nvml-api-13".to_owned()),
                source_sha256: compiled_source_metadata().map(|source| source.sha256),
                abi_verified: true,
                source_failure: None,
                runtime: NvmlRuntimeObservationV1 {
                    failure: Some(failure),
                    ..NvmlRuntimeObservationV1::default()
                },
            };
            let evidence = evaluate_nvml(&observation, Some("610.43.02"));
            assert_eq!(evidence.status, NvmlEvidenceStatusV1::Fail);
            assert_eq!(evidence.reasons[0].code, reason);
            assert!(!evidence.initialized);
            assert!(!evidence.shutdown_attempted);
        }
    }
}
