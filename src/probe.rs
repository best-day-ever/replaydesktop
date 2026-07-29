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
const HOST03_FIXTURE_SCHEMA: &str = "replaydesktop.host03-nvfbc-capture-fixtures.v1";
const HOST04_FIXTURE_SCHEMA: &str = "replaydesktop.host04-nvenc-fixtures.v1";
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_STDERR_BYTES: usize = 8 * 1024;
const MAX_FIXTURE_BYTES: usize = 1024 * 1024;
const MAX_OBSERVATION_CODE_BYTES: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(2);
const MIN_LIVE_NVFBC_WORKER_TIMEOUT: Duration = Duration::from_secs(5);

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<crate::model::CapturePrimitiveObservationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nvenc: Option<crate::model::NvencTuplesEvidenceV1>,
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
        if probe == ProbeId::NvfbcCapture && !fixture_uses_host03_contract(&self.fixture) {
            return ProbeOutcome::observed(probe, unavailable_capture_observation());
        }
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

fn unavailable_capture_observation() -> PrimitiveObservationV1 {
    let provider = crate::native_nvfbc::LiveUnavailableCaptureProvider;
    PrimitiveObservationV1 {
        available: false,
        code: "nvfbc-source-unavailable".to_owned(),
        host_foundation: None,
        selected_output: None,
        capture: Some(crate::native_nvfbc::CaptureProvider::observe(&provider)),
        nvenc: None,
    }
}

fn fixture_uses_host03_contract(path: &Path) -> bool {
    read_bounded_file(path, MAX_FIXTURE_BYTES)
        .ok()
        .filter(|bytes| validate_duplicate_free_json(bytes).is_ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| {
            value
                .get("schema")
                .and_then(serde_json::Value::as_str)
                .map(|schema| schema == HOST03_FIXTURE_SCHEMA)
        })
        .unwrap_or(false)
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
            .checked_add(self.effective_timeout(request))
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
        if matches!(request.source, ProbeSourceV1::Live)
            && request.probe == ProbeId::NvfbcCapture
            && let (Some(nvfbc_root), Some(cuda_root)) = (
                std::env::var_os("REPLAY_NVFBC_SDK_ROOT"),
                std::env::var_os("REPLAY_CUDA_SDK_ROOT"),
            )
            && Path::new(&nvfbc_root).is_absolute()
            && Path::new(&cuda_root).is_absolute()
        {
            command
                .env("REPLAY_NVFBC_SDK_ROOT", nvfbc_root)
                .env("REPLAY_CUDA_SDK_ROOT", cuda_root);
        }
        if matches!(request.source, ProbeSourceV1::Live)
            && matches!(
                request.probe,
                ProbeId::HostFoundation | ProbeId::SelectedOutput | ProbeId::NvfbcCapture
            )
        {
            // The worker starts from an empty environment. Forward only the
            // bounded local display/auth locators needed to open the socket;
            // the worker still proves the session and Xorg peer independently.
            if let Some(display) = std::env::var_os("DISPLAY")
                && crate::local_xorg::valid_environment_display_locator(&display)
            {
                command.env("DISPLAY", display);
            }
            if let Some(xauthority) = std::env::var_os("XAUTHORITY")
                && crate::local_xorg::valid_environment_xauthority_locator(&xauthority)
            {
                command.env("XAUTHORITY", xauthority);
            }
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

    fn effective_timeout(&self, request: &ProbeRequestV1) -> Duration {
        if matches!(request.source, ProbeSourceV1::Live) && request.probe == ProbeId::NvfbcCapture {
            self.timeout.max(MIN_LIVE_NVFBC_WORKER_TIMEOUT)
        } else {
            self.timeout
        }
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
            || (response.observation.capture.is_some() && request.probe != ProbeId::NvfbcCapture)
            || (response.observation.nvenc.is_some() && request.probe != ProbeId::NvencTuples)
        {
            return Err(ProbeFailure::ProtocolMismatch);
        }
        if response.observation.code == "secret-inherited" {
            return Err(ProbeFailure::SecretLeak);
        }
        validate_observation_for_request(request, &response.observation)
            .map_err(|_| ProbeFailure::ProtocolMismatch)?;
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
        || (request.requested_output.is_some()
            && !matches!(
                request.probe,
                ProbeId::SelectedOutput | ProbeId::NvfbcCapture
            ))
        || (request.probe == ProbeId::NvfbcCapture && request.requested_output.is_none())
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
                capture: None,
                nvenc: None,
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
                    capture: None,
                    nvenc: None,
                },
            )
        }
        ProbeSourceV1::Live if request.probe == ProbeId::NvfbcCapture => {
            let requested_output = request
                .requested_output
                .as_ref()
                .ok_or(WorkerRequestError)?;
            let capture = crate::native_nvfbc::observe_live_selected_capture(requested_output);
            normal_worker_output(
                &request,
                PrimitiveObservationV1 {
                    available: capture.failure.is_none(),
                    code: if capture.failure.is_none() {
                        "nvfbc-live-one-frame-observed".to_owned()
                    } else {
                        "nvfbc-live-one-frame-rejected".to_owned()
                    },
                    host_foundation: None,
                    selected_output: None,
                    capture: Some(capture),
                    nvenc: None,
                },
            )
        }
        ProbeSourceV1::Live if request.probe == ProbeId::NvencTuples => {
            let mut provider = crate::native_nvenc::LiveUnavailableNvencProvider::new(
                crate::model::NvencApiVersionV1::new(13, 1),
            );
            let nvenc = crate::native_nvenc::evaluate_nvenc_policy(
                crate::model::NvencGpuGenerationV1::Unknown,
                &mut provider,
            );
            normal_worker_output(
                &request,
                PrimitiveObservationV1 {
                    available: false,
                    code: "nvenc-live-provider-unavailable".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                    capture: None,
                    nvenc: Some(nvenc),
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
                capture: None,
                nvenc: None,
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
    if schema == HOST03_FIXTURE_SCHEMA {
        return fixture_nvfbc_worker(request, path, case_id);
    }
    if schema == HOST04_FIXTURE_SCHEMA {
        return fixture_nvenc_worker(request, path, case_id);
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
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
                })
                .unwrap_or_else(|| PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-observation-missing".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
                })
                .unwrap_or_else(|| PrimitiveObservationV1 {
                    available: false,
                    code: "fixture-observation-missing".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
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
                    capture: None,
                    nvenc: None,
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
                capture: None,
                nvenc: None,
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
            capture: None,
            nvenc: None,
        },
    )
}

fn fixture_nvfbc_worker(
    request: &ProbeRequestV1,
    path: &Path,
    case_id: &str,
) -> Result<WorkerOutput, WorkerRequestError> {
    let bytes = read_bounded_file(path, MAX_FIXTURE_BYTES)?;
    let fixture: Host03FixtureDocument =
        serde_json::from_slice(&bytes).map_err(|_| WorkerRequestError)?;
    if fixture.schema != HOST03_FIXTURE_SCHEMA
        || fixture.selected_output_fixture.contains('/')
        || fixture.selected_output_fixture.contains('\\')
        || fixture.selected_output_fixture.is_empty()
        || fixture.selected_output_case.is_empty()
    {
        return Err(WorkerRequestError);
    }
    let mut identifiers = HashSet::with_capacity(fixture.cases.len());
    if fixture
        .cases
        .iter()
        .any(|case| case.id.is_empty() || !identifiers.insert(case.id.as_str()))
    {
        return Err(WorkerRequestError);
    }
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(WorkerRequestError)?;

    if request.probe == ProbeId::SelectedOutput {
        let output_fixture = path
            .parent()
            .ok_or(WorkerRequestError)?
            .join(&fixture.selected_output_fixture);
        let requested = request
            .requested_output
            .as_ref()
            .and_then(|output| output.display.as_deref());
        let selected_output = crate::output_mapping::collect_fixture_output_topology(
            &output_fixture,
            &fixture.selected_output_case,
            requested,
        )
        .map_err(|_| WorkerRequestError)?;
        return normal_worker_output(
            request,
            PrimitiveObservationV1 {
                available: selected_output.collection_failure.is_none(),
                code: "selected-output-fixture-observed".to_owned(),
                host_foundation: None,
                selected_output: Some(selected_output),
                capture: None,
                nvenc: None,
            },
        );
    }
    if request.probe != ProbeId::NvfbcCapture {
        return normal_worker_output(
            request,
            PrimitiveObservationV1 {
                available: false,
                code: "fixture-observation-missing".to_owned(),
                host_foundation: None,
                selected_output: None,
                capture: None,
                nvenc: None,
            },
        );
    }

    match case.behavior {
        Host03FixtureBehavior::Normal => {
            let mut capture = fixture.base_capture.clone();
            overlay_fixture_json(&mut capture, &case.capture_patch);
            let capture: crate::model::CapturePrimitiveObservationV1 =
                serde_json::from_value(capture).map_err(|_| WorkerRequestError)?;
            let provider = crate::native_nvfbc::FixtureCaptureProvider::new(capture);
            let capture = crate::native_nvfbc::CaptureProvider::observe(&provider);
            normal_worker_output(
                request,
                PrimitiveObservationV1 {
                    available: capture.failure.is_none(),
                    code: if capture.failure.is_none() {
                        "nvfbc-fixture-observed".to_owned()
                    } else {
                        "nvfbc-fixture-terminal".to_owned()
                    },
                    host_foundation: None,
                    selected_output: None,
                    capture: Some(capture),
                    nvenc: None,
                },
            )
        }
        Host03FixtureBehavior::Timeout => {
            thread::sleep(Duration::from_secs(120));
            Err(WorkerRequestError)
        }
        Host03FixtureBehavior::ChildCrash => Ok(WorkerOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 70,
        }),
        Host03FixtureBehavior::MalformedResponse => Ok(WorkerOutput {
            stdout: "{\"schema\":".to_owned(),
            stderr: String::new(),
            exit_code: 0,
        }),
    }
}

fn fixture_nvenc_worker(
    request: &ProbeRequestV1,
    path: &Path,
    case_id: &str,
) -> Result<WorkerOutput, WorkerRequestError> {
    let bytes = read_bounded_file(path, MAX_FIXTURE_BYTES)?;
    let fixture: Host04FixtureDocument =
        serde_json::from_slice(&bytes).map_err(|_| WorkerRequestError)?;
    if fixture.schema != HOST04_FIXTURE_SCHEMA || fixture.provenance != "diagnostic" {
        return Err(WorkerRequestError);
    }
    let mut identifiers = HashSet::with_capacity(fixture.process_cases.len());
    if fixture
        .process_cases
        .iter()
        .any(|case| case.id.is_empty() || !identifiers.insert(case.id.as_str()))
    {
        return Err(WorkerRequestError);
    }
    let case = fixture
        .process_cases
        .iter()
        .find(|case| case.id == case_id)
        .ok_or(WorkerRequestError)?;

    if request.probe != ProbeId::NvencTuples {
        return normal_worker_output(
            request,
            PrimitiveObservationV1 {
                available: false,
                code: "fixture-observation-missing".to_owned(),
                host_foundation: None,
                selected_output: None,
                capture: None,
                nvenc: None,
            },
        );
    }

    match case.behavior {
        Host04FixtureBehavior::Normal => {
            if case.api_version.major == 0 {
                return Err(WorkerRequestError);
            }
            let mut provider = crate::native_nvenc::DiagnosticNvencProvider::new(case.api_version);
            let nvenc =
                crate::native_nvenc::evaluate_nvenc_policy(case.gpu_generation, &mut provider);
            normal_worker_output(
                request,
                PrimitiveObservationV1 {
                    available: false,
                    code: "nvenc-diagnostic-policy-observed".to_owned(),
                    host_foundation: None,
                    selected_output: None,
                    capture: None,
                    nvenc: Some(nvenc),
                },
            )
        }
        Host04FixtureBehavior::Timeout => {
            thread::sleep(Duration::from_secs(120));
            Err(WorkerRequestError)
        }
    }
}

fn overlay_fixture_json(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base), serde_json::Value::Object(patch)) => {
            for (key, patch_value) in patch {
                if let Some(base_value) = base.get_mut(key) {
                    overlay_fixture_json(base_value, patch_value);
                } else {
                    base.insert(key.clone(), patch_value.clone());
                }
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

fn normal_worker_output(
    request: &ProbeRequestV1,
    observation: PrimitiveObservationV1,
) -> Result<WorkerOutput, WorkerRequestError> {
    validate_observation_for_request(request, &observation).map_err(|_| WorkerRequestError)?;
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
    if let Some(capture) = &observation.capture
        && (capture.schema != crate::model::NVFBC_CAPTURE_PRIMITIVES_SCHEMA_V1
            || capture.lifecycle.len() > crate::model::MAX_CAPTURE_LIFECYCLE_EVENTS_V1)
    {
        return Err(());
    }
    if let Some(nvenc) = &observation.nvenc
        && !crate::native_nvenc::validate_nvenc_tuples_evidence(nvenc)
    {
        return Err(());
    }
    if usize::from(observation.host_foundation.is_some())
        + usize::from(observation.selected_output.is_some())
        + usize::from(observation.capture.is_some())
        + usize::from(observation.nvenc.is_some())
        > 1
    {
        return Err(());
    }
    Ok(())
}

fn validate_observation_for_request(
    request: &ProbeRequestV1,
    observation: &PrimitiveObservationV1,
) -> Result<(), ()> {
    validate_observation(observation)?;
    if (observation.host_foundation.is_some() && request.probe != ProbeId::HostFoundation)
        || (observation.selected_output.is_some() && request.probe != ProbeId::SelectedOutput)
        || (observation.capture.is_some() && request.probe != ProbeId::NvfbcCapture)
        || (observation.nvenc.is_some() && request.probe != ProbeId::NvencTuples)
    {
        return Err(());
    }
    if request.probe == ProbeId::NvencTuples {
        let Some(nvenc) = observation.nvenc.as_ref() else {
            return Ok(());
        };
        if observation.available {
            return Err(());
        }
        let valid_provider = match request.source {
            ProbeSourceV1::Live => {
                nvenc.provider == crate::model::NvencProviderKindV1::LiveUnavailable
            }
            ProbeSourceV1::Fixture { .. } => {
                nvenc.provider == crate::model::NvencProviderKindV1::DiagnosticFixture
            }
        };
        return valid_provider.then_some(()).ok_or(());
    }
    if request.probe != ProbeId::NvfbcCapture {
        return observation.capture.is_none().then_some(()).ok_or(());
    }
    let capture = observation.capture.as_ref().ok_or(())?;
    if observation.host_foundation.is_some()
        || observation.selected_output.is_some()
        || observation.available != capture.failure.is_none()
        || !crate::evidence::validate_capture_source_identifiers(&capture.source)
        || capture
            .frame
            .as_ref()
            .is_some_and(|frame| !crate::evidence::validate_capture_frame_identifiers(frame))
    {
        return Err(());
    }

    use crate::model::{CaptureProviderKindV1 as Provider, CaptureSourceStatusV1 as Source};
    let valid_provider = match request.source {
        ProbeSourceV1::Live => matches!(
            (capture.provider, capture.source.status),
            (Provider::SourceAuthenticated, Source::Authenticated)
                | (Provider::LiveUnavailable, Source::Unavailable)
        ),
        ProbeSourceV1::Fixture { .. } => matches!(
            (capture.provider, capture.source.status),
            (Provider::Fixture, Source::Fixture) | (Provider::LiveUnavailable, Source::Unavailable)
        ),
    };
    valid_provider.then_some(()).ok_or(())
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host03FixtureDocument {
    schema: String,
    selected_output_fixture: String,
    selected_output_case: String,
    base_capture: serde_json::Value,
    cases: Vec<Host03FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host03FixtureCase {
    id: String,
    behavior: Host03FixtureBehavior,
    #[serde(default)]
    capture_patch: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Host03FixtureBehavior {
    Normal,
    Timeout,
    ChildCrash,
    MalformedResponse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host04FixtureDocument {
    schema: String,
    provenance: String,
    #[serde(rename = "sdk_contract")]
    _sdk_contract: serde_json::Value,
    #[serde(rename = "copy_boundary_cases")]
    _copy_boundary_cases: Vec<serde_json::Value>,
    #[serde(rename = "tuple_cases")]
    _tuple_cases: Vec<serde_json::Value>,
    #[serde(rename = "api_version_cases")]
    _api_version_cases: Vec<serde_json::Value>,
    #[serde(rename = "complete_attempt")]
    _complete_attempt: serde_json::Value,
    #[serde(rename = "bitstream_cases")]
    _bitstream_cases: Vec<serde_json::Value>,
    process_cases: Vec<Host04ProcessCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Host04ProcessCase {
    id: String,
    behavior: Host04FixtureBehavior,
    api_version: crate::model::NvencApiVersionV1,
    gpu_generation: crate::model::NvencGpuGenerationV1,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Host04FixtureBehavior {
    Normal,
    Timeout,
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
                matches!(probe, ProbeId::SelectedOutput | ProbeId::NvfbcCapture)
                    .then_some(requested_output)
                    .flatten(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CaptureFailureV1;

    #[test]
    fn host03_process_selected_output_is_forwarded_only_to_bound_probes() {
        struct BindingBackend;

        impl ProbeBackend for BindingBackend {
            fn observe(
                &self,
                probe: ProbeId,
                _parent_nonce: &str,
                requested_output: Option<&crate::model::OutputNameV1>,
            ) -> ProbeOutcome {
                assert_eq!(
                    requested_output.is_some(),
                    matches!(probe, ProbeId::SelectedOutput | ProbeId::NvfbcCapture),
                    "probe {probe:?}"
                );
                ProbeOutcome::failed(probe, ProbeFailure::Spawn)
            }
        }

        let output = crate::model::OutputNameV1 {
            hex: "44502d302e33".to_owned(),
            display: Some("DP-0.3".to_owned()),
        };
        let outcomes = collect_probes(&BindingBackend, "run-binding-test", Some(&output));
        assert_eq!(outcomes.len(), ProbeId::ALL.len());
    }

    #[test]
    fn host03_process_live_capture_outer_deadline_exceeds_native_grab() {
        let runner = BoundedProbeRunner::current(Duration::from_millis(40));
        let live_capture = ProbeRequestV1 {
            schema: REQUEST_SCHEMA.to_owned(),
            nonce: "nonce-deadline-test".to_owned(),
            probe: ProbeId::NvfbcCapture,
            source: ProbeSourceV1::Live,
            requested_output: Some(crate::model::OutputNameV1 {
                hex: "44502d302e33".to_owned(),
                display: Some("DP-0.3".to_owned()),
            }),
        };
        assert_eq!(
            runner.effective_timeout(&live_capture),
            MIN_LIVE_NVFBC_WORKER_TIMEOUT
        );

        let fixture_capture = ProbeRequestV1 {
            source: ProbeSourceV1::Fixture {
                path: PathBuf::from("fixture.json"),
                case_id: "timeout".to_owned(),
            },
            ..live_capture
        };
        assert_eq!(
            runner.effective_timeout(&fixture_capture),
            Duration::from_millis(40)
        );
    }

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

    #[test]
    fn host03_worker_protocol_binds_capture_provider_to_request_source() {
        let fixture: Host03FixtureDocument = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/host03-nvfbc-capture.json"
        ))
        .expect("HOST-03 fixture must decode");
        let fixture_capture: crate::model::CapturePrimitiveObservationV1 =
            serde_json::from_value(fixture.base_capture).expect("base capture must decode");
        let output = crate::model::OutputNameV1 {
            hex: "44502d302e33".to_owned(),
            display: Some("DP-0.3".to_owned()),
        };
        let fixture_request = ProbeRequestV1 {
            schema: REQUEST_SCHEMA.to_owned(),
            nonce: format!("nonce-{}", "a".repeat(64)),
            probe: ProbeId::NvfbcCapture,
            source: ProbeSourceV1::Fixture {
                path: PathBuf::from("fixture.json"),
                case_id: "positive".to_owned(),
            },
            requested_output: Some(output.clone()),
        };
        let fixture_observation = PrimitiveObservationV1 {
            available: true,
            code: "nvfbc-fixture-observed".to_owned(),
            host_foundation: None,
            selected_output: None,
            capture: Some(fixture_capture.clone()),
            nvenc: None,
        };
        assert!(validate_observation_for_request(&fixture_request, &fixture_observation).is_ok());

        let mut authenticated = fixture_capture.clone();
        authenticated.provider = crate::model::CaptureProviderKindV1::SourceAuthenticated;
        authenticated.source.status = crate::model::CaptureSourceStatusV1::Authenticated;
        let spoofed_fixture = PrimitiveObservationV1 {
            capture: Some(authenticated),
            ..fixture_observation.clone()
        };
        assert!(validate_observation_for_request(&fixture_request, &spoofed_fixture).is_err());

        let live_request = ProbeRequestV1 {
            source: ProbeSourceV1::Live,
            ..fixture_request
        };
        assert!(validate_observation_for_request(&live_request, &fixture_observation).is_err());

        let unavailable = unavailable_capture_observation();
        assert!(validate_observation_for_request(&live_request, &unavailable).is_ok());
    }

    #[test]
    fn host03_worker_protocol_rejects_runtime_paths_and_address_like_surfaces() {
        let fixture: Host03FixtureDocument = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/host03-nvfbc-capture.json"
        ))
        .expect("HOST-03 fixture must decode");
        let base: crate::model::CapturePrimitiveObservationV1 =
            serde_json::from_value(fixture.base_capture).expect("base capture must decode");
        let request = ProbeRequestV1 {
            schema: REQUEST_SCHEMA.to_owned(),
            nonce: format!("nonce-{}", "b".repeat(64)),
            probe: ProbeId::NvfbcCapture,
            source: ProbeSourceV1::Fixture {
                path: PathBuf::from("fixture.json"),
                case_id: "positive".to_owned(),
            },
            requested_output: Some(crate::model::OutputNameV1 {
                hex: "44502d302e33".to_owned(),
                display: Some("DP-0.3".to_owned()),
            }),
        };

        let observation = |capture| PrimitiveObservationV1 {
            available: true,
            code: "nvfbc-fixture-observed".to_owned(),
            host_foundation: None,
            selected_output: None,
            capture: Some(capture),
            nvenc: None,
        };

        let mut runtime_path = base.clone();
        runtime_path.source.nvfbc_runtime_library =
            Some("/private/operator/libnvidia-fbc.so.1".to_owned());
        assert!(validate_observation_for_request(&request, &observation(runtime_path)).is_err());

        for sentinel in [
            "140737488355328",
            "7ffdeadbeef",
            "0x7ffdeadbeef",
            "../raw-frame",
            "device-pointer-001",
        ] {
            let mut injected = base.clone();
            let frame = injected.frame.as_mut().expect("base frame");
            frame.source_surface = sentinel.to_owned();
            frame.edges[0].from_surface = sentinel.to_owned();
            assert!(
                validate_observation_for_request(&request, &observation(injected)).is_err(),
                "{sentinel}"
            );
        }
    }

    #[test]
    fn host03_contract_legacy_fixtures_do_not_add_a_worker_deadline() {
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let missing_worker =
            BoundedProbeRunner::with_executable("/definitely/not/a/worker", Duration::from_secs(1));
        let legacy = FixtureProbeBackend::new(
            missing_worker.clone(),
            fixture_root.join("host01-edge-cases.json"),
            "normal",
        );
        let outcome = legacy.observe(ProbeId::NvfbcCapture, "nonce-test", None);
        assert_eq!(outcome.failure, None);
        assert_eq!(
            outcome
                .observation
                .and_then(|observation| observation.capture)
                .and_then(|capture| capture.failure),
            Some(CaptureFailureV1::SourceUnavailable)
        );

        let host03 = FixtureProbeBackend::new(
            missing_worker,
            fixture_root.join("host03-nvfbc-capture.json"),
            "valid-zero-copy",
        );
        assert_eq!(
            host03
                .observe(ProbeId::NvfbcCapture, "nonce-test", None)
                .failure,
            Some(ProbeFailure::Spawn)
        );
    }

    #[test]
    fn host03_contract_and_host03_fixture_matrix_exhaust_boundaries() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/host03-nvfbc-capture.json");
        let fixture: Host03FixtureDocument = serde_json::from_slice(
            &std::fs::read(&fixture_path).expect("HOST-03 fixture must be readable"),
        )
        .expect("HOST-03 fixture must be structurally valid");
        let selected_observation = crate::output_mapping::collect_fixture_output_topology(
            &fixture_path
                .parent()
                .expect("fixture has a parent")
                .join(&fixture.selected_output_fixture),
            &fixture.selected_output_case,
            Some("DP-0.3"),
        )
        .expect("selected-output fixture must load");
        let selected = crate::output_mapping::prove_output_gpu_mapping(
            selected_observation
                .topology
                .as_ref()
                .expect("selected topology"),
        )
        .expect("selected output must be proven");

        for (case_id, expected) in [
            ("source-mismatch", CaptureFailureV1::SourceMismatch),
            ("api-mismatch", CaptureFailureV1::SourceMismatch),
            ("alternate-output", CaptureFailureV1::BindingMismatch),
            ("alternate-gpu", CaptureFailureV1::BindingMismatch),
            ("false-alias", CaptureFailureV1::InvalidCopyLedger),
            ("peer-without-access", CaptureFailureV1::InvalidCopyLedger),
            ("host-staged", CaptureFailureV1::InvalidCopyLedger),
            ("unknown-edge", CaptureFailureV1::InvalidCopyLedger),
            ("disconnected-edge", CaptureFailureV1::InvalidCopyLedger),
            ("malformed-frame", CaptureFailureV1::InvalidFrame),
            ("stale-frame", CaptureFailureV1::StaleFrame),
            ("cursor-mismatch", CaptureFailureV1::InvalidFrame),
            ("cleanup-missing", CaptureFailureV1::CleanupUncertain),
            ("cleanup-duplicate", CaptureFailureV1::CleanupUncertain),
            ("cleanup-out-of-order", CaptureFailureV1::CleanupUncertain),
            ("pointer-sentinel", CaptureFailureV1::InvalidFrame),
        ] {
            let capture = host03_fixture_capture(&fixture, case_id)
                .expect("semantic fixture case must decode");
            let evidence =
                crate::native_nvfbc::evaluate_capture_observation(capture, Some(&selected));
            assert_eq!(evidence.failure, Some(expected), "case {case_id}");
            assert!(evidence.lease.is_none(), "case {case_id}");
            assert!(evidence.copy_ledger.is_none(), "case {case_id}");
        }

        for case_id in [
            "injected-verdict",
            "injected-ledger",
            "injected-cleanup-result",
            "source-path-sentinel",
            "fallback-xcb",
            "fallback-pipewire",
            "fallback-software",
        ] {
            assert!(
                host03_fixture_capture(&fixture, case_id).is_err(),
                "authority/fallback case must fail strict primitive decoding: {case_id}"
            );
        }

        for (case_id, zero_copy, device_copy, conversion, post_processing) in [
            ("valid-zero-copy", 1, 0, 0, false),
            ("valid-device-copy", 0, 1, 0, false),
            ("valid-conversion", 0, 0, 1, false),
            ("required-post-processing", 1, 0, 0, true),
        ] {
            let capture =
                host03_fixture_capture(&fixture, case_id).expect("valid fixture case must decode");
            let evidence =
                crate::native_nvfbc::evaluate_capture_observation(capture, Some(&selected));
            assert_eq!(evidence.failure, None, "case {case_id}");
            let lease = evidence.lease.expect("valid case has a lease");
            let ledger = evidence.copy_ledger.expect("valid case has a ledger");
            assert_eq!(
                lease.required_post_processing, post_processing,
                "case {case_id}"
            );
            assert_eq!(ledger.zero_copy_edges, zero_copy, "case {case_id}");
            assert_eq!(ledger.device_copy_edges, device_copy, "case {case_id}");
            assert_eq!(ledger.conversion_edges, conversion, "case {case_id}");
        }

        for case_id in [
            "fail-after-library",
            "fail-after-status",
            "fail-after-context",
            "fail-after-session",
            "fail-after-frame",
        ] {
            let capture =
                host03_fixture_capture(&fixture, case_id).expect("partial fixture must decode");
            let evidence =
                crate::native_nvfbc::evaluate_capture_observation(capture, Some(&selected));
            assert_eq!(evidence.failure, Some(CaptureFailureV1::Busy));
            assert!(evidence.cleanup.complete, "case {case_id}");
        }
    }

    fn host03_fixture_capture(
        fixture: &Host03FixtureDocument,
        case_id: &str,
    ) -> Result<crate::model::CapturePrimitiveObservationV1, serde_json::Error> {
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .expect("fixture case must exist");
        let mut capture = fixture.base_capture.clone();
        overlay_fixture_json(&mut capture, &case.capture_patch);
        serde_json::from_value(capture)
    }
}
