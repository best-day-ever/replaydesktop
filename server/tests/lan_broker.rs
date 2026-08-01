use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use ed25519_dalek::{
    SigningKey,
    pkcs8::{EncodePrivateKey as _, spki::der::pem::LineEnding},
};
use rand_core_06::OsRng;
use replay_control::{
    admission::MemoryAdmission,
    api::AppState,
    build_router,
    domain::Role,
    knock::{KnockService, unix_time},
    kyber_jwt::{KyberJwtIssuer, KyberJwtVerifier},
    password::hash_password,
    ticket::TicketIssuer,
};
use rsa::pkcs8::EncodePublicKey as _;
use serde_json::{Value, json};
use tower::ServiceExt;

const HOST_TOKEN: &[u8] = b"local-test-host-token-32-bytes-minimum";

async fn json_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    host_token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(token) = host_token {
        builder = builder.header("x-replay-host-token", token);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response bytes");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response json")
    };
    (status, body)
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn heartbeat_login_discovery_and_direct_jwt_trace() {
    let store = replay_control::Store::connect("sqlite::memory:")
        .await
        .expect("database");
    let password_hash = hash_password("correct horse battery staple").expect("password hash");
    store
        .create_user("finn", &password_hash, Role::Admin, unix_time())
        .await
        .expect("user");

    let ticket_key = SigningKey::generate(&mut OsRng);
    let ticket_private = ticket_key
        .to_pkcs8_pem(LineEnding::LF)
        .expect("ticket private key");
    let ticket_issuer =
        TicketIssuer::from_pem(ticket_private.as_bytes(), "replay.test", "test-key")
            .expect("ticket issuer");
    let knock = Arc::new(KnockService::new(
        store.clone(),
        Arc::new(MemoryAdmission::default()),
        ticket_issuer,
    ));

    let rsa_private = rsa::RsaPrivateKey::new(&mut OsRng, 2048).expect("RSA private key");
    let rsa_private_pem = rsa_private
        .to_pkcs8_pem(LineEnding::LF)
        .expect("RSA private PEM");
    let rsa_public_pem = rsa_private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("RSA public PEM");
    let lan_issuer = KyberJwtIssuer::from_pem(rsa_private_pem.as_bytes()).expect("LAN issuer");
    let verifier = KyberJwtVerifier::from_pem(rsa_public_pem.as_bytes()).expect("LAN verifier");
    let app = build_router(
        AppState::new(
            store.clone(),
            Arc::clone(&knock),
            "unused.example.test",
            "unused.example.test:8444",
        )
        .with_lan_mode(lan_issuer.clone(), HOST_TOKEN, 45),
    );

    let host_id = uuid::Uuid::new_v4();
    let registration = json!({
        "registration_id": host_id,
        "name": "archbae1337",
        "hostname": "archbae1337",
        "lan_ipv4": "192.168.33.42",
        "kymux_port": 8080,
        "certificate_sha256": "ab".repeat(32),
        "agent_version": "test"
    });
    let (status, _) = json_request(
        &app,
        "POST",
        "/v1/hosts/register",
        None,
        Some("wrong-token-wrong-token-wrong-token"),
        registration.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, registered) = json_request(
        &app,
        "POST",
        "/v1/hosts/register",
        None,
        Some(std::str::from_utf8(HOST_TOKEN).expect("host token")),
        registration,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(registered["online"], true);
    let workstation_id = registered["id"].as_str().expect("workstation id");

    let (status, login) = json_request(
        &app,
        "POST",
        "/v1/auth/login",
        None,
        None,
        json!({"username": "finn", "password": "correct horse battery staple"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let access = login["access_token"].as_str().expect("access token");
    let (status, workstations) = json_request(
        &app,
        "GET",
        "/v1/workstations",
        Some(access),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(workstations.as_array().map(Vec::len), Some(1));
    assert_eq!(workstations[0]["online"], true);

    let uri = format!("/v1/workstations/{workstation_id}/lan-sessions");
    let (status, session) = json_request(&app, "POST", &uri, Some(access), None, Value::Null).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(session["status"], "ready");
    assert_eq!(session["direct_endpoint"], "192.168.33.42:8080");
    assert_eq!(session["workstation_certificate_sha256"], "ab".repeat(32));
    let claims = verifier
        .verify(session["kyber_token"].as_str().expect("Kyber token"))
        .expect("verified Kyber token");
    assert_eq!(claims.aud, "kyber");
    assert_eq!(claims.sub, "finn");

    let stale_app = build_router(
        AppState::new(
            store,
            knock,
            "unused.example.test",
            "unused.example.test:8444",
        )
        .with_lan_mode(lan_issuer, HOST_TOKEN, -1),
    );
    let (status, _) = json_request(&stale_app, "POST", &uri, Some(access), None, Value::Null).await;
    assert_eq!(status, StatusCode::CONFLICT);
}
