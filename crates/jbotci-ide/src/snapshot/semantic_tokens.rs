#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{Selmaho, Word, WordKind, WordLike, WordLikeData};
use jbotci_source::SourceSpan;

/// Transport-neutral semantic-token classes, in LSP legend order.
#[repr(u32)]
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Gismu,
    Lujvo,
    Fuhivla,
    Cmevla,
    SumtiWord,
    SelbriWord,
    Connective,
    Terminator,
    QuotationMarker,
    Number,
    Letteral,
    Attitudinal,
    TenseModal,
    Cmavo,
    String,
}

impl SemanticTokenKind {
    pub const ALL: [Self; 15] = [
        Self::Gismu,
        Self::Lujvo,
        Self::Fuhivla,
        Self::Cmevla,
        Self::SumtiWord,
        Self::SelbriWord,
        Self::Connective,
        Self::Terminator,
        Self::QuotationMarker,
        Self::Number,
        Self::Letteral,
        Self::Attitudinal,
        Self::TenseModal,
        Self::Cmavo,
        Self::String,
    ];

    #[requires(true)]
    #[ensures(ret < Self::ALL.len() as u32)]
    pub const fn legend_index(self) -> u32 {
        self as u32
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn lsp_name(self) -> &'static str {
        match self {
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuhivla => "fuhivla",
            Self::Cmevla => "cmevla",
            Self::SumtiWord => "sumtiWord",
            Self::SelbriWord => "selbriWord",
            Self::Connective => "connective",
            Self::Terminator => "terminator",
            Self::QuotationMarker => "quotationMarker",
            Self::Number => "number",
            Self::Letteral => "letteral",
            Self::Attitudinal => "attitudinal",
            Self::TenseModal => "tenseModal",
            Self::Cmavo => "cmavo",
            Self::String => "string",
        }
    }
}

/// One morphology-derived token and its half-open source span.
#[invariant(span.byte_start < span.byte_end && span.char_start < span.char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    pub kind: SemanticTokenKind,
    pub span: SourceSpan,
}

#[requires(words.len() == word_spans.len())]
#[ensures(true)]
pub(super) fn build_semantic_tokens(
    words: &[WordLike],
    word_spans: &[SourceSpan],
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    for (word, span) in words.iter().zip(word_spans) {
        push_word_like_tokens(&mut tokens, word, span);
    }
    tokens
}

#[requires(true)]
#[ensures(true)]
fn push_word_like_tokens(tokens: &mut Vec<SemanticToken>, word_like: &WordLike, span: &SourceSpan) {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => push_word(tokens, word),
        data!(WordLike::QuotedWord { zo, word }) => {
            push_token(tokens, SemanticTokenKind::QuotationMarker, zo.span());
            push_token(tokens, SemanticTokenKind::String, word.span());
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            push_token(tokens, SemanticTokenKind::QuotationMarker, mahoi.span());
            push_token(tokens, SemanticTokenKind::String, word.span());
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => {
            push_token(tokens, SemanticTokenKind::QuotationMarker, zoi.span());
            push_token(
                tokens,
                SemanticTokenKind::QuotationMarker,
                opening_delimiter.span(),
            );
            push_token(tokens, SemanticTokenKind::String, &quoted_text.span);
            push_token(
                tokens,
                SemanticTokenKind::QuotationMarker,
                closing_delimiter.span(),
            );
        }
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            push_token(tokens, SemanticTokenKind::QuotationMarker, lohu.span());
            for word in quoted_words {
                push_token(tokens, SemanticTokenKind::String, word.span());
            }
            push_token(tokens, SemanticTokenKind::QuotationMarker, lehu.span());
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => {
            push_token(tokens, SemanticTokenKind::QuotationMarker, marker.span());
            push_token(tokens, SemanticTokenKind::String, &quoted_text.span);
        }
        // BU compounds are one morphology word and should not expose their base
        // word's unrelated class (for example, the A in `.abu`).
        data!(WordLike::LerfuWord { .. }) => {
            push_token(tokens, SemanticTokenKind::Letteral, span);
        }
        // ZEI compounds are morphology-level lujvo equivalents, independently
        // of the classes of the words joined by ZEI.
        data!(WordLike::ZeiCompound { .. }) => {
            push_token(tokens, SemanticTokenKind::Lujvo, span);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_word(tokens: &mut Vec<SemanticToken>, word: &Word) {
    let kind = match word.kind() {
        WordKind::Cmavo => cmavo_token_kind(word.selmaho_kind()),
        WordKind::Gismu => SemanticTokenKind::Gismu,
        WordKind::Lujvo => SemanticTokenKind::Lujvo,
        WordKind::Fuhivla => SemanticTokenKind::Fuhivla,
        WordKind::Cmevla => SemanticTokenKind::Cmevla,
    };
    push_token(tokens, kind, word.span());
}

/// Assign every modeled selma'o to one stable presentation group.
///
/// This match is intentionally exhaustive instead of using spelling tables:
/// adding a selma'o to morphology requires an explicit highlighting decision.
#[requires(true)]
#[ensures(true)]
fn cmavo_token_kind(selmaho: Option<Selmaho>) -> SemanticTokenKind {
    match selmaho {
        Some(Selmaho::Koha | Selmaho::La | Selmaho::Lahe | Selmaho::Le | Selmaho::Li) => {
            SemanticTokenKind::SumtiWord
        }
        Some(Selmaho::Goha | Selmaho::Me | Selmaho::Moi | Selmaho::Nu) => {
            SemanticTokenKind::SelbriWord
        }
        Some(
            Selmaho::A
            | Selmaho::Bihi
            | Selmaho::Cehe
            | Selmaho::Ga
            | Selmaho::Gaho
            | Selmaho::Gi
            | Selmaho::Giha
            | Selmaho::Gihi
            | Selmaho::Guha
            | Selmaho::Ja
            | Selmaho::Jehi
            | Selmaho::Joi
            | Selmaho::Pehe,
        ) => SemanticTokenKind::Connective,
        Some(
            Selmaho::Beho
            | Selmaho::Faho
            | Selmaho::Ku
            | Selmaho::Loho
            | Selmaho::Sehu
            | Selmaho::Toi
            | Selmaho::Vau
            | Selmaho::Veho,
        ) => SemanticTokenKind::Terminator,
        Some(
            Selmaho::Lehu
            | Selmaho::Lihu
            | Selmaho::Lohu
            | Selmaho::Lu
            | Selmaho::Zo
            | Selmaho::Zoi,
        ) => SemanticTokenKind::QuotationMarker,
        Some(Selmaho::Pa) => SemanticTokenKind::Number,
        Some(Selmaho::Bu | Selmaho::By | Selmaho::Lau) => SemanticTokenKind::Letteral,
        Some(Selmaho::Cai | Selmaho::Daho | Selmaho::Ui | Selmaho::Ui3a | Selmaho::Y) => {
            SemanticTokenKind::Attitudinal
        }
        Some(
            Selmaho::Bai
            | Selmaho::Caha
            | Selmaho::Cuhe
            | Selmaho::Faha
            | Selmaho::Mohi
            | Selmaho::Pu
            | Selmaho::Roi
            | Selmaho::Tahe
            | Selmaho::Va
            | Selmaho::Veha
            | Selmaho::Viha
            | Selmaho::Zaho
            | Selmaho::Zeha
            | Selmaho::Zi,
        ) => SemanticTokenKind::TenseModal,
        Some(
            Selmaho::Bahe
            | Selmaho::Be
            | Selmaho::Bei
            | Selmaho::Co
            | Selmaho::Coi
            | Selmaho::Cu
            | Selmaho::Doi
            | Selmaho::Fa
            | Selmaho::Fuha
            | Selmaho::Goi
            | Selmaho::I
            | Selmaho::Jai
            | Selmaho::Johi
            | Selmaho::Lihau
            | Selmaho::Lohoi
            | Selmaho::Luhei
            | Selmaho::Mai
            | Selmaho::Mohe
            | Selmaho::Na
            | Selmaho::Nahe
            | Selmaho::Nai
            | Selmaho::Niho
            | Selmaho::Noi
            | Selmaho::Noiha
            | Selmaho::Sa
            | Selmaho::Se
            | Selmaho::Sei
            | Selmaho::Si
            | Selmaho::Soi
            | Selmaho::Su
            | Selmaho::To
            | Selmaho::Tuhe
            | Selmaho::Vei
            | Selmaho::Vuhu
            | Selmaho::Xi
            | Selmaho::Zei
            | Selmaho::Zohu,
        )
        | None => SemanticTokenKind::Cmavo,
    }
}

#[requires(true)]
#[ensures(true)]
fn push_token(tokens: &mut Vec<SemanticToken>, kind: SemanticTokenKind, span: &SourceSpan) {
    if span.byte_start < span.byte_end && span.char_start < span.char_end {
        tokens.push(new!(SemanticToken {
            kind,
            span: span.clone(),
        }));
    }
}
