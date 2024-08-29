use crate::types::{
    resources::{
        apt,
        deserialize::{Dependency, Resource as DeResource},
        directory, file, group, host, resolv_conf, symlink, user, Resource,
    },
    ApiKey, Group,
};
use common::{
    error::Terminate,
    resources::{
        apt::{package::Name as AptPackageName, preference::Name as AptPreferenceName},
        group::Name as GroupName,
        user::Name as UserName,
    },
    Hostname,
};
use log::error;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    net::IpAddr,
    path::PathBuf,
};
use uuid::Uuid;

/// This struct contains temporary helper collections that are
/// freed after configuration validation has concluded.
#[derive(Clone, Debug, Default)]
pub struct ValidationHelpers {
    /// This list contains every resource ID and the IDs of resources
    /// that each resource depends on. This is used during validation
    /// to detect dependency loops.
    pub dependencies: HashMap<Uuid, HashSet<Uuid>>,
    /// This list contains IDs from resources that were sourced/inherited
    /// from a group instead of the client configuration. The name
    /// of the group is stored in order to return accurate errors if
    /// another, conflicting resource is found and give the user a hint
    /// which groups must be reconciled.
    pub origins: HashMap<Uuid, Hostname>,
    /// This collection stores resource dependencies that were
    /// explicitly mentioned in configuration files.
    /// During validation these dependencies are resolved and
    /// the actual resource metadata of a given dependency is added
    /// to the resource relationship data.
    pub requires: HashMap<Uuid, Vec<Dependency>>,
    /// Some resources manage filesystem nodes of different types.
    /// This collection helps to ensure during validation that a node
    /// at a given path is not managed by multiple resources of the same
    /// or different kinds,
    pub paths: HashSet<PathBuf>,
    /// This collection stores paths of `file` resources, ensuring that
    /// different resources of this type do not conflict. A conflict
    /// exists when the `path` of one `file` resource happens to be the
    /// parent node to the `path` of another`, since only directories
    /// and symlinks (pointing to a directory) can be parents to a file.
    pub file_paths: HashSet<PathBuf>,
    pub host_ip_addresses: HashSet<IpAddr>,
    pub group_names: HashSet<GroupName>,
    pub user_names: HashSet<UserName>,
    pub apt_package_names: HashSet<AptPackageName>,
    pub apt_preference_names: HashSet<AptPreferenceName>,
}

impl ValidationHelpers {
    /// Replace the currently allocated collections with new, empty
    /// collections, which results in deallocating the old collections
    /// that are not longer relevant when validation is finished.
    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The `Client` struct contains all data parsed from configuration
/// files as well as temporary helper objects and collections that
/// help during resource validation.
#[derive(Clone, Debug)]
pub struct Client {
    pub name: Hostname,
    pub api_key: ApiKey,
    pub assigned_groups: Vec<Hostname>,
    pub variables: HashMap<String, toml::Value>,
    pub temporary: ValidationHelpers,
    pub resources: VecDeque<Resource>,
}

impl Hash for Client {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for Client {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Client {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Eq for Client {}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl
    TryFrom<(
        Hostname,
        deserialize::Client,
        &mut HashMap<Hostname, (Group, usize)>,
    )> for Client
{
    type Error = Terminate;

    fn try_from(
        (name, intermediate, groups): (
            Hostname,
            deserialize::Client,
            &mut HashMap<Hostname, (Group, usize)>,
        ),
    ) -> Result<Self, Self::Error> {
        let scope = "validation";

        // Initialize the client and validate the client's own configuration,
        // substituting variables in the process.
        // This does not take resources from groups into account.
        let mut client = Self {
            name,
            api_key: intermediate.api_key,
            assigned_groups: intermediate.assigned_groups,
            variables: intermediate.variables,
            temporary: ValidationHelpers::default(),
            resources: VecDeque::new(),
        };

        for item in intermediate.resources {
            let requires = item.requires().to_vec();

            // Convert resource from the deserialized to the final form,
            // substituting variables in the process.
            let resource = Resource::try_from((&item, &client.variables)).map_err(|error| {
                error!(
                    scope,
                    client:% = client.name,
                    resource = item.kind();
                    "{}",
                    error
                );
                Terminate
            })?;

            // Save dependencies as they appear in the deserialized resource.
            client.temporary.requires.insert(resource.id(), requires);

            client.resources.push_back(resource);
        }

        // Extend the client's resource catalog with resources from groups
        // that the client is a member of, substituting variables in the process.
        client.extend_from_groups(groups)?;

        client.temporary.file_paths = client
            .resources
            .iter()
            .filter_map(|resource| resource.as_file())
            .map(|file| file.parameters.path.to_path_buf())
            .collect();

        client.validate()?;

        client.temporary.clear();

        Ok(client)
    }
}

impl Client {
    pub fn name(&self) -> &Hostname {
        &self.name
    }

    /// Dependencies between resources are stored in a flat structure,
    /// a map of hashsets. Per resource this structure documents
    /// which other resources it depends on.
    /// In order to detect a loop within the dependency structure,
    /// we have to scour the dependencies of a resource and the dependencies
    /// of each dependency recursively. If the resource ID turns up
    /// at any point, establishing a new dependency between this resource
    /// and the starting dependency would introduce a loop.
    /// If the search turns up empty, the relationship can be safely
    /// established.
    fn dependency_introduces_loop(&self, node: Uuid, target: Uuid) -> bool {
        match self.temporary.dependencies.get(&node) {
            Some(ids) => {
                ids.contains(&target)
                    || ids
                        .iter()
                        .any(|id| self.dependency_introduces_loop(*id, target))
            }
            None => false,
        }
    }

    /// Return the resource corresponding to a dependency. Most dependencies
    /// contain a `type` and a primary parameter such as `path` by which
    /// they are uniquely identifiable within the resource catalog.
    /// If a dependency is specified in the configuration that does not
    /// correspond to a known resource, `None` is returned.
    fn resolve_dependency(&self, dependency: &Dependency) -> Option<Resource> {
        match dependency {
            Dependency::AptPackage { name } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_apt_package()
                        .is_some_and(|item| item.parameters.name == *name)
                })
                .cloned(),
            Dependency::AptPreference { name } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_apt_preference()
                        .is_some_and(|item| item.parameters.name == *name)
                })
                .cloned(),
            Dependency::Directory { path } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_directory()
                        .is_some_and(|item| item.parameters.path == *path)
                })
                .cloned(),
            Dependency::File { path } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_file()
                        .is_some_and(|item| item.parameters.path == *path)
                })
                .cloned(),
            Dependency::Group { name } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_group()
                        .is_some_and(|item| item.parameters.name == *name)
                })
                .cloned(),
            Dependency::Host { ip_address } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_host()
                        .is_some_and(|item| item.parameters.ip_address == *ip_address)
                })
                .cloned(),
            Dependency::ResolvConf => self
                .resources
                .iter()
                .find(|resource| resource.as_resolv_conf().is_some())
                .cloned(),
            Dependency::Symlink { path } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_symlink()
                        .is_some_and(|item| item.parameters.path == *path)
                })
                .cloned(),
            Dependency::User { name } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_user()
                        .is_some_and(|item| item.parameters.name == *name)
                })
                .cloned(),
        }
    }

    /// Iterate and validate every resource from each group that this client
    /// is a member of. Variables are substituted in the process.
    /// Then add the resources originating from a group to the client's own
    /// pool of resources (for each type of resource respectively), except when:
    ///
    /// * the client already contains this exact resource in which case it
    ///   takes precedence and group resources are ignored.
    /// * the resource appears in multiple groups and not in the client in which
    ///   case processing fails because we do not know which group resource to
    ///   include.
    fn extend_from_groups(
        &mut self,
        groups: &mut HashMap<Hostname, (Group, usize)>,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        for group_name in &self.assigned_groups {
            let (group, count) = groups.get_mut(group_name).ok_or_else(|| {
                error!(scope, client:% = self.name, group:% = group_name; "unknown group `{}`", group_name);
                Terminate
            })?;

            *count += 1;

            for item in &group.resources {
                let requires = item.requires().to_vec();

                // Convert resource from the deserialized to the final form,
                // substituting variables in the process.
                let resource = Resource::try_from((item, &self.variables)).map_err(|error| {
                    error!(
                        scope,
                        client:% = self.name,
                        group:% = group_name,
                        resource = item.kind();
                        "{}",
                        error
                    );
                    Terminate
                })?;

                // Save dependencies as they appear in the deserialized resource.
                self.temporary.requires.insert(resource.id(), requires);

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self.resources.iter().find(|other| **other == resource) {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = self.temporary.origins.get(&duplicate.id()) {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = resource.kind();
                            //                            name:% = package.parameters.name;
                            "duplicate resource defined in group `{}`",
                            origin,
                        );

                        return Err(Terminate);
                    } else {
                        continue;
                    }
                } else {
                    // If no similar resource is present, save this one into
                    // the catalog and also record that this resource stems from
                    // a group.
                    self.temporary
                        .origins
                        .insert(resource.id(), group_name.clone());
                    self.resources.push_back(resource);
                }
            }
        }

        Ok(())
    }

    /// Validate resources from the resource catalog in relationship to
    /// each other. Some resources depend on the configuration of others.
    /// Resources also form relationships with each other to indicate
    /// the order that they need to be applied by the client. These
    /// relationships are also validated and added to the resource.
    /// This function also ensures that relationships do not introduce a
    /// dependency loop which would cause the client to loop indefinitely.
    fn validate(&mut self) -> Result<(), Terminate> {
        let scope = "validation";

        // Keep track of resources that have been processed.
        let mut validated = HashSet::new();

        // The resource that is currently processed is removed from the
        // resource catalog. This enables the validation process to
        // borrow the resource catalog immutably while the resource is
        // processed, which is needed in order to validate the resource
        // in the context of other resources.
        // When validation succeeds, the resource is added to the back
        // of the queue.
        while let Some(mut resource) = self.resources.pop_front() {
            // Break the loop once all resources have been processed.
            if !validated.insert(resource.id()) {
                self.resources.push_back(resource);
                break;
            }

            match resource {
                Resource::AptPackage(ref mut item) => self.validate_apt_package(item)?,
                Resource::AptPreference(ref mut item) => self.validate_apt_preference(item)?,
                Resource::Directory(ref mut item) => self.validate_directory(item)?,
                Resource::File(ref mut item) => self.validate_file(item)?,
                Resource::Group(ref mut item) => self.validate_group(item)?,
                Resource::Host(ref mut item) => self.validate_host(item)?,
                Resource::ResolvConf(ref mut item) => self.validate_resolv_conf(item)?,
                Resource::Symlink(ref mut item) => self.validate_symlink(item)?,
                Resource::User(ref mut item) => self.validate_user(item)?,
            }

            // Save the metadata of explicit dependencies (other resources)
            // that this resource should depend on.
            for dependency in self
                .temporary
                .requires
                .get(&resource.id())
                .map(|c| c.as_slice())
                .unwrap_or_default()
            {
                match self.resolve_dependency(dependency) {
                    Some(other_resource) => {
                        if resource.may_depend_on(&other_resource) {
                            let metadata = resource.metadata();
                            let other_metadata = other_resource.metadata().clone();

                            if self.dependency_introduces_loop(other_metadata.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = resource.kind();
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    resource.repr(),
                                    other_resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .temporary
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other_metadata.id)
                            {
                                resource.push_requirement(other_metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = resource.kind();
                                "{} cannot depend on {}",
                                resource.repr(),
                                other_resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = resource.kind();
                            "{} depends on {} which cannot be found",
                            resource.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }

            self.resources.push_back(resource);
        }

        Ok(())
    }

    fn validate_file(&mut self, file: &mut file::File) -> Result<(), Terminate> {
        let scope = "validation";

        let path = file.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(file.parameters.path.to_path_buf())
        {
            error!(
                scope,
                client:% = self.name,
                resource = file.kind(),
                path;
                "path `{}` appears multiple times, must be unique among resources of type `file`, `symlink` and `directory`",
                path
            );

            return Err(Terminate);
        }

        // Files (their paths) cannot be parents to each other.
        // Check if any file conflicts with this file in that regard.
        if let Some(parent) = &file.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = file.kind(),
                    path;
                    "another file `{}` is found to be a parent of {}, but files cannot be parents to other files",
                    file.repr(),
                    parent.display()
                );

                return Err(Terminate);
            }
        }

        // Save the metadata of ancestral directories and symlinks
        // that this file depends on.
        for ancestor in self.resources.iter().filter(|item| {
            item.as_directory().is_some_and(|d| {
                file.parameters
                    .path
                    .ancestors()
                    .any(|a| a == *d.parameters.path)
            })
        }) {
            let metadata = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(file.id())
                .or_default()
                .insert(metadata.id);

            file.relationships.requires.push(metadata);
        }

        for ancestor in self.resources.iter().filter(|item| {
            item.as_symlink().is_some_and(|s| {
                file.parameters
                    .path
                    .ancestors()
                    .any(|a| a == *s.parameters.path)
            })
        }) {
            let metadata = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(file.id())
                .or_default()
                .insert(metadata.id);

            file.relationships.requires.push(metadata);
        }

        Ok(())
    }

    fn validate_directory(
        &mut self,
        directory: &mut directory::Directory,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        let path = directory.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(directory.parameters.path.to_path_buf())
        {
            error!(
                scope,
                client:% = self.name,
                resource = directory.kind(),
                path;
                "path `{}` appears multiple times, must be unique among resources of type `file`, `symlink` and `directory`",
                path
            );

            return Err(Terminate);
        }

        // Files (their paths) cannot be parents to directories.
        // Check if any file conflicts with this directory in that regard.
        if let Some(parent) = &directory.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = directory.kind(),
                    path;
                    "file `{}` is found to be a parent of this directory, but files cannot be parents to directories",
                    parent.display()
                );

                return Err(Terminate);
            }
        }

        // Save the paths of child nodes. This becomes relevant when
        // the `purge` parameter is `true` and the directory must
        // remove unmanaged child nodes it may contain.
        for child in self
            .resources
            .iter()
            .filter_map(|item| item.as_directory())
            .filter(|d| {
                d.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            })
        {
            directory.relationships.children.push(child.into());
        }

        for child in self
            .resources
            .iter()
            .filter_map(|item| item.as_file())
            .filter(|f| {
                f.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            })
        {
            directory.relationships.children.push(child.into());
        }

        for child in self
            .resources
            .iter()
            .filter_map(|item| item.as_symlink())
            .filter(|s| {
                s.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            })
        {
            directory.relationships.children.push(child.into());
        }

        for child in self
            .resources
            .iter()
            .filter_map(|item| item.as_apt_preference())
            .filter(|p| {
                p.parameters
                    .target
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            })
        {
            directory.relationships.children.push(child.into());
        }

        // Save the metadata of user resources whose `home` directory
        // matches this directory's `path`.
        for user in self.resources.iter().filter(|item| {
            item.as_user()
                .is_some_and(|u| u.parameters.home == directory.parameters.path)
        }) {
            let metadata = directory.metadata();
            let other = user.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            directory.relationships.requires.push(other);
        }

        // Save the metadata of ancestral directories and symlinks
        // that this directory depends on.
        for ancestor in self.resources.iter().filter(|item| {
            item.as_directory().is_some_and(|d| {
                directory
                    .parameters
                    .path
                    .ancestors()
                    .skip(1)
                    .any(|a| a == *d.parameters.path)
            })
        }) {
            let metadata = directory.metadata();
            let other = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            directory.relationships.requires.push(other);
        }

        for ancestor in self.resources.iter().filter(|item| {
            item.as_symlink().is_some_and(|s| {
                directory
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *s.parameters.path)
            })
        }) {
            let metadata = directory.metadata();
            let other = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            directory.relationships.requires.push(other);
        }

        Ok(())
    }

    fn validate_symlink(&mut self, symlink: &mut symlink::Symlink) -> Result<(), Terminate> {
        let scope = "validation";

        let path = symlink.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(symlink.parameters.path.to_path_buf())
        {
            error!(
                scope,
                client:% = self.name,
                resource = symlink.kind(),
                path;
                "path `{}` appears multiple times, must be unique among resources of type `file`, `symlink` and `directory`",
                path
            );

            return Err(Terminate);
        }

        // Files (their paths) cannot be parents to symlinks.
        // Check if any file conflicts with this symlink in that regard.
        if let Some(parent) = &symlink.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = symlink.kind(),
                    path;
                    "file `{}` is found to be a parent of this symlink, but files cannot be parents to symlinks",
                    parent.display()
                );

                return Err(Terminate);
            }
        }

        // Save metadata of ancestral directories that the symlink
        // depends on.
        for ancestor in self.resources.iter().filter(|item| {
            item.as_directory().is_some_and(|d| {
                symlink
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *d.parameters.path)
            })
        }) {
            let metadata = symlink.metadata();
            let other = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            symlink.relationships.requires.push(other);
        }

        // Save metadata of ancestral symlinks that this symlink
        // depends on.
        for ancestor in self.resources.iter().filter(|item| {
            item.as_symlink().is_some_and(|s| {
                symlink
                    .parameters
                    .path
                    .ancestors()
                    .skip(1)
                    .any(|a| a == *s.parameters.path)
            })
        }) {
            let metadata = symlink.metadata();
            let other = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            symlink.relationships.requires.push(other);
        }

        // Save metadata of the target that this symlink points to.
        if let Some(other) = self
            .resources
            .iter()
            .find(|item| {
                item.as_directory()
                    .is_some_and(|d| d.parameters.path == symlink.parameters.target)
            })
            .map(|d| d.metadata().clone())
            .or(self
                .resources
                .iter()
                .find(|item| {
                    item.as_file()
                        .is_some_and(|f| f.parameters.path == symlink.parameters.target)
                })
                .map(|f| f.metadata().clone()))
        {
            let metadata = symlink.metadata();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            symlink.relationships.requires.push(other);
        }

        Ok(())
    }

    fn validate_host(&mut self, host: &mut host::Host) -> Result<(), Terminate> {
        let scope = "validation";

        let ip_address = host.parameters.ip_address.to_string();

        // Check for uniqueness of the IP address parameter.
        if !self
            .temporary
            .host_ip_addresses
            .insert(host.parameters.ip_address)
        {
            error!(
                scope,
                client:% = self.name,
                resource = host.kind(),
                ip_address;
                "IP address `{}` appears multiple times, must be unique among host entries",
                ip_address
            );

            return Err(Terminate);
        }

        // Save the metadata of the target file or symlink for the host
        // entry.
        // Also check if the target is a file resource that sets its
        // content  or source parameter. This combination is not supported.
        if let Some(file) = self
            .resources
            .iter()
            .filter_map(|item| item.as_file())
            .find(|f| *f.parameters.path == host.parameters.target)
        {
            if file.parameters.content.is_some() || file.parameters.source.is_some() {
                error!(
                    scope,
                    client:% = self.name,
                    resource = host.kind(),
                    ip_address;
                    "there cannot be both a {} resource and a {} whose `content` or `source` parameters are set",
                    host.repr(),
                    file.repr()
                );

                return Err(Terminate);
            } else {
                let metadata = host.metadata();
                let other = file.metadata().clone();

                self.temporary
                    .dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                host.relationships.requires.push(other);
            }
        }

        if let Some(symlink) = self.resources.iter().find(|item| {
            item.as_symlink()
                .is_some_and(|s| *s.parameters.path == host.parameters.target)
        }) {
            let metadata = host.metadata();
            let other = symlink.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            host.relationships.requires.push(other);
        }

        Ok(())
    }

    fn validate_group(&mut self, group: &mut group::Group) -> Result<(), Terminate> {
        let scope = "validation";

        let name = group.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .group_names
            .insert(group.parameters.name.clone())
        {
            error!(
                scope,
                client:% = self.name,
                resource = group.kind(),
                name;
                "group name `{}` appears multiple times, group names must be unique",
                name
            );

            return Err(Terminate);
        }

        // Add users as dependency to a group if the group is a
        // user's primary group.
        // Primary groups must be handled after users as user creation
        // usually involves creating the primary group as well.
        for user in self.resources.iter().filter_map(|item| item.as_user()) {
            if user.parameters.group == group.parameters.name {
                let metadata = group.metadata();
                let other = user.metadata().clone();

                self.temporary
                    .dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                group.relationships.requires.push(other);
            }
        }

        Ok(())
    }

    fn validate_user(&mut self, user: &mut user::User) -> Result<(), Terminate> {
        let scope = "validation";

        let name = user.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .user_names
            .insert(user.parameters.name.clone())
        {
            error!(
                scope,
                client:% = self.name,
                resource = user.kind(),
                name;
                "user name `{}` appears multiple times, user names must be unique",
                name
            );

            return Err(Terminate);
        }

        // Add group resources as dependencies if their name appears
        // in the list of user group names.
        // Supplementary groups must be processed before users.
        for name in &user.parameters.groups {
            if let Some(group) = self.resources.iter().find(|item| {
                item.as_group()
                    .is_some_and(|group| group.parameters.name == *name)
            }) {
                let metadata = user.metadata();
                let other = group.metadata().clone();

                self.temporary
                    .dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                user.relationships.requires.push(other);
            }
        }

        Ok(())
    }

    fn validate_resolv_conf(
        &mut self,
        resolv_conf: &mut resolv_conf::ResolvConf,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        // Ensure that there's only one `resolv.conf` resource.
        if self
            .resources
            .iter()
            .any(|item| item.as_resolv_conf().is_some())
        {
            error!(
                scope,
                client:% = self.name,
                resource = resolv_conf.kind();
                "there cannot be more than one {}",
                resolv_conf.repr()
            );

            return Err(Terminate);
        }

        // Save the metadata of the target file or symlink corresponding to the
        // /etc/resolv.conf file.
        // Also check if the target is a file resource that sets its
        // content or source parameter. This combination is not supported.
        if let Some(file) = self
            .resources
            .iter()
            .filter_map(|item| item.as_file())
            .find(|f| *f.parameters.path == resolv_conf.parameters.target)
        {
            if file.parameters.content.is_some() || file.parameters.source.is_some() {
                error!(
                    scope,
                    client:% = self.name,
                    resource = resolv_conf.kind();
                    "there cannot be both a {} resource and a {} whose `content` or `source` parameters are set",
                    resolv_conf.repr(),
                    file.repr()
                );

                return Err(Terminate);
            } else {
                let metadata = resolv_conf.metadata();
                let other = file.metadata().clone();

                self.temporary
                    .dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                resolv_conf.relationships.requires.push(other);
            }
        }

        if let Some(symlink) = self.resources.iter().find(|item| {
            item.as_symlink()
                .is_some_and(|s| *s.parameters.path == resolv_conf.parameters.target)
        }) {
            let metadata = resolv_conf.metadata();
            let other = symlink.metadata().clone();

            self.temporary
                .dependencies
                .entry(metadata.id)
                .or_default()
                .insert(other.id);

            resolv_conf.relationships.requires.push(other);
        }

        Ok(())
    }

    fn validate_apt_package(
        &mut self,
        package: &mut apt::package::Package,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        let name = package.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .apt_package_names
            .insert(package.parameters.name.clone())
        {
            error!(
                scope,
                client:% = self.name,
                resource = package.kind(),
                name;
                "package name `{}` appears multiple times, package names must be unique",
                name
            );

            return Err(Terminate);
        }

        // Save the metadata of explicit dependencies that this
        // `apt::package` should depend on.
        for dependency in self
            .temporary
            .requires
            .get(&package.id())
            .map(|c| c.as_slice())
            .unwrap_or_default()
        {
            match self.resolve_dependency(dependency) {
                Some(resource) => {
                    if package.may_depend_on(&resource) {
                        let metadata = package.metadata();
                        let other = resource.metadata().clone();

                        if self.dependency_introduces_loop(other.id, metadata.id) {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = package.kind(),
                                name;
                                "{} cannot depend on {} as it would introduce a dependency loop",
                                package.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        } else if self
                            .temporary
                            .dependencies
                            .entry(metadata.id)
                            .or_default()
                            .insert(other.id)
                        {
                            package.relationships.requires.push(metadata.clone());
                        }
                    } else {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = package.kind(),
                            name;
                            "{} cannot depend on {}",
                            package.repr(),
                            resource.repr()
                        );

                        return Err(Terminate);
                    }
                }
                None => {
                    error!(
                        scope,
                        client:% = self.name,
                        resource = package.kind(),
                        name;
                        "{} depends on {} which cannot be found",
                        package.repr(),
                        dependency.repr()
                    );

                    return Err(Terminate);
                }
            }
        }

        Ok(())
    }

    fn validate_apt_preference(
        &mut self,
        preference: &mut apt::preference::Preference,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        let name = preference.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .apt_preference_names
            .insert(preference.parameters.name.clone())
        {
            error!(
                scope,
                client:% = self.name,
                resource = preference.kind(),
                name;
                "preference name `{}` appears multiple times, preference names must be unique",
                name
            );

            return Err(Terminate);
        }

        if !self
            .temporary
            .paths
            .insert(preference.parameters.target.clone())
        {
            error!(
                scope,
                client:% = self.name,
                resource = preference.kind(),
                name;
                "{} conflicts with another resource that manages the target path `{}`",
                preference.repr(),
                preference.parameters.target.display()
            );

            return Err(Terminate);
        }

        // Save metadata of ancestral directories that the target file
        // depends on.
        for ancestor in self.resources.iter().filter(|item| {
            item.as_directory().is_some_and(|d| {
                preference
                    .parameters
                    .target
                    .ancestors()
                    .any(|a| a == *d.parameters.path)
            })
        }) {
            let other = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(preference.id())
                .or_default()
                .insert(other.id);

            preference.relationships.requires.push(other);
        }

        for ancestor in self.resources.iter().filter(|item| {
            item.as_symlink().is_some_and(|s| {
                preference
                    .parameters
                    .target
                    .ancestors()
                    .any(|a| a == *s.parameters.path)
            })
        }) {
            let metadata = ancestor.metadata().clone();

            self.temporary
                .dependencies
                .entry(preference.id())
                .or_default()
                .insert(metadata.id);

            preference.relationships.requires.push(metadata);
        }

        Ok(())
    }
}

pub mod deserialize {
    use super::*;
    use serde::Deserialize;

    #[derive(Clone, Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Client {
        #[serde(rename(deserialize = "api-key"))]
        pub api_key: ApiKey,
        #[serde(default, rename(deserialize = "groups"))]
        pub assigned_groups: Vec<Hostname>,
        #[serde(default)]
        pub variables: HashMap<String, toml::Value>,
        #[serde(default)]
        pub resources: Vec<DeResource>,
    }
}
