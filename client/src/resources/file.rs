use super::{Error, Resource, ResourceTrait};
use crate::util::uid_and_gid;
use anyhow::Context;
use common::{
    resources::file::{Content, Parameters, Relationships},
    Action, Ensure, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    os::unix::fs::{chown, MetadataExt, PermissionsExt},
    process::Command,
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

impl File {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(
        &mut self,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        applied_resources: &HashMap<Uuid, Resource>,
    ) {
        if let Some(action) = self.maybe_return_early(applied_resources) {
            self.action = action;
            return;
        }

        debug!("`{}`: applying resource", self.repr(),);

        match self._apply(agent, base_url, api_key) {
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
    /// file in the file system change.
    /// Note that some extra parameters need to be passed to this method as the file
    /// content may need to be downloaded from pullconfd.
    pub fn _apply(
        &self,
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
                    match self.create(agent, base_url, api_key) {
                        Ok(action) => Ok(action),
                        Err(error) => {
                            debug!(
                                "`{}`: deleting file `{}` as at least one condition failed",
                                self.repr(),
                                self.parameters.path.display()
                            );
                            fs::remove_file(&*self.parameters.path).ok();
                            Err(error)
                        }
                    }
                }
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(metadata) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(agent, base_url, api_key, metadata),
                Ensure::Absent => self.delete(metadata),
            },
        }
    }

    /// Change the file's ownership, mode and content if these parameters differ
    /// from the desired state.
    fn maybe_update(
        &self,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
        metadata: fs::Metadata,
    ) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: file exists, checking if current and desired states match",
            self.repr()
        );

        let mut action = Action::default();

        if !metadata.is_file() {
            anyhow::bail!("failed to update resource as it is not a file")
        }

        let permissions =
            fs::Permissions::from_mode(u32::from_str_radix(&self.parameters.mode, 8)?);

        // Update permissions if necessary.
        if (metadata.permissions().mode() & 0o777) != permissions.mode() {
            debug!(
                "`{}`: updating file mode to `{}`",
                self.repr(),
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
            debug!(
                "`{}`: updating file owner (uid: `{}`) and group (gid: `{}`)",
                self.repr(),
                uid,
                gid
            );

            chown(&*self.parameters.path, Some(uid), Some(gid))
                .context("failed to set file owner and group")?;

            action = Action::Changed;
        }

        // Compute an etag from the current file contents.
        let etag = {
            debug!(
                "`{}`: computing etag (sha256 digest) from current file content",
                self.repr()
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

            debug!("`{}`: downloading file from `{}`", self.repr(), url);

            match agent
                .get(url.as_str())
                .header("Accept", "text/plain")
                .header("X-API-KEY", api_key)
                .header("If-None-Match", &etag)
                .call()
            {
                Ok(mut response) => {
                    let status = response.status();

                    if status == 304 {
                        debug!(
                            "`{}`: remote file content matches current file content",
                            self.repr(),
                        );
                    } else if status == 200 {
                        debug!(
                            "`{}`: remote file content has changed, writing new content to file",
                            self.repr()
                        );

                        let bytes = response
                            .body_mut()
                            .read_to_vec()
                            .context("failed to write payload to buffer")?;

                        let mut handle = fs::OpenOptions::new()
                            .write(true)
                            .open(&*self.parameters.path)
                            .context("failed to open file in write mode")?;

                        handle
                            .write_all(&bytes)
                            .context("failed to write payload to file")?;

                        action = Action::Changed;
                    } else if status.is_client_error() || status.is_server_error() {
                        if let Some(_content_type) = response
                            .body()
                            .mime_type()
                            .filter(|value| *value == "application/json")
                        {
                            let error = response
                                .body_mut()
                                .read_json::<Error>()
                                .context(format!("failed to deserialize error response"))?;

                            anyhow::bail!(
                                "pullconfd failed to process the request: {}, {}",
                                error.title,
                                error.detail
                            );
                        } else {
                            let error = response
                                .body_mut()
                                .read_to_string()
                                .context("failed to deserialize error response")?;

                            anyhow::bail!("server failed to process the request: {}", error);
                        }
                    } else {
                        anyhow::bail!("received unexpected status from server: `{}`", status);
                    }
                }
                Err(error) => anyhow::bail!("failed to download file content: {}", error),
            }
        } else if let Some(content) = &self.parameters.content {
            let content = self.maybe_replace_placeholders(content)?;

            if format!("{:x}", Sha256::digest(content.as_bytes())) != etag {
                debug!(
                    "`{}`: remote file content has changed, writing new content to file",
                    self.repr()
                );

                fs::write(&*self.parameters.path, content.as_bytes())
                    .context("failed to write inline string to file")?;

                action = Action::Changed;
            } else {
                debug!(
                    "`{}`: remote file content matches current file content",
                    self.repr()
                );
            }
        }

        Ok(action)
    }

    /// Create the file and set ownership, mode and content.
    /// The file content is either downloaded from the server or copied from the
    /// configuration.
    fn create(
        &self,
        agent: &Agent,
        base_url: &Url,
        api_key: &str,
    ) -> Result<Action, anyhow::Error> {
        debug!("`{}`: file does no exist, creating file", self.repr(),);

        let (uid, gid) = uid_and_gid(&self.parameters.owner, &self.parameters.group)?;

        let mut handle =
            fs::File::create_new(&*self.parameters.path).context("failed to create file")?;

        let permissions =
            fs::Permissions::from_mode(u32::from_str_radix(&self.parameters.mode, 8)?);

        debug!(
            "`{}`: setting file mode to `{}`",
            self.repr(),
            permissions.mode()
        );

        handle
            .set_permissions(permissions)
            .context("failed to set permissions")?;

        debug!(
            "`{}`: setting file owner (uid: `{}`) and group (gid: `{}`)",
            self.repr(),
            uid,
            gid
        );

        chown(&*self.parameters.path, Some(uid), Some(gid))
            .context("failed to set file owner and group")?;

        if let Some(path) = &self.parameters.source {
            let url = base_url.join(&format!("/assets{}", path.display()))?;

            debug!("`{}`: downloading file from `{}`", self.repr(), url);

            match agent
                .get(url.as_str())
                .header("Accept", "text/plain")
                .header("X-API-KEY", api_key)
                .call()
            {
                Ok(mut response) => {
                    let status = response.status();

                    if status == 200 {
                        debug!("`{}`: writing content to file", self.repr());

                        let bytes = response
                            .body_mut()
                            .read_to_vec()
                            .context("failed to write payload to buffer")?;

                        handle
                            .write_all(&bytes)
                            .context("failed to write payload to file")?;
                    } else if status.is_client_error() || status.is_server_error() {
                        if let Some(_content_type) = response
                            .body()
                            .mime_type()
                            .filter(|value| *value == "application/json")
                        {
                            let error = response
                                .body_mut()
                                .read_json::<Error>()
                                .context(format!("failed to deserialize error response"))?;

                            anyhow::bail!(
                                "pullconfd failed to process the request: {}, {}",
                                error.title,
                                error.detail
                            );
                        } else {
                            let error = response
                                .body_mut()
                                .read_to_string()
                                .context("failed to deserialize error response")?;

                            anyhow::bail!("server failed to process the request: {}", error);
                        }
                    } else {
                        anyhow::bail!("received unexpected status from server: `{}`", status);
                    }
                }
                Err(error) => anyhow::bail!("failed to download file content: {}", error),
            }
        } else if let Some(content) = &self.parameters.content {
            let content = self.maybe_replace_placeholders(content)?;

            debug!("`{}`: writing content to file", self.repr(),);

            handle
                .write_all(content.as_bytes())
                .context("failed to write static content to file")?;
        }

        Ok(Action::Created)
    }

    /// Delete this file.
    fn delete(&self, metadata: fs::Metadata) -> Result<Action, anyhow::Error> {
        debug!("`{}`: deleting file", self.repr());

        if metadata.is_file() {
            fs::remove_file(&*self.parameters.path).context("failed to delete file")?
        } else {
            anyhow::bail!("failed to delete resource as it is not a file")
        }

        Ok(Action::Deleted)
    }

    fn maybe_replace_placeholders(&self, content: &Content) -> Result<String, anyhow::Error> {
        let mut _content = content.value.clone();

        if content.replace.is_empty() {
            return Ok(_content);
        }

        for item in &content.replace {
            let program = item.command.first().ok_or(anyhow::anyhow!(
                "command for content replacement must contain at least the name of a program"
            ))?;

            let mut command = Command::new(program);
            command.args(&item.command[1..]);

            for env in &item.environment {
                command.env(
                    &env.name,
                    env.value.as_ref().map(|v| v.as_str()).unwrap_or_default(),
                );
            }

            debug!(
                "`{}`: executing {:?} with args {:?} and environment variables {:?}",
                self.repr(),
                command.get_program(),
                command.get_args(),
                command.get_envs()
            );

            let output = command
                .output()
                .context("failed to execute command for content replacement")?;

            let stdout = String::from_utf8(output.stdout)?;

            if output.status.success() {
                _content = _content.replace(&item.placeholder, &stdout);
            } else {
                anyhow::bail!(
                    "failed to execute command for content replacement, {:?} exited with {}: {}",
                    command.get_program(),
                    output.status,
                    stdout
                );
            }
        }

        Ok(_content)
    }
}
