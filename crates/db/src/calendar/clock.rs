use std::fmt::{self, Display, Formatter};

const MICROS_PER_SECOND: i64 = 1_000_000;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Clock {
    micros: i64,
}

impl Clock {
    pub(crate) fn from_micros(micros: i64) -> Self {
        Self { micros }
    }

    pub(crate) fn new(hours: i64, minutes: i64, seconds: i64, micros: i64) -> Self {
        let seconds = hours * SECONDS_PER_HOUR + minutes * SECONDS_PER_MINUTE + seconds;

        Self::from_micros(seconds * MICROS_PER_SECOND + micros)
    }
}

impl Display for Clock {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let seconds = self.micros.div_euclid(MICROS_PER_SECOND);
        let fraction = self.micros.rem_euclid(MICROS_PER_SECOND);

        let hours = seconds / SECONDS_PER_HOUR;
        let minutes = seconds / SECONDS_PER_MINUTE % 60;
        let seconds = seconds % SECONDS_PER_MINUTE;

        write!(formatter, "{hours:02}:{minutes:02}:{seconds:02}")?;

        if fraction == 0 {
            return Ok(());
        }

        let fraction = format!("{fraction:06}");

        write!(formatter, ".{}", fraction.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::Clock;

    #[test]
    fn times_keep_only_the_fraction_they_have() {
        assert_eq!(Clock::from_micros(0).to_string(), "00:00:00");
        assert_eq!(Clock::from_micros(45_296_500_000).to_string(), "12:34:56.5");
    }

    #[test]
    fn hours_go_past_a_day_for_durations() {
        assert_eq!(Clock::new(26, 3, 4, 0).to_string(), "26:03:04");
        assert_eq!(Clock::new(9, 5, 0, 250_000).to_string(), "09:05:00.25");
    }
}
