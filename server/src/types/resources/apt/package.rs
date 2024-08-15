use crate::types::resources::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    Hostname, PackageEnsure, PackageName, PackageVersion, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub ensure: PackageEnsure,
    pub name: PackageName,
    pub version: Option<PackageVersion>,
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
pub struct Package {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
    pub from_group: Option<Hostname>,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Package {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let requires = parameters.requires.clone();

        let parameters = {
            let ensure = match &parameters.ensure {
                Some(parameter) => parameter.resolve("ensure", variables)?,
                None => PackageEnsure::default(),
            };

            let name = parameters.name.resolve("name", variables)?;

            let version = match &parameters.version {
                Some(parameter) => parameter.resolve("version", variables)?,
                None => None,
            };

            Parameters {
                ensure,
                name,
                version,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::AptPackage,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::from(requires),
            from_group: None,
        })
    }
}

impl Package {
    pub fn kind(&self) -> &str {
        "apt::package"
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

    /// This resource may depend on any other resource.
    pub fn may_depend_on(&self, _resource: &Resource) -> bool {
        true
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
        pub version: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> &str {
            "apt::package"
        }
    }
}
