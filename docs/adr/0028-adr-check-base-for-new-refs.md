# ADR 0028: CI の adr ジョブを adr-guard v0.10.0 に上げ、タグや新しいブランチの push では main との分岐点から検査する

ステータス: 採択（未実装）

## 文脈

CI の `adr` ジョブ(`.github/workflows/ci.yml`)は、`tett23/adr-guard@v0.8.2` で、コミット済みの
ADR がステータスの行とリンク以外で変わっていないことを検査する([ADR 0001](0001-adopt-global-working-rules.md))。
検査の範囲は次のとおりである。

- pull request: `base` は PR の base の SHA。
- push: `base` は `github.event.before`(push する前の ref の SHA)。

`v` タグを push すると、このジョブが毎回失敗する。v1.3.0(run 42341089804)と v1.4.0
(run 42537687161)の両方で失敗した。同じコミットを main に push したときの run は通っている。

原因は次のとおりである。

- 新しい ref(タグ、新しいブランチ)の push では、`github.event.before` が
  `0000000000000000000000000000000000000000` になる。
- adr-guard の `check-range` は、`base` がコミットとして存在しないと、範囲を `base..head` ではなく
  `head` から辿れるすべてのコミットにする(adr-guard の `src/main.rs` の `check_range`)。
- 全履歴には、重複した ADR の番号を振り直したコミット(f24d32f「Renumber the duplicate ADRs 0016 and
  0017 to 0020 and 0021」)がある。このコミットは、コミット済みの ADR を意図して移動・変更している。
  そのため、全履歴を検査すると必ずここで失敗する。

ジョブの失敗は、リリースそのものを止めない。`release` ジョブは `adr` に依存していない
(`needs: [build-linux, build-macos, build-wasm]`)。それでも、タグの run が毎回「失敗」になる。
そのため、本当の失敗と見分けられなくなる。

新しいブランチを push したときも、同じ理由で失敗する。今はブランチを手元でマージしてから main だけを
push しているので、表に出ていない。

また、CI の adr-guard は v0.8.2 のままで古い。最新は v0.10.0(2026-11-07)である。手元の pre-commit の
hook は、手元に入っている adr-guard(0.10.0)で動いているので、CI と手元で版がずれている。v0.8.2 から
v0.10.0 までに、次が入った(adr-guard の README と `action.yml`)。

- v0.8.3: ステータス「採択（未実装）」と `unimplemented` サブコマンド。
- v0.9.0: ADR の番号の重複の検査(`check-numbers`)と振り直し(`renumber`)。action の入力
  `check-numbers`(既定 true)と `allow-renumber`(既定 false)。
- v0.10.0: モジュール名つきの ADR の参照のリンク、シンボリックリンクの文書をリンクの検査から外すこと。

このリポジトリでは、手元の 0.10.0 で `check-numbers`、`check-status`、`links` が通る。

## 実装すること

`adr` ジョブで、adr-guard に渡す `base` を求める step を、adr-guard の前に足す。

- pull request: 今と同じく PR の base の SHA。
- push で、`github.event.before` が 0 でない: 今と同じく `github.event.before`。
- push で、`github.event.before` が 0(新しいタグ、新しいブランチ): `git merge-base origin/main <head>`。
  - タグが main の上のコミットを指すとき、分岐点はそのコミット自身になる。範囲は空になり、
    `check-range` はコミットを 1 つも検査しない。そのコミットまでは、main への push のときに
    検査してある。
  - 新しいブランチでは、main から分かれた後のコミットだけを検査する。
- `actions/checkout` はすでに `fetch-depth: 0` なので、`origin/main` はある。

求めた値は step の出力にして、adr-guard の `base` に渡す。`head` は今のままにする。

あわせて、`tett23/adr-guard@v0.8.2` を `tett23/adr-guard@v0.10.0` に上げる。入力は今のまま
(`allow-status: true`、`allow-links: true`)にする。`check-numbers` は既定の true のまま使う。

`docs/specifications.md` の開発とリリースの節(10 節)に、タグや新しいブランチの push では main との
分岐点から検査することを書く。

## 実装しないこと

- タグの push で `adr` ジョブを飛ばすこと(`if: github.ref_type != 'tag'`)。より簡単だが、新しい
  ブランチの push の失敗は残る。
- `allow-renumber: true` にすること。f24d32f は番号の振り直しだけでなく ADR の本文も変えているので、
  付けても全履歴の検査は通らない。また、これから番号を振り直す予定は無い。
- adr-guard 自体を変えること(0 の SHA の扱い)。どこから検査すべきかはリポジトリの運用で決まるので、
  CI の側で決める。
- f24d32f を書き換えること。main を rebase しない規則がある。

## テスト設計

CI の設定なので、自動テストは書かない。次のように確かめる。

- 実装の前に、手元で今の失敗を再現する。手元の adr-guard(0.10.0)で
  `adr-guard check-range --allow-status --allow-links 0000000000000000000000000000000000000000 v1.4.0`
  が、f24d32f で失敗することを確かめる。
- 上げる前に、手元の adr-guard 0.10.0 で `adr-guard check-numbers`、`adr-guard check-status`、
  `adr-guard links` が通ることを確かめる(済み)。
- 実装の後、step と同じ計算を手元のシェルで行い、次を確かめる。
  - v1.4.0 では、分岐点が v1.4.0 のコミット自身になる。その値で `check-range` を実行すると成功すること。
  - main から分けたブランチでは、分岐点が分かれたところになること。
- main に push した run で、`adr` ジョブが成功すること。
- 新しいブランチを push して、その run の `adr` ジョブが成功することを確かめる(確かめたらブランチを消す)。
- 次に `v` タグを push したとき(次のリリース)、`adr` ジョブが成功することを確かめる。

## トレードオフ

- タグが main に無いコミットを指すときは、main から分かれた後のコミットだけを検査する。タグは main の
  マージコミットに付ける運用なので、困らない。
- main が無いリポジトリでは動かない。このリポジトリは main がある。
- step を 1 つ足すので、ジョブが少し長くなる。

## 参考文献

- adr-guard `src/main.rs` の `check_range`、`action.yml`(v0.8.2)
  https://github.com/tett23/adr-guard
- GitHub Docs, Events that trigger workflows: push(`before` は新しい ref のとき 0 の SHA になる)
  https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#push
- CI の run 42341089804(v1.3.0)、42537687161(v1.4.0)
