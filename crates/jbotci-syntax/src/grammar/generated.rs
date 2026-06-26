//! Declarative generated syntax parser.

#![allow(dead_code)]

use chumsky::span::SimpleSpan;
use chumsky::{Parser, input::Input, primitive::end, recursive::Recursive};
use jbotci_morphology::{Cmavo, Selmaho};

use super::ast::*;
use super::generated_runtime;
use super::tokens::{
    cmavo, cmevla_word, leading_indicator, pa_word, relation_word, selmaho, spanned_tokens,
    syntax_error,
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
        statement_or_fragment: StatementSyntax;
        statement: StatementSyntax;
        bridi: BridiSyntax;
        bridi_tail: BridiTailSyntax;
        bo_grouped_bridi_tail: BoGroupedBridiTailSyntax;
        bo_grouped_bridi_tail_without_tail_terms: BoGroupedBridiTailSyntax;
        forethought_bridi_connection: ForethoughtBridiConnectionSyntax;
        forethought_bridi_connection_without_tail_terms: ForethoughtBridiConnectionSyntax;
        subbridi: SubbridiSyntax;
        term: TermSyntax;
        sumti: SumtiSyntax;
        sumti_grouped: SumtiSyntax;
        sumti_afterthought: SumtiSyntax;
        sumti_bound: SumtiSyntax;
        sumti_forethought: SumtiSyntax;
        sumti_base: SumtiSyntax;
        relative_clause: RelativeClauseSyntax;
        selbri: SelbriSyntax;
        co_selbri: SelbriSyntax;
        tanru_unit: TanruUnitSyntax;
        bo_or_linked_tanru_unit: TanruUnitSyntax;
        tanru_unit_atom: TanruUnitSyntax;
        jai_inner_tanru_unit: TanruUnitSyntax;
        tense_modal: TenseModalSyntax;
        mekso: MeksoSyntax;
        mekso_base: MeksoSyntax;
        mekso_precedence: MeksoSyntax;
        mekso_operand: MeksoSyntax;
        mekso_operator: MeksoOperatorSyntax;
        reverse_polish_parts: ReversePolishPartsSyntax;
        letter_string: std::vec::Vec<Token>;
        letter_tokens: std::vec::Vec<Token>;
        free_modifier: FreeModifierSyntax;
    }

    alias text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> TextSyntax {
        context "text";
        choice((
            explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier),
            regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal),
        ));
    }

    product explicit_xauha_lohoi_lookahead -> () {
        fields {
            field xauha = cmavo(Xauha).ignored();
            field body = raw_words_until(Kuhau).ignored();
            field kuhau = cmavo(Kuhau).ignored();
        }
    }

    product explicit_xauha_lohoi_text(paragraph, statement_or_fragment, free_modifier) -> TextSyntax {
        context "text";
        construct variant ExplicitXauhaLohoi;
        fields {
            require explicit_xauha_lohoi_lookahead().lookahead();
            field paragraphs = text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
        }
    }

    product regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> TextSyntax {
        context "text";
        construct variant Regular;
        fields {
            field leading_nai = many(cmavo(Nai));
            field leading_cmevla = many(text_leading_cmevla_word());
            field leading_indicators = many(leading_indicator());
            field leading_free_modifiers = many(free_modifier);
            field leading_connective = opt(text_leading_connective(tense_modal));
            field leading_i_statements = many(leading_i_statement(free_modifier, tense_modal));
            #[tree_child(primary)]
            field paragraphs = text_paragraphs(paragraph, statement_or_fragment, free_modifier);
        }
    }

    alias text_paragraphs(paragraph, statement_or_fragment, free_modifier) -> std::vec::Vec<ParagraphSyntax> {
        context "paragraphs";
        opt_or_default(choice((
            text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier),
            many1(niho_paragraph(statement_or_fragment, free_modifier)),
        )));
    }

    alias text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier) -> std::vec::Vec<ParagraphSyntax> {
        context "paragraphs";
        prepend(
            paragraph,
            many(niho_paragraph(statement_or_fragment, free_modifier)),
        );
    }

    alias text_leading_connective(tense_modal) -> ConnectiveSyntax {
        context "text connective";
        require modal_forethought_connective(tense_modal).not();
        choice((
            standard_statement_connective,
            cehe_connective(),
        ));
    }

    product leading_i_statement(free_modifier, tense_modal) -> LeadingIStatementSyntax {
        context "paragraph statement";
        fields {
            field i = cmavo(I);
            field connective = opt(boxed(i_paragraph_statement_connective(tense_modal)));
            field free_modifiers = many(free_modifier);
        }
    }

    alias paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        choice((
            i_niho_paragraph(statement_or_fragment, free_modifier),
            simple_paragraph(statement_or_fragment, free_modifier),
        ));
    }

    node simple_paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        construct variant SimpleParagraph;
        fields {
            default i: Option<Token> = None;
            default niho: Vec<Token> = Vec::new();
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
            #[tree_child(primary)]
            field statements = paragraph_statement_sequence(statement_or_fragment, free_modifier);
        }
    }

    alias paragraph_statement_sequence(statement_or_fragment, free_modifier) -> std::vec::Vec<ParagraphStatementSyntax> {
        context "paragraph";
        append(
            prepend(
                initial_paragraph_statement(statement_or_fragment),
                many(following_paragraph_statement(statement_or_fragment, free_modifier)),
            ),
            many(trailing_ijek_paragraph_statement()),
        );
    }

    node i_niho_paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        construct variant INihoParagraph;
        fields {
            field i = some(cmavo(I));
            field niho = many1(selmaho(Niho));
            field free_modifiers = many(free_modifier);
            #[tree_child(primary)]
            field statements = opt_or_default(paragraph_statement_sequence(statement_or_fragment, free_modifier));
        }
    }

    node niho_paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        construct variant NihoParagraph;
        fields {
            default i: Option<Token> = None;
            field niho = many1(selmaho(Niho));
            field free_modifiers = many(free_modifier);
            #[tree_child(primary)]
            field statements = opt_or_default(paragraph_statement_sequence(statement_or_fragment, free_modifier));
        }
    }

    alias paragraph_statement(statement_or_fragment, free_modifier, tense_modal) -> ParagraphStatementSyntax {
        context "paragraph statement";
        choice((
            trailing_ijek_paragraph_statement(),
            i_paragraph_statement(statement_or_fragment, free_modifier, tense_modal),
            initial_paragraph_statement(statement_or_fragment),
        ));
    }

    node initial_paragraph_statement(statement_or_fragment) -> ParagraphStatementSyntax {
        context "paragraph statement";
        construct variant InitialParagraphStatement;
        fields {
            default i: Option<Token> = None;
            default connective: Option<Box<ConnectiveSyntax>> = None;
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
            #[tree_child(primary)]
            field statement = some(boxed(statement_or_fragment));
        }
    }

    node i_paragraph_statement(statement_or_fragment, free_modifier, tense_modal) -> ParagraphStatementSyntax {
        context "paragraph statement";
        construct variant IParagraphStatement;
        fields {
            field i = some(cmavo(I));
            field connective = opt(boxed(i_paragraph_statement_connective(tense_modal)));
            field free_modifiers = many(free_modifier);
            #[tree_child(primary)]
            field statement = opt(boxed(statement_or_fragment));
        }
    }

    node following_paragraph_statement(statement_or_fragment, free_modifier) -> ParagraphStatementSyntax {
        context "paragraph statement";
        construct variant FollowingParagraphStatement;
        fields {
            field i = some(cmavo(I));
            require statement_connective.not();
            default connective: Option<Box<ConnectiveSyntax>> = None;
            field free_modifiers = many(free_modifier);
            #[tree_child(primary)]
            field statement = opt(boxed(statement_or_fragment));
        }
    }

    node trailing_ijek_paragraph_statement -> ParagraphStatementSyntax {
        context "paragraph statement";
        construct variant TrailingIjekParagraphStatement;
        fields {
            field i = cmavo(I);
            field connective = statement_connective;
        }
    }

    alias statement(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> StatementSyntax {
        context "statement";
        choice((
            i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens),
            preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens),
            statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens),
        ));
    }

    alias statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> StatementSyntax {
        context "statement";
        choice((
            prenex_statement(statement, term),
            bridi_statement(bridi, subbridi, tense_modal),
            text_group_statement(text, tense_modal),
        ));
    }

    alias statement_or_fragment(statement, term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> StatementSyntax {
        context "paragraph statement";
        choice((
            statement,
            fragment_statement(term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens),
        ));
    }

    alias fragment_statement(term, sumti, subbridi, selbri, mekso, tense_modal, letter_tokens) -> StatementSyntax {
        context "fragment";
        choice((
            prenex_fragment(term),
            selbri_fragment(selbri),
            ek_fragment(),
            gihek_fragment(),
            multiple_na_fragment(),
            single_na_fragment(),
            terms_fragment(term),
            mekso_fragment(mekso, letter_tokens),
            relative_clause_fragment(sumti, subbridi, tense_modal),
            linked_sumti_continuation_fragment(sumti, tense_modal),
            linked_sumti_fragment(sumti, tense_modal),
        ));
    }

    alias statement_after_i_connective(bridi, subbridi, tense_modal, text) -> StatementSyntax {
        context "statement";
        choice((
            bridi_statement(bridi, subbridi, tense_modal),
            text_group_statement(text, tense_modal),
        ));
    }

    node multiple_na_fragment -> StatementSyntax {
        context "fragment";
        construct variant MultipleNaFragment;
        fields {
            field first_na = selmaho(Na);
            field second_na = selmaho(Na);
            field additional_na = many(selmaho(Na));
        }
    }

    node single_na_fragment -> StatementSyntax {
        context "fragment";
        construct variant SingleNaFragment;
        fields {
            field na = selmaho(Na).not_next_selmaho(Ku).wf();
        }
    }

    node ek_fragment -> StatementSyntax {
        context "fragment";
        construct variant EkFragment;
        fields {
            #[tree_child(primary)]
            field connective = ek_connective();
        }
    }

    node gihek_fragment -> StatementSyntax {
        context "fragment";
        construct variant GihekFragment;
        fields {
            #[tree_child(primary)]
            field connective = gihek_connective();
        }
    }

    node i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> StatementSyntax {
        context "statement connection";
        construct variant IStatementConnection;
        fields {
            field leading_statement = boxed(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
            field continuations = many1(choice((
                chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens),
                simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens),
            )));
        }
    }

    product pending_i_connective -> PendingIConnectiveSyntax {
        context "statement connective";
        fields {
            field i = cmavo(I);
            field connective = statement_connective;
            require cmavo(I).lookahead();
        }
    }

    product chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> IStatementConnectionTailSyntax {
        context "statement connection";
        construct variant Chained;
        no_partial_valid;
        fields {
            field pending = many1(pending_i_connective);
            field i = cmavo(I);
            field connective = i_statement_connective(tense_modal);
            field trailing_statement = boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
        }
    }

    product simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> IStatementConnectionTailSyntax {
        context "statement connection";
        construct variant Simple;
        no_partial_valid;
        fields {
            field i = cmavo(I);
            field connective = i_statement_connective(tense_modal);
            field trailing_statement = boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
        }
    }

    node preposed_i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens) -> StatementSyntax {
        context "statement connection";
        construct variant PreposedIStatementConnection;
        fields {
            field leading_statement = boxed(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
            field connective = statement_connective;
            field i = cmavo(I);
            field trailing_statement = boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
        }
    }

    node text_group_statement(text, tense_modal) -> StatementSyntax {
        context "text group";
        construct variant TextGroup;
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field tuhe = cmavo(Tuhe).wf();
            #[tree_child(primary)]
            field text = boxed(text);
            field tuhu = opt(cmavo(Tuhu).wf());
        }
    }

    node prenex_fragment(term) -> StatementSyntax {
        context "prenex";
        construct variant PrenexFragment;
        fields {
            field terms = many(term);
            field zohu = cmavo(Zohu).wf();
        }
    }

    node prenex_statement(statement, term) -> StatementSyntax {
        context "prenex";
        construct variant Prenex;
        fields {
            field prenex_terms = many(term);
            field zohu = cmavo(Zohu).wf();
            #[tree_child(primary)]
            field inner_statement = boxed(statement);
        }
    }

    node bridi_statement(bridi, subbridi, tense_modal) -> StatementSyntax {
        context "statement";
        construct variant BridiStatement;
        fields {
            #[tree_child(primary)]
            field bridi = boxed(bridi);
            field continuations = many(bridi_statement_continuation(subbridi, tense_modal));
        }
    }

    alias bridi_statement_continuation(subbridi, tense_modal) -> BridiStatementContinuationSyntax {
        context "bridi continuation";
        choice((
            bo_bridi_statement_continuation(subbridi, tense_modal),
            ke_bridi_statement_continuation(subbridi, tense_modal),
        ));
    }

    product bo_bridi_statement_continuation(subbridi, tense_modal) -> BridiStatementContinuationSyntax {
        context "bridi continuation";
        construct variant BoGroupedBridiStatementContinuation;
        fields {
            field connective = bridi_tail_connective;
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field trailing_subbridi = boxed(subbridi);
        }
    }

    product ke_bridi_statement_continuation(subbridi, tense_modal) -> BridiStatementContinuationSyntax {
        context "bridi continuation";
        construct variant KeGroupedBridiStatementContinuation;
        fields {
            field connective = relation_afterthought_connective;
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field trailing_subbridi = boxed(subbridi);
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    node selbri_fragment(selbri) -> StatementSyntax {
        context "selbri";
        construct variant SelbriFragment;
        fields {
            #[tree_child(primary)]
            field selbri = boxed(selbri);
        }
    }

    node terms_fragment(term) -> StatementSyntax {
        context "terms";
        construct variant TermsFragment;
        model_variant Terms;
        fields {
            #[tree_child(primary)]
            field terms = many1(term);
            field vau = opt(cmavo(Vau).wf());
        }
    }

    node mekso_fragment(mekso, letter_tokens) -> StatementSyntax {
        context "mex";
        construct variant MeksoFragment;
        fields {
            #[tree_child(primary)]
            field quantifier = boxed(quantifier(mekso, letter_tokens));
        }
    }

    alias relative_clause_list(sumti, subbridi, tense_modal) -> std::vec::Vec<RelativeClauseSyntax> {
        context "relative clauses";
        prepend(
            relative_clause_atom(sumti, subbridi, tense_modal),
            many(relative_clause_tail(sumti, subbridi, tense_modal)),
        );
    }

    node relative_clause_fragment(sumti, subbridi, tense_modal) -> StatementSyntax {
        context "relative clauses";
        construct variant RelativeClauseFragment;
        fields {
            #[tree_child(primary)]
            field relative_clauses = relative_clause_list(sumti, subbridi, tense_modal);
        }
    }

    node linked_sumti_continuation_fragment(sumti, tense_modal) -> StatementSyntax {
        context "linked arguments";
        construct variant LinkedSumtiContinuationFragment;
        fields {
            #[tree_child(primary)]
            field bei_links = many1(bei_link(sumti, tense_modal));
        }
    }

    node linked_sumti_fragment(sumti, tense_modal) -> StatementSyntax {
        context "linked arguments";
        construct variant LinkedSumtiFragment;
        fields {
            #[tree_child(primary)]
            field linkargs = linkargs(sumti, tense_modal);
        }
    }

    alias bridi(term, selbri, subbridi, tense_modal, bridi_tail) -> BridiSyntax {
        context "bridi";
        choice((
            bridi_with_leading_terms(term, bridi_tail),
            bridi_with_post_cu_terms(term, bridi_tail),
            bare_cu_bridi(bridi_tail),
            bare_cu_terms_bridi(term, bridi_tail),
            relation_only_bridi(bridi_tail),
        ));
    }

    node bridi_with_leading_terms(term, bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            field leading_terms = many1(term);
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bridi_tail);
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bridi_with_post_cu_terms(term, bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            field leading_terms = many1(term);
            field cu = some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
            field bridi_tail = boxed(cu_terms_bridi_tail(term, bridi_tail));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bare_cu_bridi(bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms: Vec<TermSyntax> = Vec::new();
            field cu = some(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bridi_tail);
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bare_cu_terms_bridi(term, bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms: Vec<TermSyntax> = Vec::new();
            field cu = some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
            field bridi_tail = boxed(cu_terms_bridi_tail(term, bridi_tail));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node relation_only_bridi(bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms: Vec<TermSyntax> = Vec::new();
            default cu: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> = None;
            field bridi_tail = boxed(bridi_tail);
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node cu_terms_bridi_tail(term, bridi_tail) -> BridiTailSyntax {
        context "bridi tail";
        fields {
            field terms = many1(term);
            field bridi_tail = boxed(bridi_tail);
        }
    }

    alias bridi_tail(bridi_tail, bo_grouped_bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> BridiTailSyntax {
        context "bridi tail";
        choice((
            bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal),
            bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal),
        ));
    }

    node bridi_tail_without_tail_terms(bridi_tail, bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> BridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal));
            field ke_continuation = opt(boxed(bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
        }
    }

    node bridi_tail_with_possible_tail_terms(bridi_tail, bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> BridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal));
            require (relation_connective_as_bridi_tail, opt(boxed(tense_modal)), cmavo(Ke)).not();
            field ke_continuation = opt(boxed(gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal)));
        }
    }

    node afterthought_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, selbri, subbridi, term, tense_modal) -> AfterthoughtBridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(bo_grouped_bridi_tail_without_tail_terms);
            field continuations = many(bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal));
        }
    }

    node afterthought_bridi_tail(bo_grouped_bridi_tail, selbri, subbridi, term, tense_modal) -> AfterthoughtBridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(bo_grouped_bridi_tail);
            field continuations = many(bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal));
        }
    }

    node bo_grouped_bridi_tail_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> BoGroupedBridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal));
            field bo_continuation = opt(boxed(bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal)));
        }
    }

    node bo_grouped_bridi_tail(bo_grouped_bridi_tail, forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> BoGroupedBridiTailSyntax {
        context "bridi tail";
        construct direct;
        fields {
            field first = boxed(simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal));
            field bo_continuation = opt(boxed(bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal)));
        }
    }

    alias simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms, selbri, subbridi, term, tense_modal) -> SimpleBridiTailSyntax {
        context "bridi tail";
        choice((
            forethought_simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms),
            selbri_simple_bridi_tail_without_tail_terms(selbri),
        ));
    }

    alias simple_bridi_tail(forethought_bridi_connection, selbri, subbridi, term, tense_modal) -> SimpleBridiTailSyntax {
        context "bridi tail";
        choice((
            forethought_simple_bridi_tail(forethought_bridi_connection),
            selbri_simple_bridi_tail(selbri, term),
        ));
    }

    node forethought_simple_bridi_tail_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> SimpleBridiTailSyntax {
        context "forethought bridi connection";
        construct tuple_variant ForethoughtBridiTailConnection;
        fields {
            field connection = boxed(forethought_bridi_connection_without_tail_terms);
        }
    }

    node forethought_simple_bridi_tail(forethought_bridi_connection) -> SimpleBridiTailSyntax {
        context "forethought bridi connection";
        construct tuple_variant ForethoughtBridiTailConnection;
        fields {
            field connection = boxed(forethought_bridi_connection);
        }
    }

    node selbri_simple_bridi_tail_without_tail_terms(selbri) -> SimpleBridiTailSyntax {
        context "bridi tail";
        construct variant SelbriBridiTail;
        fields {
            field selbri = boxed(selbri);
            default terms: Vec<TermSyntax> = Vec::new();
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node selbri_simple_bridi_tail(selbri, term) -> SimpleBridiTailSyntax {
        context "bridi tail";
        construct variant SelbriBridiTail;
        fields {
            field selbri = boxed(selbri);
            field terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    alias forethought_bridi_connection(forethought_bridi_connection, subbridi, term, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        choice((
            direct_forethought_bridi_connection(subbridi, term, tense_modal),
            grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal),
            negated_forethought_bridi_connection(forethought_bridi_connection),
        ));
    }

    alias forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, subbridi, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        choice((
            direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal),
            grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal),
            negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms),
        ));
    }

    node direct_forethought_bridi_connection(subbridi, term, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant BridiConnection;
        fields {
            field gek = modal_forethought_connective(tense_modal);
            field first = boxed(subbridi);
            field gik = gik_connective;
            field second = boxed(subbridi);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node direct_forethought_bridi_connection_without_tail_terms(subbridi, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant BridiConnection;
        fields {
            field gek = modal_forethought_connective(tense_modal);
            field first = boxed(subbridi);
            field gik = gik_connective;
            field second = boxed(subbridi);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
            default tail_terms: Vec<TermSyntax> = Vec::new();
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node grouped_forethought_bridi_connection(forethought_bridi_connection, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant GroupedBridiConnection;
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field inner = boxed(forethought_bridi_connection);
            field kehe = opt(arc(cmavo(Kehe).wf()));
        }
    }

    node grouped_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms, tense_modal) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant GroupedBridiConnection;
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field inner = boxed(forethought_bridi_connection_without_tail_terms);
            field kehe = opt(arc(cmavo(Kehe).wf()));
        }
    }

    node negated_forethought_bridi_connection(forethought_bridi_connection) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant NegatedBridiConnection;
        fields {
            field na = selmaho(Na).wf();
            field inner = boxed(forethought_bridi_connection);
        }
    }

    node negated_forethought_bridi_connection_without_tail_terms(forethought_bridi_connection_without_tail_terms) -> ForethoughtBridiConnectionSyntax {
        context "forethought bridi connection";
        construct variant NegatedBridiConnection;
        fields {
            field na = selmaho(Na).wf();
            field inner = boxed(forethought_bridi_connection_without_tail_terms);
        }
    }

    node bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> GroupedBridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            field connective = bridi_tail_connective;
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field bridi_tail = boxed(bridi_tail);
            field kehe = opt(arc(cmavo(Kehe).wf()));
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node gihek_bridi_tail_ke_continuation(bridi_tail, term, tense_modal) -> GroupedBridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            field connective = gihek_connective();
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field bridi_tail = boxed(bridi_tail);
            field kehe = opt(arc(cmavo(Kehe).wf()));
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bridi_tail_bo_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> BoundBridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            field connective = bridi_tail_connective;
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail_without_tail_terms);
            default tail_terms: Vec<TermSyntax> = Vec::new();
            default vau: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> = None;
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bridi_tail_bo_continuation(bo_grouped_bridi_tail, term, tense_modal) -> BoundBridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            field connective = bridi_tail_connective;
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail);
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> BridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            require (bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke)))).not();
            field connective = bridi_tail_connective;
            default tense_modal: Option<Box<TenseModalSyntax>> = None;
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail_without_tail_terms);
            default tail_terms: Vec<TermSyntax> = Vec::new();
            default vau: Option<std::sync::Arc<WithFreeModifiers<Token, FreeModifierSyntax>>> = None;
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> BridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            require (bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke)))).not();
            field connective = bridi_tail_connective;
            default tense_modal: Option<Box<TenseModalSyntax>> = None;
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail);
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    alias subbridi(subbridi, bridi, term) -> SubbridiSyntax {
        context "subbridi";
        choice((
            prenex_subbridi(subbridi, term),
            bridi_subbridi(bridi),
        ));
    }

    node bridi_subbridi(bridi) -> SubbridiSyntax {
        context "subbridi";
        construct tuple_variant Bridi;
        fields {
            field bridi = boxed(bridi);
        }
    }

    node prenex_subbridi(subbridi, term) -> SubbridiSyntax {
        context "prenex";
        construct variant Prenex;
        fields {
            field prenex_terms = many(term);
            field zohu = cmavo(Zohu).wf();
            field inner_subbridi = boxed(subbridi);
        }
    }

    alias term(term, sumti, tense_modal, subbridi, selbri) -> TermSyntax {
        context "term";
        require (relation_word(), cmavo(Bu).not()).not();
        choice((
            pehe_termset_connection(sumti, tense_modal, subbridi, selbri, term),
            bound_term_connection(sumti, tense_modal, subbridi, selbri, term),
            termset_group(sumti, tense_modal, subbridi, selbri, term),
            connected_term(sumti, tense_modal, subbridi, selbri, term),
            simple_term(sumti, tense_modal, subbridi, selbri, term),
        ));
    }

    node pehe_termset_connection(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "termset connection";
        fields {
            field leading_term = boxed(pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term));
            field continuations = many1((cmavo(Pehe).wf(), statement_connective, boxed(pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term))));
        }
    }

    alias pehe_termset_operand(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "term";
        choice((
            bound_term_connection(sumti, tense_modal, subbridi, selbri, term),
            termset_group(sumti, tense_modal, subbridi, selbri, term),
            simple_term(sumti, tense_modal, subbridi, selbri, term),
        ));
    }

    alias simple_term(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "term";
        choice((
            place_tagged_sumti_term(sumti),
            feature(ZantufaTags, jai_tagged_sumti_term(tense_modal, sumti)),
            tagged_sumti_before_tag_term(tense_modal, selbri),
            tagged_sumti_term(tense_modal, sumti, selbri),
            noiha_adverbial_term(selbri),
            fihoi_adverbial_term(subbridi),
            soi_adverbial_term(subbridi),
            na_ku_term(),
            sumti_term(sumti),
            bare_na_term(selbri, tense_modal),
            forethought_termset(term, tense_modal),
            nuhi_termset(term),
            ke_termset(term),
        ));
    }

    node bound_term_connection(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "term connection";
        fields {
            field leading_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            field connective = boxed(joik_ek_connective);
            field bo = cmavo(Bo).wf();
            require term_hierarchy_post_bo_argument_gate(sumti);
            field trailing_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            require term_hierarchy_post_bo_argument_gate(sumti);
        }
    }

    product term_hierarchy_post_bo_argument_gate(sumti) -> () {
        context "term connection";
        fields {
            require choice((
                term_hierarchy_enabled_empty_gate(),
                term_hierarchy_disabled_sumti_guard(sumti),
            ));
        }
    }

    product term_hierarchy_enabled_empty_gate -> () {
        context "term connection";
        fields {
            require feature(TermHierarchy, empty());
        }
    }

    product term_hierarchy_disabled_sumti_guard(sumti) -> () {
        context "term connection";
        fields {
            require feature(TermHierarchy, empty()).not();
            require sumti.not();
        }
    }

    node connected_term(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "term connection";
        fields {
            field leading_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            field continuations = many((term_connective, boxed(simple_term(sumti, tense_modal, subbridi, selbri, term))));
        }
    }

    node termset_group(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "termset";
        fields {
            field leading_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            field continuations = many1((cmavo(Cehe).wf(), boxed(simple_term(sumti, tense_modal, subbridi, selbri, term))));
        }
    }

    node forethought_termset(term, tense_modal) -> TermSyntax {
        context "termset";
        construct variant ForethoughtTermsetConnection;
        fields {
            field m_nuhi = opt(cmavo(Nuhi).wf());
            field gek = modal_forethought_connective(tense_modal);
            field terms = many1(boxed(term));
            field nuhu = opt(cmavo(Nuhu).wf());
            field gik = gik_connective;
            field gik_terms = many1(boxed(term));
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
            field gik_nuhu = opt(cmavo(Nuhu).wf());
        }
    }

    node nuhi_termset(term) -> TermSyntax {
        context "termset";
        construct variant NuhiTermset;
        fields {
            field nuhi = cmavo(Nuhi).wf();
            field termset = many1(boxed(term));
            field nuhu = opt(cmavo(Nuhu).wf());
        }
    }

    node ke_termset(term) -> TermSyntax {
        context "termset";
        construct variant KeTermset;
        fields {
            field ke = cmavo(Ke).warn(ExperimentalKeTermset).wf();
            field termset = many1(boxed(term));
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    alias noiha_adverbial_term(selbri) -> TermSyntax {
        context "NOIhA adverbial";
        choice((
            noiha_variable_adverbial_term(selbri),
            noiha_relative_adverbial_term(selbri),
        ));
    }

    node noiha_variable_adverbial_term(selbri) -> TermSyntax {
        context "NOIhA adverbial";
        construct variant BridiVariableAdverbialTerm;
        fields {
            field poiha = selmaho(Noiha).wf();
            default tail_elements: Vec<DescriptionTailElementSyntax> = Vec::new();
            field selbri = some(boxed(selbri));
            default relative_clauses: Vec<RelativeClauseSyntax> = Vec::new();
            field brigahi_ku = cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
        }
    }

    node noiha_relative_adverbial_term(selbri) -> TermSyntax {
        context "NOIhA adverbial";
        construct variant RelativeAdverbialTerm;
        fields {
            field noiha = selmaho(Noiha).wf();
            default tail_elements: Vec<DescriptionTailElementSyntax> = Vec::new();
            field selbri = some(boxed(selbri));
            default relative_clauses: Vec<RelativeClauseSyntax> = Vec::new();
            field fehu = opt(cmavo(Fehu).wf());
        }
    }

    node fihoi_adverbial_term(subbridi) -> TermSyntax {
        context "FIhOI adverbial";
        construct variant AdHocBridiAdverbialTerm;
        fields {
            field fihoi = cmavo(Fihoi).wf();
            field subbridi = boxed(subbridi);
            field fihau = opt(cmavo(Fihau).wf());
        }
    }

    node soi_adverbial_term(subbridi) -> TermSyntax {
        context "SOI adverbial";
        construct variant ReciprocalBridiAdverbialTerm;
        fields {
            field soi = selmaho(Soi).wf();
            field subbridi = boxed(subbridi);
            field sehu = opt(cmavo(Sehu).wf());
        }
    }

    node sumti_term(sumti) -> TermSyntax {
        context "term";
        construct tuple_variant Sumti;
        fields {
            field sumti = boxed(sumti);
        }
    }

    node place_tagged_sumti_term(sumti) -> TermSyntax {
        context "place tag";
        construct variant PlaceTaggedSumti;
        fields {
            field fa = selmaho(Fa).wf();
            field sumti = boxed(choice((
                sumti,
                tagged_elided_sumti(),
            )));
            default ku: Option<WithFreeModifiers<Token, FreeModifierSyntax>> = None;
        }
    }

    node na_ku_term -> TermSyntax {
        context "NA KU term";
        construct variant BridiNegation;
        fields {
            field na = selmaho(Na);
            field na_ku = cmavo(Ku).wf();
        }
    }

    node bare_na_term(selbri, tense_modal) -> TermSyntax {
        context "NA term";
        construct tuple_variant BareNegation;
        fields {
            field na = selmaho(Na).wf();
            require bare_na_term_forbidden_follow(selbri, tense_modal).not();
        }
    }

    alias bare_na_term_forbidden_follow(selbri, tense_modal) -> () {
        context "NA term";
        choice((
            bare_na_selbri_follow(selbri),
            bare_na_modal_forethought_follow(tense_modal),
            bare_na_ja_follow(),
            bare_na_a_follow(),
            bare_na_giha_follow(),
        ));
    }

    product bare_na_selbri_follow(selbri) -> () {
        context "NA term";
        fields {
            require selbri;
        }
    }

    product bare_na_modal_forethought_follow(tense_modal) -> () {
        context "NA term";
        fields {
            require modal_forethought_connective(tense_modal);
        }
    }

    product bare_na_ja_follow -> () {
        context "NA term";
        fields {
            require selmaho(Ja);
        }
    }

    product bare_na_a_follow -> () {
        context "NA term";
        fields {
            require opt(selmaho(Se));
            require selmaho(A);
        }
    }

    product bare_na_giha_follow -> () {
        context "NA term";
        fields {
            require opt(selmaho(Se));
            require selmaho(Giha);
        }
    }

    node tagged_sumti_before_tag_term(tense_modal, selbri) -> TermSyntax {
        context "tag";
        fields {
            require modal_forethought_connective(tense_modal).not();
            field tense_modal = boxed(leading_term_tag_tense_modal(tense_modal, selbri));
            require tense_modal.lookahead();
        }
    }

    node tagged_sumti_term(tense_modal, sumti, selbri) -> TermSyntax {
        context "tag";
        construct variant TaggedSumti;
        fields {
            require modal_forethought_connective(tense_modal).not();
            field tense_modal = some(boxed(leading_term_tag_tense_modal(tense_modal, selbri)));
            require selbri.not();
            field sumti = boxed(choice((
                sumti,
                tagged_elided_sumti(),
            )));
        }
    }

    alias leading_term_tag_tense_modal(tense_modal, selbri) -> TenseModalSyntax {
        context "tag";
        choice((
            pu_before_nahe_leading_term_tag_tense(),
            pu_distance_before_tag_leading_term_tag_tense(),
            zi_before_zi_leading_term_tag_tense(),
            va_before_va_leading_term_tag_tense(),
            mohi_before_mohi_leading_term_tag_tense(),
            caha_before_tag_leading_term_tag_tense(tense_modal),
            interval_property_leading_term_tag_tense(selbri),
            tense_modal,
        ));
    }

    node pu_before_nahe_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field pu = selmaho(Pu).wf();
            field nai = opt(cmavo(Nai).wf());
            require selmaho(Nahe).lookahead();
        }
    }

    node pu_distance_before_tag_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field pu = selmaho(Pu).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = selmaho(Zi).wf();
            require selmaho(Zi).lookahead();
        }
    }

    node zi_before_zi_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field zi = selmaho(Zi).wf();
            require selmaho(Zi).lookahead();
        }
    }

    node va_before_va_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field va = selmaho(Va).wf();
            require selmaho(Va).lookahead();
        }
    }

    node mohi_before_mohi_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field mohi = selmaho(Mohi).wf();
            field direction = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Va).wf());
            require selmaho(Mohi).lookahead();
        }
    }

    node caha_before_tag_leading_term_tag_tense(tense_modal) -> TenseModalSyntax {
        context "tag";
        construct tuple_variant Actuality;
        fields {
            field caha = selmaho(Caha).wf().followed_by(tense_modal.lookahead());
        }
    }

    alias interval_property_leading_term_tag_tense(selbri) -> TenseModalSyntax {
        context "interval property";
        interval_property_tense().followed_by(leading_interval_property_follower(selbri).lookahead());
    }

    alias leading_interval_property_follower(selbri) -> () {
        context "tag";
        choice((
            pu_leading_interval_property_follower(),
            zi_leading_interval_property_follower(),
            zeha_leading_interval_property_follower(),
            nahe_caha_leading_interval_property_follower(),
            modal_leading_interval_property_follower(),
            fiho_leading_interval_property_follower(selbri),
        ));
    }

    product pu_leading_interval_property_follower -> () {
        context "tag";
        fields {
            require selmaho(Pu);
        }
    }

    product zi_leading_interval_property_follower -> () {
        context "tag";
        fields {
            require selmaho(Zi);
        }
    }

    product zeha_leading_interval_property_follower -> () {
        context "tag";
        fields {
            require selmaho(Zeha);
        }
    }

    product nahe_caha_leading_interval_property_follower -> () {
        context "tag";
        fields {
            require selmaho(Nahe);
            require selmaho(Caha);
        }
    }

    product modal_leading_interval_property_follower -> () {
        context "modal tag";
        fields {
            require modal_tense();
        }
    }

    product fiho_leading_interval_property_follower(selbri) -> () {
        context "FIhO modal";
        fields {
            require fiho_tense(selbri);
        }
    }

    node tagged_elided_sumti -> SumtiSyntax {
        context "elided sumti";
        construct variant ElidedSumti;
        fields {
            default tag: Option<Box<SumtiTagSyntax>> = None;
            field maybe_ku = opt(cmavo(Ku).wf());
            default free_modifiers: Vec<FreeModifierSyntax> = Vec::new();
        }
    }

    node jai_tagged_sumti_term(tense_modal, sumti) -> TermSyntax {
        context "tag";
        construct variant JaiTaggedSumti;
        fields {
            field jai = cmavo(Jai).warn(ExperimentalZantufaJaiTagTerm).wf();
            field tag = opt(boxed(tense_modal));
            field sumti = boxed(sumti);
        }
    }

    node sumti(sumti, sumti_grouped, subbridi, tense_modal) -> SumtiSyntax {
        context "sumti";
        fields {
            field base_sumti = boxed(sumti_grouped);
            field vuho_attachment = opt(vuho_sumti_attachment_tail(sumti, subbridi, tense_modal));
        }
    }

    node sumti_grouped(sumti, sumti_afterthought, tense_modal) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_afterthought);
            field grouped_tail = opt(grouped_sumti_tail(sumti, tense_modal));
        }
    }

    node sumti_afterthought(sumti_bound) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_bound);
            field continuations = many(sumti_afterthought_tail(sumti_bound));
        }
    }

    node sumti_bound(sumti_bound, sumti_forethought, tense_modal) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_forethought);
            field bound_tail = opt(bound_sumti_tail(sumti_bound, tense_modal));
        }
    }

    alias sumti_forethought(sumti, sumti_forethought, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti";
        choice((
            forethought_sumti(sumti, sumti_forethought, tense_modal),
            simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens),
        ));
    }

    node forethought_sumti(sumti, sumti_forethought, tense_modal) -> SumtiSyntax {
        context "forethought sumti connection";
        construct variant ForethoughtSumtiConnection;
        fields {
            field gek = modal_forethought_connective(tense_modal);
            field leading_sumti = boxed(sumti);
            field gik = gik_connective;
            field trailing_sumti = boxed(sumti_forethought);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
        }
    }

    product bound_sumti_tail(sumti_bound, tense_modal) -> BoundSumtiTailSyntax {
        context "sumti connection";
        construct direct;
        fields {
            field connective = boxed(argument_connective);
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field trailing_sumti = boxed(sumti_bound);
        }
    }

    product sumti_afterthought_tail(sumti_bound) -> SumtiConnectionSyntax {
        context "sumti connective";
        construct direct;
        fields {
            field connective = argument_connective;
            field sumti = boxed(sumti_bound);
        }
    }

    product grouped_sumti_tail(sumti, tense_modal) -> GroupedSumtiTailSyntax {
        context "sumti connection";
        construct direct;
        fields {
            field connective = argument_connective;
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field inner_sumti = boxed(sumti);
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    alias vuho_sumti_attachment_tail(sumti, subbridi, tense_modal) -> VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        choice((
            vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal),
            vuho_connected_sumti_attachment_tail(sumti),
        ));
    }

    product vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal) -> VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        construct direct;
        fields {
            field vuho = cmavo(Vuho).wf();
            field relative_clauses = relative_clause_list(sumti, subbridi, tense_modal);
            field sumti_connection = opt(boxed(sumti_connection_tail(sumti)));
        }
    }

    product vuho_connected_sumti_attachment_tail(sumti) -> VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        construct direct;
        fields {
            field vuho = cmavo(Vuho).wf();
            default relative_clauses: Vec<RelativeClauseSyntax> = Vec::new();
            field sumti_connection = some(boxed(sumti_connection_tail(sumti)));
        }
    }

    node simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti";
        fields {
            field base_sumti = boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    alias sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti";
        choice((
            sumti_base,
            quantified_sumti(sumti_base, mekso, letter_tokens),
        ));
    }

    alias sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier) -> SumtiSyntax {
        context "sumti";
        choice((
            scalar_negated_sumti_with_bo(sumti),
            scalar_negated_sumti(sumti),
            lahe_sumti(sumti, subbridi, tense_modal),
            lahe_term_wrapper(term),
            scalar_negated_term_wrapper_with_bo(term),
            scalar_negated_term_wrapper(term),
            bridi_description_sumti(subbridi),
            name_sumti(),
            description_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            number_sumti(mekso),
            lerfu_string_sumti(letter_string, free_modifier),
            compound_quote_sumti(),
            pro_sumti(),
            text_quote_sumti(text),
        ));
    }

    node quantified_sumti(sumti_base, mekso, letter_tokens) -> SumtiSyntax {
        context "quantified sumti";
        construct variant QuantifiedSumti;
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            field inner_sumti = boxed(sumti_base);
        }
    }

    node connected_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "sumti connection";
        construct variant SumtiConnection;
        fields {
            field leading_sumti = boxed(simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            field connective = argument_connective;
            field trailing_sumti = boxed(simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
        }
    }

    node grouped_sumti_connection(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            field connective = argument_connective;
            field tense_modal = opt(boxed(tense_modal));
            field ke = cmavo(Ke).wf();
            field inner_sumti = boxed(sumti);
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    node sumti_with_relative_clauses(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti relative phrase";
        construct variant SumtiWithRelativeClauses;
        fields {
            field base_sumti = boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            default vuho: Option<WithFreeModifiers<Token, FreeModifierSyntax>> = None;
            scratch first_relative_clause = relative_clause_atom(sumti, subbridi, tense_modal);
            scratch additional_relative_clauses = many(relative_clause_tail(sumti, subbridi, tense_modal));
            let relative_clauses: Vec<RelativeClauseSyntax> = std::iter::once(first_relative_clause)
                .chain(additional_relative_clauses)
                .collect();
        }
    }

    node vuho_sumti_attachment(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "sumti relative phrase";
        fields {
            field base_sumti = boxed(choice((
                connected_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
                sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens),
            )));
            field vuho = cmavo(Vuho).wf();
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field sumti_connection = opt(sumti_connection_tail(sumti));
        }
    }

    node sumti_connection_tail(sumti) -> SumtiConnectionSyntax {
        context "sumti connective";
        construct direct;
        fields {
            field connective = argument_connective;
            field sumti = boxed(sumti);
        }
    }

    node pa_run_quantifier(letter_tokens) -> QuantifierSyntax {
        context "quantifier";
        construct variant NumberQuantifier;
        fields {
            scratch number_words = number_words(letter_tokens).wf();
            let number: WithFreeModifiers<WordRun, FreeModifierSyntax> = WithFreeModifiers::new(
                WordRun::try_from_vec(number_words.value).expect("many1 guarantees non-empty number words"),
                number_words.free_modifiers,
            );
            field boi = opt(cmavo(Boi).wf());
        }
    }

    node mekso_quantifier(mekso) -> QuantifierSyntax {
        context "quantifier";
        construct variant MeksoQuantifier;
        fields {
            field vei = cmavo(Vei).wf();
            field mekso = boxed(mekso);
            field veho = opt(cmavo(Veho).wf());
        }
    }

    alias quantifier(mekso, letter_tokens) -> QuantifierSyntax {
        context "quantifier";
        choice((
            mekso_quantifier(mekso),
            pa_run_quantifier(letter_tokens),
        ));
    }

    node number_mekso(letter_tokens) -> MeksoSyntax {
        context "number mex";
        construct tuple_variant NumberMekso;
        fields {
            field quantifier = boxed(pa_run_quantifier(letter_tokens));
        }
    }

    node primitive_mekso_operator -> MeksoOperatorSyntax {
        context "operator";
        construct tuple_variant Primitive;
        fields {
            field vuhu = selmaho(Vuhu).wf();
        }
    }

    alias mekso_operator(mekso, mekso_operator, selbri) -> MeksoOperatorSyntax {
        context "operator";
        choice((
            afterthought_mekso_operator(mekso, mekso_operator, selbri),
            bound_mekso_operator(mekso, mekso_operator, selbri),
            mekso_operator_atom(mekso, mekso_operator, selbri),
        ));
    }

    node afterthought_mekso_operator(mekso, mekso_operator, selbri) -> MeksoOperatorSyntax {
        context "operator";
        fields {
            field leading_operator = boxed(bound_or_atom_mekso_operator(mekso, mekso_operator, selbri));
            field continuations = many((standard_statement_connective, boxed(bound_or_atom_mekso_operator(mekso, mekso_operator, selbri))));
        }
    }

    alias bound_or_atom_mekso_operator(mekso, mekso_operator, selbri) -> MeksoOperatorSyntax {
        context "operator";
        choice((
            bound_mekso_operator(mekso, mekso_operator, selbri),
            mekso_operator_atom(mekso, mekso_operator, selbri),
        ));
    }

    node bound_mekso_operator(mekso, mekso_operator, selbri) -> MeksoOperatorSyntax {
        context "operator";
        construct variant BoundOperatorConnection;
        fields {
            field left_operator = boxed(mekso_operator_atom(mekso, mekso_operator, selbri));
            field connective = standard_statement_connective;
            field bo = cmavo(Bo).wf();
            field right_operator = boxed(mekso_operator);
        }
    }

    alias mekso_operator_atom(mekso, mekso_operator, selbri) -> MeksoOperatorSyntax {
        context "operator";
        choice((
            converted_mekso_operator(mekso_operator),
            scalar_negated_mekso_operator(mekso_operator),
            forethought_mekso_operator(mekso_operator),
            grouped_mekso_operator(mekso_operator),
            selbri_mekso_operator(selbri),
            operand_mekso_operator(mekso),
            primitive_mekso_operator(),
        ));
    }

    node converted_mekso_operator(mekso_operator) -> MeksoOperatorSyntax {
        context "converted operator";
        construct variant Converted;
        fields {
            field se = selmaho(Se).wf();
            field inner_operator = boxed(mekso_operator);
        }
    }

    node scalar_negated_mekso_operator(mekso_operator) -> MeksoOperatorSyntax {
        context "converted operator";
        construct variant ScalarNegated;
        fields {
            field nahe = selmaho(Nahe).wf();
            field inner_operator = boxed(mekso_operator);
        }
    }

    node forethought_mekso_operator(mekso_operator) -> MeksoOperatorSyntax {
        context "operator";
        fields {
            field guhek = guhek_connective;
            field left_operator = boxed(mekso_operator);
            field gik = gik_connective;
            field right_operator = boxed(mekso_operator);
        }
    }

    node grouped_mekso_operator(mekso_operator) -> MeksoOperatorSyntax {
        context "grouped operator";
        construct variant GroupedOperator;
        fields {
            field ke = cmavo(Ke).wf();
            field inner_operator = boxed(mekso_operator);
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    node selbri_mekso_operator(selbri) -> MeksoOperatorSyntax {
        context "selbri-to-operator";
        construct variant SelbriAsOperator;
        fields {
            field nahu = cmavo(Nahu).wf();
            field selbri = boxed(selbri);
            field tehu = opt(cmavo(Tehu).wf());
        }
    }

    node operand_mekso_operator(mekso) -> MeksoOperatorSyntax {
        context "operand-to-operator";
        construct variant OperandAsOperator;
        fields {
            field maho = cmavo(Maho).wf();
            field mekso = boxed(mekso);
            field tehu = opt(cmavo(Tehu).wf());
        }
    }

    alias mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> MeksoSyntax {
        context "operand";
        choice((
            afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
        ));
    }

    node afterthought_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> MeksoSyntax {
        context "operand connective";
        fields {
            field leading_expression = boxed(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
            field continuations = many((operand_connective, boxed(bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier))));
        }
    }

    alias bound_or_simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> MeksoSyntax {
        context "operand";
        choice((
            bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
            simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier),
        ));
    }

    node bound_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> MeksoSyntax {
        context "operand connective";
        construct variant BoundMeksoOperandConnection;
        fields {
            field left_expression = boxed(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
            field operand_connective = operand_connective;
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
            field right_expression = boxed(mekso_operand);
        }
    }

    alias simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier) -> MeksoSyntax {
        context "operand";
        choice((
            forethought_mekso_operand(mekso_operand, tense_modal),
            qualified_mekso_operand(mekso_operand),
            parenthesized_mekso_operand(mekso),
            sumti_mekso_operand(sumti),
            selbri_mekso_operand(selbri),
            array_mekso_operand(mekso),
            number_mekso(letter_tokens),
            lerfu_string_mekso(letter_string, free_modifier),
        ));
    }

    node qualified_mekso_operand(mekso_operand) -> MeksoSyntax {
        context "qualified operand";
        construct variant QualifiedOperand;
        fields {
            scratch nahe = selmaho(Nahe);
            scratch bo = cmavo(Bo);
            let markers: WithFreeModifiers<Vec<Token>, FreeModifierSyntax> = WithFreeModifiers::new(vec![nahe, bo], Vec::new());
            field inner_expression = boxed(mekso_operand);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node forethought_mekso_operand(mekso_operand, tense_modal) -> MeksoSyntax {
        context "forethought mex";
        construct variant ForethoughtMeksoConnection;
        fields {
            field gek = modal_forethought_connective(tense_modal);
            field left_expression = boxed(mekso_operand);
            field gik = gik_connective;
            field right_expression = boxed(mekso_operand);
        }
    }

    node sumti_mekso_operand(sumti) -> MeksoSyntax {
        context "sumti operand";
        construct variant SumtiOperand;
        fields {
            field mohe = cmavo(Mohe).wf();
            field sumti = boxed(sumti);
            field tehu = opt(cmavo(Tehu).wf());
        }
    }

    node selbri_mekso_operand(selbri) -> MeksoSyntax {
        context "selbri operand";
        construct variant SelbriOperand;
        fields {
            field nihe = cmavo(Nihe).wf();
            field selbri = boxed(selbri);
            field tehu = opt(cmavo(Tehu).wf());
        }
    }

    node parenthesized_mekso_operand(mekso) -> MeksoSyntax {
        context "parenthesized mex";
        construct variant ParenthesizedMekso;
        fields {
            field vei = cmavo(Vei).wf();
            field inner_expression = boxed(mekso);
            field veho = opt(cmavo(Veho).wf());
        }
    }

    node array_mekso_operand(mekso) -> MeksoSyntax {
        context "mekso array";
        construct variant MeksoArray;
        fields {
            field johi = cmavo(Johi).wf();
            scratch expression_items = many1(mekso);
            let expressions: MeksoVec = MeksoVec::try_from_vec(expression_items)
                .expect("many1 guarantees non-empty mex array");
            field tehu = opt(cmavo(Tehu).wf());
        }
    }

    alias letter_string(letter_tokens) -> std::vec::Vec<Token> {
        context "lerfu string";
        concat(
            letter_tokens,
            many(choice((
                pa_word_as_words(),
                letter_tokens,
            ))),
        );
    }

    alias number_words(letter_tokens) -> std::vec::Vec<Token> {
        context "number";
        concat(
            pa_word_as_words(),
            many(choice((
                pa_word_as_words(),
                letter_tokens,
            ))),
        );
    }

    alias number_or_letter_words(letter_tokens, letter_string) -> std::vec::Vec<Token> {
        context "number or lerfu string";
        choice((
            number_words(letter_tokens),
            letter_string,
        ));
    }

    product number_or_letter_mekso(letter_tokens, letter_string, free_modifier) -> MeksoSyntax {
        context "number or lerfu string";
        fields {
            field words = number_or_letter_words(letter_tokens, letter_string);
            field boi = opt(cmavo(Boi));
            field free_modifiers = many(free_modifier);
        }
    }

    alias letter_tokens(letter_string, letter_tokens) -> std::vec::Vec<Token> {
        context "lerfu word";
        choice((
            plain_letter_word_as_words(),
            lau_letter_tokens(letter_tokens),
            tei_letter_tokens(letter_string),
        ));
    }

    alias pa_word_as_words -> std::vec::Vec<Token> {
        context "number";
        singleton(pa_word());
    }

    alias plain_letter_word_as_words -> std::vec::Vec<Token> {
        context "lerfu word";
        singleton(word_category(LetterWord));
    }

    alias lau_letter_tokens(letter_tokens) -> std::vec::Vec<Token> {
        context "lerfu word";
        prepend(selmaho(Lau), letter_tokens);
    }

    alias tei_letter_tokens(letter_string) -> std::vec::Vec<Token> {
        context "lerfu word";
        prepend(
            cmavo(Tei),
            append(letter_string, singleton(cmavo(Foi))),
        );
    }

    node lerfu_string_mekso(letter_string, free_modifier) -> MeksoSyntax {
        context "lerfu string";
        fields {
            field letters = letter_string;
            field boi = opt(cmavo(Boi));
            field free_modifiers = many(free_modifier);
        }
    }

    alias mekso_base(mekso_base, mekso_operand, mekso_operator) -> MeksoSyntax {
        context "mex";
        choice((
            mekso_operand,
            forethought_call_mekso(mekso_base, mekso_operator),
        ));
    }

    node mekso_precedence(mekso_base, mekso_precedence, mekso_operator) -> MeksoSyntax {
        context "mex";
        fields {
            field left_expression = boxed(mekso_base);
            field tail = opt((cmavo(Bihe).wf(), boxed(mekso_operator), boxed(mekso_precedence)));
        }
    }

    node infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> MeksoSyntax {
        context "mex";
        fields {
            field first_expression = boxed(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
            field continuations = many((boxed(mekso_operator), boxed(mekso_precedence)));
        }
    }

    node forethought_call_mekso(mekso_base, mekso_operator) -> MeksoSyntax {
        context "forethought mex";
        construct variant ForethoughtCall;
        fields {
            field peho = opt(cmavo(Peho).wf());
            field operator = boxed(mekso_operator);
            field operands = many1(mekso_base);
            field kuhe = opt(cmavo(Kuhe).wf());
        }
    }

    alias mekso(mekso_base, mekso_precedence, mekso_operator, reverse_polish_parts) -> MeksoSyntax {
        context "mex";
        choice((
            infix_mekso(mekso_base, mekso_precedence, mekso_operator),
            reverse_polish_mekso(reverse_polish_parts),
        ));
    }

    product reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> ReversePolishPartsSyntax {
        context "reverse Polish mex";
        fields {
            field first_operand = boxed(mekso_operand);
            field tails = many((boxed(reverse_polish_parts), mekso_operator));
        }
    }

    node reverse_polish_mekso(reverse_polish_parts) -> MeksoSyntax {
        context "reverse Polish mex";
        construct variant ReversePolish;
        fields {
            field fuha = cmavo(Fuha).wf();
            field parts = boxed(reverse_polish_parts);
        }
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
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind: SumtiWrapperKindSyntax = SumtiWrapperKindSyntax::Referent;
            field wrapper = selmaho(Lahe).wf();
            default wrapper_bo: Option<WithFreeModifiers<Token, FreeModifierSyntax>> = None;
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper_with_bo(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind: SumtiWrapperKindSyntax = SumtiWrapperKindSyntax::ScalarNegationWithBo;
            scratch raw_wrapper = selmaho(Nahe);
            let wrapper: WithFreeModifiers<Token, FreeModifierSyntax> = WithFreeModifiers::new(raw_wrapper, Vec::new());
            field wrapper_bo = some(cmavo(Bo).wf());
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind: SumtiWrapperKindSyntax = SumtiWrapperKindSyntax::ScalarNegation;
            field wrapper = selmaho(Nahe).wf();
            default wrapper_bo: Option<WithFreeModifiers<Token, FreeModifierSyntax>> = None;
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

    alias description_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        choice((
            description_connection_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens),
            descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens),
        ));
    }

    node description_head -> DescriptionHeadSyntax {
        context "descriptor";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
        }
    }

    node description_head_connective -> ConnectiveSyntax {
        context "descriptor connective";
        construct variant DescriptionHeadConnective;
        fields {
            field connective = boxed(jek_connective);
        }
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

    product description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens) -> DescriptionTailSyntax {
        context "description tail";
        fields {
            field leading_tail_elements = leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal);
            field tail = boxed(choice((
                quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens),
                quantifier_sumti_description_tail(sumti, mekso, letter_tokens),
                relation_description_tail(sumti, subbridi, selbri, tense_modal),
            )));
        }
    }

    product leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal) -> LeadingDescriptionTailElementsSyntax {
        context "description tail";
        fields {
            field tail_sumti = opt(description_tail_sumti(sumti_base));
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    product description_tail_sumti(sumti_base) -> DescriptionTailSumtiSyntax {
        context "description tail";
        fields {
            require pa_word().not();
            field sumti = boxed(sumti_base);
        }
    }

    product relation_description_tail(sumti, subbridi, selbri, tense_modal) -> DescriptionTailSyntax {
        context "description tail";
        fields {
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    product quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> DescriptionTailSyntax {
        context "description tail";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            require selmaho(Roi).not();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
    }

    product quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> DescriptionTailSyntax {
        context "description tail";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            field sumti = boxed(sumti);
        }
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

    node text_quote(text) -> QuoteSyntax {
        context "text quote";
        construct variant TextQuote;
        fields {
            field lu = cmavo(Lu).wf();
            field text = boxed(text);
            field lihu = opt(cmavo(Lihu).wf());
        }
    }

    node text_quote_sumti(text) -> SumtiSyntax {
        context "text quote";
        construct tuple_variant QuotedSumti;
        fields {
            field quote = boxed(text_quote(text));
        }
    }

    alias compound_quote -> QuoteSyntax {
        context "quote";
        choice((
            experimental_mehoi_compound_quote(),
            experimental_zohoi_compound_quote(),
            experimental_rahoi_compound_quote(),
            experimental_gohoi_compound_quote(),
            generic_compound_quote(),
        ));
    }

    node experimental_mehoi_compound_quote -> QuoteSyntax {
        context "quote";
        construct tuple_variant DelimitedWordQuote;
        fields {
            field quote = quote_marker(Mehoi).warn(ExperimentalMehOiQuote).wf();
        }
    }

    node experimental_zohoi_compound_quote -> QuoteSyntax {
        context "quote";
        construct tuple_variant DelimitedWordQuote;
        fields {
            field quote = choice((
                quote_marker(Zohoi),
                quote_marker(Lahoi),
            )).warn(ExperimentalZohOiQuote).wf();
        }
    }

    node experimental_rahoi_compound_quote -> QuoteSyntax {
        context "quote";
        construct tuple_variant DelimitedWordQuote;
        fields {
            field quote = quote_marker(Rahoi).warn(ExperimentalZantufaRahoiQuote).wf();
        }
    }

    node experimental_gohoi_compound_quote -> QuoteSyntax {
        context "quote";
        construct tuple_variant DelimitedWordQuote;
        fields {
            field quote = choice((
                quote_marker(Gohoi),
                quote_marker(Zehoi),
                quote_marker(Tahai),
                quote_marker(Bohei),
            )).warn(ExperimentalGohoiSelbriUnit).wf();
        }
    }

    node generic_compound_quote -> QuoteSyntax {
        context "quote";
        fields {
            field quote = word_category(Quote).wf();
        }
    }

    node compound_quote_sumti -> SumtiSyntax {
        context "quote";
        construct tuple_variant QuotedSumti;
        fields {
            field quote = boxed(compound_quote());
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

    alias vocative_argument(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "vocative phrase";
        choice((
            selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal),
            cmevla_vocative_sumti(sumti, subbridi, tense_modal),
            sumti,
        ));
    }

    node coi_vocative_marker_words -> VocativeMarkerWordsSyntax {
        context "vocative marker";
        fields {
            field first_coi = selmaho(Coi);
            field first_nai = opt(cmavo(Nai));
            field additional_coi = many((selmaho(Coi), opt(cmavo(Nai))));
            field doi = opt(cmavo(Doi));
        }
    }

    node doi_vocative_marker_words -> VocativeMarkerWordsSyntax {
        context "vocative marker";
        fields {
            field doi = cmavo(Doi);
        }
    }

    node vocative_free_modifier(sumti, subbridi, selbri, tense_modal) -> FreeModifierSyntax {
        context "vocative phrase";
        construct variant Vocative;
        fields {
            field vocative_markers = choice((
                coi_vocative_marker_words(),
                doi_vocative_marker_words(),
            )).wf();
            field sumti = opt(boxed(vocative_argument(sumti, subbridi, selbri, tense_modal)));
            field dohu = opt(cmavo(Dohu).prohibited_wf());
        }
    }

    node parenthetical_text(text) -> FreeModifierSyntax {
        context "parenthetical text";
        construct variant ParentheticalText;
        fields {
            field to = selmaho(To).wf();
            field text = boxed(text);
            field toi = opt(cmavo(Toi).prohibited_wf());
        }
    }

    node sei_free_modifier(term, selbri) -> FreeModifierSyntax {
        context "metalinguistic bridi";
        construct variant MetalinguisticBridi;
        fields {
            field sei = selmaho(Sei).wf();
            field terms = many(term);
            field cu = opt(cmavo(Cu).wf());
            field selbri = boxed(selbri);
            field sehu = opt(cmavo(Sehu).prohibited_wf());
        }
    }

    node xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier) -> FreeModifierSyntax {
        context "subscript";
        construct variant Subscript;
        fields {
            field xi = selmaho(Xi).wf();
            field expression = boxed(choice((
                number_or_letter_mekso(letter_tokens, letter_string, free_modifier),
                mekso,
            )));
        }
    }

    node mai_free_modifier(letter_tokens, letter_string) -> FreeModifierSyntax {
        context "utterance ordinal";
        construct variant UtteranceOrdinal;
        fields {
            field number = number_or_letter_words(letter_tokens, letter_string);
            field mai = selmaho(Mai).wf();
        }
    }

    node soi_free_modifier(sumti) -> FreeModifierSyntax {
        context "reciprocal";
        construct variant ReciprocalSumti;
        fields {
            field soi = cmavo(Soi).wf();
            field leading_sumti = boxed(sumti);
            field trailing_sumti = opt(boxed(sumti));
            field sehu = opt(cmavo(Sehu).wf());
        }
    }

    alias text_replacement_free_modifier -> FreeModifierSyntax {
        context "replacement free modifier";
        choice((
            full_text_replacement_free_modifier(),
            new_only_text_replacement_free_modifier(),
            close_only_text_replacement_free_modifier(),
        ));
    }

    node full_text_replacement_free_modifier -> FreeModifierSyntax {
        context "replacement free modifier";
        construct variant TextReplacement;
        fields {
            field lohai = some(cmavo(Lohai));
            field old_words = raw_words_until(Sahai, Lehai);
            field sahai = opt(cmavo(Sahai));
            field new_words = raw_words_until(Lehai);
            field lehai = cmavo(Lehai).wf();
        }
    }

    node new_only_text_replacement_free_modifier -> FreeModifierSyntax {
        context "replacement free modifier";
        construct variant TextReplacement;
        fields {
            default lohai: Option<Token> = None;
            default old_words: Vec<Token> = Vec::new();
            field sahai = some(cmavo(Sahai));
            field new_words = raw_words_until(Lehai);
            field lehai = cmavo(Lehai).wf();
        }
    }

    node close_only_text_replacement_free_modifier -> FreeModifierSyntax {
        context "replacement free modifier";
        construct variant TextReplacement;
        fields {
            default lohai: Option<Token> = None;
            default old_words: Vec<Token> = Vec::new();
            default sahai: Option<Token> = None;
            default new_words: Vec<Token> = Vec::new();
            field lehai = cmavo(Lehai).wf();
        }
    }

    alias free_modifier(sumti, subbridi, selbri, text, mekso, term, tense_modal, letter_tokens, letter_string, free_modifier) -> FreeModifierSyntax {
        context "free modifier";
        choice((
            text_replacement_free_modifier(),
            sei_free_modifier(term, selbri),
            xi_free_modifier(mekso, letter_tokens, letter_string, free_modifier),
            mai_free_modifier(letter_tokens, letter_string),
            soi_free_modifier(sumti),
            parenthetical_text(text),
            vocative_free_modifier(sumti, subbridi, selbri, tense_modal),
        ));
    }

    alias relative_clause_tail(sumti, subbridi, tense_modal) -> RelativeClauseSyntax {
        context "relative clauses";
        choice((
            joined_relative_clause_tail(sumti, subbridi, tense_modal),
            connected_relative_clause_tail(sumti, subbridi, tense_modal),
        ));
    }

    node joined_relative_clause_tail(sumti, subbridi, tense_modal) -> RelativeClauseSyntax {
        context "relative clause";
        construct variant JoinedRelativeClauses;
        fields {
            field zihe = cmavo(Zihe).wf();
            field inner = boxed(relative_clause_atom(sumti, subbridi, tense_modal));
        }
    }

    node connected_relative_clause_tail(sumti, subbridi, tense_modal) -> RelativeClauseSyntax {
        context "relative clause";
        construct variant RelativeClauseConnection;
        fields {
            field connective = relative_clause_connective;
            field inner = boxed(relative_clause_atom(sumti, subbridi, tense_modal));
        }
    }

    alias relative_clause_atom(sumti, subbridi, tense_modal) -> RelativeClauseSyntax {
        context "relative clause";
        choice((
            sumti_association_relative_clause(sumti, tense_modal),
            bridi_relative_clause(subbridi),
        ));
    }

    node sumti_association_relative_clause(sumti, tense_modal) -> RelativeClauseSyntax {
        context "sumti association phrase";
        construct variant SumtiAssociationPhrase;
        fields {
            field association_marker = selmaho(Goi).wf();
            field sumti = boxed(choice((
                tense_tagged_relative_sumti(tense_modal, sumti),
                na_ku_relative_sumti(),
                sumti,
            )));
            field gehu = opt(cmavo(Gehu).wf());
        }
    }

    node na_ku_relative_sumti -> SumtiSyntax {
        context "sumti association phrase";
        construct variant NegatedSumti;
        fields {
            field na = selmaho(Na);
            field ku = cmavo(Ku).wf();
        }
    }

    node tense_tagged_relative_sumti(tense_modal, sumti) -> SumtiSyntax {
        context "tagged sumti";
        fields {
            field tense_modal = boxed(tense_modal);
            field sumti = boxed(choice((
                sumti,
                tagged_elided_sumti(),
            )));
        }
    }

    alias bridi_relative_clause(subbridi) -> RelativeClauseSyntax {
        context "relative clause";
        choice((
            restrictive_bridi_relative_clause(subbridi),
            incidental_bridi_relative_clause(subbridi),
        ));
    }

    node restrictive_bridi_relative_clause(subbridi) -> RelativeClauseSyntax {
        context "relative clause";
        construct variant RestrictiveRelativeBridi;
        fields {
            field poi = choice((
                cmavo(Poi),
                cmavo(Pohoi),
            )).wf();
            field subbridi = boxed(subbridi);
            field kuho = opt(cmavo(Kuho).wf());
        }
    }

    node incidental_bridi_relative_clause(subbridi) -> RelativeClauseSyntax {
        context "relative clause";
        construct variant IncidentalRelativeBridi;
        fields {
            field noi = choice((
                cmavo(Noi),
                cmavo(Nohoi),
                cmavo(Voi),
                cmavo(Voihi),
            )).wf();
            field subbridi = boxed(subbridi);
            field kuho = opt(cmavo(Kuho).wf());
        }
    }

    alias relative_clause_connective -> ConnectiveSyntax {
        context "relative clause connective";
        choice((
            joik_connective(),
            jek_connective(),
        ));
    }

    alias joik_ek_connective -> ConnectiveSyntax {
        context "sumti connective";
        choice((
            joik_connective(),
            ek_connective(),
        ));
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

    alias joik_connective -> ConnectiveSyntax {
        context "joik";
        choice((
            joi_connective(),
            simple_interval_connective(),
            closed_interval_connective(),
        ));
    }

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

    alias argument_connective -> ConnectiveSyntax {
        context "sumti connective";
        choice((
            cehe_connective(),
            ek_connective(),
            jehi_connective(),
            joik_connective(),
            vuhu_nonlogical_connective(),
        ));
    }

    alias operand_connective -> ConnectiveSyntax {
        context "operand connective";
        choice((
            joik_connective(),
            ek_connective(),
            jek_connective(),
        ));
    }

    alias term_connective -> ConnectiveSyntax {
        context "term connective";
        choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        ));
    }

    alias relation_afterthought_connective -> ConnectiveSyntax {
        context "selbri connective";
        choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        ));
    }

    alias standard_statement_connective -> ConnectiveSyntax {
        context "statement connective";
        choice((
            joik_connective(),
            jek_connective(),
        ));
    }

    alias statement_connective -> ConnectiveSyntax {
        context "statement connective";
        choice((
            joik_connective(),
            jek_connective(),
            ek_connective(),
            vuhu_nonlogical_connective(),
        ));
    }

    alias i_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        choice((
            i_standard_statement_connective(tense_modal),
            i_tag_bo_statement_connective(tense_modal),
        ));
    }

    alias i_paragraph_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        choice((
            i_standard_paragraph_statement_connective(tense_modal),
            i_tag_bo_paragraph_statement_connective(tense_modal),
        ));
    }

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
            field connective = boxed(standard_paragraph_statement_connective);
            field tag_bo = opt((opt(boxed(tense_modal)), cmavo(Bo)));
        }
    }

    alias standard_paragraph_statement_connective -> ConnectiveSyntax {
        context "statement connective";
        choice((
            paragraph_joik_connective(),
            paragraph_jek_connective(),
        ));
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

    alias paragraph_joik_connective -> ConnectiveSyntax {
        context "joik";
        choice((
            paragraph_joi_connective(),
            paragraph_simple_interval_connective(),
            paragraph_closed_interval_connective(),
        ));
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

    alias bridi_tail_connective -> ConnectiveSyntax {
        context "bridi tail connective";
        choice((
            gihek_connective(),
            relation_connective_as_bridi_tail(),
        ));
    }

    product relation_connective_as_bridi_tail -> ConnectiveSyntax {
        context "bridi tail connective";
        construct variant RelationConnectiveAsBridiTail;
        fields {
            #[tree_child(primary)]
            field connective = boxed(relation_afterthought_connective);
        }
    }

    alias tag_connective -> ConnectiveSyntax {
        context "connected tag";
        choice((
            joik_connective(),
            jek_connective(),
        ));
    }

    alias modal_forethought_connective(tense_modal) -> ConnectiveSyntax {
        context "forethought connective";
        choice((
            ga_forethought_connective(),
            joik_jek_gi_forethought_connective(),
            jek_gi_forethought_connective(),
            modal_gi_forethought_connective(tense_modal),
            feature(ZantufaConnectives, zantufa_initial_gi_forethought_connective()),
        ));
    }

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

    alias tense_modal(selbri) -> TenseModalSyntax {
        context "tag";
        require choice((
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
        )).lookahead();
        choice((
            connected_tense_modal(selbri),
            tense_modal_atom(selbri),
        ));
    }

    node connected_tense_modal(selbri) -> TenseModalSyntax {
        context "connected tag";
        fields {
            field first = boxed(tense_modal_atom(selbri));
            field continuations = many1((tag_connective, boxed(tense_modal_atom(selbri))));
        }
    }

    alias tense_modal_atom(selbri) -> TenseModalSyntax {
        context "tag";
        choice((
            composite_tense(),
            fiho_tense(selbri),
            modal_tense(),
            flat_prefixed_tense(),
            feature(ZantufaTags, zantufa_recursive_tag_tense()),
            sticky_tense(),
        ));
    }

    node fiho_tense(selbri) -> TenseModalSyntax {
        context "FIhO modal";
        fields {
            field fiho = cmavo(Fiho).wf();
            field selbri = boxed(selbri);
            field fehu = opt(cmavo(Fehu).wf());
        }
    }

    alias flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        choice((
            nahe_se_flat_prefixed_tense(),
            se_flat_prefixed_tense(),
            fa_flat_tag_tense(),
        ));
    }

    node fa_flat_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field fa = selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        }
    }

    alias flat_tag_atom -> FlatTagAtomSyntax {
        context "tag";
        choice((
            fa_flat_tag_atom(),
            modal_flat_tag_atom(),
            composite_flat_tag_atom(),
        ));
    }

    product fa_flat_tag_atom -> FlatTagAtomSyntax {
        context "tag";
        construct variant Fa;
        fields {
            field fa = selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        }
    }

    product modal_flat_tag_atom -> FlatTagAtomSyntax {
        context "modal tag";
        construct variant Modal;
        fields {
            field modal = boxed(modal_tense());
        }
    }

    product composite_flat_tag_atom -> FlatTagAtomSyntax {
        context "tag";
        construct variant Composite;
        fields {
            field composite = boxed(composite_tense());
        }
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

    alias composite_tense -> TenseModalSyntax {
        context "tag";
        choice((
            prefixed_time_space_caha_tense(),
            time_space_caha_ki_tense(),
            cuhe_tense(),
        ));
    }

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

    alias time_space_caha_tense -> TenseModalSyntax {
        context "tag";
        choice((
            time_then_space_caha_tense(),
            space_then_time_caha_tense(),
            caha_tense(),
        ));
    }

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

    alias time_tense -> TenseModalSyntax {
        context "time tense";
        choice((
            time_tense_with_zi(),
            time_tense_with_offset(),
            time_tense_with_interval(),
            time_tense_with_properties(),
        ));
    }

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

    alias interval_property_tense -> TenseModalSyntax {
        context "interval property";
        choice((
            numbered_interval_property_tense(),
            tahe_interval_property_tense(),
            zaho_interval_property_tense(),
        ));
    }

    node numbered_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field number = interval_property_number_words().wf();
            field roi = selmaho(Roi).wf();
            field nai = opt(cmavo(Nai).wf());
        }
    }

    alias interval_property_number_words -> std::vec::Vec<Token> {
        context "number";
        concat(
            singleton(pa_word()),
            many(choice((
                pa_word_as_words(),
                plain_letter_word_as_words(),
            ))),
        );
    }

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

    alias space_tense -> TenseModalSyntax {
        context "space tense";
        choice((
            space_tense_with_va(),
            space_tense_with_offset(),
            space_tense_with_interval(),
            space_tense_with_mohi(),
        ));
    }

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

    alias space_interval_tense -> TenseModalSyntax {
        context "space interval";
        choice((
            space_interval_with_extent_tense(),
            space_interval_properties_tense(),
        ));
    }

    node space_interval_with_extent_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field extent = boxed(veha_viha_space_interval_tense());
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

    alias veha_viha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        choice((
            veha_space_interval_tense(),
            viha_space_interval_tense(),
        ));
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

    alias selbri(selbri, co_selbri, tense_modal) -> SelbriSyntax {
        context "selbri";
        choice((
            tagged_selbri(selbri, co_selbri, tense_modal),
            untagged_selbri(selbri, co_selbri),
        ));
    }

    alias untagged_selbri(selbri, co_selbri) -> SelbriSyntax {
        context "selbri";
        choice((
            negated_selbri(selbri),
            co_selbri,
            forethought_selbri_connection(selbri),
        ));
    }

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

    alias tanru_unit(bo_or_linked_tanru_unit) -> TanruUnitSyntax {
        context "tanru unit";
        connected_tanru_unit(bo_or_linked_tanru_unit);
    }

    node connected_tanru_unit(bo_or_linked_tanru_unit) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field leading_unit = boxed(bo_or_linked_tanru_unit);
            field continuations = many((relation_afterthought_connective, boxed(bo_or_linked_tanru_unit)));
        }
    }

    alias bo_or_linked_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        choice((
            forethought_selbri_group_tanru_unit(bo_or_linked_tanru_unit, selbri),
            bound_tanru_unit(bo_or_linked_tanru_unit, tanru_unit_atom, sumti, tense_modal),
            assigned_pro_bridi_tanru_unit(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string),
            linked_tanru_unit(tanru_unit_atom, sumti, tense_modal),
        ));
    }

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

    alias tanru_unit_base_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
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
    }

    node tanru_unit_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field conversions = many(selmaho(Se).wf());
            field base = boxed(tanru_unit_base_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
        }
    }

    alias tanru_unit_base_atom(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
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
    }

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
            field inner_unit = boxed(scalar_negated_tanru_unit_inner(tanru_unit_atom, tanru_unit, tense_modal));
        }
    }

    alias scalar_negated_tanru_unit_inner(tanru_unit_atom, tanru_unit, tense_modal) -> TanruUnitSyntax {
        context "scalar-negated tanru unit";
        choice((
            tagged_selbri_group_tanru_unit(tanru_unit, tense_modal),
            pro_bridi_tanru_unit(),
            tanru_unit_atom,
        ));
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

    alias jai_inner_tanru_unit(jai_inner_tanru_unit, sumti, selbri, text, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "modal conversion";
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
    }

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

    node empty_linked_sumti -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            default fa: Option<WithFreeModifiers<Token, FreeModifierSyntax>> = None;
            default sumti: Option<Box<SumtiSyntax>> = None;
        }
    }

    alias linked_sumti_tail(sumti) -> SumtiSyntax {
        context "linked arguments";
        choice((
            sumti,
            tagged_elided_sumti(),
        ));
    }

    node place_tagged_linked_sumti(sumti) -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field fa = selmaho(Fa).wf();
            field sumti = boxed(linked_sumti_tail(sumti));
        }
    }

    node tense_tagged_linked_sumti(sumti, tense_modal) -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field tense_modal = boxed(tense_modal);
            field sumti = boxed(linked_sumti_tail(sumti));
        }
    }

    node plain_linked_sumti(sumti) -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field sumti = boxed(sumti);
        }
    }

    alias linked_sumti(sumti, tense_modal) -> LinkedSumtiSyntax {
        context "linked arguments";
        choice((
            place_tagged_linked_sumti(sumti),
            tense_tagged_linked_sumti(sumti, tense_modal),
            plain_linked_sumti(sumti),
            empty_linked_sumti(),
        ));
    }

    node bei_link(sumti, tense_modal) -> AdditionalLinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field bei = cmavo(Bei).wf();
            field link = linked_sumti(sumti, tense_modal);
        }
    }

    node linkargs(sumti, tense_modal) -> LinkedSumtiListSyntax {
        context "linked arguments";
        fields {
            field be = cmavo(Be).wf();
            field first_link = linked_sumti(sumti, tense_modal);
            field bei_links = many(bei_link(sumti, tense_modal));
            field beho = opt(cmavo(Beho).wf());
        }
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

    node abstractor_connection -> AbstractorConnectionSyntax {
        context "abstractor connection";
        fields {
            field connective = standard_statement_connective;
            field nu = selmaho(Nu).wf();
            field nai = opt(cmavo(Nai).wf());
        }
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
            if let AtomRef::Token(token) = atom {
                self.first = Some(token);
            }
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
