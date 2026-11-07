# ADR 0024: 英数の入力ソースを足して、ほかの IME を外せるようにする

ステータス: 破棄

## 文脈

macOS では、ASCII を入力できる入力ソースを少なくとも 1 つ有効にしておく必要がある。
NablaSKK は入力ソースを 1 つしか登録しておらず
([ADR 0014](0014-app-and-input-menu-icons.md))、その入力ソースは日本語
(`TISIntendedLanguage` が `ja`)なので、この条件を満たさない。

作業者の環境(`defaults read com.apple.HIToolbox AppleEnabledInputSources` で確かめた)で
有効になっている入力ソースは次のとおり。

- NablaSKK(`jp.tett23.inputmethod.nablaskk`、Keyboard Input Method)
- macOS 付属の日本語入力(Kotoeri)の、ローマ字入力とかな入力。それぞれ日本語、英字
  (`com.apple.inputmethod.Roman`)、カタカナのモードを持つ。
- 50 音パレットと文字ビューア(キーボード入力ではない)

ABC は有効になっておらず、ASCII の条件は Kotoeri の「英字」で満たしている。作業者は
NablaSKK しか使っていないが、Kotoeri を外すと ASCII を入力できる入力ソースが無くなって
しまう。そのため Kotoeri を外せず、入力ソースの切り替えで Kotoeri に入ってしまう。

本家 AquaSKK は入力ソースを 6 つ登録している(`platform/mac/proj/AquaSKK-Info.plist`)。
そのうち「ASCII」は、入力モードの ID が `com.apple.inputmethod.Roman` で、
`TISIntendedLanguage` が `en`、`tsInputModeScriptKey` が `smRoman` である。macOS 付属の
日本語入力の「英字」も、入力モードの ID が `com.apple.inputmethod.Roman` である(上の
`defaults` の出力)。この形の入力モードを持つと、IME だけで ASCII の条件を満たせるはずである。
ただし、NablaSKK で確かめてはいないので、実機で確かめる(テスト設計)。

[ADR 0014](0014-app-and-input-menu-icons.md) は「入力モードごとに入力ソースを登録すること」を実装しないことにした。この ADR は、
その一部を変え、英数の入力ソースを 1 つだけ足す。ひらがな、カタカナ、半角カナ、全角英数の
入力ソースは足さない。統合の入力ソースの中でモードを切り替える使い方は今のまま変えない。

### 英数の入力ソースの動き

英数の入力ソースでの動きは、次の 2 案から選んだ。

1. **キーをすべてアプリに渡す(ABC と同じ)。** エンジンを通さない。統合の入力ソースの
   エンジンのモードとは結びつけない。
2. 本家と同じく、入力ソースとエンジンのモードを双方向に同期する。英数の入力ソースを選ぶと
   エンジンを英数モードにする。エンジンで Ctrl-J を押してかなモードにすると、入力ソースを
   統合に切り替える(`selectInputMode`)。

1 を選ぶ。

- 作業者に要るのは「ASCII の入力ソースがあること」で、英数の入力ソースを普段から使うこと
  ではない。英数を打つのは、今までどおり統合の入力ソースの中の英数モード(`l`、
  `SwitchToAsciiIfIdle`)で足りる。
- 2 は、入力ソースの選択とエンジンのモードという 2 つの状態を結びつける。本家は
  `selectInputMode` と `setValue:forTag:` が互いを呼んで無限に繰り返すのを、フラグで
  止めている(`SKKInputController.mm` と `SKKInputMenu.mm` のコメント)。1 なら、このような
  結びつきが生じない。
- macOS は、パスワードの欄などで ASCII の入力ソースに切り替えることがある。ABC と同じ動きなら、
  そこで SKK のキー操作が働いて困ることがない。

## 実装すること

- `macos/Info.plist` に `ComponentInputModeDict` を足し、入力モードを 2 つ登録する。
  - `com.apple.inputmethod.Japanese`(統合): `TISInputSourceID` は今と同じ
    `jp.tett23.inputmethod.nablaskk`。`TISIntendedLanguage` は `ja`、
    `tsInputModeScriptKey` は `smJapanese`、`tsInputModePrimaryInScriptKey` は true。
    メニューのアイコンは今の `MenuIcon.tiff`。
  - `com.apple.inputmethod.Roman`(英数): `TISInputSourceID` は
    `jp.tett23.inputmethod.nablaskk.Ascii`。`TISIntendedLanguage` は `en`、
    `tsInputModeScriptKey` は `smRoman`、`tsInputModePrimaryInScriptKey` は true。
    メニューのアイコンは新しい `MenuIconAscii.tiff`。
  - どちらも `tsInputModeIsVisibleKey` と `tsInputModeDefaultStateKey` を true にする
    (NablaSKK を足すと両方が有効になる)。
  - `tsInputModeJISKeyboardShortcutKey` は付けない。JIS キーボードの 英数/かな キー
    (keycode 0x66、0x68)は今までどおりエンジンが受ける(`AlwaysHandled`)。
  - `tsVisibleInputModeOrderedArrayKey` は 統合、英数 の順。
- 入力ソースの名前を `macos/Resources/ja.lproj/InfoPlist.strings` と
  `en.lproj/InfoPlist.strings` に書く(本家と同じく、キーは入力モードの ID)。
  - 統合: 「NablaSKK」(英語も同じ)。
  - 英数: 「NablaSKK 英数」(英語は「NablaSKK ASCII」)。
  - `macos/build-app.sh` で `Contents/Resources/` に入れる。
- 英数のメニューのアイコンを描く。∇ の右下に小さく「A」を重ねた形を、文字を使わず図形だけで
  描く(`macos/icons/MenuIconAscii.svg`、16×16、黒と透明だけ、[ADR 0014](0014-app-and-input-menu-icons.md)
  と同じ決まり)。画像は `macos/make-icons.sh` で作る([ADR 0015](0015-render-icons-with-sips.md)
  の手順)。
- `SKKRustInputController` に `setValue(_:forTag:client:)` を足す。
  - `kTextServiceInputModePropertyTag` で渡された入力モードの ID を、今の入力ソースとして
    覚える。覚える場所はコントローラーをまたいだ 1 か所(`static`)にする。入力ソースの
    選択は IME 全体で 1 つだからである。
  - 統合から英数に変わったときは、`deactivateServer` と同じく、入力中の文字を確定して
    挿入し、マークテキストを消し、補完と注釈のウィンドウを閉じる。エンジンの入力モードは
    変えない。
  - `DebugLog` に、受け取った ID と、入力ソースが変わったかを書く。
- `handle(_:client:)` は、英数の入力ソースではすぐに false を返す(キーをアプリに渡す)。
  `ClientModeMemory` への記録もしない。
- 入力モードの ID から入力ソースを決める純粋な関数を `macos/Sources/InputMethodLogic.swift`
  に置く(`InputSourceMode.from(modeID:)`)。`com.apple.inputmethod.Roman` なら英数、それ以外
  (統合の ID、知らない ID、nil)なら統合。知らない ID を統合にするのは、今までの動き
  (常にエンジンを通す)を保つためである。
- `macos/check-bundle.sh` で、`ComponentInputModeDict` の 2 つの入力モードと、その
  `TISInputSourceID`、`TISIntendedLanguage`、`tsInputModeScriptKey`、アイコンと
  `InfoPlist.strings` があることを確かめる。
- [ADR 0014](0014-app-and-input-menu-icons.md) のステータスの行を「採択→アイコンを描く道具(qlmanage)は [ADR 0015](0015-render-icons-with-sips.md)、入力ソースを
  1 つだけにすることは ADR 0024」に変える。
- `docs/specifications.md` の 3 章と、README(日本語と英語)の入れ方と制限の節を直す。
  - 入力ソースは統合と英数の 2 つで、英数は ABC と同じくキーをすべてアプリに渡すこと。
  - 英数があるので、ABC やほかの IME を外せること。
  - 前の版から上げたときは、入力ソースの一覧で「NablaSKK 英数」が有効か確かめること
    (足されていなければ「+」から足す)。

## 実装しないこと

- ひらがな、カタカナ、半角カナ、全角英数の入力ソース。統合の中のモード切り替えで足りる。
- 入力ソースとエンジンのモードの同期(案 2)。英数の入力ソースを選んでもエンジンは英数モードに
  ならず、統合の中で `l` を押しても英数の入力ソースには切り替わらない。
- 英数の入力ソースでの SKK のキー操作(Ctrl-J でかなモードに戻ることなど)。統合に戻るには
  macOS の入力ソースの切り替え(Ctrl+Space、メニューバーなど)を使う。
- JIS キーボードの 英数/かな キーで入力ソースを選ぶこと(`tsInputModeJISKeyboardShortcutKey`)。
- キーボード配列の指定(本家の `keyboard_layout` と `overrideKeyboardWithKeyboardNamed`)。
  ABC を外しても今の配列でキーが届くかは、実機で確かめる(テスト設計)。届かなければ、別の
  ADR で扱う。
- 作業者の環境から Kotoeri を外すこと。実機で確かめたあと、作業者が自分で外す。

## テスト設計

テストを先に書いて失敗させてから実装する。

- `macos/run-tests.sh`(`InputSourceMode.from(modeID:)`):
  - `com.apple.inputmethod.Roman` で英数になること。
  - `com.apple.inputmethod.Japanese`、`jp.tett23.inputmethod.nablaskk`、空文字列、知らない ID、
    nil で統合になること。
  - 英数のときはキーを処理しない(アプリに渡す)と判断し、統合のときは処理すると判断すること。
  - 統合から英数に変わったときだけ「入力中の文字を確定する」と判断し、英数から統合、統合から
    統合、英数から英数では判断しないこと。
- `macos/check-bundle.sh`:
  - 作ったものが通ること。
  - `Info.plist` から `ComponentInputModeDict` を消した複製、英数の入力モードの
    `TISIntendedLanguage` を `ja` にした複製、`MenuIconAscii.tiff` を消した複製、
    `InfoPlist.strings` を消した複製で、それぞれ失敗すること。
- `macos/make-icons.sh`: `MenuIconAscii.tiff` に 16px と 32px の 2 枚が入っていて、黒と透明
  だけでできていること([ADR 0014](0014-app-and-input-menu-icons.md) と同じ確かめ方)。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)がすべて通ること。
- 実機で、作業者が次を確かめる(IME の差し替えは作業者がする)。
  1. 入力ソースの一覧(システム設定 → キーボード → 入力ソース)に「NablaSKK」と
     「NablaSKK 英数」が出て、英数のメニューのアイコンが ∇ と見分けられること。
  2. 統合の入力ソースで、今までどおり変換、`l`、`SwitchToAsciiIfIdle`(F19)が働くこと。
  3. 統合で ▽ の読みを入力中に英数の入力ソースへ切り替えると、読みが確定して入ること。
  4. 英数の入力ソースで、英字、記号、Ctrl-J、Esc がそのままアプリに届くこと(Ghostty の nvim
     と、テキストエディットで)。
  5. 英数から統合に戻すと、切り替える前のエンジンのモードのまま入力できること。
  6. Kotoeri の入力ソースをすべて外せること(「−」が押せること)。外したあとも 2〜5 が
     変わらないこと。ABC が有効になっていないことを
     `defaults read com.apple.HIToolbox AppleEnabledInputSources` で確かめること。
  7. 6 のあと、パスワードの欄(ログイン画面以外、たとえばブラウザのパスワード欄)で英字が
     入力できること。
  8. `DebugLog` に、入力ソースを切り替えたときの ID が出ること(`setValue` が呼ばれていること)。

6 で Kotoeri を外せない場合や、7 で入力できない場合は、この ADR の前提(`com.apple.inputmethod.Roman`
の入力モードで ASCII の条件を満たせる)が崩れている。そのときは実装をマージせず、調べた結果を
添えて ADR を見直す。

## トレードオフ

- 入力メニューに NablaSKK の項目が 2 つ並ぶ。入力ソースの切り替え(Ctrl+Space)で英数に入る
  ことがある。英数に入っても ABC と同じなので、何が起きたかは分かりやすい。
- 英数の入力ソースでは SKK のキー操作が働かないので、Ctrl-J で戻れない。本家の同期(案 2)を
  使えば戻れるが、状態が 2 か所に分かれ、繰り返しを止める仕組みが要る。要るようになったら
  別の ADR で足す。
- `ComponentInputModeDict` を足すと、入力ソースの種類が「Keyboard Input Method」から
  「Input Mode」に変わる。前の版から上げたとき、一覧の項目を足し直す必要があるかもしれない
  (README に書く)。統合の `TISInputSourceID` は変えないので、ID で入力ソースを選ぶ設定は
  そのまま使えるはずである。
- 入力モードを持つ形は本家と同じだが、英数の動きは本家(エンジンを英数モードにする)と違う。
  意図的に変えた点として、仕様書にもこの ADR へのリンクを書く。

## 参考文献

- AquaSKK `platform/mac/proj/AquaSKK-Info.plist`(`ComponentInputModeDict`、
  `com.apple.inputmethod.Roman` の入力モード)、`platform/mac/proj/InfoPlist.strings`
  (入力モードの名前)、`platform/mac/src/server/SKKInputController.mm`(`setValue:forTag:client:`)、
  `platform/mac/src/server/SKKInputMenu.mm`(`selectInputMode`)
  https://github.com/codefirst/aquaskk
- 作業者の環境の `defaults read com.apple.HIToolbox AppleEnabledInputSources` の出力
  (2026-11-07)
- [ADR 0014](0014-app-and-input-menu-icons.md)、[ADR 0015](0015-render-icons-with-sips.md)
