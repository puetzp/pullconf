use crate::resources::{Resource, ResourceTrait};
use common::{
    resources::apt::package::{Ensure, Parameters, Relationships, Version},
    Action, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
use serde::Deserialize;
use std::{collections::HashMap, fs, process::Command, str::FromStr};
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

    fn action(&self) -> Action {
        self.action
    }

    fn dependencies(&self) -> &[ResourceMetadata] {
        self.relationships.requires.as_slice()
    }

    fn triggers(&self) -> &[TriggerMetadata] {
        self.relationships.triggers.as_slice()
    }

    fn is_present(&self) -> bool {
        self.parameters.ensure.is_present()
    }

    fn check_prerequisites(&self) -> Option<Action> {
        fn find(package: &Package, program: &str) -> Option<Action> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        None
                    } else {
                        error!(
                            "`{}`: cannot apply resource as executable `{}` is missing",
                            package.repr(),
                            program
                        );

                        Some(Action::Failed)
                    }
                }
                Err(error) => {
                    error!(
                        "`{}`: cannot apply resource as executable `{}` cannot be accessed: {}",
                        package.repr(),
                        program,
                        error
                    );

                    Some(Action::Failed)
                }
            }
        }

        let dpkg_query = find(self, DPKG_QUERY);
        let apt_get = find(self, APT_GET);

        dpkg_query.or(apt_get)
    }
}

impl Package {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(applied_resources) {
            self.action = action;
            return;
        }

        if let Some(action) = self.check_prerequisites() {
            self.action = action;
            return;
        }

        debug!("`{}`: applying resource", self.repr(),);

        match self._apply() {
            Ok(action) => {
                info!("`{}`: successfully applied resource", self.repr(),);

                self.action = action;
            }
            Err(error) => {
                error!("`{}`: failed to apply resource: {:#}", self.repr(), error);

                self.action = Action::Failed;
            }
        }
    }

    /// Apply this resource's configuration.
    pub fn _apply(&self) -> Result<Action, anyhow::Error> {
        if let Some(current_version) = self.exists()? {
            match self.parameters.ensure {
                Ensure::Present => {
                    if self
                        .parameters
                        .version
                        .as_ref()
                        .is_some_and(|version| *version != current_version)
                    {
                        self.install(Action::Changed)
                    } else {
                        Ok(Action::Unchanged)
                    }
                }
                Ensure::Absent => self.remove(false),
                Ensure::Purged => self.remove(true),
            }
        } else {
            match self.parameters.ensure {
                Ensure::Present => self.install(Action::Created),
                Ensure::Absent | Ensure::Purged => Ok(Action::Unchanged),
            }
        }
    }

    /// Install or up-/downgrade the package.
    /// The `action` parameter is used to return the correct action
    /// according to the context this function is executed in.
    fn install(&self, action: Action) -> Result<Action, anyhow::Error> {
        debug!("`{}`: installing package", self.repr());

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
                "failed to install package, `{}` exited with status `{}`: {}",
                APT_GET,
                output.status.code().unwrap(),
                s.trim_end()
            );
        }

        Ok(action)
    }

    /// Remove the package from the system.
    fn remove(&self, purge: bool) -> Result<Action, anyhow::Error> {
        debug!("`{}`: removing package", self.repr());

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
                "failed to remove package, `{}` exited with status `{}`: {}",
                APT_GET,
                output.status.code().unwrap(),
                s.trim_end()
            );
        }

        Ok(Action::Deleted)
    }

    /// Try to find a package by this name within the system.
    fn exists(&self) -> Result<Option<Version>, anyhow::Error> {
        let mut command = Command::new(DPKG_QUERY);
        command.args(["-W", "-f", "'${VERSION}'", self.parameters.name.as_str()]);

        debug!(
            "`{}`: executing {:?} with args {:?}",
            self.repr(),
            command.get_program(),
            command.get_args()
        );

        let output = command.output()?;

        let s = String::from_utf8_lossy(&output.stdout).to_owned();

        if output.status.success() {
            if s == "''" {
                return Ok(None);
            }

            match Version::from_str(s.trim_start_matches('\'').trim_end_matches('\'')) {
                Ok(version) => Ok(Some(version)),
                Err(error) => anyhow::bail!(
                    "failed to parse output from `{}` as package version: {}",
                    DPKG_QUERY,
                    error
                ),
            }
        } else {
            Ok(None)
        }
    }
}
