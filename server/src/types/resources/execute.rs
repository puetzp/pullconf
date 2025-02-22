use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::execute::{Environment, Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType, TriggerMetadata,
};
use serde::Serialize;
use std::collections::HashMap;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Execute {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Execute {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.name == other.parameters.name
    }
}

impl Eq for Execute {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Execute {
    type Error = String;

    fn try_from(
        (parameters, variables): (UnresolvedParameters, &HashMap<String, StrictYaml>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match parameters.ensure {
                Some(parameter) => Ensure::resolve(parameter, variables)?,
                None => Ensure::default(),
            };

            let name = String::resolve(parameters.name, variables)?;

            let command = Vec::<String>::resolve(parameters.command, variables)?;

            if command.is_empty() {
                return Err(format!(
                    "command array in execute resource `{}` cannot be empty",
                    name,
                ));
            }

            let unless = match parameters.unless {
                Some(parameter) => Vec::<String>::resolve(parameter, variables)?,
                None => vec![],
            };

            let environment = match parameters.environment {
                Some(parameter) => Vec::<Environment>::resolve(parameter, variables)?,
                None => vec![],
            };

            let passive = match parameters.passive {
                Some(parameter) => bool::resolve(parameter, variables)?,
                None => false,
            };

            Parameters {
                ensure,
                name,
                command,
                unless,
                environment,
                passive,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Execute,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Execute {
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

    pub fn must_depend_on(&self, _resource: &Resource) -> bool {
        false
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Execute(execute) => execute.parameters.name != self.parameters.name,
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
    pub command: UnresolvedNode,
    pub unless: Option<UnresolvedNode>,
    pub environment: Option<UnresolvedNode>,
    pub passive: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::Execute
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

        let command = {
            let key = "command";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let unless = {
            let key = "unless";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let environment = {
            let key = "environment";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let passive = {
            let key = "passive";

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
            command,
            unless,
            environment,
            passive,
        })
    }
}
