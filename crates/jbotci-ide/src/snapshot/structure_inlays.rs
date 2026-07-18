#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_output::{
    BracketRenderOptions, BracketSourceConstruct, BracketSourceFragment, BracketSourceRange,
    pretty_bracket_source_fragments_with_options,
    pretty_recovered_syntax_bracket_source_fragments_with_options,
};
use jbotci_source::SourceSpan;
use jbotci_syntax::{SyntaxRecoveryParse, SyntaxRecoveryParseData};
use serde::{Deserialize, Serialize};

use super::DocumentSnapshot;
use crate::LineIndex;

/// A glyph vocabulary and selection policy applied to tree-anchored fragments.
///
/// New decoration systems such as pandi belong here as new profiles; the
/// fragment traversal and range/anchor logic remain shared.
#[invariant(true)]
#[invariant(::RawBrackets { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "profile")]
pub enum DecorationProfile {
    RawBrackets {
        #[serde(flatten)]
        options: RawBracketsOptions,
    },
}

impl Default for DecorationProfile {
    #[requires(true)]
    #[ensures(matches!(ret, Self::RawBrackets { .. }))]
    fn default() -> Self {
        Self::RawBrackets {
            options: RawBracketsOptions::default(),
        }
    }
}

/// Selection options for the raw-brackets decoration profile.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RawBracketsOptions {
    /// One-based maximum structural nesting depth. Zero suppresses all hints.
    pub max_nesting_depth: Option<usize>,
    pub constructs: StructureConstructFilter,
}

impl Default for RawBracketsOptions {
    #[requires(true)]
    #[ensures(ret.max_nesting_depth.is_none())]
    #[ensures(ret.constructs == StructureConstructFilter::All)]
    fn default() -> Self {
        Self {
            max_nesting_depth: None,
            constructs: StructureConstructFilter::All,
        }
    }
}

/// Grammar-boundary subset selected by a decoration profile.
#[invariant(true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructureConstructFilter {
    #[default]
    All,
    SumtiBoundaries,
    BridiTails,
}

/// Whether an inlay opens or closes one structural fragment.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructureInlayKind {
    Open,
    Close,
}

/// One transport-independent decoration anchored to a zero-width source span.
#[invariant(anchor.is_empty(), "structure inlays must have zero-width anchors")]
#[invariant(!label.is_empty(), "structure inlays must have visible labels")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureInlay {
    pub anchor: SourceSpan,
    pub label: String,
    pub kind: StructureInlayKind,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DecorationFragment {
    range: Option<BracketSourceRange>,
    constructs: Vec<BracketSourceConstruct>,
    children: Vec<DecorationFragment>,
}

impl DecorationFragment {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn ranges_are_within(&self, byte_len: usize) -> bool {
        self.range.is_none_or(|range| range.byte_end <= byte_len)
            && self
                .children
                .iter()
                .all(|child| child.ranges_are_within(byte_len))
    }
}

#[invariant(!open.is_empty() && !close.is_empty())]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundaryLabels {
    open: &'static str,
    close: &'static str,
}

impl DecorationProfile {
    #[requires(true)]
    #[ensures(true)]
    fn boundary_labels(
        &self,
        zero_based_depth: usize,
        constructs: &[BracketSourceConstruct],
    ) -> Option<BoundaryLabels> {
        match self {
            Self::RawBrackets { options } => {
                let nesting_depth = zero_based_depth
                    .checked_add(1)
                    .expect("a syntax tree cannot exhaust usize nesting depth");
                if options
                    .max_nesting_depth
                    .is_some_and(|maximum| nesting_depth > maximum)
                    || !options.constructs.includes(constructs)
                {
                    return None;
                }
                let (open, close) = raw_bracket_pair(zero_based_depth);
                Some(new!(BoundaryLabels { open, close }))
            }
        }
    }
}

impl StructureConstructFilter {
    #[requires(true)]
    #[ensures(true)]
    fn includes(self, constructs: &[BracketSourceConstruct]) -> bool {
        match self {
            Self::All => true,
            Self::SumtiBoundaries => constructs.contains(&BracketSourceConstruct::Sumti),
            Self::BridiTails => constructs.contains(&BracketSourceConstruct::BridiTail),
        }
    }
}

impl DocumentSnapshot {
    /// Return profile decorations whose anchor positions lie in `range`.
    ///
    /// Both input and output remain in source coordinates. Transport adapters
    /// resolve their requested ranges and the returned zero-width anchors via
    /// [`LineIndex`] using their negotiated position encoding.
    #[requires(range.byte_end <= self.text.len())]
    #[requires(range.char_end <= self.line_index.char_len())]
    #[ensures(ret.iter().all(|inlay| range.byte_start <= inlay.anchor.byte_start && inlay.anchor.byte_start < range.byte_end))]
    pub fn structure_inlays(
        &self,
        profile: &DecorationProfile,
        range: &SourceSpan,
    ) -> Vec<StructureInlay> {
        let mut inlays = Vec::new();
        collect_structure_inlays(
            &self.structure_fragments,
            profile,
            range,
            &self.line_index,
            0,
            &mut inlays,
        );
        inlays
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn build_decoration_fragments(
    parse: &SyntaxRecoveryParse,
    source: &str,
) -> Vec<DecorationFragment> {
    let options = BracketRenderOptions::default();
    let source_fragments = match parse.as_data() {
        data!(SyntaxRecoveryParse::Valid { parse }) => {
            pretty_bracket_source_fragments_with_options(parse.parse_tree.as_ref(), source, options)
        }
        data!(SyntaxRecoveryParse::Recovered { parse }) => {
            pretty_recovered_syntax_bracket_source_fragments_with_options(parse, source, options)
        }
    }
    .expect("a snapshot's recovered bracket fragments must match its source");
    collect_decoration_fragments(&source_fragments)
}

#[requires(true)]
#[ensures(true)]
fn collect_decoration_fragments(fragments: &[BracketSourceFragment]) -> Vec<DecorationFragment> {
    fragments
        .iter()
        .filter_map(|fragment| match fragment {
            BracketSourceFragment::Text { .. } => None,
            BracketSourceFragment::Span {
                range,
                constructs,
                children,
            } => Some(DecorationFragment {
                range: *range,
                constructs: constructs.clone(),
                children: collect_decoration_fragments(children),
            }),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn collect_structure_inlays(
    fragments: &[DecorationFragment],
    profile: &DecorationProfile,
    query_range: &SourceSpan,
    line_index: &LineIndex,
    depth: usize,
    inlays: &mut Vec<StructureInlay>,
) {
    for fragment in fragments {
        let labels = profile.boundary_labels(depth, &fragment.constructs);
        if let (Some(range), Some(labels)) = (fragment.range, labels)
            && anchor_is_in_range(range.byte_start, query_range)
        {
            inlays.push(structure_inlay(
                range.byte_start,
                labels.open,
                StructureInlayKind::Open,
                line_index,
            ));
        }

        collect_structure_inlays(
            &fragment.children,
            profile,
            query_range,
            line_index,
            depth
                .checked_add(1)
                .expect("a syntax tree cannot exhaust usize nesting depth"),
            inlays,
        );

        if let (Some(range), Some(labels)) = (fragment.range, labels)
            && anchor_is_in_range(range.byte_end, query_range)
        {
            inlays.push(structure_inlay(
                range.byte_end,
                labels.close,
                StructureInlayKind::Close,
                line_index,
            ));
        }
    }
}

#[requires(query_range.byte_start <= query_range.byte_end)]
#[ensures(true)]
fn anchor_is_in_range(byte_offset: usize, query_range: &SourceSpan) -> bool {
    query_range.byte_start <= byte_offset && byte_offset < query_range.byte_end
}

#[requires(byte_offset <= line_index.byte_len())]
#[requires(!label.is_empty())]
#[ensures(ret.anchor.byte_start == byte_offset)]
fn structure_inlay(
    byte_offset: usize,
    label: &str,
    kind: StructureInlayKind,
    line_index: &LineIndex,
) -> StructureInlay {
    let offsets = line_index.offsets_for_byte(byte_offset);
    assert_eq!(
        offsets.byte, byte_offset,
        "tree-anchored bracket ranges must end on Unicode scalar boundaries",
    );
    let anchor = SourceSpan::new(None, byte_offset, byte_offset, offsets.char, offsets.char)
        .expect("equal source offsets are ordered");
    new!(StructureInlay {
        anchor,
        label: label.to_owned(),
        kind,
    })
}

#[requires(true)]
#[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
fn raw_bracket_pair(depth: usize) -> (&'static str, &'static str) {
    match depth % 3 {
        0 => ("(", ")"),
        1 => ("[", "]"),
        _ => ("{", "}"),
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use bityzba::data;

    use super::*;
    use crate::{PositionEncoding, PositionRange};

    const STRUCTURE_INLAY_SNAPSHOT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/structure-inlays.snapshot.txt",
    ));

    #[requires(true)]
    #[ensures(ret.byte_end == snapshot.text.len())]
    fn whole_document_span(snapshot: &DocumentSnapshot) -> SourceSpan {
        SourceSpan::new(
            None,
            0,
            snapshot.text.len(),
            0,
            snapshot.line_index.char_len(),
        )
        .expect("whole-document offsets are ordered")
    }

    #[requires(true)]
    #[ensures(matches!(ret, DecorationProfile::RawBrackets { .. }))]
    fn raw_profile(
        max_nesting_depth: Option<usize>,
        constructs: StructureConstructFilter,
    ) -> DecorationProfile {
        DecorationProfile::RawBrackets {
            options: RawBracketsOptions {
                max_nesting_depth,
                constructs,
            },
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fixture_profiles_match_tree_anchored_golden() {
        let fixtures = [
            (
                "nested-quotes",
                "mi cusku lu do cusku lu mi klama li'u li'u\n",
            ),
            ("recovered-mid-document", "mi ku i do viska le mlatu\n"),
        ];
        let profiles = [
            ("full", raw_profile(None, StructureConstructFilter::All)),
            (
                "depth-2",
                raw_profile(Some(2), StructureConstructFilter::All),
            ),
            (
                "sumti-boundaries",
                raw_profile(None, StructureConstructFilter::SumtiBoundaries),
            ),
            (
                "bridi-tails",
                raw_profile(None, StructureConstructFilter::BridiTails),
            ),
        ];
        let mut actual = String::new();

        for (fixture_name, source) in fixtures {
            let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
            writeln!(actual, "fixture {fixture_name}").expect("string writes cannot fail");
            writeln!(actual, "source {source:?}").expect("string writes cannot fail");
            writeln!(
                actual,
                "parse {}",
                match snapshot.parse.as_data() {
                    data!(SyntaxRecoveryParse::Valid { .. }) => "valid",
                    data!(SyntaxRecoveryParse::Recovered { .. }) => "recovered",
                },
            )
            .expect("string writes cannot fail");

            for (profile_name, profile) in &profiles {
                writeln!(actual, "profile {profile_name}").expect("string writes cannot fail");
                for inlay in snapshot.structure_inlays(profile, &whole_document_span(&snapshot)) {
                    let position = snapshot
                        .line_index
                        .position_for_byte(inlay.anchor.byte_start, PositionEncoding::Utf32);
                    writeln!(
                        actual,
                        "  {}:{} byte={} {:?} {:?}",
                        position.line,
                        position.column,
                        inlay.anchor.byte_start,
                        inlay.kind,
                        inlay.label,
                    )
                    .expect("string writes cannot fail");
                }
            }
            actual.push('\n');
        }

        assert_eq!(actual, STRUCTURE_INLAY_SNAPSHOT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_quotes_depth_and_range_are_structural() {
        let nested_source = "mi cusku lu do cusku lu mi klama li'u li'u\n";
        let nested = DocumentSnapshot::new(nested_source.to_owned(), 1);
        let nested_range = whole_document_span(&nested);
        let full = nested.structure_inlays(&DecorationProfile::default(), &nested_range);
        let depth_two = nested.structure_inlays(
            &raw_profile(Some(2), StructureConstructFilter::All),
            &nested_range,
        );
        assert!(!full.is_empty());
        assert!(depth_two.len() < full.len());
        assert!(
            nested
                .structure_inlays(
                    &raw_profile(Some(0), StructureConstructFilter::All),
                    &nested_range,
                )
                .is_empty(),
            "a zero depth limit suppresses every structural level",
        );
        let quoted_start = nested_source.find("do cusku").expect("quoted text fixture");
        let quoted_end = nested_source
            .rfind("li'u")
            .expect("outer quote terminator fixture");
        assert!(
            full.iter().any(|inlay| {
                quoted_start < inlay.anchor.byte_start && inlay.anchor.byte_start < quoted_end
            }),
            "nested quote syntax must retain anchors inside its quoted text",
        );

        let recovered_source = "mi ku i do viska le mlatu\n";
        let recovered = DocumentSnapshot::new(recovered_source.to_owned(), 1);
        assert!(matches!(
            recovered.parse.as_data(),
            data!(SyntaxRecoveryParse::Recovered { .. })
        ));
        let recovered_inlays = recovered.structure_inlays(
            &DecorationProfile::default(),
            &whole_document_span(&recovered),
        );
        assert!(
            recovered_inlays
                .iter()
                .any(|inlay| inlay.anchor.byte_start > "mi ku".len()),
            "recovered syntax after the error must still produce inlays",
        );

        let subset_start = recovered_source.find("do").expect("subset fixture");
        let subset_end = recovered_source.len() - 1;
        let positions = PositionRange::new(
            recovered
                .line_index
                .position_for_byte(subset_start, PositionEncoding::Utf32),
            recovered
                .line_index
                .position_for_byte(subset_end, PositionEncoding::Utf32),
        );
        let subset_range =
            recovered
                .line_index
                .span_for_positions(&positions, PositionEncoding::Utf32, None);
        let subset = recovered.structure_inlays(&DecorationProfile::default(), &subset_range);
        assert!(!subset.is_empty());
        assert!(subset.len() < recovered_inlays.len());
        assert!(subset.iter().all(|inlay| {
            subset_range.byte_start <= inlay.anchor.byte_start
                && inlay.anchor.byte_start < subset_range.byte_end
        }));
    }
}
