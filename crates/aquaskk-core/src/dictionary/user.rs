//! User dictionary (port of `SKKLocalUserDictionary`).

use super::common::{find_with, reverse_lookup_in};
use super::file::{DictionaryEntry, DictionaryFile, Encoding};
use super::{CompletionHelper, Dictionary};
use crate::candidate::{Candidate, CandidateSuite, OkuriHint};
use crate::entry::Entry;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_IDLE_COUNT: u32 = 20;
const MAX_SAVE_INTERVAL: Duration = Duration::from_secs(60 * 5);

/// Writable dictionary keeping the user's learned candidates, ordered by
/// recency (most recent first). Saved back to disk with throttling.
#[derive(Debug)]
pub struct LocalUserDictionary {
    path: PathBuf,
    encoding: Encoding,
    idle_count: u32,
    last_save: Instant,
    file: DictionaryFile,
    private_mode: bool,
    dirty: bool,
}

fn find_entry(container: &[DictionaryEntry], query: &str) -> Option<usize> {
    container.iter().position(|entry| entry.0 == query)
}

impl LocalUserDictionary {
    /// Open (or create) a user dictionary. A missing file is not an error.
    pub fn open(path: impl AsRef<Path>, encoding: Encoding) -> Self {
        let mut file = DictionaryFile::new();

        if path.as_ref().exists() {
            if let Err(err) = file.load(&path, encoding) {
                eprintln!(
                    "LocalUserDictionary: can't load file {}: {err}",
                    path.as_ref().display()
                );
            }
        }

        let mut dictionary = Self {
            path: path.as_ref().to_path_buf(),
            encoding,
            idle_count: 0,
            last_save: Instant::now(),
            file,
            private_mode: false,
            dirty: false,
        };
        dictionary.fix();
        dictionary
    }

    /// Register a selected candidate, moving it to the front.
    pub fn register(&mut self, entry: &Entry, candidate: &Candidate) {
        if entry.is_okuri_ari() {
            let hint = OkuriHint {
                okuri: entry.okuri_string().to_string(),
                candidates: vec![Candidate::new(&candidate.to_string())],
            };

            Self::update(self.file.okuri_ari_mut(), entry.entry_string(), |suite| {
                suite.update_hint(&hint);
            });
        } else {
            let mut tmp = candidate.clone();
            tmp.encode_word();

            Self::update(self.file.okuri_nasi_mut(), entry.entry_string(), |suite| {
                suite.update_candidate(&tmp);
            });
        }

        self.dirty = true;
        self.save(false);
    }

    /// Remove a candidate from the entry.
    pub fn remove(&mut self, entry: &Entry, candidate: &Candidate) {
        if entry.is_okuri_ari() {
            let target = Candidate::new(&candidate.to_string());
            Self::remove_in(self.file.okuri_ari_mut(), entry.entry_string(), &target);
        } else {
            let mut tmp = candidate.clone();
            tmp.encode_word();
            let target = Candidate::new(&tmp.to_string());
            Self::remove_in(self.file.okuri_nasi_mut(), entry.entry_string(), &target);
        }

        self.dirty = true;
        self.save(false);
    }

    /// In private mode changes are neither recorded nor saved.
    pub fn set_private_mode(&mut self, flag: bool) {
        if self.private_mode != flag {
            if flag {
                self.save(true);
            } else if self.path.exists() {
                if let Err(err) = self.file.load(&self.path, self.encoding) {
                    eprintln!("LocalUserDictionary: reload failed: {err}");
                }
            }

            self.private_mode = flag;
        }
    }

    /// Persist the dictionary. `force` skips the idle/interval throttle.
    pub fn save(&mut self, force: bool) {
        if self.private_mode || !self.dirty {
            return;
        }

        if !force {
            self.idle_count += 1;
            if self.idle_count < MAX_IDLE_COUNT && self.last_save.elapsed() < MAX_SAVE_INTERVAL {
                return;
            }
        }

        self.idle_count = 0;
        self.last_save = Instant::now();

        let tmp_path = self.path.with_extension("tmp");

        if let Err(err) = self.file.save(&tmp_path, self.encoding) {
            eprintln!("LocalUserDictionary: can't save {}: {err}", tmp_path.display());
            return;
        }

        if let Err(err) = std::fs::rename(&tmp_path, &self.path) {
            eprintln!("LocalUserDictionary: rename() failed: {err}");
        } else {
            self.dirty = false;
        }
    }

    fn update(
        container: &mut Vec<DictionaryEntry>,
        index: &str,
        apply: impl FnOnce(&mut CandidateSuite),
    ) {
        let mut suite = CandidateSuite::new();

        if let Some(pos) = find_entry(container, index) {
            suite.parse(&container[pos].1);
            container.remove(pos);
        }

        apply(&mut suite);

        container.insert(0, (index.to_string(), suite.to_line(false)));
    }

    fn remove_in(container: &mut Vec<DictionaryEntry>, index: &str, candidate: &Candidate) {
        let Some(pos) = find_entry(container, index) else { return };

        let mut suite = CandidateSuite::new();
        suite.parse(&container[pos].1);
        suite.remove_candidate(candidate);

        if suite.is_empty() {
            container.remove(pos);
        } else {
            container[pos].1 = suite.to_line(false);
        }
    }

    fn fetch(&self, entry: &Entry, okuri_ari: bool) -> Option<String> {
        let container = if okuri_ari { self.file.okuri_ari() } else { self.file.okuri_nasi() };

        find_entry(container, entry.entry_string()).map(|pos| container[pos].1.clone())
    }

    // A bare "#" entry in the user dictionary is meaningless; drop it.
    fn fix(&mut self) {
        let container = self.file.okuri_nasi_mut();
        if let Some(pos) = find_entry(container, "#") {
            container.remove(pos);
        }
    }
}

impl Drop for LocalUserDictionary {
    fn drop(&mut self) {
        self.save(true);
    }
}

impl Dictionary for LocalUserDictionary {
    fn find(&self, entry: &Entry, result: &mut CandidateSuite) {
        let mut suite = CandidateSuite::new();

        find_with(
            entry,
            &mut suite,
            |_| self.fetch(entry, true),
            |_| self.fetch(entry, false),
        );

        if !entry.is_okuri_ari() {
            for candidate in suite.candidates_mut() {
                candidate.decode_word();
            }
        }

        result.add_suite(&suite);
    }

    fn reverse_lookup(&self, candidate: &str) -> Option<String> {
        reverse_lookup_in(self.file.okuri_nasi(), candidate)
    }

    fn complete(&self, helper: &mut CompletionHelper) {
        let prefix = helper.entry().to_string();

        for (key, _) in self.file.okuri_nasi() {
            if !key.starts_with(&prefix) {
                continue;
            }

            helper.add(key);

            if !helper.can_continue() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dict(name: &str) -> LocalUserDictionary {
        let dir = std::env::temp_dir().join("aquaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::remove_file(&path).ok();
        LocalUserDictionary::open(&path, Encoding::Utf8)
    }

    #[test]
    fn register_and_find() {
        let mut dict = temp_dict("user-register");

        dict.register(&Entry::from_entry("かんじ"), &Candidate::new("漢字"));
        dict.register(&Entry::from_entry("かんじ"), &Candidate::new("幹事"));

        let mut suite = CandidateSuite::new();
        dict.find(&Entry::from_entry("かんじ"), &mut suite);

        // Most recently registered comes first
        assert_eq!(suite.candidates()[0].word(), "幹事");
        assert_eq!(suite.candidates()[1].word(), "漢字");
    }

    #[test]
    fn register_okuri_ari() {
        let mut dict = temp_dict("user-okuri");

        let mut entry = Entry::from_entry("おく");
        entry.set_okuri("r", "り");

        dict.register(&entry, &Candidate::new("送"));

        let mut suite = CandidateSuite::new();
        dict.find(&entry, &mut suite);
        assert_eq!(suite.candidates()[0].word(), "送");
    }

    #[test]
    fn remove() {
        let mut dict = temp_dict("user-remove");

        dict.register(&Entry::from_entry("かんじ"), &Candidate::new("漢字"));
        dict.remove(&Entry::from_entry("かんじ"), &Candidate::new("漢字"));

        let mut suite = CandidateSuite::new();
        dict.find(&Entry::from_entry("かんじ"), &mut suite);
        assert!(suite.is_empty());
    }

    #[test]
    fn candidate_escaping() {
        let mut dict = temp_dict("user-escape");

        dict.register(&Entry::from_entry("すらっしゅ"), &Candidate::new("a/b"));

        let mut suite = CandidateSuite::new();
        dict.find(&Entry::from_entry("すらっしゅ"), &mut suite);
        assert_eq!(suite.candidates()[0].word(), "a/b");
    }

    #[test]
    fn save_and_reload() {
        let dir = std::env::temp_dir().join("aquaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user-save");
        std::fs::remove_file(&path).ok();

        {
            let mut dict = LocalUserDictionary::open(&path, Encoding::Utf8);
            dict.register(&Entry::from_entry("かんじ"), &Candidate::new("漢字"));
        } // Drop saves

        let dict = LocalUserDictionary::open(&path, Encoding::Utf8);
        let mut suite = CandidateSuite::new();
        dict.find(&Entry::from_entry("かんじ"), &mut suite);
        assert_eq!(suite.candidates()[0].word(), "漢字");

        std::fs::remove_file(&path).ok();
    }
}
