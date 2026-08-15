use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use walkdir::WalkDir;

const ALLOWED_PLACEHOLDERS: &[(&str, &str)] = &[
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Cukta",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Gentufa",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Gimfihi",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Jvozba",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Vlacku",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/discord.rs:DiscordCommand::Vlasei",
        "Discord command wrapper delegates payload validity to the parsed typed tool request",
    ),
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingSearchCache::Loaded",
        "loaded embedding cache validity is owned by the native embedding service type",
    ),
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingSearchCache::Unavailable",
        "unavailable embedding cache stores a validated cached error message",
    ),
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingSearchCache::Unloaded",
        "embedding search cache starts empty and is initialized on first semantic tool use",
    ),
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingToolRequest::Cukta",
        "embedding worker request delegates payload validity to the typed cukta tool request",
    ),
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingToolRequest::Vlacku",
        "embedding worker request delegates payload validity to the typed vlacku tool request",
    ),
    (
        "apps/jbotci/src/lib.rs:CliUsePrecomputed::Always",
        "CLI setup precomputed-pack policy is a closed clap value selector with no payload",
    ),
    (
        "apps/jbotci/src/lib.rs:CliUsePrecomputed::Auto",
        "CLI setup precomputed-pack policy is a closed clap value selector with no payload",
    ),
    (
        "apps/jbotci/src/lib.rs:CliUsePrecomputed::Never",
        "CLI setup precomputed-pack policy is a closed clap value selector with no payload",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Cukta",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Gentufa",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Gimfihi",
        "CLI enum delegates validation to clap and gimfihi option handling",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Jvozba",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Lsp",
        "CLI LSP command is a unit variant with no payload state to constrain",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Mulgau",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Setup",
        "CLI enum delegates validation to clap and setup option handling",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Tersmu",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Vlacku",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Vlasei",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Vlatai",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:Command::Zbasu",
        "CLI enum delegates validation to clap and command option structs",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiCliNormalizer::CandidateSide",
        "CLI ALINE normalizer is a closed selector mapped directly to the phonetic parameter set",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiCliNormalizer::SourceSide",
        "CLI ALINE normalizer is a closed selector mapped directly to the phonetic parameter set",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiCliNormalizer::Symmetric",
        "CLI ALINE normalizer is a closed selector mapped directly to the phonetic parameter set",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiCliScorer::Classic",
        "CLI gimfihi scorer is a closed selector mapped directly to the typed scoring mode",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiCliScorer::Phonetic",
        "CLI gimfihi scorer is a closed selector mapped directly to the typed scoring mode",
    ),
    (
        "apps/jbotci/src/lsp.rs:StructureBracketsInitialization::Enabled",
        "the boolean wire variant directly represents every valid structure-bracket enablement state",
    ),
    (
        "apps/jbotci/src/lsp.rs:StructureBracketsInitialization::Profile",
        "the structure profile wire variant delegates validity to DecorationProfile",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolAlineNormalizer::CandidateSide",
        "MCP ALINE normalizer is a closed selector mapped directly to the CLI normalizer",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolAlineNormalizer::SourceSide",
        "MCP ALINE normalizer is a closed selector mapped directly to the CLI normalizer",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolAlineNormalizer::Symmetric",
        "MCP ALINE normalizer is a closed selector mapped directly to the CLI normalizer",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCollisionScope::All",
        "MCP/Discord gimfi'i collision scope is a closed selector mapped directly to CLI collision scopes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCollisionScope::None",
        "MCP/Discord gimfi'i collision scope is a closed selector mapped directly to CLI collision scopes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCollisionScope::Official",
        "MCP/Discord gimfi'i collision scope is a closed selector mapped directly to CLI collision scopes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaFormat::Html",
        "MCP/Discord cukta output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaFormat::Markdown",
        "MCP/Discord cukta output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaFormat::Raw",
        "MCP/Discord cukta output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaMode::Example",
        "MCP/Discord cukta mode is a closed selector mapped directly to CLI lookup modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaMode::Meaning",
        "MCP/Discord cukta mode is a closed selector mapped directly to CLI lookup modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaMode::Section",
        "MCP/Discord cukta mode is a closed selector mapped directly to CLI lookup modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaMode::Toc",
        "MCP/Discord cukta mode is a closed selector mapped directly to CLI lookup modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaMode::Word",
        "MCP/Discord cukta mode is a closed selector mapped directly to CLI lookup modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaSearchResultKind::Example",
        "MCP cukta target is a closed selector of CLL content kinds mapped to CLI target filters",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaSearchResultKind::Paragraph",
        "MCP cukta target is a closed selector of CLL content kinds mapped to CLI target filters",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaSearchResultKind::Section",
        "MCP cukta target is a closed selector of CLL content kinds mapped to CLI target filters",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Brackets",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Json",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Png",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Raw",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Svg",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaFormat::Tree",
        "MCP/Discord gentufa output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiFormat::Json",
        "MCP/Discord gimfi'i output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiFormat::Table",
        "MCP/Discord gimfi'i output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiScorer::Classic",
        "MCP gimfihi scorer is a closed selector mapped directly to the CLI scorer",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiScorer::Phonetic",
        "MCP gimfihi scorer is a closed selector mapped directly to the CLI scorer",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaMode::Cmevla",
        "MCP/Discord jvozba mode is a closed selector mapped directly to CLI composition modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaMode::Lujvo",
        "MCP/Discord jvozba mode is a closed selector mapped directly to CLI composition modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaPartKind::FixedRafsi",
        "MCP/Discord jvozba part kind is a closed selector whose payload text is carried by ToolJvozbaPart",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaPartKind::Word",
        "MCP/Discord jvozba part kind is a closed selector whose payload text is carried by ToolJvozbaPart",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuMode::Lujvo",
        "MCP/Discord vlacku mode is a closed selector mapped directly to explicit dictionary search modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuMode::Meaning",
        "MCP/Discord vlacku mode is a closed selector mapped directly to explicit dictionary search modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuMode::Rafsi",
        "MCP/Discord vlacku mode is a closed selector mapped directly to explicit dictionary search modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuMode::Sound",
        "MCP/Discord vlacku mode is a closed selector mapped directly to explicit dictionary search modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuMode::Word",
        "MCP/Discord vlacku mode is a closed selector mapped directly to explicit dictionary search modes",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiFormat::Brackets",
        "MCP/Discord vlasei output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiFormat::Ipa",
        "MCP/Discord vlasei output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiFormat::Json",
        "MCP/Discord vlasei output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiFormat::Raw",
        "MCP/Discord vlasei output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiFormat::Tree",
        "MCP/Discord vlasei output format is a closed selector mapped directly to CLI render formats",
    ),
    (
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:DataChoice::Present",
        "contract scanner fixture must contain an accepted no-op marker",
    ),
    (
        "crates/bityzba/tests/type_invariant.rs:PlainChoice::Named",
        "bityzba fixture covers plain enum arm marker behavior",
    ),
    (
        "crates/bityzba/tests/type_invariant.rs:Tree::Branch",
        "bityzba fixture intentionally exercises audited no-op enum arm syntax",
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
        "crates/bityzba/tests/ui/fail/enum_unknown_variant_invariant.rs:Choice::Missing",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/bityzba/tests/ui/fail/enum_unknown_variant_invariant.rs:Choice::Named",
        "trybuild failure fixture intentionally uses placeholder syntax",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:AtomKind::Faha",
        "the motion-prefix flag is intentionally unconstrained; both Boolean states identify valid FAhA atoms",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:AtomKind::Roi",
        "the spatial-prefix and baseline-number flags are intentionally independent; all Boolean combinations identify valid ROI atoms",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:AtomKind::Tahe",
        "the spatial-prefix flag is intentionally unconstrained; both Boolean states identify valid TAhE atoms",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:AtomKind::Zaho",
        "the spatial-prefix flag is intentionally unconstrained; both Boolean states identify valid ZAhO atoms",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::ElidableTerminator",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::Hash",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::Nonterminal",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::Operator",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::Terminal",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/ebnf.rs:CllEbnfToken::Text",
        "EBNF presentation tokens are generated by the CLL grammar tokenizer before rendering",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::BlockQuote",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::CmavoList",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Code",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Definition",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::DisplayMath",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Ebnf",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Example",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::GrammarTemplate",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Heading",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::InterlinearGloss",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::List",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Lojbanization",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::LujvoMaking",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Media",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Paragraph",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Rule",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::SimpleListTable",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::Table",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllBlock::VariableList",
        "CLL content blocks are parsed presentation variants generated only by the DocBook loader",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllError::Load",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllError::NotFound",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllError::Parse",
        "CLL errors carry renderer/loader diagnostic text without additional semantic invariants",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Anchor",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::CiteTitle",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Code",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Elidable",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Emphasis",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::InlineMath",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::LanguageSpan",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Link",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Quote",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Subscript",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Superscript",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/model.rs:CllInline::Text",
        "CLL inline variants are generated from normalized DocBook inline nodes",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CuktaRequest::Example",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CuktaRequest::Search",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CuktaRequest::Section",
        "cukta request variants are validated from CLI/web mode parsing before execution",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::ByteOffsetNotCharBoundary",
        "diagnostic enum records rejected UTF-8 boundary inputs",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::ByteOffsetOutOfBounds",
        "diagnostic enum records rejected source offsets",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:DiagnosticSpanError::CharOffsetOutOfBounds",
        "diagnostic enum records rejected source offsets",
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
        "crates/jbotci-dictionary/src/import.rs:LensiskuImportError::Json",
        "error wrapper carries serde's diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:RafsiField::List",
        "raw import field accepts the upstream Lensisku list shape before normalization",
    ),
    (
        "crates/jbotci-dictionary/src/import.rs:RafsiField::Text",
        "raw import field accepts the upstream Lensisku scalar shape before normalization",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryLujvoSegmentKind::Hyphen",
        "generated lujvo segment kind is a closed selector validated against segment source fields by Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryLujvoSegmentKind::Rafsi",
        "generated lujvo segment kind is a closed selector validated against segment source fields by Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryValidationError::InvalidEntry",
        "validation error wrapper carries path and entry diagnostics",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryValidationError::InvalidLujvoIndexEntry",
        "validation error wrapper carries the lujvo index position and structural diagnostic",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryValidationError::InvalidSoundIndexEntry",
        "validation error wrapper carries the sound index position and structural diagnostic",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiAvailability::Free",
        "a free short rafsi has no claimants to constrain; the Taken alternative carries and validates the claimant list",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiClaimKind::Experimental",
        "rafsi claim standing is a closed selector over official and experimental gismu claims",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:RafsiClaimKind::Official",
        "rafsi claim standing is a closed selector over official and experimental gismu claims",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Backend",
        "embedding error variant carries only an already formatted backend diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::DimensionMismatch",
        "embedding error variant carries expected and actual dimensions produced by validation paths",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Environment",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::Http",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::InvalidIndex",
        "embedding error variant carries only an already formatted diagnostic message",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::InvalidModel",
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
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::MissingCompatiblePack",
        "embedding error variant carries the requested model key for rendering",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingError::UnsupportedModel",
        "embedding error variant carries the unsupported model key for rendering",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupIndexSource::BuiltLocal",
        "embedding setup index source is a closed status selector reported to users",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupIndexSource::DownloadedPrecomputed",
        "embedding setup index source is a closed status selector reported to users",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupIndexSource::Reused",
        "embedding setup index source is a closed status selector reported to users",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::Complete",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::DownloadingIndex",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::DownloadingModel",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::Error",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::Indexing",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::LoadingModel",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::ResolvingPaths",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::ReusingIndex",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::ValidatingIndex",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::ValidatingModel",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgressPhase::WritingIndex",
        "embedding setup progress phase is a closed status selector serialized for UI progress reporting",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:UsePrecomputed::Always",
        "embedding setup precomputed-pack policy is a closed CLI/API selector with no payload",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:UsePrecomputed::Auto",
        "embedding setup precomputed-pack policy is a closed CLI/API selector with no payload",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:UsePrecomputed::Never",
        "embedding setup precomputed-pack policy is a closed CLI/API selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::Absolute",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::Backslash",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::ControlCharacter",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::CurrentDirectory",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::Empty",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::EmptyComponent",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::ParentDirectory",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::SchemeOrDrive",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::TrailingSeparator",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ArtifactPathError::UrlMetacharacter",
        "artifact path errors are closed unit classifications; the rejected path is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:Sha256DigestError::Character",
        "digest character errors are a unit classification after the parser has rejected the supplied text",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:VectorStoreKeyError::ControlCharacter",
        "vector-store key errors are closed unit classifications; the rejected key is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:VectorStoreKeyError::Empty",
        "vector-store key errors are closed unit classifications; the rejected key is retained by the caller",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/core.rs:MergeSpec::Pair",
        "tokenizer merge specs are external artifact projections normalized by merge_pair before ranking",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/core.rs:MergeSpec::Text",
        "tokenizer merge specs are external artifact projections normalized by merge_pair before ranking",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/pack.rs:DistanceMetric::Dot",
        "distance metric is a closed wire-format selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/pack.rs:Pooling::MeanNormalizedWindows",
        "pooling strategy is a closed wire-format selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/pack.rs:VectorElementType::F16Le",
        "vector element type is a closed wire-format selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/pack.rs:VectorElementType::F32Le",
        "vector element type is a closed wire-format selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressKind::Corpus",
        "progress kind is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressKind::Model",
        "progress kind is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressKind::Pack",
        "progress kind is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressKind::Validation",
        "progress kind is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::Complete",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::Embedding",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::LoadingManifest",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::LoadingModel",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::LoadingTokenizer",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::Validating",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ProgressPhase::WritingPack",
        "progress phase is a closed status selector with no payload",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:Tensor::F32",
        "WebGPU tensor variant validity is checked while loading the manifest and constructing buffers",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:Tensor::Q4OnnxGather",
        "WebGPU tensor variant validity is checked while loading the manifest and constructing buffers",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:Tensor::Q4OnnxMatmul",
        "WebGPU tensor variant validity is checked while loading the manifest and constructing buffers",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:UniformValue::F32",
        "shader uniform variant is a typed scalar slot whose valid range is shader-specific",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:UniformValue::U32",
        "shader uniform variant is a typed scalar slot whose valid range is shader-specific",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockLayoutChild::Leaf",
        "internal borrowed layout cursor delegates validity to the referenced leaf part",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockLayoutChild::Node",
        "internal borrowed layout cursor delegates validity to the referenced block tree node",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockFrame::Chain",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockFrame::Collection",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockFrame::Field",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockFrame::Node",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Fai",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Modal",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::Numbered",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceSlotLabel::PlaceQuestion",
        "gentufa reference slot labels mirror the validated CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::InvalidSize",
        "export error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Png",
        "export error variant wraps the PNG encoder diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Svg",
        "export error variant wraps the SVG parser diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaExportError::Xml",
        "export error variant wraps the XML parser diagnostic without adding semantic state",
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
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::DuplicateSourceLanguage",
        "duplicate source language is produced from normalized source input for diagnostics",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::ExtraPresetLanguage",
        "extra preset language is produced from normalized source input for diagnostics",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::InvalidIpa",
        "IPA error carries the rejected IPA string and a human-readable reason with no further constraint",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::InvalidSourceSpec",
        "gimfihi error variant carries raw invalid source text plus a diagnostic message",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::InvalidSourceWord",
        "invalid source word preserves normalized invalid input, including the empty word case",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::InvalidWeight",
        "gimfihi error variant carries raw invalid user input, including possible empty text",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::MissingExplicitWeight",
        "missing custom weight is reported against normalized source input for diagnostics",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::MissingPresetLanguage",
        "missing preset language is produced from the validated preset table",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::Phonetic",
        "phonetic scoring errors preserve the typed lower-layer diagnostic without additional constraints",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::UnknownCollisionScope",
        "gimfihi error variant carries raw invalid user input, including possible empty text",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::UnknownPreset",
        "gimfihi error variant carries raw invalid user input, including possible empty text",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiError::UnknownShape",
        "gimfihi error variant carries raw invalid user input, including possible empty text",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiScorer::Classic",
        "gimfihi scorer is a closed selector over the classic and phonetic algorithms",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiScorer::Phonetic",
        "gimfihi scorer is a closed selector over the classic and phonetic algorithms",
    ),
    (
        "crates/jbotci-gimfihi/src/transliterate.rs:Snap::Letters",
        "Letters tags a static Lojban-letter string in the IPA segment table; the table is valid by construction",
    ),
    (
        "crates/jbotci-ide/src/snapshot/inlays.rs:InlayKind::Structure",
        "the structure kind delegates boundary validity to the closed StructureInlayKind selector",
    ),
    (
        "crates/jbotci-ide/src/snapshot/structure_inlays.rs:DecorationProfile::RawBrackets",
        "raw-brackets profile validity is delegated to typed options where every depth and construct-filter combination is valid",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::CouldNotBuildCompound",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::CouldNotBuildLujvo",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::FinalConsonant",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::FixedRafsiEmpty",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NoDictionaryEntry",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NoRafsiAvailable",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::NonFinalUniversalLongRafsi",
        "error payload is created only from the validated jvozba build path and rendered immediately",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaError::RequiresAtLeastTwoInputs",
        "jvozba error variant carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaInput::FixedRafsi",
        "public jvozba input enum is kept direct; parsing and build paths normalize and validate payloads before use",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaInput::Word",
        "public jvozba input enum is kept direct; parsing and build paths normalize and validate payloads before use",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaMode::Cmevla",
        "public build mode is a closed selector enum serialized directly for CLI and web callers",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaMode::Lujvo",
        "public build mode is a closed selector enum serialized directly for CLI and web callers",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaSegmentKind::Hyphen",
        "segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:JvozbaSegmentKind::Rafsi",
        "segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SAMatchTag::ExperimentalQuoteSelmaho",
        "experimental quote erasure tags are private static category names",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SAMatchTag::Selmaho",
        "selmaho erasure tags carry validated Selmaho values",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SegmentMode::Display",
        "segment mode is a private closed selector for trace label and FAhO handling",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SegmentMode::Morphology",
        "segment mode is a private closed selector for trace label and FAhO handling",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::Invalid",
        "diagnostic enum records rejected parser inputs",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::SourceSpan",
        "error wrapper delegates validity to SourceLocationError",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:MorphologyError::UnterminatedZoiQuote",
        "diagnostic enum records rejected quote input",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:LujvoPart::Hyphen",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:LujvoPart::Rafsi",
        "Phonemes owns canonical non-empty phoneme validity",
    ),
    (
        "crates/jbotci-morphology/src/tree.rs:WordLike::PlainWord",
        "bare word-like values delegate all validity to the wrapped Word",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Field",
        "JSON field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Node",
        "JSON builder frame validity is governed by traversal sequencing",
    ),
    (
        "crates/jbotci-output/src/json.rs:JsonFrame::Sequence",
        "JSON sequence frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/lib.rs:BracketSourceFragment::Span",
        "bracket source spans preserve renderer grouping, including empty intermediate spans",
    ),
    (
        "crates/jbotci-output/src/lib.rs:BracketSourceFragment::Text",
        "bracket source fragments preserve renderer output, including empty intermediate text",
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
        "crates/jbotci-output/src/lib.rs:OutputError::Json",
        "error wrapper carries serde's diagnostic without adding semantic state",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::Recovery",
        "error wrapper carries recovery renderer diagnostics",
    ),
    (
        "crates/jbotci-output/src/lib.rs:OutputError::References",
        "error wrapper carries reference analysis diagnostics",
    ),
    (
        "crates/jbotci-output/src/recovered.rs:RecoveredMorphologySequenceItem::Valid",
        "valid sequence items delegate source and morphology validity to the wrapped WordLike",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Fai",
        "unit slot marker carries no payload beyond the selected variant",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Modal",
        "reference display slot words are renderer projections of validated syntax",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::Numbered",
        "reference display slot is projected from validated semantic PlaceSlot values",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSlotName::PlaceQuestion",
        "reference display slot is projected from validated semantic PlaceSlot values",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:LeafRole::Elided",
        "unit leaf role is a closed renderer style selector with no payload",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:LeafRole::Error",
        "unit leaf role is a closed renderer style selector with no payload",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:LeafRole::Normal",
        "unit leaf role is a closed renderer style selector with no payload",
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
        "crates/jbotci-output/src/surface.rs:DisplaySpan::LojbanWord",
        "display spans are private renderer ranges produced from validated word spans before source-bound clipping",
    ),
    (
        "crates/jbotci-output/src/surface.rs:DisplaySpan::VerbatimText",
        "verbatim display spans are private renderer ranges produced from validated quote spans before source-bound clipping",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyFrame::Field",
        "field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyFrame::Node",
        "builder stack frame validity is governed by enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Chain",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Collection",
        "collection frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Field",
        "field frame permits empty values while traversal is in progress",
    ),
    (
        "crates/jbotci-output/src/tree.rs:SyntaxFrame::Node",
        "builder stack frame validity is governed by enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Collection",
        "empty render collections are valid intermediate output",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Error",
        "recovery error validity is enforced by the validated RecoveryTreeError payload",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Node",
        "render node payload owns constructor and entry shape",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Span",
        "span field ordering is preserved from SourceSpan before rendering",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Syntax",
        "syntax provenance wrapper delegates rendered value validity to its payload",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Text",
        "text render payload is a source-derived scalar with no extra enum-level rule",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Verbatim",
        "verbatim render text is source-derived and may be empty",
    ),
    (
        "crates/jbotci-output/src/tree.rs:TreeValue::Word",
        "word render fields are produced from validated morphology atoms",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Aspirated",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Back",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::High",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Lateral",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Long",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Manner",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Nasal",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Place",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Retroflex",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Round",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Syllabic",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineFeature::Voice",
        "ALINE feature enum is a closed selector set with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineNormalizer::CandidateSide",
        "ALINE normalizer is a closed selector over the three normative denominator modes",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineNormalizer::SourceSide",
        "ALINE normalizer is a closed selector over the three normative denominator modes",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineNormalizer::Symmetric",
        "ALINE normalizer is a closed selector over the three normative denominator modes",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:IpaSurfaceChunk::Text",
        "IPA text chunks preserve source-derived quote text and may be empty before filtering",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:IpaSurfaceChunk::Word",
        "IPA chunks borrow validated morphology words",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::EmptyBracketedIpa",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::EmptyQuery",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::MissingClosingBracket",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::MissingOpeningBracket",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::Morphology",
        "morphology error payload is already formatted by the source diagnostic type",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::NestedBrackets",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::NoPronounceableWords",
        "phonetic error payload preserves the rejected user input for display",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::PartialBracketedQuery",
        "phonetic error variant is a closed diagnostic selector with no payload invariants",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::Syllabification",
        "syllabification error payload is already formatted by the source diagnostic type",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:PhoneticError::UnsupportedSegment",
        "phonetic error payload preserves the unsupported IPA segment context for display",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:ExactPattern::Glob",
        "compiled glob patterns carry their validation in GlobPattern",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:ExactPattern::Regex",
        "compiled regex patterns carry their validation in regex::Regex",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::AnyMany",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::AnyOne",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Consonant",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Literal",
        "glob compiler normalizes literal tokens before constructing this internal matcher enum",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobToken::Vowel",
        "glob token variant is a closed matcher tag with no payload invariants",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionKind::Hyphen",
        "composition kind is a closed display tag; surface/source fields carry data validity",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionKind::Rafsi",
        "composition kind is a closed display tag; surface/source fields carry data validity",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::AnswerSelection",
        "static checker answer-selection elements are recursively validated StaticType values",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Concrete",
        "static checker concrete types are validated TypeExpr values",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Function",
        "static checker function parameters and result are recursively validated StaticType values",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::GeneralizedQuantifier",
        "static checker generalized-quantifier element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Group",
        "static checker group element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Interval",
        "static checker interval element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::List",
        "static checker list element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Predicate",
        "static checker predicate carries a validated row and aligned closure policies",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::PureProperty",
        "registry-only purity refinement wraps a recursively validated StaticType; its contextual function-parameter placement is validated by canonical_prelude_type_schema",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Query",
        "static checker query elements are recursively validated StaticType values",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::ReferenceComputation",
        "static checker reference-computation result is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Referents",
        "static checker referent element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Set",
        "static checker set element is a recursively validated StaticType value",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::Tuple",
        "static checker tuple elements are recursively validated StaticType values",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticType::TypeParameter",
        "registry type parameters carry a validated TypeParameterName identity",
    ),
    (
        "crates/jbotci-semantics/src/completeness/model.rs:ProjectionFailureSite::WholeGraph",
        "the whole-graph site has no fields to constrain; its raw root is fixed by WHOLE_GRAPH_RAW_ROOT_TYPE",
    ),
    (
        "crates/jbotci-semantics/src/completeness/model.rs:Witness::NoCorpusWitness",
        "unit witness marker for a field the frozen corpus does not exercise; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/completeness/model.rs:WitnessExpect::Present",
        "unit expectation asserting the coordinate is populated (present and non-null); no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/formulas.rs:GeneratedDirectTermOperand::Bound",
        "borrowed hierarchy operand delegates validity to the invariant-bearing BoundTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedAlternativeArgumentSource::Built",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedAlternativeArgumentSource::Sumti",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedAlternativeArgumentSource::SumtiBound",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedAlternativeArgumentSource::SumtiForethought",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierScopeNode::Sumti",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierScopeNode::SumtiBound",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierSource::NoGadriDescription",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierSource::OuterQuantifiedDescription",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierSource::QuantifiedSumti",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedBridiFormulaScope::ImplicitExistential",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedBridiFormulaScope::Term",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiBranch::Sumti",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiBranch::SumtiAfterthought",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiBranch::SumtiBound",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiBranch::SumtiForethought",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiBranch::SumtiGrouped",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiConnective::Argument",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDistributedSumtiConnective::Forethought",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedEventTenseModal::LeadingTermTag",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedEventTenseModal::TenseModal",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Ek",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Gihek",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::LinkedSumti",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::LinkedSumtiContinuation",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Mekso",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::MultipleNa",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Prenex",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::RelativeClause",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Selbri",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::SingleNa",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::Terms",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedFragmentRoot::ZantufaMekso",
        "borrowed fragment root delegates validity to its typed generated syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedOrderedFormulaScope::Argument",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedOrderedFormulaScope::Bundle",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedOrderedFormulaScope::ImplicitExistential",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedOrderedFormulaScope::Term",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPredicationEventuality::Absent",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPredicationEventuality::Fresh",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexFormulaScope::Negation",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexFormulaScope::Quantifier",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexSumtiSyntax::Complete",
        "borrowed prenex sumti view delegates validity to the referenced complete sumti syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexTermEvent::EndGroup",
        "prenex traversal group delimiters are balanced by the collector and scope-stack checks",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexTermEvent::Negation",
        "prenex negation events carry optional validated source provenance with no additional payload constraint",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexTermEvent::StartGroup",
        "prenex traversal group delimiters are balanced by the collector and scope-stack checks",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexTermEvent::Sumti",
        "prenex sumti events carry a typed borrowed syntax view and an intentionally total topic marker",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedOrderedFormulaScope::Argument",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedOrderedFormulaScope::Bundle",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedOrderedFormulaScope::ImplicitExistential",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedOrderedFormulaScope::Term",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPropertyTanruContext::Description",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPropertyTanruContext::PropertyAbstraction",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRecurrenceQuantity::Value",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRecurrenceQuantityCacheValue::Integer",
        "direct recurrence quantity cache integers are fully constrained by the i64 payload type",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRelationParameterSyntax::GohaWord",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRelationParameterSyntax::ProBridi",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRelationQuestionSyntax::GohaWord",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRelationQuestionSyntax::ProBridi",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedScalarNegationScope::MarkerOnly",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTanruAtomBaseView::Cei",
        "borrowed generated-syntax view delegates validity to the referenced syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTanruAtomBaseView::Normal",
        "borrowed generated-syntax view delegates validity to the referenced syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTanruAtomView::Cei",
        "borrowed generated-syntax view delegates validity to the referenced syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTanruAtomView::Normal",
        "borrowed generated-syntax view delegates validity to the referenced syntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTermFormulaScope::Negation",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::ParagraphBoundary",
        "paragraph boundary markers use Vec1, so the typed plan item is nonempty by construction",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::PendingStatementConnection",
        "pending statement connections borrow their typed separator and connective syntax nodes",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::Root",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::StandaloneFreeModifiers",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::StandaloneParagraphBoundary",
        "paragraph boundary markers use Vec1, so the typed plan item is nonempty by construction",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlanItem::TrailingSeparator",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::Bridi",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::ForethoughtStatement",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::Fragment",
        "borrowed text root delegates fragment validity to GeneratedFragmentRoot",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::PrenexStatement",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::PreposedStatementConnection",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::StatementConnection",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::TextGroupStatement",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextRoot::ZantufaStatementTerms",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedLinkedSumtiRef::Empty",
        "the empty linked-sumti leaf is a closed marker with no payload state to constrain",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedLinkedSumtiRef::PlaceTagged",
        "borrowed leaf validity is owned by the invariant-bearing PlaceTaggedLinkedSumtiSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedLinkedSumtiRef::Plain",
        "borrowed leaf validity is owned by the invariant-bearing PlainLinkedSumtiSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedLinkedSumtiRef::TenseTagged",
        "borrowed leaf validity is owned by the invariant-bearing TenseTaggedLinkedSumtiSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::BareNaTerm",
        "borrowed leaf validity is owned by the invariant-bearing BareNaTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::ElidedNaheFihoTagTerm",
        "borrowed leaf validity is owned by the invariant-bearing ElidedNaheFihoTagTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::FihoiAdverbialTerm",
        "borrowed leaf validity is owned by the invariant-bearing FihoiAdverbialTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::ForethoughtTermset",
        "borrowed leaf validity is owned by the invariant-bearing ForethoughtTermsetSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::GekTermset",
        "borrowed leaf validity is owned by the invariant-bearing GekTermsetSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::JaiTaggedSumtiTerm",
        "borrowed leaf validity is owned by the invariant-bearing JaiTaggedSumtiTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::KeTermset",
        "borrowed leaf validity is owned by the invariant-bearing KeTermsetSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::NaKuTerm",
        "borrowed leaf validity is owned by the invariant-bearing NaKuTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::NoihaAdverbialTerm",
        "borrowed leaf validity is owned by the invariant-bearing NoihaAdverbialTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::NuhiTermset",
        "borrowed leaf validity is owned by the invariant-bearing NuhiTermsetSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::PlaceTaggedSumtiTerm",
        "borrowed leaf validity is owned by the invariant-bearing PlaceTaggedSumtiTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::SoiAdverbialTerm",
        "borrowed leaf validity is owned by the invariant-bearing SoiAdverbialTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::SumtiTerm",
        "borrowed leaf validity is owned by the invariant-bearing SumtiTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm",
        "borrowed leaf validity is owned by the invariant-bearing TaggedSumtiBeforeTagTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/generated_term_view.rs:GeneratedSimpleTermRef::TaggedSumtiTerm",
        "borrowed leaf validity is owned by the invariant-bearing TaggedSumtiTermSyntax node",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:MathLiteralValue::Integer",
        "all i64 values are valid math integer literal payloads",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticIdPrefix::Referent",
        "referent ID prefixes carry a strongly typed semantic sort; graph validation checks sort/object agreement",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticIdPrefix::Structural",
        "structural ID prefixes carry a strongly typed object kind; constructors choose the allowed structural kind",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticOperator::Formula",
        "formula operators are closed enum values with no additional payload constraint",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticSort::Eventuality",
        "eventuality sort payload is a closed EventualitySort enum and has no additional per-variant constraint",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SequenceRelation::ParagraphBoundary",
        "paragraph boundaries store one required typed transition plus zero or more additional transitions",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SequenceRelation::SameTopicContinuation",
        "same-topic continuation is a unit relation with no invalid representation",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:PredicateArgument::Computed",
        "the PlaceOf type and validated nonempty unique candidate domain carry all syntactic constraints",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:PredicateArgument::Eventuality",
        "every validated value type may be checked against the event slot during application",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:PredicateArgument::Numbered",
        "PositiveInteger proves place positivity and application checks the independently valid value type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:PredicateArgument::Plain",
        "every validated type may be checked at the numbered fill cursor",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:AnswerSelection::Contextual",
        "the selection's closed literal payload is the whole constraint; matching a query is an Answer invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:AnswerSelection::Polar",
        "the selection's closed literal payload is the whole constraint; matching a query is an Answer invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:AnswerSelection::Unresolved",
        "the selection's closed literal payload is the whole constraint; matching a query is an Answer invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Binary",
        "the connective's operand categories are fixed by the field types and it constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Bind",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Let",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::LetRec",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Not",
        "the connective's operand categories are fixed by the field types and it constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Presuppose",
        "the connective's operand categories are fixed by the field types and it constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Quantified",
        "the connective's operand categories are fixed by the field types and it constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Content::Supplement",
        "the connective's operand categories are fixed by the field types and it constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Query::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Query::Open",
        "the query's payload is already validated by its own type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/content.rs:Query::Polar",
        "the query's payload is already validated by its own type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Act::Ask",
        "the act's operand categories are fixed by the field types and its force is derived rather than stored redundantly",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Act::Assert",
        "the act's operand categories are fixed by the field types and its force is derived rather than stored redundantly",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Act::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Act::Express",
        "the act's operand categories are fixed by the field types and its force is derived rather than stored redundantly",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Act::Mention",
        "the act's operand categories are fixed by the field types and its force is derived rather than stored redundantly",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Discourse::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Discourse::Following",
        "the operand category is fixed by the field type; reference-only performance is checked where a value is performed",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Discourse::Perform",
        "the operand category is fixed by the field type; reference-only performance is checked where a value is performed",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Discourse::PerformUtterance",
        "the operand category is fixed by the field type; reference-only performance is checked where a value is performed",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Discourse::Prior",
        "the operand category is fixed by the field type; reference-only performance is checked where a value is performed",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::Act",
        "the performable union joins already validated categories; reference-only performance is a Discourse and document invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::Bind",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::Discourse",
        "the performable union joins already validated categories; reference-only performance is a Discourse and document invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::Entry",
        "the performable union joins already validated categories; reference-only performance is a Discourse and document invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::Let",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:Performable::LetRec",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/performable.rs:TranscriptEntry::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PlaceFill::Eventuality",
        "a fill's legality is decided by the application kernel at the predicate term that consumes it",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PlaceFill::Numbered",
        "a fill's legality is decided by the application kernel at the predicate term that consumes it",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PlaceFill::Plain",
        "a fill's legality is decided by the application kernel at the predicate term that consumes it",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PredTerm::Bind",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PredTerm::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PredTerm::Let",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PredTerm::LetRec",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/predicate.rs:PredTerm::Relation",
        "a relation term is validated by its predicate signature, whose row and identity are already checked",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:PlaceLabel::Eventuality",
        "the unit variant is the one distinguished event-row label",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:PlaceLabel::Numbered",
        "PositiveInteger proves every numbered row label is canonical, positive, and unbounded",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RelationRef::DropPlace",
        "the nested relation is validated and PositiveInteger proves the deleted ordinal is canonical and positive",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RelationRef::Lexical",
        "LexicalRoot proves the lowercase or escaped-lowercase relation namespace",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RelationRef::Scalar",
        "ScalarKind is closed and the nested relation reference is validated",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RelationRef::Tanru",
        "both modifier and row-owning head are independently validated relation references",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RelationRef::Variable",
        "Variable proves the lexically bound relation-reference namespace",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Act",
        "Force is a closed enum and every member indexes a valid Act type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::AnswerSelection",
        "every ordered sequence of validated types is a valid answer tuple parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Atom",
        "TypeAtom is the closed primitive and literal-family type namespace",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Function",
        "every ordered parameter sequence and validated result form a syntactically valid Fn type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::GeneralizedQuantifier",
        "every validated inner type is a valid GQ type parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Group",
        "every validated inner type is a valid Group type parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Interval",
        "every validated inner type is syntactically valid for Interval; orderedness is checked by signatures",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::List",
        "every validated inner type is a valid List type parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::PlaceOf",
        "the relation, accepted type, and optional nonempty unique candidate set are independently validated",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Predicate",
        "Row proves canonical unique row labels and open-tail placement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Query",
        "every ordered sequence of validated types is a valid query tuple parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::ReferenceComputation",
        "every validated inner type is a valid RefComp result type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Referents",
        "every validated inner type is a valid Referents type parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Set",
        "every validated inner type is a valid Set type parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Sign",
        "SignKind is a closed enum and every member indexes a valid Sign type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::SignToken",
        "SignKind is a closed enum and every member indexes a valid SignToken type",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:TypeExpr::Tuple",
        "every ordered sequence of validated types is a valid tuple parameter",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::Bind",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::Lambda",
        "the callable's signature is carried by its own already validated payload",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::Let",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::LetRec",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:FnValue::Registered",
        "the callable's signature is carried by its own already validated payload",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::AnswerExhaustivity",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::AnswerPolarity",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::EndpointInclusion",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::Force",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::Integer",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::LabelLevel",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::LexicalScopePolicy",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::Proximity",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::ScalarKind",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::Scale",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::SignKind",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Literal::Text",
        "the literal's closed family type is the whole constraint",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Act",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Content",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Discourse",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Entry",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Function",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Predicate",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Query",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Operand::Value",
        "the operand union joins already validated categories and constrains nothing further",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Value::Bind",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Value::Bound",
        "a bound reference carries the type its binder declared; the document scope audit proves that agreement",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Value::Let",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Value::LetRec",
        "the generic binding form validates its own declarations, initializer types, and free-binder set",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/value.rs:Value::Literal",
        "the value's payload is already validated by its own type",
    ),
    (
        "crates/jbotci-semantics/src/notation/mod.rs:NotationProfile::Smusni",
        "unit notation-profile selector; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:GeneratedFailureSite::TypedPosition",
        "the generated site is the unchecked serialization shape; DispositionRegistry::try_from_generated is the single boundary that validates it",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:GeneratedFailureSite::WholeGraph",
        "the generated site is the unchecked serialization shape; DispositionRegistry::try_from_generated is the single boundary that validates it",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:RegisteredDisposition::Failure",
        "FailureBoundary already validates the exact disposition/reason/site/class/owner join",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:RegisteredFailureSite::TypedPosition",
        "the typed position carries an already parsed v0 type and a closed minimum raw owner",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:RegisteredFailureSite::WholeGraph",
        "the whole-graph site has no fields to constrain; its raw root is fixed by the registry join",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/kernel_printer.rs:Expected::Unknown",
        "an absent expectation is the printer's neutral context, in which every crossing prints explicitly",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs:Datum::Atom",
        "Atom is already a validated lexical type, so every Atom payload is valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs:Datum::Integer",
        "integer payload is an already validated arbitrary-precision canonical decimal spelling",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs:Datum::List",
        "every sequence of already validated Datum children is a valid S-expression list",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs:Datum::String",
        "every Rust string has one canonical escaped S-expression string spelling",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:CompactFallbackCause::UnrecognizedObjectFamily",
        "every closed SemanticObjectKind is valid evidence for this private conservative fallback cause",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:GeneralizedQuantification::ConstructionRejected",
        "unit outcome tag records that typed constructor assembly rejected the operands; it has no payload state to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:GeneralizedQuantification::DeclinedOperand",
        "unit outcome tag records that an operand had no compact projection; it has no payload state to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:GeneralizedQuantification::PurityUnproven",
        "unit outcome tag records the standalone purity judgment's conservative refusal; it has no payload state to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:GeneralizedQuantification::Rendered",
        "successful generalized-quantifier construction carries an already validated Content value",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:Capture::Local",
        "a local capture is an already validated LocalFallback with a registered reason and an ordered raw tree",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:Capture::WholeGraph",
        "a whole-graph capture is an already validated TypedGraph with a registered reason and an ordered raw tree",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Atom",
        "NfcText validates the exact string payload of RawAtom",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::List",
        "every sequence of validated raw values is a valid RawList payload",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Map",
        "every sequence of validated RawMapEntry values is a valid RawMap payload",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Null",
        "the unit variant exactly represents RawNull",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Object",
        "RawObject contains validated identity and text fields; cross-tree identity order is enforced by RawTree",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Record",
        "RawRecord carries independently validated NFC names, fields, and recursively valid raw values",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Ref",
        "ObjectId proves positivity; cross-tree reference order is enforced by RawTree",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Scalar",
        "both the model scalar type and its exact lexical value are independently NFC-normalized",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::String",
        "NfcText validates the exact string payload of RawString",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::TypedAtom",
        "both model enum type and case are independently NFC-validated strings",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawValue::Variant",
        "RawVariant carries independently validated NFC enum identity, constructor, fields, and raw values",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Bool",
        "every bool is a valid typed structural scalar",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Map",
        "map entries contain already validated structural keys and values; all entry sequences are representable",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Sequence",
        "sequence children are already validated StructuralValue instances and empty sequences remain meaningful",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Signed",
        "every i128 is a valid typed structural scalar",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::String",
        "every Rust string is representable through the canonical escaped string path",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Unit",
        "unit is a payload-free marker used for absent and unit Serde values",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralValue::Unsigned",
        "every u128 is a valid typed structural scalar",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:ApplicationArgument::ComputedPlace",
        "the place and value are independently validated expressions and every pair is a syntactically valid computed fill",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:ApplicationArgument::EventualityPlace",
        "every validated value expression is syntactically valid at the distinguished event-place marker",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:ApplicationArgument::NumberedPlace",
        "PositiveInteger proves the unbounded place marker is positive and the value expression is independently validated",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:ApplicationArgument::Value",
        "every validated expression is a syntactically valid ordinary application argument",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:LetBinding::Prelude",
        "PreludeBinding carries a registry-validated prelude name, type, and expression",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:LetBinding::Variable",
        "ValueBinding carries a validated variable declaration and expression",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:PlaceMarker::Eventuality",
        "the unit variant is the one distinguished event-place marker",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:PlaceMarker::Numbered",
        "PositiveInteger proves every numbered application marker is canonical, positive, and unbounded",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Application",
        "Application proves the nonempty exact application production",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Atom",
        "ValueAtom proves membership in a closed value-position atom namespace",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Bind",
        "BindForm proves the canonical one-variable Bind production",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Integer",
        "Integer proves canonical arbitrary-precision decimal syntax",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Lambda",
        "Lambda proves a nonempty duplicate-free typed parameter list",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Let",
        "LetForm proves the canonical one-binding Let production",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::LetRec",
        "LetRec proves nonempty unique variable bindings with lambda initializers",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Rational",
        "Rational proves a positive denominator and lowest terms",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Sign",
        "Sign proves the exact typed token binder and nonempty fact list",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::String",
        "NfcText proves the string is normalized before the shared escaping boundary",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Utterance",
        "Utterance proves the exact typed token binder and nonempty fact list",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Expr::Variable",
        "Variable proves the reserved dollar-prefixed symbol grammar",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:LexicalEdgeAttempt::Constructed",
        "constructed attempts carry an already validated lexical dynamic edge",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:LexicalEdgeFallbackReason::Lookup",
        "lookup fallback carries a validated closed lookup-failure value",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:LexicalPolicyLookupFailure::UnknownRelation",
        "unknown-relation failure carries the exact validated attempted key",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:LexicalPolicyLookupFailure::UnsupportedPlace",
        "unsupported-place failure carries the exact validated attempted key",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Abstraction",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Cardinal",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Connective",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Figurative",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Identity",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::LetterOf",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Ordinal",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::PredicationNegation",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::Recurrence",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::ReferentOf",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::ScalarNegation",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxExpr::TaggedPlace",
        "composition-node variant whose fields carry their own constraints; the variant adds no cross-field invariant beyond the child expressions and the CompositeApprox escalation checks",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxReferent::Parameter",
        "context-referent variant whose payload is constrained by its own field types and the enum's other arms; no additional cross-field invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxReferent::PersonalMass",
        "context-referent variant whose payload is constrained by its own field types and the enum's other arms; no additional cross-field invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ApproxReferent::Unspecified",
        "context-referent variant whose payload is constrained by its own field types and the enum's other arms; no additional cross-field invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:NumTok::Percent",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:NumTok::Point",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:NumTok::Quantifier",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Piece::Num",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Piece::Tok",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::Abstraction",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::Actuality",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::Aspect",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::Figurative",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::PlaceDeletion",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::ScalarNegation",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::SpaceWhole",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::TaggedPlace",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:PrefixOp::TimeWhole",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Bo",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Co",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Connective",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Expr",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Ke",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Kee",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Kei",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Na",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Postfix",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:Tok::Prefix",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ZeiPartRef::Word",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:ZeiPartRef::WordLike",
        "internal composition-builder token; validity is established by the classification and parsing passes that construct it, not by a payload invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:MixedContent::Element",
        "mutable serializer construction state; validity is established by the emission code and the canonical serializer, mirroring the audited XmlElement no-op",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:MixedContent::Text",
        "mutable serializer construction state; validity is established by the emission code and the canonical serializer, mirroring the audited XmlElement no-op",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlRepresentationPlan::Compact",
        "unit representation-plan marker; the alternative variant enforces the nonempty incompatibility evidence",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::AssignedNameRecord",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::BoundVariableWord",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::CompositionRelationLabel",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::ConnectorProvenance",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::DescriptorWord",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::IntroducedBy",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::QuantityText",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlWaiverFamily::SourceRecord",
        "unit omission-family marker; no payload to constrain",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Co",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Compound",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::ConnectiveBranches",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Conversion",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Forward",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFramePropagation::Jai",
        "fixture frame propagation is a serialization projection of validated frame ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceSlot::Modal",
        "fixture place slots are serialization projections of PlaceSlot values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceSlot::Numbered",
        "fixture place slots are serialization projections of PlaceSlot values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceSlot::PlaceQuestion",
        "fixture place slots are serialization projections of PlaceSlot values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::AmbiguousNodes",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::ResolvedFrame",
        "fixture reference targets are serialization projections of ReferenceTarget values",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceTarget::ResolvedNode",
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
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Co",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Compound",
        "frame ids are validated through PlaceAnalysis lookup APIs and empty modifier lists are valid",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::ConnectiveBranches",
        "connective-branch propagation may be temporarily empty for partially analyzed or unresolved selbri structures",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Conversion",
        "NonZeroU8 owns converted-place non-zero validity and frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Forward",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceFramePropagation::Jai",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceSlot::Modal",
        "modal slot payload is an optional syntax node anchor and any option state is meaningful",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceSlot::Numbered",
        "NonZeroU8 owns the non-zero numbered place invariant",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceSlot::PlaceQuestion",
        "place-question slots are unit markers; numbered and modal variants carry the constrained payloads",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::AmbiguousNodes",
        "an empty ambiguity set is valid while callers preserve an explicit unresolved state separately",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::ResolvedFrame",
        "frame ids are validated through PlaceAnalysis lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceTarget::ResolvedNode",
        "node ids are validated through SyntaxIndex lookup APIs",
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
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Boxed",
        "boxed schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Chain",
        "chain schema arguments are independently validated binding types and every pairing faithfully represents source type arguments",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Fixed",
        "fixed schema elements are validated recursively and every usize length, including zero, is a valid Rust array shape",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::NonEmptyRepeated",
        "nonempty-repetition schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Optional",
        "optional schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::RecoveredField",
        "recovered-field schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Reference",
        "schema reference validity is fully determined by the validated BindingReference payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Repeated",
        "repetition schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Shared",
        "shared schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::Tuple",
        "tuple schema elements are validated recursively and arbitrary ordered arity, including zero, is a valid Rust tuple shape",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:BindingType::WithIndicators",
        "indicator schema wrapper validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:ParserExpr::Chain",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:ParserExpr::Postfix",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:ParserExpr::Rust",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:ParserExpr::Vector",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Arc",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Boxed",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Choice",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Cmavo",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Ignored",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Lookahead",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Many",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Many1",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Not",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::NotNextRule",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::NotNextSelmaho",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::NotNextToken",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Opaque",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Opt",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::PayloadStart",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Rule",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Selmaho",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::Sequence",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::WithFreeModifiers",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveryExpr::WordCategory",
        "macro recovery metadata variants delegate validity to their typed payloads and generated metadata tests",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:Rule::Alias",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:Rule::Enum",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:Rule::Struct",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::Assert",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::One",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::OneOrMore",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::OneOrMoreSpread",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::Spread",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::ZeroOrMore",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:VectorItem::ZeroOrMoreSpread",
        "macro parser AST variants delegate validity to their typed syn or grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Boxed",
        "external boxed schema validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Chain",
        "external chain arguments are independently validated binding types and every pairing mirrors normalized source type arguments",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Fixed",
        "external fixed elements are validated recursively and every usize length, including zero, mirrors a valid Rust array shape",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::NonEmptyRepeated",
        "external nonempty-repetition validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Optional",
        "external optional schema validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::RecoveredField",
        "external recovered-field validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Repeated",
        "external repetition schema validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Shared",
        "external shared schema validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::Tuple",
        "external tuple elements are validated recursively and arbitrary ordered arity, including zero, mirrors valid Rust tuple shapes",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:BindingType::WithIndicators",
        "external indicator schema validity is fully determined by its validated recursive binding type payload",
    ),
    (
        "crates/jbotci-syntax-macros/tests/binding-schema-consumer/src/lib.rs:ModelKind::Sum",
        "external sum model kind is a unit discriminant with no payload combination to constrain",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryTrialClassification::AcceptedProgress",
        "accepted-progress payload validity is enforced by the invariant-bearing RecoveryProgressTrial type",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryTrialClassification::AcceptedSuccess",
        "accepted-success payload validity is enforced by the invariant-bearing RecoverySuccessTrial type, while the observation flag accepts both values",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryTrialClassification::Rejected",
        "rejected trials carry only an optional invariant-bearing trace report, so every payload state is valid",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxDiagnosticObservation::Candidate",
        "diagnostic candidates delegate validity to the private copy-on-write SyntaxParseError payload",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:MaybeRef::Ref",
        "borrowed parser error tokens delegate validity to the referenced token type",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:MaybeRef::Val",
        "owned parser error tokens delegate validity to the token type",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RichPattern::EndOfInput",
        "end of input is a closed parser expectation with no payload",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RichPattern::Label",
        "parser labels preserve arbitrary diagnostic text supplied by grammar callers",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RichReason::Custom",
        "custom parser reasons delegate validity to their typed diagnostic payload",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RichReason::ExpectedFound",
        "empty expectation sets and absent found tokens are valid parser error states",
    ),
    (
        "crates/jbotci-syntax/src/grammar/tokens.rs:ExperimentalCmavoContext::Label",
        "experimental cmavo context labels are private static grammar category names",
    ),
    (
        "crates/jbotci-syntax/src/grammar/tokens.rs:ExperimentalCmavoContext::Selmaho",
        "experimental cmavo context carries validated Selmaho values",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxError::Parse",
        "diagnostic enum records parser error location and message",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithIndicators::Plain",
        "plain indicator wrapper carries only the generic payload; payload invariants are owned by its type",
    ),
    (
        "crates/jbotci-tree-macros/src/lib.rs:UnwrappedTreeType::Atom",
        "tree macro helper variants delegate validity to borrowed syn types collected from parsed input",
    ),
    (
        "crates/jbotci-tree-macros/src/lib.rs:UnwrappedTreeType::Children",
        "tree macro helper variants delegate validity to borrowed syn types collected from parsed input",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:Recovered::Error",
        "generic recovery slot wrapper delegates missing versus invalid classification to the typed recovery item",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:Recovered::Prefix",
        "generic recovery slot wrapper delegates prefix error validity to Vec1 and typed recovery items",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:Recovered::Valid",
        "generic recovery slot wrapper delegates semantic validity to the contained recovered field state",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:TreePathStep::SequenceIndex",
        "tree path sequence indices accept every usize value",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:WrappedNode::Named",
        "tree macro test wrapper has no marker-specific payload rule",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:WrappedNode::Tuple",
        "tree macro test wrapper delegates validity to the wrapped payload",
    ),
    (
        "crates/jbotci-ui/src/diagnostics.rs:ActiveDiagnosticTarget::Context",
        "diagnostic hover context target validity depends on the current diagnostics and is guarded at use sites",
    ),
    (
        "crates/jbotci-ui/src/diagnostics.rs:ActiveDiagnosticTarget::Primary",
        "diagnostic hover primary target validity depends on the current diagnostics and is guarded at use sites",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:NativeEmbeddingSearchCommand::Clear",
        "native embedding clear command carries only a typed response channel",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:NativeEmbeddingSearchCommand::Search",
        "native embedding search command validity is enforced by worker-handle preconditions before sending",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Cukta",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Export",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Gentufa",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Gimfihi",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Settings",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncTaskKind::Vlacku",
        "activity task kind is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:PlatformAvailability::Available",
        "platform availability success state is a unit discriminant with no payload to constrain",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:PlatformAvailability::Unavailable",
        "platform unavailability reason is produced by platform service implementations and serialized as display text",
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
        "crates/jbotci-web-core/src/lib.rs:GentufaBracketFragment::Span",
        "web bracket spans are presentation wrappers whose payload is validated by child fragments",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaBracketFragment::Text",
        "web bracket fragments mirror renderer output, including empty fallback text",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebError::Dialect",
        "error wrapper carries parser diagnostic text without additional semantic state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Blank",
        "blank gentufa result is a unit state with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Error",
        "web API result delegates payload constraints to GentufaError and construction path",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebResult::Success",
        "web API result delegates payload constraints to GentufaSuccess and construction path",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPieceKind::Hyphen",
        "composition piece kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPieceKind::Rafsi",
        "composition piece kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaItemKind::FixedRafsi",
        "web jvozba item kind is a closed UI selector whose value is stored on the surrounding item",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaItemKind::Word",
        "web jvozba item kind is a closed UI selector whose value is stored on the surrounding item",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaMode::Cmevla",
        "web jvozba mode is a closed UI selector serialized directly in local storage",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaMode::Lujvo",
        "web jvozba mode is a closed UI selector serialized directly in local storage",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::Empty",
        "web jvozba output state carries no payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaOutput::Error",
        "web jvozba error payload carries the shared builder diagnostic text",
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
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentKind::Hyphen",
        "web jvozba segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentKind::Rafsi",
        "web jvozba segment kind is a closed presentation selector with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegmentTone::Hyphen",
        "web jvozba segment tone is a closed presentation selector with no payload to constrain",
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
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Meaning",
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
        "crates/jbotci-web-core/src/lib.rs:VlackuWebMode::Word",
        "web search mode is a closed UI selector serialized directly in URLs and local state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Brivla",
        "word type section is a closed grouping selector derived from dictionary metadata",
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
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeSection::Other",
        "word type section is a closed grouping selector derived from dictionary metadata",
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
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::EmbeddingCorpusJson",
        "embedding corpus worker request has no input payload beyond the discriminant",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaBlocksPng",
        "export request is a serde protocol DTO and delegates block-layout validity to GentufaBlocksLayout",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaBlocksSvg",
        "export request is a serde protocol DTO and delegates block-layout validity to GentufaBlocksLayout",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GentufaPage",
        "compute request is a serde protocol DTO and delegates payload validity to typed fields plus the runner",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeRequest::GimfihiPage",
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
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::CuktaPage",
        "compute response is a serde protocol DTO whose payloads are typed page data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::EmbeddingCorpusJson",
        "embedding corpus response intentionally carries opaque JSON for the browser embedding worker",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaBlocksPng",
        "export response carries renderer output and the runner converts renderer errors before constructing it",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaBlocksSvg",
        "export response carries renderer output and the runner converts renderer errors before constructing it",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GentufaPage",
        "compute response is a serde protocol DTO whose payloads are typed page data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::GimfihiPage",
        "compute response is a serde protocol DTO whose payloads are typed result data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebComputeResponse::VlackuPage",
        "compute response is a serde protocol DTO whose payloads are typed result data and metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Cukta",
        "route variant delegates URL state constraints to CuktaWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Gentufa",
        "route variant delegates URL state constraints to GentufaWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Gimfihi",
        "route variant delegates URL state constraints to GimfihiWebState and canonical route builders",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Settings",
        "settings route is a unit state with no payload to constrain",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebRoute::Vlacku",
        "route variant delegates URL state constraints to VlackuWebState and canonical route builders",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:BracketExpectations::Legacy",
        "fixture bracket expectation wrapper delegates text validity to TextExpectation and restricts script selection through accessors",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:BracketExpectations::Scripts",
        "fixture bracket expectation wrapper delegates per-script optionality to ScriptBracketExpectations",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::DuplicateId",
        "fixture error wrapper carries duplicate-id diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::EncodeToml",
        "fixture error wrapper carries TOML encoder diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::InvalidDialect",
        "fixture error wrapper carries dialect diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::InvalidLojbanSource",
        "fixture error wrapper carries fixture source declaration diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::InvalidXfail",
        "fixture error wrapper carries xfail diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::LegacyExpectationFormat",
        "fixture error wrapper carries legacy-format diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::ParseJson",
        "fixture error wrapper carries JSON parser diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::ParseToml",
        "fixture error wrapper carries TOML parser diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::Read",
        "fixture error wrapper carries filesystem diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::UnknownFacet",
        "fixture error wrapper carries facet-name diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::Walk",
        "fixture error wrapper carries directory traversal diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureError::Write",
        "fixture error wrapper carries filesystem diagnostics",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaFixtureInput::FixedRafsi",
        "fixture jvozba input preserves fixture text so failure cases can exercise downstream validation",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaFixtureInput::Word",
        "fixture jvozba input preserves fixture text so failure cases can exercise downstream validation",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaFixtureMode::Cmevla",
        "fixture jvozba mode is a closed serialization selector",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaFixtureMode::Lujvo",
        "fixture jvozba mode is a closed serialization selector",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaSegmentKindExpectation::Hyphen",
        "fixture jvozba segment kind is a closed expected-output selector",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaSegmentKindExpectation::Rafsi",
        "fixture jvozba segment kind is a closed expected-output selector",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Provenance::Adhoc",
        "ad hoc provenance intentionally permits absent description",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Provenance::Cll",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Provenance::Corpus",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Provenance::Muplis",
        "fixture tree validation checks provenance completeness at import time",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Provenance::Other",
        "fixture tree validation checks custom provenance names at import time",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TextExpectationWire::Table",
        "fixture text expectation wire table is validated after deserialization so serde can report field-level parse errors",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TextExpectationWire::Text",
        "fixture text expectation wire format accepts any legacy inline string; payload validity is checked after deserialization",
    ),
    (
        "xtask/src/main.rs:Command::DesktopBuild",
        "xtask desktop build command delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::DesktopBundleLinux",
        "xtask desktop bundle command delegates validation to typed bundle target helpers",
    ),
    (
        "xtask/src/main.rs:Command::DesktopBundleMacos",
        "xtask desktop bundle command delegates validation to typed bundle target helpers",
    ),
    (
        "xtask/src/main.rs:Command::DesktopBundleWindows",
        "xtask desktop bundle command delegates validation to typed bundle target helpers",
    ),
    (
        "xtask/src/main.rs:Command::DesktopServe",
        "xtask desktop serve command delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::DistServer",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::Fmt",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::RenderDockerBuild",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::RenderDockerRun",
        "xtask command enum delegates validation to clap and option structs",
    ),
    (
        "xtask/src/main.rs:Command::ServeWebRelease",
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

#[test]
#[requires(true)]
#[ensures(true)]
fn qualified_invariant_attributes_are_audited_by_final_path_segment() {
    let source = concat!(
        "#[invariant(::Unqualified(..) => true)]\n",
        "#[bityzba::invariant(\n",
        "    ::Qualified { .. } => true\n",
        ")]\n",
        "#[bityzba::not_invariant(::NearMiss => true)]\n",
        "enum Example {\n",
        "    Unqualified,\n",
        "    Qualified,\n",
        "    NearMiss,\n",
        "}\n",
    );
    let mut placeholders = BTreeSet::new();

    scan_rust_source(
        Path::new("tests/qualified_invariant_fixture.rs"),
        source,
        &mut placeholders,
    );

    assert_eq!(
        placeholders,
        BTreeSet::from([
            "tests/qualified_invariant_fixture.rs:Example::Qualified".to_owned(),
            "tests/qualified_invariant_fixture.rs:Example::Unqualified".to_owned(),
        ]),
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
    for root in ["crates", "apps", "tests", "xtask", "xtask-common"] {
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
    let relative_path = normalized_source_path(relative_path);
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
                placeholders.insert(format!("{relative_path}:{enum_name}::{variant}"));
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

#[requires(true)]
#[ensures(!ret.contains('\\'))]
fn normalized_source_path(relative_path: &Path) -> String {
    relative_path.to_string_lossy().replace('\\', "/")
}

#[requires(index < lines.len())]
#[ensures(true)]
fn invariant_attribute(lines: &[&str], index: usize) -> Option<(Option<String>, usize)> {
    let line = lines[index].trim();
    if invariant_attribute_arguments(line).is_none() {
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
fn invariant_attribute_arguments(attribute: &str) -> Option<&str> {
    let attribute = attribute.trim().strip_prefix("#[")?;
    let open = attribute.find('(')?;
    let path = attribute[..open].trim();
    if path.rsplit("::").next()?.trim() != "invariant" {
        return None;
    }
    Some(&attribute[open + 1..])
}

#[requires(true)]
#[ensures(true)]
fn placeholder_variant(attribute: &str) -> Option<&str> {
    let rest = invariant_attribute_arguments(attribute)?
        .trim_start()
        .strip_prefix("::")?;
    if !rest.trim_end().ends_with("=> true)]") {
        return None;
    }
    let end = rest
        .char_indices()
        .find(|(_, ch)| !(*ch == '_' || ch.is_ascii_alphanumeric()))
        .map_or(rest.len(), |(index, _)| index);
    (end > 0).then_some(&rest[..end])
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
