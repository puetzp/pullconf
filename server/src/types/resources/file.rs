use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::{
        directory::ChildNode,
        file::{Content, Mode, Parameters, Relationships},
        group::Name as Groupname,
        user::Name as Username,
    },
    Ensure, ResourceMetadata, ResourceType, SafePathBuf, TriggerMetadata,
};
use serde::Serialize;
use std::collections::HashMap;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct File {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.path == other.parameters.path
    }
}

impl Eq for File {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for File {
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

            let mode = match parameters.mode {
                Some(parameter) => Mode::resolve(parameter, variables)?,
                None => Mode::default(),
            };

            let owner = match parameters.owner {
                Some(parameter) => Username::resolve(parameter, variables)?,
                None => Username::root(),
            };

            let group = parameters
                .group
                .map(|parameter| Groupname::resolve(parameter, variables))
                .transpose()?;

            let content = parameters
                .content
                .map(|parameter| Content::resolve(parameter, variables))
                .transpose()?;

            let source = parameters
                .source
                .map(|parameter| SafePathBuf::resolve(parameter, variables))
                .transpose()?;

            // The contents of a file can either be set via the `content` or `source`
            // parameters, but not both. If neither parameter is set, the file contents
            // are not managed at all.
            if source.is_some() && content.is_some() {
                return Err(
                    "parameters `content` and `source` are mutually exclusive and cannot be defined both at the same time".to_string()
                );
            }

            if let Some(content) = &content {
                for item in &content.replace {
                    if item.command.is_empty() {
                        return Err(
                            "command args in `content` must at least contain the name of a program to execute".to_string()
                        );
                    }
                }
            }

            Parameters {
                ensure,
                path,
                mode,
                owner,
                group,
                content,
                source,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::File,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl File {
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
                .any(|ancestor| ancestor == *directory.parameters.path),
            Resource::Symlink(symlink) => self
                .parameters
                .path
                .ancestors()
                .any(|ancestor| ancestor == *symlink.parameters.path),
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::File(file) => file.parameters.path != self.parameters.path,
            Resource::Host(host) => host.parameters.target != *self.parameters.path,
            Resource::Symlink(symlink) => symlink.parameters.target != self.parameters.path,
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

impl From<&File> for ChildNode {
    fn from(file: &File) -> Self {
        Self::File {
            path: file.parameters.path.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub path: UnresolvedNode,
    pub ensure: Option<UnresolvedNode>,
    pub mode: Option<UnresolvedNode>,
    pub owner: Option<UnresolvedNode>,
    pub group: Option<UnresolvedNode>,
    pub content: Option<UnresolvedNode>,
    pub source: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::File
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

        let mode = {
            let key = "mode";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
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

        let content = {
            let key = "content";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let _source = {
            let key = "source";

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
            mode,
            owner,
            group,
            content,
            source: _source,
        })
    }
}
