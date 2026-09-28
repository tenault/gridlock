//
// gridlock ..... tests/grapheme_break_test.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

use std::env;
use std::fs;
use std::path::Path;

use gridlock::unicode::graphemes;


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    GRAPHEME BREAK TEST    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[test]
fn graphemes_cluster_correctly() {

    // ..... read test data .....

    let root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(root).join("tests/data/GraphemeBreakTest.txt");
    let cases = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read test suite: {}", e));

    let count = extract_case_count(&cases).unwrap_or_else(|| panic!("Failed to set expectations."));

    // ..... run tests .....

    let mut passed = 0;
    let mut failed = 0;
    let mut failures: Vec<TestCaseFailure> = Vec::new();

    for (num, line) in cases.lines().enumerate() {
        if let Some(test) = parse_test(line.trim(), num + 1) {
            match run_test(&test) {
                Ok(_)  => { passed += 1 },
                Err(e) => { failed += 1; failures.push(e); },
            }
        }
    }

    // ..... grade results .....

    assert!(
        failures.is_empty(),
        "\n[ GRAPHEME BREAK TEST RESULTS ]\n -> PASSED ... {}\n -> FAILED ... {}\n\n{}",
        passed, failed, failures.iter().map(|f| f.describe()).collect::<Vec<_>>().join("\n"),
    );

    assert!(
        passed + failed == count,
        "\n[ GRAPHEME BREAK TEST FAILED ]\n -> PARSED ..... {}\n -> EXPECTED ... {}\n",
        passed + failed, count,
    );
}


// ~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    SUPPORT TYPES    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~

// ~~~~~ GRAPHEME RULES ~~~~~

/// Represents a single UAX #29 grapheme break rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphemeRule {
    GB1,   // sot - Any
    GB2,   // Any - eot
    GB3,   // CR + LF
    GB4,   // Control - Any
    GB5,   // Any - Control
    GB6,   // L + (L | V | LV | LVT)
    GB7,   // (LV | V) + (V | T)
    GB8,   // (LVT | T) + T
    GB9,   // Any + (Extend | ZWJ)
    GB9a,  // Any + SpacingMark
    GB9b,  // Prepend + Any
    GB9c,  // IndicLinker IndicExtend* + IndicConsonant
    GB11,  // ExtPict Extend* ZWJ + ExtPict
    GB12,  // sot (RI RI)* RI + RI
    GB13,  // [^RI] (RI RI)* RI + RI
    GB999, // Any - Any
}

impl GraphemeRule {
    /// Identifies a rule from a given string in test comment format (e.g. `1.0` or `9.3`)
    fn from(rule: &str) -> Option<Self> {
        match rule.trim() {
            "1.0"   => Some(Self::GB1),
            "2.0"   => Some(Self::GB2),
            "3.0"   => Some(Self::GB3),
            "4.0"   => Some(Self::GB4),
            "5.0"   => Some(Self::GB5),
            "6.0"   => Some(Self::GB6),
            "7.0"   => Some(Self::GB7),
            "8.0"   => Some(Self::GB8),
            "9.0"   => Some(Self::GB9),
            "9.1"   => Some(Self::GB9a),
            "9.2"   => Some(Self::GB9b),
            "9.3"   => Some(Self::GB9c),
            "11.0"  => Some(Self::GB11),
            "12.0"  => Some(Self::GB12),
            "13.0"  => Some(Self::GB13),
            "999.0" => Some(Self::GB999),
            _ => None,
        }
    }

    /// Human-readable rule description.
    fn describe(&self) -> &'static str {
        match self {
            Self::GB1   => "[GB1]   sot - Any",
            Self::GB2   => "[GB2]   Any - eot",
            Self::GB3   => "[GB3]   CR + LF",
            Self::GB4   => "[GB4]   Control - Any",
            Self::GB5   => "[GB5]   Any - Control",
            Self::GB6   => "[GB6]   L + (L | V | LV | LVT)",
            Self::GB7   => "[GB7]   (LV | V) + (V | T)",
            Self::GB8   => "[GB8]   (LVT | T) + T",
            Self::GB9   => "[GB9]   Any + (Extend | ZWJ)",
            Self::GB9a  => "[GB9a]  Any + SpacingMark",
            Self::GB9b  => "[GB9b]  Prepend + Any",
            Self::GB9c  => "[GB9c]  IndicLinker IndicExtend* + IndicConsonant",
            Self::GB11  => "[GB11]  ExtPict Extend* ZWJ + ExtPict",
            Self::GB12  => "[GB12]  sot (RI RI)* RI + RI",
            Self::GB13  => "[GB13]  [^RI] (RI RI)* RI + RI",
            Self::GB999 => "[GB999] Any - Any",
        }
    }
}

// ~~~~~ RULE EXTRACTOR ~~~~~

/// Extracts rules in the format `[1.0]` from a comment string.
struct RuleExtractor<'a> {
    bytes: &'a [u8],
    pos:   usize,
}

impl<'a> Iterator for RuleExtractor<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'[' { break; }
            self.pos += 1;
        }

        self.pos += 1;
        if self.pos >= self.bytes.len() { return None; }

        let start = self.pos;
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b']' {
                let end = self.pos;
                let rule = unsafe { std::str::from_utf8_unchecked(&self.bytes[start..end]) };

                return Some(rule);
            }
            self.pos += 1;
        }

        None
    }
}

// ~~~~~ TEST CASE + FAILURE ~~~~~

/// Represents a parsed test case from GraphemeBreakTest.txt
#[derive(Clone, Debug, PartialEq)]
struct TestCase {
    /// Raw unicode scalars to test.
    scalars: Vec<u32>,
    /// Expected grapheme break positions.
    breaks: Vec<usize>,
    /// Index-based break position rules.
    rules: Vec<GraphemeRule>,
    /// Line number in test file.
    line: usize,
}

/// Failure state for any `TestCase`.
#[derive(Debug)]
struct TestCaseFailure {
    line:       usize,
    expected:   Vec<usize>,
    actual:     Vec<usize>,
    rules:      Vec<GraphemeRule>,
    violations: Vec<usize>,
    text:       String,
    scalars:    Vec<u32>,
}

impl TestCaseFailure {
    /// Pretty-prints the failure for fast debugging.
    fn describe(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!("[ GRAPHEME CLUSTER FAILURE ({}) ]\n", self.text.escape_debug()));
        out.push_str(&format!(" -> LINE .............. {}\n", self.line));
        let hex: String = self.scalars.iter()
            .map(|s| format!("{:#06x}", s))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(" -> SCALARS ........... [{}]\n", hex));
        out.push_str(&format!(" -> EXPECTED BREAKS ... {:?}\n", self.expected));
        out.push_str(&format!(" -> ACTUAL BREAKS ..... {:?}\n", self.actual));
        out.push_str(" -> RULE VIOLATIONS\n");
        for (idx, rule) in self.rules.iter().enumerate() {
            let marker = if self.violations.contains(&idx) { "!!!" } else { "   " };
            out.push_str(&format!(" {} -> {}\n", marker, rule.describe()));
        }

        out
    }
}


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ TEST CASE ~~~~~

/// Ingests a single line from `GraphemeBreakTests.txt` and converts it into a usable `TestCase`.
fn parse_test(line: &str, num: usize) -> Option<TestCase> {

    // ..... read line .....

    if line.is_empty() || line.starts_with('#') { return None; }

    let (test, comment) = match line.split_once('#') {
        Some((t, c)) => (t.trim(), c.trim()),
        None => return None,
    };

    // ..... build case .....

    let mut scalars = Vec::new();
    let mut breaks  = Vec::new();
    let mut tokens  = test.split_whitespace();

    match tokens.next() {
        Some("÷") => breaks.push(0),
        Some("×") => {},
        _ => return None,
    }

    while let Some(token) = tokens.next() {
        let scalar = match u32::from_str_radix(token, 16) {
            Ok(s)  => s,
            Err(_) => return None,
        };

        scalars.push(scalar);

        match tokens.next() {
            Some("÷") => breaks.push(scalars.len()),
            Some("×") => {},
            None => breaks.push(scalars.len()),
            _ => return None,
        }
    }

    if scalars.is_empty() { return None; }

    // ..... convert rules .....

    let mut rules: Vec<GraphemeRule> = Vec::new();

    for rule in extract_rules(comment) {
        if let Some(r) = GraphemeRule::from(rule) { rules.push(r); }
    }

    if rules.len() != scalars.len() + 1 { return None; }

    // ..... export case .....

    Some(TestCase {
        scalars,
        breaks,
        rules,
        line: num,
    })
}

/// Runs a `TestCase` and returns any failures.
fn run_test(test: &TestCase) -> Result<(), TestCaseFailure> {

    // ..... build test string .....

    let mut text = String::new();

    for scalar in &test.scalars {
        if let Some(c) = char::from_u32(*scalar) { text.push(c); }
    }

    // ..... split graphemes .....

    let mut breaks = Vec::new();
    let mut offset = 0;

    for cluster in graphemes(&text) {
        breaks.push(offset);
        offset += cluster.scalars().count();
    }

    breaks.push(offset);

    // ..... compare results .....

    if test.breaks != breaks {
        let violations = find_violations(&test.breaks, &breaks);

        return Err(TestCaseFailure {
            line:       test.line,
            expected:   test.breaks.clone(),
            actual:     breaks,
            rules:      test.rules.clone(),
            violations,
            text:       text.clone(),
            scalars:    test.scalars.clone(),
        });
    }

    Ok(())
}

// ~~~~~ HELPERS ~~~~~

/// Extracts the expected test case count from `GraphemeBreakTest.txt`.
fn extract_case_count(data: &str) -> Option<usize> {
    data.lines()
        .find_map(|l| { l.trim().strip_prefix("# Lines:").and_then(|r| r.trim().parse().ok()) })
}

/// Iterator over grapheme break rules in test comments.
fn extract_rules(comment: &str) -> RuleExtractor<'_> {
    RuleExtractor { bytes: comment.as_bytes(), pos: 0 }
}

/// Finds the index (or indices) of extra/missing breaks between `expected` and `actual`.
fn find_violations(expected: &[usize], actual: &[usize]) -> Vec<usize> {
    let mut violations = Vec::new();

    for &pos in actual { // find extra breaks
        if !expected.contains(&pos) { violations.push(pos); }
    }

    for &pos in expected { // find missing breaks
        if !actual.contains(&pos) { violations.push(pos); }
    }

    violations.sort();
    violations.dedup();
    violations
}


// ~~~~~~~~~~~~~~~~~~~~~~
// [[    SELF TESTS    ]]
// ~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod self_tests {
    use super::*;
    use GraphemeRule::*;

    // ..... parse_test() .....

    #[test]
    fn parse_test_empty_line_returns_none() { assert_eq!(parse_test("", 1), None); }

    #[test]
    fn parse_test_comment_only_returns_none() { assert_eq!(parse_test("# some rule", 1), None); }

    #[test]
    fn parse_test_no_comment_returns_none() { assert_eq!(parse_test("÷ 000D ÷ 000D ÷", 1), None); }

    #[test]
    fn parse_test_bad_hex_returns_none() {
        let line = "÷ ABCD × WXYZ ÷ # ÷ [1.0] (Prepend) × [9.2] (V) ÷ [2.0]";
        assert_eq!(parse_test(line, 369), None);
    }

    #[test]
    fn parse_test_missing_rules_returns_none() {
        let line = "÷ 06DD × 1160 ÷ # ÷ [1.0] (Prepend) × 9.2 (V) ÷ [2.0]";
        assert_eq!(parse_test(line, 369), None);
    }

    #[test]
    fn parse_test_parses_tests() {
        let line = "÷ 00A9 ÷ AC01 ÷ # ÷ [1.0] COPYRIGHT SIGN ÷ [999.0] HANGUL SYLLABLE GAG ÷ [2.0]";
        let test = parse_test(line, 735).expect("parse_test() does not parse tests!");

        assert_eq!(test.scalars, vec![0x00a9, 0xac01]);
        assert_eq!(test.breaks, vec![0, 1, 2]);
        assert_eq!(test.rules.len(), 3);
        assert!(test.rules.contains(&GB1));
        assert!(test.rules.contains(&GB999));
        assert!(test.rules.contains(&GB2));
    }

    // ..... extract_case_count() .....

    #[test]
    fn extract_case_count_missing_returns_none() {
        assert_eq!(extract_case_count("no cases here"), None);
    }

    #[test]
    fn extract_case_count_extracts_count() {
        let data = "÷ 11A3A ÷ # ÷ [1.0] ConjunctLinkermExtend ÷ [2.0]\n# Lines: 853\n# EOF";
        assert_eq!(extract_case_count(data), Some(853));
    }

    // ..... extract_rules() .....

    #[test]
    fn extract_rules_empty_returns_empty() {
        let rules: Vec<&str> = extract_rules("").collect();
        assert!(rules.is_empty());
    }

    #[test]
    fn extract_rules_requires_brackets() {
        let rules: Vec<&str> = extract_rules("5.0").collect();
        assert!(rules.is_empty());
    }

    #[test]
    fn extract_rules_single_returns_rule() {
        let rules: Vec<&str> = extract_rules("[1.0]").collect();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0], "1.0");
    }

    #[test]
    fn extract_rules_multiple_returns_rules() {
        let rules: Vec<&str> = extract_rules("[1.0] [9.3] [2.0]").collect();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0], "1.0");
        assert_eq!(rules[1], "9.3");
        assert_eq!(rules[2], "2.0");
    }

    #[test]
    fn extract_rules_ignores_extra_text() {
        let line = "÷ 000D ÷ 000D ÷ # ÷ [1.0] <CARRIAGE RETURN> ÷ [4.0] <CARRIAGE RETURN> ÷ [2.0]";
        let rules: Vec<&str> = extract_rules(line).collect();
        assert_eq!(rules.len(), 3);
    }

    // ..... GraphemeRule::from() .....

    #[test]
    fn grapheme_rule_from_valid_builds() {
        assert_eq!(GraphemeRule::from("1.0"),   Some(GB1));
        assert_eq!(GraphemeRule::from("9.0"),   Some(GB9));
        assert_eq!(GraphemeRule::from("9.3"),   Some(GB9c));
        assert_eq!(GraphemeRule::from("11.0"),  Some(GB11));
        assert_eq!(GraphemeRule::from("999.0"), Some(GB999));
    }

    #[test]
    fn grapheme_rule_from_invalid_returns_none() {
        assert_eq!(GraphemeRule::from(""), None);
        assert_eq!(GraphemeRule::from("0"), None);
        assert_eq!(GraphemeRule::from("5.1"), None);
        assert_eq!(GraphemeRule::from("[9.3]"), None);
        assert_eq!(GraphemeRule::from("not a rule"), None);
    }

    #[test]
    fn grapheme_rule_from_handles_whitespace() {
        assert_eq!(GraphemeRule::from("  4.0  "), Some(GraphemeRule::GB4));
    }

    // ..... TestCaseFailure::describe() .....

    #[test]
    fn failure_description_has_all_fields() {
        let failure = TestCaseFailure {
            line:       870,
            expected:   vec![0, 3, 5, 6],
            actual:     vec![0, 1, 3, 4, 5, 6],
            rules:      vec![GB1, GB9, GB9c, GB999, GB9, GB999, GB2],
            violations: vec![1, 4],
            text:       "ហ្ឫទ័យ".to_string(),
            scalars:    vec![0x17a0, 0x17d2, 0x17ab, 0x1791, 0x17d0, 0x1799],
        };

        let desc = failure.describe();
        assert!(desc.contains("LINE"));
        assert!(desc.contains("SCALARS"));
        assert!(desc.contains("EXPECTED BREAKS ... [0, 3, 5, 6]"));
        assert!(desc.contains("ACTUAL BREAKS ..... [0, 1, 3, 4, 5, 6]"));
        assert!(desc.contains("RULE VIOLATIONS"));
        assert!(desc.contains("[GB1]"));
        assert!(desc.contains("[GB9]"));
        assert!(desc.contains("[GB9c]"));
        assert!(desc.contains("[GB999]"));
        assert!(desc.contains("[GB2]"));
        assert!(desc.contains("!!!"));
    }
}
