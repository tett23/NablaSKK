# ADR 0004: リポジトリの Claude Code の hook を外す

ステータス: 採択

## 文脈

[ADR 0001](0001-adopt-global-working-rules.md) で、adr-guard を三か所に導入した。

1. git の pre-commit hook(`.githooks/pre-commit`)
2. Claude Code の PreToolUse hook(リポジトリの `.claude/settings.json`)
3. CI の `adr` ジョブ

その後、作業者の全体の設定 `~/.claude/settings.json` に、同じ hook
(`Write|Edit|MultiEdit` に `adr-guard claude-hook`)が入った。グローバルの
`~/.claude/CLAUDE.md` も「Claude Code の hook は全体の設定で有効なので、リポジトリには
不要」と改めた。

いまは二つの設定が同じ hook を持っており、Claude Code で編集するたびに adr-guard が
二回実行される。中身は同一で、リポジトリ側を残す理由は無い。

`.claude/settings.json` に入っているのはこの hook だけで、ほかの設定は無い。

## 実装すること

- リポジトリの `.claude/settings.json` を削除する(`.claude/` ディレクトリごと。
  ほかにファイルは無い)。
- [ADR 0001](0001-adopt-global-working-rules.md) のステータスの行を「採択→Claude Code の hook(`.claude/settings.json`)は
  ADR 0004」に変える(ステータスの行だけの変更)。
- README の開発の節から「Claude Code の hook は `.claude/settings.json` に入っています」
  を外し、Claude Code で編集する場合は全体の設定(`~/.claude/settings.json`)に
  adr-guard の hook を入れる旨を書く(adr-guard の README の例へのリンク)。
- `docs/specifications.md` の開発の節を「git の pre-commit hook と CI で検査する。
  Claude Code の hook は利用者の全体の設定で入れる」に直す。

## 実装しないこと

- git の pre-commit hook と CI の `adr` ジョブの変更(そのまま残す)。
- 作業者の全体の設定(`~/.claude/settings.json`)の変更。
- [ADR 0001](0001-adopt-global-working-rules.md) の本文の変更(ステータスの行以外は変えない)。

## テスト設計

- 削除後に、Claude Code(このリポジトリで動く Claude)からコミット済みの [ADR 0001](0001-adopt-global-working-rules.md) の
  本文を Edit しようとすると、全体の設定の hook によって拒否されること(実際に
  Edit を試みて確かめる)。
- 新しい ADR の作成や、ADR 以外のファイルの編集は止まらないこと。
- コミット済みの ADR の本文を書き換えたコミットを、pre-commit hook が引き続き拒否する
  こと(一時ディレクトリのクローンで確かめる)。
- [ADR 0001](0001-adopt-global-working-rules.md) のステータスの行の変更が pre-commit hook と CI の `adr` ジョブを通ること。
- CI がすべて成功すること。

## トレードオフ

- 全体の設定に hook を入れていない人が Claude Code でこのリポジトリを編集すると、
  Claude Code 側での検査が無くなる。ただし pre-commit hook と CI で必ず検査される
  ので、コミット済みの ADR が書き換わったまま main に入ることは無い。README に
  全体の設定への入れ方を書いて補う。
- リポジトリだけを見ても Claude Code の hook の有無が分からなくなる。作業者の環境に
  合わせた規則(グローバルの `~/.claude/CLAUDE.md`)を優先した。
