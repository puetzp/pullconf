use crate::resources::{Action, Resource, ResourceTrait};
use anyhow::Context;
use common::{
    resources::cron::job::{Parameters, Relationships},
    Ensure, ResourceMetadata,
};
use log::{debug, error, info, warn};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    time::SystemTime,
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for Job {
    fn kind(&self) -> &str {
        "cron::job"
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
}

impl Job {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, pid: u32, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(pid, applied_resources) {
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
                      self.repr()
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
        let file = match fs::File::open(&*self.parameters.target) {
            Ok(file) => Some(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => anyhow::bail!(
                "failed to open file `{}`: {:#}",
                self.parameters.target.display(),
                error
            ),
        };

        match self.parameters.ensure {
            Ensure::Present => {
                // Build the desired file content from the resource parameters.
                let mut content = format!(
                    "{} {} {}\n",
                    self.parameters.schedule, self.parameters.user, self.parameters.command
                );

                for item in &self.parameters.environment {
                    let line = match &item.value {
                        Some(value) => {
                            if value.is_empty() {
                                format!("{}=\"\"\n", item.name)
                            } else {
                                format!("{}=\"{}\"\n", item.name, value)
                            }
                        }
                        None => format!("{}=\n", item.name),
                    };

                    content.insert_str(0, &line);
                }

                // Create or update the file, depending on the current file
                // state.
                match file {
                    Some(mut file) => {
                        // Compute the checksum from the desired file contents.
                        let checksum = format!("{:x}", Sha256::digest(content.as_bytes()));

                        // Compute the checksum of the current file contents.
                        let current_checksum = {
                            let mut data = vec![];
                            file.read_to_end(&mut data)?;
                            format!("{:x}", Sha256::digest(data))
                        };

                        // If the current and desired content checksums do not
                        // match, update the file.
                        if current_checksum == checksum {
                            Ok(Action::Unchanged)
                        } else {
                            let mtime = file.metadata()?.modified()?;

                            self.update(pid, content, mtime)
                        }
                    }
                    None => match self.create(pid, content) {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(pid,
                                   resource = self.kind(),
                                   name = self.display();
                                   "deleting file `{}` as at least one condition failed",
                                   self.parameters.target.display()
                            );
                            fs::remove_file(&*self.parameters.target).ok();
                            Err(error)
                        }
                    },
                }
            }
            Ensure::Absent => match file {
                Some(file) => self.delete(pid, file.metadata()?),
                None => Ok(Action::Unchanged),
            },
        }
    }

    /// Update the target file contents.
    fn update(
        &self,
        pid: u32,
        content: String,
        mtime: SystemTime,
    ) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               name = self.display();
               "updating target file `{}`",
               self.parameters.target.display()
        );

        let tmp_path = format!("/tmp/{}.pullconf", self.parameters.name);

        fs::write(&tmp_path, content.as_bytes())?;

        if fs::metadata(&self.parameters.target)
            .context("failed to query target file metadata")?
            .modified()
            .is_ok_and(|_mtime| _mtime == mtime)
        {
            debug!(
                pid,
                resource = self.kind(),
                name = self.display();
                "renaming replacement file `{}` to original target file {}",
                tmp_path,
                self.parameters.target.display()
            );

            fs::rename(tmp_path, &self.parameters.target)
                .context("failed to replace target file")?;
        } else {
            anyhow::bail!(
                "target file `{}` changed before replacement file `{}` could be renamed",
                self.parameters.target.display(),
                tmp_path
            );
        }

        Ok(Action::Changed)
    }

    /// Create the target file.
    fn create(&self, pid: u32, content: String) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               name = self.display();
               "creating target file `{}` as it does not exist",
               self.parameters.target.display()
        );

        fs::write(&self.parameters.target, content)
            .context("failed to write contents to target file")?;

        Ok(Action::Created)
    }

    /// Delete the target file.
    fn delete(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               name = self.display();
               "deleting target file `{}`",
               self.parameters.target.display()
        );

        if metadata.is_file() {
            fs::remove_file(&*self.parameters.target).context("failed to delete target file")?
        } else {
            anyhow::bail!(
                "failed to delete target `{}` as it is not a file",
                self.parameters.target.display()
            )
        }

        Ok(Action::Deleted)
    }
}
