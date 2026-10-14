use std::iter::Peekable;
use std::str::Chars;

const MASK: char = '?';
const LONGEST: usize = 2048;
const CUT: &str = " …";

pub fn masked(text: &str) -> String {
    let mut scanner = Scanner {
        rest: text.chars().peekable(),
        out: String::with_capacity(text.len().min(LONGEST)),
        previous: None,
    };

    scanner.run();

    shortened(scanner.out.trim())
}

struct Scanner<'a> {
    rest: Peekable<Chars<'a>>,
    out: String,
    previous: Option<char>,
}

impl Scanner<'_> {
    fn run(&mut self) {
        while let Some(character) = self.rest.next() {
            match character {
                '\'' | '"' => self.quoted(character),
                '`' => self.identifier(),
                '$' if self.starts_dollar_quote() => self.dollar_quoted(),
                '-' if self.rest.peek() == Some(&'-') => self.line_comment(),
                '#' => self.line_comment(),
                '/' if self.rest.peek() == Some(&'*') => self.block_comment(),
                digit if digit.is_ascii_digit() && !self.after_word() => self.number(),
                space if space.is_whitespace() => self.space(),
                other => self.keep(other),
            }
        }
    }

    fn quoted(&mut self, quote: char) {
        while let Some(character) = self.rest.next() {
            if character == '\\' {
                self.rest.next();
                continue;
            }

            if character == quote && self.rest.peek() != Some(&quote) {
                break;
            }

            if character == quote {
                self.rest.next();
            }
        }

        self.keep(MASK);
    }

    fn identifier(&mut self) {
        self.keep('`');

        for character in self.rest.by_ref() {
            self.out.push(character);

            if character == '`' {
                break;
            }
        }

        self.previous = Some('`');
    }

    fn starts_dollar_quote(&self) -> bool {
        if self.after_word() {
            return false;
        }

        let mut ahead = self.rest.clone();

        match ahead.next() {
            Some('$') => return true,
            Some(character) if character.is_alphabetic() || character == '_' => {}
            _ => return false,
        }

        ahead
            .find(|character| !(character.is_alphanumeric() || *character == '_'))
            .is_some_and(|character| character == '$')
    }

    fn dollar_quoted(&mut self) {
        let mut tag = String::from('$');

        for character in self.rest.by_ref() {
            tag.push(character);

            if character == '$' {
                break;
            }
        }

        let mut body = String::new();

        for character in self.rest.by_ref() {
            body.push(character);

            if body.ends_with(&tag) {
                break;
            }
        }

        self.keep(MASK);
    }

    fn line_comment(&mut self) {
        for character in self.rest.by_ref() {
            if character == '\n' {
                break;
            }
        }

        self.space();
    }

    fn block_comment(&mut self) {
        self.rest.next();
        let mut depth = 1_usize;

        while depth > 0 {
            let Some(character) = self.rest.next() else {
                break;
            };

            match (character, self.rest.peek()) {
                ('/', Some('*')) => depth += 1,
                ('*', Some('/')) => depth -= 1,
                _ => continue,
            }

            self.rest.next();
        }

        self.space();
    }

    fn number(&mut self) {
        while self
            .rest
            .peek()
            .is_some_and(|character| character.is_ascii_alphanumeric() || "._".contains(*character))
        {
            self.rest.next();
        }

        self.keep(MASK);
    }

    fn space(&mut self) {
        if self.previous.is_some_and(|previous| previous != ' ') {
            self.keep(' ');
        }
    }

    fn keep(&mut self, character: char) {
        self.out.push(character);
        self.previous = Some(character);
    }

    fn after_word(&self) -> bool {
        self.previous
            .is_some_and(|previous| previous.is_alphanumeric() || "_$".contains(previous))
    }
}

fn shortened(text: &str) -> String {
    if text.chars().count() <= LONGEST {
        return text.to_owned();
    }

    let kept: String = text.chars().take(LONGEST).collect();

    format!("{kept}{CUT}")
}

#[cfg(test)]
mod tests {
    use super::masked;

    #[test]
    fn string_and_number_literals_become_question_marks() {
        let text = "SELECT * FROM users WHERE email = 'ann@example.com' AND age > 41";

        assert_eq!(
            masked(text),
            "SELECT * FROM users WHERE email = ? AND age > ?"
        );
    }

    #[test]
    fn names_with_digits_and_placeholders_stay() {
        let text = "SELECT col1, t2.x FROM table3 t2 WHERE id = $1 AND v = ?";

        assert_eq!(masked(text), text);
    }

    #[test]
    fn doubled_and_escaped_quotes_do_not_end_a_string_early() {
        assert_eq!(masked("SELECT 'it''s secret'"), "SELECT ?");
        assert_eq!(masked(r"SELECT 'it\'s secret', 'pw'"), "SELECT ?, ?");
    }

    #[test]
    fn a_backslash_never_lets_a_literal_leak() {
        let text = r"SELECT 'C:\' || password || 'x'";

        assert!(!masked(text).contains("password"));
    }

    #[test]
    fn double_quoted_text_is_masked_because_mysql_reads_it_as_a_string() {
        assert_eq!(masked(r#"SELECT "hunter2""#), "SELECT ?");
    }

    #[test]
    fn backtick_names_stay() {
        assert_eq!(masked("SELECT `select` FROM `t`"), "SELECT `select` FROM `t`");
    }

    #[test]
    fn dollar_quoted_bodies_are_masked() {
        assert_eq!(
            masked("SELECT $$secret$$, $tag$more secret$tag$"),
            "SELECT ?, ?"
        );
    }

    #[test]
    fn comments_are_dropped_with_whatever_they_hold() {
        let text = "SELECT 1 -- password is hunter2\nFROM t /* token abc */ WHERE x # note\n";

        assert_eq!(masked(text), "SELECT ? FROM t WHERE x");
    }

    #[test]
    fn nested_block_comments_are_dropped_whole() {
        assert_eq!(masked("SELECT /* a /* b */ c */ 1"), "SELECT ?");
    }

    #[test]
    fn an_unterminated_string_masks_everything_after_it() {
        assert_eq!(masked("SELECT 'secret and more"), "SELECT ?");
    }

    #[test]
    fn numbers_of_any_shape_are_masked() {
        assert_eq!(masked("SELECT 1.5e10, 0x1F, 1_000"), "SELECT ?, ?, ?");
    }

    #[test]
    fn whitespace_is_folded() {
        assert_eq!(masked("  SELECT\n\t*\n  FROM   t  "), "SELECT * FROM t");
    }

    #[test]
    fn a_long_statement_is_cut() {
        let text = format!("SELECT {}", "x, ".repeat(2000));

        let shown = masked(&text);

        assert!(shown.ends_with(" …"));
        assert!(shown.chars().count() <= 2048 + 2);
    }
}
