// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/entry/SKKCandidate.{h,cpp}
//   src/engine/entry/SKKCandidateParser.h
//   src/engine/entry/SKKCandidateSuite.h
//   src/engine/entry/SKKOkuriHint.h
// Copyright (C) 2007-2008 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Conversion candidates (port of `SKKCandidate`, `SKKCandidateParser`,
//! `SKKOkuriHint` and `SKKCandidateSuite`).

/// A single conversion candidate: word plus optional annotation.
#[derive(Debug, Clone, Default)]
pub struct Candidate {
    word: String,
    annotation: String,
    /// Numeric-conversion variant of the word.
    variant: String,
    /// Dynamically generated candidates are not saved to the user dictionary.
    avoid_study: bool,
}

impl Candidate {
    /// Parse "word;annotation" form.
    pub fn new(candidate: &str) -> Self {
        match candidate.split_once(';') {
            Some((word, annotation)) => Self {
                word: word.to_string(),
                annotation: annotation.to_string(),
                ..Self::default()
            },
            None => Self { word: candidate.to_string(), ..Self::default() },
        }
    }

    /// Use the string as the word verbatim, without annotation splitting.
    pub fn from_word(word: &str) -> Self {
        Self { word: word.to_string(), ..Self::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.word.is_empty()
    }

    pub fn word(&self) -> &str {
        &self.word
    }

    pub fn annotation(&self) -> &str {
        &self.annotation
    }

    pub fn variant(&self) -> &str {
        if self.variant.is_empty() {
            &self.word
        } else {
            &self.variant
        }
    }

    pub fn avoid_study(&self) -> bool {
        self.avoid_study
    }

    pub fn set_variant(&mut self, variant: &str) {
        self.variant = variant.to_string();
    }

    pub fn set_avoid_study(&mut self) {
        self.avoid_study = true;
    }

    /// Escape characters that conflict with dictionary syntax.
    pub fn encode(src: &str) -> String {
        src.replace('[', "[5b]").replace('/', "[2f]").replace(';', "[3b]")
    }

    pub fn decode(src: &str) -> String {
        src.replace("[5b]", "[").replace("[2f]", "/").replace("[3b]", ";")
    }

    pub fn encode_word(&mut self) {
        self.word = Self::encode(&self.word);
    }

    pub fn decode_word(&mut self) {
        self.word = Self::decode(&self.word);
    }
}

// Serializes back to the "word;annotation" dictionary form.
impl std::fmt::Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.annotation.is_empty() {
            write!(f, "{}", self.word)
        } else {
            write!(f, "{};{}", self.word, self.annotation)
        }
    }
}

// The annotation is not compared.
impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.variant() == other.variant()
    }
}

impl Eq for Candidate {}

/// Okuri hint: okurigana string paired with its candidates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OkuriHint {
    pub okuri: String,
    pub candidates: Vec<Candidate>,
}

/// Parse one dictionary line body of the form
/// "/word1/word2;annotation/.../[okuri/word/]/".
pub fn parse_candidates(line: &str) -> (Vec<Candidate>, Vec<OkuriHint>) {
    let mut candidates = Vec::new();
    let mut hints: Vec<OkuriHint> = Vec::new();

    let mut tmp = String::new();
    let mut hint = OkuriHint::default();
    let mut in_hint = false;

    for ch in line.chars() {
        if in_hint {
            // Okuri hint block: [okuri/word1/word2/]
            if tmp.is_empty() {
                match ch {
                    '/' | '[' => continue,
                    ']' => {
                        if !hint.candidates.is_empty() {
                            hints.push(std::mem::take(&mut hint));
                        } else {
                            hint = OkuriHint::default();
                        }
                        tmp.clear();
                        in_hint = false;
                        continue;
                    }
                    _ => {}
                }
            }

            if ch == '/' {
                if hint.okuri.is_empty() {
                    hint.okuri = std::mem::take(&mut tmp);
                } else {
                    hint.candidates.push(Candidate::new(&std::mem::take(&mut tmp)));
                }
                continue;
            }

            tmp.push(ch);
        } else {
            if ch == '/' {
                if !tmp.is_empty() {
                    candidates.push(Candidate::new(&std::mem::take(&mut tmp)));
                }
                continue;
            }

            if ch == '[' && tmp.is_empty() {
                in_hint = true;
                continue;
            }

            tmp.push(ch);
        }
    }

    (candidates, hints)
}

/// The full set of candidates for one dictionary entry
/// (port of `SKKCandidateSuite`).
#[derive(Debug, Clone, Default)]
pub struct CandidateSuite {
    candidates: Vec<Candidate>,
    hints: Vec<OkuriHint>,
}

impl CandidateSuite {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_line(line: &str) -> Self {
        let mut suite = Self::default();
        suite.parse(line);
        suite
    }

    pub fn parse(&mut self, line: &str) {
        let (candidates, hints) = parse_candidates(line);
        self.candidates = candidates;
        self.hints = hints;
    }

    pub fn clear(&mut self) {
        self.candidates.clear();
        self.hints.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn candidates_mut(&mut self) -> &mut Vec<Candidate> {
        &mut self.candidates
    }

    pub fn hints(&self) -> &[OkuriHint] {
        &self.hints
    }

    fn add_to(container: &mut Vec<Candidate>, src: &[Candidate]) {
        for item in src {
            if !item.is_empty() && !container.contains(item) {
                container.push(item.clone());
            }
        }
    }

    fn update_in(container: &mut Vec<Candidate>, candidate: &Candidate) {
        container.retain(|c| c != candidate);
        container.insert(0, candidate.clone());
    }

    pub fn add_candidate(&mut self, candidate: &Candidate) {
        Self::add_to(&mut self.candidates, std::slice::from_ref(candidate));
    }

    pub fn add_candidates(&mut self, candidates: &[Candidate]) {
        Self::add_to(&mut self.candidates, candidates);
    }

    pub fn add_hint(&mut self, hint: &OkuriHint) {
        match self.hints.iter_mut().find(|h| h.okuri == hint.okuri) {
            Some(existing) => Self::add_to(&mut existing.candidates, &hint.candidates),
            None => self.hints.push(hint.clone()),
        }
    }

    pub fn add_hints(&mut self, hints: &[OkuriHint]) {
        for hint in hints {
            self.add_hint(hint);
        }
    }

    pub fn add_suite(&mut self, suite: &CandidateSuite) {
        Self::add_to(&mut self.candidates, &suite.candidates);
        self.add_hints(&suite.hints);
    }

    /// Move (or insert) the candidate to the front.
    pub fn update_candidate(&mut self, candidate: &Candidate) {
        Self::update_in(&mut self.candidates, candidate);
    }

    /// Move the hint's first candidate to the front, updating okuri hints.
    pub fn update_hint(&mut self, hint: &OkuriHint) {
        let Some(first) = hint.candidates.first() else { return };

        Self::update_in(&mut self.candidates, first);

        match self.hints.iter_mut().find(|h| h.okuri == hint.okuri) {
            Some(existing) => Self::update_in(&mut existing.candidates, first),
            None => self.hints.insert(0, hint.clone()),
        }
    }

    pub fn remove_candidate(&mut self, candidate: &Candidate) {
        self.remove_if(|c| c == candidate);
    }

    pub fn remove_if(&mut self, pred: impl Fn(&Candidate) -> bool) {
        if !self.hints.is_empty() {
            let removed: Vec<Candidate> =
                self.candidates.iter().filter(|c| pred(c)).cloned().collect();

            for candidate in &removed {
                for hint in &mut self.hints {
                    hint.candidates.retain(|c| c != candidate);
                }
                self.hints.retain(|h| !h.candidates.is_empty());
            }
        }

        self.candidates.retain(|c| !pred(c));
    }

    /// Candidates recorded for exactly this okurigana.
    pub fn find_okuri_strictly(&self, okuri: &str) -> Option<CandidateSuite> {
        let hint = self.hints.iter().find(|h| h.okuri == okuri)?;

        let mut suite = CandidateSuite::new();
        suite.add_candidates(&hint.candidates);

        if suite.is_empty() {
            None
        } else {
            Some(suite)
        }
    }

    fn flatten(container: &[Candidate], exclude_avoid_study: bool) -> String {
        let mut result = String::new();

        for candidate in container {
            if exclude_avoid_study && candidate.avoid_study() {
                continue;
            }
            result.push('/');
            result += &candidate.to_string();
        }

        result
    }

    /// Serialize back to the dictionary line body.
    pub fn to_line(&self, exclude_avoid_study: bool) -> String {
        let mut result = Self::flatten(&self.candidates, exclude_avoid_study);

        for hint in &self.hints {
            result += "/[";
            result += &hint.okuri;
            result += &Self::flatten(&hint.candidates, exclude_avoid_study);
            result += "/]";
        }

        if !result.is_empty() {
            result.push('/');
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_parse() {
        let c = Candidate::new("候補;注釈");
        assert_eq!(c.word(), "候補");
        assert_eq!(c.annotation(), "注釈");
        assert_eq!(c.to_string(), "候補;注釈");

        let c = Candidate::from_word("候補;注釈");
        assert_eq!(c.word(), "候補;注釈");
    }

    #[test]
    fn candidate_encode() {
        assert_eq!(Candidate::encode("a/b;c[d"), "a[2f]b[3b]c[5b]d");
        assert_eq!(Candidate::decode("a[2f]b[3b]c[5b]d"), "a/b;c[d");
    }

    #[test]
    fn parser() {
        let (candidates, hints) = parse_candidates("/尽/[く/尽/]/[くす/尽/]/");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].word(), "尽");
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].okuri, "く");
        assert_eq!(hints[0].candidates[0].word(), "尽");
        assert_eq!(hints[1].okuri, "くす");
    }

    #[test]
    fn parser_annotations() {
        let (candidates, _) = parse_candidates("/川;river/河;annotation/");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].annotation(), "river");
    }

    #[test]
    fn suite_roundtrip() {
        let line = "/尽/[く/尽/]/";
        let suite = CandidateSuite::from_line(line);
        assert_eq!(suite.to_line(false), line);
    }

    #[test]
    fn suite_update_and_remove() {
        let mut suite = CandidateSuite::from_line("/一/二/三/");

        suite.update_candidate(&Candidate::new("二"));
        assert_eq!(suite.to_line(false), "/二/一/三/");

        suite.remove_candidate(&Candidate::new("一"));
        assert_eq!(suite.to_line(false), "/二/三/");

        suite.add_candidate(&Candidate::new("三"));
        assert_eq!(suite.to_line(false), "/二/三/");
    }

    #[test]
    fn suite_okuri_strict() {
        let suite = CandidateSuite::from_line("/尽/付/[く/尽/]/");

        let strict = suite.find_okuri_strictly("く").unwrap();
        assert_eq!(strict.candidates().len(), 1);
        assert_eq!(strict.candidates()[0].word(), "尽");

        assert!(suite.find_okuri_strictly("す").is_none());
    }
}
