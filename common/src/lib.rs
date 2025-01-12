pub mod name;
pub mod path;
pub mod resources;

pub use name::Hostname;
pub use path::SafePathBuf;

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! impl_resource_types {
    ($( ($variant:ident, $display:literal) ),*) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub enum ResourceType {
            $(
                #[serde(rename = $display)]
                $variant,
            )*
        }

        impl FromStr for ResourceType {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $display => Ok(Self::$variant),
                    )*
                    _ => anyhow::bail!("invalid resource type: {}", s),
                }
            }
        }

        impl fmt::Display for ResourceType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(
                        Self::$variant => f.write_str($display),
                    )*
                }
            }
        }
    }
}

impl_resource_types!(
    (AptPackage, "apt::package"),
    (AptPreference, "apt::preference"),
    (CronJob, "cron::job"),
    (Directory, "directory"),
    (File, "file"),
    (Group, "group"),
    (Host, "host"),
    (ResolvConf, "resolv.conf"),
    (Symlink, "symlink"),
    (User, "user")
);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Links {
    #[serde(rename = "self")]
    pub this: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize)]
pub struct ResourceMetadata {
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub id: Uuid,
}

impl PartialOrd for ResourceMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourceMetadata {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for ResourceMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl ResourceMetadata {
    pub fn kind(&self) -> String {
        self.kind.to_string()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Ensure {
    #[default]
    #[serde(rename = "present")]
    Present,
    #[serde(rename = "absent")]
    Absent,
}

impl Ensure {
    pub fn is_present(&self) -> bool {
        *self == Self::Present
    }

    pub fn is_absent(&self) -> bool {
        *self == Self::Absent
    }
}

impl FromStr for Ensure {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "present" => Ok(Self::Present),
            "absent" => Ok(Self::Absent),
            _ => anyhow::bail!("invalid `ensure` value: {}", s),
        }
    }
}

macro_rules! impl_string_newtype {
    ($type:ty) => {
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let v = String::deserialize(deserializer)?;

                Self::from_str(&v).map_err(Error::custom)
            }
        }

        impl Deref for $type {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.0.as_str()
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&*self.0, f)
            }
        }

        impl $type {
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

pub(crate) use impl_string_newtype;
