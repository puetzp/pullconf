use crate::{
    configuration::Source,
    types::{
        resources::{
            apt, directory, execute, file, group, host, symlink, user, Dependency, Resource,
            Trigger, UnresolvedResource,
        },
        ApiKey, Group,
    },
};
use common::{
    resources::{
        apt::package::Name as AptPackageName, group::Name as GroupName, user::Name as UserName,
    },
    Action, Hostname, ResourceType, TriggerMetadata,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
};
use strict_yaml_rust::{strict_yaml::Hash as StrictYamlHash, StrictYaml};
use uuid::Uuid;

/// This struct contains temporary helper collections that are
/// freed after configuration validation has concluded.
#[derive(Clone, Debug, Default)]
pub struct ValidationHelpers {
    /// This list contains every resource ID and the IDs of resources
    /// that each resource depends on. The list is used during validation
    /// to detect loops that would prevent proper resource exection
    /// on the client side.
    pub predecessors: HashMap<Uuid, HashSet<Uuid>>,
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
    /// This collection stores explicit triggers per resource.
    /// During validation these triggers are resolved and
    /// the actual resource metadata of a given triggered resource
    /// is added to the resource relationship data.
    pub triggers: HashMap<Uuid, Vec<Trigger>>,
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
    pub execute_names: HashSet<String>,
    pub file_paths: HashSet<PathBuf>,
    pub host_ip_addresses: HashSet<IpAddr>,
    pub group_names: HashSet<GroupName>,
    pub user_names: HashSet<UserName>,
    pub apt_package_names: HashSet<AptPackageName>,
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
    pub variables: HashMap<String, StrictYaml>,
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

impl TryFrom<(unresolved::Client, &mut HashMap<Hostname, (Group, usize)>)> for Client {
    type Error = String;

    fn try_from(
        (intermediate, groups): (unresolved::Client, &mut HashMap<Hostname, (Group, usize)>),
    ) -> Result<Self, Self::Error> {
        // Initialize the client and validate the client's own configuration,
        // substituting variables in the process.
        // This does not take resources from groups into account.
        let mut client = Self {
            name: intermediate.name,
            api_key: intermediate.api_key,
            assigned_groups: intermediate.assigned_groups,
            variables: intermediate.variables,
            temporary: ValidationHelpers::default(),
            resources: VecDeque::new(),
        };

        client.variables.insert(
            "hostname".to_string(),
            StrictYaml::String(client.name.to_string()),
        );

        for item in intermediate.resources {
            let requires = item.requires().to_vec();
            let triggers = item.triggers().to_vec();

            // Convert resource from the deserialized to the final form,
            // substituting variables in the process.
            let resource = Resource::try_from((item, &client.variables))
                .map_err(|error| format!("`{}`>{}", client.name, error))?;

            // Save triggers and dependencies as they appear in the deserialized resource.
            client.temporary.requires.insert(resource.id(), requires);
            client.temporary.triggers.insert(resource.id(), triggers);

            client.resources.push_back(resource);
        }

        // Extend the client's resource catalog with resources from groups
        // that the client is a member of, substituting variables in the process.
        client
            .extend_from_groups(groups)
            .map_err(|error| format!("`client[{}]`>{}", client.name, error))?;

        client.temporary.file_paths = client
            .resources
            .iter()
            .filter_map(|resource| resource.as_file())
            .map(|file| file.parameters.path.to_path_buf())
            .collect();

        client
            .validate()
            .map_err(|error| format!("`client[{}]`>{}", client.name, error))?;

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
    fn relationship_introduces_loop(&self, node: Uuid, target: Uuid) -> bool {
        match self.temporary.predecessors.get(&node) {
            Some(ids) => {
                ids.contains(&target)
                    || ids
                        .iter()
                        .any(|id| self.relationship_introduces_loop(*id, target))
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
            Dependency::Directory { path } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_directory()
                        .is_some_and(|item| item.parameters.path == *path)
                })
                .cloned(),
            Dependency::Execute { name } => self
                .resources
                .iter()
                .find(|resource| {
                    resource
                        .as_execute()
                        .is_some_and(|item| item.parameters.name == *name)
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

    /// Return the resource corresponding to a trigger reference.
    /// Triggers are always references to resources of type `execute` which
    /// are uniquely identifiable by their `name` attribute.
    /// If the configuration references a trigger that does not correspond
    /// to a known `execute` resource, `None` is returned.
    fn resolve_trigger<'a>(
        &self,
        trigger: &'a Trigger,
    ) -> Option<(execute::Execute, &'a [Action])> {
        match trigger {
            Trigger::Execute { name, when } => match self
                .resources
                .iter()
                .filter_map(|resource| resource.as_execute())
                .find(|execute| execute.parameters.name == *name)
            {
                Some(execute) => Some((execute.clone(), when.as_slice())),
                None => None,
            },
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
    ) -> Result<(), String> {
        for group_name in &self.assigned_groups {
            let (group, count) = groups
                .get_mut(group_name)
                .ok_or(format!("reference to unknown group `{}`", group_name))?;

            *count += 1;

            for item in &group.resources {
                let requires = item.requires().to_vec();
                let triggers = item.triggers().to_vec();

                // Convert resource from the deserialized to the final form,
                // substituting variables in the process.
                let resource = Resource::try_from((item.clone(), &self.variables))?;

                // Save triggers and dependencies as they appear in the deserialized resource.
                self.temporary.requires.insert(resource.id(), requires);
                self.temporary.triggers.insert(resource.id(), triggers);

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self.resources.iter().find(|other| **other == resource) {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = self.temporary.origins.get(&duplicate.id()) {
                        return Err(format!(
                            "duplicate resource `{}` defined in group `{}`",
                            duplicate.repr(),
                            origin,
                        ));
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
    fn validate(&mut self) -> Result<(), String> {
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
                Resource::Directory(ref mut item) => self.validate_directory(item)?,
                Resource::Execute(ref mut item) => self.validate_execute(item)?,
                Resource::File(ref mut item) => self.validate_file(item)?,
                Resource::Group(ref mut item) => self.validate_group(item)?,
                Resource::Host(ref mut item) => self.validate_host(item)?,
                Resource::Symlink(ref mut item) => self.validate_symlink(item)?,
                Resource::User(ref mut item) => self.validate_user(item)?,
            }

            // Process implicit dependencies by saving the metadata of
            // other resources that this resource depends on.
            // This also presupposes the order of execution on the client
            // side because the dependency is also explicitly declared
            // to preceed this resource.
            for other in &self.resources {
                let metadata = resource.metadata().clone();
                let other_metadata = other.metadata().clone();

                if resource.must_depend_on(other) {
                    if self.relationship_introduces_loop(other_metadata.id, metadata.id) {
                        return Err(format!(
                            "`{}`: resource must depend on `{}`, but the current configuration would introduce a loop",
                            resource.repr(),
                            other.repr()
                        ));
                    } else if self
                        .temporary
                        .predecessors
                        .entry(metadata.id)
                        .or_default()
                        .insert(other_metadata.id)
                    {
                        resource.push_requirement(other_metadata.clone());
                        resource.push_predecessor(other_metadata.clone());
                    }
                }

                // The `execute` resource is special in that it must
                // be applied after all the other resources that trigger
                // it, as based on the `triggers` meta-parameter.
                // `Resource::must_depend_on` cannot solve this use
                // case since no dependency must be formed, only a hint
                // to the proper order of execution.
                // This relationship type accounts for multiple resources
                // triggering an `execute` resource in which case the
                // resource must be applied if at least one of the
                // triggering resources is applied successfully, regardless
                // of the state of the other resources.
                // Of course if none of the triggering resources is applied
                // successfully, the `execute` resource will not run either.
                //
                // In addition explicit dependencies apply to `execute` just
                // like any other resource. So if there also exists an
                // explicit dependency to any of the triggering resources,
                // the resource must be applied successfully or `execute`
                // will short-circuit and fail.
                if resource.kind() == ResourceType::Execute {
                    if self
                        .temporary
                        .triggers
                        .get(&other_metadata.id)
                        .is_some_and(|list| list.iter().any(|item| *item == resource))
                    {
                        if self.relationship_introduces_loop(other_metadata.id, metadata.id) {
                            return Err(format!(
                                "`{}`: resource must be applied after `{}` as per the `triggers` parameter, but the current configuration would introduce a loop",
                                resource.repr(),
                                other.repr()
                            ));
                        } else if self
                            .temporary
                            .predecessors
                            .entry(metadata.id)
                            .or_default()
                            .insert(other_metadata.id)
                        {
                            resource.push_predecessor(other_metadata.clone());
                        }
                    }
                }
            }

            // Process explicit dependencies by saving the metadata of
            // other resources that this resource must depend on
            // according to the `requires` meta-parameter found in
            // the configuration.
            // This also presupposes the order of execution on the client
            // side because the dependency is also explicitly declared
            // to preceed this resource.
            for dependency in self
                .temporary
                .requires
                .get(&resource.id())
                .map(|c| c.as_slice())
                .unwrap_or_default()
            {
                if *dependency == resource {
                    return Err(format!(
                        "`{}`: resource cannot depend in itself",
                        resource.repr(),
                    ));
                }

                match self.resolve_dependency(dependency) {
                    Some(other_resource) => {
                        if resource.may_depend_on(&other_resource) {
                            let metadata = resource.metadata();
                            let other_metadata = other_resource.metadata().clone();

                            if self.relationship_introduces_loop(other_metadata.id, metadata.id) {
                                return Err(format!(
                                    "`{}`: resource cannot depend on `{}` as the current configuration would introduce a loop",
                                    resource.repr(),
                                    other_resource.repr()
                                ));
                            } else if self
                                .temporary
                                .predecessors
                                .entry(metadata.id)
                                .or_default()
                                .insert(other_metadata.id)
                            {
                                resource.push_requirement(other_metadata.clone());
                                resource.push_predecessor(other_metadata.clone());
                            }
                        } else {
                            return Err(format!(
                                "`{}`: resource cannot depend on `{}`",
                                resource.repr(),
                                other_resource.repr()
                            ));
                        }
                    }
                    None => {
                        return Err(format!(
                            "`{}`: resource depends on `{}` which is undefined",
                            resource.repr(),
                            dependency.repr()
                        ));
                    }
                }
            }

            // Process explicit triggers by saving the metadata of
            // triggered resources per the `triggers` meta-parameter
            // found in the configuration.
            for trigger in self
                .temporary
                .triggers
                .get(&resource.id())
                .map(|c| c.as_slice())
                .unwrap_or_default()
            {
                if *trigger == resource {
                    return Err(format!(
                        "`{}`: resource cannot trigger itself",
                        resource.repr(),
                    ));
                }

                match self.resolve_trigger(trigger) {
                    Some((execute, when)) => {
                        let metadata = TriggerMetadata::from(execute.metadata(), when);
                        resource.push_trigger(metadata);
                    }
                    None => {
                        return Err(format!(
                            "`{}`: resource triggers `{}` which is undefined",
                            resource.repr(),
                            trigger.repr()
                        ));
                    }
                }
            }

            self.resources.push_back(resource);
        }

        Ok(())
    }

    fn validate_file(&mut self, file: &mut file::File) -> Result<(), String> {
        let path = file.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(file.parameters.path.to_path_buf())
        {
            return Err(format!(
                "`{}`: path `{}` appears in multiple resources, must be unique among resources of type `file`, `symlink` and `directory`",
                file.repr(),
                path
            ));
        }

        // Files (their paths) cannot be parents to each other.
        // Check if any file conflicts with this file in that regard.
        if let Some(parent) = &file.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                return Err(format!(
                    "`{}`: another file `{}` is found to be a parent of this file, but files cannot be parents to other files",
                    file.repr(),
                    parent.display()
                ));
            }
        }

        Ok(())
    }

    fn validate_directory(&mut self, directory: &mut directory::Directory) -> Result<(), String> {
        let path = directory.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(directory.parameters.path.to_path_buf())
        {
            return Err(format!(
                "`{}`: path `{}` appears in multiple resources, must be unique among resources of type `file`, `symlink` and `directory`",
                directory.repr(),
                path
            ));
        }

        // Files (their paths) cannot be parents to directories.
        // Check if any file conflicts with this directory in that regard.
        if let Some(parent) = &directory.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                return Err(format!(
                    "`{}`: file `{}` is found to be a parent of this directory, but files cannot be parents to directories",
                    directory.repr(),
                    parent.display()
                ));
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

        Ok(())
    }

    fn validate_symlink(&mut self, symlink: &mut symlink::Symlink) -> Result<(), String> {
        let path = symlink.parameters.path.display().to_string();

        // Check for uniqueness of the path parameter.
        if !self
            .temporary
            .paths
            .insert(symlink.parameters.path.to_path_buf())
        {
            return Err(format!(
                "`{}`: path `{}` appears in multiple resources, must be unique among resources of type `file`, `symlink` and `directory`",
                symlink.repr(),
                path
            ));
        }

        // Files (their paths) cannot be parents to symlinks.
        // Check if any file conflicts with this symlink in that regard.
        if let Some(parent) = &symlink.parameters.path.parent() {
            if self.temporary.file_paths.contains(*parent) {
                return Err(format!(
                    "`{}`: file `{}` is found to be a parent of this symlink, but files cannot be parents to symlinks",
                    symlink.repr(),
                    parent.display()
                ));
            }
        }

        Ok(())
    }

    fn validate_host(&mut self, host: &mut host::Host) -> Result<(), String> {
        let ip_address = host.parameters.ip_address.to_string();

        // Check for uniqueness of the IP address parameter.
        if !self
            .temporary
            .host_ip_addresses
            .insert(host.parameters.ip_address)
        {
            return Err(format!(
                "`{}`: IP address `{}` appears in multiple `{}` resources, must be unique among host entries",
                host.repr(),
                ip_address,
                host.kind(),
            ));
        }

        // Check if there is also a file managing `/etc/hosts` whose `content`
        // or `source` parameter are set. This combination is not supported if a
        // `host` resource exists.
        if let Some(file) = self
            .resources
            .iter()
            .filter_map(|item| item.as_file())
            .find(|f| *f.parameters.path == host.parameters.target)
        {
            if file.parameters.content.is_some() || file.parameters.source.is_some() {
                return Err(format!(
                    "`{}`: resource conflicts with `{}` whose `content` or `source` parameters are set",
                    host.repr(),
                    file.repr()
                ));
            }
        }

        Ok(())
    }

    fn validate_group(&mut self, group: &mut group::Group) -> Result<(), String> {
        let name = group.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .group_names
            .insert(group.parameters.name.clone())
        {
            return Err(format!(
                "`{}`: group name `{}` appears in multiple `{}` resources, group names must be unique",
                group.repr(),
                name,
                group.kind(),
            ));
        }

        Ok(())
    }

    fn validate_user(&mut self, user: &mut user::User) -> Result<(), String> {
        let name = user.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .user_names
            .insert(user.parameters.name.clone())
        {
            return Err(format!(
                "`{}`: user name `{}` appears in multiple `{}` resources, user names must be unique",
                user.repr(),
                name,
                user.kind(),
            ));
        }

        Ok(())
    }

    fn validate_apt_package(&mut self, package: &mut apt::package::Package) -> Result<(), String> {
        let name = package.parameters.name.to_string();

        // Check for uniqueness of the name parameter.
        if !self
            .temporary
            .apt_package_names
            .insert(package.parameters.name.clone())
        {
            return Err(format!(
                "`{}`: package name `{}` appears in multiple `{}` resources, package names must be unique",
                package.repr(),
                name,
                package.kind(),
            ));
        }

        Ok(())
    }

    fn validate_execute(&mut self, execute: &mut execute::Execute) -> Result<(), String> {
        let name = &execute.parameters.name;

        // Check for uniqueness of the name parameter.
        if !self.temporary.execute_names.insert(name.clone()) {
            return Err(format!(
                "`{}`: name `{}` appears in multiple `{}` resources, resource names must be unique",
                execute.repr(),
                name,
                execute.kind(),
            ));
        }

        Ok(())
    }
}

pub mod unresolved {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Client {
        pub name: Hostname,
        pub api_key: ApiKey,
        pub assigned_groups: Vec<Hostname>,
        pub variables: HashMap<String, StrictYaml>,
        pub resources: Vec<UnresolvedResource>,
    }

    impl TryFrom<(Source, StrictYamlHash)> for Client {
        type Error = String;

        fn try_from((source, mut hash): (Source, StrictYamlHash)) -> Result<Self, Self::Error> {
            let mut assigned_groups = vec![];
            let mut variables = HashMap::new();
            let mut resources = vec![];

            let name = {
                let key = "name";

                let s = hash
                    .remove(&StrictYaml::String(key.into()))
                    .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                    .into_string()
                    .ok_or(format!("{}: node must be a string", source.clone() + key))?;

                Hostname::from_str(&s)
                    .map_err(|error| format!("{}: {}", source.clone() + key, error))?
            };

            let api_key = {
                let key = "api_key";

                let s = hash
                    .remove(&StrictYaml::String(key.into()))
                    .ok_or(format!("{}: failed to find required key `{}`", source, key))?
                    .into_string()
                    .ok_or(format!("{}: node must be a string", source.clone() + key))?;

                ApiKey::from_str(&s)
                    .map_err(|error| format!("{}: {}", source.clone() + key, error))?
            };

            {
                let key = "resources";
                let source = source.clone() + key;

                if let Some(node) = hash.remove(&StrictYaml::String(key.into())) {
                    for (index, item) in node
                        .into_vec()
                        .ok_or(format!("{}: node must be an array", source))?
                        .into_iter()
                        .enumerate()
                    {
                        let source = source.clone() + index;

                        let hash = item
                            .into_hash()
                            .ok_or(format!("{}: node must be a hash", source))?;

                        let resource = UnresolvedResource::try_from((source, hash))?;

                        resources.push(resource);
                    }
                }
            }

            {
                let key = "groups";
                let source = source.clone() + key;

                if let Some(node) = hash.remove(&StrictYaml::String(key.into())) {
                    for (index, item) in node
                        .into_vec()
                        .ok_or(format!("{}: node must be an array", source))?
                        .into_iter()
                        .enumerate()
                    {
                        let source = source.clone() + index;

                        let s = item
                            .as_str()
                            .ok_or(format!("{}: node must be a string", source))?;

                        let group = Hostname::from_str(s)
                            .map_err(|error| format!("{}: {}", source, error))?;

                        assigned_groups.push(group);
                    }
                }
            }

            {
                let key = "variables";
                let source = source.clone() + key;

                if let Some(node) = hash.remove(&StrictYaml::String(key.into())) {
                    for (k, v) in node
                        .into_hash()
                        .ok_or(format!("{}: node must be a hash", source))?
                        .into_iter()
                    {
                        let k = k
                            .as_str()
                            .ok_or(format!("{}: hash keys must be strings", source))?;

                        variables.insert(k.to_string(), v);
                    }
                }
            }

            if let Some(key) = hash.pop_back().and_then(|(key, _)| key.into_string()) {
                return Err(format!("{}: encountered unexpected key `{}`", source, key));
            }

            assigned_groups.sort();
            assigned_groups.dedup();

            Ok(Self {
                name,
                api_key,
                assigned_groups,
                variables,
                resources,
            })
        }
    }
}
