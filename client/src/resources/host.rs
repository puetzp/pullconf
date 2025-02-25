use super::{Resource, ResourceResult, ResourceTrait};
use anyhow::Context;
use common::{
    resources::host::{Parameters, Relationships},
    Action, Ensure, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use std::{
    collections::HashMap,
    default::Default,
    fs,
    io::{self, Read},
    time::{Instant, SystemTime},
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Host {
    pub id: Uuid,
    #[serde(serialize_with = "serialize_parameters")]
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default, skip_deserializing)]
    pub result: ResourceResult,
}

fn serialize_parameters<S>(parameters: &Parameters, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("Parameters", 1)?;
    s.serialize_field("ip_address", &parameters.ip_address)?;
    s.end()
}
impl ResourceTrait for Host {
    fn kind(&self) -> &str {
        "host"
    }

    fn display(&self) -> String {
        self.parameters.ip_address.to_string()
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn action(&self) -> Action {
        self.result.action
    }

    fn order(&self) -> usize {
        self.result.order
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
}

impl Host {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, order: usize, applied_resources: &HashMap<Uuid, Resource>) {
        let timer = Instant::now();

        self.result.order = order;

        if let Some((action, message)) = self.maybe_return_early(applied_resources) {
            self.result.action = action;
            self.result.message = Some(message);
            return;
        }

        debug!("`{}`: applying resource", self.repr());

        match self._apply() {
            Ok(action) => {
                info!("`{}`: successfully applied resource", self.repr());

                self.result.action = action;
            }
            Err(error) => {
                error!("`{}`: failed to apply resource: {:#}", self.repr(), error);

                self.result.action = Action::Failed;
            }
        }

        self.result.duration_ms = timer.elapsed().as_millis() as usize;
    }

    /// Apply this resource's configuration.
    pub fn _apply(&self) -> Result<Action, anyhow::Error> {
        match fs::File::open(&self.parameters.target) {
            Ok(mut file) => {
                // If the file is found read its entire contents to a string.
                // Anticipate that users may have used non-utf8 characters to
                // write comments.
                let content = {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;
                    String::from_utf8_lossy(&data).into_owned()
                };

                // Also take note of the last file modification time.
                // This is used in various stages to double-check that the file
                // has not changed while the configuration is applied.
                // Otherwise we would possibly overwrite entries added by users
                // in the meantime.
                let mtime = file.metadata()?.modified()?;

                // Build a vector from the necessary host parameters that form
                // an entry to the hosts file:
                // <ip-address> <hostname> [<alias> ..]
                let parameters = {
                    let mut v = vec![
                        self.parameters.ip_address.to_string(),
                        self.parameters.hostname.to_string(),
                    ];

                    for alias in &self.parameters.aliases {
                        v.push(alias.to_string());
                    }

                    v
                };

                // Introduce a variable to save the state of the current host
                // configuration.
                // `None` means it could not be found at all.
                let mut _match = None;

                for (index, line) in content.lines().enumerate() {
                    let mut columns = line.split_whitespace().peekable();

                    if columns.peek().is_some_and(|column| {
                        parameters
                            .first()
                            .is_some_and(|ip_address| ip_address == column)
                    }) {
                        if columns.eq(parameters.iter().map(|item| item.as_str())) {
                            _match = Some(Match::Full(index));
                            break;
                        } else {
                            _match = Some(Match::Partial(index));
                            break;
                        }
                    }
                }

                match _match {
                    Some(Match::Full(index)) | Some(Match::Partial(index)) => debug!(
                        "`{}`: host was found in target file `{}` at line `{}`",
                        self.repr(),
                        self.parameters.target.display(),
                        index
                    ),
                    _ => {}
                }

                // Apply the resource according to:
                // 1. the desired host state (present/absent) and
                // 2. the current host state (not found/full match/partial match)
                match self.parameters.ensure {
                    Ensure::Absent => match _match {
                        Some(Match::Full(index)) => self.delete(index, mtime, content),
                        Some(Match::Partial(index)) => self.delete(index, mtime, content),
                        None => {
                            debug!(
                                "`{}`: current host state matches the desired state",
                                self.repr(),
                            );

                            Ok(Action::default())
                        }
                    },
                    Ensure::Present => match _match {
                        Some(Match::Full(_)) => {
                            debug!(
                                "`{}`: current host state matches the desired state",
                                self.repr()
                            );

                            Ok(Action::default())
                        }
                        Some(Match::Partial(index)) => {
                            self.update(index, mtime, content, parameters)
                        }
                        None => self.create(mtime, content, parameters),
                    },
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                debug!(
                    "`{}`: skipping host as target file `{}` does not exist",
                    self.repr(),
                    self.parameters.target.display()
                );

                Ok(Action::Skipped)
            }
            Err(error) => anyhow::bail!("failed to open target file: {:#}", error),
        }
    }

    /// Update the host in the target file.
    /// This effectively replaces the target file with the updated host in it.
    fn update(
        &self,
        index: usize,
        mtime: SystemTime,
        content: String,
        parameters: Vec<String>,
    ) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: trying to update host in target file `{}`",
            self.repr(),
            self.parameters.target.display()
        );

        let mut new_content = String::new();

        for (new_index, line) in content.lines().enumerate() {
            if new_index == index {
                new_content.push_str(&parameters.as_slice().join("\t"));
                new_content.push('\n');
            } else {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        debug!(
            "`{}`: writing replacement file for target file `{}` with an updated version of this host",self.repr(),
            self.parameters.target.display()
        );

        fs::write("/tmp/hosts.pullconf", new_content.as_bytes())?;

        if fs::metadata(&self.parameters.target)
            .context("failed to query target file metadata")?
            .modified()
            .is_ok_and(|_mtime| _mtime == mtime)
        {
            debug!(
                "`{}`: renaming replacement file to original target file `{}`",
                self.repr(),
                self.parameters.target.display()
            );

            fs::rename("/tmp/hosts.pullconf", &self.parameters.target)
                .context("failed to replace target file")?;
        } else {
            anyhow::bail!("target file changed before replacement file could be renamed");
        }

        Ok(Action::Changed)
    }

    /// Add the host to the target file.
    /// This effectively replaces the target file with the host appended to it.
    fn create(
        &self,
        mtime: SystemTime,
        mut content: String,
        parameters: Vec<String>,
    ) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: trying to append host to target file `{}`",
            self.repr(),
            self.parameters.target.display()
        );

        if !content.as_bytes().last().is_some_and(|byte| *byte == 0xA) {
            content.push('\n');
        }

        content.push_str(&parameters.as_slice().join("\t"));
        content.push('\n');

        debug!(
            "`{}`: writing replacement file for target file `{}` with this host appended",
            self.repr(),
            self.parameters.target.display()
        );

        fs::write("/tmp/hosts.pullconf", content.as_bytes())?;

        if fs::metadata(&self.parameters.target)
            .context("failed to query target file metadata")?
            .modified()
            .is_ok_and(|_mtime| _mtime == mtime)
        {
            debug!(
                "`{}`: renaming replacement file to original target file `{}`",
                self.repr(),
                self.parameters.target.display()
            );

            fs::rename("/tmp/hosts.pullconf", &self.parameters.target)
                .context("failed to replace target file")?;
        } else {
            anyhow::bail!("target file changed before replacement file could be renamed");
        }

        Ok(Action::Created)
    }

    /// Delete the host from the target file.
    /// This effectively replaces the target file with the host removed.
    fn delete(
        &self,
        index: usize,
        mtime: SystemTime,
        content: String,
    ) -> Result<Action, anyhow::Error> {
        debug!(
            "`{}`: trying to delete host from target file `{}`",
            self.repr(),
            self.parameters.target.display()
        );

        let mut new_content = String::new();

        for (_index, line) in content.lines().enumerate() {
            if _index != index {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        debug!(
            "`{}`: writing replacement file for target file `{}` without this host",
            self.repr(),
            self.parameters.target.display()
        );

        fs::write("/tmp/hosts.pullconf", new_content.as_bytes())?;

        if fs::metadata(&self.parameters.target)
            .context("failed to query target file metadata")?
            .modified()
            .is_ok_and(|_mtime| _mtime == mtime)
        {
            debug!(
                "`{}`: renaming replacement file to original target file `{}`",
                self.repr(),
                self.parameters.target.display()
            );

            fs::rename("/tmp/hosts.pullconf", &self.parameters.target)
                .context("failed to replace target file")?;
        } else {
            anyhow::bail!("target file changed before replacement file could be renamed");
        }

        Ok(Action::Deleted)
    }
}

/// This enum is used during configuration to indicate that the host
/// could be found in the target file at the given index with a full
/// or partial match.
enum Match {
    Full(usize),
    Partial(usize),
}
