# upstream-patches

Rust への移植中に見つけた本家 [codefirst/aquaskk](https://github.com/codefirst/aquaskk)
側の不具合に対する修正パッチです。

## 0001-jconv-fix-combining-character-emit.patch

`eucj_to_utf8::emit()` で、2 文字に分解される JIS X 0213 文字
(例: か゚ = U+304B U+309A)の 2 文字目のマスクが `0xfff` になっており、
合成マーク U+309A が U+009A として出力されてしまう。`0xffff` が正しい。

適用方法:

```sh
cd aquaskk
git apply ../upstream-patches/0001-jconv-fix-combining-character-emit.patch
```
