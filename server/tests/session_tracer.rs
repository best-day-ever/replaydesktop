use std::{net::Ipv4Addr, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use ed25519_dalek::{
    SigningKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding},
};
use rand_core_06::OsRng;
use replay_control::{
    admission::MemoryAdmission,
    api::AppState,
    build_router,
    domain::Role,
    knock::{KnockService, make_knock_packet, unix_time},
    password::hash_password,
    ticket::{TicketIssuer, TicketVerifier},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: axum::Router,
    store: replay_control::Store,
    knock: Arc<KnockService>,
    admission: MemoryAdmission,
    verifier: TicketVerifier,
    workstation_id: Uuid,
}

async fn fixture() -> Fixture {
    let store = replay_control::Store::connect("sqlite::memory:")
        .await
        .expect("database");
    let now = unix_time();
    let password_hash = hash_password("correct horse battery staple").expect("password hash");
    store
        .create_user("finn", &password_hash, Role::User, now)
        .await
        .expect("user");
    let workstation = store
        .create_workstation(
            "render-01",
            Ipv4Addr::new(192, 168, 20, 41),
            47990,
            47990,
            "ab".repeat(32).as_str(),
            now,
        )
        .await
        .expect("workstation");
    store
        .grant_workstation("finn", "render-01", now)
        .await
        .expect("grant");

    let signing_key = SigningKey::generate(&mut OsRng);
    let private_pem = signing_key
        .to_pkcs8_pem(LineEnding::LF)
        .expect("private pem");
    let public_pem = signing_key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("public pem");
    let issuer =
        TicketIssuer::from_pem(private_pem.as_bytes(), "replay.test", "test-key").expect("issuer");
    let verifier = TicketVerifier::from_pem(public_pem.as_bytes(), "replay.test", "test-key")
        .expect("verifier");
    let admission = MemoryAdmission::default();
    let knock = Arc::new(KnockService::new(
        store.clone(),
        Arc::new(admission.clone()),
        issuer,
    ));
    let app = build_router(AppState::new(
        store.clone(),
        Arc::clone(&knock),
        "desktop.example.test",
        "desktop.example.test:8444",
    ));
    Fixture {
        app,
        store,
        knock,
        admission,
        verifier,
        workstation_id: workstation.id,
    }
}

async fn json_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
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
async fn login_knock_direct_ticket_and_close_trace() {
    let fixture = fixture().await;
    let (status, login) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "finn",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let access = login["access_token"].as_str().expect("access token");

    let (status, workstations) = json_request(
        &fixture.app,
        "GET",
        "/v1/workstations",
        Some(access),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(workstations[0]["name"], "render-01");

    let uri = format!("/v1/workstations/{}/sessions", fixture.workstation_id);
    let (status, pending) =
        json_request(&fixture.app, "POST", &uri, Some(access), Value::Null).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(pending["status"], "pending");
    let session_id =
        Uuid::parse_str(pending["session_id"].as_str().expect("session id")).expect("session uuid");
    let packet = make_knock_packet(
        session_id,
        pending["knock_token"].as_str().expect("knock token"),
    )
    .expect("knock packet");
    let source = Ipv4Addr::new(203, 0, 113, 24);
    let mut wrong_packet = packet.clone();
    *wrong_packet.last_mut().expect("last byte") ^= 1;
    assert!(
        !fixture
            .knock
            .process_packet(&wrong_packet, source, unix_time())
            .await
            .expect("reject wrong knock")
    );
    assert!(
        fixture
            .knock
            .process_packet(&packet, source, unix_time())
            .await
            .expect("process knock")
    );
    assert!(
        !fixture
            .knock
            .process_packet(&packet, source, unix_time())
            .await
            .expect("reject replayed knock")
    );

    let uri = format!("/v1/sessions/{session_id}");
    let (status, ready) = json_request(&fixture.app, "GET", &uri, Some(access), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["direct_endpoint"], "desktop.example.test:47990");
    assert_eq!(ready["workstation_certificate_sha256"], "ab".repeat(32));
    let kymux_token = ready["kymux_token"].as_str().expect("Kymux token");
    let claims = fixture
        .verifier
        .verify(kymux_token, fixture.workstation_id, source)
        .expect("verified ticket");
    assert_eq!(claims.sid, session_id.to_string());
    assert_eq!(
        fixture
            .admission
            .active_leases()
            .expect("active leases")
            .len(),
        1
    );

    let (status, _) = json_request(&fixture.app, "DELETE", &uri, Some(access), Value::Null).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = json_request(&fixture.app, "DELETE", &uri, Some(access), Value::Null).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        fixture
            .admission
            .active_leases()
            .expect("active leases")
            .is_empty()
    );
    assert_eq!(
        fixture
            .admission
            .closed_lease_ids()
            .expect("closed leases")
            .len(),
        1
    );
}

#[tokio::test]
async fn a_revoked_grant_invalidates_an_already_issued_knock() {
    let fixture = fixture().await;
    let (_, login) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "finn",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    let access = login["access_token"].as_str().expect("access token");
    let uri = format!("/v1/workstations/{}/sessions", fixture.workstation_id);
    let (_, pending) = json_request(&fixture.app, "POST", &uri, Some(access), Value::Null).await;
    fixture
        .store
        .revoke_workstation("finn", "render-01")
        .await
        .expect("revoke");
    let session_id =
        Uuid::parse_str(pending["session_id"].as_str().expect("session id")).expect("session uuid");
    let packet = make_knock_packet(
        session_id,
        pending["knock_token"].as_str().expect("knock token"),
    )
    .expect("knock packet");
    assert!(
        !fixture
            .knock
            .process_packet(&packet, Ipv4Addr::new(203, 0, 113, 24), unix_time())
            .await
            .expect("rejected knock")
    );
    assert!(
        fixture
            .admission
            .active_leases()
            .expect("active leases")
            .is_empty()
    );
}

#[tokio::test]
async fn ungranted_user_cannot_create_a_connection() {
    let fixture = fixture().await;
    let now = unix_time();
    let password_hash = hash_password("another excellent password").expect("password hash");
    fixture
        .store
        .create_user("outsider", &password_hash, Role::User, now)
        .await
        .expect("user");
    let (status, login) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "outsider",
            "password": "another excellent password"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let access = login["access_token"].as_str().expect("access token");
    let uri = format!("/v1/workstations/{}/sessions", fixture.workstation_id);
    let (status, _) = json_request(&fixture.app, "POST", &uri, Some(access), Value::Null).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn disabling_a_user_revokes_login_and_existing_access_tokens() {
    let fixture = fixture().await;
    let (_, login) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "finn",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    let access = login["access_token"].as_str().expect("access token");
    fixture
        .store
        .disable_user("finn", unix_time())
        .await
        .expect("disable user");
    let (status, _) = json_request(
        &fixture.app,
        "GET",
        "/v1/workstations",
        Some(access),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "finn",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_tokens_rotate_and_cannot_be_replayed() {
    let fixture = fixture().await;
    let (_, login) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/login",
        None,
        json!({
            "username": "finn",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    let refresh = login["refresh_token"].as_str().expect("refresh token");
    let (status, rotated) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/refresh",
        None,
        json!({"refresh_token": refresh}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(rotated["refresh_token"], refresh);
    let (status, _) = json_request(
        &fixture.app,
        "POST",
        "/v1/auth/refresh",
        None,
        json!({"refresh_token": refresh}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
