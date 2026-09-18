// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/dictionary/SKKDictionaryFile.{h,cpp}
// Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! SKK dictionary file I/O (port of `SKKDictionaryFile`).

use crate::jconv;
use std::io::Write;
use std::path::Path;

/// Reading and raw (unparsed) candidate line.
pub type DictionaryEntry = (String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    EucJp,
    #[default]
    Utf8,
    /// Detect EUC-JP vs UTF-8 per file (coding cookie, then content).
    Auto,
}

impl Encoding {
    /// Determine the encoding of dictionary bytes.
    ///
    /// A `coding:` cookie on the first line (the ddskk convention, e.g.
    /// ";; -*- mode: fundamental; coding: utf-8 -*-") wins; otherwise the
    /// content decides: valid UTF-8 is UTF-8, anything else is EUC-JP.
    /// Pure ASCII resolves to UTF-8, which decodes identically either way.
    pub fn detect(bytes: &[u8]) -> Encoding {
        let first_line = bytes.split(|&b| b == b'\n').next().unwrap_or(&[]);
        let first_line = String::from_utf8_lossy(first_line).to_ascii_lowercase();

        if let Some(pos) = first_line.find("coding:") {
            let value = first_line[pos + "coding:".len()..].trim_start();
            if value.starts_with("utf-8") || value.starts_with("utf8") {
                return Encoding::Utf8;
            }
            if value.starts_with("euc") {
                return Encoding::EucJp;
            }
        }

        if std::str::from_utf8(bytes).is_ok() {
            Encoding::Utf8
        } else {
            Encoding::EucJp
        }
    }

    /// Resolve `Auto` against actual bytes; concrete encodings unchanged.
    pub fn resolve(&self, bytes: &[u8]) -> Encoding {
        match self {
            Encoding::Auto => Encoding::detect(bytes),
            other => *other,
        }
    }

    fn decode(&self, bytes: &[u8]) -> String {
        match self.resolve(bytes) {
            Encoding::EucJp => jconv::utf8_from_eucj(bytes),
            _ => String::from_utf8_lossy(bytes).into_owned(),
        }
    }

    fn encode(&self, text: &str) -> Vec<u8> {
        match self {
            Encoding::EucJp => jconv::eucj_from_utf8(text),
            _ => text.as_bytes().to_vec(),
        }
    }
}

const OKURI_ARI_MARK: &str = ";; okuri-ari entries.";
const OKURI_NASI_MARK: &str = ";; okuri-nasi entries.";

/// In-memory SKK dictionary file. Contents are held as UTF-8 regardless of
/// the on-disk encoding.
#[derive(Debug, Clone, Default)]
pub struct DictionaryFile {
    okuri_ari: Vec<DictionaryEntry>,
    okuri_nasi: Vec<DictionaryEntry>,
}

impl DictionaryFile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, path: impl AsRef<Path>, encoding: Encoding) -> std::io::Result<()> {
        let bytes = std::fs::read(path)?;
        self.load_from_str(&encoding.decode(&bytes));
        Ok(())
    }

    pub fn load_from_str(&mut self, text: &str) {
        self.okuri_ari.clear();
        self.okuri_nasi.clear();

        #[derive(PartialEq)]
        enum Section {
            Header,
            OkuriAri,
            OkuriNasi,
        }

        let mut section = Section::Header;

        for line in text.lines() {
            if line.starts_with(';') {
                if line.contains("okuri-ari entries") {
                    section = Section::OkuriAri;
                } else if line.contains("okuri-nasi entries") {
                    section = Section::OkuriNasi;
                }
                continue;
            }

            let Some((key, value)) = line.split_once(' ') else { continue };
            if key.is_empty() || value.is_empty() {
                continue;
            }

            let entry = (key.to_string(), value.to_string());
            match section {
                Section::Header => {}
                Section::OkuriAri => self.okuri_ari.push(entry),
                Section::OkuriNasi => self.okuri_nasi.push(entry),
            }
        }
    }

    pub fn save(&self, path: impl AsRef<Path>, encoding: Encoding) -> std::io::Result<()> {
        let mut out = std::fs::File::create(path)?;

        let mut text = String::new();
        text.push_str(OKURI_ARI_MARK);
        text.push('\n');
        for (key, value) in &self.okuri_ari {
            text.push_str(key);
            text.push(' ');
            text.push_str(value);
            text.push('\n');
        }
        text.push_str(OKURI_NASI_MARK);
        text.push('\n');
        for (key, value) in &self.okuri_nasi {
            text.push_str(key);
            text.push(' ');
            text.push_str(value);
            text.push('\n');
        }

        out.write_all(&encoding.encode(&text))?;
        out.flush()
    }

    pub fn is_empty(&self) -> bool {
        self.okuri_ari.is_empty() && self.okuri_nasi.is_empty()
    }

    /// Sort both sections by reading, for binary search.
    pub fn sort(&mut self) {
        self.okuri_ari.sort_by(|a, b| a.0.cmp(&b.0));
        self.okuri_nasi.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub fn okuri_ari(&self) -> &[DictionaryEntry] {
        &self.okuri_ari
    }

    pub fn okuri_nasi(&self) -> &[DictionaryEntry] {
        &self.okuri_nasi
    }

    pub fn okuri_ari_mut(&mut self) -> &mut Vec<DictionaryEntry> {
        &mut self.okuri_ari
    }

    pub fn okuri_nasi_mut(&mut self) -> &mut Vec<DictionaryEntry> {
        &mut self.okuri_nasi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
;; -*- mode: fundamental; coding: utf-8 -*-
;; okuri-ari entries.
おくr /送/贈/[り/送/]/
;; okuri-nasi entries.
かんじ /漢字/幹事/
きかん /期間/機関/
";

    #[test]
    fn load_sections() {
        let mut file = DictionaryFile::new();
        file.load_from_str(SAMPLE);

        assert_eq!(file.okuri_ari().len(), 1);
        assert_eq!(file.okuri_ari()[0].0, "おくr");
        assert_eq!(file.okuri_nasi().len(), 2);
        assert_eq!(file.okuri_nasi()[0], ("かんじ".to_string(), "/漢字/幹事/".to_string()));
    }

    #[test]
    fn save_roundtrip() {
        let mut file = DictionaryFile::new();
        file.load_from_str(SAMPLE);

        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dict-file-roundtrip");

        file.save(&path, Encoding::Utf8).unwrap();

        let mut reloaded = DictionaryFile::new();
        reloaded.load(&path, Encoding::Utf8).unwrap();

        assert_eq!(reloaded.okuri_ari(), file.okuri_ari());
        assert_eq!(reloaded.okuri_nasi(), file.okuri_nasi());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn detect_by_coding_cookie() {
        let utf8 = b";; -*- mode: fundamental; coding: utf-8 -*-\n";
        assert_eq!(Encoding::detect(utf8), Encoding::Utf8);

        let eucj = b";; -*- mode: fundamental; coding: euc-jp -*-\n";
        assert_eq!(Encoding::detect(eucj), Encoding::EucJp);

        // The cookie wins even when the body would sniff differently
        let mut mixed = b";; coding: euc-jp\n".to_vec();
        mixed.extend_from_slice("かんじ /漢字/\n".as_bytes());
        assert_eq!(Encoding::detect(&mixed), Encoding::EucJp);
    }

    #[test]
    fn detect_by_content() {
        assert_eq!(Encoding::detect("かんじ /漢字/\n".as_bytes()), Encoding::Utf8);
        assert_eq!(
            Encoding::detect(&crate::jconv::eucj_from_utf8("かんじ /漢字/\n")),
            Encoding::EucJp
        );
        // Pure ASCII decodes identically either way
        assert_eq!(Encoding::detect(b";; empty dictionary\n"), Encoding::Utf8);
    }

    #[test]
    fn auto_loads_both_encodings() {
        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();

        for (name, encoding) in [("auto-euc", Encoding::EucJp), ("auto-utf8", Encoding::Utf8)] {
            let path = dir.join(name);

            let mut file = DictionaryFile::new();
            file.load_from_str(SAMPLE);
            file.save(&path, encoding).unwrap();

            let mut reloaded = DictionaryFile::new();
            reloaded.load(&path, Encoding::Auto).unwrap();
            assert_eq!(reloaded.okuri_nasi(), file.okuri_nasi(), "{name}");

            std::fs::remove_file(&path).ok();
        }
    }

    #[test]
    fn eucj_roundtrip() {
        let mut file = DictionaryFile::new();
        file.load_from_str(SAMPLE);

        let dir = std::env::temp_dir().join("nablaskk-core-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dict-file-eucj");

        file.save(&path, Encoding::EucJp).unwrap();

        let mut reloaded = DictionaryFile::new();
        reloaded.load(&path, Encoding::EucJp).unwrap();

        assert_eq!(reloaded.okuri_nasi(), file.okuri_nasi());

        std::fs::remove_file(&path).ok();
    }
}
