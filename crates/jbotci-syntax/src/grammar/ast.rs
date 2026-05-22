// The internal syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use crate::{Indicator, WithIndicators};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::WordLike;
use serde::Serialize;
use serde::ser::{SerializeSeq, Serializer};
use vec1::{Vec1, smallvec_v1::SmallVec1};

pub type WordRun = SmallVec1<[WithIndicators<WordLike>; 2]>;
pub type MathExpressionVec = Vec1<MathExpressionSyntax>;

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
    pub fn extend_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        out.push(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        visitor(&self.value);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= 1)]
    pub fn word_count(&self) -> usize {
        1 + self
            .free_modifiers
            .iter()
            .map(FreeModifierSyntax::word_count)
            .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(ret.is_some())]
    pub fn first_word(&self) -> Option<&WithIndicators<WordLike>> {
        Some(&self.value)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

impl WithFreeModifiers<Vec<WithIndicators<WordLike>>> {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        out.extend(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        for word in &self.value {
            visitor(word);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= self.value.len())]
    pub fn word_count(&self) -> usize {
        self.value.len()
            + self
                .free_modifiers
                .iter()
                .map(FreeModifierSyntax::word_count)
                .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn first_word(&self) -> Option<&WithIndicators<WordLike>> {
        self.value.first().or_else(|| {
            self.free_modifiers
                .iter()
                .find_map(FreeModifierSyntax::first_word)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

impl WithFreeModifiers<WordRun> {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        out.extend(self.value);
        for free_modifier in self.free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        for word in &self.value {
            visitor(word);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(ret >= self.value.len())]
    pub fn word_count(&self) -> usize {
        self.value.len()
            + self
                .free_modifiers
                .iter()
                .map(FreeModifierSyntax::word_count)
                .sum::<usize>()
    }

    #[requires(true)]
    #[ensures(ret.is_some())]
    pub fn first_word(&self) -> Option<&WithIndicators<WordLike>> {
        Some(self.value.first())
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }
}

#[requires(true)]
#[ensures(true)]
fn visit_word_slice(
    words: &[WithIndicators<WordLike>],
    visitor: &mut impl FnMut(&WithIndicators<WordLike>),
) {
    for word in words {
        visitor(word);
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
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
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
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
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
    pub bo: WithFreeModifiers<WithIndicators<WordLike>>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub predicate_tail: Box<PredicateTail2Syntax>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum PredicateTail3Syntax {
    Relation {
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
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
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
    JaiTagged {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tag: Option<TenseModalSyntax>,
        argument: ArgumentSyntax,
    },
    Tagged {
        tense_modal: Option<TenseModalSyntax>,
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
    pub argument: Box<ArgumentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum ArgumentSyntax {
    Quote(QuoteSyntax),
    MathExpression {
        li: WithFreeModifiers<WithIndicators<WordLike>>,
        expression: MathExpressionSyntax,
        loho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Letter {
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
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
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Tagged {
        tag: ArgumentTagSyntax,
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WithIndicators<WordLike>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WithFreeModifiers<WithIndicators<WordLike>>,
        wrapper_bo: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
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
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Relation {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PredicateTail {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Forethought {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NonLogical {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Interval {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
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
        number: WithFreeModifiers<WordRun>,
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
        letter: WithFreeModifiers<WordRun>,
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
        expressions: MathExpressionVec,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[invariant(true)]
pub enum MathOperatorSyntax {
    Vuhu(WithFreeModifiers<WithIndicators<WordLike>>),
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
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
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
        connectives: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        extra_leaves: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    },
    Ki(WithFreeModifiers<WithIndicators<WordLike>>),
    Fiho {
        fiho: WithFreeModifiers<WithIndicators<WordLike>>,
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
    Mehoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Gohoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Muhoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Luhei {
        luhei: WithFreeModifiers<WithIndicators<WordLike>>,
        text: TextSyntax,
        liau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Moi {
        number: WordRun,
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
            PredicateStatementContinuationMarkerSyntax::Bo(bo) => {
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
                let mut words = number.into_vec();
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
        if let Some(vau) = self.vau {
            words.extend(vau.words());
        }
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
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
        if let Some(vau) = self.vau {
            words.extend(vau.words());
        }
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
        words.extend(self.bo.words());
        if let Some(cu) = self.cu {
            words.extend(cu.words());
        }
        words.extend(self.predicate_tail.words());
        for term in self.tail_terms {
            words.extend(term.words());
        }
        if let Some(vau) = self.vau {
            words.extend(vau.words());
        }
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
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
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
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
                gihi,
                tail_terms,
                vau,
                free_modifiers,
            } => {
                let mut words = gek.words();
                words.extend(first.words());
                words.extend(gik.words());
                words.extend(second.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                for term in tail_terms {
                    words.extend(term.words());
                }
                if let Some(vau) = vau {
                    words.extend(vau.words());
                }
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
            GekSentenceSyntax::Na { na, inner } => {
                let mut words = na.words();
                words.extend(inner.words());
                words
            }
        }
    }
}

impl SubsentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            SubsentenceSyntax::Plain(predicate) => predicate.visit_words(visitor),
            SubsentenceSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_subsentence,
            } => {
                for term in prenex_terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
                inner_subsentence.visit_words(visitor);
            }
        }
    }
}

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            FragmentSyntax::Ek(connective) | FragmentSyntax::Gihek(connective) => {
                connective.visit_words(visitor);
            }
            FragmentSyntax::Other(words) => words.visit_words(visitor),
            FragmentSyntax::Ijek { i, connective } => {
                visitor(i);
                connective.visit_words(visitor);
            }
            FragmentSyntax::Prenex { terms, zohu } => {
                for term in terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
            }
            FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            } => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            FragmentSyntax::BeiLink(bei_only_links) => {
                for bei_link in bei_only_links {
                    bei_link.visit_words(visitor);
                }
            }
            FragmentSyntax::RelativeClause(relative_clauses) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            FragmentSyntax::MathExpression(math_expression) => math_expression.visit_words(visitor),
            FragmentSyntax::Term { terms, vau } => {
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
            }
            FragmentSyntax::Relation(relation) => relation.visit_words(visitor),
        }
    }
}

impl TermSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            TermSyntax::NuhiTermset {
                nuhi,
                termset,
                nuhu,
            } => {
                nuhi.visit_words(visitor);
                for term in termset {
                    term.visit_words(visitor);
                }
                if let Some(nuhu) = nuhu {
                    nuhu.visit_words(visitor);
                }
            }
            TermSyntax::GekNuhiTermset {
                m_nuhi,
                gek,
                terms,
                nuhu,
                gik,
                gik_terms,
                gihi,
                gik_nuhu,
            } => {
                if let Some(nuhi) = m_nuhi {
                    nuhi.visit_words(visitor);
                }
                gek.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(nuhu) = nuhu {
                    nuhu.visit_words(visitor);
                }
                gik.visit_words(visitor);
                for term in gik_terms {
                    term.visit_words(visitor);
                }
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
                if let Some(nuhu) = gik_nuhu {
                    nuhu.visit_words(visitor);
                }
            }
            TermSyntax::Cehe {
                leading_terms,
                cehe,
                trailing_terms,
            } => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                cehe.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            TermSyntax::Pehe {
                leading_terms,
                pehe,
                connective,
                trailing_terms,
            } => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                pehe.visit_words(visitor);
                connective.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            TermSyntax::Argument(argument) => argument.visit_words(visitor),
            TermSyntax::Fa { fa, argument, ku } => {
                fa.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(ku) = ku {
                    ku.visit_words(visitor);
                }
            }
            TermSyntax::NaKu { na, na_ku } => {
                visitor(na);
                na_ku.visit_words(visitor);
            }
            TermSyntax::BareNa(na) => na.visit_words(visitor),
            TermSyntax::NoihaAdverbial {
                noiha,
                tail_elements,
                relation,
                relative_clauses,
                fehu,
            } => {
                noiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(relation) = relation {
                    relation.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            TermSyntax::PoihaBrigahi {
                poiha,
                tail_elements,
                relation,
                relative_clauses,
                brigahi_ku,
            } => {
                poiha.visit_words(visitor);
                for tail_element in tail_elements {
                    tail_element.visit_words(visitor);
                }
                if let Some(relation) = relation {
                    relation.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                brigahi_ku.visit_words(visitor);
            }
            TermSyntax::FihoiAdverbial {
                fihoi,
                subsentence,
                fihau,
            } => {
                fihoi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(fihau) = fihau {
                    fihau.visit_words(visitor);
                }
            }
            TermSyntax::SoiAdverbial {
                soi,
                subsentence,
                sehu,
            } => {
                soi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            TermSyntax::JaiTagged { jai, tag, argument } => {
                jai.visit_words(visitor);
                if let Some(tag) = tag {
                    tag.visit_words(visitor);
                }
                argument.visit_words(visitor);
            }
            TermSyntax::Tagged {
                tense_modal,
                argument,
            } => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                argument.visit_words(visitor);
            }
            TermSyntax::Connected {
                leading_terms,
                connective,
                trailing_terms,
            } => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                connective.visit_words(visitor);
                for term in trailing_terms {
                    term.visit_words(visitor);
                }
            }
            TermSyntax::BoConnected {
                leading_terms,
                bo_connective,
                tense_modal,
                bo,
                trailing_term,
            } => {
                for term in leading_terms {
                    term.visit_words(visitor);
                }
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_term.visit_words(visitor);
            }
        }
    }
}

impl ArgumentTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            ArgumentTagSyntax::TenseModal(tense_modal) => tense_modal.visit_words(visitor),
            ArgumentTagSyntax::Fa(fa) => fa.visit_words(visitor),
        }
    }
}

impl MathExpressionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            MathExpressionSyntax::Number(quantifier) => quantifier.visit_words(visitor),
            MathExpressionSyntax::Letter { letter, boi } => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Vei {
                vei,
                inner_expression,
                veho,
            } => {
                vei.visit_words(visitor);
                inner_expression.visit_words(visitor);
                if let Some(veho) = veho {
                    veho.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Gek {
                gek,
                left_expression,
                gik,
                right_expression,
            } => {
                gek.visit_words(visitor);
                left_expression.visit_words(visitor);
                gik.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            MathExpressionSyntax::Forethought {
                peho,
                operator,
                operands,
                kuhe,
            } => {
                if let Some(peho) = peho {
                    peho.visit_words(visitor);
                }
                operator.visit_words(visitor);
                for operand in operands {
                    operand.visit_words(visitor);
                }
                if let Some(kuhe) = kuhe {
                    kuhe.visit_words(visitor);
                }
            }
            MathExpressionSyntax::ReversePolish {
                fuha,
                operands,
                operators,
            } => {
                fuha.visit_words(visitor);
                for operand in operands {
                    operand.visit_words(visitor);
                }
                for operator in operators {
                    operator.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Nihe {
                nihe,
                relation,
                tehu,
            } => {
                nihe.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Mohe {
                mohe,
                argument,
                tehu,
            } => {
                mohe.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Johi {
                johi,
                expressions,
                tehu,
            } => {
                johi.visit_words(visitor);
                for expression in expressions {
                    expression.visit_words(visitor);
                }
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Lahe {
                markers,
                inner_expression,
                luhu,
            } => {
                markers.visit_words(visitor);
                inner_expression.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            MathExpressionSyntax::Connected {
                left_expression,
                connective,
                right_expression,
            } => {
                left_expression.visit_words(visitor);
                connective.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            MathExpressionSyntax::Binary {
                operator,
                left_expression,
                right_expression,
            } => {
                left_expression.visit_words(visitor);
                operator.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
            MathExpressionSyntax::Bihe {
                left_expression,
                bihe,
                operator,
                right_expression,
            } => {
                left_expression.visit_words(visitor);
                bihe.visit_words(visitor);
                operator.visit_words(visitor);
                right_expression.visit_words(visitor);
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            ArgumentSyntax::Quote(quote) => quote.visit_words(visitor),
            ArgumentSyntax::MathExpression {
                li,
                expression,
                loho,
            } => {
                li.visit_words(visitor);
                expression.visit_words(visitor);
                if let Some(loho) = loho {
                    loho.visit_words(visitor);
                }
            }
            ArgumentSyntax::Letter { letter, boi } => {
                letter.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            } => {
                quantifier.visit_words(visitor);
                inner_argument.visit_words(visitor);
            }
            ArgumentSyntax::RelativeClause {
                base_argument,
                vuho,
                relative_clauses,
            } => {
                base_argument.visit_words(visitor);
                if let Some(vuho) = vuho {
                    vuho.visit_words(visitor);
                }
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            ArgumentSyntax::Vuho {
                base_argument,
                vuho_marker,
                relative_clauses,
                connected_argument,
            } => {
                base_argument.visit_words(visitor);
                vuho_marker.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                if let Some(connected_argument) = connected_argument {
                    connected_argument.connective.visit_words(visitor);
                    connected_argument.argument.visit_words(visitor);
                }
            }
            ArgumentSyntax::BridiDescription {
                lohoi,
                subsentence,
                kuhau,
            } => {
                lohoi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(kuhau) = kuhau {
                    kuhau.visit_words(visitor);
                }
            }
            ArgumentSyntax::NaKu { na, ku } => {
                visitor(na);
                ku.visit_words(visitor);
            }
            ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            } => {
                tag.visit_words(visitor);
                inner_argument.visit_words(visitor);
            }
            ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument,
                luhu,
            } => {
                visitor(nahe);
                bo.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            ArgumentSyntax::Nahe {
                nahe,
                inner_argument,
                luhu,
            } => {
                nahe.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            ArgumentSyntax::TermWrapped {
                wrapper,
                wrapper_bo,
                inner_term,
                luhu,
                ..
            } => {
                wrapper.visit_words(visitor);
                if let Some(wrapper_bo) = wrapper_bo {
                    wrapper_bo.visit_words(visitor);
                }
                inner_term.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            ArgumentSyntax::Koha(koha) => koha.visit_words(visitor),
            ArgumentSyntax::Zohe {
                tag,
                maybe_ku,
                free_modifiers,
            } => {
                if let Some(tag) = tag {
                    tag.visit_words(visitor);
                }
                if let Some(ku) = maybe_ku {
                    ku.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            ArgumentSyntax::Lahe {
                lahe,
                relative_clauses,
                inner_argument,
                luhu,
            } => {
                lahe.visit_words(visitor);
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                inner_argument.visit_words(visitor);
                if let Some(luhu) = luhu {
                    luhu.visit_words(visitor);
                }
            }
            ArgumentSyntax::Connected {
                leading_argument,
                connective,
                trailing_argument,
            } => {
                leading_argument.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_argument.visit_words(visitor);
            }
            ArgumentSyntax::Ke {
                ke,
                inner_argument,
                kehe,
            } => {
                ke.visit_words(visitor);
                inner_argument.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            ArgumentSyntax::Bo {
                leading_argument,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_argument,
            } => {
                leading_argument.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_argument.visit_words(visitor);
            }
            ArgumentSyntax::Gek {
                gek,
                leading_argument,
                gik,
                trailing_argument,
                gihi,
            } => {
                gek.visit_words(visitor);
                leading_argument.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_argument.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            ArgumentSyntax::Descriptor(descriptor) => descriptor.visit_words(visitor),
            ArgumentSyntax::ConnectedDescriptor(connected_descriptor) => {
                connected_descriptor.visit_words(visitor);
            }
            ArgumentSyntax::Name { la, names } => {
                la.visit_words(visitor);
                names.visit_words(visitor);
            }
            ArgumentSyntax::Cmevla(cmevla) => cmevla.visit_words(visitor),
            ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            } => {
                for relative_clause in leading_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
                relation.visit_words(visitor);
                for relative_clause in trailing_relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
        }
    }
}

impl GoiRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.goi.visit_words(visitor);
        self.argument.visit_words(visitor);
        if let Some(gehu) = &self.gehu {
            gehu.visit_words(visitor);
        }
    }
}

impl SelbriRelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.nohoi.visit_words(visitor);
        self.relation.visit_words(visitor);
        if let Some(kuhoi) = &self.kuhoi {
            kuhoi.visit_words(visitor);
        }
    }
}

impl RelativeClauseSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            RelativeClauseSyntax::Goi(relative_clause) => relative_clause.visit_words(visitor),
            RelativeClauseSyntax::Noi {
                noi,
                subsentence,
                kuho,
            }
            | RelativeClauseSyntax::Poi {
                poi: noi,
                subsentence,
                kuho,
            } => {
                noi.visit_words(visitor);
                subsentence.visit_words(visitor);
                if let Some(kuho) = kuho {
                    kuho.visit_words(visitor);
                }
            }
            RelativeClauseSyntax::Zihe { zihe, inner } => {
                zihe.visit_words(visitor);
                inner.visit_words(visitor);
            }
            RelativeClauseSyntax::Connected { connective, inner } => {
                connective.visit_words(visitor);
                inner.visit_words(visitor);
            }
        }
    }
}

impl QuoteSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            QuoteSyntax::Lu { lu, text, lihu } => {
                lu.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(lihu) = lihu {
                    lihu.visit_words(visitor);
                }
            }
            QuoteSyntax::Zo(zo) | QuoteSyntax::Zoi(zo) => zo.visit_words(visitor),
            QuoteSyntax::ZohOi(zohoi) => zohoi.visit_words(visitor),
            QuoteSyntax::Lohu(lohu) => lohu.visit_words(visitor),
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        if let Some(quantifier) = &self.outer_quantifier {
            quantifier.visit_words(visitor);
        }
        if let Some(descriptor) = &self.descriptor {
            descriptor.visit_words(visitor);
        }
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(relation) = &self.relation {
            relation.visit_words(visitor);
        }
        for relative_clause in &self.relative_clauses {
            relative_clause.visit_words(visitor);
        }
        if let Some(ku) = &self.ku {
            ku.visit_words(visitor);
        }
    }
}

impl DescriptorHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.descriptor.visit_words(visitor);
    }
}

impl ConnectedDescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.leading_descriptor_head.visit_words(visitor);
        self.connective.visit_words(visitor);
        self.trailing_descriptor_head.visit_words(visitor);
        for element in &self.tail_elements {
            element.visit_words(visitor);
        }
        if let Some(relation) = &self.relation {
            relation.visit_words(visitor);
        }
        for relative_clause in &self.relative_clauses {
            relative_clause.visit_words(visitor);
        }
        if let Some(ku) = &self.ku {
            ku.visit_words(visitor);
        }
    }
}

impl ConnectiveSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(
        kind: ConnectiveKind,
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    ) -> Self {
        match kind {
            ConnectiveKind::Afterthought => Self::Afterthought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
            ConnectiveKind::Relation => Self::Relation {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
            ConnectiveKind::PredicateTail => Self::PredicateTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
            ConnectiveKind::Forethought => Self::Forethought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
            ConnectiveKind::NonLogical => Self::NonLogical {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
            ConnectiveKind::Interval => Self::Interval {
                se,
                nahe,
                na,
                cmavo,
                nai,
            },
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> ConnectiveKind {
        match self {
            Self::Afterthought { .. } => ConnectiveKind::Afterthought,
            Self::Relation { .. } => ConnectiveKind::Relation,
            Self::PredicateTail { .. } => ConnectiveKind::PredicateTail,
            Self::Forethought { .. } => ConnectiveKind::Forethought,
            Self::NonLogical { .. } => ConnectiveKind::NonLogical,
            Self::Interval { .. } => ConnectiveKind::Interval,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn cmavo(&self) -> &WithFreeModifiers<Vec<WithIndicators<WordLike>>> {
        match self {
            Self::Afterthought { cmavo, .. }
            | Self::Relation { cmavo, .. }
            | Self::PredicateTail { cmavo, .. }
            | Self::Forethought { cmavo, .. }
            | Self::NonLogical { cmavo, .. }
            | Self::Interval { cmavo, .. } => cmavo,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn into_parts(
        self,
    ) -> (
        ConnectiveKind,
        Option<WithIndicators<WordLike>>,
        Option<WithIndicators<WordLike>>,
        Option<WithIndicators<WordLike>>,
        WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    ) {
        match self {
            Self::Afterthought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::Afterthought, se, nahe, na, cmavo, nai),
            Self::Relation {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::Relation, se, nahe, na, cmavo, nai),
            Self::PredicateTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::PredicateTail, se, nahe, na, cmavo, nai),
            Self::Forethought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::Forethought, se, nahe, na, cmavo, nai),
            Self::NonLogical {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::NonLogical, se, nahe, na, cmavo, nai),
            Self::Interval {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (ConnectiveKind::Interval, se, nahe, na, cmavo, nai),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        let (se, nahe, na, cmavo, nai) = match self {
            Self::Afterthought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }
            | Self::Relation {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }
            | Self::PredicateTail {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }
            | Self::Forethought {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }
            | Self::NonLogical {
                se,
                nahe,
                na,
                cmavo,
                nai,
            }
            | Self::Interval {
                se,
                nahe,
                na,
                cmavo,
                nai,
            } => (se, nahe, na, cmavo, nai),
        };
        if let Some(se) = se {
            visitor(se);
        }
        if let Some(nahe) = nahe {
            visitor(nahe);
        }
        if let Some(na) = na {
            visitor(na);
        }
        cmavo.visit_words(visitor);
        if let Some(nai) = nai {
            nai.visit_words(visitor);
        }
    }
}

impl BeiLinkSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.bei.visit_words(visitor);
        if let Some(fa) = &self.fa {
            fa.visit_words(visitor);
        }
        if let Some(argument) = &self.argument {
            argument.visit_words(visitor);
        }
    }
}

impl ArgumentTailElementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            ArgumentTailElementSyntax::Argument(argument) => argument.visit_words(visitor),
            ArgumentTailElementSyntax::RelativeClauses(relative_clauses) => {
                for relative_clause in relative_clauses {
                    relative_clause.visit_words(visitor);
                }
            }
            ArgumentTailElementSyntax::Quantifier(quantifier) => quantifier.visit_words(visitor),
        }
    }
}

impl QuantifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            QuantifierSyntax::Number { number, boi } => {
                number.visit_words(visitor);
                if let Some(boi) = boi {
                    boi.visit_words(visitor);
                }
            }
            QuantifierSyntax::Vei {
                vei,
                math_expression,
                veho,
            } => {
                vei.visit_words(visitor);
                math_expression.visit_words(visitor);
                if let Some(veho) = veho {
                    veho.visit_words(visitor);
                }
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            MathOperatorSyntax::Vuhu(vuhu) => vuhu.visit_words(visitor),
            MathOperatorSyntax::Maho {
                maho,
                math_expression,
                tehu,
            } => {
                maho.visit_words(visitor);
                math_expression.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            MathOperatorSyntax::Se { se, inner_operator } => {
                se.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            MathOperatorSyntax::Nahe {
                nahe,
                inner_operator,
            } => {
                nahe.visit_words(visitor);
                inner_operator.visit_words(visitor);
            }
            MathOperatorSyntax::Nahu {
                nahu,
                relation,
                tehu,
            } => {
                nahu.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(tehu) = tehu {
                    tehu.visit_words(visitor);
                }
            }
            MathOperatorSyntax::Ke {
                ke,
                inner_operator,
                kehe,
            } => {
                ke.visit_words(visitor);
                inner_operator.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            MathOperatorSyntax::Bo {
                left_operator,
                connective,
                bo,
                right_operator,
            } => {
                left_operator.visit_words(visitor);
                connective.visit_words(visitor);
                bo.visit_words(visitor);
                right_operator.visit_words(visitor);
            }
            MathOperatorSyntax::Connected {
                left_operator,
                connective,
                right_operator,
            } => {
                left_operator.visit_words(visitor);
                connective.visit_words(visitor);
                right_operator.visit_words(visitor);
            }
        }
    }
}

impl RelationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            RelationSyntax::Connected {
                connective,
                leading_relation,
                trailing_relation,
            } => {
                leading_relation.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            RelationSyntax::Co {
                leading_relation,
                co,
                trailing_relation,
            } => {
                leading_relation.visit_words(visitor);
                co.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            RelationSyntax::Bo {
                leading_relation,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_relation,
            } => {
                leading_relation.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_relation.visit_words(visitor);
            }
            RelationSyntax::Na { na, inner_relation } => {
                na.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            RelationSyntax::Base(word) => visitor(word),
            RelationSyntax::Se { se, inner_relation } => {
                se.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            RelationSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                ke.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            } => {
                tense_modal.visit_words(visitor);
                inner_relation.visit_words(visitor);
            }
            RelationSyntax::Guha {
                guhek,
                leading_predicate,
                gik,
                trailing_predicate,
                gihi,
            } => {
                guhek.visit_words(visitor);
                leading_predicate.visit_words(visitor);
                gik.visit_words(visitor);
                trailing_predicate.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
            }
            RelationSyntax::Abstraction(abstraction) => abstraction.visit_words(visitor),
            RelationSyntax::Compound(units) => {
                for unit in units.iter() {
                    unit.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl RelationUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            RelationUnitSyntax::Word(word) => word.visit_words(visitor),
            RelationUnitSyntax::Goha { goha, raho } => {
                goha.visit_words(visitor);
                if let Some(raho) = raho {
                    raho.visit_words(visitor);
                }
            }
            RelationUnitSyntax::Se { se, inner_unit } => {
                se.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            RelationUnitSyntax::Ke {
                ke, relation, kehe, ..
            } => {
                ke.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            RelationUnitSyntax::Nahe { nahe, inner_unit } => {
                nahe.visit_words(visitor);
                inner_unit.visit_words(visitor);
            }
            RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_unit,
            } => {
                leading_unit.visit_words(visitor);
                if let Some(connective) = bo_connective {
                    connective.visit_words(visitor);
                }
                if let Some(tense_modal) = bo_tense_modal {
                    tense_modal.visit_words(visitor);
                }
                bo.visit_words(visitor);
                trailing_unit.visit_words(visitor);
            }
            RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            } => {
                leading_unit.visit_words(visitor);
                connective.visit_words(visitor);
                trailing_unit.visit_words(visitor);
            }
            RelationUnitSyntax::SelbriRelativeClause {
                base,
                selbri_relative_clauses,
            } => {
                base.visit_words(visitor);
                for selbri_relative_clause in selbri_relative_clauses {
                    selbri_relative_clause.visit_words(visitor);
                }
            }
            RelationUnitSyntax::Wrapped(relation) => relation.visit_words(visitor),
            RelationUnitSyntax::Jai {
                jai,
                tense_modal,
                inner_unit,
            } => {
                jai.visit_words(visitor);
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                inner_unit.visit_words(visitor);
            }
            RelationUnitSyntax::Be {
                base,
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            } => {
                base.visit_words(visitor);
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
            }
            RelationUnitSyntax::PreposedBe {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
                base,
            } => {
                be.visit_words(visitor);
                if let Some(fa) = fa {
                    fa.visit_words(visitor);
                }
                if let Some(first_argument) = first_argument {
                    first_argument.visit_words(visitor);
                }
                for bei_link in bei_links {
                    bei_link.visit_words(visitor);
                }
                if let Some(beho) = beho {
                    beho.visit_words(visitor);
                }
                base.visit_words(visitor);
            }
            RelationUnitSyntax::Abstraction(abstraction) => abstraction.visit_words(visitor),
            RelationUnitSyntax::Me {
                me,
                argument,
                mehu,
                moi_marker,
            } => {
                me.visit_words(visitor);
                argument.visit_words(visitor);
                if let Some(mehu) = mehu {
                    mehu.visit_words(visitor);
                }
                if let Some(moi_marker) = moi_marker {
                    moi_marker.visit_words(visitor);
                }
            }
            RelationUnitSyntax::Mehoi(mehoi) => mehoi.visit_words(visitor),
            RelationUnitSyntax::Gohoi(gohoi) => gohoi.visit_words(visitor),
            RelationUnitSyntax::Muhoi(muhoi) => muhoi.visit_words(visitor),
            RelationUnitSyntax::Luhei { luhei, text, liau } => {
                luhei.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(liau) = liau {
                    liau.visit_words(visitor);
                }
            }
            RelationUnitSyntax::Moi { number, moi } => {
                visit_word_slice(number, visitor);
                moi.visit_words(visitor);
            }
            RelationUnitSyntax::Nuha {
                nuha,
                math_operator,
            } => {
                nuha.visit_words(visitor);
                math_operator.visit_words(visitor);
            }
            RelationUnitSyntax::Xohi { xohi, tag } => {
                xohi.visit_words(visitor);
                tag.visit_words(visitor);
            }
            RelationUnitSyntax::Cei { base, assignments } => {
                base.visit_words(visitor);
                for assignment in assignments {
                    assignment.cei.visit_words(visitor);
                    assignment.relation_unit.visit_words(visitor);
                }
            }
        }
    }
}

impl AbstractionSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.nu.visit_words(visitor);
        if let Some(nai) = &self.nai {
            nai.visit_words(visitor);
        }
        for additional_nu in &self.additional_nu {
            additional_nu.visit_words(visitor);
        }
        self.subsentence.visit_words(visitor);
        if let Some(kei) = &self.kei {
            kei.visit_words(visitor);
        }
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.connective.visit_words(visitor);
        self.nu.visit_words(visitor);
        if let Some(nai) = &self.nai {
            nai.visit_words(visitor);
        }
    }
}

impl CompositeTenseModalPartSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_leaf_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        match self {
            CompositeTenseModalPartSyntax::Word(word) => out.push(word),
            CompositeTenseModalPartSyntax::Fiho(fiho) => {
                out.push(fiho.fiho.value);
                out.extend(fiho.relation.words());
                if let Some(fehu) = fiho.fehu {
                    out.push(fehu.value);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            CompositeTenseModalPartSyntax::Word(word) => visitor(word),
            CompositeTenseModalPartSyntax::Fiho(fiho) => {
                fiho.fiho.visit_words(visitor);
                fiho.relation.visit_words(visitor);
                if let Some(fehu) = &fiho.fehu {
                    fehu.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            CompositeTenseModalPartSyntax::Word(word) => vec![word],
            CompositeTenseModalPartSyntax::Fiho(fiho) => {
                let mut words = vec![fiho.fiho.value];
                words.extend(fiho.relation.words());
                if let Some(fehu) = fiho.fehu {
                    words.push(fehu.value);
                }
                words
            }
        }
    }
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        let (leaves, free_modifiers) = self.leaf_words_and_free_modifiers();
        out.extend(leaves);
        for free_modifier in free_modifiers {
            free_modifier.extend_words_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            TenseModalSyntax::Composite { parts } => {
                for part in &parts.value {
                    part.visit_words(visitor);
                }
                for free_modifier in &parts.free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            TenseModalSyntax::Pu(word)
            | TenseModalSyntax::TimeInterval(word)
            | TenseModalSyntax::SpaceDistance(word)
            | TenseModalSyntax::SpaceDirection(word)
            | TenseModalSyntax::Caha(word) => word.visit_words(visitor),
            TenseModalSyntax::PuDistance { pu, distance } => {
                visitor(pu);
                distance.visit_words(visitor);
            }
            TenseModalSyntax::PuCaha { pu, caha } => {
                visitor(pu);
                caha.visit_words(visitor);
            }
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            } => {
                visitor(mohi);
                direction.visit_words(visitor);
                if let Some(distance) = distance {
                    distance.visit_words(visitor);
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
            } => {
                if let Some(nahe) = nahe {
                    nahe.visit_words(visitor);
                }
                if let Some(se) = se {
                    se.visit_words(visitor);
                }
                bai.visit_words(visitor);
                if let Some(nai) = nai {
                    nai.visit_words(visitor);
                }
                if let Some(ki) = ki {
                    ki.visit_words(visitor);
                }
                connectives.visit_words(visitor);
                extra_leaves.visit_words(visitor);
            }
            TenseModalSyntax::Ki(ki) => ki.visit_words(visitor),
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
            } => {
                fiho.visit_words(visitor);
                relation.visit_words(visitor);
                if let Some(fehu) = fehu {
                    fehu.visit_words(visitor);
                }
            }
            TenseModalSyntax::Zaho(words) => words.visit_words(visitor),
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
            } => {
                if let Some(number) = number {
                    visit_word_slice(number, visitor);
                }
                roi_or_tahe.visit_words(visitor);
                if let Some(nai) = nai {
                    nai.visit_words(visitor);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
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
            FragmentSyntax::Ek(connective) | FragmentSyntax::Gihek(connective) => {
                connective.words()
            }
            FragmentSyntax::Other(words) => words.words(),
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
                gihi,
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
                if let Some(gihi) = gihi {
                    words.push(gihi);
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
            TermSyntax::JaiTagged { jai, tag, argument } => {
                let mut words = jai.words();
                if let Some(tag) = tag {
                    words.extend(tag.words());
                }
                words.extend(argument.words());
                words
            }
            TermSyntax::Tagged {
                tense_modal,
                argument,
            } => {
                let mut words = tense_modal
                    .into_iter()
                    .flat_map(TenseModalSyntax::words)
                    .collect::<Vec<_>>();
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

impl ArgumentTagSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            ArgumentTagSyntax::TenseModal(tense_modal) => tense_modal.words(),
            ArgumentTagSyntax::Fa(fa) => fa.words(),
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
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        match self {
            ArgumentSyntax::Quote(quote) => quote.words(),
            ArgumentSyntax::MathExpression {
                li,
                expression,
                loho,
            } => {
                let mut words = li.words();
                words.extend(expression.words());
                if let Some(loho) = loho {
                    words.extend(loho.words());
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
                relative_clauses,
            } => {
                let mut words = base_argument.words();
                if let Some(vuho) = vuho {
                    words.extend(vuho.words());
                }
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words
            }
            ArgumentSyntax::Vuho {
                base_argument,
                vuho_marker,
                relative_clauses,
                connected_argument,
            } => {
                let mut words = base_argument.words();
                words.extend(vuho_marker.words());
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
                subsentence,
                kuhau,
            } => {
                let mut words = lohoi.words();
                words.extend(subsentence.words());
                if let Some(kuhau) = kuhau {
                    words.extend(kuhau.words());
                }
                words
            }
            ArgumentSyntax::NaKu { na, ku } => {
                let mut words = vec![na];
                words.extend(ku.words());
                words
            }
            ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            } => {
                let mut words = tag.words();
                words.extend(inner_argument.words());
                words
            }
            ArgumentSyntax::NaheBo {
                nahe,
                bo,
                inner_argument,
                luhu,
            } => {
                let mut words = vec![nahe];
                words.extend(bo.words());
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            ArgumentSyntax::Nahe {
                nahe,
                inner_argument,
                luhu,
            } => {
                let mut words = nahe.words();
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            ArgumentSyntax::TermWrapped {
                wrapper,
                wrapper_bo,
                inner_term,
                luhu,
                ..
            } => {
                let mut words = wrapper.words();
                if let Some(wrapper_bo) = wrapper_bo {
                    words.extend(wrapper_bo.words());
                }
                words.extend(inner_term.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
                }
                words
            }
            ArgumentSyntax::Koha(koha) => koha.words(),
            ArgumentSyntax::Zohe {
                tag,
                maybe_ku,
                free_modifiers,
            } => {
                let mut words = tag
                    .into_iter()
                    .flat_map(ArgumentTagSyntax::words)
                    .collect::<Vec<_>>();
                if let Some(ku) = maybe_ku {
                    words.extend(ku.words());
                }
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
            } => {
                let mut words = lahe.words();
                for relative_clause in relative_clauses {
                    words.extend(relative_clause.words());
                }
                words.extend(inner_argument.words());
                if let Some(luhu) = luhu {
                    words.extend(luhu.words());
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
                inner_argument,
                kehe,
            } => {
                let mut words = ke.words();
                words.extend(inner_argument.words());
                if let Some(kehe) = kehe {
                    words.extend(kehe.words());
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
                gihi,
            } => {
                let mut words = gek.words();
                words.extend(leading_argument.words());
                words.extend(gik.words());
                words.extend(trailing_argument.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            ArgumentSyntax::Descriptor(descriptor) => descriptor.words(),
            ArgumentSyntax::ConnectedDescriptor(connected_descriptor) => {
                connected_descriptor.words()
            }
            ArgumentSyntax::Name { la, names } => {
                let mut words = la.words();
                words.extend(names.words());
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
            QuoteSyntax::Zo(zo) | QuoteSyntax::Zoi(zo) => zo.words(),
            QuoteSyntax::ZohOi(zohoi) => zohoi.words(),
            QuoteSyntax::Lohu(lohu) => lohu.words(),
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
        let (_, se, nahe, na, cmavo, nai) = self.into_parts();
        let mut words = Vec::new();
        if let Some(se) = se {
            words.push(se);
        }
        if let Some(nahe) = nahe {
            words.push(nahe);
        }
        if let Some(na) = na {
            words.push(na);
        }
        cmavo.extend_words_into(&mut words);
        if let Some(nai) = nai {
            nai.extend_words_into(&mut words);
        }
        words
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
            MathOperatorSyntax::Vuhu(vuhu) => vuhu.words(),
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
                connective,
                bo,
                right_operator,
            } => {
                let mut words = left_operator.words();
                words.extend(connective.words());
                words.extend(bo.words());
                words.extend(right_operator.words());
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
                gihi,
            } => {
                let mut words = guhek.words();
                words.extend(leading_predicate.words());
                words.extend(gik.words());
                words.extend(trailing_predicate.words());
                if let Some(gihi) = gihi {
                    words.push(gihi);
                }
                words
            }
            RelationSyntax::Abstraction(abstraction) => abstraction.words(),
            RelationSyntax::Compound(units) => units
                .into_smallvec()
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
            RelationUnitSyntax::Word(word) => word.words(),
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
            RelationUnitSyntax::Mehoi(mehoi) => mehoi.words(),
            RelationUnitSyntax::Gohoi(gohoi) => gohoi.words(),
            RelationUnitSyntax::Muhoi(muhoi) => muhoi.words(),
            RelationUnitSyntax::Luhei { luhei, text, liau } => {
                let mut words = luhei.words();
                words.extend(text.words());
                if let Some(liau) = liau {
                    words.extend(liau.words());
                }
                words
            }
            RelationUnitSyntax::Moi { number, moi } => {
                let mut words = number.into_vec();
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
        let mut words = self.nu.words();
        if let Some(nai) = self.nai {
            words.extend(nai.words());
        }
        for additional_nu in self.additional_nu {
            words.extend(additional_nu.words());
        }
        words.extend((*self.subsentence).words());
        if let Some(kei) = self.kei {
            words.extend(kei.words());
        }
        words
    }
}

impl AdditionalNuSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = self.connective.words();
        words.extend(self.nu.words());
        if let Some(nai) = self.nai {
            words.extend(nai.words());
        }
        words
    }
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn free_modifier_count(&self) -> usize {
        match self {
            TenseModalSyntax::Composite { parts } => parts.free_modifiers.len(),
            TenseModalSyntax::Pu(word)
            | TenseModalSyntax::TimeInterval(word)
            | TenseModalSyntax::SpaceDistance(word)
            | TenseModalSyntax::SpaceDirection(word)
            | TenseModalSyntax::Caha(word) => word.free_modifiers.len(),
            TenseModalSyntax::PuDistance { distance, .. } => distance.free_modifiers.len(),
            TenseModalSyntax::PuCaha { caha, .. } => caha.free_modifiers.len(),
            TenseModalSyntax::SpaceMovement {
                direction,
                distance,
                ..
            } => distance
                .as_ref()
                .map_or(direction.free_modifiers.len(), |distance| {
                    distance.free_modifiers.len()
                }),
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
            } => {
                if !extra_leaves.value.is_empty() {
                    extra_leaves.free_modifiers.len()
                } else if !connectives.value.is_empty() {
                    connectives.free_modifiers.len()
                } else if let Some(ki) = ki {
                    ki.free_modifiers.len()
                } else if let Some(nai) = nai {
                    nai.free_modifiers.len()
                } else if !bai.free_modifiers.is_empty() {
                    bai.free_modifiers.len()
                } else if let Some(se) = se {
                    se.free_modifiers.len()
                } else if let Some(nahe) = nahe {
                    nahe.free_modifiers.len()
                } else {
                    bai.free_modifiers.len()
                }
            }
            TenseModalSyntax::Ki(ki) => ki.free_modifiers.len(),
            TenseModalSyntax::Fiho { fiho, fehu, .. } => fehu
                .as_ref()
                .map_or(fiho.free_modifiers.len(), |fehu| fehu.free_modifiers.len()),
            TenseModalSyntax::Zaho(words) => words.free_modifiers.len(),
            TenseModalSyntax::Interval {
                roi_or_tahe, nai, ..
            } => nai
                .as_ref()
                .map_or(roi_or_tahe.free_modifiers.len(), |nai| {
                    nai.free_modifiers.len()
                }),
        }
    }

    #[requires(true)]
    #[ensures(ret.1.len() == old(self.free_modifier_count()))]
    pub fn leaf_words_and_free_modifiers(
        self,
    ) -> (Vec<WithIndicators<WordLike>>, Vec<FreeModifierSyntax>) {
        match self {
            TenseModalSyntax::Composite { parts } => {
                let mut words = Vec::new();
                for part in parts.value {
                    part.extend_leaf_words_into(&mut words);
                }
                (words, parts.free_modifiers)
            }
            TenseModalSyntax::Pu(word) | TenseModalSyntax::Caha(word) => {
                (vec![word.value], word.free_modifiers)
            }
            TenseModalSyntax::PuDistance { pu, distance } => {
                (vec![pu, distance.value], distance.free_modifiers)
            }
            TenseModalSyntax::TimeInterval(word) => (vec![word.value], word.free_modifiers),
            TenseModalSyntax::PuCaha { pu, caha } => (vec![pu, caha.value], caha.free_modifiers),
            TenseModalSyntax::SpaceDistance(word) => (vec![word.value], word.free_modifiers),
            TenseModalSyntax::SpaceDirection(word) => (vec![word.value], word.free_modifiers),
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            } => {
                let mut words = vec![mohi, direction.value];
                let mut free_modifiers = direction.free_modifiers;
                if let Some(distance) = distance {
                    words.push(distance.value);
                    free_modifiers = distance.free_modifiers;
                }
                (words, free_modifiers)
            }
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
            } => {
                let mut words = Vec::new();
                let nahe_is_some = nahe.is_some();
                let nahe_free_modifiers = if let Some(nahe) = nahe {
                    words.push(nahe.value);
                    nahe.free_modifiers
                } else {
                    Vec::new()
                };
                let se_is_some = se.is_some();
                let se_free_modifiers = if let Some(se) = se {
                    words.push(se.value);
                    se.free_modifiers
                } else {
                    Vec::new()
                };
                words.push(bai.value);
                let bai_free_modifiers = bai.free_modifiers;
                let nai_is_some = nai.is_some();
                let nai_free_modifiers = if let Some(nai) = nai {
                    words.push(nai.value);
                    nai.free_modifiers
                } else {
                    Vec::new()
                };
                let ki_is_some = ki.is_some();
                let ki_free_modifiers = if let Some(ki) = ki {
                    words.push(ki.value);
                    ki.free_modifiers
                } else {
                    Vec::new()
                };
                let connectives_have_words = !connectives.value.is_empty();
                words.extend(connectives.value);
                let connectives_free_modifiers = connectives.free_modifiers;
                let extra_leaves_have_words = !extra_leaves.value.is_empty();
                words.extend(extra_leaves.value);
                let extra_leaves_free_modifiers = extra_leaves.free_modifiers;
                let free_modifiers = if extra_leaves_have_words {
                    extra_leaves_free_modifiers
                } else if connectives_have_words {
                    connectives_free_modifiers
                } else if ki_is_some {
                    ki_free_modifiers
                } else if nai_is_some {
                    nai_free_modifiers
                } else if !bai_free_modifiers.is_empty() {
                    bai_free_modifiers
                } else if se_is_some {
                    se_free_modifiers
                } else if nahe_is_some {
                    nahe_free_modifiers
                } else {
                    bai_free_modifiers
                };
                (words, free_modifiers)
            }
            TenseModalSyntax::Ki(ki) => (vec![ki.value], ki.free_modifiers),
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
            } => {
                let mut words = vec![fiho.value];
                let mut free_modifiers = fiho.free_modifiers;
                words.extend((*relation).words());
                if let Some(fehu) = fehu {
                    words.push(fehu.value);
                    free_modifiers = fehu.free_modifiers;
                }
                (words, free_modifiers)
            }
            TenseModalSyntax::Zaho(words) => (words.value, words.free_modifiers),
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
            } => {
                let mut words = number.map_or_else(Vec::new, WordRun::into_vec);
                words.push(roi_or_tahe.value);
                let mut free_modifiers = roi_or_tahe.free_modifiers;
                if let Some(nai) = nai {
                    words.push(nai.value);
                    free_modifiers = nai.free_modifiers;
                }
                (words, free_modifiers)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn leaf_words(self) -> Vec<WithIndicators<WordLike>> {
        self.leaf_words_and_free_modifiers().0
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(self) -> Vec<WithIndicators<WordLike>> {
        let mut words = Vec::new();
        self.extend_words_into(&mut words);
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn free_modifiers(self) -> Vec<FreeModifierSyntax> {
        self.leaf_words_and_free_modifiers().1
    }
}

impl TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        visit_word_slice(&self.leading_nai, visitor);
        visit_word_slice(&self.leading_cmevla, visitor);
        for indicator in &self.leading_indicators {
            indicator.visit_words(visitor);
        }
        for free_modifier in &self.leading_free_modifiers {
            free_modifier.visit_words(visitor);
        }
        if let Some(leading_connective) = &self.leading_connective {
            leading_connective.visit_words(visitor);
        }
        for paragraph in &self.paragraphs {
            paragraph.visit_words(visitor);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl ParagraphSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        if let Some(i) = &self.i {
            visitor(i);
        }
        visit_word_slice(&self.niho, visitor);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
        for paragraph_statement in &self.statements {
            paragraph_statement.visit_words(visitor);
        }
    }
}

impl ParagraphStatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        if let Some(i) = &self.i {
            visitor(i);
        }
        if let Some(connective) = &self.connective {
            connective.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
        if let Some(statement) = &self.statement {
            statement.visit_words(visitor);
        }
    }
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            StatementSyntax::Tuhe {
                tense_modal,
                tuhe,
                text,
                tuhu,
            } => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                tuhe.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(tuhu) = tuhu {
                    tuhu.visit_words(visitor);
                }
            }
            StatementSyntax::Prenex {
                prenex_terms,
                zohu,
                inner_statement,
            } => {
                for term in prenex_terms {
                    term.visit_words(visitor);
                }
                zohu.visit_words(visitor);
                inner_statement.visit_words(visitor);
            }
            StatementSyntax::Predicate(predicate) => predicate.visit_words(visitor),
            StatementSyntax::Connected {
                i,
                connective,
                leading_statement,
                trailing_statement,
            } => {
                leading_statement.visit_words(visitor);
                visitor(i);
                connective.visit_words(visitor);
                trailing_statement.visit_words(visitor);
            }
            StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement,
                trailing_statement,
            } => {
                leading_statement.visit_words(visitor);
                connective.visit_words(visitor);
                visitor(i);
                trailing_statement.visit_words(visitor);
            }
            StatementSyntax::Iau {
                inner_statement,
                iau,
                reset_terms,
            } => {
                inner_statement.visit_words(visitor);
                iau.visit_words(visitor);
                for term in reset_terms {
                    term.visit_words(visitor);
                }
            }
            StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            } => {
                leading_statement.visit_words(visitor);
                continuation.visit_words(visitor);
            }
            StatementSyntax::Fragment(fragment) => fragment.visit_words(visitor),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }
}

impl PredicateStatementContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        match &self.marker {
            PredicateStatementContinuationMarkerSyntax::Bo(bo) => {
                bo.visit_words(visitor);
                self.trailing_subsentence.visit_words(visitor);
            }
            PredicateStatementContinuationMarkerSyntax::Ke { ke, kehe } => {
                ke.visit_words(visitor);
                self.trailing_subsentence.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
        }
    }
}

impl FreeModifierSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn extend_words_into(self, out: &mut Vec<WithIndicators<WordLike>>) {
        out.extend(self.words());
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            FreeModifierSyntax::Sei {
                sei,
                terms,
                cu,
                relation,
                sehu,
            } => {
                sei.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(cu) = cu {
                    cu.visit_words(visitor);
                }
                relation.visit_words(visitor);
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            FreeModifierSyntax::To { to, text, toi } => {
                to.visit_words(visitor);
                text.visit_words(visitor);
                if let Some(toi) = toi {
                    toi.visit_words(visitor);
                }
            }
            FreeModifierSyntax::Xi { xi, expression } => {
                xi.visit_words(visitor);
                expression.visit_words(visitor);
            }
            FreeModifierSyntax::Mai { number, mai } => {
                visit_word_slice(number, visitor);
                mai.visit_words(visitor);
            }
            FreeModifierSyntax::Soi {
                soi,
                leading_argument,
                trailing_argument,
                sehu,
            } => {
                soi.visit_words(visitor);
                leading_argument.visit_words(visitor);
                if let Some(argument) = trailing_argument {
                    argument.visit_words(visitor);
                }
                if let Some(sehu) = sehu {
                    sehu.visit_words(visitor);
                }
            }
            FreeModifierSyntax::Vocative {
                vocative_markers,
                argument,
                dohu,
            } => {
                vocative_markers.visit_words(visitor);
                if let Some(argument) = argument {
                    argument.visit_words(visitor);
                }
                if let Some(dohu) = dohu {
                    dohu.visit_words(visitor);
                }
            }
            FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            } => {
                if let Some(lohai) = lohai {
                    visitor(lohai);
                }
                visit_word_slice(old_words, visitor);
                if let Some(sahai) = sahai {
                    visitor(sahai);
                }
                visit_word_slice(new_words, visitor);
                lehai.visit_words(visitor);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn word_count(&self) -> usize {
        let mut count = 0;
        self.visit_words(&mut |_| count += 1);
        count
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn first_word(&self) -> Option<&WithIndicators<WordLike>> {
        match self {
            FreeModifierSyntax::Sei { sei, .. } => sei.first_word(),
            FreeModifierSyntax::To { to, .. } => to.first_word(),
            FreeModifierSyntax::Xi { xi, .. } => xi.first_word(),
            FreeModifierSyntax::Mai { number, .. } => Some(number.first()),
            FreeModifierSyntax::Soi { soi, .. } => soi.first_word(),
            FreeModifierSyntax::Vocative {
                vocative_markers, ..
            } => vocative_markers.first_word(),
            FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
            } => lohai
                .as_ref()
                .or_else(|| old_words.first())
                .or(sahai.as_ref())
                .or_else(|| new_words.first())
                .or_else(|| lehai.first_word()),
        }
    }
}

impl PredicateSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        for term in &self.leading_terms {
            term.visit_words(visitor);
        }
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.first.visit_words(visitor);
        if let Some(ke_continuation) = &self.ke_continuation {
            ke_continuation.visit_words(visitor);
        }
    }
}

impl KePredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        self.ke.visit_words(visitor);
        self.predicate_tail.visit_words(visitor);
        if let Some(kehe) = &self.kehe {
            kehe.visit_words(visitor);
        }
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail1Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.first.visit_words(visitor);
        for continuation in &self.continuations {
            continuation.visit_words(visitor);
        }
    }
}

impl PredicateTailContinuationSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail2Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.first.visit_words(visitor);
        if let Some(bo_continuation) = &self.bo_continuation {
            bo_continuation.visit_words(visitor);
        }
    }
}

impl BoPredicateTailSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        self.connective.visit_words(visitor);
        if let Some(tense_modal) = &self.tense_modal {
            tense_modal.visit_words(visitor);
        }
        self.bo.visit_words(visitor);
        if let Some(cu) = &self.cu {
            cu.visit_words(visitor);
        }
        self.predicate_tail.visit_words(visitor);
        for term in &self.tail_terms {
            term.visit_words(visitor);
        }
        if let Some(vau) = &self.vau {
            vau.visit_words(visitor);
        }
        for free_modifier in &self.free_modifiers {
            free_modifier.visit_words(visitor);
        }
    }
}

impl PredicateTail3Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            PredicateTail3Syntax::Relation {
                relation,
                terms,
                vau,
                free_modifiers,
            } => {
                relation.visit_words(visitor);
                for term in terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            PredicateTail3Syntax::GekSentence(gek_sentence) => gek_sentence.visit_words(visitor),
        }
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        match self {
            GekSentenceSyntax::Pair {
                gek,
                first,
                gik,
                second,
                gihi,
                tail_terms,
                vau,
                free_modifiers,
            } => {
                gek.visit_words(visitor);
                first.visit_words(visitor);
                gik.visit_words(visitor);
                second.visit_words(visitor);
                if let Some(gihi) = gihi {
                    visitor(gihi);
                }
                for term in tail_terms {
                    term.visit_words(visitor);
                }
                if let Some(vau) = vau {
                    vau.visit_words(visitor);
                }
                for free_modifier in free_modifiers {
                    free_modifier.visit_words(visitor);
                }
            }
            GekSentenceSyntax::Ke {
                tense_modal,
                ke,
                inner,
                kehe,
            } => {
                if let Some(tense_modal) = tense_modal {
                    tense_modal.visit_words(visitor);
                }
                ke.visit_words(visitor);
                inner.visit_words(visitor);
                if let Some(kehe) = kehe {
                    kehe.visit_words(visitor);
                }
            }
            GekSentenceSyntax::Na { na, inner } => {
                na.visit_words(visitor);
                inner.visit_words(visitor);
            }
        }
    }
}
