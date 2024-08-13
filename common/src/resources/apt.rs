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
