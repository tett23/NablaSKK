# NablaSKK ロードマップ

まだ手を付けていない候補。着手するときは、それぞれ ADR を書いてから実装する。
順番は優先度を表さない。

## 候補

- **候補ウィンドウを独立して表示する**: 今は候補をマークテキストの中に
  `【a:候補 s:候補 …】` の形で並べている。InputMethodKit の `IMKCandidates` を使い、
  本家のように別のウィンドウに候補を出す。
- **入力モードごとに入力ソースを登録する**: `Info.plist` の
  `ComponentInputModeDict` に、ひらがな・カタカナ・半角カナ・英数・全角英数を
  登録し、メニューバーの表示を今のモードに合わせて切り替える。
- **設定アプリでオプションを変えられるようにする**: egg-like-newline など、本家の
  設定項目のうち、まだ設定アプリにないものを足す。
- **設定アプリにアイコンを付ける**: 今は設定アプリの `Info.plist` にアイコンの指定が
  なく、Dock や Finder では汎用のアイコンになる。アイコン(`.icns`)を用意して
  `CFBundleIconFile` で指定し、`macos/build-app.sh` で同梱して `macos/check-bundle.sh`
  でも確かめる。絵柄を新しく描くか、本家 AquaSKK のものを元にするかと、素材のライセンスは
  ADR で決める。
- **Nix で配布する**: Apple Developer Program の費用を払い続けられないので、
  Developer ID の署名と公証はしない。Homebrew の cask は公証していないアプリを
  入れにくいので、Nix(flake。nix-darwin と home-manager から使えるようにする)で
  アプリと CLI ツールを入れる方法を用意し、README で案内する。
