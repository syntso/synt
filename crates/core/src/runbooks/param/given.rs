use serde_json::Value;

use crate::CoreError;

const LONGEST_TEXT: usize = 4096;
const LONGEST_WORD: usize = 256;
const WORD_SYMBOLS: &str = "-_.:/@=+,%";
const WILDCARDS: [char; 2] = ['*', '?'];

pub(super) struct Given<'a> {
    value: &'a Value,
}

impl<'a> Given<'a> {
    pub(super) fn new(value: &'a Value) -> Self {
        Self { value }
    }

    pub(super) fn text(&self) -> Result<&'a str, CoreError> {
        let Some(text) = self.value.as_str() else {
            return Err(CoreError::NotText(self.value.clone()));
        };

        if text.len() > LONGEST_TEXT {
            return Err(CoreError::TextTooLong(LONGEST_TEXT));
        }

        if text.contains(char::is_control) {
            return Err(CoreError::ControlInText);
        }

        Ok(text)
    }

    pub(super) fn word(&self) -> Result<&'a str, CoreError> {
        let word = self.text()?;
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || WORD_SYMBOLS.contains(character);

        if word.is_empty() || word.len() > LONGEST_WORD || !word.chars().all(allowed) {
            return Err(CoreError::NotWord {
                word: word.to_owned(),
                longest: LONGEST_WORD,
                symbols: WORD_SYMBOLS,
            });
        }

        Ok(word)
    }

    pub(super) fn integer(&self) -> Result<i64, CoreError> {
        let parsed = match self.value {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        };

        parsed.ok_or_else(|| CoreError::NotInteger(self.value.clone()))
    }

    pub(super) fn boolean(&self) -> Result<bool, CoreError> {
        match self.value {
            Value::Bool(flag) => Ok(*flag),
            Value::String(text) if text == "true" => Ok(true),
            Value::String(text) if text == "false" => Ok(false),
            _ => Err(CoreError::NotBoolean(self.value.clone())),
        }
    }

    pub(super) fn target(&self) -> Result<&'a str, CoreError> {
        let target = self.text()?;
        let plain = !target.is_empty()
            && !target.starts_with('-')
            && !target.contains(char::is_whitespace)
            && !target.contains(WILDCARDS);

        if !plain {
            return Err(CoreError::NotTarget(target.to_owned()));
        }

        Ok(target)
    }

    pub(super) fn path(&self) -> Result<&'a str, CoreError> {
        let path = self.text()?;

        if path.is_empty() {
            return Err(CoreError::EmptyPathParam);
        }

        Ok(path)
    }
}
