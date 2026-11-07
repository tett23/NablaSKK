// NablaSKK: WebAssembly build of the engine for the browser demo
// (ADR 0023).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

//! C ABI over [`nablaskk_core::session::Session`] for
//! wasm32-unknown-unknown, without wasm-bindgen: the module imports
//! nothing. Bytes cross the boundary as (pointer, length) in linear
//! memory. Buffers this module returns start with the body's length as
//! a little-endian u32, followed by the UTF-8 body; release them with
//! `nablaskk_free(ptr, 4 + length)`. The layout of the state and of the
//! rule results is in docs/specifications.md.
//!
//! Like `nablaskk-ffi`, a session is not thread-safe.

pub mod rules;

use nablaskk_core::backend::Backend;
use nablaskk_core::bridge::{Annotator, BufferedFrontEnd, CandidateWindow, NullWidgets};
use nablaskk_core::candidate::Candidate;
use nablaskk_core::config::Config;
use nablaskk_core::dictionary::{CommonDictionary, Encoding, LocalUserDictionary};
use nablaskk_core::input_mode::InputMode;
use nablaskk_core::jconv;
use nablaskk_core::keymap::Keymap;
use nablaskk_core::session::{Session, SessionParameter};
use nablaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::rc::Rc;

const BUILTIN_KEYMAP: &str = include_str!("../../../data/keymap.conf");
const BUILTIN_KANA_RULES: &str = include_str!("../../../data/kana-rule.utf8.conf");

/// Candidate window state (as in nablaskk-ffi).
#[derive(Debug, Clone, Default)]
struct WindowState {
    visible: bool,
    candidates: Vec<String>,
    cursor: usize,
    page: usize,
    page_count: usize,
}

struct SharedWindow(Rc<RefCell<WindowState>>);

impl CandidateWindow for SharedWindow {
    fn update(&mut self, view: &[Candidate], cursor: usize, page: usize, page_count: usize) {
        let mut state = self.0.borrow_mut();
        state.candidates = view.iter().map(|c| c.variant().to_string()).collect();
        state.cursor = cursor;
        state.page = page;
        state.page_count = page_count;
    }

    fn show(&mut self) {
        self.0.borrow_mut().visible = true;
    }

    // Unlike nablaskk-ffi, a hidden window reports no page either, so the
    // JSON does not carry the last page around
    fn hide(&mut self) {
        *self.0.borrow_mut() = WindowState::default();
    }
}

/// Annotation of the candidate being selected (as in nablaskk-ffi).
#[derive(Debug, Clone, Default)]
struct AnnotationState {
    visible: bool,
    annotation: String,
}

struct SharedAnnotator(Rc<RefCell<AnnotationState>>);

impl Annotator for SharedAnnotator {
    fn update(&mut self, candidate: &Candidate, _mark: usize) {
        self.0.borrow_mut().annotation = candidate.annotation().to_string();
    }

    fn show(&mut self) {
        let mut state = self.0.borrow_mut();
        state.visible = !state.annotation.is_empty();
    }

    fn hide(&mut self) {
        let mut state = self.0.borrow_mut();
        state.visible = false;
        state.annotation.clear();
    }
}

pub struct WasmSession {
    session: Session,
    keymap: Keymap,
    /// The rules the session started with: restored before user rules are
    /// applied, and the built-in table for their conflict checks.
    kana_rules: String,
    frontend: Rc<RefCell<BufferedFrontEnd>>,
    window: Rc<RefCell<WindowState>>,
    annotation: Rc<RefCell<AnnotationState>>,
}

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

unsafe fn text(ptr: *const u8, len: usize) -> String {
    String::from_utf8_lossy(bytes(ptr, len)).into_owned()
}

/// A length-prefixed buffer for the caller, released with `nablaskk_free`.
fn into_buffer(body: String) -> *mut u8 {
    let length = (body.len() as u32).to_le_bytes();
    let buffer: Box<[u8]> = length.iter().copied().chain(body.into_bytes()).collect();
    Box::into_raw(buffer) as *mut u8
}

/// A JSON string literal.
fn json_string(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn mode_name(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Hirakana => "hirakana",
        InputMode::Katakana => "katakana",
        InputMode::Jisx0201Kana => "jisx0201kana",
        InputMode::Ascii => "ascii",
        InputMode::Jisx0208Latin => "jisx0208latin",
    }
}

fn converter(rules: &str) -> RomanKanaConverter {
    let mut converter = RomanKanaConverter::new();
    converter.load(rules);
    converter
}

/// Allocate `len` bytes for the caller to fill; release with
/// `nablaskk_free(ptr, len)`.
#[no_mangle]
pub extern "C" fn nablaskk_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// Release a buffer from `nablaskk_alloc` (`len` as allocated) or one this
/// module returned (`len` = 4 + the body's length).
///
/// # Safety
/// `ptr` and `len` must describe such a buffer, released at most once.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
    }
}

/// Create a session. The keymap (keymap.conf format) and kana rules
/// (kana-rule.conf format) are UTF-8; an empty one means the built-in
/// file. The dictionary is SKK-JISYO bytes in EUC-JP or UTF-8, detected
/// here; empty means no dictionary. The user dictionary lives in memory.
///
/// # Safety
/// Each (pointer, length) must describe readable memory, or have length 0.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_session_new(
    keymap_ptr: *const u8,
    keymap_len: usize,
    rules_ptr: *const u8,
    rules_len: usize,
    dict_ptr: *const u8,
    dict_len: usize,
) -> *mut WasmSession {
    let keymap_text = text(keymap_ptr, keymap_len);
    let rules_text = text(rules_ptr, rules_len);
    let kana_rules = if rules_text.is_empty() { BUILTIN_KANA_RULES.to_string() } else { rules_text };

    let mut keymap = Keymap::new();
    keymap.load(if keymap_text.is_empty() { BUILTIN_KEYMAP } else { &keymap_text });

    let mut backend = Backend::new(LocalUserDictionary::in_memory());
    let dictionary = bytes(dict_ptr, dict_len);
    if !dictionary.is_empty() {
        let text = match Encoding::detect(dictionary) {
            Encoding::EucJp => jconv::utf8_from_eucj(dictionary),
            _ => String::from_utf8_lossy(dictionary).into_owned(),
        };
        backend.add_dictionary(Box::new(CommonDictionary::from_text(&text)));
    }

    let frontend = Rc::new(RefCell::new(BufferedFrontEnd::default()));
    let window = Rc::new(RefCell::new(WindowState::default()));
    let annotation = Rc::new(RefCell::new(AnnotationState::default()));

    // The page shows annotations; the rest are AquaSKK's defaults
    let config = Config { enable_annotation: true, ..Config::default() };

    let session = Session::new(SessionParameter {
        backend,
        converter: converter(&kana_rules),
        config,
        frontend: Box::new(frontend.clone()),
        window: Box::new(SharedWindow(window.clone())),
        messenger: Box::new(NullWidgets),
        clipboard: Box::new(NullWidgets),
        annotator: Box::new(SharedAnnotator(annotation.clone())),
        completor: Box::new(NullWidgets),
    });

    Box::into_raw(Box::new(WasmSession { session, keymap, kana_rules, frontend, window, annotation }))
}

/// # Safety
/// `session` must come from `nablaskk_session_new`, freed at most once.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_session_free(session: *mut WasmSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Feed one key, with the values macos/Sources/KeyTranslator.swift makes
/// (`mods`: shift=2, ctrl=4, alt=8, meta=16). Returns 1 when the engine
/// handled it, 0 when the page should let the key through.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_handle(session: *mut WasmSession, charcode: u8, keycode: u8, mods: u32) -> i32 {
    let Some(session) = session.as_mut() else { return 0 };
    let event = session.keymap.fetch(charcode, keycode, mods);
    session.session.handle_event(&event) as i32
}

/// The display state as JSON. The fixed text is taken: the next call
/// reports only what was fixed after this one.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_take_state(session: *mut WasmSession) -> *mut u8 {
    let Some(session) = session.as_mut() else { return std::ptr::null_mut() };

    let (fixed, cursor) = {
        let mut frontend = session.frontend.borrow_mut();
        (std::mem::take(&mut frontend.fixed), frontend.cursor)
    };
    let window = session.window.borrow();
    let items: Vec<String> = window.candidates.iter().map(|c| json_string(c)).collect();
    let annotation = session.annotation.borrow();
    let annotation = if annotation.visible { json_string(&annotation.annotation) } else { "null".to_string() };

    into_buffer(format!(
        concat!(
            "{{\"fixed\":{},\"composing\":{},\"cursor\":{},\"mode\":\"{}\",",
            "\"candidates\":{{\"visible\":{},\"items\":[{}],\"cursor\":{},\"page\":{},\"pageCount\":{}}},",
            "\"annotation\":{}}}"
        ),
        json_string(&fixed),
        json_string(session.session.composing_string()),
        cursor,
        mode_name(session.session.input_mode()),
        window.visible,
        items.join(","),
        window.cursor,
        window.page,
        window.page_count,
        annotation,
    ))
}

/// Replace the user's kana rules (kana-rule.conf lines, UTF-8): the
/// session's rules are restored, then the lines without problems or
/// conflicts are applied in order. Returns a JSON array with one result
/// per line that is not blank or a comment.
///
/// # Safety
/// `session` must be a valid session pointer, and (`ptr`, `len`) readable
/// memory or length 0.
#[no_mangle]
pub unsafe extern "C" fn nablaskk_set_user_rules(session: *mut WasmSession, ptr: *const u8, len: usize) -> *mut u8 {
    let Some(session) = session.as_mut() else { return std::ptr::null_mut() };

    let results = rules::check(&text(ptr, len), &rules::builtin_rules(&session.kana_rules));

    let mut converter = converter(&session.kana_rules);
    let applied: String = results
        .iter()
        .filter(|result| result.reason.is_none())
        .map(|result| format!("{}\n", result.text))
        .collect();
    converter.load(&applied);
    *session.session.converter_mut() = converter;

    let items: Vec<String> = results
        .iter()
        .map(|result| {
            format!(
                "{{\"line\":{},\"input\":{},\"applied\":{},\"reason\":{}}}",
                result.line,
                json_string(&result.input),
                result.reason.is_none(),
                result.reason.as_deref().map(json_string).unwrap_or_else(|| "null".to_string()),
            )
        })
        .collect();
    into_buffer(format!("[{}]", items.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c\nd\u{1}"), "\"a\\\"b\\\\c\\nd\\u0001\"");
        assert_eq!(json_string("▽かんじ"), "\"▽かんじ\"");
    }

    #[test]
    fn buffers_are_length_prefixed() {
        let ptr = into_buffer("漢字".to_string());
        let len = unsafe { u32::from_le_bytes(*(ptr as *const [u8; 4])) } as usize;
        assert_eq!(len, "漢字".len());
        assert_eq!(unsafe { bytes(ptr.add(4), len) }, "漢字".as_bytes());
        unsafe { nablaskk_free(ptr, 4 + len) };
    }
}
