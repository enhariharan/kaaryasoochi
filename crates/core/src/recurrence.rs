// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

use chrono::{
    DateTime, Datelike, Days, Duration, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone,
    Timelike, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::cronexpr::CronExpr;
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
    fn index(self) -> u8 {
        Self::ALL.iter().position(|x| *x == self).unwrap() as u8
    }
}

/// A set of weekdays as a bitmask: bit 0 = Mon .. bit 6 = Sun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekdaySet(pub u8);

impl WeekdaySet {
    pub const MON_FRI: Self = Self(0b001_1111);
    pub fn contains(&self, d: chrono::Weekday) -> bool {
        self.0 & (1 << d.num_days_from_monday()) != 0
    }
    pub fn is_valid(&self) -> bool {
        (1..=0b111_1111).contains(&self.0)
    }
}

impl From<Weekday> for WeekdaySet {
    fn from(w: Weekday) -> Self {
        Self(1 << w.index())
    }
}

/// How a job repeats after its first run.
///
/// Calendar-based rules (days, weeks, months, weekdays, day-of-month) are evaluated in the
/// job's time zone, so "07:01 every weekday" stays 07:01 local across DST changes. Second and
/// minute intervals are fixed durations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    EverySeconds(u32),
    EveryMinutes(u32),
    EveryDays(u32),
    EveryWeeks(u32),
    EveryMonths(u32),
    /// Every given weekday, at the local time-of-day of the anchor.
    Weekday(Weekday),
    /// Every one of several weekdays (e.g. Mon-Fri), at the anchor's local time-of-day.
    Weekdays(WeekdaySet),
    /// Every nth day of the month (1-31); short months clamp to the last day.
    DayOfMonth(u8),
    /// A 5-field crontab expression. The job's first-run time is only a lower bound ("start
    /// from"); occurrences come from the expression.
    Cron(CronExpr),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepeatError {
    #[error("repeat interval must be between 1 and {REPEAT_N_MAX}")]
    IntervalOutOfRange,
    #[error("day of month must be between 1 and 31")]
    DayOutOfRange,
    #[error("select at least one weekday")]
    NoWeekdays,
    #[error("invalid cron expression")]
    BadCron,
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
            Self::Weekdays(s) if !s.is_valid() => Err(RepeatError::NoWeekdays),
            _ => Ok(()),
        }
    }

    /// Storage encoding: `(kind, value)`. `weekday` uses 0=Mon..6=Sun; `weekdays` is a bitmask.
    pub fn encode(&self) -> (&'static str, u32) {
        match *self {
            Self::EverySeconds(n) => ("seconds", n),
            Self::EveryMinutes(n) => ("minutes", n),
            Self::EveryDays(n) => ("days", n),
            Self::EveryWeeks(n) => ("weeks", n),
            Self::EveryMonths(n) => ("months", n),
            Self::Weekday(w) => ("weekday", w.index() as u32),
            Self::Weekdays(s) => ("weekdays", s.0 as u32),
            Self::DayOfMonth(d) => ("day_of_month", d as u32),
            Self::Cron(_) => ("cron", 0),
        }
    }

    /// The cron expression that behaves *exactly* like this rule, when there is one. Used to
    /// open older rules in the cron builder; anything not exactly expressible (e.g. every 3
    /// days, every 90 minutes, day 31 clamping to month-end) returns `None` and is kept as is.
    pub fn to_cron(&self, anchor: DateTime<Utc>, tz: Tz) -> Option<CronExpr> {
        let local = anchor.with_timezone(&tz);
        let (m, h) = (local.minute(), local.hour());
        let text = match *self {
            Self::Cron(x) => return Some(x),
            Self::EveryDays(1) => format!("{m} {h} * * *"),
            Self::DayOfMonth(d) if d <= 28 => format!("{m} {h} {d} * *"),
            Self::Weekday(w) => return Self::Weekdays(w.into()).to_cron(anchor, tz),
            Self::Weekdays(set) if set.is_valid() => {
                // Bit 0 is Monday here, but cron counts from Sunday = 0.
                let days: Vec<String> = (0..7u8)
                    .filter(|i| set.0 & (1 << i) != 0)
                    .map(|i| ((i + 1) % 7).to_string())
                    .collect();
                format!("{m} {h} * * {}", days.join(","))
            }
            _ => return None,
        };
        text.parse().ok()
    }

    /// The expression text for `Cron` repeats (stored next to `encode`'s kind/value).
    pub fn cron_text(&self) -> Option<String> {
        match self {
            Self::Cron(x) => Some(x.to_string()),
            _ => None,
        }
    }

    /// Like [`decode`](Self::decode) but also understands `cron` (whose text is in `expr`).
    pub fn decode_full(kind: &str, value: u32, expr: &str) -> Result<Self, RepeatError> {
        if kind == "cron" {
            return expr
                .parse()
                .map(Self::Cron)
                .map_err(|_| RepeatError::BadCron);
        }
        Self::decode(kind, value)
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
            "weekdays" => Self::Weekdays(WeekdaySet(
                u8::try_from(value).map_err(|_| RepeatError::NoWeekdays)?,
            )),
            "day_of_month" => {
                Self::DayOfMonth(u8::try_from(value).map_err(|_| RepeatError::DayOutOfRange)?)
            }
            _ => return Err(RepeatError::Unknown),
        };
        r.validate()?;
        Ok(r)
    }

    /// First occurrence strictly after `after`, given the job's `anchor` (first run) and the
    /// time zone its calendar rules are evaluated in. If `after` is before the anchor, the
    /// anchor itself is next. Returns `None` only on arithmetic overflow.
    pub fn next_after(
        &self,
        anchor: DateTime<Utc>,
        after: DateTime<Utc>,
        tz: Tz,
    ) -> Option<DateTime<Utc>> {
        if let Self::Cron(x) = self {
            // The anchor is only "start from": never run before it.
            let from = if after < anchor {
                anchor - Duration::seconds(1)
            } else {
                after
            };
            return x.next_after(from, tz);
        }
        if after < anchor {
            return Some(anchor);
        }
        let a = anchor.with_timezone(&tz).naive_local();
        let after_l = after.with_timezone(&tz).naive_local();
        let first_after = |cands: &mut dyn Iterator<Item = NaiveDate>| {
            cands
                .filter_map(|d| resolve(tz, d.and_time(a.time())))
                .find(|c| *c > after)
        };
        match *self {
            Self::EverySeconds(n) => step_fixed(anchor, after, Duration::seconds(n as i64)),
            Self::EveryMinutes(n) => step_fixed(anchor, after, Duration::minutes(n as i64)),
            Self::EveryDays(n) => every_days(a, after_l, n as u64, &first_after),
            Self::EveryWeeks(n) => every_days(a, after_l, n as u64 * 7, &first_after),
            Self::EveryMonths(n) => {
                let elapsed =
                    (after_l.year() - a.year()) * 12 + after_l.month() as i32 - a.month() as i32;
                let k0 = (elapsed.max(0) as u32 / n).saturating_sub(1);
                // Derive from the anchor each time so Jan 31 -> Feb 28 -> Mar 31 (no drift).
                first_after(
                    &mut (k0..k0 + 4)
                        .filter_map(|k| a.date().checked_add_months(Months::new(k * n))),
                )
            }
            Self::Weekday(w) => weekdays(a, after_l, WeekdaySet::from(w), &first_after),
            Self::Weekdays(s) => weekdays(a, after_l, s, &first_after),
            Self::Cron(_) => unreachable!("handled above"),
            Self::DayOfMonth(day) => {
                let start = after_l.date().with_day(1)?;
                first_after(&mut (0..3).filter_map(|k| {
                    let m = start.checked_add_months(Months::new(k))?;
                    let last = last_day_of_month(m.year(), m.month());
                    NaiveDate::from_ymd_opt(m.year(), m.month(), (day as u32).min(last))
                }))
            }
        }
    }
}

type FirstAfter<'a> = &'a dyn Fn(&mut dyn Iterator<Item = NaiveDate>) -> Option<DateTime<Utc>>;

fn every_days(
    a: NaiveDateTime,
    after_l: NaiveDateTime,
    step: u64,
    first_after: FirstAfter,
) -> Option<DateTime<Utc>> {
    let elapsed = (after_l.date() - a.date()).num_days().max(0) as u64;
    let k0 = (elapsed / step).saturating_sub(1);
    first_after(&mut (k0..k0 + 4).filter_map(|k| a.date().checked_add_days(Days::new(k * step))))
}

fn weekdays(
    _a: NaiveDateTime,
    after_l: NaiveDateTime,
    set: WeekdaySet,
    first_after: FirstAfter,
) -> Option<DateTime<Utc>> {
    first_after(
        &mut (0..9)
            .filter_map(|k| after_l.date().checked_add_days(Days::new(k)))
            .filter(|d| set.contains(d.weekday())),
    )
}

/// Local wall-clock time -> UTC. Ambiguous (DST fall-back) picks the first occurrence; a
/// non-existent time (spring-forward gap) moves to the first valid half hour after it.
fn resolve(tz: Tz, n: NaiveDateTime) -> Option<DateTime<Utc>> {
    (0..6).find_map(
        |i| match tz.from_local_datetime(&(n + Duration::minutes(30 * i))) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => Some(t.with_timezone(&Utc)),
            LocalResult::None => None,
        },
    )
}

fn step_fixed(
    anchor: DateTime<Utc>,
    after: DateTime<Utc>,
    step: Duration,
) -> Option<DateTime<Utc>> {
    let step_s = step.num_seconds().max(1);
    let elapsed = (after - anchor).num_seconds();
    anchor.checked_add_signed(Duration::seconds(
        (elapsed / step_s + 1).checked_mul(step_s)?,
    ))
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
    use chrono_tz::{America::New_York, Asia::Kolkata, UTC};

    fn t(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn every_n_minutes_skips_missed_runs() {
        let a = t(2026, 1, 1, 0, 0);
        let r = Repeat::EveryMinutes(15);
        assert_eq!(
            r.next_after(a, t(2026, 1, 1, 0, 40), UTC),
            Some(t(2026, 1, 1, 0, 45))
        );
        assert_eq!(
            r.next_after(a, t(2026, 1, 1, 0, 45), UTC),
            Some(t(2026, 1, 1, 1, 0))
        );
    }

    #[test]
    fn before_anchor_returns_anchor() {
        let a = t(2026, 1, 1, 0, 0);
        for r in [
            Repeat::EveryDays(1),
            Repeat::Weekdays(WeekdaySet::MON_FRI),
            Repeat::DayOfMonth(5),
        ] {
            assert_eq!(r.next_after(a, t(2025, 12, 1, 0, 0), UTC), Some(a));
        }
    }

    #[test]
    fn days_and_weeks_step_from_anchor() {
        let a = t(2026, 1, 1, 8, 0);
        assert_eq!(
            Repeat::EveryDays(3).next_after(a, t(2026, 1, 5, 0, 0), UTC),
            Some(t(2026, 1, 7, 8, 0))
        );
        assert_eq!(
            Repeat::EveryWeeks(2).next_after(a, a, UTC),
            Some(t(2026, 1, 15, 8, 0))
        );
    }

    #[test]
    fn months_do_not_drift() {
        let a = t(2026, 1, 31, 9, 0);
        let r = Repeat::EveryMonths(1);
        let feb = r.next_after(a, a, UTC).unwrap();
        assert_eq!(feb, t(2026, 2, 28, 9, 0));
        assert_eq!(r.next_after(a, feb, UTC).unwrap(), t(2026, 3, 31, 9, 0));
    }

    #[test]
    fn weekday_uses_anchor_time() {
        let a = t(2026, 1, 1, 8, 30); // Thursday
        assert_eq!(
            Repeat::Weekday(Weekday::Mon).next_after(a, t(2026, 1, 2, 0, 0), UTC),
            Some(t(2026, 1, 5, 8, 30))
        );
        assert_eq!(
            Repeat::Weekday(Weekday::Thu).next_after(a, a, UTC),
            Some(t(2026, 1, 8, 8, 30))
        );
    }

    #[test]
    fn mon_fri_skips_weekend_in_kolkata() {
        // Mon 2026-09-28 07:01 IST == 01:31 UTC.
        let a = t(2026, 9, 28, 1, 31);
        let r = Repeat::Weekdays(WeekdaySet::MON_FRI);
        assert_eq!(r.next_after(a, a, Kolkata), Some(t(2026, 9, 29, 1, 31)));
        // After Friday's run the next one is Monday, not Saturday.
        assert_eq!(
            r.next_after(a, t(2026, 10, 2, 1, 31), Kolkata),
            Some(t(2026, 10, 5, 1, 31))
        );
    }

    #[test]
    fn weekday_set_is_evaluated_in_local_date() {
        // 23:30 New York on Friday is already Saturday in UTC; the rule must follow local dates.
        let a = New_York
            .with_ymd_and_hms(2026, 9, 25, 23, 30, 0)
            .unwrap()
            .with_timezone(&Utc);
        let r = Repeat::Weekdays(WeekdaySet::MON_FRI);
        let next = r
            .next_after(a, a, New_York)
            .unwrap()
            .with_timezone(&New_York);
        assert_eq!(
            (next.weekday(), next.format("%H:%M").to_string()),
            (chrono::Weekday::Mon, "23:30".into())
        );
    }

    #[test]
    fn daily_stays_at_local_time_across_dst() {
        // US fall-back on 2026-11-01: 07:00 local is 11:00Z before and 12:00Z after.
        let a = New_York
            .with_ymd_and_hms(2026, 10, 31, 7, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(a, t(2026, 10, 31, 11, 0));
        assert_eq!(
            Repeat::EveryDays(1).next_after(a, a, New_York),
            Some(t(2026, 11, 1, 12, 0))
        );
    }

    #[test]
    fn nonexistent_local_time_moves_forward() {
        // 02:30 on 2026-03-08 does not exist in New York; the gap is 02:00-03:00, so expect the first valid time, 03:00 EDT (07:00Z).
        let a = New_York
            .with_ymd_and_hms(2026, 3, 7, 2, 30, 0)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            Repeat::EveryDays(1).next_after(a, a, New_York),
            Some(t(2026, 3, 8, 7, 0))
        );
    }

    #[test]
    fn day_of_month_clamps() {
        let a = t(2026, 1, 1, 6, 0);
        assert_eq!(
            Repeat::DayOfMonth(31).next_after(a, t(2026, 2, 1, 0, 0), UTC),
            Some(t(2026, 2, 28, 6, 0))
        );
        assert_eq!(
            Repeat::DayOfMonth(15).next_after(a, t(2026, 12, 20, 0, 0), UTC),
            Some(t(2027, 1, 15, 6, 0))
        );
    }

    #[test]
    fn cron_repeat_starts_from_the_anchor_and_round_trips() {
        let r = Repeat::Cron("1 7 * * 1-5".parse().unwrap());
        // "Start from" Mon 2026-09-28 07:01 IST (01:31Z): that minute itself is the first run.
        let start = t(2026, 9, 28, 1, 31);
        assert_eq!(
            r.next_after(start, t(2026, 9, 1, 0, 0), Kolkata),
            Some(start)
        );
        assert_eq!(
            r.next_after(start, start, Kolkata),
            Some(t(2026, 9, 29, 1, 31))
        );
        assert_eq!(
            r.next_after(start, t(2026, 10, 2, 1, 31), Kolkata),
            Some(t(2026, 10, 5, 1, 31))
        );
        let (kind, value) = r.encode();
        assert_eq!(
            Repeat::decode_full(kind, value, &r.cron_text().unwrap()),
            Ok(r)
        );
        assert_eq!(
            Repeat::decode_full("cron", 0, "not a cron"),
            Err(RepeatError::BadCron)
        );
    }

    #[test]
    fn older_rules_convert_to_cron_only_when_exact() {
        // Mon 2026-09-28 07:01 IST.
        let a = t(2026, 9, 28, 1, 31);
        let c = |r: Repeat| r.to_cron(a, Kolkata).map(|x| x.to_string());
        assert_eq!(
            c(Repeat::Weekdays(WeekdaySet::MON_FRI)).as_deref(),
            Some("1 7 * * 1-5")
        );
        assert_eq!(
            c(Repeat::Weekday(Weekday::Sun)).as_deref(),
            Some("1 7 * * 0")
        );
        assert_eq!(
            c(Repeat::Weekdays(WeekdaySet(0b110_0000))).as_deref(),
            Some("1 7 * * 0,6")
        ); // Sat+Sun
        assert_eq!(c(Repeat::EveryDays(1)).as_deref(), Some("1 7 * * *"));
        assert_eq!(c(Repeat::DayOfMonth(15)).as_deref(), Some("1 7 15 * *"));
        // Not exactly expressible: kept as the older rule.
        for r in [
            Repeat::EveryDays(3),
            Repeat::EveryWeeks(2),
            Repeat::EveryMonths(1),
            Repeat::EveryMinutes(90),
            Repeat::EverySeconds(30),
            Repeat::DayOfMonth(31),
        ] {
            assert_eq!(c(r), None, "{r:?}");
        }
        // And the conversion really matches the old behaviour.
        let old = Repeat::Weekdays(WeekdaySet::MON_FRI);
        let new = old.to_cron(a, Kolkata).map(Repeat::Cron).unwrap();
        let mut cur = a;
        for _ in 0..12 {
            let (x, y) = (
                old.next_after(a, cur, Kolkata),
                new.next_after(a, cur, Kolkata),
            );
            assert_eq!(x, y);
            cur = x.unwrap();
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        for r in [
            Repeat::EverySeconds(5),
            Repeat::EveryWeeks(2),
            Repeat::Weekday(Weekday::Sun),
            Repeat::Weekdays(WeekdaySet::MON_FRI),
            Repeat::DayOfMonth(31),
        ] {
            let (k, v) = r.encode();
            assert_eq!(Repeat::decode(k, v), Ok(r));
        }
        assert!(Repeat::decode("days", 0).is_err());
        assert!(Repeat::decode("day_of_month", 32).is_err());
        assert!(Repeat::decode("weekdays", 0).is_err());
        assert!(Repeat::decode("weekdays", 128).is_err());
        assert!(Repeat::decode("nope", 1).is_err());
    }
}
