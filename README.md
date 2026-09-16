# aquaskk-rust

[AquaSKK](https://github.com/codefirst/aquaskk) の入力エンジン(C++ 約22,000行)を
Rust で再実装したものです。外部クレート依存はありません(zero dependencies)。

## 構成

```
crates/
  aquaskk-core/   エンジン本体(ライブラリ)
  aquaskk-ffi/    C ABI(Swift / Objective-C ホスト向け)
  skkserv/        SKK 辞書サーバー(バイナリ)
  skk-cli/        ターミナルで SKK 入力を試せるデモ
data/             かな変換ルール・キーマップ(本家由来)
swift/            Swift ラッパーとスモークテスト
upstream-patches/ 本家 C++ 側の修正パッチ
```

### aquaskk-core

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

macOS の InputMethodKit 層(本家 `platform/mac`, Objective-C++)は含まれません。
`bridge::FrontEnd` などのトレイトを実装することで任意のホストに組み込めます。

### skkserv

skkserv プロトコル(ポート 1178)を話す辞書サーバーです。通信は EUC-JP。

```sh
cargo build --release
./target/release/skkserv -p 1178 /usr/share/skk/SKK-JISYO.L
# UTF-8 辞書の場合
./target/release/skkserv -u -p 1178 SKK-JISYO.utf8
```

本家では未実装だったサーバー補完(コマンド `4`)にも対応しています。

### skk-cli

ターミナル上で実際に SKK 入力を試せるデモです(raw mode、termios 直叩き)。

```sh
cargo run --release -p skk-cli -- /usr/share/skk/SKK-JISYO.L
```

### aquaskk-ffi + Swift

Swift / Objective-C から使うための C ABI です。ヘッダは
`crates/aquaskk-ffi/include/aquaskk.h`、Swift ラッパーは
`swift/SKKSession.swift`。

```sh
sh swift/run-smoke-test.sh   # Rust ビルド + swiftc でリンクして実行
```

macOS の InputMethodKit コントローラからは `handle(charcode:keycode:mods:)`
の戻り値と `takeFixed()` / `composing` を `insertText` / `setMarkedText`
に橋渡しする形で組み込めます。

## テスト

```sh
cargo test
```

本家のユニットテスト(SKKTrie_TEST, SKKKeymap_TEST など)を移植したものに加え、
keymap → 状態機械 → エディタ → 辞書を通したセッション統合テスト
(変換・送りあり変換・単語登録・補完・モード切替など)を含みます。

## 本家との差分

- 文字コード変換は encoding ライブラリを使わず、本家の
  EUC-JISX0213 ⇔ Unicode 対応表から生成したテーブル
  (`src/jconv/eucjp_tables.rs`)で実装しています。
- 辞書の非同期ロード(pthread タイマー)は同期ロード + `reload_if_updated()`
  に置き換えました。
- `SKKBackEnd` / `SKKRomanKanaConverter` のシングルトンは廃止し、
  通常の値として引き回します。
- Gadget 辞書の `jdate:`(元号変換)は本家では空実装でしたが、
  この移植では実装してあります(令和/平成/昭和/大正/明治、改元年の併記付き)。
- 本家 `eucj_to_utf8::emit()` にあった合成文字のマスクバグ(`0xfff` →
  正しくは `0xffff`。か゚等の JIS X 0213 合成かなが壊れる)は修正済みです。
  本家向けの修正パッチを `upstream-patches/` に置いてあります。

## ライセンス

GNU GPL v2 or later(本家 AquaSKK に準じます)。
