pub mod apt;
pub mod directory;
pub mod execute;
pub mod file;
pub mod group;
pub mod host;
pub mod symlink;
pub mod user;

use common::{Action, ResourceMetadata, TriggerMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use ureq::Agent;
use url::Url;

/// A struct containing the deserialized form of a pullconfd API error.
#[derive(Debug, Deserialize)]
pub struct Error {
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ResourceResult {
    pub order: usize,
    pub action: Action,
    pub message: Option<String>,
    pub duration: usize,
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Resource {
    #[serde(rename = "apt::package")]
    AptPackage(apt::package::Package),
    Directory(directory::Directory),
    Execute(execute::Execute),
    File(file::File),
    Group(group::Group),
    Host(host::Host),
    Symlink(symlink::Symlink),
    User(user::User),
}

impl Resource {
    /// Allow calling the `id` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn id(&self) -> &str {
        match self {
            Self::AptPackage(resource) => resource.id(),
            Self::Directory(resource) => resource.id(),
            Self::Execute(resource) => resource.id(),
            Self::File(resource) => resource.id(),
            Self::Group(resource) => resource.id(),
            Self::Host(resource) => resource.id(),
            Self::Symlink(resource) => resource.id(),
            Self::User(resource) => resource.id(),
        }
    }

    /// Allow calling the `repr` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn repr(&self) -> String {
        match self {
            Self::AptPackage(resource) => resource.repr(),
            Self::Directory(resource) => resource.repr(),
            Self::Execute(resource) => resource.repr(),
            Self::File(resource) => resource.repr(),
            Self::Group(resource) => resource.repr(),
            Self::Host(resource) => resource.repr(),
            Self::Symlink(resource) => resource.repr(),
            Self::User(resource) => resource.repr(),
        }
    }

    /// Allow calling the `is_ready` function from resources implementing the
    /// `ResourceTrait`.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn is_ready(&self, applied_resources: &HashMap<String, Resource>) -> bool {
        match self {
            Self::AptPackage(resource) => resource.is_ready(applied_resources),
            Self::Directory(resource) => resource.is_ready(applied_resources),
            Self::Execute(resource) => resource.is_ready(applied_resources),
            Self::File(resource) => resource.is_ready(applied_resources),
            Self::Group(resource) => resource.is_ready(applied_resources),
            Self::Host(resource) => resource.is_ready(applied_resources),
            Self::Symlink(resource) => resource.is_ready(applied_resources),
            Self::User(resource) => resource.is_ready(applied_resources),
        }
    }

    /// Allow calling the `apply` function from various resources.
    /// This shortcut allows the calling function to skip the usual pattern
    /// matching stuff to infer the resource type.
    pub fn apply(
        &mut self,
        order: usize,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        applied_resources: &HashMap<String, Resource>,
    ) {
        match self {
            Self::AptPackage(ref mut resource) => resource.apply(order, applied_resources),
            Self::Directory(ref mut resource) => resource.apply(order, applied_resources),
            Self::Execute(ref mut resource) => resource.apply(order, applied_resources),
            Self::File(ref mut resource) => {
                resource.apply(agent, base_url, api_key, order, applied_resources)
            }
            Self::Group(ref mut resource) => resource.apply(order, applied_resources),
            Self::Host(ref mut resource) => resource.apply(order, applied_resources),
            Self::Symlink(ref mut resource) => resource.apply(order, applied_resources),
            Self::User(ref mut resource) => resource.apply(order, applied_resources),
        }
    }

    /// Check whether the resource has been skipped.
    pub fn is_skipped(&self) -> bool {
        match self {
            Self::AptPackage(resource) => resource.result.action == Action::Skipped,
            Self::Directory(resource) => resource.result.action == Action::Skipped,
            Self::Execute(resource) => resource.result.action == Action::Skipped,
            Self::File(resource) => resource.result.action == Action::Skipped,
            Self::Group(resource) => resource.result.action == Action::Skipped,
            Self::Host(resource) => resource.result.action == Action::Skipped,
            Self::Symlink(resource) => resource.result.action == Action::Skipped,
            Self::User(resource) => resource.result.action == Action::Skipped,
        }
    }

    /// Check whether the resource has failed to apply.
    pub fn is_failed(&self) -> bool {
        match self {
            Self::AptPackage(resource) => resource.result.action == Action::Failed,
            Self::Directory(resource) => resource.result.action == Action::Failed,
            Self::Execute(resource) => resource.result.action == Action::Failed,
            Self::File(resource) => resource.result.action == Action::Failed,
            Self::Group(resource) => resource.result.action == Action::Failed,
            Self::Host(resource) => resource.result.action == Action::Failed,
            Self::Symlink(resource) => resource.result.action == Action::Failed,
            Self::User(resource) => resource.result.action == Action::Failed,
        }
    }

    /// Check whether the resource is set to absent.
    pub fn is_absent(&self) -> bool {
        match self {
            Self::AptPackage(resource) => {
                resource.parameters.ensure.is_absent() || resource.parameters.ensure.is_purged()
            }
            Self::Directory(resource) => resource.parameters.ensure.is_absent(),
            Self::Execute(resource) => resource.parameters.ensure.is_absent(),
            Self::File(resource) => resource.parameters.ensure.is_absent(),
            Self::Group(resource) => resource.parameters.ensure.is_absent(),
            Self::Host(resource) => resource.parameters.ensure.is_absent(),
            Self::Symlink(resource) => resource.parameters.ensure.is_absent(),
            Self::User(resource) => resource.parameters.ensure.is_absent(),
        }
    }

    /// This is a shortcut to the list of metadata of resources
    /// that are triggered by this resource.
    pub fn triggers(&self) -> &[TriggerMetadata] {
        match self {
            Self::AptPackage(resource) => resource.triggers(),
            Self::Directory(resource) => resource.triggers(),
            Self::Execute(resource) => resource.triggers(),
            Self::File(resource) => resource.triggers(),
            Self::Group(resource) => resource.triggers(),
            Self::Host(resource) => resource.triggers(),
            Self::Symlink(resource) => resource.triggers(),
            Self::User(resource) => resource.triggers(),
        }
    }

    /// Return the state of the resource.
    pub fn action(&self) -> Action {
        match self {
            Self::AptPackage(resource) => resource.action(),
            Self::Directory(resource) => resource.action(),
            Self::Execute(resource) => resource.action(),
            Self::File(resource) => resource.action(),
            Self::Group(resource) => resource.action(),
            Self::Host(resource) => resource.action(),
            Self::Symlink(resource) => resource.action(),
            Self::User(resource) => resource.action(),
        }
    }

    /// Return the order at which the resource was applied.
    pub fn order(&self) -> usize {
        match self {
            Self::AptPackage(resource) => resource.order(),
            Self::Directory(resource) => resource.order(),
            Self::Execute(resource) => resource.order(),
            Self::File(resource) => resource.order(),
            Self::Group(resource) => resource.order(),
            Self::Host(resource) => resource.order(),
            Self::Symlink(resource) => resource.order(),
            Self::User(resource) => resource.order(),
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
        format!("{}[{}]", self.kind(), self.display())
    }

    /// Return the ID of this resource as assigned by pullconfd.
    fn id(&self) -> &str;

    /// Return the state of the resource.
    fn action(&self) -> Action;

    /// Return the order at which the resource was applied.
    fn order(&self) -> usize;

    /// Check if this resource must in fact be applied, which depends on its
    /// dependencies. If they returned certain values, this resource can be
    /// skipped (Action::Skipped).
    /// There might also be cases were this resource's state interferes
    /// with that of a dependency, in which case this resource fails
    /// (Action::Failed).
    fn maybe_return_early(
        &mut self,
        applied_resources: &HashMap<String, Resource>,
    ) -> Option<(Action, String)> {
        if let Some(dependency) = self.find_failed_dependency(applied_resources) {
            let message = format!(
                "skipping resource as dependency `{}` has failed to apply",
                dependency.repr()
            );
            log::warn!("`{}`: {}", self.repr(), message);
            return Some((Action::Skipped, message));
        }

        if let Some(dependency) = self.find_skipped_dependency(applied_resources) {
            let message = format!(
                "skipping resource as dependency `{}` has been skipped",
                dependency.repr()
            );
            log::warn!("`{}`: {}", self.repr(), message);
            return Some((Action::Skipped, message));
        }

        if self.is_present() {
            if let Some(dependency) = self.find_absent_dependency(applied_resources) {
                let message = format!(
                    "cannot apply resource as dependency `{}` is set to absent",
                    dependency.repr()
                );
                log::error!("`{}`: {}", self.repr(), message);
                return Some((Action::Failed, message));
            }
        }

        None
    }

    fn is_present(&self) -> bool;

    /// Check any prerequisites that are needed for this resource to
    /// function properly. For example a resource may depend on a
    /// certain program to be installed because it is used when the
    /// resource is applied. When the program cannot be found the
    /// resource should fail early to avoid failing when it is applied
    /// and possibly leaving the resource in a half-applied state.
    fn check_prerequisites(&self) -> Result<(), String> {
        Ok(())
    }

    /// Return a collection of resource metadata that points at
    /// resources that the implementing resource depends on.
    fn dependencies(&self) -> &[ResourceMetadata];

    /// Return a collection of resource metadata that points at
    /// resources that should run before this resource.
    fn predecessors(&self) -> &[ResourceMetadata];

    /// Return a collection of resource metadata that points at
    /// resources that are triggered by the implementing resource.
    fn triggers(&self) -> &[TriggerMetadata];

    /// Determine if this resource is ready to be applied by checking if each of
    /// its predecessors has been applied.
    fn is_ready(&self, applied_resources: &HashMap<String, Resource>) -> bool {
        self.predecessors().is_empty()
            || self
                .predecessors()
                .iter()
                .all(|predecessor| applied_resources.contains_key(&predecessor.id))
    }

    /// Find the first dependency that can be found in the collection of
    /// already applied resources that has failed.
    fn find_failed_dependency<'a>(
        &'a self,
        applied_resources: &'a HashMap<String, Resource>,
    ) -> Option<&'a Resource> {
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
        applied_resources: &'a HashMap<String, Resource>,
    ) -> Option<&'a Resource> {
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
        applied_resources: &'a HashMap<String, Resource>,
    ) -> Option<&'a Resource> {
        self.dependencies().iter().find_map(|dependency| {
            applied_resources
                .get(&dependency.id)
                .filter(|resource| resource.is_absent())
        })
    }
}
