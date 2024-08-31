use crate::types::resources::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::{
        cron::job::{Parameters, Relationships},
        user::Name as Username,
    },
    Ensure, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use toml::Value;
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

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Job {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match &parameters.ensure {
                Some(parameter) => parameter.resolve("ensure", variables)?,
                None => Ensure::default(),
            };

            let name = parameters.name.resolve("name", variables)?;

            let schedule = parameters.schedule.resolve("schedule", variables)?;

            let user = match &parameters.user {
                Some(parameter) => parameter.resolve("user", variables)?,
                None => Username::root(),
            };

            let command = parameters.command.resolve("command", variables)?;

            let target = PathBuf::from(format!("/etc/cron.d/{}", name));

            Parameters {
                ensure,
                target,
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
        format!("{} `{}`", self.kind(), self.display())
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

pub mod de {
    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Parameters {
        #[serde(default)]
        pub ensure: Option<VariableOrValue>,
        pub name: VariableOrValue,
        pub schedule: VariableOrValue,
        #[serde(default)]
        pub user: Option<VariableOrValue>,
        pub command: VariableOrValue,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> ResourceType {
            ResourceType::CronJob
        }
    }
}
