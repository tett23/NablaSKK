//! Input events (port of `SKKEvent.h` / `SKKKeyState.h`).

/// Key input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EventId {
    #[default]
    Null,
    JMode,
    Enter,
    Cancel,
    Backspace,
    Delete,
    Tab,
    Paste,
    Left,
    Right,
    Up,
    Down,
    Char,
    Ping,
    Undo,
    AsciiMode,
    HirakanaMode,
    KatakanaMode,
    Jisx0201KanaMode,
    Jisx0208LatinMode,
    Yes,
    No,
    On,
    Off,
}

/// SKK_CHAR attribute bits.
pub mod attribute {
    pub const NONE: u32 = 0;
    pub const DIRECT: u32 = 1 << 0;
    pub const UPPER_CASES: u32 = 1 << 1;
    pub const TOGGLE_KANA: u32 = 1 << 2;
    pub const TOGGLE_JISX0201_KANA: u32 = 1 << 3;
    pub const SWITCH_TO_ASCII: u32 = 1 << 4;
    pub const SWITCH_TO_JISX0208_LATIN: u32 = 1 << 5;
    pub const ENTER_JAPANESE: u32 = 1 << 6;
    pub const ENTER_ABBREV: u32 = 1 << 7;
    pub const NEXT_COMPLETION: u32 = 1 << 8;
    pub const PREV_COMPLETION: u32 = 1 << 9;
    pub const NEXT_CANDIDATE: u32 = 1 << 10;
    pub const PREV_CANDIDATE: u32 = 1 << 11;
    pub const REMOVE_TRIGGER: u32 = 1 << 12;
    pub const INPUT_CHARS: u32 = 1 << 13;
    pub const COMP_CONVERSION: u32 = 1 << 14;
    pub const STICKY_KEY: u32 = 1 << 15;
}

/// Handling options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandleOption {
    #[default]
    Default,
    /// Report the key as handled even if nothing consumed it.
    AlwaysHandled,
    /// Process the key but report it as unhandled.
    PseudoHandled,
    CapsLock,
}

/// A keyboard event routed through the keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Event {
    pub id: EventId,
    pub code: u8,
    pub attribute: u32,
    pub option: HandleOption,
}

impl Event {
    pub fn new(id: EventId, code: u8, attribute: u32) -> Self {
        Self { id, code, attribute, option: HandleOption::Default }
    }

    pub fn is_direct(&self) -> bool {
        self.attribute & attribute::DIRECT != 0
    }
    pub fn is_upper_cases(&self) -> bool {
        self.attribute & attribute::UPPER_CASES != 0
    }
    pub fn is_toggle_kana(&self) -> bool {
        self.attribute & attribute::TOGGLE_KANA != 0
    }
    pub fn is_toggle_jisx0201_kana(&self) -> bool {
        self.attribute & attribute::TOGGLE_JISX0201_KANA != 0
    }
    pub fn is_switch_to_ascii(&self) -> bool {
        self.attribute & attribute::SWITCH_TO_ASCII != 0
    }
    pub fn is_switch_to_jisx0208_latin(&self) -> bool {
        self.attribute & attribute::SWITCH_TO_JISX0208_LATIN != 0
    }
    pub fn is_enter_japanese(&self) -> bool {
        self.attribute & attribute::ENTER_JAPANESE != 0
    }
    pub fn is_enter_abbrev(&self) -> bool {
        self.attribute & attribute::ENTER_ABBREV != 0
    }
    pub fn is_next_completion(&self) -> bool {
        self.attribute & attribute::NEXT_COMPLETION != 0
    }
    pub fn is_prev_completion(&self) -> bool {
        self.attribute & attribute::PREV_COMPLETION != 0
    }
    pub fn is_next_candidate(&self) -> bool {
        self.attribute & attribute::NEXT_CANDIDATE != 0
    }
    pub fn is_prev_candidate(&self) -> bool {
        self.attribute & attribute::PREV_CANDIDATE != 0
    }
    pub fn is_remove_trigger(&self) -> bool {
        self.attribute & attribute::REMOVE_TRIGGER != 0
    }
    pub fn is_input_chars(&self) -> bool {
        self.attribute & attribute::INPUT_CHARS != 0
    }
    pub fn is_comp_conversion(&self) -> bool {
        self.attribute & attribute::COMP_CONVERSION != 0
    }
    pub fn is_sticky_key(&self) -> bool {
        self.attribute & attribute::STICKY_KEY != 0
    }
}

/// Modifier bits (port of `SKKKeyState::Modifier`).
pub mod modifier {
    pub const SHIFT: u32 = 1 << 1;
    pub const CTRL: u32 = 1 << 2;
    pub const ALT: u32 = 1 << 3;
    pub const META: u32 = 1 << 4;
}

/// Packed key state:
/// `| unused | modifier | key code | ascii |` (8 bits each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyState(u32);

impl KeyState {
    pub fn key_code(code: u32, mods: u32) -> Self {
        Self((mods << 16) | ((code & 0xff) << 8))
    }

    pub fn char_code(code: u32, mods: u32) -> Self {
        Self((mods << 16) | (code & 0xff))
    }
}
