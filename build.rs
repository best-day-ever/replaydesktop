use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[derive(Clone)]
struct AuthenticatedHeader {
    canonical_root: PathBuf,
    canonical_path: PathBuf,
    digest: String,
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
    let header = authenticate_header(&root, NVML_HEADER_NAME, "NVML");
    require_header_markers(
        &header.canonical_path,
        &[
            "#define NVML_API_VERSION            13",
            "Copyright 1993-2026 NVIDIA Corporation.",
        ],
        "NVML",
    );
    emit_header_rerun(&header);

    let out_dir = required_out_dir();
    compile_oracle(
        "native/nvml_abi_oracle.c",
        &[header.canonical_root.as_path()],
        &out_dir,
        "nvml_abi_oracle",
    );
    require_unchanged_header(&root, NVML_HEADER_NAME, &header, "NVML");

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

    let nvfbc_header = authenticate_header(&nvfbc_root, NVFBC_HEADER_NAME, "NvFBC");
    let cuda_header = authenticate_header(&cuda_root, CUDA_HEADER_NAME, "CUDA");
    let cuda_typedefs = authenticate_header(&cuda_root, CUDA_TYPEDEFS_HEADER_NAME, "CUDA typedefs");

    require_header_markers(
        &nvfbc_header.canonical_path,
        &[
            "#define NVFBC_VERSION_MAJOR 1",
            "#define NVFBC_VERSION_MINOR 9",
            "} NVFBC_API_FUNCTION_LIST;",
            "NvFBCCreateInstance(NVFBC_API_FUNCTION_LIST *pFunctionList)",
        ],
        "NvFBC",
    );
    require_header_markers(
        &cuda_header.canonical_path,
        &[
            "#define CUDA_VERSION 13030",
            "typedef unsigned long long CUdeviceptr_v2;",
            "CU_POINTER_ATTRIBUTE_MEMORY_TYPE",
        ],
        "CUDA",
    );
    require_header_markers(
        &cuda_typedefs.canonical_path,
        &[
            "PFN_cuGetProcAddress_v12000",
            "PFN_cuCtxCreate_v3020",
            "PFN_cuMemcpyDtoD_v3020",
        ],
        "CUDA typedefs",
    );

    emit_header_rerun(&nvfbc_header);
    emit_header_rerun(&cuda_header);
    emit_header_rerun(&cuda_typedefs);

    let out_dir = required_out_dir();
    compile_oracle(
        "native/nvfbc_abi_oracle.c",
        &[
            nvfbc_header.canonical_root.as_path(),
            cuda_header.canonical_root.as_path(),
        ],
        &out_dir,
        "nvfbc_abi_oracle",
    );

    require_unchanged_header(&nvfbc_root, NVFBC_HEADER_NAME, &nvfbc_header, "NvFBC");
    require_unchanged_header(&cuda_root, CUDA_HEADER_NAME, &cuda_header, "CUDA");
    require_unchanged_header(
        &cuda_root,
        CUDA_TYPEDEFS_HEADER_NAME,
        &cuda_typedefs,
        "CUDA typedefs",
    );

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

fn authenticate_header(root: &Path, name: &str, label: &str) -> AuthenticatedHeader {
    if !root.is_absolute() {
        panic!("{label} source root must be absolute");
    }
    let canonical_root =
        std::fs::canonicalize(root).unwrap_or_else(|_| panic!("{label} source root is unreadable"));
    if !canonical_root.is_dir() {
        panic!("{label} source root is not a directory");
    }

    let candidate = canonical_root.join(name);
    let link_metadata = std::fs::symlink_metadata(&candidate)
        .unwrap_or_else(|_| panic!("{label} header is missing"));
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        panic!("{label} header must be a regular non-symlink file");
    }

    let canonical_path = std::fs::canonicalize(&candidate)
        .unwrap_or_else(|_| panic!("{label} header is unreadable"));
    if !canonical_path.starts_with(&canonical_root) {
        panic!("{label} header escapes its asserted source root");
    }
    let metadata = std::fs::metadata(&canonical_path)
        .unwrap_or_else(|_| panic!("{label} header metadata is unreadable"));
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_HEADER_BYTES {
        panic!("{label} header size is outside the accepted range");
    }
    let digest =
        sha256sum(&canonical_path).unwrap_or_else(|| panic!("{label} header hashing failed"));

    AuthenticatedHeader {
        canonical_root,
        canonical_path,
        digest,
    }
}

fn require_header_markers(header: &Path, markers: &[&str], label: &str) {
    let bytes =
        std::fs::read(header).unwrap_or_else(|_| panic!("{label} header became unreadable"));
    let text =
        std::str::from_utf8(&bytes).unwrap_or_else(|_| panic!("{label} header is not UTF-8"));
    if markers.iter().any(|marker| !text.contains(marker)) {
        panic!("{label} header identity does not match the supported API");
    }
}

fn emit_header_rerun(header: &AuthenticatedHeader) {
    println!("cargo:rerun-if-changed={}", header.canonical_path.display());
}

fn require_unchanged_header(
    asserted_root: &Path,
    name: &str,
    before: &AuthenticatedHeader,
    label: &str,
) {
    let after = authenticate_header(asserted_root, name, label);
    if after.canonical_root != before.canonical_root
        || after.canonical_path != before.canonical_path
        || after.digest != before.digest
    {
        panic!("{label} header changed while the ABI oracle was compiling");
    }
}

fn sha256sum(header: &Path) -> Option<String> {
    let output = Command::new("/usr/bin/sha256sum")
        .arg("--")
        .arg(header)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .output()
        .ok()?;
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
