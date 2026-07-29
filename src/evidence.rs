use crate::digest::{Sha256DigestV1, sha256_bytes};
use crate::model::{
    CaptureAdmissionV1, CapturePathEvidenceV1, G0EvidenceEnvelopeV1, G0ExtensionRecordV1,
    G0ExtensionStatusV1, MAX_G0_ENVELOPE_BYTES, NVFBC_CAPTURE_EXTENSION_ID,
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
        if readback.base.run_id != expected_run_id {
            return Err(EvidenceError::RunIdMismatch);
        }
        Ok(readback)
    }

    fn read_current(&self) -> Result<G0EvidenceEnvelopeV1, EvidenceError> {
        read_and_decode(&self.path).map(|(readback, _)| readback)
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
                crate::native_nvfbc::validate_capture_path_evidence(&evidence)
            }
        };
    }
    record.status == G0ExtensionStatusV1::Unproven
        && serde_json::from_str::<LegacyCaptureObservationV1>(record.payload.get())
            .is_ok_and(|legacy| legacy.is_valid())
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
}
