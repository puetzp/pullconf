use crate::types::resources::{
    deserialize::Resource, directory, file, group, host, resolv_conf, symlink, user,
};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(from = "deserialize::Group")]
pub struct Group {
    pub directories: Vec<directory::de::Parameters>,
    pub files: Vec<file::de::Parameters>,
    pub groups: Vec<group::de::Parameters>,
    pub hosts: Vec<host::de::Parameters>,
    pub resolv_conf: Vec<resolv_conf::de::Parameters>,
    pub symlinks: Vec<symlink::de::Parameters>,
    pub users: Vec<user::de::Parameters>,
}

impl From<deserialize::Group> for Group {
    fn from(intermediate: deserialize::Group) -> Self {
        let mut group = Self::default();

        for resource in intermediate.resources {
            match resource {
                Resource::Directory(directory) => group.directories.push(directory),
                Resource::File(file) => group.files.push(file),
                Resource::Group(_group) => group.groups.push(_group),
                Resource::Host(host) => group.hosts.push(host),
                Resource::ResolvConf(resolv_conf) => group.resolv_conf.push(resolv_conf),
                Resource::Symlink(symlink) => group.symlinks.push(symlink),
                Resource::User(user) => group.users.push(user),
            }
        }

        group
    }
}

mod deserialize {
    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Group {
        #[serde(default)]
        pub resources: Vec<Resource>,
    }
}
