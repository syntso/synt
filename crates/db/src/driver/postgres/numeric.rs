use std::fmt::{self, Display, Formatter};

const NEGATIVE: u16 = 0x4000;
const NOT_A_NUMBER: u16 = 0xC000;
const POSITIVE_INFINITY: u16 = 0xD000;
const NEGATIVE_INFINITY: u16 = 0xF000;
const HEADER: usize = 8;

pub(super) struct Numeric {
    weight: i64,
    sign: u16,
    scale: usize,
    groups: Vec<i16>,
}

impl Numeric {
    pub(super) fn read(bytes: &[u8]) -> Option<Self> {
        let count = usize::try_from(read_i16(bytes, 0)?).ok()?;
        let weight = i64::from(read_i16(bytes, 2)?);
        let sign = read_u16(bytes, 4)?;
        let scale = usize::from(read_u16(bytes, 6)?);

        let groups = (0..count)
            .map(|index| read_i16(bytes, HEADER + 2 * index))
            .collect::<Option<Vec<i16>>>()?;

        Some(Self {
            weight,
            sign,
            scale,
            groups,
        })
    }

    fn write_whole(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.weight < 0 {
            return formatter.write_str("0");
        }

        write!(formatter, "{}", self.group(0))?;

        for position in 1..=self.weight {
            write!(formatter, "{:04}", self.group(position))?;
        }

        Ok(())
    }

    fn write_fraction(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut left = self.scale;
        let mut position = self.weight + 1;

        while left > 0 {
            let digits = format!("{:04}", self.group(position));
            let shown = left.min(digits.len());

            formatter.write_str(&digits[..shown])?;

            left -= shown;
            position += 1;
        }

        Ok(())
    }

    fn group(&self, position: i64) -> i16 {
        usize::try_from(position)
            .ok()
            .and_then(|index| self.groups.get(index))
            .copied()
            .unwrap_or(0)
    }
}

impl Display for Numeric {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.sign {
            NOT_A_NUMBER => return formatter.write_str("NaN"),
            POSITIVE_INFINITY => return formatter.write_str("Infinity"),
            NEGATIVE_INFINITY => return formatter.write_str("-Infinity"),
            NEGATIVE => formatter.write_str("-")?,
            _ => {}
        }

        self.write_whole(formatter)?;

        if self.scale == 0 {
            return Ok(());
        }

        formatter.write_str(".")?;
        self.write_fraction(formatter)
    }
}

fn read_i16(bytes: &[u8], at: usize) -> Option<i16> {
    let pair = bytes.get(at..at + 2)?.try_into().ok()?;

    Some(i16::from_be_bytes(pair))
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let pair = bytes.get(at..at + 2)?.try_into().ok()?;

    Some(u16::from_be_bytes(pair))
}

#[cfg(test)]
mod tests {
    use super::Numeric;

    fn numeric(bytes: &[u8]) -> Option<String> {
        Numeric::read(bytes).map(|number| number.to_string())
    }

    fn encoded(weight: i16, sign: u16, scale: u16, digits: &[i16]) -> Vec<u8> {
        let count = i16::try_from(digits.len()).expect("count");
        let mut bytes = Vec::new();

        bytes.extend(count.to_be_bytes());
        bytes.extend(weight.to_be_bytes());
        bytes.extend(sign.to_be_bytes());
        bytes.extend(scale.to_be_bytes());
        for digit in digits {
            bytes.extend(digit.to_be_bytes());
        }

        bytes
    }

    #[test]
    fn whole_numbers_join_their_base_10000_groups() {
        assert_eq!(
            numeric(&encoded(1, 0, 0, &[12, 3456])).as_deref(),
            Some("123456")
        );
    }

    #[test]
    fn fractions_keep_the_declared_scale() {
        assert_eq!(
            numeric(&encoded(0, 0, 2, &[3, 1400])).as_deref(),
            Some("3.14")
        );
        assert_eq!(numeric(&encoded(-1, 0, 4, &[5])).as_deref(), Some("0.0005"));
        assert_eq!(
            numeric(&encoded(-2, 0, 6, &[1200])).as_deref(),
            Some("0.000012")
        );
    }

    #[test]
    fn signs_and_special_values_are_spelled_out() {
        assert_eq!(
            numeric(&encoded(0, 0x4000, 1, &[7, 5000])).as_deref(),
            Some("-7.5")
        );
        assert_eq!(numeric(&encoded(0, 0xC000, 0, &[])).as_deref(), Some("NaN"));
        assert_eq!(numeric(&encoded(0, 0, 0, &[])).as_deref(), Some("0"));
    }

    #[test]
    fn truncated_bytes_are_not_a_number() {
        assert_eq!(numeric(&[0, 2, 0]), None);
    }
}
