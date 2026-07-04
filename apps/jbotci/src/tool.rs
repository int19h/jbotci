use super::*;

pub use jbotci_embeddings::native::NativeEmbeddingSearchService as ToolEmbeddingSearchService;
pub const TOOL_DEFAULT_EMBEDDING_MODEL_KEY: &str = DEFAULT_MODEL_KEY;

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
/// `vlasei` instead.
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
    /// Prepend the full dictionary definition of every word before the tree.
    /// Informative but verbose; off by default.
    #[serde(default)]
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
/// For full sentence grammar, use `gentufa` instead.
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

/// Output format for `gimfihi` candidate gismu.
#[invariant(::Table => true)]
#[invariant(::Json => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolGimfihiFormat {
    /// Compact ranked table (the default): one row per candidate with score and
    /// per-rafsi collision notes.
    Table,
    /// Full structured JSON of all candidates and their scoring, for
    /// programmatic use.
    Json,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolGimfihiRequest {
    /// The source words, one per language. Provide weights here, or via
    /// `preset`.
    #[serde(default)]
    pub sources: Vec<ToolGimfihiSource>,
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

/// Build the semantic representation of Lojban text as a JSON graph
/// (`lojban-semantics-json-1`): the utterances, eventualities, referents,
/// predications, and formulas that make up its meaning, with full argument
/// structure. This is the deepest analysis jbotci offers — reach for it when you
/// need the actual logical meaning, beyond morphology (`vlasei`) or grammar
/// (`gentufa`). The result is always this JSON graph.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ToolTersmuRequest {
    /// The Lojban text to interpret.
    pub text: String,
    /// Optional dialect selector: a builtin dialect name (e.g. `zantufa`,
    /// `gadganzu`, `ce-ki-tau`) or a parenthesized formula combining them, e.g.
    /// `(cbm ce-ki-tau)`. Omit for standard Lojban.
    #[serde(default)]
    pub dialect: Option<String>,
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
    let tool_format = request.format;
    let show_refs = request
        .show_refs
        .unwrap_or(matches!(tool_format, ToolGentufaFormat::Tree));
    let (format, output_type, content_type) = match tool_format {
        ToolGentufaFormat::Brackets => (GentufaFormat::Brackets, None, TEXT_PLAIN_CONTENT_TYPE),
        ToolGentufaFormat::Tree => (GentufaFormat::Tree, None, TEXT_PLAIN_CONTENT_TYPE),
        ToolGentufaFormat::Raw => (GentufaFormat::Raw, None, TEXT_PLAIN_CONTENT_TYPE),
        ToolGentufaFormat::Json => (GentufaFormat::Json, None, APPLICATION_JSON_CONTENT_TYPE),
        ToolGentufaFormat::Svg => (
            GentufaFormat::Blocks,
            Some(GentufaImageOutputType::Svg),
            "image/svg+xml; charset=utf-8",
        ),
        ToolGentufaFormat::Png => (
            GentufaFormat::Blocks,
            Some(GentufaImageOutputType::Png),
            "image/png",
        ),
    };
    run_tool_command(
        Command::Gentufa(GentufaInput {
            file: None,
            ascii: false,
            detailed_errors: true,
            error_context: 1,
            trace_phase: None,
            trace_limit: None,
            trace_list: false,
            format,
            trace: None,
            dialect: request.dialect,
            show_defs: request.show_defs,
            indent: request.indent,
            mark_stress: None,
            mark_glides: None,
            show_spans: request.show_spans,
            show_refs,
            show_elided: request.show_elided,
            decompose_lujvo: request.decompose_lujvo,
            output_type,
            output_file: None,
            text: vec![request.text],
        }),
        Some(content_type),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_vlasei(request: ToolVlaseiRequest) -> Result<ToolRenderedOutput> {
    let tool_format = request.format;
    let format = match tool_format {
        ToolVlaseiFormat::Brackets => VlaseiFormat::Brackets,
        ToolVlaseiFormat::Tree => VlaseiFormat::Tree,
        ToolVlaseiFormat::Ipa => VlaseiFormat::Ipa,
        ToolVlaseiFormat::Raw => VlaseiFormat::Raw,
        ToolVlaseiFormat::Json => VlaseiFormat::Json,
    };
    let content_type = match tool_format {
        ToolVlaseiFormat::Json => APPLICATION_JSON_CONTENT_TYPE,
        _ => TEXT_PLAIN_CONTENT_TYPE,
    };
    run_tool_command(
        Command::Vlasei(VlaseiInput {
            file: None,
            ascii: false,
            detailed_errors: true,
            trace_phase: None,
            trace_limit: None,
            trace_list: false,
            format,
            trace: None,
            dialect: request.dialect,
            indent: request.indent,
            mark_stress: None,
            mark_glides: None,
            show_spans: request.show_spans,
            decompose_lujvo: request.decompose_lujvo,
            text: vec![request.text],
        }),
        Some(content_type),
    )
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
    let tool_format = request.format;
    let format = match tool_format {
        ToolCuktaFormat::Markdown => CuktaCliFormat::Markdown,
        ToolCuktaFormat::Html => CuktaCliFormat::Html,
        ToolCuktaFormat::Raw => CuktaCliFormat::Raw,
    };
    let content_type = match tool_format {
        ToolCuktaFormat::Markdown => "text/markdown; charset=utf-8",
        ToolCuktaFormat::Html => "text/html; charset=utf-8",
        ToolCuktaFormat::Raw => TEXT_PLAIN_CONTENT_TYPE,
    };
    let query = request.query.unwrap_or_default();
    // The typed `search_result_kinds` set maps directly onto the CLI's per-kind
    // target flags; the CLI's string `targets` channel stays empty. Filters only
    // apply to the search modes and are rejected (downstream) for navigation.
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
        format,
        query: Vec::new(),
    };
    match request.mode {
        ToolCuktaMode::Meaning => input.query = vec![query],
        ToolCuktaMode::Word => input.valsi = Some(query),
        ToolCuktaMode::Section => input.section = Some(query),
        ToolCuktaMode::Example => input.example = Some(query),
        ToolCuktaMode::Toc => input.toc = true,
    }
    run_tool_command_with_context(Command::Cukta(input), Some(content_type), tool_context)
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
    let query = request.query;
    if query.is_empty() {
        bail!("vlacku query must not be empty");
    }
    let (requests, query_text) = match request.mode {
        ToolVlackuMode::Word => (vec![VlackuRequest::valsi(query)], Vec::new()),
        ToolVlackuMode::Rafsi => (vec![VlackuRequest::rafsi(query)], Vec::new()),
        ToolVlackuMode::Lujvo => (vec![VlackuRequest::lujvo(query)], Vec::new()),
        ToolVlackuMode::Sound => (vec![VlackuRequest::sound(query)], Vec::new()),
        ToolVlackuMode::Meaning => (Vec::new(), vec![query]),
    };
    run_tool_command_with_context(
        Command::Vlacku(VlackuInput {
            count: request.count,
            ascii: false,
            word_types: request.word_types,
            min_votes: request.min_votes,
            min_similarity: request.min_similarity,
            sumti_places: CliSumtiPlaces::Index,
            decompose_lujvo: request.decompose_lujvo,
            show_etymology: request.show_etymology,
            requests,
            query: query_text,
        }),
        Some(TEXT_PLAIN_CONTENT_TYPE),
        tool_context,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_tool_jvozba(request: ToolJvozbaRequest) -> Result<ToolRenderedOutput> {
    let sources = request
        .parts
        .into_iter()
        .map(|part| match part.kind {
            ToolJvozbaPartKind::Word => JvozbaSourceInput::Word(part.value),
            ToolJvozbaPartKind::FixedRafsi => JvozbaSourceInput::FixedRafsi(part.value),
        })
        .collect();
    run_tool_command(
        Command::Jvozba(JvozbaInput {
            cmevla: request.mode == ToolJvozbaMode::Cmevla,
            sources,
        }),
        Some(TEXT_PLAIN_CONTENT_TYPE),
    )
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
    let tool_format = request.format;
    let format = match tool_format {
        ToolGimfihiFormat::Table => GimfihiCliFormat::Table,
        ToolGimfihiFormat::Json => GimfihiCliFormat::Json,
    };
    let content_type = match tool_format {
        ToolGimfihiFormat::Json => APPLICATION_JSON_CONTENT_TYPE,
        ToolGimfihiFormat::Table => TEXT_PLAIN_CONTENT_TYPE,
    };
    let sources = request
        .sources
        .into_iter()
        .map(|source| tool_gimfihi_source_to_input(source, word_kind))
        .collect::<Result<Vec<_>>>()?;
    run_tool_command(
        Command::Gimfihi(GimfihiInput {
            sources,
            preset: request.preset,
            shapes: request.shapes,
            check_collisions: request.check_collisions.into(),
            all_letters: request.all_letters,
            show_collisions: request.show_collisions,
            require_free_short_rafsi: request.require_free_short_rafsi,
            count: request.count,
            highlight: request.highlight,
            format,
        }),
        Some(content_type),
    )
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
    run_tool_command(
        Command::Tersmu(TersmuInput {
            file: None,
            format: TersmuFormat::Json,
            trace: None,
            dialect: request.dialect,
            story_time: request.story_time,
            // Default to pretty-printed JSON for readability; `0` opts into compact.
            indent: Some(request.indent.unwrap_or(2)),
            text: vec![request.text],
        }),
        Some(APPLICATION_JSON_CONTENT_TYPE),
    )
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
