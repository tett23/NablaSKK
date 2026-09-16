//! Dictionary entry key (port of `SKKEntry`).

use crate::input_mode::InputMode;
use crate::jconv;

/// A dictionary lookup key: the reading, plus okurigana information for
/// okuri-ari entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    normal_entry: String,
    okuri_entry: String,
    prefix: String,
    kana: String,
    prompt: String,
}

impl Entry {
    pub fn new(entry: &str, okuri: &str) -> Self {
        let mut result = Self {
            normal_entry: entry.to_string(),
            kana: okuri.to_string(),
            ..Self::default()
        };
        result.update_entry();
        result
    }

    pub fn from_entry(entry: &str) -> Self {
        Self::new(entry, "")
    }

    pub fn set_entry(&mut self, entry: &str) {
        self.normal_entry = entry.to_string();

        // Remove the okuri prefix at the end of the reading ("かk" -> "か").
        if !self.prefix.is_empty() && self.normal_entry.ends_with(&self.prefix) {
            let cut = self.normal_entry.len() - self.prefix.len();
            self.normal_entry.truncate(cut);
        }

        self.update_entry();
    }

    pub fn append_entry(&mut self, str_: &str) {
        self.normal_entry += str_;
    }

    pub fn set_okuri(&mut self, prefix: &str, kana: &str) {
        self.prefix = prefix.to_string();
        self.kana = kana.to_string();

        self.update_entry();
    }

    /// The dictionary key: "かk" for okuri-ari, the reading otherwise.
    pub fn entry_string(&self) -> &str {
        if self.is_okuri_ari() {
            &self.okuri_entry
        } else {
            &self.normal_entry
        }
    }

    pub fn okuri_string(&self) -> &str {
        &self.kana
    }

    /// Display form: "か*かった" style for okuri-ari.
    pub fn prompt_string(&self) -> &str {
        &self.prompt
    }

    pub fn toggle_kana(&self, mode: InputMode) -> String {
        match mode {
            InputMode::Hirakana => jconv::hirakana_to_katakana(&self.normal_entry),
            InputMode::Katakana => jconv::katakana_to_hirakana(&self.normal_entry),
            InputMode::Jisx0201Kana => jconv::jisx0201_kana_to_katakana(&self.normal_entry),
            _ => String::new(),
        }
    }

    pub fn toggle_jisx0201_kana(&self, mode: InputMode) -> String {
        match mode {
            InputMode::Hirakana => jconv::hirakana_to_jisx0201_kana(&self.normal_entry),
            InputMode::Katakana => jconv::katakana_to_jisx0201_kana(&self.normal_entry),
            InputMode::Jisx0201Kana => jconv::jisx0201_kana_to_hirakana(&self.normal_entry),
            InputMode::Ascii => jconv::ascii_to_jisx0208_latin(&self.normal_entry),
            _ => String::new(),
        }
    }

    /// Normalize the reading to hiragana for dictionary lookup. In katakana
    /// or half-width kana modes the typed reading doesn't match the
    /// dictionary, so both the reading and the okuri prefix are re-derived.
    pub fn normalize(&self, mode: InputMode) -> Entry {
        let mut entry = self.clone();
        let (result, roman) = match mode {
            InputMode::Katakana => (
                jconv::katakana_to_hirakana(&self.normal_entry),
                jconv::katakana_to_roman(&self.kana),
            ),
            InputMode::Jisx0201Kana => (
                jconv::jisx0201_kana_to_hirakana(&self.normal_entry),
                jconv::jisx0201_kana_to_roman(&self.kana),
            ),
            _ => (self.normal_entry.clone(), jconv::hirakana_to_roman(&self.kana)),
        };

        entry.set_entry(&result);

        if !roman.is_empty() {
            let prefix: String = roman.chars().take(1).collect();
            entry.set_okuri(&prefix, &self.kana);
        }

        entry
    }

    pub fn is_empty(&self) -> bool {
        self.normal_entry.is_empty()
    }

    pub fn is_okuri_ari(&self) -> bool {
        !self.kana.is_empty()
    }

    fn update_entry(&mut self) {
        self.okuri_entry = self.normal_entry.clone();
        self.prompt = self.normal_entry.clone();
        self.okuri_entry += &self.prefix;

        if self.is_okuri_ari() {
            self.prompt.push('*');
            self.prompt += &self.kana;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okuri_nasi() {
        let entry = Entry::from_entry("かんじ");
        assert!(!entry.is_okuri_ari());
        assert_eq!(entry.entry_string(), "かんじ");
        assert_eq!(entry.prompt_string(), "かんじ");
    }

    #[test]
    fn okuri_ari() {
        let mut entry = Entry::from_entry("おく");
        entry.set_okuri("r", "り");
        assert!(entry.is_okuri_ari());
        assert_eq!(entry.entry_string(), "おくr");
        assert_eq!(entry.prompt_string(), "おく*り");
        assert_eq!(entry.okuri_string(), "り");
    }

    #[test]
    fn set_entry_strips_prefix() {
        let mut entry = Entry::from_entry("");
        entry.set_okuri("k", "き");
        entry.set_entry("かk");
        assert_eq!(entry.entry_string(), "かk");
        assert_eq!(entry.prompt_string(), "か*き");
    }

    #[test]
    fn toggle() {
        let entry = Entry::from_entry("かんじ");
        assert_eq!(entry.toggle_kana(InputMode::Hirakana), "カンジ");
        assert_eq!(entry.toggle_jisx0201_kana(InputMode::Hirakana), "ｶﾝｼﾞ");
    }

    #[test]
    fn normalize_katakana() {
        let entry = Entry::new("オク", "リ");
        let normalized = entry.normalize(InputMode::Katakana);
        assert_eq!(normalized.entry_string(), "おくr");
        assert_eq!(normalized.okuri_string(), "リ");
    }
}
