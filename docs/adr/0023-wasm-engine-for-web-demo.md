# ADR 0023: エンジンを wasm にしてリリースに添付し、ブラウザで試せるようにする

ステータス: 採択（未実装）

## 文脈

作業者の GitHub Pages (tett23/tett23) に NablaSKK の紹介ページを作る。ページの「試す」の節では、ブラウザの中で NablaSKK のエンジンを動かし、SKK で打てるようにする
(ページの仕様は tett23/tett23 の [ADR 0003](https://github.com/tett23/tett23/blob/main/.github/docs/adr/0003-nablaskk-page.md))。
ページ側は [ADR 0002](https://github.com/tett23/tett23/blob/main/.github/docs/adr/0002-product-pages-on-github-pages.md) で、受け入れの条件を次のように決めている。

- wasm は NablaSKK の CI でビルドし、GitHub Releases に添付する。ページ側では wasm をビルドしない。
- ページ側は `wasm.lock` に `名前 URL SHA-256` を書いて取り込む。添付するファイルの名前は版をまたいで変えない (版はタグで区別する)。
- ブラウザ (Safari と Chrome の最新版) と Deno の両方で、wasm を読み込むだけで動く。import は空か、ページ側で簡単に用意できるものだけにする。
- ブラウザのキーイベントを NablaSKK のキーイベントに変換するのは、ページ側の仕事とし、テストもページ側に置く。

`skk-cli` を改造する案は取らない。端末の raw モードとバイト列の入力に結びついていて、ブラウザに持ちこむとほぼ書き直しになる。エンジンを wasm にすれば、macOS の IME と同じ道筋で動く。

調べた結果は次のとおり。

- `nablaskk-core` は、今のままで `cargo build -p nablaskk-core --target wasm32-unknown-unknown` が通る。
  `std::fs` と `std::net` は wasm32-unknown-unknown でもコンパイルでき、呼ぶとエラーを返す。
- `nablaskk-core` に依存する試作の cdylib を wasm32-unknown-unknown でビルドし、Deno 2.9.7 で読み込んだ。結果は次のとおり。
  - `WebAssembly.Module.imports` は空だった。
  - セッションの作成に約 5ms かかった (SKK-JISYO.S を読む時間を含む)。
  - `Kanji` で ▽かんじ、Space で ▼漢字、Enter で「漢字」が確定した。`KaKu` で ▼書く になった。
  - 大きさは、ワークスペースの既定の release プロファイルで約 350KB (gzip で約 130KB) だった。
- 止まる (trap する) のは `LocalUserDictionary::open` だった。`Instant::now()` を呼ぶが、wasm32-unknown-unknown の std では時計が無く、panic する (wasm では `unreachable` の trap になる)。
  このほかに、セッションの作成からキー入力・変換・確定までの道筋で、時計・ファイル・ネットワークに触るところは無かった。
- `SystemTime` を使うのは、ファイルの更新時刻 (`std::fs::metadata`) とテストだけである。`SystemTime::now()` を呼ぶのはテストだけだった。
- skkserv の辞書 (`ProxyDictionary`)、自動更新の辞書 (`AutoUpdateDictionary`)、ファイルの辞書 (`CommonDictionary::open`) は、wasm の crate から作らなければ呼ばれない。リンクの段階で取り除かれ、import も増えない (上の試作で確かめた)。
- 日時の辞書 (`GadgetDictionary`) の時刻は、`cfg(unix)` で libc の `localtime_r` を使い、それ以外では既定値 (0 年 0 月 0 日) を返す。
- 辞書は文字列から読める (`CommonDictionary::from_text`)。文字コードは `Encoding::detect` で判別でき、EUC-JP は `jconv::utf8_from_eucj` で UTF-8 にできる。
- ローマ字かなルールの行がぶつかるかどうかは、設定アプリ (Swift) の `KanaRule.problem` と `KanaRuleSettings.conflicts(builtin:)` (`macos/Shared/KanaRuleConfig.swift`) が判定している。Rust の側には無い。
- SKK-JISYO.S (skk-dev/dict、最後に変わったコミット f415e667775957326de57a26ae891b155b567d6c、2019-04-27) は次のとおり。
  - 大きさは 55,683 バイト (gzip で 27,869 バイト)、3,410 行。文字コードは EUC-JP。SHA-256 は `9899d5a204bfd5e84d4bf3d52d52bcea724dadea7f5b416f2712a65411cb9285`。
  - ライセンスは GPL の第 2 版またはそれ以降 (ファイルの先頭に書いてある)。NablaSKK (GPL-2.0-or-later) と同じ条件で配布できる。
  - `かんじ /漢字/幹事/` と `かk /書/掛/欠/架/駆/懸/` を含む。
- キーマップとローマ字かなルールは、`nablaskk-ffi` と同じく `data/keymap.conf` と `data/kana-rule.utf8.conf` (`data/kana-rule.conf` の UTF-8 版) を `include_str!` で埋めこめる。

## 実装すること

### nablaskk-core

- `LocalUserDictionary::in_memory()` を足す。ファイルにも時計にも触らないユーザー辞書を作る。
  - 学習 (`register`・`remove`) はメモリの上で効く。単語登録した語は、同じセッションの中で引ける。
  - 保存 (`save`)、読み直し (`reload_if_changed`)、プライベートモードの切り替え、drop の時の保存は、何もしない。
  - そのために、最後に保存した時刻 (`last_save`) を `Option<Instant>` にする。ファイルのユーザー辞書は、今と同じく `open` のときに `Instant::now()` を入れる (保存の間引きの動きは変えない)。
- skkserv の辞書、自動更新の辞書、ファイルの辞書、日時の辞書には、cfg を足さない。wasm の crate はこれらを作らない。
  (引き継ぎでは cfg で外す案だったが、調べた結果、どれも wasm32-unknown-unknown でコンパイルでき、作らなければ呼ばれず、import も増えない。cfg を足すと、同じ型に 2 つの形ができる。)

### crates/nablaskk-wasm (新しい crate)

- `crate-type = ["cdylib", "rlib"]`。依存は `nablaskk-core` だけ (wasm-bindgen は使わない)。rlib は、同じ関数をネイティブのテストから呼ぶためである。
- エクスポートする関数は、`nablaskk-ffi` と同じく C の ABI で、名前の先頭を `nablaskk_` にする。文字列は線形メモリ上の (ポインタ, バイト長) で受け渡し、NUL 終端にしない。
  wasm32 なので、ポインタと長さは JavaScript では `number` (i32) になる。ページ側は `>>> 0` で符号なしにして使う。

  | 関数 | 引数 | 戻り値 |
  | --- | --- | --- |
  | `nablaskk_alloc` | `len: u32` | 確保した `len` バイトの先頭のポインタ |
  | `nablaskk_free` | `ptr, len: u32` | なし。`nablaskk_alloc` で確保したもの、またはこの wasm が返したバッファを解放する |
  | `nablaskk_session_new` | `keymap_ptr, keymap_len, rules_ptr, rules_len, dict_ptr, dict_len` | セッションのポインタ |
  | `nablaskk_session_free` | `session` | なし |
  | `nablaskk_handle` | `session, charcode: u8, keycode: u8, mods: u32` | `1` (IME が処理した) / `0` (処理しなかった) |
  | `nablaskk_take_state` | `session` | 表示の状態 (JSON) のバッファ |
  | `nablaskk_set_user_rules` | `session, ptr, len` | 各行の結果 (JSON) のバッファ |

  - 線形メモリは、Rust の既定どおり `memory` という名前でエクスポートされる。
  - この wasm が返すバッファは、先頭の 4 バイトが UTF-8 の本体のバイト長 (リトルエンディアンの u32) で、その後に本体が続く。ページ側は読んだ後に `nablaskk_free(ptr, 4 + 本体の長さ)` で解放する。
  - 引数で渡すバッファは、ページ側が `nablaskk_alloc` で確保して書きこみ、呼んだ後に `nablaskk_free` で解放する。wasm はそのバッファを持ちつづけない。
  - セッションは、`nablaskk-ffi` と同じくスレッドに対応しない。
- `nablaskk_session_new`:
  - キーマップは keymap.conf の形式の UTF-8。長さが 0 なら、埋めこんだ `data/keymap.conf` を使う。
  - ローマ字かなルールは kana-rule.conf の形式の UTF-8。長さが 0 なら、埋めこんだ `data/kana-rule.utf8.conf` を使う。
  - 辞書は SKK-JISYO の形式のバイト列。文字コード (EUC-JP か UTF-8 か) は wasm の中で `Encoding::detect` で判別する。長さが 0 なら辞書を足さない。
  - ユーザー辞書は `LocalUserDictionary::in_memory()` を使う。
  - エンジンの設定は `Config::default()` (本家の UserDefaults.plist の値) とし、注釈 (`enable_annotation`) だけを有効にする。ページで注釈を見せるためである。
- `nablaskk_handle` の引数は、`macos/Sources/KeyTranslator.swift` が作る値と同じにする。ページ側は、ブラウザの `KeyboardEvent` を次のように変換する。
  - `mods` はビットの和で、shift = 2、ctrl = 4、alt (Option) = 8、meta (Command) = 16 (`skk_session_handle` と keymap.conf と同じ)。
  - 次のキーは、`KeyboardEvent.key` で決まる値を渡す。

    | `key` | `charcode` | `keycode` |
    | --- | --- | --- |
    | `Enter` | `0x0d` | `36` (`0x24`) |
    | `Tab` | `0x09` | `48` (`0x30`) |
    | ` ` (Space) | `0x20` | `49` (`0x31`) |
    | `Backspace` | `0x08` | `51` (`0x33`) |
    | `Escape` | `0x1b` | `53` (`0x35`) |
    | `Delete` | `0x7f` | `117` (`0x75`) |
    | `ArrowLeft` | `0` | `123` (`0x7b`) |
    | `ArrowRight` | `0` | `124` (`0x7c`) |
    | `ArrowDown` | `0` | `125` (`0x7d`) |
    | `ArrowUp` | `0` | `126` (`0x7e`) |

  - そのほかの文字のキーの `charcode` は、`key` が ASCII の印字可能な 1 文字 (`0x21`〜`0x7e`) ならその文字にする。Shift を押していれば、Shift の付いた文字になる (`K`、`!`)。これは KeyTranslator が Shift のときに `characters` を使うのと同じである。
    Option を押していて `key` が ASCII でないとき (macOS の Option+a の `å` など) は、`KeyboardEvent.code` の物理キーの US 配列の文字にする (KeyTranslator の `charactersIgnoringModifiers` が Option を無視するのに合わせる)。どちらにも当たらなければ `0` にする。
  - Ctrl を押したときも、`charcode` は文字 (`ctrl::j` なら `j`) のままにする。KeyTranslator が制御文字を「文字 + ctrl」に直した後の値と同じになる。
  - そのほかの文字のキーの `keycode` は、`KeyboardEvent.code` の物理キーの macOS の仮想キーコード (`HIToolbox/Events.h` の `kVK_*`、`KeyA` なら `0x00`、`Semicolon` なら `0x29`) にする。対応の無いキーは `0xff` にする。
    同梱のキーマップは、文字のキーをキーコードで引かないが、AZIK などの補助のキーマップは引く (`keycode::0x21` など)。
  - テンキーは `Numpad0` などの `kVK_ANSI_Keypad*` のキーコードにする。同梱のキーマップは、これを直接入力 (`Direct`) に割り当てている。
- `nablaskk_take_state` は、表示の状態を JSON で返す。確定した文字列は、取り出すと空になる (`skk_session_take_fixed` と同じ)。

  ```json
  {
    "fixed": "漢字",
    "composing": "▼漢字",
    "cursor": 0,
    "mode": "hirakana",
    "candidates": { "visible": false, "items": [], "cursor": 0, "page": 0, "pageCount": 0 },
    "annotation": null
  }
  ```

  - `composing` は `Session::composing_string`、`cursor` は `skk_session_composing_cursor` と同じ (末尾からの文字数、0 は末尾)。
  - `mode` は `hirakana`・`katakana`・`jisx0201kana`・`ascii`・`jisx0208latin` のどれか。
  - `candidates` は候補のウィンドウの状態 (`nablaskk-ffi` の `WindowState` と同じ)。最初の Space では候補は ▼ の文字列に入り、ウィンドウは出ない。
    本家の既定 (`max_count_of_inline_candidates` = 4) どおり、▼ で 4 つの候補を見せた後の Space (5 回目) でウィンドウが出る。
  - `annotation` は選んでいる候補の注釈で、無ければ `null`。
  - JSON の文字列のエスケープは、外部の crate を使わずに書く (`"`、`\`、制御文字)。
- `nablaskk_set_user_rules` は、ユーザーのローマ字かなルールを丸ごと入れかえる (ページ側の「足し直す」)。
  - 渡すのは kana-rule.conf の形式の行 (`入力,ひらがな,カタカナ,半角カナ`) を改行でつないだ UTF-8。空の行と `#` で始まる行は読み飛ばす。
  - まずローマ字かなルールを `nablaskk_session_new` で使ったものに戻し、次に、問題もぶつかりも無い行だけを、上から順に足す。
  - 行の問題とぶつかりは、設定アプリの `KanaRule.problem` と `KanaRuleSettings.conflicts(builtin:)` を Rust に移した関数で判定する。判定の順番と理由の文言は設定アプリと同じにする
    (例: `ka.` は「「ka → か」がすぐに確定しなくなるため使えません」、`l` で始まる行は「l は英数モードへの切替キーなので、このルールは入力できません」)。
    組み込みのルールの名前は「組み込み」とし、組み込みのルールは `nablaskk_session_new` で使ったルールから読む (`KanaRuleSettings.builtinRules(from:)` と同じ読み方)。
  - 入力キーとひらがなは、設定アプリと同じく `&comma;`・`&space;`・`&sharp;` を戻してから判定する。
  - 4 つの欄がそろっていない行は反映せず、「入力,ひらがな,カタカナ,半角カナ を , で区切って書いてください」を理由にする。設定アプリは入力と出力の 2 つから残りを作るが、wasm では作らない。
  - 戻り値は、読み飛ばさなかった行ごとの結果の配列。

    ```json
    [
      { "line": 1, "input": "fj", "applied": true, "reason": null },
      { "line": 2, "input": "ka.", "applied": false, "reason": "「ka → か」がすぐに確定しなくなるため使えません" }
    ]
    ```

    `line` は渡した文字列の中の行番号 (1 から数える)。
- 新しいファイルの先頭には `Written by tett23, 2026.` と GPL-2.0-or-later の行を書く。ルールの判定は Swift の新規実装を移したものなので、同じく `Written by tett23, 2026.` とする。

### 同梱する辞書

- skk-dev/dict の SKK-JISYO.S (上のコミットのもの、EUC-JP のまま) を `data/SKK-JISYO.S` に置く。中身は変えない。
- NOTICE に、出所 (skk-dev/dict のコミット)、著作権者 (ファイルの先頭のとおり)、ライセンス (GPL v2 またはそれ以降) を書く。

### CI とリリース

- `.github/workflows/ci.yml` に `build-wasm` のジョブを足す (ubuntu-latest)。
  1. `rustup target add wasm32-unknown-unknown` し、`cargo build --release -p nablaskk-wasm --target wasm32-unknown-unknown` で wasm を作る。release プロファイルはワークスペースの既定のままにする。
  2. Deno を用意し (setup-deno の、Node 24 で動く最新のメジャー版)、下の Deno のテストを、できた wasm と `data/SKK-JISYO.S` で回す。
  3. `nablaskk.wasm` と `SKK-JISYO.S` を artifact にする。
- `release` のジョブの `needs` に `build-wasm` を足し、`nablaskk.wasm` と `SKK-JISYO.S` もリリースに添付する。ファイルの名前は版をまたいで変えない。
- `test` のジョブの `cargo test --workspace` と clippy は、`nablaskk-wasm` もネイティブで回す。

### 文書

- `docs/specifications.md` に wasm の節を足す (エクスポートする関数、受け渡しの形、キーの値の対応、状態とルールの結果の JSON、同梱する辞書)。
- README (日本語と英語) の「CI / リリース」に、添付するファイル (`nablaskk.wasm`、`SKK-JISYO.S`) を書く。

### ページ側への引き渡し

リリースの後に、tett23/tett23 のセッションに次を伝える: タグ、添付したファイルの URL と SHA-256、エクスポートした関数の一覧と入出力の形 (仕様書の wasm の節)。

## 実装しないこと

- skkserv への接続、大きい辞書 (SKK-JISYO.L など)、辞書の自動更新、日時の辞書 (`GadgetDictionary`)。
- ユーザー辞書の永続化 (ページを閉じると消える)。
- ブラウザのキーイベントの変換を wasm に入れること (ページ側の [ADR 0003](https://github.com/tett23/tett23/blob/main/.github/docs/adr/0003-nablaskk-page.md) でページ側の仕事としている)。
- エンジンの設定 (`SKK_OPTION_*`)、クリップボード、補完 (サジェスト)、入力モードの設定を wasm から変える関数。ページで要るようになったら、別の ADR で足す。
- 句読点の設定と、ユーザーのルールの 、。 を句読点の設定に合わせること ([ADR 0019](0019-user-rules-follow-punctuation-settings.md))。ページには句読点の設定が無い。
- 本家の受け入れテスト 400 ケースを wasm で回すこと。テストの設定 (`MockConfig`)、選択した文字列 (`sel=`)、クリップボード (`yank=`) を渡す口が要り、上の「エクスポートしない」とぶつかる。
- macOS 版 (IME、設定アプリ、`nablaskk-ffi`) の振る舞いの変更。設定アプリのルールの判定は Swift のまま残す。
- wasm の大きさを削る工夫 (`opt-level = "s"`、wasm-opt など)。gzip で約 130KB なので、今は要らない。

## テスト設計

実装の前に書いて、失敗するのを確かめる。

`nablaskk-core` のテスト (`cargo test`):

1. `LocalUserDictionary::in_memory()` に `register` した候補を `find` で引ける。`remove` で消える。
2. `in_memory()` の辞書で `save(true)` と `reload_if_changed()` を呼んでも、ファイルを作らず、学習した内容も消えない。
3. ファイルのユーザー辞書の保存の間引き (今のテスト) が変わらない。

`nablaskk-wasm` のテスト (`cargo test`、ネイティブ。エクスポートした関数を Rust から呼ぶ):

4. 引き継ぎのテスト: `Kanji` と打つと ▽かんじ、Space を押すと ▼漢字 になる。Enter で「漢字」が確定する。
5. 引き継ぎのテスト: `KaKu` と打つと ▼書く になる。
6. 引き継ぎのテスト: `fj,、,、,､` を `nablaskk_set_user_rules` で足すと `applied: true` で、`fj` と打つと「、」が確定する。
7. 引き継ぎのテスト: `ka.,…,…,…` を足すと `applied: false` で、理由は「「ka → か」がすぐに確定しなくなるため使えません」。その後も `ka` は「か」になる。
8. ルールの判定 (設定アプリの `macos/Tests/main.swift` のルールのテストと同じ例): 同じルール、入力キーの衝突、追加したルールどうしの衝突、大文字、`q`・`l`・`/` で始まる行、9 文字以上、空白、4 つの欄がそろわない行、`&comma;` を使った行、ほかのルールの先頭になる行 (`f` は通る)。
9. `nablaskk_set_user_rules` をもう一度呼ぶと、前に足したルールは消える (`fj` を外すと、`fj` は「、」にならない)。
10. 単語登録: 辞書に無い読みで Space を押すと登録になり、登録した語は同じセッションの中で引ける。
11. 状態の JSON: `mode` の 5 つの値、`candidates` (`Kai` と打って Space を 5 回押すと `visible: true` になり、`items` の先頭が 5 つ目の候補「改」)、`annotation`、確定した文字列が取り出すと空になること、`"` と `\` のエスケープ。
12. 長さ 0 のキーマップとルールで、埋めこんだものが使われる。長さ 0 の辞書では、`kanji` の Space で登録になる。
13. 辞書のバイト列が EUC-JP でも UTF-8 でも (同じ中身を UTF-8 にしたもの)、同じ変換になる。
14. 共通のシナリオのファイル (`crates/nablaskk-wasm/testdata/scenarios.txt`、キーの列と、期待する状態の JSON の組) のすべてが通る。シナリオは 4〜7 と 10 と、直接入力、カタカナ、半角カナ、英数、全角英数、送り仮名、取り消し (Esc と ctrl::g)、BackSpace を含める。

Deno のテスト (`build-wasm` のジョブ。wasm を `WebAssembly.instantiate(bytes, {})` で読みこむ):

15. `WebAssembly.Module.imports` が空である。
16. 14 と同じ `scenarios.txt` のすべてが通る (ネイティブのビルドと wasm のビルドで、同じキーの列から同じ状態が得られる)。
17. 4〜7 を、`nablaskk_alloc` と `nablaskk_free` を通して確かめる (ページ側の使い方と同じ)。

手で確かめる (実装の後):

- 作業者の手元の Chrome (アプリの中のブラウザ) で、wasm を読みこむだけの HTML を開き、4〜6 を打って確かめる。Safari での確認は、ページ側の実装のときに行う。
- リリースの下書きに `nablaskk.wasm` と `SKK-JISYO.S` が添付され、`SKK-JISYO.S` の SHA-256 が上の値と同じである。

## トレードオフ

- ローマ字かなルールの判定が、Swift (設定アプリ) と Rust (wasm) の 2 つになる。どちらかを変えたら、もう片方も同じに直す必要がある。同じ例のテストを両方に置いて、ずれを見つけやすくする。
  設定アプリを Rust の判定に切り替えれば 1 つにできるが、macOS 版を変えることになるので、この ADR ではしない。
- ページは、キーマップとルールを wasm に埋めこんだものを使う。版を上げるとページの動きも変わるが、ページ側は `wasm.lock` で版を固定しているので、上げるまでは変わらない。
- 辞書を 55KB ほどリポジトリに足す。CI で skk-dev/dict から取ってくれば足さずに済むが、CI がネットワークと skk-dev/dict に依存する。ネイティブのテストでも同じ辞書を使いたいので、置くことにした。
- 4 つの欄がそろわない行を受け付けないので、設定アプリの「入力と出力だけ」の書き方とは違う。カタカナと半角カナを作るのに Foundation の変換と同じ結果を Rust で作る必要があり、ページでは 4 つの欄で書けば足りるからである。
- 状態を JSON で返すので、ページ側は `JSON.parse` するだけでよい。その代わり、キーを押すたびに JSON を作って読む。キー 1 回あたり数百バイトなので、問題にならない。
- 本家の受け入れテストを wasm で回さないので、wasm の動きの保証は、ネイティブの受け入れテストと、共通のシナリオに頼る。wasm とネイティブで違うのは、ユーザー辞書と、ファイル・時計に触るところだけである。

## 参考文献

- tett23/tett23 の [ADR 0002](https://github.com/tett23/tett23/blob/main/.github/docs/adr/0002-product-pages-on-github-pages.md) と [ADR 0003](https://github.com/tett23/tett23/blob/main/.github/docs/adr/0003-nablaskk-page.md)
- `crates/nablaskk-core/src/dictionary/user.rs`、`crates/nablaskk-core/src/dictionary/common.rs`、`crates/nablaskk-core/src/dictionary/file.rs`、`crates/nablaskk-core/src/dictionary/gadget.rs`、`crates/nablaskk-core/src/config.rs`
- `crates/nablaskk-ffi/src/lib.rs`、`macos/Sources/KeyTranslator.swift`、`macos/Shared/KanaRuleConfig.swift`、`data/keymap.conf`、`data/kana-rule.utf8.conf`
- skk-dev/dict の SKK-JISYO.S: https://github.com/skk-dev/dict/blob/f415e667775957326de57a26ae891b155b567d6c/SKK-JISYO.S
- macOS の仮想キーコード: `HIToolbox/Events.h` (`kVK_ANSI_A` = 0x00、`kVK_Return` = 0x24、`kVK_LeftArrow` = 0x7B など)
- KeyboardEvent の `key` と `code`: https://www.w3.org/TR/uievents-key/ 、https://www.w3.org/TR/uievents-code/
- WebAssembly の JavaScript API (`WebAssembly.Module.imports`): https://webassembly.github.io/spec/js-api/
