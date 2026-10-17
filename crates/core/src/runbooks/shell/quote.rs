const DOUBLE_QUOTE_SPECIALS: &str = "\\$`\"}";

pub(super) trait Quote {
    fn single_quoted(&self) -> String;

    fn inside_double_quotes(&self) -> String;
}

impl Quote for str {
    fn single_quoted(&self) -> String {
        format!("'{}'", self.replace('\'', "'\\''"))
    }

    fn inside_double_quotes(&self) -> String {
        let mut escaped = String::with_capacity(self.len());

        for character in self.chars() {
            if DOUBLE_QUOTE_SPECIALS.contains(character) {
                escaped.push('\\');
            }

            escaped.push(character);
        }

        escaped
    }
}

#[cfg(test)]
mod tests {
    use super::Quote;

    #[test]
    fn literal_text_is_single_quoted() {
        assert_eq!("it's".single_quoted(), "'it'\\''s'");
    }

    #[test]
    fn double_quoted_text_cannot_expand_or_end_an_expansion() {
        assert_eq!(
            "a $b `c` \\ \" }".inside_double_quotes(),
            "a \\$b \\`c\\` \\\\ \\\" \\}"
        );
    }
}
