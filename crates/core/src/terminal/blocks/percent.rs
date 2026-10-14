pub(super) fn percent_decode(text: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(text.len());
    let mut rest = text;

    while let Some((&byte, after)) = rest.split_first() {
        if let Some(value) = escaped(byte, after) {
            decoded.push(value);
            rest = &after[2..];
            continue;
        }

        decoded.push(byte);
        rest = after;
    }

    decoded
}

fn escaped(byte: u8, after: &[u8]) -> Option<u8> {
    if byte != b'%' {
        return None;
    }

    let pair = after.get(..2)?;

    if !pair.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }

    u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn escapes_become_bytes() {
        assert_eq!(percent_decode(b"echo%20a%3Bb"), b"echo a;b");
    }

    #[test]
    fn a_broken_escape_stays_as_it_is() {
        assert_eq!(percent_decode(b"100%"), b"100%");
        assert_eq!(percent_decode(b"%zz"), b"%zz");
        assert_eq!(percent_decode(b"%+f"), b"%+f");
    }
}
