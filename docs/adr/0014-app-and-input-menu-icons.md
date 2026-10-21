# ADR 0014: 設定アプリと入力メニューにアイコンを付ける

ステータス: 採択→アイコンを描く道具(qlmanage)は[ADR 0015](0015-render-icons-with-sips.md)

## 文脈

設定アプリ(NablaSKK Preferences)にも IME 本体(NablaSKK.app)にもアイコンの指定がない。
そのため、Dock、Finder、入力メニュー(メニューバー)では汎用のアイコンになる。
`docs/roadmap.md` の候補「設定アプリと入力メニューにアイコンを付ける」に当たる。

作業者と、∇ を軸にした案を 10 通り(A〜J)と、H 案の背景を暗くした版を 3 通り比べて
H2(紺の地に金色の ∇)を選び、紺の明るさを 3 段階比べて **H2a** に決めた。

- 形: 数学の本の活字のような、太細のある ∇(左の辺が太く、内側を細く抜いた形)。
  NablaSKK の名前の ∇ であり、SKK で読みの入力中を表す ▽ とも同じ形になる。
- 設定アプリ: 紺のグラデーション(上 `#2A3A5E`、下 `#1A253F`、縁 `#121A2E`)の
  角丸の四角に、金色(`#E8C77A`)の ∇。
- 入力メニュー: 同じ ∇ の単色(テンプレート画像)。メニューバーの明暗に合わせて、
  macOS が色を付ける。

macOS 付属の入力メソッドは、入力メニューのアイコンを `Info.plist` の
`tsInputMethodIconFileKey`(TIFF)で指定し、`TISIconIsTemplate` を true にしている
(`/System/Library/Input Methods/` の各 `Info.plist` で確かめた)。

入力メニューのアイコンをいくつ持つかは、入力ソースをいくつ登録するかで決まる。本家
AquaSKK は、入力ソースを 6 つ登録している(`platform/mac/proj/AquaSKK-Info.plist`)。

- 「AquaSKK 統合」: 1 つの入力ソースで、SKK のモードは IME の中で切り替える。
- 「ひらかな」「カタカナ」「半角カナ」「全角英数」「ASCII」: 入力モードごとの入力ソース。
  それぞれにメニューのアイコンがあり、JIS キーボードの かな/英数 キーで選ばれるための
  番号(`tsInputModeJISKeyboardShortcutKey`)も付いている。本家は、選ばれた入力ソースと
  エンジンのモードを双方向に同期している(`SKKInputController.mm` の `setValue:forTag:`、
  `syncInputSource`)。

本家の Wiki の説明(検索結果の要約で読んだもの。元のページは開けなかった)によると、
入力モードごとの入力ソースは、古い仕組み(TSM)で作っていたころの形を「TSM 版互換の
入力メニュー」として残したもので、InputMethodKit 版で「統合入力メニュー」を足した。
統合は、英数キーや Cmd+Space でほかの入力ソースにすぐ切り替えられ、文書ごとに入力モードを
保てる。その代わり、今の入力モードがメニューバーのアイコンでは分からない。入力モードごとに
入力ソースを登録するのは InputMethodKit の制約ではなく、macOS の入力ソースの作法で、IME が
選べる。

NablaSKK は今、統合と同じ 1 つの入力ソースだけを登録している。作業者は本家でも統合だけを
使っている。そこで、NablaSKK の macOS 版は統合だけをサポートすると決める。エンジン
(`nablaskk-core`)の入力モードの扱いは、macOS 以外のホストからも使えるように残す
(ホストとの境界はトレイトで、macOS に依存しない)。

アイコンの絵柄は、作業者と Claude がこのリポジトリのために作ったもので、本家 AquaSKK や
ほかの素材を元にしていない。ほかのファイルと同じ GPL-2.0-or-later で配布する。

## 実装すること

- 元の絵を SVG で置く。文字は使わず、図形だけで描く(フォントに左右されない)。
  - `macos/icons/AppIcon.svg`: 設定アプリのアイコン(1024×1024)。
  - `macos/icons/MenuIcon.svg`: 入力メニューのアイコン(16×16、黒と透明だけ)。
- SVG から画像を作るスクリプト `macos/make-icons.sh` を置く。macOS に付いている道具
  だけを使う(`qlmanage` で PNG にし、`sips` で縮め、`iconutil` で `.icns` に、
  `tiffutil` で 1 倍と 2 倍をまとめた TIFF にする)。
  - `macos/icons/AppIcon.icns`: 16〜1024px(1 倍と 2 倍)。
  - `macos/icons/MenuIcon.tiff`: 16px と 32px(2 倍)。
- 作った `AppIcon.icns` と `MenuIcon.tiff` もリポジトリに入れる。ビルドのたびに作らない
  ので、CI やビルドの手順は変わらない。絵を変えたときだけ、スクリプトを実行し直して
  コミットする。
- `macos/build-app.sh` で同梱する。
  - 設定アプリ: `Resources/AppIcon.icns`。`Info.plist` に `CFBundleIconFile` =
    `AppIcon`。
  - IME 本体: `Resources/AppIcon.icns`(Finder 用)と `Resources/MenuIcon.tiff`。
    `Info.plist` に `CFBundleIconFile` = `AppIcon`、`tsInputMethodIconFileKey` =
    `MenuIcon.tiff`、`TISIconIsTemplate` = true。
- `macos/check-bundle.sh` で、上のファイルが同梱され、`Info.plist` のキーが入っている
  ことを確かめる。
- 仕様書と README(日本語と英語)に、アイコンのことと、macOS 版の入力ソースは統合に
  当たる 1 つだけで、本家の入力モードごとの入力ソース(互換入力メニュー)はサポートしない
  ことを書く。
- `docs/roadmap.md` から、この候補「設定アプリと入力メニューにアイコンを付ける」と、
  サポートしないと決めた候補「入力モードごとに入力ソースを登録する」を外す。

## 実装しないこと

- 入力モードごとに入力ソースを登録すること(本家の「ひらかな」「カタカナ」「半角カナ」
  「全角英数」「ASCII」、互換入力メニュー)。NablaSKK の macOS 版は、統合に当たる 1 つの
  入力ソースだけをサポートする。そのため、入力モードごとのアイコンも作らない。
- エンジンの入力モードの扱いを消すこと。今のモードを読む `Session::input_mode`、
  外からモードを設定する `skk_session_set_input_mode`(FFI)などは、macOS 以外のホストが
  入力モードごとの入力ソースを持つ場合にも使えるよう、そのまま残す。
- 補完や注釈のウィンドウなど、ほかの画面のアイコン。
- ビルドのたびに SVG から画像を作ること。
- ダークモード用の別の設定アプリのアイコン(macOS の通常のアプリアイコンと同じく 1 種類)。

## テスト設計

- `macos/check-bundle.sh`:
  - 作ったものが通ること。
  - `AppIcon.icns` を消した複製、`MenuIcon.tiff` を消した複製、`Info.plist` から
    `tsInputMethodIconFileKey` を消した複製で、それぞれ失敗すること。
- `macos/make-icons.sh`:
  - 実行すると、`AppIcon.icns` に 16、32、128、256、512px(それぞれ 1 倍と 2 倍)の画像が
    入っていること(`iconutil -c iconset` で戻して `sips` で大きさを確かめる)。
  - `MenuIcon.tiff` に 16px と 32px の 2 枚が入っていること(`sips` で確かめる)。
  - `MenuIcon.tiff` が黒と透明だけでできていること(テンプレート画像として色が付く)。
- 既存の検証(`cargo test`、`cargo clippy`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が次を確かめる。
  - 設定アプリを開くと、Dock に H2 のアイコンが出ること。Finder でも同じであること。
  - 入力メニューに ∇ が出て、ライトとダークのメニューバーの両方で見えること
    (テンプレート画像として色が変わること)。
  - 入力ソースの一覧(システム設定のキーボード)でも ∇ が出ること。

## トレードオフ

- 作った画像(バイナリ)をリポジトリに入れるので、絵を変えるたびに差分が大きくなる。
  ビルドのたびに作れば入れずに済むが、`qlmanage` の描き方が macOS の版で変わると、
  同じ SVG から違う画像ができるおそれがある。入れておけば、配布物の見た目が固定される。
- SVG から PNG にする道具に、macOS 付属の `qlmanage` を使う。外部の道具(rsvg-convert
  など)を入れずに済むが、`qlmanage` はもともとプレビュー用で、描き方の細かい違いを
  選べない。
- 16px の入力メニューのアイコンは、∇ の内側の抜きが細くなる。比較のページで見て、
  形が分かることを作業者と確かめた。

## 参考文献

- AquaSKK `platform/mac/proj/AquaSKK-Info.plist`、`platform/mac/proj/InfoPlist.strings`、
  `platform/mac/src/server/SKKInputController.mm`、`platform/mac/src/server/SKKInputMenu.mm`
  https://github.com/codefirst/aquaskk/tree/master/platform/mac
- 入力メニューの設定 - AquaSKK Wiki - OSDN(検索結果の要約で読んだ。元のページは開けなかった)
  https://ja.osdn.net/projects/aquaskk/wiki/%E5%85%A5%E5%8A%9B%E3%83%A1%E3%83%8B%E3%83%A5%E3%83%BC%E3%81%AE%E8%A8%AD%E5%AE%9A
- みずぴー日記「入力モード」(`ComponentInputModeDict` と `tsInputModeJISKeyboardShortcutKey`)
  https://mzp.hatenablog.com/entry/2017/10/26/084053

- Apple Developer Documentation, CFBundleIconFile
  https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleiconfile
- macOS 付属の入力メソッドの `Info.plist`(`/System/Library/Input Methods/*.app`)の
  `tsInputMethodIconFileKey` と `TISIconIsTemplate`
- `man iconutil`、`man tiffutil`、`man sips`、`man qlmanage`
