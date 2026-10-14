const DOUBLE_QUOTE_SPECIALS: &str = "\\$`\"}";

pub(super) fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub(super) fn double_quoted(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for character in text.chars() {
        if DOUBLE_QUOTE_SPECIALS.contains(character) {
            escaped.push('\\');
        }

        escaped.push(character);
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::{double_quoted, quoted};

    #[test]
    fn literal_text_is_single_quoted() {
        assert_eq!(quoted("it's"), "'it'\\''s'");
    }

    #[test]
    fn double_quoted_text_cannot_expand_or_end_an_expansion() {
        assert_eq!(
            double_quoted("a $b `c` \\ \" }"),
            "a \\$b \\`c\\` \\\\ \\\" \\}"
        );
    }
}
