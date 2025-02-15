use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::{
        group::Name as Groupname,
        user::{ExpiryDate, Name, Parameters, Password, Relationships},
    },
    Ensure, ResourceMetadata, ResourceType, SafePathBuf, TriggerMetadata,
};
use serde::Serialize;
use std::{collections::HashMap, str::FromStr};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct User {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.name == other.parameters.name
    }
}

impl Eq for User {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for User {
    type Error = String;

    fn try_from(
        (parameters, variables): (UnresolvedParameters, &HashMap<String, StrictYaml>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match parameters.ensure {
                Some(parameter) => Ensure::resolve(parameter, variables)?,
                None => Ensure::default(),
            };

            let name = Name::resolve(parameters.name, variables)?;

            let system = match parameters.system {
                Some(parameter) => bool::resolve(parameter, variables)?,
                None => false,
            };

            let comment = parameters
                .comment
                .map(|parameter| String::resolve(parameter, variables))
                .transpose()?;

            let shell = parameters
                .shell
                .map(|parameter| SafePathBuf::resolve(parameter, variables))
                .transpose()?;

            let home = match parameters.home {
                Some(parameter) => SafePathBuf::resolve(parameter, variables)?,
                None => SafePathBuf::from_str(&format!("/home/{}", name)).unwrap(),
            };

            let password = match parameters.password {
                Some(parameter) => Password::resolve(parameter, variables)?,
                None => Password::Locked,
            };

            let expiry_date = parameters
                .expiry_date
                .map(|parameter| ExpiryDate::resolve(parameter, variables))
                .transpose()?;

            let group = match parameters.group {
                Some(parameter) => Groupname::resolve(parameter, variables)?,
                None => Groupname::from(&name),
            };

            let mut groups = match parameters.groups {
                Some(parameter) => Vec::<Groupname>::resolve(parameter, variables)?,
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
            relationships: Relationships::default(),
        })
    }
}

impl User {
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
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
        format!("{}[{}]", self.kind(), self.display())
    }

    pub fn must_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            // Add group resources as dependencies if their name appears
            // in the list of user group names.
            // Supplementary groups must be processed before users.
            Resource::Group(group) => self.parameters.groups.contains(&group.parameters.name),
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Directory(directory) => self.parameters.home != directory.parameters.path,
            Resource::Group(group) => !self.parameters.groups.contains(&group.parameters.name),
            Resource::User(user) => user.parameters.name != self.parameters.name,
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }

    pub fn push_trigger(&mut self, metadata: TriggerMetadata) {
        self.relationships.triggers.push(metadata)
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub name: UnresolvedNode,
    pub system: Option<UnresolvedNode>,
    pub comment: Option<UnresolvedNode>,
    pub shell: Option<UnresolvedNode>,
    pub home: Option<UnresolvedNode>,
    pub password: Option<UnresolvedNode>,
    pub expiry_date: Option<UnresolvedNode>,
    pub group: Option<UnresolvedNode>,
    pub groups: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::User
    }
}

impl TryFrom<(Source, Hash)> for UnresolvedParameters {
    type Error = String;

    fn try_from((source, mut hash): (Source, Hash)) -> Result<Self, Self::Error> {
        let ensure = {
            let key = "ensure";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let name = {
            let key = "name";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let system = {
            let key = "system";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let comment = {
            let key = "comment";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let shell = {
            let key = "shell";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let home = {
            let key = "home";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let password = {
            let key = "password";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let expiry_date = {
            let key = "expiry_date";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let group = {
            let key = "group";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let groups = {
            let key = "groups";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self {
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
        })
    }
}
