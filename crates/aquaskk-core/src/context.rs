//! Shared input context (port of `SKKInputContext`, `SKKOutputBuffer`,
//! `SKKRegistration` and `SKKUndoContext`).

use crate::backend::Backend;
use crate::bridge::FrontEnd;
use crate::candidate::Candidate;
use crate::entry::Entry;

/// Composing-text output assembled during an event
/// (port of `SKKOutputBuffer`). Committed text is buffered in `fixed`
/// until [`OutputBuffer::output`] flushes it to the frontend.
#[derive(Debug, Clone, Default)]
pub struct OutputBuffer {
    fixed: String,
    composing: String,
    last: String,
    cursor: i32,
    mark: usize,
    start: usize,
    length: usize,
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

impl OutputBuffer {
    /// Commit text (flushed on `output`).
    pub fn fix(&mut self, str_: &str) {
        self.fixed += str_;
    }

    /// Append to the composing string. `cursor` is a relative character
    /// offset (<= 0) that accumulates.
    pub fn compose(&mut self, str_: &str, cursor: i32) {
        // utf8::push semantics: a non-negative accumulated cursor appends;
        // a negative one inserts that many characters back from the end.
        if self.cursor >= 0 || self.composing.is_empty() {
            self.composing += str_;
        } else {
            let chars: Vec<char> = self.composing.chars().collect();
            let pos = (chars.len() as i32 + self.cursor).max(0) as usize;
            let byte_pos = self
                .composing
                .char_indices()
                .nth(pos)
                .map(|(i, _)| i)
                .unwrap_or(self.composing.len());
            self.composing.insert_str(byte_pos, str_);
        }

        self.cursor += cursor;
    }

    /// Append the selected candidate, remembering its range for highlight.
    pub fn convert(&mut self, str_: &str) {
        let left_len = {
            let chars: Vec<char> = self.composing.chars().collect();
            (chars.len() as i32 + self.cursor.min(0)).max(0) as usize
        };
        self.start = left_len;
        self.length = char_len(str_);

        self.compose(str_, 0);
    }

    /// Record the position where an annotation/completion should anchor.
    pub fn set_mark(&mut self) {
        self.mark = (char_len(&self.composing) as i32 + self.cursor).max(0) as usize;
    }

    pub fn mark(&self) -> usize {
        self.mark
    }

    pub fn clear(&mut self) {
        self.composing.clear();
        self.cursor = 0;
        self.mark = 0;
        self.start = 0;
        self.length = 0;
    }

    /// Flush buffered output to the frontend.
    pub fn output(&mut self, frontend: &mut dyn FrontEnd) {
        if !self.fixed.is_empty() {
            frontend.insert_string(&self.fixed);
            self.fixed.clear();
        }

        if self.composing == self.last && self.composing.is_empty() {
            return;
        }

        if self.length != 0 {
            frontend.compose_string_with_range(&self.composing, self.start, self.length);
        } else {
            frontend.compose_string(&self.composing, self.cursor);
        }

        self.last = self.composing.clone();
    }

    pub fn is_composing(&self) -> bool {
        !self.composing.is_empty()
    }

    pub fn composing_string(&self) -> &str {
        &self.composing
    }
}

/// Word registration state shared between nested sessions
/// (port of `SKKRegistration`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegistrationState {
    #[default]
    None,
    Started,
    Finished,
    Aborted,
}

#[derive(Debug, Clone, Default)]
pub struct Registration {
    state: RegistrationState,
    word: String,
}

impl Registration {
    pub fn start(&mut self) {
        self.state = RegistrationState::Started;
    }

    pub fn finish(&mut self, word: &str) {
        self.state = RegistrationState::Finished;
        self.word = word.to_string();
    }

    pub fn abort(&mut self) {
        self.state = RegistrationState::Aborted;
        self.word.clear();
    }

    pub fn clear(&mut self) {
        self.state = RegistrationState::None;
        self.word.clear();
    }

    pub fn state(&self) -> RegistrationState {
        self.state
    }

    pub fn word(&self) -> &str {
        &self.word
    }
}

/// Undo (re-conversion) support (port of `SKKUndoContext`).
#[derive(Debug, Clone, Default)]
pub struct UndoContext {
    entry: String,
    candidate: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoResult {
    Failed,
    KanaEntry,
    AsciiEntry,
}

impl UndoContext {
    /// Reverse-look-up the selected text and prepare re-conversion.
    pub fn undo(&mut self, frontend: &dyn FrontEnd, backend: &Backend) -> UndoResult {
        self.candidate = frontend.selected_string();

        match backend.reverse_lookup(&self.candidate) {
            Some(entry) => self.entry = entry,
            None => {
                self.candidate.clear();
                self.entry.clear();
                return UndoResult::Failed;
            }
        }

        if self.entry.chars().any(|c| !c.is_ascii_graphic() && c != ' ') {
            UndoResult::KanaEntry
        } else {
            UndoResult::AsciiEntry
        }
    }

    pub fn is_active(&self) -> bool {
        !self.entry.is_empty()
    }

    pub fn clear(&mut self) {
        self.entry.clear();
        self.candidate.clear();
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn candidate(&self) -> &str {
        &self.candidate
    }
}

/// State shared across the editor stack (port of `SKKInputContext`).
#[derive(Debug, Clone, Default)]
pub struct InputContext {
    pub entry: Entry,
    pub candidate: Candidate,
    pub output: OutputBuffer,
    pub undo: UndoContext,
    pub registration: Registration,

    pub event_handled: bool,
    pub needs_setback: bool,
    pub dynamic_completion: bool,
    pub annotation: bool,
}
