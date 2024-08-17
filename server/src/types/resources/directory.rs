use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    DirectoryChildNode, Ensure, Groupname, ResourceMetadata, ResourceType, SafePathBuf, Username,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub purge: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Relationships {
    #[serde(skip_serializing)]
    pub _requires: Vec<Dependency>,
    pub requires: Vec<ResourceMetadata>,
    pub children: Vec<DirectoryChildNode>,
}

impl From<Vec<Dependency>> for Relationships {
    fn from(_requires: Vec<Dependency>) -> Self {
        Self {
            _requires,
            requires: vec![],
            children: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Directory {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Directory {
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

            let path = parameters.path.resolve("path", variables)?;

            let owner = match &parameters.owner {
                Some(parameter) => parameter.resolve("owner", variables)?,
                None => Username::root(),
            };

            let group = match &parameters.group {
                Some(parameter) => parameter.resolve("group", variables)?,
                None => None,
            };

            let purge = match &parameters.purge {
                Some(parameter) => parameter.resolve("purge", variables)?,
                None => false,
            };

            Parameters {
                ensure,
                path,
                owner,
                group,
                purge,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Directory,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::from(requires),
        })
    }
}

impl Directory {
    pub fn kind(&self) -> &str {
        "directory"
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
            Resource::Directory(directory) => {
                directory.parameters.path != self.parameters.path
                    && !directory
                        .parameters
                        .path
                        .ancestors()
                        .any(|a| a == *self.parameters.path)
            }
            Resource::File(file) => !file
                .parameters
                .path
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::Host(host) => !host
                .parameters
                .target
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::ResolvConf(resolv_conf) => !resolv_conf
                .parameters
                .target
                .ancestors()
                .any(|a| a == *self.parameters.path),
            Resource::Symlink(symlink) => {
                !symlink
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *self.parameters.path)
                    && !symlink
                        .parameters
                        .target
                        .ancestors()
                        .any(|a| a == *self.parameters.path)
            }
            _ => true,
        }
    }
}

impl From<&Directory> for DirectoryChildNode {
    fn from(directory: &Directory) -> Self {
        Self::Directory {
            path: directory.parameters.path.clone(),
        }
    }
}

pub mod de {
    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Parameters {
        pub path: VariableOrValue,
        #[serde(default)]
        pub ensure: Option<VariableOrValue>,
        #[serde(default)]
        pub owner: Option<VariableOrValue>,
        #[serde(default)]
        pub group: Option<VariableOrValue>,
        #[serde(default)]
        pub purge: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> &str {
            "directory"
        }
    }
}
