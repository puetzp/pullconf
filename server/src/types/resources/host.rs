use super::{Resolvable, Resource, UnresolvedNode};
use crate::configuration::Source;
use common::{
    resources::host::{Parameters, Relationships},
    Ensure, Hostname, ResourceMetadata, ResourceType, TriggerMetadata,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, net::IpAddr, path::Path};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};

#[derive(Clone, Debug, Serialize)]
pub struct Host {
    #[serde(flatten)]
    pub metadata: ResourceMetadata,
    pub parameters: Parameters,
    pub relationships: Relationships,
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.parameters.ip_address == other.parameters.ip_address
    }
}

impl Eq for Host {}

impl TryFrom<(UnresolvedParameters, &HashMap<String, StrictYaml>)> for Host {
    type Error = String;

    fn try_from(
        (parameters, variables): (UnresolvedParameters, &HashMap<String, StrictYaml>),
    ) -> Result<Self, Self::Error> {
        let parameters = {
            let ensure = match parameters.ensure {
                Some(parameter) => Ensure::resolve(parameter, variables)?,
                None => Ensure::default(),
            };

            let ip_address = IpAddr::resolve(parameters.ip_address, variables)?;

            let hostname = Hostname::resolve(parameters.hostname, variables)?;

            let aliases = match parameters.aliases {
                Some(parameter) => Vec::<Hostname>::resolve(parameter, variables)?,
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

        let kind = ResourceType::Host;

        let id = {
            let mut hasher = Sha256::new();
            hasher.update(kind.to_string());
            hasher.update(parameters.ip_address.to_string());
            format!("{:x}", hasher.finalize())
        };

        Ok(Self {
            metadata: ResourceMetadata { kind, id },
            parameters,
            relationships: Relationships::default(),
        })
    }
}

impl Host {
    pub fn kind(&self) -> ResourceType {
        self.metadata.kind
    }

    pub fn display(&self) -> String {
        self.parameters.ip_address.to_string()
    }

    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    pub fn repr(&self) -> String {
        format!("{}[{}]", self.kind(), self.display())
    }

    pub fn must_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::File(file) => *file.parameters.path == self.parameters.target,
            Resource::Symlink(symlink) => *symlink.parameters.path == self.parameters.target,
            _ => false,
        }
    }

    pub fn may_depend_on(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Host(host) => host.parameters.ip_address != self.parameters.ip_address,
            _ => true,
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        self.relationships.requires.push(metadata)
    }

    pub fn push_predecessor(&mut self, metadata: ResourceMetadata) {
        self.relationships.after.push(metadata)
    }

    pub fn push_trigger(&mut self, metadata: TriggerMetadata) {
        self.relationships.triggers.push(metadata)
    }
}

#[derive(Clone, Debug)]
pub struct UnresolvedParameters {
    pub ensure: Option<UnresolvedNode>,
    pub ip_address: UnresolvedNode,
    pub hostname: UnresolvedNode,
    pub aliases: Option<UnresolvedNode>,
}

impl UnresolvedParameters {
    pub fn kind(&self) -> ResourceType {
        ResourceType::Host
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

        let ip_address = {
            let key = "ip_address";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let hostname = {
            let key = "hostname";

            hash.remove(&StrictYaml::String(key.to_string()))
                .map(|node| UnresolvedNode {
                    source: source.clone() + key,
                    inner: node,
                })
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
        };

        let aliases = {
            let key = "aliases";

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
            ip_address,
            hostname,
            aliases,
        })
    }
}
