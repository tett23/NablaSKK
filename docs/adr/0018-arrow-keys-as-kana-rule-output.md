# ADR 0018: ローマ字かなルールの出力で方向キーを押す

ステータス: 破棄

## 文脈

作業者は、ローマ字かなルールの出力として、方向キーを押したのと同じ動き(カーソルの移動)を
させたい。例えば `hh` と打つとカーソルが 1 つ左へ動く、`z(` と打つと `()` が入って
カーソルが括弧の内側へ戻る、といった使い方である。

[ADR 0020](0020-arrow-keys-in-kana-rule-input.md) では、要望を「方向キーをルールの入力にする」と取り違えて実装し、
破棄した。この ADR は、出力で方向キーを押す方の要望を扱う。

今の仕組みと制約は次のとおり。

- ルールの出力は文字列で、確定した文字列は、入力中でなければアプリに挿入され、▽ の読みの
  入力中なら読みに入る(`InputEngine::route`)。
- エンジンの中には、カーソルを動かす仕組みがすでにある。カーソル移動の事象
  (`EditorEvent::CursorLeft` など)は、いちばん上の編集器が受け取る。▽ の読みや送り仮名、
  単語登録の入力欄なら、その中でカーソルが動く。入力中でなければ(`PrimaryEditor`)、
  処理せずアプリへ渡す(`event_handled = false`)。
- IME から、確定済みの文字の上でアプリのカーソルを動かす方法は限られる。InputMethodKit の
  `IMKTextInput` にはカーソルを動かす API がない。`insertText:replacementRange:` で
  範囲を置き換える方法は、左右にしか動かせず、ターミナル(Ghostty など)では使えない。
  どのアプリでも上下左右に動かせるのは、IME が方向キーのキーイベントを作って送る
  (`CGEventPost`)方法だけである。キーイベントを送るには、送るプロセスがアクセシビリティの
  許可(システム設定 › プライバシーとセキュリティ › アクセシビリティ)を得ている必要がある。
- NablaSKK.app は ad-hoc 署名である。macOS のプライバシーの許可(TCC)はコード署名に
  結び付くので、作り直すたびに許可を与え直す必要がある見込みが高い(実機で確かめる)。

作業者と相談し、キーイベントを送る方法を選んだ。▽ の読みの入力中など、エンジンの中で
カーソルを動かせるときは、キーイベントは送らずにエンジンの中で動かす。

本家 AquaSKK には、ルールの出力でキーを押す仕組みはない。NablaSKK 独自の拡張になる。

## 実装すること

### ファイルの書き方

`kana-rule.conf` の出力の欄(ひらがな・カタカナ・半角カナ)で、次の 4 つを方向キーとして
読む。既存の `&comma;` などと同じ実体参照の形にする。

| 書き方 | キー |
|---|---|
| `&left;` | ← |
| `&right;` | → |
| `&up;` | ↑ |
| `&down;` | ↓ |

方向キーは出力の末尾にだけ置ける(いくつでもよい)。前に文字があってもよい。例:

```
hh,&left;,&left;,&left;
z(,()&left;,()&left;,()&left;
```

入力の欄の `&left;` などは、方向キーとしては扱わない([ADR 0020](0020-arrow-keys-in-kana-rule-input.md) を破棄したため)。

### データ構造(エンジン)

- ルールを読むとき、出力の欄の `&left;` `&right;` `&up;` `&down;` を、Unicode の私用領域の
  文字 U+F702、U+F703、U+F700、U+F701 に置き換える(AppKit が方向キーに使う
  `NSLeftArrowFunctionKey` などと同じ値)。木と変換の結果(`ConversionResult`、
  `QueueState`)はこれまでどおり文字列で、方向キーはその中の 1 文字として運ぶ。
- 変換途中の表示(`intermediate`。`n` を打ったときの `ん` など)には、これらの文字を出さない。
- `FrontEnd` トレイトに、アプリへ方向キーを送るよう頼むメソッド `send_cursor_key` を足す
  (既定の実装は何もしない)。`BufferedFrontEnd` は頼まれた方向キーを順にためる。

### 動き(エンジン)

- `InputEngine::route` で、確定した文字列を方向キーの文字の前後で分け、先頭から順に、
  文字は今までどおり編集器へ、方向キーはカーソル移動の事象として、いちばん上の編集器へ渡す。
  - ▽ の読み、送り仮名、単語登録の入力欄では、その中でカーソルが動く。
  - 入力中でない(`PrimaryEditor`)ときは、`FrontEnd::send_cursor_key` でアプリへ方向キーを
    送るよう頼み、キーは処理したことにする。
- 英数モードと全角英数モードでは、ルールを使わないので変わらない。

### FFI と IME

- FFI に、たまった方向キーを取り出す `skk_session_take_cursor_keys` を足す(取り出すと空に
  なる。`skk_session_take_fixed` と同じ扱い)。`include/nablaskk.h` と
  `swift/SKKSession.swift` も更新する。
- IME(`SKKRustInputController`)は、キーを処理して確定文字列を挿入したあとに方向キーを
  取り出し、キーごとに `CGEvent` の keyDown と keyUp(keyCode 123〜126)を作って送る。
  送るイベントには目印(`eventSourceUserData`)を付け、IME に戻ってきたときは
  エンジンに渡さずアプリへそのまま渡す。
- アクセシビリティの許可がないとき(`AXIsProcessTrusted()` が false)は、キーイベントを
  送らず、診断ログに記録する。そのプロセスで初めてのときだけ、許可を求めるダイアログを出す
  (`AXIsProcessTrustedWithOptions` の prompt)。
- ルールのキーが何も文字を出さずに処理された形になるので、VSCode などの Chromium 系では、
  今のマスク([ADR 0010](0010-vscode-key-handling.md))がそのまま当てはまるかを確かめ、要るなら同じ扱いにする。

### 設定アプリ

- 「ローマ字」ページの出力の欄の隣に、方向キーを出力の末尾に足すメニュー(← 左、→ 右、
  ↑ 上、↓ 下)を置く。出力の欄には `&left;` などの書き方がそのまま出る。
- 方向キーが末尾以外にある出力(`&left;あ` など)は、理由を示して保存しない
  (「方向キーは出力の最後にだけ置けます」)。
- カタカナ・半角カナの出力を作るとき、`&left;` などは変えない。
- 方向キーを出力するルールがあるときは、ページに、アクセシビリティの許可が要ること
  (入力中でないときにカーソルを動かすため)を書く。
- 仕様書(5.3 節)と README(日本語と英語)に、書き方、制限、許可のことを書く。

## 実装しないこと

- 方向キー以外のキー(Enter、Tab、Backspace、Home/End、修飾キー付きのキーなど)を出力に
  書くこと。要るとわかったら別の ADR で足す。
- 方向キーを出力の先頭や途中に置くこと。IME は確定文字列をすぐ挿入し、キーイベントは
  そのあとにアプリへ届くので、「方向キーのあとに文字」は順番が守れない。
- 方向キーを入力の欄に書くこと([ADR 0020](0020-arrow-keys-in-kana-rule-input.md)、破棄)。
- `insertText:replacementRange:` でカーソルを動かす方法。
- アクセシビリティの許可を設定アプリから確かめたり、与えたりすること(許可は IME の
  プロセスに対するもので、設定アプリからは調べられない)。
- 署名を変えて、作り直しても許可が残るようにすること(Developer ID での署名は
  ロードマップの Nix の配布と合わせて考える)。

## テスト設計

先に書いて失敗させてから実装する。

- `trie.rs`:
  - `hh,&left;,&left;,&left;` を読むと、`hh` の出力が U+F702 になること。`&right;` `&up;`
    `&down;` も同様。
  - `z(,()&left;,…` の出力が `()` + U+F702 になること。
  - 入力の欄の `&left;` は方向キーにならないこと。出力の欄のふつうの `←`(組み込みの `zh`)は
    文字のままであること。
- `session_test.rs`(セッションを通したテスト。`BufferedFrontEnd` にたまった方向キーを見る):
  - ひらがなモードで入力中でないとき、`hh` で確定文字列は空、方向キーは [←]、キーは処理済み。
  - `z(` で確定文字列は `()`、方向キーは [←]。
  - ▽ の読みの途中(`Kana` のあとに `hh`)では、方向キーは送られず、読みの中でカーソルが
    動く(続けて `i` を打つと `▽かいな`)。`z(` なら読みが `▽かな()` でカーソルが `)` の前。
  - カタカナ・半角カナのモードでも同じに働くこと。
  - 変換途中の表示に U+F700〜U+F703 が出ないこと(`x` → `&left;` と `xa` がある木で `x`)。
  - 英数モードでは何も起きないこと。
  - 単語登録の入力欄でもカーソルが動くこと。
- 本家の受け入れテスト(`session_test.dat`、400 ケース)が全部通ること。
- FFI(`nablaskk-ffi` のテストと `sh swift/run-smoke-test.sh`): `skk_session_take_cursor_keys`
  が送る方向キーを順に返し、取り出すと空になること。
- `sh macos/run-tests.sh`:
  - `KanaRule`: `&left;`、`()&left;`、`&left;&left;` が問題なしで、`&left;あ` が
    「方向キーは出力の最後にだけ置けます」になること。カタカナ・半角カナの出力で `&left;` が
    変わらないこと。ファイルへの書き出しと読み戻し。メニューから足すと末尾に付くこと。
  - SKKSession を通して、`hh` で方向キー [←] が取り出せること。
  - IME の判断(`InputMethodLogic`): 方向キーをキーコード 123〜126 にする変換と、目印の
    付いたイベントをエンジンに渡さない判断。
- `sh macos/run-ui-tests.sh`: 出力の欄の隣のメニューから ← を選ぶと、出力の末尾に `&left;`
  が付くこと。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)が通ること。
- 実機で作業者が確かめる(キーイベントを送る部分は自動テストできない):
  - 初めて方向キーのルールを使うと、アクセシビリティの許可を求められること。許可の前は
    カーソルが動かず、診断ログに記録されること。
  - 許可したあと、TextEdit、CotEditor、Ghostty、Chrome、VSCode、Claude で、入力中でない
    ときに `hh` でカーソルが左へ動き、`z(` で `()` の内側にカーソルが来ること。
  - ▽ の読みの途中で `hh` を打つと、読みの中でカーソルが動くこと。
  - NablaSKK を作り直して差し替えたあと、許可を与え直す必要があるか(結果を ADR の
    ステータスの行は変えずに、仕様書と README に書く)。

## トレードオフ

- キーイベントを送るので、NablaSKK にアクセシビリティの許可を与える必要がある。この許可が
  あると、IME はほかのアプリを操作できる。許可を要らなくするには、▽ の読みの中でしか
  動かせない形にするしかなく、作業者の使い方には足りない。
- ad-hoc 署名なので、作り直すたびに許可を与え直すことになる見込みが高い。ドッグフーディングで
  頻繁に差し替える作業者には手間になる。
- 方向キーは出力の末尾にしか置けない。`&left;あ` のような順番は守れないので、書けないように
  する。
- 方向キーを私用領域の文字で運ぶので、木や変換の結果の型を変えずに済む。その代わり、
  文字列を扱う場所(変換途中の表示など)で、これらの文字を出さないように気を付ける必要がある。
  方向キーの列を別の値として運べば取り違えは起きないが、変換の結果を受け取る部分を
  すべて変えることになる。
- 出力の欄に `&left;` がそのまま見える。矢印の記号で見せる方が分かりやすいが、出力の文字
  としての `←`(組み込みの `zh` など)と見分けが付かなくなる。

## 参考文献

- Apple Developer Documentation, `CGEvent.init(keyboardEventSource:virtualKey:keyDown:)`、
  `CGEvent.post(tap:)`
  https://developer.apple.com/documentation/coregraphics/cgevent
- Apple Developer Documentation, `AXIsProcessTrustedWithOptions(_:)`
  https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions
- Apple Developer Documentation, `IMKTextInput`
  https://developer.apple.com/documentation/inputmethodkit/imktextinput
- Apple Developer Documentation, `NSLeftArrowFunctionKey` などの関数キーの文字
  https://developer.apple.com/documentation/appkit/1535851-function-key_unicodes
- AquaSKK `src/engine/editor/SKKInputEngine.cpp`、`src/engine/editor/SKKPrimaryEditor.cpp`
  https://github.com/codefirst/aquaskk
