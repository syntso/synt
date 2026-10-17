const FORMAT_RANGES: &[(char, char)] = &[
    ('\u{00ad}', '\u{00ad}'),
    ('\u{0600}', '\u{0605}'),
    ('\u{061c}', '\u{061c}'),
    ('\u{06dd}', '\u{06dd}'),
    ('\u{070f}', '\u{070f}'),
    ('\u{0890}', '\u{0891}'),
    ('\u{08e2}', '\u{08e2}'),
    ('\u{180e}', '\u{180e}'),
    ('\u{200b}', '\u{200f}'),
    ('\u{202a}', '\u{202e}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206f}'),
    ('\u{feff}', '\u{feff}'),
    ('\u{fff9}', '\u{fffb}'),
    ('\u{110bd}', '\u{110bd}'),
    ('\u{110cd}', '\u{110cd}'),
    ('\u{13430}', '\u{1343f}'),
    ('\u{1bca0}', '\u{1bca3}'),
    ('\u{1d173}', '\u{1d17a}'),
    ('\u{e0001}', '\u{e0001}'),
    ('\u{e0020}', '\u{e007f}'),
];

pub(super) fn is_format_character(character: char) -> bool {
    FORMAT_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&character))
}

#[cfg(test)]
mod tests {
    use super::is_format_character;

    #[test]
    fn bidi_controls_and_zero_width_characters_are_format_characters() {
        for character in [
            '\u{202e}', '\u{2066}', '\u{2069}', '\u{200b}', '\u{200d}', '\u{feff}',
        ] {
            assert!(is_format_character(character));
        }
    }

    #[test]
    fn visible_characters_are_not() {
        for character in ['a', ' ', '\u{00e9}', '\u{4e2d}', '\u{1f600}'] {
            assert!(!is_format_character(character));
        }
    }
}
