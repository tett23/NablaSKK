//! SKK dictionaries (port of the `dictionary` and `backend` directories).

mod auto_update;
mod common;
mod factory;
mod file;
mod gadget;
mod proxy;
mod user;

pub use auto_update::AutoUpdateDictionary;
pub use common::CommonDictionary;
pub use factory::{create, DictionaryKey, DictionaryType, NullDictionary};
pub use file::{DictionaryEntry, DictionaryFile, Encoding};
pub use gadget::GadgetDictionary;
pub use proxy::ProxyDictionary;
pub use user::LocalUserDictionary;

use crate::candidate::CandidateSuite;
use crate::entry::Entry;
use std::collections::BTreeSet;

/// Abstract dictionary (port of `SKKBaseDictionary`). All strings are UTF-8.
pub trait Dictionary {
    /// Look up candidates for an entry.
    fn find(&self, entry: &Entry, result: &mut CandidateSuite);

    /// Look up the reading for a candidate word.
    fn reverse_lookup(&self, _candidate: &str) -> Option<String> {
        None
    }

    /// Collect completions for the helper's entry prefix.
    fn complete(&self, _helper: &mut CompletionHelper) {}
}

/// Completion accumulator (port of `SKKCompletionHelper` and the backend's
/// `CompletionHelper`): de-duplicates and applies length/count limits.
#[derive(Debug, Clone)]
pub struct CompletionHelper {
    entry: String,
    minimum_length: usize,
    limit: usize,
    needs_length_check: bool,
    seen: BTreeSet<String>,
    result: Vec<String>,
}

impl CompletionHelper {
    /// `limit` == 0 means unlimited.
    pub fn new(entry: &str, minimum_length: usize, limit: usize) -> Self {
        let needs_length_check = entry.chars().count() < minimum_length;
        let mut seen = BTreeSet::new();
        seen.insert(entry.to_string());

        Self {
            entry: entry.to_string(),
            minimum_length,
            limit,
            needs_length_check,
            seen,
            result: Vec::new(),
        }
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn add(&mut self, completion: &str) {
        if !self.can_continue() {
            return;
        }

        if self.needs_length_check && completion.chars().count() <= self.minimum_length {
            return;
        }

        if self.seen.insert(completion.to_string()) {
            self.result.push(completion.to_string());
        }
    }

    pub fn can_continue(&self) -> bool {
        self.limit == 0 || self.result.len() < self.limit
    }

    pub fn into_result(self) -> Vec<String> {
        self.result
    }
}
