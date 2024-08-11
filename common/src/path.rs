use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::ops::Deref;
use std::path::{Component, PathBuf};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SafePathBuf(PathBuf);

impl TryFrom<PathBuf> for SafePathBuf {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_absolute() {
            anyhow::bail!("path must be absolute, got `{}`", path.display())
        } else if path
            .components()
            .any(|c| c != Component::RootDir && !matches!(c, Component::Normal(_)))
        {
            anyhow::bail!(
                "path components must not contain relative references such as '.' or '..'",
            )
        } else {
            Ok(Self(path))
        }
    }
}

impl FromStr for SafePathBuf {
    type Err = anyhow::Error;

    fn from_str(p: &str) -> Result<Self, Self::Err> {
        Self::try_from(PathBuf::from_str(p)?)
    }
}

impl<'de> Deserialize<'de> for SafePathBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let p = PathBuf::deserialize(deserializer)?;

        Self::try_from(p).map_err(Error::custom)
    }
}

impl Deref for SafePathBuf {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
