use super::{Resource, ResourceTrait};
use anyhow::Context;
use common::{
    resources::symlink::{Parameters, Relationships},
    Action, Ensure, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
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
}

impl Symlink {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(applied_resources) {
            self.action = action;
            return;
        }

        debug!("`{}`: applying resource", self.repr(),);

        match self._apply() {
            Ok(action) => {
                info!("`{}`: successfully applied resource", self.repr());

                self.action = action;
            }
            Err(error) => {
                error!("`{}`: failed to apply resource: {:#}", self.repr(), error);

                self.action = Action::Failed;
            }
        }
    }

    /// Apply this resource's configuration. This function can be called repeatedly
    /// and produce the same result if neither the configuration nor the actual
    /// symlink in the file system change.
    pub fn _apply(&self) -> Result<Action, anyhow::Error> {
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
                    match self.create() {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(
                                "`{}`: deleting symlink as at least one condition failed",
                                self.repr(),
                            );

                            fs::remove_file(&*self.parameters.path).ok();

                            Err(error)
                        }
                    }
                }
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(metadata) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(metadata),
                Ensure::Absent => self.delete(metadata),
            },
        }
    }

    /// Re-create this symlink if the current target differs from the one that was configured.
    fn maybe_update(&self, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: symlink exists, checking if current and desired states match",
            self.repr()
        );

        let mut action = Action::default();

        if !metadata.is_symlink() {
            anyhow::bail!("failed to update resource as it is not a symlink")
        }

        match fs::read_link(&*self.parameters.path) {
            Ok(target) => {
                if target != *self.parameters.target {
                    debug!(
                        "`{}`: symlink exists, but points to the wrong target, will be deleted and re-created",
                        self.repr()
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
    fn create(&self) -> Result<Action, anyhow::Error> {
        debug!("`{}`: creating symlink as it does no exist", self.repr(),);

        create_symlink(&*self.parameters.target, &*self.parameters.path)
            .context("failed to create symlink")?;

        Ok(Action::Created)
    }

    /// Delete this symlink.
    fn delete(&self, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!("`{}`: deleting symlink", self.repr());

        if metadata.is_symlink() {
            fs::remove_file(&*self.parameters.path).context("failed to delete symlink")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a symlink")
        }

        Ok(Action::Deleted)
    }
}
