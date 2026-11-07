# ADR 0026: ローマ字かなルールの出力で abbrev モードに入る

ステータス: 採択（未実装）

## 文脈

作業者から、abbrev モードに入るキー `/` が遠いと要望があった。設定アプリの「ローマ字」ページの
ユーザー定義ルールで、決めた入力(2 文字など)を打つと abbrev モードに入るようにしたい。

今の仕組みは次のとおり。

- abbrev モードに入る操作は、キーマップの属性 `EnterAbbrev`(既定は `/`、`data/keymap.conf`)。
  かなモードで入力中でないときに押すと、abbrev の ▽ に入る(`session.rs` の `kana_input` で
  `AsciiEntry` へ移る)。ただし、押したキーがローマ字かなルールの続きになるとき
  (`can_convert`)は、ルールが先に効く。
- 「キー」ページで `EnterAbbrev` をほかのキーに変えられる。しかし割り当てられるのは 1 つの
  キーなので、近いキー(英字など)にすると、そのキーの文字が打てなくなる。
- ローマ字かなルールの出力は文字列で、確定すると編集器に入る(`InputEngine::route`)。出力で
  モードを変える仕組みはない。本家 AquaSKK にもない。NablaSKK 独自の拡張になる。

出力でキーの操作をする拡張は、[ADR 0018](0018-arrow-keys-as-kana-rule-output.md)(出力で方向キーを押す)で
試して破棄した。[ADR 0018](0018-arrow-keys-as-kana-rule-output.md) がうまく動かなかったのは、確定済みの文字の上でアプリのカーソルを
動かすため、IME からキーイベントを送る(`CGEventPost`)必要があったところである
(`docs/specifications.md` の 11 章)。abbrev モードに入るのはエンジンの中の状態の移り変わりだけで、
アプリにキーイベントを送らない。

## 実装すること

### ファイルの書き方

`kana-rule.conf` の出力の欄(ひらがな、カタカナ、半角カナ)で、欄の全体が `&abbrev;` のとき、
「abbrev モードに入る」として読む。既存の `&comma;` などと同じ実体参照の形にする。例:

```
jl,&abbrev;,&abbrev;,&abbrev;
```

- `&abbrev;` は欄の全体にだけ書ける。前後に文字があるとき(`あ&abbrev;` など)は、今までどおり
  文字列として読む(設定アプリでは注意を出して保存しない。下の「設定アプリ」)。
- 入力の欄の `&abbrev;` は、abbrev の指示としては扱わない。
- 5 番目の欄(続けて入力に残す文字。`jj,っ,ッ,ｯ,j` の `j`)は、abbrev のルールでは使わない。

### データ構造(エンジン)

- ルールを読むとき(`trie.rs`)、出力の欄の全体が `&abbrev;` なら、私用領域の 1 文字 U+F8FE に
  置き換える。木と変換の結果(`QueueState`)はこれまでどおり文字列で、abbrev の指示はその中の
  1 文字として運ぶ。
- 変換途中の表示(`intermediate`)には、この文字を出さない。

### 動き(エンジン)

- `InputEngine::route` で、確定した文字列が U+F8FE のとき、編集器には入れず、「abbrev に入る」
  という要求を覚える。
- `kana_input`(かなモードで入力中でない状態。カタカナと半角カナのモード、単語登録の入力欄も
  含む)で文字を処理したあと、要求があれば `AsciiEntry` に移る。`EnterAbbrev` のキーを
  押したときと同じ状態になる。
- ▽ の読みと送り仮名の入力中に要求が出たときは、何もしない
  (何も入らず、状態も変えない)。どの状態でも、キーの処理の終わりに要求を消す。
- 英数モード、全角英数モード、abbrev の中では、ルールを使わないので変わらない。

### 設定アプリ

- 「ローマ字」ページの説明文に、出力に `&abbrev;` と書くと abbrev モードに入ることを書く。
- 出力に `&abbrev;` を含み、出力の全体が `&abbrev;` でない行には、注意
  「&abbrev; は出力にそれだけを書いてください」を出し、保存しない(`KanaRule.problem` に足す)。
- カタカナと半角カナの出力は、今までどおりひらがなの出力から作る。`&abbrev;` は英字なので
  そのまま残る。
- wasm の規則の検査(`crates/nablaskk-wasm/src/rules.rs`、設定アプリの検査の移植、
  [ADR 0023](0023-wasm-engine-for-web-demo.md))にも、同じ検査と同じ文言を足す。

### 文書

- `docs/specifications.md` の 5.3 節(kana-rule.conf)と、README(日本語と英語)の設定アプリの説明に、
  `&abbrev;` を書く。本家に無い拡張であることも書く。

## 実装しないこと

- abbrev 以外のモードの切り替え(英数、全角英数、ひらがな⇔カタカナなど)を出力に書くこと。
  要るようになったら、同じ形で別の ADR で足す。
- 出力の途中や、文字のあとに `&abbrev;` を書くこと(`あ&abbrev;` で「あ」を入れてから abbrev に
  入る、など)。
- 方向キーなど、アプリにキーイベントを送ること([ADR 0018](0018-arrow-keys-as-kana-rule-output.md) で破棄した)。
- 組み込みのルールに abbrev のルールを足すこと。どの入力にするかは作業者が決める。

## テスト設計

テストを先に書いて失敗させてから実装する。

- `cargo test`(エンジン):
  - `trie.rs`: `jl,&abbrev;,&abbrev;,&abbrev;` を読むと、`jl` の出力が U+F8FE になること。
    `jl,あ&abbrev;,…` は文字列 `あ&abbrev;` のままであること。
  - セッション(組み込みのルールに `jl,&abbrev;,&abbrev;,&abbrev;` を重ねる):
    - ひらがなモードで `j` `l` を打つと abbrev の ▽ に入ること。続けて `n` `e` `t` Space で
      `/` から入ったときと同じ変換になること。
    - `j` だけを打った途中の表示が `j` であること(U+F8FE が出ないこと)。
    - カタカナモードと半角カナモードでも abbrev に入ること。
    - ▽ の読みの入力中(`K` `a` のあと)に `j` `l` を打つと、▽か のまま何も増えないこと。
    - ▼ の変換中に `j` `l` を打つと、候補が確定してから abbrev の ▽ に入ること(`j` で確定して
      かなモードに戻り、`l` でルールが完成するため。`/` を打ったときと同じ)。
    - 単語登録の入力欄で `j` `l` を打つと、登録の中で abbrev に入ること。
    - 英数モードで `j` `l` が `jl` のまま入ること。
    - 要求を出したキーの次のキーで、要求が残っていないこと(abbrev を抜けたあと、もう一度
      abbrev に入らないこと)。
  - 本家の受け入れテスト(400 ケース)がすべて一致すること。
- `macos/run-tests.sh`(`KanaRule.problem`):
  - 出力 `&abbrev;` は問題なし。
  - 出力 `あ&abbrev;`、`&abbrev;あ` は「&abbrev; は出力にそれだけを書いてください」。
  - `&abbrev;` の行を保存すると `jl,&abbrev;,&abbrev;,&abbrev;` が書かれること。
- `cargo test -p nablaskk-wasm` と `deno test`: wasm の検査が同じ文言を返すこと
  (`testdata/scenarios.txt` に `rules` の行を足す)。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が次を確かめる(IME の差し替えは作業者がする)。
  1. 設定アプリの「ローマ字」ページで、作業者が選んだ入力に出力 `&abbrev;` のルールを足すと、
     次に入力を始めたときから、その入力で abbrev の ▽ に入ること。
  2. `/` からも今までどおり入れること。
  3. Ghostty とテキストエディットの両方で、abbrev の変換と確定が `/` から入ったときと同じで
     あること。

## トレードオフ

- 出力の欄に、文字列でないものが入る。`&abbrev;` という文字列そのものを出力にしたい人は書けなく
  なる(`&amp;` のような書き方は足さない)。実際に困ることは少ないと考える。
- 本家の `kana-rule.conf` に無い書き方なので、本家で同じファイルを読むと、`&abbrev;` が文字列として
  入る。NablaSKK のユーザー定義ルールのファイルは NablaSKK の設定のディレクトリにあり、本家と
  共有しない。
- 「キー」ページの `EnterAbbrev` と、ルールの `&abbrev;` の 2 か所から abbrev に入れるようになる。
  1 つのキーで入るか、キーの並びで入るかで、役目が分かれる。

## 参考文献

- [ADR 0018](0018-arrow-keys-as-kana-rule-output.md)(破棄。出力でキーの操作をする拡張の前例)
- [ADR 0023](0023-wasm-engine-for-web-demo.md)(wasm の規則の検査)
- AquaSKK `src/engine/state/SKKStatePrimary-inl.h`(`EnterAbbrev`)
  https://github.com/codefirst/aquaskk
