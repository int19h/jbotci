use std::ops::Range;

use bityzba::{data, expensive_ensures, expensive_requires, invariant, new, requires};
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
    Fragment, LojbanText, Paragraph, ParagraphStatement, ParseOptions, Statement, SyntaxError,
    SyntaxField, SyntaxParse, SyntaxValue,
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
    relation: RelationSyntax,
    tail_terms: Vec<TermSyntax>,
    vau: Option<WordWithModifiers>,
    gek_sentence: Option<GekSentenceSyntax>,
    bo_continuation: Option<PredicateTailBoContinuationSyntax>,
    ke_continuation: Option<PredicateTailKeContinuationSyntax>,
    continuations: Vec<PredicateTailContinuationSyntax>,
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
    paragraph_niho: Vec<WordWithModifiers>,
    paragraph_statements: Vec<ParagraphStatementSyntax>,
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
        terms: Vec<TermSyntax>,
        cu: Option<WordWithModifiers>,
        relation: RelationSyntax,
        sehu: Option<WordWithModifiers>,
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
        expression: MathExpressionSyntax,
    },
    Mai {
        number: Vec<WordWithModifiers>,
        mai: WordWithModifiers,
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
        argument: Option<ArgumentSyntax>,
        dohu: Option<WordWithModifiers>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum StatementSyntax {
    Tuhe {
        tense_modal: Option<TenseModalSyntax>,
        tuhe: WordWithModifiers,
        text: Box<TextSyntax>,
        tuhu: Option<WordWithModifiers>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WordWithModifiers,
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(BasicPredicate),
    Connected {
        i: WordWithModifiers,
        connective: ConnectiveSyntax,
        leading_statement: Box<StatementSyntax>,
        trailing_statement: Box<StatementSyntax>,
    },
    Fragment(FragmentSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum FragmentSyntax {
    Gihek {
        connective: ConnectiveSyntax,
    },
    BeLink {
        be: WordWithModifiers,
        fa: Option<WordWithModifiers>,
        first_argument: Box<ArgumentSyntax>,
        beho: Option<WordWithModifiers>,
    },
    RelativeClause {
        relative_clauses: Vec<GoiRelativeClauseSyntax>,
    },
    MathExpression {
        number: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
    },
    Term {
        terms: Vec<TermSyntax>,
        vau: Option<WordWithModifiers>,
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
        termset: Vec<TermSyntax>,
        nuhu: Option<WordWithModifiers>,
    },
    GekNuhiTermset {
        m_nuhi: Option<WordWithModifiers>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WordWithModifiers>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gik_nuhu: Option<WordWithModifiers>,
    },
    Cehe {
        leading_terms: Vec<TermSyntax>,
        cehe: WordWithModifiers,
        trailing_terms: Vec<TermSyntax>,
    },
    Pehe {
        leading_terms: Vec<TermSyntax>,
        pehe: WordWithModifiers,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Argument(ArgumentSyntax),
    Fa {
        fa: WordWithModifiers,
        argument: ArgumentSyntax,
    },
    NaKu {
        na: WordWithModifiers,
        na_ku: WordWithModifiers,
    },
    BareNa {
        na: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Tagged {
        tense_modal: TenseModalSyntax,
        argument: ArgumentSyntax,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum ArgumentSyntax {
    Quote {
        quote: QuoteSyntax,
    },
    MathExpression {
        li: WordWithModifiers,
        expression: MathExpressionSyntax,
        loho: Option<WordWithModifiers>,
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
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Tagged {
        tag_words: Vec<WordWithModifiers>,
        tag_tense_modal: Option<TenseModalSyntax>,
        tag_fa: Option<WordWithModifiers>,
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WordWithModifiers,
        bo: WordWithModifiers,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WordWithModifiers>,
    },
    Koha {
        koha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zohe {
        tag_words: Vec<WordWithModifiers>,
        maybe_ku: Option<WordWithModifiers>,
    },
    Lahe {
        lahe: WordWithModifiers,
        relative_clauses: Vec<RelativeClauseSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WordWithModifiers>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WordWithModifiers,
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WordWithModifiers>,
    },
    Bo {
        leading_argument: Box<ArgumentSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
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
        names: Vec<WordWithModifiers>,
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
        marker: WordWithModifiers,
        subsentence: SubsentenceSyntax,
        kuho: Option<WordWithModifiers>,
    },
    Zihe {
        zihe: WordWithModifiers,
        inner: Box<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GoiRelativeClauseSyntax {
    goi: WordWithModifiers,
    argument: ArgumentSyntax,
    gehu: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum QuoteSyntax {
    Lu {
        lu: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        text: TextSyntax,
        lihu: Option<WordWithModifiers>,
    },
    Zo {
        zo: WordWithModifiers,
        word: WordWithModifiers,
    },
    ZohOi {
        zohoi: WordWithModifiers,
        quoted_text: String,
    },
    Zoi {
        zoi: WordWithModifiers,
        opening_delimiter: WordWithModifiers,
        closing_delimiter: WordWithModifiers,
        quoted_text: String,
    },
    Lohu {
        lohu: WordWithModifiers,
        quoted_words: Vec<WordWithModifiers>,
        lehu: WordWithModifiers,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BeiLinkSyntax {
    bei: WordWithModifiers,
    fa: Option<WordWithModifiers>,
    argument: Option<ArgumentSyntax>,
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
        trailing_relation: Box<RelationSyntax>,
    },
    Bo {
        leading_relation: Box<RelationSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        trailing_relation: Box<RelationSyntax>,
    },
    Na {
        na: WordWithModifiers,
        inner_relation: Box<RelationSyntax>,
    },
    Base {
        word: WordWithModifiers,
    },
    Se {
        se: WordWithModifiers,
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        relation: Box<RelationSyntax>,
        kehe: Option<WordWithModifiers>,
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
    },
    Pu {
        word: WordWithModifiers,
    },
    PuDistance {
        pu: WordWithModifiers,
        distance: WordWithModifiers,
    },
    TimeInterval {
        word: WordWithModifiers,
    },
    PuCaha {
        pu: WordWithModifiers,
        caha: WordWithModifiers,
    },
    SpaceDistance {
        word: WordWithModifiers,
    },
    SpaceDirection {
        word: WordWithModifiers,
    },
    SpaceMovement {
        mohi: WordWithModifiers,
        direction: WordWithModifiers,
        distance: Option<WordWithModifiers>,
    },
    Simple {
        nahe: Option<WordWithModifiers>,
        se: Option<WordWithModifiers>,
        bai: WordWithModifiers,
        nai: Option<WordWithModifiers>,
        ki: Option<WordWithModifiers>,
        connectives: Vec<WordWithModifiers>,
        extra_leaves: Vec<WordWithModifiers>,
    },
    Ki {
        ki: WordWithModifiers,
    },
    Fiho {
        fiho: WordWithModifiers,
        relation: Box<RelationSyntax>,
        fehu: Option<WordWithModifiers>,
    },
    Caha {
        word: WordWithModifiers,
    },
    Zaho {
        words: Vec<WordWithModifiers>,
    },
    Interval {
        number: Vec<WordWithModifiers>,
        roi_or_tahe: WordWithModifiers,
        nai: Option<WordWithModifiers>,
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
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WordWithModifiers,
        relation: RelationSyntax,
        kehe: Option<WordWithModifiers>,
    },
    Nahe {
        nahe: WordWithModifiers,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Bo {
        leading_unit: Box<RelationUnitSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WordWithModifiers,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Connected {
        leading_unit: Box<RelationUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Wrapped {
        relation: RelationSyntax,
    },
    Jai {
        jai: WordWithModifiers,
        tense_modal: Option<TenseModalSyntax>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
        base: Box<RelationUnitSyntax>,
        be: WordWithModifiers,
        fa: Option<WordWithModifiers>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WordWithModifiers>,
    },
    Abstraction {
        abstraction: AbstractionSyntax,
    },
    Me {
        me: WordWithModifiers,
        argument: ArgumentSyntax,
        mehu: Option<WordWithModifiers>,
    },
    Moi {
        number: Vec<WordWithModifiers>,
        moi: WordWithModifiers,
    },
    Nuha {
        nuha: WordWithModifiers,
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
    let statement_words = text
        .paragraph_statements
        .into_iter()
        .flat_map(ParagraphStatementSyntax::words)
        .collect::<Vec<_>>();
    let has_statement = !statement_words.is_empty();
    let statement = Statement::fragment(Fragment::other(statement_words));
    let _ = options;
    Ok(new!(LojbanText {
        leading_nai: text.leading_nai,
        leading_cmevla: text.leading_cmevla,
        leading_indicators: text.leading_indicators,
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs: vec![new!(Paragraph {
            i: None,
            niho: Vec::new(),
            free_modifiers: Vec::new(),
            statements: vec![new!(ParagraphStatement {
                i: None,
                connective: None,
                free_modifiers: Vec::new(),
                statement: has_statement.then_some(statement),
            })],
        })],
    }))
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                text,
                tuhu,
            } => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.push(tuhe);
                words.extend(text.words());
                words.extend(tuhu);
                words
            }
            StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_statement,
            } => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.push(zohu);
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
            StatementSyntax::Fragment(fragment) => fragment.words(),
        }
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
        words.extend(self.paragraph_niho);
        for paragraph_statement in self.paragraph_statements {
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
                terms,
                cu,
                relation,
                sehu,
            } => {
                let mut words = vec![sei];
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(cu);
                words.extend(relation.words());
                words.extend(sehu);
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
            FreeModifierSyntax::Xi { xi, expression } => {
                let mut words = vec![xi];
                words.extend(expression.words());
                words
            }
            FreeModifierSyntax::Mai { number, mai } => {
                let mut words = number;
                words.push(mai);
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
                argument,
                dohu,
            } => {
                let mut words = vocative_markers;
                if let Some(argument) = argument {
                    words.extend(argument.words());
                }
                words.extend(dohu);
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
                inner_subsentence,
            } => {
                let mut words = prenex_terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.push(zohu);
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
            FragmentSyntax::Gihek { connective } => connective.words(),
            FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                beho,
            } => {
                let mut words = vec![be];
                words.extend(fa);
                words.extend(first_argument.words());
                words.extend(beho);
                words
            }
            FragmentSyntax::RelativeClause { relative_clauses } => relative_clauses
                .into_iter()
                .flat_map(GoiRelativeClauseSyntax::words)
                .collect(),
            FragmentSyntax::MathExpression { number, boi } => {
                [number, boi.into_iter().collect()].concat()
            }
            FragmentSyntax::Term { terms, vau } => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(vau);
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
                termset,
                nuhu,
            } => {
                let mut words = vec![nuhi];
                for term in termset {
                    words.extend(term.words());
                }
                words.extend(nuhu);
                words
            }
            TermSyntax::GekNuhiTermset {
                m_nuhi,
                gek,
                terms,
                nuhu,
                gik,
                gik_terms,
                gik_nuhu,
            } => {
                let mut words = m_nuhi.into_iter().collect::<Vec<_>>();
                words.extend(gek.words());
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(nuhu);
                words.extend(gik.words());
                for term in gik_terms {
                    words.extend(term.words());
                }
                words.extend(gik_nuhu);
                words
            }
            TermSyntax::Cehe {
                leading_terms,
                cehe,
                trailing_terms,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.push(cehe);
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::Pehe {
                leading_terms,
                pehe,
                connective,
                trailing_terms,
            } => {
                let mut words = Vec::new();
                for term in leading_terms {
                    words.extend(term.words());
                }
                words.push(pehe);
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::Argument(argument) => argument.words(),
            TermSyntax::Fa { fa, argument } => {
                let mut words = vec![fa];
                words.extend(argument.words());
                words
            }
            TermSyntax::NaKu { na, na_ku } => vec![na, na_ku],
            TermSyntax::BareNa { na, free_modifiers } => {
                let mut words = vec![na];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            TermSyntax::Tagged {
                tense_modal,
                argument,
            } => {
                let mut words = tense_modal.words();
                words.extend(argument.words());
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
                expression,
                loho,
            } => [vec![li], expression.words(), loho.into_iter().collect()].concat(),
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
                relative_clauses,
            } => {
                let mut words = base_argument.words();
                words.extend(vuho);
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words
            }
            ArgumentSyntax::Tagged {
                tag_words,
                inner_argument,
                ..
            } => {
                let mut words = tag_words;
                words.extend(inner_argument.words());
                words
            }
            ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument,
                luhu,
            } => {
                let mut words = vec![nahe, bo];
                words.extend(inner_argument.words());
                words.extend(luhu);
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
            } => [tag_words, maybe_ku.into_iter().collect()].concat(),
            ArgumentSyntax::Lahe {
                lahe,
                relative_clauses,
                inner_argument,
                luhu,
            } => {
                let mut words = vec![lahe];
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(inner_argument.words());
                words.extend(luhu);
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
                inner_argument,
                kehe,
            } => {
                let mut words = vec![ke];
                words.extend(inner_argument.words());
                words.extend(kehe);
                words
            }
            ArgumentSyntax::Bo {
                leading_argument,
                bo_connective,
                bo_tense_modal,
                bo,
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
            ArgumentSyntax::Name { la, names } => [vec![la], names].concat(),
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
        words.extend(self.argument.words());
        words.extend(self.gehu);
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
                marker,
                subsentence,
                kuho,
            } => {
                let mut words = vec![marker];
                words.extend(subsentence.words());
                words.extend(kuho);
                words
            }
            RelativeClauseSyntax::Zihe { zihe, inner } => {
                let mut words = vec![zihe];
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
            } => {
                let mut words = vec![lu];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(text.words());
                words.extend(lihu);
                words
            }
            QuoteSyntax::Zo { zo, word } => vec![zo, word],
            QuoteSyntax::ZohOi { zohoi, .. } => vec![zohoi],
            QuoteSyntax::Zoi {
                zoi,
                opening_delimiter,
                closing_delimiter,
                ..
            } => vec![zoi, opening_delimiter, closing_delimiter],
            QuoteSyntax::Lohu {
                lohu,
                quoted_words,
                lehu,
            } => [vec![lohu], quoted_words, vec![lehu]].concat(),
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
        ]
        .concat()
    }
}

impl BeiLinkSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.bei];
        words.extend(self.fa);
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
            QuantifierSyntax::Number { number, boi } => {
                [number, boi.into_iter().collect()].concat()
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
                trailing_relation,
            } => {
                let mut words = leading_relation.words();
                words.push(co);
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Bo {
                leading_relation,
                bo_connective,
                bo_tense_modal,
                bo,
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
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Na { na, inner_relation } => {
                let mut words = vec![na];
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Base { word } => vec![word],
            RelationSyntax::Se { se, inner_relation } => {
                let mut words = vec![se];
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                let mut words = vec![ke];
                words.extend(relation.words());
                words.extend(kehe);
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
            RelationUnitSyntax::Se { se, inner_unit } => {
                let mut words = vec![se];
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                let mut words = vec![ke];
                words.extend(relation.words());
                words.extend(kehe);
                words
            }
            RelationUnitSyntax::Nahe { nahe, inner_unit } => {
                let mut words = vec![nahe];
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
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
            RelationUnitSyntax::Wrapped { relation } => relation.words(),
            RelationUnitSyntax::Jai {
                jai,
                tense_modal,
                inner_unit,
            } => {
                let mut words = vec![jai];
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Be {
                base,
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            } => {
                let mut words = base.words();
                words.push(be);
                words.extend(fa);
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                words.extend(beho);
                words
            }
            RelationUnitSyntax::Abstraction { abstraction } => abstraction.words(),
            RelationUnitSyntax::Me { me, argument, mehu } => {
                let mut words = vec![me];
                words.extend(argument.words());
                words.extend(mehu);
                words
            }
            RelationUnitSyntax::Moi { number, moi } => {
                let mut words = number;
                words.push(moi);
                words
            }
            RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            } => {
                let mut words = vec![nuha];
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
    fn words(self) -> Vec<WordWithModifiers> {
        match self {
            TenseModalSyntax::Composite { leaves, .. } => leaves,
            TenseModalSyntax::Pu { word } | TenseModalSyntax::Caha { word } => vec![word],
            TenseModalSyntax::PuDistance { pu, distance } => vec![pu, distance],
            TenseModalSyntax::TimeInterval { word } => vec![word],
            TenseModalSyntax::PuCaha { pu, caha } => vec![pu, caha],
            TenseModalSyntax::SpaceDistance { word } => vec![word],
            TenseModalSyntax::SpaceDirection { word } => vec![word],
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            } => [vec![mohi, direction], distance.into_iter().collect()].concat(),
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
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
            TenseModalSyntax::Ki { ki } => vec![ki],
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
            } => {
                let mut words = vec![fiho];
                words.extend((*relation).words());
                words.extend(fehu);
                words
            }
            TenseModalSyntax::Zaho { words } => words,
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
            } => [number, vec![roi_or_tahe], nai.into_iter().collect()].concat(),
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
    argument.define(argument_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        text.clone(),
        free_modifier.clone(),
        source,
    ));
    relation.define(relation_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        free_modifier.clone(),
    ));

    let argument_term = argument.clone().map(TermSyntax::Argument);
    let fa_term = cmavo_of("FA", FA_WORDS)
        .then(argument.clone())
        .map(|(fa, argument)| TermSyntax::Fa { fa, argument });
    let na_ku_term = na_cmavo()
        .then(cmavo("ku"))
        .map(|(na, na_ku)| TermSyntax::NaKu { na, na_ku });
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
        .ignore_then(tense_modal());
    let tagged_term_before_tag =
        tagged_term_start
            .clone()
            .then(tense_modal().rewind())
            .map(|(tense_modal, _)| TermSyntax::Tagged {
                tense_modal,
                argument: implicit_zohe_argument(),
            });
    let tagged_term_before_non_relation = tagged_term_start
        .then(relation.clone().rewind().not())
        .then(
            argument
                .clone()
                .or(cmavo("ku").or_not().map(|maybe_ku| ArgumentSyntax::Zohe {
                    tag_words: Vec::new(),
                    maybe_ku,
                })),
        )
        .map(|((tense_modal, _), argument)| TermSyntax::Tagged {
            tense_modal,
            argument,
        });
    let tagged_term = choice((tagged_term_before_tag, tagged_term_before_non_relation));
    let base_simple_term = choice((
        fa_term,
        tagged_term,
        argument_term,
        na_ku_term,
        bare_na_term,
    ))
    .boxed();
    let term = recursive(|term| {
        let gek_nuhi_termset = cmavo("nu'i")
            .or_not()
            .then(modal_forethought_connective())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .then(gik_connective())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .map(
                |((((((m_nuhi, gek), terms), nuhu), gik), gik_terms), gik_nuhu)| {
                    TermSyntax::GekNuhiTermset {
                        m_nuhi,
                        gek,
                        terms,
                        nuhu,
                        gik,
                        gik_terms,
                        gik_nuhu,
                    }
                },
            );
        let nuhi_termset = cmavo("nu'i")
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(cmavo("nu'u").or_not())
            .map(|((nuhi, termset), nuhu)| TermSyntax::NuhiTermset {
                nuhi,
                termset,
                nuhu,
            });
        let simple_term =
            choice((base_simple_term.clone(), gek_nuhi_termset, nuhi_termset)).boxed();
        let term2 = simple_term
            .clone()
            .then(
                cmavo("ce'e")
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
                cehe_tail.map_or(leading_term.clone(), |(cehe, trailing_terms)| {
                    TermSyntax::Cehe {
                        leading_terms: vec![leading_term],
                        cehe,
                        trailing_terms,
                    }
                })
            })
            .boxed();
        term2
            .clone()
            .then(
                cmavo("pe'e")
                    .then(statement_connective())
                    .then(term2.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(leading_term, pehe_tails)| {
                pehe_tails.into_iter().fold(
                    leading_term,
                    |leading_term, ((pehe, connective), trailing_term)| TermSyntax::Pehe {
                        leading_terms: vec![leading_term],
                        pehe,
                        connective,
                        trailing_terms: vec![trailing_term],
                    },
                )
            })
    })
    .boxed();
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
            let ke = tense_modal()
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
        let implicit_tagged_term_before_grouped_gek =
            tense_modal()
                .then(cmavo("ke").rewind())
                .map(|(tense_modal, _)| TermSyntax::Tagged {
                    tense_modal,
                    argument: implicit_zohe_argument(),
                });
        let non_grouped_gek_term = cmavo("ke").rewind().not().ignore_then(term.clone());
        let gek_leading_term = choice((
            implicit_tagged_term_before_grouped_gek,
            non_grouped_gek_term,
        ))
        .boxed();
        let bo_continuation = predicate_tail_connective()
            .then(tense_modal().or_not())
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
            .then(tense_modal().or_not())
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
            .then(tense_modal().or_not())
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
            .then(cu.clone().or_not())
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
            .map(
                |(
                    (
                        (((((leading_terms, cu), relation), tail_terms), vau), bo_continuation),
                        continuations,
                    ),
                    ke_continuation,
                )| BasicPredicate {
                    leading_terms,
                    cu,
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
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
            .map(
                |(
                    ((((relation, tail_terms), vau), bo_continuation), continuations),
                    ke_continuation,
                )| BasicPredicate {
                    leading_terms: Vec::new(),
                    cu: None,
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
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
            .map(
                |(
                    (
                        (
                            ((((cu, _cu_free_modifiers), relation), tail_terms), vau),
                            bo_continuation,
                        ),
                        continuations,
                    ),
                    ke_continuation,
                )| BasicPredicate {
                    leading_terms: Vec::new(),
                    cu: Some(cu),
                    relation,
                    tail_terms,
                    vau,
                    gek_sentence: None,
                    bo_continuation,
                    ke_continuation,
                    continuations,
                },
            )
            .boxed();
        let forethought_predicate = gek_sentence.clone().map(|gek_sentence| BasicPredicate {
            leading_terms: Vec::new(),
            cu: None,
            relation: RelationSyntax::Compound { units: Vec::new() },
            tail_terms: Vec::new(),
            vau: None,
            gek_sentence: Some(gek_sentence),
            bo_continuation: None,
            ke_continuation: None,
            continuations: Vec::new(),
        });
        let forethought_predicate_with_leading_terms = gek_leading_term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(cu.clone().or_not())
            .then(gek_sentence)
            .map(|((leading_terms, cu), gek_sentence)| BasicPredicate {
                leading_terms,
                cu,
                relation: RelationSyntax::Compound { units: Vec::new() },
                tail_terms: Vec::new(),
                vau: None,
                gek_sentence: Some(gek_sentence),
                bo_continuation: None,
                ke_continuation: None,
                continuations: Vec::new(),
            });

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
        .then(subsentence.clone())
        .map(
            |((prenex_terms, zohu), inner_subsentence)| SubsentenceSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subsentence: Box::new(inner_subsentence),
            },
        );
    subsentence.define(choice((prenex_subsentence, plain_subsentence)));
    let predicate = basic_predicate.map(StatementSyntax::Predicate);

    let fragment_term = term.clone();

    let term_fragment = fragment_term
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(cmavo("vau").or_not())
        .map(|(terms, vau)| StatementSyntax::Fragment(FragmentSyntax::Term { terms, vau }));

    let relative_clause_fragment = goi_relative_clause(argument.clone())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|relative_clauses| {
            StatementSyntax::Fragment(FragmentSyntax::RelativeClause { relative_clauses })
        });
    let gihek_fragment = predicate_tail_connective()
        .map(|connective| StatementSyntax::Fragment(FragmentSyntax::Gihek { connective }));

    let be_link_fragment = cmavo("be")
        .then(cmavo_of("FA", FA_WORDS).or_not())
        .then(argument.clone())
        .then(cmavo("be'o").or_not())
        .map(|(((be, fa), first_argument), beho)| {
            StatementSyntax::Fragment(FragmentSyntax::BeLink {
                be,
                fa,
                first_argument: Box::new(first_argument),
                beho,
            })
        });

    let math_expression_fragment = number_quantifier().map(|quantifier| {
        if let QuantifierSyntax::Number { number, boi } = quantifier {
            StatementSyntax::Fragment(FragmentSyntax::MathExpression { number, boi })
        } else {
            unreachable!("number_quantifier returns Number")
        }
    });

    let relation_fragment = relation
        .clone()
        .map(|relation| StatementSyntax::Fragment(FragmentSyntax::Relation { relation }));

    let prenex_statement = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(statement.clone())
        .map(
            |((prenex_terms, zohu), inner_statement)| StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_statement: Box::new(inner_statement),
            },
        );
    let tuhe_statement = tense_modal()
        .or_not()
        .then(cmavo("tu'e"))
        .then(text.clone())
        .then(cmavo("tu'u").or_not())
        .map(
            |(((tense_modal, tuhe), text), tuhu)| StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                text: Box::new(text),
                tuhu,
            },
        );

    let simple_statement_after_i_connective = choice((
        predicate,
        tuhe_statement,
        gihek_fragment,
        be_link_fragment,
        relative_clause_fragment,
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
        .then(tense_modal().or_not().then(cmavo("bo")).or_not())
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
                }
            });
            (i, connective, trailing_statement)
        });
    let i_bo_statement_tail = cmavo("i")
        .then(tense_modal().or_not())
        .then(cmavo("bo"))
        .then(simple_statement_after_i_connective)
        .map(|(((i, tense_modal), bo), trailing_statement)| {
            let mut cmavo = tense_modal.map_or_else(Vec::new, TenseModalSyntax::words);
            cmavo.push(bo);
            (
                i,
                ConnectiveSyntax {
                    kind: ConnectiveKind::Relation,
                    se: None,
                    nahe: None,
                    na: None,
                    cmavo,
                    nai: None,
                },
                trailing_statement,
            )
        });
    let connected_statement_tail =
        choice((i_connective_statement_tail, i_bo_statement_tail)).boxed();
    let statement_body = simple_statement
        .clone()
        .then(connected_statement_tail.repeated().collect::<Vec<_>>())
        .map(|(leading_statement, continuations)| {
            build_connected_statement(leading_statement, continuations)
        });

    statement.define(statement_body);
    free_modifier.define(choice((
        mai_free(),
        xi_free(),
        sei_free(term.clone(), relation.clone()),
        soi_free(argument.clone()),
        to_free(text.clone(), free_modifier.clone()),
        vocative_free(argument.clone(), relation.clone(), subsentence.clone()),
    )));

    let initial_statement = statement.clone().map(|statement| ParagraphStatementSyntax {
        i: None,
        connective: None,
        free_modifiers: Vec::new(),
        statement: Some(statement),
    });

    let i_connective_tag_bo = statement_connective()
        .or_not()
        .then(tense_modal().then(cmavo("bo")).or_not())
        .map(|(connective, tag_bo)| match (connective, tag_bo) {
            (None, None) => None,
            (Some(connective), None) => Some(connective),
            (connective, Some((tense_modal, bo))) => {
                let (kind, se, nahe, na, nai, mut cmavo) = connective.map_or(
                    (ConnectiveKind::Relation, None, None, None, None, Vec::new()),
                    |connective| {
                        (
                            connective.kind,
                            connective.se,
                            connective.nahe,
                            connective.na,
                            connective.nai,
                            connective.cmavo,
                        )
                    },
                );
                cmavo.extend(tense_modal.words());
                cmavo.push(bo);
                Some(ConnectiveSyntax {
                    kind,
                    se,
                    nahe,
                    na,
                    cmavo,
                    nai,
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
        .then(
            cmavo_of("NIhO", &["ni'o", "no'i"])
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(initial_statement.or_not())
        .then(following_statement.repeated().collect::<Vec<_>>())
        .map(
            |(
                (
                    (
                        (
                            (
                                ((leading_nai, leading_cmevla), leading_indicators),
                                leading_free_modifiers,
                            ),
                            leading_connective,
                        ),
                        paragraph_niho,
                    ),
                    initial,
                ),
                following,
            )| {
                let paragraph_statements = initial.into_iter().chain(following).collect::<Vec<_>>();
                TextSyntax {
                    leading_nai,
                    leading_cmevla,
                    leading_indicators,
                    leading_free_modifiers,
                    leading_connective,
                    paragraph_niho,
                    paragraph_statements,
                }
            },
        );

    text.define(text_body);
    text.then_ignore(end()).boxed()
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.clone().words().len() >= old(leading_statement.clone().words().len()))]
fn build_connected_statement(
    leading_statement: StatementSyntax,
    continuations: Vec<(WordWithModifiers, ConnectiveSyntax, StatementSyntax)>,
) -> StatementSyntax {
    let mut statements = vec![leading_statement];
    let mut connectors = Vec::new();
    for (i, connective, trailing_statement) in continuations {
        connectors.push((i, connective));
        statements.push(trailing_statement);
    }

    for index in (0..connectors.len()).rev() {
        if connective_has_bo(&connectors[index].1) {
            let trailing_statement = statements.remove(index + 1);
            let leading_statement = statements.remove(index);
            let (i, connective) = connectors.remove(index);
            statements.insert(
                index,
                StatementSyntax::Connected {
                    i,
                    connective,
                    leading_statement: Box::new(leading_statement),
                    trailing_statement: Box::new(trailing_statement),
                },
            );
        }
    }

    let mut statements = statements.into_iter();
    let mut connected_statement = statements
        .next()
        .expect("there is always at least the leading statement");
    for ((i, connective), trailing_statement) in connectors.into_iter().zip(statements) {
        connected_statement = StatementSyntax::Connected {
            i,
            connective,
            leading_statement: Box::new(connected_statement),
            trailing_statement: Box::new(trailing_statement),
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
        paragraph_niho: Vec::new(),
        paragraph_statements: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn sei_free<'tokens, T, R>(term: T, relation: R) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    cmavo_of("SEI", &["sei", "ti'o"])
        .then(term.repeated().collect::<Vec<_>>())
        .then(cmavo("cu").or_not())
        .then(relation)
        .then(cmavo("se'u").or_not())
        .map(
            |((((sei, terms), cu), relation), sehu)| FreeModifierSyntax::Sei {
                sei,
                terms,
                cu,
                relation,
                sehu,
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
    let math_operand_atom = choice((gek, vei, nihe, mohe, number, letter)).boxed();
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
    math_expression1
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
        .boxed()
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
    let math_operand_atom = choice((gek, vei, number, letter)).boxed();
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
    math_expression1
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
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_parser_with<'tokens, A, R, T, F>(
    argument: A,
    relation: R,
    subsentence: impl Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let quote = quote_argument(source, text);

    let math_expression = cmavo_of("LI", &["li", "me'o"])
        .then(math_expression_body_with_context(
            argument.clone(),
            relation.clone(),
        ))
        .then(cmavo("lo'o").or_not())
        .map(|((li, expression), loho)| ArgumentSyntax::MathExpression {
            li,
            expression,
            loho,
        });

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
        .then(
            choice((
                xi_free(),
                soi_free(argument.clone()),
                vocative_free(argument.clone(), relation.clone(), subsentence.clone()),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(koha, free_modifiers)| ArgumentSyntax::Koha {
            koha,
            free_modifiers,
        });
    let zohe = cmavo("ku").map(|ku| ArgumentSyntax::Zohe {
        tag_words: Vec::new(),
        maybe_ku: Some(ku),
    });

    let lahe = lahe_cmavo()
        .then(
            relative_clauses(argument.clone(), subsentence.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .map(
            |(((lahe, relative_clauses), inner_argument), luhu)| ArgumentSyntax::Lahe {
                lahe,
                relative_clauses,
                inner_argument: Box::new(inner_argument),
                luhu,
            },
        );

    let name = la_cmavo()
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .map(|(la, names)| ArgumentSyntax::Name { la, names });

    let tail_argument = pa_word()
        .rewind()
        .not()
        .ignore_then(argument.clone())
        .map(|argument| match argument {
            ArgumentSyntax::RelativeClause {
                base_argument,
                vuho: _,
                relative_clauses,
            } => vec![
                ArgumentTailElementSyntax::Argument(base_argument),
                ArgumentTailElementSyntax::RelativeClauses(relative_clauses),
            ],
            argument => vec![ArgumentTailElementSyntax::Argument(Box::new(argument))],
        });
    let contextual_quantifier = quantifier_with_context(argument.clone(), relation.clone());
    let descriptor_relative_clauses = relative_clauses(argument.clone(), subsentence.clone())
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
        .clone()
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(argument.clone())
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

    let descriptor_tail = leading_tail_elements
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
            relative_clauses(argument.clone(), subsentence.clone())
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

    let tense_tagged_argument =
        tense_modal()
            .then(argument.clone())
            .map(|(tense_modal, inner_argument)| {
                let tag_words = tense_modal.clone().words();
                ArgumentSyntax::Tagged {
                    tag_words,
                    tag_tense_modal: Some(tense_modal),
                    tag_fa: None,
                    inner_argument: Box::new(inner_argument),
                }
            });
    let fa_tagged_argument =
        cmavo_of("FA", FA_WORDS)
            .then(argument.clone())
            .map(|(fa, inner_argument)| ArgumentSyntax::Tagged {
                tag_words: vec![fa.clone()],
                tag_tense_modal: None,
                tag_fa: Some(fa),
                inner_argument: Box::new(inner_argument),
            });
    let nahe_bo_argument = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo"))
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .map(
            |(((nahe, bo), inner_argument), luhu)| ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument: Box::new(inner_argument),
                luhu,
            },
        );

    let unquantified_base_argument_core = choice((
        quote,
        math_expression,
        letter,
        lahe,
        name,
        tense_tagged_argument,
        fa_tagged_argument,
        nahe_bo_argument,
        descriptor_with_outer_quantifier,
        descriptor_with_gadri,
        descriptor_without_gadri,
        zohe,
        koha,
    ));
    let base_relative_clauses = relative_clauses(argument.clone(), subsentence.clone())
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
                    .then(argument3)
                    .or_not(),
            )
            .map(|(leading_argument, bo_tail)| {
                bo_tail.map_or(
                    leading_argument.clone(),
                    |(((bo_connective, bo_tense_modal), bo), trailing_argument)| {
                        ArgumentSyntax::Bo {
                            leading_argument: Box::new(leading_argument),
                            bo_connective: Some(bo_connective),
                            bo_tense_modal,
                            bo,
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
                .then(argument.clone())
                .then(cmavo("ke'e").or_not())
                .or_not(),
        )
        .map(|(leading_argument, ke_tail)| {
            ke_tail.map_or(
                leading_argument.clone(),
                |((((connective, tense_modal), ke), inner_argument), kehe)| {
                    let connective = tense_modal.map_or(connective.clone(), |tense_modal| {
                        append_connective_words(connective, tense_modal.words())
                    });
                    ArgumentSyntax::Connected {
                        leading_argument: Box::new(leading_argument),
                        connective,
                        trailing_argument: Box::new(ArgumentSyntax::Ke {
                            ke,
                            inner_argument: Box::new(inner_argument),
                            kehe,
                        }),
                    }
                },
            )
        })
        .boxed();

    argument1
        .then(
            cmavo("vu'o")
                .then(
                    relative_clauses(argument, subsentence)
                        .or_not()
                        .map(Option::unwrap_or_default),
                )
                .or_not(),
        )
        .map(|(base_argument, vuho_attachment)| {
            if let Some((vuho, relative_clauses)) = vuho_attachment {
                if relative_clauses.is_empty() {
                    base_argument
                } else {
                    ArgumentSyntax::RelativeClause {
                        base_argument: Box::new(base_argument),
                        vuho: Some(vuho),
                        relative_clauses,
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
        .map(|(number, boi)| QuantifierSyntax::Number { number, boi })
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
fn quote_argument<'tokens, T>(
    source: Option<&'tokens str>,
    text: T,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let compound_quote = any().try_map(move |word: WordWithModifiers, span| {
        let Some(word_like) = quote_word_like(&word) else {
            return Err(Rich::custom(span, "expected quote"));
        };

        match word_like.as_data() {
            data!(WordLike::ZoQuote { word: quoted, .. }) => Ok(ArgumentSyntax::Quote {
                quote: QuoteSyntax::Zo {
                    zo: word.clone(),
                    word: base_word_from_record((**quoted).clone()),
                },
            }),
            data!(WordLike::ZoiQuote {
                opening_delimiter,
                quoted_text,
                closing_delimiter,
                ..
            }) => Ok(ArgumentSyntax::Quote {
                quote: QuoteSyntax::Zoi {
                    zoi: word.clone(),
                    opening_delimiter: base_word_from_record((**opening_delimiter).clone()),
                    closing_delimiter: base_word_from_record((**closing_delimiter).clone()),
                    quoted_text: source_text(source, quoted_text),
                },
            }),
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
                },
            }),
            data!(WordLike::SingleWordQuote {
                marker: _,
                quoted_text,
            }) => Ok(ArgumentSyntax::Quote {
                quote: QuoteSyntax::ZohOi {
                    zohoi: word.clone(),
                    quoted_text: source_text(source, quoted_text),
                },
            }),
            _ => Err(Rich::custom(span, "expected quote")),
        }
    });

    let lu_quote = cmavo("lu")
        .then(simple_vocative_free().repeated().collect::<Vec<_>>())
        .then(text)
        .then(cmavo("li'u").or_not())
        .map(
            |(((lu, free_modifiers), text), lihu)| ArgumentSyntax::Quote {
                quote: QuoteSyntax::Lu {
                    lu,
                    free_modifiers,
                    text,
                    lihu,
                },
            },
        );

    choice((compound_quote, lu_quote)).boxed()
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
) -> BoxedParser<'tokens, Vec<RelativeClauseSyntax>>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let clause = relative_clause(argument, subsentence);
    clause
        .clone()
        .then(
            cmavo("zi'e")
                .then(clause)
                .map(|(zihe, inner)| RelativeClauseSyntax::Zihe {
                    zihe,
                    inner: Box::new(inner),
                })
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
) -> BoxedParser<'tokens, RelativeClauseSyntax>
where
    R: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let goi = goi_relative_clause(argument).map(RelativeClauseSyntax::Goi);
    let noi = cmavo_of("NOI", &["poi", "noi", "voi"])
        .then(subsentence)
        .then(cmavo("ku'o").or_not())
        .map(|((marker, subsentence), kuho)| RelativeClauseSyntax::Noi {
            marker,
            subsentence,
            kuho,
        });
    choice((goi, noi)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause<'tokens, A>(argument: A) -> BoxedParser<'tokens, GoiRelativeClauseSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    cmavo_of("GOI", &["pe", "ne", "po", "po'e", "po'u", "no'u", "goi"])
        .then(argument)
        .then(cmavo("ge'u").or_not())
        .map(|((goi, argument), gehu)| GoiRelativeClauseSyntax {
            goi,
            argument,
            gehu,
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn simple_vocative_free<'tokens>() -> BoxedParser<'tokens, FreeModifierSyntax> {
    let vocative_argument = choice((
        koha_argument().map(|koha| ArgumentSyntax::Koha {
            koha,
            free_modifiers: Vec::new(),
        }),
        cmevla_word()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|cmevla| ArgumentSyntax::Cmevla {
                cmevla,
                free_modifiers: Vec::new(),
            }),
        relation_word().map(|word| ArgumentSyntax::RelationVocative {
            relation: RelationSyntax::Base { word },
            leading_relative_clauses: Vec::new(),
            trailing_relative_clauses: Vec::new(),
        }),
    ));

    vocative_markers()
        .then(vocative_argument.or_not())
        .then(cmavo("do'u").or_not())
        .map(
            |((vocative_markers, argument), dohu)| FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn xi_free<'tokens>() -> BoxedParser<'tokens, FreeModifierSyntax> {
    let number_or_letter =
        number_or_letter_words()
            .then(cmavo("boi").or_not())
            .map(|(number, boi)| {
                MathExpressionSyntax::Number(QuantifierSyntax::Number { number, boi })
            });
    let xi_expression = choice((number_or_letter, math_expression_body()));

    cmavo_of("XI", &["xi", "te'ai"])
        .then(xi_expression)
        .map(|(xi, expression)| FreeModifierSyntax::Xi { xi, expression })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn mai_free<'tokens>() -> BoxedParser<'tokens, FreeModifierSyntax> {
    number_or_letter_words()
        .then(cmavo_of("MAI", MAI_WORDS))
        .map(|(number, mai)| FreeModifierSyntax::Mai { number, mai })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn soi_free<'tokens, A>(argument: A) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    cmavo("soi")
        .then(argument.clone())
        .then(argument.or_not())
        .then(cmavo("se'u").or_not())
        .map(
            |(((soi, leading_argument), trailing_argument), sehu)| FreeModifierSyntax::Soi {
                soi,
                free_modifiers: Vec::new(),
                leading_argument: Box::new(leading_argument),
                trailing_argument: trailing_argument.map(Box::new),
                sehu,
                sehu_free_modifiers: Vec::new(),
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
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let optional_relative_clauses = relative_clauses(argument.clone(), subsentence.clone())
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
        .then(simple_vocative_free().repeated().collect::<Vec<_>>())
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
                        relative_clauses,
                    }
                }
            },
        );
    let vocative_argument = choice((relation_vocative, cmevla_vocative, argument));

    vocative_markers()
        .then(vocative_argument.or_not())
        .then(cmavo("do'u").or_not())
        .map(
            |((vocative_markers, argument), dohu)| FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
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
                },
            ),
    ))
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

#[requires(true)]
#[ensures(true)]
fn relation_parser_with<'tokens, P, R, S, F>(
    argument: P,
    relation: R,
    subsentence: S,
    free_modifier: F,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let me_unit = cmavo("me")
        .then(argument.clone())
        .then(cmavo("me'u").or_not())
        .map(|((me, argument), mehu)| RelationUnitSyntax::Me { me, argument, mehu });

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
        .map(|(number, moi)| RelationUnitSyntax::Moi { number, moi });
    let nuha_unit = cmavo("nu'a")
        .then(math_operator())
        .map(|(nuha, math_operator)| RelationUnitSyntax::Nuha {
            nuha,
            math_operator,
        });
    let xohi_unit = cmavo("xo'i")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal())
        .map(|((xohi, free_modifiers), tag)| RelationUnitSyntax::Xohi {
            xohi,
            free_modifiers,
            tag,
        });

    let ke_unit = cmavo("ke")
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            free_modifier.clone(),
        ))
        .then(cmavo("ke'e").or_not())
        .map(|((ke, relation), kehe)| RelationUnitSyntax::Ke {
            ke_tense_modal: None,
            ke,
            relation,
            kehe,
        });

    let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(choice((
            ke_unit.clone(),
            moi_unit.clone(),
            nuha_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(|(se, inner_unit)| RelationUnitSyntax::Se {
            se,
            inner_unit: Box::new(inner_unit),
        });

    let wrapped_tense_unit = tense_modal()
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            free_modifier.clone(),
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
        .then(choice((
            wrapped_tense_unit,
            ke_unit.clone(),
            moi_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(|(nahe, inner_unit)| RelationUnitSyntax::Nahe {
            nahe,
            inner_unit: Box::new(inner_unit),
        });

    let jai_unit = cmavo("jai")
        .then(tense_modal().or_not())
        .then(choice((
            se_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        )))
        .map(|((jai, tense_modal), inner_unit)| RelationUnitSyntax::Jai {
            jai,
            tense_modal,
            inner_unit: Box::new(inner_unit),
        });

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
        .then(abstraction_subsentence_unit.clone())
        .map(|(se, inner_unit)| RelationUnitSyntax::Se {
            se,
            inner_unit: Box::new(inner_unit),
        });

    let base_unit = choice((
        goha_raho_unit.clone(),
        me_unit.clone(),
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
    let bei_link = bei_link(argument.clone());
    let be_link = cmavo("be")
        .then(cmavo_of("FA", FA_WORDS).or_not())
        .then(argument.clone().or_not())
        .then(bei_link.repeated().collect::<Vec<_>>())
        .then(cmavo("be'o").or_not())
        .map(|((((be, fa), first_argument), bei_links), beho)| {
            (be, fa, first_argument, bei_links, beho)
        });

    let linked_unit_from = |base_unit: BoxedParser<'tokens, RelationUnitSyntax>| {
        base_unit
            .then(be_link.clone().or_not())
            .map(|(base, be_link)| {
                be_link.map_or(base.clone(), |(be, fa, first_argument, bei_links, beho)| {
                    RelationUnitSyntax::Be {
                        base: Box::new(base),
                        be,
                        fa,
                        first_argument,
                        bei_links,
                        beho,
                    }
                })
            })
            .boxed()
    };
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
        let atom_unit = choice((guha_unit, cei_unit.clone(), linked_unit.clone())).boxed();
        let connected_bo_tail = statement_connective()
            .then(tense_modal().or_not())
            .then(cmavo("bo"))
            .then(bo_unit.clone())
            .map(|(((connective, bo_tense_modal), bo), trailing_unit)| {
                (Some(connective), bo_tense_modal, bo, trailing_unit)
            });
        let bare_bo_tail = cmavo("bo")
            .then(bo_unit)
            .map(|(bo, trailing_unit)| (None, None, bo, trailing_unit));
        atom_unit
            .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
            .map(|(leading_unit, bo_tail)| {
                bo_tail.map_or(
                    leading_unit.clone(),
                    |(bo_connective, bo_tense_modal, bo, trailing_unit)| RelationUnitSyntax::Bo {
                        leading_unit: Box::new(leading_unit),
                        bo_connective,
                        bo_tense_modal,
                        bo,
                        trailing_unit: Box::new(trailing_unit),
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

    let post_tense_relation = na_cmavo()
        .then(relation.clone())
        .map(|(na, inner_relation)| RelationSyntax::Na {
            na,
            inner_relation: Box::new(inner_relation),
        })
        .or(relation_units.clone());

    let tagged = tense_modal()
        .then(post_tense_relation)
        .map(|(tense_modal, inner_relation)| RelationSyntax::TenseModal {
            tense_modal,
            inner_relation: Box::new(inner_relation),
        });

    let base_relation = choice((tagged, relation_units));
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
        .then(relation)
        .map(|(na, inner_relation)| RelationSyntax::Na {
            na,
            inner_relation: Box::new(inner_relation),
        });
    let co_relation = recursive(|co_relation| {
        connected_relation
            .clone()
            .then(cmavo("co").then(co_relation).or_not())
            .map(|(leading_relation, co_tail)| {
                co_tail.map_or(leading_relation.clone(), |(co, trailing_relation)| {
                    RelationSyntax::Co {
                        leading_relation: Box::new(leading_relation),
                        co,
                        trailing_relation: Box::new(trailing_relation),
                    }
                })
            })
    });

    choice((na_relation, co_relation)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_units_inner<'tokens, P, S, F>(
    argument: P,
    subsentence: S,
    free_modifier: F,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive(|inner_relation| {
        let me_unit = cmavo("me")
            .then(argument.clone())
            .then(cmavo("me'u").or_not())
            .map(|((me, argument), mehu)| RelationUnitSyntax::Me { me, argument, mehu });
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
            .map(|(number, moi)| RelationUnitSyntax::Moi { number, moi });
        let nuha_unit = cmavo("nu'a")
            .then(math_operator())
            .map(|(nuha, math_operator)| RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            });
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
            .then(abstraction_subsentence_unit.clone())
            .map(|(se, inner_unit)| RelationUnitSyntax::Se {
                se,
                inner_unit: Box::new(inner_unit),
            });
        let ke_unit = cmavo("ke")
            .then(inner_relation.clone())
            .then(cmavo("ke'e").or_not())
            .map(|((ke, relation), kehe)| RelationUnitSyntax::Ke {
                ke_tense_modal: None,
                ke,
                relation,
                kehe,
            });
        let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(|(se, inner_unit)| RelationUnitSyntax::Se {
                se,
                inner_unit: Box::new(inner_unit),
            });
        let nahe_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(|(nahe, inner_unit)| RelationUnitSyntax::Nahe {
                nahe,
                inner_unit: Box::new(inner_unit),
            });
        let bei_link = bei_link(argument.clone());
        let be_link = cmavo("be")
            .then(cmavo_of("FA", FA_WORDS).or_not())
            .then(argument.clone().or_not())
            .then(bei_link.repeated().collect::<Vec<_>>())
            .then(cmavo("be'o").or_not())
            .map(|((((be, fa), first_argument), bei_links), beho)| {
                (be, fa, first_argument, bei_links, beho)
            });

        let base_unit = choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
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
                    be_link.map_or(base.clone(), |(be, fa, first_argument, bei_links, beho)| {
                        RelationUnitSyntax::Be {
                            base: Box::new(base),
                            be,
                            fa,
                            first_argument,
                            bei_links,
                            beho,
                        }
                    })
                })
                .boxed()
        };
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
            let atom_unit = choice((guha_unit, cei_unit.clone(), linked_unit.clone())).boxed();
            let connected_bo_tail = statement_connective()
                .then(tense_modal().or_not())
                .then(cmavo("bo"))
                .then(bo_unit.clone())
                .map(|(((connective, bo_tense_modal), bo), trailing_unit)| {
                    (Some(connective), bo_tense_modal, bo, trailing_unit)
                });
            let bare_bo_tail = cmavo("bo")
                .then(bo_unit)
                .map(|(bo, trailing_unit)| (None, None, bo, trailing_unit));
            atom_unit
                .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
                .map(|(leading_unit, bo_tail)| {
                    bo_tail.map_or(
                        leading_unit.clone(),
                        |(bo_connective, bo_tense_modal, bo, trailing_unit)| {
                            RelationUnitSyntax::Bo {
                                leading_unit: Box::new(leading_unit),
                                bo_connective,
                                bo_tense_modal,
                                bo,
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

#[expensive_requires(!units.is_empty(), "relation unit sequences must be non-empty")]
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
        [RelationUnitSyntax::Se { se, inner_unit }] => RelationSyntax::Se {
            se: se.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit.as_ref())),
        },
        [
            RelationUnitSyntax::Ke {
                ke_tense_modal,
                ke,
                relation,
                kehe,
            },
        ] => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
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
                trailing_unit,
            },
        ] => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
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
        RelationUnitSyntax::Se { se, inner_unit } => RelationSyntax::Se {
            se: se.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit)),
        },
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
        },
        RelationUnitSyntax::Abstraction { abstraction } => RelationSyntax::Abstraction {
            abstraction: abstraction.clone(),
        },
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        } => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
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
        relation,
        tail_terms: Vec::new(),
        vau: None,
        gek_sentence: None,
        bo_continuation: None,
        ke_continuation: None,
        continuations: Vec::new(),
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
            let mut leaves = first.clone().words();
            let mut parts = vec![first];
            let mut connectives = Vec::new();
            for (connective_leaves, connective_cmavo, part) in continuations {
                leaves.extend(connective_leaves);
                connectives.extend(connective_cmavo);
                leaves.extend(part.clone().words());
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
    }
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
                Some(PuTail::Distance(distance)) => TenseModalSyntax::PuDistance { pu, distance },
                Some(PuTail::Caha(caha)) => TenseModalSyntax::PuCaha { pu, caha },
                None => TenseModalSyntax::Pu { word: pu },
            }),
        cmavo_of("VA", &["vi", "va", "vu"]).map(|word| TenseModalSyntax::SpaceDistance { word }),
        cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"])
            .map(|word| TenseModalSyntax::TimeInterval { word }),
        cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        )
        .map(|word| TenseModalSyntax::SpaceDirection { word }),
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
                },
            ),
        cmavo_of("CAhA", CAHA_WORDS).map(|word| TenseModalSyntax::Caha { word }),
        fiho_tense_modal(),
        cmavo_of(
            "ZAhO",
            &[
                "ba'o", "ca'o", "co'a", "co'i", "co'u", "de'a", "di'a", "mo'u", "pu'o", "za'o",
            ],
        )
        .map(|word| TenseModalSyntax::Zaho { words: vec![word] }),
        simple_tense_modal(),
        cmavo("ki").map(|ki| TenseModalSyntax::Ki { ki }),
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
            }),
        cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
            .then(cmavo("nai").or_not())
            .map(|(roi_or_tahe, nai)| TenseModalSyntax::Interval {
                number: Vec::new(),
                roi_or_tahe,
                nai,
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
    if text.paragraph_statements.is_empty() {
        nil()
    } else {
        list(vec![node(
            "Paragraph",
            vec![
                field("i", nothing()),
                field(
                    "niho",
                    list(text.paragraph_niho.into_iter().map(word_value).collect()),
                ),
                field("freeModifiers", nil()),
                field(
                    "statements",
                    list(
                        text.paragraph_statements
                            .into_iter()
                            .map(paragraph_statement_tree)
                            .collect(),
                    ),
                ),
            ],
        )])
    }
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
            terms,
            cu,
            relation,
            sehu,
        } => node(
            "SeiFree",
            vec![
                field("sei", word_value(sei)),
                field("leadingFreeModifiers", nil()),
                field(
                    "terms",
                    if terms.is_empty() {
                        nothing()
                    } else {
                        just(list(terms.into_iter().map(term_tree).collect()))
                    },
                ),
                field("cu", maybe_word(cu)),
                field("cuFreeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("sehu", maybe_word(sehu)),
                field("sehuFreeModifiers", nil()),
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
        FreeModifierSyntax::Xi { xi, expression } => node(
            "XiFree",
            vec![
                field("xi", word_value(xi)),
                field("freeModifiers", nil()),
                field("mathExpression", math_expression_tree(expression)),
            ],
        ),
        FreeModifierSyntax::Mai { number, mai } => node(
            "MaiFree",
            vec![
                field("number", nonempty_number_words(number)),
                field("mai", word_value(mai)),
                field("freeModifiers", nil()),
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
            argument,
            dohu,
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
                field("freeModifiers", nil()),
                field(
                    "argument",
                    argument.map_or_else(nothing, |argument| just(argument_tree(argument))),
                ),
                field("dohu", maybe_word(dohu)),
                field("dohuFreeModifiers", nil()),
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
            text,
            tuhu,
        } => node(
            "TuheStatement",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("tuhe", word_value(tuhe)),
                field("tuheFreeModifiers", nil()),
                field("paragraphs", paragraphs_tree(*text)),
                field("tuhu", maybe_word(tuhu)),
                field("tuhuFreeModifiers", nil()),
            ],
        ),
        StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_statement,
        } => node(
            "PrenexStatement",
            vec![
                field(
                    "prenexTerms",
                    list(prenex_terms.into_iter().map(term_tree).collect()),
                ),
                field("zohu", word_value(zohu)),
                field("zohuFreeModifiers", nil()),
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
        StatementSyntax::Fragment(fragment) => node(
            "StatementFragment",
            vec![field("fragment", fragment_tree(fragment))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn fragment_tree(fragment: FragmentSyntax) -> SyntaxValue {
    match fragment {
        FragmentSyntax::Gihek { connective } => node(
            "GihekFragment",
            vec![
                field("connective", connective_tree(connective)),
                field("freeModifiers", nil()),
            ],
        ),
        FragmentSyntax::BeLink {
            be,
            fa,
            first_argument,
            beho,
        } => node(
            "BeLinkFragment",
            vec![
                field("be", word_value(be)),
                field("freeModifiers", nil()),
                field("fa", maybe_word(fa)),
                field("faFreeModifiers", nil()),
                field("firstArgument", just(argument_tree(*first_argument))),
                field("beiLinks", nil()),
                field("beho", maybe_word(beho)),
                field("behoFreeModifiers", nil()),
            ],
        ),
        FragmentSyntax::RelativeClause { relative_clauses } => node(
            "RelativeClauseFragment",
            vec![field(
                "relativeClauses",
                list(
                    relative_clauses
                        .into_iter()
                        .map(goi_relative_clause_tree)
                        .collect(),
                ),
            )],
        ),
        FragmentSyntax::MathExpression { number, boi } => node(
            "MathExpressionFragment",
            vec![field(
                "mathExpression",
                node(
                    "NumberExpression",
                    vec![
                        field("number", nonempty_number_words(number)),
                        field("boi", maybe_word(boi)),
                        field("freeModifiers", nil()),
                    ],
                ),
            )],
        ),
        FragmentSyntax::Term { terms, vau } => node(
            "TermFragment",
            vec![
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("vau", maybe_word(vau)),
                field("vauFreeModifiers", nil()),
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
            field("leadingFreeModifiers", nil()),
            field("argument", argument_tree(relative_clause.argument)),
            field("gehu", maybe_word(relative_clause.gehu)),
            field("trailingFreeModifiers", nil()),
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
            field("cuFreeModifiers", nil()),
            field("predicateTail", predicate_tail),
            field("freeModifiers", nil()),
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
            termset,
            nuhu,
        } => node(
            "NuhiTermset",
            vec![
                field("nuhi", word_value(nuhi)),
                field("nuhiFreeModifiers", nil()),
                field(
                    "termset",
                    list(termset.into_iter().map(term_tree).collect()),
                ),
                field("nuhu", maybe_word(nuhu)),
                field("nuhuFreeModifiers", nil()),
            ],
        ),
        TermSyntax::GekNuhiTermset {
            m_nuhi,
            gek,
            terms,
            nuhu,
            gik,
            gik_terms,
            gik_nuhu,
        } => node(
            "GekNuhiTermset",
            vec![
                field("mNuhi", maybe_word(m_nuhi)),
                field("nuhiFreeModifiers", nil()),
                field("gek", connective_tree(gek)),
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("nuhu", maybe_word(nuhu)),
                field("nuhuFreeModifiers", nil()),
                field("gik", connective_tree(gik)),
                field(
                    "gikTerms",
                    list(gik_terms.into_iter().map(term_tree).collect()),
                ),
                field("gikNuhu", maybe_word(gik_nuhu)),
                field("gikNuhuFreeModifiers", nil()),
            ],
        ),
        TermSyntax::Cehe {
            leading_terms,
            cehe,
            trailing_terms,
        } => node(
            "CeheTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("cehe", word_value(cehe)),
                field("freeModifiers", nil()),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::Pehe {
            leading_terms,
            pehe,
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
                field("freeModifiers", nil()),
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
        TermSyntax::Fa { fa, argument } => node(
            "FaTerm",
            vec![
                field("fa", word_value(fa)),
                field("freeModifiers", nil()),
                field("argument", argument_tree(argument)),
                field("ku", nothing()),
                field("kuFreeModifiers", nil()),
            ],
        ),
        TermSyntax::NaKu { na, na_ku } => node(
            "NaKuTerm",
            vec![
                field("na", word_value(na)),
                field("naKu", word_value(na_ku)),
                field("freeModifiers", nil()),
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
        TermSyntax::Tagged {
            tense_modal,
            argument,
        } => node(
            "TaggedTerm",
            vec![
                field("tenseModal", just(tense_modal_tree(tense_modal))),
                field("freeModifiers", nil()),
                field("argument", argument_tree(argument)),
            ],
        ),
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
            expression,
            loho,
        } => node(
            "MathExpressionArgument",
            vec![
                field("li", word_value(li)),
                field("liFreeModifiers", nil()),
                field("mathExpression", math_expression_tree(expression)),
                field("loho", maybe_word(loho)),
                field("lohoFreeModifiers", nil()),
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
            relative_clauses,
        } => node(
            "RelativeClauseArgument",
            vec![
                field("baseArgument", argument_tree(*base_argument)),
                field("vuho", maybe_word(vuho)),
                field("vuhoFreeModifiers", nil()),
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
        ArgumentSyntax::Tagged {
            tag_words,
            tag_tense_modal,
            tag_fa,
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
                field("freeModifiers", nil()),
                field("innerArgument", argument_tree(*inner_argument)),
            ],
        ),
        ArgumentSyntax::NaheBo {
            nahe,
            bo,
            inner_argument,
            luhu,
        } => node(
            "NaheBoArgument",
            vec![
                field("nahe", word_value(nahe)),
                field("bo", word_value(bo)),
                field("freeModifiers", nil()),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field("luhuFreeModifiers", nil()),
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
        } => node(
            "ZoheArgument",
            vec![
                field(
                    "tagWords",
                    list(tag_words.into_iter().map(word_value).collect()),
                ),
                field("maybeKu", maybe_word(maybe_ku)),
                field("freeModifiers", nil()),
            ],
        ),
        ArgumentSyntax::Lahe {
            lahe,
            relative_clauses,
            inner_argument,
            luhu,
        } => node(
            "LaheArgument",
            vec![
                field("lahe", word_value(lahe)),
                field("freeModifiers", nil()),
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
                field("luhuFreeModifiers", nil()),
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
            inner_argument,
            kehe,
        } => node(
            "KeArgument",
            vec![
                field("ke", word_value(ke)),
                field("keFreeModifiers", nil()),
                field("innerArgument", argument_tree(*inner_argument)),
                field("kehe", maybe_word(kehe)),
                field("keheFreeModifiers", nil()),
            ],
        ),
        ArgumentSyntax::Bo {
            leading_argument,
            bo_connective,
            bo_tense_modal,
            bo,
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
                field("freeModifiers", nil()),
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
        ArgumentSyntax::Name { la, names } => node(
            "NameArgument",
            vec![
                field("la", gadri_word_value(la)),
                field("laFreeModifiers", nil()),
                field("names", nonempty_name_words(names)),
                field("nameFreeModifiers", nil()),
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
                        field("zohuFreeModifiers", nil()),
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
            marker,
            subsentence,
            kuho,
        } => {
            let constructor = if cmavo_text_matches(&marker, "poi") {
                "PoiRelativeClause"
            } else {
                "NoiRelativeClause"
            };

            node(
                constructor,
                vec![
                    field(
                        if constructor == "NoiRelativeClause" {
                            "noi"
                        } else {
                            "poi"
                        },
                        word_value(marker),
                    ),
                    field("leadingFreeModifiers", nil()),
                    field("subsentence", subsentence_tree(subsentence)),
                    field("kuho", maybe_word(kuho)),
                    field("trailingFreeModifiers", nil()),
                ],
            )
        }
        RelativeClauseSyntax::Zihe { zihe, inner } => node(
            "ZiheRelativeClause",
            vec![
                field("zihe", word_value(zihe)),
                field("freeModifiers", nil()),
                field("inner", relative_clause_tree(*inner)),
            ],
        ),
    }
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
                field("lihuFreeModifiers", nil()),
            ],
        ),
        QuoteSyntax::Zo { zo, word } => node(
            "ZoQuote",
            vec![
                field("zo", word_value(zo)),
                field("word", word_value(word)),
                field("freeModifiers", nil()),
            ],
        ),
        QuoteSyntax::ZohOi { zohoi, quoted_text } => node(
            "ZohOiQuote",
            vec![
                field("zohoi", word_value(zohoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field("freeModifiers", nil()),
            ],
        ),
        QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
        } => node(
            "ZoiQuote",
            vec![
                field("zoi", word_value(zoi)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field("freeModifiers", nil()),
            ],
        ),
        QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            lehu,
        } => node(
            "LohuQuote",
            vec![
                field("lohu", word_value(lohu)),
                field(
                    "quotedWords",
                    list(quoted_words.into_iter().map(word_value).collect()),
                ),
                field("lehu", word_value(lehu)),
                field("lehuFreeModifiers", nil()),
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
            field("freeModifiers", nil()),
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
        QuantifierSyntax::Number { number, boi } => node(
            "NumberQuantifier",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field("boiFreeModifiers", nil()),
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
        QuantifierSyntax::Number { number, boi } => node(
            "NumberExpression",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field("freeModifiers", nil()),
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
            trailing_relation,
        } => node(
            "CoRelation",
            vec![
                field("leadingRelation", relation_tree(*leading_relation)),
                field("co", word_value(co)),
                field("freeModifiers", nil()),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Bo {
            leading_relation,
            bo_connective,
            bo_tense_modal,
            bo,
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
                field("freeModifiers", nil()),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Na { na, inner_relation } => node(
            "NaRelation",
            vec![
                field("na", word_value(na)),
                field("freeModifiers", nil()),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Base { word } => {
            node("BaseRelation", vec![field("word", word_value(word))])
        }
        RelationSyntax::Se { se, inner_relation } => node(
            "SeRelation",
            vec![
                field("se", word_value(se)),
                field("freeModifiers", nil()),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => node(
            "KeRelation",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field("keFreeModifiers", nil()),
                field("innerRelation", relation_tree(*relation)),
                field("kehe", maybe_word(kehe)),
                field("keheFreeModifiers", nil()),
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
    let leaves = match &tense_modal {
        TenseModalSyntax::Fiho { .. } => Vec::new(),
        _ => tense_modal.clone().words(),
    };
    let ki_field = match &tense_modal {
        TenseModalSyntax::Simple { ki: Some(ki), .. } | TenseModalSyntax::Ki { ki } => {
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
        TenseModalSyntax::Pu { word } => (
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
        TenseModalSyntax::PuDistance { pu, distance } => (
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
        TenseModalSyntax::TimeInterval { word } => (
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
        TenseModalSyntax::PuCaha { pu, caha } => (
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
        TenseModalSyntax::SpaceDistance { word } => (
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
        TenseModalSyntax::SpaceDirection { word } => (
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
        TenseModalSyntax::Ki { ki: _ } => (
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
        TenseModalSyntax::Caha { word } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            just(word_value(word)),
            nil(),
        ),
        TenseModalSyntax::Zaho { words } => (
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
            field("freeModifiers", nil()),
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
        RelationUnitSyntax::Se { se, inner_unit } => node(
            "SeRelationUnit",
            vec![
                field("se", word_value(se)),
                field("freeModifiers", nil()),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => node(
            "KeRelationUnit",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field("keFreeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("kehe", maybe_word(kehe)),
                field("keheFreeModifiers", nil()),
            ],
        ),
        RelationUnitSyntax::Nahe { nahe, inner_unit } => node(
            "NaheRelationUnit",
            vec![
                field("nahe", word_value(nahe)),
                field("freeModifiers", nil()),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
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
                field("freeModifiers", nil()),
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
        RelationUnitSyntax::Wrapped { relation } => node(
            "WrappedRelationUnit",
            vec![field("relation", relation_tree(relation))],
        ),
        RelationUnitSyntax::Jai {
            jai,
            tense_modal,
            inner_unit,
        } => node(
            "JaiRelationUnit",
            vec![
                field("jai", word_value(jai)),
                field("freeModifiers", nil()),
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
            fa,
            first_argument,
            bei_links,
            beho,
        } => node(
            "BeRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field("be", word_value(be)),
                field("freeModifiers", nil()),
                field("fa", maybe_word(fa)),
                field("faFreeModifiers", nil()),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field("behoFreeModifiers", nil()),
            ],
        ),
        RelationUnitSyntax::Abstraction { abstraction } => node(
            "AbstractionRelationUnit",
            vec![field("abstraction", abstraction_tree(abstraction))],
        ),
        RelationUnitSyntax::Me { me, argument, mehu } => node(
            "MeRelationUnit",
            vec![
                field("me", word_value(me)),
                field("meFreeModifiers", nil()),
                field("argument", argument_tree(argument)),
                field("mehu", maybe_word(mehu)),
                field("mehuFreeModifiers", nil()),
                field("moiMarker", nothing()),
                field("moiFreeModifiers", nil()),
            ],
        ),
        RelationUnitSyntax::Moi { number, moi } => node(
            "MoiRelationUnit",
            vec![
                field("number", nonempty_number_words(number)),
                field("moi", word_value(moi)),
                field("freeModifiers", nil()),
            ],
        ),
        RelationUnitSyntax::Nuha {
            nuha,
            math_operator,
        } => node(
            "NuhaRelationUnit",
            vec![
                field("nuha", word_value(nuha)),
                field("freeModifiers", nil()),
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
fn bei_link<'tokens, A>(argument: A) -> BoxedParser<'tokens, BeiLinkSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    cmavo("bei")
        .then(cmavo_of("FA", FA_WORDS).or_not())
        .then(argument.or_not())
        .map(|((bei, fa), argument)| BeiLinkSyntax { bei, fa, argument })
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
            field("beiFreeModifiers", nil()),
            field("fa", maybe_word(link.fa)),
            field("faFreeModifiers", nil()),
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

#[expensive_ensures(ret.iter().all(|token| token.span.start <= token.span.end))]
#[requires(true)]
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

#[expensive_ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
#[requires(true)]
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

#[expensive_ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
#[requires(true)]
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

#[expensive_ensures(matches!(ret, SyntaxError::Parse { ref reason, .. } if !reason.is_empty()) || !matches!(ret, SyntaxError::Parse { .. }))]
#[requires(true)]
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
