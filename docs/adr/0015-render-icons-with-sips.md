# ADR 0015: アイコンの SVG を qlmanage ではなく sips で描く

ステータス: 採択

## 文脈

[ADR 0014](0014-app-and-input-menu-icons.md) は、`macos/make-icons.sh` で SVG を PNG に
するときに `qlmanage` を使うと決めた。実装してみると、`qlmanage` は SVG を透明な背景では
なく白い背景の上に描くことが分かった(観察したこと)。

- 入力メニュー用(16px)に描いた画像は、256 ピクセルすべてが不透明だった。半透明の
  ピクセルはなかった。
- `sips -g hasAlpha` はアルファチャンネルがあると答えるが、中身は不透明で埋まっている。

このままでは、入力メニュー用は白い四角の上の黒い ∇ になり、テンプレート画像として
使えない(macOS はアルファの形で色を付ける)。設定アプリ用も、角丸の外側が白くなる。

`sips` は SVG を直接読める。`sips -s format png -z <大きさ> <大きさ>` で SVG から描くと、
次のようになった(macOS 27.0 で観察したこと)。

- 入力メニュー用(16px と 32px): 色は黒だけ(RGB の最大値が 0)。輪郭は半透明でぼかされ、
  四隅は透明。
- 設定アプリ用(16px と 1024px): 四隅(角丸の外側)は透明。絵柄は SVG のとおり。
- AppKit の `NSImage` で同じ SVG を透明なビットマップに描いた結果と、見えるピクセルと
  半透明のピクセルの数が一致した。

AppKit を JavaScript for Automation(`osascript -l JavaScript`)から使う方法も試したが、
作業者が Automation を使いたくないので採らない。

## 実装すること

- `macos/make-icons.sh` で、SVG から PNG にする道具を `qlmanage` から `sips` に変える。
  大きさごとに SVG から直接描く(1024px から縮めない)。小さい大きさでも輪郭がぼけない。
  macOS に付いている道具だけを使う、という [ADR 0014](0014-app-and-input-menu-icons.md) の方針は変えない(`sips`、`iconutil`、
  `tiffutil`)。
- [ADR 0014](0014-app-and-input-menu-icons.md) のステータスの行を、この ADR を指すように変える。
- そのほかの [ADR 0014](0014-app-and-input-menu-icons.md) の内容(絵柄、同梱するファイル、`Info.plist` のキー、統合だけを
  サポートすること)は変えない。

## 実装しないこと

- 外部の道具(rsvg-convert など)を入れること。
- JavaScript for Automation、または描くための Swift のスクリプトを置くこと。
- `qlmanage` の背景を透明にする回避策を探すこと。

## テスト設計

[ADR 0014](0014-app-and-input-menu-icons.md) のテスト設計に、次を足す。

- `macos/make-icons.sh` で作った `MenuIcon.tiff` の 16px と 32px の画像で、
  - 四隅が透明であること。
  - 半透明のピクセル(ぼかした輪郭)があること。
  - 色が黒だけであること(RGB がすべて 0)。
- `AppIcon.icns` の各大きさの画像で、四隅が透明であること。
- 確かめるには、画素を読む小さなスクリプトを開発のときに使う(リポジトリには入れない)。

## トレードオフ

- `sips` の SVG の扱いは、man ページに詳しく書かれていない。macOS の版によって描き方が
  変わるかもしれない。作った画像をリポジトリに入れる([ADR 0014](0014-app-and-input-menu-icons.md))ので、配布物の見た目は
  固定される。
- `sips` が SVG を読めなくなった場合は、この ADR を見直す。

## 参考文献

- [ADR 0014](0014-app-and-input-menu-icons.md)
- `man sips`
