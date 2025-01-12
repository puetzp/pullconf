use log::debug;
use std::{env, net::SocketAddr, path::PathBuf, str::FromStr};

pub enum FileType {
    Directory,
    File,
}

pub fn parse_path(kind: FileType, variable: &str, default: &str) -> Result<PathBuf, String> {
    match env::var(variable).ok() {
        Some(v) => {
            let path =
                PathBuf::from_str(&v).map_err(|error| format!("`{}`: {}", variable, error))?;

            match kind {
                FileType::Directory => {
                    if !path.is_dir() || !path.is_absolute() {
                        return  Err(format!(
                            "`{}`: value must be an absolute path pointing to an existing directory", variable
                        ));
                    }
                }
                FileType::File => {
                    if !path.is_file() || !path.is_absolute() {
                        return Err(format!(
                            "`{}`: value must be an absolute path pointing to an existing file",
                            variable
                        ));
                    }
                }
            }

            let path = path
                .canonicalize()
                .map_err(|error| format!("`{}`: {}", variable, error))?;

            debug!(
                "`{}`: environment variable evaluates to `{}`",
                variable,
                path.display()
            );

            Ok(path)
        }
        None => {
            debug!(
                "`{}`: environment variable not found, using default `{}`",
                variable, default
            );
            Ok(PathBuf::from_str(default).unwrap())
        }
    }
}

pub fn parse_socket(variable: &str, default: &str) -> Result<SocketAddr, String> {
    match env::var(variable).ok() {
        Some(v) => {
            let addr =
                SocketAddr::from_str(&v).map_err(|error| format!("`{}`: {}", variable, error))?;

            debug!(
                "`{}`: environment variable evaluates to `{}`",
                variable, addr
            );

            Ok(addr)
        }
        None => {
            debug!(
                "`{}`: environment variable not found, using default `{}`",
                variable, default
            );
            Ok(SocketAddr::from_str(default).unwrap())
        }
    }
}
