//! Parse `--since`: a relative window (`180d`, `12w`, `36h`) or a date
//! (`2026-05-01`) or an RFC 3339 timestamp.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, NaiveDate, Utc};

pub fn parse_since(s: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Ok(t.with_timezone(&Utc));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).expect("midnight is valid").and_utc());
    }
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .with_context(|| format!("--since {s:?}: missing unit (h, d or w)"))?;
    let (n, unit) = s.split_at(split);
    let n: i64 = n
        .parse()
        .with_context(|| format!("--since {s:?}: expected a number before the unit"))?;
    let d = match unit {
        "h" => Duration::hours(n),
        "d" => Duration::days(n),
        "w" => Duration::weeks(n),
        _ => bail!("--since {s:?}: unit must be h, d or w, or give a date like 2026-05-01"),
    };
    Ok(now - d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-16T12:00:00Z")
            .unwrap()
            .into()
    }

    #[test]
    fn relative() {
        assert_eq!(
            parse_since("36h", now()).unwrap().to_rfc3339(),
            "2026-11-15T00:00:00+00:00"
        );
        assert_eq!(
            parse_since("2d", now()).unwrap().to_rfc3339(),
            "2026-11-14T12:00:00+00:00"
        );
        assert_eq!(
            parse_since("1w", now()).unwrap().to_rfc3339(),
            "2026-11-09T12:00:00+00:00"
        );
    }

    #[test]
    fn absolute() {
        assert_eq!(
            parse_since("2026-05-01", now()).unwrap().to_rfc3339(),
            "2026-05-01T00:00:00+00:00"
        );
        assert_eq!(
            parse_since("2026-05-01T08:00:00+08:00", now())
                .unwrap()
                .to_rfc3339(),
            "2026-05-01T00:00:00+00:00"
        );
    }

    #[test]
    fn rejects() {
        for bad in ["", "d", "10", "10mo", "x1d"] {
            assert!(parse_since(bad, now()).is_err(), "{bad}");
        }
    }
}
