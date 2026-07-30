use std::{net::SocketAddr, path::PathBuf};

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
    #[arg(long, env = "REPLAY_PUBLIC_HOST")]
    pub public_host: String,
    #[arg(long, env = "REPLAY_KNOCK_ENDPOINT")]
    pub knock_endpoint: String,
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
