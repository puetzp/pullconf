pub mod error;
pub mod name;
pub mod path;
pub mod resources;

pub use name::Hostname;
pub use path::SafePathBuf;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ResourceType {
    #[serde(rename = "apt::package")]
    AptPackage,
    #[serde(rename = "apt::preference")]
    AptPreference,
    #[serde(rename = "cron::job")]
    CronJob,
    #[serde(rename = "directory")]
    Directory,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "resolv.conf")]
    ResolvConf,
    #[serde(rename = "symlink")]
    Symlink,
    #[serde(rename = "user")]
    User,
}

impl FromStr for ResourceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "apt::package" => Ok(Self::AptPackage),
            "apt::preference" => Ok(Self::AptPreference),
            "cron::job" => Ok(Self::CronJob),
            "directory" => Ok(Self::Directory),
            "file" => Ok(Self::File),
            "group" => Ok(Self::Group),
            "host" => Ok(Self::Host),
            "resolv.conf" => Ok(Self::ResolvConf),
            "symlink" => Ok(Self::Symlink),
            "user" => Ok(Self::User),
            _ => anyhow::bail!("invalid resource type: {}", s),
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AptPackage => f.write_str("apt::package"),
            Self::AptPreference => f.write_str("apt::preference"),
            Self::CronJob => f.write_str("cron::job"),
            Self::Directory => f.write_str("directory"),
            Self::File => f.write_str("file"),
            Self::Group => f.write_str("group"),
            Self::Host => f.write_str("host"),
            Self::ResolvConf => f.write_str("resolv.conf"),
            Self::Symlink => f.write_str("symlink"),
            Self::User => f.write_str("user"),
        }
    }
}

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
