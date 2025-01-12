use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::resolv_conf::{Parameters, Relationships, ResolverOption, SortlistPair},
    Ensure, Hostname, ResourceMetadata, ResourceType,
};
use serde::Serialize;
use std::{collections::HashMap, net::IpAddr, path::Path};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
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

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for ResolvConf {
    type Error = String;

    fn try_from(
        (parameters, variables): (UnresolvedParameters, &HashMap<String, StrictYaml>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match parameters.ensure {
                Some(parameter) => Ensure::resolve(parameter, variables)?,
                None => Ensure::default(),
            };

            let nameservers = parameters
                .nameservers
                .map(|parameter| Vec::<IpAddr>::resolve(parameter, variables))
                .transpose()?
                .unwrap_or_default();

            let search = parameters
                .search
                .map(|parameter| Vec::<Hostname>::resolve(parameter, variables))
                .transpose()?
                .unwrap_or_default();

            let sortlist = parameters
                .sortlist
                .map(|parameter| Vec::<SortlistPair>::resolve(parameter, variables))
                .transpose()?
                .unwrap_or_default();

            let options = parameters
                .options
                .map(|parameter| Vec::<ResolverOption>::resolve(parameter, variables))
                .transpose()?
                .unwrap_or_default();

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
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
    }

    pub fn id(&self) -> Uuid {
        self.metadata.id
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{}[{}]", self.kind(), self.parameters.target.display())
    }

    pub fn must_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::File(file) => *file.parameters.path == self.parameters.target,
            Resource::Symlink(symlink) => *symlink.parameters.path == self.parameters.target,
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        !matches!(resource, Resource::ResolvConf(_))
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub nameservers: Option<UnresolvedNode>,
    pub search: Option<UnresolvedNode>,
    pub sortlist: Option<UnresolvedNode>,
    pub options: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::ResolvConf
    }
}

impl TryFrom<(Source, Hash)> for UnresolvedParameters {
    type Error = String;

    fn try_from((source, mut hash): (Source, Hash)) -> Result<Self, Self::Error> {
        let ensure = {
            let key = "ensure";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let nameservers = {
            let key = "nameservers";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let search = {
            let key = "search";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let sortlist = {
            let key = "sortlist";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        let options = {
            let key = "options";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
        };

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self {
            ensure,
            nameservers,
            search,
            sortlist,
            options,
        })
    }
}
