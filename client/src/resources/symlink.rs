use super::{Action, Resource, ResourceTrait};
use anyhow::Context;
use common::{
    resources::symlink::{Parameters, Relationships},
    Ensure, ResourceMetadata,
};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    collections::HashMap, default::Default, fs, io, os::unix::fs::symlink as create_symlink,
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct Symlink {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for Symlink {
    fn kind(&self) -> &str {
        "symlink"
    }

    fn display(&self) -> String {
        self.parameters.path.display().to_string()
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
                  path = self.display(),
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
                  path = self.display(),
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
                    path = self.display(),
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

impl Symlink {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, pid: u32, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(pid, applied_resources) {
            self.action = action;
            return;
        }

        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "applying {}",
               self.repr(),
        );

        match self._apply(pid) {
            Ok(action) => {
                info!(pid,
                      resource = self.kind(),
                      path = self.display(),
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
                       path = self.display(),
                       result:% = action;
                       "failed to apply {}: {:#}",
                       self.repr(),
                       error
                );

                self.action = action;
            }
        }
    }

    /// Apply this resource's configuration. This function can be called repeatedly
    /// and produce the same result if neither the configuration nor the actual
    /// symlink in the file system change.
    pub fn _apply(&self, pid: u32) -> Result<Action, anyhow::Error> {
        // Check if the intended symlink target exists by searching for it in
        // the filesystem.
        let target_exists = match fs::symlink_metadata(&*self.parameters.target) {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => anyhow::bail!("failed to query symlink metadata: {:#}", error),
        };

        // Fail early if the symlink is set to present but the target does not exist.
        if !target_exists && self.parameters.ensure.is_present() {
            anyhow::bail!(
                "cannot create symlink as target {} does not exist",
                self.parameters.target.display()
            )
        }

        let metadata = match fs::symlink_metadata(&*self.parameters.path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => anyhow::bail!("failed to query symlink metadata: {:#}", error),
        };

        match metadata {
            None => match self.parameters.ensure {
                Ensure::Present => {
                    // When some error occurs during symlink creation it can be safely
                    // deleted again (cleaned up) as it did not exist in the first place.
                    match self.create(pid) {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(pid,
                                   resource = self.kind(),
                                   path = self.display();
                                   "deleting symlink as at least one condition failed",
                            );

                            fs::remove_file(&*self.parameters.path).ok();

                            Err(error)
                        }
                    }
                }
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(metadata) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(pid, metadata),
                Ensure::Absent => self.delete(pid, metadata),
            },
        }
    }

    /// Re-create this symlink if the current target differs from the one that was configured.
    fn maybe_update(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(
            "{}: symlink exists, checking if current and desired states match",
            self.parameters.path.display()
        );

        let mut action = Action::default();

        if !metadata.is_symlink() {
            anyhow::bail!("failed to update resource as it is not a symlink")
        }

        match fs::read_link(&*self.parameters.path) {
            Ok(target) => {
                if target != *self.parameters.target {
                    debug!(pid,
                           resource = self.kind(),
                           path = self.display();
                           "symlink exists, but points to the wrong target, will be deleted and re-created",
                    );

                    fs::remove_file(&*self.parameters.path).context("failed to delete symlink")?;

                    create_symlink(&*self.parameters.target, &*self.parameters.path)
                        .context("failed to create symlink")?;

                    action = Action::Changed;
                }
            }
            Err(error) => anyhow::bail!("failed to query current symlink target: {:#}", error),
        }

        Ok(action)
    }

    /// Create this symlink.
    fn create(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "creating symlink as it does no exist",
        );

        create_symlink(&*self.parameters.target, &*self.parameters.path)
            .context("failed to create symlink")?;

        Ok(Action::Created)
    }

    /// Delete this symlink.
    fn delete(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "deleting symlink"
        );

        if metadata.is_symlink() {
            fs::remove_file(&*self.parameters.path).context("failed to delete symlink")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a symlink")
        }

        Ok(Action::Deleted)
    }
}
