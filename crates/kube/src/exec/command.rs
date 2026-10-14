use schemars::JsonSchema;
use serde::Deserialize;

use crate::KubeError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "Vec<String>")]
pub struct Command(Vec<String>);

impl Command {
    pub fn words(&self) -> &[String] {
        &self.0
    }

    pub fn line(&self) -> String {
        self.0.join(" ")
    }
}

impl TryFrom<Vec<String>> for Command {
    type Error = KubeError;

    fn try_from(words: Vec<String>) -> Result<Self, KubeError> {
        if words
            .first()
            .is_none_or(|program| program.trim().is_empty())
        {
            return Err(KubeError::invalid(
                "the command is empty; give the program and its arguments as a list",
            ));
        }

        Ok(Self(words))
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn a_command_needs_a_program() {
        assert!(Command::try_from(Vec::new()).is_err());
        assert!(Command::try_from(vec![" ".to_owned()]).is_err());
    }

    #[test]
    fn the_line_joins_the_words() {
        let command = Command::try_from(vec!["ls".to_owned(), "-l".to_owned()]).unwrap();

        assert_eq!(command.line(), "ls -l");
    }
}
