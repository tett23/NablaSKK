# ADR 0002: C ヘッダの辞書タイプに skkserv(UTF-8)を宣言する

ステータス: 採択

## 文脈

辞書タイプ 6(skkserv、UTF-8)は v0.2.x で NablaSKK 独自に追加した。エンジン
(`DictionaryType::ProxyUtf8`)、`skk_session_add_dictionary` の受け付け、
Swift ラッパー(`SKKSession.DictionaryType.proxyUTF8`)、設定アプリ
(`DictionaryEntry.Kind.proxyUTF8`)、`docs/specifications.md` はすべて 6 を扱う。

一方、`crates/nablaskk-ffi/include/nablaskk.h` の辞書タイプの列挙には 6 が無い。

```c
/* Dictionary types (DictionarySet numbering of the original AquaSKK). */
enum {
    SKK_DICT_COMMON = 0,
    SKK_DICT_AUTO_UPDATE = 1,
    SKK_DICT_PROXY = 2,       /* "host:port" of a running skkserv */
    SKK_DICT_GADGET = 4,
    SKK_DICT_COMMON_UTF8 = 5
};
```

- `skk_session_add_dictionary` の関数コメントには 6 が書いてあり、列挙だけが古い。
- C / Objective-C から使う人は、6 を数値で直接書くしかない。
- `SKK_DICT_PROXY` のコメントに、通信が EUC-JP であることが書かれていない。
- 列挙のコメントは「本家の番号」としか言っておらず、6 が NablaSKK 独自であることが分からない。
- ヘッダと Swift ラッパーの値が一致していることを確かめるテストが無いので、今回のずれに気付けなかった。

仕様書(ADR 0001)は FFI の正を `nablaskk.h` としているので、ヘッダを実装に合わせる。

## 実装すること

- `nablaskk.h` の列挙に `SKK_DICT_PROXY_UTF8 = 6` を追加する。コメントは「"host:port" of a running skkserv speaking UTF-8 (NablaSKK addition)」。
- `SKK_DICT_PROXY` のコメントに、EUC-JP であることを書く。
- 列挙の前のコメントを「本家 DictionarySet の番号。6 は NablaSKK 独自」とする。
- `swift/smoke-test` に、ヘッダの `SKK_DICT_*` と `SKKSession.DictionaryType` の値が一致することの検査を追加する(0, 1, 2, 4, 5, 6 のすべて)。あわせて `SKK_MODE_*` と `SKKSession.InputMode`、`SKK_OPTION_*` と `SKKSession.Option` も同じ検査に含める。
- この変更はヘッダへの追加だけで、ABI と既存の値は変わらない。リリースノートでは「追加」に書く。

## 実装しないこと

- 辞書タイプの番号の変更や、3(ことえり、未対応)の追加。
- Swift ラッパーや設定アプリを、数値ではなくヘッダの定数を使う形に書き換えること(Swift 側は今も値で一致させていて、今回のテストで一致を保証する)。
- `docs/specifications.md` の変更(すでに 6 を載せている)。

## テスト設計

- `sh swift/run-smoke-test.sh` に追加する検査:
  - `SKK_DICT_COMMON` 〜 `SKK_DICT_PROXY_UTF8` が、それぞれ `SKKSession.DictionaryType` の同名のケースの `rawValue` と等しい。
  - `SKK_MODE_*` が `SKKSession.InputMode` と、`SKK_OPTION_*` が `SKKSession.Option` と等しい。
  - `SKK_DICT_PROXY_UTF8` を渡した `addDictionary` が成功を返す(接続できなくても辞書の追加自体は成功する)。
- 追加前のヘッダで同じテストを動かすと、`SKK_DICT_PROXY_UTF8` が未定義でコンパイルに失敗することを確かめる(テストがずれを検出できることの確認)。
- 既存の検証(`cargo test`、`cargo clippy --all-targets`、`sh macos/run-tests.sh`、`sh macos/build-app.sh`)がすべて通ること。

## トレードオフ

- 値の一致を実行時の検査で保証するので、ずれはスモークテストを動かすまで分からない。ヘッダから Swift の列挙を自動生成すれば構造的に防げるが、生成の仕組みを持ち込むほどの規模ではない。CI で毎回スモークテストを動かしているので、push の時点で検出できる。
- 列挙の定数を増やすだけなので、既存の利用者への影響は無い。
