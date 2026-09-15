//
// gridlock .............. unicode/grapheme.rs
// copyright (c) 2026 malakai smith (@tenault)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0.
//

// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    ENVIRONMENT    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

use std::iter::Peekable;
use std::str::CharIndices;

use crate::unicode::{GRAPHEME_BREAKS, GraphType};


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    GRAPHEME CLUSTER    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// A segmented grapheme cluster with metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphemeCluster {
    /// Raw utf-8 bytes of the cluster.
    pub bytes: Box<[u8]>,

    /// Display width (1 or 2)
    pub width: u8,
}

impl GraphemeCluster {

    // ,,,,,,,,,,,,,,,,,,,,,,
    // [    constructors    ]
    // ''''''''''''''''''''''

    /// Manually contructs a new cluster with a given set of bytes and optical width.
    pub fn new(bytes: Box<[u8]>, width: u8) -> Self {
        Self { bytes, width }
    }

    /// Creates an empty cluster.
    pub fn blank() -> Self {
        Self { bytes: Box::new([]), width: 0 }
    }

    // ,,,,,,,,,,,,,,,,,
    // [    utility    ]
    // '''''''''''''''''

    /// Get the byte-length of this cluster.
    #[inline]
    pub fn len(&self) -> usize { self.bytes.len() }

    /// Check if this cluster is empty.
    #[inline]
    pub fn is_empty(&self) -> bool { self.bytes.is_empty() }

    /// Borrow the raw bytes as a slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] { &self.bytes }

    /// View the bytes as a string.
    #[inline]
    pub fn as_str(&self) -> Option<&str> { std::str::from_utf8(&self.bytes).ok() }
}


// ~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    SUPPORT TYPES    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~

// ~~~~~ ITERATOR ~~~~~

/// Iterator over grapheme clusters in a string
pub(crate) struct GraphemeSplitter<'a> {
    chars: Peekable<CharIndices<'a>>,
    text: &'a str,
}

impl<'a> GraphemeSplitter<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self { chars: text.char_indices().peekable(), text }
    }

    /// Get the next grapheme boundary pair (start/end indices).
    pub(crate) fn next_boundary_pair(&mut self) -> Option<(usize, usize)> {
        let (idx, _) = self.chars.peek()?;
        let mut start = *idx;

        let (_, prev) = self.chars.next()?;
        let mut prev_type = _identify(prev);

        // acc unbreakable chars
        while let Some(&(_, next)) = self.chars.peek() {
            let next_type = _identify(next);

            if _should_break(prev_type, next_type) { break; }

            self.chars.next();
            prev_type = next_type;
        }

        let end = self.chars.peek()
            .map(|(idx, _)| *idx)
            .unwrap_or(self.text.len());

        Some((start, end))
    }

    /// Determine if a grapheme cluster should break according to UAX #29.
    fn _should_break(&self, prev: GraphType, next: GraphType) -> bool {
        return match (prev, next) {
            (GraphType::CR, GraphType::LF) => false,
            (GraphType::Control | GraphType::CR | GraphType::LF, _) => true,
            (_, GraphType::Control | GraphType::CR | GraphType::LF) => true,
            (GraphType::L, GraphType::L | GraphType::V | GraphType::LV | GraphType::LVT) => false,
            (GraphType::V | GraphType::LV, GraphType::V | GraphType::T) => false,
            (GraphType::T | GraphType::LVT, GraphType::T) => false,
            (_, GraphType::Extend | GraphType::ZWJ) => false,
            (_, GraphType::SpacingMark) => false,
            (GraphType::Prepend, _) => false,
            (_, _) => true,
        }
    }
}


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ SEGMENTATION ~~~~~

pub fn segments(text: &str) -> Vec<GraphemeCluster> {
    if text.is_empty() { return Vec::new(); }

    let mut clusters = Vec::new();
    let mut splitter = GraphemeSplitter::new(text);

    while let Some((start, end)) = splitter.next_boundary_pair() {
        let bytes = &text[start..end];
        let width = 1; // compute_display_width(bytes);

        if !bytes.is_empty() {
            clusters.push(GraphemeCluster { bytes: bytes.into(), width });
        }
    }

    clusters
}

// ~~~~~ INTERNAL ~~~~~

/// Efficiently search a list of ranges to determine if they contain a char.
#[inline(always)]
fn _binary_search(
    ranges: &[(u32, u32, GraphType)],
    v: u32,
) -> Option<GraphType> {
    let mut lo = 0;
    let mut hi = ranges.len();

    while lo < hi {
        let mid = (lo + hi) >> 1;
        let (start, end, variant) = ranges[mid];

        if v >= start && v <= end {
            return Some(variant);
        } else if v < start {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }

    None
}

/// Identify a char by its grapheme break property.
#[inline]
fn _identify(ch: char) -> GraphType {
    let v = ch as u32;

    // ASCII shortcut
    if v < 0x80 {
        return match v {
            0x00..=0x09 | 0x0b..=0x0c | 0x0e..=0x1f | 0x7f => GraphType::Control,
            0x0a => GraphType::LF,
            0x0d => GraphType::CR,
            _ => GraphType::Other,
        };
    }

    // Hangul LV/LVT syllables shortcut
    if v >= 0xac00 && v <= 0xd7a3 {
        return if (v - 0xac00) % 28 == 0 {
            GraphType::LV
        } else {
            GraphType::LVT
        };
    }

    // Binary search all grapheme breaks
    match _binary_search(GRAPHEME_BREAKS, v) {
        Some(variant) => variant,
        None => GraphType::Other,
    }
}

