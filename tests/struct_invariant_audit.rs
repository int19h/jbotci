use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use walkdir::WalkDir;

const ALLOWED_PLACEHOLDERS: &[(&str, &str)] = &[
    (
        "apps/jbotci-server/src/main.rs:Cli",
        "CLI root delegates input validation to clap",
    ),
    (
        "apps/jbotci/src/main.rs:Cli",
        "CLI root delegates input validation to clap",
    ),
    (
        "apps/jbotci/src/main.rs:GentufaInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:JvozbaInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:SearchInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:TextInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:VlaseiInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:ImplType",
        "contract scanner fixture intentionally contains accepted no-op markers",
    ),
    (
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:Marker",
        "contract scanner fixture intentionally contains accepted no-op markers",
    ),
    (
        "crates/bityzba/tests/type_invariant.rs:PlainMarker",
        "bityzba fixture covers explicit no-op type markers",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:BadMapper",
        "test-only mapper carries no state beyond call counters",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:BuiltinDialect",
        "builtin dialect table is static data validated by dialect-definition tests",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectError",
        "diagnostic struct carries a human-readable error message",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:ImportedDictionary",
        "raw Lensisku import shape is validated at parse and fixture-import boundaries",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:ImportedDictionaryEntry",
        "raw Lensisku entry shape is normalized before becoming dictionary model data",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:ImportedDictionaryUser",
        "raw Lensisku user metadata preserves upstream scalar shape",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:ImportedKeyword",
        "raw Lensisku keyword metadata preserves upstream scalar shape",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:Dictionary",
        "dictionary-wide validity is checked by validate and the expensive impl invariant",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DefinitionId",
        "Lensisku definition ids are opaque upstream identifiers",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryEntry",
        "dictionary entry field consistency is checked by Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryUser",
        "dictionary user metadata preserves upstream scalar shape",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:EntryIndex",
        "entry index bounds are slice-relative and checked at lookup use sites",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:Keyword",
        "keyword text is upstream dictionary data normalized by import generation",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:OwnedDictionaryIndexes",
        "owned index aggregate is produced by build_owned_indexes",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:OwnedRafsiIndexEntry",
        "owned index entry is produced from non-empty BTreeMap buckets",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:OwnedSelmahoIndexEntry",
        "owned index entry is produced from non-empty BTreeMap buckets",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:OwnedWordIndexEntry",
        "owned index entry is produced from non-empty BTreeMap buckets",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:Rafsi",
        "rafsi text is upstream dictionary data normalized by import generation",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiIndexEntry",
        "borrowed index entry is generated from owned validated buckets",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiIndexTarget",
        "target combines an index with a closed rafsi provenance enum",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiMatch",
        "lookup match delegates validity to the borrowed dictionary entry",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RawSelmaho",
        "selmaho text is upstream dictionary data normalized by import generation",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:Score",
        "Lensisku score is an opaque upstream ranking value",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:SelmahoIndexEntry",
        "borrowed index entry is generated from owned validated buckets",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:WordIndexEntry",
        "borrowed index entry is generated from owned validated buckets",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:CmavoPrefix",
        "cmavo prefix is an internal parser cursor over table-derived text",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:Segmenter",
        "segmenter is mutable parser state whose invariants are algorithm-local",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SourceChar",
        "source character pairs one char with its byte position",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:PhonemeRenderOptions",
        "render options are independent booleans with no cross-field invariant",
    ),
    (
        "crates/jbotci-output/src/brackets.rs:BracketContext",
        "render context borrows source text and options without extra state rules",
    ),
    (
        "crates/jbotci-output/src/brackets.rs:SourceWordBracketVisitor",
        "visitor holds traversal-local rendering state",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonEntry",
        "JSON entry mirrors traversal metadata and may contain empty values",
    ),
    (
        "crates/jbotci-output/src/json.rs:MorphologyJsonBuilder",
        "builder validity is governed by traversal enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/json.rs:MorphologyNodeInfo",
        "node info is derived from static morphology tree metadata",
    ),
    (
        "crates/jbotci-output/src/json.rs:SyntaxJsonBuilder",
        "builder validity is governed by traversal enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/json.rs:SyntaxNodeInfo",
        "node info is derived from static syntax tree metadata",
    ),
    (
        "crates/jbotci-output/src/lib.rs:BracketRenderOptions",
        "render options are independent flags with no cross-field invariant",
    ),
    (
        "crates/jbotci-output/src/lib.rs:JsonRenderOptions",
        "JSON indentation accepts any width chosen by callers",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputFormat",
        "output features are interpreted by the renderer for the selected base",
    ),
    (
        "crates/jbotci-output/src/lib.rs:TreeRenderOptions",
        "render options are independent flags with no cross-field invariant",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyTreeBuilder",
        "builder validity is governed by traversal enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxTreeBuilder",
        "builder validity is governed by traversal enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeEntry",
        "tree entry delegates label and value meaning to traversal metadata",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeNode",
        "tree node labels come from static traversal metadata",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeRenderer",
        "renderer owns options only",
    ),
    (
        "crates/jbotci-search/src/lib.rs:SearchHit",
        "search score semantics are index-specific",
    ),
    (
        "crates/jbotci-semantics/src/lib.rs:ScopedModifier",
        "semantic model is a placeholder port scaffold with no derived grammar contract yet",
    ),
    (
        "crates/jbotci-semantics/src/lib.rs:SemanticParagraph",
        "semantic model is a placeholder port scaffold with no derived grammar contract yet",
    ),
    (
        "crates/jbotci-semantics/src/lib.rs:SemanticStatement",
        "semantic model is a placeholder port scaffold with no derived grammar contract yet",
    ),
    (
        "crates/jbotci-semantics/src/lib.rs:SemanticText",
        "semantic model is a placeholder port scaffold with no derived grammar contract yet",
    ),
    (
        "crates/jbotci-source/src/lib.rs:SourceId",
        "source ids are opaque caller-provided labels",
    ),
    (
        "crates/jbotci-source/src/lib.rs:Spanned",
        "span and value each own their validity",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParsedStatement",
        "parser result aggregate combines validated text and collected warnings",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserState",
        "parser state is mutable chumsky inspector state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:LeadingIStatementSyntax",
        "private parser staging node is consumed into validated paragraph nodes",
    ),
    (
        "crates/jbotci-syntax/src/grammar/tense.rs:CompositeTenseModalClassification",
        "mutable classification state is projected into validated tense structs",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:LojbanText",
        "legacy facade preserves parsed text structure from validated TextSyntax",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:Paragraph",
        "legacy facade preserves parsed paragraph structure from validated ParagraphSyntax",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:ParagraphStatement",
        "legacy facade preserves parsed statement structure from validated ParagraphStatementSyntax",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:ParseOptions",
        "parse options are independent caller-selected controls",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SourceSpanVisitor",
        "visitor wraps a callback without adding semantic state",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:TraceOptions",
        "trace options are independent caller-selected controls",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:ArgumentConnectionSyntax",
        "argument connection delegates marker validity to ConnectiveSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:PredicateStatementContinuationSyntax",
        "continuation marker enum owns the BO/KE marker checks",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:PredicateTail1Syntax",
        "predicate-tail aggregate delegates marker validity to continuation nodes",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:PredicateTail2Syntax",
        "predicate-tail aggregate delegates marker validity to BO continuation nodes",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:PredicateTailSyntax",
        "predicate-tail aggregate delegates marker validity to child nodes",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithFreeModifiers",
        "generic wrapper delegates validity to its payload and FreeModifierSyntax",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:FieldRef",
        "tree field metadata is generated from static model definitions",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:LeafNode",
        "tree macro test fixture intentionally has no extra field invariant",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:NodeKindVisitor",
        "tree macro test visitor stores collected labels",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:PairNode",
        "tree macro test fixture intentionally has no extra field invariant",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecordingVisitor",
        "tree macro test visitor stores traversal events",
    ),
    (
        "tests/fixture_suite.rs:FakeBackend",
        "fixture test backend stores scripted outputs and captured invocations",
    ),
    (
        "tests/support/fixtures/mod.rs:CllSelector",
        "fixture selector validity is checked by fixture profile loading",
    ),
    (
        "tests/support/fixtures/mod.rs:CommandOutputExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:Expectations",
        "fixture expectation aggregate permits absent facets",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureExport",
        "fixture export is a serialization aggregate",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureProfile",
        "fixture profile validity is checked while loading and selecting tests",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureSelector",
        "fixture selector validity is checked by selector matching code",
    ),
    (
        "tests/support/fixtures/mod.rs:FixtureSummary",
        "fixture summary is derived reporting data",
    ),
    (
        "tests/support/fixtures/mod.rs:ImportSummary",
        "fixture import summary is derived reporting data",
    ),
    (
        "tests/support/fixtures/mod.rs:LoadedTestCase",
        "loaded fixture combines a test case with its source path",
    ),
    (
        "tests/support/fixtures/mod.rs:MorphologyExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:MuplisSelector",
        "fixture selector validity is checked by fixture profile loading",
    ),
    (
        "tests/support/fixtures/mod.rs:OutputExpectations",
        "fixture expectation aggregate permits absent output formats",
    ),
    (
        "tests/support/fixtures/mod.rs:ParseErrorExpectation",
        "fixture parse-error expectation permits optional diagnostic fragments",
    ),
    (
        "tests/support/fixtures/mod.rs:StructuredExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:SyntaxExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:TestCase",
        "fixture loading validates ids, facets, and expectation shape",
    ),
    (
        "tests/support/fixtures/mod.rs:TextExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:XfailExpectation",
        "fixture xfail reason validation is handled by fixture loading",
    ),
    (
        "tests/support/fixtures/runner.rs:FacetResult",
        "runner result combines facet status with diagnostic messages",
    ),
    (
        "tests/support/fixtures/runner.rs:RunSummary",
        "runner summary is derived reporting data",
    ),
    (
        "xtask/src/main.rs:CachedExport",
        "xtask cache entry mirrors fixture export metadata",
    ),
    (
        "xtask/src/main.rs:Cli",
        "xtask CLI root delegates input validation to clap",
    ),
    (
        "xtask/src/main.rs:DebugMatchWriter",
        "debug writer carries expected lines and write position state",
    ),
    (
        "xtask/src/main.rs:DebugPrefixWriter",
        "debug writer carries indentation state",
    ),
    (
        "xtask/src/main.rs:DictionaryMetadata",
        "dictionary metadata borrows Cargo-provided output paths",
    ),
    (
        "xtask/src/main.rs:FieldLengths",
        "field length counters are derived statistics",
    ),
    (
        "xtask/src/main.rs:FixtureImportArgs",
        "xtask command args delegate validation to clap and command code",
    ),
    (
        "xtask/src/main.rs:FixtureRewriteArgs",
        "xtask command args delegate validation to clap and command code",
    ),
    (
        "xtask/src/main.rs:FixtureRunArgs",
        "xtask command args delegate validation to clap and command code",
    ),
    (
        "xtask/src/main.rs:FixtureVectorStatsArgs",
        "xtask command args delegate validation to clap and command code",
    ),
    (
        "xtask/src/main.rs:LengthSummary",
        "length summary is derived reporting data",
    ),
    (
        "xtask/src/main.rs:NotImplementedBackend",
        "placeholder backend carries no state",
    ),
    (
        "xtask/src/main.rs:RewriteSummary",
        "rewrite summary is derived reporting data",
    ),
    (
        "xtask/src/main.rs:VectorStats",
        "vector stats are derived reporting data",
    ),
    (
        "xtask/src/main.rs:VendorDictionaryArgs",
        "xtask command args delegate validation to clap and command code",
    ),
];

#[test]
#[requires(true)]
#[ensures(true)]
fn struct_placeholder_invariant_audit_is_current() {
    let found = struct_placeholder_invariants();
    let allowed = allowed_placeholder_keys();

    let unexpected = found.difference(&allowed).cloned().collect::<Vec<_>>();
    let stale = allowed.difference(&found).cloned().collect::<Vec<_>>();

    assert!(
        unexpected.is_empty() && stale.is_empty(),
        "unexpected struct placeholder invariants:\n{}\n\nstale allowlist entries:\n{}",
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
fn struct_placeholder_invariants() -> BTreeSet<String> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut placeholders = BTreeSet::new();
    for root in ["crates", "apps", "tests", "xtask"] {
        let source_root = workspace.join(root);
        if source_root.exists() {
            collect_struct_placeholder_invariants(workspace, &source_root, &mut placeholders);
        }
    }
    placeholders
}

#[requires(source_root.exists())]
#[ensures(true)]
fn collect_struct_placeholder_invariants(
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
    let lines = source.lines().collect::<Vec<_>>();
    let mut pending_placeholder = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if let Some((is_placeholder, next_index)) = invariant_attribute(&lines, index) {
            pending_placeholder |= is_placeholder;
            index = next_index + 1;
            continue;
        }
        if let Some(struct_name) = struct_name(line) {
            if pending_placeholder {
                placeholders.insert(format!("{}:{struct_name}", relative_path.display()));
            }
            pending_placeholder = false;
            index += 1;
            continue;
        }
        if pending_placeholder
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with("//")
        {
            pending_placeholder = false;
        }
        index += 1;
    }
}

#[requires(index < lines.len())]
#[ensures(true)]
fn invariant_attribute(lines: &[&str], index: usize) -> Option<(bool, usize)> {
    let line = lines[index].trim();
    if !line.starts_with("#[invariant(") {
        return None;
    }

    let mut attribute = String::from(line);
    let mut end = index;
    while !attribute.contains(")]") && end + 1 < lines.len() {
        end += 1;
        attribute.push_str(lines[end].trim());
    }

    let Some(inner) = attribute.strip_prefix("#[invariant(") else {
        return Some((false, end));
    };
    let inner = inner.strip_suffix(")]").unwrap_or(inner).trim();
    Some((inner == "true" || inner.starts_with("true,"), end))
}

#[requires(true)]
#[ensures(true)]
fn struct_name(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    while let Some(word) = words.next() {
        if word == "struct" {
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
