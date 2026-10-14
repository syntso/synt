use crate::runbooks::template::{Piece, Template};

pub(crate) fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub(crate) fn double_quoted(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for character in text.chars() {
        if "\\$`\"}".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }

    escaped
}

pub(crate) fn shell_word(template: &str) -> String {
    let parsed = Template::parse(template);

    if parsed.pieces().is_empty() {
        return quoted("");
    }

    parsed
        .pieces()
        .iter()
        .map(|piece| match piece {
            Piece::Text(text) => quoted(text),
            Piece::Param(name) => format!("\"${{{name}}}\""),
        })
        .collect()
}

pub(crate) fn comment(template: &str) -> String {
    let parsed = Template::parse(template);

    let text: String = parsed
        .pieces()
        .iter()
        .map(|piece| match piece {
            Piece::Text(text) => text.clone(),
            Piece::Param(name) => format!("${name}"),
        })
        .collect();

    text.replace(char::is_control, " ")
}

#[cfg(test)]
mod tests {
    use super::{comment, double_quoted, quoted, shell_word};

    #[test]
    fn literal_text_is_single_quoted() {
        assert_eq!(quoted("it's"), "'it'\\''s'");
        assert_eq!(shell_word("echo $HOME"), "'echo $HOME'");
        assert_eq!(shell_word(""), "''");
    }

    #[test]
    fn double_quoted_text_cannot_expand_or_end_an_expansion() {
        assert_eq!(
            double_quoted("a $b `c` \\ \" }"),
            "a \\$b \\`c\\` \\\\ \\\" \\}"
        );
    }

    #[test]
    fn placeholders_become_quoted_shell_variables() {
        assert_eq!(
            shell_word("systemctl restart {service}.service"),
            "'systemctl restart '\"${service}\"'.service'"
        );
        assert_eq!(shell_word("{host}"), "\"${host}\"");
    }

    #[test]
    fn escaped_braces_are_written_as_plain_braces() {
        assert_eq!(
            shell_word("awk '{{print $1}}'"),
            "'awk '\\''{print $1}'\\'''"
        );
    }

    #[test]
    fn comments_name_params_as_variables_on_one_line() {
        assert_eq!(comment("Restart {service}\nnow"), "Restart $service now");
    }
}
