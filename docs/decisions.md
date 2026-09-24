# 決定事項と却下したアプローチ

「なぜそうなっているか」を残すためのファイル。時系列は `git log` を見る。
各項目は「試した → 失敗した理由 → 採用した案」の形で書く。

## エンジン(Rust)

### 外部依存ゼロ
- 当初 EUC-JP 変換に `encoding_rs` を使った → 作業者から「zero dependencies」
  の指示 → 本家 `jconv_eucj2ucs-inl.h` の対応表からテーブル
  (`crates/nablaskk-core/src/jconv/eucjp_tables.rs`)を生成して自前実装。
  生成スクリプトはセッションのスクラッチ領域にあったものでリポジトリには
  ない。再生成が必要なら本家の inl ファイルからコメントを除去し、
  plane1/plane2 表を (euc, ucs) のソート済み配列に落とす。
- ローカル時刻(Gadget 辞書の today/now)、raw モード(skk-cli の termios)は
  `extern "C"` で libc を直接宣言。

### 本家との忠実さ
- 状態機械(`machine.rs`)は本家 GenericStateMachine のアルゴリズムを
  そのまま移植。メソッドポインタの代わりに `StateId` enum。Forward /
  DeepHistory / ShallowHistory の意味を変えると受け入れテストが崩れる。
- `RomanKanaConverter::convert` は本家どおり「次状態(`tt`→っ の `t`)を
  呼び出し側(InputQueue)が戻す」設計。converter 単体に "katta" を渡しても
  かった にはならない(最初のテストで勘違いした)。
- `Config::default()` は最初に推測で埋めたため
  `suppress_newline_on_commit=false` になっていた → 変換中の Enter が
  Electron アプリに貫通してメッセージ送信された → 本家のインストール済み
  `UserDefaults.plist` と全項目を突き合わせて修正
  (suppress_newline_on_commit=true, delete_okuri_when_quit=true,
  dynamic_completion_range=1, max_count_of_inline_candidates=4)。
- 受け入れテストのユーザー辞書は UTF-8 前提(本家 MockConfig の値も
  そのまま使う: inline 5, suppress newline true, delete okuri when quit true)。

### 辞書
- 辞書エンコーディングは `Encoding::Auto`(先頭行の `coding:` クッキー →
  内容が valid UTF-8 かで判定)。辞書タイプ 0 と自動更新辞書は Auto、
  タイプ 5 は UTF-8 固定のまま(番号は本家 DictionarySet 互換)。
- ユーザー辞書は Auto で開くと既存ファイルのエンコーディングを保存時も維持。
  新規作成は UTF-8。
- ユーザー辞書の外部編集(設定アプリ)との整合: `reload_if_changed()` を
  IME の `activateServer` で呼ぶ。**未保存の学習がある場合はそちらを保存して
  外部編集を上書きする**(学習を失わない方を優先)。マージはしない。
- 辞書の非同期ロード(本家の pthread タイマー)は同期ロード +
  `reload_if_updated()` に置き換え。シングルトン(SKKBackEnd 等)は廃止。

### 本家で見つけた不具合と扱い
- `eucj_to_utf8::emit()` の合成文字マスク `0xfff`(正しくは `0xffff`):
  Rust 版は修正済み。本家向けパッチを `upstream-patches/` に置いたが
  **上流には未送付**。
- 本家 keymap.conf は `0x7f`(macOS の ⌫)を SKK_DELETE に割り当てており、
  SKK_DELETE は行末では no-op。それでも本家で ⌫ が効く理由は未解明(推測:
  何か見落としがある)。NablaSKK の mac 層では keyCode で解決
  (51→0x08 backspace、117→0x7f forward delete)。パッチは作っていない。
- Gadget 辞書の `jdate:`(元号変換)は本家では空実装 → 実装した(令和〜明治、
  改元年は両元号を併記)。

## macOS 層(Swift)

### キー処理
- Chromium 系(VSCode など Electron)は、IME が「処理済み」を返しても
  マークテキストがハンドラの前後に存在しないと生のキーをページに流す
  (Chromium の `render_widget_host_view_cocoa.mm` を読んで確認)。
  Ghostty は IME の戻り値を見ずに、テキストが出なかったキーを自前で
  端末に送る(`SurfaceView_AppKit.swift` を読んで確認)。
- 本家 AquaSKK の回避策(`cancelKeyEventForASCII`: 0x0c をマークして
  即座に空にする)は、ハンドラを抜ける時点でマークが空なので効かない →
  採用しなかった。代わりにゼロ幅スペースをマークテキストとして残したまま
  ハンドラを抜け、`DispatchQueue.main.async` で空マークに戻す
  (`ClientQuirks` + `maskKeyEvent`)。Chromium は Ctrl 系のみ、Ghostty は
  「何も出さずに消費したキー」すべてが対象。ネイティブアプリには適用しない。
- VSCode ターミナルの Ctrl-J: 第1案(上の Chromium マスクだけ)は直らなかった。
  再調査で「エンジンに `0x0a`+ctrl や ctrl フラグ欠落の形で届くと未定義キー
  として素通しされ、モードも変わらない」ことを FFI 経由で再現 →
  `KeyTranslator` で制御文字 0x01–0x1a を「文字 + ctrl」に正規化
  (Return/Tab/Esc/Enter/Delete は keyCode で除外)。
- Cmd 系ショートカットはアプリに渡す。例外は変換中の Cmd-V(ペースト)。
  `SKK_PASTE` に `meta::v` を追加、クリップボードは Swift 側から
  `skk_session_set_clipboard` で渡す(改行は除去)。
- 変換中の Enter は IME が消費(本家デフォルトと同じ)。egg-like-newline は
  `skk_session_set_option` でオプトイン。

### 設定アプリ
- 別アプリ(`NablaSKK Preferences.app`)を IME バンドルの Resources に同梱。
  1 回のコピーで両方インストールされるようにするため。起動は IMK の
  `menu()`(入力ソースメニューの「NablaSKK の設定...」)。
- SwiftUI の `App`/`WindowGroup` ライフサイクルは、swiftc 直ビルドでは
  ウィンドウが背面に回ったあと Dock クリックで出てこなかった → AppKit の
  `NSApplicationDelegate` でウィンドウを 1 つ保持し、
  `applicationShouldHandleReopen` と `applicationDidBecomeActive` の両方で
  前面に出す構成に変更。閉じると終了、多重起動禁止、編集メニューは手組み。
  `windowResizability` は macOS 13 以降のため使わない(対象は 12 以降)。
- `dictionaries.conf` の無効エントリは行頭 `-`(旧パーサでも読み飛ばされる)。
  GUI から追加した辞書は `~/Library/Application Support/NablaSKK/dictionaries/`
  にコピーして、そのパスを設定に書く。同名ファイルは上書き(辞書の更新版を
  ドロップし直す用途を優先。当初は `-1` の連番で並存させていた)し、同じ
  パスを指すエントリが既にあれば再有効化するだけで二重登録しない。
  変更は即保存し、IME は `activateServer` 時に mtime で再読み込み。
- skkserv は本家では辞書リストの 1 タイプ(2、EUC-JP 固定)だが、設定アプリに
  専用の「skkserv」タブ(有効/ホスト/ポート/文字コードのラジオ)を置いた。
  値は `settings.conf`(`skkserv=on|off`, `skkserv_host`, `skkserv_port`,
  `skkserv_encoding=euc-jp|utf-8`)に保存し、IME は dictionaries.conf の
  全エントリを追加した**あと**に proxy 辞書を追加する(ローカル辞書優先)。
  UTF-8 サーバー(yaskkserv2 の `--utf8` など)向けに辞書タイプ 6
  (skkserv UTF-8)を NablaSKK 独自に追加。`ProxyDictionary::with_encoding`
  で要求・応答の変換を切り替える。辞書リストの「+」からは skkserv タイプを
  外した(手書きの 2/6 行は従来どおり表示・編集できる)。
- エンジンのセッションは全クライアントで 1 つを共有し、別クライアントが
  アクティブになると `clear()` でひらがなに戻る(本家はクライアントごとに
  セッションを持つのでモードも別々)。Ghostty のウィンドウ切り替えで英数が
  ひらがなに変わるのを避けるため、アプリ(bundle identifier)ごとに最後の
  モードを覚え、ASCII だった場合だけ `activateServer` で ASCII に戻す。
  記録はキー処理のたびに行う。`deactivateServer` で記録すると、次の
  クライアントの `activateServer` が先に走ってセッションが初期化された後の
  ひらがなを記録してしまうことがあった(初版はこれで直らなかった)。
  ウィンドウ単位にしたかったが、IMK はアクティブ化のたびに新しい
  コントローラと新しい `uniqueClientIdentifierString` を渡してくる(同じ
  ウィンドウでも毎回違う UUID)ので、切り替えをまたいで残る鍵はアプリ単位
  しかなかった(2 版目はこれで直らなかった)。
  FFI に `skk_session_set_input_mode`(エンジンの AsciiMode などのモード
  イベントを送る)を追加。
- サジェスト(本家の動的補完 `enable_dynamic_completion`)を実装。エンジンは
  既に `DynamicCompletor` を呼んでいたので FFI に `SharedCompletor` を足して
  補完一覧・共通接頭辞長を公開し(`skk_session_completion_*`)、IME 側の
  `CompletionWindow.swift` が本家 `CompletionWindow`/`CompletionView`/
  `MacDynamicCompletor` の見た目(淡色の箱、未入力部分を太字、"TAB で補完" の
  板)でキャレット下に出す。オプションは `settings.conf` の `suggest=on|off`
  (既定 off、本家と同じ)、`suggest_count`(既定 5。本家の
  `dynamic_completion_range` 既定 1 は 1 件しか出ず一覧の意味がないので変えた)、
  `completion_extended=on|off`(本家 `enable_extended_completion`、既定 on は
  本家 plist と同じ。Rust `Backend` の既定は false のままで、IME が起動時に
  オプションで on にする。受け入れテストが user 辞書のみの補完を前提に
  しているため Backend 側は変えない)。
- キーバインドは本家では手書きの keymap.conf パッチのみだが、設定アプリに
  「キー」タブを置いた。行ごとに keymap.conf 構文でキーを書き、差分だけを
  `~/Library/Application Support/NablaSKK/keymap.conf` に保存する。エンジン側は
  `Keymap::load_replacing`(NablaSKK 独自)を追加し、1 行ごとに「そのシンボルに
  束縛されたキーを全部外してから登録」する。本家の `load` はマージのみなので
  `SKK_JMODE ctrl::k` と書いても ctrl::j が残ってしまうため。FFI は
  `skk_session_reset_keymap` / `skk_session_override_keymap`。IME は mtime を
  見て「組み込み keymap に戻す → 上書き行を適用」。SKK_PASTE は IME 側が
  エンジンより先にキーを判定している(クリップボード受け渡し)ため対象外。
  UpperCases / Direct / InputChars / AlwaysHandled / PseudoHandled も GUI から
  は触らせない(手書きの keymap.conf 行としては解釈しないので、必要なら
  data/keymap.conf を直接変える)。
- 句読点の切り替えは本家と同じく kana-rule のサブルール(`data/comma.rule`,
  `data/period.rule` の中身を Swift 側に埋め込み)で実現。エンジンには
  `reset_kana_rules` / `patch_kana_rules` の FFI を追加し、IME は
  `settings.conf`(key=value)の mtime を見て「組み込みルールに戻す → 有効な
  サブルールを重ねる」を毎回やり直す(差分適用はしない)。読点と句点は
  独立したオプション。
- 候補ウィンドウは IMKCandidates ではなくマークテキスト内にインライン描画
  (`【a:候補 s:候補 …】`)。ラベル選択はエンジン側の `selector.select` が
  処理する。独立ウィンドウ化は未着手。
- 入力ソースは 1 つだけ登録(ComponentInputModeDict なし)。モード切替は
  SKK 内部で行う。作業者の過去作 rskk-mac の plist と同じ最小構成。

### 配布
- Developer ID 署名・公証は有料のため見送り。curl 経由のダウンロードには
  検疫属性が付かないことを実測で確認し、`install.sh`(rustup 方式)を
  正式な導入経路にした。CI には署名・公証ステップを用意済みで、
  リポジトリシークレットを設定すると有効になる(未設定ならad-hoc)。
- ビルドは swiftc 直呼び(Xcode プロジェクトなし)。ユニバーサルは
  `UNIVERSAL=1`(FFI staticlib を両ターゲットでビルド → lipo、swiftc を
  `-target` 別に 2 回 → lipo)。

## 改名
- `aquaskk-rust` → NablaSKK。ライセンス、帰属ヘッダ、本家への言及、本家の
  実在ディレクトリを指すコード(`AquaSKK/SKK-JISYO.L` の再利用パス、変数名
  `aquaskkDictionary`)、`data/` 以下、`.gitignore` の `/aquaskk-reference`
  は意図的に残している。SumiSKK という名前はこのリポジトリの履歴には
  出てこない(このセッションの改名は aquaskk-rust からの 1 回だけ)。
