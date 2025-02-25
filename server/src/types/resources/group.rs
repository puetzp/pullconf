use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::group::{Name, Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType, TriggerMetadata,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};

#[derive(Clone, Debug, Serialize)]
pub struct Group {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Group {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.name == other.parameters.name
    }
}

impl Eq for Group {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Group {
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

            Parameters {
                ensure,
                name,
                system,
            }
        };

        let kind = ResourceType::Group;

        let id = {
            let mut hasher = Sha256::new();
            hasher.update(kind.to_string());
            hasher.update(&*parameters.name);
            format!("{:x}", hasher.finalize())
        };

        Ok(Self {
            metadata: ResourceMetadata { kind, id },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Group {
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
    }

    pub fn display(&self) -> String {
        self.parameters.name.to_string()
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
            // Primary groups must be handled after users as user creation
            // usually involves creating the primary group as well.
            Resource::User(user) => user.parameters.group == self.parameters.name,
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Group(group) => group.parameters.name != self.parameters.name,
            Resource::User(user) => user.parameters.group != self.parameters.name,
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

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub name: UnresolvedNode,
    pub system: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::Group
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

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self {
            ensure,
            name,
            system,
        })
    }
}
