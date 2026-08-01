use std::{net::Ipv4Addr, str::FromStr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand_core_06::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow},
};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    ACCESS_TOKEN_TTL_SECONDS, ApiSession, ConnectionSession, ConnectionStatus, ExpiredLease,
    KNOCK_TTL_SECONDS, KnockAuthorization, LEASE_TTL_SECONDS, PendingConnection,
    REFRESH_TOKEN_TTL_SECONDS, Role, User, Workstation, WorkstationPresence,
};

const WORKSTATION_COLUMNS: &str = "
    w.id AS workstation_id,
    w.name AS workstation_name,
    w.lan_ipv4,
    w.kymux_port,
    w.wan_port,
    w.certificate_sha256,
    w.active AS workstation_active
";

const WORKSTATION_PRESENCE_COLUMNS: &str = "
    w.registration_id,
    w.registered_hostname,
    w.agent_version,
    w.last_seen_at
";

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiTokens {
    pub session: ApiSession,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database file security check failed")]
    Io(#[from] std::io::Error),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("stored data is invalid: {0}")]
    InvalidData(String),
    #[error("record not found")]
    NotFound,
    #[error("access denied")]
    AccessDenied,
    #[error("workstation is offline")]
    HostOffline,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        secure_database_file(database_url, options.get_filename())?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: Role,
        now: i64,
    ) -> Result<User, StoreError> {
        let username = username.trim();
        if username.is_empty() || username.len() > 128 {
            return Err(StoreError::InvalidData(
                "username must contain between 1 and 128 characters".to_owned(),
            ));
        }
        let id = Uuid::new_v4();
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, active, created_at)
             VALUES (?, ?, ?, ?, 1, ?)",
        )
        .bind(id.to_string())
        .bind(username)
        .bind(password_hash)
        .bind(role.as_str())
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO workstation_grants (user_id, workstation_id, created_at)
             SELECT ?, id, ? FROM workstations
             WHERE active = 1 AND registration_id IS NOT NULL",
        )
        .bind(id.to_string())
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(User {
            id,
            username: username.to_owned(),
            password_hash: password_hash.to_owned(),
            role,
            active: true,
        })
    }

    pub async fn disable_user(&self, username: &str, now: i64) -> Result<(), StoreError> {
        let mut transaction = self.pool.begin().await?;
        let result = sqlx::query("UPDATE users SET active = 0 WHERE username = ? COLLATE NOCASE")
            .bind(username)
            .execute(&mut *transaction)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        sqlx::query(
            "UPDATE api_sessions
             SET revoked_at = ?
             WHERE user_id = (
                 SELECT id FROM users WHERE username = ? COLLATE NOCASE
             ) AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(username)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn user_by_username(&self, username: &str) -> Result<Option<User>, StoreError> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, role, active
             FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| user_from_row(&row)).transpose()
    }

    pub async fn create_workstation(
        &self,
        name: &str,
        lan_ipv4: Ipv4Addr,
        kymux_port: u16,
        wan_port: u16,
        certificate_sha256: &str,
        now: i64,
    ) -> Result<Workstation, StoreError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 128 {
            return Err(StoreError::InvalidData(
                "workstation name must contain between 1 and 128 characters".to_owned(),
            ));
        }
        let certificate_sha256 = certificate_sha256.to_ascii_lowercase();
        if certificate_sha256.len() != 64
            || !certificate_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StoreError::InvalidData(
                "certificate fingerprint must be 64 hexadecimal characters".to_owned(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workstations (
                id, name, lan_ipv4, kymux_port, wan_port,
                certificate_sha256, active, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
        )
        .bind(id.to_string())
        .bind(name)
        .bind(lan_ipv4.to_string())
        .bind(i64::from(kymux_port))
        .bind(i64::from(wan_port))
        .bind(&certificate_sha256)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(Workstation {
            id,
            name: name.to_owned(),
            lan_ipv4,
            kymux_port,
            wan_port,
            certificate_sha256,
            active: true,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register_host(
        &self,
        registration_id: Uuid,
        name: &str,
        hostname: &str,
        lan_ipv4: Ipv4Addr,
        kymux_port: u16,
        certificate_sha256: &str,
        agent_version: &str,
        now: i64,
    ) -> Result<WorkstationPresence, StoreError> {
        let name = validated_text(name, "workstation name")?;
        let hostname = validated_text(hostname, "hostname")?;
        let agent_version = validated_text(agent_version, "agent version")?;
        let certificate_sha256 = validated_fingerprint(certificate_sha256)?;
        if registration_id.is_nil() {
            return Err(StoreError::InvalidData(
                "registration id cannot be nil".to_owned(),
            ));
        }

        let mut transaction = self.pool.begin().await?;
        let existing_by_registration =
            sqlx::query("SELECT id FROM workstations WHERE registration_id = ?")
                .bind(registration_id.to_string())
                .fetch_optional(&mut *transaction)
                .await?;
        let workstation_id = if let Some(row) = existing_by_registration {
            parse_uuid(row.try_get::<String, _>("id")?)?
        } else if let Some(row) = sqlx::query(
            "SELECT id, registration_id FROM workstations WHERE name = ? COLLATE NOCASE",
        )
        .bind(name)
        .fetch_optional(&mut *transaction)
        .await?
        {
            let owner: Option<String> = row.try_get("registration_id")?;
            if owner.is_some() {
                return Err(StoreError::InvalidData(
                    "workstation name belongs to another registered host".to_owned(),
                ));
            }
            parse_uuid(row.try_get::<String, _>("id")?)?
        } else {
            let id = Uuid::new_v4();
            let occupied: Option<i64> =
                sqlx::query_scalar("SELECT wan_port FROM workstations WHERE wan_port = ?")
                    .bind(i64::from(kymux_port))
                    .fetch_optional(&mut *transaction)
                    .await?;
            let wan_port = if occupied.is_none() {
                kymux_port
            } else {
                let next: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(wan_port), 19999) + 1 FROM workstations",
                )
                .fetch_one(&mut *transaction)
                .await?;
                u16::try_from(next).map_err(|_| {
                    StoreError::InvalidData("no placeholder WAN port is available".to_owned())
                })?
            };
            sqlx::query(
                "INSERT INTO workstations (
                    id, name, lan_ipv4, kymux_port, wan_port,
                    certificate_sha256, active, created_at
                 ) VALUES (?, ?, ?, ?, ?, ?, 1, ?)",
            )
            .bind(id.to_string())
            .bind(name)
            .bind(lan_ipv4.to_string())
            .bind(i64::from(kymux_port))
            .bind(i64::from(wan_port))
            .bind(&certificate_sha256)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
            id
        };

        sqlx::query(
            "UPDATE workstations
             SET name = ?, lan_ipv4 = ?, kymux_port = ?, certificate_sha256 = ?,
                 registration_id = ?, registered_hostname = ?, agent_version = ?,
                 last_seen_at = ?, active = 1
             WHERE id = ?",
        )
        .bind(name)
        .bind(lan_ipv4.to_string())
        .bind(i64::from(kymux_port))
        .bind(&certificate_sha256)
        .bind(registration_id.to_string())
        .bind(hostname)
        .bind(agent_version)
        .bind(now)
        .bind(workstation_id.to_string())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO workstation_grants (user_id, workstation_id, created_at)
             SELECT id, ?, ? FROM users WHERE active = 1",
        )
        .bind(workstation_id.to_string())
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.workstation_presence(workstation_id)
            .await?
            .ok_or(StoreError::NotFound)
    }

    pub async fn grant_workstation(
        &self,
        username: &str,
        workstation_name: &str,
        now: i64,
    ) -> Result<(), StoreError> {
        let result = sqlx::query(
            "INSERT INTO workstation_grants (user_id, workstation_id, created_at)
             SELECT u.id, w.id, ?
             FROM users u, workstations w
             WHERE u.username = ? COLLATE NOCASE
               AND w.name = ? COLLATE NOCASE",
        )
        .bind(now)
        .bind(username)
        .bind(workstation_name)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub async fn revoke_workstation(
        &self,
        username: &str,
        workstation_name: &str,
    ) -> Result<(), StoreError> {
        let result = sqlx::query(
            "DELETE FROM workstation_grants
             WHERE user_id = (
                 SELECT id FROM users WHERE username = ? COLLATE NOCASE
             ) AND workstation_id = (
                 SELECT id FROM workstations WHERE name = ? COLLATE NOCASE
             )",
        )
        .bind(username)
        .bind(workstation_name)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub async fn issue_api_session(
        &self,
        user_id: Uuid,
        now: i64,
    ) -> Result<ApiTokens, StoreError> {
        let tokens = new_api_tokens(user_id, now);
        insert_api_session(&self.pool, &tokens, now).await?;
        Ok(tokens)
    }

    pub async fn rotate_refresh(
        &self,
        refresh_token: &str,
        now: i64,
    ) -> Result<Option<ApiTokens>, StoreError> {
        let digest = token_digest(refresh_token.as_bytes());
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT s.id, s.user_id
             FROM api_sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.refresh_hash = ?
               AND s.refresh_expires_at >= ?
               AND s.revoked_at IS NULL
               AND u.active = 1",
        )
        .bind(digest.as_slice())
        .bind(now)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            transaction.rollback().await?;
            return Ok(None);
        };
        let old_session_id: String = row.try_get("id")?;
        let user_id = parse_uuid(row.try_get::<String, _>("user_id")?)?;
        let revoked = sqlx::query(
            "UPDATE api_sessions SET revoked_at = ?
             WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(old_session_id)
        .execute(&mut *transaction)
        .await?;
        if revoked.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        let tokens = new_api_tokens(user_id, now);
        sqlx::query(
            "INSERT INTO api_sessions (
                id, user_id, access_hash, access_expires_at,
                refresh_hash, refresh_expires_at, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tokens.session.id.to_string())
        .bind(tokens.session.user_id.to_string())
        .bind(token_digest(tokens.access_token.as_bytes()).as_slice())
        .bind(tokens.session.access_expires_at)
        .bind(token_digest(tokens.refresh_token.as_bytes()).as_slice())
        .bind(tokens.session.refresh_expires_at)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(Some(tokens))
    }

    pub async fn authenticate_access(
        &self,
        access_token: &str,
        now: i64,
    ) -> Result<Option<User>, StoreError> {
        let digest = token_digest(access_token.as_bytes());
        let row = sqlx::query(
            "SELECT u.id, u.username, u.password_hash, u.role, u.active
             FROM api_sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.access_hash = ?
               AND s.access_expires_at >= ?
               AND s.revoked_at IS NULL
               AND u.active = 1",
        )
        .bind(digest.as_slice())
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| user_from_row(&row)).transpose()
    }

    pub async fn logout(&self, access_token: &str, now: i64) -> Result<bool, StoreError> {
        let digest = token_digest(access_token.as_bytes());
        let result = sqlx::query(
            "UPDATE api_sessions SET revoked_at = ?
             WHERE access_hash = ? AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(digest.as_slice())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn list_workstations(&self, user: &User) -> Result<Vec<Workstation>, StoreError> {
        let query = format!(
            "SELECT {WORKSTATION_COLUMNS}
             FROM workstations w
             JOIN workstation_grants g ON g.workstation_id = w.id
             WHERE g.user_id = ? AND w.active = 1
             ORDER BY w.name COLLATE NOCASE"
        );
        let rows = sqlx::query(&query)
            .bind(user.id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(workstation_from_row).collect()
    }

    pub async fn list_workstations_with_presence(
        &self,
        user: &User,
    ) -> Result<Vec<WorkstationPresence>, StoreError> {
        let query = format!(
            "SELECT {WORKSTATION_COLUMNS}, {WORKSTATION_PRESENCE_COLUMNS}
             FROM workstations w
             JOIN workstation_grants g ON g.workstation_id = w.id
             WHERE g.user_id = ? AND w.active = 1
             ORDER BY w.name COLLATE NOCASE"
        );
        let rows = sqlx::query(&query)
            .bind(user.id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(workstation_presence_from_row).collect()
    }

    pub async fn create_lan_connection(
        &self,
        user: &User,
        workstation_id: Uuid,
        offline_cutoff: i64,
        kyber_token: &str,
        token_expires_at: i64,
        now: i64,
    ) -> Result<(Uuid, Workstation), StoreError> {
        let presence = self
            .authorized_workstation_presence(user, workstation_id)
            .await?
            .ok_or(StoreError::AccessDenied)?;
        if presence
            .last_seen_at
            .is_none_or(|last_seen| last_seen < offline_cutoff)
        {
            return Err(StoreError::HostOffline);
        }
        let session_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO connection_sessions (
                id, user_id, workstation_id, knock_hash, status,
                ticket_token, ticket_issued_at, ticket_expires_at,
                created_at, knock_expires_at, lease_expires_at
             ) VALUES (?, ?, ?, ?, 'ready', ?, ?, ?, ?, ?, ?)",
        )
        .bind(session_id.to_string())
        .bind(user.id.to_string())
        .bind(workstation_id.to_string())
        .bind(token_digest(&random_secret()).as_slice())
        .bind(kyber_token)
        .bind(now)
        .bind(token_expires_at)
        .bind(now)
        .bind(now)
        .bind(token_expires_at)
        .execute(&self.pool)
        .await?;
        Ok((session_id, presence.workstation))
    }

    pub async fn create_pending_connection(
        &self,
        user: &User,
        workstation_id: Uuid,
        now: i64,
    ) -> Result<(PendingConnection, String), StoreError> {
        let workstation = self
            .authorized_workstation(user, workstation_id)
            .await?
            .ok_or(StoreError::AccessDenied)?;
        let id = Uuid::new_v4();
        let secret = random_secret();
        let knock_expires_at = now + KNOCK_TTL_SECONDS;
        let lease_expires_at = now + LEASE_TTL_SECONDS;
        sqlx::query(
            "INSERT INTO connection_sessions (
                id, user_id, workstation_id, knock_hash, status,
                created_at, knock_expires_at, lease_expires_at
             ) VALUES (?, ?, ?, ?, 'pending', ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(user.id.to_string())
        .bind(workstation.id.to_string())
        .bind(token_digest(&secret).as_slice())
        .bind(now)
        .bind(knock_expires_at)
        .bind(lease_expires_at)
        .execute(&self.pool)
        .await?;
        Ok((
            PendingConnection {
                id,
                user_id: user.id,
                workstation,
                knock_expires_at,
                lease_expires_at,
            },
            URL_SAFE_NO_PAD.encode(secret),
        ))
    }

    pub async fn authorize_knock(
        &self,
        session_id: Uuid,
        secret: &[u8],
        now: i64,
    ) -> Result<Option<KnockAuthorization>, StoreError> {
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE connection_sessions
             SET status = 'opening_lease'
             WHERE id = ?
               AND knock_hash = ?
               AND status = 'pending'
               AND knock_expires_at >= ?",
        )
        .bind(session_id.to_string())
        .bind(token_digest(secret).as_slice())
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(None);
        }
        let query = format!(
            "SELECT c.id, c.user_id, c.lease_expires_at, {WORKSTATION_COLUMNS}
             FROM connection_sessions c
             JOIN users u ON u.id = c.user_id
             JOIN workstations w ON w.id = c.workstation_id
             WHERE c.id = ?
               AND u.active = 1
               AND w.active = 1
               AND EXISTS (
                   SELECT 1
                   FROM workstation_grants g
                   WHERE g.user_id = c.user_id
                     AND g.workstation_id = c.workstation_id
               )"
        );
        let row = sqlx::query(&query)
            .bind(session_id.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
        let Some(row) = row else {
            sqlx::query(
                "UPDATE connection_sessions
                 SET status = 'failed', failure_reason = 'principal or workstation disabled'
                 WHERE id = ?",
            )
            .bind(session_id.to_string())
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            return Ok(None);
        };
        let authorization = KnockAuthorization {
            id: parse_uuid(row.try_get::<String, _>("id")?)?,
            user_id: parse_uuid(row.try_get::<String, _>("user_id")?)?,
            workstation: workstation_from_row(&row)?,
            lease_expires_at: row.try_get("lease_expires_at")?,
        };
        transaction.commit().await?;
        Ok(Some(authorization))
    }

    pub async fn record_admission_lease(
        &self,
        session_id: Uuid,
        source_ipv4: Ipv4Addr,
        lease_id: &str,
    ) -> Result<(), StoreError> {
        let result = sqlx::query(
            "UPDATE connection_sessions
             SET source_ipv4 = ?, admission_lease_id = ?
             WHERE id = ? AND status = 'opening_lease'",
        )
        .bind(source_ipv4.to_string())
        .bind(lease_id)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn mark_connection_ready(
        &self,
        session_id: Uuid,
        source_ipv4: Ipv4Addr,
        lease_id: &str,
        ticket_jti: Uuid,
        ticket_token: &str,
        ticket_issued_at: i64,
        ticket_expires_at: i64,
    ) -> Result<(), StoreError> {
        let result = sqlx::query(
            "UPDATE connection_sessions
             SET status = 'ready',
                 source_ipv4 = ?,
                 admission_lease_id = ?,
                 ticket_jti = ?,
                 ticket_token = ?,
                 ticket_issued_at = ?,
                 ticket_expires_at = ?,
                 failure_reason = NULL
             WHERE id = ? AND status = 'opening_lease'",
        )
        .bind(source_ipv4.to_string())
        .bind(lease_id)
        .bind(ticket_jti.to_string())
        .bind(ticket_token)
        .bind(ticket_issued_at)
        .bind(ticket_expires_at)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub async fn mark_connection_failed(
        &self,
        session_id: Uuid,
        reason: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE connection_sessions
             SET status = 'failed', failure_reason = ?
             WHERE id = ? AND status IN ('pending', 'opening_lease')",
        )
        .bind(reason)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn connection_for_user(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ConnectionSession>, StoreError> {
        let query = format!(
            "SELECT c.*, {WORKSTATION_COLUMNS}
             FROM connection_sessions c
             JOIN workstations w ON w.id = c.workstation_id
             WHERE c.id = ? AND c.user_id = ?"
        );
        let row = sqlx::query(&query)
            .bind(session_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| connection_from_row(&row)).transpose()
    }

    pub async fn close_connection(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, StoreError> {
        let row = sqlx::query(
            "SELECT status, admission_lease_id
             FROM connection_sessions
             WHERE id = ? AND user_id = ?",
        )
        .bind(session_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Err(StoreError::NotFound);
        };
        let status: String = row.try_get("status")?;
        if !matches!(status.as_str(), "pending" | "opening_lease" | "ready") {
            return Ok(None);
        }
        Ok(row.try_get("admission_lease_id")?)
    }

    pub async fn mark_closed(&self, session_id: Uuid, now: i64) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE connection_sessions
             SET status = 'closed', closed_at = ?
             WHERE id = ? AND status IN ('pending', 'opening_lease', 'ready')",
        )
        .bind(now)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn expired_leases(&self, now: i64) -> Result<Vec<ExpiredLease>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, admission_lease_id
             FROM connection_sessions
             WHERE status = 'ready'
               AND lease_expires_at <= ?
               AND admission_lease_id IS NOT NULL",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ExpiredLease {
                    session_id: parse_uuid(row.try_get::<String, _>("id")?)?,
                    lease_id: row.try_get("admission_lease_id")?,
                })
            })
            .collect()
    }

    pub async fn revoked_leases(&self) -> Result<Vec<ExpiredLease>, StoreError> {
        let rows = sqlx::query(
            "SELECT c.id, c.admission_lease_id
             FROM connection_sessions c
             JOIN users u ON u.id = c.user_id
             JOIN workstations w ON w.id = c.workstation_id
             WHERE c.status = 'ready'
               AND c.admission_lease_id IS NOT NULL
               AND (
                   u.active = 0
                   OR w.active = 0
                   OR NOT EXISTS (
                       SELECT 1
                       FROM workstation_grants g
                       WHERE g.user_id = c.user_id
                         AND g.workstation_id = c.workstation_id
                   )
               )",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ExpiredLease {
                    session_id: parse_uuid(row.try_get::<String, _>("id")?)?,
                    lease_id: row.try_get("admission_lease_id")?,
                })
            })
            .collect()
    }

    pub async fn orphaned_leases(&self) -> Result<Vec<ExpiredLease>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, admission_lease_id
             FROM connection_sessions
             WHERE status = 'opening_lease'
               AND admission_lease_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ExpiredLease {
                    session_id: parse_uuid(row.try_get::<String, _>("id")?)?,
                    lease_id: row.try_get("admission_lease_id")?,
                })
            })
            .collect()
    }

    pub async fn stale_opening_leases(&self, now: i64) -> Result<Vec<ExpiredLease>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, admission_lease_id
             FROM connection_sessions
             WHERE status = 'opening_lease'
               AND knock_expires_at <= ?
               AND admission_lease_id IS NOT NULL",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ExpiredLease {
                    session_id: parse_uuid(row.try_get::<String, _>("id")?)?,
                    lease_id: row.try_get("admission_lease_id")?,
                })
            })
            .collect()
    }

    pub async fn mark_expired(&self, session_id: Uuid, now: i64) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE connection_sessions
             SET status = 'expired', closed_at = ?
             WHERE id = ? AND status = 'ready'",
        )
        .bind(now)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn authorized_workstation(
        &self,
        user: &User,
        workstation_id: Uuid,
    ) -> Result<Option<Workstation>, StoreError> {
        let query = format!(
            "SELECT {WORKSTATION_COLUMNS}
             FROM workstations w
             JOIN workstation_grants g ON g.workstation_id = w.id
             WHERE w.id = ? AND g.user_id = ? AND w.active = 1"
        );
        let row = sqlx::query(&query)
            .bind(workstation_id.to_string())
            .bind(user.id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| workstation_from_row(&row)).transpose()
    }

    async fn authorized_workstation_presence(
        &self,
        user: &User,
        workstation_id: Uuid,
    ) -> Result<Option<WorkstationPresence>, StoreError> {
        let query = format!(
            "SELECT {WORKSTATION_COLUMNS}, {WORKSTATION_PRESENCE_COLUMNS}
             FROM workstations w
             JOIN workstation_grants g ON g.workstation_id = w.id
             WHERE w.id = ? AND g.user_id = ? AND w.active = 1"
        );
        let row = sqlx::query(&query)
            .bind(workstation_id.to_string())
            .bind(user.id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| workstation_presence_from_row(&row))
            .transpose()
    }

    async fn workstation_presence(
        &self,
        workstation_id: Uuid,
    ) -> Result<Option<WorkstationPresence>, StoreError> {
        let query = format!(
            "SELECT {WORKSTATION_COLUMNS}, {WORKSTATION_PRESENCE_COLUMNS}
             FROM workstations w WHERE w.id = ?"
        );
        let row = sqlx::query(&query)
            .bind(workstation_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| workstation_presence_from_row(&row))
            .transpose()
    }
}

#[cfg(unix)]
fn secure_database_file(database_url: &str, path: &std::path::Path) -> Result<(), std::io::Error> {
    use std::{
        fs::{self, OpenOptions, Permissions},
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };

    if database_url.contains(":memory:") || database_url.contains("mode=memory") {
        return Ok(());
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(std::io::Error::other("SQLite path is not a regular file"));
            }
            fs::set_permissions(path, Permissions::from_mode(0o600))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?;
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

#[cfg(not(unix))]
fn secure_database_file(
    _database_url: &str,
    _path: &std::path::Path,
) -> Result<(), std::io::Error> {
    Ok(())
}

fn new_api_tokens(user_id: Uuid, now: i64) -> ApiTokens {
    ApiTokens {
        session: ApiSession {
            id: Uuid::new_v4(),
            user_id,
            access_expires_at: now + ACCESS_TOKEN_TTL_SECONDS,
            refresh_expires_at: now + REFRESH_TOKEN_TTL_SECONDS,
        },
        access_token: URL_SAFE_NO_PAD.encode(random_secret()),
        refresh_token: URL_SAFE_NO_PAD.encode(random_secret()),
    }
}

async fn insert_api_session(
    pool: &SqlitePool,
    tokens: &ApiTokens,
    now: i64,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO api_sessions (
            id, user_id, access_hash, access_expires_at,
            refresh_hash, refresh_expires_at, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tokens.session.id.to_string())
    .bind(tokens.session.user_id.to_string())
    .bind(token_digest(tokens.access_token.as_bytes()).as_slice())
    .bind(tokens.session.access_expires_at)
    .bind(token_digest(tokens.refresh_token.as_bytes()).as_slice())
    .bind(tokens.session.refresh_expires_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

fn random_secret() -> [u8; 32] {
    let mut secret = [0_u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

fn token_digest(token: &[u8]) -> [u8; 32] {
    Sha256::digest(token).into()
}

fn parse_uuid(value: impl AsRef<str>) -> Result<Uuid, StoreError> {
    Uuid::parse_str(value.as_ref()).map_err(|error| StoreError::InvalidData(error.to_string()))
}

fn user_from_row(row: &SqliteRow) -> Result<User, StoreError> {
    let role_value: String = row.try_get("role")?;
    let role = Role::parse(&role_value)
        .ok_or_else(|| StoreError::InvalidData(format!("unknown role {role_value}")))?;
    Ok(User {
        id: parse_uuid(row.try_get::<String, _>("id")?)?,
        username: row.try_get("username")?,
        password_hash: row.try_get("password_hash")?,
        role,
        active: row.try_get::<i64, _>("active")? != 0,
    })
}

fn workstation_from_row(row: &SqliteRow) -> Result<Workstation, StoreError> {
    let kymux_port: i64 = row.try_get("kymux_port")?;
    let wan_port: i64 = row.try_get("wan_port")?;
    Ok(Workstation {
        id: parse_uuid(row.try_get::<String, _>("workstation_id")?)?,
        name: row.try_get("workstation_name")?,
        lan_ipv4: row.try_get::<String, _>("lan_ipv4")?.parse().map_err(
            |error: std::net::AddrParseError| StoreError::InvalidData(error.to_string()),
        )?,
        kymux_port: u16::try_from(kymux_port)
            .map_err(|error| StoreError::InvalidData(error.to_string()))?,
        wan_port: u16::try_from(wan_port)
            .map_err(|error| StoreError::InvalidData(error.to_string()))?,
        certificate_sha256: row.try_get("certificate_sha256")?,
        active: row.try_get::<i64, _>("workstation_active")? != 0,
    })
}

fn workstation_presence_from_row(row: &SqliteRow) -> Result<WorkstationPresence, StoreError> {
    let registration_id = row
        .try_get::<Option<String>, _>("registration_id")?
        .map(parse_uuid)
        .transpose()?;
    Ok(WorkstationPresence {
        workstation: workstation_from_row(row)?,
        registration_id,
        hostname: row.try_get("registered_hostname")?,
        agent_version: row.try_get("agent_version")?,
        last_seen_at: row.try_get("last_seen_at")?,
    })
}

fn validated_text<'a>(value: &'a str, label: &str) -> Result<&'a str, StoreError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(StoreError::InvalidData(format!(
            "{label} must contain between 1 and 128 non-control characters"
        )));
    }
    Ok(value)
}

fn validated_fingerprint(value: &str) -> Result<String, StoreError> {
    let value = value.to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StoreError::InvalidData(
            "certificate fingerprint must be 64 hexadecimal characters".to_owned(),
        ));
    }
    Ok(value)
}

fn connection_from_row(row: &SqliteRow) -> Result<ConnectionSession, StoreError> {
    let status_value: String = row.try_get("status")?;
    let status = ConnectionStatus::parse(&status_value)
        .ok_or_else(|| StoreError::InvalidData(format!("unknown status {status_value}")))?;
    let source_ipv4 = row
        .try_get::<Option<String>, _>("source_ipv4")?
        .map(|value| value.parse())
        .transpose()
        .map_err(|error: std::net::AddrParseError| StoreError::InvalidData(error.to_string()))?;
    let ticket_jti = row
        .try_get::<Option<String>, _>("ticket_jti")?
        .map(parse_uuid)
        .transpose()?;
    Ok(ConnectionSession {
        id: parse_uuid(row.try_get::<String, _>("id")?)?,
        user_id: parse_uuid(row.try_get::<String, _>("user_id")?)?,
        workstation: workstation_from_row(row)?,
        status,
        source_ipv4,
        admission_lease_id: row.try_get("admission_lease_id")?,
        ticket_jti,
        kymux_token: row.try_get("ticket_token")?,
        ticket_issued_at: row.try_get("ticket_issued_at")?,
        ticket_expires_at: row.try_get("ticket_expires_at")?,
        failure_reason: row.try_get("failure_reason")?,
        knock_expires_at: row.try_get("knock_expires_at")?,
        lease_expires_at: row.try_get("lease_expires_at")?,
    })
}
