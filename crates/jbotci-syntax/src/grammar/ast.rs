// The internal syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::WordWithModifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PredicateSyntax {
    pub(super) leading_terms: Vec<TermSyntax>,
    pub(super) cu: Option<WordWithModifiers>,
    pub(super) cu_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) predicate_tail: PredicateTailSyntax,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PredicateTailSyntax {
    pub(super) first: PredicateTail1Syntax,
    pub(super) ke_continuation: Option<KePredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct KePredicateTailSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) tense_modal: Option<TenseModalSyntax>,
    pub(super) ke: WordWithModifiers,
    pub(super) ke_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) predicate_tail: Box<PredicateTailSyntax>,
    pub(super) kehe: Option<WordWithModifiers>,
    pub(super) kehe_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) tail_terms: Vec<TermSyntax>,
    pub(super) vau: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PredicateTail1Syntax {
    pub(super) first: PredicateTail2Syntax,
    pub(super) continuations: Vec<PredicateTailContinuationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PredicateTailContinuationSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) tense_modal: Option<TenseModalSyntax>,
    pub(super) cu: Option<WordWithModifiers>,
    pub(super) cu_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) predicate_tail: PredicateTail2Syntax,
    pub(super) tail_terms: Vec<TermSyntax>,
    pub(super) vau: Option<WordWithModifiers>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct PredicateTail2Syntax {
    pub(super) first: PredicateTail3Syntax,
    pub(super) bo_continuation: Option<BoPredicateTailSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct BoPredicateTailSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) tense_modal: Option<TenseModalSyntax>,
    pub(super) bo: WordWithModifiers,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) cu: Option<WordWithModifiers>,
    pub(super) cu_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) predicate_tail: Box<PredicateTail2Syntax>,
    pub(super) tail_terms: Vec<TermSyntax>,
    pub(super) vau: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum PredicateTail3Syntax {
    Relation {
        relation: RelationSyntax,
        terms: Vec<TermSyntax>,
        vau: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    GekSentence {
        gek_sentence: GekSentenceSyntax,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum GekSentenceSyntax {
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
pub(super) enum SubsentenceSyntax {
    Plain(PredicateSyntax),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WordWithModifiers,
        zohu_free_modifiers: Vec<FreeModifierSyntax>,
        inner_subsentence: Box<SubsentenceSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct TextSyntax {
    pub(super) leading_nai: Vec<WordWithModifiers>,
    pub(super) leading_cmevla: Vec<WordWithModifiers>,
    pub(super) leading_indicators: Vec<WordWithModifiers>,
    pub(super) leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) leading_connective: Option<ConnectiveSyntax>,
    pub(super) paragraphs: Vec<ParagraphSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct ParagraphSyntax {
    pub(super) i: Option<WordWithModifiers>,
    pub(super) niho: Vec<WordWithModifiers>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) statements: Vec<ParagraphStatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct ParagraphStatementSyntax {
    pub(super) i: Option<WordWithModifiers>,
    pub(super) connective: Option<ConnectiveSyntax>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) statement: Option<StatementSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum FreeModifierSyntax {
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
    Replacement {
        lohai: Option<WordWithModifiers>,
        old_words: Vec<WordWithModifiers>,
        sahai: Option<WordWithModifiers>,
        new_words: Vec<WordWithModifiers>,
        lehai: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum StatementSyntax {
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
    Predicate(PredicateSyntax),
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
pub(super) struct PredicateStatementContinuationSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) tense_modal: Option<TenseModalSyntax>,
    pub(super) marker: PredicateStatementContinuationMarkerSyntax,
    pub(super) trailing_subsentence: SubsentenceSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum PredicateStatementContinuationMarkerSyntax {
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
pub(super) enum FragmentSyntax {
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
pub(super) enum TermSyntax {
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
pub(super) enum TermWrapperKindSyntax {
    Lahe,
    NaheBo,
    Nahe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct ArgumentConnectionSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) argument: Box<ArgumentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum ArgumentSyntax {
    Quote {
        quote: QuoteSyntax,
        free_modifiers: Vec<FreeModifierSyntax>,
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
    ConnectedDescriptor {
        connected_descriptor: ConnectedDescriptorSyntax,
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
pub(super) enum RelativeClauseSyntax {
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
pub(super) struct GoiRelativeClauseSyntax {
    pub(super) goi: WordWithModifiers,
    pub(super) leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) argument: ArgumentSyntax,
    pub(super) gehu: Option<WordWithModifiers>,
    pub(super) trailing_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct SelbriRelativeClauseSyntax {
    pub(super) nohoi: WordWithModifiers,
    pub(super) leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) relation: RelationSyntax,
    pub(super) kuhoi: Option<WordWithModifiers>,
    pub(super) trailing_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum QuoteSyntax {
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
    // v0 exposes this constructor in the Quote ADT, but current v0 grammar
    // classifies morphology-level LAhO quotes as ZoiQuote.
    #[allow(dead_code)]
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
    // v0 exposes this constructor in the Quote ADT; current v0 grammar parses
    // ordinary `me'o` through MathExpressionArgument.
    #[allow(dead_code)]
    Meho {
        meho: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        math_expression: MathExpressionSyntax,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct DescriptorSyntax {
    pub(super) descriptor: Option<WordWithModifiers>,
    pub(super) descriptor_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) outer_quantifier: Option<QuantifierSyntax>,
    pub(super) tail_elements: Vec<ArgumentTailElementSyntax>,
    pub(super) relation: Option<RelationSyntax>,
    pub(super) relative_clauses: Vec<RelativeClauseSyntax>,
    pub(super) ku: Option<WordWithModifiers>,
    pub(super) ku_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct DescriptorHeadSyntax {
    pub(super) descriptor: WordWithModifiers,
    pub(super) descriptor_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct ConnectedDescriptorSyntax {
    pub(super) leading_descriptor_head: DescriptorHeadSyntax,
    pub(super) connective: ConnectiveSyntax,
    pub(super) trailing_descriptor_head: DescriptorHeadSyntax,
    pub(super) tail_elements: Vec<ArgumentTailElementSyntax>,
    pub(super) relation: Option<RelationSyntax>,
    pub(super) relative_clauses: Vec<RelativeClauseSyntax>,
    pub(super) ku: Option<WordWithModifiers>,
    pub(super) ku_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct ConnectiveSyntax {
    pub(super) kind: ConnectiveKind,
    pub(super) se: Option<WordWithModifiers>,
    pub(super) nahe: Option<WordWithModifiers>,
    pub(super) na: Option<WordWithModifiers>,
    pub(super) cmavo: Vec<WordWithModifiers>,
    pub(super) nai: Option<WordWithModifiers>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct BeiLinkSyntax {
    pub(super) bei: WordWithModifiers,
    pub(super) bei_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) fa: Option<WordWithModifiers>,
    pub(super) fa_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkArgumentSyntax {
    pub(super) fa: Option<WordWithModifiers>,
    pub(super) fa_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) argument: Option<ArgumentSyntax>,
}

#[invariant(self.fa.is_none() || self.first_argument.is_some(), "lifted FA link tags must have an argument")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BeLinkSyntax {
    pub(super) be: WordWithModifiers,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) fa: Option<WordWithModifiers>,
    pub(super) fa_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) first_argument: Option<ArgumentSyntax>,
    pub(super) bei_links: Vec<BeiLinkSyntax>,
    pub(super) beho: Option<WordWithModifiers>,
    pub(super) beho_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum ConnectiveKind {
    Afterthought,
    Relation,
    PredicateTail,
    Forethought,
    NonLogical,
    Interval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum ArgumentTailElementSyntax {
    Argument(Box<ArgumentSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Quantifier(QuantifierSyntax),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum QuantifierSyntax {
    Number {
        number: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Vei {
        vei: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WordWithModifiers>,
        veho_free_modifiers: Vec<FreeModifierSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum MathExpressionSyntax {
    Number(QuantifierSyntax),
    Letter {
        letter: Vec<WordWithModifiers>,
        boi: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Vei {
        vei: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_expression: Box<MathExpressionSyntax>,
        veho: Option<WordWithModifiers>,
        veho_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Gek {
        gek: ConnectiveSyntax,
        left_expression: Box<MathExpressionSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Forethought {
        peho: Option<WordWithModifiers>,
        free_modifiers: Vec<FreeModifierSyntax>,
        operator: MathOperatorSyntax,
        operands: Vec<MathExpressionSyntax>,
        kuhe: Option<WordWithModifiers>,
        kuhe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    ReversePolish {
        fuha: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        operands: Vec<MathExpressionSyntax>,
        operators: Vec<MathOperatorSyntax>,
    },
    Nihe {
        nihe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        relation: RelationSyntax,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Mohe {
        mohe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        argument: Box<ArgumentSyntax>,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
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
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WordWithModifiers>,
        luhu_free_modifiers: Vec<FreeModifierSyntax>,
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
        free_modifiers: Vec<FreeModifierSyntax>,
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
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum MathOperatorSyntax {
    Vuhu {
        vuhu: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Maho {
        maho: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    Se {
        se: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        relation: RelationSyntax,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    // v0 exposes this constructor; parser support is being ported with the
    // operator precedence rules.
    #[allow(dead_code)]
    Ke {
        ke: WordWithModifiers,
        ke_free_modifiers: Vec<FreeModifierSyntax>,
        inner_operator: Box<MathOperatorSyntax>,
        kehe: Option<WordWithModifiers>,
        kehe_free_modifiers: Vec<FreeModifierSyntax>,
    },
    // v0 exposes this constructor; parser support is being ported with the
    // operator precedence rules.
    #[allow(dead_code)]
    Bo {
        left_operator: Box<MathOperatorSyntax>,
        bo: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        right_operator: Box<MathOperatorSyntax>,
    },
    // v0 exposes this constructor even though current parser branches produce
    // JohiExpression for the ordinary JOhI operand form.
    #[allow(dead_code)]
    Johi {
        johi: WordWithModifiers,
        free_modifiers: Vec<FreeModifierSyntax>,
        expressions: Vec<MathExpressionSyntax>,
        tehu: Option<WordWithModifiers>,
        tehu_free_modifiers: Vec<FreeModifierSyntax>,
    },
    // v0 exposes this constructor for operator slots accepting numeric forms.
    #[allow(dead_code)]
    Number { number: Vec<WordWithModifiers> },
    Connected {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MathOperatorSyntax>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum RelationSyntax {
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
        leading_predicate: Box<PredicateSyntax>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<PredicateSyntax>,
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
pub(super) struct TimeTenseSyntax {
    pub(super) direction: Vec<WordWithModifiers>,
    pub(super) distance: Option<WordWithModifiers>,
    pub(super) interval: Option<WordWithModifiers>,
    pub(super) nai: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct SpaceTenseSyntax {
    pub(super) direction: Vec<WordWithModifiers>,
    pub(super) distance: Vec<WordWithModifiers>,
    pub(super) interval: Vec<WordWithModifiers>,
    pub(super) dimensions: Vec<WordWithModifiers>,
    pub(super) mohi: Option<WordWithModifiers>,
    pub(super) fehe: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct IntervalTenseSyntax {
    pub(super) number: Vec<WordWithModifiers>,
    pub(super) roi_or_tahe: WordWithModifiers,
    pub(super) nai: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct SimpleTenseModalSyntax {
    pub(super) nahe: Option<WordWithModifiers>,
    pub(super) se: Option<WordWithModifiers>,
    pub(super) bai: Option<WordWithModifiers>,
    pub(super) nai: Option<WordWithModifiers>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct FihoModalSyntax {
    pub(super) nahe: Option<WordWithModifiers>,
    pub(super) fiho: WordWithModifiers,
    pub(super) fiho_free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) relation: RelationSyntax,
    pub(super) fehu: Option<WordWithModifiers>,
    pub(super) fehu_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum TenseModalSyntax {
    Composite {
        leaves: Vec<WordWithModifiers>,
        time: Option<TimeTenseSyntax>,
        space: Option<SpaceTenseSyntax>,
        simple: Option<SimpleTenseModalSyntax>,
        interval: Option<IntervalTenseSyntax>,
        zaho: Vec<WordWithModifiers>,
        caha: Option<WordWithModifiers>,
        ki: Option<WordWithModifiers>,
        cuhe: Option<WordWithModifiers>,
        fiho: Vec<FihoModalSyntax>,
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
pub(super) struct AbstractionSyntax {
    pub(super) nu: WordWithModifiers,
    pub(super) nai: Option<WordWithModifiers>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) additional_nu: Vec<AdditionalNuSyntax>,
    pub(super) subsentence: Box<SubsentenceSyntax>,
    pub(super) kei: Option<WordWithModifiers>,
    pub(super) kei_free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct AdditionalNuSyntax {
    pub(super) connective: ConnectiveSyntax,
    pub(super) nu: WordWithModifiers,
    pub(super) nai: Option<WordWithModifiers>,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum RelationUnitSyntax {
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
pub(super) struct CeiAssignmentSyntax {
    pub(super) cei: WordWithModifiers,
    pub(super) free_modifiers: Vec<FreeModifierSyntax>,
    pub(super) relation_unit: RelationUnitSyntax,
}

impl StatementSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
            FreeModifierSyntax::Replacement {
                lohai,
                old_words,
                sahai,
                new_words,
                lehai,
                free_modifiers,
            } => {
                let mut words = lohai.into_iter().collect::<Vec<_>>();
                words.extend(old_words);
                words.extend(sahai);
                words.extend(new_words);
                words.push(lehai);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
        }
    }
}

impl PredicateSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        let mut words = Vec::new();
        for term in self.leading_terms {
            words.extend(term.words());
        }
        words.extend(self.cu);
        for free_modifier in self.cu_free_modifiers {
            words.extend(free_modifier.words());
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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

impl PredicateTail1Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.connective.words();
        if let Some(tense_modal) = self.tense_modal {
            words.extend(tense_modal.words());
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
        for free_modifier in self.free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl PredicateTail2Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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

impl PredicateTail3Syntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
            PredicateTail3Syntax::GekSentence { gek_sentence } => gek_sentence.words(),
        }
    }
}

impl GekSentenceSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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

impl FragmentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        match self {
            MathExpressionSyntax::Number(quantifier) => quantifier.words(),
            MathExpressionSyntax::Letter {
                letter,
                boi,
                free_modifiers,
            } => {
                let mut words = [letter, boi.into_iter().collect()].concat();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathExpressionSyntax::Vei {
                vei,
                free_modifiers,
                inner_expression,
                veho,
                veho_free_modifiers,
            } => {
                let mut words = vec![vei];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_expression.words());
                words.extend(veho);
                for free_modifier in veho_free_modifiers {
                    words.extend(free_modifier.words());
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
                free_modifiers,
                operator,
                operands,
                kuhe,
                kuhe_free_modifiers,
            } => {
                let mut words = peho.into_iter().collect::<Vec<_>>();
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(operator.words());
                for operand in operands {
                    words.extend(operand.words());
                }
                words.extend(kuhe);
                for free_modifier in kuhe_free_modifiers {
                    words.extend(free_modifier.words());
                }
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
                free_modifiers,
                relation,
                tehu,
                tehu_free_modifiers,
            } => {
                let mut words = vec![nihe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(relation.words());
                words.extend(tehu);
                for free_modifier in tehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathExpressionSyntax::Mohe {
                mohe,
                free_modifiers,
                argument,
                tehu,
                tehu_free_modifiers,
            } => {
                let mut words = vec![mohe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(argument.words());
                words.extend(tehu);
                for free_modifier in tehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
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
                free_modifiers,
                inner_expression,
                luhu,
                luhu_free_modifiers,
            } => {
                let mut words = markers;
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_expression.words());
                words.extend(luhu);
                for free_modifier in luhu_free_modifiers {
                    words.extend(free_modifier.words());
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
                free_modifiers,
                operator,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.push(bihe);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
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
                free_modifiers,
                right_expression,
            } => {
                let mut words = left_expression.words();
                words.extend(operator.words());
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(right_expression.words());
                words
            }
        }
    }
}

impl ArgumentSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
            ArgumentSyntax::ConnectedDescriptor {
                connected_descriptor,
            } => connected_descriptor.words(),
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
            QuoteSyntax::Meho {
                meho,
                free_modifiers,
                math_expression,
            } => {
                let mut words = vec![meho];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(math_expression.words());
                words
            }
        }
    }
}

impl DescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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

impl DescriptorHeadSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        let mut words = vec![self.descriptor];
        for free_modifier in self.descriptor_free_modifiers {
            words.extend(free_modifier.words());
        }
        words
    }
}

impl ConnectedDescriptorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
                free_modifiers,
                math_expression,
                veho,
                veho_free_modifiers,
            } => {
                let mut words = vec![vei];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(math_expression.words());
                words.extend(veho);
                for free_modifier in veho_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
        }
    }
}

impl MathOperatorSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        match self {
            MathOperatorSyntax::Vuhu {
                vuhu,
                free_modifiers,
            } => {
                let mut words = vec![vuhu];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathOperatorSyntax::Maho {
                maho,
                free_modifiers,
                math_expression,
                tehu,
                tehu_free_modifiers,
            } => {
                let mut words = vec![maho];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(math_expression.words());
                words.extend(tehu);
                for free_modifier in tehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathOperatorSyntax::Se {
                se,
                free_modifiers,
                inner_operator,
            } => {
                let mut words = vec![se];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahe {
                nahe,
                free_modifiers,
                inner_operator,
            } => {
                let mut words = vec![nahe];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_operator.words());
                words
            }
            MathOperatorSyntax::Nahu {
                nahu,
                free_modifiers,
                relation,
                tehu,
                tehu_free_modifiers,
            } => {
                let mut words = vec![nahu];
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(relation.words());
                words.extend(tehu);
                for free_modifier in tehu_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathOperatorSyntax::Ke {
                ke,
                ke_free_modifiers,
                inner_operator,
                kehe,
                kehe_free_modifiers,
            } => {
                let mut words = vec![ke];
                for free_modifier in ke_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(inner_operator.words());
                words.extend(kehe);
                for free_modifier in kehe_free_modifiers {
                    words.extend(free_modifier.words());
                }
                words
            }
            MathOperatorSyntax::Bo {
                left_operator,
                bo,
                free_modifiers,
                right_operator,
            } => {
                let mut words = left_operator.words();
                words.push(bo);
                for free_modifier in free_modifiers {
                    words.extend(free_modifier.words());
                }
                words.extend(right_operator.words());
                words
            }
            MathOperatorSyntax::Johi {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn leaf_words(self) -> Vec<WordWithModifiers> {
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
    pub(super) fn words(self) -> Vec<WordWithModifiers> {
        let mut words = self.clone().leaf_words();
        for free_modifier in self.free_modifiers() {
            words.extend(free_modifier.words());
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn free_modifiers(self) -> Vec<FreeModifierSyntax> {
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
