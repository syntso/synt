use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{OnFailure, Step};
use crate::runbooks::param::ParamSpec;

const LONGEST_NAME: usize = 200;
const MOST_PARAMS: usize = 50;
const MOST_STEPS: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Runbook {
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamSpec>,

    pub steps: Vec<Step>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollback: Vec<Step>,
}

impl Runbook {
    pub(crate) fn from_toml(text: &str) -> Result<Self, String> {
        let runbook: Self = toml::from_str(text).map_err(|error| error.message().to_owned())?;

        runbook.check()?;
        Ok(runbook)
    }

    pub(crate) fn to_toml(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|error| error.to_string())
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        self.check_name()?;
        self.check_params()?;
        self.check_steps()?;
        self.check_rollback()?;

        self.check_params_used()
    }

    fn check_name(&self) -> Result<(), String> {
        let fits = !self.name.trim().is_empty() && self.name.len() <= LONGEST_NAME;

        if !fits || self.name.contains(char::is_control) {
            return Err(format!(
                "name must be 1 to {LONGEST_NAME} characters on one line"
            ));
        }

        Ok(())
    }

    fn check_params(&self) -> Result<(), String> {
        if self.params.len() > MOST_PARAMS {
            return Err(format!("a runbook has at most {MOST_PARAMS} params"));
        }

        let mut seen = BTreeSet::new();

        for spec in &self.params {
            spec.check()?;

            if !seen.insert(spec.name.as_str()) {
                return Err(format!("param {} is declared twice", spec.name));
            }
        }

        Ok(())
    }

    fn check_steps(&self) -> Result<(), String> {
        if self.steps.is_empty() || self.steps.len() > MOST_STEPS {
            return Err(format!("a runbook has 1 to {MOST_STEPS} steps"));
        }

        for (index, step) in self.steps.iter().enumerate() {
            step.check()
                .map_err(|reason| format!("step {}: {reason}", index + 1))?;
        }

        let rolls_back = self
            .steps
            .iter()
            .any(|step| step.on_failure == OnFailure::Rollback);

        if rolls_back && self.rollback.is_empty() {
            return Err("a step rolls back, but the runbook has no rollback steps".to_owned());
        }

        Ok(())
    }

    fn check_rollback(&self) -> Result<(), String> {
        if self.rollback.len() > MOST_STEPS {
            return Err(format!("a runbook has at most {MOST_STEPS} rollback steps"));
        }

        for (index, step) in self.rollback.iter().enumerate() {
            let position = index + 1;

            step.check()
                .map_err(|reason| format!("rollback step {position}: {reason}"))?;

            if step.on_failure != OnFailure::Stop {
                return Err(format!(
                    "rollback step {position}: rollback steps always go on after a failure, \
                     so on_failure does not apply"
                ));
            }
        }

        Ok(())
    }

    fn check_params_used(&self) -> Result<(), String> {
        let declared: BTreeSet<&str> = self.params.iter().map(|spec| spec.name.as_str()).collect();

        let used = self
            .steps
            .iter()
            .chain(&self.rollback)
            .flat_map(Step::params_used);

        for name in used {
            if !declared.contains(name.as_str()) {
                return Err(format!(
                    "{{{name}}} names no param; declare it, or write {{{{ and }}}} for literal braces"
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Runbook;
    use crate::runbooks::document::OnFailure;

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

    fn restart() -> Runbook {
        Runbook::from_toml(RESTART).unwrap()
    }

    fn broken(change: impl FnOnce(&mut Runbook)) -> String {
        let mut runbook = restart();
        change(&mut runbook);

        runbook.check().unwrap_err()
    }

    #[test]
    fn the_example_runbook_reads() {
        let runbook = restart();

        assert_eq!(runbook.name, "Restart API");
        assert_eq!(runbook.params.len(), 2);
        assert_eq!(runbook.steps[1].on_failure, OnFailure::Rollback);
        assert_eq!(runbook.steps[1].retry().attempts, 5);
        assert_eq!(
            runbook.steps[0].params["command"],
            json!("sudo systemctl restart {service}")
        );
        assert_eq!(runbook.rollback.len(), 1);
    }

    #[test]
    fn a_runbook_survives_writing_and_reading_back() {
        let runbook = restart();

        let written = runbook.to_toml().unwrap();

        assert_eq!(Runbook::from_toml(&written).unwrap(), runbook);
    }

    #[test]
    fn unknown_fields_are_refused() {
        let error = Runbook::from_toml(&format!("{RESTART}\nauthor = \"ann\"\n")).unwrap_err();

        assert!(error.contains("author"), "{error}");
    }

    #[test]
    fn every_placeholder_names_a_declared_param() {
        let error = broken(|runbook| {
            runbook.steps[0]
                .params
                .insert("command".to_owned(), json!("restart {unit}"));
        });

        assert!(error.starts_with("{unit} names no param"), "{error}");
    }

    #[test]
    fn rolling_back_needs_rollback_steps() {
        let error = broken(|runbook| runbook.rollback.clear());

        assert!(error.contains("no rollback steps"), "{error}");
    }

    #[test]
    fn rollback_steps_cannot_choose_what_happens_on_failure() {
        let error = broken(|runbook| runbook.rollback[0].on_failure = OnFailure::Continue);

        assert!(error.starts_with("rollback step 1"), "{error}");
    }

    #[test]
    fn a_runbook_cannot_run_runbooks() {
        let error = broken(|runbook| runbook.steps[0].action = "runbook.run".to_owned());

        assert_eq!(error, "step 1: runbook.run cannot be a step of a runbook");
    }

    #[test]
    fn params_are_declared_once() {
        let error = broken(|runbook| {
            let copy = runbook.params[0].clone();
            runbook.params.push(copy);
        });

        assert_eq!(error, "param host is declared twice");
    }

    #[test]
    fn a_runbook_has_steps_and_a_name() {
        assert!(broken(|runbook| runbook.steps.clear()).contains("1 to 100 steps"));
        assert!(broken(|runbook| runbook.name = "two\nlines".to_owned()).starts_with("name"));
        assert!(broken(|runbook| runbook.steps[1].title.clear()).starts_with("step 2"));
    }
}
