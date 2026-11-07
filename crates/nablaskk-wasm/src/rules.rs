// NablaSKK: checks for user romaji-kana rules, ported from the
// preferences app (macos/Shared/KanaRuleConfig.swift: KanaRule.problem
// and KanaRuleSettings.conflicts) so the web demo rejects the same lines
// with the same reasons (ADR 0023).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

/// One rule as the checks see it: what is typed and the hiragana output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub input: String,
    pub output: String,
}

/// The verdict on one non-blank, non-comment line of user rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineResult {
    /// 1-based line number in the text that was checked.
    pub line: usize,
    /// The line as written, handed to the engine when it is applied.
    pub text: String,
    /// The typed input, unescaped.
    pub input: String,
    /// Why the line is not applied; None when it is.
    pub reason: Option<String>,
}

const MAX_INPUT_LENGTH: usize = 8;

/// Keys that switch modes in kana mode unless they continue a rule
/// (KanaRule.modeKeys).
const MODE_KEYS: &[(char, &str)] = &[
    ('q', "ひらがな/カタカナ切替"),
    ('l', "英数モードへの切替"),
    ('/', "abbrev モードへの切替"),
];

const BUILTIN_ORIGIN: &str = "組み込み";

const FIELDS_MESSAGE: &str = "入力,ひらがな,カタカナ,半角カナ を , で区切って書いてください";

fn unescape(text: &str) -> String {
    text.replace("&comma;", ",").replace("&space;", " ").replace("&sharp;", "#")
}

/// Built-in rules from kana-rule.conf text in file order; a later line for
/// the same input replaces the earlier one in place
/// (KanaRuleSettings.builtinRules(from:)).
pub fn builtin_rules(text: &str) -> Vec<Rule> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            (fields.len() >= 4).then(|| Rule { input: unescape(fields[0]), output: unescape(fields[1]) })
        })
        .fold(Vec::new(), |mut rules: Vec<Rule>, rule| {
            match rules.iter_mut().find(|existing| existing.input == rule.input) {
                Some(existing) => existing.output = rule.output,
                None => rules.push(rule),
            }
            rules
        })
}

/// Why a rule cannot be typed at all, or None (KanaRule.problemMessage).
pub fn problem(input: &str, output: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let message = if input.is_empty() {
        "入力キーを指定してください".to_string()
    } else if output.is_empty() {
        "出力を指定してください".to_string()
    } else if !bytes.iter().all(|b| (0x21..=0x7e).contains(b)) {
        "入力キーは空白を含まない半角英数記号にしてください".to_string()
    } else if bytes.iter().any(u8::is_ascii_uppercase) {
        "大文字は▽変換の開始になるので使えません".to_string()
    } else if bytes.len() > MAX_INPUT_LENGTH {
        format!("入力キーは {MAX_INPUT_LENGTH} 文字までです")
    } else if let Some((key, name)) = MODE_KEYS.iter().find(|(key, _)| input.starts_with(*key)) {
        format!("{key} は{name}キーなので、このルールは入力できません")
    } else if output.contains(['\n', '\r']) {
        "入力キーは空白を含まない半角英数記号にしてください".to_string()
    } else {
        return None;
    };
    Some(message)
}

/// Why `rule` conflicts with the accepted rules, or None
/// (the loop body of KanaRuleSettings.conflicts(builtin:)). Each accepted
/// rule says whether it is built in.
fn conflict(rule: &Rule, accepted: &[(Rule, bool)]) -> Option<String> {
    if let Some((same, is_builtin)) = accepted.iter().rev().find(|(other, _)| other.input == rule.input) {
        let origin = if *is_builtin { format!("{BUILTIN_ORIGIN}のルール") } else { "追加したルール".to_string() };
        return Some(if same.output == rule.output {
            format!("同じルールが{origin}にあります")
        } else {
            format!("{origin}「{} → {}」と入力キーが衝突します", same.input, same.output)
        });
    }

    // Extending a rule that completes on its last key today (nothing else
    // continues it) would make that rule wait for another key
    let blocked = accepted.iter().map(|(other, _)| other).find(|prefix| {
        rule.input.len() > prefix.input.len()
            && rule.input.starts_with(&prefix.input)
            && !accepted
                .iter()
                .any(|(other, _)| other.input != prefix.input && other.input.starts_with(&prefix.input))
    })?;
    Some(format!("「{} → {}」がすぐに確定しなくなるため使えません", blocked.input, blocked.output))
}

/// Check each line of user rules against the built-in table and the lines
/// above it (KanaRuleSettings.conflicts(builtin:)). Lines need the four
/// fields of kana-rule.conf; unlike the preferences app, the katakana and
/// half-width outputs are not derived.
pub fn check(text: &str, builtin: &[Rule]) -> Vec<LineResult> {
    let initial: Vec<(Rule, bool)> = builtin.iter().map(|rule| (rule.clone(), true)).collect();

    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'))
        .scan(initial, |accepted, (index, line)| {
            let fields: Vec<&str> = line.split(',').collect();
            let rule = Rule {
                input: unescape(fields[0]),
                output: fields.get(1).map(|field| unescape(field)).unwrap_or_default(),
            };
            let complete = fields.len() >= 4 && !fields[2].is_empty() && !fields[3].is_empty();
            let reason = if complete {
                problem(&rule.input, &rule.output).or_else(|| conflict(&rule, accepted))
            } else {
                Some(FIELDS_MESSAGE.to_string())
            };
            let input = rule.input.clone();
            if reason.is_none() {
                accepted.push((rule, false));
            }
            Some(LineResult { line: index + 1, text: line.to_string(), input, reason })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> Vec<Rule> {
        builtin_rules(include_str!("../../../data/kana-rule.utf8.conf"))
    }

    /// The reasons for rules given as (input, hiragana) pairs, like
    /// `conflicted` in macos/Tests/main.swift.
    fn conflicted(pairs: &[(&str, &str)]) -> Vec<Option<String>> {
        let text: String = pairs
            .iter()
            .map(|(input, output)| {
                let (input, output) = (input.replace(',', "&comma;"), output.replace(',', "&comma;"));
                format!("{input},{output},{output},{output}\n")
            })
            .collect();
        check(&text, &builtin()).into_iter().map(|result| result.reason).collect()
    }

    fn reason(input: &str, output: &str) -> Option<String> {
        conflicted(&[(input, output)]).remove(0)
    }

    #[test]
    fn builtin_table_is_parsed() {
        let rules = builtin();
        let find = |input: &str| rules.iter().find(|rule| rule.input == input).map(|rule| rule.output.as_str());
        assert_eq!(find("z."), Some("…"));
        assert_eq!(find("z,"), Some("‥"));
        assert_eq!(find("ka"), Some("か"));
        assert_eq!(find("bb"), Some("っ"));
    }

    #[test]
    fn later_builtin_line_replaces_in_place() {
        let rules = builtin_rules("a,あ,ア,ｱ\nb,x,x,x\na,ぁ,ァ,ｧ\n# c,c,c,c\n\n");
        let pairs: Vec<_> = rules.iter().map(|rule| format!("{}{}", rule.input, rule.output)).collect();
        assert_eq!(pairs, ["aぁ", "bx"]);
    }

    #[test]
    fn problems_match_the_preferences_app() {
        assert_eq!(problem("fj", "、"), None);
        assert_eq!(problem("", "、").as_deref(), Some("入力キーを指定してください"));
        assert_eq!(problem("fj", "").as_deref(), Some("出力を指定してください"));
        assert_eq!(problem("Fj", "、").as_deref(), Some("大文字は▽変換の開始になるので使えません"));
        assert_eq!(problem("f j", "、").as_deref(), Some("入力キーは空白を含まない半角英数記号にしてください"));
        assert_eq!(problem("ふ", "、").as_deref(), Some("入力キーは空白を含まない半角英数記号にしてください"));
        assert_eq!(problem("abcdefghi", "x").as_deref(), Some("入力キーは 8 文字までです"));
        assert_eq!(problem("abcdefgh", "x"), None);
        assert_eq!(problem("lj", "x").as_deref(), Some("l は英数モードへの切替キーなので、このルールは入力できません"));
        assert_eq!(problem("qj", "x").as_deref(), Some("q はひらがな/カタカナ切替キーなので、このルールは入力できません"));
        assert_eq!(problem("/j", "x").as_deref(), Some("/ はabbrev モードへの切替キーなので、このルールは入力できません"));
    }

    #[test]
    fn conflicts_match_the_preferences_app() {
        assert_eq!(conflicted(&[("fj", "、"), ("fk", "。")]), [None, None]);
        assert_eq!(reason("z.", "…").as_deref(), Some("同じルールが組み込みのルールにあります"));
        assert_eq!(reason("z.", "。").as_deref(), Some("組み込みのルール「z. → …」と入力キーが衝突します"));
        assert_eq!(
            conflicted(&[("fj", "、"), ("fj", "、")])[1].as_deref(),
            Some("同じルールが追加したルールにあります")
        );
        assert_eq!(
            conflicted(&[("fj", "、"), ("fj", "。")])[1].as_deref(),
            Some("追加したルール「fj → 、」と入力キーが衝突します")
        );
        assert_eq!(reason("ka.", "x").as_deref(), Some("「ka → か」がすぐに確定しなくなるため使えません"));
        assert_eq!(reason(",,", "x").as_deref(), Some("「, → 、」がすぐに確定しなくなるため使えません"));
        assert_eq!(
            conflicted(&[("fj", "、"), ("fjk", "x")])[1].as_deref(),
            Some("「fj → 、」がすぐに確定しなくなるため使えません")
        );
        assert_eq!(reason("n.", "x"), None, "n already waits for na, ni ...");
        assert_eq!(conflicted(&[("z", "x"), ("f", "y")]), [None, None], "a prefix of existing rules is allowed");
        // Invalid rules are reported by their problem, not by conflicts
        assert_eq!(
            conflicted(&[("lj", "x"), ("lj", "y")]),
            [
                Some("l は英数モードへの切替キーなので、このルールは入力できません".to_string()),
                Some("l は英数モードへの切替キーなので、このルールは入力できません".to_string()),
            ]
        );
    }

    #[test]
    fn lines_need_four_fields() {
        for line in ["fj,、", "fj,、,、", "fj,、,,､", "fj,、,、,"] {
            let results = check(line, &builtin());
            assert_eq!(results[0].reason.as_deref(), Some(FIELDS_MESSAGE), "{line}");
        }
        // A fifth field (the engine's next state) is allowed
        assert_eq!(check("fj,、,、,､,f", &builtin())[0].reason, None);
    }

    #[test]
    fn lines_are_numbered_and_escapes_are_undone() {
        let text = "# comment\n\nfj,、,、,､\n&sharp;&comma;,x,x,x\nf&space;j,x,x,x\n";
        let results = check(text, &builtin());
        let summary: Vec<_> = results.iter().map(|r| (r.line, r.input.as_str(), r.reason.is_none())).collect();
        assert_eq!(summary, [(3, "fj", true), (4, "#,", true), (5, "f j", false)]);
        assert_eq!(results[0].text, "fj,、,、,､");
    }

    #[test]
    fn rejected_lines_do_not_block_later_ones() {
        // ka. is rejected, so it is not an accepted rule that fj could hit
        let results = check("ka.,x,x,x\nka.,y,y,y\n", &builtin());
        assert_eq!(results[1].reason.as_deref(), Some("「ka → か」がすぐに確定しなくなるため使えません"));
    }
}
