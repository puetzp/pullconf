use crate::types::{client, ApiKey, Client, Group};
use common::{error::Terminate, Hostname};
use log::{debug, error, warn};
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr, time::Instant};

#[derive(Default)]
pub struct Configuration {
    pub clients: HashMap<Hostname, Client>,
    pub api_keys: HashMap<ApiKey, Hostname>,
}

impl TryFrom<&PathBuf> for Configuration {
    type Error = Terminate;

    fn try_from(resources: &PathBuf) -> Result<Self, Self::Error> {
        let scope = "validation";

        debug!(scope, source:% = resources.display(); "parsing configuration");

        let start = Instant::now();

        let client_directory = {
            let mut path = resources.to_owned();
            path.push("clients");

            if !path.is_dir() {
                error!(
                    scope,
                    source:% = path.display();
                    "directory containing client configuration files does not exist"
                );

                return Err(Terminate);
            }

            path
        };

        let group_directory = {
            let mut path = resources.to_owned();
            path.push("groups");

            if !path.is_dir() {
                error!(
                    scope,
                    source:% = path.display();
                    "directory containing group configuration files does not exist"
                );

                return Err(Terminate);
            }

            path
        };

        let mut groups = HashMap::new();

        let entries = match fs::read_dir(&group_directory) {
            Ok(e) => e,
            Err(error) => {
                error!(
                    scope,
                    source:% = group_directory.display();
                    "{}",
                    error
                );

                return Err(Terminate);
            }
        };

        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    error!(
                        scope,
                        source:% = group_directory.display();
                        "{}",
                        error
                    );

                    return Err(Terminate);
                }
            };

            if path.is_file() {
                if path
                    .extension()
                    .is_some_and(|extension| extension == "toml")
                {
                    let (name, group) = parse_file::<Group>(&path)?;

                    if groups.insert(name.clone(), (group, 0)).is_some() {
                        error!(
                            scope,
                            source:% = path.display();
                            "group {} appears multiple times, but group names must be unique",
                            name
                        );

                        return Err(Terminate);
                    }
                } else {
                    warn!(
                        scope,
                        source:% = path.display();
                        "ignoring file as it does not end with a .toml extension",
                    );
                }
            } else {
                warn!(
                    scope,
                    source:% = path.display();
                    "ignoring nested directory"
                );
            }
        }

        let entries = match fs::read_dir(&client_directory) {
            Ok(e) => e,
            Err(error) => {
                error!(
                    scope,
                    source:% = client_directory.display();
                    "{}",
                    error
                );

                return Err(Terminate);
            }
        };

        let mut clients: HashMap<Hostname, Client> = HashMap::new();
        let mut api_keys: HashMap<ApiKey, Hostname> = HashMap::new();

        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(error) => {
                    error!(
                        scope,
                        source:% = client_directory.display();
                        "{}",
                        error
                    );

                    return Err(Terminate);
                }
            };

            if path.is_file() {
                if path
                    .extension()
                    .is_some_and(|extension| extension == "toml")
                {
                    let (name, intermediate) = parse_file::<client::deserialize::Client>(&path)?;

                    if clients.contains_key(&name) {
                        error!(
                            scope,
                            source:% = path.display();
                            "client `{}` appears multiple times, but client names must be unique",
                            name
                        );

                        return Err(Terminate);
                    }

                    let client = Client::try_from((name, intermediate, &mut groups))?;

                    if let Some(other) =
                        api_keys.insert(client.api_key.clone(), client.name.clone())
                    {
                        error!(
                            scope,
                            source:% = path.display();
                            "API key hash from client `{}` matches that from client `{}`, but API keys must be unique",
                            client.name(),
                            other
                        );

                        return Err(Terminate);
                    } else {
                        clients.insert(client.name().clone(), client);
                    }
                } else {
                    warn!(
                        scope,
                        source:% = path.display();
                        "ignoring file as it does not end with a .toml extension",
                    );
                }
            } else {
                warn!(
                    scope,
                    source:% = path.display();
                    "ignoring nested directory"
                );
            }
        }

        for (name, (_, count)) in &groups {
            if *count == 0 {
                warn!("group `{}` is never referenced by any client", name);
            }
        }

        debug!(
            scope;
            "took {} ms to parse configuration from `{}`",
            start.elapsed().as_millis(),
            resources.display()
        );

        Ok(Self { clients, api_keys })
    }
}

fn parse_file<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<(Hostname, T), Terminate> {
    let scope = "validation";

    let name = match path.file_stem().and_then(|name| name.to_str()) {
        Some(name) => match Hostname::from_str(name) {
            Ok(name) => name,
            Err(error) => {
                error!(
                    scope,
                    source:% = path.display();
                    "invalid file name: {}",
                    error
                );

                return Err(Terminate);
            }
        },
        None => {
            error!(
                scope,
                source:% = path.display();
                "file name must be valid Unicode",
            );

            return Err(Terminate);
        }
    };

    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(error) => {
            error!(
                scope,
                source:% = path.display();
                "{}",
                error
            );

            return Err(Terminate);
        }
    };

    match toml::from_str::<T>(&contents) {
        Ok(value) => Ok((name, value)),
        Err(error) => {
            error!(
                scope,
                source:% = path.display();
                "{}",
                error.to_string().trim_end()
            );

            Err(Terminate)
        }
    }
}
