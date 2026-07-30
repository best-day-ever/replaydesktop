use std::fmt;
use std::path::PathBuf;

use crate::model::{
    G0ExtensionStatusV1, HOST_FOUNDATION_EXTENSION_ID, NVENC_TUPLES_EXTENSION_ID,
    NVFBC_CAPTURE_EXTENSION_ID, SELECTED_OUTPUT_EXTENSION_ID,
};

pub const DEFAULT_PROBE_TIMEOUT_MS: u64 = 2_000;
pub const MAX_PROBE_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DoctorOptions {
    pub command: DoctorCommand,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DoctorCommand {
    Run {
        output: Option<String>,
        evidence: PathBuf,
        probe_timeout_ms: u64,
    },
    Diagnose {
        fixture: PathBuf,
        fixture_case: String,
        output: Option<String>,
        evidence: PathBuf,
        probe_timeout_ms: u64,
    },
    VerifyEvidence {
        evidence: PathBuf,
        run_id: Option<String>,
        required_statuses: RequiredExtensionStatuses,
        validate_extensions: Vec<String>,
    },
    ArchivePreReboot {
        evidence: PathBuf,
        archive_root: PathBuf,
    },
    ArchivePostRepair {
        evidence: PathBuf,
        archive_root: PathBuf,
    },
    VerifyArchive {
        index: PathBuf,
    },
    ProbeWorker {
        request_json: String,
    },
    Help,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(i32)]
pub enum DoctorExit {
    Success = 0,
    G0Fail = 2,
    Usage = 64,
    Internal = 70,
    Persistence = 74,
}

impl DoctorExit {
    pub const fn code(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DoctorOutput {
    pub exit: DoctorExit,
    pub stdout: String,
    pub stderr: String,
}

impl DoctorOutput {
    pub(crate) fn usage(error: &CliError) -> Self {
        Self {
            exit: DoctorExit::Usage,
            stdout: String::new(),
            stderr: format!("replay-host-doctor: {}\n{}", error, public_usage()),
        }
    }

    pub(crate) fn failure(exit: DoctorExit, code: &'static str) -> Self {
        let value = serde_json::json!({
            "schema": "replaydesktop.host-doctor-error.v1",
            "code": code,
        });
        Self {
            exit,
            stdout: String::new(),
            stderr: format!(
                "{}\n",
                serde_json::to_string(&value).expect("static error JSON must serialize")
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CliError {
    MissingCommand,
    UnknownCommand,
    MissingOption(&'static str),
    DuplicateOption(&'static str),
    UnknownOption,
    InvalidTimeout,
    InvalidOutput,
    InvalidStatus,
    InvalidExtension,
    InvalidRunId,
    InvalidFixtureCase,
    InvalidPath,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("a command is required"),
            Self::UnknownCommand => formatter.write_str("unknown command"),
            Self::MissingOption(option) => write!(formatter, "missing required option {option}"),
            Self::DuplicateOption(option) => {
                write!(formatter, "option {option} appears more than once")
            }
            Self::UnknownOption => formatter.write_str("unknown or misplaced option"),
            Self::InvalidTimeout => formatter
                .write_str("probe timeout must be an integer from 1 through 60000 milliseconds"),
            Self::InvalidOutput => formatter.write_str(
                "output must be 1 through 256 UTF-8 bytes and contain no control characters",
            ),
            Self::InvalidStatus => {
                formatter.write_str("required extension status must be pass, fail, or unproven")
            }
            Self::InvalidExtension => {
                formatter.write_str("extension validation requires one known G0 extension ID")
            }
            Self::InvalidRunId => formatter.write_str("run identity must be visible ASCII"),
            Self::InvalidFixtureCase => {
                formatter.write_str("fixture case must be a bounded ASCII identifier")
            }
            Self::InvalidPath => formatter.write_str("paths must be non-empty and contain no NUL"),
        }
    }
}

impl std::error::Error for CliError {}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct RequiredExtensionStatuses {
    pub host01: Option<G0ExtensionStatusV1>,
    pub host02: Option<G0ExtensionStatusV1>,
    pub host03: Option<G0ExtensionStatusV1>,
    pub host04: Option<G0ExtensionStatusV1>,
}

pub fn dispatch(argv: Vec<String>) -> DoctorOutput {
    match parse(argv) {
        Ok(options) => crate::execute_doctor(options),
        Err(error) => DoctorOutput::usage(&error),
    }
}

pub fn parse(argv: Vec<String>) -> Result<DoctorOptions, CliError> {
    let command = argv.get(1).ok_or(CliError::MissingCommand)?;
    let parsed = match command.as_str() {
        "--help" | "-h" | "help" => {
            if argv.len() != 2 {
                return Err(CliError::UnknownOption);
            }
            DoctorCommand::Help
        }
        "run" => parse_run(&argv[2..])?,
        "diagnose" => parse_diagnose(&argv[2..])?,
        "verify-evidence" => parse_verify(&argv[2..])?,
        "archive-pre-reboot" => parse_archive(&argv[2..], false)?,
        "archive-post-repair" => parse_archive(&argv[2..], true)?,
        "verify-archive" => parse_verify_archive(&argv[2..])?,
        "__probe-worker" => parse_worker(&argv[2..])?,
        _ => return Err(CliError::UnknownCommand),
    };
    Ok(DoctorOptions {
        command: parsed,
        argv,
    })
}

fn parse_run(arguments: &[String]) -> Result<DoctorCommand, CliError> {
    let mut output = None;
    let mut evidence = None;
    let mut timeout = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--output" => {
                let value = parse_output(next(arguments, &mut index)?)?;
                set_once(&mut output, "--output", value)?;
            }
            "--evidence" => {
                set_once(&mut evidence, "--evidence", next(arguments, &mut index)?)?;
            }
            "--probe-timeout-ms" => {
                let value = parse_timeout(next(arguments, &mut index)?)?;
                set_once(&mut timeout, "--probe-timeout-ms", value)?;
            }
            _ => return Err(CliError::UnknownOption),
        }
        index += 1;
    }
    Ok(DoctorCommand::Run {
        output,
        evidence: PathBuf::from(evidence.ok_or(CliError::MissingOption("--evidence"))?),
        probe_timeout_ms: timeout.unwrap_or(DEFAULT_PROBE_TIMEOUT_MS),
    })
}

fn parse_diagnose(arguments: &[String]) -> Result<DoctorCommand, CliError> {
    let mut fixture = None;
    let mut fixture_case = None;
    let mut output = None;
    let mut evidence = None;
    let mut timeout = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--fixture" => {
                set_once(&mut fixture, "--fixture", next(arguments, &mut index)?)?;
            }
            "--fixture-case" => {
                let value = next(arguments, &mut index)?;
                if !valid_fixture_case(&value) {
                    return Err(CliError::InvalidFixtureCase);
                }
                set_once(&mut fixture_case, "--fixture-case", value)?;
            }
            "--output" => {
                let value = parse_output(next(arguments, &mut index)?)?;
                set_once(&mut output, "--output", value)?;
            }
            "--evidence" => {
                set_once(&mut evidence, "--evidence", next(arguments, &mut index)?)?;
            }
            "--probe-timeout-ms" => {
                let value = parse_timeout(next(arguments, &mut index)?)?;
                set_once(&mut timeout, "--probe-timeout-ms", value)?;
            }
            _ => return Err(CliError::UnknownOption),
        }
        index += 1;
    }
    Ok(DoctorCommand::Diagnose {
        fixture: PathBuf::from(fixture.ok_or(CliError::MissingOption("--fixture"))?),
        fixture_case: fixture_case.unwrap_or_else(|| "positive".to_owned()),
        output,
        evidence: PathBuf::from(evidence.ok_or(CliError::MissingOption("--evidence"))?),
        probe_timeout_ms: timeout.unwrap_or(DEFAULT_PROBE_TIMEOUT_MS),
    })
}

fn parse_verify(arguments: &[String]) -> Result<DoctorCommand, CliError> {
    let mut evidence = None;
    let mut run_id = None;
    let mut required_statuses = RequiredExtensionStatuses::default();
    let mut validate_extensions = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--evidence" => {
                set_once(&mut evidence, "--evidence", next(arguments, &mut index)?)?;
            }
            "--run-id" => {
                let value = next(arguments, &mut index)?;
                if value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|byte| byte.is_ascii_graphic())
                {
                    return Err(CliError::InvalidRunId);
                }
                set_once(&mut run_id, "--run-id", value)?;
            }
            "--require-host01" => {
                let value = parse_status(next(arguments, &mut index)?)?;
                set_once(&mut required_statuses.host01, "--require-host01", value)?;
            }
            "--require-host02" => {
                let value = parse_status(next(arguments, &mut index)?)?;
                set_once(&mut required_statuses.host02, "--require-host02", value)?;
            }
            "--require-host03" => {
                let value = parse_status(next(arguments, &mut index)?)?;
                set_once(&mut required_statuses.host03, "--require-host03", value)?;
            }
            "--require-host04" => {
                let value = parse_status(next(arguments, &mut index)?)?;
                set_once(&mut required_statuses.host04, "--require-host04", value)?;
            }
            "--validate-extension" => {
                let value = next(arguments, &mut index)?;
                if !known_extension(&value) {
                    return Err(CliError::InvalidExtension);
                }
                if validate_extensions
                    .iter()
                    .any(|existing| existing == &value)
                {
                    return Err(CliError::DuplicateOption("--validate-extension"));
                }
                validate_extensions.push(value);
            }
            _ => return Err(CliError::UnknownOption),
        }
        index += 1;
    }
    Ok(DoctorCommand::VerifyEvidence {
        evidence: PathBuf::from(evidence.ok_or(CliError::MissingOption("--evidence"))?),
        run_id,
        required_statuses,
        validate_extensions,
    })
}

fn parse_archive(arguments: &[String], post_repair: bool) -> Result<DoctorCommand, CliError> {
    let mut evidence = None;
    let mut archive_root = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--evidence" => {
                let value = parse_path(next(arguments, &mut index)?)?;
                set_once(&mut evidence, "--evidence", value)?;
            }
            "--archive-root" => {
                let value = parse_path(next(arguments, &mut index)?)?;
                set_once(&mut archive_root, "--archive-root", value)?;
            }
            _ => return Err(CliError::UnknownOption),
        }
        index += 1;
    }
    let evidence = PathBuf::from(evidence.ok_or(CliError::MissingOption("--evidence"))?);
    let archive_root =
        PathBuf::from(archive_root.ok_or(CliError::MissingOption("--archive-root"))?);
    Ok(if post_repair {
        DoctorCommand::ArchivePostRepair {
            evidence,
            archive_root,
        }
    } else {
        DoctorCommand::ArchivePreReboot {
            evidence,
            archive_root,
        }
    })
}

fn parse_verify_archive(arguments: &[String]) -> Result<DoctorCommand, CliError> {
    if arguments.len() != 2 || arguments[0] != "--index" {
        return Err(CliError::UnknownOption);
    }
    Ok(DoctorCommand::VerifyArchive {
        index: PathBuf::from(parse_path(arguments[1].clone())?),
    })
}

fn parse_worker(arguments: &[String]) -> Result<DoctorCommand, CliError> {
    if arguments.len() != 2 || arguments[0] != "--request-json" {
        return Err(CliError::UnknownOption);
    }
    Ok(DoctorCommand::ProbeWorker {
        request_json: arguments[1].clone(),
    })
}

fn next(arguments: &[String], index: &mut usize) -> Result<String, CliError> {
    *index += 1;
    arguments
        .get(*index)
        .cloned()
        .ok_or(CliError::UnknownOption)
}

fn set_once<T>(slot: &mut Option<T>, option: &'static str, value: T) -> Result<(), CliError> {
    if slot.replace(value).is_some() {
        return Err(CliError::DuplicateOption(option));
    }
    Ok(())
}

fn parse_timeout(value: String) -> Result<u64, CliError> {
    let timeout = value.parse::<u64>().map_err(|_| CliError::InvalidTimeout)?;
    if !(1..=MAX_PROBE_TIMEOUT_MS).contains(&timeout) {
        return Err(CliError::InvalidTimeout);
    }
    Ok(timeout)
}

fn parse_output(value: String) -> Result<String, CliError> {
    crate::output_mapping::output_name_from_cli(&value)
        .map(|_| value)
        .ok_or(CliError::InvalidOutput)
}

fn parse_status(value: String) -> Result<G0ExtensionStatusV1, CliError> {
    match value.as_str() {
        "pass" => Ok(G0ExtensionStatusV1::Pass),
        "fail" => Ok(G0ExtensionStatusV1::Fail),
        "unproven" => Ok(G0ExtensionStatusV1::Unproven),
        _ => Err(CliError::InvalidStatus),
    }
}

fn known_extension(value: &str) -> bool {
    matches!(
        value,
        HOST_FOUNDATION_EXTENSION_ID
            | SELECTED_OUTPUT_EXTENSION_ID
            | NVFBC_CAPTURE_EXTENSION_ID
            | NVENC_TUPLES_EXTENSION_ID
    )
}

fn parse_path(value: String) -> Result<String, CliError> {
    if value.is_empty() || value.as_bytes().contains(&0) {
        return Err(CliError::InvalidPath);
    }
    Ok(value)
}

fn valid_fixture_case(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

pub fn public_usage() -> &'static str {
    "usage:\n  replay-host-doctor run [--output <XRANDR_NAME>] --evidence <PATH> [--probe-timeout-ms <N>]\n  replay-host-doctor diagnose --fixture <PATH> [--fixture-case <ID>] [--output <XRANDR_NAME>] --evidence <PATH> [--probe-timeout-ms <N>]\n  replay-host-doctor verify-evidence --evidence <PATH> [--run-id <ID>] [--require-host01 <STATUS>] [--require-host02 <STATUS>] [--require-host03 <STATUS>] [--require-host04 <STATUS>] [--validate-extension <ID>]\n  replay-host-doctor archive-pre-reboot --evidence <PATH> --archive-root <PATH>\n  replay-host-doctor archive-post-repair --evidence <PATH> --archive-root <PATH>\n  replay-host-doctor verify-archive --index <PATH>\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_exit_duplicate_and_unknown_options_are_usage_errors() {
        let duplicate = dispatch(vec![
            "replay-host-doctor".to_owned(),
            "run".to_owned(),
            "--evidence".to_owned(),
            "one".to_owned(),
            "--evidence".to_owned(),
            "two".to_owned(),
        ]);
        assert_eq!(duplicate.exit, DoctorExit::Usage);

        let unknown = dispatch(vec![
            "replay-host-doctor".to_owned(),
            "run".to_owned(),
            "--output".to_owned(),
            "anything".to_owned(),
        ]);
        assert_eq!(unknown.exit, DoctorExit::Usage);
    }

    #[test]
    fn cli_exit_timeout_is_strictly_bounded() {
        for value in ["0", "60001", "-1", "1.5", "slow"] {
            let output = dispatch(vec![
                "replay-host-doctor".to_owned(),
                "run".to_owned(),
                "--evidence".to_owned(),
                "unused".to_owned(),
                "--probe-timeout-ms".to_owned(),
                value.to_owned(),
            ]);
            assert_eq!(output.exit, DoctorExit::Usage);
        }
    }

    #[test]
    fn host02_cli_output_is_optional_scalar_and_never_defaulted() {
        let omitted = parse(vec![
            "replay-host-doctor".to_owned(),
            "run".to_owned(),
            "--evidence".to_owned(),
            "evidence.json".to_owned(),
        ])
        .expect("omitting --output must enter discovery mode");
        assert!(matches!(
            omitted.command,
            DoctorCommand::Run { output: None, .. }
        ));

        let selected = parse(vec![
            "replay-host-doctor".to_owned(),
            "run".to_owned(),
            "--output".to_owned(),
            "DP-0".to_owned(),
            "--evidence".to_owned(),
            "evidence.json".to_owned(),
        ])
        .expect("one exact --output must be accepted");
        assert!(matches!(
            selected.command,
            DoctorCommand::Run {
                output: Some(ref output),
                ..
            } if output == "DP-0"
        ));

        let duplicate = dispatch(vec![
            "replay-host-doctor".to_owned(),
            "run".to_owned(),
            "--output".to_owned(),
            "DP-0".to_owned(),
            "--output".to_owned(),
            "DP-1".to_owned(),
            "--evidence".to_owned(),
            "evidence.json".to_owned(),
        ]);
        assert_eq!(duplicate.exit, DoctorExit::Usage);
        assert!(duplicate.stderr.contains("--output"));
    }
}
