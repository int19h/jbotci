// The internal syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use crate::{Indicator, WithIndicators};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::WordLike;
use serde::Serialize;
use serde::ser::{SerializeSeq, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct WithFreeModifiers<T> {
    pub value: T,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

impl<T> WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(value: T, free_modifiers: Vec<FreeModifierSyntax>) -> Self {
        Self {
            value,
            free_modifiers,
        }
    }
}

impl<T: Serialize> Serialize for WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.free_modifiers.is_empty() {
            return self.value.serialize(serializer);
        }
        let mut seq = serializer.serialize_seq(Some(1 + self.free_modifiers.len()))?;
        seq.serialize_element(&self.value)?;
        for free_modifier in &self.free_modifiers {
            seq.serialize_element(free_modifier)?;
        }
        seq.end()
    }
}

impl WithFreeModifiers<WithIndicators<WordLike>> {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = vec![self.value];
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl WithFreeModifiers<Vec<WithIndicators<WordLike>>> {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.value;
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateSyntax {
    pub leading_terms: Vec<TermSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub predicate_tail: PredicateTailSyntax,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTailSyntax {
    pub first: PredicateTail1Syntax,
    pub ke_continuation: Option<KePredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct KePredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub ke: WithFreeModifiers<WithIndicators<WordLike>>,
    pub predicate_tail: Box<PredicateTailSyntax>,
    pub kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTail1Syntax {
    pub first: PredicateTail2Syntax,
    pub continuations: Vec<PredicateTailContinuationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTailContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub predicate_tail: PredicateTail2Syntax,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct PredicateTail2Syntax {
    pub first: PredicateTail3Syntax,
    pub bo_continuation: Option<BoPredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct BoPredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<TenseModalSyntax>,
    pub bo: WithIndicators<WordLike>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub predicate_tail: Box<PredicateTail2Syntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithIndicators<WordLike>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum PredicateTail3Syntax {
    Relation {
        relation: RelationSyntax,
        terms: Vec<TermSyntax>,
        vau: Option<WithIndicators<WordLike>>,
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
        tail_terms: Vec<TermSyntax>,
        vau: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Na {
        na: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
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
    pub paragraphs: Vec<ParagraphSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ParagraphSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub niho: Vec<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    pub statements: Vec<ParagraphStatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct ParagraphStatementSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub connective: Option<ConnectiveSyntax>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
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
        text: Box<TextSyntax>,
        toi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Xi {
        xi: WithFreeModifiers<WithIndicators<WordLike>>,
        expression: MathExpressionSyntax,
    },
    Mai {
        number: Vec<WithIndicators<WordLike>>,
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
        text: Box<TextSyntax>,
        tuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(PredicateSyntax),
    Connected {
        i: WithIndicators<WordLike>,
        connective: ConnectiveSyntax,
        leading_statement: Box<StatementSyntax>,
        trailing_statement: Box<StatementSyntax>,
    },
    PreIConnected {
        connective: ConnectiveSyntax,
        i: WithIndicators<WordLike>,
        leading_statement: Box<StatementSyntax>,
        trailing_statement: Box<StatementSyntax>,
    },
    Iau {
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
    Bo {
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum FragmentSyntax {
    // v0 exposes this constructor even though the current grammar produces
    // TermFragment for parsed standalone arguments.
    #[allow(dead_code)]
    Argument(ArgumentSyntax),
    Ek {
        connective: ConnectiveSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Gihek {
        connective: ConnectiveSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Other {
        words: Vec<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    // v0 exposes this constructor for a fragment shape that is currently parsed
    // through VocativeFree when it appears in source text.
    #[allow(dead_code)]
    Vocative {
        vocative_markers: Vec<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
        vocative_argument: Option<ArgumentSyntax>,
        dohu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
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
        subsentence: Box<SubsentenceSyntax>,
        fihau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    SoiAdverbial {
        soi: WithFreeModifiers<WithIndicators<WordLike>>,
        subsentence: Box<SubsentenceSyntax>,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
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
pub struct ArgumentConnectionSyntax {
    pub connective: ConnectiveSyntax,
    pub argument: Box<ArgumentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ArgumentSyntax {
    Quote {
        quote: QuoteSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    MathExpression {
        li: WithIndicators<WordLike>,
        li_free_modifiers: Vec<FreeModifierSyntax>,
        expression: MathExpressionSyntax,
        loho: Option<WithIndicators<WordLike>>,
        loho_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Letter {
        letter: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
        inner_argument: Box<ArgumentSyntax>,
    },
    RelativeClause {
        base_argument: Box<ArgumentSyntax>,
        vuho: Option<WithIndicators<WordLike>>,
        vuho_free_modifiers: Vec<FreeModifierSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Vuho {
        base_argument: Box<ArgumentSyntax>,
        vuho_marker: WithIndicators<WordLike>,
        vuho_free_modifiers: Vec<FreeModifierSyntax>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        connected_argument: Option<ArgumentConnectionSyntax>,
    },
    BridiDescription {
        lohoi: WithIndicators<WordLike>,
        lohoi_free_modifiers: Vec<FreeModifierSyntax>,
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WithIndicators<WordLike>>,
        kuhau_free_modifiers: Vec<FreeModifierSyntax>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Tagged {
        tag_words: Vec<WithIndicators<WordLike>>,
        tag_tense_modal: Option<TenseModalSyntax>,
        tag_fa: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WithIndicators<WordLike>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithIndicators<WordLike>>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithIndicators<WordLike>>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WithFreeModifiers<WithIndicators<WordLike>>,
        wrapper_bo: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        inner_term: Box<TermSyntax>,
        luhu: Option<WithIndicators<WordLike>>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Koha(WithFreeModifiers<WithIndicators<WordLike>>),
    Zohe {
        tag_words: Vec<WithIndicators<WordLike>>,
        maybe_ku: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        lahe: WithFreeModifiers<WithIndicators<WordLike>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithIndicators<WordLike>>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WithIndicators<WordLike>,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WithIndicators<WordLike>>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
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
    },
    Descriptor(DescriptorSyntax),
    ConnectedDescriptor(ConnectedDescriptorSyntax),
    Name {
        la: WithIndicators<WordLike>,
        la_free_modifiers: Vec<FreeModifierSyntax>,
        names: Vec<WithIndicators<WordLike>>,
        name_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Cmevla(WithFreeModifiers<Vec<WithIndicators<WordLike>>>),
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
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Poi {
        poi: WithFreeModifiers<WithIndicators<WordLike>>,
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zihe {
        zihe: WithFreeModifiers<WithIndicators<WordLike>>,
        inner: Box<RelativeClauseSyntax>,
    },
    Connected {
        connective: ConnectiveSyntax,
        inner: Box<RelativeClauseSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct GoiRelativeClauseSyntax {
    pub goi: WithFreeModifiers<WithIndicators<WordLike>>,
    pub argument: ArgumentSyntax,
    pub gehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct SelbriRelativeClauseSyntax {
    pub nohoi: WithFreeModifiers<WithIndicators<WordLike>>,
    pub relation: RelationSyntax,
    pub kuhoi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum QuoteSyntax {
    Lu {
        lu: WithFreeModifiers<WithIndicators<WordLike>>,
        text: TextSyntax,
        lihu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zo {
        zo: WithIndicators<WordLike>,
        word: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    ZohOi {
        zohoi: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    Zoi {
        zoi: WithIndicators<WordLike>,
        opening_delimiter: WithIndicators<WordLike>,
        closing_delimiter: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    // v0 exposes this constructor in the Quote ADT, but current v0 grammar
    // classifies morphology-level LAhO quotes as ZoiQuote.
    #[allow(dead_code)]
    Laho {
        laho: WithIndicators<WordLike>,
        opening_delimiter: WithIndicators<WordLike>,
        closing_delimiter: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    Lohu {
        lohu: WithIndicators<WordLike>,
        quoted_words: Vec<WithIndicators<WordLike>>,
        lehu: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    // v0 exposes this constructor in the Quote ADT; current v0 grammar parses
    // ordinary `me'o` through MathExpressionArgument.
    #[allow(dead_code)]
    Meho {
        meho: WithFreeModifiers<WithIndicators<WordLike>>,
        math_expression: MathExpressionSyntax,
    },
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
pub struct ConnectiveSyntax {
    pub kind: ConnectiveKind,
    pub se: Option<WithIndicators<WordLike>>,
    pub nahe: Option<WithIndicators<WordLike>>,
    pub na: Option<WithIndicators<WordLike>>,
    pub cmavo: Vec<WithIndicators<WordLike>>,
    pub nai: Option<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
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
        number: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum MathExpressionSyntax {
    Number(QuantifierSyntax),
    Letter {
        letter: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
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
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mohe {
        mohe: WithFreeModifiers<WithIndicators<WordLike>>,
        argument: Box<ArgumentSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Johi {
        johi: WithFreeModifiers<WithIndicators<WordLike>>,
        expressions: Vec<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Lahe {
        markers: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
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
        bihe: WithFreeModifiers<WithIndicators<WordLike>>,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    // v0 exposes this constructor in the syntax ADT; the current v0 grammar
    // mostly materializes prefix operator forms as ForethoughtExpression.
    #[allow(dead_code)]
    Unary {
        operator: MathOperatorSyntax,
        inner_expression: Box<MathExpressionSyntax>,
    },
    // v0 exposes this constructor in the syntax ADT; BO grouping is currently
    // represented in connected operands while the full rule audit continues.
    #[allow(dead_code)]
    Bo {
        left_expression: Box<MathExpressionSyntax>,
        operator: MathOperatorSyntax,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum MathOperatorSyntax {
    Vuhu {
        vuhu: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Maho {
        maho: WithFreeModifiers<WithIndicators<WordLike>>,
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WithFreeModifiers<WithIndicators<WordLike>>,
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    // v0 exposes this constructor; parser support is being ported with the
    // operator precedence rules.
    #[allow(dead_code)]
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_operator: Box<MathOperatorSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    // v0 exposes this constructor; parser support is being ported with the
    // operator precedence rules.
    #[allow(dead_code)]
    Bo {
        left_operator: Box<MathOperatorSyntax>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        right_operator: Box<MathOperatorSyntax>,
    },
    // v0 exposes this constructor even though current parser branches produce
    // JohiExpression for the ordinary JOhI operand form.
    #[allow(dead_code)]
    Johi {
        johi: WithFreeModifiers<WithIndicators<WordLike>>,
        expressions: Vec<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    // v0 exposes this constructor for operator slots accepting numeric forms.
    #[allow(dead_code)]
    Number {
        number: Vec<WithIndicators<WordLike>>,
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
        connective: ConnectiveSyntax,
        leading_relation: Box<RelationSyntax>,
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
        inner_relation: Box<RelationSyntax>,
    },
    Base(WithIndicators<WordLike>),
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        relation: Box<RelationSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TenseModal {
        tense_modal: TenseModalSyntax,
        inner_relation: Box<RelationSyntax>,
    },
    Guha {
        guhek: ConnectiveSyntax,
        leading_predicate: Box<PredicateSyntax>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<PredicateSyntax>,
    },
    Abstraction(AbstractionSyntax),
    Compound(Vec<RelationUnitSyntax>),
}

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
    pub number: Vec<WithIndicators<WordLike>>,
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
    pub fiho: WithIndicators<WordLike>,
    pub fiho_free_modifiers: Vec<FreeModifierSyntax>,
    pub relation: RelationSyntax,
    pub fehu: Option<WithIndicators<WordLike>>,
    pub fehu_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum TenseModalSyntax {
    Composite {
        leaves: Vec<WithIndicators<WordLike>>,
        time: Option<TimeTenseSyntax>,
        space: Option<SpaceTenseSyntax>,
        simple: Option<SimpleTenseModalSyntax>,
        interval: Option<IntervalTenseSyntax>,
        zaho: Vec<WithIndicators<WordLike>>,
        caha: Option<WithIndicators<WordLike>>,
        ki: Option<WithIndicators<WordLike>>,
        cuhe: Option<WithIndicators<WordLike>>,
        fiho: Vec<FihoModalSyntax>,
        connectives: Vec<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Pu {
        word: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    PuDistance {
        pu: WithIndicators<WordLike>,
        distance: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    TimeInterval {
        word: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    PuCaha {
        pu: WithIndicators<WordLike>,
        caha: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceDistance {
        word: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceDirection {
        word: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    SpaceMovement {
        mohi: WithIndicators<WordLike>,
        direction: WithIndicators<WordLike>,
        distance: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Simple {
        nahe: Option<WithIndicators<WordLike>>,
        se: Option<WithIndicators<WordLike>>,
        bai: WithIndicators<WordLike>,
        nai: Option<WithIndicators<WordLike>>,
        ki: Option<WithIndicators<WordLike>>,
        connectives: Vec<WithIndicators<WordLike>>,
        extra_leaves: Vec<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ki {
        ki: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Fiho {
        fiho: WithIndicators<WordLike>,
        relation: Box<RelationSyntax>,
        fehu: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Caha {
        word: WithIndicators<WordLike>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Zaho {
        words: Vec<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Interval {
        number: Vec<WithIndicators<WordLike>>,
        roi_or_tahe: WithIndicators<WordLike>,
        nai: Option<WithIndicators<WordLike>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct AbstractionSyntax {
    pub nu: WithIndicators<WordLike>,
    pub nai: Option<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    pub additional_nu: Vec<AdditionalNuSyntax>,
    pub subsentence: Box<SubsentenceSyntax>,
    pub kei: Option<WithIndicators<WordLike>>,
    pub kei_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct AdditionalNuSyntax {
    pub connective: ConnectiveSyntax,
    pub nu: WithIndicators<WordLike>,
    pub nai: Option<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum RelationUnitSyntax {
    Word {
        word: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Goha {
        goha: WithFreeModifiers<WithIndicators<WordLike>>,
        raho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<TenseModalSyntax>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        relation: RelationSyntax,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
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
        base: Box<RelationUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativeClauseSyntax>,
    },
    Wrapped(RelationSyntax),
    Jai {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tense_modal: Option<TenseModalSyntax>,
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
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
        base: Box<RelationUnitSyntax>,
    },
    Abstraction(AbstractionSyntax),
    Me {
        me: WithFreeModifiers<WithIndicators<WordLike>>,
        argument: ArgumentSyntax,
        mehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        moi_marker: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mehoi {
        mehoi: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    Gohoi {
        gohoi: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    Muhoi {
        muhoi: WithIndicators<WordLike>,
        opening_delimiter: WithIndicators<WordLike>,
        closing_delimiter: WithFreeModifiers<WithIndicators<WordLike>>,
        quoted_text: String,
    },
    Luhei {
        luhei: WithFreeModifiers<WithIndicators<WordLike>>,
        text: TextSyntax,
        liau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Moi {
        number: Vec<WithIndicators<WordLike>>,
        moi: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Nuha {
        nuha: WithFreeModifiers<WithIndicators<WordLike>>,
        math_operator: MathOperatorSyntax,
    },
    Xohi {
        xohi: WithFreeModifiers<WithIndicators<WordLike>>,
        tag: TenseModalSyntax,
    },
    Cei {
        base: Box<RelationUnitSyntax>,
        assignments: Vec<CeiAssignmentSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub struct CeiAssignmentSyntax {
    pub cei: WithFreeModifiers<WithIndicators<WordLike>>,
    pub relation_unit: RelationUnitSyntax,
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
                words.extend(tuhe.words());
                words.extend(text.words());
                if let Some(tuhu) = tuhu {
                    words.extend(tuhu.words());
                }
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
                words.extend(zohu.words());
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
                reset_terms,
            } => {
                let mut words = inner_statement.words();
                words.extend(iau.words());
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        match self.marker {
            PredicateStatementContinuationMarkerSyntax::Bo { bo } => {
                words.extend(bo.words());
                words.extend(self.trailing_subsentence.words());
            }
            PredicateStatementContinuationMarkerSyntax::Ke { ke, kehe } => {
                words.extend(ke.words());
                words.extend(self.trailing_subsentence.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
            }
        }
        words
    }
}

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.leading_nai;
        words.extend(self.leading_cmevla);
        for indicator in self.leading_indicators {
            words.extend(indicator.words());
        }
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            FreeModifierSyntax::Sei {
                sei,
                terms,
                cu,
                relation,
                sehu,
            } => {
                let mut words = sei.words();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(cu) = cu {
                    words.extend(cu.words());
                }
                words.extend(relation.words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            FreeModifierSyntax::To { to, text, toi } => {
                let mut words = to.words();
                words.extend(text.words());
                if let Some(toi) = toi {
                    words.extend(toi.words());
                }
                words
            }
            FreeModifierSyntax::Xi { xi, expression } => {
                let mut words = xi.words();
                words.extend(expression.words());
                words
            }
            FreeModifierSyntax::Mai { number, mai } => {
                let mut words = number;
                words.extend(mai.words());
                words
            }
            FreeModifierSyntax::Soi {
                soi,
                leading_argument,
                trailing_argument,
                sehu,
            } => {
                let mut words = soi.words();
                words.extend(leading_argument.words());
                if let Some(argument) = trailing_argument {
                    words.extend(argument.words());
                }
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
                }
                words
            }
            FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
            } => {
                let mut words = vocative_markers.words();
                if let Some(argument) = argument {
                    words.extend(argument.words());
                }
                if let Some(dohu) = dohu {
                    words.extend(dohu.words());
                }
                words
            }
            FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            } => {
                let mut words = lohai.into_iter().collect::<Vec<_>>();
                words.extend(old_words);
                words.extend(sahai);
                words.extend(new_words);
                words.extend(lehai.words());
                words
            }
        }
    }
}

impl PredicateSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = Vec::new();
        for term in self.leading_terms {
            words.extend(term.words());
        }
        if let Some(cu) = self.cu {
            words.extend(cu.words());
        }
        words.extend(self.predicate_tail.words());
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.first.words();
        if let Some(ke_continuation) = self.ke_continuation {
            words.extend(ke_continuation.words());
        }
        words
    }
}

impl KePredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        words.extend(self.ke.words());
        words.extend(self.predicate_tail.words());
        if let Some(kehe) = self.kehe {
            words.extend(kehe.words());
        }
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        words
    }
}

impl PredicateTail1Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.first.words();
        for continuation in self.continuations {
            words.extend(continuation.words());
        }
        words
    }
}

impl PredicateTailContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        if let Some(cu) = self.cu {
            words.extend(cu.words());
        }
        words.extend(self.predicate_tail.words());
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTail2Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.first.words();
        if let Some(bo_continuation) = self.bo_continuation {
            words.extend(bo_continuation.words());
        }
        words
    }
}

impl BoPredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
        }
        words.push(self.bo);
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        if let Some(cu) = self.cu {
            words.extend(cu.words());
        }
        words.extend(self.predicate_tail.words());
        for term in self.tail_terms {
            words.extend(term.words());
        }
        words.extend(self.vau);
        words
    }
}

impl PredicateTail3Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            PredicateTail3Syntax::Relation {
                relation,
                terms,
                vau,
                free_modifiers,
            } => {
                let mut words = relation.words();
                for term in terms {
                    words.extend(term.words());
                }
                words.extend(vau);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            PredicateTail3Syntax::GekSentence(gek_sentence) => gek_sentence.words(),
        }
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
                inner,
                kehe,
            } => {
                let mut words = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(ke.words());
                words.extend(inner.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
                words.extend(zohu.words());
                words.extend(inner_subsentence.words());
                words
            }
        }
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            FragmentSyntax::Argument(argument) => argument.words(),
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
            } => {
                let mut words = vocative_markers;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                if let Some(vocative_argument) = vocative_argument {
                    words.extend(vocative_argument.words());
                }
                if let Some(dohu) = dohu {
                    words.extend(dohu.words());
                }
                words
            }
            FragmentSyntax::Ijek { i, connective } => {
                let mut words = vec![i];
                words.extend(connective.words());
                words
            }
            FragmentSyntax::Prenex { terms, zohu } => {
                let mut words = terms
                    .into_iter()
                    .flat_map(TermSyntax::words)
                    .collect::<Vec<_>>();
                words.extend(zohu.words());
                words
            }
            FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            } => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            FragmentSyntax::BeiLink(bei_only_links) => bei_only_links
                .into_iter()
                .flat_map(BeiLinkSyntax::words)
                .collect(),
            FragmentSyntax::RelativeClause(relative_clauses) => relative_clauses
                .into_iter()
                .flat_map(RelativeClauseSyntax::words)
                .collect(),
            FragmentSyntax::MathExpression(math_expression) => math_expression.words(),
            FragmentSyntax::Term { terms, vau } => {
                let mut words = Vec::new();
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
                words
            }
            FragmentSyntax::Relation(relation) => relation.words(),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            TermSyntax::NuhiTermset {
                nuhi,
                termset,
                nuhu,
            } => {
                let mut words = nuhi.words();
                for term in termset {
                    words.extend(term.words());
                }
                if let Some(nuhu) = nuhu {
                    words.extend(nuhu.words());
                }
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
                let mut words = Vec::new();
                if let Some(nuhi) = m_nuhi {
                    words.extend(nuhi.words());
                }
                words.extend(gek.words());
                for term in terms {
                    words.extend(term.words());
                }
                if let Some(nuhu) = nuhu {
                    words.extend(nuhu.words());
                }
                words.extend(gik.words());
                for term in gik_terms {
                    words.extend(term.words());
                }
                if let Some(nuhu) = gik_nuhu {
                    words.extend(nuhu.words());
                }
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
                words.extend(cehe.words());
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
                words.extend(pehe.words());
                words.extend(connective.words());
                for term in trailing_terms {
                    words.extend(term.words());
                }
                words
            }
            TermSyntax::Argument(argument) => argument.words(),
            TermSyntax::Fa { fa, argument, ku } => {
                let mut words = fa.words();
                words.extend(argument.words());
                if let Some(ku) = ku {
                    words.extend(ku.words());
                }
                words
            }
            TermSyntax::NaKu { na, na_ku } => {
                let mut words = vec![na];
                words.extend(na_ku.words());
                words
            }
            TermSyntax::BareNa(na) => na.words(),
            TermSyntax::NoihaAdverbial {
                noiha,
                tail_elements,
                relation,
                relative_clauses,
                fehu,
            } => {
                let mut words = noiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                if let Some(fehu) = fehu {
                    words.extend(fehu.words());
                }
                words
            }
            TermSyntax::PoihaBrigahi {
                poiha,
                tail_elements,
                relation,
                relative_clauses,
                brigahi_ku,
            } => {
                let mut words = poiha.words();
                for tail_element in tail_elements {
                    words.extend(tail_element.words());
                }
                if let Some(relation) = relation {
                    words.extend(relation.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(brigahi_ku.words());
                words
            }
            TermSyntax::FihoiAdverbial {
                fihoi,
                subsentence,
                fihau,
            } => {
                let mut words = fihoi.words();
                words.extend(subsentence.words());
                if let Some(fihau) = fihau {
                    words.extend(fihau.words());
                }
                words
            }
            TermSyntax::SoiAdverbial {
                soi,
                subsentence,
                sehu,
            } => {
                let mut words = soi.words();
                words.extend(subsentence.words());
                if let Some(sehu) = sehu {
                    words.extend(sehu.words());
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
                words.extend(bo.words());
                words.extend(trailing_term.words());
                words
            }
        }
    }
}

impl MathExpressionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            MathExpressionSyntax::Number(quantifier) => quantifier.words(),
            MathExpressionSyntax::Letter { letter, boi } => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            MathExpressionSyntax::Vei {
                vei,
                inner_expression,
                veho,
            } => {
                let mut words = vei.words();
                words.extend(inner_expression.words());
                if let Some(veho) = veho {
                    words.extend(veho.words());
                }
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
                let mut words = Vec::new();
                if let Some(peho) = peho {
                    words.extend(peho.words());
                }
                words.extend(operator.words());
                for operand in operands {
                    words.extend(operand.words());
                }
                if let Some(kuhe) = kuhe {
                    words.extend(kuhe.words());
                }
                words
            }
            MathExpressionSyntax::ReversePolish {
                fuha,
                operands,
                operators,
            } => {
                let mut words = fuha.words();
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
                let mut words = nihe.words();
                words.extend(relation.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathExpressionSyntax::Mohe {
                mohe,
                argument,
                tehu,
            } => {
                let mut words = mohe.words();
                words.extend(argument.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathExpressionSyntax::Johi {
                johi,
                expressions,
                tehu,
            } => {
                let mut words = johi.words();
                for expression in expressions {
                    words.extend(expression.words());
                }
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathExpressionSyntax::Lahe {
                markers,
                inner_expression,
                luhu,
            } => {
                let mut words = markers.words();
                words.extend(inner_expression.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
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
                words.extend(bihe.words());
                words.extend(operator.words());
                words.extend(right_expression.words());
                words
            }
            MathExpressionSyntax::Unary {
                operator,
                inner_expression,
            } => {
                let mut words = operator.words();
                words.extend(inner_expression.words());
                words
            }
            MathExpressionSyntax::Bo {
                left_expression,
                operator,
                bo,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.extend(operator.words());
                words.extend(bo.words());
                words.extend(right_expression.words());
                words
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            ArgumentSyntax::Quote {
                quote,
                free_modifiers,
            } => {
                let mut words = quote.words();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
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
            ArgumentSyntax::Letter { letter, boi } => {
                let mut words = letter.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
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
            ArgumentSyntax::NaKu { na, ku } => {
                let mut words = vec![na];
                words.extend(ku.words());
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
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = vec![nahe];
                words.extend(bo.words());
                words.extend(inner_argument.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Nahe {
                nahe,
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = nahe.words();
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
                inner_term,
                luhu,
                luhu_free_modifiers,
                ..
            } => {
                let mut words = wrapper.words();
                if let Some(wrapper_bo) = wrapper_bo {
                    words.extend(wrapper_bo.words());
                }
                words.extend(inner_term.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            ArgumentSyntax::Koha(koha) => koha.words(),
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
                relative_clauses,
                inner_argument,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = lahe.words();
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
                trailing_argument,
            } => {
                let mut words = leading_argument.words();
                if let Some(connective) = bo_connective {
                    words.extend(connective.words());
                }
                if let Some(tense_modal) = bo_tense_modal {
                    words.extend(tense_modal.words());
                }
                words.extend(bo.words());
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
            ArgumentSyntax::Descriptor(descriptor) => descriptor.words(),
            ArgumentSyntax::ConnectedDescriptor(connected_descriptor) => {
                connected_descriptor.words()
            }
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
            ArgumentSyntax::Cmevla(cmevla) => cmevla.words(),
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.goi.words();
        words.extend(self.argument.words());
        if let Some(gehu) = self.gehu {
            words.extend(gehu.words());
        }
        words
    }
}

impl SelbriRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.nohoi.words();
        words.extend(self.relation.words());
        if let Some(kuhoi) = self.kuhoi {
            words.extend(kuhoi.words());
        }
        words
    }
}

impl RelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            RelativeClauseSyntax::Goi(relative_clause) => relative_clause.words(),
            RelativeClauseSyntax::Noi {
                noi,
                subsentence,
                kuho,
            } => {
                let mut words = noi.words();
                words.extend(subsentence.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            RelativeClauseSyntax::Poi {
                poi,
                subsentence,
                kuho,
            } => {
                let mut words = poi.words();
                words.extend(subsentence.words());
                if let Some(kuho) = kuho {
                    words.extend(kuho.words());
                }
                words
            }
            RelativeClauseSyntax::Zihe { zihe, inner } => {
                let mut words = zihe.words();
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            QuoteSyntax::Lu { lu, text, lihu } => {
                let mut words = lu.words();
                words.extend(text.words());
                if let Some(lihu) = lihu {
                    words.extend(lihu.words());
                }
                words
            }
            QuoteSyntax::Zo { zo, word } => {
                let mut words = vec![zo];
                words.extend(word.words());
                words
            }
            QuoteSyntax::ZohOi { zohoi, .. } => zohoi.words(),
            QuoteSyntax::Zoi {
                zoi,
                opening_delimiter,
                closing_delimiter,
                ..
            } => {
                let mut words = vec![zoi, opening_delimiter];
                words.extend(closing_delimiter.words());
                words
            }
            QuoteSyntax::Laho {
                laho,
                opening_delimiter,
                closing_delimiter,
                ..
            } => {
                let mut words = vec![laho, opening_delimiter];
                words.extend(closing_delimiter.words());
                words
            }
            QuoteSyntax::Lohu {
                lohu,
                quoted_words,
                lehu,
            } => {
                let mut words = [vec![lohu], quoted_words].concat();
                words.extend(lehu.words());
                words
            }
            QuoteSyntax::Meho {
                meho,
                math_expression,
            } => {
                let mut words = meho.words();
                words.extend(math_expression.words());
                words
            }
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self
            .outer_quantifier
            .into_iter()
            .flat_map(QuantifierSyntax::words)
            .collect::<Vec<_>>();
        if let Some(descriptor) = self.descriptor {
            words.extend(descriptor.words());
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
        if let Some(ku) = self.ku {
            words.extend(ku.words());
        }
        words
    }
}

impl DescriptorHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        self.descriptor.words()
    }
}

impl ConnectedDescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.leading_descriptor_head.words();
        words.extend(self.connective.words());
        words.extend(self.trailing_descriptor_head.words());
        for element in self.tail_elements {
            words.extend(element.words());
        }
        if let Some(relation) = self.relation {
            words.extend(relation.words());
        }
        for relative_clause in self.relative_clauses {
            words.extend(relative_clause.words());
        }
        if let Some(ku) = self.ku {
            words.extend(ku.words());
        }
        words
    }
}

impl ConnectiveSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.bei.words();
        if let Some(fa) = self.fa {
            words.extend(fa.words());
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            QuantifierSyntax::Number { number, boi } => {
                let mut words = number.words();
                if let Some(boi) = boi {
                    words.extend(boi.words());
                }
                words
            }
            QuantifierSyntax::Vei {
                vei,
                math_expression,
                veho,
            } => {
                let mut words = vei.words();
                words.extend(math_expression.words());
                if let Some(veho) = veho {
                    words.extend(veho.words());
                }
                words
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            MathOperatorSyntax::Vuhu { vuhu } => vuhu.words(),
            MathOperatorSyntax::Maho {
                maho,
                math_expression,
                tehu,
            } => {
                let mut words = maho.words();
                words.extend(math_expression.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathOperatorSyntax::Se { se, inner_operator } => {
                let mut words = se.words();
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahe {
                nahe,
                inner_operator,
            } => {
                let mut words = nahe.words();
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahu {
                nahu,
                relation,
                tehu,
            } => {
                let mut words = nahu.words();
                words.extend(relation.words());
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathOperatorSyntax::Ke {
                ke,
                inner_operator,
                kehe,
            } => {
                let mut words = ke.words();
                words.extend(inner_operator.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            MathOperatorSyntax::Bo {
                left_operator,
                bo,
                right_operator,
            } => {
                let mut words = left_operator.words();
                words.extend(bo.words());
                words.extend(right_operator.words());
                words
            }
            MathOperatorSyntax::Johi {
                johi,
                expressions,
                tehu,
            } => {
                let mut words = johi.words();
                for expression in expressions {
                    words.extend(expression.words());
                }
                if let Some(tehu) = tehu {
                    words.extend(tehu.words());
                }
                words
            }
            MathOperatorSyntax::Number { number } => number,
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
                words.extend(co.words());
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
                words.extend(bo.words());
                words.extend(trailing_relation.words());
                words
            }
            RelationSyntax::Na { na, inner_relation } => {
                let mut words = na.words();
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Base(word) => vec![word],
            RelationSyntax::Se { se, inner_relation } => {
                let mut words = se.words();
                words.extend(inner_relation.words());
                words
            }
            RelationSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                let mut words = ke.words();
                words.extend(relation.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
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
            RelationSyntax::Abstraction(abstraction) => abstraction.words(),
            RelationSyntax::Compound(units) => units
                .into_iter()
                .flat_map(RelationUnitSyntax::words)
                .collect(),
        }
    }
}

impl RelationUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            RelationUnitSyntax::Word { word } => word.words(),
            RelationUnitSyntax::Goha { goha, raho } => {
                let mut words = goha.words();
                if let Some(raho) = raho {
                    words.extend(raho.words());
                }
                words
            }
            RelationUnitSyntax::Se { se, inner_unit } => {
                let mut words = se.words();
                words.extend(inner_unit.words());
                words
            }
            RelationUnitSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                let mut words = ke.words();
                words.extend(relation.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
                }
                words
            }
            RelationUnitSyntax::Nahe { nahe, inner_unit } => {
                let mut words = nahe.words();
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
                words.extend(bo.words());
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
            RelationUnitSyntax::Wrapped(relation) => relation.words(),
            RelationUnitSyntax::Jai {
                jai,
                tense_modal,
                inner_unit,
            } => {
                let mut words = jai.words();
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
                words.extend(be.words());
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words
            }
            RelationUnitSyntax::PreposedBe {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
                base,
            } => {
                let mut words = be.words();
                if let Some(fa) = fa {
                    words.extend(fa.words());
                }
                if let Some(first_argument) = first_argument {
                    words.extend(first_argument.words());
                }
                words.extend(bei_links.into_iter().flat_map(BeiLinkSyntax::words));
                if let Some(beho) = beho {
                    words.extend(beho.words());
                }
                words.extend(base.words());
                words
            }
            RelationUnitSyntax::Abstraction(abstraction) => abstraction.words(),
            RelationUnitSyntax::Me {
                me,
                argument,
                mehu,
                moi_marker,
            } => {
                let mut words = me.words();
                words.extend(argument.words());
                if let Some(mehu) = mehu {
                    words.extend(mehu.words());
                }
                if let Some(moi_marker) = moi_marker {
                    words.extend(moi_marker.words());
                }
                words
            }
            RelationUnitSyntax::Mehoi { mehoi, .. } => mehoi.words(),
            RelationUnitSyntax::Gohoi { gohoi, .. } => gohoi.words(),
            RelationUnitSyntax::Muhoi {
                muhoi,
                opening_delimiter,
                closing_delimiter,
                ..
            } => {
                let mut words = vec![muhoi, opening_delimiter];
                words.extend(closing_delimiter.words());
                words
            }
            RelationUnitSyntax::Luhei { luhei, text, liau } => {
                let mut words = luhei.words();
                words.extend(text.words());
                if let Some(liau) = liau {
                    words.extend(liau.words());
                }
                words
            }
            RelationUnitSyntax::Moi { number, moi } => {
                let mut words = number;
                words.extend(moi.words());
                words
            }
            RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            } => {
                let mut words = nuha.words();
                words.extend(math_operator.words());
                words
            }
            RelationUnitSyntax::Xohi { xohi, tag } => {
                let mut words = xohi.words();
                words.extend(tag.words());
                words
            }
            RelationUnitSyntax::Cei { base, assignments } => {
                let mut words = base.words();
                for assignment in assignments {
                    words.extend(assignment.cei.words());
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn leaf_words(self) -> Vec<WithIndicators<WordLike>> {
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
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.clone().leaf_words();
        for free_modifier in self.free_modifiers() {
            words.extend(free_modifier.words());
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn free_modifiers(self) -> Vec<FreeModifierSyntax> {
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
