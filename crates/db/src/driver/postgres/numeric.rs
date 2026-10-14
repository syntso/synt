use std::fmt::Write;

const NEGATIVE: u16 = 0x4000;
const NOT_A_NUMBER: u16 = 0xC000;
const POSITIVE_INFINITY: u16 = 0xD000;
const NEGATIVE_INFINITY: u16 = 0xF000;
const HEADER: usize = 8;

pub(super) fn numeric(bytes: &[u8]) -> Option<String> {
    let count = usize::try_from(read_i16(bytes, 0)?).ok()?;
    let weight = i64::from(read_i16(bytes, 2)?);
    let sign = read_u16(bytes, 4)?;
    let scale = usize::from(read_u16(bytes, 6)?);

    let digits = (0..count)
        .map(|index| read_i16(bytes, HEADER + 2 * index))
        .collect::<Option<Vec<i16>>>()?;

    match sign {
        NOT_A_NUMBER => return Some("NaN".to_owned()),
        POSITIVE_INFINITY => return Some("Infinity".to_owned()),
        NEGATIVE_INFINITY => return Some("-Infinity".to_owned()),
        _ => {}
    }

    let whole = whole_part(&digits, weight);
    let fraction = fraction_part(&digits, weight, scale);
    let minus = if sign == NEGATIVE { "-" } else { "" };

    if scale == 0 {
        return Some(format!("{minus}{whole}"));
    }

    Some(format!("{minus}{whole}.{fraction}"))
}

fn whole_part(digits: &[i16], weight: i64) -> String {
    if weight < 0 {
        return "0".to_owned();
    }

    let mut text = String::new();

    for position in 0..=weight {
        let digit = group(digits, position);

        let written = if text.is_empty() {
            write!(text, "{digit}")
        } else {
            write!(text, "{digit:04}")
        };
        written.expect("writing to a String cannot fail");
    }

    text
}

fn fraction_part(digits: &[i16], weight: i64, scale: usize) -> String {
    let mut text = String::new();
    let mut position = weight + 1;

    while text.len() < scale {
        write!(text, "{:04}", group(digits, position)).expect("writing to a String cannot fail");
        position += 1;
    }

    text.truncate(scale);
    text
}

fn group(digits: &[i16], position: i64) -> i16 {
    usize::try_from(position)
        .ok()
        .and_then(|index| digits.get(index))
        .copied()
        .unwrap_or(0)
}

fn read_i16(bytes: &[u8], at: usize) -> Option<i16> {
    let pair = bytes.get(at..at + 2)?;

    Some(i16::from_be_bytes([pair[0], pair[1]]))
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let pair = bytes.get(at..at + 2)?;

    Some(u16::from_be_bytes([pair[0], pair[1]]))
}

#[cfg(test)]
mod tests {
    use super::numeric;

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
