// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Hariharan Narayanan

//! Human-facing helpers for cron expressions: an English description and the presets behind the
//! schedule builder's "Simple" tab.

use crate::cronexpr::{CronExpr, Field};

const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn join(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [head @ .., last] => format!("{} and {last}", head.join(", ")),
    }
}

fn contiguous(v: &[u32]) -> bool {
    v.len() >= 2 && v.windows(2).all(|w| w[1] == w[0] + 1)
}

fn time_part(x: &CronExpr) -> String {
    let (mins, hrs) = (x.values(Field::Minute), x.values(Field::Hour));
    let (all_m, all_h) = (x.is_every(Field::Minute), x.is_every(Field::Hour));
    let times = |hs: &[u32], ms: &[u32]| -> Vec<String> {
        hs.iter()
            .flat_map(|h| ms.iter().map(move |m| format!("{h:02}:{m:02}")))
            .collect()
    };
    if all_m && all_h {
        return "Every minute".into();
    }
    if all_h {
        if let Some(s) = x.step(Field::Minute) {
            return format!("Every {s} minutes");
        }
        if mins.len() == 1 {
            return format!("At minute {} past every hour", mins[0]);
        }
    }
    if let (Some(s), 1) = (x.step(Field::Hour), mins.len()) {
        return format!("At minute {} past every {} hour", mins[0], ordinal(s));
    }
    if !all_m && !all_h && mins.len() * hrs.len() <= 6 {
        return format!("At {}", join(&times(&hrs, &mins)));
    }
    if contiguous(&hrs) {
        let range = format!("between {:02}:00 and {:02}:59", hrs[0], hrs[hrs.len() - 1]);
        if all_m {
            return format!("Every minute, {range}");
        }
        if let Some(s) = x.step(Field::Minute) {
            return format!("Every {s} minutes, {range}");
        }
        if mins.len() == 1 {
            return format!("At minute {}, {range}", mins[0]);
        }
    }
    let list = |v: &[u32]| join(&v.iter().map(u32::to_string).collect::<Vec<_>>());
    let m = if all_m {
        "every minute".to_string()
    } else {
        format!("minute {}", list(&mins))
    };
    let h = if all_h {
        "every hour".to_string()
    } else {
        format!("hour {}", list(&hrs))
    };
    format!("At {m} past {h}")
}

fn day_part(x: &CronExpr) -> Option<String> {
    let (dom_any, dow_any) = (x.is_every(Field::DayOfMonth), x.is_every(Field::DayOfWeek));
    let dow = x.values(Field::DayOfWeek);
    let dow_text = if dow_any {
        None
    } else if dow.len() >= 3 && contiguous(&dow) {
        Some(format!(
            "{} through {}",
            DAY_NAMES[dow[0] as usize],
            DAY_NAMES[dow[dow.len() - 1] as usize]
        ))
    } else {
        Some(join(
            &dow.iter()
                .map(|d| DAY_NAMES[*d as usize].to_string())
                .collect::<Vec<_>>(),
        ))
    };
    let dom = x.values(Field::DayOfMonth);
    let dom_text = if dom_any {
        None
    } else if let Some(s) = x.step(Field::DayOfMonth) {
        Some(format!("every {} day of the month", ordinal(s)))
    } else {
        let d: Vec<_> = dom.iter().map(|d| d.to_string()).collect();
        Some(format!("day {} of the month", join(&d)))
    };
    match (dom_text, dow_text) {
        (None, None) => None,
        (Some(d), None) => Some(format!("on {d}")),
        (None, Some(w)) => Some(format!("on {w}")),
        (Some(d), Some(w)) if x.day_fields_are_ored() => Some(format!("on {d} and on {w}")),
        (Some(d), Some(w)) => Some(format!("on {w}, only on {d}")),
    }
}

/// e.g. `1 7 * * 1-5` -> "At 07:01, on Monday through Friday".
pub fn describe(x: &CronExpr) -> String {
    let time = time_part(x);
    let day = day_part(x);
    let months = (!x.is_every(Field::Month)).then(|| {
        let names: Vec<_> = x
            .values(Field::Month)
            .iter()
            .map(|m| MONTH_NAMES[*m as usize - 1].to_string())
            .collect();
        format!("in {}", join(&names))
    });
    let mut out = time.clone();
    let restricted = day.is_some() || months.is_some();
    for part in [day, months].into_iter().flatten() {
        out.push_str(", ");
        out.push_str(&part);
    }
    // Only clock times ("At 07:01") read better with "every day"; "past every hour" does not.
    if !restricted && time.contains(':') {
        out.push_str(", every day");
    }
    out
}

/// The friendly recurrences offered by the schedule builder's "Simple" tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimplePreset {
    EveryNMinutes(u8),
    EveryNHours {
        n: u8,
        minute: u8,
    },
    Daily {
        hour: u8,
        minute: u8,
    },
    /// `days` is a bitmask, bit 0 = Sunday.
    Weekly {
        days: u8,
        hour: u8,
        minute: u8,
    },
    Monthly {
        day: u8,
        hour: u8,
        minute: u8,
    },
    Yearly {
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
    },
}

impl SimplePreset {
    pub fn to_expr(&self) -> CronExpr {
        let s = match *self {
            Self::EveryNMinutes(1) => "* * * * *".to_string(),
            Self::EveryNMinutes(n) => format!("*/{n} * * * *"),
            Self::EveryNHours { n: 1, minute } => format!("{minute} * * * *"),
            Self::EveryNHours { n, minute } => format!("{minute} */{n} * * *"),
            Self::Daily { hour, minute } => format!("{minute} {hour} * * *"),
            Self::Weekly { days, hour, minute } => {
                let list: Vec<_> = (0..7)
                    .filter(|d| days & (1 << d) != 0)
                    .map(|d| d.to_string())
                    .collect();
                // No day selected would be invalid; fall back to every day.
                let dow = if list.is_empty() {
                    "*".to_string()
                } else {
                    list.join(",")
                };
                format!("{minute} {hour} * * {dow}")
            }
            Self::Monthly { day, hour, minute } => format!("{minute} {hour} {day} * *"),
            Self::Yearly {
                month,
                day,
                hour,
                minute,
            } => format!("{minute} {hour} {day} {month} *"),
        };
        s.parse().expect("presets always produce valid expressions")
    }

    /// The preset that produces exactly `x`, if there is one.
    pub fn from_expr(x: &CronExpr) -> Option<Self> {
        let every = |f| x.is_every(f);
        let one = |f| {
            Some(x.values(f))
                .filter(|v| v.len() == 1)
                .map(|v| v[0] as u8)
        };
        let (minute, hour) = (one(Field::Minute), one(Field::Hour));
        let dates_any = every(Field::Month) && every(Field::DayOfMonth) && every(Field::DayOfWeek);

        if every(Field::Hour) && dates_any {
            if every(Field::Minute) {
                return Some(Self::EveryNMinutes(1));
            }
            if let Some(n) = x.step(Field::Minute) {
                return Some(Self::EveryNMinutes(n as u8));
            }
        }
        if let (Some(minute), true) = (minute, dates_any) {
            if every(Field::Hour) {
                return Some(Self::EveryNHours { n: 1, minute });
            }
            if let Some(n) = x.step(Field::Hour) {
                return Some(Self::EveryNHours { n: n as u8, minute });
            }
        }
        let (minute, hour) = (minute?, hour?);
        if dates_any {
            return Some(Self::Daily { hour, minute });
        }
        if every(Field::Month) && every(Field::DayOfMonth) {
            let days = x
                .values(Field::DayOfWeek)
                .iter()
                .fold(0u8, |m, d| m | 1 << d);
            return Some(Self::Weekly { days, hour, minute });
        }
        if every(Field::Month) && every(Field::DayOfWeek) {
            return Some(Self::Monthly {
                day: one(Field::DayOfMonth)?,
                hour,
                minute,
            });
        }
        if every(Field::DayOfWeek) {
            return Some(Self::Yearly {
                month: one(Field::Month)?,
                day: one(Field::DayOfMonth)?,
                hour,
                minute,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> String {
        describe(&s.parse().unwrap())
    }

    #[test]
    fn describes_common_schedules() {
        assert_eq!(d("* * * * *"), "Every minute");
        assert_eq!(d("*/15 * * * *"), "Every 15 minutes");
        assert_eq!(d("30 * * * *"), "At minute 30 past every hour");
        assert_eq!(d("0 */2 * * *"), "At minute 0 past every 2nd hour");
        assert_eq!(d("1 7 * * 1-5"), "At 07:01, on Monday through Friday");
        assert_eq!(d("0 0 * * *"), "At 00:00, every day");
        assert_eq!(d("0 9,12,15 * * *"), "At 09:00, 12:00 and 15:00, every day");
        assert_eq!(
            d("*/10 9-17 * * mon-fri"),
            "Every 10 minutes, between 09:00 and 17:59, on Monday through Friday"
        );
        assert_eq!(d("0 0 1,15 * *"), "At 00:00, on day 1 and 15 of the month");
        assert_eq!(
            d("0 8 * * 1,3,5"),
            "At 08:00, on Monday, Wednesday and Friday"
        );
        assert_eq!(
            d("0 0 1 1 *"),
            "At 00:00, on day 1 of the month, in January"
        );
        assert_eq!(
            d("0 0 13 * 5"),
            "At 00:00, on day 13 of the month and on Friday"
        );
        assert_eq!(d("0 12 * 6-8 *"), "At 12:00, in June, July and August");
        assert_eq!(d("5 0-23/4 * * *"), "At minute 5 past every 4th hour");
    }

    #[test]
    fn ordinals() {
        assert_eq!(
            [1, 2, 3, 4, 11, 12, 13, 21, 22].map(ordinal),
            ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd"]
        );
    }

    #[test]
    fn presets_round_trip() {
        let presets = [
            SimplePreset::EveryNMinutes(1),
            SimplePreset::EveryNMinutes(15),
            SimplePreset::EveryNHours { n: 1, minute: 30 },
            SimplePreset::EveryNHours { n: 6, minute: 0 },
            SimplePreset::Daily { hour: 7, minute: 1 },
            SimplePreset::Weekly {
                days: 0b011_1110,
                hour: 7,
                minute: 1,
            }, // Mon-Fri
            SimplePreset::Weekly {
                days: 0b100_0001,
                hour: 9,
                minute: 0,
            }, // Sat+Sun
            SimplePreset::Monthly {
                day: 15,
                hour: 2,
                minute: 30,
            },
            SimplePreset::Yearly {
                month: 12,
                day: 25,
                hour: 8,
                minute: 0,
            },
        ];
        for p in presets {
            assert_eq!(
                SimplePreset::from_expr(&p.to_expr()),
                Some(p),
                "{p:?} -> {}",
                p.to_expr()
            );
        }
        assert_eq!(
            SimplePreset::Weekly {
                days: 0b011_1110,
                hour: 7,
                minute: 1
            }
            .to_expr()
            .to_string(),
            "1 7 * * 1-5"
        );
    }

    #[test]
    fn custom_schedules_have_no_preset() {
        for s in [
            "0,10,25 * * * *",
            "0 9,17 * * *",
            "0 0 1,15 * *",
            "0 0 13 * fri",
            "*/15 9-17 * * *",
        ] {
            assert_eq!(SimplePreset::from_expr(&s.parse().unwrap()), None, "{s}");
        }
    }
}
