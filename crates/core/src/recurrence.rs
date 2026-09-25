// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use chrono::{DateTime, Datelike, Days, Duration, Months, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::limits::REPEAT_N_MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Self::Mon,
        Self::Tue,
        Self::Wed,
        Self::Thu,
        Self::Fri,
        Self::Sat,
        Self::Sun,
    ];
    fn to_chrono(self) -> chrono::Weekday {
        match self {
            Self::Mon => chrono::Weekday::Mon,
            Self::Tue => chrono::Weekday::Tue,
            Self::Wed => chrono::Weekday::Wed,
            Self::Thu => chrono::Weekday::Thu,
            Self::Fri => chrono::Weekday::Fri,
            Self::Sat => chrono::Weekday::Sat,
            Self::Sun => chrono::Weekday::Sun,
        }
    }
}

/// How a job repeats after its first run. All times are UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    EverySeconds(u32),
    EveryMinutes(u32),
    EveryDays(u32),
    EveryWeeks(u32),
    EveryMonths(u32),
    /// Every given weekday, at the time-of-day of the anchor.
    Weekday(Weekday),
    /// Every nth day of the month (1-31); short months clamp to the last day.
    DayOfMonth(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepeatError {
    #[error("repeat interval must be between 1 and {REPEAT_N_MAX}")]
    IntervalOutOfRange,
    #[error("day of month must be between 1 and 31")]
    DayOutOfRange,
    #[error("unknown repeat encoding")]
    Unknown,
}

impl Repeat {
    pub fn validate(&self) -> Result<(), RepeatError> {
        match *self {
            Self::EverySeconds(n)
            | Self::EveryMinutes(n)
            | Self::EveryDays(n)
            | Self::EveryWeeks(n)
            | Self::EveryMonths(n)
                if !(1..=REPEAT_N_MAX).contains(&n) =>
            {
                Err(RepeatError::IntervalOutOfRange)
            }
            Self::DayOfMonth(d) if !(1..=31).contains(&d) => Err(RepeatError::DayOutOfRange),
            _ => Ok(()),
        }
    }

    /// Storage encoding: `(kind, value)`. Weekday uses 0=Mon..6=Sun.
    pub fn encode(&self) -> (&'static str, u32) {
        match *self {
            Self::EverySeconds(n) => ("seconds", n),
            Self::EveryMinutes(n) => ("minutes", n),
            Self::EveryDays(n) => ("days", n),
            Self::EveryWeeks(n) => ("weeks", n),
            Self::EveryMonths(n) => ("months", n),
            Self::Weekday(w) => (
                "weekday",
                Weekday::ALL.iter().position(|x| *x == w).unwrap() as u32,
            ),
            Self::DayOfMonth(d) => ("day_of_month", d as u32),
        }
    }

    pub fn decode(kind: &str, value: u32) -> Result<Self, RepeatError> {
        let r = match kind {
            "seconds" => Self::EverySeconds(value),
            "minutes" => Self::EveryMinutes(value),
            "days" => Self::EveryDays(value),
            "weeks" => Self::EveryWeeks(value),
            "months" => Self::EveryMonths(value),
            "weekday" => Self::Weekday(
                *Weekday::ALL
                    .get(value as usize)
                    .ok_or(RepeatError::Unknown)?,
            ),
            "day_of_month" => {
                Self::DayOfMonth(u8::try_from(value).map_err(|_| RepeatError::DayOutOfRange)?)
            }
            _ => return Err(RepeatError::Unknown),
        };
        r.validate()?;
        Ok(r)
    }

    /// First occurrence strictly after `after`, given the job's `anchor` (first run).
    /// Returns `None` only on arithmetic overflow.
    pub fn next_after(&self, anchor: DateTime<Utc>, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match *self {
            Self::EverySeconds(n) => step_fixed(anchor, after, Duration::seconds(n as i64)),
            Self::EveryMinutes(n) => step_fixed(anchor, after, Duration::minutes(n as i64)),
            Self::EveryDays(n) => step_fixed(anchor, after, Duration::days(n as i64)),
            Self::EveryWeeks(n) => step_fixed(anchor, after, Duration::weeks(n as i64)),
            Self::EveryMonths(n) => {
                // Always derive from the anchor so Jan 31 -> Feb 28 -> Mar 31 (no drift).
                let mut k = 1u32;
                loop {
                    let c = anchor.checked_add_months(Months::new(k.checked_mul(n)?))?;
                    if c > after {
                        return Some(c);
                    }
                    k += 1;
                }
            }
            Self::Weekday(w) => {
                let target = w.to_chrono();
                let mut d = after.date_naive();
                for _ in 0..8 {
                    if d.weekday() == target {
                        let c = at_anchor_time(d, anchor);
                        if c > after {
                            return Some(c);
                        }
                    }
                    d = d.checked_add_days(Days::new(1))?;
                }
                None
            }
            Self::DayOfMonth(day) => {
                let (mut y, mut m) = (after.year(), after.month());
                for _ in 0..3 {
                    let last = last_day_of_month(y, m);
                    let d = NaiveDate::from_ymd_opt(y, m, (day as u32).min(last))?;
                    let c = at_anchor_time(d, anchor);
                    if c > after {
                        return Some(c);
                    }
                    if m == 12 {
                        y += 1;
                        m = 1
                    } else {
                        m += 1
                    }
                }
                None
            }
        }
    }
}

fn step_fixed(
    anchor: DateTime<Utc>,
    after: DateTime<Utc>,
    step: Duration,
) -> Option<DateTime<Utc>> {
    if after < anchor {
        return Some(anchor);
    }
    let step_s = step.num_seconds().max(1);
    let elapsed = (after - anchor).num_seconds();
    anchor.checked_add_signed(Duration::seconds(
        (elapsed / step_s + 1).checked_mul(step_s)?,
    ))
}

fn at_anchor_time(d: NaiveDate, anchor: DateTime<Utc>) -> DateTime<Utc> {
    Utc.from_utc_datetime(&d.and_time(anchor.time()))
}

fn last_day_of_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn every_n_minutes_skips_missed_runs() {
        let a = t(2026, 1, 1, 0, 0);
        assert_eq!(
            Repeat::EveryMinutes(15).next_after(a, t(2026, 1, 1, 0, 40)),
            Some(t(2026, 1, 1, 0, 45))
        );
        assert_eq!(
            Repeat::EveryMinutes(15).next_after(a, t(2026, 1, 1, 0, 45)),
            Some(t(2026, 1, 1, 1, 0))
        );
    }
    #[test]
    fn before_anchor_returns_anchor() {
        let a = t(2026, 1, 1, 0, 0);
        assert_eq!(
            Repeat::EveryDays(1).next_after(a, t(2025, 12, 1, 0, 0)),
            Some(a)
        );
    }
    #[test]
    fn months_do_not_drift() {
        let a = t(2026, 1, 31, 9, 0);
        let r = Repeat::EveryMonths(1);
        let feb = r.next_after(a, a).unwrap();
        assert_eq!(feb, t(2026, 2, 28, 9, 0));
        assert_eq!(r.next_after(a, feb).unwrap(), t(2026, 3, 31, 9, 0));
    }
    #[test]
    fn weekday_uses_anchor_time() {
        let a = t(2026, 1, 1, 8, 30); // Thursday
                                      // after Fri 2026-01-02 -> next Monday 2026-01-05
        assert_eq!(
            Repeat::Weekday(Weekday::Mon).next_after(a, t(2026, 1, 2, 0, 0)),
            Some(t(2026, 1, 5, 8, 30))
        );
        // same weekday, time already passed -> a week later
        assert_eq!(
            Repeat::Weekday(Weekday::Thu).next_after(a, a),
            Some(t(2026, 1, 8, 8, 30))
        );
    }
    #[test]
    fn day_of_month_clamps() {
        let a = t(2026, 1, 1, 6, 0);
        assert_eq!(
            Repeat::DayOfMonth(31).next_after(a, t(2026, 2, 1, 0, 0)),
            Some(t(2026, 2, 28, 6, 0))
        );
        assert_eq!(
            Repeat::DayOfMonth(15).next_after(a, t(2026, 12, 20, 0, 0)),
            Some(t(2027, 1, 15, 6, 0))
        );
    }
    #[test]
    fn encode_decode_roundtrip() {
        for r in [
            Repeat::EverySeconds(5),
            Repeat::EveryWeeks(2),
            Repeat::Weekday(Weekday::Sun),
            Repeat::DayOfMonth(31),
        ] {
            let (k, v) = r.encode();
            assert_eq!(Repeat::decode(k, v), Ok(r));
        }
        assert!(Repeat::decode("days", 0).is_err());
        assert!(Repeat::decode("day_of_month", 32).is_err());
        assert!(Repeat::decode("nope", 1).is_err());
    }
}
