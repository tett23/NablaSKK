# ADR 0029: adr-guard を v0.13.1 に上げ、ADR のステータスの行を新形式に移す

Status: accepted-pending

## 文脈

CI の `adr` ジョブ(`.github/workflows/ci.yml`)は、`tett23/adr-guard@v0.10.0` で、コミット済みの
ADR がステータスの行とリンク以外で変わっていないことを検査する([ADR 0001](0001-adopt-global-working-rules.md)、
[ADR 0028](0028-adr-check-base-for-new-refs.md))。手元の pre-commit の hook(`.githooks/pre-commit`)は
`adr-guard pre-commit --allow-status --allow-links` を実行し、手元に入っている adr-guard の版で動く。

その後、adr-guard は v0.13.1 になった。v0.10.0 から v0.13.1 までに、次が入った(adr-guard の README と
リリース)。

- v0.11.0: Codex の PreToolUse hook として動く `adr-guard codex-hook`
  ([adr-guard の ADR 0021](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0021-codex-hook.md))。
- v0.12.0: ステータスの行の新形式と、状態遷移の検査
  ([adr-guard の ADR 0022](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0022-status-transitions.md))。
  - 新形式は `Status: <トークン>` である。トークンは `proposed`、`accepted`、`accepted-pending`、
    `rejected`、`discarded`、`deprecated` のいずれか、または `superseded [<変更の JSON 配列>]` である。
    変更は `{"part":<部分名か null>,"by":[<リポジトリのルートからのパス>]}` と書く。
  - 追加した ADR と、ステータスの行を書き換えた ADR には、新形式が要る。新しい ADR は、最初の
    コミットで `Status: proposed` でなければならない。
  - pre-commit と `check-range` は、HEAD からの状態遷移を検査する。遷移の違反は `--allow-status`、
    環境変数 `ADR_GUARD_ALLOW`、コミットのトレーラーでも免除されない。
  - 旧形式(`ステータス: 採択` など)の行は、書き換えない限りそのまま読める。
  - 旧形式を新形式に書き換える移行スクリプト `scripts/migrate-adr-status` が adr-guard のソースに
    ある。リリースのアーカイブには入らない。
  - 修正先を辿る `adr-guard trace`
    ([adr-guard の ADR 0023](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0023-trace-amendments.md))。
- v0.13.0: 移行スクリプトの `--dry-run` と、移行できない ADR の全件の数え上げ
  ([adr-guard の ADR 0024](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0024-migration-dry-run-and-error-count.md))。
  状態 `superseded-pending`(実装を待つ ADR が修正を受けた状態)、変更の `kind` と `note`、
  `https://` の修正先、`trace --list` の 6 列
  ([adr-guard の ADR 0025](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0025-extend-canonical-status-amendments.md))。
  旧形式の参照の読み方の拡張
  ([adr-guard の ADR 0026](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0026-read-more-legacy-references.md)、
  [adr-guard の ADR 0027](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0027-read-legacy-replaced-status.md))。
- v0.13.1: 移行スクリプトが旧形式の行から部分名を作るときに、修飾子(`ADR NNNN` の直前の語)、先頭の「。」、
  末尾の「→」を残さなくなった。部分名を規則で決められない行は、「部分名を決められません」と報告し、
  書き換えない([adr-guard の ADR 0028](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0028-fix-legacy-part-names.md))。

手元の adr-guard は 0.13.1 で、CI の v0.10.0 と版がずれている。このリポジトリの ADR は、すべて旧形式の
ステータスの行を持つ。また、`CLAUDE.md` の「進め方」は、採択された ADR のステータスを「採択」にして
コミットすると書いている。v0.13.1 の pre-commit は、最初のコミットが `proposed` でない ADR を拒否する
ので、この手順には従えない。

README.md と README.en.md の adr-guard のインストール手順は、`--tag v0.8.2` のままである。
[ADR 0028](0028-adr-check-base-for-new-refs.md) は CI だけを v0.10.0 に上げ、README のタグを変えなかった。

## 調査した事項

次は、この ADR の worktree(origin/main から作ったブランチ `adr-guard-v0.13.0`)で実行して確かめた。

- `adr-guard -V` は `adr-guard 0.13.1` を出力した。
- `.github/workflows/ci.yml` の `adr` ジョブは `uses: tett23/adr-guard@v0.10.0` で、入力は
  `allow-status: true`、`allow-links: true`、`base: ${{ steps.range.outputs.base }}`、
  `head: ${{ steps.range.outputs.head }}` である。CI を v0.10.0 にしたのは、
  [ADR 0028](0028-adr-check-base-for-new-refs.md) を実装したコミット 2157690 である。
- `.githooks/pre-commit` は `exec adr-guard pre-commit --allow-status --allow-links` を実行する。
  `.claude/` は無い。
- README.md の 334 行目と README.en.md の 338 行目は
  `cargo install --git https://github.com/tett23/adr-guard --tag v0.8.2` である。
- `docs/specifications.md` の「10. 開発とリリース」は、CI の adr-guard を v0.10.0 と書いている。
- adr-guard の v0.13.1 のタグはコミット `ab55769152ed0f8a3465ee3435a2df43ab9d067f` を指す。v0.13.1 の
  `action.yml` の入力は `base`、`head`、`version`、`allow-status`、`allow-links`、`links`、
  `check-status`、`check-numbers`、`allow-renumber` である。
- この ADR を足す前に、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` は、
  いずれも終了コード 0 で終わった。
- worktree のルートで、adr-guard の v0.13.1 のソースの `scripts/migrate-adr-status`(ビルドしたもの)を
  `--dry-run` で実行した。この ADR を足す前は、終了コード 0 で、標準エラーに
  `移行できる: 28件、移行済み: 0件、移行できない: 0件` を出力した。28 件の内訳は `Status: accepted` が 21 件、`Status: discarded` が 3 件、`Status: superseded [...]` が 4 件
  である。
- v0.13.1 の移行スクリプトの `--dry-run` が出す新形式のステータスの行は、v0.13.0 の移行スクリプトの
  出力とすべて同じだった(`diff` で差が無かった)。
- worktree を `$TMPDIR` に clone し、実装の手順を試した。この ADR を `Status: proposed` で足した
  インデックスで、`adr-guard pre-commit --allow-status --allow-links` は終了コード 0 で終わった。それを
  コミットし、ステータスを `Status: accepted-pending` にしたインデックスでも、終了コード 0 で終わった。
- 続けて、移行スクリプトの `--dry-run` は `移行できる: 28件、移行済み: 1件、移行できない: 0件`
  (移行済みの 1 件はこの ADR)を出力した。`--write` で書き換え、[ADR 0028](0028-adr-check-base-for-new-refs.md)
  のステータスの行を「実装すること」の行にしたインデックスで、pre-commit は終了コード 0 で終わった。
- そのコミットの後、`--dry-run` は `移行できる: 0件、移行済み: 29件、移行できない: 0件` を出力した。
  `adr-guard check-status`、`adr-guard links`、`adr-guard check-numbers`、clone した時点の HEAD から
  の `adr-guard check-range <HEAD> HEAD --allow-status --allow-links` は、いずれも終了コード 0 で
  終わった。`^ステータス:` で始まる行は残らなかった。
- 旧形式の ADR を足した main のコミット b7ab1ba([ADR 0028](0028-adr-check-base-for-new-refs.md) を
  `ステータス: 採択（未実装）` で足したもの)で
  `adr-guard check-range b7ab1ba^ b7ab1ba --allow-status --allow-links` を実行すると、
  「新形式のステータス行は `Status: ` で始めてください」 を出して終了コード 1 で終わった。同じ ADR の
  ステータスを `採択（未実装）` から `採択` に書き換えたコミット 2157690 でも、同じ診断を出した。
  全履歴にはこれらのコミットが入るので、v0.13.1 で全履歴を検査すると、ここでも失敗する(全履歴の
  検査は実行していない)。CI は [ADR 0028](0028-adr-check-base-for-new-refs.md)
  の step で範囲を決めるので、全履歴は検査しない。

## 実装すること

- `.github/workflows/ci.yml` の `adr` ジョブの `tett23/adr-guard@v0.10.0` を `tett23/adr-guard@v0.13.1`
  に上げる。いまと同じくタグで指定する。
  入力は今のまま(`allow-status: true`、`allow-links: true`、`base`、`head`)にする。`base` を求める
  step([ADR 0028](0028-adr-check-base-for-new-refs.md))は変えない。
- README.md と README.en.md の adr-guard のインストール手順のタグを `v0.13.1` にする。
- worktree のルートで、adr-guard の v0.13.1 のソースの `scripts/migrate-adr-status` を `--dry-run` で実行し、移行できない
  ADR が 0 件であることを確かめてから、`--write` ですべての ADR のステータスの行を新形式に書き換える。
- [ADR 0028](0028-adr-check-base-for-new-refs.md) のステータスの行を
  `Status: superseded [{"part":"CI の adr-guard の版","by":["docs/adr/0029-update-adr-guard-to-v0-13-1.md"]}]`
  にする。
- `CLAUDE.md` の「進め方」の ADR の手順を、次にする。
  1. ADR を書き、`Status: proposed` のまま、コミットせずにレビューを受ける。
  2. 採択されたら、`Status: proposed` のままコミットする。
  3. 続くコミットで、ステータスを `accepted-pending`(実装の要る ADR)か `accepted`(実装の要らない
     ADR)にし、実装に入る。実装を終えたら `accepted-pending` を `accepted` にする。
- `docs/specifications.md` の「10. 開発とリリース」で、CI の adr-guard の版を v0.13.1 にし、ADR の
  ステータスの行が新形式(`Status: …`)であること、pre-commit の hook と CI が状態遷移を検査すること、
  新しい ADR を `proposed` で一度コミットすることを書く。
- `adr-guard links --fix` を実行し、`adr-guard links`、`adr-guard check-status`、
  `adr-guard check-numbers` が終了コード 0 で終わるようにする。

## 実装しないこと

- pre-commit の hook のコマンドの変更。`adr-guard pre-commit --allow-status --allow-links` のままに
  する。
- Claude Code の hook の変更。利用者の全体の設定(`~/.claude/settings.json`)で入れるままにする。
- Codex の hook(v0.11.0 の `adr-guard codex-hook`)の導入。
- 変更の `kind`(`supplement`、`note`)と `note`、`https://` の修正先の利用。
- 移行スクリプトをこのリポジトリに置くこと。移行は一度だけ行い、adr-guard の v0.13.1 のソースから実行する。
- CI の版を SHA で固定すること。いまと同じくタグで指定する。

## 不変条件

なし。CI と README の adr-guard の版、ADR のステータスの行の書き方、ADR の手順だけを決める ADR であり、
NablaSKK の振る舞いを変えない。

## テスト設計

CI の設定と文書の変更なので、自動テストは書かない。次のように確かめる。

- 移行スクリプトの `--write` の後に `--dry-run` を実行すると、`移行できる: 0件` で、移行できない ADR が
  0 件であること。
- `rg -n '^ステータス:' docs/adr` が何も出力しないこと。
- 手元の adr-guard(0.13.1)で `adr-guard check-numbers`、`adr-guard check-status`、`adr-guard links` が
  終了コード 0 で終わること。
- 各コミットで pre-commit の hook が通り、
  `adr-guard check-range origin/main HEAD --allow-status --allow-links` が終了コード 0 で終わること。
- `adr-guard trace --list docs/adr/0028-adr-check-base-for-new-refs.md` が、この ADR への辺
  (部分名 `CI の adr-guard の版`)を出すこと。
- `.github/workflows/ci.yml` が YAML として読めること。
- ブランチの push と pull request の run で、`adr` ジョブが v0.13.1 で成功すること。

## トレードオフ

- 手元と CI の版がそろい、新しい ADR を新形式で書ける。その代わり、ADR を足すたびに `proposed` の
  コミットと採択のコミットの 2 つが要る。
- 状態遷移の違反は `--allow-status` でも免除されないので、誤ったステータスの書き換えを手元と CI で
  止められる。その代わり、誤って入れたステータスを戻すには、許された遷移を辿る必要がある。
- CI を上げた後は、旧形式のステータスの行で ADR を足すコミットと、旧形式の行を旧形式のまま書き換える
  コミット(`採択（未実装）` から `採択` など)が、CI で失敗する。移行より前に分けたブランチにそのような
  コミットがあれば、マージの前にステータスの行を新形式に直す。
- 28 件の ADR のステータスの行を一度に書き換えるので、差分は大きい。書き換えはステータスの行だけで、
  `--allow-status` の範囲に収まる。

## 参考文献

- [adr-guard の ADR 0021: Codex の PreToolUse hook で、コミット済みのADRと記録の編集を拒否する](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0021-codex-hook.md)
- [adr-guard の ADR 0022: 状態と遷移を別々の代数的データ型で表す](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0022-status-transitions.md)
- [adr-guard の ADR 0023: trace で元の決定から修正ADRをたどる](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0023-trace-amendments.md)
- [adr-guard の ADR 0024: 移行スクリプトに --dry-run を設け、失敗を最後まで数える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0024-migration-dry-run-and-error-count.md)
- [adr-guard の ADR 0025: 新形式のステータス行に、実装待ちの修正、補足、注記、別のリポジトリの修正先を加える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0025-extend-canonical-status-amendments.md)
- [adr-guard の ADR 0026: 旧形式のステータス行の修正先を、リンク先と裸の番号からも読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0026-read-more-legacy-references.md)
- [adr-guard の ADR 0027: 旧形式の `置換（…）` を、修正を受けた状態として読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0027-read-legacy-replaced-status.md)
- [adr-guard の ADR 0028: 旧形式の部分名から修飾子と区切りの記号を除き、決められない部分名を移行しない](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0028-fix-legacy-part-names.md)
- adr-guard の README(v0.13.1)
  https://github.com/tett23/adr-guard/blob/v0.13.1/README.md
- adr-guard v0.11.0、v0.12.0、v0.13.0、v0.13.1 のリリース
  https://github.com/tett23/adr-guard/releases/tag/v0.11.0
  https://github.com/tett23/adr-guard/releases/tag/v0.12.0
  https://github.com/tett23/adr-guard/releases/tag/v0.13.0
  https://github.com/tett23/adr-guard/releases/tag/v0.13.1
