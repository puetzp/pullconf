use common::error::Terminate;
use log::{debug, error};
use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

pub enum FileType {
    Directory,
    File,
}

pub fn parse_path(kind: FileType, variable: &str, default: &str) -> Result<PathBuf, Terminate> {
    let scope = "environment";

    match env::var(variable).ok() {
        Some(v) => {
            let path = match PathBuf::from_str(&v) {
                Ok(p) => p,
                Err(error) => {
                    error!(scope, variable; "{}", error);
                    return Err(Terminate);
                }
            };

            match kind {
                FileType::Directory => {
                    if !path.is_dir() || !path.is_absolute() {
                        error!(
                            scope,
                            variable;
                            "value must be an absolute path pointing to an existing directory"
                        );
                        return Err(Terminate);
                    }
                }
                FileType::File => {
                    if !path.is_file() || !path.is_absolute() {
                        error!(
                            scope,
                            variable;
                            "value must be an absolute path pointing to an existing file"
                        );
                        return Err(Terminate);
                    }
                }
            }

            let path = match path.canonicalize() {
                Ok(p) => p,
                Err(error) => {
                    error!(scope, variable; "{}", error);
                    return Err(Terminate);
                }
            };

            debug!(scope, variable; "variable evaluates to {}", path.display());

            Ok(path)
        }
        None => {
            debug!(scope, variable; "variable not found, using default {}", default);
            Ok(PathBuf::from_str(default).unwrap())
        }
    }
}

pub fn parse_socket(variable: &str, default: &str) -> Result<SocketAddr, Terminate> {
    let scope = "environment";

    match env::var(variable).ok() {
        Some(v) => {
            let addr = match SocketAddr::from_str(&v) {
                Ok(s) => s,
                Err(error) => {
                    error!(scope, variable; "{}", error);
                    return Err(Terminate);
                }
            };

            debug!(scope, variable; "variable evaluates to {}", addr);

            Ok(addr)
        }
        None => {
            debug!(scope, variable; "variable not found, using default {}", default);
            Ok(SocketAddr::from_str(default).unwrap())
        }
    }
}
