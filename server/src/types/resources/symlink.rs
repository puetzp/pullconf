use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::directory::ChildNode,
    resources::symlink::{Parameters, Relationships},
    Ensure, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

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

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Symlink {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match &parameters.ensure {
                Some(parameter) => parameter.resolve("ensure", variables)?,
                None => Ensure::default(),
            };

            let path = parameters.path.resolve("path", variables)?;

            let target = parameters.target.resolve("target", variables)?;

            Parameters {
                ensure,
                path,
                target,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Symlink,
                id: Uuid::new_v4(),
            },
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
            Resource::AptPreference(preference) => {
                preference.parameters.target != *self.parameters.path
            }
            Resource::Directory(directory) => directory.parameters.path != self.parameters.target,
            Resource::File(file) => file.parameters.path != self.parameters.target,
            Resource::Host(host) => host.parameters.target != *self.parameters.path,
            Resource::ResolvConf(resolv_conf) => {
                resolv_conf.parameters.target != *self.parameters.path
            }
            Resource::Symlink(symlink) => symlink.parameters.path != self.parameters.path,
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }
}

impl From<&Symlink> for ChildNode {
    fn from(symlink: &Symlink) -> Self {
        Self::Symlink {
            path: symlink.parameters.path.clone(),
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
        pub path: VariableOrValue,
        pub target: VariableOrValue,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> ResourceType {
            ResourceType::Symlink
        }
    }
}
