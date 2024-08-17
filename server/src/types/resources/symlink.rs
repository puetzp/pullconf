use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{DirectoryChildNode, Ensure, ResourceMetadata, ResourceType, SafePathBuf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub target: SafePathBuf,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Symlink {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

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
    pub fn kind(&self) -> &str {
        "symlink"
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

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
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
}

impl From<&Symlink> for DirectoryChildNode {
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
        pub fn kind(&self) -> &str {
            "symlink"
        }
    }
}
