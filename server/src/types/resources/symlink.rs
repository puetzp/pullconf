use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::directory::ChildNode,
    resources::symlink::{Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType, SafePathBuf, TriggerMetadata,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};

#[derive(Clone, Debug, Serialize)]
pub struct Symlink {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Symlink {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.path == other.parameters.path
    }
}

impl Eq for Symlink {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Symlink {
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

            let target = SafePathBuf::resolve(parameters.target, variables)?;

            Parameters {
                ensure,
                path,
                target,
            }
        };

        let kind = ResourceType::Symlink;

        let id = {
            let mut hasher = Sha256::new();
            hasher.update(kind.to_string());
            hasher.update(parameters.path.to_str().unwrap());
            format!("{:x}", hasher.finalize())
        };

        Ok(Self {
            metadata: ResourceMetadata { kind, id },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Symlink {
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
    }

    pub fn display(&self) -> String {
        self.parameters.path.display().to_string()
    }

    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{}[{}]", self.kind(), self.display())
    }

    pub fn must_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Directory(directory) => {
                self.parameters
                    .path
                    .ancestors()
                    .any(|ancestor| ancestor == *directory.parameters.path)
                    || directory.parameters.path == self.parameters.target
            }
            Resource::File(file) => file.parameters.path == self.parameters.target,
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
            Resource::Directory(directory) => directory.parameters.path != self.parameters.target,
            Resource::File(file) => file.parameters.path != self.parameters.target,
            Resource::Host(host) => host.parameters.target != *self.parameters.path,
            Resource::Symlink(symlink) => symlink.parameters.path != self.parameters.path,
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }

    pub fn push_predecessor(&mut self, metadata: ResourceMetadata) {
        self.relationships.after.push(metadata)
    }

    pub fn push_trigger(&mut self, metadata: TriggerMetadata) {
        self.relationships.triggers.push(metadata)
    }
}

impl From<&Symlink> for ChildNode {
    fn from(symlink: &Symlink) -> Self {
        Self::Symlink {
            path: symlink.parameters.path.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub path: UnresolvedNode,
    pub target: UnresolvedNode,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::Symlink
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

        let target = {
            let key = "target";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self {
            ensure,
            path,
            target,
        })
    }
}
