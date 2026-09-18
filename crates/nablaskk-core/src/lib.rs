//! nablaskk-core: Rust reimplementation of the AquaSKK input method engine.
//!
//! Ported from <https://github.com/codefirst/aquaskk> (GPL-2.0-or-later).

pub mod backend;
pub mod bridge;
pub mod calculator;
pub mod candidate;
pub mod completer;
pub mod config;
pub mod context;
pub mod dictionary;
pub mod editor;
pub mod entry;
pub mod event;
pub mod input_mode;
pub mod keymap;
pub mod machine;
pub mod jconv;
pub mod numeric;
pub mod selector;
pub mod session;
pub mod trie;
