use crate::types::{client, ApiKey, Client, Group};
use common::Hostname;
use log::{debug, warn};
use std::{
    collections::HashMap,
    fmt, fs,
    io::ErrorKind,
    ops::{Add, AddAssign},
    path::PathBuf,
    time::Instant,
};
use strict_yaml_rust::{StrictYaml, StrictYamlLoader};

#[derive(Clone, Debug)]
pub struct Source {
    pub file: PathBuf,
    pub path: String,
}

impl Source {
    pub fn new(file: PathBuf, path: &str) -> Self {
        Self {
            file,
            path: path.to_string(),
        }
    }
}

impl Add<usize> for Source {
    type Output = Self;

    fn add(self, other: usize) -> Self {
        Self {
            file: self.file,
            path: format!("{}[{}]", self.path, other),
        }
    }
}

impl Add<&str> for Source {
    type Output = Self;

    fn add(self, other: &str) -> Self {
        Self {
            file: self.file,
            path: format!("{}.{}", self.path, other),
        }
    }
}

impl AddAssign<usize> for Source {
    fn add_assign(&mut self, other: usize) {
        self.path = format!("{}[{}]", self.path, other);
    }
}

impl AddAssign<&str> for Source {
    fn add_assign(&mut self, other: &str) {
        self.path = format!("{}.{}", self.path, other);
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`>`{}`", self.file.display(), self.path)
    }
}

#[derive(Default)]
pub struct Configuration {
    pub clients: HashMap<Hostname, Client>,
    pub api_keys: HashMap<ApiKey, Hostname>,
}

impl TryFrom<&PathBuf> for Configuration {
    type Error = String;

    fn try_from(resource_path: &PathBuf) -> Result<Self, Self::Error> {
        debug!("`{}`: parsing configuration", resource_path.display());

        let start = Instant::now();

        let mut groups = HashMap::new();
        let mut unresolved_clients = HashMap::new();

        parse(0, resource_path, &mut groups, &mut unresolved_clients)?;

        let mut clients: HashMap<Hostname, Client> = HashMap::new();
        let mut api_keys: HashMap<ApiKey, Hostname> = HashMap::new();

        for unresolved_client in unresolved_clients.into_values() {
            let client = Client::try_from((unresolved_client, &mut groups))?;

            if let Some(other) = api_keys.insert(client.api_key.clone(), client.name.clone()) {
                return Err(format!(
                    "`{}`: API key from client `{}` matches that from client `{}`, but API keys must be unique",
                    resource_path.display(),
                    client.name(),
                    other
                ));
            } else {
                clients.insert(client.name().clone(), client);
            }
        }

        for (group, count) in groups.values() {
            if *count == 0 {
                warn!(
                    "`{}`: group `{}` is never referenced by any client",
                    resource_path.display(),
                    group.name
                );
            }
        }

        debug!(
            "`{}`: took {} ms to parse configuration",
            resource_path.display(),
            start.elapsed().as_millis()
        );

        Ok(Self { clients, api_keys })
    }
}

fn parse(
    recursion: usize,
    path: &PathBuf,
    groups: &mut HashMap<Hostname, (Group, usize)>,
    unresolved_clients: &mut HashMap<Hostname, client::unresolved::Client>,
) -> Result<(), String> {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
    {
        warn!(
            "`{}`: ignoring file or directory as it starts with a dot",
            path.display()
        );

        return Ok(());
    }

    match fs::read_dir(path) {
        Ok(entries) => {
            if recursion > 10 {
                warn!(
                    "`{}`: reached recursion limit, ignoring directory",
                    path.display()
                );
            } else {
                for entry in entries {
                    let entry =
                        entry.map_err(|error| format!("`{}`: {}", path.display(), error))?;
                    parse(recursion + 1, &entry.path(), groups, unresolved_clients)?
                }
            }
        }
        Err(error) if error.kind() == ErrorKind::NotADirectory => {
            if path
                .extension()
                .is_some_and(|extension| extension == "yaml" || extension == "yml")
            {
                let contents = fs::read_to_string(&path)
                    .map_err(|error| format!("`{}`: {}", path.display(), error))?;

                let yaml = StrictYamlLoader::load_from_str(&contents)
                    .map_err(|error| format!("`{}`: {}", path.display(), error))?;

                for (index, document) in yaml.into_iter().enumerate() {
                    let source = Source::new(path.clone(), "documents") + index;

                    let mut hash = document
                        .into_hash()
                        .ok_or(format!("{}: node must be a hash", source))?;

                    let key = "type";

                    let kind = hash
                        .remove(&StrictYaml::String(key.into()))
                        .ok_or(format!(
                            "{}: failed to find required key `{}`",
                            source.clone(),
                            key
                        ))?
                        .into_string()
                        .ok_or(format!("{}: node must be a string", source.clone() + key))?;

                    match kind.as_str() {
                        "client" => {
                            let client =
                                client::unresolved::Client::try_from((source.clone(), hash))?;

                            if unresolved_clients.contains_key(&client.name) {
                                return Err(format!(
                                    "{}: encountered duplicate client `{}`",
                                    source, client.name
                                ));
                            } else {
                                unresolved_clients.insert(client.name.clone(), client);
                            }
                        }
                        "group" => {
                            let group = Group::try_from((source.clone(), hash))?;

                            if groups.contains_key(&group.name) {
                                return Err(format!(
                                    "{}: encountered duplicate group `{}`",
                                    source, group.name
                                ));
                            } else {
                                groups.insert(group.name.clone(), (group, 0));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "{}: encountered invalid value `{}`",
                                source + key,
                                kind
                            ))
                        }
                    }
                }
            } else {
                warn!(
                    "`{}`: ignoring file as it does not end with a `yaml` or `yml` extension",
                    path.display()
                );
            }
        }
        Err(error) => return Err(format!("`{}`: {}", path.display(), error)),
    }

    Ok(())
}
