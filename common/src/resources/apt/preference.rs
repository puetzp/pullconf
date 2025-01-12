use crate::{Ensure, ResourceMetadata};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    pub name: Name,
    pub explanation: Option<String>,
    pub package: String,
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

crate::impl_string_newtype!(Name);
