use super::CodecError;

pub(crate) struct Decoder<'a> {
    input: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, CodecError> {
        let [value] = self.take_array::<1>()?;
        Ok(value)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_be_bytes(self.take_array()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_be_bytes(self.take_array()?))
    }

    pub(crate) fn len(&mut self) -> Result<usize, CodecError> {
        let len = u32::from_be_bytes(self.take_array()?);
        usize::try_from(len).map_err(|_| CodecError("length does not fit in memory"))
    }

    pub(crate) fn bytes(&mut self) -> Result<&'a [u8], CodecError> {
        let len = self.len()?;
        self.take(len)
    }

    pub(crate) fn str(&mut self) -> Result<&'a str, CodecError> {
        let bytes = self.bytes()?;
        std::str::from_utf8(bytes).map_err(|_| CodecError("text is not utf-8"))
    }

    pub(crate) fn finish(self) -> Result<(), CodecError> {
        if self.input.is_empty() {
            Ok(())
        } else {
            Err(CodecError("trailing bytes"))
        }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if len > self.input.len() {
            return Err(CodecError("unexpected end of input"));
        }

        let (taken, rest) = self.input.split_at(len);
        self.input = rest;

        Ok(taken)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let bytes = self.take(N)?;
        bytes
            .try_into()
            .map_err(|_| CodecError("unexpected end of input"))
    }
}

#[cfg(test)]
mod tests {
    use super::Decoder;
    use crate::codec::Encoder;

    #[test]
    fn reads_back_what_was_written() {
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out);
        encoder.u64(42);
        encoder.i64(-7);
        encoder.str("synt");

        let mut decoder = Decoder::new(&out);

        assert_eq!(decoder.u64(), Ok(42));
        assert_eq!(decoder.i64(), Ok(-7));
        assert_eq!(decoder.str(), Ok("synt"));
        assert!(decoder.finish().is_ok());
    }

    #[test]
    fn refuses_lengths_beyond_the_input() {
        let mut decoder = Decoder::new(&[0, 0, 0, 9, 1]);
        assert!(decoder.bytes().is_err());
    }

    #[test]
    fn refuses_trailing_bytes() {
        let mut decoder = Decoder::new(&[1, 2]);
        decoder.u8().ok();

        assert!(decoder.finish().is_err());
    }
}
