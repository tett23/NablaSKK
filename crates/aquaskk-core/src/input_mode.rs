//! SKK input modes (port of `SKKInputMode.h`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputMode {
    #[default]
    Hirakana,
    Katakana,
    Jisx0201Kana,
    Ascii,
    Jisx0208Latin,
}
