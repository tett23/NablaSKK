// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/session/SKKInputMode.h
// Copyright (C) 2008 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

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
