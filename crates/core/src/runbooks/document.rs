mod expect;
mod on_failure;
mod retry;
mod step;

pub use expect::Expect;
pub use on_failure::OnFailure;
pub use retry::Retry;
pub use step::{Step, StepAction};

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_proto::{OneLine, RunPhase};

use super::param::{ParamName, ParamSpec};

use crate::CoreError;

const MOST_PARAMS: usize = 50;
const MOST_STEPS: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookDocument {
    name: OneLine,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    params: Vec<ParamSpec>,

    steps: Vec<Step>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    rollback: Vec<Step>,
}

impl RunbookDocument {
    pub(crate) fn new(name: OneLine, params: Vec<ParamSpec>, steps: Vec<Step>) -> Self {
        Self {
            name,
            description: String::new(),
            params,
            steps,
            rollback: Vec::new(),
        }
    }

    pub(crate) fn from_toml(text: &str) -> Result<Self, CoreError> {
        let document: Self = toml::from_str(text)
            .map_err(|error| CoreError::Malformed(error.message().to_owned()))?;

        document.check()?;
        Ok(document)
    }

    pub(crate) fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub(crate) fn name(&self) -> &OneLine {
        &self.name
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    pub(crate) fn params(&self) -> &[ParamSpec] {
        &self.params
    }

    pub(crate) fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub(crate) fn rollback(&self) -> &[Step] {
        &self.rollback
    }

    pub(crate) fn every_step(&self) -> impl Iterator<Item = &Step> {
        self.steps.iter().chain(&self.rollback)
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        self.check_params()?;
        self.check_steps()?;
        self.check_rollback()?;

        self.check_params_used()
    }

    fn check_params(&self) -> Result<(), CoreError> {
        if self.params.len() > MOST_PARAMS {
            return Err(CoreError::TooManyParams(MOST_PARAMS));
        }

        let mut seen = BTreeSet::new();

        for spec in &self.params {
            spec.check()?;

            if !seen.insert(spec.name()) {
                return Err(CoreError::DeclaredTwice(spec.name().clone()));
            }
        }

        Ok(())
    }

    fn check_steps(&self) -> Result<(), CoreError> {
        if self.steps.is_empty() || self.steps.len() > MOST_STEPS {
            return Err(CoreError::StepCount(MOST_STEPS));
        }

        for (number, step) in (1..).zip(&self.steps) {
            step.check()
                .map_err(|error| error.in_step(RunPhase::Steps, number))?;
        }

        let rolls_back = self
            .steps
            .iter()
            .any(|step| step.on_failure() == OnFailure::Rollback);

        if rolls_back && self.rollback.is_empty() {
            return Err(CoreError::NoRollback);
        }

        Ok(())
    }

    fn check_rollback(&self) -> Result<(), CoreError> {
        if self.rollback.len() > MOST_STEPS {
            return Err(CoreError::RollbackCount(MOST_STEPS));
        }

        for (number, step) in (1..).zip(&self.rollback) {
            Self::check_rollback_step(step)
                .map_err(|error| error.in_step(RunPhase::Rollback, number))?;
        }

        Ok(())
    }

    fn check_rollback_step(step: &Step) -> Result<(), CoreError> {
        step.check()?;

        if step.on_failure() != OnFailure::Stop {
            return Err(CoreError::RollbackOnFailure);
        }

        Ok(())
    }

    fn check_params_used(&self) -> Result<(), CoreError> {
        let declared: BTreeSet<&ParamName> = self.params.iter().map(ParamSpec::name).collect();

        let undeclared = self
            .every_step()
            .flat_map(Step::params_used)
            .find(|name| !declared.contains(name));

        match undeclared {
            Some(name) => Err(CoreError::UndeclaredParam(name)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{OnFailure, RunbookDocument};

    const RESTART: &str = r#"
name = "Restart API"
description = "Restart the orders API and check its health."

[[params]]
name = "host"
kind = "target"
default = "@orders/api-1"

[[params]]
name = "service"
kind = "word"
default = "orders-api"

[[steps]]
title = "Restart {service}"
action = "ssh.exec"
target = "{host}"
params = { command = "sudo systemctl restart {service}" }

[[steps]]
title = "Check health"
action = "ssh.exec"
target = "{host}"
params = { command = "curl -fsS http://127.0.0.1:8080/health" }
expect = { exit = 0, contains = "ok" }
retry = { attempts = 5, every_seconds = 3 }
on_failure = "rollback"

[[rollback]]
title = "Start the previous release"
action = "ssh.exec"
target = "{host}"
params = { command = "sudo systemctl start {service}-previous" }
"#;

    fn restart() -> RunbookDocument {
        RunbookDocument::from_toml(RESTART).unwrap()
    }

    fn broken(from: &str, to: &str) -> String {
        assert!(RESTART.contains(from), "{from}");

        RunbookDocument::from_toml(&RESTART.replacen(from, to, 1))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn the_example_runbook_reads() {
        let runbook = restart();

        assert_eq!(runbook.name().as_str(), "Restart API");
        assert_eq!(runbook.params().len(), 2);
        assert_eq!(runbook.steps()[1].on_failure(), OnFailure::Rollback);
        assert_eq!(runbook.steps()[1].retry().attempts(), 5);
        assert_eq!(
            runbook.steps()[0].params()["command"],
            json!("sudo systemctl restart {service}")
        );
        assert_eq!(runbook.rollback().len(), 1);
    }

    #[test]
    fn a_runbook_survives_writing_and_reading_back() {
        let runbook = restart();

        let written = runbook.to_toml().unwrap();

        assert_eq!(RunbookDocument::from_toml(&written).unwrap(), runbook);
    }

    #[test]
    fn unknown_fields_are_refused() {
        let error = RunbookDocument::from_toml(&format!("{RESTART}\nauthor = \"ann\"\n"))
            .unwrap_err()
            .to_string();

        assert!(error.contains("author"), "{error}");
    }

    #[test]
    fn every_placeholder_names_a_declared_param() {
        let error = broken("restart {service}\" }", "restart {unit}\" }");

        assert!(error.starts_with("{unit} names no param"), "{error}");
    }

    #[test]
    fn rolling_back_needs_rollback_steps() {
        let without = &RESTART[..RESTART.find("[[rollback]]").unwrap()];

        let error = RunbookDocument::from_toml(without).unwrap_err().to_string();

        assert!(error.contains("no rollback steps"), "{error}");
    }

    #[test]
    fn rollback_steps_cannot_choose_what_happens_on_failure() {
        let error = broken(
            "{service}-previous\" }",
            "{service}-previous\" }\non_failure = \"continue\"",
        );

        assert!(error.starts_with("rollback step 1"), "{error}");
    }

    #[test]
    fn a_runbook_cannot_run_runbooks() {
        let error = broken("action = \"ssh.exec\"", "action = \"runbook.run\"");

        assert_eq!(error, "runbook.run cannot be a step of a runbook");
    }

    #[test]
    fn params_are_declared_once() {
        let error = broken("name = \"service\"", "name = \"host\"");

        assert_eq!(error, "param host is declared twice");
    }

    #[test]
    fn retries_stay_within_limits() {
        let error = broken("attempts = 5", "attempts = 50");

        assert_eq!(error, "step 2: retry attempts must be 1 to 20, got 50");
    }

    #[test]
    fn a_runbook_has_steps_and_a_name() {
        let only_name = "name = \"Nothing\"\nsteps = []\n";

        assert!(
            RunbookDocument::from_toml(only_name)
                .unwrap_err()
                .to_string()
                .contains("1 to 100 steps")
        );
        assert!(broken("Restart API", "two\\nlines").contains("on one line"));
        assert!(broken("Check health", " ").contains("on one line"));
    }
}
