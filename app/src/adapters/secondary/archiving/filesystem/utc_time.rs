use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;
/// Days from 0000-03-01 to 1970-01-01 in the proleptic Gregorian calendar.
const DAYS_FROM_YEAR_ZERO_TO_EPOCH: u64 = 719_468;
const DAYS_PER_ERA: u64 = 146_097;
const YEARS_PER_ERA: u64 = 400;
/// Month index of January when the year starts in March.
const MARCH_BASED_JANUARY: u64 = 10;

/// A wall-clock instant in UTC, to the millisecond; times before the UNIX epoch read as the epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UtcTime {
    year: u64,
    month: u64,
    day: u64,
    hour: u64,
    minute: u64,
    second: u64,
    millis: u32,
}

impl From<SystemTime> for UtcTime {
    fn from(time: SystemTime) -> Self {
        let since_epoch = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = since_epoch.as_secs();
        let (year, month, day) = civil_from_days(secs / SECONDS_PER_DAY);
        let secs_of_day = secs % SECONDS_PER_DAY;
        Self {
            year,
            month,
            day,
            hour: secs_of_day / SECONDS_PER_HOUR,
            minute: secs_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
            second: secs_of_day % SECONDS_PER_MINUTE,
            millis: since_epoch.subsec_millis(),
        }
    }
}

impl UtcTime {
    /// `2026-10-19T10-42-05.123Z`: safe as a file name, sorts by time.
    pub(super) fn folder_name(&self) -> String {
        self.format('-')
    }

    /// `2026-10-19T10:42:05.123Z`.
    pub(super) fn iso_8601(&self) -> String {
        self.format(':')
    }

    fn format(&self, time_separator: char) -> String {
        let s = time_separator;
        format!(
            "{:04}-{:02}-{:02}T{:02}{s}{:02}{s}{:02}.{:03}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millis
        )
    }
}

/// Year, month and day of a day count since the UNIX epoch (H. Hinnant's civil-from-days).
fn civil_from_days(days_since_epoch: u64) -> (u64, u64, u64) {
    let days = days_since_epoch + DAYS_FROM_YEAR_ZERO_TO_EPOCH;
    let era = days / DAYS_PER_ERA;
    let day_of_era = days % DAYS_PER_ERA;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let (month, year_offset) = if month_index < MARCH_BASED_JANUARY {
        (month_index + 3, 0)
    } else {
        (month_index - 9, 1)
    };
    (year_of_era + era * YEARS_PER_ERA + year_offset, month, day)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;

    fn at(secs: u64, millis: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs) + Duration::from_millis(millis)
    }

    #[test]
    fn should_name_folder_by_utc_start_with_millis() {
        // Given
        let started_at = at(1_792_406_525, 123);
        // When
        let name = UtcTime::from(started_at).folder_name();
        // Then
        assert_eq!(name, "2026-10-19T10-42-05.123Z");
    }

    #[test]
    fn should_format_started_at_as_iso_8601() {
        // Given
        let started_at = at(1_792_406_525, 123);
        // When
        let text = UtcTime::from(started_at).iso_8601();
        // Then
        assert_eq!(text, "2026-10-19T10:42:05.123Z");
    }

    #[test]
    fn should_name_folder_on_leap_day() {
        // Given
        let started_at = at(1_709_164_800 + 23 * 3600 + 59 * 60 + 59, 999);
        // When
        let name = UtcTime::from(started_at).folder_name();
        // Then
        assert_eq!(name, "2024-02-29T23-59-59.999Z");
    }

    #[test]
    fn should_name_folder_at_epoch() {
        // Given
        let started_at = UNIX_EPOCH;
        // When
        let name = UtcTime::from(started_at).folder_name();
        // Then
        assert_eq!(name, "1970-01-01T00-00-00.000Z");
    }

    #[test]
    fn should_sort_folder_names_by_time() {
        // Given
        let earlier = UtcTime::from(at(1_709_164_800, 0)).folder_name();
        // When
        let later = UtcTime::from(at(1_792_406_525, 0)).folder_name();
        // Then
        assert!(earlier < later);
    }
}
