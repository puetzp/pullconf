use crate::configuration::Source;
use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
};
use strict_yaml_rust::StrictYaml;

#[derive(Clone, Debug)]
pub struct UnresolvedNode {
    pub source: Source,
    pub inner: StrictYaml,
}

impl UnresolvedNode {
    pub fn maybe_variable(
        &self,
        depth: usize,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Option<StrictYaml>, String> {
        match self
            .inner
            .as_str()
            .and_then(|s| s.strip_prefix("${pullconf::"))
        {
            Some(variable) => {
                let variable = match variable.strip_suffix('}') {
                    Some(v) => v.to_string(),
                    None => {
                        return Err(format!(
                            "{}: failed to find closing brace for variable",
                            self.source
                        ))
                    }
                };

                if depth > 10 {
                    return Err(format!(
                        "{}: recursion limit reached while resolving variable `{}`",
                        self.source, variable
                    ));
                }

                match variables.get(&variable) {
                    Some(node) => {
                        let nested = UnresolvedNode {
                            source: self.source.clone(),
                            inner: node.clone(),
                        };

                        Ok(nested
                            .maybe_variable(depth + 1, variables)?
                            .or(Some(node.clone())))
                    }
                    None => Err(format!(
                        "{}: reference to unknown variable `{}`",
                        self.source, variable
                    )),
                }
            }
            None => Ok(None),
        }
    }

    pub fn maybe_nested_variable(
        &self,
        depth: usize,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Option<StrictYaml>, String> {
        match self.inner.as_str() {
            Some(s) => {
                let mut _variables = vec![];

                let prefixes = s
                    .match_indices("${pullconf::")
                    .collect::<Vec<(usize, &str)>>();

                let mut suffixes = s.match_indices('}').collect::<VecDeque<(usize, &str)>>();

                for (prefix_idx, _) in prefixes {
                    let mut variable = None;

                    while let Some((suffix_idx, _)) = suffixes.pop_front() {
                        if suffix_idx > prefix_idx {
                            variable = Some(s[prefix_idx + 12..suffix_idx].to_string());
                            break;
                        }
                    }

                    if let Some(variable) = variable {
                        _variables.push(variable);
                    } else {
                        return Err(format!(
                            "{}: failed to find closing brace for variable",
                            self.source
                        ));
                    }
                }

                let mut substituted_string = s.to_string();

                for variable in _variables {
                    if depth > 10 {
                        return Err(format!(
                            "{}: recursion limit reached while resolving variable `{}`",
                            self.source, variable
                        ));
                    }

                    match variables.get(&variable) {
                        Some(node) => {
                            let nested = UnresolvedNode {
                                source: self.source.clone(),
                                inner: node.clone(),
                            };

                            let substring = nested
                                .maybe_nested_variable(depth + 1, variables)?
                                .unwrap_or(node.clone())
                                .as_str()
                                .ok_or(format!("{}: node must be a string", self.source))?
                                .to_string();

                            substituted_string = substituted_string
                                .replace(&format!("${{pullconf::{}}}", variable), &substring);
                        }
                        None => {
                            return Err(format!(
                                "{}: reference to unknown variable `{}`",
                                self.source, variable
                            ))
                        }
                    }
                }

                Ok(Some(StrictYaml::String(substituted_string)))
            }
            None => Ok(None),
        }
    }
}

pub trait Resolvable {
    fn resolve(
        node: UnresolvedNode,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Self, String>
    where
        Self: Sized;
}

impl<T: Resolvable> Resolvable for Vec<T> {
    fn resolve(
        node: UnresolvedNode,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Self, String> {
        match node.maybe_variable(0, variables)? {
            Some(value) => {
                let node = UnresolvedNode {
                    source: node.source,
                    inner: value,
                };

                Self::resolve(node, variables)
            }
            None => {
                let mut array = vec![];

                for (index, item) in node
                    .inner
                    .into_vec()
                    .ok_or_else(|| format!("{}: node must be an array", node.source))?
                    .into_iter()
                    .enumerate()
                {
                    let _node = UnresolvedNode {
                        source: node.source.clone() + index,
                        inner: item,
                    };

                    array.push(T::resolve(_node, variables)?);
                }

                Ok(array)
            }
        }
    }
}

impl Resolvable for common::resources::cron::job::Environment {
    fn resolve(
        node: UnresolvedNode,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Self, String> {
        match node.maybe_variable(0, variables)? {
            Some(value) => {
                let node = UnresolvedNode {
                    source: node.source,
                    inner: value,
                };

                Self::resolve(node, variables)
            }
            None => match node.inner.into_hash() {
                Some(mut hash) => {
                    let name = {
                        let key = "name";

                        hash.remove(&StrictYaml::String(key.into()))
                            .ok_or(format!(
                                "{}: failed to find required key `{}`",
                                node.source, key
                            ))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .and_then(|_node| String::resolve(_node, variables))?
                    };

                    let value = {
                        let key = "value";

                        hash.remove(&StrictYaml::String(key.into()))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .map(|_node| String::resolve(_node, variables))
                            .transpose()?
                    };

                    if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
                        return Err(format!(
                            "{}: encountered unexpected key `{}`",
                            node.source, key
                        ));
                    }

                    Ok(Self { name, value })
                }
                None => Err(format!("{}: node must be a hash", node.source)),
            },
        }
    }
}

impl Resolvable for common::resources::file::Content {
    fn resolve(
        node: UnresolvedNode,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Self, String> {
        match node.maybe_variable(0, variables)? {
            Some(value) => {
                let node = UnresolvedNode {
                    source: node.source,
                    inner: value,
                };

                Self::resolve(node, variables)
            }
            None => match node.inner.into_hash() {
                Some(mut hash) => {
                    let value = {
                        let key = "value";

                        hash.remove(&StrictYaml::String(key.into()))
                            .ok_or(format!(
                                "{}: failed to find required key `{}`",
                                node.source, key
                            ))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .and_then(|_node| String::resolve(_node, variables))?
                    };

                    let replace = {
                        let key = "replace";

                        hash.remove(&StrictYaml::String(key.into()))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .map(|_node| {
                                Vec::<common::resources::file::Replacement>::resolve(
                                    _node, variables,
                                )
                            })
                            .transpose()?
                            .unwrap_or_default()
                    };

                    if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
                        return Err(format!(
                            "{}: encountered unexpected key `{}`",
                            node.source, key
                        ));
                    }

                    Ok(Self { value, replace })
                }
                None => Err(format!("{}: node must be a hash", node.source)),
            },
        }
    }
}

impl Resolvable for common::resources::file::Replacement {
    fn resolve(
        node: UnresolvedNode,
        variables: &HashMap<String, StrictYaml>,
    ) -> Result<Self, String> {
        match node.maybe_variable(0, variables)? {
            Some(value) => {
                let node = UnresolvedNode {
                    source: node.source,
                    inner: value,
                };

                Self::resolve(node, variables)
            }
            None => match node.inner.into_hash() {
                Some(mut hash) => {
                    let variable = {
                        let key = "variable";

                        hash.remove(&StrictYaml::String(key.into()))
                            .ok_or(format!(
                                "{}: failed to find required key `{}`",
                                node.source, key
                            ))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .and_then(|_node| String::resolve(_node, variables))?
                    };

                    let command = {
                        let key = "command";

                        hash.remove(&StrictYaml::String(key.into()))
                            .ok_or(format!(
                                "{}: failed to find required key `{}`",
                                node.source, key
                            ))
                            .map(|_node| UnresolvedNode {
                                source: node.source.clone() + key,
                                inner: _node,
                            })
                            .and_then(|_node| Vec::<String>::resolve(_node, variables))?
                    };

                    if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
                        return Err(format!(
                            "{}: encountered unexpected key `{}`",
                            node.source, key
                        ));
                    }

                    Ok(Self { variable, command })
                }
                None => Err(format!("{}: node must be a hash", node.source)),
            },
        }
    }
}

macro_rules! impl_resolvable_via_string {
    ($( $type:ty ),*) => {
        $(
            impl Resolvable for $type {
                fn resolve(
                    node: UnresolvedNode,
                    variables: &HashMap<String, StrictYaml>,
                ) -> Result<Self, String> {
                    if let Some(value) = node.maybe_nested_variable(0, variables)? {
                        match value.as_str() {
                            Some(s) => {
                                Self::from_str(s)
                                    .map_err(|error| format!("{}: {}", node.source, error))
                            }
                            None => Err(format!("{}: node must be a string", node.source)),
                        }
                    } else {
                        let s = node
                            .inner
                            .as_str()
                            .ok_or_else(|| format!("{}: node must be a string", node.source))?;

                        Self::from_str(s).map_err(|error| format!("{}: {}", node.source, error))
                    }
                }
            }
        )*
    };
}

impl_resolvable_via_string!(
    bool,
    i16,
    std::net::IpAddr,
    std::path::PathBuf,
    String,
    u8,
    common::Ensure,
    common::Hostname,
    common::SafePathBuf,
    common::resources::apt::package::Ensure,
    common::resources::apt::package::Name,
    common::resources::apt::package::Version,
    common::resources::apt::preference::Name,
    common::resources::cron::job::Name,
    common::resources::file::Mode,
    common::resources::group::Name,
    common::resources::resolv_conf::ResolverOption,
    common::resources::resolv_conf::SortlistPair,
    common::resources::user::ExpiryDate,
    common::resources::user::Name,
    common::resources::user::Password
);
