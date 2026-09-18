// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/selector/ (SKKSelector, SKKBaseSelector,
//   SKKInlineSelector, SKKWindowSelector)
// Copyright (C) 2008 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Candidate selection (port of the `selector` directory).

use crate::backend::Backend;
use crate::bridge::CandidateWindow;
use crate::candidate::{Candidate, CandidateSuite};
use crate::entry::Entry;

/// Inline candidate cursor over `candidates[..inline_count]`
/// (port of SKKInlineSelector).
#[derive(Debug, Clone, Default)]
struct InlineSelector {
    count: usize,
    pos: usize,
}

impl InlineSelector {
    fn initialize(&mut self, total: usize, inline_count: usize) {
        self.count = total.min(inline_count);
        self.pos = 0;
    }

    fn next(&mut self) -> bool {
        if self.count == 0 || self.pos + 1 == self.count {
            return false;
        }
        self.pos += 1;
        true
    }

    fn prev(&mut self) -> bool {
        if self.count == 0 || self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        true
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Paged window candidate cursor over `candidates[inline_count..]`
/// (port of SKKWindowSelector).
#[derive(Debug, Clone, Default)]
struct WindowSelector {
    offset_base: usize, // == inline_count
    total: usize,
    pages: Vec<usize>,
    page_pos: usize,
    cursor_pos: usize,
    offset: usize,
}

impl WindowSelector {
    fn initialize(&mut self, total: usize, inline_count: usize, pages: Vec<usize>) {
        self.offset_base = inline_count.min(total);
        self.total = total - self.offset_base;
        self.pages = pages;
        self.page_pos = 0;
        self.cursor_pos = 0;
        self.offset = 0;
    }

    fn view_len(&self) -> usize {
        self.pages.get(self.page_pos).copied().unwrap_or(0)
    }

    /// Index into the full candidate list for the view cursor.
    fn current_index(&self) -> usize {
        self.offset_base + self.offset + self.cursor_pos
    }

    fn view_range(&self) -> std::ops::Range<usize> {
        let start = self.offset_base + self.offset;
        start..(start + self.view_len()).min(self.offset_base + self.total)
    }

    fn next(&mut self) -> bool {
        if self.pages.is_empty() || self.page_pos + 1 == self.pages.len() {
            return false;
        }

        self.offset += self.pages[self.page_pos];
        self.page_pos += 1;
        self.cursor_pos = 0;
        true
    }

    fn prev(&mut self) -> bool {
        if self.page_pos == 0 {
            return false;
        }

        self.page_pos -= 1;
        self.offset -= self.pages[self.page_pos];
        self.cursor_pos = 0;
        true
    }

    fn is_empty(&self) -> bool {
        self.total == 0
    }

    fn cursor_left(&mut self) -> bool {
        if self.cursor_pos != 0 {
            self.cursor_pos -= 1;
            return true;
        }
        false
    }

    fn cursor_right(&mut self) -> bool {
        if self.cursor_pos + 1 < self.view_len() {
            self.cursor_pos += 1;
            return true;
        }
        false
    }

    fn cursor_up(&mut self) {
        self.cursor_pos = 0;
    }

    fn cursor_down(&mut self) {
        self.cursor_pos = self.view_len().saturating_sub(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Active {
    Inline,
    Window,
}

/// Candidate selection over inline display plus the candidate window
/// (port of SKKSelector). The `notify` callback of the original is
/// replaced by [`Selector::current`], which the caller syncs to the editor.
pub struct Selector {
    suite: CandidateSuite,
    active: Active,
    inline: InlineSelector,
    window: WindowSelector,
}

impl Default for Selector {
    fn default() -> Self {
        Self {
            suite: CandidateSuite::new(),
            active: Active::Inline,
            inline: InlineSelector::default(),
            window: WindowSelector::default(),
        }
    }
}

impl Selector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_inline(&self) -> bool {
        self.active == Active::Inline
    }

    /// Look up candidates for the entry. Returns true if any were found.
    pub fn execute(
        &mut self,
        backend: &Backend,
        window: &mut dyn CandidateWindow,
        entry: &Entry,
        inline_count: usize,
    ) -> bool {
        self.suite.clear();
        backend.find(entry, &mut self.suite);

        let total = self.suite.candidates().len();

        self.inline.initialize(total, inline_count);

        let window_candidates = &self.suite.candidates()[inline_count.min(total)..];
        let pages = window.setup(window_candidates);
        self.window.initialize(total, inline_count, pages);

        self.active = if !self.inline.is_empty() { Active::Inline } else { Active::Window };

        !self.suite.is_empty()
    }

    /// The currently selected candidate.
    pub fn current(&self) -> Option<&Candidate> {
        if self.suite.is_empty() {
            return None;
        }

        let index = match self.active {
            Active::Inline => self.inline.pos,
            Active::Window => self.window.current_index(),
        };

        self.suite.candidates().get(index)
    }

    pub fn next(&mut self, window: &mut dyn CandidateWindow) -> bool {
        let moved = match self.active {
            Active::Inline => self.inline.next(),
            Active::Window => self.window.next(),
        };

        if !moved {
            if self.active != Active::Inline || self.window.is_empty() {
                return false;
            }
            self.active = Active::Window;
        }

        if self.active == Active::Window {
            self.show(window);
        }

        true
    }

    pub fn prev(&mut self, window: &mut dyn CandidateWindow) -> bool {
        let moved = match self.active {
            Active::Inline => self.inline.prev(),
            Active::Window => self.window.prev(),
        };

        if !moved {
            if self.active == Active::Inline || self.inline.is_empty() {
                return false;
            }
            self.active = Active::Inline;
            window.hide();
        } else if self.active == Active::Window {
            self.show(window);
        }

        true
    }

    pub fn cursor_left(&mut self, window: &mut dyn CandidateWindow) {
        if self.active == Active::Window && self.window.cursor_left() {
            self.show(window);
        }
    }

    pub fn cursor_right(&mut self, window: &mut dyn CandidateWindow) {
        if self.active == Active::Window && self.window.cursor_right() {
            self.show(window);
        }
    }

    pub fn cursor_up(&mut self, window: &mut dyn CandidateWindow) {
        if self.active == Active::Window {
            self.window.cursor_up();
            self.show(window);
        }
    }

    pub fn cursor_down(&mut self, window: &mut dyn CandidateWindow) {
        if self.active == Active::Window {
            self.window.cursor_down();
            self.show(window);
        }
    }

    /// Select a window candidate by its label key.
    pub fn select(&mut self, window: &mut dyn CandidateWindow, label: u8) -> bool {
        if self.is_inline() {
            return false;
        }

        if let Some(index) = window.label_index(label) {
            if index < self.window.view_len() {
                self.window.cursor_pos = index;
                self.show(window);
                return true;
            }
        }

        false
    }

    pub fn show(&mut self, window: &mut dyn CandidateWindow) {
        if self.is_inline() {
            return;
        }

        let view = &self.suite.candidates()[self.window.view_range()];
        window.update(view, self.window.cursor_pos, self.window.page_pos + 1, self.window.pages.len());
        window.show();
    }

    pub fn hide(&mut self, window: &mut dyn CandidateWindow) {
        if self.is_inline() {
            return;
        }

        window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::NullWidgets;
    use crate::dictionary::{CommonDictionary, Encoding, LocalUserDictionary};

    fn backend(name: &str) -> Backend {
        let dir = std::env::temp_dir().join("aquaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::remove_file(&path).ok();

        let mut backend = Backend::new(LocalUserDictionary::open(&path, Encoding::Utf8));
        backend.add_dictionary(Box::new(CommonDictionary::from_text(
            ";; okuri-nasi entries.\nすう /一/二/三/四/五/六/七/八/九/十/\n",
        )));
        backend
    }

    #[test]
    fn inline_then_window() {
        let backend = backend("selector-inline");
        let mut selector = Selector::new();
        let mut window = NullWidgets;

        assert!(selector.execute(&backend, &mut window, &Entry::from_entry("すう"), 3));
        assert!(selector.is_inline());
        assert_eq!(selector.current().unwrap().word(), "一");

        // Move through the 3 inline candidates, then into the window
        assert!(selector.next(&mut window));
        assert!(selector.next(&mut window));
        assert_eq!(selector.current().unwrap().word(), "三");

        assert!(selector.next(&mut window));
        assert!(!selector.is_inline());
        assert_eq!(selector.current().unwrap().word(), "四");

        // Back to inline
        assert!(selector.prev(&mut window));
        assert!(selector.is_inline());
        assert_eq!(selector.current().unwrap().word(), "三");
    }

    #[test]
    fn exhausted() {
        let backend = backend("selector-exhausted");
        let mut selector = Selector::new();
        let mut window = NullWidgets;

        selector.execute(&backend, &mut window, &Entry::from_entry("すう"), 0);
        assert!(!selector.is_inline());

        // 10 candidates, one 7-item page + one 3-item page
        assert!(selector.next(&mut window));
        assert!(!selector.next(&mut window));

        assert!(selector.prev(&mut window));
        assert!(!selector.prev(&mut window));
    }

    #[test]
    fn not_found() {
        let backend = backend("selector-nf");
        let mut selector = Selector::new();
        let mut window = NullWidgets;

        assert!(!selector.execute(&backend, &mut window, &Entry::from_entry("ない"), 3));
        assert!(selector.current().is_none());
    }
}
