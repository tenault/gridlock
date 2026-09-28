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

use crate::unicode::symbols::uax29g::*;


// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// [[    GRAPHEME SPLITTER    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// Iterator over grapheme clusters in a string.
///
/// Conforms to `UAX29-C1-1` by default, or optionally `UAX29-C1-2` with `old()`.
pub struct GraphemeSplitter<'a> {
    bytes: &'a [u8],
    pos:    usize,
    range:  GraphRange,
    last:   Option<UnicodeScalar>,
    legacy: bool,
}

impl<'a> GraphemeSplitter<'a> {

    // ,,,,,,,,,,,,,,,,,,,,,,
    // [    constructors    ]
    // ''''''''''''''''''''''

    #[inline]
    pub fn new(text: &'a str) -> Self {
        Self {
            bytes:  text.as_bytes(),
            pos:    0,
            range:  GraphRange { start: 0, end: 0, graph: GraphType::O },
            last:   None,
            legacy: false,
        }
    }

    #[inline]
    pub fn old(text: &'a str) -> Self {
        Self {
            bytes:  text.as_bytes(),
            pos:    0,
            range:  GraphRange { start: 0, end: 0, graph: GraphType::O },
            last:   None,
            legacy: true,
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
    /// - Hangul syllable arithmetic (permits LV/LVT omission in `UAX29_GRAPHS`)
    /// - Paged binary search (greatly reduces graph lookup space)
    fn identify(&mut self, code: u32) -> GraphType {
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
            if code < s { Ordering::Greater }
            else if code > e { Ordering::Less }
            else { Ordering::Equal }
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
    /// Rules re-ordered by frequency for cost-savings, and rule non-overlap preserves compliance.
    fn is_boundary(
        &self,
        previous: &UnicodeScalar,
        current:  &UnicodeScalar,
        state:    &ClusterState,
        legacy:   bool,
    ) -> bool {
        use GraphType::*;

        // ..... GB3 -- CR × LF .....

        if previous.graph == CR && current.graph == LF { return false; }

        // ..... GB4 -- (Control | CR | LF) ÷ Any .....

        if matches!(previous.graph, C | CR | LF) { return true; }

        // ..... GB5 -- Any ÷ (Control | CR | LF) .....

        if matches!(current.graph,  C | CR | LF) { return true; }

        // ..... GB11 -- ExtPict Extend* ZWJ × ExtPict .....

        if state.joining && current.graph == EP { return false; }

        // ..... GB9 -- Any × (Extend | ZWJ)  .....

        if matches!(current.graph, E | ZW) { return false; }

        // ..... GB6 -- L × (L | V | LV | LVT) .....

        if previous.graph == L && matches!(current.graph, L | V | LV | LVT) { return false; }

        // ..... GB7 -- (LV | V) × (V | T) .....

        if matches!(previous.graph, LV | V) && matches!(current.graph, V | T) { return false; }

        // ..... GB8 -- (LVT | T) × T .....

        if matches!(previous.graph, LVT | T) && current.graph == T { return false; }

        // ..... GB9a -- Any × SpacingMark .....

        if current.graph == SM { return legacy; }

        // ..... GB9b -- Prepend × Any .....

        if previous.graph == P { return legacy; }

        // ..... GB9c -- ConjunctLinker ConjunctExtender* × LinkingConsonant .....

        if current.graph == IC && state.linking { return legacy; }

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

            if self.is_boundary(&current, &next, &state, self.legacy) {
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


// ~~~~~~~~~~~~~~~~~~~~~~~
// [[    CONVENIENCE    ]]
// ~~~~~~~~~~~~~~~~~~~~~~~

/// Iterator over grapheme clusters in a given string.
///
/// Conforms to `UAX29-C1-1`, segmenting into `GraphemeCluster`.
///
/// ```rust,no_run
/// use gridlock::unicode::graphemes;
///
/// for cluster in graphemes("héllo") { /* ... */ }
/// ```
pub fn graphemes(text: &str) -> GraphemeSplitter<'_> { GraphemeSplitter::new(text) }

/// Iterator over legacy grapheme clusters in a given string.
///
/// Conforms to `UAX29-C1-2`, segmenting into `GraphemeCluster`. Unless you have a specific need for
/// clusters which do not conform to GB9a, GB9b, and GB9c, it's best to use `graphemes()` instead.
///
/// ```rust,no_run
/// use gridlock::unicode::legacy_graphemes;
///
/// for cluster in legacy_graphemes("héllo") { /* ... */ }
/// ```
pub fn legacy_graphemes(text: &str) -> GraphemeSplitter<'_> { GraphemeSplitter::old(text) }


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
    /// Whether the cluster suffix currently matches `ExtPict Extend*`.
    extending: bool,
    /// Whether the cluster suffix currently matches `ExtPict Extend* ZWJ`.
    joining: bool,
    /// Whether the cluster suffix currently matches `InCB=Linker InCB=Extend*`.
    linking: bool,
}

impl ClusterState {
    #[inline]
    fn new() -> Self {
        Self {
            regionals: 0,
            extending: false,
            joining:   false,
            linking:   false,
        }
    }

    /// Ingest a unicode scalar and fold it into the current cluster state.
    ///
    /// For indic scripts, Unicode defines the set `\p{InCB=Extend}` as:
    /// ```text
    /// \p{gcb=Extend}
    /// + \p{gcb=ZWJ}
    /// - \p{InCB=Linker}
    /// - \p{InCB=Consonant}
    /// - 0x200c
    /// ```
    /// However, in practice, no indic consonant has an overlap with Extend or ZWJ, so we can safely
    /// ignore the consonant check entirely.
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
            E | ZW if next.code != 0x200c && !is_indic_linker(next.code) => { /* protect state */ },
            _ if is_indic_linker(next.code) => { self.linking = true; },
            _  => { self.linking = false; },
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

    /// Manually contructs a new cluster with a given set of bytes and display width.
    #[inline]
    pub fn from(bytes: &'a [u8], offset: usize, width: u8) -> Self {
        Self { bytes, offset, width }
    }

    /// Creates an empty cluster.
    #[inline]
    pub fn new() -> Self {
        Self { bytes: &[], offset: 0, width: 0 }
    }

    // ,,,,,,,,,,,,,,,,,
    // [    utility    ]
    // '''''''''''''''''

    /// Borrow the raw bytes as a slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] { &self.bytes }

    /// View the bytes as a string.
    #[inline]
    pub fn as_str(&self) -> Option<&str> { std::str::from_utf8(&self.bytes).ok() }

    /// Check if this cluster is empty.
    #[inline]
    pub fn is_empty(&self) -> bool { self.bytes.is_empty() }

    /// Get the length of this cluster in bytes.
    #[inline]
    pub fn len(&self) -> usize { self.bytes.len() }

    /// Iterate over the unicode scalars in this cluster.
    #[inline]
    pub fn scalars(&self) -> ScalarIterator<'_> {
        ScalarIterator { bytes: self.bytes, pos: 0 }
    }
}

// ~~~~~ SCALAR ITERATOR ~~~~~

pub struct ScalarIterator<'a> {
    bytes: &'a [u8],
    pos:   usize,
}

impl<'a> Iterator for ScalarIterator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() { return None; }
        let (scalar, len) = extract_scalar(self.bytes, self.pos);
        self.pos += len;
        Some(scalar)
    }
}

// ~~~~~~~~~~~~~~~~~~~
// [[    UTILITY    ]]
// ~~~~~~~~~~~~~~~~~~~

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
    } else if first < 0xe0 {
        (
            ((first & 0x1f) as u32) << 6
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

/// Returns whether a given code is part of the set `\p{InCB=Linker}`.
#[inline]
fn is_indic_linker(code: u32) -> bool { INDIC_LINKERS.binary_search(&code).is_ok() }


// ~~~~~~~~~~~~~~~~~
// [[    TESTS    ]]
// ~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod extract_scalar_tests {
    use super::*;

    #[test]
    fn single_byte() {
        let bytes = b"h"; // 0x68
        let (code, len) = extract_scalar(bytes, 0);
        assert_eq!(code, 0x68);
        assert_eq!(len, 1);
    }

    #[test]
    fn double_byte() {
        let bytes = "é".as_bytes(); // 0xc3 0xa9
        let (code, len) = extract_scalar(bytes, 0);
        assert_eq!(code, 0xe9);
        assert_eq!(len, 2);
    }

    #[test]
    fn triple_byte() {
        let bytes = "€".as_bytes(); // 0xe2 0x82 0xac
        let (code, len) = extract_scalar(bytes, 0);
        assert_eq!(code, 0x20ac);
        assert_eq!(len, 3);
    }

    #[test]
    fn quadruple_byte() {
        let bytes = "𐍈".as_bytes(); // 0xf0 0x90 0x8d 0x88
        let (code, len) = extract_scalar(bytes, 0);
        assert_eq!(code, 0x10348);
        assert_eq!(len, 4);
    }

    #[test]
    fn sequential_extraction() {
        let bytes = "aβ🀄".as_bytes(); // 0x61 0xce 0xb2 0xf0 0x9f 0x80 0x84

        let (c1, l1) = extract_scalar(bytes, 0);       // 0x61
        let (c2, l2) = extract_scalar(bytes, l1);      // 0xce 0xb2
        let (c3, _)  = extract_scalar(bytes, l1 + l2); // 0xf0 0x9f 0x80 0x84
        assert_eq!(c1, 0x61);
        assert_eq!(c2, 0x03b2);
        assert_eq!(c3, 0x1f004);
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn gb3() {
        let prev  = UnicodeScalar { code: 0x0d, len: 1, graph: GraphType::CR };
        let next  = UnicodeScalar { code: 0x0a, len: 1, graph: GraphType::LF };
        let state = ClusterState::new();

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb4() {
        let prev  = UnicodeScalar { code: 0x00, len: 1, graph: GraphType::C };
        let next  = UnicodeScalar { code: 0x65, len: 1, graph: GraphType::O };
        let state = ClusterState::new();

        let splitter = GraphemeSplitter::new("");
        assert!(splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb6() {
        let prev  = UnicodeScalar { code: 0x1100, len: 3, graph: GraphType::L };
        let next  = UnicodeScalar { code: 0x1161, len: 3, graph: GraphType::V };
        let state = ClusterState::new();

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb8() {
        let prev  = UnicodeScalar { code: 0xac01, len: 3, graph: GraphType::LVT };
        let next  = UnicodeScalar { code: 0x11a7, len: 3, graph: GraphType::T };
        let state = ClusterState::new();

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb9() {
        let prev  = UnicodeScalar { code: 0x0069, len: 1, graph: GraphType::O };
        let next  = UnicodeScalar { code: 0x0301, len: 2, graph: GraphType::E };
        let state = ClusterState::new();

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb11() {
        let base = UnicodeScalar { code: 0x1f469, len: 4, graph: GraphType::EP };
        let prev = UnicodeScalar { code: 0x200d,  len: 3, graph: GraphType::ZW };
        let next = UnicodeScalar { code: 0x1f469, len: 4, graph: GraphType::EP };
        let mut state = ClusterState::new();
        state.fold(&base);
        state.fold(&prev);

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }

    #[test]
    fn gb12_odd() {
        let prev = UnicodeScalar { code: 0x1f1fa, len: 4, graph: GraphType::RI };
        let next = UnicodeScalar { code: 0x1f1f8, len: 4, graph: GraphType::RI };
        let mut state = ClusterState::new();
        state.fold(&prev);

        let splitter = GraphemeSplitter::new("");
        assert!(!splitter.is_boundary(&prev, &next, &state));
    }


    #[test]
    fn gb12_even() {
        let prev = UnicodeScalar { code: 0x1f1fa, len: 4, graph: GraphType::RI };
        let next = UnicodeScalar { code: 0x1f1f8, len: 4, graph: GraphType::RI };
        let mut state = ClusterState::new();
        state.fold(&prev);
        state.fold(&next);

        let splitter = GraphemeSplitter::new("");
        assert!(splitter.is_boundary(&prev, &next, &state));
    }
}

#[cfg(test)]
mod identify_tests {
    use super::*;

    #[test]
    fn ascii() {
        let mut splitter = GraphemeSplitter::new("ABC");
        let scalar = splitter.extract(0);
        assert_eq!(scalar.graph, GraphType::O);
    }

    #[test]
    fn hangul_lv() {
        let mut splitter = GraphemeSplitter::new("가");
        let scalar = splitter.extract(0);
        assert_eq!(scalar.graph, GraphType::LV);
    }

    #[test]
    fn hangul_lvt() {
        let mut splitter = GraphemeSplitter::new("갑");
        let scalar = splitter.extract(0);
        assert_eq!(scalar.graph, GraphType::LVT);
    }

    #[test]
    fn regional_indicator() {
        let mut splitter = GraphemeSplitter::new("🇺🇸");
        let s1 = splitter.extract(0);
        let s2 = splitter.extract(4);
        assert_eq!(s1.graph, GraphType::RI);
        assert_eq!(s2.graph, GraphType::RI);
    }

    #[test]
    fn extend_and_zwj() {
        let mut splitter = GraphemeSplitter::new("e\u{0301}\u{200D}");
        let s = splitter.extract(0);
        let e = splitter.extract(1);
        let z = splitter.extract(3);
        assert_eq!(s.graph, GraphType::O);
        assert_eq!(e.graph, GraphType::E);
        assert_eq!(z.graph, GraphType::ZW);
    }

    #[test]
    fn extended_pictographic() {
        let mut splitter = GraphemeSplitter::new("👨‍👩‍👧");
        let scalar = splitter.extract(0);
        assert_eq!(scalar.graph, GraphType::EP);
    }
}

#[cfg(test)]
mod cluster_state_tests {
    use super::*;

    #[test]
    fn initial_is_empty() {
        let state = ClusterState::new();
        assert_eq!(state.regionals, 0);
        assert!(!state.extending);
        assert!(!state.joining);
        assert!(!state.linking);
    }

    #[test]
    fn regionals_resets_on_non_ri() {
        let r1 = UnicodeScalar { code: 0x1f1fa, len: 4, graph: GraphType::RI };
        let r2 = UnicodeScalar { code: 0x1f1f8, len: 4, graph: GraphType::RI };
        let a  = UnicodeScalar { code: 0x0065,  len: 1, graph: GraphType::O  };
        let mut state = ClusterState::new();

        state.fold(&r1);
        assert_eq!(state.regionals, 1);

        state.fold(&r2);
        assert_eq!(state.regionals, 2);

        state.fold(&a);
        assert_eq!(state.regionals, 0);
    }

    #[test]
    fn joining_requires_extend_before_zwj() {
        let ep = UnicodeScalar { code: 0x1f468, len: 4, graph: GraphType::EP };
        let zw = UnicodeScalar { code: 0x200d,  len: 3, graph: GraphType::ZW };
        let mut state = ClusterState::new();

        state.fold(&ep);
        assert!(state.extending);
        assert!(!state.joining);

        state.fold(&zw);
        assert!(!state.extending);
        assert!(state.joining);
    }

    #[test]
    fn joining_resets_on_non_zwj() {
        let ep = UnicodeScalar { code: 0x1f468, len: 4, graph: GraphType::EP };
        let zw = UnicodeScalar { code: 0x200d,  len: 3, graph: GraphType::ZW };
        let e  = UnicodeScalar { code: 0x0301,  len: 2, graph: GraphType::E  };
        let mut state = ClusterState::new();

        state.fold(&ep);
        state.fold(&zw);
        assert!(state.joining);

        state.fold(&e);
        assert!(!state.joining);
        assert!(!state.extending);
    }
}

#[cfg(test)]
mod grapheme_cluster_tests {
    use super::*;

    #[test]
    fn from_builds_correctly() {
        let bytes   = "ñ".as_bytes();
        let cluster = GraphemeCluster::from(bytes, 5, 1);
        assert_eq!(cluster.offset, 5);
        assert_eq!(cluster.width,  1);
        assert_eq!(cluster.len(),  2);
        assert_eq!(cluster.as_str(), Some("ñ"));
    }

    #[test]
    fn scalar_iterator_works() {
        let bytes   = "ñ".as_bytes();
        let cluster = GraphemeCluster::from(bytes, 5, 1);
        let scalars: Vec<u32> = cluster.scalars().collect();
        assert_eq!(scalars, vec![0x00f1]);
    }

    #[test]
    fn empty_cluster_is_empty() {
        let cluster = GraphemeCluster::new();
        assert!(cluster.is_empty());
        assert_eq!(cluster.len(), 0);
        assert_eq!(cluster.offset, 0);
        assert_eq!(cluster.as_str(), Some(""));
    }
}

#[cfg(test)]
mod grapheme_splitter_tests {
    use super::*;

    #[test]
    fn empty_string_makes_no_clusters() {
        let clusters: Vec<GraphemeCluster> = graphemes("").collect();
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn ascii_splits_correctly() {
        let text = "hello";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 5);
        assert_eq!(clusters[0].as_str(), Some("h"));
        assert_eq!(clusters[4].as_str(), Some("o"));
    }

    #[test]
    fn combining_accent_stays_with_base() {
        let text = "e\u{0301}";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].as_str(), Some("é"));
    }

    #[test]
    fn flag_pair_stays_paired() {
        let text = "🇺🇸";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn emoji_family_stays_together() {
        let text = "👨‍👩‍👧‍👦";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn hangul_syllable_stays_clustered() {
        let text = "가";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn crlf_stays_clustered() {
        let text = "\r\n";
        let clusters: Vec<GraphemeCluster> = graphemes(text).collect();
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn size_hint_is_mostly_accurate() {
        let text = "hello";
        let (lo, hi) = graphemes(text).size_hint();
        assert!(lo > 0);
        assert!(hi.unwrap() <= 5);
    }
}
