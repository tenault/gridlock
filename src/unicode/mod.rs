//
// gridlock ................... unicode/mod.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

//! # Unicode
//!
//! A complete implementation of [UAX #29](https://www.unicode.org/reports/tr29) for native grapheme
//! interactions.

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

pub(crate) mod grapheme;
pub(crate) mod sentence;
pub(crate) mod symbols;
pub(crate) mod word;

pub use grapheme::GraphemeCluster;

pub(crate) use symbols::{GRAPHEME_BREAKS, GraphType};
