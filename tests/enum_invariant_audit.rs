use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use walkdir::WalkDir;

const ALLOWED_PLACEHOLDERS: &[(&str, &str)] = &[
    (
        "crates/bityzba/tests/type_invariant.rs:Tree::Branch",
        "bityzba fixture intentionally exercises audited no-op enum arm syntax",
    ),
    (
        "crates/bityzba/tests/type_invariant.rs:PlainChoice::Named",
        "bityzba fixture covers plain enum arm marker behavior",
    ),
    (
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:DataChoice::Present",
        "contract scanner fixture must contain an accepted no-op marker",
    ),
    (
        "crates/bityzba/tests/ui/fail/enum_duplicate_variant_invariant.rs:Choice::Named",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/bityzba/tests/ui/fail/enum_tuple_variant_requires_pattern.rs:Choice::Pair",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/bityzba/tests/ui/fail/enum_unknown_variant_invariant.rs:Choice::Named",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/bityzba/tests/ui/fail/enum_unknown_variant_invariant.rs:Choice::Missing",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:SExpr::Leaf",
        "render tree leaf text is normalized by constructors and empty leaves collapse to nodes",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:SExpr::Node",
        "empty render nodes are meaningful intermediate values",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::Json",
        "error wrapper carries serde's diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-output/src/tree.rs:RenderEntry::Primary",
        "render entry delegates all validity to TreeValue",
    ),
    (
        "crates/jbotci-output/src/tree.rs:RenderEntry::Labelled",
        "labels are static visitor metadata and TreeValue owns payload validity",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Node",
        "render node payload owns constructor and entry shape",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Collection",
        "empty render collections are valid intermediate output",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Word",
        "word render fields are produced from validated morphology atoms",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Verbatim",
        "verbatim render text is source-derived and may be empty",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Text",
        "text render payload is a source-derived scalar with no extra enum-level rule",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Span",
        "span field ordering is preserved from SourceSpan before rendering",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Node",
        "builder stack frame validity is governed by enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Field",
        "field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Collection",
        "collection frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyFrame::Node",
        "builder stack frame validity is governed by enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyFrame::Field",
        "field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/surface.rs:SurfaceChunk::Word",
        "surface chunks are intermediate render fragments filtered before output",
    ),
    (
        "crates/jbotci-output/src/surface.rs:SurfaceChunk::QuotedWords",
        "quoted word chunks may be empty for source-derived quote bodies",
    ),
    (
        "crates/jbotci-output/src/surface.rs:SurfaceChunk::QuotedText",
        "quoted text chunks preserve source text without an enum-level rule",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Node",
        "JSON builder frame validity is governed by traversal sequencing",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Field",
        "JSON field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Sequence",
        "JSON sequence frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-source/src/lib.rs:SourceLocationError::ByteRangeInverted",
        "diagnostic enum records rejected constructor inputs",
    ),
    (
        "crates/jbotci-source/src/lib.rs:SourceLocationError::CharRangeInverted",
        "diagnostic enum records rejected constructor inputs",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:WrappedNode::Tuple",
        "tree macro test wrapper delegates validity to the wrapped payload",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:WrappedNode::Named",
        "tree macro test wrapper has no marker-specific payload rule",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryValidationError::InvalidEntry",
        "validation error wrapper carries path and entry diagnostics",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:RafsiField::Text",
        "raw import field accepts the upstream Lensisku scalar shape before normalization",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:RafsiField::List",
        "raw import field accepts the upstream Lensisku list shape before normalization",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:LensiskuImportError::Json",
        "error wrapper carries serde's diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-search/src/lib.rs:SearchError::DimensionMismatch",
        "diagnostic enum records vector-search implementation errors",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectDefinitionEntry::Cmavo",
        "entry payload is validated by CmavoDialectEntry",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectDefinitionEntry::Feature",
        "feature payload is closed over DialectFeature and toggle enums",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectToken::Atom",
        "tokenizer emits atoms from non-empty spans before parser validation",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SAMatchTag::Selmaho",
        "selmaho strings come from the static morphology table",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::Unsupported",
        "diagnostic enum records rejected parser inputs",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::Invalid",
        "diagnostic enum records rejected parser inputs",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::UnterminatedZoiQuote",
        "diagnostic enum records rejected quote input",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::SourceSpan",
        "error wrapper delegates validity to SourceLocationError",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:Jvopau::Rafsi",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:Jvopau::Hyphen",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithIndicators::Bare",
        "generic wrapper delegates word validity to the payload type",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithIndicators::Emphasized",
        "constructor contracts enforce BAhE while generic payload owns word validity",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithIndicators::WithIndicator",
        "constructor contracts enforce UI/CAI/Y and NAI marker shape",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:Statement::Fragment",
        "legacy facade enum delegates validity to Fragment",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:Fragment::Other",
        "legacy facade fragment preserves parser words without extra structure",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:FreeModifier::Words",
        "legacy facade modifier preserves parser words without extra structure",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:Connective::Words",
        "legacy facade connective preserves parser words without extra structure",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxError::Parse",
        "diagnostic enum records parser error location and message",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:PredicateTail3Syntax::GekSentence",
        "variant delegates all grammar markers to GekSentenceSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SubsentenceSyntax::Plain",
        "plain subsentence is exactly a PredicateSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:StatementSyntax::Predicate",
        "variant delegates all grammar markers to PredicateSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:StatementSyntax::ExperimentalPredicateContinuation",
        "variant combines two validated syntax payloads without its own marker",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:StatementSyntax::Fragment",
        "variant delegates all grammar markers to FragmentSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::Ek",
        "fragment is exactly a validated afterthought connective",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::Gihek",
        "fragment is exactly a validated predicate-tail connective",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::MathExpression",
        "fragment delegates all grammar markers to MathExpressionSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::Relation",
        "fragment delegates all grammar markers to RelationSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:TermSyntax::Argument",
        "term is exactly a validated ArgumentSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentTagSyntax::TenseModal",
        "tag delegates all grammar markers to TenseModalSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentSyntax::Quote",
        "argument delegates all grammar markers to QuoteSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentSyntax::Quantified",
        "variant combines validated quantifier and argument payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentSyntax::Tagged",
        "variant combines a validated tag and argument payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentSyntax::Connected",
        "variant combines validated argument payloads through a validated connective",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentSyntax::RelationVocative",
        "vocative relation has no required relative-clause marker beyond RelationSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelativeClauseSyntax::Connected",
        "variant combines a validated connective and relative-clause payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentTailElementSyntax::Argument",
        "tail element is exactly a validated ArgumentSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentTailElementSyntax::Quantifier",
        "tail element is exactly a validated QuantifierSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MathExpressionSyntax::Number",
        "math expression delegates all marker checks to QuantifierSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MathExpressionSyntax::Gek",
        "forethought math expression uses validated connective and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MathExpressionSyntax::Connected",
        "connected math expression uses validated connective and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MathExpressionSyntax::Binary",
        "binary math expression uses a validated operator and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MathOperatorSyntax::Connected",
        "connected math operator uses validated connective and operator payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelationSyntax::Connected",
        "connected relation uses validated connective and relation payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelationSyntax::TenseModal",
        "relation prefix delegates marker checks to TenseModalSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelationSyntax::Compound",
        "compound relation non-emptiness is enforced by RelationUnitVec",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelationUnitSyntax::Connected",
        "connected relation unit uses validated connective and unit payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelationUnitSyntax::Wrapped",
        "wrapped relation unit is exactly a validated RelationSyntax payload",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Vlasei",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Gentufa",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Mulgau",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Tersmu",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Vlacku",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Jvozba",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Cukta",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/main.rs:Command::Zbasu",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "tests/support/fixtures/mod.rs:Provenance::Cll",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "tests/support/fixtures/mod.rs:Provenance::Muplis",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "tests/support/fixtures/mod.rs:Provenance::Corpus",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "tests/support/fixtures/mod.rs:Provenance::Adhoc",
        "ad hoc provenance intentionally permits absent description",
    ),
    (
        "tests/support/fixtures/mod.rs:Provenance::Other",
        "fixture tree validation checks custom provenance names at import time",
    ),
    (
        "tests/support/fixtures/mod.rs:AllowedNextExpectation::Negative",
        "negative expectation delegates validity to the nested expectation",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::Read",
        "fixture error wrapper carries filesystem diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::Write",
        "fixture error wrapper carries filesystem diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::ParseToml",
        "fixture error wrapper carries TOML parser diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::EncodeToml",
        "fixture error wrapper carries TOML encoder diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::ParseJson",
        "fixture error wrapper carries JSON parser diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::Walk",
        "fixture error wrapper carries directory traversal diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::DuplicateId",
        "fixture error wrapper carries duplicate-id diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::UnknownFacet",
        "fixture error wrapper carries facet-name diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::InvalidDialect",
        "fixture error wrapper carries dialect diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::InvalidXfail",
        "fixture error wrapper carries xfail diagnostics",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureError::LegacyExpectationFormat",
        "fixture error wrapper carries legacy-format diagnostics",
    ),
    (
        "xtask/src/main.rs:Command::Fmt",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureCheck",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureImport",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureList",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureRewrite",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureVectorStats",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::FixtureTest",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::VendorDictionary",
        "xtask command enum delegates validation to clap and option structs",
    ),
];

#[test]
#[requires(true)]
#[ensures(true)]
fn enum_placeholder_invariant_audit_is_current() {
    let found = enum_placeholder_invariants();
    let allowed = allowed_placeholder_keys();

    let unexpected = found.difference(&allowed).cloned().collect::<Vec<_>>();
    let stale = allowed.difference(&found).cloned().collect::<Vec<_>>();

    assert!(
        unexpected.is_empty() && stale.is_empty(),
        "unexpected enum placeholder invariants:\n{}\n\nstale allowlist entries:\n{}",
        unexpected.join("\n"),
        stale.join("\n"),
    );
}

#[requires(true)]
#[ensures(true)]
fn allowed_placeholder_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (key, reason) in ALLOWED_PLACEHOLDERS {
        assert!(
            !key.is_empty(),
            "placeholder allowlist key must not be empty"
        );
        assert!(
            !reason.is_empty(),
            "placeholder allowlist reason must not be empty"
        );
        assert!(
            keys.insert((*key).to_owned()),
            "duplicate placeholder allowlist key: {key}",
        );
    }
    keys
}

#[requires(true)]
#[ensures(true)]
fn enum_placeholder_invariants() -> BTreeSet<String> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut placeholders = BTreeSet::new();
    for root in ["crates", "apps", "tests", "xtask"] {
        let source_root = workspace.join(root);
        if source_root.exists() {
            collect_enum_placeholder_invariants(workspace, &source_root, &mut placeholders);
        }
    }
    placeholders
}

#[requires(source_root.exists())]
#[ensures(true)]
fn collect_enum_placeholder_invariants(
    workspace: &Path,
    source_root: &Path,
    placeholders: &mut BTreeSet<String>,
) {
    for entry in WalkDir::new(source_root) {
        let entry = entry.expect("source walk entry should be readable");
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(workspace)
            .expect("walked path should be under workspace");
        let source = fs::read_to_string(entry.path()).expect("Rust source should be readable");
        scan_rust_source(relative_path, &source, placeholders);
    }
}

#[requires(true)]
#[ensures(true)]
fn scan_rust_source(relative_path: &Path, source: &str, placeholders: &mut BTreeSet<String>) {
    let mut pending = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(variant) = placeholder_variant(line) {
            pending.push(variant.to_owned());
            continue;
        }
        if let Some(enum_name) = enum_name(line) {
            for variant in pending.drain(..) {
                placeholders.insert(format!(
                    "{}:{enum_name}::{variant}",
                    relative_path.display()
                ));
            }
            continue;
        }
        if !pending.is_empty()
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with("//")
        {
            pending.clear();
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn placeholder_variant(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("#[invariant(::")?;
    if !rest.contains("=> true)]") {
        return None;
    }
    let end = rest
        .char_indices()
        .find(|(_, ch)| !(*ch == '_' || ch.is_ascii_alphanumeric()))
        .map_or(rest.len(), |(index, _)| index);
    Some(&rest[..end])
}

#[requires(true)]
#[ensures(true)]
fn enum_name(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    while let Some(word) = words.next() {
        if word == "enum" {
            let name = words.next()?;
            let end = name
                .char_indices()
                .find(|(_, ch)| !(*ch == '_' || ch.is_ascii_alphanumeric()))
                .map_or(name.len(), |(index, _)| index);
            return Some(&name[..end]);
        }
    }
    None
}
