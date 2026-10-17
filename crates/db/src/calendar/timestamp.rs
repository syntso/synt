use std::fmt::{self, Display, Formatter};

use super::{Clock, Date};

const MICROS_PER_DAY: i64 = 86_400_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timestamp {
    date: Date,
    clock: Clock,
}

impl Timestamp {
    pub(crate) fn from_micros_since_1970(micros: i64) -> Self {
        let days = micros.div_euclid(MICROS_PER_DAY);
        let rest = micros.rem_euclid(MICROS_PER_DAY);

        Self {
            date: Date::from_days_since_1970(days),
            clock: Clock::from_micros(rest),
        }
    }
}

impl Display for Timestamp {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.date, self.clock)
    }
}

#[cfg(test)]
mod tests {
    use super::Timestamp;

    fn timestamp(micros: i64) -> String {
        Timestamp::from_micros_since_1970(micros).to_string()
    }

    #[test]
    fn timestamps_join_date_and_time() {
        assert_eq!(
            timestamp(1_700_000_000_123_456),
            "2023-11-14 22:13:20.123456"
        );
        assert_eq!(timestamp(-1), "1969-12-31 23:59:59.999999");
    }
}
