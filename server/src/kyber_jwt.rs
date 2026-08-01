use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const KYBER_JWT_TTL_SECONDS: i64 = 12 * 60 * 60;
const MAX_KYBER_JWT_BYTES: usize = 4096;

#[derive(Clone)]
pub struct KyberJwtIssuer {
    encoding_key: EncodingKey,
}

pub struct KyberJwtVerifier {
    decoding_key: DecodingKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KyberClaims {
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub sub: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedKyberJwt {
    pub token: String,
    pub expires_at: i64,
}

#[derive(Debug, Error)]
pub enum KyberJwtError {
    #[error("invalid RSA key")]
    InvalidKey,
    #[error("Kyber JWT signing failed")]
    Sign,
    #[error("Kyber JWT exceeds the client token limit")]
    TooLarge,
    #[error("Kyber JWT verification failed")]
    Verify,
}

impl KyberJwtIssuer {
    pub fn from_pem(private_key_pem: &[u8]) -> Result<Self, KyberJwtError> {
        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key_pem)
                .map_err(|_| KyberJwtError::InvalidKey)?,
        })
    }

    pub fn issue(&self, subject: &str, now: i64) -> Result<IssuedKyberJwt, KyberJwtError> {
        let expires_at = now + KYBER_JWT_TTL_SECONDS;
        let claims = KyberClaims {
            aud: "kyber".to_owned(),
            iat: now,
            exp: expires_at,
            sub: subject.to_owned(),
        };
        let token = encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|_| KyberJwtError::Sign)?;
        if token.len() > MAX_KYBER_JWT_BYTES {
            return Err(KyberJwtError::TooLarge);
        }
        Ok(IssuedKyberJwt { token, expires_at })
    }
}

impl KyberJwtVerifier {
    pub fn from_pem(public_key_pem: &[u8]) -> Result<Self, KyberJwtError> {
        Ok(Self {
            decoding_key: DecodingKey::from_rsa_pem(public_key_pem)
                .map_err(|_| KyberJwtError::InvalidKey)?,
        })
    }

    pub fn verify(&self, token: &str) -> Result<KyberClaims, KyberJwtError> {
        if token.len() > MAX_KYBER_JWT_BYTES {
            return Err(KyberJwtError::TooLarge);
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_required_spec_claims(&["aud", "exp", "sub"]);
        validation.set_audience(&["kyber"]);
        validation.leeway = 0;
        decode::<KyberClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| KyberJwtError::Verify)
    }
}
