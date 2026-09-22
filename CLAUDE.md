# NablaSKK — 作業規則

このリポジトリで作業するときの恒久的な規則。経緯や決定の根拠は
`docs/decisions.md`、進行中の状態は `HANDOFF.md`(gitignore、あれば)を見る。

## 位置づけ
- AquaSKK (https://github.com/codefirst/aquaskk) のエンジンを Rust に移植したもの。
  **本家の後継は名乗らない**。本家への言及・帰属は残し、自称は NablaSKK。
- ライセンスは GPL-2.0-or-later。BSD 由来部分(jconv、GenericStateMachine)は
  条件全文と免責を各ファイル先頭と NOTICE に保持する。**LICENSE / NOTICE /
  各ファイル先頭の帰属ヘッダ(`// Ported from AquaSKK ...`)は改名・整理の
  対象にしない。**
- 新規ファイルの先頭にも著作権行を書く。移植ファイルは
  `Copyright (C) <年> <本家の名義>` + `Ported to Rust by tett23, 2026.`、
  新規実装は `Written by tett23, 2026.`。

## 実装上の制約
- **外部クレート依存ゼロ**(`Cargo.toml` の `[dependencies]` は空)。
  必要なものは std か `extern "C"` で libc を直接宣言して書く。
- エンジンの挙動は本家に合わせる。設定のデフォルトは本家 UserDefaults.plist
  の値(`crates/nablaskk-core/src/config.rs`)。本家と意図的に変えた点は
  `docs/decisions.md` の対応表に追記する。
- 本家の受け入れテスト(`crates/nablaskk-core/testdata/session_test.dat`、
  400 ケース)は常に全一致させる。エンジンを触ったら必ず `cargo test`。
- Rust–Swift 境界(`crates/nablaskk-ffi`)は C ABI。文字列は NUL 終端 UTF-8、
  Rust が返した `char*` は呼び出し側が `skk_string_free` で解放。
  セッションはスレッド非対応(Rc/RefCell)なので main thread からのみ呼ぶ。
  API を足したら `include/nablaskk.h` と `swift/SKKSession.swift` も同時に更新。

## 進め方
- 作業者は IME を常用(ドッグフーディング)している。**入力中の IME を勝手に
  再起動・差し替えしない**。ビルドしたら差し替えコマンドを提示するだけにする。
- GUI や IME の挙動変更は、作業者が実機で確認してから「コミットして」と
  言われるまでコミットしない(依頼があった場合)。それ以外の変更は通常どおり。
- コミットメッセージ末尾は `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- バックグラウンドに回りうるシェルコマンドで素の `ls` を使わない
  (`ls`→`eza` の alias が stdin を読んで永久に止まる)。`command ls` を使う。

## 検証コマンド
```
cargo test                    # Rust 全テスト + 受け入れ 400 ケース
cargo clippy --all-targets    # 警告ゼロを維持
sh swift/run-smoke-test.sh    # Swift ↔ FFI 実リンクのスモークテスト
sh macos/run-tests.sh         # NSEvent 変換・ClientQuirks・設定ファイル・ユーザー辞書モデル
sh macos/build-app.sh         # NablaSKK.app(設定アプリ同梱、ad-hoc 署名)
```
差し替え(作業者が実行する):
```
rm -rf ~/Library/Input\ Methods/NablaSKK.app && cp -r macos/dist/NablaSKK.app ~/Library/Input\ Methods/ && pkill -x NablaSKK; pkill -x "NablaSKK Preferences"
```
