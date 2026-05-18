use std::ops::Range;

use bityzba::{data, invariant, new, requires};
use chumsky::Boxed;
use chumsky::error::{Rich, RichReason};
use chumsky::input::MappedInput;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_morphology::{
    Word, WordKind, WordLike, WordLikeData, WordWithModifiers, WordWithModifiersData,
};
use jbotci_source::SourceSpan;

use crate::{
    Connective, Fragment, FreeModifier, LojbanText, Paragraph, ParagraphStatement, ParseOptions,
    Statement, SyntaxError, SyntaxField, SyntaxParse, SyntaxValue,
};

mod parser;
mod render;

type Span = SimpleSpan;
type Token = WordWithModifiers;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens, Token, Span, &'tokens [SpannedToken]>;
type ParseExtra<'tokens> = extra::Err<Rich<'tokens, Token, Span>>;
type BoxedParser<'tokens, O> =
    Boxed<'tokens, 'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>>;

const PA_WORDS: &[&str] = &[
    "dau", "fei", "gai", "jau", "rei", "vai", "pi'e", "pi", "fi'u", "za'u", "me'i", "ni'u", "ki'o",
    "ce'i", "ma'u", "ra'e", "da'a", "so'a", "ji'i", "su'o", "su'e", "ro", "rau", "so'u", "so'i",
    "so'e", "so'o", "mo'a", "du'e", "te'o", "ka'o", "ci'i", "tu'o", "xo", "pai", "ro'oi", "su'oi",
    "xo'e", "no'o", "no", "pa", "re", "ci", "vo", "mu", "xa", "ze", "bi", "so", "0", "1", "2", "3",
    "4", "5", "6", "7", "8", "9",
];
const MOI_WORDS: &[&str] = &["moi", "mei", "si'e", "cu'o", "va'e", "cei'a"];
const MAI_WORDS: &[&str] = &["mo'o", "mai"];
const LAU_WORDS: &[&str] = &["lau", "tau", "zai", "ce'a"];
const CAI_WORDS: &[&str] = &["pei", "cai", "cu'i", "sai", "ru'e"];
const CAHA_WORDS: &[&str] = &["ca'a", "pu'i", "nu'o", "ka'e", "bi'ai"];
const KOHA_WORDS: &[&str] = &[
    "da'u", "da'e", "di'u", "di'e", "de'u", "de'e", "dei", "do'i", "mi'o", "ma'a", "mi'a", "do'o",
    "ko'a", "fo'u", "ko'e", "ko'i", "ko'o", "ko'u", "fo'a", "fo'e", "fo'i", "fo'o", "vo'a", "vo'e",
    "vo'i", "vo'o", "vo'u", "ru", "ri", "ra", "ta", "tu", "ti", "zi'o", "ke'a", "ma", "zu'i",
    "zo'e", "ce'u", "mi'ai", "nau'o", "nau'u", "xai", "zu'ai", "da", "de", "di", "ko", "mi", "do",
];
const GOHA_WORDS: &[&str] = &[
    "mo", "nei", "go'u", "go'o", "go'i", "no'a", "go'e", "go'a", "du", "bu'a", "bu'e", "bu'i",
    "co'e",
];
const FA_WORDS: &[&str] = &["fa", "fe", "fi", "fo", "fu", "fai", "fi'a"];
const UI_WORDS: &[&str] = &[
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
const VUHU_WORDS: &[&str] = &[
    "ge'a", "fu'u", "pi'i", "fe'i", "vu'u", "su'i", "ju'u", "gei", "pa'i", "fa'i", "te'a", "cu'a",
    "va'a", "ne'o", "de'o", "fe'a", "sa'o", "ri'o", "sa'i", "pi'a", "si'i", "joi'i",
];
const NU_WORDS: &[&str] = &[
    "nu", "ni", "du'u", "si'o", "li'i", "ka", "jei", "su'u", "zu'o", "mu'e", "pu'u", "za'i",
    "kai'u", "poi'i", "xe'ei",
];
const COI_WORDS: &[&str] = &[
    "ju'i", "coi", "fi'i", "ta'a", "mu'o", "fe'o", "co'o", "pe'u", "ke'o", "nu'e", "re'i", "be'e",
    "je'e", "mi'e", "ki'e", "vi'o", "co'oi", "di'ai", "ki'ai", "sa'ei", "a'oi", "o'ai",
];

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BasicPredicate {
    leading_terms: Vec<TermSyntax>,
    cu: Option<WordWithModifiers>,
    cu_free_modifiers: Vec<FreeModifierSyntax>,
    relation: RelationSyntax,
    tail_terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
    tail_free_modifiers: Vec<FreeModifierSyntax>,
    gek_sentence: Option<GekSentenceSyntax>,
    bo_continuation: Option<PredicateTailBoContinuationSyntax>,
    ke_continuation: Option<PredicateTailKeContinuationSyntax>,
    continuations: Vec<PredicateTailContinuationSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct PredicateTailBoContinuationSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<TenseModalSyntax>,
    bo: WordWithModifiers,
    free_modifiers: Vec<FreeModifierSyntax>,
    cu: Option<WordWithModifiers>,
    cu_free_modifiers: Vec<FreeModifierSyntax>,
    predicate_tail: Box<BasicPredicate>,
    tail_terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct PredicateTailKeContinuationSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<TenseModalSyntax>,
    ke: WordWithModifiers,
    ke_free_modifiers: Vec<FreeModifierSyntax>,
    predicate_tail: Box<BasicPredicate>,
    kehe: Option<WordWithModifiers>,
    kehe_free_modifiers: Vec<FreeModifierSyntax>,
    tail_terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum GekSentenceSyntax {
    Pair {
        gek: ConnectiveSyntax,
        first: Box<SubsentenceSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubsentenceSyntax>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Na {
        na: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner: Box<GekSentenceSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum SubsentenceSyntax {
    Plain(BasicPredicate),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WordWithModifiers,
        zohu_free_modifiers: Vec<FreeModifierSyntax>,
        inner_subsentence: Box<SubsentenceSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct PredicateTailContinuationSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<TenseModalSyntax>,
    cu: Option<WordWithModifiers>,
    cu_free_modifiers: Vec<FreeModifierSyntax>,
    relation: RelationSyntax,
    terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
    bo_continuation: Option<PredicateTailBoContinuationSyntax>,
    tail_terms: Vec<TermSyntax>,
    tail_vau: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct TextSyntax {
    leading_nai: Vec<WordWithModifiers>,
    leading_cmevla: Vec<WordWithModifiers>,
    leading_indicators: Vec<WordWithModifiers>,
    leading_free_modifiers: Vec<FreeModifierSyntax>,
    leading_connective: Option<ConnectiveSyntax>,
    paragraphs: Vec<ParagraphSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ParagraphSyntax {
    i: Option<WordWithModifiers>,
    niho: Vec<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
    statements: Vec<ParagraphStatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ParagraphStatementSyntax {
    i: Option<WordWithModifiers>,
    connective: Option<ConnectiveSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
    statement: Option<StatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum FreeModifierSyntax {
    Sei {
        sei: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        terms: Vec<TermSyntax>,
        cu: Option<WordWithModifiers>,
        cu_free_modifiers: Vec<FreeModifierSyntax>,
        relation: RelationSyntax,
        sehu: Option<WordWithModifiers>,
        sehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    To {
        to: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        text: Box<TextSyntax>,
        toi: Option<WordWithModifiers>,
        toi_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Xi {
        xi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        expression: MathExpressionSyntax,
    },
    Mai {
        number: Vec<WordWithModifiers>,
        mai: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Soi {
        soi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        leading_argument: Box<ArgumentSyntax>,
        trailing_argument: Option<Box<ArgumentSyntax>>,
        sehu: Option<WordWithModifiers>,
        sehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Vocative {
        vocative_markers: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
        argument: Option<ArgumentSyntax>,
        dohu: Option<WordWithModifiers>,
        dohu_free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum StatementSyntax {
    Tuhe {
        tense_modal: Option<TenseModalSyntax>,
        tuhe: WordWithModifiers,
        tuhe_free_modifiers: Vec<FreeModifierSyntax>,
        text: Box<TextSyntax>,
        tuhu: Option<WordWithModifiers>,
        tuhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WordWithModifiers,
        zohu_free_modifiers: Vec<FreeModifierSyntax>,
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(BasicPredicate),
    Connected {
        i: WordWithModifiers,
        connective: ConnectiveSyntax,
        leading_statement: Box<StatementSyntax>,
        trailing_statement: Box<StatementSyntax>,
    },
    PreIConnected {
        connective: ConnectiveSyntax,
        i: WordWithModifiers,
        leading_statement: Box<StatementSyntax>,
        trailing_statement: Box<StatementSyntax>,
    },
    Iau {
        inner_statement: Box<StatementSyntax>,
        iau: WordWithModifiers,
        iau_free_modifiers: Vec<FreeModifierSyntax>,
        reset_terms: Vec<TermSyntax>,
    },
    ExperimentalPredicateContinuation {
        leading_statement: Box<StatementSyntax>,
        continuation: PredicateStatementContinuationSyntax,
    },
    Fragment(FragmentSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct PredicateStatementContinuationSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<TenseModalSyntax>,
    marker: PredicateStatementContinuationMarkerSyntax,
    trailing_subsentence: SubsentenceSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum PredicateStatementContinuationMarkerSyntax {
    Bo {
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum FragmentSyntax {
    // v0 exposes this constructor even though the current grammar produces
    // TermFragment for parsed standalone arguments.
    #[allow(dead_code)]
    Argument {
        argument: ArgumentSyntax,
    },
    Ek {
        connective: ConnectiveSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Gihek {
        connective: ConnectiveSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Other {
        words: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    // v0 exposes this constructor for a fragment shape that is currently parsed
    // through VocativeFree when it appears in source text.
    #[allow(dead_code)]
    Vocative {
        vocative_markers: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
        vocative_argument: Option<ArgumentSyntax>,
        dohu: Option<WordWithModifiers>,
        dohu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ijek {
        i: WordWithModifiers,
        connective: ConnectiveSyntax,
    },
    Prenex {
        terms: Vec<TermSyntax>,
        zohu: WordWithModifiers,
        zohu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    BeLink {
        be: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        fa: Option<WordWithModifiers>,
        fa_free_modifiers: Vec<FreeModifierSyntax>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WordWithModifiers>,
        beho_free_modifiers: Vec<FreeModifierSyntax>,
    },
    BeiLink {
        bei_only_links: Vec<BeiLinkSyntax>,
    },
    RelativeClause {
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    MathExpression {
        math_expression: MathExpressionSyntax,
    },
    Term {
        terms: Vec<TermSyntax>,
        vau: Option<WordWithModifiers>,
        vau_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Relation {
        relation: RelationSyntax,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum TermSyntax {
    NuhiTermset {
        nuhi: WordWithModifiers,
        nuhi_free_modifiers: Vec<FreeModifierSyntax>,
        termset: Vec<TermSyntax>,
        nuhu: Option<WordWithModifiers>,
        nuhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    GekNuhiTermset {
        m_nuhi: Option<WordWithModifiers>,
        nuhi_free_modifiers: Vec<FreeModifierSyntax>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WordWithModifiers>,
        nuhu_free_modifiers: Vec<FreeModifierSyntax>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gik_nuhu: Option<WordWithModifiers>,
        gik_nuhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Cehe {
        leading_terms: Vec<TermSyntax>,
        cehe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_terms: Vec<TermSyntax>,
    },
    Pehe {
        leading_terms: Vec<TermSyntax>,
        pehe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Argument(ArgumentSyntax),
    Fa {
        fa: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        argument: ArgumentSyntax,
        ku: Option<WordWithModifiers>,
        ku_free_modifiers: Vec<FreeModifierSyntax>,
    },
    NaKu {
        na: WordWithModifiers,
        na_ku: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    BareNa {
        na: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    NoihaAdverbial {
        noiha: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<RelationSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        fehu: Option<WordWithModifiers>,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    PoihaBrigahi {
        poiha: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<RelationSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        brigahi_ku: WordWithModifiers,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    FihoiAdverbial {
        fihoi: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: Box<SubsentenceSyntax>,
        fihau: Option<WordWithModifiers>,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    SoiAdverbial {
        soi: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: Box<SubsentenceSyntax>,
        sehu: Option<WordWithModifiers>,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Tagged {
        tense_modal: Option<TenseModalSyntax>,
        free_modifiers: Vec<FreeModifierSyntax>,
        argument: ArgumentSyntax,
    },
    Connected {
        leading_terms: Vec<TermSyntax>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    BoConnected {
        leading_terms: Vec<TermSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_term: Box<TermSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum TermWrapperKindSyntax {
    Lahe,
    NaheBo,
    Nahe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ArgumentConnectionSyntax {
    connective: ConnectiveSyntax,
    argument: Box<ArgumentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum ArgumentSyntax {
    Quote {
        quote: QuoteSyntax,
    },
    MathExpression {
        li: WordWithModifiers,
        li_free_modifiers: Vec<FreeModifierSyntax>,
        expression: MathExpressionSyntax,
        loho: Option<WordWithModifiers>,
        loho_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Letter {
        letter: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
        boi_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
        inner_argument: Box<ArgumentSyntax>,
    },
    RelativeClause {
        base_argument: Box<ArgumentSyntax>,
        vuho: Option<WordWithModifiers>,
        vuho_free_modifiers: Vec<FreeModifierSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Vuho {
        base_argument: Box<ArgumentSyntax>,
        vuho_marker: WordWithModifiers,
        vuho_free_modifiers: Vec<FreeModifierSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        connected_argument: Option<ArgumentConnectionSyntax>,
    },
    BridiDescription {
        lohoi: WordWithModifiers,
        lohoi_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WordWithModifiers>,
        kuhau_free_modifiers: Vec<FreeModifierSyntax>,
    },
    NaKu {
        na: WordWithModifiers,
        ku: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Tagged {
        tag_words: Vec<WordWithModifiers>,
        tag_tense_modal: Option<TenseModalSyntax>,
        tag_fa: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WordWithModifiers,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WordWithModifiers>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Nahe {
        nahe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WordWithModifiers>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WordWithModifiers,
        wrapper_bo: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_term: Box<TermSyntax>,
        luhu: Option<WordWithModifiers>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Koha {
        koha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zohe {
        tag_words: Vec<WordWithModifiers>,
        maybe_ku: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        lahe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WordWithModifiers>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Bo {
        leading_argument: Box<ArgumentSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Gek {
        gek: ConnectiveSyntax,
        leading_argument: Box<ArgumentSyntax>,
        gik: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Descriptor {
        descriptor: DescriptorSyntax,
    },
    Name {
        la: WordWithModifiers,
        la_free_modifiers: Vec<FreeModifierSyntax>,
        names: Vec<WordWithModifiers>,
        name_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Cmevla {
        cmevla: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    RelationVocative {
        leading_relative_clauses: Vec<RelativeClauseSyntax>,
        relation: RelationSyntax,
        trailing_relative_clauses: Vec<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum RelativeClauseSyntax {
    Goi(GoiRelativeClauseSyntax),
    Noi {
        noi: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: SubsentenceSyntax,
        kuho: Option<WordWithModifiers>,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Poi {
        poi: WordWithModifiers,
        leading_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: SubsentenceSyntax,
        kuho: Option<WordWithModifiers>,
        trailing_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zihe {
        zihe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner: Box<RelativeClauseSyntax>,
    },
    Connected {
        connective: ConnectiveSyntax,
        inner: Box<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GoiRelativeClauseSyntax {
    goi: WordWithModifiers,
    leading_free_modifiers: Vec<FreeModifierSyntax>,
    argument: ArgumentSyntax,
    gehu: Option<WordWithModifiers>,
    trailing_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SelbriRelativeClauseSyntax {
    nohoi: WordWithModifiers,
    leading_free_modifiers: Vec<FreeModifierSyntax>,
    relation: RelationSyntax,
    kuhoi: Option<WordWithModifiers>,
    trailing_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum QuoteSyntax {
    Lu {
        lu: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        text: TextSyntax,
        lihu: Option<WordWithModifiers>,
        lihu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zo {
        zo: WordWithModifiers,
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    ZohOi {
        zohoi: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zoi {
        zoi: WordWithModifiers,
        opening_delimiter: WordWithModifiers,
        closing_delimiter: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Laho {
        laho: WordWithModifiers,
        opening_delimiter: WordWithModifiers,
        closing_delimiter: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lohu {
        lohu: WordWithModifiers,
        quoted_words: Vec<WordWithModifiers>,
        lehu: WordWithModifiers,
        lehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct DescriptorSyntax {
    descriptor: Option<WordWithModifiers>,
    descriptor_free_modifiers: Vec<FreeModifierSyntax>,
    outer_quantifier: Option<QuantifierSyntax>,
    tail_elements: Vec<ArgumentTailElementSyntax>,
    relation: Option<RelationSyntax>,
    relative_clauses: Vec<RelativeClauseSyntax>,
    ku: Option<WordWithModifiers>,
    ku_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ConnectiveSyntax {
    kind: ConnectiveKind,
    se: Option<WordWithModifiers>,
    nahe: Option<WordWithModifiers>,
    na: Option<WordWithModifiers>,
    cmavo: Vec<WordWithModifiers>,
    nai: Option<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BeiLinkSyntax {
    bei: WordWithModifiers,
    bei_free_modifiers: Vec<FreeModifierSyntax>,
    fa: Option<WordWithModifiers>,
    fa_free_modifiers: Vec<FreeModifierSyntax>,
    argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkArgumentSyntax {
    fa: Option<WordWithModifiers>,
    fa_free_modifiers: Vec<FreeModifierSyntax>,
    argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.first_argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BeLinkSyntax {
    be: WordWithModifiers,
    free_modifiers: Vec<FreeModifierSyntax>,
    fa: Option<WordWithModifiers>,
    fa_free_modifiers: Vec<FreeModifierSyntax>,
    first_argument: Option<ArgumentSyntax>,
    bei_links: Vec<BeiLinkSyntax>,
    beho: Option<WordWithModifiers>,
    beho_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum ConnectiveKind {
    Afterthought,
    Relation,
    PredicateTail,
    Forethought,
    NonLogical,
    Interval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum ArgumentTailElementSyntax {
    Argument(Box<ArgumentSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Quantifier(QuantifierSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum QuantifierSyntax {
    Number {
        number: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Vei {
        vei: WordWithModifiers,
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WordWithModifiers>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum MathExpressionSyntax {
    Number(QuantifierSyntax),
    Letter {
        letter: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
    },
    Vei {
        vei: WordWithModifiers,
        inner_expression: Box<MathExpressionSyntax>,
        veho: Option<WordWithModifiers>,
    },
    Gek {
        gek: ConnectiveSyntax,
        left_expression: Box<MathExpressionSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Forethought {
        peho: Option<WordWithModifiers>,
        operator: MathOperatorSyntax,
        operands: Vec<MathExpressionSyntax>,
        kuhe: Option<WordWithModifiers>,
    },
    ReversePolish {
        fuha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        operands: Vec<MathExpressionSyntax>,
        operators: Vec<MathOperatorSyntax>,
    },
    Nihe {
        nihe: WordWithModifiers,
        relation: RelationSyntax,
        tehu: Option<WordWithModifiers>,
    },
    Mohe {
        mohe: WordWithModifiers,
        argument: Box<ArgumentSyntax>,
        tehu: Option<WordWithModifiers>,
    },
    Johi {
        johi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        expressions: Vec<MathExpressionSyntax>,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        markers: Vec<WordWithModifiers>,
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WordWithModifiers>,
    },
    Connected {
        left_expression: Box<MathExpressionSyntax>,
        connective: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Binary {
        operator: MathOperatorSyntax,
        left_expression: Box<MathExpressionSyntax>,
        right_expression: Box<MathExpressionSyntax>,
    },
    Bihe {
        left_expression: Box<MathExpressionSyntax>,
        bihe: WordWithModifiers,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum MathOperatorSyntax {
    Vuhu {
        vuhu: WordWithModifiers,
    },
    Maho {
        maho: WordWithModifiers,
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WordWithModifiers>,
    },
    Se {
        se: WordWithModifiers,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WordWithModifiers,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WordWithModifiers,
        relation: RelationSyntax,
        tehu: Option<WordWithModifiers>,
    },
    Connected {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MathOperatorSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum RelationSyntax {
    Connected {
        connective: ConnectiveSyntax,
        leading_relation: Box<RelationSyntax>,
        trailing_relation: Box<RelationSyntax>,
    },
    Co {
        leading_relation: Box<RelationSyntax>,
        co: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_relation: Box<RelationSyntax>,
    },
    Bo {
        leading_relation: Box<RelationSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_relation: Box<RelationSyntax>,
    },
    Na {
        na: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_relation: Box<RelationSyntax>,
    },
    Base {
        word: WordWithModifiers,
    },
    Se {
        se: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        relation: Box<RelationSyntax>,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    TenseModal {
        tense_modal: TenseModalSyntax,
        inner_relation: Box<RelationSyntax>,
    },
    Guha {
        guhek: ConnectiveSyntax,
        leading_predicate: Box<BasicPredicate>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<BasicPredicate>,
    },
    Abstraction {
        abstraction: AbstractionSyntax,
    },
    Compound {
        units: Vec<RelationUnitSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct TimeTenseSyntax {
    direction: Vec<WordWithModifiers>,
    distance: Option<WordWithModifiers>,
    interval: Option<WordWithModifiers>,
    nai: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SpaceTenseSyntax {
    direction: Vec<WordWithModifiers>,
    distance: Vec<WordWithModifiers>,
    interval: Vec<WordWithModifiers>,
    dimensions: Vec<WordWithModifiers>,
    mohi: Option<WordWithModifiers>,
    fehe: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct IntervalTenseSyntax {
    number: Vec<WordWithModifiers>,
    roi_or_tahe: WordWithModifiers,
    nai: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum TenseModalSyntax {
    Composite {
        leaves: Vec<WordWithModifiers>,
        time: Option<TimeTenseSyntax>,
        space: Option<SpaceTenseSyntax>,
        nahe: Option<WordWithModifiers>,
        interval: Option<IntervalTenseSyntax>,
        zaho: Vec<WordWithModifiers>,
        caha: Option<WordWithModifiers>,
        ki: Option<WordWithModifiers>,
        cuhe: Option<WordWithModifiers>,
        connectives: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Pu {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    PuDistance {
        pu: WordWithModifiers,
        distance: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    TimeInterval {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    PuCaha {
        pu: WordWithModifiers,
        caha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceDistance {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceDirection {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceMovement {
        mohi: WordWithModifiers,
        direction: WordWithModifiers,
        distance: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Simple {
        nahe: Option<WordWithModifiers>,
        se: Option<WordWithModifiers>,
        bai: WordWithModifiers,
        nai: Option<WordWithModifiers>,
        ki: Option<WordWithModifiers>,
        connectives: Vec<WordWithModifiers>,
        extra_leaves: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ki {
        ki: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Fiho {
        fiho: WordWithModifiers,
        relation: Box<RelationSyntax>,
        fehu: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Caha {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zaho {
        words: Vec<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Interval {
        number: Vec<WordWithModifiers>,
        roi_or_tahe: WordWithModifiers,
        nai: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct AbstractionSyntax {
    nu: WordWithModifiers,
    nai: Option<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
    additional_nu: Vec<AdditionalNuSyntax>,
    subsentence: Box<SubsentenceSyntax>,
    kei: Option<WordWithModifiers>,
    kei_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct AdditionalNuSyntax {
    connective: ConnectiveSyntax,
    nu: WordWithModifiers,
    nai: Option<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum RelationUnitSyntax {
    Word {
        word: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Goha {
        goha: WordWithModifiers,
        raho: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Se {
        se: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        relation: RelationSyntax,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Nahe {
        nahe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Bo {
        leading_unit: Box<RelationUnitSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Connected {
        leading_unit: Box<RelationUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    SelbriRelativeClause {
        base: Box<RelationUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativeClauseSyntax>,
    },
    Wrapped {
        relation: RelationSyntax,
    },
    Jai {
        jai: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        tense_modal: Option<TenseModalSyntax>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
        base: Box<RelationUnitSyntax>,
        be: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        fa: Option<WordWithModifiers>,
        fa_free_modifiers: Vec<FreeModifierSyntax>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WordWithModifiers>,
        beho_free_modifiers: Vec<FreeModifierSyntax>,
    },
    PreposedBe {
        be: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        fa: Option<WordWithModifiers>,
        fa_free_modifiers: Vec<FreeModifierSyntax>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WordWithModifiers>,
        beho_free_modifiers: Vec<FreeModifierSyntax>,
        base: Box<RelationUnitSyntax>,
    },
    Abstraction {
        abstraction: AbstractionSyntax,
    },
    Me {
        me: WordWithModifiers,
        me_free_modifiers: Vec<FreeModifierSyntax>,
        argument: ArgumentSyntax,
        mehu: Option<WordWithModifiers>,
        mehu_free_modifiers: Vec<FreeModifierSyntax>,
        moi_marker: Option<WordWithModifiers>,
        moi_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Mehoi {
        mehoi: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Gohoi {
        gohoi: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Muhoi {
        muhoi: WordWithModifiers,
        opening_delimiter: WordWithModifiers,
        closing_delimiter: WordWithModifiers,
        quoted_text: String,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Luhei {
        luhei: WordWithModifiers,
        luhei_free_modifiers: Vec<FreeModifierSyntax>,
        text: TextSyntax,
        liau: Option<WordWithModifiers>,
        liau_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Moi {
        number: Vec<WordWithModifiers>,
        moi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Nuha {
        nuha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        math_operator: MathOperatorSyntax,
    },
    Xohi {
        xohi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        tag: TenseModalSyntax,
    },
    Cei {
        base: Box<RelationUnitSyntax>,
        assignments: Vec<CeiAssignmentSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct CeiAssignmentSyntax {
    cei: WordWithModifiers,
    free_modifiers: Vec<FreeModifierSyntax>,
    relation_unit: RelationUnitSyntax,
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_syntax_tree(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_source(words, None, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_syntax_tree_with_source(
    words: &[WordWithModifiers],
    source: Option<&str>,
    _options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    let text = parser::parse_statement(words, source)?;
    Ok(new!(SyntaxParse {
        parse_tree: render::lojban_text_tree(text),
        warnings: Vec::new(),
    }))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_text(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    let text = parser::parse_statement(words, None)?;
    let paragraphs = text
        .paragraphs
        .into_iter()
        .map(public_paragraph)
        .collect::<Vec<_>>();
    let _ = options;
    Ok(new!(LojbanText {
        leading_nai: text.leading_nai,
        leading_cmevla: text.leading_cmevla,
        leading_indicators: text.leading_indicators,
        leading_free_modifiers: text
            .leading_free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        leading_connective: text.leading_connective.map(public_connective),
        paragraphs,
    }))
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph(paragraph: ParagraphSyntax) -> Paragraph {
    new!(Paragraph {
        i: paragraph.i,
        niho: paragraph.niho,
        free_modifiers: paragraph
            .free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        statements: paragraph
            .statements
            .into_iter()
            .map(public_paragraph_statement)
            .collect(),
    })
}

#[requires(true)]
#[ensures(true)]
fn public_paragraph_statement(statement: ParagraphStatementSyntax) -> ParagraphStatement {
    new!(ParagraphStatement {
        i: statement.i,
        connective: statement.connective.map(public_connective),
        free_modifiers: statement
            .free_modifiers
            .into_iter()
            .map(public_free_modifier)
            .collect(),
        statement: statement.statement.map(public_statement),
    })
}

#[requires(true)]
#[ensures(true)]
fn public_statement(statement: StatementSyntax) -> Statement {
    Statement::fragment(Fragment::other(statement.words()))
}

#[requires(true)]
#[ensures(true)]
fn public_free_modifier(free_modifier: FreeModifierSyntax) -> FreeModifier {
    FreeModifier::words(free_modifier.words())
}

#[requires(true)]
#[ensures(true)]
fn public_connective(connective: ConnectiveSyntax) -> Connective {
    Connective::words(connective.words())
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                tuhe_free_modifiers,
                text,
                tuhu,
                tuhu_free_modifiers,
            } => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(tuhe);
                for free_modifier in tuhe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(text.words());
                words.extend(tuhu);
                for free_modifier in tuhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                zohu_free_modifiers,
                inner_statement,
            } => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.push(zohu);
                for free_modifier in zohu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_statement.words());
                words
            }
            StatementSyntax::Predicate(predicate) => predicate.words(),
            StatementSyntax::Connected {
                i,
                connective,
                leading_statement,
                trailing_statement,
            } => {
                let mut words = leading_statement.words();
                words.push(i);
                words.extend(connective.words());
                words.extend(trailing_statement.words());
                words
            }
            StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement,
                trailing_statement,
            } => {
                let mut words = leading_statement.words();
                words.extend(connective.words());
                words.push(i);
                words.extend(trailing_statement.words());
                words
            }
            StatementSyntax::Iau {
                inner_statement,
                iau,
                iau_free_modifiers,
                reset_terms,
            } => {
                let mut words = inner_statement.words();
                words.push(iau);
                for free_modifier in iau_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for term in reset_terms {
                    words.extend(term.words());
                }
                words
            }
            StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            } => {
                let mut words = leading_statement.words();
                words.extend(continuation.words());
                words
            }
            StatementSyntax::Fragment(fragment) => fragment.words(),
        }
    }
}

impl PredicateStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        match self.marker {
            PredicateStatementContinuationMarkerSyntax::Bo { bo, free_modifiers } => {
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(self.trailing_subsentence.words());
            }
            PredicateStatementContinuationMarkerSyntax::Ke {
                ke,
                ke_free_modifiers,
                kehe,
                kehe_free_modifiers,
            } => {
                let mut words = vec![ke];
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(self.trailing_subsentence.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
            }
        }
        words
    }
}

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.leading_nai;
        words.extend(self.leading_cmevla);
        words.extend(self.leading_indicators);
        for free_modifier in self.leading_free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(leading_connective) = self.leading_connective {
            words.extend(leading_connective.words());
        }
        for paragraph in self.paragraphs {
            words.extend(paragraph.words());
        }
        words
    }
}

impl ParagraphSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.i.into_iter().collect::<Vec<_>>();
        words.extend(self.niho);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        for paragraph_statement in self.statements {
            words.extend(paragraph_statement.words());
        }
        words
    }
}

impl ParagraphStatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.i.into_iter().collect::<Vec<_>>();
        if let Some(connective) = self.connective {
            words.extend(connective.words());
        }
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(statement) = self.statement {
            words.extend(statement.words());
        }
        words
    }
}

impl FreeModifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            FreeModifierSyntax::Sei {
                sei,
                leading_free_modifiers,
                terms,
                cu,
                cu_free_modifiers,
                relation,
                sehu,
                sehu_free_modifiers,
            } => {
                let mut words = vec![sei];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(cu);
                for free_modifier in cu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(relation.words());
                words.extend(sehu);
                for free_modifier in sehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FreeModifierSyntax::To {
                to,
                free_modifiers,
                text,
                toi,
                toi_free_modifiers,
            } => {
                let mut words = vec![to];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(text.words());
                words.extend(toi);
                for free_modifier in toi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FreeModifierSyntax::Xi {
                xi,
                free_modifiers,
                expression,
            } => {
                let mut words = vec![xi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(expression.words());
                words
            }
            FreeModifierSyntax::Mai {
                number,
                mai,
                free_modifiers,
            } => {
                let mut words = number;
                words.push(mai);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FreeModifierSyntax::Soi {
                soi,
                free_modifiers,
                leading_argument,
                trailing_argument,
                sehu,
                sehu_free_modifiers,
            } => {
                let mut words = vec![soi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(leading_argument.words());
                if let Some(argument) = trailing_argument {
                    words.extend(argument.words());
                }
                words.extend(sehu);
                for free_modifier in sehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FreeModifierSyntax::Vocative {
                vocative_markers,
                free_modifiers,
                argument,
                dohu,
                dohu_free_modifiers,
            } => {
                let mut words = vocative_markers;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(argument) = argument {
                    words.extend(argument.words());
                }
                words.extend(dohu);
                for free_modifier in dohu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
        }
    }
}

impl BasicPredicate {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = Vec::new();
        for term in self.leading_terms {
            words.extend(term.words());
        }
        words.extend(self.cu);
        for free_modifier in self.cu_free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(gek_sentence) = self.gek_sentence {
            words.extend(gek_sentence.words());
        } else {
            words.extend(self.relation.words());
            for term in self.tail_terms {
                words.extend(term.words());
            }
            words.extend(self.vau);
            for free_modifier in self.tail_free_modifiers {
                words.extend(free_modifier.words());
            }
        }
        if let Some(bo_continuation) = self.bo_continuation {
            words.extend(bo_continuation.words());
        }
        for continuation in self.continuations {
            words.extend(continuation.words());
        }
        if let Some(ke_continuation) = self.ke_continuation {
            words.extend(ke_continuation.words());
        }
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTailBoContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        words.push(self.bo);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.cu);
        for free_modifier in self.cu_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.predicate_tail.words());
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        words
    }
}

impl PredicateTailKeContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        words.push(self.ke);
        for free_modifier in self.ke_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.predicate_tail.words());
        words.extend(self.kehe);
        for free_modifier in self.kehe_free_modifiers {
            words.extend(free_modifier.words());
        }
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        words
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            GekSentenceSyntax::Pair {
                gek,
                first,
                gik,
                second,
                tail_terms,
                vau,
                free_modifiers,
            } => {
                let mut words = gek.words();
                words.extend(first.words());
                words.extend(gik.words());
                words.extend(second.words());
                for term in tail_terms {
                    words.extend(term.words());
                }
                words.extend(vau);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            GekSentenceSyntax::Ke {
                tense_modal,
                ke,
                ke_free_modifiers,
                inner,
                kehe,
                kehe_free_modifiers,
            } => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(ke);
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            GekSentenceSyntax::Na {
                na,
                free_modifiers,
                inner,
            } => {
                let mut words = vec![na];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner.words());
                words
            }
        }
    }
}

impl SubsentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            SubsentenceSyntax::Plain(predicate) => predicate.words(),
            SubsentenceSyntax::Prenex {
                prenex_terms,
                zohu,
                zohu_free_modifiers,
                inner_subsentence,
            } => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.push(zohu);
                for free_modifier in zohu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_subsentence.words());
                words
            }
        }
    }
}

impl PredicateTailContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        words.extend(self.cu);
        for free_modifier in self.cu_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.relation.words());
        for term in self.terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(bo_continuation) = self.bo_continuation {
            words.extend(bo_continuation.words());
        }
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.tail_vau);
        words
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            FragmentSyntax::Argument { argument } => argument.words(),
            FragmentSyntax::Ek {
                connective,
                free_modifiers,
            }
            | FragmentSyntax::Gihek {
                connective,
                free_modifiers,
            } => {
                let mut words = connective.words();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FragmentSyntax::Other {
                words,
                free_modifiers,
            } => {
                let mut all_words = words;
                for free_modifier in free_modifiers {
                    all_words.extend(free_modifier.words());
                }
                all_words
            }
            FragmentSyntax::Vocative {
                vocative_markers,
                free_modifiers,
                vocative_argument,
                dohu,
                dohu_free_modifiers,
            } => {
                let mut words = vocative_markers;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(vocative_argument) = vocative_argument {
                    words.extend(vocative_argument.words());
                }
                words.extend(dohu);
                for free_modifier in dohu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FragmentSyntax::Ijek { i, connective } => {
                let mut words = vec![i];
                words.extend(connective.words());
                words
            }
            FragmentSyntax::Prenex {
                terms,
                zohu,
                zohu_free_modifiers,
            } => {
                let mut words = terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.push(zohu);
                for free_modifier in zohu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FragmentSyntax::BeLink {
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
            } => {
                let mut words = vec![be];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(fa);
                for free_modifier in fa_free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                words.extend(beho);
                for free_modifier in beho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FragmentSyntax::BeiLink { bei_only_links } => bei_only_links
                .into_iter()
                .flat_map(BeiLinkSyntax::words)
                .collect(),
            FragmentSyntax::RelativeClause { relative_clauses } => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            FragmentSyntax::MathExpression { math_expression } => math_expression.words(),
            FragmentSyntax::Term {
                terms,
                vau,
                vau_free_modifiers,
            } => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(vau);
                for free_modifier in vau_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            FragmentSyntax::Relation { relation } => relation.words(),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            TermSyntax::NuhiTermset {
                nuhi,
                nuhi_free_modifiers,
                termset,
                nuhu,
                nuhu_free_modifiers,
            } => {
                let mut words = vec![nuhi];
                for free_modifier in nuhi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for term in termset {
                    words.extend(term.words());
                }
                words.extend(nuhu);
                for free_modifier in nuhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::GekNuhiTermset {
                m_nuhi,
                nuhi_free_modifiers,
                gek,
                terms,
                nuhu,
                nuhu_free_modifiers,
                gik,
                gik_terms,
                gik_nuhu,
                gik_nuhu_free_modifiers,
            } => {
                let mut words = m_nuhi.into_iter().collect::<Vec<_>>();
                for free_modifier in nuhi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(gek.words());
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(nuhu);
                for free_modifier in nuhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(gik.words());
                for term in gik_terms {
                    words.extend(term.words());
                }
                words.extend(gik_nuhu);
                for free_modifier in gik_nuhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::Cehe {
                leading_terms,
                cehe,
                free_modifiers,
                trailing_terms,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.push(cehe);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::Pehe {
                leading_terms,
                pehe,
                free_modifiers,
                connective,
                trailing_terms,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.push(pehe);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::Argument(argument) => argument.words(),
            TermSyntax::Fa {
                fa,
                free_modifiers,
                argument,
                ku,
                ku_free_modifiers,
            } => {
                let mut words = vec![fa];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(argument.words());
                words.extend(ku);
                for free_modifier in ku_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::NaKu {
                na,
                na_ku,
                free_modifiers,
            } => {
                let mut words = vec![na, na_ku];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::BareNa { na, free_modifiers } => {
                let mut words = vec![na];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::NoihaAdverbial {
                noiha,
                leading_free_modifiers,
                tail_elements,
                relation,
                relative_clauses,
                fehu,
                trailing_free_modifiers,
            } => {
                let mut words = vec![noiha];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(fehu);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::PoihaBrigahi {
                poiha,
                leading_free_modifiers,
                tail_elements,
                relation,
                relative_clauses,
                brigahi_ku,
                trailing_free_modifiers,
            } => {
                let mut words = vec![poiha];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.push(brigahi_ku);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::FihoiAdverbial {
                fihoi,
                leading_free_modifiers,
                subsentence,
                fihau,
                trailing_free_modifiers,
            } => {
                let mut words = vec![fihoi];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(subsentence.words());
                words.extend(fihau);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::SoiAdverbial {
                soi,
                leading_free_modifiers,
                subsentence,
                sehu,
                trailing_free_modifiers,
            } => {
                let mut words = vec![soi];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(subsentence.words());
                words.extend(sehu);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::Tagged {
                tense_modal,
                free_modifiers,
                argument,
            } => {
                let mut words = tense_modal
                    .into_iter()
                    .flat_map(TenseModalSyntax::words)
                    .collect::<Vec<_>>();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(argument.words());
                words
            }
            TermSyntax::Connected {
                leading_terms,
                connective,
                trailing_terms,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::BoConnected {
                leading_terms,
                bo_connective,
                tense_modal,
                bo,
                free_modifiers,
                trailing_term,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(trailing_term.words());
                words
            }
        }
    }
}

impl MathExpressionSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            MathExpressionSyntax::Number(quantifier) => quantifier.words(),
            MathExpressionSyntax::Letter { letter, boi } => {
                [letter, boi.into_iter().collect()].concat()
            }
            MathExpressionSyntax::Vei {
                vei,
                inner_expression,
                veho,
            } => {
                let mut words = vec![vei];
                words.extend(inner_expression.words());
                words.extend(veho);
                words
            }
            MathExpressionSyntax::Gek {
                gek,
                left_expression,
                gik,
                right_expression,
            } => {
                let mut words = gek.words();
                words.extend(left_expression.words());
                words.extend(gik.words());
                words.extend(right_expression.words());
                words
            }
            MathExpressionSyntax::Forethought {
                peho,
                operator,
                operands,
                kuhe,
            } => {
                let mut words = peho.into_iter().collect::<Vec<_>>();
                words.extend(operator.words());
                for operand in operands {
                    words.extend(operand.words());
                }
                words.extend(kuhe);
                words
            }
            MathExpressionSyntax::ReversePolish {
                fuha,
                free_modifiers,
                operands,
                operators,
            } => {
                let mut words = vec![fuha];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                for operand in operands {
                    words.extend(operand.words());
                }
                for operator in operators {
                    words.extend(operator.words());
                }
                words
            }
            MathExpressionSyntax::Nihe {
                nihe,
                relation,
                tehu,
            } => {
                let mut words = vec![nihe];
                words.extend(relation.words());
                words.extend(tehu);
                words
            }
            MathExpressionSyntax::Mohe {
                mohe,
                argument,
                tehu,
            } => {
                let mut words = vec![mohe];
                words.extend(argument.words());
                words.extend(tehu);
                words
            }
            MathExpressionSyntax::Johi {
                johi,
                free_modifiers,
                expressions,
                tehu,
                tehu_free_modifiers,
            } => {
                let mut words = vec![johi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                for expression in expressions {
                    words.extend(expression.words());
                }
                words.extend(tehu);
                for free_modifier in tehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathExpressionSyntax::Lahe {
                markers,
                inner_expression,
                luhu,
            } => {
                let mut words = markers;
                words.extend(inner_expression.words());
                words.extend(luhu);
                words
            }
            MathExpressionSyntax::Connected {
                left_expression,
                connective,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.extend(connective.words());
                words.extend(right_expression.words());
                words
            }
            MathExpressionSyntax::Binary {
                operator,
                left_expression,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
            MathExpressionSyntax::Bihe {
                left_expression,
                bihe,
                operator,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.push(bihe);
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            ArgumentSyntax::Quote { quote } => quote.words(),
            ArgumentSyntax::MathExpression {
                li,
                li_free_modifiers,
                expression,
                loho,
                loho_free_modifiers,
            } => {
                let mut words = vec![li];
                for free_modifier in li_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(expression.words());
                words.extend(loho);
                for free_modifier in loho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Letter {
                letter,
                boi,
                boi_free_modifiers,
            } => {
                let mut words = letter;
                words.extend(boi);
                for free_modifier in boi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            } => {
                let mut words = quantifier.words();
                words.extend(inner_argument.words());
                words
            }
            ArgumentSyntax::RelativeClause {
                base_argument,
                vuho,
                vuho_free_modifiers,
                relative_clauses,
            } => {
                let mut words = base_argument.words();
                words.extend(vuho);
                for free_modifier in vuho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words
            }
            ArgumentSyntax::Vuho {
                base_argument,
                vuho_marker,
                vuho_free_modifiers,
                relative_clauses,
                connected_argument,
            } => {
                let mut words = base_argument.words();
                words.push(vuho_marker);
                for free_modifier in vuho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(connected_argument) = connected_argument {
                    words.extend(connected_argument.connective.words());
                    words.extend(connected_argument.argument.words());
                }
                words
            }
            ArgumentSyntax::BridiDescription {
                lohoi,
                lohoi_free_modifiers,
                subsentence,
                kuhau,
                kuhau_free_modifiers,
            } => {
                let mut words = vec![lohoi];
                for free_modifier in lohoi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(subsentence.words());
                words.extend(kuhau);
                for free_modifier in kuhau_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::NaKu {
                na,
                ku,
                free_modifiers,
            } => {
                let mut words = vec![na, ku];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Tagged {
                tag_words,
                free_modifiers,
                inner_argument,
                ..
            } => {
                let mut words = tag_words;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_argument.words());
                words
            }
            ArgumentSyntax::NaheBo {
                nahe,
                bo,
                free_modifiers,
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = vec![nahe, bo];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_argument.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Nahe {
                nahe,
                free_modifiers,
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = vec![nahe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_argument.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::TermWrapped {
                wrapper,
                wrapper_bo,
                free_modifiers,
                inner_term,
                luhu,
                luhu_free_modifiers,
                ..
            } => {
                let mut words = vec![wrapper];
                words.extend(wrapper_bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_term.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Koha {
                koha,
                free_modifiers,
            } => {
                let mut words = vec![koha];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Zohe {
                tag_words,
                maybe_ku,
                free_modifiers,
            } => {
                let mut words = [tag_words, maybe_ku.into_iter().collect()].concat();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Lahe {
                lahe,
                free_modifiers,
                relative_clauses,
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = vec![lahe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(inner_argument.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Connected {
                leading_argument,
                connective,
                trailing_argument,
            } => {
                let mut words = leading_argument.words();
                words.extend(connective.words());
                words.extend(trailing_argument.words());
                words
            }
            ArgumentSyntax::Ke {
                ke,
                ke_free_modifiers,
                inner_argument,
                kehe,
                kehe_free_modifiers,
            } => {
                let mut words = vec![ke];
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_argument.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Bo {
                leading_argument,
                bo_connective,
                bo_tense_modal,
                bo,
                free_modifiers,
                trailing_argument,
            } => {
                let mut words = leading_argument.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(trailing_argument.words());
                words
            }
            ArgumentSyntax::Gek {
                gek,
                leading_argument,
                gik,
                trailing_argument,
            } => {
                let mut words = gek.words();
                words.extend(leading_argument.words());
                words.extend(gik.words());
                words.extend(trailing_argument.words());
                words
            }
            ArgumentSyntax::Descriptor { descriptor } => descriptor.words(),
            ArgumentSyntax::Name {
                la,
                la_free_modifiers,
                names,
                name_free_modifiers,
            } => {
                let mut words = vec![la];
                for free_modifier in la_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(names);
                for free_modifier in name_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Cmevla {
                cmevla,
                free_modifiers,
            } => {
                let mut words = cmevla;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            } => {
                let mut words = leading_relative_clauses
                    .into_iter()
                    .flat_map(RelativeClauseSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(relation.words());
                words.extend(
                    trailing_relative_clauses
                        .into_iter()
                        .flat_map(RelativeClauseSyntax::words),
                );
                words
            }
        }
    }
}

impl GoiRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.goi];
        for free_modifier in self.leading_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.argument.words());
        words.extend(self.gehu);
        for free_modifier in self.trailing_free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl SelbriRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.nohoi];
        for free_modifier in self.leading_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.relation.words());
        words.extend(self.kuhoi);
        for free_modifier in self.trailing_free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl RelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            RelativeClauseSyntax::Goi(relative_clause) => relative_clause.words(),
            RelativeClauseSyntax::Noi {
                noi,
                leading_free_modifiers,
                subsentence,
                kuho,
                trailing_free_modifiers,
            } => {
                let mut words = vec![noi];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(subsentence.words());
                words.extend(kuho);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelativeClauseSyntax::Poi {
                poi,
                leading_free_modifiers,
                subsentence,
                kuho,
                trailing_free_modifiers,
            } => {
                let mut words = vec![poi];
                for free_modifier in leading_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(subsentence.words());
                words.extend(kuho);
                for free_modifier in trailing_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelativeClauseSyntax::Zihe {
                zihe,
                free_modifiers,
                inner,
            } => {
                let mut words = vec![zihe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner.words());
                words
            }
            RelativeClauseSyntax::Connected { connective, inner } => {
                let mut words = connective.words();
                words.extend(inner.words());
                words
            }
        }
    }
}

impl QuoteSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            QuoteSyntax::Lu {
                lu,
                free_modifiers,
                text,
                lihu,
                lihu_free_modifiers,
            } => {
                let mut words = vec![lu];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(text.words());
                words.extend(lihu);
                for free_modifier in lihu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuoteSyntax::Zo {
                zo,
                word,
                free_modifiers,
            } => {
                let mut words = vec![zo, word];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuoteSyntax::ZohOi {
                zohoi,
                free_modifiers,
                ..
            } => {
                let mut words = vec![zohoi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuoteSyntax::Zoi {
                zoi,
                opening_delimiter,
                closing_delimiter,
                free_modifiers,
                ..
            } => {
                let mut words = vec![zoi, opening_delimiter, closing_delimiter];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuoteSyntax::Laho {
                laho,
                opening_delimiter,
                closing_delimiter,
                free_modifiers,
                ..
            } => {
                let mut words = vec![laho, opening_delimiter, closing_delimiter];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuoteSyntax::Lohu {
                lohu,
                quoted_words,
                lehu,
                lehu_free_modifiers,
            } => {
                let mut words = [vec![lohu], quoted_words, vec![lehu]].concat();
                for free_modifier in lehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self
            .outer_quantifier
            .into_iter()
            .flat_map(QuantifierSyntax::words)
            .collect::<Vec<_>>();
        words.extend(self.descriptor);
        for free_modifier in self.descriptor_free_modifiers {
            words.extend(free_modifier.words());
        }
        for element in self.tail_elements {
            words.extend(element.words());
        }
        if let Some(relation) = self.relation {
            words.extend(relation.words());
        }
        for relative_clause in self.relative_clauses {
            words.extend(relative_clause.words());
        }
        words.extend(self.ku);
        for free_modifier in self.ku_free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl ConnectiveSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        [
            self.se.into_iter().collect(),
            self.nahe.into_iter().collect(),
            self.na.into_iter().collect(),
            self.cmavo,
            self.nai.into_iter().collect(),
            self.free_modifiers
                .into_iter()
                .flat_map(FreeModifierSyntax::words)
                .collect(),
        ]
        .concat()
    }
}

impl BeiLinkSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.bei];
        for free_modifier in self.bei_free_modifiers {
            words.extend(free_modifier.words());
        }
        words.extend(self.fa);
        for free_modifier in self.fa_free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(argument) = self.argument {
            words.extend(argument.words());
        }
        words
    }
}

impl ArgumentTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            ArgumentTailElementSyntax::Argument(argument) => argument.words(),
            ArgumentTailElementSyntax::RelativeClauses(relative_clauses) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            ArgumentTailElementSyntax::Quantifier(quantifier) => quantifier.words(),
        }
    }
}

impl QuantifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            QuantifierSyntax::Number {
                number,
                boi,
                free_modifiers,
            } => {
                let mut words = [number, boi.into_iter().collect()].concat();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            QuantifierSyntax::Vei {
                vei,
                math_expression,
                veho,
            } => {
                let mut words = vec![vei];
                words.extend(math_expression.words());
                words.extend(veho);
                words
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            MathOperatorSyntax::Vuhu { vuhu } => vec![vuhu],
            MathOperatorSyntax::Maho {
                maho,
                math_expression,
                tehu,
            } => {
                let mut words = vec![maho];
                words.extend(math_expression.words());
                words.extend(tehu);
                words
            }
            MathOperatorSyntax::Se { se, inner_operator } => {
                let mut words = vec![se];
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahe {
                nahe,
                inner_operator,
            } => {
                let mut words = vec![nahe];
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahu {
                nahu,
                relation,
                tehu,
            } => {
                let mut words = vec![nahu];
                words.extend(relation.words());
                words.extend(tehu);
                words
            }
            MathOperatorSyntax::Connected {
                left_operator,
                connective,
                right_operator,
            } => {
                let mut words = left_operator.words();
                words.extend(connective.words());
                words.extend(right_operator.words());
                words
            }
        }
    }
}

impl RelationSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            RelationSyntax::Connected {
                connective,
                leading_relation,
                trailing_relation,
            } => {
                let mut words = leading_relation.words();
                words.extend(connective.words());
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Co {
                leading_relation,
                co,
                free_modifiers,
                trailing_relation,
            } => {
                let mut words = leading_relation.words();
                words.push(co);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Bo {
                leading_relation,
                bo_connective,
                bo_tense_modal,
                bo,
                free_modifiers,
                trailing_relation,
            } => {
                let mut words = leading_relation.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Na {
                na,
                free_modifiers,
                inner_relation,
            } => {
                let mut words = vec![na];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Base { word } => vec![word],
            RelationSyntax::Se {
                se,
                free_modifiers,
                inner_relation,
            } => {
                let mut words = vec![se];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Ke {
                ke,
                ke_free_modifiers,
                relation,
                kehe,
                kehe_free_modifiers,
                ..
            } => {
                let mut words = vec![ke];
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(relation.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            } => {
                let mut words = tense_modal.words();
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Guha {
                guhek,
                leading_predicate,
                gik,
                trailing_predicate,
            } => {
                let mut words = guhek.words();
                words.extend(leading_predicate.words());
                words.extend(gik.words());
                words.extend(trailing_predicate.words());
                words
            }
            RelationSyntax::Abstraction { abstraction } => abstraction.words(),
            RelationSyntax::Compound { units } => units
                .into_iter()
                .flat_map(RelationUnitSyntax::words)
                .collect(),
        }
    }
}

impl RelationUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            RelationUnitSyntax::Word {
                word,
                free_modifiers,
            } => {
                let mut words = vec![word];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Goha {
                goha,
                raho,
                free_modifiers,
            } => {
                let mut words = vec![goha];
                words.extend(raho);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Se {
                se,
                free_modifiers,
                inner_unit,
            } => {
                let mut words = vec![se];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Ke {
                ke,
                ke_free_modifiers,
                relation,
                kehe,
                kehe_free_modifiers,
                ..
            } => {
                let mut words = vec![ke];
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(relation.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Nahe {
                nahe,
                free_modifiers,
                inner_unit,
            } => {
                let mut words = vec![nahe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                free_modifiers,
                trailing_unit,
            } => {
                let mut words = leading_unit.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(trailing_unit.words());
                words
            }
            RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            } => {
                let mut words = leading_unit.words();
                words.extend(connective.words());
                words.extend(trailing_unit.words());
                words
            }
            RelationUnitSyntax::SelbriRelativeClause {
                base,
                selbri_relative_clauses,
            } => {
                let mut words = base.words();
                for selbri_relative_clause in selbri_relative_clauses {
                    words.extend(selbri_relative_clause.words());
                }
                words
            }
            RelationUnitSyntax::Wrapped { relation } => relation.words(),
            RelationUnitSyntax::Jai {
                jai,
                free_modifiers,
                tense_modal,
                inner_unit,
            } => {
                let mut words = vec![jai];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Be {
                base,
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
            } => {
                let mut words = base.words();
                words.push(be);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(fa);
                for free_modifier in fa_free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                words.extend(beho);
                for free_modifier in beho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::PreposedBe {
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
                base,
            } => {
                let mut words = vec![be];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(fa);
                for free_modifier in fa_free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                words.extend(beho);
                for free_modifier in beho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(base.words());
                words
            }
            RelationUnitSyntax::Abstraction { abstraction } => abstraction.words(),
            RelationUnitSyntax::Me {
                me,
                me_free_modifiers,
                argument,
                mehu,
                mehu_free_modifiers,
                moi_marker,
                moi_free_modifiers,
            } => {
                let mut words = vec![me];
                for free_modifier in me_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(argument.words());
                words.extend(mehu);
                for free_modifier in mehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(moi_marker);
                for free_modifier in moi_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Mehoi {
                mehoi,
                free_modifiers,
                ..
            } => {
                let mut words = vec![mehoi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Gohoi {
                gohoi,
                free_modifiers,
                ..
            } => {
                let mut words = vec![gohoi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Muhoi {
                muhoi,
                opening_delimiter,
                closing_delimiter,
                free_modifiers,
                ..
            } => {
                let mut words = vec![muhoi, opening_delimiter, closing_delimiter];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Luhei {
                luhei,
                luhei_free_modifiers,
                text,
                liau,
                liau_free_modifiers,
            } => {
                let mut words = vec![luhei];
                for free_modifier in luhei_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(text.words());
                words.extend(liau);
                for free_modifier in liau_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Moi {
                number,
                moi,
                free_modifiers,
            } => {
                let mut words = number;
                words.push(moi);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            RelationUnitSyntax::Nuha {
                nuha,
                free_modifiers,
                math_operator,
            } => {
                let mut words = vec![nuha];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(math_operator.words());
                words
            }
            RelationUnitSyntax::Xohi {
                xohi,
                free_modifiers,
                tag,
            } => {
                let mut words = vec![xohi];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(tag.words());
                words
            }
            RelationUnitSyntax::Cei { base, assignments } => {
                let mut words = base.words();
                for assignment in assignments {
                    words.push(assignment.cei);
                    for free_modifier in assignment.free_modifiers {
                        words.extend(free_modifier.words());
                    }
                    words.extend(assignment.relation_unit.words());
                }
                words
            }
        }
    }
}

impl AbstractionSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.nu];
        words.extend(self.nai);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        for additional_nu in self.additional_nu {
            words.extend(additional_nu.words());
        }
        words.extend((*self.subsentence).words());
        words.extend(self.kei);
        for free_modifier in self.kei_free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        words.push(self.nu);
        words.extend(self.nai);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn leaf_words(self) -> Vec<WordWithModifiers> {
        match self {
            TenseModalSyntax::Composite { leaves, .. } => leaves,
            TenseModalSyntax::Pu { word, .. } | TenseModalSyntax::Caha { word, .. } => vec![word],
            TenseModalSyntax::PuDistance { pu, distance, .. } => vec![pu, distance],
            TenseModalSyntax::TimeInterval { word, .. } => vec![word],
            TenseModalSyntax::PuCaha { pu, caha, .. } => vec![pu, caha],
            TenseModalSyntax::SpaceDistance { word, .. } => vec![word],
            TenseModalSyntax::SpaceDirection { word, .. } => vec![word],
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
                ..
            } => [vec![mohi, direction], distance.into_iter().collect()].concat(),
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
                ..
            } => [
                nahe.into_iter().collect::<Vec<_>>(),
                se.into_iter().collect(),
                vec![bai],
                nai.into_iter().collect(),
                ki.into_iter().collect(),
                connectives,
                extra_leaves,
            ]
            .concat(),
            TenseModalSyntax::Ki { ki, .. } => vec![ki],
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
                ..
            } => {
                let mut words = vec![fiho];
                words.extend((*relation).words());
                words.extend(fehu);
                words
            }
            TenseModalSyntax::Zaho { words, .. } => words,
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
                ..
            } => [number, vec![roi_or_tahe], nai.into_iter().collect()].concat(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.clone().leaf_words();
        for free_modifier in self.free_modifiers() {
            words.extend(free_modifier.words());
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    fn free_modifiers(self) -> Vec<FreeModifierSyntax> {
        match self {
            TenseModalSyntax::Composite { free_modifiers, .. }
            | TenseModalSyntax::Pu { free_modifiers, .. }
            | TenseModalSyntax::PuDistance { free_modifiers, .. }
            | TenseModalSyntax::TimeInterval { free_modifiers, .. }
            | TenseModalSyntax::PuCaha { free_modifiers, .. }
            | TenseModalSyntax::SpaceDistance { free_modifiers, .. }
            | TenseModalSyntax::SpaceDirection { free_modifiers, .. }
            | TenseModalSyntax::SpaceMovement { free_modifiers, .. }
            | TenseModalSyntax::Simple { free_modifiers, .. }
            | TenseModalSyntax::Ki { free_modifiers, .. }
            | TenseModalSyntax::Fiho { free_modifiers, .. }
            | TenseModalSyntax::Caha { free_modifiers, .. }
            | TenseModalSyntax::Zaho { free_modifiers, .. }
            | TenseModalSyntax::Interval { free_modifiers, .. } => free_modifiers,
        }
    }
}

#[cfg(test)]
mod tests {
    use bityzba::requires;
    use jbotci_morphology::segment_words_with_modifiers;

    use crate::SyntaxValueData;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

        let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

        let data!(SyntaxValue::Node { node }) = parsed.parse_tree.as_data() else {
            panic!("expected node");
        };
        assert_eq!(node.constructor, "LojbanText");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_stray_cu() {
        let words = segment_words_with_modifiers("cu").expect("valid morphology");

        let error = parse_syntax_tree(&words, &ParseOptions::default()).expect_err("invalid");

        assert!(matches!(error, SyntaxError::Parse { .. }));
    }
}
