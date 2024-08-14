use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PackageName(String);

impl FromStr for PackageName {
    type Err = String;

    /// Package name syntax rules are taken from:
    /// https://www.debian.org/doc/debian-policy/ch-controlfields.html#s-f-source
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().count() < 2 {
            return Err("package name must be at least two characters long".to_string());
        }

        let first = s.chars().next().unwrap();

        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err("package name must start with an alphanumeric character".to_string());
        }

        if let Some(ref c) = s.chars().find(|c| {
            !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '+' || *c == '-' || *c == '.')
        }) {
            return Err(format!("package name contains invalid character `{}`", c));
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl From<&PackageName> for PackageName {
    fn from(name: &PackageName) -> Self {
        name.clone()
    }
}

impl Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PackageName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PackageVersion(String);

impl FromStr for PackageVersion {
    type Err = String;

    /// Package version syntax rules are taken from:
    /// https://www.debian.org/doc/debian-policy/ch-controlfields.html#version
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = match s.split_once(':') {
            None => s,
            Some((epoch, rest)) => match epoch.parse::<u8>() {
                Err(error) => {
                    return Err(format!(
                        "epoch component of package version string is invalid: {}",
                        error
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
                        "Debian revision component of package version string contains invalid character: `{}`",
                        c
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
                "upstream version component of package version string contains invalid character: `{}`",
                c
            ));
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for PackageVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Self::from_str(&v).map_err(Error::custom)
    }
}

impl From<&PackageVersion> for PackageVersion {
    fn from(name: &PackageVersion) -> Self {
        name.clone()
    }
}

impl Deref for PackageVersion {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PackageVersion {
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
        assert!(PackageName::from_str("a").is_err());
        // Starts with invalid character.
        assert!(PackageName::from_str(".a").is_err());
        // Contains invalid character.
        assert!(PackageName::from_str("asdasdad%a").is_err());
        // Valid.
        assert!(PackageName::from_str("nginx").is_ok());

        Ok(())
    }

    #[test]
    fn parse_package_versions() -> Result<(), String> {
        // Invalid epoch.
        assert!(PackageVersion::from_str("3242343:0.0.0-1").is_err());
        // Invalid debian revision.
        assert!(PackageVersion::from_str("1:0.0.0-1#").is_err());
        // Invalid upstream version.
        assert!(PackageVersion::from_str("1:0.0.*-1f").is_err());
        // Valid.
        assert!(PackageVersion::from_str("1:0.0.0-1").is_ok());

        Ok(())
    }
}
