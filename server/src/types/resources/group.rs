use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{Ensure, Groupname, Hostname, ResourceMetadata, ResourceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Groupname,
    pub system: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Relationships {
    #[serde(skip_serializing)]
    pub _requires: Vec<Dependency>,
    pub requires: Vec<ResourceMetadata>,
}

impl From<Vec<Dependency>> for Relationships {
    fn from(_requires: Vec<Dependency>) -> Self {
        Self {
            _requires,
            requires: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Group {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
    pub from_group: Option<Hostname>,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Group {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let requires = parameters.requires.clone();

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
            relationships: Relationships::from(requires),
            from_group: None,
        })
    }
}

impl Group {
    pub fn kind(&self) -> &str {
        "group"
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
            Resource::Group(group) => group.parameters.name != self.parameters.name,
            Resource::User(user) => user.parameters.group != self.parameters.name,
            _ => true,
        }
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
        pub fn kind(&self) -> &str {
            "group"
        }
    }
}
