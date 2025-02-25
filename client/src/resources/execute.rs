use super::{Resource, ResourceResult, ResourceTrait};
use anyhow::Context;
use common::{
    resources::execute::{Parameters, Relationships},
    Action, Ensure, ResourceMetadata, TriggerMetadata,
};
use log::{debug, error, info};
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use std::{collections::HashMap, process::Command, time::Instant};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Execute {
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

impl ResourceTrait for Execute {
    fn kind(&self) -> &str {
        "execute"
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

    fn maybe_return_early(
        &mut self,
        applied_resources: &HashMap<String, Resource>,
    ) -> Option<(Action, String)> {
        if let Some(dependency) = self.find_failed_dependency(applied_resources) {
            let message = format!(
                "skipping resource as dependency `{}` has failed to apply",
                dependency.repr()
            );
            log::warn!("`{}`: {}", self.repr(), message);
            return Some((Action::Skipped, message));
        }

        if let Some(dependency) = self.find_skipped_dependency(applied_resources) {
            let message = format!(
                "skipping resource as dependency `{}` has been skipped",
                dependency.repr()
            );
            log::warn!("`{}`: {}", self.repr(), message);
            return Some((Action::Skipped, message));
        }

        None
    }
}

impl Execute {
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

        // Return early in the special case that this resource only
        // applies when triggered by another resource. In this case
        // this resource depends on the triggering resource and the
        // triggering resource in turn contains a reference to the
        // `execute` resource. When the triggering resource was
        // successfully applied, the trigger is activated.
        // Note that only a single triggering resource must be applied
        // successfully in order for the triggered `execute` resource
        // to be applied as well.
        // On the other hand, the `execute` resource should not be
        // applied when no triggering resource was successfully
        // applied earlier.
        if self.parameters.passive
            && !self.predecessors().iter().any(|predecessor| {
                applied_resources
                    .get(&predecessor.id)
                    .is_some_and(|resource| {
                        resource.triggers().iter().any(|trigger| {
                            trigger.id() == self.id() && trigger.when().contains(&resource.action())
                        })
                    })
            })
        {
            self.result.action = Action::Unchanged;
            return;
        }

        if let Some(program) = self.parameters.unless.first() {
            let mut command = Command::new(program);
            command.args(&self.parameters.unless[1..]);

            for env in &self.parameters.environment {
                command.env(
                    &env.name,
                    env.value.as_ref().map(|v| v.as_str()).unwrap_or_default(),
                );
            }

            let output = match command.output() {
                Ok(output) => output,
                Err(error) => {
                    error!("`{}`: failed to apply resource: {:#}", self.repr(), error);
                    self.result.action = Action::Failed;
                    return;
                }
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            debug!(
                "`{}`: `unless` command {:?} exited with {}: \"{}\"",
                self.repr(),
                command.get_program(),
                output.status,
                stdout.is_empty().then_some(stderr).unwrap_or(stdout)
            );

            if output.status.success() {
                self.result.action = Action::Unchanged;
                return;
            }
        }

        debug!("`{}`: applying resource", self.repr(),);

        match self._apply() {
            Ok(action) => {
                info!("`{}`: successfully applied resource", self.repr(),);

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
        match self.parameters.ensure {
            Ensure::Present => {
                let program = self.parameters.command.first().ok_or(anyhow::anyhow!(
                    "command must contain at least the name of a program to run"
                ))?;

                let mut command = Command::new(program);
                command.args(&self.parameters.command[1..]);

                for env in &self.parameters.environment {
                    command.env(
                        &env.name,
                        env.value.as_ref().map(|v| v.as_str()).unwrap_or_default(),
                    );
                }

                debug!(
                    "`{}`: executing `{:#?}` with environment variables {:?}",
                    self.repr(),
                    command,
                    command.get_envs()
                );

                let output = command.output().context(format!(
                    "failed to execute command {:?}",
                    command.get_program()
                ))?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    debug!(
                        "`{}`: command {:?} exited with {}: \"{}\"",
                        self.repr(),
                        command.get_program(),
                        output.status,
                        stdout.is_empty().then_some(stderr).unwrap_or(stdout)
                    );

                    Ok(Action::Changed)
                } else {
                    anyhow::bail!(
                        "failed to execute command, {:?} exited with {}: \"{}\"",
                        command.get_program(),
                        output.status,
                        stdout.is_empty().then_some(stderr).unwrap_or(stdout)
                    );
                }
            }
            Ensure::Absent => Ok(Action::Unchanged),
        }
    }
}
