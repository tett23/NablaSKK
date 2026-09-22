# NablaSKK

[AquaSKK](https://github.com/codefirst/aquaskk) の入力エンジン(C++ 約22,000行)を
Rust で再実装したものです。外部クレート依存はありません(zero dependencies)。

## インストール

### 必要なもの

- [Rust](https://rustup.rs/)(stable。ビルドのみに使用)
- macOS 入力メソッド / Swift 連携を使う場合: macOS 12 以降と
  Xcode Command Line Tools(`xcode-select --install`)

### macOS 入力メソッド (NablaSKK.app)

最も簡単なのはインストールスクリプトです(最新リリースを取得):

```sh
curl -fsSL https://raw.githubusercontent.com/tett23/NablaSKK/main/install.sh | sh
```

curl 経由のダウンロードには検疫属性が付かないため、無署名でも
Gatekeeper の警告は出ません。ソースからビルドする場合:

```sh
git clone https://github.com/tett23/NablaSKK
cd NablaSKK
sh macos/build-app.sh
cp -r macos/dist/NablaSKK.app ~/Library/Input\ Methods/
```

1. 一度ログアウトして再ログインします(既に旧版が動いている場合は
   `pkill NablaSKK` でも可)
2. 「システム設定 → キーボード → 入力ソース → 編集 → +」で
   「日本語」から **NablaSKK** を追加します
3. 辞書は、メニューバーの入力ソースメニューにある **「辞書を管理...」**
   で設定します(設定アプリが開きます)。辞書ファイルをウィンドウに
   ドラッグ&ドロップで追加でき、チェックボックスで有効/無効を切り替え、
   ドラッグや矢印ボタンで検索順を並べ替えられます。変更は即座に保存され、
   次にどこかのアプリで入力を始めたときに反映されます。

   設定の実体は `~/Library/Application Support/NablaSKK/dictionaries.conf`
   で、手で編集しても構いません(初回起動時に生成。既存 AquaSKK の
   SKK-JISYO.L があれば自動で利用します)。例:

   ```
   0 /usr/local/share/skk/SKK-JISYO.L
   - 5 /Users/me/my-dictionary.utf8
   4
   ```

   1行が「タイプ 場所」で、0=SKK-JISYO(EUC-JP/UTF-8 自動判別)、
   1=自動ダウンロード("host url path")、2=skkserv(host:port)、
   4=Gadget(today/now/=式)、5=SKK-JISYO(UTF-8 固定)。行頭の `-` は
   無効化した辞書です。

ユーザー辞書は `~/Library/Application Support/NablaSKK/skk-jisyo`
(UTF-8)に保存されます。設定アプリの「ユーザー辞書」タブで、見出し語の
検索・追加・削除、候補の追加・並べ替え・注釈の編集ができます。編集は
自動保存され、次にどこかのアプリで入力を始めたときに IME が再読み込み
します(IME 側に未保存の学習がある場合はそちらが優先されます)。

アンインストールは入力ソースから削除した上で:

```sh
rm -rf ~/Library/Input\ Methods/NablaSKK.app
rm -rf ~/Library/Application\ Support/NablaSKK   # 設定・ユーザー辞書ごと消す場合
```

### Gatekeeper の警告について

リリースのアーティファクトは ad-hoc 署名です。Gatekeeper の検証は
「検疫属性が付いたファイル」にのみ走るため、警告が出るかどうかは
入手経路で決まります:

- **インストールスクリプト / curl / ソースビルド** — 検疫属性が
  付かないため警告は出ません(推奨)
- **ブラウザで zip をダウンロード** — 警告が出ます。インストール後に
  一度だけ検疫属性を外してください:

  ```sh
  xattr -dr com.apple.quarantine ~/Library/Input\ Methods/NablaSKK.app
  ```

Developer ID 証明書がある場合は、リポジトリシークレット
(`MACOS_CERT_P12` ほか、`.github/workflows/ci.yml` のコメント参照)を
設定すると CI が署名+公証まで行い、警告なしで配布できます。
ローカルでは `SIGN_IDENTITY="Developer ID Application: ..." sh
macos/build-app.sh` で署名し、`xcrun notarytool submit --wait` →
`xcrun stapler staple` で公証します。

### 不具合調査用のログ

特定のアプリでキーが効かない等の調査用に、オプトインの診断ログがあります:

```sh
touch ~/Library/Application\ Support/NablaSKK/debug-enabled   # 有効化
tail -f ~/Library/Application\ Support/NablaSKK/debug.log
rm ~/Library/Application\ Support/NablaSKK/debug-enabled      # 無効化
```

記録するのはクライアントの bundle id・キーコード・修飾キー・処理結果のみで、
修飾なしの通常の文字キーは内容を記録しません。

### skkserv / skk-cli (コマンドラインツール)

```sh
cargo install --path crates/skkserv   # SKK 辞書サーバー
cargo install --path crates/skk-cli   # ターミナル用デモ
```

`~/.cargo/bin` に入ります。`cargo build --release` して
`target/release/` のバイナリを直接使っても構いません。

### ライブラリとして使う

Rust からは `nablaskk-core` をパス依存で、Swift / Objective-C からは
`nablaskk-ffi`(staticlib / cdylib + `include/nablaskk.h`)を
リンクして使います。詳細は後述の各節を参照してください。

## 構成

```
crates/
  nablaskk-core/   エンジン本体(ライブラリ)
  nablaskk-ffi/    C ABI(Swift / Objective-C ホスト向け)
  skkserv/        SKK 辞書サーバー(バイナリ)
  skk-cli/        ターミナルで SKK 入力を試せるデモ
data/             かな変換ルール・キーマップ(本家由来)
swift/            Swift ラッパーとスモークテスト
macos/            macOS 入力メソッド本体と辞書設定アプリ(SwiftUI)
upstream-patches/ 本家 C++ 側の修正パッチ
```

### nablaskk-core

本家 `src/engine` のポートです。モジュール対応:

| Rust モジュール | 本家 | 内容 |
|---|---|---|
| `jconv` | `utility/jconv` | かな変換・EUC-JP(JIS X 0213)⇔UTF-8 コーデック(テーブル生成による自前実装) |
| `trie` | `trie/` | kana-rule.conf の木とローマ字かな変換 |
| `candidate` | `entry/` | 変換候補・候補パーサ・送りヒント |
| `entry` | `entry/SKKEntry` | 見出し語と正規化 |
| `numeric` | `backend/SKKNumericConverter` | 数値変換 (#0〜#9) |
| `dictionary` | `dictionary/` `backend/` | SKK-JISYO の読み書き・共有/ユーザー/Gadget/skkserv/自動更新辞書・ファクトリ |
| `backend` | `backend/SKKBackEnd` | 辞書統合検索(シングルトンではなく通常の構造体) |
| `keymap` / `event` | `keymap/` | keymap.conf のパースとイベント解決 |
| `machine` | `state/GenericStateMachine.h` | 階層型状態機械(メソッドポインタの代わりに StateId enum) |
| `editor` | `editor/` | テキストバッファ・入力キュー・エディタスタック |
| `selector` / `completer` | `selector/` `completer/` | 候補選択・見出し語補完 |
| `session` | `state/` `session/` | SKK 状態遷移と再帰的単語登録セッション |
| `bridge` / `config` | `bridge/` | ホスト UI 境界のトレイトと設定 |
| `calculator` | `utility/calculator.h` | =式 の簡易計算 |

macOS の InputMethodKit 層は本家(Objective-C++)の移植ではなく、
`macos/` に Swift + FFI で新規実装しています。エンジン自体は
`bridge::FrontEnd` などのトレイトを実装することで任意のホストに組み込めます。

### skkserv

skkserv プロトコル(ポート 1178)を話す辞書サーバーです。通信は EUC-JP。

```sh
cargo build --release
./target/release/skkserv -p 1178 /usr/share/skk/SKK-JISYO.L
```

辞書のエンコーディング(EUC-JP / UTF-8)はファイルごとに自動判別されます
(先頭行の `coding:` クッキー優先、なければ内容で判定)。
`-e` / `-u` で強制指定もできます。

本家では未実装だったサーバー補完(コマンド `4`)にも対応しています。

### skk-cli

ターミナル上で実際に SKK 入力を試せるデモです(raw mode、termios 直叩き)。

```sh
cargo run --release -p skk-cli -- /usr/share/skk/SKK-JISYO.L
```

### macOS 入力メソッド (NablaSKK.app)

InputMethodKit 製の実際に使える IME です(インストール手順は上記
「[インストール](#インストール)」参照)。NSEvent をエンジンイベントへ
変換して FFI 越しに Rust のセッションを駆動します。候補ウィンドウは
マークテキスト内へのインライン描画で、a/s/d/f/j/k/l で選択できます。

### nablaskk-ffi + Swift

Swift / Objective-C から使うための C ABI です。ヘッダは
`crates/nablaskk-ffi/include/nablaskk.h`、Swift ラッパーは
`swift/SKKSession.swift`。

```sh
sh swift/run-smoke-test.sh   # Rust ビルド + swiftc でリンクして実行
```

macOS の InputMethodKit コントローラからは `handle(charcode:keycode:mods:)`
の戻り値と `takeFixed()` / `composing` を `insertText` / `setMarkedText`
に橋渡しする形で組み込めます。

## CI / リリース

GitHub Actions(`.github/workflows/ci.yml`)が push / PR ごとに
テスト(受け入れ400ケース含む)・clippy・Swiftスモークテストを実行し、
ビルド済みアーティファクトを生成します:

- `NablaSKK-macos-universal.app.zip` — 入力メソッド本体(arm64 + x86_64)
- `nablaskk-tools-macos-universal.tar.gz` / `nablaskk-tools-linux-x86_64.tar.gz`
  — skkserv と skk-cli
- `nablaskk-ffi-macos-universal.tar.gz` — libnablaskk_ffi.a + nablaskk.h +
  SKKSession.swift(Swift 組み込み用)

`v*` タグを push すると、これらを添付したドラフトリリースが作成されます:

```sh
git tag v0.1.0 && git push origin v0.1.0
```

ローカルでユニバーサル版アプリを作るには `UNIVERSAL=1 sh macos/build-app.sh`。

## テスト

```sh
cargo test
```

本家のユニットテスト(SKKTrie_TEST, SKKKeymap_TEST など)の移植、
セッション統合テスト(変換・送りあり変換・単語登録・補完・モード切替など)に加え、
**本家の受け入れテストスイート(SKKInputSession_TEST の test.dat、400ケース)を
そのまま再生するパリティテスト**を含み、全ケースで本家エンジンと出力が一致します。

## 本家との差分

- 辞書ファイルのエンコーディングは EUC-JP に加えて UTF-8 に対応し、
  ファイルごとに自動判別します(`coding:` クッキー → 内容判定の順。
  ユーザー辞書は既存ファイルのエンコーディングを保存時も維持します)。
- 文字コード変換は encoding ライブラリを使わず、本家の
  EUC-JISX0213 ⇔ Unicode 対応表から生成したテーブル
  (`src/jconv/eucjp_tables.rs`)で実装しています。
- 辞書の非同期ロード(pthread タイマー)は同期ロード + `reload_if_updated()`
  に置き換えました。
- `SKKBackEnd` / `SKKRomanKanaConverter` のシングルトンは廃止し、
  通常の値として引き回します。
- IME が消費したキーを自前でも処理してしまうクライアントへの対策を入れています
  (キー処理の間だけゼロ幅スペースをマークテキストとして保持)。
  Ghostty は IME の処理結果を見ずにキーを端末へ送るため、ひらがなモードの `l`
  (ASCII へ切替)などテキストを出さないキーすべてが対象です。Chromium 系
  (VSCode などの Electron アプリ、Chrome 等)は Ctrl 系キーのみ対象です。
- 単語登録中・見出し語入力中に Cmd-V でペーストできます(本家は Ctrl-Y のみ。
  改行は除去されます。変換中でないときの Cmd-V は従来どおりアプリに渡ります)。
- Gadget 辞書の `jdate:`(元号変換)は本家では空実装でしたが、
  この移植では実装してあります(令和/平成/昭和/大正/明治、改元年の併記付き)。
- 本家 `eucj_to_utf8::emit()` にあった合成文字のマスクバグ(`0xfff` →
  正しくは `0xffff`。か゚等の JIS X 0213 合成かなが壊れる)は修正済みです。
  本家向けの修正パッチを `upstream-patches/` に置いてあります。

## ライセンスと謝辞

GNU GPL v2 or later(本家 AquaSKK に準じます)。由来と謝辞は
[NOTICE](NOTICE) を参照してください。
