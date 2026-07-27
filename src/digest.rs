use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Sha256DigestV1([u8; 32]);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DigestParseError {
    Length { actual: usize },
    NonLowercaseHex { index: usize },
}

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
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Display for DigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length { actual } => {
                write!(
                    formatter,
                    "SHA-256 digest must be 64 lowercase hexadecimal bytes, got {actual}"
                )
            }
            Self::NonLowercaseHex { index } => {
                write!(
                    formatter,
                    "SHA-256 digest contains non-lowercase-hex byte at index {index}"
                )
            }
        }
    }
}

impl std::error::Error for DigestParseError {}

impl FromStr for Sha256DigestV1 {
    type Err = DigestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(DigestParseError::Length {
                actual: value.len(),
            });
        }

        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_lower_hex(pair[0])
                .ok_or(DigestParseError::NonLowercaseHex { index: index * 2 })?;
            let low = decode_lower_hex(pair[1]).ok_or(DigestParseError::NonLowercaseHex {
                index: index * 2 + 1,
            })?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

const fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

impl Serialize for Sha256DigestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha256DigestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

pub fn sha256_bytes(bytes: &[u8]) -> Sha256DigestV1 {
    let output = Sha256::digest(bytes);
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&output);
    Sha256DigestV1(digest)
}

pub fn sha256_reader<R: Read>(mut reader: R) -> io::Result<Sha256DigestV1> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    let output = hasher.finalize();
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&output);
    Ok(Sha256DigestV1(digest))
}

pub fn sha256_file<P: AsRef<Path>>(path: P) -> io::Result<Sha256DigestV1> {
    sha256_reader(File::open(path)?)
}
