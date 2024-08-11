use super::{Action, Resource, ResourceTrait};
use crate::util::uid_and_gid;
use anyhow::Context;
use common::{Ensure, FileMode, Groupname, ResourceMetadata, SafePathBuf, Username};
use log::{debug, error, info, warn};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{chown, MetadataExt, PermissionsExt},
};
use ureq::Agent;
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct File {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for File {
    fn kind(&self) -> &str {
        "file"
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

impl File {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(
        &mut self,
        pid: u32,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        applied_resources: &HashMap<Uuid, Resource>,
    ) {
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

        match self._apply(pid, agent, base_url, api_key) {
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
    /// file in the file system change.
    /// Note that some extra parameters need to be passed to this method as the file
    /// content may need to be downloaded from pullconfd.
    pub fn _apply(
        &self,
        pid: u32,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
    ) -> Result<Action, anyhow::Error> {
        let metadata = match fs::metadata(&*self.parameters.path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => anyhow::bail!("failed to query file metadata: {:#}", error),
        };

        match metadata {
            None => match self.parameters.ensure {
                Ensure::Present => {
                    // When some error occurs during file creation it can be safely
                    // deleted again (cleaned up) as it did not exist in the first place.
                    match self.create(pid, agent, base_url, api_key) {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(pid,
                                   resource = self.kind(),
                                   path = self.display();
                                   "deleting file as at least one condition failed"
                            );
                            fs::remove_file(&*self.parameters.path).ok();
                            Err(error)
                        }
                    }
                }
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(metadata) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(pid, agent, base_url, api_key, metadata),
                Ensure::Absent => self.delete(pid, metadata),
            },
        }
    }

    /// Change the file's ownership, mode and content if these parameters differ
    /// from the desired state.
    fn maybe_update(
        &self,
        pid: u32,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        metadata: fs::Metadata,
    ) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "file exists, checking if current and desired states match"
        );

        let mut action = Action::default();

        if !metadata.is_file() {
            anyhow::bail!("failed to update resource as it is not a file")
        }

        let permissions =
            fs::Permissions::from_mode(u32::from_str_radix(&self.parameters.mode, 8)?);

        // Update permissions if necessary.
        if (metadata.permissions().mode() & 0o777) != permissions.mode() {
            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "updating file mode to {}",
                   permissions.mode()
            );

            let handle = fs::File::open(&*self.parameters.path)
                .context("failed to open file in read-only mode")?;

            handle
                .set_permissions(permissions)
                .context("failed to set permissions")?;

            action = Action::Changed;
        }

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        // Update ownership if necessary.
        if metadata.uid() != uid || metadata.gid() != gid {
            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "updating file owner (uid: {}) and group (gid: {})",
                   uid,
                   gid
            );

            chown(&*self.parameters.path, Some(uid), Some(gid))
                .context("failed to set file owner and group")?;

            action = Action::Changed;
        }

        // Compute an etag from the current file contents.
        let etag = {
            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "computing etag (sha256 digest) from current file content",
            );

            let mut bytes = vec![];

            let mut handle = fs::File::open(&*self.parameters.path)
                .context("failed to open file in read-only mode")?;

            handle.read_to_end(&mut bytes)?;

            format!("{:x}", Sha256::digest(bytes))
        };

        // Either download the file content from the server (the etag ensures that
        // the server does not re-send the data when the remote file content does
        // not differ from the current content) or simply write the inline content
        // from the configuration to the file (if etag and checksum differ).
        if let Some(path) = &self.parameters.source {
            let url = base_url.join(&format!("/assets{}", path.display()))?;

            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "downloading file from {}",
                   url
            );

            let response = agent
                .get(url.as_str())
                .set("Accept", "text/plain")
                .set("X-API-KEY", api_key)
                .set("If-None-Match", &etag)
                .call()
                .context("failed to download file contents")?;

            if response.status() != 304 {
                debug!(pid,
                       resource = self.kind(),
                       path = self.display();
                       "remote file content has changed, writing new content to file",
                );

                let mut bytes = vec![];

                response
                    .into_reader()
                    .read_to_end(&mut bytes)
                    .context("failed to write payload to buffer")?;

                let mut handle = fs::OpenOptions::new()
                    .write(true)
                    .open(&*self.parameters.path)
                    .context("failed to open file in write mode")?;

                handle
                    .write_all(&bytes)
                    .context("failed to write payload to file")?;

                action = Action::Changed;
            } else {
                debug!(pid,
                       resource = self.kind(),
                       path = self.display();
                       "remote file content matches current file content",
                );
            }
        } else if let Some(content) = &self.parameters.content {
            if format!("{:x}", Sha256::digest(content.as_bytes())) != etag {
                debug!(pid,
                       resource = self.kind(),
                       path = self.display();
                       "remote file content has changed, writing new content to file",
                );

                fs::write(&*self.parameters.path, content.as_bytes())
                    .context("failed to write inline string to file")?;

                action = Action::Changed;
            } else {
                debug!(pid,
                       resource = self.kind(),
                       path = self.display();
                       "remote file content matches current file content",
                );
            }
        }

        Ok(action)
    }

    /// Create the directory and set ownership, mode and content.
    /// The file content is either downloaded from the server or copied from the
    /// configuration.
    fn create(
        &self,
        pid: u32,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
    ) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "file does no exist, creating file",
        );

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        let mut handle =
            fs::File::create_new(&*self.parameters.path).context("failed to create file")?;

        let permissions =
            fs::Permissions::from_mode(u32::from_str_radix(&self.parameters.mode, 8)?);

        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "setting file mode to {}",
               permissions.mode()
        );

        handle
            .set_permissions(permissions)
            .context("failed to set permissions")?;

        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "setting file owner (uid: {}) and group ({})",
               uid,
               gid
        );

        chown(&*self.parameters.path, Some(uid), Some(gid))
            .context("failed to set file owner and group")?;

        if let Some(path) = &self.parameters.source {
            let mut bytes = vec![];

            let url = base_url.join(&format!("/assets{}", path.display()))?;

            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "downloading file from {}",
                   url
            );

            agent
                .get(url.as_str())
                .set("Accept", "text/plain")
                .set("X-API-KEY", api_key)
                .call()
                .context("failed to download file contents")?
                .into_reader()
                .read_to_end(&mut bytes)
                .context("failed to write payload to buffer")?;

            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "writing content to file",
            );

            handle
                .write_all(&bytes)
                .context("failed to write payload to file")?;
        } else if let Some(content) = &self.parameters.content {
            debug!(pid,
                   resource = self.kind(),
                   path = self.display();
                   "writing content to file",
            );

            handle
                .write_all(content.as_bytes())
                .context("failed to write static content to file")?;
        }

        Ok(Action::Created)
    }

    /// Delete this file.
    fn delete(&self, pid: u32, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!(pid,
               resource = self.kind(),
               path = self.display();
               "deleting file"
        );

        if metadata.is_file() {
            fs::remove_file(&*self.parameters.path).context("failed to delete file")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a file")
        }

        Ok(Action::Deleted)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub mode: FileMode,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub content: Option<String>,
    pub source: Option<SafePathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Relationships {
    requires: Vec<ResourceMetadata>,
}
