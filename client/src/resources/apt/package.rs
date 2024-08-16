use crate::resources::{Action, Resource, ResourceTrait};
use common::{PackageEnsure, PackageName, PackageVersion, ResourceMetadata};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{collections::HashMap, default::Default, fs, process::Command, str::FromStr};
use uuid::Uuid;

const DPKG_QUERY: &str = "/usr/bin/dpkg-query";
const APT_GET: &str = "/usr/bin/apt-get";

#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for Package {
    fn kind(&self) -> &str {
        "apt::package"
    }

    fn display(&self) -> String {
        self.parameters.name.to_string()
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn dependencies(&self) -> &[ResourceMetadata] {
        self.relationships.requires.as_slice()
    }

    fn maybe_return_early(
        &self,
        pid: u32,
        applied_resources: &HashMap<Uuid, Resource>,
    ) -> Option<Action> {
        if let Some(dependency) = self.find_failed_dependency(applied_resources) {
            let action = Action::Skipped;

            warn!(pid,
                  resource = self.kind(),
                  name = self.display(),
                  result:% = action;
                  "skipping {} as {} has failed to apply",
                  self.repr(),
                  dependency.repr()
            );

            return Some(action);
        }

        if let Some(dependency) = self.find_skipped_dependency(applied_resources) {
            let action = Action::Skipped;

            warn!(pid,
                  resource = self.kind(),
                  name = self.display(),
                  result:% = action;
                  "skipping {} as {} has been skipped",
                  self.repr(),
                  dependency.repr()
            );

            return Some(action);
        }

        if self.parameters.ensure.is_present() {
            if let Some(dependency) = self.find_absent_dependency(applied_resources) {
                let action = Action::Failed;

                error!(
                    pid,
                    resource = self.kind(),
                    name = self.display(),
                    result:% = action;
                    "cannot apply {} as {} is set to absent",
                    self.repr(),
                    dependency.repr()
                );

                return Some(action);
            }
        }

        None
    }

    fn check_prerequisites(&self, pid: u32) -> Option<Action> {
        fn find(package: &Package, pid: u32, program: &str) -> Option<Action> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        None
                    } else {
                        let action = Action::Failed;

                        error!(
                            pid,
                            resource = package.kind(),
                            name = package.display(),
                            result:% = action;
                            "cannot apply {} as executable `{}` is missing",
                            package.repr(),
                            program
                        );

                        Some(action)
                    }
                }
                Err(error) => {
                    let action = Action::Failed;

                    error!(
                        pid,
                        resource = package.kind(),
                        name = package.display(),
                        result:% = action;
                        "cannot apply {} as executable `{}` cannot be accessed: {}",
                        package.repr(),
                        program,
                        error
                    );

                    Some(action)
                }
            }
        }

        let dpkg_query = find(self, pid, DPKG_QUERY);
        let apt_get = find(self, pid, APT_GET);

        dpkg_query.or(apt_get)
    }
}

impl Package {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, pid: u32, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(pid, applied_resources) {
            self.action = action;
            return;
        }

        if let Some(action) = self.check_prerequisites(pid) {
            self.action = action;
            return;
        }

        debug!(pid,
               resource = self.kind(),
               name = self.display();
               "applying {}",
               self.repr(),
        );

        match self._apply(pid) {
            Ok(action) => {
                info!(pid,
                      resource = self.kind(),
                      name = self.display(),
                      result:% = action;
                      "successfully applied {}",
                      self.repr(),
                );

                self.action = action;
            }
            Err(error) => {
                let action = Action::Failed;

                error!(pid,
                       resource = self.kind(),
                       name = self.display(),
                       result:% = action;
                       "failed to apply {}: {:#}",
                       self.repr(),
                       error
                );

                self.action = action;
            }
        }
    }

    /// Apply this resource's configuration.
    pub fn _apply(&self, pid: u32) -> Result<Action, anyhow::Error> {
        if let Some(current_version) = self.exists(pid)? {
            match self.parameters.ensure {
                PackageEnsure::Present => {
                    if self
                        .parameters
                        .version
                        .as_ref()
                        .is_some_and(|version| *version != current_version)
                    {
                        self.install(pid, Action::Changed)
                    } else {
                        Ok(Action::Unchanged)
                    }
                }
                PackageEnsure::Absent => self.remove(pid, false),
                PackageEnsure::Purged => self.remove(pid, true),
            }
        } else {
            match self.parameters.ensure {
                PackageEnsure::Present => self.install(pid, Action::Created),
                PackageEnsure::Absent | PackageEnsure::Purged => Ok(Action::Unchanged),
            }
        }
    }

    /// Install or up-/downgrade the package.
    /// The `action` parameter is used to return the correct action
    /// according to the context this function is executed in.
    fn install(&self, pid: u32, action: Action) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "installing package"
        );

        let mut command = Command::new(APT_GET);
        command.arg("install");

        if let Some(version) = &self.parameters.version {
            command.arg(&format!("{}={}", self.parameters.name.as_str(), version));
        } else {
            command.arg(self.parameters.name.as_str());
        }

        let output = command
            .arg("--quiet")
            .arg("--quiet")
            .arg("--yes")
            .output()?;

        if !output.status.success() {
            let s = String::from_utf8_lossy(&output.stderr).to_owned();

            anyhow::bail!(
                "failed to install package, {} exited with status {}: {}",
                APT_GET,
                output.status.code().unwrap(),
                s.trim_end()
            );
        }

        Ok(action)
    }

    /// Remove the package from the system.
    fn remove(&self, pid: u32, purge: bool) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "removing package"
        );

        let mut command = Command::new(APT_GET);
        command.arg("remove");

        if purge {
            command.arg("--purge");
        }

        let output = command
            .arg("--quiet")
            .arg("--quiet")
            .arg("--yes")
            .arg(self.parameters.name.as_str())
            .output()?;

        if !output.status.success() {
            let s = String::from_utf8_lossy(&output.stderr).to_owned();

            anyhow::bail!(
                "failed to remove package, {} exited with status {}: {}",
                APT_GET,
                output.status.code().unwrap(),
                s.trim_end()
            );
        }

        Ok(Action::Deleted)
    }

    /// Try to find a package by this name within the system.
    fn exists(&self, pid: u32) -> Result<Option<PackageVersion>, anyhow::Error> {
        let mut command = Command::new(DPKG_QUERY);
        command.args(["-W", "-f", "'${VERSION}'", self.parameters.name.as_str()]);

        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "executing {:?} with args {:?}",
            command.get_program(),
            command.get_args()
        );

        let output = command.output()?;

        let s = String::from_utf8_lossy(&output.stdout).to_owned();

        if output.status.success() {
            match PackageVersion::from_str(s.trim_start_matches('\'').trim_end_matches('\'')) {
                Ok(version) => Ok(Some(version)),
                Err(error) => anyhow::bail!(
                    "failed to parse output from dpkg-query as package version: {}",
                    error
                ),
            }
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub ensure: PackageEnsure,
    pub name: PackageName,
    pub version: Option<PackageVersion>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Relationships {
    requires: Vec<ResourceMetadata>,
}
