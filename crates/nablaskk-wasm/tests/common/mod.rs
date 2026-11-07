// NablaSKK: native harness for the exported wasm functions (ADR 0023).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

#![allow(dead_code)]

use nablaskk_wasm::*;

pub const DICTIONARY: &[u8] = include_bytes!("../../../../data/SKK-JISYO.S");

pub const SHIFT: u32 = 2;
pub const CTRL: u32 = 4;

/// A session driven through the C ABI exactly as the page does: inputs
/// are copied into buffers from `nablaskk_alloc`, outputs are read from
/// length-prefixed buffers and released with `nablaskk_free`.
pub struct Demo(*mut WasmSession);

fn with_buffer<T>(bytes: &[u8], f: impl FnOnce(*mut u8, usize) -> T) -> T {
    let ptr = nablaskk_alloc(bytes.len());
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
    let result = f(ptr, bytes.len());
    unsafe { nablaskk_free(ptr, bytes.len()) };
    result
}

fn read_buffer(ptr: *mut u8) -> String {
    let len = unsafe { u32::from_le_bytes(*(ptr as *const [u8; 4])) } as usize;
    let text = unsafe { std::str::from_utf8(std::slice::from_raw_parts(ptr.add(4), len)) }
        .unwrap()
        .to_string();
    unsafe { nablaskk_free(ptr, 4 + len) };
    text
}

impl Demo {
    pub fn new(keymap: &str, rules: &str, dictionary: &[u8]) -> Self {
        let session = with_buffer(keymap.as_bytes(), |kp, kl| {
            with_buffer(rules.as_bytes(), |rp, rl| {
                with_buffer(dictionary, |dp, dl| unsafe { nablaskk_session_new(kp, kl, rp, rl, dp, dl) })
            })
        });
        assert!(!session.is_null());
        Demo(session)
    }

    /// The page's session: built-in keymap and rules, SKK-JISYO.S.
    pub fn standard() -> Self {
        Self::new("", "", DICTIONARY)
    }

    pub fn handle(&mut self, charcode: u8, keycode: u8, mods: u32) -> bool {
        unsafe { nablaskk_handle(self.0, charcode, keycode, mods) == 1 }
    }

    /// Type printable ASCII; capitals carry shift, as from a keyboard.
    pub fn type_str(&mut self, text: &str) {
        for c in text.bytes() {
            let mods = if c.is_ascii_uppercase() { SHIFT } else { 0 };
            self.handle(c, 0xff, mods);
        }
    }

    pub fn space(&mut self) {
        self.handle(0x20, 49, 0);
    }

    pub fn enter(&mut self) {
        self.handle(0x0d, 36, 0);
    }

    pub fn escape(&mut self) {
        self.handle(0x1b, 53, 0);
    }

    pub fn state(&mut self) -> String {
        read_buffer(unsafe { nablaskk_take_state(self.0) })
    }

    pub fn set_user_rules(&mut self, text: &str) -> String {
        let session = self.0;
        read_buffer(with_buffer(text.as_bytes(), |p, l| unsafe { nablaskk_set_user_rules(session, p, l) }))
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        unsafe { nablaskk_session_free(self.0) };
    }
}

/// Pull a string field out of the state JSON (enough for these tests;
/// the scenario file compares the whole JSON).
pub fn field<'a>(json: &'a str, name: &str) -> &'a str {
    let key = format!("\"{name}\":\"");
    let start = json.find(&key).unwrap_or_else(|| panic!("{name} in {json}")) + key.len();
    let rest = &json[start..];
    let mut end = 0;
    let bytes = rest.as_bytes();
    while bytes[end] != b'"' {
        end += if bytes[end] == b'\\' { 2 } else { 1 };
    }
    &rest[..end]
}

/// Keys in the scenario notation: printable ASCII as is (capitals with
/// shift), `<SPC>` `<RET>` `<ESC>` `<BS>` `<TAB>` `<DEL>` `<LEFT>`
/// `<RIGHT>` `<UP>` `<DOWN>` `<lt>`, and `<C-x>` for Ctrl+x. Returns
/// (charcode, keycode, mods) per key.
pub fn parse_keys(keys: &str) -> Vec<(u8, u8, u32)> {
    let mut result = Vec::new();
    let mut rest = keys;
    while let Some(c) = rest.chars().next() {
        if c == '<' {
            let end = rest.find('>').expect("unterminated <...>");
            let name = &rest[1..end];
            rest = &rest[end + 1..];
            result.push(match name {
                "SPC" => (0x20, 49, 0),
                "RET" => (0x0d, 36, 0),
                "TAB" => (0x09, 48, 0),
                "BS" => (0x08, 51, 0),
                "ESC" => (0x1b, 53, 0),
                "DEL" => (0x7f, 117, 0),
                "LEFT" => (0, 123, 0),
                "RIGHT" => (0, 124, 0),
                "DOWN" => (0, 125, 0),
                "UP" => (0, 126, 0),
                "lt" => (b'<', 0xff, SHIFT),
                ctrl if ctrl.starts_with("C-") && ctrl.len() == 3 => (ctrl.as_bytes()[2], 0xff, CTRL),
                other => panic!("unknown key <{other}>"),
            });
        } else {
            let byte = c as u8;
            assert!(c.is_ascii_graphic(), "printable ASCII only: {c:?}");
            let shifted = byte.is_ascii_uppercase() || b"~!@#$%^&*()_+{}|:\"<>?".contains(&byte);
            result.push((byte, 0xff, if shifted { SHIFT } else { 0 }));
            rest = &rest[1..];
        }
    }
    result
}
