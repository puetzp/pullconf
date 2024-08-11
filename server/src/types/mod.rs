pub mod client;
pub mod group;
pub mod resources;

pub use client::Client;
pub use group::Group;

use serde::{de::Error, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{ops::Deref, str::FromStr};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ApiKey(String);

impl FromStr for ApiKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 64 {
            return Err("API key hash has invalid length, must be a sha256 hash of exactly 64 hexadecimal characters".to_string());
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_hexdigit()))
        {
            return Err(format!(
                "API key hash contains an unexpected character <{}>, sha256 hash must be formatted in lowercased hexdigits",
                c
            ));
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for ApiKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl Deref for ApiKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl ApiKey {
    pub fn encrypt(s: &str) -> Self {
        Self(format!("{:x}", Sha256::digest(s.as_bytes())))
    }
}
