// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/backend/SKKBackEnd.{h,cpp}
//   src/engine/backend/SKKCandidateFilter.h
// Copyright (C) 2008-2010 Tomotaka SUWA <tomotaka.suwa@gmail.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Dictionary search frontend (port of `SKKBackEnd`).
//!
//! Owns the user dictionary and the configured system dictionaries, and
//! layers numeric conversion, candidate filtering and completion on top.
//! Unlike the original this is a plain struct, not a singleton.

use crate::candidate::{Candidate, CandidateSuite};
use crate::dictionary::{CompletionHelper, Dictionary, LocalUserDictionary};
use crate::entry::Entry;
use crate::numeric::NumericConverter;

pub struct Backend {
    user_dictionary: LocalUserDictionary,
    dictionaries: Vec<Box<dyn Dictionary + Send>>,
    use_numeric_conversion: bool,
    enable_extended_completion: bool,
    minimum_completion_length: usize,
}

impl Backend {
    pub fn new(user_dictionary: LocalUserDictionary) -> Self {
        Self {
            user_dictionary,
            dictionaries: Vec::new(),
            use_numeric_conversion: false,
            enable_extended_completion: false,
            minimum_completion_length: 0,
        }
    }

    /// Add a system dictionary. Search order is the user dictionary first,
    /// then added dictionaries in order.
    pub fn add_dictionary(&mut self, dictionary: Box<dyn Dictionary + Send>) {
        self.dictionaries.push(dictionary);
    }

    pub fn user_dictionary(&self) -> &LocalUserDictionary {
        &self.user_dictionary
    }

    pub fn user_dictionary_mut(&mut self) -> &mut LocalUserDictionary {
        &mut self.user_dictionary
    }

    fn each_dictionary(&self) -> impl Iterator<Item = &dyn Dictionary> {
        std::iter::once(&self.user_dictionary as &dyn Dictionary)
            .chain(self.dictionaries.iter().map(|d| d.as_ref() as &dyn Dictionary))
    }

    /// Complete readings beginning with `key`. `limit` == 0 means unlimited.
    pub fn complete(&self, key: &str, limit: usize) -> Vec<String> {
        let mut helper = CompletionHelper::new(key, self.minimum_completion_length, limit);

        if key.is_empty() || !self.enable_extended_completion {
            self.user_dictionary.complete(&mut helper);
        } else {
            for dictionary in self.each_dictionary() {
                dictionary.complete(&mut helper);
            }
        }

        helper.into_result()
    }

    /// Look up candidates across all dictionaries.
    pub fn find(&self, entry: &Entry, result: &mut CandidateSuite) -> bool {
        result.clear();

        for dictionary in self.each_dictionary() {
            dictionary.find(entry, result);
        }

        if !entry.is_okuri_ari() {
            let mut converter = NumericConverter::new();

            if self.use_numeric_conversion && converter.setup(entry.entry_string()) {
                let numeric_entry = Entry::from_entry(converter.normalized_key());
                let mut suite = CandidateSuite::new();

                for dictionary in self.each_dictionary() {
                    dictionary.find(&numeric_entry, &mut suite);
                }

                for candidate in suite.candidates_mut() {
                    converter.apply(candidate);
                    result.candidates_mut().push(candidate.clone());
                }
            }

            // Drop a candidate identical to the input itself.
            result.remove_candidate(&Candidate::new(converter.original_key()));
        }

        // skk-ignore-dic-word support
        result.remove_if(|c| c.word().starts_with("(skk-ignore-dic-word "));

        !result.is_empty()
    }

    pub fn reverse_lookup(&self, candidate: &str) -> Option<String> {
        if candidate.is_empty() {
            return None;
        }

        self.each_dictionary().find_map(|d| d.reverse_lookup(candidate))
    }

    /// Record a selected candidate in the user dictionary.
    pub fn register(&mut self, entry: &Entry, candidate: &Candidate) {
        if entry.entry_string().is_empty()
            || (entry.is_okuri_ari() && (entry.okuri_string().is_empty() || candidate.is_empty()))
        {
            eprintln!("Backend: invalid registration received");
            return;
        }

        if candidate.avoid_study() {
            return;
        }

        let entry = self.normalize(entry);
        self.user_dictionary.register(&entry, candidate);
    }

    /// Remove a candidate from the user dictionary.
    pub fn remove(&mut self, entry: &Entry, candidate: &Candidate) {
        if entry.entry_string().is_empty() {
            eprintln!("Backend: invalid removal received");
            return;
        }

        let entry = self.normalize(entry);
        self.user_dictionary.remove(&entry, candidate);
    }

    pub fn use_numeric_conversion(&mut self, flag: bool) {
        self.use_numeric_conversion = flag;
    }

    pub fn enable_extended_completion(&mut self, flag: bool) {
        self.enable_extended_completion = flag;
    }

    pub fn enable_private_mode(&mut self, flag: bool) {
        self.user_dictionary.set_private_mode(flag);
    }

    pub fn set_minimum_completion_length(&mut self, length: usize) {
        self.minimum_completion_length = length;
    }

    fn normalize(&self, entry: &Entry) -> Entry {
        if entry.is_okuri_ari() {
            return entry.clone();
        }

        let mut converter = NumericConverter::new();
        let mut result = entry.clone();

        // Don't normalize a purely numeric reading on register/remove.
        if self.use_numeric_conversion && converter.setup(entry.entry_string()) {
            let normalized = converter.normalized_key();
            if normalized != "#" {
                result.set_entry(normalized);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{CommonDictionary, Encoding};

    const SYSTEM_DICT: &str = "\
;; okuri-ari entries.
つk /付/就/[く/付/就/]/
;; okuri-nasi entries.
かんじ /漢字/幹事/
だい# /第#0/第#1/第#3/
";

    fn temp_backend(name: &str) -> Backend {
        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::remove_file(&path).ok();

        let mut backend = Backend::new(LocalUserDictionary::open(&path, Encoding::Utf8));
        backend.add_dictionary(Box::new(CommonDictionary::from_text(SYSTEM_DICT)));
        backend
    }

    #[test]
    fn find_and_learn() {
        let mut backend = temp_backend("backend-learn");
        let entry = Entry::from_entry("かんじ");
        let mut suite = CandidateSuite::new();

        assert!(backend.find(&entry, &mut suite));
        assert_eq!(suite.candidates()[0].word(), "漢字");

        // Selecting 幹事 moves it to the front on the next lookup
        backend.register(&entry, &Candidate::new("幹事"));

        assert!(backend.find(&entry, &mut suite));
        assert_eq!(suite.candidates()[0].word(), "幹事");
    }

    #[test]
    fn numeric_conversion() {
        let mut backend = temp_backend("backend-numeric");
        backend.use_numeric_conversion(true);

        let entry = Entry::from_entry("だい12");
        let mut suite = CandidateSuite::new();

        assert!(backend.find(&entry, &mut suite));

        let variants: Vec<&str> = suite.candidates().iter().map(|c| c.variant()).collect();
        assert!(variants.contains(&"第12"));
        assert!(variants.contains(&"第１２"));
        assert!(variants.contains(&"第十二"));
    }

    #[test]
    fn completion() {
        let mut backend = temp_backend("backend-complete");

        backend.register(&Entry::from_entry("かんり"), &Candidate::new("管理"));
        backend.register(&Entry::from_entry("かんとう"), &Candidate::new("関東"));

        let result = backend.complete("かん", 0);
        assert_eq!(result, vec!["かんとう", "かんり"]);

        backend.enable_extended_completion(true);
        let result = backend.complete("かん", 0);
        assert!(result.contains(&"かんじ".to_string()));
    }

    #[test]
    fn ignore_dic_word() {
        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("backend-ignore");
        std::fs::remove_file(&path).ok();

        let mut backend = Backend::new(LocalUserDictionary::open(&path, Encoding::Utf8));
        backend.add_dictionary(Box::new(CommonDictionary::from_text(
            ";; okuri-nasi entries.\nてすと /(skk-ignore-dic-word \"テスト\")/試験/\n",
        )));

        let mut suite = CandidateSuite::new();
        assert!(backend.find(&Entry::from_entry("てすと"), &mut suite));
        assert_eq!(suite.candidates().len(), 1);
        assert_eq!(suite.candidates()[0].word(), "試験");
    }
}
