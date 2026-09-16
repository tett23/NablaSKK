//! aquaskk-core: Rust reimplementation of the AquaSKK input method engine.
//!
//! Ported from <https://github.com/codefirst/aquaskk> (GPL-2.0-or-later).

pub mod backend;
pub mod candidate;
pub mod dictionary;
pub mod entry;
pub mod event;
pub mod input_mode;
pub mod keymap;
pub mod jconv;
pub mod numeric;
pub mod trie;
