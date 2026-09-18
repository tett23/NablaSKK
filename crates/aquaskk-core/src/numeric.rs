// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/backend/SKKNumericConverter.{h,cpp}
// Copyright (C) 2006-2008 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Numeric conversion (port of `SKKNumericConverter`).
//!
//! SKK dictionaries record numeric entries with `#` placeholders
//! (e.g. "だい#1" -> "第１"). This module normalizes lookup keys and
//! applies the recorded conversion type to candidates.

use crate::candidate::Candidate;

static ZENKAKU_DIGITS: &[&str] = &["０", "１", "２", "３", "４", "５", "６", "７", "８", "９"];
static KANJI_DIGITS: &[&str] = &["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
static DAIJI_DIGITS: &[&str] = &["", "壱", "弐", "参", "四", "伍", "六", "七", "八", "九"];

fn digit(c: u8) -> usize {
    (c - b'0') as usize
}

// 1024 -> １０２４
fn convert_type1(src: &str) -> String {
    src.bytes().filter(u8::is_ascii_digit).map(|c| ZENKAKU_DIGITS[digit(c)]).collect()
}

// 1024 -> 一〇二四
fn convert_type2(src: &str) -> String {
    src.bytes().filter(u8::is_ascii_digit).map(|c| KANJI_DIGITS[digit(c)]).collect()
}

// Positional kanji numerals; unit tables shared by type 3 and type 5.
fn positional_kanji(
    src: &str,
    digits: &[&str],
    unit1: &[&str],
    unit2: &[&str],
    zero: &str,
    explicit_one: bool,
) -> String {
    let bytes = src.as_bytes();

    if bytes == b"0" {
        return zero.to_string();
    }

    let mut result = String::new();
    let mut previous_len = 0;

    let start = src.find(|c| c != '0').unwrap_or(src.len());

    for i in start..bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_digit() {
            continue;
        }

        let d = digit(c);
        if d >= 2 || (!explicit_one && d == 1) {
            result += digits[d];
        }

        let distance = bytes.len() - i;

        if distance > 4 && (distance - 1).is_multiple_of(4) {
            // 万/億/兆... boundary digit
            if explicit_one && d == 1 {
                result += digits[1];
            }
            // Only add the unit if this 4-digit group produced something
            if previous_len < result.len() {
                result += unit1[(distance - 1) / 4];
                previous_len = result.len();
            }
        } else if distance > 1 {
            if d != 0 {
                // The 一 in 一千万
                if explicit_one && d == 1 && distance > 4 && (distance - 2) % 4 == 2 {
                    result += digits[1];
                }
                result += unit2[(distance - 2) % 4];
            }
        } else if explicit_one && d == 1 {
            result += digits[1];
        }
    }

    result
}

// 1024 -> 千二十四
fn convert_type3(src: &str) -> String {
    positional_kanji(
        src,
        KANJI_DIGITS,
        &["", "万", "億", "兆", "京", "垓"],
        &["十", "百", "千"],
        "〇",
        true,
    )
}

// 1024 -> 壱阡弐拾四
fn convert_type5(src: &str) -> String {
    positional_kanji(
        src,
        DAIJI_DIGITS,
        &["", "萬", "億", "兆", "京", "垓"],
        &["拾", "百", "阡"],
        "零",
        false,
    )
}

// 34 -> ３四 (shogi kifu)
fn convert_type9(src: &str) -> String {
    let mut chars = src.chars();
    let first: String = chars.next().map(|c| c.to_string()).unwrap_or_default();
    let second: String = chars.next().map(|c| c.to_string()).unwrap_or_default();
    convert_type1(&first) + &convert_type2(&second)
}

/// Splits numbers out of a lookup key and applies conversions to candidates.
#[derive(Debug, Clone, Default)]
pub struct NumericConverter {
    original: String,
    normalized: String,
    params: Vec<String>,
}

impl NumericConverter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalize the key, replacing digit runs with `#`.
    /// Returns true if the key contained numbers.
    pub fn setup(&mut self, query: &str) -> bool {
        self.params.clear();
        self.original = query.to_string();

        let mut normalized = String::new();
        let mut current_number = String::new();

        for c in query.chars() {
            if c.is_ascii_digit() {
                current_number.push(c);
            } else {
                if !current_number.is_empty() {
                    self.params.push(std::mem::take(&mut current_number));
                    normalized.push('#');
                }
                normalized.push(c);
            }
        }
        if !current_number.is_empty() {
            self.params.push(current_number);
            normalized.push('#');
        }

        self.normalized = normalized;

        !self.params.is_empty()
    }

    pub fn original_key(&self) -> &str {
        &self.original
    }

    pub fn normalized_key(&self) -> &str {
        if self.params.is_empty() {
            &self.original
        } else {
            &self.normalized
        }
    }

    /// Replace `#N` placeholders in the candidate with converted numbers,
    /// storing the result as the candidate's variant.
    pub fn apply(&self, candidate: &mut Candidate) {
        if self.params.is_empty() {
            return;
        }

        let src: Vec<char> = candidate.word().chars().collect();
        let mut result = String::new();
        let mut param_index = 0;
        let mut i = 0;

        while i < src.len() {
            if src[i] == '#'
                && i + 1 < src.len()
                && matches!(src[i + 1], '0'..='5' | '9')
                && param_index < self.params.len()
            {
                let param = &self.params[param_index];
                match src[i + 1] {
                    '0' => result += param,                         // as-is
                    '1' => result += &convert_type1(param),         // full-width
                    '2' => result += &convert_type2(param),         // kanji digits
                    '3' => result += &convert_type3(param),         // positional kanji
                    '4' => result += param,                         // re-lookup (unsupported)
                    '5' => result += &convert_type5(param),         // formal numerals
                    '9' => result += &convert_type9(param),         // kifu
                    _ => unreachable!(),
                }
                param_index += 1;
                i += 2;
            } else {
                result.push(src[i]);
                i += 1;
            }
        }

        candidate.set_variant(&result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup() {
        let mut conv = NumericConverter::new();
        assert!(conv.setup("だい12かい"));
        assert_eq!(conv.normalized_key(), "だい#かい");
        assert_eq!(conv.original_key(), "だい12かい");

        assert!(!conv.setup("かんじ"));
        assert_eq!(conv.normalized_key(), "かんじ");
    }

    #[test]
    fn conversions() {
        assert_eq!(convert_type1("1024"), "１０２４");
        assert_eq!(convert_type2("1024"), "一〇二四");
        assert_eq!(convert_type3("1024"), "千二十四");
        assert_eq!(convert_type3("0"), "〇");
        assert_eq!(convert_type3("10000"), "一万");
        assert_eq!(convert_type3("100000000"), "一億");
        assert_eq!(convert_type3("10000000"), "一千万");
        assert_eq!(convert_type3("111"), "百十一");
        assert_eq!(convert_type5("1024"), "壱阡弐拾四");
        assert_eq!(convert_type5("0"), "零");
        assert_eq!(convert_type9("34"), "３四");
    }

    #[test]
    fn apply() {
        let mut conv = NumericConverter::new();
        conv.setup("だい12かい");

        let mut candidate = Candidate::new("第#1回");
        conv.apply(&mut candidate);
        assert_eq!(candidate.variant(), "第１２回");
        assert_eq!(candidate.word(), "第#1回");

        let mut candidate = Candidate::new("第#3回");
        conv.apply(&mut candidate);
        assert_eq!(candidate.variant(), "第十二回");
    }
}
