use crate::types::{
    resources::{
        deserialize::{Dependency, Resource as DeResource},
        directory, file, group, host, resolv_conf, symlink, user, Resource,
    },
    ApiKey, Group,
};
use common::{error::Terminate, Hostname, SafePathBuf};
use log::error;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    path::PathBuf,
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(into = "serialize::Client")]
pub struct Client {
    pub name: Hostname,
    pub api_key: ApiKey,
    pub assigned_groups: Vec<Hostname>,
    pub variables: HashMap<String, toml::Value>,
    pub directories: Vec<directory::Directory>,
    pub files: Vec<file::File>,
    pub groups: Vec<group::Group>,
    pub hosts: Vec<host::Host>,
    pub resolv_conf: Option<resolv_conf::ResolvConf>,
    pub symlinks: Vec<symlink::Symlink>,
    pub users: Vec<user::User>,
    pub dependencies: HashMap<Uuid, HashSet<Uuid>>,
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
        // Initialize the client and validate the client's own configuration,
        // substituting variables in the process.
        // This does not take resources from groups into account.
        let mut client = Client::try_from((name, intermediate))?;

        // Extend the client's resource catalog with resources from groups
        // that the client is a member of, substituting variables in the process.
        client.extend_from_groups(groups)?;

        // Validate the relationships between the resources from the client's
        // resource catalog.
        // Add dependencies between resources where applicable.
        let mut paths = HashSet::new();

        let file_paths = client
            .files
            .iter()
            .map(|f| f.parameters.path.to_path_buf())
            .collect::<HashSet<PathBuf>>();

        client.validate_files(&mut paths, &file_paths)?;
        client.validate_directories(&mut paths, &file_paths)?;
        client.validate_symlinks(&mut paths, &file_paths)?;
        client.validate_hosts()?;
        client.validate_groups()?;
        client.validate_users()?;
        client.validate_resolv_conf()?;

        Ok(client)
    }
}

impl TryFrom<(Hostname, deserialize::Client)> for Client {
    type Error = Terminate;

    /// Create the client instance.
    /// Validate and insert each resource from the intermediate client
    /// representation. Variables are substituted in the process.
    fn try_from(
        (name, intermediate): (Hostname, deserialize::Client),
    ) -> Result<Self, Self::Error> {
        let scope = "validation";

        let mut client = Self {
            name,
            api_key: intermediate.api_key,
            assigned_groups: intermediate.assigned_groups,
            variables: intermediate.variables,
            directories: vec![],
            files: vec![],
            groups: vec![],
            hosts: vec![],
            resolv_conf: None,
            symlinks: vec![],
            users: vec![],
            dependencies: HashMap::new(),
        };

        for resource in intermediate.resources {
            match resource {
                DeResource::Directory(item) => {
                    let parameters = directory::Directory::try_from((&item, &client.variables))
                        .map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "directory";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.directories.push(parameters);
                }
                DeResource::File(item) => {
                    let parameters =
                        file::File::try_from((&item, &client.variables)).map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "file";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.files.push(parameters)
                }
                DeResource::Group(item) => {
                    let parameters =
                        group::Group::try_from((&item, &client.variables)).map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "group";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.groups.push(parameters)
                }
                DeResource::Host(item) => {
                    let parameters =
                        host::Host::try_from((&item, &client.variables)).map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "host";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.hosts.push(parameters)
                }
                DeResource::ResolvConf(item) => {
                    let parameters = resolv_conf::ResolvConf::try_from((&item, &client.variables))
                        .map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "resolv.conf";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    if client.resolv_conf.replace(parameters).is_some() {
                        error!(
                            scope,
                            client:% = client.name,
                            resource = "resolv.conf";
                            "resource appears multiple times, cannot be more than one of this kind",
                        );

                        return Err(Terminate);
                    }
                }
                DeResource::Symlink(item) => {
                    let parameters = symlink::Symlink::try_from((&item, &client.variables))
                        .map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "symlink";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.symlinks.push(parameters)
                }
                DeResource::User(item) => {
                    let parameters =
                        user::User::try_from((&item, &client.variables)).map_err(|error| {
                            error!(
                                scope,
                                client:% = client.name,
                                resource = "user";
                                "{}",
                                error
                            );
                            Terminate
                        })?;

                    client.users.push(parameters)
                }
            }
        }

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
        match self.dependencies.get(&node) {
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
            Dependency::Directory { path } => self
                .directories
                .iter()
                .find(|d| d.parameters.path == *path)
                .map(Resource::Directory),
            Dependency::File { path } => self
                .files
                .iter()
                .find(|f| f.parameters.path == *path)
                .map(Resource::File),
            Dependency::Group { name } => self
                .groups
                .iter()
                .find(|g| g.parameters.name == *name)
                .map(Resource::Group),
            Dependency::Host { ip_address } => self
                .hosts
                .iter()
                .find(|h| h.parameters.ip_address == *ip_address)
                .map(Resource::Host),
            Dependency::ResolvConf => self.resolv_conf.as_ref().map(Resource::ResolvConf),
            Dependency::Symlink { path } => self
                .symlinks
                .iter()
                .find(|s| s.parameters.path == *path)
                .map(Resource::Symlink),
            Dependency::User { name } => self
                .users
                .iter()
                .find(|u| u.parameters.name == *name)
                .map(Resource::User),
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

            // Process directory resources according to the rules described above.
            for item in &group.directories {
                // Replace variables in parameters.
                let mut directory = directory::Directory::try_from((item, &self.variables))
                    .map_err(|error| {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "directory";
                            "{}",
                            error
                        );
                        Terminate
                    })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .directories
                    .iter()
                    .find(|d| d.parameters.path == directory.parameters.path)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "directory",
                            path:% = directory.parameters.path.display();
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
                    directory.from_group = Some(group_name.clone());
                    self.directories.push(directory);
                }
            }

            // Process file resources according to the rules described above.
            for item in &group.files {
                // Replace variables in parameters.
                let mut file = file::File::try_from((item, &self.variables)).map_err(|error| {
                    error!(
                        scope,
                        client:% = self.name,
                        group:% = group_name,
                        resource = "file";
                        "{}",
                        error
                    );
                    Terminate
                })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .files
                    .iter()
                    .find(|f| f.parameters.path == file.parameters.path)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "file",
                            path:% = file.parameters.path.display();
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
                    file.from_group = Some(group_name.clone());
                    self.files.push(file);
                }
            }

            // Process group resources according to the rules described above.
            for item in &group.groups {
                // Replace variables in parameters.
                let mut _group =
                    group::Group::try_from((item, &self.variables)).map_err(|error| {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "group";
                            "{}",
                            error
                        );
                        Terminate
                    })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .groups
                    .iter()
                    .find(|g| g.parameters.name == _group.parameters.name)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "group",
                            name:% = _group.parameters.name;
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
                    _group.from_group = Some(group_name.clone());
                    self.groups.push(_group);
                }
            }

            // Process host resources according to the rules described above.
            for item in &group.hosts {
                // Replace variables in parameters.
                let mut host = host::Host::try_from((item, &self.variables)).map_err(|error| {
                    error!(
                        scope,
                        client:% = self.name,
                        group:% = group_name,
                        resource = "host";
                        "{}",
                        error
                    );
                    Terminate
                })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .hosts
                    .iter()
                    .find(|h| h.parameters.ip_address == host.parameters.ip_address)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "host",
                            ip_address:% = host.parameters.ip_address;
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
                    host.from_group = Some(group_name.clone());
                    self.hosts.push(host);
                }
            }

            // Process resolv.conf resources according to the rules described above.
            for item in &group.resolv_conf {
                // Replace variables in parameters.
                let mut resolv_conf = resolv_conf::ResolvConf::try_from((item, &self.variables))
                    .map_err(|error| {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "resolv.conf";
                            "{}",
                            error
                        );
                        Terminate
                    })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = &self.resolv_conf {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "resolv.conf";
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
                    resolv_conf.from_group = Some(group_name.clone());
                    self.resolv_conf = Some(resolv_conf);
                }
            }

            // Process symlink resources according to the rules described above.
            for item in &group.symlinks {
                // Replace variables in parameters.
                let mut symlink =
                    symlink::Symlink::try_from((item, &self.variables)).map_err(|error| {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "symlink";
                            "{}",
                            error
                        );
                        Terminate
                    })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .symlinks
                    .iter()
                    .find(|s| s.parameters.path == symlink.parameters.path)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "symlink",
                            path:% = symlink.parameters.path.display();
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
                    symlink.from_group = Some(group_name.clone());
                    self.symlinks.push(symlink);
                }
            }

            // Process user resources according to the rules described above.
            for item in &group.users {
                // Replace variables in parameters.
                let mut user = user::User::try_from((item, &self.variables)).map_err(|error| {
                    error!(
                        scope,
                        client:% = self.name,
                        group:% = group_name,
                        resource = "user";
                        "{}",
                        error
                    );
                    Terminate
                })?;

                // Check if a similar resource is already present ...
                if let Some(duplicate) = self
                    .users
                    .iter()
                    .find(|u| u.parameters.name == user.parameters.name)
                {
                    // ... and if it was sourced from another group in which case
                    // processing fails. Otherwise the group resource is skipped
                    // because the saved resource originates from the client
                    // and takes precedence.
                    if let Some(origin) = &duplicate.from_group {
                        error!(
                            scope,
                            client:% = self.name,
                            group:% = group_name,
                            resource = "user",
                            name:% = user.parameters.name;
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
                    user.from_group = Some(group_name.clone());
                    self.users.push(user);
                }
            }
        }

        Ok(())
    }

    fn validate_files(
        &mut self,
        paths: &mut HashSet<SafePathBuf>,
        file_paths: &HashSet<PathBuf>,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        // To iterate and modify file resources the collection must
        // be cloned.
        // Each resource is modified on the basis of other (file)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _files = self.files.clone();
        _files.sort_by(|a, b| a.parameters.path.cmp(&b.parameters.path));

        for file in _files.iter_mut() {
            let path = file.parameters.path.display().to_string();

            // Check for uniqueness of the path parameter.
            if !paths.insert(file.parameters.path.clone()) {
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
                if file_paths.contains(*parent) {
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
            for ancestor in self.directories.iter().filter(|d| {
                file.parameters
                    .path
                    .ancestors()
                    .any(|a| a == *d.parameters.path)
            }) {
                let metadata = ancestor.metadata().clone();

                self.dependencies
                    .entry(file.id())
                    .or_default()
                    .insert(metadata.id);

                file.relationships.requires.push(metadata);
            }

            for ancestor in self.symlinks.iter().filter(|s| {
                file.parameters
                    .path
                    .ancestors()
                    .any(|a| a == *s.parameters.path)
            }) {
                let metadata = ancestor.metadata().clone();

                self.dependencies
                    .entry(file.id())
                    .or_default()
                    .insert(metadata.id);

                file.relationships.requires.push(metadata);
            }

            // Save the metadata of explicit dependencies that this
            // file should depend on.
            for dependency in &file.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if file.may_depend_on(&resource) {
                            let metadata = resource.metadata().clone();

                            if self.dependency_introduces_loop(resource.id(), file.id()) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = file.kind(),
                                    path;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    file.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(file.id())
                                .or_default()
                                .insert(metadata.id)
                            {
                                file.relationships.requires.push(metadata);
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = file.kind(),
                                path;
                                "{} cannot depend on {}",
                                file.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "file",
                            path;
                            "{} depends on {} which cannot be found",
                            file.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.files = _files;

        Ok(())
    }

    fn validate_directories(
        &mut self,
        paths: &mut HashSet<SafePathBuf>,
        file_paths: &HashSet<PathBuf>,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        // To iterate and modify directory resources the collection
        // must be cloned.
        // Each resource is modified on the basis of other (directory)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _directories = self.directories.clone();
        _directories.sort_by(|a, b| a.parameters.path.cmp(&b.parameters.path));

        for directory in _directories.iter_mut() {
            let path = directory.parameters.path.display().to_string();

            // Check for uniqueness of the path parameter.
            if !paths.insert(directory.parameters.path.clone()) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = "directory",
                    path;
                    "path `{}` appears multiple times, must be unique among resources of type `file`, `symlink` and `directory`",
                    path
                );

                return Err(Terminate);
            }

            // Files (their paths) cannot be parents to directories.
            // Check if any file conflicts with this directory in that regard.
            if let Some(parent) = &directory.parameters.path.parent() {
                if file_paths.contains(*parent) {
                    error!(
                        scope,
                        client:% = self.name,
                        resource = "directory",
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
            for child in self.directories.iter().filter(|d| {
                d.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            }) {
                directory.relationships.children.push(child.into());
            }

            for child in self.files.iter().filter(|f| {
                f.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            }) {
                directory.relationships.children.push(child.into());
            }

            for child in self.symlinks.iter().filter(|s| {
                s.parameters
                    .path
                    .parent()
                    .is_some_and(|path| path == *directory.parameters.path)
            }) {
                directory.relationships.children.push(child.into());
            }

            // Save the metadata of ancestral directories and symlinks
            // that this directory depends on.
            for ancestor in self.directories.iter().filter(|d| {
                directory
                    .parameters
                    .path
                    .ancestors()
                    .skip(1)
                    .any(|a| a == *d.parameters.path)
            }) {
                let metadata = directory.metadata();
                let other = ancestor.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                directory.relationships.requires.push(other);
            }

            for ancestor in self.symlinks.iter().filter(|s| {
                directory
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *s.parameters.path)
            }) {
                let metadata = directory.metadata();
                let other = ancestor.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                directory.relationships.requires.push(other);
            }

            // Save the metadata of explicit dependencies that this
            // directory should depend on.
            for dependency in &directory.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if directory.may_depend_on(&resource) {
                            let metadata = directory.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "directory",
                                    path;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    directory.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                directory.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "directory",
                                path;
                                "{} cannot depend on {}",
                                directory.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "directory",
                            path;
                            "{} depends on {} which cannot be found",
                            directory.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.directories = _directories;

        Ok(())
    }

    fn validate_symlinks(
        &mut self,
        paths: &mut HashSet<SafePathBuf>,
        file_paths: &HashSet<PathBuf>,
    ) -> Result<(), Terminate> {
        let scope = "validation";

        // To iterate and modify symlink resources the collection
        // must be cloned.
        // Each resource is modified on the basis of other (symlink)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _symlinks = self.symlinks.clone();
        _symlinks.sort_by(|a, b| a.parameters.path.cmp(&b.parameters.path));

        for symlink in _symlinks.iter_mut() {
            let path = symlink.parameters.path.display().to_string();

            // Check for uniqueness of the path parameter.
            if !paths.insert(symlink.parameters.path.clone()) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = "symlink",
                    path;
                    "path `{}` appears multiple times, must be unique among resources of type `file`, `symlink` and `directory`",
                    path
                );

                return Err(Terminate);
            }

            // Files (their paths) cannot be parents to symlinks.
            // Check if any file conflicts with this symlink in that regard.
            if let Some(parent) = &symlink.parameters.path.parent() {
                if file_paths.contains(*parent) {
                    error!(
                        scope,
                        client:% = self.name,
                        resource = "symlink",
                        path;
                        "file `{}` is found to be a parent of this symlink, but files cannot be parents to symlinks",
                        parent.display()
                    );

                    return Err(Terminate);
                }
            }

            // Save metadata of ancestral directories that the symlink
            // depends on.
            for ancestor in self.directories.iter().filter(|d| {
                symlink
                    .parameters
                    .path
                    .ancestors()
                    .any(|a| a == *d.parameters.path)
            }) {
                let metadata = symlink.metadata();
                let other = ancestor.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                symlink.relationships.requires.push(other);
            }

            // Save metadata of ancestral symlinks that this symlink
            // depends on.
            for ancestor in self.symlinks.iter().filter(|s| {
                symlink
                    .parameters
                    .path
                    .ancestors()
                    .skip(1)
                    .any(|a| a == *s.parameters.path)
            }) {
                let metadata = symlink.metadata();
                let other = ancestor.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                symlink.relationships.requires.push(other);
            }

            // Save metadata of the target that this symlink points to.
            if let Some(other) = self
                .directories
                .iter()
                .find(|d| d.parameters.path == symlink.parameters.target)
                .map(|d| d.metadata().clone())
                .or(self
                    .files
                    .iter()
                    .find(|f| f.parameters.path == symlink.parameters.target)
                    .map(|f| f.metadata().clone()))
            {
                let metadata = symlink.metadata();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                symlink.relationships.requires.push(other);
            }
            // Save the metadata of explicit dependencies that this
            // symlink should depend on.
            for dependency in &symlink.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if symlink.may_depend_on(&resource) {
                            let metadata = symlink.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "symlink",
                                    path;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    symlink.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                symlink.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "symlink",
                                path;
                                "{} cannot depend on {}",
                                symlink.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "symlink",
                            path;
                            "{} depends on {} which cannot be found",
                            symlink.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.symlinks = _symlinks;

        Ok(())
    }

    fn validate_hosts(&mut self) -> Result<(), Terminate> {
        let scope = "validation";

        // Save host IP addresses to check for their uniqueness.
        let mut ip_addresses = HashSet::new();

        // To iterate and modify host resources the collection must
        // be cloned.
        // Each resource is modified on the basis of other (host)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _hosts = self.hosts.clone();

        for host in _hosts.iter_mut() {
            let ip_address = host.parameters.ip_address.to_string();

            // Check for uniqueness of the IP address parameter.
            if !ip_addresses.insert(host.parameters.ip_address) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = "host",
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
                .files
                .iter()
                .find(|f| *f.parameters.path == host.parameters.target)
            {
                if file.parameters.content.is_some() || file.parameters.source.is_some() {
                    error!(
                        scope,
                        client:% = self.name,
                        resource = "host",
                        ip_address;
                        "there cannot be both a {} resource and a {} whose `content` or `source` parameters are set",
                        host.repr(),
                        file.repr()
                    );

                    return Err(Terminate);
                } else {
                    let metadata = host.metadata();
                    let other = file.metadata().clone();

                    self.dependencies
                        .entry(metadata.id)
                        .or_default()
                        .insert(other.id);

                    host.relationships.requires.push(other);
                }
            }

            if let Some(symlink) = self
                .symlinks
                .iter()
                .find(|s| *s.parameters.path == host.parameters.target)
            {
                let metadata = host.metadata();
                let other = symlink.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                host.relationships.requires.push(other);
            }

            // Save the metadata of explicit dependencies that this
            // host should depend on.
            for dependency in &host.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if host.may_depend_on(&resource) {
                            let metadata = host.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "host",
                                    ip_address;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    host.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                host.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "host",
                                ip_address;
                                "{} cannot depend on {}",
                                host.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "host",
                            ip_address;
                            "{} depends on {} which cannot be found",
                            host.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.hosts = _hosts;

        Ok(())
    }

    fn validate_groups(&mut self) -> Result<(), Terminate> {
        let scope = "validation";

        // Save group names to check for their uniqueness.
        let mut names = HashSet::new();

        // To iterate and modify group resources the collection must
        // be cloned.
        // Each resource is modified on the basis of other (group)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _groups = self.groups.clone();

        for group in _groups.iter_mut() {
            let name = group.parameters.name.to_string();

            // Check for uniqueness of the name parameter.
            if !names.insert(group.parameters.name.clone()) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = "group",
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
            for user in &self.users {
                if user.parameters.group == group.parameters.name {
                    let metadata = group.metadata();
                    let other = user.metadata().clone();

                    self.dependencies
                        .entry(metadata.id)
                        .or_default()
                        .insert(other.id);

                    group.relationships.requires.push(other);
                }
            }

            // Save the metadata of explicit dependencies that this
            // group should depend on.
            for dependency in &group.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if group.may_depend_on(&resource) {
                            let metadata = group.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "group",
                                    name;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    group.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                group.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "group",
                                name;
                                "{} cannot depend on {}",
                                group.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "group",
                            name;
                            "{} depends on {} which cannot be found",
                            group.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.groups = _groups;

        Ok(())
    }

    fn validate_users(&mut self) -> Result<(), Terminate> {
        let scope = "validation";

        // Save user names to check for their uniqueness.
        let mut names = HashSet::new();

        // To iterate and modify user resources the collection must
        // be cloned.
        // Each resource is modified on the basis of other (user)
        // resources, e.g. to validate dependencies.
        // As such we need both a mutable object as well as immutable
        // collections of resources to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _users = self.users.clone();

        for user in _users.iter_mut() {
            let name = user.parameters.name.to_string();

            // Check for uniqueness of the name parameter.
            if !names.insert(user.parameters.name.clone()) {
                error!(
                    scope,
                    client:% = self.name,
                    resource = "user",
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
                if let Some(group) = self
                    .groups
                    .iter()
                    .find(|group| group.parameters.name == *name)
                {
                    let metadata = user.metadata();
                    let other = group.metadata().clone();

                    self.dependencies
                        .entry(metadata.id)
                        .or_default()
                        .insert(other.id);

                    user.relationships.requires.push(other);
                }
            }

            // Save the metadata of explicit dependencies that this
            // user should depend on.
            for dependency in &user.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if user.may_depend_on(&resource) {
                            let metadata = user.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "user",
                                    name;
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    user.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                user.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "user",
                                name;
                                "{} cannot depend on {}",
                                user.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "user",
                            name;
                            "{} depends on {} which cannot be found",
                            user.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }
        }

        self.users = _users;

        Ok(())
    }

    fn validate_resolv_conf(&mut self) -> Result<(), Terminate> {
        let scope = "validation";

        // To modify the resolv.conf resource it must be cloned.
        // Each resource is modified on the basis of other resources,
        // e.g. to validate dependencies.
        // As such we need both a mutable object as well as an immutable
        // resource catalog to check against.
        // Since ownerships rules need to be satisfied as well, a clone
        // is inevitable.
        let mut _resolv_conf = self.resolv_conf.clone();

        self.resolv_conf = None;

        if let Some(mut resolv_conf) = _resolv_conf {
            // Save the metadata of the target file or symlink corresponding to the
            // /etc/resolv.conf file.
            // Also check if the target is a file resource that sets its
            // content or source parameter. This combination is not supported.
            if let Some(file) = self
                .files
                .iter()
                .find(|f| *f.parameters.path == resolv_conf.parameters.target)
            {
                if file.parameters.content.is_some() || file.parameters.source.is_some() {
                    error!(
                        scope,
                        client:% = self.name,
                        resource = "resolv.conf";
                        "there cannot be both a {} resource and a {} whose `content` or `source` parameters are set",
                        resolv_conf.repr(),
                        file.repr()
                    );

                    return Err(Terminate);
                } else {
                    let metadata = resolv_conf.metadata();
                    let other = file.metadata().clone();

                    self.dependencies
                        .entry(metadata.id)
                        .or_default()
                        .insert(other.id);

                    resolv_conf.relationships.requires.push(other);
                }
            }

            if let Some(symlink) = self
                .symlinks
                .iter()
                .find(|s| *s.parameters.path == resolv_conf.parameters.target)
            {
                let metadata = resolv_conf.metadata();
                let other = symlink.metadata().clone();

                self.dependencies
                    .entry(metadata.id)
                    .or_default()
                    .insert(other.id);

                resolv_conf.relationships.requires.push(other);
            }

            // Save the metadata of explicit dependencies that this
            // resource should depend on.
            for dependency in &resolv_conf.relationships._requires {
                match self.resolve_dependency(dependency) {
                    Some(resource) => {
                        if resolv_conf.may_depend_on(&resource) {
                            let metadata = resolv_conf.metadata();
                            let other = resource.metadata().clone();

                            if self.dependency_introduces_loop(other.id, metadata.id) {
                                error!(
                                    scope,
                                    client:% = self.name,
                                    resource = "resolv.conf";
                                    "{} cannot depend on {} as it would introduce a dependency loop",
                                    resolv_conf.repr(),
                                    resource.repr()
                                );

                                return Err(Terminate);
                            } else if self
                                .dependencies
                                .entry(metadata.id)
                                .or_default()
                                .insert(other.id)
                            {
                                resolv_conf.relationships.requires.push(metadata.clone());
                            }
                        } else {
                            error!(
                                scope,
                                client:% = self.name,
                                resource = "resolv.conf";
                                "{} cannot depend on {}",
                                resolv_conf.repr(),
                                resource.repr()
                            );

                            return Err(Terminate);
                        }
                    }
                    None => {
                        error!(
                            scope,
                            client:% = self.name,
                            resource = "resolv.conf";
                            "{} depends on {} which cannot be found",
                            resolv_conf.repr(),
                            dependency.repr()
                        );

                        return Err(Terminate);
                    }
                }
            }

            self.resolv_conf = Some(resolv_conf);
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

pub mod serialize {
    use common::{Hostname, Links};
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize)]
    pub struct Client {
        links: Links,
        name: Hostname,
    }

    impl From<super::Client> for Client {
        fn from(client: super::Client) -> Self {
            Self {
                links: Links {
                    this: format!("/api/clients/{}", client.name()),
                    ..Default::default()
                },
                name: client.name().clone(),
            }
        }
    }
}
