# ADR 0012: 辞書の (concat "...") の候補を文字列として表示する

ステータス: 採択

## 文脈

`/ao` と打って変換すると、`and/or` ではなく `(concat "and\057or")` がそのまま候補に
出て、そのまま確定する。作業者から報告があり、エンジンに直接キーを送って再現した。

SKK の辞書の書式では、候補の中に `/` と `;` を書けない(候補と注釈の区切りに使う)。
そのため、これらを含む候補は Emacs Lisp の式で書く。SKK-JISYO.L には
`ao /(concat "and\057or")/` のような行がある(`\057` は `/` の 8 進表記)。作業者の
ユーザー辞書にも、この候補を確定して学習した同じ形の行がある。

本家 AquaSKK にはこの式を評価する処理がなく、式をそのまま表示する(作業者が調査済み。
ソースにも `concat` を扱う処理がない)。DDSKK は `skk-quote-char` でこの形を書き、
macSKK と libskk は評価する(作業者のパッチのコミットメッセージによる)。NablaSKK は
本家の `SKKCandidate` を `crates/nablaskk-core/src/candidate.rs` の `Candidate` に
移植しているので、本家と同じく式をそのまま表示している。

作業者は本家向けのパッチ(`Evaluate (concat "...") candidates in dictionaries for
display`)を書いている。この ADR は、その方針を NablaSKK に移す。本家の挙動から意図的に
変える点なので、ADR に書く。

## 実装すること

- `Candidate::new`(辞書の `候補;注釈` を分ける方)で、候補が `(concat "..." ...)` の
  形で、引数が Emacs Lisp の文字列だけのときに、文字列を連結した結果を表示用の値
  (`variant`)にする。
  - 文字列の中のエスケープ: 8 進(`\057`、1〜3 桁)、`\n`、`\t`、`\uXXXX`、
    `\UXXXXXXXX`。それ以外の `\` の後の文字はその文字を表す(`\\`、`\"`)。
  - 次の場合は評価せず、今までどおり書かれたとおりに表示する: `(concat ` で始まらない、
    文字列以外の引数がある、文字列が閉じていない、閉じ括弧の後に文字がある、文字列が
    1 つもない、8 進で 0x80 以上の値(生のバイト)、範囲外やサロゲートの `\u`/`\U`。
- `word()` と `Display`(辞書の形に戻す処理)は辞書の形のまま変えない。ユーザー辞書には
  今までどおり `(concat "and\057or")` の形で書くので、ほかの SKK と辞書を共有できる。
- 表示も確定も `variant()` を使っているので、変換候補の表示、確定する文字、候補の
  重複の判定(`variant()` で比べる)は評価した結果になる。たとえば `(concat "(^^)")` と
  `(^^)` は同じ候補として扱う。
- 数値変換が `set_variant` で値を設定した場合は、そちらが優先する(今までどおり)。
- 単語登録で入力した単語(`Candidate::from_word`)は評価しない。
- 仕様書の辞書の節と、README の「本家との差分」に書く。

### データ構造

`Candidate` の構造は変えない。今まで数値変換だけが設定していた `variant` を、辞書から
読んだときにも設定する。

## 実装しないこと

- `concat` 以外の Lisp の式(`(skk-current-date)` など)の評価。
- 生のバイトを表す 8 進のエスケープ(0x80 以上)の解釈。
- 単語登録のときに `/` や `;` を含む単語を `(concat ...)` の形で書くこと(今の
  `[2f]` などのエスケープのまま)。
- サジェストの見出し語の変換(見出し語は対象外)。

## テスト設計

先に書いて失敗させてから実装する。テストは作業者のパッチのテストを移す。

- `candidate.rs` のテスト:
  - `(concat "and\057or")` の `variant()` が `and/or` で、`word()` と `to_string()` は
    `(concat "and\057or")` のままであること。
  - 注釈付きの `(concat "I\057O");入出力` で、`variant()` が `I/O`、注釈が `入出力`。
  - `(concat "^^\073")` → `^^;`、`(concat "(^^)")` → `(^^)`、`(concat "\\_\"")` → `\_"`、
    `(concat "改\n行")` → 改行入り、`(concat "a" "b")` → `ab`、
    `(concat "\u2022\U0001F600")` → `•😀`、`(concat "\57")` → `/`、
    `(concat "\0571")` → `/1`。
  - 評価しないもの: `(^^)`、`(concatenate "a")`、`(concat "a" b)`、`(concat "abc)`、
    `(concat "a") `(末尾に空白)は、書かれたとおりの値になること。
  - `(concat "(^^)")` と `(^^)` が同じ候補として扱われること。
  - 数値変換の `set_variant` が評価した値より優先すること。
- セッションのテスト(FFI か Swift のスモークテスト):
  1. `ao /(concat "and\057or")/` を含む一時辞書で `/ao` と打ち、Space を押すと、候補が
     `▼and/or` になること。
  2. 確定すると `and/or` が出力されること。
  3. ユーザー辞書の `ao` の行が `(concat "and\057or")` の形で保存されること。
- 修正前のコードでこれらが失敗することを確かめてから実装する。
- 既存の検証(`cargo test` の受け入れ 400 ケースを含む、`cargo clippy`、
  `sh swift/run-smoke-test.sh`、`sh macos/run-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が `/ao` → Space で `and/or` を確定できることを確かめる。

## トレードオフ

- 本家と挙動が変わる。本家では式がそのまま見えていたので、式の形で辞書を確かめて
  いた人には見え方が変わる。ただし辞書とユーザー辞書の中身は変えないので、本家や
  ほかの SKK に戻しても辞書はそのまま使える。
- 評価するのは文字列の連結だけで、ほかの式はそのまま表示する。Emacs Lisp を
  評価しないので安全だが、DDSKK で動く式の一部は NablaSKK では動かない。
- 重複の判定が評価した値になるので、同じ文字列を別の書き方で書いた候補は 1 つに
  まとまる。辞書を書いた人の意図とは違うかもしれないが、表示上は同じものなので
  まとめる方が自然である。

## 参考文献

- 作業者の AquaSKK 向けパッチ `Evaluate (concat "...") candidates in dictionaries for
  display`(`src/engine/entry/SKKCandidate.{h,cpp}`、`src/engine/tests/SKKCandidate_TEST.cpp`)
- AquaSKK `src/engine/entry/SKKCandidate.cpp`
  https://github.com/codefirst/aquaskk/blob/master/src/engine/entry/SKKCandidate.cpp
- SKK-JISYO.L(`ao`、`dosv`、`io`、`ioport` などの行)
  https://github.com/skk-dev/dict
- GNU Emacs Lisp Reference Manual, General Escape Syntax(文字列のエスケープ)
  https://www.gnu.org/software/emacs/manual/html_node/elisp/General-Escape-Syntax.html
