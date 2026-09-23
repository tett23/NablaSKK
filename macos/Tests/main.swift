// Key translation tests: builds real NSEvents the way different clients
// deliver them and checks the engine sees the binding keymap.conf expects.
// Run with: sh macos/run-tests.sh

import Cocoa

var failures = 0

func check(_ condition: Bool, _ label: String) {
    if condition {
        print("ok   \(label)")
    } else {
        print("FAIL \(label)")
        failures += 1
    }
}

func keyEvent(characters: String, ignoringModifiers: String, keyCode: UInt16,
              flags: NSEvent.ModifierFlags = []) -> NSEvent {
    NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0,
                     windowNumber: 0, context: nil, characters: characters,
                     charactersIgnoringModifiers: ignoringModifiers, isARepeat: false,
                     keyCode: keyCode)!
}

let j = UInt8(ascii: "j")

// Ctrl-J as AppKit normally reports it
var key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38, flags: .control))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: letter + control flag")

// Ctrl-J delivered with the control character in both fields
key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "\n", keyCode: 38, flags: .control))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: control character in both fields")

// Ctrl-J delivered without the Control flag
key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: control flag missing")

// Dedicated keys keep their own codes
key = KeyTranslator.translate(keyEvent(characters: "\r", ignoringModifiers: "\r", keyCode: 36))
check(key.charcode == 0x0d && key.mods.isEmpty, "return stays return")
key = KeyTranslator.translate(keyEvent(characters: "\t", ignoringModifiers: "\t", keyCode: 48))
check(key.charcode == 0x09 && key.mods.isEmpty, "tab stays tab")
key = KeyTranslator.translate(keyEvent(characters: "\u{7f}", ignoringModifiers: "\u{7f}", keyCode: 51))
check(key.charcode == 0x08, "delete key is backspace")

// Shifted printable keys use the shifted character
key = KeyTranslator.translate(
    keyEvent(characters: "K", ignoringModifiers: "K", keyCode: 40, flags: .shift))
check(key.charcode == UInt8(ascii: "K") && key.mods == [.shift], "shift-k")

// End to end: every Ctrl-J shape switches ASCII -> hiragana and is consumed
for (label, event) in [
    ("normal", keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38, flags: .control)),
    ("control char", keyEvent(characters: "\n", ignoringModifiers: "\n", keyCode: 38, flags: .control)),
    ("no flag", keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38)),
] {
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-key-test")
    session.handle(charcode: UInt8(ascii: "l"), keycode: 37)
    check(session.inputMode == .ascii, "\(label): in ascii mode")

    let translated = KeyTranslator.translate(event)
    let handled = session.handle(
        charcode: translated.charcode, keycode: translated.keycode, mods: translated.mods)
    check(handled && session.inputMode == .hirakana, "\(label): ctrl-j consumed, mode is hiragana")
}

// Client quirks: which consumed keys need masking
check(ClientQuirks.keyLeak(for: "com.mitchellh.ghostty") == .allKeys, "ghostty re-encodes every key")
check(ClientQuirks.keyLeak(for: "com.apple.TextEdit") == .none, "TextEdit honors the IME result")
check(ClientQuirks.keyLeak(for: nil) == .none, "unknown client")

// Ghostty: `l` (hiragana -> ASCII) produces nothing and must be masked
check(ClientQuirks.shouldMask(.allKeys, handled: true, produced: false, wasComposing: false, hasControl: false),
      "ghostty: silent plain key is masked")
check(!ClientQuirks.shouldMask(.allKeys, handled: true, produced: true, wasComposing: false, hasControl: false),
      "ghostty: keys that produce text are not masked")
check(!ClientQuirks.shouldMask(.allKeys, handled: true, produced: false, wasComposing: true, hasControl: false),
      "ghostty: marked text before the key already hides it")
check(!ClientQuirks.shouldMask(.allKeys, handled: false, produced: false, wasComposing: false, hasControl: false),
      "ghostty: pass-through keys are not masked")
// Chromium: only Control combinations leak
check(ClientQuirks.shouldMask(.controlKeys, handled: true, produced: false, wasComposing: false, hasControl: true),
      "chromium: silent ctrl key is masked")
check(!ClientQuirks.shouldMask(.controlKeys, handled: true, produced: false, wasComposing: false, hasControl: false),
      "chromium: silent plain key is left alone")
check(!ClientQuirks.shouldMask(.none, handled: true, produced: false, wasComposing: false, hasControl: true),
      "native clients are never masked")

// The engine side of the Ghostty case: `l` is consumed and produces nothing
do {
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-key-test")
    let handled = session.handle(charcode: UInt8(ascii: "l"), keycode: 37)
    check(handled && session.inputMode == .ascii && session.composing.isEmpty && session.takeFixed().isEmpty,
          "l: consumed, ascii mode, no text and no marked text")
}

// dictionaries.conf: parse / serialize round trip, disabled entries, legacy lines
do {
    let text = """
    # comment
    0 /usr/share/skk/SKK-JISYO.L
    - 5 /Users/me/My Dictionary.utf8
    4
    2 localhost:1178
    garbage line
    """
    let entries = DictionaryConfig.parse(text)
    check(entries.count == 4, "config: parses four entries, skipping comments and garbage")
    check(entries[0].enabled && entries[0].kind == .common && entries[0].location == "/usr/share/skk/SKK-JISYO.L",
          "config: enabled common dictionary")
    check(!entries[1].enabled && entries[1].kind == .commonUTF8 && entries[1].location == "/Users/me/My Dictionary.utf8",
          "config: disabled entry keeps its type and a location with spaces")
    check(entries[2].kind == .gadget && entries[2].location.isEmpty, "config: gadget without location")
    check(entries[3].kind == .proxy && entries[3].location == "localhost:1178", "config: skkserv")
    check(DictionaryConfig.parse("6 localhost:1178")[0].kind == .proxyUTF8, "config: type 6 is skkserv (UTF-8)")
    check(!DictionaryEntry.Kind.addable.contains(.proxy) && !DictionaryEntry.Kind.addable.contains(.proxyUTF8)
          && DictionaryEntry.Kind.addable.contains(.common),
          "config: skkserv kinds are not offered by the + menu")

    let reparsed = DictionaryConfig.parse(DictionaryConfig.serialize(entries))
    check(reparsed.map { ($0.enabled, $0.kind, $0.location) }.elementsEqual(
            entries.map { ($0.enabled, $0.kind, $0.location) }, by: { $0 == $1 }),
          "config: serialize/parse round trip")
    check(DictionaryConfig.serialize(entries).contains("\n- 5 /Users/me/My Dictionary.utf8\n"),
          "config: disabled entries are written with a leading dash")
    check(entries.map(\.name) == ["SKK-JISYO.L", "My Dictionary.utf8", "gadget", "localhost:1178"],
          "config: list names come from the file name or the location")
    check(DictionaryEntry(kind: .autoUpdate, location: "openlab.jp /skk/dict/SKK-JISYO.L /tmp/SKK-JISYO.L").name == "SKK-JISYO.L",
          "config: auto-update name uses the save path")
    check(DictionaryEntry(kind: .common, location: "").name == "(未設定)", "config: empty location shows a placeholder")
}

// Importing dictionaries: copy into the support directory, overwrite an
// earlier copy with the same name, leave the source untouched
do {
    let scratch = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("nablaskk-import-test")
    try? FileManager.default.removeItem(at: scratch)
    try! FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
    let a = scratch.appendingPathComponent("SKK-JISYO.test")
    let b = scratch.appendingPathComponent("other/SKK-JISYO.test")
    try! FileManager.default.createDirectory(at: b.deletingLastPathComponent(), withIntermediateDirectories: true)
    try! "a".write(to: a, atomically: true, encoding: .utf8)
    try! "b".write(to: b, atomically: true, encoding: .utf8)

    let imported = try! DictionaryConfig.importDictionary(at: a)
    check(imported.hasPrefix(DictionaryConfig.dictionariesDirectory.path)
          && FileManager.default.contentsEqual(atPath: a.path, andPath: imported),
          "import: copied into the dictionaries directory")
    check(try! DictionaryConfig.importDictionary(at: a) == imported, "import: identical file reuses the copy")
    let second = try! DictionaryConfig.importDictionary(at: b)
    check(second == imported && (try? String(contentsOfFile: imported, encoding: .utf8)) == "b",
          "import: a different file with the same name overwrites the copy")
    check((try? String(contentsOf: b, encoding: .utf8)) == "b", "import: the source file is left in place")
    check(!FileManager.default.fileExists(atPath: DictionaryConfig.dictionariesDirectory
              .appendingPathComponent(".SKK-JISYO.test.importing").path),
          "import: no staging file is left behind")
    check(try! DictionaryConfig.importDictionary(at: URL(fileURLWithPath: imported)) == imported,
          "import: a file already in the support directory is used in place")

    try? FileManager.default.removeItem(atPath: imported)
    try? FileManager.default.removeItem(at: scratch)
}

// User dictionary model: sections, annotations, okuri hints, escaping, round trip
do {
    let text = """
    ;; okuri-ari entries.
    おくr /送/贈/[り/送/]/
    ;; okuri-nasi entries.
    かんじ /漢字;kanji/幹事/
    すらっしゅ /a[2f]b/
    """
    let dictionary = UserDictionary.parse(text)
    check(dictionary.okuriAri.count == 1 && dictionary.okuriNasi.count == 2, "user dict: sections")

    let okuri = dictionary.okuriAri[0]
    check(okuri.reading == "おくr" && okuri.okuriAri && okuri.candidates.map(\.word) == ["送", "贈"],
          "user dict: okuri-ari candidates")
    check(okuri.hints == [UserOkuriHint(okuri: "り", words: ["送"])], "user dict: okuri hint parsed")

    let kanji = dictionary.okuriNasi[0]
    check(kanji.candidates[0].word == "漢字" && kanji.candidates[0].annotation == "kanji", "user dict: annotation split")
    check(dictionary.okuriNasi[1].candidates[0].word == "a/b", "user dict: escaped slash decoded")

    let serialized = dictionary.serialize()
    check(serialized.contains("おくr /送/贈/[り/送/]/\n"), "user dict: okuri line round trip")
    check(serialized.contains("かんじ /漢字;kanji/幹事/\n"), "user dict: annotation round trip")
    check(serialized.contains("すらっしゅ /a[2f]b/\n"), "user dict: slash re-encoded on save")
    check(UserDictionary.parse(serialized).serialize() == serialized, "user dict: serialize/parse round trip")

    check(UserDictionaryEntry.isOkuriAri(reading: "おくr") && !UserDictionaryEntry.isOkuriAri(reading: "かんじ")
          && !UserDictionaryEntry.isOkuriAri(reading: "abbrev"), "user dict: okuri-ari detection by trailing romaji")

    // EUC-JP files are read and written back in EUC-JP
    let url = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("nablaskk-userdict-euc")
    try! text.data(using: .japaneseEUC)!.write(to: url)
    var euc = try! UserDictionary.load(from: url)
    check(euc.isEUC && euc.okuriNasi[0].reading == "かんじ", "user dict: EUC-JP file loads")
    euc.okuriNasi[0].candidates.append(UserCandidate(word: "感じ"))
    try! euc.save(to: url)
    let data = try! Data(contentsOf: url)
    check(String(data: data, encoding: .utf8) == nil && String(data: data, encoding: .japaneseEUC)!.contains("感じ"),
          "user dict: EUC-JP file stays EUC-JP after save")
    try? FileManager.default.removeItem(at: url)
}

// Input settings: independent punctuation options and the rule patch they produce
do {
    check(InputSettings.parse("").kanaRulePatch.isEmpty, "settings: defaults produce no patch")
    var s = InputSettings.parse("comma=fullwidth\nperiod=japanese\n")
    check(s.fullWidthComma && !s.fullWidthPeriod, "settings: comma alone")
    check(s.kanaRulePatch == InputSettings.commaRule, "settings: comma-only patch")
    s = InputSettings.parse("# c\nperiod = fullwidth\n")
    check(!s.fullWidthComma && s.fullWidthPeriod && s.kanaRulePatch == InputSettings.periodRule,
          "settings: period alone, whitespace tolerated")
    let both = InputSettings(fullWidthComma: true, fullWidthPeriod: true)
    check(InputSettings.parse(both.serialize()) == both, "settings: serialize/parse round trip")
    check(both.kanaRulePatch == InputSettings.commaRule + InputSettings.periodRule, "settings: both patches")

    // suggest (dynamic completion) block
    let sg = InputSettings()
    check(!sg.suggestEnabled && sg.suggestCount == 5 && sg.completionExtended, "settings: suggest defaults (off, 5, all dictionaries)")
    let sgParsed = InputSettings.parse("suggest=on\nsuggest_count=3\ncompletion_extended=off\n")
    check(sgParsed.suggestEnabled && sgParsed.suggestCount == 3 && !sgParsed.completionExtended, "settings: suggest parsed")
    check(InputSettings.parse(sgParsed.serialize()) == sgParsed, "settings: suggest round trip")
    check(InputSettings.parse("suggest_count=0\n").suggestCount == 1 && InputSettings.parse("suggest_count=99\n").suggestCount == 20,
          "settings: suggest count clamped")

    // End to end: typing ▽かん lists readings from the dictionary, the
    // typed part being the common prefix; off by default
    let suggestDict = NSTemporaryDirectory() + "nablaskk-suggest-dict"
    try! ";; okuri-ari entries.\n;; okuri-nasi entries.\nかんじ /漢字/\nかんとう /関東/\nかんさい /関西/\nきた /北/\n"
        .write(toFile: suggestDict, atomically: true, encoding: .utf8)
    let suggestSession = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-suggest-user")
    suggestSession.addDictionary(.commonUTF8, location: suggestDict)
    for c in "Kan".utf8 { suggestSession.handle(charcode: c) }
    check(!suggestSession.completionVisible, "suggest: hidden while the option is off")
    suggestSession.clear()
    suggestSession.setOption(.enableDynamicCompletion, 1)
    suggestSession.setOption(.dynamicCompletionRange, 2)
    for c in "Kan".utf8 { suggestSession.handle(charcode: c) }
    check(!suggestSession.completionVisible, "suggest: user dictionary only (engine default) finds nothing")
    suggestSession.clear()
    suggestSession.setOption(.enableExtendedCompletion, 1)
    for c in "Kan".utf8 { suggestSession.handle(charcode: c) }
    check(suggestSession.completionVisible && suggestSession.completions.count == 2
          && Set(suggestSession.completions).isSubset(of: ["かんじ", "かんとう", "かんさい"])
          && suggestSession.completionPrefixLength == 2,
          "suggest: two readings starting with かん, prefix length 2")
    suggestSession.setOption(.dynamicCompletionRange, 10)
    for c in "ni".utf8 { suggestSession.handle(charcode: c) }
    check(!suggestSession.completionVisible, "suggest: nothing matches かんに")
    suggestSession.handle(charcode: 0x08)
    check(suggestSession.completions.count == 3, "suggest: range 10 lists all three after backspace")
    suggestSession.handle(charcode: 0x09)  // TAB completes to the first
    check(suggestSession.composing.hasPrefix("▽かん") && suggestSession.composing.count > 3, "suggest: TAB completes the reading")
    suggestSession.clear()
    check(!suggestSession.completionVisible, "suggest: hidden after clear")
    _ = suggestSession.takeFixed()
    try? FileManager.default.removeItem(atPath: suggestDict)

    // skkserv block: defaults, parse, round trip, validation
    let defaults = InputSettings()
    check(!defaults.skkservEnabled && defaults.skkservHost == "localhost" && defaults.skkservPort == 1178
          && defaults.skkservEncoding == .eucJP && !defaults.skkservIsConfigured,
          "settings: skkserv defaults off, localhost:1178, EUC-JP")
    let server = InputSettings.parse("skkserv=on\nskkserv_host=skk.example.org\nskkserv_port=1179\nskkserv_encoding=UTF-8\n")
    check(server.skkservIsConfigured && server.skkservLocation == "skk.example.org:1179"
          && server.skkservEncoding == .utf8 && server.skkservEncoding.dictionaryKind == .proxyUTF8,
          "settings: skkserv host/port/encoding parsed (UTF-8 → type 6)")
    check(InputSettings.parse(server.serialize()) == server, "settings: skkserv serialize/parse round trip")
    check(InputSettings.parse("skkserv=on\nskkserv_host=  \n").skkservIsConfigured == false,
          "settings: blank host is not configured")
    check(InputSettings.parse("skkserv=on\nskkserv_port=70000\n").skkservIsConfigured == false,
          "settings: out-of-range port is not configured")
    check(InputSettings.parse("skkserv_encoding=garbage\n").skkservEncoding == .eucJP,
          "settings: unknown encoding falls back to EUC-JP")

    // End to end through the engine: patch, then reset
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-key-test")
    session.patchKanaRules(both.kanaRulePatch)
    for c in ",.".utf8 { session.handle(charcode: c) }
    check(session.takeFixed() == "，．", "settings: engine types full-width punctuation after patch")
    session.resetKanaRules()
    for c in ",.".utf8 { session.handle(charcode: c) }
    check(session.takeFixed() == "、。", "settings: reset restores default punctuation")
}

// Key bindings: defaults match data/keymap.conf, syntax check, round trip, engine override
do {
    let builtin = try! String(contentsOfFile: "data/keymap.conf", encoding: .utf8)
    var defaults: [String: String] = [:]
    for line in builtin.split(separator: "\n") {
        let fields = line.split(whereSeparator: { $0 == " " || $0 == "\t" })
        if fields.count == 2, !fields[0].hasPrefix("#") { defaults[String(fields[0])] = String(fields[1]) }
    }
    for action in KeymapSettings.actions {
        check(defaults[action.symbol] == action.defaultKeys, "keymap: default for \(action.symbol) matches data/keymap.conf")
    }

    for good in ["ctrl::j", "group::hex::0x03,0x0a,0x0d||ctrl::m", "keycode::7b", "alt::hex::0x20||shift::hex::0x20",
                 "group::A-K,M-P,R-Z", "/", "ctrl::/"] {
        check(KeymapSettings.isValid(good), "keymap: valid spec \(good)")
    }
    for bad in ["", "ctrl::", "ctrl j", "hex::0xzz", "ab", "ctrl::||q", "hex::0x123"] {
        check(!KeymapSettings.isValid(bad), "keymap: invalid spec \(bad)")
    }

    let jmode = KeymapSettings.actions[0]
    var settings = KeymapSettings()
    settings.set("ctrl::k", for: jmode)
    settings.set(" ctrl::j ", for: KeymapSettings.actions[0])
    check(settings.overrides.isEmpty, "keymap: setting the default clears the override")
    settings.set("ctrl::k", for: jmode)
    settings.set("bad spec", for: KeymapSettings.actions[1])
    check(settings.overrideText == "SKK_JMODE\tctrl::k", "keymap: invalid overrides are not sent to the engine")
    check(KeymapSettings.parse(settings.serialize()) == settings, "keymap: serialize/parse round trip")
    check(KeymapSettings.parse("Unknown q\n# SKK_JMODE ctrl::x\nSKK_JMODE ctrl::k\n").overrides == ["SKK_JMODE": "ctrl::k"],
          "keymap: unknown symbols and comments are ignored")

    // End to end: Ctrl-K enters kana mode after the override, Ctrl-J no longer does
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-keymap-test")
    session.overrideKeymap(settings.overrideText)
    session.handle(charcode: UInt8(ascii: "l"))
    check(session.inputMode == .ascii, "keymap: ascii mode before override test")
    session.handle(charcode: UInt8(ascii: "j"), mods: [.ctrl])
    check(session.inputMode == .ascii, "keymap: Ctrl-J no longer switches to kana")
    session.handle(charcode: UInt8(ascii: "k"), mods: [.ctrl])
    check(session.inputMode == .hirakana, "keymap: Ctrl-K switches to kana after override")
    session.resetKeymap()
    session.handle(charcode: UInt8(ascii: "l"))
    session.handle(charcode: UInt8(ascii: "j"), mods: [.ctrl])
    check(session.inputMode == .hirakana, "keymap: reset restores Ctrl-J")
    _ = session.takeFixed()
}

if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("PASS all key translation tests")
// Key bindings: defaults match data/keymap.conf, syntax check, round trip, engine override
do {
    let builtin = try! String(contentsOfFile: "data/keymap.conf", encoding: .utf8)
    var defaults: [String: String] = [:]
    for line in builtin.split(separator: "\n") {
        let fields = line.split(whereSeparator: { $0 == " " || $0 == "\t" })
        if fields.count == 2, !fields[0].hasPrefix("#") { defaults[String(fields[0])] = String(fields[1]) }
    }
    for action in KeymapSettings.actions {
        check(defaults[action.symbol] == action.defaultKeys, "keymap: default for \(action.symbol) matches data/keymap.conf")
    }

    for good in ["ctrl::j", "group::hex::0x03,0x0a,0x0d||ctrl::m", "keycode::7b", "alt::hex::0x20||shift::hex::0x20",
                 "group::A-K,M-P,R-Z", "/", "ctrl::/"] {
        check(KeymapSettings.isValid(good), "keymap: valid spec \(good)")
    }
    for bad in ["", "ctrl::", "ctrl j", "hex::0xzz", "ab", "ctrl::||q", "hex::0x123"] {
        check(!KeymapSettings.isValid(bad), "keymap: invalid spec \(bad)")
    }

    let jmode = KeymapSettings.actions[0]
    var settings = KeymapSettings()
    settings.set("ctrl::k", for: jmode)
    settings.set(" ctrl::j ", for: KeymapSettings.actions[0])
    check(settings.overrides.isEmpty, "keymap: setting the default clears the override")
    settings.set("ctrl::k", for: jmode)
    settings.set("bad spec", for: KeymapSettings.actions[1])
    check(settings.overrideText == "SKK_JMODE\tctrl::k", "keymap: invalid overrides are not sent to the engine")
    check(KeymapSettings.parse(settings.serialize()) == settings, "keymap: serialize/parse round trip")
    check(KeymapSettings.parse("Unknown q\n# SKK_JMODE ctrl::x\nSKK_JMODE ctrl::k\n").overrides == ["SKK_JMODE": "ctrl::k"],
          "keymap: unknown symbols and comments are ignored")

    // End to end: Ctrl-K enters kana mode after the override, Ctrl-J no longer does
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-keymap-test")
    session.overrideKeymap(settings.overrideText)
    session.handle(charcode: UInt8(ascii: "l"))
    check(session.inputMode == .ascii, "keymap: ascii mode before override test")
    session.handle(charcode: UInt8(ascii: "j"), mods: [.ctrl])
    check(session.inputMode == .ascii, "keymap: Ctrl-J no longer switches to kana")
    session.handle(charcode: UInt8(ascii: "k"), mods: [.ctrl])
    check(session.inputMode == .hirakana, "keymap: Ctrl-K switches to kana after override")
    session.resetKeymap()
    session.handle(charcode: UInt8(ascii: "l"))
    session.handle(charcode: UInt8(ascii: "j"), mods: [.ctrl])
    check(session.inputMode == .hirakana, "keymap: reset restores Ctrl-J")
    _ = session.takeFixed()
}

