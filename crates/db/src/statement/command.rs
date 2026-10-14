mod words;

use self::words::{CONTAINERS, READ, READ_CONTAINERS, WRITE};
use super::classification::Classification;
use super::kind::StatementKind;
use crate::Effect;

const SUBCOMMANDS_THAT_WRITE: &str = "XGROUP ";

pub(super) struct Command {
    name: String,
    container: bool,
}

impl Command {
    pub(super) fn parse<S: AsRef<str>>(words: &[S]) -> Option<Self> {
        let (first, rest) = words.split_first()?;
        let name = upper(first);

        if !CONTAINERS.contains(&name.as_str()) {
            return Some(Self {
                name,
                container: false,
            });
        }

        let name = match rest.first() {
            Some(subcommand) => format!("{name} {}", upper(subcommand)),
            None => name,
        };

        Some(Self {
            name,
            container: true,
        })
    }

    pub(super) fn classification(&self) -> Classification {
        let kind = StatementKind::named(self.name.to_ascii_lowercase());

        Classification::new(self.effect(), kind)
    }

    fn effect(&self) -> Effect {
        if self.reads() {
            return Effect::Read;
        }

        if self.writes() {
            return Effect::Write;
        }

        Effect::Privileged
    }

    fn reads(&self) -> bool {
        let name = self.name.as_str();

        READ.contains(&name) || (self.container && READ_CONTAINERS.contains(&name))
    }

    fn writes(&self) -> bool {
        if self.container {
            return self.name.starts_with(SUBCOMMANDS_THAT_WRITE);
        }

        WRITE.contains(&self.name.as_str())
    }
}

fn upper<S: AsRef<str>>(word: &S) -> String {
    word.as_ref().trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use crate::{Classification, Effect, StatementKind};

    fn classify(words: &[&str]) -> Classification {
        Classification::of_command(words)
    }

    fn effect(words: &[&str]) -> Effect {
        classify(words).effect
    }

    #[test]
    fn lookups_are_reads_in_any_letter_case() {
        assert_eq!(effect(&["GET", "user:1"]), Effect::Read);
        assert_eq!(effect(&["hgetall", "user:1"]), Effect::Read);
        assert_eq!(effect(&["Scan", "0", "MATCH", "user:*"]), Effect::Read);
        assert_eq!(effect(&["PING"]), Effect::Read);
    }

    #[test]
    fn changes_to_keys_are_writes() {
        assert_eq!(effect(&["SET", "user:1", "x"]), Effect::Write);
        assert_eq!(effect(&["DEL", "user:1"]), Effect::Write);
        assert_eq!(effect(&["LPUSH", "queue", "job"]), Effect::Write);
        assert_eq!(effect(&["XGROUP", "CREATE", "s", "g", "$"]), Effect::Write);
    }

    #[test]
    fn server_control_and_scripts_are_privileged() {
        for words in [
            vec!["FLUSHALL"],
            vec!["FLUSHDB"],
            vec!["CONFIG", "GET", "requirepass"],
            vec!["EVAL", "return 1", "0"],
            vec!["SHUTDOWN"],
            vec!["SELECT", "1"],
            vec!["MULTI"],
            vec!["ACL", "LIST"],
        ] {
            assert_eq!(effect(&words), Effect::Privileged, "{words:?}");
        }
    }

    #[test]
    fn an_unknown_command_is_privileged() {
        assert_eq!(effect(&["MADEUP", "x"]), Effect::Privileged);
    }

    #[test]
    fn subcommands_decide_for_container_commands() {
        assert_eq!(effect(&["OBJECT", "encoding", "k"]), Effect::Read);
        assert_eq!(effect(&["MEMORY", "USAGE", "k"]), Effect::Read);
        assert_eq!(effect(&["MEMORY", "PURGE"]), Effect::Privileged);
        assert_eq!(
            classify(&["object", "encoding", "k"]).kind,
            StatementKind::named("object encoding")
        );
    }

    #[test]
    fn no_words_is_privileged() {
        let none: [&str; 0] = [];

        assert_eq!(classify(&none).effect, Effect::Privileged);
    }
}
