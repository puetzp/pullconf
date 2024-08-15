use super::{apt, directory, file, group, host, resolv_conf, symlink, user};
use common::{Groupname, PackageName, SafePathBuf, Username};
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

        value
            .clone()
            .try_into()
            .map_err(|error| format!("parameter `{}` contains an invalid value: {}", label, error))
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
