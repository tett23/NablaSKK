# ADR 0009: ローマ字かな規則の読み込みで全角スペースを落とさない

ステータス: 採択

## 文脈

ひらがなモードで `z` に続けてスペースを押すと、全角スペース(U+3000)ではなく
半角スペースが入る。作業者から報告があり、エンジンに直接キーを送って再現した
(`z` → `Space` で確定するのは 0x20)。`z.` → `…` など、z で始まるほかの規則は効く。

同梱の規則ファイル `data/kana-rule.utf8.conf` の 268 行目には `z ,　,　,　` が正しく
書かれている。原因は `crates/nablaskk-core/src/trie.rs` の `RomanKanaConverter::load`
にある。この関数は行を次の順で欄に分ける。

1. 半角スペースを `&space;` に置き換える。
2. `,` を半角スペースに置き換える。
3. `split_whitespace()` で区切る。

Rust の `split_whitespace()` は Unicode の White_Space で区切るので、U+3000 も
区切りとみなす。`z ,　,　,　` は `z&space;` の 1 欄だけになり、欄が 4 つに足りない
行として読み飛ばされる。そのため `z ` の規則が無く、`z` の後のスペースは規則に
当たらないまま半角で出る。

本家の `SKKRomanKanaConverter` は C++ のストリームの `>>` で欄を取り出す。`>>` が
区切りとみなすのは ASCII の空白だけなので、全角スペースは欄の中身として残り、
`z ` は全角スペースになる。この移植だけが本家と違う。

同じ問題は、出力が全角スペースだけになる規則すべてに起きる。同梱の規則では
`z ` の 1 行だけだが、ユーザーの `kana-rule.conf` やサブ規則の行も同じ関数で読む。
設定アプリ(Swift)の規則の読み込みは `,` で区切るので、この問題はない。本家の
受け入れテスト(`session_test.dat`)には `z ` のケースがないので、これまで検出
されなかった。

## 実装すること

- `RomanKanaConverter::load` の区切りを ASCII の空白だけにする
  (`split_whitespace()` をやめ、`char::is_ascii_whitespace` で区切って空の欄を除く)。
  エスケープ(`&space;`、`&comma;`、`&sharp;`)の扱いと、欄の数の判定は変えない。
- これで本家と同じく、`z ` はひらがな・カタカナ・半角カナのいずれでも全角スペースに
  なる。仕様書と README は本家どおりの挙動を前提にしているので、書き換えは要らない。

## 実装しないこと

- 規則ファイルの形式の変更。
- キーマップ(`keymap.rs`)の読み込みの変更。キーマップの中身は ASCII だけなので、
  `split_whitespace()` のままで問題ない。
- 設定アプリの規則の読み込みと表示の変更。

## テスト設計

先に書いて失敗させてから実装する。

- `trie.rs` のテストで、同梱の規則を読み込んだ変換器に `z ` を渡すと、ひらがな・
  カタカナ・半角カナのそれぞれで出力が `　`(U+3000)になること。
- `trie.rs` のテストで、出力に全角スペースを含む規則(例: `qq,あ　い,ア　イ,ｱ　ｲ`)を
  `load` で足すと、全角スペースが欄の中に残ること。
- FFI を通したセッションのテスト(`crates/nablaskk-ffi`)で、ひらがなモードで `z`、
  `Space` と送ると `　` が確定すること。
- 修正前のコードで上の 3 つが失敗することを確かめてから直す。
- 既存の検証(`cargo test` の受け入れ 400 ケースを含む、`cargo clippy`、
  `sh swift/run-smoke-test.sh`、`sh macos/run-tests.sh`)がすべて通ること。
- IME の挙動が変わるので、作業者が実機で `z` + `Space` を確かめてからコミットする。

## トレードオフ

- 規則の欄の区切りに、全角スペース以外の Unicode の空白(ノーブレークスペースなど)を
  使っている規則ファイルがあれば、これまでは区切りとして読めていたものが欄の中身に
  なる。本家も同じ読み方なので、本家と共有する規則ファイルの互換性はむしろ上がる。

## 参考文献

- AquaSKK `src/engine/trie/SKKRomanKanaConverter.cpp`
  https://github.com/codefirst/aquaskk/blob/master/src/engine/trie/SKKRomanKanaConverter.cpp
- Rust `str::split_whitespace`(Unicode の White_Space で区切る)
  https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace
