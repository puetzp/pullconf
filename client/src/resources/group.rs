use super::{Action, Resource, ResourceTrait};
use common::{Ensure, Groupname, ResourceMetadata};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    collections::HashMap,
    default::Default,
    fs,
    process::{Command, Stdio},
};
use uuid::Uuid;

const GROUPADD: &str = "/usr/sbin/groupadd";
const GROUPDEL: &str = "/usr/sbin/groupdel";

#[derive(Clone, Debug, Deserialize)]
pub struct Group {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for Group {
    fn kind(&self) -> &str {
        "group"
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
        fn find(group: &Group, pid: u32, program: &str) -> Option<Action> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        None
                    } else {
                        let action = Action::Failed;

                        error!(
                            pid,
                            resource = group.kind(),
                            name = group.display(),
                            result:% = action;
                            "cannot apply {} as executable `{}` is missing",
                            group.repr(),
                            program
                        );

                        Some(action)
                    }
                }
                Err(error) => {
                    let action = Action::Failed;

                    error!(
                        pid,
                        resource = group.kind(),
                        name = group.display(),
                        result:% = action;
                        "cannot apply {} as executable `{}` cannot be accessed: {}",
                        group.repr(),
                        program,
                        error
                    );

                    Some(action)
                }
            }
        }

        let groupadd = find(self, pid, GROUPADD);
        let groupdel = find(self, pid, GROUPDEL);

        groupadd.or(groupdel)
    }
}

impl Group {
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
        if exists(&self.parameters.name)? {
            match self.parameters.ensure {
                Ensure::Present => Ok(Action::Unchanged),
                Ensure::Absent => self.delete(pid),
            }
        } else {
            match self.parameters.ensure {
                Ensure::Present => self.create(pid),
                Ensure::Absent => Ok(Action::Unchanged),
            }
        }
    }

    /// Add the group to the system.
    fn create(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "creating group"
        );

        let mut command = Command::new(GROUPADD);

        if self.parameters.system {
            command.arg("--system");
        }

        let status = command
            .arg(self.parameters.name.as_str())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()?;

        if !status.success() {
            anyhow::bail!(
                "failed to create group, {} exited with status {}",
                GROUPADD,
                status.code().unwrap()
            );
        }

        Ok(Action::Created)
    }

    /// Delete the group from the system.
    fn delete(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "deleting group"
        );

        let status = Command::new(GROUPDEL)
            .arg(self.parameters.name.as_str())
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()?;

        if !status.success() {
            anyhow::bail!(
                "failed to delete group, {} exited with status {}",
                GROUPDEL,
                status.code().unwrap()
            );
        }

        Ok(Action::Deleted)
    }
}

/// Try to find a group by its name within the system.
pub(super) fn exists(name: &Groupname) -> Result<bool, anyhow::Error> {
    for line in fs::read_to_string("/etc/group")?.lines() {
        if let Some((first_column, _)) = line.split_once(':') {
            if first_column == name.as_str() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Groupname,
    pub system: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Relationships {
    requires: Vec<ResourceMetadata>,
}
