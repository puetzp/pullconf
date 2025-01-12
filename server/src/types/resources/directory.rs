use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::{
        directory::{ChildNode, Parameters, Relationships},
        group::Name as Groupname,
        user::Name as Username,
    },
    Ensure, ResourceMetadata, ResourceType, SafePathBuf,
};
use serde::Serialize;
use std::collections::HashMap;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Directory {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.path == other.parameters.path
    }
}

impl Eq for Directory {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Directory {
    type Error = String;

    fn try_from(
        (parameters, variables): (UnresolvedParameters, &HashMap<String, StrictYaml>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match parameters.ensure {
                Some(parameter) => Ensure::resolve(parameter, variables)?,
                None => Ensure::default(),
            };

            let path = SafePathBuf::resolve(parameters.path, variables)?;

            let owner = match parameters.owner {
                Some(parameter) => Username::resolve(parameter, variables)?,
                None => Username::root(),
            };

            let group = parameters
                .group
                .map(|parameter| Groupname::resolve(parameter, variables))
                .transpose()?;

            let purge = match parameters.purge {
                Some(parameter) => bool::resolve(parameter, variables)?,
                None => false,
            };

            Parameters {
                ensure,
                path,
                owner,
                group,
                purge,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Directory,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Directory {
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
    }

    pub fn display(&self) -> String {
        self.parameters.path.display().to_string()
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
            Resource::Directory(directory) => self
                .parameters
                .path
                .ancestors()
                .skip(1)
                .any(|ancestor| ancestor == *directory.parameters.path),
            Resource::Symlink(symlink) => self
                .parameters
                .path
                .ancestors()
                .skip(1)
                .any(|ancestor| ancestor == *symlink.parameters.path),
            Resource::User(user) => user.parameters.home == self.parameters.path,
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::AptPreference(preference) => !preference
                .parameters
                .target
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::Directory(directory) => {
                directory.parameters.path != self.parameters.path
                    && !directory
                        .parameters
                        .path
                        .ancestors()
                        .any(|a| a == *self.parameters.path)
            }
            Resource::File(file) => !file
                .parameters
                .path
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::Host(host) => !host
                .parameters
                .target
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::ResolvConf(resolv_conf) => !resolv_conf
                .parameters
                .target
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::Symlink(symlink) => {
                !symlink
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *self.parameters.path)
                    && !symlink
                        .parameters
                        .target
                        .ancestors()
                        .any(|a| a == *self.parameters.path)
            }
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }
}

impl From<&Directory> for ChildNode {
    fn from(directory: &Directory) -> Self {
        Self::Directory {
            path: directory.parameters.path.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub path: UnresolvedNode,
    pub ensure: Option<UnresolvedNode>,
    pub owner: Option<UnresolvedNode>,
    pub group: Option<UnresolvedNode>,
    pub purge: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::Directory
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

        let path = {
            let key = "path";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let owner = {
            let key = "owner";

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

        let purge = {
            let key = "purge";

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
            path,
            owner,
            group,
            purge,
        })
    }
}
