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
    BoxedParser, ContinuationTimeLimit, ParserState, RecoveryDirective, SpannedToken,
    SyntaxParseError, SyntaxRecoveryMemoSession, SyntaxRuleFrame,
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
        bridi: BridiSyntax;
        bridi_tail: BridiTailSyntax;
        bo_grouped_bridi_tail: BoGroupedBridiTailSyntax;
        bo_grouped_bridi_tail_without_tail_terms: BoGroupedBridiTailWithoutTailTermsSyntax;
        forethought_bridi_connection: ForethoughtBridiConnectionSyntax;
        forethought_bridi_connection_without_tail_terms: ForethoughtBridiConnectionWithoutTailTermsSyntax;
        subbridi: SubbridiSyntax;
        term: TermSyntax;
        sumti: SumtiSyntax;
        sumti_grouped: SumtiGroupedSyntax;
        sumti_afterthought: SumtiAfterthoughtSyntax;
        sumti_bound: SumtiBoundSyntax;
        sumti_forethought: SumtiForethoughtSyntax;
        sumti_base: SumtiBaseSyntax;
        selbri: SelbriSyntax;
        co_selbri: CoSelbriSyntax;
        tanru_unit: TanruUnitSyntax;
        bo_or_linked_tanru_unit: BoOrLinkedTanruUnitSyntax;
        tanru_unit_atom: TanruUnitAtomSyntax;
        jai_inner_tanru_unit: JaiInnerTanruUnitSyntax;
        tense_modal: TenseModalSyntax;
        mekso: MeksoSyntax;
        mekso_base: MeksoBaseSyntax;
        mekso_precedence: MeksoPrecedenceSyntax;
        mekso_operand: MeksoOperandSyntax;
        mekso_operator: MeksoOperatorSyntax;
        reverse_polish_parts: ReversePolishPartsSyntax;
        letter_string: LetterStringSyntax;
        letter_tokens: LetterTokensSyntax;
        free_modifier: FreeModifierSyntax;
    }

    /// Syntax model for leading indicator parsed by the `leading_indicator` grammar rule.
    rule "leading indicator" leading_indicator -> struct {
        /// A word from selmaho `Ui`.
        field indicator <- choice((selmaho(Ui), selmaho(Cai)));
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Syntax model for text parsed by the `text` grammar rule.
    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> enum {
        /// The `explicit_xauha_lohoi_text` alternative of text.
        explicit_xauha_lohoi_text,
        /// The `regular_text` alternative of text.
        regular_text,
    }

    alias "word" word_before_kuhau = word_not_cmavo(Kuhau);

    /// Syntax model for text parsed by the `explicit_xauha_lohoi_text` grammar rule.
    rule "text" explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier) -> struct {
        assert [
            cmavo(Xauha);
            zero_or_more word_before_kuhau();
            cmavo(Kuhau);
        ].ignored();
        /// The paragraphs component of this syntax node.
        field paragraphs <- text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
    }

    /// Syntax model for text parsed by the `regular_text` grammar rule.
    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> struct {
        /// Ordered sequence of zero or more leading nai components.
        field leading_nai <- [zero_or_more cmavo(Nai)];
        /// Ordered sequence of zero or more leading cmevla components.
        field leading_cmevla <- [zero_or_more text_leading_cmevla_word()];
        /// Ordered sequence of zero or more leading indicators components.
        field leading_indicators <- [zero_or_more leading_indicator()];
        /// Ordered sequence of zero or more leading free modifiers components.
        field leading_free_modifiers <- [zero_or_more free_modifier];
        /// The optional leading connective component.
        field leading_connective <- opt(
            modal_forethought_connective(tense_modal)
                .not()
                .ignore_then(text_leading_connective),
        );
        /// Ordered sequence of zero or more leading i statements components.
        field leading_i_statements <- [zero_or_more leading_i_statement(free_modifier, tense_modal)];
        #[tree_child(primary)]
        /// The optional paragraphs component.
        field paragraphs <- opt(arc(text_paragraphs(
            paragraph,
            statement_or_fragment,
            free_modifier,
        )));
    }

    /// Syntax model for paragraphs parsed by the `text_paragraphs` grammar rule.
    rule "paragraphs" text_paragraphs(paragraph, statement_or_fragment, free_modifier) -> enum {
        /// The `text_paragraph_with_additional_niho` alternative of paragraphs.
        text_paragraph_with_additional_niho,
        /// The `text_niho_paragraphs` alternative of paragraphs.
        text_niho_paragraphs,
    }

    /// Syntax model for paragraphs parsed by the `text_paragraph_with_additional_niho` grammar rule.
    rule "paragraphs" text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The first component of this syntax node.
        field first <- paragraph;
        /// Ordered sequence of zero or more additional niho components.
        field additional_niho <- [zero_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Syntax model for paragraphs parsed by the `text_niho_paragraphs` grammar rule.
    rule "paragraphs" text_niho_paragraphs(statement_or_fragment, free_modifier) -> struct {
        /// Non-empty ordered sequence of paragraphs components.
        field paragraphs <- [one_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Syntax model for paragraph statement parsed by the `leading_i_statement` grammar rule.
    rule "paragraph statement" leading_i_statement(free_modifier, tense_modal) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The optional connective component.
        field connective <- opt(arc(i_paragraph_statement_connective(tense_modal)));
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Syntax model for paragraph parsed by the `paragraph` grammar rule.
    rule "paragraph" paragraph(statement_or_fragment, free_modifier) -> enum {
        /// The `i_niho_paragraph` alternative of paragraph.
        i_niho_paragraph,
        /// The `simple_paragraph` alternative of paragraph.
        simple_paragraph,
    }

    /// Syntax model for paragraph parsed by the `simple_paragraph` grammar rule.
    rule "paragraph" simple_paragraph(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The statements component of this syntax node.
        field statements <- paragraph_statement_sequence(statement_or_fragment, free_modifier);
    }

    /// Syntax model for paragraph statement sequence parsed by the `paragraph_statement_sequence` grammar rule.
    rule "paragraph statement sequence" paragraph_statement_sequence(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        /// The initial component of this syntax node.
        field initial <- initial_paragraph_statement(statement_or_fragment);
        /// Ordered sequence of zero or more following components.
        field following <- [zero_or_more following_paragraph_statement(statement_or_fragment, free_modifier)];
        /// Ordered sequence of zero or more trailing components.
        field trailing <- [zero_or_more trailing_ijek_paragraph_statement()];
    }

    /// Syntax model for paragraph parsed by the `i_niho_paragraph` grammar rule.
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

    /// Syntax model for paragraph parsed by the `niho_paragraph` grammar rule.
    rule "paragraph" niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        /// Non-empty ordered sequence of niho components.
        field niho <- [one_or_more selmaho(Niho)];
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        /// The optional statements component.
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    /// Syntax model for paragraph statement parsed by the `initial_paragraph_statement` grammar rule.
    rule "paragraph statement" initial_paragraph_statement(statement_or_fragment) -> struct {
        #[tree_child(primary)]
        /// The shared statement child syntax node.
        field statement <- arc(statement_or_fragment);
    }

    /// Syntax model for paragraph statement parsed by the `following_paragraph_statement` grammar rule.
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

    /// Syntax model for paragraph statement parsed by the `trailing_ijek_paragraph_statement` grammar rule.
    rule "paragraph statement" trailing_ijek_paragraph_statement -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The connective component of this syntax node.
        field connective <- statement_connective;
    }

    /// Syntax model for statement parsed by the `statement` grammar rule.
    rule "statement" statement(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        /// The `i_statement_connection` alternative of statement.
        i_statement_connection,
        /// The `preposed_i_statement_connection` alternative of statement.
        preposed_i_statement_connection,
        /// The `statement_base` alternative of statement.
        statement_base,
    }

    /// Syntax model for statement parsed by the `statement_base` grammar rule.
    rule "statement" statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> enum {
        /// The `prenex_statement` alternative of statement.
        prenex_statement,
        /// The `forethought_statement` alternative of statement.
        when feature(ZantufaConnectives) forethought_statement,
        /// The `bridi_statement` alternative of statement.
        bridi_statement,
        /// The `text_group_statement` alternative of statement.
        text_group_statement,
    }

    /// Syntax model for paragraph statement parsed by the `statement_or_fragment` grammar rule.
    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        /// The `zantufa_statement_terms_statement` alternative of paragraph statement.
        when feature(ZantufaTerms) zantufa_statement_terms_statement,
        /// The `statement_or_fragment_statement` alternative of paragraph statement.
        statement_or_fragment_statement,
        /// The `fragment_statement` alternative of paragraph statement.
        fragment_statement,
    }

    /// Syntax model for paragraph statement parsed by the `zantufa_statement_terms_statement` grammar rule.
    rule "paragraph statement" zantufa_statement_terms_statement(statement, term) -> struct {
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The tail component of this syntax node.
        field tail <- zantufa_statement_terms_tail(term);
    }

    /// Syntax model for paragraph statement parsed by the `zantufa_statement_terms_tail` grammar rule.
    rule "paragraph statement" zantufa_statement_terms_tail(term) -> enum {
        /// The `zantufa_iau_statement_terms_tail` alternative of paragraph statement.
        zantufa_iau_statement_terms_tail,
        /// The `zantufa_bare_statement_terms_tail` alternative of paragraph statement.
        zantufa_bare_statement_terms_tail,
    }

    /// Syntax model for paragraph statement parsed by the `zantufa_iau_statement_terms_tail` grammar rule.
    rule "paragraph statement" zantufa_iau_statement_terms_tail(term) -> struct {
        /// The `Ihau` cmavo marker.
        field iau <- cmavo(Ihau).warn(ExperimentalIauReset).wf();
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
    }

    /// Syntax model for paragraph statement parsed by the `zantufa_bare_statement_terms_tail` grammar rule.
    rule "paragraph statement" zantufa_bare_statement_terms_tail(term) -> struct {
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more arc(term)];
    }

    /// Syntax model for paragraph statement parsed by the `statement_or_fragment_statement` grammar rule.
    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        /// The statement component of this syntax node.
        field statement <- statement;
    }

    /// Syntax model for fragment parsed by the `fragment_statement` grammar rule.
    rule "fragment" fragment_statement(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        /// The `prenex_fragment` alternative of fragment.
        prenex_fragment,
        /// The `selbri_fragment` alternative of fragment.
        selbri_fragment,
        /// The `ek_fragment` alternative of fragment.
        ek_fragment,
        /// The `gihek_fragment` alternative of fragment.
        gihek_fragment,
        /// The `multiple_na_fragment` alternative of fragment.
        multiple_na_fragment,
        /// The `single_na_fragment` alternative of fragment.
        single_na_fragment,
        /// The `terms_fragment` alternative of fragment.
        terms_fragment,
        /// The `mekso_fragment` alternative of fragment.
        mekso_fragment,
        /// The `relative_clause_fragment` alternative of fragment.
        relative_clause_fragment,
        /// The `linked_sumti_continuation_fragment` alternative of fragment.
        linked_sumti_continuation_fragment,
        /// The `linked_sumti_fragment` alternative of fragment.
        linked_sumti_fragment,
        /// The `zantufa_mekso_fragment` alternative of fragment.
        zantufa_mekso_fragment,
    }

    /// Syntax model for statement parsed by the `statement_after_i_connective` grammar rule.
    rule "statement" statement_after_i_connective(statement, bridi, subbridi, tense_modal, text) -> enum {
        /// The `forethought_statement` alternative of statement.
        when feature(ZantufaConnectives) forethought_statement,
        /// The `bridi_statement` alternative of statement.
        bridi_statement,
        /// The `text_group_statement` alternative of statement.
        text_group_statement,
    }

    /// Syntax model for fragment parsed by the `multiple_na_fragment` grammar rule.
    rule "fragment" multiple_na_fragment -> struct {
        /// A word from selmaho `Na`.
        field first_na <- selmaho(Na);
        /// A word from selmaho `Na`.
        field second_na <- selmaho(Na);
        /// Ordered sequence of zero or more additional na components.
        field additional_na <- [zero_or_more selmaho(Na)];
    }

    /// Syntax model for fragment parsed by the `single_na_fragment` grammar rule.
    rule "fragment" single_na_fragment -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
    }

    /// Syntax model for fragment parsed by the `ek_fragment` grammar rule.
    rule "fragment" ek_fragment -> struct {
        #[tree_child(primary)]
        /// The connective component of this syntax node.
        field connective <- ek_connective();
    }

    /// Syntax model for fragment parsed by the `gihek_fragment` grammar rule.
    rule "fragment" gihek_fragment -> struct {
        #[tree_child(primary)]
        /// The connective component of this syntax node.
        field connective <- gihek_connective();
    }

    /// Syntax model for statement connection parsed by the `i_statement_connection` grammar rule.
    rule "statement connection" i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens)];
    }

    /// Syntax model for statement connective parsed by the `pending_i_connective` grammar rule.
    rule "statement connective" pending_i_connective -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The connective component of this syntax node.
        field connective <- statement_connective;
        assert cmavo(I);
    }

    /// Syntax model for statement connection parsed by the `i_statement_connection_tail` grammar rule.
    rule "statement connection" i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        /// The `chained_i_connective_statement_tail` alternative of statement connection.
        chained_i_connective_statement_tail,
        /// The `simple_i_connective_statement_tail` alternative of statement connection.
        simple_i_connective_statement_tail,
    }

    /// Syntax model for statement connection parsed by the `chained_i_connective_statement_tail` grammar rule.
    rule "statement connection" chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// Non-empty ordered sequence of pending components.
        field pending <- [one_or_more pending_i_connective];
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The connective component of this syntax node.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Syntax model for statement connection parsed by the `simple_i_connective_statement_tail` grammar rule.
    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The connective component of this syntax node.
        field connective <- i_statement_connective(tense_modal);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Syntax model for statement connection parsed by the `preposed_i_statement_connection` grammar rule.
    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> struct {
        /// The shared leading statement child syntax node.
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        /// The connective component of this syntax node.
        field connective <- statement_connective;
        /// The `I` cmavo marker.
        field i <- cmavo(I);
        /// The shared trailing statement child syntax node.
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Syntax model for text group parsed by the `text_group_statement` grammar rule.
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

    /// Syntax model for prenex parsed by the `prenex_fragment` grammar rule.
    rule "prenex" prenex_fragment(term) -> struct {
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
        /// The `Zohu` cmavo marker.
        field zohu <- cmavo(Zohu).wf();
    }

    /// Syntax model for prenex parsed by the `prenex_statement` grammar rule.
    rule "prenex" prenex_statement(statement, term) -> struct {
        /// Ordered sequence of zero or more prenex terms components.
        field prenex_terms <- [zero_or_more term];
        /// The `Zohu` cmavo marker.
        field zohu <- cmavo(Zohu).wf();
        #[tree_child(primary)]
        /// The shared inner statement child syntax node.
        field inner_statement <- arc(statement);
    }

    /// Syntax model for statement parsed by the `forethought_statement` grammar rule.
    rule "statement" forethought_statement(statement, tense_modal) -> struct {
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// The shared first child syntax node.
        field first <- arc(statement);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_statement_branch(statement);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_statement_branch(statement)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Syntax model for statement branch parsed by the `forethought_statement_branch` grammar rule.
    rule "statement branch" forethought_statement_branch(statement) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared statement child syntax node.
        field statement <- arc(statement);
    }

    /// Syntax model for statement branch parsed by the `zantufa_forethought_statement_branch` grammar rule.
    rule "statement branch" zantufa_forethought_statement_branch(statement) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// The shared statement child syntax node.
        field statement <- arc(statement);
    }

    /// Syntax model for statement parsed by the `bridi_statement` grammar rule.
    rule "statement" bridi_statement(bridi, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared bridi child syntax node.
        field bridi <- arc(bridi);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more bridi_statement_continuation(subbridi, tense_modal)];
    }

    /// Syntax model for bridi continuation parsed by the `bridi_statement_continuation` grammar rule.
    rule "bridi continuation" bridi_statement_continuation(subbridi, tense_modal) -> enum {
        /// The `bo_bridi_statement_continuation` alternative of bridi continuation.
        bo_bridi_statement_continuation,
        /// The `ke_bridi_statement_continuation` alternative of bridi continuation.
        ke_bridi_statement_continuation,
    }

    /// Syntax model for bridi continuation parsed by the `bo_bridi_statement_continuation` grammar rule.
    rule "bridi continuation" bo_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        /// The connective component of this syntax node.
        field connective <- bridi_tail_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared trailing subbridi child syntax node.
        field trailing_subbridi <- arc(subbridi);
    }

    /// Syntax model for bridi continuation parsed by the `ke_bridi_statement_continuation` grammar rule.
    rule "bridi continuation" ke_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        /// The connective component of this syntax node.
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

    /// Syntax model for selbri parsed by the `selbri_fragment` grammar rule.
    rule "selbri" selbri_fragment(selbri) -> struct {
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
    }

    /// Syntax model for terms parsed by the `terms_fragment` grammar rule.
    rule "terms" terms_fragment(term) -> struct {
        #[tree_child(primary)]
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(cmavo(Vau).wf()).elidable_terminator(Vau);
    }

    /// Syntax model for mex parsed by the `mekso_fragment` grammar rule.
    rule "mex" mekso_fragment(mekso, letter_tokens) -> struct {
        #[tree_child(primary)]
        /// The shared quantifier child syntax node.
        field quantifier <- arc(quantifier(mekso, letter_tokens));
    }

    /// Syntax model for mex parsed by the `zantufa_mekso_fragment` grammar rule.
    rule "mex" zantufa_mekso_fragment(mekso) -> struct {
        #[tree_child(primary)]
        /// The shared expression child syntax node.
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_statement_item());
    }

    /// Syntax model for relative clauses parsed by the `relative_clause_list` grammar rule.
    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal, statement) -> struct {
        /// The first component of this syntax node.
        field first <- relative_clause_atom(sumti, subbridi, tense_modal, statement);
        /// Ordered sequence of zero or more additional components.
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal, statement)];
    }

    /// Syntax model for relative clauses parsed by the `relative_clause_fragment` grammar rule.
    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal, statement) -> struct {
        #[tree_child(primary)]
        /// The relative clauses component of this syntax node.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
    }

    /// Syntax model for linked arguments parsed by the `linked_sumti_continuation_fragment` grammar rule.
    rule "linked arguments" linked_sumti_continuation_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        /// Non-empty ordered sequence of bei links components.
        field bei_links <- [one_or_more bei_link(sumti, tense_modal)];
    }

    /// Syntax model for linked arguments parsed by the `linked_sumti_fragment` grammar rule.
    rule "linked arguments" linked_sumti_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        /// The linkargs component of this syntax node.
        field linkargs <- linkargs(sumti, tense_modal);
    }

    /// Syntax model for bridi parsed by the `bridi` grammar rule.
    rule "bridi" bridi(term, selbri, subbridi, tense_modal, bridi_tail) -> enum {
        /// The `bridi_with_leading_terms` alternative of bridi.
        bridi_with_leading_terms,
        /// The `bridi_with_post_cu_terms` alternative of bridi.
        bridi_with_post_cu_terms,
        /// The `bare_cu_bridi` alternative of bridi.
        bare_cu_bridi,
        /// The `bare_cu_terms_bridi` alternative of bridi.
        bare_cu_terms_bridi,
        /// The `relation_only_bridi` alternative of bridi.
        relation_only_bridi,
    }

    /// Syntax model for bridi parsed by the `bridi_with_leading_terms` grammar rule.
    rule "bridi" bridi_with_leading_terms(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of leading terms components.
        field leading_terms <- [one_or_more term];
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Syntax model for bridi parsed by the `bridi_with_post_cu_terms` grammar rule.
    rule "bridi" bridi_with_post_cu_terms(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of leading terms components.
        field leading_terms <- [one_or_more term];
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Syntax model for bridi parsed by the `bare_cu_bridi` grammar rule.
    rule "bridi" bare_cu_bridi(bridi_tail) -> struct {
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Syntax model for bridi parsed by the `bare_cu_terms_bridi` grammar rule.
    rule "bridi" bare_cu_terms_bridi(term, bridi_tail) -> struct {
        /// The `Cu` cmavo marker.
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Syntax model for bridi parsed by the `relation_only_bridi` grammar rule.
    rule "bridi" relation_only_bridi(bridi_tail) -> struct {
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Syntax model for bridi tail parsed by the `cu_terms_bridi_tail` grammar rule.
    rule "bridi tail" cu_terms_bridi_tail(term, bridi_tail) -> struct {
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more term];
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bridi_tail);
    }

    /// Syntax model for bridi tail parsed by the `bridi_tail` grammar rule.
    rule "bridi tail" bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// The `zantufa_grouped_bridi_tail` alternative of bridi tail.
        when feature(ZantufaTerms) zantufa_grouped_bridi_tail,
        /// The `bridi_tail_with_possible_tail_terms` alternative of bridi tail.
        bridi_tail_with_possible_tail_terms,
        /// The `bridi_tail_without_tail_terms` alternative of bridi tail.
        bridi_tail_without_tail_terms,
    }

    /// Syntax model for bridi tail parsed by the `zantufa_grouped_bridi_tail` grammar rule.
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

    /// Syntax model for bridi tail parsed by the `bridi_tail_without_tail_terms` grammar rule.
    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional ke continuation component.
        field ke_continuation <- opt(arc(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Syntax model for bridi tail parsed by the `bridi_tail_with_possible_tail_terms` grammar rule.
    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        assert !(relation_connective_as_bridi_tail, opt(arc(tense_modal)), cmavo(Ke));
        /// The optional ke continuation component.
        field ke_continuation <- opt(arc(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Syntax model for bridi tail parsed by the `afterthought_bridi_tail_without_tail_terms` grammar rule.
    rule "bridi tail" afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The bridi tails component of this syntax node.
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail_without_tail_terms),
            zero_or_more: bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Syntax model for bridi tail parsed by the `afterthought_bridi_tail` grammar rule.
    rule "bridi tail" afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        /// The bridi tails component of this syntax node.
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail),
            zero_or_more: bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Syntax model for bridi tail parsed by the `bo_grouped_bridi_tail_without_tail_terms` grammar rule.
    rule "bridi tail" bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    /// Syntax model for bridi tail parsed by the `bo_grouped_bridi_tail` grammar rule.
    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        /// The shared first child syntax node.
        field first <- arc(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
        /// The optional bo continuation component.
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal)));
    }

    /// Syntax model for bridi tail parsed by the `simple_bridi_tail_without_tail_terms` grammar rule.
    rule "bridi tail" simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        /// The `forethought_simple_bridi_tail_without_tail_terms` alternative of bridi tail.
        forethought_simple_bridi_tail_without_tail_terms,
        /// The `selbri_simple_bridi_tail_without_tail_terms` alternative of bridi tail.
        selbri_simple_bridi_tail_without_tail_terms,
    }

    /// Syntax model for bridi tail parsed by the `simple_bridi_tail` grammar rule.
    rule "bridi tail" simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> enum {
        /// The `forethought_simple_bridi_tail` alternative of bridi tail.
        forethought_simple_bridi_tail,
        /// The `selbri_simple_bridi_tail` alternative of bridi tail.
        selbri_simple_bridi_tail,
    }

    /// Syntax model for forethought bridi connection parsed by the `forethought_simple_bridi_tail_without_tail_terms` grammar rule.
    rule "forethought bridi connection" forethought_simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        /// The shared connection child syntax node.
        field connection <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Syntax model for forethought bridi connection parsed by the `forethought_simple_bridi_tail` grammar rule.
    rule "forethought bridi connection" forethought_simple_bridi_tail(forethought_bridi_connection) -> struct {
        /// The shared connection child syntax node.
        field connection <- arc(forethought_bridi_connection);
    }

    /// Syntax model for bridi tail parsed by the `selbri_simple_bridi_tail_without_tail_terms` grammar rule.
    rule "bridi tail" selbri_simple_bridi_tail_without_tail_terms(selbri) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Syntax model for bridi tail parsed by the `selbri_simple_bridi_tail` grammar rule.
    rule "bridi tail" selbri_simple_bridi_tail(selbri, term) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// Ordered sequence of zero or more terms components.
        field terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Syntax model for forethought bridi connection parsed by the `forethought_bridi_connection` grammar rule.
    rule "forethought bridi connection" forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal) -> enum {
        /// The `direct_forethought_bridi_connection` alternative of forethought bridi connection.
        direct_forethought_bridi_connection,
        /// The `grouped_forethought_bridi_connection` alternative of forethought bridi connection.
        grouped_forethought_bridi_connection,
        /// The `negated_forethought_bridi_connection` alternative of forethought bridi connection.
        negated_forethought_bridi_connection,
    }

    /// Syntax model for forethought bridi connection parsed by the `forethought_bridi_connection_without_tail_terms` grammar rule.
    rule "forethought bridi connection" forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal) -> enum {
        /// The `direct_forethought_bridi_connection_without_tail_terms` alternative of forethought bridi connection.
        direct_forethought_bridi_connection_without_tail_terms,
        /// The `grouped_forethought_bridi_connection_without_tail_terms` alternative of forethought bridi connection.
        grouped_forethought_bridi_connection_without_tail_terms,
        /// The `negated_forethought_bridi_connection_without_tail_terms` alternative of forethought bridi connection.
        negated_forethought_bridi_connection_without_tail_terms,
    }

    /// Syntax model for forethought bridi connection parsed by the `direct_forethought_bridi_connection` grammar rule.
    rule "forethought bridi connection" direct_forethought_bridi_connection(subbridi, term, tense_modal) -> struct {
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// The shared first child syntax node.
        field first <- arc(subbridi);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        /// Ordered sequence of zero or more tail terms components.
        field tail_terms <- [zero_or_more term];
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Syntax model for forethought bridi connection parsed by the `direct_forethought_bridi_connection_without_tail_terms` grammar rule.
    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal) -> struct {
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// The shared first child syntax node.
        field first <- arc(subbridi);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_bridi_branch(subbridi);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        /// The optional `Vau` cmavo marker.
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Syntax model for forethought bridi branch parsed by the `forethought_bridi_branch` grammar rule.
    rule "forethought bridi branch" forethought_bridi_branch(subbridi) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared branch child syntax node.
        field branch <- arc(subbridi);
    }

    /// Syntax model for forethought bridi branch parsed by the `zantufa_forethought_bridi_branch` grammar rule.
    rule "forethought bridi branch" zantufa_forethought_bridi_branch(subbridi) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// The shared branch child syntax node.
        field branch <- arc(subbridi);
    }

    /// Syntax model for forethought bridi connection parsed by the `grouped_forethought_bridi_connection` grammar rule.
    rule "forethought bridi connection" grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Syntax model for forethought bridi connection parsed by the `grouped_forethought_bridi_connection_without_tail_terms` grammar rule.
    rule "forethought bridi connection" grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Syntax model for forethought bridi connection parsed by the `negated_forethought_bridi_connection` grammar rule.
    rule "forethought bridi connection" negated_forethought_bridi_connection(forethought_bridi_connection) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection);
    }

    /// Syntax model for forethought bridi connection parsed by the `negated_forethought_bridi_connection_without_tail_terms` grammar rule.
    rule "forethought bridi connection" negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        /// The shared inner child syntax node.
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_ke_continuation` grammar rule.
    rule "bridi tail connective" bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        /// The connective component of this syntax node.
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

    /// Syntax model for bridi tail connective parsed by the `gihek_bridi_tail_ke_continuation` grammar rule.
    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        /// The connective component of this syntax node.
        field connective <- gihek_connective();
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

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_bo_continuation_without_tail_terms` grammar rule.
    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        /// The connective component of this syntax node.
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

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_bo_continuation` grammar rule.
    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        /// The connective component of this syntax node.
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

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_continuation_without_tail_terms` grammar rule.
    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The connective component of this syntax node.
        field connective <- bridi_tail_connective;
        /// The optional `Cu` cmavo marker.
        field cu <- opt(arc(cmavo(Cu).wf()));
        /// The shared bridi tail child syntax node.
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_continuation` grammar rule.
    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        /// The connective component of this syntax node.
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

    /// Syntax model for subbridi parsed by the `subbridi` grammar rule.
    rule "subbridi" subbridi(subbridi, bridi, term) -> enum {
        /// The `prenex_subbridi` alternative of subbridi.
        prenex_subbridi,
        /// The `bridi_subbridi` alternative of subbridi.
        bridi_subbridi,
    }

    /// Syntax model for subbridi parsed by the `bridi_subbridi` grammar rule.
    rule "subbridi" bridi_subbridi(bridi) -> struct {
        /// The shared bridi child syntax node.
        field bridi <- arc(bridi);
    }

    /// Syntax model for prenex parsed by the `prenex_subbridi` grammar rule.
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

    /// Syntax model for term parsed by the `term` grammar rule.
    rule "term" term(statement, term, sumti, tense_modal, subbridi, selbri, free_modifier) -> enum {
        /// The `pehe_termset_connection` alternative of term.
        pehe_termset_connection,
        /// The `bound_term_connection` alternative of term.
        bound_term_connection,
        /// The `termset_group` alternative of term.
        termset_group,
        /// The `connected_term` alternative of term.
        connected_term,
        /// The `simple_term` alternative of term.
        simple_term,
    }

    /// Syntax model for termset connection parsed by the `pehe_termset_connection` grammar rule.
    rule "termset connection" pehe_termset_connection(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Syntax model for termset connection continuation parsed by the `pehe_termset_connection_continuation` grammar rule.
    rule "termset connection continuation" pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        /// The `Pehe` cmavo marker.
        field pehe <- cmavo(Pehe).wf();
        /// The connective component of this syntax node.
        field connective <- statement_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Syntax model for term parsed by the `pehe_termset_operand` grammar rule.
    rule "term" pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
        /// The `bound_term_connection` alternative of term.
        bound_term_connection,
        /// The `termset_group` alternative of term.
        termset_group,
        /// The `simple_term` alternative of term.
        simple_term,
    }

    /// Syntax model for term parsed by the `simple_term` grammar rule.
    rule "term" simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
        /// The `place_tagged_sumti_term` alternative of term.
        place_tagged_sumti_term,
        /// The `jai_tagged_sumti_term` alternative of term.
        jai_tagged_sumti_term,
        /// The `tagged_sumti_before_tag_term` alternative of term.
        tagged_sumti_before_tag_term,
        /// The `tagged_sumti_term` alternative of term.
        tagged_sumti_term,
        /// The `noiha_adverbial_term` alternative of term.
        noiha_adverbial_term,
        /// The `fihoi_adverbial_term` alternative of term.
        fihoi_adverbial_term,
        /// The `soi_adverbial_term` alternative of term.
        soi_adverbial_term,
        /// The `na_ku_term` alternative of term.
        na_ku_term,
        /// The `sumti_term` alternative of term.
        sumti_term,
        /// The `bare_na_term` alternative of term.
        bare_na_term,
        /// The `forethought_termset` alternative of term.
        forethought_termset,
        /// The `nuhi_termset` alternative of term.
        nuhi_termset,
        /// The `ke_termset` alternative of term.
        ke_termset,
    }

    /// Syntax model for term connection parsed by the `bound_term_connection` grammar rule.
    rule "term connection" bound_term_connection(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        /// The shared connective child syntax node.
        field connective <- arc(bound_term_connective);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        assert choice((
            feature(TermHierarchy),
            (
                feature(TermHierarchy).not(),
                sumti.not(),
            ).ignored(),
        ));
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        assert choice((
            feature(TermHierarchy),
            (
                feature(TermHierarchy).not(),
                sumti.not(),
            ).ignored(),
        ));
    }

    /// Syntax model for term connective parsed by the `bound_term_connective` grammar rule.
    rule "term connective" bound_term_connective -> enum {
        /// The `joik_connective` alternative of term connective.
        joik_connective,
        /// The `ek_connective` alternative of term connective.
        ek_connective,
    }

    /// Syntax model for term connection parsed by the `connected_term` grammar rule.
    rule "term connection" connected_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Syntax model for term connection continuation parsed by the `connected_term_continuation` grammar rule.
    rule "term connection continuation" connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        /// The connective component of this syntax node.
        field connective <- connected_term_connective;
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Syntax model for term connective parsed by the `connected_term_connective` grammar rule.
    rule "term connective" connected_term_connective -> enum {
        /// The `joik_connective` alternative of term connective.
        joik_connective,
        /// The `jek_connective` alternative of term connective.
        jek_connective,
        /// The `ek_connective` alternative of term connective.
        ek_connective,
        /// The `vuhu_nonlogical_connective` alternative of term connective.
        vuhu_nonlogical_connective,
    }

    /// Syntax model for termset parsed by the `termset_group` grammar rule.
    rule "termset" termset_group(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        /// The shared leading term child syntax node.
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Syntax model for termset continuation parsed by the `termset_group_continuation` grammar rule.
    rule "termset continuation" termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        /// The `Cehe` cmavo marker.
        field cehe <- cmavo(Cehe).wf();
        /// The shared trailing term child syntax node.
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Syntax model for termset parsed by the `forethought_termset` grammar rule.
    rule "termset" forethought_termset(term, tense_modal) -> struct {
        /// The optional `Nuhi` cmavo marker.
        field m_nuhi <- opt(cmavo(Nuhi).wf());
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more arc(term)];
        /// The optional `Nuhu` cmavo marker.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_termset_branch(term);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_termset_branch(term)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Syntax model for termset parsed by the `forethought_termset_branch` grammar rule.
    rule "termset" forethought_termset_branch(term) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more arc(term)];
        /// The optional `Nuhu` cmavo marker.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Syntax model for termset parsed by the `zantufa_forethought_termset_branch` grammar rule.
    rule "termset" zantufa_forethought_termset_branch(term) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// Non-empty ordered sequence of terms components.
        field terms <- [one_or_more arc(term)];
        /// The optional `Nuhu` cmavo marker.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Syntax model for termset parsed by the `nuhi_termset` grammar rule.
    rule "termset" nuhi_termset(term) -> struct {
        /// The `Nuhi` cmavo marker.
        field nuhi <- cmavo(Nuhi).wf();
        /// Non-empty ordered sequence of termset components.
        field termset <- [one_or_more arc(term)];
        /// The optional `Nuhu` cmavo marker.
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Syntax model for termset parsed by the `ke_termset` grammar rule.
    rule "termset" ke_termset(term) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).warn(ExperimentalKeTermset).wf();
        /// Non-empty ordered sequence of termset components.
        field termset <- [one_or_more arc(term)];
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for NOIhA adverbial parsed by the `noiha_adverbial_term` grammar rule.
    rule "NOIhA adverbial" noiha_adverbial_term(free_modifier, selbri) -> enum {
        /// The `noiha_variable_adverbial_term` alternative of NOIhA adverbial.
        noiha_variable_adverbial_term,
        /// The `noiha_relative_adverbial_term` alternative of NOIhA adverbial.
        noiha_relative_adverbial_term,
    }

    /// Syntax model for NOIhA adverbial parsed by the `noiha_variable_adverbial_term` grammar rule.
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

    /// Syntax model for NOIhA adverbial parsed by the `noiha_relative_adverbial_term` grammar rule.
    rule "NOIhA adverbial" noiha_relative_adverbial_term(selbri) -> struct {
        /// A word from selmaho `Noiha`.
        field noiha <- selmaho(Noiha).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Fehu` cmavo marker.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Syntax model for FIhOI adverbial parsed by the `fihoi_adverbial_term` grammar rule.
    rule "FIhOI adverbial" fihoi_adverbial_term(statement) -> struct {
        /// The `Fihoi` cmavo marker.
        field fihoi <- cmavo(Fihoi).warn(ExperimentalFihoiAdverbial).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Fihau` cmavo marker.
        field fihau <- opt(cmavo(Fihau).wf()).elidable_terminator(Fihau);
    }

    /// Syntax model for SOI adverbial parsed by the `soi_adverbial_term` grammar rule.
    rule "SOI adverbial" soi_adverbial_term(statement) -> struct {
        /// A word from selmaho `Soi`.
        field soi <- selmaho(Soi).warn(ExperimentalSoiAdverbial).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    /// Syntax model for term parsed by the `sumti_term` grammar rule.
    rule "term" sumti_term(sumti) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for place tag parsed by the `place_tagged_sumti_term` grammar rule.
    rule "place tag" place_tagged_sumti_term(sumti) -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Syntax model for NA KU term parsed by the `na_ku_term` grammar rule.
    rule "NA KU term" na_ku_term -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na);
        /// The `Ku` cmavo marker.
        field na_ku <- cmavo(Ku).wf();
    }

    /// Syntax model for NA term parsed by the `bare_na_term` grammar rule.
    rule "NA term" bare_na_term(selbri, tense_modal) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).wf();
        assert !choice((
            selbri.ignored(),
            modal_forethought_connective(tense_modal).ignored(),
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

    /// Syntax model for tag parsed by the `tagged_sumti_before_tag_term` grammar rule.
    rule "tag" tagged_sumti_before_tag_term(tense_modal, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert tense_modal.lookahead();
    }

    /// Syntax model for tag parsed by the `tagged_sumti_term` grammar rule.
    rule "tag" tagged_sumti_term(tense_modal, sumti, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert !selbri;
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Syntax model for tag parsed by the `jai_tagged_sumti_term` grammar rule.
    rule "tag" jai_tagged_sumti_term(tense_modal, sumti) -> struct {
        assert feature(ZantufaTags);
        /// The `Jai` cmavo marker.
        field jai <- cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
        /// The optional tag component.
        field tag <- opt(arc(tense_modal));
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for tag parsed by the `leading_term_tag_tense_modal` grammar rule.
    rule "tag" leading_term_tag_tense_modal(tense_modal, selbri) -> enum {
        /// The `pu_before_nahe_leading_term_tag_tense` alternative of tag.
        pu_before_nahe_leading_term_tag_tense,
        /// The `pu_distance_before_tag_leading_term_tag_tense` alternative of tag.
        pu_distance_before_tag_leading_term_tag_tense,
        /// The `zi_before_zi_leading_term_tag_tense` alternative of tag.
        zi_before_zi_leading_term_tag_tense,
        /// The `va_before_va_leading_term_tag_tense` alternative of tag.
        va_before_va_leading_term_tag_tense,
        /// The `mohi_before_mohi_leading_term_tag_tense` alternative of tag.
        mohi_before_mohi_leading_term_tag_tense,
        /// The `caha_before_tag_leading_term_tag_tense` alternative of tag.
        caha_before_tag_leading_term_tag_tense,
        /// The `interval_property_leading_term_tag_tense` alternative of tag.
        interval_property_leading_term_tag_tense,
        /// The `tense_modal` alternative of tag.
        tense_modal,
    }

    /// Syntax model for tag parsed by the `pu_before_nahe_leading_term_tag_tense` grammar rule.
    rule "tag" pu_before_nahe_leading_term_tag_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        assert selmaho(Nahe);
    }

    /// Syntax model for tag parsed by the `pu_distance_before_tag_leading_term_tag_tense` grammar rule.
    rule "tag" pu_distance_before_tag_leading_term_tag_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// A word from selmaho `Zi`.
        field distance <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Syntax model for tag parsed by the `zi_before_zi_leading_term_tag_tense` grammar rule.
    rule "tag" zi_before_zi_leading_term_tag_tense -> struct {
        /// A word from selmaho `Zi`.
        field zi <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Syntax model for tag parsed by the `va_before_va_leading_term_tag_tense` grammar rule.
    rule "tag" va_before_va_leading_term_tag_tense -> struct {
        /// A word from selmaho `Va`.
        field va <- selmaho(Va).wf();
        assert selmaho(Va);
    }

    /// Syntax model for tag parsed by the `mohi_before_mohi_leading_term_tag_tense` grammar rule.
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

    /// Syntax model for tag parsed by the `caha_before_tag_leading_term_tag_tense` grammar rule.
    rule "tag" caha_before_tag_leading_term_tag_tense(tense_modal) -> struct {
        /// A word from selmaho `Caha`.
        field caha <- selmaho(Caha).wf().followed_by(tense_modal.lookahead());
    }

    /// Syntax model for interval property parsed by the `interval_property_leading_term_tag_tense` grammar rule.
    rule "interval property" interval_property_leading_term_tag_tense(selbri) -> struct {
        /// The shared property child syntax node.
        field property <- arc(interval_property_tense().followed_by(choice((
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

    /// Syntax model for sumti parsed by the `tagged_or_elided_sumti` grammar rule.
    rule "sumti" tagged_or_elided_sumti(sumti) -> enum {
        /// The `sumti` alternative of sumti.
        sumti,
        /// The `tagged_elided_sumti` alternative of sumti.
        tagged_elided_sumti,
    }

    /// Syntax model for elided sumti parsed by the `tagged_elided_sumti` grammar rule.
    rule "elided sumti" tagged_elided_sumti -> struct {
        /// The optional `Ku` cmavo marker.
        field maybe_ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Syntax model for sumti parsed by the `sumti` grammar rule.
    rule "sumti" sumti(sumti, sumti_grouped, subbridi, tense_modal, statement) -> struct {
        /// The shared base sumti child syntax node.
        field base_sumti <- arc(sumti_grouped);
        /// The optional vuho attachment component.
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for sumti connection parsed by the `sumti_grouped` grammar rule.
    rule "sumti connection" sumti_grouped(sumti, sumti_afterthought, tense_modal, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_afterthought);
        /// The optional grouped tail component.
        field grouped_tail <- opt(grouped_sumti_tail(sumti, tense_modal));
    }

    /// Syntax model for sumti connection parsed by the `sumti_afterthought` grammar rule.
    rule "sumti connection" sumti_afterthought(sumti_bound, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_bound);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more sumti_afterthought_tail(sumti_bound)];
    }

    /// Syntax model for sumti connection parsed by the `sumti_bound` grammar rule.
    rule "sumti connection" sumti_bound(sumti_bound, sumti_forethought, tense_modal, statement) -> struct {
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti_forethought);
        /// The optional bound tail component.
        field bound_tail <- opt(bound_sumti_tail(sumti_bound, tense_modal));
    }

    /// Syntax model for sumti parsed by the `sumti_forethought` grammar rule.
    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// The `forethought_sumti` alternative of sumti.
        forethought_sumti,
        /// The `simple_sumti` alternative of sumti.
        simple_sumti,
    }

    /// Syntax model for forethought sumti connection parsed by the `forethought_sumti` grammar rule.
    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal, statement) -> struct {
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// The shared leading sumti child syntax node.
        field leading_sumti <- arc(sumti);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_sumti_branch(sumti_forethought);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_sumti_branch(sumti_forethought)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Syntax model for forethought sumti connection parsed by the `forethought_sumti_branch` grammar rule.
    rule "forethought sumti connection" forethought_sumti_branch(sumti_forethought) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_forethought);
    }

    /// Syntax model for forethought sumti connection parsed by the `zantufa_forethought_sumti_branch` grammar rule.
    rule "forethought sumti connection" zantufa_forethought_sumti_branch(sumti_forethought) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_forethought);
    }

    /// Syntax model for sumti connection parsed by the `bound_sumti_tail` grammar rule.
    rule "sumti connection" bound_sumti_tail(sumti_bound, tense_modal) -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(argument_connective);
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared trailing sumti child syntax node.
        field trailing_sumti <- arc(sumti_bound);
    }

    /// Syntax model for sumti connective parsed by the `sumti_afterthought_tail` grammar rule.
    rule "sumti connective" sumti_afterthought_tail(sumti_bound) -> struct {
        /// The connective component of this syntax node.
        field connective <- argument_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_bound);
    }

    /// Syntax model for sumti connection parsed by the `grouped_sumti_tail` grammar rule.
    rule "sumti connection" grouped_sumti_tail(sumti, tense_modal) -> struct {
        /// The connective component of this syntax node.
        field connective <- argument_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for sumti relative phrase parsed by the `vuho_sumti_attachment_tail` grammar rule.
    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> enum {
        /// The `vuho_relative_sumti_attachment_tail` alternative of sumti relative phrase.
        vuho_relative_sumti_attachment_tail,
        /// The `vuho_connected_sumti_attachment_tail` alternative of sumti relative phrase.
        vuho_connected_sumti_attachment_tail,
    }

    /// Syntax model for sumti relative phrase parsed by the `vuho_relative_sumti_attachment_tail` grammar rule.
    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> struct {
        /// The `Vuho` cmavo marker.
        field vuho <- cmavo(Vuho).wf();
        /// The relative clauses component of this syntax node.
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
        /// The optional sumti connection component.
        field sumti_connection <- opt(arc(sumti_connection_tail(sumti)));
    }

    /// Syntax model for sumti relative phrase parsed by the `vuho_connected_sumti_attachment_tail` grammar rule.
    rule "sumti relative phrase" vuho_connected_sumti_attachment_tail(sumti) -> struct {
        /// The `Vuho` cmavo marker.
        field vuho <- cmavo(Vuho).wf();
        /// The shared sumti connection child syntax node.
        field sumti_connection <- arc(sumti_connection_tail(sumti));
    }

    /// Syntax model for sumti parsed by the `simple_sumti` grammar rule.
    rule "sumti" simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> struct {
        /// The shared base sumti child syntax node.
        field base_sumti <- arc(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement));
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for sumti parsed by the `sumti_atom` grammar rule.
    rule "sumti" sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// The `sumti_base` alternative of sumti.
        sumti_base,
        /// The `quantified_sumti` alternative of sumti.
        quantified_sumti,
    }

    /// Syntax model for sumti parsed by the `sumti_base` grammar rule.
    rule "sumti" sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement) -> enum {
        /// The `scalar_negated_sumti_with_bo` alternative of sumti.
        scalar_negated_sumti_with_bo,
        /// The `scalar_negated_sumti` alternative of sumti.
        scalar_negated_sumti,
        /// The `lahe_sumti` alternative of sumti.
        lahe_sumti,
        /// The `lahe_term_wrapper` alternative of sumti.
        lahe_term_wrapper,
        /// The `scalar_negated_term_wrapper_with_bo` alternative of sumti.
        scalar_negated_term_wrapper_with_bo,
        /// The `scalar_negated_term_wrapper` alternative of sumti.
        scalar_negated_term_wrapper,
        /// The `bridi_description_sumti` alternative of sumti.
        bridi_description_sumti,
        /// The `name_sumti` alternative of sumti.
        name_sumti,
        /// The `description_connection_sumti` alternative of sumti.
        description_connection_sumti,
        /// The `descriptor_with_outer_quantifier_sumti` alternative of sumti.
        descriptor_with_outer_quantifier_sumti,
        /// The `descriptor_with_gadri_sumti` alternative of sumti.
        descriptor_with_gadri_sumti,
        /// The `descriptor_without_gadri_sumti` alternative of sumti.
        descriptor_without_gadri_sumti,
        /// The `number_sumti` alternative of sumti.
        number_sumti,
        /// The `lerfu_string_sumti` alternative of sumti.
        lerfu_string_sumti,
        /// The `quoted_sumti` alternative of sumti.
        quoted_sumti,
        /// The `pro_sumti` alternative of sumti.
        pro_sumti,
    }

    /// Syntax model for quantified sumti parsed by the `quantified_sumti` grammar rule.
    rule "quantified sumti" quantified_sumti(sumti_base, mekso, letter_tokens) -> struct {
        /// The quantifier component of this syntax node.
        field quantifier <- quantifier(mekso, letter_tokens);
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti_base);
    }

    /// Syntax model for sumti connective parsed by the `sumti_connection_tail` grammar rule.
    rule "sumti connective" sumti_connection_tail(sumti) -> struct {
        /// The connective component of this syntax node.
        field connective <- argument_connective;
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for quantifier parsed by the `pa_run_quantifier` grammar rule.
    rule "quantifier" pa_run_quantifier(letter_tokens) -> struct {
        /// The number component of this syntax node.
        field number <- number_words(letter_tokens).wf();
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi).wf()).elidable_terminator(Boi);
    }

    /// Syntax model for quantifier parsed by the `mekso_quantifier` grammar rule.
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

    /// Syntax model for quantifier parsed by the `zantufa_raw_mekso_quantifier` grammar rule.
    rule "quantifier" zantufa_raw_mekso_quantifier(mekso, letter_tokens) -> struct {
        assert zantufa_raw_mekso_quantifier_guard(letter_tokens);
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
    }

    /// Syntax model for quantifier parsed by the `zantufa_priority_raw_mekso_quantifier` grammar rule.
    rule "quantifier" zantufa_priority_raw_mekso_quantifier(mekso, letter_tokens) -> struct {
        assert zantufa_raw_mekso_quantifier_guard(letter_tokens);
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
    }

    /// Syntax model for quantifier parsed by the `quantifier` grammar rule.
    rule "quantifier" quantifier(mekso, letter_tokens) -> enum {
        /// The `zantufa_priority_raw_mekso_quantifier` alternative of quantifier.
        when feature(ZantufaMex) zantufa_priority_raw_mekso_quantifier,
        /// The `mekso_quantifier` alternative of quantifier.
        mekso_quantifier,
        /// The `pa_run_quantifier` alternative of quantifier.
        pa_run_quantifier,
        /// The `zantufa_raw_mekso_quantifier` alternative of quantifier.
        when feature(ZantufaMex) zantufa_raw_mekso_quantifier,
    }

    /// Syntax model for number mex parsed by the `number_mekso` grammar rule.
    rule "number mex" number_mekso(letter_tokens) -> struct {
        /// The shared quantifier child syntax node.
        field quantifier <- arc(pa_run_quantifier(letter_tokens));
    }

    /// Syntax model for VUhU operator parsed by the `primitive_mekso_operator` grammar rule.
    rule "VUhU operator" primitive_mekso_operator -> struct {
        /// A word from selmaho `Vuhu`.
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Syntax model for operator parsed by the `mekso_operator` grammar rule.
    rule "operator" mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// The `afterthought_mekso_operator` alternative of operator.
        afterthought_mekso_operator,
        /// The `bound_mekso_operator` alternative of operator.
        bound_mekso_operator,
        /// The `simple_mekso_operator` alternative of operator.
        simple_mekso_operator,
    }

    /// Syntax model for operator parsed by the `afterthought_mekso_operator` grammar rule.
    rule "operator" afterthought_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        /// The operators component of this syntax node.
        field operators <- chain(
            first: arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri)),
            zero_or_more: afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri),
            element: trailing_operator,
        );
    }

    /// Syntax model for operator continuation parsed by the `afterthought_mekso_operator_continuation` grammar rule.
    rule "operator continuation" afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri) -> struct {
        /// The connective component of this syntax node.
        field connective <- standard_statement_connective;
        /// The shared trailing operator child syntax node.
        field trailing_operator <- arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri));
    }

    /// Syntax model for operator parsed by the `bound_or_atom_mekso_operator` grammar rule.
    rule "operator" bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// The `bound_mekso_operator` alternative of operator.
        bound_mekso_operator,
        /// The `simple_mekso_operator` alternative of operator.
        simple_mekso_operator,
    }

    /// Syntax model for operator parsed by the `bound_mekso_operator` grammar rule.
    rule "operator" bound_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        /// The shared left operator child syntax node.
        field left_operator <- arc(simple_mekso_operator(mekso, mekso_operator, sumti, selbri));
        /// The connective component of this syntax node.
        field connective <- standard_statement_connective;
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared right operator child syntax node.
        field right_operator <- arc(mekso_operator);
    }

    /// Syntax model for operator parsed by the `simple_mekso_operator` grammar rule.
    rule "operator" simple_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// The `converted_mekso_operator` alternative of operator.
        converted_mekso_operator,
        /// The `scalar_negated_mekso_operator` alternative of operator.
        scalar_negated_mekso_operator,
        /// The `forethought_mekso_operator` alternative of operator.
        forethought_mekso_operator,
        /// The `grouped_mekso_operator` alternative of operator.
        grouped_mekso_operator,
        /// The `selbri_mekso_operator` alternative of operator.
        selbri_mekso_operator,
        /// The `operand_mekso_operator` alternative of operator.
        operand_mekso_operator,
        /// The `zantufa_maho_selbri_mekso_operator` alternative of operator.
        when feature(ZantufaMex) zantufa_maho_selbri_mekso_operator,
        /// The `zantufa_maho_sumti_mekso_operator` alternative of operator.
        when feature(ZantufaMex) zantufa_maho_sumti_mekso_operator,
        /// The `zantufa_connective_mekso_operator` alternative of operator.
        when feature(ZantufaMex) zantufa_connective_mekso_operator,
        /// The `primitive_mekso_operator` alternative of operator.
        primitive_mekso_operator,
    }

    /// Syntax model for converted operator parsed by the `converted_mekso_operator` grammar rule.
    rule "converted operator" converted_mekso_operator(mekso_operator) -> struct {
        /// A word from selmaho `Se`.
        field se <- selmaho(Se).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(mekso_operator);
    }

    /// Syntax model for converted operator parsed by the `scalar_negated_mekso_operator` grammar rule.
    rule "converted operator" scalar_negated_mekso_operator(mekso_operator) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(mekso_operator);
    }

    /// Syntax model for operator parsed by the `forethought_mekso_operator` grammar rule.
    rule "operator" forethought_mekso_operator(mekso_operator) -> struct {
        /// The guhek component of this syntax node.
        field guhek <- guhek_connective;
        /// The shared left operator child syntax node.
        field left_operator <- arc(mekso_operator);
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared right operator child syntax node.
        field right_operator <- arc(mekso_operator);
    }

    /// Syntax model for grouped operator parsed by the `grouped_mekso_operator` grammar rule.
    rule "grouped operator" grouped_mekso_operator(mekso_operator) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared inner operator child syntax node.
        field inner_operator <- arc(mekso_operator);
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for selbri-to-operator parsed by the `selbri_mekso_operator` grammar rule.
    rule "selbri-to-operator" selbri_mekso_operator(selbri) -> struct {
        /// The `Nahu` cmavo marker.
        field nahu <- cmavo(Nahu).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for operand-to-operator parsed by the `operand_mekso_operator` grammar rule.
    rule "operand-to-operator" operand_mekso_operator(mekso) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).wf();
        /// The shared mekso child syntax node.
        field mekso <- arc(mekso);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for selbri-to-operator parsed by the `zantufa_maho_selbri_mekso_operator` grammar rule.
    rule "selbri-to-operator" zantufa_maho_selbri_mekso_operator(selbri) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for sumti-to-operator parsed by the `zantufa_maho_sumti_mekso_operator` grammar rule.
    rule "sumti-to-operator" zantufa_maho_sumti_mekso_operator(sumti) -> struct {
        /// The `Maho` cmavo marker.
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for connective operator parsed by the `zantufa_connective_mekso_operator` grammar rule.
    rule "connective operator" zantufa_connective_mekso_operator -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(operand_connective);
        assert !cmavo(Cu);
    }

    /// Syntax model for operand parsed by the `mekso_operand` grammar rule.
    rule "operand" mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// The `afterthought_mekso_operand` alternative of operand.
        afterthought_mekso_operand,
        /// The `bound_mekso_operand` alternative of operand.
        bound_mekso_operand,
        /// The `simple_mekso_operand` alternative of operand.
        simple_mekso_operand,
    }

    /// Syntax model for operand connective parsed by the `afterthought_mekso_operand` grammar rule.
    rule "operand connective" afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        /// The operands component of this syntax node.
        field operands <- chain(
            first: arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier)),
            zero_or_more: afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            element: trailing_expression,
        );
    }

    /// Syntax model for operand continuation parsed by the `afterthought_mekso_operand_continuation` grammar rule.
    rule "operand continuation" afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        /// The operand connective component of this syntax node.
        field operand_connective <- operand_connective;
        /// The shared trailing expression child syntax node.
        field trailing_expression <- arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
    }

    /// Syntax model for operand parsed by the `bound_or_simple_mekso_operand` grammar rule.
    rule "operand" bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// The `bound_mekso_operand` alternative of operand.
        bound_mekso_operand,
        /// The `simple_mekso_operand` alternative of operand.
        simple_mekso_operand,
    }

    /// Syntax model for operand connective parsed by the `bound_mekso_operand` grammar rule.
    rule "operand connective" bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        /// The shared left expression child syntax node.
        field left_expression <- arc(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
        /// The operand connective component of this syntax node.
        field operand_connective <- operand_connective;
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_operand);
    }

    /// Syntax model for operand parsed by the `simple_mekso_operand` grammar rule.
    rule "operand" simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// The `forethought_mekso_operand` alternative of operand.
        forethought_mekso_operand,
        /// The `qualified_mekso_operand` alternative of operand.
        qualified_mekso_operand,
        /// The `parenthesized_mekso_operand` alternative of operand.
        parenthesized_mekso_operand,
        /// The `sumti_mekso_operand` alternative of operand.
        sumti_mekso_operand,
        /// The `selbri_mekso_operand` alternative of operand.
        selbri_mekso_operand,
        /// The `array_mekso_operand` alternative of operand.
        array_mekso_operand,
        /// The `number_mekso` alternative of operand.
        number_mekso,
        /// The `lerfu_string_mekso` alternative of operand.
        lerfu_string_mekso,
        /// The `zantufa_scalar_negated_mekso_operand` alternative of operand.
        when feature(ZantufaMex) zantufa_scalar_negated_mekso_operand,
        /// The `zantufa_selbri_mohe_mekso_operand` alternative of operand.
        when feature(ZantufaMex) zantufa_selbri_mohe_mekso_operand,
    }

    /// Syntax model for scalar-negated operand parsed by the `zantufa_scalar_negated_mekso_operand` grammar rule.
    rule "scalar-negated operand" zantufa_scalar_negated_mekso_operand(mekso_operand) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso_operand);
    }

    /// Syntax model for qualified operand parsed by the `qualified_mekso_operand` grammar rule.
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

    /// Syntax model for forethought mex parsed by the `forethought_mekso_operand` grammar rule.
    rule "forethought mex" forethought_mekso_operand(mekso_operand, tense_modal) -> struct {
        /// The gek component of this syntax node.
        field gek <- modal_forethought_connective(tense_modal);
        /// The shared left expression child syntax node.
        field left_expression <- arc(mekso_operand);
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_operand);
    }

    /// Syntax model for sumti operand parsed by the `sumti_mekso_operand` grammar rule.
    rule "sumti operand" sumti_mekso_operand(sumti) -> struct {
        /// The `Mohe` cmavo marker.
        field mohe <- cmavo(Mohe).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for selbri operand parsed by the `zantufa_selbri_mohe_mekso_operand` grammar rule.
    rule "selbri operand" zantufa_selbri_mohe_mekso_operand(selbri) -> struct {
        /// The `Mohe` cmavo marker.
        field mohe <- cmavo(Mohe).warn(ExperimentalZantufaMex).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for selbri operand parsed by the `selbri_mekso_operand` grammar rule.
    rule "selbri operand" selbri_mekso_operand(selbri) -> struct {
        /// The `Nihe` cmavo marker.
        field nihe <- cmavo(Nihe).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for parenthesized mex parsed by the `parenthesized_mekso_operand` grammar rule.
    rule "parenthesized mex" parenthesized_mekso_operand(mekso) -> struct {
        /// The `Vei` cmavo marker.
        field vei <- cmavo(Vei).wf();
        /// The shared inner expression child syntax node.
        field inner_expression <- arc(mekso);
        /// The optional `Veho` cmavo marker.
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// Syntax model for mekso array parsed by the `array_mekso_operand` grammar rule.
    rule "mekso array" array_mekso_operand(mekso) -> struct {
        /// The `Johi` cmavo marker.
        field johi <- cmavo(Johi).wf();
        /// Non-empty ordered sequence of expressions components.
        field expressions <- [one_or_more mekso];
        /// The optional `Tehu` cmavo marker.
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Syntax model for lerfu string parsed by the `letter_string` grammar rule.
    rule "lerfu string" letter_string(letter_tokens) -> struct {
        /// The shared first letter child syntax node.
        field first_letter <- arc(letter_tokens);
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more letter_string_continuation(letter_tokens)];
    }

    /// Syntax model for lerfu string continuation parsed by the `letter_string_continuation` grammar rule.
    rule "lerfu string continuation" letter_string_continuation(letter_tokens) -> enum {
        /// The `letter_string_pa_continuation` alternative of lerfu string continuation.
        letter_string_pa_continuation,
        /// The `letter_string_lerfu_continuation` alternative of lerfu string continuation.
        letter_string_lerfu_continuation,
    }

    /// Syntax model for lerfu string continuation parsed by the `letter_string_pa_continuation` grammar rule.
    rule "lerfu string continuation" letter_string_pa_continuation -> struct {
        /// The pa component of this syntax node.
        field pa <- pa_word();
    }

    /// Syntax model for lerfu string continuation parsed by the `letter_string_lerfu_continuation` grammar rule.
    rule "lerfu string continuation" letter_string_lerfu_continuation(letter_tokens) -> struct {
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Syntax model for number parsed by the `number_words` grammar rule.
    rule "number" number_words(letter_tokens) -> struct {
        /// The first number component of this syntax node.
        field first_number <- pa_word();
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more number_word_continuation(letter_tokens)];
    }

    /// Syntax model for number continuation parsed by the `number_word_continuation` grammar rule.
    rule "number continuation" number_word_continuation(letter_tokens) -> enum {
        /// The `number_word_pa_continuation` alternative of number continuation.
        number_word_pa_continuation,
        /// The `number_word_lerfu_continuation` alternative of number continuation.
        number_word_lerfu_continuation,
    }

    /// Syntax model for number continuation parsed by the `number_word_pa_continuation` grammar rule.
    rule "number continuation" number_word_pa_continuation -> struct {
        /// The pa component of this syntax node.
        field pa <- pa_word();
    }

    /// Syntax model for number continuation parsed by the `number_word_lerfu_continuation` grammar rule.
    rule "number continuation" number_word_lerfu_continuation(letter_tokens) -> struct {
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Syntax model for number or lerfu string parsed by the `number_or_letter_words` grammar rule.
    rule "number or lerfu string" number_or_letter_words(letter_tokens, letter_string) -> enum {
        /// The `number_words` alternative of number or lerfu string.
        number_words,
        /// The `letter_string` alternative of number or lerfu string.
        letter_string,
    }

    /// Syntax model for lerfu word parsed by the `letter_tokens` grammar rule.
    rule "lerfu word" letter_tokens(letter_string, letter_tokens) -> enum {
        /// The `simple_lerfu_word` alternative of lerfu word.
        simple_lerfu_word,
        /// The `lau_lerfu_word` alternative of lerfu word.
        lau_lerfu_word,
        /// The `tei_lerfu_word` alternative of lerfu word.
        tei_lerfu_word,
    }

    /// Syntax model for lerfu word parsed by the `simple_lerfu_word` grammar rule.
    rule "lerfu word" simple_lerfu_word -> struct {
        /// The word component of this syntax node.
        field word <- word_category(LetterWord);
    }

    /// Syntax model for lerfu word parsed by the `lau_lerfu_word` grammar rule.
    rule "lerfu word" lau_lerfu_word(letter_tokens) -> struct {
        /// A word from selmaho `Lau`.
        field lau <- selmaho(Lau);
        /// The shared letter child syntax node.
        field letter <- arc(letter_tokens);
    }

    /// Syntax model for lerfu word parsed by the `tei_lerfu_word` grammar rule.
    rule "lerfu word" tei_lerfu_word(letter_string) -> struct {
        /// The `Tei` cmavo marker.
        field tei <- cmavo(Tei);
        /// The shared letters child syntax node.
        field letters <- arc(letter_string);
        /// The `Foi` cmavo marker.
        field foi <- cmavo(Foi);
    }

    /// Syntax model for lerfu string parsed by the `lerfu_string_mekso` grammar rule.
    rule "lerfu string" lerfu_string_mekso(letter_string, free_modifier) -> struct {
        /// The letters component of this syntax node.
        field letters <- letter_string;
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Syntax model for mex parsed by the `mekso_base` grammar rule.
    rule "mex" mekso_base(mekso, mekso_base, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator) -> enum {
        /// The `zantufa_bo_grouped_mekso_base` alternative of mex.
        when feature(ZantufaMex) zantufa_bo_grouped_mekso_base,
        /// The `mekso_operand` alternative of mex.
        mekso_operand,
        /// The `forethought_call_mekso` alternative of mex.
        forethought_call_mekso,
        /// The `zantufa_grouped_mekso_operand_sequence` alternative of mex.
        when feature(ZantufaMex) zantufa_grouped_mekso_operand_sequence,
    }

    /// Syntax model for grouped mex parsed by the `zantufa_bo_grouped_mekso_base` grammar rule.
    rule "grouped mex" zantufa_bo_grouped_mekso_base(mekso_operand) -> struct {
        /// The shared first child syntax node.
        field first <- arc(mekso_operand);
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more zantufa_bo_grouped_mekso_continuation(mekso_operand)];
    }

    /// Syntax model for grouped mex parsed by the `zantufa_bo_grouped_mekso_continuation` grammar rule.
    rule "grouped mex" zantufa_bo_grouped_mekso_continuation(mekso_operand) -> struct {
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).warn(ExperimentalZantufaMex).wf();
        /// The shared expression child syntax node.
        field expression <- arc(mekso_operand);
    }

    /// Syntax model for grouped mex parsed by the `zantufa_grouped_mekso_operand_sequence` grammar rule.
    rule "grouped mex" zantufa_grouped_mekso_operand_sequence(mekso_operand) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).warn(ExperimentalZantufaMex).wf();
        /// Non-empty ordered sequence of operands components.
        field operands <- [one_or_more arc(mekso_operand)];
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for mex parsed by the `mekso_precedence` grammar rule.
    rule "mex" mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> struct {
        /// The shared left expression child syntax node.
        field left_expression <- arc(mekso_base);
        /// The optional tail component.
        field tail <- opt(mekso_precedence_tail(mekso_precedence, mekso_operator));
    }

    /// Syntax model for mex precedence tail parsed by the `mekso_precedence_tail` grammar rule.
    rule "mex precedence tail" mekso_precedence_tail(mekso_precedence, mekso_operator) -> struct {
        /// The `Bihe` cmavo marker.
        field bihe <- cmavo(Bihe).wf();
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_precedence);
    }

    /// Syntax model for mex parsed by the `infix_mekso` grammar rule.
    rule "mex" infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        /// The shared first expression child syntax node.
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    /// Syntax model for mex continuation parsed by the `infix_mekso_continuation` grammar rule.
    rule "mex continuation" infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// The shared right expression child syntax node.
        field right_expression <- arc(mekso_precedence);
    }

    /// Syntax model for mex parsed by the `zantufa_infix_mekso` grammar rule.
    rule "mex" zantufa_infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        /// The shared first expression child syntax node.
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    /// Syntax model for mex continuation parsed by the `zantufa_infix_mekso_continuation` grammar rule.
    rule "mex continuation" zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        /// Non-empty ordered sequence of operators components.
        field operators <- [one_or_more arc(mekso_operator)];
        /// The optional right expression component.
        field right_expression <- opt(arc(mekso_precedence));
    }

    /// Syntax model for forethought mex parsed by the `forethought_call_mekso` grammar rule.
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

    /// Syntax model for mex parsed by the `mekso` grammar rule.
    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts) -> enum {
        /// The `zantufa_reverse_polish_mekso` alternative of mex.
        when feature(ZantufaMex) zantufa_reverse_polish_mekso,
        /// The `zantufa_infix_mekso` alternative of mex.
        when feature(ZantufaMex) zantufa_infix_mekso,
        /// The `infix_mekso` alternative of mex.
        infix_mekso,
        /// The `reverse_polish_mekso` alternative of mex.
        reverse_polish_mekso,
    }

    /// Syntax model for reverse Polish mex parsed by the `zantufa_reverse_polish_mekso` grammar rule.
    rule "reverse Polish mex" zantufa_reverse_polish_mekso(mekso_base, mekso_operator) -> struct {
        /// The `Fuha` cmavo marker.
        field fuha <- cmavo(Fuha).warn(ExperimentalZantufaMex).wf();
        /// Non-empty ordered sequence of operands components.
        field operands <- [one_or_more mekso_base];
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
        /// Ordered sequence of zero or more tails components.
        field tails <- [zero_or_more zantufa_reverse_polish_tail(mekso_base, mekso_operator)];
        /// The optional `Kuhe` cmavo marker.
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Syntax model for reverse Polish mex tail parsed by the `zantufa_reverse_polish_tail` grammar rule.
    rule "reverse Polish mex tail" zantufa_reverse_polish_tail(mekso_base, mekso_operator) -> struct {
        /// Ordered sequence of zero or more operands components.
        field operands <- [zero_or_more mekso_base];
        /// The shared operator child syntax node.
        field operator <- arc(mekso_operator);
    }

    /// Syntax model for reverse Polish mex parsed by the `reverse_polish_parts` grammar rule.
    rule "reverse Polish mex" reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> struct {
        /// The shared first operand child syntax node.
        field first_operand <- arc(mekso_operand);
        /// Ordered sequence of zero or more tails components.
        field tails <- [zero_or_more reverse_polish_parts_tail(reverse_polish_parts, mekso_operator)];
    }

    /// Syntax model for reverse Polish mex tail parsed by the `reverse_polish_parts_tail` grammar rule.
    rule "reverse Polish mex tail" reverse_polish_parts_tail(reverse_polish_parts, mekso_operator) -> struct {
        /// The shared right parts child syntax node.
        field right_parts <- arc(reverse_polish_parts);
        /// The operator component of this syntax node.
        field operator <- mekso_operator;
    }

    /// Syntax model for reverse Polish mex parsed by the `reverse_polish_mekso` grammar rule.
    rule "reverse Polish mex" reverse_polish_mekso(reverse_polish_parts) -> struct {
        /// The `Fuha` cmavo marker.
        field fuha <- cmavo(Fuha).wf();
        /// The shared parts child syntax node.
        field parts <- arc(reverse_polish_parts);
    }

    /// Syntax model for number sumti parsed by the `number_sumti` grammar rule.
    rule "number sumti" number_sumti(mekso) -> struct {
        /// A word from selmaho `Li`.
        field li <- selmaho(Li).wf();
        #[tree_child(primary)]
        /// The shared expression child syntax node.
        field expression <- arc(mekso);
        /// The optional `Loho` cmavo marker.
        field loho <- opt(cmavo(Loho).wf()).elidable_terminator(Loho);
    }

    /// Syntax model for lerfu string parsed by the `lerfu_string_sumti` grammar rule.
    rule "lerfu string" lerfu_string_sumti(letter_string, free_modifier) -> struct {
        /// The words component of this syntax node.
        field words <- letter_string;
        assert !selmaho(Moi);
        assert !selmaho(Mai);
        /// The optional `Boi` cmavo marker.
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        /// Ordered sequence of zero or more free modifiers components.
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Syntax model for converted sumti parsed by the `lahe_sumti` grammar rule.
    rule "converted sumti" lahe_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        /// A word from selmaho `Lahe`.
        field lahe <- selmaho(Lahe).wf();
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for converted term parsed by the `lahe_term_wrapper` grammar rule.
    rule "converted term" lahe_term_wrapper(term) -> struct {
        /// A word from selmaho `Lahe`.
        field lahe <- selmaho(Lahe).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for scalar-negated term parsed by the `scalar_negated_term_wrapper_with_bo` grammar rule.
    rule "scalar-negated term" scalar_negated_term_wrapper_with_bo(term) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for scalar-negated term parsed by the `scalar_negated_term_wrapper` grammar rule.
    rule "scalar-negated term" scalar_negated_term_wrapper(term) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        /// The shared inner term child syntax node.
        field inner_term <- arc(term);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for scalar-negated sumti parsed by the `scalar_negated_sumti_with_bo` grammar rule.
    rule "scalar-negated sumti" scalar_negated_sumti_with_bo(sumti) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe);
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for scalar-negated sumti parsed by the `scalar_negated_sumti` grammar rule.
    rule "scalar-negated sumti" scalar_negated_sumti(sumti) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        /// The shared inner sumti child syntax node.
        field inner_sumti <- arc(sumti);
        /// The optional `Luhu` cmavo marker.
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Syntax model for bridi description parsed by the `bridi_description_sumti` grammar rule.
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

    /// Syntax model for bridi description parsed by the `lohoi_description_head_continuation` grammar rule.
    rule "bridi description" lohoi_description_head_continuation -> struct {
        /// The connective component of this syntax node.
        field connective <- joik_connective;
        /// A word from selmaho `Lohoi`.
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
    }

    /// Syntax model for sumti parsed by the `pro_sumti` grammar rule.
    rule "sumti" pro_sumti -> struct {
        /// The koha component of this syntax node.
        field koha <- word_category(ProSumti).wf();
    }

    /// Syntax model for name parsed by the `name_sumti` grammar rule.
    rule "name" name_sumti -> struct {
        /// A word from selmaho `La`.
        field la <- selmaho(La).wf();
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
    }

    /// Syntax model for descriptor parsed by the `description_head` grammar rule.
    rule "descriptor" description_head -> struct {
        /// A word from selmaho `Le`.
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    /// Syntax model for descriptor connective parsed by the `description_head_connective` grammar rule.
    rule "descriptor connective" description_head_connective -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(jek_connective);
    }

    /// Syntax model for description parsed by the `description_connection_sumti` grammar rule.
    rule "description" description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        /// The shared leading description head child syntax node.
        field leading_description_head <- arc(description_head());
        /// The connective component of this syntax node.
        field connective <- description_head_connective();
        /// The shared trailing description head child syntax node.
        field trailing_description_head <- arc(description_head());
        /// The tail component of this syntax node.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Syntax model for description parsed by the `descriptor_with_gadri_sumti` grammar rule.
    rule "description" descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        /// The description component of this syntax node.
        field description <- description_head();
        /// The tail component of this syntax node.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Syntax model for description parsed by the `descriptor_with_outer_quantifier_sumti` grammar rule.
    rule "description" descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        /// The outer quantifier component of this syntax node.
        field outer_quantifier <- quantifier(mekso, letter_tokens);
        /// The description component of this syntax node.
        field description <- description_head();
        /// The tail component of this syntax node.
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Syntax model for description parsed by the `descriptor_without_gadri_sumti` grammar rule.
    rule "description" descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        /// The quantifier component of this syntax node.
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Ku` cmavo marker.
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for description tail parsed by the `description_tail` grammar rule.
    rule "description tail" description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        /// The leading tail elements component of this syntax node.
        field leading_tail_elements <- leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement);
        /// The shared tail child syntax node.
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement));
    }

    /// Syntax model for description tail parsed by the `description_tail_body` grammar rule.
    rule "description tail" description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// The `quantifier_relation_description_tail` alternative of description tail.
        quantifier_relation_description_tail,
        /// The `quantifier_sumti_description_tail` alternative of description tail.
        quantifier_sumti_description_tail,
        /// The `relation_description_tail` alternative of description tail.
        relation_description_tail,
    }

    /// Syntax model for description tail parsed by the `leading_description_tail_elements` grammar rule.
    rule "description tail" leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement) -> struct {
        /// The optional tail sumti component.
        field tail_sumti <- opt(description_tail_sumti(sumti_base));
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for description tail parsed by the `description_tail_sumti` grammar rule.
    rule "description tail" description_tail_sumti(sumti_base) -> struct {
        assert !pa_word();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_base);
    }

    /// Syntax model for description tail parsed by the `relation_description_tail` grammar rule.
    rule "description tail" relation_description_tail(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for description tail parsed by the `quantifier_relation_description_tail` grammar rule.
    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        /// The quantifier component of this syntax node.
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional relative clauses component.
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for description tail parsed by the `quantifier_sumti_description_tail` grammar rule.
    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> struct {
        /// The quantifier component of this syntax node.
        field quantifier <- quantifier(mekso, letter_tokens);
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for quote parsed by the `quote` grammar rule.
    rule "quote" quote(text) -> enum {
        /// The `experimental_mehoi_compound_quote` alternative of quote.
        experimental_mehoi_compound_quote,
        /// The `experimental_zohoi_compound_quote` alternative of quote.
        experimental_zohoi_compound_quote,
        /// The `experimental_rahoi_compound_quote` alternative of quote.
        experimental_rahoi_compound_quote,
        /// The `experimental_gohoi_compound_quote` alternative of quote.
        experimental_gohoi_compound_quote,
        /// The `generic_compound_quote` alternative of quote.
        generic_compound_quote,
        /// The `text_quote` alternative of quote.
        text_quote,
    }

    /// Syntax model for text quote parsed by the `text_quote` grammar rule.
    rule "text quote" text_quote(text) -> struct {
        /// The `Lu` cmavo marker.
        field lu <- cmavo(Lu).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Lihu` cmavo marker.
        field lihu <- opt(cmavo(Lihu).wf()).elidable_terminator(Lihu);
    }

    /// Syntax model for quote parsed by the `experimental_mehoi_compound_quote` grammar rule.
    rule "quote" experimental_mehoi_compound_quote -> struct {
        /// The quote component of this syntax node.
        field quote <- quote_marker(Mehoi).warn(ExperimentalMehOiQuote).wf();
    }

    /// Syntax model for quote parsed by the `experimental_zohoi_compound_quote` grammar rule.
    rule "quote" experimental_zohoi_compound_quote -> struct {
        /// The quote component of this syntax node.
        field quote <- choice((
            quote_marker(Zohoi),
            quote_marker(Lahoi),
        )).warn(ExperimentalZohOiQuote).wf();
    }

    /// Syntax model for quote parsed by the `experimental_rahoi_compound_quote` grammar rule.
    rule "quote" experimental_rahoi_compound_quote -> struct {
        /// The quote component of this syntax node.
        field quote <- quote_marker(Rahoi).warn(ExperimentalZantufaRahoiQuote).wf();
    }

    /// Syntax model for quote parsed by the `experimental_gohoi_compound_quote` grammar rule.
    rule "quote" experimental_gohoi_compound_quote -> struct {
        /// The quote component of this syntax node.
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Syntax model for quote parsed by the `generic_compound_quote` grammar rule.
    rule "quote" generic_compound_quote -> struct {
        /// The quote component of this syntax node.
        field quote <- word_category(Quote).wf();
    }

    /// Syntax model for quote parsed by the `quoted_sumti` grammar rule.
    rule "quote" quoted_sumti(text) -> struct {
        #[tree_child(primary)]
        /// The shared quote child syntax node.
        field quote <- arc(quote(text));
    }

    /// Syntax model for vocative phrase parsed by the `selbri_vocative_sumti` grammar rule.
    rule "vocative phrase" selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for vocative phrase parsed by the `cmevla_vocative_sumti` grammar rule.
    rule "vocative phrase" cmevla_vocative_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        /// The optional leading relative clauses component.
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        /// Non-empty ordered sequence of names components.
        field names <- [one_or_more cmevla_word()].wf();
        /// The optional trailing relative clauses component.
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for vocative phrase parsed by the `vocative_sumti` grammar rule.
    rule "vocative phrase" vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> enum {
        /// The `selbri_vocative_sumti` alternative of vocative phrase.
        selbri_vocative_sumti,
        /// The `cmevla_vocative_sumti` alternative of vocative phrase.
        cmevla_vocative_sumti,
        /// The `sumti` alternative of vocative phrase.
        sumti,
    }

    /// Syntax model for vocative marker parsed by the `vocative_marker_words` grammar rule.
    rule "vocative marker" vocative_marker_words -> enum {
        /// The `coi_vocative_marker_words` alternative of vocative marker.
        coi_vocative_marker_words,
        /// The `doi_vocative_marker_words` alternative of vocative marker.
        doi_vocative_marker_words,
    }

    /// Syntax model for vocative marker parsed by the `coi_vocative_marker_words` grammar rule.
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

    /// Syntax model for vocative marker parsed by the `additional_coi_vocative_marker` grammar rule.
    rule "vocative marker" additional_coi_vocative_marker -> struct {
        /// A word from selmaho `Coi`.
        field coi <- selmaho(Coi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Syntax model for vocative marker parsed by the `doi_vocative_marker_words` grammar rule.
    rule "vocative marker" doi_vocative_marker_words -> struct {
        /// The `Doi` cmavo marker.
        field doi <- cmavo(Doi);
    }

    /// Syntax model for free modifier parsed by the `free_modifier` grammar rule.
    rule "free modifier" free_modifier(sumti, subbridi, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier, statement) -> enum {
        /// The `text_replacement_free_modifier` alternative of free modifier.
        text_replacement_free_modifier,
        /// The `zantufa_sei_statement_free_modifier` alternative of free modifier.
        when feature(ZantufaTerms) zantufa_sei_statement_free_modifier,
        /// The `sei_free_modifier` alternative of free modifier.
        sei_free_modifier,
        /// The `xi_free_modifier` alternative of free modifier.
        xi_free_modifier,
        /// The `mai_free_modifier` alternative of free modifier.
        mai_free_modifier,
        /// The `zantufa_mekso_mai_free_modifier` alternative of free modifier.
        when feature(ZantufaMex) zantufa_mekso_mai_free_modifier,
        /// The `soi_free_modifier` alternative of free modifier.
        soi_free_modifier,
        /// The `parenthetical_text` alternative of free modifier.
        parenthetical_text,
        /// The `vocative_free_modifier` alternative of free modifier.
        vocative_free_modifier,
    }

    /// Syntax model for vocative phrase parsed by the `vocative_free_modifier` grammar rule.
    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        /// The vocative markers component of this syntax node.
        field vocative_markers <- vocative_marker_words().wf();
        /// The optional sumti component.
        field sumti <- opt(arc(vocative_sumti(sumti, subbridi, selbri, tense_modal, statement)));
        /// The optional `Dohu` cmavo marker.
        field dohu <- opt(cmavo(Dohu).prohibited_wf()).elidable_terminator(Dohu);
    }

    /// Syntax model for parenthetical text parsed by the `parenthetical_text` grammar rule.
    rule "parenthetical text" parenthetical_text(text) -> struct {
        /// A word from selmaho `To`.
        field to <- selmaho(To).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Toi` cmavo marker.
        field toi <- opt(cmavo(Toi).prohibited_wf()).elidable_terminator(Toi);
    }

    /// Syntax model for metalinguistic comment parsed by the `sei_free_modifier` grammar rule.
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

    /// Syntax model for metalinguistic comment parsed by the `zantufa_sei_statement_free_modifier` grammar rule.
    rule "metalinguistic comment" zantufa_sei_statement_free_modifier(statement) -> struct {
        /// A word from selmaho `Sei`.
        field sei <- selmaho(Sei).warn(ExperimentalZantufaStatementFreeModifier).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Sehu` cmavo marker.
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    /// Syntax model for subscript parsed by the `xi_free_modifier` grammar rule.
    rule "subscript" xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier) -> enum {
        /// The `xi_number_free_modifier` alternative of subscript.
        xi_number_free_modifier,
        /// The `xi_lerfu_string_free_modifier` alternative of subscript.
        xi_lerfu_string_free_modifier,
        /// The `xi_parenthesized_free_modifier` alternative of subscript.
        xi_parenthesized_free_modifier,
    }

    /// Syntax model for subscript parsed by the `xi_number_free_modifier` grammar rule.
    rule "subscript" xi_number_free_modifier(letter_tokens) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(number_mekso(letter_tokens));
    }

    /// Syntax model for subscript parsed by the `xi_lerfu_string_free_modifier` grammar rule.
    rule "subscript" xi_lerfu_string_free_modifier(letter_string, free_modifier) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(lerfu_string_mekso(letter_string, free_modifier));
    }

    /// Syntax model for subscript parsed by the `xi_parenthesized_free_modifier` grammar rule.
    rule "subscript" xi_parenthesized_free_modifier(mekso) -> struct {
        /// A word from selmaho `Xi`.
        field xi <- selmaho(Xi).wf();
        /// The shared expression child syntax node.
        field expression <- arc(parenthesized_mekso_operand(mekso));
    }

    /// Syntax model for utterance ordinal parsed by the `mai_free_modifier` grammar rule.
    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        /// The number component of this syntax node.
        field number <- number_or_letter_words(letter_tokens, letter_string)
            .followed_by(selmaho(Mai).ignored());
        /// A word from selmaho `Mai`.
        field mai <- selmaho(Mai).wf();
    }

    /// Syntax model for utterance ordinal parsed by the `zantufa_mekso_mai_free_modifier` grammar rule.
    rule "utterance ordinal" zantufa_mekso_mai_free_modifier(mekso) -> struct {
        /// A word from selmaho `Mai`.
        field expression <- arc(mekso.followed_by(selmaho(Mai).ignored()));
        /// A word from selmaho `Mai`.
        field mai <- selmaho(Mai).warn(ExperimentalZantufaMex).wf();
    }

    /// Syntax model for reciprocal parsed by the `soi_free_modifier` grammar rule.
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

    /// Syntax model for replacement phrase parsed by the `text_replacement_free_modifier` grammar rule.
    rule "replacement phrase" text_replacement_free_modifier -> enum {
        /// The `full_text_replacement_free_modifier` alternative of replacement phrase.
        full_text_replacement_free_modifier,
        /// The `new_only_text_replacement_free_modifier` alternative of replacement phrase.
        new_only_text_replacement_free_modifier,
        /// The `close_only_text_replacement_free_modifier` alternative of replacement phrase.
        close_only_text_replacement_free_modifier,
    }

    alias "replacement free modifier word" word_before_sahai_or_lehai =
        word_not_cmavo(Sahai, Lehai);

    alias "replacement free modifier word" word_before_lehai =
        word_not_cmavo(Lehai);

    /// Syntax model for replacement phrase parsed by the `full_text_replacement_free_modifier` grammar rule.
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

    /// Syntax model for replacement phrase parsed by the `new_only_text_replacement_free_modifier` grammar rule.
    rule "replacement phrase" new_only_text_replacement_free_modifier -> struct {
        /// The `Sahai` cmavo marker.
        field sahai <- cmavo(Sahai);
        /// Ordered sequence of zero or more new words components.
        field new_words <- [zero_or_more word_before_lehai()];
        /// The `Lehai` cmavo marker.
        field lehai <- cmavo(Lehai).wf();
    }

    /// Syntax model for replacement phrase parsed by the `close_only_text_replacement_free_modifier` grammar rule.
    rule "replacement phrase" close_only_text_replacement_free_modifier -> struct {
        /// The `Lehai` cmavo marker.
        field lehai <- cmavo(Lehai).wf();
    }

    /// Syntax model for relative clauses parsed by the `relative_clause_tail` grammar rule.
    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal, statement) -> enum {
        /// The `joined_relative_clause_tail` alternative of relative clauses.
        joined_relative_clause_tail,
        /// The `connected_relative_clause_tail` alternative of relative clauses.
        connected_relative_clause_tail,
    }

    /// Syntax model for relative clause parsed by the `joined_relative_clause_tail` grammar rule.
    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        /// The `Zihe` cmavo marker.
        field zihe <- cmavo(Zihe).wf();
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for relative clause parsed by the `connected_relative_clause_tail` grammar rule.
    rule "relative clause" connected_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        /// The connective component of this syntax node.
        field connective <- relative_clause_connective;
        /// The shared inner child syntax node.
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    /// Syntax model for relative clause connective parsed by the `relative_clause_connective` grammar rule.
    rule "relative clause connective" relative_clause_connective -> enum {
        /// The `joik_connective` alternative of relative clause connective.
        joik_connective,
        /// The `jek_connective` alternative of relative clause connective.
        jek_connective,
    }

    /// Syntax model for relative clause parsed by the `relative_clause_atom` grammar rule.
    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal, statement) -> enum {
        /// The `sumti_association_relative_clause` alternative of relative clause.
        sumti_association_relative_clause,
        /// The `bridi_relative_clause` alternative of relative clause.
        bridi_relative_clause,
    }

    /// Syntax model for sumti association phrase parsed by the `sumti_association_relative_clause` grammar rule.
    rule "sumti association phrase" sumti_association_relative_clause(sumti, tense_modal) -> struct {
        /// A word from selmaho `Goi`.
        field association_marker <- selmaho(Goi).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(relative_sumti(sumti, tense_modal));
        /// The optional `Gehu` cmavo marker.
        field gehu <- opt(cmavo(Gehu).wf()).elidable_terminator(Gehu);
    }

    /// Syntax model for sumti association phrase parsed by the `relative_sumti` grammar rule.
    rule "sumti association phrase" relative_sumti(sumti, tense_modal) -> enum {
        /// The `tense_tagged_relative_sumti` alternative of sumti association phrase.
        tense_tagged_relative_sumti,
        /// The `na_ku_relative_sumti` alternative of sumti association phrase.
        na_ku_relative_sumti,
        /// The `plain_relative_sumti` alternative of sumti association phrase.
        plain_relative_sumti,
    }

    /// Syntax model for sumti association phrase parsed by the `na_ku_relative_sumti` grammar rule.
    rule "sumti association phrase" na_ku_relative_sumti -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na);
        /// The `Ku` cmavo marker.
        field ku <- cmavo(Ku).wf();
    }

    /// Syntax model for tagged sumti parsed by the `tense_tagged_relative_sumti` grammar rule.
    rule "tagged sumti" tense_tagged_relative_sumti(tense_modal, sumti) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Syntax model for sumti association phrase parsed by the `plain_relative_sumti` grammar rule.
    rule "sumti association phrase" plain_relative_sumti(sumti) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for relative bridi parsed by the `bridi_relative_clause` grammar rule.
    rule "relative bridi" bridi_relative_clause(subbridi, statement) -> enum {
        /// The `zantufa_restrictive_statement_relative_clause` alternative of relative bridi.
        when feature(ZantufaTerms) zantufa_restrictive_statement_relative_clause,
        /// The `zantufa_incidental_statement_relative_clause` alternative of relative bridi.
        when feature(ZantufaTerms) zantufa_incidental_statement_relative_clause,
        /// The `restrictive_bridi_relative_clause` alternative of relative bridi.
        restrictive_bridi_relative_clause,
        /// The `incidental_bridi_relative_clause` alternative of relative bridi.
        incidental_bridi_relative_clause,
    }

    /// Syntax model for relative clause parsed by the `zantufa_restrictive_statement_relative_clause` grammar rule.
    rule "relative clause" zantufa_restrictive_statement_relative_clause(statement) -> struct {
        /// The poi component of this syntax node.
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

    /// Syntax model for relative clause parsed by the `zantufa_incidental_statement_relative_clause` grammar rule.
    rule "relative clause" zantufa_incidental_statement_relative_clause(statement) -> struct {
        /// The noi component of this syntax node.
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        /// The shared statement child syntax node.
        field statement <- arc(statement);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Syntax model for relative clause parsed by the `restrictive_bridi_relative_clause` grammar rule.
    rule "relative clause" restrictive_bridi_relative_clause(subbridi, statement) -> struct {
        /// The poi component of this syntax node.
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

    /// Syntax model for relative clause parsed by the `incidental_bridi_relative_clause` grammar rule.
    rule "relative clause" incidental_bridi_relative_clause(subbridi, statement) -> struct {
        /// The noi component of this syntax node.
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).wf();
        /// The shared subbridi child syntax node.
        field subbridi <- arc(subbridi);
        /// The optional `Kuho` cmavo marker.
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Syntax model for ek parsed by the `ek_connective` grammar rule.
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

    /// Syntax model for ek parsed by the `jehi_connective` grammar rule.
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

    /// Syntax model for jek parsed by the `jek_connective` grammar rule.
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

    /// Syntax model for joik parsed by the `joik_connective` grammar rule.
    rule "joik" joik_connective -> enum {
        /// The `joi_connective` alternative of joik.
        joi_connective,
        /// The `simple_interval_connective` alternative of joik.
        simple_interval_connective,
        /// The `closed_interval_connective` alternative of joik.
        closed_interval_connective,
    }

    /// Syntax model for joik parsed by the `joi_connective` grammar rule.
    rule "joik" joi_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Joi`.
        field joi <- selmaho(Joi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for interval parsed by the `simple_interval_connective` grammar rule.
    rule "interval" simple_interval_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for interval parsed by the `closed_interval_connective` grammar rule.
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

    /// Syntax model for non-logical connective parsed by the `vuhu_nonlogical_connective` grammar rule.
    rule "non-logical connective" vuhu_nonlogical_connective -> struct {
        #[tree_child(primary)]
        /// A word from selmaho `Vuhu`.
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Syntax model for sumti connective parsed by the `argument_connective` grammar rule.
    rule "sumti connective" argument_connective -> enum {
        /// The `cehe_connective` alternative of sumti connective.
        cehe_connective,
        /// The `ek_connective` alternative of sumti connective.
        ek_connective,
        /// The `jehi_connective` alternative of sumti connective.
        jehi_connective,
        /// The `joik_connective` alternative of sumti connective.
        joik_connective,
        /// The `vuhu_nonlogical_connective` alternative of sumti connective.
        vuhu_nonlogical_connective,
    }

    /// Syntax model for operand connective parsed by the `operand_connective` grammar rule.
    rule "operand connective" operand_connective -> enum {
        /// The `joik_connective` alternative of operand connective.
        joik_connective,
        /// The `ek_connective` alternative of operand connective.
        ek_connective,
        /// The `jek_connective` alternative of operand connective.
        jek_connective,
    }

    /// Syntax model for selbri connective parsed by the `relation_afterthought_connective` grammar rule.
    rule "selbri connective" relation_afterthought_connective -> enum {
        /// The `joik_connective` alternative of selbri connective.
        joik_connective,
        /// The `jek_connective` alternative of selbri connective.
        jek_connective,
        /// The `ek_connective` alternative of selbri connective.
        ek_connective,
        /// The `vuhu_nonlogical_connective` alternative of selbri connective.
        vuhu_nonlogical_connective,
    }

    /// Syntax model for statement connective parsed by the `standard_statement_connective` grammar rule.
    rule "statement connective" standard_statement_connective -> enum {
        /// The `joik_connective` alternative of statement connective.
        joik_connective,
        /// The `jek_connective` alternative of statement connective.
        jek_connective,
    }

    /// Syntax model for statement connective parsed by the `statement_connective` grammar rule.
    rule "statement connective" statement_connective -> enum {
        /// The `joik_connective` alternative of statement connective.
        joik_connective,
        /// The `jek_connective` alternative of statement connective.
        jek_connective,
        /// The `ek_connective` alternative of statement connective.
        ek_connective,
        /// The `vuhu_nonlogical_connective` alternative of statement connective.
        vuhu_nonlogical_connective,
    }

    /// Syntax model for text connective parsed by the `text_leading_connective` grammar rule.
    rule "text connective" text_leading_connective -> enum {
        /// The `standard_statement_connective` alternative of text connective.
        standard_statement_connective,
        /// The `cehe_connective` alternative of text connective.
        cehe_connective,
    }

    /// Syntax model for statement connective parsed by the `i_statement_connective` grammar rule.
    rule "statement connective" i_statement_connective(tense_modal) -> enum {
        /// The `i_standard_statement_connective` alternative of statement connective.
        i_standard_statement_connective,
        /// The `i_tag_bo_statement_connective` alternative of statement connective.
        i_tag_bo_statement_connective,
    }

    /// Syntax model for statement connective parsed by the `i_standard_statement_connective` grammar rule.
    rule "statement connective" i_standard_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(statement_connective);
        /// The optional `Bo` cmavo marker.
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo).wf()));
    }

    /// Syntax model for statement connective parsed by the `i_paragraph_statement_connective` grammar rule.
    rule "statement connective" i_paragraph_statement_connective(tense_modal) -> enum {
        /// The `i_standard_paragraph_statement_connective` alternative of statement connective.
        i_standard_paragraph_statement_connective,
        /// The `i_tag_bo_paragraph_statement_connective` alternative of statement connective.
        i_tag_bo_paragraph_statement_connective,
    }

    /// Syntax model for statement connective parsed by the `i_standard_paragraph_statement_connective` grammar rule.
    rule "statement connective" i_standard_paragraph_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(paragraph_standard_statement_connective);
        /// The optional `Bo` cmavo marker.
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo)));
    }

    /// Syntax model for statement connective parsed by the `paragraph_standard_statement_connective` grammar rule.
    rule "statement connective" paragraph_standard_statement_connective -> enum {
        /// The `paragraph_joi_connective` alternative of statement connective.
        paragraph_joi_connective,
        /// The `paragraph_simple_interval_connective` alternative of statement connective.
        paragraph_simple_interval_connective,
        /// The `paragraph_closed_interval_connective` alternative of statement connective.
        paragraph_closed_interval_connective,
        /// The `paragraph_jek_connective` alternative of statement connective.
        paragraph_jek_connective,
    }

    /// Syntax model for jek parsed by the `paragraph_jek_connective` grammar rule.
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

    /// Syntax model for joik parsed by the `paragraph_joi_connective` grammar rule.
    rule "joik" paragraph_joi_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Joi`.
        field joi <- selmaho(Joi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Syntax model for interval parsed by the `paragraph_simple_interval_connective` grammar rule.
    rule "interval" paragraph_simple_interval_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Bihi`.
        field bihi <- selmaho(Bihi);
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai));
    }

    /// Syntax model for interval parsed by the `paragraph_closed_interval_connective` grammar rule.
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

    /// Syntax model for statement connective parsed by the `i_tag_bo_paragraph_statement_connective` grammar rule.
    rule "statement connective" i_tag_bo_paragraph_statement_connective(tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo);
    }

    /// Syntax model for statement connective parsed by the `i_tag_bo_statement_connective` grammar rule.
    rule "statement connective" i_tag_bo_statement_connective(tense_modal) -> struct {
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
    }

    /// Syntax model for termset connective parsed by the `cehe_connective` grammar rule.
    rule "termset connective" cehe_connective -> struct {
        #[tree_child(primary)]
        /// The `Cehe` cmavo marker.
        field cehe <- cmavo(Cehe).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for gihek parsed by the `gihek_connective` grammar rule.
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

    /// Syntax model for forethought selbri connective parsed by the `guhek_connective` grammar rule.
    rule "forethought selbri connective" guhek_connective -> struct {
        /// The optional nahe component.
        field nahe <- opt(selmaho(Nahe));
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Guha`.
        field guha <- selmaho(Guha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for bridi tail connective parsed by the `bridi_tail_connective` grammar rule.
    rule "bridi tail connective" bridi_tail_connective -> enum {
        /// The `gihek_connective` alternative of bridi tail connective.
        gihek_connective,
        /// The `relation_connective_as_bridi_tail` alternative of bridi tail connective.
        relation_connective_as_bridi_tail,
    }

    /// Syntax model for bridi tail connective parsed by the `relation_connective_as_bridi_tail` grammar rule.
    rule "bridi tail connective" relation_connective_as_bridi_tail -> struct {
        #[tree_child(primary)]
        /// The shared connective child syntax node.
        field connective <- arc(relation_afterthought_connective);
    }

    /// Syntax model for forethought connective parsed by the `modal_forethought_connective` grammar rule.
    rule "forethought connective" modal_forethought_connective(tense_modal) -> enum {
        /// The `ga_forethought_connective` alternative of forethought connective.
        ga_forethought_connective,
        /// The `joik_jek_gi_forethought_connective` alternative of forethought connective.
        joik_jek_gi_forethought_connective,
        /// The `jek_gi_forethought_connective` alternative of forethought connective.
        jek_gi_forethought_connective,
        /// The `modal_gi_forethought_connective` alternative of forethought connective.
        modal_gi_forethought_connective,
        /// The `zantufa_initial_gi_forethought_connective` alternative of forethought connective.
        when feature(ZantufaConnectives) zantufa_initial_gi_forethought_connective,
    }

    /// Syntax model for forethought connective parsed by the `ga_forethought_connective` grammar rule.
    rule "forethought connective" ga_forethought_connective -> struct {
        /// The optional se component.
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        /// A word from selmaho `Ga`.
        field ga <- selmaho(Ga).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for forethought connective parsed by the `zantufa_initial_gi_forethought_connective` grammar rule.
    rule "forethought connective" zantufa_initial_gi_forethought_connective -> struct {
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).warn(ExperimentalZantufaGek).wf();
        /// The shared tail child syntax node.
        field tail <- arc(standard_statement_connective);
        /// The optional `Bo` cmavo marker.
        field bo <- opt(cmavo(Bo).wf());
    }

    /// Syntax model for forethought connective parsed by the `joik_jek_gi_forethought_connective` grammar rule.
    rule "forethought connective" joik_jek_gi_forethought_connective -> struct {
        /// The shared connective child syntax node.
        field connective <- arc(joik_connective);
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Bo` cmavo marker.
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Syntax model for forethought connective parsed by the `jek_gi_forethought_connective` grammar rule.
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

    /// Syntax model for forethought connective parsed by the `modal_gi_forethought_connective` grammar rule.
    rule "forethought connective" modal_gi_forethought_connective(tense_modal) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Bo` cmavo marker.
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Syntax model for forethought connective parsed by the `gik_connective` grammar rule.
    rule "forethought connective" gik_connective -> struct {
        #[tree_child(primary)]
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for forethought connective parsed by the `zantufa_extra_gik_connective` grammar rule.
    rule "forethought connective" zantufa_extra_gik_connective -> struct {
        #[tree_child(primary)]
        /// The `Gi` cmavo marker.
        field gi <- cmavo(Gi).warn(ExperimentalZantufaNaryForethought).wf();
    }

    /// Syntax model for tag parsed by the `tense_modal` grammar rule.
    rule "tag" tense_modal(selbri) -> struct {
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
            pa_word(),
        ));
        #[tree_child(primary)]
        /// The body component of this syntax node.
        field body <- tense_modal_body(selbri);
    }

    /// Syntax model for tag parsed by the `tense_modal_body` grammar rule.
    rule "tag" tense_modal_body(selbri) -> enum {
        /// The `connected_tense_modal` alternative of tag.
        connected_tense_modal,
        /// The `tense_modal_atom` alternative of tag.
        tense_modal_atom,
    }

    /// Syntax model for connected tag parsed by the `connected_tense_modal` grammar rule.
    rule "connected tag" connected_tense_modal(selbri) -> struct {
        /// The shared first child syntax node.
        field first <- arc(tense_modal_atom(selbri));
        /// Non-empty ordered sequence of continuations components.
        field continuations <- [one_or_more connected_tense_modal_continuation(selbri)];
    }

    /// Syntax model for connected tag continuation parsed by the `connected_tense_modal_continuation` grammar rule.
    rule "connected tag continuation" connected_tense_modal_continuation(selbri) -> struct {
        /// The connective component of this syntax node.
        field connective <- tense_modal_connective;
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal_atom(selbri));
    }

    /// Syntax model for tag connective parsed by the `tense_modal_connective` grammar rule.
    rule "tag connective" tense_modal_connective -> enum {
        /// The `joik_connective` alternative of tag connective.
        joik_connective,
        /// The `jek_connective` alternative of tag connective.
        jek_connective,
    }

    /// Syntax model for tag parsed by the `tense_modal_atom` grammar rule.
    rule "tag" tense_modal_atom(selbri) -> enum {
        /// The `composite_tense` alternative of tag.
        composite_tense,
        /// The `fiho_tense` alternative of tag.
        fiho_tense,
        /// The `modal_tense` alternative of tag.
        modal_tense,
        /// The `nahe_se_flat_prefixed_tense` alternative of tag.
        nahe_se_flat_prefixed_tense,
        /// The `se_flat_prefixed_tense` alternative of tag.
        se_flat_prefixed_tense,
        /// The `fa_flat_tag_tense` alternative of tag.
        fa_flat_tag_tense,
        /// The `zantufa_recursive_tag_tense` alternative of tag.
        when feature(ZantufaTags) zantufa_recursive_tag_tense,
        /// The `sticky_tense` alternative of tag.
        sticky_tense,
    }

    /// Syntax model for FIhO modal parsed by the `fiho_tense` grammar rule.
    rule "FIhO modal" fiho_tense(selbri) -> struct {
        /// The `Fiho` cmavo marker.
        field fiho <- cmavo(Fiho).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
        /// The optional `Fehu` cmavo marker.
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Syntax model for tag parsed by the `fa_flat_tag_tense` grammar rule.
    rule "tag" fa_flat_tag_tense -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    /// Syntax model for tag parsed by the `flat_tag_atom` grammar rule.
    rule "tag" flat_tag_atom -> enum {
        /// The `fa_flat_tag_atom` alternative of tag.
        fa_flat_tag_atom,
        /// The `modal_flat_tag_atom` alternative of tag.
        modal_flat_tag_atom,
        /// The `composite_flat_tag_atom` alternative of tag.
        composite_flat_tag_atom,
    }

    /// Syntax model for tag parsed by the `fa_flat_tag_atom` grammar rule.
    rule "tag" fa_flat_tag_atom -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    /// Syntax model for modal tag parsed by the `modal_flat_tag_atom` grammar rule.
    rule "modal tag" modal_flat_tag_atom -> struct {
        /// The shared modal child syntax node.
        field modal <- arc(modal_tense());
    }

    /// Syntax model for tag parsed by the `composite_flat_tag_atom` grammar rule.
    rule "tag" composite_flat_tag_atom -> struct {
        /// The shared composite child syntax node.
        field composite <- arc(composite_tense());
    }

    /// Syntax model for tag parsed by the `nahe_se_flat_prefixed_tense` grammar rule.
    rule "tag" nahe_se_flat_prefixed_tense -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).warn(ExperimentalFlattenedTag).wf();
        /// The optional se component.
        field se <- opt(selmaho(Se).wf());
        /// The atom component of this syntax node.
        field atom <- flat_tag_atom();
    }

    /// Syntax model for tag parsed by the `se_flat_prefixed_tense` grammar rule.
    rule "tag" se_flat_prefixed_tense -> struct {
        /// A word from selmaho `Se`.
        field se <- selmaho(Se).warn(ExperimentalFlattenedTag).wf();
        /// The atom component of this syntax node.
        field atom <- flat_tag_atom();
    }

    /// Syntax model for tag parsed by the `zantufa_recursive_tag_tense` grammar rule.
    rule "tag" zantufa_recursive_tag_tense -> struct {
        /// The first prefix component of this syntax node.
        field first_prefix <- choice((
            selmaho(Nahe),
            selmaho(Se),
        )).warn(ExperimentalZantufaRecursiveTag).wf();
        /// Ordered sequence of zero or more additional prefixes components.
        field additional_prefixes <- [zero_or_more choice((
            selmaho(Nahe),
            selmaho(Se),
        )).wf()];
        /// The atom component of this syntax node.
        field atom <- choice((
            selmaho(Fa).warn(ExperimentalFaAsTag),
            selmaho(Pu),
            selmaho(Zi),
            selmaho(Zeha),
            selmaho(Va),
            selmaho(Faha),
            selmaho(Veha),
            selmaho(Viha),
            selmaho(Caha),
            selmaho(Zaho),
            selmaho(Cuhe),
            cmavo(Ki),
        )).wf();
    }

    /// Syntax model for tag parsed by the `composite_tense` grammar rule.
    rule "tag" composite_tense -> enum {
        /// The `prefixed_time_space_caha_tense` alternative of tag.
        prefixed_time_space_caha_tense,
        /// The `time_space_caha_ki_tense` alternative of tag.
        time_space_caha_ki_tense,
        /// The `cuhe_tense` alternative of tag.
        cuhe_tense,
    }

    /// Syntax model for tag parsed by the `prefixed_time_space_caha_tense` grammar rule.
    rule "tag" prefixed_time_space_caha_tense -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared tense child syntax node.
        field tense <- arc(time_space_caha_tense);
        /// The optional ki component.
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Syntax model for tag parsed by the `time_space_caha_ki_tense` grammar rule.
    rule "tag" time_space_caha_ki_tense -> struct {
        /// The shared tense child syntax node.
        field tense <- arc(time_space_caha_tense);
        /// The optional ki component.
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Syntax model for tag parsed by the `time_space_caha_tense` grammar rule.
    rule "tag" time_space_caha_tense -> enum {
        /// The `time_then_space_caha_tense` alternative of tag.
        time_then_space_caha_tense,
        /// The `space_then_time_caha_tense` alternative of tag.
        space_then_time_caha_tense,
        /// The `caha_tense` alternative of tag.
        caha_tense,
    }

    /// Syntax model for time tense parsed by the `time_then_space_caha_tense` grammar rule.
    rule "time tense" time_then_space_caha_tense -> struct {
        /// The shared time child syntax node.
        field time <- arc(time_tense);
        /// The optional space component.
        field space <- opt(arc(space_tense));
        /// The optional caha component.
        field caha <- opt(arc(caha_tense()));
    }

    /// Syntax model for space tense parsed by the `space_then_time_caha_tense` grammar rule.
    rule "space tense" space_then_time_caha_tense -> struct {
        /// The shared space child syntax node.
        field space <- arc(space_tense);
        /// The optional time component.
        field time <- opt(arc(time_tense));
        /// The optional caha component.
        field caha <- opt(arc(caha_tense()));
    }

    /// Syntax model for time tense parsed by the `time_tense` grammar rule.
    rule "time tense" time_tense -> enum {
        /// The `time_tense_with_zi` alternative of time tense.
        time_tense_with_zi,
        /// The `time_tense_with_offset` alternative of time tense.
        time_tense_with_offset,
        /// The `time_tense_with_interval` alternative of time tense.
        time_tense_with_interval,
        /// The `time_tense_with_properties` alternative of time tense.
        time_tense_with_properties,
    }

    /// Syntax model for time tense parsed by the `time_tense_with_zi` grammar rule.
    rule "time tense" time_tense_with_zi -> struct {
        /// The shared zi child syntax node.
        field zi <- arc(zi_time_distance_tense());
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Syntax model for time tense parsed by the `time_tense_with_offset` grammar rule.
    rule "time tense" time_tense_with_offset -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Non-empty ordered sequence of offsets components.
        field offsets <- [one_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Syntax model for time tense parsed by the `time_tense_with_interval` grammar rule.
    rule "time tense" time_tense_with_interval -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The shared zeha child syntax node.
        field zeha <- arc(zeha_time_interval_tense());
        /// Ordered sequence of zero or more properties components.
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Syntax model for time tense parsed by the `time_tense_with_properties` grammar rule.
    rule "time tense" time_tense_with_properties -> struct {
        /// The optional zi component.
        field zi <- opt(arc(zi_time_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        /// The optional zeha component.
        field zeha <- opt(arc(zeha_time_interval_tense()));
        /// Non-empty ordered sequence of properties components.
        field properties <- [one_or_more arc(interval_property_tense)];
    }

    /// Syntax model for interval property parsed by the `interval_property_tense` grammar rule.
    rule "interval property" interval_property_tense -> enum {
        /// The `numbered_interval_property_tense` alternative of interval property.
        numbered_interval_property_tense,
        /// The `tahe_interval_property_tense` alternative of interval property.
        tahe_interval_property_tense,
        /// The `zaho_interval_property_tense` alternative of interval property.
        zaho_interval_property_tense,
    }

    /// Syntax model for interval property parsed by the `numbered_interval_property_tense` grammar rule.
    rule "interval property" numbered_interval_property_tense -> struct {
        /// The number component of this syntax node.
        field number <- interval_property_number_words().wf();
        /// A word from selmaho `Roi`.
        field roi <- selmaho(Roi).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for number parsed by the `interval_property_number_words` grammar rule.
    rule "number" interval_property_number_words -> struct {
        /// The first number component of this syntax node.
        field first_number <- pa_word();
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more interval_property_number_word_continuation];
    }

    /// Syntax model for number continuation parsed by the `interval_property_number_word_continuation` grammar rule.
    rule "number continuation" interval_property_number_word_continuation -> enum {
        /// The `interval_property_number_pa_continuation` alternative of number continuation.
        interval_property_number_pa_continuation,
        /// The `interval_property_number_letter_continuation` alternative of number continuation.
        interval_property_number_letter_continuation,
    }

    /// Syntax model for number continuation parsed by the `interval_property_number_pa_continuation` grammar rule.
    rule "number continuation" interval_property_number_pa_continuation -> struct {
        /// The pa component of this syntax node.
        field pa <- pa_word();
    }

    /// Syntax model for number continuation parsed by the `interval_property_number_letter_continuation` grammar rule.
    rule "number continuation" interval_property_number_letter_continuation -> struct {
        /// The letter component of this syntax node.
        field letter <- word_category(LetterWord);
    }

    /// Syntax model for interval property parsed by the `tahe_interval_property_tense` grammar rule.
    rule "interval property" tahe_interval_property_tense -> struct {
        /// A word from selmaho `Tahe`.
        field tahe <- selmaho(Tahe).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for interval property parsed by the `zaho_interval_property_tense` grammar rule.
    rule "interval property" zaho_interval_property_tense -> struct {
        /// A word from selmaho `Zaho`.
        field zaho <- selmaho(Zaho).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for time tense parsed by the `pu_time_offset_tense` grammar rule.
    rule "time tense" pu_time_offset_tense -> struct {
        /// A word from selmaho `Pu`.
        field pu <- selmaho(Pu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional distance component.
        field distance <- opt(selmaho(Zi).wf());
    }

    /// Syntax model for time tense parsed by the `zi_time_distance_tense` grammar rule.
    rule "time tense" zi_time_distance_tense -> struct {
        /// A word from selmaho `Zi`.
        field zi <- selmaho(Zi).wf();
    }

    /// Syntax model for time interval parsed by the `zeha_time_interval_tense` grammar rule.
    rule "time interval" zeha_time_interval_tense -> struct {
        /// A word from selmaho `Zeha`.
        field zeha <- selmaho(Zeha).wf();
        /// The optional `Nai` cmavo marker.
        field direction <- opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
    }

    /// Syntax model for space tense parsed by the `space_tense` grammar rule.
    rule "space tense" space_tense -> enum {
        /// The `space_tense_with_va` alternative of space tense.
        space_tense_with_va,
        /// The `space_tense_with_offset` alternative of space tense.
        space_tense_with_offset,
        /// The `space_tense_with_interval` alternative of space tense.
        space_tense_with_interval,
        /// The `space_tense_with_mohi` alternative of space tense.
        space_tense_with_mohi,
    }

    /// Syntax model for space tense parsed by the `space_tense_with_va` grammar rule.
    rule "space tense" space_tense_with_va -> struct {
        /// The shared va child syntax node.
        field va <- arc(va_space_distance_tense());
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense));
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Syntax model for space tense parsed by the `space_tense_with_offset` grammar rule.
    rule "space tense" space_tense_with_offset -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Non-empty ordered sequence of offsets components.
        field offsets <- [one_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense));
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Syntax model for space tense parsed by the `space_tense_with_interval` grammar rule.
    rule "space tense" space_tense_with_interval -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The shared interval child syntax node.
        field interval <- arc(space_interval_tense);
        /// The optional mohi component.
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Syntax model for space tense parsed by the `space_tense_with_mohi` grammar rule.
    rule "space tense" space_tense_with_mohi -> struct {
        /// The optional va component.
        field va <- opt(arc(va_space_distance_tense()));
        /// Ordered sequence of zero or more offsets components.
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        /// The optional interval component.
        field interval <- opt(arc(space_interval_tense));
        /// The shared mohi child syntax node.
        field mohi <- arc(mohi_space_offset_tense());
    }

    /// Syntax model for space tense parsed by the `va_space_distance_tense` grammar rule.
    rule "space tense" va_space_distance_tense -> struct {
        /// A word from selmaho `Va`.
        field va <- selmaho(Va).wf();
    }

    /// Syntax model for space tense parsed by the `faha_space_offset_tense` grammar rule.
    rule "space tense" faha_space_offset_tense -> struct {
        /// A word from selmaho `Faha`.
        field faha <- selmaho(Faha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
        /// The optional distance component.
        field distance <- opt(selmaho(Va).wf());
    }

    /// Syntax model for space interval parsed by the `faha_interval_direction_tense` grammar rule.
    rule "space interval" faha_interval_direction_tense -> struct {
        /// A word from selmaho `Faha`.
        field faha <- selmaho(Faha).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for space interval parsed by the `space_interval_tense` grammar rule.
    rule "space interval" space_interval_tense -> enum {
        /// The `space_interval_with_extent_tense` alternative of space interval.
        space_interval_with_extent_tense,
        /// The `space_interval_properties_tense` alternative of space interval.
        space_interval_properties_tense,
    }

    /// Syntax model for space interval parsed by the `space_interval_with_extent_tense` grammar rule.
    rule "space interval" space_interval_with_extent_tense -> struct {
        /// The shared extent child syntax node.
        field extent <- arc(space_interval_extent_tense);
        /// The optional direction component.
        field direction <- opt(arc(faha_interval_direction_tense()));
        /// The optional properties component.
        field properties <- opt(arc(space_interval_properties_tense()));
    }

    /// Syntax model for space interval parsed by the `space_interval_extent_tense` grammar rule.
    rule "space interval" space_interval_extent_tense -> enum {
        /// The `veha_space_interval_tense` alternative of space interval.
        veha_space_interval_tense,
        /// The `viha_space_interval_tense` alternative of space interval.
        viha_space_interval_tense,
    }

    /// Syntax model for space interval parsed by the `space_interval_properties_tense` grammar rule.
    rule "space interval" space_interval_properties_tense -> struct {
        /// The shared first child syntax node.
        field first <- arc(fehe_interval_property_tense());
        /// Ordered sequence of zero or more additional components.
        field additional <- [zero_or_more arc(fehe_interval_property_tense())];
    }

    /// Syntax model for space interval parsed by the `veha_space_interval_tense` grammar rule.
    rule "space interval" veha_space_interval_tense -> struct {
        /// A word from selmaho `Veha`.
        field veha <- selmaho(Veha).wf();
        /// The optional viha component.
        field viha <- opt(selmaho(Viha).wf());
    }

    /// Syntax model for space interval parsed by the `viha_space_interval_tense` grammar rule.
    rule "space interval" viha_space_interval_tense -> struct {
        /// A word from selmaho `Viha`.
        field viha <- selmaho(Viha).wf();
    }

    /// Syntax model for space interval property parsed by the `fehe_interval_property_tense` grammar rule.
    rule "space interval property" fehe_interval_property_tense -> struct {
        /// The `Fehe` cmavo marker.
        field fehe <- cmavo(Fehe).wf();
        /// The shared property child syntax node.
        field property <- arc(interval_property_tense);
    }

    /// Syntax model for space tense parsed by the `mohi_space_offset_tense` grammar rule.
    rule "space tense" mohi_space_offset_tense -> struct {
        /// A word from selmaho `Mohi`.
        field mohi <- selmaho(Mohi).wf();
        /// The shared offset child syntax node.
        field offset <- arc(faha_space_offset_tense());
    }

    /// Syntax model for tag parsed by the `caha_tense` grammar rule.
    rule "tag" caha_tense -> struct {
        /// A word from selmaho `Caha`.
        field caha <- selmaho(Caha).wf();
    }

    /// Syntax model for tag parsed by the `ki_composite_tense` grammar rule.
    rule "tag" ki_composite_tense -> struct {
        /// The `Ki` cmavo marker.
        field ki <- cmavo(Ki).wf();
    }

    /// Syntax model for tag parsed by the `cuhe_tense` grammar rule.
    rule "tag" cuhe_tense -> struct {
        /// A word from selmaho `Cuhe`.
        field cuhe <- selmaho(Cuhe).wf();
    }

    /// Syntax model for modal tag parsed by the `modal_tense` grammar rule.
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

    /// Syntax model for tag parsed by the `sticky_tense` grammar rule.
    rule "tag" sticky_tense -> struct {
        /// The `Ki` cmavo marker.
        field ki <- cmavo(Ki).wf();
    }

    /// Syntax model for selbri parsed by the `selbri` grammar rule.
    rule "selbri" selbri(selbri, co_selbri, tense_modal, statement) -> enum {
        /// The `tagged_selbri` alternative of selbri.
        tagged_selbri,
        /// The `untagged_selbri` alternative of selbri.
        untagged_selbri,
    }

    /// Syntax model for selbri parsed by the `untagged_selbri` grammar rule.
    rule "selbri" untagged_selbri(selbri, co_selbri, statement) -> enum {
        /// The `negated_selbri` alternative of selbri.
        negated_selbri,
        /// The `co_selbri` alternative of selbri.
        co_selbri,
        /// The `forethought_selbri_connection` alternative of selbri.
        forethought_selbri_connection,
    }

    /// Syntax model for tagged selbri parsed by the `tagged_selbri` grammar rule.
    rule "tagged selbri" tagged_selbri(selbri, co_selbri, tense_modal, statement) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(untagged_selbri(selbri, co_selbri, statement));
    }

    /// Syntax model for negated selbri parsed by the `negated_selbri` grammar rule.
    rule "negated selbri" negated_selbri(selbri) -> struct {
        /// A word from selmaho `Na`.
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(selbri);
    }

    /// Syntax model for selbri parsed by the `co_selbri` grammar rule.
    rule "selbri" co_selbri(co_selbri, tanru_unit, statement) -> struct {
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(connected_selbri(tanru_unit, statement));
        /// The optional co tail component.
        field co_tail <- opt(co_selbri_tail(co_selbri));
    }

    /// Syntax model for selbri parsed by the `co_selbri_tail` grammar rule.
    rule "selbri" co_selbri_tail(co_selbri) -> struct {
        /// The `Co` cmavo marker.
        field co <- cmavo(Co).wf();
        /// The shared trailing selbri child syntax node.
        field trailing_selbri <- arc(co_selbri);
    }

    /// Syntax model for forethought selbri connection parsed by the `forethought_selbri_connection` grammar rule.
    rule "forethought selbri connection" forethought_selbri_connection(selbri) -> struct {
        /// The guhek component of this syntax node.
        field guhek <- guhek_connective;
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(selbri);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_selbri_branch(selbri);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_branch(selbri)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Syntax model for forethought selbri connection parsed by the `forethought_selbri_branch` grammar rule.
    rule "forethought selbri connection" forethought_selbri_branch(selbri) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
    }

    /// Syntax model for forethought selbri connection parsed by the `zantufa_forethought_selbri_branch` grammar rule.
    rule "forethought selbri connection" zantufa_forethought_selbri_branch(selbri) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// The shared selbri child syntax node.
        field selbri <- arc(selbri);
    }

    /// Syntax model for selbri connection parsed by the `connected_selbri` grammar rule.
    rule "selbri connection" connected_selbri(tanru_unit, statement) -> struct {
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(tanru_selbri(tanru_unit, statement));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more connected_selbri_continuation(tanru_unit, statement)];
    }

    /// Syntax model for selbri connection continuation parsed by the `connected_selbri_continuation` grammar rule.
    rule "selbri connection continuation" connected_selbri_continuation(tanru_unit, statement) -> struct {
        /// The connective component of this syntax node.
        field connective <- relation_afterthought_connective;
        /// The shared trailing selbri child syntax node.
        field trailing_selbri <- arc(tanru_selbri(tanru_unit, statement));
    }

    /// Syntax model for tanru parsed by the `tanru_selbri` grammar rule.
    rule "tanru" tanru_selbri(tanru_unit, statement) -> struct {
        /// The first unit component of this syntax node.
        field first_unit <- tanru_unit;
        /// Ordered sequence of zero or more additional units components.
        field additional_units <- [zero_or_more tanru_unit];
    }

    /// Syntax model for tanru unit parsed by the `tanru_unit` grammar rule.
    rule "tanru unit" tanru_unit(bo_or_linked_tanru_unit, statement) -> struct {
        /// The units component of this syntax node.
        field units <- chain(
            first: arc(bo_or_linked_tanru_unit),
            zero_or_more: tanru_unit_continuation(bo_or_linked_tanru_unit, statement),
            element: trailing_unit,
        );
    }

    /// Syntax model for tanru unit continuation parsed by the `tanru_unit_continuation` grammar rule.
    rule "tanru unit continuation" tanru_unit_continuation(bo_or_linked_tanru_unit, statement) -> struct {
        /// The connective component of this syntax node.
        field connective <- relation_afterthought_connective;
        /// The shared trailing unit child syntax node.
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Syntax model for tanru unit parsed by the `bo_or_linked_tanru_unit` grammar rule.
    rule "tanru unit" bo_or_linked_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        /// The `forethought_selbri_group_tanru_unit` alternative of tanru unit.
        forethought_selbri_group_tanru_unit,
        /// The `bound_tanru_unit` alternative of tanru unit.
        bound_tanru_unit,
        /// The `assigned_pro_bridi_tanru_unit` alternative of tanru unit.
        assigned_pro_bridi_tanru_unit,
        /// The `linked_tanru_unit` alternative of tanru unit.
        linked_tanru_unit,
    }

    /// Syntax model for forethought selbri connection parsed by the `forethought_selbri_group_tanru_unit` grammar rule.
    rule "forethought selbri connection" forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri, statement) -> struct {
        /// The guhek component of this syntax node.
        field guhek <- guhek_connective;
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(selbri);
        /// The first branch component of this syntax node.
        field first_branch <- forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement);
        /// Ordered sequence of zero or more additional branches components.
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement)];
        /// The optional gihi component.
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Syntax model for forethought selbri connection parsed by the `forethought_selbri_group_branch` grammar rule.
    rule "forethought selbri connection" forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        /// The gik component of this syntax node.
        field gik <- gik_connective;
        /// The shared unit child syntax node.
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Syntax model for forethought selbri connection parsed by the `zantufa_forethought_selbri_group_branch` grammar rule.
    rule "forethought selbri connection" zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        assert feature(ZantufaConnectives);
        /// The gik component of this syntax node.
        field gik <- zantufa_extra_gik_connective;
        /// The shared unit child syntax node.
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Syntax model for BO-grouped tanru unit parsed by the `bound_tanru_unit` grammar rule.
    rule "BO-grouped tanru unit" bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        /// The shared leading unit child syntax node.
        field leading_unit <- arc(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement));
        /// The optional bo connective component.
        field bo_connective <- opt(arc(relation_afterthought_connective));
        /// The optional bo tense modal component.
        field bo_tense_modal <- opt(arc(tense_modal));
        /// The `Bo` cmavo marker.
        field bo <- cmavo(Bo).wf();
        /// The shared trailing unit child syntax node.
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Syntax model for pro-bridi assignment parsed by the `assigned_pro_bridi_tanru_unit` grammar rule.
    rule "pro-bridi assignment" assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        /// The shared base child syntax node.
        field base <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        /// Non-empty ordered sequence of assignments components.
        field assignments <- [one_or_more pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement)];
    }

    /// Syntax model for pro-bridi assignment parsed by the `pro_bridi_tanru_unit_assignment` grammar rule.
    rule "pro-bridi assignment" pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        /// The `Cei` cmavo marker.
        field cei <- cmavo(Cei).wf();
        /// The shared tanru unit child syntax node.
        field tanru_unit <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Syntax model for tanru unit parsed by the `linked_tanru_unit` grammar rule.
    rule "tanru unit" linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom);
        /// The optional linkargs component.
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    /// Syntax model for tanru unit parsed by the `linked_tanru_unit_for_cei` grammar rule.
    rule "tanru unit" linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        /// The optional linkargs component.
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    /// Syntax model for tanru unit parsed by the `tanru_unit_atom_for_cei` grammar rule.
    rule "tanru unit" tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        /// Ordered sequence of zero or more conversions components.
        field conversions <- [zero_or_more selmaho(Se).wf()];
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Syntax model for tanru unit parsed by the `tanru_unit_atom_base_for_cei` grammar rule.
    rule "tanru unit" tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        /// The `pro_bridi_tanru_unit` alternative of tanru unit.
        pro_bridi_tanru_unit,
        /// The `ordinal_tanru_unit` alternative of tanru unit.
        ordinal_tanru_unit,
        /// The `word_tanru_unit` alternative of tanru unit.
        word_tanru_unit,
        /// The `preposed_linkargs_tanru_unit` alternative of tanru unit.
        preposed_linkargs_tanru_unit,
        /// The `jai_modal_tanru_unit` alternative of tanru unit.
        jai_modal_tanru_unit,
        /// The `scalar_negated_tanru_unit` alternative of tanru unit.
        scalar_negated_tanru_unit,
        /// The `zantufa_statement_abstraction_tanru_unit` alternative of tanru unit.
        when feature(ZantufaTerms) zantufa_statement_abstraction_tanru_unit,
        /// The `abstraction_tanru_unit` alternative of tanru unit.
        abstraction_tanru_unit,
        /// The `sumti_selbri_tanru_unit` alternative of tanru unit.
        sumti_selbri_tanru_unit,
        /// The `zantufa_me_tanru_unit` alternative of tanru unit.
        zantufa_me_tanru_unit,
        /// The `zantufa_mex_moi_tanru_unit` alternative of tanru unit.
        zantufa_mex_moi_tanru_unit,
        /// The `operator_selbri_tanru_unit` alternative of tanru unit.
        operator_selbri_tanru_unit,
        /// The `quoted_bridi_selbri_tanru_unit` alternative of tanru unit.
        quoted_bridi_selbri_tanru_unit,
        /// The `quoted_text_selbri_tanru_unit` alternative of tanru unit.
        quoted_text_selbri_tanru_unit,
        /// The `text_selbri_tanru_unit` alternative of tanru unit.
        text_selbri_tanru_unit,
        /// The `tag_selbri_tanru_unit` alternative of tanru unit.
        tag_selbri_tanru_unit,
        /// The `goha_word_tanru_unit` alternative of tanru unit.
        goha_word_tanru_unit,
        /// The `grouped_tanru_unit` alternative of tanru unit.
        grouped_tanru_unit,
    }

    /// Syntax model for tanru unit parsed by the `tanru_unit_atom` grammar rule.
    rule "tanru unit" tanru_unit_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        /// Ordered sequence of zero or more conversions components.
        field conversions <- [zero_or_more selmaho(Se).wf()];
        /// The shared base child syntax node.
        field base <- arc(tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Syntax model for tanru unit parsed by the `tanru_unit_atom_base` grammar rule.
    rule "tanru unit" tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        /// The `ordinal_tanru_unit` alternative of tanru unit.
        ordinal_tanru_unit,
        /// The `word_tanru_unit` alternative of tanru unit.
        word_tanru_unit,
        /// The `preposed_linkargs_tanru_unit` alternative of tanru unit.
        preposed_linkargs_tanru_unit,
        /// The `jai_modal_tanru_unit` alternative of tanru unit.
        jai_modal_tanru_unit,
        /// The `scalar_negated_tanru_unit` alternative of tanru unit.
        scalar_negated_tanru_unit,
        /// The `zantufa_statement_abstraction_tanru_unit` alternative of tanru unit.
        when feature(ZantufaTerms) zantufa_statement_abstraction_tanru_unit,
        /// The `abstraction_tanru_unit` alternative of tanru unit.
        abstraction_tanru_unit,
        /// The `sumti_selbri_tanru_unit` alternative of tanru unit.
        sumti_selbri_tanru_unit,
        /// The `zantufa_me_tanru_unit` alternative of tanru unit.
        zantufa_me_tanru_unit,
        /// The `zantufa_mex_moi_tanru_unit` alternative of tanru unit.
        zantufa_mex_moi_tanru_unit,
        /// The `operator_selbri_tanru_unit` alternative of tanru unit.
        operator_selbri_tanru_unit,
        /// The `quoted_bridi_selbri_tanru_unit` alternative of tanru unit.
        quoted_bridi_selbri_tanru_unit,
        /// The `quoted_text_selbri_tanru_unit` alternative of tanru unit.
        quoted_text_selbri_tanru_unit,
        /// The `text_selbri_tanru_unit` alternative of tanru unit.
        text_selbri_tanru_unit,
        /// The `tag_selbri_tanru_unit` alternative of tanru unit.
        tag_selbri_tanru_unit,
        /// The `goha_word_tanru_unit` alternative of tanru unit.
        goha_word_tanru_unit,
        /// The `pro_bridi_tanru_unit` alternative of tanru unit.
        pro_bridi_tanru_unit,
        /// The `grouped_tanru_unit` alternative of tanru unit.
        grouped_tanru_unit,
    }

    /// Syntax model for tagged selbri parsed by the `tagged_selbri_group_tanru_unit` grammar rule.
    rule "tagged selbri" tagged_selbri_group_tanru_unit(tanru_unit, tense_modal, statement) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared inner selbri child syntax node.
        field inner_selbri <- arc(connected_selbri(tanru_unit, statement));
    }

    /// Syntax model for linked arguments parsed by the `preposed_linkargs_tanru_unit` grammar rule.
    rule "linked arguments" preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal, statement) -> struct {
        /// The linkargs component of this syntax node.
        field linkargs <- linkargs(sumti, tense_modal);
        /// The shared base child syntax node.
        field base <- arc(tanru_unit);
    }

    /// Syntax model for scalar-negated tanru unit parsed by the `scalar_negated_tanru_unit` grammar rule.
    rule "scalar-negated tanru unit" scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement));
    }

    /// Syntax model for scalar-negated tanru unit parsed by the `scalar_negated_tanru_inner_unit` grammar rule.
    rule "scalar-negated tanru unit" scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> enum {
        /// The `tagged_selbri_group_tanru_unit` alternative of scalar-negated tanru unit.
        tagged_selbri_group_tanru_unit,
        /// The `pro_bridi_tanru_unit` alternative of scalar-negated tanru unit.
        pro_bridi_tanru_unit,
        /// The `tanru_unit_atom` alternative of scalar-negated tanru unit.
        tanru_unit_atom,
    }

    /// Syntax model for modal conversion parsed by the `jai_modal_tanru_unit` grammar rule.
    rule "modal conversion" jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal) -> struct {
        /// The `Jai` cmavo marker.
        field jai <- cmavo(Jai).wf();
        /// The optional tense modal component.
        field tense_modal <- opt(arc(tense_modal));
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Syntax model for modal conversion parsed by the `jai_inner_tanru_unit` grammar rule.
    rule "modal conversion" jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, letter_tokens, letter_string) -> enum {
        /// The `converted_jai_inner_tanru_unit` alternative of modal conversion.
        converted_jai_inner_tanru_unit,
        /// The `scalar_negated_jai_inner_tanru_unit` alternative of modal conversion.
        scalar_negated_jai_inner_tanru_unit,
        /// The `sumti_selbri_tanru_unit` alternative of modal conversion.
        sumti_selbri_tanru_unit,
        /// The `quoted_bridi_selbri_tanru_unit` alternative of modal conversion.
        quoted_bridi_selbri_tanru_unit,
        /// The `quoted_text_selbri_tanru_unit` alternative of modal conversion.
        quoted_text_selbri_tanru_unit,
        /// The `text_selbri_tanru_unit` alternative of modal conversion.
        text_selbri_tanru_unit,
        /// The `grouped_jai_inner_tanru_unit` alternative of modal conversion.
        grouped_jai_inner_tanru_unit,
        /// The `ordinal_tanru_unit` alternative of modal conversion.
        ordinal_tanru_unit,
        /// The `operator_selbri_tanru_unit` alternative of modal conversion.
        operator_selbri_tanru_unit,
        /// The `pro_bridi_tanru_unit` alternative of modal conversion.
        pro_bridi_tanru_unit,
        /// The `word_tanru_unit` alternative of modal conversion.
        word_tanru_unit,
    }

    /// Syntax model for converted tanru unit parsed by the `converted_jai_inner_tanru_unit` grammar rule.
    rule "converted tanru unit" converted_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// A word from selmaho `Se`.
        field se <- selmaho(Se).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Syntax model for scalar-negated tanru unit parsed by the `scalar_negated_jai_inner_tanru_unit` grammar rule.
    rule "scalar-negated tanru unit" scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// A word from selmaho `Nahe`.
        field nahe <- selmaho(Nahe).wf();
        /// The shared inner unit child syntax node.
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Syntax model for quoted bridi selbri parsed by the `quoted_bridi_selbri_tanru_unit` grammar rule.
    rule "quoted bridi selbri" quoted_bridi_selbri_tanru_unit -> struct {
        /// The quote component of this syntax node.
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Syntax model for text selbri parsed by the `text_selbri_tanru_unit` grammar rule.
    rule "text selbri" text_selbri_tanru_unit(text) -> struct {
        /// The `Luhei` cmavo marker.
        field luhei <- cmavo(Luhei).warn(ExperimentalZantufaLuheiSelbriUnit).wf();
        /// The shared text child syntax node.
        field text <- arc(text);
        /// The optional `Lihau` cmavo marker.
        field lihau <- opt(cmavo(Lihau).wf()).elidable_terminator(Lihau);
    }

    /// Syntax model for quoted text selbri parsed by the `quoted_text_selbri_tanru_unit` grammar rule.
    rule "quoted text selbri" quoted_text_selbri_tanru_unit -> struct {
        /// The muhoi component of this syntax node.
        field muhoi <- delimited_quote_marker(Muhoi).warn(ExperimentalZantufaMuhoiSelbriUnit).wf();
    }

    /// Syntax model for tag selbri parsed by the `tag_selbri_tanru_unit` grammar rule.
    rule "tag selbri" tag_selbri_tanru_unit(tense_modal) -> struct {
        /// The `Xohi` cmavo marker.
        field xohi <- cmavo(Xohi).warn(ExperimentalXohiTagSelbri).wf();
        /// The shared tag child syntax node.
        field tag <- arc(tense_modal);
    }

    /// Syntax model for ordinal selbri parsed by the `ordinal_tanru_unit` grammar rule.
    rule "ordinal selbri" ordinal_tanru_unit(letter_tokens, letter_string) -> struct {
        /// The number component of this syntax node.
        field number <- number_or_letter_words(letter_tokens, letter_string);
        /// A word from selmaho `Moi`.
        field moi <- selmaho(Moi).wf();
    }

    /// Syntax model for tanru unit parsed by the `word_tanru_unit` grammar rule.
    rule "tanru unit" word_tanru_unit -> struct {
        /// The word component of this syntax node.
        field word <- tanru_unit_relation_word().wf();
    }

    /// Syntax model for tanru unit parsed by the `goha_word_tanru_unit` grammar rule.
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

    /// Syntax model for pro-bridi parsed by the `pro_bridi_tanru_unit` grammar rule.
    rule "pro-bridi" pro_bridi_tanru_unit -> struct {
        /// A word from selmaho `Goha`.
        field goha <- selmaho(Goha).wf();
        /// The optional `Raho` cmavo marker.
        field raho <- opt(cmavo(Raho).wf());
    }

    /// Syntax model for sumti-to-selbri parsed by the `sumti_selbri_tanru_unit` grammar rule.
    rule "sumti-to-selbri" sumti_selbri_tanru_unit(sumti, letter_string) -> struct {
        /// The `Me` cmavo marker.
        field me <- cmavo(Me).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti_selbri_sumti(sumti, letter_string));
        /// The optional `Mehu` cmavo marker.
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        /// The optional moi marker component.
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    /// Syntax model for sumti-to-selbri parsed by the `zantufa_me_tanru_unit` grammar rule.
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

    /// Syntax model for sumti-to-selbri parsed by the `zantufa_me_selbri_body` grammar rule.
    rule "sumti-to-selbri" zantufa_me_selbri_body(mekso, mekso_operator, tense_modal) -> enum {
        /// The `zantufa_me_operator_selbri_body` alternative of sumti-to-selbri.
        zantufa_me_operator_selbri_body,
        /// The `zantufa_me_mekso_selbri_body` alternative of sumti-to-selbri.
        zantufa_me_mekso_selbri_body,
        /// The `zantufa_me_tag_selbri_body` alternative of sumti-to-selbri.
        zantufa_me_tag_selbri_body,
    }

    /// Syntax model for sumti-to-selbri parsed by the `zantufa_me_operator_selbri_body` grammar rule.
    rule "sumti-to-selbri" zantufa_me_operator_selbri_body(mekso_operator) -> struct {
        /// Non-empty ordered sequence of operators components.
        field operators <- [one_or_more mekso_operator];
    }

    /// Syntax model for sumti-to-selbri parsed by the `zantufa_me_mekso_selbri_body` grammar rule.
    rule "sumti-to-selbri" zantufa_me_mekso_selbri_body(mekso) -> struct {
        /// The shared expression child syntax node.
        field expression <- arc(mekso);
    }

    /// Syntax model for sumti-to-selbri parsed by the `zantufa_me_tag_selbri_body` grammar rule.
    rule "sumti-to-selbri" zantufa_me_tag_selbri_body(tense_modal) -> struct {
        /// The shared tag child syntax node.
        field tag <- arc(tense_modal);
    }

    /// Syntax model for mex selbri parsed by the `zantufa_mex_moi_tanru_unit` grammar rule.
    rule "mex selbri" zantufa_mex_moi_tanru_unit(mekso) -> struct {
        /// A word from selmaho `Moi`.
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_before_selmaho(Moi));
        /// A word from selmaho `Moi`.
        field moi <- selmaho(Moi).warn(ExperimentalZantufaMex).wf();
    }

    /// Syntax model for sumti selbri parsed by the `sumti_selbri_sumti` grammar rule.
    rule "sumti selbri" sumti_selbri_sumti(sumti, letter_string) -> enum {
        /// The `sumti` alternative of sumti selbri.
        sumti,
        /// The `me_lerfu_sumti` alternative of sumti selbri.
        me_lerfu_sumti,
    }

    /// Syntax model for lerfu string parsed by the `me_lerfu_sumti` grammar rule.
    rule "lerfu string" me_lerfu_sumti(letter_string) -> struct {
        /// The words component of this syntax node.
        field words <- letter_string;
    }

    /// Syntax model for operator-to-selbri parsed by the `operator_selbri_tanru_unit` grammar rule.
    rule "operator-to-selbri" operator_selbri_tanru_unit(mekso_operator) -> struct {
        /// The `Nuha` cmavo marker.
        field nuha <- cmavo(Nuha).wf();
        /// The shared mekso operator child syntax node.
        field mekso_operator <- arc(mekso_operator);
    }

    /// Syntax model for grouped tanru parsed by the `grouped_tanru_unit` grammar rule.
    rule "grouped tanru" grouped_tanru_unit(tanru_unit, statement) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(connected_selbri(tanru_unit, statement));
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for grouped tanru parsed by the `grouped_jai_inner_tanru_unit` grammar rule.
    rule "grouped tanru" grouped_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        /// The `Ke` cmavo marker.
        field ke <- cmavo(Ke).wf();
        /// The shared selbri child syntax node.
        field selbri <- arc(connected_jai_inner_selbri(jai_inner_tanru_unit));
        /// The optional `Kehe` cmavo marker.
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Syntax model for selbri connection parsed by the `connected_jai_inner_selbri` grammar rule.
    rule "selbri connection" connected_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        /// The shared leading selbri child syntax node.
        field leading_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
        /// Ordered sequence of zero or more continuations components.
        field continuations <- [zero_or_more connected_jai_inner_selbri_continuation(jai_inner_tanru_unit)];
    }

    /// Syntax model for selbri connection continuation parsed by the `connected_jai_inner_selbri_continuation` grammar rule.
    rule "selbri connection continuation" connected_jai_inner_selbri_continuation(jai_inner_tanru_unit) -> struct {
        /// The connective component of this syntax node.
        field connective <- relation_afterthought_connective;
        /// The shared trailing selbri child syntax node.
        field trailing_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
    }

    /// Syntax model for selbri parsed by the `tanru_jai_inner_selbri` grammar rule.
    rule "selbri" tanru_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        /// The first unit component of this syntax node.
        field first_unit <- jai_inner_tanru_unit;
        /// Ordered sequence of zero or more additional units components.
        field additional_units <- [zero_or_more jai_inner_tanru_unit];
    }

    /// Syntax model for linked arguments parsed by the `linked_sumti` grammar rule.
    rule "linked arguments" linked_sumti(sumti, tense_modal) -> enum {
        /// The `place_tagged_linked_sumti` alternative of linked arguments.
        place_tagged_linked_sumti,
        /// The `tense_tagged_linked_sumti` alternative of linked arguments.
        tense_tagged_linked_sumti,
        /// The `plain_linked_sumti` alternative of linked arguments.
        plain_linked_sumti,
        /// The `empty_linked_sumti` alternative of linked arguments.
        empty_linked_sumti,
    }

    /// Syntax model for linked arguments parsed by the `place_tagged_linked_sumti` grammar rule.
    rule "linked arguments" place_tagged_linked_sumti(sumti) -> struct {
        /// A word from selmaho `Fa`.
        field fa <- selmaho(Fa).wf();
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Syntax model for linked arguments parsed by the `tense_tagged_linked_sumti` grammar rule.
    rule "linked arguments" tense_tagged_linked_sumti(sumti, tense_modal) -> struct {
        /// The shared tense modal child syntax node.
        field tense_modal <- arc(tense_modal);
        /// The shared sumti child syntax node.
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Syntax model for linked arguments parsed by the `plain_linked_sumti` grammar rule.
    rule "linked arguments" plain_linked_sumti(sumti) -> struct {
        /// The shared sumti child syntax node.
        field sumti <- arc(sumti);
    }

    /// Syntax model for linked arguments parsed by the `empty_linked_sumti` grammar rule.
    rule "linked arguments" empty_linked_sumti -> struct {
    }

    /// Syntax model for linked arguments parsed by the `bei_link` grammar rule.
    rule "linked arguments" bei_link(sumti, tense_modal) -> struct {
        /// The `Bei` cmavo marker.
        field bei <- cmavo(Bei).wf();
        /// The link component of this syntax node.
        field link <- linked_sumti(sumti, tense_modal);
    }

    /// Syntax model for linked arguments parsed by the `linkargs` grammar rule.
    rule "linked arguments" linkargs(sumti, tense_modal) -> struct {
        /// The `Be` cmavo marker.
        field be <- cmavo(Be).wf();
        /// The first link component of this syntax node.
        field first_link <- linked_sumti(sumti, tense_modal);
        /// Ordered sequence of zero or more bei links components.
        field bei_links <- [zero_or_more bei_link(sumti, tense_modal)];
        /// The optional `Beho` cmavo marker.
        field beho <- opt(cmavo(Beho).wf()).elidable_terminator(Beho);
    }

    /// Syntax model for abstraction parsed by the `abstraction_tanru_unit` grammar rule.
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

    /// Syntax model for abstractor connection parsed by the `abstractor_connection` grammar rule.
    rule "abstractor connection" abstractor_connection -> struct {
        /// The connective component of this syntax node.
        field connective <- standard_statement_connective;
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Syntax model for abstraction parsed by the `zantufa_statement_abstraction_tanru_unit` grammar rule.
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

    /// Syntax model for abstractor connection parsed by the `zantufa_abstractor_connection` grammar rule.
    rule "abstractor connection" zantufa_abstractor_connection -> struct {
        /// The connective component of this syntax node.
        field connective <- joik_connective;
        /// A word from selmaho `Nu`.
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
        /// The optional `Nai` cmavo marker.
        field nai <- opt(cmavo(Nai).wf());
    }
    }

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
        let result = strict_generated_text_parser_with_eof()
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
                })
            }
        };
        bityzba::new!(GeneratedRecoveredParsedTextAttempt {
            result,
            trace: finish.trace,
            unconsumed_directives: finish.unconsumed_recovery_directives,
            recovery_directives: finish.recovery_directives,
            effective_fail_token_indices: finish.effective_fail_token_indices,
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
                active_rule_contexts: error.active_rule_contexts().to_vec(),
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
