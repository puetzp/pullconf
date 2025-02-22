use super::{Resource, ResourceTrait};
use common::{
    resources::group::{Name, Parameters, Relationships},
    Action, Ensure, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
use serde::Deserialize;
use std::{
    collections::HashMap,
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

    fn action(&self) -> Action {
        self.action
    }

    fn dependencies(&self) -> &[ResourceMetadata] {
        self.relationships.requires.as_slice()
    }

    fn predecessors(&self) -> &[ResourceMetadata] {
        self.relationships.after.as_slice()
    }

    fn triggers(&self) -> &[TriggerMetadata] {
        self.relationships.triggers.as_slice()
    }

    fn is_present(&self) -> bool {
        self.parameters.ensure.is_present()
    }

    fn check_prerequisites(&self) -> Option<Action> {
        fn find(group: &Group, program: &str) -> Option<Action> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        None
                    } else {
                        error!(
                            "`{}`: cannot apply resource as executable `{}` is missing",
                            group.repr(),
                            program
                        );

                        Some(Action::Failed)
                    }
                }
                Err(error) => {
                    error!(
                        "`{}`: cannot apply resource as executable `{}` cannot be accessed: {}",
                        group.repr(),
                        program,
                        error
                    );

                    Some(Action::Failed)
                }
            }
        }

        let groupadd = find(self, GROUPADD);
        let groupdel = find(self, GROUPDEL);

        groupadd.or(groupdel)
    }
}

impl Group {
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
        if exists(&self.parameters.name)? {
            match self.parameters.ensure {
                Ensure::Present => Ok(Action::Unchanged),
                Ensure::Absent => self.delete(),
            }
        } else {
            match self.parameters.ensure {
                Ensure::Present => self.create(),
                Ensure::Absent => Ok(Action::Unchanged),
            }
        }
    }

    /// Add the group to the system.
    fn create(&self) -> Result<Action, anyhow::Error> {
        debug!("`{}`: creating group", self.repr());

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
    fn delete(&self) -> Result<Action, anyhow::Error> {
        debug!("`{}`: deleting group", self.repr());

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
pub(super) fn exists(name: &Name) -> Result<bool, anyhow::Error> {
    for line in fs::read_to_string("/etc/group")?.lines() {
        if let Some((first_column, _)) = line.split_once(':') {
            if first_column == name.as_str() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
