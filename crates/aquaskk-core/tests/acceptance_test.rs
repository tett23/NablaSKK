//! Data-driven acceptance test: replays the original engine's
//! `SKKInputSession_TEST` suite (testdata/session_test.dat) against
//! the Rust session and compares every event's output.

use aquaskk_core::backend::Backend;
use aquaskk_core::bridge::{Clipboard, FrontEnd, NullWidgets};
use aquaskk_core::config::Config;
use aquaskk_core::dictionary::{Encoding, LocalUserDictionary};
use aquaskk_core::event::modifier;
use aquaskk_core::input_mode::InputMode;
use aquaskk_core::keymap::Keymap;
use aquaskk_core::session::{Session, SessionParameter};
use aquaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::rc::Rc;

// ======================================================================
// Mocks (ports of MockFrontEnd / MockClipboard)
// ======================================================================

#[derive(Debug, Clone, Default)]
struct RecordedOutput {
    fixed: String,
    marked: String,
    pos: i32,
    selection: String,
}

#[derive(Clone, Default)]
struct MockFrontEnd(Rc<RefCell<RecordedOutput>>);

impl FrontEnd for MockFrontEnd {
    fn insert_string(&mut self, str_: &str) {
        self.0.borrow_mut().fixed += str_;
    }

    fn compose_string(&mut self, str_: &str, cursor: i32) {
        let mut output = self.0.borrow_mut();
        output.marked = str_.to_string();
        output.pos = cursor;
    }

    fn compose_string_with_range(&mut self, str_: &str, _start: usize, _length: usize) {
        let mut output = self.0.borrow_mut();
        output.marked = str_.to_string();
        output.pos = 0;
    }

    fn selected_string(&self) -> String {
        self.0.borrow().selection.clone()
    }
}

#[derive(Clone, Default)]
struct MockClipboard(Rc<RefCell<String>>);

impl Clipboard for MockClipboard {
    fn paste_string(&mut self) -> String {
        self.0.borrow().clone()
    }
}

// ======================================================================
// test.dat parsing (port of TestData.h)
// ======================================================================

#[derive(Debug, Clone, Default)]
struct TestEvent {
    code: u8,
    mods: u32,
    selection: String,
    yank: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Expectation {
    fixed: String,
    marked: String,
    mode: Option<InputMode>,
    pos: i32,
    ret: bool,
}

impl Default for Expectation {
    fn default() -> Self {
        Self { fixed: String::new(), marked: String::new(), mode: None, pos: 0, ret: true }
    }
}

#[derive(Debug, Clone)]
struct TestEntry {
    line: usize,
    input: TestEvent,
    expected: Expectation,
}

fn parse_code(str_: &str) -> u8 {
    if let Some(hex) = str_.strip_prefix("0x") {
        u8::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        str_.bytes().next().unwrap_or(0)
    }
}

fn parse_mode(str_: &str) -> Option<InputMode> {
    Some(match str_ {
        "J" => InputMode::Hirakana,
        "K" => InputMode::Katakana,
        "Q" => InputMode::Jisx0201Kana,
        "A" => InputMode::Ascii,
        "L" => InputMode::Jisx0208Latin,
        _ => return None,
    })
}

fn parse_event(field: &str) -> TestEvent {
    let mut result = TestEvent::default();
    let mut parts = field.split(',');

    let key = parts.next().unwrap_or("");

    for option in parts {
        if let Some((name, value)) = option.split_once('=') {
            match name {
                "sel" => result.selection = value.to_string(),
                "yank" => result.yank = value.to_string(),
                _ => {}
            }
        }
    }

    let mut key_spec = "";
    for token in key.split("::") {
        match token {
            "shift" => result.mods |= modifier::SHIFT,
            "ctrl" => result.mods |= modifier::CTRL,
            "alt" => result.mods |= modifier::ALT,
            "meta" => result.mods |= modifier::META,
            other => key_spec = other,
        }
    }

    result.code = parse_code(key_spec);
    result
}

fn parse_expectation(field: &str) -> Expectation {
    let mut result = Expectation::default();

    for item in field.split(',') {
        if let Some((name, value)) = item.split_once('=') {
            match name {
                "fixed" => result.fixed = value.to_string(),
                "marked" => result.marked = value.replace("%20", " "),
                "mode" => result.mode = parse_mode(value),
                "pos" => result.pos = value.parse().unwrap_or(0),
                "ret" => result.ret = value == "1",
                _ => {}
            }
        }
    }

    result
}

fn load_tests(text: &str) -> Vec<TestEntry> {
    let mut tests = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let mut fields = line.split_whitespace();
        let (Some(event), Some(expected)) = (fields.next(), fields.next()) else { continue };

        if event.starts_with('#') {
            continue;
        }

        tests.push(TestEntry {
            line: index + 1,
            input: parse_event(event),
            expected: parse_expectation(expected),
        });
    }

    tests
}

// ======================================================================
// Runner (port of SKKInputSession_TEST)
// ======================================================================

#[test]
fn original_session_acceptance_suite() {
    let dir = std::env::temp_dir().join("aquaskk-acceptance-test");
    std::fs::create_dir_all(&dir).unwrap();
    let user_dict = dir.join("skk.jisyo");
    std::fs::remove_file(&user_dict).ok();

    let backend = Backend::new(LocalUserDictionary::open(&user_dict, Encoding::Utf8));

    let mut converter = RomanKanaConverter::new();
    let kana_rule = format!("{}/testdata/kana-rule.conf", env!("CARGO_MANIFEST_DIR"));
    converter.initialize(&kana_rule).unwrap();

    let mut keymap = Keymap::new();
    keymap.load(include_str!("../testdata/keymap.conf"));

    // MockConfig values from the original suite
    let config = Config {
        fix_intermediate_conversion: true,
        enable_dynamic_completion: false,
        dynamic_completion_range: 0,
        enable_annotation: false,
        display_shortest_match_of_kana_conversions: false,
        suppress_newline_on_commit: true,
        max_count_of_inline_candidates: 5,
        handle_recursive_entry_as_okuri: false,
        inline_backspace_implies_commit: false,
        delete_okuri_when_quit: true,
    };

    let frontend = MockFrontEnd::default();
    let clipboard = MockClipboard::default();
    let output = frontend.0.clone();
    let yank = clipboard.0.clone();

    let mut session = Session::new(SessionParameter {
        backend,
        converter,
        config,
        frontend: Box::new(frontend),
        window: Box::new(NullWidgets),
        messenger: Box::new(NullWidgets),
        clipboard: Box::new(clipboard),
        annotator: Box::new(NullWidgets),
        completor: Box::new(NullWidgets),
    });

    let tests = load_tests(include_str!("../testdata/session_test.dat"));
    assert_eq!(tests.len(), 400, "test data failed to load");

    let mut failures = Vec::new();

    for entry in &tests {
        {
            let mut recorded = output.borrow_mut();
            recorded.fixed.clear();
            recorded.marked.clear();
            recorded.pos = 0;
            recorded.selection = entry.input.selection.clone();
        }
        *yank.borrow_mut() = entry.input.yank.clone();

        let event = keymap.fetch(entry.input.code, 0, entry.input.mods);
        let ret = session.handle_event(&event);

        let recorded = output.borrow().clone();
        let actual = Expectation {
            fixed: recorded.fixed,
            marked: recorded.marked,
            mode: Some(session.input_mode()),
            pos: recorded.pos,
            ret,
        };

        let expected = Expectation {
            // The original suite specifies mode on every line
            mode: entry.expected.mode,
            ..entry.expected.clone()
        };

        if actual != expected {
            failures.push(format!(
                "line {}: input code=0x{:02x} mods={}\n  expected: {:?}\n    actual: {:?}",
                entry.line, entry.input.code, entry.input.mods, expected, actual
            ));
        }
    }

    std::fs::remove_file(&user_dict).ok();

    assert!(
        failures.is_empty(),
        "{} / {} cases failed:\n{}",
        failures.len(),
        tests.len(),
        failures.join("\n")
    );
}
