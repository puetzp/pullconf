pub mod deserialize;
pub mod directory;
pub mod file;
pub mod group;
pub mod host;
pub mod resolv_conf;
pub mod symlink;
pub mod user;

pub use directory::Directory;
pub use file::File;
pub use group::Group;
pub use host::Host;
pub use resolv_conf::ResolvConf;
pub use symlink::Symlink;
pub use user::User;

use common::ResourceMetadata;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Resource<'a> {
    Directory(&'a Directory),
    File(&'a File),
    Group(&'a Group),
    Host(&'a Host),
    ResolvConf(&'a ResolvConf),
    Symlink(&'a Symlink),
    User(&'a User),
}

impl<'a> From<&'a Directory> for Resource<'a> {
    fn from(directory: &'a Directory) -> Self {
        Self::Directory(directory)
    }
}

impl<'a> From<&'a File> for Resource<'a> {
    fn from(file: &'a File) -> Self {
        Self::File(file)
    }
}

impl<'a> From<&'a Group> for Resource<'a> {
    fn from(group: &'a Group) -> Self {
        Self::Group(group)
    }
}

impl<'a> From<&'a Host> for Resource<'a> {
    fn from(host: &'a Host) -> Self {
        Self::Host(host)
    }
}

impl<'a> From<&'a ResolvConf> for Resource<'a> {
    fn from(resolv_conf: &'a ResolvConf) -> Self {
        Self::ResolvConf(resolv_conf)
    }
}

impl<'a> From<&'a Symlink> for Resource<'a> {
    fn from(symlink: &'a Symlink) -> Self {
        Self::Symlink(symlink)
    }
}

impl<'a> From<&'a User> for Resource<'a> {
    fn from(user: &'a User) -> Self {
        Self::User(user)
    }
}

impl<'a> Resource<'a> {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Directory(directory) => directory.id(),
            Self::File(file) => file.id(),
            Self::Group(group) => group.id(),
            Self::Host(host) => host.id(),
            Self::ResolvConf(resolv_conf) => resolv_conf.id(),
            Self::Symlink(symlink) => symlink.id(),
            Self::User(user) => user.id(),
        }
    }

    pub fn repr(&self) -> String {
        match self {
            Self::Directory(directory) => directory.repr(),
            Self::File(file) => file.repr(),
            Self::Group(group) => group.repr(),
            Self::Host(host) => host.repr(),
            Self::ResolvConf(resolv_conf) => resolv_conf.repr(),
            Self::Symlink(symlink) => symlink.repr(),
            Self::User(user) => user.repr(),
        }
    }

    pub fn metadata(&self) -> &ResourceMetadata {
        match self {
            Self::Directory(directory) => directory.metadata(),
            Self::File(file) => file.metadata(),
            Self::Group(group) => group.metadata(),
            Self::Host(host) => host.metadata(),
            Self::ResolvConf(resolv_conf) => resolv_conf.metadata(),
            Self::Symlink(symlink) => symlink.metadata(),
            Self::User(user) => user.metadata(),
        }
    }
}
