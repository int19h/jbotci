use std::ops::Range;

use crate::{ExperimentalConstruct, Token, WithIndicators, WithIndicatorsData};
use bityzba::{data, invariant, new, requires};
use chumsky::error::RichReason;
use chumsky::input::MapExtra;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_diagnostics::{
    TraceContext, TraceEventKind, TraceFailureBranch, TraceFailureSummary, TraceLevel,
};
use jbotci_morphology::{Cmavo, Selmaho, Word, WordKind, WordLike, WordLikeData};
use jbotci_source::SourceSpan;

use super::{
    BoxedParser, ParseExtra, ParserInput, ParserState, SpannedToken, SyntaxFound, SyntaxFoundData,
    SyntaxParseCustomKind, SyntaxParseError,
};
use crate::{
    SyntaxConstructContext, SyntaxError, SyntaxErrorKind, SyntaxExpectation, SyntaxExpectedToken,
    SyntaxExpectedTokenData, SyntaxWordCategory, syntax_construct_depth,
    syntax_construct_is_descendant_of, syntax_construct_is_known,
    syntax_expectation_summary_message,
};

#[requires(true)]
#[ensures(true)]
pub(super) fn cmavo<'tokens>(cmavo: Cmavo) -> BoxedParser<'tokens, Token> {
    token_matching(
        "cmavo",
        cmavo.canonical_text(),
        vec![new!(SyntaxExpectedToken::Cmavo(cmavo))],
        move |word, state| parser_word_is_cmavo(state, word, cmavo),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn selmaho<'tokens>(selmaho: Selmaho) -> BoxedParser<'tokens, Token> {
    token_matching_with_experimental_context(
        selmaho.name(),
        selmaho.name(),
        vec![new!(SyntaxExpectedToken::Selmaho(selmaho))],
        ExperimentalCmavoContext::Selmaho(selmaho),
        move |word, state| parser_word_is_selmaho(state, word, selmaho),
    )
}

#[requires(!label.is_empty())]
#[requires(!cmavo.is_empty())]
#[ensures(true)]
pub(super) fn cmavo_one_of<'tokens>(
    label: &'static str,
    cmavo: &'static [Cmavo],
) -> BoxedParser<'tokens, Token> {
    token_matching(
        label,
        label,
        cmavo
            .iter()
            .copied()
            .map(|cmavo| new!(SyntaxExpectedToken::Cmavo(cmavo)))
            .collect(),
        move |word, state| parser_word_is_one_of_cmavo(state, word, cmavo),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn le_cmavo<'tokens>() -> BoxedParser<'tokens, Token> {
    selmaho(Selmaho::Le)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn la_cmavo<'tokens>() -> BoxedParser<'tokens, Token> {
    selmaho(Selmaho::La)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn lahe_cmavo<'tokens>() -> BoxedParser<'tokens, Token> {
    selmaho(Selmaho::Lahe)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn pa_word<'tokens>() -> BoxedParser<'tokens, Token> {
    selmaho(Selmaho::Pa)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn na_cmavo<'tokens>() -> BoxedParser<'tokens, Token> {
    selmaho(Selmaho::Na)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn koha_argument<'tokens>() -> BoxedParser<'tokens, Token> {
    token_matching_with_experimental_context(
        "KOhA sumti",
        "KOhA sumti",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::ProSumti,
        ))],
        ExperimentalCmavoContext::Selmaho(Selmaho::Koha),
        |word, state| parser_word_is_selmaho(state, word, Selmaho::Koha),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_word<'tokens>() -> BoxedParser<'tokens, Token> {
    token_matching(
        "selbri word",
        "SELBRI WORD",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::SelbriWord,
        ))],
        |word, _state| is_relation_word(word),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn brivla_relation_word<'tokens>(cbm_enabled: bool) -> BoxedParser<'tokens, Token> {
    let brivla = token_matching(
        "BRIVLA",
        "BRIVLA",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::Brivla
        ))],
        |word, state| is_relation_word(word) && !parser_word_is_selmaho(state, word, Selmaho::Goha),
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
                        ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
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
pub(super) fn cmevla_word<'tokens>() -> BoxedParser<'tokens, Token> {
    token_matching(
        "CMEVLA",
        "CMEVLA",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::Cmevla
        ))],
        |word, _state| is_cmevla_word(word),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn letter_word<'tokens>() -> BoxedParser<'tokens, Token> {
    token_matching_with_experimental_context(
        "lerfu",
        "LERFU",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::LetterWord,
        ))],
        ExperimentalCmavoContext::Selmaho(Selmaho::By),
        |word, _state| is_letter_word(word),
    )
}

#[requires(!label.is_empty())]
#[requires(!debug_label.is_empty())]
#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn token_matching<'tokens>(
    label: &'static str,
    debug_label: &'static str,
    expected: Vec<SyntaxExpectedToken>,
    bridi: impl Fn(&Token, &mut ParserState) -> bool + Clone + 'tokens,
) -> BoxedParser<'tokens, Token> {
    token_matching_with_experimental_context(
        label,
        debug_label,
        expected,
        ExperimentalCmavoContext::Label(label),
        bridi,
    )
}

#[requires(!label.is_empty())]
#[requires(!debug_label.is_empty())]
#[requires(!expected.is_empty())]
#[ensures(true)]
pub(super) fn token_matching_with_experimental_context<'tokens>(
    label: &'static str,
    debug_label: &'static str,
    expected: Vec<SyntaxExpectedToken>,
    experimental_context: ExperimentalCmavoContext,
    bridi: impl Fn(&Token, &mut ParserState) -> bool + Clone + 'tokens,
) -> BoxedParser<'tokens, Token> {
    custom::<_, ParserInput<'tokens>, Token, ParseExtra<'tokens>>(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        match input.next() {
            Some(word)
                if {
                    let state = input.state();
                    bridi(&word, state)
                } =>
            {
                let span = word.core_word().byte_range().unwrap_or(0..0);
                let state: &mut ParserState = input.state();
                warn_experimental_cmavo(state, experimental_context, &word);
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
            Some(word) => {
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
                Err(SyntaxParseError::expected_found(
                    span,
                    expected.clone(),
                    new!(SyntaxFound::Token(word)),
                ))
            }
            None => {
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
                Err(SyntaxParseError::expected_found(
                    span,
                    expected.clone(),
                    new!(SyntaxFound::EndOfInput),
                ))
            }
        }
    })
    .labelled(debug_label)
    .as_terminal()
    .boxed()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Label(_) => true)]
#[invariant(::Selmaho(_) => true)]
pub(super) enum ExperimentalCmavoContext {
    Label(&'static str),
    Selmaho(Selmaho),
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

#[requires(true)]
#[ensures(true)]
fn warn_experimental_cmavo(
    state: &mut ParserState,
    context: ExperimentalCmavoContext,
    word: &Token,
) {
    if let Some(cmavo) = parser_word_cmavo(state, word)
        && let Some(construct) = experimental_construct_for_cmavo(context, cmavo)
    {
        state.warn(construct, word);
    }
    warn_experimental_indicators(state, word);
}

#[requires(true)]
#[ensures(true)]
fn experimental_construct_for_cmavo(
    context: ExperimentalCmavoContext,
    cmavo: Cmavo,
) -> Option<ExperimentalConstruct> {
    match (context, cmavo) {
        (ExperimentalCmavoContext::Selmaho(Selmaho::Coi), Cmavo::Ahoi | Cmavo::Ohai) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryCoiVocative)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Doi), Cmavo::Dahoi) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryDoiVocative)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Faha), Cmavo::Xeihe) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryFahaTag)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Pa), Cmavo::Suhai | Cmavo::Xehe) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryPaNumber)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Ui | Selmaho::Ui3a), Cmavo::Lihoi) => {
            Some(ExperimentalConstruct::ExperimentalDictionaryUiIndicator)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Noiha), Cmavo::Noihoha) => {
            Some(ExperimentalConstruct::ExperimentalZantufaCmavo)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Noiha), _) => {
            Some(ExperimentalConstruct::ExperimentalNoihaAdverbial)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Soi), _) => {
            Some(ExperimentalConstruct::ExperimentalSoiAdverbial)
        }
        (ExperimentalCmavoContext::Selmaho(Selmaho::Lohoi), _) => {
            Some(ExperimentalConstruct::ExperimentalLohOiBridiDescription)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Fihoi) => {
            Some(ExperimentalConstruct::ExperimentalFihoiAdverbial)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Lohai | Cmavo::Sahai | Cmavo::Lehai) => {
            Some(ExperimentalConstruct::ExperimentalLohAiReplacementFree)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Nohoi) => {
            Some(ExperimentalConstruct::ExperimentalNohoiSelbriRelativeClause)
        }
        (
            ExperimentalCmavoContext::Label("cmavo"),
            Cmavo::Bohei | Cmavo::Gohoi | Cmavo::Tahai | Cmavo::Zehoi,
        ) => Some(ExperimentalConstruct::ExperimentalGohoiSelbriUnit),
        (ExperimentalCmavoContext::Selmaho(Selmaho::Lihau | Selmaho::Luhei), _) => {
            Some(ExperimentalConstruct::ExperimentalZantufaLuheiSelbriUnit)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Luhei) => {
            Some(ExperimentalConstruct::ExperimentalZantufaLuheiSelbriUnit)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Muhoi) => {
            Some(ExperimentalConstruct::ExperimentalZantufaMuhoiSelbriUnit)
        }
        (ExperimentalCmavoContext::Label("cmavo"), Cmavo::Xohi) => {
            Some(ExperimentalConstruct::ExperimentalXohiTagSelbri)
        }
        _ if is_general_experimental_cmavo_for_context(context, cmavo) => {
            Some(ExperimentalConstruct::ExperimentalCmavo)
        }
        _ if is_zantufa_experimental_cmavo_for_context(context, cmavo) => {
            Some(ExperimentalConstruct::ExperimentalZantufaCmavo)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn warn_experimental_indicators(state: &mut ParserState, word: &Token) {
    warn_experimental_indicators_inner(state, word.as_indicators(), word);
}

#[requires(true)]
#[ensures(true)]
fn warn_experimental_indicators_inner(
    state: &mut ParserState,
    word: &WithIndicators<WordLike>,
    context: &Token,
) {
    let data!(WithIndicators::WithIndicator {
        base,
        indicator,
        nai,
        ..
    }) = word.as_data()
    else {
        return;
    };

    warn_experimental_indicators_inner(state, base, context);

    if let Some(cmavo_context) = indicator_cmavo_context(indicator)
        && let Some(cmavo) = indicator.cmavo()
        && let Some(construct) = experimental_construct_for_cmavo(cmavo_context, cmavo)
    {
        state.warn_word(construct, context, indicator);
    }

    if let Some(nai) = nai
        && let Some(construct) = experimental_construct_for_cmavo(
            ExperimentalCmavoContext::Selmaho(Selmaho::Nai),
            Cmavo::Nai,
        )
    {
        state.warn_word(construct, context, nai);
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_cmavo_context(indicator: &Word) -> Option<ExperimentalCmavoContext> {
    let cmavo = indicator.cmavo()?;
    if cmavo.is_selmaho(Selmaho::Noi) {
        Some(ExperimentalCmavoContext::Selmaho(Selmaho::Noi))
    } else if cmavo.is_selmaho(Selmaho::Ui) {
        Some(ExperimentalCmavoContext::Selmaho(Selmaho::Ui))
    } else if cmavo.is_selmaho(Selmaho::Cai) {
        Some(ExperimentalCmavoContext::Selmaho(Selmaho::Cai))
    } else if cmavo == Cmavo::Y {
        Some(ExperimentalCmavoContext::Selmaho(Selmaho::Y))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn parser_word_cmavo(state: &mut ParserState, word: &Token) -> Option<Cmavo> {
    state.token_cmavo(word)
}

#[requires(true)]
#[ensures(true)]
fn parser_word_is_cmavo(state: &mut ParserState, word: &Token, cmavo: Cmavo) -> bool {
    parser_word_cmavo(state, word) == Some(cmavo)
}

#[requires(!cmavo.is_empty())]
#[ensures(true)]
fn parser_word_is_one_of_cmavo(state: &mut ParserState, word: &Token, cmavo: &[Cmavo]) -> bool {
    parser_word_cmavo(state, word).is_some_and(|actual| cmavo.contains(&actual))
}

#[requires(true)]
#[ensures(true)]
fn parser_word_is_selmaho(state: &mut ParserState, word: &Token, selmaho: Selmaho) -> bool {
    parser_word_cmavo(state, word).is_some_and(|cmavo| selmaho.contains(cmavo))
}

#[requires(true)]
#[ensures(true)]
fn is_general_experimental_cmavo_for_context(
    context: ExperimentalCmavoContext,
    cmavo: Cmavo,
) -> bool {
    match context {
        ExperimentalCmavoContext::Selmaho(Selmaho::Bai) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::By) => matches!(
            cmavo,
            Cmavo::Ahy | Cmavo::Ehy | Cmavo::Ihy | Cmavo::Iy | Cmavo::Ohy | Cmavo::Uhy | Cmavo::Uy
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Caha) => matches!(cmavo, Cmavo::Bihai),
        ExperimentalCmavoContext::Selmaho(Selmaho::Coi) => matches!(
            cmavo,
            Cmavo::Cohoi | Cmavo::Dihai | Cmavo::Kihai | Cmavo::Sahei
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Koha) => matches!(
            cmavo,
            Cmavo::Mihai | Cmavo::Nauho | Cmavo::Nauhu | Cmavo::Xai | Cmavo::Zuhai
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Lahe) => matches!(cmavo, Cmavo::Zohei),
        ExperimentalCmavoContext::Selmaho(Selmaho::Le) => matches!(
            cmavo,
            Cmavo::Leihe
                | Cmavo::Leihi
                | Cmavo::Loihe
                | Cmavo::Loihi
                | Cmavo::Mohoi
                | Cmavo::Moihoi
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Me) => matches!(cmavo, Cmavo::Mehau),
        ExperimentalCmavoContext::Selmaho(Selmaho::Moi) => matches!(cmavo, Cmavo::Ceiha),
        ExperimentalCmavoContext::Selmaho(Selmaho::Nai) => matches!(cmavo, Cmavo::Jahai),
        ExperimentalCmavoContext::Selmaho(Selmaho::Nahe) => matches!(cmavo, Cmavo::Nahei),
        ExperimentalCmavoContext::Selmaho(Selmaho::Nu) => {
            matches!(cmavo, Cmavo::Kaihu | Cmavo::Poihi | Cmavo::Xehei)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Pa) => {
            matches!(cmavo, Cmavo::Rohoi | Cmavo::Suhoi | Cmavo::Xohe)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Roi) => {
            matches!(cmavo, Cmavo::Muhei | Cmavo::Vahei)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Se) => matches!(
            cmavo,
            Cmavo::Suhei | Cmavo::Tohai | Cmavo::Vohai | Cmavo::Xohai
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Ui) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Vuhu) => matches!(cmavo, Cmavo::Joihi),
        ExperimentalCmavoContext::Selmaho(Selmaho::Xi) => matches!(cmavo, Cmavo::Tehai),
        ExperimentalCmavoContext::Selmaho(Selmaho::Zaho) => matches!(
            cmavo,
            Cmavo::Cohaha
                | Cmavo::Cohauha
                | Cmavo::Cohuha
                | Cmavo::Sauha
                | Cmavo::Xaho
                | Cmavo::Xohu
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Zo) => matches!(cmavo, Cmavo::Mahoi),
        ExperimentalCmavoContext::Selmaho(Selmaho::Zohu) => matches!(cmavo, Cmavo::Cehai),
        _ => false,
    }
}
#[requires(true)]
#[ensures(true)]
fn is_zantufa_experimental_cmavo_for_context(
    context: ExperimentalCmavoContext,
    cmavo: Cmavo,
) -> bool {
    match context {
        ExperimentalCmavoContext::Selmaho(Selmaho::Bai) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::By) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Coi) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Cuhe) => {
            matches!(cmavo, Cmavo::Bahau | Cmavo::Puhau)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Daho) => {
            matches!(cmavo, Cmavo::Daiho | Cmavo::Dohai)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Doi) => matches!(cmavo, Cmavo::Dahei),
        ExperimentalCmavoContext::Selmaho(Selmaho::Faha) => {
            matches!(cmavo, Cmavo::Duhoi | Cmavo::Zuhau)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Goi) => matches!(cmavo, Cmavo::Voihe),
        ExperimentalCmavoContext::Selmaho(Selmaho::Goha) => {
            matches!(cmavo, Cmavo::Ceihi | Cmavo::Gaiho | Cmavo::Xehu)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Jai) => {
            matches!(cmavo, Cmavo::Jahei | Cmavo::Johai)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Joi) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Koha) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Lahe) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Le) => matches!(
            cmavo,
            Cmavo::Lahei | Cmavo::Lehei | Cmavo::Lohei | Cmavo::Mehei | Cmavo::Rihoi | Cmavo::Zohau
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Li) => matches!(
            cmavo,
            Cmavo::Bohai | Cmavo::Lihai | Cmavo::Lihei | Cmavo::Maiho
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Lohoi) => matches!(
            cmavo,
            Cmavo::Lohoi | Cmavo::Mauha | Cmavo::Xauha | Cmavo::Xuhu
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Lu) => {
            matches!(cmavo, Cmavo::Lahau | Cmavo::Tuhai)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Me) => matches!(cmavo, Cmavo::Xohi),
        ExperimentalCmavoContext::Selmaho(Selmaho::Moi) => matches!(cmavo, Cmavo::Moiho),
        ExperimentalCmavoContext::Selmaho(Selmaho::Mohe) => matches!(cmavo, Cmavo::Boihau),
        ExperimentalCmavoContext::Selmaho(Selmaho::Nahe) => {
            matches!(cmavo, Cmavo::Dehai | Cmavo::Nohei)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Noi) => {
            matches!(cmavo, Cmavo::Nohoi | Cmavo::Pohoi | Cmavo::Voihi)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Noiha) => matches!(cmavo, Cmavo::Noihoha),
        ExperimentalCmavoContext::Selmaho(Selmaho::Nu) => matches!(
            cmavo,
            Cmavo::Jahoi
                | Cmavo::Kahai
                | Cmavo::Kaihai
                | Cmavo::Kihi
                | Cmavo::Paihe
                | Cmavo::Suhai
                | Cmavo::Zahai
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Pa) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Roi) => {
            matches!(cmavo, Cmavo::Bahoi | Cmavo::Dehei | Cmavo::Xuhau)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Se) => {
            matches!(cmavo, Cmavo::Dehai | Cmavo::Nahoi)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Sei) => matches!(
            cmavo,
            Cmavo::Saihe | Cmavo::Seihe | Cmavo::Soihe | Cmavo::Suhoi
        ),
        ExperimentalCmavoContext::Selmaho(Selmaho::Sehu) => matches!(cmavo, Cmavo::Xehau),
        ExperimentalCmavoContext::Selmaho(Selmaho::To) => {
            matches!(cmavo, Cmavo::Mauhe | Cmavo::Noihi)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Toi) => {
            matches!(cmavo, Cmavo::Gehuhi | Cmavo::Mauho)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Ui) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Ui3a) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Vuhu) => matches!(
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
        ExperimentalCmavoContext::Selmaho(Selmaho::Xi) => {
            matches!(cmavo, Cmavo::Fauhe | Cmavo::Xihe | Cmavo::Xihi)
        }
        ExperimentalCmavoContext::Selmaho(Selmaho::Y) => matches!(cmavo, Cmavo::Ieho),
        ExperimentalCmavoContext::Selmaho(Selmaho::Zohu) => {
            matches!(cmavo, Cmavo::Gehai | Cmavo::Kehau)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_koha_argument(word: &Token) -> bool {
    word.is_selmaho(Selmaho::Koha)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_relation_word(word: &Token) -> bool {
    is_relation_indicators(word.as_indicators())
}

#[requires(true)]
#[ensures(true)]
fn is_relation_indicators(word: &WithIndicators<WordLike>) -> bool {
    if let data!(WithIndicators::WithIndicator { base, .. }) = word.as_data() {
        return is_relation_indicators(base);
    }

    if word.is_selmaho(Selmaho::Goha) {
        return true;
    }

    match word.as_data() {
        data!(WithIndicators::Plain(word_like))
        | data!(WithIndicators::Emphasized { word_like, .. }) => {
            word_like_is_relation_word(word_like)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret == (is_relation_word(word) && !word.is_selmaho(Selmaho::Goha)))]
pub(crate) fn is_brivla_relation_word(word: &Token) -> bool {
    is_relation_word(word) && !word.is_selmaho(Selmaho::Goha)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_is_relation_word(word_like: &WordLike) -> bool {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => {
            matches!(
                word.kind(),
                WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
            )
        }
        data!(WordLike::ZeiCompound { .. }) => true,
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_cmevla_word(word: &Token) -> bool {
    is_cmevla_indicators(word.as_indicators())
}

#[requires(true)]
#[ensures(true)]
fn is_cmevla_indicators(word: &WithIndicators<WordLike>) -> bool {
    match word.as_data() {
        data!(WithIndicators::Plain(word_like))
        | data!(WithIndicators::Emphasized { word_like, .. }) => {
            word_like_kind(word_like).is_some_and(|kind| kind == WordKind::Cmevla)
        }
        data!(WithIndicators::WithIndicator { base, .. }) => is_cmevla_indicators(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_letter_word(word: &Token) -> bool {
    is_letter_indicators(word.as_indicators())
}

#[requires(true)]
#[ensures(true)]
fn is_letter_indicators(word: &WithIndicators<WordLike>) -> bool {
    match word.as_data() {
        data!(WithIndicators::Plain(word_like))
        | data!(WithIndicators::Emphasized { word_like, .. }) => match word_like.as_data() {
            data!(WordLike::LerfuWord { .. }) => true,
            data!(WordLike::PlainWord(word)) => {
                word.kind() == WordKind::Cmavo
                    && word.cmavo().is_some_and(|cmavo| {
                        (!matches!(cmavo, Cmavo::A | Cmavo::E | Cmavo::I | Cmavo::O | Cmavo::U)
                            && cmavo.is_selmaho(Selmaho::By))
                            || cmavo == Cmavo::Sehe
                            || cmavo == Cmavo::Y
                    })
            }
            _ => false,
        },
        data!(WithIndicators::WithIndicator { base, .. }) => is_letter_indicators(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_kind(word_like: &WordLike) -> Option<WordKind> {
    let data!(WordLike::PlainWord(word)) = word_like.as_data() else {
        return None;
    };
    Some(word.kind())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn bare_word_kind_and_phonemes(word: &Token) -> Option<(WordKind, String)> {
    let data!(WithIndicators::Plain(word_like)) = word.as_indicators().as_data() else {
        return None;
    };
    let data!(WordLike::PlainWord(word)) = word_like.as_data() else {
        return None;
    };
    Some((word.kind(), word.phonemes().into_string()))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn base_word_from_record(word: Word) -> Token {
    Token::bare(WordLike::bare(word))
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
pub(super) fn spanned_tokens(words: &[Token]) -> Vec<SpannedToken> {
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
pub(super) fn word_byte_range(word: &Token) -> Option<Range<usize>> {
    word_indicators_byte_range(word.as_indicators())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_indicators_byte_range(word: &WithIndicators<WordLike>) -> Option<Range<usize>> {
    match word.as_data() {
        data!(WithIndicators::Plain(word_like)) => word_like_byte_range(word_like),
        data!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }) => word_like_byte_range(word_like).map(|range| {
            let start = extra_bahe
                .iter()
                .fold(bahe.span().byte_start, |start, bahe| {
                    start.min(bahe.span().byte_start)
                });
            let end = extra_bahe.iter().fold(bahe.span().byte_end, |end, bahe| {
                end.max(bahe.span().byte_end)
            });
            start.min(range.start)..end.max(range.end)
        }),
        data!(WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }) => word_indicators_byte_range(base).map(|range| {
            let indicator_start = indicator_bahe
                .iter()
                .fold(indicator.span().byte_start, |start, bahe| {
                    start.min(bahe.span().byte_start)
                });
            let end = nai
                .as_ref()
                .map(|nai| {
                    nai_bahe.iter().fold(nai.span().byte_end, |end, bahe| {
                        end.max(bahe.span().byte_end)
                    })
                })
                .unwrap_or_else(|| indicator.span().byte_end);
            range.start
                ..end
                    .max(indicator_start)
                    .max(indicator.span().byte_end)
                    .max(range.end)
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => Some(word.span().byte_start..word.span().byte_end),
        data!(WordLike::QuotedWord { zo, word }) => {
            Some(zo.span().byte_start..word.span().byte_end)
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span().byte_start..closing_delimiter.span().byte_end),
        data!(WordLike::QuotedWords { lohu, lehu, .. }) => {
            Some(lohu.span().byte_start..lehu.span().byte_end)
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => Some(marker.span().byte_start..quoted_text.span.byte_end),
        data!(WordLike::LerfuWord { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span().byte_end.max(range.end))
        }
        data!(WordLike::ZeiCompound { left, right, .. }) => word_like_byte_range(left)
            .map(|range| range.start..right.span().byte_end.max(range.end)),
    }
}

#[requires(true)]
#[ensures(matches!(ret, SyntaxError::Parse { ref reason, .. } if !reason.is_empty()) || !matches!(ret, SyntaxError::Parse { .. }))]
pub(super) fn syntax_error(
    errors: Vec<SyntaxParseError<'_>>,
    error_context_depth: usize,
) -> SyntaxError {
    let Some(error) = merge_farthest_errors(errors) else {
        return SyntaxError::Parse {
            kind: SyntaxErrorKind::InvalidConstruct,
            byte_start: 0,
            byte_end: 0,
            reason: "unknown Chumsky syntax error".to_owned(),
            expected: Vec::new(),
            expectations: Vec::new(),
            contexts: Vec::new(),
        };
    };
    let preferred_context = error.preferred_context();
    let error = error.into_report_error();

    let expectations = error.expectations();
    let expected = error.expected_strings();
    let current_context = error.current_context().or(preferred_context);
    let contexts = error.report_contexts(error_context_depth);
    let summary_context = error.summary_context().or_else(|| current_context.clone());
    let kind = syntax_error_kind(&error, &expectations, current_context.as_ref());
    let reason = syntax_error_reason(
        error.reason(),
        &expected,
        &expectations,
        summary_context
            .as_ref()
            .map(|context| context.construct.as_str()),
    );

    let byte_start = error.span().start.min(error.span().end);
    let byte_end = error.span().start.max(error.span().end);
    SyntaxError::Parse {
        kind,
        byte_start,
        byte_end,
        reason,
        expected,
        expectations,
        contexts,
    }
}

#[requires(true)]
#[ensures(matches!(ret, SyntaxError::Parse { ref reason, .. } if !reason.is_empty()) || !matches!(ret, SyntaxError::Parse { .. }))]
pub(super) fn syntax_error_with_diagnostic_candidate<'tokens>(
    mut errors: Vec<SyntaxParseError<'tokens>>,
    diagnostic_candidate: Option<SyntaxParseError<'tokens>>,
    error_context_depth: usize,
) -> SyntaxError {
    if let Some(candidate) = diagnostic_candidate {
        let root_farthest_start = errors.iter().map(|error| error.span().start).max();
        if root_farthest_start.is_none_or(|start| candidate.span().start > start)
            && diagnostic_candidate_refines_root_context(&candidate, &errors)
            || root_farthest_start.is_some_and(|start| {
                candidate.span().start == start
                    && diagnostic_candidate_matches_root_context(&candidate, &errors)
            })
        {
            errors.push(candidate);
        }
    }
    syntax_error(errors, error_context_depth)
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_candidate_refines_root_context(
    candidate: &SyntaxParseError<'_>,
    errors: &[SyntaxParseError<'_>],
) -> bool {
    let Some(root_farthest_start) = errors.iter().map(|error| error.span().start).max() else {
        return true;
    };
    let Some(candidate_context) = candidate.preferred_context() else {
        return true;
    };
    errors
        .iter()
        .filter(|error| error.span().start == root_farthest_start)
        .any(|error| match error.preferred_context() {
            None => true,
            Some(root) => {
                candidate_context.construct == root.construct
                    || syntax_construct_is_descendant_of(
                        &candidate_context.construct,
                        &root.construct,
                    )
            }
        })
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_candidate_matches_root_context(
    candidate: &SyntaxParseError<'_>,
    errors: &[SyntaxParseError<'_>],
) -> bool {
    let Some(root_farthest_start) = errors.iter().map(|error| error.span().start).max() else {
        return true;
    };
    let candidate_context = candidate.preferred_context();
    errors
        .iter()
        .filter(|error| error.span().start == root_farthest_start)
        .any(
            |error| match (error.preferred_context(), candidate_context.as_ref()) {
                (None, _) | (_, None) => true,
                (Some(root), Some(candidate)) => root.construct == candidate.construct,
            },
        )
}

#[requires(true)]
#[ensures(true)]
fn syntax_error_kind(
    error: &SyntaxParseError<'_>,
    expectations: &[SyntaxExpectation],
    context: Option<&SyntaxConstructContext>,
) -> SyntaxErrorKind {
    if let Some(found) = error.found() {
        if let data!(SyntaxFound::Token(token)) = found.as_data() {
            return syntax_error_kind_for_token(token);
        }
    }
    if error.custom_kind() == Some(SyntaxParseCustomKind::BridiTailKeContinuationConflict) {
        return SyntaxErrorKind::InvalidBridiTailConnection;
    }
    if error
        .found()
        .is_some_and(|found| matches!(found.as_data(), data!(SyntaxFound::EndOfInput)))
        || error.span().start == error.span().end
    {
        return syntax_incomplete_kind(context, expectations);
    }
    match error.reason() {
        RichReason::Custom(_) => SyntaxErrorKind::InvalidConstruct,
        RichReason::ExpectedFound { .. } => SyntaxErrorKind::UnexpectedWord,
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_error_kind_for_token(token: &Token) -> SyntaxErrorKind {
    syntax_error_kind_for_word_like(token.core_word())
}

#[requires(true)]
#[ensures(true)]
fn syntax_error_kind_for_word_like(word_like: &WordLike) -> SyntaxErrorKind {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => match word.kind() {
            WordKind::Cmavo => SyntaxErrorKind::UnexpectedCmavo,
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => {
                SyntaxErrorKind::UnexpectedBrivla
            }
            WordKind::Cmevla => SyntaxErrorKind::UnexpectedCmevla,
        },
        data!(WordLike::QuotedWord { .. })
        | data!(WordLike::DelimitedNonLojbanQuote { .. })
        | data!(WordLike::QuotedWords { .. })
        | data!(WordLike::DelimitedWordQuote { .. }) => SyntaxErrorKind::UnexpectedQuote,
        data!(WordLike::LerfuWord { .. }) => SyntaxErrorKind::UnexpectedLerfu,
        data!(WordLike::ZeiCompound { .. }) => SyntaxErrorKind::UnexpectedZeiCompound,
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind(
    context: Option<&SyntaxConstructContext>,
    expectations: &[SyntaxExpectation],
) -> SyntaxErrorKind {
    if let Some(kind) = syntax_incomplete_kind_from_committed_expectations(expectations, context) {
        return kind;
    }
    if let Some(context) = context
        && let Some(kind) = syntax_incomplete_kind_for_construct(&context.construct)
    {
        return kind;
    }
    syntax_incomplete_kind_from_expectations(expectations).unwrap_or(SyntaxErrorKind::UnexpectedEnd)
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_from_committed_expectations(
    expectations: &[SyntaxExpectation],
    context: Option<&SyntaxConstructContext>,
) -> Option<SyntaxErrorKind> {
    let mut selected = None;
    for expectation in expectations {
        let data!(crate::SyntaxExpectationReason::ContinueCurrent { construct }) =
            expectation.reason.as_data()
        else {
            continue;
        };
        if !incomplete_expectation_context_is_compatible(construct, context) {
            continue;
        }
        let candidate = syntax_incomplete_kind_candidate_for_construct(construct)?;
        selected = select_committed_incomplete_kind_candidate(selected, candidate);
    }
    selected.map(|candidate| candidate.kind)
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn incomplete_expectation_context_is_compatible(
    construct: &str,
    context: Option<&SyntaxConstructContext>,
) -> bool {
    let Some(context) = context else {
        return true;
    };
    construct == context.construct
        || syntax_construct_is_descendant_of(construct, &context.construct)
        || syntax_construct_is_descendant_of(&context.construct, construct)
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_from_expectations(
    expectations: &[SyntaxExpectation],
) -> Option<SyntaxErrorKind> {
    let mut selected = None;
    for expectation in expectations {
        let candidate =
            syntax_incomplete_kind_candidate_for_expectation_reason(&expectation.reason)?;
        selected = select_incomplete_kind_candidate(selected, candidate);
    }
    selected.map(|candidate| candidate.kind)
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_for_expectation_reason(
    reason: &crate::SyntaxExpectationReason,
) -> Option<SyntaxErrorKind> {
    syntax_incomplete_kind_candidate_for_expectation_reason(reason).map(|candidate| candidate.kind)
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_candidate_for_expectation_reason(
    reason: &crate::SyntaxExpectationReason,
) -> Option<IncompleteKindCandidate> {
    match reason.as_data() {
        data!(crate::SyntaxExpectationReason::ContinueCurrent { construct })
        | data!(crate::SyntaxExpectationReason::StartNested { construct }) => {
            syntax_incomplete_kind_candidate_for_construct(construct)
        }
        data!(crate::SyntaxExpectationReason::EndThenStart { starts, ends }) => {
            if starts == "end of input" {
                syntax_incomplete_kind_candidate_for_constructs(ends.iter().map(String::as_str))
            } else {
                syntax_incomplete_kind_candidate_for_construct(starts)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_for_constructs<'a>(
    constructs: impl Iterator<Item = &'a str>,
) -> Option<SyntaxErrorKind> {
    syntax_incomplete_kind_candidate_for_constructs(constructs).map(|candidate| candidate.kind)
}

#[requires(true)]
#[ensures(true)]
fn syntax_incomplete_kind_candidate_for_constructs<'a>(
    constructs: impl Iterator<Item = &'a str>,
) -> Option<IncompleteKindCandidate> {
    let mut selected = None;
    for construct in constructs {
        let candidate = syntax_incomplete_kind_candidate_for_construct(construct)?;
        selected = select_incomplete_kind_candidate(selected, candidate);
    }
    selected
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct IncompleteKindCandidate {
    kind: SyntaxErrorKind,
    depth: usize,
}

#[requires(true)]
#[ensures(true)]
fn select_incomplete_kind_candidate(
    selected: Option<IncompleteKindCandidate>,
    candidate: IncompleteKindCandidate,
) -> Option<IncompleteKindCandidate> {
    match selected {
        None => Some(candidate),
        Some(selected) if candidate.depth < selected.depth => Some(candidate),
        Some(selected) => Some(selected),
    }
}

#[requires(true)]
#[ensures(true)]
fn select_committed_incomplete_kind_candidate(
    selected: Option<IncompleteKindCandidate>,
    candidate: IncompleteKindCandidate,
) -> Option<IncompleteKindCandidate> {
    match selected {
        None => Some(candidate),
        Some(selected) if candidate.depth > selected.depth => Some(candidate),
        Some(selected) => Some(selected),
    }
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_incomplete_kind_candidate_for_construct(
    construct: &str,
) -> Option<IncompleteKindCandidate> {
    let kind = syntax_incomplete_kind_for_construct(construct)?;
    let depth = if syntax_construct_is_known(construct) {
        syntax_construct_depth(construct)
    } else {
        usize::MAX
    };
    Some(IncompleteKindCandidate { kind, depth })
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_incomplete_kind_for_construct(construct: &str) -> Option<SyntaxErrorKind> {
    if is_forethought_connection_construct(construct) {
        Some(SyntaxErrorKind::IncompleteForethoughtConnection)
    } else if construct == "mex"
        || construct == "number sumti"
        || syntax_construct_is_descendant_of("mex", construct)
        || syntax_construct_is_descendant_of("number sumti", construct)
    {
        Some(SyntaxErrorKind::IncompleteMekso)
    } else if construct == "quote" || syntax_construct_is_descendant_of("quote", construct) {
        Some(SyntaxErrorKind::IncompleteQuote)
    } else if construct == "free modifier"
        || syntax_construct_is_descendant_of("free modifier", construct)
    {
        Some(SyntaxErrorKind::IncompleteFreeModifier)
    } else if construct == "sumti" || syntax_construct_is_descendant_of("sumti", construct) {
        Some(SyntaxErrorKind::IncompleteSumti)
    } else if construct == "selbri"
        || construct == "tanru"
        || construct == "tanru unit"
        || syntax_construct_is_descendant_of("selbri", construct)
    {
        Some(SyntaxErrorKind::IncompleteSelbri)
    } else if construct == "bridi" || construct == "subbridi" {
        Some(SyntaxErrorKind::IncompleteBridi)
    } else if construct == "term"
        || construct == "terms"
        || construct == "tail terms"
        || construct == "termset"
        || construct == "tag"
        || construct == "place tag"
        || construct == "NA KU term"
        || syntax_construct_is_descendant_of("term", construct)
        || syntax_construct_is_descendant_of("terms", construct)
    {
        Some(SyntaxErrorKind::IncompleteTerm)
    } else if construct == "statement"
        || construct == "fragment"
        || construct == "prenex"
        || construct == "text group"
        || syntax_construct_is_descendant_of("statement", construct)
    {
        Some(SyntaxErrorKind::IncompleteStatement)
    } else if construct == "text" {
        Some(SyntaxErrorKind::IncompleteText)
    } else {
        None
    }
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn is_forethought_connection_construct(construct: &str) -> bool {
    construct == "forethought mex"
        || (construct.starts_with("forethought ") && construct.ends_with(" connection"))
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
        .reduce(SyntaxParseError::merge_for_parser)?;
    let preferred_context = merged.preferred_context();
    let merged = merged.into_report_error();
    let expectations = merged.expectations();
    let expected = merged.expected_strings();
    let current_context = merged.current_context().or(preferred_context);
    let summary_context = merged.summary_context().or_else(|| current_context.clone());
    let reason = syntax_error_reason(
        merged.reason(),
        &expected,
        &expectations,
        summary_context
            .as_ref()
            .map(|context| context.construct.as_str()),
    );
    let branches = farthest
        .into_iter()
        .flat_map(trace_failure_branches)
        .collect::<Vec<_>>();
    Some(new!(TraceFailureSummary {
        byte_start: merged.span().start,
        byte_end: merged.span().end,
        reason,
        branches,
        current_context: current_context.map(trace_context),
    }))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_error_reason(
    reason: &RichReason<'_, Token>,
    expected: &[String],
    expectations: &[crate::SyntaxExpectation],
    summary_scope: Option<&str>,
) -> String {
    if !expectations.is_empty() {
        return syntax_expectation_summary_message(expectations, summary_scope);
    }
    match reason {
        RichReason::Custom(message) => message.to_string(),
        RichReason::ExpectedFound { .. } if expected.is_empty() => "unexpected input".to_owned(),
        RichReason::ExpectedFound { .. } => format!("expected {}", expected.join(", ")),
    }
}

#[requires(true)]
#[ensures(true)]
fn trace_failure_branches(error: &SyntaxParseError<'_>) -> Vec<TraceFailureBranch> {
    let error = error.clone().into_report_error();
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
        .reduce(SyntaxParseError::merge_for_parser)
}
