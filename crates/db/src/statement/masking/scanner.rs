use super::part::Part;

pub(super) struct Scanner<'a> {
    text: &'a [char],
    at: usize,
    parts: Vec<Part>,
    backslash_escapes: bool,
}

impl<'a> Scanner<'a> {
    pub(super) fn parts(text: &'a [char], backslash_escapes: bool) -> Vec<Part> {
        let mut scanner = Self {
            text,
            at: 0,
            parts: vec![Part::Shown; text.len()],
            backslash_escapes,
        };

        scanner.run();

        scanner.parts
    }

    fn run(&mut self) {
        while let Some(character) = self.peek(0) {
            let start = self.at;

            let part = match character {
                '\'' | '"' => self.quoted(character),
                '`' => self.identifier(),
                '$' if self.starts_dollar_quote() => self.dollar_quoted(),
                '-' if self.peek(1) == Some('-') => self.line_comment(),
                '#' => self.line_comment(),
                '/' if self.peek(1) == Some('*') => self.block_comment(),
                digit if digit.is_ascii_digit() && !self.after_word() => self.number(),
                _ => {
                    self.at += 1;
                    Part::Shown
                }
            };

            self.mark(start, part);
        }
    }

    fn quoted(&mut self, quote: char) -> Part {
        self.at += 1;

        while let Some(character) = self.next() {
            if character == '\\' && self.backslash_escapes {
                self.at += 1;
                continue;
            }

            if character != quote {
                continue;
            }

            if self.peek(0) != Some(quote) {
                break;
            }

            self.at += 1;
        }

        Part::Literal
    }

    fn identifier(&mut self) -> Part {
        self.at += 1;
        let start = self.at;

        let closing = self.text[start..]
            .iter()
            .position(|character| *character == '`');

        let Some(length) = closing else {
            self.at = self.text.len();
            return Part::Literal;
        };

        self.at = start + length + 1;

        let plain = self.text[start..start + length]
            .iter()
            .all(|character| character.is_alphanumeric() || "_$ ".contains(*character));

        if plain { Part::Shown } else { Part::Literal }
    }

    fn starts_dollar_quote(&self) -> bool {
        if self.after_word() {
            return false;
        }

        match self.peek(1) {
            Some('$') => return true,
            Some(character) if character.is_alphabetic() || character == '_' => {}
            _ => return false,
        }

        self.text[self.at + 1..]
            .iter()
            .find(|character| !(character.is_alphanumeric() || **character == '_'))
            .is_some_and(|character| *character == '$')
    }

    fn dollar_quoted(&mut self) -> Part {
        let start = self.at;
        self.at += 1;

        while self.next().is_some_and(|character| character != '$') {}

        let tag = &self.text[start..self.at.min(self.text.len())];
        let body = &self.text[self.at.min(self.text.len())..];

        self.at += body
            .windows(tag.len())
            .position(|window| window == tag)
            .map_or(body.len(), |found| found + tag.len());

        Part::Literal
    }

    fn line_comment(&mut self) -> Part {
        while self.next().is_some_and(|character| character != '\n') {}

        Part::Comment
    }

    fn block_comment(&mut self) -> Part {
        self.at += 2;
        let mut depth = 1_usize;

        while depth > 0 {
            let Some(character) = self.next() else {
                break;
            };

            match (character, self.peek(0)) {
                ('/', Some('*')) => depth += 1,
                ('*', Some('/')) => depth -= 1,
                _ => continue,
            }

            self.at += 1;
        }

        Part::Comment
    }

    fn number(&mut self) -> Part {
        self.at += 1;

        while self
            .peek(0)
            .is_some_and(|character| character.is_ascii_alphanumeric() || "._".contains(character))
        {
            self.at += 1;
        }

        Part::Literal
    }

    fn mark(&mut self, start: usize, part: Part) {
        let end = self.at.min(self.text.len());

        for kept in &mut self.parts[start..end] {
            *kept = part;
        }
    }

    fn after_word(&self) -> bool {
        let Some(previous) = self.at.checked_sub(1) else {
            return false;
        };

        self.parts[previous] == Part::Literal
            || self.text[previous].is_alphanumeric()
            || "_$?".contains(self.text[previous])
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.text.get(self.at + ahead).copied()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek(0)?;
        self.at += 1;

        Some(character)
    }
}
