use std::fmt::{self, Display, Formatter};

const DAYS_FROM_YEAR_ZERO_TO_1970: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Date {
    year: i64,
    month: i64,
    day: i64,
}

impl Date {
    pub(crate) fn new(year: i64, month: i64, day: i64) -> Self {
        Self { year, month, day }
    }

    pub(crate) fn from_days_since_1970(days: i64) -> Self {
        let shifted = days + DAYS_FROM_YEAR_ZERO_TO_1970;
        let era = shifted.div_euclid(DAYS_PER_ERA);
        let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;

        let day = day_of_year - (153 * month_index + 2) / 5 + 1;
        let month = if month_index < 10 {
            month_index + 3
        } else {
            month_index - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);

        Self::new(year, month, day)
    }
}

impl Display for Date {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Date;

    fn date(days: i64) -> String {
        Date::from_days_since_1970(days).to_string()
    }

    #[test]
    fn days_count_from_the_unix_epoch() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(10_957), "2000-01-01");
        assert_eq!(date(-1), "1969-12-31");
        assert_eq!(date(19_782), "2024-02-29");
    }

    #[test]
    fn a_date_given_by_its_parts_is_written_the_same_way() {
        assert_eq!(Date::new(2026, 10, 4).to_string(), "2026-10-04");
    }
}
