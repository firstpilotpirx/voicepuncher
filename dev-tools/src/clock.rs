//! Times as the former runner printed them: seconds with one decimal, clock times, ISO 8601 with the local
//! offset, and numbers in a journal line as Perl's JSON wrote them.

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, Zoned};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the epoch, with their fraction.
pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64())
}

/// `%.1f` of a number of seconds.
pub fn tenths(seconds: f64) -> String {
    format!("{seconds:.1}")
}

/// A number rounded as `sprintf("%.1f")` rounds it.
pub fn round1(value: f64) -> f64 {
    tenths(value).parse().unwrap_or(value)
}

/// The local clock time, `HH:MM:SS`.
pub fn clock_time() -> String {
    Zoned::now().strftime("%H:%M:%S").to_string()
}

/// The local zone (`TZ`, else the system's).
pub fn local_zone() -> TimeZone {
    TimeZone::system()
}

/// An epoch time in `zone` as `YYYY-MM-DDTHH:MM:SS+HH:MM`, its fraction dropped.
pub fn iso(seconds: f64, zone: &TimeZone) -> String {
    SignedDuration::try_from_secs_f64(seconds.trunc())
        .ok()
        .and_then(|since| Timestamp::from_duration(since).ok())
        .map(|at| {
            at.to_zoned(zone.clone())
                .strftime("%Y-%m-%dT%H:%M:%S%:z")
                .to_string()
        })
        .unwrap_or_default()
}

/// The epoch seconds of an ISO time with an offset (`2026-10-23T10:00:00+02:00`).
pub fn epoch(text: &str) -> Option<f64> {
    let at: Timestamp = text.parse().ok()?;
    Some(at.as_duration().as_secs_f64())
}

/// A number as Perl's JSON wrote it: a whole number without a fraction.
pub fn number(value: f64) -> Value {
    let whole = format!("{value:.0}");
    if (value - value.trunc()).abs() < f64::EPSILON && value.abs() < 1e15 {
        if let Ok(integer) = whole.parse::<i64>() {
            return Value::from(integer);
        }
    }
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_seconds_as_the_former_runner_did() {
        assert_eq!(tenths(5.25), "5.2");
        assert_eq!(tenths(94.75), "94.8");
        assert_eq!(tenths(1110.3 - 990.05), "120.2");
    }

    #[test]
    fn should_write_whole_numbers_without_a_fraction() {
        assert_eq!(number(40.0).to_string(), "40");
        assert_eq!(number(61.75).to_string(), "61.75");
        assert_eq!(number(round1(94.75)).to_string(), "94.8");
    }

    #[test]
    fn should_write_an_iso_time_with_the_offset() {
        assert_eq!(iso(1005.25, &TimeZone::UTC), "1970-01-01T00:16:45+00:00");
        assert_eq!(epoch("1970-01-01T02:16:45+02:00"), Some(1005.0));
    }
}
