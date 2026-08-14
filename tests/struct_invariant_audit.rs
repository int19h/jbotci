use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use walkdir::WalkDir;

const ALLOWED_PLACEHOLDERS: &[(&str, &str)] = &[
    (
        "apps/jbotci-server/src/lib.rs:EmbeddingToolJob",
        "embedding worker jobs pair a typed request with the response channel for that request",
    ),
    (
        "apps/jbotci-server/src/lib.rs:GimfihiSchemaProjection",
        "typed schema test projection delegates validity to its invariant-bearing properties payload",
    ),
    (
        "apps/jbotci-server/src/lib.rs:GimfihiSchemaProperties",
        "typed schema test aggregate delegates constraints to each typed property projection",
    ),
    (
        "apps/jbotci-server/src/lib.rs:HealthResponse",
        "health payload is a fixed transport shape",
    ),
    (
        "apps/jbotci-server/src/lib.rs:SalienceSchemaProperties",
        "typed schema test aggregate contains every independently validated numeric salience property",
    ),
    (
        "apps/jbotci-server/src/lib.rs:ToolServices",
        "tool services owns a channel to the dedicated embedding worker thread",
    ),
    (
        "apps/jbotci-server/src/mcp.rs:JsonRpcMessage",
        "MCP JSON-RPC message DTO intentionally accepts malformed field combinations so protocol errors can be returned",
    ),
    (
        "apps/jbotci/src/benchmark.rs:BenchmarkMeasurement",
        "benchmark measurement is mutable accumulator state bounded by NonZeroUsize and record_iteration contracts",
    ),
    (
        "apps/jbotci/src/benchmark.rs:BenchmarkReport",
        "benchmark report is derived measurement output validated by finish and render contracts",
    ),
    (
        "apps/jbotci/src/benchmark.rs:BenchmarkStatusCounts",
        "benchmark status counts are derived counters updated only from CliStatus",
    ),
    (
        "apps/jbotci/src/benchmark.rs:ProcessResourceDelta",
        "process resource deltas are optional platform measurements with unavailable metrics represented by None",
    ),
    (
        "apps/jbotci/src/benchmark.rs:ProcessResourceUsage",
        "process resource snapshots mirror optional platform APIs with unavailable metrics represented by None",
    ),
    (
        "apps/jbotci/src/benchmark.rs:WallTimeStats",
        "wall-time stats are derived from non-empty iteration measurements by wall_time_stats",
    ),
    (
        "apps/jbotci/src/commands/setup.rs:CliSetupProgressReporter",
        "CLI setup progress reporter owns rendering state derived from the selected progress policy",
    ),
    (
        "apps/jbotci/src/lib.rs:Cli",
        "CLI root delegates input validation to clap",
    ),
    (
        "apps/jbotci/src/lib.rs:CliColorPolicy",
        "resolved color policy is two independent stream decisions",
    ),
    (
        "apps/jbotci/src/lib.rs:CliParsedTraceSpec",
        "trace spec parsing validates level and filter shape before constructing this transport value",
    ),
    (
        "apps/jbotci/src/lib.rs:CliProgressPolicy",
        "CLI progress policy is derived from terminal capability and caller-selected verbosity",
    ),
    (
        "apps/jbotci/src/lib.rs:CliTraceConfig",
        "trace limit is validated once at CLI entry and phase is a closed enum",
    ),
    (
        "apps/jbotci/src/lib.rs:CuktaInput",
        "CLI cukta input delegates raw mode and target validation to validate_cukta_input",
    ),
    (
        "apps/jbotci/src/lib.rs:GentufaInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/lib.rs:GimfihiInput",
        "gimfihi CLI input is raw clap transport state and is validated by gimfihi_request_from_input",
    ),
    (
        "apps/jbotci/src/lib.rs:JvozbaInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/lib.rs:SetupInput",
        "setup CLI input delegates model and directory validation to setup command execution",
    ),
    (
        "apps/jbotci/src/lib.rs:TersmuInput",
        "tersmu CLI input is clap transport state validated by render_tersmu before semantic building",
    ),
    (
        "apps/jbotci/src/lib.rs:TextInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/lib.rs:VlackuInput",
        "custom clap parser preserves ordered request flags and command validation checks mode combinations",
    ),
    (
        "apps/jbotci/src/lib.rs:VlaseiInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/lib.rs:VlataiInput",
        "CLI input selector is validated by clap and vlatai command execution",
    ),
    (
        "apps/jbotci/src/lsp.rs:CompletionCancellationGuard",
        "completion cancellation guard owns one independently valid shared token and cancels it on drop",
    ),
    (
        "apps/jbotci/src/lsp.rs:DocumentState",
        "document state is mutable under the document-store lock and its transition methods enforce text, version, generation, and snapshot coherence",
    ),
    (
        "apps/jbotci/src/lsp.rs:DocumentStore",
        "document store is an unconstrained shared mutex wrapper whose contents are governed by its transition methods",
    ),
    (
        "apps/jbotci/src/lsp.rs:InlayInitializationOptions",
        "kind enablement booleans and the independently validated structure-bracket value form accept every typed combination",
    ),
    (
        "apps/jbotci/src/lsp.rs:ServerInitializationOptions",
        "the optional current and legacy inlay wire shapes are validated for mutual exclusion during initialization",
    ),
    (
        "apps/jbotci/src/lsp.rs:ServerState",
        "LSP adapter state combines independently valid transport, document-store, encoding, capability, and callback values",
    ),
    (
        "apps/jbotci/src/tool.rs:StringEnumTypeTransform",
        "stateless unit transform; all instances are equivalent so there is no invariant to enforce",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolCuktaRequest",
        "shared cukta tool request is API transport state normalized into CuktaInput and validated during execution",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolExecutionContext",
        "tool execution context is constructed only as stateless, borrowed embedding service, or cached embedding error",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGentufaRequest",
        "shared gentufa tool request is API transport state validated by the CLI option validator during execution",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiCommandInput",
        "tool gimfi'i command conversion pairs API transport state with the caller-specific word interpretation mode",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiRequest",
        "shared gimfi'i tool request is API transport state normalized into GimfihiInput and validated during execution",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolGimfihiSource",
        "MCP gimfi'i source carries free-form language/word/weight fields validated downstream by the gimfi'i engine",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaPart",
        "shared jvozba part is API transport state whose value is interpreted according to the closed part-kind enum",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolJvozbaRequest",
        "shared jvozba tool request is API transport state normalized into the CLI source list before composition",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolRenderedOutput",
        "shared tool output is a transport envelope produced by run_tool_command from validated CLI status and byte output",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolTersmuRequest",
        "shared tersmu tool request is API transport state normalized into TersmuInput and validated during execution",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlackuRequest",
        "shared vlacku tool request is API transport state normalized into explicit dictionary search requests",
    ),
    (
        "apps/jbotci/src/tool.rs:ToolVlaseiRequest",
        "shared vlasei tool request is API transport state validated by the CLI option validator during execution",
    ),
    (
        "apps/jbotci/tests/lsp.rs:LspClient",
        "protocol test harness permits stdin to become absent during orderly shutdown while ownership of the remaining child handles stays independent",
    ),
    (
        "apps/jbotci/tests/support/cli.rs:CapturedCliRun",
        "test helper records CLI process output after run_cli returns a status",
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
        "crates/jbotci-cll/src/import.rs:BlockParseState",
        "private CLL block parse state is a monotonically advanced chapter-local counter",
    ),
    (
        "crates/jbotci-cll/src/import.rs:PendingIndexEntry",
        "pending index entries are private loader intermediates from DocBook indexterm nodes",
    ),
    (
        "crates/jbotci-cll/src/import.rs:SectionParseContext",
        "section parse context is private loader state derived from an already parsed section heading",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:BlockPlainTextVisitor",
        "plain text visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:ChrestomathyGroupVisitor",
        "chrestomathy grouping visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllLinkTargetCounts",
        "CLL link target counter is private test traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:InlinePlainTextVisitor",
        "inline plain text visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/links.rs:LinkResolution",
        "link resolutions are private loader intermediates derived from the completed anchor index",
    ),
    (
        "crates/jbotci-cll/src/links.rs:LinkResolutionVisitor",
        "link resolution visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CllSearchChunk",
        "CLL search chunks are generated from parsed sections and tagged-word extraction",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CllSearchMatch",
        "CLL search matches are ranked only by cukta_word_search_matches after target filtering",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CuktaSearchOutput",
        "cukta search output is built by cukta_search from normalized query/count inputs",
    ),
    (
        "crates/jbotci-cll/src/search.rs:CuktaTargetFilter",
        "target filters intentionally preserve all checkbox states before validation/defaulting",
    ),
    (
        "crates/jbotci-cll/src/search.rs:SearchChunkVisitor",
        "search chunk visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-cll/src/search.rs:TaggedWordsVisitor",
        "tagged words visitor is private traversal accumulator state",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:TraceFailureBranch",
        "branch context and expectation payloads are collected from structured parser metadata",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:TraceRecorderState",
        "recorder state is deliberately mutable; public recorder methods enforce event and limit invariants",
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
        "crates/jbotci-dialect/src/lib.rs:CustomDialect",
        "custom dialect definitions are parsed and normalized through dialect resolution helpers",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectDefinition",
        "dialect word entry validity is enforced by CmavoDialectEntry and feature validity by the closed DialectFeature enum",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectError",
        "diagnostic struct carries a human-readable error message",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:DialectSettings",
        "dialect settings are persisted transport state normalized by import/export helpers",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:JohauShorthandSwap",
        "JOHAU shorthand swap records are static internal mappings with closed code and atom fields",
    ),
    (
        "crates/jbotci-dictionary-data/build.rs:DictionaryMetadata",
        "vendored Lensisku metadata is validated against dictionary contents and hashes in the build script",
    ),
    (
        "crates/jbotci-dictionary-data/build.rs:GeneratedLujvoEntry",
        "generated lujvo entries are build-script intermediates created from morphology-backed decomposition and checked through Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary-data/build.rs:GeneratedLujvoSegment",
        "generated lujvo segments are build-script intermediates created from validated morphology parts and checked through Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary-data/build.rs:GeneratedSoundEntry",
        "generated sound entries are build-script intermediates created from standard IPA tokenization and checked through Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary-data/src/lib.rs:DictionarySnapshotMetadata",
        "embedded snapshot metadata is generated from validated build metadata and checked by dictionary-data tests",
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
        "crates/jbotci-dictionary/src/lib.rs:DefinitionId",
        "Lensisku definition ids are opaque upstream identifiers",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:Dictionary",
        "dictionary-wide validity is checked by validate and the expensive impl invariant",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryEntry",
        "dictionary entry field consistency is checked by Dictionary::validate",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryLujvoEntry",
        "borrowed lujvo index entries are static generated data validated against dictionary entries and segment structure",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryLujvoSegment",
        "borrowed lujvo index segments are static generated data validated as part of the dictionary-wide lujvo index",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionaryPatternEntry",
        "borrowed pattern index entries are static generated data validated against dictionary entries and normalized key generation",
    ),
    (
        "crates/jbotci-dictionary/src/lib.rs:DictionarySoundEntry",
        "borrowed sound index entries are static generated data validated against dictionary entries and token tables",
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
        "crates/jbotci-dictionary/src/lib.rs:OwnedPatternIndexEntry",
        "owned pattern index entries are produced by build_owned_indexes",
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
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputCorpus",
        "browser/native embedding corpus DTO is generated from embedded dictionary and CLL data immediately before JSON serialization",
    ),
    (
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputCorpusDto",
        "raw corpus deserialization accepts all field combinations so conversion can return precise typed validation errors",
    ),
    (
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputDocument",
        "browser/native embedding document DTO is generated from v0-parity embedding input builders",
    ),
    (
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputDocumentDto",
        "raw document deserialization accepts all field combinations so conversion can recompute and validate every fingerprint",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:CllEmbeddingItem",
        "CLL embedding item rows are generated from embedded CLL search chunk order",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:CorpusManifest",
        "corpus manifests are generated from validated item files and vector shards",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:DictionaryEmbeddingItem",
        "dictionary embedding item rows are generated from embedded dictionary entry order",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:DictionarySemanticHit",
        "dictionary semantic hits are produced by joining vector hits to generated item rows",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingBuildRow",
        "embedding build rows are constructed immediately from hashed corpus inputs and consumed within one pack build",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingCatalog",
        "embedding catalog is a static transport manifest written by setup",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingCatalogModel",
        "embedding catalog model rows are written by setup after pack validation",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingModelSpec",
        "embedding model specs are fixed catalog records created by model_spec",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingPackManifest",
        "embedding pack manifests are generated after all corpus shards are written and validated",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingRuntime",
        "embedding runtime entries are fixed manifest transport metadata",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:FailingBackend",
        "failing backend is a test-only fixture whose fields are controlled by individual tests",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:FakeBackend",
        "test fake backend is constrained by test construction and used only for fixture packs",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:LoadedCorpusCacheKey",
        "loaded corpus cache keys are assembled from validated manifest and shard metadata before lookup",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:NativeF2LlmModel",
        "native F2LLM model rows are fixed catalog entries projected into EmbeddingModelSpec",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:NativePartialBuildCheckpoint",
        "native partial checkpoint compatibility is validated against model and corpus metadata before reuse",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:NativePartialCorpus",
        "native partial corpus state is a resumable checkpoint DTO validated with its checkpoint before reuse",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:NativePartialShard",
        "native partial shard metadata is validated against shard files and expected row ranges before reuse",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:QueryEmbedding",
        "query embeddings are produced by backend implementations and normalized before search",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:ReusablePackRows",
        "native incremental rebuild cache is loaded only from a compatible previously validated pack",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:ReusableVectorRows",
        "native incremental rebuild rows are loaded from a previously validated pack and keyed by stored input hashes",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupOptions",
        "embedding setup options are validated by model lookup and path resolution",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgress",
        "embedding setup progress is transport state produced by setup phases and consumed for display",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupReport",
        "embedding setup reports are returned only after pack construction or validated reuse",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:VectorHit",
        "vector hits are produced by bounded vector ranking over validated row-major matrices",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:VectorShardManifest",
        "vector shard manifests are generated from written shard files and SHA-256 checks",
    ),
    (
        "crates/jbotci-embeddings/src/native.rs:NativeEmbeddingSearchService",
        "native embedding search service owns validated manifest and backend state from setup",
    ),
    (
        "crates/jbotci-embeddings/src/native.rs:NativeLlamaEmbeddingBackend",
        "native backend fields are produced by llama.cpp model/context initialization",
    ),
    (
        "crates/jbotci-embeddings/src/native.rs:OwnedLlamaContext",
        "owned llama context validity is enforced by its private constructor, drop order, and unsafe lifetime safety contract",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/artifact.rs:ObjectSafeSources",
        "unit test source has no state; it exists only to prove the two async source traits remain separately object safe",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/core.rs:SpecialTokens",
        "F2LLM special token ids are external tokenizer metadata interpreted by the tokenizer",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/core.rs:TokenizerArtifact",
        "F2LLM tokenizer artifacts are external manifest DTOs validated while loading the runtime",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/progress.rs:ObjectSafeProgressSink",
        "unit test progress sink has no state; it exists only to prove async fallible progress remains object safe",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:GpuErrorScopes",
        "WebGPU error-scope guards are opaque RAII tokens with validity enforced by wgpu",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:VectorBuffer",
        "WebGPU vector buffers are constructed by runtime buffer allocation helpers",
    ),
    (
        "crates/jbotci-f2llm-runtime/src/webgpu.rs:WebGpuRuntime",
        "WebGPU runtime owns mutable caches and transient buffers whose validity is maintained by its fallible methods",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockCollapseFrame",
        "private block-collapse traversal frame; stack sequencing, not the field tuple itself, enforces traversal balance",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockTemp",
        "temporary block color state is consumed inside the layout builder before transport output",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockCollector",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedBlockPayload",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedFieldFrame",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GeneratedNodeFrame",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GentufaBlockAnnotation",
        "block annotations are projected from dictionary search results before layout decoration",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GentufaBlockOptions",
        "block options are independent presentation controls with typed phoneme rendering options",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceMarker",
        "reference markers are renderer annotations derived from semantic reference analysis",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:EmbeddedGentufaFonts",
        "embedded font provider is a zero-sized access point for compile-time font bytes",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaFontData",
        "font byte slices are supplied by embedded native assets or validated browser fetches",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaSvgOptions",
        "SVG options are independent presentation controls with a caller-provided title",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:ReferenceStackBottoms",
        "reference stack bottoms are derived renderer layout measurements checked by reference sizing tests",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:SvgAttribute",
        "typed SVG DOM attributes are escaped during serialization before parser handoff",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:SvgDocument",
        "typed SVG document validity is delegated to the root element and parser round-trip tests",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:SvgElement",
        "typed SVG element validity is constrained by the closed SvgTag enum and serializer tests",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:TextMeasureStyleKey",
        "text measurement cache keys are direct value tuples over closed role/script selectors",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:TextMeasurer",
        "text measurer owns a usvg font database and cache populated through measurement calls",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:TextSize",
        "text sizes are produced by usvg bounding boxes and checked by focused renderer tests",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiCandidate",
        "gimfihi candidates are generated only after morphology validation and scoring",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiOutput",
        "gimfihi output is assembled by compose_gismu from validated candidates and sources",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiRequest",
        "gimfihi request is a CLI/web transport envelope validated by compose_gismu",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GimfihiSourceInput",
        "gimfihi source input is raw CLI/web transport state validated by resolve_sources",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GismuCollision",
        "collision payloads are produced by dictionary-backed collision checks",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:GismuScoreScratch",
        "gimfihi scoring scratch is mutable work storage rebuilt by each scoring helper before use",
    ),
    (
        "crates/jbotci-gimfihi/src/lib.rs:LcsScratch",
        "LCS scratch rows are mutable work storage whose temporary lengths are controlled by longest_common_subsequence_len_chars",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion.rs:CompletionCancellationToken",
        "completion cancellation token is a shared atomic flag whose only valid state transition is enforced by cancel",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion.rs:CompletionDocumentWordCollector",
        "private TreeVisitor accumulator whose word-set and current-word relationship is enforced by record and completion_document_words contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:NodeFrame",
        "private mutable traversal frame; node balance and constructor validity are enforced by TreeContextCollector enter and exit contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:RecoveredSkippedTokenVisitor",
        "private recursive-walker accumulator; text-depth balance is enforced by walk_text and the generated walker",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:RecoveredTreeContextVisitor",
        "private TreeVisitor adapter whose state validity is delegated to TreeContextCollector traversal contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:SequenceFrame",
        "private mutable traversal frame; suffix-token state and bounds are populated atomically by TreeContextCollector",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:SkippedAnchor",
        "private derived anchor record produced only after a skipped-token span and restart point have both been observed",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:SkippedTextFrame",
        "private skipped-token traversal frame whose temporary boundary state is normalized before an anchor is emitted",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:TextFrame",
        "private mutable traversal frame; nesting depth and statement starts are assigned by ordered tree events",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:TextRecord",
        "private completed traversal record derived from a bounded text node before restart selection",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:TreeContextCollector",
        "private TreeVisitor accumulator; cut bounds and stack balance are enforced by its constructor and traversal method contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/completion/tree_context.rs:ValidTreeContextVisitor",
        "private TreeVisitor adapter whose state validity is delegated to TreeContextCollector traversal contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/incremental_diagnostics.rs:RecoveredTokenCollector",
        "private TreeVisitor token accumulator whose per-visit growth and final source ordering are enforced by traversal and confirmed_tree_tokens contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/incremental_diagnostics.rs:ValidTokenCollector",
        "private TreeVisitor token accumulator whose per-visit growth and final source ordering are enforced by traversal and confirmed_tree_tokens contracts",
    ),
    (
        "crates/jbotci-ide/src/snapshot/inlays.rs:InlayOptions",
        "the three kind toggles and independently validated structure profile accept every typed combination",
    ),
    (
        "crates/jbotci-ide/src/snapshot/inlays.rs:StructureBracketInlayOptions",
        "structure enablement and the independently validated decoration profile accept every typed combination",
    ),
    (
        "crates/jbotci-ide/src/snapshot/structure_inlays.rs:RawBracketsOptions",
        "raw-brackets depth and construct-filter settings are independent and every typed combination is valid",
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
        "crates/jbotci-morphology/src/lib.rs:MorphologyOptions",
        "compiled dialect entry validity is enforced by CompiledDialectDefinition and other fields are independent parser options",
    ),
    (
        "crates/jbotci-morphology/src/lib.rs:PhonemeRenderOptions",
        "render options are independent booleans with no cross-field invariant",
    ),
    (
        "crates/jbotci-morphology/src/segment.rs:LujvoParseFailure",
        "private lujvo parse failure records the furthest parse position with a closed expectation enum",
    ),
    (
        "crates/jbotci-morphology/src/segment.rs:NormalizationError",
        "normalization error records an arbitrary rejected source character and its source index",
    ),
    (
        "crates/jbotci-morphology/src/segment.rs:PronunciationChar",
        "pronunciation annotation pairs original and derived chars inside the strict syllabifier; constructors enforce valid annotated values",
    ),
    (
        "crates/jbotci-orthography/src/lib.rs:NormalizedLatinChar",
        "orthography conversion helper stores a normalized character plus stress flag",
    ),
    (
        "crates/jbotci-output/src/brackets.rs:BracketContext",
        "render context borrows source text and options without extra state rules",
    ),
    (
        "crates/jbotci-output/src/brackets.rs:GeneratedBracketFrame",
        "generated bracket rendering uses this as a mutable traversal stack frame; S-expression shape is normalized by sexpr::node and final rendering tests",
    ),
    (
        "crates/jbotci-output/src/brackets.rs:GeneratedBracketVisitor",
        "generated bracket rendering uses this as a mutable TreeVisitor accumulator; stack/root balance is controlled by TreeVisitor enter/exit calls and output tests",
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
        "crates/jbotci-output/src/qr_code.rs:QrBuild",
        "QR build state is internal renderer assembly data validated by encoded-output tests",
    ),
    (
        "crates/jbotci-output/src/qr_code.rs:QrCoord",
        "QR coordinates are internal renderer grid positions bounded by placement code",
    ),
    (
        "crates/jbotci-output/src/qr_code.rs:QrLogoLayer",
        "QR logo layers are derived renderer masks covered by logo placement tests",
    ),
    (
        "crates/jbotci-output/src/recovered.rs:RecoveredBracketBuilder",
        "mutable visitor state is governed by enter and exit method contracts during traversal",
    ),
    (
        "crates/jbotci-output/src/recovered.rs:RecoveredBracketFrame",
        "private mutable traversal frame accumulates children until pop while enter and exit methods govern its lifecycle",
    ),
    (
        "crates/jbotci-output/src/references.rs:GeneratedSyntaxWordCollector",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceAnnotations",
        "annotation vectors are sorted/deduplicated projections from ReferenceDisplayModel",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceDisplayModel",
        "display model maps syntax ids to renderer annotations derived from semantic reference analysis",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceName",
        "reference name components are assembled by renderer naming logic and validated by focused tests",
    ),
    (
        "crates/jbotci-output/src/references.rs:ReferenceSource",
        "source metadata is an intermediate projection from syntax ids and rendered words",
    ),
    (
        "crates/jbotci-output/src/references.rs:TreeWordLabel",
        "word labels are copied from already rendered tree word values",
    ),
    (
        "crates/jbotci-output/src/sexpr.rs:FlattenFrame",
        "private S-expression flatten traversal frame; stack sequencing controls pending and flattened child flow",
    ),
    (
        "crates/jbotci-output/src/trace.rs:TraceRenderOptions",
        "trace rendering now only carries a color flag, so all values are valid",
    ),
    (
        "crates/jbotci-output/src/tree.rs:CollapseFrame",
        "private tree-collapse traversal frame; stack sequencing controls pending and collapsed child flow",
    ),
    (
        "crates/jbotci-output/src/tree.rs:GeneratedReferenceDisplay",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-output/src/tree.rs:GeneratedStatementConnectionPart",
        "temporary generated-model tree projection aggregate whose fields are independently rendered values",
    ),
    (
        "crates/jbotci-output/src/tree.rs:GeneratedSyntaxRenderModel",
        "stateless render-model marker type; all instances are equivalent",
    ),
    (
        "crates/jbotci-output/src/tree.rs:GeneratedSyntaxTokenTreeValueCollector",
        "visitor accumulates rendered generated token values for a borrowed source and every accumulated prefix is valid",
    ),
    (
        "crates/jbotci-output/src/tree.rs:MorphologyTreeBuilder",
        "builder validity is governed by traversal enter/exit sequencing",
    ),
    (
        "crates/jbotci-output/src/tree.rs:RawGeneratedSyntaxRenderModel",
        "generated syntax render model is a zero-sized marker type with no state to validate",
    ),
    (
        "crates/jbotci-output/src/tree.rs:RecoveredSyntaxRenderModel",
        "recovered syntax render model is a zero-sized marker type with no state to validate",
    ),
    (
        "crates/jbotci-output/src/tree.rs:RenderedPosition",
        "rendered syntax token end positions are copied directly from validated source spans",
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
        "crates/jbotci-phonetic/src/lib.rs:AlineFeatures",
        "ALINE feature vectors are derived from a fixed IPA segment table",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:AlineSimilarityScratch",
        "ALINE scratch rows are resized before scoring and arbitrary cached buffer contents are valid",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:IpaRenderedWord",
        "IPA word rendering metadata is produced by render_word_ipa and consumed immediately for boundary merging",
    ),
    (
        "crates/jbotci-phonetic/src/lib.rs:IpaTokenizedText",
        "tokenized IPA text is assembled from tokenizer output and consumed as an internal paired return value",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobPattern",
        "glob patterns are constructed only by compile_glob_pattern after token validation",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionPiece",
        "composition pieces are projected from validated morphology decomposition segments",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuSearchOutput",
        "lookup execution owns card, diagnostic, and worst-outcome aggregation semantics",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:WordClassification",
        "word classifications are produced from morphology segmentation of a single word-like token",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:BundlePaths",
        "offline bundle and generated-Rust paths are caller-selected and every I/O failure is returned as a typed error",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:BundleSnapshot",
        "raw snapshot bytes intentionally admit invalid mutations so the verifier can reject every byte, schema, and manifest failure class",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:DictionaryIdentity",
        "private dictionary identity is emitted only by the pinned whole-file audit and checked against curated rows before use",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:DispositionSeed",
        "raw mechanical ledger projection intentionally reaches the generator validation boundary before becoming a normative DispositionRow",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:GeneratedRelationSource",
        "raw TOML source DTO intentionally admits invalid combinations so mint validation returns a typed rejection",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:LexicalSource",
        "raw TOML source DTO intentionally admits invalid identities, types, and arities for fail-closed mint validation",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:PlaceDeletionSource",
        "raw TOML source DTO is validated against lexical rows, total surviving labels, and evidence before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:PreludeSource",
        "raw TOML prelude DTO is parsed, typechecked, dependency-checked, and digest-checked before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:RegistryProvenance",
        "raw provenance-sidecar DTO intentionally admits drift so the exact authority audit can return a typed rejection",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:RegistrySource",
        "raw top-level TOML DTO is an unchecked staging aggregate whose complete tables are validated before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:RelationFormerSource",
        "raw TOML relation-former DTO is template-typechecked and checked for total provenance before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:ScaleLiteralSource",
        "raw TOML scale DTO is checked for a closed type and nonempty source coverage before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:ScopePolicyProvenance",
        "raw provenance row is compared field-for-field with the reviewed policy source before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:ScopePolicySource",
        "raw TOML policy DTO is range-, foreign-key-, and reviewed-provenance-checked before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:StaticTypeRegistry",
        "private checker maps are constructed only from validated lexical and prelude rows and never cross the mint boundary",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:Tables",
        "private staged table aggregate intentionally exists before cross-table order, key, evidence, and semantic validation",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs:TagReductionSource",
        "raw TOML tag DTO is Hole-validated, typechecked, and graph-identity-checked before minting",
    ),
    (
        "crates/jbotci-semantics/codegen/smusni_v0_surface.rs:SerdeContainerOptions",
        "private serde attribute scan state admits every independently optional tag/content observation",
    ),
    (
        "crates/jbotci-semantics/examples/smusni_corpus_report.rs:CorpusReport",
        "corpus measurements are mutable derived counters; measure and record_success enforce their transitions",
    ),
    (
        "crates/jbotci-semantics/src/completeness/model.rs:CompletenessContract",
        "a disposition map with no cross-field invariant; every entry-key to disposition mapping is valid",
    ),
    (
        "crates/jbotci-semantics/src/facade.rs:SemanticBuildOptions",
        "semantic build options are caller transport state with no invalid combination beyond lifetimes",
    ),
    (
        "crates/jbotci-semantics/src/facade.rs:SemanticsError",
        "semantic errors are produced by constructors that attach nonempty diagnostic messages",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/connectives.rs:GeneratedIndicatorCmavoVisitor",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedAlternativeArgument",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierBundleScope",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedArgumentQuantifierScope",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedBuiltParagraphBoundary",
        "the item index accepts every usize and Vec1 makes the paragraph marker list nonempty by construction",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDaSeriesScopeBinding",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedDescriptionAbstraction",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedForethoughtPrefixContext",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedForethoughtSelbriInspector",
        "tree-walker discovery state is a boolean for which both states are valid",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedGraphBuilder",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedLogicalSumtiConnection",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexFormulaScopeGroup",
        "private prenex scope accumulator is validated when the balanced group stack is coequalized and finalized",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPrenexTermCollector",
        "private mutable tree-walker state delegates syntax validity to borrowed nodes and validates balanced events when finalized",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedArgumentFormulaScope",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedPreparedArgumentQuantifierBundleScope",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedRecurrenceEventModifiers",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedScopedFormula",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedSemanticDaSeriesScopeBinding",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedStatementConnectionTail",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedStickyEventUpdate",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTermAssignments",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:GeneratedTextPlan",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:IndicatorBaseSpec",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:IndicatorDisplayDraft",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/mod.rs:IndicatorPart",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/pro_bridi.rs:GeneratedTanruUnitCollector",
        "tree-walker accumulator may validly contain any ordered prefix of borrowed invariant-bearing tanru units",
    ),
    (
        "crates/jbotci-semantics/src/generated_builder/sources.rs:GeneratedSpanCollector",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:Actuality",
        "actuality is a single closed enum field",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:DeicticGround",
        "deictic ground is assembled by the utterance constructor from fixed special referents",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:GeneratedReferent",
        "both fields are single-variant enums, so the struct has exactly one valid inhabitant",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:IntervalEndpointInclusion",
        "interval endpoint inclusion is a pair of closed endpoint enum values",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:PlaceDescription",
        "relation place descriptions are reserved metadata DTOs not externally constructed yet",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:PlaceIndex",
        "NonZeroUsize owns the one-based place-index invariant and constructors enforce xN boundaries",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:RelationExpansion",
        "relation expansion DTOs are reserved metadata fields not externally constructed yet",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticDiagnostic",
        "semantic diagnostics are produced by constructors with nonempty messages",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SemanticSource",
        "semantic source DTOs are projected from validated source spans by source_from_spans",
    ),
    (
        "crates/jbotci-semantics/src/model.rs:SourceByteSpan",
        "source byte spans are projected from validated SourceSpan values",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:FunctionSignature",
        "every ordered validated parameter sequence and validated result form a valid function signature",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/apply.rs:PredicateSignature",
        "every validated relation identity and canonical effective row form a valid predicate signature",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/binder.rs:TypedParameter",
        "a lambda parameter is any variable/type pair; distinctness is the enclosing lambda's invariant",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/document.rs:ScopeAudit",
        "the scope walk's environment is whatever the current position makes live; the walk itself is the check, and its result is a census or the first failure",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:DeReHostRegion",
        "every region id is a legal field value; what makes one a de-re host is that its fallible constructor is the only way to obtain the type",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:RaisedBinding",
        "the three components are exactly the ones a validated Bind was taken apart into, and rebinding them revalidates through Bind::new",
    ),
    (
        "crates/jbotci-semantics/src/notation/kernel/types.rs:RowSlot",
        "every positive or distinguished label paired with a validated accepted type is a valid row slot",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:GeneratedDispositionRow",
        "unchecked generated constants are accepted only through DispositionRegistry::try_from_generated",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:GeneratedLexicalPolicyRow",
        "private generated lexical rows contain build-time-audited constants and closed enum values",
    ),
    (
        "crates/jbotci-semantics/src/notation/registry.rs:GeneratedProjectionFailureReasonRow",
        "unchecked generated reason constants are joined and validated only by DispositionRegistry::try_from_generated",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs:Parser",
        "private mutable parser state starts at byte zero and cursor-mutating methods contractually preserve UTF-8 boundaries",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:CompactFallback",
        "one failed projection edge pairs any graph identity with any declining boundary; every combination names a real edge",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:CompactFallbackLog",
        "the ordered, deduplicated per-edge channel is a BTreeSet property rather than a state constraint, and every set of edges is a valid log",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:ElaborationCounters",
        "private mutable counter state is updated only by recognition and fallback helpers with transition postconditions",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:Elaborator",
        "private read-only graph context owns mutable traversal registries whose lifecycle is enforced by rendering helper contracts",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:HostFrame",
        "private mutable accumulator for one open host position; the boundary kind and its live binders are independently valid, and the one real constraint (one identity is one binder) is the postcondition of HostFrame::bind",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/elaborate.rs:ReferenceBinding",
        "graph-owned identity, parsed declared type, and rendered computation are independently valid before Bind wrapping",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:LocalFallback",
        "the expected type, registered fallback reason, and identity-checked raw tree are independently validated",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:ObjectId",
        "PositiveInteger provides canonical arbitrary-precision positivity for every fallback object identity",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawField",
        "every NFC field name and validated raw value form a valid raw field",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawMapEntry",
        "every pair of recursively validated raw values is a valid typed raw-map entry",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawObject",
        "the identity, NFC type name, and raw fields are locally valid; RawTree proves global identity order",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawRecord",
        "every NFC model type name and sequence of recursively validated raw fields is a valid inline record",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:RawVariant",
        "every NFC enum/constructor identity and sequence of recursively validated raw fields is a valid inline variant",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/internal_raw.rs:TypedGraph",
        "the NFC model-root name and identity-checked RawTree are independently valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/planner.rs:ReferencePlan",
        "private planner output is constructed only by plan_references; partial analyses are hidden and complete-only queries require compact eligibility",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/planner.rs:ProjectedIdentities",
        "the pre-scan reports which identities the renderer owns; any set of graph identities, including none, is a possible answer",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/planner.rs:ScopeFailure",
        "a closed failure kind plus optional typed binder and use-site evidence admits every typed combination",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/purity.rs:PuritySummary",
        "the section-3.2 summary is the product of three closed refinement coordinates, and every coordinate combination is a valid conservative summary",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/purity.rs:SummaryEnvironment",
        "the lexical summary environment maps validated variables to validated kernel operands; every such finite map is valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/renderer.rs:SpanResolver",
        "the resolver is a private derived index over one already validated graph, so every field combination it can build is valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:MapBuilder",
        "private Serde map callback state permits both the between-entry and pending-key phases, with sequencing enforced by method contracts",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:SequenceBuilder",
        "every sequence of already validated structural values is a valid in-progress Serde sequence",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructBuilder",
        "private mutable Serde struct state is populated from static derived type and field names and validated when finalized",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructVariantBuilder",
        "private mutable Serde variant state is populated from static derived type, variant, and field names and validated when finalized",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:StructuralSerializer",
        "stateless unit serializer has exactly one valid value",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:TupleStructBuilder",
        "private mutable Serde tuple-struct state is populated from a static derived type name and validated when finalized",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/structural.rs:TupleVariantBuilder",
        "private mutable Serde tuple-variant state is populated from static derived type and variant names and validated when finalized",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:BindForm",
        "ValueBinding fixes the name to a validated variable and both binding and body are independently valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:Declaration",
        "every validated variable and type pair is a valid typed declaration",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:LetForm",
        "LetBinding proves the closed let-name alternatives and the body is independently validated",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:PreludeBinding",
        "PreludeName proves registry membership and every validated type/expression pair is a valid initializer",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:V0Document",
        "every validated body and optional sequence of validated word cards is a valid packaging value",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:ValueBinding",
        "every validated variable declaration and initializer expression is a valid variable binding",
    ),
    (
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs:WordCard",
        "LexicalRoot and NfcText independently prove both word-card fields",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:LexicalPolicyKey",
        "every combination of normalized relation, nonzero place, and closed dynamic family is a valid lookup key",
    ),
    (
        "crates/jbotci-semantics/src/notation/lexical_edge.rs:PreHostLexicalIr",
        "every ordered sequence of independently validated pre-host candidates is valid",
    ),
    (
        "crates/jbotci-semantics/src/notation/word_cards.rs:WordCardBuilder",
        "word-card assembly accumulator is deliberately mutable; the complete cards/built_ids discipline (registry is exactly the card-id set, card ids pairwise distinct) is enforced by cheap and expensive postconditions on every mutating method",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:RenderState",
        "XML render state is deliberately mutable; balanced scope and definition transitions are enforced by method contracts and assertions",
    ),
    (
        "crates/jbotci-semantics/src/notation/xml.rs:XmlElement",
        "private mutable XML construction state is constrained by its constructors and canonical serializer rather than a validated wrapper",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:AbstractionNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:BridiNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:BridiTailNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:CeiAssignmentSource",
        "CEI assignment sources pair a closed CEI label with a syntax node id during validated traversal",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:DiscourseReferences",
        "reference facts are produced by the discourse traversal and exposed as an ordered edge slice",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixturePlaceFrame",
        "fixture frame records are stable projections of typed place frame facts",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureReferenceEdge",
        "fixture reference records are stable projections of typed discourse reference facts",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureSelbriPlace",
        "fixture relation-place records are stable projections of typed place assignments",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureSpanKey",
        "fixture span keys are derived from syntax source spans for expectation output only",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureSumtiAssignment",
        "fixture assignment records are stable projections of typed reference analysis facts",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FreeModifierNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedBridiTailAnalysis",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedDiscourseReferenceBuilder",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedIndexedSyntaxNode",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedPlaceAnalysisBuilder",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedPrenexCeiAssignmentSourceCollector",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedPrenexRelationVariableBindingCollector",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedReferenceAnalysis",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedSyntaxIndex",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:GeneratedSyntaxIndexBuilder",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:MeksoNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:MeksoOperatorNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:NodeMention",
        "node mention validity is maintained by discourse traversal and resolved through SyntaxIndex ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ParagraphNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceAnalysis",
        "place-analysis map consistency is produced by PlaceAnalysisBuilder and exposed through typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceCursor",
        "place cursor is private traversal state initialized by constructors that choose the first numbered slot",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:RawSyntaxNodeId",
        "raw syntax node ids are opaque SyntaxIndex keys whose bounds are checked by node lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceEdge",
        "reference edges are typed traversal facts whose id/index consistency is maintained by the builder",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceEdgeId",
        "reference edge ids are assigned by the builder and carried by ReferenceEdge facts",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceFixtureProjection",
        "fixture projection is a sorted serialization aggregate derived from ReferenceAnalysis",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SelbriNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SelbriPlaceFrameId",
        "place frame ids are opaque PlaceAnalysis keys whose bounds are checked by frame lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:StatementNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiMention",
        "argument mention validity is maintained by discourse traversal and resolved through SyntaxIndex ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiPlaceAssignmentId",
        "assignment ids are opaque PlaceAnalysis keys whose bounds are checked by assignment lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:TanruUnitNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:TermNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:TextNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/tests/support/schema_scan.rs:EnumDef",
        "test-only parsed enum record; any combination of rename/tagging/variants is a valid parse result",
    ),
    (
        "crates/jbotci-semantics/tests/support/schema_scan.rs:Field",
        "test-only parsed field record whose fields are populated valid by the source scanner",
    ),
    (
        "crates/jbotci-semantics/tests/support/schema_scan.rs:Model",
        "test-only parsed-model aggregate; any parsed structs/enums/node-keys are a valid scan result",
    ),
    (
        "crates/jbotci-semantics/tests/support/schema_scan.rs:SerializedSurface",
        "test-only classified surface aggregate; any value-struct/enum/node-key maps are a valid derivation",
    ),
    (
        "crates/jbotci-semantics/tests/support/schema_scan.rs:Variant",
        "test-only parsed enum-variant record whose fields are populated valid by the source scanner",
    ),
    (
        "crates/jbotci-semantics/tests/support/type_graph.rs:Resolved",
        "test-only resolution result whose owner/key are populated valid by the resolver",
    ),
    (
        "crates/jbotci-semantics/tests/support/type_graph.rs:TypeGraph",
        "test-only edge/field maps derived from the source scan; any consistent maps are a valid graph",
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
        "crates/jbotci-syntax-macros/src/lib.rs:AliasRule",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:ChainExpr",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:Condition",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:EnumBranch",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:GeneratedStructModel",
        "syntax macro generated model state is assembled from typed grammar metadata",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:GeneratedTreeModel",
        "syntax macro generated model state is assembled from typed grammar metadata",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:GrammarTypeEnv",
        "syntax macro generation environment is assembled from typed grammar metadata",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:NodeRule",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecoveredParserGeneration",
        "recovered parser generation state is built by fallible grammar analysis before code emission",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:RecursiveRule",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:StrictParserGeneration",
        "strict parser generation state is built by fallible grammar analysis before code emission",
    ),
    (
        "crates/jbotci-syntax-macros/src/lib.rs:SyntaxGrammar",
        "syntax macro parser AST delegates validity to typed syn and grammar payloads",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_mex.rs:BaselineMexRejection",
        "zero-sized grammar refinement policy has no independently invalid state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_selbri.rs:BaselineSelbriAssignmentRejection",
        "zero-sized grammar refinement policy has no independently invalid state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_selbri.rs:C4NodeVisitor",
        "tree-visitor discovery state is a boolean for which both states are valid",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_selbri.rs:RecoveredC4NodeVisitor",
        "recovered-tree visitor discovery state is a boolean for which both states are valid",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_selbri.rs:RestrictedBaselineSelbriAssignmentRejection",
        "zero-sized grammar refinement policy has no independently invalid state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_relative.rs:BaselineRelativeContinuationRejection",
        "zero-sized whole-candidate classification policy has no independently invalid state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/generated_runtime.rs:SyntaxGrammarDialect",
        "generated grammar dialect flags are independent booleans projected from ParseOptions",
    ),
    (
        "crates/jbotci-syntax/src/grammar/generated_runtime.rs:SyntaxGrammarEnv",
        "generated grammar environment pairs independent dialect and policy snapshots",
    ),
    (
        "crates/jbotci-syntax/src/grammar/generated_runtime.rs:SyntaxGrammarPolicy",
        "generated grammar policy flags are independent parser behavior switches",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ChildRecoveryCheckpointObservations",
        "a valid observation range and shared validated observation node are independently valid child-frame data",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:GeneratedModelNoopVisitor",
        "stateless generated-model validation visitor; all instances are equivalent",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserCheckpoint",
        "checkpoint mirrors parser-core save state with warning count plus whether trace would record the save",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserState",
        "mutable parser inspector state uses impl invariants for parser-location and memo-key relationships",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserStateFinish",
        "parser finish value carries deduplicated warnings and optional trace report from ParserState",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryCheckpointCollection",
        "mutable checkpoint arena state uses impl invariants for arena, identity, observation, and replay-node relationships",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryCheckpointId",
        "checkpoint identities are private arena indices whose bounds are enforced by collection methods",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryCheckpointIndex",
        "every private site-to-field threshold map is a valid existence-query index",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryReachabilityKindTelemetry",
        "raw event counters deliberately admit every intermediate combination while telemetry is accumulated",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:RecoveryReachabilityTelemetry",
        "local and boundary-resync telemetry are independent invariant-bearing counter snapshots",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxDiagnosticObservationId",
        "nonzero trial and frame components make every identity pair structurally valid",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxMemoReplayEffects",
        "memo replay position and side effects are independently valid after typed output validation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxMemoRuleFrame",
        "rule frames intentionally represent every partial accumulation and finalization stage during parser descent",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxMemoSideEffects",
        "warning and diagnostic observation snapshots are independent replay payloads",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxMemoSuccessHit",
        "memo lookup returns a valid stored success plus its observed trial-sensitivity classification",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxMemoValue",
        "syntax memo values are type-erased parser outputs validated by typed downcast on lookup",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxRecoveryMemoSession",
        "private trial allocation methods maintain the monotonic identity counter over the shared store",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxRecoveryMemoStore",
        "private memo APIs maintain key, observation-node, and sensitivity-cache relationships",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxRecoveryMemoTrial",
        "a nonzero trial identity and shared memo store are independently valid components",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:SyntaxRuleObservationNode",
        "observation child indices are finalized and checked by the enclosing memo store APIs",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:BaselineTagRejection",
        "stateless parser rejection policy has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:ClassifiedAtom",
        "the independent prefix flags and closed atom kind deliberately admit every typed combination",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:NonElidedNaheFihoTagTermRejection",
        "stateless parser rejection policy has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:PostNaExtensionTagRejection",
        "stateless parser rejection policy has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/baseline_tag.rs:ZantufaTagRejection",
        "stateless parser rejection policy has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SharedStackIter",
        "an optional borrowed persistent-stack node is the iterator's complete valid state",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SharedStackNode",
        "every typed value and optional persistent parent combination is a valid stack node",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SharedVec",
        "empty and shared copy-on-write vectors are both valid; Some(empty) has the same semantics as the allocation-free None representation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SyntaxParseError",
        "lifetime-bearing parser error preserves invariants through constructors and merge helpers",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SyntaxParseErrorData",
        "copy-on-write parser error payload uses private construction and mutation paths over invariant-bearing fields",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Boxed",
        "type-erased parser storage is valid for every parser admitted by its constructor bounds",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Checkpoint",
        "cursor lifetime and inspector snapshot validity are enforced by their component types",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Cursor",
        "parse invocation identity is carried entirely by invariant lifetime markers",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Custom",
        "custom parser validity is fully expressed by the callback bound on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:DropProbe",
        "unit test drop probe has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Empty",
        "unit parser has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:End",
        "unit parser has exactly one valid value",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Errors",
        "absence and presence of a routed alternative error are both valid driver states",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:IgnoreThen",
        "combinator validity is fully expressed by the parser bounds on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:InputRef",
        "mutable parser invocation state is private and maintained by token and rewind operations",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Labelled",
        "parser, label, and context mode are independent until constrained by Parser implementation bounds",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:LocatedError",
        "every parser position and syntax error pair is a valid routed alternative",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Map",
        "combinator validity is fully expressed by parser and callback bounds on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:MapErrWithState",
        "combinator validity is fully expressed by parser and callback bounds on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:MapExtra",
        "state access validity is enforced by the borrowed ParserState lifetime",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:MapWith",
        "combinator validity is fully expressed by parser and callback bounds on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Or",
        "ordered alternatives need no relation beyond the shared output bound on their Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:OwnedRecursiveRoot",
        "the root and family owner are coupled by private construction and each component owns its validity",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Recursive",
        "a weak recursive backedge may validly outlive its owner; parser execution checks owner availability",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RecursiveFamily",
        "shared family storage is always valid and node initialization is tracked by each OnceCell",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RecursiveFamilyStorage",
        "heterogeneous node arena accepts every parser node admitted by its erased trait bound",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:RecursiveNode",
        "both declared and defined OnceCell states are valid phases of recursive grammar construction",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:SimpleSpan",
        "all endpoint pairs are valid because compatibility requires inverted empty spans between tokens",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Spanned",
        "the generic value and span components each own their validity without a cross-field constraint",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser_core.rs:Then",
        "combinator validity is fully expressed by the parser bounds on its Parser implementation",
    ),
    (
        "crates/jbotci-syntax/src/grammar/tokens.rs:IncompleteKindCandidate",
        "diagnostic incomplete-kind candidates are copied ranking tuples built only from syntax metadata",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:GeneratedModelSourceSpanVisitor",
        "generated source-span visitor validity is enforced by Rust references and lifetimes",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:ParseOptions",
        "parse options are independent caller-selected controls",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:RecoveredSyntaxErrorIndexVisitor",
        "recovered syntax error-index visitor carries independent traversal state and error count",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:RecoveredSyntaxParseAttempt",
        "recovered parse attempt combines parser result with optional trace report without extra cross-field constraints",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:RecoveredTokenAndErrorVisitor",
        "test visitor is mutable traversal accumulator state checked by the enclosing recovery assertions",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxConstructMetadata",
        "syntax construct metadata is a static parser table consumed by trace formatting",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxParseAttempt",
        "parse attempt combines parser result with optional trace report without extra cross-field constraints",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxRecoveryErrorPolicy",
        "both limits are nonzero by type and independently configurable; an explicit global hard-cap override may be smaller than the locality limit",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxRecoveryParseAttempt",
        "recovery parse attempt combines a validated strict-or-recovered outcome with an independent optional trace report",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:TokenIdentity",
        "private Arc-backed cache key is constructed only from validated Token and intentionally defines identity by allocation address",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:WithFreeModifiers",
        "generic wrapper delegates validity to its payload and FreeModifierSyntax",
    ),
    (
        "crates/jbotci-tree-macros/src/lib.rs:TreeChildFlags",
        "tree macro child flags are independent switches parsed from field attributes",
    ),
    (
        "crates/jbotci-tree-macros/src/lib.rs:TreeModelOptions",
        "tree macro options are independent switches parsed from macro attributes",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:Chain",
        "generated syntax migration placeholder audited by generated semantics and renderer tests",
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
        "crates/jbotci-tree/src/lib.rs:MissingRecoveryMarker",
        "projection keys accept every source position and diagnostic index",
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
        "crates/jbotci-tree/src/lib.rs:ProjectionRecoveryItem",
        "projection test probes intentionally exercise independent recovery kind, index, and span combinations",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecordingVisitor",
        "tree macro test visitor stores traversal events",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecoveredPrefix",
        "generic recovery prefix stores a non-empty Vec1 of typed recovery items plus a boxed recovered value",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecoveredRecordingVisitor",
        "tree macro recovered test visitor stores traversal events",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecoveryError",
        "generic recovery error pairs a validated tree path with a typed recovery item",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:RecoveryProjection",
        "projection state is either empty or stores one validated marker key",
    ),
    (
        "crates/jbotci-tree/src/lib.rs:TreePath",
        "tree paths are any ordered sequence of validated path steps; tree-relative validity is checked during lookup",
    ),
    (
        "crates/jbotci-ui/src/cukta.rs:CuktaPageSnapshot",
        "cukta page snapshots group memoized render inputs whose validity is enforced by source state and clamped width helpers",
    ),
    (
        "crates/jbotci-ui/src/diagnostics.rs:DiagnosticOverlayMark",
        "diagnostic overlay marks are transient render annotations whose index is validated against the paired diagnostics slice at render time",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:JbotciF2LlmTokenizer",
        "F2LLM tokenizer facade wraps tokenizer state already validated during artifact loading",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:JbotciF2LlmWebGpuRuntime",
        "F2LLM WebGPU runtime facade wraps runtime state already validated during artifact loading",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:JsProgressSink",
        "JavaScript progress callbacks are optional opaque host functions; their behavior is checked when awaited",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:JsVectorStore",
        "JavaScript vector-store callbacks are opaque host functions whose failures are typed at the async boundary",
    ),
    (
        "crates/jbotci-ui/src/gentufa.rs:DesktopReferenceMarkerMetrics",
        "desktop reference marker metrics are direct layout measurements used by overlay placement",
    ),
    (
        "crates/jbotci-ui/src/gentufa.rs:DesktopReferenceOverlayMetrics",
        "desktop reference overlay metrics are direct layout measurements used by overlay placement",
    ),
    (
        "crates/jbotci-ui/src/gentufa.rs:GentufaPageSnapshot",
        "gentufa page snapshots group memoized diagnostic render inputs whose source state owns validity",
    ),
    (
        "crates/jbotci-ui/src/gimfihi.rs:GimfihiPageSnapshot",
        "gimfihi page snapshots group memoized source-word render inputs with no invalid field combinations",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:BlockReferenceFitMetrics",
        "block reference fit metrics are measured renderer geometry consumed by fitting effects",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:BlockReferenceFitUpdate",
        "block reference fit updates are transient DOM measurement results applied immediately",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:BlockReferenceHeightLayoutMetrics",
        "block reference height layout metrics are measured renderer geometry consumed by sizing effects",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:BlockReferenceHeightMetrics",
        "block reference height metrics are measured renderer geometry consumed by sizing effects",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:BlockReferenceHeightUpdates",
        "block reference height updates are transient DOM measurement results applied immediately",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:DesktopGentufaTreeAnchorMetrics",
        "desktop tree anchor metrics are direct layout measurements used to derive overlay geometry",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:DesktopGentufaTreeLayout",
        "desktop tree layout is a transient overlay geometry result derived from measured rows",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:DesktopGentufaTreeMetrics",
        "desktop tree metrics are direct layout measurements used to derive overlay geometry",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:DesktopTooltipMeasure",
        "desktop tooltip measurement is direct platform geometry consumed by placement code",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:DesktopTooltipPlacement",
        "desktop tooltip placement is derived transient UI geometry used immediately for rendering",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:GentufaTreeLineAnchor",
        "tree line anchors are derived from rendered row positions and are validated by layout tests",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:JvozbaPaneMetrics",
        "jvozba pane metrics are direct layout measurements used to derive pane placement",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:NativeEmbeddingSearchWorkerHandle",
        "native embedding worker handle owns channels whose lifecycle is managed by setup and shutdown code",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:ReferenceBottoms",
        "reference bottoms are transient browser DOM measurements checked by reference height sizer tests",
    ),
    (
        "crates/jbotci-ui/src/layout.rs:TopbarLayoutMetrics",
        "topbar metrics are direct layout measurements used by platform layout commands",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncActivityGuard",
        "activity guard is an RAII token whose cleanup invariant is enforced by finish and Drop",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncActivityTask",
        "activity tasks are internal guard tokens created only by AsyncActivityState::begin",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaAsyncPageState",
        "async page state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaPendingScroll",
        "pending scroll state is transient browser navigation state normalized by the cukta scroll handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaSemanticResultState",
        "cukta semantic result state mirrors browser worker hits and is keyed by the committed search state",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaTocInteractionState",
        "cukta TOC interaction state is transient UI state normalized by event handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DialectHighlightToken",
        "dialect highlight tokens are transient lexer spans consumed only by the browser highlighter",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:EmbeddingModelOption",
        "embedding model options are fixed presentation rows projected from the embedding model catalog",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:EmbeddingSettingsState",
        "embedding settings state is transient browser worker status parsed from JSON responses",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaAsyncPageState",
        "async page state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaComputeInputs",
        "gentufa compute inputs are a Dioxus reactive dependency bundle constrained by field types and downstream request construction",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaDisplayState",
        "gentufa display toggles are two independent boolean URL controls with no invalid combination",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaLayoutInputs",
        "gentufa layout inputs are a Dioxus reactive dependency bundle of render state and measured lengths",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GimfihiAsyncResultState",
        "async result state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:HoveredReference",
        "hovered reference state is copied from validated web-core reference markers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:LatestAsyncTask",
        "latest-task state couples Dioxus task handles with activity ids returned by the activity state",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ReferenceHoverState",
        "browser hover state is transient UI state derived from reference label DOM nodes",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:UserSettings",
        "browser settings are persisted transport state constrained by closed enum fields",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuAsyncResultState",
        "async result state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuJvozbaDragState",
        "vlacku jvozba drag state is transient browser pointer state constrained by drag handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuJvozbaPaneState",
        "vlacku jvozba pane state is transient persisted UI state normalized by load/save helpers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuSemanticResultState",
        "vlacku semantic result state mirrors browser worker hits and is keyed by the committed search state",
    ),
    (
        "crates/jbotci-ui/src/page_find.rs:PageFindState",
        "page-find state is a transient UI aggregate whose per-route fields are validated by PageFindRouteState",
    ),
    (
        "crates/jbotci-ui/src/page_find.rs:PageFindTextKey",
        "page-find text keys are transient content identity tokens and every hash plus occurrence pair is a valid key",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingSearchRequest",
        "embedding search requests are platform DTOs checked by service preconditions before execution",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingSearchResponse",
        "embedding search responses are platform DTOs produced from validated search hits",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingSetupProgress",
        "embedding setup progress is a platform transport projection of SetupProgress",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingStatus",
        "embedding status is platform transport state produced by embedding setup and search services",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:ExportRequest",
        "export requests combine renderer payloads and dimensions already validated by export callers",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:JvozbaPaneLayout",
        "platform jvozba pane layout is a transient placement result derived from measured viewport state",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:MemorySettingsStore",
        "memory settings store is fallback platform state constrained by typed settings values",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:NativeComputeService",
        "native compute service is a zero-sized desktop service facade",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:PlatformServiceError",
        "platform service errors carry display diagnostics produced by service implementations",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Rect",
        "platform rectangles are direct geometry DTOs supplied by browser or desktop layout measurements",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Size",
        "platform size is a direct geometry DTO supplied by browser or desktop layout measurements",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TimeoutHandle",
        "timeout handles are opaque platform timer tokens returned by browser scheduling services",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TooltipPlacement",
        "platform tooltip placement is transient geometry produced by platform layout services",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TopbarLayoutMetrics",
        "platform topbar metrics are direct layout measurements used by shared placement code",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TreeLine",
        "platform tree lines are renderer geometry derived from measured syntax rows",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TreeLineAnchor",
        "platform tree line anchors are renderer geometry derived from measured syntax rows",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedClipboardService",
        "unsupported clipboard service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedEmbeddingService",
        "unsupported embedding service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedExportService",
        "unsupported export service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Viewport",
        "platform viewport is a direct geometry DTO supplied by browser or desktop layout measurements",
    ),
    (
        "crates/jbotci-ui/src/routing.rs:PendingLocalRouteWrites",
        "pending route writes are transient browser navigation synchronization state normalized by record and consume helpers",
    ),
    (
        "crates/jbotci-ui/src/routing.rs:RouteLocationSyncAction",
        "route sync action pairs parsed route state with a hydration flag derived by browser navigation handlers",
    ),
    (
        "crates/jbotci-ui/src/settings.rs:SettingsPageSnapshot",
        "settings page snapshots group memoized render inputs whose source settings states own validity",
    ),
    (
        "crates/jbotci-ui/src/vlacku.rs:VlackuPageSnapshot",
        "vlacku page snapshots group memoized dictionary render inputs whose source states own validity",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaIndexEntry",
        "web Cukta index entries are presentation DTOs projected from validated CLL site data",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaModeOption",
        "web Cukta mode options are fixed UI selector DTOs built by cukta_mode_options",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaPageData",
        "web Cukta page data is a presentation DTO produced by build_cukta_web_page",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaSearchResultCard",
        "web Cukta result cards are projected from ranked CLL search matches",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaSectionLink",
        "web Cukta section links are presentation DTOs built from resolved CLL sections",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaSemanticSearchHit",
        "web semantic hit DTOs are parsed from browser worker vector-search output before rendering",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaTargetOption",
        "web Cukta target options are fixed checkbox DTOs built from normalized target state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaTocNode",
        "web Cukta TOC nodes are presentation DTOs built from the parsed CLL chapter tree",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebSearchState",
        "web Cukta search state is normalized by normalize_cukta_state before page building",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaWebState",
        "web Cukta route state is normalized by parse_cukta_web_route and normalize_cukta_state",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:DictionaryTooltipCard",
        "dictionary tooltip cards are presentation payloads projected from validated vlacku cards",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaCell",
        "web cells are renderer transport data built from parser leaves",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaError",
        "web error payload preserves structured parser diagnostics for transport",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaSuccess",
        "successful gentufa response is assembled by parse_gentufa_for_web and checked by focused tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeGuide",
        "gentufa tree guide geometry is derived from rendered syntax rows and covered by web-core tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeRow",
        "tree rows are renderer transport data built from syntax traversal order",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebExport",
        "web export is renderer transport data assembled only after SVG or PNG export succeeds",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebExportRequest",
        "web export request combines validated route state with a closed script selector",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebOptions",
        "web options are independent presentation controls with serde defaults",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebRequest",
        "web request is a serde transport envelope validated by the parser entry point",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebState",
        "gentufa route state is normalized by parse and canonical URL builders before use",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GimfihiPresetOption",
        "gimfihi preset options are fixed UI selector rows built from the preset table",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GimfihiWebResult",
        "gimfihi result payload is assembled from the shared composer and covered by web-core tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GimfihiWebSource",
        "gimfihi web source rows are editable URL/UI state normalized before request construction",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GimfihiWebState",
        "gimfihi web state is a direct URL/local UI state envelope normalized by the result builder",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPiece",
        "composition pieces are display projections from morphology/jvozba decomposition output",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuDictionaryCountNode",
        "dictionary count tree nodes are derived summary data from embedded dictionary metadata",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuDictionaryInfo",
        "dictionary info is derived summary data from the embedded dictionary",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaItem",
        "web jvozba items are persisted UI input state validated by the shared jvozba builder",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuJvozbaSegment",
        "web jvozba segments are display projections from shared jvozba builder output",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuSemanticSearchHit",
        "web semantic hit DTOs are parsed from browser worker vector-search output before rendering",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebCard",
        "vlacku cards are renderer transport data derived from dictionary/search result cards",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebResult",
        "vlacku result payload is assembled from dictionary/search APIs and covered by web-core tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebState",
        "vlacku web state is a direct URL/local UI state envelope normalized by the result builder",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeOption",
        "word type filter options are derived from embedded dictionary metadata each render",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebFeatureAvailability",
        "feature flags are fixed transport data for currently enabled web functionality",
    ),
    (
        "tests/fixture_suite.rs:FakeBackend",
        "fixture test backend stores scripted outputs and captured invocations",
    ),
    (
        "tests/fixture_suite.rs:RecoveredSyntaxTreeExpectationVisitor",
        "fixture test visitor is transient mutable accumulator state converted into validated recovered tree expectations",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:CllSelector",
        "fixture selector validity is checked by fixture profile loading",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:CommandOutputExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:DiagnosticExpectation",
        "fixture diagnostic payload is validated by exact runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:Expectations",
        "fixture expectation aggregate permits absent facets",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureExport",
        "fixture export is a serialization aggregate",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureLojbanSourceShape",
        "fixture source shape records raw TOML key presence so invalid combinations can be diagnosed",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:FixtureSummary",
        "fixture summary is derived reporting data",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:GentufaOutputExpectation",
        "fixture expectation aggregate permits absent gentufa output formats",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:ImportSummary",
        "fixture import summary is derived reporting data",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaExpectation",
        "jvozba fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaOutputExpectation",
        "jvozba output fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:JvozbaSegmentExpectation",
        "jvozba segment fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:LoadedTestCase",
        "loaded fixture combines a test case with its source path",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:MorphologyExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:MuplisSelector",
        "fixture selector validity is checked by fixture profile loading",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:OutputExpectations",
        "fixture expectation aggregate permits absent output formats",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:ReferenceExpectation",
        "semantic refs expectation payload is checked by fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:ScriptBracketExpectations",
        "fixture expectation aggregate permits absent script-specific outputs",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:SemanticsExpectations",
        "fixture expectation aggregate permits absent semantic facets",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:SyntaxExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TersmuOutputExpectation",
        "tersmu fixture expectation payload is checked by exact fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TestCase",
        "fixture loading validates ids, facets, and expectation shape",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TestCaseWire",
        "fixture wire shape preserves raw optional source fields before loader-level validation",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TextExpectation",
        "fixture expectation payload is checked by fixture runner comparisons",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:TextExpectationTable",
        "fixture text expectation wire table is validated immediately after deserialization",
    ),
    (
        "crates/jbotci-semantics/src/model/scope.rs:SourceOrderKey",
        "every source-order key orders loci within one region; no value is invalid",
    ),
    (
        "crates/jbotci-semantics/src/model/scope_recorder.rs:ScopeRecorder",
        "mutable builder state whose constraints are enforced when the scope tree is built",
    ),
    (
        "crates/jbotci-semantics/src/model/scope_recorder.rs:ScopeRegionRecord",
        "a region under construction is incomplete by design; ScopeRegion validates the finished value",
    ),
    (
        "crates/jbotci-semantics/src/model/scope_recorder.rs:ScopeOccurrenceRecord",
        "a recorded occurrence is validated when the model value is built, after region renumbering",
    ),
    (
        "crates/jbotci-semantics/src/model/scope_recorder.rs:ScopeFinalization",
        "the finalization pass borrows recorder state and holds no constraint of its own",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:VlaseiOutputExpectation",
        "fixture expectation aggregate permits absent vlasei output formats",
    ),
    (
        "xtask-common/src/fixtures/mod.rs:XfailExpectation",
        "fixture xfail reason validation is handled by fixture loading",
    ),
    (
        "xtask-common/src/fixtures/runner.rs:FacetResult",
        "runner result combines facet status with diagnostic messages",
    ),
    (
        "xtask-common/src/fixtures/runner.rs:RunSummary",
        "runner summary is derived reporting data",
    ),
    (
        "xtask/src/main.rs:Cli",
        "xtask CLI root delegates input validation to clap",
    ),
    (
        "xtask/src/main.rs:DistServerArgs",
        "xtask dist-server args delegate validation to clap defaults and command code",
    ),
    (
        "xtask/src/main.rs:RenderDockerBuildArgs",
        "xtask render Docker build args delegate validation to clap defaults and command code",
    ),
    (
        "xtask/src/main.rs:RenderDockerRunArgs",
        "xtask render Docker run args delegate validation to clap defaults and command code",
    ),
    (
        "xtask/src/main.rs:ServeWebReleaseArgs",
        "xtask release web server args delegate validation to clap defaults and command code",
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
    for root in ["crates", "apps", "tests", "xtask", "xtask-common"] {
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
    let relative_path = normalized_source_path(relative_path);
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
                placeholders.insert(format!("{relative_path}:{struct_name}"));
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

#[requires(true)]
#[ensures(!ret.contains('\\'))]
fn normalized_source_path(relative_path: &Path) -> String {
    relative_path.to_string_lossy().replace('\\', "/")
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
