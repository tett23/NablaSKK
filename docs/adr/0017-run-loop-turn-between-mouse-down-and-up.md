# ADR 0017: UI テストのクリックで、mouse-down と mouse-up のあいだに run loop を回す

ステータス: 採択

## 文脈

[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) で、設定アプリの UI テストの `click(at:)` を次のように変えた。

1. mouse-up を待ち行列に積む。
2. mouse-down を `window.sendEvent` で送る。
3. 戻ったあと、mouse-up がまだ待ち行列にあれば、すぐに取り出して送る。

このプルリクエスト(tett23/NablaSKK#2)の CI では、2 つの実行のうち push の実行は通ったが、
pull_request の実行(run 41791810286)で 2 つの検査が落ちた。

- 最初のクリック(4 組のうち 1 組目の「入力」)でページが変わらなかった
  (`click 入力: the page switches` と `got [0]`)。5 秒の条件待ちでも変わらなかったので、
  描画の遅れではなく、クリックが効かなかった。
- 残り 11 回のクリックと、↑↓、辞書・キー・ローマ字・ユーザー辞書の検査はすべて通った。
  `stray mouse event` の検査は落ちていない。[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) の目的だった、失敗の連鎖は
  起きていない。

### 手元で確かめたこと(macOS 27.0)

- テストのウィンドウは、最初のクリックの時点でキーウィンドウではなく、アプリもアクティブでは
  ない(`isKeyWindow`、`NSApp.isActive` ともに false)。それでも手元ではクリックが効くので、
  キーウィンドウかどうかは CI だけの違いではない。
- 最初の検査(辞書の強調)を消して、ウィンドウを出した直後にクリックしても、手元では効いた。
- 直したテストを 40 回(うち 20 回は CPU に負荷をかけて)実行して、一度も落ちなかった。

手元では再現できていない。

### 推測(確かめていないこと)

[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) の前は、mouse-down のあと、タイマーが mouse-up を積むまでの約 0.1 秒、
`nextEvent` が run loop を回していた。いまは、mouse-down と mouse-up が run loop を一度も回さずに
続けて届く。人が押すクリックでは、そのあいだに必ず run loop が回る。

SwiftUI が、最初のマウスのイベントでジェスチャーの準備などを run loop の次の回に回していて、
遅いランナーではその前に mouse-up が届くと押したことにならない、という可能性がある。
変更の前の CI では、最初のクリックが落ちた記録はない(run 41787952533 の 1 回目と 2 回目、
数分前の main の実行)。ただし、根拠はこれだけで、SwiftUI の中で何が起きているかは確かめて
いない。

## 実装すること

- `click(at:)` の手順 3 の前に、mouse-up がまだ待ち行列にあれば(追跡ループが取らなかったら)、
  run loop を 0.1 秒回してから mouse-up を取り出して送る。0.1 秒は、[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) の前に mouse-up を
  積んでいたタイマーと同じ長さである。
  - `RunLoop.main.run(until:)` は待ち行列のイベントを配らないので、待ち行列の mouse-up は
    そのまま残る。待ったあとに締め切りなしで取り出すので、[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) で直した「mouse-up が次の
    クリックに渡る」問題は戻らない。
- クリックしてもページが変わらなかったときに、原因を絞れるよう、サイドバーのクリックの失敗の
  検査名に、そのときの選択中のページを含める(いまの `got [...]` に加えて
  `pane=<値>`)。
- [ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) のステータスの行を、この ADR を指すように変える。

## 実装しないこと

- 失敗したクリックをやり直すこと。最初のクリックが効かない不具合
  ([ADR 0005](0005-fix-sidebar-selection-highlight.md) で直した種類)を隠してしまう。
- CI で UI テストを自動で再実行すること。
- この推測を確かめるために SwiftUI の中を調べること。

## テスト設計

- [ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) のテスト設計の、遅れを入れる 3 通りと、余分な mouse-up を積む改変を、直した
  `main.swift` でもう一度行い、同じ結果になること(遅れでは全部通る、余分な mouse-up では
  1 つだけ落ちる)。
- 手元で 20 回、CPU に負荷をかけて 20 回続けて実行し、すべて通ること。
- CI: このプルリクエストへの push の実行と pull_request の実行の両方で UI テストが通ること。
  さらに、macos-latest の test ジョブを 5 回再実行して(`gh run rerun --job`)、すべて通ること。
  手元で再現できないので、これが推測を裏づけるただひとつの手段である。5 回で落ちなくても、
  原因を確かめたことにはならない。そのことは仕様やリリースノートにも、確かめたこととしては
  書かない。

## トレードオフ

- クリック 1 回につき 0.1 秒、テストが長くなる。クリックは 20 回ほどなので、2 秒ほどである。
- 推測が外れていれば、最初のクリックはまた落ちうる。そのときは、検査名に足した情報を手がかりに
  調べ直す。少なくとも、[ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md) のおかげで、落ちるのはそのクリックの検査だけで済む。

## 参考文献

- GitHub Actions run 41791810286(pull_request、tett23/NablaSKK#2)
- GitHub Actions run 41791785471(push、同じコミット、通った)
- [ADR 0016](0016-deterministic-clicks-in-preferences-ui-test.md)
