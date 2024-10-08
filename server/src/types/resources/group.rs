use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::group::{Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

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

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Group {
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

            let system = match &parameters.system {
                Some(parameter) => parameter.resolve("system", variables)?,
                None => false,
            };

            Parameters {
                ensure,
                name,
                system,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Group,
                id: Uuid::new_v4(),
            },
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

    pub fn id(&self) -> Uuid {
        self.metadata.id
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{} `{}`", self.kind(), self.display())
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
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> ResourceType {
            ResourceType::Group
        }
    }
}
