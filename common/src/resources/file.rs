use crate::{Ensure, Groupname, ResourceMetadata, SafePathBuf, Username};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{ops::Deref, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub mode: Mode,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub content: Option<String>,
    pub source: Option<SafePathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Mode(String);

impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s).map_err(Error::custom)
    }
}

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(3..=4).contains(&s.len()) || s.chars().any(|c| !('0'..='7').contains(&c)) {
            anyhow::bail!("value is not a valid file mode");
        }

        Ok(Self(s.to_string()))
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self("644".to_string())
    }
}

impl Deref for Mode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}
