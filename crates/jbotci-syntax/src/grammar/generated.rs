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
    SpannedToken, SyntaxMemoScope, SyntaxParseError, SyntaxRecoveryMemoSession, SyntaxRuleFrame,
};
use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxWarning, SyntaxWordCategory, Token, TraceReport,
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
        binding_schema __jbotci_syntax_binding_schema;
        env generated_runtime::SyntaxGrammarEnv;
        strict_parsers;

    recursive {
        text: TextSyntax;
        paragraph: ParagraphSyntax;
        statement_or_fragment: StatementOrFragmentSyntax;
        statement: StatementSyntax;
        description_relative_statement: StatementSyntax;
        bridi: BridiSyntax;
        description_relative_bridi: BridiSyntax;
        bridi_tail: BridiTailSyntax;
        description_relative_bridi_tail: BridiTailSyntax;
        bo_grouped_bridi_tail: BoGroupedBridiTailSyntax;
        description_relative_bo_grouped_bridi_tail: BoGroupedBridiTailSyntax;
        bo_grouped_bridi_tail_without_tail_terms: BoGroupedBridiTailWithoutTailTermsSyntax;
        description_relative_bo_grouped_bridi_tail_without_tail_terms: BoGroupedBridiTailWithoutTailTermsSyntax;
        forethought_bridi_connection: ForethoughtBridiConnectionSyntax;
        description_relative_forethought_bridi_connection: ForethoughtBridiConnectionSyntax;
        forethought_bridi_connection_without_tail_terms: ForethoughtBridiConnectionWithoutTailTermsSyntax;
        description_relative_forethought_bridi_connection_without_tail_terms: ForethoughtBridiConnectionWithoutTailTermsSyntax;
        subbridi: SubbridiSyntax;
        description_relative_subbridi: SubbridiSyntax;
        bare_continuable_relative_clause_list: RelativeClauseListSyntax;
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
        // Rolling Zantufa's own NUhI-less termset is referenced from the same six leaf inventories
        // and carries whole `term+` runs, so leaving it out would rebuild that subgraph six times
        // over on every parse for exactly the reason recorded above.
        zantufa_gek_termset: ZantufaGekTermsetSyntax;
        sumti: SumtiSyntax;
        sumti_grouped: SumtiGroupedSyntax;
        sumti_afterthought: SumtiAfterthoughtSyntax;
        sumti_bound: SumtiBoundSyntax;
        sumti_forethought: SumtiForethoughtSyntax;
        sumti_base: SumtiBaseSyntax;
        selbri: SelbriSyntax;
        selbri_without_terminal_relative: SelbriWithoutTerminalRelativeSyntax;
        description_relative_full_selbri: SelbriSyntax;
        co_selbri: CoSelbriSyntax;
        cei_free_co_selbri: CoSelbriSyntax;
        tanru_selbri: TanruSelbriSyntax;
        cei_free_tanru_selbri: TanruSelbriSyntax;
        connected_selbri: ConnectedSelbriSyntax;
        cei_free_connected_selbri: ConnectedSelbriSyntax;
        bound_selbri: BoundSelbriSyntax;
        cei_free_bound_selbri: BoundSelbriSyntax;
        plain_bo_selbri: PlainBoSelbriSyntax;
        cei_free_plain_bo_selbri: PlainBoSelbriSyntax;
        tanru_unit: TanruUnitSyntax;
        cei_free_tanru_unit: TanruUnitSyntax;
        tanru_unit_atom: TanruUnitAtomSyntax;
        jai_inner_tanru_unit: JaiInnerTanruUnitSyntax;
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
        zantufa_mex: ZantufaMexSyntax;
        zantufa_mex_1: ZantufaMex1Syntax;
        zantufa_mex_2: ZantufaMex2Syntax;
        zantufa_operand: ZantufaOperandSyntax;
        zantufa_operator: ZantufaOperatorSyntax;
        zantufa_forethought_mekso: ZantufaForethoughtMeksoSyntax;
        zantufa_tcita_selci: ZantufaTcitaSelciSyntax;
        reverse_polish_parts: ReversePolishPartsSyntax;
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

    /// Top-level text syntax, distinguishing XAUhA…KUhAU framing from ordinary text.
    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// Text introduced by XAUhA and closed by KUhAU; the payload retains the framed paragraphs.
        explicit_xauha_lohoi_text,
        /// Ordinary text, retaining its leading material and optional paragraph tree.
        regular_text,
    }

    alias "word" word_before_kuhau = word_not_cmavo(Kuhau);

    /// XAUhA…KUhAU-framed text; framing words are consumed while paragraphs remain public.
    rule "text" explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier) -> struct {
        assert [
            cmavo(Xauha);
            zero_or_more word_before_kuhau();
            cmavo(Kuhau);
        ].ignored();
        /// The paragraphs enclosed by the ignored XAUhA…KUhAU framing sequence.
        field paragraphs <- text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
    }

    /// Ordinary text with source-ordered leading material and an optional paragraph tree.
    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
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
            modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci)
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
    rule "statement" statement(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> enum {
        /// Uses the `i_statement_connection` product form, whose payload preserves `leading_statement` and `continuations`.
        i_statement_connection,
        /// Uses the `preposed_i_statement_connection` product form, whose payload preserves `leading_statement`, `connective`, `i`, and `trailing_statement`.
        preposed_i_statement_connection,
        /// Uses the nested `statement_base` sum form and preserves its selected alternative.
        statement_base,
    }

    /// Sum node for statement; selects among the `prenex_statement`, `forethought_statement`, `bridi_statement`, and `text_group_statement` forms.
    rule "statement" statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> enum {
        /// Uses the `prenex_statement` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_statement`.
        prenex_statement,
        /// Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.
        when feature(ZantufaConnectives) forethought_statement,
        /// Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.
        bridi_statement,
        /// Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.
        text_group_statement,
    }

    /// Sum node for paragraph statement; selects among the `zantufa_statement_terms_statement`, `statement_or_fragment_statement`, and `fragment_statement` forms.
    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens, free_modifier, forethought_bridi_connection, normal_term) -> enum {
        /// Uses the `zantufa_statement_terms_statement` product form, whose payload preserves `statement` and `tail`.
        when feature(ZantufaTerms) zantufa_statement_terms_statement,
        /// Uses the `statement_or_fragment_statement` product form, whose payload preserves `statement`.
        statement_or_fragment_statement,
        /// Uses the nested `fragment_statement` sum form and preserves its selected alternative.
        fragment_statement,
    }

    /// Product node for paragraph statement; preserves `statement` and `tail` in source order.
    rule "paragraph statement" zantufa_statement_terms_statement(statement, term) -> struct {
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The `zantufa_statement_terms_tail` grammar result in the `tail` structural role of the `zantufa_statement_terms_statement` production.
        field tail <- zantufa_statement_terms_tail(term);
    }

    /// Sum node for paragraph statement; selects among the `zantufa_iau_statement_terms_tail` and `zantufa_bare_statement_terms_tail` forms.
    rule "paragraph statement" zantufa_statement_terms_tail(term) -> enum {
        /// Uses the `zantufa_iau_statement_terms_tail` product form, whose payload preserves `iau` and `terms`.
        zantufa_iau_statement_terms_tail,
        /// Uses the `zantufa_bare_statement_terms_tail` product form, whose payload preserves `terms`.
        zantufa_bare_statement_terms_tail,
    }

    /// Product node for paragraph statement; preserves `iau` and `terms` in source order.
    rule "paragraph statement" zantufa_iau_statement_terms_tail(term) -> struct {
        /// The `Ihau` cmavo marker.
        field iau <- cmavo(Ihau).warn(ExperimentalIauReset).wf();
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
    }

    /// Transparent product node for paragraph statement; preserves the `terms` component.
    rule "paragraph statement" zantufa_bare_statement_terms_tail(term) -> struct {
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more arc(term)];
    }

    /// Transparent product node for paragraph statement; preserves the `statement` component.
    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        /// The `statement` grammar result in the `statement` structural role of the `statement_or_fragment_statement` production.
        field statement <- statement;
    }

    /// Sum node for fragment; selects among 12 forms including `prenex_fragment`, `selbri_fragment`, and `ek_fragment`.
    rule "fragment" fragment_statement(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens, free_modifier, forethought_bridi_connection, normal_term) -> enum {
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
        /// Uses the `zantufa_mekso_fragment` product form, whose payload preserves `expression`.
        zantufa_mekso_fragment,
    }

    /// Sum node for statement; selects among the `forethought_statement`, `bridi_statement`, and `text_group_statement` forms.
    rule "statement" statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.
        when feature(ZantufaConnectives) forethought_statement,
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
    rule "statement connection" i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens, zantufa_mex, zantufa_tcita_selci));
        /// Non-empty ordered sequence of continuations components.
        #[recovery_boundary]
        field continuations <- [one_or_more i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci)];
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
    rule "statement connection" i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> enum {
        /// Uses the `chained_i_connective_statement_tail` product form, whose payload preserves `pending`, `i`, `connective`, and `trailing_statement`.
        chained_i_connective_statement_tail,
        /// Uses the `simple_i_connective_statement_tail` product form, whose payload preserves `i`, `connective`, and `trailing_statement`.
        simple_i_connective_statement_tail,
    }

    /// Product node for statement connection; preserves `pending`, `i`, `connective`, and `trailing_statement` in source order.
    rule "statement connection" chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// Non-empty ordered sequence of pending components.
        field pending <- [one_or_more pending_i_connective];
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The `i_statement_connective` connective joining the adjacent constituents of the `chained_i_connective_statement_tail` production.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci));
    }

    /// Product node for statement connection; preserves `i`, `connective`, and `trailing_statement` in source order.
    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The `i_statement_connective` connective joining the adjacent constituents of the `simple_i_connective_statement_tail` production.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci));
    }

    /// Product node for statement connection; preserves `leading_statement`, `connective`, `i`, and `trailing_statement` in source order.
    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens, zantufa_mex, zantufa_tcita_selci));
        /// The `statement_connective` connective joining the adjacent constituents of the `preposed_i_statement_connection` production.
        field connective <- statement_connective;
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci));
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

    /// Product node for statement; preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "statement" forethought_statement(statement, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The forethought connective that opens the statement and determines how its branches combine.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The first statement branch, which appears immediately after the opening forethought connective.
        field first <- arc(statement);
        /// The first GIK connective together with the statement branch that follows it.
        field first_branch <- forethought_statement_branch(statement);
        /// Additional Zantufa GIK-led statement branches in their source order.
        field additional_branches <- [zero_or_more zantufa_forethought_statement_branch(statement)];
        /// The optional experimental GIhI terminator following all statement branches.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for statement branch; preserves `gik` and `statement` in source order.
    rule "statement branch" forethought_statement_branch(statement) -> struct {
        /// The GI-family `gik_connective` connective separating the forethought branches of the `forethought_statement_branch` production.
        field gik <- gik_connective;
        /// The shared statement child syntax node.
        field statement <- arc(statement);
    }

    /// Product node for statement branch; preserves `gik` and `statement` in source order.
    rule "statement branch" zantufa_forethought_statement_branch(statement) -> struct {
        assert feature(ZantufaConnectives);
        /// The GI-family `zantufa_extra_gik_connective` connective separating the forethought branches of the `zantufa_forethought_statement_branch` production.
        field gik <- zantufa_extra_gik_connective;
        /// The shared statement child syntax node.
        field statement <- arc(statement);
    }

    /// Product node for statement; preserves `bridi` and `continuations` in source order.
    rule "statement" bridi_statement(bridi, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared bridi child syntax node.
        field bridi <- arc(bridi);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more bridi_statement_continuation(subbridi, tense_modal)];
    }

    /// Sum node for bridi continuation; selects among the `bo_bridi_statement_continuation` and `ke_bridi_statement_continuation` forms.
    rule "bridi continuation" bridi_statement_continuation(subbridi, tense_modal) -> enum {
        /// Uses the `bo_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `bo`, and `trailing_subbridi`.
        bo_bridi_statement_continuation,
        /// Uses the `ke_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe`.
        ke_bridi_statement_continuation,
    }

    /// Product node for bridi continuation; preserves `connective`, `tense_modal`, `bo`, and `trailing_subbridi` in source order.
    rule "bridi continuation" bo_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bo_bridi_statement_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared trailing subbridi child syntax node.
        field trailing_subbridi <- arc(subbridi);
    }

    /// Product node for bridi continuation; preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe` in source order.
    rule "bridi continuation" ke_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        /// The `relation_afterthought_connective` connective joining the adjacent constituents of the `ke_bridi_statement_continuation` production.
        field connective <- relation_afterthought_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared trailing subbridi child syntax node.
        field trailing_subbridi <- arc(subbridi);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
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
    rule "mex" mekso_fragment(mekso, letter_tokens, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The shared quantifier child syntax node.
        field quantifier <- arc(quantifier(mekso, letter_tokens, free_modifier));
    }

    /// Transparent product node for mex; preserves the `expression` component.
    rule "mex" zantufa_mekso_fragment(mekso) -> struct {
        #[tree_child(primary)]
        /// The shared expression child syntax node.
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_statement_item());
    }

    // A bare continuation marker must remain visible to the containing relative
    // list rather than being consumed by the terminal selbri in the preceding
    // clause body. Instantiate the existing statement/bridi family with the
    // no-terminal-relative selbri entry; all generated node types stay shared.
    alias "statement" description_relative_statement(
        description_relative_statement,
        description_relative_bridi,
        term,
        sumti,
        description_relative_subbridi,
        description_relative_full_selbri,
        mekso,
        tense_modal,
        text,
        letter_tokens,
        zantufa_mex,
        zantufa_tcita_selci,
    ) = statement(
        description_relative_statement,
        description_relative_bridi,
        term,
        sumti,
        description_relative_subbridi,
        description_relative_full_selbri,
        mekso,
        tense_modal,
        text,
        letter_tokens,
        zantufa_mex,
        zantufa_tcita_selci,
    ).recursive_output(description_relative_statement);

    alias "bridi" description_relative_bridi(
        term,
        description_relative_full_selbri,
        description_relative_subbridi,
        tense_modal,
        description_relative_bridi_tail,
    ) = bridi(
        term,
        description_relative_full_selbri,
        description_relative_subbridi,
        tense_modal,
        description_relative_bridi_tail,
    ).recursive_output(description_relative_bridi);

    alias "bridi tail" description_relative_bridi_tail(
        description_relative_bridi_tail,
        description_relative_bo_grouped_bridi_tail,
        description_relative_bo_grouped_bridi_tail_without_tail_terms,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ) = bridi_tail(
        description_relative_bridi_tail,
        description_relative_bo_grouped_bridi_tail,
        description_relative_bo_grouped_bridi_tail_without_tail_terms,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ).recursive_output(description_relative_bridi_tail);

    alias "bridi tail" description_relative_bo_grouped_bridi_tail(
        description_relative_bo_grouped_bridi_tail,
        description_relative_forethought_bridi_connection,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ) = bo_grouped_bridi_tail(
        description_relative_bo_grouped_bridi_tail,
        description_relative_forethought_bridi_connection,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ).recursive_output(description_relative_bo_grouped_bridi_tail);

    alias "bridi tail" description_relative_bo_grouped_bridi_tail_without_tail_terms(
        description_relative_bo_grouped_bridi_tail_without_tail_terms,
        description_relative_forethought_bridi_connection_without_tail_terms,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ) = bo_grouped_bridi_tail_without_tail_terms(
        description_relative_bo_grouped_bridi_tail_without_tail_terms,
        description_relative_forethought_bridi_connection_without_tail_terms,
        description_relative_full_selbri,
        description_relative_subbridi,
        term,
        tense_modal,
    ).recursive_output(description_relative_bo_grouped_bridi_tail_without_tail_terms);

    alias "forethought bridi connection" description_relative_forethought_bridi_connection(
        description_relative_forethought_bridi_connection,
        description_relative_subbridi,
        term,
        tense_modal,
        baseline_term_tense_modal,
        description_relative_full_selbri,
        zantufa_mex,
        letter_tokens,
        zantufa_tcita_selci,
    ) = forethought_bridi_connection(
        description_relative_forethought_bridi_connection,
        description_relative_subbridi,
        term,
        tense_modal,
        baseline_term_tense_modal,
        description_relative_full_selbri,
        zantufa_mex,
        letter_tokens,
        zantufa_tcita_selci,
    ).recursive_output(description_relative_forethought_bridi_connection);

    alias "forethought bridi connection" description_relative_forethought_bridi_connection_without_tail_terms(
        description_relative_forethought_bridi_connection_without_tail_terms,
        description_relative_subbridi,
        tense_modal,
        baseline_term_tense_modal,
        description_relative_full_selbri,
        zantufa_mex,
        letter_tokens,
        zantufa_tcita_selci,
    ) = forethought_bridi_connection_without_tail_terms(
        description_relative_forethought_bridi_connection_without_tail_terms,
        description_relative_subbridi,
        tense_modal,
        baseline_term_tense_modal,
        description_relative_full_selbri,
        zantufa_mex,
        letter_tokens,
        zantufa_tcita_selci,
    ).recursive_output(description_relative_forethought_bridi_connection_without_tail_terms);

    alias "subbridi" description_relative_subbridi(
        description_relative_subbridi,
        description_relative_bridi,
        term,
    ) = subbridi(
        description_relative_subbridi,
        description_relative_bridi,
        term,
    ).recursive_output(description_relative_subbridi);

    alias "relative clauses" bare_continuable_relative_clause_list(
        sumti,
        description_relative_subbridi,
        tense_modal,
        description_relative_statement,
    normal_term,
    ) = memo_scope(
        DescriptionRelative,
        relative_clause_list(
            sumti,
            description_relative_subbridi,
            tense_modal,
            description_relative_statement,
        normal_term,
    ),
    ).recursive_output(bare_continuable_relative_clause_list);

    alias "selbri" description_relative_full_selbri(
        selbri_without_terminal_relative,
        selbri,
    ) = selbri_without_terminal_relative.map_to(selbri);

    /// Product node for relative clauses; preserves `first` and `additional` in source order.
    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The initial `relative_clause_atom` constituent before the continuations of the `relative_clause_list` production.
        field first <- relative_clause_atom(sumti, subbridi, tense_modal, statement, normal_term);
        /// Ordered sequence of zero or more additional components.
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal, statement, normal_term)];
    }

    /// Transparent product node for relative clauses; preserves the `relative_clauses` component.
    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        #[tree_child(primary)]
        /// The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `relative_clause_fragment` production.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term);
    }

    /// Transparent product node for linked arguments; preserves the `bei_links` component.
    rule "linked arguments" linked_sumti_continuation_fragment(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        #[tree_child(primary)]
        /// Non-empty ordered sequence of bei links components.
        field bei_links <- [one_or_more bei_link(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term)];
    }

    /// Transparent product node for linked arguments; preserves the `linkargs` component.
    rule "linked arguments" linked_sumti_fragment(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        #[tree_child(primary)]
        /// The `linkargs` grammar result in the `linkargs` structural role of the `linked_sumti_fragment` production.
        field linkargs <- linkargs(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term);
    }

    /// Sum node for bridi; selects among the `bridi_with_leading_terms`, `bridi_with_post_cu_terms`, `bare_cu_bridi`, `bare_cu_terms_bridi`, and `relation_only_bridi` forms.
    rule "bridi" bridi(term, selbri, subbridi, tense_modal, bridi_tail) -> enum {
        /// Uses the `bridi_with_leading_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.
        bridi_with_leading_terms,
        /// Uses the `bridi_with_post_cu_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.
        bridi_with_post_cu_terms,
        /// Uses the `bare_cu_bridi` product form, whose payload preserves `cu` and `bridi_tail`.
        bare_cu_bridi,
        /// Uses the `bare_cu_terms_bridi` product form, whose payload preserves `cu` and `bridi_tail`.
        bare_cu_terms_bridi,
        /// Uses the `relation_only_bridi` product form, whose payload preserves `bridi_tail`.
        relation_only_bridi,
    }

    /// Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.
    rule "bridi" bridi_with_leading_terms(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of leading terms components.
        field leading_terms <- [one_or_more term];
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.
    rule "bridi" bridi_with_post_cu_terms(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of leading terms components.
        field leading_terms <- [one_or_more term];
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Product node for bridi; preserves `cu` and `bridi_tail` in source order.
    rule "bridi" bare_cu_bridi(bridi_tail) -> struct {
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi; preserves `cu` and `bridi_tail` in source order.
    rule "bridi" bare_cu_terms_bridi(term, bridi_tail) -> struct {
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Transparent product node for bridi; preserves the `bridi_tail` component.
    rule "bridi" relation_only_bridi(bridi_tail) -> struct {
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi tail; preserves `terms` and `bridi_tail` in source order.
    rule "bridi tail" cu_terms_bridi_tail(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more term];
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Sum node for bridi tail; selects among the `zantufa_grouped_bridi_tail`, `bridi_tail_with_possible_tail_terms`, and `bridi_tail_without_tail_terms` forms.
    rule "bridi tail" bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `zantufa_grouped_bridi_tail` product form, whose payload preserves `ke`, `bridi_tail`, `kehe`, `tail_terms`, and `vau`.
        when feature(ZantufaTerms) zantufa_grouped_bridi_tail,
        /// Uses the `bridi_tail_with_possible_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.
        bridi_tail_with_possible_tail_terms,
        /// Uses the `bridi_tail_without_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.
        bridi_tail_without_tail_terms,
    }

    /// Product node for bridi tail; preserves `ke`, `bridi_tail`, `kehe`, `tail_terms`, and `vau` in source order.
    rule "bridi tail" zantufa_grouped_bridi_tail(bridi_tail, term) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).warn(ExperimentalZantufaGroupedBridiTail).wf();
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional ke continuation component.
        field ke_continuation <- opt(arc(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        assert !(relation_connective_as_bridi_tail, opt(arc(tense_modal)), cmavo(Ke));
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
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `bo_continuation` in source order.
    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal)));
    }

    /// Sum node for bridi tail; selects among the `forethought_simple_bridi_tail_without_tail_terms` and `selbri_simple_bridi_tail_without_tail_terms` forms.
    rule "bridi tail" simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `forethought_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `connection`.
        forethought_simple_bridi_tail_without_tail_terms,
        /// Uses the `selbri_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `selbri` and `vau`.
        selbri_simple_bridi_tail_without_tail_terms,
    }

    /// Sum node for bridi tail; selects among the `forethought_simple_bridi_tail` and `selbri_simple_bridi_tail` forms.
    rule "bridi tail" simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> enum {
        /// Uses the `forethought_simple_bridi_tail` product form, whose payload preserves `connection`.
        forethought_simple_bridi_tail,
        /// Uses the `selbri_simple_bridi_tail` product form, whose payload preserves `selbri`, `terms`, and `vau`.
        selbri_simple_bridi_tail,
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
    rule "forethought bridi connection" forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal, baseline_term_tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// Uses the `grouped_forethought_bridi_connection` product form, whose payload preserves `tense_modals`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection,
        /// Uses the `direct_forethought_bridi_connection` product form, whose payload preserves `gek`, `first`, `first_branch`, and 4 other fields.
        direct_forethought_bridi_connection,
        /// Uses the `negated_forethought_bridi_connection` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection,
    }

    /// Sum node for forethought bridi connection; selects among the `direct_forethought_bridi_connection_without_tail_terms`, `grouped_forethought_bridi_connection_without_tail_terms`, and `negated_forethought_bridi_connection_without_tail_terms` forms.
    rule "forethought bridi connection" forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal, baseline_term_tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// Uses the `grouped_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `tense_modals`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection_without_tail_terms,
        /// Uses the `direct_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `gek`, `first`, `first_branch`, and 3 other fields.
        direct_forethought_bridi_connection_without_tail_terms,
        /// Uses the `negated_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection_without_tail_terms,
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 4 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection(subbridi, term, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The opening forethought connective that determines how the subbridi branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The first subbridi branch, which follows the opening connective without an intervening GIK.
        field first <- arc(subbridi);
        /// The first GIK-led subbridi branch paired with the opening connective.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// Additional Zantufa GIK-led subbridi branches, retained in source order.
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        /// The optional experimental GIhI terminator following the complete branch sequence.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        /// Terms attached to the completed forethought bridi after its connected subbridi branches.
        field tail_terms <- [zero_or_more term];
        /// The optional elidable VAU terminator for the bridi tail.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 3 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The opening forethought connective that determines how the subbridi branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The first subbridi branch, which follows the opening connective without an intervening GIK.
        field first <- arc(subbridi);
        /// The first GIK-led subbridi branch paired with the opening connective.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// Additional Zantufa GIK-led subbridi branches, retained in source order.
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        /// The optional experimental GIhI terminator following the complete branch sequence.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
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

    /// Product node for forethought bridi branch; preserves `gik` and `branch` in source order.
    rule "forethought bridi branch" zantufa_forethought_bridi_branch(subbridi) -> struct {
        assert feature(ZantufaConnectives);
        /// The additional Zantufa GIK connective that introduces this branch.
        field gik <- zantufa_extra_gik_connective;
        /// The subbridi governed by this additional branch's GIK connective.
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

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.
    rule "bridi tail connective" bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_ke_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
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

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.
    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        /// The `gihek_connective` connective joining the adjacent constituents of the `gihek_bridi_tail_ke_continuation` production.
        field connective <- gihek_connective();
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
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

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, `cu`, and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation_without_tail_terms` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, and 4 other fields in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective`, `cu`, and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation_without_tail_terms` production.
        field connective <- bridi_tail_connective;
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `cu`, `bridi_tail`, `tail_terms`, and `vau` in source order.
    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation` production.
        field connective <- bridi_tail_connective;
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
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
    /// Like the levels below it, this rule re-lists the leaf inventory instead of nesting a sum
    /// branch: a nested branch would add a public wrapper variant to Debug and serde output. The
    /// binding-schema drift guard keeps every level's leaf inventory synchronized with
    /// `simple_term`.
    rule "term" term(gek_termset, zantufa_gek_termset, statement, term, cehe_term, loose_term, nonabs_term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `pehe_termset_connection` product form, whose payload preserves `leading_term` and `continuations`.
        pehe_termset_connection,
        /// Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.
        termset_group,
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the `stag_bound_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        stag_bound_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the absorption-safe `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The CEhE level of the composed term hierarchy: `terms_2 <- term (CEhE free* nonabs_term)*`
    /// (camxes.peg:116). It is the operand level of the PEhE connection above it.
    rule "term" cehe_term(gek_termset, zantufa_gek_termset, statement, term, loose_term, nonabs_term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.
        termset_group,
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the `stag_bound_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        stag_bound_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the absorption-safe `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The loose connective level of the composed term hierarchy: camxes-exp `abs_term_1 <-
    /// abs_term_2 (joik_ek !tag_bo_ke_bridi_tail !tag_bo_subsentence abs_term_2)*`
    /// (camxes-exp.peg:153). It is the leading operand level of the CEhE connection above it.
    rule "term" loose_term(gek_termset, zantufa_gek_termset, statement, term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the `stag_bound_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        stag_bound_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the absorption-safe `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
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
    rule "term" nonabs_term(gek_termset, zantufa_gek_termset, statement, term, bound_term, simple_term, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the `stag_bound_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        stag_bound_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the unguarded `nonabs_tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        nonabs_tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// Product node for termset connection; preserves `leading_term` and `continuations` in source order.
    rule "termset connection" pehe_termset_connection(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(cehe_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more pehe_termset_connection_continuation(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci)];
    }

    /// Product node for termset connection continuation; preserves `pehe`, `connective`, and `trailing_term` in source order.
    rule "termset connection continuation" pehe_termset_connection_continuation(statement, sumti, cehe_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The `Pehe` cmavo marker.
        field pehe <- cmavo(Pehe).wf();
        /// The PEhE connective. camxes-standard spells the PEhE level `joik_jek` (camxes.peg:114),
        /// which is the JOIK-or-JEK inventory; #806 carries that domain, so EK and VUhU are
        /// rejected here with a documented-gap ledger row against camxes-exp's literal `joik_jek`.
        field connective <- standard_statement_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(cehe_term);
    }

    /// Sum node for term; selects among 13 forms including `place_tagged_sumti_term`, `jai_tagged_sumti_term`, and `tagged_sumti_before_tag_term`.
    rule "term" simple_term(gek_termset, zantufa_gek_termset, statement, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The BO-bound precedence level for ordinary terms in the camxes-exp hierarchy.
    ///
    /// The leaf rules are deliberately listed directly rather than through `simple_term`: a
    /// nested sum branch would add a public wrapper variant to Debug and serde output. The
    /// binding-schema drift guard keeps this leaf inventory synchronized with `simple_term`.
    rule "term" bound_term(gek_termset, zantufa_gek_termset, statement, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, simple_term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the diagnosed BO-bound connection with the mandatory absorption-safe stag.
        stag_bound_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The BO-bound ordinary-term connection with one or more continuations.
    ///
    /// camxes-exp's absorption-safe `abs_term_2` requires the stag before BO
    /// (camxes-exp.peg:154); camxes-standard has no term-level BO at all, so every occurrence is
    /// diagnosed. The operands intentionally remain `simple_term`: sumti greediness must continue
    /// to own chains whose trailing operand is a bare sumti, rather than silently changing their
    /// term-level grouping.
    rule "term connection" stag_bound_term_connection(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The first simple term at the BO-bound precedence level.
        field leading_term <- arc(simple_term);
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more stag_bound_term_continuation(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci)];
    }

    /// One mandatory-stag BO continuation at the absorption-safe term level.
    rule "term connection continuation" stag_bound_term_continuation(statement, sumti, simple_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The connective joining the adjacent simple terms.
        field connective <- term_afterthought_connective;
        /// The mandatory camxes-exp `stag` before BO.
        field tense_modal <- arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection));
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
    rule "term connection" connected_term(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(bound_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more connected_term_continuation(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci)];
    }

    /// Product node for term connection continuation; preserves `connective` and `trailing_term` in source order.
    rule "term connection continuation" connected_term_continuation(statement, sumti, bound_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        assert zantufa_na_led_term_joik_guard();
        /// The `term_afterthought_connective` connective joining the adjacent constituents of the `connected_term_continuation` production.
        field connective <- term_afterthought_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(bound_term);
    }

    // Zantufa's NA-led JOIK collides with the established successful baseline
    // parse of `term NA JOI term`. Exclude exactly that leading shape at term
    // consumers; every other JOIK extension remains reachable there.
    alias "term joik" zantufa_na_led_term_joik_guard = choice((
        feature(ZantufaConnectives).not(),
        (selmaho(Na), opt(selmaho(Se)), choice((selmaho(Joi), selmaho(Bihi)))).not(),
    )).ignored();

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
    /// The leaves are re-listed directly rather than nested behind a sum branch, exactly as every
    /// other ladder level does it (mechanism E): a nested branch would add a public wrapper
    /// variant to Debug and serde output. The binding-schema drift guard keeps this inventory
    /// synchronized with `simple_term`.
    rule "term" normal_term(gek_termset, zantufa_gek_termset, statement, term, bound_normal_term, normal_term_atom, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `connected_normal_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_normal_term,
        /// Uses the `bound_normal_term_connection` product form, whose payload preserves `leading_term` and `continuations`.
        bound_normal_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the unguarded `nonabs_tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        nonabs_tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The normal-flavour loose connection with one or more continuations.
    rule "term connection" connected_normal_term(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The first normal-flavour term at the loose precedence level.
        field leading_term <- arc(bound_normal_term);
        /// The nonempty source-ordered loose continuation sequence.
        field continuations <- [one_or_more connected_normal_term_continuation(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci)];
    }

    /// One normal-flavour loose continuation.
    rule "term connection continuation" connected_normal_term_continuation(statement, sumti, bound_normal_term, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        assert zantufa_na_led_term_joik_guard();
        /// The connective joining the adjacent normal-flavour terms.
        field connective <- term_afterthought_connective;
        /// The BO-bound normal-flavour term following the connective.
        field trailing_term <- arc(bound_normal_term);
    }

    /// The optional-stag BO-bound level of the normal-flavour term constituent.
    rule "term" bound_normal_term(gek_termset, zantufa_gek_termset, statement, term, normal_term_atom, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the diagnosed optional-stag BO-bound normal-flavour connection.
        bound_normal_term_connection,
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the unguarded `nonabs_tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        nonabs_tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// The diagnosed optional-stag BO connection at the normal-flavour term level.
    ///
    /// camxes-standard has no term-level BO at all, so every occurrence is diagnosed, exactly as
    /// the mandatory-stag twin `stag_bound_term_connection` is. Unlike that twin the operands are
    /// the unguarded leaves, because camxes-exp's normal `term_2 <- term_3 (joik_ek stag?
    /// BO_clause term_3)*` (camxes-exp.peg:143) takes the unguarded `tag_term` on both sides.
    rule "term connection" bound_normal_term_connection(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The first unguarded leaf at the BO-bound precedence level.
        field leading_term <- arc(normal_term_atom);
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more bound_normal_term_continuation(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci)];
    }

    /// One optional-stag BO continuation at the normal-flavour term level.
    rule "term connection continuation" bound_normal_term_continuation(statement, sumti, normal_term_atom, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The connective joining the adjacent normal-flavour terms.
        field connective <- term_afterthought_connective;
        /// The optional camxes-exp `stag`; unlike the absorption-safe tier, the normal flavour
        /// leaves it out.
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
        /// The `Bo` cmavo marker, which owns the experimental warning for the whole connection.
        field bo <- cmavo(Bo).warn(ExperimentalTermBoConnection).wf();
        /// The unguarded leaf following BO.
        field trailing_term <- arc(normal_term_atom);
    }

    /// The unguarded leaf inventory of the normal-flavour term constituent.
    ///
    /// This is `term_3 <- sumti / tag_term / termset` (camxes-exp.peg:145) and camxes-standard's
    /// bare `nonabs_term` (camxes.peg:128) at once: the same leaves `simple_term` lists, with the
    /// unguarded `nonabs_tagged_sumti_term` in place of its absorption-guarded twin.
    rule "term" normal_term_atom(gek_termset, zantufa_gek_termset, statement, sumti, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
        /// Uses the `elided_nahe_fiho_tag_term` product form for the sourced final tag-term fragment.
        elided_nahe_fiho_tag_term,
        /// Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.
        tagged_sumti_before_tag_term,
        /// Uses the unguarded `nonabs_tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.
        nonabs_tagged_sumti_term,
        /// Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.
        noiha_adverbial_term,
        /// Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.
        fihoi_adverbial_term,
        /// Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.
        soi_adverbial_term,
        /// Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.
        na_ku_term,
        /// Uses the `sumti_term` product form, whose payload preserves `sumti`.
        sumti_term,
        /// Uses the `bare_na_term` product form, whose payload preserves `na`.
        bare_na_term,
        /// Uses the `gek_termset` product form, whose payload preserves the classified NUhI-less candidate.
        gek_termset,
        /// Uses the `zantufa_gek_termset` product form, whose payload preserves the classified
        /// rolling-Zantufa NUhI-less candidate.
        zantufa_gek_termset,
        /// Uses the NUhI-mandatory `forethought_termset` product form, whose payload preserves
        /// `nuhi`, `gek`, `terms`, and 2 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// Product node for termset; preserves `leading_term` and `continuations` in source order.
    ///
    /// This is the CEhE level. Its leading operand is the full loose/BO term level, while each
    /// continuation takes the unguarded `nonabs` flavour, exactly as camxes.peg:116 pairs `term`
    /// with `nonabs_term`.
    rule "termset" termset_group(statement, sumti, loose_term, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(loose_term);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more termset_group_continuation(statement, sumti, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci)];
    }

    /// Product node for termset continuation; preserves `cehe` and `trailing_term` in source order.
    rule "termset continuation" termset_group_continuation(statement, sumti, nonabs_term, tense_modal, baseline_term_tense_modal, subbridi, selbri, term, letter_tokens, letter_string, free_modifier, forethought_bridi_connection, zantufa_mex, zantufa_tcita_selci) -> struct {
        /// The `Cehe` cmavo marker.
        field cehe <- cmavo(Cehe).wf();
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(nonabs_term);
    }

    /// The NUhI-gek forethought termset: `NUhI free* gek terms NUhU? free* gik terms NUhU? free*`
    /// (camxes.peg:136, camxes-exp.peg:191).
    ///
    /// This is the first of the three sourced termset shapes. The NUhI is MANDATORY: the NUhI-less
    /// surface is `gek_termset` in camxes-standard and camxes-exp alike, and rolling Zantufa's own
    /// NUhI-less shape has neither a NUhI selma'o nor a NUhU slot, so an optional-NUhI reading of
    /// this arm would source its NUhU slots and its branch count from nothing at all. Both operand
    /// positions are the full GUARDED `terms` sequences (B1), which is what separates this arm from
    /// the NUhI-less one.
    ///
    /// Product node for termset; preserves `nuhi`, `gek`, `terms`, and 2 other fields in source order.
    rule "termset" forethought_termset(term, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The mandatory NUhI marker introducing the forethought termset before its connective.
        field nuhi <- cmavo(Nuhi).wf();
        /// The opening forethought connective that determines how the term sequences are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
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

    /// Rolling Zantufa's own NUhI-less termset: `gek_term <- gek term+ (gik term+)+ GIhI?`
    /// (zantufa-1.9999.peg:32), which Zantufa lists last in its `term_2` leaf inventory.
    ///
    /// It differs from the sourced `gek_termset` in exactly the ways this arm exists to carry:
    /// each operand position is a whole `term+` run rather than a single term, so the branches need
    /// not be balanced, the branch sequence is n-ary rather than binary, and a GIhI may close it.
    /// None of that is sourced by camxes-standard or camxes-exp, so the arm is
    /// `ZantufaConnectives`-gated and ordered BEHIND the sourced `gek_termset` at every level that
    /// offers both.
    ///
    /// Zantufa's `gik <- GI_clause` (zantufa-1.9999.peg:72) carries no NAI, because Zantufa has no
    /// NAI selma'o at all — `gi nai` parses there with `nai` absorbed as a UI free modifier. The
    /// first branch nevertheless spells its connective the shared `gik_connective`: the surface is
    /// accepted by Zantufa either way, so this is a reading difference rather than a widening, and
    /// jbotci reads NAI as NAI everywhere else a GIK appears. The 6b ledger carries the row.
    rule "termset" zantufa_gek_termset(term, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        assert feature(ZantufaConnectives);
        #[tree_child(primary)]
        /// The completed candidate, retained only when the GEK sumti connection — which Zantufa
        /// spells n-ary as `sumti_3 <- gek sumti (gik sumti)+ GIhI?` (zantufa-1.9999.peg:36) — does
        /// not own its identical extent.
        field termset <- arc(
            zantufa_gek_termset_candidate(term, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci)
                .reject_output(crate::grammar::baseline_termset::ZantufaBaselineGekSumtiRejection)
        );
    }

    /// The classified body of rolling Zantufa's NUhI-less termset.
    rule "termset" zantufa_gek_termset_candidate(term, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The opening forethought connective that determines how the term sequences are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The initial nonempty term sequence following the opening connective.
        field terms <- [one_or_more arc(term)];
        /// The first GIK-led term-sequence branch paired with the opening connective.
        field first_branch <- zantufa_forethought_termset_first_branch(term);
        /// Additional Zantufa GIK-led term-sequence branches, retained in source order.
        field additional_branches <- [zero_or_more zantufa_forethought_termset_branch(term)];
        /// The optional experimental GIhI terminator following the complete branch sequence.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for termset; preserves `gik` and `terms` in source order.
    rule "termset" zantufa_forethought_termset_first_branch(term) -> struct {
        /// The GIK connective that introduces this branch and pairs with the opening connective.
        field gik <- gik_connective;
        /// The nonempty term sequence governed by this branch's GIK connective.
        field terms <- [one_or_more arc(term)];
    }

    /// Product node for termset; preserves `gik` and `terms` in source order.
    rule "termset" zantufa_forethought_termset_branch(term) -> struct {
        assert feature(ZantufaConnectives);
        /// The additional Zantufa GIK connective that introduces this branch.
        field gik <- zantufa_extra_gik_connective;
        /// The nonempty term sequence governed by this additional branch's GIK connective.
        field terms <- [one_or_more arc(term)];
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
    rule "termset" gek_termset(balanced_termset_operands, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        #[tree_child(primary)]
        /// The completed NUhI-less candidate, retained only when the baseline GEK sumti connection
        /// does not own its identical extent.
        field termset <- arc(
            gek_termset_candidate(balanced_termset_operands, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci)
                .reject_output(crate::grammar::baseline_termset::BaselineGekSumtiRejection)
        );
    }

    /// The classified body of the NUhI-less forethought termset.
    rule "termset" gek_termset_candidate(balanced_termset_operands, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The opening forethought connective that determines how the operands are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
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

    /// Product node for termset; preserves `ke`, `termset`, and `kehe` in source order.
    rule "termset" ke_termset(term, tense_modal, baseline_term_tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        assert !grouped_forethought_bridi_term_escape(tense_modal, baseline_term_tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci).ignored();
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).warn(ExperimentalKeTermset).wf();
        /// Non-empty ordered sequence of termset components.
        field termset <- [one_or_more arc(term)];
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Lookahead shape that reserves KE tag+ KE forethought bridi groups from
    /// the overlapping experimental KE termset owner.
    rule "forethought bridi connection" grouped_forethought_bridi_term_escape(tense_modal, baseline_term_tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The outer grouping KE.
        field outer_ke <- cmavo(Ke).wf();
        /// One or more source-ordered tags that make the ownership collision possible.
        field tense_modals <- [one_or_more arc(standard_forethought_tense_modal(baseline_term_tense_modal, tense_modal))];
        /// The inner grouping KE following the tags.
        field inner_ke <- cmavo(Ke).wf();
        /// The forethought connective beginning inside the inner group.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
    }

    alias "tag" standard_forethought_tense_modal(baseline_term_tense_modal, tense_modal) =
        baseline_term_tense_modal.map_to(tense_modal);

    /// Sum node for NOIhA adverbial; selects among the `noiha_variable_adverbial_term` and `noiha_relative_adverbial_term` forms.
    rule "NOIhA adverbial" noiha_adverbial_term(free_modifier, selbri) -> enum {
        /// Uses the `noiha_variable_adverbial_term` product form, whose payload preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku`.
        noiha_variable_adverbial_term,
        /// Uses the `noiha_relative_adverbial_term` product form, whose payload preserves `noiha`, `selbri`, and `fehu`.
        noiha_relative_adverbial_term,
    }

    /// Product node for NOIhA adverbial; preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku` in source order.
    rule "NOIhA adverbial" noiha_variable_adverbial_term(free_modifier, selbri) -> struct {
        /// A word from selmaho `Noiha`.
        field poiha <- selmaho(Noiha).wf();
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The `Ku` cmavo marker.
        field brigahi_ku <- cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
    }

    /// Product node for NOIhA adverbial; preserves `noiha`, `selbri`, and `fehu` in source order.
    rule "NOIhA adverbial" noiha_relative_adverbial_term(selbri) -> struct {
        /// A word from selmaho `Noiha`.
        field noiha <- selmaho(Noiha).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Fehu` cmavo marker.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Product node for FIhOI adverbial; preserves `fihoi`, `statement`, and `fihau` in source order.
    rule "FIhOI adverbial" fihoi_adverbial_term(statement) -> struct {
        /// The `Fihoi` cmavo marker.
        field fihoi <- cmavo(Fihoi).warn(ExperimentalFihoiAdverbial).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Fihau` cmavo marker.
        field fihau <- opt(cmavo(Fihau).wf()).elidable_terminator(Fihau);
    }

    /// Product node for SOI adverbial; preserves `soi`, `statement`, and `sehu` in source order.
    rule "SOI adverbial" soi_adverbial_term(statement) -> struct {
        /// A word from selmaho `Soi`.
        field soi <- selmaho(Soi).warn(ExperimentalSoiAdverbial).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    /// Transparent product node for term; preserves the `sumti` component.
    rule "term" sumti_term(sumti) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
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
    rule "NA term" bare_na_term(selbri, tense_modal, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        assert !choice((
            selbri
                .reject_output(crate::grammar::baseline_tag::PostNaExtensionTagRejection)
                .ignored(),
            modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci).ignored(),
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
    rule "tag" tagged_sumti_before_tag_term(tense_modal, baseline_term_tense_modal, selbri, letter_tokens, letter_string, zantufa_mex, zantufa_tcita_selci) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal.map_to(tense_modal),
            selbri,
            letter_tokens,
            letter_string,
        ));
        assert tense_modal.lookahead();
    }

    /// Product node for tag; preserves `tense_modal` and `sumti` in source order.
    rule "tag" tagged_sumti_term(tense_modal, baseline_term_tense_modal, sumti, selbri, letter_tokens, letter_string, zantufa_mex, zantufa_tcita_selci, normal_term) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal.map_to(tense_modal),
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
    /// guarded twin is `tagged_sumti_term`; the two rules differ only by that assertion, and the
    /// binding-schema drift guard keeps the flavoured leaf inventories aligned.
    rule "tag" nonabs_tagged_sumti_term(tense_modal, baseline_term_tense_modal, sumti, selbri, letter_tokens, letter_string, zantufa_mex, zantufa_tcita_selci, normal_term) -> struct {
        assert !modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(
            baseline_term_tense_modal.map_to(tense_modal),
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

    /// Product node for tag; preserves `jai`, `tag`, and `sumti` in source order.
    rule "tag" jai_tagged_sumti_term(tense_modal, sumti) -> struct {
        assert feature(ZantufaTags);
        /// The `Jai` cmavo marker.
        field jai <- cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
        /// The optional tag component.
        field tag <- opt(arc(tense_modal));
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Sum node for tag; selects among 8 forms including `pu_before_nahe_leading_term_tag_tense`, `pu_distance_before_tag_leading_term_tag_tense`, and `zi_before_zi_leading_term_tag_tense`.
    rule "tag" leading_term_tag_tense_modal(tense_modal, selbri, letter_tokens, letter_string) -> enum {
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
        /// Uses the `tense_modal` product form, whose payload preserves `body`.
        tense_modal,
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
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement, normal_term));
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
        field bound_tail <- opt(bound_sumti_tail(sumti_bound, tense_modal));
    }

    /// Sum node for sumti; selects among the `forethought_sumti` and `simple_sumti` forms.
    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, selbri, letter_tokens, free_modifier, statement, zantufa_mex, zantufa_tcita_selci, normal_term) -> enum {
        /// Uses the `forethought_sumti` product form, whose payload preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi`.
        forethought_sumti,
        /// Uses the `simple_sumti` product form, whose payload preserves `base_sumti` and `relative_clauses`.
        simple_sumti,
    }

    /// Product node for forethought sumti connection; preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal, statement, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The opening forethought connective that determines how the sumti branches are combined.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
        /// The first sumti branch, which follows the opening connective without an intervening GIK.
        field leading_sumti <- arc(sumti);
        /// The first GIK-led sumti branch paired with the opening connective.
        field first_branch <- forethought_sumti_branch(sumti_forethought);
        /// Additional Zantufa GIK-led sumti branches, retained in source order.
        field additional_branches <- [zero_or_more zantufa_forethought_sumti_branch(sumti_forethought)];
        /// The optional experimental GIhI terminator following the complete branch sequence.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.
    rule "forethought sumti connection" forethought_sumti_branch(sumti_forethought) -> struct {
        /// The GIK connective that introduces this branch and pairs with the opening forethought connective.
        field gik <- gik_connective;
        /// The sumti governed by this branch's GIK connective.
        field sumti <- arc(sumti_forethought);
    }

    /// Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.
    rule "forethought sumti connection" zantufa_forethought_sumti_branch(sumti_forethought) -> struct {
        assert feature(ZantufaConnectives);
        /// The additional Zantufa GIK connective that introduces this branch.
        field gik <- zantufa_extra_gik_connective;
        /// The sumti governed by this additional branch's GIK connective.
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
        assert zantufa_na_led_term_joik_guard();
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
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for sumti relative phrase; tries the structurally closed scoped-continuation route before baseline VUhO-relative ownership and the bare-VUhO extension.
    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement, normal_term) -> enum {
        /// Experimental VUhO-scoped continuation with required relatives and one required sumti continuation, reachable only immediately before explicit LUhU.
        experimental_vuho_scoped_sumti_attachment_tail,
        /// Baseline VUhO followed by a required relative-clause list.
        vuho_relative_sumti_attachment_tail,
        /// Experimental bare VUhO attachment.
        experimental_bare_vuho_sumti_attachment_tail,
    }

    /// Product node for baseline sumti relative phrase; preserves `vuho` and required `relative_clauses` in source order.
    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The `Vuho` cmavo marker.
        field vuho <- cmavo(Vuho).wf();
        /// The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `vuho_relative_sumti_attachment_tail` production.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term);
    }

    /// Product node for the camxes-exp VUhO-scoped continuation; preserves `vuho`, required `relative_clauses`, and required `sumti_connection` in source order.
    rule "sumti relative phrase" experimental_vuho_scoped_sumti_attachment_tail(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The warning-gated `Vuho` marker that identifies experimental scoped ownership.
        field vuho <- cmavo(Vuho).warn(ExperimentalVuhoScopedAttachment).wf();
        /// Required relative clauses scoped together with the continuation.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term);
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
    rule "sumti" simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term) -> struct {
        /// The shared base sumti child syntax node.
        field base_sumti <- arc(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term));
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term));
    }

    /// Sum node for sumti; selects among the `sumti_base` and `quantified_sumti` forms.
    rule "sumti" sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, free_modifier, statement, normal_term) -> enum {
        /// Uses the nested `sumti_base` sum form and preserves its selected alternative.
        sumti_base,
        /// Uses the `quantified_sumti` product form, whose payload preserves `quantifier` and `inner_sumti`.
        quantified_sumti,
    }

    /// Sum node for sumti; selects among 16 forms including `scalar_negated_sumti_with_bo`, `scalar_negated_sumti`, and `lahe_sumti`.
    rule "sumti" sumti_base(sumti, sumti_base, term, subbridi, selbri, selbri_without_terminal_relative, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement, description_relative_subbridi, description_relative_statement, normal_term) -> enum {
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
        /// Uses the `bridi_description_sumti` product form, whose payload preserves `lohoi`, `additional_heads`, `statement`, and `kuhau`.
        bridi_description_sumti,
        /// Uses the `name_sumti` product form, whose payload preserves `la`, `relative_clauses`, and `names`.
        name_sumti,
        /// Uses the `description_connection_sumti` product form, whose payload preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku`.
        description_connection_sumti,
        /// Uses the `descriptor_with_outer_quantifier_sumti` product form, whose payload preserves `outer_quantifier`, `description`, `tail`, and `ku`.
        descriptor_with_outer_quantifier_sumti,
        /// Uses the `descriptor_with_gadri_sumti` product form, whose payload preserves `description`, `tail`, and `ku`.
        descriptor_with_gadri_sumti,
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

    /// Product node for quantified sumti; preserves `quantifier` and `inner_sumti` in source order.
    rule "quantified sumti" quantified_sumti(sumti_base, mekso, letter_tokens, free_modifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantified_sumti` production.
        field quantifier <- quantifier(mekso, letter_tokens, free_modifier);
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti_base);
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

    // ilmentufa's Zantufa grammars guard raw-mex quantifiers with
    // `!selbri !sumti_6`; otherwise a BY pro-sumti sentence such as
    // `my tcidu` is stolen as a raw-mex quantified description.
    alias "raw mekso quantifier guard" zantufa_raw_mekso_quantifier_guard(letter_tokens) =
        choice((
            relation_word().ignored(),
            selmaho(Goha).ignored(),
            cmavo(Ke).ignored(),
            cmavo(Me).ignored(),
            cmavo(Nuha).ignored(),
            selmaho(Se).ignored(),
            cmavo(Jai).ignored(),
            cmavo(Nu).ignored(),
            pa_word().followed_by(selmaho(Moi).ignored()).ignored(),
            selmaho(Lahe).ignored(),
            selmaho(Nahe).ignored(),
            selmaho(Lohoi).ignored(),
            word_category(ProSumti).ignored(),
            description_head().ignored(),
            selmaho(Li).ignored(),
            letter_string(letter_tokens).ignored(),
            word_category(Quote).ignored(),
            cmavo(Lu).ignored(),
        )).not();

    /// Transparent product node for quantifier; preserves the `mekso` component.
    rule "quantifier" zantufa_raw_mekso_quantifier(mekso, letter_tokens) -> struct {
        assert zantufa_raw_mekso_quantifier_guard(letter_tokens);
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
    }

    /// Transparent product node for quantifier; preserves the `mekso` component.
    rule "quantifier" zantufa_priority_raw_mekso_quantifier(mekso, letter_tokens) -> struct {
        assert zantufa_raw_mekso_quantifier_guard(letter_tokens);
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
    }

    /// Sum node for quantifier; selects among the `zantufa_priority_raw_mekso_quantifier`, `mekso_quantifier`, `pa_run_quantifier`, and `zantufa_raw_mekso_quantifier` forms.
    rule "quantifier" quantifier(mekso, letter_tokens, free_modifier) -> enum {
        /// Uses the `zantufa_priority_raw_mekso_quantifier` product form, whose payload preserves `mekso`.
        when feature(ZantufaMex) zantufa_priority_raw_mekso_quantifier,
        /// Uses the `mekso_quantifier` product form, whose payload preserves `vei`, `mekso`, and `veho`.
        mekso_quantifier,
        /// Uses the `pa_run_quantifier` product form, whose payload preserves `number` and `boi`.
        pa_run_quantifier,
        /// Uses the `zantufa_raw_mekso_quantifier` product form, whose payload preserves `mekso`.
        when feature(ZantufaMex) zantufa_raw_mekso_quantifier,
    }

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
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
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
        assert feature(ZantufaMex).not();
        /// The operator_2-width left operator.
        field left_operator <- arc(simple_mekso_operator(atomic_mekso_operator, mekso_operator));
        /// The `standard_statement_connective` connective joining the adjacent constituents of the `bound_mekso_operator` production.
        field connective <- standard_statement_connective;
        /// The optional tense modal between the connective and BO.
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
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

    /// Product node for selbri-to-operator; preserves `maho`, `selbri`, and `tehu` in source order.
    rule "selbri-to-operator" zantufa_maho_selbri_mekso_operator(selbri) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for sumti-to-operator; preserves `maho`, `sumti`, and `tehu` in source order.
    rule "sumti-to-operator" zantufa_maho_sumti_mekso_operator(sumti) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Transparent product node for connective operator; preserves the `connective` component.
    rule "connective operator" zantufa_connective_mekso_operator -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(operand_connective);
        assert !cmavo(Cu);
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
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
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
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The operand_2-width right expression child syntax node.
        field right_expression <- arc(bound_or_simple_mekso_operand);
    }

    /// Sum node for operand; selects among 12 forms including `forethought_mekso_operand`, `qualified_mekso_operand`, `scalar_negated_mekso_operand`, `lahe_qualified_mekso_operand`, and `parenthesized_mekso_operand`.
    rule "operand" simple_mekso_operand(mekso, mekso_base, mekso_operand, simple_mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator, zantufa_mex, zantufa_tcita_selci) -> enum {
        /// Uses the `forethought_mekso_operand` product form, whose payload preserves `gek`, `left_expression`, `gik`, and `right_expression`.
        forethought_mekso_operand,
        /// Uses the `qualified_mekso_operand` product form, whose payload preserves `nahe`, `bo`, `inner_expression`, and `luhu`.
        qualified_mekso_operand,
        /// Uses the `scalar_negated_mekso_operand` product form, whose payload preserves `nahe`, `inner_expression`, and `luhu`.
        scalar_negated_mekso_operand,
        /// Uses the `lahe_qualified_mekso_operand` product form, whose payload preserves `lahe`, `inner_expression`, and `luhu`.
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
    rule "forethought mex" forethought_mekso_operand(mekso_operand, simple_mekso_operand, tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The `modal_forethought_connective` forethought connective opening the paired branches of the `forethought_mekso_operand` production.
        field gek <- modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci);
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

    /// Product node for selbri operand; preserves `mohe`, `selbri`, and `tehu` in source order.
    rule "selbri operand" zantufa_selbri_mohe_mekso_operand(selbri) -> struct {
        /// The `Mohe` cmavo marker.
        field mohe <- cmavo(Mohe).warn(ExperimentalZantufaMex).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
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
        letter_string_pa_continuation,
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
        number_word_lerfu_continuation,
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

    // A right-less Zantufa connective can otherwise commit the priority choice
    // before the baseline operand parser sees its optional stag plus BO. Zantufa
    // itself rejects that continuation, so handing it back removes no sourced
    // parse and preserves the baseline grammar included in the union profile.
    alias "Zantufa priority mex bound-operand guard" zantufa_priority_mex_bound_operand_guard(tense_modal) =
        (opt(arc(tense_modal)), cmavo(Bo)).not();

    /// Sum node for mex, with baseline ownership before the warning-union Zantufa fallback.
    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts, zantufa_mex, tense_modal) -> enum {
        /// Gives the faithful Zantufa projection priority only under the meaning-changing flag.
        when feature(ZantufaMexReinterpretation) reinterpret_zantufa_mex,
        /// Gives Zantufa-only continuations priority while handing baseline surfaces back.
        when feature(ZantufaMex) zantufa_priority_mex,
        /// Uses the `infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.
        infix_mekso,
        /// Uses the `reverse_polish_mekso` product form, whose payload preserves `fuha` and `parts`.
        reverse_polish_mekso,
        /// Additive fallback for Zantufa-only surfaces in the warning union.
        when feature(ZantufaMex) zantufa_mex,
    }

    /// Transparent priority route for a Zantufa-only mex surface.
    rule "Zantufa priority mex" zantufa_priority_mex(zantufa_mex, tense_modal) -> struct {
        /// The completed Zantufa tree, rejected here when the baseline grammar owns its surface.
        field mex <- arc(
            zantufa_mex.reject_output(crate::grammar::baseline_mex::BaselineMexRejection)
        );
        assert zantufa_priority_mex_bound_operand_guard(tense_modal);
    }

    /// Transparent priority wrapper used only by the meaning-changing reinterpretation flag.
    rule "Zantufa mex reinterpretation" reinterpret_zantufa_mex(zantufa_mex) -> struct {
        /// The faithful Zantufa mex projection.
        field mex <- arc(zantufa_mex);
    }

    /// Product node for the complete Zantufa mex expression.
    rule "Zantufa mex" zantufa_mex(zantufa_mex_1, zantufa_operator) -> struct {
        /// The first mex_1 group.
        field first_expression <- arc(zantufa_mex_1);
        /// Source-ordered operator-led continuations.
        field continuations <- [zero_or_more zantufa_mex_continuation(zantufa_mex_1, zantufa_operator)];
    }

    /// Product node for a Zantufa mex continuation.
    rule "Zantufa mex continuation" zantufa_mex_continuation(zantufa_mex_1, zantufa_operator) -> struct {
        /// One or more source operators; a connected operator node is intentionally not substituted.
        field operators <- [one_or_more arc(zantufa_operator)];
        /// The optional right mex_1 group.
        field right_expression <- opt(arc(zantufa_mex_1));
    }

    /// Product node for Zantufa mex_1, including repeated BIhE tails.
    rule "Zantufa mex precedence" zantufa_mex_1(zantufa_mex_2, zantufa_operator) -> struct {
        /// The leading mex_2 group.
        field first_group <- arc(zantufa_mex_group(zantufa_mex_2));
        /// Repeated BIhE operator-sequence tails.
        field tails <- [zero_or_more zantufa_bihe_mekso_tail(zantufa_mex_2, zantufa_operator)];
    }

    /// Sum node for either Zantufa mex_1 grouping form.
    rule "Zantufa mex group" zantufa_mex_group(zantufa_mex_2) -> enum {
        /// KE-grouped one-or-more mex_2 expressions.
        zantufa_ke_grouped_mekso,
        /// A mex_2 expression with zero or more BO-linked expressions.
        zantufa_bo_grouped_mekso,
    }

    /// Product node for a KE-grouped Zantufa mex_1 group.
    rule "Zantufa KE-grouped mex" zantufa_ke_grouped_mekso(zantufa_mex_2) -> struct {
        /// The opening KE marker.
        field ke <- cmavo(Ke).warn(ExperimentalZantufaMex).wf();
        /// Non-empty source-ordered mex_2 expressions.
        field expressions <- [one_or_more arc(zantufa_mex_2)];
        /// The optional KEhE terminator.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for a BO-grouped Zantufa mex_1 group.
    rule "Zantufa BO-grouped mex" zantufa_bo_grouped_mekso(zantufa_mex_2) -> struct {
        /// The first mex_2 expression.
        field first_expression <- arc(zantufa_mex_2);
        /// Source-ordered BO continuations.
        field continuations <- [zero_or_more zantufa_bo_grouped_mekso_continuation(zantufa_mex_2)];
    }

    /// Product node for a Zantufa BO-group continuation.
    rule "Zantufa BO-grouped mex continuation" zantufa_bo_grouped_mekso_continuation(zantufa_mex_2) -> struct {
        /// The BO marker.
        field bo <- cmavo(Bo).warn(ExperimentalZantufaMex).wf();
        /// The following mex_2 expression.
        field expression <- arc(zantufa_mex_2);
    }

    /// Product node for one repeated Zantufa BIhE tail.
    rule "Zantufa mex precedence tail" zantufa_bihe_mekso_tail(zantufa_mex_2, zantufa_operator) -> struct {
        /// The BIhE marker.
        field bihe <- cmavo(Bihe).warn(ExperimentalZantufaMex).wf();
        /// One or more source operators.
        field operators <- [one_or_more arc(zantufa_operator)];
        /// The optional following group.
        field right_group <- opt(arc(zantufa_mex_group(zantufa_mex_2)));
    }

    /// Sum node for Zantufa mex_2.
    rule "Zantufa mex atom" zantufa_mex_2(zantufa_mex, zantufa_mex_2, zantufa_operand, zantufa_operator, zantufa_forethought_mekso, sumti, selbri, letter_string, letter_tokens, free_modifier) -> enum {
        /// A Zantufa operand.
        zantufa_operand,
        /// A Zantufa reverse-Polish expression.
        zantufa_reverse_polish_mekso,
        /// A Zantufa operator-first forethought expression.
        zantufa_forethought_mekso,
    }

    /// Product node for reverse Polish Zantufa mex.
    rule "Zantufa reverse Polish mex" zantufa_reverse_polish_mekso(zantufa_mex_2, zantufa_operator) -> struct {
        /// The `Fuha` cmavo marker.
        field fuha <- cmavo(Fuha).warn(ExperimentalZantufaMex).wf();
        /// Non-empty ordered sequence of mex_2 expressions.
        field operands <- [one_or_more arc(zantufa_mex_2)];
        /// The following Zantufa operator.
        field operator <- arc(zantufa_operator);
        /// Ordered reverse-Polish tails.
        field tails <- [zero_or_more zantufa_reverse_polish_tail(zantufa_mex_2, zantufa_operator)];
        /// The optional `Kuhe` cmavo marker.
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Product node for a Zantufa reverse-Polish tail.
    rule "Zantufa reverse Polish mex tail" zantufa_reverse_polish_tail(zantufa_mex_2, zantufa_operator) -> struct {
        /// Ordered sequence of zero or more mex_2 expressions.
        field operands <- [zero_or_more arc(zantufa_mex_2)];
        /// The following Zantufa operator.
        field operator <- arc(zantufa_operator);
    }

    /// Product node for Zantufa operator-first forethought mex.
    rule "Zantufa forethought mex" zantufa_forethought_mekso(zantufa_mex_2, zantufa_operator, zantufa_forethought_mekso, letter_string, letter_tokens) -> struct {
        assert (letter_string(letter_tokens), opt(cmavo(Boi))).not();
        /// The optional PEhO marker.
        field peho <- opt(cmavo(Peho).warn(ExperimentalZantufaMex).wf());
        /// The leading Zantufa operator.
        field operator <- arc(zantufa_operator);
        /// Non-empty source-ordered mex_2 expressions.
        field operands <- [one_or_more arc(zantufa_mex_2)];
        /// The optional recursively nested forethought tail.
        field continuation <- opt(arc(zantufa_forethought_mekso));
        /// The optional KUhE terminator.
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Sum node for the exact Zantufa operand inventory.
    rule "Zantufa operand" zantufa_operand(zantufa_mex, zantufa_operand, sumti, selbri, letter_string, letter_tokens, free_modifier) -> enum {
        /// A number with its BOI boundary.
        number_mekso,
        /// A lerfu string with its BOI boundary.
        lerfu_string_mekso,
        /// A VEI-grouped full Zantufa mex.
        zantufa_parenthesized_mekso_operand,
        /// A MOhE selbri operand.
        zantufa_selbri_mohe_mekso_operand,
        /// A MOhE sumti operand.
        zantufa_sumti_mohe_mekso_operand,
        /// A LAhE-qualified full Zantufa mex.
        zantufa_lahe_qualified_mekso_operand,
        /// A NAhE BO-qualified full Zantufa mex.
        zantufa_nahe_bo_qualified_mekso_operand,
        /// Recursive scalar negation.
        zantufa_scalar_negated_mekso_operand,
    }

    /// Product node for a VEI-grouped Zantufa operand.
    rule "Zantufa parenthesized mex" zantufa_parenthesized_mekso_operand(zantufa_mex) -> struct {
        /// The VEI marker.
        field vei <- cmavo(Vei).warn(ExperimentalZantufaMex).wf();
        /// The full inner Zantufa mex.
        field inner_expression <- arc(zantufa_mex);
        /// The optional VEhO terminator.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// Product node for a Zantufa MOhE sumti operand.
    rule "Zantufa sumti operand" zantufa_sumti_mohe_mekso_operand(sumti) -> struct {
        /// The MOhE marker.
        field mohe <- cmavo(Mohe).warn(ExperimentalZantufaMex).wf();
        /// The wrapped sumti.
        field sumti <- arc(sumti);
        /// The optional TEhU terminator.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for a wide Zantufa LAhE-qualified operand.
    rule "Zantufa LAhE-qualified operand" zantufa_lahe_qualified_mekso_operand(zantufa_mex) -> struct {
        /// The LAhE marker.
        field lahe <- selmaho(Lahe).warn(ExperimentalZantufaMex).wf();
        /// The full inner Zantufa mex.
        field inner_expression <- arc(zantufa_mex);
        /// The optional LUhU terminator.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for a wide Zantufa NAhE BO-qualified operand.
    rule "Zantufa NAhE BO-qualified operand" zantufa_nahe_bo_qualified_mekso_operand(zantufa_mex) -> struct {
        /// The NAhE marker.
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        /// The mandatory BO marker.
        field bo <- cmavo(Bo).wf();
        /// The full inner Zantufa mex.
        field inner_expression <- arc(zantufa_mex);
        /// The optional LUhU terminator.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for recursive Zantufa scalar negation.
    rule "Zantufa scalar-negated operand" zantufa_scalar_negated_mekso_operand(zantufa_operand) -> struct {
        /// The NAhE marker.
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        /// The recursively nested Zantufa operand.
        field inner_expression <- arc(zantufa_operand);
    }

    /// Sum node for the exact Zantufa operator inventory.
    rule "Zantufa operator" zantufa_operator(zantufa_mex, zantufa_operator, sumti, selbri) -> enum {
        /// Recursive SE conversion.
        zantufa_converted_mekso_operator,
        /// Recursive NAhE scalar negation.
        zantufa_scalar_negated_mekso_operator,
        /// MAhO wrapping a full Zantufa mex.
        zantufa_maho_mekso_operator,
        /// MAhO wrapping a selbri.
        zantufa_maho_selbri_mekso_operator,
        /// MAhO wrapping a sumti.
        zantufa_maho_sumti_mekso_operator,
        /// A primitive VUhU operator.
        zantufa_primitive_mekso_operator,
        /// A joik or ek connective operator, excluding CU.
        zantufa_connective_mekso_operator,
    }

    /// Product node for recursive Zantufa SE conversion.
    rule "Zantufa converted operator" zantufa_converted_mekso_operator(zantufa_operator) -> struct {
        /// The SE marker.
        field se <- selmaho(Se).warn(ExperimentalZantufaMex).wf();
        /// The recursively nested operator.
        field inner_operator <- arc(zantufa_operator);
    }

    /// Product node for recursive Zantufa NAhE negation.
    rule "Zantufa scalar-negated operator" zantufa_scalar_negated_mekso_operator(zantufa_operator) -> struct {
        /// The NAhE marker.
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        /// The recursively nested operator.
        field inner_operator <- arc(zantufa_operator);
    }

    /// Product node for MAhO wrapping a full Zantufa mex.
    rule "Zantufa mex-to-operator" zantufa_maho_mekso_operator(zantufa_mex) -> struct {
        /// The MAhO marker.
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        /// The wrapped full Zantufa mex.
        field mekso <- arc(zantufa_mex);
        /// The optional TEhU terminator.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Transparent product node for a primitive Zantufa operator.
    rule "Zantufa primitive operator" zantufa_primitive_mekso_operator -> struct {
        /// The VUhU word.
        field vuhu <- selmaho(Vuhu).warn(ExperimentalZantufaMex).wf();
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
    rule "converted sumti" lahe_sumti(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// A word from selmaho `Lahe`.
        field lahe <- selmaho(Lahe).wf();
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term));
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
    rule "scalar-negated sumti" scalar_negated_sumti_with_bo(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// Optional relative clauses attached in the standard post-BO slot before the inner sumti.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term));
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

    /// Product node for bridi description; preserves `lohoi`, `additional_heads`, `statement`, and `kuhau` in source order.
    rule "bridi description" bridi_description_sumti(statement) -> struct {
        /// A word from selmaho `Lohoi`.
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
        /// Ordered sequence of zero or more additional heads components.
        field additional_heads <- [zero_or_more lohoi_description_head_continuation()];
        #[tree_child(primary)]
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Kuhau` cmavo marker.
        field kuhau <- opt(cmavo(Kuhau).wf()).elidable_terminator(Kuhau);
    }

    /// Product node for bridi description; preserves `connective` and `lohoi` in source order.
    rule "bridi description" lohoi_description_head_continuation -> struct {
        /// The `joik_connective` connective joining the adjacent constituents of the `lohoi_description_head_continuation` production.
        field connective <- joik_connective;
        /// A word from selmaho `Lohoi`.
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
    }

    /// Transparent product node for sumti; preserves the `koha` component.
    rule "sumti" pro_sumti -> struct {
        /// The `word_category` grammar result in the `koha` structural role of the `pro_sumti` production.
        field koha <- word_category(ProSumti).wf();
    }

    /// Product node for name; preserves `la`, `relative_clauses`, and `names` in source order.
    rule "name" name_sumti(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        assert feature(Cbm).not();
        /// A word from selmaho `La`.
        field la <- selmaho(La).wf();
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement, normal_term));
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
    }

    /// Transparent product node for descriptor; preserves the `description` component.
    rule "descriptor" description_head -> struct {
        /// The required description-head word from either selmaho `Le` or selmaho `La`.
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    /// Transparent product node for descriptor connective; preserves the `connective` component.
    rule "descriptor connective" description_head_connective -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(jek_connective);
    }

    /// Product node for description; preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku` in source order.
    rule "description" description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, selbri_without_terminal_relative, text, mekso, tense_modal, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The shared leading description head child syntax node.
        field leading_description_head <- arc(description_head());
        /// The `description_head_connective` connective joining the adjacent constituents of the `description_connection_sumti` production.
        field connective <- description_head_connective();
        /// The shared trailing description head child syntax node.
        field trailing_description_head <- arc(description_head());
        /// The `description_tail` grammar result in the `tail` structural role of the `description_connection_sumti` production.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, selbri_without_terminal_relative, text, mekso, tense_modal, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `description_head` grammar result in the `description` structural role of the `descriptor_with_gadri_sumti` production.
        field description <- description_head();
        /// The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_gadri_sumti` production.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `outer_quantifier`, `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, selbri_without_terminal_relative, text, mekso, tense_modal, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `quantifier` grammar result in the `outer_quantifier` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field outer_quantifier <- quantifier(mekso, letter_tokens, free_modifier);
        /// The `description_head` grammar result in the `description` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field description <- description_head();
        /// The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_outer_quantifier_sumti` production.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `quantifier`, `selbri`, `ku`, and `relative_clauses` in source order.
    rule "description" descriptor_without_gadri_sumti(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `descriptor_without_gadri_sumti` production.
        field quantifier <- quantifier(mekso, letter_tokens, free_modifier);
        assert !selmaho(Roi);
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri: std::sync::Arc<SelbriSyntax> <- arc(choice((
            feature(ZantufaSelbriReinterpretation).ignore_then(selbri),
            selbri.followed_by(cmavo(Ku).lookahead()),
            selbri_without_terminal_relative.map_recovered_to(selbri),
        )));
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
        /// The optional relative clauses component.
        field relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Product node for description tail; preserves `leading_tail_elements` and `tail` in source order.
    rule "description tail" description_tail(sumti, sumti_base, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `leading_description_tail_elements` grammar result in the `leading_tail_elements` structural role of the `description_tail` production.
        field leading_tail_elements <- leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term);
        /// The shared tail child syntax node.
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term));
    }

    /// Sum node for description tail; selects among the `quantifier_relation_description_tail`, `quantifier_sumti_description_tail`, and `relation_description_tail` forms.
    rule "description tail" description_tail_body(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> enum {
        /// Uses the `quantifier_relation_description_tail` product form, whose payload preserves `quantifier`, `selbri`, and `relative_clauses`.
        quantifier_relation_description_tail,
        /// Uses the `quantifier_sumti_description_tail` product form, whose payload preserves `quantifier` and `sumti`.
        quantifier_sumti_description_tail,
        /// Uses the `relation_description_tail` product form, whose payload preserves `selbri` and `relative_clauses`.
        relation_description_tail,
    }

    /// Product node for description tail; preserves `tail_sumti` and `relative_clauses` in source order.
    rule "description tail" leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The optional tail sumti component.
        field tail_sumti <- opt(description_tail_sumti(sumti_base));
        /// The optional relative clauses component.
        field relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Transparent product node for description tail; preserves the `sumti` component.
    rule "description tail" description_tail_sumti(sumti_base) -> struct {
        assert !pa_word();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_base);
    }

    /// Product node for description tail; preserves `selbri` and `relative_clauses` in source order.
    rule "description tail" relation_description_tail(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The shared selbri child syntax node.
        field selbri: std::sync::Arc<SelbriSyntax> <- arc(choice((
            feature(ZantufaSelbriReinterpretation).ignore_then(selbri),
            selbri_without_terminal_relative.map_recovered_to(selbri),
        )));
        /// The optional relative clauses component.
        field relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Product node for description tail; preserves `quantifier`, `selbri`, and `relative_clauses` in source order.
    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, mekso, letter_tokens, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_relation_description_tail` production.
        field quantifier <- quantifier(mekso, letter_tokens, free_modifier);
        assert !selmaho(Roi);
        /// The shared selbri child syntax node.
        field selbri: std::sync::Arc<SelbriSyntax> <- arc(choice((
            feature(ZantufaSelbriReinterpretation).ignore_then(selbri),
            selbri_without_terminal_relative.map_recovered_to(selbri),
        )));
        /// The optional relative clauses component.
        field relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Product node for description tail; preserves `quantifier` and `sumti` in source order.
    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens, free_modifier) -> struct {
        /// The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_sumti_description_tail` production.
        field quantifier <- quantifier(mekso, letter_tokens, free_modifier);
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Sum node for quote; selects among 6 forms including `experimental_mehoi_compound_quote`, `experimental_zohoi_compound_quote`, and `experimental_rahoi_compound_quote`.
    rule "quote" quote(text) -> enum {
        /// Uses the `experimental_mehoi_compound_quote` product form, whose payload preserves `quote`.
        experimental_mehoi_compound_quote,
        /// Uses the `experimental_zohoi_compound_quote` product form, whose payload preserves `quote`.
        experimental_zohoi_compound_quote,
        /// Uses the `experimental_rahoi_compound_quote` product form, whose payload preserves `quote`.
        experimental_rahoi_compound_quote,
        /// Uses the `experimental_gohoi_compound_quote` product form, whose payload preserves `quote`.
        experimental_gohoi_compound_quote,
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
    rule "quote" experimental_mehoi_compound_quote -> struct {
        /// The `quote_marker` grammar result in the `quote` structural role of the `experimental_mehoi_compound_quote` production.
        field quote <- quote_marker(Mehoi).warn(ExperimentalMehOiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_zohoi_compound_quote -> struct {
        /// The selected grammar alternative in the `quote` structural role of the `experimental_zohoi_compound_quote` production.
        field quote <- choice((
            quote_marker(Zohoi),
            quote_marker(Lahoi),
        )).warn(ExperimentalZohOiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_rahoi_compound_quote -> struct {
        /// The `quote_marker` grammar result in the `quote` structural role of the `experimental_rahoi_compound_quote` production.
        field quote <- quote_marker(Rahoi).warn(ExperimentalZantufaRahoiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_gohoi_compound_quote -> struct {
        /// The selected grammar alternative in the `quote` structural role of the `experimental_gohoi_compound_quote` production.
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" generic_compound_quote -> struct {
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
    rule "vocative phrase" selbri_vocative_sumti(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri: std::sync::Arc<SelbriSyntax> <- arc(choice((
            feature(ZantufaSelbriReinterpretation).ignore_then(selbri),
            selbri_without_terminal_relative.map_recovered_to(selbri),
        )));
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Product node for vocative phrase; preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses` in source order.
    rule "vocative phrase" cmevla_vocative_sumti(sumti, subbridi, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term));
    }

    /// Sum node for vocative phrase; selects among the `selbri_vocative_sumti`, `cmevla_vocative_sumti`, and `sumti` forms.
    rule "vocative phrase" vocative_sumti(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> enum {
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

    /// Sum node for free modifier; selects among 9 forms including `text_replacement_free_modifier`, `zantufa_sei_statement_free_modifier`, and `sei_free_modifier`.
    rule "free modifier" free_modifier(sumti, subbridi, selbri, selbri_without_terminal_relative, text, mekso, zantufa_mex_2, term, tense_modal, letter_tokens, letter_string, free_modifier, statement, description_relative_subbridi, description_relative_statement, normal_term) -> enum {
        /// Uses the nested `text_replacement_free_modifier` sum form and preserves its selected alternative.
        text_replacement_free_modifier,
        /// Uses the `zantufa_sei_statement_free_modifier` product form, whose payload preserves `sei`, `statement`, and `sehu`.
        when feature(ZantufaTerms) zantufa_sei_statement_free_modifier,
        /// Uses the `sei_free_modifier` product form, whose payload preserves `sei`, `terms`, `cu`, `selbri`, and `sehu`.
        sei_free_modifier,
        /// Uses the nested `xi_free_modifier` sum form and preserves its selected alternative.
        xi_free_modifier,
        /// Uses the `mai_free_modifier` product form, whose payload preserves `number` and `mai`.
        mai_free_modifier,
        /// Uses the `zantufa_mekso_mai_free_modifier` product form, whose payload preserves `expression` and `mai`.
        when feature(ZantufaMex) zantufa_mekso_mai_free_modifier,
        /// Uses the `soi_free_modifier` product form, whose payload preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu`.
        soi_free_modifier,
        /// Uses the `parenthetical_text` product form, whose payload preserves `to`, `text`, and `toi`.
        parenthetical_text,
        /// Uses the `vocative_free_modifier` product form, whose payload preserves `vocative_markers`, `sumti`, and `dohu`.
        vocative_free_modifier,
    }

    /// Product node for vocative phrase; preserves `vocative_markers`, `sumti`, and `dohu` in source order.
    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        /// The `vocative_marker_words` grammar result in the `vocative_markers` structural role of the `vocative_free_modifier` production.
        field vocative_markers <- vocative_marker_words().wf_when(UnrestrictedFree);
        /// The optional sumti component.
        field sumti <- opt(arc(vocative_sumti(sumti, subbridi, selbri, selbri_without_terminal_relative, tense_modal, statement, description_relative_subbridi, description_relative_statement, normal_term)));
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

    /// Product node for metalinguistic comment; preserves `sei`, `statement`, and `sehu` in source order.
    rule "metalinguistic comment" zantufa_sei_statement_free_modifier(statement) -> struct {
        /// A word from selmaho `Sei`.
        field sei <- selmaho(Sei).warn(ExperimentalZantufaStatementFreeModifier).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    /// Sum node for subscript; preserves standard ownership before the Zantufa mex_2 extension.
    rule "subscript" xi_free_modifier(mekso, zantufa_mex_2, letter_tokens, letter_string, free_modifier) -> enum {
        /// Uses the `xi_number_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_number_free_modifier,
        /// Uses the `xi_lerfu_string_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_lerfu_string_free_modifier,
        /// Uses the `xi_parenthesized_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_parenthesized_free_modifier,
        /// Uses an exact Zantufa mex_2 subscript only after all standard routes fail.
        when feature(ZantufaMex) zantufa_mex_2_xi_free_modifier,
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

    /// Product node for a Zantufa mex_2 subscript.
    rule "subscript" zantufa_mex_2_xi_free_modifier(zantufa_mex_2) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The exact Zantufa mex_2 payload.
        field expression <- arc(zantufa_mex_2);
    }

    /// Product node for utterance ordinal; preserves `number` and `mai` in source order.
    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        /// The `number_or_letter_words` grammar result in the `number` structural role of the `mai_free_modifier` production.
        field number <- number_or_letter_words(letter_tokens, letter_string)
            .followed_by(selmaho(Mai).ignored());
        /// A word from selmaho `Mai`.
        field mai <- selmaho(Mai).wf();
    }

    /// Product node for utterance ordinal; preserves `expression` and `mai` in source order.
    rule "utterance ordinal" zantufa_mekso_mai_free_modifier(zantufa_mex_2) -> struct {
        /// The exact Zantufa mex_2 payload, accepted only when immediately followed by a MAI-family word.
        field expression <- arc(zantufa_mex_2.followed_by(selmaho(Mai).ignored()));
        /// A word from selmaho `Mai`.
        field mai <- selmaho(Mai).warn(ExperimentalZantufaMex).wf();
    }

    /// Product node for reciprocal; preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu` in source order.
    rule "reciprocal" soi_free_modifier(sumti) -> struct {
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
    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal, statement, normal_term) -> enum {
        /// Uses the ownership-filtered camxes-exp continuation route.
        relative_clause_exp_continuation,
        /// Uses the `joined_relative_clause_tail` product form, whose payload preserves `zihe` and `inner`.
        joined_relative_clause_tail,
        /// Uses a warning-gated bare adjacent relative clause.
        when feature(ZantufaTerms) zantufa_bare_relative_clause_tail,
    }

    /// A bare adjacent relative clause continuation from rolling Zantufa.
    rule "Zantufa bare relative clause continuation" zantufa_bare_relative_clause_tail(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        #[tree_child(primary)]
        /// The adjacent relative clause, warned at its leading marker.
        field inner <- arc(
            relative_clause_atom(sumti, subbridi, tense_modal, statement, normal_term)
        );
    }

    /// Transparent ownership wrapper for a camxes-exp relative-clause continuation.
    rule "relative clause" relative_clause_exp_continuation(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        #[tree_child(primary)]
        /// The completed continuation, retained only when baseline ZIhE does not own its identical extent.
        field continuation <- arc(
            exp_relative_continuation(sumti, subbridi, tense_modal, statement, normal_term)
                .reject_output(crate::grammar::baseline_relative::BaselineRelativeContinuationRejection)
        );
    }

    /// Product node for relative clause; preserves `zihe` and `inner` in source order.
    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The `Zihe` cmavo marker.
        field zihe <- cmavo(Zihe).wf();
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement, normal_term));
    }

    /// Product node for the camxes-exp relative-clause continuation; preserves `connective` and `inner` in source order.
    rule "relative clause" exp_relative_continuation(sumti, subbridi, tense_modal, statement, normal_term) -> struct {
        /// The camxes-exp connective joining the adjacent relative clauses.
        field connective <- exp_relative_clause_connective;
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement, normal_term));
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
    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal, statement, normal_term) -> enum {
        /// Uses the `sumti_association_relative_clause` product form, whose payload preserves `association_marker`, `sumti`, and `gehu`.
        sumti_association_relative_clause,
        /// Uses the nested `bridi_relative_clause` sum form and preserves its selected alternative.
        bridi_relative_clause,
    }

    /// Product node for sumti association phrase; preserves `association_marker`, `sumti`, and `gehu` in source order.
    ///
    /// The payload is the shared normal-flavour term constituent, which is what all three sources
    /// spell here: `relative_clause_1 <- GOI_clause free* nonabs_term GEhU?` (camxes.peg:168),
    /// `GOI_clause free* term GEhU?` (camxes-exp.peg:207) and `GOI_clause term GEhU?`
    /// (zantufa-1.9999.peg:43). It is deliberately ONE term rather than a `terms` run: on
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

    /// Sum node for relative bridi; selects among the `zantufa_restrictive_statement_relative_clause`, `zantufa_incidental_statement_relative_clause`, `restrictive_bridi_relative_clause`, and `incidental_bridi_relative_clause` forms.
    rule "relative bridi" bridi_relative_clause(subbridi, statement) -> enum {
        /// Uses the `zantufa_restrictive_statement_relative_clause` product form, whose payload preserves `poi`, `statement`, and `kuho`.
        when feature(ZantufaTerms) zantufa_restrictive_statement_relative_clause,
        /// Uses the `zantufa_incidental_statement_relative_clause` product form, whose payload preserves `noi`, `statement`, and `kuho`.
        when feature(ZantufaTerms) zantufa_incidental_statement_relative_clause,
        /// Uses the `restrictive_bridi_relative_clause` product form, whose payload preserves `poi`, `subbridi`, and `kuho`.
        restrictive_bridi_relative_clause,
        /// Uses the `incidental_bridi_relative_clause` product form, whose payload preserves `noi`, `subbridi`, and `kuho`.
        incidental_bridi_relative_clause,
    }

    /// Product node for relative clause; preserves `poi`, `statement`, and `kuho` in source order.
    rule "relative clause" zantufa_restrictive_statement_relative_clause(statement) -> struct {
        /// The selected grammar alternative in the `poi` structural role of the `zantufa_restrictive_statement_relative_clause` production.
        field poi <- choice((
            cmavo(Poi),
            cmavo(Pohoi),
            cmavo(Voi),
            cmavo(Voihi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `noi`, `statement`, and `kuho` in source order.
    rule "relative clause" zantufa_incidental_statement_relative_clause(statement) -> struct {
        /// The selected grammar alternative in the `noi` structural role of the `zantufa_incidental_statement_relative_clause` production.
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `poi`, `subbridi`, and `kuho` in source order.
    rule "relative clause" restrictive_bridi_relative_clause(subbridi, statement) -> struct {
        /// The selected grammar alternative in the `poi` structural role of the `restrictive_bridi_relative_clause` production.
        field poi <- choice((
            cmavo(Poi),
            cmavo(Pohoi),
            cmavo(Voi),
            cmavo(Voihi),
        )).wf();
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `noi`, `subbridi`, and `kuho` in source order.
    rule "relative clause" incidental_bridi_relative_clause(subbridi, statement) -> struct {
        /// The selected grammar alternative in the `noi` structural role of the `incidental_bridi_relative_clause` production.
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).wf();
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
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

    /// Sum node for joik. Baseline paired intervals retain priority; Zantufa-only
    /// leading and trailing shapes are then tried before locally successful simple arms.
    rule "joik" joik_connective -> enum {
        /// Uses the `closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.
        closed_interval_connective,
        /// Zantufa JOIK beginning with GAhO; paired GAhO+BIhI was already claimed above.
        when feature(ZantufaConnectives) zantufa_gaho_joik_connective,
        /// Zantufa JOIK whose required right GAhO must be consumed before a simple arm can commit.
        when feature(ZantufaConnectives) zantufa_right_gaho_joik_connective,
        /// Zantufa JOIK beginning with explicit NA.
        when feature(ZantufaConnectives) zantufa_na_joik_connective,
        /// Uses the `joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        joi_connective,
        /// Uses the `simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        simple_interval_connective,
    }

    /// Zantufa GAhO-led JOIK over the representable JOI/BIhI inventory.
    rule "Zantufa joik" zantufa_gaho_joik_connective -> struct {
        /// Required left endpoint marker, which also owns the experimental warning.
        field left_gaho <- selmaho(Gaho).warn(ExperimentalZantufaGek).wf();
        /// Optional explicit left negation after the endpoint marker.
        field na <- opt(selmaho(Na).wf());
        /// Optional member reversal.
        field se <- opt(selmaho(Se).wf());
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory: jbotci JOI plus BIhI.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi))).wf();
        /// Optional independent right endpoint marker.
        field right_gaho <- opt(selmaho(Gaho).wf());
    }

    /// Zantufa NA-led JOIK. Term consumers reject this completed typed variant
    /// to preserve the successful baseline `term NA JOI term` grouping.
    rule "Zantufa joik" zantufa_na_joik_connective -> struct {
        /// Required explicit left negation, which also owns the experimental warning.
        field na <- selmaho(Na).warn(ExperimentalZantufaGek).wf();
        /// Optional member reversal.
        field se <- opt(selmaho(Se).wf());
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory: jbotci JOI plus BIhI.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi))).wf();
        /// Optional independent right endpoint marker.
        field right_gaho <- opt(selmaho(Gaho).wf());
    }

    /// Zantufa JOIK with a required right endpoint and no Zantufa-only prefix.
    rule "Zantufa joik" zantufa_right_gaho_joik_connective -> struct {
        /// Optional member reversal.
        field se <- opt(selmaho(Se).wf());
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory: jbotci JOI plus BIhI.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi))).wf();
        /// Required right endpoint marker, which owns the experimental warning.
        field right_gaho <- selmaho(Gaho).warn(ExperimentalZantufaGek).wf();
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

    /// Sum node for selbri connective; selects among the `joik_connective`, `jek_connective`, `ek_connective`, and `vuhu_nonlogical_connective` forms.
    rule "selbri connective" relation_afterthought_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.
        vuhu_nonlogical_connective,
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
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
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
        /// Zantufa paragraph JOIK beginning with GAhO.
        when feature(ZantufaConnectives) paragraph_zantufa_gaho_joik_connective,
        /// Zantufa paragraph JOIK whose required right GAhO precedes simple ownership.
        when feature(ZantufaConnectives) paragraph_zantufa_right_gaho_joik_connective,
        /// Zantufa paragraph JOIK beginning with explicit NA.
        when feature(ZantufaConnectives) paragraph_zantufa_na_joik_connective,
        /// Uses the `paragraph_joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        paragraph_joi_connective,
        /// Uses the `paragraph_simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        paragraph_simple_interval_connective,
        /// Uses the `paragraph_jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        paragraph_jek_connective,
    }

    /// Paragraph form of a Zantufa GAhO-led JOIK.
    rule "Zantufa joik" paragraph_zantufa_gaho_joik_connective -> struct {
        /// Required left endpoint marker.
        field left_gaho <- selmaho(Gaho).warn(ExperimentalZantufaGek);
        /// Optional explicit left negation.
        field na <- opt(selmaho(Na));
        /// Optional member reversal.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi)));
        /// Optional independent right endpoint marker.
        field right_gaho <- opt(selmaho(Gaho));
    }

    /// Paragraph form of a Zantufa NA-led JOIK.
    rule "Zantufa joik" paragraph_zantufa_na_joik_connective -> struct {
        /// Required explicit left negation.
        field na <- selmaho(Na).warn(ExperimentalZantufaGek);
        /// Optional member reversal.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi)));
        /// Optional independent right endpoint marker.
        field right_gaho <- opt(selmaho(Gaho));
    }

    /// Paragraph form of a Zantufa right-GAhO-only JOIK.
    rule "Zantufa joik" paragraph_zantufa_right_gaho_joik_connective -> struct {
        /// Optional member reversal.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// Audited representable rolling JOI inventory.
        field joiz <- choice((selmaho(Joi), selmaho(Bihi)));
        /// Required independent right endpoint marker.
        field right_gaho <- selmaho(Gaho).warn(ExperimentalZantufaGek);
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

    /// Sum node for bridi tail connective; selects among the `gihek_connective` and `relation_connective_as_bridi_tail` forms.
    rule "bridi tail connective" bridi_tail_connective -> enum {
        /// Uses the `gihek_connective` product form, whose payload preserves `na`, `se`, `giha`, and `nai`.
        gihek_connective,
        /// Uses the `relation_connective_as_bridi_tail` product form, whose payload preserves `connective`.
        relation_connective_as_bridi_tail,
    }

    /// Transparent product node for bridi tail connective; preserves the `connective` component.
    rule "bridi tail connective" relation_connective_as_bridi_tail -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(relation_afterthought_connective);
    }

    /// Forethought connective family with baseline and structurally disjoint Zantufa BO arms.
    rule "forethought connective" modal_forethought_connective(tense_modal, selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// Zantufa GA form with required BO and no structural NAI.
        when feature(ZantufaConnectives) zantufa_ga_bo_forethought_connective,
        /// Uses the `ga_forethought_connective` product form, whose payload preserves `se`, `ga`, and `nai`.
        ga_forethought_connective,
        /// Uses the `joik_jek_gi_forethought_connective` product form, whose payload preserves `connective`, `gi`, and `bo`.
        joik_jek_gi_forethought_connective,
        /// Uses the `jek_gi_forethought_connective` product form, whose payload preserves `na`, `se`, `ja`, and 3 other fields.
        jek_gi_forethought_connective,
        /// Zantufa tag-GI form with required BO and no structural NAI.
        when feature(ZantufaConnectives) zantufa_modal_gi_bo_forethought_connective,
        /// Uses the `modal_gi_forethought_connective` product form, whose payload preserves `tense_modal`, `gi`, and `nai`.
        modal_gi_forethought_connective,
        /// Uses the `zantufa_initial_gi_forethought_connective` product form, whose payload preserves `gi`, `tail`, and `bo`.
        when feature(ZantufaConnectives) zantufa_initial_gi_forethought_connective,
        /// Zantufa GI-first opening whose tail is a whole rolling-Zantufa tag.
        when feature(ZantufaConnectives) zantufa_initial_gi_tag_forethought_connective,
    }

    /// Zantufa GA opening with required BO. Splitting this from the baseline
    /// NAI-bearing node prevents a connector node from containing both fields.
    rule "forethought connective" zantufa_ga_bo_forethought_connective -> struct {
        /// Optional member reversal.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// GA-family opening word.
        field ga <- selmaho(Ga).wf();
        /// Required Zantufa BO suffix.
        field bo <- cmavo(Bo).warn(ExperimentalZantufaGek).wf();
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

    /// Product node for forethought connective; preserves `gi`, `tail`, and `bo` in source order.
    rule "forethought connective" zantufa_initial_gi_forethought_connective -> struct {
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).warn(ExperimentalZantufaGek).wf();
        /// The shared tail child syntax node.
        field tail <- arc(standard_statement_connective);
        /// The optional `Bo` cmavo marker.
        field bo <- opt(cmavo(Bo).wf());
    }

    /// Zantufa GI-first opening with a typed whole-tag tail.
    rule "forethought connective" zantufa_initial_gi_tag_forethought_connective(selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// GI marker owning the Zantufa connective warning.
        field gi <- cmavo(Gi).warn(ExperimentalZantufaGek).wf();
        /// Whole rolling-Zantufa tag, not the cross-profile shared tag node.
        field tag <- arc(zantufa_tag(selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci));
        /// Optional BO suffix immediately after the connective cluster.
        field bo <- opt(cmavo(Bo).wf());
    }

    /// Product node for forethought connective; preserves `connective`, `gi`, and `bo` in source order.
    rule "forethought connective" joik_jek_gi_forethought_connective -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(joik_connective);
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Bo` cmavo marker from the existing Zantufa extension.
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Zantufa tag-GI opening with required BO. The separate node makes the
    /// source grammars' mutually exclusive structural NAI/BO ownership explicit.
    rule "forethought connective" zantufa_modal_gi_bo_forethought_connective(tense_modal) -> struct {
        /// Tag preceding GI.
        field tense_modal <- arc(tense_modal);
        /// GI marker after the tag.
        field gi <- cmavo(Gi).wf();
        /// Required Zantufa BO suffix.
        field bo <- cmavo(Bo).warn(ExperimentalZantufaGek).wf();
    }

    /// Product node for forethought connective; preserves `na`, `se`, `ja`, and 3 other fields in source order.
    rule "forethought connective" jek_gi_forethought_connective -> struct {
        /// The optional na component.
        field na <- opt(selmaho(Na));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Ja`.
        field ja <- selmaho(Ja).warn(ExperimentalZantufaGek).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Bo` cmavo marker.
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
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

    /// Transparent product node for forethought connective; preserves the `gi` component.
    rule "forethought connective" zantufa_extra_gik_connective -> struct {
        #[tree_child(primary)]
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).warn(ExperimentalZantufaNaryForethought).wf();
    }

    /// Transparent product node for tag; preserves the `body` component.
    rule "tag" tense_modal(selbri, sumti, mekso, zantufa_mex, zantufa_tcita_selci, letter_tokens, letter_string) -> struct {
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
            selmaho(Roi),
            cmavo(Gaihi),
            cmavo(Deiha),
        ));
        #[tree_child(primary)]
        /// The `tense_modal_body` grammar result in the `body` structural role of the `tense_modal` production.
        field body <- tense_modal_body(selbri, sumti, mekso, zantufa_mex, zantufa_tcita_selci, letter_tokens, letter_string);
    }

    /// Sum node for tag; selects among baseline/experimental arms and the whole Zantufa tag form.
    rule "tag" tense_modal_body(selbri, sumti, mekso, zantufa_mex, zantufa_tcita_selci, letter_tokens, letter_string) -> enum {
        /// Uses the `connected_tense_modal` product form, whose payload preserves `first` and `continuations`.
        connected_tense_modal,
        /// Uses the nested `tense_modal_atom` sum form and preserves its selected alternative.
        tense_modal_atom,
        /// Uses one whole rolling-Zantufa tag only after standard and camxes-exp ownership fail.
        when feature(ZantufaTags) zantufa_tag,
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
        /// Uses the nested `composite_tense` sum form and preserves its selected alternative.
        composite_tense,
        /// Uses the `fiho_tense` product form, whose payload preserves `fiho`, `selbri`, and `fehu`.
        fiho_tense,
        /// Uses the `modal_tense` product form, whose payload preserves `nahe`, `se`, `bai`, `nai`, and `ki`.
        modal_tense,
        /// Uses the `sticky_tense` product form, whose payload preserves `ki`.
        sticky_tense,
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

    /// Whole rolling-Zantufa tag: a nonempty tcita run with zero or more JOIK-linked runs.
    rule "Zantufa tag" zantufa_tag(selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> struct {
        /// The first nonempty tcita-selci run.
        field first_run <- [one_or_more arc(zantufa_tcita_selci)];
        /// Source-ordered JOIK-linked continuation runs.
        field continuations <- [zero_or_more zantufa_tag_continuation(zantufa_tcita_selci)];
    }

    /// One JOIK-linked rolling-Zantufa tag run.
    rule "Zantufa tag continuation" zantufa_tag_continuation(zantufa_tcita_selci) -> struct {
        /// The rolling JOIK connective between runs.
        field connective <- joik_connective;
        /// The following nonempty tcita-selci run.
        field run <- [one_or_more arc(zantufa_tcita_selci)];
    }

    /// Exact recursive rolling-Zantufa tcita-selci.
    rule "Zantufa tag atom" zantufa_tcita_selci(selbri, zantufa_mex, letter_tokens, zantufa_tcita_selci) -> enum {
        /// A recursive NAhE/SE-prefixed tcita-selci.
        zantufa_prefixed_tcita_selci,
        /// A member of the audited rolling BAI inventory supported by jbotci morphology.
        zantufa_bai_tcita_selci,
        /// An optional full Zantufa mex followed by ROI.
        zantufa_roi_tcita_selci,
        /// A FIhO/selbri/FEhU tcita-selci.
        zantufa_fiho_tcita_selci,
    }

    /// Recursive rolling-Zantufa NAhE/SE prefix form.
    rule "Zantufa prefixed tag atom" zantufa_prefixed_tcita_selci(zantufa_tcita_selci) -> struct {
        /// One recursive prefix followed by rolling grammar's `post_clause` boundary.
        field prefix <- choice((selmaho(Nahe), selmaho(Se))).wf();
        /// The recursively nested tcita-selci.
        field inner <- arc(zantufa_tcita_selci);
    }

    /// Rolling-Zantufa optional-mex ROI tcita-selci, split to avoid a nullable recursive cycle.
    rule "Zantufa ROI tag atom" zantufa_roi_tcita_selci(zantufa_mex, letter_tokens) -> enum {
        /// Bare ROI with the optional mex absent.
        zantufa_bare_roi_tcita_selci,
        /// A full epoch-1 Zantufa mex followed by ROI.
        zantufa_mex_roi_tcita_selci,
    }

    /// Bare rolling-Zantufa ROI tcita-selci.
    rule "Zantufa bare ROI tag atom" zantufa_bare_roi_tcita_selci -> struct {
        /// The ROI marker followed by rolling grammar's `post_clause` boundary.
        field roi <- selmaho(Roi).wf();
    }

    /// Full-mex rolling-Zantufa ROI tcita-selci.
    rule "Zantufa mex ROI tag atom" zantufa_mex_roi_tcita_selci(zantufa_mex, letter_tokens) -> struct {
        // The optional-mex source production is factored into bare and present
        // arms. Require the entire present arm in strict lookahead before the
        // recovery parser enters the mutually recursive mex/tag graph; this
        // preserves the source language while preventing missing-token recovery
        // from making the mex arm nullable at a tcita boundary.
        assert choice((
            cmavo(Ke).ignored(),
            pa_word().ignored(),
            letter_tokens.ignored(),
            cmavo(Vei).ignored(),
            cmavo(Mohe).ignored(),
            selmaho(Lahe).ignored(),
            selmaho(Nahe).ignored(),
            cmavo(Fuha).ignored(),
            cmavo(Peho).ignored(),
            selmaho(Se).ignored(),
            cmavo(Maho).ignored(),
            selmaho(Vuhu).ignored(),
            selmaho(Na).ignored(),
            selmaho(Joi).ignored(),
            selmaho(Bihi).ignored(),
            selmaho(Gaho).ignored(),
            selmaho(A).ignored(),
        )).lookahead();
        assert (zantufa_mex, selmaho(Roi)).lookahead();
        /// Full epoch-1 Zantufa mex payload.
        field expression <- arc(zantufa_mex);
        /// The ROI marker followed by rolling grammar's `post_clause` boundary.
        field roi <- selmaho(Roi).wf();
    }

    /// Rolling-Zantufa FIhO tcita-selci.
    rule "Zantufa FIhO tag atom" zantufa_fiho_tcita_selci(selbri) -> struct {
        /// FIhO marker followed by rolling grammar's `post_clause` boundary.
        field fiho <- cmavo(Fiho).wf();
        /// Ad-hoc modal selbri.
        field selbri <- arc(selbri);
        /// Optional elidable FEhU terminator, with its own rolling `post_clause` boundary.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Audited rolling-Zantufa BAI member supported by the pinned jbotci cmavo inventory.
    rule "Zantufa BAI tag atom" zantufa_bai_tcita_selci -> struct {
        /// Exact lexical member; this intentionally includes rolling repurposings such as GAIhI and DEIhA and excludes FA.
        field bai <- choice((
            cmavo(Pu), cmavo(Zi), cmavo(Zeha), cmavo(Va), cmavo(Faha),
            cmavo(Veha), cmavo(Viha), cmavo(Zaho), cmavo(Tahe),
            cmavo(Cuhe), cmavo(Ki),
            cmavo(Gaihi), cmavo(Deiha),
            cmavo(Zuhe), cmavo(Zuhau), cmavo(Zuha), cmavo(Zu), cmavo(Zohi),
            cmavo(Zoha), cmavo(Zehu), cmavo(Zeho), cmavo(Zehi), cmavo(Zehe),
            cmavo(Zau), cmavo(Zahai), cmavo(Za), cmavo(Xohu), cmavo(Xaho),
            cmavo(Vuha), cmavo(Vu), cmavo(Vihu), cmavo(Vihi), cmavo(Vihe),
            cmavo(Vi), cmavo(Vehu), cmavo(Vehi), cmavo(Vehe), cmavo(Vahu),
            cmavo(Vaho), cmavo(Tuhi), cmavo(Toho), cmavo(Tihuhi), cmavo(Tihuha),
            cmavo(Tihu), cmavo(Tihi), cmavo(Tiha), cmavo(Tehe), cmavo(Tai),
            cmavo(Tahi), cmavo(Sihu), cmavo(Sau), cmavo(Ruhu), cmavo(Ruhi),
            cmavo(Rihu), cmavo(Rihi), cmavo(Riha), cmavo(Reho), cmavo(Rai),
            cmavo(Rahi), cmavo(Raha), cmavo(Puho), cmavo(Puhe), cmavo(Puhau),
            cmavo(Puha), cmavo(Pohi), cmavo(Piho), cmavo(Pahu), cmavo(Paho),
            cmavo(Paha), cmavo(Nihi), cmavo(Niha), cmavo(Nehu), cmavo(Nehi),
            cmavo(Neha), cmavo(Nau), cmavo(Naho), cmavo(Muhu), cmavo(Muhai),
            cmavo(Muhi), cmavo(Mohu), cmavo(Mehe), cmavo(Meha), cmavo(Mau),
            cmavo(Mahi), cmavo(Mahe), cmavo(Lihe), cmavo(Leha), cmavo(Lahu),
            cmavo(Kuhu), cmavo(Koi), cmavo(Kohau), cmavo(Kihu), cmavo(Kihoi),
            cmavo(Kihi), cmavo(Kai), cmavo(Kahi), cmavo(Kahai), cmavo(Kaha),
            cmavo(Jihu), cmavo(Jiho), cmavo(Jihe), cmavo(Jahi), cmavo(Jahe),
            cmavo(Gau), cmavo(Gahu), cmavo(Gaha), cmavo(Fihe), cmavo(Fau),
            cmavo(Fahe), cmavo(Duhoi), cmavo(Duho), cmavo(Duhi), cmavo(Duha),
            cmavo(Dohe), cmavo(Diho), cmavo(Dihi), cmavo(Diha), cmavo(Dehihu),
            cmavo(Dehiho), cmavo(Dehihi), cmavo(Dehihe), cmavo(Dehiha),
            cmavo(Dehi), cmavo(Deha), cmavo(Cuhu), cmavo(Cohu), cmavo(Cohi),
            cmavo(Coha), cmavo(Cihu), cmavo(Ciho), cmavo(Cihe), cmavo(Cau),
            cmavo(Cahu), cmavo(Caho), cmavo(Cahi), cmavo(Ca), cmavo(Buhu),
            cmavo(Behi), cmavo(Behei), cmavo(Behau), cmavo(Beha), cmavo(Bau),
            cmavo(Bai), cmavo(Baho), cmavo(Bahi), cmavo(Bahau), cmavo(Ba),
        )).wf();
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

    /// Sum node for selbri; gives the full-operand Zantufa CEI owner first
    /// refusal before the standard tagged and untagged owners.
    rule "selbri" selbri(selbri, co_selbri, cei_free_co_selbri, sumti, subbridi, tense_modal, statement, free_modifier, description_relative_subbridi, description_relative_statement, normal_term) -> enum {
        /// Faithful full-selbri CEI ownership selected by the meaning-changing flag.
        when feature(ZantufaSelbriReinterpretation) reinterpret_zantufa_assigned_selbri,
        /// Rolling-Zantufa selbri-level relative attachment.
        when feature(ZantufaTerms) zantufa_relative_selbri,
        /// A Zantufa CEI chain whose assignments take full selbri operands.
        when feature(ZantufaTerms) zantufa_priority_assigned_selbri,
        /// Uses the `tagged_selbri` product form, whose payload preserves `tense_modal` and `inner_selbri`.
        tagged_selbri,
        /// Uses the nested `untagged_selbri` sum form and preserves its selected alternative.
        untagged_selbri,
    }

    /// Transparent priority wrapper that bypasses the baseline classifier only
    /// under the explicit meaning-changing reinterpretation flag.
    rule "Zantufa reinterpreted assigned selbri" reinterpret_zantufa_assigned_selbri(selbri, cei_free_co_selbri) -> struct {
        assert feature(ZantufaTerms);
        #[tree_child(primary)]
        /// The faithful rolling-Zantufa assignment candidate.
        field selbri <- arc(zantufa_assigned_selbri(selbri, cei_free_co_selbri));
    }

    /// Rolling-Zantufa relative attachment at selbri level, before any CEI
    /// assignments in source order.
    rule "Zantufa relative selbri" zantufa_relative_selbri(selbri, cei_free_co_selbri, sumti, tense_modal, description_relative_subbridi, description_relative_statement, normal_term) -> struct {
        assert feature(ZantufaTerms);
        /// The level-2 selbri receiving the relative clause list.
        field leading_selbri <- arc(cei_free_co_selbri);
        /// The warning-bearing selbri-level relative clause list.
        field relative_clauses <- arc(
            bare_continuable_relative_clause_list(sumti, description_relative_subbridi, tense_modal, description_relative_statement, normal_term)
        );
        /// Zero or more following full-selbri CEI assignments.
        field assignments <- [zero_or_more zantufa_selbri_assignment(selbri)];
    }

    /// Transparent priority wrapper that returns completed shared surfaces to
    /// the standard selbri owner.
    rule "Zantufa priority assigned selbri" zantufa_priority_assigned_selbri(selbri, cei_free_co_selbri) -> struct {
        #[tree_child(primary)]
        /// The completed assignment candidate after baseline-ownership filtering.
        field selbri <- arc(
            zantufa_assigned_selbri(selbri, cei_free_co_selbri)
                .reject_output(crate::grammar::baseline_selbri::BaselineSelbriAssignmentRejection)
        );
    }

    /// Zantufa selbri-level pro-bridi assignment. This arm is deliberately
    /// extension-first: the completed candidate classifier returns shared
    /// same-extent surfaces to the standard CEI owner.
    rule "Zantufa assigned selbri" zantufa_assigned_selbri(selbri, cei_free_co_selbri) -> struct {
        /// The level-2 selbri to which the assignments apply.
        field leading_selbri <- arc(cei_free_co_selbri);
        /// One or more source-ordered full-selbri assignments.
        field assignments <- [one_or_more zantufa_selbri_assignment(selbri)];
    }

    /// Description-boundary CEI chain. Earlier operands are full selbri; the
    /// final operand retains the no-terminal-relative boundary recursively.
    rule "Zantufa assigned selbri without terminal relative" zantufa_assigned_selbri_without_terminal_relative(selbri, selbri_without_terminal_relative, cei_free_co_selbri) -> struct {
        /// The level-2 selbri to which the assignments apply.
        field leading_selbri <- arc(cei_free_co_selbri);
        /// Full operands before the final assignment remain unrestricted.
        field preceding_assignments <- [zero_or_more zantufa_selbri_assignment(selbri).followed_by(cmavo(Cei).lookahead())];
        /// The final assignment follows the restricted right spine.
        field final_assignment <- zantufa_selbri_assignment_without_terminal_relative(selbri_without_terminal_relative);
    }

    /// Consumer-specific selbri entry that preserves CEI repetition while
    /// making terminal selbri-relative attachment unavailable at this boundary.
    rule "selbri without terminal relative" selbri_without_terminal_relative(selbri, selbri_without_terminal_relative, co_selbri, cei_free_co_selbri, tense_modal, statement, free_modifier) -> enum {
        /// A filtered full-selbri CEI chain whose final operand stays restricted.
        when feature(ZantufaTerms) zantufa_priority_assigned_selbri_without_terminal_relative,
        /// A tagged selbri whose recursive right edge stays restricted.
        tagged_selbri_without_terminal_relative,
        /// An untagged selbri whose NA right edge stays restricted.
        untagged_selbri_without_terminal_relative,
    }

    /// Priority wrapper for a description-boundary CEI chain.
    rule "Zantufa priority assigned selbri without terminal relative" zantufa_priority_assigned_selbri_without_terminal_relative(selbri, selbri_without_terminal_relative, cei_free_co_selbri) -> struct {
        #[tree_child(primary)]
        /// The completed candidate after baseline-ownership filtering.
        field selbri <- arc(
            zantufa_assigned_selbri_without_terminal_relative(
                selbri,
                selbri_without_terminal_relative,
                cei_free_co_selbri,
            ).reject_output(crate::grammar::baseline_selbri::RestrictedBaselineSelbriAssignmentRejection)
        );
    }

    /// Tagged description-boundary selbri.
    rule "tagged selbri without terminal relative" tagged_selbri_without_terminal_relative(selbri_without_terminal_relative, co_selbri, tense_modal) -> struct {
        /// The leading tense/modal tag.
        field tense_modal <- arc(tense_modal);
        /// The restricted untagged inner selbri.
        field inner_selbri <- arc(untagged_selbri_without_terminal_relative(selbri_without_terminal_relative, co_selbri));
    }

    /// Untagged description-boundary selbri.
    rule "untagged selbri without terminal relative" untagged_selbri_without_terminal_relative(selbri_without_terminal_relative, co_selbri) -> enum {
        /// NA followed by another restricted selbri.
        negated_selbri_without_terminal_relative,
        /// The ordinary level-2 selbri base.
        co_selbri,
    }

    /// NA recursion that retains the description boundary on its right edge.
    rule "negated selbri without terminal relative" negated_selbri_without_terminal_relative(selbri_without_terminal_relative) -> struct {
        /// The NA marker.
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
        /// The recursively restricted inner selbri.
        field inner_selbri <- arc(selbri_without_terminal_relative);
    }

    /// One full-selbri Zantufa CEI assignment.
    rule "Zantufa selbri assignment" zantufa_selbri_assignment(selbri) -> struct {
        assert feature(ZantufaTerms);
        /// The warning-bearing CEI marker.
        field cei <- cmavo(Cei).warn(ExperimentalZantufaSelbriAssignment).wf();
        /// The full following selbri operand.
        field selbri <- arc(selbri);
    }

    /// One Zantufa CEI assignment whose operand retains the description boundary.
    rule "Zantufa selbri assignment without terminal relative" zantufa_selbri_assignment_without_terminal_relative(selbri_without_terminal_relative) -> struct {
        assert feature(ZantufaTerms);
        /// The warning-bearing CEI marker.
        field cei <- cmavo(Cei).warn(ExperimentalZantufaSelbriAssignment).wf();
        /// The restricted following selbri operand.
        field selbri <- arc(selbri_without_terminal_relative);
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

    // The rolling-Zantufa CEI owner sits outside selbri level 2. Its leading
    // operand therefore uses the standard rebuilt ladder with only the legacy
    // tanru-unit CEI repetition removed. Nested explicit groups still use the
    // ordinary grammar supplied by tanru_unit_atom; only CEI at this ladder's
    // own unit boundary is left for zantufa_assigned_selbri.
    alias "selbri" cei_free_co_selbri(cei_free_co_selbri, cei_free_tanru_selbri, statement, free_modifier) =
        memo_scope(
            CeiFree,
            co_selbri(cei_free_co_selbri, cei_free_tanru_selbri, statement, free_modifier),
        ).recursive_output(cei_free_co_selbri);

    alias "tanru" cei_free_tanru_selbri(cei_free_connected_selbri) =
        tanru_selbri(cei_free_connected_selbri).recursive_output(cei_free_tanru_selbri);

    alias "selbri connection" cei_free_connected_selbri(
        cei_free_bound_selbri,
        cei_free_tanru_selbri,
        tense_modal,
        free_modifier,
    ) = connected_selbri(
        cei_free_bound_selbri,
        cei_free_tanru_selbri,
        tense_modal,
        free_modifier,
    ).recursive_output(cei_free_connected_selbri);

    alias "BO-bound selbri" cei_free_bound_selbri(
        cei_free_bound_selbri,
        cei_free_plain_bo_selbri,
        tense_modal,
        free_modifier,
    ) = bound_selbri(
        cei_free_bound_selbri,
        cei_free_plain_bo_selbri,
        tense_modal,
        free_modifier,
    ).recursive_output(cei_free_bound_selbri);

    alias "plain BO selbri" cei_free_plain_bo_selbri(
        cei_free_plain_bo_selbri,
        cei_free_tanru_unit,
        selbri,
        cei_free_co_selbri,
        free_modifier,
    ) = plain_bo_selbri(
        cei_free_plain_bo_selbri,
        cei_free_tanru_unit,
        selbri,
        cei_free_co_selbri,
        free_modifier,
    ).recursive_output(cei_free_plain_bo_selbri);

    alias "tanru unit" cei_free_tanru_unit(
        tanru_unit_atom,
        sumti,
        tense_modal,
        statement,
        selbri,
        forethought_bridi_connection,
        tanru_unit,
    normal_term,
    ) = linked_tanru_unit(
        tanru_unit_atom,
        sumti,
        tense_modal,
        statement,
        selbri,
        forethought_bridi_connection,
    normal_term,
    ).map_to(tanru_unit);

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
    }

    /// Product node for an ordinary level-4 selbri continuation.
    rule "selbri connection continuation" simple_connected_selbri_continuation(bound_selbri) -> struct {
        /// The standard joik/jek selbri connective.
        field connective <- arc(selbri_afterthought_connective);
        /// The following level-5 selbri.
        field trailing_selbri <- arc(bound_selbri);
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
    rule "plain BO selbri" plain_bo_selbri(plain_bo_selbri, tanru_unit, selbri, co_selbri, free_modifier) -> enum {
        /// A CEI-capable tanru unit with an optional plain BO continuation.
        plain_bo_tanru_unit,
        /// A standard binary or structurally disjoint Zantufa forethought owner.
        forethought_selbri_connection,
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

    /// Sum node separating the standard binary owner from the two structurally
    /// disjoint Zantufa shapes.
    rule "forethought selbri connection" forethought_selbri_connection(selbri, plain_bo_selbri, co_selbri, free_modifier) -> enum {
        /// A Zantufa forethought with at least two GI branches.
        zantufa_nary_forethought_selbri_connection,
        /// A Zantufa forethought whose explicit GIhI is its disjointness marker.
        zantufa_gihi_forethought_selbri_connection,
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

    /// Product node for the first wide Zantufa GI branch.
    rule "forethought selbri connection" zantufa_first_forethought_selbri_branch(co_selbri) -> struct {
        /// The un-warned first GI-family connective.
        field gik <- gik_connective;
        /// The wide level-2 branch selbri.
        field selbri <- arc(co_selbri);
    }

    /// Product node for an additional wide Zantufa forethought branch.
    rule "forethought selbri connection" zantufa_forethought_selbri_branch(co_selbri) -> struct {
        assert feature(ZantufaConnectives);
        /// The additional GI-family connective.
        field gik <- zantufa_extra_gik_connective;
        /// The wide level-2 branch selbri.
        field selbri <- arc(co_selbri);
    }

    /// Zantufa wide forethought selected by one or more additional GI branches.
    rule "forethought selbri connection" zantufa_nary_forethought_selbri_connection(co_selbri, free_modifier) -> struct {
        assert feature(ZantufaConnectives);
        /// Optional NAhE preceding the independent free-modifier slot.
        field nahe <- opt(selmaho(Nahe));
        /// Free modifiers between NAhE (when present) and GUhA.
        field free_modifiers <- [zero_or_more free_modifier];
        /// The forethought connective opener without NAhE.
        field guhek <- guhek_connective;
        /// The wide level-2 left operand.
        field leading_selbri <- arc(co_selbri);
        /// The first wide GI branch.
        field first_branch <- zantufa_first_forethought_selbri_branch(co_selbri);
        /// One or more additional warning-bearing GI branches.
        field additional_branches <- [one_or_more zantufa_forethought_selbri_branch(co_selbri)];
        /// An optional warning-bearing explicit GIhI terminator.
        field gihi <- opt(selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)).elidable_terminator(Gihi);
    }

    /// Zantufa wide forethought selected by an explicit GIhI terminator.
    rule "forethought selbri connection" zantufa_gihi_forethought_selbri_connection(co_selbri, free_modifier) -> struct {
        assert feature(ZantufaConnectives);
        /// Optional NAhE preceding the independent free-modifier slot.
        field nahe <- opt(selmaho(Nahe));
        /// Free modifiers between NAhE (when present) and GUhA.
        field free_modifiers <- [zero_or_more free_modifier];
        /// The forethought connective opener without NAhE.
        field guhek <- guhek_connective;
        /// The wide level-2 left operand.
        field leading_selbri <- arc(co_selbri);
        /// The first wide GI branch.
        field first_branch <- zantufa_first_forethought_selbri_branch(co_selbri);
        /// The required warning-bearing explicit GIhI terminator.
        field gihi <- selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi).wf();
    }

    /// Product node for a complete tanru unit: an atom with optional linkargs,
    /// followed by zero or more CEI assignments.
    rule "tanru unit" tanru_unit(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The first linked atom.
        field base <- arc(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term));
        /// Source-ordered CEI assignments.
        field assignments <- [zero_or_more pro_bridi_tanru_unit_assignment(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term)];
    }

    /// Product node for one CEI assignment.
    rule "pro-bridi assignment" pro_bridi_tanru_unit_assignment(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The CEI marker.
        field cei <- cmavo(Cei).wf();
        /// The following linked atom.
        field tanru_unit <- arc(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term));
    }

    /// Product node for tanru unit; preserves `base` and `linkargs` in source order.
    rule "tanru unit" linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom);
        /// The optional linkargs component.
        field linkargs <- opt(linkargs(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term));
    }

    /// Product node for tanru unit; preserves `conversions` and `base` in source order.
    rule "tanru unit" tanru_unit_atom(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term) -> struct {
        /// Ordered sequence of zero or more conversions components.
        field conversions <- [zero_or_more selmaho(Se).wf()];
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom_base(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term));
    }

    /// Sum node for tanru unit; selects among the standard and gated Zantufa forms.
    rule "tanru unit" tanru_unit_atom_base(tanru_unit_atom, tanru_unit, tanru_selbri, connected_selbri, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, statement, forethought_bridi_connection, normal_term) -> enum {
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
        /// Uses the `zantufa_statement_abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei`.
        when feature(ZantufaTerms) zantufa_statement_abstraction_tanru_unit,
        /// Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.
        abstraction_tanru_unit,
        /// Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.
        sumti_selbri_tanru_unit,
        /// Uses the `zantufa_me_tanru_unit` product form, whose payload preserves `me`, `body`, `mehu`, and `moi_marker`.
        zantufa_me_tanru_unit,
        /// Uses the `zantufa_mex_moi_tanru_unit` product form, whose payload preserves `expression` and `moi`.
        zantufa_mex_moi_tanru_unit,
        /// Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.
        operator_selbri_tanru_unit,
        /// Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.
        quoted_bridi_selbri_tanru_unit,
        /// Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.
        quoted_text_selbri_tanru_unit,
        /// Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.
        text_selbri_tanru_unit,
        /// Uses the `tag_selbri_tanru_unit` product form, whose payload preserves `xohi` and `tag`.
        tag_selbri_tanru_unit,
        /// Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.
        goha_word_tanru_unit,
        /// Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.
        pro_bridi_tanru_unit,
        /// Uses a flat Zantufa KE group with one or more direct CO tails.
        when feature(ZantufaConnectives) zantufa_ke_co_grouped_tanru_unit,
        /// Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.
        grouped_tanru_unit,
    }

    /// A flat Zantufa KE group over level-3 operands. Requiring a nonempty
    /// direct CO-tail list makes the arm structurally disjoint from standard KE.
    rule "Zantufa KE/CO grouped tanru" zantufa_ke_co_grouped_tanru_unit(tanru_selbri) -> struct {
        assert feature(ZantufaConnectives);
        /// The warning-bearing KE group opener.
        field ke <- cmavo(Ke).warn(ExperimentalZantufaKeCoGrouping).wf();
        /// The first level-3 operand.
        field leading_selbri <- arc(tanru_selbri);
        /// One or more flat, source-ordered CO operands.
        field co_tails <- [one_or_more zantufa_ke_co_grouped_tanru_tail(tanru_selbri)];
        /// The optional KEhE group terminator.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// One direct CO operand in a flat Zantufa KE group.
    rule "Zantufa KE/CO grouped tanru continuation" zantufa_ke_co_grouped_tanru_tail(tanru_selbri) -> struct {
        /// The CO marker.
        field co <- cmavo(Co).wf();
        /// The following level-3 operand.
        field trailing_selbri <- arc(tanru_selbri);
    }

    /// Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.
    rule "tagged selbri" tagged_selbri_group_tanru_unit(connected_selbri, tense_modal) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(connected_selbri);
    }

    /// Product node for linked arguments; preserves `linkargs` and `base` in source order.
    rule "linked arguments" preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal, statement, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The `linkargs` grammar result in the `linkargs` structural role of the `preposed_linkargs_tanru_unit` production.
        field linkargs <- linkargs(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term);
        /// The shared base child syntax node.
        field base <- arc(tanru_unit);
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
    rule "modal conversion" jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal) -> struct {
        /// The `Jai` cmavo marker.
        field jai <- cmavo(Jai).wf();
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Sum node for modal conversion; selects among 11 forms including `converted_jai_inner_tanru_unit`, `scalar_negated_jai_inner_tanru_unit`, and `sumti_selbri_tanru_unit`.
    rule "modal conversion" jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, atomic_mekso_operator, letter_tokens, letter_string, normal_term) -> enum {
        /// Uses the `converted_jai_inner_tanru_unit` product form, whose payload preserves `se` and `inner_unit`.
        converted_jai_inner_tanru_unit,
        /// Uses the `scalar_negated_jai_inner_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.
        scalar_negated_jai_inner_tanru_unit,
        /// Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.
        sumti_selbri_tanru_unit,
        /// Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.
        quoted_bridi_selbri_tanru_unit,
        /// Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.
        quoted_text_selbri_tanru_unit,
        /// Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.
        text_selbri_tanru_unit,
        /// Uses the `grouped_jai_inner_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.
        grouped_jai_inner_tanru_unit,
        /// Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.
        ordinal_tanru_unit,
        /// Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.
        operator_selbri_tanru_unit,
        /// Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.
        pro_bridi_tanru_unit,
        /// Uses the `word_tanru_unit` product form, whose payload preserves `word`.
        word_tanru_unit,
    }

    /// Product node for converted tanru unit; preserves `se` and `inner_unit` in source order.
    rule "converted tanru unit" converted_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// A word from selmaho `Se`.
        field se <- selmaho(Se).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.
    rule "scalar-negated tanru unit" scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Transparent product node for quoted bridi selbri; preserves the `quote` component.
    rule "quoted bridi selbri" quoted_bridi_selbri_tanru_unit -> struct {
        /// The selected grammar alternative in the `quote` structural role of the `quoted_bridi_selbri_tanru_unit` production.
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Product node for text selbri; preserves `luhei`, `text`, and `lihau` in source order.
    rule "text selbri" text_selbri_tanru_unit(text) -> struct {
        /// The `Luhei` cmavo marker.
        field luhei <- cmavo(Luhei).warn(ExperimentalZantufaLuheiSelbriUnit).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Lihau` cmavo marker.
        field lihau <- opt(cmavo(Lihau).wf()).elidable_terminator(Lihau);
    }

    /// Transparent product node for quoted text selbri; preserves the `muhoi` component.
    rule "quoted text selbri" quoted_text_selbri_tanru_unit -> struct {
        /// The `delimited_quote_marker` grammar result in the `muhoi` structural role of the `quoted_text_selbri_tanru_unit` production.
        field muhoi <- delimited_quote_marker(Muhoi).warn(ExperimentalZantufaMuhoiSelbriUnit).wf();
    }

    /// Product node for tag selbri; preserves `xohi` and `tag` in source order.
    rule "tag selbri" tag_selbri_tanru_unit(tense_modal) -> struct {
        /// The `Xohi` cmavo marker.
        field xohi <- cmavo(Xohi).warn(ExperimentalXohiTagSelbri).wf();
        /// The shared tag child syntax node.
        field tag <- arc(tense_modal);
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

    /// Product node for sumti-to-selbri; preserves `me`, `body`, `mehu`, and `moi_marker` in source order.
    rule "sumti-to-selbri" zantufa_me_tanru_unit(mekso, mekso_operator, tense_modal) -> struct {
        /// The `Me` cmavo marker.
        field me <- cmavo(Me).warn(ExperimentalZantufaMex).wf();
        /// The shared body child syntax node.
        field body <- arc(zantufa_me_selbri_body(mekso, mekso_operator, tense_modal));
        /// The optional `Mehu` cmavo marker.
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        /// The optional moi marker component.
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    /// Sum node for sumti-to-selbri; selects among the `zantufa_me_operator_selbri_body`, `zantufa_me_mekso_selbri_body`, and `zantufa_me_tag_selbri_body` forms.
    rule "sumti-to-selbri" zantufa_me_selbri_body(mekso, mekso_operator, tense_modal) -> enum {
        /// Uses the `zantufa_me_operator_selbri_body` product form, whose payload preserves `operators`.
        zantufa_me_operator_selbri_body,
        /// Uses the `zantufa_me_mekso_selbri_body` product form, whose payload preserves `expression`.
        zantufa_me_mekso_selbri_body,
        /// Uses the `zantufa_me_tag_selbri_body` product form, whose payload preserves `tag`.
        zantufa_me_tag_selbri_body,
    }

    /// Transparent product node for sumti-to-selbri; preserves the `operators` component.
    rule "sumti-to-selbri" zantufa_me_operator_selbri_body(mekso_operator) -> struct {
        /// Non-empty ordered sequence of operators components.
        field operators <- [one_or_more mekso_operator];
    }

    /// Transparent product node for sumti-to-selbri; preserves the `expression` component.
    rule "sumti-to-selbri" zantufa_me_mekso_selbri_body(mekso) -> struct {
        /// The shared expression child syntax node.
        field expression <- arc(mekso);
    }

    /// Transparent product node for sumti-to-selbri; preserves the `tag` component.
    rule "sumti-to-selbri" zantufa_me_tag_selbri_body(tense_modal) -> struct {
        /// The shared tag child syntax node.
        field tag <- arc(tense_modal);
    }

    /// Product node for mex selbri; preserves `expression` and `moi` in source order.
    rule "mex selbri" zantufa_mex_moi_tanru_unit(mekso) -> struct {
        /// The required shared mekso expression parsed by `mekso`, completed immediately before the following MOI-family word.
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_before_selmaho(Moi));
        /// A word from selmaho `Moi`.
        field moi <- selmaho(Moi).warn(ExperimentalZantufaMex).wf();
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

    /// Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.
    rule "grouped tanru" grouped_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(connected_jai_inner_selbri(jai_inner_tanru_unit));
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for selbri connection; preserves `leading_selbri` and `continuations` in source order.
    rule "selbri connection" connected_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more connected_jai_inner_selbri_continuation(jai_inner_tanru_unit)];
    }

    /// Product node for selbri connection continuation; preserves `connective` and `trailing_selbri` in source order.
    rule "selbri connection continuation" connected_jai_inner_selbri_continuation(jai_inner_tanru_unit) -> struct {
        /// The `relation_afterthought_connective` connective joining the adjacent constituents of the `connected_jai_inner_selbri_continuation` production.
        field connective <- relation_afterthought_connective;
        /// The shared trailing selbri child syntax node.
        field trailing_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
    }

    /// Product node for selbri; preserves `first_unit` and `additional_units` in source order.
    rule "selbri" tanru_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        /// The initial `jai_inner_tanru_unit` constituent before the continuations of the `tanru_jai_inner_selbri` production.
        field first_unit <- jai_inner_tanru_unit;
        /// Ordered sequence of zero or more additional units components.
        field additional_units <- [zero_or_more jai_inner_tanru_unit];
    }

    /// Sum node for linked arguments; selects among the `place_tagged_linked_sumti`, `tense_tagged_linked_sumti`, `plain_linked_sumti`, and `empty_linked_sumti` forms.
    rule "linked arguments" linked_sumti(sumti, tense_modal, normal_term) -> enum {
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
        /// Uses the marker-only `empty_linked_sumti` product form.
        empty_linked_sumti,
    }

    /// The loose connection level for BE/BEI arguments in the camxes-exp term hierarchy.
    ///
    /// These leaves are listed directly so ordinary links retain their established Debug and
    /// serde shape. The binding-schema drift guard keeps them synchronized with `linked_sumti`.
    rule "linked arguments" linked_term(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> enum {
        /// Uses the diagnosed loose connection over BO-bound linked terms.
        connected_linked_term,
        /// Uses the diagnosed BO-bound linked-term connection.
        bound_linked_term_connection,
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
        /// Uses the marker-only `empty_linked_sumti` product form.
        empty_linked_sumti,
    }

    /// A hierarchy-only loose connection over linked terms with one or more continuations.
    rule "linked arguments" connected_linked_term(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The first BO-bound linked term at the loose precedence level.
        field leading_link <- arc(bound_linked_term(sumti, tense_modal, normal_term));
        /// The nonempty source-ordered loose continuation sequence.
        field continuations <- [one_or_more connected_linked_term_continuation(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term)];
    }

    /// One loose linked-term continuation.
    rule "linked arguments" connected_linked_term_continuation(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        assert term_loose_connection_guard(tense_modal, selbri, forethought_bridi_connection);
        /// The connective joining the adjacent linked terms.
        field connective <- term_afterthought_connective;
        /// The BO-bound linked term following the connective.
        field trailing_link <- arc(bound_linked_term(sumti, tense_modal, normal_term));
    }

    /// The optional-stag BO-bound level for BE/BEI arguments.
    rule "linked arguments" bound_linked_term(sumti, tense_modal, normal_term) -> enum {
        /// Uses the diagnosed BO-bound linked-term connection.
        bound_linked_term_connection,
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
    }

    /// A nonempty linked-term operand; the empty BE/BEI marker form is intentionally excluded.
    rule "linked arguments" bound_linked_term_operand(sumti, tense_modal, normal_term) -> enum {
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
    }

    /// The diagnosed BO-bound BE/BEI connection with one or more continuations.
    rule "linked arguments" bound_linked_term_connection(sumti, tense_modal, normal_term) -> struct {
        /// The first nonempty linked argument at the BO-bound precedence level.
        field leading_link <- arc(bound_linked_term_operand(sumti, tense_modal, normal_term));
        /// The nonempty source-ordered BO-bound continuation sequence.
        field continuations <- [one_or_more bound_linked_term_continuation(sumti, tense_modal, normal_term)];
    }

    /// One optional-stag BO continuation in a BE/BEI argument connection.
    rule "linked arguments" bound_linked_term_continuation(sumti, tense_modal, normal_term) -> struct {
        /// The connective joining the adjacent linked arguments.
        field connective <- term_afterthought_connective;
        /// The optional camxes-exp `stag`; unlike ordinary terms, links use the `term` flavor.
        field tense_modal <- opt(arc(tense_modal.reject_output(crate::grammar::baseline_tag::ZantufaTagRejection)));
        /// The `Bo` cmavo marker, which owns the experimental warning for the whole connection.
        field bo <- cmavo(Bo).warn(ExperimentalTermBoConnection).wf();
        /// The nonempty linked argument following BO.
        field trailing_link <- arc(bound_linked_term_operand(sumti, tense_modal, normal_term));
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

    /// Marker-only product node for linked arguments; the parser retains no public fields.
    rule "linked arguments" empty_linked_sumti -> struct {
    }

    /// Product node for linked arguments; preserves `bei` and `link` in source order.
    rule "linked arguments" bei_link(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The `Bei` cmavo marker.
        field bei <- cmavo(Bei).wf();
        /// The `linked_term` grammar result in the `link` structural role of the `bei_link` production.
        field link <- linked_term(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term);
    }

    /// Product node for linked arguments; preserves `be`, `first_link`, `bei_links`, and `beho` in source order.
    rule "linked arguments" linkargs(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term) -> struct {
        /// The `Be` cmavo marker.
        field be <- cmavo(Be).wf();
        /// The initial `linked_term` constituent before the continuations of the `linkargs` production.
        field first_link <- linked_term(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term);
        /// Ordered sequence of zero or more bei links components.
        field bei_links <- [zero_or_more bei_link(sumti, tense_modal, selbri, forethought_bridi_connection, normal_term)];
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

    /// Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei` in source order.
    rule "abstraction" zantufa_statement_abstraction_tanru_unit(statement) -> struct {
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// Ordered sequence of zero or more abstractor connections components.
        field abstractor_connections <- [zero_or_more zantufa_abstractor_connection()];
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Kei` cmavo marker.
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

    /// Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.
    rule "abstractor connection" zantufa_abstractor_connection -> struct {
        /// The `joik_connective` connective joining the adjacent constituents of the `zantufa_abstractor_connection` production.
        field connective <- joik_connective;
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
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

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone)]
    pub(crate) struct GeneratedRecoveryBranch {
        pub span_start: usize,
        pub active_rule_contexts: Vec<SyntaxRuleFrame>,
    }

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone)]
    pub(crate) struct GeneratedParseFailure {
        pub public_error: crate::SyntaxError,
        pub branches: Vec<GeneratedRecoveryBranch>,
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
                parser: recovered_generated_text_parser_with_eof(),
                continuation_sentinel_index: None,
                continuation_time_limit: None,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.continuation_time_limit == continuation_time_limit)]
        pub(in crate::grammar) fn new_with_continuation_time_limit(
            continuation_time_limit: Option<ContinuationTimeLimit>,
        ) -> Self {
            Self {
                memo_session: SyntaxRecoveryMemoSession::new(),
                parser: recovered_generated_text_parser_with_eof(),
                continuation_sentinel_index: None,
                continuation_time_limit,
            }
        }

        #[bityzba::requires(true)]
        #[bityzba::ensures(ret.continuation_sentinel_index == Some(sentinel_index))]
        #[bityzba::ensures(ret.continuation_time_limit == continuation_time_limit)]
        pub(in crate::grammar) fn new_for_expected_continuations(
            sentinel_index: usize,
            continuation_time_limit: Option<ContinuationTimeLimit>,
        ) -> Self {
            Self {
                memo_session: SyntaxRecoveryMemoSession::new(),
                parser: recovered_generated_text_parser_with_eof(),
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
        let tokens = spanned_tokens(words);
        let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
        let mut state = ParserState::new(words, options);
        let result = strict_generated_text_parser_with_eof()
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
        parse_text_detailed_tracked_attempt_inner(words, options, None, None)
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
            words,
            options,
            Some(sentinel_index),
            continuation_time_limit,
        )
    }

    #[bityzba::requires(continuation_sentinel_index.is_none_or(|index| index < words.len()))]
    #[bityzba::ensures(continuation_sentinel_index.is_some() -> ret.result.is_err())]
    fn parse_text_detailed_tracked_attempt_inner(
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
        let result = recovery_checkpoint_strict_generated_text_parser_with_eof()
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
                let (diagnostic_candidate, diagnostic_candidates) =
                    failure_context.expect("failure context captured for syntax errors");
                let branches = generated_recovery_branches(&diagnostic_candidates, &errors);
                let public_error = syntax_error_with_diagnostic_candidate(
                    errors.clone(),
                    diagnostic_candidate,
                    options.error_context_depth,
                );
                Err(GeneratedParseFailure {
                    public_error,
                    branches,
                    checkpoints: RecoveryCheckpointIndex::from_checkpoints(
                        finish.recovery_checkpoints,
                    ),
                })
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
                let (diagnostic_candidate, diagnostic_candidates) =
                    failure_context.expect("failure context captured for syntax errors");
                let branches = generated_recovery_branches(&diagnostic_candidates, &errors);
                let public_error = syntax_error_with_diagnostic_candidate(
                    errors.clone(),
                    diagnostic_candidate,
                    options.error_context_depth,
                );
                Err(GeneratedParseFailure {
                    public_error,
                    branches,
                    checkpoints: RecoveryCheckpointIndex::from_checkpoints(
                        finish.recovery_checkpoints,
                    ),
                })
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

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn generated_recovery_branches(
        diagnostic_candidates: &[SyntaxParseError<'_>],
        errors: &[SyntaxParseError<'_>],
    ) -> Vec<GeneratedRecoveryBranch> {
        let source = if diagnostic_candidates.is_empty() {
            errors
        } else {
            diagnostic_candidates
        };
        let Some(deepest_start) = source.iter().map(|error| error.span().start).max() else {
            return Vec::new();
        };
        source
            .iter()
            .filter(|error| error.span().start == deepest_start)
            .map(|error| GeneratedRecoveryBranch {
                span_start: error.span().start,
                active_rule_contexts: error.active_rule_contexts(),
            })
            .collect()
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn strict_generated_text_parser_with_eof<'tokens>()
    -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<TextSyntax>> {
        custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
            let text = input.parse(&strict_generated_text_shared_parser())?;
            input.parse(end()).map(|()| text)
        })
        .boxed()
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn recovery_checkpoint_strict_generated_text_parser_with_eof<'tokens>()
    -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<TextSyntax>> {
        custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
            let text = input.parse(&recovery_checkpoint_strict_generated_text_shared_parser())?;
            input.parse(end()).map(|()| text)
        })
        .boxed()
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn recovered_generated_text_parser_with_eof<'tokens>()
    -> BoxedParser<'tokens, generated_runtime::SharedSyntaxOutput<recovered::TextSyntax>> {
        let parser = recovered_generated_text_shared_parser();
        custom::<_, _>(move |input: &mut InputRef<'tokens, '_>| {
            let text = input.parse(&parser)?;
            input.parse(end()).map(|()| text)
        })
        .boxed()
    }
}
