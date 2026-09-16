//! Reading completion (port of `SKKCompleter`).
//!
//! The buddy callbacks of the original are replaced by [`Completer::current`],
//! which the caller syncs back to the composing editor.

use crate::backend::Backend;
use crate::candidate::{Candidate, CandidateSuite};
use crate::entry::Entry;

#[derive(Debug, Clone, Default)]
pub struct Completer {
    completions: Vec<String>,
    pos: usize,
}

impl Completer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Complete `query`. Returns true if any completions exist.
    pub fn execute(&mut self, backend: &Backend, query: &str, limit: usize) -> bool {
        self.pos = 0;
        self.completions = backend.complete(query, limit);
        !self.completions.is_empty()
    }

    pub fn current(&self) -> Option<&str> {
        self.completions.get(self.pos).map(String::as_str)
    }

    pub fn next(&mut self) {
        if self.completions.is_empty() {
            return;
        }

        self.pos = if self.pos + 1 < self.completions.len() { self.pos + 1 } else { 0 };
    }

    pub fn prev(&mut self) {
        if self.completions.is_empty() {
            return;
        }

        self.pos = if self.pos == 0 { self.completions.len() - 1 } else { self.pos - 1 };
    }

    /// Remove the current completion's entry from the user dictionary.
    /// Returns true when the entry is gone from every dictionary.
    pub fn remove(&mut self, backend: &mut Backend) -> bool {
        let Some(current) = self.current().map(str::to_string) else { return false };

        let entry = Entry::from_entry(&current);
        backend.remove(&entry, &Candidate::default());

        let mut suite = CandidateSuite::new();
        !backend.find(&entry, &mut suite)
    }
}
