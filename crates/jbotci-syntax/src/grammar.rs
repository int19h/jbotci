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
    predicate_tail: Box<BasicPredicate>,
    kehe: Option<WordWithModifiers>,
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
    },
    Ke {
        tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WordWithModifiers>,
    },
    Na {
        na: WordWithModifiers,
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
    relation: RelationSyntax,
    terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
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
    outer_quantifier: Option<QuantifierSyntax>,
    descriptor: Option<WordWithModifiers>,
    tail_elements: Vec<ArgumentTailElementSyntax>,
    relation: Option<RelationSyntax>,
    relative_clauses: Vec<RelativeClauseSyntax>,
    ku: Option<WordWithModifiers>,
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
    additional_nu: Vec<AdditionalNuSyntax>,
    subsentence: Box<SubsentenceSyntax>,
    kei: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct AdditionalNuSyntax {
    connective: ConnectiveSyntax,
    nu: WordWithModifiers,
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
    let text = parse_statement(words, source)?;
    Ok(new!(SyntaxParse {
        parse_tree: lojban_text_tree(text),
        warnings: Vec::new(),
    }))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_text(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    let text = parse_statement(words, None)?;
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
        words.extend(self.predicate_tail.words());
        words.extend(self.kehe);
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
            } => {
                let mut words = gek.words();
                words.extend(first.words());
                words.extend(gik.words());
                words.extend(second.words());
                for term in tail_terms {
                    words.extend(term.words());
                }
                words.extend(vau);
                words
            }
            GekSentenceSyntax::Ke {
                tense_modal,
                ke,
                inner,
                kehe,
            } => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(ke);
                words.extend(inner.words());
                words.extend(kehe);
                words
            }
            GekSentenceSyntax::Na { na, inner } => {
                let mut words = vec![na];
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
        words.extend(self.relation.words());
        for term in self.terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
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
        for additional_nu in self.additional_nu {
            words.extend(additional_nu.words());
        }
        words.extend((*self.subsentence).words());
        words.extend(self.kei);
        words
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        words.push(self.nu);
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

#[requires(true)]
#[ensures(ret.clone().free_modifiers().len() >= old(free_modifiers.len()))]
fn attach_tense_modal_free_modifiers(
    tense_modal: TenseModalSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
    match tense_modal {
        TenseModalSyntax::Composite {
            leaves,
            time,
            space,
            nahe,
            interval,
            zaho,
            caha,
            ki,
            cuhe,
            connectives,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Composite {
                leaves,
                time,
                space,
                nahe,
                interval,
                zaho,
                caha,
                ki,
                cuhe,
                connectives,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Pu {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Pu {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::PuDistance {
            pu,
            distance,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::PuDistance {
                pu,
                distance,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::TimeInterval {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::TimeInterval {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::PuCaha {
            pu,
            caha,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::PuCaha {
                pu,
                caha,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceDistance {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceDistance {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceDirection {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceDirection {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki,
            connectives,
            extra_leaves,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Ki {
            ki,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Ki {
                ki,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Caha {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Caha {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Zaho {
            words,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Zaho {
                words,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Interval {
            number,
            roi_or_tahe,
            nai,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
                free_modifiers: existing_free_modifiers,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_statement(
    words: &[WordWithModifiers],
    source: Option<&str>,
) -> Result<TextSyntax, SyntaxError> {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);

    statement_parser(source)
        .parse(
            tokens
                .as_slice()
                .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
        )
        .into_result()
        .map_err(syntax_error)
}

#[requires(true)]
#[ensures(true)]
fn statement_parser<'tokens>(source: Option<&'tokens str>) -> BoxedParser<'tokens, TextSyntax> {
    let mut text = Recursive::declare();
    let mut argument = Recursive::declare();
    let mut relation = Recursive::declare();
    let mut statement = Recursive::declare();
    let mut subsentence = Recursive::declare();
    let mut free_modifier = Recursive::declare();
    let mut term = Recursive::declare();
    argument.define(argument_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        term.clone(),
        text.clone(),
        free_modifier.clone(),
        source,
    ));
    let tense_modal_with_free_modifiers = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(tense_modal, free_modifiers)| {
            attach_tense_modal_free_modifiers(tense_modal, free_modifiers)
        })
        .boxed();
    relation.define(relation_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        text.clone(),
        free_modifier.clone(),
        source,
    ));

    let argument_term = argument.clone().map(TermSyntax::Argument);
    let elided_argument = cmavo("ku")
        .or_not()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(maybe_ku, free_modifiers)| ArgumentSyntax::Zohe {
            tag_words: Vec::new(),
            maybe_ku,
            free_modifiers,
        });
    let fa_term = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone().or(elided_argument))
        .map(|((fa, free_modifiers), argument)| TermSyntax::Fa {
            fa,
            free_modifiers,
            argument,
            ku: None,
            ku_free_modifiers: Vec::new(),
        });
    let na_ku_term = na_cmavo()
        .then(cmavo("ku"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, na_ku), free_modifiers)| TermSyntax::NaKu {
            na,
            na_ku,
            free_modifiers,
        });
    let bare_na_term_blocker = choice((
        relation.clone().ignored(),
        modal_forethought_connective().ignored(),
        cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]).ignored(),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
            .ignored(),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("GIhA", &["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"]))
            .ignored(),
    ));
    let bare_na_term = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(bare_na_term_blocker.rewind().not())
        .map(|((na, free_modifiers), _)| TermSyntax::BareNa { na, free_modifiers });
    let tagged_term_start = modal_forethought_connective()
        .rewind()
        .not()
        .ignore_then(leading_term_tag_tense_modal())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>());
    let tagged_term_before_tag = tagged_term_start.clone().then(tense_modal().rewind()).map(
        |((tense_modal, free_modifiers), _)| TermSyntax::Tagged {
            tense_modal: Some(tense_modal),
            free_modifiers,
            argument: implicit_zohe_argument(),
        },
    );
    let tagged_term_before_non_relation = tagged_term_start
        .then(relation.clone().rewind().not())
        .then(
            argument
                .clone()
                .or(cmavo("ku").or_not().map(|maybe_ku| ArgumentSyntax::Zohe {
                    tag_words: Vec::new(),
                    maybe_ku,
                    free_modifiers: Vec::new(),
                })),
        )
        .map(
            |(((tense_modal, free_modifiers), _), argument)| TermSyntax::Tagged {
                tense_modal: Some(tense_modal),
                free_modifiers,
                argument,
            },
        );
    let tagged_term = choice((tagged_term_before_tag, tagged_term_before_non_relation));
    let noiha_adverbial = cmavo_of("NOIhA", &["noi'a", "poi'a", "poi'o'a", "soi'a", "noi'o'a"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument_tail_with(
            argument.clone(),
            relation.clone(),
            subsentence.clone(),
            free_modifier.clone(),
        ))
        .then(cmavo("fe'u").map(Ok).or(cmavo("ku").map(Err)).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                (
                    ((noiha, leading_free_modifiers), (tail_elements, relation, relative_clauses)),
                    terminator,
                ),
                trailing_free_modifiers,
            )| {
                match terminator {
                    Some(Err(brigahi_ku)) => TermSyntax::PoihaBrigahi {
                        poiha: noiha,
                        leading_free_modifiers,
                        tail_elements,
                        relation,
                        relative_clauses,
                        brigahi_ku,
                        trailing_free_modifiers,
                    },
                    Some(Ok(fehu)) => TermSyntax::NoihaAdverbial {
                        noiha,
                        leading_free_modifiers,
                        tail_elements,
                        relation,
                        relative_clauses,
                        fehu: Some(fehu),
                        trailing_free_modifiers,
                    },
                    None => TermSyntax::NoihaAdverbial {
                        noiha,
                        leading_free_modifiers,
                        tail_elements,
                        relation,
                        relative_clauses,
                        fehu: None,
                        trailing_free_modifiers,
                    },
                }
            },
        )
        .boxed();
    let fihoi_adverbial = cmavo("fi'oi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(cmavo("fi'au").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((fihoi, leading_free_modifiers), subsentence), fihau), trailing_free_modifiers)| {
                TermSyntax::FihoiAdverbial {
                    fihoi,
                    leading_free_modifiers,
                    subsentence: Box::new(subsentence),
                    fihau,
                    trailing_free_modifiers,
                }
            },
        )
        .boxed();
    let soi_adverbial = cmavo_of("SOI", &["soi", "xoi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(cmavo("se'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((soi, leading_free_modifiers), subsentence), sehu), trailing_free_modifiers)| {
                TermSyntax::SoiAdverbial {
                    soi,
                    leading_free_modifiers,
                    subsentence: Box::new(subsentence),
                    sehu,
                    trailing_free_modifiers,
                }
            },
        )
        .boxed();
    let base_simple_term = choice((
        fa_term,
        tagged_term,
        noiha_adverbial,
        fihoi_adverbial,
        soi_adverbial,
        argument_term,
        na_ku_term,
        bare_na_term,
    ))
    .boxed();
    let term_body = {
        let term = term.clone();
        let gek_nuhi_termset = cmavo("nu'i")
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(modal_forethought_connective())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(gik_connective())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (
                        (
                            (
                                (
                                    ((((m_nuhi, nuhi_free_modifiers), gek), terms), nuhu),
                                    nuhu_free_modifiers,
                                ),
                                gik,
                            ),
                            gik_terms,
                        ),
                        gik_nuhu,
                    ),
                    gik_nuhu_free_modifiers,
                )| {
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
                    }
                },
            );
        let nuhi_termset = cmavo("nu'i")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |((((nuhi, nuhi_free_modifiers), termset), nuhu), nuhu_free_modifiers)| {
                    TermSyntax::NuhiTermset {
                        nuhi,
                        nuhi_free_modifiers,
                        termset,
                        nuhu,
                        nuhu_free_modifiers,
                    }
                },
            );
        let simple_term =
            choice((base_simple_term.clone(), gek_nuhi_termset, nuhi_termset)).boxed();
        let cehe_term = simple_term
            .clone()
            .then(
                cmavo("ce'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(
                        simple_term
                            .clone()
                            .repeated()
                            .at_least(1)
                            .collect::<Vec<_>>(),
                    )
                    .or_not(),
            )
            .map(|(leading_term, cehe_tail)| {
                cehe_tail.map_or(
                    leading_term.clone(),
                    |((cehe, free_modifiers), trailing_terms)| TermSyntax::Cehe {
                        leading_terms: vec![leading_term],
                        cehe,
                        free_modifiers,
                        trailing_terms,
                    },
                )
            })
            .boxed();
        let bo_tail = argument_connective()
            .or_not()
            .then(tense_modal_with_free_modifiers.clone().or_not())
            .then(cmavo("bo"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(simple_term.clone())
            .map(
                |((((bo_connective, tense_modal), bo), free_modifiers), trailing_term)| {
                    (
                        bo_connective,
                        tense_modal,
                        bo,
                        free_modifiers,
                        trailing_term,
                    )
                },
            );
        let term2 = cehe_term
            .clone()
            .then(bo_tail.repeated().collect::<Vec<_>>())
            .map(|(first, tails)| {
                tails.into_iter().fold(
                    first,
                    |leading_term, (bo_connective, tense_modal, bo, free_modifiers, trailing_term)| {
                        TermSyntax::BoConnected {
                            leading_terms: vec![leading_term],
                            bo_connective,
                            tense_modal,
                            bo,
                            free_modifiers,
                            trailing_term: Box::new(trailing_term),
                        }
                    },
                )
            })
            .boxed();
        let pehe_term = term2
            .clone()
            .then(
                cmavo("pe'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(statement_connective())
                    .then(term2.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(leading_term, pehe_tails)| {
                pehe_tails.into_iter().fold(
                    leading_term,
                    |leading_term, (((pehe, free_modifiers), connective), trailing_term)| {
                        TermSyntax::Pehe {
                            leading_terms: vec![leading_term],
                            pehe,
                            free_modifiers,
                            connective,
                            trailing_terms: vec![trailing_term],
                        }
                    },
                )
            })
            .boxed();
        let connected_term = term2
            .clone()
            .then(
                argument_connective()
                    .then(term2.clone())
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(first, tails)| {
                tails
                    .into_iter()
                    .fold(first, |leading_term, (connective, trailing_term)| {
                        TermSyntax::Connected {
                            leading_terms: vec![leading_term],
                            connective,
                            trailing_terms: vec![trailing_term],
                        }
                    })
            })
            .boxed();
        choice((pehe_term, connected_term, term2)).boxed()
    };
    term.define(term_body.boxed());
    let tail_term = term.clone();
    let cu = cmavo("cu");
    let basic_predicate = recursive(|basic_predicate| {
        let gek_sentence = recursive(|gek_sentence| {
            let pair = modal_forethought_connective()
                .then(subsentence.clone())
                .then(gik_connective())
                .then(subsentence.clone())
                .then(tail_term.clone().repeated().collect::<Vec<_>>())
                .then(cmavo("vau").or_not())
                .map(|(((((gek, first), gik), second), tail_terms), vau)| {
                    GekSentenceSyntax::Pair {
                        gek,
                        first: Box::new(first),
                        gik,
                        second: Box::new(second),
                        tail_terms,
                        vau,
                    }
                });
            let ke = tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("ke"))
                .then(gek_sentence.clone())
                .then(cmavo("ke'e").or_not())
                .map(|(((tense_modal, ke), inner), kehe)| GekSentenceSyntax::Ke {
                    tense_modal,
                    ke,
                    inner: Box::new(inner),
                    kehe,
                });
            let na =
                na_cmavo()
                    .then(gek_sentence.clone())
                    .map(|(na, inner)| GekSentenceSyntax::Na {
                        na,
                        inner: Box::new(inner),
                    });
            choice((pair, ke, na)).boxed()
        });
        let implicit_tagged_term_before_grouped_gek = tense_modal_with_free_modifiers
            .clone()
            .then(cmavo("ke").rewind())
            .map(|(tense_modal, _)| TermSyntax::Tagged {
                tense_modal: Some(tense_modal),
                free_modifiers: Vec::new(),
                argument: implicit_zohe_argument(),
            });
        let non_grouped_gek_term = cmavo("ke").rewind().not().ignore_then(term.clone());
        let gek_leading_term = choice((
            implicit_tagged_term_before_grouped_gek,
            non_grouped_gek_term,
        ))
        .boxed();
        let bo_continuation = predicate_tail_connective()
            .then(tense_modal_with_free_modifiers.clone().or_not())
            .then(cmavo("bo"))
            .then(basic_predicate.clone())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .map(
                |(((((connective, tense_modal), bo), predicate_tail), tail_terms), vau)| {
                    PredicateTailBoContinuationSyntax {
                        connective,
                        tense_modal,
                        bo,
                        predicate_tail: Box::new(predicate_tail),
                        tail_terms,
                        vau,
                    }
                },
            )
            .boxed();
        let ke_continuation = predicate_tail_connective()
            .then(tense_modal_with_free_modifiers.clone().or_not())
            .then(cmavo("ke"))
            .then(basic_predicate.clone())
            .then(cmavo("ke'e").or_not())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .map(
                |((((((connective, tense_modal), ke), predicate_tail), kehe), tail_terms), vau)| {
                    PredicateTailKeContinuationSyntax {
                        connective,
                        tense_modal,
                        ke,
                        predicate_tail: Box::new(predicate_tail),
                        kehe,
                        tail_terms,
                        vau,
                    }
                },
            )
            .boxed();
        let bo_or_ke_continuation_start = predicate_tail_connective()
            .then(tense_modal_with_free_modifiers.clone().or_not())
            .then(choice((cmavo("bo"), cmavo("ke"))))
            .rewind();
        let predicate_tail_continuation = bo_or_ke_continuation_start
            .not()
            .ignore_then(predicate_tail_connective())
            .then(relation.clone())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .then(bo_continuation.clone().or_not())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .map(
                |(
                    (((((connective, relation), terms), vau), bo_continuation), tail_terms),
                    tail_vau,
                )| {
                    PredicateTailContinuationSyntax {
                        connective,
                        relation,
                        terms,
                        vau,
                        bo_continuation,
                        tail_terms,
                        tail_vau,
                    }
                },
            )
            .boxed();
        let predicate_with_leading_terms = term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cu.clone()
                    .or_not()
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>()),
            )
            .then(relation.clone())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .then(bo_continuation.clone().or_not())
            .then(
                predicate_tail_continuation
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .then(ke_continuation.clone().or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (
                        (
                            (
                                (
                                    (
                                        ((leading_terms, (cu, cu_free_modifiers)), relation),
                                        tail_terms,
                                    ),
                                    vau,
                                ),
                                bo_continuation,
                            ),
                            continuations,
                        ),
                        ke_continuation,
                    ),
                    free_modifiers,
                )| BasicPredicate {
                    leading_terms,
                    cu,
                    cu_free_modifiers,
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
                    free_modifiers,
                },
            );

        let relation_only = relation
            .clone()
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .then(bo_continuation.clone().or_not())
            .then(
                predicate_tail_continuation
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .then(ke_continuation.clone().or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (
                        ((((relation, tail_terms), vau), bo_continuation), continuations),
                        ke_continuation,
                    ),
                    free_modifiers,
                )| BasicPredicate {
                    leading_terms: Vec::new(),
                    cu: None,
                    cu_free_modifiers: Vec::new(),
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
                    free_modifiers,
                },
            );
        let bare_cu_predicate = cu
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(relation.clone())
            .then(tail_term.clone().repeated().collect::<Vec<_>>())
            .then(cmavo("vau").or_not())
            .then(bo_continuation.or_not())
            .then(
                predicate_tail_continuation
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .then(ke_continuation.or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (
                        (
                            (
                                ((((cu, cu_free_modifiers), relation), tail_terms), vau),
                                bo_continuation,
                            ),
                            continuations,
                        ),
                        ke_continuation,
                    ),
                    free_modifiers,
                )| BasicPredicate {
                    leading_terms: Vec::new(),
                    cu: Some(cu),
                    cu_free_modifiers,
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
                    free_modifiers,
                },
            )
            .boxed();
        let forethought_predicate = gek_sentence
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(gek_sentence, free_modifiers)| BasicPredicate {
                leading_terms: Vec::new(),
                cu: None,
                cu_free_modifiers: Vec::new(),
                relation: RelationSyntax::Compound { units: Vec::new() },
                tail_terms: Vec::new(),
                vau: None,
                gek_sentence: Some(gek_sentence),
                bo_continuation: None,
                ke_continuation: None,
                continuations: Vec::new(),
                free_modifiers,
            });
        let forethought_predicate_with_leading_terms = gek_leading_term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cu.clone()
                    .or_not()
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>()),
            )
            .then(gek_sentence)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((leading_terms, (cu, cu_free_modifiers)), gek_sentence), free_modifiers)| {
                    BasicPredicate {
                        leading_terms,
                        cu,
                        cu_free_modifiers,
                        relation: RelationSyntax::Compound { units: Vec::new() },
                        tail_terms: Vec::new(),
                        vau: None,
                        gek_sentence: Some(gek_sentence),
                        bo_continuation: None,
                        ke_continuation: None,
                        continuations: Vec::new(),
                        free_modifiers,
                    }
                },
            );

        choice((
            forethought_predicate_with_leading_terms,
            forethought_predicate,
            predicate_with_leading_terms,
            bare_cu_predicate,
            relation_only,
        ))
        .boxed()
    });
    let plain_subsentence = basic_predicate.clone().map(SubsentenceSyntax::Plain);
    let prenex_subsentence = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_subsentence)| {
                SubsentenceSyntax::Prenex {
                    prenex_terms,
                    zohu,
                    zohu_free_modifiers,
                    inner_subsentence: Box::new(inner_subsentence),
                }
            },
        );
    subsentence.define(choice((prenex_subsentence, plain_subsentence)));
    let predicate_statement_bo_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .map(
            |((((connective, tense_modal), bo), free_modifiers), trailing_subsentence)| {
                PredicateStatementContinuationSyntax {
                    connective,
                    tense_modal,
                    marker: PredicateStatementContinuationMarkerSyntax::Bo { bo, free_modifiers },
                    trailing_subsentence,
                }
            },
        );
    let predicate_statement_ke_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("ke"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(cmavo("ke'e").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                (
                    ((((connective, tense_modal), ke), ke_free_modifiers), trailing_subsentence),
                    kehe,
                ),
                kehe_free_modifiers,
            )| PredicateStatementContinuationSyntax {
                connective,
                tense_modal,
                marker: PredicateStatementContinuationMarkerSyntax::Ke {
                    ke,
                    ke_free_modifiers,
                    kehe,
                    kehe_free_modifiers,
                },
                trailing_subsentence,
            },
        );
    let predicate_statement_continuation = choice((
        predicate_statement_bo_continuation,
        predicate_statement_ke_continuation,
    ));
    let predicate = basic_predicate
        .clone()
        .then(
            predicate_statement_continuation
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(predicate, continuations)| build_predicate_statement(predicate, continuations));

    let fragment_term = term.clone();

    let term_fragment = fragment_term
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(cmavo("vau").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((terms, vau), vau_free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Term {
                terms,
                vau,
                vau_free_modifiers,
            })
        });

    let relative_clause_fragment =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone()).map(
            |relative_clauses| {
                StatementSyntax::Fragment(FragmentSyntax::RelativeClause { relative_clauses })
            },
        );
    let ek_fragment = ek_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Ek {
                connective,
                free_modifiers,
            })
        });
    let gihek_fragment = predicate_tail_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Gihek {
                connective,
                free_modifiers,
            })
        });

    let multiple_na_fragment = na_cmavo()
        .then(na_cmavo())
        .then(na_cmavo().repeated().collect::<Vec<_>>())
        .then(
            cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"])
                .rewind()
                .not(),
        )
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((((first_na, second_na), rest_na), _), free_modifiers)| {
            let mut words = vec![first_na, second_na];
            words.extend(rest_na);
            StatementSyntax::Fragment(FragmentSyntax::Other {
                words,
                free_modifiers,
            })
        });
    let single_na_fragment_blocker = choice((
        cmavo("ku").ignored(),
        na_cmavo().ignored(),
        cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]).ignored(),
        argument_connective().ignored(),
        predicate_tail_connective().ignored(),
    ));
    let single_na_fragment = na_cmavo()
        .then(single_na_fragment_blocker.rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, _), free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Other {
                words: vec![na],
                free_modifiers,
            })
        });

    let be_link_fragment = be_link_parser(argument.clone(), free_modifier.clone()).map(|link| {
        let data!(BeLinkSyntax {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
        }) = link.into_data();

        {
            StatementSyntax::Fragment(FragmentSyntax::BeLink {
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
            })
        }
    });
    let bei_link_fragment = bei_link_parser(argument.clone(), free_modifier.clone())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|bei_only_links| {
            StatementSyntax::Fragment(FragmentSyntax::BeiLink { bei_only_links })
        });

    let math_expression_fragment = quantifier().map(|quantifier| {
        StatementSyntax::Fragment(FragmentSyntax::MathExpression {
            math_expression: MathExpressionSyntax::Number(quantifier),
        })
    });

    let relation_fragment = relation
        .clone()
        .map(|relation| StatementSyntax::Fragment(FragmentSyntax::Relation { relation }));

    let prenex_fragment = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((terms, zohu), zohu_free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Prenex {
                terms,
                zohu,
                zohu_free_modifiers,
            })
        });

    let prenex_statement = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(statement.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_statement)| {
                StatementSyntax::Prenex {
                    prenex_terms,
                    zohu,
                    zohu_free_modifiers,
                    inner_statement: Box::new(inner_statement),
                }
            },
        );
    let tuhe_statement = tense_modal_with_free_modifiers
        .clone()
        .or_not()
        .then(cmavo("tu'e"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(cmavo("tu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((tense_modal, tuhe), tuhe_free_modifiers), text), tuhu), tuhu_free_modifiers)| {
                StatementSyntax::Tuhe {
                    tense_modal,
                    tuhe,
                    tuhe_free_modifiers,
                    text: Box::new(text),
                    tuhu,
                    tuhu_free_modifiers,
                }
            },
        );

    let simple_statement_after_i_connective = choice((
        predicate,
        tuhe_statement,
        prenex_fragment,
        ek_fragment,
        gihek_fragment,
        be_link_fragment,
        bei_link_fragment,
        relative_clause_fragment,
        multiple_na_fragment,
        single_na_fragment,
        term_fragment,
        math_expression_fragment,
        relation_fragment,
    ))
    .boxed();

    let simple_statement = choice((
        prenex_statement,
        simple_statement_after_i_connective.clone(),
    ));

    let i_connective_statement_tail = cmavo("i")
        .then(statement_connective())
        .then(
            tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("bo"))
                .or_not(),
        )
        .then(simple_statement_after_i_connective.clone())
        .map(|(((i, connective), tag_bo), trailing_statement)| {
            let connective = tag_bo.map_or(connective.clone(), |(tense_modal, bo)| {
                let mut cmavo = connective.cmavo;
                if let Some(tense_modal) = tense_modal {
                    cmavo.extend(tense_modal.words());
                }
                cmavo.push(bo);
                ConnectiveSyntax {
                    kind: connective.kind,
                    se: connective.se,
                    nahe: connective.nahe,
                    na: connective.na,
                    cmavo,
                    nai: connective.nai,
                    free_modifiers: connective.free_modifiers,
                }
            });
            (false, i, connective, trailing_statement)
        });
    let i_bo_statement_tail = cmavo("i")
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("bo"))
        .then(simple_statement_after_i_connective.clone())
        .map(|(((i, tense_modal), bo), trailing_statement)| {
            let mut cmavo = tense_modal.map_or_else(Vec::new, TenseModalSyntax::words);
            cmavo.push(bo);
            (
                false,
                i,
                ConnectiveSyntax {
                    kind: ConnectiveKind::Relation,
                    se: None,
                    nahe: None,
                    na: None,
                    cmavo,
                    nai: None,
                    free_modifiers: Vec::new(),
                },
                trailing_statement,
            )
        });
    let connected_statement_tail = choice((
        i_connective_statement_tail,
        i_bo_statement_tail,
        statement_connective()
            .then(
                tense_modal_with_free_modifiers
                    .clone()
                    .or_not()
                    .then(cmavo("bo"))
                    .or_not(),
            )
            .then(cmavo("i"))
            .then(simple_statement_after_i_connective.clone())
            .map(|(((connective, tag_bo), i), trailing_statement)| {
                let connective = tag_bo.map_or(connective.clone(), |(tense_modal, bo)| {
                    let mut cmavo = connective.cmavo;
                    if let Some(tense_modal) = tense_modal {
                        cmavo.extend(tense_modal.words());
                    }
                    cmavo.push(bo);
                    ConnectiveSyntax {
                        kind: connective.kind,
                        se: connective.se,
                        nahe: connective.nahe,
                        na: connective.na,
                        cmavo,
                        nai: connective.nai,
                        free_modifiers: connective.free_modifiers,
                    }
                });
                (true, i, connective, trailing_statement)
            }),
    ))
    .boxed();
    let statement_body = simple_statement
        .clone()
        .then(connected_statement_tail.repeated().collect::<Vec<_>>())
        .map(|(leading_statement, continuations)| {
            build_connected_statement(leading_statement, continuations)
        });

    let iau_statement_body = statement_body
        .then(
            cmavo("ia'u")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(term.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(statement, iau_tail)| match iau_tail {
            Some(((iau, iau_free_modifiers), reset_terms)) => StatementSyntax::Iau {
                inner_statement: Box::new(statement),
                iau,
                iau_free_modifiers,
                reset_terms,
            },
            None => statement,
        });

    statement.define(iau_statement_body);
    free_modifier.define(choice((
        mai_free(free_modifier.clone()),
        xi_free(free_modifier.clone()),
        sei_free(term.clone(), relation.clone(), free_modifier.clone()),
        soi_free(argument.clone(), free_modifier.clone()),
        to_free(text.clone(), free_modifier.clone()),
        vocative_free(
            argument.clone(),
            relation.clone(),
            subsentence.clone(),
            free_modifier.clone(),
        ),
    )));

    let initial_statement = statement.clone().map(|statement| ParagraphStatementSyntax {
        i: None,
        connective: None,
        free_modifiers: Vec::new(),
        statement: Some(statement),
    });

    let i_connective_tag_bo = statement_connective()
        .or_not()
        .then(
            tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("bo"))
                .or_not(),
        )
        .map(|(connective, tag_bo)| match (connective, tag_bo) {
            (None, None) => None,
            (Some(connective), None) => Some(connective),
            (connective, Some((tense_modal, bo))) => {
                let (kind, se, nahe, na, nai, mut cmavo, free_modifiers) = connective.map_or(
                    (
                        ConnectiveKind::Relation,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                        Vec::new(),
                    ),
                    |connective| {
                        (
                            connective.kind,
                            connective.se,
                            connective.nahe,
                            connective.na,
                            connective.nai,
                            connective.cmavo,
                            connective.free_modifiers,
                        )
                    },
                );
                if let Some(tense_modal) = tense_modal {
                    cmavo.extend(tense_modal.words());
                }
                cmavo.push(bo);
                Some(ConnectiveSyntax {
                    kind,
                    se,
                    nahe,
                    na,
                    cmavo,
                    nai,
                    free_modifiers,
                })
            }
        });

    let following_statement = cmavo("i")
        .then(i_connective_tag_bo)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(statement.clone().or_not())
        .map(
            |(((i, connective), free_modifiers), statement)| ParagraphStatementSyntax {
                i: Some(i),
                connective,
                free_modifiers,
                statement,
            },
        );

    let paragraph_without_niho = initial_statement
        .clone()
        .then(following_statement.clone().repeated().collect::<Vec<_>>())
        .map(|(initial, following)| {
            build_paragraph(
                None,
                Vec::new(),
                Vec::new(),
                std::iter::once(initial).chain(following).collect(),
            )
        });
    let paragraph_starting_with_i = following_statement
        .clone()
        .then(following_statement.clone().repeated().collect::<Vec<_>>())
        .map(|(initial, following)| {
            build_paragraph(
                None,
                Vec::new(),
                Vec::new(),
                std::iter::once(initial).chain(following).collect(),
            )
        });
    let paragraph = choice((paragraph_without_niho, paragraph_starting_with_i)).boxed();
    let paragraph_with_niho = cmavo_of("NIhO", &["ni'o", "no'i"])
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(paragraph.clone().or_not())
        .map(|((niho, free_modifiers), paragraph)| match paragraph {
            Some(mut paragraph) => {
                if paragraph.niho.is_empty() {
                    paragraph.niho = niho;
                }
                if paragraph.free_modifiers.is_empty() {
                    paragraph.free_modifiers = free_modifiers;
                }
                paragraph
            }
            None => build_paragraph(None, niho, free_modifiers, Vec::new()),
        })
        .boxed();
    let paragraphs = choice((
        paragraph
            .clone()
            .then(paragraph_with_niho.clone().repeated().collect::<Vec<_>>())
            .map(|(first, rest)| std::iter::once(first).chain(rest).collect::<Vec<_>>()),
        paragraph_with_niho
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>(),
    ))
    .or_not()
    .map(Option::unwrap_or_default);

    let text_body = cmavo("nai")
        .repeated()
        .collect::<Vec<_>>()
        .then(cmevla_word().repeated().collect::<Vec<_>>())
        .then(leading_indicator().repeated().collect::<Vec<_>>())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(
            modal_forethought_connective()
                .rewind()
                .not()
                .ignore_then(statement_connective())
                .or_not(),
        )
        .then(paragraphs)
        .map(
            |(
                (
                    (((leading_nai, leading_cmevla), leading_indicators), leading_free_modifiers),
                    leading_connective,
                ),
                paragraphs,
            )| {
                TextSyntax {
                    leading_nai,
                    leading_cmevla,
                    leading_indicators,
                    leading_free_modifiers,
                    leading_connective,
                    paragraphs,
                }
            },
        );

    text.define(text_body);
    text.then_ignore(end()).boxed()
}

#[requires(true)]
#[ensures(true)]
fn build_paragraph(
    i: Option<WordWithModifiers>,
    niho: Vec<WordWithModifiers>,
    free_modifiers: Vec<FreeModifierSyntax>,
    statements: Vec<ParagraphStatementSyntax>,
) -> ParagraphSyntax {
    ParagraphSyntax {
        i,
        niho,
        free_modifiers,
        statements: normalize_trailing_ijek_fragment(statements),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_trailing_ijek_fragment(
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let Some(last) = statements.pop() else {
        return statements;
    };
    match last {
        ParagraphStatementSyntax {
            i: Some(i),
            connective: Some(connective),
            free_modifiers,
            statement: None,
        } if free_modifiers.is_empty() => {
            statements.push(ParagraphStatementSyntax {
                i: None,
                connective: None,
                free_modifiers: Vec::new(),
                statement: Some(StatementSyntax::Fragment(FragmentSyntax::Ijek {
                    i,
                    connective,
                })),
            });
            statements
        }
        other => {
            statements.push(other);
            statements
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn build_predicate_statement(
    predicate: BasicPredicate,
    continuations: Vec<PredicateStatementContinuationSyntax>,
) -> StatementSyntax {
    continuations.into_iter().fold(
        StatementSyntax::Predicate(predicate),
        |leading_statement, continuation| StatementSyntax::ExperimentalPredicateContinuation {
            leading_statement: Box::new(leading_statement),
            continuation,
        },
    )
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.clone().words().len() >= old(leading_statement.clone().words().len()))]
fn build_connected_statement(
    leading_statement: StatementSyntax,
    continuations: Vec<(bool, WordWithModifiers, ConnectiveSyntax, StatementSyntax)>,
) -> StatementSyntax {
    let mut statements = vec![leading_statement];
    let mut connectors = Vec::new();
    for (pre_i, i, connective, trailing_statement) in continuations {
        connectors.push((pre_i, i, connective));
        statements.push(trailing_statement);
    }

    for index in (0..connectors.len()).rev() {
        if connective_has_bo(&connectors[index].2) {
            let trailing_statement = statements.remove(index + 1);
            let leading_statement = statements.remove(index);
            let (pre_i, i, connective) = connectors.remove(index);
            statements.insert(
                index,
                if pre_i {
                    StatementSyntax::PreIConnected {
                        connective,
                        i,
                        leading_statement: Box::new(leading_statement),
                        trailing_statement: Box::new(trailing_statement),
                    }
                } else {
                    StatementSyntax::Connected {
                        i,
                        connective,
                        leading_statement: Box::new(leading_statement),
                        trailing_statement: Box::new(trailing_statement),
                    }
                },
            );
        }
    }

    let mut statements = statements.into_iter();
    let mut connected_statement = statements
        .next()
        .expect("there is always at least the leading statement");
    for ((pre_i, i, connective), trailing_statement) in connectors.into_iter().zip(statements) {
        connected_statement = if pre_i {
            StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement: Box::new(connected_statement),
                trailing_statement: Box::new(trailing_statement),
            }
        } else {
            StatementSyntax::Connected {
                i,
                connective,
                leading_statement: Box::new(connected_statement),
                trailing_statement: Box::new(trailing_statement),
            }
        };
    }
    connected_statement
}

#[requires(true)]
#[ensures(ret == connective.cmavo.iter().any(|word| cmavo_text_matches(word, "bo")))]
fn connective_has_bo(connective: &ConnectiveSyntax) -> bool {
    connective
        .cmavo
        .iter()
        .any(|word| cmavo_text_matches(word, "bo"))
}

#[requires(true)]
#[ensures(true)]
fn empty_text() -> TextSyntax {
    TextSyntax {
        leading_nai: Vec::new(),
        leading_cmevla: Vec::new(),
        leading_indicators: Vec::new(),
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn sei_free<'tokens, T, R, F>(
    term: T,
    relation: R,
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    cmavo_of("SEI", &["sei", "ti'o"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(term.repeated().collect::<Vec<_>>())
        .then(
            cmavo("cu")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .then(relation)
        .then(cmavo("se'u").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |(
                (((((sei, leading_free_modifiers), terms), cu), relation), sehu),
                sehu_free_modifiers,
            )| {
                let (cu, cu_free_modifiers) = cu
                    .map(|(cu, free_modifiers)| (Some(cu), free_modifiers))
                    .unwrap_or((None, Vec::new()));
                FreeModifierSyntax::Sei {
                    sei,
                    leading_free_modifiers,
                    terms,
                    cu,
                    cu_free_modifiers,
                    relation,
                    sehu,
                    sehu_free_modifiers,
                }
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn to_free<'tokens, T, F>(text: T, free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let empty_parenthetical = cmavo("to")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmavo("toi"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            move |(((to, free_modifiers), toi), toi_free_modifiers)| FreeModifierSyntax::To {
                to,
                free_modifiers,
                text: Box::new(empty_text()),
                toi: Some(toi),
                toi_free_modifiers,
            },
        );

    let nonempty_parenthetical = cmavo("to")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(
            cmavo("toi")
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((to, free_modifiers), text), toi)| {
            let (toi, toi_free_modifiers) = toi
                .map(|(toi, toi_free_modifiers)| (Some(toi), toi_free_modifiers))
                .unwrap_or((None, Vec::new()));
            FreeModifierSyntax::To {
                to,
                free_modifiers,
                text: Box::new(text),
                toi,
                toi_free_modifiers,
            }
        });

    choice((empty_parenthetical, nonempty_parenthetical)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body<'tokens>() -> BoxedParser<'tokens, MathExpressionSyntax> {
    math_parser_pair().0
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context<'tokens, A, R>(
    argument: A,
    relation: R,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    math_parser_pair_with_context(argument, relation).0
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair_with_context<'tokens, A, R>(
    argument: A,
    relation: R,
) -> (
    BoxedParser<'tokens, MathExpressionSyntax>,
    BoxedParser<'tokens, MathOperatorSyntax>,
)
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let mut expression = Recursive::declare();
    let mut operator = Recursive::declare();
    expression.define(math_expression_body_with_context_inner(
        expression.clone(),
        operator.clone(),
        argument.clone(),
        relation.clone(),
    ));
    operator.define(math_operator_with_context(
        expression.clone(),
        operator.clone(),
        relation,
    ));
    (expression.boxed(), operator.boxed())
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context_inner<'tokens, E, O, A, R>(
    expression: E,
    operator: O,
    argument: A,
    relation: R,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let number = number_quantifier().map(MathExpressionSyntax::Number);
    let letter = letter_string()
        .then(cmavo("boi").or_not())
        .map(|(letter, boi)| MathExpressionSyntax::Letter { letter, boi });
    let nihe = cmavo("ni'e")
        .then(relation.clone())
        .then(cmavo("te'u").or_not())
        .map(|((nihe, relation), tehu)| MathExpressionSyntax::Nihe {
            nihe,
            relation,
            tehu,
        });
    let mohe = cmavo("mo'e")
        .then(argument)
        .then(cmavo("te'u").or_not())
        .map(|((mohe, argument), tehu)| MathExpressionSyntax::Mohe {
            mohe,
            argument: Box::new(argument),
            tehu,
        });
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo("jo'i")
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo("te'u").or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                MathExpressionSyntax::Johi {
                    johi,
                    free_modifiers,
                    expressions,
                    tehu,
                    tehu_free_modifiers,
                }
            },
        );
    let vei = cmavo("vei")
        .then(expression.clone())
        .then(cmavo("ve'o").or_not())
        .map(
            |((vei, inner_expression), veho)| MathExpressionSyntax::Vei {
                vei,
                inner_expression: Box::new(inner_expression),
                veho,
            },
        );
    let gek = modal_forethought_connective()
        .then(expression.clone())
        .then(gik_connective())
        .then(expression)
        .map(
            |(((gek, left_expression), gik), right_expression)| MathExpressionSyntax::Gek {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            },
        );
    let math_operand_atom = choice((gek, vei, nihe, mohe, johi, number, letter)).boxed();
    let math_operand = math_operand_atom
        .clone()
        .then(
            argument_connective()
                .then(math_operand_atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (connective, right_expression)| MathExpressionSyntax::Connected {
                    left_expression: Box::new(left_expression),
                    connective,
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();
    let math_expression2 = recursive(|math_expression2| {
        let lahe = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(cmavo("bo"))
            .then(math_expression2.clone())
            .then(cmavo("lu'u").or_not())
            .map(
                |(((nahe, bo), inner_expression), luhu)| MathExpressionSyntax::Lahe {
                    markers: vec![nahe, bo],
                    inner_expression: Box::new(inner_expression),
                    luhu,
                },
            );
        let forethought = cmavo("pe'o")
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo("ku'e").or_not())
            .map(
                |(((peho, operator), operands), kuhe)| MathExpressionSyntax::Forethought {
                    peho,
                    operator,
                    operands,
                    kuhe,
                },
            );
        choice((math_operand.clone(), lahe, forethought)).boxed()
    });
    let reverse_polish_parts = recursive(|reverse_polish_parts| {
        math_operand
            .clone()
            .then(
                reverse_polish_parts
                    .clone()
                    .then(operator.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first_operand, tails)| {
                let mut operands = vec![first_operand];
                let mut operators = Vec::new();
                for ((mut tail_operands, mut tail_operators), operator) in tails {
                    operands.append(&mut tail_operands);
                    operators.append(&mut tail_operators);
                    operators.push(operator);
                }
                (operands, operators)
            })
    });
    let reverse_polish =
        cmavo("fu'a")
            .then(reverse_polish_parts)
            .map(
                |(fuha, (operands, operators))| MathExpressionSyntax::ReversePolish {
                    fuha,
                    free_modifiers: Vec::new(),
                    operands,
                    operators,
                },
            );
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo("bi'e")
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => MathExpressionSyntax::Bihe {
                    left_expression: Box::new(left_expression),
                    bihe,
                    operator,
                    right_expression: Box::new(right_expression),
                },
            })
    });
    let infix_expression = math_expression1
        .clone()
        .then(
            operator
                .then(math_expression1)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (operator, right_expression)| MathExpressionSyntax::Binary {
                    operator,
                    left_expression: Box::new(left_expression),
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair<'tokens>() -> (
    BoxedParser<'tokens, MathExpressionSyntax>,
    BoxedParser<'tokens, MathOperatorSyntax>,
) {
    let mut expression = Recursive::declare();
    let mut operator = Recursive::declare();
    expression.define(math_expression_body_with(
        expression.clone(),
        operator.clone(),
    ));
    operator.define(math_operator_with(expression.clone(), operator.clone()));
    (expression.boxed(), operator.boxed())
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with<'tokens, E, O>(
    expression: E,
    operator: O,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number = number_quantifier().map(MathExpressionSyntax::Number);
    let letter = letter_string()
        .then(cmavo("boi").or_not())
        .map(|(letter, boi)| MathExpressionSyntax::Letter { letter, boi });
    let vei = cmavo("vei")
        .then(expression.clone())
        .then(cmavo("ve'o").or_not())
        .map(
            |((vei, inner_expression), veho)| MathExpressionSyntax::Vei {
                vei,
                inner_expression: Box::new(inner_expression),
                veho,
            },
        );
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo("jo'i")
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo("te'u").or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                MathExpressionSyntax::Johi {
                    johi,
                    free_modifiers,
                    expressions,
                    tehu,
                    tehu_free_modifiers,
                }
            },
        );
    let gek = modal_forethought_connective()
        .then(expression.clone())
        .then(gik_connective())
        .then(expression)
        .map(
            |(((gek, left_expression), gik), right_expression)| MathExpressionSyntax::Gek {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            },
        );
    let math_operand_atom = choice((gek, vei, johi, number, letter)).boxed();
    let math_operand = math_operand_atom
        .clone()
        .then(
            argument_connective()
                .then(math_operand_atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (connective, right_expression)| MathExpressionSyntax::Connected {
                    left_expression: Box::new(left_expression),
                    connective,
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();
    let math_expression2 = recursive(|math_expression2| {
        let forethought = cmavo("pe'o")
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo("ku'e").or_not())
            .map(
                |(((peho, operator), operands), kuhe)| MathExpressionSyntax::Forethought {
                    peho,
                    operator,
                    operands,
                    kuhe,
                },
            );
        choice((math_operand.clone(), forethought)).boxed()
    });
    let reverse_polish_parts = recursive(|reverse_polish_parts| {
        math_operand
            .clone()
            .then(
                reverse_polish_parts
                    .clone()
                    .then(operator.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first_operand, tails)| {
                let mut operands = vec![first_operand];
                let mut operators = Vec::new();
                for ((mut tail_operands, mut tail_operators), operator) in tails {
                    operands.append(&mut tail_operands);
                    operators.append(&mut tail_operators);
                    operators.push(operator);
                }
                (operands, operators)
            })
    });
    let reverse_polish =
        cmavo("fu'a")
            .then(reverse_polish_parts)
            .map(
                |(fuha, (operands, operators))| MathExpressionSyntax::ReversePolish {
                    fuha,
                    free_modifiers: Vec::new(),
                    operands,
                    operators,
                },
            );
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo("bi'e")
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => MathExpressionSyntax::Bihe {
                    left_expression: Box::new(left_expression),
                    bihe,
                    operator,
                    right_expression: Box::new(right_expression),
                },
            })
    });
    let infix_expression = math_expression1
        .clone()
        .then(
            operator
                .then(math_expression1)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (operator, right_expression)| MathExpressionSyntax::Binary {
                    operator,
                    left_expression: Box::new(left_expression),
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_with<'tokens, A, R, S, F>(
    argument: A,
    relation: R,
    subsentence: S,
    free_modifier: F,
) -> BoxedParser<
    'tokens,
    (
        Vec<ArgumentTailElementSyntax>,
        Option<RelationSyntax>,
        Vec<RelativeClauseSyntax>,
    ),
>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let tail_argument = pa_word()
        .rewind()
        .not()
        .ignore_then(argument.clone())
        .map(|argument| match argument {
            ArgumentSyntax::RelativeClause {
                base_argument,
                vuho: _,
                vuho_free_modifiers: _,
                relative_clauses,
            } => vec![
                ArgumentTailElementSyntax::Argument(base_argument),
                ArgumentTailElementSyntax::RelativeClauses(relative_clauses),
            ],
            argument => vec![ArgumentTailElementSyntax::Argument(Box::new(argument))],
        });
    let contextual_quantifier = quantifier_with_context(argument.clone(), relation.clone());
    let descriptor_relative_clauses =
        relative_clauses(argument.clone(), subsentence, free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);

    let leading_tail_elements = tail_argument
        .or_not()
        .then(descriptor_relative_clauses.clone())
        .map(|(argument, relative_clauses)| {
            let mut tail_elements = argument.into_iter().flatten().collect::<Vec<_>>();
            if !relative_clauses.is_empty() {
                tail_elements.push(ArgumentTailElementSyntax::RelativeClauses(relative_clauses));
            }
            tail_elements
        });

    let relation_tail = relation
        .clone()
        .then(descriptor_relative_clauses.clone())
        .map(|(relation, relative_clauses)| (Vec::new(), Some(relation), relative_clauses));
    let quantifier_relation_tail = contextual_quantifier
        .clone()
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(relation.clone())
        .then(descriptor_relative_clauses.clone())
        .map(|((quantifier, relation), relative_clauses)| {
            (vec![quantifier], Some(relation), relative_clauses)
        });
    let quantifier_argument_tail = contextual_quantifier
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(argument)
        .map(|(quantifier, argument)| {
            (
                vec![
                    quantifier,
                    ArgumentTailElementSyntax::Argument(Box::new(argument)),
                ],
                None,
                Vec::new(),
            )
        });

    leading_tail_elements
        .then(choice((
            quantifier_relation_tail,
            quantifier_argument_tail,
            relation_tail,
        )))
        .map(
            |(mut leading_tail_elements, (tail_elements, relation, relative_clauses))| {
                leading_tail_elements.extend(tail_elements);
                (leading_tail_elements, relation, relative_clauses)
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_parser_with<'tokens, A, R, S, T, F>(
    argument: A,
    relation: R,
    subsentence: impl Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    single_term: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let quote = quote_argument(source, text, free_modifier.clone());

    let math_expression = cmavo_of("LI", &["li", "me'o"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(math_expression_body_with_context(
            argument.clone(),
            relation.clone(),
        ))
        .then(cmavo("lo'o").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((li, li_free_modifiers), expression), loho), loho_free_modifiers)| {
                ArgumentSyntax::MathExpression {
                    li,
                    li_free_modifiers,
                    expression,
                    loho,
                    loho_free_modifiers,
                }
            },
        );

    let letter = letter_string()
        .then(cmavo("boi").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((letter, boi), boi_free_modifiers)| ArgumentSyntax::Letter {
                letter,
                boi,
                boi_free_modifiers,
            },
        );

    let koha = koha_argument()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(koha, free_modifiers)| ArgumentSyntax::Koha {
            koha,
            free_modifiers,
        });
    let lahe = lahe_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(
            relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                ((((lahe, free_modifiers), relative_clauses), inner_argument), luhu),
                luhu_free_modifiers,
            )| ArgumentSyntax::Lahe {
                lahe,
                free_modifiers,
                relative_clauses,
                inner_argument: Box::new(inner_argument),
                luhu,
                luhu_free_modifiers,
            },
        );
    let lahe_term_wrapper = lahe_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((wrapper, free_modifiers), inner_term), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::Lahe,
                    wrapper,
                    wrapper_bo: None,
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();

    let name = la_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((la, la_free_modifiers), names), name_free_modifiers)| ArgumentSyntax::Name {
                la,
                la_free_modifiers,
                names,
                name_free_modifiers,
            },
        );

    let contextual_quantifier = quantifier_with_context(argument.clone(), relation.clone());
    let descriptor_tail = argument_tail_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        free_modifier.clone(),
    );

    let descriptor_with_gadri = le_cmavo()
        .or(la_cmavo())
        .then(descriptor_tail.clone())
        .then(cmavo("ku").or_not())
        .map(
            |((descriptor, (tail_elements, relation, relative_clauses)), ku)| {
                ArgumentSyntax::Descriptor {
                    descriptor: DescriptorSyntax {
                        outer_quantifier: None,
                        descriptor: Some(descriptor),
                        tail_elements,
                        relation,
                        relative_clauses,
                        ku,
                    },
                }
            },
        );
    let descriptor_with_outer_quantifier = contextual_quantifier
        .clone()
        .then(le_cmavo().or(la_cmavo()))
        .then(descriptor_tail.clone())
        .then(cmavo("ku").or_not())
        .map(
            |(
                ((outer_quantifier, descriptor), (tail_elements, relation, relative_clauses)),
                ku,
            )| {
                ArgumentSyntax::Descriptor {
                    descriptor: DescriptorSyntax {
                        outer_quantifier: Some(outer_quantifier),
                        descriptor: Some(descriptor),
                        tail_elements,
                        relation,
                        relative_clauses,
                        ku,
                    },
                }
            },
        );

    let descriptor_without_gadri = contextual_quantifier
        .clone()
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(relation.clone())
        .then(
            relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .map(
            |((quantifier, relation), relative_clauses)| ArgumentSyntax::Descriptor {
                descriptor: DescriptorSyntax {
                    outer_quantifier: None,
                    descriptor: None,
                    tail_elements: vec![quantifier],
                    relation: Some(relation),
                    relative_clauses,
                    ku: None,
                },
            },
        );

    let tense_tagged_argument = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .map(|((tense_modal, free_modifiers), inner_argument)| {
            let tag_words = tense_modal.clone().words();
            ArgumentSyntax::Tagged {
                tag_words,
                tag_tense_modal: Some(tense_modal),
                tag_fa: None,
                free_modifiers,
                inner_argument: Box::new(inner_argument),
            }
        });
    let fa_tagged_argument = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .map(
            |((fa, free_modifiers), inner_argument)| ArgumentSyntax::Tagged {
                tag_words: vec![fa.clone()],
                tag_tense_modal: None,
                tag_fa: Some(fa),
                free_modifiers,
                inner_argument: Box::new(inner_argument),
            },
        );
    let nahe_bo_argument = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((nahe, bo), free_modifiers), inner_argument), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::NaheBo {
                    nahe,
                    bo,
                    free_modifiers,
                    inner_argument: Box::new(inner_argument),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        );
    let nahe_bo_term_wrapper = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                ((((wrapper, wrapper_bo), free_modifiers), inner_term), luhu),
                luhu_free_modifiers,
            )| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::NaheBo,
                    wrapper,
                    wrapper_bo: Some(wrapper_bo),
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let nahe_argument = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo").rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((nahe, _), free_modifiers), inner_argument), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::Nahe {
                    nahe,
                    free_modifiers,
                    inner_argument: Box::new(inner_argument),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let nahe_term_wrapper = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo").rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((wrapper, _), free_modifiers), inner_term), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::Nahe,
                    wrapper,
                    wrapper_bo: None,
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let bridi_description = cmavo_of("LOhOI", &["lo'oi", "mau'a", "xau'a"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(cmavo("ku'au").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((lohoi, lohoi_free_modifiers), subsentence), kuhau), kuhau_free_modifiers)| {
                ArgumentSyntax::BridiDescription {
                    lohoi,
                    lohoi_free_modifiers,
                    subsentence: Box::new(subsentence),
                    kuhau,
                    kuhau_free_modifiers,
                }
            },
        )
        .boxed();
    let na_ku_argument = na_cmavo()
        .then(cmavo("ku"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, ku), free_modifiers)| ArgumentSyntax::NaKu {
            na,
            ku,
            free_modifiers,
        })
        .boxed();

    let quoted_or_simple_argument_core = choice((
        quote,
        math_expression,
        letter,
        lahe,
        lahe_term_wrapper,
        name,
        bridi_description,
    ))
    .boxed();
    let tagged_or_negated_argument_core = choice((
        tense_tagged_argument,
        fa_tagged_argument,
        nahe_bo_argument,
        nahe_bo_term_wrapper,
        nahe_argument,
        nahe_term_wrapper,
        na_ku_argument,
    ))
    .boxed();
    let descriptor_argument_core = choice((
        descriptor_with_outer_quantifier,
        descriptor_with_gadri,
        descriptor_without_gadri,
        koha,
    ))
    .boxed();
    let unquantified_base_argument_core = choice((
        quoted_or_simple_argument_core,
        tagged_or_negated_argument_core,
        descriptor_argument_core,
    ))
    .boxed();
    let base_relative_clauses =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let unquantified_base_argument = unquantified_base_argument_core
        .clone()
        .then(base_relative_clauses.clone())
        .map(|(base_argument, relative_clauses)| {
            if relative_clauses.is_empty() {
                base_argument
            } else {
                ArgumentSyntax::RelativeClause {
                    base_argument: Box::new(base_argument),
                    vuho: None,
                    vuho_free_modifiers: Vec::new(),
                    relative_clauses,
                }
            }
        });
    let quantified_argument = contextual_quantifier
        .then(unquantified_base_argument_core)
        .then(base_relative_clauses)
        .map(|((quantifier, inner_argument), relative_clauses)| {
            let quantified = ArgumentSyntax::Quantified {
                quantifier,
                inner_argument: Box::new(inner_argument),
            };
            if relative_clauses.is_empty() {
                quantified
            } else {
                ArgumentSyntax::RelativeClause {
                    base_argument: Box::new(quantified),
                    vuho: None,
                    vuho_free_modifiers: Vec::new(),
                    relative_clauses,
                }
            }
        });
    let base_argument = choice((unquantified_base_argument, quantified_argument));

    let argument4 = recursive(|argument4| {
        let gek_argument = modal_forethought_connective()
            .then(argument.clone())
            .then(gik_connective())
            .then(argument4)
            .map(
                |(((gek, leading_argument), gik), trailing_argument)| ArgumentSyntax::Gek {
                    gek,
                    leading_argument: Box::new(leading_argument),
                    gik,
                    trailing_argument: Box::new(trailing_argument),
                },
            );

        choice((gek_argument, base_argument.clone())).boxed()
    });
    let argument3 = recursive(|argument3| {
        argument4
            .clone()
            .then(
                argument_connective()
                    .then(tense_modal().or_not())
                    .then(cmavo("bo"))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(argument3)
                    .or_not(),
            )
            .map(|(leading_argument, bo_tail)| {
                bo_tail.map_or(
                    leading_argument.clone(),
                    |(
                        (((bo_connective, bo_tense_modal), bo), free_modifiers),
                        trailing_argument,
                    )| {
                        ArgumentSyntax::Bo {
                            leading_argument: Box::new(leading_argument),
                            bo_connective: Some(bo_connective),
                            bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_argument: Box::new(trailing_argument),
                        }
                    },
                )
            })
            .boxed()
    });
    let argument2 = argument3
        .clone()
        .then(
            argument_connective()
                .then(argument3.clone())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |leading_argument, (connective, trailing_argument)| ArgumentSyntax::Connected {
                    leading_argument: Box::new(leading_argument),
                    connective,
                    trailing_argument: Box::new(trailing_argument),
                },
            )
        })
        .boxed();

    let argument1 = argument2
        .clone()
        .then(
            argument_connective()
                .then(tense_modal().or_not())
                .then(cmavo("ke"))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(argument.clone())
                .then(cmavo("ke'e").or_not())
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(leading_argument, ke_tail)| {
            ke_tail.map_or(
                leading_argument.clone(),
                |(
                    (((((connective, tense_modal), ke), ke_free_modifiers), inner_argument), kehe),
                    kehe_free_modifiers,
                )| {
                    let connective = tense_modal.map_or(connective.clone(), |tense_modal| {
                        append_connective_words(connective, tense_modal.words())
                    });
                    ArgumentSyntax::Connected {
                        leading_argument: Box::new(leading_argument),
                        connective,
                        trailing_argument: Box::new(ArgumentSyntax::Ke {
                            ke,
                            ke_free_modifiers,
                            inner_argument: Box::new(inner_argument),
                            kehe,
                            kehe_free_modifiers,
                        }),
                    }
                },
            )
        })
        .boxed();

    argument1
        .then(
            cmavo("vu'o")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(
                    relative_clauses(argument.clone(), subsentence, free_modifier.clone())
                        .or_not()
                        .map(Option::unwrap_or_default),
                )
                .then(
                    argument_connective()
                        .then(argument)
                        .map(|(connective, argument)| ArgumentConnectionSyntax {
                            connective,
                            argument: Box::new(argument),
                        })
                        .or_not(),
                )
                .or_not(),
        )
        .map(|(base_argument, vuho_attachment)| {
            if let Some((((vuho, vuho_free_modifiers), relative_clauses), connected_argument)) =
                vuho_attachment
            {
                if !relative_clauses.is_empty() && connected_argument.is_none() {
                    ArgumentSyntax::RelativeClause {
                        base_argument: Box::new(base_argument),
                        vuho: Some(vuho),
                        vuho_free_modifiers,
                        relative_clauses,
                    }
                } else {
                    ArgumentSyntax::Vuho {
                        base_argument: Box::new(base_argument),
                        vuho_marker: vuho,
                        vuho_free_modifiers,
                        relative_clauses,
                        connected_argument,
                    }
                }
            } else {
                base_argument
            }
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn implicit_zohe_argument() -> ArgumentSyntax {
    ArgumentSyntax::Zohe {
        tag_words: Vec::new(),
        maybe_ku: None,
        free_modifiers: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn letter_string<'tokens>() -> BoxedParser<'tokens, Vec<WordWithModifiers>> {
    recursive(|letter_string| {
        let letter_tokens = letter_word_tokens_from(letter_string.clone());
        let continuation = choice((pa_word().map(|word| vec![word]), letter_tokens.clone()))
            .repeated()
            .collect::<Vec<_>>();
        letter_tokens.then(continuation).map(|(mut first, rest)| {
            for mut group in rest {
                first.append(&mut group);
            }
            first
        })
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_words<'tokens>() -> BoxedParser<'tokens, Vec<WordWithModifiers>> {
    let letter_tokens = letter_word_tokens_from(letter_string());
    pa_word()
        .map(|word| vec![word])
        .then(
            choice((pa_word().map(|word| vec![word]), letter_tokens))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(mut first, rest)| {
            for mut group in rest {
                first.append(&mut group);
            }
            first
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_or_letter_words<'tokens>() -> BoxedParser<'tokens, Vec<WordWithModifiers>> {
    choice((number_words(), letter_string())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn letter_word_tokens_from<'tokens, L>(
    letter_string: L,
) -> BoxedParser<'tokens, Vec<WordWithModifiers>>
where
    L: Parser<'tokens, ParserInput<'tokens>, Vec<WordWithModifiers>, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive(|letter_tokens| {
        let by = letter_word().map(|word| vec![word]);
        let lau = cmavo_of("LAU", LAU_WORDS)
            .then(letter_tokens.clone())
            .map(|(lau, mut rest)| {
                let mut words = vec![lau];
                words.append(&mut rest);
                words
            });
        let tei = cmavo("tei")
            .then(letter_string.clone())
            .then(cmavo("foi"))
            .map(|((tei, mut inner), foi)| {
                let mut words = vec![tei];
                words.append(&mut inner);
                words.push(foi);
                words
            });
        choice((by, lau, tei)).boxed()
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_quantifier<'tokens>() -> BoxedParser<'tokens, QuantifierSyntax> {
    number_words()
        .then(cmavo("boi").or_not())
        .map(|(number, boi)| QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier<'tokens>() -> BoxedParser<'tokens, QuantifierSyntax> {
    let vei_quantifier = cmavo("vei")
        .then(math_expression_body())
        .then(cmavo("ve'o").or_not())
        .map(|((vei, math_expression), veho)| QuantifierSyntax::Vei {
            vei,
            math_expression: Box::new(math_expression),
            veho,
        });
    choice((vei_quantifier, number_quantifier())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_context<'tokens, A, R>(
    argument: A,
    relation: R,
) -> BoxedParser<'tokens, QuantifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let vei_quantifier = cmavo("vei")
        .then(math_expression_body_with_context(argument, relation))
        .then(cmavo("ve'o").or_not())
        .map(|((vei, math_expression), veho)| QuantifierSyntax::Vei {
            vei,
            math_expression: Box::new(math_expression),
            veho,
        });
    choice((vei_quantifier, number_quantifier())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quote_argument<'tokens, T, F>(
    source: Option<&'tokens str>,
    text: T,
    free_modifier: F,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let compound_quote = any()
        .try_map(move |word: WordWithModifiers, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, "expected quote"));
            };

            match word_like.as_data() {
                data!(WordLike::ZoQuote { word: quoted, .. }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::Zo {
                        zo: word.clone(),
                        word: base_word_from_record((**quoted).clone()),
                        free_modifiers: Vec::new(),
                    },
                }),
                data!(WordLike::ZoiQuote {
                    zoi,
                    opening_delimiter,
                    quoted_text,
                    closing_delimiter,
                    ..
                }) => {
                    let opening_delimiter = base_word_from_record((**opening_delimiter).clone());
                    let closing_delimiter = base_word_from_record((**closing_delimiter).clone());
                    let quoted_text = source_text(source, quoted_text);
                    if word_record_text_matches(zoi, "la'o") {
                        Ok(ArgumentSyntax::Quote {
                            quote: QuoteSyntax::Laho {
                                laho: word.clone(),
                                opening_delimiter,
                                closing_delimiter,
                                quoted_text,
                                free_modifiers: Vec::new(),
                            },
                        })
                    } else {
                        Ok(ArgumentSyntax::Quote {
                            quote: QuoteSyntax::Zoi {
                                zoi: word.clone(),
                                opening_delimiter,
                                closing_delimiter,
                                quoted_text,
                                free_modifiers: Vec::new(),
                            },
                        })
                    }
                }
                data!(WordLike::LohuQuote {
                    quoted_words,
                    lehu,
                    ..
                }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::Lohu {
                        lohu: word.clone(),
                        quoted_words: quoted_words
                            .iter()
                            .cloned()
                            .map(base_word_from_record)
                            .collect(),
                        lehu: base_word_from_record((**lehu).clone()),
                        lehu_free_modifiers: Vec::new(),
                    },
                }),
                data!(WordLike::SingleWordQuote {
                    marker: _,
                    quoted_text,
                }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::ZohOi {
                        zohoi: word.clone(),
                        quoted_text: source_text(source, quoted_text),
                        free_modifiers: Vec::new(),
                    },
                }),
                _ => Err(Rich::custom(span, "expected quote")),
            }
        })
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(argument, free_modifiers)| attach_quote_free_modifiers(argument, free_modifiers));

    let lu_quote = cmavo("lu")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(cmavo("li'u").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |((((lu, free_modifiers), text), lihu), lihu_free_modifiers)| ArgumentSyntax::Quote {
                quote: QuoteSyntax::Lu {
                    lu,
                    free_modifiers,
                    text,
                    lihu,
                    lihu_free_modifiers,
                },
            },
        );

    choice((compound_quote, lu_quote)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn attach_quote_free_modifiers(
    argument: ArgumentSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> ArgumentSyntax {
    match argument {
        ArgumentSyntax::Quote { quote } => ArgumentSyntax::Quote {
            quote: quote_with_free_modifiers(quote, free_modifiers),
        },
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_with_free_modifiers(
    quote: QuoteSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> QuoteSyntax {
    match quote {
        QuoteSyntax::Lu {
            lu,
            free_modifiers: mut leading_free_modifiers,
            text,
            lihu,
            lihu_free_modifiers,
        } => {
            leading_free_modifiers.extend(free_modifiers);
            QuoteSyntax::Lu {
                lu,
                free_modifiers: leading_free_modifiers,
                text,
                lihu,
                lihu_free_modifiers,
            }
        }
        QuoteSyntax::Zo { zo, word, .. } => QuoteSyntax::Zo {
            zo,
            word,
            free_modifiers,
        },
        QuoteSyntax::ZohOi {
            zohoi, quoted_text, ..
        } => QuoteSyntax::ZohOi {
            zohoi,
            quoted_text,
            free_modifiers,
        },
        QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            ..
        } => QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        },
        QuoteSyntax::Laho {
            laho,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            ..
        } => QuoteSyntax::Laho {
            laho,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        },
        QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            lehu,
            ..
        } => QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            lehu,
            lehu_free_modifiers: free_modifiers,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_word_like(word: &WordWithModifiers) -> Option<&WordLike> {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like })
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => Some(word_like),
        data!(WordWithModifiers::WithIndicator { base, .. }) => quote_word_like(base),
        data!(WordWithModifiers::StandaloneIndicator { .. }) | data!(WordWithModifiers::NotEof) => {
            None
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clauses<'tokens, A, S>(
    argument: A,
    subsentence: S,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, Vec<RelativeClauseSyntax>>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let clause = relative_clause(argument, subsentence, free_modifier.clone());
    clause
        .clone()
        .then(
            choice((
                cmavo("zi'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(clause.clone())
                    .map(
                        |((zihe, free_modifiers), inner)| RelativeClauseSyntax::Zihe {
                            zihe,
                            free_modifiers,
                            inner: Box::new(inner),
                        },
                    ),
                relative_clause_connective()
                    .then(clause)
                    .map(|(connective, inner)| RelativeClauseSyntax::Connected {
                        connective,
                        inner: Box::new(inner),
                    }),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn relative_clause<'tokens, R>(
    argument: impl Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    subsentence: R,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, RelativeClauseSyntax>
where
    R: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let goi = goi_relative_clause(argument, free_modifier.clone()).map(RelativeClauseSyntax::Goi);
    let noi = cmavo_of("NOI", &["poi", "noi", "voi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence)
        .then(cmavo("ku'o").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |((((marker, leading_free_modifiers), subsentence), kuho), trailing_free_modifiers)| {
                if cmavo_text_matches(&marker, "poi") {
                    RelativeClauseSyntax::Poi {
                        poi: marker,
                        leading_free_modifiers,
                        subsentence,
                        kuho,
                        trailing_free_modifiers,
                    }
                } else {
                    RelativeClauseSyntax::Noi {
                        noi: marker,
                        leading_free_modifiers,
                        subsentence,
                        kuho,
                        trailing_free_modifiers,
                    }
                }
            },
        );
    choice((goi, noi)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause<'tokens, A>(
    argument: A,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, GoiRelativeClauseSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    cmavo_of("GOI", &["pe", "ne", "po", "po'e", "po'u", "no'u", "goi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument)
        .then(cmavo("ge'u").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |((((goi, leading_free_modifiers), argument), gehu), trailing_free_modifiers)| {
                GoiRelativeClauseSyntax {
                    goi,
                    leading_free_modifiers,
                    argument,
                    gehu,
                    trailing_free_modifiers,
                }
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn xi_free<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number_or_letter = number_or_letter_words()
        .then(cmavo("boi").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, boi), free_modifiers)| {
            MathExpressionSyntax::Number(QuantifierSyntax::Number {
                number,
                boi,
                free_modifiers,
            })
        });
    let xi_expression = choice((number_or_letter, math_expression_body()));

    cmavo_of("XI", &["xi", "te'ai"])
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(xi_expression)
        .map(
            |((xi, free_modifiers), expression)| FreeModifierSyntax::Xi {
                xi,
                free_modifiers,
                expression,
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn mai_free<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    number_or_letter_words()
        .then(cmavo_of("MAI", MAI_WORDS))
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|((number, mai), free_modifiers)| FreeModifierSyntax::Mai {
            number,
            mai,
            free_modifiers,
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn soi_free<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    cmavo("soi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(argument.or_not())
        .then(cmavo("se'u").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |(
                ((((soi, free_modifiers), leading_argument), trailing_argument), sehu),
                sehu_free_modifiers,
            )| FreeModifierSyntax::Soi {
                soi,
                free_modifiers,
                leading_argument: Box::new(leading_argument),
                trailing_argument: trailing_argument.map(Box::new),
                sehu,
                sehu_free_modifiers,
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vocative_free<'tokens, A, R>(
    argument: A,
    relation: R,
    subsentence: impl Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let optional_relative_clauses =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let relation_vocative = optional_relative_clauses
        .clone()
        .then(relation)
        .then(optional_relative_clauses.clone())
        .map(
            |((leading_relative_clauses, relation), trailing_relative_clauses)| {
                ArgumentSyntax::RelationVocative {
                    leading_relative_clauses,
                    relation,
                    trailing_relative_clauses,
                }
            },
        );
    let cmevla_vocative = optional_relative_clauses
        .clone()
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(optional_relative_clauses)
        .map(
            |(((leading_relative_clauses, cmevla), free_modifiers), trailing_relative_clauses)| {
                let argument = ArgumentSyntax::Cmevla {
                    cmevla,
                    free_modifiers,
                };
                let relative_clauses = leading_relative_clauses
                    .into_iter()
                    .chain(trailing_relative_clauses)
                    .collect::<Vec<_>>();
                if relative_clauses.is_empty() {
                    argument
                } else {
                    ArgumentSyntax::RelativeClause {
                        base_argument: Box::new(argument),
                        vuho: None,
                        vuho_free_modifiers: Vec::new(),
                        relative_clauses,
                    }
                }
            },
        );
    let vocative_argument = choice((relation_vocative, cmevla_vocative, argument));

    vocative_markers()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(vocative_argument.or_not())
        .then(cmavo("do'u").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |((((vocative_markers, free_modifiers), argument), dohu), dohu_free_modifiers)| {
                FreeModifierSyntax::Vocative {
                    vocative_markers,
                    free_modifiers,
                    argument,
                    dohu,
                    dohu_free_modifiers,
                }
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vocative_markers<'tokens>() -> BoxedParser<'tokens, Vec<WordWithModifiers>> {
    let coi_marker = cmavo_of("COI", COI_WORDS)
        .then(cmavo("nai").or_not())
        .map(|(coi, nai)| [vec![coi], nai.into_iter().collect()].concat());

    choice((
        coi_marker
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(cmavo("doi").or_not())
            .map(|(coi_markers, doi)| {
                let mut markers = coi_markers.into_iter().flatten().collect::<Vec<_>>();
                markers.extend(doi);
                markers
            }),
        cmavo("doi").map(|doi| vec![doi]),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let tagged_term_start = choice((tense_modal().ignored(), cmavo_of("FA", FA_WORDS).ignored()));
    let cehe_connective = cmavo("ce'e")
        .then_ignore(tagged_term_start.rewind().not())
        .then(cmavo("nai").or_not())
        .map(|(cmavo, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::NonLogical,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        });
    choice((
        cehe_connective,
        na_cmavo()
            .or_not()
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
            .then(cmavo("nai").or_not())
            .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Afterthought,
                se,
                nahe: None,
                na,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        na_cmavo()
            .or_not()
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("JEhI", &["je'i", "ja", "je", "jo", "ju"]))
            .then(cmavo("nai").or_not())
            .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Afterthought,
                se,
                nahe: None,
                na,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of(
            "JOI",
            &[
                "ce", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a", "pi'u",
            ],
        )
        .then(cmavo("nai").or_not())
        .map(|(cmavo, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::NonLogical,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        }),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Interval,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("GAhO", &["ga'o", "ke'i"])
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .then(cmavo_of("GAhO", &["ga'o", "ke'i"]))
            .map(
                |((((left_interval, se), cmavo), nai), right_interval)| ConnectiveSyntax {
                    kind: ConnectiveKind::Interval,
                    se,
                    nahe: None,
                    na: None,
                    cmavo: vec![left_interval, cmavo, right_interval],
                    nai,
                    free_modifiers: Vec::new(),
                },
            ),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn ek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Afterthought,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(ret.cmavo.len() >= old(words.len()))]
fn append_connective_words(
    connective: ConnectiveSyntax,
    words: Vec<WordWithModifiers>,
) -> ConnectiveSyntax {
    let mut cmavo = connective.cmavo;
    cmavo.extend(words);
    ConnectiveSyntax {
        kind: connective.kind,
        se: connective.se,
        nahe: connective.nahe,
        na: connective.na,
        cmavo,
        nai: connective.nai,
        free_modifiers: connective.free_modifiers,
    }
}

#[requires(true)]
#[ensures(true)]
fn jek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Relation,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn joik_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of(
                "JOI",
                &[
                    "ce", "ce'e", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a", "pi'u",
                ],
            ))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::NonLogical,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Interval,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("GAhO", &["ga'o", "ke'i"])
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .then(cmavo_of("GAhO", &["ga'o", "ke'i"]))
            .map(
                |((((left_interval, se), cmavo), nai), right_interval)| ConnectiveSyntax {
                    kind: ConnectiveKind::Interval,
                    se,
                    nahe: None,
                    na: None,
                    cmavo: vec![left_interval, cmavo, right_interval],
                    nai,
                    free_modifiers: Vec::new(),
                },
            ),
    ))
    .boxed()
}

#[requires(!connective.cmavo.is_empty())]
#[ensures(ret.len() >= old(connective.cmavo.len()))]
fn connective_tense_modal_leaves(connective: ConnectiveSyntax) -> Vec<WordWithModifiers> {
    let mut leaves = Vec::new();
    leaves.extend(connective.se);
    leaves.extend(connective.nahe);
    leaves.extend(connective.na);
    leaves.extend(connective.cmavo);
    leaves.extend(connective.nai);
    leaves
}

#[requires(true)]
#[ensures(true)]
fn statement_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn guhek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("GUhA", &["gu'a", "gu'e", "gu'i", "gu'o", "gu'u"]))
        .then(cmavo("nai").or_not())
        .map(|(((nahe, se), guha), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se,
            nahe,
            na: None,
            cmavo: vec![guha],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn modal_forethought_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let ga = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .or_not()
        .then(cmavo_of("GA", &["ga", "ge", "ge'i", "go", "gu"]))
        .then(cmavo("nai").or_not())
        .map(|((se, ga), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se,
            nahe: None,
            na: None,
            cmavo: vec![ga],
            nai,
            free_modifiers: Vec::new(),
        });
    let modal_gi = tense_modal().then(cmavo("gi")).map(|(tense_modal, gi)| {
        let mut cmavo = tense_modal.words();
        cmavo.push(gi);
        ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
            free_modifiers: Vec::new(),
        }
    });
    let joik_gi = joik_connective()
        .then(cmavo("gi"))
        .then(cmavo("bo").or_not())
        .map(|((connective, gi), bo)| {
            let extra = [Some(gi), bo].into_iter().flatten().collect::<Vec<_>>();
            append_connective_words(connective, extra)
        });
    choice((ga, joik_gi, modal_gi)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn gik_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    cmavo("gi")
        .then(cmavo("nai").or_not())
        .map(|(gi, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![gi],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("GIhA", &["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::PredicateTail,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_operator<'tokens>() -> BoxedParser<'tokens, MathOperatorSyntax> {
    math_parser_pair().1
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with<'tokens, E, O>(
    expression: E,
    operator: O,
) -> BoxedParser<'tokens, MathOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let vuhu = cmavo_of("VUhU", VUHU_WORDS).map(|vuhu| MathOperatorSyntax::Vuhu { vuhu });
    let maho = cmavo("ma'o")
        .then(expression)
        .then(cmavo("te'u").or_not())
        .map(|((maho, math_expression), tehu)| MathOperatorSyntax::Maho {
            maho,
            math_expression: Box::new(math_expression),
            tehu,
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator)
        .map(
            |(((guhek, left_operator), gik), right_operator)| MathOperatorSyntax::Connected {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            },
        );
    let atom = choice((forethought, maho, vuhu)).boxed();
    atom.clone()
        .then(
            statement_connective()
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    MathOperatorSyntax::Connected {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    }
                })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with_context<'tokens, E, O, R>(
    expression: E,
    operator: O,
    relation: R,
) -> BoxedParser<'tokens, MathOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let vuhu = cmavo_of("VUhU", VUHU_WORDS).map(|vuhu| MathOperatorSyntax::Vuhu { vuhu });
    let maho = cmavo("ma'o")
        .then(expression)
        .then(cmavo("te'u").or_not())
        .map(|((maho, math_expression), tehu)| MathOperatorSyntax::Maho {
            maho,
            math_expression: Box::new(math_expression),
            tehu,
        });
    let se = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(operator.clone())
        .map(|(se, inner_operator)| MathOperatorSyntax::Se {
            se,
            inner_operator: Box::new(inner_operator),
        });
    let nahe = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(operator.clone())
        .map(|(nahe, inner_operator)| MathOperatorSyntax::Nahe {
            nahe,
            inner_operator: Box::new(inner_operator),
        });
    let nahu = cmavo("na'u")
        .then(relation)
        .then(cmavo("te'u").or_not())
        .map(|((nahu, relation), tehu)| MathOperatorSyntax::Nahu {
            nahu,
            relation,
            tehu,
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator)
        .map(
            |(((guhek, left_operator), gik), right_operator)| MathOperatorSyntax::Connected {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            },
        );
    let atom = choice((se, nahe, forethought, nahu, maho, vuhu)).boxed();
    atom.clone()
        .then(
            statement_connective()
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    MathOperatorSyntax::Connected {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    }
                })
        })
        .boxed()
}

#[requires(!marker_text.is_empty())]
#[ensures(true)]
fn single_word_quoted_relation_unit<'tokens, F>(
    marker_text: &'static str,
    source: Option<&'tokens str>,
    free_modifier: F,
    build: fn(WordWithModifiers, String, Vec<FreeModifierSyntax>) -> RelationUnitSyntax,
) -> BoxedParser<'tokens, RelationUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    any()
        .try_map(move |word: WordWithModifiers, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            let data!(WordLike::SingleWordQuote {
                marker,
                quoted_text,
            }) = word_like.as_data()
            else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            if word_record_text_matches(marker, marker_text) {
                Ok((word.clone(), source_text(source, quoted_text)))
            } else {
                Err(Rich::custom(span, format!("expected {marker_text} quote")))
            }
        })
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(move |((word, quoted_text), free_modifiers)| build(word, quoted_text, free_modifiers))
        .boxed()
}

#[requires(!marker_text.is_empty())]
#[ensures(true)]
fn delimited_quoted_relation_unit<'tokens, F>(
    marker_text: &'static str,
    source: Option<&'tokens str>,
    free_modifier: F,
    build: fn(
        WordWithModifiers,
        WordWithModifiers,
        WordWithModifiers,
        String,
        Vec<FreeModifierSyntax>,
    ) -> RelationUnitSyntax,
) -> BoxedParser<'tokens, RelationUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    any()
        .try_map(move |word: WordWithModifiers, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            let data!(WordLike::ZoiQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) = word_like.as_data()
            else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            if word_record_text_matches(zoi, marker_text) {
                Ok((
                    word.clone(),
                    base_word_from_record((**opening_delimiter).clone()),
                    base_word_from_record((**closing_delimiter).clone()),
                    source_text(source, quoted_text),
                ))
            } else {
                Err(Rich::custom(span, format!("expected {marker_text} quote")))
            }
        })
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            move |((word, opening_delimiter, closing_delimiter, quoted_text), free_modifiers)| {
                build(
                    word,
                    opening_delimiter,
                    closing_delimiter,
                    quoted_text,
                    free_modifiers,
                )
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_parser_with<'tokens, P, R, S, T, F>(
    argument: P,
    relation: R,
    subsentence: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let me_unit = cmavo("me")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(cmavo("me'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmavo_of("MOI", MOI_WORDS).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                (((((me, me_free_modifiers), argument), mehu), mehu_free_modifiers), moi_marker),
                moi_free_modifiers,
            )| RelationUnitSyntax::Me {
                me,
                me_free_modifiers,
                argument,
                mehu,
                mehu_free_modifiers,
                moi_marker,
                moi_free_modifiers,
            },
        );
    let mehoi_unit = single_word_quoted_relation_unit(
        "me'oi",
        source,
        free_modifier.clone(),
        |mehoi, quoted_text, free_modifiers| RelationUnitSyntax::Mehoi {
            mehoi,
            quoted_text,
            free_modifiers,
        },
    );
    let gohoi_unit = single_word_quoted_relation_unit(
        "go'oi",
        source,
        free_modifier.clone(),
        |gohoi, quoted_text, free_modifiers| RelationUnitSyntax::Gohoi {
            gohoi,
            quoted_text,
            free_modifiers,
        },
    );
    let muhoi_unit = delimited_quoted_relation_unit(
        "mu'oi",
        source,
        free_modifier.clone(),
        |muhoi, opening_delimiter, closing_delimiter, quoted_text, free_modifiers| {
            RelationUnitSyntax::Muhoi {
                muhoi,
                opening_delimiter,
                closing_delimiter,
                quoted_text,
                free_modifiers,
            }
        },
    );
    let luhei_unit = cmavo("lu'ei")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(cmavo("li'au").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((luhei, luhei_free_modifiers), text), liau), liau_free_modifiers)| {
                RelationUnitSyntax::Luhei {
                    luhei,
                    luhei_free_modifiers,
                    text,
                    liau,
                    liau_free_modifiers,
                }
            },
        )
        .boxed();

    let brivla_word_unit = brivla_relation_word()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(word, free_modifiers)| RelationUnitSyntax::Word {
            word,
            free_modifiers,
        });
    let goha_word_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then_ignore(
            choice((
                cmavo("ra'o").ignored(),
                cmavo("be").ignored(),
                free_modifier.clone().ignored(),
            ))
            .rewind()
            .not(),
        )
        .map(|word| RelationUnitSyntax::Word {
            word,
            free_modifiers: Vec::new(),
        });
    let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
    let goha_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then(cmavo("ra'o").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| RelationUnitSyntax::Goha {
            goha,
            raho,
            free_modifiers,
        });
    let goha_raho_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then(cmavo("ra'o"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| RelationUnitSyntax::Goha {
            goha,
            raho: Some(raho),
            free_modifiers,
        });
    let moi_unit = number_or_letter_words()
        .then(cmavo_of("MOI", MOI_WORDS))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, moi), free_modifiers)| RelationUnitSyntax::Moi {
            number,
            moi,
            free_modifiers,
        });
    let nuha_unit = cmavo("nu'a")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(math_operator())
        .map(
            |((nuha, free_modifiers), math_operator)| RelationUnitSyntax::Nuha {
                nuha,
                free_modifiers,
                math_operator,
            },
        );
    let xohi_unit = cmavo("xo'i")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal())
        .map(|((xohi, free_modifiers), tag)| RelationUnitSyntax::Xohi {
            xohi,
            free_modifiers,
            tag,
        });

    let ke_unit = cmavo("ke")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .then(cmavo("ke'e").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((ke, ke_free_modifiers), relation), kehe), kehe_free_modifiers)| {
                RelationUnitSyntax::Ke {
                    ke_tense_modal: None,
                    ke,
                    ke_free_modifiers,
                    relation,
                    kehe,
                    kehe_free_modifiers,
                }
            },
        );

    let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(choice((
            ke_unit.clone(),
            moi_unit.clone(),
            nuha_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(
            |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                se,
                free_modifiers,
                inner_unit: Box::new(inner_unit),
            },
        );

    let wrapped_tense_unit = tense_modal()
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .map(
            |(tense_modal, inner_relation)| RelationUnitSyntax::Wrapped {
                relation: RelationSyntax::TenseModal {
                    tense_modal,
                    inner_relation: Box::new(inner_relation),
                },
            },
        );

    let nahe_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(choice((
            wrapped_tense_unit,
            ke_unit.clone(),
            moi_unit.clone(),
            se_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(
            |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                nahe,
                free_modifiers,
                inner_unit: Box::new(inner_unit),
            },
        );

    let jai_unit = cmavo("jai")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal().or_not())
        .then(choice((
            se_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(
            |(((jai, free_modifiers), tense_modal), inner_unit)| RelationUnitSyntax::Jai {
                jai,
                free_modifiers,
                tense_modal,
                inner_unit: Box::new(inner_unit),
            },
        );

    let nu_cmavo = || cmavo_of("NU", NU_WORDS);
    let additional_nu = statement_connective()
        .then(nu_cmavo())
        .map(|(connective, nu)| AdditionalNuSyntax { connective, nu });
    let abstraction_subsentence_unit = nu_cmavo()
        .then(additional_nu.repeated().collect::<Vec<_>>())
        .then(subsentence)
        .then(cmavo("kei").or_not())
        .map(
            |(((nu, additional_nu), subsentence), kei)| RelationUnitSyntax::Abstraction {
                abstraction: AbstractionSyntax {
                    nu,
                    additional_nu,
                    subsentence: Box::new(subsentence),
                    kei,
                },
            },
        )
        .boxed();

    let se_abstraction_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(abstraction_subsentence_unit.clone())
        .map(
            |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                se,
                free_modifiers,
                inner_unit: Box::new(inner_unit),
            },
        );

    let base_unit = choice((
        goha_raho_unit.clone(),
        me_unit.clone(),
        mehoi_unit.clone(),
        gohoi_unit.clone(),
        muhoi_unit.clone(),
        luhei_unit.clone(),
        se_abstraction_unit.clone(),
        abstraction_subsentence_unit.clone(),
        jai_unit.clone(),
        nahe_unit.clone(),
        se_unit.clone(),
        ke_unit.clone(),
        xohi_unit.clone(),
        nuha_unit.clone(),
        moi_unit.clone(),
        word_unit.clone(),
        goha_unit.clone(),
    ))
    .boxed();
    let base_unit_for_cei = choice((
        goha_raho_unit.clone(),
        me_unit.clone(),
        mehoi_unit.clone(),
        gohoi_unit.clone(),
        muhoi_unit.clone(),
        luhei_unit.clone(),
        se_abstraction_unit.clone(),
        abstraction_subsentence_unit.clone(),
        jai_unit.clone(),
        nahe_unit.clone(),
        se_unit.clone(),
        ke_unit.clone(),
        xohi_unit,
        nuha_unit.clone(),
        moi_unit.clone(),
        goha_unit.clone(),
        word_unit.clone(),
    ))
    .boxed();
    let be_link = be_link_parser(argument.clone(), free_modifier.clone());
    let selbri_relative_clause = cmavo("no'oi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation.clone())
        .then(cmavo("ku'oi").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((nohoi, leading_free_modifiers), relation), kuhoi), trailing_free_modifiers)| {
                SelbriRelativeClauseSyntax {
                    nohoi,
                    leading_free_modifiers,
                    relation,
                    kuhoi,
                    trailing_free_modifiers,
                }
            },
        )
        .boxed();

    let linked_unit_from = |base_unit: BoxedParser<'tokens, RelationUnitSyntax>| {
        base_unit
            .then(be_link.clone().or_not())
            .map(|(base, be_link)| {
                be_link.map_or(base.clone(), |link| {
                    let data!(BeLinkSyntax {
                        be,
                        free_modifiers,
                        fa,
                        fa_free_modifiers,
                        first_argument,
                        bei_links,
                        beho,
                        beho_free_modifiers,
                    }) = link.into_data();

                    RelationUnitSyntax::Be {
                        base: Box::new(base),
                        be,
                        free_modifiers,
                        fa,
                        fa_free_modifiers,
                        first_argument,
                        bei_links,
                        beho,
                        beho_free_modifiers,
                    }
                })
            })
            .then(
                selbri_relative_clause
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(linked_unit, selbri_relative_clauses)| {
                if selbri_relative_clauses.is_empty() {
                    linked_unit
                } else {
                    RelationUnitSyntax::SelbriRelativeClause {
                        base: Box::new(linked_unit),
                        selbri_relative_clauses,
                    }
                }
            })
            .boxed()
    };
    let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
        let data!(BeLinkSyntax {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
        }) = link.into_data();

        RelationUnitSyntax::PreposedBe {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
            base: Box::new(base),
        }
    });
    let linked_unit = linked_unit_from(base_unit);
    let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
    let cei_unit = linked_unit_for_cei
        .clone()
        .then(
            cmavo("cei")
                .then(linked_unit_for_cei.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(base, be_link)| RelationUnitSyntax::Cei {
            base: Box::new(base),
            assignments: be_link
                .into_iter()
                .map(|(cei, relation_unit)| CeiAssignmentSyntax {
                    cei,
                    free_modifiers: Vec::new(),
                    relation_unit,
                })
                .collect(),
        })
        .boxed();

    let bo_unit = recursive(|bo_unit| {
        let guha_unit = guhek_connective()
            .then(relation.clone())
            .then(gik_connective())
            .then(bo_unit.clone())
            .map(
                |(((guhek, leading_relation), gik), trailing_unit)| RelationUnitSyntax::Wrapped {
                    relation: RelationSyntax::Guha {
                        guhek,
                        leading_predicate: Box::new(relation_to_empty_predicate(leading_relation)),
                        gik,
                        trailing_predicate: Box::new(relation_to_empty_predicate(
                            relation_unit_to_relation(&trailing_unit),
                        )),
                    },
                },
            );
        let atom_unit = choice((
            guha_unit,
            preposed_unit.clone(),
            cei_unit.clone(),
            linked_unit.clone(),
        ))
        .boxed();
        let connected_bo_tail = statement_connective()
            .then(tense_modal().or_not())
            .then(cmavo("bo"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(bo_unit.clone())
            .map(
                |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                    (
                        Some(connective),
                        bo_tense_modal,
                        bo,
                        free_modifiers,
                        trailing_unit,
                    )
                },
            );
        let bare_bo_tail = cmavo("bo")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(bo_unit)
            .map(|((bo, free_modifiers), trailing_unit)| {
                (None, None, bo, free_modifiers, trailing_unit)
            });
        atom_unit
            .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
            .map(|(leading_unit, bo_tail)| {
                bo_tail.map_or(
                    leading_unit.clone(),
                    |(bo_connective, bo_tense_modal, bo, free_modifiers, trailing_unit)| {
                        RelationUnitSyntax::Bo {
                            leading_unit: Box::new(leading_unit),
                            bo_connective,
                            bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_unit: Box::new(trailing_unit),
                        }
                    },
                )
            })
    });

    let connected_unit = bo_unit
        .clone()
        .then(
            statement_connective()
                .then(bo_unit)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |leading_unit, (connective, trailing_unit)| {
                    RelationUnitSyntax::Connected {
                        leading_unit: Box::new(leading_unit),
                        connective,
                        trailing_unit: Box::new(trailing_unit),
                    }
                })
        });

    let relation_units = connected_unit
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(relation_from_units);

    let base_relation = relation_units;
    let connected_relation = base_relation
        .clone()
        .then(statement_connective().then(base_relation.clone()).or_not())
        .map(|(leading_relation, connected)| {
            connected.map_or(
                leading_relation.clone(),
                |(connective, trailing_relation)| RelationSyntax::Connected {
                    connective,
                    leading_relation: Box::new(leading_relation),
                    trailing_relation: Box::new(trailing_relation),
                },
            )
        });
    let na_relation = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation)
        .map(
            |((na, free_modifiers), inner_relation)| RelationSyntax::Na {
                na,
                free_modifiers,
                inner_relation: Box::new(inner_relation),
            },
        );
    let co_relation = recursive(|co_relation| {
        connected_relation
            .clone()
            .then(
                cmavo("co")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(co_relation)
                    .or_not(),
            )
            .map(|(leading_relation, co_tail)| {
                co_tail.map_or(
                    leading_relation.clone(),
                    |((co, free_modifiers), trailing_relation)| RelationSyntax::Co {
                        leading_relation: Box::new(leading_relation),
                        co,
                        free_modifiers,
                        trailing_relation: Box::new(trailing_relation),
                    },
                )
            })
    });

    let untagged_relation = choice((na_relation, co_relation)).boxed();
    let tagged_relation =
        tense_modal()
            .then(untagged_relation.clone())
            .map(|(tense_modal, inner_relation)| RelationSyntax::TenseModal {
                tense_modal,
                inner_relation: Box::new(inner_relation),
            });

    choice((tagged_relation, untagged_relation)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_units_inner<'tokens, P, S, T, F>(
    argument: P,
    subsentence: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive(|inner_relation| {
        let me_unit = cmavo("me")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(argument.clone())
            .then(cmavo("me'u").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(cmavo_of("MOI", MOI_WORDS).or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (
                        ((((me, me_free_modifiers), argument), mehu), mehu_free_modifiers),
                        moi_marker,
                    ),
                    moi_free_modifiers,
                )| RelationUnitSyntax::Me {
                    me,
                    me_free_modifiers,
                    argument,
                    mehu,
                    mehu_free_modifiers,
                    moi_marker,
                    moi_free_modifiers,
                },
            );
        let mehoi_unit = single_word_quoted_relation_unit(
            "me'oi",
            source,
            free_modifier.clone(),
            |mehoi, quoted_text, free_modifiers| RelationUnitSyntax::Mehoi {
                mehoi,
                quoted_text,
                free_modifiers,
            },
        );
        let gohoi_unit = single_word_quoted_relation_unit(
            "go'oi",
            source,
            free_modifier.clone(),
            |gohoi, quoted_text, free_modifiers| RelationUnitSyntax::Gohoi {
                gohoi,
                quoted_text,
                free_modifiers,
            },
        );
        let muhoi_unit = delimited_quoted_relation_unit(
            "mu'oi",
            source,
            free_modifier.clone(),
            |muhoi, opening_delimiter, closing_delimiter, quoted_text, free_modifiers| {
                RelationUnitSyntax::Muhoi {
                    muhoi,
                    opening_delimiter,
                    closing_delimiter,
                    quoted_text,
                    free_modifiers,
                }
            },
        );
        let luhei_unit = cmavo("lu'ei")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(text.clone())
            .then(cmavo("li'au").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |((((luhei, luhei_free_modifiers), text), liau), liau_free_modifiers)| {
                    RelationUnitSyntax::Luhei {
                        luhei,
                        luhei_free_modifiers,
                        text,
                        liau,
                        liau_free_modifiers,
                    }
                },
            )
            .boxed();
        let brivla_word_unit = brivla_relation_word()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(word, free_modifiers)| RelationUnitSyntax::Word {
                word,
                free_modifiers,
            });
        let goha_word_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then_ignore(
                choice((
                    cmavo("ra'o").ignored(),
                    cmavo("be").ignored(),
                    free_modifier.clone().ignored(),
                ))
                .rewind()
                .not(),
            )
            .map(|word| RelationUnitSyntax::Word {
                word,
                free_modifiers: Vec::new(),
            });
        let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
        let goha_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then(cmavo("ra'o").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| RelationUnitSyntax::Goha {
                goha,
                raho,
                free_modifiers,
            });
        let goha_raho_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then(cmavo("ra'o"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| RelationUnitSyntax::Goha {
                goha,
                raho: Some(raho),
                free_modifiers,
            });
        let moi_unit = number_or_letter_words()
            .then(cmavo_of("MOI", MOI_WORDS))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((number, moi), free_modifiers)| RelationUnitSyntax::Moi {
                number,
                moi,
                free_modifiers,
            });
        let nuha_unit = cmavo("nu'a")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(math_operator())
            .map(
                |((nuha, free_modifiers), math_operator)| RelationUnitSyntax::Nuha {
                    nuha,
                    free_modifiers,
                    math_operator,
                },
            );
        let xohi_unit = cmavo("xo'i")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(tense_modal())
            .map(|((xohi, free_modifiers), tag)| RelationUnitSyntax::Xohi {
                xohi,
                free_modifiers,
                tag,
            });
        let nu_cmavo = || cmavo_of("NU", NU_WORDS);
        let additional_nu = statement_connective()
            .then(nu_cmavo())
            .map(|(connective, nu)| AdditionalNuSyntax { connective, nu });
        let abstraction_subsentence_unit = nu_cmavo()
            .then(additional_nu.repeated().collect::<Vec<_>>())
            .then(subsentence.clone())
            .then(cmavo("kei").or_not())
            .map(
                |(((nu, additional_nu), subsentence), kei)| RelationUnitSyntax::Abstraction {
                    abstraction: AbstractionSyntax {
                        nu,
                        additional_nu,
                        subsentence: Box::new(subsentence),
                        kei,
                    },
                },
            )
            .boxed();
        let se_abstraction_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(abstraction_subsentence_unit.clone())
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se,
                    free_modifiers,
                    inner_unit: Box::new(inner_unit),
                },
            );
        let ke_unit = cmavo("ke")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_relation.clone())
            .then(cmavo("ke'e").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |((((ke, ke_free_modifiers), relation), kehe), kehe_free_modifiers)| {
                    RelationUnitSyntax::Ke {
                        ke_tense_modal: None,
                        ke,
                        ke_free_modifiers,
                        relation,
                        kehe,
                        kehe_free_modifiers,
                    }
                },
            );
        let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se,
                    free_modifiers,
                    inner_unit: Box::new(inner_unit),
                },
            );
        let nahe_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                se_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(
                |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                    nahe,
                    free_modifiers,
                    inner_unit: Box::new(inner_unit),
                },
            );
        let be_link = be_link_parser(argument.clone(), free_modifier.clone());
        let selbri_relative_clause = cmavo("no'oi")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_relation.clone())
            .then(cmavo("ku'oi").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (((nohoi, leading_free_modifiers), relation), kuhoi),
                    trailing_free_modifiers,
                )| {
                    SelbriRelativeClauseSyntax {
                        nohoi,
                        leading_free_modifiers,
                        relation,
                        kuhoi,
                        trailing_free_modifiers,
                    }
                },
            )
            .boxed();

        let base_unit = choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subsentence_unit.clone(),
            nahe_unit.clone(),
            se_unit.clone(),
            ke_unit.clone(),
            xohi_unit.clone(),
            nuha_unit.clone(),
            moi_unit.clone(),
            word_unit.clone(),
            goha_unit.clone(),
        ))
        .boxed();
        let base_unit_for_cei = choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit,
            abstraction_subsentence_unit,
            nahe_unit.clone(),
            se_unit.clone(),
            ke_unit.clone(),
            xohi_unit,
            nuha_unit.clone(),
            moi_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        ))
        .boxed();
        let linked_unit_from = |base_unit: BoxedParser<'tokens, RelationUnitSyntax>| {
            base_unit
                .then(be_link.clone().or_not())
                .map(|(base, be_link)| {
                    be_link.map_or(base.clone(), |link| {
                        let data!(BeLinkSyntax {
                            be,
                            free_modifiers,
                            fa,
                            fa_free_modifiers,
                            first_argument,
                            bei_links,
                            beho,
                            beho_free_modifiers,
                        }) = link.into_data();

                        RelationUnitSyntax::Be {
                            base: Box::new(base),
                            be,
                            free_modifiers,
                            fa,
                            fa_free_modifiers,
                            first_argument,
                            bei_links,
                            beho,
                            beho_free_modifiers,
                        }
                    })
                })
                .then(
                    selbri_relative_clause
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .map(|(linked_unit, selbri_relative_clauses)| {
                    if selbri_relative_clauses.is_empty() {
                        linked_unit
                    } else {
                        RelationUnitSyntax::SelbriRelativeClause {
                            base: Box::new(linked_unit),
                            selbri_relative_clauses,
                        }
                    }
                })
                .boxed()
        };
        let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
            let data!(BeLinkSyntax {
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
            }) = link.into_data();

            RelationUnitSyntax::PreposedBe {
                be,
                free_modifiers,
                fa,
                fa_free_modifiers,
                first_argument,
                bei_links,
                beho,
                beho_free_modifiers,
                base: Box::new(base),
            }
        });
        let linked_unit = linked_unit_from(base_unit);
        let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
        let cei_unit = linked_unit_for_cei
            .clone()
            .then(
                cmavo("cei")
                    .then(linked_unit_for_cei.clone())
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(base, be_link)| RelationUnitSyntax::Cei {
                base: Box::new(base),
                assignments: be_link
                    .into_iter()
                    .map(|(cei, relation_unit)| CeiAssignmentSyntax {
                        cei,
                        free_modifiers: Vec::new(),
                        relation_unit,
                    })
                    .collect(),
            })
            .boxed();
        let bo_unit = recursive(|bo_unit| {
            let guha_unit = guhek_connective()
                .then(inner_relation.clone())
                .then(gik_connective())
                .then(bo_unit.clone())
                .map(|(((guhek, leading_relation), gik), trailing_unit)| {
                    RelationUnitSyntax::Wrapped {
                        relation: RelationSyntax::Guha {
                            guhek,
                            leading_predicate: Box::new(relation_to_empty_predicate(
                                leading_relation,
                            )),
                            gik,
                            trailing_predicate: Box::new(relation_to_empty_predicate(
                                relation_unit_to_relation(&trailing_unit),
                            )),
                        },
                    }
                });
            let atom_unit = choice((
                guha_unit,
                preposed_unit.clone(),
                cei_unit.clone(),
                linked_unit.clone(),
            ))
            .boxed();
            let connected_bo_tail = statement_connective()
                .then(tense_modal().or_not())
                .then(cmavo("bo"))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit.clone())
                .map(
                    |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                        (
                            Some(connective),
                            bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_unit,
                        )
                    },
                );
            let bare_bo_tail = cmavo("bo")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit)
                .map(|((bo, free_modifiers), trailing_unit)| {
                    (None, None, bo, free_modifiers, trailing_unit)
                });
            atom_unit
                .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
                .map(|(leading_unit, bo_tail)| {
                    bo_tail.map_or(
                        leading_unit.clone(),
                        |(bo_connective, bo_tense_modal, bo, free_modifiers, trailing_unit)| {
                            RelationUnitSyntax::Bo {
                                leading_unit: Box::new(leading_unit),
                                bo_connective,
                                bo_tense_modal,
                                bo,
                                free_modifiers,
                                trailing_unit: Box::new(trailing_unit),
                            }
                        },
                    )
                })
        });
        bo_unit
            .clone()
            .then(
                statement_connective()
                    .then(bo_unit)
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first, continuations)| {
                continuations.into_iter().fold(
                    first,
                    |leading_unit, (connective, trailing_unit)| RelationUnitSyntax::Connected {
                        leading_unit: Box::new(leading_unit),
                        connective,
                        trailing_unit: Box::new(trailing_unit),
                    },
                )
            })
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(relation_from_units)
    })
    .boxed()
}

#[requires(!units.is_empty(), "relation unit sequences must be non-empty")]
#[ensures(true)]
fn relation_from_units(units: Vec<RelationUnitSyntax>) -> RelationSyntax {
    match units.as_slice() {
        [
            RelationUnitSyntax::Word {
                word,
                free_modifiers,
            },
        ] if free_modifiers.is_empty() => RelationSyntax::Base { word: word.clone() },
        [
            RelationUnitSyntax::Goha {
                goha,
                raho: None,
                free_modifiers,
            },
        ] if free_modifiers.is_empty() => RelationSyntax::Base { word: goha.clone() },
        [RelationUnitSyntax::Word { .. } | RelationUnitSyntax::Goha { .. }] => {
            RelationSyntax::Compound { units }
        }
        [
            RelationUnitSyntax::Se {
                se,
                free_modifiers,
                inner_unit,
            },
        ] => RelationSyntax::Se {
            se: se.clone(),
            free_modifiers: free_modifiers.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit.as_ref())),
        },
        [
            RelationUnitSyntax::Ke {
                ke_tense_modal,
                ke,
                ke_free_modifiers,
                relation,
                kehe,
                kehe_free_modifiers,
            },
        ] => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            ke_free_modifiers: ke_free_modifiers.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
            kehe_free_modifiers: kehe_free_modifiers.clone(),
        },
        [RelationUnitSyntax::Abstraction { abstraction }] => RelationSyntax::Abstraction {
            abstraction: abstraction.clone(),
        },
        [
            RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                free_modifiers,
                trailing_unit,
            },
        ] => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
            free_modifiers: free_modifiers.clone(),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        [
            RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            },
        ] => RelationSyntax::Connected {
            connective: connective.clone(),
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        [RelationUnitSyntax::Wrapped { relation }] => relation.clone(),
        _ => RelationSyntax::Compound { units },
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_to_relation(unit: &RelationUnitSyntax) -> RelationSyntax {
    match unit {
        RelationUnitSyntax::Word {
            word,
            free_modifiers,
        } if free_modifiers.is_empty() => RelationSyntax::Base { word: word.clone() },
        RelationUnitSyntax::Goha {
            goha,
            raho: None,
            free_modifiers,
        } if free_modifiers.is_empty() => RelationSyntax::Base { word: goha.clone() },
        RelationUnitSyntax::Se {
            se,
            free_modifiers,
            inner_unit,
        } => RelationSyntax::Se {
            se: se.clone(),
            free_modifiers: free_modifiers.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit)),
        },
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            ke_free_modifiers,
            relation,
            kehe,
            kehe_free_modifiers,
        } => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            ke_free_modifiers: ke_free_modifiers.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
            kehe_free_modifiers: kehe_free_modifiers.clone(),
        },
        RelationUnitSyntax::Abstraction { abstraction } => RelationSyntax::Abstraction {
            abstraction: abstraction.clone(),
        },
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_unit,
        } => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
            free_modifiers: free_modifiers.clone(),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        RelationUnitSyntax::Connected {
            leading_unit,
            connective,
            trailing_unit,
        } => RelationSyntax::Connected {
            connective: connective.clone(),
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        RelationUnitSyntax::Wrapped { relation } => relation.clone(),
        unit => RelationSyntax::Compound {
            units: vec![unit.clone()],
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_to_empty_predicate(relation: RelationSyntax) -> BasicPredicate {
    BasicPredicate {
        leading_terms: Vec::new(),
        cu: None,
        cu_free_modifiers: Vec::new(),
        relation,
        tail_terms: Vec::new(),
        vau: None,
        gek_sentence: None,
        bo_continuation: None,
        ke_continuation: None,
        continuations: Vec::new(),
        free_modifiers: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn fiho_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let word_unit = relation_word().map(|word| RelationUnitSyntax::Word {
        word,
        free_modifiers: Vec::new(),
    });
    let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(word_unit.clone())
        .map(|(se, inner_unit)| RelationUnitSyntax::Se {
            se,
            free_modifiers: Vec::new(),
            inner_unit: Box::new(inner_unit),
        });
    let relation = choice((se_unit, word_unit))
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(relation_from_units);

    cmavo("fi'o")
        .then(relation)
        .then(cmavo("fe'u").or_not())
        .map(|((fiho, relation), fehu)| TenseModalSyntax::Fiho {
            fiho,
            relation: Box::new(relation),
            fehu,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let pu = cmavo_of("PU", &["pu", "ca", "ba"])
        .then(cmavo("nai").or_not())
        .then(cmavo_of("ZI", &["zi", "za", "zu"]).or_not())
        .map(|((pu, nai), distance)| {
            let mut leaves = vec![pu.clone()];
            leaves.extend(nai.clone());
            leaves.extend(distance.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: Some(TimeTenseSyntax {
                    direction: vec![pu],
                    distance,
                    interval: None,
                    nai,
                }),
                space: None,
                nahe: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let zi = cmavo_of("ZI", &["zi", "za", "zu"]).map(|zi| TenseModalSyntax::Composite {
        leaves: vec![zi.clone()],
        time: Some(TimeTenseSyntax {
            direction: Vec::new(),
            distance: Some(zi),
            interval: None,
            nai: None,
        }),
        space: None,
        nahe: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: None,
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let zeha = cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]).map(|zeha| {
        TenseModalSyntax::Composite {
            leaves: vec![zeha.clone()],
            time: Some(TimeTenseSyntax {
                direction: Vec::new(),
                distance: None,
                interval: Some(zeha),
                nai: None,
            }),
            space: None,
            nahe: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let faha = cmavo_of(
        "FAhA",
        &[
            "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a", "ru'u",
            "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a", "zo'i", "ze'o",
        ],
    )
    .then(cmavo("nai").or_not())
    .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
    .map(|((faha, nai), distance)| {
        let mut leaves = vec![faha.clone()];
        leaves.extend(nai);
        leaves.extend(distance.clone());
        TenseModalSyntax::Composite {
            leaves,
            time: None,
            space: Some(SpaceTenseSyntax {
                direction: vec![faha],
                distance: distance.into_iter().collect(),
                interval: Vec::new(),
                dimensions: Vec::new(),
                mohi: None,
                fehe: None,
            }),
            nahe: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let va = cmavo_of("VA", &["vi", "va", "vu"]).map(|va| TenseModalSyntax::Composite {
        leaves: vec![va.clone()],
        time: None,
        space: Some(SpaceTenseSyntax {
            direction: Vec::new(),
            distance: vec![va],
            interval: Vec::new(),
            dimensions: Vec::new(),
            mohi: None,
            fehe: None,
        }),
        nahe: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: None,
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let veha = cmavo_of("VEhA", &["ve'i", "ve'a", "ve'u", "ve'e"])
        .then(cmavo_of("VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"]).or_not())
        .then(
            cmavo_of(
                "FAhA",
                &[
                    "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                    "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                    "zo'i", "ze'o",
                ],
            )
            .then(cmavo("nai").or_not())
            .or_not(),
        )
        .map(|((veha, viha), faha)| {
            let mut leaves = vec![veha.clone()];
            leaves.extend(viha.clone());
            if let Some((faha, nai)) = &faha {
                leaves.push(faha.clone());
                leaves.extend(nai.clone());
            }
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: faha
                        .as_ref()
                        .map_or_else(Vec::new, |(faha, _)| vec![faha.clone()]),
                    distance: Vec::new(),
                    interval: vec![veha],
                    dimensions: viha.into_iter().collect(),
                    mohi: None,
                    fehe: None,
                }),
                nahe: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let viha = cmavo_of("VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"]).map(|viha| {
        TenseModalSyntax::Composite {
            leaves: vec![viha.clone()],
            time: None,
            space: Some(SpaceTenseSyntax {
                direction: Vec::new(),
                distance: Vec::new(),
                interval: Vec::new(),
                dimensions: vec![viha],
                mohi: None,
                fehe: None,
            }),
            nahe: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let numbered_interval = pa_word()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(cmavo_of("ROI", &["roi", "re'u"]))
        .then(cmavo("nai").or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let mut leaves = number.clone();
            leaves.push(roi_or_tahe.clone());
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number,
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let tahe_interval = cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
        .then(cmavo("nai").or_not())
        .map(|(roi_or_tahe, nai)| {
            let mut leaves = vec![roi_or_tahe.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number: Vec::new(),
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let fehe_tahe_interval = cmavo("fe'e")
        .then(cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"]))
        .then(cmavo("nai").or_not())
        .map(|((fehe, roi_or_tahe), nai)| {
            let mut leaves = vec![fehe.clone(), roi_or_tahe.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: Vec::new(),
                    distance: Vec::new(),
                    interval: Vec::new(),
                    dimensions: Vec::new(),
                    mohi: None,
                    fehe: Some(fehe),
                }),
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number: Vec::new(),
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let fehe_numbered_interval = cmavo("fe'e")
        .then(pa_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(cmavo_of("ROI", &["roi", "re'u"]))
        .then(cmavo("nai").or_not())
        .map(|(((fehe, number), roi_or_tahe), nai)| {
            let mut leaves = vec![fehe.clone()];
            leaves.extend(number.clone());
            leaves.push(roi_or_tahe.clone());
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: Vec::new(),
                    distance: Vec::new(),
                    interval: Vec::new(),
                    dimensions: Vec::new(),
                    mohi: None,
                    fehe: Some(fehe),
                }),
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number,
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let fehe_zaho = cmavo("fe'e")
        .then(cmavo_of(
            "ZAhO",
            &[
                "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o",
            ],
        ))
        .then(cmavo("nai").or_not())
        .map(|((fehe, zaho), nai)| {
            let mut leaves = vec![fehe.clone(), zaho.clone()];
            leaves.extend(nai);
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: Vec::new(),
                    distance: Vec::new(),
                    interval: Vec::new(),
                    dimensions: Vec::new(),
                    mohi: None,
                    fehe: Some(fehe),
                }),
                nahe: None,
                interval: None,
                zaho: vec![zaho],
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let mohi = cmavo("mo'i")
        .then(cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        ))
        .then(cmavo("nai").or_not())
        .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
        .map(|(((mohi, faha), nai), distance)| {
            let mut leaves = vec![mohi.clone(), faha.clone()];
            leaves.extend(nai);
            leaves.extend(distance.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: vec![faha],
                    distance: distance.into_iter().collect(),
                    interval: Vec::new(),
                    dimensions: Vec::new(),
                    mohi: Some(mohi),
                    fehe: None,
                }),
                nahe: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let caha = cmavo_of("CAhA", CAHA_WORDS).map(|caha| TenseModalSyntax::Composite {
        leaves: vec![caha.clone()],
        time: None,
        space: None,
        nahe: None,
        interval: None,
        zaho: Vec::new(),
        caha: Some(caha),
        ki: None,
        cuhe: None,
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let zaho = cmavo_of(
        "ZAhO",
        &[
            "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o",
        ],
    )
    .then(cmavo("nai").or_not())
    .map(|(zaho, nai)| {
        let mut leaves = vec![zaho.clone()];
        leaves.extend(nai);
        TenseModalSyntax::Composite {
            leaves,
            time: None,
            space: None,
            nahe: None,
            interval: None,
            zaho: vec![zaho],
            caha: None,
            ki: None,
            cuhe: None,
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let ki = cmavo("ki").map(|ki| TenseModalSyntax::Composite {
        leaves: vec![ki.clone()],
        time: None,
        space: None,
        nahe: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: Some(ki),
        cuhe: None,
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let cuhe = cmavo_of("CUhE", &["cu'e", "nau"]).map(|cuhe| TenseModalSyntax::Composite {
        leaves: vec![cuhe.clone()],
        time: None,
        space: None,
        nahe: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: Some(cuhe),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });

    let bare_atom = choice((
        fehe_tahe_interval,
        fehe_numbered_interval,
        fehe_zaho,
        mohi,
        faha,
        veha,
        viha,
        va,
        numbered_interval,
        tahe_interval,
        pu,
        zi,
        zeha,
        caha,
        zaho,
        ki,
        cuhe,
    ))
    .boxed();
    let atom = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(bare_atom.clone())
        .map(|(nahe, atom)| prefix_tense_modal_nahe(nahe, atom))
        .or(bare_atom)
        .boxed();

    atom.clone()
        .then(
            choice((
                choice((joik_connective(), jek_connective()))
                    .then(atom.clone())
                    .map(|(connective, atom)| {
                        let connective_cmavo = connective.cmavo.clone();
                        let connective_leaves = connective_tense_modal_leaves(connective);
                        (connective_leaves, connective_cmavo, atom)
                    }),
                atom.map(|atom| (Vec::new(), Vec::new(), atom)),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            let mut leaves = first.clone().leaf_words();
            let mut parts = vec![first];
            let mut connectives = Vec::new();
            for (connective_leaves, connective_cmavo, part) in continuations {
                leaves.extend(connective_leaves);
                connectives.extend(connective_cmavo);
                leaves.extend(part.clone().leaf_words());
                parts.push(part);
            }
            match combine_composite_tense_modals(parts) {
                TenseModalSyntax::Composite {
                    time,
                    space,
                    nahe,
                    interval,
                    zaho,
                    caha,
                    ki,
                    cuhe,
                    ..
                } => TenseModalSyntax::Composite {
                    leaves,
                    time,
                    space,
                    nahe,
                    interval,
                    zaho,
                    caha,
                    ki,
                    cuhe,
                    connectives,
                    free_modifiers: Vec::new(),
                },
                other => other,
            }
        })
        .boxed()
}

#[requires(matches!(modal, TenseModalSyntax::Composite { .. }))]
#[ensures(matches!(ret, TenseModalSyntax::Composite { nahe: Some(_), .. }))]
fn prefix_tense_modal_nahe(nahe: WordWithModifiers, modal: TenseModalSyntax) -> TenseModalSyntax {
    let TenseModalSyntax::Composite {
        mut leaves,
        time,
        space,
        nahe: _,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        connectives,
        free_modifiers,
    } = modal
    else {
        unreachable!("prefix_tense_modal_nahe requires a composite tense modal")
    };
    leaves.insert(0, nahe.clone());
    TenseModalSyntax::Composite {
        leaves,
        time,
        space,
        nahe: Some(nahe),
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        connectives,
        free_modifiers,
    }
}

#[requires(!parts.is_empty())]
#[ensures(matches!(ret, TenseModalSyntax::Composite { .. }))]
fn combine_composite_tense_modals(parts: Vec<TenseModalSyntax>) -> TenseModalSyntax {
    let mut leaves = Vec::new();
    let mut time_direction = Vec::new();
    let mut time_distance = None;
    let mut time_interval = None;
    let mut time_nai = None;
    let mut space_direction = Vec::new();
    let mut space_distance = Vec::new();
    let mut space_interval = Vec::new();
    let mut space_dimensions = Vec::new();
    let mut space_mohi = None;
    let mut space_fehe = None;
    let mut nahe = None;
    let mut zaho = Vec::new();
    let mut caha = None;
    let mut ki = None;
    let mut cuhe = None;
    let mut connectives = Vec::new();
    let mut interval = None;
    let mut free_modifiers = Vec::new();

    for part in parts {
        if let TenseModalSyntax::Composite {
            leaves: part_leaves,
            time,
            space,
            nahe: part_nahe,
            interval: part_interval,
            zaho: part_zaho,
            caha: part_caha,
            ki: part_ki,
            cuhe: part_cuhe,
            connectives: part_connectives,
            free_modifiers: part_free_modifiers,
        } = part
        {
            leaves.extend(part_leaves);
            if let Some(time) = time {
                time_direction.extend(time.direction);
                time_distance = time_distance.or(time.distance);
                time_interval = time_interval.or(time.interval);
                time_nai = time_nai.or(time.nai);
            }
            if let Some(space) = space {
                space_direction.extend(space.direction);
                space_distance.extend(space.distance);
                space_interval.extend(space.interval);
                space_dimensions.extend(space.dimensions);
                space_mohi = space_mohi.or(space.mohi);
                space_fehe = space_fehe.or(space.fehe);
            }
            nahe = nahe.or(part_nahe);
            zaho.extend(part_zaho);
            caha = caha.or(part_caha);
            ki = ki.or(part_ki);
            cuhe = cuhe.or(part_cuhe);
            interval = interval.or(part_interval);
            connectives.extend(part_connectives);
            free_modifiers.extend(part_free_modifiers);
        }
    }

    let time = (!time_direction.is_empty() || time_distance.is_some() || time_interval.is_some())
        .then_some(TimeTenseSyntax {
            direction: time_direction,
            distance: time_distance,
            interval: time_interval,
            nai: time_nai,
        });
    let space = (!space_direction.is_empty()
        || !space_distance.is_empty()
        || !space_interval.is_empty()
        || !space_dimensions.is_empty()
        || space_mohi.is_some()
        || space_fehe.is_some())
    .then_some(SpaceTenseSyntax {
        direction: space_direction,
        distance: space_distance,
        interval: space_interval,
        dimensions: space_dimensions,
        mohi: space_mohi,
        fehe: space_fehe,
    });

    TenseModalSyntax::Composite {
        leaves,
        time,
        space,
        nahe,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        connectives,
        free_modifiers,
    }
}

#[requires(true)]
#[ensures(true)]
fn leading_term_tag_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let pu_before_nahe = cmavo_of("PU", &["pu", "ca", "ba"])
        .then(cmavo("nai").or_not())
        .then(
            cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
                .rewind()
                .ignored(),
        )
        .map(|((pu, nai), _)| {
            let mut leaves = vec![pu.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: Some(TimeTenseSyntax {
                    direction: vec![pu],
                    distance: None,
                    interval: None,
                    nai,
                }),
                space: None,
                nahe: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let zaho_property = cmavo_of(
        "ZAhO",
        &[
            "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o",
        ],
    )
    .then(cmavo("nai").or_not())
    .map(|(zaho, nai)| {
        let mut leaves = vec![zaho.clone()];
        leaves.extend(nai);
        TenseModalSyntax::Composite {
            leaves,
            time: None,
            space: None,
            nahe: None,
            interval: None,
            zaho: vec![zaho],
            caha: None,
            ki: None,
            cuhe: None,
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let numbered_interval = pa_word()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(cmavo_of("ROI", &["roi", "re'u"]))
        .then(cmavo("nai").or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let mut leaves = number.clone();
            leaves.push(roi_or_tahe.clone());
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number,
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let tahe_interval = cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
        .then(cmavo("nai").or_not())
        .map(|(roi_or_tahe, nai)| {
            let mut leaves = vec![roi_or_tahe.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                nahe: None,
                interval: Some(IntervalTenseSyntax {
                    number: Vec::new(),
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let property_split_follower = choice((
        cmavo_of("PU", &["pu", "ca", "ba"]).ignored(),
        cmavo_of("ZI", &["zi", "za", "zu"]).ignored(),
        cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]).ignored(),
        cmavo_of("VA", &["vi", "va", "vu"]).ignored(),
        cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        )
        .ignored(),
        cmavo_of("CAhA", CAHA_WORDS).ignored(),
        cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(cmavo_of("CAhA", CAHA_WORDS))
            .ignored(),
        simple_tense_modal().ignored(),
        fiho_tense_modal().ignored(),
    ));
    let leading_interval_property = choice((zaho_property, numbered_interval, tahe_interval))
        .then(property_split_follower.rewind());

    choice((
        pu_before_nahe,
        leading_interval_property.map(|(tense_modal, _)| tense_modal),
        tense_modal(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    #[derive(Clone)]
    #[invariant(true)]
    enum PuTail {
        Distance(WordWithModifiers),
        Caha(WordWithModifiers),
    }

    choice((
        composite_tense_modal(),
        cmavo_of("PU", &["pu", "ca", "ba"])
            .then(
                choice((
                    cmavo_of("ZI", &["zi", "za", "zu"]).map(PuTail::Distance),
                    cmavo_of("CAhA", CAHA_WORDS).map(PuTail::Caha),
                ))
                .or_not(),
            )
            .map(|(pu, tail)| match tail {
                Some(PuTail::Distance(distance)) => TenseModalSyntax::PuDistance {
                    pu,
                    distance,
                    free_modifiers: Vec::new(),
                },
                Some(PuTail::Caha(caha)) => TenseModalSyntax::PuCaha {
                    pu,
                    caha,
                    free_modifiers: Vec::new(),
                },
                None => TenseModalSyntax::Pu {
                    word: pu,
                    free_modifiers: Vec::new(),
                },
            }),
        cmavo_of("VA", &["vi", "va", "vu"]).map(|word| TenseModalSyntax::SpaceDistance {
            word,
            free_modifiers: Vec::new(),
        }),
        cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]).map(|word| {
            TenseModalSyntax::TimeInterval {
                word,
                free_modifiers: Vec::new(),
            }
        }),
        cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        )
        .map(|word| TenseModalSyntax::SpaceDirection {
            word,
            free_modifiers: Vec::new(),
        }),
        cmavo("mo'i")
            .then(cmavo_of(
                "FAhA",
                &[
                    "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                    "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                    "zo'i", "ze'o",
                ],
            ))
            .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
            .map(
                |((mohi, direction), distance)| TenseModalSyntax::SpaceMovement {
                    mohi,
                    direction,
                    distance,
                    free_modifiers: Vec::new(),
                },
            ),
        cmavo_of("CAhA", CAHA_WORDS).map(|word| TenseModalSyntax::Caha {
            word,
            free_modifiers: Vec::new(),
        }),
        fiho_tense_modal(),
        cmavo_of(
            "ZAhO",
            &[
                "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o",
            ],
        )
        .map(|word| TenseModalSyntax::Zaho {
            words: vec![word],
            free_modifiers: Vec::new(),
        }),
        simple_tense_modal(),
        cmavo("ki").map(|ki| TenseModalSyntax::Ki {
            ki,
            free_modifiers: Vec::new(),
        }),
        pa_word()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cmavo_of("ROI", &["roi", "re'u"])
                    .or(cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])),
            )
            .then(cmavo("nai").or_not())
            .map(|((number, roi_or_tahe), nai)| TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
            .then(cmavo("nai").or_not())
            .map(|(roi_or_tahe, nai)| TenseModalSyntax::Interval {
                number: Vec::new(),
                roi_or_tahe,
                nai,
                free_modifiers: Vec::new(),
            }),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn simple_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let simple_atom = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of(
            "BAI",
            &[
                "du'o", "si'u", "zau", "ki'i", "du'i", "cu'u", "tu'i", "ti'u", "di'o", "ji'u",
                "ri'a", "ni'i", "mu'i", "ki'u", "va'u", "koi", "ca'i", "ta'i", "pu'e", "ja'i",
                "kai", "bai", "fi'e", "de'i", "ci'o", "mau", "mu'u", "ri'i", "ra'i", "ka'a",
                "pa'u", "pa'a", "le'a", "ku'u", "tai", "bau", "ma'i", "ci'e", "fau", "po'i", "cau",
                "ma'e", "ci'u", "ra'a", "pu'a", "li'e", "la'u", "ba'i", "ka'i", "sau", "fa'e",
                "be'i", "ti'i", "ja'e", "ga'a", "va'o", "ji'o", "me'a", "do'e", "ji'e", "pi'o",
                "gau", "zu'e", "me'e", "rai",
            ],
        ))
        .then(cmavo("nai").or_not())
        .then(cmavo("ki").or_not())
        .map(|((((nahe, se), bai), nai), ki)| TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki,
            connectives: Vec::new(),
            extra_leaves: Vec::new(),
            free_modifiers: Vec::new(),
        });

    simple_atom
        .clone()
        .then(
            cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"])
                .then(simple_atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |first, (connective, next)| {
                    let TenseModalSyntax::Simple {
                        nahe,
                        se,
                        bai,
                        nai,
                        ki,
                        mut connectives,
                        mut extra_leaves,
                        free_modifiers,
                    } = first
                    else {
                        return first;
                    };
                    connectives.push(connective);
                    extra_leaves.extend(next.words());
                    TenseModalSyntax::Simple {
                        nahe,
                        se,
                        bai,
                        nai,
                        ki,
                        connectives,
                        extra_leaves,
                        free_modifiers,
                    }
                })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn cmavo<'tokens>(text: &'static str) -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("cmavo", move |word| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
fn cmavo_of<'tokens>(
    label: &'static str,
    texts: &'static [&'static str],
) -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching(label, move |word| {
        texts.iter().any(|text| cmavo_text_matches(word, text))
    })
}

#[requires(true)]
#[ensures(true)]
fn le_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of(
        "LE",
        &["lei", "loi", "le'i", "lo'i", "le'e", "lo'e", "lo", "le"],
    )
}

#[requires(true)]
#[ensures(true)]
fn la_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("LA", &["lai", "la'i", "la"])
}

#[requires(true)]
#[ensures(true)]
fn lahe_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of(
        "LAhE",
        &["tu'a", "lu'a", "lu'o", "la'e", "vu'i", "lu'i", "lu'e"],
    )
}

#[requires(true)]
#[ensures(true)]
fn leading_indicator<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    choice((cmavo_of("UI", UI_WORDS), cmavo_of("CAI", CAI_WORDS))).boxed()
}

#[requires(true)]
#[ensures(true)]
fn pa_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("PA", PA_WORDS)
}

#[requires(true)]
#[ensures(true)]
fn na_cmavo<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    cmavo_of("NA", &["na", "ja'a"])
}

#[requires(true)]
#[ensures(true)]
fn koha_argument<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("KOhA argument", is_koha_argument)
}

#[requires(true)]
#[ensures(true)]
fn relation_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("relation word", is_relation_word)
}

#[requires(true)]
#[ensures(true)]
fn brivla_relation_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("BRIVLA", is_brivla_relation_word)
}

#[requires(true)]
#[ensures(true)]
fn cmevla_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("CMEVLA", is_cmevla_word)
}

#[requires(true)]
#[ensures(true)]
fn letter_word<'tokens>() -> BoxedParser<'tokens, WordWithModifiers> {
    token_matching("letter word", is_letter_word)
}

#[requires(true)]
#[ensures(true)]
fn token_matching<'tokens>(
    label: &'static str,
    predicate: impl Fn(&WordWithModifiers) -> bool + Clone + 'tokens,
) -> BoxedParser<'tokens, WordWithModifiers> {
    custom(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        match input.next() {
            Some(word) if predicate(&word) => Ok(word),
            _ => {
                let span = input.span_since(&cursor);
                input.rewind(checkpoint);
                Err(Rich::custom(span, format!("expected {label}")))
            }
        }
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn is_koha_argument(word: &WordWithModifiers) -> bool {
    KOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
fn is_relation_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::WithIndicator { base, .. }) => return is_relation_word(base),
        data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            return word_like_is_relation_word(word_like);
        }
        data!(WordWithModifiers::StandaloneIndicator { .. }) | data!(WordWithModifiers::NotEof) => {
            return false;
        }
        data!(WordWithModifiers::BaseWord { .. }) => {}
    }

    if GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text)) {
        return true;
    }

    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => word_like_is_relation_word(word_like),
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret == (is_relation_word(word) && !GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))))]
fn is_brivla_relation_word(word: &WordWithModifiers) -> bool {
    is_relation_word(word) && !GOHA_WORDS.iter().any(|text| cmavo_text_matches(word, text))
}

#[requires(true)]
#[ensures(true)]
fn word_like_is_relation_word(word_like: &WordLike) -> bool {
    match word_like.as_data() {
        data!(WordLike::Bare { word }) => {
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
fn is_cmevla_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like })
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            word_like_kind(word_like).is_some_and(|kind| kind == WordKind::Cmevla)
        }
        data!(WordWithModifiers::WithIndicator { base, .. }) => is_cmevla_word(base),
        data!(WordWithModifiers::StandaloneIndicator { .. }) | data!(WordWithModifiers::NotEof) => {
            false
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_letter_word(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like })
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => match word_like.as_data() {
            data!(WordLike::Letter { .. }) => true,
            data!(WordLike::Bare { word }) => {
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
        data!(WordWithModifiers::StandaloneIndicator { .. }) | data!(WordWithModifiers::NotEof) => {
            false
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn word_like_kind(word_like: &WordLike) -> Option<WordKind> {
    let data!(WordLike::Bare { word }) = word_like.as_data() else {
        return None;
    };
    Some(word.kind)
}

#[requires(true)]
#[ensures(true)]
fn cmavo_text_matches(word: &WordWithModifiers, expected: &str) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like })
        | data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            word_like_cmavo_text_matches(word_like, expected)
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, .. }) => {
            word_record_text_matches(indicator, expected)
        }
        data!(WordWithModifiers::WithIndicator { base, .. }) => cmavo_text_matches(base, expected),
        data!(WordWithModifiers::NotEof) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_like_cmavo_text_matches(word_like: &WordLike, expected: &str) -> bool {
    match word_like.as_data() {
        data!(WordLike::Bare { word }) => word_record_text_matches(word, expected),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_record_text_matches(word: &jbotci_morphology::Word, expected: &str) -> bool {
    word.kind == WordKind::Cmavo && phonemes_match_syntax_text(&word.phonemes, expected)
}

#[requires(true)]
#[ensures(true)]
fn phonemes_match_syntax_text(actual: &str, expected: &str) -> bool {
    actual == expected
        || actual
            .chars()
            .map(|ch| match ch {
                'ĭ' => 'i',
                'ŭ' => 'u',
                ch => ch,
            })
            .eq(expected.chars())
}

#[requires(true)]
#[ensures(true)]
fn bare_word_kind_and_phonemes(word: &WordWithModifiers) -> Option<(WordKind, &str)> {
    let data!(WordWithModifiers::BaseWord { word_like }) = word.as_data() else {
        return None;
    };
    let data!(WordLike::Bare { word }) = word_like.as_data() else {
        return None;
    };
    Some((word.kind, word.phonemes.as_str()))
}

#[requires(true)]
#[ensures(true)]
fn base_word_from_record(word: Word) -> WordWithModifiers {
    WordWithModifiers::base_word(WordLike::bare(word))
}

#[requires(true)]
#[ensures(true)]
fn source_text(source: Option<&str>, span: &SourceSpan) -> String {
    source
        .and_then(|source| source.get(span.byte_start..span.byte_end))
        .unwrap_or_default()
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn lojban_text_tree(text: TextSyntax) -> SyntaxValue {
    let paragraphs = paragraphs_tree(text.clone());
    node(
        "LojbanText",
        vec![
            field(
                "leadingNai",
                list(text.leading_nai.into_iter().map(word_value).collect()),
            ),
            field(
                "leadingCmevla",
                list(
                    text.leading_cmevla
                        .into_iter()
                        .map(name_word_value)
                        .collect(),
                ),
            ),
            field(
                "leadingIndicators",
                list(
                    text.leading_indicators
                        .into_iter()
                        .map(word_value)
                        .collect(),
                ),
            ),
            field(
                "leadingFreeModifiers",
                list(
                    text.leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "leadingConnective",
                text.leading_connective
                    .map_or_else(nothing, |connective| just(connective_tree(connective))),
            ),
            field("paragraphs", paragraphs),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn paragraphs_tree(text: TextSyntax) -> SyntaxValue {
    list(text.paragraphs.into_iter().map(paragraph_tree).collect())
}

#[requires(true)]
#[ensures(true)]
fn paragraph_tree(paragraph: ParagraphSyntax) -> SyntaxValue {
    node(
        "Paragraph",
        vec![
            field("i", maybe_word(paragraph.i)),
            field(
                "niho",
                list(paragraph.niho.into_iter().map(word_value).collect()),
            ),
            field(
                "freeModifiers",
                list(
                    paragraph
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "statements",
                list(
                    paragraph
                        .statements
                        .into_iter()
                        .map(paragraph_statement_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn paragraph_statement_tree(statement: ParagraphStatementSyntax) -> SyntaxValue {
    node(
        "ParagraphStatement",
        vec![
            field("i", maybe_word(statement.i)),
            field(
                "connective",
                statement
                    .connective
                    .map_or_else(nothing, |connective| just(connective_tree(connective))),
            ),
            field(
                "freeModifiers",
                list(
                    statement
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "statement",
                statement
                    .statement
                    .map_or_else(nothing, |statement| just(statement_tree(statement))),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn free_modifier_tree(free_modifier: FreeModifierSyntax) -> SyntaxValue {
    match free_modifier {
        FreeModifierSyntax::Sei {
            sei,
            leading_free_modifiers,
            terms,
            cu,
            cu_free_modifiers,
            relation,
            sehu,
            sehu_free_modifiers,
        } => node(
            "SeiFree",
            vec![
                field("sei", word_value(sei)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "terms",
                    if terms.is_empty() {
                        nothing()
                    } else {
                        just(list(terms.into_iter().map(term_tree).collect()))
                    },
                ),
                field("cu", maybe_word(cu)),
                field(
                    "cuFreeModifiers",
                    list(
                        cu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field("sehu", maybe_word(sehu)),
                field(
                    "sehuFreeModifiers",
                    list(
                        sehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::To {
            to,
            free_modifiers,
            text,
            toi,
            toi_free_modifiers,
        } => node(
            "ToFree",
            vec![
                field("to", word_value(to)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("text", lojban_text_tree(*text)),
                field("toi", maybe_word(toi)),
                field(
                    "toiFreeModifiers",
                    list(
                        toi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::Xi {
            xi,
            free_modifiers,
            expression,
        } => node(
            "XiFree",
            vec![
                field("xi", word_value(xi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("mathExpression", math_expression_tree(expression)),
            ],
        ),
        FreeModifierSyntax::Mai {
            number,
            mai,
            free_modifiers,
        } => node(
            "MaiFree",
            vec![
                field("number", nonempty_number_words(number)),
                field("mai", word_value(mai)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FreeModifierSyntax::Soi {
            soi,
            free_modifiers,
            leading_argument,
            trailing_argument,
            sehu,
            sehu_free_modifiers,
        } => node(
            "SoiFree",
            vec![
                field("soi", word_value(soi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("leadingArgument", argument_tree(*leading_argument)),
                field(
                    "trailingArgument",
                    trailing_argument
                        .map_or_else(nothing, |argument| just(argument_tree(*argument))),
                ),
                field("sehu", maybe_word(sehu)),
                field(
                    "sehuFreeModifiers",
                    list(
                        sehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::Vocative {
            vocative_markers,
            free_modifiers,
            argument,
            dohu,
            dohu_free_modifiers,
        } => node(
            "VocativeFree",
            vec![
                field(
                    "vocativeMarkers",
                    list(
                        vocative_markers
                            .into_iter()
                            .map(vocative_marker_value)
                            .collect(),
                    ),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "argument",
                    argument.map_or_else(nothing, |argument| just(argument_tree(argument))),
                ),
                field("dohu", maybe_word(dohu)),
                field(
                    "dohuFreeModifiers",
                    list(
                        dohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn statement_tree(statement: StatementSyntax) -> SyntaxValue {
    match statement {
        StatementSyntax::Tuhe {
            tense_modal,
            tuhe,
            tuhe_free_modifiers,
            text,
            tuhu,
            tuhu_free_modifiers,
        } => node(
            "TuheStatement",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("tuhe", word_value(tuhe)),
                field(
                    "tuheFreeModifiers",
                    list(
                        tuhe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("paragraphs", paragraphs_tree(*text)),
                field("tuhu", maybe_word(tuhu)),
                field(
                    "tuhuFreeModifiers",
                    list(
                        tuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            zohu_free_modifiers,
            inner_statement,
        } => node(
            "PrenexStatement",
            vec![
                field(
                    "prenexTerms",
                    list(prenex_terms.into_iter().map(term_tree).collect()),
                ),
                field("zohu", word_value(zohu)),
                field(
                    "zohuFreeModifiers",
                    list(
                        zohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerStatement", statement_tree(*inner_statement)),
            ],
        ),
        StatementSyntax::Predicate(predicate) => node(
            "StatementPredicate",
            vec![field("predicate", predicate_tree(predicate))],
        ),
        StatementSyntax::Connected {
            i,
            connective,
            leading_statement,
            trailing_statement,
        } => node(
            "ConnectedStatement",
            vec![
                field("i", word_value(i)),
                field("connective", connective_tree(connective)),
                field("leadingStatement", statement_tree(*leading_statement)),
                field("trailingStatement", statement_tree(*trailing_statement)),
            ],
        ),
        StatementSyntax::PreIConnected {
            connective,
            i,
            leading_statement,
            trailing_statement,
        } => node(
            "PreIConnectedStatement",
            vec![
                field("connective", connective_tree(connective)),
                field("i", word_value(i)),
                field("leadingStatement", statement_tree(*leading_statement)),
                field("trailingStatement", statement_tree(*trailing_statement)),
            ],
        ),
        StatementSyntax::Iau {
            inner_statement,
            iau,
            iau_free_modifiers,
            reset_terms,
        } => node(
            "IauStatement",
            vec![
                field("innerStatement", statement_tree(*inner_statement)),
                field("iau", word_value(iau)),
                field(
                    "iauFreeModifiers",
                    list(
                        iau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "resetTerms",
                    list(reset_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        StatementSyntax::ExperimentalPredicateContinuation {
            leading_statement,
            continuation,
        } => node(
            "ExperimentalPredicateContinuationStatement",
            vec![
                field("leadingStatement", statement_tree(*leading_statement)),
                field(
                    "continuation",
                    predicate_statement_continuation_tree(continuation),
                ),
            ],
        ),
        StatementSyntax::Fragment(fragment) => node(
            "StatementFragment",
            vec![field("fragment", fragment_tree(fragment))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_continuation_tree(
    continuation: PredicateStatementContinuationSyntax,
) -> SyntaxValue {
    node(
        "PredicateStatementContinuation",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field(
                "marker",
                predicate_statement_continuation_marker_tree(continuation.marker),
            ),
            field(
                "trailingSubsentence",
                subsentence_tree(continuation.trailing_subsentence),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_continuation_marker_tree(
    marker: PredicateStatementContinuationMarkerSyntax,
) -> SyntaxValue {
    match marker {
        PredicateStatementContinuationMarkerSyntax::Bo { bo, free_modifiers } => node(
            "PredicateStatementBo",
            vec![
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        PredicateStatementContinuationMarkerSyntax::Ke {
            ke,
            ke_free_modifiers,
            kehe,
            kehe_free_modifiers,
        } => node(
            "PredicateStatementKe",
            vec![
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn fragment_tree(fragment: FragmentSyntax) -> SyntaxValue {
    match fragment {
        FragmentSyntax::Argument { argument } => node(
            "ArgumentFragment",
            vec![field("argument", argument_tree(argument))],
        ),
        FragmentSyntax::Ek {
            connective,
            free_modifiers,
        } => node(
            "EkFragment",
            vec![
                field("connective", connective_tree(connective)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Gihek {
            connective,
            free_modifiers,
        } => node(
            "GihekFragment",
            vec![
                field("connective", connective_tree(connective)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Other {
            words,
            free_modifiers,
        } => node(
            "OtherFragment",
            vec![
                field(
                    "otherWords",
                    list(words.into_iter().map(word_value).collect()),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Vocative {
            vocative_markers,
            free_modifiers,
            vocative_argument,
            dohu,
            dohu_free_modifiers,
        } => node(
            "VocativeFragment",
            vec![
                field(
                    "vocativeMarkers",
                    list(
                        vocative_markers
                            .into_iter()
                            .map(vocative_marker_value)
                            .collect(),
                    ),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "vocativeArgument",
                    vocative_argument
                        .map_or_else(nothing, |argument| just(argument_tree(argument))),
                ),
                field("dohu", maybe_word(dohu)),
                field(
                    "dohuFreeModifiers",
                    list(
                        dohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::Ijek { i, connective } => node(
            "IjekFragment",
            vec![
                field("i", word_value(i)),
                field("connective", connective_tree(connective)),
            ],
        ),
        FragmentSyntax::Prenex {
            terms,
            zohu,
            zohu_free_modifiers,
        } => node(
            "PrenexFragment",
            vec![
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("zohu", word_value(zohu)),
                field(
                    "zohuFreeModifiers",
                    list(
                        zohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::BeLink {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
        } => node(
            "BeLinkFragment",
            vec![
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::BeiLink { bei_only_links } => node(
            "BeiLinkFragment",
            vec![field(
                "beiOnlyLinks",
                list(bei_only_links.into_iter().map(bei_link_tree).collect()),
            )],
        ),
        FragmentSyntax::RelativeClause { relative_clauses } => node(
            "RelativeClauseFragment",
            vec![field(
                "relativeClauses",
                list(
                    relative_clauses
                        .into_iter()
                        .map(relative_clause_tree)
                        .collect(),
                ),
            )],
        ),
        FragmentSyntax::MathExpression { math_expression } => node(
            "MathExpressionFragment",
            vec![field(
                "mathExpression",
                math_expression_tree(math_expression),
            )],
        ),
        FragmentSyntax::Term {
            terms,
            vau,
            vau_free_modifiers,
        } => node(
            "TermFragment",
            vec![
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("vau", maybe_word(vau)),
                field(
                    "vauFreeModifiers",
                    list(
                        vau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::Relation { relation } => node(
            "RelationFragment",
            vec![field("relation", relation_tree(relation))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause_tree(relative_clause: GoiRelativeClauseSyntax) -> SyntaxValue {
    node(
        "GoiRelativeClause",
        vec![
            field("goi", word_value(relative_clause.goi)),
            field(
                "leadingFreeModifiers",
                list(
                    relative_clause
                        .leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("argument", argument_tree(relative_clause.argument)),
            field("gehu", maybe_word(relative_clause.gehu)),
            field(
                "trailingFreeModifiers",
                list(
                    relative_clause
                        .trailing_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tree(predicate: BasicPredicate) -> SyntaxValue {
    let predicate_tail = predicate_tail_tree(predicate.clone());
    node(
        "Predicate",
        vec![
            field(
                "leadingTerms",
                list(predicate.leading_terms.into_iter().map(term_tree).collect()),
            ),
            field("cu", maybe_word(predicate.cu)),
            field(
                "cuFreeModifiers",
                list(
                    predicate
                        .cu_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("predicateTail", predicate_tail),
            field(
                "freeModifiers",
                list(
                    predicate
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_tree(predicate: BasicPredicate) -> SyntaxValue {
    let ke_continuation = predicate.ke_continuation.clone();
    node(
        "PredicateTail",
        vec![
            field(
                "first",
                node(
                    "PredicateTail1",
                    vec![
                        field("first", predicate_tail2_tree(predicate.clone())),
                        field(
                            "continuations",
                            list(
                                predicate
                                    .continuations
                                    .into_iter()
                                    .map(predicate_tail_continuation_tree)
                                    .collect(),
                            ),
                        ),
                    ],
                ),
            ),
            field(
                "keContinuation",
                ke_continuation.map_or_else(nothing, |ke_continuation| {
                    just(predicate_tail_ke_continuation_tree(ke_continuation))
                }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2_tree(predicate: BasicPredicate) -> SyntaxValue {
    let tail3 = predicate.gek_sentence.map_or_else(
        || {
            node(
                "RelationPredicateTail3",
                vec![
                    field("relation", relation_tree(predicate.relation)),
                    field(
                        "terms",
                        list(predicate.tail_terms.into_iter().map(term_tree).collect()),
                    ),
                    field("vau", maybe_word(predicate.vau)),
                    field("freeModifiers", nil()),
                ],
            )
        },
        |gek_sentence| {
            node(
                "GekSentencePredicateTail3",
                vec![field("gekSentence", gek_sentence_tree(gek_sentence))],
            )
        },
    );
    node(
        "PredicateTail2",
        vec![
            field("first", tail3),
            field(
                "boContinuation",
                predicate
                    .bo_continuation
                    .map_or_else(nothing, |bo_continuation| {
                        just(predicate_tail_bo_continuation_tree(bo_continuation))
                    }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_bo_continuation_tree(
    continuation: PredicateTailBoContinuationSyntax,
) -> SyntaxValue {
    node(
        "BoPredicateTail",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field("bo", word_value(continuation.bo)),
            field("freeModifiers", nil()),
            field("cu", nothing()),
            field("cuFreeModifiers", nil()),
            field(
                "predicateTail",
                predicate_tail2_tree(*continuation.predicate_tail),
            ),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.vau)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_ke_continuation_tree(
    continuation: PredicateTailKeContinuationSyntax,
) -> SyntaxValue {
    node(
        "KePredicateTail",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field("ke", word_value(continuation.ke)),
            field("keFreeModifiers", nil()),
            field(
                "predicateTail",
                predicate_tail_tree(*continuation.predicate_tail),
            ),
            field("kehe", maybe_word(continuation.kehe)),
            field("keheFreeModifiers", nil()),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.vau)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn gek_sentence_tree(gek_sentence: GekSentenceSyntax) -> SyntaxValue {
    match gek_sentence {
        GekSentenceSyntax::Pair {
            gek,
            first,
            gik,
            second,
            tail_terms,
            vau,
        } => node(
            "GekSentencePair",
            vec![
                field("gek", connective_tree(gek)),
                field("first", subsentence_tree(*first)),
                field("gik", connective_tree(gik)),
                field("second", subsentence_tree(*second)),
                field(
                    "tailTerms",
                    list(tail_terms.into_iter().map(term_tree).collect()),
                ),
                field("vau", maybe_word(vau)),
                field("freeModifiers", nil()),
            ],
        ),
        GekSentenceSyntax::Ke {
            tense_modal,
            ke,
            inner,
            kehe,
        } => node(
            "KeGekSentence",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field("keFreeModifiers", nil()),
                field("inner", gek_sentence_tree(*inner)),
                field("kehe", maybe_word(kehe)),
                field("keheFreeModifiers", nil()),
            ],
        ),
        GekSentenceSyntax::Na { na, inner } => node(
            "NaGekSentence",
            vec![
                field("na", word_value(na)),
                field("freeModifiers", nil()),
                field("inner", gek_sentence_tree(*inner)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_continuation_tree(continuation: PredicateTailContinuationSyntax) -> SyntaxValue {
    node(
        "PredicateTailContinuation",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field("tenseModal", nothing()),
            field("cu", nothing()),
            field("cuFreeModifiers", nil()),
            field(
                "predicateTail",
                node(
                    "PredicateTail2",
                    vec![
                        field(
                            "first",
                            node(
                                "RelationPredicateTail3",
                                vec![
                                    field("relation", relation_tree(continuation.relation)),
                                    field(
                                        "terms",
                                        list(
                                            continuation.terms.into_iter().map(term_tree).collect(),
                                        ),
                                    ),
                                    field("vau", maybe_word(continuation.vau)),
                                    field("freeModifiers", nil()),
                                ],
                            ),
                        ),
                        field(
                            "boContinuation",
                            continuation
                                .bo_continuation
                                .map_or_else(nothing, |bo_continuation| {
                                    just(predicate_tail_bo_continuation_tree(bo_continuation))
                                }),
                        ),
                    ],
                ),
            ),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.tail_vau)),
            field("freeModifiers", nil()),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn term_tree(term: TermSyntax) -> SyntaxValue {
    match term {
        TermSyntax::NuhiTermset {
            nuhi,
            nuhi_free_modifiers,
            termset,
            nuhu,
            nuhu_free_modifiers,
        } => node(
            "NuhiTermset",
            vec![
                field("nuhi", word_value(nuhi)),
                field(
                    "nuhiFreeModifiers",
                    list(
                        nuhi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "termset",
                    list(termset.into_iter().map(term_tree).collect()),
                ),
                field("nuhu", maybe_word(nuhu)),
                field(
                    "nuhuFreeModifiers",
                    list(
                        nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
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
        } => node(
            "GekNuhiTermset",
            vec![
                field("mNuhi", maybe_word(m_nuhi)),
                field(
                    "nuhiFreeModifiers",
                    list(
                        nuhi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("gek", connective_tree(gek)),
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("nuhu", maybe_word(nuhu)),
                field(
                    "nuhuFreeModifiers",
                    list(
                        nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("gik", connective_tree(gik)),
                field(
                    "gikTerms",
                    list(gik_terms.into_iter().map(term_tree).collect()),
                ),
                field("gikNuhu", maybe_word(gik_nuhu)),
                field(
                    "gikNuhuFreeModifiers",
                    list(
                        gik_nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::Cehe {
            leading_terms,
            cehe,
            free_modifiers,
            trailing_terms,
        } => node(
            "CeheTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("cehe", word_value(cehe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::Pehe {
            leading_terms,
            pehe,
            free_modifiers,
            connective,
            trailing_terms,
        } => node(
            "PeheTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("pehe", word_value(pehe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("connective", connective_tree(connective)),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::Argument(argument) => node(
            "ArgumentTerm",
            vec![field("argument", argument_tree(argument))],
        ),
        TermSyntax::Fa {
            fa,
            free_modifiers,
            argument,
            ku,
            ku_free_modifiers,
        } => node(
            "FaTerm",
            vec![
                field("fa", word_value(fa)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("argument", argument_tree(argument)),
                field("ku", maybe_word(ku)),
                field(
                    "kuFreeModifiers",
                    list(
                        ku_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::NaKu {
            na,
            na_ku,
            free_modifiers,
        } => node(
            "NaKuTerm",
            vec![
                field("na", word_value(na)),
                field("naKu", word_value(na_ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        TermSyntax::BareNa { na, free_modifiers } => node(
            "BareNaTerm",
            vec![
                field("na", word_value(na)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        TermSyntax::NoihaAdverbial {
            noiha,
            leading_free_modifiers,
            tail_elements,
            relation,
            relative_clauses,
            fehu,
            trailing_free_modifiers,
        } => node(
            "NoihaAdverbialTerm",
            vec![
                field("noiha", word_value(noiha)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "tailElements",
                    list(
                        tail_elements
                            .into_iter()
                            .map(argument_tail_element_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relation",
                    relation.map_or_else(nothing, |relation| just(relation_tree(relation))),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("fehu", maybe_word(fehu)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::PoihaBrigahi {
            poiha,
            leading_free_modifiers,
            tail_elements,
            relation,
            relative_clauses,
            brigahi_ku,
            trailing_free_modifiers,
        } => node(
            "PoihaBrigahiTerm",
            vec![
                field("poiha", word_value(poiha)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "tailElements",
                    list(
                        tail_elements
                            .into_iter()
                            .map(argument_tail_element_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relation",
                    relation.map_or_else(nothing, |relation| just(relation_tree(relation))),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("brigahiKu", word_value(brigahi_ku)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::FihoiAdverbial {
            fihoi,
            leading_free_modifiers,
            subsentence,
            fihau,
            trailing_free_modifiers,
        } => node(
            "FihoiAdverbialTerm",
            vec![
                field("fihoi", word_value(fihoi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("fihau", maybe_word(fihau)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::SoiAdverbial {
            soi,
            leading_free_modifiers,
            subsentence,
            sehu,
            trailing_free_modifiers,
        } => node(
            "SoiAdverbialTerm",
            vec![
                field("soi", word_value(soi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("sehu", maybe_word(sehu)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::Tagged {
            tense_modal,
            free_modifiers,
            argument,
        } => node(
            "TaggedTerm",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("argument", argument_tree(argument)),
            ],
        ),
        TermSyntax::Connected {
            leading_terms,
            connective,
            trailing_terms,
        } => node(
            "ConnectedTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("connective", connective_tree(connective)),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::BoConnected {
            leading_terms,
            bo_connective,
            tense_modal,
            bo,
            free_modifiers,
            trailing_term,
        } => node(
            "BoConnectedTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingTerm", term_tree(*trailing_term)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn term_wrapper_kind_tree(kind: TermWrapperKindSyntax) -> SyntaxValue {
    match kind {
        TermWrapperKindSyntax::Lahe => node("LaheTermWrapper", Vec::new()),
        TermWrapperKindSyntax::NaheBo => node("NaheBoTermWrapper", Vec::new()),
        TermWrapperKindSyntax::Nahe => node("NaheTermWrapper", Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_tree(argument: ArgumentSyntax) -> SyntaxValue {
    match argument {
        ArgumentSyntax::Quote { quote } => node(
            "QuoteArgument",
            vec![
                field("quote", quote_tree(quote)),
                field("freeModifiers", nil()),
            ],
        ),
        ArgumentSyntax::MathExpression {
            li,
            li_free_modifiers,
            expression,
            loho,
            loho_free_modifiers,
        } => node(
            "MathExpressionArgument",
            vec![
                field("li", word_value(li)),
                field(
                    "liFreeModifiers",
                    list(
                        li_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("mathExpression", math_expression_tree(expression)),
                field("loho", maybe_word(loho)),
                field(
                    "lohoFreeModifiers",
                    list(
                        loho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Letter {
            letter,
            boi,
            boi_free_modifiers,
        } => node(
            "LetterArgument",
            vec![
                field("letter", nonempty_letter_words(letter)),
                field("boi", maybe_word(boi)),
                field(
                    "boiFreeModifiers",
                    list(
                        boi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Quantified {
            quantifier,
            inner_argument,
        } => node(
            "QuantifiedArgument",
            vec![
                field("quantifier", quantifier_expression_tree(quantifier)),
                field("innerArgument", argument_tree(*inner_argument)),
            ],
        ),
        ArgumentSyntax::RelativeClause {
            base_argument,
            vuho,
            vuho_free_modifiers,
            relative_clauses,
        } => node(
            "RelativeClauseArgument",
            vec![
                field("baseArgument", argument_tree(*base_argument)),
                field("vuho", maybe_word(vuho)),
                field(
                    "vuhoFreeModifiers",
                    list(
                        vuho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Vuho {
            base_argument,
            vuho_marker,
            vuho_free_modifiers,
            relative_clauses,
            connected_argument,
        } => node(
            "VuhoArgument",
            vec![
                field("baseArgument", argument_tree(*base_argument)),
                field("vuhoMarker", word_value(vuho_marker)),
                field(
                    "vuhoFreeModifiers",
                    list(
                        vuho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field(
                    "connectedArgument",
                    connected_argument.map_or_else(nothing, |connected_argument| {
                        just(node(
                            "(,)",
                            vec![
                                unnamed_field(connective_tree(connected_argument.connective)),
                                unnamed_field(argument_tree(*connected_argument.argument)),
                            ],
                        ))
                    }),
                ),
            ],
        ),
        ArgumentSyntax::BridiDescription {
            lohoi,
            lohoi_free_modifiers,
            subsentence,
            kuhau,
            kuhau_free_modifiers,
        } => node(
            "BridiDescriptionArgument",
            vec![
                field("lohoi", word_value(lohoi)),
                field(
                    "lohoiFreeModifiers",
                    list(
                        lohoi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("kuhau", maybe_word(kuhau)),
                field(
                    "kuhauFreeModifiers",
                    list(
                        kuhau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::NaKu {
            na,
            ku,
            free_modifiers,
        } => node(
            "NaKuArgument",
            vec![
                field("na", word_value(na)),
                field("ku", word_value(ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Tagged {
            tag_words,
            tag_tense_modal,
            tag_fa,
            free_modifiers,
            inner_argument,
        } => node(
            "TaggedArgument",
            vec![
                field(
                    "tagWords",
                    list(tag_words.into_iter().map(word_value).collect()),
                ),
                field(
                    "tagTenseModal",
                    tag_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("tagFa", maybe_word(tag_fa)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
            ],
        ),
        ArgumentSyntax::NaheBo {
            nahe,
            bo,
            free_modifiers,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "NaheBoArgument",
            vec![
                field("nahe", word_value(nahe)),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Nahe {
            nahe,
            free_modifiers,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "NaheArgument",
            vec![
                field("nahe", word_value(nahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::TermWrapped {
            term_wrapper_kind,
            wrapper,
            wrapper_bo,
            free_modifiers,
            inner_term,
            luhu,
            luhu_free_modifiers,
        } => node(
            "TermWrappedArgument",
            vec![
                field("termWrapperKind", term_wrapper_kind_tree(term_wrapper_kind)),
                field("wrapper", word_value(wrapper)),
                field("wrapperBo", maybe_word(wrapper_bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerTerm", term_tree(*inner_term)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Koha {
            koha,
            free_modifiers,
        } => node(
            "KohaArgument",
            vec![
                field("koha", word_value(koha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Zohe {
            tag_words,
            maybe_ku,
            free_modifiers,
        } => node(
            "ZoheArgument",
            vec![
                field(
                    "tagWords",
                    list(tag_words.into_iter().map(word_value).collect()),
                ),
                field("maybeKu", maybe_word(maybe_ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Lahe {
            lahe,
            free_modifiers,
            relative_clauses,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "LaheArgument",
            vec![
                field("lahe", word_value(lahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "laheRelativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Connected {
            leading_argument,
            connective,
            trailing_argument,
        } => node(
            "ConnectedArgument",
            vec![
                field("leadingArgument", argument_tree(*leading_argument)),
                field("connective", connective_tree(connective)),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Ke {
            ke,
            ke_free_modifiers,
            inner_argument,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeArgument",
            vec![
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Bo {
            leading_argument,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_argument,
        } => node(
            "BoArgument",
            vec![
                field("leadingArgument", argument_tree(*leading_argument)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Gek {
            gek,
            leading_argument,
            gik,
            trailing_argument,
        } => node(
            "GekArgument",
            vec![
                field("gek", connective_tree(gek)),
                field("leadingArgument", argument_tree(*leading_argument)),
                field("gik", connective_tree(gik)),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Descriptor { descriptor } => node(
            "DescriptorArgument",
            vec![field("descriptor", descriptor_tree(descriptor))],
        ),
        ArgumentSyntax::Name {
            la,
            la_free_modifiers,
            names,
            name_free_modifiers,
        } => node(
            "NameArgument",
            vec![
                field("la", gadri_word_value(la)),
                field(
                    "laFreeModifiers",
                    list(
                        la_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("names", nonempty_name_words(names)),
                field(
                    "nameFreeModifiers",
                    list(
                        name_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Cmevla {
            cmevla,
            free_modifiers,
        } => node(
            "CmevlaArgument",
            vec![
                field("cmevla", nonempty_name_words(cmevla)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::RelationVocative {
            leading_relative_clauses,
            relation,
            trailing_relative_clauses,
        } => node(
            "RelationVocativeArgument",
            vec![
                field(
                    "leadingRelativeClauses",
                    list(
                        leading_relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field(
                    "trailingRelativeClauses",
                    list(
                        trailing_relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn subsentence_tree(subsentence: SubsentenceSyntax) -> SyntaxValue {
    match subsentence {
        SubsentenceSyntax::Plain(predicate) => node(
            "PlainSubsentence",
            vec![unnamed_field(predicate_tree(predicate))],
        ),
        SubsentenceSyntax::Prenex {
            prenex_terms,
            zohu,
            zohu_free_modifiers,
            inner_subsentence,
        } => node(
            "PrenexSubsentence",
            vec![
                unnamed_field(node(
                    "Prenex",
                    vec![
                        field(
                            "terms",
                            list(prenex_terms.into_iter().map(term_tree).collect()),
                        ),
                        field("zohu", word_value(zohu)),
                        field(
                            "zohuFreeModifiers",
                            list(
                                zohu_free_modifiers
                                    .into_iter()
                                    .map(free_modifier_tree)
                                    .collect(),
                            ),
                        ),
                    ],
                )),
                unnamed_field(subsentence_tree(*inner_subsentence)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_tree(relative_clause: RelativeClauseSyntax) -> SyntaxValue {
    match relative_clause {
        RelativeClauseSyntax::Goi(relative_clause) => goi_relative_clause_tree(relative_clause),
        RelativeClauseSyntax::Noi {
            noi,
            leading_free_modifiers,
            subsentence,
            kuho,
            trailing_free_modifiers,
        } => node(
            "NoiRelativeClause",
            vec![
                field("noi", word_value(noi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(subsentence)),
                field("kuho", maybe_word(kuho)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelativeClauseSyntax::Poi {
            poi,
            leading_free_modifiers,
            subsentence,
            kuho,
            trailing_free_modifiers,
        } => node(
            "PoiRelativeClause",
            vec![
                field("poi", word_value(poi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(subsentence)),
                field("kuho", maybe_word(kuho)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelativeClauseSyntax::Zihe {
            zihe,
            free_modifiers,
            inner,
        } => node(
            "ZiheRelativeClause",
            vec![
                field("zihe", word_value(zihe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("inner", relative_clause_tree(*inner)),
            ],
        ),
        RelativeClauseSyntax::Connected { connective, inner } => node(
            "ConnectedRelativeClause",
            vec![
                field("connective", connective_tree(connective)),
                field("inner", relative_clause_tree(*inner)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_relative_clause_tree(relative_clause: SelbriRelativeClauseSyntax) -> SyntaxValue {
    node(
        "SelbriRelativeClause",
        vec![
            field("nohoi", word_value(relative_clause.nohoi)),
            field(
                "leadingFreeModifiers",
                list(
                    relative_clause
                        .leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("relation", relation_tree(relative_clause.relation)),
            field("kuhoi", maybe_word(relative_clause.kuhoi)),
            field(
                "trailingFreeModifiers",
                list(
                    relative_clause
                        .trailing_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn quote_tree(quote: QuoteSyntax) -> SyntaxValue {
    match quote {
        QuoteSyntax::Lu {
            lu,
            free_modifiers,
            text,
            lihu,
            lihu_free_modifiers,
        } => node(
            "LuQuote",
            vec![
                field("lu", word_value(lu)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("text", lojban_text_tree(text)),
                field("lihu", maybe_word(lihu)),
                field(
                    "lihuFreeModifiers",
                    list(
                        lihu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        QuoteSyntax::Zo {
            zo,
            word,
            free_modifiers,
        } => node(
            "ZoQuote",
            vec![
                field("zo", word_value(zo)),
                field("word", word_value(word)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::ZohOi {
            zohoi,
            quoted_text,
            free_modifiers,
        } => node(
            "ZohOiQuote",
            vec![
                field("zohoi", word_value(zohoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "ZoiQuote",
            vec![
                field("zoi", word_value(zoi)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Laho {
            laho,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "LahoQuote",
            vec![
                field("laho", word_value(laho)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            lehu,
            lehu_free_modifiers,
        } => node(
            "LohuQuote",
            vec![
                field("lohu", word_value(lohu)),
                field(
                    "quotedWords",
                    list(quoted_words.into_iter().map(word_value).collect()),
                ),
                field("lehu", word_value(lehu)),
                field(
                    "lehuFreeModifiers",
                    list(
                        lehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn descriptor_tree(descriptor: DescriptorSyntax) -> SyntaxValue {
    node(
        "Descriptor",
        vec![
            field(
                "descriptor",
                descriptor
                    .descriptor
                    .map_or_else(nothing, |descriptor| just(word_value(descriptor))),
            ),
            field("descriptorFreeModifiers", nil()),
            field(
                "outerQuantifier",
                descriptor
                    .outer_quantifier
                    .map_or_else(nothing, |quantifier| just(quantifier_tree(quantifier))),
            ),
            field(
                "tailElements",
                list(
                    descriptor
                        .tail_elements
                        .into_iter()
                        .map(argument_tail_element_tree)
                        .collect(),
                ),
            ),
            field(
                "relation",
                descriptor
                    .relation
                    .map_or_else(nothing, |relation| just(relation_tree(relation))),
            ),
            field(
                "relativeClauses",
                list(
                    descriptor
                        .relative_clauses
                        .into_iter()
                        .map(relative_clause_tree)
                        .collect(),
                ),
            ),
            field("ku", maybe_word(descriptor.ku)),
            field("kuFreeModifiers", nil()),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn connective_tree(connective: ConnectiveSyntax) -> SyntaxValue {
    let kind = match connective.kind {
        ConnectiveKind::Afterthought => "AfterthoughtConnective",
        ConnectiveKind::Relation => "RelationConnective",
        ConnectiveKind::PredicateTail => "PredicateTailConnective",
        ConnectiveKind::Forethought => "ForethoughtConnective",
        ConnectiveKind::NonLogical => "NonLogicalConnective",
        ConnectiveKind::Interval => "IntervalConnective",
    };

    node(
        "Connective",
        vec![
            field("kind", node(kind, Vec::new())),
            field("se", maybe_word(connective.se)),
            field("nahe", maybe_word(connective.nahe)),
            field("na", maybe_word(connective.na)),
            field(
                "cmavo",
                list(connective.cmavo.into_iter().map(word_value).collect()),
            ),
            field("nai", maybe_word(connective.nai)),
            field(
                "freeModifiers",
                list(
                    connective
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_element_tree(element: ArgumentTailElementSyntax) -> SyntaxValue {
    match element {
        ArgumentTailElementSyntax::Argument(argument) => node(
            "ArgumentTailArgument",
            vec![unnamed_field(argument_tree(*argument))],
        ),
        ArgumentTailElementSyntax::RelativeClauses(relative_clauses) => node(
            "ArgumentTailRelativeClauses",
            vec![unnamed_field(list(
                relative_clauses
                    .into_iter()
                    .map(relative_clause_tree)
                    .collect(),
            ))],
        ),
        ArgumentTailElementSyntax::Quantifier(quantifier) => node(
            "ArgumentTailQuantifier",
            vec![unnamed_field(quantifier_tree(quantifier))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_tree(quantifier: QuantifierSyntax) -> SyntaxValue {
    match quantifier {
        QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers,
        } => node(
            "NumberQuantifier",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field(
                    "boiFreeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuantifierSyntax::Vei {
            vei,
            math_expression,
            veho,
        } => node(
            "VeiQuantifier",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("mathExpression", math_expression_tree(*math_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_expression_tree(quantifier: QuantifierSyntax) -> SyntaxValue {
    match quantifier {
        QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers,
        } => node(
            "NumberExpression",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuantifierSyntax::Vei {
            vei,
            math_expression,
            veho,
        } => node(
            "VeiExpression",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*math_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn math_expression_tree(expression: MathExpressionSyntax) -> SyntaxValue {
    match expression {
        MathExpressionSyntax::Number(quantifier) => quantifier_expression_tree(quantifier),
        MathExpressionSyntax::Letter { letter, boi } => node(
            "LetterExpression",
            vec![
                field("letter", nonempty_letter_words(letter)),
                field("boi", maybe_word(boi)),
                field("freeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Vei {
            vei,
            inner_expression,
            veho,
        } => node(
            "VeiExpression",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*inner_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Gek {
            gek,
            left_expression,
            gik,
            right_expression,
        } => node(
            "GekExpression",
            vec![
                field("gek", connective_tree(gek)),
                field("leftExpression", math_expression_tree(*left_expression)),
                field("gik", connective_tree(gik)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Forethought {
            peho,
            operator,
            operands,
            kuhe,
        } => node(
            "ForethoughtExpression",
            vec![
                field("peho", maybe_word(peho)),
                field("freeModifiers", nil()),
                field("operator", math_operator_tree(operator)),
                field(
                    "operands",
                    list(operands.into_iter().map(math_expression_tree).collect()),
                ),
                field("kuhe", maybe_word(kuhe)),
                field("kuheFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::ReversePolish {
            fuha,
            free_modifiers,
            operands,
            operators,
        } => node(
            "ReversePolishExpression",
            vec![
                field("fuha", word_value(fuha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "operands",
                    list(operands.into_iter().map(math_expression_tree).collect()),
                ),
                field(
                    "operators",
                    list(operators.into_iter().map(math_operator_tree).collect()),
                ),
            ],
        ),
        MathExpressionSyntax::Nihe {
            nihe,
            relation,
            tehu,
        } => node(
            "NiheExpression",
            vec![
                field("nihe", word_value(nihe)),
                field("freeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Mohe {
            mohe,
            argument,
            tehu,
        } => node(
            "MoheExpression",
            vec![
                field("mohe", word_value(mohe)),
                field("freeModifiers", nil()),
                field("argument", argument_tree(*argument)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Johi {
            johi,
            free_modifiers,
            expressions,
            tehu,
            tehu_free_modifiers,
        } => node(
            "JohiExpression",
            vec![
                field("johi", word_value(johi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("expressions", nonempty_math_expressions(expressions)),
                field("tehu", maybe_word(tehu)),
                field(
                    "tehuFreeModifiers",
                    list(
                        tehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        MathExpressionSyntax::Lahe {
            markers,
            inner_expression,
            luhu,
        } => node(
            "LaheExpression",
            vec![
                field(
                    "markers",
                    list(markers.into_iter().map(word_value).collect()),
                ),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*inner_expression)),
                field("luhu", maybe_word(luhu)),
                field("luhuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Connected {
            left_expression,
            connective,
            right_expression,
        } => node(
            "ConnectedExpression",
            vec![
                field("leftExpression", math_expression_tree(*left_expression)),
                field("connective", connective_tree(connective)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Binary {
            operator,
            left_expression,
            right_expression,
        } => node(
            "BinaryExpression",
            vec![
                field("operator", math_operator_tree(operator)),
                field("leftExpression", math_expression_tree(*left_expression)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Bihe {
            left_expression,
            bihe,
            operator,
            right_expression,
        } => node(
            "BiheExpression",
            vec![
                field("leftExpression", math_expression_tree(*left_expression)),
                field("bihe", word_value(bihe)),
                field("freeModifiers", nil()),
                field("operator", math_operator_tree(operator)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator_tree(operator: MathOperatorSyntax) -> SyntaxValue {
    match operator {
        MathOperatorSyntax::Vuhu { vuhu } => node(
            "VuhuOperator",
            vec![
                field("vuhu", word_value(vuhu)),
                field("freeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Maho {
            maho,
            math_expression,
            tehu,
        } => node(
            "MahoOperator",
            vec![
                field("maho", word_value(maho)),
                field("freeModifiers", nil()),
                field("mathExpression", math_expression_tree(*math_expression)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Se { se, inner_operator } => node(
            "SeOperator",
            vec![
                field("se", word_value(se)),
                field("freeModifiers", nil()),
                field("innerOperator", math_operator_tree(*inner_operator)),
            ],
        ),
        MathOperatorSyntax::Nahe {
            nahe,
            inner_operator,
        } => node(
            "NaheOperator",
            vec![
                field("nahe", word_value(nahe)),
                field("freeModifiers", nil()),
                field("innerOperator", math_operator_tree(*inner_operator)),
            ],
        ),
        MathOperatorSyntax::Nahu {
            nahu,
            relation,
            tehu,
        } => node(
            "NahuOperator",
            vec![
                field("nahu", word_value(nahu)),
                field("freeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Connected {
            left_operator,
            connective,
            right_operator,
        } => node(
            "ConnectedOperator",
            vec![
                field("leftOperator", math_operator_tree(*left_operator)),
                field("connective", connective_tree(connective)),
                field("rightOperator", math_operator_tree(*right_operator)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_tree(relation: RelationSyntax) -> SyntaxValue {
    match relation {
        RelationSyntax::Connected {
            connective,
            leading_relation,
            trailing_relation,
        } => node(
            "ConnectedRelation",
            vec![
                field("connective", connective_tree(connective)),
                field("leadingRelation", relation_tree(*leading_relation)),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Co {
            leading_relation,
            co,
            free_modifiers,
            trailing_relation,
        } => node(
            "CoRelation",
            vec![
                field("leadingRelation", relation_tree(*leading_relation)),
                field("co", word_value(co)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Bo {
            leading_relation,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_relation,
        } => node(
            "BoRelation",
            vec![
                field("leadingRelation", relation_tree(*leading_relation)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Na {
            na,
            free_modifiers,
            inner_relation,
        } => node(
            "NaRelation",
            vec![
                field("na", word_value(na)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Base { word } => {
            node("BaseRelation", vec![field("word", word_value(word))])
        }
        RelationSyntax::Se {
            se,
            free_modifiers,
            inner_relation,
        } => node(
            "SeRelation",
            vec![
                field("se", word_value(se)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Ke {
            ke_tense_modal,
            ke,
            ke_free_modifiers,
            relation,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeRelation",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerRelation", relation_tree(*relation)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationSyntax::TenseModal {
            tense_modal,
            inner_relation,
        } => node(
            "TenseModalRelation",
            vec![
                field("tenseModal", tense_modal_tree(tense_modal)),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Guha {
            guhek,
            leading_predicate,
            gik,
            trailing_predicate,
        } => node(
            "GuhaRelation",
            vec![
                field("guhek", connective_tree(guhek)),
                field("leadingPredicate", predicate_tree(*leading_predicate)),
                field("gik", connective_tree(gik)),
                field("trailingPredicate", predicate_tree(*trailing_predicate)),
            ],
        ),
        RelationSyntax::Abstraction { abstraction } => node(
            "AbstractionRelation",
            vec![field("abstraction", abstraction_tree(abstraction))],
        ),
        RelationSyntax::Compound { units } => node(
            "CompoundRelation",
            vec![field("relationUnits", nonempty_relation_units(units))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_tree(abstraction: AbstractionSyntax) -> SyntaxValue {
    node(
        "Abstraction",
        vec![
            field("nu", word_value(abstraction.nu)),
            field("nai", nothing()),
            field("freeModifiers", nil()),
            field(
                "additionalNu",
                list(
                    abstraction
                        .additional_nu
                        .into_iter()
                        .map(additional_nu_tree)
                        .collect(),
                ),
            ),
            field("subsentence", subsentence_tree(*abstraction.subsentence)),
            field("kei", maybe_word(abstraction.kei)),
            field("keiFreeModifiers", nil()),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn additional_nu_tree(additional_nu: AdditionalNuSyntax) -> SyntaxValue {
    node(
        "AdditionalNu",
        vec![
            field("connective", connective_tree(additional_nu.connective)),
            field("nu", word_value(additional_nu.nu)),
            field("nai", nothing()),
            field("freeModifiers", nil()),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_tree(tense_modal: TenseModalSyntax) -> SyntaxValue {
    let free_modifiers = tense_modal.clone().free_modifiers();
    let leaves = match &tense_modal {
        TenseModalSyntax::Fiho { .. } => Vec::new(),
        _ => tense_modal.clone().leaf_words(),
    };
    let ki_field = match &tense_modal {
        TenseModalSyntax::Simple { ki: Some(ki), .. } | TenseModalSyntax::Ki { ki, .. } => {
            just(word_value(ki.clone()))
        }
        TenseModalSyntax::Composite { ki: Some(ki), .. } => just(word_value(ki.clone())),
        _ => nothing(),
    };
    let cuhe_field = match &tense_modal {
        TenseModalSyntax::Composite {
            cuhe: Some(cuhe), ..
        } => just(word_value(cuhe.clone())),
        _ => nothing(),
    };
    let connectives_field = match &tense_modal {
        TenseModalSyntax::Simple { connectives, .. } => {
            list(connectives.iter().cloned().map(word_value).collect())
        }
        TenseModalSyntax::Composite { connectives, .. } => {
            list(connectives.iter().cloned().map(word_value).collect())
        }
        _ => nil(),
    };
    let (time, space, simple, interval, zaho, caha, fiho) = match tense_modal {
        TenseModalSyntax::Composite {
            leaves: _,
            time,
            space,
            nahe,
            interval,
            zaho,
            caha,
            connectives: _,
            ..
        } => (
            time.map_or_else(nothing, |time| just(time_tense_tree(time))),
            space.map_or_else(nothing, |space| just(space_tense_tree(space))),
            nahe.map_or_else(nothing, |nahe| {
                just(node(
                    "SimpleTenseModal",
                    vec![
                        field("nahe", just(word_value(nahe))),
                        field("se", nothing()),
                        field("bai", nothing()),
                        field("nai", nothing()),
                    ],
                ))
            }),
            interval.map_or_else(nothing, |interval| {
                just(node(
                    "Interval",
                    vec![
                        field(
                            "number",
                            if interval.number.is_empty() {
                                nothing()
                            } else {
                                just(nonempty_number_words(interval.number))
                            },
                        ),
                        field("roiOrTahe", word_value(interval.roi_or_tahe)),
                        field("nai", maybe_word(interval.nai)),
                    ],
                ))
            }),
            list(zaho.into_iter().map(word_value).collect()),
            caha.map_or_else(nothing, |caha| just(word_value(caha))),
            nil(),
        ),
        TenseModalSyntax::Pu { word, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(word)])),
                    field("distance", nothing()),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::PuDistance { pu, distance, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(pu)])),
                    field("distance", just(word_value(distance))),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::TimeInterval { word, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", nil()),
                    field("distance", nothing()),
                    field("interval", just(word_value(word))),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::PuCaha { pu, caha, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(pu)])),
                    field("distance", nothing()),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            just(word_value(caha)),
            nil(),
        ),
        TenseModalSyntax::SpaceDistance { word, .. } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", nil()),
                    field("distance", list(vec![word_value(word)])),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", nothing()),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::SpaceDirection { word, .. } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", list(vec![word_value(word)])),
                    field("distance", nil()),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", nothing()),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
            ..
        } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", list(vec![word_value(direction)])),
                    field(
                        "distance",
                        list(distance.into_iter().map(word_value).collect()),
                    ),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", just(word_value(mohi))),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki: _,
            connectives: _,
            ..
        } => (
            nothing(),
            nothing(),
            just(node(
                "SimpleTenseModal",
                vec![
                    field("nahe", maybe_word(nahe)),
                    field("se", maybe_word(se)),
                    field("bai", just(word_value(bai))),
                    field("nai", maybe_word(nai)),
                ],
            )),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Ki { ki: _, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
            ..
        } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            list(vec![node(
                "FihoModal",
                vec![
                    field("nahe", nothing()),
                    field("fiho", word_value(fiho)),
                    field("fihoFreeModifiers", nil()),
                    field("relation", relation_tree(*relation)),
                    field("fehu", maybe_word(fehu)),
                    field("fehuFreeModifiers", nil()),
                ],
            )]),
        ),
        TenseModalSyntax::Caha { word, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            just(word_value(word)),
            nil(),
        ),
        TenseModalSyntax::Zaho { words, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            list(words.into_iter().map(word_value).collect()),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Interval {
            number,
            roi_or_tahe,
            nai,
            ..
        } => (
            nothing(),
            nothing(),
            nothing(),
            just(node(
                "Interval",
                vec![
                    field(
                        "number",
                        if number.is_empty() {
                            nothing()
                        } else {
                            just(nonempty_number_words(number))
                        },
                    ),
                    field("roiOrTahe", word_value(roi_or_tahe)),
                    field("nai", maybe_word(nai)),
                ],
            )),
            nil(),
            nothing(),
            nil(),
        ),
    };

    node(
        "TenseModal",
        vec![
            field("leaves", list(leaves.into_iter().map(word_value).collect())),
            field("time", time),
            field("space", space),
            field("simple", simple),
            field("interval", interval),
            field("zaho", zaho),
            field("caha", caha),
            field("ki", ki_field),
            field("cuhe", cuhe_field),
            field("fiho", fiho),
            field("connectives", connectives_field),
            field(
                "freeModifiers",
                list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn time_tense_tree(time: TimeTenseSyntax) -> SyntaxValue {
    node(
        "Time",
        vec![
            field(
                "direction",
                list(time.direction.into_iter().map(word_value).collect()),
            ),
            field("distance", maybe_word(time.distance)),
            field("interval", maybe_word(time.interval)),
            field("nai", maybe_word(time.nai)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn space_tense_tree(space: SpaceTenseSyntax) -> SyntaxValue {
    node(
        "Space",
        vec![
            field(
                "direction",
                list(space.direction.into_iter().map(word_value).collect()),
            ),
            field(
                "distance",
                list(space.distance.into_iter().map(word_value).collect()),
            ),
            field(
                "interval",
                list(space.interval.into_iter().map(word_value).collect()),
            ),
            field(
                "dimensions",
                list(space.dimensions.into_iter().map(word_value).collect()),
            ),
            field("mohi", maybe_word(space.mohi)),
            field("fehe", maybe_word(space.fehe)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn nonempty_relation_units(units: Vec<RelationUnitSyntax>) -> SyntaxValue {
    let mut rendered = units
        .into_iter()
        .map(relation_unit_tree)
        .collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_letter_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(letter_word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_number_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_math_expressions(expressions: Vec<MathExpressionSyntax>) -> SyntaxValue {
    let mut rendered = expressions
        .into_iter()
        .map(math_expression_tree)
        .collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_name_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(name_word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn letter_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_cmavo_i(normalize_syntax_word(word)))
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_tree(unit: RelationUnitSyntax) -> SyntaxValue {
    match unit {
        RelationUnitSyntax::Word {
            word,
            free_modifiers,
        } => node(
            "WordRelationUnit",
            vec![
                field("word", word_value(word)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Goha {
            goha,
            raho,
            free_modifiers,
        } => node(
            "GohaRelationUnit",
            vec![
                field("goha", word_value(goha)),
                field("raho", maybe_word(raho)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Se {
            se,
            free_modifiers,
            inner_unit,
        } => node(
            "SeRelationUnit",
            vec![
                field("se", word_value(se)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            ke_free_modifiers,
            relation,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeRelationUnit",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Nahe {
            nahe,
            free_modifiers,
            inner_unit,
        } => node(
            "NaheRelationUnit",
            vec![
                field("nahe", word_value(nahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_unit,
        } => node(
            "BoRelationUnit",
            vec![
                field("leadingUnit", relation_unit_tree(*leading_unit)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingUnit", relation_unit_tree(*trailing_unit)),
            ],
        ),
        RelationUnitSyntax::Connected {
            leading_unit,
            connective,
            trailing_unit,
        } => node(
            "ConnectedRelationUnit",
            vec![
                field("leadingUnit", relation_unit_tree(*leading_unit)),
                field("connective", connective_tree(connective)),
                field("trailingUnit", relation_unit_tree(*trailing_unit)),
            ],
        ),
        RelationUnitSyntax::SelbriRelativeClause {
            base,
            selbri_relative_clauses,
        } => node(
            "SelbriRelativeClauseRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field(
                    "selbriRelativeClauses",
                    list(
                        selbri_relative_clauses
                            .into_iter()
                            .map(selbri_relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Wrapped { relation } => node(
            "WrappedRelationUnit",
            vec![field("relation", relation_tree(relation))],
        ),
        RelationUnitSyntax::Jai {
            jai,
            free_modifiers,
            tense_modal,
            inner_unit,
        } => node(
            "JaiRelationUnit",
            vec![
                field("jai", word_value(jai)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
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
        } => node(
            "BeRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
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
        } => node(
            "PreposedBeRelationUnit",
            vec![
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("base", relation_unit_tree(*base)),
            ],
        ),
        RelationUnitSyntax::Abstraction { abstraction } => node(
            "AbstractionRelationUnit",
            vec![field("abstraction", abstraction_tree(abstraction))],
        ),
        RelationUnitSyntax::Me {
            me,
            me_free_modifiers,
            argument,
            mehu,
            mehu_free_modifiers,
            moi_marker,
            moi_free_modifiers,
        } => node(
            "MeRelationUnit",
            vec![
                field("me", word_value(me)),
                field(
                    "meFreeModifiers",
                    list(
                        me_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("argument", argument_tree(argument)),
                field("mehu", maybe_word(mehu)),
                field(
                    "mehuFreeModifiers",
                    list(
                        mehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("moiMarker", maybe_word(moi_marker)),
                field(
                    "moiFreeModifiers",
                    list(
                        moi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Mehoi {
            mehoi,
            quoted_text,
            free_modifiers,
        } => node(
            "MehoiRelationUnit",
            vec![
                field("mehoi", word_value(mehoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Gohoi {
            gohoi,
            quoted_text,
            free_modifiers,
        } => node(
            "GohoiRelationUnit",
            vec![
                field("gohoi", word_value(gohoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Muhoi {
            muhoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "MuhoiRelationUnit",
            vec![
                field("muhoi", word_value(muhoi)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Luhei {
            luhei,
            luhei_free_modifiers,
            text,
            liau,
            liau_free_modifiers,
        } => node(
            "LuheiRelationUnit",
            vec![
                field("luhei", word_value(luhei)),
                field(
                    "luheiFreeModifiers",
                    list(
                        luhei_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("text", lojban_text_tree(text)),
                field("liau", maybe_word(liau)),
                field(
                    "liauFreeModifiers",
                    list(
                        liau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Moi {
            number,
            moi,
            free_modifiers,
        } => node(
            "MoiRelationUnit",
            vec![
                field("number", nonempty_number_words(number)),
                field("moi", word_value(moi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Nuha {
            nuha,
            free_modifiers,
            math_operator,
        } => node(
            "NuhaRelationUnit",
            vec![
                field("nuha", word_value(nuha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("mathOperator", math_operator_tree(math_operator)),
            ],
        ),
        RelationUnitSyntax::Xohi {
            xohi,
            free_modifiers,
            tag,
        } => node(
            "XohiRelationUnit",
            vec![
                field("xohi", word_value(xohi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("tag", tense_modal_tree(tag)),
            ],
        ),
        RelationUnitSyntax::Cei { base, assignments } => node(
            "CeiRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field(
                    "assignments",
                    list(assignments.into_iter().map(cei_assignment_tree).collect()),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn cei_assignment_tree(assignment: CeiAssignmentSyntax) -> SyntaxValue {
    node(
        "CeiAssignment",
        vec![
            field("cei", word_value(assignment.cei)),
            field(
                "freeModifiers",
                list(
                    assignment
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("relationUnit", relation_unit_tree(assignment.relation_unit)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn link_argument_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, LinkArgumentSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let fa_tail = argument
        .clone()
        .map(|argument| (Some(argument), None, Vec::new()))
        .or(cmavo("ku")
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)));
    let fa_link_argument = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(fa_tail)
        .map(
            |((fa, mut fa_free_modifiers), (argument, maybe_ku, trailing_free_modifiers))| {
                if let Some(argument) = argument {
                    new!(LinkArgumentSyntax {
                        fa: Some(fa),
                        fa_free_modifiers,
                        argument: Some(argument),
                    })
                } else {
                    fa_free_modifiers.extend(trailing_free_modifiers);
                    new!(LinkArgumentSyntax {
                        fa: None,
                        fa_free_modifiers: Vec::new(),
                        argument: Some(ArgumentSyntax::Zohe {
                            tag_words: vec![fa],
                            maybe_ku,
                            free_modifiers: fa_free_modifiers,
                        }),
                    })
                }
            },
        );
    let plain_argument = argument.map(|argument| {
        new!(LinkArgumentSyntax {
            fa: None,
            fa_free_modifiers: Vec::new(),
            argument: Some(argument),
        })
    });

    choice((fa_link_argument, plain_argument)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn empty_link_argument() -> LinkArgumentSyntax {
    new!(LinkArgumentSyntax {
        fa: None,
        fa_free_modifiers: Vec::new(),
        argument: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn be_link_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, BeLinkSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(argument.clone(), free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo("be")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(link_argument)
        .then(
            bei_link_parser(argument, free_modifier.clone())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(cmavo("be'o").or_not())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            |(((((be, free_modifiers), link_argument), bei_links), beho), beho_free_modifiers)| {
                let data!(LinkArgumentSyntax {
                    fa,
                    fa_free_modifiers,
                    argument,
                }) = link_argument.into_data();

                new!(BeLinkSyntax {
                    be,
                    free_modifiers,
                    fa,
                    fa_free_modifiers,
                    first_argument: argument,
                    bei_links,
                    beho,
                    beho_free_modifiers,
                })
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn bei_link_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, BeiLinkSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(argument, free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo("bei")
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(link_argument)
        .map(|((bei, bei_free_modifiers), link_argument)| {
            let data!(LinkArgumentSyntax {
                fa,
                fa_free_modifiers,
                argument,
            }) = link_argument.into_data();

            BeiLinkSyntax {
                bei,
                bei_free_modifiers,
                fa,
                fa_free_modifiers,
                argument,
            }
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn maybe_word(word: Option<WordWithModifiers>) -> SyntaxValue {
    word.map_or_else(nothing, |word| just(word_value(word)))
}

#[requires(true)]
#[ensures(true)]
fn maybe_argument(argument: Option<ArgumentSyntax>) -> SyntaxValue {
    argument.map_or_else(nothing, |argument| just(argument_tree(argument)))
}

#[requires(true)]
#[ensures(true)]
fn bei_link_tree(link: BeiLinkSyntax) -> SyntaxValue {
    node(
        "BeiLink",
        vec![
            field("bei", word_value(link.bei)),
            field(
                "beiFreeModifiers",
                list(
                    link.bei_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("fa", maybe_word(link.fa)),
            field(
                "faFreeModifiers",
                list(
                    link.fa_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("argument", maybe_argument(link.argument)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn gadri_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_cmavo_i(normalize_syntax_word(word)))
}

#[requires(true)]
#[ensures(true)]
fn vocative_marker_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn name_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn syntax_word_value(word: WordWithModifiers) -> SyntaxValue {
    SyntaxValue::word(word)
}

#[requires(true)]
#[ensures(true)]
fn normalize_cmavo_i(word: WordWithModifiers) -> WordWithModifiers {
    match word.into_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            WordWithModifiers::base_word(normalize_word_like_cmavo_i(*word_like))
        }
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => {
            WordWithModifiers::emphasized(*bahe, normalize_word_like_cmavo_i(*word_like))
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            WordWithModifiers::standalone_indicator(
                normalize_word_record_cmavo_i(*indicator),
                nai.map(|nai| normalize_word_record_cmavo_i(*nai)),
            )
        }
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => WordWithModifiers::with_indicator(
            normalize_cmavo_i(*base),
            normalize_word_record_cmavo_i(*indicator),
            nai.map(|nai| normalize_word_record_cmavo_i(*nai)),
        ),
        data!(WordWithModifiers::NotEof) => WordWithModifiers::not_eof(),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_word_like_cmavo_i(word_like: WordLike) -> WordLike {
    match word_like.into_data() {
        data!(WordLike::Bare { word }) => WordLike::bare(normalize_word_record_cmavo_i(*word)),
        other => WordLike::from_data(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_word_record_cmavo_i(word: jbotci_morphology::Word) -> jbotci_morphology::Word {
    if word.kind == WordKind::Cmavo {
        let phonemes = word
            .phonemes
            .chars()
            .map(|ch| match ch {
                'ĭ' => 'i',
                'ŭ' => 'u',
                ch => ch,
            })
            .collect();
        word.with_data(data! {
            phonemes: phonemes,
        })
    } else {
        word
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word(word: WordWithModifiers) -> WordWithModifiers {
    match word.into_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            WordWithModifiers::base_word(normalize_syntax_word_like(*word_like))
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            WordWithModifiers::standalone_indicator(*indicator, nai.map(|nai| *nai))
        }
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => WordWithModifiers::emphasized(
            normalize_syntax_word_record(*bahe),
            normalize_syntax_word_like(*word_like),
        ),
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => WordWithModifiers::with_indicator(
            normalize_syntax_word(*base),
            *indicator,
            nai.map(|nai| *nai),
        ),
        data!(WordWithModifiers::NotEof) => WordWithModifiers::not_eof(),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word_like(word_like: WordLike) -> WordLike {
    match word_like.into_data() {
        data!(WordLike::Bare { word }) => WordLike::bare(normalize_syntax_word_record(*word)),
        data!(WordLike::ZoQuote { zo, word }) => WordLike::zo_quote(
            normalize_syntax_word_record(*zo),
            normalize_syntax_word_record(*word),
        ),
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => WordLike::zoi_quote(
            normalize_syntax_word_record(*zoi),
            normalize_syntax_word_record(*opening_delimiter),
            quoted_text,
            normalize_syntax_word_record(*closing_delimiter),
        ),
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => WordLike::lohu_quote(
            normalize_syntax_word_record(*lohu),
            quoted_words
                .into_iter()
                .map(normalize_syntax_word_record)
                .collect(),
            normalize_syntax_word_record(*lehu),
        ),
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => WordLike::single_word_quote(normalize_syntax_word_record(*marker), quoted_text),
        data!(WordLike::Letter { base, bu }) => WordLike::letter(
            normalize_syntax_word_like(*base),
            normalize_syntax_word_record(*bu),
        ),
        data!(WordLike::ZeiLujvo { left, zei, right }) => WordLike::zei_lujvo(
            normalize_syntax_word_like(*left),
            normalize_syntax_word_record(*zei),
            normalize_syntax_word_record(*right),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word_record(word: jbotci_morphology::Word) -> jbotci_morphology::Word {
    word
}

#[requires(true)]
#[ensures(true)]
fn node(constructor: impl AsRef<str>, fields: Vec<SyntaxField>) -> SyntaxValue {
    SyntaxValue::node(constructor.as_ref().to_owned(), fields)
}

#[requires(true)]
#[ensures(true)]
fn field(name: impl AsRef<str>, value: SyntaxValue) -> SyntaxField {
    new!(SyntaxField {
        name: Some(name.as_ref().to_owned()),
        value: value,
    })
}

#[requires(true)]
#[ensures(true)]
fn unnamed_field(value: SyntaxValue) -> SyntaxField {
    new!(SyntaxField {
        name: None,
        value: value,
    })
}

#[requires(true)]
#[ensures(true)]
fn just(value: SyntaxValue) -> SyntaxValue {
    node("Just", vec![unnamed_field(value)])
}

#[requires(true)]
#[ensures(true)]
fn nothing() -> SyntaxValue {
    node("Nothing", Vec::new())
}

#[requires(true)]
#[ensures(true)]
fn nil() -> SyntaxValue {
    node("[]", Vec::new())
}

#[requires(true)]
#[ensures(true)]
fn plain_list(items: Vec<SyntaxValue>) -> SyntaxValue {
    SyntaxValue::list(items)
}

#[requires(true)]
#[ensures(true)]
fn list(items: Vec<SyntaxValue>) -> SyntaxValue {
    items.into_iter().rfold(nil(), |tail, head| {
        node("(:)", vec![unnamed_field(head), unnamed_field(tail)])
    })
}

#[requires(true)]
#[ensures(ret.iter().all(|token| token.span.start <= token.span.end))]
fn spanned_tokens(words: &[WordWithModifiers]) -> Vec<SpannedToken> {
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
fn word_byte_range(word: &WordWithModifiers) -> Option<Range<usize>> {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => word_like_byte_range(word_like),
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            Some(indicator.span.byte_start..nai.as_ref().unwrap_or(indicator).span.byte_end)
        }
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
        data!(WordWithModifiers::NotEof) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::Bare { word }) => Some(word.span.byte_start..word.span.byte_end),
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
fn syntax_error(errors: Vec<Rich<'_, WordWithModifiers, Span>>) -> SyntaxError {
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
