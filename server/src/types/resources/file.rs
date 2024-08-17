use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::{
        directory::ChildNode,
        file::{Mode, Parameters, Relationships},
        user::Name as Username,
    },
    Ensure, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct File {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for File {
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

            let mode = match &parameters.mode {
                Some(parameter) => parameter.resolve("mode", variables)?,
                None => Mode::default(),
            };

            let owner = match &parameters.owner {
                Some(parameter) => parameter.resolve("owner", variables)?,
                None => Username::root(),
            };

            let group = match &parameters.group {
                Some(parameter) => parameter.resolve("group", variables)?,
                None => None,
            };

            let content = match &parameters.content {
                Some(parameter) => parameter.resolve("content", variables)?,
                None => None,
            };

            let source = match &parameters.source {
                Some(parameter) => parameter.resolve("source", variables)?,
                None => None,
            };

            // The contents of a file can either be set via the `content` or `source`
            // parameters, but not both. If neither parameter is set, the file contents
            // are not managed at all.
            if source.is_some() && content.is_some() {
                return Err(
                    "parameters `content` and `source` are mutually exclusive and cannot be defined both at the same time".to_string()
                );
            }

            Parameters {
                ensure,
                path,
                mode,
                owner,
                group,
                content,
                source,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::File,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl File {
    pub fn kind(&self) -> &str {
        "file"
    }

    pub fn display(&self) -> String {
        self.parameters.path.display().to_string()
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
            Resource::File(file) => file.parameters.path != self.parameters.path,
            Resource::Host(host) => host.parameters.target != *self.parameters.path,
            Resource::ResolvConf(resolv_conf) => {
                resolv_conf.parameters.target != *self.parameters.path
            }
            Resource::Symlink(symlink) => symlink.parameters.target != self.parameters.path,
            _ => true,
        }
    }
}

impl From<&File> for ChildNode {
    fn from(file: &File) -> Self {
        Self::File {
            path: file.parameters.path.clone(),
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
        pub mode: Option<VariableOrValue>,
        #[serde(default)]
        pub owner: Option<VariableOrValue>,
        #[serde(default)]
        pub group: Option<VariableOrValue>,
        #[serde(default)]
        pub content: Option<VariableOrValue>,
        #[serde(default)]
        pub source: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> &str {
            "file"
        }
    }
}
