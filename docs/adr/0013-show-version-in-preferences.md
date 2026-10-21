# ADR 0013: 設定アプリにバージョンを表示する

ステータス: 採択（未実装）

## 文脈

設定アプリ(NablaSKK Preferences)には、NablaSKK のバージョンを確かめる場所がない。
作業者から「不便」と報告があった。

いまバージョンを確かめるには、`~/Library/Input Methods/NablaSKK.app` の
`Info.plist` を `PlistBuddy` などで読むしかない。しかも、バージョンだけでは
ビルドを見分けられないことがあった。

- リリースの前に手元でビルドしたものは、バージョンを上げる前なので、前のリリースと
  同じバージョンになる。2026-10-21 には、[ADR 0010](0010-vscode-key-handling.md) の実装を含むビルドがインストール
  されていたのに、`Info.plist` は 1.1.4 のままだった。
- 調査のために前のリリースへ戻すこと(v1.1.4 に戻して VSCodeVim を調べるなど)がある。
  どのビルドが動いているかを、作業者がすぐ確かめられる方がよい。

設定アプリは IME の `Contents/Resources` に同梱され、同じビルドで作られる。両方の
`Info.plist` のバージョンが `Cargo.toml` と一致することは `macos/check-bundle.sh` が
確かめている([ADR 0008](0008-regression-tests-for-untested-paths.md))。

## 実装すること

- `macos/build-app.sh` が、ビルドしたコミットを IME と設定アプリの両方の `Info.plist` に
  書く(新しいキー `NablaSKKCommit`、短いコミットハッシュ)。作業ツリーに未コミットの
  変更があれば、末尾に `+` を付ける。git が使えない場合はキーを書かない。
- 設定アプリのサイドバーの一番下に、`NablaSKK 1.2.0 (8128940)` の形で表示する。
  - バージョンは設定アプリの `CFBundleShortVersionString`、括弧の中は
    `NablaSKKCommit`。`NablaSKKCommit` がなければ括弧ごと省く。
  - 補足の色(`PreferencesPalette` の secondary)の小さい文字で表示し、選択して
    コピーできるようにする。
  - VoiceOver では「バージョン 1.2.0、コミット 8128940」と読む。
  - サイドバーのページ移動(↑↓、[ADR 0007](0007-preferences-accessibility-followups.md))の対象にはしない。
- 表示する文字列を作る処理は、`Info.plist` の辞書を受け取る関数にまとめ、
  `macos/run-tests.sh` で検査する。
- `macos/check-bundle.sh` で、両方の `Info.plist` の `NablaSKKCommit` が同じであること
  (片方だけにある場合も違反)を確かめる。
- 仕様書の設定アプリの節と、README に書く。

### データ構造

`Info.plist` に `NablaSKKCommit`(文字列、短いコミットハッシュ。未コミットの変更が
あれば末尾に `+`)を足す。ビルドのときに書くので、リポジトリの `Info.plist` には
書かない。

## 実装しないこと

- 入力メニューへのバージョンの表示。
- いま動いている IME のプロセスのバージョンを調べて表示すること。表示するのは
  設定アプリのバンドルのバージョンで、IME と同じビルドから作られる。
- 新しいバージョンがあるかを調べる機能。
- `CFBundleVersion`(いまは常に `1`)の扱いの変更。

## テスト設計

先に書いて失敗させてから実装する。

- `macos/run-tests.sh`:
  - `CFBundleShortVersionString` が `1.2.0`、`NablaSKKCommit` が `8128940` の辞書から
    `NablaSKK 1.2.0 (8128940)` を作ること。
  - `NablaSKKCommit` がない、または空なら `NablaSKK 1.2.0` になること。
  - `NablaSKKCommit` が `8128940+` ならそのまま表示すること。
  - バージョンがない辞書では `NablaSKK` だけになること。
  - 読み上げの文字列が「バージョン 1.2.0、コミット 8128940」になること。
- `macos/check-bundle.sh`: ビルドしたものが通ること。片方の `Info.plist` の
  `NablaSKKCommit` を消した複製と、値を変えた複製で失敗すること。
- `macos/run-ui-tests.sh`: 既存のサイドバーの検査(クリックで選んだ項目だけが
  強調される、↑↓ で移る)が、表示を足しても通ること。
- 既存の検証(`cargo test`、`cargo clippy`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/build-app.sh`、`sh macos/check-bundle.sh`)が
  すべて通ること。
- 実機で、作業者が設定アプリを開き、サイドバーの下にバージョンとコミットが出ること、
  選択してコピーできること、VoiceOver で読めることを確かめる。

## トレードオフ

- コミットハッシュをビルドのときに書くので、同じソースでもビルドした場所によって
  `+` の有無が変わる。手元のビルドとリリースのビルドを見分けやすくなる利点の方が
  大きい。
- サイドバーの下は狭いが、ページを開かずに見られる。「情報」のようなページを足す方法も
  あるが、1 行のためにページを増やすほどではない。

## 参考文献

- [ADR 0007](0007-preferences-accessibility-followups.md)(サイドバーのキーボード操作)
- [ADR 0008](0008-regression-tests-for-untested-paths.md)(`macos/check-bundle.sh`)
- Apple Developer Documentation, CFBundleShortVersionString
  https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleshortversionstring
