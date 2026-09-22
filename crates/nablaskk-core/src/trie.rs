// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/trie/SKKTrie.{h,cpp}
//   src/engine/trie/SKKRomanKanaConverter.{h,cpp}
// Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Romaji -> kana conversion rule tree
//! (port of `SKKTrie` / `SKKRomanKanaConverter`).

use crate::input_mode::InputMode;
use crate::jconv;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
struct Rule {
    hirakana: String,
    katakana: String,
    jisx0201kana: String,
    next: String,
}

impl Rule {
    fn kana(&self, mode: InputMode) -> &str {
        match mode {
            InputMode::Hirakana => &self.hirakana,
            InputMode::Katakana => &self.katakana,
            InputMode::Jisx0201Kana => &self.jisx0201kana,
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Node {
    rule: Option<Rule>,
    children: BTreeMap<u8, Node>,
}

impl Node {
    fn add(&mut self, key: &[u8], rule: Rule) {
        match key {
            [] => self.rule = Some(rule),
            [head, rest @ ..] => self.children.entry(*head).or_default().add(rest, rule),
        }
    }
}

/// Result of a single conversion pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversionResult {
    /// Converted (or passed-through) output.
    pub output: String,
    /// Unconsumed romaji, or the next-state string of the rule
    /// (e.g. "っ" rules leave the sokuon-producing consonant here).
    pub next: String,
    /// Kana for an incomplete prefix that is itself a valid rule
    /// (e.g. "n" while it could still become "ny").
    pub intermediate: String,
}

/// Rule tree loaded from kana-rule.conf (port of `SKKRomanKanaConverter`).
#[derive(Debug, Clone, Default)]
pub struct RomanKanaConverter {
    root: Node,
}

enum Step {
    /// Consumed `skip` bytes; `rule` is None on a rule-less partial match
    /// (e.g. "chm" with no "ch" rule), which emits nothing.
    Converted { rule: Option<Rule>, skip: usize },
    /// First byte matches nothing; pass it through.
    NotConverted { code: u8 },
    /// Queue ended mid-rule; wait for more input.
    Short { intermediate: Option<Rule> },
}

impl RomanKanaConverter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load rules from an EUC-JP (or ASCII) kana-rule file, replacing
    /// existing rules.
    pub fn initialize(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.root = Node::default();
        self.patch(path)
    }

    /// Merge rules from a file into the current tree.
    pub fn patch(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let bytes = std::fs::read(path)?;
        self.load(&jconv::utf8_from_eucj(&bytes));
        Ok(())
    }

    /// Merge rules from UTF-8 rule text.
    pub fn load(&mut self, text: &str) {
        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Escape spaces so they survive tokenization, then split on commas.
            let escaped = line.replace(' ', "&space;").replace(',', " ");
            let fields: Vec<String> = escaped
                .split_whitespace()
                .map(unescape)
                .collect();

            if let [roman, hirakana, katakana, jisx0201kana, rest @ ..] = fields.as_slice() {
                let rule = Rule {
                    hirakana: hirakana.clone(),
                    katakana: katakana.clone(),
                    jisx0201kana: jisx0201kana.clone(),
                    next: rest.first().cloned().unwrap_or_default(),
                };
                self.root.add(roman.as_bytes(), rule);
            }
        }
    }

    /// Convert a romaji string. Returns the result and whether the final
    /// pass matched a rule.
    pub fn convert(&self, mode: InputMode, input: &str) -> (ConversionResult, bool) {
        let mut result = ConversionResult::default();
        let mut output: Vec<u8> = Vec::new();
        let mut converted = false;
        let mut queue = input.as_bytes().to_vec();

        while !queue.is_empty() {
            result.next.clear();
            result.intermediate.clear();

            match self.traverse(&queue) {
                Step::Converted { rule, skip } => {
                    if let Some(rule) = &rule {
                        output.extend_from_slice(rule.kana(mode).as_bytes());
                        result.next = rule.next.clone();
                    }
                    converted = rule.is_some();
                    queue.drain(..skip);
                }
                Step::NotConverted { code } => {
                    converted = false;
                    output.push(code);
                    queue.drain(..1);
                }
                Step::Short { intermediate } => {
                    converted = false;
                    if let Some(rule) = intermediate {
                        result.intermediate = rule.kana(mode).to_string();
                    }
                    result.next = String::from_utf8_lossy(&queue).into_owned();
                    break;
                }
            }
        }

        result.output = String::from_utf8_lossy(&output).into_owned();

        (result, converted)
    }

    fn traverse(&self, queue: &[u8]) -> Step {
        let mut node = &self.root;
        let mut depth = 0;

        loop {
            if depth == queue.len() {
                return Step::Short { intermediate: node.rule.clone() };
            }

            match node.children.get(&queue[depth]) {
                Some(child) if !child.children.is_empty() => {
                    node = child;
                    depth += 1;
                }
                Some(child) => {
                    // Exact match at a terminal node.
                    return Step::Converted { rule: child.rule.clone(), skip: depth + 1 };
                }
                None if depth > 0 => {
                    // Partial match (e.g. "nk" -> ん + k, or rule-less "chm").
                    return Step::Converted { rule: node.rule.clone(), skip: depth };
                }
                None => {
                    return Step::NotConverted { code: queue[0] };
                }
            }
        }
    }
}

fn unescape(field: &str) -> String {
    field
        .replace("&comma;", ",")
        .replace("&space;", " ")
        .replace("&sharp;", "#")
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputMode::*;

    fn converter() -> RomanKanaConverter {
        let mut conv = RomanKanaConverter::new();
        conv.load(include_str!("../../../data/kana-rule.utf8.conf"));
        conv
    }

    #[test]
    fn simple() {
        let conv = converter();

        let (result, converted) = conv.convert(Hirakana, "a");
        assert!(converted);
        assert_eq!(result.output, "あ");
        assert_eq!(result.next, "");

        let (result, converted) = conv.convert(Hirakana, "gya");
        assert!(converted);
        assert_eq!(result.output, "ぎゃ");
    }

    #[test]
    fn intermediate_and_short() {
        let conv = converter();

        let (result, converted) = conv.convert(Hirakana, "k");
        assert!(!converted);
        assert_eq!(result.output, "");
        assert_eq!(result.next, "k");

        // "n" is both a rule (ん) and a prefix (ny...)
        let (result, _) = conv.convert(Hirakana, "n");
        assert_eq!(result.intermediate, "ん");
        assert_eq!(result.next, "n");
    }

    #[test]
    fn partial_match() {
        let conv = converter();

        // "nk" -> ん + unconsumed k
        let (result, converted) = conv.convert(Hirakana, "nk");
        assert!(!converted);
        assert_eq!(result.output, "ん");
        assert_eq!(result.next, "k");
    }

    #[test]
    fn sokuon_next_state() {
        let conv = converter();

        // "tt" -> っ with next state "t" (the caller feeds it back)
        let (result, converted) = conv.convert(Hirakana, "tt");
        assert!(converted);
        assert_eq!(result.output, "っ");
        assert_eq!(result.next, "t");
    }

    #[test]
    fn katakana_and_jisx0201() {
        let conv = converter();

        let (result, _) = conv.convert(Katakana, "a");
        assert_eq!(result.output, "ア");

        let (result, _) = conv.convert(Jisx0201Kana, "ga");
        assert_eq!(result.output, "ｶﾞ");
    }

    #[test]
    fn patch_overrides_rule() {
        let mut conv = converter();
        let (before, _) = conv.convert(Hirakana, ",");
        assert_eq!(before.output, "、");

        // comma.rule / period.rule from the original data/ directory
        conv.load("&comma;,，,，,&comma;\n.,．,．,.\n");
        let (comma, _) = conv.convert(Hirakana, ",");
        let (period, _) = conv.convert(Hirakana, ".");
        assert_eq!(comma.output, "，");
        assert_eq!(period.output, "．");
        // untouched rules survive the patch
        let (kana, _) = conv.convert(Hirakana, "ka");
        assert_eq!(kana.output, "か");
    }

    #[test]
    fn passthrough() {
        let conv = converter();

        let (result, converted) = conv.convert(Hirakana, "q1");
        assert!(!converted);
        assert!(result.output.contains('1') || !result.output.is_empty());
    }
}
