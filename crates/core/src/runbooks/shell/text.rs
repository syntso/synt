use super::quote::Quote;
use crate::runbooks::template::{Piece, TextTemplate};

pub(super) struct ShellText {
    template: TextTemplate,
}

impl ShellText {
    pub(super) fn new(template: &str) -> Self {
        Self {
            template: TextTemplate::parse(template),
        }
    }

    pub(super) fn word(&self) -> String {
        if self.template.pieces().is_empty() {
            return "".single_quoted();
        }

        self.template
            .pieces()
            .iter()
            .map(|piece| match piece {
                Piece::Text(text) => text.single_quoted(),
                Piece::Param(name) => format!("\"${{{name}}}\""),
            })
            .collect()
    }

    pub(super) fn comment(&self) -> String {
        let text: String = self
            .template
            .pieces()
            .iter()
            .map(|piece| match piece {
                Piece::Text(text) => text.clone(),
                Piece::Param(name) => format!("${name}"),
            })
            .collect();

        text.replace(char::is_control, " ")
    }
}

#[cfg(test)]
mod tests {
    use super::ShellText;

    fn word(template: &str) -> String {
        ShellText::new(template).word()
    }

    #[test]
    fn literal_text_is_a_single_quoted_word() {
        assert_eq!(word("echo $HOME"), "'echo $HOME'");
        assert_eq!(word(""), "''");
    }

    #[test]
    fn placeholders_become_quoted_shell_variables() {
        assert_eq!(
            word("systemctl restart {service}.service"),
            "'systemctl restart '\"${service}\"'.service'"
        );
        assert_eq!(word("{host}"), "\"${host}\"");
    }

    #[test]
    fn escaped_braces_are_written_as_plain_braces() {
        assert_eq!(word("awk '{{print $1}}'"), "'awk '\\''{print $1}'\\'''");
    }

    #[test]
    fn comments_name_params_as_variables_on_one_line() {
        assert_eq!(
            ShellText::new("Restart {service}\nnow").comment(),
            "Restart $service now"
        );
    }
}
