// NablaSKK: replays testdata/scenarios.txt natively. The Deno test
// (deno/scenario_test.ts) replays the same file against the built wasm,
// so both builds must give the same states for the same keys (ADR 0023).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

mod common;

use common::*;

#[test]
fn scenarios() {
    let text = include_str!("../testdata/scenarios.txt");
    let mut demo = Demo::standard();
    let mut failures = Vec::new();
    let mut count = 0;

    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let actual = match fields.as_slice() {
            ["new"] => {
                demo = Demo::standard();
                continue;
            }
            ["keys", keys, _] => {
                for (charcode, keycode, mods) in parse_keys(keys) {
                    demo.handle(charcode, keycode, mods);
                }
                demo.state()
            }
            ["rules", rules, _] => demo.set_user_rules(&rules.replace("\\n", "\n")),
            _ => panic!("line {}: malformed: {line}", index + 1),
        };
        count += 1;
        let expected = fields[2];
        if actual != expected {
            failures.push(format!("line {}: {}\n  expected: {expected}\n    actual: {actual}", index + 1, fields[1]));
        }
    }

    assert!(count > 0);
    assert!(failures.is_empty(), "{} / {count} failed:\n{}", failures.len(), failures.join("\n"));
}
