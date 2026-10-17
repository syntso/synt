use serde_json::Value;

use super::Piece;
use crate::CoreError;
use crate::runbooks::param::{ParamName, Values};

const OPEN: char = '{';
const CLOSE: char = '}';
const SHELL_VARIABLE: char = '$';

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextTemplate {
    pieces: Vec<Piece>,
}

impl TextTemplate {
    pub(crate) fn parse(text: &str) -> Self {
        let mut template = Self { pieces: Vec::new() };
        let mut literal = String::new();
        let mut rest = text;

        while let Some(character) = rest.chars().next() {
            let escaped = rest.starts_with("{{") || rest.starts_with("}}");
            let after_dollar = literal.ends_with(SHELL_VARIABLE);

            if escaped {
                literal.push(character);
                rest = &rest[2..];
                continue;
            }

            if let Some(name) = Self::placeholder(rest).filter(|_| !after_dollar) {
                rest = &rest[name.as_str().len() + 2..];
                template.flush(&mut literal);
                template.pieces.push(Piece::Param(name));
                continue;
            }

            literal.push(character);
            rest = &rest[character.len_utf8()..];
        }

        template.flush(&mut literal);
        template
    }

    pub(crate) fn escape(text: &str) -> String {
        text.replace(OPEN, "{{").replace(CLOSE, "}}")
    }

    pub(crate) fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub(crate) fn params(&self) -> impl Iterator<Item = &ParamName> {
        self.pieces.iter().filter_map(|piece| match piece {
            Piece::Param(name) => Some(name),
            Piece::Text(_) => None,
        })
    }

    pub(crate) fn render(&self, values: &Values) -> Result<Value, CoreError> {
        if let [Piece::Param(name)] = self.pieces.as_slice() {
            return values.get(name.as_str()).cloned();
        }

        self.render_text(values).map(Value::String)
    }

    pub(crate) fn render_text(&self, values: &Values) -> Result<String, CoreError> {
        let mut text = String::new();

        for piece in &self.pieces {
            match piece {
                Piece::Text(literal) => text.push_str(literal),
                Piece::Param(name) => text.push_str(&values.text(name.as_str())?),
            }
        }

        Ok(text)
    }

    fn placeholder(text: &str) -> Option<ParamName> {
        let inner = text.strip_prefix(OPEN)?;
        let end = inner.find(CLOSE)?;

        ParamName::new(&inner[..end]).ok()
    }

    fn flush(&mut self, literal: &mut String) {
        if literal.is_empty() {
            return;
        }

        self.pieces.push(Piece::Text(std::mem::take(literal)));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Piece, TextTemplate};
    use crate::runbooks::param::{ParamName, Values};

    fn values() -> Values {
        Values::from_pairs([
            ("host", json!("@orders/api-1")),
            ("service", json!("orders-api")),
            ("port", json!(8080)),
            ("verbose", json!(true)),
        ])
    }

    fn render(text: &str) -> serde_json::Value {
        TextTemplate::parse(text).render(&values()).unwrap()
    }

    #[test]
    fn a_placeholder_is_replaced_by_the_value_of_its_param() {
        assert_eq!(
            render("sudo systemctl restart {service}"),
            json!("sudo systemctl restart orders-api")
        );
    }

    #[test]
    fn a_lone_placeholder_keeps_the_type_of_its_value() {
        assert_eq!(render("{port}"), json!(8080));
        assert_eq!(render("{verbose}"), json!(true));
        assert_eq!(render("port {port}"), json!("port 8080"));
    }

    #[test]
    fn doubled_braces_are_literal_braces() {
        assert_eq!(
            render("awk '{{print $1}}' {service}.log"),
            json!("awk '{print $1}' orders-api.log")
        );
    }

    #[test]
    fn shell_variables_are_left_to_the_shell() {
        assert_eq!(
            render("echo ${service} {service}"),
            json!("echo ${service} orders-api")
        );
    }

    #[test]
    fn braces_around_anything_but_a_name_stay_as_written() {
        assert_eq!(
            render("find . -exec rm {} ;"),
            json!("find . -exec rm {} ;")
        );
        assert_eq!(render("{ echo a; }"), json!("{ echo a; }"));
        assert_eq!(render("{9lives}"), json!("{9lives}"));
        assert_eq!(render("tail {"), json!("tail {"));
    }

    #[test]
    fn the_names_a_template_uses_are_listed_in_order() {
        let template = TextTemplate::parse("{host}:{port} ${HOME} {{host}} {host}");

        assert_eq!(
            template.params().map(ParamName::as_str).collect::<Vec<_>>(),
            ["host", "port", "host"]
        );
    }

    #[test]
    fn a_missing_value_is_an_error() {
        let error = TextTemplate::parse("restart {unit}")
            .render(&values())
            .unwrap_err()
            .to_string();

        assert!(error.contains("unit"), "{error}");
    }

    #[test]
    fn escaped_text_renders_back_to_itself() {
        let text = "awk '{print $1}' ${HOME} {{ }}";
        let escaped = TextTemplate::escape(text);

        assert_eq!(
            TextTemplate::parse(&escaped).render(&values()).unwrap(),
            json!(text)
        );
        assert_eq!(TextTemplate::parse(&escaped).params().count(), 0);
    }

    #[test]
    fn text_and_placeholders_alternate_as_pieces() {
        let template = TextTemplate::parse("a{host}b");

        assert_eq!(
            template.pieces(),
            [
                Piece::Text("a".to_owned()),
                Piece::Param(ParamName::new("host").unwrap()),
                Piece::Text("b".to_owned()),
            ]
        );
    }
}
