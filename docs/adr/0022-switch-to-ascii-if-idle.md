# ADR 0022: 入力中は何もしない「英数モードへ」の操作 (SwitchToAsciiIfIdle) を足す

ステータス: 採択

## 文脈

作業者は、Ghostty の中で Esc を押すと、Karabiner-Elements が Esc に続けて F19 を送るようにしている。
NablaSKK のキーマップで F19 を「英数モードへ」(`SwitchToAscii`) に割り当て、vim で INSERT から NORMAL に戻ったときに、NablaSKK を英数モードにするためである
(作業者の dotfiles の `nablaskk/keymap.conf` の `SwitchToAscii l||keycode::0x50`)。

作業者から、nvim の挿入モードで ▼ の変換を Esc で取り消すと、読みが確定して入り、NORMAL に戻ってしまうと報告があった。調べた結果は次のとおり。

- nvim が受け取ったキーを記録すると (`vim.on_key`)、▼漢字 で Esc を 1 回押した後、`か`・`ん`・`じ` が届き、続く Esc で NORMAL に戻っていた。
- NablaSKK の診断ログでは、同じ時刻に、Esc (keycode 53) を `handled=true`・`wasComposing=true`・`mode=hirakana` で処理し、
  続く F19 (keycode 80) を `handled=true`・`wasComposing=true`・`mode=ascii` で処理していた。
- エンジンで確かめると (`Session::handle_event` に `data/keymap.conf` のキーマップで Esc を渡す)、Esc は次のように動き、どれもアプリに渡さない (`handled=true`)。

  | Esc の前 | Esc の後 |
  | --- | --- |
  | ▽かんじ | 入力中の文字なし |
  | ▼漢字 | ▽かんじ |
  | ▽おく*r | ▽おく |
  | ▼送り | ▽おく |

  ▼ で Esc を押すと ▽ に戻るのは本家と同じ動きである。
- ▽ の入力中に `SwitchToAscii` のキーを押すと、読みを確定して英数モードになる (`kana_entry` の `self.commit(); Transition(Ascii)`)。これも本家と同じ動きである。
- つまり、▼ で Esc を押すと ▽ に戻り、直後の F19 が ▽ の読みを確定して英数モードにする。英数モードでは次の Esc を処理しないので、Esc がアプリに届き、nvim が NORMAL に戻る。
- 今のキーマップの操作には、「入力中でなければ英数モードにし、入力中なら何もしない」ものが無い。`SwitchToAscii` を変えると、`l` で読みを確定して英数モードにする本家の動きも変わってしまう。

## 実装すること

- キーマップの操作 `SwitchToAsciiIfIdle` を足す。属性 (`attribute`) を 1 つ足し、`keymap.rs` で名前を引けるようにする。
- `SwitchToAsciiIfIdle` のキーの動き:
  - 入力中 (単語登録中、または入力中の文字がある。`Session::result` の「Always handled while registering or composing」と同じ条件) なら、何もしない。状態も入力中の文字も変えず、キーはアプリに渡さない。
  - 入力中でなければ、`SwitchToAscii` と同じく英数モードにする (かな・カタカナ・半角カナ・全角英数の各モードから)。
  - 英数モードでは、`SwitchToAscii` と同じく処理しない (今と同じく、キーはアプリに届く)。
- 既定のキーは割り当てない (`data/keymap.conf` には書かない)。
- 設定アプリの「キー」ページの操作の一覧 (`KeymapSettings.actions`) に、`SwitchToAsciiIfIdle` を「英数モードへ (入力中は何もしない)」として足す。既定のキーは空。
- 既定のキーが空の操作は、`data/keymap.conf` に行が無いことを、macOS のテストで確かめるように直す (今は、既定のキーと `data/keymap.conf` の行が一致することを確かめている)。
- これは本家に無い操作なので、`docs/specifications.md` のキーマップの節に、本家との違いとして書く。

## 実装しないこと

- `SwitchToAscii` の動きを変えること (本家と同じまま)。
- 英数モードで `SwitchToAsciiIfIdle` のキーをいつも処理すること (`AlwaysHandled` を設定アプリで書けるようにすること)。今と同じく、英数モードではキーがアプリに届く。
- 作業者の dotfiles の `nablaskk/keymap.conf` を直すこと。リリースの後に、dotfiles の ADR で `keycode::0x50` を `SwitchToAscii` から `SwitchToAsciiIfIdle` に移す。

## テスト設計

実装の前に書いて、失敗するのを確かめる。

`crates/nablaskk-core` のテスト (キーマップに `SwitchToAsciiIfIdle keycode::0x50` を足したもので、F19 のキーを渡す):

1. かなモードで入力中の文字が無いとき: 処理し (`handled=true`)、英数モードになる。
2. ▽かんじ のとき: 処理し、▽かんじ のまま、確定した文字は無く、かなモードのまま。
3. ▼漢字 のとき: 処理し、▼漢字 のまま、確定した文字は無い。
4. ▼漢字 で Esc、続けて F19 (作業者の操作の再現): ▽かんじ のまま、確定した文字は無い。
5. ▽おく*r (送り仮名の入力中) のとき: 処理し、入力中の文字は変わらない。
6. 単語登録中のとき: 処理し、登録を続ける。
7. 英数モードのとき: 処理しない (`handled=false`)、英数モードのまま。
8. カタカナ・全角英数モードで入力中の文字が無いとき: 英数モードになる。
9. キーマップ: `SwitchToAsciiIfIdle` の名前を引けて、その属性を持つ。`SwitchToAscii` の属性は持たない。
10. `SwitchToAscii` (`l`) の動きは変わらない (▽ で読みを確定して英数モード)。本家の受け入れテスト 400 ケースが全一致する。

macOS のテスト (`sh macos/run-tests.sh`):

11. 操作の一覧に `SwitchToAsciiIfIdle` があり、既定のキーは空で、`data/keymap.conf` に行が無い。
12. 設定ファイルの `SwitchToAsciiIfIdle keycode::0x50` を読んだエンジンで、F19 が上の 1 と 2 のように動く。

作業者が実機で確かめる (dotfiles の `keymap.conf` を一時的に `SwitchToAscii l` と `SwitchToAsciiIfIdle keycode::0x50` にして):

- Ghostty の nvim の挿入モードで ▼漢字 にして Esc を押すと、▽かんじ に戻り、挿入モードのままである。
- 続けて Esc を押すと、読みが消え、挿入モードのままである。もう一度 Esc を押すと NORMAL に戻り、NablaSKK は英数モードになる。
- 入力中の文字が無いときに Esc を押すと、今と同じく NORMAL に戻り、英数モードになる。

## トレードオフ

- 本家に無い操作が 1 つ増える。
- 入力中に Esc を押したとき、英数モードへの切り替えは起きない。NORMAL に戻るまでに、入力中の文字を消すための Esc が 1 回多く要る (▼ からなら Esc を 3 回)。
- 英数モードで F19 がアプリに届く点は、今と変わらない。

## 参考文献

- `crates/nablaskk-core/src/session.rs` (`kana_input`・`kana_entry`・`select_candidate`・`okuri_input`・`result`)、`crates/nablaskk-core/src/editor.rs` (`handle_cancel`)、`crates/nablaskk-core/src/keymap.rs`、`crates/nablaskk-core/src/event.rs`
- `macos/Shared/KeymapConfig.swift`、`macos/Tests/main.swift`、`data/keymap.conf`
- 作業者の環境での記録 (2026-11-05): nvim の `vim.on_key` で取ったキーの記録と、NablaSKK の診断ログ (`~/Library/Application Support/NablaSKK/debug.log`)
