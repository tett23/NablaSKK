//! Shared (system) SKK dictionary
//! (port of `SKKCommonDictionary` / `SKKDictionaryKeeper`).

use super::file::{DictionaryEntry, DictionaryFile, Encoding};
use super::{CompletionHelper, Dictionary};
use crate::candidate::{parse_candidates, CandidateSuite};
use crate::entry::Entry;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A read-only dictionary loaded from a SKK-JISYO file, sorted for binary
/// search. Call [`CommonDictionary::reload_if_updated`] to pick up file
/// changes (replaces the original's background polling thread).
#[derive(Debug, Clone, Default)]
pub struct CommonDictionary {
    path: PathBuf,
    encoding: Encoding,
    last_update: Option<SystemTime>,
    file: DictionaryFile,
}

impl CommonDictionary {
    pub fn open(path: impl AsRef<Path>, encoding: Encoding) -> std::io::Result<Self> {
        let mut dictionary = Self {
            path: path.as_ref().to_path_buf(),
            encoding,
            last_update: None,
            file: DictionaryFile::new(),
        };
        dictionary.reload()?;
        Ok(dictionary)
    }

    #[cfg(test)]
    pub(crate) fn from_str(text: &str) -> Self {
        let mut file = DictionaryFile::new();
        file.load_from_str(text);
        file.sort();
        Self { file, ..Self::default() }
    }

    pub fn reload(&mut self) -> std::io::Result<()> {
        self.file.load(&self.path, self.encoding)?;
        self.file.sort();
        self.last_update = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        Ok(())
    }

    /// Reload if the file changed on disk. Returns true when reloaded.
    pub fn reload_if_updated(&mut self) -> std::io::Result<bool> {
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();

        if modified.is_some() && modified != self.last_update {
            self.reload()?;
            return Ok(true);
        }

        Ok(false)
    }

    fn fetch(container: &[DictionaryEntry], query: &str) -> Option<String> {
        container
            .binary_search_by(|entry| entry.0.as_str().cmp(query))
            .ok()
            .map(|index| container[index].1.clone())
    }

    fn find_okuri_ari(&self, query: &str) -> Option<String> {
        Self::fetch(self.file.okuri_ari(), query)
    }

    fn find_okuri_nasi(&self, query: &str) -> Option<String> {
        Self::fetch(self.file.okuri_nasi(), query)
    }
}

/// Shared lookup logic for file-based dictionaries
/// (port of `SKKDictionaryTemplate::Find`).
pub(crate) fn find_with(
    entry: &Entry,
    result: &mut CandidateSuite,
    okuri_ari: impl Fn(&str) -> Option<String>,
    okuri_nasi: impl Fn(&str) -> Option<String>,
) {
    let key = entry.entry_string();
    let mut suite = CandidateSuite::new();

    if entry.is_okuri_ari() {
        if let Some(line) = okuri_ari(key) {
            suite.parse(&line);
        }

        // Prefer candidates recorded for this exact okurigana.
        if let Some(mut strict) = suite.find_okuri_strictly(entry.okuri_string()) {
            strict.add_hints(suite.hints());
            suite = strict;
        }
    } else if let Some(line) = okuri_nasi(key) {
        suite.parse(&line);
    }

    result.add_suite(&suite);
}

/// Scan `container` for an entry whose candidates include `candidate`
/// (port of the keeper's `ReverseLookup`).
pub(crate) fn reverse_lookup_in(
    container: &[DictionaryEntry],
    candidate: &str,
) -> Option<String> {
    let needle = format!("/{candidate}");

    for (key, line) in container {
        if !line.contains(&needle) {
            continue;
        }

        let (candidates, _) = parse_candidates(line);
        if candidates.iter().any(|c| c.word() == candidate) {
            return Some(key.clone());
        }
    }

    None
}

impl Dictionary for CommonDictionary {
    fn find(&self, entry: &Entry, result: &mut CandidateSuite) {
        find_with(
            entry,
            result,
            |q| self.find_okuri_ari(q),
            |q| self.find_okuri_nasi(q),
        );
    }

    fn reverse_lookup(&self, candidate: &str) -> Option<String> {
        reverse_lookup_in(self.file.okuri_nasi(), candidate)
    }

    fn complete(&self, helper: &mut CompletionHelper) {
        let container = self.file.okuri_nasi();
        let prefix = helper.entry().to_string();

        let start = container.partition_point(|entry| entry.0.as_str() < prefix.as_str());

        for (key, _) in &container[start..] {
            if !key.starts_with(&prefix) {
                break;
            }

            helper.add(key);

            if !helper.can_continue() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
;; okuri-ari entries.
つk /付/就/着/[く/付/就/着/]/[け/付/]/
;; okuri-nasi entries.
かんじ /漢字/幹事/
かんとう /関東/巻頭/
かんり /管理/
";

    #[test]
    fn find_okuri_nasi() {
        let dict = CommonDictionary::from_str(SAMPLE);
        let mut suite = CandidateSuite::new();

        dict.find(&Entry::from_entry("かんじ"), &mut suite);
        assert_eq!(suite.candidates().len(), 2);
        assert_eq!(suite.candidates()[0].word(), "漢字");

        let mut suite = CandidateSuite::new();
        dict.find(&Entry::from_entry("そんざいしない"), &mut suite);
        assert!(suite.is_empty());
    }

    #[test]
    fn find_okuri_ari_strict() {
        let dict = CommonDictionary::from_str(SAMPLE);

        let mut entry = Entry::from_entry("つ");
        entry.set_okuri("k", "け");

        let mut suite = CandidateSuite::new();
        dict.find(&entry, &mut suite);

        // Strict okuri matching narrows down to け candidates
        assert_eq!(suite.candidates()[0].word(), "付");
        assert_eq!(suite.candidates().len(), 1);
    }

    #[test]
    fn reverse() {
        let dict = CommonDictionary::from_str(SAMPLE);
        assert_eq!(dict.reverse_lookup("幹事"), Some("かんじ".to_string()));
        assert_eq!(dict.reverse_lookup("ない"), None);
    }

    #[test]
    fn complete() {
        let dict = CommonDictionary::from_str(SAMPLE);

        let mut helper = CompletionHelper::new("かん", 0, 0);
        dict.complete(&mut helper);
        assert_eq!(helper.into_result(), vec!["かんじ", "かんとう", "かんり"]);

        let mut helper = CompletionHelper::new("かん", 0, 2);
        dict.complete(&mut helper);
        assert_eq!(helper.into_result().len(), 2);
    }
}
