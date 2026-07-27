use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::rust_connection::{DefaultStream, RustConnection};

pub const HOST_FOUNDATION_EVIDENCE_SCHEMA_V1: &str = "replaydesktop.host-foundation.v1";
const MAX_LOGINCTL_BYTES: usize = 64 * 1024;
const MAX_PROC_BYTES: usize = 64 * 1024;
const MAX_XAUTHORITY_BYTES: usize = 1024 * 1024;
const MAX_VERSION_BYTES: usize = 64;
const MAX_COUNT: u32 = 4096;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeerExecutableV1 {
    Xorg,
    Xwayland,
    Xephyr,
    Xnest,
    Xvnc,
    Xdummy,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostFoundationObservationV1 {
    pub session_candidate_count: u32,
    pub session_active: bool,
    pub session_local: bool,
    pub session_user_matches: bool,
    pub session_leader_contains_doctor: bool,
    pub session_type_x11: bool,
    pub display_number: Option<u16>,
    pub transport_unix: bool,
    pub peer_credentials_available: bool,
    pub peer_uid_matches: bool,
    pub peer_executable: PeerExecutableV1,
    pub x11_setup_succeeded: bool,
    pub x11_protocol_major: Option<u16>,
    pub x11_root_count: u32,
    pub randr_succeeded: bool,
    pub randr_provider_count: u32,
    pub randr_output_count: u32,
    pub uinput_access: bool,
    pub render_node_count: u32,
    pub accessible_render_node_count: u32,
    pub connected_drm_connector_count: u32,
    pub nvidia_kernel_version: Option<String>,
    #[serde(default)]
    pub nvml: crate::native_nvml::NvmlObservationV1,
}

impl HostFoundationObservationV1 {
    pub(crate) fn is_valid(&self) -> bool {
        if self.session_candidate_count > 64
            || self.x11_root_count > MAX_COUNT
            || self.randr_provider_count > MAX_COUNT
            || self.randr_output_count > MAX_COUNT
            || self.render_node_count > MAX_COUNT
            || self.accessible_render_node_count > self.render_node_count
            || self.connected_drm_connector_count > MAX_COUNT
            || self.display_number.is_some_and(|display| display > 1023)
        {
            return false;
        }
        if let Some(version) = &self.nvidia_kernel_version
            && (version.is_empty()
                || version.len() > MAX_VERSION_BYTES
                || !version.bytes().all(|byte| byte.is_ascii_graphic()))
        {
            return false;
        }
        self.nvml.is_valid()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FoundationCheckStatusV1 {
    Pass,
    Fail,
    Unproven,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostFoundationReasonV1 {
    pub code: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalXorgEvidenceV1 {
    pub status: FoundationCheckStatusV1,
    pub reason: Option<HostFoundationReasonV1>,
    pub session_candidate_count: u32,
    pub display_number: Option<u16>,
    pub transport_unix: bool,
    pub peer_executable: PeerExecutableV1,
    pub x11_protocol_major: Option<u16>,
    pub x11_root_count: u32,
    pub randr_provider_count: u32,
    pub randr_output_count: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationCheckEvidenceV1 {
    pub status: FoundationCheckStatusV1,
    pub observed: u32,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostFoundationEvidenceV1 {
    pub schema: String,
    pub local_xorg: LocalXorgEvidenceV1,
    pub uinput: FoundationCheckEvidenceV1,
    pub render_access: FoundationCheckEvidenceV1,
    pub physical_output: FoundationCheckEvidenceV1,
    pub nvidia_kernel_version: Option<String>,
    pub nvml: crate::native_nvml::NvmlEvidenceV1,
    pub selected_output_correlation: FoundationCheckEvidenceV1,
    pub reasons: Vec<HostFoundationReasonV1>,
}

impl HostFoundationEvidenceV1 {
    pub fn has_failure(&self) -> bool {
        self.reasons.iter().any(|reason| {
            if reason.code == "NVML_SOURCE_UNAVAILABLE" {
                self.nvml.status == crate::native_nvml::NvmlEvidenceStatusV1::Fail
            } else {
                reason.code != "SELECTED_OUTPUT_CORRELATION_UNPROVEN"
            }
        })
    }
}

pub fn prove_local_xorg(observation: &HostFoundationObservationV1) -> LocalXorgEvidenceV1 {
    let reason = local_xorg_failure(observation);
    LocalXorgEvidenceV1 {
        status: if reason.is_some() {
            FoundationCheckStatusV1::Fail
        } else {
            FoundationCheckStatusV1::Pass
        },
        reason,
        session_candidate_count: observation.session_candidate_count,
        display_number: observation.display_number,
        transport_unix: observation.transport_unix,
        peer_executable: observation.peer_executable,
        x11_protocol_major: observation.x11_protocol_major,
        x11_root_count: observation.x11_root_count,
        randr_provider_count: observation.randr_provider_count,
        randr_output_count: observation.randr_output_count,
    }
}

pub fn evaluate_host_foundation(
    observation: &HostFoundationObservationV1,
) -> HostFoundationEvidenceV1 {
    let local_xorg = prove_local_xorg(observation);
    let mut reasons = local_xorg.reason.iter().cloned().collect::<Vec<_>>();
    if !observation.uinput_access {
        reasons.push(reason(
            "UINPUT_ACCESS_REQUIRED",
            "Grant the invoking user read/write access to /dev/uinput, then rerun the doctor.",
        ));
    }
    if observation.accessible_render_node_count == 0 {
        reasons.push(reason(
            "DRM_RENDER_ACCESS_REQUIRED",
            "Grant the invoking user access to at least one DRM render node, then rerun the doctor.",
        ));
    }
    if observation.connected_drm_connector_count == 0 {
        reasons.push(reason(
            "PHYSICAL_OUTPUT_REQUIRED",
            "Connect and enable a physical DRM output before rerunning the doctor.",
        ));
    }
    let nvml = crate::native_nvml::evaluate_nvml(
        &observation.nvml,
        observation.nvidia_kernel_version.as_deref(),
    );
    reasons.extend(
        nvml.reasons
            .iter()
            .map(|entry| reason(&entry.code, &entry.remediation)),
    );
    reasons.push(reason(
        "SELECTED_OUTPUT_CORRELATION_UNPROVEN",
        "Select and correlate one physical X11 output in the owning later gate.",
    ));

    HostFoundationEvidenceV1 {
        schema: HOST_FOUNDATION_EVIDENCE_SCHEMA_V1.to_owned(),
        local_xorg,
        uinput: check(
            observation.uinput_access,
            u32::from(observation.uinput_access),
        ),
        render_access: check(
            observation.accessible_render_node_count > 0,
            observation.accessible_render_node_count,
        ),
        physical_output: check(
            observation.connected_drm_connector_count > 0,
            observation.connected_drm_connector_count,
        ),
        nvidia_kernel_version: observation.nvidia_kernel_version.clone(),
        nvml,
        selected_output_correlation: FoundationCheckEvidenceV1 {
            status: FoundationCheckStatusV1::Unproven,
            observed: 0,
        },
        reasons,
    }
}

fn check(passed: bool, observed: u32) -> FoundationCheckEvidenceV1 {
    FoundationCheckEvidenceV1 {
        status: if passed {
            FoundationCheckStatusV1::Pass
        } else {
            FoundationCheckStatusV1::Fail
        },
        observed,
    }
}

fn local_xorg_failure(observation: &HostFoundationObservationV1) -> Option<HostFoundationReasonV1> {
    if observation.session_candidate_count != 1 {
        return Some(reason(
            "SESSION_NOT_XORG",
            "Log into exactly one active local Xorg user session, then rerun the doctor.",
        ));
    }
    if !observation.session_local {
        return Some(reason(
            "SESSION_REMOTE",
            "Run the doctor from the active local seat rather than a remote session.",
        ));
    }
    if !observation.session_active
        || !observation.session_user_matches
        || !observation.session_leader_contains_doctor
        || !observation.session_type_x11
        || observation.display_number.is_none()
    {
        return Some(reason(
            "SESSION_NOT_XORG",
            "Log into one active local Xorg user session and run the doctor inside it.",
        ));
    }
    if !observation.transport_unix {
        return Some(reason(
            "X11_TRANSPORT_NOT_UNIX",
            "Use the local Xorg UNIX-domain socket; TCP and proxy transports are not admissible.",
        ));
    }
    if !observation.peer_credentials_available || !observation.peer_uid_matches {
        return Some(reason(
            "X11_PEER_CREDENTIALS_INVALID",
            "Use the X socket owned by the current local Xorg seat and rerun the doctor.",
        ));
    }
    if observation.peer_executable != PeerExecutableV1::Xorg {
        return Some(reason(
            "X11_PEER_NOT_XORG",
            "Exit Xwayland, nested, VNC, dummy, or proxy servers and log into real Xorg.",
        ));
    }
    if !observation.x11_setup_succeeded
        || observation.x11_protocol_major != Some(11)
        || observation.x11_root_count == 0
    {
        return Some(reason(
            "X11_SETUP_FAILED",
            "Repair local Xorg authorization/setup and rerun the doctor.",
        ));
    }
    if !observation.randr_succeeded
        || observation.randr_provider_count == 0
        || observation.randr_output_count == 0
    {
        return Some(reason(
            "XRANDR_QUERY_FAILED",
            "Enable the physical Xorg RandR provider/output and rerun the doctor.",
        ));
    }
    None
}

fn reason(code: &str, remediation: &str) -> HostFoundationReasonV1 {
    HostFoundationReasonV1 {
        code: code.to_owned(),
        remediation: remediation.to_owned(),
    }
}

pub fn observe_live_host_foundation() -> HostFoundationObservationV1 {
    let mut observation = empty_observation();
    collect_device_facts(&mut observation);
    observation.nvml = crate::native_nvml::observe_live_nvml();
    let sessions = collect_sessions();
    observation.session_candidate_count = u32::try_from(sessions.len()).unwrap_or(u32::MAX);
    if sessions.len() != 1 {
        return observation;
    }
    let session = &sessions[0];
    observation.session_active = session.active;
    observation.session_local = !session.remote && session.remote_host.is_empty();
    observation.session_user_matches = session.user == rustix::process::geteuid().as_raw();
    observation.session_leader_contains_doctor = ancestry_contains(session.leader);
    observation.session_type_x11 = session.session_type == "x11";
    observation.display_number = parse_display_number(&session.display).map(|(display, _)| display);

    let Some((display, screen)) = parse_display_number(&session.display) else {
        return observation;
    };
    let socket_path = PathBuf::from(format!("/tmp/.X11-unix/X{display}"));
    let Ok(socket) = UnixStream::connect(&socket_path) else {
        return observation;
    };
    observation.transport_unix = true;
    let Ok(credentials) = rustix::net::sockopt::socket_peercred(&socket) else {
        return observation;
    };
    observation.peer_credentials_available = true;
    let peer_uid = credentials.uid.as_raw();
    observation.peer_uid_matches = peer_uid == session.user || credentials.uid.is_root();
    let peer_pid = credentials.pid.as_raw_pid();
    observation.peer_executable = classify_peer_executable(peer_pid);
    if observation.peer_executable != PeerExecutableV1::Xorg {
        return observation;
    }

    let (auth_name, auth_data) = xauthority_for(session.leader, display).unwrap_or_default();
    let Ok((stream, _)) = DefaultStream::from_unix_stream(socket) else {
        return observation;
    };
    let Ok(connection) = RustConnection::connect_to_stream_with_auth_info(
        stream,
        usize::from(screen),
        auth_name,
        auth_data,
    ) else {
        return observation;
    };
    let setup = connection.setup();
    observation.x11_setup_succeeded = true;
    observation.x11_protocol_major = Some(setup.protocol_major_version);
    observation.x11_root_count = u32::try_from(setup.roots.len()).unwrap_or(u32::MAX);
    let Some(root) = setup
        .roots
        .get(usize::from(screen))
        .map(|screen| screen.root)
    else {
        return observation;
    };
    let randr_present = connection
        .extension_information(x11rb::protocol::randr::X11_EXTENSION_NAME)
        .ok()
        .flatten()
        .is_some();
    if !randr_present {
        return observation;
    }
    let Ok(version_cookie) = connection.randr_query_version(1, 6) else {
        return observation;
    };
    if version_cookie.reply().is_err() {
        return observation;
    }
    let Ok(provider_cookie) = connection.randr_get_providers(root) else {
        return observation;
    };
    let Ok(providers) = provider_cookie.reply() else {
        return observation;
    };
    let Ok(resource_cookie) = connection.randr_get_screen_resources_current(root) else {
        return observation;
    };
    let Ok(resources) = resource_cookie.reply() else {
        return observation;
    };
    observation.randr_succeeded = true;
    observation.randr_provider_count = u32::try_from(providers.providers.len()).unwrap_or(u32::MAX);
    observation.randr_output_count = u32::try_from(resources.outputs.len()).unwrap_or(u32::MAX);
    observation
}

fn empty_observation() -> HostFoundationObservationV1 {
    HostFoundationObservationV1 {
        session_candidate_count: 0,
        session_active: false,
        session_local: false,
        session_user_matches: false,
        session_leader_contains_doctor: false,
        session_type_x11: false,
        display_number: None,
        transport_unix: false,
        peer_credentials_available: false,
        peer_uid_matches: false,
        peer_executable: PeerExecutableV1::Unknown,
        x11_setup_succeeded: false,
        x11_protocol_major: None,
        x11_root_count: 0,
        randr_succeeded: false,
        randr_provider_count: 0,
        randr_output_count: 0,
        uinput_access: false,
        render_node_count: 0,
        accessible_render_node_count: 0,
        connected_drm_connector_count: 0,
        nvidia_kernel_version: None,
        nvml: crate::native_nvml::NvmlObservationV1::default(),
    }
}

#[derive(Debug)]
struct LogindSession {
    user: u32,
    remote: bool,
    remote_host: String,
    session_type: String,
    active: bool,
    display: String,
    leader: i32,
}

fn collect_sessions() -> Vec<LogindSession> {
    let Some(output) = run_loginctl(&["list-sessions", "--no-legend", "--no-pager"]) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    let mut identifiers = HashSet::new();
    for line in output.lines() {
        let Some(identifier) = line.split_whitespace().next() else {
            continue;
        };
        if identifier.len() > 64
            || !identifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || !identifiers.insert(identifier.to_owned())
        {
            continue;
        }
        if let Some(session) = show_session(identifier)
            && session.user == rustix::process::geteuid().as_raw()
            && session.active
        {
            sessions.push(session);
        }
    }
    sessions
}

fn show_session(identifier: &str) -> Option<LogindSession> {
    const PROPERTIES: [&str; 13] = [
        "Id",
        "User",
        "Seat",
        "TTY",
        "Display",
        "Remote",
        "RemoteHost",
        "Type",
        "Class",
        "Active",
        "State",
        "Leader",
        "Scope",
    ];
    let mut arguments = vec!["show-session", identifier, "--no-pager"];
    let property_arguments = PROPERTIES
        .iter()
        .map(|property| format!("--property={property}"))
        .collect::<Vec<_>>();
    arguments.extend(property_arguments.iter().map(String::as_str));
    let output = run_loginctl(&arguments)?;
    let mut values = HashMap::new();
    for line in output.lines() {
        let (key, value) = line.split_once('=')?;
        if !PROPERTIES.contains(&key) || values.insert(key, value).is_some() {
            return None;
        }
    }
    if values.len() != PROPERTIES.len()
        || values.get("Id").copied()? != identifier
        || values.get("Class").copied()? != "user"
        || values.get("State").copied()? != "active"
        || values.get("Seat").copied()?.is_empty()
        || values.get("Scope").copied()?.is_empty()
    {
        return None;
    }
    Some(LogindSession {
        user: values.get("User")?.parse().ok()?,
        remote: match values.get("Remote").copied()? {
            "yes" => true,
            "no" => false,
            _ => return None,
        },
        remote_host: values.get("RemoteHost").copied()?.to_owned(),
        session_type: values.get("Type").copied()?.to_owned(),
        active: values.get("Active").copied()? == "yes",
        display: values.get("Display").copied()?.to_owned(),
        leader: values.get("Leader")?.parse().ok()?,
    })
}

fn run_loginctl(arguments: &[&str]) -> Option<String> {
    let output = Command::new("/usr/bin/loginctl")
        .args(arguments)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success()
        || !output.stderr.is_empty()
        || output.stdout.len() > MAX_LOGINCTL_BYTES
    {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn ancestry_contains(leader: i32) -> bool {
    if leader <= 1 {
        return false;
    }
    let mut pid = i32::try_from(std::process::id()).unwrap_or(i32::MAX);
    let mut seen = HashSet::new();
    for _ in 0..128 {
        if pid == leader {
            return true;
        }
        if pid <= 1 || !seen.insert(pid) {
            return false;
        }
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };
        let Some(close) = stat.rfind(')') else {
            return false;
        };
        let mut fields = stat[close + 1..].split_whitespace();
        let Some(_state) = fields.next() else {
            return false;
        };
        let Some(parent) = fields.next().and_then(|value| value.parse::<i32>().ok()) else {
            return false;
        };
        pid = parent;
    }
    false
}

fn parse_display_number(value: &str) -> Option<(u16, u16)> {
    let local = value.strip_prefix(':')?;
    let (display, screen) = local.split_once('.').unwrap_or((local, "0"));
    if display.is_empty()
        || screen.is_empty()
        || !display.bytes().all(|byte| byte.is_ascii_digit())
        || !screen.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let display = display.parse::<u16>().ok()?;
    let screen = screen.parse::<u16>().ok()?;
    (display <= 1023 && screen <= 255).then_some((display, screen))
}

fn classify_peer_executable(pid: i32) -> PeerExecutableV1 {
    if pid <= 1 {
        return PeerExecutableV1::Unknown;
    }
    let Ok(path) = std::fs::read_link(format!("/proc/{pid}/exe")) else {
        return PeerExecutableV1::Unknown;
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        return PeerExecutableV1::Unknown;
    };
    match path.file_name().and_then(|name| name.to_str()) {
        Some("Xorg")
            if metadata.is_file()
                && metadata.uid() == 0
                && metadata.mode() & 0o111 != 0
                && metadata.mode() & 0o022 == 0 =>
        {
            PeerExecutableV1::Xorg
        }
        Some("Xorg") => PeerExecutableV1::Other,
        Some("Xwayland") => PeerExecutableV1::Xwayland,
        Some("Xephyr") => PeerExecutableV1::Xephyr,
        Some("Xnest") => PeerExecutableV1::Xnest,
        Some("Xvnc") => PeerExecutableV1::Xvnc,
        Some("Xdummy") => PeerExecutableV1::Xdummy,
        Some(_) => PeerExecutableV1::Other,
        None => PeerExecutableV1::Unknown,
    }
}

fn xauthority_for(leader: i32, display: u16) -> Option<(Vec<u8>, Vec<u8>)> {
    let bytes = read_bounded(
        Path::new(&format!("/proc/{leader}/environ")),
        MAX_PROC_BYTES,
    )?;
    let variables = bytes
        .split(|byte| *byte == 0)
        .filter_map(|entry| {
            entry
                .iter()
                .position(|byte| *byte == b'=')
                .map(|separator| (&entry[..separator], &entry[separator + 1..]))
        })
        .collect::<HashMap<_, _>>();
    let path = variables
        .get(&b"XAUTHORITY"[..])
        .filter(|value| !value.is_empty())
        .map(|value| PathBuf::from(std::ffi::OsStr::from_bytes(value)))
        .or_else(|| {
            variables.get(&b"HOME"[..]).map(|value| {
                let mut path = PathBuf::from(std::ffi::OsStr::from_bytes(value));
                path.push(".Xauthority");
                path
            })
        })?;
    parse_xauthority(&read_bounded(&path, MAX_XAUTHORITY_BYTES)?, display)
}

fn parse_xauthority(bytes: &[u8], display: u16) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut cursor = 0;
    while cursor < bytes.len() {
        let family = take_u16(bytes, &mut cursor)?;
        let _address = take_field(bytes, &mut cursor)?;
        let number = take_field(bytes, &mut cursor)?;
        let name = take_field(bytes, &mut cursor)?;
        let data = take_field(bytes, &mut cursor)?;
        if matches!(family, 256 | 65535)
            && number == display.to_string().as_bytes()
            && name == b"MIT-MAGIC-COOKIE-1"
            && !data.is_empty()
        {
            return Some((name.to_vec(), data.to_vec()));
        }
    }
    None
}

fn take_u16(bytes: &[u8], cursor: &mut usize) -> Option<u16> {
    let value = u16::from_be_bytes(bytes.get(*cursor..*cursor + 2)?.try_into().ok()?);
    *cursor += 2;
    Some(value)
}

fn take_field<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let length = usize::from(take_u16(bytes, cursor)?);
    let value = bytes.get(*cursor..*cursor + length)?;
    *cursor += length;
    Some(value)
}

fn read_bounded(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= limit).then_some(bytes)
}

fn collect_device_facts(observation: &mut HostFoundationObservationV1) {
    observation.uinput_access = rustix::fs::access(
        "/dev/uinput",
        rustix::fs::Access::READ_OK | rustix::fs::Access::WRITE_OK,
    )
    .is_ok();
    if let Ok(entries) = std::fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !name.starts_with("renderD") || !name[7..].bytes().all(|byte| byte.is_ascii_digit())
            {
                continue;
            }
            observation.render_node_count = observation.render_node_count.saturating_add(1);
            if rustix::fs::access(
                entry.path(),
                rustix::fs::Access::READ_OK | rustix::fs::Access::WRITE_OK,
            )
            .is_ok()
            {
                observation.accessible_render_node_count =
                    observation.accessible_render_node_count.saturating_add(1);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.contains('-') {
                continue;
            }
            let status = std::fs::read_to_string(entry.path().join("status")).ok();
            let modes = std::fs::read_to_string(entry.path().join("modes")).ok();
            if status
                .as_deref()
                .is_some_and(|value| value.trim() == "connected")
                && modes
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
            {
                observation.connected_drm_connector_count =
                    observation.connected_drm_connector_count.saturating_add(1);
            }
        }
    }
    observation.nvidia_kernel_version =
        read_bounded(Path::new("/proc/driver/nvidia/version"), MAX_PROC_BYTES)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| {
                text.split_ascii_whitespace()
                    .find(|token| version_token(token))
                    .map(str::to_owned)
            });
}

fn version_token(value: &str) -> bool {
    value.len() <= MAX_VERSION_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
        && value.bytes().filter(|byte| *byte == b'.').count() >= 2
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_xorg_display_parser_rejects_remote_and_trailing_data() {
        assert_eq!(parse_display_number(":0"), Some((0, 0)));
        assert_eq!(parse_display_number(":12.1"), Some((12, 1)));
        assert_eq!(parse_display_number("host:0"), None);
        assert_eq!(parse_display_number(":0.1.extra"), None);
    }

    #[test]
    fn host01_source_neutral_version_parser_preserves_exact_components() {
        assert!(version_token("610.43.02"));
        assert!(!version_token("610.43"));
        assert!(!version_token("610.43.02/evil"));
    }
}
