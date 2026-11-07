# ADR 0025: 設定アプリの「キー」ページで sticky shift を割り当てられるようにする

ステータス: 採択

## 文脈

作業者は、設定アプリ(NablaSKK Preferences)で sticky shift を設定したい。

sticky shift は、Shift を押し続ける代わりに、決めたキー(よく使われるのは `;`)を 1 回押すと、
次の文字を大文字で打ったのと同じに扱う仕組みである。かなモードで押すと ▽ を始め、▽ の読みの
入力中に押すと送り仮名(`*`)を始める。

エンジンはすでに対応している。

- 本家 AquaSKK と同じく、キーマップの属性 `StickyKey` を読む(`crates/nablaskk-core/src/keymap.rs`)。
  状態ごとの動きも本家の移植である(`session.rs` の `kana_input` と ▽ の読みの入力中の処理。
  本家の `SKKStatePrimary-inl.h`、`SKKStateComposing-inl.h`)。
- 本家の受け入れテスト(`crates/nablaskk-core/testdata/session_test.dat` の「sticky shift」)が
  通っている。このテストは、テスト用のキーマップ(`testdata/keymap.conf`)で `;` を
  `StickyKey` にして、次を確かめている。
  - `;` `a` で ▽あ、`;` `r` `i` で ▼有り。
  - ▽ の読みが空のときに `;` を押すと、`；` が確定する。
- 本家の組み込みのキーマップ(`data/keymap.conf`)には `StickyKey` の行が無い。そのため、既定では
  どのキーも sticky shift にならない。

足りないのは設定の入り口だけである。今は、ユーザーのキーマップ(`keymap.conf`)を手で書けば
`StickyKey ;` を足せる。しかし、設定アプリの「キー」ページの一覧(`KeymapSettings.actions`)に
`StickyKey` が無いので、設定アプリからは割り当てられない。

設定の形は、次の 2 案から選んだ。

1. **「キー」ページの一覧に `StickyKey` の行を足す。** ほかの操作と同じく、キーを書いて割り当てる。
   既定のキーは無い。
2. 「ローマ字」ページに「; を sticky shift にする」のトグルを置く。

1 を選ぶ。

- キーを選べる。`;` は AZIK では促音(`っ`)に使う(`data/azik.rule`)。ほかのキーにしたい人もいる。
- 既定のキーが無い操作を「キー」ページに並べる形は、`SwitchToAsciiIfIdle`
  ([ADR 0022](0022-switch-to-ascii-if-idle.md))と同じである。設定ファイル、IME での読み込み、
  「デフォルトに戻す」の仕組みを、そのまま使える。
- 2 は、トグルとキーマップという 2 か所に、同じ設定を置くことになる。

## 実装すること

- `macos/Shared/KeymapConfig.swift` の `KeymapSettings.actions` に、
  `KeymapAction(symbol: "StickyKey", label: "sticky shift (▽・送り仮名の開始)", defaultKeys: "")`
  を足す。位置は `EnterJapanese`(「日本語入力開始 (Q)」)の次にする。
- 「キー」ページの説明文に 1 文足す。sticky shift のキーは、ローマ字かなルールより先に
  効くことを書く(例: `;` を割り当てると、ルールの `;` → `；` は、▽ の読みが空のときに
  2 回押して入れる)。
- `docs/specifications.md` の 5.2 節と、README(日本語と英語)の設定アプリの説明に、
  「キー」ページで `StickyKey` を割り当てられることを書く。

## 実装しないこと

- 既定のキーを決めること。本家と同じく、既定では割り当てない。
- エンジンの変更。sticky shift の動きは本家の移植のまま変えない。
- 「ローマ字」ページのルールの検査(`KanaRule.modeKeys`)を、ユーザーのキーマップに
  合わせること。sticky shift のキーで始まるルールは入力できなくなるが、今の検査は既定の
  `q`、`l`、`/` だけを見る。これを変えるのは別の ADR で扱う。
- 「ローマ字」ページのトグル(案 2)。

## テスト設計

テストを先に書いて失敗させてから実装する。

- `macos/run-tests.sh`(設定ファイルのテスト):
  - `KeymapSettings.actions` に `StickyKey` があり、既定のキーが空であること。
  - `StickyKey` に `;` を設定して保存すると、`keymap.conf` に `StickyKey ;` の行が書かれること。
    空にすると行が消えること。
  - `StickyKey ;` の行がある `keymap.conf` を読むと、`StickyKey` のキーが `;` になること。
- `cargo test`(エンジン): 組み込みのキーマップに、ユーザーのキーマップとして `StickyKey ;` を
  重ねたセッションで、次を確かめるテストを足す(本家の受け入れテストはテスト用のキーマップで
  確かめているので、IME と同じ重ね方でも動くことを確かめる)。
  - `;` `a` で ▽あ、`;` `r` `i` で ▼有り。
  - `;` `;` で `；` が確定すること。
  - 割り当てないときは、`;` で `；` が確定すること(今までどおり)。
- `sh macos/run-ui-tests.sh`: 「キー」ページを開くと「sticky shift」の行があること。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が次を確かめる(IME の差し替えは作業者がする)。
  1. 設定アプリの「キー」ページで sticky shift に `;` を書くと、次に入力を始めたときから
     `;` `k` `a` で ▽か、`;` `k` `a` `;` `k` `u` で ▼書く などになること。
  2. 欄を空にすると、`;` で `；` が入る動きに戻ること。

## トレードオフ

- sticky shift のキーは、ローマ字かなルールより先に効く。`;` を割り当てると、AZIK の `;` の
  ルールや、`;` で始まるユーザーのルールが使えなくなる。設定アプリはこれを警告しない
  (実装しないこと)。
- 一覧の行が 1 つ増える。既定のキーが空の行は、ほかに `SwitchToAsciiIfIdle` がある。

## 参考文献

- AquaSKK `src/engine/keymap/SKKKeymapEntry.cpp`(`StickyKey`)、
  `src/engine/state/SKKStatePrimary-inl.h`、`src/engine/state/SKKStateComposing-inl.h`、
  `src/engine/tests/keymap.conf`、`src/engine/tests/test.dat`(sticky shift)
  https://github.com/codefirst/aquaskk
- [ADR 0022](0022-switch-to-ascii-if-idle.md)
