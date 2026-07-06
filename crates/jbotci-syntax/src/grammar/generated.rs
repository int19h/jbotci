//! Declarative generated syntax parser.

use chumsky::span::SimpleSpan;
use chumsky::{
    Parser,
    input::Input,
    primitive::{custom, end},
    recursive::Recursive,
};
use jbotci_morphology::{Cmavo, Selmaho};

use super::generated_runtime;
use super::tokens::{
    cmavo, cmevla_word, pa_word, relation_word, selmaho, spanned_tokens,
    syntax_error_with_diagnostic_candidate,
};
use super::{BoxedParser, ParseExtra, ParserInput, ParserState};
use crate::{
    ExperimentalConstruct, ParseOptions, SyntaxWarning, SyntaxWordCategory, Token, TraceReport,
};

#[doc(hidden)]
pub mod generated_model {
    use crate::tree::WithFreeModifiers;

    use super::*;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {
            #![tree_with_free_modifiers]
        }
        model;
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

    rule "leading indicator" leading_indicator -> struct {
        field indicator <- choice((selmaho(Ui), selmaho(Cai)));
        field nai <- opt(cmavo(Nai));
    }

    rule "text" text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> enum {
        explicit_xauha_lohoi_text,
        regular_text,
    }

    alias "word" word_before_kuhau = word_not_cmavo(Kuhau);

    rule "text" explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier) -> struct {
        assert [
            cmavo(Xauha);
            zero_or_more word_before_kuhau();
            cmavo(Kuhau);
        ].ignored();
        field paragraphs <- text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
    }

    rule "text" regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> struct {
        field leading_nai <- [zero_or_more cmavo(Nai)];
        field leading_cmevla <- [zero_or_more text_leading_cmevla_word()];
        field leading_indicators <- [zero_or_more leading_indicator()];
        field leading_free_modifiers <- [zero_or_more free_modifier];
        field leading_connective <- opt(
            modal_forethought_connective(tense_modal)
                .not()
                .ignore_then(text_leading_connective),
        );
        field leading_i_statements <- [zero_or_more leading_i_statement(free_modifier, tense_modal)];
        #[tree_child(primary)]
        field paragraphs <- opt(arc(text_paragraphs(
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
        field connective <- opt(arc(i_paragraph_statement_connective(tense_modal)));
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
        field i <- cmavo(I);
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    rule "paragraph" niho_paragraph(statement_or_fragment, free_modifier) -> struct {
        field niho <- [one_or_more selmaho(Niho)];
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statements <- opt(arc(paragraph_statement_sequence(statement_or_fragment, free_modifier)));
    }

    rule "paragraph statement" initial_paragraph_statement(statement_or_fragment) -> struct {
        #[tree_child(primary)]
        field statement <- arc(statement_or_fragment);
    }

    rule "paragraph statement" following_paragraph_statement(statement_or_fragment, free_modifier) -> struct {
        field i <- cmavo(I);
        assert !statement_connective;
        field free_modifiers <- [zero_or_more free_modifier];
        #[tree_child(primary)]
        field statement <- opt(arc(statement_or_fragment));
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
        when feature(ZantufaConnectives) forethought_statement,
        bridi_statement,
        text_group_statement,
    }

    rule "paragraph statement" statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
        when feature(ZantufaTerms) zantufa_statement_terms_statement,
        statement_or_fragment_statement,
        fragment_statement,
    }

    rule "paragraph statement" zantufa_statement_terms_statement(statement, term) -> struct {
        field statement <- arc(statement);
        field tail <- zantufa_statement_terms_tail(term);
    }

    rule "paragraph statement" zantufa_statement_terms_tail(term) -> enum {
        zantufa_iau_statement_terms_tail,
        zantufa_bare_statement_terms_tail,
    }

    rule "paragraph statement" zantufa_iau_statement_terms_tail(term) -> struct {
        field iau <- cmavo(Ihau).warn(ExperimentalIauReset).wf();
        field terms <- [zero_or_more term];
    }

    rule "paragraph statement" zantufa_bare_statement_terms_tail(term) -> struct {
        field terms <- [one_or_more arc(term)];
    }

    rule "paragraph statement" statement_or_fragment_statement(statement) -> struct {
        #[tree_child(primary)]
        field statement <- statement;
    }

    rule "fragment" fragment_statement(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> enum {
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
        zantufa_mekso_fragment,
    }

    rule "statement" statement_after_i_connective(statement, bridi, subbridi, tense_modal, text) -> enum {
        when feature(ZantufaConnectives) forethought_statement,
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
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
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
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    rule "statement connection" simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> struct {
        field i <- cmavo(I);
        field connective <- i_statement_connective(tense_modal);
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    rule "statement connection" preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> struct {
        field leading_statement <- arc(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
        field connective <- statement_connective;
        field i <- cmavo(I);
        field trailing_statement <- arc(statement_after_i_connective(statement, bridi, subbridi, tense_modal, text));
    }

    rule "text group" text_group_statement(text, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field tuhe <- cmavo(Tuhe).wf();
        #[tree_child(primary)]
        field text <- arc(text);
        field tuhu <- opt(cmavo(Tuhu).wf()).elidable_terminator(Tuhu);
    }

    rule "prenex" prenex_fragment(term) -> struct {
        field terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
    }

    rule "prenex" prenex_statement(statement, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        #[tree_child(primary)]
        field inner_statement <- arc(statement);
    }

    rule "statement" forethought_statement(statement, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- arc(statement);
        field first_branch <- forethought_statement_branch(statement);
        field additional_branches <- [zero_or_more zantufa_forethought_statement_branch(statement)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    rule "statement branch" forethought_statement_branch(statement) -> struct {
        field gik <- gik_connective;
        field statement <- arc(statement);
    }

    rule "statement branch" zantufa_forethought_statement_branch(statement) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field statement <- arc(statement);
    }

    rule "statement" bridi_statement(bridi, subbridi, tense_modal) -> struct {
        #[tree_child(primary)]
        field bridi <- arc(bridi);
        field continuations <- [zero_or_more bridi_statement_continuation(subbridi, tense_modal)];
    }

    rule "bridi continuation" bridi_statement_continuation(subbridi, tense_modal) -> enum {
        bo_bridi_statement_continuation,
        ke_bridi_statement_continuation,
    }

    rule "bridi continuation" bo_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_subbridi <- arc(subbridi);
    }

    rule "bridi continuation" ke_bridi_statement_continuation(subbridi, tense_modal) -> struct {
        assert feature(ZantufaConnectives).not();
        field connective <- relation_afterthought_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field trailing_subbridi <- arc(subbridi);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "selbri" selbri_fragment(selbri) -> struct {
        #[tree_child(primary)]
        field selbri <- arc(selbri);
    }

    rule "terms" terms_fragment(term) -> struct {
        #[tree_child(primary)]
        field terms <- [one_or_more term];
        field vau <- opt(cmavo(Vau).wf()).elidable_terminator(Vau);
    }

    rule "mex" mekso_fragment(mekso, letter_tokens) -> struct {
        #[tree_child(primary)]
        field quantifier <- arc(quantifier(mekso, letter_tokens));
    }

    rule "mex" zantufa_mekso_fragment(mekso) -> struct {
        #[tree_child(primary)]
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_statement_item());
    }

    rule "relative clauses" relative_clause_list(sumti, subbridi, tense_modal, statement) -> struct {
        field first <- relative_clause_atom(sumti, subbridi, tense_modal, statement);
        field additional <- [zero_or_more relative_clause_tail(sumti, subbridi, tense_modal, statement)];
    }

    rule "relative clauses" relative_clause_fragment(sumti, subbridi, tense_modal, statement) -> struct {
        #[tree_child(primary)]
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
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
        field bridi_tail <- arc(bridi_tail);
    }

    rule "bridi" bridi_with_post_cu_terms(term, bridi_tail) -> struct {
        field leading_terms <- [one_or_more term];
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    rule "bridi" bare_cu_bridi(bridi_tail) -> struct {
        field cu <- arc(cmavo(Cu).wf());
        field bridi_tail <- arc(bridi_tail);
    }

    rule "bridi" bare_cu_terms_bridi(term, bridi_tail) -> struct {
        field cu <- arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf());
        field bridi_tail <- arc(cu_terms_bridi_tail(term, bridi_tail));
    }

    rule "bridi" relation_only_bridi(bridi_tail) -> struct {
        field bridi_tail <- arc(bridi_tail);
    }

    rule "bridi tail" cu_terms_bridi_tail(term, bridi_tail) -> struct {
        field terms <- [one_or_more term];
        field bridi_tail <- arc(bridi_tail);
    }

    rule "bridi tail" bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> enum {
        when feature(ZantufaTerms) zantufa_grouped_bridi_tail,
        bridi_tail_with_possible_tail_terms,
        bridi_tail_without_tail_terms,
    }

    rule "bridi tail" zantufa_grouped_bridi_tail(bridi_tail, term) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalZantufaGroupedBridiTail).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "bridi tail" bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
        field ke_continuation <- opt(arc(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    rule "bridi tail" bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
        assert !(relation_connective_as_bridi_tail, opt(arc(tense_modal)), cmavo(Ke));
        field ke_continuation <- opt(arc(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
    }

    rule "bridi tail" afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail_without_tail_terms),
            zero_or_more: bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal),
            element: bridi_tail,
        );
    }

    rule "bridi tail" afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> struct {
        field bridi_tails <- chain(
            first: arc(bo_grouped_bridi_tail),
            zero_or_more: bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal),
            element: bridi_tail,
        );
    }

    rule "bridi tail" bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
    }

    rule "bridi tail" bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> struct {
        field first <- arc(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
        field bo_continuation <- opt(arc(bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal)));
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
        field connection <- arc(forethought_bridi_connection_without_tail_terms);
    }

    rule "forethought bridi connection" forethought_simple_bridi_tail(forethought_bridi_connection) -> struct {
        field connection <- arc(forethought_bridi_connection);
    }

    rule "bridi tail" selbri_simple_bridi_tail_without_tail_terms(selbri) -> struct {
        field selbri <- arc(selbri);
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "bridi tail" selbri_simple_bridi_tail(selbri, term) -> struct {
        field selbri <- arc(selbri);
        field terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
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
        field first <- arc(subbridi);
        field first_branch <- forethought_bridi_branch(subbridi);
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "forethought bridi connection" direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field first <- arc(subbridi);
        field first_branch <- forethought_bridi_branch(subbridi);
        field additional_branches <- [zero_or_more zantufa_forethought_bridi_branch(subbridi)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "forethought bridi branch" forethought_bridi_branch(subbridi) -> struct {
        field gik <- gik_connective;
        field branch <- arc(subbridi);
    }

    rule "forethought bridi branch" zantufa_forethought_bridi_branch(subbridi) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field branch <- arc(subbridi);
    }

    rule "forethought bridi connection" grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- arc(forethought_bridi_connection);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    rule "forethought bridi connection" grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
    }

    rule "forethought bridi connection" negated_forethought_bridi_connection(forethought_bridi_connection) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- arc(forethought_bridi_connection);
    }

    rule "forethought bridi connection" negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> struct {
        field na <- selmaho(Na).wf();
        field inner <- arc(forethought_bridi_connection_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "bridi tail connective" gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> struct {
        field connective <- gihek_connective();
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field bridi_tail <- arc(bridi_tail);
        field kehe <- opt(arc(cmavo(Kehe).wf())).elidable_terminator(Kehe);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "bridi tail connective" bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        field connective <- bridi_tail_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "bridi tail connective" bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail_without_tail_terms);
    }

    rule "bridi tail connective" bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> struct {
        assert !(bridi_tail_connective, opt(arc(tense_modal)), choice((cmavo(Bo), cmavo(Ke))));
        field connective <- bridi_tail_connective;
        field cu <- opt(arc(cmavo(Cu).wf()));
        field bridi_tail <- arc(bo_grouped_bridi_tail);
        field tail_terms <- [zero_or_more term];
        field vau <- opt(arc(cmavo(Vau).wf())).elidable_terminator(Vau);
    }

    rule "subbridi" subbridi(subbridi, bridi, term) -> enum {
        prenex_subbridi,
        bridi_subbridi,
    }

    rule "subbridi" bridi_subbridi(bridi) -> struct {
        field bridi <- arc(bridi);
    }

    rule "prenex" prenex_subbridi(subbridi, term) -> struct {
        field prenex_terms <- [zero_or_more term];
        field zohu <- cmavo(Zohu).wf();
        field inner_subbridi <- arc(subbridi);
    }

    alias "term" term_guard =
        (relation_word(), cmavo(Bu).not()).not();

    rule "term" term(statement, term, sumti, tense_modal, subbridi, selbri, free_modifier) -> enum {
        pehe_termset_connection,
        bound_term_connection,
        termset_group,
        connected_term,
        simple_term,
    }

    rule "termset connection" pehe_termset_connection(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [one_or_more pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    rule "termset connection continuation" pehe_termset_connection_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field pehe <- cmavo(Pehe).wf();
        field connective <- statement_connective;
        field trailing_term <- arc(pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    rule "term" pehe_termset_operand(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
        bound_term_connection,
        termset_group,
        simple_term,
    }

    rule "term" simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> enum {
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

    rule "term connective" bound_term_connective -> enum {
        joik_connective,
        ek_connective,
    }

    rule "term connection" connected_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [zero_or_more connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    rule "term connection continuation" connected_term_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field connective <- connected_term_connective;
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    rule "term connective" connected_term_connective -> enum {
        joik_connective,
        jek_connective,
        ek_connective,
        vuhu_nonlogical_connective,
    }

    rule "termset" termset_group(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        assert term_guard();
        field leading_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
        field continuations <- [one_or_more termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier)];
    }

    rule "termset continuation" termset_group_continuation(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier) -> struct {
        field cehe <- cmavo(Cehe).wf();
        field trailing_term <- arc(simple_term(statement, sumti, tense_modal, subbridi, selbri, term, free_modifier));
    }

    rule "termset" forethought_termset(term, tense_modal) -> struct {
        field m_nuhi <- opt(cmavo(Nuhi).wf());
        field gek <- modal_forethought_connective(tense_modal);
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
        field first_branch <- forethought_termset_branch(term);
        field additional_branches <- [zero_or_more zantufa_forethought_termset_branch(term)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    rule "termset" forethought_termset_branch(term) -> struct {
        field gik <- gik_connective;
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    rule "termset" zantufa_forethought_termset_branch(term) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field terms <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    rule "termset" nuhi_termset(term) -> struct {
        field nuhi <- cmavo(Nuhi).wf();
        field termset <- [one_or_more arc(term)];
        field nuhu <- opt(cmavo(Nuhu).wf()).elidable_terminator(Nuhu);
    }

    rule "termset" ke_termset(term) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalKeTermset).wf();
        field termset <- [one_or_more arc(term)];
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "NOIhA adverbial" noiha_adverbial_term(free_modifier, selbri) -> enum {
        noiha_variable_adverbial_term,
        noiha_relative_adverbial_term,
    }

    rule "NOIhA adverbial" noiha_variable_adverbial_term(free_modifier, selbri) -> struct {
        field poiha <- selmaho(Noiha).wf();
        field free_modifiers <- [zero_or_more free_modifier];
        field selbri <- arc(selbri);
        field brigahi_ku <- cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
    }

    rule "NOIhA adverbial" noiha_relative_adverbial_term(selbri) -> struct {
        field noiha <- selmaho(Noiha).wf();
        field selbri <- arc(selbri);
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    rule "FIhOI adverbial" fihoi_adverbial_term(statement) -> struct {
        field fihoi <- cmavo(Fihoi).warn(ExperimentalFihoiAdverbial).wf();
        field statement <- arc(statement);
        field fihau <- opt(cmavo(Fihau).wf()).elidable_terminator(Fihau);
    }

    rule "SOI adverbial" soi_adverbial_term(statement) -> struct {
        field soi <- selmaho(Soi).warn(ExperimentalSoiAdverbial).wf();
        field statement <- arc(statement);
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    rule "term" sumti_term(sumti) -> struct {
        field sumti <- arc(sumti);
    }

    rule "place tag" place_tagged_sumti_term(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- arc(tagged_or_elided_sumti(sumti));
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
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert tense_modal.lookahead();
    }

    rule "tag" tagged_sumti_term(tense_modal, sumti, selbri) -> struct {
        assert !modal_forethought_connective(tense_modal);
        field tense_modal <- arc(leading_term_tag_tense_modal(tense_modal, selbri));
        assert !selbri;
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    rule "tag" jai_tagged_sumti_term(tense_modal, sumti) -> struct {
        assert feature(ZantufaTags);
        field jai <- cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
        field tag <- opt(arc(tense_modal));
        field sumti <- arc(sumti);
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

    rule "sumti" tagged_or_elided_sumti(sumti) -> enum {
        sumti,
        tagged_elided_sumti,
    }

    rule "elided sumti" tagged_elided_sumti -> struct {
        field maybe_ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    rule "sumti" sumti(sumti, sumti_grouped, subbridi, tense_modal, statement) -> struct {
        field base_sumti <- arc(sumti_grouped);
        field vuho_attachment <- opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement));
    }

    rule "sumti connection" sumti_grouped(sumti, sumti_afterthought, tense_modal, statement) -> struct {
        field leading_sumti <- arc(sumti_afterthought);
        field grouped_tail <- opt(grouped_sumti_tail(sumti, tense_modal));
    }

    rule "sumti connection" sumti_afterthought(sumti_bound, statement) -> struct {
        field leading_sumti <- arc(sumti_bound);
        field continuations <- [zero_or_more sumti_afterthought_tail(sumti_bound)];
    }

    rule "sumti connection" sumti_bound(sumti_bound, sumti_forethought, tense_modal, statement) -> struct {
        field leading_sumti <- arc(sumti_forethought);
        field bound_tail <- opt(bound_sumti_tail(sumti_bound, tense_modal));
    }

    rule "sumti" sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        forethought_sumti,
        simple_sumti,
    }

    rule "forethought sumti connection" forethought_sumti(sumti, sumti_forethought, tense_modal, statement) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field leading_sumti <- arc(sumti);
        field first_branch <- forethought_sumti_branch(sumti_forethought);
        field additional_branches <- [zero_or_more zantufa_forethought_sumti_branch(sumti_forethought)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    rule "forethought sumti connection" forethought_sumti_branch(sumti_forethought) -> struct {
        field gik <- gik_connective;
        field sumti <- arc(sumti_forethought);
    }

    rule "forethought sumti connection" zantufa_forethought_sumti_branch(sumti_forethought) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field sumti <- arc(sumti_forethought);
    }

    rule "sumti connection" bound_sumti_tail(sumti_bound, tense_modal) -> struct {
        field connective <- arc(argument_connective);
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_sumti <- arc(sumti_bound);
    }

    rule "sumti connective" sumti_afterthought_tail(sumti_bound) -> struct {
        field connective <- argument_connective;
        field sumti <- arc(sumti_bound);
    }

    rule "sumti connection" grouped_sumti_tail(sumti, tense_modal) -> struct {
        field connective <- argument_connective;
        field tense_modal <- opt(arc(tense_modal));
        field ke <- cmavo(Ke).wf();
        field inner_sumti <- arc(sumti);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "sumti relative phrase" vuho_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> enum {
        vuho_relative_sumti_attachment_tail,
        vuho_connected_sumti_attachment_tail,
    }

    rule "sumti relative phrase" vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field relative_clauses <- relative_clause_list(sumti, subbridi, tense_modal, statement);
        field sumti_connection <- opt(arc(sumti_connection_tail(sumti)));
    }

    rule "sumti relative phrase" vuho_connected_sumti_attachment_tail(sumti) -> struct {
        field vuho <- cmavo(Vuho).wf();
        field sumti_connection <- arc(sumti_connection_tail(sumti));
    }

    rule "sumti" simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> struct {
        field base_sumti <- arc(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement));
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "sumti" sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens, statement) -> enum {
        sumti_base,
        quantified_sumti,
    }

    rule "sumti" sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier, statement) -> enum {
        scalar_negated_sumti_with_bo,
        scalar_negated_sumti,
        lahe_sumti,
        lahe_term_wrapper,
        scalar_negated_term_wrapper_with_bo,
        scalar_negated_term_wrapper,
        bridi_description_sumti,
        name_sumti,
        description_connection_sumti,
        descriptor_with_outer_quantifier_sumti,
        descriptor_with_gadri_sumti,
        descriptor_without_gadri_sumti,
        number_sumti,
        lerfu_string_sumti,
        quoted_sumti,
        pro_sumti,
    }

    rule "quantified sumti" quantified_sumti(sumti_base, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        field inner_sumti <- arc(sumti_base);
    }

    rule "sumti connective" sumti_connection_tail(sumti) -> struct {
        field connective <- argument_connective;
        field sumti <- arc(sumti);
    }

    rule "quantifier" pa_run_quantifier(letter_tokens) -> struct {
        field number <- number_words(letter_tokens).wf();
        field boi <- opt(cmavo(Boi).wf()).elidable_terminator(Boi);
    }

    rule "quantifier" mekso_quantifier(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field mekso <- arc(mekso);
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    rule "quantifier" zantufa_raw_mekso_quantifier(mekso) -> struct {
        field mekso <- arc(mekso);
    }

    rule "quantifier" zantufa_priority_raw_mekso_quantifier(mekso) -> struct {
        field mekso <- arc(mekso);
    }

    rule "quantifier" quantifier(mekso, letter_tokens) -> enum {
        when feature(ZantufaMex) zantufa_priority_raw_mekso_quantifier,
        mekso_quantifier,
        pa_run_quantifier,
        when feature(ZantufaMex) zantufa_raw_mekso_quantifier,
    }

    rule "number mex" number_mekso(letter_tokens) -> struct {
        field quantifier <- arc(pa_run_quantifier(letter_tokens));
    }

    rule "VUhU operator" primitive_mekso_operator -> struct {
        field vuhu <- selmaho(Vuhu).wf();
    }

    rule "operator" mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        afterthought_mekso_operator,
        bound_mekso_operator,
        simple_mekso_operator,
    }

    rule "operator" afterthought_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        field operators <- chain(
            first: arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri)),
            zero_or_more: afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri),
            element: trailing_operator,
        );
    }

    rule "operator continuation" afterthought_mekso_operator_continuation(mekso, mekso_operator, sumti, selbri) -> struct {
        field connective <- standard_statement_connective;
        field trailing_operator <- arc(bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri));
    }

    rule "operator" bound_or_atom_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        bound_mekso_operator,
        simple_mekso_operator,
    }

    rule "operator" bound_mekso_operator(mekso, mekso_operator, sumti, selbri) -> struct {
        field left_operator <- arc(simple_mekso_operator(mekso, mekso_operator, sumti, selbri));
        field connective <- standard_statement_connective;
        field bo <- cmavo(Bo).wf();
        field right_operator <- arc(mekso_operator);
    }

    rule "operator" simple_mekso_operator(mekso, mekso_operator, sumti, selbri) -> enum {
        converted_mekso_operator,
        scalar_negated_mekso_operator,
        forethought_mekso_operator,
        grouped_mekso_operator,
        selbri_mekso_operator,
        operand_mekso_operator,
        when feature(ZantufaMex) zantufa_maho_selbri_mekso_operator,
        when feature(ZantufaMex) zantufa_maho_sumti_mekso_operator,
        when feature(ZantufaMex) zantufa_connective_mekso_operator,
        primitive_mekso_operator,
    }

    rule "converted operator" converted_mekso_operator(mekso_operator) -> struct {
        field se <- selmaho(Se).wf();
        field inner_operator <- arc(mekso_operator);
    }

    rule "converted operator" scalar_negated_mekso_operator(mekso_operator) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_operator <- arc(mekso_operator);
    }

    rule "operator" forethought_mekso_operator(mekso_operator) -> struct {
        field guhek <- guhek_connective;
        field left_operator <- arc(mekso_operator);
        field gik <- gik_connective;
        field right_operator <- arc(mekso_operator);
    }

    rule "grouped operator" grouped_mekso_operator(mekso_operator) -> struct {
        field ke <- cmavo(Ke).wf();
        field inner_operator <- arc(mekso_operator);
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "selbri-to-operator" selbri_mekso_operator(selbri) -> struct {
        field nahu <- cmavo(Nahu).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "operand-to-operator" operand_mekso_operator(mekso) -> struct {
        field maho <- cmavo(Maho).wf();
        field mekso <- arc(mekso);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "selbri-to-operator" zantufa_maho_selbri_mekso_operator(selbri) -> struct {
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "sumti-to-operator" zantufa_maho_sumti_mekso_operator(sumti) -> struct {
        field maho <- cmavo(Maho).warn(ExperimentalZantufaMex).wf();
        field sumti <- arc(sumti);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "connective operator" zantufa_connective_mekso_operator -> struct {
        field connective <- arc(operand_connective);
        assert !cmavo(Cu);
    }

    rule "operand" mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        afterthought_mekso_operand,
        bound_mekso_operand,
        simple_mekso_operand,
    }

    rule "operand connective" afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field operands <- chain(
            first: arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier)),
            zero_or_more: afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            element: trailing_expression,
        );
    }

    rule "operand continuation" afterthought_mekso_operand_continuation(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field operand_connective <- operand_connective;
        field trailing_expression <- arc(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
    }

    rule "operand" bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> enum {
        bound_mekso_operand,
        simple_mekso_operand,
    }

    rule "operand connective" bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> struct {
        field left_expression <- arc(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
        field operand_connective <- operand_connective;
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field right_expression <- arc(mekso_operand);
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
        when feature(ZantufaMex) zantufa_scalar_negated_mekso_operand,
        when feature(ZantufaMex) zantufa_selbri_mohe_mekso_operand,
    }

    rule "scalar-negated operand" zantufa_scalar_negated_mekso_operand(mekso_operand) -> struct {
        field nahe <- selmaho(Nahe).warn(ExperimentalZantufaMex).wf();
        field inner_expression <- arc(mekso_operand);
    }

    rule "qualified operand" qualified_mekso_operand(mekso_operand) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo);
        field inner_expression <- arc(mekso_operand);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "forethought mex" forethought_mekso_operand(mekso_operand, tense_modal) -> struct {
        field gek <- modal_forethought_connective(tense_modal);
        field left_expression <- arc(mekso_operand);
        field gik <- gik_connective;
        field right_expression <- arc(mekso_operand);
    }

    rule "sumti operand" sumti_mekso_operand(sumti) -> struct {
        field mohe <- cmavo(Mohe).wf();
        field sumti <- arc(sumti);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "selbri operand" zantufa_selbri_mohe_mekso_operand(selbri) -> struct {
        field mohe <- cmavo(Mohe).warn(ExperimentalZantufaMex).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "selbri operand" selbri_mekso_operand(selbri) -> struct {
        field nihe <- cmavo(Nihe).wf();
        field selbri <- arc(selbri);
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "parenthesized mex" parenthesized_mekso_operand(mekso) -> struct {
        field vei <- cmavo(Vei).wf();
        field inner_expression <- arc(mekso);
        field veho <- opt(cmavo(Veho).wf()).elidable_terminator(Veho);
    }

    rule "mekso array" array_mekso_operand(mekso) -> struct {
        field johi <- cmavo(Johi).wf();
        field expressions <- [one_or_more mekso];
        field tehu <- opt(cmavo(Tehu).wf()).elidable_terminator(Tehu);
    }

    rule "lerfu string" letter_string(letter_tokens) -> struct {
        field first_letter <- arc(letter_tokens);
        field continuations <- [zero_or_more letter_string_continuation(letter_tokens)];
    }

    rule "lerfu string continuation" letter_string_continuation(letter_tokens) -> enum {
        letter_string_pa_continuation,
        letter_string_lerfu_continuation,
    }

    rule "lerfu string continuation" letter_string_pa_continuation -> struct {
        field pa <- pa_word();
    }

    rule "lerfu string continuation" letter_string_lerfu_continuation(letter_tokens) -> struct {
        field letter <- arc(letter_tokens);
    }

    rule "number" number_words(letter_tokens) -> struct {
        field first_number <- pa_word();
        field continuations <- [zero_or_more number_word_continuation(letter_tokens)];
    }

    rule "number continuation" number_word_continuation(letter_tokens) -> enum {
        number_word_pa_continuation,
        number_word_lerfu_continuation,
    }

    rule "number continuation" number_word_pa_continuation -> struct {
        field pa <- pa_word();
    }

    rule "number continuation" number_word_lerfu_continuation(letter_tokens) -> struct {
        field letter <- arc(letter_tokens);
    }

    rule "number or lerfu string" number_or_letter_words(letter_tokens, letter_string) -> enum {
        number_words,
        letter_string,
    }

    rule "lerfu word" letter_tokens(letter_string, letter_tokens) -> enum {
        simple_lerfu_word,
        lau_lerfu_word,
        tei_lerfu_word,
    }

    rule "lerfu word" simple_lerfu_word -> struct {
        field word <- word_category(LetterWord);
    }

    rule "lerfu word" lau_lerfu_word(letter_tokens) -> struct {
        field lau <- selmaho(Lau);
        field letter <- arc(letter_tokens);
    }

    rule "lerfu word" tei_lerfu_word(letter_string) -> struct {
        field tei <- cmavo(Tei);
        field letters <- arc(letter_string);
        field foi <- cmavo(Foi);
    }

    rule "lerfu string" lerfu_string_mekso(letter_string, free_modifier) -> struct {
        field letters <- letter_string;
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        field free_modifiers <- [zero_or_more free_modifier];
    }

    rule "mex" mekso_base(mekso, mekso_base, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier, mekso_operator) -> enum {
        when feature(ZantufaMex) zantufa_bo_grouped_mekso_base,
        mekso_operand,
        forethought_call_mekso,
        when feature(ZantufaMex) zantufa_grouped_mekso_operand_sequence,
    }

    rule "grouped mex" zantufa_bo_grouped_mekso_base(mekso_operand) -> struct {
        field first <- arc(mekso_operand);
        field continuations <- [one_or_more zantufa_bo_grouped_mekso_continuation(mekso_operand)];
    }

    rule "grouped mex" zantufa_bo_grouped_mekso_continuation(mekso_operand) -> struct {
        field bo <- cmavo(Bo).warn(ExperimentalZantufaMex).wf();
        field expression <- arc(mekso_operand);
    }

    rule "grouped mex" zantufa_grouped_mekso_operand_sequence(mekso_operand) -> struct {
        field ke <- cmavo(Ke).warn(ExperimentalZantufaMex).wf();
        field operands <- [one_or_more arc(mekso_operand)];
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "mex" mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field left_expression <- arc(mekso_base);
        field tail <- opt(mekso_precedence_tail(mekso_precedence, mekso_operator));
    }

    rule "mex precedence tail" mekso_precedence_tail(mekso_precedence, mekso_operator) -> struct {
        field bihe <- cmavo(Bihe).wf();
        field operator <- arc(mekso_operator);
        field right_expression <- arc(mekso_precedence);
    }

    rule "mex" infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        field continuations <- [zero_or_more infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    rule "mex continuation" infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        field operator <- arc(mekso_operator);
        field right_expression <- arc(mekso_precedence);
    }

    rule "mex" zantufa_infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> struct {
        field first_expression <- arc(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
        field continuations <- [zero_or_more zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator)];
    }

    rule "mex continuation" zantufa_infix_mekso_continuation(mekso_precedence, mekso_operator) -> struct {
        field operators <- [one_or_more arc(mekso_operator)];
        field right_expression <- opt(arc(mekso_precedence));
    }

    rule "forethought mex" forethought_call_mekso(mekso_base, mekso_operator) -> struct {
        field peho <- opt(cmavo(Peho).wf());
        field operator <- arc(mekso_operator);
        field operands <- [one_or_more mekso_base];
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    rule "mex" mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts) -> enum {
        when feature(ZantufaMex) zantufa_reverse_polish_mekso,
        when feature(ZantufaMex) zantufa_infix_mekso,
        infix_mekso,
        reverse_polish_mekso,
    }

    rule "reverse Polish mex" zantufa_reverse_polish_mekso(mekso_base, mekso_operator) -> struct {
        field fuha <- cmavo(Fuha).warn(ExperimentalZantufaMex).wf();
        field operands <- [one_or_more mekso_base];
        field operator <- arc(mekso_operator);
        field tails <- [zero_or_more zantufa_reverse_polish_tail(mekso_base, mekso_operator)];
        field kuhe <- opt(cmavo(Kuhe).wf()).elidable_terminator(Kuhe);
    }

    rule "reverse Polish mex tail" zantufa_reverse_polish_tail(mekso_base, mekso_operator) -> struct {
        field operands <- [zero_or_more mekso_base];
        field operator <- arc(mekso_operator);
    }

    rule "reverse Polish mex" reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> struct {
        field first_operand <- arc(mekso_operand);
        field tails <- [zero_or_more reverse_polish_parts_tail(reverse_polish_parts, mekso_operator)];
    }

    rule "reverse Polish mex tail" reverse_polish_parts_tail(reverse_polish_parts, mekso_operator) -> struct {
        field right_parts <- arc(reverse_polish_parts);
        field operator <- mekso_operator;
    }

    rule "reverse Polish mex" reverse_polish_mekso(reverse_polish_parts) -> struct {
        field fuha <- cmavo(Fuha).wf();
        field parts <- arc(reverse_polish_parts);
    }

    rule "number sumti" number_sumti(mekso) -> struct {
        field li <- selmaho(Li).wf();
        #[tree_child(primary)]
        field expression <- arc(mekso);
        field loho <- opt(cmavo(Loho).wf()).elidable_terminator(Loho);
    }

    rule "lerfu string" lerfu_string_sumti(letter_string, free_modifier) -> struct {
        field words <- letter_string;
        assert !selmaho(Moi);
        assert !selmaho(Mai);
        field boi <- opt(cmavo(Boi)).elidable_terminator(Boi);
        field free_modifiers <- [zero_or_more free_modifier];
    }

    rule "converted sumti" lahe_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        field lahe <- selmaho(Lahe).wf();
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "converted term" lahe_term_wrapper(term) -> struct {
        field lahe <- selmaho(Lahe).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "scalar-negated term" scalar_negated_term_wrapper_with_bo(term) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "scalar-negated term" scalar_negated_term_wrapper(term) -> struct {
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        field inner_term <- arc(term);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "scalar-negated sumti" scalar_negated_sumti_with_bo(sumti) -> struct {
        field nahe <- selmaho(Nahe);
        field bo <- cmavo(Bo).wf();
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "scalar-negated sumti" scalar_negated_sumti(sumti) -> struct {
        field nahe <- selmaho(Nahe).wf();
        #[tree_child(primary)]
        field inner_sumti <- arc(sumti);
        field luhu <- opt(cmavo(Luhu).wf()).elidable_terminator(Luhu);
    }

    rule "bridi description" bridi_description_sumti(statement) -> struct {
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
        field additional_heads <- [zero_or_more lohoi_description_head_continuation()];
        #[tree_child(primary)]
        field statement <- arc(statement);
        field kuhau <- opt(cmavo(Kuhau).wf()).elidable_terminator(Kuhau);
    }

    rule "bridi description" lohoi_description_head_continuation -> struct {
        field connective <- joik_connective;
        field lohoi <- selmaho(Lohoi).warn(ExperimentalLohOiBridiDescription).wf();
    }

    rule "sumti" pro_sumti -> struct {
        field koha <- word_category(ProSumti).wf();
    }

    rule "name" name_sumti -> struct {
        field la <- selmaho(La).wf();
        field names <- [one_or_more cmevla_word()].wf();
    }

    rule "descriptor" description_head -> struct {
        field description <- choice((selmaho(Le), selmaho(La))).wf();
    }

    rule "descriptor connective" description_head_connective -> struct {
        field connective <- arc(jek_connective);
    }

    rule "description" description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field leading_description_head <- arc(description_head());
        field connective <- description_head_connective();
        field trailing_description_head <- arc(description_head());
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    rule "description" descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field description <- description_head();
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    rule "description" descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens, statement) -> struct {
        field outer_quantifier <- quantifier(mekso, letter_tokens);
        field description <- description_head();
        field tail <- description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
    }

    rule "description" descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        #[tree_child(primary)]
        field selbri <- arc(selbri);
        field ku <- opt(cmavo(Ku).wf()).elidable_terminator(Ku);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "description tail" description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field leading_tail_elements <- leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement);
        field tail <- arc(description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement));
    }

    rule "description tail" description_tail_body(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> enum {
        quantifier_relation_description_tail,
        quantifier_sumti_description_tail,
        relation_description_tail,
    }

    rule "description tail" leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal, statement) -> struct {
        field tail_sumti <- opt(description_tail_sumti(sumti_base));
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "description tail" description_tail_sumti(sumti_base) -> struct {
        assert !pa_word();
        field sumti <- arc(sumti_base);
    }

    rule "description tail" relation_description_tail(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field selbri <- arc(selbri);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "description tail" quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens, statement) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        assert !selmaho(Roi);
        field selbri <- arc(selbri);
        field relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "description tail" quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> struct {
        field quantifier <- quantifier(mekso, letter_tokens);
        field sumti <- arc(sumti);
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
        field text <- arc(text);
        field lihu <- opt(cmavo(Lihu).wf()).elidable_terminator(Lihu);
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

    rule "quote" quoted_sumti(text) -> struct {
        #[tree_child(primary)]
        field quote <- arc(quote(text));
    }

    rule "vocative phrase" selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        #[tree_child(primary)]
        field selbri <- arc(selbri);
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "vocative phrase" cmevla_vocative_sumti(sumti, subbridi, tense_modal, statement) -> struct {
        field leading_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
        field names <- [one_or_more cmevla_word()].wf();
        field trailing_relative_clauses <- opt(relative_clause_list(sumti, subbridi, tense_modal, statement));
    }

    rule "vocative phrase" vocative_sumti(sumti, subbridi, selbri, tense_modal, statement) -> enum {
        selbri_vocative_sumti,
        cmevla_vocative_sumti,
        sumti,
    }

    rule "vocative marker" vocative_marker_words -> enum {
        coi_vocative_marker_words,
        doi_vocative_marker_words,
    }

    rule "vocative marker" coi_vocative_marker_words -> struct {
        field first_coi <- selmaho(Coi);
        field first_nai <- opt(cmavo(Nai));
        field additional_coi <- [zero_or_more additional_coi_vocative_marker()];
        field doi <- opt(cmavo(Doi));
    }

    rule "vocative marker" additional_coi_vocative_marker -> struct {
        field coi <- selmaho(Coi);
        field nai <- opt(cmavo(Nai));
    }

    rule "vocative marker" doi_vocative_marker_words -> struct {
        field doi <- cmavo(Doi);
    }

    rule "free modifier" free_modifier(sumti, subbridi, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier, statement) -> enum {
        text_replacement_free_modifier,
        when feature(ZantufaTerms) zantufa_sei_statement_free_modifier,
        sei_free_modifier,
        xi_free_modifier,
        mai_free_modifier,
        when feature(ZantufaMex) zantufa_mekso_mai_free_modifier,
        soi_free_modifier,
        parenthetical_text,
        vocative_free_modifier,
    }

    rule "vocative phrase" vocative_free_modifier(sumti, subbridi, selbri, tense_modal, statement) -> struct {
        field vocative_markers <- vocative_marker_words().wf();
        field sumti <- opt(arc(vocative_sumti(sumti, subbridi, selbri, tense_modal, statement)));
        field dohu <- opt(cmavo(Dohu).prohibited_wf()).elidable_terminator(Dohu);
    }

    rule "parenthetical text" parenthetical_text(text) -> struct {
        field to <- selmaho(To).wf();
        field text <- arc(text);
        field toi <- opt(cmavo(Toi).prohibited_wf()).elidable_terminator(Toi);
    }

    rule "metalinguistic comment" sei_free_modifier(term, selbri) -> struct {
        field sei <- selmaho(Sei).wf();
        field terms <- [zero_or_more term];
        field cu <- opt(cmavo(Cu).wf());
        field selbri <- arc(selbri);
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    rule "metalinguistic comment" zantufa_sei_statement_free_modifier(statement) -> struct {
        field sei <- selmaho(Sei).warn(ExperimentalZantufaStatementFreeModifier).wf();
        field statement <- arc(statement);
        field sehu <- opt(cmavo(Sehu).prohibited_wf()).elidable_terminator(Sehu);
    }

    rule "subscript" xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier) -> enum {
        xi_number_free_modifier,
        xi_lerfu_string_free_modifier,
        xi_parenthesized_free_modifier,
    }

    rule "subscript" xi_number_free_modifier(letter_tokens) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(number_mekso(letter_tokens));
    }

    rule "subscript" xi_lerfu_string_free_modifier(letter_string, free_modifier) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(lerfu_string_mekso(letter_string, free_modifier));
    }

    rule "subscript" xi_parenthesized_free_modifier(mekso) -> struct {
        field xi <- selmaho(Xi).wf();
        field expression <- arc(parenthesized_mekso_operand(mekso));
    }

    rule "utterance ordinal" mai_free_modifier(letter_tokens, letter_string) -> struct {
        field number <- number_or_letter_words(letter_tokens, letter_string)
            .followed_by(selmaho(Mai).ignored());
        field mai <- selmaho(Mai).wf();
    }

    rule "utterance ordinal" zantufa_mekso_mai_free_modifier(mekso) -> struct {
        field expression <- arc(mekso.followed_by(selmaho(Mai).ignored()));
        field mai <- selmaho(Mai).warn(ExperimentalZantufaMex).wf();
    }

    rule "reciprocal" soi_free_modifier(sumti) -> struct {
        field soi <- cmavo(Soi).wf();
        field leading_sumti <- arc(sumti);
        field trailing_sumti <- opt(arc(sumti));
        field sehu <- opt(cmavo(Sehu).wf()).elidable_terminator(Sehu);
    }

    rule "replacement phrase" text_replacement_free_modifier -> enum {
        full_text_replacement_free_modifier,
        new_only_text_replacement_free_modifier,
        close_only_text_replacement_free_modifier,
    }

    alias "replacement free modifier word" word_before_sahai_or_lehai =
        word_not_cmavo(Sahai, Lehai);

    alias "replacement free modifier word" word_before_lehai =
        word_not_cmavo(Lehai);

    rule "replacement phrase" full_text_replacement_free_modifier -> struct {
        field lohai <- cmavo(Lohai);
        field old_words <- [zero_or_more word_before_sahai_or_lehai()];
        field sahai <- opt(cmavo(Sahai));
        field new_words <- [zero_or_more word_before_lehai()];
        field lehai <- cmavo(Lehai).wf();
    }

    rule "replacement phrase" new_only_text_replacement_free_modifier -> struct {
        field sahai <- cmavo(Sahai);
        field new_words <- [zero_or_more word_before_lehai()];
        field lehai <- cmavo(Lehai).wf();
    }

    rule "replacement phrase" close_only_text_replacement_free_modifier -> struct {
        field lehai <- cmavo(Lehai).wf();
    }

    rule "relative clauses" relative_clause_tail(sumti, subbridi, tense_modal, statement) -> enum {
        joined_relative_clause_tail,
        connected_relative_clause_tail,
    }

    rule "relative clause" joined_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field zihe <- cmavo(Zihe).wf();
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    rule "relative clause" connected_relative_clause_tail(sumti, subbridi, tense_modal, statement) -> struct {
        field connective <- relative_clause_connective;
        field inner <- arc(relative_clause_atom(sumti, subbridi, tense_modal, statement));
    }

    rule "relative clause connective" relative_clause_connective -> enum {
        joik_connective,
        jek_connective,
    }

    rule "relative clause" relative_clause_atom(sumti, subbridi, tense_modal, statement) -> enum {
        sumti_association_relative_clause,
        bridi_relative_clause,
    }

    rule "sumti association phrase" sumti_association_relative_clause(sumti, tense_modal) -> struct {
        field association_marker <- selmaho(Goi).wf();
        field sumti <- arc(relative_sumti(sumti, tense_modal));
        field gehu <- opt(cmavo(Gehu).wf()).elidable_terminator(Gehu);
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
        field tense_modal <- arc(tense_modal);
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    rule "sumti association phrase" plain_relative_sumti(sumti) -> struct {
        field sumti <- arc(sumti);
    }

    rule "relative bridi" bridi_relative_clause(subbridi, statement) -> enum {
        when feature(ZantufaTerms) zantufa_restrictive_statement_relative_clause,
        when feature(ZantufaTerms) zantufa_incidental_statement_relative_clause,
        restrictive_bridi_relative_clause,
        incidental_bridi_relative_clause,
    }

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

    rule "relative clause" zantufa_incidental_statement_relative_clause(statement) -> struct {
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).warn(ExperimentalZantufaStatementRelativeClause).wf();
        field statement <- arc(statement);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

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

    rule "relative clause" incidental_bridi_relative_clause(subbridi, statement) -> struct {
        field noi <- choice((
            cmavo(Noi),
            cmavo(Nohoi),
        )).wf();
        field subbridi <- arc(subbridi);
        field kuho <- opt(cmavo(Kuho).wf()).elidable_terminator(Kuho);
    }

    rule "ek" ek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field a <- selmaho(A).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "ek" jehi_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field jehi <- selmaho(Jehi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "jek" jek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "joik" joik_connective -> enum {
        joi_connective,
        simple_interval_connective,
        closed_interval_connective,
    }

    rule "joik" joi_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field joi <- selmaho(Joi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "interval" simple_interval_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

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

    rule "non-logical connective" vuhu_nonlogical_connective -> struct {
        #[tree_child(primary)]
        field vuhu <- selmaho(Vuhu).wf();
    }

    rule "sumti connective" argument_connective -> enum {
        cehe_connective,
        ek_connective,
        jehi_connective,
        joik_connective,
        vuhu_nonlogical_connective,
    }

    rule "operand connective" operand_connective -> enum {
        joik_connective,
        ek_connective,
        jek_connective,
    }

    rule "selbri connective" relation_afterthought_connective -> enum {
        joik_connective,
        jek_connective,
        ek_connective,
        vuhu_nonlogical_connective,
    }

    rule "statement connective" standard_statement_connective -> enum {
        joik_connective,
        jek_connective,
    }

    rule "statement connective" statement_connective -> enum {
        joik_connective,
        jek_connective,
        ek_connective,
        vuhu_nonlogical_connective,
    }

    rule "text connective" text_leading_connective -> enum {
        standard_statement_connective,
        cehe_connective,
    }

    rule "statement connective" i_statement_connective(tense_modal) -> enum {
        i_standard_statement_connective,
        i_tag_bo_statement_connective,
    }

    rule "statement connective" i_standard_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        field connective <- arc(statement_connective);
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo).wf()));
    }

    rule "statement connective" i_paragraph_statement_connective(tense_modal) -> enum {
        i_standard_paragraph_statement_connective,
        i_tag_bo_paragraph_statement_connective,
    }

    rule "statement connective" i_standard_paragraph_statement_connective(tense_modal) -> struct {
        #[tree_child(primary)]
        field connective <- arc(paragraph_standard_statement_connective);
        field tag_bo <- opt((opt(arc(tense_modal)), cmavo(Bo)));
    }

    rule "statement connective" paragraph_standard_statement_connective -> enum {
        paragraph_joi_connective,
        paragraph_simple_interval_connective,
        paragraph_closed_interval_connective,
        paragraph_jek_connective,
    }

    rule "jek" paragraph_jek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja);
        field nai <- opt(cmavo(Nai));
    }

    rule "joik" paragraph_joi_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field joi <- selmaho(Joi);
        field nai <- opt(cmavo(Nai));
    }

    rule "interval" paragraph_simple_interval_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field bihi <- selmaho(Bihi);
        field nai <- opt(cmavo(Nai));
    }

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

    rule "statement connective" i_tag_bo_paragraph_statement_connective(tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo);
    }

    rule "statement connective" i_tag_bo_statement_connective(tense_modal) -> struct {
        field tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
    }

    rule "termset connective" cehe_connective -> struct {
        #[tree_child(primary)]
        field cehe <- cmavo(Cehe).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "gihek" gihek_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field giha <- selmaho(Giha).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "forethought selbri connective" guhek_connective -> struct {
        field nahe <- opt(selmaho(Nahe));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field guha <- selmaho(Guha).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "bridi tail connective" bridi_tail_connective -> enum {
        gihek_connective,
        relation_connective_as_bridi_tail,
    }

    rule "bridi tail connective" relation_connective_as_bridi_tail -> struct {
        #[tree_child(primary)]
        field connective <- arc(relation_afterthought_connective);
    }

    rule "forethought connective" modal_forethought_connective(tense_modal) -> enum {
        ga_forethought_connective,
        joik_jek_gi_forethought_connective,
        jek_gi_forethought_connective,
        modal_gi_forethought_connective,
        when feature(ZantufaConnectives) zantufa_initial_gi_forethought_connective,
    }

    rule "forethought connective" ga_forethought_connective -> struct {
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ga <- selmaho(Ga).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "forethought connective" zantufa_initial_gi_forethought_connective -> struct {
        field gi <- cmavo(Gi).warn(ExperimentalZantufaGek).wf();
        field tail <- arc(standard_statement_connective);
        field bo <- opt(cmavo(Bo).wf());
    }

    rule "forethought connective" joik_jek_gi_forethought_connective -> struct {
        field connective <- arc(joik_connective);
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    rule "forethought connective" jek_gi_forethought_connective -> struct {
        field na <- opt(selmaho(Na));
        field se <- opt(selmaho(Se));
        #[tree_child(primary)]
        field ja <- selmaho(Ja).warn(ExperimentalZantufaGek).wf();
        field nai <- opt(cmavo(Nai).wf());
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    rule "forethought connective" modal_gi_forethought_connective(tense_modal) -> struct {
        field tense_modal <- arc(tense_modal);
        field gi <- cmavo(Gi).wf();
        field bo <- opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
    }

    rule "forethought connective" gik_connective -> struct {
        #[tree_child(primary)]
        field gi <- cmavo(Gi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "forethought connective" zantufa_extra_gik_connective -> struct {
        #[tree_child(primary)]
        field gi <- cmavo(Gi).warn(ExperimentalZantufaNaryForethought).wf();
    }

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

    rule "tag" tense_modal_body(selbri) -> enum {
        connected_tense_modal,
        tense_modal_atom,
    }

    rule "connected tag" connected_tense_modal(selbri) -> struct {
        field first <- arc(tense_modal_atom(selbri));
        field continuations <- [one_or_more connected_tense_modal_continuation(selbri)];
    }

    rule "connected tag continuation" connected_tense_modal_continuation(selbri) -> struct {
        field connective <- tense_modal_connective;
        field tense_modal <- arc(tense_modal_atom(selbri));
    }

    rule "tag connective" tense_modal_connective -> enum {
        joik_connective,
        jek_connective,
    }

    rule "tag" tense_modal_atom(selbri) -> enum {
        composite_tense,
        fiho_tense,
        modal_tense,
        nahe_se_flat_prefixed_tense,
        se_flat_prefixed_tense,
        fa_flat_tag_tense,
        when feature(ZantufaTags) zantufa_recursive_tag_tense,
        sticky_tense,
    }

    rule "FIhO modal" fiho_tense(selbri) -> struct {
        field fiho <- cmavo(Fiho).wf();
        field selbri <- arc(selbri);
        field fehu <- opt(cmavo(Fehu).wf()).elidable_terminator(Fehu);
    }

    rule "tag" fa_flat_tag_tense -> struct {
        field fa <- selmaho(Fa).warn(ExperimentalFaAsTag).wf();
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
        field modal <- arc(modal_tense());
    }

    rule "tag" composite_flat_tag_atom -> struct {
        field composite <- arc(composite_tense());
    }

    rule "tag" nahe_se_flat_prefixed_tense -> struct {
        field nahe <- selmaho(Nahe).warn(ExperimentalFlattenedTag).wf();
        field se <- opt(selmaho(Se).wf());
        field atom <- flat_tag_atom();
    }

    rule "tag" se_flat_prefixed_tense -> struct {
        field se <- selmaho(Se).warn(ExperimentalFlattenedTag).wf();
        field atom <- flat_tag_atom();
    }

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

    rule "tag" composite_tense -> enum {
        prefixed_time_space_caha_tense,
        time_space_caha_ki_tense,
        cuhe_tense,
    }

    rule "tag" prefixed_time_space_caha_tense -> struct {
        field nahe <- selmaho(Nahe).wf();
        field tense <- arc(time_space_caha_tense);
        field ki <- opt(arc(ki_composite_tense()));
    }

    rule "tag" time_space_caha_ki_tense -> struct {
        field tense <- arc(time_space_caha_tense);
        field ki <- opt(arc(ki_composite_tense()));
    }

    rule "tag" time_space_caha_tense -> enum {
        time_then_space_caha_tense,
        space_then_time_caha_tense,
        caha_tense,
    }

    rule "time tense" time_then_space_caha_tense -> struct {
        field time <- arc(time_tense);
        field space <- opt(arc(space_tense));
        field caha <- opt(arc(caha_tense()));
    }

    rule "space tense" space_then_time_caha_tense -> struct {
        field space <- arc(space_tense);
        field time <- opt(arc(time_tense));
        field caha <- opt(arc(caha_tense()));
    }

    rule "time tense" time_tense -> enum {
        time_tense_with_zi,
        time_tense_with_offset,
        time_tense_with_interval,
        time_tense_with_properties,
    }

    rule "time tense" time_tense_with_zi -> struct {
        field zi <- arc(zi_time_distance_tense());
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    rule "time tense" time_tense_with_offset -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [one_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    rule "time tense" time_tense_with_interval -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- arc(zeha_time_interval_tense());
        field properties <- [zero_or_more arc(interval_property_tense)];
    }

    rule "time tense" time_tense_with_properties -> struct {
        field zi <- opt(arc(zi_time_distance_tense()));
        field offsets <- [zero_or_more arc(pu_time_offset_tense())];
        field zeha <- opt(arc(zeha_time_interval_tense()));
        field properties <- [one_or_more arc(interval_property_tense)];
    }

    rule "interval property" interval_property_tense -> enum {
        numbered_interval_property_tense,
        tahe_interval_property_tense,
        zaho_interval_property_tense,
    }

    rule "interval property" numbered_interval_property_tense -> struct {
        field number <- interval_property_number_words().wf();
        field roi <- selmaho(Roi).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "number" interval_property_number_words -> struct {
        field first_number <- pa_word();
        field continuations <- [zero_or_more interval_property_number_word_continuation];
    }

    rule "number continuation" interval_property_number_word_continuation -> enum {
        interval_property_number_pa_continuation,
        interval_property_number_letter_continuation,
    }

    rule "number continuation" interval_property_number_pa_continuation -> struct {
        field pa <- pa_word();
    }

    rule "number continuation" interval_property_number_letter_continuation -> struct {
        field letter <- word_category(LetterWord);
    }

    rule "interval property" tahe_interval_property_tense -> struct {
        field tahe <- selmaho(Tahe).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "interval property" zaho_interval_property_tense -> struct {
        field zaho <- selmaho(Zaho).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "time tense" pu_time_offset_tense -> struct {
        field pu <- selmaho(Pu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Zi).wf());
    }

    rule "time tense" zi_time_distance_tense -> struct {
        field zi <- selmaho(Zi).wf();
    }

    rule "time interval" zeha_time_interval_tense -> struct {
        field zeha <- selmaho(Zeha).wf();
        field direction <- opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
    }

    rule "space tense" space_tense -> enum {
        space_tense_with_va,
        space_tense_with_offset,
        space_tense_with_interval,
        space_tense_with_mohi,
    }

    rule "space tense" space_tense_with_va -> struct {
        field va <- arc(va_space_distance_tense());
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    rule "space tense" space_tense_with_offset -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [one_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    rule "space tense" space_tense_with_interval -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- arc(space_interval_tense);
        field mohi <- opt(arc(mohi_space_offset_tense()));
    }

    rule "space tense" space_tense_with_mohi -> struct {
        field va <- opt(arc(va_space_distance_tense()));
        field offsets <- [zero_or_more arc(faha_space_offset_tense())];
        field interval <- opt(arc(space_interval_tense));
        field mohi <- arc(mohi_space_offset_tense());
    }

    rule "space tense" va_space_distance_tense -> struct {
        field va <- selmaho(Va).wf();
    }

    rule "space tense" faha_space_offset_tense -> struct {
        field faha <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
        field distance <- opt(selmaho(Va).wf());
    }

    rule "space interval" faha_interval_direction_tense -> struct {
        field faha <- selmaho(Faha).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "space interval" space_interval_tense -> enum {
        space_interval_with_extent_tense,
        space_interval_properties_tense,
    }

    rule "space interval" space_interval_with_extent_tense -> struct {
        field extent <- arc(space_interval_extent_tense);
        field direction <- opt(arc(faha_interval_direction_tense()));
        field properties <- opt(arc(space_interval_properties_tense()));
    }

    rule "space interval" space_interval_extent_tense -> enum {
        veha_space_interval_tense,
        viha_space_interval_tense,
    }

    rule "space interval" space_interval_properties_tense -> struct {
        field first <- arc(fehe_interval_property_tense());
        field additional <- [zero_or_more arc(fehe_interval_property_tense())];
    }

    rule "space interval" veha_space_interval_tense -> struct {
        field veha <- selmaho(Veha).wf();
        field viha <- opt(selmaho(Viha).wf());
    }

    rule "space interval" viha_space_interval_tense -> struct {
        field viha <- selmaho(Viha).wf();
    }

    rule "space interval property" fehe_interval_property_tense -> struct {
        field fehe <- cmavo(Fehe).wf();
        field property <- arc(interval_property_tense);
    }

    rule "space tense" mohi_space_offset_tense -> struct {
        field mohi <- selmaho(Mohi).wf();
        field offset <- arc(faha_space_offset_tense());
    }

    rule "tag" caha_tense -> struct {
        field caha <- selmaho(Caha).wf();
    }

    rule "tag" ki_composite_tense -> struct {
        field ki <- cmavo(Ki).wf();
    }

    rule "tag" cuhe_tense -> struct {
        field cuhe <- selmaho(Cuhe).wf();
    }

    rule "modal tag" modal_tense -> struct {
        field nahe <- opt(selmaho(Nahe).wf());
        field se <- opt(selmaho(Se).wf());
        field bai <- selmaho(Bai).wf();
        field nai <- opt(cmavo(Nai).wf());
        field ki <- opt(cmavo(Ki).wf());
    }

    rule "tag" sticky_tense -> struct {
        field ki <- cmavo(Ki).wf();
    }

    rule "selbri" selbri(selbri, co_selbri, tense_modal, statement) -> enum {
        tagged_selbri,
        untagged_selbri,
    }

    rule "selbri" untagged_selbri(selbri, co_selbri, statement) -> enum {
        negated_selbri,
        co_selbri,
        forethought_selbri_connection,
    }

    rule "tagged selbri" tagged_selbri(selbri, co_selbri, tense_modal, statement) -> struct {
        field tense_modal <- arc(tense_modal);
        field inner_selbri <- arc(untagged_selbri(selbri, co_selbri, statement));
    }

    rule "negated selbri" negated_selbri(selbri) -> struct {
        field na <- selmaho(Na).not_next_selmaho(Ku).wf();
        field inner_selbri <- arc(selbri);
    }

    rule "selbri" co_selbri(co_selbri, tanru_unit, statement) -> struct {
        field leading_selbri <- arc(connected_selbri(tanru_unit, statement));
        field co_tail <- opt(co_selbri_tail(co_selbri));
    }

    rule "selbri" co_selbri_tail(co_selbri) -> struct {
        field co <- cmavo(Co).wf();
        field trailing_selbri <- arc(co_selbri);
    }

    rule "forethought selbri connection" forethought_selbri_connection(selbri) -> struct {
        field guhek <- guhek_connective;
        field leading_selbri <- arc(selbri);
        field first_branch <- forethought_selbri_branch(selbri);
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_branch(selbri)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    rule "forethought selbri connection" forethought_selbri_branch(selbri) -> struct {
        field gik <- gik_connective;
        field selbri <- arc(selbri);
    }

    rule "forethought selbri connection" zantufa_forethought_selbri_branch(selbri) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field selbri <- arc(selbri);
    }

    rule "selbri connection" connected_selbri(tanru_unit, statement) -> struct {
        field leading_selbri <- arc(tanru_selbri(tanru_unit, statement));
        field continuations <- [zero_or_more connected_selbri_continuation(tanru_unit, statement)];
    }

    rule "selbri connection continuation" connected_selbri_continuation(tanru_unit, statement) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_selbri <- arc(tanru_selbri(tanru_unit, statement));
    }

    rule "tanru" tanru_selbri(tanru_unit, statement) -> struct {
        field first_unit <- tanru_unit;
        field additional_units <- [zero_or_more tanru_unit];
    }

    rule "tanru unit" tanru_unit(bo_or_linked_tanru_unit, statement) -> struct {
        field units <- chain(
            first: arc(bo_or_linked_tanru_unit),
            zero_or_more: tanru_unit_continuation(bo_or_linked_tanru_unit, statement),
            element: trailing_unit,
        );
    }

    rule "tanru unit continuation" tanru_unit_continuation(bo_or_linked_tanru_unit, statement) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    rule "tanru unit" bo_or_linked_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        forethought_selbri_group_tanru_unit,
        bound_tanru_unit,
        assigned_pro_bridi_tanru_unit,
        linked_tanru_unit,
    }

    rule "forethought selbri connection" forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri, statement) -> struct {
        field guhek <- guhek_connective;
        field leading_selbri <- arc(selbri);
        field first_branch <- forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement);
        field additional_branches <- [zero_or_more zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement)];
        field gihi <- opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi))).elidable_terminator(Gihi);
    }

    rule "forethought selbri connection" forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        field gik <- gik_connective;
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    rule "forethought selbri connection" zantufa_forethought_selbri_group_branch(bo_or_linked_tanru_unit, statement) -> struct {
        assert feature(ZantufaConnectives);
        field gik <- zantufa_extra_gik_connective;
        field unit <- arc(bo_or_linked_tanru_unit);
    }

    rule "BO-grouped tanru unit" bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        field leading_unit <- arc(linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement));
        field bo_connective <- opt(arc(relation_afterthought_connective));
        field bo_tense_modal <- opt(arc(tense_modal));
        field bo <- cmavo(Bo).wf();
        field trailing_unit <- arc(bo_or_linked_tanru_unit);
    }

    rule "pro-bridi assignment" assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field base <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        field assignments <- [one_or_more pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement)];
    }

    rule "pro-bridi assignment" pro_bridi_tanru_unit_assignment(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field cei <- cmavo(Cei).wf();
        field tanru_unit <- arc(linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    rule "tanru unit" linked_tanru_unit(tanru_unit_atom, sumti, tense_modal, statement) -> struct {
        field base <- arc(tanru_unit_atom);
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    rule "tanru unit" linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field base <- arc(tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
        field linkargs <- opt(linkargs(sumti, tense_modal));
    }

    rule "tanru unit" tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field conversions <- [zero_or_more selmaho(Se).wf()];
        field base <- arc(tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    rule "tanru unit" tanru_unit_atom_base_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        pro_bridi_tanru_unit,
        ordinal_tanru_unit,
        word_tanru_unit,
        preposed_linkargs_tanru_unit,
        jai_modal_tanru_unit,
        scalar_negated_tanru_unit,
        when feature(ZantufaTerms) zantufa_statement_abstraction_tanru_unit,
        abstraction_tanru_unit,
        sumti_selbri_tanru_unit,
        zantufa_me_tanru_unit,
        zantufa_mex_moi_tanru_unit,
        operator_selbri_tanru_unit,
        quoted_bridi_selbri_tanru_unit,
        quoted_text_selbri_tanru_unit,
        text_selbri_tanru_unit,
        tag_selbri_tanru_unit,
        goha_word_tanru_unit,
        grouped_tanru_unit,
    }

    rule "tanru unit" tanru_unit_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> struct {
        field conversions <- [zero_or_more selmaho(Se).wf()];
        field base <- arc(tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement));
    }

    rule "tanru unit" tanru_unit_atom_base(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso, mekso_operator, letter_tokens, letter_string, statement) -> enum {
        ordinal_tanru_unit,
        word_tanru_unit,
        preposed_linkargs_tanru_unit,
        jai_modal_tanru_unit,
        scalar_negated_tanru_unit,
        when feature(ZantufaTerms) zantufa_statement_abstraction_tanru_unit,
        abstraction_tanru_unit,
        sumti_selbri_tanru_unit,
        zantufa_me_tanru_unit,
        zantufa_mex_moi_tanru_unit,
        operator_selbri_tanru_unit,
        quoted_bridi_selbri_tanru_unit,
        quoted_text_selbri_tanru_unit,
        text_selbri_tanru_unit,
        tag_selbri_tanru_unit,
        goha_word_tanru_unit,
        pro_bridi_tanru_unit,
        grouped_tanru_unit,
    }

    rule "tagged selbri" tagged_selbri_group_tanru_unit(tanru_unit, tense_modal, statement) -> struct {
        field tense_modal <- arc(tense_modal);
        field inner_selbri <- arc(connected_selbri(tanru_unit, statement));
    }

    rule "linked arguments" preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal, statement) -> struct {
        field linkargs <- linkargs(sumti, tense_modal);
        field base <- arc(tanru_unit);
    }

    rule "scalar-negated tanru unit" scalar_negated_tanru_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_unit <- arc(scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement));
    }

    rule "scalar-negated tanru unit" scalar_negated_tanru_inner_unit(tanru_unit_atom, tanru_unit, tense_modal, statement) -> enum {
        tagged_selbri_group_tanru_unit,
        pro_bridi_tanru_unit,
        tanru_unit_atom,
    }

    rule "modal conversion" jai_modal_tanru_unit(jai_inner_tanru_unit, tense_modal) -> struct {
        field jai <- cmavo(Jai).wf();
        field tense_modal <- opt(arc(tense_modal));
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    rule "modal conversion" jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, letter_tokens, letter_string) -> enum {
        converted_jai_inner_tanru_unit,
        scalar_negated_jai_inner_tanru_unit,
        sumti_selbri_tanru_unit,
        quoted_bridi_selbri_tanru_unit,
        quoted_text_selbri_tanru_unit,
        text_selbri_tanru_unit,
        grouped_jai_inner_tanru_unit,
        ordinal_tanru_unit,
        operator_selbri_tanru_unit,
        pro_bridi_tanru_unit,
        word_tanru_unit,
    }

    rule "converted tanru unit" converted_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        field se <- selmaho(Se).wf();
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    rule "scalar-negated tanru unit" scalar_negated_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        field nahe <- selmaho(Nahe).wf();
        field inner_unit <- arc(jai_inner_tanru_unit);
    }

    rule "quoted bridi selbri" quoted_bridi_selbri_tanru_unit -> struct {
        field quote <- choice((
            quote_marker(Gohoi),
            quote_marker(Zehoi),
            quote_marker(Tahai),
            quote_marker(Bohei),
        )).warn(ExperimentalGohoiSelbriUnit).wf();
    }

    rule "text selbri" text_selbri_tanru_unit(text) -> struct {
        field luhei <- cmavo(Luhei).warn(ExperimentalZantufaLuheiSelbriUnit).wf();
        field text <- arc(text);
        field lihau <- opt(cmavo(Lihau).wf()).elidable_terminator(Lihau);
    }

    rule "quoted text selbri" quoted_text_selbri_tanru_unit -> struct {
        field muhoi <- delimited_quote_marker(Muhoi).warn(ExperimentalZantufaMuhoiSelbriUnit).wf();
    }

    rule "tag selbri" tag_selbri_tanru_unit(tense_modal) -> struct {
        field xohi <- cmavo(Xohi).warn(ExperimentalXohiTagSelbri).wf();
        field tag <- arc(tense_modal);
    }

    rule "ordinal selbri" ordinal_tanru_unit(letter_tokens, letter_string) -> struct {
        field number <- number_or_letter_words(letter_tokens, letter_string);
        field moi <- selmaho(Moi).wf();
    }

    rule "tanru unit" word_tanru_unit -> struct {
        field word <- tanru_unit_relation_word().wf();
    }

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

    rule "pro-bridi" pro_bridi_tanru_unit -> struct {
        field goha <- selmaho(Goha).wf();
        field raho <- opt(cmavo(Raho).wf());
    }

    rule "sumti-to-selbri" sumti_selbri_tanru_unit(sumti, letter_string) -> struct {
        field me <- cmavo(Me).wf();
        field sumti <- arc(sumti_selbri_sumti(sumti, letter_string));
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    rule "sumti-to-selbri" zantufa_me_tanru_unit(mekso, mekso_operator, tense_modal) -> struct {
        field me <- cmavo(Me).warn(ExperimentalZantufaMex).wf();
        field body <- arc(zantufa_me_selbri_body(mekso, mekso_operator, tense_modal));
        field mehu <- opt(cmavo(Mehu).wf()).elidable_terminator(Mehu);
        field moi_marker <- opt(selmaho(Moi).wf());
    }

    rule "sumti-to-selbri" zantufa_me_selbri_body(mekso, mekso_operator, tense_modal) -> enum {
        zantufa_me_operator_selbri_body,
        zantufa_me_mekso_selbri_body,
        zantufa_me_tag_selbri_body,
    }

    rule "sumti-to-selbri" zantufa_me_operator_selbri_body(mekso_operator) -> struct {
        field operators <- [one_or_more mekso_operator];
    }

    rule "sumti-to-selbri" zantufa_me_mekso_selbri_body(mekso) -> struct {
        field expression <- arc(mekso);
    }

    rule "sumti-to-selbri" zantufa_me_tag_selbri_body(tense_modal) -> struct {
        field tag <- arc(tense_modal);
    }

    rule "mex selbri" zantufa_mex_moi_tanru_unit(mekso) -> struct {
        field expression: std::sync::Arc<MeksoSyntax> <- arc(mekso.complete_before_selmaho(Moi));
        field moi <- selmaho(Moi).warn(ExperimentalZantufaMex).wf();
    }

    rule "sumti selbri" sumti_selbri_sumti(sumti, letter_string) -> enum {
        sumti,
        me_lerfu_sumti,
    }

    rule "lerfu string" me_lerfu_sumti(letter_string) -> struct {
        field words <- letter_string;
    }

    rule "operator-to-selbri" operator_selbri_tanru_unit(mekso_operator) -> struct {
        field nuha <- cmavo(Nuha).wf();
        field mekso_operator <- arc(mekso_operator);
    }

    rule "grouped tanru" grouped_tanru_unit(tanru_unit, statement) -> struct {
        field ke <- cmavo(Ke).wf();
        field selbri <- arc(connected_selbri(tanru_unit, statement));
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "grouped tanru" grouped_jai_inner_tanru_unit(jai_inner_tanru_unit) -> struct {
        field ke <- cmavo(Ke).wf();
        field selbri <- arc(connected_jai_inner_selbri(jai_inner_tanru_unit));
        field kehe <- opt(cmavo(Kehe).wf()).elidable_terminator(Kehe);
    }

    rule "selbri connection" connected_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        field leading_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
        field continuations <- [zero_or_more connected_jai_inner_selbri_continuation(jai_inner_tanru_unit)];
    }

    rule "selbri connection continuation" connected_jai_inner_selbri_continuation(jai_inner_tanru_unit) -> struct {
        field connective <- relation_afterthought_connective;
        field trailing_selbri <- arc(tanru_jai_inner_selbri(jai_inner_tanru_unit));
    }

    rule "selbri" tanru_jai_inner_selbri(jai_inner_tanru_unit) -> struct {
        field first_unit <- jai_inner_tanru_unit;
        field additional_units <- [zero_or_more jai_inner_tanru_unit];
    }

    rule "linked arguments" linked_sumti(sumti, tense_modal) -> enum {
        place_tagged_linked_sumti,
        tense_tagged_linked_sumti,
        plain_linked_sumti,
        empty_linked_sumti,
    }

    rule "linked arguments" place_tagged_linked_sumti(sumti) -> struct {
        field fa <- selmaho(Fa).wf();
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    rule "linked arguments" tense_tagged_linked_sumti(sumti, tense_modal) -> struct {
        field tense_modal <- arc(tense_modal);
        field sumti <- arc(tagged_or_elided_sumti(sumti));
    }

    rule "linked arguments" plain_linked_sumti(sumti) -> struct {
        field sumti <- arc(sumti);
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
        field beho <- opt(cmavo(Beho).wf()).elidable_terminator(Beho);
    }

    rule "abstraction" abstraction_tanru_unit(subbridi) -> struct {
        field nu <- selmaho(Nu).wf();
        field nai <- opt(cmavo(Nai).wf());
        field abstractor_connections <- [zero_or_more abstractor_connection()];
        field subbridi <- arc(subbridi);
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

    rule "abstractor connection" abstractor_connection -> struct {
        field connective <- standard_statement_connective;
        field nu <- selmaho(Nu).wf();
        field nai <- opt(cmavo(Nai).wf());
    }

    rule "abstraction" zantufa_statement_abstraction_tanru_unit(statement) -> struct {
        field nu <- selmaho(Nu).warn(ExperimentalZantufaStatementAbstraction).wf();
        field nai <- opt(cmavo(Nai).wf());
        field abstractor_connections <- [zero_or_more zantufa_abstractor_connection()];
        field statement <- arc(statement);
        field kei <- opt(cmavo(Kei).wf()).elidable_terminator(Kei);
    }

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
        let diagnostic_candidate = state.diagnostic_candidate();
        let finish = state.finish();
        let result = match result {
            Ok(text) => Ok(GeneratedParsedText {
                text,
                warnings: finish.warnings,
            }),
            Err(errors) => Err(syntax_error_with_diagnostic_candidate(
                errors,
                diagnostic_candidate,
                options.error_context_depth,
            )),
        };
        GeneratedParsedTextAttempt {
            result,
            trace: finish.trace,
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn strict_generated_text_parser_with_eof<'tokens>() -> BoxedParser<'tokens, TextSyntax> {
        custom::<_, ParserInput<'tokens>, _, ParseExtra<'tokens>>(move |input| {
            let text = input.parse(&strict_generated_text_parser())?;
            input.parse(end()).map(|()| text)
        })
        .boxed()
    }
}
