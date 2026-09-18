// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/backend/SKKDictionaryFactory.{h,cpp}
//   src/engine/backend/SKKDictionaryKey.h
// Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Dictionary construction by type (port of `SKKDictionaryFactory` /
//! `SKKDictionaryKey`). Types keep the original DictionarySet.plist
//! numbering; `Kotoeri` was macOS-specific and maps to a null dictionary.

use super::file::Encoding;
use super::{
    AutoUpdateDictionary, CommonDictionary, CompletionHelper, Dictionary, GadgetDictionary,
    ProxyDictionary,
};
use crate::candidate::CandidateSuite;
use crate::entry::Entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictionaryType {
    /// SKK 辞書 (EUC-JP)
    Common = 0,
    /// SKK 辞書 (自動ダウンロード)
    AutoUpdate = 1,
    /// skkserv 辞書
    Proxy = 2,
    /// ことえり辞書 (macOS 固有; 未対応)
    Kotoeri = 3,
    /// プログラム実行変換辞書
    Gadget = 4,
    /// SKK 辞書 (UTF-8)
    CommonUtf8 = 5,
}

impl DictionaryType {
    pub fn from_id(id: i32) -> Option<Self> {
        Some(match id {
            0 => Self::Common,
            1 => Self::AutoUpdate,
            2 => Self::Proxy,
            3 => Self::Kotoeri,
            4 => Self::Gadget,
            5 => Self::CommonUtf8,
            _ => return None,
        })
    }
}

/// A dictionary configuration entry (port of `SKKDictionaryKey`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictionaryKey {
    pub dictionary_type: DictionaryType,
    pub location: String,
}

impl DictionaryKey {
    pub fn new(dictionary_type: DictionaryType, location: &str) -> Self {
        Self { dictionary_type, location: location.to_string() }
    }
}

/// Dictionary that finds nothing (port of `SKKNullDictionary`).
#[derive(Debug, Clone, Copy, Default)]
pub struct NullDictionary;

impl Dictionary for NullDictionary {
    fn find(&self, _entry: &Entry, _result: &mut CandidateSuite) {}
    fn complete(&self, _helper: &mut CompletionHelper) {}
}

/// Build a dictionary from its configuration. Unsupported or failing
/// configurations yield a null dictionary, matching the original's
/// error handling.
pub fn create(key: &DictionaryKey) -> Box<dyn Dictionary + Send> {
    let location = key.location.as_str();

    match key.dictionary_type {
        DictionaryType::Common => match CommonDictionary::open(location, Encoding::EucJp) {
            Ok(dictionary) => Box::new(dictionary),
            Err(err) => {
                eprintln!("dictionary::create: can't load {location}: {err}");
                Box::new(NullDictionary)
            }
        },
        DictionaryType::CommonUtf8 => match CommonDictionary::open(location, Encoding::Utf8) {
            Ok(dictionary) => Box::new(dictionary),
            Err(err) => {
                eprintln!("dictionary::create: can't load {location}: {err}");
                Box::new(NullDictionary)
            }
        },
        DictionaryType::AutoUpdate => match AutoUpdateDictionary::open(location) {
            Ok(dictionary) => Box::new(dictionary),
            Err(err) => {
                eprintln!("dictionary::create: can't set up {location}: {err}");
                Box::new(NullDictionary)
            }
        },
        DictionaryType::Proxy => Box::new(ProxyDictionary::new(location)),
        DictionaryType::Gadget => Box::new(GadgetDictionary::new()),
        DictionaryType::Kotoeri => {
            eprintln!("dictionary::create: Kotoeri dictionaries are not supported");
            Box::new(NullDictionary)
        }
    }
}
