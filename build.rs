use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const NVML_SOURCE_ROOT_ENV: &str = "REPLAY_NVML_SDK_ROOT";
const NVFBC_SOURCE_ROOT_ENV: &str = "REPLAY_NVFBC_SDK_ROOT";
const CUDA_SOURCE_ROOT_ENV: &str = "REPLAY_CUDA_SDK_ROOT";
const NVML_HEADER_NAME: &str = "nvml.h";
const NVFBC_HEADER_NAME: &str = "NvFBC.h";
const CUDA_HEADER_NAME: &str = "cuda.h";
const CUDA_TYPEDEFS_HEADER_NAME: &str = "cudaTypedefs.h";
const NVML_SOURCE_IDENTITY: &str = "nvidia-nvml-api-13";
const NVFBC_SOURCE_IDENTITY: &str = "nvidia-nvfbc-api-1.9-cuda-driver-api-13.3";
const CUDA_SOURCE_IDENTITY: &str = "nvidia-cuda-driver-api-13.3";
const MAX_HEADER_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    len: u64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        }
    }
}

struct AuthenticatedHeader {
    asserted_root: PathBuf,
    canonical_root: PathBuf,
    root_identity: FileIdentity,
    canonical_path: PathBuf,
    file_identity: FileIdentity,
    name: &'static str,
    bytes: Vec<u8>,
    digest: String,
}

struct HeaderSnapshot {
    root: PathBuf,
    files: Vec<(PathBuf, String)>,
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(replay_nvml_source)");
    println!("cargo:rustc-check-cfg=cfg(replay_nvfbc_source)");
    println!("cargo:rerun-if-env-changed={NVML_SOURCE_ROOT_ENV}");
    println!("cargo:rerun-if-env-changed={NVFBC_SOURCE_ROOT_ENV}");
    println!("cargo:rerun-if-env-changed={CUDA_SOURCE_ROOT_ENV}");
    println!("cargo:rerun-if-changed=native/nvml_abi_oracle.c");
    println!("cargo:rerun-if-changed=native/nvfbc_abi_oracle.c");
    configure_nvml_source();
    configure_nvfbc_source();
}

fn configure_nvml_source() {
    let Some(root) = std::env::var_os(NVML_SOURCE_ROOT_ENV).map(PathBuf::from) else {
        return;
    };
    emit_asserted_source_reruns(&root, &[NVML_HEADER_NAME]);
    let header = authenticate_header(&root, NVML_HEADER_NAME, "NVML");
    require_header_markers(
        &header.bytes,
        &[
            "#define NVML_API_VERSION            13",
            "Copyright 1993-2026 NVIDIA Corporation.",
        ],
        "NVML",
    );

    let out_dir = required_out_dir();
    let snapshot = create_header_snapshot(&out_dir, "nvml", &[&header]);
    verify_header_snapshot(&snapshot, "NVML");
    compile_oracle(
        "native/nvml_abi_oracle.c",
        &[snapshot.root.as_path()],
        &out_dir,
        "nvml_abi_oracle",
    );
    verify_header_snapshot(&snapshot, "NVML");
    require_unchanged_header(&header, "NVML");
    unseal_header_snapshot(&snapshot);

    println!("cargo:rustc-cfg=replay_nvml_source");
    println!("cargo:rustc-env=REPLAY_NVML_SOURCE_IDENTITY={NVML_SOURCE_IDENTITY}");
    println!(
        "cargo:rustc-env=REPLAY_NVML_SOURCE_SHA256={}",
        header.digest
    );
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=replay_nvml_abi_oracle");
}

fn configure_nvfbc_source() {
    let nvfbc_root = std::env::var_os(NVFBC_SOURCE_ROOT_ENV).map(PathBuf::from);
    let cuda_root = std::env::var_os(CUDA_SOURCE_ROOT_ENV).map(PathBuf::from);
    let (nvfbc_root, cuda_root) = match (nvfbc_root, cuda_root) {
        (None, None) => return,
        (Some(_), None) => {
            panic!("{CUDA_SOURCE_ROOT_ENV} must be set when {NVFBC_SOURCE_ROOT_ENV} is set")
        }
        (None, Some(_)) => {
            panic!("{NVFBC_SOURCE_ROOT_ENV} must be set when {CUDA_SOURCE_ROOT_ENV} is set")
        }
        (Some(nvfbc_root), Some(cuda_root)) => (nvfbc_root, cuda_root),
    };

    emit_asserted_source_reruns(&nvfbc_root, &[NVFBC_HEADER_NAME]);
    emit_asserted_source_reruns(&cuda_root, &[CUDA_HEADER_NAME, CUDA_TYPEDEFS_HEADER_NAME]);

    let nvfbc_header = authenticate_header(&nvfbc_root, NVFBC_HEADER_NAME, "NvFBC");
    let cuda_header = authenticate_header(&cuda_root, CUDA_HEADER_NAME, "CUDA");
    let cuda_typedefs = authenticate_header(&cuda_root, CUDA_TYPEDEFS_HEADER_NAME, "CUDA typedefs");

    require_header_markers(
        &nvfbc_header.bytes,
        &[
            "#define NVFBC_VERSION_MAJOR 1",
            "#define NVFBC_VERSION_MINOR 9",
            "} NVFBC_API_FUNCTION_LIST;",
            "NvFBCCreateInstance(NVFBC_API_FUNCTION_LIST *pFunctionList)",
        ],
        "NvFBC",
    );
    require_header_markers(
        &cuda_header.bytes,
        &[
            "#define CUDA_VERSION 13030",
            "typedef unsigned long long CUdeviceptr_v2;",
            "CU_POINTER_ATTRIBUTE_MEMORY_TYPE",
        ],
        "CUDA",
    );
    require_header_markers(
        &cuda_typedefs.bytes,
        &[
            "PFN_cuGetProcAddress_v12000",
            "PFN_cuCtxCreate_v3020",
            "PFN_cuMemcpyDtoD_v3020",
        ],
        "CUDA typedefs",
    );

    let out_dir = required_out_dir();
    let snapshot = create_header_snapshot(
        &out_dir,
        "nvfbc-cuda",
        &[&nvfbc_header, &cuda_header, &cuda_typedefs],
    );
    verify_header_snapshot(&snapshot, "NvFBC/CUDA");
    compile_oracle(
        "native/nvfbc_abi_oracle.c",
        &[snapshot.root.as_path()],
        &out_dir,
        "nvfbc_abi_oracle",
    );
    verify_header_snapshot(&snapshot, "NvFBC/CUDA");
    require_unchanged_header(&nvfbc_header, "NvFBC");
    require_unchanged_header(&cuda_header, "CUDA");
    require_unchanged_header(&cuda_typedefs, "CUDA typedefs");
    unseal_header_snapshot(&snapshot);

    println!("cargo:rustc-cfg=replay_nvfbc_source");
    println!("cargo:rustc-env=REPLAY_NVFBC_SOURCE_IDENTITY={NVFBC_SOURCE_IDENTITY}");
    println!("cargo:rustc-env=REPLAY_CUDA_SOURCE_IDENTITY={CUDA_SOURCE_IDENTITY}");
    println!(
        "cargo:rustc-env=REPLAY_NVFBC_SOURCE_SHA256={}",
        nvfbc_header.digest
    );
    println!(
        "cargo:rustc-env=REPLAY_CUDA_SOURCE_SHA256={}",
        cuda_header.digest
    );
    println!(
        "cargo:rustc-env=REPLAY_CUDA_TYPEDEFS_SOURCE_SHA256={}",
        cuda_typedefs.digest
    );
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=replay_nvfbc_abi_oracle");
}

fn required_out_dir() -> PathBuf {
    std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("Cargo did not provide OUT_DIR"))
}

fn emit_asserted_source_reruns(root: &Path, header_names: &[&str]) {
    println!("cargo:rerun-if-changed={}", root.display());
    for name in header_names {
        println!("cargo:rerun-if-changed={}", root.join(name).display());
    }
}

fn authenticate_header(
    asserted_root: &Path,
    name: &'static str,
    label: &str,
) -> AuthenticatedHeader {
    if !asserted_root.is_absolute() {
        panic!("{label} source root must be absolute");
    }
    reject_symlink_components(asserted_root, label);
    let canonical_root = std::fs::canonicalize(asserted_root)
        .unwrap_or_else(|_| panic!("{label} source root is unreadable"));
    if canonical_root != asserted_root {
        panic!("{label} source root must use one canonical, symlink-free spelling");
    }
    let root_metadata = std::fs::symlink_metadata(asserted_root)
        .unwrap_or_else(|_| panic!("{label} source root metadata is unreadable"));
    if !root_metadata.is_dir() {
        panic!("{label} source root is not a directory");
    }
    let root_identity = FileIdentity::from_metadata(&root_metadata);

    let candidate = asserted_root.join(name);
    reject_symlink_components(&candidate, label);
    let path_metadata = std::fs::symlink_metadata(&candidate)
        .unwrap_or_else(|_| panic!("{label} header is missing"));
    if !path_metadata.is_file() || path_metadata.file_type().is_symlink() {
        panic!("{label} header must be a regular non-symlink file");
    }

    let canonical_path = std::fs::canonicalize(&candidate)
        .unwrap_or_else(|_| panic!("{label} header is unreadable"));
    if canonical_path != candidate || !canonical_path.starts_with(&canonical_root) {
        panic!("{label} header path identity is ambiguous");
    }

    let mut file =
        File::open(&candidate).unwrap_or_else(|_| panic!("{label} header is unreadable"));
    let opened_metadata = file
        .metadata()
        .unwrap_or_else(|_| panic!("{label} header metadata is unreadable"));
    let file_identity = FileIdentity::from_metadata(&opened_metadata);
    if file_identity != FileIdentity::from_metadata(&path_metadata)
        || !opened_metadata.is_file()
        || opened_metadata.len() == 0
        || opened_metadata.len() > MAX_HEADER_BYTES
    {
        panic!("{label} header changed while it was being opened");
    }

    let mut bytes = Vec::with_capacity(
        usize::try_from(opened_metadata.len())
            .unwrap_or_else(|_| panic!("{label} header size does not fit memory")),
    );
    std::io::Read::by_ref(&mut file)
        .take(MAX_HEADER_BYTES + 1)
        .read_to_end(&mut bytes)
        .unwrap_or_else(|_| panic!("{label} header read failed"));
    let finished_metadata = file
        .metadata()
        .unwrap_or_else(|_| panic!("{label} header metadata became unreadable"));
    if FileIdentity::from_metadata(&finished_metadata) != file_identity
        || u64::try_from(bytes.len()).ok() != Some(file_identity.len)
    {
        panic!("{label} header changed while it was being read");
    }

    let final_path_metadata = std::fs::symlink_metadata(&candidate)
        .unwrap_or_else(|_| panic!("{label} header path disappeared"));
    if FileIdentity::from_metadata(&final_path_metadata) != file_identity
        || final_path_metadata.file_type().is_symlink()
    {
        panic!("{label} header path changed while it was being authenticated");
    }
    let digest = sha256_bytes(&bytes).unwrap_or_else(|| panic!("{label} header hashing failed"));

    AuthenticatedHeader {
        asserted_root: asserted_root.to_path_buf(),
        canonical_root,
        root_identity,
        canonical_path,
        file_identity,
        name,
        bytes,
        digest,
    }
}

fn reject_symlink_components(path: &Path, label: &str) {
    let mut current = PathBuf::new();
    let components: Vec<_> = path.components().collect();
    for (index, component) in components.iter().enumerate() {
        match component {
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
            Component::CurDir | Component::ParentDir => {
                panic!("{label} source path must not contain dot components");
            }
        }
        let metadata = std::fs::symlink_metadata(&current)
            .unwrap_or_else(|_| panic!("{label} source path component is missing"));
        if metadata.file_type().is_symlink() {
            panic!("{label} source path must not contain symlinks");
        }
        if index + 1 != components.len() && !metadata.is_dir() {
            panic!("{label} source path has a non-directory component");
        }
    }
}

fn require_header_markers(bytes: &[u8], markers: &[&str], label: &str) {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|_| panic!("{label} header is not UTF-8"));
    if markers.iter().any(|marker| !text.contains(marker)) {
        panic!("{label} header identity does not match the supported API");
    }
}

fn require_unchanged_header(before: &AuthenticatedHeader, label: &str) {
    let after = authenticate_header(&before.asserted_root, before.name, label);
    if after.canonical_root != before.canonical_root
        || after.root_identity != before.root_identity
        || after.canonical_path != before.canonical_path
        || after.file_identity != before.file_identity
        || after.digest != before.digest
    {
        panic!("{label} source identity changed while the ABI oracle was compiling");
    }
}

fn create_header_snapshot(
    out_dir: &Path,
    namespace: &str,
    headers: &[&AuthenticatedHeader],
) -> HeaderSnapshot {
    let fingerprint = headers
        .iter()
        .map(|header| &header.digest[..16])
        .collect::<Vec<_>>()
        .join("-");
    let snapshot_root = out_dir.join(format!(
        "replay-authenticated-{namespace}-headers-{fingerprint}"
    ));
    create_or_validate_private_directory(&snapshot_root);

    let mut expected_names = BTreeSet::new();
    let mut files = Vec::with_capacity(headers.len());
    for header in headers {
        let name = OsString::from(header.name);
        if !expected_names.insert(name) {
            panic!("authenticated header snapshot contains a duplicate name");
        }
        let snapshot_path = snapshot_root.join(header.name);
        create_or_validate_snapshot_file(&snapshot_path, &header.bytes, &header.digest);
        files.push((snapshot_path, header.digest.clone()));
    }
    require_exact_snapshot_entries(&snapshot_root, &expected_names);
    seal_snapshot_directory(&snapshot_root);

    HeaderSnapshot {
        root: snapshot_root,
        files,
    }
}

fn create_or_validate_private_directory(path: &Path) {
    let created = {
        #[cfg(unix)]
        {
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            match builder.create(path) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(_) => panic!("could not create private authenticated-header snapshot"),
            }
        }
        #[cfg(not(unix))]
        {
            match std::fs::create_dir(path) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(_) => panic!("could not create private authenticated-header snapshot"),
            }
        }
    };
    #[cfg(not(unix))]
    let _ = created;

    let metadata = std::fs::symlink_metadata(path)
        .unwrap_or_else(|_| panic!("authenticated-header snapshot metadata is unreadable"));
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        panic!("authenticated-header snapshot must be a real directory");
    }
    #[cfg(unix)]
    {
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o700 && (created || mode != 0o500) {
            panic!("authenticated-header snapshot is not private");
        }
    }
}

fn seal_snapshot_directory(path: &Path) {
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o500))
            .unwrap_or_else(|_| panic!("authenticated-header snapshot sealing failed"));
    }
    verify_sealed_snapshot_directory(path);
}

fn verify_sealed_snapshot_directory(path: &Path) {
    let metadata = std::fs::symlink_metadata(path)
        .unwrap_or_else(|_| panic!("authenticated-header snapshot directory is missing"));
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        panic!("authenticated-header snapshot directory is invalid");
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o500 {
        panic!("authenticated-header snapshot directory is not sealed");
    }
}

fn unseal_header_snapshot(snapshot: &HeaderSnapshot) {
    #[cfg(unix)]
    {
        std::fs::set_permissions(&snapshot.root, std::fs::Permissions::from_mode(0o700))
            .unwrap_or_else(|_| panic!("authenticated-header snapshot cleanup failed"));
    }
    #[cfg(not(unix))]
    let _ = snapshot;
}

fn create_or_validate_snapshot_file(path: &Path, bytes: &[u8], digest: &str) {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o400);
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(bytes)
                .unwrap_or_else(|_| panic!("authenticated-header snapshot write failed"));
            file.sync_all()
                .unwrap_or_else(|_| panic!("authenticated-header snapshot sync failed"));
            drop(file);
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => panic!("authenticated-header snapshot creation failed"),
    }
    verify_snapshot_file(path, digest);
}

fn require_exact_snapshot_entries(root: &Path, expected: &BTreeSet<OsString>) {
    let observed: BTreeSet<_> = std::fs::read_dir(root)
        .unwrap_or_else(|_| panic!("authenticated-header snapshot is unreadable"))
        .map(|entry| {
            entry
                .unwrap_or_else(|_| panic!("authenticated-header snapshot entry is unreadable"))
                .file_name()
        })
        .collect();
    if observed != *expected {
        panic!("authenticated-header snapshot contains unexpected files");
    }
}

fn verify_header_snapshot(snapshot: &HeaderSnapshot, label: &str) {
    verify_sealed_snapshot_directory(&snapshot.root);
    let expected_names: BTreeSet<_> = snapshot
        .files
        .iter()
        .map(|(path, _)| {
            path.file_name()
                .unwrap_or_else(|| panic!("{label} snapshot file has no name"))
                .to_os_string()
        })
        .collect();
    require_exact_snapshot_entries(&snapshot.root, &expected_names);
    for (path, digest) in &snapshot.files {
        verify_snapshot_file(path, digest);
    }
}

fn verify_snapshot_file(path: &Path, digest: &str) {
    let metadata = std::fs::symlink_metadata(path)
        .unwrap_or_else(|_| panic!("authenticated-header snapshot file is missing"));
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_HEADER_BYTES
    {
        panic!("authenticated-header snapshot file is invalid");
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o400 {
        panic!("authenticated-header snapshot file must be read-only and private");
    }
    let observed =
        sha256sum(path).unwrap_or_else(|| panic!("authenticated-header snapshot hashing failed"));
    if observed != digest {
        panic!("authenticated-header snapshot digest mismatch");
    }
}

fn sha256_bytes(bytes: &[u8]) -> Option<String> {
    let mut child = Command::new("/usr/bin/sha256sum")
        .arg("-")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(bytes).ok()?;
    let output = child.wait_with_output().ok()?;
    parse_sha256_output(&output)
}

fn sha256sum(path: &Path) -> Option<String> {
    let output = Command::new("/usr/bin/sha256sum")
        .arg("--")
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    parse_sha256_output(&output)
}

fn parse_sha256_output(output: &std::process::Output) -> Option<String> {
    if !output.status.success() || !output.stderr.is_empty() {
        return None;
    }
    let stdout = std::str::from_utf8(&output.stdout).ok()?;
    let digest = stdout.split_ascii_whitespace().next()?.to_owned();
    (digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
    .then_some(digest)
}

fn compile_oracle(source: &str, include_roots: &[&Path], out_dir: &Path, stem: &str) {
    let object = out_dir.join(format!("{stem}.o"));
    let archive = out_dir.join(format!("libreplay_{stem}.a"));
    remove_stale_output(&object);
    remove_stale_output(&archive);

    let mut command = Command::new("/usr/bin/cc");
    command.args([
        OsStr::new("-std=c11"),
        OsStr::new("-Wall"),
        OsStr::new("-Wextra"),
        OsStr::new("-Werror"),
        OsStr::new("-fPIC"),
        OsStr::new("-c"),
        OsStr::new(source),
        OsStr::new("-o"),
    ]);
    command.arg(&object);
    for root in include_roots {
        command.arg("-I").arg(root);
    }
    let compiled = command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .status()
        .is_ok_and(|status| status.success());
    if !compiled {
        panic!("{stem} source/ABI oracle compilation failed");
    }

    let archived = Command::new("/usr/bin/ar")
        .args([OsStr::new("crs")])
        .arg(&archive)
        .arg(&object)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .status()
        .is_ok_and(|status| status.success());
    if !archived {
        panic!("{stem} source/ABI oracle archive creation failed");
    }
}

fn remove_stale_output(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => panic!("could not replace a stale ABI oracle build artifact"),
    }
}
