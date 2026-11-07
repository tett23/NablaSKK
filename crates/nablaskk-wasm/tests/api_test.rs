// NablaSKK: tests of the exported wasm functions, run natively (ADR 0023).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

mod common;

use common::*;

#[test]
fn kanji_converts_and_commits() {
    let mut demo = Demo::standard();
    demo.type_str("Kanji");
    assert_eq!(field(&demo.state(), "composing"), "▽かんじ");
    demo.space();
    assert_eq!(field(&demo.state(), "composing"), "▼漢字");
    demo.enter();
    let state = demo.state();
    assert_eq!(field(&state, "fixed"), "漢字");
    assert_eq!(field(&state, "composing"), "");
}

#[test]
fn okuri_conversion() {
    let mut demo = Demo::standard();
    demo.type_str("KaKu");
    assert_eq!(field(&demo.state(), "composing"), "▼書く");
}

#[test]
fn user_rule_types_comma() {
    let mut demo = Demo::standard();
    assert_eq!(
        demo.set_user_rules("fj,、,、,､"),
        r#"[{"line":1,"input":"fj","applied":true,"reason":null}]"#
    );
    demo.type_str("fj");
    assert_eq!(field(&demo.state(), "fixed"), "、");
}

#[test]
fn conflicting_rule_is_not_applied() {
    let mut demo = Demo::standard();
    assert_eq!(
        demo.set_user_rules("ka.,…,…,…"),
        r#"[{"line":1,"input":"ka.","applied":false,"reason":"「ka → か」がすぐに確定しなくなるため使えません"}]"#
    );
    demo.type_str("ka");
    assert_eq!(field(&demo.state(), "fixed"), "か");
}

#[test]
fn setting_rules_again_replaces_them() {
    let mut demo = Demo::standard();
    demo.set_user_rules("fj,、,、,､");
    assert_eq!(demo.set_user_rules("fk,。,。,｡"), r#"[{"line":1,"input":"fk","applied":true,"reason":null}]"#);
    demo.type_str("fj");
    assert_ne!(field(&demo.state(), "fixed"), "、");
    demo.escape();
    demo.state();
    demo.type_str("fk");
    assert_eq!(field(&demo.state(), "fixed"), "。");
    assert_eq!(demo.set_user_rules(""), "[]");
}

#[test]
fn registered_word_is_found_in_the_same_session() {
    let mut demo = Demo::standard();
    // ほげ is not in SKK-JISYO.S: Space starts registration
    demo.type_str("Hoge");
    demo.space();
    assert!(field(&demo.state(), "composing").contains("[登録：ほげ]"));
    demo.type_str("Kanji");
    demo.space();
    demo.enter(); // 漢字 into the registration word
    demo.enter(); // finish registration
    assert_eq!(field(&demo.state(), "fixed"), "漢字");

    demo.type_str("Hoge");
    demo.space();
    assert_eq!(field(&demo.state(), "composing"), "▼漢字");
}

#[test]
fn state_reports_mode_window_and_annotation() {
    let mut demo = Demo::standard();
    assert!(demo.state().contains(r#""mode":"hirakana""#));
    demo.handle(b'q', 0xff, 0);
    assert!(demo.state().contains(r#""mode":"katakana""#));
    demo.handle(b'q', 0xff, CTRL);
    assert!(demo.state().contains(r#""mode":"jisx0201kana""#));
    demo.handle(b'j', 0xff, CTRL);
    demo.handle(b'L', 0xff, SHIFT);
    assert!(demo.state().contains(r#""mode":"jisx0208latin""#));
    demo.handle(b'j', 0xff, CTRL);
    demo.handle(b'l', 0xff, 0);
    assert!(demo.state().contains(r#""mode":"ascii""#));
    demo.handle(b'j', 0xff, CTRL);

    // Four candidates are shown inline, the fifth Space opens the window
    demo.type_str("Kai");
    for _ in 0..4 {
        demo.space();
    }
    assert!(demo.state().contains(r#""candidates":{"visible":false,"items":[]"#));
    demo.space();
    let state = demo.state();
    assert!(state.contains(r#""candidates":{"visible":true,"items":["改","#), "{state}");
    demo.handle(b'g', 0xff, CTRL);
    demo.handle(b'g', 0xff, CTRL);
    demo.state();

    assert!(demo.state().contains(r#""annotation":null"#));
}

#[test]
fn state_reports_the_annotation() {
    // SKK-JISYO.S has no annotations; use a dictionary that does
    let mut demo = Demo::new("", "", ";; okuri-nasi entries.\nきごう /記号;symbol/\n".as_bytes());
    demo.type_str("Kigou");
    demo.space();
    let state = demo.state();
    assert!(state.contains(r#""annotation":"symbol""#), "{state}");
}

#[test]
fn fixed_text_is_taken_once() {
    let mut demo = Demo::standard();
    demo.type_str("a");
    assert_eq!(field(&demo.state(), "fixed"), "あ");
    assert_eq!(field(&demo.state(), "fixed"), "");
}

#[test]
fn json_escapes_quotes_and_backslashes() {
    let mut demo = Demo::standard();
    demo.set_user_rules("fj,\",\",\"\nfk,\\,\\,\\\n");
    demo.type_str("fjfk");
    assert!(demo.state().contains(r#""fixed":"\"\\""#));
}

#[test]
fn empty_keymap_and_rules_use_the_built_in_ones() {
    let mut demo = Demo::new("", "", b"");
    demo.type_str("Kanji");
    demo.space();
    // No dictionary: the reading goes to registration
    assert!(field(&demo.state(), "composing").contains("[登録：かんじ]"));
}

#[test]
fn given_keymap_and_rules_are_used() {
    let mut demo = Demo::new("SKK_JMODE\tctrl::j\nInputChars\tgroup::hex::0x20-0x7e\n", "a,X,X,X\n", b"");
    demo.type_str("a");
    assert_eq!(field(&demo.state(), "fixed"), "X");
    // The built-in rules for conflict checks are the given ones
    assert_eq!(
        demo.set_user_rules("a,あ,ア,ｱ"),
        r#"[{"line":1,"input":"a","applied":false,"reason":"組み込みのルール「a → X」と入力キーが衝突します"}]"#
    );
}

#[test]
fn dictionary_in_utf8_converts_the_same() {
    let text = nablaskk_core::jconv::utf8_from_eucj(DICTIONARY).replace("coding: euc-jp", "coding: utf-8");
    let mut demo = Demo::new("", "", text.as_bytes());
    demo.type_str("Kanji");
    demo.space();
    assert_eq!(field(&demo.state(), "composing"), "▼漢字");
}
