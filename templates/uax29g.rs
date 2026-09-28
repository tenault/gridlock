//
// gridlock ........ unicode/symbols/uax29g.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

//! The symbols in this module are generated from the following unicode documents:
//!
//! - DerivedCoreProperties.txt
//! - emoji-data.txt
//! - GraphemeBreakProperty.txt

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

use self::GraphType::*;


// ~~~~~~~~~~~~~~~~~~~
// [[    SYMBOLS    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ GRAPHEME BREAKS ~~~~~

/// An O(1) lookup table for extended ASCII, which covers the vast majority of terminal text.
pub(crate) const ASCII_GRAPHS: &[GraphType] = &[
    {{ASCII_GRAPHS}}
];

/// The set `\p{InCB=Linker}` as defined by UAX #29 (GB9c).
pub(crate) const INDIC_LINKERS: &[u32] = &[
    {{INDIC_LINKERS}}
];

/// An O(1) lookup table for UAX29_GRAPHS to greatly reduce binary search space, paged by 0x100.
pub(crate) const GRAPH_PEEKS: &[u16] = &[
    {{GRAPH_PEEKS}}
];

/// A list of UAX #29 grapheme break property ranges, sorted by start codepoint.
///
/// Ranges below `0x100` are ommited (as well as `LV` and `LVT`) due to their trivial computation.
pub(crate) const UAX29_GRAPHS: &[(u32, u32, GraphType)] = &[
    {{UAX29_GRAPHS}}
];


// ~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    SUPPORT TYPES    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~

/// Grapheme cluster variants for quick identification.
///
/// Each variant (minus LVT) is encoded with a max of 2-letters to assist spacing in lookup tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphType {
    /// Prepend character
    P,
    /// Carriage return
    CR,
    /// Line feed
    LF,
    /// Control character
    C,
    /// Extender (combining marks, emoji modifiers)
    E,
    /// Extended pictogrpahic (emoji modifier sequences and friends)
    EP,
    /// Regional indicator (A-Z regional flags)
    RI,
    /// Spacing mark
    SM,
    /// Leading jamo (Hangul)
    L,
    /// Vowel jamo (Hangul)
    V,
    /// Trailing jamo (Hangul)
    T,
    /// LV syllable (Hangul)
    LV,
    /// LVT syllable (Hangul)
    LVT,
    /// Zero-width joiner
    ZW,
    /// Indic conjunct break consonant
    IC,
    /// Literally any other character
    O,
}
