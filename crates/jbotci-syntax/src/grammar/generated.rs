//! Declarative generated syntax parser.

#![allow(dead_code)]

use chumsky::span::SimpleSpan;
use chumsky::{Parser, input::Input, primitive::end, recursive::Recursive};
use jbotci_morphology::{Cmavo, Selmaho};

use super::ast::*;
use super::generated_runtime;
use super::tokens::{
    cmavo, cmevla_word, pa_word, relation_word, selmaho, spanned_tokens, syntax_error,
};
use super::{BoxedParser, ParseExtra, ParserInput, ParserState};
use crate::{ExperimentalConstruct, ParseOptions, SyntaxWordCategory, Token};

macro_rules! declare_generated_syntax_grammar {
    ($($prefix:tt)*) => {
        jbotci_syntax_macros::syntax_grammar! {
            $($prefix)*

    recursive {
        text: TextSyntax;
        paragraph: ParagraphSyntax;
        paragraph_statement: ParagraphStatementSyntax;
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
        sumti_base: SumtiSyntax;
        selbri: SelbriSyntax;
        co_selbri: SelbriSyntax;
        tanru_unit: TanruUnitSyntax;
        bo_or_linked_tanru_unit: TanruUnitSyntax;
        tanru_unit_atom: TanruUnitSyntax;
        jai_inner_tanru_unit: TanruUnitSyntax;
        tense_modal: TenseModalSyntax;
        mekso: MeksoSyntax;
        mekso_base: MeksoBaseSyntax;
        mekso_precedence: MeksoPrecedenceSyntax;
        mekso_operand: MeksoOperandSyntax;
        mekso_operator: MeksoOperatorSyntax;
        reverse_polish_parts: ReversePolishPartsSyntax;
        letter_string: vec1::Vec1<Token>;
        letter_tokens: vec1::Vec1<Token>;
        free_modifier: FreeModifierSyntax;
    }

    rule "leading indicator" leading_indicator -> struct {
        field indicator <- choice((selmaho(Ui), selmaho(Cai)));
        field nai <- opt(cmavo(Nai));
    }

    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> enum {
        explicit_xauha_lohoi_text,
        regular_text,
    }

    rule "text" explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier) -> struct {
        assert (
            cmavo(Xauha).ignored(),
            raw_words_until(Kuhau).ignored(),
            cmavo(Kuhau).ignored(),
        ).ignored();
        field paragraphs <- text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
    }

    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> struct {
        field leading_nai <- [zero_or_more cmavo(Nai)];
        field leading_cmevla <- [zero_or_more text_leading_cmevla_word()];
        field leading_indicators <- [zero_or_more leading_indicator()];
        field leading_free_modifiers <- [zero_or_more free_modifier];
        field leading_connective <- opt(guard_not(
            modal_forethought_connective(tense_modal),
            choice((
                standard_statement_connective,
                cehe_connective(),
            )),
        ));
        field leading_i_statements <- [zero_or_more leading_i_statement(free_modifier, tense_modal)];
        #[tree_child(primary)]
        field paragraphs <- opt(boxed(text_paragraphs(
            paragraph,
            statement_or_fragment,
            free_modifier,
        )));
    }

    rule "paragraphs" text_paragraphs(paragraph, statement_or_fragment, free_modifier) -> enum {
        text_paragraph_with_additional_niho,
        text_niho_paragraphs,
    }

    rule "paragraphs" text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        field first <- paragraph;
        field additional_niho <- [zero_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    rule "paragraphs" text_niho_paragraphs(statement_or_fragment, free_modifier) -> struct {
        field paragraphs <- [one_or_more niho_paragraph(statement_or_fragment, free_modifier)];
    }

    rule "paragraph statement" leading_i_statement(free_modifier, tense_modal) -> struct {
        field i <- cmavo(I);
        field connective <- opt(boxed(choice((
            i_standard_paragraph_statement_connective(tense_modal),
            i_tag_bo_paragraph_statement_connective(tense_modal),
        ))));
        field free_modifiers <- [zero_or_more free_modifier];
    }

    rule "paragraph" paragraph(statement_or_fragment, free_modifier) -> enum {
        i_niho_paragraph,
        simple_paragraph,
    }

    rule "paragraph" simple_paragraph(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        field statements <- paragraph_statement_sequence(statement_or_fragment, free_modifier);
    }

    rule "paragraph statement sequence" paragraph_statement_sequence(statement_or_fragment, free_modifier) -> struct {
        #[tree_child(primary)]
        field initial <- initial_paragraph_statement(statement_or_fragment);
        field following <- [zero_or_more following_paragraph_statement(statement_or_fragment, free_modifier)];
        field trailing <- [zero_or_more trailing_ijek_paragraph_statement()];
    }

    rule "paragraph" i_niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        field i <- some(cmavo(I));
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(boxed(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    rule "paragraph" niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(boxed(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    rule "paragraph statement" initial_paragraph_statement(statement_or_fragment) -> struct {
        #[tree_child(primary)]
        field statement <- some(boxed(statement_or_fragment));
    }

    rule "paragraph statement" following_paragraph_statement(statement_or_fragment, free_modifier) -> struct {
        field i <- some(cmavo(I));
        assert !statement_connective;
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statement <- opt(boxed(statement_or_fragment));
    }

    rule "paragraph statement" trailing_ijek_paragraph_statement -> struct {
        field i <- cmavo(I);
        field connective <- statement_connective;
    }

    rule "statement" statement(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        i_statement_connection,
        preposed_i_statement_connection,
        statement_base,
    }

    rule "statement" statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> enum {
        prenex_statement,
        bridi_statement,
        text_group_statement,
    }

    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        statement_or_fragment_statement,
        fragment_statement,
    }

    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        field statement <- statement;
    }

    rule "fragment" fragment_statement(term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        prenex_fragment,
        selbri_fragment,
        ek_fragment,
        gihek_fragment,
        multiple_na_fragment,
        single_na_fragment,
        terms_fragment,
        mekso_fragment,
        relative_clause_fragment,
        linked_sumti_continuation_fragment,
        linked_sumti_fragment,
    }

    rule "statement" statement_after_i_connective(bridi, subbridi, tense_modal, text) -> enum {
        bridi_statement,
        text_group_statement,
    }

    rule "fragment" multiple_na_fragment -> struct {
        field first_na <- selmaho(Na);
        field second_na <- selmaho(Na);
        field additional_na <- [zero_or_more selmaho(Na)];
    }

    rule "fragment" single_na_fragment -> struct {
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
    }

    rule "fragment" ek_fragment -> struct {
        #[tree_child(primary)]
        field connective <- ek_connective();
    }

    rule "fragment" gihek_fragment -> struct {
        #[tree_child(primary)]
        field connective <- gihek_connective();
    }

    rule "statement connection" i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field leading_statement <- boxed(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        field continuations <- [one_or_more i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens)];
    }

    rule "statement connective" pending_i_connective -> struct {
        field i <- cmavo(I);
        field connective <- statement_connective;
        assert cmavo(I);
    }

    rule "statement connection" i_statement_connection_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> enum {
        chained_i_connective_statement_tail,
        simple_i_connective_statement_tail,
    }

    rule "statement connection" chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field pending <- [one_or_more pending_i_connective];
        field i <- cmavo(I);
        field connective <- i_statement_connective(tense_modal);
        field trailing_statement <- boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
    }

    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field i <- cmavo(I);
        field connective <- i_statement_connective(tense_modal);
        field trailing_statement <- boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
    }

    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> struct {
        field leading_statement <- boxed(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        field connective <- statement_connective;
        field i <- cmavo(I);
        field trailing_statement <- boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
    }

    rule "text group" text_group_statement(text, tense_modal) -> struct {
        field tense_modal <- opt(boxed(tense_modal));
        field tuhe <- cmavo(Tuhe).wf();
        #[tree_child(primary)]
        field text <- boxed(text);
        field tuhu <- opt(cmavo(Tuhu).wf());
    }

    rule "prenex" prenex_fragment(term) -> struct {
        field terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
    }

    rule "prenex" prenex_statement(statement, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        #[tree_child(primary)]
        field inner_statement <- boxed(statement);
    }

    rule "statement" bridi_statement(bridi, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        field bridi <- boxed(bridi);
        field continuations <- [zero_or_more bridi_statement_continuation(subbridi, tense_modal)];
    }

    rule "bridi continuation" bridi_statement_continuation(subbridi, tense_modal) -> enum {
        bo_bridi_statement_continuation,
        ke_bridi_statement_continuation,
    }

    rule "bridi continuation" bo_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_subbridi <- boxed(subbridi);
    }

    rule "bridi continuation" ke_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        field connective <- relation_afterthought_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field trailing_subbridi <- boxed(subbridi);
        field kehe <- opt(cmavo(Kehe).wf());
    }

    rule "selbri" selbri_fragment(selbri) -> struct {
        #[tree_child(primary)]
        field selbri <- boxed(selbri);
    }

    rule "terms" terms_fragment(term) -> struct {
        #[tree_child(primary)]
        field terms <- [one_or_more term];
        field vau <- opt(cmavo(Vau).wf());
    }

    rule "mex" mekso_fragment(mekso, letter_tokens) -> struct {
        #[tree_child(primary)]
        field quantifier <- boxed(quantifier(mekso, letter_tokens));
    }

    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal) -> struct {
        field first <- relative_clause_atom(sumti, subbridi, tense_modal);
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal)];
    }

    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal);
    }

    rule "linked arguments" linked_sumti_continuation_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        field bei_links <- [one_or_more bei_link(sumti, tense_modal)];
    }

    rule "linked arguments" linked_sumti_fragment(sumti, tense_modal) -> struct {
        #[tree_child(primary)]
        field linkargs <- linkargs(sumti, tense_modal);
    }

    rule "bridi" bridi(term, selbri, subbridi, tense_modal, bridi_tail) -> enum {
        bridi_with_leading_terms,
        bridi_with_post_cu_terms,
        bare_cu_bridi,
        bare_cu_terms_bridi,
        relation_only_bridi,
    }

    rule "bridi" bridi_with_leading_terms(term, bridi_tail) -> struct {
        field leading_terms <- [one_or_more term];
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bridi_tail);
    }

    rule "bridi" bridi_with_post_cu_terms(term, bridi_tail) -> struct {
        field leading_terms <- [one_or_more term];
        field cu <- some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
        field bridi_tail <- boxed(cu_terms_bridi_tail(term, bridi_tail));
    }

    rule "bridi" bare_cu_bridi(bridi_tail) -> struct {
        field cu <- some(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bridi_tail);
    }

    rule "bridi" bare_cu_terms_bridi(term, bridi_tail) -> struct {
        field cu <- some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
        field bridi_tail <- boxed(cu_terms_bridi_tail(term, bridi_tail));
    }

    rule "bridi" relation_only_bridi(bridi_tail) -> struct {
        field bridi_tail <- boxed(bridi_tail);
    }

    rule "bridi tail" cu_terms_bridi_tail(term, bridi_tail) -> struct {
        field terms <- [one_or_more term];
        field bridi_tail <- boxed(bridi_tail);
    }

    rule "bridi tail" bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        bridi_tail_with_possible_tail_terms,
        bridi_tail_without_tail_terms,
    }

    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        field ke_continuation <- opt(boxed(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        assert !(relation_connective_as_bridi_tail, opt(boxed(tense_modal)), cmavo(Ke));
        field ke_continuation <- opt(boxed(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    rule "bridi tail" afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(bo_grouped_bridi_tail_without_tail_terms);
        field continuations <- [zero_or_more bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)];
    }

    rule "bridi tail" afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(bo_grouped_bridi_tail);
        field continuations <- [zero_or_more bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal)];
    }

    rule "bridi tail" bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        field bo_continuation <- opt(boxed(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        field first <- boxed(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
        field bo_continuation <- opt(boxed(bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal)));
    }

    rule "bridi tail" simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        forethought_simple_bridi_tail_without_tail_terms,
        selbri_simple_bridi_tail_without_tail_terms,
    }

    rule "bridi tail" simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> enum {
        forethought_simple_bridi_tail,
        selbri_simple_bridi_tail,
    }

    rule "forethought bridi connection" forethought_simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        field connection <- boxed(forethought_bridi_connection_without_tail_terms);
    }

    rule "forethought bridi connection" forethought_simple_bridi_tail(forethought_bridi_connection) -> struct {
        field connection <- boxed(forethought_bridi_connection);
    }

    rule "bridi tail" selbri_simple_bridi_tail_without_tail_terms(selbri) -> struct {
        field selbri <- boxed(selbri);
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "bridi tail" selbri_simple_bridi_tail(selbri, term) -> struct {
        field selbri <- boxed(selbri);
        field terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "forethought bridi connection" forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal) -> enum {
        direct_forethought_bridi_connection,
        grouped_forethought_bridi_connection,
        negated_forethought_bridi_connection,
    }

    rule "forethought bridi connection" forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal) -> enum {
        direct_forethought_bridi_connection_without_tail_terms,
        grouped_forethought_bridi_connection_without_tail_terms,
        negated_forethought_bridi_connection_without_tail_terms,
    }

    rule "forethought bridi connection" direct_forethought_bridi_connection(subbridi, term, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- boxed(subbridi);
        field gik <- gik_connective;
        field second <- boxed(subbridi);
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- boxed(subbridi);
        field gik <- gik_connective;
        field second <- boxed(subbridi);
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "forethought bridi connection" grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal) -> struct {
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- boxed(forethought_bridi_connection);
        field kehe <- opt(arc(cmavo(Kehe).wf()));
    }

    rule "forethought bridi connection" grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal) -> struct {
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- boxed(forethought_bridi_connection_without_tail_terms);
        field kehe <- opt(arc(cmavo(Kehe).wf()));
    }

    rule "forethought bridi connection" negated_forethought_bridi_connection(forethought_bridi_connection) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- boxed(forethought_bridi_connection);
    }

    rule "forethought bridi connection" negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- boxed(forethought_bridi_connection_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- boxed(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf()));
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- gihek_connective();
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- boxed(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf()));
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bo_grouped_bridi_tail_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bo_grouped_bridi_tail_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- boxed(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf()));
    }

    rule "subbridi" subbridi(subbridi, bridi, term) -> enum {
        prenex_subbridi,
        bridi_subbridi,
    }

    rule "subbridi" bridi_subbridi(bridi) -> struct {
        field bridi <- boxed(bridi);
    }

    rule "prenex" prenex_subbridi(subbridi, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        field inner_subbridi <- boxed(subbridi);
    }

    alias "term" term_guard =
        guard_not((relation_word(), cmavo(Bu).not()), empty());

    rule "term" term(term, sumti, tense_modal, subbridi, selbri) -> enum {
        pehe_termset_connection,
        bound_term_connection,
        termset_group,
        connected_term,
        simple_term,
    }

    rule "termset connection" pehe_termset_connection(sumti, tense_modal, subbridi, selbri, term) -> struct {
        assert term_guard();
        field leading_term <- boxed(pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term));
        field continuations <- [one_or_more (cmavo(Pehe).wf(), statement_connective, boxed(pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term)))];
    }

    rule "term" pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term) -> enum {
        bound_term_connection,
        termset_group,
        simple_term,
    }

    rule "term" simple_term(sumti, tense_modal, subbridi, selbri, term) -> enum {
        place_tagged_sumti_term,
        jai_tagged_sumti_term,
        tagged_sumti_before_tag_term,
        tagged_sumti_term,
        noiha_adverbial_term,
        fihoi_adverbial_term,
        soi_adverbial_term,
        na_ku_term,
        sumti_term,
        bare_na_term,
        forethought_termset,
        nuhi_termset,
        ke_termset,
    }

    rule "term connection" bound_term_connection(sumti, tense_modal, subbridi, selbri, term) -> struct {
        assert term_guard();
        field leading_term <- boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
        field connective <- boxed(choice((
            joik_connective(),
            ek_connective(),
        )));
        field bo <- cmavo(Bo).wf();
        assert term_hierarchy_post_bo_argument_gate(sumti);
        field trailing_term <- boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
        assert term_hierarchy_post_bo_argument_gate(sumti);
    }

    alias "term connection" term_hierarchy_post_bo_argument_gate(sumti) =
        choice((
            feature(TermHierarchy, empty()),
            (
                feature(TermHierarchy, empty()).not(),
                sumti.not(),
            ).ignored(),
        ));

    rule "term connection" connected_term(sumti, tense_modal, subbridi, selbri, term) -> struct {
        assert term_guard();
        field leading_term <- boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
        field continuations <- [zero_or_more (choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        )), boxed(simple_term(sumti, tense_modal, subbridi, selbri, term)))];
    }

    rule "termset" termset_group(sumti, tense_modal, subbridi, selbri, term) -> struct {
        assert term_guard();
        field leading_term <- boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
        field continuations <- [one_or_more (cmavo(Cehe).wf(), boxed(simple_term(sumti, tense_modal, subbridi, selbri, term)))];
    }

    rule "termset" forethought_termset(term, tense_modal) -> struct {
        field m_nuhi <- opt(cmavo(Nuhi).wf());
        field gek <- modal_forethought_connective(tense_modal);
        field terms <- [one_or_more boxed(term)];
        field nuhu <- opt(cmavo(Nuhu).wf());
        field gik <- gik_connective;
        field gik_terms <- [one_or_more boxed(term)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        field gik_nuhu <- opt(cmavo(Nuhu).wf());
    }

    rule "termset" nuhi_termset(term) -> struct {
        field nuhi <- cmavo(Nuhi).wf();
        field termset <- [one_or_more boxed(term)];
        field nuhu <- opt(cmavo(Nuhu).wf());
    }

    rule "termset" ke_termset(term) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalKeTermset).wf();
        field termset <- [one_or_more boxed(term)];
        field kehe <- opt(cmavo(Kehe).wf());
    }

    rule "NOIhA adverbial" noiha_adverbial_term(selbri) -> enum {
        noiha_variable_adverbial_term,
        noiha_relative_adverbial_term,
    }

    rule "NOIhA adverbial" noiha_variable_adverbial_term(selbri) -> struct {
        field poiha <- selmaho(Noiha).wf();
        field selbri <- some(boxed(selbri));
        field brigahi_ku <- cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
    }

    rule "NOIhA adverbial" noiha_relative_adverbial_term(selbri) -> struct {
        field noiha <- selmaho(Noiha).wf();
        field selbri <- some(boxed(selbri));
        field fehu <- opt(cmavo(Fehu).wf());
    }

    rule "FIhOI adverbial" fihoi_adverbial_term(subbridi) -> struct {
        field fihoi <- cmavo(Fihoi).wf();
        field subbridi <- boxed(subbridi);
        field fihau <- opt(cmavo(Fihau).wf());
    }

    rule "SOI adverbial" soi_adverbial_term(subbridi) -> struct {
        field soi <- selmaho(Soi).wf();
        field subbridi <- boxed(subbridi);
        field sehu <- opt(cmavo(Sehu).wf());
    }

    rule "term" sumti_term(sumti) -> struct {
        field sumti <- boxed(sumti);
    }

    rule "place tag" place_tagged_sumti_term(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- boxed(tagged_or_elided_sumti(sumti));
    }

    rule "NA KU term" na_ku_term -> struct {
        field na <- selmaho(Na);
        field na_ku <- cmavo(Ku).wf();
    }

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

    rule "tag" tagged_sumti_before_tag_term(tense_modal, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        field tense_modal <- boxed(leading_term_tag_tense_modal(tense_modal, selbri));
        assert tense_modal.lookahead();
    }

    rule "tag" tagged_sumti_term(tense_modal, sumti, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        field tense_modal <- some(boxed(leading_term_tag_tense_modal(tense_modal, selbri)));
        assert !selbri;
        field sumti <- boxed(tagged_or_elided_sumti(sumti));
    }

    rule "tag" jai_tagged_sumti_term(tense_modal, sumti) -> struct {
        assert feature(ZantufaTags, empty());
        field jai <- cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
        field tag <- opt(boxed(tense_modal));
        field sumti <- boxed(sumti);
    }

    rule "tag" leading_term_tag_tense_modal(tense_modal, selbri) -> enum {
        pu_before_nahe_leading_term_tag_tense,
        pu_distance_before_tag_leading_term_tag_tense,
        zi_before_zi_leading_term_tag_tense,
        va_before_va_leading_term_tag_tense,
        mohi_before_mohi_leading_term_tag_tense,
        caha_before_tag_leading_term_tag_tense,
        interval_property_leading_term_tag_tense,
        tense_modal,
    }

    rule "tag" pu_before_nahe_leading_term_tag_tense -> struct {
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        assert selmaho(Nahe);
    }

    rule "tag" pu_distance_before_tag_leading_term_tag_tense -> struct {
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    rule "tag" zi_before_zi_leading_term_tag_tense -> struct {
        field zi <- selmaho(Zi).wf();
        assert selmaho(Zi);
    }

    rule "tag" va_before_va_leading_term_tag_tense -> struct {
        field va <- selmaho(Va).wf();
        assert selmaho(Va);
    }

    rule "tag" mohi_before_mohi_leading_term_tag_tense -> struct {
        field mohi <- selmaho(Mohi).wf();
        field direction <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Va).wf());
        assert selmaho(Mohi);
    }

    rule "tag" caha_before_tag_leading_term_tag_tense(tense_modal) -> struct {
        field caha <- selmaho(Caha).wf().followed_by(tense_modal.lookahead());
    }

    rule "interval property" interval_property_leading_term_tag_tense(selbri) -> struct {
        field property <- boxed(interval_property_tense().followed_by(choice((
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

    rule "sumti" tagged_or_elided_sumti(sumti) -> enum {
        sumti,
        tagged_elided_sumti,
    }

    rule "elided sumti" tagged_elided_sumti -> struct {
        field maybe_ku <- opt(cmavo(Ku).wf());
    }

    rule "sumti" sumti(sumti, sumti_grouped, subbridi, tense_modal) -> struct {
        field base_sumti <- boxed(sumti_grouped);
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal));
    }

    rule "sumti connection" sumti_grouped(sumti, sumti_afterthought, tense_modal) -> struct {
        field leading_sumti <- boxed(sumti_afterthought);
        field grouped_tail <- opt(grouped_sumti_tail(sumti, tense_modal));
    }

    rule "sumti connection" sumti_afterthought(sumti_bound) -> struct {
        field leading_sumti <- boxed(sumti_bound);
        field continuations <- [zero_or_more sumti_afterthought_tail(sumti_bound)];
    }

    rule "sumti connection" sumti_bound(sumti_bound, sumti_forethought, tense_modal) -> struct {
        field leading_sumti <- boxed(sumti_forethought);
        field bound_tail <- opt(bound_sumti_tail(sumti_bound, tense_modal));
    }

    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> enum {
        forethought_sumti,
        simple_sumti,
    }

    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field leading_sumti <- boxed(sumti);
        field gik <- gik_connective;
        field trailing_sumti <- boxed(sumti_forethought);
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
    }

    rule "sumti connection" bound_sumti_tail(sumti_bound, tense_modal) -> struct {
        field connective <- boxed(argument_connective);
        field tense_modal <- opt(boxed(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_sumti <- boxed(sumti_bound);
    }

    rule "sumti connective" sumti_afterthought_tail(sumti_bound) -> struct {
        field connective <- argument_connective;
        field sumti <- boxed(sumti_bound);
    }

    rule "sumti connection" grouped_sumti_tail(sumti, tense_modal) -> struct {
        field connective <- argument_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner_sumti <- boxed(sumti);
        field kehe <- opt(cmavo(Kehe).wf());
    }

    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal) -> enum {
        vuho_relative_sumti_attachment_tail,
        vuho_connected_sumti_attachment_tail,
    }

    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal);
        field sumti_connection <- opt(boxed(sumti_connection_tail(sumti)));
    }

    rule "sumti relative phrase" vuho_connected_sumti_attachment_tail(sumti) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field sumti_connection <- some(boxed(sumti_connection_tail(sumti)));
    }

    rule "sumti" simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> struct {
        field base_sumti <- boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
        field relative_clauses <- opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
    }

    rule "sumti" sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> enum {
        sumti_base,
        quantified_sumti,
    }

    alias "sumti" sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier) =
        choice((
            scalar_negated_sumti_with_bo(sumti),
            scalar_negated_sumti(sumti),
            lahe_sumti(sumti, subbridi, tense_modal),
            lahe_term_wrapper(term),
            scalar_negated_term_wrapper_with_bo(term),
            scalar_negated_term_wrapper(term),
            bridi_description_sumti(subbridi),
            name_sumti(),
            description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens),
            number_sumti(mekso),
            lerfu_string_sumti(letter_string, free_modifier),
            quoted_sumti(text),
            pro_sumti(),
        ));

    rule "quantified sumti" quantified_sumti(sumti_base, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        field inner_sumti <- boxed(sumti_base);
    }

    rule "sumti relative phrase" sumti_with_relative_clauses(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> struct {
        field base_sumti <- boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal);
    }

    rule "sumti connective" sumti_connection_tail(sumti) -> struct {
        field connective <- argument_connective;
        field sumti <- boxed(sumti);
    }

    rule "quantifier" pa_run_quantifier(letter_tokens) -> struct {
        field number <- number_words(letter_tokens).wf();
        field boi <- opt(cmavo(Boi).wf());
    }

    rule "quantifier" mekso_quantifier(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field mekso <- boxed(mekso);
        field veho <- opt(cmavo(Veho).wf());
    }

    rule "quantifier" quantifier(mekso, letter_tokens) -> enum {
        mekso_quantifier,
        pa_run_quantifier,
    }

    rule "number mex" number_mekso(letter_tokens) -> struct {
        field quantifier <- boxed(pa_run_quantifier(letter_tokens));
    }

    rule "operator" primitive_mekso_operator -> struct {
        field vuhu <- selmaho(Vuhu).wf();
    }

    rule "operator" mekso_operator(mekso, mekso_operator, selbri) -> enum {
        afterthought_mekso_operator,
        bound_mekso_operator,
        simple_mekso_operator,
    }

    rule "operator" afterthought_mekso_operator(mekso, mekso_operator, selbri) -> struct {
        field leading_operator <- boxed(bound_or_atom_mekso_operator(mekso, mekso_operator, selbri));
        field continuations <- [zero_or_more (standard_statement_connective, boxed(bound_or_atom_mekso_operator(mekso, mekso_operator, selbri)))];
    }

    rule "operator" bound_or_atom_mekso_operator(mekso, mekso_operator, selbri) -> enum {
        bound_mekso_operator,
        simple_mekso_operator,
    }

    rule "operator" bound_mekso_operator(mekso, mekso_operator, selbri) -> struct {
        field left_operator <- boxed(simple_mekso_operator(mekso, mekso_operator, selbri));
        field connective <- standard_statement_connective;
        field bo <- cmavo(Bo).wf();
        field right_operator <- boxed(mekso_operator);
    }

    rule "operator" simple_mekso_operator(mekso, mekso_operator, selbri) -> enum {
        converted_mekso_operator,
        scalar_negated_mekso_operator,
        forethought_mekso_operator,
        grouped_mekso_operator,
        selbri_mekso_operator,
        operand_mekso_operator,
        primitive_mekso_operator,
    }

    rule "converted operator" converted_mekso_operator(mekso_operator) -> struct {
        field se <- selmaho(Se).wf();
        field inner_operator <- boxed(mekso_operator);
    }

    rule "converted operator" scalar_negated_mekso_operator(mekso_operator) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_operator <- boxed(mekso_operator);
    }

    rule "operator" forethought_mekso_operator(mekso_operator) -> struct {
        field guhek <- guhek_connective;
        field left_operator <- boxed(mekso_operator);
        field gik <- gik_connective;
        field right_operator <- boxed(mekso_operator);
    }

    rule "grouped operator" grouped_mekso_operator(mekso_operator) -> struct {
        field ke <- cmavo(Ke).wf();
        field inner_operator <- boxed(mekso_operator);
        field kehe <- opt(cmavo(Kehe).wf());
    }

    rule "selbri-to-operator" selbri_mekso_operator(selbri) -> struct {
        field nahu <- cmavo(Nahu).wf();
        field selbri <- boxed(selbri);
        field tehu <- opt(cmavo(Tehu).wf());
    }

    rule "operand-to-operator" operand_mekso_operator(mekso) -> struct {
        field maho <- cmavo(Maho).wf();
        field mekso <- boxed(mekso);
        field tehu <- opt(cmavo(Tehu).wf());
    }

    rule "operand" mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        afterthought_mekso_operand,
        bound_mekso_operand,
        simple_mekso_operand,
    }

    rule "operand connective" afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field leading_expression <- boxed(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
        field continuations <- [zero_or_more (operand_connective, boxed(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier)))];
    }

    rule "operand" bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        bound_mekso_operand,
        simple_mekso_operand,
    }

    rule "operand connective" bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field left_expression <- boxed(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
        field operand_connective <- operand_connective;
        field tense_modal <- opt(boxed(tense_modal));
        field bo <- cmavo(Bo).wf();
        field right_expression <- boxed(mekso_operand);
    }

    rule "operand" simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        forethought_mekso_operand,
        qualified_mekso_operand,
        parenthesized_mekso_operand,
        sumti_mekso_operand,
        selbri_mekso_operand,
        array_mekso_operand,
        number_mekso,
        lerfu_string_mekso,
    }

    rule "qualified operand" qualified_mekso_operand(mekso_operand) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo);
        field inner_expression <- boxed(mekso_operand);
        field luhu <- opt(cmavo(Luhu).wf());
    }

    rule "forethought mex" forethought_mekso_operand(mekso_operand, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field left_expression <- boxed(mekso_operand);
        field gik <- gik_connective;
        field right_expression <- boxed(mekso_operand);
    }

    rule "sumti operand" sumti_mekso_operand(sumti) -> struct {
        field mohe <- cmavo(Mohe).wf();
        field sumti <- boxed(sumti);
        field tehu <- opt(cmavo(Tehu).wf());
    }

    rule "selbri operand" selbri_mekso_operand(selbri) -> struct {
        field nihe <- cmavo(Nihe).wf();
        field selbri <- boxed(selbri);
        field tehu <- opt(cmavo(Tehu).wf());
    }

    rule "parenthesized mex" parenthesized_mekso_operand(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field inner_expression <- boxed(mekso);
        field veho <- opt(cmavo(Veho).wf());
    }

    rule "mekso array" array_mekso_operand(mekso) -> struct {
        field johi <- cmavo(Johi).wf();
        field expressions <- [one_or_more mekso];
        field tehu <- opt(cmavo(Tehu).wf());
    }

    alias "lerfu string" letter_string(letter_tokens) =
        [..letter_tokens; zero_or_more ..choice((
            [pa_word()],
            letter_tokens,
        ))];

    alias "number" number_words(letter_tokens) =
        [..[pa_word()]; zero_or_more ..choice((
            [pa_word()],
            letter_tokens,
        ))];

    alias "number or lerfu string" number_or_letter_words(letter_tokens, letter_string) =
        choice((
            number_words(letter_tokens),
            letter_string,
        ));

    alias "lerfu word" letter_tokens(letter_string, letter_tokens) =
        choice((
            [word_category(LetterWord)],
            lau_letter_tokens(letter_tokens),
            tei_letter_tokens(letter_string),
        ));

    alias "lerfu word" lau_letter_tokens(letter_tokens) = [selmaho(Lau); ..letter_tokens];

    alias "lerfu word" tei_letter_tokens(letter_string) =
        [cmavo(Tei); ..letter_string; cmavo(Foi)];

    rule "lerfu string" lerfu_string_mekso(letter_string, free_modifier) -> struct {
        field letters <- letter_string;
        field boi <- opt(cmavo(Boi));
        field free_modifiers <- [zero_or_more free_modifier];
    }

    rule "mex" mekso_base(mekso, mekso_base, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator) -> enum {
        mekso_operand,
        forethought_call_mekso,
    }

    rule "mex" mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field left_expression <- boxed(mekso_base);
        field tail <- opt((cmavo(Bihe).wf(), boxed(mekso_operator), boxed(mekso_precedence)));
    }

    rule "mex" infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field first_expression <- boxed(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        field continuations <- [zero_or_more (boxed(mekso_operator), boxed(mekso_precedence))];
    }

    rule "forethought mex" forethought_call_mekso(mekso_base, mekso_operator) -> struct {
        field peho <- opt(cmavo(Peho).wf());
        field operator <- boxed(mekso_operator);
        field operands <- [one_or_more mekso_base];
        field kuhe <- opt(cmavo(Kuhe).wf());
    }

    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts) -> enum {
        infix_mekso,
        reverse_polish_mekso,
    }

    rule "reverse Polish mex" reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> struct {
        field first_operand <- boxed(mekso_operand);
        field tails <- [zero_or_more (boxed(reverse_polish_parts), mekso_operator)];
    }

    rule "reverse Polish mex" reverse_polish_mekso(reverse_polish_parts) -> struct {
        field fuha <- cmavo(Fuha).wf();
        field parts <- boxed(reverse_polish_parts);
    }

    node number_sumti(mekso) -> SumtiSyntax {
        context "number sumti";
        construct variant NumberSumti;
        fields {
            field li = selmaho(Li).wf();
            field expression = boxed(mekso);
            field loho = opt(cmavo(Loho).wf());
        }
    }

    node lerfu_string_sumti(letter_string, free_modifier) -> SumtiSyntax {
        context "lerfu string";
        fields {
            field words = letter_string;
            require selmaho(Moi).not();
            require selmaho(Mai).not();
            field boi = opt(cmavo(Boi));
            field free_modifiers = many(free_modifier);
        }
    }

    node lahe_sumti(sumti, subbridi, tense_modal) -> SumtiSyntax {
        context "converted sumti";
        construct variant ReferentSumti;
        fields {
            field lahe = selmaho(Lahe).wf();
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field inner_sumti = boxed(sumti);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node lahe_term_wrapper(term) -> SumtiSyntax {
        context "converted term";
        construct variant ReferentTermWrapper;
        fields {
            field lahe = selmaho(Lahe).wf();
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper_with_bo(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant ScalarNegatedTermWrapperWithBo;
        fields {
            field nahe = selmaho(Nahe);
            field bo = cmavo(Bo).wf();
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant ScalarNegatedTermWrapper;
        fields {
            field nahe = selmaho(Nahe).wf();
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_sumti_with_bo(sumti) -> SumtiSyntax {
        context "scalar-negated sumti";
        construct variant ScalarNegatedSumtiWithBo;
        fields {
            field nahe = selmaho(Nahe);
            field bo = cmavo(Bo).wf();
            field inner_sumti = boxed(sumti);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_sumti(sumti) -> SumtiSyntax {
        context "scalar-negated sumti";
        construct variant ScalarNegatedSumti;
        fields {
            field nahe = selmaho(Nahe).wf();
            field inner_sumti = boxed(sumti);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node bridi_description_sumti(subbridi) -> SumtiSyntax {
        context "bridi description";
        construct variant BridiDescription;
        fields {
            field lohoi = selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
            field subbridi = boxed(subbridi);
            field kuhau = opt(cmavo(Kuhau).wf());
        }
    }

    node pro_sumti -> SumtiSyntax {
        context "sumti";
        construct tuple_variant ProSumti;
        fields {
            field koha = word_category(ProSumti).wf();
        }
    }

    node name_sumti -> SumtiSyntax {
        context "name";
        fields {
            field la = selmaho(La).wf();
            field names = many1(cmevla_word()).wf();
        }
    }

    rule "descriptor" description_head -> struct {
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    rule "descriptor connective" description_head_connective -> struct {
        field connective <- boxed(jek_connective);
    }

    node description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field leading_description_head = boxed(description_head());
            field connective = description_head_connective();
            field trailing_description_head = boxed(description_head());
            field tail = description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens);
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = description_head();
            field tail = description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens);
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field outer_quantifier = quantifier(mekso, letter_tokens);
            field description = description_head();
            field tail = description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens);
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            require selmaho(Roi).not();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    rule "description tail" description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens) -> struct {
        field leading_tail_elements <- leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal);
        field tail <- boxed(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens));
    }

    rule "description tail" description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> enum {
        quantifier_relation_description_tail,
        quantifier_sumti_description_tail,
        relation_description_tail,
    }

    rule "description tail" leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal) -> struct {
        field tail_sumti <- opt(description_tail_sumti(sumti_base));
        field relative_clauses <- opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
    }

    rule "description tail" description_tail_sumti(sumti_base) -> struct {
        assert !pa_word();
        field sumti <- boxed(sumti_base);
    }

    rule "description tail" relation_description_tail(sumti, subbridi, selbri, tense_modal) -> struct {
        field selbri <- boxed(selbri);
        field relative_clauses <- opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
    }

    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        field selbri <- boxed(selbri);
        field relative_clauses <- opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
    }

    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        field sumti <- boxed(sumti);
    }

    node relative_tail_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field first_relative_clause = relative_clause_atom(sumti, subbridi, tense_modal);
            field additional_relative_clauses = many(relative_clause_tail(sumti, subbridi, tense_modal));
            field tail_quantifier = opt(quantifier(mekso, letter_tokens));
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node outer_quantified_relative_tail_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field outer_quantifier = quantifier(mekso, letter_tokens);
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field first_relative_clause = relative_clause_atom(sumti, subbridi, tense_modal);
            field additional_relative_clauses = many(relative_clause_tail(sumti, subbridi, tense_modal));
            field tail_quantifier = opt(quantifier(mekso, letter_tokens));
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node sumti_tail_relation_description_sumti(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            require pa_word().not();
            field tail_sumti = boxed(sumti);
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node sumti_tail_description_sumti(sumti, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field tail_sumti = boxed(sumti);
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node tail_quantified_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node gadri_elided_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node simple_description_sumti(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node outer_quantified_sumti_tail_description_sumti(sumti, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field outer_quantifier = quantifier(mekso, letter_tokens);
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field tail_sumti = boxed(sumti);
            field ku = opt(cmavo(Ku).wf());
        }
    }

    node outer_quantified_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field outer_quantifier = quantifier(mekso, letter_tokens);
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field tail_quantifier = opt(quantifier(mekso, letter_tokens));
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
    }

    rule "quote" quote(text) -> enum {
        experimental_mehoi_compound_quote,
        experimental_zohoi_compound_quote,
        experimental_rahoi_compound_quote,
        experimental_gohoi_compound_quote,
        generic_compound_quote,
        text_quote,
    }

    rule "text quote" text_quote(text) -> struct {
        field lu <- cmavo(Lu).wf();
        field text <- boxed(text);
        field lihu <- opt(cmavo(Lihu).wf());
    }

    rule "quote" experimental_mehoi_compound_quote -> struct {
        field quote <- quote_marker(Mehoi).warn(ExperimentalMehOiQuote).wf();
    }

    rule "quote" experimental_zohoi_compound_quote -> struct {
        field quote <- choice((
            quote_marker(Zohoi),
            quote_marker(Lahoi),
        )).warn(ExperimentalZohOiQuote).wf();
    }

    rule "quote" experimental_rahoi_compound_quote -> struct {
        field quote <- quote_marker(Rahoi).warn(ExperimentalZantufaRahoiQuote).wf();
    }

    rule "quote" experimental_gohoi_compound_quote -> struct {
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    rule "quote" generic_compound_quote -> struct {
        field quote <- word_category(Quote).wf();
    }

    node quoted_sumti(text) -> SumtiSyntax {
        context "quote";
        construct tuple_variant QuotedSumti;
        fields {
            field quote = boxed(quote(text));
        }
    }

    node selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "vocative phrase";
        fields {
            field leading_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field selbri = boxed(selbri);
            field trailing_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    node cmevla_vocative_sumti(sumti, subbridi, tense_modal) -> SumtiSyntax {
        context "vocative phrase";
        fields {
            field leading_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field names = many1(cmevla_word()).wf();
            field trailing_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    rule "vocative marker" vocative_marker_words -> enum {
        coi_vocative_marker_words,
        doi_vocative_marker_words,
    }

    rule "vocative marker" coi_vocative_marker_words -> struct {
        field first_coi <- selmaho(Coi);
        field first_nai <- opt(cmavo(Nai));
        field additional_coi <- [zero_or_more (selmaho(Coi), opt(cmavo(Nai)))];
        field doi <- opt(cmavo(Doi));
    }

    rule "vocative marker" doi_vocative_marker_words -> struct {
        field doi <- cmavo(Doi);
    }

    rule "free modifier" free_modifier(sumti, subbridi, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier) -> enum {
        text_replacement_free_modifier,
        sei_free_modifier,
        xi_free_modifier,
        mai_free_modifier,
        soi_free_modifier,
        parenthetical_text,
        vocative_free_modifier,
    }

    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, tense_modal) -> struct {
        field vocative_markers <- vocative_marker_words().wf();
        field sumti <- opt(boxed(choice((
            selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal),
            cmevla_vocative_sumti(sumti, subbridi, tense_modal),
            sumti,
        ))));
        field dohu <- opt(cmavo(Dohu).prohibited_wf());
    }

    rule "parenthetical text" parenthetical_text(text) -> struct {
        field to <- selmaho(To).wf();
        field text <- boxed(text);
        field toi <- opt(cmavo(Toi).prohibited_wf());
    }

    rule "metalinguistic bridi" sei_free_modifier(term, selbri) -> struct {
        field sei <- selmaho(Sei).wf();
        field terms <- [zero_or_more term];
        field cu <- opt(cmavo(Cu).wf());
        field selbri <- boxed(selbri);
        field sehu <- opt(cmavo(Sehu).prohibited_wf());
    }

    rule "subscript" xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier) -> enum {
        xi_number_free_modifier,
        xi_lerfu_string_free_modifier,
        xi_parenthesized_free_modifier,
    }

    rule "subscript" xi_number_free_modifier(letter_tokens) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- boxed(number_mekso(letter_tokens));
    }

    rule "subscript" xi_lerfu_string_free_modifier(letter_string, free_modifier) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- boxed(lerfu_string_mekso(letter_string, free_modifier));
    }

    rule "subscript" xi_parenthesized_free_modifier(mekso) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- boxed(parenthesized_mekso_operand(mekso));
    }

    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        field number <- number_or_letter_words(letter_tokens, letter_string);
        field mai <- selmaho(Mai).wf();
    }

    rule "reciprocal" soi_free_modifier(sumti) -> struct {
        field soi <- cmavo(Soi).wf();
        field leading_sumti <- boxed(sumti);
        field trailing_sumti <- opt(boxed(sumti));
        field sehu <- opt(cmavo(Sehu).wf());
    }

    rule "replacement free modifier" text_replacement_free_modifier -> enum {
        full_text_replacement_free_modifier,
        new_only_text_replacement_free_modifier,
        close_only_text_replacement_free_modifier,
    }

    rule "replacement free modifier" full_text_replacement_free_modifier -> struct {
        field lohai <- some(cmavo(Lohai));
        field old_words <- raw_words_until(Sahai, Lehai);
        field sahai <- opt(cmavo(Sahai));
        field new_words <- raw_words_until(Lehai);
        field lehai <- cmavo(Lehai).wf();
    }

    rule "replacement free modifier" new_only_text_replacement_free_modifier -> struct {
        field sahai <- some(cmavo(Sahai));
        field new_words <- raw_words_until(Lehai);
        field lehai <- cmavo(Lehai).wf();
    }

    rule "replacement free modifier" close_only_text_replacement_free_modifier -> struct {
        field lehai <- cmavo(Lehai).wf();
    }

    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal) -> enum {
        joined_relative_clause_tail,
        connected_relative_clause_tail,
    }

    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal) -> struct {
        field zihe <- cmavo(Zihe).wf();
        field inner <- boxed(relative_clause_atom(sumti, subbridi, tense_modal));
    }

    rule "relative clause" connected_relative_clause_tail(sumti, subbridi, tense_modal) -> struct {
        field connective <- choice((
            joik_connective(),
            jek_connective(),
        ));
        field inner <- boxed(relative_clause_atom(sumti, subbridi, tense_modal));
    }

    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal) -> enum {
        sumti_association_relative_clause,
        bridi_relative_clause,
    }

    rule "sumti association phrase" sumti_association_relative_clause(sumti, tense_modal) -> struct {
        field association_marker <- selmaho(Goi).wf();
        field sumti <- boxed(relative_sumti(sumti, tense_modal));
        field gehu <- opt(cmavo(Gehu).wf());
    }

    rule "sumti association phrase" relative_sumti(sumti, tense_modal) -> enum {
        tense_tagged_relative_sumti,
        na_ku_relative_sumti,
        plain_relative_sumti,
    }

    rule "sumti association phrase" na_ku_relative_sumti -> struct {
        field na <- selmaho(Na);
        field ku <- cmavo(Ku).wf();
    }

    rule "tagged sumti" tense_tagged_relative_sumti(tense_modal, sumti) -> struct {
        field tense_modal <- boxed(tense_modal);
        field sumti <- boxed(tagged_or_elided_sumti(sumti));
    }

    rule "sumti association phrase" plain_relative_sumti(sumti) -> struct {
        field sumti <- boxed(sumti);
    }

    rule "relative clause" bridi_relative_clause(subbridi) -> enum {
        restrictive_bridi_relative_clause,
        incidental_bridi_relative_clause,
    }

    rule "relative clause" restrictive_bridi_relative_clause(subbridi) -> struct {
        field poi <- choice((
            cmavo(Poi),
            cmavo(Pohoi),
        )).wf();
        field subbridi <- boxed(subbridi);
        field kuho <- opt(cmavo(Kuho).wf());
    }

    rule "relative clause" incidental_bridi_relative_clause(subbridi) -> struct {
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
            cmavo(Voi),
            cmavo(Voihi),
        )).wf();
        field subbridi <- boxed(subbridi);
        field kuho <- opt(cmavo(Kuho).wf());
    }

    product ek_connective -> ConnectiveSyntax {
        context "ek";
        construct variant Afterthought;
        model_variant EkAfterthoughtConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            scratch a = selmaho(A).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![a.value], a.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product jehi_connective -> ConnectiveSyntax {
        context "ek";
        construct variant Afterthought;
        model_variant JehiAfterthoughtConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            scratch jehi = selmaho(Jehi).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![jehi.value], jehi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product jek_connective -> ConnectiveSyntax {
        context "jek";
        construct variant Selbri;
        model_variant JekSelbriConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            scratch ja = selmaho(Ja).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![ja.value], ja.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    alias "joik" joik_connective =
        choice((
            joi_connective(),
            simple_interval_connective(),
            closed_interval_connective(),
        ));

    product joi_connective -> ConnectiveSyntax {
        context "joik";
        construct variant NonLogical;
        model_variant JoiNonLogicalConnective;
        fields {
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch joi = selmaho(Joi).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![joi.value], joi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product simple_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        model_variant SimpleIntervalConnective;
        fields {
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch bihi = selmaho(Bihi).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![bihi.value], bihi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product closed_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        model_variant ClosedIntervalConnective;
        fields {
            scratch left_interval = selmaho(Gaho);
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch bihi = selmaho(Bihi);
            scratch nai_token = opt(cmavo(Nai));
            scratch right_interval = selmaho(Gaho).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(
                vec![left_interval, bihi, right_interval.value],
                right_interval.free_modifiers,
            ));
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product vuhu_nonlogical_connective -> ConnectiveSyntax {
        context "non-logical connective";
        construct variant NonLogical;
        model_variant VuhuNonLogicalConnective;
        fields {
            default se: Option<Token> = None;
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch vuhu = selmaho(Vuhu).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![vuhu.value], vuhu.free_modifiers));
            default nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> = None;
        }
    }

    alias "sumti connective" argument_connective =
        choice((
            cehe_connective(),
            ek_connective(),
            jehi_connective(),
            joik_connective(),
            vuhu_nonlogical_connective(),
        ));

    alias "operand connective" operand_connective =
        choice((
            joik_connective(),
            ek_connective(),
            jek_connective(),
        ));

    alias "selbri connective" relation_afterthought_connective =
        choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        ));

    alias "statement connective" standard_statement_connective =
        choice((
            joik_connective(),
            jek_connective(),
        ));

    alias "statement connective" statement_connective =
        choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        ));

    alias "statement connective" i_statement_connective(tense_modal) =
        choice((
            i_standard_statement_connective(tense_modal),
            i_tag_bo_statement_connective(tense_modal),
        ));

    product i_standard_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        construct variant IStandardStatementConnective;
        fields {
            #[tree_child(primary)]
            field connective = boxed(statement_connective);
            field tag_bo = opt((opt(boxed(tense_modal)), cmavo(Bo).wf()));
        }
    }

    product i_standard_paragraph_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        construct variant IStandardParagraphStatementConnective;
        fields {
            #[tree_child(primary)]
            field connective = boxed(choice((
                paragraph_joi_connective(),
                paragraph_simple_interval_connective(),
                paragraph_closed_interval_connective(),
                paragraph_jek_connective(),
            )));
            field tag_bo = opt((opt(boxed(tense_modal)), cmavo(Bo)));
        }
    }

    product paragraph_jek_connective -> ConnectiveSyntax {
        context "jek";
        construct variant ParagraphJekConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            #[tree_child(primary)]
            field ja = selmaho(Ja);
            field nai = opt(cmavo(Nai));
        }
    }

    product paragraph_joi_connective -> ConnectiveSyntax {
        context "joik";
        construct variant NonLogical;
        model_variant ParagraphJoiNonLogicalConnective;
        fields {
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch joi = selmaho(Joi);
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![joi], Vec::new()));
            scratch nai_token = opt(cmavo(Nai));
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product paragraph_simple_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        model_variant ParagraphSimpleIntervalConnective;
        fields {
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch bihi = selmaho(Bihi);
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![bihi], Vec::new()));
            scratch nai_token = opt(cmavo(Nai));
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product paragraph_closed_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        model_variant ParagraphClosedIntervalConnective;
        fields {
            scratch left_interval = selmaho(Gaho);
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch bihi = selmaho(Bihi);
            scratch nai_token = opt(cmavo(Nai));
            scratch right_interval = selmaho(Gaho);
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(
                vec![left_interval, bihi, right_interval],
                Vec::new(),
            ));
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product i_tag_bo_paragraph_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        construct variant ITagBoParagraphStatementConnective;
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo);
        }
    }

    product i_tag_bo_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        construct variant ITagBoStatementConnective;
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
        }
    }

    product cehe_connective -> ConnectiveSyntax {
        context "termset connective";
        construct variant NonLogical;
        model_variant CeheNonLogicalConnective;
        fields {
            default se: Option<Token> = None;
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch cehe = cmavo(Cehe).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![cehe.value], cehe.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product gihek_connective -> ConnectiveSyntax {
        context "gihek";
        construct variant BridiTail;
        model_variant GihekBridiTailConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            scratch giha = selmaho(Giha).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![giha.value], giha.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product guhek_connective -> ConnectiveSyntax {
        context "forethought selbri connective";
        construct variant Forethought;
        model_variant GuhekForethoughtConnective;
        fields {
            field nahe = opt(selmaho(Nahe));
            field se = opt(selmaho(Se));
            default na: Option<Token> = None;
            scratch guha = selmaho(Guha).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![guha.value], guha.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    alias "bridi tail connective" bridi_tail_connective =
        choice((
            gihek_connective(),
            relation_connective_as_bridi_tail(),
        ));

    product relation_connective_as_bridi_tail -> ConnectiveSyntax {
        context "bridi tail connective";
        construct variant RelationConnectiveAsBridiTail;
        fields {
            #[tree_child(primary)]
            field connective = boxed(relation_afterthought_connective);
        }
    }

    alias "forethought connective" modal_forethought_connective(tense_modal) =
        choice((
            ga_forethought_connective(),
            joik_jek_gi_forethought_connective(),
            jek_gi_forethought_connective(),
            modal_gi_forethought_connective(tense_modal),
            feature(ZantufaConnectives, zantufa_initial_gi_forethought_connective()),
        ));

    product ga_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant Forethought;
        model_variant GaForethoughtConnective;
        fields {
            field se = opt(selmaho(Se));
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch ga = selmaho(Ga).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![ga.value], ga.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    product zantufa_initial_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant ZantufaInitialGiForethoughtConnective;
        fields {
            field gi = cmavo(Gi).warn(ExperimentalZantufaGek).wf();
            field tail = boxed(choice((
                joik_connective(),
                jek_connective(),
            )));
            field bo = opt(cmavo(Bo).wf());
        }
    }

    product joik_jek_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant JoikJekGiForethoughtConnective;
        fields {
            field connective = boxed(joik_connective());
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
    }

    product jek_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant JekGiForethoughtConnective;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            field ja = selmaho(Ja).warn(ExperimentalZantufaGek).wf();
            field nai = opt(cmavo(Nai).wf());
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
    }

    product modal_gi_forethought_connective(tense_modal) -> ConnectiveSyntax {
        context "forethought connective";
        construct variant ModalGiForethoughtConnective;
        fields {
            field tense_modal = boxed(tense_modal);
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
    }

    product gik_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant Forethought;
        model_variant GikForethoughtConnective;
        fields {
            default se: Option<Token> = None;
            default nahe: Option<Token> = None;
            default na: Option<Token> = None;
            scratch gi = cmavo(Gi).wf();
            #[tree_child(primary)]
            let cmavo: std::sync::Arc<WithFreeModifiers<Vec<Token>, FreeModifierSyntax>> =
                std::sync::Arc::new(WithFreeModifiers::new(vec![gi.value], gi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> =
                nai_token.map(std::sync::Arc::new);
        }
    }

    alias "tag" tense_modal(selbri) =
        guard(
            choice((
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
            )),
            choice((
                connected_tense_modal(selbri),
                tense_modal_atom(selbri),
            )),
        );

    node connected_tense_modal(selbri) -> TenseModalSyntax {
        context "connected tag";
        fields {
            field first = boxed(tense_modal_atom(selbri));
            field continuations = many1((
                choice((
                    joik_connective(),
                    jek_connective(),
                )),
                boxed(tense_modal_atom(selbri)),
            ));
        }
    }

    alias "tag" tense_modal_atom(selbri) =
        choice((
            composite_tense(),
            fiho_tense(selbri),
            modal_tense(),
            nahe_se_flat_prefixed_tense(),
            se_flat_prefixed_tense(),
            fa_flat_tag_tense(),
            feature(ZantufaTags, zantufa_recursive_tag_tense()),
            sticky_tense(),
        ));

    node fiho_tense(selbri) -> TenseModalSyntax {
        context "FIhO modal";
        fields {
            field fiho = cmavo(Fiho).wf();
            field selbri = boxed(selbri);
            field fehu = opt(cmavo(Fehu).wf());
        }
    }

    node fa_flat_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field fa = selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        }
    }

    rule "tag" flat_tag_atom -> enum {
        fa_flat_tag_atom,
        modal_flat_tag_atom,
        composite_flat_tag_atom,
    }

    rule "tag" fa_flat_tag_atom -> struct {
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
    }

    rule "modal tag" modal_flat_tag_atom -> struct {
        field modal <- boxed(modal_tense());
    }

    rule "tag" composite_flat_tag_atom -> struct {
        field composite <- boxed(composite_tense());
    }

    node nahe_se_flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field nahe = selmaho(Nahe).warn(ExperimentalFlattenedTag).wf();
            field se = opt(selmaho(Se).wf());
            field atom = flat_tag_atom();
        }
    }

    node se_flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field se = selmaho(Se).warn(ExperimentalFlattenedTag).wf();
            field atom = flat_tag_atom();
        }
    }

    node zantufa_recursive_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field first_prefix = choice((
                selmaho(Nahe),
                selmaho(Se),
            )).warn(ExperimentalZantufaRecursiveTag).wf();
            field additional_prefixes = many(choice((
                selmaho(Nahe),
                selmaho(Se),
            )).wf());
            field atom = choice((
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
    }

    alias "tag" composite_tense =
        choice((
            prefixed_time_space_caha_tense(),
            time_space_caha_ki_tense(),
            cuhe_tense(),
        ));

    node prefixed_time_space_caha_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field nahe = selmaho(Nahe).wf();
            field tense = boxed(time_space_caha_tense());
            field ki = opt(boxed(ki_composite_tense()));
        }
    }

    node time_space_caha_ki_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field tense = boxed(time_space_caha_tense());
            field ki = opt(boxed(ki_composite_tense()));
        }
    }

    alias "tag" time_space_caha_tense =
        choice((
            time_then_space_caha_tense(),
            space_then_time_caha_tense(),
            caha_tense(),
        ));

    node time_then_space_caha_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field time = boxed(time_tense());
            field space = opt(boxed(space_tense()));
            field caha = opt(boxed(caha_tense()));
        }
    }

    node space_then_time_caha_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field space = boxed(space_tense());
            field time = opt(boxed(time_tense()));
            field caha = opt(boxed(caha_tense()));
        }
    }

    alias "time tense" time_tense =
        choice((
            time_tense_with_zi(),
            time_tense_with_offset(),
            time_tense_with_interval(),
            time_tense_with_properties(),
        ));

    node time_tense_with_zi -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = boxed(zi_time_distance_tense());
            field offsets = many(boxed(pu_time_offset_tense()));
            field zeha = opt(boxed(zeha_time_interval_tense()));
            field properties = many(boxed(interval_property_tense()));
        }
    }

    node time_tense_with_offset -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(boxed(zi_time_distance_tense()));
            field offsets = many1(boxed(pu_time_offset_tense()));
            field zeha = opt(boxed(zeha_time_interval_tense()));
            field properties = many(boxed(interval_property_tense()));
        }
    }

    node time_tense_with_interval -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(boxed(zi_time_distance_tense()));
            field offsets = many(boxed(pu_time_offset_tense()));
            field zeha = boxed(zeha_time_interval_tense());
            field properties = many(boxed(interval_property_tense()));
        }
    }

    node time_tense_with_properties -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(boxed(zi_time_distance_tense()));
            field offsets = many(boxed(pu_time_offset_tense()));
            field zeha = opt(boxed(zeha_time_interval_tense()));
            field properties = many1(boxed(interval_property_tense()));
        }
    }

    alias "interval property" interval_property_tense =
        choice((
            numbered_interval_property_tense(),
            tahe_interval_property_tense(),
            zaho_interval_property_tense(),
        ));

    node numbered_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field number = interval_property_number_words().wf();
            field roi = selmaho(Roi).wf();
            field nai = opt(cmavo(Nai).wf());
        }
    }

    alias "number" interval_property_number_words =
        [pa_word(); zero_or_more ..choice((
            [pa_word()],
            [word_category(LetterWord)],
        ))];

    node tahe_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field tahe = selmaho(Tahe).wf();
            field nai = opt(cmavo(Nai).wf());
        }
    }

    node zaho_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field zaho = selmaho(Zaho).wf();
            field nai = opt(cmavo(Nai).wf());
        }
    }

    node pu_time_offset_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field pu = selmaho(Pu).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Zi).wf());
        }
    }

    node zi_time_distance_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = selmaho(Zi).wf();
        }
    }

    node zeha_time_interval_tense -> TenseModalSyntax {
        context "time interval";
        fields {
            field zeha = selmaho(Zeha).wf();
            field direction = opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
        }
    }

    alias "space tense" space_tense =
        choice((
            space_tense_with_va(),
            space_tense_with_offset(),
            space_tense_with_interval(),
            space_tense_with_mohi(),
        ));

    node space_tense_with_va -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = boxed(va_space_distance_tense());
            field offsets = many(boxed(faha_space_offset_tense()));
            field interval = opt(boxed(space_interval_tense()));
            field mohi = opt(boxed(mohi_space_offset_tense()));
        }
    }

    node space_tense_with_offset -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(boxed(va_space_distance_tense()));
            field offsets = many1(boxed(faha_space_offset_tense()));
            field interval = opt(boxed(space_interval_tense()));
            field mohi = opt(boxed(mohi_space_offset_tense()));
        }
    }

    node space_tense_with_interval -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(boxed(va_space_distance_tense()));
            field offsets = many(boxed(faha_space_offset_tense()));
            field interval = boxed(space_interval_tense());
            field mohi = opt(boxed(mohi_space_offset_tense()));
        }
    }

    node space_tense_with_mohi -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(boxed(va_space_distance_tense()));
            field offsets = many(boxed(faha_space_offset_tense()));
            field interval = opt(boxed(space_interval_tense()));
            field mohi = boxed(mohi_space_offset_tense());
        }
    }

    node va_space_distance_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = selmaho(Va).wf();
        }
    }

    node faha_space_offset_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field faha = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Va).wf());
        }
    }

    node faha_interval_direction_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field faha = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
        }
    }

    alias "space interval" space_interval_tense =
        choice((
            space_interval_with_extent_tense(),
            space_interval_properties_tense(),
        ));

    node space_interval_with_extent_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field extent = boxed(choice((
                veha_space_interval_tense(),
                viha_space_interval_tense(),
            )));
            field direction = opt(boxed(faha_interval_direction_tense()));
            field properties = opt(boxed(space_interval_properties_tense()));
        }
    }

    node space_interval_properties_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field first = boxed(fehe_interval_property_tense());
            field additional = many(boxed(fehe_interval_property_tense()));
        }
    }

    node veha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field veha = selmaho(Veha).wf();
            field viha = opt(selmaho(Viha).wf());
        }
    }

    node viha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field viha = selmaho(Viha).wf();
        }
    }

    node fehe_interval_property_tense -> TenseModalSyntax {
        context "space interval property";
        fields {
            field fehe = cmavo(Fehe).wf();
            field property = boxed(interval_property_tense());
        }
    }

    node mohi_space_offset_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field mohi = selmaho(Mohi).wf();
            field offset = boxed(faha_space_offset_tense());
        }
    }

    node caha_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field caha = selmaho(Caha).wf();
        }
    }

    node ki_composite_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field ki = cmavo(Ki).wf();
        }
    }

    node cuhe_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field cuhe = selmaho(Cuhe).wf();
        }
    }

    node pu_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field pu = selmaho(Pu).wf();
        }
    }

    node va_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field va = selmaho(Va).wf();
        }
    }

    node modal_tense -> TenseModalSyntax {
        context "modal tag";
        fields {
            field nahe = opt(selmaho(Nahe).wf());
            field se = opt(selmaho(Se).wf());
            field bai = selmaho(Bai).wf();
            field nai = opt(cmavo(Nai).wf());
            field ki = opt(cmavo(Ki).wf());
        }
    }

    node fa_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field fa = selmaho(Fa).wf();
        }
    }

    node sticky_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field ki = cmavo(Ki).wf();
        }
    }

    node tagged_selbri(selbri, co_selbri, tense_modal) -> SelbriSyntax {
        context "tagged selbri";
        construct variant TaggedSelbri;
        fields {
            field tense_modal = boxed(tense_modal);
            field inner_selbri = boxed(untagged_selbri(selbri, co_selbri));
        }
    }

    node negated_selbri(selbri) -> SelbriSyntax {
        context "negated selbri";
        construct variant Negated;
        fields {
            field na = selmaho(Na).not_next_selmaho(Ku).wf();
            field inner_selbri = boxed(selbri);
        }
    }

    alias "selbri" selbri(selbri, co_selbri, tense_modal) =
        choice((
            tagged_selbri(selbri, co_selbri, tense_modal),
            untagged_selbri(selbri, co_selbri),
        ));

    alias "selbri" untagged_selbri(selbri, co_selbri) =
        choice((
            negated_selbri(selbri),
            co_selbri,
            forethought_selbri_connection(selbri),
        ));

    node co_selbri(co_selbri, tanru_unit) -> SelbriSyntax {
        context "selbri";
        fields {
            field leading_selbri = boxed(connected_selbri(tanru_unit));
            field co_tail = opt((cmavo(Co).wf(), boxed(co_selbri)));
        }
    }

    node forethought_selbri_connection(selbri) -> SelbriSyntax {
        context "forethought selbri connection";
        fields {
            field guhek = guhek_connective;
            field leading_selbri = boxed(selbri);
            field gik = gik_connective;
            field trailing_selbri = boxed(selbri);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        }
    }

    node connected_selbri(tanru_unit) -> SelbriSyntax {
        context "selbri connection";
        fields {
            field leading_selbri = boxed(tanru_selbri(tanru_unit));
            field continuations = many((relation_afterthought_connective, boxed(tanru_selbri(tanru_unit))));
        }
    }

    node tanru_selbri(tanru_unit) -> SelbriSyntax {
        context "selbri";
        fields {
            field first_unit = tanru_unit;
            field additional_units = many(tanru_unit);
        }
    }

    alias "tanru unit" tanru_unit(bo_or_linked_tanru_unit) = connected_tanru_unit(bo_or_linked_tanru_unit);

    node connected_tanru_unit(bo_or_linked_tanru_unit) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field leading_unit = boxed(bo_or_linked_tanru_unit);
            field continuations = many((relation_afterthought_connective, boxed(bo_or_linked_tanru_unit)));
        }
    }

    alias "tanru unit" bo_or_linked_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) =
        choice((
            forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri),
            bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal),
            assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string),
            linked_tanru_unit(tanru_unit_atom, sumti, tense_modal),
        ));

    node forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri) -> TanruUnitSyntax {
        context "forethought selbri connection";
        fields {
            field guhek = guhek_connective;
            field leading_selbri = boxed(selbri);
            field gik = gik_connective;
            field trailing_unit = boxed(bo_or_linked_tanru_unit);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        }
    }

    node bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal) -> TanruUnitSyntax {
        context "BO-grouped tanru unit";
        construct variant BoundTanruUnitConnection;
        fields {
            field leading_unit = boxed(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal));
            field bo_connective = opt(boxed(relation_afterthought_connective));
            field bo_tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field trailing_unit = boxed(bo_or_linked_tanru_unit);
        }
    }

    node assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "pro-bridi assignment";
        fields {
            field base = boxed(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
            field assignments = many1((cmavo(Cei).wf(), boxed(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string))));
        }
    }

    node linked_tanru_unit(tanru_unit_atom, sumti, tense_modal) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field base = boxed(tanru_unit_atom);
            field linkargs = opt(linkargs(sumti, tense_modal));
        }
    }

    node linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field base = boxed(tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
            field linkargs = opt(linkargs(sumti, tense_modal));
        }
    }

    node tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field conversions = many(selmaho(Se).wf());
            field base = boxed(tanru_unit_base_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
        }
    }

    alias "tanru unit" tanru_unit_base_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) =
        choice((
            pro_bridi_tanru_unit(),
            ordinal_tanru_unit(letter_tokens, letter_string),
            word_tanru_unit(),
            preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal),
            jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal),
            scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal),
            abstraction_tanru_unit(subbridi),
            sumti_selbri_tanru_unit(sumti, letter_string),
            operator_selbri_tanru_unit(mekso_operator),
            quoted_bridi_selbri_tanru_unit(),
            quoted_text_selbri_tanru_unit(),
            text_selbri_tanru_unit(text),
            tag_selbri_tanru_unit(tense_modal),
            goha_word_tanru_unit(free_modifier),
            grouped_tanru_unit(tanru_unit),
        ));

    node tanru_unit_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field conversions = many(selmaho(Se).wf());
            field base = boxed(tanru_unit_base_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
        }
    }

    alias "tanru unit" tanru_unit_base_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) =
        choice((
            ordinal_tanru_unit(letter_tokens, letter_string),
            word_tanru_unit(),
            preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal),
            jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal),
            scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal),
            abstraction_tanru_unit(subbridi),
            sumti_selbri_tanru_unit(sumti, letter_string),
            operator_selbri_tanru_unit(mekso_operator),
            quoted_bridi_selbri_tanru_unit(),
            quoted_text_selbri_tanru_unit(),
            text_selbri_tanru_unit(text),
            tag_selbri_tanru_unit(tense_modal),
            goha_word_tanru_unit(free_modifier),
            pro_bridi_tanru_unit(),
            grouped_tanru_unit(tanru_unit),
        ));

    node tagged_selbri_group_tanru_unit(tanru_unit, tense_modal) -> TanruUnitSyntax {
        context "tagged selbri";
        fields {
            field tense_modal = boxed(tense_modal);
            field inner_selbri = boxed(connected_selbri(tanru_unit));
        }
    }

    node preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal) -> TanruUnitSyntax {
        context "linked arguments";
        fields {
            field linkargs = linkargs(sumti, tense_modal);
            field base = boxed(tanru_unit);
        }
    }

    node scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal) -> TanruUnitSyntax {
        context "scalar-negated tanru unit";
        construct variant ScalarNegatedTanruUnit;
        fields {
            field nahe = selmaho(Nahe).wf();
            field inner_unit = boxed(choice((
                tagged_selbri_group_tanru_unit(tanru_unit, tense_modal),
                pro_bridi_tanru_unit(),
                tanru_unit_atom,
            )));
        }
    }

    node jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal) -> TanruUnitSyntax {
        context "modal conversion";
        construct variant ModalConversion;
        fields {
            field jai = cmavo(Jai).wf();
            field tense_modal = opt(boxed(tense_modal));
            field inner_unit = boxed(jai_inner_tanru_unit);
        }
    }

    alias "modal conversion" jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, letter_tokens, letter_string) =
        choice((
            converted_jai_inner_tanru_unit(jai_inner_tanru_unit),
            scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit),
            sumti_selbri_tanru_unit(sumti, letter_string),
            quoted_bridi_selbri_tanru_unit(),
            quoted_text_selbri_tanru_unit(),
            text_selbri_tanru_unit(text),
            grouped_tanru_unit(jai_inner_tanru_unit),
            ordinal_tanru_unit(letter_tokens, letter_string),
            operator_selbri_tanru_unit(mekso_operator),
            pro_bridi_tanru_unit(),
            word_tanru_unit(),
        ));

    node converted_jai_inner_tanru_unit(jai_inner_tanru_unit) -> TanruUnitSyntax {
        context "converted tanru unit";
        construct variant ConvertedTanruUnit;
        fields {
            field se = selmaho(Se).wf();
            field inner_unit = boxed(jai_inner_tanru_unit);
        }
    }

    node scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit) -> TanruUnitSyntax {
        context "scalar-negated tanru unit";
        construct variant ScalarNegatedTanruUnit;
        model_variant ScalarNegatedJaiInnerTanruUnit;
        fields {
            field nahe = selmaho(Nahe).wf();
            field inner_unit = boxed(jai_inner_tanru_unit);
        }
    }

    node quoted_bridi_selbri_tanru_unit -> TanruUnitSyntax {
        context "quoted bridi selbri";
        construct tuple_variant QuotedBridiSelbri;
        fields {
            field quote = choice((
                quote_marker(Gohoi),
                quote_marker(Zehoi),
                quote_marker(Tahai),
                quote_marker(Bohei),
            )).warn(ExperimentalGohoiSelbriUnit).wf();
        }
    }

    node text_selbri_tanru_unit(text) -> TanruUnitSyntax {
        context "text selbri";
        construct variant TextSelbri;
        fields {
            field luhei = cmavo(Luhei).warn(ExperimentalZantufaLuheiSelbriUnit).wf();
            field text = boxed(text);
            field liau = opt(cmavo(Lihau).wf());
        }
    }

    node quoted_text_selbri_tanru_unit -> TanruUnitSyntax {
        context "quoted text selbri";
        construct tuple_variant QuotedTextSelbri;
        fields {
            field muhoi = delimited_quote_marker(Muhoi).warn(ExperimentalZantufaMuhoiSelbriUnit).wf();
        }
    }

    node tag_selbri_tanru_unit(tense_modal) -> TanruUnitSyntax {
        context "tag selbri";
        construct variant TagSelbri;
        fields {
            field xohi = cmavo(Xohi).warn(ExperimentalXohiTagSelbri).wf();
            field tag = boxed(tense_modal);
        }
    }

    node ordinal_tanru_unit(letter_tokens, letter_string) -> TanruUnitSyntax {
        context "ordinal selbri";
        fields {
            field number = number_or_letter_words(letter_tokens, letter_string);
            field moi = selmaho(Moi).wf();
        }
    }

    node word_tanru_unit -> TanruUnitSyntax {
        context "tanru unit";
        construct tuple_variant TanruUnitWord;
        fields {
            field word = tanru_unit_relation_word().wf();
        }
    }

    node goha_word_tanru_unit(free_modifier) -> TanruUnitSyntax {
        context "tanru unit";
        construct tuple_variant TanruUnitWord;
        model_variant GohaWordTanruUnit;
        fields {
            field word = selmaho(Goha)
                .followed_by(choice((
                    cmavo(Raho).ignored(),
                    cmavo(Be).ignored(),
                    pa_word().ignored(),
                    free_modifier.ignored(),
                )).not())
                .wf();
        }
    }

    node pro_bridi_tanru_unit -> TanruUnitSyntax {
        context "pro-bridi";
        construct variant ProBridi;
        fields {
            field goha = selmaho(Goha).wf();
            field raho = opt(cmavo(Raho).wf());
        }
    }

    node sumti_selbri_tanru_unit(sumti, letter_string) -> TanruUnitSyntax {
        context "sumti selbri";
        construct variant SumtiSelbri;
        fields {
            field me = cmavo(Me).wf();
            field sumti = boxed(choice((sumti, me_lerfu_sumti(letter_string))));
            field mehu = opt(cmavo(Mehu).wf());
            field moi_marker = opt(selmaho(Moi).wf());
        }
    }

    node me_lerfu_sumti(letter_string) -> SumtiSyntax {
        context "lerfu string";
        fields {
            field words = letter_string;
        }
    }

    node operator_selbri_tanru_unit(mekso_operator) -> TanruUnitSyntax {
        context "operator-to-selbri";
        construct variant OperatorSelbri;
        fields {
            field nuha = cmavo(Nuha).wf();
            field mekso_operator = boxed(mekso_operator);
        }
    }

    node grouped_tanru_unit(tanru_unit) -> TanruUnitSyntax {
        context "grouped tanru";
        construct variant GroupedTanruUnit;
        fields {
            default ke_tense_modal: Option<Box<TenseModalSyntax>> = None;
            field ke = cmavo(Ke).wf();
            field selbri = boxed(connected_selbri(tanru_unit));
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    rule "linked arguments" linked_sumti(sumti, tense_modal) -> enum {
        place_tagged_linked_sumti,
        tense_tagged_linked_sumti,
        plain_linked_sumti,
        empty_linked_sumti,
    }

    rule "linked arguments" place_tagged_linked_sumti(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- boxed(tagged_or_elided_sumti(sumti));
    }

    rule "linked arguments" tense_tagged_linked_sumti(sumti, tense_modal) -> struct {
        field tense_modal <- boxed(tense_modal);
        field sumti <- boxed(tagged_or_elided_sumti(sumti));
    }

    rule "linked arguments" plain_linked_sumti(sumti) -> struct {
        field sumti <- boxed(sumti);
    }

    rule "linked arguments" empty_linked_sumti -> struct {
    }

    rule "linked arguments" bei_link(sumti, tense_modal) -> struct {
        field bei <- cmavo(Bei).wf();
        field link <- linked_sumti(sumti, tense_modal);
    }

    rule "linked arguments" linkargs(sumti, tense_modal) -> struct {
        field be <- cmavo(Be).wf();
        field first_link <- linked_sumti(sumti, tense_modal);
        field bei_links <- [zero_or_more bei_link(sumti, tense_modal)];
        field beho <- opt(cmavo(Beho).wf());
    }

    node abstraction_tanru_unit(subbridi) -> TanruUnitSyntax {
        context "abstraction";
        fields {
            field nu = selmaho(Nu).wf();
            field nai = opt(cmavo(Nai).wf());
            field abstractor_connections = many(abstractor_connection());
            field subbridi = boxed(subbridi);
            field kei = opt(cmavo(Kei).wf());
        }
    }

    rule "abstractor connection" abstractor_connection -> struct {
        field connective <- standard_statement_connective;
        field nu <- selmaho(Nu).wf();
        field nai <- opt(cmavo(Nai).wf());
    }
        }
    };
}

#[doc(hidden)]
pub mod generated_model {
    #![allow(dead_code)]

    use super::*;

    declare_generated_syntax_grammar! {
        tree_model {
            #![tree_with_free_modifiers]
            pub type WordRun = ::vec1::Vec1<Token>;
            pub type MeksoVec = ::vec1::Vec1<MeksoSyntax>;
        }
        model;
        env generated_runtime::SyntaxGrammarEnv;
        strict_parsers;
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

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    pub fn parse_text(
        words: &[Token],
        options: &ParseOptions,
    ) -> Result<TextSyntax, crate::SyntaxError> {
        let tokens = spanned_tokens(words);
        let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
        let mut state = ParserState::new(words, options);
        strict_generated_text_parser()
            .then_ignore(end())
            .parse_with_state(
                tokens
                    .as_slice()
                    .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
                &mut state,
            )
            .into_result()
            .map_err(syntax_error)
    }
}
