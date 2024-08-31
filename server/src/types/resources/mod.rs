pub mod apt;
pub mod cron;
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
pub use cron::job::Job as CronJob;
pub use directory::Directory;
pub use file::File;
pub use group::Group;
pub use host::Host;
pub use resolv_conf::ResolvConf;
pub use symlink::Symlink;
pub use user::User;

use common::{ResourceMetadata, ResourceType};
use deserialize::Resource as DeResource;
use serde::Serialize;
use std::collections::HashMap;
use toml::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Resource {
    AptPackage(AptPackage),
    AptPreference(AptPreference),
    CronJob(CronJob),
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

impl From<CronJob> for Resource {
    fn from(job: CronJob) -> Self {
        Self::CronJob(job)
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
            Self::CronJob(job) => job.id(),
            Self::Directory(directory) => directory.id(),
            Self::File(file) => file.id(),
            Self::Group(group) => group.id(),
            Self::Host(host) => host.id(),
            Self::ResolvConf(resolv_conf) => resolv_conf.id(),
            Self::Symlink(symlink) => symlink.id(),
            Self::User(user) => user.id(),
        }
    }

    pub fn kind(&self) -> ResourceType {
        match self {
            Self::AptPackage(package) => package.kind(),
            Self::AptPreference(preference) => preference.kind(),
            Self::CronJob(job) => job.kind(),
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
            Self::CronJob(job) => job.repr(),
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
            Self::CronJob(job) => job.metadata(),
            Self::Directory(directory) => directory.metadata(),
            Self::File(file) => file.metadata(),
            Self::Group(group) => group.metadata(),
            Self::Host(host) => host.metadata(),
            Self::ResolvConf(resolv_conf) => resolv_conf.metadata(),
            Self::Symlink(symlink) => symlink.metadata(),
            Self::User(user) => user.metadata(),
        }
    }

    pub fn may_depend_on(&self, other: &Self) -> bool {
        match self {
            Self::AptPackage(item) => item.may_depend_on(other),
            Self::AptPreference(item) => item.may_depend_on(other),
            Self::CronJob(item) => item.may_depend_on(other),
            Self::Directory(item) => item.may_depend_on(other),
            Self::File(item) => item.may_depend_on(other),
            Self::Group(item) => item.may_depend_on(other),
            Self::Host(item) => item.may_depend_on(other),
            Self::ResolvConf(item) => item.may_depend_on(other),
            Self::Symlink(item) => item.may_depend_on(other),
            Self::User(item) => item.may_depend_on(other),
        }
    }

    pub fn push_requirement(&mut self, metadata: ResourceMetadata) {
        match self {
            Self::AptPackage(item) => item.push_requirement(metadata),
            Self::AptPreference(item) => item.push_requirement(metadata),
            Self::CronJob(item) => item.push_requirement(metadata),
            Self::Directory(item) => item.push_requirement(metadata),
            Self::File(item) => item.push_requirement(metadata),
            Self::Group(item) => item.push_requirement(metadata),
            Self::Host(item) => item.push_requirement(metadata),
            Self::ResolvConf(item) => item.push_requirement(metadata),
            Self::Symlink(item) => item.push_requirement(metadata),
            Self::User(item) => item.push_requirement(metadata),
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

    pub fn as_cron_job(&self) -> Option<&CronJob> {
        match self {
            Self::CronJob(item) => Some(item),
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

impl TryFrom<(&DeResource, &HashMap<String, Value>)> for Resource {
    type Error = String;

    fn try_from(
        (resource, variables): (&DeResource, &HashMap<String, Value>),
    ) -> Result<Self, Self::Error> {
        let resource = match resource {
            DeResource::AptPackage(item) => {
                Self::AptPackage(AptPackage::try_from((item, variables))?)
            }
            DeResource::AptPreference(item) => {
                Self::AptPreference(AptPreference::try_from((item, variables))?)
            }
            DeResource::CronJob(item) => Self::CronJob(CronJob::try_from((item, variables))?),
            DeResource::Directory(item) => Self::Directory(Directory::try_from((item, variables))?),
            DeResource::File(item) => Self::File(File::try_from((item, variables))?),
            DeResource::Group(item) => Self::Group(Group::try_from((item, variables))?),
            DeResource::Host(item) => Self::Host(Host::try_from((item, variables))?),
            DeResource::ResolvConf(item) => {
                Self::ResolvConf(ResolvConf::try_from((item, variables))?)
            }
            DeResource::Symlink(item) => Self::Symlink(Symlink::try_from((item, variables))?),
            DeResource::User(item) => Self::User(User::try_from((item, variables))?),
        };

        Ok(resource)
    }
}
