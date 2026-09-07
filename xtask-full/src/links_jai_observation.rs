//! Actual epoch-10 fixture observations, independent of expectation status and disposition.
//!
//! The selected coverage is an input, so removing an expectation cannot silently remove a
//! surface. Stage labels describe the executable being run; they never select a grammar.

use std::path::PathBuf;

use anyhow::{Context, Result};
use bityzba::{invariant, new, requires};
use clap::{Args, ValueEnum};
use jbotci_syntax::generated_model::{self as model, recovered};
use serde::{Deserialize, Serialize};

use super::{
    JsonRenderOptions, MorphologyOptions, ParseOptions, SourceId, TreeRenderOptions,
    analyze_generated_references, compact_generated_model_json_string_with_options, fixtures,
    load_fixture_path, morphology_error_diagnostic_expectation_items,
    morphology_warning_diagnostic_expectation_items,
    parse_syntax_tree_recovered_with_source_and_options, parse_syntax_tree_with_source_and_options,
    pretty_generated_model_tree_with_options, recovered_syntax_diagnostic_expectation_items,
    recovered_syntax_tree_expectation,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
    syntax_error_diagnostic_expectation_items, syntax_warning_diagnostic_expectation_items,
};

#[invariant(true)]
#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Stage {
    Base,
    #[serde(rename = "c-a")]
    #[value(name = "c-a")]
    CA,
    #[serde(rename = "c-b")]
    #[value(name = "c-b")]
    CB,
}

#[invariant(true)]
#[derive(Debug, Args)]
pub(super) struct ObserveArgs {
    /// JSON array of fixture paths, fixed coverage, and optional frozen BE/BEI anchors.
    #[arg(long)]
    selection: PathBuf,
    #[arg(long, value_enum)]
    stage: Stage,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LinkMarker {
    Be,
    Bei,
}

#[invariant(byte_start < byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkAnchor {
    marker: LinkMarker,
    byte_start: usize,
    byte_end: usize,
}

#[invariant(recovery_max_errors.is_none_or(|limit| limit > 0))]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    path: PathBuf,
    target: Option<LinkAnchor>,
    refs: bool,
    gentufa_tree: bool,
    gentufa_json: bool,
    recovered: bool,
    recovery_max_errors: Option<usize>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum LinkOwner {
    FullLinkedTerm,
    ConnectedLinkedTerm,
    BoundLinkedTermConnection,
    PlaceTaggedLinkedSumti,
    TenseTaggedLinkedSumti,
    PlainLinkedSumti,
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct LinkObservation {
    anchor: LinkAnchor,
    owner: LinkOwner,
}

#[invariant(true)]
#[invariant(::Found => true)]
#[invariant(::Ambiguous => true)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum TargetObservation<T> {
    NotRequested,
    Found { link: T },
    Missing,
    Ambiguous { matches: Vec<T> },
}

#[invariant(true)]
#[invariant(::Success => true)]
#[invariant(::Failure => true)]
#[invariant(::BlockedByMorphology => true)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum StrictObservation {
    Success {
        raw: String,
        target: TargetObservation<LinkObservation>,
        diagnostics: Vec<fixtures::DiagnosticExpectation>,
    },
    Failure {
        error: String,
        diagnostics: Vec<fixtures::DiagnosticExpectation>,
    },
    BlockedByMorphology {
        error: String,
        diagnostics: Vec<fixtures::DiagnosticExpectation>,
    },
}

#[invariant(true)]
#[invariant(::Success => true)]
#[invariant(::Failure => true)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum Surface<T> {
    Inapplicable,
    BlockedBySyntaxFailure,
    BlockedByMorphologyFailure,
    Success { value: T },
    Failure { error: String },
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct RecoveredObservation {
    parser_status: fixtures::ExpectationStatus,
    raw: String,
    errors: Vec<String>,
    diagnostics: Vec<fixtures::DiagnosticExpectation>,
    tree: fixtures::RecoveredTreeExpectation,
    target: TargetObservation<RecoveredLinkObservation>,
}

#[invariant(byte_start < byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct ParsedExtent {
    byte_start: usize,
    byte_end: usize,
}

/// The *containing field* wrapper, not the wrapper seen by the Full classifier.
/// The optional extent describes surviving source tokens separately from wrapper validity.
#[invariant(true)]
#[invariant(::Valid => true)]
#[invariant(::Prefix => true)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum RecoveredLinkField {
    Valid {
        owner: LinkOwner,
        parsed_extent: Option<ParsedExtent>,
    },
    Prefix {
        owner: LinkOwner,
        parsed_extent: Option<ParsedExtent>,
    },
    Error,
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct RecoveredLinkObservation {
    anchor: LinkAnchor,
    parent_parsed_extent: Option<ParsedExtent>,
    field: RecoveredLinkField,
}

#[invariant(true)]
#[derive(Default)]
struct ParsedExtentProbe {
    extent: Option<ParsedExtent>,
}

impl<'tree> jbotci_tree::TreeVisitor<'tree> for ParsedExtentProbe {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let recovered::AtomRef::Token(token) = atom;
        for span in token.source_spans() {
            self.extent = Some(new!(ParsedExtent {
                byte_start: self
                    .extent
                    .map_or(span.byte_start, |old| old.byte_start.min(span.byte_start)),
                byte_end: self
                    .extent
                    .map_or(span.byte_end, |old| old.byte_end.max(span.byte_end)),
            }));
        }
    }
    // Recovery items are deliberately not parsed-value evidence. The complete raw tree and
    // the independently captured recovery projection still retain every such item.
}

#[requires(true)]
#[ensures(true)]
fn parsed_extent(node: &impl recovered::TreeNode) -> Option<ParsedExtent> {
    let mut probe = ParsedExtentProbe::default();
    recovered::TreeNode::visit_in_order(node, &mut probe);
    probe.extent
}

#[requires(true)]
#[ensures(ret.is_none() == matches!(slot, recovered::Recovered::Error(_)))]
fn parsed_value<T>(slot: &recovered::Recovered<T>) -> Option<&T> {
    match slot {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(prefix) => Some(&prefix.value),
        recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_owner(value: &recovered::LinkedTermSyntax) -> LinkOwner {
    match value {
        recovered::LinkedTermSyntax::FullLinkedTerm(_) => LinkOwner::FullLinkedTerm,
        recovered::LinkedTermSyntax::ConnectedLinkedTerm(_) => LinkOwner::ConnectedLinkedTerm,
        recovered::LinkedTermSyntax::BoundLinkedTermConnection(_) => {
            LinkOwner::BoundLinkedTermConnection
        }
        recovered::LinkedTermSyntax::PlaceTaggedLinkedSumti(_) => LinkOwner::PlaceTaggedLinkedSumti,
        recovered::LinkedTermSyntax::TenseTaggedLinkedSumti(_) => LinkOwner::TenseTaggedLinkedSumti,
        recovered::LinkedTermSyntax::PlainLinkedSumti(_) => LinkOwner::PlainLinkedSumti,
    }
}

#[invariant(true)]
#[derive(Default)]
struct RecoveredLinkCollector {
    links: Vec<RecoveredLinkObservation>,
}

impl RecoveredLinkCollector {
    #[requires(true)]
    #[ensures(self.links.len() >= old(self.links.len()))]
    fn record(
        &mut self,
        marker: LinkMarker,
        token: &recovered::Recovered<jbotci_syntax::tree::Token>,
        link: &recovered::Recovered<recovered::LinkedTermSyntax>,
        parent: &impl recovered::TreeNode,
    ) {
        let Some(token) = parsed_value(token) else {
            return; // No sourced marker means no claimed target, even if the field exists.
        };
        let spans = token.source_spans();
        let (Some(first), Some(last)) = (spans.first(), spans.last()) else {
            return;
        };
        let field = match link {
            recovered::Recovered::Valid(value) => RecoveredLinkField::Valid {
                owner: recovered_owner(value),
                parsed_extent: parsed_extent(value.as_ref()),
            },
            recovered::Recovered::Prefix(prefix) => RecoveredLinkField::Prefix {
                owner: recovered_owner(&prefix.value),
                parsed_extent: parsed_extent(prefix.value.as_ref()),
            },
            recovered::Recovered::Error(_) => RecoveredLinkField::Error,
        };
        self.links.push(RecoveredLinkObservation {
            anchor: new!(LinkAnchor {
                marker,
                byte_start: first.byte_start,
                byte_end: last.byte_end
            }),
            parent_parsed_extent: parsed_extent(parent),
            field,
        });
    }
}

impl<'tree> recovered::TreeWalker<'tree> for RecoveredLinkCollector {
    #[requires(true)]
    #[ensures(self.links.len() >= old(self.links.len()))]
    fn walk_linkargs(&mut self, node: &'tree recovered::LinkargsSyntax) {
        self.record(LinkMarker::Be, &node.be.value, &node.first_link, node);
        recovered::walk::linkargs(self, node);
    }

    #[requires(true)]
    #[ensures(self.links.len() >= old(self.links.len()))]
    fn walk_bei_link(&mut self, node: &'tree recovered::BeiLinkSyntax) {
        self.record(LinkMarker::Bei, &node.bei.value, &node.link, node);
        recovered::walk::bei_link(self, node);
    }
}

#[requires(true)]
#[ensures(target.is_none() -> matches!(ret, TargetObservation::NotRequested))]
fn recovered_target_observation(
    tree: &recovered::TextSyntax,
    target: Option<&LinkAnchor>,
) -> TargetObservation<RecoveredLinkObservation> {
    let Some(target) = target else {
        return TargetObservation::NotRequested;
    };
    let mut collector = RecoveredLinkCollector::default();
    recovered::TreeWalkable::walk_with(tree, &mut collector);
    collector.links.retain(|link| link.anchor == *target);
    match collector.links.len() {
        0 => TargetObservation::Missing,
        1 => TargetObservation::Found {
            link: collector.links.pop().expect("one matching link"),
        },
        _ => TargetObservation::Ambiguous {
            matches: collector.links,
        },
    }
}

#[invariant(true)]
#[derive(Debug, Serialize)]
struct Observation {
    stage: Stage,
    id: String,
    path: PathBuf,
    source: String,
    dialect: Option<String>,
    target: Option<LinkAnchor>,
    recovery_max_errors: Option<usize>,
    syntax: StrictObservation,
    refs: Surface<String>,
    gentufa_tree: Surface<String>,
    gentufa_json: Surface<String>,
    recovered: Surface<RecoveredObservation>,
}

#[invariant(true)]
#[derive(Default)]
struct LinkCollector {
    links: Vec<LinkObservation>,
}

impl LinkCollector {
    #[requires(true)]
    #[ensures(self.links.len() == old(self.links.len()) + 1)]
    fn record(
        &mut self,
        marker: LinkMarker,
        token: &jbotci_syntax::tree::Token,
        link: &model::LinkedTermSyntax,
    ) {
        let spans = token.source_spans();
        let first = spans
            .first()
            .expect("a parsed BE/BEI has source attribution");
        let last = spans
            .last()
            .expect("a parsed BE/BEI has source attribution");
        let owner = match link {
            model::LinkedTermSyntax::FullLinkedTerm(_) => LinkOwner::FullLinkedTerm,
            model::LinkedTermSyntax::ConnectedLinkedTerm(_) => LinkOwner::ConnectedLinkedTerm,
            model::LinkedTermSyntax::BoundLinkedTermConnection(_) => {
                LinkOwner::BoundLinkedTermConnection
            }
            model::LinkedTermSyntax::PlaceTaggedLinkedSumti(_) => LinkOwner::PlaceTaggedLinkedSumti,
            model::LinkedTermSyntax::TenseTaggedLinkedSumti(_) => LinkOwner::TenseTaggedLinkedSumti,
            model::LinkedTermSyntax::PlainLinkedSumti(_) => LinkOwner::PlainLinkedSumti,
        };
        self.links.push(LinkObservation {
            anchor: new!(LinkAnchor {
                marker,
                byte_start: first.byte_start,
                byte_end: last.byte_end
            }),
            owner,
        });
    }
}

impl<'tree> model::TreeWalker<'tree> for LinkCollector {
    #[requires(true)]
    #[ensures(self.links.len() > old(self.links.len()))]
    fn walk_linkargs(&mut self, node: &'tree model::LinkargsSyntax) {
        self.record(LinkMarker::Be, &node.be.value, &node.first_link);
        model::walk::linkargs(self, node);
    }

    #[requires(true)]
    #[ensures(self.links.len() > old(self.links.len()))]
    fn walk_bei_link(&mut self, node: &'tree model::BeiLinkSyntax) {
        self.record(LinkMarker::Bei, &node.bei.value, &node.link);
        model::walk::bei_link(self, node);
    }
}

#[requires(true)]
#[ensures(target.is_none() -> matches!(ret, TargetObservation::NotRequested))]
fn target_observation(
    tree: &model::TextSyntax,
    target: Option<&LinkAnchor>,
) -> TargetObservation<LinkObservation> {
    let Some(target) = target else {
        return TargetObservation::NotRequested;
    };
    let mut collector = LinkCollector::default();
    model::TreeWalkable::walk_with(tree, &mut collector);
    collector.links.retain(|link| link.anchor == *target);
    match collector.links.len() {
        0 => TargetObservation::Missing,
        1 => TargetObservation::Found {
            link: collector.links.pop().expect("one matching link"),
        },
        _ => TargetObservation::Ambiguous {
            matches: collector.links,
        },
    }
}

#[requires(true)]
#[ensures(!applicable -> matches!(ret, Surface::Inapplicable))]
fn blocked<T>(applicable: bool, morphology: bool) -> Surface<T> {
    if !applicable {
        Surface::Inapplicable
    } else if morphology {
        Surface::BlockedByMorphologyFailure
    } else {
        Surface::BlockedBySyntaxFailure
    }
}

#[requires(true)]
#[ensures(true)]
fn observe(stage: Stage, request: Request) -> Result<Observation> {
    let fixture = load_fixture_path(&request.path)?;
    let case = &fixture.test_case;
    let dialect = case.dialect_definition()?;
    let options = ParseOptions::default().with_dialect_definition(&dialect);
    let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &case.lojban,
        &MorphologyOptions::default().with_dialect_definition(&dialect),
        Some(SourceId("<fixture>".to_owned())),
    )
    .into_data();
    let morphology_diagnostics =
        morphology_warning_diagnostic_expectation_items(&case.lojban, &attempt.warnings);
    // Assemble only completed observations: no placeholder parse status can escape an early
    // return. In particular, a strict failure has neither a strict tree nor a successor anchor.
    let assemble = |syntax, refs, gentufa_tree, gentufa_json, recovered| Observation {
        stage,
        id: case.id.clone(),
        path: request.path.clone(),
        source: case.lojban.clone(),
        dialect: case.dialect.clone(),
        target: request.target.clone(),
        recovery_max_errors: request.recovery_max_errors,
        syntax,
        refs,
        gentufa_tree,
        gentufa_json,
        recovered,
    };
    let words = match attempt.result {
        Ok(words) => words,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.extend(morphology_error_diagnostic_expectation_items(
                &case.lojban,
                &error,
            ));
            return Ok(assemble(
                StrictObservation::BlockedByMorphology {
                    error: format!("{error:?}"),
                    diagnostics,
                },
                blocked(request.refs, true),
                blocked(request.gentufa_tree, true),
                blocked(request.gentufa_json, true),
                blocked(request.recovered, true),
            ));
        }
    };

    let strict = parse_syntax_tree_with_source_and_options(&words, &case.lojban, &options);
    let recovered = if request.recovered {
        let recovery_options = request.recovery_max_errors.map_or_else(
            || options.clone(),
            |limit| options.clone().with_max_recovery_errors(limit),
        );
        let recovered = parse_syntax_tree_recovered_with_source_and_options(
            &words,
            &case.lojban,
            &recovery_options,
        );
        let mut diagnostics = morphology_diagnostics.clone();
        diagnostics.extend(recovered_syntax_diagnostic_expectation_items(
            &case.lojban,
            &recovered,
        ));
        diagnostics.sort_by(|a, b| (a.byte_span, &a.code).cmp(&(b.byte_span, &b.code)));
        Surface::Success {
            value: RecoveredObservation {
                parser_status: if recovered.errors.is_empty() {
                    fixtures::ExpectationStatus::Success
                } else {
                    fixtures::ExpectationStatus::Failure
                },
                raw: format!("{:?}", recovered.parse_tree),
                errors: recovered
                    .errors
                    .iter()
                    .map(|error| format!("{error:?}"))
                    .collect(),
                diagnostics,
                tree: recovered_syntax_tree_expectation(&recovered),
                target: recovered_target_observation(
                    &recovered.parse_tree,
                    request.target.as_ref(),
                ),
            },
        }
    } else {
        Surface::Inapplicable
    };

    let parsed = match strict {
        Ok(parsed) => parsed,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.extend(syntax_error_diagnostic_expectation_items(
                &case.lojban,
                &error,
            ));
            return Ok(assemble(
                StrictObservation::Failure {
                    error: format!("{error:?}"),
                    diagnostics,
                },
                blocked(request.refs, false),
                blocked(request.gentufa_tree, false),
                blocked(request.gentufa_json, false),
                recovered,
            ));
        }
    };
    let mut diagnostics = morphology_diagnostics;
    diagnostics.extend(syntax_warning_diagnostic_expectation_items(
        &case.lojban,
        &parsed.warnings,
    ));
    let syntax = StrictObservation::Success {
        raw: format!("{:?}", parsed.parse_tree),
        target: target_observation(&parsed.parse_tree, request.target.as_ref()),
        diagnostics,
    };
    let refs = if request.refs {
        match analyze_generated_references(&parsed.parse_tree) {
            Ok(analysis) => Surface::Success {
                value: analysis.fixture_projection_json()?,
            },
            Err(error) => Surface::Failure {
                error: format!("semantic refs error: {error}"),
            },
        }
    } else {
        Surface::Inapplicable
    };
    let gentufa_tree = if request.gentufa_tree {
        Surface::Success {
            value: pretty_generated_model_tree_with_options(
                &parsed.parse_tree,
                &case.lojban,
                TreeRenderOptions {
                    color: false,
                    indent: 2,
                    show_spans: true,
                    ..TreeRenderOptions::default()
                },
            )?,
        }
    } else {
        Surface::Inapplicable
    };
    let gentufa_json = if request.gentufa_json {
        Surface::Success {
            value: compact_generated_model_json_string_with_options(
                &parsed.parse_tree,
                JsonRenderOptions {
                    indent: 0,
                    ..JsonRenderOptions::default()
                },
            )?,
        }
    } else {
        Surface::Inapplicable
    };
    Ok(assemble(
        syntax,
        refs,
        gentufa_tree,
        gentufa_json,
        recovered,
    ))
}

/// Emit one complete JSON observation per line, so long-form cases need not accumulate in RAM.
#[requires(true)]
#[ensures(true)]
pub(super) fn run(args: ObserveArgs) -> Result<()> {
    let selection: Vec<Request> = serde_json::from_slice(&std::fs::read(&args.selection)?)
        .context("reading links/JAI observation selection")?;
    anyhow::ensure!(!selection.is_empty(), "observation selection is empty");
    for request in selection {
        let observed = observe(args.stage, request)?;
        println!("{}", serde_json::to_string(&observed)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[invariant(anchor.byte_end <= *parent_end && *parent_end <= source.len())]
    struct MissingPayloadCase {
        source: &'static str,
        anchor: LinkAnchor,
        parent_end: usize,
    }

    #[requires(true)]
    #[ensures(true)]
    fn request(path: &str, target: Option<LinkAnchor>) -> Request {
        new!(Request {
            path: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace root")
                .join(path),
            target,
            refs: true,
            gentufa_tree: true,
            gentufa_json: true,
            recovered: true,
            recovery_max_errors: None,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn captures_full_and_existing_legacy_as_actual_successes() {
        for full in [false, true] {
            let path = if full {
                "tests/fixtures/adhoc/syntax/links-jai/be-na-ku.toml"
            } else {
                "tests/fixtures/adhoc/v0/warnings/standard-no-warning/standard-postposed-be-linkargs.toml"
            };
            let request = request(
                path,
                Some(new!(LinkAnchor {
                    marker: LinkMarker::Be,
                    byte_start: 9,
                    byte_end: 11,
                })),
            );
            let result = observe(Stage::CA, request).expect("capture succeeds");
            let StrictObservation::Success {
                raw,
                target,
                diagnostics,
            } = result.syntax
            else {
                panic!("strict parse failed: {path}");
            };
            assert!(!raw.is_empty());
            assert!(diagnostics.is_empty());
            let TargetObservation::Found { link } = target else {
                panic!("frozen BE was not found: {path}");
            };
            if full {
                assert!(matches!(link.owner, LinkOwner::FullLinkedTerm));
            } else {
                assert!(matches!(link.owner, LinkOwner::PlainLinkedSumti));
            }
            assert!(matches!(result.refs, Surface::Success { .. }));
            assert!(matches!(result.gentufa_tree, Surface::Success { .. }));
            assert!(matches!(result.gentufa_json, Surface::Success { .. }));
            assert!(matches!(result.recovered, Surface::Success { .. }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strict_failure_blocks_downstream_but_keeps_recovery_and_diagnostics() {
        let request = request(
            "tests/fixtures/adhoc/syntax/terms/nuhi-less-nuhu-rejected.toml",
            Some(new!(LinkAnchor {
                marker: LinkMarker::Be,
                byte_start: 0,
                byte_end: 2
            })),
        );
        let result = observe(Stage::CA, request).expect("a parser rejection is an observation");
        let StrictObservation::Failure { error, diagnostics } = result.syntax else {
            panic!("expected actual strict rejection");
        };
        assert!(!error.is_empty());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == jbotci_diagnostics::DiagnosticSeverity::Error
        }));
        assert!(matches!(result.refs, Surface::BlockedBySyntaxFailure));
        assert!(matches!(
            result.gentufa_tree,
            Surface::BlockedBySyntaxFailure
        ));
        assert!(matches!(
            result.gentufa_json,
            Surface::BlockedBySyntaxFailure
        ));
        let Surface::Success { value } = result.recovered else {
            panic!("recovery must still be observed");
        };
        assert!(!value.raw.is_empty());
        assert_eq!(value.parser_status, fixtures::ExpectationStatus::Failure);
        assert!(!value.errors.is_empty());
        // This rejected input can recover wholly as recovery items. Capturing that result is
        // distinct from the LR requirement that a particular Full/Prefix node actually win.
        assert!(!value.tree.recovery_items.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn missing_link_payloads_are_required_errors_not_empty_values() {
        for dialect in ["()", "(zantufa)", "(+zantufa-terms)"] {
            let definition = jbotci_dialect::parse_dialect_definition(dialect).unwrap();
            let options = ParseOptions::default().with_dialect_definition(&definition);
            for case in [
                new!(MissingPayloadCase {
                    source: "mi broda be",
                    anchor: new!(LinkAnchor {
                        marker: LinkMarker::Be,
                        byte_start: 9,
                        byte_end: 11
                    }),
                    parent_end: 11,
                }),
                new!(MissingPayloadCase {
                    source: "mi broda be be'o",
                    anchor: new!(LinkAnchor {
                        marker: LinkMarker::Be,
                        byte_start: 9,
                        byte_end: 11
                    }),
                    parent_end: 16,
                }),
                new!(MissingPayloadCase {
                    source: "mi broda be ko'a bei",
                    anchor: new!(LinkAnchor {
                        marker: LinkMarker::Bei,
                        byte_start: 17,
                        byte_end: 20
                    }),
                    parent_end: 20,
                }),
                new!(MissingPayloadCase {
                    source: "mi broda be be'o ko'a",
                    anchor: new!(LinkAnchor {
                        marker: LinkMarker::Be,
                        byte_start: 9,
                        byte_end: 11
                    }),
                    parent_end: 16,
                }),
                new!(MissingPayloadCase {
                    source: "mi broda be bei ko'a be'o",
                    anchor: new!(LinkAnchor {
                        marker: LinkMarker::Be,
                        byte_start: 9,
                        byte_end: 11
                    }),
                    parent_end: 25,
                }),
            ] {
                let source = case.source;
                let anchor = &case.anchor;
                let words = jbotci_morphology::segment_words_with_modifiers(source).unwrap();
                assert!(
                    parse_syntax_tree_with_source_and_options(&words, source, &options).is_err()
                );
                let parsed =
                    parse_syntax_tree_recovered_with_source_and_options(&words, source, &options);
                assert_eq!(parsed.errors.len(), 1, "{dialect}: {source}");
                assert!(parsed.warnings.is_empty(), "{dialect}: {source}");
                let TargetObservation::Found { link } =
                    recovered_target_observation(&parsed.parse_tree, Some(anchor))
                else {
                    panic!("required link field was lost: {dialect}: {source}");
                };
                assert!(
                    matches!(link.field, RecoveredLinkField::Error),
                    "{dialect}: {source}"
                );
                let extent = link
                    .parent_parsed_extent
                    .expect("the marker remains sourced");
                assert_eq!(
                    (extent.byte_start, extent.byte_end),
                    (anchor.byte_start, case.parent_end)
                );

                // A missing BEI payload must not discard the preceding complete BE payload.
                // E3 retains the following recovered text, but its failed required field and
                // diagnostics prevent any claim of a successful empty link/outer reassignment.
                if anchor.marker == LinkMarker::Bei {
                    let first = new!(LinkAnchor {
                        marker: LinkMarker::Be,
                        byte_start: 9,
                        byte_end: 11
                    });
                    let TargetObservation::Found { link } =
                        recovered_target_observation(&parsed.parse_tree, Some(&first))
                    else {
                        panic!("earlier BE payload was lost: {dialect}: {source}");
                    };
                    let RecoveredLinkField::Valid {
                        owner,
                        parsed_extent,
                    } = link.field
                    else {
                        panic!("earlier BE payload is no longer Valid: {dialect}: {source}");
                    };
                    assert_eq!(owner, LinkOwner::PlainLinkedSumti);
                    let extent = parsed_extent.expect("the preceding sumti is sourced");
                    assert_eq!((extent.byte_start, extent.byte_end), (12, 16));
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_target_reports_the_containing_field_and_only_parsed_extents() {
        for (source, expected_status, expected_extent) in [
            (
                "mi broda be na ku be'o .i mi ku .i do broda",
                "valid",
                Some((12, 17)),
            ),
            ("mi broda be na ku ku be'o", "prefix", Some((12, 17))),
            (
                "mi broda be ku ko'a be'o .i mi ku .i do broda",
                "error",
                None,
            ),
        ] {
            let words = jbotci_morphology::segment_words_with_modifiers(source).unwrap();
            let parsed = parse_syntax_tree_recovered_with_source_and_options(
                &words,
                source,
                &ParseOptions::default(),
            );
            assert!(!parsed.errors.is_empty());
            let anchor = new!(LinkAnchor {
                marker: LinkMarker::Be,
                byte_start: 9,
                byte_end: 11
            });
            let TargetObservation::Found { link } =
                recovered_target_observation(&parsed.parse_tree, Some(&anchor))
            else {
                panic!("missing BE in {source}");
            };
            assert_eq!(link.anchor, anchor);
            let (status, extent) = match link.field {
                RecoveredLinkField::Valid {
                    owner,
                    parsed_extent,
                } => {
                    assert_eq!(owner, LinkOwner::FullLinkedTerm);
                    ("valid", parsed_extent)
                }
                RecoveredLinkField::Prefix {
                    owner,
                    parsed_extent,
                } => {
                    assert_eq!(owner, LinkOwner::FullLinkedTerm);
                    ("prefix", parsed_extent)
                }
                RecoveredLinkField::Error => ("error", None),
            };
            assert_eq!(status, expected_status, "{source}");
            assert_eq!(
                extent.map(|span| (span.byte_start, span.byte_end)),
                expected_extent
            );
        }
    }
}
