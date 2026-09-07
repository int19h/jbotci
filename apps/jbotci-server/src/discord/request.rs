//! Typed Discord request model.
//!
//! Every `/jbotci` subcommand decodes into one variant of [`DiscordRequest`]:
//! the exact source fields the user typed plus the resolved presentation
//! options the customization modal edits. The defaults here are the epic's
//! product decisions (brackets with no image for gentufa, compounds on, glosses
//! and elided terminators off, five cards per page, ...). The whole request,
//! together with the publication metadata in [`PublishedRequest`], is what the
//! published Discord message carries so it can be reopened and re-applied after
//! every local cache is gone.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires, try_new};

/// Maximum size of one editable source field, measured in UTF-16 code units.
///
/// Discord's modal Text Input caps `max_length` and `value` at 4000
/// characters. The platform counts characters the way JavaScript does, so the
/// UTF-16 measure is the conservative bound: it is never smaller than the
/// code-point count and matches what the client enforces for astral-plane
/// input. The slash command advertises and enforces the same bound.
pub(crate) const MAX_SOURCE_UNITS: usize = 4000;

/// Result cards per page for dictionary and ranked searches (PM decision).
pub(crate) const PAGE_SIZE: usize = 5;

/// Highest page number any Discord result can address.
///
/// A String Select holds at most 25 options, so a dedicated page selector can
/// name pages 1..=25. The Vlacku selector shares its options with two detail
/// choices and therefore stops at [`VLACKU_MAX_PAGE`]; both bounds are enforced
/// when the page is chosen, not silently clamped afterwards.
pub(crate) const MAX_PAGE: u8 = 25;
pub(crate) const VLACKU_MAX_PAGE: u8 = 23;

/// UTF-16 code-unit length of `text`, the measure Discord applies to its
/// character limits.
#[requires(true)]
#[ensures(ret >= text.chars().count())]
pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The seven `/jbotci` subcommands.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum DiscordTool {
    Gentufa,
    Vlasei,
    Vlatai,
    Vlacku,
    Cukta,
    Jvozba,
    Gimfihi,
}

impl DiscordTool {
    pub(crate) const ALL: [DiscordTool; 7] = [
        DiscordTool::Gentufa,
        DiscordTool::Vlasei,
        DiscordTool::Vlatai,
        DiscordTool::Vlacku,
        DiscordTool::Cukta,
        DiscordTool::Jvozba,
        DiscordTool::Gimfihi,
    ];

    /// The subcommand name registered with Discord.
    #[requires(true)]
    #[ensures(!ret.is_empty() && ret.is_ascii())]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Gentufa => "gentufa",
            Self::Vlasei => "vlasei",
            Self::Vlatai => "vlatai",
            Self::Vlacku => "vlacku",
            Self::Cukta => "cukta",
            Self::Jvozba => "jvozba",
            Self::Gimfihi => "gimfihi",
        }
    }

    /// One-letter code used inside component custom IDs.
    #[requires(true)]
    #[ensures(ret.is_ascii_lowercase())]
    pub(crate) const fn code(self) -> char {
        match self {
            Self::Gentufa => 'g',
            Self::Vlasei => 'v',
            Self::Vlatai => 't',
            Self::Vlacku => 'k',
            Self::Cukta => 'c',
            Self::Jvozba => 'j',
            Self::Gimfihi => 'f',
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|tool| tool.code() == code))]
    pub(crate) fn from_code(code: char) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.code() == code)
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|tool| tool.name() == name))]
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// Whether jbotci.app has a page for this tool. Only these tools get an
    /// "Open in app" link in their modal; the others get no link component at
    /// all (user decision recorded in #893/#901).
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Gentufa | Self::Vlacku | Self::Cukta | Self::Gimfihi))]
    pub(crate) const fn has_web_page(self) -> bool {
        matches!(
            self,
            Self::Gentufa | Self::Vlacku | Self::Cukta | Self::Gimfihi
        )
    }

    /// The editable source fields of this tool, in canonical order. The first
    /// field is the one the slash command's primary argument fills.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn fields(self) -> &'static [SourceField] {
        match self {
            Self::Gentufa | Self::Vlasei | Self::Vlatai => {
                &[SourceField::Text, SourceField::Dialect]
            }
            Self::Vlacku | Self::Cukta => &[SourceField::Query],
            Self::Jvozba => &[SourceField::Parts, SourceField::FixedRafsi],
            Self::Gimfihi => &[SourceField::Sources],
        }
    }
}

impl fmt::Display for DiscordTool {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One editable text field of a request. Field identity is shared by the slash
/// schema, the input-block codec, the modal builders and the presenters.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SourceField {
    Text,
    Dialect,
    Query,
    Parts,
    FixedRafsi,
    Sources,
}

impl SourceField {
    /// Human-readable label shown above the field in the published message
    /// and used as the field header in the input block.
    #[requires(true)]
    #[ensures(!ret.is_empty() && !ret.contains('\n'))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Dialect => "dialect",
            Self::Query => "query",
            Self::Parts => "parts",
            Self::FixedRafsi => "fixed rafsi",
            Self::Sources => "sources",
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|field| field.label() == label))]
    pub(crate) fn from_label(label: &str) -> Option<Self> {
        [
            Self::Text,
            Self::Dialect,
            Self::Query,
            Self::Parts,
            Self::FixedRafsi,
            Self::Sources,
        ]
        .into_iter()
        .find(|field| field.label() == label)
    }

    /// Byte tag used in the integrity digest.
    #[requires(true)]
    #[ensures(ret.is_ascii_lowercase())]
    pub(crate) const fn digest_tag(self) -> u8 {
        match self {
            Self::Text => b't',
            Self::Dialect => b'd',
            Self::Query => b'q',
            Self::Parts => b'p',
            Self::FixedRafsi => b'r',
            Self::Sources => b's',
        }
    }
}

/// Text accepted for one editable field: exactly what Discord delivered, bounded
/// by [`MAX_SOURCE_UNITS`]. Nothing is trimmed or normalized so the modal can be
/// prefilled with, and the app link can carry, the user's exact input.
#[invariant(utf16_len(text) <= MAX_SOURCE_UNITS, "source text fits one modal text input")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceText {
    text: String,
}

impl SourceText {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|source| source.as_str() == text) || ret.is_err())]
    pub(crate) fn new(text: &str) -> Result<Self, OversizeSource> {
        let units = utf16_len(text);
        if units > MAX_SOURCE_UNITS {
            return Err(new!(OversizeSource {
                units,
                limit: MAX_SOURCE_UNITS,
            }));
        }
        Ok(new!(SourceText {
            text: text.to_owned()
        }))
    }

    #[requires(true)]
    #[ensures(utf16_len(ret) <= MAX_SOURCE_UNITS)]
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

/// A source field exceeded the platform-enforced size.
#[invariant(*units > *limit)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OversizeSource {
    pub(crate) units: usize,
    pub(crate) limit: usize,
}

impl fmt::Display for OversizeSource {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} characters exceed the {}-character limit",
            self.units, self.limit
        )
    }
}

/// A one-based result page.
#[invariant(*value >= 1 && *value <= MAX_PAGE)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct PageNumber {
    value: u8,
}

impl PageNumber {
    /// Page one.
    #[requires(true)]
    #[ensures(ret.get() == 1)]
    pub(crate) fn first() -> Self {
        new!(PageNumber { value: 1 })
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (value >= 1 && value <= MAX_PAGE))]
    pub(crate) fn new(value: u8) -> Option<Self> {
        try_new!(PageNumber { value }).ok()
    }

    #[requires(true)]
    #[ensures(ret >= 1 && ret <= MAX_PAGE)]
    pub(crate) fn get(self) -> u8 {
        self.value
    }

    /// Zero-based index of the first result on this page.
    #[requires(true)]
    #[ensures(ret == (self.get() as usize - 1) * PAGE_SIZE)]
    pub(crate) fn first_index(self) -> usize {
        (usize::from(self.get()) - 1) * PAGE_SIZE
    }
}

/// Monotonic per-message revision, bumped on every successful publication.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Revision {
    value: u32,
}

impl Revision {
    pub(crate) const INITIAL: Revision = Revision { value: 1 };

    #[requires(true)]
    #[ensures(ret.value == value)]
    pub(crate) const fn new(value: u32) -> Self {
        Self { value }
    }

    #[requires(true)]
    #[ensures(ret == self.value)]
    pub(crate) const fn get(self) -> u32 {
        self.value
    }

    /// The revision the next successful update publishes. `None` only at the
    /// (unreachable in practice) counter ceiling, which is reported rather
    /// than wrapped so an old form can never look current again.
    #[requires(true)]
    #[ensures(ret.is_none_or(|next| next.value == self.value + 1))]
    pub(crate) fn next(self) -> Option<Self> {
        self.value.checked_add(1).map(|value| Self { value })
    }
}

/// A Discord snowflake identifier (application, user, message, channel).
#[invariant(!value.is_empty() && value.len() <= 20 && value.bytes().all(|byte| byte.is_ascii_digit()))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Snowflake {
    value: String,
}

impl Snowflake {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|snowflake| snowflake.as_str() == value) || ret.is_err())]
    pub(crate) fn parse(value: &str) -> Result<Self, InvalidSnowflake> {
        try_new!(Snowflake {
            value: value.to_owned()
        })
        .map_err(|_| InvalidSnowflake {
            value: value.to_owned(),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Snowflake {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidSnowflake {
    pub(crate) value: String,
}

impl fmt::Display for InvalidSnowflake {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}` is not a Discord snowflake", self.value)
    }
}

// ---------------------------------------------------------------------------
// Per-tool requests
// ---------------------------------------------------------------------------

/// Which text projection of a syntax parse the gentufa message shows.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum GentufaTextView {
    #[default]
    Brackets,
    Tree,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GentufaOptions {
    pub(crate) view: GentufaTextView,
    /// Attach the block diagram PNG. Off by default; it is an optional addition
    /// to the text result, never an alternative output format.
    pub(crate) include_diagram: bool,
    pub(crate) show_elided: bool,
    /// Coalesce dictionary-attested compounds into one diagram leaf (#892).
    pub(crate) show_compounds: bool,
    /// Add gloss rows to the diagram.
    pub(crate) show_glosses: bool,
}

impl Default for GentufaOptions {
    #[requires(true)]
    #[ensures(ret.view == GentufaTextView::Brackets && !ret.include_diagram)]
    #[ensures(!ret.show_elided && ret.show_compounds && !ret.show_glosses)]
    fn default() -> Self {
        Self {
            view: GentufaTextView::Brackets,
            include_diagram: false,
            show_elided: false,
            show_compounds: true,
            show_glosses: false,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GentufaRequest {
    pub(crate) text: SourceText,
    /// Dialect formula. `None` when never supplied, `Some("")` when the modal
    /// was submitted with the field cleared; both mean standard Lojban.
    pub(crate) dialect: Option<SourceText>,
    pub(crate) options: GentufaOptions,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum VlaseiView {
    #[default]
    Words,
    Brackets,
    Tree,
    Ipa,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct VlaseiOptions {
    pub(crate) view: VlaseiView,
    pub(crate) decompose_lujvo: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VlaseiRequest {
    pub(crate) text: SourceText,
    pub(crate) dialect: Option<SourceText>,
    pub(crate) options: VlaseiOptions,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VlataiOptions {
    /// Mark stress with acute accents in phoneme text (shared default on).
    pub(crate) mark_stress: bool,
    /// Mark glides with breves in phoneme text (shared default on).
    pub(crate) mark_glides: bool,
    /// Show formation details: possible rafsi, lujvo parts, fu'ivla stage.
    pub(crate) show_details: bool,
}

impl Default for VlataiOptions {
    #[requires(true)]
    #[ensures(ret.mark_stress && ret.mark_glides && ret.show_details)]
    fn default() -> Self {
        Self {
            mark_stress: true,
            mark_glides: true,
            show_details: true,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VlataiRequest {
    pub(crate) text: SourceText,
    pub(crate) dialect: Option<SourceText>,
    pub(crate) options: VlataiOptions,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum VlackuMode {
    #[default]
    Word,
    Rafsi,
    Lujvo,
    Sound,
    Meaning,
}

impl VlackuMode {
    pub(crate) const ALL: [VlackuMode; 5] = [
        VlackuMode::Word,
        VlackuMode::Rafsi,
        VlackuMode::Lujvo,
        VlackuMode::Sound,
        VlackuMode::Meaning,
    ];

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn slash_value(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Rafsi => "rafsi",
            Self::Lujvo => "lujvo",
            Self::Sound => "sound",
            Self::Meaning => "meaning",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Word => "Word",
            Self::Rafsi => "Rafsi",
            Self::Lujvo => "Lujvo",
            Self::Sound => "Sound",
            Self::Meaning => "Meaning",
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|mode| mode.slash_value() == value))]
    pub(crate) fn from_slash_value(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.slash_value() == value)
    }
}

/// One selectable dictionary word class. `Brivla` is the union filter the
/// shared search layer already understands.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum VlackuWordType {
    Gismu,
    Lujvo,
    Fuhivla,
    Cmavo,
    Cmevla,
    Brivla,
}

impl VlackuWordType {
    pub(crate) const ALL: [VlackuWordType; 6] = [
        VlackuWordType::Gismu,
        VlackuWordType::Lujvo,
        VlackuWordType::Fuhivla,
        VlackuWordType::Cmavo,
        VlackuWordType::Cmevla,
        VlackuWordType::Brivla,
    ];

    /// The shared search layer's filter spelling (`normalize_word_type_filter`
    /// input).
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn filter_value(self) -> &'static str {
        match self {
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuhivla => "fu'ivla",
            Self::Cmavo => "cmavo",
            Self::Cmevla => "cmevla",
            Self::Brivla => "brivla",
        }
    }

    #[requires(true)]
    #[ensures(ret < 6)]
    const fn bit(self) -> u8 {
        match self {
            Self::Gismu => 0,
            Self::Lujvo => 1,
            Self::Fuhivla => 2,
            Self::Cmavo => 3,
            Self::Cmevla => 4,
            Self::Brivla => 5,
        }
    }
}

/// Set of word-class filters; empty means every class.
#[invariant(*bits < 64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VlackuWordTypeSet {
    bits: u8,
}

impl VlackuWordTypeSet {
    #[requires(true)]
    #[ensures(ret.is_empty())]
    pub(crate) fn empty() -> Self {
        new!(VlackuWordTypeSet { bits: 0 })
    }

    #[requires(true)]
    #[ensures(ret.contains(word_type))]
    pub(crate) fn with(self, word_type: VlackuWordType) -> Self {
        new!(VlackuWordTypeSet {
            bits: self.bits | (1 << word_type.bit()),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn contains(self, word_type: VlackuWordType) -> bool {
        self.bits & (1 << word_type.bit()) != 0
    }

    #[requires(true)]
    #[ensures(ret == (self.bits() == 0))]
    pub(crate) fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn iter(self) -> impl Iterator<Item = VlackuWordType> {
        VlackuWordType::ALL
            .into_iter()
            .filter(move |word_type| self.contains(*word_type))
    }

    #[requires(true)]
    #[ensures(ret < 64)]
    pub(crate) fn bits(self) -> u8 {
        self.bits
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (bits < 64))]
    pub(crate) fn from_bits(bits: u8) -> Option<Self> {
        try_new!(VlackuWordTypeSet { bits }).ok()
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VlackuOptions {
    pub(crate) mode: VlackuMode,
    pub(crate) word_types: VlackuWordTypeSet,
    /// Show the rafsi decomposition of lujvo results (shared default on).
    pub(crate) decompose_lujvo: bool,
    pub(crate) show_etymology: bool,
    pub(crate) page: PageNumber,
}

impl Default for VlackuOptions {
    #[requires(true)]
    #[ensures(ret.mode == VlackuMode::Word && ret.word_types.is_empty())]
    #[ensures(ret.decompose_lujvo && !ret.show_etymology && ret.page.get() == 1)]
    fn default() -> Self {
        Self {
            mode: VlackuMode::Word,
            word_types: VlackuWordTypeSet::empty(),
            decompose_lujvo: true,
            show_etymology: false,
            page: PageNumber::first(),
        }
    }
}

#[invariant(options.page.get() <= VLACKU_MAX_PAGE)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VlackuRequest {
    pub(crate) query: SourceText,
    pub(crate) options: VlackuOptions,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CuktaMode {
    Meaning,
    Word,
    Section,
    Example,
    Contents,
}

impl CuktaMode {
    pub(crate) const ALL: [CuktaMode; 5] = [
        CuktaMode::Meaning,
        CuktaMode::Word,
        CuktaMode::Section,
        CuktaMode::Example,
        CuktaMode::Contents,
    ];

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn slash_value(self) -> &'static str {
        match self {
            Self::Meaning => "meaning",
            Self::Word => "word",
            Self::Section => "section",
            Self::Example => "example",
            Self::Contents => "contents",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Meaning => "Meaning search",
            Self::Word => "Word search",
            Self::Section => "Section",
            Self::Example => "Example",
            Self::Contents => "Contents",
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|mode| mode.slash_value() == value))]
    pub(crate) fn from_slash_value(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.slash_value() == value)
    }

    /// Whether the mode needs a query/reference to run.
    #[requires(true)]
    #[ensures(ret == !matches!(self, Self::Contents))]
    pub(crate) const fn requires_query(self) -> bool {
        !matches!(self, Self::Contents)
    }

    /// Whether the mode is a search whose result-kind filter and page apply.
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Meaning | Self::Word))]
    pub(crate) const fn is_search(self) -> bool {
        matches!(self, Self::Meaning | Self::Word)
    }
}

/// CLL search result kinds; empty means every kind.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum CuktaResultKind {
    Section,
    Paragraph,
    Example,
}

impl CuktaResultKind {
    pub(crate) const ALL: [CuktaResultKind; 3] = [
        CuktaResultKind::Section,
        CuktaResultKind::Paragraph,
        CuktaResultKind::Example,
    ];

    #[requires(true)]
    #[ensures(ret < 3)]
    const fn bit(self) -> u8 {
        match self {
            Self::Section => 0,
            Self::Paragraph => 1,
            Self::Example => 2,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Section => "Sections",
            Self::Paragraph => "Paragraphs",
            Self::Example => "Examples",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn value(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Paragraph => "paragraph",
            Self::Example => "example",
        }
    }
}

#[invariant(*bits < 8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CuktaResultKindSet {
    bits: u8,
}

impl CuktaResultKindSet {
    #[requires(true)]
    #[ensures(ret.is_empty())]
    pub(crate) fn empty() -> Self {
        new!(CuktaResultKindSet { bits: 0 })
    }

    #[requires(true)]
    #[ensures(ret.contains(kind))]
    pub(crate) fn with(self, kind: CuktaResultKind) -> Self {
        new!(CuktaResultKindSet {
            bits: self.bits | (1 << kind.bit()),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn contains(self, kind: CuktaResultKind) -> bool {
        self.bits & (1 << kind.bit()) != 0
    }

    #[requires(true)]
    #[ensures(ret == (self.bits() == 0))]
    pub(crate) fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[requires(true)]
    #[ensures(ret < 8)]
    pub(crate) fn bits(self) -> u8 {
        self.bits
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (bits < 8))]
    pub(crate) fn from_bits(bits: u8) -> Option<Self> {
        try_new!(CuktaResultKindSet { bits }).ok()
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CuktaOptions {
    pub(crate) mode: CuktaMode,
    pub(crate) kinds: CuktaResultKindSet,
    pub(crate) page: PageNumber,
}

/// The mode is always the resolved one: an explicit slash/modal choice, else
/// meaning search when a query was given, else contents.
///
/// A mode that needs a query may be recorded without one: asking for a search
/// and not saying what to search for is an incomplete task, which the reader
/// completes in the form, not a request that cannot exist. Running it is what
/// requires the query, and that is checked where the request is run.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuktaRequest {
    pub(crate) query: Option<SourceText>,
    pub(crate) options: CuktaOptions,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum JvozbaTarget {
    #[default]
    Lujvo,
    Cmevla,
}

impl JvozbaTarget {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn slash_value(self) -> &'static str {
        match self {
            Self::Lujvo => "lujvo",
            Self::Cmevla => "cmevla",
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|target| target.slash_value() == value))]
    pub(crate) fn from_slash_value(value: &str) -> Option<Self> {
        [Self::Lujvo, Self::Cmevla]
            .into_iter()
            .find(|target| target.slash_value() == value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct JvozbaOptions {
    pub(crate) target: JvozbaTarget,
}

/// Rewrite a message published before the ordered parts syntax. The old
/// builder appended every fixed rafsi after all of the words, so writing them
/// as trailing `-rafsi-` spans reproduces exactly the sequence that message
/// would have built. A message with no such field is already ordered and is
/// returned unchanged.
#[requires(true)]
#[ensures(ret.is_err() || old(legacy.is_none()) -> ret.as_ref().is_ok_and(|text| *text == old(parts.clone())))]
fn jvozba_parts_with_legacy_rafsi(
    parts: SourceText,
    legacy: Option<SourceText>,
) -> Result<SourceText, RequestStateError> {
    let Some(legacy) = legacy else {
        return Ok(parts);
    };
    let mut combined = parts.as_str().to_owned();
    for piece in legacy
        .as_str()
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|piece| !piece.is_empty())
    {
        if !combined.is_empty() {
            combined.push(' ');
        }
        combined.push('-');
        combined.push_str(piece);
        combined.push('-');
    }
    SourceText::new(&combined).map_err(|_| RequestStateError::LegacyPartsTooLong {
        tool: DiscordTool::Jvozba,
        units: utf16_len(&combined),
    })
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JvozbaRequest {
    /// The pieces in the order the reader wrote them: words to look up, and
    /// rafsi given literally between hyphens, as in `blanu -blo- zdani`.
    pub(crate) parts: SourceText,
    pub(crate) options: JvozbaOptions,
}

pub(crate) use jbotci_gimfihi::{CollisionScope, GimfihiPreset, GismuShape};

/// Candidate letter shapes; empty means both standard gismu shapes.
#[invariant(*bits < 4)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GismuShapeSet {
    bits: u8,
}

impl GismuShapeSet {
    #[requires(true)]
    #[ensures(ret.is_empty())]
    pub(crate) fn empty() -> Self {
        new!(GismuShapeSet { bits: 0 })
    }

    #[requires(true)]
    #[ensures(ret.contains(shape))]
    pub(crate) fn with(self, shape: GismuShape) -> Self {
        new!(GismuShapeSet {
            bits: self.bits | (1 << Self::bit(shape)),
        })
    }

    #[requires(true)]
    #[ensures(ret < 2)]
    const fn bit(shape: GismuShape) -> u8 {
        match shape {
            GismuShape::Ccvcv => 0,
            GismuShape::Cvccv => 1,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn contains(self, shape: GismuShape) -> bool {
        self.bits & (1 << Self::bit(shape)) != 0
    }

    #[requires(true)]
    #[ensures(ret == (self.bits() == 0))]
    pub(crate) fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[requires(true)]
    #[ensures(ret < 4)]
    pub(crate) fn bits(self) -> u8 {
        self.bits
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (bits < 4))]
    pub(crate) fn from_bits(bits: u8) -> Option<Self> {
        try_new!(GismuShapeSet { bits }).ok()
    }

    /// The shapes to generate, in the shared layer's canonical order.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn shapes(self) -> Vec<GismuShape> {
        let selected = [GismuShape::Ccvcv, GismuShape::Cvccv]
            .into_iter()
            .filter(|shape| self.contains(*shape))
            .collect::<Vec<_>>();
        if selected.is_empty() {
            jbotci_gimfihi::default_shapes()
        } else {
            selected
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GimfihiOptions {
    pub(crate) preset: Option<GimfihiPreset>,
    pub(crate) shapes: GismuShapeSet,
    pub(crate) collisions: CollisionScope,
    pub(crate) show_collisions: bool,
    pub(crate) all_letters: bool,
    pub(crate) require_free_short_rafsi: bool,
    pub(crate) page: PageNumber,
}

impl Default for GimfihiOptions {
    #[requires(true)]
    #[ensures(ret.preset.is_none() && ret.collisions == CollisionScope::All)]
    // Both standard shapes, named rather than implied: the form shows them
    // both checked, and a submission that changes nothing must mean what it
    // was opened from.
    #[ensures(ret.shapes.contains(GismuShape::Ccvcv) && ret.shapes.contains(GismuShape::Cvccv))]
    #[ensures(!ret.show_collisions && !ret.all_letters && !ret.require_free_short_rafsi)]
    #[ensures(ret.page.get() == 1)]
    fn default() -> Self {
        Self {
            preset: None,
            shapes: GismuShapeSet::empty()
                .with(GismuShape::Ccvcv)
                .with(GismuShape::Cvccv),
            collisions: CollisionScope::All,
            show_collisions: false,
            all_letters: false,
            require_free_short_rafsi: false,
            page: PageNumber::first(),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GimfihiRequest {
    /// `LANG[:WEIGHT]:WORD` records separated by commas, semicolons or newlines.
    /// `None` when the slash command omitted them (the shared default source
    /// configuration applies), `Some("")` after an explicit clear.
    pub(crate) sources: Option<SourceText>,
    pub(crate) options: GimfihiOptions,
}

/// One decoded `/jbotci` request.
#[invariant(::Gentufa(_) => true)]
#[invariant(::Vlasei(_) => true)]
#[invariant(::Vlatai(_) => true)]
#[invariant(::Vlacku(_) => true)]
#[invariant(::Cukta(_) => true)]
#[invariant(::Jvozba(_) => true)]
#[invariant(::Gimfihi(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiscordRequest {
    Gentufa(GentufaRequest),
    Vlasei(VlaseiRequest),
    Vlatai(VlataiRequest),
    Vlacku(VlackuRequest),
    Cukta(CuktaRequest),
    Jvozba(JvozbaRequest),
    Gimfihi(GimfihiRequest),
}

impl DiscordRequest {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn tool(&self) -> DiscordTool {
        match self {
            Self::Gentufa(_) => DiscordTool::Gentufa,
            Self::Vlasei(_) => DiscordTool::Vlasei,
            Self::Vlatai(_) => DiscordTool::Vlatai,
            Self::Vlacku(_) => DiscordTool::Vlacku,
            Self::Cukta(_) => DiscordTool::Cukta,
            Self::Jvozba(_) => DiscordTool::Jvozba,
            Self::Gimfihi(_) => DiscordTool::Gimfihi,
        }
    }

    /// The result page for page-able tools; page one otherwise.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn page(&self) -> PageNumber {
        match self {
            Self::Vlacku(request) => request.options.page,
            Self::Cukta(request) => request.options.page,
            Self::Gimfihi(request) => request.options.page,
            Self::Gentufa(_) | Self::Vlasei(_) | Self::Vlatai(_) | Self::Jvozba(_) => {
                PageNumber::first()
            }
        }
    }

    /// Every source field of the tool, present or not, in canonical order.
    #[requires(true)]
    #[ensures(ret.len() == self.tool().fields().len())]
    #[ensures(ret.iter().zip(self.tool().fields()).all(|((field, _), expected)| field == expected))]
    pub(crate) fn source_fields(&self) -> Vec<(SourceField, Option<&SourceText>)> {
        match self {
            Self::Gentufa(request) => vec![
                (SourceField::Text, Some(&request.text)),
                (SourceField::Dialect, request.dialect.as_ref()),
            ],
            Self::Vlasei(request) => vec![
                (SourceField::Text, Some(&request.text)),
                (SourceField::Dialect, request.dialect.as_ref()),
            ],
            Self::Vlatai(request) => vec![
                (SourceField::Text, Some(&request.text)),
                (SourceField::Dialect, request.dialect.as_ref()),
            ],
            Self::Vlacku(request) => vec![(SourceField::Query, Some(&request.query))],
            Self::Cukta(request) => vec![(SourceField::Query, request.query.as_ref())],
            Self::Jvozba(request) => vec![
                (SourceField::Parts, Some(&request.parts)),
                // Nothing is published in this field any more; it is kept in
                // the layout so a message from before the ordered syntax
                // still decodes, and its value is folded into the parts.
                (SourceField::FixedRafsi, None),
            ],
            Self::Gimfihi(request) => vec![(SourceField::Sources, request.sources.as_ref())],
        }
    }

    /// The options packed into the custom-id options word. Bit layouts are
    /// documented on [`options_word_mask`]; unknown bits are rejected by
    /// [`DiscordRequest::from_parts`].
    #[requires(true)]
    #[ensures(ret & !options_word_mask(self.tool()) == 0)]
    pub(crate) fn options_word(&self) -> u32 {
        match self {
            Self::Gentufa(request) => {
                let options = request.options;
                u32::from(options.view == GentufaTextView::Tree)
                    | u32::from(options.include_diagram) << 1
                    | u32::from(options.show_elided) << 2
                    | u32::from(options.show_compounds) << 3
                    | u32::from(options.show_glosses) << 4
            }
            Self::Vlasei(request) => {
                let options = request.options;
                let view = match options.view {
                    VlaseiView::Words => 0,
                    VlaseiView::Brackets => 1,
                    VlaseiView::Tree => 2,
                    VlaseiView::Ipa => 3,
                };
                view | u32::from(options.decompose_lujvo) << 2
            }
            Self::Vlatai(request) => {
                let options = request.options;
                u32::from(options.mark_stress)
                    | u32::from(options.mark_glides) << 1
                    | u32::from(options.show_details) << 2
            }
            Self::Vlacku(request) => {
                let options = request.options;
                let mode = match options.mode {
                    VlackuMode::Word => 0,
                    VlackuMode::Rafsi => 1,
                    VlackuMode::Lujvo => 2,
                    VlackuMode::Sound => 3,
                    VlackuMode::Meaning => 4,
                };
                mode | u32::from(options.word_types.bits()) << 3
                    | u32::from(options.decompose_lujvo) << 9
                    | u32::from(options.show_etymology) << 10
            }
            Self::Cukta(request) => {
                let options = request.options;
                let mode = match options.mode {
                    CuktaMode::Meaning => 0,
                    CuktaMode::Word => 1,
                    CuktaMode::Section => 2,
                    CuktaMode::Example => 3,
                    CuktaMode::Contents => 4,
                };
                mode | u32::from(options.kinds.bits()) << 3
            }
            Self::Jvozba(request) => u32::from(request.options.target == JvozbaTarget::Cmevla),
            Self::Gimfihi(request) => {
                let options = request.options;
                let preset = options.preset.map_or(0, |preset| preset_code(preset));
                let collisions = match options.collisions {
                    CollisionScope::All => 0,
                    CollisionScope::Official => 1,
                    CollisionScope::None => 2,
                };
                preset
                    | u32::from(options.shapes.bits()) << 4
                    | collisions << 6
                    | u32::from(options.show_collisions) << 8
                    | u32::from(options.all_letters) << 9
                    | u32::from(options.require_free_short_rafsi) << 10
            }
        }
    }

    /// Rebuild a request from its tool, options word, page and source fields
    /// (the four things the published message carries). Every unknown bit,
    /// enum value, page bound violation or missing required field is an error.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|request| request.tool() == tool) || ret.is_err())]
    pub(crate) fn from_parts(
        tool: DiscordTool,
        options_word: u32,
        page: PageNumber,
        mut fields: Vec<(SourceField, Option<SourceText>)>,
    ) -> Result<Self, RequestStateError> {
        if options_word & !options_word_mask(tool) != 0 {
            return Err(RequestStateError::UnknownOptionBits { tool, options_word });
        }
        let expected = tool.fields();
        if fields.len() != expected.len()
            || fields
                .iter()
                .zip(expected)
                .any(|((field, _), expected)| field != expected)
        {
            return Err(RequestStateError::FieldLayout { tool });
        }
        let mut take = |field: SourceField| -> Option<SourceText> {
            fields
                .iter_mut()
                .find(|(candidate, _)| *candidate == field)
                .and_then(|(_, value)| value.take())
        };
        let flag = |bit: u32| options_word & (1 << bit) != 0;
        let required = |value: Option<SourceText>, field: SourceField| {
            value.ok_or(RequestStateError::MissingField { tool, field })
        };
        let request = match tool {
            DiscordTool::Gentufa => Self::Gentufa(GentufaRequest {
                text: required(take(SourceField::Text), SourceField::Text)?,
                dialect: take(SourceField::Dialect),
                options: GentufaOptions {
                    view: if flag(0) {
                        GentufaTextView::Tree
                    } else {
                        GentufaTextView::Brackets
                    },
                    include_diagram: flag(1),
                    show_elided: flag(2),
                    show_compounds: flag(3),
                    show_glosses: flag(4),
                },
            }),
            DiscordTool::Vlasei => Self::Vlasei(VlaseiRequest {
                text: required(take(SourceField::Text), SourceField::Text)?,
                dialect: take(SourceField::Dialect),
                options: VlaseiOptions {
                    view: match options_word & 0b11 {
                        0 => VlaseiView::Words,
                        1 => VlaseiView::Brackets,
                        2 => VlaseiView::Tree,
                        _ => VlaseiView::Ipa,
                    },
                    decompose_lujvo: flag(2),
                },
            }),
            DiscordTool::Vlatai => Self::Vlatai(VlataiRequest {
                text: required(take(SourceField::Text), SourceField::Text)?,
                dialect: take(SourceField::Dialect),
                options: VlataiOptions {
                    mark_stress: flag(0),
                    mark_glides: flag(1),
                    show_details: flag(2),
                },
            }),
            DiscordTool::Vlacku => {
                let mode = match options_word & 0b111 {
                    0 => VlackuMode::Word,
                    1 => VlackuMode::Rafsi,
                    2 => VlackuMode::Lujvo,
                    3 => VlackuMode::Sound,
                    4 => VlackuMode::Meaning,
                    _ => {
                        return Err(RequestStateError::UnknownEnumValue {
                            tool,
                            what: "vlacku mode",
                        });
                    }
                };
                let word_types =
                    VlackuWordTypeSet::from_bits(((options_word >> 3) & 0b11_1111) as u8)
                        .expect("six masked bits form a word-type set");
                if page.get() > VLACKU_MAX_PAGE {
                    return Err(RequestStateError::PageOutOfRange {
                        tool,
                        page: page.get(),
                        max: VLACKU_MAX_PAGE,
                    });
                }
                let query = required(take(SourceField::Query), SourceField::Query)?;
                Self::Vlacku(new!(VlackuRequest {
                    query,
                    options: VlackuOptions {
                        mode,
                        word_types,
                        decompose_lujvo: flag(9),
                        show_etymology: flag(10),
                        page,
                    },
                }))
            }
            DiscordTool::Cukta => {
                let mode = match options_word & 0b111 {
                    0 => CuktaMode::Meaning,
                    1 => CuktaMode::Word,
                    2 => CuktaMode::Section,
                    3 => CuktaMode::Example,
                    4 => CuktaMode::Contents,
                    _ => {
                        return Err(RequestStateError::UnknownEnumValue {
                            tool,
                            what: "cukta mode",
                        });
                    }
                };
                let kinds = CuktaResultKindSet::from_bits(((options_word >> 3) & 0b111) as u8)
                    .expect("three masked bits form a result-kind set");
                // A mode that needs a query may have been published without
                // one: that is an incomplete task the reader finishes in the
                // form, and it must come back exactly as it was published.
                let query = take(SourceField::Query);
                Self::Cukta(CuktaRequest {
                    query,
                    options: CuktaOptions { mode, kinds, page },
                })
            }
            DiscordTool::Jvozba => Self::Jvozba(JvozbaRequest {
                parts: jvozba_parts_with_legacy_rafsi(
                    required(take(SourceField::Parts), SourceField::Parts)?,
                    take(SourceField::FixedRafsi),
                )?,
                options: JvozbaOptions {
                    target: if flag(0) {
                        JvozbaTarget::Cmevla
                    } else {
                        JvozbaTarget::Lujvo
                    },
                },
            }),
            DiscordTool::Gimfihi => {
                let preset = match options_word & 0b1111 {
                    0 => None,
                    code => Some(preset_from_code(code).ok_or(
                        RequestStateError::UnknownEnumValue {
                            tool,
                            what: "gimfihi preset",
                        },
                    )?),
                };
                let shapes = GismuShapeSet::from_bits(((options_word >> 4) & 0b11) as u8)
                    .expect("two masked bits form a shape set");
                let collisions = match (options_word >> 6) & 0b11 {
                    0 => CollisionScope::All,
                    1 => CollisionScope::Official,
                    2 => CollisionScope::None,
                    _ => {
                        return Err(RequestStateError::UnknownEnumValue {
                            tool,
                            what: "collision scope",
                        });
                    }
                };
                Self::Gimfihi(GimfihiRequest {
                    sources: take(SourceField::Sources),
                    options: GimfihiOptions {
                        preset,
                        shapes,
                        collisions,
                        show_collisions: flag(8),
                        all_letters: flag(9),
                        require_free_short_rafsi: flag(10),
                        page,
                    },
                })
            }
        };
        Ok(request)
    }
}

/// Bits a tool's options word may use. Anything outside is an unknown option
/// from a different schema and must fail decoding. Layout:
///
/// | tool    | bits                                                                   |
/// |---------|------------------------------------------------------------------------|
/// | gentufa | 0 tree view, 1 diagram, 2 elided, 3 compounds, 4 glosses               |
/// | vlasei  | 0-1 view (words/brackets/tree/ipa), 2 decompose lujvo                  |
/// | vlatai  | 0 stress, 1 glides, 2 details                                          |
/// | vlacku  | 0-2 mode, 3-8 word-type set, 9 decompose, 10 etymology                 |
/// | cukta   | 0-2 mode, 3-5 result-kind set                                          |
/// | jvozba  | 0 cmevla target                                                        |
/// | gimfihi | 0-3 preset (0 none), 4-5 shape set, 6-7 collisions, 8 show collisions, |
/// |         | 9 all letters, 10 require free short rafsi                             |
#[requires(true)]
#[ensures(ret != 0)]
pub(crate) const fn options_word_mask(tool: DiscordTool) -> u32 {
    match tool {
        DiscordTool::Gentufa => 0b1_1111,
        DiscordTool::Vlasei => 0b111,
        DiscordTool::Vlatai => 0b111,
        DiscordTool::Vlacku => 0b111_1111_1111,
        DiscordTool::Cukta => 0b11_1111,
        DiscordTool::Jvozba => 0b1,
        DiscordTool::Gimfihi => 0b111_1111_1111,
    }
}

#[requires(true)]
#[ensures(ret >= 1 && ret <= 9)]
fn preset_code(preset: GimfihiPreset) -> u32 {
    match preset {
        GimfihiPreset::Data1985 => 1,
        GimfihiPreset::Data1987 => 2,
        GimfihiPreset::Data1994 => 3,
        GimfihiPreset::Data1995 => 4,
        GimfihiPreset::Data1999 => 5,
        GimfihiPreset::Evenly => 6,
        GimfihiPreset::Ilmen6 => 7,
        GimfihiPreset::Ilmen8 => 8,
        GimfihiPreset::Ilmen12 => 9,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|preset| preset_code(preset) == code))]
fn preset_from_code(code: u32) -> Option<GimfihiPreset> {
    jbotci_gimfihi::all_presets()
        .iter()
        .copied()
        .find(|preset| preset_code(*preset) == code)
}

/// Why a published request could not be rebuilt.
#[invariant(::UnknownOptionBits { .. } => true)]
#[invariant(::UnknownEnumValue { .. } => true)]
#[invariant(::FieldLayout { .. } => true)]
#[invariant(::MissingField { .. } => true)]
#[invariant(::PageOutOfRange { .. } => true)]
#[invariant(::LegacyPartsTooLong { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RequestStateError {
    UnknownOptionBits {
        tool: DiscordTool,
        options_word: u32,
    },
    UnknownEnumValue {
        tool: DiscordTool,
        what: &'static str,
    },
    FieldLayout {
        tool: DiscordTool,
    },
    MissingField {
        tool: DiscordTool,
        field: SourceField,
    },
    PageOutOfRange {
        tool: DiscordTool,
        page: u8,
        max: u8,
    },
    /// A message published before the ordered parts syntax cannot be
    /// rewritten into it, because the two old fields together are longer than
    /// one field may be.
    LegacyPartsTooLong {
        tool: DiscordTool,
        units: usize,
    },
}

impl fmt::Display for RequestStateError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOptionBits { tool, options_word } => write!(
                formatter,
                "{tool}: options word {options_word:#x} uses bits this version does not know"
            ),
            Self::UnknownEnumValue { tool, what } => {
                write!(formatter, "{tool}: unknown {what} value")
            }
            Self::LegacyPartsTooLong { tool, units } => write!(
                formatter,
                "{tool}: this result was published before the ordered parts syntax, and its \
                 words and fixed rafsi together need {units} characters, more than one field \
                 holds. Run the command again with the parts in one field."
            ),
            Self::FieldLayout { tool } => {
                write!(formatter, "{tool}: source fields do not match the tool")
            }
            Self::MissingField { tool, field } => {
                write!(formatter, "{tool}: the {} field is missing", field.label())
            }
            Self::PageOutOfRange { tool, page, max } => {
                write!(
                    formatter,
                    "{tool}: page {page} exceeds the maximum of {max}"
                )
            }
        }
    }
}

impl std::error::Error for RequestStateError {}

/// Where the published message keeps the source fields.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SourceStore {
    /// The deliberate input Text Display inside the Section.
    Inline,
    /// The readable `jbotci-input.txt` attachment shown by a File component.
    Attachment,
}

impl SourceStore {
    #[requires(true)]
    #[ensures(ret.is_ascii_lowercase())]
    pub(crate) const fn code(self) -> char {
        match self {
            Self::Inline => 'i',
            Self::Attachment => 'a',
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|store| store.code() == code))]
    pub(crate) fn from_code(code: char) -> Option<Self> {
        [Self::Inline, Self::Attachment]
            .into_iter()
            .find(|store| store.code() == code)
    }
}

/// A request together with the publication metadata the message carries.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublishedRequest {
    pub(crate) request: DiscordRequest,
    pub(crate) revision: Revision,
    /// The user who ran the slash command; only they may apply changes.
    pub(crate) initiator: Snowflake,
    /// Build that produced the published result, so a later apply can note
    /// when the analysis version changed.
    pub(crate) build_tag: BuildTag,
}

/// Short identifier of the build that produced a result: the git commit when
/// the build knew it, else the crate version. Restricted to characters that
/// are safe inside a custom ID field.
#[invariant(!value.is_empty() && value.len() <= 16 && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BuildTag {
    value: String,
}

impl BuildTag {
    /// The running server's build tag.
    #[requires(true)]
    #[ensures(!ret.value.is_empty())]
    pub(crate) fn current() -> Self {
        let value = option_env!("JBOTCI_GIT_COMMIT_SHORT")
            .filter(|commit| !commit.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION").replace('.', "_")));
        Self::parse(&value).expect("build tag is alphanumeric")
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|tag| tag.as_str() == value) || ret.is_err())]
    pub(crate) fn parse(value: &str) -> Result<Self, InvalidBuildTag> {
        try_new!(BuildTag {
            value: value.to_owned()
        })
        .map_err(|_| InvalidBuildTag)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidBuildTag;

impl fmt::Display for InvalidBuildTag {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("build tag must be 1-16 alphanumeric, `-` or `_` characters")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn text(value: &str) -> SourceText {
        SourceText::new(value).expect("test text fits")
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn sample_requests() -> Vec<DiscordRequest> {
        vec![
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi klama"),
                dialect: None,
                options: GentufaOptions::default(),
            }),
            DiscordRequest::Gentufa(GentufaRequest {
                text: text("mi pa moi klama"),
                dialect: Some(text("(cbm ce-ki-tau)")),
                options: GentufaOptions {
                    view: GentufaTextView::Tree,
                    include_diagram: true,
                    show_elided: true,
                    show_compounds: false,
                    show_glosses: true,
                },
            }),
            DiscordRequest::Vlasei(VlaseiRequest {
                text: text("coi"),
                dialect: Some(text("")),
                options: VlaseiOptions {
                    view: VlaseiView::Ipa,
                    decompose_lujvo: true,
                },
            }),
            DiscordRequest::Vlatai(VlataiRequest {
                text: text("klama"),
                dialect: None,
                options: VlataiOptions {
                    mark_stress: false,
                    mark_glides: true,
                    show_details: false,
                },
            }),
            DiscordRequest::Vlacku(new!(VlackuRequest {
                query: text("kla*"),
                options: VlackuOptions {
                    mode: VlackuMode::Meaning,
                    word_types: VlackuWordTypeSet::empty()
                        .with(VlackuWordType::Gismu)
                        .with(VlackuWordType::Brivla),
                    decompose_lujvo: false,
                    show_etymology: true,
                    page: PageNumber::new(VLACKU_MAX_PAGE).expect("page"),
                },
            })),
            DiscordRequest::Cukta(CuktaRequest {
                query: None,
                options: CuktaOptions {
                    mode: CuktaMode::Contents,
                    kinds: CuktaResultKindSet::empty(),
                    page: PageNumber::first(),
                },
            }),
            DiscordRequest::Cukta(CuktaRequest {
                query: Some(text("tanru")),
                options: CuktaOptions {
                    mode: CuktaMode::Word,
                    kinds: CuktaResultKindSet::empty().with(CuktaResultKind::Example),
                    page: PageNumber::new(MAX_PAGE).expect("page"),
                },
            }),
            DiscordRequest::Jvozba(JvozbaRequest {
                parts: text("klama bajra"),
                options: JvozbaOptions {
                    target: JvozbaTarget::Cmevla,
                },
            }),
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: None,
                options: GimfihiOptions::default(),
            }),
            DiscordRequest::Gimfihi(GimfihiRequest {
                sources: Some(text("eng:go, spa:[ir]")),
                options: GimfihiOptions {
                    preset: Some(GimfihiPreset::Ilmen12),
                    shapes: GismuShapeSet::empty().with(GismuShape::Cvccv),
                    collisions: CollisionScope::None,
                    show_collisions: true,
                    all_letters: true,
                    require_free_short_rafsi: true,
                    page: PageNumber::new(7).expect("page"),
                },
            }),
        ]
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn options_words_round_trip_through_from_parts() {
        for request in sample_requests() {
            let fields = request
                .source_fields()
                .into_iter()
                .map(|(field, value)| (field, value.cloned()))
                .collect();
            let rebuilt = DiscordRequest::from_parts(
                request.tool(),
                request.options_word(),
                request.page(),
                fields,
            )
            .expect("request rebuilds");
            assert_eq!(rebuilt, request);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_option_bits_and_enum_values_fail() {
        for tool in DiscordTool::ALL {
            let fields = tool
                .fields()
                .iter()
                .map(|field| (*field, Some(text("x"))))
                .collect::<Vec<_>>();
            let bad = options_word_mask(tool) + 1;
            let error = DiscordRequest::from_parts(tool, bad, PageNumber::first(), fields)
                .expect_err("unknown bits fail");
            assert!(
                matches!(error, RequestStateError::UnknownOptionBits { .. }),
                "{tool}"
            );
        }
        let vlacku_fields = vec![(SourceField::Query, Some(text("x")))];
        assert!(matches!(
            DiscordRequest::from_parts(
                DiscordTool::Vlacku,
                0b111,
                PageNumber::first(),
                vlacku_fields
            ),
            Err(RequestStateError::UnknownEnumValue {
                what: "vlacku mode",
                ..
            })
        ));
        let gimfihi_fields = vec![(SourceField::Sources, None)];
        assert!(matches!(
            DiscordRequest::from_parts(
                DiscordTool::Gimfihi,
                0b1111,
                PageNumber::first(),
                gimfihi_fields
            ),
            Err(RequestStateError::UnknownEnumValue {
                what: "gimfihi preset",
                ..
            })
        ));
        let gimfihi_fields = vec![(SourceField::Sources, None)];
        assert!(matches!(
            DiscordRequest::from_parts(
                DiscordTool::Gimfihi,
                0b11 << 6,
                PageNumber::first(),
                gimfihi_fields
            ),
            Err(RequestStateError::UnknownEnumValue {
                what: "collision scope",
                ..
            })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn missing_required_fields_and_page_bounds_fail() {
        let error = DiscordRequest::from_parts(
            DiscordTool::Gentufa,
            0,
            PageNumber::first(),
            vec![(SourceField::Text, None), (SourceField::Dialect, None)],
        )
        .expect_err("gentufa needs text");
        assert!(matches!(
            error,
            RequestStateError::MissingField {
                field: SourceField::Text,
                ..
            }
        ));
        // Contents needs no query; every other cukta mode does.
        assert!(
            DiscordRequest::from_parts(
                DiscordTool::Cukta,
                4,
                PageNumber::first(),
                vec![(SourceField::Query, None)]
            )
            .is_ok()
        );
        // A search mode published without its query is an incomplete task,
        // and it comes back exactly as it was published so the form can
        // finish it.
        let DiscordRequest::Cukta(incomplete) = DiscordRequest::from_parts(
            DiscordTool::Cukta,
            0,
            PageNumber::first(),
            vec![(SourceField::Query, None)],
        )
        .expect("an incomplete task rebuilds") else {
            panic!("cukta request");
        };
        assert_eq!(incomplete.options.mode, CuktaMode::Meaning);
        assert_eq!(incomplete.query, None);
        let too_far = PageNumber::new(VLACKU_MAX_PAGE + 1).expect("page 24 exists");
        assert!(matches!(
            DiscordRequest::from_parts(
                DiscordTool::Vlacku,
                0,
                too_far,
                vec![(SourceField::Query, Some(text("x")))]
            ),
            Err(RequestStateError::PageOutOfRange {
                max: VLACKU_MAX_PAGE,
                ..
            })
        ));
        assert!(matches!(
            DiscordRequest::from_parts(
                DiscordTool::Jvozba,
                0,
                PageNumber::first(),
                vec![(SourceField::Parts, Some(text("x")))]
            ),
            Err(RequestStateError::FieldLayout { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_text_enforces_the_utf16_bound() {
        let ascii = "a".repeat(MAX_SOURCE_UNITS);
        assert!(SourceText::new(&ascii).is_ok());
        assert!(SourceText::new(&format!("{ascii}a")).is_err());
        // One astral-plane character costs two UTF-16 units.
        let astral = "😀".repeat(MAX_SOURCE_UNITS / 2);
        assert!(SourceText::new(&astral).is_ok());
        let error = SourceText::new(&format!("{astral}a")).expect_err("over by one unit");
        assert_eq!(error.units, MAX_SOURCE_UNITS + 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tool_codes_and_names_are_distinct_and_reversible() {
        for tool in DiscordTool::ALL {
            assert_eq!(DiscordTool::from_code(tool.code()), Some(tool));
            assert_eq!(DiscordTool::from_name(tool.name()), Some(tool));
        }
        let codes = DiscordTool::ALL
            .iter()
            .map(|tool| tool.code())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(codes.len(), DiscordTool::ALL.len());
        assert!(!DiscordTool::Vlasei.has_web_page());
        assert!(!DiscordTool::Vlatai.has_web_page());
        assert!(!DiscordTool::Jvozba.has_web_page());
        assert!(BuildTag::current().as_str().len() <= 16);
    }
}
