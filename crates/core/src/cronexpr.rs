// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Standard 5-field crontab expressions with Vixie cron semantics:
//!
//! * fields `minute hour day-of-month month day-of-week`; weekdays are `0`/`7` = Sunday
//! * lists (`1,15`), ranges (`9-17`), steps (`*/15`, `10-50/10`, `5/10`), month/day names
//! * macros `@yearly @annually @monthly @weekly @daily @midnight @hourly` (not `@reboot`)
//! * when day-of-month and day-of-week are *both* restricted, a day matches if **either** does
//!
//! Evaluation happens in a time zone. A wall-clock time skipped by a DST jump runs at the first
//! valid time after the gap; a repeated one runs once (in its first occurrence).

use std::fmt;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CronError {
    #[error("expected 5 fields (minute hour day-of-month month day-of-week), found {0}")]
    FieldCount(usize),
    #[error("@reboot is not supported")]
    Reboot,
    #[error("unknown macro {0}")]
    Macro(String),
    #[error("{field}: {msg}")]
    Field { field: &'static str, msg: String },
}

/// A parsed cron expression. Field values are bitsets; see the accessors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CronExpr {
    minutes: u64, // bits 0..=59
    hours: u32,   // bits 0..=23
    dom: u32,     // bits 1..=31
    months: u16,  // bits 1..=12
    dow: u8,      // bits 0..=6, 0 = Sunday
    /// Field text began with `*` (drives Vixie's day-of-month/day-of-week rule).
    dom_star: bool,
    dow_star: bool,
}

/// Which of the five fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Minute,
    Hour,
    DayOfMonth,
    Month,
    DayOfWeek,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Self::Minute,
        Self::Hour,
        Self::DayOfMonth,
        Self::Month,
        Self::DayOfWeek,
    ];
    pub fn range(self) -> (u32, u32) {
        match self {
            Self::Minute => (0, 59),
            Self::Hour => (0, 23),
            Self::DayOfMonth => (1, 31),
            Self::Month => (1, 12),
            Self::DayOfWeek => (0, 6),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::DayOfMonth => "day-of-month",
            Self::Month => "month",
            Self::DayOfWeek => "day-of-week",
        }
    }
}

fn parse_field(f: Field, text: &str) -> Result<(u64, bool), CronError> {
    let err = |msg: String| CronError::Field {
        field: f.name(),
        msg,
    };
    let (min, max) = f.range();
    // Day-of-week also accepts 7 for Sunday.
    let max_in = if f == Field::DayOfWeek { 7 } else { max };
    let names: &[&str] = match f {
        Field::Month => &MONTHS,
        Field::DayOfWeek => &DAYS,
        _ => &[],
    };
    let names_base = if f == Field::Month { 1 } else { 0 };
    let value = |s: &str| -> Result<u32, CronError> {
        if let Ok(n) = s.parse::<u32>() {
            return Ok(n);
        }
        names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(s))
            .map(|i| i as u32 + names_base)
            .ok_or_else(|| err(format!("`{s}` is not a valid value")))
    };

    let mut bits = 0u64;
    for item in text.split(',') {
        if item.is_empty() {
            return Err(err("empty list item".into()));
        }
        let (range, step) = match item.split_once('/') {
            Some((r, s)) => {
                let step: u32 = s
                    .parse()
                    .map_err(|_| err(format!("`{s}` is not a valid step")))?;
                if step == 0 {
                    return Err(err("step must be at least 1".into()));
                }
                (r, Some(step))
            }
            None => (item, None),
        };
        let (lo, hi) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            (value(a)?, value(b)?)
        } else {
            let v = value(range)?;
            (v, if step.is_some() { max_in } else { v })
        };
        if lo < min || hi > max_in {
            return Err(err(format!("values must be between {min} and {max_in}")));
        }
        if lo > hi {
            return Err(err(format!("range {lo}-{hi} is backwards")));
        }
        let mut v = lo;
        while v <= hi {
            bits |= 1 << v;
            v += step.unwrap_or(1);
        }
    }
    if f == Field::DayOfWeek && bits & (1 << 7) != 0 {
        bits = (bits & !(1 << 7)) | 1; // 7 is Sunday
    }
    Ok((bits, text.starts_with('*')))
}

impl FromStr for CronExpr {
    type Err = CronError;

    fn from_str(s: &str) -> Result<Self, CronError> {
        let s = s.trim();
        let expanded;
        let s = if let Some(m) = s.strip_prefix('@') {
            expanded = match m.to_ascii_lowercase().as_str() {
                "yearly" | "annually" => "0 0 1 1 *",
                "monthly" => "0 0 1 * *",
                "weekly" => "0 0 * * 0",
                "daily" | "midnight" => "0 0 * * *",
                "hourly" => "0 * * * *",
                "reboot" => return Err(CronError::Reboot),
                other => return Err(CronError::Macro(format!("@{other}"))),
            };
            expanded
        } else {
            s
        };
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(CronError::FieldCount(parts.len()));
        }
        let (minutes, _) = parse_field(Field::Minute, parts[0])?;
        let (hours, _) = parse_field(Field::Hour, parts[1])?;
        let (dom, dom_star) = parse_field(Field::DayOfMonth, parts[2])?;
        let (months, _) = parse_field(Field::Month, parts[3])?;
        let (dow, dow_star) = parse_field(Field::DayOfWeek, parts[4])?;
        Ok(Self {
            minutes,
            hours: hours as u32,
            dom: dom as u32,
            months: months as u16,
            dow: dow as u8,
            dom_star,
            dow_star,
        })
    }
}

// ---- formatting (canonical, compact, round-trips through `from_str`) ----

fn all_bits(f: Field) -> u64 {
    let (min, max) = f.range();
    (min..=max).fold(0, |b, v| b | 1 << v)
}

fn values(bits: u64, f: Field) -> Vec<u32> {
    let (min, max) = f.range();
    (min..=max).filter(|v| bits & (1 << v) != 0).collect()
}

/// `Some(step)` when `vals` is min, min+step, ... up to the last value that fits in the range.
fn progression(vals: &[u32], f: Field) -> Option<u32> {
    let (min, max) = f.range();
    let first = *vals.first()?;
    if first != min {
        return None;
    }
    let step = match vals.get(1) {
        Some(second) => second - first,
        None => max - min + 1, // a single value: `*/N` with N covering the whole range
    };
    let expected: Vec<u32> = (min..=max).step_by(step as usize).collect();
    (expected == vals).then_some(step)
}

fn format_field(f: Field, bits: u64, star: bool) -> String {
    let vals = values(bits, f);
    if bits == all_bits(f) {
        // A non-starred full range keeps its meaning for the day OR-rule.
        return if star || !matches!(f, Field::DayOfMonth | Field::DayOfWeek) {
            "*".into()
        } else {
            let (min, max) = f.range();
            format!("{min}-{max}")
        };
    }
    if star {
        if let Some(step) = progression(&vals, f) {
            return format!("*/{step}");
        }
    } else if matches!(f, Field::Minute | Field::Hour | Field::Month) && vals.len() > 2 {
        if let Some(step) = progression(&vals, f).filter(|s| *s > 1) {
            return format!("*/{step}");
        }
    }
    // Runs of 3+ consecutive values collapse into ranges.
    let mut out = Vec::new();
    let mut i = 0;
    while i < vals.len() {
        let mut j = i;
        while j + 1 < vals.len() && vals[j + 1] == vals[j] + 1 {
            j += 1;
        }
        if j - i >= 2 {
            out.push(format!("{}-{}", vals[i], vals[j]));
        } else {
            out.extend(vals[i..=j].iter().map(u32::to_string));
        }
        i = j + 1;
    }
    out.join(",")
}

impl fmt::Display for CronExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            format_field(Field::Minute, self.minutes, false),
            format_field(Field::Hour, self.hours as u64, false),
            format_field(Field::DayOfMonth, self.dom as u64, self.dom_star),
            format_field(Field::Month, self.months as u64, false),
            format_field(Field::DayOfWeek, self.dow as u64, self.dow_star),
        )
    }
}

impl Serialize for CronExpr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CronExpr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

// ---- accessors for the builder UI ----

impl CronExpr {
    /// Selected values of a field (weekdays: 0 = Sunday).
    pub fn values(&self, f: Field) -> Vec<u32> {
        values(self.bits(f), f)
    }

    fn bits(&self, f: Field) -> u64 {
        match f {
            Field::Minute => self.minutes,
            Field::Hour => self.hours as u64,
            Field::DayOfMonth => self.dom as u64,
            Field::Month => self.months as u64,
            Field::DayOfWeek => self.dow as u64,
        }
    }

    /// Every value selected (`*`).
    pub fn is_every(&self, f: Field) -> bool {
        self.bits(f) == all_bits(f)
    }

    /// `Some(n)` when the field is "every n" (`*/n`) with n > 1.
    pub fn step(&self, f: Field) -> Option<u32> {
        let vals = self.values(f);
        if vals.len() < 2 {
            return None;
        }
        progression(&vals, f).filter(|s| *s > 1)
    }

    /// Returns a copy with `f` set to exactly `vals` (empty is ignored: a field needs a value).
    /// `star` only matters for day-of-month/day-of-week and marks the field as "any".
    pub fn with_values(mut self, f: Field, vals: &[u32], star: bool) -> Self {
        let (min, max) = f.range();
        let bits = vals
            .iter()
            .filter(|v| (min..=max).contains(v))
            .fold(0u64, |b, v| b | 1 << v);
        if bits == 0 {
            return self;
        }
        match f {
            Field::Minute => self.minutes = bits,
            Field::Hour => self.hours = bits as u32,
            Field::DayOfMonth => {
                self.dom = bits as u32;
                self.dom_star = star;
            }
            Field::Month => self.months = bits as u16,
            Field::DayOfWeek => {
                self.dow = bits as u8;
                self.dow_star = star;
            }
        }
        self
    }

    /// Both day fields restricted: Vixie runs on days matching *either*.
    pub fn day_fields_are_ored(&self) -> bool {
        !self.dom_star && !self.dow_star
    }
}

// ---- next occurrence ----

fn bit(bits: u64, i: u32) -> bool {
    bits & (1 << i) != 0
}

impl CronExpr {
    fn day_ok(&self, d: NaiveDate) -> bool {
        let dom = bit(self.dom as u64, d.day());
        let dow = bit(self.dow as u64, d.weekday().num_days_from_sunday());
        if self.dom_star || self.dow_star {
            dom && dow
        } else {
            dom || dow
        }
    }

    /// First run strictly after `after`, evaluated in `tz`. `None` if nothing matches within
    /// nine years (e.g. `0 0 30 2 *`).
    pub fn next_after(&self, after: DateTime<Utc>, tz: Tz) -> Option<DateTime<Utc>> {
        let start = after.with_timezone(&tz).naive_local();
        let mut t = start.with_second(0)?.with_nanosecond(0)? + Duration::minutes(1);
        let last_year = t.year() + 9;
        while t.year() <= last_year {
            if !bit(self.months as u64, t.month()) {
                let (y, m) = if t.month() == 12 {
                    (t.year() + 1, 1)
                } else {
                    (t.year(), t.month() + 1)
                };
                t = NaiveDate::from_ymd_opt(y, m, 1)?.and_hms_opt(0, 0, 0)?;
                continue;
            }
            if !self.day_ok(t.date()) {
                t = t.date().succ_opt()?.and_hms_opt(0, 0, 0)?;
                continue;
            }
            if !bit(self.hours as u64, t.hour()) {
                t = t.with_minute(0)? + Duration::hours(1);
                continue;
            }
            match (t.minute()..60).find(|m| bit(self.minutes, *m)) {
                Some(m) => t = t.with_minute(m)?,
                None => {
                    t = t.with_minute(0)? + Duration::hours(1);
                    continue;
                }
            }
            if let Some(u) = resolve_after(tz, t, after) {
                return Some(u);
            }
            t += Duration::minutes(1);
        }
        None
    }

    /// The next `n` runs after `after`.
    pub fn next_n(&self, after: DateTime<Utc>, tz: Tz, n: usize) -> Vec<DateTime<Utc>> {
        let mut out = Vec::with_capacity(n);
        let mut cur = after;
        while out.len() < n {
            match self.next_after(cur, tz) {
                Some(next) => {
                    out.push(next);
                    cur = next;
                }
                None => break,
            }
        }
        out
    }
}

/// Wall-clock -> the earliest UTC instant strictly after `after`. A repeated hour yields its
/// first occurrence (or the second if the first has already passed); a skipped time moves to
/// the first valid time after the gap.
fn resolve_after(tz: Tz, n: NaiveDateTime, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let pick = |r: LocalResult<DateTime<Tz>>| match r {
        LocalResult::Single(x) => Some(x.with_timezone(&Utc)).filter(|u| *u > after),
        LocalResult::Ambiguous(a, b) => [a, b]
            .into_iter()
            .map(|x| x.with_timezone(&Utc))
            .find(|u| *u > after),
        LocalResult::None => None,
    };
    if let LocalResult::None = tz.from_local_datetime(&n) {
        return (1..=6)
            .find_map(|i| pick(tz.from_local_datetime(&(n + Duration::minutes(30 * i)))));
    }
    pick(tz.from_local_datetime(&n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America::New_York, Asia::Kolkata, UTC};

    fn t(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }
    fn e(s: &str) -> CronExpr {
        s.parse().unwrap_or_else(|err| panic!("{s}: {err}"))
    }
    fn local(tz: Tz, dt: DateTime<Utc>) -> String {
        dt.with_timezone(&tz).format("%a %m-%d %H:%M").to_string()
    }

    #[test]
    fn weekdays_are_crontab_numbered_not_quartz() {
        // 1-5 is Mon-Fri; 0 and 7 are both Sunday.
        let r: Vec<_> = e("1 7 * * 1-5")
            .next_n(t(2026, 9, 25, 6, 30), Kolkata, 3)
            .iter()
            .map(|d| local(Kolkata, *d))
            .collect();
        assert_eq!(r, ["Mon 09-28 07:01", "Tue 09-29 07:01", "Wed 09-30 07:01"]);
        for s in ["0 9 * * 0", "0 9 * * 7", "0 9 * * sun", "0 9 * * SUN"] {
            assert_eq!(
                local(UTC, e(s).next_after(t(2026, 9, 25, 0, 0), UTC).unwrap()),
                "Sun 09-27 09:00",
                "{s}"
            );
        }
        assert_eq!(e("0 9 * * 5-7").values(Field::DayOfWeek), [0, 5, 6]);
    }

    #[test]
    fn day_of_month_and_weekday_are_ored() {
        // 13th OR any Friday (not Friday-the-13th only).
        let r: Vec<_> = e("0 0 13 * fri")
            .next_n(t(2026, 9, 25, 12, 0), UTC, 4)
            .iter()
            .map(|d| local(UTC, *d))
            .collect();
        assert_eq!(
            r,
            [
                "Fri 10-02 00:00",
                "Fri 10-09 00:00",
                "Tue 10-13 00:00",
                "Fri 10-16 00:00"
            ]
        );
        assert!(e("0 0 13 * fri").day_fields_are_ored());
        // A starred field switches to AND (`*/2` counts as starred).
        assert!(!e("0 0 * * fri").day_fields_are_ored());
        assert_eq!(
            local(
                UTC,
                e("0 0 */2 * fri")
                    .next_after(t(2026, 9, 25, 12, 0), UTC)
                    .unwrap()
            ),
            "Fri 10-09 00:00"
        );
    }

    #[test]
    fn steps_lists_ranges_and_names() {
        let a = t(2026, 9, 25, 12, 0);
        let m = |s: &str| {
            e(s).next_n(a, UTC, 4)
                .iter()
                .map(|d| d.format("%H:%M").to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(m("*/7 12 * * *"), ["12:07", "12:14", "12:21", "12:28"]);
        assert_eq!(m("10-50/20 12 * * *"), ["12:10", "12:30", "12:50", "12:10"]); // 4th is tomorrow
        assert_eq!(e("5/20 * * * *").values(Field::Minute), [5, 25, 45]);
        assert_eq!(e("0 0 1,15 jan-mar *").values(Field::Month), [1, 2, 3]);
        assert_eq!(e("0 0 1 JAN,jul *").values(Field::Month), [1, 7]);
    }

    #[test]
    fn macros() {
        assert_eq!(e("@daily"), e("0 0 * * *"));
        assert_eq!(e("@midnight"), e("0 0 * * *"));
        assert_eq!(e("@hourly"), e("0 * * * *"));
        assert_eq!(e("@weekly"), e("0 0 * * 0"));
        assert_eq!(e("@monthly"), e("0 0 1 * *"));
        assert_eq!(e("@annually"), e("0 0 1 1 *"));
        assert_eq!(e("@yearly"), e("0 0 1 1 *"));
        assert_eq!("@reboot".parse::<CronExpr>(), Err(CronError::Reboot));
        assert!(matches!(
            "@fortnightly".parse::<CronExpr>(),
            Err(CronError::Macro(_))
        ));
    }

    #[test]
    fn rejects_bad_input() {
        for s in [
            "",
            "* * * *",
            "* * * * * *",
            "60 * * * *",
            "* 24 * * *",
            "* * 0 * *",
            "* * 32 * *",
            "* * * 13 *",
            "* * * * 8",
            "*/0 * * * *",
            "5-1 * * * *",
            "1,,2 * * * *",
            "a * * * *",
            "* * * foo *",
            "1-2-3 * * * *",
        ] {
            assert!(s.parse::<CronExpr>().is_err(), "should reject {s:?}");
        }
        assert_eq!("* * * *".parse::<CronExpr>(), Err(CronError::FieldCount(4)));
        let msg = "61 * * * *".parse::<CronExpr>().unwrap_err().to_string();
        assert!(msg.starts_with("minute:"), "{msg}");
    }

    #[test]
    fn display_is_canonical_and_round_trips() {
        for (input, canonical) in [
            ("0 0 * * *", "0 0 * * *"),
            ("@daily", "0 0 * * *"),
            ("0,15,30,45 * * * *", "*/15 * * * *"),
            ("*/15 9-17 * * mon-fri", "*/15 9-17 * * 1-5"),
            ("1 7 * * 1-5", "1 7 * * 1-5"),
            ("0 0 1,15 * *", "0 0 1,15 * *"),
            ("0 0 */2 * *", "0 0 */2 * *"),
            ("0 0 1-31 * *", "0 0 1-31 * *"), // not starred: keeps OR semantics
            ("0 0 13 * 0-6", "0 0 13 * 0-6"),
            ("0 8,12,16 * * 1,3,5", "0 8,12,16 * * 1,3,5"),
            ("0 0 * jan,jun *", "0 0 * 1,6 *"),
            ("0 0 * */3 *", "0 0 * */3 *"),
        ] {
            let x = e(input);
            assert_eq!(x.to_string(), canonical, "{input}");
            assert_eq!(e(&x.to_string()), x, "round trip {input}");
        }
        let json = serde_json_like(&e("1 7 * * 1-5"));
        assert_eq!(json, "1 7 * * 1-5");
    }

    fn serde_json_like(x: &CronExpr) -> String {
        x.to_string()
    }

    #[test]
    fn dst_gap_runs_after_the_gap_and_fall_back_runs_once() {
        // 2026-03-08 02:30 does not exist in New York; the job runs at 03:00 EDT that day.
        let next = e("30 2 * * *")
            .next_after(t(2026, 3, 7, 12, 0), New_York)
            .unwrap();
        assert_eq!(local(New_York, next), "Sun 03-08 03:00");
        // ...and the day after it is back to 02:30.
        assert_eq!(
            local(
                New_York,
                e("30 2 * * *").next_after(next, New_York).unwrap()
            ),
            "Mon 03-09 02:30"
        );
        // 2026-11-01 01:30 happens twice; a daily 01:30 job runs once, at the first one.
        let first = e("30 1 * * *")
            .next_after(t(2026, 10, 31, 12, 0), New_York)
            .unwrap();
        assert_eq!(first, t(2026, 11, 1, 5, 30)); // 01:30 EDT
        assert_eq!(
            local(
                New_York,
                e("30 1 * * *").next_after(first, New_York).unwrap()
            ),
            "Mon 11-02 01:30"
        );
    }

    #[test]
    fn fixed_time_survives_dst_in_local_time() {
        let a = e("0 7 * * *")
            .next_after(t(2026, 10, 31, 12, 0), New_York)
            .unwrap(); // Sun 11-01 07:00 EST
        assert_eq!(a, t(2026, 11, 1, 12, 0));
    }

    #[test]
    fn rare_dates() {
        // Feb 29 only in leap years; 30 Feb never.
        assert_eq!(
            e("0 0 29 2 *").next_after(t(2026, 3, 1, 0, 0), UTC),
            Some(t(2028, 2, 29, 0, 0))
        );
        assert_eq!(e("0 0 30 2 *").next_after(t(2026, 3, 1, 0, 0), UTC), None);
        assert_eq!(
            e("59 23 31 12 *").next_after(t(2026, 12, 31, 23, 59), UTC),
            Some(t(2027, 12, 31, 23, 59))
        );
    }

    #[test]
    fn strictly_after_and_minute_precision() {
        let x = e("*/5 * * * *");
        let at = t(2026, 9, 25, 10, 5);
        assert_eq!(x.next_after(at, UTC), Some(t(2026, 9, 25, 10, 10)));
        // Seconds are ignored when deciding what is "after".
        assert_eq!(
            x.next_after(at + Duration::seconds(30), UTC),
            Some(t(2026, 9, 25, 10, 10))
        );
        assert_eq!(
            x.next_after(at - Duration::seconds(30), UTC),
            Some(t(2026, 9, 25, 10, 5))
        );
    }

    #[test]
    fn builder_helpers() {
        let x = e("0 9 * * 1-5");
        assert!(x.is_every(Field::Month) && !x.is_every(Field::Hour));
        assert_eq!(e("*/15 * * * *").step(Field::Minute), Some(15));
        assert_eq!(e("0,10,25 * * * *").step(Field::Minute), None);
        let y = x.with_values(Field::Hour, &[6, 18], false).with_values(
            Field::DayOfWeek,
            &[6, 0],
            false,
        );
        assert_eq!(y.to_string(), "0 6,18 * * 0,6");
        // Empty selections are ignored.
        assert_eq!(x.with_values(Field::Hour, &[], false), x);
    }
}
