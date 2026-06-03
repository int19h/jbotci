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
        "crates/jbotci-output/src/lib.rs:BracketSourceFragment::Text",
        "bracket source fragments preserve renderer output, including empty intermediate text",
    ),
    (
        "crates/jbotci-output/src/lib.rs:BracketSourceFragment::Span",
        "bracket source spans preserve renderer grouping, including empty intermediate spans",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::Json",
        "error wrapper carries serde's diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::Diagnostic",
        "error wrapper carries Ariadne renderer diagnostics",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::Ipa",
        "error wrapper carries pronunciation renderer diagnostics",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::References",
        "error wrapper carries reference analysis diagnostics",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Environment",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Io",
        "embedding error variant wraps std::io::Error with contextual text",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Json",
        "embedding error variant wraps serde_json::Error with contextual text",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Http",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::InvalidModel",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::InvalidIndex",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::UnsupportedModel",
        "embedding error variant carries the unsupported model key for rendering",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::MissingCompatiblePack",
        "embedding error variant carries the requested model key for rendering",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::DimensionMismatch",
        "embedding error variant carries expected and actual dimensions produced by validation paths",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Backend",
        "embedding error variant carries only an already formatted backend diagnostic message",
    ),
    (
        "xtask/src/main.rs:Command::ExportWebEmbeddingCorpus",
        "xtask command variant delegates validation to clap and the typed argument struct",
    ),
    (
        "xtask/src/main.rs:Command::BuildWebEmbeddings",
        "xtask command variant delegates validation to clap and the typed argument struct",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:RawLujvoSegment::Rafsi",
        "internal fallback lujvo parser validates segment text before converting to Phonemes",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:RawLujvoSegment::Hyphen",
        "internal fallback lujvo parser only emits known hyphen segments before Phonemes validation",
    ),
    (
        "crates/jbotci-morphology/src/lujvo.rs:LujvoBuildMode::Lujvo",
        "composition mode is a closed selector enum kept direct for low-level hot-path matching",
    ),
    (
        "crates/jbotci-morphology/src/lujvo.rs:LujvoBuildMode::Cmevla",
        "composition mode is a closed selector enum kept direct for low-level hot-path matching",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaMode::Lujvo",
        "public build mode is a closed selector enum serialized directly for CLI and web callers",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaMode::Cmevla",
        "public build mode is a closed selector enum serialized directly for CLI and web callers",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaInput::Word",
        "public jvozba input enum is kept direct; parsing and build paths normalize and validate payloads before use",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaInput::FixedRafsi",
        "public jvozba input enum is kept direct; parsing and build paths normalize and validate payloads before use",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaSegmentKind::Rafsi",
        "segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaSegmentKind::Hyphen",
        "segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::RequiresAtLeastTwoInputs",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::FixedRafsiEmpty",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NonFinalUniversalLongRafsi",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::FinalConsonant",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NoRafsiAvailable",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NoDictionaryEntry",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::CouldNotBuildLujvo",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::CouldNotBuildCompound",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Word",
        "web search mode is a closed UI selector serialized directly in URLs and local state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Rafsi",
        "web search mode is a closed UI selector serialized directly in URLs and local state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Sound",
        "web search mode is a closed UI selector serialized directly in URLs and local state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Meaning",
        "web search mode is a closed UI selector serialized directly in URLs and local state",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::BlockQuote",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::CmavoList",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Code",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Definition",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::DisplayMath",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Ebnf",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Example",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::GrammarTemplate",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Heading",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::InterlinearGloss",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::List",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Lojbanization",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::LujvoMaking",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Media",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Paragraph",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Rule",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::SimpleListTable",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::Table",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllBlock::VariableList",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::ElidableTerminator",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::Hash",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::Nonterminal",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::Operator",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::Terminal",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllEbnfToken::Text",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllError::Load",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllError::NotFound",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllError::Parse",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Anchor",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::CiteTitle",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Code",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Elidable",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Emphasis",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::InlineMath",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::LanguageSpan",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Link",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Quote",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Subscript",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Superscript",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllInline::Text",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CuktaRequest::Example",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CuktaRequest::Search",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CuktaRequest::Section",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaPageKind::Error",
        "web Cukta page variants are presentation states produced by build_cukta_web_page",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaPageKind::Index",
        "web Cukta page variants are presentation states produced by build_cukta_web_page",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaPageKind::Search",
        "web Cukta page variants are presentation states produced by build_cukta_web_page",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaPageKind::Section",
        "web Cukta page variants are presentation states produced by build_cukta_web_page",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebMode::Meaning",
        "web Cukta search mode is a closed URL/UI selector with disabled semantic mode preserved",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebMode::Word",
        "web Cukta search mode is a closed URL/UI selector with disabled semantic mode preserved",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebView::Index",
        "web Cukta view is a closed route selector parsed from the current client URL",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebView::Search",
        "web Cukta view is a closed route selector parsed from the current client URL",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebView::Section",
        "web Cukta view is a closed route selector parsed from the current client URL",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuVoteDisplay::Hidden",
        "vote display variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuVoteDisplay::Known",
        "vote display label is produced by formatting dictionary vote metadata before rendering",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuVoteDisplay::Unknown",
        "vote display variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPieceKind::Rafsi",
        "composition piece kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPieceKind::Hyphen",
        "composition piece kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Cmavo",
        "word type section is a closed grouping selector derived from dictionary metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Cmevla",
        "word type section is a closed grouping selector derived from dictionary metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Brivla",
        "word type section is a closed grouping selector derived from dictionary metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Other",
        "word type section is a closed grouping selector derived from dictionary metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaMode::Lujvo",
        "web jvozba mode is a closed UI selector serialized directly in local storage",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaMode::Cmevla",
        "web jvozba mode is a closed UI selector serialized directly in local storage",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaItemKind::Word",
        "web jvozba item kind is a closed UI selector whose value is stored on the surrounding item",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaItemKind::FixedRafsi",
        "web jvozba item kind is a closed UI selector whose value is stored on the surrounding item",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::Empty",
        "web jvozba output state carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::NeedsMore",
        "web jvozba output state carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::Success",
        "web jvozba success payload delegates validity to the shared jvozba builder output",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::Error",
        "web jvozba error payload carries the shared builder diagnostic text",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentKind::Rafsi",
        "web jvozba segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentKind::Hyphen",
        "web jvozba segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentTone::RafsiA",
        "web jvozba segment tone is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentTone::RafsiB",
        "web jvozba segment tone is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentTone::Hyphen",
        "web jvozba segment tone is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:PhoneticError::Message",
        "error wrapper carries renderer or tokenizer diagnostics without additional semantic state",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Syllabic",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Place",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Manner",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Voice",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Nasal",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Retroflex",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Lateral",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Aspirated",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::High",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Back",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Round",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeature::Long",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Valsi",
        "CLI and search validation reject empty valsi requests before lookup execution",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Rafsi",
        "CLI and search validation reject empty rafsi requests before lookup execution",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Lujvo",
        "CLI and search validation reject empty lujvo requests before lookup execution",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Glob",
        "glob compilation validates request text and reports invalid patterns as lookup diagnostics",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Sound",
        "sound query parsing validates request text before ALINE matching",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuRequest::Meaning",
        "semantic query text is validated by CLI and embedded locally before vector search",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionKind::Rafsi",
        "composition kind is a closed display tag; surface/source fields carry data validity",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionKind::Hyphen",
        "composition kind is a closed display tag; surface/source fields carry data validity",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Literal",
        "glob compiler normalizes literal tokens before constructing this internal matcher enum",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Consonant",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Vowel",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::AnyOne",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::AnyMany",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Forward",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Conversion",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Jai",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::ConnectiveBranches",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Compound",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Co",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceSlot::Numbered",
        "fixture place slots are serialization projections of PlaceSlot values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceSlot::Modal",
        "fixture place slots are serialization projections of PlaceSlot values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::ResolvedNode",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::ResolvedFrame",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::AmbiguousNodes",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::Unresolved",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::Vague",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::CharOffsetOutOfBounds",
        "diagnostic enum records rejected source offsets",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::ByteOffsetOutOfBounds",
        "diagnostic enum records rejected source offsets",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::ByteOffsetNotCharBoundary",
        "diagnostic enum records rejected UTF-8 boundary inputs",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::SourceLocation",
        "error wrapper delegates validity to SourceLocationError",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:TraceOptionError::InvalidLevel",
        "diagnostic enum records rejected trace levels",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:TraceRecorder::Active",
        "active recorder state owns trace invariants while the enum only selects enabled storage",
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
        "crates/jbotci-output/src/tree.rs:TreeValue::Syntax",
        "syntax provenance wrapper delegates rendered value validity to its payload",
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
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Numbered",
        "reference display slot is projected from validated semantic PlaceSlot values",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Modal",
        "reference display slot words are renderer projections of validated syntax",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Fai",
        "unit slot marker carries no payload beyond the selected variant",
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
        "apps/jbotci/src/main.rs:Command::Gerna",
        "cfg-gated nightly command delegates payload validity to GernaInput; stable builds cannot reference the variant in a stronger invariant",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceSlot::Numbered",
        "NonZeroU8 owns the non-zero numbered place invariant",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceSlot::Modal",
        "modal slot payload is an optional syntax node anchor and any option state is meaningful",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Forward",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Conversion",
        "NonZeroU8 owns converted-place non-zero validity and frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Jai",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::ConnectiveBranches",
        "connective-branch propagation may be temporarily empty for partially analyzed or unresolved selbri structures",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Compound",
        "frame ids are validated through PlaceAnalysis lookup APIs and empty modifier lists are valid",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Co",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::ResolvedNode",
        "node ids are validated through SyntaxIndex lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::ResolvedFrame",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::AmbiguousNodes",
        "an empty ambiguity set is valid while callers preserve an explicit unresolved state separately",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::Unresolved",
        "unresolved diagnostic text is produced by constructors in this module and has no enum-level structural invariant",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::Vague",
        "vagueness kind owns the payload validity",
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
        "crates/jbotci-output/src/surface.rs:IpaSurfaceChunk::Word",
        "IPA chunks borrow validated morphology words",
    ),
    (
        "crates/jbotci-output/src/surface.rs:IpaSurfaceChunk::Text",
        "IPA text chunks preserve source-derived quote text and may be empty before filtering",
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
        "crates/jbotci-tree/src/lib.rs:TreePathStep::SequenceIndex",
        "tree path sequence indices accept every usize value",
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
        "crates/jbotci-dialect/src/lib.rs:DialectFormulaComponent::Atom",
        "formula normalization drops empty atoms before rendering and this private parser state is not constructed outside dialect helpers",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectFormulaComponent::Group",
        "grouped formula text is produced by the local parenthesis collector and normalized before rendering",
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
        "crates/jbotci-morphology/src/tree.rs:LujvoPart::Rafsi",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:LujvoPart::Hyphen",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:WordLike::PlainWord",
        "bare word-like values delegate all validity to the wrapped Word",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithIndicators::Plain",
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
        "crates/jbotci-syntax/src/lib.rs:SyntaxError::Parse",
        "diagnostic enum records parser error location and message",
    ),
    (
        "crates/jbotci-semantics/src/lib.rs:SemanticsError::NotImplemented",
        "semantic builder placeholder has no payload beyond the diagnostic variant",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SimpleBridiTailSyntax::ForethoughtBridiTailConnection",
        "variant delegates all grammar markers to ForethoughtBridiConnectionSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SubbridiSyntax::Bridi",
        "plain subbridi is exactly a BridiSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:StatementSyntax::Bridi",
        "variant delegates all grammar markers to BridiSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:StatementSyntax::ExperimentalBridiContinuation",
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
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::BridiTailConnective",
        "fragment is exactly a validated predicate-tail connective",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::Mekso",
        "fragment delegates all grammar markers to MeksoSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:FragmentSyntax::Selbri",
        "fragment delegates all grammar markers to SelbriSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:TermSyntax::Sumti",
        "term is exactly a validated SumtiSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiTagSyntax::TenseModal",
        "tag delegates all grammar markers to TenseModalSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiSyntax::QuotedSumti",
        "argument delegates all grammar markers to QuoteSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiSyntax::QuantifiedSumti",
        "variant combines validated quantifier and argument payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiSyntax::TaggedSumti",
        "variant combines a validated tag and argument payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiSyntax::SumtiConnection",
        "variant combines validated argument payloads through a validated connective",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiSyntax::SelbriVocative",
        "vocative relation has no required relative-clause marker beyond SelbriSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:RelativeClauseSyntax::RelativeClauseConnection",
        "variant combines a validated connective and relative-clause payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:DescriptionTailElementSyntax::DescriptionTailSumti",
        "tail element is exactly a validated SumtiSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:DescriptionTailElementSyntax::DescriptionTailQuantifier",
        "tail element is exactly a validated QuantifierSyntax payload",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MeksoSyntax::NumberMekso",
        "math expression delegates all marker checks to QuantifierSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MeksoSyntax::ForethoughtMeksoConnection",
        "forethought math expression uses validated connective and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MeksoSyntax::MeksoConnection",
        "connected math expression uses validated connective and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MeksoSyntax::Infix",
        "binary math expression uses a validated operator and expression payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:MeksoOperatorSyntax::OperatorConnection",
        "connected math operator uses validated connective and operator payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SelbriSyntax::SelbriConnection",
        "connected relation uses validated connective and relation payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SelbriSyntax::TaggedSelbri",
        "relation prefix delegates marker checks to TenseModalSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SelbriSyntax::Tanru",
        "compound relation non-emptiness is enforced by TanruUnitVec",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:TanruUnitSyntax::TanruUnitConnection",
        "connected tanru unit uses validated connective and unit payloads",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:TanruUnitSyntax::SelbriGroupTanruUnit",
        "wrapped tanru unit is exactly a validated SelbriSyntax payload",
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
        "apps/jbotci/src/main.rs:Command::Setup",
        "CLI enum delegates validation to clap and setup option handling",
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
        "xtask/src/main.rs:Command::RefsV0Parity",
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
    (
        "xtask/src/main.rs:Command::BuildWebRelease",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::DistServer",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Blank",
        "blank gentufa result is a unit state with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Success",
        "web API result delegates payload constraints to GentufaSuccess and construction path",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Error",
        "web API result delegates payload constraints to GentufaError and construction path",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebError::Dialect",
        "error wrapper carries parser diagnostic text without additional semantic state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaBracketFragment::Text",
        "web bracket fragments mirror renderer output, including empty fallback text",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaBracketFragment::Span",
        "web bracket spans are presentation wrappers whose payload is validated by child fragments",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Numbered",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Modal",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Fai",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockLayoutChild::Node",
        "internal borrowed layout cursor delegates validity to the referenced block tree node",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockLayoutChild::Leaf",
        "internal borrowed layout cursor delegates validity to the referenced leaf part",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Xml",
        "export error variant wraps the XML parser diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Svg",
        "export error variant wraps the SVG parser diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Png",
        "export error variant wraps the PNG encoder diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::InvalidSize",
        "export error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:SvgNode::Element",
        "typed SVG DOM node validity is delegated to the contained element",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:SvgNode::Text",
        "typed SVG DOM text is escaped during serialization before parser handoff",
    ),
    (
        "apps/jbotci-web/src/main.rs:AsyncTaskKind::Gentufa",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "apps/jbotci-web/src/main.rs:AsyncTaskKind::Cukta",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "apps/jbotci-web/src/main.rs:AsyncTaskKind::Vlacku",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "apps/jbotci-web/src/main.rs:AsyncTaskKind::Settings",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "apps/jbotci-web/src/main.rs:AsyncTaskKind::Export",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::EmbeddingCorpusJson",
        "embedding corpus worker request has no input payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::CuktaPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::CuktaSemanticPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::VlackuPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::VlackuSemanticPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaBlocksSvg",
        "export request is a serde protocol DTO and delegates block-layout validity to GentufaBlocksLayout",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaBlocksPng",
        "export request is a serde protocol DTO and delegates block-layout validity to GentufaBlocksLayout",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaPage",
        "compute response is a serde protocol DTO whose payloads are typed page data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::CuktaPage",
        "compute response is a serde protocol DTO whose payloads are typed page data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::VlackuPage",
        "compute response is a serde protocol DTO whose payloads are typed result data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::EmbeddingCorpusJson",
        "embedding corpus response intentionally carries opaque JSON for the browser embedding worker",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaBlocksSvg",
        "export response carries renderer output and the runner converts renderer errors before constructing it",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaBlocksPng",
        "export response carries renderer output and the runner converts renderer errors before constructing it",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeError::Json",
        "compute error variant carries serde's formatted diagnostic text",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeError::Export",
        "compute error variant carries renderer diagnostic text",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Gentufa",
        "route variant delegates URL state constraints to GentufaWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Cukta",
        "route variant delegates URL state constraints to CuktaWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Vlacku",
        "route variant delegates URL state constraints to VlackuWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Settings",
        "settings route is a unit state with no payload to constrain",
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
    let lines = source.lines().collect::<Vec<_>>();
    let mut pending = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if let Some((variant, next_index)) = invariant_attribute(&lines, index) {
            if let Some(variant) = variant {
                pending.push(variant);
            }
            index = next_index + 1;
            continue;
        }
        if let Some(enum_name) = enum_name(line) {
            for variant in pending.drain(..) {
                placeholders.insert(format!(
                    "{}:{enum_name}::{variant}",
                    relative_path.display()
                ));
            }
            index += 1;
            continue;
        }
        if !pending.is_empty()
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with("//")
        {
            pending.clear();
        }
        index += 1;
    }
}

#[requires(index < lines.len())]
#[ensures(true)]
fn invariant_attribute(lines: &[&str], index: usize) -> Option<(Option<String>, usize)> {
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

    Some((placeholder_variant(&attribute).map(str::to_owned), end))
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
