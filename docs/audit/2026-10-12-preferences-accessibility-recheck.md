# 設定アプリのアクセシビリティ再監査

- 実施日: 2026-10-12
- 対象: [ADR 0006](../adr/0006-preferences-accessibility-fixes.md)(v1.1.1)と [ADR 0007](../adr/0007-preferences-accessibility-followups.md) の対応を入れた設定アプリ(`preferences-a11y-followup`
  ブランチ、v1.1.2 として公開する版)
- 元の監査: `docs/audit/2026-10-12-preferences-accessibility.md`(この記録では変更しない)
- この記録はコミット後に変更しない。

## 方法

| 方法 | 実施者 | 内容 |
|---|---|---|
| 自動テスト | Claude | `sh macos/run-ui-tests.sh`(画面外のウィンドウへのクリック・キー入力と画素の判定)、`sh macos/run-tests.sh`(読み上げラベルの文言)。CI の macOS ジョブでも実行 |
| 実機での確認 | 作業者 | VoiceOver(Command + F5)とフルキーボードアクセスで、[ADR 0007](../adr/0007-preferences-accessibility-followups.md) のテスト設計の「実機での確認」の手順を実施 |

## 結果

### 実機での確認(作業者)

[ADR 0007](../adr/0007-preferences-accessibility-followups.md) の手順をすべて実施し、問題なしとの報告を受けた。

| 項目 | 結果 |
|---|---|
| サイドバーで項目を選んだあと、↑↓ キーでページが移る | 問題なし |
| VoiceOver: サイドバーが「設定のページ」として読まれ、各項目の「n / 6」と選択状態が読まれる | 問題なし |
| VoiceOver: アイコンだけのボタン・辞書の行の部品・行を選ぶボタンが日本語で読まれ、装飾のアイコンが読まれない | 問題なし |
| フルキーボードアクセス: Tab で各ページの部品をたどれ、行を選ぶボタンで行を選んで ↑↓・削除のボタンを使える | 問題なし |
| つまみのドラッグでの並べ替え、ファイルのドロップでの辞書の追加、一覧の入力欄への 1 回のクリックでの入力([ADR 0006](../adr/0006-preferences-accessibility-fixes.md) で依頼済み) | 問題なし(v1.1.1 の確認時) |

### 自動テスト

| 項目 | 結果 |
|---|---|
| 注意・エラー・補足の文字のコントラスト(ライト・ダーク × ウィンドウ・一覧・赤い行の背景) | すべて 4.5:1 以上(最小はライトの注意の赤い行の上で 4.67:1) |
| 選択中のサイドバーの項目(白い文字 / 背景) | 5.05:1(既定のアクセント色) |
| 一覧の入力欄(辞書の場所・ユーザー辞書の候補・ローマ字ルール)への 1 回のクリックでの入力 | 合格 |
| 一覧の行のクリックでの選択、サイドバーの強調 | 合格 |
| 見つからない辞書ファイル・誤ったキーの文での表示、上書きしたキーの既定値の表示 | 合格 |
| 無効化した辞書の名前の色(補足の色。不透明度 0.5 で薄めていない) | 合格 |
| 組み込みのルールを文字で表示 | 合格 |
| サイドバーの ↑↓ キー(端では止まる) | 合格 |
| 読み上げラベルの文言 | 合格 |

## 元の監査の指摘の状況

| 指摘 | 重大度 | 状況 |
|---|---|---|
| A-1 アイコンだけのボタンに日本語のラベルがない | 高 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| A-2 辞書の有効化チェックボックスと種類に名前がない | 高 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| A-3 入力欄の名前がプレースホルダー頼み | 中 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| A-4 サイドバーが一覧として読まれない | 中 | 対応済み([ADR 0007](../adr/0007-preferences-accessibility-followups.md))。`List` の意味は持たないが、まとまりの名前・「n / 6」・↑↓ キーで補った |
| A-5 装飾のアイコンが読み上げ対象になりうる | 低 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-1 注意文のコントラスト | 高 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-2 補足文のコントラスト | 中 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-3 エラー文のコントラスト | 中 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-4 読み取り専用のルールを無効の欄で表示 | 中 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-5 選択中のサイドバーのコントラスト | 低 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-6 無効化した辞書の行が薄い | 低 | 対応済み([ADR 0007](../adr/0007-preferences-accessibility-followups.md)) |
| C-7 辞書の場所の誤りを色とツールチップだけで示す | 高 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-8 キーの誤りを色とツールチップだけで示す | 高 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| C-9 補足の情報がツールチップだけにある | 低 | 対応済み([ADR 0007](../adr/0007-preferences-accessibility-followups.md))。辞書名のツールチップは [ADR 0006](../adr/0006-preferences-accessibility-fixes.md) で削除(名前は読み上げのラベルに含む) |
| O-1 一覧の入力欄に 1 回目のクリックでフォーカスが入らない | 中 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)) |
| O-3 キーボードだけでの操作(未確認) | — | 実機で確認済み(本記録) |
| T-1 10pt の文字の多用 | 低 | 対応済み([ADR 0006](../adr/0006-preferences-accessibility-fixes.md)、[ADR 0007](../adr/0007-preferences-accessibility-followups.md)) |

## 残っていること

- macOS の文字サイズ設定への追従(macOS の仕組みとして動的な文字サイズがない)。
- アクセント色を変えている利用者での、選択中のサイドバーのコントラスト(既定の青でのみ計算)。
- 新しいページや部品を足すときは、`PreferencesPalette` の色と `PreferencesAccessibility` の
  ラベルを使い、UI テストに検査を足す。
