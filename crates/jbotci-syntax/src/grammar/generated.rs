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

    /// A UI/CAI indicator together with its optional attached NAI word.
    rule "leading indicator" leading_indicator -> struct {
        /// The UI or CAI indicator word.
        field indicator <- choice((selmaho(Ui), selmaho(Cai)));
        /// The optional NAI word attached to the indicator.
        field nai <- opt(cmavo(Nai));
    }

    /// Top-level text syntax, distinguishing XAUhA…KUhAU framing from ordinary text.
    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> enum {
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
    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> struct {
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
            modal_forethought_connective(tense_modal)
                .not()
                .ignore_then(text_leading_connective),
        );
        /// I-led statement prefixes that occur before the paragraph tree.
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
        field first <- paragraph;
        field additional_niho <- [zero_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Transparent product node for paragraphs; preserves the `paragraphs` component.
    rule "paragraphs" text_niho_paragraphs(statement_or_fragment, free_modifier) -> struct {
        field paragraphs <- [one_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    /// Product node for paragraph statement; preserves `i`, `connective`, and `free_modifiers` in source order.
    rule "paragraph statement" leading_i_statement(free_modifier, tense_modal) -> struct {
        field i <- cmavo(I);
        field connective <- opt(arc(i_paragraph_statement_connective(tense_modal)));
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
        field statements <- paragraph_statement_sequence(statement_or_fragment, free_modifier);
    }

    /// Product node for paragraph statement sequence; preserves `initial`, `following`, and `trailing` in source order.
    rule "paragraph statement sequence" paragraph_statement_sequence(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        field initial <- initial_paragraph_statement(statement_or_fragment);
        field following <- [zero_or_more following_paragraph_statement(statement_or_fragment, free_modifier)];
        field trailing <- [zero_or_more trailing_ijek_paragraph_statement()];
    }

    /// Product node for paragraph; preserves `i`, `niho`, `free_modifiers`, and `statements` in source order.
    rule "paragraph" i_niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        field i <- cmavo(I);
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    /// Product node for paragraph; preserves `niho`, `free_modifiers`, and `statements` in source order.
    rule "paragraph" niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    /// Transparent product node for paragraph statement; preserves the `statement` component.
    rule "paragraph statement" initial_paragraph_statement(statement_or_fragment) -> struct {
        #[tree_child(primary)]
        field statement <- arc(statement_or_fragment);
    }

    /// Product node for paragraph statement; preserves `i`, `free_modifiers`, and `statement` in source order.
    rule "paragraph statement" following_paragraph_statement(statement_or_fragment, free_modifier) -> struct {
        field i <- cmavo(I);
        assert !statement_connective;
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statement <- opt(arc(statement_or_fragment));
    }

    /// Product node for paragraph statement; preserves `i` and `connective` in source order.
    rule "paragraph statement" trailing_ijek_paragraph_statement -> struct {
        field i <- cmavo(I);
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

    /// Sum node for statement; selects among the `prenex_statement`, `forethought_statement`, `bridi_statement`, and `text_group_statement` forms.
    rule "statement" statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> enum {
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
    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        /// Uses the `zantufa_statement_terms_statement` product form, whose payload preserves `statement` and `tail`.
        when feature(ZantufaTerms) zantufa_statement_terms_statement,
        /// Uses the `statement_or_fragment_statement` product form, whose payload preserves `statement`.
        statement_or_fragment_statement,
        /// Uses the nested `fragment_statement` sum form and preserves its selected alternative.
        fragment_statement,
    }

    /// Product node for paragraph statement; preserves `statement` and `tail` in source order.
    rule "paragraph statement" zantufa_statement_terms_statement(statement, term) -> struct {
        field statement <- arc(statement);
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
        field iau <- cmavo(Ihau).warn(ExperimentalIauReset).wf();
        field terms <- [zero_or_more term];
    }

    /// Transparent product node for paragraph statement; preserves the `terms` component.
    rule "paragraph statement" zantufa_bare_statement_terms_tail(term) -> struct {
        field terms <- [one_or_more arc(term)];
    }

    /// Transparent product node for paragraph statement; preserves the `statement` component.
    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        field statement <- statement;
    }

    /// Sum node for fragment; selects among 12 forms including `prenex_fragment`, `selbri_fragment`, and `ek_fragment`.
    rule "fragment" fragment_statement(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
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
    rule "statement" statement_after_i_connective(statement, bridi, subbridi, tense_modal, text) -> enum {
        /// Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.
        when feature(ZantufaConnectives) forethought_statement,
        /// Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.
        bridi_statement,
        /// Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.
        text_group_statement,
    }

    /// Product node for fragment; preserves `first_na`, `second_na`, and `additional_na` in source order.
    rule "fragment" multiple_na_fragment -> struct {
        field first_na <- selmaho(Na);
        field second_na <- selmaho(Na);
        field additional_na <- [zero_or_more selmaho(Na)];
    }

    /// Transparent product node for fragment; preserves the `na` component.
    rule "fragment" single_na_fragment -> struct {
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
    }

    /// Transparent product node for fragment; preserves the `connective` component.
    rule "fragment" ek_fragment -> struct {
        #[tree_child(primary)]
        field connective <- ek_connective();
    }

    /// Transparent product node for fragment; preserves the `connective` component.
    rule "fragment" gihek_fragment -> struct {
        #[tree_child(primary)]
        field connective <- gihek_connective();
    }

    /// Product node for statement connection; preserves `leading_statement` and `continuations` in source order.
    rule "statement connection" i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        field continuations <- [one_or_more i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens)];
    }

    /// Product node for statement connective; preserves `i` and `connective` in source order.
    rule "statement connective" pending_i_connective -> struct {
        field i <- cmavo(I);
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
        field pending <- [one_or_more pending_i_connective];
        field i <- cmavo(I);
        field connective <- i_statement_connective(tense_modal);
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Product node for statement connection; preserves `i`, `connective`, and `trailing_statement` in source order.
    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field i <- cmavo(I);
        field connective <- i_statement_connective(tense_modal);
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Product node for statement connection; preserves `leading_statement`, `connective`, `i`, and `trailing_statement` in source order.
    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> struct {
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        field connective <- statement_connective;
        field i <- cmavo(I);
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    /// Product node for text group; preserves `tense_modal`, `tuhe`, `text`, and `tuhu` in source order.
    rule "text group" text_group_statement(text, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field tuhe <- cmavo(Tuhe).wf();
        #[tree_child(primary)]
        field text <- arc(text);
        field tuhu <- opt(cmavo(Tuhu).wf()).elidable_terminator(Tuhu);
    }

    /// Product node for prenex; preserves `terms` and `zohu` in source order.
    rule "prenex" prenex_fragment(term) -> struct {
        field terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
    }

    /// Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_statement` in source order.
    rule "prenex" prenex_statement(statement, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        #[tree_child(primary)]
        field inner_statement <- arc(statement);
    }

    /// Product node for statement; preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "statement" forethought_statement(statement, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- arc(statement);
        field first_branch <- forethought_statement_branch(statement);
        field additional_branches <- [zero_or_more zantufa_forethought_statement_branch(statement)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for statement branch; preserves `gik` and `statement` in source order.
    rule "statement branch" forethought_statement_branch(statement) -> struct {
        field gik <- gik_connective;
        field statement <- arc(statement);
    }

    /// Product node for statement branch; preserves `gik` and `statement` in source order.
    rule "statement branch" zantufa_forethought_statement_branch(statement) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field statement <- arc(statement);
    }

    /// Product node for statement; preserves `bridi` and `continuations` in source order.
    rule "statement" bridi_statement(bridi, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        field bridi <- arc(bridi);
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
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_subbridi <- arc(subbridi);
    }

    /// Product node for bridi continuation; preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe` in source order.
    rule "bridi continuation" ke_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        field connective <- relation_afterthought_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field trailing_subbridi <- arc(subbridi);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Transparent product node for selbri; preserves the `selbri` component.
    rule "selbri" selbri_fragment(selbri) -> struct {
        #[tree_child(primary)]
        field selbri <- arc(selbri);
    }

    /// Product node for terms; preserves `terms` and `vau` in source order.
    rule "terms" terms_fragment(term) -> struct {
        #[tree_child(primary)]
        field terms <- [one_or_more term];
        field vau <- opt(cmavo(Vau).wf()).elidable_terminator(Vau);
    }

    /// Transparent product node for mex; preserves the `quantifier` component.
    rule "mex" mekso_fragment(mekso, letter_tokens) -> struct {
        #[tree_child(primary)]
        field quantifier <- arc(quantifier(mekso, letter_tokens));
    }

    /// Transparent product node for mex; preserves the `expression` component.
    rule "mex" zantufa_mekso_fragment(mekso) -> struct {
        #[tree_child(primary)]
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_statement_item());
    }

    /// Product node for relative clauses; preserves `first` and `additional` in source order.
    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal, statement) -> struct {
        field first <- relative_clause_atom(sumti, subbridi, tense_modal, statement);
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal, statement)];
    }

    /// Transparent product node for relative clauses; preserves the `relative_clauses` component.
    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal, statement) -> struct {
        #[tree_child(primary)]
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
    }

    /// Transparent product node for linked arguments; preserves the `bei_links` component.
    rule "linked arguments" linked_sumti_continuation_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        field bei_links <- [one_or_more bei_link(sumti, tense_modal)];
    }

    /// Transparent product node for linked arguments; preserves the `linkargs` component.
    rule "linked arguments" linked_sumti_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        field linkargs <- linkargs(sumti, tense_modal);
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
        field leading_terms <- [one_or_more term];
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.
    rule "bridi" bridi_with_post_cu_terms(term, bridi_tail) -> struct {
        field leading_terms <- [one_or_more term];
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Product node for bridi; preserves `cu` and `bridi_tail` in source order.
    rule "bridi" bare_cu_bridi(bridi_tail) -> struct {
        field cu <- arc(cmavo(Cu).wf());
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi; preserves `cu` and `bridi_tail` in source order.
    rule "bridi" bare_cu_terms_bridi(term, bridi_tail) -> struct {
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    /// Transparent product node for bridi; preserves the `bridi_tail` component.
    rule "bridi" relation_only_bridi(bridi_tail) -> struct {
        field bridi_tail <- arc(bridi_tail);
    }

    /// Product node for bridi tail; preserves `terms` and `bridi_tail` in source order.
    rule "bridi tail" cu_terms_bridi_tail(term, bridi_tail) -> struct {
        field terms <- [one_or_more term];
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
        field ke <- cmavo(Ke).warn(ExperimentalZantufaGroupedBridiTail).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        field ke_continuation <- opt(arc(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `ke_continuation` in source order.
    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        assert !(relation_connective_as_bridi_tail, opt(arc(tense_modal)), cmavo(Ke));
        field ke_continuation <- opt(arc(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    /// Transparent product node for bridi tail; preserves the `bridi_tails` component.
    rule "bridi tail" afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail_without_tail_terms),
            zero_or_more: bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Transparent product node for bridi tail; preserves the `bridi_tails` component.
    rule "bridi tail" afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail),
            zero_or_more: bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal),
            element: bridi_tail,
        );
    }

    /// Product node for bridi tail; preserves `first` and `bo_continuation` in source order.
    rule "bridi tail" bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    /// Product node for bridi tail; preserves `first` and `bo_continuation` in source order.
    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
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
        field connection <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Transparent product node for forethought bridi connection; preserves the `connection` component.
    rule "forethought bridi connection" forethought_simple_bridi_tail(forethought_bridi_connection) -> struct {
        field connection <- arc(forethought_bridi_connection);
    }

    /// Product node for bridi tail; preserves `selbri` and `vau` in source order.
    rule "bridi tail" selbri_simple_bridi_tail_without_tail_terms(selbri) -> struct {
        field selbri <- arc(selbri);
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail; preserves `selbri`, `terms`, and `vau` in source order.
    rule "bridi tail" selbri_simple_bridi_tail(selbri, term) -> struct {
        field selbri <- arc(selbri);
        field terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Sum node for forethought bridi connection; selects among the `direct_forethought_bridi_connection`, `grouped_forethought_bridi_connection`, and `negated_forethought_bridi_connection` forms.
    rule "forethought bridi connection" forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal) -> enum {
        /// Uses the `direct_forethought_bridi_connection` product form, whose payload preserves `gek`, `first`, `first_branch`, and 4 other fields.
        direct_forethought_bridi_connection,
        /// Uses the `grouped_forethought_bridi_connection` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection,
        /// Uses the `negated_forethought_bridi_connection` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection,
    }

    /// Sum node for forethought bridi connection; selects among the `direct_forethought_bridi_connection_without_tail_terms`, `grouped_forethought_bridi_connection_without_tail_terms`, and `negated_forethought_bridi_connection_without_tail_terms` forms.
    rule "forethought bridi connection" forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal) -> enum {
        /// Uses the `direct_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `gek`, `first`, `first_branch`, and 3 other fields.
        direct_forethought_bridi_connection_without_tail_terms,
        /// Uses the `grouped_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.
        grouped_forethought_bridi_connection_without_tail_terms,
        /// Uses the `negated_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `na` and `inner`.
        negated_forethought_bridi_connection_without_tail_terms,
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 4 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection(subbridi, term, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- arc(subbridi);
        field first_branch <- forethought_bridi_branch(subbridi);
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 3 other fields in source order.
    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- arc(subbridi);
        field first_branch <- forethought_bridi_branch(subbridi);
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for forethought bridi branch; preserves `gik` and `branch` in source order.
    rule "forethought bridi branch" forethought_bridi_branch(subbridi) -> struct {
        field gik <- gik_connective;
        field branch <- arc(subbridi);
    }

    /// Product node for forethought bridi branch; preserves `gik` and `branch` in source order.
    rule "forethought bridi branch" zantufa_forethought_bridi_branch(subbridi) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field branch <- arc(subbridi);
    }

    /// Product node for forethought bridi connection; preserves `tense_modal`, `ke`, `inner`, and `kehe` in source order.
    rule "forethought bridi connection" grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- arc(forethought_bridi_connection);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Product node for forethought bridi connection; preserves `tense_modal`, `ke`, `inner`, and `kehe` in source order.
    rule "forethought bridi connection" grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    /// Product node for forethought bridi connection; preserves `na` and `inner` in source order.
    rule "forethought bridi connection" negated_forethought_bridi_connection(forethought_bridi_connection) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- arc(forethought_bridi_connection);
    }

    /// Product node for forethought bridi connection; preserves `na` and `inner` in source order.
    rule "forethought bridi connection" negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.
    rule "bridi tail connective" bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.
    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- gihek_connective();
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, `cu`, and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, and 4 other fields in source order.
    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    /// Product node for bridi tail connective; preserves `connective`, `cu`, and `bridi_tail` in source order.
    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    /// Product node for bridi tail connective; preserves `connective`, `cu`, `bridi_tail`, `tail_terms`, and `vau` in source order.
    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
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
        field bridi <- arc(bridi);
    }

    /// Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_subbridi` in source order.
    rule "prenex" prenex_subbridi(subbridi, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        field inner_subbridi <- arc(subbridi);
    }

    alias "term" term_guard =
        (relation_word(), cmavo(Bu).not()).not();

    /// Sum node for term; selects among the `pehe_termset_connection`, `bound_term_connection`, `termset_group`, `connected_term`, and `simple_term` forms.
    rule "term" term(statement, term, sumti, tense_modal, subbridi, selbri, free_modifier) -> enum {
        /// Uses the `pehe_termset_connection` product form, whose payload preserves `leading_term` and `continuations`.
        pehe_termset_connection,
        /// Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.
        bound_term_connection,
        /// Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.
        termset_group,
        /// Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.
        connected_term,
        /// Uses the nested `simple_term` sum form and preserves its selected alternative.
        simple_term,
    }

    /// Product node for termset connection; preserves `leading_term` and `continuations` in source order.
    rule "termset connection" pehe_termset_connection(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [one_or_more pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Product node for termset connection continuation; preserves `pehe`, `connective`, and `trailing_term` in source order.
    rule "termset connection continuation" pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field pehe <- cmavo(Pehe).wf();
        field connective <- statement_connective;
        field trailing_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Sum node for term; selects among the `bound_term_connection`, `termset_group`, and `simple_term` forms.
    rule "term" pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
        /// Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.
        bound_term_connection,
        /// Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.
        termset_group,
        /// Uses the nested `simple_term` sum form and preserves its selected alternative.
        simple_term,
    }

    /// Sum node for term; selects among 13 forms including `place_tagged_sumti_term`, `jai_tagged_sumti_term`, and `tagged_sumti_before_tag_term`.
    rule "term" simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
        /// Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_sumti_term,
        /// Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.
        jai_tagged_sumti_term,
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
        /// Uses the `forethought_termset` product form, whose payload preserves `m_nuhi`, `gek`, `terms`, and 4 other fields.
        forethought_termset,
        /// Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.
        nuhi_termset,
        /// Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.
        ke_termset,
    }

    /// Product node for term connection; preserves `leading_term`, `connective`, `bo`, and `trailing_term` in source order.
    rule "term connection" bound_term_connection(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field connective <- arc(bound_term_connective);
        field bo <- cmavo(Bo).wf();
        assert choice((
            feature(TermHierarchy),
            (
                feature(TermHierarchy).not(),
                sumti.not(),
            ).ignored(),
        ));
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        assert choice((
            feature(TermHierarchy),
            (
                feature(TermHierarchy).not(),
                sumti.not(),
            ).ignored(),
        ));
    }

    /// Sum node for term connective; selects among the `joik_connective` and `ek_connective` forms.
    rule "term connective" bound_term_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
    }

    /// Product node for term connection; preserves `leading_term` and `continuations` in source order.
    rule "term connection" connected_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [zero_or_more connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Product node for term connection continuation; preserves `connective` and `trailing_term` in source order.
    rule "term connection continuation" connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field connective <- connected_term_connective;
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Sum node for term connective; selects among the `joik_connective`, `jek_connective`, `ek_connective`, and `vuhu_nonlogical_connective` forms.
    rule "term connective" connected_term_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.
        vuhu_nonlogical_connective,
    }

    /// Product node for termset; preserves `leading_term` and `continuations` in source order.
    rule "termset" termset_group(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [one_or_more termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    /// Product node for termset continuation; preserves `cehe` and `trailing_term` in source order.
    rule "termset continuation" termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field cehe <- cmavo(Cehe).wf();
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    /// Product node for termset; preserves `m_nuhi`, `gek`, `terms`, and 4 other fields in source order.
    rule "termset" forethought_termset(term, tense_modal) -> struct {
        field m_nuhi <- opt(cmavo(Nuhi).wf());
        field gek <- modal_forethought_connective(tense_modal);
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
        field first_branch <- forethought_termset_branch(term);
        field additional_branches <- [zero_or_more zantufa_forethought_termset_branch(term)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for termset; preserves `gik`, `terms`, and `nuhu` in source order.
    rule "termset" forethought_termset_branch(term) -> struct {
        field gik <- gik_connective;
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Product node for termset; preserves `gik`, `terms`, and `nuhu` in source order.
    rule "termset" zantufa_forethought_termset_branch(term) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Product node for termset; preserves `nuhi`, `termset`, and `nuhu` in source order.
    rule "termset" nuhi_termset(term) -> struct {
        field nuhi <- cmavo(Nuhi).wf();
        field termset <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    /// Product node for termset; preserves `ke`, `termset`, and `kehe` in source order.
    rule "termset" ke_termset(term) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalKeTermset).wf();
        field termset <- [one_or_more arc(term)];
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for NOIhA adverbial; selects among the `noiha_variable_adverbial_term` and `noiha_relative_adverbial_term` forms.
    rule "NOIhA adverbial" noiha_adverbial_term(free_modifier, selbri) -> enum {
        /// Uses the `noiha_variable_adverbial_term` product form, whose payload preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku`.
        noiha_variable_adverbial_term,
        /// Uses the `noiha_relative_adverbial_term` product form, whose payload preserves `noiha`, `selbri`, and `fehu`.
        noiha_relative_adverbial_term,
    }

    /// Product node for NOIhA adverbial; preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku` in source order.
    rule "NOIhA adverbial" noiha_variable_adverbial_term(free_modifier, selbri) -> struct {
        field poiha <- selmaho(Noiha).wf();
        field free_modifiers <- [zero_or_more free_modifier];
        field selbri <- arc(selbri);
        field brigahi_ku <- cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
    }

    /// Product node for NOIhA adverbial; preserves `noiha`, `selbri`, and `fehu` in source order.
    rule "NOIhA adverbial" noiha_relative_adverbial_term(selbri) -> struct {
        field noiha <- selmaho(Noiha).wf();
        field selbri <- arc(selbri);
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Product node for FIhOI adverbial; preserves `fihoi`, `statement`, and `fihau` in source order.
    rule "FIhOI adverbial" fihoi_adverbial_term(statement) -> struct {
        field fihoi <- cmavo(Fihoi).warn(ExperimentalFihoiAdverbial).wf();
        field statement <- arc(statement);
        field fihau <- opt(cmavo(Fihau).wf()).elidable_terminator(Fihau);
    }

    /// Product node for SOI adverbial; preserves `soi`, `statement`, and `sehu` in source order.
    rule "SOI adverbial" soi_adverbial_term(statement) -> struct {
        field soi <- selmaho(Soi).warn(ExperimentalSoiAdverbial).wf();
        field statement <- arc(statement);
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    /// Transparent product node for term; preserves the `sumti` component.
    rule "term" sumti_term(sumti) -> struct {
        field sumti <- arc(sumti);
    }

    /// Product node for place tag; preserves `fa` and `sumti` in source order.
    rule "place tag" place_tagged_sumti_term(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Product node for NA KU term; preserves `na` and `na_ku` in source order.
    rule "NA KU term" na_ku_term -> struct {
        field na <- selmaho(Na);
        field na_ku <- cmavo(Ku).wf();
    }

    /// Transparent product node for NA term; preserves the `na` component.
    rule "NA term" bare_na_term(selbri, tense_modal) -> struct {
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

    /// Transparent product node for tag; preserves the `tense_modal` component.
    rule "tag" tagged_sumti_before_tag_term(tense_modal, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert tense_modal.lookahead();
    }

    /// Product node for tag; preserves `tense_modal` and `sumti` in source order.
    rule "tag" tagged_sumti_term(tense_modal, sumti, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert !selbri;
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Product node for tag; preserves `jai`, `tag`, and `sumti` in source order.
    rule "tag" jai_tagged_sumti_term(tense_modal, sumti) -> struct {
        assert feature(ZantufaTags);
        field jai <- cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
        field tag <- opt(arc(tense_modal));
        field sumti <- arc(sumti);
    }

    /// Sum node for tag; selects among 8 forms including `pu_before_nahe_leading_term_tag_tense`, `pu_distance_before_tag_leading_term_tag_tense`, and `zi_before_zi_leading_term_tag_tense`.
    rule "tag" leading_term_tag_tense_modal(tense_modal, selbri) -> enum {
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
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        assert selmaho(Nahe);
    }

    /// Product node for tag; preserves `pu`, `nai`, and `distance` in source order.
    rule "tag" pu_distance_before_tag_leading_term_tag_tense -> struct {
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Transparent product node for tag; preserves the `zi` component.
    rule "tag" zi_before_zi_leading_term_tag_tense -> struct {
        field zi <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    /// Transparent product node for tag; preserves the `va` component.
    rule "tag" va_before_va_leading_term_tag_tense -> struct {
        field va <- selmaho(Va).wf();
        assert selmaho(Va);
    }

    /// Product node for tag; preserves `mohi`, `direction`, `nai`, and `distance` in source order.
    rule "tag" mohi_before_mohi_leading_term_tag_tense -> struct {
        field mohi <- selmaho(Mohi).wf();
        field direction <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Va).wf());
        assert selmaho(Mohi);
    }

    /// Transparent product node for tag; preserves the `caha` component.
    rule "tag" caha_before_tag_leading_term_tag_tense(tense_modal) -> struct {
        field caha <- selmaho(Caha).wf().followed_by(tense_modal.lookahead());
    }

    /// Transparent product node for interval property; preserves the `property` component.
    rule "interval property" interval_property_leading_term_tag_tense(selbri) -> struct {
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

    /// Sum node for sumti; selects among the `sumti` and `tagged_elided_sumti` forms.
    rule "sumti" tagged_or_elided_sumti(sumti) -> enum {
        /// Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.
        sumti,
        /// Uses the `tagged_elided_sumti` product form, whose payload preserves `maybe_ku`.
        tagged_elided_sumti,
    }

    /// Transparent product node for elided sumti; preserves the `maybe_ku` component.
    rule "elided sumti" tagged_elided_sumti -> struct {
        field maybe_ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for sumti; preserves `base_sumti` and `vuho_attachment` in source order.
    rule "sumti" sumti(sumti, sumti_grouped, subbridi, tense_modal, statement) -> struct {
        field base_sumti <- arc(sumti_grouped);
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `grouped_tail` in source order.
    rule "sumti connection" sumti_grouped(sumti, sumti_afterthought, tense_modal, statement) -> struct {
        field leading_sumti <- arc(sumti_afterthought);
        field grouped_tail <- opt(grouped_sumti_tail(sumti, tense_modal));
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `continuations` in source order.
    rule "sumti connection" sumti_afterthought(sumti_bound, statement) -> struct {
        field leading_sumti <- arc(sumti_bound);
        field continuations <- [zero_or_more sumti_afterthought_tail(sumti_bound)];
    }

    /// Product node for sumti connection; preserves `leading_sumti` and `bound_tail` in source order.
    rule "sumti connection" sumti_bound(sumti_bound, sumti_forethought, tense_modal, statement) -> struct {
        field leading_sumti <- arc(sumti_forethought);
        field bound_tail <- opt(bound_sumti_tail(sumti_bound, tense_modal));
    }

    /// Sum node for sumti; selects among the `forethought_sumti` and `simple_sumti` forms.
    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// Uses the `forethought_sumti` product form, whose payload preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi`.
        forethought_sumti,
        /// Uses the `simple_sumti` product form, whose payload preserves `base_sumti` and `relative_clauses`.
        simple_sumti,
    }

    /// Product node for forethought sumti connection; preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal, statement) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field leading_sumti <- arc(sumti);
        field first_branch <- forethought_sumti_branch(sumti_forethought);
        field additional_branches <- [zero_or_more zantufa_forethought_sumti_branch(sumti_forethought)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.
    rule "forethought sumti connection" forethought_sumti_branch(sumti_forethought) -> struct {
        field gik <- gik_connective;
        field sumti <- arc(sumti_forethought);
    }

    /// Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.
    rule "forethought sumti connection" zantufa_forethought_sumti_branch(sumti_forethought) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field sumti <- arc(sumti_forethought);
    }

    /// Product node for sumti connection; preserves `connective`, `tense_modal`, `bo`, and `trailing_sumti` in source order.
    rule "sumti connection" bound_sumti_tail(sumti_bound, tense_modal) -> struct {
        field connective <- arc(argument_connective);
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_sumti <- arc(sumti_bound);
    }

    /// Product node for sumti connective; preserves `connective` and `sumti` in source order.
    rule "sumti connective" sumti_afterthought_tail(sumti_bound) -> struct {
        field connective <- argument_connective;
        field sumti <- arc(sumti_bound);
    }

    /// Product node for sumti connection; preserves `connective`, `tense_modal`, `ke`, `inner_sumti`, and `kehe` in source order.
    rule "sumti connection" grouped_sumti_tail(sumti, tense_modal) -> struct {
        field connective <- argument_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner_sumti <- arc(sumti);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Sum node for sumti relative phrase; selects among the `vuho_relative_sumti_attachment_tail` and `vuho_connected_sumti_attachment_tail` forms.
    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> enum {
        /// Uses the `vuho_relative_sumti_attachment_tail` product form, whose payload preserves `vuho`, `relative_clauses`, and `sumti_connection`.
        vuho_relative_sumti_attachment_tail,
        /// Uses the `vuho_connected_sumti_attachment_tail` product form, whose payload preserves `vuho` and `sumti_connection`.
        vuho_connected_sumti_attachment_tail,
    }

    /// Product node for sumti relative phrase; preserves `vuho`, `relative_clauses`, and `sumti_connection` in source order.
    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
        field sumti_connection <- opt(arc(sumti_connection_tail(sumti)));
    }

    /// Product node for sumti relative phrase; preserves `vuho` and `sumti_connection` in source order.
    rule "sumti relative phrase" vuho_connected_sumti_attachment_tail(sumti) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field sumti_connection <- arc(sumti_connection_tail(sumti));
    }

    /// Product node for sumti; preserves `base_sumti` and `relative_clauses` in source order.
    rule "sumti" simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> struct {
        field base_sumti <- arc(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement));
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Sum node for sumti; selects among the `sumti_base` and `quantified_sumti` forms.
    rule "sumti" sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// Uses the nested `sumti_base` sum form and preserves its selected alternative.
        sumti_base,
        /// Uses the `quantified_sumti` product form, whose payload preserves `quantifier` and `inner_sumti`.
        quantified_sumti,
    }

    /// Sum node for sumti; selects among 16 forms including `scalar_negated_sumti_with_bo`, `scalar_negated_sumti`, and `lahe_sumti`.
    rule "sumti" sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement) -> enum {
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
        /// Uses the `name_sumti` product form, whose payload preserves `la` and `names`.
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
    rule "quantified sumti" quantified_sumti(sumti_base, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        field inner_sumti <- arc(sumti_base);
    }

    /// Product node for sumti connective; preserves `connective` and `sumti` in source order.
    rule "sumti connective" sumti_connection_tail(sumti) -> struct {
        field connective <- argument_connective;
        field sumti <- arc(sumti);
    }

    /// Product node for quantifier; preserves `number` and `boi` in source order.
    rule "quantifier" pa_run_quantifier(letter_tokens) -> struct {
        field number <- number_words(letter_tokens).wf();
        field boi <- opt(cmavo(Boi).wf()).elidable_terminator(Boi);
    }

    /// Product node for quantifier; preserves `vei`, `mekso`, and `veho` in source order.
    rule "quantifier" mekso_quantifier(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field mekso <- arc(mekso);
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
        field mekso <- arc(mekso);
    }

    /// Transparent product node for quantifier; preserves the `mekso` component.
    rule "quantifier" zantufa_priority_raw_mekso_quantifier(mekso, letter_tokens) -> struct {
        assert zantufa_raw_mekso_quantifier_guard(letter_tokens);
        field mekso <- arc(mekso);
    }

    /// Sum node for quantifier; selects among the `zantufa_priority_raw_mekso_quantifier`, `mekso_quantifier`, `pa_run_quantifier`, and `zantufa_raw_mekso_quantifier` forms.
    rule "quantifier" quantifier(mekso, letter_tokens) -> enum {
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
    rule "number mex" number_mekso(letter_tokens) -> struct {
        field quantifier <- arc(pa_run_quantifier(letter_tokens));
    }

    /// Transparent product node for VUhU operator; preserves the `vuhu` component.
    rule "VUhU operator" primitive_mekso_operator -> struct {
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Sum node for operator; selects among the `afterthought_mekso_operator`, `bound_mekso_operator`, and `simple_mekso_operator` forms.
    rule "operator" mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// Uses the `afterthought_mekso_operator` product form, whose payload preserves `operators`.
        afterthought_mekso_operator,
        /// Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.
        bound_mekso_operator,
        /// Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.
        simple_mekso_operator,
    }

    /// Transparent product node for operator; preserves the `operators` component.
    rule "operator" afterthought_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        field operators <- chain(
            first: arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri)),
            zero_or_more: afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri),
            element: trailing_operator,
        );
    }

    /// Product node for operator continuation; preserves `connective` and `trailing_operator` in source order.
    rule "operator continuation" afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri) -> struct {
        field connective <- standard_statement_connective;
        field trailing_operator <- arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri));
    }

    /// Sum node for operator; selects among the `bound_mekso_operator` and `simple_mekso_operator` forms.
    rule "operator" bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.
        bound_mekso_operator,
        /// Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.
        simple_mekso_operator,
    }

    /// Product node for operator; preserves `left_operator`, `connective`, `bo`, and `right_operator` in source order.
    rule "operator" bound_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        field left_operator <- arc(simple_mekso_operator(mekso, mekso_operator, sumti, selbri));
        field connective <- standard_statement_connective;
        field bo <- cmavo(Bo).wf();
        field right_operator <- arc(mekso_operator);
    }

    /// Sum node for operator; selects among 10 forms including `converted_mekso_operator`, `scalar_negated_mekso_operator`, and `forethought_mekso_operator`.
    rule "operator" simple_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        /// Uses the `converted_mekso_operator` product form, whose payload preserves `se` and `inner_operator`.
        converted_mekso_operator,
        /// Uses the `scalar_negated_mekso_operator` product form, whose payload preserves `nahe` and `inner_operator`.
        scalar_negated_mekso_operator,
        /// Uses the `forethought_mekso_operator` product form, whose payload preserves `guhek`, `left_operator`, `gik`, and `right_operator`.
        forethought_mekso_operator,
        /// Uses the `grouped_mekso_operator` product form, whose payload preserves `ke`, `inner_operator`, and `kehe`.
        grouped_mekso_operator,
        /// Uses the `selbri_mekso_operator` product form, whose payload preserves `nahu`, `selbri`, and `tehu`.
        selbri_mekso_operator,
        /// Uses the `operand_mekso_operator` product form, whose payload preserves `maho`, `mekso`, and `tehu`.
        operand_mekso_operator,
        /// Uses the `zantufa_maho_selbri_mekso_operator` product form, whose payload preserves `maho`, `selbri`, and `tehu`.
        when feature(ZantufaMex) zantufa_maho_selbri_mekso_operator,
        /// Uses the `zantufa_maho_sumti_mekso_operator` product form, whose payload preserves `maho`, `sumti`, and `tehu`.
        when feature(ZantufaMex) zantufa_maho_sumti_mekso_operator,
        /// Uses the `zantufa_connective_mekso_operator` product form, whose payload preserves `connective`.
        when feature(ZantufaMex) zantufa_connective_mekso_operator,
        /// Uses the `primitive_mekso_operator` product form, whose payload preserves `vuhu`.
        primitive_mekso_operator,
    }

    /// Product node for converted operator; preserves `se` and `inner_operator` in source order.
    rule "converted operator" converted_mekso_operator(mekso_operator) -> struct {
        field se <- selmaho(Se).wf();
        field inner_operator <- arc(mekso_operator);
    }

    /// Product node for converted operator; preserves `nahe` and `inner_operator` in source order.
    rule "converted operator" scalar_negated_mekso_operator(mekso_operator) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_operator <- arc(mekso_operator);
    }

    /// Product node for operator; preserves `guhek`, `left_operator`, `gik`, and `right_operator` in source order.
    rule "operator" forethought_mekso_operator(mekso_operator) -> struct {
        field guhek <- guhek_connective;
        field left_operator <- arc(mekso_operator);
        field gik <- gik_connective;
        field right_operator <- arc(mekso_operator);
    }

    /// Product node for grouped operator; preserves `ke`, `inner_operator`, and `kehe` in source order.
    rule "grouped operator" grouped_mekso_operator(mekso_operator) -> struct {
        field ke <- cmavo(Ke).wf();
        field inner_operator <- arc(mekso_operator);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for selbri-to-operator; preserves `nahu`, `selbri`, and `tehu` in source order.
    rule "selbri-to-operator" selbri_mekso_operator(selbri) -> struct {
        field nahu <- cmavo(Nahu).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for operand-to-operator; preserves `maho`, `mekso`, and `tehu` in source order.
    rule "operand-to-operator" operand_mekso_operator(mekso) -> struct {
        field maho <- cmavo(Maho).wf();
        field mekso <- arc(mekso);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for selbri-to-operator; preserves `maho`, `selbri`, and `tehu` in source order.
    rule "selbri-to-operator" zantufa_maho_selbri_mekso_operator(selbri) -> struct {
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for sumti-to-operator; preserves `maho`, `sumti`, and `tehu` in source order.
    rule "sumti-to-operator" zantufa_maho_sumti_mekso_operator(sumti) -> struct {
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        field sumti <- arc(sumti);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Transparent product node for connective operator; preserves the `connective` component.
    rule "connective operator" zantufa_connective_mekso_operator -> struct {
        field connective <- arc(operand_connective);
        assert !cmavo(Cu);
    }

    /// Sum node for operand; selects among the `afterthought_mekso_operand`, `bound_mekso_operand`, and `simple_mekso_operand` forms.
    rule "operand" mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// Uses the `afterthought_mekso_operand` product form, whose payload preserves `operands`.
        afterthought_mekso_operand,
        /// Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.
        bound_mekso_operand,
        /// Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.
        simple_mekso_operand,
    }

    /// Transparent product node for operand connective; preserves the `operands` component.
    rule "operand connective" afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field operands <- chain(
            first: arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier)),
            zero_or_more: afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            element: trailing_expression,
        );
    }

    /// Product node for operand continuation; preserves `operand_connective` and `trailing_expression` in source order.
    rule "operand continuation" afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field operand_connective <- operand_connective;
        field trailing_expression <- arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
    }

    /// Sum node for operand; selects among the `bound_mekso_operand` and `simple_mekso_operand` forms.
    rule "operand" bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.
        bound_mekso_operand,
        /// Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.
        simple_mekso_operand,
    }

    /// Product node for operand connective; preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression` in source order.
    rule "operand connective" bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field left_expression <- arc(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
        field operand_connective <- operand_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field right_expression <- arc(mekso_operand);
    }

    /// Sum node for operand; selects among 10 forms including `forethought_mekso_operand`, `qualified_mekso_operand`, and `parenthesized_mekso_operand`.
    rule "operand" simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        /// Uses the `forethought_mekso_operand` product form, whose payload preserves `gek`, `left_expression`, `gik`, and `right_expression`.
        forethought_mekso_operand,
        /// Uses the `qualified_mekso_operand` product form, whose payload preserves `nahe`, `bo`, `inner_expression`, and `luhu`.
        qualified_mekso_operand,
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
        /// Uses the `zantufa_scalar_negated_mekso_operand` product form, whose payload preserves `nahe` and `inner_expression`.
        when feature(ZantufaMex) zantufa_scalar_negated_mekso_operand,
        /// Uses the `zantufa_selbri_mohe_mekso_operand` product form, whose payload preserves `mohe`, `selbri`, and `tehu`.
        when feature(ZantufaMex) zantufa_selbri_mohe_mekso_operand,
    }

    /// Product node for scalar-negated operand; preserves `nahe` and `inner_expression` in source order.
    rule "scalar-negated operand" zantufa_scalar_negated_mekso_operand(mekso_operand) -> struct {
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        field inner_expression <- arc(mekso_operand);
    }

    /// Product node for qualified operand; preserves `nahe`, `bo`, `inner_expression`, and `luhu` in source order.
    rule "qualified operand" qualified_mekso_operand(mekso_operand) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo);
        field inner_expression <- arc(mekso_operand);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for forethought mex; preserves `gek`, `left_expression`, `gik`, and `right_expression` in source order.
    rule "forethought mex" forethought_mekso_operand(mekso_operand, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field left_expression <- arc(mekso_operand);
        field gik <- gik_connective;
        field right_expression <- arc(mekso_operand);
    }

    /// Product node for sumti operand; preserves `mohe`, `sumti`, and `tehu` in source order.
    rule "sumti operand" sumti_mekso_operand(sumti) -> struct {
        field mohe <- cmavo(Mohe).wf();
        field sumti <- arc(sumti);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for selbri operand; preserves `mohe`, `selbri`, and `tehu` in source order.
    rule "selbri operand" zantufa_selbri_mohe_mekso_operand(selbri) -> struct {
        field mohe <- cmavo(Mohe).warn(ExperimentalZantufaMex).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for selbri operand; preserves `nihe`, `selbri`, and `tehu` in source order.
    rule "selbri operand" selbri_mekso_operand(selbri) -> struct {
        field nihe <- cmavo(Nihe).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for parenthesized mex; preserves `vei`, `inner_expression`, and `veho` in source order.
    rule "parenthesized mex" parenthesized_mekso_operand(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field inner_expression <- arc(mekso);
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    /// Product node for mekso array; preserves `johi`, `expressions`, and `tehu` in source order.
    rule "mekso array" array_mekso_operand(mekso) -> struct {
        field johi <- cmavo(Johi).wf();
        field expressions <- [one_or_more mekso];
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    /// Product node for lerfu string; preserves `first_letter` and `continuations` in source order.
    rule "lerfu string" letter_string(letter_tokens) -> struct {
        field first_letter <- arc(letter_tokens);
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
        field pa <- pa_word();
    }

    /// Transparent product node for lerfu string continuation; preserves the `letter` component.
    rule "lerfu string continuation" letter_string_lerfu_continuation(letter_tokens) -> struct {
        field letter <- arc(letter_tokens);
    }

    /// Product node for number; preserves `first_number` and `continuations` in source order.
    rule "number" number_words(letter_tokens) -> struct {
        field first_number <- pa_word();
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
        field pa <- pa_word();
    }

    /// Transparent product node for number continuation; preserves the `letter` component.
    rule "number continuation" number_word_lerfu_continuation(letter_tokens) -> struct {
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
        field word <- word_category(LetterWord);
    }

    /// Product node for lerfu word; preserves `lau` and `letter` in source order.
    rule "lerfu word" lau_lerfu_word(letter_tokens) -> struct {
        field lau <- selmaho(Lau);
        field letter <- arc(letter_tokens);
    }

    /// Product node for lerfu word; preserves `tei`, `letters`, and `foi` in source order.
    rule "lerfu word" tei_lerfu_word(letter_string) -> struct {
        field tei <- cmavo(Tei);
        field letters <- arc(letter_string);
        field foi <- cmavo(Foi);
    }

    /// Product node for lerfu string; preserves `letters`, `boi`, and `free_modifiers` in source order.
    rule "lerfu string" lerfu_string_mekso(letter_string, free_modifier) -> struct {
        field letters <- letter_string;
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Sum node for mex; selects among the `zantufa_bo_grouped_mekso_base`, `mekso_operand`, `forethought_call_mekso`, and `zantufa_grouped_mekso_operand_sequence` forms.
    rule "mex" mekso_base(mekso, mekso_base, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator) -> enum {
        /// Uses the `zantufa_bo_grouped_mekso_base` product form, whose payload preserves `first` and `continuations`.
        when feature(ZantufaMex) zantufa_bo_grouped_mekso_base,
        /// Uses the nested `mekso_operand` sum form and preserves its selected alternative.
        mekso_operand,
        /// Uses the `forethought_call_mekso` product form, whose payload preserves `peho`, `operator`, `operands`, and `kuhe`.
        forethought_call_mekso,
        /// Uses the `zantufa_grouped_mekso_operand_sequence` product form, whose payload preserves `ke`, `operands`, and `kehe`.
        when feature(ZantufaMex) zantufa_grouped_mekso_operand_sequence,
    }

    /// Product node for grouped mex; preserves `first` and `continuations` in source order.
    rule "grouped mex" zantufa_bo_grouped_mekso_base(mekso_operand) -> struct {
        field first <- arc(mekso_operand);
        field continuations <- [one_or_more zantufa_bo_grouped_mekso_continuation(mekso_operand)];
    }

    /// Product node for grouped mex; preserves `bo` and `expression` in source order.
    rule "grouped mex" zantufa_bo_grouped_mekso_continuation(mekso_operand) -> struct {
        field bo <- cmavo(Bo).warn(ExperimentalZantufaMex).wf();
        field expression <- arc(mekso_operand);
    }

    /// Product node for grouped mex; preserves `ke`, `operands`, and `kehe` in source order.
    rule "grouped mex" zantufa_grouped_mekso_operand_sequence(mekso_operand) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalZantufaMex).wf();
        field operands <- [one_or_more arc(mekso_operand)];
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for mex; preserves `left_expression` and `tail` in source order.
    rule "mex" mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field left_expression <- arc(mekso_base);
        field tail <- opt(mekso_precedence_tail(mekso_precedence, mekso_operator));
    }

    /// Product node for mex precedence tail; preserves `bihe`, `operator`, and `right_expression` in source order.
    rule "mex precedence tail" mekso_precedence_tail(mekso_precedence, mekso_operator) -> struct {
        field bihe <- cmavo(Bihe).wf();
        field operator <- arc(mekso_operator);
        field right_expression <- arc(mekso_precedence);
    }

    /// Product node for mex; preserves `first_expression` and `continuations` in source order.
    rule "mex" infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        field continuations <- [zero_or_more infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    /// Product node for mex continuation; preserves `operator` and `right_expression` in source order.
    rule "mex continuation" infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        field operator <- arc(mekso_operator);
        field right_expression <- arc(mekso_precedence);
    }

    /// Product node for mex; preserves `first_expression` and `continuations` in source order.
    rule "mex" zantufa_infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        field continuations <- [zero_or_more zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    /// Product node for mex continuation; preserves `operators` and `right_expression` in source order.
    rule "mex continuation" zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        field operators <- [one_or_more arc(mekso_operator)];
        field right_expression <- opt(arc(mekso_precedence));
    }

    /// Product node for forethought mex; preserves `peho`, `operator`, `operands`, and `kuhe` in source order.
    rule "forethought mex" forethought_call_mekso(mekso_base, mekso_operator) -> struct {
        field peho <- opt(cmavo(Peho).wf());
        field operator <- arc(mekso_operator);
        field operands <- [one_or_more mekso_base];
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Sum node for mex; selects among the `zantufa_reverse_polish_mekso`, `zantufa_infix_mekso`, `infix_mekso`, and `reverse_polish_mekso` forms.
    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts) -> enum {
        /// Uses the `zantufa_reverse_polish_mekso` product form, whose payload preserves `fuha`, `operands`, `operator`, `tails`, and `kuhe`.
        when feature(ZantufaMex) zantufa_reverse_polish_mekso,
        /// Uses the `zantufa_infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.
        when feature(ZantufaMex) zantufa_infix_mekso,
        /// Uses the `infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.
        infix_mekso,
        /// Uses the `reverse_polish_mekso` product form, whose payload preserves `fuha` and `parts`.
        reverse_polish_mekso,
    }

    /// Product node for reverse Polish mex; preserves `fuha`, `operands`, `operator`, `tails`, and `kuhe` in source order.
    rule "reverse Polish mex" zantufa_reverse_polish_mekso(mekso_base, mekso_operator) -> struct {
        field fuha <- cmavo(Fuha).warn(ExperimentalZantufaMex).wf();
        field operands <- [one_or_more mekso_base];
        field operator <- arc(mekso_operator);
        field tails <- [zero_or_more zantufa_reverse_polish_tail(mekso_base, mekso_operator)];
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    /// Product node for reverse Polish mex tail; preserves `operands` and `operator` in source order.
    rule "reverse Polish mex tail" zantufa_reverse_polish_tail(mekso_base, mekso_operator) -> struct {
        field operands <- [zero_or_more mekso_base];
        field operator <- arc(mekso_operator);
    }

    /// Product node for reverse Polish mex; preserves `first_operand` and `tails` in source order.
    rule "reverse Polish mex" reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> struct {
        field first_operand <- arc(mekso_operand);
        field tails <- [zero_or_more reverse_polish_parts_tail(reverse_polish_parts, mekso_operator)];
    }

    /// Product node for reverse Polish mex tail; preserves `right_parts` and `operator` in source order.
    rule "reverse Polish mex tail" reverse_polish_parts_tail(reverse_polish_parts, mekso_operator) -> struct {
        field right_parts <- arc(reverse_polish_parts);
        field operator <- mekso_operator;
    }

    /// Product node for reverse Polish mex; preserves `fuha` and `parts` in source order.
    rule "reverse Polish mex" reverse_polish_mekso(reverse_polish_parts) -> struct {
        field fuha <- cmavo(Fuha).wf();
        field parts <- arc(reverse_polish_parts);
    }

    /// Product node for number sumti; preserves `li`, `expression`, and `loho` in source order.
    rule "number sumti" number_sumti(mekso) -> struct {
        field li <- selmaho(Li).wf();
        #[tree_child(primary)]
        field expression <- arc(mekso);
        field loho <- opt(cmavo(Loho).wf()).elidable_terminator(Loho);
    }

    /// Product node for lerfu string; preserves `words`, `boi`, and `free_modifiers` in source order.
    rule "lerfu string" lerfu_string_sumti(letter_string, free_modifier) -> struct {
        field words <- letter_string;
        assert !selmaho(Moi);
        assert !selmaho(Mai);
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        field free_modifiers <- [zero_or_more free_modifier];
    }

    /// Product node for converted sumti; preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu` in source order.
    rule "converted sumti" lahe_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        field lahe <- selmaho(Lahe).wf();
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for converted term; preserves `lahe`, `inner_term`, and `luhu` in source order.
    rule "converted term" lahe_term_wrapper(term) -> struct {
        field lahe <- selmaho(Lahe).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated term; preserves `nahe`, `bo`, `inner_term`, and `luhu` in source order.
    rule "scalar-negated term" scalar_negated_term_wrapper_with_bo(term) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated term; preserves `nahe`, `inner_term`, and `luhu` in source order.
    rule "scalar-negated term" scalar_negated_term_wrapper(term) -> struct {
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated sumti; preserves `nahe`, `bo`, `inner_sumti`, and `luhu` in source order.
    rule "scalar-negated sumti" scalar_negated_sumti_with_bo(sumti) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for scalar-negated sumti; preserves `nahe`, `inner_sumti`, and `luhu` in source order.
    rule "scalar-negated sumti" scalar_negated_sumti(sumti) -> struct {
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    /// Product node for bridi description; preserves `lohoi`, `additional_heads`, `statement`, and `kuhau` in source order.
    rule "bridi description" bridi_description_sumti(statement) -> struct {
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
        field additional_heads <- [zero_or_more lohoi_description_head_continuation()];
        #[tree_child(primary)]
        field statement <- arc(statement);
        field kuhau <- opt(cmavo(Kuhau).wf()).elidable_terminator(Kuhau);
    }

    /// Product node for bridi description; preserves `connective` and `lohoi` in source order.
    rule "bridi description" lohoi_description_head_continuation -> struct {
        field connective <- joik_connective;
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
    }

    /// Transparent product node for sumti; preserves the `koha` component.
    rule "sumti" pro_sumti -> struct {
        field koha <- word_category(ProSumti).wf();
    }

    /// Product node for name; preserves `la` and `names` in source order.
    rule "name" name_sumti -> struct {
        field la <- selmaho(La).wf();
        field names <- [one_or_more cmevla_word()].wf();
    }

    /// Transparent product node for descriptor; preserves the `description` component.
    rule "descriptor" description_head -> struct {
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    /// Transparent product node for descriptor connective; preserves the `connective` component.
    rule "descriptor connective" description_head_connective -> struct {
        field connective <- arc(jek_connective);
    }

    /// Product node for description; preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku` in source order.
    rule "description" description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field leading_description_head <- arc(description_head());
        field connective <- description_head_connective();
        field trailing_description_head <- arc(description_head());
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field description <- description_head();
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `outer_quantifier`, `description`, `tail`, and `ku` in source order.
    rule "description" descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field outer_quantifier <- quantifier(mekso, letter_tokens);
        field description <- description_head();
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    /// Product node for description; preserves `quantifier`, `selbri`, `ku`, and `relative_clauses` in source order.
    rule "description" descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        #[tree_child(primary)]
        field selbri <- arc(selbri);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for description tail; preserves `leading_tail_elements` and `tail` in source order.
    rule "description tail" description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field leading_tail_elements <- leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement);
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement));
    }

    /// Sum node for description tail; selects among the `quantifier_relation_description_tail`, `quantifier_sumti_description_tail`, and `relation_description_tail` forms.
    rule "description tail" description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> enum {
        /// Uses the `quantifier_relation_description_tail` product form, whose payload preserves `quantifier`, `selbri`, and `relative_clauses`.
        quantifier_relation_description_tail,
        /// Uses the `quantifier_sumti_description_tail` product form, whose payload preserves `quantifier` and `sumti`.
        quantifier_sumti_description_tail,
        /// Uses the `relation_description_tail` product form, whose payload preserves `selbri` and `relative_clauses`.
        relation_description_tail,
    }

    /// Product node for description tail; preserves `tail_sumti` and `relative_clauses` in source order.
    rule "description tail" leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement) -> struct {
        field tail_sumti <- opt(description_tail_sumti(sumti_base));
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Transparent product node for description tail; preserves the `sumti` component.
    rule "description tail" description_tail_sumti(sumti_base) -> struct {
        assert !pa_word();
        field sumti <- arc(sumti_base);
    }

    /// Product node for description tail; preserves `selbri` and `relative_clauses` in source order.
    rule "description tail" relation_description_tail(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field selbri <- arc(selbri);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for description tail; preserves `quantifier`, `selbri`, and `relative_clauses` in source order.
    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        field selbri <- arc(selbri);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for description tail; preserves `quantifier` and `sumti` in source order.
    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
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
        field lu <- cmavo(Lu).wf();
        field text <- arc(text);
        field lihu <- opt(cmavo(Lihu).wf()).elidable_terminator(Lihu);
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_mehoi_compound_quote -> struct {
        field quote <- quote_marker(Mehoi).warn(ExperimentalMehOiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_zohoi_compound_quote -> struct {
        field quote <- choice((
            quote_marker(Zohoi),
            quote_marker(Lahoi),
        )).warn(ExperimentalZohOiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_rahoi_compound_quote -> struct {
        field quote <- quote_marker(Rahoi).warn(ExperimentalZantufaRahoiQuote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" experimental_gohoi_compound_quote -> struct {
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" generic_compound_quote -> struct {
        field quote <- word_category(Quote).wf();
    }

    /// Transparent product node for quote; preserves the `quote` component.
    rule "quote" quoted_sumti(text) -> struct {
        #[tree_child(primary)]
        field quote <- arc(quote(text));
    }

    /// Product node for vocative phrase; preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses` in source order.
    rule "vocative phrase" selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        field selbri <- arc(selbri);
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for vocative phrase; preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses` in source order.
    rule "vocative phrase" cmevla_vocative_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        field names <- [one_or_more cmevla_word()].wf();
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    /// Sum node for vocative phrase; selects among the `selbri_vocative_sumti`, `cmevla_vocative_sumti`, and `sumti` forms.
    rule "vocative phrase" vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> enum {
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
        field first_coi <- selmaho(Coi);
        field first_nai <- opt(cmavo(Nai));
        field additional_coi <- [zero_or_more additional_coi_vocative_marker()];
        field doi <- opt(cmavo(Doi));
    }

    /// Product node for vocative marker; preserves `coi` and `nai` in source order.
    rule "vocative marker" additional_coi_vocative_marker -> struct {
        field coi <- selmaho(Coi);
        field nai <- opt(cmavo(Nai));
    }

    /// Transparent product node for vocative marker; preserves the `doi` component.
    rule "vocative marker" doi_vocative_marker_words -> struct {
        field doi <- cmavo(Doi);
    }

    /// Sum node for free modifier; selects among 9 forms including `text_replacement_free_modifier`, `zantufa_sei_statement_free_modifier`, and `sei_free_modifier`.
    rule "free modifier" free_modifier(sumti, subbridi, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier, statement) -> enum {
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
    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field vocative_markers <- vocative_marker_words().wf();
        field sumti <- opt(arc(vocative_sumti(sumti, subbridi, selbri, tense_modal, statement)));
        field dohu <- opt(cmavo(Dohu).prohibited_wf()).elidable_terminator(Dohu);
    }

    /// Product node for parenthetical text; preserves `to`, `text`, and `toi` in source order.
    rule "parenthetical text" parenthetical_text(text) -> struct {
        field to <- selmaho(To).wf();
        field text <- arc(text);
        field toi <- opt(cmavo(Toi).prohibited_wf()).elidable_terminator(Toi);
    }

    /// Product node for metalinguistic comment; preserves `sei`, `terms`, `cu`, `selbri`, and `sehu` in source order.
    rule "metalinguistic comment" sei_free_modifier(term, selbri) -> struct {
        field sei <- selmaho(Sei).wf();
        field terms <- [zero_or_more term];
        field cu <- opt(cmavo(Cu).wf());
        field selbri <- arc(selbri);
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    /// Product node for metalinguistic comment; preserves `sei`, `statement`, and `sehu` in source order.
    rule "metalinguistic comment" zantufa_sei_statement_free_modifier(statement) -> struct {
        field sei <- selmaho(Sei).warn(ExperimentalZantufaStatementFreeModifier).wf();
        field statement <- arc(statement);
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    /// Sum node for subscript; selects among the `xi_number_free_modifier`, `xi_lerfu_string_free_modifier`, and `xi_parenthesized_free_modifier` forms.
    rule "subscript" xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier) -> enum {
        /// Uses the `xi_number_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_number_free_modifier,
        /// Uses the `xi_lerfu_string_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_lerfu_string_free_modifier,
        /// Uses the `xi_parenthesized_free_modifier` product form, whose payload preserves `xi` and `expression`.
        xi_parenthesized_free_modifier,
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_number_free_modifier(letter_tokens) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(number_mekso(letter_tokens));
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_lerfu_string_free_modifier(letter_string, free_modifier) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(lerfu_string_mekso(letter_string, free_modifier));
    }

    /// Product node for subscript; preserves `xi` and `expression` in source order.
    rule "subscript" xi_parenthesized_free_modifier(mekso) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(parenthesized_mekso_operand(mekso));
    }

    /// Product node for utterance ordinal; preserves `number` and `mai` in source order.
    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        field number <- number_or_letter_words(letter_tokens, letter_string)
            .followed_by(selmaho(Mai).ignored());
        field mai <- selmaho(Mai).wf();
    }

    /// Product node for utterance ordinal; preserves `expression` and `mai` in source order.
    rule "utterance ordinal" zantufa_mekso_mai_free_modifier(mekso) -> struct {
        field expression <- arc(mekso.followed_by(selmaho(Mai).ignored()));
        field mai <- selmaho(Mai).warn(ExperimentalZantufaMex).wf();
    }

    /// Product node for reciprocal; preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu` in source order.
    rule "reciprocal" soi_free_modifier(sumti) -> struct {
        field soi <- cmavo(Soi).wf();
        field leading_sumti <- arc(sumti);
        field trailing_sumti <- opt(arc(sumti));
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
        field lohai <- cmavo(Lohai);
        field old_words <- [zero_or_more word_before_sahai_or_lehai()];
        field sahai <- opt(cmavo(Sahai));
        field new_words <- [zero_or_more word_before_lehai()];
        field lehai <- cmavo(Lehai).wf();
    }

    /// Product node for replacement phrase; preserves `sahai`, `new_words`, and `lehai` in source order.
    rule "replacement phrase" new_only_text_replacement_free_modifier -> struct {
        field sahai <- cmavo(Sahai);
        field new_words <- [zero_or_more word_before_lehai()];
        field lehai <- cmavo(Lehai).wf();
    }

    /// Transparent product node for replacement phrase; preserves the `lehai` component.
    rule "replacement phrase" close_only_text_replacement_free_modifier -> struct {
        field lehai <- cmavo(Lehai).wf();
    }

    /// Sum node for relative clauses; selects among the `joined_relative_clause_tail` and `connected_relative_clause_tail` forms.
    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal, statement) -> enum {
        /// Uses the `joined_relative_clause_tail` product form, whose payload preserves `zihe` and `inner`.
        joined_relative_clause_tail,
        /// Uses the `connected_relative_clause_tail` product form, whose payload preserves `connective` and `inner`.
        connected_relative_clause_tail,
    }

    /// Product node for relative clause; preserves `zihe` and `inner` in source order.
    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field zihe <- cmavo(Zihe).wf();
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    /// Product node for relative clause; preserves `connective` and `inner` in source order.
    rule "relative clause" connected_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field connective <- relative_clause_connective;
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    /// Sum node for relative clause connective; selects among the `joik_connective` and `jek_connective` forms.
    rule "relative clause connective" relative_clause_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
    }

    /// Sum node for relative clause; selects among the `sumti_association_relative_clause` and `bridi_relative_clause` forms.
    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal, statement) -> enum {
        /// Uses the `sumti_association_relative_clause` product form, whose payload preserves `association_marker`, `sumti`, and `gehu`.
        sumti_association_relative_clause,
        /// Uses the nested `bridi_relative_clause` sum form and preserves its selected alternative.
        bridi_relative_clause,
    }

    /// Product node for sumti association phrase; preserves `association_marker`, `sumti`, and `gehu` in source order.
    rule "sumti association phrase" sumti_association_relative_clause(sumti, tense_modal) -> struct {
        field association_marker <- selmaho(Goi).wf();
        field sumti <- arc(relative_sumti(sumti, tense_modal));
        field gehu <- opt(cmavo(Gehu).wf()).elidable_terminator(Gehu);
    }

    /// Sum node for sumti association phrase; selects among the `tense_tagged_relative_sumti`, `na_ku_relative_sumti`, and `plain_relative_sumti` forms.
    rule "sumti association phrase" relative_sumti(sumti, tense_modal) -> enum {
        /// Uses the `tense_tagged_relative_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_relative_sumti,
        /// Uses the `na_ku_relative_sumti` product form, whose payload preserves `na` and `ku`.
        na_ku_relative_sumti,
        /// Uses the `plain_relative_sumti` product form, whose payload preserves `sumti`.
        plain_relative_sumti,
    }

    /// Product node for sumti association phrase; preserves `na` and `ku` in source order.
    rule "sumti association phrase" na_ku_relative_sumti -> struct {
        field na <- selmaho(Na);
        field ku <- cmavo(Ku).wf();
    }

    /// Product node for tagged sumti; preserves `tense_modal` and `sumti` in source order.
    rule "tagged sumti" tense_tagged_relative_sumti(tense_modal, sumti) -> struct {
        field tense_modal <- arc(tense_modal);
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Transparent product node for sumti association phrase; preserves the `sumti` component.
    rule "sumti association phrase" plain_relative_sumti(sumti) -> struct {
        field sumti <- arc(sumti);
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
        field poi <- choice((
            cmavo(Poi),
            cmavo(Pohoi),
            cmavo(Voi),
            cmavo(Voihi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        field statement <- arc(statement);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `noi`, `statement`, and `kuho` in source order.
    rule "relative clause" zantufa_incidental_statement_relative_clause(statement) -> struct {
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        field statement <- arc(statement);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `poi`, `subbridi`, and `kuho` in source order.
    rule "relative clause" restrictive_bridi_relative_clause(subbridi, statement) -> struct {
        field poi <- choice((
            cmavo(Poi),
            cmavo(Pohoi),
            cmavo(Voi),
            cmavo(Voihi),
        )).wf();
        field subbridi <- arc(subbridi);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for relative clause; preserves `noi`, `subbridi`, and `kuho` in source order.
    rule "relative clause" incidental_bridi_relative_clause(subbridi, statement) -> struct {
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).wf();
        field subbridi <- arc(subbridi);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    /// Product node for ek; preserves `na`, `se`, `a`, and `nai` in source order.
    rule "ek" ek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field a <- selmaho(A).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for ek; preserves `na`, `se`, `jehi`, and `nai` in source order.
    rule "ek" jehi_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field jehi <- selmaho(Jehi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.
    rule "jek" jek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for joik; selects among the `joi_connective`, `simple_interval_connective`, and `closed_interval_connective` forms.
    rule "joik" joik_connective -> enum {
        /// Uses the `joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        joi_connective,
        /// Uses the `simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        simple_interval_connective,
        /// Uses the `closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.
        closed_interval_connective,
    }

    /// Product node for joik; preserves `se`, `joi`, and `nai` in source order.
    rule "joik" joi_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field joi <- selmaho(Joi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval; preserves `se`, `bihi`, and `nai` in source order.
    rule "interval" simple_interval_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.
    rule "interval" closed_interval_connective -> struct {
        #[tree_child(primary)]
        field left_interval <- selmaho(Gaho);
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi);
        field nai <- opt(cmavo(Nai));
        #[tree_child(primary)]
        field right_interval <- selmaho(Gaho).wf();
    }

    /// Transparent product node for non-logical connective; preserves the `vuhu` component.
    rule "non-logical connective" vuhu_nonlogical_connective -> struct {
        #[tree_child(primary)]
        field vuhu <- selmaho(Vuhu).wf();
    }

    /// Sum node for sumti connective; selects among the `cehe_connective`, `ek_connective`, `jehi_connective`, `joik_connective`, and `vuhu_nonlogical_connective` forms.
    rule "sumti connective" argument_connective -> enum {
        /// Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.
        cehe_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `jehi_connective` product form, whose payload preserves `na`, `se`, `jehi`, and `nai`.
        jehi_connective,
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.
        vuhu_nonlogical_connective,
    }

    /// Sum node for operand connective; selects among the `joik_connective`, `ek_connective`, and `jek_connective` forms.
    rule "operand connective" operand_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.
        ek_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
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
        field connective <- arc(statement_connective);
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
        field connective <- arc(paragraph_standard_statement_connective);
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo)));
    }

    /// Sum node for statement connective; selects among the `paragraph_joi_connective`, `paragraph_simple_interval_connective`, `paragraph_closed_interval_connective`, and `paragraph_jek_connective` forms.
    rule "statement connective" paragraph_standard_statement_connective -> enum {
        /// Uses the `paragraph_joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.
        paragraph_joi_connective,
        /// Uses the `paragraph_simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.
        paragraph_simple_interval_connective,
        /// Uses the `paragraph_closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.
        paragraph_closed_interval_connective,
        /// Uses the `paragraph_jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        paragraph_jek_connective,
    }

    /// Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.
    rule "jek" paragraph_jek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja);
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for joik; preserves `se`, `joi`, and `nai` in source order.
    rule "joik" paragraph_joi_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field joi <- selmaho(Joi);
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for interval; preserves `se`, `bihi`, and `nai` in source order.
    rule "interval" paragraph_simple_interval_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi);
        field nai <- opt(cmavo(Nai));
    }

    /// Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.
    rule "interval" paragraph_closed_interval_connective -> struct {
        #[tree_child(primary)]
        field left_interval <- selmaho(Gaho);
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi);
        field nai <- opt(cmavo(Nai));
        #[tree_child(primary)]
        field right_interval <- selmaho(Gaho);
    }

    /// Product node for statement connective; preserves `tense_modal` and `bo` in source order.
    rule "statement connective" i_tag_bo_paragraph_statement_connective(tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo);
    }

    /// Product node for statement connective; preserves `tense_modal` and `bo` in source order.
    rule "statement connective" i_tag_bo_statement_connective(tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
    }

    /// Product node for termset connective; preserves `cehe` and `nai` in source order.
    rule "termset connective" cehe_connective -> struct {
        #[tree_child(primary)]
        field cehe <- cmavo(Cehe).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for gihek; preserves `na`, `se`, `giha`, and `nai` in source order.
    rule "gihek" gihek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field giha <- selmaho(Giha).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for forethought selbri connective; preserves `nahe`, `se`, `guha`, and `nai` in source order.
    rule "forethought selbri connective" guhek_connective -> struct {
        field nahe <- opt(selmaho(Nahe));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field guha <- selmaho(Guha).wf();
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
        field connective <- arc(relation_afterthought_connective);
    }

    /// Sum node for forethought connective; selects among the `ga_forethought_connective`, `joik_jek_gi_forethought_connective`, `jek_gi_forethought_connective`, `modal_gi_forethought_connective`, and `zantufa_initial_gi_forethought_connective` forms.
    rule "forethought connective" modal_forethought_connective(tense_modal) -> enum {
        /// Uses the `ga_forethought_connective` product form, whose payload preserves `se`, `ga`, and `nai`.
        ga_forethought_connective,
        /// Uses the `joik_jek_gi_forethought_connective` product form, whose payload preserves `connective`, `gi`, and `bo`.
        joik_jek_gi_forethought_connective,
        /// Uses the `jek_gi_forethought_connective` product form, whose payload preserves `na`, `se`, `ja`, and 3 other fields.
        jek_gi_forethought_connective,
        /// Uses the `modal_gi_forethought_connective` product form, whose payload preserves `tense_modal`, `gi`, and `bo`.
        modal_gi_forethought_connective,
        /// Uses the `zantufa_initial_gi_forethought_connective` product form, whose payload preserves `gi`, `tail`, and `bo`.
        when feature(ZantufaConnectives) zantufa_initial_gi_forethought_connective,
    }

    /// Product node for forethought connective; preserves `se`, `ga`, and `nai` in source order.
    rule "forethought connective" ga_forethought_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ga <- selmaho(Ga).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for forethought connective; preserves `gi`, `tail`, and `bo` in source order.
    rule "forethought connective" zantufa_initial_gi_forethought_connective -> struct {
        field gi <- cmavo(Gi).warn(ExperimentalZantufaGek).wf();
        field tail <- arc(standard_statement_connective);
        field bo <- opt(cmavo(Bo).wf());
    }

    /// Product node for forethought connective; preserves `connective`, `gi`, and `bo` in source order.
    rule "forethought connective" joik_jek_gi_forethought_connective -> struct {
        field connective <- arc(joik_connective);
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Product node for forethought connective; preserves `na`, `se`, `ja`, and 3 other fields in source order.
    rule "forethought connective" jek_gi_forethought_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja).warn(ExperimentalZantufaGek).wf();
        field nai <- opt(cmavo(Nai).wf());
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Product node for forethought connective; preserves `tense_modal`, `gi`, and `bo` in source order.
    rule "forethought connective" modal_gi_forethought_connective(tense_modal) -> struct {
        field tense_modal <- arc(tense_modal);
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    /// Product node for forethought connective; preserves `gi` and `nai` in source order.
    rule "forethought connective" gik_connective -> struct {
        #[tree_child(primary)]
        field gi <- cmavo(Gi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Transparent product node for forethought connective; preserves the `gi` component.
    rule "forethought connective" zantufa_extra_gik_connective -> struct {
        #[tree_child(primary)]
        field gi <- cmavo(Gi).warn(ExperimentalZantufaNaryForethought).wf();
    }

    /// Transparent product node for tag; preserves the `body` component.
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
        field body <- tense_modal_body(selbri);
    }

    /// Sum node for tag; selects among the `connected_tense_modal` and `tense_modal_atom` forms.
    rule "tag" tense_modal_body(selbri) -> enum {
        /// Uses the `connected_tense_modal` product form, whose payload preserves `first` and `continuations`.
        connected_tense_modal,
        /// Uses the nested `tense_modal_atom` sum form and preserves its selected alternative.
        tense_modal_atom,
    }

    /// Product node for connected tag; preserves `first` and `continuations` in source order.
    rule "connected tag" connected_tense_modal(selbri) -> struct {
        field first <- arc(tense_modal_atom(selbri));
        field continuations <- [one_or_more connected_tense_modal_continuation(selbri)];
    }

    /// Product node for connected tag continuation; preserves `connective` and `tense_modal` in source order.
    rule "connected tag continuation" connected_tense_modal_continuation(selbri) -> struct {
        field connective <- tense_modal_connective;
        field tense_modal <- arc(tense_modal_atom(selbri));
    }

    /// Sum node for tag connective; selects among the `joik_connective` and `jek_connective` forms.
    rule "tag connective" tense_modal_connective -> enum {
        /// Uses the nested `joik_connective` sum form and preserves its selected alternative.
        joik_connective,
        /// Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.
        jek_connective,
    }

    /// Sum node for tag; selects among 8 forms including `composite_tense`, `fiho_tense`, and `modal_tense`.
    rule "tag" tense_modal_atom(selbri) -> enum {
        /// Uses the nested `composite_tense` sum form and preserves its selected alternative.
        composite_tense,
        /// Uses the `fiho_tense` product form, whose payload preserves `fiho`, `selbri`, and `fehu`.
        fiho_tense,
        /// Uses the `modal_tense` product form, whose payload preserves `nahe`, `se`, `bai`, `nai`, and `ki`.
        modal_tense,
        /// Uses the `nahe_se_flat_prefixed_tense` product form, whose payload preserves `nahe`, `se`, and `atom`.
        nahe_se_flat_prefixed_tense,
        /// Uses the `se_flat_prefixed_tense` product form, whose payload preserves `se` and `atom`.
        se_flat_prefixed_tense,
        /// Uses the `fa_flat_tag_tense` product form, whose payload preserves `fa`.
        fa_flat_tag_tense,
        /// Uses the `zantufa_recursive_tag_tense` product form, whose payload preserves `first_prefix`, `additional_prefixes`, and `atom`.
        when feature(ZantufaTags) zantufa_recursive_tag_tense,
        /// Uses the `sticky_tense` product form, whose payload preserves `ki`.
        sticky_tense,
    }

    /// Product node for FIhO modal; preserves `fiho`, `selbri`, and `fehu` in source order.
    rule "FIhO modal" fiho_tense(selbri) -> struct {
        field fiho <- cmavo(Fiho).wf();
        field selbri <- arc(selbri);
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    /// Transparent product node for tag; preserves the `fa` component.
    rule "tag" fa_flat_tag_tense -> struct {
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    /// Sum node for tag; selects among the `fa_flat_tag_atom`, `modal_flat_tag_atom`, and `composite_flat_tag_atom` forms.
    rule "tag" flat_tag_atom -> enum {
        /// Uses the `fa_flat_tag_atom` product form, whose payload preserves `fa`.
        fa_flat_tag_atom,
        /// Uses the `modal_flat_tag_atom` product form, whose payload preserves `modal`.
        modal_flat_tag_atom,
        /// Uses the `composite_flat_tag_atom` product form, whose payload preserves `composite`.
        composite_flat_tag_atom,
    }

    /// Transparent product node for tag; preserves the `fa` component.
    rule "tag" fa_flat_tag_atom -> struct {
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    /// Transparent product node for modal tag; preserves the `modal` component.
    rule "modal tag" modal_flat_tag_atom -> struct {
        field modal <- arc(modal_tense());
    }

    /// Transparent product node for tag; preserves the `composite` component.
    rule "tag" composite_flat_tag_atom -> struct {
        field composite <- arc(composite_tense());
    }

    /// Product node for tag; preserves `nahe`, `se`, and `atom` in source order.
    rule "tag" nahe_se_flat_prefixed_tense -> struct {
        field nahe <- selmaho(Nahe).warn(ExperimentalFlattenedTag).wf();
        field se <- opt(selmaho(Se).wf());
        field atom <- flat_tag_atom();
    }

    /// Product node for tag; preserves `se` and `atom` in source order.
    rule "tag" se_flat_prefixed_tense -> struct {
        field se <- selmaho(Se).warn(ExperimentalFlattenedTag).wf();
        field atom <- flat_tag_atom();
    }

    /// Product node for tag; preserves `first_prefix`, `additional_prefixes`, and `atom` in source order.
    rule "tag" zantufa_recursive_tag_tense -> struct {
        field first_prefix <- choice((
            selmaho(Nahe),
            selmaho(Se),
        )).warn(ExperimentalZantufaRecursiveTag).wf();
        field additional_prefixes <- [zero_or_more choice((
            selmaho(Nahe),
            selmaho(Se),
        )).wf()];
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

    /// Sum node for tag; selects among the `prefixed_time_space_caha_tense`, `time_space_caha_ki_tense`, and `cuhe_tense` forms.
    rule "tag" composite_tense -> enum {
        /// Uses the `prefixed_time_space_caha_tense` product form, whose payload preserves `nahe`, `tense`, and `ki`.
        prefixed_time_space_caha_tense,
        /// Uses the `time_space_caha_ki_tense` product form, whose payload preserves `tense` and `ki`.
        time_space_caha_ki_tense,
        /// Uses the `cuhe_tense` product form, whose payload preserves `cuhe`.
        cuhe_tense,
    }

    /// Product node for tag; preserves `nahe`, `tense`, and `ki` in source order.
    rule "tag" prefixed_time_space_caha_tense -> struct {
        field nahe <- selmaho(Nahe).wf();
        field tense <- arc(time_space_caha_tense);
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Product node for tag; preserves `tense` and `ki` in source order.
    rule "tag" time_space_caha_ki_tense -> struct {
        field tense <- arc(time_space_caha_tense);
        field ki <- opt(arc(ki_composite_tense()));
    }

    /// Sum node for tag; selects among the `time_then_space_caha_tense`, `space_then_time_caha_tense`, and `caha_tense` forms.
    rule "tag" time_space_caha_tense -> enum {
        /// Uses the `time_then_space_caha_tense` product form, whose payload preserves `time`, `space`, and `caha`.
        time_then_space_caha_tense,
        /// Uses the `space_then_time_caha_tense` product form, whose payload preserves `space`, `time`, and `caha`.
        space_then_time_caha_tense,
        /// Uses the `caha_tense` product form, whose payload preserves `caha`.
        caha_tense,
    }

    /// Product node for time tense; preserves `time`, `space`, and `caha` in source order.
    rule "time tense" time_then_space_caha_tense -> struct {
        field time <- arc(time_tense);
        field space <- opt(arc(space_tense));
        field caha <- opt(arc(caha_tense()));
    }

    /// Product node for space tense; preserves `space`, `time`, and `caha` in source order.
    rule "space tense" space_then_time_caha_tense -> struct {
        field space <- arc(space_tense);
        field time <- opt(arc(time_tense));
        field caha <- opt(arc(caha_tense()));
    }

    /// Sum node for time tense; selects among the `time_tense_with_zi`, `time_tense_with_offset`, `time_tense_with_interval`, and `time_tense_with_properties` forms.
    rule "time tense" time_tense -> enum {
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
    rule "time tense" time_tense_with_zi -> struct {
        field zi <- arc(zi_time_distance_tense());
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_offset -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [one_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_interval -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- arc(zeha_time_interval_tense());
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    /// Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.
    rule "time tense" time_tense_with_properties -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [one_or_more arc(interval_property_tense)];
    }

    /// Sum node for interval property; selects among the `numbered_interval_property_tense`, `tahe_interval_property_tense`, and `zaho_interval_property_tense` forms.
    rule "interval property" interval_property_tense -> enum {
        /// Uses the `numbered_interval_property_tense` product form, whose payload preserves `number`, `roi`, and `nai`.
        numbered_interval_property_tense,
        /// Uses the `tahe_interval_property_tense` product form, whose payload preserves `tahe` and `nai`.
        tahe_interval_property_tense,
        /// Uses the `zaho_interval_property_tense` product form, whose payload preserves `zaho` and `nai`.
        zaho_interval_property_tense,
    }

    /// Product node for interval property; preserves `number`, `roi`, and `nai` in source order.
    rule "interval property" numbered_interval_property_tense -> struct {
        field number <- interval_property_number_words().wf();
        field roi <- selmaho(Roi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for number; preserves `first_number` and `continuations` in source order.
    rule "number" interval_property_number_words -> struct {
        field first_number <- pa_word();
        field continuations <- [zero_or_more interval_property_number_word_continuation];
    }

    /// Sum node for number continuation; selects among the `interval_property_number_pa_continuation` and `interval_property_number_letter_continuation` forms.
    rule "number continuation" interval_property_number_word_continuation -> enum {
        /// Uses the `interval_property_number_pa_continuation` product form, whose payload preserves `pa`.
        interval_property_number_pa_continuation,
        /// Uses the `interval_property_number_letter_continuation` product form, whose payload preserves `letter`.
        interval_property_number_letter_continuation,
    }

    /// Transparent product node for number continuation; preserves the `pa` component.
    rule "number continuation" interval_property_number_pa_continuation -> struct {
        field pa <- pa_word();
    }

    /// Transparent product node for number continuation; preserves the `letter` component.
    rule "number continuation" interval_property_number_letter_continuation -> struct {
        field letter <- word_category(LetterWord);
    }

    /// Product node for interval property; preserves `tahe` and `nai` in source order.
    rule "interval property" tahe_interval_property_tense -> struct {
        field tahe <- selmaho(Tahe).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for interval property; preserves `zaho` and `nai` in source order.
    rule "interval property" zaho_interval_property_tense -> struct {
        field zaho <- selmaho(Zaho).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for time tense; preserves `pu`, `nai`, and `distance` in source order.
    rule "time tense" pu_time_offset_tense -> struct {
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Zi).wf());
    }

    /// Transparent product node for time tense; preserves the `zi` component.
    rule "time tense" zi_time_distance_tense -> struct {
        field zi <- selmaho(Zi).wf();
    }

    /// Product node for time interval; preserves `zeha` and `direction` in source order.
    rule "time interval" zeha_time_interval_tense -> struct {
        field zeha <- selmaho(Zeha).wf();
        field direction <- opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
    }

    /// Sum node for space tense; selects among the `space_tense_with_va`, `space_tense_with_offset`, `space_tense_with_interval`, and `space_tense_with_mohi` forms.
    rule "space tense" space_tense -> enum {
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
    rule "space tense" space_tense_with_va -> struct {
        field va <- arc(va_space_distance_tense());
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_offset -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [one_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_interval -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- arc(space_interval_tense);
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    /// Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.
    rule "space tense" space_tense_with_mohi -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- arc(mohi_space_offset_tense());
    }

    /// Transparent product node for space tense; preserves the `va` component.
    rule "space tense" va_space_distance_tense -> struct {
        field va <- selmaho(Va).wf();
    }

    /// Product node for space tense; preserves `faha`, `nai`, and `distance` in source order.
    rule "space tense" faha_space_offset_tense -> struct {
        field faha <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Va).wf());
    }

    /// Product node for space interval; preserves `faha` and `nai` in source order.
    rule "space interval" faha_interval_direction_tense -> struct {
        field faha <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Sum node for space interval; selects among the `space_interval_with_extent_tense` and `space_interval_properties_tense` forms.
    rule "space interval" space_interval_tense -> enum {
        /// Uses the `space_interval_with_extent_tense` product form, whose payload preserves `extent`, `direction`, and `properties`.
        space_interval_with_extent_tense,
        /// Uses the `space_interval_properties_tense` product form, whose payload preserves `first` and `additional`.
        space_interval_properties_tense,
    }

    /// Product node for space interval; preserves `extent`, `direction`, and `properties` in source order.
    rule "space interval" space_interval_with_extent_tense -> struct {
        field extent <- arc(space_interval_extent_tense);
        field direction <- opt(arc(faha_interval_direction_tense()));
        field properties <- opt(arc(space_interval_properties_tense()));
    }

    /// Sum node for space interval; selects among the `veha_space_interval_tense` and `viha_space_interval_tense` forms.
    rule "space interval" space_interval_extent_tense -> enum {
        /// Uses the `veha_space_interval_tense` product form, whose payload preserves `veha` and `viha`.
        veha_space_interval_tense,
        /// Uses the `viha_space_interval_tense` product form, whose payload preserves `viha`.
        viha_space_interval_tense,
    }

    /// Product node for space interval; preserves `first` and `additional` in source order.
    rule "space interval" space_interval_properties_tense -> struct {
        field first <- arc(fehe_interval_property_tense());
        field additional <- [zero_or_more arc(fehe_interval_property_tense())];
    }

    /// Product node for space interval; preserves `veha` and `viha` in source order.
    rule "space interval" veha_space_interval_tense -> struct {
        field veha <- selmaho(Veha).wf();
        field viha <- opt(selmaho(Viha).wf());
    }

    /// Transparent product node for space interval; preserves the `viha` component.
    rule "space interval" viha_space_interval_tense -> struct {
        field viha <- selmaho(Viha).wf();
    }

    /// Product node for space interval property; preserves `fehe` and `property` in source order.
    rule "space interval property" fehe_interval_property_tense -> struct {
        field fehe <- cmavo(Fehe).wf();
        field property <- arc(interval_property_tense);
    }

    /// Product node for space tense; preserves `mohi` and `offset` in source order.
    rule "space tense" mohi_space_offset_tense -> struct {
        field mohi <- selmaho(Mohi).wf();
        field offset <- arc(faha_space_offset_tense());
    }

    /// Transparent product node for tag; preserves the `caha` component.
    rule "tag" caha_tense -> struct {
        field caha <- selmaho(Caha).wf();
    }

    /// Transparent product node for tag; preserves the `ki` component.
    rule "tag" ki_composite_tense -> struct {
        field ki <- cmavo(Ki).wf();
    }

    /// Transparent product node for tag; preserves the `cuhe` component.
    rule "tag" cuhe_tense -> struct {
        field cuhe <- selmaho(Cuhe).wf();
    }

    /// Product node for modal tag; preserves `nahe`, `se`, `bai`, `nai`, and `ki` in source order.
    rule "modal tag" modal_tense -> struct {
        field nahe <- opt(selmaho(Nahe).wf());
        field se <- opt(selmaho(Se).wf());
        field bai <- selmaho(Bai).wf();
        field nai <- opt(cmavo(Nai).wf());
        field ki <- opt(cmavo(Ki).wf());
    }

    /// Transparent product node for tag; preserves the `ki` component.
    rule "tag" sticky_tense -> struct {
        field ki <- cmavo(Ki).wf();
    }

    /// Sum node for selbri; selects among the `tagged_selbri` and `untagged_selbri` forms.
    rule "selbri" selbri(selbri, co_selbri, tense_modal, statement) -> enum {
        /// Uses the `tagged_selbri` product form, whose payload preserves `tense_modal` and `inner_selbri`.
        tagged_selbri,
        /// Uses the nested `untagged_selbri` sum form and preserves its selected alternative.
        untagged_selbri,
    }

    /// Sum node for selbri; selects among the `negated_selbri`, `co_selbri`, and `forethought_selbri_connection` forms.
    rule "selbri" untagged_selbri(selbri, co_selbri, statement) -> enum {
        /// Uses the `negated_selbri` product form, whose payload preserves `na` and `inner_selbri`.
        negated_selbri,
        /// Uses the `co_selbri` product form, whose payload preserves `leading_selbri` and `co_tail`.
        co_selbri,
        /// Uses the `forethought_selbri_connection` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.
        forethought_selbri_connection,
    }

    /// Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.
    rule "tagged selbri" tagged_selbri(selbri, co_selbri, tense_modal, statement) -> struct {
        field tense_modal <- arc(tense_modal);
        field inner_selbri <- arc(untagged_selbri(selbri, co_selbri, statement));
    }

    /// Product node for negated selbri; preserves `na` and `inner_selbri` in source order.
    rule "negated selbri" negated_selbri(selbri) -> struct {
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
        field inner_selbri <- arc(selbri);
    }

    /// Product node for selbri; preserves `leading_selbri` and `co_tail` in source order.
    rule "selbri" co_selbri(co_selbri, tanru_unit, statement) -> struct {
        field leading_selbri <- arc(connected_selbri(tanru_unit, statement));
        field co_tail <- opt(co_selbri_tail(co_selbri));
    }

    /// Product node for selbri; preserves `co` and `trailing_selbri` in source order.
    rule "selbri" co_selbri_tail(co_selbri) -> struct {
        field co <- cmavo(Co).wf();
        field trailing_selbri <- arc(co_selbri);
    }

    /// Product node for forethought selbri connection; preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "forethought selbri connection" forethought_selbri_connection(selbri) -> struct {
        field guhek <- guhek_connective;
        field leading_selbri <- arc(selbri);
        field first_branch <- forethought_selbri_branch(selbri);
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_branch(selbri)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for forethought selbri connection; preserves `gik` and `selbri` in source order.
    rule "forethought selbri connection" forethought_selbri_branch(selbri) -> struct {
        field gik <- gik_connective;
        field selbri <- arc(selbri);
    }

    /// Product node for forethought selbri connection; preserves `gik` and `selbri` in source order.
    rule "forethought selbri connection" zantufa_forethought_selbri_branch(selbri) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field selbri <- arc(selbri);
    }

    /// Product node for selbri connection; preserves `leading_selbri` and `continuations` in source order.
    rule "selbri connection" connected_selbri(tanru_unit, statement) -> struct {
        field leading_selbri <- arc(tanru_selbri(tanru_unit, statement));
        field continuations <- [zero_or_more connected_selbri_continuation(tanru_unit, statement)];
    }

    /// Product node for selbri connection continuation; preserves `connective` and `trailing_selbri` in source order.
    rule "selbri connection continuation" connected_selbri_continuation(tanru_unit, statement) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_selbri <- arc(tanru_selbri(tanru_unit, statement));
    }

    /// Product node for tanru; preserves `first_unit` and `additional_units` in source order.
    rule "tanru" tanru_selbri(tanru_unit, statement) -> struct {
        field first_unit <- tanru_unit;
        field additional_units <- [zero_or_more tanru_unit];
    }

    /// Transparent product node for tanru unit; preserves the `units` component.
    rule "tanru unit" tanru_unit(bo_or_linked_tanru_unit, statement) -> struct {
        field units <- chain(
            first: arc(bo_or_linked_tanru_unit),
            zero_or_more: tanru_unit_continuation(bo_or_linked_tanru_unit, statement),
            element: trailing_unit,
        );
    }

    /// Product node for tanru unit continuation; preserves `connective` and `trailing_unit` in source order.
    rule "tanru unit continuation" tanru_unit_continuation(bo_or_linked_tanru_unit, statement) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Sum node for tanru unit; selects among the `forethought_selbri_group_tanru_unit`, `bound_tanru_unit`, `assigned_pro_bridi_tanru_unit`, and `linked_tanru_unit` forms.
    rule "tanru unit" bo_or_linked_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        /// Uses the `forethought_selbri_group_tanru_unit` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.
        forethought_selbri_group_tanru_unit,
        /// Uses the `bound_tanru_unit` product form, whose payload preserves `leading_unit`, `bo_connective`, `bo_tense_modal`, `bo`, and `trailing_unit`.
        bound_tanru_unit,
        /// Uses the `assigned_pro_bridi_tanru_unit` product form, whose payload preserves `base` and `assignments`.
        assigned_pro_bridi_tanru_unit,
        /// Uses the `linked_tanru_unit` product form, whose payload preserves `base` and `linkargs`.
        linked_tanru_unit,
    }

    /// Product node for forethought selbri connection; preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi` in source order.
    rule "forethought selbri connection" forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri, statement) -> struct {
        field guhek <- guhek_connective;
        field leading_selbri <- arc(selbri);
        field first_branch <- forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement);
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    /// Product node for forethought selbri connection; preserves `gik` and `unit` in source order.
    rule "forethought selbri connection" forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        field gik <- gik_connective;
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Product node for forethought selbri connection; preserves `gik` and `unit` in source order.
    rule "forethought selbri connection" zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Product node for BO-grouped tanru unit; preserves `leading_unit`, `bo_connective`, `bo_tense_modal`, `bo`, and `trailing_unit` in source order.
    rule "BO-grouped tanru unit" bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        field leading_unit <- arc(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement));
        field bo_connective <- opt(arc(relation_afterthought_connective));
        field bo_tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    /// Product node for pro-bridi assignment; preserves `base` and `assignments` in source order.
    rule "pro-bridi assignment" assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field base <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        field assignments <- [one_or_more pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement)];
    }

    /// Product node for pro-bridi assignment; preserves `cei` and `tanru_unit` in source order.
    rule "pro-bridi assignment" pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field cei <- cmavo(Cei).wf();
        field tanru_unit <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Product node for tanru unit; preserves `base` and `linkargs` in source order.
    rule "tanru unit" linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        field base <- arc(tanru_unit_atom);
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    /// Product node for tanru unit; preserves `base` and `linkargs` in source order.
    rule "tanru unit" linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field base <- arc(tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    /// Product node for tanru unit; preserves `conversions` and `base` in source order.
    rule "tanru unit" tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field conversions <- [zero_or_more selmaho(Se).wf()];
        field base <- arc(tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Sum node for tanru unit; selects among 18 forms including `pro_bridi_tanru_unit`, `ordinal_tanru_unit`, and `word_tanru_unit`.
    rule "tanru unit" tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        /// Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.
        pro_bridi_tanru_unit,
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
        /// Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.
        grouped_tanru_unit,
    }

    /// Product node for tanru unit; preserves `conversions` and `base` in source order.
    rule "tanru unit" tanru_unit_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field conversions <- [zero_or_more selmaho(Se).wf()];
        field base <- arc(tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    /// Sum node for tanru unit; selects among 18 forms including `ordinal_tanru_unit`, `word_tanru_unit`, and `preposed_linkargs_tanru_unit`.
    rule "tanru unit" tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
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
        /// Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.
        grouped_tanru_unit,
    }

    /// Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.
    rule "tagged selbri" tagged_selbri_group_tanru_unit(tanru_unit, tense_modal, statement) -> struct {
        field tense_modal <- arc(tense_modal);
        field inner_selbri <- arc(connected_selbri(tanru_unit, statement));
    }

    /// Product node for linked arguments; preserves `linkargs` and `base` in source order.
    rule "linked arguments" preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal, statement) -> struct {
        field linkargs <- linkargs(sumti, tense_modal);
        field base <- arc(tanru_unit);
    }

    /// Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.
    rule "scalar-negated tanru unit" scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_unit <- arc(scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement));
    }

    /// Sum node for scalar-negated tanru unit; selects among the `tagged_selbri_group_tanru_unit`, `pro_bridi_tanru_unit`, and `tanru_unit_atom` forms.
    rule "scalar-negated tanru unit" scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> enum {
        /// Uses the `tagged_selbri_group_tanru_unit` product form, whose payload preserves `tense_modal` and `inner_selbri`.
        tagged_selbri_group_tanru_unit,
        /// Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.
        pro_bridi_tanru_unit,
        /// Uses the `tanru_unit_atom` product form, whose payload preserves `conversions` and `base`.
        tanru_unit_atom,
    }

    /// Product node for modal conversion; preserves `jai`, `tense_modal`, and `inner_unit` in source order.
    rule "modal conversion" jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal) -> struct {
        field jai <- cmavo(Jai).wf();
        field tense_modal <- opt(arc(tense_modal));
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Sum node for modal conversion; selects among 11 forms including `converted_jai_inner_tanru_unit`, `scalar_negated_jai_inner_tanru_unit`, and `sumti_selbri_tanru_unit`.
    rule "modal conversion" jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, letter_tokens, letter_string) -> enum {
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
        field se <- selmaho(Se).wf();
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.
    rule "scalar-negated tanru unit" scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    /// Transparent product node for quoted bridi selbri; preserves the `quote` component.
    rule "quoted bridi selbri" quoted_bridi_selbri_tanru_unit -> struct {
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    /// Product node for text selbri; preserves `luhei`, `text`, and `lihau` in source order.
    rule "text selbri" text_selbri_tanru_unit(text) -> struct {
        field luhei <- cmavo(Luhei).warn(ExperimentalZantufaLuheiSelbriUnit).wf();
        field text <- arc(text);
        field lihau <- opt(cmavo(Lihau).wf()).elidable_terminator(Lihau);
    }

    /// Transparent product node for quoted text selbri; preserves the `muhoi` component.
    rule "quoted text selbri" quoted_text_selbri_tanru_unit -> struct {
        field muhoi <- delimited_quote_marker(Muhoi).warn(ExperimentalZantufaMuhoiSelbriUnit).wf();
    }

    /// Product node for tag selbri; preserves `xohi` and `tag` in source order.
    rule "tag selbri" tag_selbri_tanru_unit(tense_modal) -> struct {
        field xohi <- cmavo(Xohi).warn(ExperimentalXohiTagSelbri).wf();
        field tag <- arc(tense_modal);
    }

    /// Product node for ordinal selbri; preserves `number` and `moi` in source order.
    rule "ordinal selbri" ordinal_tanru_unit(letter_tokens, letter_string) -> struct {
        field number <- number_or_letter_words(letter_tokens, letter_string);
        field moi <- selmaho(Moi).wf();
    }

    /// Transparent product node for tanru unit; preserves the `word` component.
    rule "tanru unit" word_tanru_unit -> struct {
        field word <- tanru_unit_relation_word().wf();
    }

    /// Transparent product node for tanru unit; preserves the `word` component.
    rule "tanru unit" goha_word_tanru_unit(free_modifier) -> struct {
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
        field goha <- selmaho(Goha).wf();
        field raho <- opt(cmavo(Raho).wf());
    }

    /// Product node for sumti-to-selbri; preserves `me`, `sumti`, `mehu`, and `moi_marker` in source order.
    rule "sumti-to-selbri" sumti_selbri_tanru_unit(sumti, letter_string) -> struct {
        field me <- cmavo(Me).wf();
        field sumti <- arc(sumti_selbri_sumti(sumti, letter_string));
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    /// Product node for sumti-to-selbri; preserves `me`, `body`, `mehu`, and `moi_marker` in source order.
    rule "sumti-to-selbri" zantufa_me_tanru_unit(mekso, mekso_operator, tense_modal) -> struct {
        field me <- cmavo(Me).warn(ExperimentalZantufaMex).wf();
        field body <- arc(zantufa_me_selbri_body(mekso, mekso_operator, tense_modal));
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
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
        field operators <- [one_or_more mekso_operator];
    }

    /// Transparent product node for sumti-to-selbri; preserves the `expression` component.
    rule "sumti-to-selbri" zantufa_me_mekso_selbri_body(mekso) -> struct {
        field expression <- arc(mekso);
    }

    /// Transparent product node for sumti-to-selbri; preserves the `tag` component.
    rule "sumti-to-selbri" zantufa_me_tag_selbri_body(tense_modal) -> struct {
        field tag <- arc(tense_modal);
    }

    /// Product node for mex selbri; preserves `expression` and `moi` in source order.
    rule "mex selbri" zantufa_mex_moi_tanru_unit(mekso) -> struct {
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_before_selmaho(Moi));
        field moi <- selmaho(Moi).warn(ExperimentalZantufaMex).wf();
    }

    /// Sum node for sumti selbri; selects among the `sumti` and `me_lerfu_sumti` forms.
    rule "sumti selbri" sumti_selbri_sumti(sumti, letter_string) -> enum {
        /// Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.
        sumti,
        /// Uses the `me_lerfu_sumti` product form, whose payload preserves `words`.
        me_lerfu_sumti,
    }

    /// Transparent product node for lerfu string; preserves the `words` component.
    rule "lerfu string" me_lerfu_sumti(letter_string) -> struct {
        field words <- letter_string;
    }

    /// Product node for operator-to-selbri; preserves `nuha` and `mekso_operator` in source order.
    rule "operator-to-selbri" operator_selbri_tanru_unit(mekso_operator) -> struct {
        field nuha <- cmavo(Nuha).wf();
        field mekso_operator <- arc(mekso_operator);
    }

    /// Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.
    rule "grouped tanru" grouped_tanru_unit(tanru_unit, statement) -> struct {
        field ke <- cmavo(Ke).wf();
        field selbri <- arc(connected_selbri(tanru_unit, statement));
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.
    rule "grouped tanru" grouped_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        field ke <- cmavo(Ke).wf();
        field selbri <- arc(connected_jai_inner_selbri(jai_inner_tanru_unit));
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    /// Product node for selbri connection; preserves `leading_selbri` and `continuations` in source order.
    rule "selbri connection" connected_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        field leading_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
        field continuations <- [zero_or_more connected_jai_inner_selbri_continuation(jai_inner_tanru_unit)];
    }

    /// Product node for selbri connection continuation; preserves `connective` and `trailing_selbri` in source order.
    rule "selbri connection continuation" connected_jai_inner_selbri_continuation(jai_inner_tanru_unit) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
    }

    /// Product node for selbri; preserves `first_unit` and `additional_units` in source order.
    rule "selbri" tanru_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        field first_unit <- jai_inner_tanru_unit;
        field additional_units <- [zero_or_more jai_inner_tanru_unit];
    }

    /// Sum node for linked arguments; selects among the `place_tagged_linked_sumti`, `tense_tagged_linked_sumti`, `plain_linked_sumti`, and `empty_linked_sumti` forms.
    rule "linked arguments" linked_sumti(sumti, tense_modal) -> enum {
        /// Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.
        place_tagged_linked_sumti,
        /// Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.
        tense_tagged_linked_sumti,
        /// Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.
        plain_linked_sumti,
        /// Uses the marker-only `empty_linked_sumti` product form.
        empty_linked_sumti,
    }

    /// Product node for linked arguments; preserves `fa` and `sumti` in source order.
    rule "linked arguments" place_tagged_linked_sumti(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Product node for linked arguments; preserves `tense_modal` and `sumti` in source order.
    rule "linked arguments" tense_tagged_linked_sumti(sumti, tense_modal) -> struct {
        field tense_modal <- arc(tense_modal);
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    /// Transparent product node for linked arguments; preserves the `sumti` component.
    rule "linked arguments" plain_linked_sumti(sumti) -> struct {
        field sumti <- arc(sumti);
    }

    /// Marker-only product node for linked arguments; the parser retains no public fields.
    rule "linked arguments" empty_linked_sumti -> struct {
    }

    /// Product node for linked arguments; preserves `bei` and `link` in source order.
    rule "linked arguments" bei_link(sumti, tense_modal) -> struct {
        field bei <- cmavo(Bei).wf();
        field link <- linked_sumti(sumti, tense_modal);
    }

    /// Product node for linked arguments; preserves `be`, `first_link`, `bei_links`, and `beho` in source order.
    rule "linked arguments" linkargs(sumti, tense_modal) -> struct {
        field be <- cmavo(Be).wf();
        field first_link <- linked_sumti(sumti, tense_modal);
        field bei_links <- [zero_or_more bei_link(sumti, tense_modal)];
        field beho <- opt(cmavo(Beho).wf()).elidable_terminator(Beho);
    }

    /// Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei` in source order.
    rule "abstraction" abstraction_tanru_unit(subbridi) -> struct {
        field nu <- selmaho(Nu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field abstractor_connections <- [zero_or_more abstractor_connection()];
        field subbridi <- arc(subbridi);
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

    /// Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.
    rule "abstractor connection" abstractor_connection -> struct {
        field connective <- standard_statement_connective;
        field nu <- selmaho(Nu).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    /// Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei` in source order.
    rule "abstraction" zantufa_statement_abstraction_tanru_unit(statement) -> struct {
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
        field nai <- opt(cmavo(Nai).wf());
        field abstractor_connections <- [zero_or_more zantufa_abstractor_connection()];
        field statement <- arc(statement);
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

    /// Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.
    rule "abstractor connection" zantufa_abstractor_connection -> struct {
        field connective <- joik_connective;
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
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
