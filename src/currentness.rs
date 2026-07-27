use crate::digest::{Sha256DigestV1, sha256_bytes, sha256_file};
use crate::model::G0EvidenceEnvelopeV1;
use std::fmt;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_AGE_NS: u64 = 5 * 60 * 1_000_000_000;
const DEFAULT_MAX_FUTURE_SKEW_NS: u64 = 5 * 1_000_000_000;
const DEFAULT_MAX_RUN_DURATION_NS: u64 = 5 * 60 * 1_000_000_000;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RunIdentityV1 {
    run_id: String,
    boot_id: String,
    session_id: String,
    argv: Vec<String>,
    executable_sha256: Sha256DigestV1,
    wall_started_unix_ns: u64,
    monotonic_started_ns: u64,
}

impl RunIdentityV1 {
    pub fn capture(argv: Vec<String>) -> Result<Self, CurrentnessError> {
        let boot_id = current_boot_id()?;
        let session_id = current_session_id()?;
        let executable_sha256 = current_executable_digest()?;
        let wall_started_unix_ns = wall_unix_ns()?;
        let monotonic_started_ns = monotonic_ns()?;

        let mut identity_material = Vec::new();
        identity_material.extend_from_slice(boot_id.as_bytes());
        identity_material.push(0);
        identity_material.extend_from_slice(session_id.as_bytes());
        identity_material.push(0);
        identity_material.extend_from_slice(&std::process::id().to_be_bytes());
        identity_material.extend_from_slice(&wall_started_unix_ns.to_be_bytes());
        identity_material.extend_from_slice(&monotonic_started_ns.to_be_bytes());
        identity_material.extend_from_slice(executable_sha256.as_bytes());
        for argument in &argv {
            identity_material.push(0);
            identity_material.extend_from_slice(argument.as_bytes());
        }
        let run_id = format!("run-{}", sha256_bytes(&identity_material));

        Ok(Self {
            run_id,
            boot_id,
            session_id,
            argv,
            executable_sha256,
            wall_started_unix_ns,
            monotonic_started_ns,
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn boot_id(&self) -> &str {
        &self.boot_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub const fn executable_sha256(&self) -> Sha256DigestV1 {
        self.executable_sha256
    }

    pub const fn wall_started_unix_ns(&self) -> u64 {
        self.wall_started_unix_ns
    }

    pub const fn monotonic_started_ns(&self) -> u64 {
        self.monotonic_started_ns
    }

    pub fn finish_times(&self) -> Result<(u64, u64), CurrentnessError> {
        let wall_finished = wall_unix_ns()?;
        let monotonic_finished = monotonic_ns()?;
        if wall_finished < self.wall_started_unix_ns
            || monotonic_finished < self.monotonic_started_ns
        {
            return Err(CurrentnessError::ClockRegression);
        }
        Ok((wall_finished, monotonic_finished))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CurrentnessPolicy {
    pub max_age_ns: u64,
    pub max_future_skew_ns: u64,
    pub max_run_duration_ns: u64,
}

impl Default for CurrentnessPolicy {
    fn default() -> Self {
        Self {
            max_age_ns: DEFAULT_MAX_AGE_NS,
            max_future_skew_ns: DEFAULT_MAX_FUTURE_SKEW_NS,
            max_run_duration_ns: DEFAULT_MAX_RUN_DURATION_NS,
        }
    }
}

#[derive(Debug)]
pub enum CurrentnessError {
    Io {
        fact: &'static str,
        source: io::Error,
    },
    InvalidFact {
        fact: &'static str,
    },
    RunIdMismatch,
    BootMismatch,
    SessionMismatch,
    ExecutableMismatch,
    EvidenceExpired,
    EvidenceFromFuture,
    RunDurationExceeded,
    ClockRegression,
}

impl fmt::Display for CurrentnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { fact, .. } => write!(formatter, "cannot observe current {fact}"),
            Self::InvalidFact { fact } => write!(formatter, "current {fact} is malformed"),
            Self::RunIdMismatch => formatter.write_str("evidence run identity does not match"),
            Self::BootMismatch => formatter.write_str("evidence boot identity is not current"),
            Self::SessionMismatch => {
                formatter.write_str("evidence process-session identity is not current")
            }
            Self::ExecutableMismatch => {
                formatter.write_str("evidence executable identity is not current")
            }
            Self::EvidenceExpired => {
                formatter.write_str("evidence is outside the currentness window")
            }
            Self::EvidenceFromFuture => {
                formatter.write_str("evidence timestamp exceeds allowed future skew")
            }
            Self::RunDurationExceeded => {
                formatter.write_str("evidence run exceeded its duration bound")
            }
            Self::ClockRegression => formatter.write_str("an observed clock moved backwards"),
        }
    }
}

impl std::error::Error for CurrentnessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn verify_current_run(
    envelope: &G0EvidenceEnvelopeV1,
    expected_run_id: &str,
    policy: &CurrentnessPolicy,
) -> Result<(), CurrentnessError> {
    let base = &envelope.base;
    if base.run_id != expected_run_id {
        return Err(CurrentnessError::RunIdMismatch);
    }
    if base.boot_id != current_boot_id()? {
        return Err(CurrentnessError::BootMismatch);
    }
    if base.session_id != current_session_id()? {
        return Err(CurrentnessError::SessionMismatch);
    }
    if base.executable_sha256 != current_executable_digest()? {
        return Err(CurrentnessError::ExecutableMismatch);
    }

    let wall_duration = base
        .wall_finished_unix_ns
        .checked_sub(base.wall_started_unix_ns)
        .ok_or(CurrentnessError::ClockRegression)?;
    let monotonic_duration = base
        .monotonic_finished_ns
        .checked_sub(base.monotonic_started_ns)
        .ok_or(CurrentnessError::ClockRegression)?;
    if wall_duration > policy.max_run_duration_ns || monotonic_duration > policy.max_run_duration_ns
    {
        return Err(CurrentnessError::RunDurationExceeded);
    }

    let wall_now = wall_unix_ns()?;
    let monotonic_now = monotonic_ns()?;
    if base.wall_finished_unix_ns > wall_now.saturating_add(policy.max_future_skew_ns)
        || base.monotonic_finished_ns > monotonic_now
    {
        return Err(CurrentnessError::EvidenceFromFuture);
    }
    if wall_now.saturating_sub(base.wall_finished_unix_ns) > policy.max_age_ns
        || monotonic_now.saturating_sub(base.monotonic_finished_ns) > policy.max_age_ns
    {
        return Err(CurrentnessError::EvidenceExpired);
    }
    Ok(())
}

fn current_boot_id() -> Result<String, CurrentnessError> {
    let value = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|source| {
        CurrentnessError::Io {
            fact: "boot identity",
            source,
        }
    })?;
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(CurrentnessError::InvalidFact {
            fact: "boot identity",
        });
    }
    Ok(value.to_owned())
}

fn current_session_id() -> Result<String, CurrentnessError> {
    let session = rustix::process::getsid(None).map_err(|error| CurrentnessError::Io {
        fact: "process-session identity",
        source: io::Error::from_raw_os_error(error.raw_os_error()),
    })?;
    Ok(format!("sid-{}", session.as_raw_nonzero().get()))
}

fn current_executable_digest() -> Result<Sha256DigestV1, CurrentnessError> {
    sha256_file("/proc/self/exe").map_err(|source| CurrentnessError::Io {
        fact: "executable identity",
        source,
    })
}

fn wall_unix_ns() -> Result<u64, CurrentnessError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CurrentnessError::ClockRegression)?;
    u64::try_from(elapsed.as_nanos())
        .map_err(|_| CurrentnessError::InvalidFact { fact: "wall clock" })
}

fn monotonic_ns() -> Result<u64, CurrentnessError> {
    let uptime =
        std::fs::read_to_string("/proc/uptime").map_err(|source| CurrentnessError::Io {
            fact: "monotonic clock",
            source,
        })?;
    let token = uptime
        .split_ascii_whitespace()
        .next()
        .ok_or(CurrentnessError::InvalidFact {
            fact: "monotonic clock",
        })?;
    parse_decimal_seconds_ns(token).ok_or(CurrentnessError::InvalidFact {
        fact: "monotonic clock",
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currentness_decimal_uptime_parser_is_exact() {
        assert_eq!(parse_decimal_seconds_ns("12.34"), Some(12_340_000_000));
        assert_eq!(parse_decimal_seconds_ns("0.000000001"), Some(1));
        assert_eq!(parse_decimal_seconds_ns("7"), Some(7_000_000_000));
        assert_eq!(parse_decimal_seconds_ns("1.0000000000"), None);
        assert_eq!(parse_decimal_seconds_ns("bad"), None);
    }

    #[test]
    fn currentness_default_duration_contains_four_maximum_probe_deadlines() {
        let policy = CurrentnessPolicy::default();
        assert!(policy.max_run_duration_ns >= 4 * 60 * 1_000_000_000);
        assert!(policy.max_run_duration_ns <= policy.max_age_ns);
    }
}
