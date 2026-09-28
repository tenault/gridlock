//
// gridlock ................... sphinx/main.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

use std::env;
use std::fs;

/// Ingests and converts unicode property range lists into a symbols.rs file according to some given
/// template.
///
/// Usage: cargo run -- <template.rs> <file1.txt> <file2.txt> ... <fileN.txt>
fn main() {

    // ~~~~~ collect files ~~~~~

    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!(
            "Usage: {} <template.rs> <file1.txt> <file2.txt> ... <file3.txt>",
            env::args().next().unwrap_or_default(),
        );
        std::process::exit(1);
    }

    let template = fs::read_to_string(&args[0])
        .unwrap_or_else(|e| panic!("Failed to read template {}: {}", &args[0], e));

    let files = &args[1..];

    // ~~~~~ ingest entries ~~~~~

    let mut graphs: Vec<(u32, u32, &str)> = Vec::new();
    let mut ascii:   [&str; 256] = ["O"; 256];
    let mut linkers: Vec<u32> = Vec::new();

    for file in files {
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file, e));

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('@') { continue; }

            let mut parts = line.split(" ; ");
            let code_half = parts.next().unwrap_or("").trim();
            let prop_half = parts.next().unwrap_or("").trim();

            let prop = prop_half.split('#').next().unwrap_or("").trim(); // drop comments

            if prop.is_empty() || code_half.is_empty() { continue; }

            let (start, end) = if let Some((lo, hi)) = code_half.split_once("..") {
                (
                    u32::from_str_radix(lo.trim(), 16).expect("bad hex"),
                    u32::from_str_radix(hi.trim(), 16).expect("bad hex"),
                )
            } else {
                let v = u32::from_str_radix(code_half, 16).expect("bad hex");
                (v, v)
            };

            let variant = match prop {
                "Prepend" => "P",
                "CR" => "CR",
                "LF" => "LF",
                "Control" => "C",
                "Extend" => "E",
                "Extended_Pictographic" => "EP",
                "Regional_Indicator" => "RI",
                "SpacingMark" => "SM",
                "L" => "L",
                "V" => "V",
                "T" => "T",
                "ZWJ" => "ZW",
                "InCB; Linker" => "IL",
                "InCB; Consonant" => "IC",
                _ => continue,
            };

            // extract ascii graphs
            if end < 0x100 {
                for v in start..=end { ascii[v as usize] = variant; }
                continue;
            }

            // extract indic conjunct break linkers
            if variant == "IL" {
                for v in start..=end { linkers.push(v); }
                continue;
            }

            graphs.push((start, end, variant));
        }
    }
   
    // ~~~~~ sort and merge adjacent ranges ~~~~~

    graphs.sort_by_key(|&(s, _, _)| s);

    let mut merged: Vec<(u32, u32, &str)> = Vec::new();
    for (start, end, variant) in &graphs {
        if let Some(last) = merged.last_mut() {
            if last.2 == *variant && last.1 + 1 == *start {
                last.1 = *end;
                continue;
            }
        }

        merged.push((*start, *end, *variant));
    }

    linkers.sort();
    linkers.dedup();

    // ~~~~~ build lookup array ~~~~~

    let mut peeks: Vec<u16> = Vec::new();

    let max = 0x20000; // UAX29_GRAPHS only has 5 ranges > 0x20000, so chasm traversal is pointless

    let mut i = 0;
    for v in 0..(max >> 8) {
        let start = v << 8;
        while i < merged.len() && (merged[i].1 as usize) < start { i += 1; }

        peeks.push(i as u16);
    }

    // ~~~~~ format output ~~~~~

    let indent_ascii   = extract_indent(&template, "{{ASCII_GRAPHS}}");
    let indent_linkers = extract_indent(&template, "{{INDIC_LINKERS}}");
    let indent_peeks   = extract_indent(&template, "{{GRAPH_PEEKS}}");
    let indent_graphs  = extract_indent(&template, "{{UAX29_GRAPHS}}");

    let formatted_ascii = export(ascii, 16, indent_ascii);

    let formatted_linkers = export(
        linkers.iter().map(|&v| format!("0x{:04x}", v)),
        4,
        indent_linkers,
    );

    let formatted_peeks = export(
        peeks.iter().map(|&v| v.to_string()),
        16,
        indent_peeks,
    );

    let formatted_graphs = export(
        merged.iter().map(|(s, e, v)| {
            let start = format!("{:#06x}", s);
            let end   = format!("{:#06x}", e);

            let space = if start.len() < 7 { "  " } else { " " };

            format!("({},{}{},{}{})",
                start, space,
                end, space,
                v,
            )
        }),
        4,
        indent_graphs,
    );

    let out = template
        .replace("{{ASCII_GRAPHS}}",  &formatted_ascii)
        .replace("{{INDIC_LINKERS}}", &formatted_linkers)
        .replace("{{GRAPH_PEEKS}}",   &formatted_peeks)
        .replace("{{UAX29_GRAPHS}}",  &formatted_graphs);

    // ~~~~~ export ~~~~~

    fs::write("symbols.rs", out).expect("Failed to write symbols.rs");
    println!(
        "Read {} entries; merged & wrote {} entries to symbols.rs",
        graphs.len(), merged.len()
    );
}

fn export<I, S>(data: I, cols: usize, indent: usize) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let data: Vec<S> = data.into_iter().collect();
    if data.is_empty() { return String::new(); }

    let width = data.iter().map(|v| v.as_ref().len()).max().unwrap_or(0);

    let mut out = String::new();

    for (i, v) in data.iter().enumerate() {
        let v = v.as_ref();

        if i != 0 && i % cols == 0 {
            for _ in 0..indent { out.push(' '); }
        }

        out.push_str(v);
        out.push(',');

        if (i + 1) < data.len() {
            if (i + 1) % cols != 0 {
                for _ in 0..(width - v.len() + 1) { out.push(' '); }
            } else {
                out.push('\n')
            }
        }
    }

    out
}

fn extract_indent(source: &str, key: &str) -> usize {
    for line in source.lines() {
        if line.contains(key) {
            return line.len() - line.trim_start().len();
        }
    }
    0
}
