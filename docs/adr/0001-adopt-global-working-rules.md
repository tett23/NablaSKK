# ADR 0001: グローバルの作業規則(ADR・仕様書・Git 運用・GitHub Actions)への準拠

ステータス: 採択

## 文脈

作業者のグローバル規則(`~/.claude/CLAUDE.md`)は次を求めている。

- 実装の前に ADR を `docs/adr/` に作り、コミット前に人間のレビューを受ける。コミット済みの ADR はステータスの行以外を変えない(adr-guard で強制する)。
- `docs/specifications.md` と `README.md` を、main で常に最新に保つ。
- main への rebase は原則禁止。マージは常に `--no-ff`。
- `v` で始まるタグをリリース対象とし、`v0.0.1-rc` のようなタグはプレリリースにする。
- GitHub Actions は Node 24 以降で動く最新のメジャー版を使う。

v1.0.1 時点の NablaSKK は次の点で外れている。

- `docs/adr/` が無く、adr-guard も未導入(git hook・Claude Code hook・CI のいずれも無い)。設計判断は `docs/decisions.md` に追記してきた。
- `docs/specifications.md` が無い。仕様は README と `docs/decisions.md` に分散している。
- main に直接コミットしてきた。
- リリース用のワークフローは `-rc` タグも通常リリースの下書きにする。
- `actions/checkout@v4`、`actions/upload-artifact@v4`、`actions/download-artifact@v4`、`softprops/action-gh-release@v2` が Node 20 で動く版のまま(`Swatinem/rust-cache@v2` は Node 24 で動く)。

## 実装すること

### ADR と adr-guard

- `docs/adr/` を作り、以後の設計判断は ADR として記録する。この ADR が 0001。
- adr-guard v0.1.0 を README の手順どおり三か所に導入する。
  - `.githooks/pre-commit`(`exec adr-guard pre-commit`、実行権限付き)。各クローンで `git config core.hooksPath .githooks` が要るので、README の開発手順に書く。
  - `.claude/settings.json` の PreToolUse hook(`Write|Edit|MultiEdit` に `adr-guard claude-hook`)。
  - CI に `adr` ジョブを追加し、`tett23/adr-guard@v0.1.0` で `check-range` を実行する(`fetch-depth: 0`、`persist-credentials: false`)。
- `docs/decisions.md` は ADR 導入前の記録として残し、冒頭に「以後の判断は `docs/adr/` に記録する。このファイルには追記しない」と書く。過去の判断を ADR に書き写すことはしない。

### 仕様書

- `docs/specifications.md` を新設し、v1.0.1 時点の利用者から見える仕様をまとめる。
  - 対応環境、入力モードとキー操作(本家準拠、keymap.conf)
  - 辞書(`dictionaries.conf` の書式とタイプ番号、追加・上書き、skkserv、ユーザー辞書)
  - 設定ファイル(`settings.conf`・`keymap.conf`・`kana-rule.conf` のキーと既定値)
  - 設定アプリの各ページ、FFI の API の一覧(`include/nablaskk.h` を正とする)
  - 本家との意図的な違い(`docs/decisions.md` の対応表への参照)
- README は使い方の入口として残し、詳しい仕様は `docs/specifications.md` を参照させる。開発手順に hook の有効化を足す。

### Git 運用

- 以後の変更は作業ブランチで行い、main へは `git merge --no-ff` で入れる。main を rebase しない。
- プロジェクトの `CLAUDE.md` の「進め方」に、ADR を先に書くこと、ブランチと `--no-ff` マージ、ADR・仕様書・README の更新を足す。既存の規則(IME を勝手に差し替えない、実機確認後にコミットする等)はそのまま残す。

### GitHub Actions

- 各アクションを Node 24 で動く最新メジャーに上げる。
  - `actions/checkout@v7`、`actions/upload-artifact@v7`、`actions/download-artifact@v8`、`softprops/action-gh-release@v3`、`Swatinem/rust-cache@v2`(据え置き)
- `release` ジョブで、タグ名に `-` を含む場合は `prerelease: true` にする(`v1.1.0-rc` など)。
- 上げたあと、作業ブランチの CI と、`-rc` タグでの試しのリリース(プレリリースになること、成果物 4 点が付くこと)で確かめる。試しのタグとプレリリースは確認後に削除する。

## 実装しないこと

- 既存の Rust コードを関数型のスタイルに書き直すこと(本家からの移植で、受け入れテスト 400 件で挙動を固定している。新しく書くコードから従う)。
- `docs/decisions.md` の内容を ADR に移すこと、書きかえること。
- 過去のコミット・タグの書きかえ。
- 監査や性能計測のスナップショット(現時点で対象が無い)。
- Node.js の導入(このリポジトリに Node を使うジョブは無い)。

## テスト設計

- adr-guard
  - コミット済みの ADR のステータス以外の行を書きかえてコミットしようとすると、pre-commit hook が拒否すること。
  - Claude Code から同じ編集をすると、hook が拒否すること。
  - ステータスの行だけの変更と、新しい ADR の追加は通ること。
  - CI の `adr` ジョブが、ADR を書きかえたコミットを含むブランチで失敗し、含まないブランチで成功すること。
- GitHub Actions
  - 作業ブランチの push で `test (ubuntu-latest)`・`test (macos-latest)`・`adr` が成功すること。
  - `-rc` タグで `release` まで成功し、プレリリースの下書きになること。通常のタグでは従来どおり通常の下書きになること。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、`sh macos/run-tests.sh`、`sh macos/build-app.sh`)がすべて通ること。

## トレードオフ

- ADR のレビューを挟むぶん、小さな変更でも着手までに一往復増える。代わりに判断の経緯が残り、あとから書きかえられない。
- `docs/decisions.md` を凍結して ADR と並べるため、判断の記録が二か所に分かれる。書き写すと経緯の日付や文脈を失うので、凍結と参照で済ませる。
- 仕様書は README・`docs/decisions.md`・ヘッダと内容が重なる。重複は仕様書を正とし、README からは参照にとどめることで、食い違いを抑える。
- adr-guard が手元に無いクローンでは pre-commit hook がコミットを拒否する。CI でも検査されるので、README に導入手順を書いて対応する。
- `softprops/action-gh-release` を v2 から v3 に上げると入力の互換性が変わる可能性がある。`-rc` タグでの試しのリリースで確かめてから main に入れる。
