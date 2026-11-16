//! Parse `--since`: a relative window (`180d`, `12w`, `36h`), a date
//! (`2026-05-01`, midnight UTC) or an RFC 3339 timestamp.

use anyhow::{Context, Result, bail};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

pub fn parse_since(s: &str, now: Timestamp) -> Result<Timestamp> {
    let s = s.trim();
    if let Ok(t) = s.parse::<Timestamp>() {
        return Ok(t);
    }
    if let Ok(d) = s.parse::<Date>() {
        return Ok(d.to_zoned_datetime(TimeZone::UTC)?.timestamp());
    }
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .with_context(|| format!("--since {s:?}: missing unit (h, d or w)"))?;
    let (n, unit) = s.split_at(split);
    let n: i64 = n
        .parse()
        .with_context(|| format!("--since {s:?}: expected a number before the unit"))?;
    let hours = match unit {
        "h" => n,
        "d" => n * 24,
        "w" => n * 24 * 7,
        _ => bail!("--since {s:?}: unit must be h, d or w, or give a date like 2026-05-01"),
    };
    Ok(now.checked_sub(SignedDuration::from_hours(hours))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Timestamp {
        "2026-11-16T12:00:00Z".parse().unwrap()
    }

    fn at(s: &str) -> String {
        parse_since(s, now()).unwrap().to_string()
    }

    #[test]
    fn relative() {
        assert_eq!(at("36h"), "2026-11-15T00:00:00Z");
        assert_eq!(at("2d"), "2026-11-14T12:00:00Z");
        assert_eq!(at("1w"), "2026-11-09T12:00:00Z");
    }

    #[test]
    fn absolute() {
        assert_eq!(at("2026-05-01"), "2026-05-01T00:00:00Z");
        assert_eq!(at("2026-05-01T08:00:00+08:00"), "2026-05-01T00:00:00Z");
    }

    #[test]
    fn rejects() {
        ["", "d", "10", "10mo", "x1d"]
            .iter()
            .for_each(|bad| assert!(parse_since(bad, now()).is_err(), "{bad}"));
    }
}
