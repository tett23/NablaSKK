# ADR 0017: シンボリックリンクの設定ファイルをリンクのまま保つ

ステータス: 採択

## 文脈

作業者は、`~/Library/Application Support/NablaSKK/` の設定ファイルを home-manager で
管理している。`dictionaries.conf`、`kana-rule.conf`、`keymap.conf`、`settings.conf` は
シンボリックリンクで、Nix ストアのリンクを経て `~/dotfiles/nablaskk/` の実ファイルを指す
(`mkOutOfStoreSymlink` の形。リンク先は書き込める)。

設定アプリでローマ字ルールを編集したところ、`kana-rule.conf` がリンクから通常の
ファイルに置き換わっていた(中身は編集した内容、権限は 755)。dotfiles 側のファイルには
編集が入らず、home-manager の管理からも外れた。

原因は、保存のしかたにある。設定アプリは設定ファイルを atomic に書く
(`String.write(to:atomically: true)`、`Data.write(to:options: .atomic)`)。
これは同じディレクトリの一時ファイルに書いてから、元の名前へ rename するので、パスが
シンボリックリンクならリンクそのものが置き換わる。一時ディレクトリでリンクを作って
`write(to:atomically: true)` で書くと、リンクが通常のファイル(権限 755)になり、
リンク先は変わらないことを確かめた。

同じ形の保存は、ほかにもある。

- 設定アプリ: `InputSettings.save`(`settings.conf`)、`KeymapSettings.save`
  (`keymap.conf`)、`KanaRuleSettings.save`(`kana-rule.conf`)、
  `DictionaryConfig.save`(`dictionaries.conf`)、`UserDictionary.save`
  (`skk-jisyo`)。
- エンジン: ユーザー辞書の保存(`LocalUserDictionary::save`。`skk-jisyo.tmp` に書いて
  rename)と、自動更新辞書のダウンロード(`AutoUpdateDictionary`。`.download` に
  書いて rename)。本家 AquaSKK も同じ書き方(`SKKLocalUserDictionary`)で、リンクを
  置き換える。

もう 1 つ、変更の検知にも問題がある。IME と設定アプリは、設定ファイルが変わったかを
`FileManager.attributesOfItem(atPath:)` の更新日時で見ている。このメソッドは末尾の
シンボリックリンクをたどらず、リンク自身の属性を返す(試して、種類が
`NSFileTypeSymbolicLink` になることを確かめた)。そのため、リンク先のファイルを手で
編集しても、IME は設定を読み直さない。

## 実装すること

- **保存はリンクの先へ**: 設定ファイルとユーザー辞書を保存するときは、パスの
  シンボリックリンクをすべてたどった先(最後の実ファイル)に、今までどおり atomic に書く。
  一時ファイルはリンク先と同じディレクトリに作るので、rename は同じファイルシステムの
  中で行われる。リンクそのものは変えない。
  - Swift(`macos/Shared/`): 書き込む URL を `resolvingSymlinksInPath()` で解決する
    小さな関数を 1 つ置き、上の 5 つの保存で使う。
  - Rust(`nablaskk-core`): パスが存在するときは `std::fs::canonicalize` で解決し、
    一時ファイルの場所と rename の先にする。存在しないとき(初めての保存)は今までどおり。
    ユーザー辞書と自動更新辞書の両方で使う。
- **書けないときはリンクを残す**: リンク先が書けない場所(Nix ストアの中の実ファイル
  など)なら保存は失敗する。リンクを通常のファイルで置き換えて書き込みを通すことは
  しない。設定アプリは、今の保存の失敗と同じく、ページにエラーを出す。エンジンは今と
  同じく標準エラーに書き、未保存(dirty)のまま残す。
- **変更の検知もリンクの先で**: 設定ファイルとユーザー辞書の更新日時は、リンクを
  たどった先のファイルのものを使う(Swift の `modificationDate` と、設定アプリの
  ユーザー辞書ページ)。エンジンの `modified_time` は `std::fs::metadata` で、もとから
  リンクをたどるので変えない。
- 仕様書に、設定ファイルとユーザー辞書がシンボリックリンクなら、リンク先に保存し、
  リンク先の変更も検知することを書く。README(日本語と英語)にも、home-manager などで
  リンクにしてよいことを書く。

## 実装しないこと

- すでに通常のファイルに置き換わった作業者の `kana-rule.conf` を、アプリが直すこと。
  作業者が home-manager で戻す(`kana-rule.conf` と `kana-rule.conf.hm-backup` の
  扱いは作業者が決める)。
- サポートディレクトリそのものや、`dictionaries/` ディレクトリがリンクである場合の
  特別な扱い。今も、パスの途中のリンクはふつうにたどられるので、問題は起きない。
- 辞書の取り込み(`importDictionary`)の書き方を変えること。取り込んだ辞書は設定アプリが
  作るコピーで、利用者がリンクにするものではない。
- 書き込めないリンク先のために、別の場所へ保存する仕組み。

## テスト設計

先に書いて失敗させてから実装する。

- Rust(`cargo test`):
  - 一時ディレクトリに実ファイルとそれを指すリンクを作り、リンクのパスでユーザー辞書を
    開いて単語を登録し、保存する。保存のあと、パスがリンクのままで、リンク先のファイルに
    単語が入っていること。
  - リンクが 2 段(リンク → リンク → 実ファイル)でも同じであること。
  - リンク先のディレクトリを書き込み禁止にすると、保存が失敗し、リンクはそのままで、
    辞書は未保存のまま残ること。
  - パスが存在しない(初めての保存)ときは、今までどおり通常のファイルができること。
  - 自動更新辞書は、ダウンロードの結果をリンク先に書くこと(ネットワークを使わずに試せる
    ように、ダウンロードした中身を置き換える部分を関数に分けて試す)。
- Swift(`sh macos/run-tests.sh`):
  - `settings.conf`、`keymap.conf`、`kana-rule.conf`、`dictionaries.conf`、
    ユーザー辞書のそれぞれで、パスをリンクにして保存すると、リンクのままでリンク先の中身が
    変わること。2 段のリンクでも同じであること。
  - リンク先のディレクトリが書けないと保存がエラーになり、リンクが残ること。
  - リンク先のファイルの更新日時が変わると、`modificationDate` も変わること。
- 本家の受け入れテスト(`session_test.dat`、400 ケース)を含め、既存の検証
  (`cargo test`、`cargo clippy --all-targets`、`sh swift/run-smoke-test.sh`、
  `sh macos/run-tests.sh`、`sh macos/run-ui-tests.sh`、`sh macos/build-app.sh`、
  `sh macos/check-bundle.sh`)が通ること。
- 実機で作業者が確かめる:
  - home-manager で戻したリンクの `kana-rule.conf` のまま、設定アプリでルールを
    編集すると、リンクが残り、`~/dotfiles/nablaskk/kana-rule.conf` に編集が入ること。
  - `~/dotfiles/nablaskk/` の設定ファイルを手で編集すると、IME が読み直すこと。

## トレードオフ

- 本家はユーザー辞書を置き換える書き方で、リンクを残さない。NablaSKK は、dotfiles で
  設定や辞書を管理する使い方を優先して、本家と変える。
- リンク先が書けないときは保存できない。リンクを置き換えれば保存は通るが、利用者が
  dotfiles で管理しているつもりのファイルが黙って管理から外れるので、エラーにする。
  home-manager が Nix ストアの実ファイルを指すリンクを置く設定(`mkOutOfStoreSymlink`
  を使わない形)では、設定アプリで編集できなくなるが、それは home-manager で管理すると
  決めたことの結果として受け入れる。
- 保存のたびにパスを解決するので、わずかに手間が増える。保存は頻繁ではないので
  問題にならない。

## 参考文献

- Apple Developer Documentation, `FileManager.attributesOfItem(atPath:)`(末尾の
  シンボリックリンクをたどらない)
  https://developer.apple.com/documentation/foundation/filemanager/attributesofitem(atpath:)
- Apple Developer Documentation, `URL.resolvingSymlinksInPath()`
  https://developer.apple.com/documentation/foundation/url/resolvingsymlinksinpath()
- Rust 標準ライブラリ, `std::fs::canonicalize`、`std::fs::rename`
  https://doc.rust-lang.org/std/fs/fn.canonicalize.html
- AquaSKK `src/engine/dictionary/SKKLocalUserDictionary.cpp`
  https://github.com/codefirst/aquaskk
- home-manager, `config.lib.file.mkOutOfStoreSymlink`
  https://nix-community.github.io/home-manager/
