// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/keymap/SKKKeymap.{h,cpp}
//   src/engine/keymap/SKKKeymapEntry.{h,cpp}
// Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! keymap.conf parsing and lookup (port of `SKKKeymap` / `SKKKeymapEntry`).

use crate::event::{modifier, Event, EventId, HandleOption, KeyState};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Symbol {
    Event(EventId),
    Attribute(u32),
    HandleOption(HandleOption),
}

fn lookup_symbol(name: &str) -> Option<Symbol> {
    use crate::event::attribute::*;
    use EventId::*;
    use Symbol::*;

    Some(match name {
        "SKK_JMODE" => Event(JMode),
        "SKK_ENTER" => Event(Enter),
        "SKK_CANCEL" => Event(Cancel),
        "SKK_BACKSPACE" => Event(Backspace),
        "SKK_DELETE" => Event(Delete),
        "SKK_TAB" => Event(Tab),
        "SKK_PASTE" => Event(Paste),
        "SKK_LEFT" => Event(Left),
        "SKK_RIGHT" => Event(Right),
        "SKK_UP" => Event(Up),
        "SKK_DOWN" => Event(Down),
        "SKK_CHAR" => Event(Char),
        "SKK_PING" => Event(Ping),
        "SKK_YES" => Event(Yes),
        "SKK_NO" => Event(No),
        "SKK_UNDO" => Event(Undo),

        "Direct" => Attribute(DIRECT),
        "UpperCases" => Attribute(UPPER_CASES),
        "ToggleKana" => Attribute(TOGGLE_KANA),
        "ToggleJisx0201Kana" => Attribute(TOGGLE_JISX0201_KANA),
        "SwitchToAscii" => Attribute(SWITCH_TO_ASCII),
        "SwitchToJisx0208Latin" => Attribute(SWITCH_TO_JISX0208_LATIN),
        "EnterJapanese" => Attribute(ENTER_JAPANESE),
        "EnterAbbrev" => Attribute(ENTER_ABBREV),
        "NextCompletion" => Attribute(NEXT_COMPLETION),
        "PrevCompletion" => Attribute(PREV_COMPLETION),
        "NextCandidate" => Attribute(NEXT_CANDIDATE),
        "PrevCandidate" => Attribute(PREV_CANDIDATE),
        "RemoveTrigger" => Attribute(REMOVE_TRIGGER),
        "InputChars" => Attribute(INPUT_CHARS),
        "CompConversion" => Attribute(COMP_CONVERSION),
        "StickyKey" => Attribute(STICKY_KEY),

        "AlwaysHandled" => HandleOption(crate::event::HandleOption::AlwaysHandled),
        "PseudoHandled" => HandleOption(crate::event::HandleOption::PseudoHandled),

        _ => return None,
    })
}

/// One "ConfigKey value||value" line parsed into key states
/// (port of `SKKKeymapEntry`).
struct KeymapEntry {
    symbol: Symbol,
    not: bool,
    keys: Vec<KeyState>,
}

impl KeymapEntry {
    fn parse(config_key: &str, config_value: &str) -> Option<Self> {
        let (not, key_name) = match config_key.strip_prefix("Not") {
            Some(rest) => (true, rest),
            None => (false, config_key),
        };

        let symbol = lookup_symbol(key_name)?;

        let mut keys = Vec::new();

        for spec in config_value.split("||") {
            // Labels are "::"-separated; the last token is the key itself.
            let mut group = false;
            let mut hex = false;
            let mut keycode = false;
            let mut mods = 0u32;
            let mut key_spec = "";

            for token in spec.split("::") {
                match token {
                    "group" => group = true,
                    "hex" => hex = true,
                    "keycode" => keycode = true,
                    "shift" => mods |= modifier::SHIFT,
                    "ctrl" => mods |= modifier::CTRL,
                    "alt" => mods |= modifier::ALT,
                    "meta" => mods |= modifier::META,
                    other => key_spec = other,
                }
            }

            let make_key = |code: u32| {
                if keycode {
                    KeyState::key_code(code, mods)
                } else {
                    KeyState::char_code(code, mods)
                }
            };

            let parse_code = |item: &str| -> Option<u32> {
                if hex || keycode {
                    let item = item.trim_start_matches("0x");
                    u32::from_str_radix(item, 16).ok()
                } else {
                    item.bytes().next().map(u32::from)
                }
            };

            if group {
                for item in key_spec.split(',') {
                    // A range like "A-K", or a single code
                    if let Some((from, to)) = item.split_once('-') {
                        if let (Some(from), Some(to)) = (parse_code(from), parse_code(to)) {
                            for code in from..=to {
                                keys.push(make_key(code));
                            }
                        }
                    } else if let Some(code) = parse_code(item) {
                        keys.push(make_key(code));
                    }
                }
            } else if let Some(code) = parse_code(key_spec) {
                keys.push(make_key(code));
            }
        }

        Some(Self { symbol, not, keys })
    }
}

/// Maps key states to events, attributes and handling options
/// (port of `SKKKeymap`).
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    events: HashMap<KeyState, EventId>,
    attributes: HashMap<KeyState, u32>,
    options: HashMap<KeyState, HandleOption>,
}

impl Keymap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        *self = Self::default();
        self.patch(path)
    }

    pub fn patch(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let text = std::fs::read_to_string(path)?;
        self.load(&text);
        Ok(())
    }

    /// Merge keymap definitions from config text.
    pub fn load(&mut self, text: &str) {
        for line in text.lines() {
            let mut tokens = line.split_whitespace();
            let (Some(key), Some(value)) = (tokens.next(), tokens.next()) else { continue };

            if key.starts_with('#') {
                continue;
            }

            let Some(entry) = KeymapEntry::parse(key, value) else {
                eprintln!("Keymap: invalid key name [{key}]");
                continue;
            };

            for state in entry.keys {
                match entry.symbol {
                    Symbol::Event(id) => {
                        self.events.insert(state, id);
                    }
                    Symbol::Attribute(bits) => {
                        if entry.not {
                            // e.g. "NotInputChars q" strips the attribute
                            if let Some(attr) = self.attributes.get_mut(&state) {
                                *attr &= !bits;
                            }
                        } else {
                            self.events.insert(state, EventId::Char);
                            *self.attributes.entry(state).or_insert(0) |= bits;
                        }
                    }
                    Symbol::HandleOption(option) => {
                        self.options.insert(state, option);
                    }
                }
            }
        }
    }

    /// Translate a raw key press into an [`Event`].
    pub fn fetch(&self, charcode: u8, keycode: u8, mods: u32) -> Event {
        let mut event = Event { code: charcode, ..Event::default() };

        event.id = *find(&self.events, charcode, keycode, mods).unwrap_or(&EventId::Char);

        if event.id == EventId::Char {
            if let Some(attr) = find(&self.attributes, charcode, keycode, mods) {
                event.attribute = *attr;
            }
        }

        if let Some(option) = find(&self.options, charcode, keycode, mods) {
            event.option = *option;
        }

        event
    }
}

fn find<T>(map: &HashMap<KeyState, T>, charcode: u8, keycode: u8, mods: u32) -> Option<&T> {
    // Key codes take precedence over character codes
    if let Some(value) = map.get(&KeyState::key_code(keycode.into(), mods)) {
        return Some(value);
    }

    if let Some(value) = map.get(&KeyState::char_code(charcode.into(), mods)) {
        return Some(value);
    }

    // Compatibility: retry printable characters without the shift modifier
    if charcode.is_ascii_graphic() && mods & modifier::SHIFT != 0 {
        return find(map, charcode, keycode, mods & !modifier::SHIFT);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::attribute::*;
    use crate::event::modifier::*;

    fn keymap() -> Keymap {
        let mut keymap = Keymap::new();
        keymap.load(include_str!("../testdata/keymap.conf"));
        keymap
    }

    #[test]
    fn events() {
        let keymap = keymap();

        assert_eq!(keymap.fetch(0, 0, 0), Event::new(EventId::Char, 0, 0));
        assert_eq!(keymap.fetch(b'j', 0, CTRL), Event::new(EventId::JMode, b'j', 0));
        assert_eq!(keymap.fetch(0x03, 0, 0), Event::new(EventId::Enter, 0x03, 0));
        assert_eq!(keymap.fetch(0x09, 0, 0), Event::new(EventId::Tab, 0x09, 0));
        assert_eq!(keymap.fetch(b'i', 0, CTRL), Event::new(EventId::Tab, b'i', 0));
        assert_eq!(keymap.fetch(b'g', 0, CTRL), Event::new(EventId::Cancel, b'g', 0));
        assert_eq!(keymap.fetch(0x1c, 0, 0), Event::new(EventId::Left, 0x1c, 0));
        assert_eq!(keymap.fetch(b'f', 0, CTRL), Event::new(EventId::Right, b'f', 0));
        assert_eq!(keymap.fetch(b'v', 0, META), Event::new(EventId::Paste, b'v', 0));
    }

    #[test]
    fn attributes() {
        let keymap = keymap();

        assert_eq!(keymap.fetch(b'b', 0, 0), Event::new(EventId::Char, b'b', INPUT_CHARS));
        assert_eq!(
            keymap.fetch(b'q', 0, 0),
            Event::new(EventId::Char, b'q', TOGGLE_KANA | INPUT_CHARS)
        );
        assert_eq!(
            keymap.fetch(b'q', 0, CTRL),
            Event::new(EventId::Char, b'q', TOGGLE_JISX0201_KANA)
        );
        assert_eq!(
            keymap.fetch(b'A', 0, 0),
            Event::new(EventId::Char, b'A', UPPER_CASES | INPUT_CHARS)
        );
        // Key codes take precedence: 0x51 is in the Direct group
        assert_eq!(keymap.fetch(b'1', 0x51, 0), Event::new(EventId::Char, b'1', DIRECT));
        assert_eq!(
            keymap.fetch(0x20, 0, SHIFT),
            Event::new(EventId::Char, 0x20, PREV_CANDIDATE | COMP_CONVERSION)
        );
    }

    #[test]
    fn patch() {
        let mut keymap = keymap();
        keymap.load(include_str!("../testdata/keymap_patch.conf"));

        // unchanged
        assert_eq!(keymap.fetch(b'b', 0, 0), Event::new(EventId::Char, b'b', INPUT_CHARS));
        // NotToggleKana removed the attribute
        assert_eq!(keymap.fetch(b'q', 0, 0), Event::new(EventId::Char, b'q', INPUT_CHARS));
        // added attribute
        assert_eq!(
            keymap.fetch(b'"', 0, 0),
            Event::new(EventId::Char, b'"', UPPER_CASES | INPUT_CHARS)
        );
    }
}
