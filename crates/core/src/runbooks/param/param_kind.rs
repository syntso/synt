use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const LONGEST_TEXT: usize = 4096;
const LONGEST_WORD: usize = 256;
const WORD_SYMBOLS: &str = "-_.:/@=+,%";
const WILDCARDS: [char; 2] = ['*', '?'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    String,
    Word,
    Integer,
    Boolean,
    Target,
    Path,
    Choice,
}

impl ParamKind {
    pub(crate) fn accept(self, value: &Value) -> Result<Value, String> {
        match self {
            Self::String | Self::Choice => text(value).map(Value::from),
            Self::Word => word(value).map(Value::from),
            Self::Integer => integer(value).map(Value::from),
            Self::Boolean => boolean(value).map(Value::from),
            Self::Target => target(value).map(Value::from),
            Self::Path => path(value).map(Value::from),
        }
    }

    pub(crate) fn of_literal(text: &str) -> Self {
        if word(&Value::from(text)).is_ok() {
            return Self::Word;
        }

        Self::String
    }
}

fn text(value: &Value) -> Result<&str, String> {
    let Some(text) = value.as_str() else {
        return Err(format!("expected text, got {value}"));
    };

    if text.len() > LONGEST_TEXT {
        return Err(format!("text is longer than {LONGEST_TEXT} bytes"));
    }

    if text.contains(char::is_control) {
        return Err("text cannot hold control characters such as new lines".to_owned());
    }

    Ok(text)
}

fn word(value: &Value) -> Result<&str, String> {
    let word = text(value)?;
    let allowed =
        |character: char| character.is_ascii_alphanumeric() || WORD_SYMBOLS.contains(character);

    if word.is_empty() || word.len() > LONGEST_WORD || !word.chars().all(allowed) {
        return Err(format!(
            "{word:?} is not a word: use 1 to {LONGEST_WORD} letters, digits or {WORD_SYMBOLS}"
        ));
    }

    Ok(word)
}

fn integer(value: &Value) -> Result<i64, String> {
    let parsed = match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    };

    parsed.ok_or_else(|| format!("expected a whole number, got {value}"))
}

fn boolean(value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(flag) => Ok(*flag),
        Value::String(text) if text == "true" => Ok(true),
        Value::String(text) if text == "false" => Ok(false),
        _ => Err(format!("expected true or false, got {value}")),
    }
}

fn target(value: &Value) -> Result<&str, String> {
    let target = text(value)?;
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

fn path(value: &Value) -> Result<&str, String> {
    let path = text(value)?;

    if path.is_empty() {
        return Err("a path cannot be empty".to_owned());
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ParamKind;

    #[test]
    fn text_refuses_new_lines_that_would_start_another_command() {
        assert_eq!(
            ParamKind::String.accept(&json!("a b; c")),
            Ok(json!("a b; c"))
        );
        assert!(ParamKind::String.accept(&json!("a\nreboot")).is_err());
        assert!(ParamKind::String.accept(&json!(5)).is_err());
    }

    #[test]
    fn a_word_holds_no_spaces_or_shell_symbols() {
        assert_eq!(
            ParamKind::Word.accept(&json!("orders-api")),
            Ok(json!("orders-api"))
        );
        assert_eq!(
            ParamKind::Word.accept(&json!("v1.2+build=3")),
            Ok(json!("v1.2+build=3"))
        );
        assert!(ParamKind::Word.accept(&json!("a b")).is_err());
        assert!(ParamKind::Word.accept(&json!("a;b")).is_err());
        assert!(ParamKind::Word.accept(&json!("$(id)")).is_err());
        assert!(ParamKind::Word.accept(&json!("")).is_err());
    }

    #[test]
    fn integers_come_as_numbers_or_digits() {
        assert_eq!(ParamKind::Integer.accept(&json!(3)), Ok(json!(3)));
        assert_eq!(ParamKind::Integer.accept(&json!("42")), Ok(json!(42)));
        assert!(ParamKind::Integer.accept(&json!(1.5)).is_err());
        assert!(ParamKind::Integer.accept(&json!("four")).is_err());
    }

    #[test]
    fn booleans_come_as_flags_or_words() {
        assert_eq!(ParamKind::Boolean.accept(&json!(false)), Ok(json!(false)));
        assert_eq!(ParamKind::Boolean.accept(&json!("true")), Ok(json!(true)));
        assert!(ParamKind::Boolean.accept(&json!("yes")).is_err());
    }

    #[test]
    fn a_target_names_exactly_one_host() {
        assert_eq!(
            ParamKind::Target.accept(&json!("@orders/api-1")),
            Ok(json!("@orders/api-1"))
        );
        assert_eq!(
            ParamKind::Target.accept(&json!("deploy@10.0.1.12:2222")),
            Ok(json!("deploy@10.0.1.12:2222"))
        );
        assert!(ParamKind::Target.accept(&json!("@orders/*")).is_err());
        assert!(
            ParamKind::Target
                .accept(&json!("-oProxyCommand=x"))
                .is_err()
        );
        assert!(ParamKind::Target.accept(&json!("api 1")).is_err());
    }

    #[test]
    fn a_path_is_any_text_but_empty() {
        assert_eq!(
            ParamKind::Path.accept(&json!("/srv/my app")),
            Ok(json!("/srv/my app"))
        );
        assert!(ParamKind::Path.accept(&json!("")).is_err());
    }

    #[test]
    fn literals_that_are_words_become_word_params() {
        assert_eq!(ParamKind::of_literal("orders-api"), ParamKind::Word);
        assert_eq!(ParamKind::of_literal("hello world"), ParamKind::String);
    }
}
