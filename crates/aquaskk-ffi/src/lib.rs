//! C ABI over [`aquaskk_core::session::Session`], for embedding the
//! engine in Swift / Objective-C hosts (e.g. a macOS InputMethodKit
//! input controller). See `include/aquaskk.h` for the C declarations.
//!
//! All strings crossing the boundary are NUL-terminated UTF-8. Strings
//! returned by `skk_*` functions are owned by the caller and must be
//! released with `skk_string_free`.

use aquaskk_core::backend::Backend;
use aquaskk_core::bridge::{BufferedFrontEnd, NullWidgets};
use aquaskk_core::config::Config;
use aquaskk_core::dictionary::{self, DictionaryKey, DictionaryType, Encoding, LocalUserDictionary};
use aquaskk_core::input_mode::InputMode;
use aquaskk_core::keymap::Keymap;
use aquaskk_core::session::{Session, SessionParameter};
use aquaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::rc::Rc;

pub struct SkkSession {
    session: Session,
    keymap: Keymap,
    frontend: Rc<RefCell<BufferedFrontEnd>>,
}

fn cstr<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

fn into_cstring(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

/// Create a session with the default kana rules and keymap embedded.
/// `user_dictionary_path` is created on first save; pass NULL for a
/// path in the user's home directory.
#[no_mangle]
pub extern "C" fn skk_session_new(user_dictionary_path: *const c_char) -> *mut SkkSession {
    let user_path = cstr(user_dictionary_path).map(str::to_string).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.rust-skk-jisyo")
    });

    let backend = Backend::new(LocalUserDictionary::open(&user_path, Encoding::Utf8));

    let mut converter = RomanKanaConverter::new();
    converter.load(include_str!("../../../data/kana-rule.utf8.conf"));

    let mut keymap = Keymap::new();
    keymap.load(include_str!("../../../data/keymap.conf"));

    let frontend = Rc::new(RefCell::new(BufferedFrontEnd::default()));

    let session = Session::new(SessionParameter {
        backend,
        converter,
        config: Config::default(),
        frontend: Box::new(frontend.clone()),
        window: Box::new(NullWidgets),
        messenger: Box::new(NullWidgets),
        clipboard: Box::new(NullWidgets),
        annotator: Box::new(NullWidgets),
        completor: Box::new(NullWidgets),
    });

    Box::into_raw(Box::new(SkkSession { session, keymap, frontend }))
}

/// # Safety
/// `session` must be a pointer returned by `skk_session_new`.
#[no_mangle]
pub unsafe extern "C" fn skk_session_free(session: *mut SkkSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Add a system dictionary. `dictionary_type` uses the original
/// DictionarySet numbering (0=EUC-JP, 1=auto-update, 2=skkserv,
/// 4=gadget, 5=UTF-8). Returns 0 on success.
///
/// # Safety
/// `session` must be a valid session pointer; `location` a valid C string.
#[no_mangle]
pub unsafe extern "C" fn skk_session_add_dictionary(
    session: *mut SkkSession,
    dictionary_type: i32,
    location: *const c_char,
) -> i32 {
    let Some(session) = session.as_mut() else { return -1 };
    let Some(location) = cstr(location) else { return -1 };
    let Some(dictionary_type) = DictionaryType::from_id(dictionary_type) else { return -1 };

    let key = DictionaryKey::new(dictionary_type, location);
    session.session.backend_mut().add_dictionary(dictionary::create(&key));

    0
}

/// Feed one key event. `mods` is a bitmask: shift=2, ctrl=4, alt=8,
/// meta=16 (matching keymap.conf). Returns 1 when the event was
/// consumed by the IME, 0 when the host should handle the key itself.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_handle(
    session: *mut SkkSession,
    charcode: u8,
    keycode: u8,
    mods: u32,
) -> i32 {
    let Some(session) = session.as_mut() else { return 0 };

    let event = session.keymap.fetch(charcode, keycode, mods);
    session.session.handle_event(&event) as i32
}

/// Text committed since the last call (transfers ownership; free with
/// `skk_string_free`).
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_take_fixed(session: *mut SkkSession) -> *mut c_char {
    let Some(session) = session.as_mut() else { return std::ptr::null_mut() };

    let fixed = std::mem::take(&mut session.frontend.borrow_mut().fixed);
    into_cstring(fixed)
}

/// The current marked (composing) text.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_composing(session: *const SkkSession) -> *mut c_char {
    let Some(session) = session.as_ref() else { return std::ptr::null_mut() };

    into_cstring(session.session.composing_string().to_string())
}

/// The current input mode: 0=hirakana 1=katakana 2=jisx0201kana
/// 3=ascii 4=jisx0208latin.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_input_mode(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };

    match session.session.input_mode() {
        InputMode::Hirakana => 0,
        InputMode::Katakana => 1,
        InputMode::Jisx0201Kana => 2,
        InputMode::Ascii => 3,
        InputMode::Jisx0208Latin => 4,
    }
}

/// Commit any pending text (it arrives via `skk_session_take_fixed`).
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_commit(session: *mut SkkSession) {
    if let Some(session) = session.as_mut() {
        session.session.commit();
    }
}

/// Reset to the initial state, discarding pending input.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_clear(session: *mut SkkSession) {
    if let Some(session) = session.as_mut() {
        session.session.clear();
    }
}

/// Flush the user dictionary to disk.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_save(session: *mut SkkSession) {
    if let Some(session) = session.as_mut() {
        session.session.backend_mut().user_dictionary_mut().save(true);
    }
}

/// Release a string returned by this library.
///
/// # Safety
/// `str_` must have been returned by an `skk_*` function, at most once.
#[no_mangle]
pub unsafe extern "C" fn skk_string_free(str_: *mut c_char) {
    if !str_.is_null() {
        drop(CString::from_raw(str_));
    }
}
