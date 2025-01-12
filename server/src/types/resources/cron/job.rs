use crate::{
    configuration::Source,
    types::resources::{Resolvable, Resource, UnresolvedNode},
};
use common::{
    resources::{
        cron::job::{Environment, Name, Parameters, Relationships},
        user::Name as Username,
    },
    Ensure, ResourceMetadata, ResourceType,
};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Job {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.name == other.parameters.name
    }
}

impl Eq for Job {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Job {
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

            let mut environment = match parameters.environment {
                Some(parameter) => Vec::<Environment>::resolve(parameter, variables)?,
                None => vec![],
            };

            if environment.iter().any(|variable| variable.name.is_empty()) {
                return Err(
                    "environment variable names in `environment` cannot be empty".to_string(),
                );
            }

            environment.sort_by(|a, b| a.name.cmp(&b.name));

            let environment_count = environment.len();

            environment.dedup_by(|a, b| a.name == b.name);

            if environment_count != environment.len() {
                return Err(
                    "environment variable names in `environment` must be unique".to_string()
                );
            }

            let schedule = String::resolve(parameters.schedule, variables)?;

            let user = match parameters.user {
                Some(parameter) => Username::resolve(parameter, variables)?,
                None => Username::root(),
            };

            let command = String::resolve(parameters.command, variables)?;

            let target = PathBuf::from(format!("/etc/cron.d/{}", name));

            Parameters {
                ensure,
                target,
                environment,
                name,
                schedule,
                user,
                command,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::CronJob,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Job {
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
            Resource::CronJob(item) => item.parameters.name != self.parameters.name,
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub name: UnresolvedNode,
    pub environment: Option<UnresolvedNode>,
    pub schedule: UnresolvedNode,
    pub user: Option<UnresolvedNode>,
    pub command: UnresolvedNode,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::CronJob
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

        let environment = {
            let key = "environment";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let schedule = {
            let key = "schedule";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let user = {
            let key = "user";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
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

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self {
            ensure,
            name,
            environment,
            schedule,
            user,
            command,
        })
    }
}
