use std::{collections::HashMap, net::Ipv4Addr, sync::Mutex};

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
    errors::Error as JwtError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::KYMUX_TICKET_TTL_SECONDS;

const MAX_KYMUX_TOKEN_BYTES: usize = 1024;

#[derive(Clone)]
pub struct TicketIssuer {
    encoding_key: EncodingKey,
    issuer: String,
    key_id: String,
}

pub struct TicketVerifier {
    decoding_key: DecodingKey,
    issuer: String,
    key_id: String,
    seen_jti: Mutex<HashMap<Uuid, i64>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TicketClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub sid: String,
    pub src: Ipv4Addr,
    pub jti: String,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedTicket {
    pub token: String,
    pub jti: Uuid,
    pub issued_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Error)]
pub enum TicketError {
    #[error("invalid Ed25519 key")]
    InvalidKey,
    #[error("ticket signing failed")]
    Sign,
    #[error("ticket exceeds the Kymux authentication token limit")]
    TooLarge,
    #[error("ticket verification failed")]
    Verify,
    #[error("ticket source address does not match the peer")]
    SourceMismatch,
    #[error("ticket has already been used")]
    Replay,
    #[error("ticket replay state is unavailable")]
    State,
}

impl TicketIssuer {
    pub fn from_pem(
        private_key_pem: &[u8],
        issuer: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Result<Self, TicketError> {
        Ok(Self {
            encoding_key: EncodingKey::from_ed_pem(private_key_pem)
                .map_err(|_| TicketError::InvalidKey)?,
            issuer: issuer.into(),
            key_id: key_id.into(),
        })
    }

    pub fn issue(
        &self,
        user_id: Uuid,
        workstation_id: Uuid,
        session_id: Uuid,
        source_ipv4: Ipv4Addr,
        now: i64,
    ) -> Result<IssuedTicket, TicketError> {
        let jti = Uuid::new_v4();
        let expires_at = now + KYMUX_TICKET_TTL_SECONDS;
        let claims = TicketClaims {
            iss: self.issuer.clone(),
            sub: user_id.to_string(),
            aud: workstation_id.to_string(),
            sid: session_id.to_string(),
            src: source_ipv4,
            jti: jti.to_string(),
            iat: now,
            nbf: now.saturating_sub(2),
            exp: expires_at,
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("replay-kymux+jwt".to_owned());
        header.kid = Some(self.key_id.clone());
        let token = encode(&header, &claims, &self.encoding_key).map_err(|_| TicketError::Sign)?;
        if token.len() > MAX_KYMUX_TOKEN_BYTES {
            return Err(TicketError::TooLarge);
        }
        Ok(IssuedTicket {
            token,
            jti,
            issued_at: now,
            expires_at,
        })
    }
}

impl TicketVerifier {
    pub fn from_pem(
        public_key_pem: &[u8],
        issuer: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Result<Self, TicketError> {
        Ok(Self {
            decoding_key: DecodingKey::from_ed_pem(public_key_pem)
                .map_err(|_| TicketError::InvalidKey)?,
            issuer: issuer.into(),
            key_id: key_id.into(),
            seen_jti: Mutex::new(HashMap::new()),
        })
    }

    pub fn verify(
        &self,
        token: &str,
        workstation_id: Uuid,
        peer_ipv4: Ipv4Addr,
    ) -> Result<TicketClaims, TicketError> {
        if token.len() > MAX_KYMUX_TOKEN_BYTES {
            return Err(TicketError::TooLarge);
        }
        let header = decode_header(token).map_err(|_| TicketError::Verify)?;
        if header.alg != Algorithm::EdDSA
            || header.kid.as_deref() != Some(self.key_id.as_str())
            || header.typ.as_deref() != Some("replay-kymux+jwt")
        {
            return Err(TicketError::Verify);
        }
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[workstation_id.to_string()]);
        validation.validate_nbf = true;
        validation.leeway = 5;
        let data = decode::<TicketClaims>(token, &self.decoding_key, &validation)
            .map_err(|_error: JwtError| TicketError::Verify)?;
        if data.claims.src != peer_ipv4 {
            return Err(TicketError::SourceMismatch);
        }
        let now = crate::knock::unix_time();
        if data.claims.exp <= data.claims.iat
            || data.claims.exp - data.claims.iat > KYMUX_TICKET_TTL_SECONDS
            || data.claims.nbf > data.claims.iat
            || data.claims.iat > now.saturating_add(5)
        {
            return Err(TicketError::Verify);
        }
        Uuid::parse_str(&data.claims.sub).map_err(|_| TicketError::Verify)?;
        Uuid::parse_str(&data.claims.sid).map_err(|_| TicketError::Verify)?;
        let jti = Uuid::parse_str(&data.claims.jti).map_err(|_| TicketError::Verify)?;
        let mut seen_jti = self.seen_jti.lock().map_err(|_| TicketError::State)?;
        seen_jti.retain(|_, expires_at| *expires_at >= now);
        if seen_jti.insert(jti, data.claims.exp).is_some() {
            return Err(TicketError::Replay);
        }
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{
        SigningKey,
        pkcs8::{EncodePrivateKey, EncodePublicKey, spki::der::pem::LineEnding},
    };
    use rand_core_06::OsRng;

    use super::*;

    #[test]
    fn ticket_binds_workstation_session_and_source() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let private_pem = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("private pem");
        let public_pem = signing_key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("public pem");
        let issuer =
            TicketIssuer::from_pem(private_pem.as_bytes(), "replay.test", "test-key").expect("key");
        let verifier = TicketVerifier::from_pem(public_pem.as_bytes(), "replay.test", "test-key")
            .expect("key");
        let workstation_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let peer = Ipv4Addr::new(203, 0, 113, 8);
        let now = crate::knock::unix_time();
        assert!(matches!(
            verifier.verify(&"x".repeat(MAX_KYMUX_TOKEN_BYTES + 1), workstation_id, peer),
            Err(TicketError::TooLarge)
        ));

        let issued_ticket = issuer
            .issue(Uuid::new_v4(), workstation_id, session_id, peer, now)
            .expect("issue");
        assert!(issued_ticket.token.len() <= MAX_KYMUX_TOKEN_BYTES);
        let claims = verifier
            .verify(&issued_ticket.token, workstation_id, peer)
            .expect("verify");
        assert_eq!(claims.sid, session_id.to_string());
        assert_eq!(claims.src, peer);
        assert!(matches!(
            verifier.verify(
                &issued_ticket.token,
                workstation_id,
                Ipv4Addr::new(203, 0, 113, 9)
            ),
            Err(TicketError::SourceMismatch)
        ));
        assert!(matches!(
            verifier.verify(&issued_ticket.token, workstation_id, peer),
            Err(TicketError::Replay)
        ));
        assert!(matches!(
            verifier.verify(&issued_ticket.token, Uuid::new_v4(), peer),
            Err(TicketError::Verify)
        ));

        let wrong_issuer =
            TicketVerifier::from_pem(public_pem.as_bytes(), "other-issuer", "test-key")
                .expect("wrong issuer verifier");
        assert!(matches!(
            wrong_issuer.verify(&issued_ticket.token, workstation_id, peer),
            Err(TicketError::Verify)
        ));

        let other_signing_key = SigningKey::generate(&mut OsRng);
        let other_public_pem = other_signing_key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("other public pem");
        let wrong_key =
            TicketVerifier::from_pem(other_public_pem.as_bytes(), "replay.test", "test-key")
                .expect("wrong key verifier");
        assert!(matches!(
            wrong_key.verify(&issued_ticket.token, workstation_id, peer),
            Err(TicketError::Verify)
        ));

        let expired_ticket = issuer
            .issue(
                Uuid::new_v4(),
                workstation_id,
                Uuid::new_v4(),
                peer,
                now - 120,
            )
            .expect("expired ticket encoding");
        assert!(matches!(
            verifier.verify(&expired_ticket.token, workstation_id, peer),
            Err(TicketError::Verify)
        ));
    }
}
