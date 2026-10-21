# NablaSKK ロードマップ

まだ手を付けていない候補。着手するときは、それぞれ ADR を書いてから実装する。
順番は優先度を表さない。

## 候補

- **候補ウィンドウを独立して表示する**: 今は候補をマークテキストの中に
  `【a:候補 s:候補 …】` の形で並べている。InputMethodKit の `IMKCandidates` を使い、
  本家のように別のウィンドウに候補を出す。
- **設定アプリでオプションを変えられるようにする**: egg-like-newline など、本家の
  設定項目のうち、まだ設定アプリにないものを足す。
- **Nix で配布する**: Apple Developer Program の費用を払い続けられないので、
  Developer ID の署名と公証はしない。Homebrew の cask は公証していないアプリを
  入れにくいので、Nix(flake。nix-darwin と home-manager から使えるようにする)で
  アプリと CLI ツールを入れる方法を用意し、README で案内する。
