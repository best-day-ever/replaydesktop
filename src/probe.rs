use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const REQUEST_SCHEMA: &str = "replaydesktop.host-probe-request.v1";
const RESPONSE_SCHEMA: &str = "replaydesktop.host-probe-response.v1";
const FIXTURE_SCHEMA: &str = "replaydesktop.host01-diagnostic-fixtures.v1";
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_STDERR_BYTES: usize = 8 * 1024;
const MAX_FIXTURE_BYTES: usize = 1024 * 1024;
const MAX_OBSERVATION_CODE_BYTES: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeId {
    HostFoundation,
    SelectedOutput,
    NvfbcCapture,
    NvencTuples,
}

impl ProbeId {
    pub const ALL: [Self; 4] = [
        Self::HostFoundation,
        Self::SelectedOutput,
        Self::NvfbcCapture,
        Self::NvencTuples,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostFoundation => "host-foundation",
            Self::SelectedOutput => "selected-output",
            Self::NvfbcCapture => "nvfbc-capture",
            Self::NvencTuples => "nvenc-tuples",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveObservationV1 {
    pub available: bool,
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_foundation: Option<crate::local_xorg::HostFoundationObservationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_output: Option<crate::output_mapping::OutputCollectorObservationV1>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProbeOutcome {
    pub probe: ProbeId,
    pub observation: Option<PrimitiveObservationV1>,
    pub failure: Option<ProbeFailure>,
}

impl ProbeOutcome {
    fn observed(probe: ProbeId, observation: PrimitiveObservationV1) -> Self {
        Self {
            probe,
            observation: Some(observation),
            failure: None,
        }
    }

    fn failed(probe: ProbeId, failure: ProbeFailure) -> Self {
        Self {
            probe,
            observation: None,
            failure: Some(failure),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProbeFailure {
    Spawn,
    Timeout,
    StdoutLimit,
    StderrLimit,
    StderrOutput,
    AbnormalExit,
    MalformedResponse,
    ProtocolMismatch,
    SecretLeak,
}

impl ProbeFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Spawn => "worker-spawn",
            Self::Timeout => "worker-timeout",
            Self::StdoutLimit => "worker-stdout-limit",
            Self::StderrLimit => "worker-stderr-limit",
            Self::StderrOutput => "worker-stderr",
            Self::AbnormalExit => "worker-abnormal-exit",
            Self::MalformedResponse => "worker-malformed-response",
            Self::ProtocolMismatch => "worker-protocol-mismatch",
            Self::SecretLeak => "worker-secret-leak",
        }
    }
}

pub trait ProbeBackend {
    fn observe(
        &self,
        probe: ProbeId,
        parent_nonce: &str,
        requested_output: Option<&crate::model::OutputNameV1>,
    ) -> ProbeOutcome;
}

#[derive(Debug, Clone)]
pub struct LiveProbeBackend {
    runner: BoundedProbeRunner,
}

impl LiveProbeBackend {
    pub fn new(runner: BoundedProbeRunner) -> Self {
        Self { runner }
    }
}

impl ProbeBackend for LiveProbeBackend {
    fn observe(
        &self,
        probe: ProbeId,
        parent_nonce: &str,
        requested_output: Option<&crate::model::OutputNameV1>,
    ) -> ProbeOutcome {
        let request = ProbeRequestV1 {
            schema: REQUEST_SCHEMA.to_owned(),
            nonce: parent_nonce.to_owned(),
            probe,
            source: ProbeSourceV1::Live,
            requested_output: requested_output.cloned(),
        };
        match self.runner.run(&request) {
            Ok(observation) => ProbeOutcome::observed(probe, observation),
            Err(failure) => ProbeOutcome::failed(probe, failure),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FixtureProbeBackend {
    runner: BoundedProbeRunner,
    fixture: PathBuf,
    case_id: String,
}

impl FixtureProbeBackend {
    pub fn new(
        runner: BoundedProbeRunner,
        fixture: impl Into<PathBuf>,
        case_id: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            fixture: fixture.into(),
            case_id: case_id.into(),
        }
    }
}

impl ProbeBackend for FixtureProbeBackend {
    fn observe(
        &self,
        probe: ProbeId,
        parent_nonce: &str,
        requested_output: Option<&crate::model::OutputNameV1>,
    ) -> ProbeOutcome {
        let request = ProbeRequestV1 {
            schema: REQUEST_SCHEMA.to_owned(),
            nonce: parent_nonce.to_owned(),
            probe,
            source: ProbeSourceV1::Fixture {
                path: self.fixture.clone(),
                case_id: self.case_id.clone(),
            },
            requested_output: requested_output.cloned(),
        };
        match self.runner.run(&request) {
            Ok(observation) => ProbeOutcome::observed(probe, observation),
            Err(failure) => ProbeOutcome::failed(probe, failure),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundedProbeRunner {
    executable: PathBuf,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl BoundedProbeRunner {
    pub fn current(timeout: Duration) -> Self {
        Self {
            executable: PathBuf::from("/proc/self/exe"),
            timeout,
            stdout_limit: MAX_RESPONSE_BYTES,
            stderr_limit: MAX_STDERR_BYTES,
        }
    }

    pub fn with_executable(executable: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            executable: executable.into(),
            timeout,
            stdout_limit: MAX_RESPONSE_BYTES,
            stderr_limit: MAX_STDERR_BYTES,
        }
    }

    fn run(&self, request: &ProbeRequestV1) -> Result<PrimitiveObservationV1, ProbeFailure> {
        let request_json =
            serde_json::to_string(request).map_err(|_| ProbeFailure::ProtocolMismatch)?;
        if request_json.len() > MAX_REQUEST_BYTES {
            return Err(ProbeFailure::ProtocolMismatch);
        }
        let deadline = Instant::now()
            .checked_add(self.timeout)
            .ok_or(ProbeFailure::Timeout)?;

        let mut command = Command::new(&self.executable);
        command
            .arg("__probe-worker")
            .arg("--request-json")
            .arg(request_json)
            .env_clear()
            .env("REPLAY_HOST_DOCTOR_WORKER", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if matches!(request.source, ProbeSourceV1::Live)
            && let Some(root) = std::env::var_os("REPLAY_NVML_SDK_ROOT")
        {
            command.env("REPLAY_NVML_SDK_ROOT", root);
        }
        let mut child = command.spawn().map_err(|_| ProbeFailure::Spawn)?;
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_and_reap(&mut child);
                return Err(ProbeFailure::Spawn);
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                terminate_and_reap(&mut child);
                return Err(ProbeFailure::Spawn);
            }
        };
        let stdout_reader = read_stream(stdout, self.stdout_limit);
        let stderr_reader = read_stream(stderr, self.stderr_limit);

        let status = loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    break child.wait().map_err(|_| ProbeFailure::AbnormalExit)?;
                }
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
                Ok(None) => {
                    terminate_and_reap(&mut child);
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(ProbeFailure::Timeout);
                }
                Err(_) => {
                    terminate_and_reap(&mut child);
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(ProbeFailure::AbnormalExit);
                }
            }
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| ProbeFailure::MalformedResponse)?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| ProbeFailure::MalformedResponse)?;
        self.validate_completed(request, status, stdout, stderr)
    }

    fn validate_completed(
        &self,
        request: &ProbeRequestV1,
        status: ExitStatus,
        stdout: StreamCapture,
        stderr: StreamCapture,
    ) -> Result<PrimitiveObservationV1, ProbeFailure> {
        if stdout.overflowed {
            return Err(ProbeFailure::StdoutLimit);
        }
        if stderr.overflowed {
            return Err(ProbeFailure::StderrLimit);
        }

        if let Ok(secret) = std::env::var("REPLAY_HOST_DOCTOR_SECRET_SENTINEL")
            && !secret.is_empty()
            && secret.len() <= MAX_OBSERVATION_CODE_BYTES
            && (contains_bytes(&stdout.bytes, secret.as_bytes())
                || contains_bytes(&stderr.bytes, secret.as_bytes()))
        {
            return Err(ProbeFailure::SecretLeak);
        }
        if !stderr.bytes.is_empty() {
            return Err(ProbeFailure::StderrOutput);
        }
        if !status.success() {
            return Err(ProbeFailure::AbnormalExit);
        }

        validate_duplicate_free_json(&stdout.bytes).map_err(|_| ProbeFailure::MalformedResponse)?;
        let response: ProbeResponseV1 =
            serde_json::from_slice(&stdout.bytes).map_err(|_| ProbeFailure::MalformedResponse)?;
        if response.schema != RESPONSE_SCHEMA
            || response.nonce != request.nonce
            || response.probe != request.probe
            || (response.observation.host_foundation.is_some()
                && request.probe != ProbeId::HostFoundation)
            || (response.observation.selected_output.is_some()
                && request.probe != ProbeId::SelectedOutput)
        {
            return Err(ProbeFailure::ProtocolMismatch);
        }
        if response.observation.code == "secret-inherited" {
            return Err(ProbeFailure::SecretLeak);
        }
        validate_observation(&response.observation).map_err(|_| ProbeFailure::ProtocolMismatch)?;
        Ok(response.observation)
    }
}

#[derive(Debug)]
struct StreamCapture {
    bytes: Vec<u8>,
    overflowed: bool,
}

fn terminate_and_reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_stream<R>(mut reader: R, limit: usize) -> thread::JoinHandle<StreamCapture>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::with_capacity(limit.min(4096));
        let mut overflowed = false;
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let remaining = limit.saturating_sub(bytes.len());
                    let retained = remaining.min(read);
                    bytes.extend_from_slice(&buffer[..retained]);
                    if retained < read {
                        overflowed = true;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    overflowed = true;
                    break;
                }
            }
        }
        StreamCapture { bytes, overflowed }
    })
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProbeRequestV1 {
    schema: String,
    nonce: String,
    probe: ProbeId,
    source: ProbeSourceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested_output: Option<crate::model::OutputNameV1>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ProbeSourceV1 {
    Live,
    Fixture { path: PathBuf, case_id: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProbeResponseV1 {
    schema: String,
    nonce: String,
    probe: ProbeId,
    observation: PrimitiveObservationV1,
}

pub fn worker_output(request_json: &str) -> WorkerOutput {
    match worker_output_inner(request_json) {
        Ok(output) => output,
        Err(_) => WorkerOutput {
            stdout: String::new(),
            stderr: "{\"schema\":\"replaydesktop.host-probe-worker-error.v1\",\"code\":\"worker-request-invalid\"}\n".to_owned(),
            exit_code: 70,
        },
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

fn worker_output_inner(request_json: &str) -> Result<WorkerOutput, WorkerRequestError> {
    if request_json.len() > MAX_REQUEST_BYTES {
        return Err(WorkerRequestError);
    }
    validate_duplicate_free_json(request_json.as_bytes()).map_err(|_| WorkerRequestError)?;
    let request: ProbeRequestV1 =
        serde_json::from_str(request_json).map_err(|_| WorkerRequestError)?;
    if request.schema != REQUEST_SCHEMA
        || !valid_nonce(&request.nonce)
        || std::env::var_os("REPLAY_HOST_DOCTOR_WORKER").as_deref() != Some("1".as_ref())
        || (request.requested_output.is_some() && request.probe != ProbeId::SelectedOutput)
        || request
            .requested_output
            .as_ref()
            .is_some_and(|name| !crate::output_mapping::validate_output_name_evidence(name))
    {
        return Err(WorkerRequestError);
    }

    match &request.source {
        ProbeSourceV1::Live if request.probe == ProbeId::HostFoundation => normal_worker_output(
            &request,
            PrimitiveObservationV1 {
                available: true,
                code: "host-foundation-observed".to_owned(),
                host_foundation: Some(crate::local_xorg::observe_live_host_foundation()),
                selected_output: None,
            },
        ),
        ProbeSourceV1::Live if request.probe == ProbeId::SelectedOutput => {
            let selected_output = crate::output_mapping::collect_live_output_topology(
                request.requested_output.clone(),
            );
            normal_worker_output(
                &request,
                PrimitiveObservationV1 {
                    available: selected_output.collection_failure.is_none(),
                    code: selected_output.collection_failure.map_or_else(
                        || "selected-output-observed".to_owned(),
                        |failure| failure.as_code().to_owned(),
                    ),
                    host_foundation: None,
                    selected_output: Some(selected_output),
                },
            )
        }
        ProbeSourceV1::Live => normal_worker_output(
            &request,
            PrimitiveObservationV1 {
                available: false,
                code: "native-probe-not-implemented".to_owned(),
                host_foundation: None,
                selected_output: None,
            },
        ),
        ProbeSourceV1::Fixture { path, case_id } => fixture_worker_output(&request, path, case_id),
    }
}

fn fixture_worker_output(
    request: &ProbeRequestV1,
    path: &Path,
    case_id: &str,
) -> Result<WorkerOutput, WorkerRequestError> {
    let bytes = read_bounded_file(path, MAX_FIXTURE_BYTES)?;
    validate_duplicate_free_json(&bytes).map_err(|_| WorkerRequestError)?;
    let schema = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("schema")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .ok_or(WorkerRequestError)?;
    if schema == "replaydesktop.host02-output-topologies.v1" {
        return fixture_output_mapping_worker(request, path, case_id);
    }
    let fixture: FixtureDocument =
        serde_json::from_slice(&bytes).map_err(|_| WorkerRequestError)?;
    if fixture.schema != FIXTURE_SCHEMA {
        return Err(WorkerRequestError);
    }
    let mut identifiers = HashSet::with_capacity(fixture.cases.len());
    for case in &fixture.cases {
        if !identifiers.insert(case.id.as_str()) {
            return Err(WorkerRequestError);
        }
    }
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(WorkerRequestError)?;
    match case.behavior {
        FixtureBehavior::Normal => {
            let mut seen = HashSet::new();
            for observation in &case.observations {
                if !seen.insert(observation.probe) {
                    return Err(WorkerRequestError);
                }
                if observation.host_foundation.is_some()
                    && observation.probe != ProbeId::HostFoundation
                {
                    return Err(WorkerRequestError);
                }
                validate_observation(&PrimitiveObservationV1 {
                    available: observation.available,
                    code: observation.code.clone(),
                    host_foundation: observation.host_foundation.clone(),
                    selected_output: observation.selected_output.clone(),
                })
                .map_err(|_| WorkerRequestError)?;
            }
            let observation = case
                .observations
                .iter()
                .find(|observation| observation.probe == request.probe)
                .map(|observation| PrimitiveObservationV1 {
                    available: observation.available,
                    code: observation.code.clone(),
                    host_foundation: observation.host_foundation.clone(),
                    selected_output: observation.selected_output.clone(),
                })
                .unwrap_or_else(|| PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-observation-missing".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                });
            normal_worker_output(request, observation)
        }
        FixtureBehavior::Timeout => {
            thread::sleep(Duration::from_secs(120));
            normal_worker_output(
                request,
                PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-timeout-returned".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                },
            )
        }
        FixtureBehavior::MalformedResponse => Ok(WorkerOutput {
            stdout: "{\"schema\":".to_owned(),
            stderr: String::new(),
            exit_code: 0,
        }),
        FixtureBehavior::OversizeOutput => Ok(WorkerOutput {
            stdout: "x".repeat(MAX_RESPONSE_BYTES * 4),
            stderr: String::new(),
            exit_code: 0,
        }),
        FixtureBehavior::OversizeStderr => Ok(WorkerOutput {
            stdout: String::new(),
            stderr: "x".repeat(MAX_STDERR_BYTES * 4),
            exit_code: 0,
        }),
        FixtureBehavior::ChildCrash => Ok(WorkerOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 70,
        }),
        FixtureBehavior::DuplicateResponse => {
            let normal = response_json(
                request,
                PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-duplicate-response".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                },
            )?;
            Ok(WorkerOutput {
                stdout: format!("{normal}\n{normal}\n"),
                stderr: String::new(),
                exit_code: 0,
            })
        }
        FixtureBehavior::InjectedAuthority => {
            let observation = case
                .observations
                .iter()
                .find(|observation| observation.probe == request.probe)
                .map(|observation| PrimitiveObservationV1 {
                    available: observation.available,
                    code: observation.code.clone(),
                    host_foundation: observation.host_foundation.clone(),
                    selected_output: observation.selected_output.clone(),
                })
                .unwrap_or_else(|| PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-observation-missing".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                });
            let response = ProbeResponseV1 {
                schema: RESPONSE_SCHEMA.to_owned(),
                nonce: request.nonce.clone(),
                probe: request.probe,
                observation,
            };
            let mut value = serde_json::to_value(response).map_err(|_| WorkerRequestError)?;
            let object = value.as_object_mut().ok_or(WorkerRequestError)?;
            let authority = case
                .authority
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .ok_or(WorkerRequestError)?;
            for (key, value) in authority {
                object.insert(key.clone(), value.clone());
            }
            Ok(WorkerOutput {
                stdout: format!(
                    "{}\n",
                    serde_json::to_string(&value).map_err(|_| WorkerRequestError)?
                ),
                stderr: String::new(),
                exit_code: 0,
            })
        }
        FixtureBehavior::SecretSentinel => {
            let inherited = std::env::var_os("REPLAY_HOST_DOCTOR_SECRET_SENTINEL").is_some();
            normal_worker_output(
                request,
                PrimitiveObservationV1 {
                    available: false,
                    code: if inherited {
                        "secret-inherited".to_owned()
                    } else {
                        "secret-not-inherited".to_owned()
                    },
                    host_foundation: None,
                    selected_output: None,
                },
            )
        }
        FixtureBehavior::StderrSentinel => {
            let sentinel = case.sentinel.as_deref().ok_or(WorkerRequestError)?;
            if sentinel.len() > MAX_OBSERVATION_CODE_BYTES || sentinel.as_bytes().contains(&0) {
                return Err(WorkerRequestError);
            }
            let stdout = response_json(
                request,
                PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-stderr-rejected".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                },
            )?;
            Ok(WorkerOutput {
                stdout: format!("{stdout}\n"),
                stderr: format!("{sentinel}\n"),
                exit_code: 0,
            })
        }
    }
}

fn fixture_output_mapping_worker(
    request: &ProbeRequestV1,
    path: &Path,
    case_id: &str,
) -> Result<WorkerOutput, WorkerRequestError> {
    if request.probe != ProbeId::SelectedOutput {
        return normal_worker_output(
            request,
            PrimitiveObservationV1 {
                available: false,
                code: "fixture-observation-missing".to_owned(),
                host_foundation: None,
                selected_output: None,
            },
        );
    }
    let requested = request
        .requested_output
        .as_ref()
        .and_then(|output| output.display.as_deref());
    let selected_output =
        crate::output_mapping::collect_fixture_output_topology(path, case_id, requested)
            .map_err(|_| WorkerRequestError)?;
    normal_worker_output(
        request,
        PrimitiveObservationV1 {
            available: true,
            code: "selected-output-fixture-observed".to_owned(),
            host_foundation: None,
            selected_output: Some(selected_output),
        },
    )
}

fn normal_worker_output(
    request: &ProbeRequestV1,
    observation: PrimitiveObservationV1,
) -> Result<WorkerOutput, WorkerRequestError> {
    validate_observation(&observation).map_err(|_| WorkerRequestError)?;
    Ok(WorkerOutput {
        stdout: format!("{}\n", response_json(request, observation)?),
        stderr: String::new(),
        exit_code: 0,
    })
}

fn response_json(
    request: &ProbeRequestV1,
    observation: PrimitiveObservationV1,
) -> Result<String, WorkerRequestError> {
    serde_json::to_string(&ProbeResponseV1 {
        schema: RESPONSE_SCHEMA.to_owned(),
        nonce: request.nonce.clone(),
        probe: request.probe,
        observation,
    })
    .map_err(|_| WorkerRequestError)
}

fn validate_observation(observation: &PrimitiveObservationV1) -> Result<(), ()> {
    if observation.code.is_empty()
        || observation.code.len() > MAX_OBSERVATION_CODE_BYTES
        || observation.code.as_bytes().contains(&0)
    {
        return Err(());
    }
    if let Some(foundation) = &observation.host_foundation {
        if !observation.available || observation.code != "host-foundation-observed" {
            return Err(());
        }
        if !foundation.is_valid() {
            return Err(());
        }
    }
    if let Some(selected_output) = &observation.selected_output
        && !crate::output_mapping::validate_output_collection_observation(selected_output)
    {
        return Err(());
    }
    if observation.host_foundation.is_some() && observation.selected_output.is_some() {
        return Err(());
    }
    Ok(())
}

fn valid_nonce(value: &str) -> bool {
    value.strip_prefix("nonce-").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

fn read_bounded_file(path: &Path, limit: usize) -> Result<Vec<u8>, WorkerRequestError> {
    let file = std::fs::File::open(path).map_err(|_| WorkerRequestError)?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| WorkerRequestError)?;
    if bytes.len() > limit {
        return Err(WorkerRequestError);
    }
    Ok(bytes)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureDocument {
    schema: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    id: String,
    behavior: FixtureBehavior,
    #[serde(default)]
    observations: Vec<FixtureObservation>,
    authority: Option<serde_json::Value>,
    sentinel: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FixtureBehavior {
    Normal,
    Timeout,
    MalformedResponse,
    OversizeOutput,
    OversizeStderr,
    ChildCrash,
    DuplicateResponse,
    InjectedAuthority,
    SecretSentinel,
    StderrSentinel,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureObservation {
    probe: ProbeId,
    available: bool,
    code: String,
    #[serde(default)]
    host_foundation: Option<crate::local_xorg::HostFoundationObservationV1>,
    #[serde(default)]
    selected_output: Option<crate::output_mapping::OutputCollectorObservationV1>,
}

#[derive(Debug)]
struct WorkerRequestError;

fn validate_duplicate_free_json(bytes: &[u8]) -> Result<(), serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    DuplicateFreeSeed.deserialize(&mut deserializer)?;
    deserializer.end()
}

struct DuplicateFreeSeed;

impl<'de> DeserializeSeed<'de> for DuplicateFreeSeed {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateFreeVisitor)
    }
}

struct DuplicateFreeVisitor;

impl<'de> Visitor<'de> for DuplicateFreeVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DuplicateFreeSeed.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(DuplicateFreeSeed)?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate object key"));
            }
            map.next_value_seed(DuplicateFreeSeed)?;
        }
        Ok(())
    }
}

pub fn collect_probes(
    backend: &dyn ProbeBackend,
    run_id: &str,
    requested_output: Option<&crate::model::OutputNameV1>,
) -> Vec<ProbeOutcome> {
    ProbeId::ALL
        .into_iter()
        .map(|probe| {
            let mut material = Vec::with_capacity(run_id.len() + probe.as_str().len() + 1);
            material.extend_from_slice(run_id.as_bytes());
            material.push(0);
            material.extend_from_slice(probe.as_str().as_bytes());
            let nonce = format!("nonce-{}", crate::sha256_bytes(&material));
            backend.observe(
                probe,
                &nonce,
                (probe == ProbeId::SelectedOutput)
                    .then_some(requested_output)
                    .flatten(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_probe_duplicate_keys_are_rejected() {
        let response = br#"{"schema":"x","schema":"y"}"#;
        assert!(validate_duplicate_free_json(response).is_err());
    }

    #[test]
    fn bounded_probe_observation_contract_has_no_authority_fields() {
        let injected = br#"{
            "schema":"replaydesktop.host-probe-response.v1",
            "nonce":"nonce-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "probe":"host-foundation",
            "observation":{"available":true,"code":"ok"},
            "status":"pass",
            "run_id":"child-owned"
        }"#;
        validate_duplicate_free_json(injected).expect("fixture is duplicate-free");
        assert!(serde_json::from_slice::<ProbeResponseV1>(injected).is_err());
    }
}
