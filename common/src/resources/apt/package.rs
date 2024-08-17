use crate::ResourceMetadata;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Name,
    pub version: Option<Version>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Ensure {
    #[default]
    #[serde(rename = "present")]
    Present,
    #[serde(rename = "absent")]
    Absent,
    #[serde(rename = "purged")]
    Purged,
}

impl Ensure {
    pub fn is_present(&self) -> bool {
        *self == Self::Present
    }

    pub fn is_absent(&self) -> bool {
        *self == Self::Absent
    }

    pub fn is_purged(&self) -> bool {
        *self == Self::Purged
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = String;

    /// Package name syntax rules are taken from:
    /// https://www.debian.org/doc/debian-policy/ch-controlfields.html#s-f-source
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().count() < 2 {
            return Err(format!(
                "package name `{}` must be at least two characters long",
                s
            ));
        }

        let first = s.chars().next().unwrap();

        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(format!(
                "package name `{}` must start with an alphanumeric character",
                s
            ));
        }

        if let Some(ref c) = s.chars().find(|c| {
            !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '+' || *c == '-' || *c == '.')
        }) {
            return Err(format!(
                "package name `{}` contains invalid character `{}`",
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Version(String);

impl FromStr for Version {
    type Err = String;

    /// Package version syntax rules are taken from:
    /// https://www.debian.org/doc/debian-policy/ch-controlfields.html#version
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let copy = s.to_string();

        let s = match s.split_once(':') {
            None => s,
            Some((epoch, rest)) => match epoch.parse::<u8>() {
                Err(error) => {
                    return Err(format!(
                        "epoch component of package version `{}` is invalid: {}",
                        copy, error
                    ))
                }
                Ok(_) => rest,
            },
        };

        let s = match s.rsplit_once('-') {
            None => s,
            Some((rest, debian_revision)) => {
                if let Some(c) = debian_revision
                    .chars()
                    .find(|c| !(c.is_ascii_alphanumeric() || *c == '+' || *c == '~' || *c == '.'))
                {
                    return Err(format!(
                        "Debian revision component of package version `{}` contains invalid character: `{}`",
                        copy, c
                    ));
                } else {
                    rest
                }
            }
        };

        if let Some(c) = s.chars().find(|c| {
            !(c.is_ascii_alphanumeric() || *c == '+' || *c == '-' || *c == '~' || *c == '.')
        }) {
            return Err(format!(
                "upstream version component of package version `{}` contains invalid character: `{}`",
                copy, c
            ));
        }

        Ok(Self(copy.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Version> for Version {
    fn from(name: &Version) -> Self {
        name.clone()
    }
}

impl Deref for Version {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl Version {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_package_names() -> Result<(), String> {
        // Too short.
        assert!(Name::from_str("a").is_err());
        // Starts with invalid character.
        assert!(Name::from_str(".a").is_err());
        // Contains invalid character.
        assert!(Name::from_str("asdasdad%a").is_err());
        // Valid.
        assert!(Name::from_str("nginx").is_ok());

        Ok(())
    }

    #[test]
    fn parse_package_versions() -> Result<(), String> {
        // Invalid epoch.
        assert!(Version::from_str("3242343:0.0.0-1").is_err());
        // Invalid debian revision.
        assert!(Version::from_str("1:0.0.0-1#").is_err());
        // Invalid upstream version.
        assert!(Version::from_str("1:0.0.*-1f").is_err());
        // Valid.
        assert!(Version::from_str("1:0.0.0-1").is_ok());

        Ok(())
    }
}
