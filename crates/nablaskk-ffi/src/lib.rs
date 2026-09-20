//! C ABI over [`nablaskk_core::session::Session`], for embedding the
//! engine in Swift / Objective-C hosts (e.g. a macOS InputMethodKit
//! input controller). See `include/nablaskk.h` for the C declarations.
//!
//! All strings crossing the boundary are NUL-terminated UTF-8. Strings
//! returned by `skk_*` functions are owned by the caller and must be
//! released with `skk_string_free`.

use nablaskk_core::backend::Backend;
use nablaskk_core::bridge::{BufferedFrontEnd, CandidateWindow, Clipboard, NullWidgets};
use nablaskk_core::candidate::Candidate;
use nablaskk_core::config::Config;
use nablaskk_core::dictionary::{self, DictionaryKey, DictionaryType, Encoding, LocalUserDictionary};
use nablaskk_core::input_mode::InputMode;
use nablaskk_core::keymap::Keymap;
use nablaskk_core::session::{Session, SessionParameter};
use nablaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::rc::Rc;

/// Candidate window state mirrored for the host UI.
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

    fn hide(&mut self) {
        let mut state = self.0.borrow_mut();
        state.visible = false;
        state.candidates.clear();
    }
}

/// Clipboard contents supplied by the host (the engine cannot reach the
/// system pasteboard itself).
struct SharedClipboard(Rc<RefCell<String>>);

impl Clipboard for SharedClipboard {
    fn paste_string(&mut self) -> String {
        self.0.borrow().clone()
    }
}

pub struct SkkSession {
    session: Session,
    keymap: Keymap,
    frontend: Rc<RefCell<BufferedFrontEnd>>,
    window: Rc<RefCell<WindowState>>,
    clipboard: Rc<RefCell<String>>,
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
    let window = Rc::new(RefCell::new(WindowState::default()));
    let clipboard = Rc::new(RefCell::new(String::new()));

    let session = Session::new(SessionParameter {
        backend,
        converter,
        config: Config::default(),
        frontend: Box::new(frontend.clone()),
        window: Box::new(SharedWindow(window.clone())),
        messenger: Box::new(NullWidgets),
        clipboard: Box::new(SharedClipboard(clipboard.clone())),
        annotator: Box::new(NullWidgets),
        completor: Box::new(NullWidgets),
    });

    Box::into_raw(Box::new(SkkSession { session, keymap, frontend, window, clipboard }))
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
/// DictionarySet numbering (0=SKK-JISYO with EUC-JP/UTF-8 auto-detection,
/// 1=auto-update, 2=skkserv, 4=gadget, 5=UTF-8 forced). Returns 0 on success.
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

/// Set the text a paste event (Ctrl-Y / Cmd-V) will insert. Call this
/// with the system pasteboard contents before feeding the paste key.
///
/// # Safety
/// `session` must be a valid session pointer; `text` a valid C string.
#[no_mangle]
pub unsafe extern "C" fn skk_session_set_clipboard(session: *mut SkkSession, text: *const c_char) {
    let Some(session) = session.as_mut() else { return };

    *session.clipboard.borrow_mut() = cstr(text).unwrap_or("").to_string();
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

/// Cursor within the composing text, as a character offset from its end
/// (0 = end, -1 = before the last character). Non-zero while editing a
/// reading or a registration word with the cursor keys.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_composing_cursor(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };

    session.frontend.borrow().cursor
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

/// True while the candidate window should be shown.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_candidates_visible(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };
    session.window.borrow().visible as i32
}

/// Number of candidates on the current window page.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_candidate_count(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };
    session.window.borrow().candidates.len() as i32
}

/// Candidate at `index` on the current page (caller frees), or NULL.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_candidate(
    session: *const SkkSession,
    index: i32,
) -> *mut c_char {
    let Some(session) = session.as_ref() else { return std::ptr::null_mut() };

    match session.window.borrow().candidates.get(index as usize) {
        Some(candidate) => into_cstring(candidate.clone()),
        None => std::ptr::null_mut(),
    }
}

/// Cursor position within the current page.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_candidate_cursor(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };
    session.window.borrow().cursor as i32
}

/// Current page (1-based) and page count, packed as page << 16 | count.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_candidate_page(session: *const SkkSession) -> i32 {
    let Some(session) = session.as_ref() else { return 0 };
    let state = session.window.borrow();
    ((state.page as i32) << 16) | (state.page_count as i32 & 0xffff)
}

/// Boolean engine options for skk_session_set_option.
pub const SKK_OPTION_SUPPRESS_NEWLINE_ON_COMMIT: i32 = 0;
pub const SKK_OPTION_INLINE_BACKSPACE_IMPLIES_COMMIT: i32 = 1;
pub const SKK_OPTION_DELETE_OKURI_WHEN_QUIT: i32 = 2;
pub const SKK_OPTION_HANDLE_RECURSIVE_ENTRY_AS_OKURI: i32 = 3;
pub const SKK_OPTION_FIX_INTERMEDIATE_CONVERSION: i32 = 4;
pub const SKK_OPTION_DISPLAY_SHORTEST_MATCH: i32 = 5;
pub const SKK_OPTION_USE_NUMERIC_CONVERSION: i32 = 6;
pub const SKK_OPTION_MAX_INLINE_CANDIDATES: i32 = 7;

/// Set an engine option. Returns 0 on success.
///
/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn skk_session_set_option(
    session: *mut SkkSession,
    option: i32,
    value: i32,
) -> i32 {
    let Some(session) = session.as_mut() else { return -1 };
    let flag = value != 0;

    match option {
        SKK_OPTION_SUPPRESS_NEWLINE_ON_COMMIT => {
            session.session.config_mut().suppress_newline_on_commit = flag;
        }
        SKK_OPTION_INLINE_BACKSPACE_IMPLIES_COMMIT => {
            session.session.config_mut().inline_backspace_implies_commit = flag;
        }
        SKK_OPTION_DELETE_OKURI_WHEN_QUIT => {
            session.session.config_mut().delete_okuri_when_quit = flag;
        }
        SKK_OPTION_HANDLE_RECURSIVE_ENTRY_AS_OKURI => {
            session.session.config_mut().handle_recursive_entry_as_okuri = flag;
        }
        SKK_OPTION_FIX_INTERMEDIATE_CONVERSION => {
            session.session.config_mut().fix_intermediate_conversion = flag;
        }
        SKK_OPTION_DISPLAY_SHORTEST_MATCH => {
            session.session.config_mut().display_shortest_match_of_kana_conversions = flag;
        }
        SKK_OPTION_USE_NUMERIC_CONVERSION => {
            session.session.backend_mut().use_numeric_conversion(flag);
        }
        SKK_OPTION_MAX_INLINE_CANDIDATES => {
            session.session.config_mut().max_count_of_inline_candidates = value.max(0) as usize;
        }
        _ => return -1,
    }

    0
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
