use serde::Deserialize;

const ESCALATORS: [&str; 13] = [
    "sudo",
    "sudoedit",
    "su",
    "doas",
    "pkexec",
    "run0",
    "runuser",
    "machinectl",
    "systemd-run",
    "chroot",
    "setpriv",
    "ssh",
    "nsenter",
];
const INDIRECTIONS: [&str; 21] = [
    "sh", "bash", "zsh", "dash", "ksh", "fish", "csh", "tcsh", "busybox", "eval", "exec", "source",
    ".", "env", "xargs", "python", "perl", "ruby", "node", "php", "base64",
];
const VERSIONED_INTERPRETERS: [&str; 3] = ["python", "perl", "ruby"];
const SEPARATORS: [char; 8] = [';', '&', '|', '(', ')', '<', '>', '\n'];
const EXPANSIONS: [char; 10] = ['$', '`', '\\', '{', '}', '!', '*', '?', '[', ']'];
const QUOTES: [char; 2] = ['"', '\''];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct RemoteCommand(String);

impl RemoteCommand {
    pub fn new(command: impl Into<String>) -> Self {
        Self(command.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn escalates(&self) -> bool {
        if self.0.contains(EXPANSIONS) {
            return true;
        }

        self.0
            .split(|character: char| character.is_whitespace() || SEPARATORS.contains(&character))
            .filter(|word| !word.is_empty())
            .any(Self::word_escalates)
    }

    fn word_escalates(word: &str) -> bool {
        if Self::has_inner_quote(word) {
            return true;
        }

        let program = Self::program_name(word);

        ESCALATORS.contains(&program)
            || INDIRECTIONS.contains(&program)
            || Self::is_versioned(program)
    }

    fn has_inner_quote(word: &str) -> bool {
        let last = word.chars().count().saturating_sub(1);

        word.chars()
            .enumerate()
            .any(|(index, character)| QUOTES.contains(&character) && index != 0 && index != last)
    }

    fn program_name(word: &str) -> &str {
        let unquoted = word.trim_matches(QUOTES);

        unquoted.rsplit('/').next().unwrap_or(unquoted)
    }

    fn is_versioned(program: &str) -> bool {
        VERSIONED_INTERPRETERS.iter().any(|interpreter| {
            program.strip_prefix(interpreter).is_some_and(|version| {
                version
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '.')
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RemoteCommand;

    fn escalates(command: &str) -> bool {
        RemoteCommand::new(command).escalates()
    }

    #[test]
    fn plain_commands_do_not_escalate() {
        assert!(!escalates("ls -la /var/log"));
        assert!(!escalates("systemctl status nginx"));
    }

    #[test]
    fn escalators_and_indirections_do() {
        assert!(escalates("sudo ls"));
        assert!(escalates("cd / && /usr/bin/su -"));
        assert!(escalates("bash -c 'id'"));
        assert!(escalates("python3.12 run.py"));
        assert!(escalates("echo $HOME"));
        assert!(escalates("s'u'do id"));
    }
}
