mod part;
mod scanner;

use self::part::Part;
use self::scanner::Scanner;

const MASK: char = '?';
const LONGEST: usize = 2048;
const CUT: &str = " …";

pub fn masked(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();

    let standard = Scanner::parts(&characters, false);
    let escaped = Scanner::parts(&characters, true);
    let parts = standard
        .iter()
        .zip(&escaped)
        .map(|(one, other)| one.stricter(*other));

    let shown = rendered(characters.iter().copied().zip(parts));

    shortened(shown.trim())
}

fn rendered(parts: impl Iterator<Item = (char, Part)>) -> String {
    let mut out = String::new();
    let mut previous = Part::Shown;

    for (character, part) in parts {
        match part {
            Part::Literal if previous != Part::Literal => out.push(MASK),
            Part::Literal => {}
            Part::Shown if !character.is_whitespace() => out.push(character),
            Part::Shown | Part::Comment => space(&mut out),
        }

        previous = part;
    }

    out
}

fn space(out: &mut String) {
    if out.chars().last().is_some_and(|last| last != ' ') {
        out.push(' ');
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
        let text = "SELECT col1, t2.x FROM table3 t2 WHERE id = $1 AND v = ? AND w = ?2";

        assert_eq!(masked(text), text);
    }

    #[test]
    fn doubled_quotes_do_not_end_a_string_early() {
        assert_eq!(masked("SELECT 'it''s secret', 'pw'"), "SELECT ?, ?");
    }

    #[test]
    fn a_backslash_never_lets_a_literal_leak() {
        let text = r"SELECT 'C:\' || password || 'x'";

        assert!(!masked(text).contains("password"));
    }

    #[test]
    fn a_backslash_at_the_end_of_a_standard_string_does_not_unmask_the_next_one() {
        let text = r"SELECT * FROM users WHERE path = 'C:\' AND password = 'hunter2'";

        let shown = masked(text);

        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.starts_with("SELECT * FROM users WHERE path = ?"));
    }

    #[test]
    fn an_escaped_quote_of_mysql_does_not_unmask_what_follows() {
        let shown = masked(r"SELECT 'it\'s secret', 'pw'");

        assert!(!shown.contains("secret"), "{shown}");
        assert!(!shown.contains("pw"), "{shown}");
    }

    #[test]
    fn postgres_escape_strings_are_masked() {
        let shown = masked(r"SELECT E'a\'b', 'c'");

        assert!(!shown.contains('b'), "{shown}");
        assert!(!shown.contains('c'), "{shown}");
    }

    #[test]
    fn double_quoted_text_is_masked_because_mysql_reads_it_as_a_string() {
        assert_eq!(masked(r#"SELECT "hunter2""#), "SELECT ?");
    }

    #[test]
    fn backtick_names_stay() {
        assert_eq!(
            masked("SELECT `select` FROM `t`"),
            "SELECT `select` FROM `t`"
        );
    }

    #[test]
    fn a_backtick_name_that_is_more_than_a_word_is_masked() {
        assert_eq!(masked("SELECT `d'"), "SELECT ?");
        assert_eq!(masked("SELECT `it's` FROM t"), "SELECT ? FROM t");
        assert_eq!(masked("SELECT `a--b`, `c/*d`"), "SELECT ?, ?");
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
