use super::{Action, Resource, ResourceTrait};
use crate::util::uid_and_gid;
use anyhow::Context;
use common::{DirectoryChildNode, Ensure, Groupname, ResourceMetadata, SafePathBuf, Username};
use log::{debug, error, info, warn};
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

impl Directory {
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
               self.repr()
        );

        match self._apply(pid) {
            Ok(action) => {
                info!(pid,
                      resource = self.kind(),
                      path = self.display(),
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
    /// directory in the file system change.
    pub fn _apply(&self, pid: u32) -> Result<Action, anyhow::Error> {
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
                    match self.create(pid) {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(pid,
                                   resource = self.kind(),
                                   path = self.display();
                                   "deleting {} as at least one condition failed",
                                   self.repr(),
                            );
                            fs::remove_dir(&*self.parameters.path).ok();
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

    /// Change the directory's ownership parameters if the actual ownership
    /// configuration in the file system differ from the desired state.
    fn maybe_update(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "directory exists, checking if current and desired states match"
        );

        let mut action = Action::default();

        if !metadata.is_dir() {
            anyhow::bail!("failed to update resource as it is not a directory")
        }

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        if metadata.uid() != uid || metadata.gid() != gid {
            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "updating directory owner (uid: {}) and group (gid: {})",
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
    fn create(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "directory does not exist, creating new empty directory"
        );

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        fs::create_dir(&*self.parameters.path).context("failed to create directory")?;

        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "setting file owner (uid: {}) and group ({})",
               uid,
               gid
        );

        chown(&*self.parameters.path, Some(uid), Some(gid))
            .context("failed to set directory owner and group")?;

        Ok(Action::Created)
    }

    // Recursively (!) delete this directory.
    fn delete(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "deleting directory",
        );

        if metadata.is_dir() {
            fs::remove_dir_all(&*self.parameters.path).context("failed to delete directory")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a directory")
        }

        Ok(Action::Deleted)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub purge: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Relationships {
    requires: Vec<ResourceMetadata>,
    children: Vec<DirectoryChildNode>,
}
