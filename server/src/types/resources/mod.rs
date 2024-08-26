pub mod apt;
pub mod deserialize;
pub mod directory;
pub mod file;
pub mod group;
pub mod host;
pub mod resolv_conf;
pub mod symlink;
pub mod user;

pub use apt::package::Package as AptPackage;
pub use apt::preference::Preference as AptPreference;
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Resource {
    AptPackage(AptPackage),
    AptPreference(AptPreference),
    Directory(Directory),
    File(File),
    Group(Group),
    Host(Host),
    ResolvConf(ResolvConf),
    Symlink(Symlink),
    User(User),
}

impl From<AptPackage> for Resource {
    fn from(package: AptPackage) -> Self {
        Self::AptPackage(package)
    }
}

impl From<AptPreference> for Resource {
    fn from(preference: AptPreference) -> Self {
        Self::AptPreference(preference)
    }
}

impl From<Directory> for Resource {
    fn from(directory: Directory) -> Self {
        Self::Directory(directory)
    }
}

impl From<File> for Resource {
    fn from(file: File) -> Self {
        Self::File(file)
    }
}

impl From<Group> for Resource {
    fn from(group: Group) -> Self {
        Self::Group(group)
    }
}

impl From<Host> for Resource {
    fn from(host: Host) -> Self {
        Self::Host(host)
    }
}

impl From<ResolvConf> for Resource {
    fn from(resolv_conf: ResolvConf) -> Self {
        Self::ResolvConf(resolv_conf)
    }
}

impl From<Symlink> for Resource {
    fn from(symlink: Symlink) -> Self {
        Self::Symlink(symlink)
    }
}

impl From<User> for Resource {
    fn from(user: User) -> Self {
        Self::User(user)
    }
}

impl Resource {
    pub fn id(&self) -> Uuid {
        match self {
            Self::AptPackage(package) => package.id(),
            Self::AptPreference(preference) => preference.id(),
            Self::Directory(directory) => directory.id(),
            Self::File(file) => file.id(),
            Self::Group(group) => group.id(),
            Self::Host(host) => host.id(),
            Self::ResolvConf(resolv_conf) => resolv_conf.id(),
            Self::Symlink(symlink) => symlink.id(),
            Self::User(user) => user.id(),
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            Self::AptPackage(package) => package.kind(),
            Self::AptPreference(preference) => preference.kind(),
            Self::Directory(directory) => directory.kind(),
            Self::File(file) => file.kind(),
            Self::Group(group) => group.kind(),
            Self::Host(host) => host.kind(),
            Self::ResolvConf(resolv_conf) => resolv_conf.kind(),
            Self::Symlink(symlink) => symlink.kind(),
            Self::User(user) => user.kind(),
        }
    }

    pub fn repr(&self) -> String {
        match self {
            Self::AptPackage(package) => package.repr(),
            Self::AptPreference(preference) => preference.repr(),
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
            Self::AptPackage(package) => package.metadata(),
            Self::AptPreference(preference) => preference.metadata(),
            Self::Directory(directory) => directory.metadata(),
            Self::File(file) => file.metadata(),
            Self::Group(group) => group.metadata(),
            Self::Host(host) => host.metadata(),
            Self::ResolvConf(resolv_conf) => resolv_conf.metadata(),
            Self::Symlink(symlink) => symlink.metadata(),
            Self::User(user) => user.metadata(),
        }
    }

    pub fn as_apt_package(&self) -> Option<&AptPackage> {
        match self {
            Self::AptPackage(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_apt_preference(&self) -> Option<&AptPreference> {
        match self {
            Self::AptPreference(item) => Some(item),
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

    pub fn as_resolv_conf(&self) -> Option<&ResolvConf> {
        match self {
            Self::ResolvConf(item) => Some(item),
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
