use super::quote::Quote;
use super::text::ShellText;
use crate::runbooks::document::Step;

pub(super) struct ShellStep<'a> {
    number: usize,
    step: &'a Step,
}

impl<'a> ShellStep<'a> {
    pub(super) fn new(number: usize, step: &'a Step) -> Self {
        Self { number, step }
    }

    pub(super) fn lines(&self, tail: &str) -> Vec<String> {
        let mut lines = vec![self.heading()];
        lines.extend(self.unchecked());

        let last = match self.command() {
            Some(command) => format!("{command} {tail}"),
            None => self.refusal(),
        };

        lines.push(last);
        lines
    }

    fn heading(&self) -> String {
        let title = ShellText::new(self.step.title().as_str()).comment();

        format!("# {}. {title}", self.number)
    }

    fn unchecked(&self) -> Vec<String> {
        let Some(expect) = self.step.expect() else {
            return Vec::new();
        };

        let contains = expect.contains().into_iter().map(|contains| {
            format!(
                "# not checked here: the output should contain {}",
                ShellText::new(contains).comment()
            )
        });

        let fields = expect.fields().map(|(pointer, wanted)| {
            format!(
                "# not checked here: {} should be {}",
                ShellText::new(pointer.as_str()).comment(),
                ShellText::new(&wanted.to_string()).comment()
            )
        });

        contains.chain(fields).collect()
    }

    fn refusal(&self) -> String {
        let message = format!(
            "step {} ({}) is not a shell command; run it in Synt",
            self.number,
            self.step.action()
        );

        format!("echo {} >&2", message.single_quoted())
    }

    fn command(&self) -> Option<String> {
        let command = self.step.shell_command()?;
        let target = self.step.target()?;

        let ssh = format!(
            "ssh -- {} {}",
            ShellText::new(target).word(),
            ShellText::new(command).word()
        );

        Some(self.wrapped(ssh))
    }

    fn wrapped(&self, command: String) -> String {
        let checked = match self.step.exit_wanted() {
            0 => command,
            wanted => format!("expect_exit {wanted} {command}"),
        };

        let retry = self.step.retry();

        if !retry.repeats() {
            return checked;
        }

        format!(
            "retry {} {} {checked}",
            retry.attempts(),
            retry.every_seconds()
        )
    }
}
