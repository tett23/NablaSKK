# ADR 0027: 本家の設定画面にあって NablaSKK にない変換の設定を、設定アプリに足す

ステータス: 採択

## 文脈

`docs/roadmap.md` の優先度が高い候補に、「設定アプリでオプションを変えられるようにする」がある。
egg-like-newline など、本家の設定項目のうち、まだ設定アプリにないものを足すという候補である。
作業者から、本家 AquaSKK の設定画面にあって NablaSKK にないものを調べて ADR にするよう頼まれた。

### 調べ方

- 本家の設定画面は `platform/mac/proj/Preferences.xib` にある。ここから、どの部品がどのキーに
  バインドされているかと、各部品の表示名を読んだ。
- 既定値は `platform/mac/plist/UserDefaults.plist` から読んだ。
- IME がそれぞれの値をどう使うかは `platform/mac/src/` から読んだ。
  - `server/SKKServer.mm` の `reloadUserDefaults`
  - `server/MacCandidateWindow.mm`、`server/MacInputModeWindow.mm`、`server/MacMessenger.mm`
  - `server/SKKInputController.mm`
  - `preferences/PreferenceController.mm`
- 読んだのは codefirst/aquaskk の e90a04b である。
- NablaSKK 側は次を突き合わせた。
  - 設定アプリ(`macos/Preferences/`)と、設定ファイル(`macos/Shared/InputSettings.swift`)。
  - IME が値を渡す処理(`macos/Sources/SKKRustInputController.swift` の `applyInputSettings`)。
  - FFI(`crates/nablaskk-ffi/include/nablaskk.h` の `SKK_OPTION_*`)。
  - エンジンの設定(`crates/nablaskk-core/src/config.rs`、`backend.rs`)。

### 本家の設定画面と NablaSKK の対応

本家の設定画面には、次のタブがある。入力設定、変換、補完、辞書、拡張設定、互換性、バージョン。

| 本家のタブ | 項目(本家の表示) | キー | 本家の既定 | NablaSKK |
|---|---|---|---|---|
| 入力設定 | キャンセルで送り仮名を削除する | `delete_okuri_when_quit` | on | エンジンと FFI にある。設定アプリに無く、エンジンの既定(on)のまま |
| 入力設定 | 見出し語の再入を送りと見なす | `handle_recursive_entry_as_okuri` | off | 同上(off のまま) |
| 入力設定 | 確定前の変換候補を表示する | `display_shortest_match_of_kana_conversions` | off | 同上(off のまま) |
| 入力設定 | Enter による確定で改行しない | `suppress_newline_on_commit` | on | 同上(on のまま) |
| 入力設定 | 後退で確定する | `inline_backspace_implies_commit` | off | 同上(off のまま) |
| 入力設定 | 単語登録を開始する時に警告音を再生 | `beep_on_registration` | off | 無い。エンジンは `Messenger::beep` を呼ぶが、macOS 版は何もしない |
| 入力設定 | 数値変換を有効にする | `use_numeric_conversion` | **on** | エンジンと FFI にある。macOS 版は値を渡さないので、エンジンの既定の **off** になっている |
| 入力設定 | キーボードレイアウト | `keyboard_layout` | U.S. | 無い |
| 入力設定 | 入力モードアイコンを表示 | `show_input_mode_icon` | on | 無い。モードを切り替えたときにカーソルの近くに出す表示そのものが無い |
| 入力設定 | 入力モードを文書毎に保持(統合時のみ) | `use_individual_input_mode` | off | 無い。アプリを切り替えたときの動きは固定(仕様書の 3 節) |
| 変換 | インライン変換: N 番目の候補まで表示する | `max_count_of_inline_candidates` | 4(0〜100) | エンジンと FFI にある。設定アプリに無く、エンジンの既定(4)のまま |
| 変換 | 候補ウィンドウのフォント | `candidate_window_font_name`、`_size` | Trebuchet MS、18 | 無い。候補ウィンドウが無く、候補はマークテキストに並べる |
| 変換 | 選択ラベル | `candidate_window_labels` | ASDFJKL | 無い。`asdfjkl` に固定(エンジンの `bridge.rs` と `InputMethodLogic.swift`) |
| 変換 | 表示位置: 入力行の上側 | `put_candidate_window_upward` | off | 無い(候補ウィンドウが無い) |
| 変換 | 注釈の表示 | `enable_annotation` | off | ある(「入力」ページ) |
| 補完 | ダイナミック補完、N 個まで表示する | `enable_dynamic_completion`、`dynamic_completion_range` | off、1(1〜100) | ある(「入力」ページのサジェスト。件数は 1〜20、既定 5) |
| 補完 | 一般辞書も検索する | `enable_extended_completion` | on | ある(「入力」ページ) |
| 補完 | 補完フィルター: N 文字以下を無視 | `minimum_completion_length` | 0(0〜15) | エンジン(`Backend::set_minimum_completion_length`)にある。FFI に無い |
| 辞書 | 辞書の一覧 | (`DictionarySet.plist`) | | ある(「辞書」ページ) |
| 辞書 | ユーザー辞書の場所 | `user_dictionary_path` | | 無い。場所は固定。シンボリックリンクにはできる([ADR 0021](0021-keep-symlinked-config-files.md)) |
| 辞書 | skkserv(本家が skkserv のサーバーになる) | `enable_skkserv`、`skkserv_port`、`skkserv_localonly` | off、1178、on | IME には無い。サーバーは別のツールの `skkserv`(仕様書の 8 節)。設定アプリの「skkserv」ページは、外のサーバーに問い合わせるクライアントの設定で、別のもの |
| 拡張設定 | 補助ルールの一覧(システムとユーザーの `.rule`) | `sub_rules`、`sub_keymaps` | | 形を変えてある。入力方式・記号入力・旧かな・句読点を「ローマ字」「入力」ページで選ぶ([ADR 0003](0003-select-azik-or-act-input-style.md))。ユーザーのルールは `kana-rule.conf` |
| 互換性 | アプリごとの空文字挿入・露払後確定・入力ソース同期 | (`BlacklistApps.plist`) | | 設定は無い。アプリごとの対処はコードに持つ(`ClientQuirks.swift`) |
| バージョン | バージョンと著作権 | | | バージョンはサイドバーの下に出す([ADR 0013](0013-show-version-in-preferences.md)) |

### この ADR で足すもの

足すのは、次の 3 つの条件をすべて満たす 8 項目である。

- エンジンに、本家から移植した実装がすでにある。
- ホストが値を渡すだけで効く。
- 本家では設定画面で変えられるのに、NablaSKK では変えられない。

8 項目は次のとおり。

- `delete_okuri_when_quit`
- `handle_recursive_entry_as_okuri`
- `display_shortest_match_of_kana_conversions`
- `suppress_newline_on_commit`
- `inline_backspace_implies_commit`
- `use_numeric_conversion`
- `max_count_of_inline_candidates`
- `minimum_completion_length`

このうち `minimum_completion_length` だけは、FFI のオプションを 1 つ足す必要がある。

**数値変換の既定が本家と違う。** 本家の既定は on である。NablaSKK の macOS 版は値を渡さないので、
エンジンの既定の off になっている。そのため、`#` を使う辞書の見出し語(SKK-JISYO.L の
「#かい /#1回/」など)が引けない。プロジェクトの規則では、設定の既定は本家の UserDefaults.plist の値に
合わせる。ほかの項目は、エンジンの既定が本家の既定と一致している。この ADR で設定を足すときに、
数値変換の既定も本家に合わせて on にする。つまり、**設定を変えていない利用者にとっても、数値変換が
効くようになるという挙動の変更**になる。

ほかの項目を足さない理由は「実装しないこと」に書く。

## 実装すること

### 設定ファイル(`settings.conf`)

`InputSettings` に次のキーを足す。キーの名前は本家のキー名をそのまま使う
(`display_shortest_match_of_kana_conversions` だけは長いので `display_shortest_match` にする)。

| キー | 値 | 既定 | 内容 |
|---|---|---|---|
| `suppress_newline_on_commit` | `on` / `off` | `on` | Enter で確定したとき、Enter をアプリに渡さない(off で egg-like-newline の逆、確定と同時に改行する) |
| `inline_backspace_implies_commit` | `on` / `off` | `off` | ▼ でインラインの候補を選んでいるときの BS で、候補を確定する(off では前の候補に戻る) |
| `delete_okuri_when_quit` | `on` / `off` | `on` | 送りありの ▼ を取り消したとき、送り仮名を消して ▽ に戻る(off では送り仮名を読みに残す) |
| `handle_recursive_entry_as_okuri` | `on` / `off` | `off` | ▽ の読みの入力中に `Q`(日本語入力開始)を押すと、送り仮名を始める(off では読みを確定する) |
| `display_shortest_match` | `on` / `off` | `off` | ローマ字の入力の途中で、最短の一致を表示する(`n` を `ん` と表示する) |
| `use_numeric_conversion` | `on` / `off` | `on` | 数値変換(`#0`〜`#9` を使う辞書の見出し語) |
| `max_count_of_inline_candidates` | 0〜100 | 4 | ▼ でインラインに表示する候補の数。これを超えると一覧に並べる |
| `minimum_completion_length` | 0〜15 | 0 | 読みがこの文字数より短いとき、この文字数以下の見出し語を補完(TAB とサジェスト)の候補から外す |

- 値の読み方は、今のキーと同じにする。
  - 既定が `on` のキー: `off` のときだけ off にする。
  - 既定が `off` のキー: `on` のときだけ on にする。
  - 数: 範囲の外の値は範囲に収める。数でない値は既定値にする。
- `serialize` はすべてのキーを書き、ファイルの先頭の説明(`header`)にも足す。
- 今の `settings.conf` には新しいキーが無い。その場合は、上の既定値になる。

### FFI

- `nablaskk.h` に `SKK_OPTION_MINIMUM_COMPLETION_LENGTH = 12` を足す。`skk_session_set_option` で、
  `Backend::set_minimum_completion_length` を呼ぶ。負の値は 0 にする。
- `swift/SKKSession.swift` の `Option` に `minimumCompletionLength = 12` を足す。

### IME

- 今の `applyInputSettings` は、4 つのオプションを渡している(`SKKRustInputController.swift`)。
  オプションを渡す処理を、テストできる `EngineSetup.applyOptions(to:settings:)`
  (`InputMethodLogic.swift`)に移す。
- `applyOptions` は、今の 4 つと、足した 8 つを渡す。

### 設定アプリ

- サイドバーに「変換」ページを足す。位置は「入力」の次にする。中身は次の 4 つの節である。
  - 確定と取り消し: `suppress_newline_on_commit`、`inline_backspace_implies_commit`、
    `delete_okuri_when_quit`
  - 送り仮名: `handle_recursive_entry_as_okuri`
  - 候補: `use_numeric_conversion`、`max_count_of_inline_candidates`(Stepper)
  - ローマ字の表示: `display_shortest_match`
- それぞれのトグルには、今の「入力」ページと同じく、オンとオフで何が変わるかを 1 文で書く。
- 「入力」ページのサジェストの節に、`minimum_completion_length` の Stepper を足す。
- 「入力」ページと同じく、節の下に「設定は次にどこかのアプリで入力を始めたときに反映されます。」と書く。

### 文書

- `docs/specifications.md` を直す。
  - 5.1 節の表に、足したキーを書く。
  - 6 節の表に「変換」ページを足す。
  - 3 節の「候補は最初の 4 件をインラインで」を、件数は設定で変えられると直す。
- README(日本語と英語)の設定アプリの説明に、「変換」ページを書く。
- 数値変換が既定で on になったことを、次のリリースノートの「変更」に書く。
- `docs/roadmap.md` を直す。
  - 優先度が高い候補「設定アプリでオプションを変えられるようにする」を外す。
  - 「実装しないこと」のうち後で検討するもの(キーボードレイアウト、単語登録の警告音、
    入力モードの表示、選択ラベル)を、「未定」に足す。

## 実装しないこと

- **`beep_on_registration`(単語登録の警告音)。** macOS 版は今 `Messenger::beep` で何もしない。
  鳴らすには、FFI に、エンジンからホストへ知らせるコールバックを足す必要がある。値を渡すだけでは
  済まないので、別の ADR にする。
- **`keyboard_layout`(キーボードレイアウトの指定)。** 本家は `overrideKeyboardWithKeyboardNamed:`
  で、入力に使うキー配列を差し替える。入力ソースの扱いは、[ADR 0024](0024-ascii-input-source.md)
  が実機で失敗している。実機で確かめながら進める必要があるので、別の ADR にする。
- **`show_input_mode_icon`(入力モードのアイコン)。** 設定ではなく、カーソルの近くにモードを出す
  表示そのものを作ることになる。
- **`use_individual_input_mode`(入力モードを文書ごとに保持)。** アプリを切り替えたときの動きは
  決めてある(仕様書の 3 節)。変えるなら、その動きを変える ADR にする。
- **候補ウィンドウのフォントと表示位置。** NablaSKK には候補ウィンドウが無い。候補ウィンドウを
  作るかどうかは、ロードマップの「未定」にある。
- **選択ラベル(`candidate_window_labels`)。** ラベルの数は、1 ページに並べる候補の数も決める。
  エンジンの `CandidateWindow::label_index` と Swift の `candidateLine` の両方を変える必要があり、
  値を渡すだけでは済まない。
- **ユーザー辞書の場所。** シンボリックリンクで別の場所に置ける([ADR 0021](0021-keep-symlinked-config-files.md))。
- **IME が skkserv のサーバーになること。** サーバーは別のツールの `skkserv` にする
  (小さいツールに分ける)。
- **拡張設定の形(`.rule` の一覧)と互換性の設定(アプリごとの対処)。** NablaSKK では、形を変えて
  実装してある(上の表)。
- **サジェストの件数の範囲と既定(本家は 1〜100 で既定 1、NablaSKK は 1〜20 で既定 5)。** 調べて
  分かった違いだが、この ADR では変えない。
- **`fix_intermediate_conversion`、`enable_private_mode`。** 本家の設定画面に無い。
- **wasm 版の設定。** wasm 版は本家の既定値で動く(仕様書の 12 節)。

## テスト設計

先に書いて、失敗することを確かめてから実装する。

- `crates/nablaskk-ffi` のテスト:
  - `SKK_OPTION_MINIMUM_COMPLETION_LENGTH` で長さを 2 にする。ユーザー辞書に「か」「かん」「かんじ」が
    あり、読みが「か」のとき、補完の候補が「かんじ」だけになること。
  - 0 では、「かん」と「かんじ」の両方になること。
  - 負の値は 0 と同じになること。
- `macos/Tests/main.swift`(`sh macos/run-tests.sh`):
  - `InputSettings` の既定値が上の表のとおりであること。新しいキーの無い今の `settings.conf` を
    読んだときも、同じになること。
  - 足したキーの `parse` と `serialize` を往復すると、値が保たれること。
  - 範囲の外の数(`max_count_of_inline_candidates=101`、`-1`、`minimum_completion_length=16`)は
    範囲に収まり、数でない値は既定値になること。
  - `EngineSetup.applyOptions` を本物の `SKKSession` に使い、次を確かめる。
    - 既定では数値変換が効くこと。「#かい /#1回/」の辞書で、▽ `1かい` を変換すると「1回」になること。
      `use_numeric_conversion=off` では、変換できないこと。
    - `suppress_newline_on_commit=off` では、▼ で Enter を押すと確定して、Enter がアプリに渡ること
      (キーを処理しなかったと返すこと)。
    - `max_count_of_inline_candidates=1` では、2 回目のスペースで一覧になること。
- `macos/Tests/PreferencesUITests/main.swift`(`sh macos/run-ui-tests.sh`):
  - サイドバーに「変換」が出て、選ぶと「数値変換」のトグルがオンで表示されること。
  - そのトグルを切ると、`settings.conf` に `use_numeric_conversion=off` が書かれること。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が次を確かめる。
  - 「変換」ページで `suppress_newline_on_commit` を切ると、チャットの入力欄で ▼ を Enter で確定したとき、
    確定と同時に送信(改行)されること。戻すと送信されないこと。
  - 数値変換が既定で効くこと(SKK-JISYO.L で「1かい」→「1回」)。

## トレードオフ

- **数値変換を既定で on にすると、設定を変えていない利用者の変換結果が変わる。** `#` のある見出し語
  が候補に出るようになる。本家の既定に合わせることを優先する。不要なら「変換」ページで切れる。
- **キーの名前を本家のキー名にした。** 今のキー(`suggest`、`annotation` など)は短い名前なので、
  ファイルの中で名前の付け方が揃わない。代わりに、本家の文書や Emacs の SKK の変数
  (`skk-egg-like-newline` など)と対応が取りやすい。
- **ページを 1 つ足した。** 今の「入力」ページに足すと、縦に長くなる。今の Form はスクロールしないので、
  ウィンドウの高さ(最小 440)に収まらなくなる。サイドバーのページは 7 つになる。
- **8 項目に絞った。** 本家の設定画面の項目のうち、残りは別の ADR か「実装しない」になる。本家と同じ
  設定画面にはならない。

## 参考文献

- AquaSKK(codefirst/aquaskk、e90a04b) https://github.com/codefirst/aquaskk
  - `platform/mac/proj/Preferences.xib`
  - `platform/mac/plist/UserDefaults.plist`、`platform/mac/plist/BlacklistApps.plist`
  - `platform/mac/src/preferences/PreferenceController.mm`
  - `platform/mac/src/server/SKKServer.mm`、`MacCandidateWindow.mm`、`MacInputModeWindow.mm`、
    `MacMessenger.mm`、`SKKInputController.mm`
- `docs/roadmap.md` の「設定アプリでオプションを変えられるようにする」
