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
  `menu()`(入力ソースメニューの「辞書を管理...」)。
- SwiftUI の `App`/`WindowGroup` ライフサイクルは、swiftc 直ビルドでは
  ウィンドウが背面に回ったあと Dock クリックで出てこなかった → AppKit の
  `NSApplicationDelegate` でウィンドウを 1 つ保持し、
  `applicationShouldHandleReopen` と `applicationDidBecomeActive` の両方で
  前面に出す構成に変更。閉じると終了、多重起動禁止、編集メニューは手組み。
  `windowResizability` は macOS 13 以降のため使わない(対象は 12 以降)。
- `dictionaries.conf` の無効エントリは行頭 `-`(旧パーサでも読み飛ばされる)。
  GUI から追加した辞書は `~/Library/Application Support/NablaSKK/dictionaries/`
  にコピーして、そのパスを設定に書く(同名・同内容は再利用、同名・別内容は
  `-1` の連番)。変更は即保存し、IME は `activateServer` 時に mtime で再読み込み。
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
