const MICROS_PER_SECOND: i64 = 1_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

pub(crate) fn date(days_since_1970: i64) -> String {
    let (year, month, day) = civil(days_since_1970);

    format!("{year:04}-{month:02}-{day:02}")
}

pub(crate) fn time_of_day(micros: i64) -> String {
    let seconds = micros.div_euclid(MICROS_PER_SECOND);
    let fraction = micros.rem_euclid(MICROS_PER_SECOND);

    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    let whole = format!("{hours:02}:{minutes:02}:{seconds:02}");

    if fraction == 0 {
        return whole;
    }

    let fraction = format!("{fraction:06}");

    format!("{whole}.{}", fraction.trim_end_matches('0'))
}

pub(crate) fn timestamp(micros_since_1970: i64) -> String {
    let day_micros = SECONDS_PER_DAY * MICROS_PER_SECOND;
    let days = micros_since_1970.div_euclid(day_micros);
    let rest = micros_since_1970.rem_euclid(day_micros);

    format!("{} {}", date(days), time_of_day(rest))
}

fn civil(days_since_1970: i64) -> (i64, i64, i64) {
    let shifted = days_since_1970 + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
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

    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{date, time_of_day, timestamp};

    #[test]
    fn days_count_from_the_unix_epoch() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(10_957), "2000-01-01");
        assert_eq!(date(-1), "1969-12-31");
        assert_eq!(date(19_782), "2024-02-29");
    }

    #[test]
    fn times_keep_only_the_fraction_they_have() {
        assert_eq!(time_of_day(0), "00:00:00");
        assert_eq!(time_of_day(45_296_500_000), "12:34:56.5");
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
