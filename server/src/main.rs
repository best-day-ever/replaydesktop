use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::{BufReader, Read, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};

use clap::Parser;
use ed25519_dalek::{
    SigningKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding},
};
use rand_core_06::OsRng;
use replay_control::{
    admission::{AdmissionBackend, MemoryAdmission, UnifiAdmission},
    api::AppState,
    build_router,
    config::{
        AdmissionMode, Cli, Command, CreateUserArgs, InitKeysArgs, RegisterHostArgs, ServeArgs,
        UserRole,
    },
    domain::{HostRegistrationRequest, Role},
    knock::{KnockService, unix_time},
    kyber_jwt::KyberJwtIssuer,
    password::{hash_insecure_development_password, hash_password},
    ticket::TicketIssuer,
};
use reqwest::{Client, Url};
use sha2::{Digest, Sha256};
use tokio::{net::UdpSocket, task::JoinHandle};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => serve(&cli.database_url, *args).await?,
        Command::InitKeys(args) => init_keys(&args)?,
        Command::CreateUser(args) => create_user(&cli.database_url, &args).await?,
        Command::DisableUser { username } => {
            let store = replay_control::Store::connect(&cli.database_url).await?;
            store.disable_user(&username, unix_time()).await?;
            info!(%username, "user disabled and API sessions revoked");
        }
        Command::CreateWorkstation(args) => {
            let store = replay_control::Store::connect(&cli.database_url).await?;
            let workstation = store
                .create_workstation(
                    &args.name,
                    args.lan_ipv4,
                    args.kymux_port,
                    args.wan_port,
                    &args.certificate_sha256,
                    unix_time(),
                )
                .await?;
            info!(
                id = %workstation.id,
                name = %workstation.name,
                "workstation created"
            );
        }
        Command::RegisterHost(args) => register_host(&args).await?,
        Command::Grant {
            username,
            workstation,
        } => {
            let store = replay_control::Store::connect(&cli.database_url).await?;
            store
                .grant_workstation(&username, &workstation, unix_time())
                .await?;
            info!(%username, %workstation, "workstation access granted");
        }
        Command::Revoke {
            username,
            workstation,
        } => {
            let store = replay_control::Store::connect(&cli.database_url).await?;
            store.revoke_workstation(&username, &workstation).await?;
            info!(%username, %workstation, "workstation access revoked");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn serve(database_url: &str, args: ServeArgs) -> Result<(), Box<dyn Error>> {
    if !args.http_bind.ip().is_loopback() && (!args.allow_insecure_http || !args.lan_mode) {
        return Err(
            "the HTTP API may bind beyond loopback only in explicitly enabled LAN mode; publish all other deployments through an authenticated TLS proxy"
                .into()
        );
    }
    if !args.http_bind.ip().is_loopback() {
        warn!(
            bind = %args.http_bind,
            "INSECURE LAN DEVELOPMENT HTTP IS ENABLED; do not expose this listener to the internet"
        );
    }
    if args.lan_mode && args.admission_backend != AdmissionMode::Memory {
        return Err(
            "LAN mode requires the memory admission backend so it cannot mutate a firewall".into(),
        );
    }
    let store = replay_control::Store::connect(database_url).await?;
    bootstrap_development_user(&store, &args).await?;
    let private_key = read_private_key(&args.ticket_private_key)?;
    let ticket_issuer =
        TicketIssuer::from_pem(&private_key, args.ticket_issuer, args.ticket_key_id)?;
    let admission: Arc<dyn AdmissionBackend> = match args.admission_backend {
        AdmissionMode::Memory => {
            warn!("using the in-memory admission backend; no real firewall rules will be opened");
            Arc::new(MemoryAdmission::default())
        }
        AdmissionMode::Unifi => {
            let base_url = required(args.unifi_base_url, "REPLAY_UNIFI_BASE_URL")?;
            Arc::new(UnifiAdmission::new(
                &base_url,
                required(args.unifi_site_id, "REPLAY_UNIFI_SITE_ID")?,
                required(args.unifi_api_key, "REPLAY_UNIFI_API_KEY")?,
                required(args.unifi_external_zone_id, "REPLAY_UNIFI_EXTERNAL_ZONE_ID")?,
                required(args.unifi_internal_zone_id, "REPLAY_UNIFI_INTERNAL_ZONE_ID")?,
            )?)
        }
    };
    let knock = Arc::new(KnockService::new(store.clone(), admission, ticket_issuer));
    let reconciled = knock.reconcile_orphaned_leases().await?;
    if reconciled > 0 {
        info!(
            reconciled,
            "closed admission leases left by an interrupted session"
        );
    }
    let mut state = AppState::new(
        store,
        Arc::clone(&knock),
        args.public_host,
        args.knock_endpoint,
    );
    if args.lan_mode {
        let token_path = args
            .host_registration_token_file
            .as_deref()
            .ok_or("REPLAY_HOST_REGISTRATION_TOKEN_FILE is required in LAN mode")?;
        let host_token = read_trimmed_secret(token_path)?;
        if host_token.len() < 32 || host_token.len() > 256 {
            return Err("host registration token must contain between 32 and 256 bytes".into());
        }
        let jwt_path = args
            .kyber_jwt_private_key
            .as_deref()
            .ok_or("REPLAY_KYBER_JWT_PRIVATE_KEY is required in LAN mode")?;
        let jwt_issuer = KyberJwtIssuer::from_pem(&read_private_key(jwt_path)?)?;
        let offline_after = i64::try_from(args.host_offline_after_seconds)?;
        if !(10..=3600).contains(&offline_after) {
            return Err("host offline threshold must be between 10 and 3600 seconds".into());
        }
        state = state.with_lan_mode(jwt_issuer, &host_token, offline_after);
        warn!(
            offline_after,
            "LAN mode enabled: direct sessions bypass UDP proof and firewall admission"
        );
    }
    let app = build_router(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http());
    let http_listener = tokio::net::TcpListener::bind(args.http_bind).await?;
    let udp_socket = UdpSocket::bind(args.knock_bind).await?;
    info!(http = %args.http_bind, knock = %args.knock_bind, "Replay control server listening");

    let mut http_task = tokio::spawn(async move { axum::serve(http_listener, app).await });
    let mut udp_task = tokio::spawn(Arc::clone(&knock).run_udp(udp_socket));
    let mut cleanup_task = spawn_cleanup(knock);
    tokio::select! {
        signal = tokio::signal::ctrl_c() => {
            signal?;
            info!("shutdown requested");
        }
        result = &mut http_task => {
            result??;
            return Err("HTTP server stopped unexpectedly".into());
        }
        result = &mut udp_task => {
            result??;
            return Err("UDP knock service stopped unexpectedly".into());
        }
        result = &mut cleanup_task => {
            result?;
            return Err("lease cleanup task stopped unexpectedly".into());
        }
    }
    http_task.abort();
    udp_task.abort();
    cleanup_task.abort();
    Ok(())
}

async fn bootstrap_development_user(
    store: &replay_control::Store,
    args: &ServeArgs,
) -> Result<(), Box<dyn Error>> {
    let credentials = match (
        args.dev_bootstrap_username.as_deref(),
        args.dev_bootstrap_password.as_deref(),
    ) {
        (None, None) => return Ok(()),
        (Some(username), Some(password)) => (username, password),
        _ => {
            return Err(
                "both REPLAY_DEV_BOOTSTRAP_USERNAME and REPLAY_DEV_BOOTSTRAP_PASSWORD are required"
                    .into(),
            );
        }
    };
    if !args.allow_insecure_dev_bootstrap {
        return Err(
            "development credentials require REPLAY_ALLOW_INSECURE_DEV_BOOTSTRAP=true".into(),
        );
    }
    if !args.lan_mode {
        return Err("development credentials are available only in explicit LAN mode".into());
    }
    if store.user_by_username(credentials.0).await?.is_some() {
        warn!(
            username = credentials.0,
            "development bootstrap user already exists; password was not reset"
        );
        return Ok(());
    }
    let password_hash = hash_insecure_development_password(credentials.1)?;
    store
        .create_user(credentials.0, &password_hash, Role::Admin, unix_time())
        .await?;
    warn!(
        username = credentials.0,
        "INSECURE DEVELOPMENT ACCOUNT CREATED; remove it before any public deployment"
    );
    Ok(())
}

async fn register_host(args: &RegisterHostArgs) -> Result<(), Box<dyn Error>> {
    if args.heartbeat_seconds < 5 || args.heartbeat_seconds > 300 {
        return Err("host heartbeat interval must be between 5 and 300 seconds".into());
    }
    let registration_id = load_or_create_registration_id(&args.registration_id_file)?;
    let token = String::from_utf8(read_trimmed_secret(&args.token_file)?)?;
    if token.len() < 32 || token.len() > 256 {
        return Err("host registration token must contain between 32 and 256 bytes".into());
    }
    let certificate_sha256 = certificate_sha256(&args.certificate_file)?;
    let url = registration_url(&args.broker_url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let request = HostRegistrationRequest {
        registration_id,
        name: args.name.clone(),
        hostname: args.hostname.clone(),
        lan_ipv4: args.lan_ipv4,
        kymux_port: args.kymux_port,
        certificate_sha256,
        agent_version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    let mut interval = tokio::time::interval(Duration::from_secs(args.heartbeat_seconds));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::net::TcpStream::connect((args.lan_ipv4, args.kymux_port)),
                ).await {
                    Ok(Ok(_stream)) => {
                        let response = client
                            .post(url.clone())
                            .header("x-replay-host-token", &token)
                            .json(&request)
                            .send()
                            .await?;
                        if !response.status().is_success() {
                            return Err(format!(
                                "broker rejected host registration with HTTP {}",
                                response.status()
                            ).into());
                        }
                        info!(
                            %registration_id,
                            host = %request.name,
                            endpoint = %format!("{}:{}", request.lan_ipv4, request.kymux_port),
                            "host heartbeat registered"
                        );
                    }
                    Ok(Err(error)) => warn!(%error, "Kyber endpoint is unavailable; heartbeat withheld"),
                    Err(_) => warn!("Kyber endpoint probe timed out; heartbeat withheld"),
                }
            }
            signal = tokio::signal::ctrl_c() => {
                signal?;
                info!("host registrar shutdown requested");
                return Ok(());
            }
        }
    }
}

fn registration_url(value: &str) -> Result<Url, Box<dyn Error>> {
    let mut url = Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("broker URL must be HTTP(S) without credentials, query, or fragment".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url.join("v1/hosts/register")?)
}

fn certificate_sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut reader = BufReader::new(fs::File::open(path)?);
    while let Some(item) = rustls_pemfile::read_one(&mut reader)? {
        if let rustls_pemfile::Item::X509Certificate(certificate) = item {
            return Ok(hex::encode(Sha256::digest(certificate.as_ref())));
        }
    }
    Err(format!("{} does not contain an X.509 certificate", path.display()).into())
}

fn load_or_create_registration_id(path: &Path) -> Result<uuid::Uuid, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(uuid::Uuid::parse_str(value.trim())?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let id = uuid::Uuid::new_v4();
            write_new_secret(path, format!("{id}\n").as_bytes())?;
            Ok(id)
        }
        Err(error) => Err(error.into()),
    }
}

fn read_trimmed_secret(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    Ok(bytes[start..end].to_vec())
}

fn spawn_cleanup(knock: Arc<KnockService>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(error) = knock.expire_leases(unix_time()).await {
                warn!(%error, "lease cleanup pass failed");
            }
        }
    })
}

fn required<T>(value: Option<T>, name: &'static str) -> Result<T, Box<dyn Error>> {
    value.ok_or_else(|| format!("{name} is required with the UniFi admission backend").into())
}

#[cfg(unix)]
fn read_private_key(path: &std::path::Path) -> Result<Vec<u8>, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!(
            "ticket private key {} must be a regular file with mode 0600",
            path.display()
        )
        .into());
    }
    Ok(fs::read(path)?)
}

#[cfg(not(unix))]
fn read_private_key(path: &std::path::Path) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(path)?)
}

fn init_keys(args: &InitKeysArgs) -> Result<(), Box<dyn Error>> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let private_pem = signing_key.to_pkcs8_pem(LineEnding::LF)?;
    let public_pem = signing_key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)?;
    write_new_secret(&args.private_key, private_pem.as_bytes())?;
    if let Err(error) = write_new_secret(&args.public_key, public_pem.as_bytes()) {
        return Err(format!(
            "private key was created at {} but public key creation failed: {error}",
            args.private_key.display()
        )
        .into());
    }
    info!(
        private = %args.private_key.display(),
        public = %args.public_key.display(),
        "Ed25519 ticket keys created"
    );
    Ok(())
}

#[cfg(unix)]
fn write_new_secret(path: &std::path::Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_new_secret(path: &std::path::Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    Ok(())
}

async fn create_user(database_url: &str, args: &CreateUserArgs) -> Result<(), Box<dyn Error>> {
    let password = if args.password_stdin {
        let mut password = String::new();
        std::io::stdin().read_to_string(&mut password)?;
        let password_without_lf = password.strip_suffix('\n').unwrap_or(&password);
        password_without_lf
            .strip_suffix('\r')
            .unwrap_or(password_without_lf)
            .to_owned()
    } else {
        let first = rpassword::prompt_password("Password: ")?;
        let second = rpassword::prompt_password("Confirm password: ")?;
        if first != second {
            return Err("passwords do not match".into());
        }
        first
    };
    let password_hash = hash_password(&password)?;
    let role = match args.role {
        UserRole::Admin => Role::Admin,
        UserRole::User => Role::User,
    };
    let store = replay_control::Store::connect(database_url).await?;
    let user = store
        .create_user(&args.username, &password_hash, role, unix_time())
        .await?;
    info!(id = %user.id, username = %user.username, role = role.as_str(), "user created");
    Ok(())
}
