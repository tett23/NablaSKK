# NablaSKK ロードマップ

まだ手を付けていない候補。着手するときは、それぞれ ADR を書いてから実装する。
優先度は作業者が決めたもの(2026-10-22)。

## 優先度: 中

- **Nix で配布する**: Apple Developer Program の費用を払い続けられないので、
  Developer ID の署名と公証はしない。Homebrew の cask は公証していないアプリを
  入れにくいので、Nix(flake。nix-darwin と home-manager から使えるようにする)で
  アプリと CLI ツールを入れる方法を用意し、README で案内する。

## 未定(要るかどうかから考える)

- **候補ウィンドウを独立して表示する**: 今は候補をマークテキストの中に
  `【a:候補 s:候補 …】` の形で並べている。InputMethodKit の `IMKCandidates` を使い、
  本家のように別のウィンドウに候補を出す。要るかどうかを作業者が考えている。
- **本家の設定画面の残りの項目**([ADR 0027](adr/0027-engine-options-in-preferences.md) で調べて、値を渡すだけでは
  済まないので見送ったもの):
  - 単語登録を始めるときの警告音(`beep_on_registration`)。FFI にエンジンからホストへの
    コールバックが要る。
  - キーボードレイアウトの指定(`keyboard_layout`)。入力ソースの扱いなので、実機で確かめながら進める。
  - 入力モードを切り替えたときに、カーソルの近くにモードを出す表示(`show_input_mode_icon`)。
  - 候補の選択ラベル(`candidate_window_labels`)。1 ページの候補の数も変わる。
