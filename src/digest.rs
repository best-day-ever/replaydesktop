use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Sha256DigestV1([u8; 32]);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DigestParseError;

impl Sha256DigestV1 {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for Sha256DigestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for Sha256DigestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("digest-not-implemented")
    }
}

impl fmt::Display for DigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SHA-256 digest parsing is not implemented")
    }
}

impl std::error::Error for DigestParseError {}

impl FromStr for Sha256DigestV1 {
    type Err = DigestParseError;

    fn from_str(_value: &str) -> Result<Self, Self::Err> {
        Err(DigestParseError)
    }
}

impl Serialize for Sha256DigestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("digest-not-implemented")
    }
}

impl<'de> Deserialize<'de> for Sha256DigestV1 {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "SHA-256 digest decoding is not implemented",
        ))
    }
}

pub fn sha256_bytes(_bytes: &[u8]) -> Sha256DigestV1 {
    unimplemented!("RED: SHA-256 byte hashing")
}

pub fn sha256_reader<R: Read>(_reader: R) -> io::Result<Sha256DigestV1> {
    unimplemented!("RED: SHA-256 reader hashing")
}

pub fn sha256_file<P: AsRef<Path>>(_path: P) -> io::Result<Sha256DigestV1> {
    unimplemented!("RED: SHA-256 file hashing")
}
