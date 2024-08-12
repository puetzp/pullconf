use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::user::{serialize_expiry_date, Password},
    Ensure, Groupname, Hostname, ResourceMetadata, ResourceType, SafePathBuf, Username,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use time::Date;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Username,
    pub system: bool,
    pub comment: Option<String>,
    pub shell: Option<SafePathBuf>,
    pub home: SafePathBuf,
    pub password: Password,
    #[serde(serialize_with = "serialize_expiry_date")]
    pub expiry_date: Option<Date>,
    // Primary group name.
    pub group: Groupname,
    // Names of supplementary groups.
    pub groups: Vec<Groupname>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Relationships {
    #[serde(skip_serializing)]
    pub _requires: Vec<Dependency>,
    pub requires: Vec<ResourceMetadata>,
}

impl From<Vec<Dependency>> for Relationships {
    fn from(_requires: Vec<Dependency>) -> Self {
        Self {
            _requires,
            requires: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct User {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
    pub from_group: Option<Hostname>,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for User {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let requires = parameters.requires.clone();

        let parameters = {
            let ensure = match &parameters.ensure {
                Some(parameter) => parameter.resolve("ensure", variables)?,
                None => Ensure::default(),
            };

            let name = parameters.name.resolve("name", variables)?;

            let system = match &parameters.system {
                Some(parameter) => parameter.resolve("system", variables)?,
                None => false,
            };

            let comment = match &parameters.comment {
                Some(parameter) => parameter.resolve("comment", variables)?,
                None => None,
            };

            let shell = match &parameters.shell {
                Some(parameter) => parameter.resolve("shell", variables)?,
                None => None,
            };

            let home = match &parameters.home {
                Some(parameter) => parameter.resolve("home", variables)?,
                None => SafePathBuf::from_str(&format!("/home/{}", name)).unwrap(),
            };

            let password = match &parameters.password {
                Some(parameter) => parameter.resolve("password", variables)?,
                None => Password::Locked,
            };

            let expiry_date = match &parameters.expiry_date {
                Some(parameter) => parameter.resolve("expiry-date", variables)?,
                None => None,
            };

            let group = match &parameters.group {
                Some(parameter) => parameter.resolve("group", variables)?,
                None => Groupname::from(&name),
            };

            let mut groups = match &parameters.groups {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("groups", variables)?
                    .into_iter()
                    .map(|item| item.resolve("groups", variables))
                    .collect::<Result<Vec<Groupname>, String>>()?,
                None => vec![],
            };

            groups.sort();

            // Ensure that the primary group name does not also appear
            // in the list of supplementary group names.
            if groups.contains(&group) {
                return Err(format!("primary group `{}` of user `{}` cannot appear in the list of supplementary groups", group, name));
            }

            Parameters {
                ensure,
                name,
                system,
                comment,
                shell,
                home,
                password,
                expiry_date,
                group,
                groups,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::User,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::from(requires),
            from_group: None,
        })
    }
}

impl User {
    pub fn kind(&self) -> &str {
        "user"
    }

    pub fn display(&self) -> String {
        self.parameters.name.to_string()
    }

    pub fn id(&self) -> Uuid {
        self.metadata.id
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{} `{}`", self.kind(), self.display())
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Directory(directory) => self.parameters.home != directory.parameters.path,
            Resource::Group(group) => !self.parameters.groups.contains(&group.parameters.name),
            Resource::User(user) => user.parameters.name != self.parameters.name,
            _ => true,
        }
    }
}

pub mod de {
    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Parameters {
        #[serde(default)]
        pub ensure: Option<VariableOrValue>,
        pub name: VariableOrValue,
        #[serde(default)]
        pub system: Option<VariableOrValue>,
        #[serde(default)]
        pub comment: Option<VariableOrValue>,
        #[serde(default)]
        pub shell: Option<VariableOrValue>,
        #[serde(default)]
        pub home: Option<VariableOrValue>,
        #[serde(default)]
        pub password: Option<VariableOrValue>,
        #[serde(default, rename(deserialize = "expiry-date"))]
        pub expiry_date: Option<VariableOrValue>,
        #[serde(default)]
        pub group: Option<VariableOrValue>,
        #[serde(default)]
        pub groups: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }
}
