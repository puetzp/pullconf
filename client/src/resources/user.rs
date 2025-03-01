use super::{group, Resource, ResourceResult, ResourceTrait};
use common::{
    resources::user::{Name, Parameters, Password, Relationships, EXPIRY_DATE_FORMAT},
    Action, Ensure, ResourceMetadata, SafePathBuf, TriggerMetadata,
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
    process::{Command, Stdio},
    str::FromStr,
    time::Instant,
};
use time::Date;

const USERADD: &str = "/usr/sbin/useradd";
const USERMOD: &str = "/usr/sbin/usermod";
const PASSWD: &str = "/usr/bin/passwd";
const DELUSER: &str = "/usr/sbin/deluser";
const ID: &str = "/usr/bin/id";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: String,
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
    s.serialize_field("name", &parameters.name)?;
    s.end()
}
impl ResourceTrait for User {
    fn kind(&self) -> &str {
        "user"
    }

    fn display(&self) -> String {
        self.parameters.name.to_string()
    }

    fn id(&self) -> &str {
        &self.id
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

    fn check_prerequisites(&self) -> Result<(), String> {
        fn find(program: &str) -> Result<(), String> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        Ok(())
                    } else {
                        Err(format!(
                            "cannot apply resource as executable `{}` is missing",
                            program
                        ))
                    }
                }
                Err(error) => Err(format!(
                    "cannot apply resource as executable `{}` cannot be accessed: {}",
                    program, error
                )),
            }
        }

        let useradd = find(USERADD);
        let usermod = find(USERMOD);
        let passwd = find(PASSWD);
        let deluser = find(DELUSER);
        let id = find(ID);

        useradd.and(usermod).and(passwd).and(deluser).and(id)
    }
}

impl User {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, order: usize, applied_resources: &HashMap<String, Resource>) {
        let timer = Instant::now();

        self.result.order = order;

        if let Some((action, message)) = self.maybe_return_early(applied_resources) {
            self.result.action = action;
            self.result.message = Some(message);
            return;
        }

        if let Err(error) = self.check_prerequisites() {
            error!("`{}`: {}", self.repr(), error);
            self.result.action = Action::Failed;
            self.result.message = Some(error);
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
                self.result.message = Some(format!("{:#}", error));
            }
        }

        self.result.duration = timer.elapsed().as_millis() as usize;
    }

    /// Apply this resource's configuration.
    pub fn _apply(&self) -> Result<Action, anyhow::Error> {
        match find(&self.parameters.name)? {
            None => match self.parameters.ensure {
                Ensure::Present => self.create(),
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(current_user) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(current_user),
                Ensure::Absent => self.delete(),
            },
        }
    }

    /// Create the user account.
    fn create(&self) -> Result<Action, anyhow::Error> {
        debug!("`{}`: creating user account", self.repr());

        {
            let mut command = Command::new(USERADD);

            command.arg("--create-home");
            command.arg("--home-dir");
            command.arg(self.parameters.home.as_path().as_os_str());

            if self.parameters.system {
                command.arg("--system");
            }

            if let Some(comment) = &self.parameters.comment {
                command.arg("--comment");
                command.arg(comment);
            }

            if let Some(shell) = &self.parameters.shell {
                command.arg("--shell");
                command.arg(shell.as_path().as_os_str());
            }

            if let Password::Unlocked(password) = &self.parameters.password {
                command.arg("--password");
                command.arg(password);
            }

            if group::exists(&self.parameters.group)? {
                command.arg("--no-user-group");
                command.arg("--gid");
                command.arg(self.parameters.group.as_str());
            } else if self.parameters.group == self.parameters.name {
                command.arg("--user-group");
            } else {
                command.arg("--gid");
                command.arg(self.parameters.group.as_str());
            }

            if !self.parameters.groups.is_empty() {
                command.arg("--groups");
                command.arg(
                    self.parameters
                        .groups
                        .iter()
                        .map(|name| name.as_str())
                        .collect::<Vec<&str>>()
                        .as_slice()
                        .join(","),
                );
            }

            command
                .arg(self.parameters.name.as_str())
                .stderr(Stdio::null())
                .stdout(Stdio::null());

            let status = command.status()?;

            debug!("`{}`: executing `{:#?}`", self.repr(), command);

            if !status.success() {
                anyhow::bail!(
                    "failed to create user account, {:?} exited with status {}",
                    command.get_program(),
                    status.code().unwrap()
                );
            }
        }

        Ok(Action::Created)
    }

    /// Update the user account if necessary.
    fn maybe_update(&self, current_user: CurrentUser) -> Result<Action, anyhow::Error> {
        let mut action = Action::default();

        let mut usermod = Command::new(USERMOD);

        match (current_user.comment, &self.parameters.comment) {
            (Some(current), Some(desired)) if current != *desired => {
                usermod.arg("--comment");
                usermod.arg(desired);
            }
            (Some(_), None) => {
                usermod.arg("--comment");
            }
            (None, Some(desired)) => {
                usermod.arg("--comment");
                usermod.arg(desired);
            }
            _ => {}
        }

        if let Some(shell) = &self.parameters.shell {
            if *shell != current_user.shell {
                usermod.arg("--shell");
                usermod.arg(shell.as_path().as_os_str());
            }
        }

        if self.parameters.home != current_user.home {
            usermod.arg("--move-home");
            usermod.arg("--home");
            usermod.arg(self.parameters.home.as_path().as_os_str());
        }

        match (current_user.expiry_date, &self.parameters.expiry_date) {
            (Some(current), Some(desired)) if current != **desired => {
                usermod.arg("--expiredate");
                usermod.arg(desired.format(&EXPIRY_DATE_FORMAT)?);
            }
            (None, Some(desired)) => {
                usermod.arg("--expiredate");
                usermod.arg(desired.format(&EXPIRY_DATE_FORMAT)?);
            }
            (Some(_), None) => {
                usermod.arg("--expiredate");
                usermod.arg("");
            }
            _ => {}
        }

        if current_user.group != self.parameters.group.as_str() {
            usermod.arg("--gid");
            usermod.arg(self.parameters.group.as_str());
        }

        let _groups = self
            .parameters
            .groups
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if current_user.groups != _groups {
            usermod.arg("--groups");
            usermod.arg(_groups.as_slice().join(","));
        }

        let mut passwd = Command::new(PASSWD);

        match (current_user.password, &self.parameters.password) {
            (Password::Unlocked(current_password), Password::Unlocked(password)) => {
                if current_password != *password {
                    passwd.arg("--password");
                    passwd.arg(password.as_str());
                }
            }
            (Password::Unlocked(_), Password::Locked) => {
                usermod.arg("--lock");
            }
            (Password::Locked, Password::Unlocked(password)) => {
                usermod.arg("--unlock");
                passwd.arg("--password");
                passwd.arg(password.as_str());
            }
            _ => {}
        }

        if usermod.get_args().count() != 0 {
            usermod.arg(self.parameters.name.as_str());
            usermod.stderr(Stdio::null());
            usermod.stdout(Stdio::null());

            debug!("`{}`: executing `{:#?}", self.repr(), usermod);

            let status = usermod.status()?;

            if !status.success() {
                anyhow::bail!(
                    "failed to update user account, {:?} exited with status {}",
                    usermod.get_program(),
                    status.code().unwrap()
                );
            }

            action = Action::Changed;
        }

        if passwd.get_args().count() != 0 {
            passwd.arg(self.parameters.name.as_str());
            passwd.stderr(Stdio::null());
            passwd.stdout(Stdio::null());

            debug!("`{}`: executing `{:#?}`", self.repr(), passwd);

            let status = passwd.status()?;

            if !status.success() {
                anyhow::bail!(
                    "failed to set user account password, {:?} exited with status {}",
                    passwd.get_program(),
                    status.code().unwrap()
                );
            }

            action = Action::Changed;
        }

        if action == Action::default() {
            debug!(
                "`{}`: desired user account configuration matches current configuration",
                self.repr(),
            );
        }

        Ok(action)
    }

    /// Delete the user account.
    fn delete(&self) -> Result<Action, anyhow::Error> {
        debug!("`{}`: deleting user account", self.repr());

        let mut command = Command::new(DELUSER);

        command.arg(self.parameters.name.as_str());

        let status = command
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()?;

        if !status.success() {
            anyhow::bail!(
                "failed to delete user account, {:?} exited with status {}",
                command.get_program(),
                status.code().unwrap()
            );
        }

        Ok(Action::Deleted)
    }
}

#[derive(Clone, Debug)]
struct CurrentUser {
    comment: Option<String>,
    shell: SafePathBuf,
    home: SafePathBuf,
    password: Password,
    expiry_date: Option<Date>,
    group: String,
    groups: Vec<String>,
}

/// Try to find this user account in the system. If it exists, return the
/// relevant subset of the current user configuration.
fn find(name: &Name) -> Result<Option<CurrentUser>, anyhow::Error> {
    let (comment, home, shell) = {
        match fs::read_to_string("/etc/passwd")?
            .lines()
            .find(|line| matches!(line.split_once(':'), Some((field, _)) if field == name.as_str()))
        {
            Some(line) => {
                let parts = line.split(':').collect::<Vec<&str>>();

                let comment = match parts.get(4) {
                    Some(value) if !value.is_empty() => Some(value.to_string()),
                    _ => None,
                };

                let home = match parts.get(5) {
                    Some(value) => SafePathBuf::from_str(value)?,
                    None => return Ok(None),
                };

                let shell = match parts.get(6) {
                    Some(value) => SafePathBuf::from_str(value)?,
                    None => return Ok(None),
                };

                (comment, home, shell)
            }
            None => return Ok(None),
        }
    };

    let (password, expiry_date) = {
        match fs::read_to_string("/etc/shadow")?
            .lines()
            .find(|line| matches!(line.split_once(':'), Some((field, _)) if field == name.as_str()))
        {
            Some(line) => {
                let parts = line.split(':').collect::<Vec<&str>>();

                let password = match parts.get(1) {
                    Some(value) => Password::from_str(value)?,
                    None => return Ok(None),
                };

                let expiry_date = match parts.get(7) {
                    Some(value) if !value.is_empty() => {
                        let epoch = time::macros::date!(1970 - 01 - 01);
                        let date = epoch + time::Duration::days(value.parse::<i64>()?);
                        Some(date)
                    }
                    _ => None,
                };

                (password, expiry_date)
            }
            None => return Ok(None),
        }
    };

    let (group, groups) = {
        let mut command = Command::new(ID);

        command.arg("--groups");
        command.arg("--name");
        command.arg(name.as_str());

        let output = command
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .output()?;

        let mut groups = if !output.status.success() {
            anyhow::bail!(
                "failed to query user's groups, {:?} exited with status {}",
                command.get_program(),
                output.status.code().unwrap()
            )
        } else {
            String::from_utf8(output.stdout)?
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        };

        // The first group is the user's primary group.
        let group = groups.remove(0);

        // These are the user's supplementary groups.
        groups.sort();

        (group, groups)
    };

    let user = CurrentUser {
        comment,
        shell,
        home,
        password,
        expiry_date,
        group,
        groups,
    };

    Ok(Some(user))
}
