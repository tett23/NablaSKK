// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/bridge/SKKConfig.h
// Copyright (C) 2009 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Engine behavior settings (port of `SKKConfig`).

#[derive(Debug, Clone)]
pub struct Config {
    /// Fix the shortest kana match on commit (n -> ん) instead of dropping it.
    pub fix_intermediate_conversion: bool,
    /// Enable dynamic completion.
    pub enable_dynamic_completion: bool,
    /// Number of dynamic completion candidates to show.
    pub dynamic_completion_range: usize,
    /// Enable annotations.
    pub enable_annotation: bool,
    /// Display the shortest kana match while typing (n -> ん).
    pub display_shortest_match_of_kana_conversions: bool,
    /// Suppress the newline on commit (skk-egg-like-newline).
    pub suppress_newline_on_commit: bool,
    /// Maximum number of candidates shown inline before the window opens.
    pub max_count_of_inline_candidates: usize,
    /// Treat re-entering entry mode as the start of okuri.
    pub handle_recursive_entry_as_okuri: bool,
    /// Backspace during inline selection commits (skk-delete-implies-kakutei).
    pub inline_backspace_implies_commit: bool,
    /// Delete okurigana when canceling okuri input (skk-delete-okuri-when-quit).
    pub delete_okuri_when_quit: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fix_intermediate_conversion: true,
            enable_dynamic_completion: false,
            dynamic_completion_range: 5,
            enable_annotation: false,
            display_shortest_match_of_kana_conversions: false,
            suppress_newline_on_commit: false,
            max_count_of_inline_candidates: 3,
            handle_recursive_entry_as_okuri: false,
            inline_backspace_implies_commit: false,
            delete_okuri_when_quit: false,
        }
    }
}
