use std::fmt::Write;

use serde_json::Value;

use super::shell_word::{comment, double_quoted, quoted, shell_word};
use crate::runbooks::document::{OnFailure, Runbook, Step};

const SHELL_ACTION: &str = "ssh.exec";
const COMMAND: &str = "command";

const RETRY: &str = "retry() {
  attempts=$1; pause=$2; shift 2; attempt=1
  until \"$@\"; do
    if [ \"$attempt\" -ge \"$attempts\" ]; then return 1; fi
    attempt=$((attempt + 1)); sleep \"$pause\"
  done
}
";

const EXPECT_EXIT: &str = "expect_exit() {
  wanted=$1; shift
  \"$@\"
  [ \"$?\" -eq \"$wanted\" ]
}
";

pub(crate) struct ShellScript<'a> {
    reference: &'a str,
    runbook: &'a Runbook,
    text: String,
}

impl<'a> ShellScript<'a> {
    pub(crate) fn new(reference: &'a str, runbook: &'a Runbook) -> Self {
        Self {
            reference,
            runbook,
            text: String::new(),
        }
    }

    pub(crate) fn render(mut self) -> String {
        self.header();
        self.params();
        self.helpers();
        self.rollback();

        for (index, step) in self.runbook.steps.iter().enumerate() {
            self.step(index + 1, step, &tail(step.on_failure));
        }

        self.text
    }

    fn header(&mut self) {
        self.line("#!/bin/sh");
        self.line(&format!("# {}", comment(&self.runbook.name)));

        if !self.runbook.description.is_empty() {
            self.line(&format!("# {}", comment(&self.runbook.description)));
        }

        self.line("#");
        self.line(&format!(
            "# Exported from the Synt runbook {}.",
            self.reference
        ));
        self.line("# Targets run through ssh, so a project target such as @project/host must");
        self.line("# be a name ssh knows, for example a Host in ~/.ssh/config.");
        self.line("set -u");
        self.blank();
    }

    fn params(&mut self) {
        if self.runbook.params.is_empty() {
            return;
        }

        for spec in &self.runbook.params {
            let name = &spec.name;
            let line = match &spec.default {
                Some(default) => format!(": \"${{{name}:={}}}\"", double_quoted(&text(default))),
                None => format!(": \"${{{name}:?set {name} before running this script}}\""),
            };

            self.line(&line);
        }

        self.blank();
    }

    fn helpers(&mut self) {
        let steps = || self.runbook.steps.iter().chain(&self.runbook.rollback);

        if steps().any(|step| step.retry().attempts > 1) {
            self.text.push_str(RETRY);
            self.blank();
        }

        if steps().any(|step| exit_wanted(step) != 0) {
            self.text.push_str(EXPECT_EXIT);
            self.blank();
        }
    }

    fn rollback(&mut self) {
        if self.runbook.rollback.is_empty() {
            return;
        }

        let runbook = self.runbook;

        self.line("rollback() {");

        for (index, step) in runbook.rollback.iter().enumerate().rev() {
            self.step(index + 1, step, "|| true");
        }

        self.line("}");
        self.blank();
    }

    fn step(&mut self, number: usize, step: &Step, tail: &str) {
        self.line(&format!("# {number}. {}", comment(&step.title)));
        self.unchecked(step);

        let Some(command) = shell_command(step) else {
            self.line(&format!(
                "echo {} >&2",
                quoted(&format!(
                    "step {number} ({}) is not a shell command; run it in Synt",
                    step.action
                ))
            ));
            self.blank();
            return;
        };

        let command = wrapped(step, command);
        self.line(&format!("{command} {tail}"));
        self.blank();
    }

    fn unchecked(&mut self, step: &Step) {
        let Some(expect) = &step.expect else {
            return;
        };

        if let Some(contains) = &expect.contains {
            self.line(&format!(
                "# not checked here: the output should contain {}",
                comment(contains)
            ));
        }

        for (pointer, wanted) in &expect.fields {
            self.line(&format!("# not checked here: {pointer} should be {wanted}"));
        }
    }

    fn line(&mut self, line: &str) {
        let _ = writeln!(self.text, "{line}");
    }

    fn blank(&mut self) {
        self.text.push('\n');
    }
}

fn shell_command(step: &Step) -> Option<String> {
    if step.action != SHELL_ACTION {
        return None;
    }

    let target = step.target.as_deref()?;
    let command = step.params.get(COMMAND)?.as_str()?;

    Some(format!(
        "ssh -- {} {}",
        shell_word(target),
        shell_word(command)
    ))
}

fn wrapped(step: &Step, command: String) -> String {
    let exit = exit_wanted(step);
    let checked = match exit {
        0 => command,
        wanted => format!("expect_exit {wanted} {command}"),
    };

    let retry = step.retry();

    if retry.attempts <= 1 {
        return checked;
    }

    format!("retry {} {} {checked}", retry.attempts, retry.every_seconds)
}

fn exit_wanted(step: &Step) -> i64 {
    step.expect
        .as_ref()
        .and_then(|expect| expect.exit)
        .unwrap_or(0)
}

fn tail(on_failure: OnFailure) -> String {
    match on_failure {
        OnFailure::Stop => "|| exit 1".to_owned(),
        OnFailure::Continue => "|| true".to_owned(),
        OnFailure::Rollback => "|| { rollback; exit 1; }".to_owned(),
    }
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellScript;
    use crate::runbooks::document::Runbook;

    const RESTART: &str = r#"
name = "Restart API"
description = "Restart the orders API and check its health."

[[params]]
name = "host"
kind = "target"
default = "api-1"

[[params]]
name = "service"
kind = "word"

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

[[steps]]
title = "Note the release"
action = "sftp.write"
target = "{host}"
params = { path = "/srv/release", data = "djc=" }
on_failure = "continue"

[[rollback]]
title = "Stop it"
action = "ssh.exec"
target = "{host}"
params = { command = "test -f /run/{service}.pid" }
expect = { exit = 1 }

[[rollback]]
title = "Start the previous release"
action = "ssh.exec"
target = "{host}"
params = { command = "sudo systemctl start {service}-previous" }
"#;

    fn script() -> String {
        let runbook = Runbook::from_toml(RESTART).unwrap();

        ShellScript::new("@lab/restart", &runbook).render()
    }

    #[test]
    fn the_script_names_the_runbook_it_came_from() {
        let script = script();

        assert!(script.starts_with("#!/bin/sh\n# Restart API\n# Restart the orders API"));
        assert!(script.contains("# Exported from the Synt runbook @lab/restart."));
    }

    #[test]
    fn params_become_variables_with_their_defaults() {
        let script = script();

        assert!(script.contains(": \"${host:=api-1}\"\n"));
        assert!(script.contains(": \"${service:?set service before running this script}\"\n"));
    }

    #[test]
    fn shell_steps_run_through_ssh_and_stop_on_failure() {
        assert!(script().contains(
            "# 1. Restart $service\n\
             ssh -- \"${host}\" 'sudo systemctl restart '\"${service}\" || exit 1\n"
        ));
    }

    #[test]
    fn retries_and_rollbacks_are_spelled_out() {
        let script = script();

        assert!(script.contains("retry() {"));
        assert!(script.contains(
            "retry 5 3 ssh -- \"${host}\" 'curl -fsS http://127.0.0.1:8080/health' \
             || { rollback; exit 1; }\n"
        ));
        assert!(script.contains("# not checked here: the output should contain ok\n"));
    }

    #[test]
    fn the_rollback_function_undoes_in_reverse_and_never_stops() {
        let script = script();
        let rollback = &script[script.find("rollback() {").unwrap()..];
        let previous = rollback.find("# 2. Start the previous release").unwrap();
        let stop = rollback.find("# 1. Stop it").unwrap();

        assert!(previous < stop);
        assert!(rollback.contains(
            "expect_exit 1 ssh -- \"${host}\" 'test -f /run/'\"${service}\"'.pid' || true\n"
        ));
        assert!(script.contains("expect_exit() {"));
    }

    #[test]
    fn steps_that_are_not_shell_commands_are_marked() {
        assert!(script().contains(
            "# 3. Note the release\n\
             echo 'step 3 (sftp.write) is not a shell command; run it in Synt' >&2\n"
        ));
    }
}
