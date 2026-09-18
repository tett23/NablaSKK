//! skk-cli: type SKK in your terminal, backed by nablaskk-core.
//!
//! Runs the real input session (state machine, editors, dictionaries)
//! against raw terminal input. Ctrl-C quits.

use nablaskk_core::backend::Backend;
use nablaskk_core::bridge::{BufferedFrontEnd, CandidateWindow, NullWidgets};
use nablaskk_core::candidate::Candidate;
use nablaskk_core::config::Config;
use nablaskk_core::dictionary::{CommonDictionary, Encoding, LocalUserDictionary};
use nablaskk_core::event::modifier;
use nablaskk_core::input_mode::InputMode;
use nablaskk_core::keymap::Keymap;
use nablaskk_core::session::{Session, SessionParameter};
use nablaskk_core::trie::RomanKanaConverter;

use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;

// ======================================================================
// Raw terminal mode (no external crates: direct termios calls)
// ======================================================================

#[cfg(unix)]
mod raw_mode {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Termios {
        c_iflag: u64,
        c_oflag: u64,
        c_cflag: u64,
        c_lflag: u64,
        c_cc: [u8; 20],
        c_ispeed: u64,
        c_ospeed: u64,
    }

    extern "C" {
        fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
        fn cfmakeraw(termios: *mut Termios);
    }

    pub struct RawMode {
        saved: Termios,
    }

    impl RawMode {
        pub fn enter() -> Option<Self> {
            unsafe {
                let mut saved = std::mem::zeroed::<Termios>();
                if tcgetattr(0, &mut saved) != 0 {
                    return None;
                }

                let mut raw = saved;
                cfmakeraw(&mut raw);
                if tcsetattr(0, 0, &raw) != 0 {
                    return None;
                }

                Some(Self { saved })
            }
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            unsafe {
                tcsetattr(0, 0, &self.saved);
            }
        }
    }
}

// ======================================================================
// Candidate window rendered into a shared line buffer
// ======================================================================

#[derive(Default)]
struct TerminalWindowState {
    line: Option<String>,
}

struct TerminalWindow(Rc<RefCell<TerminalWindowState>>);

impl CandidateWindow for TerminalWindow {
    fn update(&mut self, view: &[Candidate], cursor: usize, page: usize, page_count: usize) {
        let mut line = String::new();

        for (index, candidate) in view.iter().enumerate() {
            let label = b"asdfjkl".get(index).map(|&c| c as char).unwrap_or('?');
            if index == cursor {
                line += &format!("[{label}:{}] ", candidate.variant());
            } else {
                line += &format!(" {label}:{}  ", candidate.variant());
            }
        }

        line += &format!("({page}/{page_count})");
        self.0.borrow_mut().line = Some(line);
    }

    fn show(&mut self) {}

    fn hide(&mut self) {
        self.0.borrow_mut().line = None;
    }
}

// ======================================================================

fn usage() -> ! {
    eprintln!(
        "usage: skk-cli [options] DICTIONARY...

Interactive SKK session in the terminal. Ctrl-C quits.

options:
  -u, --utf8            treat subsequent dictionaries as UTF-8 (default EUC-JP)
  --user PATH           user dictionary (default: ~/.rust-skk-jisyo)
  -h, --help            show this help"
    );
    std::process::exit(1);
}

fn mode_label(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Hirakana => "かな",
        InputMode::Katakana => "カナ",
        InputMode::Jisx0201Kana => "ｶﾅ",
        InputMode::Ascii => "英数",
        InputMode::Jisx0208Latin => "全英",
    }
}

fn main() {
    let mut encoding = Encoding::EucJp;
    let mut dictionaries: Vec<(String, Encoding)> = Vec::new();
    let mut user_path: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-u" | "--utf8" => encoding = Encoding::Utf8,
            "--user" => match args.next() {
                Some(path) => user_path = Some(path),
                None => usage(),
            },
            "-h" | "--help" => usage(),
            path if !path.starts_with('-') => dictionaries.push((path.to_string(), encoding)),
            _ => usage(),
        }
    }

    let user_path = user_path.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/.rust-skk-jisyo")
    });

    let mut backend = Backend::new(LocalUserDictionary::open(&user_path, Encoding::Utf8));
    for (path, encoding) in &dictionaries {
        match CommonDictionary::open(path, *encoding) {
            Ok(dictionary) => backend.add_dictionary(Box::new(dictionary)),
            Err(err) => {
                eprintln!("skk-cli: can't load {path}: {err}");
                std::process::exit(1);
            }
        }
    }

    let mut converter = RomanKanaConverter::new();
    converter.load(include_str!("../../../data/kana-rule.utf8.conf"));

    let mut keymap = Keymap::new();
    keymap.load(include_str!("../../../data/keymap.conf"));

    let frontend = Rc::new(RefCell::new(BufferedFrontEnd::default()));
    let window_state = Rc::new(RefCell::new(TerminalWindowState::default()));

    let mut session = Session::new(SessionParameter {
        backend,
        converter,
        config: Config::default(),
        frontend: Box::new(frontend.clone()),
        window: Box::new(TerminalWindow(window_state.clone())),
        messenger: Box::new(NullWidgets),
        clipboard: Box::new(NullWidgets),
        annotator: Box::new(NullWidgets),
        completor: Box::new(NullWidgets),
    });

    let Some(_raw) = raw_mode_guard() else {
        eprintln!("skk-cli: stdin is not a terminal");
        std::process::exit(1);
    };

    let mut stdout = std::io::stdout();
    let mut committed = String::new();

    let render = |stdout: &mut std::io::Stdout,
                  committed: &str,
                  session: &Session,
                  window_state: &RefCell<TerminalWindowState>| {
        let mut line = format!(
            "\r\x1b[K[{}] {}{}",
            mode_label(session.input_mode()),
            committed,
            session.composing_string()
        );

        if let Some(candidates) = window_state.borrow().line.as_deref() {
            line += &format!("\r\n\x1b[K    {candidates}\x1b[A\r");
            // Reposition: go to line start then re-print prefix to place cursor
        }

        let _ = stdout.write_all(line.as_bytes());
        let _ = stdout.flush();
    };

    write!(stdout, "skk-cli: Ctrl-C で終了 / l:英数 q:カナ 大文字:変換開始\r\n").ok();
    render(&mut stdout, &committed, &session, &window_state);

    let mut stdin = std::io::stdin();
    let mut buf = [0u8; 1];

    while stdin.read_exact(&mut buf).is_ok() {
        let byte = buf[0];

        // Ctrl-C quits
        if byte == 0x03 {
            break;
        }

        let event = match byte {
            // Arrow keys: ESC [ A/B/C/D -> Mac cursor char codes
            0x1b => {
                let mut seq = [0u8; 2];
                if stdin.read_exact(&mut seq).is_err() || seq[0] != b'[' {
                    continue;
                }
                let code = match seq[1] {
                    b'A' => 0x1e, // up
                    b'B' => 0x1f, // down
                    b'C' => 0x1d, // right
                    b'D' => 0x1c, // left
                    _ => continue,
                };
                keymap.fetch(code, 0, 0)
            }
            // Terminal backspace (0x7f) means backward delete
            0x7f => keymap.fetch(0x08, 0, 0),
            // Printable + the control codes keymap.conf maps directly
            0x08 | 0x09 | 0x0d | 0x20..=0x7e => keymap.fetch(byte, 0, 0),
            // Other control codes arrive as Ctrl+letter
            0x00..=0x1f => keymap.fetch(byte | 0x60, 0, modifier::CTRL),
            _ => continue,
        };

        session.handle_event(&event);

        let fixed = std::mem::take(&mut frontend.borrow_mut().fixed);
        committed += &fixed;

        render(&mut stdout, &committed, &session, &window_state);
    }

    write!(stdout, "\r\n").ok();
}

#[cfg(unix)]
fn raw_mode_guard() -> Option<raw_mode::RawMode> {
    raw_mode::RawMode::enter()
}

#[cfg(not(unix))]
fn raw_mode_guard() -> Option<()> {
    None
}
