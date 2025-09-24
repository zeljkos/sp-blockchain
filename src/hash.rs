use blake2_rfc::blake2b::Blake2b;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Simplified Blake2b hash (32 bytes) extracted from Albatross
const BLAKE2B_LENGTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blake2bHash([u8; BLAKE2B_LENGTH]);

pub struct Blake2bHasher(Blake2b);

impl Serialize for Blake2bHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        hex::encode(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Blake2bHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex_str).map_err(serde::de::Error::custom)?;
        if bytes.len() != BLAKE2B_LENGTH {
            return Err(serde::de::Error::custom("Invalid hash length"));
        }
        let mut hash = [0u8; BLAKE2B_LENGTH];
        hash.copy_from_slice(&bytes);
        Ok(Blake2bHash(hash))
    }
}

impl Blake2bHash {
    pub fn new() -> Self {
        Self([0u8; BLAKE2B_LENGTH])
    }

    pub fn zero() -> Self {
        Self::new()
    }

    pub fn from_bytes(bytes: [u8; BLAKE2B_LENGTH]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; BLAKE2B_LENGTH] {
        &self.0
    }

    /// Hash data using Blake2b (simplified from Albatross)
    pub fn hash<T: AsRef<[u8]>>(data: T) -> Self {
        let mut hasher = Blake2bHasher::new();
        hasher.update(data.as_ref());
        hasher.finish()
    }

    pub fn len() -> usize {
        BLAKE2B_LENGTH
    }
}

impl Blake2bHasher {
    pub fn new() -> Self {
        Blake2bHasher(Blake2b::new(BLAKE2B_LENGTH))
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finish(self) -> Blake2bHash {
        let result = self.0.finalize();
        Blake2bHash::from(result.as_bytes())
    }
}

impl Default for Blake2bHash {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<[u8]> for Blake2bHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Blake2bHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl From<&[u8]> for Blake2bHash {
    fn from(bytes: &[u8]) -> Self {
        let mut hash = [0u8; BLAKE2B_LENGTH];
        let len = std::cmp::min(bytes.len(), BLAKE2B_LENGTH);
        hash[..len].copy_from_slice(&bytes[..len]);
        Self(hash)
    }
}