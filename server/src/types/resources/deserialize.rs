use super::{apt, cron, directory, file, group, host, resolv_conf, symlink, user};
use common::{
    resources::{
        apt::{package::Name as PackageName, preference::Name as PreferenceName},
        group::Name as Groupname,
        user::Name as Username,
    },
    SafePathBuf,
};
use serde::{
    de::{DeserializeOwned, Error as SerdeError, Unexpected},
    Deserialize, Deserializer,
};
use std::{collections::HashMap, net::IpAddr};
use toml::Value;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, tag = "type")]
pub enum Resource {
    #[serde(rename = "apt::package")]
    AptPackage(apt::package::de::Parameters),
    #[serde(rename = "apt::preference")]
    AptPreference(apt::preference::de::Parameters),
    #[serde(rename = "cron::job")]
    CronJob(cron::job::de::Parameters),
    #[serde(rename = "directory")]
    Directory(directory::de::Parameters),
    #[serde(rename = "file")]
    File(file::de::Parameters),
    #[serde(rename = "group")]
    Group(group::de::Parameters),
    #[serde(rename = "host")]
    Host(host::de::Parameters),
    #[serde(rename = "resolv.conf")]
    ResolvConf(resolv_conf::de::Parameters),
    #[serde(rename = "symlink")]
    Symlink(symlink::de::Parameters),
    #[serde(rename = "user")]
    User(user::de::Parameters),
}

impl Resource {
    pub fn kind(&self) -> &str {
        match self {
            Self::AptPackage(parameters) => parameters.kind(),
            Self::AptPreference(parameters) => parameters.kind(),
            Self::CronJob(parameters) => parameters.kind(),
            Self::Directory(parameters) => parameters.kind(),
            Self::File(parameters) => parameters.kind(),
            Self::Group(parameters) => parameters.kind(),
            Self::Host(parameters) => parameters.kind(),
            Self::ResolvConf(parameters) => parameters.kind(),
            Self::Symlink(parameters) => parameters.kind(),
            Self::User(parameters) => parameters.kind(),
        }
    }

    pub fn requires(&self) -> &[Dependency] {
        match self {
            Self::AptPackage(parameters) => parameters.requires.as_slice(),
            Self::AptPreference(parameters) => parameters.requires.as_slice(),
            Self::CronJob(parameters) => parameters.requires.as_slice(),
            Self::Directory(parameters) => parameters.requires.as_slice(),
            Self::File(parameters) => parameters.requires.as_slice(),
            Self::Group(parameters) => parameters.requires.as_slice(),
            Self::Host(parameters) => parameters.requires.as_slice(),
            Self::ResolvConf(parameters) => parameters.requires.as_slice(),
            Self::Symlink(parameters) => parameters.requires.as_slice(),
            Self::User(parameters) => parameters.requires.as_slice(),
        }
    }

    pub fn as_apt_package(&self) -> Option<&apt::package::de::Parameters> {
        match self {
            Self::AptPackage(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_apt_preference(&self) -> Option<&apt::preference::de::Parameters> {
        match self {
            Self::AptPreference(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_cron_job(&self) -> Option<&cron::job::de::Parameters> {
        match self {
            Self::CronJob(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_directory(&self) -> Option<&directory::de::Parameters> {
        match self {
            Self::Directory(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_file(&self) -> Option<&file::de::Parameters> {
        match self {
            Self::File(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_group(&self) -> Option<&group::de::Parameters> {
        match self {
            Self::Group(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_host(&self) -> Option<&host::de::Parameters> {
        match self {
            Self::Host(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_resolv_conf(&self) -> Option<&resolv_conf::de::Parameters> {
        match self {
            Self::ResolvConf(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_symlink(&self) -> Option<&symlink::de::Parameters> {
        match self {
            Self::Symlink(parameters) => Some(parameters),
            _ => None,
        }
    }

    pub fn as_user(&self) -> Option<&user::de::Parameters> {
        match self {
            Self::User(parameters) => Some(parameters),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum VariableOrValue {
    #[serde(deserialize_with = "deserialize_variable")]
    Variable(String),
    Value(Value),
}

impl VariableOrValue {
    pub fn resolve<T: DeserializeOwned>(
        &self,
        label: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<T, String> {
        let value = match self {
            VariableOrValue::Variable(variable) => variables.get(variable).ok_or_else(|| {
                format!(
                    "parameter `{}` refers to unknown variable `{}`",
                    label, variable
                )
            })?,
            VariableOrValue::Value(value) => value,
        };

        value.clone().try_into().map_err(|error| {
            format!(
                "parameter `{}` contains an invalid value: {}",
                label,
                error.to_string().trim_end()
            )
        })
    }
}

fn deserialize_variable<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_variable = String::deserialize(deserializer)?;

    match maybe_variable.strip_prefix("$pullconf::") {
        None => Err(SerdeError::invalid_value(
            Unexpected::Str(&maybe_variable),
            &"a string starting with prefix `$pullconf::`",
        )),
        Some(variable) => Ok(variable.to_string()),
    }
}

#[cfg(test)]
impl VariableOrValue {
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Self::Value(v) => Some(&v),
            _ => None,
        }
    }

    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Self::Variable(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(_))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, tag = "type")]
pub enum Dependency {
    #[serde(rename = "apt::package")]
    AptPackage { name: PackageName },
    #[serde(rename = "apt::preference")]
    AptPreference { name: PreferenceName },
    #[serde(rename = "directory")]
    Directory { path: SafePathBuf },
    #[serde(rename = "file")]
    File { path: SafePathBuf },
    #[serde(rename = "group")]
    Group { name: Groupname },
    #[serde(rename = "host")]
    Host {
        #[serde(rename = "ip-address")]
        ip_address: IpAddr,
    },
    #[serde(rename = "resolv.conf")]
    ResolvConf,
    #[serde(rename = "symlink")]
    Symlink { path: SafePathBuf },
    #[serde(rename = "user")]
    User { name: Username },
}

impl Dependency {
    pub fn repr(&self) -> String {
        match self {
            Self::AptPackage { name } => format!("apt::package `{}`", name),
            Self::AptPreference { name } => format!("apt::preference `{}`", name),
            Self::Directory { path } => format!("directory `{}`", path.display()),
            Self::File { path } => format!("file `{}`", path.display()),
            Self::Group { name } => format!("group `{}`", name),
            Self::Host { ip_address } => format!("host `{}`", ip_address),
            Self::ResolvConf => "resolv.conf `/etc/resolv.conf`".to_string(),
            Self::Symlink { path } => format!("symlink `{}`", path.display()),
            Self::User { name } => format!("user `{}`", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn deserialize_variable() -> Result<(), anyhow::Error> {
        #[derive(Deserialize)]
        struct TestStruct {
            _variable: VariableOrValue,
            _bool: VariableOrValue,
            _str: VariableOrValue,
        }

        let object: TestStruct = toml::from_str(
            r#"
_variable = "$pullconf::foo"
_bool = true
_str = "bar"
"#,
        )?;

        assert!(object._variable.as_variable().is_some_and(|v| v == "foo"));
        assert!(object._bool.as_value().is_some_and(|v| v.is_bool()));
        assert!(object
            ._str
            .as_value()
            .is_some_and(|v| v.as_str().is_some_and(|s| s == "bar")));

        Ok(())
    }

    #[test]
    fn deserialize_dependency() -> Result<(), anyhow::Error> {
        #[derive(Debug, Deserialize)]
        struct TestStruct {
            requires: Vec<Dependency>,
        }

        let object: TestStruct = toml::from_str(
            r#"
requires = [
    { type = "directory", path = "/foo/bar" },
    { type = "resolv.conf" },
    { type = "user", name = "foobar" },
    { type = "host", ip-address = "127.0.0.1" }
]
"#,
        )?;

        let expected = vec![
            Dependency::Directory {
                path: SafePathBuf::from_str("/foo/bar").unwrap(),
            },
            Dependency::ResolvConf,
            Dependency::User {
                name: Username::from_str("foobar").unwrap(),
            },
            Dependency::Host {
                ip_address: IpAddr::from_str("127.0.0.1").unwrap(),
            },
        ];

        assert_eq!(object.requires, expected);

        Ok(())
    }
}
