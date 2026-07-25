use super::*;
use schemars::transform::{Transform, transform_subschemas};
use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use jbotci_embeddings::native::NativeEmbeddingSearchService as ToolEmbeddingSearchService;
pub const TOOL_DEFAULT_EMBEDDING_MODEL_KEY: &str = DEFAULT_MODEL_KEY;

/// Generate the model-facing JSON schema for a production tool request.
#[requires(true)]
#[ensures(ret.is_object())]
#[ensures(!json_value_contains_key(&ret, "$ref"))]
#[ensures(!json_value_contains_key(&ret, "$defs"))]
pub fn tool_request_schema<T>() -> Value
where
    T: JsonSchema,
{
    // Inline every subschema. The MCP clients we target (including chatbot
    // harnesses) do not resolve `$ref`/`$defs`, so each referenced enum and
    // nested struct must be expanded in place. The tool request types are all
    // non-recursive, so full inlining terminates. This also keeps every field's
    // and enum variant's doc comment as an inline `description`.
    //
    // `StringEnumTypeTransform` then restores an explicit `type: "string"` on the
    // inlined enums (schemars omits it on a documented `oneOf` of consts).
    let mut settings = schemars::generate::SchemaSettings::default();
    settings.inline_subschemas = true;
    settings.transforms.push(Box::new(StringEnumTypeTransform));
    let generator = schemars::generate::SchemaGenerator::new(settings);
    serde_json::to_value(generator.into_root_schema_for::<T>())
        .expect("generated tool request schema serializes to JSON")
}

/// schemars renders a *documented* unit enum as a `oneOf` of
/// `{ "type": "string", "const": … }` — and, unlike the plain `{ "type":
/// "string", "enum": [...] }` it emits for an *undocumented* enum, it omits the
/// `type` at the enclosing level. The schema is still valid (the string type is
/// implied by every branch), but schema viewers and tool layers that read the
/// property-level `type` find none and present the field as untyped ("any").
/// This schemars [`Transform`] declares an explicit `type: "string"` alongside
/// the `oneOf`, keeping the per-variant descriptions.
#[invariant(true)]
#[derive(Clone, Debug)]
struct StringEnumTypeTransform;

impl Transform for StringEnumTypeTransform {
    #[requires(true)]
    #[ensures(true)]
    fn transform(&mut self, schema: &mut Schema) {
        if let Some(object) = schema.as_object_mut() {
            let is_string_const_enum =
                object
                    .get("oneOf")
                    .and_then(Value::as_array)
                    .is_some_and(|variants| {
                        !variants.is_empty()
                            && variants.iter().all(|variant| {
                                variant.get("const").is_some()
                                    && variant.get("type").and_then(Value::as_str) == Some("string")
                            })
                    });
            if is_string_const_enum && !object.contains_key("type") {
                object.insert("type".to_owned(), Value::String("string".to_owned()));
            }
        }
        // Recurse through nested subschemas (properties, array items, …).
        transform_subschemas(self, schema);
    }
}

#[requires(!key.is_empty())]
#[ensures(true)]
fn json_value_contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(object_key, object_value)| {
            object_key == key || json_value_contains_key(object_value, key)
        }),
        Value::Array(items) => items.iter().any(|item| json_value_contains_key(item, key)),
        _ => false,
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolStatus {
    Success,
    Failure,
    ValidMissing,
    InvalidInput,
}

impl ToolStatus {
    #[requires(true)]
    #[ensures(ret == (*self == Self::Success))]
    pub fn is_success(&self) -> bool {
        *self == Self::Success
    }
}

impl From<CliStatus> for ToolStatus {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliStatus) -> Self {
        match value {
            CliStatus::Success => Self::Success,
            CliStatus::Failure => Self::Failure,
            CliStatus::ValidMissing => Self::ValidMissing,
            CliStatus::InvalidInput => Self::InvalidInput,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRenderedOutput {
    pub status: ToolStatus,
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub content_type: Option<String>,
}

impl ToolRenderedOutput {
    #[requires(stderr.is_empty() || stderr.ends_with('\n'))]
    #[ensures(ret.status == ToolStatus::from(status))]
    fn new(
        status: CliStatus,
        stdout: Vec<u8>,
        stderr: String,
        content_type: Option<&'static str>,
    ) -> Self {
        Self {
            status: status.into(),
            stdout,
            stderr,
            content_type: content_type.map(str::to_owned),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn stdout_text(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.stdout)
    }
}

#[invariant(true)]
#[derive(Debug)]
pub struct ToolExecutionContext<'a> {
    embedding_search: Option<&'a mut ToolEmbeddingSearchService>,
    embedding_search_error: Option<String>,
}

impl<'a> ToolExecutionContext<'a> {
    #[requires(true)]
    #[ensures(ret.embedding_search.is_none())]
    pub fn stateless() -> Self {
        Self {
            embedding_search: None,
            embedding_search_error: None,
        }
    }

    #[requires(true)]
    #[ensures(ret.embedding_search.is_some())]
    pub fn with_embedding_search(embedding_search: &'a mut ToolEmbeddingSearchService) -> Self {
        Self {
            embedding_search: Some(embedding_search),
            embedding_search_error: None,
        }
    }

    #[requires(!message.trim().is_empty())]
    #[ensures(ret.embedding_search.is_none())]
    pub fn embedding_search_unavailable(message: String) -> Self {
        Self {
            embedding_search: None,
            embedding_search_error: Some(message),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(super) fn embedding_search(&mut self) -> Result<Option<&mut ToolEmbeddingSearchService>> {
        if let Some(error) = self.embedding_search_error.as_deref() {
            bail!("{error}");
        }
        Ok(self.embedding_search.as_deref_mut())
    }
}

/// Output format for a `gentufa` syntax parse. Text formats are the most
/// readable and token-efficient; `svg`/`png` return a rendered diagram image.
#[invariant(::Tree => true)]
#[invariant(::Brackets => true)]
#[invariant(::Raw => true)]
#[invariant(::Json => true)]
#[invariant(::Svg => true)]
#[invariant(::Png => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolGentufaFormat {
    /// Indented, labelled syntax tree (the default). Shows every grammatical
    /// node (bridi, sumti, selbri, …) and is the clearest format for reasoning
    /// about structure.
    Tree,
    /// Compact nested-bracket notation on one line, e.g. `([lo nánmu] cu
    /// kláma)`. Most token-efficient; omits node-type labels.
    Brackets,
    /// Verbose debug dump of the raw parser AST. For troubleshooting the parser
    /// itself, not normal use.
    Raw,
    /// The full parse tree as structured JSON, for programmatic consumers.
    Json,
    /// Constituency diagram as SVG source, returned as text (XML you can read or
    /// embed in a page). Use `png` if you want a directly displayable image.
    Svg,
    /// Constituency diagram rendered as a PNG image (best for direct display).
    Png,
}

impl Default for ToolGentufaFormat {
    #[requires(true)]
    #[ensures(ret == ToolGentufaFormat::Tree)]
    fn default() -> Self {
        Self::Tree
    }
}

/// Parse Lojban text into a syntax (grammar) tree. This runs the full grammar
/// parser, so it is the authoritative way to see how a sentence is structured
/// and where each word fits. For word-level (morphology) analysis only, use
/// `vlasei` instead. Recoverable syntax failures return the partial tree in
/// `tree`, `brackets`, `raw`, and `json` formats alongside diagnostics.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolGentufaRequest {
    /// The Lojban text to parse. May be a word, a sentence, or several
    /// sentences.
    pub text: String,
    /// How to render the parse. Defaults to the readable `tree`.
    #[serde(default)]
    pub format: ToolGentufaFormat,
    /// Optional dialect selector: a builtin dialect name (e.g. `zantufa`,
    /// `gadganzu`, `ce-ki-tau`) or a parenthesized formula combining them, e.g.
    /// `(cbm ce-ki-tau)`. Omit for standard Lojban.
    #[serde(default)]
    pub dialect: Option<String>,
    /// Prepend full dictionary definitions to the human-readable `tree`,
    /// `brackets`, and `raw` formats. Definitions ground the parse and are on
    /// by default; set this to `false` to save tokens. The flag is suppressed
    /// for `json`, `svg`, and `png` so those formats remain pure documents.
    #[serde(default = "tool_show_defs_default")]
    pub show_defs: bool,
    /// Annotate each node with its source byte span. Off by default.
    #[serde(default)]
    pub show_spans: bool,
    /// Show place-structure cross-references in the `tree` output, e.g. `k⟨1⟩`
    /// marking which sumti fills place 1 of selbri `k`. On by default for `tree`
    /// (usually what you want when inspecting a parse); only the `tree` format
    /// supports it.
    #[serde(default)]
    #[schemars(
        schema_with = "tool_show_refs_schema",
        default = "tool_show_refs_default"
    )]
    pub show_refs: Option<bool>,
    /// Show terminators/words that the grammar elides (omits implicitly). Off by
    /// default.
    #[serde(default)]
    pub show_elided: bool,
    /// Break compound words (lujvo) into their component rafsi in the output.
    /// Off by default.
    #[serde(default)]
    pub decompose_lujvo: bool,
    /// Spaces per indent level for `tree`/`json`. Omit for the standard width.
    #[serde(default)]
    pub indent: Option<usize>,
}

#[invariant(output_type.is_some() == matches!(format, GentufaFormat::Blocks))]
struct ToolGentufaCommandFormat {
    format: GentufaFormat,
    output_type: Option<GentufaImageOutputType>,
}

impl ToolGentufaFormat {
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Tree | Self::Brackets | Self::Raw))]
    fn supports_definitions(self) -> bool {
        matches!(self, Self::Tree | Self::Brackets | Self::Raw)
    }

    #[requires(true)]
    #[ensures(true)]
    fn command_format(self) -> ToolGentufaCommandFormat {
        match self {
            Self::Brackets => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Brackets,
                output_type: None,
            }),
            Self::Tree => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Tree,
                output_type: None,
            }),
            Self::Raw => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Raw,
                output_type: None,
            }),
            Self::Json => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Json,
                output_type: None,
            }),
            Self::Svg => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Blocks,
                output_type: Some(GentufaImageOutputType::Svg),
            }),
            Self::Png => new!(ToolGentufaCommandFormat {
                format: GentufaFormat::Blocks,
                output_type: Some(GentufaImageOutputType::Png),
            }),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content_type(self) -> &'static str {
        match self {
            Self::Json => APPLICATION_JSON_CONTENT_TYPE,
            Self::Svg => "image/svg+xml; charset=utf-8",
            Self::Png => "image/png",
            Self::Brackets | Self::Tree | Self::Raw => TEXT_PLAIN_CONTENT_TYPE,
        }
    }
}

impl From<ToolGentufaRequest> for Command {
    #[requires(true)]
    #[ensures(true)]
    fn from(request: ToolGentufaRequest) -> Self {
        let show_refs = request
            .show_refs
            .unwrap_or(matches!(request.format, ToolGentufaFormat::Tree));
        let show_defs = request.show_defs && request.format.supports_definitions();
        let command_format = request.format.command_format();
        Self::Gentufa(GentufaInput {
            file: None,
            ascii: false,
            detailed_errors: true,
            error_context: 1,
            max_errors: DEFAULT_MAX_ERRORS,
            trace_phase: None,
            trace_limit: None,
            trace_list: false,
            format: command_format.format,
            trace: None,
            dialect: request.dialect,
            show_defs,
            indent: request.indent,
            mark_stress: None,
            mark_glides: None,
            show_spans: request.show_spans,
            show_refs,
            show_elided: request.show_elided,
            decompose_lujvo: request.decompose_lujvo,
            output_type: command_format.output_type,
            output_file: None,
            text: vec![request.text],
        })
    }
}

/// Output format for `vlasei` morphology analysis. `tree` is the readable
/// default; `ipa` gives pronunciation.
#[invariant(::Tree => true)]
#[invariant(::Brackets => true)]
#[invariant(::Ipa => true)]
#[invariant(::Raw => true)]
#[invariant(::Json => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVlaseiFormat {
    /// Indented list of classified words with their word-class (the default),
    /// e.g. `Cmavo "lo"`, `Gismu "nánmu"`.
    Tree,
    /// Compact bracket notation on one line, e.g. `(lo nánmu cu kláma)`.
    Brackets,
    /// IPA phonetic transcription showing syllabification and stress, e.g.
    /// `lo ˈnan.mu ʃu ˈkla.ma`.
    Ipa,
    /// Verbose debug dump of the raw morphology result. For troubleshooting.
    Raw,
    /// Structured JSON of the classified words, for programmatic consumers.
    Json,
}

impl Default for ToolVlaseiFormat {
    #[requires(true)]
    #[ensures(ret == ToolVlaseiFormat::Tree)]
    fn default() -> Self {
        Self::Tree
    }
}

/// Run Lojban morphology: split text into words and classify each one
/// (gismu, cmavo, lujvo, cmevla, fu'ivla, …). Word boundaries in Lojban cannot
/// be found reliably from spaces alone — this runs the real morphology parser.
/// For full sentence grammar, use `gentufa` instead. Recoverable morphology
/// failures mark skipped regions in `tree`, `brackets`, `raw`, and `json`
/// output alongside diagnostics.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolVlaseiRequest {
    /// The Lojban text to analyze. May be a single word or a longer stream.
    pub text: String,
    /// How to render the analysis. Defaults to the readable `tree`.
    #[serde(default)]
    pub format: ToolVlaseiFormat,
    /// Optional dialect selector: a builtin dialect name (e.g. `zantufa`,
    /// `gadganzu`, `ce-ki-tau`) or a parenthesized formula combining them, e.g.
    /// `(cbm ce-ki-tau)`. Omit for standard Lojban.
    #[serde(default)]
    pub dialect: Option<String>,
    /// Annotate each word with its source byte span. Off by default.
    #[serde(default)]
    pub show_spans: bool,
    /// Break compound words (lujvo) into their component rafsi. Off by default.
    #[serde(default)]
    pub decompose_lujvo: bool,
    /// Spaces per indent level for `tree`/`json`. Omit for the standard width.
    #[serde(default)]
    pub indent: Option<usize>,
}

impl ToolVlaseiFormat {
    #[requires(true)]
    #[ensures(true)]
    fn command_format(self) -> VlaseiFormat {
        match self {
            Self::Brackets => VlaseiFormat::Brackets,
            Self::Tree => VlaseiFormat::Tree,
            Self::Ipa => VlaseiFormat::Ipa,
            Self::Raw => VlaseiFormat::Raw,
            Self::Json => VlaseiFormat::Json,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content_type(self) -> &'static str {
        match self {
            Self::Json => APPLICATION_JSON_CONTENT_TYPE,
            Self::Brackets | Self::Tree | Self::Ipa | Self::Raw => TEXT_PLAIN_CONTENT_TYPE,
        }
    }
}

impl From<ToolVlaseiRequest> for Command {
    #[requires(true)]
    #[ensures(true)]
    fn from(request: ToolVlaseiRequest) -> Self {
        Self::Vlasei(VlaseiInput {
            file: None,
            ascii: false,
            detailed_errors: true,
            max_errors: DEFAULT_MAX_ERRORS,
            trace_phase: None,
            trace_limit: None,
            trace_list: false,
            format: request.format.command_format(),
            trace: None,
            dialect: request.dialect,
            indent: request.indent,
            mark_stress: None,
            mark_glides: None,
            show_spans: request.show_spans,
            decompose_lujvo: request.decompose_lujvo,
            text: vec![request.text],
        })
    }
}

/// Output format for `cukta` (the CLL reference book). `markdown` is the
/// readable default.
#[invariant(::Markdown => true)]
#[invariant(::Html => true)]
#[invariant(::Raw => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCuktaFormat {
    /// Markdown (the default): readable prose with headings, tables, and
    /// cross-reference links.
    Markdown,
    /// Rendered HTML of the same content.
    Html,
    /// The raw underlying DocBook source. For tooling that needs the original
    /// markup.
    Raw,
}

impl Default for ToolCuktaFormat {
    #[requires(true)]
    #[ensures(ret == ToolCuktaFormat::Markdown)]
    fn default() -> Self {
        Self::Markdown
    }
}

/// What kind of CLL lookup to perform. The `query` field is interpreted
/// according to this mode.
#[invariant(::Meaning => true)]
#[invariant(::Word => true)]
#[invariant(::Section => true)]
#[invariant(::Example => true)]
#[invariant(::Toc => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCuktaMode {
    /// Semantic search (the default): `query` is a natural-language description
    /// and the best-matching passages are returned. Best for finding where a
    /// concept is explained. Requires the embedding model.
    Meaning,
    /// Keyword search: `query` is a literal term (e.g. a cmavo like `lo`) and
    /// passages containing it are returned. Works without the embedding model.
    Word,
    /// Retrieve one numbered section by reference, e.g. `query: "5.7"`.
    Section,
    /// Retrieve one numbered example by reference, e.g. `query: "6.8"`.
    Example,
    /// Return the book's full table of contents. `query` is ignored.
    Toc,
}

impl Default for ToolCuktaMode {
    #[requires(true)]
    #[ensures(ret == ToolCuktaMode::Meaning)]
    fn default() -> Self {
        Self::Meaning
    }
}

/// One kind of CLL content a `meaning`/`word` search can keep. These are content
/// *kinds*, not references — see `search_result_kinds` on the request.
#[invariant(::Section => true)]
#[invariant(::Paragraph => true)]
#[invariant(::Example => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCuktaSearchResultKind {
    /// Whole sections (a heading and its prose).
    Section,
    /// Individual paragraphs.
    Paragraph,
    /// Worked examples (Lojban with interlinear glosses).
    Example,
}

impl ToolCuktaSearchResultKind {
    /// The canonical lowercase name, matching the CLI `--target` vocabulary.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Paragraph => "paragraph",
            Self::Example => "example",
        }
    }
}

/// Read or search the CLL — *The Complete Lojban Language*, the canonical
/// reference book. Use this to look up grammar rules, find where a concept is
/// explained, or pull a specific section or example.
///
/// To fetch a specific section or example, set `mode` and put the reference in
/// `query` — e.g. `{"mode": "section", "query": "5.2"}` or `{"mode": "example",
/// "query": "6.8"}`. To search, use `mode: "meaning"` (natural language) or
/// `"word"` (literal term), optionally narrowing the kinds of hits with
/// `search_result_kinds` — e.g. `{"mode": "meaning", "query": "tanru",
/// "search-result-kinds": ["section"]}`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolCuktaRequest {
    /// How to interpret `query`. Defaults to `meaning` (semantic search).
    #[serde(default)]
    pub mode: ToolCuktaMode,
    /// The query, interpreted per `mode`: a natural-language description
    /// (`meaning`), a literal term (`word`), or a section/example reference such
    /// as `5.2` (`section`) or `6.8` (`example`). Ignored for `toc`; required
    /// otherwise.
    #[serde(default)]
    pub query: Option<String>,
    /// Maximum number of results for the search modes (`meaning`, `word`).
    /// Ignored by `section`/`example`/`toc`.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub count: Option<usize>,
    /// Narrow the results of a `meaning`/`word` search to these content kinds —
    /// the literal values `section`, `paragraph`, and/or `example`. Empty means
    /// all kinds. This is NOT how you fetch a specific section or example and it
    /// does NOT take references like `5.2`: for that, use `mode: section`/
    /// `example` with the reference in `query`.
    #[serde(default)]
    pub search_result_kinds: Vec<ToolCuktaSearchResultKind>,
    /// Output format. Defaults to the readable `markdown`.
    #[serde(default)]
    pub format: ToolCuktaFormat,
}

impl ToolCuktaRequest {
    #[requires(true)]
    #[ensures(ret == (self.mode == ToolCuktaMode::Meaning))]
    pub fn uses_semantic_search(&self) -> bool {
        self.mode == ToolCuktaMode::Meaning
    }
}

impl ToolCuktaFormat {
    #[requires(true)]
    #[ensures(true)]
    fn command_format(self) -> CuktaCliFormat {
        match self {
            Self::Markdown => CuktaCliFormat::Markdown,
            Self::Html => CuktaCliFormat::Html,
            Self::Raw => CuktaCliFormat::Raw,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content_type(self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown; charset=utf-8",
            Self::Html => "text/html; charset=utf-8",
            Self::Raw => TEXT_PLAIN_CONTENT_TYPE,
        }
    }
}

impl From<ToolCuktaRequest> for Command {
    #[requires(true)]
    #[ensures(true)]
    fn from(request: ToolCuktaRequest) -> Self {
        let query = request.query.unwrap_or_default();
        // The typed `search_result_kinds` set maps directly onto the CLI's
        // per-kind target flags; the CLI's string `targets` channel stays empty.
        // Filters only apply to the search modes and are rejected (downstream)
        // for navigation.
        let mut input = CuktaInput {
            count: request.count,
            toc: false,
            section: None,
            example: None,
            valsi: None,
            targets: Vec::new(),
            target_sections: request
                .search_result_kinds
                .contains(&ToolCuktaSearchResultKind::Section),
            target_paragraphs: request
                .search_result_kinds
                .contains(&ToolCuktaSearchResultKind::Paragraph),
            target_examples: request
                .search_result_kinds
                .contains(&ToolCuktaSearchResultKind::Example),
            format: request.format.command_format(),
            query: Vec::new(),
        };
        match request.mode {
            ToolCuktaMode::Meaning => input.query = vec![query],
            ToolCuktaMode::Word => input.valsi = Some(query),
            ToolCuktaMode::Section => input.section = Some(query),
            ToolCuktaMode::Example => input.example = Some(query),
            ToolCuktaMode::Toc => input.toc = true,
        }
        Self::Cukta(input)
    }
}

/// Which field of a dictionary entry the `query` matches against. See the
/// `query` field for the syntaxes (plain text, glob, and regex) accepted by the
/// `word` and `rafsi` modes.
#[invariant(::Word => true)]
#[invariant(::Rafsi => true)]
#[invariant(::Lujvo => true)]
#[invariant(::Sound => true)]
#[invariant(::Meaning => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVlackuMode {
    /// Match the word itself (the default). Accepts plain text, `*`/`?` globs,
    /// or `/regex/`.
    Word,
    /// Match by rafsi (a word's short affix forms). Accepts plain text, globs,
    /// or `/regex/`.
    Rafsi,
    /// Treat `query` as a lujvo (compound word) and find/analyze it, including
    /// its decomposition into component words.
    Lujvo,
    /// Phonetic search: find words that sound like `query` (given as text or
    /// `[IPA]`).
    Sound,
    /// Semantic search: `query` is a natural-language meaning and the
    /// closest-matching definitions are returned. Requires the embedding model.
    Meaning,
}

impl Default for ToolVlackuMode {
    #[requires(true)]
    #[ensures(ret == ToolVlackuMode::Word)]
    fn default() -> Self {
        Self::Word
    }
}

/// Search the Lojban dictionary (jbovlaste). Returns cards with each entry's
/// word class, rafsi, glosses, place structure, definition, and notes.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolVlackuRequest {
    /// Which field to search. Defaults to `word`.
    #[serde(default)]
    pub mode: ToolVlackuMode,
    /// The query, interpreted per `mode`. For `word`/`rafsi` it may be plain
    /// text, a `*`/`?` glob that describes the whole word, or a `/regex/` matched
    /// as an unanchored substring — so `/^kla/` matches a prefix, `/kla/` matches
    /// anywhere, and `/^klama$/` matches the exact word.
    pub query: String,
    /// Maximum number of entries to return.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub count: Option<usize>,
    /// Restrict to these word classes (e.g. `gismu`, `cmavo`, `lujvo`,
    /// `fu'ivla`, `cmevla`, `experimental`). Empty means all classes.
    #[serde(default)]
    pub word_types: Vec<String>,
    /// Only return entries whose net community vote count is at least this.
    /// Official words have effectively infinite votes.
    #[serde(default)]
    pub min_votes: Option<i32>,
    /// For `sound`/`meaning` search, only return entries scoring at least this
    /// similarity percentage (0–100).
    #[serde(default)]
    #[schemars(range(min = 0, max = 100))]
    pub min_similarity: Option<f32>,
    /// For lujvo results, show the decomposition into component rafsi. Off by
    /// default.
    #[serde(default)]
    pub decompose_lujvo: bool,
    /// Show etymology details (source words/rafsi) where available. Off by
    /// default.
    #[serde(default)]
    pub show_etymology: bool,
}

impl ToolVlackuRequest {
    #[requires(true)]
    #[ensures(ret == (self.mode == ToolVlackuMode::Meaning))]
    pub fn uses_semantic_search(&self) -> bool {
        self.mode == ToolVlackuMode::Meaning
    }
}

#[invariant((requests.len() == 1 && query_text.is_empty()) || (requests.is_empty() && query_text.len() == 1))]
struct ToolVlackuCommandQuery {
    requests: Vec<VlackuRequest>,
    query_text: Vec<String>,
}

impl ToolVlackuMode {
    #[requires(true)]
    #[ensures(true)]
    fn command_query(self, query: String) -> ToolVlackuCommandQuery {
        match self {
            Self::Word => new!(ToolVlackuCommandQuery {
                requests: vec![VlackuRequest::valsi(query)],
                query_text: Vec::new(),
            }),
            Self::Rafsi => new!(ToolVlackuCommandQuery {
                requests: vec![VlackuRequest::rafsi(query)],
                query_text: Vec::new(),
            }),
            Self::Lujvo => new!(ToolVlackuCommandQuery {
                requests: vec![VlackuRequest::lujvo(query)],
                query_text: Vec::new(),
            }),
            Self::Sound => new!(ToolVlackuCommandQuery {
                requests: vec![VlackuRequest::sound(query)],
                query_text: Vec::new(),
            }),
            Self::Meaning => new!(ToolVlackuCommandQuery {
                requests: vec![VlackuRequest::meaning(query)],
                query_text: Vec::new(),
            }),
        }
    }
}

impl TryFrom<ToolVlackuRequest> for Command {
    type Error = anyhow::Error;

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn try_from(request: ToolVlackuRequest) -> std::result::Result<Self, Self::Error> {
        let query = request.query;
        if query.is_empty() {
            bail!("vlacku query must not be empty");
        }
        let command_query = request.mode.command_query(query).into_data();
        Ok(Self::Vlacku(VlackuInput {
            count: request.count,
            ascii: false,
            word_types: request.word_types,
            min_votes: request.min_votes,
            min_similarity: request.min_similarity,
            sumti_places: CliSumtiPlaces::Index,
            decompose_lujvo: request.decompose_lujvo,
            show_etymology: request.show_etymology,
            requests: command_query.requests,
            query: command_query.query_text,
        }))
    }
}

/// What kind of word `jvozba` should assemble.
#[invariant(::Lujvo => true)]
#[invariant(::Cmevla => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolJvozbaMode {
    /// Build a lujvo — an ordinary compound predicate word (the default).
    Lujvo,
    /// Build a cmevla — a name word (ends in a consonant).
    Cmevla,
}

impl Default for ToolJvozbaMode {
    #[requires(true)]
    #[ensures(ret == ToolJvozbaMode::Lujvo)]
    fn default() -> Self {
        Self::Lujvo
    }
}

/// How one source part of a compound is supplied.
#[invariant(::Word => true)]
#[invariant(::FixedRafsi => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolJvozbaPartKind {
    /// A whole word (e.g. a gismu like `bratu`); jvozba picks its best rafsi.
    Word,
    /// A specific rafsi to use verbatim (e.g. `brat`), not chosen by jvozba.
    FixedRafsi,
}

/// One component of the compound, in the order it should appear.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolJvozbaPart {
    /// Whether `value` is a whole word or a fixed rafsi.
    pub kind: ToolJvozbaPartKind,
    /// The word or rafsi text for this part.
    pub value: String,
}

/// Assemble a lujvo (compound word) or cmevla (name) from source parts, applying
/// the standard rafsi-selection and hyphenation rules. Provide at least two
/// rafsi-producing parts, in order. The inverse operation — taking a lujvo apart
/// — is `vlacku` with `mode: "lujvo"`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolJvozbaRequest {
    /// Whether to build a lujvo or a cmevla. Defaults to `lujvo`.
    #[serde(default)]
    pub mode: ToolJvozbaMode,
    /// The source components, in order. Each is a whole word or a fixed rafsi.
    #[serde(default)]
    pub parts: Vec<ToolJvozbaPart>,
}

impl From<ToolJvozbaPart> for JvozbaSourceInput {
    #[requires(true)]
    #[ensures(true)]
    fn from(part: ToolJvozbaPart) -> Self {
        match part.kind {
            ToolJvozbaPartKind::Word => Self::Word(part.value),
            ToolJvozbaPartKind::FixedRafsi => Self::FixedRafsi(part.value),
        }
    }
}

impl From<ToolJvozbaRequest> for Command {
    #[requires(true)]
    #[ensures(true)]
    fn from(request: ToolJvozbaRequest) -> Self {
        Self::Jvozba(JvozbaInput {
            cmevla: request.mode == ToolJvozbaMode::Cmevla,
            sources: request.parts.into_iter().map(Into::into).collect(),
        })
    }
}

/// Output format for `gimfihi` candidate gismu.
#[invariant(::Table => true)]
#[invariant(::Json => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolGimfihiFormat {
    /// Compact ranked table (the default): one row per candidate with score and
    /// per-rafsi collision notes.
    Table,
    /// Full structured JSON of all candidates and their scoring, for
    /// programmatic use.
    Json,
}

/// Gismu candidate scoring algorithm. See `docs/gismu-phonetic-medoid.md`.
#[invariant(::Classic => true)]
#[invariant(::Phonetic => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolGimfihiScorer {
    /// CLL §4.14 letter-overlap scorer (default; preserves historical output).
    #[default]
    Classic,
    /// Full-precision IPA semi-global ALINE medoid scorer.
    Phonetic,
}

/// ALINE normalization denominator from `docs/gismu-phonetic-medoid.md`.
#[invariant(::SourceSide => true)]
#[invariant(::CandidateSide => true)]
#[invariant(::Symmetric => true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolAlineNormalizer {
    /// Divide by source self-similarity (default coverage semantics).
    #[default]
    SourceSide,
    /// Divide by candidate self-similarity.
    CandidateSide,
    /// Divide by the mean of candidate and source self-similarity.
    Symmetric,
}

/// Per-feature ALINE saliences from `docs/gismu-phonetic-medoid.md`.
#[invariant(
    syllabic.is_finite() && *syllabic >= 0.0
        && place.is_finite() && *place >= 0.0
        && manner.is_finite() && *manner >= 0.0
        && voice.is_finite() && *voice >= 0.0
        && nasal.is_finite() && *nasal >= 0.0
        && retroflex.is_finite() && *retroflex >= 0.0
        && lateral.is_finite() && *lateral >= 0.0
        && aspirated.is_finite() && *aspirated >= 0.0
        && high.is_finite() && *high >= 0.0
        && back.is_finite() && *back >= 0.0
        && round.is_finite() && *round >= 0.0
        && long.is_finite() && *long >= 0.0
)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolAlineSaliences {
    /// Syllabic salience (default 5).
    #[serde(default = "tool_salience_syllabic_default")]
    #[schemars(range(min = 0.0))]
    pub syllabic: f64,
    /// Place salience (default 40).
    #[serde(default = "tool_salience_place_default")]
    #[schemars(range(min = 0.0))]
    pub place: f64,
    /// Manner salience (default 50).
    #[serde(default = "tool_salience_manner_default")]
    #[schemars(range(min = 0.0))]
    pub manner: f64,
    /// Voice salience (default 10).
    #[serde(default = "tool_salience_voice_default")]
    #[schemars(range(min = 0.0))]
    pub voice: f64,
    /// Nasal salience (default 10).
    #[serde(default = "tool_salience_nasal_default")]
    #[schemars(range(min = 0.0))]
    pub nasal: f64,
    /// Retroflex salience (default 10).
    #[serde(default = "tool_salience_retroflex_default")]
    #[schemars(range(min = 0.0))]
    pub retroflex: f64,
    /// Lateral salience (default 10).
    #[serde(default = "tool_salience_lateral_default")]
    #[schemars(range(min = 0.0))]
    pub lateral: f64,
    /// Aspirated salience (default 5; aspiration segments are issue #271).
    #[serde(default = "tool_salience_aspirated_default")]
    #[schemars(range(min = 0.0))]
    pub aspirated: f64,
    /// Vowel height salience (default 5).
    #[serde(default = "tool_salience_high_default")]
    #[schemars(range(min = 0.0))]
    pub high: f64,
    /// Vowel backness salience (default 5).
    #[serde(default = "tool_salience_back_default")]
    #[schemars(range(min = 0.0))]
    pub back: f64,
    /// Roundedness salience (default 5).
    #[serde(default = "tool_salience_round_default")]
    #[schemars(range(min = 0.0))]
    pub round: f64,
    /// Length salience (default 1).
    #[serde(default = "tool_salience_long_default")]
    #[schemars(range(min = 0.0))]
    pub long: f64,
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().syllabic)]
fn tool_salience_syllabic_default() -> f64 {
    AlineSaliences::default().syllabic
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().place)]
fn tool_salience_place_default() -> f64 {
    AlineSaliences::default().place
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().manner)]
fn tool_salience_manner_default() -> f64 {
    AlineSaliences::default().manner
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().voice)]
fn tool_salience_voice_default() -> f64 {
    AlineSaliences::default().voice
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().nasal)]
fn tool_salience_nasal_default() -> f64 {
    AlineSaliences::default().nasal
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().retroflex)]
fn tool_salience_retroflex_default() -> f64 {
    AlineSaliences::default().retroflex
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().lateral)]
fn tool_salience_lateral_default() -> f64 {
    AlineSaliences::default().lateral
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().aspirated)]
fn tool_salience_aspirated_default() -> f64 {
    AlineSaliences::default().aspirated
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().high)]
fn tool_salience_high_default() -> f64 {
    AlineSaliences::default().high
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().back)]
fn tool_salience_back_default() -> f64 {
    AlineSaliences::default().back
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().round)]
fn tool_salience_round_default() -> f64 {
    AlineSaliences::default().round
}

#[requires(true)]
#[ensures(ret == AlineSaliences::default().long)]
fn tool_salience_long_default() -> f64 {
    AlineSaliences::default().long
}

impl Default for ToolAlineSaliences {
    #[requires(true)]
    #[ensures(ret.manner == AlineSaliences::default().manner)]
    fn default() -> Self {
        let defaults = AlineSaliences::default();
        new!(ToolAlineSaliences {
            syllabic: defaults.syllabic,
            place: defaults.place,
            manner: defaults.manner,
            voice: defaults.voice,
            nasal: defaults.nasal,
            retroflex: defaults.retroflex,
            lateral: defaults.lateral,
            aspirated: defaults.aspirated,
            high: defaults.high,
            back: defaults.back,
            round: defaults.round,
            long: defaults.long,
        })
    }
}

impl ToolAlineSaliences {
    #[requires(true)]
    #[ensures(ret.is_finite() && ret >= 0.0)]
    fn value(&self, feature: AlineFeature) -> f64 {
        match feature {
            AlineFeature::Syllabic => self.syllabic,
            AlineFeature::Place => self.place,
            AlineFeature::Manner => self.manner,
            AlineFeature::Voice => self.voice,
            AlineFeature::Nasal => self.nasal,
            AlineFeature::Retroflex => self.retroflex,
            AlineFeature::Lateral => self.lateral,
            AlineFeature::Aspirated => self.aspirated,
            AlineFeature::High => self.high,
            AlineFeature::Back => self.back,
            AlineFeature::Round => self.round,
            AlineFeature::Long => self.long,
        }
    }
}

#[requires(true)]
#[ensures(ret == AlineParameters::default().c_sub)]
fn tool_c_sub_default() -> f64 {
    AlineParameters::default().c_sub
}

#[requires(true)]
#[ensures(ret == AlineParameters::default().c_exp)]
fn tool_c_exp_default() -> f64 {
    AlineParameters::default().c_exp
}

#[requires(true)]
#[ensures(ret == AlineParameters::default().c_skip)]
fn tool_c_skip_default() -> f64 {
    AlineParameters::default().c_skip
}

#[requires(true)]
#[ensures(ret == AlineParameters::default().c_vwl)]
fn tool_c_vwl_default() -> f64 {
    AlineParameters::default().c_vwl
}

#[requires(true)]
#[ensures(ret == AlineParameters::default().c_flank)]
fn tool_c_flank_default() -> f64 {
    AlineParameters::default().c_flank
}

impl Default for ToolGimfihiFormat {
    #[requires(true)]
    #[ensures(ret == ToolGimfihiFormat::Table)]
    fn default() -> Self {
        Self::Table
    }
}

/// Which existing gismu a candidate is checked against for rafsi collisions.
#[invariant(::All => true)]
#[invariant(::Official => true)]
#[invariant(::None => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCollisionScope {
    /// Check against every gismu, official and experimental (the default).
    All,
    /// Check only against official gismu.
    Official,
    /// Skip collision checking entirely.
    None,
}

impl Default for ToolCollisionScope {
    #[requires(true)]
    #[ensures(ret == ToolCollisionScope::All)]
    fn default() -> Self {
        Self::All
    }
}

impl From<ToolCollisionScope> for CliCollisionScope {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: ToolCollisionScope) -> Self {
        match value {
            ToolCollisionScope::All => Self::All,
            ToolCollisionScope::Official => Self::Official,
            ToolCollisionScope::None => Self::None,
        }
    }
}

/// One source word feeding the gismu-composition algorithm: the word from a
/// natural language, optionally with a custom blending weight.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolGimfihiSource {
    /// A short language code for this source (e.g. `eng`, `cmn`, `spa`). Each
    /// language may appear only once. With a `preset`, the code must be exactly
    /// one the preset lists (see the `preset` field).
    pub language: String,
    /// The word for this concept as a **broad phonetic IPA transcription** of how it
    /// is pronounced in this language — its sounds, not its phonemes, and definitely
    /// not its spelling.
    ///
    /// Transcribe carefully:
    /// - Work at the **narrow phonetic** level to the extent permitted by the
    ///   inventory provided. For example, if some phoneme has allophones which map to
    ///   different IPA symbols in the inventory, then use those different symbols
    ///   according to the actual pronunciation. Apply the language's own vowel
    ///   weakening and reduction rules such as akanye, final devoicing, consonant
    ///   assimilation such as nb→mb or kz→gz, effect of adjacent phonemes on each
    ///   other etc, and pick positional allophones according to how the language is
    ///   actually spoken.
    /// - Drop grammatical endings (Spanish noun -o/-a: *gato* → `ɡat`).
    /// - **Do not use the schwa `ə`** — it is rejected. Where you would use it, instead
    ///   use the nearest full vowel in the provided IPA symbol inventory corresponding
    ///   to the actual allophone of schwa in this position, based on the language's
    ///   actual pronunciation of this word.
    ///
    /// Use standard IPA from this inventory; the tie bar `◌͡◌`, length `ː`,
    /// nasalization `◌̃`, palatalization `ʲ`, labialization `ʷ`, aspiration `ʰ`,
    /// and emphasis `ˤ` are fine to include:
    /// - Consonants `p b t d k g q`, `f v θ ð s z ʃ ʒ ɕ ʑ ʂ ʐ ç x ɣ χ ħ h ɦ`,
    ///   affricates `t͡ʃ d͡ʒ t͡s d͡z t͡ɕ d͡ʑ`, `m n ŋ ɲ ɳ`, `l ʎ ɫ`,
    ///   `r ɾ ɹ ɻ ʀ ʁ ɽ`, `j w ɥ ʋ`, retroflex `ʈ ɖ`.
    /// - Vowels `i y ɨ ʉ ɯ u ɪ ʊ`, `e ø ɛ œ ɘ ɜ o ɔ ɤ ɵ ɒ`, `a æ ɐ ɑ ʌ`, nasal
    ///   vowels (`ɛ̃ ɑ̃ ɔ̃` …), glides, and length (`aː`).
    ///
    /// Examples (word → IPA): English *cat* → `kæt`, *late* → `leɪ̯t`; Spanish *gato*
    /// (drop -o) → `ɡat`; Mandarin 用心 → `jʊŋɕin`; French *bon* → `bɔ̃`; Arabic
    /// *ḥasan* → `ħasan`; Russian *мягко* → `mʲaxkʌ`, *мялись* → `mʲælʲɪsʲ`, *спасибо*
    /// → `spɐsʲibʌ`.
    ///
    /// Reason carefully about the precise transcription and double-check to make sure
    /// that you didn't use morphological or orthographic representation masquerading
    /// as IPA; enumerate all the relevant features of the language phonology, such as
    /// vowel reduction, devoicing, assimilation etc, and make sure that you have
    /// correctly represented their effects in all positions. If in doubt, look the
    /// word up in Wiktionary and check Wikipedia articles on the language's phonology.
    pub word: String,
    /// Optional blending weight (1–999). Required for every source unless
    /// `preset` supplies the weights. Use presets unless user specifically requests
    /// custom weights, in which case weights are typically based on the number of
    /// speakers based on some specified criteria.
    #[serde(default)]
    #[schemars(range(min = 1, max = 999))]
    pub weight: Option<u16>,
}

impl ToolGimfihiSource {
    /// Parse the CLI `LANG[:WEIGHT]:WORD` source spec into a typed source. This
    /// is a convenience for delivery vehicles whose input is inherently a flat
    /// string (the Discord slash command); MCP callers pass the fields directly.
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
    pub fn from_spec(spec: &str) -> std::result::Result<Self, String> {
        let parsed = parse_source_spec(spec).map_err(|error| error.to_string())?;
        Ok(Self {
            language: parsed.language,
            word: parsed.word,
            weight: parsed.explicit_weight,
        })
    }
}

/// Propose candidate gismu (root words) from a set of source-language words,
/// using the standard gismu-creation algorithm: score every legal candidate by how
/// well its letters recall the weighted sources, then rank them. This *creates new
/// root words*; it does not look up existing ones. The set of inputs is determined by
/// the `sources` and/or `preset` fields. Presets are based on the number of L1 and,
/// depending on the preset, L2 speakers, relative weights assigned to them, and the
/// number of top languages picked from the list. Unless specifically directed
/// otherwise by the user, use ilmen12.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolGimfihiRequest {
    /// The source words, one per language. Provide weights here, or via
    /// `preset`.
    #[serde(default)]
    pub sources: Vec<ToolGimfihiSource>,
    /// Scoring algorithm; defaults to `classic`. The `phonetic` formula is
    /// specified by `docs/gismu-phonetic-medoid.md`.
    #[serde(default)]
    pub scorer: ToolGimfihiScorer,
    /// ALINE substitution ceiling C_sub (default 35); see
    /// `docs/gismu-phonetic-medoid.md`.
    #[serde(default = "tool_c_sub_default")]
    pub c_sub: f64,
    /// ALINE expansion ceiling C_exp (default 45); see
    /// `docs/gismu-phonetic-medoid.md`.
    #[serde(default = "tool_c_exp_default")]
    pub c_exp: f64,
    /// ALINE unmatched-segment penalty C_skip (default -10, nonpositive); see
    /// `docs/gismu-phonetic-medoid.md`.
    #[serde(default = "tool_c_skip_default")]
    #[schemars(range(max = 0.0))]
    pub c_skip: f64,
    /// ALINE vowel discount C_vwl (default 10, nonnegative); see
    /// `docs/gismu-phonetic-medoid.md`.
    #[serde(default = "tool_c_vwl_default")]
    #[schemars(range(min = 0.0))]
    pub c_vwl: f64,
    /// Source flank skip rate C_flank (default 0; must lie between C_skip and
    /// 0); see `docs/gismu-phonetic-medoid.md`.
    #[serde(default = "tool_c_flank_default")]
    #[schemars(range(max = 0.0))]
    pub c_flank: f64,
    /// Normalizer mode from `docs/gismu-phonetic-medoid.md`; defaults to
    /// `source-side`.
    #[serde(default)]
    pub normalizer: ToolAlineNormalizer,
    /// Complete feature-salience table from
    /// `docs/gismu-phonetic-medoid.md`; omitted fields use the documented
    /// Kondrak defaults.
    #[serde(default)]
    pub saliences: ToolAlineSaliences,
    /// Use a named weight preset instead of supplying per-source `weight`s. You
    /// must then provide exactly the languages that preset covers (ISO 639-3
    /// codes):
    /// - `1985`, `1987`, `1994`, `1995`, `1999`, `evenly` — the classic six:
    ///   `cmn` (Mandarin), `hin` (Hindi), `eng` (English), `spa` (Spanish),
    ///   `rus` (Russian), `ara` (Arabic). The years differ only in their
    ///   speaker-population weights; `evenly` weights all six equally.
    /// - `ilmen6` — `cmn`, `eng`, `hin`, `spa`, `ara`, `fra`
    /// - `ilmen8` — `cmn`, `eng`, `spa`, `hin`, `ara`, `ben` (Bengali), `rus`,
    ///   `por` (Portuguese).
    /// - `ilmen12` — the `ilmen8` languages plus `msa` (Malay), `jpn`
    ///   (Japanese), `deu` (German), `fra` (French).
    #[serde(default)]
    pub preset: Option<String>,
    /// Candidate letter shapes to generate. Each is `ccvcv` or `cvccv`; empty
    /// means both (the standard gismu shapes).
    #[serde(default)]
    pub shapes: Vec<String>,
    /// Which gismu to check rafsi collisions against. Defaults to `all`.
    #[serde(default)]
    pub check_collisions: ToolCollisionScope,
    /// Score using all letters rather than only the rafsi-relevant ones. Off by
    /// default.
    #[serde(default)]
    pub all_letters: bool,
    /// Also include candidates that collide with an existing gismu (each marked
    /// with the colliding word); otherwise they are omitted. Off by default.
    #[serde(default)]
    pub show_collisions: bool,
    /// Only keep candidates that have at least one free (unclaimed) short rafsi.
    /// Off by default.
    #[serde(default)]
    pub require_free_short_rafsi: bool,
    /// Maximum number of ranked candidates to return (1–512).
    #[serde(default)]
    #[schemars(range(min = 1, max = 512))]
    pub count: Option<usize>,
    /// Highlight this specific gismu in the output if it appears among the
    /// candidates.
    #[serde(default)]
    pub highlight: Option<String>,
    /// Output format. Defaults to the readable `table`.
    #[serde(default)]
    pub format: ToolGimfihiFormat,
}

impl Default for ToolGimfihiRequest {
    #[requires(true)]
    #[ensures(ret.scorer == ToolGimfihiScorer::Classic)]
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            scorer: ToolGimfihiScorer::Classic,
            c_sub: tool_c_sub_default(),
            c_exp: tool_c_exp_default(),
            c_skip: tool_c_skip_default(),
            c_vwl: tool_c_vwl_default(),
            c_flank: tool_c_flank_default(),
            normalizer: ToolAlineNormalizer::SourceSide,
            saliences: ToolAlineSaliences::default(),
            preset: None,
            shapes: Vec::new(),
            check_collisions: ToolCollisionScope::default(),
            all_letters: false,
            show_collisions: false,
            require_free_short_rafsi: false,
            count: None,
            highlight: None,
            format: ToolGimfihiFormat::default(),
        }
    }
}

impl ToolGimfihiFormat {
    #[requires(true)]
    #[ensures(true)]
    fn command_format(self) -> GimfihiCliFormat {
        match self {
            Self::Table => GimfihiCliFormat::Table,
            Self::Json => GimfihiCliFormat::Json,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content_type(self) -> &'static str {
        match self {
            Self::Json => APPLICATION_JSON_CONTENT_TYPE,
            Self::Table => TEXT_PLAIN_CONTENT_TYPE,
        }
    }
}

#[invariant(true)]
struct ToolGimfihiCommandInput {
    request: ToolGimfihiRequest,
    word_kind: GimfihiSourceWordKind,
}

impl TryFrom<ToolGimfihiCommandInput> for Command {
    type Error = anyhow::Error;

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn try_from(input: ToolGimfihiCommandInput) -> std::result::Result<Self, Self::Error> {
        let request = input.request;
        let sources = request
            .sources
            .into_iter()
            .map(|source| tool_gimfihi_source_to_input(source, input.word_kind))
            .collect::<Result<Vec<_>>>()?;
        let scorer = match request.scorer {
            ToolGimfihiScorer::Classic => GimfihiCliScorer::Classic,
            ToolGimfihiScorer::Phonetic => GimfihiCliScorer::Phonetic,
        };
        let normalizer = match request.normalizer {
            ToolAlineNormalizer::SourceSide => GimfihiCliNormalizer::SourceSide,
            ToolAlineNormalizer::CandidateSide => GimfihiCliNormalizer::CandidateSide,
            ToolAlineNormalizer::Symmetric => GimfihiCliNormalizer::Symmetric,
        };
        let saliences = AlineFeature::all()
            .iter()
            .copied()
            .map(|feature| {
                new!(GimfihiSalienceOverride {
                    feature,
                    value: request.saliences.value(feature),
                })
            })
            .collect();
        Ok(Self::Gimfihi(GimfihiInput {
            sources,
            scorer,
            c_sub: request.c_sub,
            c_exp: request.c_exp,
            c_skip: request.c_skip,
            c_vwl: request.c_vwl,
            c_flank: request.c_flank,
            normalizer,
            saliences,
            preset: request.preset,
            shapes: request.shapes,
            check_collisions: request.check_collisions.into(),
            all_letters: request.all_letters,
            show_collisions: request.show_collisions,
            require_free_short_rafsi: request.require_free_short_rafsi,
            count: request.count,
            highlight: request.highlight,
            format: request.format.command_format(),
        }))
    }
}

/// Output format for a `tersmu` semantic analysis. `tree+proj` is the default
/// human projection, `tree` is its bare structural spine, and `json` is the
/// canonical interchange graph.
#[invariant(::Json => true)]
#[invariant(::Tree => true)]
#[invariant(::TreeProj => true)]
#[invariant(::Smusni => true)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ToolTersmuFormat {
    /// Canonical `lojban-semantics-json-1` flat id-graph.
    Json,
    /// Indented utterance/formula structure showing quantifier, negation, and
    /// connective nesting with referent ids inlined.
    Tree,
    /// The default: structural tree followed by only displaced projective
    /// commitments, with frame and implicit-constant boilerplate grouped.
    /// `+proj` is the format-feature suffix added to the `tree` base format.
    #[serde(rename = "tree+proj")]
    TreeProj,
    /// EXPERIMENTAL: the model-facing `smusni` notation — a flat,
    /// keyword-oriented rendering of the same graph tuned for language models.
    /// Not the default; `tree+proj` remains the default.
    Smusni,
}

impl Default for ToolTersmuFormat {
    #[requires(true)]
    #[ensures(ret == ToolTersmuFormat::TreeProj)]
    fn default() -> Self {
        Self::TreeProj
    }
}

impl ToolTersmuFormat {
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Tree | Self::TreeProj))]
    fn supports_definitions(self) -> bool {
        matches!(self, Self::Tree | Self::TreeProj)
    }

    #[requires(true)]
    #[ensures(true)]
    fn command_format(self) -> TersmuFormat {
        match self {
            Self::Json => TersmuFormat::Json,
            Self::Tree => TersmuFormat::Tree,
            Self::TreeProj => TersmuFormat::TreeProj,
            Self::Smusni => TersmuFormat::Smusni,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn content_type(self) -> &'static str {
        match self {
            Self::Json => APPLICATION_JSON_CONTENT_TYPE,
            Self::Tree | Self::TreeProj | Self::Smusni => TEXT_PLAIN_CONTENT_TYPE,
        }
    }
}

/// Build the deep semantic representation of Lojban text. The canonical result
/// is the `lojban-semantics-json-1` graph; optional human formats are pure
/// renderings of that same graph. Reach for this when you need logical meaning,
/// beyond morphology (`vlasei`) or grammar (`gentufa`).
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolTersmuRequest {
    /// The Lojban text to interpret.
    pub text: String,
    /// How to render the graph. Defaults to `tree+proj`: a logical nesting tree
    /// plus only commitments displaced from their structural site. Use `tree`
    /// for the bare spine or `json` for the canonical graph. The `+proj` suffix
    /// follows the `base+feature` convention for format features. Human formats
    /// obey the tersmu interpretation contract documented in the tool
    /// description.
    #[serde(default)]
    pub format: ToolTersmuFormat,
    /// Optional dialect selector: a builtin dialect name (e.g. `zantufa`,
    /// `gadganzu`, `ce-ki-tau`) or a parenthesized formula combining them, e.g.
    /// `(cbm ce-ki-tau)`. Omit for standard Lojban.
    #[serde(default)]
    pub dialect: Option<String>,
    /// Prepend full dictionary definitions to the human-readable `tree` and
    /// `tree+proj` formats. Definitions ground the interpretation and are on
    /// by default; set this to `false` to save tokens. The flag is suppressed
    /// for `json` so the canonical graph remains a pure JSON document.
    #[serde(default = "tool_show_defs_default")]
    pub show_defs: bool,
    /// Carry tense forward across sentences as an advancing narrative "story
    /// time", instead of anchoring every sentence to speech time. Off by
    /// default.
    #[serde(default)]
    pub story_time: bool,
    /// Spaces per indent level. Defaults to 2 (pretty-printed for readability);
    /// set `0` for compact single-line JSON to save tokens.
    #[serde(default)]
    pub indent: Option<usize>,
}

impl From<ToolTersmuRequest> for Command {
    #[requires(true)]
    #[ensures(true)]
    fn from(request: ToolTersmuRequest) -> Self {
        let show_defs = request.show_defs && request.format.supports_definitions();
        Self::Tersmu(TersmuInput {
            file: None,
            format: request.format.command_format(),
            max_errors: DEFAULT_MAX_ERRORS,
            trace: None,
            dialect: request.dialect,
            show_defs,
            story_time: request.story_time,
            // Explicit JSON remains pretty-printed by default; `0` opts into compact.
            indent: Some(request.indent.unwrap_or(2)),
            text: vec![request.text],
        })
    }
}

#[requires(true)]
#[ensures(ret)]
fn tool_show_defs_default() -> bool {
    true
}

#[requires(true)]
#[ensures(ret.as_object().is_some())]
fn tool_show_refs_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "boolean",
        "default": true
    })
}

#[requires(true)]
#[ensures(ret == Some(true))]
fn tool_show_refs_default() -> Option<bool> {
    Some(true)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_gentufa(request: ToolGentufaRequest) -> Result<ToolRenderedOutput> {
    let content_type = request.format.content_type();
    run_tool_command(Command::from(request), Some(content_type))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_vlasei(request: ToolVlaseiRequest) -> Result<ToolRenderedOutput> {
    let content_type = request.format.content_type();
    run_tool_command(Command::from(request), Some(content_type))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_cukta(request: ToolCuktaRequest) -> Result<ToolRenderedOutput> {
    run_tool_cukta_inner(request, None)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_cukta_with_context(
    request: ToolCuktaRequest,
    context: &mut ToolExecutionContext<'_>,
) -> Result<ToolRenderedOutput> {
    run_tool_cukta_inner(request, Some(context))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_tool_cukta_inner(
    request: ToolCuktaRequest,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<ToolRenderedOutput> {
    let content_type = request.format.content_type();
    run_tool_command_with_context(Command::from(request), Some(content_type), tool_context)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_vlacku(request: ToolVlackuRequest) -> Result<ToolRenderedOutput> {
    run_tool_vlacku_inner(request, None)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_vlacku_with_context(
    request: ToolVlackuRequest,
    context: &mut ToolExecutionContext<'_>,
) -> Result<ToolRenderedOutput> {
    run_tool_vlacku_inner(request, Some(context))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_tool_vlacku_inner(
    request: ToolVlackuRequest,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<ToolRenderedOutput> {
    let command = Command::try_from(request)?;
    run_tool_command_with_context(command, Some(TEXT_PLAIN_CONTENT_TYPE), tool_context)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_jvozba(request: ToolJvozbaRequest) -> Result<ToolRenderedOutput> {
    run_tool_command(Command::from(request), Some(TEXT_PLAIN_CONTENT_TYPE))
}

/// How a tool caller's source `word` should be read.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GimfihiSourceWordKind {
    /// Bare phonemic IPA, always transliterated to Lojban (the MCP tool).
    Ipa,
    /// Lojban letters by default, with `[IPA]` opting into transliteration (the
    /// CLI/Discord/web bracket convention).
    LojbanOrBracketedIpa,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_gimfihi(
    request: ToolGimfihiRequest,
    word_kind: GimfihiSourceWordKind,
) -> Result<ToolRenderedOutput> {
    let content_type = request.format.content_type();
    let command = Command::try_from(ToolGimfihiCommandInput { request, word_kind })?;
    run_tool_command(command, Some(content_type))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn tool_gimfihi_source_to_input(
    source: ToolGimfihiSource,
    word_kind: GimfihiSourceWordKind,
) -> Result<GimfihiSourceInput> {
    if let Some(weight) = source.weight
        && !(GIMFIHI_MIN_WEIGHT..=GIMFIHI_MAX_WEIGHT).contains(&weight)
    {
        bail!(
            "source weight for `{}` must be from {GIMFIHI_MIN_WEIGHT} to {GIMFIHI_MAX_WEIGHT}, got {weight}",
            source.language
        );
    }
    Ok(match word_kind {
        GimfihiSourceWordKind::Ipa => {
            GimfihiSourceInput::from_ipa_fields(&source.language, &source.word, source.weight)
        }
        GimfihiSourceWordKind::LojbanOrBracketedIpa => {
            GimfihiSourceInput::from_fields(&source.language, &source.word, source.weight)
        }
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_tersmu(request: ToolTersmuRequest) -> Result<ToolRenderedOutput> {
    let content_type = request.format.content_type();
    run_tool_command(Command::from(request), Some(content_type))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_tool_command(
    command: Command,
    content_type: Option<&'static str>,
) -> Result<ToolRenderedOutput> {
    run_tool_command_with_context(command, content_type, None)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_tool_command_with_context(
    command: Command,
    content_type: Option<&'static str>,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<ToolRenderedOutput> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_cli_command_with_tool_context(
        command,
        &mut stdout,
        &mut stderr,
        CliColorPolicy::never(),
        DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        None,
        CliProgressPolicy::disabled(),
        None,
        tool_context,
    )?;
    let stderr =
        String::from_utf8(stderr).context("jbotci tool diagnostics were not valid UTF-8")?;
    Ok(ToolRenderedOutput::new(
        status,
        stdout,
        stderr,
        content_type,
    ))
}

const TEXT_PLAIN_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const APPLICATION_JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(ret.format == format && ret.show_defs == show_defs)]
    fn gentufa_request(format: ToolGentufaFormat, show_defs: bool) -> ToolGentufaRequest {
        ToolGentufaRequest {
            text: "mi klama".to_owned(),
            format,
            dialect: None,
            show_defs,
            show_spans: false,
            show_refs: None,
            show_elided: false,
            decompose_lujvo: false,
            indent: None,
        }
    }

    #[requires(true)]
    #[ensures(ret.format == format && ret.show_defs == show_defs)]
    fn tersmu_request(format: ToolTersmuFormat, show_defs: bool) -> ToolTersmuRequest {
        ToolTersmuRequest {
            text: "mi klama".to_owned(),
            format,
            dialect: None,
            show_defs,
            story_time: false,
            indent: None,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_tool_request_schema_inlines_and_types_documented_enums() {
        let schema = tool_request_schema::<ToolTersmuRequest>();

        assert!(!json_value_contains_key(&schema, "$ref"));
        assert!(!json_value_contains_key(&schema, "$defs"));
        assert_eq!(schema["properties"]["format"]["type"], "string");
        assert!(schema["properties"]["format"]["oneOf"].is_array());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn definition_grounding_defaults_to_true_when_requests_omit_the_field() {
        let gentufa: ToolGentufaRequest =
            serde_json::from_value(serde_json::json!({ "text": "mi klama" }))
                .expect("gentufa request without show-defs");
        let tersmu: ToolTersmuRequest =
            serde_json::from_value(serde_json::json!({ "text": "mi klama" }))
                .expect("tersmu request without show-defs");

        assert!(gentufa.show_defs);
        assert!(tersmu.show_defs);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn definition_grounding_schema_documents_true_defaults() {
        for schema in [
            tool_request_schema::<ToolGentufaRequest>(),
            tool_request_schema::<ToolTersmuRequest>(),
        ] {
            assert_eq!(schema["properties"]["show-defs"]["default"], true);
            let description = schema["properties"]["show-defs"]["description"]
                .as_str()
                .expect("show-defs description");
            assert!(description.contains("by default"), "{description}");
            assert!(description.contains("suppressed"), "{description}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tersmu_json_suppresses_definitions_and_remains_one_document() {
        let grounded =
            run_tool_tersmu(tersmu_request(ToolTersmuFormat::Json, true)).expect("grounded JSON");
        let ungrounded = run_tool_tersmu(tersmu_request(ToolTersmuFormat::Json, false))
            .expect("ungrounded JSON");

        assert_eq!(grounded.status, ToolStatus::Success);
        assert_eq!(grounded.stdout, ungrounded.stdout);
        let _: serde_json::Value =
            serde_json::from_slice(&grounded.stdout).expect("single pure JSON document");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tersmu_human_formats_prepend_definitions() {
        for format in [ToolTersmuFormat::Tree, ToolTersmuFormat::TreeProj] {
            let grounded = run_tool_tersmu(tersmu_request(format, true))
                .expect("grounded human tersmu output");
            let ungrounded = run_tool_tersmu(tersmu_request(format, false))
                .expect("ungrounded human tersmu output");
            let grounded = grounded.stdout_text().expect("UTF-8 tersmu output");
            let ungrounded = ungrounded.stdout_text().expect("UTF-8 tersmu output");
            let definitions = grounded
                .strip_suffix(ungrounded)
                .expect("show-defs only prepends definitions");

            assert!(
                definitions.starts_with("1. mi | by: officialdata | cmavo: KOhA3"),
                "{format:?}"
            );
            assert!(
                definitions.contains("\n2. klama | by: officialdata | gismu"),
                "{format:?}"
            );
            assert!(definitions.ends_with('\n'), "{format:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_machine_formats_suppress_definitions() {
        for format in [
            ToolGentufaFormat::Json,
            ToolGentufaFormat::Svg,
            ToolGentufaFormat::Png,
        ] {
            let grounded = run_tool_gentufa(gentufa_request(format, true))
                .expect("grounded machine gentufa output");
            let ungrounded = run_tool_gentufa(gentufa_request(format, false))
                .expect("ungrounded machine gentufa output");

            assert_eq!(grounded.status, ToolStatus::Success, "{format:?}");
            assert_eq!(grounded.stdout, ungrounded.stdout, "{format:?}");
            match format {
                ToolGentufaFormat::Json => {
                    let _: serde_json::Value = serde_json::from_slice(&grounded.stdout)
                        .expect("single pure JSON document");
                }
                ToolGentufaFormat::Svg => {
                    let svg = grounded.stdout_text().expect("UTF-8 SVG");
                    let document = roxmltree::Document::parse(svg).expect("valid SVG XML");
                    assert_eq!(document.root_element().tag_name().name(), "svg");
                }
                ToolGentufaFormat::Png => {
                    assert!(grounded.stdout.starts_with(b"\x89PNG\r\n\x1a\n"));
                }
                ToolGentufaFormat::Tree | ToolGentufaFormat::Brackets | ToolGentufaFormat::Raw => {
                    unreachable!("human format excluded from loop")
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_human_formats_prepend_definitions() {
        for format in [
            ToolGentufaFormat::Tree,
            ToolGentufaFormat::Brackets,
            ToolGentufaFormat::Raw,
        ] {
            let grounded = run_tool_gentufa(gentufa_request(format, true))
                .expect("grounded human gentufa output");
            let ungrounded = run_tool_gentufa(gentufa_request(format, false))
                .expect("ungrounded human gentufa output");
            let grounded = grounded.stdout_text().expect("UTF-8 gentufa output");
            let ungrounded = ungrounded.stdout_text().expect("UTF-8 gentufa output");
            let definitions = grounded
                .strip_suffix(ungrounded)
                .expect("show-defs only prepends definitions");

            assert!(
                definitions.starts_with("1. mi | by: officialdata | cmavo: KOhA3"),
                "{format:?}"
            );
            assert!(
                definitions.contains("\n2. klama | by: officialdata | gismu"),
                "{format:?}"
            );
            assert!(definitions.ends_with('\n'), "{format:?}");
        }
    }
}
