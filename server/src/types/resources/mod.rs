pub mod apt;
pub mod directory;
pub mod execute;
pub mod file;
pub mod group;
pub mod host;
mod resolve;
pub mod symlink;
pub mod user;

pub use apt::package::Package as AptPackage;
pub use directory::Directory;
pub use execute::Execute;
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
    Action, ResourceMetadata, ResourceType, SafePathBuf, TriggerMetadata,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    str::FromStr,
};
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

            pub fn push_predecessor(&mut self, metadata: ResourceMetadata) {
                match self {
                    $(
                        Self::$resource(resource) => resource.push_predecessor(metadata),
                    )*
                }
            }

            pub fn push_trigger(&mut self, metadata: TriggerMetadata) {
                match self {
                    $(
                        Self::$resource(resource) => resource.push_trigger(metadata),
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

impl_resources!(AptPackage, Directory, Execute, File, Group, Host, Symlink, User);

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

    pub fn as_execute(&self) -> Option<&Execute> {
        match self {
            Self::Execute(item) => Some(item),
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
    Execute { name: String },
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
            Self::Execute { name } => format!("execute[{}]", name),
            Self::File { path } => format!("file[{}]", path.display()),
            Self::Group { name } => format!("group[{}]", name),
            Self::Host { ip_address } => format!("host[{}]", ip_address),
            Self::Symlink { path } => format!("symlink[{}]", path.display()),
            Self::User { name } => format!("user[{}]", name),
        }
    }
}

impl PartialEq<Resource> for Dependency {
    fn eq(&self, resource: &Resource) -> bool {
        match resource {
            Resource::AptPackage(package) => {
                matches!(self, Self::AptPackage { name } if *name == package.parameters.name)
            }
            Resource::Directory(directory) => {
                matches!(self, Self::Directory { path } if *path == directory.parameters.path)
            }
            Resource::Execute(execute) => {
                matches!(self, Self::Execute { name } if *name == execute.parameters.name)
            }
            Resource::File(file) => {
                matches!(self, Self::File { path } if *path == file.parameters.path)
            }
            Resource::Group(group) => {
                matches!(self, Self::Group { name } if *name == group.parameters.name)
            }
            Resource::Host(host) => {
                matches!(self, Self::Host { ip_address } if *ip_address == host.parameters.ip_address)
            }
            Resource::Symlink(symlink) => {
                matches!(self, Self::Symlink { path } if *path == symlink.parameters.path)
            }
            Resource::User(user) => {
                matches!(self, Self::User { name } if *name == user.parameters.name)
            }
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
            "execute" => {
                let key = "name";

                match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => Ok(Dependency::Execute {
                            name: s.to_string(),
                        }),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Trigger {
    Execute { name: String, when: Vec<Action> },
}

impl Trigger {
    pub fn repr(&self) -> String {
        match self {
            Self::Execute { name, .. } => format!("execute[{}]", name),
        }
    }
}

impl PartialEq<Resource> for Trigger {
    fn eq(&self, resource: &Resource) -> bool {
        match resource {
            Resource::Execute(execute) => {
                matches!(self, Self::Execute { name, .. } if *name == execute.parameters.name)
            }
            _ => false,
        }
    }
}

impl TryFrom<(Source, StrictYaml)> for Trigger {
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

        let trigger = match kind.as_str() {
            "execute" => {
                let key = "name";

                let name = match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_string() {
                        Some(s) => s,
                        None => {
                            return Err(format!("{}: node must be a string", source.clone() + key))
                        }
                    },
                    None => {
                        return Err(format!("{}: failed to find required key `{}`", source, key))
                    }
                };

                let key = "when";

                let when = match hash.remove(&StrictYaml::String(key.into())) {
                    Some(node) => match node.into_vec() {
                        Some(v) => {
                            let mut actions = HashSet::new();

                            for (index, item) in v.into_iter().enumerate() {
                                let source = source.clone() + key + index;

                                let s = item
                                    .into_string()
                                    .ok_or(format!("{}: node must be a string", source))?;

                                let action = Action::from_str(&s)
                                    .map_err(|error| format!("{}: {}", source, error))?;

                                if ![Action::Created, Action::Deleted, Action::Changed]
                                    .contains(&action)
                                {
                                    return Err(format!(
                                        "{}: value must be one of `created`, `deleted`, `changed`",
                                        source
                                    ));
                                }

                                if !actions.insert(action) {
                                    return Err(format!("{}: found duplicate value", source));
                                }
                            }

                            Vec::from_iter(actions)
                        }
                        None => {
                            return Err(format!("{}: node must be an array", source.clone() + key))
                        }
                    },
                    None => vec![Action::Created, Action::Deleted, Action::Changed],
                };

                Ok(Trigger::Execute { name, when })
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

        trigger
    }
}

#[derive(Clone, Debug)]
pub enum UnresolvedResource {
    AptPackage {
        parameters: apt::package::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    Directory {
        parameters: directory::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    Execute {
        parameters: execute::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    File {
        parameters: file::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    Group {
        parameters: group::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    Host {
        parameters: host::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    Symlink {
        parameters: symlink::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
    User {
        parameters: user::UnresolvedParameters,
        requires: Vec<Dependency>,
        triggers: Vec<Trigger>,
    },
}

impl UnresolvedResource {
    pub fn requires(&self) -> &[Dependency] {
        match self {
            Self::AptPackage { requires, .. } => requires.as_slice(),
            Self::Directory { requires, .. } => requires.as_slice(),
            Self::Execute { requires, .. } => requires.as_slice(),
            Self::File { requires, .. } => requires.as_slice(),
            Self::Group { requires, .. } => requires.as_slice(),
            Self::Host { requires, .. } => requires.as_slice(),
            Self::Symlink { requires, .. } => requires.as_slice(),
            Self::User { requires, .. } => requires.as_slice(),
        }
    }

    pub fn triggers(&self) -> &[Trigger] {
        match self {
            Self::AptPackage { triggers, .. } => triggers.as_slice(),
            Self::Directory { triggers, .. } => triggers.as_slice(),
            Self::Execute { triggers, .. } => triggers.as_slice(),
            Self::File { triggers, .. } => triggers.as_slice(),
            Self::Group { triggers, .. } => triggers.as_slice(),
            Self::Host { triggers, .. } => triggers.as_slice(),
            Self::Symlink { triggers, .. } => triggers.as_slice(),
            Self::User { triggers, .. } => triggers.as_slice(),
        }
    }

    pub fn kind(&self) -> ResourceType {
        match self {
            Self::AptPackage { parameters, .. } => parameters.kind(),
            Self::Directory { parameters, .. } => parameters.kind(),
            Self::Execute { parameters, .. } => parameters.kind(),
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

        let triggers = {
            let key = "triggers";
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

                    array.push(Trigger::try_from((source.clone(), item))?);
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
                        triggers,
                    }
                }
                "directory" => {
                    let parameters =
                        directory::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Directory {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "execute" => {
                    let parameters =
                        execute::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Execute {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "file" => {
                    let parameters =
                        file::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::File {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "group" => {
                    let parameters =
                        group::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Group {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "host" => {
                    let parameters =
                        host::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Host {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "symlink" => {
                    let parameters =
                        symlink::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::Symlink {
                        parameters,
                        requires,
                        triggers,
                    }
                }
                "user" => {
                    let parameters =
                        user::UnresolvedParameters::try_from((source.clone() + key, hash))?;

                    Self::User {
                        parameters,
                        requires,
                        triggers,
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
