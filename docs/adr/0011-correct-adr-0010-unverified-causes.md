# ADR 0011: [ADR 0010](0010-vscode-key-handling.md) の実装で断定した原因を推測に戻し、ゼロ幅スペースのキャレット位置を記録する

ステータス: 採択

## 文脈

[ADR 0010](0010-vscode-key-handling.md)(VSCode でひらがなの確定と消費したキーがキーのまま届く不具合を直す)は
bab5809 で実装し、v1.1.5 でリリースした。作業者は、VSCode のターミナルとエディタの
両方で直ったことを実機で確かめた。

実装の途中で、上流のソースを読んで原因を調べた(xterm.js と Chromium、VSCodeVim と
Monaco)。その結論はコードを読んだだけの推測で、実機では追跡していない。それなのに、
次のところで推測を事実のように書いた。

- **コミット bab5809 のメッセージ**: ターミナルの症状を xtermjs/xterm.js#5348 と
  結び付け、エディタの症状について「VSCodeVim がゼロ幅スペースを残さなくなる
  (VSCodeVim/Vim#10111)」と書いた。
- **v1.1.5 のリリースノート**: 同じ結び付けを書いた。リリースノートは編集できるので、
  観察したことだけに直した。
- **上流へのコメントの下書き**: 作業者に渡したが、取り下げた。投稿はしていない。

コミットのメッセージは main の履歴にあり、書き換えない(main を rebase しない)。
[ADR 0010](0010-vscode-key-handling.md) は採択済みで、ステータスの行とリンク以外を書き換えない。[ADR 0010](0010-vscode-key-handling.md) は原因を
「仮説」として書いており、その書き方は正しい。

また、bab5809 には、[ADR 0010](0010-vscode-key-handling.md) に書いていない変更が 1 つ入っている。VSCode のときだけ、
キーを隠すゼロ幅スペースを置くときにキャレットを末尾に置く(`selectionRange` を
`(1, 0)` にする)。[ADR 0010](0010-vscode-key-handling.md) の「空の文字列で置き換える」に加えて、上の推測(Monaco が
キャレットの位置で出すコマンドを変え、VSCodeVim がその一部を追えない)をもとに足した。
エディタの不可視文字が直ったのは実機で確かめたが、直したのがこの変更なのか、空の
文字列での置き換えなのか、両方が要るのかは分かっていない。

実機で観察したこと:

- VSCode のターミナルで、IME が `あ` を確定しても端末に `a` が入り、IME が何も出さずに
  処理した `l` も入った(診断ログと作業者の報告)。tmux を使わない端末では確かめて
  いない。
- VSCode のエディタで、Vim 拡張の挿入モードのときに Ctrl-J の後に不可視の文字が入る
  らしい(作業者の報告)。入った文字が U+200B かどうかは確かめていない。
- v1.1.5 で、ターミナルとエディタの両方の症状が直った(作業者の確認)。

コードを読んだだけの推測(実機で確かめていない):

- Chromium は、キーの処理の前後にマークテキストがなく確定が 1 文字以下のとき、元の
  keydown と、確定した文字を載せた keypress を送る。xterm.js は keydown の `key` を
  端末へ送り、keypress を捨てる。
- Monaco は、キャレットを先頭にしたゼロ幅スペースを `compositionType` として出し、
  VSCodeVim はそれを横取りしない。取り消しの `replacePreviousChar` を VSCodeVim が
  捨てるので、U+200B が残る。
- 上の 2 つが、それぞれ xtermjs/xterm.js#5348 と VSCodeVim/Vim#10111 と同じ原因である。

## 実装すること

- 原因の扱いを決める: bab5809 のメッセージと [ADR 0010](0010-vscode-key-handling.md) にある原因の記述は、どれも
  確かめていない推測として扱う。上流の不具合だとは、この ADR では決めない。
- ゼロ幅スペースのキャレット位置を仕様として記録する: VSCode でキーを隠すときは、
  ゼロ幅スペースの後ろにキャレットを置き、後で空の文字列で置き換える。実機で直った
  組み合わせなので、確かめるまでは両方とも変えない。
- キャレット位置の判断を `ClientQuirks` に移し(どの隠し方でキャレットをどこに
  置くか)、自動テストで検査する。動作は変えない。
- 仕様書の [ADR 0010](0010-vscode-key-handling.md) の記述に、キャレットの位置と、
  原因が未確認であることを足す。
- [ADR 0010](0010-vscode-key-handling.md) のステータスの行を、この ADR を指すように変える。
- 上流への issue やコメントは、実機で追跡して確かめたことだけで書く。追跡の結果で
  推測が違うと分かったら、新しい ADR に書く。

## 実装しないこと

- コミットの履歴の書き換え(main の rebase)。
- [ADR 0010](0010-vscode-key-handling.md) の本文の書き換え。
- IME の動作の変更。
- 上流への投稿。

## テスト設計

先に書いて失敗させてから実装する。

- `macos/run-tests.sh` で、次を確かめる。
  - VSCode の隠し方(空の文字列で置き換える方)では、ゼロ幅スペースのキャレットが
    末尾(1)になること。
  - Ghostty と Chromium 系の隠し方では、キャレットが今までどおり先頭(0)のままで
    あること。
- 既存の検証(`cargo test`、`cargo clippy`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/build-app.sh`、`sh macos/check-bundle.sh`)が
  すべて通ること。
- `adr-guard links` と、[ADR 0010](0010-vscode-key-handling.md) のステータスの行だけを変えたコミットが
  `adr-guard pre-commit --allow-status --allow-links` を通ること。
- 動作を変えないので、実機での確認は要らない(作業者が使っている v1.1.5 と同じ動作)。

## トレードオフ

- 推測をもとに足したキャレットの変更を残す。外せば推測に頼らない実装になるが、実機で
  直った組み合わせを崩すことになり、エディタの不具合が戻るかもしれない。どちらが
  効いているかは、実機で追跡するまで分からない。
- コミットのメッセージは直せないので、履歴を読む人はこの ADR を見つけないと、推測を
  事実と受け取るおそれがある。[ADR 0010](0010-vscode-key-handling.md) のステータスの行からこの ADR へリンクする。

## 参考文献

- [ADR 0010](0010-vscode-key-handling.md)
- コミット bab5809(https://github.com/tett23/NablaSKK/commit/bab5809)
- v1.1.5 のリリースノート(https://github.com/tett23/NablaSKK/releases/tag/v1.1.5)
- 推測の材料にした上流のコードと報告(どれも実機では確かめていない)
  - Chromium `render_widget_host_view_cocoa.mm`
    https://github.com/chromium/chromium/blob/df1572d4ebbf1b6b7ab3ed65c8ef21c81145628b/content/app_shim_remote_cocoa/render_widget_host_view_cocoa.mm
  - xterm.js `Keyboard.ts`、`CoreBrowserTerminal.ts`
    https://github.com/xtermjs/xterm.js/tree/991857a1b1cacf06f9531ba74a6e927a7a6398bb/src
  - VSCodeVim `extensionBase.ts`
    https://github.com/VSCodeVim/Vim/blob/28f027545526363b1c0c577b71a3e175c3285123/extensionBase.ts
  - Monaco(VSCode)`nativeEditContext.ts`、`codeEditorWidget.ts`
    https://github.com/microsoft/vscode/tree/9576e160924a050614c4612adbb26bf73edf551a/src/vs/editor
  - https://github.com/xtermjs/xterm.js/issues/5348
  - https://github.com/VSCodeVim/Vim/issues/10111
