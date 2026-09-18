// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/dictionary/SKKGadgetDictionary.{h,cpp}
// Copyright (C) 2009 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Program-execution dictionary (port of `SKKGadgetDictionary`):
//! dynamic candidates for today/now/jdate/=expr. Results are marked
//! avoid-study so they never enter the user dictionary.

use super::{CompletionHelper, Dictionary};
use crate::calculator;
use crate::candidate::{Candidate, CandidateSuite};
use crate::entry::Entry;

/// Broken-down local time (the fields of `struct tm` we need).
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalTime {
    pub year: i32,
    pub month: u32,   // 1-12
    pub day: u32,     // 1-31
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub weekday: u32, // 0 = Sunday
}

#[cfg(unix)]
fn local_now() -> LocalTime {
    // struct tm from <time.h>; the trailing fields (gmtoff, zone) are
    // covered by padding to stay comfortably within the real layout.
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const i8,
    }

    extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(t: *const i64, result: *mut Tm) -> *mut Tm;
    }

    unsafe {
        let mut now: i64 = 0;
        time(&mut now);

        let mut tm = std::mem::zeroed::<Tm>();
        if localtime_r(&now, &mut tm).is_null() {
            return LocalTime::default();
        }

        LocalTime {
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
            second: tm.tm_sec as u32,
            weekday: tm.tm_wday as u32,
        }
    }
}

#[cfg(not(unix))]
fn local_now() -> LocalTime {
    LocalTime::default()
}

static WEEKDAYS_JA: &[&str] = &["日", "月", "火", "水", "木", "金", "土"];
static WEEKDAYS_EN: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn today(_entry: &str, result: &mut Vec<String>) {
    let now = local_now();
    let weekday = now.weekday as usize % 7;

    result.push(format!(
        "{:04}/{:02}/{:02}({})",
        now.year, now.month, now.day, WEEKDAYS_EN[weekday]
    ));
    result.push(format!(
        "{:04} 年 {:02} 月 {:02} 日({})",
        now.year, now.month, now.day, WEEKDAYS_JA[weekday]
    ));
}

fn now(_entry: &str, result: &mut Vec<String>) {
    let now = local_now();

    result.push(format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second));
    result.push(format!("{:02} 時 {:02} 分 {:02} 秒", now.hour, now.minute, now.second));
}

// (era name, first year, first month, first day)
static ERAS: &[(&str, i32, u32, u32)] = &[
    ("令和", 2019, 5, 1),
    ("平成", 1989, 1, 8),
    ("昭和", 1926, 12, 25),
    ("大正", 1912, 7, 30),
    ("明治", 1868, 1, 25),
];

/// Convert a Western year to Japanese era years: "jdate:2019" ->
/// 令和元年 / 平成31年. The original left this handler empty; this port
/// implements it.
fn jdate(entry: &str, result: &mut Vec<String>) {
    let Some(year) = entry.strip_prefix("jdate:").and_then(|y| y.parse::<i32>().ok()) else {
        return;
    };

    for (name, first_year, _, _) in ERAS {
        if year >= *first_year {
            let era_year = year - first_year + 1;
            if era_year == 1 {
                result.push(format!("{name}元年"));
            } else {
                result.push(format!("{name}{era_year}年"));
            }

            // Transition years belong to two eras; include the earlier one
            if year > *first_year {
                break;
            }
        }
    }
}

fn calculate(entry: &str, result: &mut Vec<String>) {
    match calculator::run(&entry[1..]) {
        Ok(value) => result.push(calculator::format(value)),
        Err(err) => result.push(err.to_string()),
    }
}

type Handler = fn(&str, &mut Vec<String>);

static HANDLERS: &[(&str, Handler)] =
    &[("today", today), ("now", now), ("jdate:", jdate), ("=", calculate)];

/// Dictionary of dynamically computed candidates.
#[derive(Debug, Clone, Copy, Default)]
pub struct GadgetDictionary;

impl GadgetDictionary {
    pub fn new() -> Self {
        Self
    }
}

impl Dictionary for GadgetDictionary {
    fn find(&self, entry: &Entry, result: &mut CandidateSuite) {
        // Okuri-ari is not supported
        if entry.is_okuri_ari() {
            return;
        }

        let key = entry.entry_string();
        let mut words = Vec::new();

        for (prefix, handler) in HANDLERS {
            if key.starts_with(prefix) {
                handler(key, &mut words);
            }
        }

        for word in words {
            let mut candidate = Candidate::new(&word);
            candidate.set_avoid_study();
            result.add_candidate(&candidate);
        }
    }

    fn complete(&self, helper: &mut CompletionHelper) {
        let entry = helper.entry().to_string();

        for (prefix, _) in HANDLERS {
            if prefix.starts_with(&entry) {
                helper.add(prefix);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(key: &str) -> Vec<String> {
        let dictionary = GadgetDictionary::new();
        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry(key), &mut suite);
        suite.candidates().iter().map(|c| c.word().to_string()).collect()
    }

    #[test]
    fn today_and_now() {
        let words = find("today");
        assert_eq!(words.len(), 2);
        assert!(words[0].contains('/'));
        assert!(words[1].contains("年"));

        let words = find("now");
        assert_eq!(words.len(), 2);
        assert!(words[0].contains(':'));
    }

    #[test]
    fn calculation() {
        assert_eq!(find("=(3+2)*5"), vec!["25"]);
        assert_eq!(find("=9.6/2"), vec!["4.8"]);
        assert_eq!(find("=1/0"), vec!["計算エラー:ゼロ除算です"]);
    }

    #[test]
    fn japanese_era() {
        assert_eq!(find("jdate:2020"), vec!["令和2年"]);
        assert_eq!(find("jdate:2019"), vec!["令和元年", "平成31年"]);
        assert_eq!(find("jdate:1989"), vec!["平成元年", "昭和64年"]);
        assert_eq!(find("jdate:1970"), vec!["昭和45年"]);
    }

    #[test]
    fn avoid_study() {
        let dictionary = GadgetDictionary::new();
        let mut suite = CandidateSuite::new();
        dictionary.find(&Entry::from_entry("=1+1"), &mut suite);
        assert!(suite.candidates()[0].avoid_study());
    }

    #[test]
    fn completion() {
        let dictionary = GadgetDictionary::new();
        let mut helper = CompletionHelper::new("to", 0, 0);
        dictionary.complete(&mut helper);
        assert_eq!(helper.into_result(), vec!["today"]);
    }
}
