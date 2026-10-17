use super::{is_format_character, percent_decode};

const SHELL_INTEGRATION: &[u8] = b"133";
const COMMAND_LINE: &[u8] = b"cmdline=";
const COMMAND_LINE_URL: &[u8] = b"cmdline_url=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Mark {
    PromptStart,
    CommandStart,
    OutputStart { command: Option<String> },
    CommandEnd { exit: Option<i32> },
}

impl Mark {
    pub(crate) fn parse(params: &[&[u8]]) -> Option<Self> {
        let [code, kind, rest @ ..] = params else {
            return None;
        };

        if *code != SHELL_INTEGRATION {
            return None;
        }

        match *kind {
            b"A" => Some(Self::PromptStart),
            b"B" => Some(Self::CommandStart),
            b"C" => Some(Self::OutputStart {
                command: command_line(rest),
            }),
            b"D" => Some(Self::CommandEnd {
                exit: exit_code(rest),
            }),
            _ => None,
        }
    }
}

fn command_line(params: &[&[u8]]) -> Option<String> {
    let position = params
        .iter()
        .position(|param| param.starts_with(COMMAND_LINE) || param.starts_with(COMMAND_LINE_URL))?;

    let first = params[position];
    let bytes = match first.strip_prefix(COMMAND_LINE_URL) {
        Some(encoded) => percent_decode(encoded),
        None => joined(&first[COMMAND_LINE.len()..], &params[position + 1..]),
    };

    readable(&bytes)
}

fn joined(first: &[u8], others: &[&[u8]]) -> Vec<u8> {
    let mut bytes = first.to_vec();

    for other in others {
        bytes.push(b';');
        bytes.extend_from_slice(other);
    }

    bytes
}

fn readable(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let cleaned: String = text
        .chars()
        .filter(|character| !is_format_character(*character))
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let trimmed = cleaned.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn exit_code(params: &[&[u8]]) -> Option<i32> {
    let first = params.first()?;
    let text = std::str::from_utf8(first).ok()?;

    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::Mark;

    fn parse(text: &str) -> Option<Mark> {
        let params: Vec<&[u8]> = text.split(';').map(str::as_bytes).collect();
        Mark::parse(&params)
    }

    #[test]
    fn the_four_marks_are_read() {
        assert_eq!(parse("133;A"), Some(Mark::PromptStart));
        assert_eq!(parse("133;B"), Some(Mark::CommandStart));
        assert_eq!(parse("133;C"), Some(Mark::OutputStart { command: None }));
        assert_eq!(parse("133;D;2"), Some(Mark::CommandEnd { exit: Some(2) }));
        assert_eq!(parse("133;D"), Some(Mark::CommandEnd { exit: None }));
    }

    #[test]
    fn the_command_line_keeps_its_semicolons() {
        let mark = parse("133;C;cmdline=echo a; echo b");

        assert_eq!(
            mark,
            Some(Mark::OutputStart {
                command: Some("echo a; echo b".to_owned())
            })
        );
    }

    #[test]
    fn an_encoded_command_line_is_decoded() {
        let mark = parse("133;C;cmdline_url=ls%20-l%3B%20false");

        assert_eq!(
            mark,
            Some(Mark::OutputStart {
                command: Some("ls -l; false".to_owned())
            })
        );
    }

    #[test]
    fn invisible_formatting_is_removed_from_the_command_line() {
        let mark = parse("133;C;cmdline=rm \u{202e}fdp.exe\u{200b} \u{2066}x\u{2069}");

        assert_eq!(
            mark,
            Some(Mark::OutputStart {
                command: Some("rm fdp.exe x".to_owned())
            })
        );
    }

    #[test]
    fn other_sequences_are_not_marks() {
        assert_eq!(parse("0;title"), None);
        assert_eq!(parse("133;Z"), None);
        assert_eq!(parse("133"), None);
    }
}
