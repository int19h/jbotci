//! Declarative generated syntax parser.

use jbotci_morphology::{Cmavo, Selmaho};

use super::generated_runtime;
use super::parser_core::{
    Input, InputRef, MapExtra, Parser, RecursiveFamily, SimpleSpan, custom, end,
};
use super::tokens::{
    cmavo, cmevla_word, pa_word, relation_word, selmaho, spanned_tokens,
    syntax_error_with_diagnostic_candidate,
};
use super::{
    BoxedParser, ContinuationTimeLimit, ParserState, RecoveryCheckpointIndex, RecoveryDirective,
    RecoveryFrameRank, SpannedToken, SyntaxFrameRank, SyntaxParseError, SyntaxRecoveryMemoSession,
    SyntaxRuleFrame,
};
use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxParseEntry, SyntaxWarning, SyntaxWordCategory,
    Token, TraceReport,
};

#[doc(hidden)]
pub mod generated_model {
    use crate::tree::{SyntaxRecoveryItem as RecoveryTreeItem, WithFreeModifiers};

    use super::*;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {
            #![tree_with_free_modifiers]
            #![tree_recovered]
        }
        model;
        env generated_runtime::SyntaxGrammarEnv;
        strict_parsers;

    recursive {
        text: TextSyntax;
        // The run of NIhO-led paragraphs, as a text reads it after its first paragraph or from
        // its first NIhO. It is recursive so that the generated parser exposes it as a root in
        // every flavor: `SyntaxParseEntry::NihoParagraphs` parses from it, so that a paragraph
        // after a NIhO sees exactly the alternatives that the whole-text parse tries there.
        text_niho_paragraphs: TextNihoParagraphsSyntax;
        paragraph: ParagraphSyntax;
        statement_or_fragment: StatementOrFragmentSyntax;
        statement: StatementSyntax;
        bridi: BridiSyntax;
        bridi_tail: BridiTailSyntax;
        sentence_bridi_tail: SentenceBridiTailSyntax;
        // Guard-only recognizers for the FA chain's reservations; see the "guard-only
        // recognizers for the FA chain" section.
        exp_gek_sentence_guard: ExpGekSentenceGuardSyntax;
        exp_guard_gek: ExpGuardGekSyntax;
        bo_grouped_bridi_tail: BoGroupedBridiTailSyntax;
        bo_grouped_bridi_tail_without_tail_terms: BoGroupedBridiTailWithoutTailTermsSyntax;
        forethought_bridi_connection: ForethoughtBridiConnectionSyntax;
        forethought_bridi_connection_without_tail_terms: ForethoughtBridiConnectionWithoutTailTermsSyntax;
        subbridi: SubbridiSyntax;
        // camxes-exp's tanru-unit relative chain (camxes-exp.peg:214-218).
        exp_selbri_relative_clauses: ExpSelbriRelativeClausesSyntax;
        // camxes-exp's `subsentence` (camxes-exp.peg:94) as a consumer-specific entry.  It is
        // the shared `subbridi` shape today, because the one delta between them -- exp's JACU
        // sentence trailer -- is an adjudicated non-adoption; naming it separately is what keeps
        // a later JACU decision from having to widen every abstraction and forethought consumer.
        exp_subsentence: SubbridiSyntax;
        term: TermSyntax;
        // Every level of the term ladder belongs here, as every level of the sumti, selbri and
        // mekso ladders does. A rule outside this block is re-constructed inline at each of its
        // reference sites, and the ladder levels nest, so omitting them multiplies the
        // combinator graph rebuilt on every parse. That omission cost the epoch's first cut
        // +72% CPU on the full fixture profile; see the epoch-6 ledger.
        cehe_term: CeheTermSyntax;
        loose_term: LooseTermSyntax;
        nonabs_term: NonabsTermSyntax;
        bound_term: BoundTermSyntax;
        simple_term: SimpleTermSyntax;
        // The normal-flavour constituent is a second ladder over the same leaves, and it is not
        // merely an optimization to declare it here: its own leaf inventory contains
        // `gek_termset`, whose operands are this very level, so the family is genuinely cyclic
        // and cannot be reconstructed inline at all.
        normal_term: NormalTermSyntax;
        bound_normal_term: BoundNormalTermSyntax;
        normal_term_atom: NormalTermAtomSyntax;
        // The NUhI-less termset is referenced from every leaf inventory of the ladder, and its
        // operand tree is self-recursive (camxes.peg:136-138), so both belong here for the same
        // reason the ladder levels do.
        gek_termset: GekTermsetSyntax;
        balanced_termset_operands: BalancedTermsetOperandsSyntax;
        sumti: SumtiSyntax;
        sumti_grouped: SumtiGroupedSyntax;
        sumti_afterthought: SumtiAfterthoughtSyntax;
        sumti_bound: SumtiBoundSyntax;
        sumti_forethought: SumtiForethoughtSyntax;
        sumti_base: SumtiBaseSyntax;
        // The description/quantifier operand tier boundary (epoch 9, #552 / #837 SUM-02).
        // `description_leading_operand` is `sumti_base` restricted to the camxes `sumti_6`
        // tier.  It is declared here, rather than being written inline at its two consuming
        // field sites, so that it has its own parser identity and therefore its own FIRST
        // set, elidable-terminator analysis and recovery metadata -- the identity #552 asks
        // for and the identity both consumers receive.
        description_leading_operand: SumtiBaseSyntax;
        // `quantifier` is reached from eight sites across the mex, sumti and description
        // families. Declaring it here gives it one parser identity, so the same subgraph is not
        // rebuilt at each of those sites, exactly as for the ladder levels above.
        quantifier: QuantifierSyntax;
        selbri: SelbriSyntax;
        co_selbri: CoSelbriSyntax;
        tanru_selbri: TanruSelbriSyntax;
        connected_selbri: ConnectedSelbriSyntax;
        bound_selbri: BoundSelbriSyntax;
        plain_bo_selbri: PlainBoSelbriSyntax;
        tanru_unit: TanruUnitSyntax;
        tanru_unit_atom: TanruUnitAtomSyntax;
        // The BE/BEI linked-argument ladder is the term ladder's shape at the link site, and it
        // belongs here for the same reason: `linkargs` -> `linked_term` -> `bound_linked_term` ->
        // `bound_linked_term_operand` nest, and each level had two reference sites, so leaving
        // them out of the family rebuilt the whole subgraph 11 times per parse at the loose level
        // and 99 times at the leaves — which in turn rebuilt `tagged_or_elided_sumti`, the
        // largest subgraph the links reach, 243 times instead of 51.
        //
        // Declaring `linkargs` rather than only the three ladder levels is what keeps the fix from
        // widening: the tanru-unit family reaches the ladder only through this one production, so
        // it takes `linkargs` as a parameter and never has to thread the ladder's own operands.
        linkargs: LinkargsSyntax;
        linked_term: LinkedTermSyntax;
        full_linked_term_candidate: FullLinkedTermSyntax;
        bound_linked_term: BoundLinkedTermSyntax;
        bound_linked_term_operand: BoundLinkedTermOperandSyntax;
        tense_modal: TenseModalSyntax;
        baseline_term_tense_modal: BaselineTermTenseModalSyntax;
        mekso: MeksoSyntax;
        mekso_base: MeksoBaseSyntax;
        mekso_precedence: MeksoPrecedenceSyntax;
        mekso_operand: MeksoOperandSyntax;
        bound_or_simple_mekso_operand: BoundOrSimpleMeksoOperandSyntax;
        simple_mekso_operand: SimpleMeksoOperandSyntax;
        mekso_operator: MeksoOperatorSyntax;
        inner_mekso_operator: InnerMeksoOperatorSyntax;
        atomic_mekso_operator: AtomicMeksoOperatorSyntax;
        reverse_polish_parts: ReversePolishPartsSyntax;
        exp_mex: ExpMexSyntax;
        exp_mex_1: ExpMex1Syntax;
        exp_complete_mex_2: ExpCompleteMex2Syntax;
        exp_mex_2: ExpMex2Syntax;
        exp_rp_parts: ExpRpPartsSyntax;
        letter_string: LetterStringSyntax;
        letter_tokens: LetterTokensSyntax;
        free_modifier: FreeModifierSyntax;
    }

    /// A UI/CAI indicator together with its optional attached NAI word.
    rule "leading indicator" leading_indicator -> struct {
        /// The UI or CAI indicator word.
        field indicator <- choice((selmaho(Ui), selmaho(Cai)));
        /// The optional NAI word attached to the indicator.
        field nai <- opt(cmavo(Nai));
    }

    /// Top-level text syntax.
    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal, selbri, letter_tokens) -> enum {
        /// Ordinary text, retaining its leading material and optional paragraph tree.
        regular_text,
    }

    /// Ordinary text with source-ordered leading material and an optional paragraph tree.
    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal, selbri, letter_tokens) -> struct {
        /// NAI words that precede the first formal text construct.
        field leading_nai <- [zero_or_more cmavo(Nai)];
        /// CMEVLA words accepted before the first formal text construct.
        field leading_cmevla <- [zero_or_more text_leading_cmevla_word()];
        /// UI/CAI indicators accepted before the first formal text construct.
        field leading_indicators <- [zero_or_more leading_indicator()];
        /// Free modifiers accepted before the first formal text construct.
        field leading_free_modifiers <- [zero_or_more free_modifier];
        /// A text-leading connective when it is not the start of a modal forethought connective.
        field leading_connective <- opt(
            modal_forethought_connective(tense_modal, selbri, letter_tokens)
                .not()
                .ignore_then(text_leading_connective),
        );
        /// I-led statement prefixes that occur before the paragraph tree.
        #[recovery_boundary]
        field leading_i_statements <- [zero_or_more leading_i_statement(free_modifier, tense_modal)];
        #[tree_child(primary)]
        /// The primary paragraph subtree, absent when the text contains only leading material.
        field paragraphs <- opt(arc(text_paragraphs(
            paragraph,
            statement_or_fragment,
            free_modifier,
        )));
    }

    /// Sum node for paragraphs; selects among the `text_paragraph_with_additional_niho` and `text_niho_paragraphs` forms.
    rule "paragraphs" text_paragraphs(paragraph, statement_or_fragment, free_modifier) -> enum {
        /// Uses the `text_paragraph_with_additional_niho` product form, whose payload preserves `first` and `additional_niho`.
        text_paragraph_with_additional_niho,
        /// Uses the `text_niho_paragraphs` product form, whose payload preserves `paragraphs`.
        text_niho_paragraphs,
    }

    /// Product node for paragraphs; preserves `first` and `additional_niho` in source order.
    rule "paragraphs" text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The initial paragraph before zero or more NIhO-led paragraph continuations.
        field first <- paragraph;
        /// Ordered sequence of zero or more additional niho components.
        #[recovery_boundary]
        field additional_niho <- [zero_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Transparent product node for paragraphs; preserves the `paragraphs` component.
    rule "paragraphs" text_niho_paragraphs(statement_or_fragment, free_modifier) -> struct {
        /// Non-empty ordered sequence of paragraphs components.
        #[recovery_boundary]
        field paragraphs <- [one_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Product node for paragraph statement; preserves `i`, `connective`, and `free_modifiers` in source order.
    rule "paragraph statement" leading_i_statement(free_modifier, tense_modal) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The optional connective component.
        field connective <- opt(arc(i_paragraph_statement_connective(tense_modal)));
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Sum node for paragraph; selects among the `i_niho_paragraph` and `simple_paragraph` forms.
    rule "paragraph" paragraph(statement_or_fragment, free_modifier) -> enum {
        /// Uses the `i_niho_paragraph` product form, whose payload preserves `i`, `niho`, `free_modifiers`, and `statements`.
        i_niho_paragraph,
        /// Uses the `simple_paragraph` product form, whose payload preserves `statements`.
        simple_paragraph,
    }

    /// Transparent product node for paragraph; preserves the `statements` component.
    rule "paragraph" simple_paragraph(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The paragraph primary statement sequence.
        field statements <- paragraph_statement_sequence(statement_or_fragment, free_modifier);
    }

    /// Product node for paragraph statement sequence; preserves `initial`, `following`, and `trailing` in source order.
    rule "paragraph statement sequence" paragraph_statement_sequence(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The initial paragraph statement before following I-led or trailing-connective entries.
        field initial <- initial_paragraph_statement(statement_or_fragment);
        /// Ordered sequence of zero or more following components.
        #[recovery_boundary]
        field following <- [zero_or_more following_paragraph_statement(statement_or_fragment, free_modifier)];
        /// Ordered sequence of zero or more trailing components.
        field trailing <- [zero_or_more trailing_ijek_paragraph_statement()];
    }

    /// Product node for paragraph; preserves `i`, `niho`, `free_modifiers`, and `statements` in source order.
    rule "paragraph" i_niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// Non-empty ordered sequence of niho components.
        field niho <- [one_or_more selmaho(Niho)];
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        /// The optional statements component.
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    /// Product node for paragraph; preserves `niho`, `free_modifiers`, and `statements` in source order.
    rule "paragraph" niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        /// Non-empty ordered sequence of niho components.
        field niho <- [one_or_more selmaho(Niho)];
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        /// The optional statements component.
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    /// Transparent product node for paragraph statement; preserves the `statement` component.
    rule "paragraph statement" initial_paragraph_statement(statement_or_fragment) -> struct {
        #[tree_child(primary)]
        /// The shared statement child syntax node.
        field statement <- arc(statement_or_fragment);
    }

    /// Product node for paragraph statement; preserves `i`, `free_modifiers`, and `statement` in source order.
    rule "paragraph statement" following_paragraph_statement(statement_or_fragment, free_modifier) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        assert !statement_connective;
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        /// The optional statement component.
        field statement <- opt(arc(statement_or_fragment));
    }

    /// Product node for paragraph statement; preserves `i` and `connective` in source order.
    rule "paragraph statement" trailing_ijek_paragraph_statement -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The statement connective after I, retained for the following paragraph statement.
        field connective <- statement_connective;
    }

    /// Sum node for statement; selects among the `i_statement_connection`, `preposed_i_statement_connection`, and `statement_base` forms.
    rule "statement" statement(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        /// Uses the `i_statement_connection` product form, whose payload preserves `leading_statement` and `continuations`.
        i_statement_connection,
        /// Uses the `preposed_i_statement_connection` product form, whose payload preserves `leading_statement`, `connective`, `i`, and `trailing_statement`.
        preposed_i_statement_connection,
        /// Uses the nested `statement_base` sum form and preserves its selected alternative.
        statement_base,
    }

    /// Sum node for statement; selects among the `prenex_statement`, `bridi_statement`, and `text_group_statement` forms.
    rule "statement" statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> enum {
        /// Uses the `prenex_statement` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_statement`.
        prenex_statement,
        // The forms without a prenex.
        splice statement_after_i_connective,
    }

    /// Sum node for paragraph statement; selects among the `statement_or_fragment_statement` and `fragment_statement` forms.
    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens, free_modifier, forethought_bridi_connection, normal_term, linkargs, linked_term, quantifier) -> enum {
        /// Uses the `statement_or_fragment_statement` product form, whose payload preserves `statement`.
        statement_or_fragment_statement,
        /// Uses the nested `fragment_statement` sum form and preserves its selected alternative.
        fragment_statement,
    }

    /// Transparent product node for paragraph statement; preserves the `statement` component.
    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        /// The `statement` grammar result in the `statement` structural role of the `statement_or_fragment_statement` production.
        field statement <- statement;
    }

    /// Sum node for fragment; selects among 12 forms including `prenex_fragment`, `selbri_fragment`, and `ek_fragment`.
    rule "fragment" fragment_statement(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens, free_modifier, forethought_bridi_connection, normal_term, linkargs, linked_term, quantifier) -> enum {
        /// Uses the `prenex_fragment` product form, whose payload preserves `terms` and `zohu`.
        prenex_fragment,
        /// Uses the `selbri_fragment` product form, whose payload preserves `selbri`.
        selbri_fragment,
        /// Uses the `ek_fragment` product form, whose payload preserves `connective`.
        ek_fragment,
        /// Uses the `gihek_fragment` product form, whose payload preserves `connective`.
        gihek_fragment,
        /// Uses the `multiple_na_fragment` product form, whose payload preserves `first_na`, `second_na`, and `additional_na`.
        multiple_na_fragment,
        /// Uses the `single_na_fragment` product form, whose payload preserves `na`.
        single_na_fragment,
        /// Uses the `terms_fragment` product form, whose payload preserves `terms` and `vau`.
        terms_fragment,
        /// Uses the `mekso_fragment` product form, whose payload preserves `quantifier`.
        mekso_fragment,
        /// Uses the `relative_clause_fragment` product form, whose payload preserves `relative_clauses`.
        relative_clause_fragment,
        /// Uses the `linked_sumti_continuation_fragment` product form, whose payload preserves `bei_links`.
        linked_sumti_continuation_fragment,
        /// Uses the `linked_sumti_fragment` product form, whose payload preserves `linkargs`.
        linked_sumti_fragment,
    }

    /// Sum node for statement; selects among the `bridi_statement` and `text_group_statement` forms.
    rule "statement" statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, letter_tokens) -> enum {
        /// Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.
        bridi_statement,
        /// Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.
        text_group_statement,
    }

    /// Product node for fragment; preserves `first_na`, `second_na`, and `additional_na` in source order.
    rule "fragment" multiple_na_fragment -> struct {
        /// A word from selmaho `Na`.
        field first_na <- selmaho(Na);
        /// A word from selmaho `Na`.
        field second_na <- selmaho(Na);
        /// Ordered sequence of zero or more additional na components.
        field additional_na <- [zero_or_more selmaho(Na)];
    }

    /// Transparent product node for fragment; preserves the `na` component.
    rule "fragment" single_na_fragment -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
    }

    /// Transparent product node for fragment; preserves the `connective` component.
    rule "fragment" ek_fragment -> struct {
        #[tree_child(primary)]
        /// The standalone `ek_connective` connective represented by the `ek_fragment` fragment.
        field connective <- ek_connective();
    }

    /// Transparent product node for fragment; preserves the `connective` component.
    rule "fragment" gihek_fragment -> struct {
        #[tree_child(primary)]
        /// The standalone `gihek_connective` connective represented by the `gihek_fragment` fragment.
        field connective <- gihek_connective();
    }

    /// Product node for statement connection; preserves `leading_statement` and `continuations` in source order.
    rule "statement connection" i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        /// Non-empty ordered sequence of continuations components.
        #[recovery_boundary]
        field continuations <- [one_or_more i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens)];
    }

    /// Product node for statement connective; preserves `i` and `connective` in source order.
    rule "statement connective" pending_i_connective -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The `statement_connective` connective retained while its following statement remains pending.
        field connective <- statement_connective;
        assert cmavo(I);
    }

    /// Sum node for statement connection; selects among the `chained_i_connective_statement_tail` and `simple_i_connective_statement_tail` forms.
    rule "statement connection" i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        /// Uses the `chained_i_connective_statement_tail` product form, whose payload preserves `pending`, `i`, `connective`, and `trailing_statement`.
        chained_i_connective_statement_tail,
        /// Uses the `simple_i_connective_statement_tail` product form, whose payload preserves `i`, `connective`, and `trailing_statement`.
        simple_i_connective_statement_tail,
    }

    /// Product node for statement connection; preserves `pending`, `i`, `connective`, and `trailing_statement` in source order.
    rule "statement connection" chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// Non-empty ordered sequence of pending components.
        field pending <- [one_or_more pending_i_connective];
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The `i_statement_connective` connective joining the adjacent constituents of the `chained_i_connective_statement_tail` production.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, letter_tokens));
    }

    /// Product node for statement connection; preserves `i`, `connective`, and `trailing_statement` in source order.
    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The `i_statement_connective` connective joining the adjacent constituents of the `simple_i_connective_statement_tail` production.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, letter_tokens));
    }

    /// Product node for statement connection; preserves `leading_statement`, `connective`, `i`, and `trailing_statement` in source order.
    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        /// The `statement_connective` connective joining the adjacent constituents of the `preposed_i_statement_connection` production.
        field connective <- statement_connective;
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, letter_tokens));
    }

    /// Product node for text group; preserves `tense_modal`, `tuhe`, `text`, and `tuhu` in source order.
    rule "text group" text_group_statement(text, tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Tuhe` cmavo marker.
        field tuhe <- cmavo(Tuhe).wf();
        #[tree_child(primary)]
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Tuhu` cmavo marker.
        field tuhu <- opt(cmavo(Tuhu).wf()).elidable_terminator(Tuhu);
    }

    /// Product node for prenex; preserves `terms` and `zohu` in source order.
    rule "prenex" prenex_fragment(term) -> struct {
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
        /// The `Zohu` cmavo marker.
        field zohu <- cmavo(Zohu).wf();
    }

    /// Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_statement` in source order.
    rule "prenex" prenex_statement(statement, term) -> struct {
        /// Ordered sequence of zero or more prenex terms components.
        field prenex_terms <- [zero_or_more term];
        /// The `Zohu` cmavo marker.
        field zohu <- cmavo(Zohu).wf();
        #[tree_child(primary)]
        /// The shared inner statement child syntax node.
        field inner_statement <- arc(statement);
    }

    /// Product node for statement; preserves the `bridi` component.
    ///
    /// camxes-standard joins statements only through an I (camxes.peg:20-22); the BO and KE
    /// envelopes this production used to carry after a bare bridi were jbotci's own, over full
    /// subbridi operands no source spells at this level. D1 deletes them: the sourced GIhA tail
    /// BO and KE joins and the I-led statement BO envelope already carry every surface that has
    /// an owner, and the I-less KE ones flip to reject.
    rule "statement" bridi_statement(bridi) -> struct {
        #[tree_child(primary)]
        /// The shared bridi child syntax node.
        field bridi <- arc(bridi);
    }

    /// Transparent product node for selbri; preserves the `selbri` component.
    rule "selbri" selbri_fragment(selbri) -> struct {
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
    }

    /// Product node for terms; preserves `terms` and `vau` in source order.
    rule "terms" terms_fragment(term) -> struct {
        #[tree_child(primary)]
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(cmavo(Vau).wf()).elidable_terminator(Vau);
    }

    /// Transparent product node for mex; preserves the `quantifier` component.
    rule "mex" mekso_fragment(mekso, letter_tokens, free_modifier, quantifier) -> struct {
        #[tree_child(primary)]
        /// The shared quantifier child syntax node.
        field quantifier <- arc(quantifier);
    }

    alias "subbridi" exp_subsentence(
        subbridi,
        bridi,
        term,
    ) = subbridi(
        subbridi,
        bridi,
        term,
    ).recursive_output(exp_subsentence);

    // The restricted leading/inner operand of the description and quantifier sites (epoch 9,
    // #552 / #837 SUM-02).  camxes spells both sites with `sumti_6`, while jbotci's `sumti_base`
    // is `sumti_6` plus two `sumti_5`-tier arms; the classifier removes exactly those two from
    // this route and leaves the produced type, and therefore every tree, unchanged.  The
    // restriction is written once here and consumed by name at both sites.
    alias "sumti" description_leading_operand(
        sumti_base,
    ) = sumti_base
        .reject_output(crate::grammar::sumti_operand_tier::QuantifierBearingSumtiRejection)
        .recursive_output(description_leading_operand);

    /// Product node for relative clauses; preserves `first` and `additional` in source order.
    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// The initial `relative_clause_atom` constituent before the continuations of the `relative_clause_list` production.
        field first <- relative_clause_atom(sumti, subbridi, tense_modal, normal_term);
        /// Ordered sequence of zero or more additional components.
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal, normal_term)];
    }

    /// Transparent product node for relative clauses; preserves the `relative_clauses` component.
    ///
    /// S1f: the standalone relative-clause fragment runs S1's policy, which it gets by being
    /// this instantiation of the shared list rather than by a policy of its own.
    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal, normal_term) -> struct {
        #[tree_child(primary)]
        /// The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `relative_clause_fragment` production.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, normal_term);
    }

    /// Transparent product node for linked arguments; preserves the `bei_links` component.
    rule "linked arguments" linked_sumti_continuation_fragment(linked_term) -> struct {
        #[tree_child(primary)]
        /// Non-empty ordered sequence of bei links components.
        field bei_links <- [one_or_more bei_link(linked_term)];
    }

    /// Transparent product node for linked arguments; preserves the `linkargs` component.
    rule "linked arguments" linked_sumti_fragment(linkargs) -> struct {
        #[tree_child(primary)]
        /// The `linkargs` grammar result in the `linkargs` structural role of the `linked_sumti_fragment` production.
        field linkargs <- linkargs;
    }

    /// Sum node for bridi; selects among the `bridi_with_leading_terms`, `bare_cu_bridi`, and
    /// `relation_only_bridi` forms.
    ///
    /// camxes-standard's sentence carries one leading `terms CU_elidible? free*` group
    /// (camxes.peg:26) and camxes-exp puts every further group inside `bridi_tail_3`, so the
    /// second outer group jbotci used to model here as `bridi_with_post_cu_terms` /
    /// `bare_cu_terms_bridi` over a shared `cu_terms_bridi_tail` is now the tail's own prefix.
    rule "bridi" bridi(term, selbri, subbridi, tense_modal, sentence_bridi_tail) -> enum {
        /// Uses the `bridi_with_leading_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.
        bridi_with_leading_terms,
        /// Uses the `bare_cu_bridi` product form, whose payload preserves `cu` and `bridi_tail`.
        bare_cu_bridi,
        /// Uses the `relation_only_bridi` product form, whose payload preserves `bridi_tail`.
        relation_only_bridi,
    }

    /// Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.
    rule "bridi" bridi_with_leading_terms(term, sentence_bridi_tail) -> struct {
        /// Non-empty ordered sequence of leading terms components.
        field leading_terms <- [one_or_more term];
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(sentence_bridi_tail);
    }

    /// Product node for bridi; preserves `cu` and `bridi_tail` in source order.
    ///
    /// A leading-termless CU before the selbri is camxes-exp's alone: camxes-standard's
    /// sentence spells `terms CU_elidible?` (camxes.peg:26), so with no terms there is no CU
    /// slot at all. The cell is therefore adopted rather than baseline, and R2 gives every
    /// non-baseline cell a warning, so the CU carries one. `mi cu broda` is untouched: its
    /// leading term sends it to `bridi_with_leading_terms`, whose CU is the sourced one.
    rule "bridi" bare_cu_bridi(sentence_bridi_tail) -> struct {
        /// The `Cu` cmavo marker, camxes-exp's leading-termless CU.
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(sentence_bridi_tail);
    }

    /// Transparent product node for bridi; preserves the `bridi_tail` component.
    rule "bridi" relation_only_bridi(sentence_bridi_tail) -> struct {
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(sentence_bridi_tail);
    }

    /// Sum node for the bridi tail of a whole bridi. camxes-exp joins whole bridi tails at the
    /// sentence level with `joik_jek` (camxes-exp.peg:81-87); camxes-standard has no such joint.
    /// Of that `joik_jek` inventory, jbotci takes only the intervals, because the other arms
    /// are either reached first by the bridi-tail joints (JA, JOI) or were never part of jbotci
    /// (A, VUhU).
    ///
    /// The baseline arms are the two arms of [`bridi_tail`], so a bridi without an interval
    /// joint keeps its tree. The interval arm comes first because PEG choice does not backtrack
    /// into a completed arm: it fails, and gives way to the baseline arms, unless an interval
    /// joint follows the first tail. Its operands are [`bridi_tail`], so a KE group or an
    /// operand never takes a sentence-level joint of its own, as in camxes-exp.
    rule "bridi tail" sentence_bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// camxes-exp's sentence-level interval joints.
        exp_interval_connected_bridi_tail,
        // The baseline arms.
        splice bridi_tail,
    }

    /// camxes-exp's sentence-level joints with an interval connective (camxes-exp.peg:81-87):
    ///
    /// ```text
    /// sentence      <- terms? bridi_tail_t1 (joik_jek bridi_tail
    ///                  / joik_jek stag? KE_clause free* bridi_tail KEhE_elidible free*)* ...
    /// bridi_tail_t1 <- bridi_tail_t2 (joik_jek stag? KE_clause free* bridi_tail KEhE_elidible free*)?
    /// bridi_tail_t2 <- bridi_tail (joik_jek stag? BO_clause free* bridi_tail)?
    /// ```
    ///
    /// Every joint groups to the left. A BO joint can only come first, and only directly after
    /// it can a KE joint come before the flat one, so the first joint carries that order.
    rule "bridi tail" exp_interval_connected_bridi_tail(bridi_tail, tense_modal) -> struct {
        /// The first bridi tail.
        field first <- arc(bridi_tail);
        /// The first joint: camxes-exp's BO level, its KE level, or the first flat joint.
        field first_joint <- exp_interval_leading_bridi_tail_joint(bridi_tail, tense_modal);
        /// The later joints, in source order.
        field further_joints <- [zero_or_more exp_interval_further_bridi_tail_joint(bridi_tail, tense_modal)];
    }

    /// The first joint of [`exp_interval_connected_bridi_tail`], in camxes-exp's order:
    /// `bridi_tail_t2`'s BO joint, then `bridi_tail_t1`'s KE joint, then the flat joint.
    rule "bridi tail connective" exp_interval_leading_bridi_tail_joint(bridi_tail, tense_modal) -> enum {
        /// The BO joint, with the KE joint that may follow it at `bridi_tail_t1`.
        exp_interval_bo_led_bridi_tail_joints,
        /// The KE joint of `bridi_tail_t1`.
        exp_interval_ke_bridi_tail_joint,
        /// The flat joint.
        exp_interval_flat_bridi_tail_joint,
    }

    /// A later joint of [`exp_interval_connected_bridi_tail`]: camxes-exp's repeated
    /// `(joik_jek bridi_tail / joik_jek stag? KE_clause ...)`, flat first.
    rule "bridi tail connective" exp_interval_further_bridi_tail_joint(bridi_tail, tense_modal) -> enum {
        /// The flat joint.
        exp_interval_flat_bridi_tail_joint,
        /// The KE joint.
        exp_interval_ke_bridi_tail_joint,
    }

    /// camxes-exp's `bridi_tail_t2` BO joint, then the optional `bridi_tail_t1` KE joint that
    /// camxes-exp tries before its flat joints.
    rule "bridi tail connective" exp_interval_bo_led_bridi_tail_joints(bridi_tail, tense_modal) -> struct {
        /// The BO joint.
        field bo_joint <- exp_interval_bo_bridi_tail_joint(bridi_tail, tense_modal);
        /// The optional KE joint after it.
        field ke_joint <- opt(exp_interval_ke_bridi_tail_joint(bridi_tail, tense_modal));
    }

    /// `joik_jek stag? BO_clause free* bridi_tail` with an interval connective.
    rule "bridi tail connective" exp_interval_bo_bridi_tail_joint(bridi_tail, tense_modal) -> struct {
        // The interval must be present in strict lookahead before recovery may enter the
        // joint, so missing-token recovery never invents an interval joint.
        assert choice((selmaho(Gaho).ignored(), (opt(selmaho(Se)), selmaho(Bihi)).ignored())).lookahead();
        #[tree_child(primary)]
        /// The interval connective.
        field connective <- exp_interval_sentence_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The bridi tail after BO.
        field bridi_tail <- arc(bridi_tail);
    }

    /// `joik_jek stag? KE_clause free* bridi_tail KEhE_elidible free*` with an interval
    /// connective. camxes-exp has no tail terms after KEhE here.
    rule "bridi tail connective" exp_interval_ke_bridi_tail_joint(bridi_tail, tense_modal) -> struct {
        // See `exp_interval_bo_bridi_tail_joint`.
        assert choice((selmaho(Gaho).ignored(), (opt(selmaho(Se)), selmaho(Bihi)).ignored())).lookahead();
        #[tree_child(primary)]
        /// The interval connective.
        field connective <- exp_interval_sentence_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The grouped bridi tail.
        field bridi_tail <- arc(bridi_tail);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// `joik_jek bridi_tail` with an interval connective.
    rule "bridi tail connective" exp_interval_flat_bridi_tail_joint(bridi_tail) -> struct {
        // See `exp_interval_bo_bridi_tail_joint`.
        assert choice((selmaho(Gaho).ignored(), (opt(selmaho(Se)), selmaho(Bihi)).ignored())).lookahead();
        #[tree_child(primary)]
        /// The interval connective.
        field connective <- exp_interval_sentence_connective;
        /// The joined bridi tail.
        field bridi_tail <- arc(bridi_tail);
    }

    /// The interval arms of camxes-exp's `joik` (camxes-exp.peg:347-349):
    /// `interval / GAhO_clause interval GAhO_clause`. They mirror `simple_interval_connective`
    /// and `closed_interval_connective`, but their BIhI carries the warning for the
    /// sentence-level joint.
    rule "interval" exp_interval_sentence_connective -> enum {
        /// `GAhO_clause interval GAhO_clause`.
        exp_sentence_closed_interval_connective,
        /// `interval`.
        exp_sentence_simple_interval_connective,
    }

    /// `SE_clause? BIhI_clause NAI_clause?` at camxes-exp's sentence-level joint.
    rule "interval" exp_sentence_simple_interval_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The BIhI word, which carries the warning for the joint.
        field bihi <- selmaho(Bihi).warn(ExperimentalIntervalSentenceConnective).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// `GAhO_clause interval GAhO_clause` at camxes-exp's sentence-level joint.
    rule "interval" exp_sentence_closed_interval_connective -> struct {
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field left_interval <- selmaho(Gaho);
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The BIhI word, which carries the warning for the joint.
        field bihi <- selmaho(Bihi).warn(ExperimentalIntervalSentenceConnective);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field right_interval <- selmaho(Gaho).wf();
    }

    /// Sum node for bridi tail; selects among the `bridi_tail_with_possible_tail_terms` and
    /// `bridi_tail_without_tail_terms` forms.
    rule "bridi tail" bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `bridi_tail_with_possible_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.
        bridi_tail_with_possible_tail_terms,
        /// Uses the `bridi_tail_without_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.
        bridi_tail_without_tail_terms,
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional ke continuation component.
        field ke_continuation <- opt(arc(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        /// The optional ke continuation component.
        field ke_continuation <- opt(arc(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Transparent product node for bridi tail; preserves the `bridi_tails` component.
    rule "bridi tail" afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The source-ordered `bridi_tails` chain assembled by the `afterthought_bridi_tail_without_tail_terms` production.
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail_without_tail_terms),
            zero_or_more: bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Transparent product node for bridi tail; preserves the `bridi_tails` component.
    rule "bridi tail" afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        /// The source-ordered `bridi_tails` chain assembled by the `afterthought_bridi_tail` production.
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail),
            zero_or_more: bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Product node for bridi tail; preserves `first` and `bo_continuation` in source order.
    rule "bridi tail" bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// camxes-exp's leading `CU_elidible? free*` at this level (camxes-exp.peg:107).
        field cu <- opt(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_joint_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `bo_continuation` in source order.
    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        /// camxes-exp's leading `CU_elidible? free*` at this level (camxes-exp.peg:107). The
        /// sourced joints carry no CU of their own, so every adopted CU after a tail connective
        /// is this one, on the operand, which is where camxes-exp puts it.
        field cu <- opt(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_joint(bo_grouped_bridi_tail, term, tense_modal)));
    }

    /// Sum node for bridi tail connective; the BO-level joint carries the connective-led
    /// continuation. It has one arm only; the sum stays so that the trees keep their shape.
    rule "bridi tail connective" bridi_tail_bo_joint(bo_grouped_bridi_tail, term, tense_modal) -> enum {
        /// The connective-led BO continuation.
        bridi_tail_bo_continuation,
    }

    /// The tail-terms-free mirror of [`bridi_tail_bo_joint`].
    rule "bridi tail connective" bridi_tail_bo_joint_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> enum {
        /// The connective-led BO continuation.
        bridi_tail_bo_continuation_without_tail_terms,
    }

    /// Sum node for bridi tail without tail terms, with the ordinary fallback order.
    rule "bridi tail" simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `forethought_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `connection`.
        forethought_simple_bridi_tail_without_tail_terms,
        /// Uses the `selbri_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `selbri` and `vau`.
        selbri_simple_bridi_tail_without_tail_terms,
        /// The tail-terms-free mirror of camxes-exp's prefixed arm.
        exp_prefixed_simple_bridi_tail_without_tail_terms,
    }

    /// Sum node for bridi tail; selects among the `forethought_simple_bridi_tail`,
    /// `selbri_simple_bridi_tail` and camxes-exp's prefixed forms.
    ///
    /// camxes-exp writes `bridi_tail_3 <- (terms CU_elidible?)* selbri tail_terms / gek_sentence`
    /// (camxes-exp.peg:108), so the repetition belongs to the selbri alternative alone and never
    /// to the GEK one. The prefixed arm is last because the boundary it shares with the sourced
    /// arms is decided the sourced way: `gi'e pu brode` is a tagged selbri, which the selbri arm
    /// reaches first over the identical extent, and only `gi'e pu cu brode` -- where no tagged
    /// selbri can be built -- falls through to the prefix.
    rule "bridi tail" simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `forethought_simple_bridi_tail` product form, whose payload preserves `connection`.
        forethought_simple_bridi_tail,
        /// Uses the `selbri_simple_bridi_tail` product form, whose payload preserves `selbri`, `terms`, and `vau`.
        selbri_simple_bridi_tail,
        /// Uses camxes-exp's prefixed form, whose payload preserves `prefixes` and `tail`.
        exp_prefixed_simple_bridi_tail,
    }

    /// One `terms CU_elidible?` group of camxes-exp's `bridi_tail_3` prefix
    /// (camxes-exp.peg:108). The CU is the group's own, which is what lets the groups repeat.
    rule "bridi tail" exp_tail_terms_prefix(term) -> struct {
        /// Non-empty ordered sequence of this group's terms.
        field terms <- [one_or_more term];
        /// The optional `Cu` cmavo marker closing this group.
        field cu <- opt(arc(cmavo(Cu).wf()));
    }

    /// camxes-exp's prefixed `bridi_tail_3`: one or more `terms CU_elidible?` groups before the
    /// selbri tail (camxes-exp.peg:108). Requiring a group is what makes the arm structurally
    /// distinct from the sourced one; the construct is diagnosed post-parse by the standing
    /// visitor, once for the whole run of groups, because the run is one node and one decision.
    rule "bridi tail" exp_prefixed_simple_bridi_tail(selbri, term) -> struct {
        /// Non-empty ordered sequence of the leading term groups.
        field prefixes <- [one_or_more exp_tail_terms_prefix(term)];
        /// The selbri tail the groups lead.
        field tail <- arc(selbri_simple_bridi_tail(selbri, term));
    }

    /// The tail-terms-free mirror of [`exp_prefixed_simple_bridi_tail`].
    rule "bridi tail" exp_prefixed_simple_bridi_tail_without_tail_terms(selbri, term) -> struct {
        /// Non-empty ordered sequence of the leading term groups.
        field prefixes <- [one_or_more exp_tail_terms_prefix(term)];
        /// The selbri tail the groups lead.
        field tail <- arc(selbri_simple_bridi_tail_without_tail_terms(selbri));
    }

    /// Transparent product node for forethought bridi connection; preserves the `connection` component.
    rule "forethought bridi connection" forethought_simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        /// The shared connection child syntax node.
        field connection <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Transparent product node for forethought bridi connection; preserves the `connection` component.
    rule "forethought bridi connection" forethought_simple_bridi_tail(forethought_bridi_connection) -> struct {
        /// The shared connection child syntax node.
        field connection <- arc(forethought_bridi_connection);
    }

    /// Product node for bridi tail; preserves `selbri` and `vau` in source order.
    rule "bridi tail" selbri_simple_bridi_tail_without_tail_terms(selbri) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail; preserves `selbri`, `terms`, and `vau` in source order.
    rule "bridi tail" selbri_simple_bridi_tail(selbri, term) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Sum node for forethought bridi connection; selects among the `direct_forethought_bridi_connection`, `grouped_forethought_bridi_connection`, and `negated_forethought_bridi_connection` forms.
    rule "forethought bridi connection" forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal, baseline_term_tense_modal, selbri, letter_tokens) -> enum {
        /// Uses the `grouped_forethought_bridi_connection` product form, whose payload preserves `tense_modals`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection,
        /// Uses the `direct_forethought_bridi_connection` product form, whose payload preserves `gek`, `first`, `first_branch`, and 4 other fields.
        direct_forethought_bridi_connection,
        /// Uses the `negated_forethought_bridi_connection` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection,
    }

    /// Sum node for forethought bridi connection; selects among the `direct_forethought_bridi_connection_without_tail_terms`, `grouped_forethought_bridi_connection_without_tail_terms`, and `negated_forethought_bridi_connection_without_tail_terms` forms.
    rule "forethought bridi connection" forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal, baseline_term_tense_modal, selbri, letter_tokens) -> enum {
        /// Uses the `grouped_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `tense_modals`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection_without_tail_terms,
        /// Uses the `direct_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `gek`, `first`, `first_branch`, and 3 other fields.
        direct_forethought_bridi_connection_without_tail_terms,
        /// Uses the `negated_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection_without_tail_terms,
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 4 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection(subbridi, term, tense_modal, selbri, letter_tokens) -> struct {
        /// The opening forethought connective that determines how the subbridi branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The first subbridi branch, which follows the opening connective without an intervening GIK.
        field first <- arc(subbridi);
        /// The first GIK-led subbridi branch paired with the opening connective.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// Terms attached to the completed forethought bridi after its connected subbridi branches.
        field tail_terms <- [zero_or_more term];
        /// The optional elidable VAU terminator for the bridi tail.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 3 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal, selbri, letter_tokens) -> struct {
        /// The opening forethought connective that determines how the subbridi branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The first subbridi branch, which follows the opening connective without an intervening GIK.
        field first <- arc(subbridi);
        /// The first GIK-led subbridi branch paired with the opening connective.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// The optional elidable VAU terminator for the bridi tail.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for forethought bridi branch; preserves `gik` and `branch` in source order.
    rule "forethought bridi branch" forethought_bridi_branch(subbridi) -> struct {
        /// The GIK connective that introduces this branch and pairs with the opening forethought connective.
        field gik <- gik_connective;
        /// The subbridi governed by this branch's GIK connective.
        field branch <- arc(subbridi);
    }

    /// Product node for forethought bridi connection; preserves `tense_modals`, `ke`, `inner`, and `kehe` in source order.
    rule "forethought bridi connection" grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal, baseline_term_tense_modal) -> struct {
        /// The source-ordered tag sequence before KE.
        field tense_modals <- [zero_or_more arc(standard_forethought_tense_modal(baseline_term_tense_modal, tense_modal))];
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Product node for forethought bridi connection; preserves `tense_modals`, `ke`, `inner`, and `kehe` in source order.
    rule "forethought bridi connection" grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal, baseline_term_tense_modal) -> struct {
        /// The source-ordered tag sequence before KE.
        field tense_modals <- [zero_or_more arc(standard_forethought_tense_modal(baseline_term_tense_modal, tense_modal))];
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Product node for forethought bridi connection; preserves `na` and `inner` in source order.
    rule "forethought bridi connection" negated_forethought_bridi_connection(forethought_bridi_connection) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection);
    }

    /// Product node for forethought bridi connection; preserves `na` and `inner` in source order.
    rule "forethought bridi connection" negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Product node for bridi tail connective; camxes-standard's one top-level tail join,
    /// `gihek stag? KE_clause bridi_tail KEhE_clause? tail_terms` (camxes.peg:76). Its connective
    /// is the shared bridi-tail connective, so camxes-exp's JA and JOI arms reach this joint as
    /// they reach the flat and BO joints (camxes-exp.peg:97).
    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `gihek_bridi_tail_ke_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation_without_tail_terms` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, and 3 other fields in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective` and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation_without_tail_terms` production.
        field connective <- bridi_tail_connective;
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `bridi_tail`, `tail_terms`, and `vau` in source order.
    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation` production.
        field connective <- bridi_tail_connective;
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Sum node for subbridi; selects among the `prenex_subbridi` and `bridi_subbridi` forms.
    rule "subbridi" subbridi(subbridi, bridi, term) -> enum {
        /// Uses the `prenex_subbridi` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_subbridi`.
        prenex_subbridi,
        /// Uses the `bridi_subbridi` product form, whose payload preserves `bridi`.
        bridi_subbridi,
    }

    /// Transparent product node for subbridi; preserves the `bridi` component.
    rule "subbridi" bridi_subbridi(bridi) -> struct {
        /// The shared bridi child syntax node.
        field bridi <- arc(bridi);
    }

    /// Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_subbridi` in source order.
    rule "prenex" prenex_subbridi(subbridi, term) -> struct {
        /// Ordered sequence of zero or more prenex terms components.
        field prenex_terms <- [zero_or_more term];
        /// The `Zohu` cmavo marker.
        field zohu <- cmavo(Zohu).wf();
        /// The shared inner subbridi child syntax node.
        field inner_subbridi <- arc(subbridi);
    }

    alias "term" term_guard =
        (relation_word(), cmavo(Bu).not()).not();

    // camxes-exp keeps a loose term continuation from consuming a connective plus the tag that
    // belongs to a following BO/KE bridi tail or BO-led subsentence (camxes-exp.peg:138-140).
    // The reservation is keyed to ARM ENGAGEMENT rather than to a dialect feature: the loose tier
    // is a default-enabled diagnosed extension, so the guard must hold wherever a loose
    // continuation is offered, in every profile and at every consumer.
    alias "term connection" term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection) = (
        (
            term_afterthought_connective,
            arc(tense_modal),
            choice((cmavo(Bo), cmavo(Ke))).wf(),
            opt(cmavo(Cu).wf()),
            choice((
                arc(selbri).ignored(),
                arc(forethought_bridi_connection).ignored(),
            )),
        ).not(),
        (term_afterthought_connective, arc(tense_modal), cmavo(Bo), cmavo(I)).not(),
    ).ignored();

    /// The PEhE level of the composed term hierarchy: `terms_1 <- terms_2 (PEhE free* joik_jek
    /// terms_2)*` (camxes.peg:114, camxes-exp.peg:121). Every consumer of a term sequence repeats
    /// this level, which is exactly the upstream `terms <- terms_1+` shape.
    ///
    /// Like the levels below it, this rule splices the level below it instead of naming it as a
    /// branch. A branch would wrap the lower level in a public variant, which Debug and serde
    /// output would show as an extra level. The splice gives this enum the variants of every
    /// lower level directly, so each level adds only its own connection.
    rule "term" term(gek_termset, statement, exp_subsentence, term, cehe_term, loose_term, nonabs_term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the `pehe_termset_connection` product form, whose payload preserves `leading_term` and `continuations`.
        pehe_termset_connection,
        // The CEhE level and the levels below it.
        splice cehe_term,
    }

    /// The CEhE level of the composed term hierarchy: `terms_2 <- term (CEhE free* nonabs_term)*`
    /// (camxes.peg:116). It is the operand level of the PEhE connection above it.
    rule "term" cehe_term(gek_termset, statement, exp_subsentence, term, loose_term, nonabs_term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.
        termset_group,
        // The loose connective level and the levels below it.
        splice loose_term,
    }

    /// The loose connective level of the composed term hierarchy: camxes-exp `abs_term_1 <-
    /// abs_term_2 (joik_ek !tag_bo_ke_bridi_tail !tag_bo_subsentence abs_term_2)*`
    /// (camxes-exp.peg:153). It is the leading operand level of the CEhE connection above it.
    rule "term" loose_term(gek_termset, statement, exp_subsentence, term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        // The BO-bound level and the leaves below it.
        splice bound_term,
    }

    /// The unguarded (`nonabs`) operand flavour of the CEhE continuation.
    ///
    /// Standard camxes reads a CEhE continuation as `nonabs_term` (camxes.peg:116, :128), whose
    /// tag-led atom carries no absorption guard, so `ko'a ce'e pu broda` assigns `pu` with an
    /// elided KU. camxes-exp instead reads the same continuation as a full absorption-safe
    /// `abs_term` (camxes-exp.peg:122), which contributes the connective and BO tiers. The union
    /// of the two sources is exactly this level: the guarded tiers with the unguarded leaf
    /// inventory. The guard only ever fires when a selbri follows the atom directly, which is a
    /// position no connective tier can occupy, so no surface outside the two sources is admitted.
    rule "term" nonabs_term(gek_termset, statement, exp_subsentence, term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the `stag_bound_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        stag_bound_term_connection,
        // The unguarded leaves.
        splice normal_term_atom,
    }

    /// Product node for termset connection; preserves `leading_term` and `continuations` in source order.
    rule "termset connection" pehe_termset_connection(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(cehe_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more pehe_termset_connection_continuation(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection)];
    }

    /// Product node for termset connection continuation; preserves `pehe`, `connective`, and `trailing_term` in source order.
    rule "termset connection continuation" pehe_termset_connection_continuation(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        /// The `Pehe` cmavo marker.
        field pehe <- cmavo(Pehe).wf();
        /// The PEhE connective. camxes-standard spells the PEhE level `joik_jek` (camxes.peg:114),
        /// which is the JOIK-or-JEK inventory; #806 carries that domain, so EK and VUhU are
        /// rejected here with a documented-gap ledger row against camxes-exp's literal `joik_jek`.
        field connective <- standard_statement_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(cehe_term);
    }

    /// Sum node for term; selects among 12 forms including `place_tagged_sumti_term`, `elided_nahe_fiho_tag_term`, and `tagged_sumti_before_tag_term`.
    rule "term" simple_term(gek_termset, statement, exp_subsentence, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the absorption-safe `fa_chain_tagged_sumti_term` product form, camxes-exp's JOIK-chained FA tag.
        fa_chain_tagged_sumti_term,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the absorption-safe `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the `fihoi_proposal_adverbial_term` product form, whose payload preserves `fihoi`, `subsentence`, and `fihau`.
        fihoi_proposal_adverbial_term,
        /// Uses the `exp_soi_adverbial_term` wrapper, whose payload preserves the classified camxes-exp candidate.
        exp_soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
    }

    /// The BO-bound precedence level for ordinary terms in the camxes-exp hierarchy.
    ///
    /// The leaves come from a splice of `simple_term`, not from a `simple_term` branch: a branch
    /// would add a public wrapper variant to Debug and serde output.
    rule "term" bound_term(gek_termset, statement, exp_subsentence, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, simple_term, letter_tokens, letter_string, free_modifier, normal_term, tanru_unit_atom, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the diagnosed BO-bound connection with the mandatory absorption-safe stag.
        stag_bound_term_connection,
        // The absorption-guarded leaves.
        splice simple_term,
    }

    /// The BO-bound ordinary-term connection with one or more continuations.
    ///
    /// camxes-exp's absorption-safe `abs_term_2` requires the stag before BO
    /// (camxes-exp.peg:154); camxes-standard has no term-level BO at all, so every occurrence is
    /// diagnosed. The operands intentionally remain `simple_term`: sumti greediness must continue
    /// to own chains whose trailing operand is a bare sumti, rather than silently changing their
    /// term-level grouping.
    rule "term connection" stag_bound_term_connection(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, exp_gek_sentence_guard, exp_guard_gek) -> struct {
        assert term_guard();
        /// The first simple term at the BO-bound precedence level.
        field leading_term <- arc(simple_term);
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more bound_term_continuation(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, exp_gek_sentence_guard, exp_guard_gek)];
    }

    /// The BO continuation shape at the absorption-safe term level.
    ///
    /// camxes-exp's `abs_term_2 <- abs_term_3 (joik_ek stag BO_clause abs_term_3)*`
    /// (camxes-exp.peg:154) requires both the connective and the stag. The sum has one arm
    /// only; it stays so that the trees keep their shape.
    rule "term connection continuation" bound_term_continuation(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, exp_gek_sentence_guard, exp_guard_gek) -> enum {
        /// Uses the sourced mandatory-stag `stag_bound_term_continuation` product form.
        stag_bound_term_continuation,
    }

    /// One mandatory-stag BO continuation at the absorption-safe term level.
    rule "term connection continuation" stag_bound_term_continuation(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, exp_gek_sentence_guard, exp_guard_gek) -> struct {
        /// The connective joining the adjacent simple terms.
        field connective <- term_afterthought_connective;
        /// The mandatory camxes-exp `stag` before BO.
        field tense_modal <- arc(tense_modal);
        /// The `Bo` cmavo marker, which owns the experimental warning for the whole connection.
        field bo <- cmavo(Bo).warn(ExperimentalTermBoConnection).wf();
        /// The simple term following BO.
        field trailing_term <- arc(simple_term);
    }

    /// Sum node for the term-level connective inventory.
    ///
    /// Both camxes-exp term tiers spell their connective `joik_ek` (camxes-exp.peg:153-154), and
    /// the owner-corrected domain for that position is JOIK or EK only (#795, #806). This
    /// deliberately diverges from camxes-exp's literal `joik_ek`, which also admits VUhU and
    /// reaches JA through its `joik`: the divergence is the I02 adjudication applied to the term
    /// site, and the rejected surfaces are witnessed with a documented-gap ledger row.
    rule "term connective" term_afterthought_connective -> enum {
        /// A JOI-family connective.
        joik_connective,
        /// An A-family connective.
        ek_connective,
    }

    /// Product node for term connection; preserves `leading_term` and `continuations` in source order.
    rule "term connection" connected_term(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(bound_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more connected_term_continuation(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection)];
    }

    /// Product node for term connection continuation; preserves `connective` and `trailing_term` in source order.
    rule "term connection continuation" connected_term_continuation(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        /// The `term_afterthought_connective` connective joining the adjacent constituents of the `connected_term_continuation` production.
        field connective <- term_afterthought_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(bound_term);
    }

    /// The NORMAL-flavour term constituent: the loose tier over an OPTIONAL-stag BO tier over
    /// the unguarded leaf inventory.
    ///
    /// camxes-exp writes the term hierarchy twice. The absorption-safe `abs_term` flavour models
    /// the ordinary sentence-term positions and is what the `nonabs_term` ladder above composes;
    /// this is the other one, `term <- term_1`, `term_1 <- term_2 (joik_ek !tag_bo_ke_bridi_tail
    /// !tag_bo_subsentence term_2)*`, `term_2 <- term_3 (joik_ek stag? BO_clause term_3)*`,
    /// `term_3 <- sumti / tag_term / termset` (camxes-exp.peg:134-149). It differs from the
    /// `abs_term` flavour in exactly two places: the stag before BO is OPTIONAL rather than
    /// mandatory, and every operand position takes the unguarded `tag_term` rather than the
    /// absorption-safe `abs_tag_term`.
    ///
    /// camxes-exp reaches it from three sites — the GOI payload (camxes-exp.peg:207), the
    /// NUhI-less termset operands (camxes-exp.peg:172) and the BE/BEI links (camxes-exp.peg:255,
    /// :266) — and camxes-standard spells the first two of those `nonabs_term`, its own bare
    /// unguarded leaf with no connective tier at all (camxes.peg:128, :138). The union of the two
    /// is therefore this family, and it is a family of its own rather than a widening of
    /// `nonabs_term`: the CEhE continuation that also consumes `nonabs_term` is sourced by
    /// camxes-standard's `nonabs_term` and camxes-exp's `abs_term` alone, so giving the whole
    /// ladder an optional-stag BO tier would admit a surface no parser accepts there.
    ///
    /// jbotci models the BE/BEI site separately as the `linked_term` family, whose leaf inventory
    /// is the four `linked_sumti` forms rather than the shared term leaves; widening that site is
    /// #816's half of the same upstream rule and is not this epoch's scope.
    ///
    /// Each level of this ladder splices the level below it, exactly as the other ladders do: a
    /// branch would add a public wrapper variant to Debug and serde output. The leaves are those
    /// of `normal_term_atom`.
    rule "term" normal_term(gek_termset, statement, exp_subsentence, term, bound_normal_term, normal_term_atom, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, normal_term, tanru_unit_atom, exp_guard_gek) -> enum {
        /// Uses the `connected_normal_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_normal_term,
        // The BO-bound level and the unguarded leaves below it.
        splice bound_normal_term,
    }

    /// The normal-flavour loose connection with one or more continuations.
    rule "term connection" connected_normal_term(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_guard();
        /// The first normal-flavour term at the loose precedence level.
        field leading_term <- arc(bound_normal_term);
        /// The nonempty source-ordered loose continuation sequence.
        field continuations <- [one_or_more connected_normal_term_continuation(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection)];
    }

    /// One normal-flavour loose continuation.
    rule "term connection continuation" connected_normal_term_continuation(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        /// The connective joining the adjacent normal-flavour terms.
        field connective <- term_afterthought_connective;
        /// The BO-bound normal-flavour term following the connective.
        field trailing_term <- arc(bound_normal_term);
    }

    /// The optional-stag BO-bound level of the normal-flavour term constituent.
    rule "term" bound_normal_term(gek_termset, statement, exp_subsentence, term, normal_term_atom, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, normal_term, tanru_unit_atom, exp_guard_gek) -> enum {
        /// Uses the diagnosed optional-stag BO-bound normal-flavour connection.
        bound_normal_term_connection,
        // The unguarded leaves.
        splice normal_term_atom,
    }

    /// The diagnosed optional-stag BO connection at the normal-flavour term level.
    ///
    /// camxes-standard has no term-level BO at all, so every occurrence is diagnosed, exactly as
    /// the mandatory-stag twin `stag_bound_term_connection` is. Unlike that twin the operands are
    /// the unguarded leaves, because camxes-exp's normal `term_2 <- term_3 (joik_ek stag?
    /// BO_clause term_3)*` (camxes-exp.peg:143) takes the unguarded `tag_term` on both sides.
    rule "term connection" bound_normal_term_connection(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier) -> struct {
        assert term_guard();
        /// The first unguarded leaf at the BO-bound precedence level.
        field leading_term <- arc(normal_term_atom);
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more normal_term_bo_continuation(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier)];
    }

    /// The BO continuation shape at the normal-flavour term level.
    ///
    /// The normal flavour leaves the stag optional (#816, camxes-exp.peg:143) but requires the
    /// connective. The sum has one arm only; it stays so that the trees keep their shape.
    rule "term connection continuation" normal_term_bo_continuation(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier) -> enum {
        /// Uses the sourced optional-stag `bound_normal_term_continuation` product form.
        bound_normal_term_continuation,
    }

    /// One optional-stag BO continuation at the normal-flavour term level.
    rule "term connection continuation" bound_normal_term_continuation(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier) -> struct {
        /// The connective joining the adjacent normal-flavour terms.
        field connective <- term_afterthought_connective;
        /// The optional camxes-exp `stag`; unlike the absorption-safe tier, the normal flavour
        /// leaves it out.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker, which owns the experimental warning for the whole connection.
        field bo <- cmavo(Bo).warn(ExperimentalTermBoConnection).wf();
        /// The unguarded leaf following BO.
        field trailing_term <- arc(normal_term_atom);
    }

    /// The unguarded leaf inventory of the normal-flavour term constituent.
    ///
    /// This is `term_3 <- sumti / tag_term / termset` (camxes-exp.peg:145) and camxes-standard's
    /// bare `nonabs_term` (camxes.peg:128) at once: the same leaves `simple_term` lists, with the
    /// unguarded `nonabs_tagged_sumti_term` and `nonabs_fa_chain_tagged_sumti_term` in place of
    /// their absorption-guarded twins. A splice cannot express that swap, so this rule lists its
    /// leaves itself, and the `normal_term_atom_swaps_only_the_guarded_tag_leaves` test in
    /// `grammar/mod.rs` checks that the two inventories stay aligned.
    rule "term" normal_term_atom(gek_termset, statement, exp_subsentence, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, normal_term, tanru_unit_atom, exp_guard_gek) -> enum {
        /// Uses the unguarded `nonabs_fa_chain_tagged_sumti_term` product form, camxes-exp's JOIK-chained FA tag.
        nonabs_fa_chain_tagged_sumti_term,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the unguarded `nonabs_tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        nonabs_tagged_sumti_term,
        /// Uses the `fihoi_proposal_adverbial_term` product form, whose payload preserves `fihoi`, `subsentence`, and `fihau`.
        fihoi_proposal_adverbial_term,
        /// Uses the `exp_soi_adverbial_term` wrapper, whose payload preserves the classified camxes-exp candidate.
        exp_soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
    }

    /// Product node for termset; preserves `leading_term` and `continuations` in source order.
    ///
    /// This is the CEhE level. Its leading operand is the full loose/BO term level, while each
    /// continuation takes the unguarded `nonabs` flavour, exactly as camxes.peg:116 pairs `term`
    /// with `nonabs_term`.
    rule "termset" termset_group(statement, sumti, loose_term, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(loose_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more termset_group_continuation(statement, sumti, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection)];
    }

    /// Product node for termset continuation; preserves `cehe` and `trailing_term` in source order.
    rule "termset continuation" termset_group_continuation(statement, sumti, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection) -> struct {
        /// The `Cehe` cmavo marker.
        field cehe <- cmavo(Cehe).wf();
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(nonabs_term);
    }

    /// The NUhI-gek forethought termset: `NUhI free* gek terms NUhU? free* gik terms NUhU? free*`
    /// (camxes.peg:136, camxes-exp.peg:191).
    ///
    /// This is the first of the three sourced termset shapes. The NUhI is MANDATORY: the NUhI-less
    /// surface is `gek_termset` in camxes-standard and camxes-exp alike, so an optional-NUhI
    /// reading of this arm would source its NUhU slots from nothing at all. Both operand
    /// positions are the full GUARDED `terms` sequences (B1), which is what separates this arm from
    /// the NUhI-less one.
    ///
    /// Product node for termset; preserves `nuhi`, `gek`, `terms`, and 2 other fields in source order.
    rule "termset" forethought_termset(term, tense_modal, selbri, letter_tokens) -> struct {
        /// The mandatory NUhI marker introducing the forethought termset before its connective.
        field nuhi <- cmavo(Nuhi).wf();
        /// The opening forethought connective that determines how the term sequences are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The initial nonempty term sequence following the opening connective.
        field terms <- [one_or_more arc(term)];
        /// The optional elidable NUhU terminator closing the initial term sequence.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
        /// The first GIK-led term-sequence branch paired with the opening connective.
        field first_branch <- forethought_termset_branch(term);
    }

    /// Product node for termset; preserves `gik`, `terms`, and `nuhu` in source order.
    rule "termset" forethought_termset_branch(term) -> struct {
        /// The GIK connective that introduces this branch and pairs with the opening forethought connective.
        field gik <- gik_connective;
        /// The nonempty term sequence governed by this branch's GIK connective.
        field terms <- [one_or_more arc(term)];
        /// The optional elidable NUhU terminator closing this branch's term sequence.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// The NUhI-less forethought termset: `gek_termset <- gek terms_gik_terms` (camxes.peg:136,
    /// camxes-exp.peg:191).
    ///
    /// This is the third of the three sourced termset shapes, and the only one that carries
    /// neither NUhI nor a NUhU slot. Its operands are single unguarded terms rather than the
    /// guarded `terms` sequences the NUhI-present arm takes (B1), and they are paired by nesting
    /// rather than by concatenation.
    ///
    /// The arm is extension-first against the baseline GEK sumti connection, which owns
    /// `ge ko'a gi ko'e broda` at `sumti_4` in camxes-standard and camxes-exp alike. Arm order
    /// alone cannot settle that, because a locally failing outer parse would let this arm reclaim
    /// the extent on backtracking, so the completed candidate is classified instead.
    rule "termset" gek_termset(balanced_termset_operands, tense_modal, selbri, letter_tokens) -> struct {
        #[tree_child(primary)]
        /// The completed NUhI-less candidate, retained only when the baseline GEK sumti connection
        /// does not own its identical extent.
        field termset <- arc(
            gek_termset_candidate(balanced_termset_operands, tense_modal, selbri, letter_tokens)
                .reject_output(crate::grammar::baseline_termset::BaselineGekSumtiRejection)
        );
    }

    /// The classified body of the NUhI-less forethought termset.
    rule "termset" gek_termset_candidate(balanced_termset_operands, tense_modal, selbri, letter_tokens) -> struct {
        /// The opening forethought connective that determines how the operands are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The balanced operand tree. Unlike the NUhI-present arm, the operand sequence is not a
        /// `terms` run: each level contributes exactly one leading and one trailing operand.
        field operands <- arc(balanced_termset_operands);
    }

    /// `terms_gik_terms <- normal_term (gik / terms_gik_terms) normal_term` (camxes.peg:138,
    /// camxes-exp.peg:193).
    ///
    /// Each level pairs one leading operand with one trailing operand around a centre that is
    /// either the GIK itself or the next nested pair, so an n-operand termset nests n/2 deep and
    /// the outermost operands are the outermost pair. The GIK alternative is listed first, exactly
    /// as upstream orders it, so the innermost pair is the one that finds the GIK.
    rule "termset" balanced_termset_operands(balanced_termset_operands, normal_term) -> enum {
        /// Uses the `gik_paired_termset_operands` product form, whose payload preserves
        /// `leading_operand`, `gik`, and `trailing_operand`.
        gik_paired_termset_operands,
        /// Uses the `nested_paired_termset_operands` product form, whose payload preserves
        /// `leading_operand`, `inner`, and `trailing_operand`.
        nested_paired_termset_operands,
    }

    /// The innermost operand pair, which is the one that carries the GIK.
    rule "termset" gik_paired_termset_operands(normal_term) -> struct {
        /// The operand before the GIK.
        field leading_operand <- arc(normal_term);
        /// The GIK connective that pairs with the opening forethought connective.
        field gik <- gik_connective;
        /// The operand after the GIK.
        field trailing_operand <- arc(normal_term);
    }

    /// An outer operand pair wrapped around the next nested pair.
    rule "termset" nested_paired_termset_operands(balanced_termset_operands, normal_term) -> struct {
        /// The operand before the nested pair.
        field leading_operand <- arc(normal_term);
        /// The nested operand pair.
        field inner <- arc(balanced_termset_operands);
        /// The operand after the nested pair.
        field trailing_operand <- arc(normal_term);
    }

    /// Product node for termset; preserves `nuhi`, `termset`, and `nuhu` in source order.
    rule "termset" nuhi_termset(term) -> struct {
        /// The `Nuhi` cmavo marker.
        field nuhi <- cmavo(Nuhi).wf();
        /// Non-empty ordered sequence of termset components.
        field termset <- [one_or_more arc(term)];
        /// The optional `Nuhu` cmavo marker.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    alias "tag" standard_forethought_tense_modal(baseline_term_tense_modal, tense_modal) =
        baseline_term_tense_modal.map_to(tense_modal);

    // ---- the SOI/FIhOI adverbial pair ------------------------------------------------
    //
    // Two sources spell an adverbial here and they do not agree, so the arms are
    // source-qualified and keyed on the exact cmavo rather than on one widened selma'o:
    //
    //   camxes-exp  SOI <- soi / xoi / fi'oi (:1842), SUBSENTENCE body, SEhU
    //               as an arm of both `tag_term` (:149) and `abs_tag_term` (:160)
    //   New-FIhOI   FIhOI <- ku'au / fi'oi (selpahi-mex.peg:1993), SUBSENTENCE body, FIhAU
    //
    // The shape jbotci carried before epoch 8 -- a statement body closed by FIhAU -- is in
    // neither source, so it retired. `ku'au` is a retained source gap: the proposal grammar's
    // second FIhOI word is not adopted here.
    //
    // Arm order is what the boundaries need. The proposal arm requires an explicit FIhAU, so
    // it is structurally disjoint and runs first; an elided-FIhAU extent is the camxes-exp
    // arm's under R2, which is the source precedence the shared cell freezes.

    /// Product node for FIhOI adverbial; preserves `fihoi`, `subsentence`, and `fihau` in source order.
    rule "FIhOI adverbial" fihoi_proposal_adverbial_term(exp_subsentence) -> struct {
        /// The `Fihoi` cmavo marker.
        field fihoi <- cmavo(Fihoi).warn(ExperimentalFihoiAdverbial).wf();
        /// The shared subsentence child syntax node.
        field subsentence <- arc(exp_subsentence);
        /// The required `Fihau` terminator, which is what selects the proposal arm.
        field fihau <- cmavo(Fihau).wf();
    }

    /// Transparent ownership wrapper for the camxes-exp SOI adverbial.
    rule "SOI adverbial" exp_soi_adverbial_term(exp_subsentence) -> struct {
        #[tree_child(primary)]
        /// The completed candidate, retained only where the baseline reciprocal does not own its reparse.
        field adverbial <- arc(
            exp_soi_subsentence_adverbial(exp_subsentence)
                .reject_output(crate::grammar::baseline_relative::BaselineReciprocalSoiRejection)
        );
    }

    /// Product node for SOI adverbial; preserves `soi`, `subsentence`, and `sehu` in source order.
    rule "SOI adverbial" exp_soi_subsentence_adverbial(exp_subsentence) -> struct {
        /// The SOI marker, warned under the neutral marker-anchored category for its word.
        field soi <- choice((
            cmavo(Soi).warn(ExperimentalSoiAdverbial),
            cmavo(Xoi).warn(ExperimentalSoiAdverbial),
            cmavo(Fihoi).warn(ExperimentalFihoiAdverbial),
        )).wf();
        /// The shared subsentence child syntax node.
        field subsentence <- arc(exp_subsentence);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    /// Transparent product node for term; preserves the `sumti` component.
    rule "term" sumti_term(sumti, term, tense_modal, baseline_term_tense_modal, selbri, letter_tokens) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// camxes-exp's tag term whose tag is a JOIK- or JEK-connected run of FA place tags:
    /// `tag_term <- !gek tag free* (sumti / KU_elidible free*)` (camxes-exp.peg:149), with
    /// `tag <- tense_modal (joik_jek tense_modal)*` (:372) and FA inside `tense_modal` (:378).
    ///
    /// The connected run is required, so a plain `fa ko'a` never reaches this arm and stays the
    /// baseline FA term. This is the guarded twin, camxes-exp's `abs_tag_term <- !gek tag free*
    /// !selbri !gek_sentence (sumti / KU_elidible free*)` (:160). `!gek` refuses a chain that
    /// opens a forethought connective, as in `fa je fe gi`. `!selbri` keeps `fa je fe broda`
    /// camxes-exp's tagged selbri, and `!gek_sentence` refuses `fa je fe ge broda gi brode`.
    /// Both guards use the guard-only recognizers below.
    /// Each FA is a tag atom here and warns as one.
    rule "place tag" fa_chain_tagged_sumti_term(sumti, normal_term, selbri, exp_guard_gek, exp_gek_sentence_guard) -> struct {
        assert !exp_guard_gek;
        /// The first FA tag atom.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        /// Non-empty source-ordered connective-led FA continuations.
        field continuations <- [one_or_more fa_chain_tag_continuation()];
        assert !selbri;
        assert !exp_gek_sentence_guard;
        /// The shared sumti child syntax node, overt or KU-terminated.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    // ---- guard-only recognizers for the FA chain ------------------------------------------
    //
    // camxes-exp's tag terms refuse to start at a GEK and refuse an elided KU before a GEK
    // sentence (`!gek` and `!gek_sentence`, camxes-exp.peg:149, :160). Without those
    // reservations the FA chain would take an elided KU and leave a forethought sentence for the
    // bridi tail, a split that camxes-exp never derives (`fa je fe ge broda gi brode`).
    //
    // The rules in this section own that reservation for jbotci. They refuse a FA chain with an
    // elided KU wherever the rest of the input reads as a GEK opener or a forethought sentence
    // in jbotci's own language, extended only by what the FA chain itself adopts from
    // camxes-exp:
    //
    // - FA atoms among the tag atoms: an operand between connectives is a run of jbotci's
    //   baseline tag atoms (which read forms such as `pu nai`) and single camxes-exp atoms;
    // - the merged tag connectives of camxes-exp's `joik` and `joik_jek` (:347, :358): JOI, JA
    //   and A with optional NA, SE and NAI, the simple and the GAhO-closed intervals, and VUhU,
    //   both between tags and in the JOIK-GI and `ga` + JOIK-JEK openers of `gek` (:361, :364).
    //
    // The sentence recognizer follows the arms of camxes-exp's `gek_sentence` (:112): a GEK pair,
    // tags before KE, and NA, with the KE and NA arms recursing through the guard so that the
    // extended domain holds at every depth. Its subsentences are jbotci's own `subbridi`.
    //
    // The section does not rebuild camxes-exp's clause, indicator or free-modifier language.
    // Indicators, NAI and free modifiers are read as the surrounding jbotci grammar reads them,
    // under jbotci's own policies (#847, #848). The rules only look ahead inside negative
    // assertions and never build a tree. The product grammar does not take the extended forms:
    // they belong to #982 or are camxes-exp-only (#980). When #982 adds one of them, a product
    // rule that then reads the same language replaces the recognizer here.

    /// A forethought sentence as the FA chain's `!gek_sentence` guard reads it: a GEK opener
    /// from [`exp_guard_gek`], a `subbridi`, a GIK, a `subbridi` and tail terms; any number of
    /// extended tags from [`exp_guard_tag`] before KE and a recognized sentence; or NA before a
    /// recognized sentence. The KE and NA arms recurse through this rule.
    rule "forethought bridi connection" exp_gek_sentence_guard(exp_gek_sentence_guard, exp_guard_gek, subbridi, term, selbri, sumti, mekso, letter_tokens, letter_string) -> enum {
        /// The GEK pair.
        exp_gek_sentence_guard_pair,
        /// Tags before KE and a recognized sentence.
        exp_gek_sentence_guard_grouped,
        /// NA before a recognized sentence.
        exp_gek_sentence_guard_negated,
    }

    /// The GEK pair of [`exp_gek_sentence_guard`].
    rule "forethought bridi connection" exp_gek_sentence_guard_pair(exp_guard_gek, subbridi, term) -> struct {
        /// The GEK opener.
        field gek <- arc(exp_guard_gek);
        /// The first subsentence.
        field first <- arc(subbridi);
        /// The GIK.
        field gik <- gik_connective;
        /// The second subsentence.
        field second <- arc(subbridi);
        /// The tail terms.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(cmavo(Vau).wf()).elidable_terminator(Vau);
    }

    /// The KE arm of [`exp_gek_sentence_guard`].
    rule "forethought bridi connection" exp_gek_sentence_guard_grouped(exp_gek_sentence_guard, selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The extended tags before KE.
        field tags <- [zero_or_more exp_guard_tag(selbri, sumti, mekso, letter_tokens, letter_string)];
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The grouped forethought sentence.
        field inner <- arc(exp_gek_sentence_guard);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// The NA arm of [`exp_gek_sentence_guard`].
    rule "forethought bridi connection" exp_gek_sentence_guard_negated(exp_gek_sentence_guard) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        /// The negated forethought sentence.
        field inner <- arc(exp_gek_sentence_guard);
    }

    /// A GEK opener as the FA chain's guards read it, in the arm order of camxes-exp's `gek`
    /// (:361): `ga` + JOIK-JEK, SE? GA, JOIK GI, and an extended tag before GIK. The `ga` arm
    /// refuses a following opener, as camxes-exp's `gak` does (:364), through this rule.
    rule "forethought connective" exp_guard_gek(exp_guard_gek, selbri, sumti, mekso, letter_tokens, letter_string) -> enum {
        /// `ga` + JOIK-JEK.
        exp_guard_gaja_gek,
        /// SE? GA.
        exp_guard_ga_gek,
        /// JOIK GI.
        exp_guard_joik_gi_gek,
        /// An extended tag before GIK.
        exp_guard_stag_gik_gek,
    }

    /// `ga` + JOIK-JEK.
    rule "forethought connective" exp_guard_gaja_gek(exp_guard_gek) -> struct {
        /// The cmavo `ga` itself, not the whole GA selma'o.
        field ga <- cmavo(Ga).wf();
        assert !exp_guard_gek;
        /// The optional se component.
        field se <- opt(selmaho(Se));
        /// The connective.
        field connective <- exp_guard_joik_jek;
    }

    /// SE? GA, with the optional NAI that jbotci's own GA opener takes.
    rule "forethought connective" exp_guard_ga_gek -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        /// A word from selmaho `Ga`.
        field ga <- selmaho(Ga).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// JOIK GI, with the optional NAI that jbotci's own tag-GI opener takes.
    rule "forethought connective" exp_guard_joik_gi_gek -> struct {
        /// The JOIK.
        field joik <- exp_guard_joik;
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// An extended tag before GIK.
    rule "forethought connective" exp_guard_stag_gik_gek(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The extended tag.
        field stag <- exp_guard_tag(selbri, sumti, mekso, letter_tokens, letter_string);
        /// The GIK.
        field gik <- gik_connective;
    }

    /// An extended tag: runs of jbotci tag atoms, FA atoms included, linked by the merged
    /// connectives, in the shape of camxes-exp's `tag` and `stag` (:372, :375).
    rule "connected tag" exp_guard_tag(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The first run of adjacent tag atoms.
        field first <- exp_guard_tag_run(selbri, sumti, mekso, letter_tokens, letter_string);
        /// The linked runs after it.
        field continuations <- [zero_or_more exp_guard_tag_continuation(selbri, sumti, mekso, letter_tokens, letter_string)];
    }

    /// One link of [`exp_guard_tag`].
    rule "connected tag continuation" exp_guard_tag_continuation(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The merged connective.
        field connective <- exp_guard_joik_jek;
        /// The next run of adjacent tag atoms.
        field run <- exp_guard_tag_run(selbri, sumti, mekso, letter_tokens, letter_string);
    }

    /// One operand of [`exp_guard_tag`]: one or more adjacent tag atoms with no connective
    /// between them, as camxes-exp's `tense_modal` is a run of atoms (:378). So `pu nai`, FA
    /// atoms and BAI may follow one another, as in `fa je fe pu nai bau gi` and
    /// `fa je fe fa pu nai gi`.
    rule "connected tag" exp_guard_tag_run(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The first tag atom.
        field first <- exp_guard_tag_atom(selbri, sumti, mekso, letter_tokens, letter_string);
        /// The adjacent tag atoms after it.
        field additional <- [zero_or_more exp_guard_tag_atom(selbri, sumti, mekso, letter_tokens, letter_string)];
    }

    /// One tag atom of [`exp_guard_tag_run`]. The baseline arms of jbotci's `tense_modal_atom`
    /// come first, so a baseline tense keeps its NAI (`pu nai`) and its compound forms. Then one
    /// prefixed camxes-exp atom, which adds FA. The atom is single on purpose: the camxes-exp arm
    /// of `tense_modal_atom` reads a whole run of atoms without NAI, so it would take `fa pu` in
    /// `fa pu nai gi` and leave the NAI behind.
    rule "tag" exp_guard_tag_atom(selbri, sumti, mekso, letter_tokens, letter_string) -> enum {
        // The baseline atoms.
        splice baseline_term_tense_modal_atom,
        /// One prefixed camxes-exp tag atom, FA included.
        exp_prefixed_tag_atom,
    }

    /// The merged connectives between tags: [`exp_guard_joik`] or VUhU, as in camxes-exp's
    /// `joik_jek` (:358). JA, the `jek` of that rule, is in the JOIK arm.
    rule "joik" exp_guard_joik_jek -> enum {
        /// The merged JOIK.
        exp_guard_joik,
        /// VUhU.
        vuhu_nonlogical_connective,
    }

    /// The merged JOIK of camxes-exp's `joik` (:347): JOI, JA or A with optional NA, SE and
    /// NAI, the simple interval, and the GAhO-closed interval.
    rule "joik" exp_guard_joik -> enum {
        /// JOI, JA or A with optional NA, SE and NAI.
        exp_guard_logical_connective,
        /// The simple interval.
        simple_interval_connective,
        /// The GAhO-closed interval.
        closed_interval_connective,
    }

    /// JOI, JA or A with optional NA, SE and NAI.
    rule "joik" exp_guard_logical_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        /// The JOI, JA or A word.
        field head <- choice((selmaho(Joi), selmaho(Ja), selmaho(A))).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// The unguarded twin of [`fa_chain_tagged_sumti_term`], for the `nonabs_term` ladder:
    /// camxes-exp's `tag_term <- !gek tag free* (sumti / KU_elidible free*)` (:149), which keeps
    /// `!gek` but has no selbri or forethought-sentence guard.
    rule "place tag" nonabs_fa_chain_tagged_sumti_term(sumti, normal_term, exp_guard_gek) -> struct {
        assert !exp_guard_gek;
        /// The first FA tag atom.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        /// Non-empty source-ordered connective-led FA continuations.
        field continuations <- [one_or_more fa_chain_tag_continuation()];
        /// The shared sumti child syntax node, overt or KU-terminated.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// One `joik_jek tense_modal` continuation of a FA tag chain whose atom is FA.
    rule "place tag continuation" fa_chain_tag_continuation -> struct {
        /// The JOIK or JEK connective between adjacent FA tags.
        field connective <- tense_modal_connective;
        /// The next FA tag atom.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    /// Product node for place tag; preserves `fa` and `sumti` in source order.
    rule "place tag" place_tagged_sumti_term(sumti, normal_term) -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Product node for NA KU term; preserves `na` and `na_ku` in source order.
    rule "NA KU term" na_ku_term -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na);
        /// The `Ku` cmavo marker.
        field na_ku <- cmavo(Ku).wf();
    }

    /// Transparent product node for NA term; preserves the `na` component.
    rule "NA term" bare_na_term(selbri, tense_modal, letter_tokens) -> struct {
        // camxes-exp's bare NA term begins with `!joik_jek` (camxes-exp.peg:160), checked before
        // NA, and its merged `joik` (:347) reads `NA SE? (JOI / JA / A)` as one connective. So
        // `na joi` and `na se je` never begin a bare NA term there, but a free modifier between
        // NA and JOI does, because NA_clause takes no free modifier. The default grammar keeps
        // its bare NA term before JOI; the `na-joik` feature gives it camxes-exp's guard.
        assert !feature(NaJoik, (selmaho(Na), opt(selmaho(Se)), choice((selmaho(Joi), selmaho(Ja)))));
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        assert !choice((
            selbri
                .reject_output(crate::grammar::baseline_tag::PostNaExtensionTagRejection)
                .ignored(),
            modal_forethought_connective(tense_modal, selbri, letter_tokens).ignored(),
            selmaho(Ja).ignored(),
            (
                opt(selmaho(Se)),
                selmaho(A),
            ).ignored(),
            (
                opt(selmaho(Se)),
                selmaho(Giha),
            ).ignored(),
        ));
    }

    /// Transparent product node for tag; preserves the `tense_modal` component.
    rule "tag" tagged_sumti_before_tag_term(tense_modal, baseline_term_tense_modal, selbri, letter_tokens, letter_string) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal,
            tense_modal,
            selbri,
            letter_tokens,
            letter_string,
        ));
        assert tense_modal.lookahead();
    }

    /// Product node for tag; preserves `tense_modal` and `sumti` in source order.
    rule "tag" tagged_sumti_term(tense_modal, baseline_term_tense_modal, sumti, selbri, letter_tokens, letter_string, normal_term) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal,
            tense_modal,
            selbri,
            letter_tokens,
            letter_string,
        ));
        assert !selbri;
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Product node for the unguarded (`nonabs`) tag term; preserves `tense_modal` and `sumti`.
    ///
    /// camxes-standard's `nonabs_term` (camxes.peg:128) is `term_1` without the absorption guard
    /// `!(!tag selbri)`, so a tag with an elided KU may stand directly before the selbri. The
    /// guarded twin is `tagged_sumti_term`; the two rules differ only by that assertion. The
    /// `normal_term_atom_swaps_only_the_guarded_tag_leaves` test in `grammar/mod.rs` checks that the
    /// flavoured leaf inventories stay aligned.
    rule "tag" nonabs_tagged_sumti_term(tense_modal, baseline_term_tense_modal, sumti, selbri, letter_tokens, letter_string, normal_term) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal,
            tense_modal,
            selbri,
            letter_tokens,
            letter_string,
        ));
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Final experimental tag term for the A21 elided-FEhU NAhE/FIhO surface.
    rule "tag" elided_nahe_fiho_tag_term(tense_modal, sumti, normal_term) -> struct {
        assert (selmaho(Nahe), cmavo(Fiho)).lookahead();
        /// The exact extension-owned NAhE/FIhO tag.
        field tense_modal <- arc(
            tense_modal.reject_output(crate::grammar::baseline_tag::NonElidedNaheFihoTagTermRejection)
        );
        /// The elided sumti following a final tag term.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Sum node for tag; selects among 8 forms including `pu_before_nahe_leading_term_tag_tense`, `pu_distance_before_tag_leading_term_tag_tense`, and `zi_before_zi_leading_term_tag_tense`.
    rule "tag" leading_term_tag_tense_modal(baseline_term_tense_modal, tense_modal, selbri, letter_tokens, letter_string) -> enum {
        /// Uses the `pu_before_nahe_leading_term_tag_tense` product form, whose payload preserves `pu` and `nai`.
        pu_before_nahe_leading_term_tag_tense,
        /// Uses the `pu_distance_before_tag_leading_term_tag_tense` product form, whose payload preserves `pu`, `nai`, and `distance`.
        pu_distance_before_tag_leading_term_tag_tense,
        /// Uses the `zi_before_zi_leading_term_tag_tense` product form, whose payload preserves `zi`.
        zi_before_zi_leading_term_tag_tense,
        /// Uses the `va_before_va_leading_term_tag_tense` product form, whose payload preserves `va`.
        va_before_va_leading_term_tag_tense,
        /// Uses the `mohi_before_mohi_leading_term_tag_tense` product form, whose payload preserves `mohi`, `direction`, `nai`, and `distance`.
        mohi_before_mohi_leading_term_tag_tense,
        /// Uses the `caha_before_tag_leading_term_tag_tense` product form, whose payload preserves `caha`.
        caha_before_tag_leading_term_tag_tense,
        /// Uses the `interval_property_leading_term_tag_tense` product form, whose payload preserves `property`.
        interval_property_leading_term_tag_tense,
        /// The baseline term tag, mapped to the shared `tense_modal` product form.
        standard_forethought_tense_modal,
    }

    /// Product node for tag; preserves `pu` and `nai` in source order.
    rule "tag" pu_before_nahe_leading_term_tag_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        assert selmaho(Nahe);
    }

    /// Product node for tag; preserves `pu`, `nai`, and `distance` in source order.
    rule "tag" pu_distance_before_tag_leading_term_tag_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// A word from selmaho `Zi`.
        field distance <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Transparent product node for tag; preserves the `zi` component.
    rule "tag" zi_before_zi_leading_term_tag_tense -> struct {
        /// A word from selmaho `Zi`.
        field zi <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Transparent product node for tag; preserves the `va` component.
    rule "tag" va_before_va_leading_term_tag_tense -> struct {
        /// A word from selmaho `Va`.
        field va <- selmaho(Va).wf();
        assert selmaho(Va);
    }

    /// Product node for tag; preserves `mohi`, `direction`, `nai`, and `distance` in source order.
    rule "tag" mohi_before_mohi_leading_term_tag_tense -> struct {
        /// A word from selmaho `Mohi`.
        field mohi <- selmaho(Mohi).wf();
        /// A word from selmaho `Faha`.
        field direction <- selmaho(Faha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional distance component.
        field distance <- opt(selmaho(Va).wf());
        assert selmaho(Mohi);
    }

    /// Transparent product node for tag; preserves the `caha` component.
    rule "tag" caha_before_tag_leading_term_tag_tense(tense_modal) -> struct {
        /// A word from selmaho `Caha`.
        field caha <- selmaho(Caha).wf().followed_by(tense_modal.lookahead());
    }

    /// Transparent product node for interval property; preserves the `property` component.
    rule "interval property" interval_property_leading_term_tag_tense(selbri, letter_tokens, letter_string) -> struct {
        /// The shared property child syntax node.
        field property: std::sync::Arc<IntervalPropertyTenseSyntax> <- arc(interval_property_tense(letter_tokens, letter_string).followed_by(choice((
            selmaho(Pu).ignored(),
            selmaho(Zi).ignored(),
            selmaho(Zeha).ignored(),
            (
                selmaho(Nahe),
                selmaho(Caha),
            ).ignored(),
            modal_tense().ignored(),
            fiho_tense(selbri).ignored(),
        )).lookahead()));
    }

    /// Sum node for sumti; selects among the `sumti` and `tagged_elided_sumti` forms.
    rule "sumti" tagged_or_elided_sumti(sumti, normal_term) -> enum {
        /// Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.
        sumti,
        /// Uses the `tagged_elided_sumti` product form, whose payload preserves `maybe_ku`.
        tagged_elided_sumti,
    }

    /// Transparent product node for elided sumti; preserves the `maybe_ku` component.
    rule "elided sumti" tagged_elided_sumti -> struct {
        /// The optional `Ku` cmavo marker.
        field maybe_ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for sumti; preserves `base_sumti` and `vuho_attachment` in source order.
    rule "sumti" sumti(sumti, sumti_grouped, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The shared base sumti child syntax node.
        field base_sumti <- arc(sumti_grouped);
        /// The optional vuho attachment component.
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `grouped_tail` in source order.
    rule "sumti connection" sumti_grouped(sumti, sumti_afterthought, tense_modal, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_afterthought);
        /// The optional grouped tail component.
        field grouped_tail <- opt(grouped_sumti_tail(sumti, tense_modal));
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `continuations` in source order.
    rule "sumti connection" sumti_afterthought(sumti_bound, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_bound);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more sumti_afterthought_tail(sumti_bound)];
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `bound_tail` in source order.
    rule "sumti connection" sumti_bound(sumti_bound, sumti_forethought, tense_modal, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_forethought);
        /// The optional bound tail component.
        field bound_tail <- opt(sumti_bound_tail(sumti_bound, tense_modal));
    }

    /// The BO-bound tail shape of the sumti connection.
    ///
    /// camxes-standard and camxes-exp both require the connective before the optional stag:
    /// `sumti_3 <- sumti_4 ((ek / joik) stag? BO_clause sumti_3)?` (camxes.peg:143). The sum has
    /// one arm only; it stays so that the trees keep their shape.
    rule "sumti connection" sumti_bound_tail(sumti_bound, tense_modal) -> enum {
        /// Uses the sourced `bound_sumti_tail` product form, whose payload preserves
        /// `connective`, `tense_modal`, `bo`, and `trailing_sumti`.
        bound_sumti_tail,
    }

    /// Sum node for sumti; selects among the `forethought_sumti` and `simple_sumti` forms.
    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, description_leading_operand, subbridi, tense_modal, mekso, selbri, letter_tokens, free_modifier, statement, normal_term, quantifier) -> enum {
        /// Uses the `forethought_sumti` product form, whose payload preserves `gek`, `leading_sumti`, and `first_branch`.
        forethought_sumti,
        /// Uses the `simple_sumti` product form, whose payload preserves `base_sumti` and `relative_clauses`.
        simple_sumti,
    }

    /// Product node for forethought sumti connection; preserves `gek`, `leading_sumti`, and `first_branch` in source order.
    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal, statement, selbri, letter_tokens) -> struct {
        /// The opening forethought connective that determines how the sumti branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The first sumti branch, which follows the opening connective without an intervening GIK.
        field leading_sumti <- arc(sumti);
        /// The first GIK-led sumti branch paired with the opening connective.
        field first_branch <- forethought_sumti_branch(sumti_forethought);
    }

    /// Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.
    rule "forethought sumti connection" forethought_sumti_branch(sumti_forethought) -> struct {
        /// The GIK connective that introduces this branch and pairs with the opening forethought connective.
        field gik <- gik_connective;
        /// The sumti governed by this branch's GIK connective.
        field sumti <- arc(sumti_forethought);
    }

    /// Product node for sumti connection; preserves `connective`, `tense_modal`, `bo`, and `trailing_sumti` in source order.
    rule "sumti connection" bound_sumti_tail(sumti_bound, tense_modal) -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(sumti_connective);
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared trailing sumti child syntax node.
        field trailing_sumti <- arc(sumti_bound);
    }

    /// Product node for sumti connective; preserves `connective` and `sumti` in source order.
    rule "sumti connective" sumti_afterthought_tail(sumti_bound) -> struct {
        /// The `sumti_connective` connective joining the adjacent constituents of the `sumti_afterthought_tail` production.
        field connective <- sumti_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_bound);
    }

    /// Product node for sumti connection; preserves `connective`, `tense_modal`, `ke`, `inner_sumti`, and `kehe` in source order.
    rule "sumti connection" grouped_sumti_tail(sumti, tense_modal) -> struct {
        /// The `sumti_connective` connective joining the adjacent constituents of the `grouped_sumti_tail` production.
        field connective <- sumti_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for sumti relative phrase; tries the structurally closed scoped-continuation route before baseline VUhO-relative ownership and the bare-VUhO extension.
    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, normal_term) -> enum {
        /// Experimental VUhO-scoped continuation with required relatives and one required sumti continuation, reachable only immediately before explicit LUhU.
        experimental_vuho_scoped_sumti_attachment_tail,
        /// Baseline VUhO followed by a required relative-clause list.
        vuho_relative_sumti_attachment_tail,
        /// Experimental bare VUhO attachment.
        experimental_bare_vuho_sumti_attachment_tail,
    }

    /// Product node for baseline sumti relative phrase; preserves `vuho` and required `relative_clauses` in source order.
    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// The `Vuho` cmavo marker.
        field vuho <- cmavo(Vuho).wf();
        /// The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `vuho_relative_sumti_attachment_tail` production.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, normal_term);
    }

    /// Product node for the camxes-exp VUhO-scoped continuation; preserves `vuho`, required `relative_clauses`, and required `sumti_connection` in source order.
    rule "sumti relative phrase" experimental_vuho_scoped_sumti_attachment_tail(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// The warning-gated `Vuho` marker that identifies experimental scoped ownership.
        field vuho <- cmavo(Vuho).warn(ExperimentalVuhoScopedAttachment).wf();
        /// Required relative clauses scoped together with the continuation.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, normal_term);
        /// The required sumti continuation child.
        field sumti_connection <- arc(sumti_connection_tail(sumti));
        // The explicit wrapper boundary makes closed-consumer ownership structural. Without this
        // lookahead, ordering the longer arm first would steal the generic top-level term
        // connection; ordering the shorter baseline arm first cannot be reconsidered after an
        // enclosing LUhU fails because parser choice is locally committed.
        assert cmavo(Luhu).lookahead();
    }

    /// Product node for the camxes-exp bare-VUhO extension.
    rule "sumti relative phrase" experimental_bare_vuho_sumti_attachment_tail -> struct {
        #[tree_child(primary)]
        /// The warning-gated bare `Vuho` marker.
        field vuho <- cmavo(Vuho).warn(ExperimentalVuhoScopedAttachment).wf();
    }

    /// Product node for sumti; preserves `base_sumti` and `relative_clauses` in source order.
    rule "sumti" simple_sumti(sumti, sumti_base, description_leading_operand, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term, quantifier) -> struct {
        /// The shared base sumti child syntax node.
        field base_sumti <- arc(sumti_atom(sumti, sumti_base, description_leading_operand, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term, quantifier));
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Sum node for sumti; selects among the `sumti_base` and `quantified_sumti` forms.
    rule "sumti" sumti_atom(sumti, sumti_base, description_leading_operand, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term, quantifier) -> enum {
        /// Uses the nested `sumti_base` sum form and preserves its selected alternative.
        sumti_base,
        /// Uses the `quantified_sumti` product form, whose payload preserves `quantifier` and `inner_sumti`.
        quantified_sumti,
    }

    /// Sum node for sumti; selects among 17 forms including `scalar_negated_sumti_with_bo`, `scalar_negated_sumti`, and `lahe_sumti`.
    rule "sumti" sumti_base(sumti, description_leading_operand, term, subbridi, exp_subsentence, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement, normal_term, quantifier) -> enum {
        /// Uses the `scalar_negated_sumti_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_sumti`, and `luhu`.
        scalar_negated_sumti_with_bo,
        /// Uses the `scalar_negated_sumti` product form, whose payload preserves `nahe`, `inner_sumti`, and `luhu`.
        scalar_negated_sumti,
        /// Uses the `lahe_sumti` product form, whose payload preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu`.
        lahe_sumti,
        /// Uses the `lahe_term_wrapper` product form, whose payload preserves `lahe`, `inner_term`, and `luhu`.
        lahe_term_wrapper,
        /// Uses the `scalar_negated_term_wrapper_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_term`, and `luhu`.
        scalar_negated_term_wrapper_with_bo,
        /// Uses the `scalar_negated_term_wrapper` product form, whose payload preserves `nahe`, `inner_term`, and `luhu`.
        scalar_negated_term_wrapper,
        /// Uses the camxes-exp `bridi_description_sumti` product form, whose payload preserves `lohoi`, `subbridi`, and `kuhau`.
        bridi_description_sumti,
        /// Uses the `name_sumti` product form, whose payload preserves `la`, `relative_clauses`, and `names`.
        name_sumti,
        /// Uses the `descriptor_with_outer_quantifier_sumti` product form, whose payload preserves `outer_quantifier`, `description`, `tail`, and `ku`.
        descriptor_with_outer_quantifier_sumti,
        /// Uses the `descriptor_with_gadri_sumti` product form, whose payload preserves `description`, `tail`, and `ku`.
        descriptor_with_gadri_sumti,
        /// Uses the camxes-exp `exp_descriptor_with_leading_sumti_sumti` product form, whose payload preserves `description`, `tail`, and `ku`.
        exp_descriptor_with_leading_sumti_sumti,
        /// Uses the `descriptor_without_gadri_sumti` product form, whose payload preserves `quantifier`, `selbri`, `ku`, and `relative_clauses`.
        descriptor_without_gadri_sumti,
        /// Uses the `number_sumti` product form, whose payload preserves `li`, `expression`, and `loho`.
        number_sumti,
        /// Uses the `lerfu_string_sumti` product form, whose payload preserves `words`, `boi`, and `free_modifiers`.
        lerfu_string_sumti,
        /// Uses the `quoted_sumti` product form, whose payload preserves `quote`.
        quoted_sumti,
        /// Uses the `pro_sumti` product form, whose payload preserves `koha`.
        pro_sumti,
    }

    /// camxes-exp's LOhOI bridi description, the `sumti_6` arm
    /// `LOhOI_clause free* subsentence KUhAU_elidible free*` (camxes-exp.peg:189). The body is a
    /// subsentence: camxes-exp has no joik chain of LOhOI heads and no I-connected statement body.
    rule "bridi description" bridi_description_sumti(exp_subsentence) -> struct {
        /// The LOhOI marker, which carries the warning for the whole construct.
        field lohoi <- cmavo(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
        #[tree_child(primary)]
        /// The described subsentence, through the camxes-exp `subsentence` entry.
        field subbridi <- arc(exp_subsentence);
        /// The optional `Kuhau` cmavo marker.
        field kuhau <- opt(cmavo(Kuhau).wf()).elidable_terminator(Kuhau);
    }

    /// Product node for quantified sumti; preserves `quantifier` and `inner_sumti` in source order.
    rule "quantified sumti" quantified_sumti(description_leading_operand, mekso, letter_tokens, free_modifier, quantifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantified_sumti` production.
        field quantifier <- quantifier;
        /// The shared inner sumti child syntax node, restricted to the camxes `sumti_6` operand
        /// tier: camxes spells this site `sumti_5 <- quantifier? sumti_6`, so an outer quantifier
        /// can never take a quantifier-bearing operand (#837 SUM-02).
        field inner_sumti <- arc(description_leading_operand);
    }

    /// Product node for sumti connective; preserves `connective` and `sumti` in source order.
    rule "sumti connective" sumti_connection_tail(sumti) -> struct {
        /// The `sumti_connective` connective joining the adjacent constituents of the `sumti_connection_tail` production.
        field connective <- sumti_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Product node for quantifier; preserves `number`, `boi`, and `free_modifiers` in source order.
    rule "quantifier" pa_run_quantifier(letter_tokens, free_modifier) -> struct {
        /// The `number_words` grammar result in the `number` structural role of the `pa_run_quantifier` production.
        field number <- number_words(letter_tokens);
        assert !selmaho(Moi);
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi).wf()).elidable_terminator(Boi);
        /// Free modifiers following the optional BOI terminator.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Product node for quantifier; preserves `vei`, `mekso`, and `veho` in source order.
    rule "quantifier" mekso_quantifier(mekso) -> struct {
        /// The `Vei` cmavo marker.
        field vei <- cmavo(Vei).wf();
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
        /// The optional `Veho` cmavo marker.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// Sum node for quantifier; selects among the `mekso_quantifier` and `pa_run_quantifier` forms.
    rule "quantifier" quantifier(mekso, letter_tokens, free_modifier, quantifier, sumti, description_leading_operand, term, subbridi, exp_subsentence, selbri, text, tense_modal, letter_string, statement, normal_term) -> enum {
        /// camxes-exp's raw-mex quantifier, under the `mex-quantifier` dialect feature, because
        /// it changes the reading of some texts that the default grammar accepts (#982). It is
        /// tried first, but it never owns a baseline surface: it refuses a mex that is exactly
        /// one number operand or one VEI operand, so those reach the baseline arms below by
        /// their structure, not by order.
        when feature(MexQuantifier) exp_mekso_quantifier,
        /// Uses the `mekso_quantifier` product form, whose payload preserves `vei`, `mekso`, and `veho`.
        mekso_quantifier,
        /// Uses the `pa_run_quantifier` product form, whose payload preserves `number` and `boi`.
        pa_run_quantifier,
    }

    /// camxes-exp's `quantifier <- !selbri !sumti_6 mex` (camxes-exp.peg:273), with jbotci's own
    /// mex limited to camxes-exp's mex language: a forethought call without PEhO, which
    /// jbotci's mex keeps (the retained standard design, I12) but camxes-exp's does not
    /// (camxes-exp.peg:282), is refused anywhere in the mex's own structure. The two guards are
    /// camxes-exp's, read by jbotci's own parsers: a selbri such as `re moi broda` and a
    /// `sumti_6` such as the letter string in `by su'i cy` stay what they are. The construct is
    /// diagnosed post-parse as `experimental-mex-quantifier`.
    rule "quantifier" exp_mekso_quantifier(sumti, description_leading_operand, term, subbridi, exp_subsentence, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement, normal_term, quantifier) -> struct {
        assert !selbri;
        assert !exp_sumti_6_guard(sumti, description_leading_operand, term, subbridi, exp_subsentence, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement, normal_term, quantifier);
        // The mex must complete and must not be exactly one baseline quantifier surface: a
        // single number operand or a single VEI operand (#843). `number_mekso` wraps the same
        // `pa_run_quantifier` rule as the baseline arm, and `parenthesized_mekso_operand` is the
        // same `VEI mex [VEhO]` surface as `mekso_quantifier`, so both read the same extent and
        // the refusal cannot change the accepted language. The mex must also not hold a
        // forethought call without PEhO, which camxes-exp's mex cannot read (#982). The test is
        // a strict lookahead, so an abandoned attempt reports nothing from inside the mex (#988)
        // and recovery never enters the arm to invent a mex.
        assert mekso
            .reject_output(crate::grammar::baseline_quantifier::BaselineQuantifierRejection)
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .lookahead();
        #[tree_child(primary)]
        /// The quantity.
        field mekso <- arc(mekso);
    }

    // camxes-exp's `sumti_6` (camxes-exp.peg:189) for the raw-mex quantifier's `!sumti_6` guard:
    // the arms of jbotci's `sumti_base` that camxes reads at the `sumti_6` tier, in their order.
    // The two quantifier-bearing arms are the `sumti_5` tier, so they are left out. Reading the
    // guard through `sumti_base` itself would not work: its quantifier-bearing arms would take
    // the extent first and hide a `sumti_6` reading such as the letter string `by`. A `splice`
    // cannot share the list either: the two left-out arms sit between the others in
    // `sumti_base`, and moving them to the front changes recovered readings (#990). The
    // `exp_sumti_6_guard_calls_the_sumti_6_arms_of_sumti_base` test in `grammar/mod.rs` keeps
    // this list in step with `sumti_base`.
    alias "sumti" exp_sumti_6_guard(sumti, description_leading_operand, term, subbridi, exp_subsentence, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement, normal_term, quantifier) = choice((
        scalar_negated_sumti_with_bo(sumti, subbridi, tense_modal, normal_term).ignored(),
        scalar_negated_sumti(sumti).ignored(),
        lahe_sumti(sumti, subbridi, tense_modal, normal_term).ignored(),
        lahe_term_wrapper(term).ignored(),
        scalar_negated_term_wrapper_with_bo(term).ignored(),
        scalar_negated_term_wrapper(term).ignored(),
        bridi_description_sumti(exp_subsentence).ignored(),
        name_sumti(sumti, subbridi, tense_modal, normal_term).ignored(),
        descriptor_with_gadri_sumti(sumti, description_leading_operand, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement, free_modifier, normal_term, quantifier).ignored(),
        exp_descriptor_with_leading_sumti_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier).ignored(),
        number_sumti(mekso).ignored(),
        lerfu_string_sumti(letter_string, free_modifier).ignored(),
        quoted_sumti(text).ignored(),
        pro_sumti().ignored(),
    ));

    /// Transparent product node for number mex; preserves the `quantifier` component.
    rule "number mex" number_mekso(letter_tokens, free_modifier) -> struct {
        /// The shared quantifier child syntax node.
        field quantifier <- arc(pa_run_quantifier(letter_tokens, free_modifier));
    }

    /// Transparent product node for VUhU operator; preserves the `vuhu` component.
    rule "VUhU operator" primitive_mekso_operator -> struct {
        /// A word from selmaho `Vuhu`.
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Product node for operator; preserves the operator_1-width head and heterogeneous continuations in source order.
    rule "operator" mekso_operator(mekso_operator, inner_mekso_operator, tense_modal) -> struct {
        /// The operator_1-width operator at the start of the chain.
        field leading_operator <- arc(inner_mekso_operator);
        /// Freely interleaved afterthought and KE-grouped continuations.
        field continuations <- [zero_or_more mekso_operator_continuation(mekso_operator, inner_mekso_operator, tense_modal)];
    }

    /// Sum node for an operator continuation; distinguishes afterthought and KE-grouped forms.
    rule "operator continuation" mekso_operator_continuation(mekso_operator, inner_mekso_operator, tense_modal) -> enum {
        /// A joik/jek continuation followed by an operator_1-width operator.
        afterthought_mekso_operator_continuation,
        /// A joik-only continuation containing a full KE-grouped operator.
        grouped_mekso_operator_continuation,
    }

    /// Product node for operator continuation; preserves `connective` and `trailing_operator` in source order.
    rule "operator continuation" afterthought_mekso_operator_continuation(inner_mekso_operator) -> struct {
        /// The `standard_statement_connective` connective joining the adjacent constituents of the `afterthought_mekso_operator_continuation` production.
        field connective <- standard_statement_connective;
        /// The operator_1-width trailing operator.
        field trailing_operator <- arc(inner_mekso_operator);
    }

    /// Product node for a joik-only KE-grouped continuation.
    rule "grouped operator continuation" grouped_mekso_operator_continuation(mekso_operator, tense_modal) -> struct {
        /// The joik connective introducing the group.
        field connective <- arc(joik_connective);
        /// The optional tense modal between the connective and KE.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The full-width grouped operator.
        field inner_operator <- arc(mekso_operator);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for operator_1; selects forethought, experimental BO-bound, or operator_2 forms.
    rule "inner operator" inner_mekso_operator(mekso, mekso_operator, inner_mekso_operator, atomic_mekso_operator, sumti, selbri, tense_modal) -> enum {
        /// Uses the forethought operator form.
        forethought_mekso_operator,
        /// Uses the camxes-exp BO-bound operator form.
        bound_mekso_operator,
        /// Uses the nested operator_2 sum form.
        simple_mekso_operator,
    }

    /// Product node for operator; preserves `left_operator`, `connective`, `bo`, and `right_operator` in source order.
    rule "operator" bound_mekso_operator(mekso, mekso_operator, inner_mekso_operator, atomic_mekso_operator, sumti, selbri, tense_modal) -> struct {
        /// The operator_2-width left operator.
        field left_operator <- arc(simple_mekso_operator(atomic_mekso_operator, mekso_operator));
        /// The `standard_statement_connective` connective joining the adjacent constituents of the `bound_mekso_operator` production.
        field connective <- standard_statement_connective;
        /// The optional tense modal between the connective and BO.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).warn(ExperimentalMexOperatorConnective).wf();
        /// The operator_1-width right operator.
        field right_operator <- arc(inner_mekso_operator);
    }

    /// Sum node for operator_2; selects an atomic operator or a KE-grouped full operator.
    rule "simple operator" simple_mekso_operator(atomic_mekso_operator, mekso_operator) -> enum {
        /// Uses the nested atomic operator sum form.
        atomic_mekso_operator,
        /// Uses the `grouped_mekso_operator` product form.
        grouped_mekso_operator,
    }

    /// Sum node for an atomic operator.
    rule "atomic operator" atomic_mekso_operator(atomic_mekso_operator, mekso, sumti, selbri) -> enum {
        /// Uses the `converted_mekso_operator` product form, whose payload preserves `se` and `inner_operator`.
        converted_mekso_operator,
        /// Uses the `scalar_negated_mekso_operator` product form, whose payload preserves `nahe` and `inner_operator`.
        scalar_negated_mekso_operator,
        /// Uses the `selbri_mekso_operator` product form, whose payload preserves `nahu`, `selbri`, and `tehu`.
        selbri_mekso_operator,
        /// Uses the `operand_mekso_operator` product form, whose payload preserves `maho`, `mekso`, and `tehu`.
        operand_mekso_operator,
        /// Uses a camxes-exp connective as an atomic operator.
        experimental_connective_mekso_operator,
        /// Uses the `primitive_mekso_operator` product form, whose payload preserves `vuhu`.
        primitive_mekso_operator,
    }

    /// Product node for converted operator; preserves `se` and `inner_operator` in source order.
    rule "converted operator" converted_mekso_operator(atomic_mekso_operator) -> struct {
        /// A word from selmaho `Se`.
        field se <- selmaho(Se).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(atomic_mekso_operator);
    }

    /// Product node for converted operator; preserves `nahe` and `inner_operator` in source order.
    rule "converted operator" scalar_negated_mekso_operator(atomic_mekso_operator) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(atomic_mekso_operator);
    }

    /// Product node for operator; preserves `guhek`, `left_operator`, `gik`, and `right_operator` in source order.
    rule "operator" forethought_mekso_operator(inner_mekso_operator, atomic_mekso_operator, mekso_operator) -> struct {
        /// The operator-context forethought connective.
        field guhek <- operator_guhek_connective;
        /// The operator_1-width left operator.
        field left_operator <- arc(inner_mekso_operator);
        /// The GI-family `gik_connective` connective separating the forethought branches of the `forethought_mekso_operator` production.
        field gik <- gik_connective;
        /// The operator_2-width right operator.
        field right_operator <- arc(simple_mekso_operator(atomic_mekso_operator, mekso_operator));
    }

    /// Product node for an operator-context GUhEK, which permits SE but not NAhE.
    rule "forethought operator connective" operator_guhek_connective -> struct {
        /// The optional SE conversion.
        field se <- opt(selmaho(Se));
        /// A word from selmaho `Guha`.
        field guha <- selmaho(Guha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for grouped operator; preserves `ke`, `inner_operator`, and `kehe` in source order.
    rule "grouped operator" grouped_mekso_operator(mekso_operator) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(mekso_operator);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for selbri-to-operator; preserves `nahu`, `selbri`, and `tehu` in source order.
    rule "selbri-to-operator" selbri_mekso_operator(selbri) -> struct {
        /// The `Nahu` cmavo marker.
        field nahu <- cmavo(Nahu).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for operand-to-operator; preserves `maho`, `mekso`, and `tehu` in source order.
    rule "operand-to-operator" operand_mekso_operator(mekso) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).wf();
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Sum node for a camxes-exp connective operator.
    rule "experimental connective operator" experimental_connective_mekso_operator -> enum {
        /// A joik or jek connective.
        standard_statement_connective,
        /// An ek connective.
        ek_connective,
    }

    /// Product node for operand; preserves `connected_expression` and `grouped_continuation` in source order.
    rule "operand" mekso_operand(mekso, mekso_operand, bound_or_simple_mekso_operand, simple_mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        /// The operand_1-width connected expression at the start of the operand.
        field connected_expression <- arc(afterthought_mekso_operand(bound_or_simple_mekso_operand));
        /// The optional joik/EK plus KE-grouped continuation at operand_0 width.
        field grouped_continuation <- opt(grouped_mekso_operand_continuation(mekso_operand, tense_modal));
    }

    /// Product node for grouped operand continuation; preserves `operand_connective`, `tense_modal`, `ke`, `inner_expression`, and `kehe` in source order.
    rule "grouped operand continuation" grouped_mekso_operand_continuation(mekso_operand, tense_modal) -> struct {
        /// The joik/EK connective introducing the grouped continuation.
        field operand_connective <- operand_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The full-width inner operand.
        field inner_expression <- arc(mekso_operand);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Transparent product node for operand connective; preserves the `operands` component.
    rule "operand connective" afterthought_mekso_operand(bound_or_simple_mekso_operand) -> struct {
        /// The source-ordered `operands` chain assembled by the `afterthought_mekso_operand` production.
        field operands <- chain(
            first: arc(bound_or_simple_mekso_operand),
            zero_or_more: afterthought_mekso_operand_continuation(bound_or_simple_mekso_operand),
            element: trailing_expression,
        );
    }

    /// Product node for operand continuation; preserves `operand_connective` and `trailing_expression` in source order.
    rule "operand continuation" afterthought_mekso_operand_continuation(bound_or_simple_mekso_operand) -> struct {
        /// The `operand_connective` connective joining the adjacent constituents of the `afterthought_mekso_operand_continuation` production.
        field operand_connective <- operand_connective;
        /// The shared trailing expression child syntax node.
        field trailing_expression <- arc(bound_or_simple_mekso_operand);
    }

    /// Sum node for operand; selects among the `bound_mekso_operand` and `simple_mekso_operand` forms.
    rule "operand" bound_or_simple_mekso_operand(bound_or_simple_mekso_operand, simple_mekso_operand, tense_modal) -> enum {
        /// Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.
        bound_mekso_operand,
        /// Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.
        simple_mekso_operand,
    }

    /// Product node for operand connective; preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression` in source order.
    rule "operand connective" bound_mekso_operand(bound_or_simple_mekso_operand, simple_mekso_operand, tense_modal) -> struct {
        /// The shared left expression child syntax node.
        field left_expression <- arc(simple_mekso_operand);
        /// The `operand_connective` connective joining the adjacent constituents of the `bound_mekso_operand` production.
        field operand_connective <- operand_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The operand_2-width right expression child syntax node.
        field right_expression <- arc(bound_or_simple_mekso_operand);
    }

    /// Sum node for operand; selects among 12 forms including `forethought_mekso_operand`, `qualified_mekso_operand`, `scalar_negated_mekso_operand`, `lahe_qualified_mekso_operand`, and `parenthesized_mekso_operand`.
    rule "operand" simple_mekso_operand(mekso, mekso_base, mekso_operand, simple_mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator, exp_mex) -> enum {
        /// Uses the `forethought_mekso_operand` product form, whose payload preserves `gek`, `left_expression`, `gik`, and `right_expression`.
        forethought_mekso_operand,
        /// Uses the `exp_nahe_bo_mex_operand` product form, whose payload preserves `nahe`, `bo`, `inner_expression`, and `luhu`.
        when feature(LaheMex) exp_nahe_bo_mex_operand,
        /// NAhE BO qualifies one operand on the baseline path.
        qualified_mekso_operand,
        /// Uses the `exp_nahe_mex_operand` product form, whose payload preserves `nahe`, `inner_expression`, and `luhu`.
        when feature(LaheMex) exp_nahe_mex_operand,
        /// NAhE qualifies one operand on the baseline path.
        scalar_negated_mekso_operand,
        /// Uses the `exp_lahe_mex_operand` product form, whose payload preserves `lahe`, `inner_expression`, and `luhu`.
        when feature(LaheMex) exp_lahe_mex_operand,
        /// LAhE qualifies one operand on the baseline path.
        lahe_qualified_mekso_operand,
        /// Uses the `parenthesized_mekso_operand` product form, whose payload preserves `vei`, `inner_expression`, and `veho`.
        parenthesized_mekso_operand,
        /// Uses the `sumti_mekso_operand` product form, whose payload preserves `mohe`, `sumti`, and `tehu`.
        sumti_mekso_operand,
        /// Uses the `selbri_mekso_operand` product form, whose payload preserves `nihe`, `selbri`, and `tehu`.
        selbri_mekso_operand,
        /// Uses the `array_mekso_operand` product form, whose payload preserves `johi`, `expressions`, and `tehu`.
        array_mekso_operand,
        /// Uses the `number_mekso` product form, whose payload preserves `quantifier`.
        number_mekso,
        /// Uses the `lerfu_string_mekso` product form, whose payload preserves `letters`, `boi`, and `free_modifiers`.
        lerfu_string_mekso,
    }

    /// NAhE BO qualifies the whole mex under `lahe-mex` (camxes-exp.peg:282).
    rule "qualified mex" exp_nahe_bo_mex_operand(exp_mex) -> struct {
        /// The scalar qualifier and its free modifiers.
        field nahe <- selmaho(Nahe).warn(ExperimentalWholeMexQualifier).wf();
        /// The BO marker and its free modifiers.
        field bo <- cmavo(Bo).wf();
        /// The whole mex. A forethought call needs PEhO.
        field inner_expression <- arc(exp_mex
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection));
        /// The optional LUhU terminator and its free modifiers.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// NAhE qualifies the whole mex under `lahe-mex` (camxes-exp.peg:282).
    rule "scalar-negated mex" exp_nahe_mex_operand(exp_mex) -> struct {
        /// The scalar qualifier and its free modifiers.
        field nahe <- selmaho(Nahe).warn(ExperimentalNaheArgumentWithoutBo).warn(ExperimentalWholeMexQualifier).wf();
        /// The whole mex. A forethought call needs PEhO.
        field inner_expression <- arc(exp_mex
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection));
        /// The optional LUhU terminator and its free modifiers.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// LAhE qualifies the whole mex under `lahe-mex` (camxes-exp.peg:282).
    rule "LAhE-qualified mex" exp_lahe_mex_operand(exp_mex) -> struct {
        /// The sumti qualifier and its free modifiers.
        field lahe <- selmaho(Lahe).warn(ExperimentalWholeMexQualifier).wf();
        /// The whole mex. A forethought call needs PEhO.
        field inner_expression <- arc(exp_mex
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection));
        /// The optional LUhU terminator and its free modifiers.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for qualified operand; preserves `nahe`, `bo`, `inner_expression`, and `luhu` in source order.
    rule "qualified operand" qualified_mekso_operand(mekso_operand) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo);
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso_operand);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated operand; preserves `nahe`, `inner_expression`, and `luhu` in source order.
    rule "scalar-negated operand" scalar_negated_mekso_operand(mekso_operand) -> struct {
        /// A word from selmaho `Nahe`.
        ///
        /// camxes-exp permits the qualifier without the standard grammar's `bo`.
        /// The BO-ful sibling remains earlier in the operand choice, preserving
        /// baseline ownership for surfaces accepted by camxes-standard.
        field nahe <- selmaho(Nahe).warn(ExperimentalNaheArgumentWithoutBo).wf();
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso_operand);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for LAhE-qualified operand; preserves `lahe`, `inner_expression`, and `luhu` in source order.
    rule "LAhE-qualified operand" lahe_qualified_mekso_operand(mekso_operand) -> struct {
        /// A word from selmaho `Lahe`.
        field lahe <- selmaho(Lahe).wf();
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso_operand);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for forethought mex; preserves `gek`, `left_expression`, `gik`, and `right_expression` in source order.
    rule "forethought mex" forethought_mekso_operand(mekso_operand, simple_mekso_operand, tense_modal, selbri, letter_tokens) -> struct {
        /// The `modal_forethought_connective` forethought connective opening the paired branches of the `forethought_mekso_operand` production.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The shared left expression child syntax node.
        field left_expression <- arc(mekso_operand);
        /// The GI-family `gik_connective` connective separating the forethought branches of the `forethought_mekso_operand` production.
        field gik <- gik_connective;
        /// The operand_3-width right expression child syntax node.
        field right_expression <- arc(simple_mekso_operand);
    }

    /// Product node for sumti operand; preserves `mohe`, `sumti`, and `tehu` in source order.
    rule "sumti operand" sumti_mekso_operand(sumti) -> struct {
        /// The `Mohe` cmavo marker.
        field mohe <- cmavo(Mohe).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for selbri operand; preserves `nihe`, `selbri`, and `tehu` in source order.
    rule "selbri operand" selbri_mekso_operand(selbri) -> struct {
        /// The `Nihe` cmavo marker.
        field nihe <- cmavo(Nihe).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for parenthesized mex; preserves `vei`, `inner_expression`, and `veho` in source order.
    rule "parenthesized mex" parenthesized_mekso_operand(mekso) -> struct {
        /// The `Vei` cmavo marker.
        field vei <- cmavo(Vei).wf();
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso);
        /// The optional `Veho` cmavo marker.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// Product node for mekso array; preserves `johi`, `expressions`, and `tehu` in source order.
    rule "mekso array" array_mekso_operand(mekso_base, mekso_operand, mekso_operator) -> struct {
        /// The `Johi` cmavo marker.
        field johi <- cmavo(Johi).wf();
        /// Non-empty ordered sequence of expressions components.
        field expressions <- [one_or_more standard_mekso_array_element(mekso_base, mekso_operand, mekso_operator)];
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Sum node for one standard-width JOhI array element.
    rule "mekso array element" standard_mekso_array_element(mekso_base, mekso_operand, mekso_operator) -> enum {
        /// A standard operand element.
        mekso_operand,
        /// An operator-led forethought element.
        forethought_call_mekso,
    }

    /// Product node for lerfu string; preserves `first_letter` and `continuations` in source order.
    rule "lerfu string" letter_string(letter_tokens) -> struct {
        /// The shared first letter child syntax node.
        field first_letter <- arc(letter_tokens);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more letter_string_continuation(letter_tokens)];
    }

    /// Sum node for lerfu string continuation; selects among the `letter_string_pa_continuation` and `letter_string_lerfu_continuation` forms.
    rule "lerfu string continuation" letter_string_continuation(letter_tokens) -> enum {
        /// Uses the `letter_string_pa_continuation` product form, whose payload preserves `pa`.
        when feature(MixedNumberLerfu) letter_string_pa_continuation,
        /// Uses the `letter_string_lerfu_continuation` product form, whose payload preserves `letter`.
        letter_string_lerfu_continuation,
    }

    /// Transparent product node for lerfu string continuation; preserves the `pa` component.
    rule "lerfu string continuation" letter_string_pa_continuation -> struct {
        /// The `pa_word` grammar result in the `pa` structural role of the `letter_string_pa_continuation` production.
        field pa <- pa_word();
    }

    /// Transparent product node for lerfu string continuation; preserves the `letter` component.
    rule "lerfu string continuation" letter_string_lerfu_continuation(letter_tokens) -> struct {
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Product node for number; preserves `first_number` and `continuations` in source order.
    rule "number" number_words(letter_tokens) -> struct {
        /// The initial `pa_word` constituent before the continuations of the `number_words` production.
        field first_number <- pa_word();
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more number_word_continuation(letter_tokens)];
    }

    /// Sum node for number continuation; selects among the `number_word_pa_continuation` and `number_word_lerfu_continuation` forms.
    rule "number continuation" number_word_continuation(letter_tokens) -> enum {
        /// Uses the `number_word_pa_continuation` product form, whose payload preserves `pa`.
        number_word_pa_continuation,
        /// Uses the `number_word_lerfu_continuation` product form, whose payload preserves `letter`.
        when feature(MixedNumberLerfu) number_word_lerfu_continuation,
    }

    /// Transparent product node for number continuation; preserves the `pa` component.
    rule "number continuation" number_word_pa_continuation -> struct {
        /// The `pa_word` grammar result in the `pa` structural role of the `number_word_pa_continuation` production.
        field pa <- pa_word();
    }

    /// Transparent product node for number continuation; preserves the `letter` component.
    rule "number continuation" number_word_lerfu_continuation(letter_tokens) -> struct {
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Sum node for number or lerfu string; selects among the `number_words` and `letter_string` forms.
    rule "number or lerfu string" number_or_letter_words(letter_tokens, letter_string) -> enum {
        /// Uses the `number_words` product form, whose payload preserves `first_number` and `continuations`.
        number_words,
        /// Uses the `letter_string` product form, whose payload preserves `first_letter` and `continuations`.
        letter_string,
    }

    /// Sum node for lerfu word; selects among the `simple_lerfu_word`, `lau_lerfu_word`, and `tei_lerfu_word` forms.
    rule "lerfu word" letter_tokens(letter_string, letter_tokens) -> enum {
        /// Uses the `simple_lerfu_word` product form, whose payload preserves `word`.
        simple_lerfu_word,
        /// Uses the `lau_lerfu_word` product form, whose payload preserves `lau` and `letter`.
        lau_lerfu_word,
        /// Uses the `tei_lerfu_word` product form, whose payload preserves `tei`, `letters`, and `foi`.
        tei_lerfu_word,
    }

    /// Transparent product node for lerfu word; preserves the `word` component.
    rule "lerfu word" simple_lerfu_word -> struct {
        /// The `word_category` grammar result in the `word` structural role of the `simple_lerfu_word` production.
        field word <- word_category(LetterWord);
    }

    /// Product node for lerfu word; preserves `lau` and `letter` in source order.
    rule "lerfu word" lau_lerfu_word(letter_tokens) -> struct {
        /// A word from selmaho `Lau`.
        field lau <- selmaho(Lau);
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Product node for lerfu word; preserves `tei`, `letters`, and `foi` in source order.
    rule "lerfu word" tei_lerfu_word(letter_string) -> struct {
        /// The `Tei` cmavo marker.
        field tei <- cmavo(Tei);
        /// The shared letters child syntax node.
        field letters <- arc(letter_string);
        /// The `Foi` cmavo marker.
        field foi <- cmavo(Foi);
    }

    /// Product node for lerfu string; preserves `letters`, `boi`, and `free_modifiers` in source order.
    rule "lerfu string" lerfu_string_mekso(letter_string, free_modifier) -> struct {
        /// The `letter_string` grammar result in the `letters` structural role of the `lerfu_string_mekso` production.
        field letters <- letter_string;
        assert !selmaho(Moi);
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Sum node for a standard mex base.
    rule "mex" mekso_base(mekso, mekso_base, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator) -> enum {
        /// Uses the nested `mekso_operand` sum form and preserves its selected alternative.
        mekso_operand,
        /// Uses the `forethought_call_mekso` product form, whose payload preserves `peho`, `operator`, `operands`, and `kuhe`.
        forethought_call_mekso,
    }

    /// Product node for mex; preserves `left_expression` and `tail` in source order.
    rule "mex" mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> struct {
        /// The shared left expression child syntax node.
        field left_expression <- arc(mekso_base);
        /// The optional tail component.
        field tail <- opt(mekso_precedence_tail(mekso_precedence, mekso_operator));
    }

    /// Product node for mex precedence tail; preserves `bihe`, `operator`, and `right_expression` in source order.
    rule "mex precedence tail" mekso_precedence_tail(mekso_precedence, mekso_operator) -> struct {
        /// The `Bihe` cmavo marker.
        field bihe <- cmavo(Bihe).wf();
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_precedence);
    }

    /// Product node for mex; preserves `first_expression` and `continuations` in source order.
    rule "mex" infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        /// The shared first expression child syntax node.
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    /// Product node for mex continuation; preserves `operator` and `right_expression` in source order.
    rule "mex continuation" infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_precedence);
    }

    /// Product node for forethought mex; preserves `peho`, `operator`, `operands`, and `kuhe` in source order.
    rule "forethought mex" forethought_call_mekso(mekso_base, mekso_operator) -> struct {
        /// The optional `Peho` cmavo marker.
        field peho <- opt(cmavo(Peho).wf());
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// Non-empty ordered sequence of operands components.
        field operands <- [one_or_more mekso_base];
        /// The optional `Kuhe` cmavo marker.
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Sum node for mex; selects among the `infix_mekso` and `reverse_polish_mekso` forms.
    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts, tense_modal) -> enum {
        /// Uses the `infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.
        infix_mekso,
        /// Uses the `reverse_polish_mekso` product form, whose payload preserves `fuha` and `parts`.
        reverse_polish_mekso,
    }

    /// Product node for reverse Polish mex; preserves `first_operand` and `tails` in source order.
    rule "reverse Polish mex" reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> struct {
        /// The shared first operand child syntax node.
        field first_operand <- arc(mekso_operand);
        /// Ordered sequence of zero or more tails components.
        field tails <- [zero_or_more reverse_polish_parts_tail(reverse_polish_parts, mekso_operator)];
    }

    /// Product node for reverse Polish mex tail; preserves `right_parts` and `operator` in source order.
    rule "reverse Polish mex tail" reverse_polish_parts_tail(reverse_polish_parts, mekso_operator) -> struct {
        /// The shared right parts child syntax node.
        field right_parts <- arc(reverse_polish_parts);
        /// The `mekso_operator` grammar result in the `operator` structural role of the `reverse_polish_parts_tail` production.
        field operator <- mekso_operator;
    }

    /// Product node for reverse Polish mex; preserves `fuha` and `parts` in source order.
    rule "reverse Polish mex" reverse_polish_mekso(reverse_polish_parts) -> struct {
        /// The `Fuha` cmavo marker.
        field fuha <- cmavo(Fuha).wf();
        /// The shared parts child syntax node.
        field parts <- arc(reverse_polish_parts);
    }

    /// Product node for number sumti; preserves `li`, `expression`, and `loho` in source order.
    rule "number sumti" number_sumti(mekso) -> struct {
        /// A word from selmaho `Li`.
        field li <- selmaho(Li).wf();
        #[tree_child(primary)]
        /// The shared expression child syntax node.
        field expression <- arc(mekso);
        /// The optional `Loho` cmavo marker.
        field loho <- opt(cmavo(Loho).wf()).elidable_terminator(Loho);
    }

    /// Product node for lerfu string; preserves `words`, `boi`, and `free_modifiers` in source order.
    rule "lerfu string" lerfu_string_sumti(letter_string, free_modifier) -> struct {
        /// The `letter_string` grammar result in the `words` structural role of the `lerfu_string_sumti` production.
        field words <- letter_string;
        assert !selmaho(Moi);
        assert !selmaho(Mai);
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Product node for converted sumti; preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu` in source order.
    rule "converted sumti" lahe_sumti(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// A word from selmaho `Lahe`.
        field lahe <- selmaho(Lahe).wf();
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for converted term; preserves `lahe`, `inner_term`, and `luhu` in source order.
    rule "converted term" lahe_term_wrapper(term) -> struct {
        /// A word from selmaho `Lahe`.
        ///
        /// Wrapping a bare term (rather than a sumti) in `LAhE` is a non-CLL extension:
        /// standard grammar only allows `LAhE` over a sumti, so the term-wrapper form warns.
        field lahe <- selmaho(Lahe).warn(ExperimentalLaheNaheTermWrapper).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated term; preserves `nahe`, `bo`, `inner_term`, and `luhu` in source order.
    rule "scalar-negated term" scalar_negated_term_wrapper_with_bo(term) -> struct {
        /// A word from selmaho `Nahe`.
        ///
        /// `NAhE BO` wrapping a bare term (rather than a sumti) is a non-CLL extension:
        /// even with `bo`, the standard grammar only allows `NAhE BO` over a sumti, so the
        /// term-wrapper form warns. The warning anchors on `na'e` to match the v0 behavior.
        field nahe <- selmaho(Nahe).warn(ExperimentalLaheNaheTermWrapper);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated term; preserves `nahe`, `inner_term`, and `luhu` in source order.
    rule "scalar-negated term" scalar_negated_term_wrapper(term) -> struct {
        /// A word from selmaho `Nahe`.
        ///
        /// Bare `na'e` wrapping a term (rather than a sumti) without `bo` is a non-CLL
        /// extension. Following v0, this carries only the term-wrapper warning
        /// (`ExperimentalLaheNaheTermWrapper`), not the sumti-oriented without-`bo`
        /// warning: the distinguishing property here is the term payload, not the missing `bo`.
        field nahe <- selmaho(Nahe).warn(ExperimentalLaheNaheTermWrapper).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated sumti; preserves `nahe`, `bo`, optional `relative_clauses`, `inner_sumti`, and `luhu` in source order.
    rule "scalar-negated sumti" scalar_negated_sumti_with_bo(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// Optional relative clauses attached in the standard post-BO slot before the inner sumti.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated sumti; preserves `nahe`, `inner_sumti`, and `luhu` in source order.
    rule "scalar-negated sumti" scalar_negated_sumti(sumti) -> struct {
        /// A word from selmaho `Nahe`.
        ///
        /// Bare `na'e` before a sumti without `bo` is a non-CLL extension (standard
        /// `sumti-6` permits only `NAhE BO` before a sumti), so it warns; the `bo`-ful
        /// sibling `scalar_negated_sumti_with_bo` is standard grammar and does not warn.
        field nahe <- selmaho(Nahe).warn(ExperimentalNaheArgumentWithoutBo).wf();
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Transparent product node for sumti; preserves the `koha` component.
    rule "sumti" pro_sumti -> struct {
        /// The `word_category` grammar result in the `koha` structural role of the `pro_sumti` production.
        field koha <- word_category(ProSumti).wf();
    }

    /// Product node for name; preserves `la`, `relative_clauses`, and `names` in source order.
    rule "name" name_sumti(sumti, subbridi, tense_modal, normal_term) -> struct {
        assert feature(Cbm).not();
        /// A word from selmaho `La`.
        field la <- selmaho(La).wf();
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
    }

    /// Transparent product node for descriptor; preserves the `description` component.
    rule "descriptor" description_head -> struct {
        /// The required description-head word from either selmaho `Le` or selmaho `La`.
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    /// Product node for description; preserves `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_gadri_sumti(sumti, description_leading_operand, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The `description_head` grammar result in the `description` structural role of the `descriptor_with_gadri_sumti` production.
        field description <- description_head();
        /// The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_gadri_sumti` production.
        field tail <- description_tail(sumti, description_leading_operand, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `outer_quantifier`, `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_outer_quantifier_sumti(sumti, description_leading_operand, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The `quantifier` grammar result in the `outer_quantifier` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field outer_quantifier <- quantifier;
        /// The `description_head` grammar result in the `description` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field description <- description_head();
        /// The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field tail <- description_tail(sumti, description_leading_operand, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `quantifier`, `selbri`, `ku`, and `relative_clauses` in source order.
    rule "description" descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `descriptor_without_gadri_sumti` production.
        field quantifier <- quantifier;
        assert !selmaho(Roi);
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    // camxes-exp's `sumti_tail` arm 3, `sumti sumti_tail_1` (camxes-exp.peg:194): a FULL sumti,
    // at the connection level, as the leading element of a description tail.  It is a sibling
    // top-level descriptor variant rather than a widening of `description_tail`'s leading field,
    // because a connected sumti cannot fit `description_tail_sumti` and widening that field would
    // move baseline trees.
    //
    // The `!quantifier` guard is a real negative lookahead on the `quantifier` production, at the
    // position the leading sumti starts.  Why it is here, stated accurately:
    //
    // It is NOT what keeps this default-enabled route from re-opening #552.  The plan's premise
    // that it is (plan-v7 F5) was MEASURED FALSE in the epoch's round-1 fix round: a
    // guard-deleted binary was swept against the guarded one over all 26,678 fixture inputs at
    // their declared dialects, comparing errors, warnings, brackets and the recovered spine, and
    // there is no difference anywhere; every constructed quantifier-leading candidate stays
    // rejected with the guard removed.  The reason is ordered choice.  A leading sumti that opens
    // with a quantifier is exactly `quantifier sumti_6...`, which is the IDENTICAL extent D1's
    // restored `sumti_tail_1 <- quantifier sumti` arm consumes inside
    // `descriptor_with_gadri_sumti` -- and `sumti_base` tries that arm FIRST.  Choice commits on
    // its success and an outer failure never re-enters a committed inner choice, so this arm is
    // unreachable for such an extent whether or not the guard is written.
    //
    // It STAYS for defence in depth: the property that makes the guard redundant today is the
    // ARM ORDER in `sumti_base` plus D1's tail arms, and a later epoch could move either without
    // noticing that an ownership boundary rested on it.  Written here, the boundary holds by
    // construction instead.  camxes-exp spells no such guard, so this remains a
    // recorded fidelity narrowing; the one class it EXCLUDES rather than re-owns is exp's
    // `quantifier gek_sentence` leading element, which `sumti_tail_1` cannot form, and that
    // non-adoption is recorded, witnessed and filed as #886.  The measurement, the candidate
    // table and the reference rows are in `docs/grammar-parity-epoch-09-descriptions.md`.

    /// Product node for description tail; preserves `leading_sumti` and `tail` in source order.
    rule "description tail" exp_full_sumti_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        assert !quantifier;
        /// The full leading sumti this camxes-exp arm admits where the baseline admits a sumti_6.
        field leading_sumti <- arc(sumti);
        /// The shared tail child syntax node.
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier));
    }

    /// Product node for description; preserves `description`, `tail`, and `ku` in source order.
    rule "description" exp_descriptor_with_leading_sumti_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The shared description head child syntax node.
        field description <- arc(description_head());
        /// The camxes-exp full-sumti leading tail, refused wherever the baseline route owns the extent.
        field tail <- exp_full_sumti_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier)
            .reject_output(crate::grammar::description_leading::ExpDescriptionLeadingSumtiRejection);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description tail; preserves `leading_tail_elements` and `tail` in source order.
    rule "description tail" description_tail(sumti, description_leading_operand, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The `leading_description_tail_elements` grammar result in the `leading_tail_elements` structural role of the `description_tail` production.
        field leading_tail_elements <- leading_description_tail_elements(sumti, description_leading_operand, subbridi, selbri, tense_modal, statement, normal_term);
        /// The shared tail child syntax node.
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier));
    }

    /// Sum node for description tail; selects among the `quantifier_relation_description_tail`, `quantifier_sumti_description_tail`, and `relation_description_tail` forms.
    rule "description tail" description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> enum {
        /// Uses the `quantifier_relation_description_tail` product form, whose payload preserves `quantifier`, `selbri`, and `relative_clauses`.
        quantifier_relation_description_tail,
        /// Uses the `quantifier_sumti_description_tail` product form, whose payload preserves `quantifier` and `sumti`.
        quantifier_sumti_description_tail,
        /// Uses the `relation_description_tail` product form, whose payload preserves `selbri` and `relative_clauses`.
        relation_description_tail,
    }

    /// Product node for description tail; preserves `tail_sumti` and `relative_clauses` in source order.
    rule "description tail" leading_description_tail_elements(sumti, description_leading_operand, subbridi, selbri, tense_modal, statement, normal_term) -> struct {
        /// The optional tail sumti component.
        field tail_sumti <- opt(description_tail_sumti(description_leading_operand));
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Transparent product node for description tail; preserves the `sumti` component.
    ///
    /// The leading element of a description tail is camxes `sumti_6`
    /// (`sumti_tail <- (sumti_6 relative_clauses?)? sumti_tail_1`, camxes.peg:156), so it can
    /// carry no quantifier at all.  The retired `assert !pa_word()` blocked only the PA
    /// spelling and let `vei ... ve'o` through, which is #552; the structural restriction on
    /// `description_leading_operand` replaces it and covers every quantifier spelling.
    rule "description tail" description_tail_sumti(description_leading_operand) -> struct {
        /// The shared sumti child syntax node, restricted to the camxes `sumti_6` operand tier.
        field sumti <- arc(description_leading_operand);
    }

    /// Product node for description tail; preserves `selbri` and `relative_clauses` in source order.
    rule "description tail" relation_description_tail(sumti, subbridi, selbri, tense_modal, statement, normal_term) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for description tail; preserves `quantifier`, `selbri`, and `relative_clauses` in source order.
    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement, free_modifier, normal_term, quantifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_relation_description_tail` production.
        field quantifier <- quantifier;
        assert !selmaho(Roi);
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for description tail; preserves `quantifier` and `sumti` in source order.
    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens, free_modifier, quantifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_sumti_description_tail` production.
        field quantifier <- quantifier;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Sum node for quote; selects among three forms. MEhOI belongs to tanru atoms, not quoted sumti.
    rule "quote" quote(text) -> enum {
        /// Uses the `experimental_zohoi_compound_quote` product form, whose payload preserves `quote`.
        experimental_zohoi_compound_quote,
        /// Uses the `generic_compound_quote` product form, whose payload preserves `quote`.
        generic_compound_quote,
        /// Uses the `text_quote` product form, whose payload preserves `lu`, `text`, and `lihu`.
        text_quote,
    }

    /// Product node for text quote; preserves `lu`, `text`, and `lihu` in source order.
    rule "text quote" text_quote(text) -> struct {
        /// The `Lu` cmavo marker.
        field lu <- cmavo(Lu).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Lihu` cmavo marker.
        field lihu <- opt(cmavo(Lihu).wf()).elidable_terminator(Lihu);
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_zohoi_compound_quote -> struct {
        /// The selected grammar alternative in the `quote` structural role of the `experimental_zohoi_compound_quote` production.
        field quote <- choice((
            quote_marker(Zohoi),
            quote_marker(Lahoi),
            quote_marker(Rahoi),
        )).warn(ExperimentalZohOiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" generic_compound_quote -> struct {
        // The completed MEhOI token belongs to the dedicated predicate atom,
        // never to the generic quoted-sumti fallback (#820).
        assert !quote_marker(Mehoi);
        /// The `word_category` grammar result in the `quote` structural role of the `generic_compound_quote` production.
        field quote <- word_category(Quote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" quoted_sumti(text) -> struct {
        #[tree_child(primary)]
        /// The shared quote child syntax node.
        field quote <- arc(quote(text));
    }

    /// Product node for vocative phrase; preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses` in source order.
    rule "vocative phrase" selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal, statement, normal_term) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for vocative phrase; preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses` in source order.
    rule "vocative phrase" cmevla_vocative_sumti(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, normal_term));
    }

    /// Sum node for vocative phrase; selects among the `selbri_vocative_sumti`, `cmevla_vocative_sumti`, and `sumti` forms.
    rule "vocative phrase" vocative_sumti(sumti, subbridi, selbri, tense_modal, statement, normal_term) -> enum {
        /// Uses the `selbri_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses`.
        selbri_vocative_sumti,
        /// Uses the `cmevla_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses`.
        cmevla_vocative_sumti,
        /// Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.
        sumti,
    }

    /// Sum node for vocative marker; selects among the `coi_vocative_marker_words` and `doi_vocative_marker_words` forms.
    rule "vocative marker" vocative_marker_words -> enum {
        /// Uses the `coi_vocative_marker_words` product form, whose payload preserves `first_coi`, `first_nai`, `additional_coi`, and `doi`.
        coi_vocative_marker_words,
        /// Uses the `doi_vocative_marker_words` product form, whose payload preserves `doi`.
        doi_vocative_marker_words,
    }

    /// Product node for vocative marker; preserves `first_coi`, `first_nai`, `additional_coi`, and `doi` in source order.
    rule "vocative marker" coi_vocative_marker_words -> struct {
        /// A word from selmaho `Coi`.
        field first_coi <- selmaho(Coi);
        /// The optional `Nai` cmavo marker.
        field first_nai <- opt(cmavo(Nai));
        /// Ordered sequence of zero or more additional coi components.
        field additional_coi <- [zero_or_more additional_coi_vocative_marker()];
        /// The optional `Doi` cmavo marker.
        field doi <- opt(cmavo(Doi));
    }

    /// Product node for vocative marker; preserves `coi` and `nai` in source order.
    rule "vocative marker" additional_coi_vocative_marker -> struct {
        /// A word from selmaho `Coi`.
        field coi <- selmaho(Coi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Transparent product node for vocative marker; preserves the `doi` component.
    rule "vocative marker" doi_vocative_marker_words -> struct {
        /// The `Doi` cmavo marker.
        field doi <- cmavo(Doi);
    }

    /// Sum node for free modifier; selects among 7 forms including `text_replacement_free_modifier`, `sei_free_modifier`, and `xi_free_modifier`.
    rule "free modifier" free_modifier(sumti, subbridi, exp_subsentence, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier, statement, normal_term, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) -> enum {
        /// Uses the nested `text_replacement_free_modifier` sum form and preserves its selected alternative.
        text_replacement_free_modifier,
        /// Uses the `sei_free_modifier` product form, whose payload preserves `sei`, `terms`, `cu`, `selbri`, and `sehu`.
        sei_free_modifier,
        /// Uses the nested `xi_free_modifier` sum form and preserves its selected alternative.
        xi_free_modifier,
        /// Uses the `mai_free_modifier` product form, whose payload preserves `number` and `mai`.
        mai_free_modifier,
        /// camxes-exp's `mex_2 MAI_clause` (camxes-exp.peg:382), for the mex forms that the
        /// baseline ordinal does not take.
        exp_mex_2_mai_free_modifier,
        /// Uses the `soi_free_modifier` product form, whose payload preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu`.
        soi_free_modifier,
        /// Uses the `parenthetical_text` product form, whose payload preserves `to`, `text`, and `toi`.
        parenthetical_text,
        /// Uses the `vocative_free_modifier` product form, whose payload preserves `vocative_markers`, `sumti`, and `dohu`.
        vocative_free_modifier,
    }

    /// Product node for vocative phrase; preserves `vocative_markers`, `sumti`, and `dohu` in source order.
    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, tense_modal, statement, normal_term) -> struct {
        /// The `vocative_marker_words` grammar result in the `vocative_markers` structural role of the `vocative_free_modifier` production.
        field vocative_markers <- vocative_marker_words().wf_when(UnrestrictedFree);
        /// The optional sumti component.
        field sumti <- opt(arc(vocative_sumti(sumti, subbridi, selbri, tense_modal, statement, normal_term)));
        /// The optional `Dohu` cmavo marker.
        field dohu <- opt(cmavo(Dohu).prohibited_wf()).elidable_terminator(Dohu);
    }

    /// Product node for parenthetical text; preserves `to`, `text`, and `toi` in source order.
    rule "parenthetical text" parenthetical_text(text) -> struct {
        /// A word from selmaho `To`.
        field to <- selmaho(To).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Toi` cmavo marker.
        field toi <- opt(cmavo(Toi).prohibited_wf()).elidable_terminator(Toi);
    }

    /// Product node for metalinguistic comment; preserves `sei`, `terms`, `cu`, `selbri`, and `sehu` in source order.
    rule "metalinguistic comment" sei_free_modifier(term, selbri) -> struct {
        /// A word from selmaho `Sei`.
        field sei <- selmaho(Sei).wf();
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
        /// The optional `Cu` cmavo marker.
        field cu <- opt(cmavo(Cu).wf());
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    /// Sum node for subscript; selects among the number, lerfu-string, parenthesized and
    /// camxes-exp `mex_2` forms.
    rule "subscript" xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier, sumti, selbri, tense_modal, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) -> enum {
        /// Uses the `xi_number_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_number_free_modifier,
        /// Uses the `xi_lerfu_string_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_lerfu_string_free_modifier,
        /// Uses the `xi_parenthesized_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_parenthesized_free_modifier,
        /// camxes-exp's `XI_clause free* mex_2` (camxes-exp.peg:385), for the mex forms that
        /// the baseline subscript does not take.
        exp_mex_2_xi_free_modifier,
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_number_free_modifier(letter_tokens, free_modifier) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(number_mekso(letter_tokens, free_modifier));
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_lerfu_string_free_modifier(letter_string, free_modifier) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(lerfu_string_mekso(letter_string, free_modifier));
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_parenthesized_free_modifier(mekso) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(parenthesized_mekso_operand(mekso));
    }

    /// camxes-exp's subscript `XI_clause free* mex_2` (camxes-exp.peg:385). The baseline arms
    /// above own a number, a lerfu string and a VEI mex, so this arm adds the other `mex_2`
    /// forms. Like camxes-exp's `mex_2`, it refuses a forethought mex without PEhO anywhere in
    /// its own mex structure (camxes-exp.peg:282).
    rule "subscript" exp_mex_2_xi_free_modifier(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) -> struct {
        /// A word from selmaho `Xi`, which carries the warning for the whole construct.
        field xi <- selmaho(Xi).warn(ExperimentalMexSubscript).wf();
        /// The mex subscript.
        field expression <- arc(exp_mex_2(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts)
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection));
    }

    /// camxes-exp's `mex_2` (camxes-exp.peg:282) in jbotci's mex model, for the subscript and
    /// the utterance ordinal: a single operand that is not a number, a lerfu string or a VEI
    /// mex, which the baseline subscript and ordinal own, and not a JOhI array, which
    /// camxes-exp has only in `operand_3`. A forethought call needs PEhO, as in camxes-exp.
    rule "mex" exp_mex_2(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) -> enum {
        /// The whole left mex and the right mex_2.
        exp_forethought_mex_operand,
        /// NAhE BO over a whole mex requires the feature.
        when feature(LaheMex) exp_nahe_bo_mex_operand,
        /// The baseline qualifier over one operand.
        qualified_mekso_operand,
        /// NAhE over a whole mex requires the feature.
        when feature(LaheMex) exp_nahe_mex_operand,
        /// The baseline qualifier over one operand.
        scalar_negated_mekso_operand,
        /// LAhE over a whole mex requires the feature.
        when feature(LaheMex) exp_lahe_mex_operand,
        /// The baseline qualifier over one operand.
        lahe_qualified_mekso_operand,
        /// A sumti used as an operand.
        sumti_mekso_operand,
        /// A selbri used as an operand.
        selbri_mekso_operand,
        /// A required PEhO and whole mex arguments.
        exp_peho_forethought_call_mekso,
        /// FUhA and the reference reverse-Polish expression.
        exp_reverse_polish_mex,
    }

    /// The complete mex_2 tier for nested reference expressions (camxes-exp.peg:282).
    rule "mex" exp_complete_mex_2(exp_mex_2, exp_mex, letter_tokens, letter_string, free_modifier) -> enum {
        /// A number and its optional BOI.
        exp_number_mex,
        /// A letter string and its optional BOI.
        lerfu_string_mekso,
        /// A parenthesized reference mex.
        exp_parenthesized_mex,
        /// The remaining reference atoms.
        exp_mex_2,
    }

    /// A number at the reference mex_2 tier.
    rule "number operand" exp_number_mex(letter_tokens, free_modifier) -> struct {
        /// The number, its optional BOI and its free modifiers.
        field quantifier <- pa_run_quantifier(letter_tokens, free_modifier);
    }

    /// A parenthesized reference mex.
    rule "parenthesized operand" exp_parenthesized_mex(exp_mex) -> struct {
        /// The opening VEI and its free modifiers.
        field vei <- cmavo(Vei).wf();
        /// The whole reference mex.
        field inner_expression <- arc(exp_mex);
        /// The optional closing VEhO and its free modifiers.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// The reference mex tier (camxes-exp.peg:278).
    rule "mex" exp_mex(exp_mex_1, mekso_operator) -> struct {
        /// The first mex_1 expression.
        field first_expression <- arc(exp_mex_1);
        /// Infix operators and their right expressions.
        field continuations <- [zero_or_more exp_mex_continuation(exp_mex_1, mekso_operator)];
    }

    /// One infix continuation at the reference mex tier.
    rule "mex continuation" exp_mex_continuation(exp_mex_1, mekso_operator) -> struct {
        /// The infix operator.
        field operator <- arc(mekso_operator);
        /// The right mex_1 expression.
        field right_expression <- arc(exp_mex_1);
    }

    /// The reference mex_1 tier binds an operator with BO (camxes-exp.peg:280).
    rule "mex" exp_mex_1(exp_complete_mex_2, exp_mex_1, mekso_operator, tense_modal) -> struct {
        /// The left mex_2 expression.
        field left_expression <- arc(exp_complete_mex_2);
        /// The optional BO-bound continuation.
        field tail <- opt(exp_mex_bo_tail(exp_mex_1, mekso_operator, tense_modal));
    }

    /// A BO-bound operator and its right mex_1 expression.
    rule "mex precedence continuation" exp_mex_bo_tail(exp_mex_1, mekso_operator, tense_modal) -> struct {
        /// The operator before the optional tag.
        field operator <- arc(mekso_operator);
        /// The optional stag before BO.
        field tense_modal <- opt(arc(tense_modal));
        /// BO and its free modifiers.
        field bo <- cmavo(Bo).wf();
        /// The right mex_1 expression.
        field right_expression <- arc(exp_mex_1);
    }

    /// The reference GEK product reads mex and mex_2 (camxes-exp.peg:282).
    rule "forethought mex" exp_forethought_mex_operand(exp_mex, exp_complete_mex_2, tense_modal, selbri, letter_tokens) -> struct {
        /// The opening GEK connective.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The whole left mex.
        field left_expression <- arc(exp_mex);
        /// GI and its free modifiers.
        field gik <- gik_connective;
        /// The right mex_2 expression.
        field right_expression <- arc(exp_complete_mex_2);
    }

    /// The reference call requires PEhO and whole mex arguments (camxes-exp.peg:282).
    rule "forethought mex" exp_peho_forethought_call_mekso(exp_mex, mekso_operator) -> struct {
        /// PEhO and its free modifiers.
        field peho <- cmavo(Peho).wf();
        /// The call operator.
        field operator <- arc(mekso_operator);
        /// One or more whole mex arguments.
        field operands <- [one_or_more exp_mex];
        /// The optional KUhE and its free modifiers.
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// FUhA and the reference rp_expression (camxes-exp.peg:282-284).
    rule "reverse Polish mex" exp_reverse_polish_mex(exp_rp_parts) -> struct {
        /// FUhA, without a free-modifier slot in the reference rule.
        field fuha <- cmavo(Fuha);
        /// The reverse-Polish expression.
        field parts <- arc(exp_rp_parts);
    }

    /// The reference rp_expression starts with mex_1 (camxes-exp.peg:284).
    rule "reverse Polish mex" exp_rp_parts(exp_mex_1, exp_rp_parts, mekso_operator) -> struct {
        /// The first mex_1 expression.
        field first_operand <- arc(exp_mex_1);
        /// Nested reverse-Polish expressions and their operators.
        field tails <- [zero_or_more exp_rp_tail(exp_rp_parts, mekso_operator)];
    }

    /// One nested reference reverse-Polish expression and its operator.
    rule "reverse Polish mex tail" exp_rp_tail(exp_rp_parts, mekso_operator) -> struct {
        /// The nested reverse-Polish expression.
        field right_parts <- arc(exp_rp_parts);
        /// The postfix operator.
        field operator <- mekso_operator;
    }

    // Give a failed ordinal probe its grammar name in detailed diagnostics.
    alias "mex followed by MAI" exp_mex_2_mai_probe(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) = (
        exp_mex_2(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts)
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection),
        selmaho(Mai),
    ).ignored();

    /// camxes-exp's utterance ordinal `mex_2 MAI_clause free*` (camxes-exp.peg:382), for the
    /// `mex_2` forms that the baseline ordinal does not take.
    rule "utterance ordinal" exp_mex_2_mai_free_modifier(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts) -> struct {
        // A free modifier is tried after almost every word, and most of the words that start
        // a `mex_2` (NAhE, LAhE, GA, PEhO and others) start other constructs too. So the arm
        // first tests, as a probe, that a whole `mex_2` and then MAI follow. A failed attempt
        // then reports only at its own start (#988), and its inner expectations, which belong
        // to a construct that is not there, never become the furthest failure of the text.
        assert exp_mex_2_mai_probe(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts);
        /// The mex before MAI.
        field expression <- arc(exp_mex_2(mekso, sumti, selbri, tense_modal, letter_tokens, mekso_base, mekso_operand, simple_mekso_operand, mekso_operator, exp_mex, exp_complete_mex_2, exp_rp_parts)
            .reject_output(crate::grammar::peho_forethought::PehoLessForethoughtRejection)
            .reject_output(crate::grammar::peho_forethought::CamxesArrayRejection));
        /// A word from selmaho `Mai`, which carries the warning for the whole construct.
        field mai <- selmaho(Mai).warn(ExperimentalMexUtteranceOrdinal).wf();
    }

    /// Product node for utterance ordinal; preserves `number` and `mai` in source order.
    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        /// The `number_or_letter_words` grammar result in the `number` structural role of the `mai_free_modifier` production.
        field number <- number_or_letter_words(letter_tokens, letter_string)
            .followed_by(selmaho(Mai).ignored());
        /// A word from selmaho `Mai`.
        field mai <- selmaho(Mai).wf();
    }

    /// Product node for reciprocal; preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu` in source order.
    ///
    /// R1's other half. The reciprocal attaches inside `.wf()`, before any term-level arm is
    /// reached, so without this reservation it would take `soi` plus one sumti out of every
    /// camxes-exp adverbial and leave the rest of the subsentence -- and any explicit SEhU --
    /// behind. The reservation is the adverbial arm itself, classifier included: it succeeds
    /// only where that arm would own the extent, so `mi broda soi mi brode`, whose completed
    /// candidate reparses as the reciprocal plus a tail, stays the reciprocal's and silent,
    /// while `mi broda soi mi brode se'u` is the adverbial's.
    rule "reciprocal" soi_free_modifier(sumti, exp_subsentence) -> struct {
        assert !exp_soi_adverbial_term(exp_subsentence);
        /// The `Soi` cmavo marker.
        field soi <- cmavo(Soi).wf();
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti);
        /// The optional trailing sumti component.
        field trailing_sumti <- opt(arc(sumti));
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    /// Sum node for replacement phrase; selects among the `full_text_replacement_free_modifier`, `new_only_text_replacement_free_modifier`, and `close_only_text_replacement_free_modifier` forms.
    rule "replacement phrase" text_replacement_free_modifier -> enum {
        /// Uses the `full_text_replacement_free_modifier` product form, whose payload preserves `lohai`, `old_words`, `sahai`, `new_words`, and `lehai`.
        full_text_replacement_free_modifier,
        /// Uses the `new_only_text_replacement_free_modifier` product form, whose payload preserves `sahai`, `new_words`, and `lehai`.
        new_only_text_replacement_free_modifier,
        /// Uses the `close_only_text_replacement_free_modifier` product form, whose payload preserves `lehai`.
        close_only_text_replacement_free_modifier,
    }

    alias "replacement free modifier word" word_before_sahai_or_lehai =
        word_not_cmavo(Sahai, Lehai);

    alias "replacement free modifier word" word_before_lehai =
        word_not_cmavo(Lehai);

    /// Product node for replacement phrase; preserves `lohai`, `old_words`, `sahai`, `new_words`, and `lehai` in source order.
    rule "replacement phrase" full_text_replacement_free_modifier -> struct {
        /// The `Lohai` cmavo marker.
        field lohai <- cmavo(Lohai);
        /// Ordered sequence of zero or more old words components.
        field old_words <- [zero_or_more word_before_sahai_or_lehai()];
        /// The optional `Sahai` cmavo marker.
        field sahai <- opt(cmavo(Sahai));
        /// Ordered sequence of zero or more new words components.
        field new_words <- [zero_or_more word_before_lehai()];
        /// The `Lehai` cmavo marker.
        field lehai <- cmavo(Lehai).wf();
    }

    /// Product node for replacement phrase; preserves `sahai`, `new_words`, and `lehai` in source order.
    rule "replacement phrase" new_only_text_replacement_free_modifier -> struct {
        /// The `Sahai` cmavo marker.
        field sahai <- cmavo(Sahai);
        /// Ordered sequence of zero or more new words components.
        field new_words <- [zero_or_more word_before_lehai()];
        /// The `Lehai` cmavo marker.
        field lehai <- cmavo(Lehai).wf();
    }

    /// Transparent product node for replacement phrase; preserves the `lehai` component.
    rule "replacement phrase" close_only_text_replacement_free_modifier -> struct {
        /// The `Lehai` cmavo marker.
        field lehai <- cmavo(Lehai).wf();
    }

    /// Sum node for relative clauses; gives the completed camxes-exp continuation route first choice, then reparses baseline ZIhE surfaces through the standard arm.
    ///
    /// This is the connective machinery of the relative list, not an owner class of its own.
    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal, normal_term) -> enum {
        /// Uses the ownership-filtered camxes-exp continuation route.
        relative_clause_exp_continuation,
        /// Uses the `joined_relative_clause_tail` product form, whose payload preserves `zihe` and `inner`.
        joined_relative_clause_tail,
    }

    /// Transparent ownership wrapper for a camxes-exp relative-clause continuation.
    rule "relative clause" relative_clause_exp_continuation(sumti, subbridi, tense_modal, normal_term) -> struct {
        #[tree_child(primary)]
        /// The completed continuation, retained only when baseline ZIhE does not own its identical extent.
        field continuation <- arc(
            exp_relative_continuation(sumti, subbridi, tense_modal, normal_term)
                .reject_output(crate::grammar::baseline_relative::BaselineRelativeContinuationRejection)
        );
    }

    /// Product node for relative clause; preserves `zihe` and `inner` in source order.
    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// The `Zihe` cmavo marker.
        field zihe <- cmavo(Zihe).wf();
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for the camxes-exp relative-clause continuation; preserves `connective` and `inner` in source order.
    rule "relative clause" exp_relative_continuation(sumti, subbridi, tense_modal, normal_term) -> struct {
        /// The camxes-exp connective joining the adjacent relative clauses.
        field connective <- exp_relative_clause_connective;
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, normal_term));
    }

    /// Product node for the exact camxes-exp `NA? SE? (JOI / JA / A) NAI?` relative-clause connective.
    rule "relative clause connective" exp_relative_clause_connective -> struct {
        /// The optional left-negation prefix.
        field na <- opt(selmaho(Na));
        /// The optional conversion prefix.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JOI-, JA-, or A-class connective head; ZIhE is lexically JOI and is classified after the whole continuation parses.
        field head <- choice((
            selmaho(Joi),
            selmaho(Ja),
            selmaho(A),
        )).warn(ExperimentalRelativeClauseConnective).wf();
        /// The optional right-negation suffix.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for relative clause; selects among the `sumti_association_relative_clause` and `bridi_relative_clause` forms.
    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal, normal_term) -> enum {
        /// Uses the `sumti_association_relative_clause` product form, whose payload preserves `association_marker`, `sumti`, and `gehu`.
        sumti_association_relative_clause,
        /// Uses the nested `bridi_relative_clause` sum form and preserves its selected alternative.
        bridi_relative_clause,
    }

    /// Product node for sumti association phrase; preserves `association_marker`, `sumti`, and `gehu` in source order.
    ///
    /// The payload is the shared normal-flavour term constituent, which is what all three sources
    /// spell here: `relative_clause_1 <- GOI_clause free* nonabs_term GEhU?` (camxes.peg:168)
    /// and `GOI_clause free* term GEhU?` (camxes-exp.peg:207). It is deliberately ONE term
    /// rather than a `terms` run: on
    /// `ko'a goi ko'e ce'e ko'i broda` camxes-standard gives the payload only `ko'e` and leaves
    /// `ce'e ko'i` at the enclosing `terms_2` level with GEhU elided, so neither the CEhE nor the
    /// PEhE tier belongs inside the payload.
    rule "sumti association phrase" sumti_association_relative_clause(normal_term) -> struct {
        /// A word from selmaho `Goi`.
        field association_marker <- selmaho(Goi).wf();
        /// The shared normal-flavour term payload.
        field sumti <- arc(normal_term);
        /// The optional `Gehu` cmavo marker.
        field gehu <- opt(cmavo(Gehu).wf()).elidable_terminator(Gehu);
    }

    /// Sum node for relative bridi; selects among the two baseline subbridi owners.
    rule "relative bridi" bridi_relative_clause(subbridi) -> enum {
        /// Uses the `restrictive_bridi_relative_clause` product form, whose payload preserves `poi`, `subbridi`, and `kuho`.
        restrictive_bridi_relative_clause,
        /// Uses the `incidental_bridi_relative_clause` product form, whose payload preserves `noi`, `subbridi`, and `kuho`.
        incidental_bridi_relative_clause,
    }

    /// Product node for relative clause; preserves `poi`, `subbridi`, and `kuho` in source order.
    ///
    /// The marker set is camxes-standard's own NOI (camxes.peg:1695), which camxes-exp shares
    /// (:1807): `po'oi`, `voi'i` and `no'oi` are extensions and do not leak through this arm
    /// un-warned.
    rule "relative clause" restrictive_bridi_relative_clause(subbridi) -> struct {
        /// The selected grammar alternative in the `poi` structural role of the `restrictive_bridi_relative_clause` production.
        field poi <- choice((
            cmavo(Poi),
            cmavo(Voi),
        )).wf();
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `noi`, `subbridi`, and `kuho` in source order.
    rule "relative clause" incidental_bridi_relative_clause(subbridi) -> struct {
        /// The `Noi` cmavo marker.
        field noi <- cmavo(Noi).wf();
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    // ---- camxes-exp's tanru-unit relative clause --------------------------------------
    //
    // `selbri_relative_clauses <- selbri_relative_clause ((ZIhE_clause / joik) free*
    // selbri_relative_clause)* / gek selbri_relative_clauses gik selbri_relative_clauses`
    // and `selbri_relative_clause_1 <- NOhOI_clause free* subsentence KUhOI_elidible free*`
    // (camxes-exp.peg:214-218), with `NOhOI <- no'oi / po'oi` (:1907).  The `joik` is the
    // source's own, shared with the ordinary relative chain at :199 and merging A, JA and JOI
    // (:346-347), so the chain here carries the same transcription that chain does.  The
    // SA-erasure prefixes at :215-217 are omitted, as every other adopted camxes-exp family
    // omits them: jbotci handles that recovery at the `#[recovery_boundary]` layer instead.

    /// Sum node for selbri relative clauses; selects among the `exp_forethought_selbri_relative_clauses` and `exp_afterthought_selbri_relative_clauses` forms.
    rule "selbri relative clauses" exp_selbri_relative_clauses(exp_selbri_relative_clauses, exp_subsentence, tense_modal, selbri, letter_tokens) -> enum {
        /// Uses the `exp_forethought_selbri_relative_clauses` product form, whose payload preserves `gek`, `first`, `gik`, and `second`.
        exp_forethought_selbri_relative_clauses,
        /// Uses the `exp_afterthought_selbri_relative_clauses` product form, whose payload preserves `first` and `additional`.
        exp_afterthought_selbri_relative_clauses,
    }

    /// Product node for selbri relative clauses; preserves `gek`, `first`, `gik`, and `second` in source order.
    rule "selbri relative clauses" exp_forethought_selbri_relative_clauses(exp_selbri_relative_clauses, tense_modal, selbri, letter_tokens) -> struct {
        /// The forethought connective that opens the pair.
        field gek <- modal_forethought_connective(tense_modal, selbri, letter_tokens);
        /// The first relative-clause chain.
        field first <- arc(exp_selbri_relative_clauses);
        /// The GI-family connective separating the branches.
        field gik <- gik_connective;
        /// The second relative-clause chain.
        field second <- arc(exp_selbri_relative_clauses);
    }

    /// Product node for selbri relative clauses; preserves `first` and `additional` in source order.
    rule "selbri relative clauses" exp_afterthought_selbri_relative_clauses(exp_subsentence) -> struct {
        /// The initial relative clause before the ZIhE/joik continuations.
        field first <- exp_selbri_relative_clause(exp_subsentence);
        /// Ordered sequence of zero or more additional components, each without the
        /// free-modifier placement camxes-exp's `joik` does not spell. The shared connective
        /// nodes carry a `free*` slot on their head, before the optional `NAI`; `(ZIhE_clause /
        /// joik) free*` (:214) puts the chain's frees AFTER the completed connective and `joik`
        /// (:347-349) has no slot inside it. Those nodes are shared with routes the epoch base
        /// already reaches, so the placement is refused on this chain's completed continuation
        /// rather than removed from them -- see the rejection's own documentation and #847.
        field additional <- [zero_or_more exp_selbri_relative_clause_continuation(exp_subsentence)
            .reject_output(crate::grammar::baseline_relative::ProhibitedRelativeConnectiveFreeModifierRejection)];
    }

    /// Product node for selbri relative clauses; preserves `connective` and `inner` in source order.
    rule "selbri relative clauses" exp_selbri_relative_clause_continuation(exp_subsentence) -> struct {
        /// The connective joining the adjacent clauses.
        field connective <- exp_selbri_relative_clause_connective;
        /// The following relative clause.
        field inner <- exp_selbri_relative_clause(exp_subsentence);
    }

    /// Sum node for relative clause connective; selects among the `zihe_selbri_relative_connective` and `exp_relative_clause_connective` forms.
    ///
    /// This is `joik` as camxes-exp spells it, whole: `NA_clause? SE_clause? (JOI_clause /
    /// JA_clause / A_clause) NAI_clause? / interval / GAhO_clause interval GAhO_clause` with
    /// `interval <- SE_clause? BIhI_clause NAI_clause?` (:346-349), under an explicit A-JA-JOI
    /// merge. The first alternative already has an exact transcription in
    /// `exp_relative_clause_connective`, which the ordinary relative chain uses for the same
    /// source `joik` at :199, and the other two already have one in the two interval arms of
    /// jbotci's `joik_connective`, so all three are reused rather than restated.
    ///
    /// What is NOT reused is `joik_connective` itself. It is not this language in either
    /// direction: it is narrower, because jbotci splits camxes-exp's merged inventory across
    /// `joik_connective`, `jek_connective` and `ek_connective`. Its narrowness left
    /// `broda po'oi mi brode je po'oi do brodi` -- camxes-exp's under R2 -- with no route.
    rule "relative clause connective" exp_selbri_relative_clause_connective -> enum {
        /// Uses the `zihe_selbri_relative_connective` product form, whose payload preserves `zihe`.
        zihe_selbri_relative_connective,
        /// Uses the shared `exp_relative_clause_connective` product form, whose payload preserves `na`, `se`, `head`, and `nai`.
        exp_relative_clause_connective,
        /// The source `joik`'s bare `interval`: `SE_clause? BIhI_clause NAI_clause?` (:349).
        simple_interval_connective,
        /// The source `joik`'s `GAhO_clause interval GAhO_clause` (:347).
        closed_interval_connective,
    }

    /// Transparent product node for relative clause connective; preserves the `zihe` component.
    rule "relative clause connective" zihe_selbri_relative_connective -> struct {
        /// The `Zihe` cmavo marker.
        field zihe <- cmavo(Zihe).wf();
    }

    /// Product node for selbri relative clause; preserves `nohoi`, `subsentence`, and `kuhoi` in source order.
    rule "selbri relative clause" exp_selbri_relative_clause(exp_subsentence) -> struct {
        /// The NOhOI marker, which carries the warning for the whole construct.
        field nohoi <- choice((
            cmavo(Nohoi),
            cmavo(Pohoi),
        )).warn(ExperimentalNohoiSelbriRelativeClause).wf();
        /// The shared subsentence child syntax node.
        field subsentence <- arc(exp_subsentence);
        /// The optional `Kuhoi` cmavo marker.
        field kuhoi <- opt(cmavo(Kuhoi).wf()).elidable_terminator(Kuhoi);
    }

    /// Product node for ek; preserves `na`, `se`, `a`, and `nai` in source order.
    rule "ek" ek_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `A`.
        field a <- selmaho(A).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for ek; preserves `na`, `se`, `jehi`, and `nai` in source order.
    rule "ek" jehi_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Jehi`.
        field jehi <- selmaho(Jehi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.
    rule "jek" jek_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Ja`.
        field ja <- selmaho(Ja).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for joik. Baseline paired intervals retain priority over the simple arms.
    rule "joik" joik_connective -> enum {
        /// Uses the `closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.
        closed_interval_connective,
        /// Uses the `joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        joi_connective,
        /// Uses the `simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        simple_interval_connective,
        /// camxes-exp's NA before JOI, under the `na-joik` dialect feature. No other arm starts
        /// with NA, so its place in the order does not decide any reading.
        when feature(NaJoik) exp_na_joi_connective,
    }

    /// camxes-exp's merged `joik` with NA before JOI: `NA_clause SE_clause? JOI_clause NAI_clause?`
    /// (camxes-exp.peg:347). It reads NA JOI as one connective, so it changes the reading of
    /// texts that the default grammar parses with a bare NA term before a JOI connection, such
    /// as `ko'a na joi ko'e broda`. It is therefore reached only under the `na-joik` dialect
    /// feature, which also gives the bare NA term camxes-exp's `!joik_jek` guard.
    rule "joik" exp_na_joi_connective -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na);
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JOI word, which carries the warning for the construct.
        field joi <- selmaho(Joi).warn(ExperimentalNaJoiConnective).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for joik; preserves `se`, `joi`, and `nai` in source order.
    rule "joik" joi_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Joi`.
        field joi <- selmaho(Joi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval; preserves `se`, `bihi`, and `nai` in source order.
    rule "interval" simple_interval_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.
    rule "interval" closed_interval_connective -> struct {
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field left_interval <- selmaho(Gaho);
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field right_interval <- selmaho(Gaho).wf();
    }

    /// Transparent product node for non-logical connective; preserves the `vuhu` component.
    rule "non-logical connective" vuhu_nonlogical_connective -> struct {
        #[tree_child(primary)]
        /// A word from selmaho `Vuhu`.
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Sum node for sumti connective; selects among the `joik_connective`, `ek_connective`, `jehi_connective`, and `experimental_vuhu_sumti_connective` forms.
    rule "sumti connective" sumti_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `jehi_connective` product form, whose payload preserves `na`, `se`, `jehi`, and `nai`.
        jehi_connective,
        /// Uses the warning-gated `experimental_vuhu_sumti_connective` product form, whose payload preserves `vuhu`.
        experimental_vuhu_sumti_connective,
    }

    /// Transparent product node for the camxes-exp VUhU sumti connective extension.
    rule "sumti connective" experimental_vuhu_sumti_connective -> struct {
        #[tree_child(primary)]
        /// The VUhU word accepted at a sumti connective boundary.
        field vuhu <- selmaho(Vuhu).warn(ExperimentalVuhuConnective).wf();
    }

    /// Sum node for operand connective; selects among the `joik_connective` and `ek_connective` forms.
    rule "operand connective" operand_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
    }

    /// Sum node for the standard selbri connective inventory. Unlike the
    /// legacy shared relation connective, this deliberately excludes EK/A and
    /// VUhU, which camxes-standard does not admit at selbri levels 4 or 5.
    rule "selbri connective" selbri_afterthought_connective -> enum {
        /// A JOI-family connective.
        joik_connective,
        /// A JA-family connective.
        jek_connective,
    }

    /// Sum node for statement connective; selects among the `joik_connective` and `jek_connective` forms.
    rule "statement connective" standard_statement_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
    }

    /// Sum node for statement connective; selects among the `joik_connective`, `jek_connective`, `ek_connective`, and `vuhu_nonlogical_connective` forms.
    rule "statement connective" statement_connective -> enum {
        // The standard JOI and JA connectives.
        splice standard_statement_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.
        vuhu_nonlogical_connective,
    }

    /// Sum node for text connective; selects among the `standard_statement_connective` and `cehe_connective` forms.
    rule "text connective" text_leading_connective -> enum {
        /// Uses the nested `standard_statement_connective` sum form and preserves its selected alternative.
        standard_statement_connective,
        /// Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.
        cehe_connective,
    }

    /// Sum node for statement connective; selects among the `i_standard_statement_connective` and `i_tag_bo_statement_connective` forms.
    rule "statement connective" i_statement_connective(tense_modal) -> enum {
        /// Uses the `i_standard_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.
        i_standard_statement_connective,
        /// Uses the `i_tag_bo_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.
        i_tag_bo_statement_connective,
    }

    /// Product node for statement connective; preserves `connective` and `tag_bo` in source order.
    rule "statement connective" i_standard_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(statement_connective);
        /// The optional pair containing an optional shared tense-modal child followed by a required `Bo` cmavo marker.
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo).wf()));
    }

    /// Sum node for statement connective; selects among the `i_standard_paragraph_statement_connective` and `i_tag_bo_paragraph_statement_connective` forms.
    rule "statement connective" i_paragraph_statement_connective(tense_modal) -> enum {
        /// Uses the `i_standard_paragraph_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.
        i_standard_paragraph_statement_connective,
        /// Uses the `i_tag_bo_paragraph_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.
        i_tag_bo_paragraph_statement_connective,
    }

    /// Product node for statement connective; preserves `connective` and `tag_bo` in source order.
    rule "statement connective" i_standard_paragraph_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(paragraph_standard_statement_connective);
        /// The optional pair containing an optional shared tense-modal child followed by a required `Bo` cmavo marker.
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo)));
    }

    /// Paragraph JOIK family with the same mixed ownership ordering as `joik_connective`.
    rule "statement connective" paragraph_standard_statement_connective -> enum {
        /// Uses the `paragraph_closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.
        paragraph_closed_interval_connective,
        /// Uses the `paragraph_joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        paragraph_joi_connective,
        /// Uses the `paragraph_simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        paragraph_simple_interval_connective,
        /// Uses the `paragraph_jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        paragraph_jek_connective,
        /// camxes-exp's NA before JOI, under the `na-joik` dialect feature.
        when feature(NaJoik) exp_paragraph_na_joi_connective,
    }

    /// The paragraph family's form of [`exp_na_joi_connective`], for the connective after a
    /// leading I, with this family's handling of free modifiers.
    rule "joik" exp_paragraph_na_joi_connective -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na);
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JOI word, which carries the warning for the construct.
        field joi <- selmaho(Joi).warn(ExperimentalNaJoiConnective);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.
    rule "jek" paragraph_jek_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Ja`.
        field ja <- selmaho(Ja);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for joik; preserves `se`, `joi`, and `nai` in source order.
    rule "joik" paragraph_joi_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Joi`.
        field joi <- selmaho(Joi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for interval; preserves `se`, `bihi`, and `nai` in source order.
    rule "interval" paragraph_simple_interval_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.
    rule "interval" paragraph_closed_interval_connective -> struct {
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field left_interval <- selmaho(Gaho);
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
        #[tree_child(primary)]
        /// A word from selmaho `Gaho`.
        field right_interval <- selmaho(Gaho);
    }

    /// Product node for statement connective; preserves `tense_modal` and `bo` in source order.
    rule "statement connective" i_tag_bo_paragraph_statement_connective(tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo);
    }

    /// Product node for statement connective; preserves `tense_modal` and `bo` in source order.
    rule "statement connective" i_tag_bo_statement_connective(tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
    }

    /// Product node for termset connective; preserves `cehe` and `nai` in source order.
    rule "termset connective" cehe_connective -> struct {
        #[tree_child(primary)]
        /// The `Cehe` cmavo marker.
        field cehe <- cmavo(Cehe).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for gihek; preserves `na`, `se`, `giha`, and `nai` in source order.
    rule "gihek" gihek_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Giha`.
        field giha <- selmaho(Giha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for forethought selbri connective; preserves `se`, `guha`, and `nai` in source order.
    rule "forethought selbri connective" guhek_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Guha`.
        field guha <- selmaho(Guha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for bridi tail connective. camxes-standard's inventory at every bridi-tail joint is
    /// GIhA alone (camxes.peg:77-79). camxes-exp's `gihek_1` adds JA and JOI (camxes-exp.peg:340).
    ///
    /// The JA/JOI arm cannot take an extent that the baseline derives. Every joint is tried
    /// only after its left operand is complete, and that operand's selbri has already taken
    /// any JA or JOI that can start a tanru continuation (`klama je cadzu`, `klama je bo cadzu`,
    /// `klama joi ke cadzu ke'e`). So the arm only sees a JA or JOI that the selbri could not
    /// use: after VAU or tail terms, or before CU, a tag, BO or KE that no tanru can follow.
    rule "bridi tail connective" bridi_tail_connective -> enum {
        /// Uses the `gihek_connective` product form, whose payload preserves `na`, `se`, `giha`, and `nai`.
        gihek_connective,
        /// camxes-exp's JA and JOI arms of `gihek_1`.
        exp_ja_joi_bridi_tail_connective,
    }

    /// camxes-exp's JA and JOI arms of `gihek_1 <- NA_clause? SE_clause? (JA_clause / JOI_clause /
    /// GIhA_clause / ...) NAI_clause?` (camxes-exp.peg:340). The GI-led arms of the same rule are
    /// not part of jbotci.
    rule "bridi tail connective" exp_ja_joi_bridi_tail_connective -> struct {
        // The JA or JOI head must be present in strict lookahead before recovery may enter the
        // arm, so missing-token recovery never invents a JA/JOI joint.
        assert (opt(selmaho(Na)), opt(selmaho(Se)), choice((selmaho(Ja), selmaho(Joi)))).lookahead();
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JA or JOI word, which carries the warning for the joint.
        field connective <- choice((selmaho(Ja), selmaho(Joi)))
            .warn(ExperimentalJacuPredicateTailConnective)
            .wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Forethought connective family.
    rule "forethought connective" modal_forethought_connective(tense_modal, selbri, letter_tokens) -> enum {
        /// Uses the `ga_forethought_connective` product form, whose payload preserves `se`, `ga`, and `nai`.
        ga_forethought_connective,
        /// Uses the `joik_jek_gi_forethought_connective` product form, whose payload preserves `connective` and `gi`.
        joik_jek_gi_forethought_connective,
        /// Uses the `modal_gi_forethought_connective` product form, whose payload preserves `tense_modal`, `gi`, and `nai`.
        modal_gi_forethought_connective,
        /// camxes-exp's JA before GI.
        exp_ja_gi_forethought_connective,
    }

    /// Product node for forethought connective; preserves `se`, `ga`, and `nai` in source order.
    rule "forethought connective" ga_forethought_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Ga`.
        field ga <- selmaho(Ga).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for forethought connective; preserves `connective` and `gi` in source order.
    rule "forethought connective" joik_jek_gi_forethought_connective -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(joik_connective);
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
    }

    /// camxes-exp's `gek <- ... / joik GI_clause free* / ...` (camxes-exp.peg:361) with a JA head,
    /// which camxes-exp's merged `joik` admits (:347): `NA? SE? JA NAI? GI`. camxes-standard's
    /// JOIK before GI takes JOI and the intervals only, which `joik_jek_gi_forethought_connective`
    /// keeps. camxes-exp also admits A here, but jbotci never accepted A before GI (#980). Free
    /// modifiers follow each word as they do in that JOI arm. The arm is last, so it is reached
    /// only where no other opener applies.
    rule "forethought connective" exp_ja_gi_forethought_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JA word, which carries the warning for the construct.
        field ja <- selmaho(Ja).warn(ExperimentalJaGiForethoughtConnective).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
    }

    /// Product node for forethought connective; preserves `tense_modal`, `gi`, and `nai` in source order.
    rule "forethought connective" modal_gi_forethought_connective(tense_modal) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional standard `Nai` suffix on the tag-opened GI connective.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for forethought connective; preserves `gi` and `nai` in source order.
    rule "forethought connective" gik_connective -> struct {
        #[tree_child(primary)]
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Transparent product node for tag; preserves the `body` component.
    rule "tag" tense_modal(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        // The words that can start a tag. ROI is not one: an interval property is
        // `number ROI NAI?`, so ROI always follows a number (PA), a NIhE or MOhE operand, or a
        // VEI mex, which are listed.
        assert choice((
            cmavo(Fiho),
            selmaho(Bai),
            selmaho(Nahe),
            selmaho(Se),
            selmaho(Fa),
            cmavo(Ki),
            selmaho(Cuhe),
            selmaho(Pu),
            selmaho(Zi),
            selmaho(Zeha),
            selmaho(Va),
            selmaho(Faha),
            selmaho(Veha),
            selmaho(Viha),
            selmaho(Caha),
            selmaho(Zaho),
            selmaho(Tahe),
            cmavo(Fehe),
            selmaho(Mohi),
            cmavo(Nihe),
            cmavo(Mohe),
            cmavo(Vei),
            pa_word(),
        ));
        #[tree_child(primary)]
        /// The `tense_modal_body` grammar result in the `body` structural role of the `tense_modal` production.
        field body <- tense_modal_body(selbri, sumti, mekso, letter_tokens, letter_string);
    }

    /// Sum node for tag; selects among the baseline and experimental arms.
    rule "tag" tense_modal_body(selbri, sumti, mekso, letter_tokens, letter_string) -> enum {
        /// Uses the `connected_tense_modal` product form, whose payload preserves `first` and `continuations`.
        connected_tense_modal,
        /// Uses the nested `tense_modal_atom` sum form and preserves its selected alternative.
        tense_modal_atom,
    }

    /// Baseline-only tag body used at term entry, where extension tags are not in the source grammar.
    rule "baseline term tag" baseline_term_tense_modal(selbri, letter_tokens, letter_string) -> enum {
        /// A baseline connected tag.
        baseline_term_connected_tense_modal,
        /// A single baseline tag atom.
        baseline_term_tense_modal_atom,
    }

    /// Baseline-only connected tag used at term entry.
    rule "baseline term connected tag" baseline_term_connected_tense_modal(selbri, letter_tokens, letter_string) -> struct {
        /// The first baseline atom.
        field first <- arc(baseline_term_tense_modal_atom(selbri, letter_tokens, letter_string));
        /// Non-empty source-ordered baseline continuations.
        field continuations <- [one_or_more baseline_term_connected_tense_modal_continuation(selbri, letter_tokens, letter_string)];
    }

    /// One continuation in a baseline-only connected term tag.
    rule "baseline term connected tag continuation" baseline_term_connected_tense_modal_continuation(selbri, letter_tokens, letter_string) -> struct {
        /// The connective between adjacent baseline atoms.
        field connective <- tense_modal_connective;
        /// The following baseline atom.
        field tense_modal <- arc(baseline_term_tense_modal_atom(selbri, letter_tokens, letter_string));
    }

    /// Exact baseline atom inventory accepted at term entry.
    rule "baseline term tag atom" baseline_term_tense_modal_atom(selbri, letter_tokens, letter_string) -> enum {
        /// A baseline composite tense.
        composite_tense,
        /// A baseline FIhO modal.
        fiho_tense,
        /// A baseline BAI modal.
        modal_tense,
        /// A baseline KI marker.
        sticky_tense,
    }

    /// Product node for connected tag; preserves `first` and `continuations` in source order.
    rule "connected tag" connected_tense_modal(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The shared first child syntax node.
        field first <- arc(tense_modal_atom(selbri, sumti, mekso, letter_tokens, letter_string));
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more connected_tense_modal_continuation(selbri, sumti, mekso, letter_tokens, letter_string)];
    }

    /// Product node for connected tag continuation; preserves `connective` and `tense_modal` in source order.
    rule "connected tag continuation" connected_tense_modal_continuation(selbri, sumti, mekso, letter_tokens, letter_string) -> struct {
        /// The `tense_modal_connective` connective joining the adjacent constituents of the `connected_tense_modal_continuation` production.
        field connective <- tense_modal_connective;
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal_atom(selbri, sumti, mekso, letter_tokens, letter_string));
    }

    /// Sum node for tag connective; selects among the `joik_connective` and `jek_connective` forms.
    rule "tag connective" tense_modal_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
    }

    /// Sum node for one connective arm of a tag.
    rule "tag" tense_modal_atom(selbri, sumti, mekso, letter_tokens, letter_string) -> enum {
        /// Uses one complete corrected camxes-exp atom run when it is not a baseline tag.
        exp_tag_atom_run,
        // The baseline atoms.
        splice baseline_term_tense_modal_atom,
    }

    /// Product node for FIhO modal; preserves `fiho`, `selbri`, and `fehu` in source order.
    rule "FIhO modal" fiho_tense(selbri) -> struct {
        /// The `Fiho` cmavo marker.
        field fiho <- cmavo(Fiho).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Fehu` cmavo marker.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Transparent ownership-filtered wrapper for one corrected camxes-exp tense-modal.
    rule "experimental tag atom run" exp_tag_atom_run(selbri, sumti, mekso) -> struct {
        /// The complete run, retained only when the baseline grammar does not own its extent.
        #[tree_child(primary)]
        field run <- arc(
            exp_tag_atom_run_body(selbri, sumti, mekso)
                .reject_output(crate::grammar::baseline_tag::BaselineTagRejection)
        );
    }

    /// One corrected camxes-exp tense-modal: a nonempty run of uniformly prefixed atoms.
    rule "experimental tag atom run body" exp_tag_atom_run_body(selbri, sumti, mekso) -> struct {
        /// The first source-ordered atom.
        field first <- arc(exp_prefixed_tag_atom(selbri, sumti, mekso));
        /// Remaining source-ordered atoms in this same tense-modal arm.
        field additional <- [zero_or_more arc(exp_prefixed_tag_atom(selbri, sumti, mekso))];
    }

    /// One corrected camxes-exp atom with the uniform optional NAhE/SE prefix domain.
    rule "experimental prefixed tag atom" exp_prefixed_tag_atom(selbri, sumti, mekso) -> struct {
        /// Optional scalar-negation prefix. A free modifier here remains parse-preserving
        /// through the preceding boundary, but camxes-exp does not source it on NAhE itself.
        field nahe <- opt(selmaho(Nahe).prohibited_wf());
        /// Optional conversion prefix. camxes-exp rejects a free modifier between SE and
        /// its atom; the explicitly unrestricted policy is the only widening route.
        field se <- opt(selmaho(Se).wf_when(UnrestrictedFree));
        /// The exact P08 atom, followed by the atom-local free-modifier boundary.
        field atom <- arc(exp_tag_atom(selbri, sumti, mekso)).wf();
    }

    /// Exact corrected camxes-exp tag-atom inventory.
    rule "experimental tag atom" exp_tag_atom(selbri, sumti, mekso) -> enum {
        /// A BAI-family modal atom.
        exp_bai_tag_atom,
        /// A CAhA actuality atom.
        exp_caha_tag_atom,
        /// A CUhE tense-question atom.
        exp_cuhe_tag_atom,
        /// A KI stickiness atom.
        exp_ki_tag_atom,
        /// A ZI time-distance atom.
        exp_zi_tag_atom,
        /// A PU time-direction atom.
        exp_pu_tag_atom,
        /// A VA space-distance atom.
        exp_va_tag_atom,
        /// An optional-MOhI FAhA direction atom.
        exp_faha_tag_atom,
        /// A ZEhA time-interval atom.
        exp_zeha_tag_atom,
        /// A VEhA space-interval atom.
        exp_veha_tag_atom,
        /// A VIhA space-interval-shape atom.
        exp_viha_tag_atom,
        /// A numeric or parenthesized-mex ROI atom.
        exp_roi_tag_atom,
        /// An optionally FEhE-prefixed TAhE atom.
        exp_tahe_tag_atom,
        /// An optionally FEhE-prefixed ZAhO atom.
        exp_zaho_tag_atom,
        /// A FIhO/selbri/FEhU atom.
        exp_fiho_tag_atom,
        /// A FA place atom.
        exp_fa_tag_atom,
    }

    /// One BAI-family atom in a corrected camxes-exp tag run.
    rule "experimental BAI tag atom" exp_bai_tag_atom -> struct {
        /// The BAI-family modal word.
        field bai <- selmaho(Bai);
    }

    /// One CAhA-family atom in a corrected camxes-exp tag run.
    rule "experimental CAhA tag atom" exp_caha_tag_atom -> struct {
        /// The CAhA-family actuality word.
        field caha <- selmaho(Caha);
    }

    /// One CUhE atom in a corrected camxes-exp tag run.
    rule "experimental CUhE tag atom" exp_cuhe_tag_atom -> struct {
        /// The CUhE-family tense question word.
        field cuhe <- selmaho(Cuhe);
    }

    /// One KI atom in a corrected camxes-exp tag run.
    rule "experimental KI tag atom" exp_ki_tag_atom -> struct {
        /// The KI stickiness marker.
        field ki <- cmavo(Ki);
    }

    /// One ZI-family atom in a corrected camxes-exp tag run.
    rule "experimental ZI tag atom" exp_zi_tag_atom -> struct {
        /// The ZI-family temporal-distance word.
        field zi <- selmaho(Zi);
    }

    /// One PU-family atom in a corrected camxes-exp tag run.
    rule "experimental PU tag atom" exp_pu_tag_atom -> struct {
        /// The PU-family temporal-direction word.
        field pu <- selmaho(Pu);
    }

    /// One VA-family atom in a corrected camxes-exp tag run.
    rule "experimental VA tag atom" exp_va_tag_atom -> struct {
        /// The VA-family spatial-distance word.
        field va <- selmaho(Va);
    }

    /// One optionally MOhI-prefixed FAhA atom in a corrected camxes-exp tag run.
    rule "experimental FAhA tag atom" exp_faha_tag_atom -> struct {
        /// Optional MOhI motion-relative prefix.
        field mohi <- opt(selmaho(Mohi));
        /// The FAhA-family spatial-direction word.
        field faha <- selmaho(Faha);
    }

    /// One ZEhA-family atom in a corrected camxes-exp tag run.
    rule "experimental ZEhA tag atom" exp_zeha_tag_atom -> struct {
        /// The ZEhA-family temporal-interval word.
        field zeha <- selmaho(Zeha);
    }

    /// One VEhA-family atom in a corrected camxes-exp tag run.
    rule "experimental VEhA tag atom" exp_veha_tag_atom -> struct {
        /// The VEhA-family spatial-interval word.
        field veha <- selmaho(Veha);
    }

    /// One VIhA-family atom in a corrected camxes-exp tag run.
    rule "experimental VIhA tag atom" exp_viha_tag_atom -> struct {
        /// The VIhA-family spatial-interval-shape word.
        field viha <- selmaho(Viha);
    }

    /// One ROI atom with its exact corrected camxes-exp interval payload.
    rule "experimental ROI tag atom" exp_roi_tag_atom(selbri, sumti, mekso) -> struct {
        /// Optional FEhE spatial-aspect prefix.
        field fehe <- opt(cmavo(Fehe));
        /// The numeric or parenthesized-mex interval payload.
        field interval <- exp_roi_interval(selbri, sumti, mekso);
        /// The ROI interval-property marker.
        field roi <- selmaho(Roi);
    }

    /// The corrected camxes-exp payload alternatives accepted before ROI.
    rule "experimental ROI interval" exp_roi_interval(selbri, sumti, mekso) -> enum {
        /// A VEI-delimited full mex.
        exp_parenthesized_roi_interval,
        /// The exact camxes-exp number language.
        exp_number,
    }

    /// A parenthesized full mex used as a corrected camxes-exp ROI payload.
    rule "experimental parenthesized ROI interval" exp_parenthesized_roi_interval(mekso) -> struct {
        /// The opening VEI marker.
        field vei <- cmavo(Vei).wf();
        /// The complete mex payload.
        field expression <- arc(mekso);
        /// The optional elidable VEhO terminator.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// The exact nonempty corrected camxes-exp number language used before ROI.
    rule "experimental number" exp_number(selbri, sumti) -> struct {
        /// The first number element.
        field first <- arc(exp_number_atom(selbri, sumti));
        /// Remaining source-ordered number elements.
        field additional <- [zero_or_more arc(exp_number_atom(selbri, sumti))];
    }

    /// One element of the exact corrected camxes-exp number language.
    rule "experimental number atom" exp_number_atom(selbri, sumti) -> enum {
        /// One PA-family digit or number word.
        exp_pa_number_atom,
        /// One NIhE/selbri/TEhU number element.
        exp_nihe_number_atom,
        /// One MOhE/sumti/TEhU number element.
        exp_mohe_number_atom,
    }

    /// One PA-family element of a corrected camxes-exp number.
    rule "experimental PA number atom" exp_pa_number_atom -> struct {
        /// The PA-family number word.
        field pa <- selmaho(Pa);
    }

    /// One NIhE selbri-derived element of a corrected camxes-exp number.
    rule "experimental NIhE number atom" exp_nihe_number_atom(selbri) -> struct {
        /// The NIhE conversion marker.
        field nihe <- cmavo(Nihe).wf();
        /// The converted selbri.
        field selbri <- arc(selbri);
        /// The optional elidable TEhU terminator.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// One MOhE sumti-derived element of a corrected camxes-exp number.
    rule "experimental MOhE number atom" exp_mohe_number_atom(sumti) -> struct {
        /// The MOhE conversion marker.
        field mohe <- cmavo(Mohe).wf();
        /// The converted sumti.
        field sumti <- arc(sumti);
        /// The optional elidable TEhU terminator.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// One optionally FEhE-prefixed TAhE atom.
    rule "experimental TAhE tag atom" exp_tahe_tag_atom -> struct {
        /// Optional FEhE spatial-aspect prefix.
        field fehe <- opt(cmavo(Fehe));
        /// The TAhE-family interval-property word.
        field tahe <- selmaho(Tahe);
    }

    /// One optionally FEhE-prefixed ZAhO atom.
    rule "experimental ZAhO tag atom" exp_zaho_tag_atom -> struct {
        /// Optional FEhE spatial-aspect prefix.
        field fehe <- opt(cmavo(Fehe));
        /// The ZAhO-family interval-property word.
        field zaho <- selmaho(Zaho);
    }

    /// One FIhO ad-hoc modal atom with its selbri payload.
    rule "experimental FIhO tag atom" exp_fiho_tag_atom(selbri) -> struct {
        /// The FIhO marker and its sourced following free-modifier boundary.
        field fiho <- cmavo(Fiho).wf();
        /// The ad-hoc modal selbri.
        field selbri <- arc(selbri);
        /// The optional elidable FEhU terminator.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// One FA place atom in the corrected camxes-exp tag inventory.
    rule "experimental FA tag atom" exp_fa_tag_atom -> struct {
        /// The FA-family place word, carrying its dedicated warning category.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag);
    }

    /// Sum node for tag; selects among the `prefixed_time_space_caha_tense`, `time_space_caha_ki_tense`, and `cuhe_tense` forms.
    rule "tag" composite_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `prefixed_time_space_caha_tense` product form, whose payload preserves `nahe`, `tense`, and `ki`.
        prefixed_time_space_caha_tense,
        /// Uses the `time_space_caha_ki_tense` product form, whose payload preserves `tense` and `ki`.
        time_space_caha_ki_tense,
        /// Uses the `cuhe_tense` product form, whose payload preserves `cuhe`.
        cuhe_tense,
    }

    /// Product node for tag; preserves `nahe`, `tense`, and `ki` in source order.
    rule "tag" prefixed_time_space_caha_tense(letter_tokens, letter_string) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared tense child syntax node.
        field tense <- arc(time_space_caha_tense(letter_tokens, letter_string));
        /// The optional ki component.
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Product node for tag; preserves `tense` and `ki` in source order.
    rule "tag" time_space_caha_ki_tense(letter_tokens, letter_string) -> struct {
        /// The shared tense child syntax node.
        field tense <- arc(time_space_caha_tense(letter_tokens, letter_string));
        /// The optional ki component.
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Sum node for tag; selects among the `time_then_space_caha_tense`, `space_then_time_caha_tense`, and `caha_tense` forms.
    rule "tag" time_space_caha_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `time_then_space_caha_tense` product form, whose payload preserves `time`, `space`, and `caha`.
        time_then_space_caha_tense,
        /// Uses the `space_then_time_caha_tense` product form, whose payload preserves `space`, `time`, and `caha`.
        space_then_time_caha_tense,
        /// Uses the `caha_tense` product form, whose payload preserves `caha`.
        caha_tense,
    }

    /// Product node for time tense; preserves `time`, `space`, and `caha` in source order.
    rule "time tense" time_then_space_caha_tense(letter_tokens, letter_string) -> struct {
        /// The shared time child syntax node.
        field time <- arc(time_tense(letter_tokens, letter_string));
        /// The optional space component.
        field space <- opt(arc(space_tense(letter_tokens, letter_string)));
        /// The optional caha component.
        field caha <- opt(arc(caha_tense()));
    }

    /// Product node for space tense; preserves `space`, `time`, and `caha` in source order.
    rule "space tense" space_then_time_caha_tense(letter_tokens, letter_string) -> struct {
        /// The shared space child syntax node.
        field space <- arc(space_tense(letter_tokens, letter_string));
        /// The optional time component.
        field time <- opt(arc(time_tense(letter_tokens, letter_string)));
        /// The optional caha component.
        field caha <- opt(arc(caha_tense()));
    }

    /// Sum node for time tense; selects among the `time_tense_with_zi`, `time_tense_with_offset`, `time_tense_with_interval`, and `time_tense_with_properties` forms.
    rule "time tense" time_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `time_tense_with_zi` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.
        time_tense_with_zi,
        /// Uses the `time_tense_with_offset` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.
        time_tense_with_offset,
        /// Uses the `time_tense_with_interval` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.
        time_tense_with_interval,
        /// Uses the `time_tense_with_properties` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.
        time_tense_with_properties,
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_zi(letter_tokens, letter_string) -> struct {
        /// The shared zi child syntax node.
        field zi <- arc(zi_time_distance_tense());
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense(letter_tokens, letter_string))];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_offset(letter_tokens, letter_string) -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Non-empty ordered sequence of offsets components.
        field offsets <- [one_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense(letter_tokens, letter_string))];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_interval(letter_tokens, letter_string) -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The shared zeha child syntax node.
        field zeha <- arc(zeha_time_interval_tense());
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense(letter_tokens, letter_string))];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_properties(letter_tokens, letter_string) -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Non-empty ordered sequence of properties components.
        field properties <- [one_or_more arc(interval_property_tense(letter_tokens, letter_string))];
    }

    /// Sum node for interval property; selects among the `numbered_interval_property_tense`, `tahe_interval_property_tense`, and `zaho_interval_property_tense` forms.
    rule "interval property" interval_property_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `numbered_interval_property_tense` product form, whose payload preserves `number`, `roi`, and `nai`.
        numbered_interval_property_tense,
        /// Uses the `tahe_interval_property_tense` product form, whose payload preserves `tahe` and `nai`.
        tahe_interval_property_tense,
        /// Uses the `zaho_interval_property_tense` product form, whose payload preserves `zaho` and `nai`.
        zaho_interval_property_tense,
    }

    /// Product node for interval property; preserves `number`, `roi`, and `nai` in source order.
    rule "interval property" numbered_interval_property_tense(letter_tokens) -> struct {
        /// The shared `number_words` grammar result in the `number` structural role of the `numbered_interval_property_tense` production.
        field number <- number_words(letter_tokens).wf();
        /// A word from selmaho `Roi`.
        field roi <- selmaho(Roi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval property; preserves `tahe` and `nai` in source order.
    rule "interval property" tahe_interval_property_tense -> struct {
        /// A word from selmaho `Tahe`.
        field tahe <- selmaho(Tahe).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval property; preserves `zaho` and `nai` in source order.
    rule "interval property" zaho_interval_property_tense -> struct {
        /// A word from selmaho `Zaho`.
        field zaho <- selmaho(Zaho).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for time tense; preserves `pu`, `nai`, and `distance` in source order.
    rule "time tense" pu_time_offset_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional distance component.
        field distance <- opt(selmaho(Zi).wf());
    }

    /// Transparent product node for time tense; preserves the `zi` component.
    rule "time tense" zi_time_distance_tense -> struct {
        /// A word from selmaho `Zi`.
        field zi <- selmaho(Zi).wf();
    }

    /// Product node for time interval; preserves `zeha` and `direction` in source order.
    rule "time interval" zeha_time_interval_tense -> struct {
        /// A word from selmaho `Zeha`.
        field zeha <- selmaho(Zeha).wf();
        /// The optional pair containing a required PU-family direction word followed by an optional `Nai` cmavo marker.
        field direction <- opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
    }

    /// Sum node for space tense; selects among the `space_tense_with_va`, `space_tense_with_offset`, `space_tense_with_interval`, and `space_tense_with_mohi` forms.
    rule "space tense" space_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `space_tense_with_va` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.
        space_tense_with_va,
        /// Uses the `space_tense_with_offset` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.
        space_tense_with_offset,
        /// Uses the `space_tense_with_interval` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.
        space_tense_with_interval,
        /// Uses the `space_tense_with_mohi` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.
        space_tense_with_mohi,
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_va(letter_tokens, letter_string) -> struct {
        /// The shared va child syntax node.
        field va <- arc(va_space_distance_tense());
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense(letter_tokens, letter_string)));
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_offset(letter_tokens, letter_string) -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Non-empty ordered sequence of offsets components.
        field offsets <- [one_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense(letter_tokens, letter_string)));
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_interval(letter_tokens, letter_string) -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The shared interval child syntax node.
        field interval <- arc(space_interval_tense(letter_tokens, letter_string));
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_mohi(letter_tokens, letter_string) -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense(letter_tokens, letter_string)));
        /// The shared mohi child syntax node.
        field mohi <- arc(mohi_space_offset_tense());
    }

    /// Transparent product node for space tense; preserves the `va` component.
    rule "space tense" va_space_distance_tense -> struct {
        /// A word from selmaho `Va`.
        field va <- selmaho(Va).wf();
    }

    /// Product node for space tense; preserves `faha`, `nai`, and `distance` in source order.
    rule "space tense" faha_space_offset_tense -> struct {
        /// A word from selmaho `Faha`.
        field faha <- selmaho(Faha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional distance component.
        field distance <- opt(selmaho(Va).wf());
    }

    /// Product node for space interval; preserves `faha` and `nai` in source order.
    rule "space interval" faha_interval_direction_tense -> struct {
        /// A word from selmaho `Faha`.
        field faha <- selmaho(Faha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for space interval; selects among the `space_interval_with_extent_tense` and `space_interval_properties_tense` forms.
    rule "space interval" space_interval_tense(letter_tokens, letter_string) -> enum {
        /// Uses the `space_interval_with_extent_tense` product form, whose payload preserves `extent`, `direction`, and `properties`.
        space_interval_with_extent_tense,
        /// Uses the `space_interval_properties_tense` product form, whose payload preserves `first` and `additional`.
        space_interval_properties_tense,
    }

    /// Product node for space interval; preserves `extent`, `direction`, and `properties` in source order.
    rule "space interval" space_interval_with_extent_tense(letter_tokens, letter_string) -> struct {
        /// The shared extent child syntax node.
        field extent <- arc(space_interval_extent_tense);
        /// The optional direction component.
        field direction <- opt(arc(faha_interval_direction_tense()));
        /// The optional properties component.
        field properties <- opt(arc(space_interval_properties_tense(letter_tokens, letter_string)));
    }

    /// Sum node for space interval; selects among the `veha_space_interval_tense` and `viha_space_interval_tense` forms.
    rule "space interval" space_interval_extent_tense -> enum {
        /// Uses the `veha_space_interval_tense` product form, whose payload preserves `veha` and `viha`.
        veha_space_interval_tense,
        /// Uses the `viha_space_interval_tense` product form, whose payload preserves `viha`.
        viha_space_interval_tense,
    }

    /// Product node for space interval; preserves `first` and `additional` in source order.
    rule "space interval" space_interval_properties_tense(letter_tokens, letter_string) -> struct {
        /// The shared first child syntax node.
        field first <- arc(fehe_interval_property_tense(letter_tokens, letter_string));
        /// Ordered sequence of zero or more additional components.
        field additional <- [zero_or_more arc(fehe_interval_property_tense(letter_tokens, letter_string))];
    }

    /// Product node for space interval; preserves `veha` and `viha` in source order.
    rule "space interval" veha_space_interval_tense -> struct {
        /// A word from selmaho `Veha`.
        field veha <- selmaho(Veha).wf();
        /// The optional viha component.
        field viha <- opt(selmaho(Viha).wf());
    }

    /// Transparent product node for space interval; preserves the `viha` component.
    rule "space interval" viha_space_interval_tense -> struct {
        /// A word from selmaho `Viha`.
        field viha <- selmaho(Viha).wf();
    }

    /// Product node for space interval property; preserves `fehe` and `property` in source order.
    rule "space interval property" fehe_interval_property_tense(letter_tokens, letter_string) -> struct {
        /// The `Fehe` cmavo marker.
        field fehe <- cmavo(Fehe).wf();
        /// The shared property child syntax node.
        field property <- arc(interval_property_tense(letter_tokens, letter_string));
    }

    /// Product node for space tense; preserves `mohi` and `offset` in source order.
    rule "space tense" mohi_space_offset_tense -> struct {
        /// A word from selmaho `Mohi`.
        field mohi <- selmaho(Mohi).wf();
        /// The shared offset child syntax node.
        field offset <- arc(faha_space_offset_tense());
    }

    /// Transparent product node for tag; preserves the `caha` component.
    rule "tag" caha_tense -> struct {
        /// A word from selmaho `Caha`.
        field caha <- selmaho(Caha).wf();
    }

    /// Transparent product node for tag; preserves the `ki` component.
    rule "tag" ki_composite_tense -> struct {
        /// The `Ki` cmavo marker.
        field ki <- cmavo(Ki).wf();
    }

    /// Transparent product node for tag; preserves the `cuhe` component.
    rule "tag" cuhe_tense -> struct {
        /// A word from selmaho `Cuhe`.
        field cuhe <- selmaho(Cuhe).wf();
    }

    /// Product node for modal tag; preserves `nahe`, `se`, `bai`, `nai`, and `ki` in source order.
    rule "modal tag" modal_tense -> struct {
        /// The optional nahe component.
        field nahe <- opt(selmaho(Nahe).wf());
        /// The optional se component.
        field se <- opt(selmaho(Se).wf());
        /// A word from selmaho `Bai`.
        field bai <- selmaho(Bai).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional `Ki` cmavo marker.
        field ki <- opt(cmavo(Ki).wf());
    }

    /// Transparent product node for tag; preserves the `ki` component.
    rule "tag" sticky_tense -> struct {
        /// The `Ki` cmavo marker.
        field ki <- cmavo(Ki).wf();
    }

    /// Sum node for selbri; preserves the existing relative/CEI and ordinary owners.
    rule "selbri" selbri(selbri, co_selbri, tense_modal, statement, free_modifier) -> enum {
        /// Uses the `tagged_selbri` product form, whose payload preserves `tense_modal` and `inner_selbri`.
        tagged_selbri,
        /// Uses the nested `untagged_selbri` sum form and preserves its selected alternative.
        untagged_selbri,
    }

    /// Sum node for selbri level 1; selects between the recursive NA arm and level 2.
    rule "selbri" untagged_selbri(selbri, co_selbri, statement, free_modifier) -> enum {
        /// Uses the `negated_selbri` product form, whose payload preserves `na` and `inner_selbri`.
        negated_selbri,
        /// Uses the level-2 `co_selbri` product form.
        co_selbri,
    }

    /// Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.
    rule "tagged selbri" tagged_selbri(selbri, co_selbri, tense_modal, statement, free_modifier) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(untagged_selbri(selbri, co_selbri, statement, free_modifier));
    }

    /// Product node for negated selbri; preserves `na` and `inner_selbri` in source order.
    rule "negated selbri" negated_selbri(selbri) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
        /// The shared inner selbri child syntax node.
        // Post-NA is another extension-free entry boundary: an experimental run here
        // can otherwise steal a successful baseline reading in which NA and following
        // tag atoms are separate terms. Filtering the completed recursive result is the
        // recursive equivalent of the D2b baseline-parser mapping at term entry.
        field inner_selbri <- arc(selbri.reject_output(crate::grammar::baseline_tag::PostNaExtensionTagRejection));
    }

    /// Product node for selbri; preserves `leading_selbri` and `co_tail` in source order.
    rule "selbri" co_selbri(co_selbri, tanru_selbri, statement, free_modifier) -> struct {
        /// The level-3 selbri before the optional CO tail.
        field leading_selbri <- arc(tanru_selbri);
        /// The optional co tail component.
        field co_tail <- opt(co_selbri_tail(co_selbri));
    }

    /// Product node for selbri; preserves `co` and `trailing_selbri` in source order.
    rule "selbri" co_selbri_tail(co_selbri) -> struct {
        /// The `Co` cmavo marker.
        field co <- cmavo(Co).wf();
        /// The shared trailing selbri child syntax node.
        field trailing_selbri <- arc(co_selbri);
    }

    /// Product node for selbri level 3; adjacency is looser than level-4 connectives.
    rule "tanru" tanru_selbri(connected_selbri) -> struct {
        /// The first maximal level-4 connective group.
        field first_selbri <- arc(connected_selbri);
        /// Remaining adjacent level-4 connective groups.
        field additional_selbri <- [zero_or_more arc(connected_selbri)];
    }

    /// Product node for selbri level 4; ordinary joik/jek continuations bind
    /// more tightly than adjacency.
    rule "selbri connection" connected_selbri(bound_selbri, tanru_selbri, tense_modal, free_modifier) -> struct {
        /// The first level-5 selbri.
        field leading_selbri <- arc(bound_selbri);
        /// Source-ordered level-4 continuations.
        field continuations <- [zero_or_more arc(connected_selbri_continuation(bound_selbri, tanru_selbri, tense_modal))];
    }

    /// Sum node for the two standard level-4 continuation forms.
    rule "selbri connection continuation" connected_selbri_continuation(bound_selbri, tanru_selbri, tense_modal) -> enum {
        /// An ordinary joik/jek continuation whose operand is level 5.
        simple_connected_selbri_continuation,
        /// The joik-only tagged KE continuation from camxes selbri level 4.
        grouped_connected_selbri_continuation,
        /// camxes-exp's JA-led KE continuation at selbri level 4.
        exp_ja_grouped_connected_selbri_continuation,
    }

    /// Product node for an ordinary level-4 selbri continuation.
    rule "selbri connection continuation" simple_connected_selbri_continuation(bound_selbri) -> struct {
        /// The standard joik/jek selbri connective.
        field connective <- arc(selbri_afterthought_connective);
        /// The following level-5 selbri.
        field trailing_selbri <- arc(bound_selbri);
    }

    /// camxes-exp's `joik stag? KE_clause free* selbri_3 KEhE_elidible free*` at selbri level 4
    /// (camxes-exp.peg:234) with a JA head, which camxes-exp's merged `joik` admits
    /// (camxes-exp.peg:347). camxes-standard's KE continuation takes JOIK alone. A JA followed
    /// by a selbri_5 is the ordinary continuation, which comes first, so this arm only takes
    /// `JA stag KE`, as in `mi broda je pu ke brode ke'e`. Without it that text would reach the
    /// bridi-tail KE joint, which is not camxes-exp's reading.
    rule "grouped selbri connection continuation" exp_ja_grouped_connected_selbri_continuation(tanru_selbri, tense_modal) -> struct {
        // The JA head and the KE must be present in strict lookahead before recovery may enter
        // the arm; otherwise missing-token recovery synthesizes both and turns any stray KEhE
        // after a selbri into this group.
        assert (
            opt(selmaho(Na)),
            opt(selmaho(Se)),
            selmaho(Ja),
            opt(cmavo(Nai)),
            opt(tense_modal),
            cmavo(Ke),
        ).lookahead();
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// The JA word, which carries the warning for the whole construct.
        field ja <- selmaho(Ja).warn(ExperimentalJaKeTanruConnective).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional tag between JA and KE.
        field tense_modal <- opt(arc(tense_modal));
        /// The KE group opener.
        field ke <- cmavo(Ke).wf();
        /// The level-3 group body.
        field inner_selbri <- arc(tanru_selbri);
        /// The optional KEhE group terminator.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for the joik-only tagged KE arm at selbri level 4.
    rule "grouped selbri connection continuation" grouped_connected_selbri_continuation(tanru_selbri, tense_modal) -> struct {
        /// The JOI-family connective; JEK is deliberately excluded.
        field connective <- arc(joik_connective);
        /// The optional tag between JOIK and KE.
        field tense_modal <- opt(arc(tense_modal));
        /// The KE group opener.
        field ke <- cmavo(Ke).wf();
        /// The level-3 group body.
        field inner_selbri <- arc(tanru_selbri);
        /// The optional KEhE group terminator.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for selbri level 5; a jek/joik plus optional tag and BO is
    /// required before the recursive right operand.
    rule "BO-bound selbri" bound_selbri(bound_selbri, plain_bo_selbri, tense_modal, free_modifier) -> struct {
        /// The leading level-6 selbri.
        field leading_selbri <- arc(plain_bo_selbri);
        /// The optional, necessarily connective-bearing BO continuation.
        field bo_tail <- opt(arc(bound_selbri_tail(bound_selbri, tense_modal)));
    }

    /// Product node for a level-5 connective BO continuation.
    rule "BO-bound selbri continuation" bound_selbri_tail(bound_selbri, tense_modal) -> struct {
        /// The required standard joik/jek connective.
        field connective <- arc(selbri_afterthought_connective);
        /// The optional tag between the connective and BO.
        field tense_modal <- opt(arc(tense_modal));
        /// The BO marker.
        field bo <- cmavo(Bo).wf();
        /// The right-recursive level-5 operand.
        field trailing_selbri <- arc(bound_selbri);
    }

    /// Sum node for selbri level 6.
    rule "plain BO selbri" plain_bo_selbri(plain_bo_selbri, tanru_unit, selbri, co_selbri, free_modifier, exp_selbri_relative_clauses) -> enum {
        /// A CEI-capable tanru unit carrying camxes-exp's tanru-unit relative clauses.
        exp_relative_tanru_unit,
        /// A CEI-capable tanru unit with an optional plain BO continuation.
        plain_bo_tanru_unit,
        /// The standard binary forethought owner.
        forethought_selbri_connection,
    }

    /// Product node for a CEI-capable unit carrying camxes-exp's relative clauses.
    ///
    /// `tanru_unit <- tanru_unit_1 (CEI free* tanru_unit_1)* selbri_relative_clauses?`
    /// (camxes-exp.peg:241) puts the chain after the CEI chain, inside the BO level.  It is a
    /// separate arm rather than an optional field on `tanru_unit` so that an ordinary tanru
    /// unit -- which is nearly every node in the corpus -- keeps the shape it has; the arm is
    /// structurally disjoint from `plain_bo_tanru_unit` because it requires the chain, and it
    /// runs first so a present chain is not left behind by the shorter arm.
    rule "tanru unit" exp_relative_tanru_unit(plain_bo_selbri, tanru_unit, exp_selbri_relative_clauses) -> struct {
        /// The leading complete tanru unit, including any CEI assignments.
        field leading_unit <- arc(tanru_unit);
        /// The required relative-clause chain.
        field relative_clauses <- arc(exp_selbri_relative_clauses);
        /// The optional connectorless BO continuation.
        field bo_tail <- opt(arc(plain_bo_selbri_tail(plain_bo_selbri)));
        // The unit's mixed-list reservation.
        //
        // camxes-exp's chain is greedy -- `selbri_relative_clause ((ZIhE_clause / joik) free*
        // selbri_relative_clause)*` (camxes-exp.peg:214) -- so a `(ZIhE_clause / joik)` still
        // standing in front of a relative marker after this unit ends is a clause this route
        // cannot form, and the list as a whole is therefore not an extent camxes-exp derives at
        // all. Without the reservation this arm takes the FIRST clause into the leading selbri
        // and the whole list is lost to every owner: nothing in any of the three grammars
        // consumes a leading `zi'e`, or a `je` before a relative marker, so the enclosing site
        // never gets a list to own. `broda po'oi mi brode zi'e poi do brodi` and its joik-joined
        // twin are the shapes (issue #877); with the reservation the leading selbri stays bare
        // and the completed mixed list reaches the selbri-level parent, which is the same
        // reading an explicit `ku'o` on clause 1 already produced through the KUhO reservation.
        //
        // It is the prefix-steal `exp_selbri_relative_clause`'s KUhO reservation prevents, at
        // the other end of the chain, and like that one it is a boolean probed inside a
        // rewinding lookahead. The connective inventory is spelled from tokens -- `joik` as
        // :347-349 spells it, plus `ZIhE_clause`, longest alternative first -- rather than by
        // reusing `exp_selbri_relative_clause_connective`: the probe must never contribute a
        // node or the connective's own experimental warning.
        //
        // Spelled from tokens, but with the free-modifier placements the four connective rules
        // it stands for actually carry: a reservation that is not the SAME LANGUAGE as the
        // thing it reserves fails at exactly the boundary where they differ, and the
        // prefix-steal happens there. Each `.wf()`
        // below is one an owning rule carries -- `cmavo(Zihe).wf()` in both
        // `zihe_selbri_relative_connective` and the sumti site's `joined_relative_clause_tail`,
        // the closing `selmaho(Gaho).wf()` of `closed_interval_connective`, and the
        // `head`/`nai` pair of `simple_interval_connective` and of the
        // `exp_relative_clause_connective` that the sumti site's `exp_relative_continuation`
        // uses for camxes-exp's `joik` at :199. Between them they cover both placements a
        // stranded connective can present: camxes-exp's own `(ZIhE_clause / joik) free*`
        // (:214), which lands after the completed connective, and the head-before-NAI slot the
        // shared connective nodes carry, which is unsourced (#847) but which the epoch base
        // already reaches at the enclosing site and which this epoch may therefore not
        // withdraw. Without them `broda po'oi mi brode zi'e to do brodi toi pe mi` and every
        // other measured member of its class -- both connective families, GOI and NOI markers,
        // both sites, one interposed free or several, TO/TOI, SEI and vocative flavours, with
        // and without a NAI behind the head -- are base-A/head-R: the probe fails, the arm
        // takes clause one, and the enclosing list whose own connective would have consumed
        // those frees never gets a list to own. The measured table is in the epoch ledger's
        // round-5 section.
        //
        // Refusing the head-before-NAI placement remains a separate job, done by
        // `ProhibitedRelativeConnectiveFreeModifierRejection` on the chain's own completed
        // continuations, where it decides what this epoch's new route may PRODUCE. This probe
        // produces nothing, so recognising the placement here neither loosens that rejection
        // nor emits the free modifiers' own diagnostics.
        //
        // The marker inventory is the whole atom inventory of the sites that own the stranded
        // list, which is NOI *and* GOI: `relative_clause_atom` is
        // `sumti_association_relative_clause` (`selmaho(Goi)`) or `bridi_relative_clause`,
        // whose two arms spell camxes-standard's `poi`/`voi` and `noi`.
        // A GOI continuation strands the connective exactly as a NOI one does --
        // `lo broda po'oi mi brode zi'e pe mi ku cu brodi` is the default-profile shape -- and
        // no `selbri_relative_clause` can begin with GOI either, since exp's marker there is
        // `NOhOI` alone (:1907), so the by-construction argument is the same for both classes.
        //
        // The marker choice stays bare. The probe ends there, so a `free*` after it could not
        // change the boolean, and probing one at end of input moves the recorded failure
        // frontier onto the probe.
        //
        // It is a TRAILING assertion rather than a `followed_by` on `bo_tail` because the
        // probe must observe the position after everything the unit consumed -- chain and BO
        // tail alike -- without making any field opaque to recovery metadata: wrapping the
        // field cost this rule all three of its `Cmavo(Bo)` resume anchors.
        assert !(
            choice((
                cmavo(Zihe).wf().ignored(),
                (
                    selmaho(Gaho),
                    opt(selmaho(Se)),
                    selmaho(Bihi),
                    opt(cmavo(Nai)),
                    selmaho(Gaho).wf(),
                )
                    .ignored(),
                (opt(selmaho(Se)), selmaho(Bihi).wf(), opt(cmavo(Nai).wf())).ignored(),
                (
                    opt(selmaho(Na)),
                    opt(selmaho(Se)),
                    choice((selmaho(Joi), selmaho(Ja), selmaho(A))).wf(),
                    opt(cmavo(Nai).wf()),
                )
                    .ignored(),
            )),
            choice((
                selmaho(Goi),
                cmavo(Poi),
                cmavo(Voi),
                cmavo(Noi),
            )),
        );
    }

    /// Product node for a CEI-capable unit with an optional plain BO tail.
    rule "plain BO tanru unit" plain_bo_tanru_unit(plain_bo_selbri, tanru_unit) -> struct {
        /// The leading complete tanru unit, including any CEI assignments.
        field leading_unit <- arc(tanru_unit);
        /// The optional connectorless BO continuation.
        field bo_tail <- opt(arc(plain_bo_selbri_tail(plain_bo_selbri)));
    }

    /// Product node for a connectorless level-6 BO continuation.
    rule "plain BO selbri continuation" plain_bo_selbri_tail(plain_bo_selbri) -> struct {
        /// The BO marker.
        field bo <- cmavo(Bo).wf();
        /// The right-recursive level-6 operand.
        field trailing_selbri <- arc(plain_bo_selbri);
    }

    /// Sum node for the forethought selbri connection. It has one arm only; the sum stays so
    /// that the trees keep their shape.
    rule "forethought selbri connection" forethought_selbri_connection(selbri, plain_bo_selbri, co_selbri, free_modifier) -> enum {
        /// The standard binary L6 owner.
        standard_forethought_selbri_connection,
    }

    /// Product node for the standard binary forethought selbri owner at L6.
    rule "forethought selbri connection" standard_forethought_selbri_connection(selbri, plain_bo_selbri, free_modifier) -> struct {
        /// Optional NAhE preceding the independent free-modifier slot.
        field nahe <- opt(selmaho(Nahe));
        /// Free modifiers between NAhE (when present) and GUhA.
        field free_modifiers <- [zero_or_more free_modifier];
        /// The forethought connective opener without NAhE.
        field guhek <- guhek_connective;
        /// The full left selbri operand.
        field leading_selbri <- arc(selbri);
        /// The single tight L6 GI branch.
        field first_branch <- forethought_selbri_branch(plain_bo_selbri);
    }

    /// Product node for the standard GI branch of a forethought selbri.
    rule "forethought selbri connection" forethought_selbri_branch(plain_bo_selbri) -> struct {
        /// The standard GI-family connective.
        field gik <- gik_connective;
        /// The tight level-6 branch selbri.
        field selbri <- arc(plain_bo_selbri);
    }

    /// Product node for a complete tanru unit: an atom with optional linkargs,
    /// followed by zero or more CEI assignments.
    rule "tanru unit" tanru_unit(tanru_unit_atom, linkargs) -> struct {
        /// The first linked atom.
        field base <- arc(linked_tanru_unit(tanru_unit_atom, linkargs));
        /// Source-ordered CEI assignments.
        field assignments <- [zero_or_more pro_bridi_tanru_unit_assignment(tanru_unit_atom, linkargs)];
    }

    /// Product node for one CEI assignment.
    rule "pro-bridi assignment" pro_bridi_tanru_unit_assignment(tanru_unit_atom, linkargs) -> struct {
        /// The CEI marker.
        field cei <- cmavo(Cei).wf();
        /// The following linked atom.
        field tanru_unit <- arc(linked_tanru_unit(tanru_unit_atom, linkargs));
    }

    /// Product node for tanru unit; preserves `base` and `linkargs` in source order.
    rule "tanru unit" linked_tanru_unit(tanru_unit_atom, linkargs) -> struct {
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom);
        /// The optional linkargs component.
        field linkargs <- opt(linkargs);
    }


    /// Product node for tanru unit; preserves `conversions` and `base` in source order.
    rule "tanru unit" tanru_unit_atom(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, mekso, mekso_base, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term, linkargs) -> struct {
        /// Ordered sequence of zero or more conversions components.
        field conversions <- [zero_or_more selmaho(Se).wf()];
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom_base(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, mekso, mekso_base, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term, linkargs));
    }

    /// Sum node for tanru unit; selects among the standard and experimental forms.
    rule "tanru unit" tanru_unit_atom_base(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, mekso, mekso_base, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term, linkargs) -> enum {
        /// Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.
        ordinal_tanru_unit,
        /// Uses the `word_tanru_unit` product form, whose payload preserves `word`.
        word_tanru_unit,
        /// Uses the `preposed_linkargs_tanru_unit` product form, whose payload preserves `linkargs` and `base`.
        preposed_linkargs_tanru_unit,
        /// Uses the `jai_modal_tanru_unit` product form, whose payload preserves `jai`, `tense_modal`, and `inner_unit`.
        jai_modal_tanru_unit,
        /// Uses the `scalar_negated_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.
        scalar_negated_tanru_unit,
        /// Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.
        abstraction_tanru_unit,
        /// Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.
        sumti_selbri_tanru_unit,
        /// Uses camxes-exp's `exp_mekso_selbri_tanru_unit` form, whose payload preserves `me`, `mekso`, `mehu`, and `moi_marker`.
        exp_mekso_selbri_tanru_unit,
        /// Uses camxes-exp's `exp_mekso_moi_tanru_unit` form, whose payload preserves `mekso` and `moi`.
        exp_mekso_moi_tanru_unit,
        /// Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.
        operator_selbri_tanru_unit,
        /// A completed one-word MEhOI quote is a direct atom, never a quoted sumti.
        mehoi_tanru_unit,
        /// Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.
        goha_word_tanru_unit,
        /// Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.
        pro_bridi_tanru_unit,
        /// Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.
        grouped_tanru_unit,
    }

    /// Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.
    rule "tagged selbri" tagged_selbri_group_tanru_unit(connected_selbri, tense_modal) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(connected_selbri);
    }

    /// Product node for linked arguments; preserves `linkargs` and `base` in source order.
    rule "linked arguments" preposed_linkargs_tanru_unit(tanru_unit_atom, linkargs) -> struct {
        /// The complete exp-sourced linkargs; the strict construct visitor warns at its BE.
        field linkargs <- linkargs;
        /// The following linked atom; CEI assignments remain at the outer tanru-unit level.
        field base <- arc(linked_tanru_unit(tanru_unit_atom, linkargs));
    }

    /// Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.
    rule "scalar-negated tanru unit" scalar_negated_tanru_unit(tanru_unit_atom, normal_term) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(scalar_negated_tanru_inner_unit(tanru_unit_atom, normal_term));
    }

    /// The standard scalar-negation operand, restricted to exactly one tanru-unit atom.
    rule "scalar-negated tanru unit" scalar_negated_tanru_inner_unit(tanru_unit_atom, normal_term) -> enum {
        /// Uses the `tanru_unit_atom` product form, whose payload preserves `conversions` and `base`.
        tanru_unit_atom,
    }

    /// Product node for modal conversion; preserves `jai`, `tense_modal`, and `inner_unit` in source order.
    rule "modal conversion" jai_modal_tanru_unit(tanru_unit_atom, tense_modal) -> struct {
        /// The `Jai` cmavo marker.
        field jai <- cmavo(Jai).wf();
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The converted tanru-unit atom.
        field inner_unit <- arc(tanru_unit_atom);
    }


    /// Direct stage-0 fu'ivla atom. Morphology already owns the one-word payload;
    /// syntax must not inspect its spelling or treat it as delimited text.
    rule "tanru unit" mehoi_tanru_unit -> struct {
        /// The completed MEhOI token, with its selbri-unit warning and free modifiers.
        field quote <- quote_marker(Mehoi).warn(ExperimentalMehOiSelbriUnit).wf();
    }

    /// Product node for ordinal selbri; preserves `number` and `moi` in source order.
    rule "ordinal selbri" ordinal_tanru_unit(letter_tokens, letter_string) -> struct {
        /// The `number_or_letter_words` grammar result in the `number` structural role of the `ordinal_tanru_unit` production.
        field number <- number_or_letter_words(letter_tokens, letter_string);
        /// A word from selmaho `Moi`.
        field moi <- selmaho(Moi).wf();
    }

    /// Transparent product node for tanru unit; preserves the `word` component.
    rule "tanru unit" word_tanru_unit -> struct {
        /// The `tanru_unit_relation_word` grammar result in the `word` structural role of the `word_tanru_unit` production.
        field word <- tanru_unit_relation_word().wf();
    }

    /// Transparent product node for tanru unit; preserves the `word` component.
    rule "tanru unit" goha_word_tanru_unit(free_modifier) -> struct {
        /// A word from selmaho `Goha`.
        field word <- selmaho(Goha)
            .followed_by(choice((
                cmavo(Raho).ignored(),
                cmavo(Be).ignored(),
                pa_word().ignored(),
                free_modifier.ignored(),
            )).not())
            .wf();
    }

    /// Product node for pro-bridi; preserves `goha` and `raho` in source order.
    rule "pro-bridi" pro_bridi_tanru_unit -> struct {
        /// A word from selmaho `Goha`.
        field goha <- selmaho(Goha).wf();
        /// The optional `Raho` cmavo marker.
        field raho <- opt(cmavo(Raho).wf());
    }

    /// camxes-exp's `ME_clause free* (sumti / mex) MEhU_elidible free* MOI_clause? free*`
    /// (camxes-exp.peg:248), mex half. camxes-exp tries the sumti first, and so does jbotci:
    /// `sumti_selbri_tanru_unit` comes earlier in the tanru-unit choice, so a ME body that a
    /// sumti covers stays the baseline sumti-to-selbri.
    ///
    /// The body is jbotci's own mex. Like CLL and unlike camxes-exp, it keeps the PEhO-less
    /// forethought call; that is the retained standard MEX design (I12 in the camxes-exp
    /// reconciliation).
    rule "sumti-to-selbri" exp_mekso_selbri_tanru_unit(mekso) -> struct {
        /// The `Me` cmavo marker, which carries the warning for the whole construct.
        field me <- cmavo(Me).warn(ExperimentalMexMeSelbriUnit).wf();
        #[tree_child(primary)]
        /// The mex body.
        field mekso <- arc(mekso);
        /// The optional `Mehu` cmavo marker.
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        /// The optional moi marker component.
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    /// camxes-exp's `mex MOI_clause free*` (camxes-exp.peg:248). The baseline
    /// `ordinal_tanru_unit` comes first in the tanru-unit choice, so a lone number or lerfu
    /// string before MOI stays an ordinal; this arm takes the mex that the ordinal cannot. As in
    /// the ME arm, the mex is jbotci's own.
    ///
    /// A number operand refuses a following MOI (#813), so a mex that ends in a number before
    /// MOI (`pa su'i re moi`) does not reach this arm.
    rule "mex selbri" exp_mekso_moi_tanru_unit(mekso) -> struct {
        #[tree_child(primary)]
        /// The mex before MOI. The mex must end right before the MOI word; a mex that does not is
        /// rewound with its diagnostics, so trying this arm leaves no trace where it fails.
        field mekso: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_before_selmaho(Moi));
        /// The MOI word, which carries the warning for the whole construct.
        field moi <- selmaho(Moi).warn(ExperimentalMexMoiSelbriUnit).wf();
    }

    /// Product node for sumti-to-selbri; preserves `me`, `sumti`, `mehu`, and `moi_marker` in source order.
    rule "sumti-to-selbri" sumti_selbri_tanru_unit(sumti, letter_string, normal_term) -> struct {
        /// The `Me` cmavo marker.
        field me <- cmavo(Me).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_selbri_sumti(sumti, letter_string, normal_term));
        /// The optional `Mehu` cmavo marker.
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        /// The optional moi marker component.
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    /// Sum node for sumti selbri; selects among the `sumti` and `me_lerfu_sumti` forms.
    rule "sumti selbri" sumti_selbri_sumti(sumti, letter_string, normal_term) -> enum {
        /// Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.
        sumti,
        /// Uses the `me_lerfu_sumti` product form, whose payload preserves `words`.
        me_lerfu_sumti,
    }

    /// Transparent product node for lerfu string; preserves the `words` component.
    rule "lerfu string" me_lerfu_sumti(letter_string) -> struct {
        /// The `letter_string` grammar result in the `words` structural role of the `me_lerfu_sumti` production.
        field words <- letter_string;
    }

    /// Product node for operator-to-selbri; preserves `nuha` and `mekso_operator` in source order.
    rule "operator-to-selbri" operator_selbri_tanru_unit(atomic_mekso_operator) -> struct {
        /// The `Nuha` cmavo marker.
        field nuha <- cmavo(Nuha).wf();
        /// The atomic mekso operator child syntax node.
        field mekso_operator <- arc(atomic_mekso_operator);
    }

    /// Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.
    rule "grouped tanru" grouped_tanru_unit(tanru_selbri) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(tanru_selbri);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for the three nonempty legacy linked-sumti forms.
    rule "linked arguments" linked_sumti(sumti, tense_modal, normal_term) -> enum {
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
    }

    /// The loose connection level for BE/BEI arguments in the camxes-exp term hierarchy.
    ///
    /// The lower levels and the `linked_sumti` leaves come from splices, so ordinary links keep
    /// their established Debug and serde shape, with no wrapper variant for a lower level.
    rule "linked arguments" linked_term(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term, bound_linked_term, bound_linked_term_operand, full_linked_term_candidate) -> enum {
        /// Try the complete new-width payload before a legacy owner can consume its prefix.
        /// The rejection guard rewinds complete legacy and unproven candidates (#793).
        full_linked_term_candidate,
        /// Uses the diagnosed loose connection over BO-bound linked terms.
        connected_linked_term,
        // The BO-bound level and the linked-sumti leaves below it.
        splice bound_linked_term,
    }

    /// A complete normal-term payload, with no additional warning or copied leaf inventory.
    rule "linked arguments" full_linked_term(normal_term) -> struct {
        /// The full payload of one BE or BEI, in its original term hierarchy.
        field term <- arc(normal_term);
    }

    // The rule-level rejection is also applied during recovery. Keeping this alias recursive
    // gives it one identity without adding a public wrapper around FullLinkedTermSyntax.
    alias "linked arguments" full_linked_term_candidate(normal_term) =
        full_linked_term(normal_term)
            .reject_output(crate::grammar::link_payload::LegacyLinkPayloadRejection)
            .recursive_output(full_linked_term_candidate);

    /// A hierarchy-only loose connection over linked terms with one or more continuations.
    rule "linked arguments" connected_linked_term(tense_modal, selbri, forethought_bridi_connection, bound_linked_term) -> struct {
        /// The first BO-bound linked term at the loose precedence level.
        field leading_link <- arc(bound_linked_term);
        /// The nonempty source-ordered loose continuation sequence.
        field continuations <- [one_or_more connected_linked_term_continuation(tense_modal, selbri, forethought_bridi_connection, bound_linked_term)];
    }

    /// One loose linked-term continuation.
    rule "linked arguments" connected_linked_term_continuation(tense_modal, selbri, forethought_bridi_connection, bound_linked_term) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        /// The connective joining the adjacent linked terms.
        field connective <- term_afterthought_connective;
        /// The BO-bound linked term following the connective.
        field trailing_link <- arc(bound_linked_term);
    }

    /// The optional-stag BO-bound level for BE/BEI arguments.
    rule "linked arguments" bound_linked_term(sumti, tense_modal, normal_term, bound_linked_term_operand) -> enum {
        /// Uses the diagnosed BO-bound linked-term connection.
        bound_linked_term_connection,
        // The nonempty linked-sumti leaves.
        splice bound_linked_term_operand,
    }

    /// A nonempty linked-term operand; the empty BE/BEI marker form is intentionally excluded.
    rule "linked arguments" bound_linked_term_operand(sumti, tense_modal, normal_term) -> enum {
        // The three nonempty linked-sumti forms.
        splice linked_sumti,
    }

    /// The diagnosed BO-bound BE/BEI connection with one or more continuations.
    rule "linked arguments" bound_linked_term_connection(tense_modal, bound_linked_term_operand) -> struct {
        /// The first nonempty linked argument at the BO-bound precedence level.
        field leading_link <- arc(bound_linked_term_operand);
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more bound_linked_term_continuation(tense_modal, bound_linked_term_operand)];
    }

    /// One optional-stag BO continuation in a BE/BEI argument connection.
    rule "linked arguments" bound_linked_term_continuation(tense_modal, bound_linked_term_operand) -> struct {
        /// The connective joining the adjacent linked arguments.
        field connective <- term_afterthought_connective;
        /// The optional camxes-exp `stag`; unlike ordinary terms, links use the `term` flavor.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker, which owns the experimental warning for the whole connection.
        field bo <- cmavo(Bo).warn(ExperimentalTermBoConnection).wf();
        /// The nonempty linked argument following BO.
        field trailing_link <- arc(bound_linked_term_operand);
    }

    /// Product node for linked arguments; preserves `fa` and `sumti` in source order.
    rule "linked arguments" place_tagged_linked_sumti(sumti, normal_term) -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Product node for linked arguments; preserves `tense_modal` and `sumti` in source order.
    rule "linked arguments" tense_tagged_linked_sumti(sumti, tense_modal, normal_term) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti, normal_term));
    }

    /// Transparent product node for linked arguments; preserves the `sumti` component.
    rule "linked arguments" plain_linked_sumti(sumti) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Product node for linked arguments; preserves `bei` and `link` in source order.
    rule "linked arguments" bei_link(linked_term) -> struct {
        /// The `Bei` cmavo marker.
        field bei <- cmavo(Bei).wf();
        /// The `linked_term` grammar result in the `link` structural role of the `bei_link` production.
        field link <- linked_term;
    }

    /// Product node for linked arguments; preserves `be`, `first_link`, `bei_links`, and `beho` in source order.
    rule "linked arguments" linkargs(linked_term) -> struct {
        /// The `Be` cmavo marker.
        field be <- cmavo(Be).wf();
        /// The initial `linked_term` constituent before the continuations of the `linkargs` production.
        field first_link <- linked_term;
        /// Ordered sequence of zero or more bei links components.
        field bei_links <- [zero_or_more bei_link(linked_term)];
        /// The optional `Beho` cmavo marker.
        field beho <- opt(cmavo(Beho).wf()).elidable_terminator(Beho);
    }

    /// Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei` in source order.
    rule "abstraction" abstraction_tanru_unit(subbridi) -> struct {
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// Ordered sequence of zero or more abstractor connections components.
        field abstractor_connections <- [zero_or_more abstractor_connection()];
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kei` cmavo marker.
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

    /// Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.
    rule "abstractor connection" abstractor_connection -> struct {
        /// The `standard_statement_connective` connective joining the adjacent constituents of the `abstractor_connection` production.
        field connective <- standard_statement_connective;
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    }

    /// Compatibility name for the now-unified tanru-unit atom used on both
    /// sides of CEI. The standard grammar has one tanru-unit-1 domain, so the
    /// epoch-5 model deliberately uses the same validated type throughout.
    pub type TanruUnitAtomForCeiSyntax = TanruUnitAtomSyntax;

    /// Compatibility name for the unified tanru-unit atom sum used after CEI.
    pub type TanruUnitAtomBaseForCeiSyntax = TanruUnitAtomBaseSyntax;

    /// Compatibility name for the unified linked tanru unit used after CEI.
    pub type LinkedTanruUnitForCeiSyntax = LinkedTanruUnitSyntax;

    /// Compatibility name for the former CEI-only wrapper. CEI assignments
    /// now live on every `TanruUnitSyntax`, matching camxes tanru-unit.
    pub type AssignedProBridiTanruUnitSyntax = TanruUnitSyntax;

    #[bityzba::invariant(true)]
    struct FirstGeneratedTokenVisitor<'tree> {
        first: Option<&'tree Token>,
    }

    impl<'tree> jbotci_tree::TreeVisitor<'tree> for FirstGeneratedTokenVisitor<'tree> {
        type Node = NodeRef<'tree>;
        type Atom = AtomRef<'tree>;

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn visit_atom(&mut self, atom: Self::Atom) {
            if self.first.is_some() {
                return;
            }
            let AtomRef::Token(token) = atom;
            self.first = Some(token);
        }
    }

    #[bityzba::contract_trait]
    impl generated_runtime::SyntaxFirstWord for FreeModifierSyntax {
        fn first_word(&self) -> Option<&Token> {
            let mut visitor = FirstGeneratedTokenVisitor { first: None };
            self.visit_in_order(&mut visitor);
            visitor.first
        }
    }

    #[bityzba::invariant(true)]
    struct FirstRecoveredGeneratedTokenVisitor<'tree> {
        first: Option<&'tree Token>,
    }

    impl<'tree> jbotci_tree::TreeVisitor<'tree> for FirstRecoveredGeneratedTokenVisitor<'tree> {
        type Node = recovered::NodeRef<'tree>;
        type Atom = recovered::AtomRef<'tree>;

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        fn visit_atom(&mut self, atom: Self::Atom) {
            if self.first.is_some() {
                return;
            }
            let recovered::AtomRef::Token(token) = atom;
            self.first = Some(token);
        }
    }

    #[bityzba::contract_trait]
    impl generated_runtime::SyntaxFirstWord for recovered::FreeModifierSyntax {
        fn first_word(&self) -> Option<&Token> {
            let mut visitor = FirstRecoveredGeneratedTokenVisitor { first: None };
            recovered::TreeNode::visit_in_order(self, &mut visitor);
            visitor.first
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub fn parse_text(
        words: &[Token],
        options: &ParseOptions,
    ) -> Result<TextSyntax, crate::SyntaxError> {
        parse_text_attempt(words, options)
            .result
            .map(|parsed| parsed.text)
    }

    #[bityzba::invariant(true)]
    pub(crate) struct GeneratedParsedText {
        pub text: TextSyntax,
        pub warnings: Vec<SyntaxWarning>,
    }

    #[bityzba::invariant(true)]
    pub(crate) struct GeneratedParsedTextAttempt {
        pub result: Result<GeneratedParsedText, crate::SyntaxError>,
        pub trace: Option<TraceReport>,
    }

    /// A failed parse, with what recovery needs: the furthest failure position and the rule
    /// frames that were active there, each with its smallest distance from the innermost
    /// frame of a failure at that position.
    #[bityzba::invariant(frame_ranks.is_empty() || furthest_start.is_some())]
    #[derive(Debug, Clone)]
    pub(crate) struct GeneratedParseFailure {
        pub public_error: crate::SyntaxError,
        pub furthest_start: Option<usize>,
        pub frame_ranks: Vec<RecoveryFrameRank>,
        pub checkpoints: RecoveryCheckpointIndex,
    }

    #[bityzba::invariant(continuation_expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
    pub(crate) struct GeneratedParsedTextDetailedAttempt {
        pub result: Result<GeneratedParsedText, GeneratedParseFailure>,
        pub trace: Option<TraceReport>,
        pub continuation_expectations: Vec<crate::SyntaxExpectation>,
    }

    #[bityzba::invariant(true)]
    pub(crate) struct GeneratedRecoveredParsedText {
        pub text: generated_runtime::SharedSyntaxOutput<recovered::TextSyntax>,
        pub warnings: Vec<SyntaxWarning>,
    }

    #[bityzba::invariant(continuation_expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
    pub(crate) struct GeneratedRecoveredParsedTextAttempt {
        pub result: Result<GeneratedRecoveredParsedText, GeneratedParseFailure>,
        pub trace: Option<TraceReport>,
        pub unconsumed_directives: usize,
        pub recovery_directives: Vec<RecoveryDirective>,
        pub effective_fail_token_indices: Vec<usize>,
        pub completed_recovery_boundary_location: Option<usize>,
        pub continuation_expectations: Vec<crate::SyntaxExpectation>,
    }

    #[bityzba::invariant(true)]
    pub(in crate::grammar) struct GeneratedRecoveryParseSession<'tokens> {
        memo_session: SyntaxRecoveryMemoSession<'tokens>,
        parser: BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<recovered::TextSyntax>>,
        continuation_sentinel_index: Option<usize>,
        continuation_time_limit: Option<ContinuationTimeLimit>,
    }

    impl<'tokens> GeneratedRecoveryParseSession<'tokens> {
        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        pub(in crate::grammar) fn new() -> Self {
            Self {
                memo_session: SyntaxRecoveryMemoSession::new(),
                parser: recovered_generated_entry_parser_with_eof(SyntaxParseEntry::Text),
                continuation_sentinel_index: None,
                continuation_time_limit: None,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.continuation_time_limit == continuation_time_limit)]
        pub(in crate::grammar) fn new_with_continuation_time_limit(
            entry: SyntaxParseEntry,
            continuation_time_limit: Option<ContinuationTimeLimit>,
        ) -> Self {
            Self {
                memo_session: SyntaxRecoveryMemoSession::new(),
                parser: recovered_generated_entry_parser_with_eof(entry),
                continuation_sentinel_index: None,
                continuation_time_limit,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.continuation_sentinel_index == Some(sentinel_index))]
        #[bityzba::ensures(ret.continuation_time_limit == continuation_time_limit)]
        pub(in crate::grammar) fn new_for_expected_continuations(
            entry: SyntaxParseEntry,
            sentinel_index: usize,
            continuation_time_limit: Option<ContinuationTimeLimit>,
        ) -> Self {
            Self {
                memo_session: SyntaxRecoveryMemoSession::new(),
                parser: recovered_generated_entry_parser_with_eof(entry),
                continuation_sentinel_index: Some(sentinel_index),
                continuation_time_limit,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(true)]
        pub(in crate::grammar) fn clear_memo(&mut self) {
            self.memo_session.clear();
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub(crate) fn parse_text_attempt(
        words: &[Token],
        options: &ParseOptions,
    ) -> GeneratedParsedTextAttempt {
        parse_entry_attempt(SyntaxParseEntry::Text, words, options)
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub(crate) fn parse_entry_attempt(
        entry: SyntaxParseEntry,
        words: &[Token],
        options: &ParseOptions,
    ) -> GeneratedParsedTextAttempt {
        let tokens = spanned_tokens(words);
        let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
        let mut state = ParserState::new(words, options);
        let result = strict_generated_entry_parser_with_eof(entry)
            .parse_with_state(
                tokens
                    .as_slice()
                    .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
                &mut state,
            )
            .into_result();
        let diagnostic_candidate = result.as_ref().err().map(|_| state.diagnostic_candidate());
        let finish = state.finish();
        let result = match result {
            Ok(text) => Ok(GeneratedParsedText {
                text: text.into_owned(),
                warnings: finish.warnings,
            }),
            Err(errors) => {
                let public_error = syntax_error_with_diagnostic_candidate(
                    errors.clone(),
                    diagnostic_candidate.expect("failure context captured for syntax errors"),
                    options.error_context_depth,
                );
                Err(public_error)
            }
        };
        GeneratedParsedTextAttempt {
            result,
            trace: finish.trace,
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub(crate) fn parse_text_detailed_attempt(
        words: &[Token],
        options: &ParseOptions,
    ) -> GeneratedParsedTextDetailedAttempt {
        let strict_attempt = parse_text_attempt(words, options);
        if let Ok(parsed) = strict_attempt.result {
            return bityzba::new!(GeneratedParsedTextDetailedAttempt {
                result: Ok(parsed),
                trace: strict_attempt.trace,
                continuation_expectations: Vec::new(),
            });
        }

        parse_text_detailed_tracked_attempt(words, options)
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub(crate) fn parse_text_detailed_tracked_attempt(
        words: &[Token],
        options: &ParseOptions,
    ) -> GeneratedParsedTextDetailedAttempt {
        parse_entry_detailed_tracked_attempt(SyntaxParseEntry::Text, words, options)
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub(crate) fn parse_entry_detailed_tracked_attempt(
        entry: SyntaxParseEntry,
        words: &[Token],
        options: &ParseOptions,
    ) -> GeneratedParsedTextDetailedAttempt {
        parse_text_detailed_tracked_attempt_inner(entry, words, options, None, None)
    }

    #[bityzba::requires(sentinel_index < words.len())]
    #[bityzba::ensures(ret.result.is_err())]
    pub(in crate::grammar) fn parse_text_detailed_tracked_attempt_for_expected_continuations(
        words: &[Token],
        options: &ParseOptions,
        sentinel_index: usize,
        continuation_time_limit: Option<ContinuationTimeLimit>,
    ) -> GeneratedParsedTextDetailedAttempt {
        parse_text_detailed_tracked_attempt_inner(
            SyntaxParseEntry::Text,
            words,
            options,
            Some(sentinel_index),
            continuation_time_limit,
        )
    }

    #[bityzba::requires(continuation_sentinel_index.is_none_or(|index| index < words.len()))]
    #[bityzba::ensures(continuation_sentinel_index.is_some() -> ret.result.is_err())]
    fn parse_text_detailed_tracked_attempt_inner(
        entry: SyntaxParseEntry,
        words: &[Token],
        options: &ParseOptions,
        continuation_sentinel_index: Option<usize>,
        continuation_time_limit: Option<ContinuationTimeLimit>,
    ) -> GeneratedParsedTextDetailedAttempt {
        let tokens = spanned_tokens(words);
        let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
        let mut state = if let Some(sentinel_index) = continuation_sentinel_index {
            ParserState::new_for_expected_continuations(
                words,
                options,
                sentinel_index,
                continuation_time_limit,
            )
        } else {
            ParserState::new_with_recovery_branches(words, options)
        };
        let result = recovery_checkpoint_strict_generated_entry_parser_with_eof(entry)
            .parse_with_state(
                tokens
                    .as_slice()
                    .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
                &mut state,
            )
            .into_result();
        let failure_context = result.as_ref().err().map(|_| {
            (
                state.diagnostic_candidate(),
                state.diagnostic_candidates_snapshot(),
                state.recovery_frame_ranks(),
            )
        });
        let continuation_expectations = if continuation_sentinel_index.is_some() {
            state.continuation_expectations()
        } else {
            Vec::new()
        };
        let finish = state.finish();
        let result = match result {
            Ok(text) => Ok(GeneratedParsedText {
                text: text.into_owned(),
                warnings: finish.warnings,
            }),
            Err(errors) => {
                let (diagnostic_candidate, diagnostic_candidates, candidate_frame_ranks) =
                    failure_context.expect("failure context captured for syntax errors");
                let (furthest_start, frame_ranks) = generated_recovery_frame_ranks(
                    &diagnostic_candidates,
                    candidate_frame_ranks,
                    &errors,
                );
                let public_error = syntax_error_with_diagnostic_candidate(
                    errors.clone(),
                    diagnostic_candidate,
                    options.error_context_depth,
                );
                Err(bityzba::new!(GeneratedParseFailure {
                    public_error,
                    furthest_start,
                    frame_ranks,
                    checkpoints: RecoveryCheckpointIndex::from_checkpoints(
                        finish.recovery_checkpoints,
                    ),
                }))
            }
        };
        bityzba::new!(GeneratedParsedTextDetailedAttempt {
            result,
            trace: finish.trace,
            continuation_expectations,
        })
    }

    #[bityzba::requires(!directives.is_empty())]
    #[bityzba::ensures(true)]
    #[bityzba::ensures(ret.effective_fail_token_indices.len() + ret.unconsumed_directives == ret.recovery_directives.len())]
    pub(crate) fn parse_recovered_text_attempt(
        words: &[Token],
        source: Option<&str>,
        options: &ParseOptions,
        directives: &[RecoveryDirective],
    ) -> GeneratedRecoveredParsedTextAttempt {
        let parser_tokens = spanned_tokens(words);
        let mut recovery_session = GeneratedRecoveryParseSession::new();
        parse_recovered_text_attempt_with_session(
            words,
            &parser_tokens,
            source,
            options,
            directives,
            &mut recovery_session,
        )
    }

    #[bityzba::requires(!directives.is_empty())]
    #[bityzba::ensures(true)]
    #[bityzba::ensures(ret.effective_fail_token_indices.len() + ret.unconsumed_directives == ret.recovery_directives.len())]
    pub(in crate::grammar) fn parse_recovered_text_attempt_with_session<'tokens>(
        words: &[Token],
        parser_tokens: &'tokens [SpannedToken],
        source: Option<&str>,
        options: &ParseOptions,
        directives: &[RecoveryDirective],
        recovery_session: &mut GeneratedRecoveryParseSession<'tokens>,
    ) -> GeneratedRecoveredParsedTextAttempt {
        let eoi_offset = parser_tokens.last().map_or(0, |token| token.span.end);
        let memo_trial = recovery_session.memo_session.begin_trial();
        let trial_id = memo_trial.trial_id.get();
        let mut state = ParserState::new_with_recovery(
            words,
            source,
            options,
            directives,
            memo_trial,
            recovery_session.continuation_sentinel_index,
            recovery_session.continuation_time_limit,
        );
        let parser = recovery_session.parser.clone();
        let result = parser
            .parse_with_state(
                parser_tokens.split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
                &mut state,
            )
            .into_result();
        let failure_context = result.as_ref().err().map(|_| {
            (
                state.diagnostic_candidate(),
                state.diagnostic_candidates_snapshot(),
                state.recovery_frame_ranks(),
            )
        });
        let continuation_expectations = if recovery_session.continuation_sentinel_index.is_some() {
            state.continuation_expectations()
        } else {
            Vec::new()
        };
        let finish = state.finish();
        recovery_session.memo_session.finish_trial(trial_id);
        let result = match result {
            Ok(text) => Ok(GeneratedRecoveredParsedText {
                text,
                warnings: finish.warnings,
            }),
            Err(errors) => {
                let (diagnostic_candidate, diagnostic_candidates, candidate_frame_ranks) =
                    failure_context.expect("failure context captured for syntax errors");
                let (furthest_start, frame_ranks) = generated_recovery_frame_ranks(
                    &diagnostic_candidates,
                    candidate_frame_ranks,
                    &errors,
                );
                let public_error = syntax_error_with_diagnostic_candidate(
                    errors.clone(),
                    diagnostic_candidate,
                    options.error_context_depth,
                );
                Err(bityzba::new!(GeneratedParseFailure {
                    public_error,
                    furthest_start,
                    frame_ranks,
                    checkpoints: RecoveryCheckpointIndex::from_checkpoints(
                        finish.recovery_checkpoints,
                    ),
                }))
            }
        };
        bityzba::new!(GeneratedRecoveredParsedTextAttempt {
            result,
            trace: finish.trace,
            unconsumed_directives: finish.unconsumed_recovery_directives,
            recovery_directives: finish.recovery_directives,
            effective_fail_token_indices: finish.effective_fail_token_indices,
            completed_recovery_boundary_location: finish.completed_recovery_boundary_location,
            continuation_expectations,
        })
    }

    /// The recovery frame ranks of a failed parse: those of the furthest diagnostic
    /// candidates, or, when the parse recorded no candidate, those of its furthest errors.
    #[bityzba::requires(true)]
    #[bityzba::ensures(ret.1.is_empty() || ret.0.is_some())]
    fn generated_recovery_frame_ranks(
        diagnostic_candidates: &[SyntaxParseError<'_>],
        candidate_frame_ranks: (Option<usize>, Vec<RecoveryFrameRank>),
        errors: &[SyntaxParseError<'_>],
    ) -> (Option<usize>, Vec<RecoveryFrameRank>) {
        if !diagnostic_candidates.is_empty() {
            debug_assert_eq!(
                candidate_frame_ranks.0,
                diagnostic_candidates
                    .first()
                    .map(|candidate| candidate.span().start),
                "the frame ranks and the candidates share their furthest position",
            );
            return candidate_frame_ranks;
        }
        let Some(furthest) = errors.iter().map(|error| error.span().start).max() else {
            return (None, Vec::new());
        };
        let mut ranks = std::collections::HashMap::<SyntaxRuleFrame, SyntaxFrameRank>::new();
        for (order, error) in errors
            .iter()
            .filter(|error| error.span().start == furthest)
            .enumerate()
        {
            for (rank, frame) in error.active_rule_frames_inner_to_outer().enumerate() {
                let rank = SyntaxFrameRank { rank, order };
                ranks
                    .entry(frame.clone())
                    .and_modify(|existing| *existing = (*existing).min(rank))
                    .or_insert(rank);
            }
        }
        let mut ranks = ranks
            .into_iter()
            .map(|(frame, rank)| RecoveryFrameRank { frame, rank })
            .collect::<Vec<_>>();
        ranks.sort_by(|left, right| {
            (left.frame.byte_start(), left.frame.rule(), left.rank).cmp(&(
                right.frame.byte_start(),
                right.frame.rule(),
                right.rank,
            ))
        });
        (Some(furthest), ranks)
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn strict_generated_entry_parser_with_eof<'tokens>(
        entry: SyntaxParseEntry,
    ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<TextSyntax>> {
        match entry {
            SyntaxParseEntry::Text => {
                let parser = strict_generated_text_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let text = input.parse(&parser)?;
                    input.parse(end()).map(|()| text)
                })
                .boxed()
            }
            SyntaxParseEntry::NihoParagraphs => {
                let parser = strict_generated_text_niho_paragraphs_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let paragraphs = input.parse(&parser)?;
                    input.parse(end()).map(|()| {
                        generated_runtime::SharedSyntaxOutput::new(text_of_niho_paragraphs(
                            paragraphs.into_owned(),
                        ))
                    })
                })
                .boxed()
            }
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn recovery_checkpoint_strict_generated_entry_parser_with_eof<'tokens>(
        entry: SyntaxParseEntry,
    ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<TextSyntax>> {
        match entry {
            SyntaxParseEntry::Text => {
                let parser = recovery_checkpoint_strict_generated_text_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let text = input.parse(&parser)?;
                    input.parse(end()).map(|()| text)
                })
                .boxed()
            }
            SyntaxParseEntry::NihoParagraphs => {
                let parser =
                    recovery_checkpoint_strict_generated_text_niho_paragraphs_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let paragraphs = input.parse(&parser)?;
                    input.parse(end()).map(|()| {
                        generated_runtime::SharedSyntaxOutput::new(text_of_niho_paragraphs(
                            paragraphs.into_owned(),
                        ))
                    })
                })
                .boxed()
            }
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn recovered_generated_entry_parser_with_eof<'tokens>(
        entry: SyntaxParseEntry,
    ) -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<recovered::TextSyntax>> {
        match entry {
            SyntaxParseEntry::Text => {
                let parser = recovered_generated_text_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let text = input.parse(&parser)?;
                    input.parse(end()).map(|()| text)
                })
                .boxed()
            }
            SyntaxParseEntry::NihoParagraphs => {
                let parser = recovered_generated_text_niho_paragraphs_shared_parser();
                custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
                    let paragraphs = input.parse(&parser)?;
                    input.parse(end()).map(|()| {
                        generated_runtime::SharedSyntaxOutput::new(
                            recovered_text_of_niho_paragraphs(paragraphs.into_owned()),
                        )
                    })
                })
                .boxed()
            }
        }
    }

    /// The text that consists of one run of NIhO-led paragraphs. The text grammar gives this
    /// same tree for the same tokens: before a NIhO, every leading part of a text is empty,
    /// and its first paragraph cannot start, so the text takes the `text_niho_paragraphs` arm.
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn text_of_niho_paragraphs(paragraphs: TextNihoParagraphsSyntax) -> TextSyntax {
        TextSyntax::RegularText(std::sync::Arc::new(RegularTextSyntax {
            leading_nai: Vec::new(),
            leading_cmevla: Vec::new(),
            leading_indicators: Vec::new(),
            leading_free_modifiers: Vec::new(),
            leading_connective: None,
            leading_i_statements: Vec::new(),
            paragraphs: Some(std::sync::Arc::new(
                TextParagraphsSyntax::TextNihoParagraphs(std::sync::Arc::new(paragraphs)),
            )),
        }))
    }

    /// The recovered form of [`text_of_niho_paragraphs`].
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn recovered_text_of_niho_paragraphs(
        paragraphs: recovered::TextNihoParagraphsSyntax,
    ) -> recovered::TextSyntax {
        recovered::TextSyntax::RegularText(std::sync::Arc::new(recovered::Recovered::valid(
            recovered::RegularTextSyntax {
                leading_nai: Vec::new(),
                leading_cmevla: Vec::new(),
                leading_indicators: Vec::new(),
                leading_free_modifiers: Vec::new(),
                leading_connective: None,
                leading_i_statements: Vec::new(),
                paragraphs: Some(std::sync::Arc::new(recovered::Recovered::valid(
                    recovered::TextParagraphsSyntax::TextNihoParagraphs(std::sync::Arc::new(
                        recovered::Recovered::valid(paragraphs),
                    )),
                ))),
            },
        )))
    }
}
