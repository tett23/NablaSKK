// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/bridge/ (SKKFrontEnd, SKKCandidateWindow, SKKMessenger,
//   SKKClipboard, SKKAnnotator, SKKDynamicCompletor)
// Copyright (C) 2007-2010 Tomotaka SUWA <tomotaka.suwa@gmail.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Host-application interfaces (port of the `bridge` directory).
//!
//! The engine drives the UI through these traits; the null implementations
//! allow headless use (tests, skkserv).

use crate::candidate::Candidate;

/// Text output to the client application (port of `SKKFrontEnd`).
pub trait FrontEnd {
    /// Commit text at the caret.
    fn insert_string(&mut self, str_: &str);

    /// Show a marked (composing) string. `cursor` is a character offset
    /// relative to the string end (0 = end).
    fn compose_string(&mut self, str_: &str, cursor: i32);

    /// Show a marked string highlighting the candidate range
    /// (`start`/`length` in characters).
    fn compose_string_with_range(&mut self, str_: &str, start: usize, length: usize) {
        let _ = (start, length);
        self.compose_string(str_, 0);
    }

    /// The currently selected text (used for undo/re-conversion).
    fn selected_string(&self) -> String {
        String::new()
    }
}

/// Candidate window (port of `SKKCandidateWindow`).
pub trait CandidateWindow {
    /// Divide the (window portion of the) candidates into pages, returning
    /// the number of candidates on each page.
    fn setup(&mut self, candidates: &[Candidate]) -> Vec<usize> {
        if candidates.is_empty() {
            Vec::new()
        } else {
            // Default: pages of 7, like the standard AquaSKK window
            let mut pages = Vec::new();
            let mut remain = candidates.len();
            while remain > 0 {
                let count = remain.min(7);
                pages.push(count);
                remain -= count;
            }
            pages
        }
    }

    /// Show the current page.
    fn update(&mut self, view: &[Candidate], cursor: usize, page: usize, page_count: usize) {
        let _ = (view, cursor, page, page_count);
    }

    /// Map a selection label key to an index in the current page.
    fn label_index(&self, label: u8) -> Option<usize> {
        b"asdfjkl".iter().position(|&c| c == label)
    }

    fn show(&mut self) {}
    fn hide(&mut self) {}
}

/// Status/feedback messages (port of `SKKMessenger`).
pub trait Messenger {
    fn send_message(&mut self, message: &str) {
        let _ = message;
    }
    fn beep(&mut self) {}
}

/// Clipboard access (port of `SKKClipboard`).
pub trait Clipboard {
    fn paste_string(&mut self) -> String {
        String::new()
    }
}

/// Annotation display (port of `SKKAnnotator`).
pub trait Annotator {
    fn update(&mut self, candidate: &Candidate, mark: usize) {
        let _ = (candidate, mark);
    }
    fn show(&mut self) {}
    fn hide(&mut self) {}
}

/// Dynamic completion display (port of `SKKDynamicCompletor`).
pub trait DynamicCompletor {
    /// `completions` is newline-joined; `common_prefix_length` in chars.
    fn update(&mut self, completions: &str, common_prefix_length: usize, mark: usize) {
        let _ = (completions, common_prefix_length, mark);
    }
    fn show(&mut self) {}
    fn hide(&mut self) {}
}

/// A frontend that collects committed/composed text, for tests and servers.
#[derive(Debug, Clone, Default)]
pub struct BufferedFrontEnd {
    pub fixed: String,
    pub composing: String,
    /// Cursor within `composing`, as a character offset from the end
    /// (0 = end, -1 = before the last character).
    pub cursor: i32,
    pub selection: String,
}

impl FrontEnd for BufferedFrontEnd {
    fn insert_string(&mut self, str_: &str) {
        self.fixed += str_;
    }

    fn compose_string(&mut self, str_: &str, cursor: i32) {
        self.composing = str_.to_string();
        self.cursor = cursor;
    }

    fn selected_string(&self) -> String {
        self.selection.clone()
    }
}

impl FrontEnd for std::rc::Rc<std::cell::RefCell<BufferedFrontEnd>> {
    fn insert_string(&mut self, str_: &str) {
        self.borrow_mut().insert_string(str_);
    }

    fn compose_string(&mut self, str_: &str, cursor: i32) {
        self.borrow_mut().compose_string(str_, cursor);
    }

    fn selected_string(&self) -> String {
        self.borrow().selected_string()
    }
}

/// No-op implementations for headless use.
#[derive(Debug, Clone, Default)]
pub struct NullWidgets;

impl CandidateWindow for NullWidgets {}
impl Messenger for NullWidgets {}
impl Clipboard for NullWidgets {}
impl Annotator for NullWidgets {}
impl DynamicCompletor for NullWidgets {}
