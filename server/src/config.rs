use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "replay-control", version, about)]
pub struct Cli {
    #[arg(
        long,
        env = "REPLAY_DATABASE_URL",
        default_value = "sqlite://replay-control.db",
        global = true
    )]
    pub database_url: String,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Serve(Box<ServeArgs>),
    InitKeys(InitKeysArgs),
    CreateUser(CreateUserArgs),
    DisableUser {
        username: String,
    },
    CreateWorkstation(CreateWorkstationArgs),
    RegisterHost(RegisterHostArgs),
    Grant {
        username: String,
        workstation: String,
    },
    Revoke {
        username: String,
        workstation: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AdmissionMode {
    Unifi,
    Memory,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, env = "REPLAY_HTTP_BIND", default_value = "127.0.0.1:8080")]
    pub http_bind: SocketAddr,
    #[arg(long, env = "REPLAY_KNOCK_BIND", default_value = "0.0.0.0:8444")]
    pub knock_bind: SocketAddr,
    #[arg(long, env = "REPLAY_PUBLIC_HOST", default_value = "127.0.0.1")]
    pub public_host: String,
    #[arg(long, env = "REPLAY_KNOCK_ENDPOINT", default_value = "127.0.0.1:8444")]
    pub knock_endpoint: String,
    #[arg(long, env = "REPLAY_ALLOW_INSECURE_HTTP", default_value_t = false)]
    pub allow_insecure_http: bool,
    #[arg(long, env = "REPLAY_LAN_MODE", default_value_t = false)]
    pub lan_mode: bool,
    #[arg(long, env = "REPLAY_HOST_REGISTRATION_TOKEN_FILE")]
    pub host_registration_token_file: Option<PathBuf>,
    #[arg(long, env = "REPLAY_KYBER_JWT_PRIVATE_KEY")]
    pub kyber_jwt_private_key: Option<PathBuf>,
    #[arg(long, env = "REPLAY_HOST_OFFLINE_AFTER_SECONDS", default_value_t = 45)]
    pub host_offline_after_seconds: u64,
    #[arg(
        long,
        env = "REPLAY_ALLOW_INSECURE_DEV_BOOTSTRAP",
        default_value_t = false
    )]
    pub allow_insecure_dev_bootstrap: bool,
    #[arg(long, env = "REPLAY_DEV_BOOTSTRAP_USERNAME")]
    pub dev_bootstrap_username: Option<String>,
    #[arg(long, env = "REPLAY_DEV_BOOTSTRAP_PASSWORD")]
    pub dev_bootstrap_password: Option<String>,
    #[arg(
        long,
        env = "REPLAY_TICKET_PRIVATE_KEY",
        default_value = "replay-ticket-private.pem"
    )]
    pub ticket_private_key: PathBuf,
    #[arg(long, env = "REPLAY_TICKET_ISSUER", default_value = "replay-control")]
    pub ticket_issuer: String,
    #[arg(long, env = "REPLAY_TICKET_KEY_ID", default_value = "replay-v1")]
    pub ticket_key_id: String,
    #[arg(long, env = "REPLAY_ADMISSION_BACKEND", default_value = "unifi")]
    pub admission_backend: AdmissionMode,
    #[arg(long, env = "REPLAY_UNIFI_BASE_URL")]
    pub unifi_base_url: Option<String>,
    #[arg(long, env = "REPLAY_UNIFI_SITE_ID")]
    pub unifi_site_id: Option<Uuid>,
    #[arg(long, env = "REPLAY_UNIFI_API_KEY")]
    pub unifi_api_key: Option<String>,
    #[arg(long, env = "REPLAY_UNIFI_EXTERNAL_ZONE_ID")]
    pub unifi_external_zone_id: Option<Uuid>,
    #[arg(long, env = "REPLAY_UNIFI_INTERNAL_ZONE_ID")]
    pub unifi_internal_zone_id: Option<Uuid>,
}

#[derive(Debug, Args)]
pub struct InitKeysArgs {
    #[arg(long, default_value = "replay-ticket-private.pem")]
    pub private_key: PathBuf,
    #[arg(long, default_value = "replay-ticket-public.pem")]
    pub public_key: PathBuf,
}

#[derive(Debug, Args)]
pub struct CreateUserArgs {
    pub username: String,
    #[arg(long, value_enum, default_value = "user")]
    pub role: UserRole,
    #[arg(
        long,
        help = "Read the new password from stdin instead of prompting on the terminal"
    )]
    pub password_stdin: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum UserRole {
    Admin,
    User,
}

#[derive(Debug, Args)]
pub struct CreateWorkstationArgs {
    pub name: String,
    #[arg(long)]
    pub lan_ipv4: std::net::Ipv4Addr,
    #[arg(long)]
    pub kymux_port: u16,
    #[arg(long)]
    pub wan_port: u16,
    #[arg(long)]
    pub certificate_sha256: String,
}

#[derive(Debug, Args)]
pub struct RegisterHostArgs {
    #[arg(long, env = "REPLAY_BROKER_URL")]
    pub broker_url: String,
    #[arg(
        long,
        env = "REPLAY_HOST_REGISTRATION_ID_FILE",
        default_value = "/var/lib/replay-control/host-registration-id"
    )]
    pub registration_id_file: PathBuf,
    #[arg(long, env = "REPLAY_HOST_REGISTRATION_TOKEN_FILE")]
    pub token_file: PathBuf,
    #[arg(long, env = "REPLAY_HOST_NAME")]
    pub name: String,
    #[arg(long, env = "REPLAY_HOST_HOSTNAME")]
    pub hostname: String,
    #[arg(long, env = "REPLAY_HOST_LAN_IPV4")]
    pub lan_ipv4: Ipv4Addr,
    #[arg(long, env = "REPLAY_HOST_KYMUX_PORT", default_value_t = 8080)]
    pub kymux_port: u16,
    #[arg(long, env = "REPLAY_HOST_CERTIFICATE_FILE")]
    pub certificate_file: PathBuf,
    #[arg(long, env = "REPLAY_HOST_HEARTBEAT_SECONDS", default_value_t = 15)]
    pub heartbeat_seconds: u64,
}
