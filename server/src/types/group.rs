use crate::{configuration::Source, types::resources::UnresolvedResource};
use common::Hostname;
use std::str::FromStr;
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};

#[derive(Clone, Debug)]
pub struct Group {
    pub name: Hostname,
    pub resources: Vec<UnresolvedResource>,
}

impl TryFrom<(Source, Hash)> for Group {
    type Error = String;

    fn try_from((source, mut hash): (Source, Hash)) -> Result<Self, Self::Error> {
        let mut resources = vec![];

        let name = {
            let key = "name";

            let s = hash
                .remove(&StrictYaml::String(key.into()))
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                .into_string()
                .ok_or(format!("{}: node must be a string", source.clone() + key))?;

            Hostname::from_str(&s)
                .map_err(|error| format!("{}: {}", source.clone() + key, error))?
        };

        {
            let key = "resources";
            let source = source.clone() + key;

            if let Some(node) = hash.remove(&StrictYaml::String(key.into())) {
                for (index, item) in node
                    .into_vec()
                    .ok_or(format!("{}: node must be an array", source))?
                    .into_iter()
                    .enumerate()
                {
                    let source = source.clone() + index;

                    let hash = item
                        .into_hash()
                        .ok_or(format!("{}: node must be a hash", source))?;

                    let resource = UnresolvedResource::try_from((source, hash))?;

                    resources.push(resource);
                }
            }
        }

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(Self { name, resources })
    }
}
