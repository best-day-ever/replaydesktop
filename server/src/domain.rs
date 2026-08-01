use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ACCESS_TOKEN_TTL_SECONDS: i64 = 10 * 60;
pub const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
pub const KNOCK_TTL_SECONDS: i64 = 30;
pub const LEASE_TTL_SECONDS: i64 = 12 * 60 * 60;
pub const KYMUX_TICKET_TTL_SECONDS: i64 = 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Admin,
    User,
}

impl Role {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(Self::Admin),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Workstation {
    pub id: Uuid,
    pub name: String,
    pub lan_ipv4: Ipv4Addr,
    pub kymux_port: u16,
    pub wan_port: u16,
    pub certificate_sha256: String,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub access_expires_at: i64,
    pub refresh_expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingConnection {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workstation: Workstation,
    pub knock_expires_at: i64,
    pub lease_expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnockAuthorization {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workstation: Workstation,
    pub lease_expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionStatus {
    Pending,
    OpeningLease,
    Ready,
    Failed,
    Closed,
    Expired,
}

impl ConnectionStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::OpeningLease => "opening_lease",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Closed => "closed",
            Self::Expired => "expired",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "opening_lease" => Some(Self::OpeningLease),
            "ready" => Some(Self::Ready),
            "failed" => Some(Self::Failed),
            "closed" => Some(Self::Closed),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workstation: Workstation,
    pub status: ConnectionStatus,
    pub source_ipv4: Option<Ipv4Addr>,
    pub admission_lease_id: Option<String>,
    pub ticket_jti: Option<Uuid>,
    pub kymux_token: Option<String>,
    pub ticket_issued_at: Option<i64>,
    pub ticket_expires_at: Option<i64>,
    pub failure_reason: Option<String>,
    pub knock_expires_at: i64,
    pub lease_expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpiredLease {
    pub session_id: Uuid,
    pub lease_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub access_expires_at: i64,
    pub refresh_token: String,
    pub refresh_expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct WorkstationSummary {
    pub id: Uuid,
    pub name: String,
}

impl From<Workstation> for WorkstationSummary {
    fn from(value: Workstation) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateConnectionResponse {
    pub session_id: Uuid,
    pub status: &'static str,
    pub knock_endpoint: String,
    pub knock_token: String,
    pub knock_expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub session_id: Uuid,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workstation_certificate_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kymux_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}
