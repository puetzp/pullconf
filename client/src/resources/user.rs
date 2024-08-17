use super::{group, Action, Resource, ResourceTrait};
use common::{
    resources::user::{Name, Parameters, Password, Relationships, EXPIRY_DATE_FORMAT},
    Ensure, ResourceMetadata, SafePathBuf,
};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    collections::HashMap,
    default::Default,
    fs,
    process::{Command, Stdio},
    str::FromStr,
};
use time::Date;
use uuid::Uuid;

const USERADD: &str = "/usr/sbin/useradd";
const USERMOD: &str = "/usr/sbin/usermod";
const PASSWD: &str = "/usr/bin/passwd";
const DELUSER: &str = "/usr/sbin/deluser";
const ID: &str = "/usr/bin/id";

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub parameters: Parameters,
    pub relationships: Relationships,
    #[serde(default)]
    pub action: Action,
}

impl ResourceTrait for User {
    fn kind(&self) -> &str {
        "user"
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

    fn check_prerequisites(&self, pid: u32) -> Option<Action> {
        fn find(user: &User, pid: u32, program: &str) -> Option<Action> {
            match fs::metadata(program) {
                Ok(metadata) => {
                    if metadata.is_file() {
                        None
                    } else {
                        let action = Action::Failed;

                        error!(
                            pid,
                            resource = user.kind(),
                            name = user.display(),
                            result:% = action;
                            "cannot apply {} as executable `{}` is missing",
                            user.repr(),
                            program
                        );

                        Some(action)
                    }
                }
                Err(error) => {
                    let action = Action::Failed;

                    error!(
                        pid,
                        resource = user.kind(),
                        name = user.display(),
                        result:% = action;
                        "cannot apply {} as executable `{}` cannot be accessed: {}",
                        user.repr(),
                        program,
                        error
                    );

                    Some(action)
                }
            }
        }

        let useradd = find(self, pid, USERADD);
        let usermod = find(self, pid, USERMOD);
        let passwd = find(self, pid, PASSWD);
        let deluser = find(self, pid, DELUSER);
        let id = find(self, pid, ID);

        useradd.or(usermod).or(passwd).or(deluser).or(id)
    }
}

impl User {
    /// A wrapper around the actual apply function. This ensure that some
    /// meaningful log messages are printed and pre-checks are done.
    pub fn apply(&mut self, pid: u32, applied_resources: &HashMap<Uuid, Resource>) {
        if let Some(action) = self.maybe_return_early(pid, applied_resources) {
            self.action = action;
            return;
        }

        if let Some(action) = self.check_prerequisites(pid) {
            self.action = action;
            return;
        }

        debug!(pid,
               resource = self.kind(),
               name = self.display();
               "applying {}",
               self.repr()
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
        match find(&self.parameters.name)? {
            None => match self.parameters.ensure {
                Ensure::Present => self.create(pid),
                Ensure::Absent => Ok(Action::Unchanged),
            },
            Some(current_user) => match self.parameters.ensure {
                Ensure::Present => self.maybe_update(pid, current_user),
                Ensure::Absent => self.delete(pid),
            },
        }
    }

    /// Create the user account.
    fn create(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "creating user account"
        );

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

            debug!(
                pid,
                resource = self.kind(),
                name = self.display();
                "executing {:?} with args {:?}",
                command.get_program(),
                command.get_args()
            );

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
    fn maybe_update(&self, pid: u32, current_user: CurrentUser) -> Result<Action, anyhow::Error> {
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
            (Some(current), Some(desired)) if current != *desired => {
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

            debug!(
                pid,
                resource = self.kind(),
                name = self.display();
                "executing {:?} with args {:?}",
                usermod.get_program(),
                usermod.get_args()
            );

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

            debug!(
                pid,
                resource = self.kind(),
                name = self.display();
                "executing {:?} with args {:?}",
                passwd.get_program(),
                passwd.get_args()
            );

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
                pid,
                resource = self.kind(),
                name = self.display();
                "desired user account configuration matches current configuration",
            );
        }

        Ok(action)
    }

    /// Delete the user account.
    fn delete(&self, pid: u32) -> Result<Action, anyhow::Error> {
        debug!(
            pid,
            resource = self.kind(),
            name = self.display();
            "deleting user account"
        );

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
