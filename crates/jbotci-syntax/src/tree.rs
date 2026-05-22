//! Source-backed syntax AST model and generated tree traversal.

// The syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use crate::WithIndicators;
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{Word, WordLike};
use serde::{Deserialize, Serialize};
use vec1::{Vec1, smallvec_v1::SmallVec1};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct WithFreeModifiers<T> {
    pub value: T,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

jbotci_tree::tree_model! {
pub type WordRun = SmallVec1<[WithIndicators<WordLike>; 2]>;
pub type MathExpressionVec = Vec1<MathExpressionSyntax>;

#[invariant(crate::indicator_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: Box<WithIndicators<WordLike>>,
    pub nai: Option<Box<Word>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateSyntax {
    pub leading_terms: Vec<TermSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTailSyntax,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTailSyntax {
    #[tree_child(primary)]
    pub first: PredicateTail1Syntax,
    pub ke_continuation: Option<KePredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct KePredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub ke: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTailSyntax>,
    pub kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTail1Syntax {
    #[tree_child(primary)]
    pub first: PredicateTail2Syntax,
    pub continuations: Vec<PredicateTailContinuationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTailContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTail2Syntax,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTail2Syntax {
    #[tree_child(primary)]
    pub first: PredicateTail3Syntax,
    pub bo_continuation: Option<BoPredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct BoPredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub bo: WithFreeModifiers<WithIndicators<WordLike>>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: Box<PredicateTail2Syntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum PredicateTail3Syntax {
    Relation {
        #[tree_child(primary)]
        relation: RelationSyntax,
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    GekSentence(GekSentenceSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum GekSentenceSyntax {
    Pair {
        gek: ConnectiveSyntax,
        first: Box<SubsentenceSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubsentenceSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum SubsentenceSyntax {
    Plain(PredicateSyntax),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_subsentence: Box<SubsentenceSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct TextSyntax {
    pub leading_nai: Vec<WithIndicators<WordLike>>,
    pub leading_cmevla: Vec<WithIndicators<WordLike>>,
    pub leading_indicators: Vec<Indicator>,
    pub leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub leading_connective: Option<ConnectiveSyntax>,
    #[tree_child(primary)]
    pub paragraphs: Vec<ParagraphSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ParagraphSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub niho: Vec<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statements: Vec<ParagraphStatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ParagraphStatementSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub connective: Option<ConnectiveSyntax>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statement: Option<StatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum FreeModifierSyntax {
    Sei {
        sei: WithFreeModifiers<WithIndicators<WordLike>>,
        terms: Vec<TermSyntax>,
        cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        relation: RelationSyntax,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    To {
        to: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        toi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Xi {
        xi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expression: MathExpressionSyntax,
    },
    Mai {
        number: WordRun,
        mai: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Soi {
        soi: WithFreeModifiers<WithIndicators<WordLike>>,
        leading_argument: Box<ArgumentSyntax>,
        trailing_argument: Option<Box<ArgumentSyntax>>,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vocative {
        vocative_markers: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        argument: Option<ArgumentSyntax>,
        dohu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Replacement {
        lohai: Option<WithIndicators<WordLike>>,
        old_words: Vec<WithIndicators<WordLike>>,
        sahai: Option<WithIndicators<WordLike>>,
        new_words: Vec<WithIndicators<WordLike>>,
        lehai: WithFreeModifiers<WithIndicators<WordLike>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum StatementSyntax {
    Tuhe {
        tense_modal: Option<TenseModalSyntax>,
        tuhe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        tuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(PredicateSyntax),
    Connected {
        leading_statement: Box<StatementSyntax>,
        i: WithIndicators<WordLike>,
        connective: ConnectiveSyntax,
        trailing_statement: Box<StatementSyntax>,
    },
    PreIConnected {
        leading_statement: Box<StatementSyntax>,
        connective: ConnectiveSyntax,
        i: WithIndicators<WordLike>,
        trailing_statement: Box<StatementSyntax>,
    },
    Iau {
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
        iau: WithFreeModifiers<WithIndicators<WordLike>>,
        reset_terms: Vec<TermSyntax>,
    },
    ExperimentalPredicateContinuation {
        leading_statement: Box<StatementSyntax>,
        continuation: PredicateStatementContinuationSyntax,
    },
    Fragment(FragmentSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateStatementContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub marker: PredicateStatementContinuationMarkerSyntax,
    pub trailing_subsentence: SubsentenceSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum PredicateStatementContinuationMarkerSyntax {
    Bo(WithFreeModifiers<WithIndicators<WordLike>>),
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum FragmentSyntax {
    Ek(ConnectiveSyntax),
    Gihek(ConnectiveSyntax),
    Other(WithFreeModifiers<Vec<WithIndicators<WordLike>>>),
    Ijek {
        i: WithIndicators<WordLike>,
        connective: ConnectiveSyntax,
    },
    Prenex {
        terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    BeLink {
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    BeiLink(Vec<BeiLinkSyntax>),
    RelativeClause(Vec<RelativeClauseSyntax>),
    MathExpression(MathExpressionSyntax),
    Term {
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Relation(RelationSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum TermSyntax {
    NuhiTermset {
        nuhi: WithFreeModifiers<WithIndicators<WordLike>>,
        termset: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    GekNuhiTermset {
        m_nuhi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
        gik_nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Cehe {
        leading_terms: Vec<TermSyntax>,
        cehe: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_terms: Vec<TermSyntax>,
    },
    Pehe {
        leading_terms: Vec<TermSyntax>,
        pehe: WithFreeModifiers<WithIndicators<WordLike>>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Argument(ArgumentSyntax),
    Fa {
        fa: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
        ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        na_ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    BareNa(WithFreeModifiers<WithIndicators<WordLike>>),
    NoihaAdverbial {
        noiha: WithFreeModifiers<WithIndicators<WordLike>>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<RelationSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PoihaBrigahi {
        poiha: WithFreeModifiers<WithIndicators<WordLike>>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<RelationSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        brigahi_ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    FihoiAdverbial {
        fihoi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        fihau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    SoiAdverbial {
        soi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    JaiTagged {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tag: Option<TenseModalSyntax>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
    },
    Tagged {
        tense_modal: Option<TenseModalSyntax>,
        #[tree_child(primary)]
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
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_term: Box<TermSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum TermWrapperKindSyntax {
    Lahe,
    NaheBo,
    Nahe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ArgumentTagSyntax {
    TenseModal(TenseModalSyntax),
    Fa(WithFreeModifiers<WithIndicators<WordLike>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ArgumentConnectionSyntax {
    pub connective: ConnectiveSyntax,
    #[tree_child(primary)]
    pub argument: Box<ArgumentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ArgumentSyntax {
    Quote(QuoteSyntax),
    MathExpression {
        li: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expression: MathExpressionSyntax,
        loho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Letter {
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    RelativeClause {
        base_argument: Box<ArgumentSyntax>,
        vuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Vuho {
        base_argument: Box<ArgumentSyntax>,
        vuho_marker: WithFreeModifiers<WithIndicators<WordLike>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        connected_argument: Option<ArgumentConnectionSyntax>,
    },
    BridiDescription {
        lohoi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Tagged {
        tag: ArgumentTagSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WithIndicators<WordLike>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WithFreeModifiers<WithIndicators<WordLike>>,
        wrapper_bo: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        inner_term: Box<TermSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Koha(WithFreeModifiers<WithIndicators<WordLike>>),
    Zohe {
        tag: Option<ArgumentTagSyntax>,
        maybe_ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        lahe: WithFreeModifiers<WithIndicators<WordLike>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Bo {
        leading_argument: Box<ArgumentSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Gek {
        gek: ConnectiveSyntax,
        leading_argument: Box<ArgumentSyntax>,
        gik: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
    },
    Descriptor(DescriptorSyntax),
    ConnectedDescriptor(ConnectedDescriptorSyntax),
    Name {
        la: WithFreeModifiers<WithIndicators<WordLike>>,
        names: WithFreeModifiers<WordRun>,
    },
    Cmevla(WithFreeModifiers<WordRun>),
    RelationVocative {
        leading_relative_clauses: Vec<RelativeClauseSyntax>,
        relation: RelationSyntax,
        trailing_relative_clauses: Vec<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum RelativeClauseSyntax {
    Goi(GoiRelativeClauseSyntax),
    Noi {
        noi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Poi {
        poi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zihe {
        zihe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
    Connected {
        connective: ConnectiveSyntax,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct GoiRelativeClauseSyntax {
    pub goi: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub argument: ArgumentSyntax,
    pub gehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct SelbriRelativeClauseSyntax {
    pub nohoi: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub relation: RelationSyntax,
    pub kuhoi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum QuoteSyntax {
    Lu {
        lu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: TextSyntax,
        lihu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zo(WithFreeModifiers<WithIndicators<WordLike>>),
    ZohOi(WithFreeModifiers<WithIndicators<WordLike>>),
    Zoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Lohu(WithFreeModifiers<WithIndicators<WordLike>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct DescriptorSyntax {
    pub descriptor: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub outer_quantifier: Option<QuantifierSyntax>,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<RelationSyntax>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct DescriptorHeadSyntax {
    pub descriptor: WithFreeModifiers<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ConnectedDescriptorSyntax {
    pub leading_descriptor_head: DescriptorHeadSyntax,
    pub connective: ConnectiveSyntax,
    pub trailing_descriptor_head: DescriptorHeadSyntax,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<RelationSyntax>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ConnectiveSyntax {
    Afterthought {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Relation {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PredicateTail {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Forethought {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NonLogical {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Interval {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct BeiLinkSyntax {
    pub bei: WithFreeModifiers<WithIndicators<WordLike>>,
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkArgumentSyntax {
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.first_argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeLinkSyntax {
    pub be: WithFreeModifiers<WithIndicators<WordLike>>,
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub first_argument: Option<ArgumentSyntax>,
    pub bei_links: Vec<BeiLinkSyntax>,
    pub beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ConnectiveKind {
    Afterthought,
    Relation,
    PredicateTail,
    Forethought,
    NonLogical,
    Interval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ArgumentTailElementSyntax {
    Argument(Box<ArgumentSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Quantifier(QuantifierSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum QuantifierSyntax {
    Number {
        #[tree_child(primary)]
        number: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum MathExpressionSyntax {
    Number(QuantifierSyntax),
    Letter {
        #[tree_child(primary)]
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Gek {
        gek: ConnectiveSyntax,
        left_expression: Box<MathExpressionSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Forethought {
        peho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        operator: MathOperatorSyntax,
        operands: Vec<MathExpressionSyntax>,
        kuhe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    ReversePolish {
        fuha: WithFreeModifiers<WithIndicators<WordLike>>,
        operands: Vec<MathExpressionSyntax>,
        operators: Vec<MathOperatorSyntax>,
    },
    Nihe {
        nihe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mohe {
        mohe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Johi {
        johi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expressions: MathExpressionVec,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Lahe {
        markers: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Connected {
        left_expression: Box<MathExpressionSyntax>,
        connective: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Binary {
        left_expression: Box<MathExpressionSyntax>,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Bihe {
        left_expression: Box<MathExpressionSyntax>,
        bihe: WithFreeModifiers<WithIndicators<WordLike>>,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum MathOperatorSyntax {
    Vuhu(WithFreeModifiers<WithIndicators<WordLike>>),
    Maho {
        maho: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Bo {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        right_operator: Box<MathOperatorSyntax>,
    },
    Connected {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MathOperatorSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum RelationSyntax {
    Connected {
        leading_relation: Box<RelationSyntax>,
        connective: ConnectiveSyntax,
        trailing_relation: Box<RelationSyntax>,
    },
    Co {
        leading_relation: Box<RelationSyntax>,
        co: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_relation: Box<RelationSyntax>,
    },
    Bo {
        leading_relation: Box<RelationSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_relation: Box<RelationSyntax>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Base(WithIndicators<WordLike>),
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TenseModal {
        tense_modal: TenseModalSyntax,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Guha {
        guhek: ConnectiveSyntax,
        leading_predicate: Box<PredicateSyntax>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<PredicateSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
    },
    Abstraction(AbstractionSyntax),
    Compound(Box<RelationUnitVec>),
}

pub type RelationUnitVec = SmallVec1<[RelationUnitSyntax; 2]>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct TimeTenseSyntax {
    pub direction: Vec<WithIndicators<WordLike>>,
    pub distance: Option<WithIndicators<WordLike>>,
    pub interval: Option<WithIndicators<WordLike>>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct SpaceTenseSyntax {
    pub direction: Vec<WithIndicators<WordLike>>,
    pub distance: Vec<WithIndicators<WordLike>>,
    pub interval: Vec<WithIndicators<WordLike>>,
    pub dimensions: Vec<WithIndicators<WordLike>>,
    pub mohi: Option<WithIndicators<WordLike>>,
    pub fehe: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct IntervalTenseSyntax {
    pub number: Option<WordRun>,
    pub roi_or_tahe: WithIndicators<WordLike>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct SimpleTenseModalSyntax {
    pub nahe: Option<WithIndicators<WordLike>>,
    pub se: Option<WithIndicators<WordLike>>,
    pub bai: Option<WithIndicators<WordLike>>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct FihoModalSyntax {
    pub nahe: Option<WithIndicators<WordLike>>,
    pub fiho: WithFreeModifiers<WithIndicators<WordLike>>,
    pub relation: RelationSyntax,
    pub fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum CompositeTenseModalPartSyntax {
    Word(WithIndicators<WordLike>),
    Fiho(FihoModalSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum TenseModalSyntax {
    Composite {
        #[tree_child(primary)]
        parts: WithFreeModifiers<Vec<CompositeTenseModalPartSyntax>>,
    },
    Pu(WithFreeModifiers<WithIndicators<WordLike>>),
    PuDistance {
        pu: WithIndicators<WordLike>,
        distance: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    TimeInterval(WithFreeModifiers<WithIndicators<WordLike>>),
    PuCaha {
        pu: WithIndicators<WordLike>,
        caha: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    SpaceDistance(WithFreeModifiers<WithIndicators<WordLike>>),
    SpaceDirection(WithFreeModifiers<WithIndicators<WordLike>>),
    SpaceMovement {
        mohi: WithIndicators<WordLike>,
        direction: WithFreeModifiers<WithIndicators<WordLike>>,
        distance: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Simple {
        nahe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        se: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        bai: WithFreeModifiers<WithIndicators<WordLike>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        ki: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Ki(WithFreeModifiers<WithIndicators<WordLike>>),
    Fiho {
        fiho: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Caha(WithFreeModifiers<WithIndicators<WordLike>>),
    Zaho(WithFreeModifiers<Vec<WithIndicators<WordLike>>>),
    Interval {
        number: Option<WordRun>,
        roi_or_tahe: WithFreeModifiers<WithIndicators<WordLike>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct AbstractionSyntax {
    pub nu: WithFreeModifiers<WithIndicators<WordLike>>,
    pub nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub additional_nu: Vec<AdditionalNuSyntax>,
    #[tree_child(primary)]
    pub subsentence: Box<SubsentenceSyntax>,
    pub kei: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct AdditionalNuSyntax {
    pub connective: ConnectiveSyntax,
    pub nu: WithFreeModifiers<WithIndicators<WordLike>>,
    pub nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum RelationUnitSyntax {
    Word(WithFreeModifiers<WithIndicators<WordLike>>),
    Goha {
        goha: WithFreeModifiers<WithIndicators<WordLike>>,
        raho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Bo {
        leading_unit: Box<RelationUnitSyntax>,
        bo_connective: Option<ConnectiveSyntax>,
        bo_tense_modal: Option<TenseModalSyntax>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Connected {
        leading_unit: Box<RelationUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    SelbriRelativeClause {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativeClauseSyntax>,
    },
    Wrapped(RelationSyntax),
    Jai {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tense_modal: Option<TenseModalSyntax>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PreposedBe {
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<ArgumentSyntax>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
    },
    Abstraction(AbstractionSyntax),
    Me {
        me: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
        mehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        moi_marker: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mehoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Gohoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Muhoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Luhei {
        luhei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: TextSyntax,
        liau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Moi {
        number: WordRun,
        moi: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Nuha {
        nuha: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_operator: MathOperatorSyntax,
    },
    Xohi {
        xohi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        tag: TenseModalSyntax,
    },
    Cei {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        assignments: Vec<CeiAssignmentSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct CeiAssignmentSyntax {
    pub cei: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub relation_unit: RelationUnitSyntax,
}

}

impl<T: TreeNode> TreeNode for WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
    where
        V: jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
    {
        self.value.visit_in_order(visitor);
        if !self.free_modifiers.is_empty() {
            let field_ref = jbotci_tree::FieldRef::new(Some("free_modifiers"), false);
            visitor.enter_field(field_ref);
            self.free_modifiers.visit_in_order(visitor);
            visitor.exit_field(field_ref);
        }
    }
}
