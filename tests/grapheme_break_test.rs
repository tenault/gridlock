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
    let root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(root).join("tests/data/GraphemeBreakTest.txt");
    let suite = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read test data: {}", e));

    let mut passed = 0;
    let mut failed = 0;
    
    for (num, line) in suite.lines().enumerate() {
        if let Some(test) = parse_test(line.trim(), num + 1) {
            let result = run_test(&test);

            match result {
                Ok((text, clusters)) => {
                    passed += 1;
                    println!("Line {}: {} => {:?}", test.line, text, clusters);
                },
                Err(e) => {
                    failed += 1;
                    eprintln!("{}", e.describe());
                },
            }
        }
    }

    println!("[ GRAPHEME BREAK TEST RESULTS ]");
    println!(" -> PASSED ... {}", passed);
    println!(" -> FAILED ... {}", failed);
}


// ~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    SUPPORT TYPES    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~

// ~~~~~ GRAPHEME RULES ~~~~~

/// Represents a single UAX #29 grapheme break rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphemeRule {
    GB1,
    GB2,
    GB3,
    GB4,
    GB5,
    GB6,
    GB7,
    GB8,
    GB9,
    GB9a,
    GB9b,
    GB9c,
    GB11,
    GB12,
    GB13,
    GB999,
}

impl GraphemeRule {
    /// Identifies a rule from a given string in test comment format (e.g. `[1.0]` or `[9.3]`)
    fn from(rule: &str) -> Option<Self> {
        let pearl = rule.trim_matches(|c| c == '[' || c == ']');
        let jewels: Vec<&str> = pearl.split('.').collect();

        if jewels.is_empty() || jewels.len() < 2 { return None; }

        match jewels[0] {
            "1" => Some(Self::GB1),
            "2" => Some(Self::GB2),
            "3" => Some(Self::GB3),
            "4" => Some(Self::GB4),
            "5" => Some(Self::GB5),
            "6" => Some(Self::GB6),
            "7" => Some(Self::GB7),
            "8" => Some(Self::GB8),
            "9" => match jewels[1] {
                "0" => Some(Self::GB9),
                "1" => Some(Self::GB9a),
                "2" => Some(Self::GB9b),
                "3" => Some(Self::GB9c),
                _ => None,
            },
            "11" => Some(Self::GB11),
            "12" => Some(Self::GB12),
            "13" => Some(Self::GB13),
            "999" => Some(Self::GB999),
            _ => None,
        }
    }

    /// Human-readable rule description.
    fn describe(&self) -> &'static str {
        match self {
            Self::GB1   => "GB1: sot + Any",
            Self::GB2   => "GB2: Any + eot",
            Self::GB3   => "GB3: CR + LF",
            Self::GB4   => "GB4: Control + Any",
            Self::GB5   => "GB5: Any + Control",
            Self::GB6   => "GB6: L + (L | V | LV | LVT)",
            Self::GB7   => "GB7: (LV | V) + (V | T)",
            Self::GB8   => "GB8: (LVT | T) + T",
            Self::GB9   => "GB9: Any + (Extend | ZWJ)",
            Self::GB9a  => "GB9a: Any + SpacingMark",
            Self::GB9b  => "GB9b: Prepend + Any",
            Self::GB9c  => "GB9c: IndicLinker IndicExtend* + IndicConsonant",
            Self::GB11  => "GB11: ExtPict Extend* ZWJ + ExtPict",
            Self::GB12  => "GB12: sot (RI RI)* RI + RI",
            Self::GB13  => "GB13: [^RI] (RI RI)* RI + RI",
            Self::GB999 => "GB999: Any + Any",
        }
    }
}

// ~~~~~ RULE EXTRACTOR ~~~~~

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

        if self.pos >= self.bytes.len() { return None; }

        let start = self.pos;
        self.pos += 1;

        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b']' {
                let end = self.pos;
                self.pos += 1;

                let rule = unsafe { std::str::from_utf8_unchecked(&self.bytes[start..end]) };
                return Some(rule);
            }
            self.pos += 1;
        }

        None
    }
}

// ~~~~~ TEST CASE ~~~~~

/// Represents a parsed test case from GraphemeBreakTest.txt
#[derive(Clone, Debug)]
struct TestCase {
    /// Raw unicode scalars to test.
    scalars: Vec<u32>,
    /// Expected grapheme break positions.
    breaks: Vec<usize>,
    /// Test file comment (for debugging).
    rules: Vec<GraphemeRule>,
    /// Line number in test file.
    line: usize,
}

// ~~~~~ TEST CASE FAILURE ~~~~~

/// Failure state for any `TestCase`.
#[derive(Debug)]
enum TestCaseFailure {
    BadScalar {
        line:   usize,
        scalar: u32,
    },
    OutOfCompliance {
        line:     usize,
        pos:      usize,
        expected: Vec<usize>,
        actual:   Vec<usize>,
        rules:    Vec<GraphemeRule>,
        text:     String,
        scalars:  Vec<u32>,

    }
}

impl TestCaseFailure {
    fn describe(&self) -> String {
        match self {
            Self::BadScalar { line, scalar } => {
                format!("Line {}: Unknown unicode scalar {:04x}", line, scalar)
            },
            Self::OutOfCompliance {
                line,
                pos,
                expected,
                actual,
                rules,
                text,
                scalars,
            } => {
                let mut out = String::new();

                out.push_str(&format!("[ GRAPHEME CLUSTER FAILURE (line {}) ]\n", line));
                out.push_str(&format!(" -> TEXT .............. {}\n", text));
                let hex: String = scalars.iter()
                    .map(|s| format!("{:#06x}", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(" -> SCALARS ........... [{}]\n", hex));
                out.push_str(&format!(" -> EXPECTED BREAKS ... {:?}\n", expected));
                out.push_str(&format!(" -> ACTUAL BREAKS ..... {:?}\n", actual));
                out.push_str(&format!(" -> DEVIATION AT ...... {}\n", pos));
                
                if !rules.is_empty() {
                    out.push_str(" -> RULES\n");
                    for rule in rules.iter() {
                        out.push_str(&format!("     -> {}\n", rule.describe()));
                    }
                }

                out
            },
        }
    }
}


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ TEST CASE ~~~~~

/// Ingests a single line from `GraphemeBreakTests.txt` and converts it into a usable `TestCase`.
fn parse_test(line: &str, num: usize) -> Option<TestCase> {
    if line.is_empty() || line.starts_with('#') { return None; }

    let (test, comment) = match line.split_once('#') {
        Some((t, c)) => (t.trim(), c.trim()),
        None => return None,
    };

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

    let mut rules: Vec<GraphemeRule> = Vec::new();

    for rule in extract_rules(comment) {
        if let Some(r) = GraphemeRule::from(rule) { rules.push(r); }
    }

    Some(TestCase {
        scalars,
        breaks,
        rules,
        line: num,
    })
}

/// Runs a `TestCase` and returns any failure.
fn run_test(test: &TestCase) -> Result<(String, Vec<String>), TestCaseFailure> {
    let mut text = String::new();

    for scalar in &test.scalars {
        match char::from_u32(*scalar) {
            Some(c) => text.push(c),
            None => return Err(TestCaseFailure::BadScalar { line: test.line, scalar: *scalar }),
        }
    }

    let clusters: Vec<String> = graphemes(&text)
        .filter_map(|c| c.as_str().map(|s| s.to_string()))
        .collect();

    let mut breaks = Vec::new();
    let mut offset = 0;

    for cluster in graphemes(&text) {
        breaks.push(offset);
        offset += cluster.scalars().count();
    }

    breaks.push(offset);

    if test.breaks != breaks {
        let pos = find_first_deviation(&test.breaks, &breaks);
        return Err(TestCaseFailure::OutOfCompliance {
            line: test.line,
            pos,
            expected: test.breaks.clone(),
            actual:   breaks,
            rules:    test.rules.clone(),
            text:     text.clone(),
            scalars:  test.scalars.clone(),
        });
    }

    Ok((text, clusters))
}

// ~~~~~ HELPERS ~~~~~

/// 
fn extract_rules(comment: &str) -> RuleExtractor<'_> {
    RuleExtractor { bytes: comment.as_bytes(), pos: 0 }
}

/// Find the first index where entries are mismatched.
fn find_first_deviation(expected: &[usize], actual: &[usize]) -> usize {
    for i in 0..expected.len().max(actual.len()) {
        if i >= expected.len() || i >= actual.len() { return i; }
        if expected[i] != actual[i] { return i; }
    }

    0
}
