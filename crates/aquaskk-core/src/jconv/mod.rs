//! Japanese character conversion utilities (port of `jconv`).
//!
//! Includes a dependency-free EUC-JP(JIS X 0213) <-> UTF-8 codec driven by
//! tables generated from the original `jconv_eucj2ucs-inl.h`, plus the kana /
//! latin translation tables, which encode SKK-specific romaji spellings
//! (e.g. っ = "tt", ん = "nn") relied on by entry normalization.

mod eucjp_tables;

use eucjp_tables::{EUC_TO_UCS, UCS_TO_EUC};

/// U+3013 GETA MARK, used for undecodable sequences.
const SUBST_CHAR: char = '\u{3013}';
/// EUC-JP code of the geta mark, used for unencodable characters.
const SUBST_EUC: u32 = 0xa2ae;

fn euc_to_ucs(euc: u32) -> Option<u32> {
    EUC_TO_UCS
        .binary_search_by_key(&euc, |&(k, _)| k)
        .ok()
        .map(|index| EUC_TO_UCS[index].1)
}

fn ucs_to_euc(ucs: u32) -> Option<u32> {
    UCS_TO_EUC
        .binary_search_by_key(&ucs, |&(k, _)| k)
        .ok()
        .map(|index| UCS_TO_EUC[index].1)
}

fn push_ucs(out: &mut String, packed: u32) {
    // Values >= 0x100000 pack two scalars (kana + combining mark).
    if packed >= 0x100000 {
        push_scalar(out, packed >> 16);
        push_scalar(out, packed & 0xffff);
    } else {
        push_scalar(out, packed);
    }
}

fn push_scalar(out: &mut String, ucs: u32) {
    out.push(char::from_u32(ucs).unwrap_or(SUBST_CHAR));
}

/// Decode EUC-JP (JIS X 0213) bytes to UTF-8, substituting undecodable
/// sequences with U+3013.
pub fn utf8_from_eucj(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            0x00..=0x7f => {
                out.push(b as char);
                i += 1;
            }
            0x8e => {
                // JIS X 0201 kana
                match bytes.get(i + 1) {
                    Some(&b2 @ 0xa1..=0xdf) => {
                        push_scalar(&mut out, 0xff61 + (b2 - 0xa1) as u32);
                        i += 2;
                    }
                    _ => {
                        out.push(SUBST_CHAR);
                        i += 1;
                    }
                }
            }
            0x8f => {
                // JIS X 0213 plane 2
                match (bytes.get(i + 1), bytes.get(i + 2)) {
                    (Some(&b1 @ 0xa1..=0xfe), Some(&b2 @ 0xa1..=0xfe)) => {
                        let key = 0x8f0000 | ((b1 as u32) << 8) | b2 as u32;
                        match euc_to_ucs(key) {
                            Some(ucs) => push_ucs(&mut out, ucs),
                            None => out.push(SUBST_CHAR),
                        }
                        i += 3;
                    }
                    _ => {
                        out.push(SUBST_CHAR);
                        i += 1;
                    }
                }
            }
            0xa1..=0xfe => {
                // JIS X 0213 plane 1
                match bytes.get(i + 1) {
                    Some(&b2 @ 0xa1..=0xfe) => {
                        let key = ((b as u32) << 8) | b2 as u32;
                        match euc_to_ucs(key) {
                            Some(ucs) => push_ucs(&mut out, ucs),
                            None => out.push(SUBST_CHAR),
                        }
                        i += 2;
                    }
                    _ => {
                        out.push(SUBST_CHAR);
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(SUBST_CHAR);
                i += 1;
            }
        }
    }

    out
}

fn push_euc(out: &mut Vec<u8>, euc: u32) {
    if euc & 0xff0000 == 0x8f0000 {
        out.push(0x8f);
    }
    out.push((euc >> 8) as u8);
    out.push((euc & 0xff) as u8);
}

/// Encode UTF-8 to EUC-JP (JIS X 0213) bytes, substituting unencodable
/// characters with the geta mark.
pub fn eucj_from_utf8(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        let ucs = c as u32;

        if ucs < 0x80 {
            out.push(ucs as u8);
            continue;
        }

        // JIS X 0201 kana
        if (0xff61..=0xff9f).contains(&ucs) {
            out.push(0x8e);
            out.push((0xa1 + (ucs - 0xff61)) as u8);
            continue;
        }

        // Kana + combining mark pairs
        if let Some(&next) = chars.peek() {
            let next_ucs = next as u32;
            if ucs < 0x10000 && next_ucs < 0x10000 {
                if let Some(euc) = ucs_to_euc((ucs << 16) | next_ucs) {
                    push_euc(&mut out, euc);
                    chars.next();
                    continue;
                }
            }
        }

        match ucs_to_euc(ucs) {
            Some(euc) => push_euc(&mut out, euc),
            None => push_euc(&mut out, SUBST_EUC),
        }
    }

    out
}

struct Kana {
    hirakana: &'static str,
    katakana: &'static str,
    jisx0201kana: &'static str,
    roman: &'static str,
}

macro_rules! kana {
    ($h:literal, $k:literal, $j:literal, $r:literal) => {
        Kana { hirakana: $h, katakana: $k, jisx0201kana: $j, roman: $r }
    };
}

/// Voiced / semi-voiced kana. Must be applied before the secondary table so
/// that multi-byte JIS X 0201 sequences ("ｶﾞ") win over their prefix ("ｶ").
static PRIMARY_KANA_TABLE: &[Kana] = &[
    kana!("が", "ガ", "ｶﾞ", "ga"), kana!("ぎ", "ギ", "ｷﾞ", "gi"), kana!("ぐ", "グ", "ｸﾞ", "gu"), kana!("げ", "ゲ", "ｹﾞ", "ge"), kana!("ご", "ゴ", "ｺﾞ", "go"),
    kana!("ざ", "ザ", "ｻﾞ", "za"), kana!("じ", "ジ", "ｼﾞ", "zi"), kana!("ず", "ズ", "ｽﾞ", "zu"), kana!("ぜ", "ゼ", "ｾﾞ", "ze"), kana!("ぞ", "ゾ", "ｿﾞ", "zo"),
    kana!("だ", "ダ", "ﾀﾞ", "da"), kana!("ぢ", "ヂ", "ﾁﾞ", "di"), kana!("づ", "ヅ", "ﾂﾞ", "du"), kana!("で", "デ", "ﾃﾞ", "de"), kana!("ど", "ド", "ﾄﾞ", "do"),
    kana!("ば", "バ", "ﾊﾞ", "ba"), kana!("び", "ビ", "ﾋﾞ", "bi"), kana!("ぶ", "ブ", "ﾌﾞ", "bu"), kana!("べ", "ベ", "ﾍﾞ", "be"), kana!("ぼ", "ボ", "ﾎﾞ", "bo"),
    kana!("ぱ", "パ", "ﾊﾟ", "pa"), kana!("ぴ", "ピ", "ﾋﾟ", "pi"), kana!("ぷ", "プ", "ﾌﾟ", "pu"), kana!("ぺ", "ペ", "ﾍﾟ", "pe"), kana!("ぽ", "ポ", "ﾎﾟ", "po"),
    kana!("う゛", "ヴ", "ｳﾞ", "vu"),
];

static SECONDARY_KANA_TABLE: &[Kana] = &[
    kana!("あ", "ア", "ｱ", "a"), kana!("い", "イ", "ｲ", "i"), kana!("う", "ウ", "ｳ", "u"), kana!("え", "エ", "ｴ", "e"), kana!("お", "オ", "ｵ", "o"),
    kana!("か", "カ", "ｶ", "ka"), kana!("き", "キ", "ｷ", "ki"), kana!("く", "ク", "ｸ", "ku"), kana!("け", "ケ", "ｹ", "ke"), kana!("こ", "コ", "ｺ", "ko"),
    kana!("さ", "サ", "ｻ", "sa"), kana!("し", "シ", "ｼ", "si"), kana!("す", "ス", "ｽ", "su"), kana!("せ", "セ", "ｾ", "se"), kana!("そ", "ソ", "ｿ", "so"),
    kana!("た", "タ", "ﾀ", "ta"), kana!("ち", "チ", "ﾁ", "ti"), kana!("つ", "ツ", "ﾂ", "tu"), kana!("て", "テ", "ﾃ", "te"), kana!("と", "ト", "ﾄ", "to"),
    kana!("な", "ナ", "ﾅ", "na"), kana!("に", "ニ", "ﾆ", "ni"), kana!("ぬ", "ヌ", "ﾇ", "nu"), kana!("ね", "ネ", "ﾈ", "ne"), kana!("の", "ノ", "ﾉ", "no"),
    kana!("は", "ハ", "ﾊ", "ha"), kana!("ひ", "ヒ", "ﾋ", "hi"), kana!("ふ", "フ", "ﾌ", "hu"), kana!("へ", "ヘ", "ﾍ", "he"), kana!("ほ", "ホ", "ﾎ", "ho"),
    kana!("ま", "マ", "ﾏ", "ma"), kana!("み", "ミ", "ﾐ", "mi"), kana!("む", "ム", "ﾑ", "mu"), kana!("め", "メ", "ﾒ", "me"), kana!("も", "モ", "ﾓ", "mo"),
    kana!("や", "ヤ", "ﾔ", "ya"), kana!("ゆ", "ユ", "ﾕ", "yu"), kana!("よ", "ヨ", "ﾖ", "yo"),
    kana!("ら", "ラ", "ﾗ", "ra"), kana!("り", "リ", "ﾘ", "ri"), kana!("る", "ル", "ﾙ", "ru"), kana!("れ", "レ", "ﾚ", "re"), kana!("ろ", "ロ", "ﾛ", "ro"),
    kana!("わ", "ワ", "ﾜ", "wa"), kana!("ゐ", "ヰ", "ｲ", "wi"), kana!("ゑ", "ヱ", "ｴ", "we"), kana!("を", "ヲ", "ｦ", "wo"), kana!("ん", "ン", "ﾝ", "nn"),
    kana!("ぁ", "ァ", "ｧ", "xa"), kana!("ぃ", "ィ", "ｨ", "xi"), kana!("ぅ", "ゥ", "ｩ", "xu"), kana!("ぇ", "ェ", "ｪ", "xe"), kana!("ぉ", "ォ", "ｫ", "xo"),
    kana!("っ", "ッ", "ｯ", "tt"), kana!("ゃ", "ャ", "ｬ", "xya"), kana!("ゅ", "ュ", "ｭ", "xyu"), kana!("ょ", "ョ", "ｮ", "xyo"), kana!("　", "　", " ", " "),
    kana!("。", "。", "｡", "。"), kana!("、", "、", "､", "､"), kana!("ー", "ー", "ｰ", "-"), kana!("「", "「", "｢", "｢"), kana!("」", "」", "｣", "｣"),
    kana!("゛", "゛", "ﾞ", "ﾞ"), kana!("゜", "゜", "ﾟ", "ﾟ"), kana!("ゎ", "ヮ", "ﾜ", "xwa"),
];

fn kana_convert(str_: &str, pick: impl Fn(&Kana) -> (&'static str, &'static str)) -> String {
    let mut result = str_.to_string();

    for table in [PRIMARY_KANA_TABLE, SECONDARY_KANA_TABLE] {
        for entry in table {
            let (from, to) = pick(entry);
            if result.contains(from) {
                result = result.replace(from, to);
            }
        }
    }

    result
}

pub fn hirakana_to_katakana(s: &str) -> String {
    kana_convert(s, |k| (k.hirakana, k.katakana))
}

pub fn hirakana_to_jisx0201_kana(s: &str) -> String {
    kana_convert(s, |k| (k.hirakana, k.jisx0201kana))
}

pub fn hirakana_to_roman(s: &str) -> String {
    kana_convert(s, |k| (k.hirakana, k.roman))
}

pub fn katakana_to_hirakana(s: &str) -> String {
    kana_convert(s, |k| (k.katakana, k.hirakana))
}

pub fn katakana_to_jisx0201_kana(s: &str) -> String {
    kana_convert(s, |k| (k.katakana, k.jisx0201kana))
}

pub fn katakana_to_roman(s: &str) -> String {
    kana_convert(s, |k| (k.katakana, k.roman))
}

pub fn jisx0201_kana_to_hirakana(s: &str) -> String {
    kana_convert(s, |k| (k.jisx0201kana, k.hirakana))
}

pub fn jisx0201_kana_to_katakana(s: &str) -> String {
    kana_convert(s, |k| (k.jisx0201kana, k.katakana))
}

pub fn jisx0201_kana_to_roman(s: &str) -> String {
    kana_convert(s, |k| (k.jisx0201kana, k.roman))
}

static JISX0208_LATIN_TABLE: &[&str] = &[
    "　", "！", "”", "＃", "＄", "％", "＆", "’", "（", "）", "＊", "＋", "，", "−", "．", "／",
    "０", "１", "２", "３", "４", "５", "６", "７", "８", "９", "：", "；", "＜", "＝", "＞", "？",
    "＠", "Ａ", "Ｂ", "Ｃ", "Ｄ", "Ｅ", "Ｆ", "Ｇ", "Ｈ", "Ｉ", "Ｊ", "Ｋ", "Ｌ", "Ｍ", "Ｎ", "Ｏ",
    "Ｐ", "Ｑ", "Ｒ", "Ｓ", "Ｔ", "Ｕ", "Ｖ", "Ｗ", "Ｘ", "Ｙ", "Ｚ", "［", "＼", "］", "＾", "＿",
    "‘", "ａ", "ｂ", "ｃ", "ｄ", "ｅ", "ｆ", "ｇ", "ｈ", "ｉ", "ｊ", "ｋ", "ｌ", "ｍ", "ｎ", "ｏ",
    "ｐ", "ｑ", "ｒ", "ｓ", "ｔ", "ｕ", "ｖ", "ｗ", "ｘ", "ｙ", "ｚ", "｛", "｜", "｝", "〜",
];

pub fn ascii_to_jisx0208_latin(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{20}'..='\u{7e}' => JISX0208_LATIN_TABLE[c as usize - 0x20].to_string(),
            _ => c.to_string(),
        })
        .collect()
}

pub fn jisx0208_latin_to_ascii(s: &str) -> String {
    let mut result = String::new();

    'outer: for c in s.chars() {
        let mut buf = [0u8; 4];
        let target = c.encode_utf8(&mut buf) as &str;
        for (index, latin) in JISX0208_LATIN_TABLE.iter().enumerate() {
            if *latin == target {
                result.push((0x20 + index as u8) as char);
                continue 'outer;
            }
        }
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_roundtrip() {
        assert_eq!(hirakana_to_katakana("がんばる"), "ガンバル");
        assert_eq!(katakana_to_hirakana("ガンバル"), "がんばる");
        assert_eq!(hirakana_to_jisx0201_kana("がんばる"), "ｶﾞﾝﾊﾞﾙ");
        assert_eq!(jisx0201_kana_to_hirakana("ｶﾞﾝﾊﾞﾙ"), "がんばる");
        assert_eq!(jisx0201_kana_to_katakana("ｶﾞﾝﾊﾞﾙ"), "ガンバル");
    }

    #[test]
    fn kana_to_roman() {
        assert_eq!(hirakana_to_roman("か"), "ka");
        assert_eq!(hirakana_to_roman("っ"), "tt");
        assert_eq!(hirakana_to_roman("ん"), "nn");
        assert_eq!(katakana_to_roman("リ"), "ri");
    }

    #[test]
    fn latin() {
        assert_eq!(ascii_to_jisx0208_latin("Abc 1!"), "Ａｂｃ　１！");
        assert_eq!(jisx0208_latin_to_ascii("Ａｂｃ　１！"), "Abc 1!");
    }

    #[test]
    fn eucj_roundtrip() {
        let euc = eucj_from_utf8("漢字かなｶﾅ ASCII 亜細亜");
        assert_eq!(utf8_from_eucj(&euc), "漢字かなｶﾅ ASCII 亜細亜");
    }

    #[test]
    fn eucj_known_bytes() {
        // "あ" = 0xA4A2, "亜" = 0xB0A1 in EUC-JP
        assert_eq!(utf8_from_eucj(&[0xa4, 0xa2]), "あ");
        assert_eq!(eucj_from_utf8("あ"), vec![0xa4, 0xa2]);
        assert_eq!(utf8_from_eucj(&[0xb0, 0xa1]), "亜");
        assert_eq!(eucj_from_utf8("亜"), vec![0xb0, 0xa1]);
        // JIS X 0201 kana "ｱ" = 0x8E B1
        assert_eq!(utf8_from_eucj(&[0x8e, 0xb1]), "ｱ");
        assert_eq!(eucj_from_utf8("ｱ"), vec![0x8e, 0xb1]);
    }

    #[test]
    fn eucj_substitution() {
        assert_eq!(utf8_from_eucj(&[0xff]), "〓");
        assert_eq!(eucj_from_utf8("\u{1F600}"), vec![0xa2, 0xae]);
    }
}
