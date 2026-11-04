# ADR 0016: 設定アプリの UI テストのクリックを、時間に頼らずに届ける

ステータス: 採択

## 文脈

`sh macos/run-ui-tests.sh`(`macos/Tests/PreferencesUITests/main.swift`、
[ADR 0005](0005-fix-sidebar-selection-highlight.md))が、GitHub Actions の macos-latest で
ときどき落ちる。run 41787952533(タグ v1.2.3、1 回目)では、サイドバーのクリックを 11 回
通したあと、4 組目の「click skkserv」から先のクリックでページが変わらなくなり(`got [5]`、
キーのまま)、↑↓ の検査も 3 つ落ちた。その後の辞書・キー・ローマ字・ユーザー辞書の検査は
通った。同じコミットの再実行と、数分前の main の実行は通っている。

### いまのクリックの作り方

`click(at:)` は次の順で動く。

1. mouse-up を 0.1 秒後に `NSApp.postEvent` で積むタイマーを仕掛ける。
2. mouse-down を `window.sendEvent` で直接送る。
3. `NSApp.nextEvent(matching: .leftMouseUp, until: 0.3 秒後)` で mouse-up を待ち、
   取れたら `window.sendEvent` で送る。
4. `settle()` で run loop を 0.4 秒回す。

タイマーにしているのは、テキストフィールドやボタンが mouseDown の中で回す追跡ループにも
mouse-up を届けるためである。

### 原因(手元で再現したこと)

手順 3 の 0.3 秒のうちにタイマーが発火しないと、`nextEvent` は何も返さない。そのあと
タイマーは `settle()` の中で発火し、mouse-up をアプリのイベント待ち行列に積む。
`RunLoop.main.run(until:)` は待ち行列のイベントを配らないので、この mouse-up は残る。

すると次のクリックで、手順 3 の `nextEvent` は**前のクリックの位置の** mouse-up を先に
取り出して送る。サイドバーの行は `.plain` の `Button` で、押した行の外で離すと何も
しないので、ページは変わらない。今度のクリックの mouse-up がまた残り、次のクリックに
渡る。こうして失敗が連鎖する。同じ位置を 2 回続けてクリックすると、残った mouse-up が
たまたま正しい位置になるので、連鎖が切れることがある。

手元(macOS 27.0)で、11 回目のクリック(4 組目の skkserv)だけタイマーを 0.5 秒にして
実行すると、サイドバーと ↑↓ の検査の結果(ok と FAIL の並び、`got [...]` の値)が
CI のログの 1 行目から 40 行目と一致した。

- CI と同じく、skkserv とローマ字のクリックは `got [5]` で落ち、↓ と ↑ の検査は
  落ち、「↓ on the last page stays」はたまたま通った(キーに戻っていたため)。
- その後の辞書ページから先は、CI ではすべて通ったが、手元では 5 つ落ちた
  (12 FAILED)。連鎖がどこで切れるかは、テキストフィールドの追跡ループが残った
  mouse-up をいくつ取り出すかで変わるらしい。この差は確かめていない。

CI でタイマーが 0.3 秒のうちに発火しなかった理由(ランナーが一時的に止まった、など)は
確かめていない。ログには時刻が検査ごとに出ていないので、分からない。ただ、理由が何で
あれ、一度の遅れが後の検査をすべて巻き込む作りであることが不安定さの原因である。

### 他の候補

- 選ばれたページの UserDefaults への書き込み: `@AppStorage` は同じプロセスの
  `UserDefaults.standard` に同期して書くので、検査が古い値を読むことはない。上の再現は
  書き込みを変えずに CI の失敗を再現したので、原因ではないと判断する。
- `settle()` の 0.4 秒: 強調の検査が描画の遅れで落ちるなら、`page switches`(UserDefaults)
  は通って `highlighted` だけが落ちるはずである。CI では両方落ちているので、主因ではない。
  ただ、遅いランナーで描画が間に合わない余地は残るので、あわせて直す。

## 実装すること

テストのコード(`macos/Tests/PreferencesUITests/main.swift`)だけを変える。設定アプリの
本体は変えない。

1. **mouse-up をタイマーではなく、mouse-down の前に積む。** `click(at:)` は、
   - mouse-up を `NSApp.postEvent(_:atStart: false)` で待ち行列に積み、
   - mouse-down を `window.sendEvent` で送り、
   - 戻ったあと、mouse-up がまだ待ち行列にあれば(追跡ループが取らなかったら)
     `nextEvent(matching: .leftMouseUp, until: .distantPast, ...)` で取り出して
     `window.sendEvent` で送る。
   待ち時間の締め切りがないので、ランナーが遅くても mouse-up を取りこぼさない。
   追跡ループがあれば、その中で mouse-up がすぐに取り出される。
2. **クリックの前と後で、待ち行列にマウスのイベントが残っていないことを確かめる。**
   残っていたら取り除き、`FAIL click: stray mouse event before/after the click` として
   数える。仕組みが壊れたときに、後の検査を巻き込まずにその場で分かるようにする。
3. **状態の検査は、一定時間待つのではなく、条件が成り立つまで待つ。** `waitUntil(timeout:_:)`
   を足し、run loop を短く回しながら条件を確かめ、成り立てばすぐ戻る。締め切り(5 秒)を
   過ぎたら、そのときの値で検査する。ページの切り替え、行の強調、テキストフィールドの
   カーソルの検査に使う。
   - 「変わらないこと」を確かめる検査(最後のページで ↓、最初のページで ↑)は条件待ちが
     使えないので、いまの `settle()` のまま待つ。
4. 1 と 3 で不要になった `click(at:)` 内のタイマーと固定の待ちを消す。`settle()` は
   キー入力の後と、「変わらないこと」の検査のために残す。

## 実装しないこと

- 設定アプリ本体(`PreferencesApp.swift` ほか)の変更。
- UI テストを XCUITest に移すこと。Xcode のプロジェクトを持たず `swiftc` で作る構成を変える
  ことになり、この不具合を直すだけには大きすぎる。
- CI で UI テストを自動で再実行すること。不安定さを隠すだけなので採らない。
- ユーザー辞書の一覧(`List`)へのクリック。いまも合成したクリックが届かないので
  `selectRowIndexes` で選んでおり、この ADR では変えない。

## テスト設計

テストのコードそのものを直す ADR なので、直したテストが時間に左右されないことを、
遅れをわざと入れて確かめる。確かめるための改変は開発のときだけ使い、リポジトリには
入れない。

- **再現(直す前)**: いまの `main.swift` で 11 回目のクリックだけタイマーを 0.5 秒にすると、
  CI と同じ並びで落ちること(上の「原因」で確かめ済み)。
- **遅れへの強さ(直した後)**: 直した `main.swift` で、11 回目のクリックの mouse-down の
  前と後に、main thread を 0.5 秒止める(`Thread.sleep`)改変を入れても、すべて通ること。
  止める位置は、mouse-down の直前、mouse-down の直後(mouse-up を送る前)、クリックの後
  の 3 通りを試す。
- **取り残しの検出**: 直した `main.swift` で、あるクリックの前に余分な mouse-up を
  待ち行列に積む改変を入れると、`stray mouse event` の検査が 1 つ落ち、それ以外の
  検査は通ること(連鎖しないこと)。
- **繰り返し**: 手元で `sh macos/run-ui-tests.sh` を 20 回続けて実行し、すべて通ること。
  CPU に負荷をかけた状態(論理 CPU の数だけ `yes > /dev/null` を走らせる)でも 20 回
  通すこと。
- **CI**: ブランチの push で CI の UI テストが通ること。
- `cargo test`、`cargo clippy --all-targets`、`sh macos/run-tests.sh` は変わらず通ること
  (Swift のテストのコードしか変えないが、念のため)。

## トレードオフ

- mouse-up を mouse-down の前に積むと、追跡ループは mouse-down の直後に mouse-up を
  受け取る。人が押してすぐ離した場合と同じで、押したまま動かす操作(ドラッグ)は
  この `click(at:)` では試せない。いまのテストにドラッグはないので困らない。
- 条件待ちの締め切りを 5 秒にすると、本当に壊れたときにテストが遅くなる(落ちる検査
  1 つにつき最大 5 秒)。通るときは条件が成り立った時点で戻るので、いまより速くなる
  見込みである。
- 「変わらないこと」の検査は、固定の待ちのままなので、遅いランナーで「まだ変わって
  いないだけ」を見逃す余地がある。誤って通る方向の誤りなので、落ちる不安定さには
  ならない。

## 参考文献

- GitHub Actions run 41787952533、attempt 1(`gh run view 41787952533 --attempt 1 --log-failed`)
- Apple Developer Documentation: `NSApplication.postEvent(_:atStart:)`、
  `NSApplication.nextEvent(matching:until:inMode:dequeue:)`
- [ADR 0005](0005-fix-sidebar-selection-highlight.md)
