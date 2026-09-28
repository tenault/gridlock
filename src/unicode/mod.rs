//
// gridlock ................... unicode/mod.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

pub(crate) mod grapheme;
pub(crate) mod sentence;
pub(crate) mod symbols;
pub(crate) mod word;

pub use grapheme::{graphemes, legacy_graphemes, GraphemeCluster};
