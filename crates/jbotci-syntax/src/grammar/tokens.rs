use std::ops::Range;

use crate::{ExperimentalConstruct, Indicator, WordWithModifiers, WordWithModifiersData};
use bityzba::{data, requires};
use chumsky::error::{Rich, RichReason};
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_morphology::{Word, WordKind, WordLike, WordLikeData, strip_diacritics};
use jbotci_source::SourceSpan;

use super::{BoxedParser, ParserState, Span, SpannedToken};
use crate::SyntaxError;

pub(super) const PA_WORDS: &[&str] = &[
    "dau", "fei", "gai", "jau", "rei", "vai", "pi'e", "pi", "fi'u", "za'u", "me'i", "ni'u", "ki'o",
    "ce'i", "ma'u", "ra'e", "da'a", "so'a", "ji'i", "su'o", "su'e", "ro", "rau", "so'u", "so'i",
    "so'e", "so'o", "mo'a", "du'e", "te'o", "ka'o", "ci'i", "tu'o", "xo", "pai", "ro'oi", "su'oi",
    "xo'e", "no'o", "no", "pa", "re", "ci", "vo", "mu", "xa", "ze", "bi", "so", "0", "1", "2", "3",
    "4", "5", "6", "7", "8", "9",
];
pub(super) const MOI_WORDS: &[&str] = &["moi", "mei", "si'e", "cu'o", "va'e", "cei'a"];
pub(super) const MAI_WORDS: &[&str] = &["mo'o", "mai"];
pub(super) const LAU_WORDS: &[&str] = &["lau", "tau", "zai", "ce'a"];
pub(crate) const CAI_WORDS: &[&str] = &["pei", "cai", "cu'i", "sai", "ru'e"];
pub(super) const CAHA_WORDS: &[&str] = &["ca'a", "pu'i", "nu'o", "ka'e", "bi'ai"];
pub(super) const BAI_WORDS: &[&str] = &[
    "du'o", "si'u", "zau", "ki'i", "du'i", "cu'u", "tu'i", "ti'u", "di'o", "ji'u", "ri'a", "ni'i",
    "mu'i", "ki'u", "va'u", "koi", "ca'i", "ta'i", "pu'e", "ja'i", "kai", "bai", "fi'e", "de'i",
    "ci'o", "mau", "mu'u", "ri'i", "ra'i", "ka'a", "pa'u", "pa'a", "le'a", "ku'u", "tai", "bau",
    "ma'i", "ci'e", "fau", "po'i", "cau", "ma'e", "ci'u", "ra'a", "pu'a", "li'e", "la'u", "ba'i",
    "ka'i", "sau", "fa'e", "be'i", "ti'i", "ja'e", "ga'a", "va'o", "ji'o", "me'a", "do'e", "ji'e",
    "pi'o", "gau", "zu'e", "me'e", "rai",
];
pub(super) const KOHA_WORDS: &[&str] = &[
    "da'u", "da'e", "di'u", "di'e", "de'u", "de'e", "dei", "do'i", "mi'o", "ma'a", "mi'a", "do'o",
    "ko'a", "fo'u", "ko'e", "ko'i", "ko'o", "ko'u", "fo'a", "fo'e", "fo'i", "fo'o", "vo'a", "vo'e",
    "vo'i", "vo'o", "vo'u", "ru", "ri", "ra", "ta", "tu", "ti", "zi'o", "ke'a", "ma", "zu'i",
    "zo'e", "ce'u", "mi'ai", "nau'o", "nau'u", "xai", "zu'ai", "da", "de", "di", "ko", "mi", "do",
];
pub(super) const GOHA_WORDS: &[&str] = &[
    "mo", "nei", "go'u", "go'o", "go'i", "no'a", "go'e", "go'a", "du", "bu'a", "bu'e", "bu'i",
    "co'e",
];
pub(super) const ROI_WORDS: &[&str] = &["roi", "re'u", "mu'ei", "va'ei", "ba'oi", "de'ei", "xu'au"];
pub(super) const ZAHO_WORDS: &[&str] = &[
    "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o", "co'a'a",
    "co'au'a", "co'u'a", "sau'a", "xa'o", "xo'u",
];
pub(super) const FA_WORDS: &[&str] = &["fa", "fe", "fi", "fo", "fu", "fai", "fi'a"];
pub(crate) const UI_WORDS: &[&str] = &[
    "i'a", "ie", "a'e", "u'i", "i'o", "i'e", "a'a", "ia", "o'i", "o'e", "e'e", "oi", "uo", "e'i",
    "u'o", "au", "ua", "a'i", "i'u", "ii", "u'a", "ui", "a'o", "ai", "a'u", "iu", "ei", "o'o",
    "e'a", "uu", "o'a", "o'u", "u'u", "e'o", "io", "e'u", "ue", "i'i", "u'e", "ba'a", "ja'o",
    "ca'e", "su'a", "ti'e", "ka'u", "se'o", "za'a", "pe'i", "ru'a", "ju'a", "ta'o", "ra'u", "li'a",
    "ba'u", "mu'a", "do'a", "to'u", "va'i", "pa'e", "zu'u", "sa'e", "la'a", "ke'u", "sa'u", "da'i",
    "je'u", "sa'a", "kau", "ta'u", "na'i", "jo'a", "bi'u", "li'o", "pau", "mi'u", "ku'i", "ji'a",
    "si'a", "po'o", "pe'a", "ro'i", "ro'e", "ro'o", "ro'u", "ro'a", "re'e", "le'o", "ju'o", "fu'i",
    "dai", "ga'i", "zo'o", "be'u", "ri'e", "se'i", "se'a", "vu'e", "ki'a", "xu", "ge'e", "bu'o",
    "ai'i", "e'ei", "fu'au", "ju'oi", "ko'oi", "oi'a", "si'au", "ue'i", "xo'o", "li'oi",
];
pub(super) const VUHU_WORDS: &[&str] = &[
    "ge'a", "fu'u", "pi'i", "fe'i", "vu'u", "su'i", "ju'u", "gei", "pa'i", "fa'i", "te'a", "cu'a",
    "va'a", "ne'o", "de'o", "fe'a", "sa'o", "ri'o", "sa'i", "pi'a", "si'i", "joi'i",
];
pub(super) const NU_WORDS: &[&str] = &[
    "nu", "ni", "du'u", "si'o", "li'i", "ka", "jei", "su'u", "zu'o", "mu'e", "pu'u", "za'i",
    "kai'u", "poi'i", "xe'ei",
];
pub(super) const COI_WORDS: &[&str] = &[
    "ju'i", "coi", "fi'i", "ta'a", "mu'o", "fe'o", "co'o", "pe'u", "ke'o", "nu'e", "re'i", "be'e",
    "je'e", "mi'e", "ki'e", "vi'o", "co'oi", "di'ai", "ki'ai", "sa'ei", "a'oi", "o'ai",
];

#[requires(true)]
#[ensures(true)]
pub(super) fn cmavo<'tokens>(text: &'static str) -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("cmavo", move |word| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cmavo_of<'tokens>(
    label: &'static str,
    texts: &'static [&'static str],
) -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching(label, move |word| {
        texts.iter().any(|text| cmavo_text_matches(word, text))
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn le_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of(
        "LE",
        &["lei", "loi", "le'i", "lo'i", "le'e", "lo'e", "lo", "le"],
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn la_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("LA", &["lai", "la'i", "la"])
}

#[requires(true)]
#[ensures(true)]
pub(super) fn lahe_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of(
        "LAhE",
        &["tu'a", "lu'a", "lu'o", "la'e", "vu'i", "lu'i", "lu'e"],
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn leading_indicator<'tokens>() -> BoxedParser<'tokens, Indicator> {
    choice((cmavo_of("UI", UI_WORDS), cmavo_of("CAI", CAI_WORDS)))
        .then(cmavo("nai").or_not())
        .map(|(indicator, nai)| {
            let indicator = indicator
                .visible_word()
                .expect("leading indicator parser matched a visible word")
                .clone();
            let nai = nai.map(|nai| {
                nai.visible_word()
                    .expect("NAI parser matched a visible word")
                    .clone()
            });
            Indicator::new(indicator, nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn pa_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("PA", PA_WORDS)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn na_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("NA", &["na", "ja'a"])
}

#[requires(true)]
#[ensures(true)]
pub(super) fn koha_argument<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("KOhA argument", is_koha_argument)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("relation word", is_relation_word)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn brivla_relation_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("BRIVLA", is_brivla_relation_word)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cmevla_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("CMEVLA", is_cmevla_word)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letter_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("letter word", is_letter_word)
}

#[requires(!label.is_empty())]
#[ensures(true)]
pub(super) fn token_matching<'tokens>(
    label: &'static str,
    predicate: impl Fn(&WordWithModifiers) -> bool + Clone + 'tokens,
) -> BoxedParser<'tokens, WordWithModifiers> {
    custom(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        match input.next() {
            Some(word) if predicate(&word) => {
                warn_experimental_cmavo(input.state(), label, &word);
                Ok(word)
            }
            _ => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                Err(Rich::custom(span, format!("expected {label}")))
            }
        }
    })
    .boxed()
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn warn_experimental_cmavo(state: &mut ParserState, label: &str, word: &WordWithModifiers) {
    let Some(construct) = experimental_construct_for_cmavo(label, word) else {
        return;
    };
    state.warn(construct, word);
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn experimental_construct_for_cmavo(
    label: &str,
    word: &WordWithModifiers,
) -> Option<ExperimentalConstruct> {
    let canonical = word.visible_word()?.canonical_phonemes();
    match (label, canonical.as_str()) {
        (
            "BAI",
            "be'ei" | "de'i'a" | "de'i'e" | "de'i'i" | "de'i'o" | "de'i'u" | "ka'ai" | "ki'oi"
            | "ko'au",
        )
        | ("BY", "a'y" | "e'y" | "i'y" | "iy" | "o'y" | "u'y" | "uy")
        | ("CAhA", "bi'ai")
        | ("COI", "co'oi" | "di'ai" | "ki'ai" | "sa'ei")
        | ("KOhA", "mi'ai" | "nau'o" | "nau'u" | "xai" | "zu'ai")
        | ("LAhE", "zo'ei")
        | ("LE", "lei'i" | "lei'e" | "loi'e" | "loi'i" | "mo'oi" | "moi'oi")
        | ("ME", "me'au")
        | ("MOI", "cei'a")
        | ("NAhE", "na'ei")
        | ("NAI", "ja'ai")
        | ("NU", "kai'u" | "poi'i" | "xe'ei")
        | ("PA", "ro'oi" | "su'oi" | "xo'e")
        | ("ROI", "mu'ei" | "va'ei")
        | ("SE", "su'ei" | "to'ai" | "vo'ai" | "xo'ai")
        | (
            "UI",
            "ai'i" | "e'ei" | "fu'au" | "ju'oi" | "ko'oi" | "oi'a" | "si'au" | "ue'i" | "xo'o",
        )
        | ("VUhU", "joi'i")
        | ("XI", "te'ai")
        | ("ZAhO", "co'a'a" | "co'au'a" | "co'u'a" | "sau'a" | "xa'o" | "xo'u")
        | ("ZO", "ma'oi")
        | ("ZOhU", "ce'ai") => Some(ExperimentalConstruct::ExperimentalCmavo),
        ("COI", "a'oi" | "o'ai") => Some(ExperimentalConstruct::ExperimentalDictionaryCoiVocative),
        ("DOI", "da'oi") => Some(ExperimentalConstruct::ExperimentalDictionaryDoiVocative),
        ("FAhA", "xei'e") => Some(ExperimentalConstruct::ExperimentalDictionaryFahaTag),
        ("PA", "su'ai" | "xe'e") => Some(ExperimentalConstruct::ExperimentalDictionaryPaNumber),
        ("SEI", "xoi") => Some(ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier),
        ("UI" | "UI3a", "li'oi") => Some(ExperimentalConstruct::ExperimentalDictionaryUiIndicator),
        ("NOIhA", _) => Some(ExperimentalConstruct::ExperimentalNoihaAdverbial),
        ("SOI", _) => Some(ExperimentalConstruct::ExperimentalSoiAdverbial),
        ("LOhOI", "lo'oi") => Some(ExperimentalConstruct::ExperimentalLohOiBridiDescription),
        ("LOhOI", "mau'a" | "xau'a") => Some(ExperimentalConstruct::ExperimentalZantufaCmavo),
        ("ROI", "ba'oi" | "de'ei" | "xu'au") => {
            Some(ExperimentalConstruct::ExperimentalZantufaCmavo)
        }
        ("cmavo", "fi'oi") => Some(ExperimentalConstruct::ExperimentalFihoiAdverbial),
        ("cmavo", "lo'ai" | "sa'ai" | "le'ai") => {
            Some(ExperimentalConstruct::ExperimentalLohAiReplacementFree)
        }
        ("cmavo", "no'oi") => Some(ExperimentalConstruct::ExperimentalNohoiSelbriRelativeClause),
        ("cmavo", "go'oi") => Some(ExperimentalConstruct::ExperimentalGohoiRelationUnit),
        ("cmavo", "lu'ei") => Some(ExperimentalConstruct::ExperimentalZantufaLuheiRelationUnit),
        ("cmavo", "mu'oi") => Some(ExperimentalConstruct::ExperimentalZantufaMuhoiRelationUnit),
        ("cmavo", "xo'i") => Some(ExperimentalConstruct::ExperimentalXohiTagRelation),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn is_koha_argument(word: &WordWithModifiers) -> bool {
    KOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn is_relation_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::WithIndicator { base, .. }) => return is_relation_word(base),
        data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            return word_like_is_relation_word(word_like);
        }
        data!(WordWithModifiers::Bare(..)) => {}
    }

    if GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text)) {
        return true;
    }

    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like)) => word_like_is_relation_word(word_like),
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret == (is_relation_word(word) && !GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))))]
pub(super) fn is_brivla_relation_word(word: &WordWithModifiers) -> bool {
    is_relation_word(word) && !GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn word_like_is_relation_word(word_like: &WordLike) -> bool {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => {
            matches!(
                word.kind,
                WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
            )
        }
        data!(WordLike::ZeiLujvo { .. }) => true,
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn is_cmevla_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like))
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            word_like_kind(word_like).is_some_and(|kind| kind == WordKind::Cmevla)
        }
        data!(WordWithModifiers::WithIndicator { base, .. }) => is_cmevla_word(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn is_letter_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like))
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => match word_like.as_data() {
            data!(WordLike::Letter { .. }) => true,
            data!(WordLike::Bare(word)) => {
                word.kind == WordKind::Cmavo
                    && ((word.phonemes != "bu" && word.phonemes.ends_with("bu"))
                        || matches!(
                            word.phonemes.as_str(),
                            "jo'o"
                                | "ru'o"
                                | "ge'o"
                                | "je'o"
                                | "lo'a"
                                | "na'a"
                                | "se'e"
                                | "to'a"
                                | "ga'e"
                                | "y'y"
                                | "y"
                                | "by"
                                | "cy"
                                | "dy"
                                | "fy"
                                | "gy"
                                | "jy"
                                | "ky"
                                | "ly"
                                | "my"
                                | "ny"
                                | "py"
                                | "ry"
                                | "sy"
                                | "ty"
                                | "vy"
                                | "xy"
                                | "zy"
                        ))
            }
            _ => false,
        },
        data!(WordWithModifiers::WithIndicator { base, .. }) => is_letter_word(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn word_like_kind(word_like: &WordLike) -> Option<WordKind> {
    let data!(WordLike::Bare(word)) = word_like.as_data() else {
        return None;
    };
    Some(word.kind)
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn cmavo_text_matches(word: &WordWithModifiers, expected: &str) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like))
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            word_like_cmavo_text_matches(word_like, expected)
        }
        data!(WordWithModifiers::WithIndicator { base, .. }) => cmavo_text_matches(base, expected),
    }
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn word_like_cmavo_text_matches(word_like: &WordLike, expected: &str) -> bool {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => word_record_text_matches(word, expected),
        _ => false,
    }
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn word_record_text_matches(word: &jbotci_morphology::Word, expected: &str) -> bool {
    word.kind == WordKind::Cmavo && phonemes_match_syntax_text(&word.phonemes, expected)
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn phonemes_match_syntax_text(actual: &str, expected: &str) -> bool {
    actual == expected || strip_diacritics(actual) == expected
}

#[requires(true)]
#[ensures(true)]
pub(super) fn bare_word_kind_and_phonemes(word: &WordWithModifiers) -> Option<(WordKind, &str)> {
    let data!(WordWithModifiers::Bare(word_like)) = word.as_data() else {
        return None;
    };
    let data!(WordLike::Bare(word)) = word_like.as_data() else {
        return None;
    };
    Some((word.kind, word.phonemes.as_str()))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn base_word_from_record(word: Word) -> WordWithModifiers {
    WordWithModifiers::bare(WordLike::bare(word))
}

#[requires(span.byte_start <= span.byte_end)]
#[ensures(source.is_some_and(|source| span.byte_end <= source.len()) -> ret.len() == span.byte_end - span.byte_start)]
pub(super) fn source_text(source: Option<&str>, span: &SourceSpan) -> String {
    source
        .and_then(|source| source.get(span.byte_start..span.byte_end))
        .unwrap_or_default()
        .to_owned()
}

#[requires(true)]
#[ensures(ret.iter().all(|token| token.span.start <= token.span.end))]
pub(super) fn spanned_tokens(words: &[WordWithModifiers]) -> Vec<SpannedToken> {
    words
        .iter()
        .cloned()
        .map(|word| {
            let range = word_byte_range(&word).unwrap_or(0..0);
            Spanned {
                inner: word,
                span: SimpleSpan::from(range),
            }
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
pub(super) fn word_byte_range(word: &WordWithModifiers) -> Option<Range<usize>> {
    match word.as_data() {
        data!(WordWithModifiers::Bare(word_like)) => word_like_byte_range(word_like),
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => word_like_byte_range(word_like)
            .map(|range| bahe.span.byte_start.min(range.start)..bahe.span.byte_end.max(range.end)),
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => word_byte_range(base).map(|range| {
            range.start
                ..nai
                    .as_ref()
                    .unwrap_or(indicator)
                    .span
                    .byte_end
                    .max(range.end)
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => Some(word.span.byte_start..word.span.byte_end),
        data!(WordLike::ZoQuote { zo, word }) => Some(zo.span.byte_start..word.span.byte_end),
        data!(WordLike::ZoiQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span.byte_start..closing_delimiter.span.byte_end),
        data!(WordLike::LohuQuote { lohu, lehu, .. }) => {
            Some(lohu.span.byte_start..lehu.span.byte_end)
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => Some(marker.span.byte_start..quoted_text.byte_end),
        data!(WordLike::Letter { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span.byte_end.max(range.end))
        }
        data!(WordLike::ZeiLujvo { left, right, .. }) => {
            word_like_byte_range(left).map(|range| range.start..right.span.byte_end.max(range.end))
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret, SyntaxError::Parse { ref reason, .. } if !reason.is_empty()) || !matches!(ret, SyntaxError::Parse { .. }))]
pub(super) fn syntax_error(errors: Vec<Rich<'_, WordWithModifiers, Span>>) -> SyntaxError {
    let Some(error) = errors.into_iter().next() else {
        return SyntaxError::Parse {
            byte_offset: 0,
            reason: "unknown Chumsky syntax error".to_owned(),
        };
    };

    let reason = match error.reason() {
        RichReason::Custom(message) => message.to_string(),
        _ => format!("{error:?}"),
    };

    SyntaxError::Parse {
        byte_offset: error.span().start,
        reason,
    }
}
