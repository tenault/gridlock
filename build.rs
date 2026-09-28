//
// gridlock ......................... build.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

mod scripts;

use std::env;
use std::fs;
use std::time;


// ~~~~~~~~~~~~~~~~~
// [[    BUILD    ]]
// ~~~~~~~~~~~~~~~~~

fn main() {

    // ..... set build dependencies .....

    println!("cargo::rerun-if-changed=data/");
    println!("cargo::rerun-if-changed=scripts/");
    println!("cargo::rerun-if-changed=templates/");
    println!("cargo::rerun-if-changed=Cargo.toml");

    // ..... generate symbol files .....

    if env::var("CARGO_FEATURE_REGENERATE").is_ok() || needs_regen() {
        scripts::uax29g::generate();
    }
}


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

fn needs_regen() -> bool {
    let symbol_files = vec![
        "src/unicode/symbols/uax29g.rs",
    ];

    let data_files = vec![
        "data/unicode/DerivedCoreProperties.txt",
        "data/unicode/emoji-data.txt",
        "data/unicode/GraphemeBreakProperty.txt",
    ];

    for file in &symbol_files {
        let sym = match fs::metadata(file) {
            Ok(m)  => m.modified().unwrap_or(time::SystemTime::UNIX_EPOCH),
            Err(_) => return true,
        };

        for datum in &data_files {
            let dat = match fs::metadata(datum) {
                Ok(m)  => m.modified().unwrap_or(time::SystemTime::UNIX_EPOCH),
                Err(_) => continue,
            };

            if dat > sym { return true; }
        }
    }

    false
}
