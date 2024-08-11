pub mod directory;
pub mod file;
pub mod group;
pub mod host;
pub mod resolv_conf;
pub mod symlink;
pub mod user;

use common::ResourceMetadata;
use serde::Deserialize;
use std::{
    collections::{HashMap, VecDeque},
    fmt,
};
use ureq::Agent;
use url::Url;
use uuid::Uuid;

/// A struct containing the deserialized form of a pullconfd API error.
#[derive(Debug, Deserialize)]
pub struct Error {
    pub title: String,
    pub detail: String,
}

/// The expected payload of a pullconfd API response when the request
/// is successful.
#[derive(Debug, Deserialize)]
pub struct Resources {
    pub data: VecDeque<Resource>,
}

/// A resource from the API response that provides the client's resource
/// catalog. It must be an enum as the included data usually contains any
/// kind of resource.
/// Each of the included resource types implements the `ResourceTrait`
/// which pre-defines a lot of processing logic.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Resource {
    Directory(directory::Directory),
    File(file::File),
    Group(group::Group),
    Host(host::Host),
    #[serde(alias = "resolv.conf")]
    ResolvConf(resolv_conf::ResolvConf),
    Symlink(symlink::Symlink),
    User(user::User),
}

impl Resource {
    /// Allow calling the `id` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
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

    /// Allow calling the `repr` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
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

    /// Allow calling the `is_ready` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn is_ready(&self, applied_resources: &HashMap<Uuid, Resource>) -> bool {
        match self {
            Self::Directory(directory) => directory.is_ready(applied_resources),
            Self::File(file) => file.is_ready(applied_resources),
            Self::Group(group) => group.is_ready(applied_resources),
            Self::Host(host) => host.is_ready(applied_resources),
            Self::ResolvConf(resolv_conf) => resolv_conf.is_ready(applied_resources),
            Self::Symlink(symlink) => symlink.is_ready(applied_resources),
            Self::User(user) => user.is_ready(applied_resources),
        }
    }

    /// Allow calling the `apply` function from various resources.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn apply(
        &mut self,
        pid: u32,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        applied_resources: &HashMap<Uuid, Resource>,
    ) {
        match self {
            Self::Directory(ref mut directory) => directory.apply(pid, applied_resources),
            Self::File(ref mut file) => {
                file.apply(pid, agent, base_url, api_key, applied_resources)
            }
            Self::Group(ref mut group) => group.apply(pid, applied_resources),
            Self::Host(ref mut host) => host.apply(pid, applied_resources),
            Self::ResolvConf(ref mut resolv_conf) => resolv_conf.apply(pid, applied_resources),
            Self::Symlink(ref mut symlink) => symlink.apply(pid, applied_resources),
            Self::User(ref mut user) => user.apply(pid, applied_resources),
        }
    }

    /// Check whether the resource has been skipped.
    pub fn is_skipped(&self) -> bool {
        match self {
            Self::Directory(directory) => directory.action == Action::Skipped,
            Self::File(file) => file.action == Action::Skipped,
            Self::Group(group) => group.action == Action::Skipped,
            Self::Host(host) => host.action == Action::Skipped,
            Self::ResolvConf(resolv_conf) => resolv_conf.action == Action::Skipped,
            Self::Symlink(symlink) => symlink.action == Action::Skipped,
            Self::User(user) => user.action == Action::Skipped,
        }
    }

    /// Check whether the resource has failed to apply.
    pub fn is_failed(&self) -> bool {
        match self {
            Self::Directory(directory) => directory.action == Action::Failed,
            Self::File(file) => file.action == Action::Failed,
            Self::Group(group) => group.action == Action::Failed,
            Self::Host(host) => host.action == Action::Failed,
            Self::ResolvConf(resolv_conf) => resolv_conf.action == Action::Failed,
            Self::Symlink(symlink) => symlink.action == Action::Failed,
            Self::User(user) => user.action == Action::Failed,
        }
    }

    /// Check whether the resource is set to absent.
    pub fn is_absent(&self) -> bool {
        match self {
            Self::Directory(directory) => directory.parameters.ensure.is_absent(),
            Self::File(file) => file.parameters.ensure.is_absent(),
            Self::Group(group) => group.parameters.ensure.is_absent(),
            Self::Host(host) => host.parameters.ensure.is_absent(),
            Self::ResolvConf(resolv_conf) => resolv_conf.parameters.ensure.is_absent(),
            Self::Symlink(symlink) => symlink.parameters.ensure.is_absent(),
            Self::User(user) => user.parameters.ensure.is_absent(),
        }
    }
}

/// This enum describes possible actions that are the result of
/// applying a resource.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub enum Action {
    // This variant applies when a resource remains unchanged,
    // either present or absent.
    #[default]
    Unchanged,
    // This variant applies when a resource needed to be created
    // because it did not exist before.
    Created,
    // This variant applies when a resource exists but needed to
    // be changed in order to reach the desired state.
    Changed,
    // This variant applies when a resource has been deleted.
    Deleted,
    // This variant applies whenever any preconditions hinder the
    // resource from being applied.
    // This is usually the case when a dependency of this resource
    // failed to apply or has been skipped itself.
    Skipped,
    // This variant applies when a resource could not successfully
    // be configured according to its desired state.
    // It also applies when certain preconditions fail, e.g. when
    // a dependency of this resource is absent.
    Failed,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => f.write_str("unchanged"),
            Self::Created => f.write_str("created"),
            Self::Changed => f.write_str("changed"),
            Self::Deleted => f.write_str("deleted"),
            Self::Skipped => f.write_str("skipped"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

pub trait ResourceTrait {
    /// Return a textual representation of this type of resource, e.g.
    /// "directory".
    fn kind(&self) -> &str;

    /// Return a textual representation of the primary parameter of this
    /// resource. For example the primary parameter of a file resource
    /// is a filesystem path, while the primary parameter of a host
    /// resource is an IP address.
    fn display(&self) -> String;

    /// Return a concatenated string from the output of the two functions
    /// above. This representation is primarily used in logs.
    fn repr(&self) -> String {
        format!("{} `{}`", self.kind(), self.display())
    }

    /// Return the UUID of this resource as assigned by pullconfd.
    fn id(&self) -> Uuid;

    /// Check if this resource must in fact be applied, which depends on its
    /// dependencies. If they returned certain values, this resource can be
    /// skipped (Action::Skipped).
    /// There might also be cases were this resource's state interferes
    /// with that of a dependency, in which case this resource fails
    /// (Action::Failed).
    fn maybe_return_early(
        &self,
        pid: u32,
        applied_resources: &HashMap<Uuid, Resource>,
    ) -> Option<Action>;

    /// Check any prerequisites that are needed for this resource to
    /// function properly. For example a resource may depend on a
    /// certain program to be installed because it is used when the
    /// resource is applied. When the program cannot be found the
    /// resource should fail early to avoid failing when it is applied
    /// and possibly leaving the resource in a half-applied state.
    fn check_prerequisites(&self, _pid: u32) -> Option<Action> {
        None
    }

    /// Return a collection of resource metadata that points at
    /// resources that the implementing resource depends on.
    fn dependencies(&self) -> &[ResourceMetadata];

    /// Determine if this resource is ready to be applied by checking if each of
    /// its dependencies has been applied.
    fn is_ready(&self, applied_resources: &HashMap<Uuid, Resource>) -> bool {
        self.dependencies().is_empty()
            || self
                .dependencies()
                .iter()
                .all(|dependency| applied_resources.contains_key(&dependency.id))
    }

    /// Find the first dependency that can be found in the collection of
    /// already applied resources that has failed.
    fn find_failed_dependency<'a>(
        &'a self,
        applied_resources: &'a HashMap<Uuid, Resource>,
    ) -> Option<&Resource> {
        self.dependencies().iter().find_map(|dependency| {
            applied_resources
                .get(&dependency.id)
                .filter(|resource| resource.is_failed())
        })
    }

    /// Find the first dependency that can be found in the collection of
    /// already applied resources that has been skipped.
    fn find_skipped_dependency<'a>(
        &'a self,
        applied_resources: &'a HashMap<Uuid, Resource>,
    ) -> Option<&Resource> {
        self.dependencies().iter().find_map(|dependency| {
            applied_resources
                .get(&dependency.id)
                .filter(|resource| resource.is_skipped())
        })
    }

    /// Find the first dependency that can be found in the collection of
    /// already applied resources that is set to absent.
    fn find_absent_dependency<'a>(
        &'a self,
        applied_resources: &'a HashMap<Uuid, Resource>,
    ) -> Option<&Resource> {
        self.dependencies().iter().find_map(|dependency| {
            applied_resources
                .get(&dependency.id)
                .filter(|resource| resource.is_absent())
        })
    }
}
