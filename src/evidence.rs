use crate::digest::{Sha256DigestV1, sha256_bytes};
use crate::model::{
    CaptureAdmissionV1, CaptureFrameObservationV1, CapturePathEvidenceV1, CaptureSourceEvidenceV1,
    G0EvidenceEnvelopeV1, G0EvidenceProvenanceV1, G0ExtensionRecordV1, G0ExtensionStatusV1,
    HOST_FOUNDATION_EXTENSION_ID, MAX_G0_ENVELOPE_BYTES, NVENC_TUPLES_EXTENSION_ID,
    NVFBC_CAPTURE_EXTENSION_ID, NvencAdmissionV1, NvencTuplesEvidenceV1,
    SELECTED_OUTPUT_EXTENSION_ID, SelectedOutputV1, decode_g0_evidence,
};
use crate::output_mapping::{
    SelectedOutputDiscoveryV1, SelectedOutputFailureEvidenceV1, validate_selected_output_discovery,
    validate_selected_output_evidence, validate_selected_output_failure,
};
use rustix::fs::{Mode, OFlags};
use serde::Deserialize;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub trait EvidenceStore {
    fn persist_and_readback(
        &self,
        envelope: &G0EvidenceEnvelopeV1,
    ) -> Result<G0EvidenceEnvelopeV1, EvidenceError>;

    fn read_exact(&self, expected_run_id: &str) -> Result<G0EvidenceEnvelopeV1, EvidenceError>;

    fn read_current(&self) -> Result<G0EvidenceEnvelopeV1, EvidenceError>;
}

#[derive(Debug, Clone)]
pub struct JsonFileEvidenceStore {
    path: PathBuf,
}

impl JsonFileEvidenceStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
pub enum EvidenceError {
    InvalidDestination(&'static str),
    Io {
        operation: &'static str,
        source: io::Error,
    },
    Encode,
    Decode,
    RunIdMismatch,
    ExecutableDigestMismatch,
    EnvelopeDigestMismatch,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDestination(reason) => {
                write!(formatter, "invalid evidence destination: {reason}")
            }
            Self::Io { operation, .. } => write!(formatter, "evidence {operation} failed"),
            Self::Encode => formatter.write_str("evidence serialization failed"),
            Self::Decode => formatter.write_str("evidence readback failed strict V1 decoding"),
            Self::RunIdMismatch => {
                formatter.write_str("evidence readback returned a different run identity")
            }
            Self::ExecutableDigestMismatch => {
                formatter.write_str("evidence readback changed the executable identity")
            }
            Self::EnvelopeDigestMismatch => {
                formatter.write_str("evidence readback bytes differ from the committed run")
            }
        }
    }
}

impl std::error::Error for EvidenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl EvidenceStore for JsonFileEvidenceStore {
    fn persist_and_readback(
        &self,
        envelope: &G0EvidenceEnvelopeV1,
    ) -> Result<G0EvidenceEnvelopeV1, EvidenceError> {
        let bytes = serde_json::to_vec(envelope).map_err(|_| EvidenceError::Encode)?;
        decode_g0_evidence(&bytes).map_err(|_| EvidenceError::Encode)?;
        if !validate_persisted_envelope(envelope) {
            return Err(EvidenceError::Encode);
        }
        let expected_bytes_digest = sha256_bytes(&bytes);
        let expected_run_id = envelope.base.run_id.clone();
        let expected_executable_digest = envelope.base.executable_sha256;

        atomic_write(&self.path, &bytes)?;
        let (readback, actual_bytes_digest) = read_and_decode(&self.path)?;
        if actual_bytes_digest != expected_bytes_digest {
            return Err(EvidenceError::EnvelopeDigestMismatch);
        }
        verify_identity(&readback, &expected_run_id, expected_executable_digest)?;
        Ok(readback)
    }

    fn read_exact(&self, expected_run_id: &str) -> Result<G0EvidenceEnvelopeV1, EvidenceError> {
        let (readback, _) = read_and_decode(&self.path)?;
        if !validate_persisted_envelope(&readback) {
            return Err(EvidenceError::Decode);
        }
        if readback.base.run_id != expected_run_id {
            return Err(EvidenceError::RunIdMismatch);
        }
        Ok(readback)
    }

    fn read_current(&self) -> Result<G0EvidenceEnvelopeV1, EvidenceError> {
        let (readback, _) = read_and_decode(&self.path)?;
        if !validate_persisted_envelope(&readback) {
            return Err(EvidenceError::Decode);
        }
        Ok(readback)
    }
}

pub fn validate_selected_output_record(record: &G0ExtensionRecordV1) -> bool {
    if record.id != SELECTED_OUTPUT_EXTENSION_ID || record.version != 1 {
        return false;
    }
    match record.status {
        G0ExtensionStatusV1::Pass => serde_json::from_str::<SelectedOutputV1>(record.payload.get())
            .is_ok_and(|selected| validate_selected_output_evidence(&selected)),
        G0ExtensionStatusV1::Fail => {
            serde_json::from_str::<SelectedOutputFailureEvidenceV1>(record.payload.get())
                .is_ok_and(|failure| validate_selected_output_failure(&failure))
        }
        G0ExtensionStatusV1::Unproven => {
            serde_json::from_str::<SelectedOutputDiscoveryV1>(record.payload.get())
                .is_ok_and(|discovery| validate_selected_output_discovery(&discovery))
        }
    }
}

pub fn validate_nvfbc_capture_record(record: &G0ExtensionRecordV1) -> bool {
    if record.id != NVFBC_CAPTURE_EXTENSION_ID || record.version != 1 {
        return false;
    }
    if let Ok(evidence) = serde_json::from_str::<CapturePathEvidenceV1>(record.payload.get()) {
        if !validate_capture_path_identifiers(&evidence) {
            return false;
        }
        return match record.status {
            G0ExtensionStatusV1::Pass => {
                evidence.admission == CaptureAdmissionV1::Pass
                    && crate::native_nvfbc::validate_capture_path_evidence(&evidence)
            }
            G0ExtensionStatusV1::Fail => {
                evidence.admission == CaptureAdmissionV1::Rejected
                    && crate::native_nvfbc::validate_capture_path_evidence(&evidence)
            }
            G0ExtensionStatusV1::Unproven => {
                matches!(
                    evidence.admission,
                    CaptureAdmissionV1::Unproven | CaptureAdmissionV1::Rejected
                ) && crate::native_nvfbc::validate_capture_path_evidence(&evidence)
            }
        };
    }
    record.status == G0ExtensionStatusV1::Unproven
        && serde_json::from_str::<LegacyCaptureObservationV1>(record.payload.get())
            .is_ok_and(|legacy| legacy.is_valid())
}

/// Validate every known record plus the relationships that only exist at the
/// envelope boundary. Structural/digest validation remains the responsibility
/// of `decode_g0_evidence`; this function deliberately adds semantic authority.
pub fn validate_persisted_envelope(envelope: &G0EvidenceEnvelopeV1) -> bool {
    if legacy_foundation_envelope(envelope) {
        return true;
    }

    let Some(foundation) = unique_record(envelope, HOST_FOUNDATION_EXTENSION_ID) else {
        return false;
    };
    let Some(selected_record) = unique_record(envelope, SELECTED_OUTPUT_EXTENSION_ID) else {
        return false;
    };
    let Some(capture_record) = unique_record(envelope, NVFBC_CAPTURE_EXTENSION_ID) else {
        return false;
    };
    let Some(nvenc_record) = unique_record(envelope, NVENC_TUPLES_EXTENSION_ID) else {
        return false;
    };
    if !crate::local_xorg::validate_host_foundation_record(foundation)
        || !validate_selected_output_record(selected_record)
        || !validate_nvfbc_capture_record(capture_record)
        || !validate_nvenc_tuples_record(nvenc_record)
    {
        return false;
    }

    if capture_record.status != G0ExtensionStatusV1::Pass {
        return true;
    }
    if envelope.base.provenance != G0EvidenceProvenanceV1::Live
        || selected_record.status != G0ExtensionStatusV1::Pass
    {
        return false;
    }
    let Ok(selected) = serde_json::from_str::<SelectedOutputV1>(selected_record.payload.get())
    else {
        return false;
    };
    let Ok(capture) = serde_json::from_str::<CapturePathEvidenceV1>(capture_record.payload.get())
    else {
        return false;
    };
    host03_pass_binding_matches(&selected, &capture)
}

pub fn validate_nvenc_tuples_record(record: &G0ExtensionRecordV1) -> bool {
    if record.id != NVENC_TUPLES_EXTENSION_ID || record.version != 1 {
        return false;
    }

    if let Ok(evidence) = serde_json::from_str::<NvencTuplesEvidenceV1>(record.payload.get()) {
        let expected_status = match evidence.admission {
            NvencAdmissionV1::Pass => G0ExtensionStatusV1::Pass,
            NvencAdmissionV1::Rejected => G0ExtensionStatusV1::Fail,
            NvencAdmissionV1::Unproven => G0ExtensionStatusV1::Unproven,
        };
        return record.status == expected_status
            && crate::native_nvenc::validate_nvenc_tuples_evidence(&evidence);
    }

    record.status == G0ExtensionStatusV1::Unproven
        && serde_json::from_str::<NvencTuplesPlaceholderV1>(record.payload.get()).is_ok_and(
            |placeholder| {
                placeholder.schema == "replaydesktop.nvenc-tuples-observation.v1"
                    && placeholder.probe == "nvenc-tuples"
                    && placeholder.admission == "unproven"
                    && placeholder.worker_status == "observed"
                    && !placeholder.primitive_available
                    && placeholder.observation_class == "primitive-unavailable"
            },
        )
}

fn unique_record<'a>(
    envelope: &'a G0EvidenceEnvelopeV1,
    identifier: &str,
) -> Option<&'a G0ExtensionRecordV1> {
    let mut matching = envelope
        .extensions
        .iter()
        .filter(|record| record.id == identifier);
    let record = matching.next()?;
    matching.next().is_none().then_some(record)
}

fn legacy_foundation_envelope(envelope: &G0EvidenceEnvelopeV1) -> bool {
    if envelope.base.provenance != G0EvidenceProvenanceV1::Diagnostic {
        return false;
    }
    [
        HOST_FOUNDATION_EXTENSION_ID,
        SELECTED_OUTPUT_EXTENSION_ID,
        NVFBC_CAPTURE_EXTENSION_ID,
        NVENC_TUPLES_EXTENSION_ID,
    ]
    .into_iter()
    .all(|identifier| {
        unique_record(envelope, identifier).is_some_and(|record| {
            record.version == 1
                && record.status == G0ExtensionStatusV1::Unproven
                && serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
                    record.payload.get(),
                )
                .is_ok_and(|payload| payload.is_empty())
        })
    })
}

fn host03_pass_binding_matches(
    selected: &SelectedOutputV1,
    capture: &CapturePathEvidenceV1,
) -> bool {
    let (Some(binding), Some(lease)) = (capture.binding.as_ref(), capture.lease.as_ref()) else {
        return false;
    };
    binding.output_name == selected.output_name
        && binding.randr_output_xid == selected.randr_output_xid
        && binding.gpu_pci_bdf == selected.nvml_pci_bdf
        && binding.gpu_uuid == selected.nvml_uuid
        && binding.topology_token == selected.topology_token
        && lease.width_px == selected.width_px
        && lease.height_px == selected.height_px
}

pub(crate) fn validate_capture_source_identifiers(source: &CaptureSourceEvidenceV1) -> bool {
    source
        .nvfbc_runtime_library
        .as_deref()
        .is_none_or(valid_runtime_library_identifier)
        && source
            .cuda_runtime_library
            .as_deref()
            .is_none_or(valid_runtime_library_identifier)
}

pub(crate) fn validate_capture_frame_identifiers(frame: &CaptureFrameObservationV1) -> bool {
    valid_surface_identifier(&frame.source_surface)
        && valid_surface_identifier(&frame.lease_surface)
        && frame.edges.iter().all(|edge| {
            valid_surface_identifier(&edge.from_surface)
                && valid_surface_identifier(&edge.to_surface)
        })
}

fn validate_capture_path_identifiers(evidence: &CapturePathEvidenceV1) -> bool {
    validate_capture_source_identifiers(&evidence.source)
        && evidence.lease.as_ref().is_none_or(|lease| {
            valid_surface_identifier(&lease.source_surface)
                && valid_surface_identifier(&lease.lease_surface)
        })
        && evidence.copy_ledger.as_ref().is_none_or(|ledger| {
            ledger.edges.iter().all(|edge| {
                valid_surface_identifier(&edge.from_surface)
                    && valid_surface_identifier(&edge.to_surface)
            })
        })
}

fn valid_runtime_library_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains(['/', '\\', ':'])
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

fn valid_surface_identifier(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        || !value.contains(['-', '_'])
        || value.bytes().any(|byte| {
            !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_'))
        })
    {
        return false;
    }
    value.split(['-', '_']).all(|part| {
        !(part.is_empty()
            || matches!(
                part,
                "ptr"
                    | "pointer"
                    | "address"
                    | "addr"
                    | "raw"
                    | "frame"
                    | "framebuffer"
                    | "deviceptr"
                    | "cudeviceptr"
            )
            || (part.len() >= 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit())))
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NvencTuplesPlaceholderV1 {
    schema: String,
    probe: String,
    admission: String,
    worker_status: String,
    primitive_available: bool,
    observation_class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCaptureObservationV1 {
    schema: String,
    probe: String,
    admission: String,
    worker_status: String,
    primitive_available: Option<bool>,
    observation_class: Option<String>,
    reason: Option<String>,
}

impl LegacyCaptureObservationV1 {
    fn is_valid(&self) -> bool {
        if self.schema != "replaydesktop.nvfbc-capture-observation.v1"
            || self.probe != "nvfbc-capture"
            || self.admission != "unproven"
        {
            return false;
        }
        match self.worker_status.as_str() {
            "observed" => {
                self.primitive_available.is_some()
                    && self.reason.is_none()
                    && self.observation_class.as_deref()
                        == self.primitive_available.map(|available| {
                            if available {
                                "primitive-available"
                            } else {
                                "primitive-unavailable"
                            }
                        })
            }
            "rejected" => {
                self.reason
                    .as_deref()
                    .is_some_and(valid_legacy_worker_reason)
                    && self.primitive_available.is_none()
                    && self.observation_class.is_none()
            }
            _ => false,
        }
    }
}

fn valid_legacy_worker_reason(reason: &str) -> bool {
    matches!(
        reason,
        "worker-spawn"
            | "worker-timeout"
            | "worker-stdout-limit"
            | "worker-stderr-limit"
            | "worker-stderr"
            | "worker-abnormal-exit"
            | "worker-malformed-response"
            | "worker-protocol-mismatch"
            | "worker-secret-leak"
    )
}

fn verify_identity(
    envelope: &G0EvidenceEnvelopeV1,
    expected_run_id: &str,
    expected_executable_digest: Sha256DigestV1,
) -> Result<(), EvidenceError> {
    if envelope.base.run_id != expected_run_id {
        return Err(EvidenceError::RunIdMismatch);
    }
    if envelope.base.executable_sha256 != expected_executable_digest {
        return Err(EvidenceError::ExecutableDigestMismatch);
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), EvidenceError> {
    if bytes.len() > MAX_G0_ENVELOPE_BYTES {
        return Err(EvidenceError::Encode);
    }

    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(EvidenceError::InvalidDestination(
                "symbolic links are not accepted",
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(EvidenceError::InvalidDestination(
                "existing destination is not a regular file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(EvidenceError::Io {
                operation: "destination inspection",
                source,
            });
        }
    }

    let (parent, final_name) = split_destination(path)?;
    let directory = File::open(parent).map_err(|source| EvidenceError::Io {
        operation: "parent-directory open",
        source,
    })?;
    if !directory
        .metadata()
        .map_err(|source| EvidenceError::Io {
            operation: "parent-directory metadata",
            source,
        })?
        .is_dir()
    {
        return Err(EvidenceError::InvalidDestination(
            "parent is not a directory",
        ));
    }

    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_name = format!(".replay-host-doctor.tmp-{}-{sequence}", std::process::id());
    let temporary_fd = rustix::fs::openat(
        &directory,
        temporary_name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|error| EvidenceError::Io {
        operation: "temporary create",
        source: errno_to_io(error),
    })?;
    let mut temporary = File::from(temporary_fd);

    let write_result = (|| {
        temporary
            .write_all(bytes)
            .map_err(|source| EvidenceError::Io {
                operation: "temporary write",
                source,
            })?;
        temporary.flush().map_err(|source| EvidenceError::Io {
            operation: "temporary flush",
            source,
        })?;
        rustix::fs::fsync(&temporary).map_err(|error| EvidenceError::Io {
            operation: "temporary fsync",
            source: errno_to_io(error),
        })?;
        drop(temporary);

        rustix::fs::renameat(&directory, temporary_name.as_str(), &directory, final_name).map_err(
            |error| EvidenceError::Io {
                operation: "atomic rename",
                source: errno_to_io(error),
            },
        )?;
        rustix::fs::fsync(&directory).map_err(|error| EvidenceError::Io {
            operation: "parent-directory fsync",
            source: errno_to_io(error),
        })
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(parent.join(&temporary_name));
    }
    write_result
}

fn read_and_decode(path: &Path) -> Result<(G0EvidenceEnvelopeV1, Sha256DigestV1), EvidenceError> {
    let (parent, final_name) = split_destination(path)?;
    let directory = File::open(parent).map_err(|source| EvidenceError::Io {
        operation: "readback parent-directory open",
        source,
    })?;
    let fd = rustix::fs::openat(
        &directory,
        final_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| EvidenceError::Io {
        operation: "readback open",
        source: errno_to_io(error),
    })?;
    let file = File::from(fd);
    if !file
        .metadata()
        .map_err(|source| EvidenceError::Io {
            operation: "readback metadata",
            source,
        })?
        .is_file()
    {
        return Err(EvidenceError::InvalidDestination(
            "readback is not a regular file",
        ));
    }

    let mut bytes = Vec::new();
    file.take((MAX_G0_ENVELOPE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| EvidenceError::Io {
            operation: "readback read",
            source,
        })?;
    if bytes.len() > MAX_G0_ENVELOPE_BYTES {
        return Err(EvidenceError::Decode);
    }
    let digest = sha256_bytes(&bytes);
    let envelope = decode_g0_evidence(&bytes)
        .map_err(|_| EvidenceError::Decode)?
        .into_v1();
    Ok((envelope, digest))
}

fn split_destination(path: &Path) -> Result<(&Path, &OsStr), EvidenceError> {
    let final_name = path
        .file_name()
        .ok_or(EvidenceError::InvalidDestination("a file name is required"))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    Ok((parent, final_name))
}

fn errno_to_io(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{G0ExtensionStatusV1, SELECTED_OUTPUT_EXTENSION_ID};
    use crate::output_mapping::{collect_fixture_output_topology, prove_output_gpu_mapping};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "replay-host-doctor-evidence-{label}-{}-{timestamp}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn evidence_atomic_oversize_failure_retains_prior_file() {
        let directory = temp_path("retain");
        std::fs::create_dir(&directory).expect("test directory must be creatable");
        let path = directory.join("evidence.json");
        std::fs::write(&path, b"prior-complete-evidence").expect("sentinel must be writable");

        assert!(atomic_write(&path, &vec![b'x'; MAX_G0_ENVELOPE_BYTES + 1]).is_err());
        assert_eq!(
            std::fs::read(&path).expect("prior file must remain readable"),
            b"prior-complete-evidence"
        );
        assert_eq!(
            std::fs::read_dir(&directory)
                .expect("directory must be readable")
                .count(),
            1,
            "failed writes must not leave temporary siblings"
        );

        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }

    #[test]
    fn evidence_atomic_wrong_run_readback_fails() {
        let directory = temp_path("wrong-run");
        std::fs::create_dir(&directory).expect("test directory must be creatable");
        let path = directory.join("evidence.json");
        std::fs::write(
            &path,
            include_bytes!("../tests/fixtures/g0-envelope-v1-foundation.json"),
        )
        .expect("fixture must be writable");

        let store = JsonFileEvidenceStore::new(&path);
        assert!(matches!(
            store.read_exact("a-different-run"),
            Err(EvidenceError::RunIdMismatch)
        ));

        std::fs::remove_dir_all(&directory).expect("test directory must be removable");
    }

    #[test]
    fn evidence_atomic_missing_parent_is_not_created() {
        let directory = temp_path("missing-parent");
        let path = directory.join("nested").join("evidence.json");
        assert!(atomic_write(&path, b"{}").is_err());
        assert!(!directory.exists());
    }

    #[test]
    fn selected_output_extension_is_strict_and_cross_field_consistent() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/host02-output-topologies.json");
        let collected =
            collect_fixture_output_topology(&fixture, "nvcontrol-mst-dp-0-3", Some("DP-0.3"))
                .expect("fixture collection");
        let mut selected = prove_output_gpu_mapping(
            collected
                .topology
                .as_ref()
                .expect("selected fixture must carry topology"),
        )
        .expect("fixture relation must pass");
        let valid = crate::extension_record(
            SELECTED_OUTPUT_EXTENSION_ID,
            G0ExtensionStatusV1::Pass,
            serde_json::to_value(&selected).expect("selected output JSON"),
        )
        .expect("extension record");
        assert!(validate_selected_output_record(&valid));

        selected.nvml_pci_bdf = "00000000:02:00.0".to_owned();
        let inconsistent = crate::extension_record(
            SELECTED_OUTPUT_EXTENSION_ID,
            G0ExtensionStatusV1::Pass,
            serde_json::to_value(&selected).expect("selected output JSON"),
        )
        .expect("extension record");
        assert!(!validate_selected_output_record(&inconsistent));
    }

    #[test]
    fn nvfbc_extension_rejects_injected_verdict_with_matching_payload_digest() {
        let injected = crate::extension_record(
            NVFBC_CAPTURE_EXTENSION_ID,
            G0ExtensionStatusV1::Unproven,
            serde_json::json!({
                "schema": "replaydesktop.nvfbc-capture-observation.v1",
                "probe": "nvfbc-capture",
                "admission": "unproven",
                "worker_status": "observed",
                "primitive_available": false,
                "observation_class": "primitive-unavailable",
                "verdict": "pass"
            }),
        )
        .expect("extension record");
        assert!(!validate_nvfbc_capture_record(&injected));
    }

    #[test]
    fn host04_placeholder_is_exact_and_cannot_claim_a_terminal_result() {
        let exact = crate::extension_record(
            crate::model::NVENC_TUPLES_EXTENSION_ID,
            G0ExtensionStatusV1::Unproven,
            serde_json::json!({
                "schema": "replaydesktop.nvenc-tuples-observation.v1",
                "probe": "nvenc-tuples",
                "admission": "unproven",
                "worker_status": "observed",
                "primitive_available": false,
                "observation_class": "primitive-unavailable"
            }),
        )
        .expect("extension record");
        assert!(validate_nvenc_tuples_record(&exact));

        for (status, payload) in [
            (
                G0ExtensionStatusV1::Pass,
                serde_json::json!({
                    "schema": "replaydesktop.nvenc-tuples-observation.v1",
                    "probe": "nvenc-tuples",
                    "admission": "pass",
                    "worker_status": "observed",
                    "primitive_available": true,
                    "observation_class": "primitive-available"
                }),
            ),
            (
                G0ExtensionStatusV1::Fail,
                serde_json::json!({
                    "schema": "replaydesktop.nvenc-tuples-observation.v1",
                    "probe": "nvenc-tuples",
                    "admission": "rejected",
                    "worker_status": "rejected",
                    "reason": "injected"
                }),
            ),
            (
                G0ExtensionStatusV1::Unproven,
                serde_json::json!({
                    "schema": "replaydesktop.nvenc-tuples-observation.v1",
                    "probe": "nvenc-tuples",
                    "admission": "unproven",
                    "worker_status": "observed",
                    "primitive_available": false,
                    "observation_class": "primitive-unavailable",
                    "verdict": "pass"
                }),
            ),
        ] {
            let record =
                crate::extension_record(crate::model::NVENC_TUPLES_EXTENSION_ID, status, payload)
                    .expect("extension record");
            assert!(!validate_nvenc_tuples_record(&record));
        }
    }

    #[test]
    fn persistence_identifiers_reject_paths_soname_injection_and_pointer_like_surfaces() {
        for value in [
            "/usr/lib/libcuda.so.1",
            r"C:\driver\libcuda.so.1",
            "../libcuda.so.1",
            "prefix:libcuda.so.1",
            "libcuda.so.1\n",
            " libcuda.so.1",
        ] {
            assert!(!valid_runtime_library_identifier(value), "{value:?}");
        }
        for value in ["libcuda.so.1", "libnvidia-fbc.so.610.43.03"] {
            assert!(valid_runtime_library_identifier(value), "{value:?}");
        }

        for value in [
            "140737488355328",
            "7ffdeadbeef",
            "0x7ffdeadbeef",
            "device-pointer-001",
            "gpu-address-001",
            "raw-frame-001",
            "../surface",
            "surface/path",
        ] {
            assert!(!valid_surface_identifier(value), "{value:?}");
        }
        for value in [
            "surface-capture-001",
            "selected-scanout-bgra",
            "nvfbc-shared-cuda-nv12",
            "application-owned-nv12",
        ] {
            assert!(valid_surface_identifier(value), "{value:?}");
        }
    }

    #[test]
    fn host03_cross_binding_covers_name_xid_gpu_geometry_and_complete_topology_token() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/host02-output-topologies.json");
        let collected =
            collect_fixture_output_topology(&fixture, "nvcontrol-mst-dp-0-3", Some("DP-0.3"))
                .expect("fixture collection");
        let selected = prove_output_gpu_mapping(
            collected
                .topology
                .as_ref()
                .expect("selected fixture must carry topology"),
        )
        .expect("fixture relation must pass");
        let capture_fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/host03-nvfbc-capture.json"
        ))
        .expect("capture fixture must be JSON");
        let primitive: crate::model::CapturePrimitiveObservationV1 =
            serde_json::from_value(capture_fixture["base_capture"].clone())
                .expect("base capture must decode");
        let mut capture =
            crate::native_nvfbc::evaluate_capture_observation(primitive, Some(&selected));
        assert!(host03_pass_binding_matches(&selected, &capture));

        capture.binding.as_mut().expect("binding").output_name.hex = "00".to_owned();
        assert!(!host03_pass_binding_matches(&selected, &capture));
        capture.binding.as_mut().expect("binding").output_name = selected.output_name.clone();

        capture.binding.as_mut().expect("binding").randr_output_xid =
            crate::model::XrandrOutputXidV1::new(selected.randr_output_xid.get() + 1);
        assert!(!host03_pass_binding_matches(&selected, &capture));
        capture.binding.as_mut().expect("binding").randr_output_xid = selected.randr_output_xid;

        capture.binding.as_mut().expect("binding").gpu_pci_bdf = "00000000:02:00.0".to_owned();
        assert!(!host03_pass_binding_matches(&selected, &capture));
        capture.binding.as_mut().expect("binding").gpu_pci_bdf = selected.nvml_pci_bdf.clone();

        capture.binding.as_mut().expect("binding").gpu_uuid =
            "GPU-00000000-0000-0000-0000-000000000000".to_owned();
        assert!(!host03_pass_binding_matches(&selected, &capture));
        capture.binding.as_mut().expect("binding").gpu_uuid = selected.nvml_uuid.clone();

        capture
            .binding
            .as_mut()
            .expect("binding")
            .topology_token
            .randr_event_count += 1;
        assert!(!host03_pass_binding_matches(&selected, &capture));
        capture.binding.as_mut().expect("binding").topology_token = selected.topology_token.clone();

        capture.lease.as_mut().expect("lease").width_px -= 1;
        assert!(!host03_pass_binding_matches(&selected, &capture));
    }

    #[test]
    fn original_foundation_envelope_remains_an_explicit_legacy_compatibility_shape() {
        let envelope = decode_g0_evidence(include_bytes!(
            "../tests/fixtures/g0-envelope-v1-foundation.json"
        ))
        .expect("foundation fixture must decode")
        .into_v1();
        assert!(validate_persisted_envelope(&envelope));
    }
}
