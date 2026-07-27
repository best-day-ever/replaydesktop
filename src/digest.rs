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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct ChunkedReader {
        bytes: Vec<u8>,
        position: usize,
        chunks: Vec<usize>,
        next_chunk: usize,
    }

    impl ChunkedReader {
        fn new(bytes: Vec<u8>, chunks: Vec<usize>) -> Self {
            Self {
                bytes,
                position: 0,
                chunks,
                next_chunk: 0,
            }
        }
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.position == self.bytes.len() {
                return Ok(0);
            }

            let requested = self.chunks[self.next_chunk % self.chunks.len()];
            self.next_chunk += 1;
            let remaining = self.bytes.len() - self.position;
            let read = requested.min(remaining).min(buffer.len());
            buffer[..read].copy_from_slice(&self.bytes[self.position..self.position + read]);
            self.position += read;
            Ok(read)
        }
    }

    struct ErrorAfterPrefix {
        prefix: Cursor<Vec<u8>>,
        failed: bool,
    }

    impl Read for ErrorAfterPrefix {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.failed {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected digest reader failure",
                ));
            }

            let read = self.prefix.read(buffer)?;
            if read == 0 {
                self.failed = true;
                return self.read(buffer);
            }
            Ok(read)
        }
    }

    #[test]
    fn digest_known_answer_and_lowercase_format() {
        let digest = sha256_bytes(b"abc");
        assert_eq!(
            digest.to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(digest.as_bytes().len(), 32);
        assert!(
            digest
                .to_string()
                .bytes()
                .all(|byte| { byte.is_ascii_digit() || matches!(byte, b'a'..=b'f') })
        );
        assert!(digest.to_string().parse::<Sha256DigestV1>().is_ok());
        assert!(
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
                .parse::<Sha256DigestV1>()
                .is_err()
        );
    }

    #[test]
    fn digest_empty_input_matches_standard_vector() {
        assert_eq!(
            sha256_bytes(b"").to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_arbitrary_chunks_reader_and_file_equality() {
        let bytes = (0..200_000)
            .map(|index| ((index * 31 + 7) % 251) as u8)
            .collect::<Vec<_>>();
        let expected = sha256_bytes(&bytes);
        let chunked = sha256_reader(ChunkedReader::new(
            bytes.clone(),
            vec![1, 3, 7, 64, 1025, 8191],
        ))
        .expect("chunked reader must hash");
        assert_eq!(chunked, expected);

        let unique = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "replay-host-doctor-digest-{}-{timestamp}-{unique}",
            std::process::id()
        ));
        std::fs::write(&path, &bytes).expect("temporary digest fixture must be writable");
        let file_digest = sha256_file(&path).expect("temporary file must hash");
        std::fs::remove_file(&path).expect("temporary digest fixture must be removable");
        assert_eq!(file_digest, expected);
    }

    #[test]
    fn digest_large_input_matches_reader_and_bytes_paths() {
        let bytes = vec![0xa5; 2 * 1024 * 1024 + 17];
        assert_eq!(
            sha256_reader(Cursor::new(&bytes)).expect("large reader must hash"),
            sha256_bytes(&bytes)
        );
    }

    #[test]
    fn digest_reader_propagates_io_error() {
        let error = sha256_reader(ErrorAfterPrefix {
            prefix: Cursor::new(b"partial bytes".to_vec()),
            failed: false,
        })
        .expect_err("reader failure must be returned");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }
}
