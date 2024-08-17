use super::group::Name as Groupname;
use crate::{Ensure, ResourceMetadata, SafePathBuf};
use serde::{
    de::{Error as SerdeError, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, ops::Deref, str::FromStr};
use time::{format_description::FormatItem, macros::format_description, Date};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Name,
    pub system: bool,
    pub comment: Option<String>,
    pub shell: Option<SafePathBuf>,
    pub home: SafePathBuf,
    pub password: Password,
    #[serde(
        deserialize_with = "deserialize_expiry_date",
        serialize_with = "serialize_expiry_date"
    )]
    pub expiry_date: Option<Date>,
    // Primary group name.
    pub group: Groupname,
    // Names of supplementary groups.
    pub groups: Vec<Groupname>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

pub const EXPIRY_DATE_FORMAT: &[FormatItem] = format_description!("[year]-[month]-[day]");

pub fn deserialize_expiry_date<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(v) => match Date::parse(&v, EXPIRY_DATE_FORMAT) {
            Ok(date) => Ok(Some(date)),
            Err(_) => Err(SerdeError::invalid_value(
                Unexpected::Str(&v),
                &"a date in the format <YYYY-MM-DD>",
            )),
        },
        None => Ok(None),
    }
}

pub fn serialize_expiry_date<S>(date: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(date) => serializer.serialize_some(&date.format(&EXPIRY_DATE_FORMAT).unwrap()),
        None => serializer.serialize_none(),
    }
}

#[derive(Clone, Debug, Eq, Default, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Password {
    #[default]
    #[serde(rename(serialize = "!"))]
    Locked,
    Unlocked(String),
}

impl FromStr for Password {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let valid_prefixes: [&str; 6] = ["$5$", "$6$", "$7$", "$2b$", "$gy$", "$y$"];

        if s.starts_with('!') || s == "*" {
            Ok(Password::Locked)
        } else if valid_prefixes.iter().any(|prefix| s.starts_with(prefix)) {
            Ok(Password::Unlocked(s.to_string()))
        } else {
            anyhow::bail!("password string is not a valid hash")
        }
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Password::from_str(&s).map_err(SerdeError::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("username cannot be an empty string")
        }

        if s.chars().count() > 32 {
            anyhow::bail!("username cannot exceed 32 characters")
        }

        if s.chars()
            .next()
            .is_some_and(|c| !(c.is_ascii_alphabetic() || c == '_'))
        {
            anyhow::bail!("username must start with an alphabetic character or an underscore")
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            anyhow::bail!("username contains invalid character `{}`", c)
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

        Name::from_str(&v).map_err(SerdeError::custom)
    }
}

impl From<&Name> for Name {
    fn from(name: &Name) -> Self {
        name.clone()
    }
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PartialEq<Groupname> for Name {
    fn eq(&self, other: &Groupname) -> bool {
        &self.0 == other.deref()
    }
}

impl Name {
    pub fn root() -> Self {
        Self(String::from("root"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
