use std::fmt::{self, Write};

const DIGITS: &[u8; 16] = b"0123456789abcdef";

pub(crate) struct Hex<'a>(pub(crate) &'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            f.write_char(char::from(DIGITS[usize::from(byte >> 4)]))?;
            f.write_char(char::from(DIGITS[usize::from(byte & 0x0f)]))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Hex;

    #[test]
    fn writes_lowercase_pairs() {
        assert_eq!(Hex(&[0x00, 0x0f, 0xa5, 0xff]).to_string(), "000fa5ff");
    }
}
