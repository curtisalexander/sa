//! Parsing and formatting of the `--for` duration.

use std::time::Duration;

/// Parses `45s`, `90m`, `2h`, `1h30m`, `1d 12h` into a [`Duration`].
///
/// A unit is required: `--for 30` is ambiguous enough that guessing wrong means
/// either a machine that sleeps mid-job or one that stays awake all night.
pub fn parse(input: &str) -> Result<Duration, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err(String::from("empty duration"));
    }

    let mut total: u64 = 0;
    let mut digits: Option<u64> = None;

    for ch in text.chars() {
        match ch {
            ' ' | '_' => continue,
            '0'..='9' => {
                let digit = u64::from(ch as u8 - b'0');
                let value = digits.unwrap_or(0);
                digits = Some(
                    value
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(digit))
                        .ok_or_else(|| overflow(input))?,
                );
            }
            _ => {
                let seconds = match ch.to_ascii_lowercase() {
                    's' => 1,
                    'm' => 60,
                    'h' => 60 * 60,
                    'd' => 24 * 60 * 60,
                    _ => return Err(bad_unit(input, ch)),
                };
                let value = digits.take().ok_or_else(|| missing_value(input, ch))?;
                total = value
                    .checked_mul(seconds)
                    .and_then(|v| total.checked_add(v))
                    .ok_or_else(|| overflow(input))?;
            }
        }
    }

    if digits.is_some() {
        return Err(no_unit(input));
    }
    if total == 0 {
        return Err(format!("duration `{input}` must be greater than zero"));
    }
    Ok(Duration::from_secs(total))
}

fn no_unit(input: &str) -> String {
    format!("duration `{input}` needs a unit, e.g. 30s, 45m, 2h, 1h30m")
}

fn bad_unit(input: &str, unit: char) -> String {
    format!("duration `{input}` has an unknown unit `{unit}`; use s, m, h or d")
}

fn missing_value(input: &str, unit: char) -> String {
    format!("duration `{input}` has a `{unit}` with no number in front of it")
}

fn overflow(input: &str) -> String {
    format!("duration `{input}` is too large")
}

/// `1d 4h 30m 5s`, dropping empty units. Never empty: sub-second is `0s`.
pub fn humanize(duration: Duration) -> String {
    let total = duration.as_secs();
    let parts = [
        (total / 86_400, 'd'),
        (total % 86_400 / 3_600, 'h'),
        (total % 3_600 / 60, 'm'),
        (total % 60, 's'),
    ];

    let text = parts
        .iter()
        .filter(|(value, _)| *value > 0)
        .map(|(value, unit)| format!("{value}{unit}"))
        .collect::<Vec<_>>()
        .join(" ");

    if text.is_empty() {
        return String::from("0s");
    }
    text
}

/// `01:23:45`, with hours allowed to grow past 24.
pub fn clock(duration: Duration) -> String {
    let total = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        total / 3_600,
        total % 3_600 / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn parses_a_single_unit() {
        assert_eq!(parse("45s").unwrap(), secs(45));
        assert_eq!(parse("90m").unwrap(), secs(5_400));
        assert_eq!(parse("2h").unwrap(), secs(7_200));
        assert_eq!(parse("1d").unwrap(), secs(86_400));
    }

    #[test]
    fn parses_compound_units() {
        assert_eq!(parse("1h30m").unwrap(), secs(5_400));
        assert_eq!(parse("2h30m15s").unwrap(), secs(9_015));
        assert_eq!(parse("1d 12h").unwrap(), secs(129_600));
    }

    #[test]
    fn ignores_case_and_padding() {
        assert_eq!(parse("  1H30M  ").unwrap(), secs(5_400));
    }

    #[test]
    fn rejects_a_bare_number() {
        let err = parse("30").unwrap_err();
        assert!(err.contains("needs a unit"), "{err}");
    }

    #[test]
    fn rejects_an_unknown_unit() {
        assert!(parse("5w").unwrap_err().contains("unknown unit"));
    }

    #[test]
    fn rejects_a_unit_without_a_number() {
        assert!(parse("h").unwrap_err().contains("no number"));
        assert!(parse("1h m").unwrap_err().contains("no number"));
    }

    #[test]
    fn rejects_zero_and_empty() {
        assert!(parse("0s").unwrap_err().contains("greater than zero"));
        assert!(parse("").unwrap_err().contains("empty"));
    }

    #[test]
    fn rejects_overflow_instead_of_wrapping() {
        assert!(
            parse("99999999999999999999d")
                .unwrap_err()
                .contains("large")
        );
        assert!(parse("9999999999999999999d").unwrap_err().contains("large"));
    }

    #[test]
    fn humanize_drops_empty_units() {
        assert_eq!(humanize(secs(0)), "0s");
        assert_eq!(humanize(secs(45)), "45s");
        assert_eq!(humanize(secs(5_400)), "1h 30m");
        assert_eq!(humanize(secs(90_061)), "1d 1h 1m 1s");
    }

    #[test]
    fn clock_pads_and_grows_hours() {
        assert_eq!(clock(secs(0)), "00:00:00");
        assert_eq!(clock(secs(83)), "00:01:23");
        assert_eq!(clock(secs(360_000)), "100:00:00");
    }

    #[test]
    fn round_trips_what_it_prints() {
        for total in [1, 59, 60, 3_599, 3_600, 5_400, 90_061] {
            let text = humanize(secs(total)).replace(' ', "");
            assert_eq!(parse(&text).unwrap(), secs(total), "{text}");
        }
    }
}
