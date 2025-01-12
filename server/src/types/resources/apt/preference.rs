use crate::{
    configuration::Source,
    types::resources::{Resolvable, Resource, UnresolvedNode},
};
use common::{
    resources::{
        apt::preference::{Name, Parameters, Relationships},
        directory::ChildNode,
    },
    Ensure, ResourceMetadata, ResourceType,
};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Preference {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Preference {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.name == other.parameters.name
    }
}

impl Eq for Preference {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Preference {
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

            let order = parameters
                .order
                .map(|parameter| u8::resolve(parameter, variables))
                .transpose()?;

            let explanation = parameters
                .explanation
                .map(|parameter| String::resolve(parameter, variables))
                .transpose()?;

            let package = String::resolve(parameters.package, variables)?;

            let pin = String::resolve(parameters.pin, variables)?;

            let pin_priority = i16::resolve(parameters.pin_priority, variables)?;

            let target = match order {
                Some(order) => PathBuf::from(format!("/etc/apt/preferences.d/{}-{}", order, name)),
                None => PathBuf::from(format!("/etc/apt/preferences.d/{}", name)),
            };

            Parameters {
                ensure,
                target,
                name,
                explanation,
                package,
                pin,
                pin_priority,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::AptPreference,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Preference {
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
            Resource::Directory(directory) => self
                .parameters
                .target
                .ancestors()
                .skip(1)
                .any(|ancestor| ancestor == *directory.parameters.path),
            Resource::Symlink(symlink) => self
                .parameters
                .target
                .ancestors()
                .skip(1)
                .any(|ancestor| ancestor == *symlink.parameters.path),
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::AptPreference(preference) => {
                preference.parameters.name != self.parameters.name
            }
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }
}

impl From<&Preference> for ChildNode {
    fn from(preference: &Preference) -> Self {
        Self::AptPreference {
            path: preference.parameters.target.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub name: UnresolvedNode,
    pub order: Option<UnresolvedNode>,
    pub explanation: Option<UnresolvedNode>,
    pub package: UnresolvedNode,
    pub pin: UnresolvedNode,
    pub pin_priority: UnresolvedNode,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::AptPreference
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

        let order = {
            let key = "order";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let explanation = {
            let key = "explanation";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let package = {
            let key = "package";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let pin = {
            let key = "pin";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let pin_priority = {
            let key = "pin_priority";

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
            name,
            order,
            explanation,
            package,
            pin,
            pin_priority,
        })
    }
}
