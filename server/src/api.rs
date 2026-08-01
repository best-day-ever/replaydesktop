use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::{
        ConnectionResponse, ConnectionStatus, CreateConnectionResponse, LoginRequest,
        LoginResponse, RefreshRequest, User, WorkstationSummary,
    },
    knock::{KnockError, KnockService, unix_time},
    password::verify_password,
    store::{ApiTokens, Store, StoreError},
};

const MAX_LOGIN_ATTEMPTS: usize = 5;
const MAX_GLOBAL_LOGIN_ATTEMPTS: usize = 100;
const LOGIN_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub knock: Arc<KnockService>,
    pub public_host: String,
    pub knock_endpoint: String,
    login_limiter: LoginLimiter,
}

impl AppState {
    pub fn new(
        store: Store,
        knock: Arc<KnockService>,
        public_host: impl Into<String>,
        knock_endpoint: impl Into<String>,
    ) -> Self {
        let _dummy_password_hash = dummy_password_hash();
        Self {
            store,
            knock,
            public_host: public_host.into(),
            knock_endpoint: knock_endpoint.into(),
            login_limiter: LoginLimiter::default(),
        }
    }
}

#[derive(Clone, Default)]
struct LoginLimiter {
    attempts: Arc<Mutex<LoginAttempts>>,
}

#[derive(Default)]
struct LoginAttempts {
    by_username: HashMap<String, VecDeque<Instant>>,
    global: VecDeque<Instant>,
}

impl LoginLimiter {
    fn check(&self, username: &str) -> bool {
        let Ok(mut attempts) = self.attempts.lock() else {
            return false;
        };
        let now = Instant::now();
        while attempts
            .global
            .front()
            .is_some_and(|attempt| now.duration_since(*attempt) > LOGIN_WINDOW)
        {
            attempts.global.pop_front();
        }
        if attempts.global.len() >= MAX_GLOBAL_LOGIN_ATTEMPTS {
            return false;
        }
        attempts.global.push_back(now);
        let entries = attempts
            .by_username
            .entry(username.to_ascii_lowercase())
            .or_default();
        while entries
            .front()
            .is_some_and(|attempt| now.duration_since(*attempt) > LOGIN_WINDOW)
        {
            entries.pop_front();
        }
        if entries.len() >= MAX_LOGIN_ATTEMPTS {
            return false;
        }
        entries.push_back(now);
        true
    }

    fn clear(&self, username: &str) {
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.by_username.remove(&username.to_ascii_lowercase());
        }
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    const fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "authentication failed",
        }
    }

    const fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: "access denied",
        }
    }

    const fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "not found",
        }
    }

    const fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error": self.message}))).into_response()
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/workstations", get(list_workstations))
        .route(
            "/v1/workstations/{workstation_id}/sessions",
            post(create_connection),
        )
        .route("/v1/sessions/{session_id}", get(get_connection))
        .route("/v1/sessions/{session_id}", delete(close_connection))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    if !state.login_limiter.check(&request.username) {
        return Err(ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "too many login attempts",
        });
    }
    let user = state
        .store
        .user_by_username(&request.username)
        .await
        .map_err(store_error)?
        .filter(|user| user.active);
    let encoded_password = user.as_ref().map_or_else(
        || dummy_password_hash().to_owned(),
        |user| user.password_hash.clone(),
    );
    let password = request.password;
    let password_matches =
        tokio::task::spawn_blocking(move || verify_password(&password, &encoded_password))
            .await
            .map_err(|error| {
                tracing::error!(%error, "password verification worker failed");
                ApiError::internal()
            })?;
    let Some(user) = user else {
        return Err(ApiError::unauthorized());
    };
    if !password_matches {
        return Err(ApiError::unauthorized());
    }
    state.login_limiter.clear(&request.username);
    let tokens = state
        .store
        .issue_api_session(user.id, unix_time())
        .await
        .map_err(store_error)?;
    Ok(Json(login_response(tokens)))
}

async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let tokens = state
        .store
        .rotate_refresh(&request.refresh_token, unix_time())
        .await
        .map_err(store_error)?
        .ok_or_else(ApiError::unauthorized)?;
    Ok(Json(login_response(tokens)))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    if !state
        .store
        .logout(token, unix_time())
        .await
        .map_err(store_error)?
    {
        return Err(ApiError::unauthorized());
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_workstations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<WorkstationSummary>>, ApiError> {
    let user = authenticated_user(&state.store, &headers).await?;
    let workstations = state
        .store
        .list_workstations(&user)
        .await
        .map_err(store_error)?
        .into_iter()
        .map(WorkstationSummary::from)
        .collect();
    Ok(Json(workstations))
}

async fn create_connection(
    State(state): State<AppState>,
    Path(workstation_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<CreateConnectionResponse>), ApiError> {
    let user = authenticated_user(&state.store, &headers).await?;
    let (pending, knock_token) = state
        .store
        .create_pending_connection(&user, workstation_id, unix_time())
        .await
        .map_err(store_error)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(CreateConnectionResponse {
            session_id: pending.id,
            status: ConnectionStatus::Pending.as_str(),
            knock_endpoint: state.knock_endpoint,
            knock_token,
            knock_expires_at: pending.knock_expires_at,
        }),
    ))
}

async fn get_connection(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ConnectionResponse>, ApiError> {
    let user = authenticated_user(&state.store, &headers).await?;
    let connection = state
        .store
        .connection_for_user(session_id, user.id)
        .await
        .map_err(store_error)?
        .ok_or_else(ApiError::not_found)?;
    let ready = connection.status == ConnectionStatus::Ready;
    let token_is_current = connection
        .ticket_expires_at
        .is_some_and(|expires_at| expires_at >= unix_time());
    Ok(Json(ConnectionResponse {
        session_id,
        status: connection.status.as_str(),
        direct_endpoint: ready
            .then(|| format!("{}:{}", state.public_host, connection.workstation.wan_port)),
        workstation_certificate_sha256: ready
            .then(|| connection.workstation.certificate_sha256.clone()),
        kymux_token: token_is_current.then_some(connection.kymux_token).flatten(),
        lease_expires_at: ready.then_some(connection.lease_expires_at),
        failure_reason: connection.failure_reason.or_else(|| {
            (ready && !token_is_current)
                .then(|| "connection ticket expired; create a new session".to_owned())
        }),
    }))
}

async fn close_connection(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let user = authenticated_user(&state.store, &headers).await?;
    state
        .knock
        .close_for_user(session_id, user.id, unix_time())
        .await
        .map_err(knock_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn login_response(tokens: ApiTokens) -> LoginResponse {
    LoginResponse {
        access_token: tokens.access_token,
        access_expires_at: tokens.session.access_expires_at,
        refresh_token: tokens.refresh_token,
        refresh_expires_at: tokens.session.refresh_expires_at,
    }
}

async fn authenticated_user(store: &Store, headers: &HeaderMap) -> Result<User, ApiError> {
    store
        .authenticate_access(bearer_token(headers)?, unix_time())
        .await
        .map_err(store_error)?
        .ok_or_else(ApiError::unauthorized)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty() && token.len() <= 256)
        .ok_or_else(ApiError::unauthorized)?;
    Ok(value)
}

fn dummy_password_hash() -> &'static str {
    static DUMMY_PASSWORD_HASH: OnceLock<String> = OnceLock::new();
    DUMMY_PASSWORD_HASH
        .get_or_init(|| {
            crate::password::hash_password("ReplayDesktop timing equalizer")
                .expect("fixed Argon2id parameters must be valid")
        })
        .as_str()
}

#[allow(clippy::needless_pass_by_value)]
fn store_error(error: StoreError) -> ApiError {
    match error {
        StoreError::NotFound => ApiError::not_found(),
        StoreError::AccessDenied => ApiError::forbidden(),
        StoreError::Io(_)
        | StoreError::Database(_)
        | StoreError::Migration(_)
        | StoreError::InvalidData(_) => {
            tracing::error!(%error, "API store operation failed");
            ApiError::internal()
        }
    }
}

fn knock_error(error: KnockError) -> ApiError {
    match error {
        KnockError::Store(StoreError::NotFound | StoreError::AccessDenied) => ApiError::not_found(),
        other => {
            tracing::error!(error = %other, "API admission operation failed");
            ApiError::internal()
        }
    }
}
