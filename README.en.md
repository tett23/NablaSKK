# NablaSKK

[日本語](README.md) | English

A Rust reimplementation of the input engine of [AquaSKK](https://github.com/codefirst/aquaskk)
(about 22,000 lines of C++), an SKK Japanese input method for macOS. It has no external
crate dependencies.

This file is a translation of [README.md](README.md). The specification
([docs/specifications.md](docs/specifications.md)) and the design decision records
(`docs/adr/`) are written in Japanese only, because this project is mainly for Japanese
speakers. Machine translation should work well enough for them.

## Highlight: add your own romaji-to-kana rules

SKK has built-in rules for symbols, such as `z.` for `…` and `z/` for `・`. NablaSKK lets you
extend this to any key combination you like. For example, if the punctuation keys are hard to
reach, you can make `fj` type `、`.

- On the "ローマ字" (Romaji) page of the preferences app, you can add and edit the keys to type
  and the text to produce in hiragana, katakana and half-width katakana. The built-in symbol
  rules are shown on the same page (read-only).
- A rule that duplicates or conflicts with a built-in rule or a rule added earlier (for example,
  extending `ka` to `ka.`, which would make `か` impossible to type) is shown in red with the
  reason, and is not saved.
- Your rules are applied after the built-in rules, the sub-rules such as AZIK and ACT, and the
  punctuation settings, so your own rules win.
- They are saved in `~/Library/Application Support/NablaSKK/kana-rule.conf`, in the same format
  as AquaSKK's kana-rule.conf (one rule per line: `input,hiragana,katakana,half-width katakana`).
  For example: `fj,、,、,､`.

## Installation

### Requirements

- [Rust](https://rustup.rs/) (stable; used only to build)
- For the macOS input method and the Swift integration: macOS 12 or later and the Xcode Command
  Line Tools (`xcode-select --install`)

### macOS input method (NablaSKK.app)

The easiest way is the install script (it fetches the latest release):

```sh
curl -fsSL https://raw.githubusercontent.com/tett23/NablaSKK/main/install.sh | sh
```

Files downloaded with curl get no quarantine attribute, so Gatekeeper shows no warning even
though the app is not signed with a Developer ID. To build from source:

```sh
git clone https://github.com/tett23/NablaSKK
cd NablaSKK
sh macos/build-app.sh
cp -r macos/dist/NablaSKK.app ~/Library/Input\ Methods/
```

1. Log out and log back in (if an older version is already running, `pkill NablaSKK` also
   works). If the input menu icon in the menu bar does not change after an update,
   `killall TextInputMenuAgent` redraws it.
2. In "System Settings → Keyboard → Input Sources → Edit → +", add **NablaSKK** from
   "Japanese". NablaSKK is a single input source; input modes such as hiragana, katakana and
   ASCII are switched inside NablaSKK (the same way as AquaSKK's "AquaSKK 統合" (unified)).
3. Set up dictionaries with **"NablaSKK の設定..."** (NablaSKK Settings) in the input source
   menu in the menu bar; it opens the preferences app. You can add dictionary files by
   dragging them into the window, turn them on and off with the checkboxes, and change the
   search order by dragging or with the arrow buttons. Changes are saved immediately and take
   effect the next time you start typing in any app.
   The bottom of the preferences app's sidebar shows the version and the commit it was
   built from (`NablaSKK 1.2.0 (8128940)`). Please include it when you report a problem.

   The settings are stored in `~/Library/Application Support/NablaSKK/dictionaries.conf`, which
   you may also edit by hand (it is created on first launch; an existing AquaSKK SKK-JISYO.L is
   used automatically). Example:

   ```
   0 /usr/local/share/skk/SKK-JISYO.L
   - 5 /Users/me/my-dictionary.utf8
   4
   ```

   Each line is "type location": 0 = SKK-JISYO (EUC-JP / UTF-8 detected automatically),
   1 = auto-download ("host url path"), 2 = skkserv (host:port, EUC-JP),
   4 = Gadget (today / now / =expression), 5 = SKK-JISYO (UTF-8 forced),
   6 = skkserv (host:port, UTF-8). A leading `-` disables the dictionary.

Suggestions (while you type a reading, the dictionary headwords starting with it are listed
under the cursor, and TAB completes) can be turned on and off on the "入力" (Input) page,
together with the number of suggestions and whether to use the system dictionaries too
(off by default).

When enabled on the "入力" (Input) page, dictionary annotations (such as "わるし（形）" in
`悪;わるし（形）`) are shown under the cursor while you choose a candidate (off by default).

On the "ローマ字" (Romaji) page you can choose AquaSKK's extended romaji input styles
(AZIK (JIS layout / US layout) and ACT). Symbol input (yakumono-ippai, such as `z1` → ○) and
old kana (such as `wi` → ゐ) can be combined with them (all off by default). You can also add
your own romaji-to-kana rules like the built-in `z.` → `…` (for example `fj` → `、`).

Key bindings can be changed per action on the "キー" (Keys) page of the preferences app
(keymap.conf syntax; only the differences are saved to `keymap.conf`; leave a field empty for
the default).

skkserv can be set up on the "skkserv" page of the preferences app: host, port and encoding
(EUC-JP / UTF-8), saved in `settings.conf`. The server is asked after all local dictionaries
have been searched.

Punctuation can be switched on the "入力" (Input) page: the comma (、 / ，) and the period
(。 / ．) independently (saved in `settings.conf`; takes effect the next time you start typing).

The user dictionary is saved in `~/Library/Application Support/NablaSKK/skk-jisyo` (UTF-8). On
the "ユーザー辞書" (User Dictionary) page of the preferences app you can search, add and delete
headwords, and add, reorder and annotate candidates. Edits are saved automatically, and the
input method reloads the file the next time you start typing in any app (if the input method
has unsaved learning of its own, that takes priority).

To uninstall, remove NablaSKK from the input sources, then:

```sh
rm -rf ~/Library/Input\ Methods/NablaSKK.app
rm -rf ~/Library/Application\ Support/NablaSKK   # also removes settings and the user dictionary
```

### About Gatekeeper warnings

The release artifacts are ad-hoc signed. Gatekeeper checks only files with the quarantine
attribute, so whether you see a warning depends on how you got the app:

- **Install script / curl / building from source**: no quarantine attribute, so no warning
  (recommended).
- **Downloading the zip in a browser**: you get a warning. After installing, remove the
  quarantine attribute once:

  ```sh
  xattr -dr com.apple.quarantine ~/Library/Input\ Methods/NablaSKK.app
  ```

If you have a Developer ID certificate, setting the repository secrets (`MACOS_CERT_P12` and
others; see the comments in `.github/workflows/ci.yml`) makes CI sign and notarize the app, so
it can be distributed without warnings. Locally, sign with
`SIGN_IDENTITY="Developer ID Application: ..." sh macos/build-app.sh`, then notarize with
`xcrun notarytool submit --wait` and `xcrun stapler staple`.

### Diagnostic log

For investigating problems such as keys not working in a particular app, there is an opt-in
diagnostic log:

```sh
touch ~/Library/Application\ Support/NablaSKK/debug-enabled   # enable
tail -f ~/Library/Application\ Support/NablaSKK/debug.log
rm ~/Library/Application\ Support/NablaSKK/debug-enabled      # disable
```

It records only the client's bundle id, the key code, the modifiers and how the key was
handled. The contents of ordinary character keys without modifiers are not recorded.

### skkserv / skk-cli (command-line tools)

```sh
cargo install --path crates/skkserv   # SKK dictionary server
cargo install --path crates/skk-cli   # terminal demo
```

They are installed in `~/.cargo/bin`. You can also run `cargo build --release` and use the
binaries in `target/release/` directly.

### Using it as a library

From Rust, depend on `nablaskk-core` by path. From Swift / Objective-C, link `nablaskk-ffi`
(staticlib / cdylib + `include/nablaskk.h`). See the sections below for details.

## Layout

```
crates/
  nablaskk-core/   the engine (library)
  nablaskk-ffi/    C ABI (for Swift / Objective-C hosts)
  skkserv/        SKK dictionary server (binary)
  skk-cli/        demo for trying SKK input in a terminal
data/             kana rules and keymaps (from AquaSKK)
swift/            Swift wrapper and smoke test
macos/            macOS input method and the preferences app (SwiftUI)
upstream-patches/ patches for AquaSKK's C++ code
```

### nablaskk-core

A port of AquaSKK's `src/engine`. Module mapping:

| Rust module | AquaSKK | Contents |
|---|---|---|
| `jconv` | `utility/jconv` | kana conversion and an EUC-JP (JIS X 0213) ⇔ UTF-8 codec (own implementation with generated tables) |
| `trie` | `trie/` | the kana-rule.conf tree and romaji-to-kana conversion |
| `candidate` | `entry/` | candidates, the candidate parser and okuri hints |
| `entry` | `entry/SKKEntry` | headwords and normalization |
| `numeric` | `backend/SKKNumericConverter` | numeric conversion (#0 to #9) |
| `dictionary` | `dictionary/` `backend/` | reading and writing SKK-JISYO; shared / user / Gadget / skkserv / auto-update dictionaries and the factory |
| `backend` | `backend/SKKBackEnd` | combined dictionary search (a plain struct, not a singleton) |
| `keymap` / `event` | `keymap/` | keymap.conf parsing and event resolution |
| `machine` | `state/GenericStateMachine.h` | hierarchical state machine (a StateId enum instead of method pointers) |
| `editor` | `editor/` | text buffers, the input queue and the editor stack |
| `selector` / `completer` | `selector/` `completer/` | candidate selection and headword completion |
| `session` | `state/` `session/` | SKK state transitions and the recursive word registration session |
| `bridge` / `config` | `bridge/` | traits for the host UI boundary, and settings |
| `calculator` | `utility/calculator.h` | simple calculation for =expression |

The macOS InputMethodKit layer is not a port of AquaSKK's Objective-C++; it is a new
implementation in Swift + FFI under `macos/`. The engine itself can be embedded in any host by
implementing traits such as `bridge::FrontEnd`.

### skkserv

A dictionary server speaking the skkserv protocol (port 1178). Communication is in EUC-JP.

```sh
cargo build --release
./target/release/skkserv -p 1178 /usr/share/skk/SKK-JISYO.L
```

The encoding of each dictionary (EUC-JP / UTF-8) is detected per file (the `coding:` cookie on
the first line first, otherwise from the contents). `-e` / `-u` force it.

Server completion (command `4`), which AquaSKK did not implement, is supported.

### skk-cli

A demo for trying SKK input in a terminal (raw mode, using termios directly).

```sh
cargo run --release -p skk-cli -- /usr/share/skk/SKK-JISYO.L
```

### macOS input method (NablaSKK.app)

A usable IME built on InputMethodKit (see [Installation](#installation) above). It translates
NSEvents into engine events and drives the Rust session through the FFI. The candidate list is
drawn inline in the marked text; choose with a/s/d/f/j/k/l.

### nablaskk-ffi + Swift

A C ABI for Swift / Objective-C. The header is `crates/nablaskk-ffi/include/nablaskk.h`, and the
Swift wrapper is `swift/SKKSession.swift`.

```sh
sh swift/run-smoke-test.sh   # builds Rust, links with swiftc and runs
```

A macOS InputMethodKit controller can embed it by bridging the return value of
`handle(charcode:keycode:mods:)` and `takeFixed()` / `composing` to `insertText` /
`setMarkedText`.

## CI / releases

GitHub Actions (`.github/workflows/ci.yml`) runs the tests (including the 400 acceptance cases),
clippy and the Swift smoke test on every push and pull request, and builds these artifacts:

- `NablaSKK-macos-universal.app.zip`: the input method (arm64 + x86_64)
- `nablaskk-tools-macos-universal.tar.gz` / `nablaskk-tools-linux-x86_64.tar.gz`: skkserv and
  skk-cli
- `nablaskk-ffi-macos-universal.tar.gz`: libnablaskk_ffi.a + nablaskk.h + SKKSession.swift (for
  embedding in Swift)

Pushing a `v*` tag creates a draft release with these attached. A tag containing `-` (such as
`v1.1.0-rc`) becomes a prerelease:

```sh
git tag v1.1.0 && git push origin v1.1.0
```

To build the universal app locally, run `UNIVERSAL=1 sh macos/build-app.sh`.

## Development

The specification is in [docs/specifications.md](docs/specifications.md) (Japanese). Design
decisions are written as ADRs in `docs/adr/` (Japanese) before implementation. In a committed
ADR, only the status line and links to ADR references may change, and committed audit records
in `docs/audit/` may not change (checked by [adr-guard](https://github.com/tett23/adr-guard)).
References such as "[ADR 0001](docs/adr/0001-adopt-global-working-rules.md)" in documents link
to the ADR (`adr-guard links --fix` adds them). Work happens on branches and is merged into main
with `--no-ff`.

After cloning, install adr-guard and enable the git hook. If you edit with Claude Code, add the
adr-guard hook to your global settings (`~/.claude/settings.json`), as in the example in the
[adr-guard README](https://github.com/tett23/adr-guard#claude-codeのhook) (it is not kept in this
repository):

```sh
cargo install --git https://github.com/tett23/adr-guard --tag v0.8.2
git config core.hooksPath .githooks
```

## Tests

```sh
cargo test
```

Besides ports of AquaSKK's unit tests (SKKTrie_TEST, SKKKeymap_TEST and others) and session
integration tests (conversion, conversion with okurigana, word registration, completion, mode
switching and more), this includes **a parity test that replays AquaSKK's acceptance test suite
(the test.dat of SKKInputSession_TEST, 400 cases) as is**; the output matches AquaSKK's engine in
every case.

The macOS tests are (also run in the CI macOS job):

```sh
sh swift/run-smoke-test.sh   # Swift ↔ FFI smoke test
sh macos/run-tests.sh        # key translation, settings files, user dictionary, IME decisions (ASCII restore, rule stacking order and more)
sh macos/run-ui-tests.sh     # preferences app UI (in an offscreen window; your settings are not touched)
sh macos/check-bundle.sh     # contents of the built .app (rule files, the preferences app, version)
```

## Differences from AquaSKK

- There is only one macOS input source, corresponding to AquaSKK's "AquaSKK 統合" (unified).
  AquaSKK's per-mode input sources ("ひらかな", "カタカナ", "半角カナ", "全角英数", "ASCII")
  are not supported. The engine keeps its input-mode handling so that hosts other than macOS
  can use it ([ADR 0014](docs/adr/0014-app-and-input-menu-icons.md)).

- Dictionary files may be UTF-8 as well as EUC-JP, detected per file (the `coding:` cookie
  first, then the contents). The user dictionary keeps the encoding of the existing file when
  saved.
- Character conversion does not use an encoding library; it uses tables generated from
  AquaSKK's EUC-JISX0213 ⇔ Unicode mapping (`src/jconv/eucjp_tables.rs`).
- Dictionary candidates such as `(concat "and\057or")` are shown and committed as the joined
  string (`and/or`) instead of the expression as written, as AquaSKK does. The dictionary and the
  user dictionary keep the expression form, so they can still be shared with other SKK
  implementations ([ADR 0012](docs/adr/0012-evaluate-concat-candidates.md)).
- Asynchronous dictionary loading (a pthread timer) is replaced with synchronous loading +
  `reload_if_updated()`.
- The `SKKBackEnd` / `SKKRomanKanaConverter` singletons are gone; they are passed around as
  ordinary values.
- There are workarounds for clients that act on keys the IME has already consumed (a zero-width
  space is held as marked text while the key is handled). Ghostty sends keys to the terminal
  without looking at the IME's result, so this covers every key that produces no text, such as
  `l` in hiragana mode (switch to ASCII). For Chromium-based apps (Electron apps, Chrome and
  others) it covers only Control keys. VSCode also receives a character committed by a single
  key (`a` → `あ`) as the key itself, so the committed character is first shown as marked text
  and then committed ([ADR 0010](docs/adr/0010-vscode-key-handling.md)).
- You can paste with Cmd-V while registering a word (AquaSKK supports only Ctrl-Y; newlines are
  removed; outside conversion, Cmd-V goes to the app as before). Chrome and some other apps
  handle Cmd-V as a menu shortcut before the IME sees it, so Cmd-V does not paste there; use
  Ctrl-Y instead (Ctrl-Y reaches the IME in every app).
- The Gadget dictionary's `jdate:` (Japanese era conversion), an empty stub in AquaSKK, is
  implemented (Reiwa / Heisei / Showa / Taisho / Meiji, with both eras shown for the year of an
  era change).
- AquaSKK's mask bug for combining characters in `eucj_to_utf8::emit()` (`0xfff`, which should be
  `0xffff`; JIS X 0213 combined kana such as か゚ were broken) is fixed. A patch for AquaSKK is in
  `upstream-patches/`.

## License and acknowledgements

GNU GPL v2 or later (following AquaSKK). See [NOTICE](NOTICE) for origins and acknowledgements.
