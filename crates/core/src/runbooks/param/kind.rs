use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::given::Given;

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
        let given = Given::new(value);

        match self {
            Self::String | Self::Choice => given.text().map(Value::from),
            Self::Word => given.word().map(Value::from),
            Self::Integer => given.integer().map(Value::from),
            Self::Boolean => given.boolean().map(Value::from),
            Self::Target => given.target().map(Value::from),
            Self::Path => given.path().map(Value::from),
        }
    }

    pub(crate) fn of_literal(text: &str) -> Self {
        let literal = Value::from(text);

        if Given::new(&literal).word().is_ok() {
            return Self::Word;
        }

        Self::String
    }
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
