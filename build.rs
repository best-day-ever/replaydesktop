use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

const NVML_SOURCE_ROOT_ENV: &str = "REPLAY_NVML_SDK_ROOT";
const NVML_HEADER_NAME: &str = "nvml.h";
const SOURCE_IDENTITY: &str = "nvidia-nvml-api-13";
const MAX_HEADER_BYTES: u64 = 4 * 1024 * 1024;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(replay_nvml_source)");
    println!("cargo:rerun-if-env-changed={NVML_SOURCE_ROOT_ENV}");
    println!("cargo:rerun-if-changed=native/nvml_abi_oracle.c");

    let Some(root) = std::env::var_os(NVML_SOURCE_ROOT_ENV).map(PathBuf::from) else {
        return;
    };
    let Some(header) = authenticated_header_path(&root) else {
        return;
    };
    let Some(digest) = sha256sum(&header) else {
        return;
    };
    if !header_has_expected_public_identity(&header) {
        return;
    }

    let Some(out_dir) = std::env::var_os("OUT_DIR").map(PathBuf::from) else {
        return;
    };
    if !compile_oracle(&root, &out_dir) {
        return;
    }

    println!("cargo:rustc-cfg=replay_nvml_source");
    println!("cargo:rustc-env=REPLAY_NVML_SOURCE_IDENTITY={SOURCE_IDENTITY}");
    println!("cargo:rustc-env=REPLAY_NVML_SOURCE_SHA256={digest}");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=replay_nvml_abi_oracle");
}

fn authenticated_header_path(root: &Path) -> Option<PathBuf> {
    if !root.is_absolute() {
        return None;
    }
    let header = root.join(NVML_HEADER_NAME);
    let metadata = std::fs::metadata(&header).ok()?;
    (metadata.is_file() && metadata.len() > 0 && metadata.len() <= MAX_HEADER_BYTES)
        .then_some(header)
}

fn header_has_expected_public_identity(header: &Path) -> bool {
    let Ok(bytes) = std::fs::read(header) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    text.lines().any(|line| {
        line.trim()
            == "#define NVML_API_VERSION            13      //!< NVML API version identifier."
    }) && text.contains("Copyright 1993-2026 NVIDIA Corporation.")
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

fn compile_oracle(include_root: &Path, out_dir: &Path) -> bool {
    let object = out_dir.join("nvml_abi_oracle.o");
    let archive = out_dir.join("libreplay_nvml_abi_oracle.a");
    let compile = Command::new("/usr/bin/cc")
        .args([
            OsStr::new("-std=c11"),
            OsStr::new("-Wall"),
            OsStr::new("-Wextra"),
            OsStr::new("-Werror"),
            OsStr::new("-fPIC"),
            OsStr::new("-c"),
            OsStr::new("native/nvml_abi_oracle.c"),
            OsStr::new("-o"),
        ])
        .arg(&object)
        .arg("-I")
        .arg(include_root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .status()
        .is_ok_and(|status| status.success());
    if !compile {
        return false;
    }
    Command::new("/usr/bin/ar")
        .args([OsStr::new("crs")])
        .arg(&archive)
        .arg(&object)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .status()
        .is_ok_and(|status| status.success())
}
