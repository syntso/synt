use schemars::JsonSchema;
use serde::Deserialize;

use crate::CoreError;

const DEFAULT_TERM: &str = "xterm-256color";
const LONGEST_TERM: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub(crate) struct Term(String);

impl Term {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    fn allowed(character: char) -> bool {
        character.is_ascii_alphanumeric() || "-_.+".contains(character)
    }
}

impl Default for Term {
    fn default() -> Self {
        Self(DEFAULT_TERM.to_owned())
    }
}

impl TryFrom<String> for Term {
    type Error = CoreError;

    fn try_from(term: String) -> Result<Self, CoreError> {
        if term.is_empty() || term.len() > LONGEST_TERM || !term.chars().all(Self::allowed) {
            return Err(CoreError::BadTerm {
                longest: LONGEST_TERM,
            });
        }

        Ok(Self(term))
    }
}

#[cfg(test)]
mod tests {
    use super::Term;

    #[test]
    fn a_term_is_a_short_plain_name() {
        assert_eq!(Term::default().as_str(), "xterm-256color");
        assert_eq!(
            Term::try_from("screen.linux+x".to_owned())
                .unwrap()
                .as_str(),
            "screen.linux+x"
        );

        assert!(Term::try_from(String::new()).is_err());
        assert!(Term::try_from("x; rm -rf /".to_owned()).is_err());
        assert!(Term::try_from("x".repeat(65)).is_err());
    }
}
