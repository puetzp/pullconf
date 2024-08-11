use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{Ensure, Hostname, ResourceMetadata, ResourceType};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::IpAddr,
    path::{Path, PathBuf},
};
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    #[serde(rename(deserialize = "ip-address"))]
    pub ip_address: IpAddr,
    pub hostname: Hostname,
    #[serde(default)]
    pub aliases: Vec<Hostname>,
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
pub struct Host {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
    pub from_group: Option<Hostname>,
}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for Host {
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

            let ip_address = parameters.ip_address.resolve("ip-address", variables)?;

            let hostname = parameters.hostname.resolve("hostname", variables)?;

            let aliases = match &parameters.aliases {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("aliases", variables)?
                    .into_iter()
                    .map(|item| item.resolve("aliases", variables))
                    .collect::<Result<Vec<Hostname>, String>>()?,
                None => vec![],
            };

            let alias_count = aliases.len();

            if alias_count > 4 {
                return Err(format!(
                    "host `{}` has {} `aliases`, cannot be more than four",
                    ip_address, alias_count
                ));
            }

            Parameters {
                ensure,
                target: Path::new("/etc/hosts").to_owned(),
                ip_address,
                hostname,
                aliases,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::Host,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::from(requires),
            from_group: None,
        })
    }
}

impl Host {
    pub fn kind(&self) -> &str {
        "host"
    }

    pub fn display(&self) -> String {
        self.parameters.ip_address.to_string()
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
            Resource::Host(host) => host.parameters.ip_address != self.parameters.ip_address,
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
        #[serde(rename(deserialize = "ip-address"))]
        pub ip_address: VariableOrValue,
        pub hostname: VariableOrValue,
        #[serde(default)]
        pub aliases: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }
}
