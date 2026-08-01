use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::{Read, Write},
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
    config::{AdmissionMode, Cli, Command, CreateUserArgs, InitKeysArgs, ServeArgs, UserRole},
    domain::Role,
    knock::{KnockService, unix_time},
    password::hash_password,
    ticket::TicketIssuer,
};
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

async fn serve(database_url: &str, args: ServeArgs) -> Result<(), Box<dyn Error>> {
    if !args.http_bind.ip().is_loopback() {
        return Err(
            "the HTTP API must bind to loopback; publish it through an authenticated TLS proxy"
                .into(),
        );
    }
    let store = replay_control::Store::connect(database_url).await?;
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
    let state = AppState::new(
        store,
        Arc::clone(&knock),
        args.public_host,
        args.knock_endpoint,
    );
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
