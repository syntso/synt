use std::fmt::{self, Write};

const DIGITS: &[u8; 16] = b"0123456789abcdef";

pub(crate) fn write(bytes: &[u8], out: &mut impl Write) -> fmt::Result {
    for byte in bytes {
        out.write_char(char::from(DIGITS[usize::from(byte >> 4)]))?;
        out.write_char(char::from(DIGITS[usize::from(byte & 0x0f)]))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write;

    #[test]
    fn writes_lowercase_pairs() {
        let mut text = String::new();
        write(&[0x00, 0x0f, 0xa5, 0xff], &mut text).unwrap();

        assert_eq!(text, "000fa5ff");
    }
}
