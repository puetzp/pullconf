use crate::types::resources::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::apt::preference::{Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Preference {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Preference {
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

            let package = parameters.package.resolve("package", variables)?;

            let pin = parameters.pin.resolve("pin", variables)?;

            let pin_priority = parameters.pin_priority.resolve("pin-priority", variables)?;

            let target = PathBuf::from(format!("/etc/apt/preferences.d/{}", name));

            Parameters {
                ensure,
                target,
                name,
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
    pub fn kind(&self) -> &str {
        "apt::preference"
    }

    pub fn display(&self) -> String {
        self.parameters.name.to_string()
    }

    pub fn id(&self) -> Uuid {
        self.metadata.id()
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{} `{}`", self.kind(), self.display())
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::AptPreference(preference) => {
                preference.parameters.name != self.parameters.name
            }
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
        pub package: VariableOrValue,
        pub pin: VariableOrValue,
        #[serde(rename = "pin-priority")]
        pub pin_priority: VariableOrValue,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> &str {
            "apt::preference"
        }
    }
}
