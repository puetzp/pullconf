use super::{Action, Resource, ResourceTrait};
use crate::util::uid_and_gid;
use anyhow::Context;
use common::{
    resources::directory::{Parameters, Relationships},
    Ensure, ResourceMetadata,
};
use log::{debug, error, info};
use serde::Deserialize;
use std::{
    collections::HashMap,
    default::Default,
    fs, io,
    os::unix::fs::{chown, MetadataExt},
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct Directory {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for Directory {
    fn kind(&self) -> &str {
        "directory"
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

    fn triggers(&self) -> &[ResourceMetadata] {
        self.relationships.triggers.as_slice()
    }

    fn is_present(&self) -> bool {
        self.parameters.ensure.is_present()
    }
}

impl Directory {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(applied_resources) {
            self.action = action;
            return;
        }

        debug!("`{}`: applying resource", self.repr());

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

    /// Apply this resource's configuration. This function can be called repeatedly
    /// and produce the same result if neither the configuration nor the actual
    /// directory in the file system change.
    pub fn _apply(&self) -> Result<Action, anyhow::Error> {
        let metadata = match fs::metadata(&*self.parameters.path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => anyhow::bail!("failed to query directory metadata: {:#}", error),
        };

        match metadata {
            None => match self.parameters.ensure {
                Ensure::Present => {
                    // When some error occurs during directory creation it can be safely
                    // deleted again (cleaned up) as it did not exist in the first place.
                    match self.create() {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(
                                "`{}`: deleting `{}` as at least one condition failed",
                                self.repr(),
                                self.parameters.path.display(),
                            );
                            fs::remove_dir(&*self.parameters.path).ok();
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

    /// Change the directory's ownership parameters if the actual ownership
    /// configuration in the file system differ from the desired state.
    fn maybe_update(&self, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: directory exists, checking if current and desired states match",
            self.repr()
        );

        let mut action = Action::default();

        if !metadata.is_dir() {
            anyhow::bail!("failed to update resource as it is not a directory")
        }

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        if metadata.uid() != uid || metadata.gid() != gid {
            debug!(
                "`{}`: updating directory owner (uid: `{}`) and group (gid: `{}`)",
                self.repr(),
                uid,
                gid
            );

            chown(&*self.parameters.path, Some(uid), Some(gid))
                .context("failed to set directory owner and group")?;

            action = Action::Changed;
        }

        if self.parameters.purge && !self.relationships.children.is_empty() {
            for entry in fs::read_dir(&*self.parameters.path)? {
                let entry = entry?;
                let path = entry.path();
                let kind = entry.file_type()?;

                if kind.is_dir() {
                    if !self
                        .relationships
                        .children
                        .iter()
                        .any(|child| child.is_dir(&path))
                    {
                        fs::remove_dir_all(path)?;
                        action = Action::Changed;
                    }
                } else if kind.is_file() {
                    if !self
                        .relationships
                        .children
                        .iter()
                        .any(|child| child.is_file(&path))
                    {
                        fs::remove_file(path)?;
                        action = Action::Changed;
                    }
                } else if kind.is_symlink() {
                    if !self
                        .relationships
                        .children
                        .iter()
                        .any(|child| child.is_symlink(&path))
                    {
                        fs::remove_file(path)?;
                        action = Action::Changed;
                    }
                }
            }
        }

        Ok(action)
    }

    /// Create the directory and set ownership parameters.
    fn create(&self) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: directory does not exist, creating new empty directory",
            self.repr()
        );

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        fs::create_dir(&*self.parameters.path).context("failed to create directory")?;

        debug!(
            "`{}`: setting file owner (uid: `{}`) and group (gid: `{}`)",
            self.repr(),
            uid,
            gid
        );

        chown(&*self.parameters.path, Some(uid), Some(gid))
            .context("failed to set directory owner and group")?;

        Ok(Action::Created)
    }

    // Recursively (!) delete this directory.
    fn delete(&self, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!("`{}`: deleting directory", self.repr());

        if metadata.is_dir() {
            fs::remove_dir_all(&*self.parameters.path).context("failed to delete directory")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a directory")
        }

        Ok(Action::Deleted)
    }
}
