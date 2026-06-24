//! Declarative generated syntax parser.

#![allow(dead_code)]

use chumsky::span::SimpleSpan;
use chumsky::{Parser, input::Input, primitive::end, recursive::Recursive};
use jbotci_morphology::{Cmavo, Selmaho};

use super::ast::*;
use super::generated_runtime;
use super::tense::{connective_tense_modal_from_leaves, tense_modal_as_composite};
use super::tokens::{
    cmavo, cmevla_word, leading_indicator, pa_word, relation_word, selmaho, spanned_tokens,
    syntax_error, syntax_trace_failure_summary,
};
use super::{
    BoxedParser, ParseExtra, ParsedPartialValidStatementAttempt, ParsedStatement,
    ParsedStatementAttempt, ParserInput, ParserState,
};
use crate::{ExperimentalConstruct, ParseOptions, SyntaxWordCategory, Token};

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DescriptionTailSyntax {
    tail_elements: Vec<DescriptionTailElementSyntax>,
    selbri: Option<Box<SelbriSyntax>>,
    relative_clauses: Vec<RelativeClauseSyntax>,
}

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundSumtiTailSyntax {
    connective: Box<ConnectiveSyntax>,
    tense_modal: Option<Box<TenseModalSyntax>>,
    bo: WithFreeModifiers<Token>,
    trailing_sumti: Box<SumtiSyntax>,
}

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupedSumtiTailSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<Box<TenseModalSyntax>>,
    ke: WithFreeModifiers<Token>,
    inner_sumti: Box<SumtiSyntax>,
    kehe: Option<WithFreeModifiers<Token>>,
}

#[bityzba::invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct VuhoSumtiAttachmentSyntax {
    vuho: WithFreeModifiers<Token>,
    relative_clauses: Vec<RelativeClauseSyntax>,
    sumti_connection: Option<Box<SumtiConnectionSyntax>>,
}

#[bityzba::invariant(i.is_cmavo(Cmavo::I))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LeadingIStatementSyntax {
    i: Token,
    connective: Option<Box<ConnectiveSyntax>>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[bityzba::invariant(!operands.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReversePolishPartsSyntax {
    operands: Vec<MeksoSyntax>,
    operators: Vec<MeksoOperatorSyntax>,
}

jbotci_syntax_macros::syntax_grammar! {
    env generated_runtime::SyntaxGrammarEnv;
    parsers;

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
        reverse_polish_parts: self::ReversePolishPartsSyntax;
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
        fields {
            require explicit_xauha_lohoi_lookahead().lookahead();
            field paragraphs = text_paragraph_with_additional_niho(paragraph, statement_or_fragment, free_modifier);
        }
        build |paragraphs| {
            bityzba::new!(TextSyntax {
                leading_nai: Vec::new(),
                leading_cmevla: Vec::new(),
                leading_indicators: Vec::new(),
                leading_free_modifiers: Vec::new(),
                leading_connective: None,
                paragraphs,
            })
        };
    }

    product regular_text(paragraph, statement_or_fragment, free_modifier, tense_modal) -> TextSyntax {
        context "text";
        fields {
            field leading_nai = many(cmavo(Nai));
            field leading_cmevla = many(text_leading_cmevla_word());
            field leading_indicators = many(leading_indicator());
            field leading_free_modifiers = many(free_modifier);
            field leading_connective = opt(text_leading_connective(tense_modal));
            field leading_i_statements = many(leading_i_statement(free_modifier, tense_modal));
            field paragraphs = text_paragraphs(paragraph, statement_or_fragment, free_modifier);
        }
        build |leading_nai, leading_cmevla, leading_indicators, leading_free_modifiers, leading_connective, leading_i_statements, paragraphs| {
            let text = bityzba::new!(TextSyntax {
                leading_nai,
                leading_cmevla,
                leading_indicators,
                leading_free_modifiers,
                leading_connective: leading_connective.map(Box::new),
                paragraphs,
            });
            leading_i_statements
                .into_iter()
                .rev()
                .fold(text, prepend_leading_i_statement)
        };
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

    product leading_i_statement(free_modifier, tense_modal) -> self::LeadingIStatementSyntax {
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
        fields {
            default i = None;
            default niho = Vec::new();
            default free_modifiers = Vec::new();
            field statements = paragraph_statement_sequence(statement_or_fragment, free_modifier);
        }
    }

    product paragraph_statement_sequence(statement_or_fragment, free_modifier) -> std::vec::Vec<ParagraphStatementSyntax> {
        context "paragraph";
        fields {
            field first_statement = initial_paragraph_statement(statement_or_fragment);
            field following_statements = many(following_paragraph_statement(statement_or_fragment, free_modifier));
            field trailing_ijek_statements = many(trailing_ijek_paragraph_statement());
        }
        build |first_statement, following_statements, trailing_ijek_statements| {
            std::iter::once(first_statement)
                .chain(following_statements)
                .chain(trailing_ijek_statements)
                .collect()
        };
    }

    node i_niho_paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        fields {
            field i = some(cmavo(I));
            field niho = many1(selmaho(Niho));
            field free_modifiers = many(free_modifier);
            field statements = opt_or_default(paragraph_statement_sequence(statement_or_fragment, free_modifier));
        }
    }

    node niho_paragraph(statement_or_fragment, free_modifier) -> ParagraphSyntax {
        context "paragraph";
        fields {
            default i = None;
            field niho = many1(selmaho(Niho));
            field free_modifiers = many(free_modifier);
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
        fields {
            default i = None;
            default connective = None;
            default free_modifiers = Vec::new();
            field statement = some(boxed(statement_or_fragment));
        }
    }

    node i_paragraph_statement(statement_or_fragment, free_modifier, tense_modal) -> ParagraphStatementSyntax {
        context "paragraph statement";
        fields {
            field i = some(cmavo(I));
            field connective = opt(boxed(i_paragraph_statement_connective(tense_modal)));
            field free_modifiers = many(free_modifier);
            field statement = opt(boxed(statement_or_fragment));
        }
    }

    node following_paragraph_statement(statement_or_fragment, free_modifier) -> ParagraphStatementSyntax {
        context "paragraph statement";
        fields {
            field i = some(cmavo(I));
            require statement_connective.not();
            default connective = None;
            field free_modifiers = many(free_modifier);
            field statement = opt(boxed(statement_or_fragment));
        }
    }

    node trailing_ijek_paragraph_statement -> ParagraphStatementSyntax {
        context "paragraph statement";
        fields {
            field i = cmavo(I);
            field connective = statement_connective;
        }
        build |i, connective| bityzba::new!(ParagraphStatementSyntax {
            i: None,
            connective: None,
            free_modifiers: Vec::new(),
            statement: Some(Box::new(bityzba::new!(StatementSyntax::Fragment(Box::new(
                bityzba::new!(FragmentSyntax::BridiConnective { i, connective })
            ))))),
        });
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
        fields {
            field first_na = selmaho(Na);
            field second_na = selmaho(Na);
            field additional_na = many(selmaho(Na));
        }
        build |first_na, second_na, additional_na| {
            let mut words = vec![first_na, second_na];
            words.extend(additional_na);
            bityzba::new!(StatementSyntax::Fragment(Box::new(
                bityzba::new!(FragmentSyntax::Other(WithFreeModifiers::new(words, Vec::new())))
            )))
        };
    }

    node single_na_fragment -> StatementSyntax {
        context "fragment";
        fields {
            field na = selmaho(Na).not_next_selmaho(Ku).wf();
        }
        build |na| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Other(WithFreeModifiers::new(vec![na.value], na.free_modifiers)))
        )));
    }

    node ek_fragment -> StatementSyntax {
        context "fragment";
        fields {
            field connective = ek_connective();
        }
        build |connective| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Ek(connective))
        )));
    }

    node gihek_fragment -> StatementSyntax {
        context "fragment";
        fields {
            field connective = gihek_connective();
        }
        build |connective| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::BridiTailConnective(connective))
        )));
    }

    node i_statement_connection(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> StatementSyntax {
        context "statement connection";
        fields {
            field leading_statement = boxed(statement_base(statement, bridi, term, sumti, subbridi, selbri, mekso, text, tense_modal, letter_tokens));
            field continuations = many1(choice((
                chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens),
                simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens),
            )));
        }
        build |leading_statement, continuations| build_connected_i_statement(*leading_statement, continuations);
    }

    product pending_i_connective -> (Token, ConnectiveSyntax) {
        context "statement connective";
        fields {
            field i = cmavo(I);
            field connective = statement_connective;
            require cmavo(I).lookahead();
        }
        build |i, connective| (i, connective);
    }

    product chained_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> (Token, ConnectiveSyntax, Box<StatementSyntax>) {
        context "statement connection";
        fields {
            field pending = many1(pending_i_connective);
            field i = cmavo(I);
            field connective = i_statement_connective(tense_modal);
            field trailing_statement = boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
        }
        build |pending, i, connective, trailing_statement| {
            build_chained_i_connective_statement_tail(pending, i, connective, trailing_statement)
        };
    }

    product simple_i_connective_statement_tail(statement, bridi, term, sumti, subbridi, selbri, mekso, tense_modal, text, letter_tokens) -> (Token, ConnectiveSyntax, Box<StatementSyntax>) {
        context "statement connection";
        fields {
            field i = cmavo(I);
            field connective = i_statement_connective(tense_modal);
            field trailing_statement = boxed(statement_after_i_connective(bridi, subbridi, tense_modal, text));
        }
        build |i, connective, trailing_statement| (i, connective, trailing_statement);
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
            field text = boxed(text);
            field tuhu = opt(cmavo(Tuhu).wf());
        }
    }

    node prenex_fragment(term) -> StatementSyntax {
        context "prenex";
        fields {
            field terms = many(term);
            field zohu = cmavo(Zohu).wf();
        }
        build |terms, zohu| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Prenex { terms, zohu })
        )));
    }

    node prenex_statement(statement, term) -> StatementSyntax {
        context "prenex";
        construct variant Prenex;
        fields {
            field prenex_terms = many(term);
            field zohu = cmavo(Zohu).wf();
            field inner_statement = boxed(statement);
        }
    }

    node bridi_statement(bridi, subbridi, tense_modal) -> StatementSyntax {
        context "statement";
        fields {
            field bridi = boxed(bridi);
            field continuations = many(bridi_statement_continuation(subbridi, tense_modal));
        }
        build |bridi, continuations| {
            continuations.into_iter().fold(
                bityzba::new!(StatementSyntax::Bridi(bridi)),
                |leading_statement, continuation| {
                    bityzba::new!(StatementSyntax::ExperimentalBridiContinuation {
                        leading_statement: Box::new(leading_statement),
                        continuation,
                    })
                },
            )
        };
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
        construct direct;
        fields {
            field connective = bridi_tail_connective;
            field tense_modal = opt(boxed(tense_modal));
            scratch bo = cmavo(Bo).wf();
            let marker = bityzba::new!(BridiStatementContinuationMarkerSyntax::BoGrouped(bo));
            field trailing_subbridi = boxed(subbridi);
        }
    }

    product ke_bridi_statement_continuation(subbridi, tense_modal) -> BridiStatementContinuationSyntax {
        context "bridi continuation";
        construct direct;
        fields {
            field connective = relation_afterthought_connective;
            field tense_modal = opt(boxed(tense_modal));
            scratch ke = cmavo(Ke).wf();
            field trailing_subbridi = boxed(subbridi);
            scratch kehe = opt(cmavo(Kehe).wf());
            let marker = bityzba::new!(BridiStatementContinuationMarkerSyntax::KeGrouped {
                ke,
                kehe,
            });
        }
    }

    node selbri_fragment(selbri) -> StatementSyntax {
        context "selbri";
        fields {
            field selbri = boxed(selbri);
        }
        build |selbri| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Selbri(selbri))
        )));
    }

    node terms_fragment(term) -> StatementSyntax {
        context "terms";
        fields {
            field first_term = term;
            field additional_terms = many(term);
            field vau = opt(cmavo(Vau).wf());
        }
        build |first_term, additional_terms, vau| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Terms {
                terms: std::iter::once(first_term).chain(additional_terms).collect(),
                vau,
            })
        )));
    }

    node mekso_fragment(mekso, letter_tokens) -> StatementSyntax {
        context "mex";
        fields {
            field quantifier = boxed(quantifier(mekso, letter_tokens));
        }
        build |quantifier| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::Mekso(Box::new(
                bityzba::new!(MeksoSyntax::NumberMekso(quantifier))
            )))
        )));
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
        fields {
            field relative_clauses = relative_clause_list(sumti, subbridi, tense_modal);
        }
        build |relative_clauses| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::RelativeClauses(relative_clauses))
        )));
    }

    node linked_sumti_continuation_fragment(sumti, tense_modal) -> StatementSyntax {
        context "linked arguments";
        fields {
            field bei_links = many1(bei_link(sumti, tense_modal));
        }
        build |bei_links| bityzba::new!(StatementSyntax::Fragment(Box::new(
            bityzba::new!(FragmentSyntax::LinkedSumtiContinuation(bei_links))
        )));
    }

    node linked_sumti_fragment(sumti, tense_modal) -> StatementSyntax {
        context "linked arguments";
        fields {
            field linkargs = linkargs(sumti, tense_modal);
        }
        build |linkargs| {
            let bityzba::data!(LinkedSumtiListSyntax {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) = linkargs.into_data();
            bityzba::new!(StatementSyntax::Fragment(Box::new(
                bityzba::new!(FragmentSyntax::LinkedSumti {
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                })
            )))
        };
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
            default free_modifiers = Vec::new();
        }
    }

    node bridi_with_post_cu_terms(term, bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            field leading_terms = many1(term);
            field cu = some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
            field bridi_tail = boxed(cu_terms_bridi_tail(term, bridi_tail));
            default free_modifiers = Vec::new();
        }
    }

    node bare_cu_bridi(bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms = Vec::new();
            field cu = some(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bridi_tail);
            default free_modifiers = Vec::new();
        }
    }

    node bare_cu_terms_bridi(term, bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms = Vec::new();
            field cu = some(arc(cmavo(Cu).warn(ExperimentalCuTermsSelbri).wf()));
            field bridi_tail = boxed(cu_terms_bridi_tail(term, bridi_tail));
            default free_modifiers = Vec::new();
        }
    }

    node relation_only_bridi(bridi_tail) -> BridiSyntax {
        context "bridi";
        fields {
            default leading_terms = Vec::new();
            default cu = None;
            field bridi_tail = boxed(bridi_tail);
            default free_modifiers = Vec::new();
        }
    }

    node cu_terms_bridi_tail(term, bridi_tail) -> BridiTailSyntax {
        context "bridi tail";
        fields {
            field terms = many1(term);
            field bridi_tail = boxed(bridi_tail);
        }
        build |terms, bridi_tail| BridiTailSyntax {
            first: Box::new(AfterthoughtBridiTailSyntax {
                first: Box::new(BoGroupedBridiTailSyntax {
                    first: Box::new(bityzba::new!(SimpleBridiTailSyntax::TermPrefixedBridiTail {
                        terms,
                        bridi_tail,
                    })),
                    bo_continuation: None,
                }),
                continuations: Vec::new(),
            }),
            ke_continuation: None,
        };
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
            default terms = Vec::new();
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers = Vec::new();
        }
    }

    node selbri_simple_bridi_tail(selbri, term) -> SimpleBridiTailSyntax {
        context "bridi tail";
        construct variant SelbriBridiTail;
        fields {
            field selbri = boxed(selbri);
            field terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers = Vec::new();
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
            default free_modifiers = Vec::new();
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
            default tail_terms = Vec::new();
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers = Vec::new();
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
            default free_modifiers = Vec::new();
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
            default free_modifiers = Vec::new();
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
            default tail_terms = Vec::new();
            default vau = None;
            default free_modifiers = Vec::new();
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
            default free_modifiers = Vec::new();
        }
    }

    node bridi_tail_continuation_without_tail_terms(bo_grouped_bridi_tail_without_tail_terms, term, tense_modal) -> BridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            require (bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke)))).not();
            field connective = bridi_tail_connective;
            default tense_modal = None;
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail_without_tail_terms);
            default tail_terms = Vec::new();
            default vau = None;
            default free_modifiers = Vec::new();
        }
    }

    node bridi_tail_continuation(bo_grouped_bridi_tail, term, tense_modal) -> BridiTailConnectionSyntax {
        context "bridi tail connective";
        fields {
            require (bridi_tail_connective, opt(boxed(tense_modal)), choice((cmavo(Bo), cmavo(Ke)))).not();
            field connective = bridi_tail_connective;
            default tense_modal = None;
            field cu = opt(arc(cmavo(Cu).wf()));
            field bridi_tail = boxed(bo_grouped_bridi_tail);
            field tail_terms = many(term);
            field vau = opt(arc(cmavo(Vau).wf()));
            default free_modifiers = Vec::new();
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
        build |leading_term, continuations| {
            continuations.into_iter().fold(*leading_term, |leading_term, ((pehe, connective), trailing_term)| {
                bityzba::new!(TermSyntax::TermsetConnection {
                    leading_terms: vec![leading_term],
                    pehe,
                    connective,
                    trailing_terms: vec![*trailing_term],
                })
            })
        };
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
            field post_bo_argument_gate = term_hierarchy_post_bo_argument_gate(sumti);
            field trailing_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            field post_bo_trailing_argument_gate = term_hierarchy_post_bo_argument_gate(sumti);
        }
        build |leading_term, connective, bo, post_bo_argument_gate, trailing_term, post_bo_trailing_argument_gate| {
            let _ = post_bo_argument_gate;
            let _ = post_bo_trailing_argument_gate;
            bityzba::new!(TermSyntax::BoundTermConnection {
                leading_terms: vec![*leading_term],
                bo_connective: Some(connective),
                tense_modal: None,
                bo,
                trailing_term,
            })
        };
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
        build |leading_term, continuations| {
            continuations.into_iter().fold(*leading_term, |leading_term, (connective, trailing_term)| {
                bityzba::new!(TermSyntax::TermConnection {
                    leading_terms: vec![leading_term],
                    connective,
                    trailing_terms: vec![*trailing_term],
                })
            })
        };
    }

    node termset_group(sumti, tense_modal, subbridi, selbri, term) -> TermSyntax {
        context "termset";
        fields {
            field leading_term = boxed(simple_term(sumti, tense_modal, subbridi, selbri, term));
            field continuations = many1((cmavo(Cehe).wf(), boxed(simple_term(sumti, tense_modal, subbridi, selbri, term))));
        }
        build |leading_term, continuations| {
            continuations.into_iter().fold(*leading_term, |leading_term, (cehe, trailing_term)| {
                bityzba::new!(TermSyntax::TermsetGroup {
                    leading_terms: vec![leading_term],
                    cehe,
                    trailing_terms: vec![*trailing_term],
                })
            })
        };
    }

    node forethought_termset(term, tense_modal) -> TermSyntax {
        context "termset";
        construct variant ForethoughtTermsetConnection;
        fields {
            field m_nuhi = opt(cmavo(Nuhi).wf());
            field gek = modal_forethought_connective(tense_modal);
            scratch term_boxes = many1(boxed(term));
            let terms = unbox_terms(term_boxes);
            field nuhu = opt(cmavo(Nuhu).wf());
            field gik = gik_connective;
            scratch gik_term_boxes = many1(boxed(term));
            let gik_terms = unbox_terms(gik_term_boxes);
            field gihi = opt(feature(ZantufaConnectives, selmaho(Gihi).warn(ExperimentalZantufaForethoughtGihi)));
            field gik_nuhu = opt(cmavo(Nuhu).wf());
        }
    }

    node nuhi_termset(term) -> TermSyntax {
        context "termset";
        construct variant Termset;
        fields {
            field nuhi = cmavo(Nuhi).wf();
            scratch term_boxes = many1(boxed(term));
            let termset = unbox_terms(term_boxes);
            field nuhu = opt(cmavo(Nuhu).wf());
        }
    }

    node ke_termset(term) -> TermSyntax {
        context "termset";
        construct variant Termset;
        fields {
            scratch ke = cmavo(Ke).warn(ExperimentalKeTermset).wf();
            let nuhi = ke;
            scratch term_boxes = many1(boxed(term));
            let termset = unbox_terms(term_boxes);
            scratch kehe = opt(cmavo(Kehe).wf());
            let nuhu = kehe;
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
            default tail_elements = Vec::new();
            field selbri = some(boxed(selbri));
            default relative_clauses = Vec::new();
            field brigahi_ku = cmavo(Ku).warn(ExperimentalZantufaPoihaBrigahi).wf();
        }
    }

    node noiha_relative_adverbial_term(selbri) -> TermSyntax {
        context "NOIhA adverbial";
        construct variant RelativeAdverbialTerm;
        fields {
            field noiha = selmaho(Noiha).wf();
            default tail_elements = Vec::new();
            field selbri = some(boxed(selbri));
            default relative_clauses = Vec::new();
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
            default ku = None;
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
        build |tense_modal| {
            bityzba::new!(TermSyntax::TaggedSumti {
                tense_modal: Some(tense_modal),
                sumti: Box::new(bityzba::new!(SumtiSyntax::ElidedSumti {
                    tag: None,
                    maybe_ku: None,
                    free_modifiers: Vec::new(),
                })),
            })
        };
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
            field next = selmaho(Nahe).lookahead();
        }
        build |pu, nai, next| {
            let _ = next;
            let mut parts = vec![pu];
            parts.extend(nai);
            composite_from_wf_tokens(parts)
        };
    }

    node pu_distance_before_tag_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field pu = selmaho(Pu).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = selmaho(Zi).wf();
            field next = selmaho(Zi).lookahead();
        }
        build |pu, nai, distance, next| {
            let _ = next;
            let mut parts = vec![pu];
            parts.extend(nai);
            parts.push(distance);
            composite_from_wf_tokens(parts)
        };
    }

    node zi_before_zi_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field zi = selmaho(Zi).wf();
            field next = selmaho(Zi).lookahead();
        }
        build |zi, next| {
            let _ = next;
            composite_from_wf_tokens(vec![zi])
        };
    }

    node va_before_va_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field va = selmaho(Va).wf();
            field next = selmaho(Va).lookahead();
        }
        build |va, next| {
            let _ = next;
            composite_from_wf_tokens(vec![va])
        };
    }

    node mohi_before_mohi_leading_term_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field mohi = selmaho(Mohi).wf();
            field direction = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Va).wf());
            field next = selmaho(Mohi).lookahead();
        }
        build |mohi, direction, nai, distance, next| {
            let _ = next;
            let mut parts = vec![mohi, direction];
            parts.extend(nai);
            parts.extend(distance);
            composite_from_wf_tokens(parts)
        };
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
            default tag = None;
            field maybe_ku = opt(cmavo(Ku).wf());
            default free_modifiers = Vec::new();
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
        build |base_sumti, vuho_attachment| apply_vuho_sumti_attachment(base_sumti, vuho_attachment);
    }

    node sumti_grouped(sumti, sumti_afterthought, tense_modal) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_afterthought);
            field grouped_tail = opt(grouped_sumti_tail(sumti, tense_modal));
        }
        build |leading_sumti, grouped_tail| apply_grouped_sumti_tail(leading_sumti, grouped_tail);
    }

    node sumti_afterthought(sumti_bound) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_bound);
            field continuations = many(sumti_afterthought_tail(sumti_bound));
        }
        build |leading_sumti, continuations| apply_afterthought_sumti_tails(leading_sumti, continuations);
    }

    node sumti_bound(sumti_bound, sumti_forethought, tense_modal) -> SumtiSyntax {
        context "sumti connection";
        fields {
            field leading_sumti = boxed(sumti_forethought);
            field bound_tail = opt(bound_sumti_tail(sumti_bound, tense_modal));
        }
        build |leading_sumti, bound_tail| apply_bound_sumti_tail(leading_sumti, bound_tail);
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

    product bound_sumti_tail(sumti_bound, tense_modal) -> self::BoundSumtiTailSyntax {
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

    product grouped_sumti_tail(sumti, tense_modal) -> self::GroupedSumtiTailSyntax {
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

    alias vuho_sumti_attachment_tail(sumti, subbridi, tense_modal) -> self::VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        choice((
            vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal),
            vuho_connected_sumti_attachment_tail(sumti),
        ));
    }

    product vuho_relative_sumti_attachment_tail(sumti, subbridi, tense_modal) -> self::VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        construct direct;
        fields {
            field vuho = cmavo(Vuho).wf();
            field relative_clauses = relative_clause_list(sumti, subbridi, tense_modal);
            field sumti_connection = opt(boxed(sumti_connection_tail(sumti)));
        }
    }

    product vuho_connected_sumti_attachment_tail(sumti) -> self::VuhoSumtiAttachmentSyntax {
        context "sumti relative phrase";
        construct direct;
        fields {
            field vuho = cmavo(Vuho).wf();
            default relative_clauses = Vec::new();
            field sumti_connection = some(boxed(sumti_connection_tail(sumti)));
        }
    }

    node simple_sumti(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti";
        fields {
            field base_sumti = boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |base_sumti, relative_clauses| {
            let relative_clauses = optional_relative_clause_list(relative_clauses);
            if relative_clauses.is_empty() {
                *base_sumti
            } else {
                bityzba::new!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti,
                    vuho: None,
                    relative_clauses,
                })
            }
        };
    }

    alias sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti";
        choice((
            sumti_base,
            quantified_sumti(sumti_base, mekso, letter_tokens),
        ));
    }

    node sumti_base(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_string, letter_tokens, free_modifier) -> SumtiSyntax {
        context "sumti";
        fields {
            field sumti = choice((
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
        build |sumti| sumti;
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
        build |leading_sumti, connective, tense_modal, ke, inner_sumti, kehe| {
            let connective = match tense_modal {
                Some(tense_modal) => append_tense_modal_words_to_connective(connective, *tense_modal),
                None => connective,
            };
            bityzba::new!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                connective,
                trailing_sumti: Box::new(bityzba::new!(SumtiSyntax::GroupedSumti {
                    ke,
                    inner_sumti,
                    kehe,
                })),
            })
        };
    }

    node sumti_with_relative_clauses(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "sumti relative phrase";
        construct variant SumtiWithRelativeClauses;
        fields {
            field base_sumti = boxed(sumti_atom(sumti, sumti_base, subbridi, tense_modal, mekso, letter_tokens));
            default vuho = None;
            scratch first_relative_clause = relative_clause_atom(sumti, subbridi, tense_modal);
            scratch additional_relative_clauses = many(relative_clause_tail(sumti, subbridi, tense_modal));
            let relative_clauses = std::iter::once(first_relative_clause)
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
        build |base_sumti, vuho, relative_clauses, sumti_connection| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            if !relative_clauses.is_empty() && sumti_connection.is_none() {
                bityzba::new!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti,
                    vuho: Some(vuho),
                    relative_clauses,
                })
            } else {
                bityzba::new!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                    base_sumti,
                    vuho_marker: vuho,
                    relative_clauses,
                    sumti_connection: sumti_connection.map(Box::new),
                })
            }
        };
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
            let number = WithFreeModifiers::new(
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
        build |leading_operator, continuations| {
            continuations.into_iter().fold(*leading_operator, |left_operator, (connective, right_operator)| {
                bityzba::new!(MeksoOperatorSyntax::OperatorConnection {
                    left_operator: Box::new(left_operator),
                    connective,
                    right_operator,
                })
            })
        };
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
        build |guhek, left_operator, gik, right_operator| {
            bityzba::new!(MeksoOperatorSyntax::OperatorConnection {
                left_operator,
                connective: append_connective_words(guhek, gik.words()),
                right_operator,
            })
        };
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
        build |leading_expression, continuations| {
            continuations.into_iter().fold(*leading_expression, |left_expression, (connective, right_expression)| {
                bityzba::new!(MeksoSyntax::MeksoConnection {
                    left_expression: Box::new(left_expression),
                    connective,
                    right_expression,
                })
            })
        };
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
        construct variant MeksoConnection;
        fields {
            field left_expression = boxed(simple_mekso_operand(mekso, mekso_operand, sumti, selbri, tense_modal, letter_string, letter_tokens, free_modifier));
            scratch operand_connective = operand_connective;
            scratch tense_modal = opt(boxed(tense_modal));
            scratch bo = cmavo(Bo).wf();
            let connective = append_optional_tense_modal_and_bo_to_connective(operand_connective, tense_modal, bo);
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
            let markers = WithFreeModifiers::new(vec![nahe, bo], Vec::new());
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
            let expressions = MeksoVec::try_from_vec(expression_items)
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
        build |words, boi, free_modifiers| {
            let (number_free_modifiers, boi) = attach_free_modifiers_to_optional_terminator(boi, free_modifiers);
            bityzba::new!(MeksoSyntax::NumberMekso(Box::new(
                bityzba::new!(QuantifierSyntax::NumberQuantifier {
                    number: WithFreeModifiers::new(
                        WordRun::try_from_vec(words).expect("number-or-letter words guarantee non-empty word run"),
                        number_free_modifiers,
                    ),
                    boi,
                })
            )))
        };
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

    product tei_letter_tokens(letter_string) -> std::vec::Vec<Token> {
        context "lerfu word";
        fields {
            field tei = cmavo(Tei);
            field inner = letter_string;
            field foi = cmavo(Foi);
        }
        build |tei, inner, foi| {
            let mut words = vec![tei];
            words.extend(inner);
            words.push(foi);
            words
        };
    }

    node lerfu_string_mekso(letter_string, free_modifier) -> MeksoSyntax {
        context "lerfu string";
        fields {
            field letters = letter_string;
            field boi = opt(cmavo(Boi));
            field free_modifiers = many(free_modifier);
        }
        build |letters, boi, free_modifiers| {
            let (letter_free_modifiers, boi) = attach_free_modifiers_to_optional_terminator(boi, free_modifiers);
            let letter = WithFreeModifiers::new(
                WordRun::try_from_vec(letters).expect("first lerfu word guarantees non-empty lerfu string"),
                letter_free_modifiers,
            );
            bityzba::new!(MeksoSyntax::LerfuStringMekso { letter, boi })
        };
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
        build |left_expression, tail| {
            if let Some(((bihe, operator), right_expression)) = tail {
                bityzba::new!(MeksoSyntax::PrecedenceInfix {
                    left_expression,
                    bihe,
                    operator,
                    right_expression,
                })
            } else {
                *left_expression
            }
        };
    }

    node infix_mekso(mekso_base, mekso_precedence, mekso_operator) -> MeksoSyntax {
        context "mex";
        fields {
            field first_expression = boxed(mekso_precedence(mekso_base, mekso_precedence, mekso_operator));
            field continuations = many((boxed(mekso_operator), boxed(mekso_precedence)));
        }
        build |first_expression, continuations| {
            continuations.into_iter().fold(*first_expression, |left_expression, (operator, right_expression)| {
                bityzba::new!(MeksoSyntax::Infix {
                    left_expression: Box::new(left_expression),
                    operator,
                    right_expression,
                })
            })
        };
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

    product reverse_polish_parts(reverse_polish_parts, mekso_operand, mekso_operator) -> self::ReversePolishPartsSyntax {
        context "reverse Polish mex";
        fields {
            field first_operand = mekso_operand;
            field tails = many((reverse_polish_parts, mekso_operator));
        }
        build |first_operand, tails| {
            let mut operands = vec![first_operand];
            let mut operators = Vec::new();
            for (tail_parts, operator) in tails {
                let mut tail_data = tail_parts.into_data();
                operands.append(&mut tail_data.operands);
                operators.append(&mut tail_data.operators);
                operators.push(operator);
            }
            bityzba::new!(ReversePolishPartsSyntax { operands, operators })
        };
    }

    node reverse_polish_mekso(reverse_polish_parts) -> MeksoSyntax {
        context "reverse Polish mex";
        fields {
            field fuha = cmavo(Fuha).wf();
            field parts = reverse_polish_parts;
        }
        build |fuha, parts| {
            let bityzba::data!(ReversePolishPartsSyntax { operands, operators }) = parts.into_data();
            bityzba::new!(MeksoSyntax::ReversePolish {
                fuha,
                operands,
                operators,
            })
        };
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
        build |words, boi, free_modifiers| {
            let (letter_free_modifiers, boi) = attach_free_modifiers_to_optional_terminator(boi, free_modifiers);
            let letter = WithFreeModifiers::new(
                WordRun::try_from_vec(words).expect("first letter guarantees non-empty lerfu words"),
                letter_free_modifiers,
            );
            bityzba::new!(SumtiSyntax::LerfuStringSumti { letter, boi })
        };
    }

    node lahe_sumti(sumti, subbridi, tense_modal) -> SumtiSyntax {
        context "converted sumti";
        construct variant ReferentSumti;
        fields {
            field lahe = selmaho(Lahe).wf();
            scratch relative_clause_parts = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            let relative_clauses = optional_relative_clause_list(relative_clause_parts);
            field inner_sumti = boxed(sumti);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node lahe_term_wrapper(term) -> SumtiSyntax {
        context "converted term";
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind = SumtiWrapperKindSyntax::Referent;
            field wrapper = selmaho(Lahe).wf();
            default wrapper_bo = None;
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper_with_bo(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind = SumtiWrapperKindSyntax::ScalarNegationWithBo;
            scratch raw_wrapper = selmaho(Nahe);
            let wrapper = WithFreeModifiers::new(raw_wrapper, Vec::new());
            field wrapper_bo = some(cmavo(Bo).wf());
            field inner_term = boxed(term);
            field luhu = opt(cmavo(Luhu).wf());
        }
    }

    node scalar_negated_term_wrapper(term) -> SumtiSyntax {
        context "scalar-negated term";
        construct variant QualifiedTerm;
        fields {
            let term_wrapper_kind = SumtiWrapperKindSyntax::ScalarNegation;
            field wrapper = selmaho(Nahe).wf();
            default wrapper_bo = None;
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
        build |la, names| {
            let names = WithFreeModifiers::new(
                WordRun::try_from_vec(names.value).expect("many1 guarantees non-empty name words"),
                names.free_modifiers,
            );
            bityzba::new!(SumtiSyntax::NameDescription { la, names })
        };
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
        build |description| bityzba::new!(DescriptionHeadSyntax { description });
    }

    node description_head_connective -> ConnectiveSyntax {
        context "descriptor connective";
        fields {
            field connective = jek_connective;
        }
        build |connective| {
            let ConnectiveSyntaxParts {
                kind: _,
                se,
                nahe,
                na,
                cmavo,
                nai,
            } = connective.into_parts();
            ConnectiveSyntax::new(ConnectiveKind::Afterthought, se, nahe, na, cmavo, nai)
        };
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
        build |leading_description_head, connective, trailing_description_head, tail, ku| {
            let DescriptionTailSyntax {
                tail_elements,
                selbri,
                relative_clauses,
            } = tail;
            bityzba::new!(SumtiSyntax::DescriptionConnection(Box::new(
                bityzba::new!(DescriptionConnectionSyntax {
                    leading_description_head,
                    connective,
                    trailing_description_head,
                    tail_elements,
                    selbri,
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node descriptor_with_gadri_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = description_head();
            field tail = description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens);
            field ku = opt(cmavo(Ku).wf());
        }
        build |description, tail, ku| {
            let bityzba::data!(DescriptionHeadSyntax { description }) = description.into_data();
            let DescriptionTailSyntax {
                tail_elements,
                selbri,
                relative_clauses,
            } = tail;
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements,
                    selbri,
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node descriptor_with_outer_quantifier_sumti(sumti, sumti_base, term, subbridi, selbri, text, mekso, tense_modal, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field outer_quantifier = quantifier(mekso, letter_tokens);
            field description = description_head();
            field tail = description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens);
            field ku = opt(cmavo(Ku).wf());
        }
        build |outer_quantifier, description, tail, ku| {
            let bityzba::data!(DescriptionHeadSyntax { description }) = description.into_data();
            let DescriptionTailSyntax {
                tail_elements,
                selbri,
                relative_clauses,
            } = tail;
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: Some(Box::new(outer_quantifier)),
                    description: Some(description),
                    tail_elements,
                    selbri,
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node descriptor_without_gadri_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            require selmaho(Roi).not();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |quantifier, selbri, relative_clauses| {
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: None,
                    tail_elements: vec![bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                        quantifier
                    ))],
                    selbri: Some(selbri),
                    relative_clauses: optional_relative_clause_list(relative_clauses),
                    ku: None,
                })
            )))
        };
    }

    product description_tail(sumti, sumti_base, subbridi, selbri, tense_modal, mekso, letter_tokens) -> self::DescriptionTailSyntax {
        context "description tail";
        fields {
            field leading_tail_elements = leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal);
            field tail = choice((
                quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens),
                quantifier_sumti_description_tail(sumti, mekso, letter_tokens),
                relation_description_tail(sumti, subbridi, selbri, tense_modal),
            ));
        }
        build |leading_tail_elements, tail| {
            let mut leading_tail_elements = leading_tail_elements;
            let DescriptionTailSyntax {
                tail_elements,
                selbri,
                relative_clauses,
            } = tail;
            leading_tail_elements.extend(tail_elements);
            DescriptionTailSyntax {
                tail_elements: leading_tail_elements,
                selbri,
                relative_clauses,
            }
        };
    }

    product leading_description_tail_elements(sumti, sumti_base, subbridi, selbri, tense_modal) -> std::vec::Vec<DescriptionTailElementSyntax> {
        context "description tail";
        fields {
            field tail_sumti = opt(description_tail_sumti(sumti_base));
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |tail_sumti, relative_clauses| {
            let mut tail_elements = tail_sumti.unwrap_or_default();
            let relative_clauses = optional_relative_clause_list(relative_clauses);
            if !relative_clauses.is_empty() {
                tail_elements.push(bityzba::new!(
                    DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
                ));
            }
            tail_elements
        };
    }

    product description_tail_sumti(sumti_base) -> std::vec::Vec<DescriptionTailElementSyntax> {
        context "description tail";
        fields {
            require pa_word().not();
            field sumti = boxed(sumti_base);
        }
        build |sumti| description_tail_sumti_elements(sumti);
    }

    product relation_description_tail(sumti, subbridi, selbri, tense_modal) -> self::DescriptionTailSyntax {
        context "description tail";
        fields {
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |selbri, relative_clauses| DescriptionTailSyntax {
            tail_elements: Vec::new(),
            selbri: Some(selbri),
            relative_clauses: optional_relative_clause_list(relative_clauses),
        };
    }

    product quantifier_relation_description_tail(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> self::DescriptionTailSyntax {
        context "description tail";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            require selmaho(Roi).not();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |quantifier, selbri, relative_clauses| {
            DescriptionTailSyntax {
                tail_elements: vec![bityzba::new!(
                    DescriptionTailElementSyntax::DescriptionTailQuantifier(quantifier)
                )],
                selbri: Some(selbri),
                relative_clauses: optional_relative_clause_list(relative_clauses),
            }
        };
    }

    product quantifier_sumti_description_tail(sumti, mekso, letter_tokens) -> self::DescriptionTailSyntax {
        context "description tail";
        fields {
            field quantifier = quantifier(mekso, letter_tokens);
            field sumti = boxed(sumti);
        }
        build |quantifier, sumti| DescriptionTailSyntax {
            tail_elements: vec![
                bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(quantifier)),
                bityzba::new!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)),
            ],
            selbri: None,
            relative_clauses: Vec::new(),
        };
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
        build |description, first_relative_clause, additional_relative_clauses, tail_quantifier, selbri, relative_clauses, ku| {
            let mut tail_elements = vec![bityzba::new!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                )
            )];
            tail_elements.extend(tail_quantifier.into_iter().map(|tail_quantifier| {
                bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                    tail_quantifier
                ))
            }));
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements,
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
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
        build |outer_quantifier, description, first_relative_clause, additional_relative_clauses, tail_quantifier, selbri, relative_clauses, ku| {
            let mut tail_elements = vec![bityzba::new!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                )
            )];
            tail_elements.extend(tail_quantifier.into_iter().map(|tail_quantifier| {
                bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                    tail_quantifier
                ))
            }));
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: Some(Box::new(outer_quantifier)),
                    description: Some(description),
                    tail_elements,
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
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
        build |description, tail_sumti, selbri, relative_clauses, ku| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements: description_tail_sumti_elements(tail_sumti),
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node sumti_tail_description_sumti(sumti, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field tail_sumti = boxed(sumti);
            field ku = opt(cmavo(Ku).wf());
        }
        build |description, tail_quantifier, tail_sumti, ku| {
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements: vec![
                        bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                            tail_quantifier
                        )),
                        bityzba::new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                            tail_sumti
                        )),
                    ],
                    selbri: None,
                    relative_clauses: Vec::new(),
                    ku,
                })
            )))
        };
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
        build |description, tail_quantifier, selbri, relative_clauses, ku| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements: vec![bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                        tail_quantifier
                    ))],
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node gadri_elided_description_sumti(sumti, subbridi, selbri, tense_modal, mekso, letter_tokens) -> SumtiSyntax {
        context "description";
        fields {
            field tail_quantifier = quantifier(mekso, letter_tokens);
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
        build |tail_quantifier, selbri, relative_clauses, ku| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: None,
                    tail_elements: vec![bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                        tail_quantifier
                    ))],
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
    }

    node simple_description_sumti(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "description";
        fields {
            field description = choice((selmaho(Le), selmaho(La))).wf();
            field selbri = boxed(selbri);
            field relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field ku = opt(cmavo(Ku).wf());
        }
        build |description, selbri, relative_clauses, ku| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: None,
                    description: Some(description),
                    tail_elements: Vec::new(),
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
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
        build |outer_quantifier, description, tail_quantifier, tail_sumti, ku| {
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: Some(Box::new(outer_quantifier)),
                    description: Some(description),
                    tail_elements: vec![
                        bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                            tail_quantifier
                        )),
                        bityzba::new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                            tail_sumti
                        )),
                    ],
                    selbri: None,
                    relative_clauses: Vec::new(),
                    ku,
                })
            )))
        };
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
        build |outer_quantifier, description, tail_quantifier, selbri, relative_clauses, ku| {
            let relative_clauses: Vec<RelativeClauseSyntax> = relative_clauses
                .map(|(first_relative_clause, additional_relative_clauses)| {
                    std::iter::once(first_relative_clause)
                        .chain(additional_relative_clauses)
                        .collect()
                })
                .unwrap_or_default();
            let tail_elements = tail_quantifier
                .into_iter()
                .map(|tail_quantifier| {
                    bityzba::new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                        tail_quantifier
                    ))
                })
                .collect();
            bityzba::new!(SumtiSyntax::Description(Box::new(
                bityzba::new!(DescriptionSyntax {
                    outer_quantifier: Some(Box::new(outer_quantifier)),
                    description: Some(description),
                    tail_elements,
                    selbri: Some(selbri),
                    relative_clauses,
                    ku,
                })
            )))
        };
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
        build |quote| {
            let variant = match quote.value.core_word().as_data() {
                bityzba::data!(jbotci_morphology::WordLike::QuotedWord { .. }) => 0,
                bityzba::data!(jbotci_morphology::WordLike::DelimitedWordQuote { .. }) => 1,
                bityzba::data!(jbotci_morphology::WordLike::DelimitedNonLojbanQuote { .. }) => 2,
                bityzba::data!(jbotci_morphology::WordLike::QuotedWords { .. }) => 3,
                _ => unreachable!("quote word category guarantees a compound quote token"),
            };
            match variant {
                0 => bityzba::new!(QuoteSyntax::WordQuote(quote)),
                1 => bityzba::new!(QuoteSyntax::DelimitedWordQuote(quote)),
                2 => bityzba::new!(QuoteSyntax::DelimitedNonLojbanQuote(quote)),
                3 => bityzba::new!(QuoteSyntax::WordsQuote(quote)),
                _ => unreachable!("quote variant is exhaustive"),
            }
        };
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
        build |leading_relative_clauses, selbri, trailing_relative_clauses| bityzba::new!(SumtiSyntax::SelbriVocative {
            leading_relative_clauses: optional_relative_clause_list(leading_relative_clauses),
            selbri,
            trailing_relative_clauses: optional_relative_clause_list(trailing_relative_clauses),
        });
    }

    node cmevla_vocative_sumti(sumti, subbridi, tense_modal) -> SumtiSyntax {
        context "vocative phrase";
        fields {
            field leading_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
            field names = many1(cmevla_word()).wf();
            field trailing_relative_clauses = opt((relative_clause_atom(sumti, subbridi, tense_modal), many(relative_clause_tail(sumti, subbridi, tense_modal))));
        }
        build |leading_relative_clauses, names, trailing_relative_clauses| {
            let names = WithFreeModifiers::new(
                WordRun::try_from_vec(names.value).expect("many1 guarantees non-empty name words"),
                names.free_modifiers,
            );
            let sumti = bityzba::new!(SumtiSyntax::NameWords(names));
            let relative_clauses: Vec<RelativeClauseSyntax> = optional_relative_clause_list(leading_relative_clauses)
                .into_iter()
                .chain(optional_relative_clause_list(trailing_relative_clauses))
                .collect();
            if relative_clauses.is_empty() {
                sumti
            } else {
                bityzba::new!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti: Box::new(sumti),
                    vuho: None,
                    relative_clauses,
                })
            }
        };
    }

    alias vocative_argument(sumti, subbridi, selbri, tense_modal) -> SumtiSyntax {
        context "vocative phrase";
        choice((
            selbri_vocative_sumti(sumti, subbridi, selbri, tense_modal),
            cmevla_vocative_sumti(sumti, subbridi, tense_modal),
            sumti,
        ));
    }

    node coi_vocative_marker_words -> generated_runtime::VocativeMarkerWordsSyntax {
        context "vocative marker";
        fields {
            field first_coi = selmaho(Coi);
            field first_nai = opt(cmavo(Nai));
            field additional_coi = many((selmaho(Coi), opt(cmavo(Nai))));
            field doi = opt(cmavo(Doi));
        }
        build |first_coi, first_nai, additional_coi, doi| {
            let mut words = vec![first_coi];
            words.extend(first_nai);
            for (coi, nai) in additional_coi {
                words.push(coi);
                words.extend(nai);
            }
            words.extend(doi);
            bityzba::new!(generated_runtime::VocativeMarkerWordsSyntax { words })
        };
    }

    node doi_vocative_marker_words -> generated_runtime::VocativeMarkerWordsSyntax {
        context "vocative marker";
        fields {
            field doi = cmavo(Doi);
        }
        build |doi| {
            bityzba::new!(generated_runtime::VocativeMarkerWordsSyntax {
                words: vec![doi],
            })
        };
    }

    node vocative_free_modifier(sumti, subbridi, selbri, tense_modal) -> FreeModifierSyntax {
        context "vocative phrase";
        fields {
            field vocative_markers = choice((
                coi_vocative_marker_words(),
                doi_vocative_marker_words(),
            )).wf();
            field sumti = opt(boxed(vocative_argument(sumti, subbridi, selbri, tense_modal)));
            field dohu = opt(cmavo(Dohu).prohibited_wf());
        }
        build |vocative_markers, sumti, dohu| {
            let marker_data = vocative_markers.value.into_data();
            let vocative_markers = WithFreeModifiers::new(
                marker_data.words,
                vocative_markers.free_modifiers,
            );
            bityzba::new!(FreeModifierSyntax::Vocative {
                vocative_markers,
                sumti,
                dohu,
            })
        };
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
        fields {
            field number = number_or_letter_words(letter_tokens, letter_string);
            field mai = selmaho(Mai).wf();
        }
        build |number, mai| bityzba::new!(FreeModifierSyntax::UtteranceOrdinal {
            number: WordRun::try_from_vec(number).expect("number-or-letter words guarantee non-empty ordinal"),
            mai,
        });
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
            default lohai = None;
            default old_words = Vec::new();
            field sahai = some(cmavo(Sahai));
            field new_words = raw_words_until(Lehai);
            field lehai = cmavo(Lehai).wf();
        }
    }

    node close_only_text_replacement_free_modifier -> FreeModifierSyntax {
        context "replacement free modifier";
        construct variant TextReplacement;
        fields {
            default lohai = None;
            default old_words = Vec::new();
            default sahai = None;
            default new_words = Vec::new();
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
        fields {
            field association_marker = selmaho(Goi).wf();
            field sumti = boxed(choice((
                tense_tagged_relative_sumti(tense_modal, sumti),
                na_ku_relative_sumti(),
                sumti,
            )));
            field gehu = opt(cmavo(Gehu).wf());
        }
        build |association_marker, sumti, gehu| {
            bityzba::new!(RelativeClauseSyntax::SumtiAssociationPhrase(Box::new(
                bityzba::new!(SumtiAssociationPhraseSyntax {
                    association_marker,
                    sumti,
                    gehu,
                })
            )))
        };
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
        build |tense_modal, sumti| {
            let tag = bityzba::new!(SumtiTagSyntax::TenseModal(tense_modal));
            match (*sumti).into_data() {
                bityzba::data!(SumtiSyntax::ElidedSumti { maybe_ku, free_modifiers, .. }) => {
                    bityzba::new!(SumtiSyntax::ElidedSumti {
                        tag: Some(Box::new(tag)),
                        maybe_ku,
                        free_modifiers,
                    })
                }
                data => bityzba::new!(SumtiSyntax::TaggedSumti {
                    tag,
                    inner_sumti: Box::new(SumtiSyntax::from_data(data)),
                }),
            }
        };
    }

    node bridi_relative_clause(subbridi) -> RelativeClauseSyntax {
        context "relative clause";
        fields {
            field noi = selmaho(Noi).wf();
            field subbridi = boxed(subbridi);
            field kuho = opt(cmavo(Kuho).wf());
        }
        build |noi, subbridi, kuho| {
            if noi.value.is_one_of_cmavo(crate::tree::RESTRICTIVE_RELATIVE_CLAUSE_CMAVO) {
                bityzba::new!(RelativeClauseSyntax::RestrictiveRelativeBridi {
                    poi: noi,
                    subbridi,
                    kuho,
                })
            } else {
                bityzba::new!(RelativeClauseSyntax::IncidentalRelativeBridi {
                    noi,
                    subbridi,
                    kuho,
                })
            }
        };
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
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe = None;
            scratch a = selmaho(A).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![a.value], a.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product jehi_connective -> ConnectiveSyntax {
        context "ek";
        construct variant Afterthought;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe = None;
            scratch jehi = selmaho(Jehi).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![jehi.value], jehi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product jek_connective -> ConnectiveSyntax {
        context "jek";
        construct variant Selbri;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe = None;
            scratch ja = selmaho(Ja).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![ja.value], ja.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
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
        fields {
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch joi = selmaho(Joi).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![joi.value], joi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product simple_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        fields {
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch bihi = selmaho(Bihi).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![bihi.value], bihi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product closed_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        fields {
            scratch left_interval = selmaho(Gaho);
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch bihi = selmaho(Bihi);
            scratch nai_token = opt(cmavo(Nai));
            scratch right_interval = selmaho(Gaho).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(
                vec![left_interval, bihi, right_interval.value],
                right_interval.free_modifiers,
            ));
            let nai = nai_token
                .map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product vuhu_nonlogical_connective -> ConnectiveSyntax {
        context "non-logical connective";
        construct variant NonLogical;
        fields {
            default se = None;
            default nahe = None;
            default na = None;
            scratch vuhu = selmaho(Vuhu).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![vuhu.value], vuhu.free_modifiers));
            default nai = None;
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
        fields {
            field connective = statement_connective;
            field tag_bo = opt((opt(boxed(tense_modal)), cmavo(Bo).wf()));
        }
        build |connective, tag_bo| {
            match tag_bo {
                Some((tense_modal, bo)) => append_optional_tense_modal_and_bo_to_connective(connective, tense_modal, bo),
                None => connective,
            }
        };
    }

    product i_standard_paragraph_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        fields {
            field connective = standard_paragraph_statement_connective;
            field tag_bo = opt((opt(boxed(tense_modal)), cmavo(Bo)));
        }
        build |connective, tag_bo| {
            match tag_bo {
                Some((tense_modal, bo)) => append_optional_tense_modal_and_bo_to_connective(
                    connective,
                    tense_modal,
                    WithFreeModifiers::new(bo, Vec::new()),
                ),
                None => connective,
            }
        };
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
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            field ja = selmaho(Ja);
            field nai = opt(cmavo(Nai));
        }
        build |na, se, ja, nai| bityzba::new!(ConnectiveSyntax::Selbri {
            se,
            nahe: None,
            na,
            cmavo: std::sync::Arc::new(WithFreeModifiers::new(vec![ja], Vec::new())),
            nai: nai.map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new()))),
        });
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
        fields {
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch joi = selmaho(Joi);
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![joi], Vec::new()));
            scratch nai_token = opt(cmavo(Nai));
            let nai = nai_token
                .map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product paragraph_simple_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        fields {
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch bihi = selmaho(Bihi);
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![bihi], Vec::new()));
            scratch nai_token = opt(cmavo(Nai));
            let nai = nai_token
                .map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product paragraph_closed_interval_connective -> ConnectiveSyntax {
        context "interval";
        construct variant Interval;
        fields {
            scratch left_interval = selmaho(Gaho);
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch bihi = selmaho(Bihi);
            scratch nai_token = opt(cmavo(Nai));
            scratch right_interval = selmaho(Gaho);
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(
                vec![left_interval, bihi, right_interval],
                Vec::new(),
            ));
            let nai = nai_token
                .map(|nai| std::sync::Arc::new(WithFreeModifiers::new(nai, Vec::new())));
        }
    }

    product i_tag_bo_paragraph_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo);
        }
        build |tense_modal, bo| statement_tag_bo_connective(
            tense_modal,
            WithFreeModifiers::new(bo, Vec::new()),
        );
    }

    product i_tag_bo_statement_connective(tense_modal) -> ConnectiveSyntax {
        context "statement connective";
        fields {
            field tense_modal = opt(boxed(tense_modal));
            field bo = cmavo(Bo).wf();
        }
        build |tense_modal, bo| statement_tag_bo_connective(tense_modal, bo);
    }

    product cehe_connective -> ConnectiveSyntax {
        context "termset connective";
        construct variant NonLogical;
        fields {
            default se = None;
            default nahe = None;
            default na = None;
            scratch cehe = cmavo(Cehe).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![cehe.value], cehe.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product gihek_connective -> ConnectiveSyntax {
        context "gihek";
        construct variant BridiTail;
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            default nahe = None;
            scratch giha = selmaho(Giha).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![giha.value], giha.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product guhek_connective -> ConnectiveSyntax {
        context "forethought selbri connective";
        construct variant Forethought;
        fields {
            field nahe = opt(selmaho(Nahe));
            field se = opt(selmaho(Se));
            default na = None;
            scratch guha = selmaho(Guha).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![guha.value], guha.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
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
        fields {
            field connective = relation_afterthought_connective;
        }
        build |connective| {
            let ConnectiveSyntaxParts {
                kind: _,
                se,
                nahe,
                na,
                cmavo,
                nai,
            } = connective.into_parts();
            ConnectiveSyntax::new(ConnectiveKind::BridiTail, se, nahe, na, cmavo, nai)
        };
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
        fields {
            field se = opt(selmaho(Se));
            default nahe = None;
            default na = None;
            scratch ga = selmaho(Ga).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![ga.value], ga.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
        }
    }

    product zantufa_initial_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        fields {
            field gi = cmavo(Gi).warn(ExperimentalZantufaGek).wf();
            field tail = choice((
                joik_connective(),
                jek_connective(),
            ));
            field bo = opt(cmavo(Bo).wf());
        }
        build |gi, tail, bo| build_initial_gi_forethought_connective(gi, tail, bo);
    }

    product joik_jek_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        fields {
            field connective = joik_connective();
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
        build |connective, gi, bo| append_gi_and_optional_bo_to_connective(connective, gi, bo);
    }

    product jek_gi_forethought_connective -> ConnectiveSyntax {
        context "forethought connective";
        fields {
            field na = opt(selmaho(Na));
            field se = opt(selmaho(Se));
            field ja = selmaho(Ja).warn(ExperimentalZantufaGek).wf();
            field nai = opt(cmavo(Nai).wf());
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
        build |na, se, ja, nai, gi, bo| {
            let connective = bityzba::new!(ConnectiveSyntax::Selbri {
                se,
                nahe: None,
                na,
                cmavo: std::sync::Arc::new(WithFreeModifiers::new(vec![ja.value], ja.free_modifiers)),
                nai: nai.map(std::sync::Arc::new),
            });
            append_gi_and_optional_bo_to_connective(connective, gi, bo)
        };
    }

    product modal_gi_forethought_connective(tense_modal) -> ConnectiveSyntax {
        context "forethought connective";
        fields {
            field tense_modal = boxed(tense_modal);
            field gi = cmavo(Gi).wf();
            field bo = opt(cmavo(Bo).warn(ExperimentalZantufaGek).wf());
        }
        build |tense_modal, gi, bo| forethought_tag_gi_connective(tense_modal, gi, bo);
    }

    product gik_connective -> ConnectiveSyntax {
        context "forethought connective";
        construct variant Forethought;
        fields {
            default se = None;
            default nahe = None;
            default na = None;
            scratch gi = cmavo(Gi).wf();
            let cmavo = std::sync::Arc::new(WithFreeModifiers::new(vec![gi.value], gi.free_modifiers));
            scratch nai_token = opt(cmavo(Nai).wf());
            let nai = nai_token.map(std::sync::Arc::new);
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
            field first = tense_modal_atom(selbri);
            field continuations = many1((tag_connective, tense_modal_atom(selbri)));
        }
        build |first, continuations| combine_connected_tense_modal(first, continuations);
    }

    node tense_modal_atom(selbri) -> TenseModalSyntax {
        context "tag";
        fields {
            field tense_modal = choice((
                composite_tense(),
                fiho_tense(selbri),
                modal_tense(),
                flat_prefixed_tense(),
                feature(ZantufaTags, zantufa_recursive_tag_tense()),
                sticky_tense(),
            ));
        }
        build |tense_modal| tense_modal;
    }

    node fiho_tense(selbri) -> TenseModalSyntax {
        context "FIhO modal";
        fields {
            field fiho = cmavo(Fiho).wf();
            field selbri = boxed(selbri);
            field fehu = opt(cmavo(Fehu).wf());
        }
        build |fiho, selbri, fehu| bityzba::new!(TenseModalSyntax::AdHocModal {
            fiho,
            selbri,
            fehu,
        });
    }

    node flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field tense_modal = choice((
                nahe_se_flat_prefixed_tense(),
                se_flat_prefixed_tense(),
                fa_flat_tag_tense(),
            ));
        }
        build |tense_modal| tense_modal;
    }

    node fa_flat_tag_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field fa = selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        }
        build |fa| composite_from_wf_tokens(vec![fa]);
    }

    product flat_tag_atom -> (std::vec::Vec<Token>, std::vec::Vec<FreeModifierSyntax>) {
        context "tag";
        fields {
            field atom = choice((
                fa_flat_tag_atom(),
                modal_flat_tag_atom(),
                composite_flat_tag_atom(),
            ));
        }
        build |atom| atom;
    }

    product fa_flat_tag_atom -> (std::vec::Vec<Token>, std::vec::Vec<FreeModifierSyntax>) {
        context "tag";
        fields {
            field fa = selmaho(Fa).warn(ExperimentalFaAsTag).wf();
        }
        build |fa| (vec![fa.value], fa.free_modifiers);
    }

    product modal_flat_tag_atom -> (std::vec::Vec<Token>, std::vec::Vec<FreeModifierSyntax>) {
        context "modal tag";
        fields {
            field modal = modal_tense();
        }
        build |modal| modal.leaf_words_and_free_modifiers();
    }

    product composite_flat_tag_atom -> (std::vec::Vec<Token>, std::vec::Vec<FreeModifierSyntax>) {
        context "tag";
        fields {
            field composite = composite_tense();
        }
        build |composite| composite.leaf_words_and_free_modifiers();
    }

    node nahe_se_flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field nahe = selmaho(Nahe).warn(ExperimentalFlattenedTag).wf();
            field se = opt(selmaho(Se).wf());
            field atom = flat_tag_atom();
        }
        build |nahe, se, atom| {
            let mut value = vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(nahe.value))];
            let mut free_modifiers = nahe.free_modifiers;
            if let Some(se) = se {
                value.push(bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(se.value)));
                free_modifiers.extend(se.free_modifiers);
            }
            let (atom_words, atom_free_modifiers) = atom;
            value.extend(atom_words.into_iter().map(|word| bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(word))));
            free_modifiers.extend(atom_free_modifiers);
            bityzba::new!(TenseModalSyntax::Composite {
                parts: WithFreeModifiers::new(value, free_modifiers),
            })
        };
    }

    node se_flat_prefixed_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field se = selmaho(Se).warn(ExperimentalFlattenedTag).wf();
            field atom = flat_tag_atom();
        }
        build |se, atom| {
            let mut free_modifiers = se.free_modifiers;
            let (atom_words, atom_free_modifiers) = atom;
            free_modifiers.extend(atom_free_modifiers);
            let mut value = vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(se.value))];
            value.extend(atom_words.into_iter().map(|word| bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(word))));
            bityzba::new!(TenseModalSyntax::Composite {
                parts: WithFreeModifiers::new(value, free_modifiers),
            })
        };
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
        build |first_prefix, additional_prefixes, atom| {
            let mut value = vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(first_prefix.value))];
            let mut free_modifiers = first_prefix.free_modifiers;
            for prefix in additional_prefixes {
                value.push(bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(prefix.value)));
                free_modifiers.extend(prefix.free_modifiers);
            }
            value.push(bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(atom.value)));
            free_modifiers.extend(atom.free_modifiers);
            bityzba::new!(TenseModalSyntax::Composite {
                parts: WithFreeModifiers::new(value, free_modifiers),
            })
        };
    }

    node composite_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field tense_modal = choice((
                prefixed_time_space_caha_tense(),
                time_space_caha_ki_tense(),
                cuhe_tense(),
            ));
        }
        build |tense_modal| tense_modal;
    }

    node prefixed_time_space_caha_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field nahe = selmaho(Nahe).wf();
            field tense = time_space_caha_tense();
            field ki = opt(ki_composite_tense());
        }
        build |nahe, tense, ki| {
            let mut parts = vec![composite_from_wf_tokens(vec![nahe]), tense];
            parts.extend(ki);
            combine_composite_tense_modals(parts)
        };
    }

    node time_space_caha_ki_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field tense = time_space_caha_tense();
            field ki = opt(ki_composite_tense());
        }
        build |tense, ki| {
            let mut parts = vec![tense];
            parts.extend(ki);
            combine_composite_tense_modals(parts)
        };
    }

    node time_space_caha_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field tense = choice((
                time_then_space_caha_tense(),
                space_then_time_caha_tense(),
                caha_tense(),
            ));
        }
        build |tense| tense;
    }

    node time_then_space_caha_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field time = time_tense();
            field space = opt(space_tense());
            field caha = opt(caha_tense());
        }
        build |time, space, caha| {
            let mut parts = vec![time];
            parts.extend(space);
            parts.extend(caha);
            combine_composite_tense_modals(parts)
        };
    }

    node space_then_time_caha_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field space = space_tense();
            field time = opt(time_tense());
            field caha = opt(caha_tense());
        }
        build |space, time, caha| {
            let mut parts = vec![space];
            parts.extend(time);
            parts.extend(caha);
            combine_composite_tense_modals(parts)
        };
    }

    node time_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field tense = choice((
                time_tense_with_zi(),
                time_tense_with_offset(),
                time_tense_with_interval(),
                time_tense_with_properties(),
            ));
        }
        build |tense| tense;
    }

    node time_tense_with_zi -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = zi_time_distance_tense();
            field offsets = many(pu_time_offset_tense());
            field zeha = opt(zeha_time_interval_tense());
            field properties = many(interval_property_tense());
        }
        build |zi, offsets, zeha, properties| {
            let mut parts = vec![zi];
            parts.extend(offsets);
            parts.extend(zeha);
            parts.extend(properties);
            combine_composite_tense_modals(parts)
        };
    }

    node time_tense_with_offset -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(zi_time_distance_tense());
            field offsets = many1(pu_time_offset_tense());
            field zeha = opt(zeha_time_interval_tense());
            field properties = many(interval_property_tense());
        }
        build |zi, offsets, zeha, properties| {
            let mut parts = Vec::new();
            parts.extend(zi);
            parts.extend(offsets);
            parts.extend(zeha);
            parts.extend(properties);
            combine_composite_tense_modals(parts)
        };
    }

    node time_tense_with_interval -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(zi_time_distance_tense());
            field offsets = many(pu_time_offset_tense());
            field zeha = zeha_time_interval_tense();
            field properties = many(interval_property_tense());
        }
        build |zi, offsets, zeha, properties| {
            let mut parts = Vec::new();
            parts.extend(zi);
            parts.extend(offsets);
            parts.push(zeha);
            parts.extend(properties);
            combine_composite_tense_modals(parts)
        };
    }

    node time_tense_with_properties -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = opt(zi_time_distance_tense());
            field offsets = many(pu_time_offset_tense());
            field zeha = opt(zeha_time_interval_tense());
            field properties = many1(interval_property_tense());
        }
        build |zi, offsets, zeha, properties| {
            let mut parts = Vec::new();
            parts.extend(zi);
            parts.extend(offsets);
            parts.extend(zeha);
            parts.extend(properties);
            combine_composite_tense_modals(parts)
        };
    }

    node interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field tense = choice((
                numbered_interval_property_tense(),
                tahe_interval_property_tense(),
                zaho_interval_property_tense(),
            ));
        }
        build |tense| tense;
    }

    node numbered_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field number = interval_property_number_words().wf();
            field roi = selmaho(Roi).wf();
            field nai = opt(cmavo(Nai).wf());
        }
        build |number, roi, nai| {
            let mut value = number.value;
            let mut free_modifiers = number.free_modifiers;
            value.push(roi.value);
            free_modifiers.extend(roi.free_modifiers);
            if let Some(nai) = nai {
                value.push(nai.value);
                free_modifiers.extend(nai.free_modifiers);
            }
            composite_from_wf_token_parts(value, free_modifiers)
        };
    }

    product interval_property_number_words -> std::vec::Vec<Token> {
        context "number";
        fields {
            field first = pa_word();
            field rest = many(choice((
                pa_word_as_words(),
                plain_letter_word_as_words(),
            )));
        }
        build |first, rest| {
            let mut words = vec![first];
            for mut group in rest {
                words.append(&mut group);
            }
            words
        };
    }

    node tahe_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field tahe = selmaho(Tahe).wf();
            field nai = opt(cmavo(Nai).wf());
        }
        build |tahe, nai| {
            let mut parts = vec![tahe];
            parts.extend(nai);
            composite_from_wf_tokens(parts)
        };
    }

    node zaho_interval_property_tense -> TenseModalSyntax {
        context "interval property";
        fields {
            field zaho = selmaho(Zaho).wf();
            field nai = opt(cmavo(Nai).wf());
        }
        build |zaho, nai| {
            let mut parts = vec![zaho];
            parts.extend(nai);
            composite_from_wf_tokens(parts)
        };
    }

    node pu_time_offset_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field pu = selmaho(Pu).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Zi).wf());
        }
        build |pu, nai, distance| {
            let mut parts = vec![pu];
            parts.extend(nai);
            parts.extend(distance);
            composite_from_wf_tokens(parts)
        };
    }

    node zi_time_distance_tense -> TenseModalSyntax {
        context "time tense";
        fields {
            field zi = selmaho(Zi).wf();
        }
        build |zi| composite_from_wf_tokens(vec![zi]);
    }

    node zeha_time_interval_tense -> TenseModalSyntax {
        context "time interval";
        fields {
            field zeha = selmaho(Zeha).wf();
            field direction = opt((selmaho(Pu).wf(), opt(cmavo(Nai).wf())));
        }
        build |zeha, direction| {
            let mut parts = vec![zeha];
            if let Some((pu, nai)) = direction {
                parts.push(pu);
                parts.extend(nai);
            }
            composite_from_wf_tokens(parts)
        };
    }

    node space_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field tense = choice((
                space_tense_with_va(),
                space_tense_with_offset(),
                space_tense_with_interval(),
                space_tense_with_mohi(),
            ));
        }
        build |tense| tense;
    }

    node space_tense_with_va -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = va_space_distance_tense();
            field offsets = many(faha_space_offset_tense());
            field interval = opt(space_interval_tense());
            field mohi = opt(mohi_space_offset_tense());
        }
        build |va, offsets, interval, mohi| {
            let mut parts = vec![va];
            parts.extend(offsets);
            parts.extend(interval);
            parts.extend(mohi);
            combine_composite_tense_modals(parts)
        };
    }

    node space_tense_with_offset -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(va_space_distance_tense());
            field offsets = many1(faha_space_offset_tense());
            field interval = opt(space_interval_tense());
            field mohi = opt(mohi_space_offset_tense());
        }
        build |va, offsets, interval, mohi| {
            let mut parts = Vec::new();
            parts.extend(va);
            parts.extend(offsets);
            parts.extend(interval);
            parts.extend(mohi);
            combine_composite_tense_modals(parts)
        };
    }

    node space_tense_with_interval -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(va_space_distance_tense());
            field offsets = many(faha_space_offset_tense());
            field interval = space_interval_tense();
            field mohi = opt(mohi_space_offset_tense());
        }
        build |va, offsets, interval, mohi| {
            let mut parts = Vec::new();
            parts.extend(va);
            parts.extend(offsets);
            parts.push(interval);
            parts.extend(mohi);
            combine_composite_tense_modals(parts)
        };
    }

    node space_tense_with_mohi -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = opt(va_space_distance_tense());
            field offsets = many(faha_space_offset_tense());
            field interval = opt(space_interval_tense());
            field mohi = mohi_space_offset_tense();
        }
        build |va, offsets, interval, mohi| {
            let mut parts = Vec::new();
            parts.extend(va);
            parts.extend(offsets);
            parts.extend(interval);
            parts.push(mohi);
            combine_composite_tense_modals(parts)
        };
    }

    node va_space_distance_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field va = selmaho(Va).wf();
        }
        build |va| composite_from_wf_tokens(vec![va]);
    }

    node faha_space_offset_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field faha = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
            field distance = opt(selmaho(Va).wf());
        }
        build |faha, nai, distance| {
            let mut parts = vec![faha];
            parts.extend(nai);
            parts.extend(distance);
            composite_from_wf_tokens(parts)
        };
    }

    node faha_interval_direction_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field faha = selmaho(Faha).wf();
            field nai = opt(cmavo(Nai).wf());
        }
        build |faha, nai| {
            let mut parts = vec![faha];
            parts.extend(nai);
            composite_from_wf_tokens(parts)
        };
    }

    node space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field tense = choice((
                space_interval_with_extent_tense(),
                space_interval_properties_tense(),
            ));
        }
        build |tense| tense;
    }

    node space_interval_with_extent_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field extent = veha_viha_space_interval_tense();
            field direction = opt(faha_interval_direction_tense());
            field properties = opt(space_interval_properties_tense());
        }
        build |extent, direction, properties| {
            let mut parts = vec![extent];
            parts.extend(direction);
            parts.extend(properties);
            combine_composite_tense_modals(parts)
        };
    }

    node space_interval_properties_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field first = fehe_interval_property_tense();
            field additional = many(fehe_interval_property_tense());
        }
        build |first, additional| {
            let mut parts = vec![first];
            parts.extend(additional);
            combine_composite_tense_modals(parts)
        };
    }

    node veha_viha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field tense = choice((
                veha_space_interval_tense(),
                viha_space_interval_tense(),
            ));
        }
        build |tense| tense;
    }

    node veha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field veha = selmaho(Veha).wf();
            field viha = opt(selmaho(Viha).wf());
        }
        build |veha, viha| {
            let mut parts = vec![veha];
            parts.extend(viha);
            composite_from_wf_tokens(parts)
        };
    }

    node viha_space_interval_tense -> TenseModalSyntax {
        context "space interval";
        fields {
            field viha = selmaho(Viha).wf();
        }
        build |viha| composite_from_wf_tokens(vec![viha]);
    }

    node fehe_interval_property_tense -> TenseModalSyntax {
        context "space interval property";
        fields {
            field fehe = cmavo(Fehe).wf();
            field property = interval_property_tense();
        }
        build |fehe, property| combine_composite_tense_modals(vec![
            composite_from_wf_tokens(vec![fehe]),
            property,
        ]);
    }

    node mohi_space_offset_tense -> TenseModalSyntax {
        context "space tense";
        fields {
            field mohi = selmaho(Mohi).wf();
            field offset = faha_space_offset_tense();
        }
        build |mohi, offset| combine_composite_tense_modals(vec![
            composite_from_wf_tokens(vec![mohi]),
            offset,
        ]);
    }

    node caha_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field caha = selmaho(Caha).wf();
        }
        build |caha| composite_from_wf_tokens(vec![caha]);
    }

    node ki_composite_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field ki = cmavo(Ki).wf();
        }
        build |ki| composite_from_wf_tokens(vec![ki]);
    }

    node cuhe_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field cuhe = selmaho(Cuhe).wf();
        }
        build |cuhe| composite_from_wf_tokens(vec![cuhe]);
    }

    node pu_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field pu = selmaho(Pu).wf();
        }
        build |pu| bityzba::new!(TenseModalSyntax::Composite {
            parts: WithFreeModifiers::new(
                vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(pu.value))],
                pu.free_modifiers,
            ),
        });
    }

    node va_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field va = selmaho(Va).wf();
        }
        build |va| bityzba::new!(TenseModalSyntax::Composite {
            parts: WithFreeModifiers::new(
                vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(va.value))],
                va.free_modifiers,
            ),
        });
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
        build |nahe, se, bai, nai, ki| bityzba::new!(TenseModalSyntax::Modal {
            nahe,
            se,
            bai,
            nai,
            ki,
        });
    }

    node fa_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field fa = selmaho(Fa).wf();
        }
        build |fa| bityzba::new!(TenseModalSyntax::Composite {
            parts: WithFreeModifiers::new(
                vec![bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(fa.value))],
                fa.free_modifiers,
            ),
        });
    }

    node sticky_tense -> TenseModalSyntax {
        context "tag";
        fields {
            field ki = cmavo(Ki).wf();
        }
        build |ki| bityzba::new!(TenseModalSyntax::Sticky(ki));
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
        build |leading_selbri, co_tail| {
            if let Some((co, trailing_selbri)) = co_tail {
                bityzba::new!(SelbriSyntax::InvertedTanru {
                    leading_selbri,
                    co,
                    trailing_selbri,
                })
            } else {
                *leading_selbri
            }
        };
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
        build |guhek, leading_selbri, gik, trailing_selbri, gihi| {
            bityzba::new!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi: Box::new(selbri_to_empty_bridi(*leading_selbri)),
                gik,
                trailing_bridi: Box::new(selbri_to_empty_bridi(*trailing_selbri)),
                gihi,
            })
        };
    }

    node connected_selbri(tanru_unit) -> SelbriSyntax {
        context "selbri connection";
        fields {
            field leading_selbri = boxed(tanru_selbri(tanru_unit));
            field continuations = many((relation_afterthought_connective, boxed(tanru_selbri(tanru_unit))));
        }
        build |leading_selbri, continuations| {
            continuations.into_iter().fold(*leading_selbri, |leading_selbri, (connective, trailing_selbri)| {
                bityzba::new!(SelbriSyntax::SelbriConnection {
                    leading_selbri: Box::new(leading_selbri),
                    connective,
                    trailing_selbri,
                })
            })
        };
    }

    node tanru_selbri(tanru_unit) -> SelbriSyntax {
        context "selbri";
        fields {
            field first_unit = tanru_unit;
            field additional_units = many(tanru_unit);
        }
        build |first_unit, additional_units| {
            let units = vec1::Vec1::try_from_vec(
                std::iter::once(first_unit).chain(additional_units).collect()
            ).expect("first tanru unit guarantees non-empty tanru");
            if units.len() == 1 {
                let unit = units.into_iter().next().expect("non-empty tanru");
                tanru_unit_to_single_selbri(unit)
            } else {
                bityzba::new!(SelbriSyntax::Tanru(Box::new(units)))
            }
        };
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
        build |leading_unit, continuations| {
            continuations.into_iter().fold(*leading_unit, |leading_unit, (connective, trailing_unit)| {
                bityzba::new!(TanruUnitSyntax::TanruUnitConnection {
                    leading_unit: Box::new(leading_unit),
                    connective,
                    trailing_unit,
                })
            })
        };
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
        build |guhek, leading_selbri, gik, trailing_unit, gihi| {
            bityzba::new!(TanruUnitSyntax::SelbriGroupTanruUnit(Box::new(
                bityzba::new!(SelbriSyntax::ForethoughtSelbriConnection {
                    guhek,
                    leading_bridi: Box::new(selbri_to_empty_bridi(*leading_selbri)),
                    gik,
                    trailing_bridi: Box::new(selbri_to_empty_bridi(tanru_unit_to_single_selbri(*trailing_unit))),
                    gihi,
                })
            )))
        };
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
        build |base, assignments| {
            bityzba::new!(TanruUnitSyntax::AssignedProBridi {
                base,
                assignments: assignments
                    .into_iter()
                    .map(|(cei, tanru_unit)| {
                        bityzba::new!(ProBridiAssignmentSyntax {
                            cei,
                            tanru_unit,
                        })
                    })
                    .collect(),
            })
        };
    }

    node linked_tanru_unit(tanru_unit_atom, sumti, tense_modal) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field base = boxed(tanru_unit_atom);
            field linkargs = opt(linkargs(sumti, tense_modal));
        }
        build |base, linkargs| {
            if let Some(linkargs) = linkargs {
                let bityzba::data!(LinkedSumtiListSyntax {
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                }) = linkargs.into_data();
                bityzba::new!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                    base,
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                })
            } else {
                *base
            }
        };
    }

    node linked_tanru_unit_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field base = boxed(tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
            field linkargs = opt(linkargs(sumti, tense_modal));
        }
        build |base, linkargs| {
            if let Some(linkargs) = linkargs {
                let bityzba::data!(LinkedSumtiListSyntax {
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                }) = linkargs.into_data();
                bityzba::new!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                    base,
                    be,
                    fa,
                    first_sumti,
                    bei_links,
                    beho,
                })
            } else {
                *base
            }
        };
    }

    node tanru_unit_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string) -> TanruUnitSyntax {
        context "tanru unit";
        fields {
            field conversions = many(selmaho(Se).wf());
            field base = boxed(tanru_unit_base_atom_for_cei(tanru_unit_atom, tanru_unit, subbridi, sumti, selbri, text, tense_modal, free_modifier, jai_inner_tanru_unit, mekso_operator, letter_tokens, letter_string));
        }
        build |conversions, base| {
            conversions.into_iter().rev().fold(*base, |inner_unit, se| {
                bityzba::new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se,
                    inner_unit: Box::new(inner_unit),
                })
            })
        };
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
        build |conversions, base| {
            conversions.into_iter().rev().fold(*base, |inner_unit, se| {
                bityzba::new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se,
                    inner_unit: Box::new(inner_unit),
                })
            })
        };
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
        build |tense_modal, inner_selbri| bityzba::new!(TanruUnitSyntax::SelbriGroupTanruUnit(Box::new(
            bityzba::new!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            })
        )));
    }

    node preposed_linkargs_tanru_unit(tanru_unit, sumti, tense_modal) -> TanruUnitSyntax {
        context "linked arguments";
        fields {
            field linkargs = linkargs(sumti, tense_modal);
            field base = boxed(tanru_unit);
        }
        build |linkargs, base| {
            let bityzba::data!(LinkedSumtiListSyntax {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) = linkargs.into_data();
            bityzba::new!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
                base,
            })
        };
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
        build |number, moi| {
            bityzba::new!(TanruUnitSyntax::OrdinalSelbri {
                number: WordRun::try_from_vec(number).expect("first ordinal word guarantees non-empty ordinal"),
                moi,
            })
        };
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
        build |words| bityzba::new!(SumtiSyntax::LerfuStringSumti {
            letter: WithFreeModifiers::new(
                WordRun::try_from_vec(words).expect("first letter guarantees non-empty lerfu words"),
                Vec::new(),
            ),
            boi: None,
        });
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
            default ke_tense_modal = None;
            field ke = cmavo(Ke).wf();
            field selbri = boxed(connected_selbri(tanru_unit));
            field kehe = opt(cmavo(Kehe).wf());
        }
    }

    node empty_linked_sumti -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            default fa = None;
            default sumti = None;
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
        build |fa, sumti| {
            match (*sumti).into_data() {
                bityzba::data!(SumtiSyntax::ElidedSumti { maybe_ku, free_modifiers, .. }) => {
                    bityzba::new!(LinkedSumtiSyntax {
                        fa: None,
                        sumti: Some(Box::new(bityzba::new!(SumtiSyntax::ElidedSumti {
                            tag: Some(Box::new(bityzba::new!(SumtiTagSyntax::PlaceTag(fa)))),
                            maybe_ku,
                            free_modifiers,
                        }))),
                    })
                }
                data => bityzba::new!(LinkedSumtiSyntax {
                    fa: Some(fa),
                    sumti: Some(Box::new(SumtiSyntax::from_data(data))),
                }),
            }
        };
    }

    node tense_tagged_linked_sumti(sumti, tense_modal) -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field tense_modal = boxed(tense_modal);
            field sumti = boxed(linked_sumti_tail(sumti));
        }
        build |tense_modal, sumti| {
            let tag = bityzba::new!(SumtiTagSyntax::TenseModal(tense_modal));
            match (*sumti).into_data() {
                bityzba::data!(SumtiSyntax::ElidedSumti { maybe_ku, free_modifiers, .. }) => {
                    bityzba::new!(LinkedSumtiSyntax {
                        fa: None,
                        sumti: Some(Box::new(bityzba::new!(SumtiSyntax::ElidedSumti {
                            tag: Some(Box::new(tag)),
                            maybe_ku,
                            free_modifiers,
                        }))),
                    })
                }
                data => bityzba::new!(LinkedSumtiSyntax {
                    fa: None,
                    sumti: Some(Box::new(bityzba::new!(SumtiSyntax::TaggedSumti {
                        tag,
                        inner_sumti: Box::new(SumtiSyntax::from_data(data)),
                    }))),
                }),
            }
        };
    }

    node plain_linked_sumti(sumti) -> LinkedSumtiSyntax {
        context "linked arguments";
        fields {
            field sumti = boxed(sumti);
        }
        build |sumti| bityzba::new!(LinkedSumtiSyntax {
            fa: None,
            sumti: Some(sumti),
        });
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
        build |bei, link| {
            let bityzba::data!(LinkedSumtiSyntax { fa, sumti }) = link.into_data();
            bityzba::new!(AdditionalLinkedSumtiSyntax { bei, fa, sumti })
        };
    }

    node linkargs(sumti, tense_modal) -> LinkedSumtiListSyntax {
        context "linked arguments";
        fields {
            field be = cmavo(Be).wf();
            field first_link = linked_sumti(sumti, tense_modal);
            field bei_links = many(bei_link(sumti, tense_modal));
            field beho = opt(cmavo(Beho).wf());
        }
        build |be, first_link, bei_links, beho| {
            let bityzba::data!(LinkedSumtiSyntax {
                fa,
                sumti: first_sumti,
            }) = first_link.into_data();
            bityzba::new!(LinkedSumtiListSyntax {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            })
        };
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
        build |nu, nai, abstractor_connections, subbridi, kei| bityzba::new!(TanruUnitSyntax::Abstraction(Box::new(
            bityzba::new!(AbstractionSyntax {
                nu,
                nai,
                abstractor_connections,
                subbridi,
                kei,
            })
        )));
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

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn prepend_leading_i_statement(text: TextSyntax, marker: LeadingIStatementSyntax) -> TextSyntax {
    let bityzba::data!(LeadingIStatementSyntax {
        i,
        connective,
        free_modifiers,
    }) = marker.into_data();
    let mut text_data = text.into_data();
    if text_data.paragraphs.is_empty() {
        text_data.paragraphs.push(bityzba::new!(ParagraphSyntax {
            i: None,
            niho: Vec::new(),
            free_modifiers: Vec::new(),
            statements: vec![bityzba::new!(ParagraphStatementSyntax {
                i: Some(i),
                connective,
                free_modifiers,
                statement: None,
            })],
        }));
        return TextSyntax::from_data(text_data);
    }

    let mut paragraph_data = text_data.paragraphs.remove(0).into_data();
    if paragraph_data.niho.is_empty() {
        paragraph_data.statements = prepend_i_to_niho_free_paragraph_statements(
            i,
            connective,
            free_modifiers,
            std::mem::take(&mut paragraph_data.statements),
        );
    } else {
        paragraph_data.i = Some(i);
        paragraph_data.statements = attach_leading_i_connective_to_niho_paragraph_statements(
            connective,
            free_modifiers,
            std::mem::take(&mut paragraph_data.statements),
        );
    }
    text_data
        .paragraphs
        .insert(0, ParagraphSyntax::from_data(paragraph_data));
    TextSyntax::from_data(text_data)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn prepend_i_to_niho_free_paragraph_statements(
    i: Token,
    connective: Option<Box<ConnectiveSyntax>>,
    free_modifiers: Vec<FreeModifierSyntax>,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    if statements.is_empty() {
        return vec![bityzba::new!(ParagraphStatementSyntax {
            i: Some(i),
            connective,
            free_modifiers,
            statement: None,
        })];
    }
    if statements.first().is_some_and(|first| first.i.is_some()) {
        statements.insert(
            0,
            bityzba::new!(ParagraphStatementSyntax {
                i: Some(i),
                connective,
                free_modifiers,
                statement: None,
            }),
        );
        return statements;
    }

    let mut first_data = statements.remove(0).into_data();
    first_data.i = Some(i);
    first_data.connective = connective;
    first_data.free_modifiers = free_modifiers;
    statements.insert(0, ParagraphStatementSyntax::from_data(first_data));
    statements
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn attach_leading_i_connective_to_niho_paragraph_statements(
    connective: Option<Box<ConnectiveSyntax>>,
    free_modifiers: Vec<FreeModifierSyntax>,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    if statements.is_empty() {
        return vec![bityzba::new!(ParagraphStatementSyntax {
            i: None,
            connective,
            free_modifiers,
            statement: None,
        })];
    }

    let mut first_data = statements.remove(0).into_data();
    first_data.connective = connective;
    let mut combined_free_modifiers = free_modifiers;
    combined_free_modifiers.append(&mut first_data.free_modifiers);
    first_data.free_modifiers = combined_free_modifiers;
    statements.insert(0, ParagraphStatementSyntax::from_data(first_data));
    statements
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn connective_words_with_free_modifiers(
    connective: ConnectiveSyntax,
) -> WithFreeModifiers<Vec<Token>> {
    let ConnectiveSyntaxParts {
        kind: _,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    let mut value = Vec::new();
    value.extend(se);
    value.extend(nahe);
    value.extend(na);
    value.append(&mut cmavo.value);
    if let Some(nai) = nai {
        value.push(nai.value);
        cmavo.free_modifiers.extend(nai.free_modifiers);
    }
    WithFreeModifiers::new(value, cmavo.free_modifiers)
}

#[bityzba::requires(!tokens.is_empty())]
#[bityzba::ensures(matches!(
    ret.as_data(),
    bityzba::data!(TenseModalSyntax::Composite { .. })
))]
fn composite_from_wf_token_parts(
    tokens: Vec<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
    bityzba::new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(
            tokens
                .into_iter()
                .map(|token| bityzba::new!(CompositeTenseModalPartSyntax::Cmavo(token)))
                .collect(),
            free_modifiers,
        ),
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn attach_free_modifiers_to_optional_terminator(
    terminator: Option<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> (Vec<FreeModifierSyntax>, Option<WithFreeModifiers<Token>>) {
    if let Some(terminator) = terminator {
        (
            Vec::new(),
            Some(WithFreeModifiers::new(terminator, free_modifiers)),
        )
    } else {
        (free_modifiers, None)
    }
}

#[bityzba::requires(!tokens.is_empty())]
#[bityzba::ensures(matches!(
    ret.as_data(),
    bityzba::data!(TenseModalSyntax::Composite { .. })
))]
fn composite_from_wf_tokens(tokens: Vec<WithFreeModifiers<Token>>) -> TenseModalSyntax {
    let mut values = Vec::with_capacity(tokens.len());
    let mut free_modifiers = Vec::new();
    for token in tokens {
        values.push(token.value);
        free_modifiers.extend(token.free_modifiers);
    }
    composite_from_wf_token_parts(values, free_modifiers)
}

#[bityzba::requires(!parts.is_empty())]
#[bityzba::ensures(matches!(
    ret.as_data(),
    bityzba::data!(TenseModalSyntax::Composite { .. })
))]
fn combine_composite_tense_modals(parts: Vec<TenseModalSyntax>) -> TenseModalSyntax {
    let mut combined_parts = Vec::new();
    let mut free_modifiers = Vec::new();
    for part in parts {
        let part = tense_modal_as_composite(part);
        let bityzba::data!(TenseModalSyntax::Composite { parts }) = part.into_data() else {
            unreachable!("tense_modal_as_composite always returns a composite")
        };
        combined_parts.extend(parts.value);
        free_modifiers.extend(parts.free_modifiers);
    }

    bityzba::new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(combined_parts, free_modifiers),
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(matches!(
    ret.as_data(),
    bityzba::data!(TenseModalSyntax::Composite { .. })
))]
fn combine_connected_tense_modal(
    first: TenseModalSyntax,
    continuations: Vec<(ConnectiveSyntax, TenseModalSyntax)>,
) -> TenseModalSyntax {
    let mut parts = vec![tense_modal_as_composite(first)];
    for (connective, tense_modal) in continuations {
        let ConnectiveSyntaxParts {
            kind: _,
            se,
            nahe,
            na,
            cmavo,
            nai,
        } = connective.into_parts();
        let mut leaves = Vec::new();
        leaves.extend(se);
        leaves.extend(nahe);
        leaves.extend(na);
        leaves.extend(cmavo.value);
        if let Some(nai) = nai {
            leaves.push(nai.value);
        }
        parts.push(connective_tense_modal_from_leaves(leaves));
        parts.push(tense_modal_as_composite(tense_modal));
    }

    combine_composite_tense_modals(parts)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn build_connected_i_statement(
    leading_statement: StatementSyntax,
    continuations: Vec<(Token, ConnectiveSyntax, Box<StatementSyntax>)>,
) -> StatementSyntax {
    let mut statements = vec![leading_statement];
    let mut connectors = Vec::new();
    for (i, connective, trailing_statement) in continuations {
        connectors.push((i, connective));
        statements.push(*trailing_statement);
    }

    let mut right_statement = statements
        .pop()
        .expect("there is always at least the leading statement");
    let mut pending_non_bo = Vec::new();
    while let Some((i, connective)) = connectors.pop() {
        let left_statement = statements
            .pop()
            .expect("connectors are paired with a leading statement");
        if connective_has_bo(&connective) {
            right_statement =
                connected_i_statement_node(i, connective, left_statement, right_statement);
        } else {
            pending_non_bo.push((i, connective, right_statement));
            right_statement = left_statement;
        }
    }

    let mut connected_statement = right_statement;
    for (i, connective, trailing_statement) in pending_non_bo.into_iter().rev() {
        connected_statement =
            connected_i_statement_node(i, connective, connected_statement, trailing_statement);
    }
    connected_statement
}

#[bityzba::requires(!pending.is_empty())]
#[bityzba::ensures(true)]
fn build_chained_i_connective_statement_tail(
    pending: Vec<(Token, ConnectiveSyntax)>,
    i: Token,
    connective: ConnectiveSyntax,
    trailing_statement: Box<StatementSyntax>,
) -> (Token, ConnectiveSyntax, Box<StatementSyntax>) {
    let mut pending = pending.into_iter();
    let (first_i, first_connective) = pending
        .next()
        .expect("pending_i_connective is parsed with many1");
    let mut pending_words = first_connective.words();
    for (pending_i, pending_connective) in pending {
        pending_words.push(pending_i);
        pending_words.extend(pending_connective.words());
    }
    pending_words.push(i);
    (
        first_i,
        prepend_connective_words(pending_words, connective),
        trailing_statement,
    )
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn connected_i_statement_node(
    i: Token,
    connective: ConnectiveSyntax,
    leading_statement: StatementSyntax,
    trailing_statement: StatementSyntax,
) -> StatementSyntax {
    bityzba::new!(StatementSyntax::StatementConnection {
        i,
        connective,
        leading_statement: Box::new(leading_statement),
        trailing_statement: Box::new(trailing_statement),
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(ret == connective.cmavo().value.iter().any(|word| word.is_cmavo(Cmavo::Bo)))]
fn connective_has_bo(connective: &ConnectiveSyntax) -> bool {
    connective
        .cmavo()
        .value
        .iter()
        .any(|word| word.is_cmavo(Cmavo::Bo))
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn build_initial_gi_forethought_connective(
    gi: WithFreeModifiers<Token>,
    tail: ConnectiveSyntax,
    bo: Option<WithFreeModifiers<Token>>,
) -> ConnectiveSyntax {
    let mut value = vec![gi.value];
    let mut free_modifiers = gi.free_modifiers;
    let tail = connective_words_with_free_modifiers(tail);
    value.extend(tail.value);
    free_modifiers.extend(tail.free_modifiers);
    if let Some(bo) = bo {
        value.push(bo.value);
        free_modifiers.extend(bo.free_modifiers);
    }
    ConnectiveSyntax::new(
        ConnectiveKind::Forethought,
        None,
        None,
        None,
        WithFreeModifiers::new(value, free_modifiers),
        None,
    )
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn statement_tag_bo_connective(
    tense_modal: Option<Box<TenseModalSyntax>>,
    bo: WithFreeModifiers<Token>,
) -> ConnectiveSyntax {
    let mut value = Vec::new();
    if let Some(tense_modal) = tense_modal {
        tense_modal.extend_words_into(&mut value);
    }
    value.push(bo.value);
    ConnectiveSyntax::new(
        ConnectiveKind::Selbri,
        None,
        None,
        None,
        WithFreeModifiers::new(value, bo.free_modifiers),
        None,
    )
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn append_gi_and_optional_bo_to_connective(
    connective: ConnectiveSyntax,
    gi: WithFreeModifiers<Token>,
    bo: Option<WithFreeModifiers<Token>>,
) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    cmavo.value.push(gi.value);
    cmavo.free_modifiers.extend(gi.free_modifiers);
    if let Some(bo) = bo {
        cmavo.value.push(bo.value);
        cmavo.free_modifiers.extend(bo.free_modifiers);
    }
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn forethought_tag_gi_connective(
    tense_modal: Box<TenseModalSyntax>,
    gi: WithFreeModifiers<Token>,
    bo: Option<WithFreeModifiers<Token>>,
) -> ConnectiveSyntax {
    let mut value = Vec::new();
    tense_modal.extend_words_into(&mut value);
    value.push(gi.value);
    let mut free_modifiers = gi.free_modifiers;
    if let Some(bo) = bo {
        value.push(bo.value);
        free_modifiers.extend(bo.free_modifiers);
    }
    ConnectiveSyntax::new(
        ConnectiveKind::Forethought,
        None,
        None,
        None,
        WithFreeModifiers::new(value, free_modifiers),
        None,
    )
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret.is_empty())]
fn description_tail_sumti_elements(sumti: Box<SumtiSyntax>) -> Vec<DescriptionTailElementSyntax> {
    match (*sumti).into_data() {
        bityzba::data!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti,
            vuho: _,
            relative_clauses,
        }) => vec![
            bityzba::new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                base_sumti
            )),
            bityzba::new!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
            ),
        ],
        sumti => vec![bityzba::new!(
            DescriptionTailElementSyntax::DescriptionTailSumti(Box::new(SumtiSyntax::from_data(
                sumti
            )))
        )],
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn apply_bound_sumti_tail(
    leading_sumti: Box<SumtiSyntax>,
    bound_tail: Option<BoundSumtiTailSyntax>,
) -> SumtiSyntax {
    let Some(BoundSumtiTailSyntax {
        connective,
        tense_modal,
        bo,
        trailing_sumti,
    }) = bound_tail
    else {
        return *leading_sumti;
    };
    bityzba::new!(SumtiSyntax::BoundSumtiConnection {
        leading_sumti,
        bo_connective: Some(connective),
        bo_tense_modal: tense_modal,
        bo,
        trailing_sumti,
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn apply_afterthought_sumti_tails(
    leading_sumti: Box<SumtiSyntax>,
    continuations: Vec<SumtiConnectionSyntax>,
) -> SumtiSyntax {
    continuations
        .into_iter()
        .fold(*leading_sumti, |leading_sumti, continuation| {
            let SumtiConnectionSyntax { connective, sumti } = continuation;
            bityzba::new!(SumtiSyntax::SumtiConnection {
                leading_sumti: Box::new(leading_sumti),
                connective,
                trailing_sumti: sumti,
            })
        })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn apply_grouped_sumti_tail(
    leading_sumti: Box<SumtiSyntax>,
    grouped_tail: Option<GroupedSumtiTailSyntax>,
) -> SumtiSyntax {
    let Some(GroupedSumtiTailSyntax {
        connective,
        tense_modal,
        ke,
        inner_sumti,
        kehe,
    }) = grouped_tail
    else {
        return *leading_sumti;
    };
    let connective = match tense_modal {
        Some(tense_modal) => append_tense_modal_words_to_connective(connective, *tense_modal),
        None => connective,
    };
    bityzba::new!(SumtiSyntax::SumtiConnection {
        leading_sumti,
        connective,
        trailing_sumti: Box::new(bityzba::new!(SumtiSyntax::GroupedSumti {
            ke,
            inner_sumti,
            kehe,
        })),
    })
}

#[bityzba::requires(
    vuho_attachment.as_ref().is_none_or(|attachment| {
        !attachment.relative_clauses.is_empty() || attachment.sumti_connection.is_some()
    })
)]
#[bityzba::ensures(true)]
fn apply_vuho_sumti_attachment(
    base_sumti: Box<SumtiSyntax>,
    vuho_attachment: Option<VuhoSumtiAttachmentSyntax>,
) -> SumtiSyntax {
    let Some(VuhoSumtiAttachmentSyntax {
        vuho,
        relative_clauses,
        sumti_connection,
    }) = vuho_attachment
    else {
        return *base_sumti;
    };
    if !relative_clauses.is_empty() && sumti_connection.is_none() {
        bityzba::new!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti,
            vuho: Some(vuho),
            relative_clauses,
        })
    } else {
        bityzba::new!(SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti,
            vuho_marker: vuho,
            relative_clauses,
            sumti_connection,
        })
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn optional_relative_clause_list(
    relative_clauses: Option<(RelativeClauseSyntax, Vec<RelativeClauseSyntax>)>,
) -> Vec<RelativeClauseSyntax> {
    relative_clauses
        .map(|(first_relative_clause, additional_relative_clauses)| {
            std::iter::once(first_relative_clause)
                .chain(additional_relative_clauses)
                .collect()
        })
        .unwrap_or_default()
}

#[bityzba::requires(true)]
#[bityzba::ensures(ret.len() == old(terms.len()))]
fn unbox_terms(terms: Vec<Box<TermSyntax>>) -> Vec<TermSyntax> {
    terms.into_iter().map(|term| *term).collect()
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn append_optional_tense_modal_and_bo_to_connective(
    connective: ConnectiveSyntax,
    tense_modal: Option<Box<TenseModalSyntax>>,
    bo: WithFreeModifiers<Token>,
) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    if let Some(tense_modal) = tense_modal {
        tense_modal.extend_words_into(&mut cmavo.value);
    }
    cmavo.value.push(bo.value);
    cmavo.free_modifiers.extend(bo.free_modifiers);
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[bityzba::requires(true)]
#[bityzba::ensures(ret.cmavo().value.len() >= old(words.len()))]
fn prepend_connective_words(words: Vec<Token>, connective: ConnectiveSyntax) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    let mut value = words;
    value.extend(cmavo.value);
    cmavo.value = value;
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn append_connective_words(connective: ConnectiveSyntax, words: Vec<Token>) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    cmavo.value.extend(words);
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn append_tense_modal_words_to_connective(
    connective: ConnectiveSyntax,
    tense_modal: TenseModalSyntax,
) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    tense_modal.extend_words_into(&mut cmavo.value);
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn selbri_to_empty_bridi(selbri: SelbriSyntax) -> BridiSyntax {
    bityzba::new!(BridiSyntax {
        leading_terms: Vec::new(),
        cu: None,
        bridi_tail: Box::new(BridiTailSyntax {
            first: Box::new(AfterthoughtBridiTailSyntax {
                first: Box::new(BoGroupedBridiTailSyntax {
                    first: Box::new(bityzba::new!(SimpleBridiTailSyntax::SelbriBridiTail {
                        selbri: Box::new(selbri),
                        terms: Vec::new(),
                        vau: None,
                        free_modifiers: Vec::new(),
                    })),
                    bo_continuation: None,
                }),
                continuations: Vec::new(),
            }),
            ke_continuation: None,
        }),
        free_modifiers: Vec::new(),
    })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn tanru_unit_to_single_selbri(unit: TanruUnitSyntax) -> SelbriSyntax {
    match unit.into_data() {
        bityzba::data!(TanruUnitSyntax::TanruUnitWord(word)) if word.free_modifiers.is_empty() => {
            bityzba::new!(SelbriSyntax::SelbriWord(word.value))
        }
        bityzba::data!(TanruUnitSyntax::ProBridi { goha, raho: None })
            if goha.free_modifiers.is_empty() =>
        {
            bityzba::new!(SelbriSyntax::SelbriWord(goha.value))
        }
        bityzba::data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
            bityzba::new!(SelbriSyntax::ConvertedSelbri {
                se,
                inner_selbri: Box::new(tanru_unit_to_single_selbri(*inner_unit)),
            })
        }
        bityzba::data!(TanruUnitSyntax::GroupedTanruUnit {
            ke_tense_modal,
            ke,
            selbri,
            kehe,
        }) => bityzba::new!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            ke,
            selbri,
            kehe,
        }),
        bityzba::data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        }) => bityzba::new!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri: Box::new(tanru_unit_to_single_selbri(*leading_unit)),
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_selbri: Box::new(tanru_unit_to_single_selbri(*trailing_unit)),
        }),
        bityzba::data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            connective,
            trailing_unit,
        }) => bityzba::new!(SelbriSyntax::SelbriConnection {
            leading_selbri: Box::new(tanru_unit_to_single_selbri(*leading_unit)),
            connective,
            trailing_selbri: Box::new(tanru_unit_to_single_selbri(*trailing_unit)),
        }),
        bityzba::data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => *selbri,
        bityzba::data!(TanruUnitSyntax::Abstraction(abstraction)) => {
            bityzba::new!(SelbriSyntax::Abstraction(abstraction))
        }
        data => {
            let unit = TanruUnitSyntax::from_data(data);
            bityzba::new!(SelbriSyntax::Tanru(Box::new(TanruUnitVec::new(unit))))
        }
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
pub(super) fn parse_statement_attempt(
    words: &[Token],
    _source: Option<&str>,
    options: &ParseOptions,
) -> ParsedStatementAttempt {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
    let mut state = ParserState::new(words, options);
    let result = strict_generated_text_parser()
        .then_ignore(end())
        .parse_with_state(
            tokens
                .as_slice()
                .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
            &mut state,
        )
        .into_result();

    match result {
        Ok(text) => {
            let finished = state.finish();
            ParsedStatementAttempt {
                result: Ok(ParsedStatement {
                    text,
                    warnings: finished.warnings,
                }),
                trace: finished.trace,
            }
        }
        Err(errors) => {
            if state.trace_enabled()
                && let Some(summary) = syntax_trace_failure_summary(&errors)
            {
                state.trace_failure_summary(summary);
            }
            let error = syntax_error(errors);
            let finished = state.finish();
            ParsedStatementAttempt {
                result: Err(error),
                trace: finished.trace,
            }
        }
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
pub(super) fn parse_statement_attempt_partial_valid(
    words: &[Token],
    _source: Option<&str>,
    options: &ParseOptions,
) -> ParsedPartialValidStatementAttempt {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
    let mut state = ParserState::new(words, options);
    let result = partial_valid_generated_text_parser()
        .then_ignore(end())
        .parse_with_state(
            tokens
                .as_slice()
                .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
            &mut state,
        )
        .into_result();

    match result {
        Ok(text) => {
            let finished = state.finish();
            ParsedPartialValidStatementAttempt {
                result: Ok(text),
                warnings: finished.warnings,
                trace: finished.trace,
            }
        }
        Err(errors) => {
            if state.trace_enabled()
                && let Some(summary) = syntax_trace_failure_summary(&errors)
            {
                state.trace_failure_summary(summary);
            }
            let error = syntax_error(errors);
            let finished = state.finish();
            ParsedPartialValidStatementAttempt {
                result: Err(error),
                warnings: finished.warnings,
                trace: finished.trace,
            }
        }
    }
}
