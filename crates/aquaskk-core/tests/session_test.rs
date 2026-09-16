//! End-to-end session tests: keymap -> state machine -> editors -> backend.

use aquaskk_core::backend::Backend;
use aquaskk_core::bridge::{BufferedFrontEnd, NullWidgets};
use aquaskk_core::config::Config;
use aquaskk_core::dictionary::{CommonDictionary, Encoding, LocalUserDictionary};
use aquaskk_core::event::Event;
use aquaskk_core::input_mode::InputMode;
use aquaskk_core::keymap::Keymap;
use aquaskk_core::session::{Session, SessionParameter};
use aquaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::rc::Rc;

const SYSTEM_DICT: &str = "\
;; okuri-ari entries.
おくr /送/贈/[り/送/]/
つk /付/就/着/[く/付/]/[け/付/]/
;; okuri-nasi entries.
かんじ /漢字/幹事/
かんとう /関東/
すう /一/二/三/四/五/六/七/八/九/十/
";

struct Harness {
    session: Session,
    keymap: Keymap,
    frontend: Rc<RefCell<BufferedFrontEnd>>,
}

impl Harness {
    fn new() -> Self {
        Self::with_config(Config::default())
    }

    fn with_config(config: Config) -> Self {
        let dir = std::env::temp_dir().join("aquaskk-session-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("user-{:?}", std::thread::current().id()));
        std::fs::remove_file(&path).ok();

        let mut backend = Backend::new(LocalUserDictionary::open(&path, Encoding::Utf8));
        backend.add_dictionary(Box::new(CommonDictionary::from_str(SYSTEM_DICT)));

        let mut converter = RomanKanaConverter::new();
        converter.load(include_str!("../../../data/kana-rule.utf8.conf"));

        let mut keymap = Keymap::new();
        keymap.load(include_str!("../testdata/keymap.conf"));

        let frontend = Rc::new(RefCell::new(BufferedFrontEnd::default()));

        let session = Session::new(SessionParameter {
            backend,
            converter,
            config,
            frontend: Box::new(frontend.clone()),
            window: Box::new(NullWidgets),
            messenger: Box::new(NullWidgets),
            clipboard: Box::new(NullWidgets),
            annotator: Box::new(NullWidgets),
            completor: Box::new(NullWidgets),
        });

        Self { session, keymap, frontend }
    }

    fn event(&self, c: u8, mods: u32) -> Event {
        self.keymap.fetch(c, 0, mods)
    }

    /// Type a string of ASCII characters (uppercase produces shift events).
    fn type_str(&mut self, text: &str) {
        for c in text.bytes() {
            let event = self.event(c, 0);
            self.session.handle_event(&event);
        }
    }

    fn press_ctrl(&mut self, c: u8) -> bool {
        let event = self.keymap.fetch(c, 0, aquaskk_core::event::modifier::CTRL);
        self.session.handle_event(&event)
    }

    fn press_enter(&mut self) -> bool {
        let event = self.event(0x0d, 0);
        self.session.handle_event(&event)
    }

    fn fixed(&self) -> String {
        self.frontend.borrow().fixed.clone()
    }

    fn composing(&self) -> String {
        self.session.composing_string().to_string()
    }
}

#[test]
fn direct_kana_input() {
    let mut h = Harness::new();

    h.type_str("konnnitiha");
    assert_eq!(h.fixed(), "こんにちは");
    assert_eq!(h.composing(), "");
}

#[test]
fn sokuon_and_youon() {
    let mut h = Harness::new();

    h.type_str("kitte");
    assert_eq!(h.fixed(), "きって");

    h.type_str("gyunyu");
    assert!(h.fixed().contains("ぎゅにゅ"));
}

#[test]
fn composing_display() {
    let mut h = Harness::new();

    h.type_str("Kanji");
    assert_eq!(h.composing(), "▽かんじ");
    assert_eq!(h.fixed(), "");
}

#[test]
fn conversion_and_commit() {
    let mut h = Harness::new();

    h.type_str("Kanji");
    h.type_str(" ");
    assert_eq!(h.composing(), "▼漢字");

    h.press_enter();
    assert_eq!(h.fixed(), "漢字");
    assert_eq!(h.composing(), "");
}

#[test]
fn candidate_cycling() {
    let mut h = Harness::new();

    h.type_str("Kanji");
    h.type_str(" ");
    assert_eq!(h.composing(), "▼漢字");
    h.type_str(" ");
    assert_eq!(h.composing(), "▼幹事");

    // x goes back to the previous candidate
    h.type_str("x");
    assert_eq!(h.composing(), "▼漢字");

    h.press_enter();
    assert_eq!(h.fixed(), "漢字");
}

#[test]
fn learning_moves_candidate_to_front() {
    let mut h = Harness::new();

    h.type_str("Kanji  \r"); // select 幹事
    assert_eq!(h.fixed(), "幹事");

    h.type_str("Kanji ");
    assert_eq!(h.composing(), "▼幹事");
}

#[test]
fn okuri_ari_conversion() {
    let mut h = Harness::new();

    h.type_str("OkuRi");
    assert_eq!(h.composing(), "▼送り");

    h.press_enter();
    assert_eq!(h.fixed(), "送り");
}

#[test]
fn okuri_strict_match() {
    let mut h = Harness::new();

    // つk with け okurigana: strict hint narrows to 付け
    h.type_str("TuKe");
    assert_eq!(h.composing(), "▼付け");
}

#[test]
fn cancel_returns_to_composing() {
    let mut h = Harness::new();

    h.type_str("Kanji ");
    assert_eq!(h.composing(), "▼漢字");

    h.press_ctrl(b'g');
    assert_eq!(h.composing(), "▽かんじ");

    h.press_ctrl(b'g');
    assert_eq!(h.composing(), "");
    assert_eq!(h.fixed(), "");
}

#[test]
fn toggle_katakana() {
    let mut h = Harness::new();

    h.type_str("Kanji");
    h.type_str("q");
    assert_eq!(h.fixed(), "カンジ");
    assert_eq!(h.composing(), "");
}

#[test]
fn katakana_mode() {
    let mut h = Harness::new();

    h.type_str("q"); // toggle to katakana mode
    assert_eq!(h.session.input_mode(), InputMode::Katakana);

    h.type_str("aiueo");
    assert_eq!(h.fixed(), "アイウエオ");

    h.type_str("q");
    assert_eq!(h.session.input_mode(), InputMode::Hirakana);
}

#[test]
fn ascii_mode_passthrough() {
    let mut h = Harness::new();

    let handled = h.session.handle_event(&h.event(b'l', 0));
    assert!(handled);
    assert_eq!(h.session.input_mode(), InputMode::Ascii);

    // In ascii mode ordinary keys are not consumed
    let handled = h.session.handle_event(&h.event(b'a', 0));
    assert!(!handled);

    // Ctrl-J returns to hiragana
    h.press_ctrl(b'j');
    assert_eq!(h.session.input_mode(), InputMode::Hirakana);
}

#[test]
fn jisx0208_latin_mode() {
    let mut h = Harness::new();

    h.type_str("L"); // 全角英数モード
    assert_eq!(h.session.input_mode(), InputMode::Jisx0208Latin);

    h.type_str("abc");
    assert_eq!(h.fixed(), "ａｂｃ");
}

#[test]
fn registration_flow() {
    let mut h = Harness::new();

    // Unknown word starts registration
    h.type_str("Mikan ");
    assert!(h.composing().contains("[登録：みかん]"));

    // Type the word (kana input works in the nested session)
    h.type_str("mikan");
    assert!(h.composing().contains("みかん"));

    // Enter finishes registration and commits the word
    h.press_enter();
    assert_eq!(h.fixed(), "みかん");
    assert_eq!(h.composing(), "");

    // The word is learned
    h.type_str("Mikan ");
    assert_eq!(h.composing(), "▼みかん");
}

#[test]
fn registration_cancel() {
    let mut h = Harness::new();

    h.type_str("Mikan ");
    assert!(h.composing().contains("[登録：みかん]"));

    h.press_ctrl(b'g');
    // Back to composing the original entry
    assert_eq!(h.composing(), "▽みかん");
}

#[test]
fn backspace_editing() {
    let mut h = Harness::new();

    h.type_str("Kanji");
    h.press_ctrl(b'h');
    assert_eq!(h.composing(), "▽かん");

    h.press_ctrl(b'h');
    h.press_ctrl(b'h');
    // Setback to direct input
    h.press_ctrl(b'h');
    assert_eq!(h.composing(), "");
}

#[test]
fn abbrev_mode() {
    let mut h = Harness::new();

    h.type_str("/");
    h.type_str("test");
    assert_eq!(h.composing(), "▽test");
}

#[test]
fn completion() {
    let mut h = Harness::new();

    // Learn some entries first
    h.type_str("Kanji \r");
    h.type_str("Kantou \r");
    let _ = h.fixed();

    h.type_str("Kan");
    let event = h.event(0x09, 0); // Tab
    h.session.handle_event(&event);

    // Completion replaces the composing text with a learned entry
    let composing = h.composing();
    assert!(
        composing == "▽かんじ" || composing == "▽かんとう",
        "unexpected completion: {composing}"
    );
}

#[test]
fn window_selection() {
    let mut h = Harness::new();

    h.type_str("Suu");
    // Cycle past the 3 inline candidates into the window
    h.type_str("    "); // 一 二 三 四(window)
    assert_eq!(h.composing(), "▼四");

    h.press_enter();
    assert_eq!(h.fixed(), "四");
}

#[test]
fn egg_like_newline_default() {
    let mut h = Harness::new();

    h.type_str("Kanji ");
    let handled = h.press_enter();
    // Default: newline is not suppressed -> event unhandled after commit
    assert!(!handled);
    assert_eq!(h.fixed(), "漢字");
}

#[test]
fn egg_like_newline_suppressed() {
    let mut h = Harness::with_config(Config {
        suppress_newline_on_commit: true,
        ..Config::default()
    });

    h.type_str("Kanji ");
    let handled = h.press_enter();
    assert!(handled);
    assert_eq!(h.fixed(), "漢字");
}
