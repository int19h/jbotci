use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use walkdir::WalkDir;

const ALLOWED_PLACEHOLDERS: &[(&str, &str)] = &[
    (
        "apps/jbotci-server/src/lib.rs:AppState",
        "server state is assembled by ServerConfig and contains shared immutable assets",
    ),
    (
        "apps/jbotci-server/src/lib.rs:HealthResponse",
        "health payload is a fixed transport shape",
    ),
    (
        "apps/jbotci-server/src/lib.rs:ServerConfig",
        "server config is normalized by ServerConfig::from_cli",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:UserSettings",
        "browser settings are persisted transport state constrained by closed enum fields",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaDisplayState",
        "gentufa display toggles are two independent boolean URL controls with no invalid combination",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ReferenceHoverState",
        "browser hover state is transient UI state derived from reference label DOM nodes",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:HoveredReference",
        "hovered reference state is copied from validated web-core reference markers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ArrowOverlay",
        "arrow overlay geometry is measured from the browser DOM and rendered transiently",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ReferenceRect",
        "reference rectangles are direct browser DOM measurements used only during hover rendering",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ReferenceBottoms",
        "reference bottoms are transient browser DOM measurements checked by reference height sizer tests",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DiagnosticOverlayMark",
        "diagnostic overlay marks are transient render annotations whose index is validated against the paired diagnostics slice at render time",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuJvozbaPaneState",
        "vlacku jvozba pane state is transient persisted UI state normalized by load/save helpers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuJvozbaDragState",
        "vlacku jvozba drag state is transient browser pointer state constrained by drag handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:EmbeddingSettingsState",
        "embedding settings state is transient browser worker status parsed from JSON responses",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:EmbeddingModelOption",
        "embedding model options are fixed presentation rows projected from the embedding model catalog",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ElementSize",
        "element sizes are direct layout measurements used transiently by render effects",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:ViewportSize",
        "viewport sizes are direct platform measurements used transiently by layout code",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:PositionedPoint",
        "positioned points are direct layout measurements used transiently by render effects",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:TopbarLayoutMetrics",
        "topbar metrics are direct layout measurements used by platform layout commands",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:JvozbaPaneMetrics",
        "jvozba pane metrics are direct layout measurements used to derive pane placement",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:JvozbaPaneLayout",
        "jvozba pane layout is a transient placement result derived from measured viewport state",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:BlockReferenceHeightMetrics",
        "block reference height metrics are measured renderer geometry consumed by sizing effects",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:BlockReferenceHeightLayoutMetrics",
        "block reference height layout metrics are measured renderer geometry consumed by sizing effects",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:BlockReferenceHeightUpdates",
        "block reference height updates are transient DOM measurement results applied immediately",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:BlockReferenceFitMetrics",
        "block reference fit metrics are measured renderer geometry consumed by fitting effects",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:BlockReferenceFitUpdate",
        "block reference fit updates are transient DOM measurement results applied immediately",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopGentufaTreeAnchorMetrics",
        "desktop tree anchor metrics are direct layout measurements used to derive overlay geometry",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopGentufaTreeMetrics",
        "desktop tree metrics are direct layout measurements used to derive overlay geometry",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopGentufaTreeLayout",
        "desktop tree layout is a transient overlay geometry result derived from measured rows",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopReferenceMarkerMetrics",
        "desktop reference marker metrics are direct layout measurements used by overlay placement",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopReferenceOverlayMetrics",
        "desktop reference overlay metrics are direct layout measurements used by overlay placement",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopTooltipMeasure",
        "desktop tooltip measurement is direct platform geometry consumed by placement code",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DesktopTooltipPlacement",
        "desktop tooltip placement is derived transient UI geometry used immediately for rendering",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaTocInteractionState",
        "cukta TOC interaction state is transient UI state normalized by event handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:NativeEmbeddingSearchWorkerHandle",
        "native embedding worker handle owns channels whose lifecycle is managed by setup and shutdown code",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:PlatformServiceError",
        "platform service errors carry display diagnostics produced by service implementations",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingStatus",
        "embedding status is platform transport state produced by embedding setup and search services",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:EmbeddingSetupProgress",
        "embedding setup progress is a platform transport projection of SetupProgress",
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
        "crates/jbotci-ui/src/platform.rs:ExportRequest",
        "export requests combine renderer payloads and dimensions already validated by export callers",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Size",
        "platform size is a direct geometry DTO supplied by browser or desktop layout measurements",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Rect",
        "platform rectangles are direct geometry DTOs supplied by browser or desktop layout measurements",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:Viewport",
        "platform viewport is a direct geometry DTO supplied by browser or desktop layout measurements",
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
        "crates/jbotci-ui/src/platform.rs:JvozbaPaneLayout",
        "platform jvozba pane layout is a transient placement result derived from measured viewport state",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TreeLineAnchor",
        "platform tree line anchors are renderer geometry derived from measured syntax rows",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:TreeLine",
        "platform tree lines are renderer geometry derived from measured syntax rows",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:MemorySettingsStore",
        "memory settings store is fallback platform state constrained by typed settings values",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedClipboardService",
        "unsupported clipboard service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedExportService",
        "unsupported export service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:UnsupportedEmbeddingService",
        "unsupported embedding service is a zero-sized platform fallback",
    ),
    (
        "crates/jbotci-ui/src/platform.rs:NativeComputeService",
        "native compute service is a zero-sized desktop service facade",
    ),
    (
        "crates/jbotci-orthography/src/lib.rs:NormalizedLatinChar",
        "orthography conversion helper stores a normalized character plus stress flag",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncActivityTask",
        "activity tasks are internal guard tokens created only by AsyncActivityState::begin",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncActivityState",
        "activity state is mutated through begin and finish helpers that preserve task-token ownership",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:AsyncActivityGuard",
        "activity guard is an RAII token whose cleanup invariant is enforced by finish and Drop",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:LatestAsyncTask",
        "latest-task state couples Dioxus task handles with activity ids returned by the activity state",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaPendingScroll",
        "pending scroll state is transient browser navigation state normalized by the cukta scroll handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:PendingLocalRouteWrites",
        "pending route writes are transient browser navigation synchronization state normalized by record and consume helpers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:RouteLocationSyncAction",
        "route sync action pairs parsed route state with a hydration flag derived by browser navigation handlers",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaAsyncPageState",
        "async page state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:DialectHighlightToken",
        "dialect highlight tokens are transient lexer spans consumed only by the browser highlighter",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:GentufaTreeLineAnchor",
        "tree line anchors are derived from rendered row positions and are validated by layout tests",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaAsyncPageState",
        "async page state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuAsyncResultState",
        "async result state is transient UI cache data keyed and replaced by latest-wins worker tasks",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:VlackuSemanticResultState",
        "vlacku semantic result state mirrors browser worker hits and is keyed by the committed search state",
    ),
    (
        "crates/jbotci-ui/src/lib.rs:CuktaSemanticResultState",
        "cukta semantic result state mirrors browser worker hits and is keyed by the committed search state",
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
        "apps/jbotci/src/main.rs:GernaInput",
        "nightly grammar-export CLI args delegate validation to clap and command code",
    ),
    (
        "apps/jbotci/src/main.rs:JvozbaInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:CuktaInput",
        "CLI cukta input delegates raw mode and target validation to validate_cukta_input",
    ),
    (
        "apps/jbotci/src/main.rs:TextInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:VlackuInput",
        "custom clap parser preserves ordered request flags and command validation checks mode combinations",
    ),
    (
        "apps/jbotci/src/main.rs:SetupInput",
        "setup CLI input delegates model and directory validation to setup command execution",
    ),
    (
        "apps/jbotci/src/main.rs:CliProgressPolicy",
        "CLI progress policy is derived from terminal capability and caller-selected verbosity",
    ),
    (
        "apps/jbotci/src/main.rs:CliSetupProgressReporter",
        "CLI setup progress reporter owns rendering state derived from the selected progress policy",
    ),
    (
        "apps/jbotci/src/main.rs:VlaseiInput",
        "CLI input selector permits stdin, file, and literal text shapes",
    ),
    (
        "apps/jbotci/src/main.rs:CapturedCliRun",
        "test helper records CLI process output after run_cli returns a status",
    ),
    (
        "apps/jbotci/src/main.rs:CliColorPolicy",
        "resolved color policy is two independent stream decisions",
    ),
    (
        "apps/jbotci/src/main.rs:CliParsedTraceSpec",
        "trace spec parsing validates level and filter shape before constructing this transport value",
    ),
    (
        "apps/jbotci/src/main.rs:CliTraceConfig",
        "trace limit is validated once at CLI entry and phase is a closed enum",
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
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:ImplType",
        "contract scanner fixture intentionally contains accepted no-op markers",
    ),
    (
        "crates/bityzba/tests/contract_scanner/complete/src/lib.rs:Marker",
        "contract scanner fixture intentionally contains accepted no-op markers",
    ),
    (
        "crates/jbotci-dialect/src/lib.rs:CustomDialect",
        "custom dialect definitions are parsed and normalized through dialect resolution helpers",
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
        "crates/jbotci-output/src/qr_code.rs:QrBlock",
        "QR block geometry is produced by the QR renderer and covered by placement tests",
    ),
    (
        "crates/jbotci-output/src/qr_code.rs:QrBuild",
        "QR build state is internal renderer assembly data validated by encoded-output tests",
    ),
    (
        "crates/jbotci-output/src/qr_code.rs:QrCode",
        "QR code data is constructed by the QR encoder before renderer placement",
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
        "crates/jbotci-output/src/qr_code.rs:QrLogoPlacement",
        "QR logo placement is selected by renderer search and validated by placement tests",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:BoSumtiTailSyntax",
        "private parser continuation state is consumed immediately into validated sumti connection nodes",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:BoTanruUnitTailSyntax",
        "private parser continuation state is consumed immediately into validated tanru unit connection nodes",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:GekNuhiTermsetHeadSyntax",
        "private parser head state is consumed immediately into validated termset connection nodes",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:KeSumtiTailSyntax",
        "private parser continuation state is consumed immediately into validated grouped sumti nodes",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeGuide",
        "gentufa tree guide geometry is derived from rendered syntax rows and covered by web-core tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeRowDraft",
        "tree row drafts are intermediate layout data produced before final rendered row validation",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeRowSortKey",
        "tree row sort keys are derived ordering data with scalar fields checked by row-order tests",
    ),
    (
        "crates/bityzba/tests/type_invariant.rs:PlainMarker",
        "bityzba fixture covers explicit no-op type markers",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllAnchor",
        "CLL anchor records are constructed from parsed DocBook ids and grouped in site indexes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllChapter",
        "CLL chapter records are constructed by the embedded DocBook loader from ordered chapter files",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllExample",
        "CLL examples are assembled by parse_example_block from section context and interlinear lines",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllExampleLine",
        "CLL example lines preserve upstream DocBook line kind and normalized text",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllIndexEntry",
        "CLL index entries are grouped from parsed indexterm nodes with duplicate section ids removed",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllMetadata",
        "CLL metadata is fixed by the embedded loader for the bundled CLL corpus",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllReference",
        "CLL references are created from parsed section/example context in the embedded loader",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllSearchChunk",
        "CLL search chunks are generated from parsed sections and tagged-word extraction",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllSearchMatch",
        "CLL search matches are ranked only by cukta_word_search_matches after target filtering",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllSection",
        "CLL sections are constructed from DocBook section nodes with computed numbering and text",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CllSite",
        "CLL site is assembled once by load_embedded_cll_site and owns all derived indexes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CuktaSearchOutput",
        "cukta search output is built by cukta_search from normalized query/count inputs",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:CuktaTargetFilter",
        "target filters intentionally preserve all checkbox states before validation/defaulting",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:BlockParseState",
        "private CLL block parse state is a monotonically advanced chapter-local counter",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:LinkResolution",
        "link resolutions are private loader intermediates derived from the completed anchor index",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:PendingIndexEntry",
        "pending index entries are private loader intermediates from DocBook indexterm nodes",
    ),
    (
        "crates/jbotci-cll/src/lib.rs:SectionParseContext",
        "section parse context is private loader state derived from an already parsed section heading",
    ),
    (
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputCorpus",
        "browser/native embedding corpus DTO is generated from embedded dictionary and CLL data immediately before JSON serialization",
    ),
    (
        "crates/jbotci-embedding-inputs/src/lib.rs:EmbeddingInputDocument",
        "browser/native embedding document DTO is generated from v0-parity embedding input builders",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingModelSpec",
        "embedding model specs are fixed catalog records created by model_spec",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingCatalogModel",
        "embedding catalog model rows are written by setup after pack validation",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingCatalog",
        "embedding catalog is a static transport manifest written by setup",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingRuntime",
        "embedding runtime entries are fixed manifest transport metadata",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:VectorShardManifest",
        "vector shard manifests are generated from written shard files and SHA-256 checks",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:CorpusManifest",
        "corpus manifests are generated from validated item files and vector shards",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:EmbeddingPackManifest",
        "embedding pack manifests are generated after all corpus shards are written and validated",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:DictionaryEmbeddingItem",
        "dictionary embedding item rows are generated from embedded dictionary entry order",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:CllEmbeddingItem",
        "CLL embedding item rows are generated from embedded CLL search chunk order",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:LoadedCorpusCacheKey",
        "loaded corpus cache keys are assembled from validated manifest and shard metadata before lookup",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:LoadedCorpus",
        "loaded corpus dimensions and vector lengths are validated by load_corpus before caching",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:VectorHit",
        "vector hits are produced by bounded vector ranking over validated row-major matrices",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:DictionarySemanticHit",
        "dictionary semantic hits are produced by joining vector hits to generated item rows",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:QueryEmbedding",
        "query embeddings are produced by backend implementations and normalized before search",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupOptions",
        "embedding setup options are validated by model lookup and path resolution",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupReport",
        "embedding setup reports are returned only after pack construction or validated reuse",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:SetupProgress",
        "embedding setup progress is transport state produced by setup phases and consumed for display",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:FakeBackend",
        "test fake backend is constrained by test construction and used only for fixture packs",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:ReusableVectorRows",
        "native incremental rebuild rows are loaded from a previously validated pack and keyed by stored input hashes",
    ),
    (
        "crates/jbotci-embeddings/src/lib.rs:ReusablePackRows",
        "native incremental rebuild cache is loaded only from a compatible previously validated pack",
    ),
    (
        "crates/jbotci-embeddings/src/native.rs:NativeLlamaEmbeddingBackend",
        "native backend fields are produced by llama.cpp model/context initialization",
    ),
    (
        "crates/jbotci-embeddings/src/native.rs:NativeEmbeddingSearchService",
        "native embedding search service owns validated manifest and backend state from setup",
    ),
    (
        "crates/jbotci-ui/src/f2llm_runtime_core.rs:TokenizerArtifact",
        "F2LLM tokenizer artifacts are external manifest DTOs validated while loading the runtime",
    ),
    (
        "crates/jbotci-ui/src/f2llm_runtime_core.rs:SpecialTokens",
        "F2LLM special token ids are external tokenizer metadata interpreted by the tokenizer",
    ),
    (
        "crates/jbotci-ui/src/f2llm_runtime_core.rs:QwenByteBpeTokenizer",
        "Qwen tokenizer state is assembled from external artifacts and exercised through tokenizer tests",
    ),
    (
        "crates/jbotci-ui/src/f2llm_runtime_core.rs:TokenWindow",
        "token windows are generated by tokenizer helpers from bounded prompt and history inputs",
    ),
    (
        "crates/jbotci-ui/src/f2llm_runtime_core.rs:PackedTokenBatch",
        "packed token batches are generated by runtime helpers immediately before model execution",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:RuntimeLoadOptions",
        "WebGPU runtime load options are caller-selected controls checked by runtime loading code",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:ArtifactManifest",
        "WebGPU artifact manifests are external DTOs validated while loading model artifacts",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:TokenizerSpec",
        "WebGPU tokenizer specs are external manifest DTOs validated while loading artifacts",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:ModelConfig",
        "WebGPU model config is external manifest metadata validated during runtime construction",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:TensorSpec",
        "WebGPU tensor specs are external manifest DTOs validated while binding tensors",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:ChunkedSpec",
        "WebGPU chunked tensor specs are external manifest DTOs validated while loading chunks",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:ChunkSpec",
        "WebGPU chunk specs are external manifest DTOs validated while loading tensor chunks",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:Q4Tensor",
        "Q4 tensor buffers are built only after tensor manifest and byte-size validation",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:F32Tensor",
        "F32 tensor buffers are built only after tensor manifest and byte-size validation",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:VectorBuffer",
        "WebGPU vector buffers are constructed by runtime buffer allocation helpers",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:WebGpuRuntime",
        "WebGPU runtime state is assembled through the fallible runtime loader before use",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:CorpusVectorSpec",
        "corpus vector specs are external manifest DTOs validated while loading semantic search shards",
    ),
    (
        "crates/jbotci-ui/src/f2llm_webgpu_runtime.rs:CorpusShard",
        "corpus shards are external artifact DTOs validated while loading semantic search data",
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
        "crates/jbotci-web-core/src/lib.rs:CuktaSemanticSearchHit",
        "web semantic hit DTOs are parsed from browser worker vector-search output before rendering",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:CuktaSectionLink",
        "web Cukta section links are presentation DTOs built from resolved CLL sections",
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
        "crates/jbotci-jvozba/src/lib.rs:LujvoDecomposition",
        "decompose_lujvo_like constructs this only after rafsi count and source resolution checks",
    ),
    (
        "crates/jbotci-jvozba/src/lib.rs:LujvoSegmentInfo",
        "segment source resolution is local to decompose_lujvo_like and hyphen segments intentionally have no source",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:AlineFeatures",
        "ALINE feature vectors are derived from a fixed IPA segment table",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:IpaSegmentVector",
        "IPA segment vectors are constructed from tokenizer table entries and derived feature vectors",
    ),
    (
        "crates/jbotci-search/src/phonetic.rs:IpaTokenSequence",
        "token sequences are constructed by tokenizer helpers that reject empty segment lists",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuSearchOptions",
        "CLI validation constrains count and mode-specific similarity use before lookup execution",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuSearchOutput",
        "lookup execution owns card, diagnostic, and worst-outcome aggregation semantics",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCard",
        "dictionary cards are transport values assembled from dictionary entries or validated word classification",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:ParsedWordDictionaryMatch",
        "parsed-word dictionary matches are derived from morphology spans and rendered dictionary cards",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:ParsedWordLookupTarget",
        "parsed-word lookup targets are transient values built from morphology spans before dictionary lookup",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:VlackuCompositionPiece",
        "composition pieces are projected from validated morphology decomposition segments",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:WordClassification",
        "word classifications are produced from morphology segmentation of a single word-like token",
    ),
    (
        "crates/jbotci-search/src/vlacku.rs:GlobPattern",
        "glob patterns are constructed only by compile_glob_pattern after token validation",
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
        "crates/jbotci-diagnostics/src/lib.rs:TraceFailureBranch",
        "branch context and expectation payloads are collected from structured parser metadata",
    ),
    (
        "crates/jbotci-diagnostics/src/lib.rs:TraceRecorderState",
        "recorder state is deliberately mutable; public recorder methods enforce event and limit invariants",
    ),
    (
        "crates/jbotci-output/src/trace.rs:TraceRenderOptions",
        "trace renderer options are caller-selected presentation controls",
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
        "crates/jbotci-morphology/src/grammar.rs:Segmenter",
        "segmenter is mutable parser state whose invariants are algorithm-local",
    ),
    (
        "crates/jbotci-morphology/src/grammar.rs:SourceChar",
        "source character pairs one char with its byte position",
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
        "crates/jbotci-output/src/diagnostics.rs:DiagnosticRenderOptions",
        "diagnostic rendering options are independent caller-selected controls",
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
        "crates/jbotci-output/src/lib.rs:BracketSourceRange",
        "bracket source ranges mirror parser byte spans supplied by renderer construction paths",
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
        "crates/jbotci-output/src/references.rs:SyntaxWordCollector",
        "collector is transient traversal state for modal slot labels",
    ),
    (
        "crates/jbotci-output/src/references.rs:TreeWordLabel",
        "word labels are copied from already rendered tree word values",
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
        "crates/jbotci-output/src/tree.rs:RenderedPosition",
        "rendered syntax token end positions are copied directly from validated source spans",
    ),
    (
        "crates/jbotci-output/src/json.rs:RenderedPosition",
        "rendered syntax token end positions are copied directly from validated source spans",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GentufaBlock",
        "gentufa block rows are renderer transport data built from parser spans",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:GentufaBlocksLayout",
        "block layout is a renderer projection with ordering covered by web-core tests",
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
        "crates/jbotci-gentufa/src/lib.rs:ElidedTerminator",
        "elided terminators are transient renderer records built from validated absent syntax fields",
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
        "crates/jbotci-web-core/src/lib.rs:GentufaTreeRow",
        "tree rows are renderer transport data built from syntax traversal order",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebOptions",
        "web options are independent presentation controls with serde defaults",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebState",
        "gentufa route state is normalized by parse and canonical URL builders before use",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:GentufaWebRequest",
        "web request is a serde transport envelope validated by the parser entry point",
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
        "crates/jbotci-web-core/src/lib.rs:DictionaryTooltipCard",
        "dictionary tooltip cards are presentation payloads projected from validated vlacku cards",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:LeafCollector",
        "leaf collector is transient traversal state for web block and tree projections",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ElidedTerminatorCollector",
        "elided terminator collector is transient traversal state consumed into validated terminator records",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:RenderedPosition",
        "rendered syntax token end positions are copied directly from validated source spans",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockLeafPart",
        "block leaf parts are transient layout slices derived from validated parser spans",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockTemp",
        "temporary block color state is consumed inside the layout builder before transport output",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:BlockTreeNode",
        "block tree nodes are transient layout state derived from syntax index metadata",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceLabel",
        "gentufa reference labels are generated from the shared CLI reference display model",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:ReferenceMarker",
        "reference markers are renderer annotations derived from semantic reference analysis",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:RenderedLeaf",
        "rendered leaves are transient projections from parsed syntax leaves",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:TransformInfo",
        "transform metadata is a display annotation for deterministic orthography conversion",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:WebFeatureAvailability",
        "feature flags are fixed transport data for currently enabled web functionality",
    ),
    (
        "crates/jbotci-gentufa/src/lib.rs:WebSourceRange",
        "source ranges mirror parser span metadata and may be absent at API boundaries",
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
        "crates/jbotci-gentufa/src/render.rs:GentufaPngOptions",
        "PNG options are normalized by callers and scale is guarded by render preconditions",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:GentufaSvgOptions",
        "SVG options are independent presentation controls with a caller-provided title",
    ),
    (
        "crates/jbotci-gentufa/src/render.rs:PositionedBlocks",
        "positioned block metrics are produced by the renderer layout pass before use",
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
        "crates/jbotci-gentufa/src/render.rs:TextMeasureKey",
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
        "crates/jbotci-gentufa/src/render.rs:ReferenceStackBottoms",
        "reference stack bottoms are derived renderer layout measurements checked by reference sizing tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebState",
        "vlacku web state is a direct URL/local UI state envelope normalized by the result builder",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebResult",
        "vlacku result payload is assembled from dictionary/search APIs and covered by web-core tests",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWebCard",
        "vlacku cards are renderer transport data derived from dictionary/search result cards",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuSemanticSearchHit",
        "web semantic hit DTOs are parsed from browser worker vector-search output before rendering",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuCompositionPiece",
        "composition pieces are display projections from morphology/jvozba decomposition output",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuWordTypeOption",
        "word type filter options are derived from embedded dictionary metadata each render",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuDictionaryInfo",
        "dictionary info is derived summary data from the embedded dictionary",
    ),
    (
        "crates/jbotci-web-core/src/lib.rs:VlackuDictionaryCountNode",
        "dictionary count tree nodes are derived summary data from embedded dictionary metadata",
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
        "crates/jbotci-search/src/lib.rs:SearchHit",
        "search score semantics are index-specific",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:AbstractionNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiMention",
        "argument mention validity is maintained by discourse traversal and resolved through SyntaxIndex ids",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiPlaceAssignment",
        "assignment referential validity is cross-checked through PlaceAnalysis frame and argument indexes",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SumtiPlaceAssignmentId",
        "assignment ids are opaque PlaceAnalysis keys whose bounds are checked by assignment lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FixtureSumtiAssignment",
        "fixture assignment records are stable projections of typed reference analysis facts",
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
        "crates/jbotci-semantics/src/references.rs:DiscourseReferenceBuilder",
        "discourse reference builder validity is governed by traversal order and consumed into DiscourseReferences",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:DiscourseReferences",
        "reference edge index consistency is produced by the builder and checked through edge lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:FreeModifierNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:IndexedSyntaxNode",
        "indexed syntax node entries are produced from generated AST traversal and keyed by SyntaxIndex",
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
        "crates/jbotci-semantics/src/references.rs:PlaceAnalysisBuilder",
        "place-analysis builder validity is traversal-local and consumed into PlaceAnalysis",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:PlaceCursor",
        "place cursor is private traversal state initialized by constructors that choose the first numbered slot",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:BridiNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:BridiTailAnalysis",
        "predicate-tail analysis is private traversal state produced alongside frame propagation",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:BridiTailNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:RawSyntaxNodeId",
        "raw syntax node ids are opaque SyntaxIndex keys whose bounds are checked by node lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceAnalysis",
        "reference analysis aggregates separately built syntax, place, and discourse indexes",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceEdge",
        "reference edge source and target validity is checked by DiscourseReferences and SyntaxIndex lookup",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:ReferenceEdgeId",
        "reference edge ids are opaque DiscourseReferences keys whose bounds are checked by edge lookup",
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
        "crates/jbotci-semantics/src/references.rs:TanruUnitNodeId",
        "syntax node ids are opaque index keys whose validity is checked by SyntaxIndex typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SelbriPlaceFrame",
        "place frame referential validity is checked through PlaceAnalysis and SyntaxIndex lookup APIs",
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
        "crates/jbotci-semantics/src/references.rs:SyntaxIndex",
        "syntax index consistency is produced by generated AST traversal and enforced through typed lookup APIs",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SyntaxIndexBuilder",
        "syntax index builder validity is governed by generated traversal enter and exit sequencing",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SyntaxNodeMetadata",
        "syntax node metadata is derived from generated traversal order and morphology leaf spans",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:SyntaxSpanKey",
        "span keys are compatibility/debug projections derived from SourceSpan metadata",
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
        "crates/jbotci-semantics/src/references.rs:V0SumtiAssignment",
        "v0 compatibility assignment is a lossy projection whose source facts remain in PlaceAnalysis",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:V0CompatibilityProjection",
        "v0 compatibility projection is a derived serialization aggregate",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:V0ReferenceEdge",
        "v0 compatibility reference edge is a lossy projection whose source facts remain in DiscourseReferences",
    ),
    (
        "crates/jbotci-semantics/src/references.rs:V0SelbriPlace",
        "v0 compatibility relation-place entry is derived from typed place assignments",
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
        "crates/jbotci-syntax/src/grammar/parser.rs:ParserDialectConfig",
        "parser dialect config is an independent feature-flag snapshot",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parser.rs:ParserDialectConfigScope",
        "parser dialect config scope only stores the previous thread-local snapshot for restoration",
    ),
    (
        "crates/jbotci-syntax/src/grammar/parse_error.rs:SyntaxParseError",
        "lifetime-bearing Chumsky error wrapper preserves invariants through constructors and merge helpers",
    ),
    (
        "crates/jbotci-syntax/src/grammar/ast.rs:ConnectiveSyntaxParts",
        "owned connective decomposition preserves validity from ConnectiveSyntax",
    ),
    (
        "crates/jbotci-syntax/src/grammar/tense.rs:CompositeTenseModalClassification",
        "mutable classification state is projected into validated tense structs",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:ParseOptions",
        "parse options are independent caller-selected controls",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxConstructMetadata",
        "syntax construct metadata is a static parser table consumed by trace formatting",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParsedStatementAttempt",
        "syntax attempt combines parser result with optional trace report without extra cross-field constraints",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserCheckpoint",
        "checkpoint mirrors Chumsky save state with warning count plus whether trace would record the save",
    ),
    (
        "crates/jbotci-syntax/src/grammar/mod.rs:ParserStateFinish",
        "parser finish value carries deduplicated warnings and optional trace report from ParserState",
    ),
    (
        "crates/jbotci-syntax/src/lib.rs:SyntaxParseAttempt",
        "parse attempt combines parser result with optional trace report without extra cross-field constraints",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:SumtiConnectionSyntax",
        "argument connection delegates marker validity to ConnectiveSyntax",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:BridiStatementContinuationSyntax",
        "continuation marker enum owns the BO/KE marker checks",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:AfterthoughtBridiTailSyntax",
        "predicate-tail aggregate delegates marker validity to continuation nodes",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:BoGroupedBridiTailSyntax",
        "predicate-tail aggregate delegates marker validity to BO continuation nodes",
    ),
    (
        "crates/jbotci-syntax/src/tree.rs:BridiTailSyntax",
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
        "crates/jbotci-tree/src/lib.rs:TreePath",
        "tree paths are any ordered sequence of validated path steps; tree-relative validity is checked during lookup",
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
        "tests/support/fixtures/mod.rs:GentufaOutputExpectation",
        "fixture expectation aggregate permits absent gentufa output formats",
    ),
    (
        "tests/support/fixtures/mod.rs:JvozbaExpectation",
        "jvozba fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:JvozbaOutputExpectation",
        "jvozba output fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:JvozbaSegmentExpectation",
        "jvozba segment fixture expectations are checked by exact fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:DiagnosticExpectation",
        "fixture diagnostic payload is validated by exact runner comparisons",
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
        "tests/support/fixtures/mod.rs:ReferenceExpectation",
        "semantic refs expectation payload is checked by fixture runner comparisons",
    ),
    (
        "tests/support/fixtures/mod.rs:ScriptBracketExpectations",
        "fixture expectation aggregate permits absent script-specific outputs",
    ),
    (
        "tests/support/fixtures/mod.rs:SemanticsExpectations",
        "fixture expectation aggregate permits absent semantic facets",
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
        "tests/support/fixtures/mod.rs:VlaseiOutputExpectation",
        "fixture expectation aggregate permits absent vlasei output formats",
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
        "crates/jbotci-dictionary-data/build.rs:DictionaryMetadata",
        "vendored Lensisku metadata is validated against dictionary contents and hashes in the build script",
    ),
    (
        "crates/jbotci-dictionary-data/src/lib.rs:DictionarySnapshotMetadata",
        "embedded snapshot metadata is generated from validated build metadata and checked by dictionary-data tests",
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
