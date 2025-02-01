pub mod apt;
pub mod directory;
pub mod file;
pub mod group;
pub mod host;
mod resolve;
pub mod symlink;
pub mod user;

pub use apt::package::Package as AptPackage;
pub use directory::Directory;
pub use file::File;
pub use group::Group;
pub use host::Host;
pub use resolve::{Resolvable, UnresolvedNode};
pub use symlink::Symlink;
pub use user::User;

use crate::configuration::Source;
use common::{
    resources::{
        apt::package::Name as AptPackageName, group::Name as GroupName, user::Name as UserName,
    },
    ResourceMetadata, ResourceType, SafePathBuf,
};
use serde::Serialize;
use std::{collections::HashMap, net::IpAddr, str::FromStr};
use strict_yaml_rust::{strict_yaml::Hash, StrictYaml};
use uuid::Uuid;

macro_rules! impl_resources {
    ($( $resource:ident ),*) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(untagged)]
        pub enum Resource {
            $(
                $resource($resource),
            )*
        }

        $(
            impl From<$resource> for Resource {
                fn from(resource: $resource) -> Self {
                    Self::$resource(resource)
                }
            }
        )*

        impl Resource {
            pub fn id(&self) -> Uuid {
                match self {
                    $(
                        Self::$resource(resource) => resource.id(),
                    )*
                }
            }

            pub fn kind(&self) -> ResourceType {
                match self {
                    $(
                        Self::$resource(resource) => resource.kind(),
                    )*
                }
            }

            pub fn repr(&self) -> String {
                match self {
                    $(
                        Self::$resource(resource) => resource.repr(),
                    )*
                }
            }

            pub fn metadata(&self) -> &ResourceMetadata {
                match self {
                    $(
                        Self::$resource(resource) => resource.metadata(),
                    )*
                }
            }

            pub fn may_depend_on(&self, other: &Self) -> bool {
                match self {
                    $(
                        Self::$resource(resource) => resource.may_depend_on(other),
                    )*
                }
            }

            pub fn must_depend_on(&self, other: &Self) -> bool {
                match self {
                    $(
                        Self::$resource(resource) => resource.must_depend_on(other),
                    )*
                }
            }

            pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
                match self {
                    $(
                        Self::$resource(resource) => resource.push_requirement(metadata),
                    )*
                }
            }
        }

        impl TryFrom<(UnresolvedResource, &HashMap<String, StrictYaml>)> for Resource {
            type Error = String;

            fn try_from(
                (resource, variables): (UnresolvedResource, &HashMap<String, StrictYaml>),
            ) -> Result<Self, Self::Error> {
                let resource = match resource {
                    $(
                        UnresolvedResource::$resource { parameters, .. } => {
                            Self::$resource($resource::try_from((parameters, variables))?)
                        }
                    )*
                };

                Ok(resource)
            }
        }
    }
}

impl_resources!(AptPackage, Directory, File, Group, Host, Symlink, User);

impl Resource {
    pub fn as_apt_package(&self) -> Option<&AptPackage> {
        match self {
            Self::AptPackage(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_directory(&self) -> Option<&Directory> {
        match self {
            Self::Directory(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_file(&self) -> Option<&File> {
        match self {
            Self::File(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_group(&self) -> Option<&Group> {
        match self {
            Self::Group(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_host(&self) -> Option<&Host> {
        match self {
            Self::Host(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_symlink(&self) -> Option<&Symlink> {
        match self {
            Self::Symlink(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_user(&self) -> Option<&User> {
        match self {
            Self::User(item) => Some(item),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Dependency {
    AptPackage { name: AptPackageName },
    Directory { path: SafePathBuf },
    File { path: SafePathBuf },
    Group { name: GroupName },
    Host { ip_address: IpAddr },
    Symlink { path: SafePathBuf },
    User { name: UserName },
}

impl Dependency {
    pub fn repr(&self) -> String {
        match self {
            Self::AptPackage { name } => format!("apt::package[{}]", name),
            Self::Directory { path } => format!("directory[{}]", path.display()),
            Self::File { path } => format!("file[{}]", path.display()),
            Self::Group { name } => format!("group[{}]", name),
            Self::Host { ip_address } => format!("host[{}]", ip_address),
            Self::Symlink { path } => format!("symlink[{}]", path.display()),
            Self::User { name } => format!("user[{}]", name),
        }
    }
}

impl TryFrom<(Source, StrictYaml)> for Dependency {
    type Error = String;

    fn try_from((source, node): (Source, StrictYaml)) -> Result<Self, Self::Error> {
        let mut hash = node
            .into_hash()
            .ok_or(format!("{}: node must be a hash", source))?;

        let kind = {
            let key = "type";

            hash.remove(&StrictYaml::String(key.into()))
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                .into_string()
                .ok_or(format!("{}: node must be a string", source.clone() + key))?
        };

        let dependency = match kind.as_str() {
            "apt::package" => {
                let key = "name";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let name = AptPackageName::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::AptPackage { name })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "directory" => {
                let key = "path";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let path = SafePathBuf::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::Directory { path })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "file" => {
                let key = "path";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let path = SafePathBuf::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::File { path })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "group" => {
                let key = "name";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let name = GroupName::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::Group { name })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "host" => {
                let key = "ip_address";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let ip_address = IpAddr::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::Host { ip_address })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "symlink" => {
                let key = "path";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let path = SafePathBuf::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::Symlink { path })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            "user" => {
                let key = "name";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => {
                            let name = UserName::from_str(&s)
                                .map_err(|error| format!("{}: {}", source.clone() + key, error))?;

                            Ok(Dependency::User { name })
                        }
                        None => Err(format!("{}: node must be a string", source.clone() + key)),
                    },
                    None => Err(format!("{}: failed to find required key `{}`", source, key)),
                }
            }
            _ => {
                return Err(format!(
                    "{}: encountered invalid value `{}`",
                    source + "type",
                    kind
                ))
            }
        };

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered invalid key `{}`", source, key));
        }

        dependency
    }
}

#[derive(Clone, Debug)]
pub enum UnresolvedResource {
    AptPackage {
        parameters: apt::package::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    Directory {
        parameters: directory::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    File {
        parameters: file::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    Group {
        parameters: group::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    Host {
        parameters: host::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    Symlink {
        parameters: symlink::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
    User {
        parameters: user::UnresolvedParameters,
        requires: Vec<Dependency>,
    },
}

impl UnresolvedResource {
    pub fn requires(&self) -> &[Dependency] {
        match self {
            Self::AptPackage { requires, .. } => requires.as_slice(),
            Self::Directory { requires, .. } => requires.as_slice(),
            Self::File { requires, .. } => requires.as_slice(),
            Self::Group { requires, .. } => requires.as_slice(),
            Self::Host { requires, .. } => requires.as_slice(),
            Self::Symlink { requires, .. } => requires.as_slice(),
            Self::User { requires, .. } => requires.as_slice(),
        }
    }

    pub fn kind(&self) -> ResourceType {
        match self {
            Self::AptPackage { parameters, .. } => parameters.kind(),
            Self::Directory { parameters, .. } => parameters.kind(),
            Self::File { parameters, .. } => parameters.kind(),
            Self::Group { parameters, .. } => parameters.kind(),
            Self::Host { parameters, .. } => parameters.kind(),
            Self::Symlink { parameters, .. } => parameters.kind(),
            Self::User { parameters, .. } => parameters.kind(),
        }
    }
}

impl TryFrom<(Source, Hash)> for UnresolvedResource {
    type Error = String;

    fn try_from((source, mut hash): (Source, Hash)) -> Result<Self, Self::Error> {
        let kind = {
            let key = "type";

            hash.remove(&StrictYaml::String(key.into()))
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                .into_string()
                .ok_or(format!("{}: node must be a string", source.clone() + key))?
        };

        let requires = {
            let key = "requires";
            let source = source.clone() + key;

            let mut array = vec![];

            if let Some(node) = hash.remove(&StrictYaml::String(key.into())) {
                for (index, item) in node
                    .into_vec()
                    .ok_or(format!("{}: node must be an array", source))?
                    .into_iter()
                    .enumerate()
                {
                    let source = source.clone() + index;

                    array.push(Dependency::try_from((source, item))?);
                }
            }

            array
        };

        let resource = {
            let key = "parameters";

            let hash = hash
                .remove(&StrictYaml::String(key.into()))
                .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                .into_hash()
                .ok_or(format!("{}: node must be a hash", source.clone() + key))?;

            match kind.as_str() {
                "apt::package" => {
                    let parameters =
                        apt::package::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::AptPackage {
                        parameters,
                        requires,
                    }
                }
                "directory" => {
                    let parameters =
                        directory::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Directory {
                        parameters,
                        requires,
                    }
                }
                "file" => {
                    let parameters =
                        file::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::File {
                        parameters,
                        requires,
                    }
                }
                "group" => {
                    let parameters =
                        group::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Group {
                        parameters,
                        requires,
                    }
                }
                "host" => {
                    let parameters =
                        host::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Host {
                        parameters,
                        requires,
                    }
                }
                "symlink" => {
                    let parameters =
                        symlink::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Symlink {
                        parameters,
                        requires,
                    }
                }
                "user" => {
                    let parameters =
                        user::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::User {
                        parameters,
                        requires,
                    }
                }
                _ => {
                    return Err(format!(
                        "{}: encountered invalid value for key `type`: `{}`",
                        source, kind
                    ))
                }
            }
        };

        if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
            return Err(format!("{}: encountered unexpected key `{}`", source, key));
        }

        Ok(resource)
    }
}
