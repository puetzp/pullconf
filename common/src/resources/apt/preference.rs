use super::package::Name as PackageName;
use crate::{Ensure, ResourceMetadata};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, ops::Deref, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    pub name: Name,
    pub explanation: Option<String>,
    pub package: Package,
    pub pin: String,
    pub pin_priority: i16,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

/// The name of a preference doubles as the name for a file created
/// in `/etc/apt/preferences.d`. As such the restrictions on file names
/// described in `man apt_preferences` apply to `Name`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.'))
        {
            return Err(format!(
                "apt preference name `{}` contains invalid character `{}`",
                s, c
            ));
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Name> for Name {
    fn from(name: &Name) -> Self {
        name.clone()
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl Name {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Package {
    Wildcard,
    List(Vec<PackageName>),
}

impl FromStr for Package {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            Ok(Self::Wildcard)
        } else {
            let packages = s
                .split_ascii_whitespace()
                .map(|chunk| PackageName::from_str(chunk))
                .collect::<Result<Vec<PackageName>, String>>()?;

            Ok(Self::List(packages))
        }
    }
}

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Package {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wildcard => f.write_str("*"),
            Self::List(items) => {
                let _items = items
                    .iter()
                    .map(|item| item.as_str())
                    .collect::<Vec<&str>>();

                f.write_str(&_items.as_slice().join(" "))
            }
        }
    }
}
