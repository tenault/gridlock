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

use std::cmp::Ordering;

use crate::unicode::{ASCII_GRAPHS, INDIC_LINKERS, GRAPH_PEEKS, UAX29_GRAPHS, GraphType};


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    GRAPHEME SPLITTER    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Iterator over grapheme clusters in a string.
pub(crate) struct GraphemeSplitter<'a> {
    bytes: &'a [u8],
    pos:   usize,
    range: GraphRange,
    last:  Option<UnicodeScalar>,
}

impl<'a> GraphemeSplitter<'a> {

    // ,,,,,,,,,,,,,,,,,,,,,
    // [    constructor    ]
    // '''''''''''''''''''''

    #[inline]
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos:   0,
            range: GraphRange { start: 0, end: 0, graph: GraphType::O },
            last:  None,
        }
    }

    // ,,,,,,,,,,,,,,,,,
    // [    utility    ]
    // '''''''''''''''''

    // ~~~~~ CODEPOINTS ~~~~~

    /// Extract and identify the codepoint at `pos`.
    ///
    /// `pos` will always be at a char boundary, since it only ever advances by `UnicodeScaler.len`.
    #[inline]
    fn extract(&mut self, pos: usize) -> UnicodeScalar {
        let (code, len) = extract_scalar(&self.bytes, pos);
        let graph = self.identify(code);
        UnicodeScalar { code, len, graph }
    }

    /// Identifies the grapheme break property for a given codepoint.
    ///
    /// Uses tiered-lookups for massive search cost-savings:
    /// - Extended ASCII (direct index)
    /// - Cached range/gap from the previous lookup (two comparisons)
    /// - Hangul syllable arithmetic (permits LV/LVT omission in UAX29_GRAPHS)
    /// - Paged binary search (greatly reduces graph lookup space)
    fn identify(&mut self, code: u32) -> GraphType {
        use Ordering::*;

        // ..... extended ASCII shortcut .....

        if code < 0x100 { return ASCII_GRAPHS[code as usize]; }

        // ..... cached range shortcut .....

        if code >= self.range.start && code <= self.range.end { return self.range.graph; }

        // ..... hangul syllable shortcut .....

        if (0xac00..=0xd7a3).contains(&code) {
            return if (code - 0xac00) % 28 == 0 { GraphType::LV } else { GraphType::LVT };
        }

        // ..... paged binary search .....

        // when `code` is >= 0x20000, it only searches the ranges from the last known peekable index
        // to the length of the graph table. As of U18, this covers 5 ranges in [0xe0000..=0xe0fff],
        // and skips the massive astral plane between [0x1fffe..0xe0000].

        // each returned variant has its range cached for shortcutting. if the variant falls between
        // ranges (e.g. GraphType::O), then the gap between the two closest ranges is cached instead
        // to speed up subsequent lookups, since most text in any language typically stays clustered
        // around the same blocks and ranges.

        let page = (code >> 8) as usize;

        let (start, end) = if page < GRAPH_PEEKS.len() {
            let lo = GRAPH_PEEKS[page] as usize;
            let hi = if page + 1 < GRAPH_PEEKS.len() {
                GRAPH_PEEKS[page + 1] as usize
            } else {
                UAX29_GRAPHS.len()
            };
            (lo, hi.max(lo))
        } else {
            let idx = GRAPH_PEEKS.len() - 1;
            let lo = GRAPH_PEEKS[idx] as usize;
            let hi = UAX29_GRAPHS.len();
            (lo, hi)
        };

        match UAX29_GRAPHS[start..end].binary_search_by(|&(s, e, _)| {
            if code < s { Greater }
            else if code > e { Less }
            else { Equal }
        }) {
            Ok(idx) => {
                let (s, e, v) = UAX29_GRAPHS[start + idx];
                self.range = GraphRange { start: s, end: e, graph: v };
                v
            },
            Err(idx) => {
                let gap_idx = start + idx;
                let gap_start = if gap_idx > 0 { UAX29_GRAPHS[gap_idx - 1].1 + 1 } else { 0 };
                let gap_end = if gap_idx < UAX29_GRAPHS.len() {
                    UAX29_GRAPHS[gap_idx].0.saturating_sub(1)
                } else {
                    u32::MAX
                };

                self.range = GraphRange { start: gap_start, end: gap_end, graph: GraphType::O };
                GraphType::O
            },
        }
    }

    // ~~~~~ MEMBERSHIP ~~~~~

    /// Decides if the current boundary between two codepoints conforms to UAX #29.
    ///
    /// Decision point occurs between `previous` (last consumed) and `current` (next candidate), and
    /// relies on a cluster-scoped `state` (e.g. number of Regional Indicator flags) for context.
    ///
    /// Rules re-ordered by frequency for cost-savings, rule non-overlap preserves compliance.
    fn is_boundary(
        &self,
        previous: &UnicodeScalar,
        current:  &UnicodeScalar,
        state:    &ClusterState,
    ) -> bool {
        use GraphType::*;

        // ..... GB3 -- CR × LF .....

        if previous.graph == CR && current.graph == LF { return false; }

        // ..... GB4 -- (Control | CR | LF) ÷ Any .....

        if matches!(previous.graph, C | CR | LF) { return true; }

        // ..... GB5 -- Any ÷ (Control | CR | LF) .....

        if matches!(current.graph,  C | CR | LF) { return true; }

        // ..... GB9 -- Any × (Extend | ZWJ)  .....

        if matches!(current.graph, E | ZW) { return false; }

        // ..... GB6 -- L × (L | V | LV | LVT) .....

        if previous.graph == L && matches!(current.graph, L | V | LV | LVT) { return false; }

        // ..... GB7 -- (LV | V) × (V | T) .....

        if matches!(previous.graph, LV | V) && matches!(current.graph, V | T) { return false; }

        // ..... GB8 -- (LVT | T) × T .....

        if matches!(previous.graph, LVT | T) && current.graph == T { return false; }

        // ..... GB9a -- Any × SpacingMark .....

        if current.graph == SM { return false; }

        // ..... GB9b -- Prepend × Any .....

        if previous.graph == P { return false; }

        // ..... GB11 -- ExtPict Extend* ZWJ × ExtPict .....

        if state.joining && current.graph == EP { return false; }

        // ..... GB9c -- ConjunctLinker ConjunctExtender* × LinkingConsonant .....

        if current.graph == IC && state.linking { return false; }

        // ..... GB12/13 -- [^RI] (RI RI)* RI × RI .....

        if current.graph == RI && state.regionals % 2 == 1 { return false; }

        // ..... GB999 -- Any ÷ Any .....

        true
    }
}

impl<'a> Iterator for GraphemeSplitter<'a> {
    type Item = GraphemeCluster<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() { return None; }

        let start = self.pos;

        // first codepoint is either carried over from the last boundary, or extracted fresh.
        let mut current = match self.last.take() {
            Some(code) => code,
            None => self.extract(start),
        };

        let mut state = ClusterState::new();
        state.fold(&current);

        loop {
            self.pos += current.len;
            if self.pos >= self.bytes.len() { break; }

            let next = self.extract(self.pos);

            if self.is_boundary(&current, &next, &state) {
                self.last = Some(next);
                break;
            }

            state.fold(&next);
            current = next;
        }

        Some(GraphemeCluster {
            bytes: &self.bytes[start..self.pos],
            offset: start,
            width: 1, // todo: compute_display_width()
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.bytes.len() - self.pos;

        // at least 1 cluster if any bytes remain, at most one per byte
        (usize::from(remaining > 0), Some(remaining))
    }
}


// ~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    SUPPORT TYPES    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~

// ~~~~~ UNICODE SCALAR ~~~~~

/// A single decoded unicode scalar.
///
/// Bundling the utf-8 byte length allows `GraphemeSplitter` to skip directly to char boundaries.
#[derive(Clone, Copy, Debug)]
struct UnicodeScalar {
    code:  u32,
    len:   usize,
    graph: GraphType,
}

// ~~~~~ RANGE CACHE ~~~~~

/// Used to cache the last range (or inter-range gap) a codepoint landed in.
///
/// Since text in most languages tends to have its graphemes fall within the same blocks and ranges,
/// storing this lets us trade two comparisons to skip the graph range lookups entirely.
#[derive(Clone, Copy, Debug)]
struct GraphRange {
    start: u32,
    end:   u32,
    graph: GraphType,
}

// ~~~~~ CLUSTER STATE ~~~~~

/// Cluster-scoped state tracking for grapheme breaks which extend beyond the current adjacent pair.
///
/// This enables conformance to GB9c, GB11, and GB12/13. The neat thing about the rules is that they
/// reset at each legitimate cluster boundary, which allows this handy little tracker to live inside
/// each `next()` call instead of the `GraphemeSplitter` itself.
///
/// For example, because regional indicator clusters can only break at even parity, we don't need to
/// store a count of every regional indicator in the text; just the ones in the current cluster.
#[derive(Clone, Copy, Debug)]
struct ClusterState {
    /// A count of Regional Indicators consumed in this cluster.
    regionals: u32,
    /// Whether cluster suffix currently matches `ExtPict Extend*`.
    extending: bool,
    /// Whether cluster suffix currently matches `ExtPict Extend* ZWJ`.
    joining: bool,
    /// Whether an indic conjunct break consonant was consumed and no non-extender intervened.
    consonant: bool,
    /// Whether at least one indic conjunct break linker was seen since the last consonant.
    linking: bool,
}

impl ClusterState {
    #[inline]
    fn new() -> Self {
        Self {
            regionals: 0,
            extending: false,
            joining:   false,
            consonant: false,
            linking:   false,
        }
    }

    /// Fold a unicode scalar into the current cluster state.
    #[inline]
    fn fold(&mut self, next: &UnicodeScalar) {
        use GraphType::*;

        // GB12/13
        self.regionals = if next.graph == RI { self.regionals + 1 } else { 0 };

        // GB11
        match next.graph {
            EP => { self.extending = true; self.joining = false },
            E  => { self.joining = false },
            ZW => { self.joining = self.extending; self.extending = false },
            _  => { self.extending = false; self.joining = false },
        }

        // GB9c
        match next.graph {
            IC => { self.consonant = true; },
            E  => { /* indic consonant extenders ride-along in conjunct runs */ },
            _ if is_indic_linker(next.code) => {
                if self.consonant { self.linking = true; }
            },
            _  => { self.consonant = false; self.linking = false; },
        }
    }
}

// ~~~~~ GRAPHEME CLUSTER ~~~~~

/// A segmented grapheme cluster with metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphemeCluster<'a> {
    /// Raw utf-8 bytes of the cluster.
    pub bytes: &'a [u8],

    /// Byte offset of this cluster within the source text.
    pub offset: usize,

    /// Display width (1 or 2)
    pub width: u8,
}

impl<'a> GraphemeCluster<'a> {

    // ,,,,,,,,,,,,,,,,,,,,,,
    // [    constructors    ]
    // ''''''''''''''''''''''

    /// Manually contructs a new cluster with a given set of bytes and optical width.
    #[inline]
    pub fn new(bytes: &'a [u8], offset: usize, width: u8) -> Self {
        Self { bytes, offset, width }
    }

    /// Creates an empty cluster.
    #[inline]
    pub fn blank() -> Self {
        Self { bytes: &[], offset: 0, width: 0 }
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


// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

// ~~~~~ SEGMENTATION ~~~~~

pub fn graphemes(text: &str) -> GraphemeSplitter<'_> {
    GraphemeSplitter::new(text)
}

// ~~~~~ UNICODE ~~~~~

/// Extracts the utf-8 unicode scalar at `pos` along with its underlying byte length.
///
/// Because `bytes` ultimately originates from a `&str`, every sequence is well-formed: each call is
/// continued from the last, stepping by the returned width, which guarantees `pos` always starts at
/// a valid codepoint and will not exceed the bounds of `bytes`.
#[inline]
fn extract_scalar(bytes: &[u8], pos: usize) -> (u32, usize) {
    let first = bytes[pos];

    if first < 0x80 {
        (
            first as u32,
            1,
        )
    } else if first < 0xE0 {
        (
            ((first & 0x1F) as u32) << 6
                | (bytes[pos + 1] & 0x3f) as u32,
            2,
        )
    } else if first < 0xf0 {
        (
            ((first & 0x0f) as u32) << 12
                | ((bytes[pos + 1] & 0x3f) as u32) << 6
                |  (bytes[pos + 2] & 0x3f) as u32,
            3,
        )
    } else {
        (
            ((first & 0x07) as u32) << 18
                | ((bytes[pos + 1] & 0x3f) as u32) << 12
                | ((bytes[pos + 2] & 0x3f) as u32) << 6
                |  (bytes[pos + 3] & 0x3f) as u32,
            4,
        )
    }
}

// ~~~~~ MEMBERSHIP ~~~~~

#[inline]
fn is_indic_linker(code: u32) -> bool { INDIC_LINKERS.binary_search(&code).is_ok() }
