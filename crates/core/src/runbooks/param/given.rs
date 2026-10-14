use serde_json::Value;

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

    pub(super) fn text(&self) -> Result<&'a str, String> {
        let Some(text) = self.value.as_str() else {
            return Err(format!("expected text, got {}", self.value));
        };

        if text.len() > LONGEST_TEXT {
            return Err(format!("text is longer than {LONGEST_TEXT} bytes"));
        }

        if text.contains(char::is_control) {
            return Err("text cannot hold control characters such as new lines".to_owned());
        }

        Ok(text)
    }

    pub(super) fn word(&self) -> Result<&'a str, String> {
        let word = self.text()?;
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || WORD_SYMBOLS.contains(character);

        if word.is_empty() || word.len() > LONGEST_WORD || !word.chars().all(allowed) {
            return Err(format!(
                "{word:?} is not a word: use 1 to {LONGEST_WORD} letters, digits or {WORD_SYMBOLS}"
            ));
        }

        Ok(word)
    }

    pub(super) fn integer(&self) -> Result<i64, String> {
        let parsed = match self.value {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        };

        parsed.ok_or_else(|| format!("expected a whole number, got {}", self.value))
    }

    pub(super) fn boolean(&self) -> Result<bool, String> {
        match self.value {
            Value::Bool(flag) => Ok(*flag),
            Value::String(text) if text == "true" => Ok(true),
            Value::String(text) if text == "false" => Ok(false),
            _ => Err(format!("expected true or false, got {}", self.value)),
        }
    }

    pub(super) fn target(&self) -> Result<&'a str, String> {
        let target = self.text()?;
        let plain = !target.is_empty()
            && !target.starts_with('-')
            && !target.contains(char::is_whitespace)
            && !target.contains(WILDCARDS);

        if !plain {
            return Err(format!(
                "{target:?} is not a target: name one host, such as @project/api-1 or deploy@10.0.1.12"
            ));
        }

        Ok(target)
    }

    pub(super) fn path(&self) -> Result<&'a str, String> {
        let path = self.text()?;

        if path.is_empty() {
            return Err("a path cannot be empty".to_owned());
        }

        Ok(path)
    }
}
