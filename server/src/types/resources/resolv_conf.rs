use super::{
    deserialize::{Dependency, VariableOrValue},
    Resource,
};
use common::{
    resources::resolv_conf::{Parameters, Relationships, ResolverOption, SortlistPair},
    Ensure, Hostname, ResourceMetadata, ResourceType,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::IpAddr, path::Path};
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct ResolvConf {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for ResolvConf {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ResolvConf {}

impl TryFrom<(&de::Parameters, &HashMap<String, Value>)> for ResolvConf {
    type Error = String;

    fn try_from(
        (parameters, variables): (&de::Parameters, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match &parameters.ensure {
                Some(parameter) => parameter.resolve("ensure", variables)?,
                None => Ensure::default(),
            };

            let nameservers = match &parameters.nameservers {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("nameservers", variables)?
                    .into_iter()
                    .map(|item| item.resolve("nameservers", variables))
                    .collect::<Result<Vec<IpAddr>, String>>()?,
                None => vec![],
            };

            let search = match &parameters.search {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("search", variables)?
                    .into_iter()
                    .map(|item| item.resolve("search", variables))
                    .collect::<Result<Vec<Hostname>, String>>()?,
                None => vec![],
            };

            let sortlist = match &parameters.sortlist {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("sortlist", variables)?
                    .into_iter()
                    .map(|item| item.resolve("sortlist", variables))
                    .collect::<Result<Vec<SortlistPair>, String>>()?,
                None => vec![],
            };

            let options = match &parameters.options {
                Some(parameter) => parameter
                    .resolve::<Vec<VariableOrValue>>("options", variables)?
                    .into_iter()
                    .map(|item| item.resolve("options", variables))
                    .collect::<Result<Vec<ResolverOption>, String>>()?,
                None => vec![],
            };

            Parameters {
                ensure,
                target: Path::new("/etc/resolv.conf").to_owned(),
                nameservers,
                search,
                sortlist,
                options,
            }
        };

        Ok(Self {
            metadata: ResourceMetadata {
                kind: ResourceType::ResolvConf,
                id: Uuid::new_v4(),
            },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl ResolvConf {
    pub fn kind(&self) -> &str {
        "resolv.conf"
    }

    pub fn id(&self) -> Uuid {
        self.metadata.id()
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{} `{}`", self.kind(), self.parameters.target.display())
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        !matches!(resource, Resource::ResolvConf(_))
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
        #[serde(default)]
        pub nameservers: Option<VariableOrValue>,
        #[serde(default)]
        pub search: Option<VariableOrValue>,
        #[serde(default)]
        pub sortlist: Option<VariableOrValue>,
        #[serde(default)]
        pub options: Option<VariableOrValue>,
        #[serde(default)]
        pub requires: Vec<Dependency>,
    }

    impl Parameters {
        pub fn kind(&self) -> &str {
            "resolv.conf"
        }
    }
}
