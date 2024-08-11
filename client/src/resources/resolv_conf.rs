use super::{Action, Resource, ResourceTrait};
use anyhow::Context;
use common::{
    resources::resolv_conf::{ResolverOption, SortlistPair},
    Ensure, Hostname, ResourceMetadata,
};
use log::{debug, error, info, warn};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    net::IpAddr,
    path::PathBuf,
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct ResolvConf {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for ResolvConf {
    fn kind(&self) -> &str {
        "resolv.conf"
    }

    fn display(&self) -> String {
        self.parameters.target.display().to_string()
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

            warn!(
                pid,
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

            warn!(
                pid,
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

impl ResolvConf {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, pid: u32, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(pid, applied_resources) {
            self.action = action;
            return;
        }

        debug!(
            pid,
            resource = self.kind(),
            path = self.display();
            "applying {}",
            self.repr()
        );

        match self._apply(pid) {
            Ok(action) => {
                info!(
                    pid,
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

                error!(
                    pid,
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

    /// Apply this resource's configuration.
    pub fn _apply(&self, pid: u32) -> Result<Action, anyhow::Error> {
        match fs::File::open(&self.parameters.target) {
            Ok(mut file) => {
                // If the file is found compute its current checksum.
                // This is used to determine if any modifications have been
                // made that do not match the desired state of the resource.
                let current_checksum = {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;
                    format!("{:x}", Sha256::digest(data))
                };

                // Build the expected file content from the resource
                // parameters. Then also compute its checksum.
                let content = {
                    let mut content = String::new();

                    for nameserver in &self.parameters.nameservers {
                        let line = format!("nameserver {}\n", nameserver);
                        content.push_str(&line);
                    }

                    if !self.parameters.search.is_empty() {
                        let search = {
                            let as_str = self
                                .parameters
                                .search
                                .iter()
                                .map(|item| item.as_str())
                                .collect::<Vec<&str>>();
                            format!("search {}\n", as_str.as_slice().join(" "))
                        };

                        content.push_str(&search);
                    }

                    if !self.parameters.sortlist.is_empty() {
                        let sortlist = {
                            let as_str = self
                                .parameters
                                .sortlist
                                .iter()
                                .map(|item| item.as_str())
                                .collect::<Vec<&str>>();

                            format!("sortlist {}\n", as_str.as_slice().join(" "))
                        };

                        content.push_str(&sortlist);
                    }

                    if !self.parameters.options.is_empty() {
                        let options = {
                            let as_str = self
                                .parameters
                                .options
                                .iter()
                                .map(|item| item.as_str())
                                .collect::<Vec<&str>>();

                            format!("options {}\n", as_str.as_slice().join(" "))
                        };

                        content.push_str(&options);
                    }

                    content
                };

                let checksum = format!("{:x}", Sha256::digest(content.as_bytes()));

                let _match = current_checksum == checksum;

                // Apply the resource based on the result of the checksum comparison
                // and the desired resource state.
                match self.parameters.ensure {
                    Ensure::Absent => self.clear(pid),
                    Ensure::Present => {
                        if _match {
                            Ok(Action::Unchanged)
                        } else {
                            self.populate(pid, content)
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                debug!(
                    pid,
                    resource = self.kind(),
                    path = self.display();
                    "skipping resource as target file {} does not exist",
                    self.parameters.target.display()
                );

                Ok(Action::Skipped)
            }
            Err(error) => anyhow::bail!("failed to open target file: {:#}", error),
        }
    }

    /// Replace the current target file contents with the desired contents.
    fn populate(&self, pid: u32, content: String) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            path = self.display();
            "populating target file {}",
            self.parameters.target.display()
        );

        debug!(
            pid,
            resource = self.kind(),
            path = self.display();
            "writing replacement file for target file {} with the desired contents",
            self.parameters.target.display()
        );

        fs::write("/tmp/resolv.conf.pullconf", content.as_bytes())?;

        if let Err(error) = fs::metadata(&self.parameters.target) {
            anyhow::bail!("target file cannot be accessed: {}", error);
        } else {
            debug!(
                pid,
                resource = self.kind(),
                path = self.display();
                "renaming replacement file to original target file {}",
                self.parameters.target.display()
            );

            fs::rename("/tmp/resolv.conf.pullconf", &self.parameters.target)
                .context("failed to replace target file")?;
        }

        Ok(Action::Created)
    }

    /// Clear the target file.
    fn clear(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            path = self.display();
            "truncating target file {}",
            self.parameters.target.display()
        );

        let _ = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.parameters.target)
            .context("failed to truncate target file")?;

        Ok(Action::Deleted)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    pub nameservers: Vec<IpAddr>,
    pub search: Vec<Hostname>,
    pub sortlist: Vec<SortlistPair>,
    pub options: Vec<ResolverOption>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Relationships {
    requires: Vec<ResourceMetadata>,
}
