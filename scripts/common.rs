//
// gridlock ................ scripts/common.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

use std::fs;


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ FILES ~~~~~

/// Convenience wrapper for `fs::read_to_string()`.
pub fn read_file(file: &str) -> String {
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("Failed to read '{}': {}", file, e))
}

// ~~~~~ FORMATTING ~~~~~

/// Formats a given set as an indented string table with n-columns and alignment.
pub fn export<I, S>(data: I, cols: usize, indent: usize) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let data: Vec<S> = data.into_iter().collect();
    if data.is_empty() { return String::new(); }

    // get max column width
    let width = data.iter().map(|v| v.as_ref().len()).max().unwrap_or(0);
    
    let mut out = String::new();

    for (i, v) in data.iter().enumerate() {
        let v = v.as_ref();

        if i != 0 && i % cols == 0 {
            for _ in 0..indent { out.push(' '); }
        }

        out.push_str(&format!("{},", v));

        if (i + 1) < data.len() {
            if (i + 1) % cols != 0 {
                for _ in 0..(width - v.len() + 1) { out.push(' '); }
            } else {
                out.push('\n');
            }
        }
    }

    out
}

/// Gets the indentation of a key string (e.g. `{{UAX29_GRAPHS}}`) for inserted alignment.
pub fn extract_indent(source: &str, key: &str) -> usize {
    for line in source.lines() {
        if line.contains(key) {
            return line.len() - line.trim_start().len();
        }
    }

    0
}
