use crate::currentness::CurrentnessPolicy;
use crate::digest::{Sha256DigestV1, sha256_bytes, sha256_reader};
use crate::model::{
    G0EvidenceEnvelopeV1, G0EvidenceProvenanceV1, G0ExtensionStatusV1, G0GateStatusV1,
    MAX_G0_ENVELOPE_BYTES, decode_g0_evidence,
};
use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ARCHIVE_MANIFEST_SCHEMA_V1: &str = "replaydesktop.g0-pre-reboot-archive-manifest.v1";
pub const ARCHIVE_INDEX_SCHEMA_V1: &str = "replaydesktop.g0-pre-reboot-archive-index.v1";
pub const POST_REPAIR_ARCHIVE_MANIFEST_SCHEMA_V1: &str =
    "replaydesktop.g0-post-repair-archive-manifest.v1";
pub const POST_REPAIR_ARCHIVE_INDEX_SCHEMA_V1: &str =
    "replaydesktop.g0-post-repair-archive-index.v1";
pub const ARCHIVE_VERSION_V1: u32 = 1;

const ARCHIVED_BINARY_NAME: &str = "replay-host-doctor";
const ARCHIVED_EVIDENCE_NAME: &str = "g0-evidence.json";
const ARCHIVE_MANIFEST_NAME: &str = "manifest.json";
const ARCHIVE_INDEX_NAME: &str = "index.json";
const MAX_ARCHIVED_BINARY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_METADATA_BYTES: usize = 1024 * 1024;
const REGULAR_FILE_TYPE_BITS: u32 = 0o100_000;
const FILE_TYPE_MASK: u32 = 0o170_000;
const DIRECTORY_MODE: Mode = Mode::RUSR.union(Mode::WUSR).union(Mode::XUSR);
const IMMUTABLE_DIRECTORY_MODE: Mode = Mode::RUSR.union(Mode::XUSR);
const WRITABLE_FILE_MODE: Mode = Mode::RUSR.union(Mode::WUSR);
const IMMUTABLE_BINARY_MODE: Mode = Mode::RUSR.union(Mode::XUSR);
const IMMUTABLE_DATA_MODE: Mode = Mode::RUSR;

static ARCHIVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ArchiveEvidencePolicy {
    FailOnly,
    PassOrFail,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ArchiveContract {
    index_schema: &'static str,
    manifest_schema: &'static str,
    evidence_policy: ArchiveEvidencePolicy,
    validate_persisted_envelope: bool,
}

const PRE_REBOOT_ARCHIVE_CONTRACT: ArchiveContract = ArchiveContract {
    index_schema: ARCHIVE_INDEX_SCHEMA_V1,
    manifest_schema: ARCHIVE_MANIFEST_SCHEMA_V1,
    evidence_policy: ArchiveEvidencePolicy::FailOnly,
    validate_persisted_envelope: false,
};

const POST_REPAIR_ARCHIVE_CONTRACT: ArchiveContract = ArchiveContract {
    index_schema: POST_REPAIR_ARCHIVE_INDEX_SCHEMA_V1,
    manifest_schema: POST_REPAIR_ARCHIVE_MANIFEST_SCHEMA_V1,
    evidence_policy: ArchiveEvidencePolicy::PassOrFail,
    validate_persisted_envelope: true,
};

impl ArchiveContract {
    fn from_index_schema(schema: &str) -> Option<Self> {
        match schema {
            ARCHIVE_INDEX_SCHEMA_V1 => Some(PRE_REBOOT_ARCHIVE_CONTRACT),
            POST_REPAIR_ARCHIVE_INDEX_SCHEMA_V1 => Some(POST_REPAIR_ARCHIVE_CONTRACT),
            _ => None,
        }
    }

    fn accepts_status(self, status: G0GateStatusV1) -> bool {
        match self.evidence_policy {
            ArchiveEvidencePolicy::FailOnly => status == G0GateStatusV1::Fail,
            ArchiveEvidencePolicy::PassOrFail => {
                matches!(status, G0GateStatusV1::Pass | G0GateStatusV1::Fail)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ArchiveFileV1 {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: Sha256DigestV1,
    pub mode: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ArchiveSourceExecutableV1 {
    pub kind: String,
    pub file_type: String,
    pub mode: u32,
    pub device: u64,
    pub inode: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ArchiveExtensionV1 {
    pub id: String,
    pub version: u32,
    pub status: G0ExtensionStatusV1,
    pub payload_sha256: Sha256DigestV1,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct G0ArchiveManifestV1 {
    pub schema: String,
    pub version: u32,
    pub run_id: String,
    pub boot_id: String,
    pub session_id: String,
    pub argv: Vec<String>,
    pub wall_started_unix_ns: u64,
    pub wall_finished_unix_ns: u64,
    pub monotonic_started_ns: u64,
    pub monotonic_finished_ns: u64,
    pub provenance: G0EvidenceProvenanceV1,
    pub evidence_status: G0GateStatusV1,
    pub evidence_schema: String,
    pub evidence_version: u32,
    pub evidence_base_schema: String,
    pub source_executable: G0ArchiveSourceExecutableV1,
    pub archived_binary: G0ArchiveFileV1,
    pub evidence: G0ArchiveFileV1,
    pub extensions: Vec<G0ArchiveExtensionV1>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct G0ArchiveIndexV1 {
    schema: String,
    version: u32,
    manifest_path: String,
    manifest_sha256: Sha256DigestV1,
}

#[derive(Debug, Deserialize)]
struct G0ArchiveIndexDispatch {
    schema: String,
}

#[derive(Debug)]
pub struct ArchivedG0V1 {
    pub index_path: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest_sha256: Sha256DigestV1,
    pub manifest: G0ArchiveManifestV1,
}

#[derive(Debug)]
pub enum ArchiveError {
    InvalidPath(&'static str),
    InvalidSource(&'static str),
    InvalidArchive(&'static str),
    Collision,
    Encode,
    Decode,
    IdentityMismatch,
    NotFreshLiveFail,
    Integrity,
    Io {
        operation: &'static str,
        source: io::Error,
    },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(reason) => write!(formatter, "invalid archive path: {reason}"),
            Self::InvalidSource(reason) => write!(formatter, "invalid archive source: {reason}"),
            Self::InvalidArchive(reason) => write!(formatter, "invalid archive: {reason}"),
            Self::Collision => formatter.write_str("archive destination already exists"),
            Self::Encode => formatter.write_str("archive metadata serialization failed"),
            Self::Decode => formatter.write_str("archive evidence failed strict V1 decoding"),
            Self::IdentityMismatch => {
                formatter.write_str("archive source identity does not match evidence")
            }
            Self::NotFreshLiveFail => {
                formatter.write_str("archive source is not a fresh live G0 FAIL")
            }
            Self::Integrity => formatter.write_str("archive integrity verification failed"),
            Self::Io { operation, .. } => write!(formatter, "archive {operation} failed"),
        }
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn archive_pre_reboot(
    archive_root: &Path,
    evidence_path: &Path,
) -> Result<ArchivedG0V1, ArchiveError> {
    archive_with_contract(archive_root, evidence_path, PRE_REBOOT_ARCHIVE_CONTRACT)
}

pub fn archive_post_repair(
    archive_root: &Path,
    evidence_path: &Path,
) -> Result<ArchivedG0V1, ArchiveError> {
    archive_with_contract(archive_root, evidence_path, POST_REPAIR_ARCHIVE_CONTRACT)
}

fn archive_with_contract(
    archive_root: &Path,
    evidence_path: &Path,
    contract: ArchiveContract,
) -> Result<ArchivedG0V1, ArchiveError> {
    let root = open_directory_path(archive_root, true)?;
    require_absent(&root, OsStr::new(ARCHIVE_INDEX_NAME))?;

    let mut evidence_source = open_existing_file(evidence_path, "evidence source open")?;
    let evidence_metadata = evidence_source
        .metadata()
        .map_err(|source| io_error("evidence source metadata", source))?;
    require_regular_source(&evidence_metadata, Some(0o600), true)?;
    let evidence_bytes = read_limited(
        &mut evidence_source,
        MAX_G0_ENVELOPE_BYTES,
        "evidence source read",
    )?;
    let evidence_digest = sha256_bytes(&evidence_bytes);
    let envelope = decode_g0_evidence(&evidence_bytes)
        .map_err(|_| ArchiveError::Decode)?
        .into_v1();
    if contract.validate_persisted_envelope
        && !crate::evidence::validate_persisted_envelope(&envelope)
    {
        return Err(ArchiveError::InvalidSource(
            "evidence fails persisted semantic validation",
        ));
    }

    let executable_fd = rustix::fs::open(
        "/proc/self/exe",
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error("proc executable open", error))?;
    let mut executable_source = File::from(executable_fd);
    let executable_metadata = executable_source
        .metadata()
        .map_err(|source| io_error("proc executable descriptor metadata", source))?;
    require_regular_source(&executable_metadata, None, false)?;
    if executable_metadata.len() > MAX_ARCHIVED_BINARY_BYTES {
        return Err(ArchiveError::InvalidSource(
            "executing binary exceeds archive bound",
        ));
    }
    let source_descriptor = G0ArchiveSourceExecutableV1 {
        kind: "proc-self-exe-magic-link".to_owned(),
        file_type: "regular".to_owned(),
        mode: executable_metadata.mode(),
        device: executable_metadata.dev(),
        inode: executable_metadata.ino(),
        size_bytes: executable_metadata.len(),
    };

    verify_fresh_live(&envelope, None, contract)?;
    if !safe_run_id(&envelope.base.run_id) {
        return Err(ArchiveError::InvalidSource(
            "run identity cannot name an archive directory",
        ));
    }

    let run_directory_name = format!("{}-{evidence_digest}", envelope.base.run_id);
    let sequence = ARCHIVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staging_name = format!(".staging-{}-{sequence}", std::process::id());
    rustix::fs::mkdirat(&root, staging_name.as_str(), DIRECTORY_MODE)
        .map_err(|error| map_create_error("staging directory create", error))?;
    rustix::fs::fsync(&root).map_err(|error| errno_error("archive root fsync", error))?;
    let staging_fd = rustix::fs::openat(
        &root,
        staging_name.as_str(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error("staging directory open", error))?;
    let staging = File::from(staging_fd);

    let transaction = (|| {
        let mut archived_binary =
            create_file_at(&staging, OsStr::new(ARCHIVED_BINARY_NAME), "binary create")?;
        let (source_binary_digest, copied_size) = copy_and_hash(
            &mut executable_source,
            &mut archived_binary,
            MAX_ARCHIVED_BINARY_BYTES,
        )?;
        if copied_size != executable_metadata.len() {
            return Err(ArchiveError::Integrity);
        }
        seal_file(&mut archived_binary, IMMUTABLE_BINARY_MODE, "binary")?;
        let archived_binary_digest = hash_open_file(&mut archived_binary, "archived binary hash")?;
        if archived_binary_digest != source_binary_digest
            || archived_binary_digest != envelope.base.executable_sha256
        {
            return Err(ArchiveError::IdentityMismatch);
        }
        let archived_binary_metadata = archived_binary
            .metadata()
            .map_err(|source| io_error("archived binary metadata", source))?;
        require_archived_file(&archived_binary_metadata, 0o500, copied_size)?;

        let mut archived_evidence = create_file_at(
            &staging,
            OsStr::new(ARCHIVED_EVIDENCE_NAME),
            "evidence create",
        )?;
        archived_evidence
            .write_all(&evidence_bytes)
            .map_err(|source| io_error("archived evidence write", source))?;
        seal_file(&mut archived_evidence, IMMUTABLE_DATA_MODE, "evidence")?;
        let archived_evidence_digest =
            hash_open_file(&mut archived_evidence, "archived evidence hash")?;
        if archived_evidence_digest != evidence_digest {
            return Err(ArchiveError::Integrity);
        }
        let archived_evidence_metadata = archived_evidence
            .metadata()
            .map_err(|source| io_error("archived evidence metadata", source))?;
        require_archived_file(
            &archived_evidence_metadata,
            0o400,
            evidence_bytes.len() as u64,
        )?;

        verify_fresh_live(&envelope, Some(source_binary_digest), contract)?;
        let manifest = manifest_from_envelope(
            &envelope,
            source_descriptor,
            copied_size,
            source_binary_digest,
            evidence_bytes.len() as u64,
            evidence_digest,
            contract.manifest_schema,
        );
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| ArchiveError::Encode)?;
        if manifest_bytes.len() > MAX_ARCHIVE_METADATA_BYTES {
            return Err(ArchiveError::Encode);
        }
        let mut manifest_file = create_file_at(
            &staging,
            OsStr::new(ARCHIVE_MANIFEST_NAME),
            "manifest create",
        )?;
        manifest_file
            .write_all(&manifest_bytes)
            .map_err(|source| io_error("manifest write", source))?;
        seal_file(&mut manifest_file, IMMUTABLE_DATA_MODE, "manifest")?;
        let manifest_sha256 = hash_open_file(&mut manifest_file, "manifest hash")?;
        if manifest_sha256 != sha256_bytes(&manifest_bytes) {
            return Err(ArchiveError::Integrity);
        }
        let manifest_metadata = manifest_file
            .metadata()
            .map_err(|source| io_error("manifest metadata", source))?;
        require_archived_file(&manifest_metadata, 0o400, manifest_bytes.len() as u64)?;

        rustix::fs::fsync(&staging)
            .map_err(|error| errno_error("staging directory fsync", error))?;
        rustix::fs::fchmod(&staging, IMMUTABLE_DIRECTORY_MODE)
            .map_err(|error| errno_error("staging directory chmod", error))?;
        rustix::fs::fsync(&staging)
            .map_err(|error| errno_error("sealed staging directory fsync", error))?;
        rustix::fs::renameat_with(
            &root,
            staging_name.as_str(),
            &root,
            run_directory_name.as_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| map_create_error("run directory commit", error))?;
        rustix::fs::fsync(&root)
            .map_err(|error| errno_error("committed run parent fsync", error))?;

        Ok((manifest, manifest_sha256))
    })();

    let (manifest, manifest_sha256) = match transaction {
        Ok(result) => result,
        Err(error) => {
            cleanup_staging(&root, &staging, &staging_name);
            return Err(error);
        }
    };

    let manifest_path = format!("{run_directory_name}/{ARCHIVE_MANIFEST_NAME}");
    let index = G0ArchiveIndexV1 {
        schema: contract.index_schema.to_owned(),
        version: ARCHIVE_VERSION_V1,
        manifest_path: manifest_path.clone(),
        manifest_sha256,
    };
    let index_bytes = serde_json::to_vec(&index).map_err(|_| ArchiveError::Encode)?;
    let index_staging_name = format!(".index-staging-{}-{sequence}", std::process::id());
    let mut index_file = create_file_at(
        &root,
        OsStr::new(&index_staging_name),
        "index staging create",
    )?;
    let index_commit = (|| {
        index_file
            .write_all(&index_bytes)
            .map_err(|source| io_error("index write", source))?;
        seal_file(&mut index_file, IMMUTABLE_DATA_MODE, "index")?;
        let archived_index_digest = hash_open_file(&mut index_file, "index hash")?;
        if archived_index_digest != sha256_bytes(&index_bytes) {
            return Err(ArchiveError::Integrity);
        }
        rustix::fs::renameat_with(
            &root,
            index_staging_name.as_str(),
            &root,
            ARCHIVE_INDEX_NAME,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| map_create_error("index commit", error))?;
        rustix::fs::fsync(&root).map_err(|error| errno_error("index parent fsync", error))
    })();
    if let Err(error) = index_commit {
        let _ = rustix::fs::unlinkat(&root, index_staging_name.as_str(), AtFlags::empty());
        return Err(error);
    }

    Ok(ArchivedG0V1 {
        index_path: archive_root.join(ARCHIVE_INDEX_NAME),
        manifest_path: archive_root.join(manifest_path),
        manifest_sha256,
        manifest,
    })
}

pub fn verify_archive(index_path: &Path) -> Result<G0ArchiveManifestV1, ArchiveError> {
    let (index_parent, index_name) = open_existing_parent(index_path)?;
    if index_name != OsStr::new(ARCHIVE_INDEX_NAME) {
        return Err(ArchiveError::InvalidPath(
            "archive index must be named index.json",
        ));
    }
    let index_bytes = read_archived_file_at(
        &index_parent,
        &index_name,
        0o400,
        MAX_ARCHIVE_METADATA_BYTES,
        "index",
    )?;
    let dispatch: G0ArchiveIndexDispatch =
        serde_json::from_slice(&index_bytes).map_err(|_| ArchiveError::InvalidArchive("index"))?;
    let contract = ArchiveContract::from_index_schema(&dispatch.schema)
        .ok_or(ArchiveError::InvalidArchive("index schema"))?;
    let index: G0ArchiveIndexV1 =
        serde_json::from_slice(&index_bytes).map_err(|_| ArchiveError::InvalidArchive("index"))?;
    if index.schema != contract.index_schema || index.version != ARCHIVE_VERSION_V1 {
        return Err(ArchiveError::InvalidArchive("index schema"));
    }
    let (run_name, manifest_name) = contained_manifest_path(&index.manifest_path)?;
    let run_fd = rustix::fs::openat(
        &index_parent,
        run_name.as_str(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error("run directory open", error))?;
    let run_directory = File::from(run_fd);
    let run_metadata = run_directory
        .metadata()
        .map_err(|source| io_error("run directory metadata", source))?;
    if !run_metadata.is_dir() || run_metadata.mode() & 0o777 != 0o500 {
        return Err(ArchiveError::InvalidArchive("run directory type or mode"));
    }

    let manifest_bytes = read_archived_file_at(
        &run_directory,
        manifest_name.as_os_str(),
        0o400,
        MAX_ARCHIVE_METADATA_BYTES,
        "manifest",
    )?;
    if sha256_bytes(&manifest_bytes) != index.manifest_sha256 {
        return Err(ArchiveError::Integrity);
    }
    let manifest: G0ArchiveManifestV1 = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| ArchiveError::InvalidArchive("manifest"))?;
    validate_manifest_shape(&manifest, &run_name, contract)?;

    let binary_name = contained_file_name(&manifest.archived_binary.path)?;
    let binary_digest = hash_archived_file_at(
        &run_directory,
        binary_name.as_os_str(),
        manifest.archived_binary.mode,
        manifest.archived_binary.size_bytes,
        MAX_ARCHIVED_BINARY_BYTES,
        "archived binary",
    )?;
    if binary_digest != manifest.archived_binary.sha256 {
        return Err(ArchiveError::Integrity);
    }

    let evidence_name = contained_file_name(&manifest.evidence.path)?;
    let evidence_bytes = read_archived_file_at(
        &run_directory,
        evidence_name.as_os_str(),
        manifest.evidence.mode,
        MAX_G0_ENVELOPE_BYTES,
        "archived evidence",
    )?;
    if evidence_bytes.len() as u64 != manifest.evidence.size_bytes
        || sha256_bytes(&evidence_bytes) != manifest.evidence.sha256
    {
        return Err(ArchiveError::Integrity);
    }
    let envelope = decode_g0_evidence(&evidence_bytes)
        .map_err(|_| ArchiveError::Decode)?
        .into_v1();
    if contract.validate_persisted_envelope
        && !crate::evidence::validate_persisted_envelope(&envelope)
    {
        return Err(ArchiveError::InvalidArchive("evidence semantic validation"));
    }
    verify_manifest_evidence(&manifest, &envelope, binary_digest, contract)?;
    Ok(manifest)
}

fn manifest_from_envelope(
    envelope: &G0EvidenceEnvelopeV1,
    source_executable: G0ArchiveSourceExecutableV1,
    binary_size: u64,
    binary_sha256: Sha256DigestV1,
    evidence_size: u64,
    evidence_sha256: Sha256DigestV1,
    manifest_schema: &str,
) -> G0ArchiveManifestV1 {
    G0ArchiveManifestV1 {
        schema: manifest_schema.to_owned(),
        version: ARCHIVE_VERSION_V1,
        run_id: envelope.base.run_id.clone(),
        boot_id: envelope.base.boot_id.clone(),
        session_id: envelope.base.session_id.clone(),
        argv: envelope.base.argv.clone(),
        wall_started_unix_ns: envelope.base.wall_started_unix_ns,
        wall_finished_unix_ns: envelope.base.wall_finished_unix_ns,
        monotonic_started_ns: envelope.base.monotonic_started_ns,
        monotonic_finished_ns: envelope.base.monotonic_finished_ns,
        provenance: envelope.base.provenance,
        evidence_status: envelope.base.status,
        evidence_schema: envelope.schema.clone(),
        evidence_version: envelope.version,
        evidence_base_schema: envelope.base.schema.clone(),
        source_executable,
        archived_binary: G0ArchiveFileV1 {
            path: ARCHIVED_BINARY_NAME.to_owned(),
            size_bytes: binary_size,
            sha256: binary_sha256,
            mode: 0o500,
        },
        evidence: G0ArchiveFileV1 {
            path: ARCHIVED_EVIDENCE_NAME.to_owned(),
            size_bytes: evidence_size,
            sha256: evidence_sha256,
            mode: 0o400,
        },
        extensions: envelope
            .extensions
            .iter()
            .map(|extension| G0ArchiveExtensionV1 {
                id: extension.id.clone(),
                version: extension.version,
                status: extension.status,
                payload_sha256: extension.payload_sha256,
            })
            .collect(),
    }
}

fn verify_manifest_evidence(
    manifest: &G0ArchiveManifestV1,
    envelope: &G0EvidenceEnvelopeV1,
    binary_digest: Sha256DigestV1,
    contract: ArchiveContract,
) -> Result<(), ArchiveError> {
    if manifest.run_id != envelope.base.run_id
        || manifest.boot_id != envelope.base.boot_id
        || manifest.session_id != envelope.base.session_id
        || manifest.argv != envelope.base.argv
        || manifest.wall_started_unix_ns != envelope.base.wall_started_unix_ns
        || manifest.wall_finished_unix_ns != envelope.base.wall_finished_unix_ns
        || manifest.monotonic_started_ns != envelope.base.monotonic_started_ns
        || manifest.monotonic_finished_ns != envelope.base.monotonic_finished_ns
        || manifest.provenance != envelope.base.provenance
        || manifest.evidence_status != envelope.base.status
        || manifest.evidence_schema != envelope.schema
        || manifest.evidence_version != envelope.version
        || manifest.evidence_base_schema != envelope.base.schema
        || binary_digest != envelope.base.executable_sha256
        || manifest.source_executable.size_bytes != manifest.archived_binary.size_bytes
        || manifest.extensions.len() != envelope.extensions.len()
    {
        return Err(ArchiveError::IdentityMismatch);
    }
    for (archived, extension) in manifest.extensions.iter().zip(&envelope.extensions) {
        if archived.id != extension.id
            || archived.version != extension.version
            || archived.status != extension.status
            || archived.payload_sha256 != extension.payload_sha256
        {
            return Err(ArchiveError::Integrity);
        }
    }
    if envelope.base.provenance != G0EvidenceProvenanceV1::Live
        || !contract.accepts_status(envelope.base.status)
    {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    Ok(())
}

fn validate_manifest_shape(
    manifest: &G0ArchiveManifestV1,
    run_name: &str,
    contract: ArchiveContract,
) -> Result<(), ArchiveError> {
    if manifest.schema != contract.manifest_schema
        || manifest.version != ARCHIVE_VERSION_V1
        || manifest.archived_binary.path != ARCHIVED_BINARY_NAME
        || manifest.archived_binary.mode != 0o500
        || manifest.evidence.path != ARCHIVED_EVIDENCE_NAME
        || manifest.evidence.mode != 0o400
        || manifest.source_executable.kind != "proc-self-exe-magic-link"
        || manifest.source_executable.file_type != "regular"
        || manifest.source_executable.mode & FILE_TYPE_MASK != REGULAR_FILE_TYPE_BITS
        || manifest.source_executable.mode & 0o111 == 0
        || manifest.source_executable.size_bytes == 0
        || manifest.source_executable.size_bytes > MAX_ARCHIVED_BINARY_BYTES
        || !safe_run_id(&manifest.run_id)
    {
        return Err(ArchiveError::InvalidArchive("manifest shape"));
    }
    let expected_run_name = format!("{}-{}", manifest.run_id, manifest.evidence.sha256);
    if run_name != expected_run_name {
        return Err(ArchiveError::InvalidArchive("run directory identity"));
    }
    Ok(())
}

fn verify_fresh_live(
    envelope: &G0EvidenceEnvelopeV1,
    executable_digest: Option<Sha256DigestV1>,
    contract: ArchiveContract,
) -> Result<(), ArchiveError> {
    if envelope.base.provenance != G0EvidenceProvenanceV1::Live
        || !contract.accepts_status(envelope.base.status)
    {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    if executable_digest.is_some_and(|digest| digest != envelope.base.executable_sha256) {
        return Err(ArchiveError::IdentityMismatch);
    }
    if envelope.base.boot_id != current_boot_id()?
        || envelope.base.session_id != current_session_id()?
    {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    let policy = CurrentnessPolicy::default();
    let wall_duration = envelope
        .base
        .wall_finished_unix_ns
        .checked_sub(envelope.base.wall_started_unix_ns)
        .ok_or(ArchiveError::NotFreshLiveFail)?;
    let monotonic_duration = envelope
        .base
        .monotonic_finished_ns
        .checked_sub(envelope.base.monotonic_started_ns)
        .ok_or(ArchiveError::NotFreshLiveFail)?;
    if wall_duration > policy.max_run_duration_ns || monotonic_duration > policy.max_run_duration_ns
    {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    let wall_now = wall_unix_ns()?;
    let monotonic_now = monotonic_ns()?;
    if envelope.base.wall_finished_unix_ns > wall_now.saturating_add(policy.max_future_skew_ns)
        || envelope.base.monotonic_finished_ns > monotonic_now
        || wall_now.saturating_sub(envelope.base.wall_finished_unix_ns) > policy.max_age_ns
        || monotonic_now.saturating_sub(envelope.base.monotonic_finished_ns) > policy.max_age_ns
    {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    Ok(())
}

fn open_directory_path(path: &Path, create: bool) -> Result<File, ArchiveError> {
    let (mut current, components) = path_start_and_components(path)?;
    for component in components {
        let opened = rustix::fs::openat(
            &current,
            component.as_os_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        );
        let next = match opened {
            Ok(fd) => fd,
            Err(error) if create && error == rustix::io::Errno::NOENT => {
                rustix::fs::mkdirat(&current, component.as_os_str(), DIRECTORY_MODE)
                    .map_err(|error| map_create_error("archive root component create", error))?;
                rustix::fs::fsync(&current)
                    .map_err(|error| errno_error("archive root component parent fsync", error))?;
                rustix::fs::openat(
                    &current,
                    component.as_os_str(),
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|error| errno_error("archive root component open", error))?
            }
            Err(error) => return Err(errno_error("archive root component open", error)),
        };
        current = File::from(next);
        let metadata = current
            .metadata()
            .map_err(|source| io_error("archive root component metadata", source))?;
        if !metadata.is_dir() {
            return Err(ArchiveError::InvalidPath(
                "archive root component is not a directory",
            ));
        }
    }
    Ok(current)
}

fn open_existing_file(path: &Path, operation: &'static str) -> Result<File, ArchiveError> {
    let (parent, name) = open_existing_parent(path)?;
    let fd = rustix::fs::openat(
        &parent,
        name.as_os_str(),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error(operation, error))?;
    Ok(File::from(fd))
}

fn open_existing_parent(path: &Path) -> Result<(File, OsString), ArchiveError> {
    let (mut current, mut components) = path_start_and_components(path)?;
    let name = components
        .pop()
        .ok_or(ArchiveError::InvalidPath("a file name is required"))?;
    for component in components {
        let fd = rustix::fs::openat(
            &current,
            component.as_os_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| errno_error("path component open", error))?;
        current = File::from(fd);
    }
    Ok((current, name))
}

fn path_start_and_components(path: &Path) -> Result<(File, Vec<OsString>), ArchiveError> {
    if path.as_os_str().is_empty() {
        return Err(ArchiveError::InvalidPath("empty paths are not accepted"));
    }
    let start = if path.is_absolute() { "/" } else { "." };
    let start_fd = rustix::fs::open(
        start,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error("path anchor open", error))?;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(value) => components.push(value.to_os_string()),
            Component::ParentDir => {
                return Err(ArchiveError::InvalidPath(
                    "parent traversal is not accepted",
                ));
            }
            Component::Prefix(_) => {
                return Err(ArchiveError::InvalidPath(
                    "platform path prefixes are not accepted",
                ));
            }
        }
    }
    Ok((File::from(start_fd), components))
}

fn create_file_at(
    directory: &File,
    name: &OsStr,
    operation: &'static str,
) -> Result<File, ArchiveError> {
    if !safe_os_component(name) {
        return Err(ArchiveError::InvalidPath("unsafe destination file name"));
    }
    let fd = rustix::fs::openat(
        directory,
        name,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        WRITABLE_FILE_MODE,
    )
    .map_err(|error| map_create_error(operation, error))?;
    Ok(File::from(fd))
}

fn require_absent(directory: &File, name: &OsStr) -> Result<(), ArchiveError> {
    match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(_) => Err(ArchiveError::Collision),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(_) => Err(ArchiveError::Collision),
    }
}

fn require_regular_source(
    metadata: &std::fs::Metadata,
    expected_mode: Option<u32>,
    require_single_link: bool,
) -> Result<(), ArchiveError> {
    if !metadata.is_file()
        || (require_single_link && metadata.nlink() != 1)
        || expected_mode.is_some_and(|mode| metadata.mode() & 0o777 != mode)
    {
        return Err(ArchiveError::InvalidSource(
            "source must be one regular file with the required mode",
        ));
    }
    Ok(())
}

fn require_archived_file(
    metadata: &std::fs::Metadata,
    expected_mode: u32,
    expected_size: u64,
) -> Result<(), ArchiveError> {
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.mode() & 0o777 != expected_mode
        || metadata.len() != expected_size
    {
        return Err(ArchiveError::Integrity);
    }
    Ok(())
}

fn copy_and_hash(
    source: &mut File,
    destination: &mut File,
    maximum: u64,
) -> Result<(Sha256DigestV1, u64), ArchiveError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = source
            .read(&mut buffer)
            .map_err(|source| io_error("executing binary read", source))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(ArchiveError::InvalidSource(
                "executing binary size overflow",
            ))?;
        if total > maximum {
            return Err(ArchiveError::InvalidSource(
                "executing binary exceeds archive bound",
            ));
        }
        destination
            .write_all(&buffer[..read])
            .map_err(|source| io_error("archived binary write", source))?;
        hasher.update(&buffer[..read]);
    }
    let output = hasher.finalize();
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&output);
    Ok((Sha256DigestV1::from_bytes(digest), total))
}

fn seal_file(file: &mut File, mode: Mode, label: &'static str) -> Result<(), ArchiveError> {
    file.flush()
        .map_err(|source| io_error(label_operation(label, "flush"), source))?;
    rustix::fs::fchmod(&*file, mode)
        .map_err(|error| errno_error(label_operation(label, "chmod"), error))?;
    rustix::fs::fsync(&*file).map_err(|error| errno_error(label_operation(label, "fsync"), error))
}

fn hash_open_file(
    file: &mut File,
    operation: &'static str,
) -> Result<Sha256DigestV1, ArchiveError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|source| io_error(operation, source))?;
    sha256_reader(file).map_err(|source| io_error(operation, source))
}

fn read_archived_file_at(
    directory: &File,
    name: &OsStr,
    expected_mode: u32,
    maximum: usize,
    label: &'static str,
) -> Result<Vec<u8>, ArchiveError> {
    let fd = rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error(label_operation(label, "open"), error))?;
    let mut file = File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|source| io_error(label_operation(label, "metadata"), source))?;
    if metadata.len() > maximum as u64 {
        return Err(ArchiveError::InvalidArchive("archived file exceeds bound"));
    }
    require_archived_file(&metadata, expected_mode, metadata.len())?;
    read_limited(&mut file, maximum, label_operation(label, "read"))
}

fn hash_archived_file_at(
    directory: &File,
    name: &OsStr,
    expected_mode: u32,
    expected_size: u64,
    maximum: u64,
    label: &'static str,
) -> Result<Sha256DigestV1, ArchiveError> {
    if expected_size > maximum {
        return Err(ArchiveError::InvalidArchive("archived file exceeds bound"));
    }
    let fd = rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| errno_error(label_operation(label, "open"), error))?;
    let file = File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|source| io_error(label_operation(label, "metadata"), source))?;
    require_archived_file(&metadata, expected_mode, expected_size)?;
    sha256_reader(file).map_err(|source| io_error(label_operation(label, "hash"), source))
}

fn read_limited(
    file: &mut File,
    maximum: usize,
    operation: &'static str,
) -> Result<Vec<u8>, ArchiveError> {
    let mut bytes = Vec::new();
    file.take((maximum + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(operation, source))?;
    if bytes.len() > maximum {
        return Err(ArchiveError::InvalidSource("source exceeds size bound"));
    }
    Ok(bytes)
}

fn contained_manifest_path(value: &str) -> Result<(String, OsString), ArchiveError> {
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ArchiveError::InvalidArchive("absolute manifest path"));
    }
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_os_string()),
            _ => Err(ArchiveError::InvalidArchive("manifest path traversal")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.len() != 2
        || components[1] != OsStr::new(ARCHIVE_MANIFEST_NAME)
        || !safe_os_component(&components[0])
    {
        return Err(ArchiveError::InvalidArchive("manifest path containment"));
    }
    Ok((
        components[0]
            .to_str()
            .ok_or(ArchiveError::InvalidArchive("manifest path encoding"))?
            .to_owned(),
        components[1].clone(),
    ))
}

fn contained_file_name(value: &str) -> Result<OsString, ArchiveError> {
    let path = Path::new(value);
    let mut components = path.components();
    let Some(Component::Normal(name)) = components.next() else {
        return Err(ArchiveError::InvalidArchive("contained file path"));
    };
    if components.next().is_some() || !safe_os_component(name) {
        return Err(ArchiveError::InvalidArchive("contained file path"));
    }
    Ok(name.to_os_string())
}

fn safe_run_id(value: &str) -> bool {
    value.starts_with("run-")
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn safe_os_component(value: &OsStr) -> bool {
    let Some(value) = value.to_str() else {
        return false;
    };
    !value.is_empty() && value != "." && value != ".." && !value.contains('/')
}

fn cleanup_staging(root: &File, staging: &File, staging_name: &str) {
    for name in [
        ARCHIVE_MANIFEST_NAME,
        ARCHIVED_EVIDENCE_NAME,
        ARCHIVED_BINARY_NAME,
    ] {
        let _ = rustix::fs::unlinkat(staging, name, AtFlags::empty());
    }
    let _ = rustix::fs::unlinkat(root, staging_name, AtFlags::REMOVEDIR);
    let _ = rustix::fs::fsync(root);
}

fn current_boot_id() -> Result<String, ArchiveError> {
    let value = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(|source| io_error("current boot identity read", source))?;
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ArchiveError::NotFreshLiveFail);
    }
    Ok(value.to_owned())
}

fn current_session_id() -> Result<String, ArchiveError> {
    let session = rustix::process::getsid(None)
        .map_err(|error| errno_error("current process session read", error))?;
    Ok(format!("sid-{}", session.as_raw_nonzero().get()))
}

fn wall_unix_ns() -> Result<u64, ArchiveError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ArchiveError::NotFreshLiveFail)?;
    u64::try_from(elapsed.as_nanos()).map_err(|_| ArchiveError::NotFreshLiveFail)
}

fn monotonic_ns() -> Result<u64, ArchiveError> {
    let uptime = std::fs::read_to_string("/proc/uptime")
        .map_err(|source| io_error("current monotonic clock read", source))?;
    let token = uptime
        .split_ascii_whitespace()
        .next()
        .ok_or(ArchiveError::NotFreshLiveFail)?;
    parse_decimal_seconds_ns(token).ok_or(ArchiveError::NotFreshLiveFail)
}

fn parse_decimal_seconds_ns(value: &str) -> Option<u64> {
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
    let seconds = seconds.parse::<u64>().ok()?;
    if fraction.len() > 9 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut nanoseconds = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().ok()?
    };
    for _ in fraction.len()..9 {
        nanoseconds = nanoseconds.checked_mul(10)?;
    }
    seconds.checked_mul(1_000_000_000)?.checked_add(nanoseconds)
}

fn map_create_error(operation: &'static str, error: rustix::io::Errno) -> ArchiveError {
    if matches!(
        error,
        rustix::io::Errno::EXIST | rustix::io::Errno::NOTEMPTY
    ) {
        ArchiveError::Collision
    } else {
        errno_error(operation, error)
    }
}

fn errno_error(operation: &'static str, error: rustix::io::Errno) -> ArchiveError {
    io_error(
        operation,
        io::Error::from_raw_os_error(error.raw_os_error()),
    )
}

fn io_error(operation: &'static str, source: io::Error) -> ArchiveError {
    ArchiveError::Io { operation, source }
}

fn label_operation(label: &'static str, suffix: &'static str) -> &'static str {
    match (label, suffix) {
        ("binary", "flush") => "binary flush",
        ("binary", "chmod") => "binary chmod",
        ("binary", "fsync") => "binary fsync",
        ("evidence", "flush") => "evidence flush",
        ("evidence", "chmod") => "evidence chmod",
        ("evidence", "fsync") => "evidence fsync",
        ("manifest", "flush") => "manifest flush",
        ("manifest", "chmod") => "manifest chmod",
        ("manifest", "fsync") => "manifest fsync",
        ("index", "flush") => "index flush",
        ("index", "chmod") => "index chmod",
        ("index", "fsync") => "index fsync",
        ("index", "open") => "index open",
        ("index", "metadata") => "index metadata",
        ("index", "read") => "index read",
        ("manifest", "open") => "manifest open",
        ("manifest", "metadata") => "manifest metadata",
        ("manifest", "read") => "manifest read",
        ("archived evidence", "open") => "archived evidence open",
        ("archived evidence", "metadata") => "archived evidence metadata",
        ("archived evidence", "read") => "archived evidence read",
        ("archived binary", "open") => "archived binary open",
        ("archived binary", "metadata") => "archived binary metadata",
        ("archived binary", "hash") => "archived binary hash",
        _ => "archive file operation",
    }
}
