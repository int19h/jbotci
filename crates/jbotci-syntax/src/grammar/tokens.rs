use std::ops::Range;

use crate::{ExperimentalConstruct, Indicator, WithIndicators};
use bityzba::{data, new, requires};
use chumsky::error::RichReason;
use chumsky::input::MapExtra;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_diagnostics::{
    TraceContext, TraceEventKind, TraceFailureBranch, TraceFailureSummary, TraceLevel,
};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordKind, WordLike, WordLikeData};
use jbotci_source::SourceSpan;

use super::{BoxedParser, ParseExtra, ParserInput, ParserState, SpannedToken, SyntaxParseError};
use crate::{
    SyntaxConstructContext, SyntaxError, SyntaxExpectedToken, SyntaxExpectedTokenData,
    SyntaxWordCategory,
};

#[requires(true)]
#[ensures(true)]
pub(super) fn cmavo<'tokens>(cmavo: Cmavo) -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        "cmavo",
        cmavo.canonical_text(),
        vec![new!(SyntaxExpectedToken::Cmavo(cmavo))],
        move |word| parser_word_is_cmavo(word, cmavo),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn selmaho<'tokens>(selmaho: Selmaho) -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        selmaho.name(),
        selmaho.name(),
        vec![new!(SyntaxExpectedToken::Selmaho(selmaho))],
        move |word| parser_word_is_selmaho(word, selmaho),
    )
}

#[requires(!label.is_empty())]
#[requires(!cmavo.is_empty())]
#[ensures(true)]
pub(super) fn cmavo_one_of<'tokens>(
    label: &'static str,
    cmavo: &'static [Cmavo],
) -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        label,
        label,
        cmavo
            .iter()
            .copied()
            .map(|cmavo| new!(SyntaxExpectedToken::Cmavo(cmavo)))
            .collect(),
        move |word| parser_word_is_one_of_cmavo(word, cmavo),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn le_cmavo<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    selmaho(Selmaho::Le)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn la_cmavo<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    selmaho(Selmaho::La)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn lahe_cmavo<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    selmaho(Selmaho::Lahe)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn leading_indicator<'tokens>() -> BoxedParser<'tokens, Indicator> {
    choice((selmaho(Selmaho::Ui), selmaho(Selmaho::Cai)))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(indicator, nai)| {
            let nai = nai.map(|nai| {
                nai.core_word()
                    .bare_word()
                    .expect("NAI parser matched a visible word")
                    .clone()
            });
            Indicator::new(indicator, nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn pa_word<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    selmaho(Selmaho::Pa)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn na_cmavo<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    selmaho(Selmaho::Na)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn koha_argument<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        "KOhA argument",
        "KOhA argument",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::KohaArgument,
        ))],
        is_koha_argument,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_word<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        "relation word",
        "RELATION WORD",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::RelationWord,
        ))],
        is_relation_word,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn brivla_relation_word<'tokens>(
    cbm_enabled: bool,
) -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    let brivla = token_matching(
        "BRIVLA",
        "BRIVLA",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::Brivla
        ))],
        is_brivla_relation_word,
    );
    if cbm_enabled {
        brivla
            .or(cmevla_word().map_with(
                |word,
                 extra: &mut MapExtra<
                    'tokens,
                    '_,
                    super::ParserInput<'tokens>,
                    super::ParseExtra<'tokens>,
                >| {
                    extra.state().warn(
                        ExperimentalConstruct::ExperimentalCbmCmevlaRelationWord,
                        &word,
                    );
                    word
                },
            ))
            .boxed()
    } else {
        brivla
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cmevla_word<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        "CMEVLA",
        "CMEVLA",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::Cmevla
        ))],
        is_cmevla_word,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letter_word<'tokens>() -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    token_matching(
        "letter word",
        "LETTER WORD",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::LetterWord,
        ))],
        is_letter_word,
    )
}

#[requires(!label.is_empty())]
#[requires(!debug_label.is_empty())]
#[ensures(true)]
pub(super) fn token_matching<'tokens>(
    label: &'static str,
    debug_label: &'static str,
    expected: Vec<SyntaxExpectedToken>,
    predicate: impl Fn(&WithIndicators<WordLike>) -> bool + Clone + 'tokens,
) -> BoxedParser<'tokens, WithIndicators<WordLike>> {
    assert!(
        !expected.is_empty(),
        "token parsers must declare expected tokens"
    );
    custom::<_, ParserInput<'tokens>, WithIndicators<WordLike>, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        match input.next() {
            Some(word) if predicate(&word) => {
                let span = word.core_word().byte_range().unwrap_or(0..0);
                let state: &mut ParserState = input.state();
                warn_experimental_cmavo(state, label, &word);
                state.trace_event(
                    TraceLevel::Primitives,
                    TraceEventKind::TerminalSuccess,
                    debug_label,
                    span.start,
                    span.end,
                    || Some(word.core_word().to_string()),
                );
                Ok(word)
            }
            _ => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                let byte_start = span.start.min(span.end);
                let byte_end = span.start.max(span.end);
                input.state().trace_event(
                    TraceLevel::Primitives,
                    TraceEventKind::TerminalFailure,
                    debug_label,
                    byte_start,
                    byte_end,
                    || Some(expected_token_detail(&expected)),
                );
                Err(SyntaxParseError::expected(span, expected.clone()))
            }
        }
    })
    .labelled(debug_label)
    .as_terminal()
    .boxed()
}

#[requires(!expected.is_empty())]
#[ensures(!ret.is_empty())]
fn expected_token_detail(expected: &[SyntaxExpectedToken]) -> String {
    format!(
        "expected {}",
        expected
            .iter()
            .map(SyntaxExpectedToken::summary_text)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn warn_experimental_cmavo(state: &mut ParserState, label: &str, word: &WithIndicators<WordLike>) {
    if let Some(cmavo) = parser_word_cmavo(word)
        && let Some(construct) = experimental_construct_for_cmavo(label, cmavo)
    {
        state.warn(construct, word);
    }
    warn_experimental_indicators(state, word);
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn experimental_construct_for_cmavo(label: &str, cmavo: Cmavo) -> Option<ExperimentalConstruct> {
    match (label, cmavo) {
        ("COI", Cmavo::Ahoi | Cmavo::Ohai) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryCoiVocative)
        }
        ("DOI", Cmavo::Dahoi) => Some(ExperimentalConstruct::ExperimentalDictionaryDoiVocative),
        ("FAhA", Cmavo::Xeihe) => Some(ExperimentalConstruct::ExperimentalDictionaryFahaTag),
        ("PA", Cmavo::Suhai | Cmavo::Xehe) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryPaNumber)
        }
        ("SEI", Cmavo::Xoi) => Some(ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier),
        ("UI" | "UI3a", Cmavo::Lihoi) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryUiIndicator)
        }
        ("NOIhA", Cmavo::Noihoha) => Some(ExperimentalConstruct::ExperimentalZantufaCmavo),
        ("NOIhA", _) => Some(ExperimentalConstruct::ExperimentalNoihaAdverbial),
        ("SOI", _) => Some(ExperimentalConstruct::ExperimentalSoiAdverbial),
        ("LOhOI", Cmavo::Lohoi) => Some(ExperimentalConstruct::ExperimentalLohOiBridiDescription),
        ("cmavo", Cmavo::Fihoi) => Some(ExperimentalConstruct::ExperimentalFihoiAdverbial),
        ("cmavo", Cmavo::Lohai | Cmavo::Sahai | Cmavo::Lehai) => {
            Some(ExperimentalConstruct::ExperimentalLohAiReplacementFree)
        }
        ("cmavo", Cmavo::Nohoi) => {
            Some(ExperimentalConstruct::ExperimentalNohoiSelbriRelativeClause)
        }
        ("cmavo", Cmavo::Gohoi) => Some(ExperimentalConstruct::ExperimentalGohoiRelationUnit),
        ("LIhAU" | "LUhEI", _) => Some(ExperimentalConstruct::ExperimentalZantufaLuheiRelationUnit),
        ("cmavo", Cmavo::Luhei) => {
            Some(ExperimentalConstruct::ExperimentalZantufaLuheiRelationUnit)
        }
        ("cmavo", Cmavo::Muhoi) => {
            Some(ExperimentalConstruct::ExperimentalZantufaMuhoiRelationUnit)
        }
        ("cmavo", Cmavo::Xohi) => Some(ExperimentalConstruct::ExperimentalXohiTagRelation),
        _ if is_general_experimental_cmavo_for_context(label, cmavo) => {
            Some(ExperimentalConstruct::ExperimentalCmavo)
        }
        _ if is_zantufa_experimental_cmavo_for_context(label, cmavo) => {
            Some(ExperimentalConstruct::ExperimentalZantufaCmavo)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn warn_experimental_indicators(state: &mut ParserState, word: &WithIndicators<WordLike>) {
    let WithIndicators::WithIndicator {
        base,
        indicator,
        nai,
    } = word
    else {
        return;
    };

    warn_experimental_indicators(state, base);

    if let Some(label) = indicator_cmavo_context(indicator)
        && let Some(cmavo) = indicator.cmavo()
        && let Some(construct) = experimental_construct_for_cmavo(label, cmavo)
    {
        state.warn_word(construct, word, indicator);
    }

    if let Some(nai) = nai
        && let Some(construct) = experimental_construct_for_cmavo("NAI", Cmavo::Nai)
    {
        state.warn_word(construct, word, nai);
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|label| !label.is_empty()))]
fn indicator_cmavo_context(indicator: &Word) -> Option<&'static str> {
    let cmavo = indicator.cmavo()?;
    if cmavo.is_selmaho(Selmaho::Ui) {
        Some("UI")
    } else if cmavo.is_selmaho(Selmaho::Cai) {
        Some("CAI")
    } else if cmavo == Cmavo::Y {
        Some("Y")
    } else {
        None
    }
}

#[requires(true)]
#[ensures(ret == word.core_word().cmavo())]
fn parser_word_cmavo(word: &WithIndicators<WordLike>) -> Option<Cmavo> {
    word.core_word().cmavo()
}

#[requires(true)]
#[ensures(ret == (parser_word_cmavo(word) == Some(cmavo)))]
fn parser_word_is_cmavo(word: &WithIndicators<WordLike>, cmavo: Cmavo) -> bool {
    parser_word_cmavo(word) == Some(cmavo)
}

#[requires(!cmavo.is_empty())]
#[ensures(ret == parser_word_cmavo(word).is_some_and(|actual| cmavo.contains(&actual)))]
fn parser_word_is_one_of_cmavo(word: &WithIndicators<WordLike>, cmavo: &[Cmavo]) -> bool {
    parser_word_cmavo(word).is_some_and(|actual| cmavo.contains(&actual))
}

#[requires(true)]
#[ensures(ret == parser_word_cmavo(word).is_some_and(|cmavo| selmaho.contains(cmavo)))]
fn parser_word_is_selmaho(word: &WithIndicators<WordLike>, selmaho: Selmaho) -> bool {
    parser_word_cmavo(word).is_some_and(|cmavo| selmaho.contains(cmavo))
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn is_general_experimental_cmavo_for_context(label: &str, cmavo: Cmavo) -> bool {
    match label {
        "BAI" => matches!(
            cmavo,
            Cmavo::Behei
                | Cmavo::Dehiha
                | Cmavo::Dehihe
                | Cmavo::Dehihi
                | Cmavo::Dehiho
                | Cmavo::Dehihu
                | Cmavo::Kahai
                | Cmavo::Kihoi
                | Cmavo::Kohau
        ),
        "BY" => matches!(
            cmavo,
            Cmavo::Ahy | Cmavo::Ehy | Cmavo::Ihy | Cmavo::Iy | Cmavo::Ohy | Cmavo::Uhy | Cmavo::Uy
        ),
        "CAhA" => matches!(cmavo, Cmavo::Bihai),
        "COI" => matches!(
            cmavo,
            Cmavo::Cohoi | Cmavo::Dihai | Cmavo::Kihai | Cmavo::Sahei
        ),
        "KOhA" => matches!(
            cmavo,
            Cmavo::Mihai | Cmavo::Nauho | Cmavo::Nauhu | Cmavo::Xai | Cmavo::Zuhai
        ),
        "LAhE" => matches!(cmavo, Cmavo::Zohei),
        "LE" => matches!(
            cmavo,
            Cmavo::Leihe
                | Cmavo::Leihi
                | Cmavo::Loihe
                | Cmavo::Loihi
                | Cmavo::Mohoi
                | Cmavo::Moihoi
        ),
        "ME" => matches!(cmavo, Cmavo::Mehau),
        "MOI" => matches!(cmavo, Cmavo::Ceiha),
        "NAI" => matches!(cmavo, Cmavo::Jahai),
        "NAhE" => matches!(cmavo, Cmavo::Nahei),
        "NU" => matches!(cmavo, Cmavo::Kaihu | Cmavo::Poihi | Cmavo::Xehei),
        "PA" => matches!(cmavo, Cmavo::Rohoi | Cmavo::Suhoi | Cmavo::Xohe),
        "ROI" => matches!(cmavo, Cmavo::Muhei | Cmavo::Vahei),
        "SE" => matches!(
            cmavo,
            Cmavo::Suhei | Cmavo::Tohai | Cmavo::Vohai | Cmavo::Xohai
        ),
        "UI" => matches!(
            cmavo,
            Cmavo::Aihi
                | Cmavo::Ehei
                | Cmavo::Fuhau
                | Cmavo::Juhoi
                | Cmavo::Kohoi
                | Cmavo::Oiha
                | Cmavo::Sihau
                | Cmavo::Uehi
                | Cmavo::Xoho
        ),
        "VUhU" => matches!(cmavo, Cmavo::Joihi),
        "XI" => matches!(cmavo, Cmavo::Tehai),
        "ZAhO" => matches!(
            cmavo,
            Cmavo::Cohaha
                | Cmavo::Cohauha
                | Cmavo::Cohuha
                | Cmavo::Sauha
                | Cmavo::Xaho
                | Cmavo::Xohu
        ),
        "ZO" => matches!(cmavo, Cmavo::Mahoi),
        "ZOhU" => matches!(cmavo, Cmavo::Cehai),
        _ => false,
    }
}
#[requires(!label.is_empty())]
#[ensures(true)]
fn is_zantufa_experimental_cmavo_for_context(label: &str, cmavo: Cmavo) -> bool {
    match label {
        "BAI" => matches!(
            cmavo,
            Cmavo::Baihau
                | Cmavo::Behau
                | Cmavo::Buhuhe
                | Cmavo::Cuhei
                | Cmavo::Dauha
                | Cmavo::Dauho
                | Cmavo::Dauhu
                | Cmavo::Dehahu
                | Cmavo::Ehuhi
                | Cmavo::Eihei
                | Cmavo::Fauhu
                | Cmavo::Gahei
                | Cmavo::Jahau
                | Cmavo::Jahoi
                | Cmavo::Jahui
                | Cmavo::Jihehe
                | Cmavo::Jihiha
                | Cmavo::Kihai
                | Cmavo::Kihohe
                | Cmavo::Kihuhe
                | Cmavo::Kihuhi
                | Cmavo::Lahai
                | Cmavo::Lahei
                | Cmavo::Lahoho
                | Cmavo::Lihehe
                | Cmavo::Lihei
                | Cmavo::Mahei
                | Cmavo::Mauhi
                | Cmavo::Mauhu
                | Cmavo::Muhai
                | Cmavo::Muhei
                | Cmavo::Muhoi
                | Cmavo::Nehahi
                | Cmavo::Nihihi
                | Cmavo::Pahahi
                | Cmavo::Pehahi
                | Cmavo::Puhehi
                | Cmavo::Puhiha
                | Cmavo::Puhihi
                | Cmavo::Puhohi
                | Cmavo::Raihe
                | Cmavo::Rihiha
                | Cmavo::Rihihe
                | Cmavo::Rihihi
                | Cmavo::Rihiho
                | Cmavo::Rihihu
                | Cmavo::Tahiha
                | Cmavo::Tahihe
                | Cmavo::Tahihi
                | Cmavo::Tahiho
                | Cmavo::Tahihu
                | Cmavo::Tahuhi
                | Cmavo::Tehai
                | Cmavo::Tihiha
                | Cmavo::Tihuha
                | Cmavo::Tihuhi
                | Cmavo::Tihuhu
                | Cmavo::Tuhiha
                | Cmavo::Tuhihe
                | Cmavo::Tuhihi
                | Cmavo::Tuhiho
                | Cmavo::Tuhihu
                | Cmavo::Vahohi
                | Cmavo::Xuhai
                | Cmavo::Zauha
                | Cmavo::Zauhe
                | Cmavo::Zauhi
                | Cmavo::Zauho
                | Cmavo::Zauhu
                | Cmavo::Zuhai
        ),
        "BY" => matches!(
            cmavo,
            Cmavo::A
                | Cmavo::Cauhe
                | Cmavo::Cauhi
                | Cmavo::Daiha
                | Cmavo::Daihe
                | Cmavo::Daihi
                | Cmavo::Daiho
                | Cmavo::Daihu
                | Cmavo::Daihy
                | Cmavo::Dauhe
                | Cmavo::Dauhi
                | Cmavo::E
                | Cmavo::Fauha
                | Cmavo::Fauhe
                | Cmavo::Fauhi
                | Cmavo::Fauho
                | Cmavo::Fauhu
                | Cmavo::Gaiha
                | Cmavo::Gaihe
                | Cmavo::Gaihi
                | Cmavo::Gaiho
                | Cmavo::Gaihu
                | Cmavo::I
                | Cmavo::Jauha
                | Cmavo::Jauhe
                | Cmavo::Jauhi
                | Cmavo::Jauho
                | Cmavo::Jauhu
                | Cmavo::Joiho
                | Cmavo::Joihu
                | Cmavo::Kauha
                | Cmavo::Kauhe
                | Cmavo::Kauhi
                | Cmavo::Kauho
                | Cmavo::Kauhu
                | Cmavo::O
                | Cmavo::U
        ),
        "COI" => matches!(
            cmavo,
            Cmavo::Feihe
                | Cmavo::Gauhi
                | Cmavo::Jeihe
                | Cmavo::Mihei
                | Cmavo::Pehei
                | Cmavo::Peihe
                | Cmavo::Rehei
                | Cmavo::Xuhei
        ),
        "CUhE" => matches!(cmavo, Cmavo::Bahau | Cmavo::Puhau),
        "DAhO" => matches!(cmavo, Cmavo::Daiho | Cmavo::Dohai),
        "DOI" => matches!(cmavo, Cmavo::Dahei),
        "FAhA" => matches!(cmavo, Cmavo::Duhoi | Cmavo::Zuhau),
        "GOI" => matches!(cmavo, Cmavo::Voihe),
        "GOhA" => matches!(
            cmavo,
            Cmavo::Bohei | Cmavo::Ceihi | Cmavo::Gaiho | Cmavo::Tahai | Cmavo::Xehu | Cmavo::Zehoi
        ),
        "JAI" => matches!(cmavo, Cmavo::Jahei | Cmavo::Johai),
        "JOI" => matches!(
            cmavo,
            Cmavo::Jauhu
                | Cmavo::Jehau
                | Cmavo::Jeihi
                | Cmavo::Jeiho
                | Cmavo::Johau
                | Cmavo::Johiha
                | Cmavo::Johuhu
                | Cmavo::Joihe
        ),
        "KOhA" => matches!(
            cmavo,
            Cmavo::Dahei
                | Cmavo::Deiha
                | Cmavo::Dihei
                | Cmavo::Foha
                | Cmavo::Fohai
                | Cmavo::Fohe
                | Cmavo::Fohi
                | Cmavo::Foho
                | Cmavo::Fohu
                | Cmavo::Kihaha
                | Cmavo::Kiheha
                | Cmavo::Kihiha
                | Cmavo::Kihoha
                | Cmavo::Kihuha
                | Cmavo::Mahau
                | Cmavo::Mahei
                | Cmavo::Mahoi
                | Cmavo::Mihau
                | Cmavo::Moho
                | Cmavo::Mohu
                | Cmavo::Rahai
                | Cmavo::Rauhi
                | Cmavo::Rohei
                | Cmavo::Sehe
                | Cmavo::Sohai
                | Cmavo::Tihau
                | Cmavo::Tohohe
                | Cmavo::Tuhau
                | Cmavo::Zohei
        ),
        "LAhE" => matches!(
            cmavo,
            Cmavo::Loihe
                | Cmavo::Loihi
                | Cmavo::Mehohe
                | Cmavo::Pihei
                | Cmavo::Pohoi
                | Cmavo::Poihei
                | Cmavo::Tehoi
                | Cmavo::Voihe
        ),
        "LE" => matches!(
            cmavo,
            Cmavo::Lahei | Cmavo::Lehei | Cmavo::Lohei | Cmavo::Mehei | Cmavo::Rihoi | Cmavo::Zohau
        ),
        "LI" => matches!(
            cmavo,
            Cmavo::Bohai | Cmavo::Lihai | Cmavo::Lihei | Cmavo::Maiho
        ),
        "LOhOI" => matches!(cmavo, Cmavo::Mauha | Cmavo::Xauha),
        "LU" => matches!(cmavo, Cmavo::Lahau | Cmavo::Tuhai),
        "ME" => matches!(cmavo, Cmavo::Xohi),
        "MOI" => matches!(cmavo, Cmavo::Moiho),
        "MOhE" => matches!(cmavo, Cmavo::Boihau),
        "NAhE" => matches!(cmavo, Cmavo::Dehai | Cmavo::Nohei),
        "NOI" => matches!(cmavo, Cmavo::Nohoi | Cmavo::Pohoi | Cmavo::Voihi),
        "NOIhA" => matches!(cmavo, Cmavo::Noihoha),
        "NU" => matches!(
            cmavo,
            Cmavo::Jahoi
                | Cmavo::Kahai
                | Cmavo::Kaihai
                | Cmavo::Kihi
                | Cmavo::Paihe
                | Cmavo::Suhai
                | Cmavo::Zahai
        ),
        "PA" => matches!(
            cmavo,
            Cmavo::Duhei
                | Cmavo::Faihu
                | Cmavo::Mehei
                | Cmavo::Sohai
                | Cmavo::Sohei
                | Cmavo::Sohoi
                | Cmavo::Xaihe
                | Cmavo::Xauhe
                | Cmavo::Xohai
                | Cmavo::Xohu
                | Cmavo::Xoihi
                | Cmavo::Zahai
        ),
        "ROI" => matches!(cmavo, Cmavo::Bahoi | Cmavo::Dehei | Cmavo::Xuhau),
        "SE" => matches!(cmavo, Cmavo::Dehai | Cmavo::Nahoi),
        "SEI" => matches!(
            cmavo,
            Cmavo::Saihe | Cmavo::Seihe | Cmavo::Soihe | Cmavo::Suhoi
        ),
        "SEhU" => matches!(cmavo, Cmavo::Xehau),
        "TO" => matches!(cmavo, Cmavo::Mauhe | Cmavo::Noihi),
        "TOI" => matches!(cmavo, Cmavo::Gehuhi | Cmavo::Mauho),
        "UI" => matches!(
            cmavo,
            Cmavo::Ahai
                | Cmavo::Auhau
                | Cmavo::Bahei
                | Cmavo::Buhei
                | Cmavo::Cuhei
                | Cmavo::Eihai
                | Cmavo::Fahai
                | Cmavo::Gahihi
                | Cmavo::Gahuhi
                | Cmavo::Gehai
                | Cmavo::Iahau
                | Cmavo::Ihau
                | Cmavo::Ihei
                | Cmavo::Ihihi
                | Cmavo::Jahohe
                | Cmavo::Jahoho
                | Cmavo::Jihai
                | Cmavo::Jihei
                | Cmavo::Jihohe
                | Cmavo::Jihoho
                | Cmavo::Kehihai
                | Cmavo::Kihai
                | Cmavo::Lahei
                | Cmavo::Lahoi
                | Cmavo::Lehohe
                | Cmavo::Mahai
                | Cmavo::Muhei
                | Cmavo::Nihei
                | Cmavo::Nohoi
                | Cmavo::Oihoi
                | Cmavo::Pohai
                | Cmavo::Saihi
                | Cmavo::Seiha
                | Cmavo::Seihi
                | Cmavo::Sohahu
                | Cmavo::Sohei
                | Cmavo::Suhei
                | Cmavo::Uhohe
                | Cmavo::Uhohi
                | Cmavo::Uhoho
                | Cmavo::Uhohu
                | Cmavo::Uhoi
                | Cmavo::Uihai
                | Cmavo::Vaihe
                | Cmavo::Xauha
                | Cmavo::Xauhe
                | Cmavo::Xauhi
                | Cmavo::Xauho
                | Cmavo::Xauhu
                | Cmavo::Xehiha
                | Cmavo::Xehihe
                | Cmavo::Xehihi
                | Cmavo::Xehiho
                | Cmavo::Xehihu
                | Cmavo::Zahei
                | Cmavo::Zahoha
                | Cmavo::Zohoi
        ),
        "UI3a" => matches!(
            cmavo,
            Cmavo::Ahai
                | Cmavo::Auhau
                | Cmavo::Bahei
                | Cmavo::Buhei
                | Cmavo::Cuhei
                | Cmavo::Eihai
                | Cmavo::Fahai
                | Cmavo::Gahihi
                | Cmavo::Gahuhi
                | Cmavo::Gehai
                | Cmavo::Iahau
                | Cmavo::Ihau
                | Cmavo::Ihei
                | Cmavo::Ihihi
                | Cmavo::Jahohe
                | Cmavo::Jahoho
                | Cmavo::Jihai
                | Cmavo::Jihei
                | Cmavo::Jihohe
                | Cmavo::Jihoho
                | Cmavo::Kehihai
                | Cmavo::Kihai
                | Cmavo::Lahei
                | Cmavo::Lahoi
                | Cmavo::Lehohe
                | Cmavo::Mahai
                | Cmavo::Muhei
                | Cmavo::Nihei
                | Cmavo::Nohoi
                | Cmavo::Oihoi
                | Cmavo::Pohai
                | Cmavo::Saihi
                | Cmavo::Seiha
                | Cmavo::Seihi
                | Cmavo::Sohahu
                | Cmavo::Sohei
                | Cmavo::Suhei
                | Cmavo::Uhohe
                | Cmavo::Uhohi
                | Cmavo::Uhoho
                | Cmavo::Uhohu
                | Cmavo::Uhoi
                | Cmavo::Uihai
                | Cmavo::Vaihe
                | Cmavo::Xauha
                | Cmavo::Xauhe
                | Cmavo::Xauhi
                | Cmavo::Xauho
                | Cmavo::Xauhu
                | Cmavo::Xehiha
                | Cmavo::Xehihe
                | Cmavo::Xehihi
                | Cmavo::Xehiho
                | Cmavo::Xehihu
                | Cmavo::Zahei
                | Cmavo::Zahoha
                | Cmavo::Zohoi
        ),
        "VUhU" => matches!(
            cmavo,
            Cmavo::Dehoha
                | Cmavo::Fehaha
                | Cmavo::Fehahe
                | Cmavo::Fehahi
                | Cmavo::Fehaho
                | Cmavo::Geiha
                | Cmavo::Pihai
                | Cmavo::Sahiha
        ),
        "XI" => matches!(cmavo, Cmavo::Fauhe | Cmavo::Xihe | Cmavo::Xihi),
        "Y" => matches!(cmavo, Cmavo::Ieho),
        "ZOhU" => matches!(cmavo, Cmavo::Gehai | Cmavo::Kehau),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_koha_argument(word: &WithIndicators<WordLike>) -> bool {
    parser_word_is_selmaho(word, Selmaho::Koha)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_relation_word(word: &WithIndicators<WordLike>) -> bool {
    if let WithIndicators::WithIndicator { base, .. } = word {
        return is_relation_word(base);
    }

    if parser_word_is_selmaho(word, Selmaho::Goha) {
        return true;
    }

    match word {
        WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
            word_like_is_relation_word(word_like)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret == (is_relation_word(word) && !parser_word_is_selmaho(word, Selmaho::Goha)))]
pub(crate) fn is_brivla_relation_word(word: &WithIndicators<WordLike>) -> bool {
    is_relation_word(word) && !parser_word_is_selmaho(word, Selmaho::Goha)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_is_relation_word(word_like: &WordLike) -> bool {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => {
            matches!(
                word.kind(),
                WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
            )
        }
        data!(WordLike::ZeiLujvo { .. }) => true,
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_cmevla_word(word: &WithIndicators<WordLike>) -> bool {
    match word {
        WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
            word_like_kind(word_like).is_some_and(|kind| kind == WordKind::Cmevla)
        }
        WithIndicators::WithIndicator { base, .. } => is_cmevla_word(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_letter_word(word: &WithIndicators<WordLike>) -> bool {
    match word {
        WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
            match word_like.as_data() {
                data!(WordLike::Letter { .. }) => true,
                data!(WordLike::Bare(word)) => {
                    let phonemes = word.phonemes();
                    word.kind() == WordKind::Cmavo
                        && ((phonemes.as_str() != "bu" && phonemes.as_str().ends_with("bu"))
                            || word.cmavo().is_some_and(|cmavo| {
                                (!matches!(
                                    cmavo,
                                    Cmavo::A | Cmavo::E | Cmavo::I | Cmavo::O | Cmavo::U
                                ) && cmavo.is_selmaho(Selmaho::By))
                                    || cmavo == Cmavo::Sehe
                                    || cmavo == Cmavo::Y
                            }))
                }
                _ => false,
            }
        }
        WithIndicators::WithIndicator { base, .. } => is_letter_word(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_kind(word_like: &WordLike) -> Option<WordKind> {
    let data!(WordLike::Bare(word)) = word_like.as_data() else {
        return None;
    };
    Some(word.kind())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn bare_word_kind_and_phonemes(
    word: &WithIndicators<WordLike>,
) -> Option<(WordKind, String)> {
    let WithIndicators::Bare(word_like) = word else {
        return None;
    };
    let data!(WordLike::Bare(word)) = word_like.as_data() else {
        return None;
    };
    Some((word.kind(), word.phonemes().into_string()))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn base_word_from_record(word: Word) -> WithIndicators<WordLike> {
    WithIndicators::bare(WordLike::bare(word))
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
pub(super) fn spanned_tokens(words: &[WithIndicators<WordLike>]) -> Vec<SpannedToken> {
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
pub(super) fn word_byte_range(word: &WithIndicators<WordLike>) -> Option<Range<usize>> {
    match word {
        WithIndicators::Bare(word_like) => word_like_byte_range(word_like),
        WithIndicators::Emphasized { bahe, word_like } => {
            word_like_byte_range(word_like).map(|range| {
                bahe.span().byte_start.min(range.start)..bahe.span().byte_end.max(range.end)
            })
        }
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => word_byte_range(base).map(|range| {
            range.start
                ..nai
                    .as_ref()
                    .unwrap_or(indicator)
                    .span()
                    .byte_end
                    .max(range.end)
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => Some(word.span().byte_start..word.span().byte_end),
        data!(WordLike::ZoQuote { zo, word }) => Some(zo.span().byte_start..word.span().byte_end),
        data!(WordLike::ZoiQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span().byte_start..closing_delimiter.span().byte_end),
        data!(WordLike::LohuQuote { lohu, lehu, .. }) => {
            Some(lohu.span().byte_start..lehu.span().byte_end)
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => Some(marker.span().byte_start..quoted_text.span.byte_end),
        data!(WordLike::Letter { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span().byte_end.max(range.end))
        }
        data!(WordLike::ZeiLujvo { left, right, .. }) => word_like_byte_range(left)
            .map(|range| range.start..right.span().byte_end.max(range.end)),
    }
}

#[requires(true)]
#[ensures(matches!(ret, SyntaxError::Parse { ref reason, .. } if !reason.is_empty()) || !matches!(ret, SyntaxError::Parse { .. }))]
pub(super) fn syntax_error(errors: Vec<SyntaxParseError<'_>>) -> SyntaxError {
    let Some(error) = merge_farthest_errors(errors) else {
        return SyntaxError::Parse {
            byte_start: 0,
            byte_end: 0,
            reason: "unknown Chumsky syntax error".to_owned(),
            expected: Vec::new(),
            expectations: Vec::new(),
            context: None,
        };
    };

    let expected = error.expected_strings();
    let reason = match error.reason() {
        RichReason::Custom(message) => message.to_string(),
        RichReason::ExpectedFound { .. } if expected.is_empty() => "unexpected input".to_owned(),
        RichReason::ExpectedFound { .. } => format!("expected {}", expected.join(", ")),
    };
    let expectations = error.expectations();

    SyntaxError::Parse {
        byte_start: error.span().start,
        byte_end: error.span().end,
        reason,
        expected,
        expectations,
        context: error.current_context(),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|summary| !summary.reason.is_empty()))]
pub(super) fn syntax_trace_failure_summary(
    errors: &[SyntaxParseError<'_>],
) -> Option<TraceFailureSummary> {
    let farthest_start = errors.iter().map(|error| error.span().start).max()?;
    let farthest = errors
        .iter()
        .filter(|error| error.span().start == farthest_start)
        .collect::<Vec<_>>();
    let merged = farthest
        .iter()
        .map(|error| (*error).clone())
        .reduce(SyntaxParseError::merge_for_report)?;
    let expected = merged.expected_strings();
    let reason = match merged.reason() {
        RichReason::Custom(message) => message.to_string(),
        RichReason::ExpectedFound { .. } if expected.is_empty() => "unexpected input".to_owned(),
        RichReason::ExpectedFound { .. } => format!("expected {}", expected.join(", ")),
    };
    let branches = farthest
        .into_iter()
        .flat_map(trace_failure_branches)
        .collect::<Vec<_>>();
    Some(new!(TraceFailureSummary {
        byte_start: merged.span().start,
        byte_end: merged.span().end,
        reason,
        branches,
        current_context: merged.current_context().map(trace_context),
    }))
}

#[requires(true)]
#[ensures(true)]
fn trace_failure_branches(error: &SyntaxParseError<'_>) -> Vec<TraceFailureBranch> {
    let expected = error.expected_strings();
    if error.context_paths().is_empty() {
        return vec![TraceFailureBranch {
            contexts: Vec::new(),
            expected,
        }];
    }
    error
        .context_paths()
        .iter()
        .map(|path| TraceFailureBranch {
            contexts: path.iter().cloned().map(trace_context).collect(),
            expected: expected.clone(),
        })
        .collect()
}

#[requires(!context.construct.is_empty())]
#[ensures(ret.construct == context.construct)]
fn trace_context(context: SyntaxConstructContext) -> TraceContext {
    TraceContext::new(
        context.construct.clone(),
        context.byte_start,
        context.byte_end,
    )
}

#[requires(true)]
#[ensures(true)]
fn merge_farthest_errors(errors: Vec<SyntaxParseError<'_>>) -> Option<SyntaxParseError<'_>> {
    let farthest_start = errors.iter().map(|error| error.span().start).max()?;
    errors
        .into_iter()
        .filter(|error| error.span().start == farthest_start)
        .reduce(SyntaxParseError::merge_for_report)
}
