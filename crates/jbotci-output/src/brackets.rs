use bityzba::data;
#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::{invariant, requires};
use jbotci_morphology::{Cmavo, Phonemes, Word, WordLike, WordLikeData};
use jbotci_syntax::ast::*;
use jbotci_syntax::{Indicator, Token, WithIndicators};
use jbotci_tree::TreeVisitor;

use crate::{
    BracketRenderOptions, BracketSourceFragment, BracketSourceRange, OutputError, sexpr, surface,
};

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct BracketContext<'source> {
    source: &'source str,
    options: BracketRenderOptions,
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_brackets_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let context = BracketContext { source, options };
    let sexpr = text(tree, &context);
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty()))]
pub(crate) fn pretty_bracket_source_fragments_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<BracketSourceFragment>, OutputError> {
    let context = BracketContext { source, options };
    let sexpr = text(tree, &context);
    Ok(sexpr::render_bracketed_source_fragments_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_morphology_brackets_with_options(
    words: &[WordLike],
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let context = BracketContext { source, options };
    let sexpr = sexpr::node(
        words
            .iter()
            .map(|word_like| word_like_brackets(word_like, &context))
            .collect(),
    );
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(true)]
fn text(tree: &TextSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = Vec::new();
    if !tree.leading_nai.is_empty() {
        children.push(list_node(words(&tree.leading_nai, source)));
    }
    if !tree.leading_cmevla.is_empty() {
        children.push(list_node(words(&tree.leading_cmevla, source)));
    }
    if !tree.leading_indicators.is_empty() {
        children.push(indicators(&tree.leading_indicators, source));
    }
    children.extend(
        tree.leading_free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    if let Some(connective) = &tree.leading_connective {
        children.push(connective_syntax(connective, source));
    }
    children.extend(tree.paragraphs.iter().map(|item| paragraph(item, source)));
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn paragraph(value: &ParagraphSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(i) = &value.i {
        children.push(word(i, source));
    }
    children.extend(words(&value.niho, source));
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    children.extend(
        value
            .statements
            .iter()
            .map(|item| paragraph_statement(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn paragraph_statement(
    value: &ParagraphStatementSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(i) = &value.i {
        children.push(word(i, source));
    }
    if let Some(connective) = &value.connective {
        children.push(connective_syntax(connective, source));
    }
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    if let Some(statement) = &value.statement {
        children.push(statement_syntax(statement, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn statement_syntax(value: &StatementSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(StatementSyntax::TextGroup {
            tense_modal,
            tuhe,
            text: inner,
            tuhu,
        }) => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(tuhe, source));
            children.push(text(inner, source));
            push_optional_elidable(
                &mut children,
                tuhu.as_ref(),
                Cmavo::Tuhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_statement,
        }) => prenex(
            prenex_terms.iter().map(|item| term(item, source)).collect(),
            with_free_word(zohu, source),
            statement_syntax(inner_statement, source),
        ),
        data!(StatementSyntax::Bridi(bridi)) => predicate_syntax(bridi, source),
        data!(StatementSyntax::StatementConnection {
            i,
            connective,
            leading_statement,
            trailing_statement,
        }) => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            word(i, source),
            connective_syntax(connective, source),
            statement_syntax(trailing_statement, source),
        ]),
        data!(StatementSyntax::PreposedIStatementConnection {
            connective,
            i,
            leading_statement,
            trailing_statement,
        }) => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            connective_syntax(connective, source),
            word(i, source),
            statement_syntax(trailing_statement, source),
        ]),
        data!(StatementSyntax::Iau {
            inner_statement,
            iau,
            reset_terms,
        }) => {
            let mut children = vec![
                statement_syntax(inner_statement, source),
                with_free_word(iau, source),
            ];
            children.extend(reset_terms.iter().map(|item| term(item, source)));
            sexpr::node(children)
        }
        data!(StatementSyntax::ExperimentalBridiContinuation {
            leading_statement,
            continuation,
        }) => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            source_words_node(continuation, source),
        ]),
        data!(StatementSyntax::Fragment(fragment)) => fragment_syntax(fragment, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_syntax(value: &BridiSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![list_node(
        value
            .leading_terms
            .iter()
            .map(|item| term(item, source))
            .collect(),
    )];
    if let Some(cu) = &value.cu {
        children.push(with_free_word(cu, source));
    }
    children.push(bridi_tail(&value.bridi_tail, source));
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn bridi_tail(value: &BridiTailSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![predicate_tail1(&value.first, source)];
    if let Some(continuation) = &value.ke_continuation {
        children.push(ke_predicate_tail(continuation, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail1(
    value: &AfterthoughtBridiTailSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![predicate_tail2(&value.first, source)];
    if !value.continuations.is_empty() {
        children.push(list_node(
            value
                .continuations
                .iter()
                .map(|item| predicate_tail_continuation(item, source))
                .collect(),
        ));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2(value: &BoGroupedBridiTailSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![predicate_tail3(&value.first, source)];
    if let Some(continuation) = &value.bo_continuation {
        children.push(bo_predicate_tail(continuation, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail3(value: &SimpleBridiTailSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail {
            selbri,
            terms,
            vau,
            free_modifiers,
        }) => {
            let mut children = vec![relation_syntax(selbri, source)];
            children.push(list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            ));
            push_optional_elidable(
                &mut children,
                vau.as_deref(),
                Cmavo::Vau,
                source,
                with_free_word,
            );
            children.push(list_node(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source))
                    .collect(),
            ));
            sexpr::node(children)
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(gek)) => {
            forethought_connection(gek, source)
        }
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail {
            terms,
            bridi_tail: inner_tail,
        }) => sexpr::node(vec![
            list_node(terms.iter().map(|item| term(item, source)).collect()),
            bridi_tail(inner_tail, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn ke_predicate_tail(
    value: &GroupedBridiTailConnectionSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    children.push(with_free_word(&value.ke, source));
    children.push(bridi_tail(&value.bridi_tail, source));
    push_optional_elidable(
        &mut children,
        value.kehe.as_deref(),
        Cmavo::Kehe,
        source,
        with_free_word,
    );
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    push_optional_elidable(
        &mut children,
        value.vau.as_deref(),
        Cmavo::Vau,
        source,
        with_free_word,
    );
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_continuation(
    value: &BridiTailConnectionSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    if let Some(cu) = &value.cu {
        children.push(with_free_word(cu, source));
    }
    children.push(predicate_tail2(&value.bridi_tail, source));
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    push_optional_elidable(
        &mut children,
        value.vau.as_deref(),
        Cmavo::Vau,
        source,
        with_free_word,
    );
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn bo_predicate_tail(
    value: &BoundBridiTailConnectionSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    children.push(with_free_word(&value.bo, source));
    if let Some(cu) = &value.cu {
        children.push(with_free_word(cu, source));
    }
    children.push(predicate_tail2(&value.bridi_tail, source));
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    push_optional_elidable(
        &mut children,
        value.vau.as_deref(),
        Cmavo::Vau,
        source,
        with_free_word,
    );
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn forethought_connection(
    value: &ForethoughtBridiConnectionSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    match value.as_data() {
        data!(ForethoughtBridiConnectionSyntax::BridiConnection {
            gek,
            first,
            gik,
            second,
            gihi,
            tail_terms,
            vau,
            free_modifiers,
        }) => {
            let mut children = vec![
                connective_syntax(gek, source),
                subbridi(first, source),
                connective_syntax(gik, source),
                subbridi(second, source),
            ];
            push_optional_elidable(&mut children, gihi.as_ref(), Cmavo::Gihi, source, word);
            children.push(list_node(
                tail_terms.iter().map(|item| term(item, source)).collect(),
            ));
            push_optional_elidable(
                &mut children,
                vau.as_deref(),
                Cmavo::Vau,
                source,
                with_free_word,
            );
            children.extend(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            sexpr::node(children)
        }
        data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
            tense_modal,
            ke,
            inner,
            kehe,
        }) => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(forethought_connection(inner, source));
            push_optional_elidable(
                &mut children,
                kehe.as_deref(),
                Cmavo::Kehe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { na, inner }) => {
            sexpr::node(vec![
                with_free_word(na, source),
                forethought_connection(inner, source),
            ])
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn subbridi(value: &SubbridiSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(SubbridiSyntax::Bridi(bridi)) => predicate_syntax(bridi, source),
        data!(SubbridiSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_subbridi,
        }) => sexpr::node(vec![
            sexpr::node(vec![
                list_node(prenex_terms.iter().map(|item| term(item, source)).collect()),
                with_free_word(zohu, source),
            ]),
            subbridi(inner_subbridi, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn fragment_syntax(value: &FragmentSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(FragmentSyntax::Ek(connective))
        | data!(FragmentSyntax::BridiTailConnective(connective)) => {
            connective_syntax(connective, source)
        }
        data!(FragmentSyntax::Other(words)) => with_free_words(words, source),
        data!(FragmentSyntax::BridiConnective { i, connective }) => {
            sexpr::node(vec![word(i, source), connective_syntax(connective, source)])
        }
        data!(FragmentSyntax::Prenex { terms, zohu }) => {
            let header = terms
                .iter()
                .map(|item| term(item, source))
                .collect::<Vec<_>>();
            sexpr::node(vec![sexpr::node(vec![
                sexpr::node(header),
                with_free_word(zohu, source),
            ])])
        }
        data!(FragmentSyntax::LinkedSumti {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) => {
            let mut children = vec![with_free_word(be, source)];
            if let Some(fa) = fa {
                children.push(with_free_word(fa, source));
            }
            if let Some(sumti) = first_sumti {
                children.push(argument_syntax(sumti, source));
            }
            children.push(list_node(
                bei_links
                    .iter()
                    .map(|item| bei_link(item, source))
                    .collect(),
            ));
            push_optional_elidable(
                &mut children,
                beho.as_ref(),
                Cmavo::Beho,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FragmentSyntax::LinkedSumtiContinuation(links)) => {
            list_node(links.iter().map(|item| bei_link(item, source)).collect())
        }
        data!(FragmentSyntax::RelativeClauses(relative_clauses)) => list_node(
            relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect(),
        ),
        data!(FragmentSyntax::Mekso(expression)) => mekso(expression, source),
        data!(FragmentSyntax::Terms { terms, vau }) => {
            let mut children = vec![list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            )];
            push_optional_elidable(
                &mut children,
                vau.as_ref(),
                Cmavo::Vau,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FragmentSyntax::Selbri(selbri)) => relation_syntax(selbri, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn term(value: &TermSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(TermSyntax::Sumti(sumti)) => argument_syntax(sumti, source),
        data!(TermSyntax::TaggedSumti { tense_modal, sumti }) => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(argument_syntax(sumti, source));
            sexpr::node(children)
        }
        data!(TermSyntax::JaiTaggedSumti { jai, tag, sumti }) => {
            let mut children = vec![with_free_word(jai, source)];
            if let Some(tag) = tag {
                children.push(tense_modal_syntax(tag, source));
            }
            children.push(argument_syntax(sumti, source));
            sexpr::node(children)
        }
        data!(TermSyntax::PlaceTaggedSumti { fa, sumti, ku }) => {
            let mut children = vec![with_free_word(fa, source), argument_syntax(sumti, source)];
            push_optional_elidable(
                &mut children,
                ku.as_ref(),
                Cmavo::Ku,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TermSyntax::BridiNegation { na, na_ku }) => {
            sexpr::node(vec![word(na, source), with_free_word(na_ku, source)])
        }
        data!(TermSyntax::BareNegation(na)) => with_free_word(na, source),
        data!(TermSyntax::Termset {
            nuhi,
            termset,
            nuhu,
        }) => {
            let mut children = vec![
                with_free_word(nuhi, source),
                list_node(termset.iter().map(|item| term(item, source)).collect()),
            ];
            push_optional_elidable(
                &mut children,
                nuhu.as_ref(),
                Cmavo::Nuhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TermSyntax::ForethoughtTermsetConnection {
            m_nuhi,
            gek,
            terms,
            nuhu,
            gik,
            gik_terms,
            gihi,
            gik_nuhu,
        }) => {
            let mut children = Vec::new();
            if let Some(nuhi) = m_nuhi {
                children.push(with_free_word(nuhi, source));
            }
            children.push(connective_syntax(gek, source));
            children.push(list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            ));
            push_optional_elidable(
                &mut children,
                nuhu.as_ref(),
                Cmavo::Nuhu,
                source,
                with_free_word,
            );
            children.push(connective_syntax(gik, source));
            children.push(list_node(
                gik_terms.iter().map(|item| term(item, source)).collect(),
            ));
            push_optional_elidable(&mut children, gihi.as_ref(), Cmavo::Gihi, source, word);
            push_optional_elidable(
                &mut children,
                gik_nuhu.as_ref(),
                Cmavo::Nuhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TermSyntax::TermsetGroup {
            leading_terms,
            cehe,
            trailing_terms,
        }) => {
            let mut children = vec![list_node(
                leading_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            )];
            children.push(with_free_word(cehe, source));
            children.push(list_node(
                trailing_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            ));
            sexpr::node(children)
        }
        data!(TermSyntax::TermsetConnection {
            leading_terms,
            pehe,
            connective,
            trailing_terms,
        }) => {
            let mut children = vec![list_node(
                leading_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            )];
            children.push(with_free_word(pehe, source));
            children.push(connective_syntax(connective, source));
            children.push(list_node(
                trailing_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            ));
            sexpr::node(children)
        }
        data!(TermSyntax::TermConnection {
            leading_terms,
            connective,
            trailing_terms,
        }) => {
            let mut children = vec![list_node(
                leading_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            )];
            children.push(connective_syntax(connective, source));
            children.push(list_node(
                trailing_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            ));
            sexpr::node(children)
        }
        data!(TermSyntax::BoundTermConnection {
            leading_terms,
            bo_connective,
            tense_modal,
            bo,
            trailing_term,
        }) => {
            let mut children = vec![list_node(
                leading_terms
                    .iter()
                    .map(|item| term(item, source))
                    .collect(),
            )];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(term(trailing_term, source));
            sexpr::node(children)
        }
        data!(TermSyntax::RelativeAdverbialTerm {
            noiha,
            tail_elements,
            selbri,
            relative_clauses,
            fehu,
        }) => {
            let mut children = vec![with_free_word(noiha, source)];
            children.push(list_node(
                tail_elements
                    .iter()
                    .map(|item| argument_tail_element(item, source))
                    .collect(),
            ));
            if let Some(selbri) = selbri {
                children.push(relation_syntax(selbri, source));
            }
            children.push(list_node(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source))
                    .collect(),
            ));
            push_optional_elidable(
                &mut children,
                fehu.as_ref(),
                Cmavo::Fehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TermSyntax::BridiVariableAdverbialTerm {
            poiha,
            tail_elements,
            selbri,
            relative_clauses,
            brigahi_ku,
        }) => {
            let mut children = vec![with_free_word(poiha, source)];
            children.push(list_node(
                tail_elements
                    .iter()
                    .map(|item| argument_tail_element(item, source))
                    .collect(),
            ));
            if let Some(selbri) = selbri {
                children.push(relation_syntax(selbri, source));
            }
            children.push(list_node(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source))
                    .collect(),
            ));
            children.push(with_free_word(brigahi_ku, source));
            sexpr::node(children)
        }
        data!(TermSyntax::AdHocBridiAdverbialTerm {
            fihoi,
            subbridi: inner,
            fihau,
        }) => {
            let mut children = vec![with_free_word(fihoi, source), subbridi(inner, source)];
            push_optional_elidable(
                &mut children,
                fihau.as_ref(),
                Cmavo::Fihau,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TermSyntax::ReciprocalBridiAdverbialTerm {
            soi,
            subbridi: inner,
            sehu,
        }) => {
            let mut children = vec![with_free_word(soi, source), subbridi(inner, source)];
            push_optional_elidable(
                &mut children,
                sehu.as_ref(),
                Cmavo::Sehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_syntax(value: &SumtiSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(SumtiSyntax::QuotedSumti(quote)) => quote_syntax(quote, source),
        data!(SumtiSyntax::NumberSumti {
            li,
            expression,
            loho,
        }) => {
            let mut children = vec![with_free_word(li, source), mekso(expression, source)];
            push_optional_elidable(
                &mut children,
                loho.as_ref(),
                Cmavo::Loho,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::LerfuStringSumti { letter, boi }) => {
            let mut children = vec![with_free_words(letter, source)];
            push_optional_elidable(
                &mut children,
                boi.as_ref(),
                Cmavo::Boi,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::QuantifiedSumti {
            quantifier,
            inner_sumti,
        }) => sexpr::node(vec![
            quantifier_syntax(quantifier, source),
            argument_syntax(inner_sumti, source),
        ]),
        data!(SumtiSyntax::SumtiConnection {
            leading_sumti,
            connective,
            trailing_sumti,
        }) => sexpr::node(vec![
            argument_syntax(leading_sumti, source),
            connective_syntax(connective, source),
            argument_syntax(trailing_sumti, source),
        ]),
        data!(SumtiSyntax::Description(description)) => descriptor_syntax(description, source),
        data!(SumtiSyntax::DescriptionConnection(description)) => {
            description_connection(description, source)
        }
        data!(SumtiSyntax::NameDescription { la, names }) => sexpr::node(vec![
            with_free_word(la, source),
            with_free_words(names, source),
        ]),
        data!(SumtiSyntax::NameWords(words)) => with_free_words(words, source),
        data!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti,
            vuho,
            relative_clauses,
        }) => {
            let mut children = vec![argument_syntax(base_sumti, source)];
            if let Some(vuho) = vuho {
                children.push(with_free_word(vuho, source));
            }
            children.extend(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti,
            vuho_marker,
            relative_clauses,
            sumti_connection,
        }) => {
            let mut children = vec![
                argument_syntax(base_sumti, source),
                with_free_word(vuho_marker, source),
            ];
            children.extend(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            if let Some(connection) = sumti_connection {
                children.push(sexpr::node(vec![
                    connective_syntax(&connection.connective, source),
                    argument_syntax(&connection.sumti, source),
                ]));
            }
            sexpr::node(children)
        }
        data!(SumtiSyntax::BridiDescription {
            lohoi,
            subbridi: inner,
            kuhau,
        }) => {
            let mut children = vec![with_free_word(lohoi, source), subbridi(inner, source)];
            push_optional_elidable(
                &mut children,
                kuhau.as_ref(),
                Cmavo::Kuhau,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::NegatedSumti { na, ku }) => {
            sexpr::node(vec![word(na, source), with_free_word(ku, source)])
        }
        data!(SumtiSyntax::TaggedSumti { tag, inner_sumti }) => sexpr::node(vec![
            argument_tag(tag, source),
            argument_syntax(inner_sumti, source),
        ]),
        data!(SumtiSyntax::ScalarNegatedSumtiWithBo {
            nahe,
            bo,
            inner_sumti,
            luhu,
        }) => {
            let mut children = vec![
                word(nahe, source),
                with_free_word(bo, source),
                argument_syntax(inner_sumti, source),
            ];
            push_optional_elidable(
                &mut children,
                luhu.as_ref(),
                Cmavo::Luhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::ScalarNegatedSumti {
            nahe,
            inner_sumti,
            luhu,
        }) => {
            let mut children = vec![
                with_free_word(nahe, source),
                argument_syntax(inner_sumti, source),
            ];
            push_optional_elidable(
                &mut children,
                luhu.as_ref(),
                Cmavo::Luhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::QualifiedTerm {
            wrapper,
            wrapper_bo,
            inner_term,
            luhu,
            ..
        }) => {
            let mut children = vec![with_free_word(wrapper, source)];
            if let Some(wrapper_bo) = wrapper_bo {
                children.push(with_free_word(wrapper_bo, source));
            }
            children.push(term(inner_term, source));
            push_optional_elidable(
                &mut children,
                luhu.as_ref(),
                Cmavo::Luhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::ProSumti(koha)) => with_free_word(koha, source),
        data!(SumtiSyntax::ElidedSumti {
            tag,
            maybe_ku,
            free_modifiers,
        }) => {
            let mut children = Vec::new();
            if let Some(tag) = tag {
                children.push(argument_tag(tag, source));
            }
            push_optional_elidable(
                &mut children,
                maybe_ku.as_ref(),
                Cmavo::Ku,
                source,
                with_free_word,
            );
            children.extend(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::ReferentSumti {
            lahe,
            relative_clauses,
            inner_sumti,
            luhu,
        }) => {
            let mut children = vec![with_free_word(lahe, source)];
            children.extend(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            children.push(argument_syntax(inner_sumti, source));
            push_optional_elidable(
                &mut children,
                luhu.as_ref(),
                Cmavo::Luhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::GroupedSumti {
            ke,
            inner_sumti,
            kehe,
        }) => {
            let mut children = vec![
                with_free_word(ke, source),
                argument_syntax(inner_sumti, source),
            ];
            push_optional_elidable(
                &mut children,
                kehe.as_ref(),
                Cmavo::Kehe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SumtiSyntax::BoundSumtiConnection {
            leading_sumti,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_sumti,
        }) => {
            let mut children = vec![argument_syntax(leading_sumti, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(argument_syntax(trailing_sumti, source));
            sexpr::node(children)
        }
        data!(SumtiSyntax::ForethoughtSumtiConnection {
            gek,
            leading_sumti,
            gik,
            trailing_sumti,
            gihi,
        }) => sexpr::node(
            vec![
                connective_syntax(gek, source),
                argument_syntax(leading_sumti, source),
                connective_syntax(gik, source),
                argument_syntax(trailing_sumti, source),
            ]
            .into_iter()
            .chain(gihi.iter().map(|gihi| word(gihi, source)))
            .collect(),
        ),
        data!(SumtiSyntax::SelbriVocative {
            leading_relative_clauses,
            selbri,
            trailing_relative_clauses,
        }) => {
            let mut children = leading_relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect::<Vec<_>>();
            children.push(relation_syntax(selbri, source));
            children.extend(
                trailing_relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_tag(value: &SumtiTagSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(SumtiTagSyntax::TenseModal(tense_modal)) => tense_modal_syntax(tense_modal, source),
        data!(SumtiTagSyntax::PlaceTag(fa)) => with_free_word(fa, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn descriptor_syntax(value: &DescriptionSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(outer_quantifier) = &value.outer_quantifier {
        children.push(quantifier_syntax(outer_quantifier, source));
    }
    if let Some(description) = &value.description {
        children.push(with_free_word(description, source));
    }
    if value.tail_elements.len() > 1 {
        children.push(list_node(
            value
                .tail_elements
                .iter()
                .map(|item| argument_tail_element(item, source))
                .collect(),
        ));
    } else {
        children.extend(
            value
                .tail_elements
                .iter()
                .map(|item| argument_tail_element(item, source)),
        );
    }
    if let Some(selbri) = &value.selbri {
        children.push(relation_syntax(selbri, source));
    }
    if !value.relative_clauses.is_empty() {
        children.push(list_node(
            value
                .relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect(),
        ));
    }
    push_optional_elidable(
        &mut children,
        value.ku.as_ref(),
        Cmavo::Ku,
        source,
        with_free_word,
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn description_connection(
    value: &DescriptionConnectionSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![
        descriptor_head(&value.leading_description_head, source),
        connective_syntax(&value.connective, source),
        descriptor_head(&value.trailing_description_head, source),
    ];
    children.extend(
        value
            .tail_elements
            .iter()
            .map(|item| argument_tail_element(item, source)),
    );
    if let Some(selbri) = &value.selbri {
        children.push(relation_syntax(selbri, source));
    }
    if !value.relative_clauses.is_empty() {
        children.push(list_node(
            value
                .relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect(),
        ));
    }
    push_optional_elidable(
        &mut children,
        value.ku.as_ref(),
        Cmavo::Ku,
        source,
        with_free_word,
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn descriptor_head(value: &DescriptionHeadSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    with_free_word(&value.description, source)
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_element(
    value: &DescriptionTailElementSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    match value.as_data() {
        data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => {
            argument_syntax(sumti, source)
        }
        data!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)) => {
            sexpr::node(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source))
                    .collect(),
            )
        }
        data!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
            quantifier
        )) => quantifier_syntax(quantifier, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clause(value: &RelativeClauseSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(RelativeClauseSyntax::SumtiAssociationPhrase(goi)) => {
            let mut children = vec![
                with_free_word(&goi.association_marker, source),
                argument_syntax(&goi.sumti, source),
            ];
            push_optional_elidable(
                &mut children,
                goi.gehu.as_ref(),
                Cmavo::Gehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(RelativeClauseSyntax::IncidentalRelativeBridi {
            noi,
            subbridi: inner,
            kuho,
        })
        | data!(RelativeClauseSyntax::RestrictiveRelativeBridi {
            poi: noi,
            subbridi: inner,
            kuho,
        }) => {
            let mut children = vec![with_free_word(noi, source), subbridi(inner, source)];
            push_optional_elidable(
                &mut children,
                kuho.as_ref(),
                Cmavo::Kuho,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(RelativeClauseSyntax::JoinedRelativeClauses { zihe, inner }) => sexpr::node(vec![
            with_free_word(zihe, source),
            relative_clause(inner, source),
        ]),
        data!(RelativeClauseSyntax::RelativeClauseConnection { connective, inner }) => {
            sexpr::node(vec![
                connective_syntax(connective, source),
                relative_clause(inner, source),
            ])
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_syntax(value: &QuoteSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(QuoteSyntax::TextQuote { lu, text, lihu }) => {
            let mut children = vec![word(&lu.value, source)];
            children.extend(
                lu.free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            children.push(self::text(text, source));
            push_optional_elidable(
                &mut children,
                lihu.as_ref(),
                Cmavo::Lihu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(QuoteSyntax::WordQuote(zo))
        | data!(QuoteSyntax::DelimitedWordQuote(zo))
        | data!(QuoteSyntax::DelimitedNonLojbanQuote(zo))
        | data!(QuoteSyntax::WordsQuote(zo)) => with_free_word(zo, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_syntax(value: &QuantifierSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(QuantifierSyntax::NumberQuantifier { number, boi }) => {
            let mut children = vec![with_free_words(number, source)];
            push_optional_elidable(
                &mut children,
                boi.as_ref(),
                Cmavo::Boi,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(QuantifierSyntax::MeksoQuantifier { vei, mekso, veho }) => {
            let mut children = vec![with_free_word(vei, source), self::mekso(mekso, source)];
            push_optional_elidable(
                &mut children,
                veho.as_ref(),
                Cmavo::Veho,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn mekso(value: &MeksoSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(MeksoSyntax::NumberMekso(number)) => quantifier_syntax(number, source),
        data!(MeksoSyntax::LerfuStringMekso { letter, boi }) => {
            let mut children = vec![with_free_words(letter, source)];
            push_optional_elidable(
                &mut children,
                boi.as_ref(),
                Cmavo::Boi,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::Infix {
            operator,
            left_expression,
            right_expression,
        }) => sexpr::node(vec![
            self::mekso(left_expression, source),
            mekso_operator(operator, source),
            self::mekso(right_expression, source),
        ]),
        data!(MeksoSyntax::MeksoConnection {
            left_expression,
            connective,
            right_expression,
        }) => sexpr::node(vec![
            self::mekso(left_expression, source),
            connective_syntax(connective, source),
            self::mekso(right_expression, source),
        ]),
        data!(MeksoSyntax::ParenthesizedMekso {
            vei,
            inner_expression,
            veho,
        }) => {
            let mut children = vec![
                with_free_word_no_leading_pause(vei, source),
                self::mekso(inner_expression, source),
            ];
            push_optional_elidable(
                &mut children,
                veho.as_ref(),
                Cmavo::Veho,
                source,
                with_free_word_no_leading_pause,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::ForethoughtMeksoConnection {
            gek,
            left_expression,
            gik,
            right_expression,
        }) => sexpr::node(vec![
            connective_syntax(gek, source),
            self::mekso(left_expression, source),
            connective_syntax(gik, source),
            self::mekso(right_expression, source),
        ]),
        data!(MeksoSyntax::ForethoughtCall {
            peho,
            operator,
            operands,
            kuhe,
        }) => {
            let mut children = Vec::new();
            if let Some(peho) = peho {
                children.push(with_free_word(peho, source));
            }
            children.push(mekso_operator(operator, source));
            children.push(list_node(
                operands
                    .iter()
                    .map(|item| self::mekso(item, source))
                    .collect(),
            ));
            push_optional_elidable(
                &mut children,
                kuhe.as_ref(),
                Cmavo::Kuhe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::ReversePolish {
            fuha,
            operands,
            operators,
        }) => {
            let mut children = vec![with_free_word(fuha, source)];
            children.extend(operands.iter().map(|item| self::mekso(item, source)));
            children.extend(operators.iter().map(|item| mekso_operator(item, source)));
            sexpr::node(children)
        }
        data!(MeksoSyntax::SelbriOperand { nihe, selbri, tehu }) => {
            let mut children = vec![
                with_free_word(nihe, source),
                relation_syntax(selbri, source),
            ];
            push_optional_elidable(
                &mut children,
                tehu.as_ref(),
                Cmavo::Tehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::SumtiOperand { mohe, sumti, tehu }) => {
            let mut children = vec![with_free_word(mohe, source), argument_syntax(sumti, source)];
            push_optional_elidable(
                &mut children,
                tehu.as_ref(),
                Cmavo::Tehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::MeksoArray {
            johi,
            expressions,
            tehu,
        }) => {
            let mut children = vec![with_free_word(johi, source)];
            children.push(list_node(
                expressions
                    .iter()
                    .map(|item| self::mekso(item, source))
                    .collect(),
            ));
            push_optional_elidable(
                &mut children,
                tehu.as_ref(),
                Cmavo::Tehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::QualifiedOperand {
            markers,
            inner_expression,
            luhu,
        }) => {
            let mut children = vec![
                with_free_words(markers, source),
                self::mekso(inner_expression, source),
            ];
            push_optional_elidable(
                &mut children,
                luhu.as_ref(),
                Cmavo::Luhu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoSyntax::PrecedenceInfix {
            left_expression,
            bihe,
            operator,
            right_expression,
        }) => sexpr::node(vec![
            self::mekso(left_expression, source),
            with_free_word_no_leading_pause(bihe, source),
            mekso_operator(operator, source),
            self::mekso(right_expression, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn mekso_operator(value: &MeksoOperatorSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(MeksoOperatorSyntax::Primitive(word)) => with_free_word(word, source),
        data!(MeksoOperatorSyntax::OperatorConnection {
            left_operator,
            connective,
            right_operator,
        }) => {
            if connective.kind() == ConnectiveKind::Forethought
                && connective.cmavo().value.len() >= 2
            {
                let mut children = vec![
                    connective_prefix(connective, source),
                    mekso_operator(left_operator, source),
                ];
                if let Some(gi) = connective.cmavo().value.last() {
                    children.push(word_no_leading_pause(gi, source));
                }
                children.push(mekso_operator(right_operator, source));
                sexpr::node(children)
            } else {
                sexpr::node(vec![
                    mekso_operator(left_operator, source),
                    connective_syntax(connective, source),
                    mekso_operator(right_operator, source),
                ])
            }
        }
        data!(MeksoOperatorSyntax::OperandAsOperator { maho, mekso, tehu }) => {
            let mut children = vec![with_free_word(maho, source), self::mekso(mekso, source)];
            push_optional_elidable(
                &mut children,
                tehu.as_ref(),
                Cmavo::Tehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoOperatorSyntax::Converted { se, inner_operator })
        | data!(MeksoOperatorSyntax::ScalarNegated {
            nahe: se,
            inner_operator,
        }) => sexpr::node(vec![
            with_free_word(se, source),
            mekso_operator(inner_operator, source),
        ]),
        data!(MeksoOperatorSyntax::SelbriAsOperator { nahu, selbri, tehu }) => {
            let mut children = vec![
                with_free_word(nahu, source),
                relation_syntax(selbri, source),
            ];
            push_optional_elidable(
                &mut children,
                tehu.as_ref(),
                Cmavo::Tehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoOperatorSyntax::GroupedOperator {
            ke,
            inner_operator,
            kehe,
        }) => {
            let mut children = vec![
                with_free_word(ke, source),
                mekso_operator(inner_operator, source),
            ];
            push_optional_elidable(
                &mut children,
                kehe.as_ref(),
                Cmavo::Kehe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(MeksoOperatorSyntax::BoundOperatorConnection {
            left_operator,
            connective,
            bo,
            right_operator,
        }) => sexpr::node(vec![
            mekso_operator(left_operator, source),
            connective_syntax(connective, source),
            with_free_word(bo, source),
            mekso_operator(right_operator, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_syntax(value: &SelbriSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(SelbriSyntax::SelbriWord(value)) => word(value, source),
        data!(SelbriSyntax::Tanru(units)) => {
            sexpr::node(units.iter().map(|unit| tanru_unit(unit, source)).collect())
        }
        data!(SelbriSyntax::SelbriConnection {
            connective,
            leading_selbri,
            trailing_selbri,
        }) => sexpr::node(vec![
            relation_syntax(leading_selbri, source),
            connective_syntax(connective, source),
            relation_syntax(trailing_selbri, source),
        ]),
        data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            co,
            trailing_selbri,
        }) => sexpr::node(vec![
            relation_syntax(leading_selbri, source),
            with_free_word(co, source),
            relation_syntax(trailing_selbri, source),
        ]),
        data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_selbri,
        }) => {
            let mut children = vec![relation_syntax(leading_selbri, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(relation_syntax(trailing_selbri, source));
            sexpr::node(children)
        }
        data!(SelbriSyntax::Negated { na, inner_selbri }) => sexpr::node(vec![
            with_free_word(na, source),
            relation_syntax(inner_selbri, source),
        ]),
        data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => sexpr::node(vec![
            with_free_word(se, source),
            relation_syntax(inner_selbri, source),
        ]),
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            ke,
            selbri,
            kehe,
        }) => {
            let mut children = Vec::new();
            if let Some(tense_modal) = ke_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(relation_syntax(selbri, source));
            push_optional_elidable(
                &mut children,
                kehe.as_ref(),
                Cmavo::Kehe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => sexpr::node(vec![
            tense_modal_syntax(tense_modal, source),
            relation_syntax(inner_selbri, source),
        ]),
        data!(SelbriSyntax::ForethoughtSelbriConnection {
            guhek,
            leading_bridi,
            gik,
            trailing_bridi,
            gihi,
        }) => sexpr::node(
            vec![
                connective_syntax(guhek, source),
                predicate_syntax(leading_bridi, source),
                connective_syntax(gik, source),
                predicate_syntax(trailing_bridi, source),
            ]
            .into_iter()
            .chain(gihi.iter().map(|gihi| word(gihi, source)))
            .collect(),
        ),
        data!(SelbriSyntax::Abstraction(abstraction)) => abstraction_syntax(abstraction, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit(value: &TanruUnitSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(TanruUnitSyntax::TanruUnitWord(word)) => with_free_word(word, source),
        data!(TanruUnitSyntax::ProBridi { goha, raho }) => {
            let mut children = vec![with_free_word(goha, source)];
            if let Some(raho) = raho {
                children.push(with_free_word(raho, source));
            }
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => sexpr::node(vec![
            with_free_word(se, source),
            tanru_unit(inner_unit, source),
        ]),
        data!(TanruUnitSyntax::GroupedTanruUnit {
            ke_tense_modal,
            ke,
            selbri,
            kehe,
        }) => {
            let mut children = Vec::new();
            if let Some(tense_modal) = ke_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(relation_syntax(selbri, source));
            push_optional_elidable(
                &mut children,
                kehe.as_ref(),
                Cmavo::Kehe,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => sexpr::node(vec![
            with_free_word(nahe, source),
            tanru_unit(inner_unit, source),
        ]),
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        }) => {
            let mut children = vec![tanru_unit(leading_unit, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(tanru_unit(trailing_unit, source));
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            connective,
            trailing_unit,
        }) => sexpr::node(vec![
            tanru_unit(leading_unit, source),
            connective_syntax(connective, source),
            tanru_unit(trailing_unit, source),
        ]),
        data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => relation_syntax(selbri, source),
        data!(TanruUnitSyntax::ModalConversion {
            jai,
            tense_modal,
            inner_unit,
        }) => {
            let mut children = vec![with_free_word(jai, source)];
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(tanru_unit(inner_unit, source));
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base,
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) => be_link_node(
            tanru_unit(base, source),
            be,
            fa.as_ref(),
            first_sumti.as_deref(),
            bei_links,
            beho.as_ref(),
            source,
            false,
        ),
        data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
            base,
        }) => be_link_node(
            tanru_unit(base, source),
            be,
            fa.as_ref(),
            first_sumti.as_deref(),
            bei_links,
            beho.as_ref(),
            source,
            true,
        ),
        data!(TanruUnitSyntax::Abstraction(abstraction)) => abstraction_syntax(abstraction, source),
        data!(TanruUnitSyntax::SumtiSelbri {
            me,
            sumti,
            mehu,
            moi_marker,
        }) => {
            let mut children = vec![with_free_word(me, source), argument_syntax(sumti, source)];
            push_optional_elidable(
                &mut children,
                mehu.as_ref(),
                Cmavo::Mehu,
                source,
                with_free_word,
            );
            if let Some(moi) = moi_marker {
                children.push(with_free_word(moi, source));
            }
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::QuotedWordSelbri(mehoi)) => with_free_word(mehoi, source),
        data!(TanruUnitSyntax::QuotedBridiSelbri(gohoi)) => with_free_word(gohoi, source),
        data!(TanruUnitSyntax::QuotedTextSelbri(muhoi)) => with_free_word(muhoi, source),
        data!(TanruUnitSyntax::TextSelbri { luhei, text, liau }) => {
            let mut children = vec![with_free_word(luhei, source), self::text(text, source)];
            push_optional_elidable(
                &mut children,
                liau.as_ref(),
                Cmavo::Lihau,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::OrdinalSelbri { number, moi }) => {
            let mut children = vec![list_node(words(number, source))];
            children.push(with_free_word(moi, source));
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::OperatorSelbri {
            nuha,
            mekso_operator: operator,
        }) => sexpr::node(vec![
            with_free_word(nuha, source),
            mekso_operator(operator, source),
        ]),
        data!(TanruUnitSyntax::TagSelbri { xohi, tag }) => sexpr::node(vec![
            with_free_word(xohi, source),
            tense_modal_syntax(tag, source),
        ]),
        data!(TanruUnitSyntax::AssignedProBridi { base, assignments }) => {
            let mut children = vec![tanru_unit(base, source)];
            children.extend(assignments.iter().map(|assignment| {
                sexpr::node(vec![
                    with_free_word(&assignment.cei, source),
                    tanru_unit(&assignment.tanru_unit, source),
                ])
            }));
            sexpr::node(children)
        }
        data!(TanruUnitSyntax::RelativeClauses {
            base,
            selbri_relative_clauses,
        }) => {
            let mut children = vec![tanru_unit(base, source)];
            children.extend(selbri_relative_clauses.iter().map(|item| {
                let mut items = vec![
                    with_free_word(&item.nohoi, source),
                    relation_syntax(&item.selbri, source),
                ];
                push_optional_elidable(
                    &mut items,
                    item.kuhoi.as_ref(),
                    Cmavo::Kuhoi,
                    source,
                    with_free_word,
                );
                sexpr::node(items)
            }));
            sexpr::node(children)
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn be_link_node(
    base: sexpr::SExpr,
    be: &WithFreeModifiers<Token>,
    fa: Option<&WithFreeModifiers<Token>>,
    first_sumti: Option<&SumtiSyntax>,
    bei_links: &[AdditionalLinkedSumtiSyntax],
    beho: Option<&WithFreeModifiers<Token>>,
    source: &BracketContext<'_>,
    preposed: bool,
) -> sexpr::SExpr {
    let mut link_children = vec![with_free_word(be, source)];
    if let Some(fa) = fa {
        link_children.push(with_free_word(fa, source));
    }
    if let Some(sumti) = first_sumti {
        link_children.push(argument_syntax(sumti, source));
    }
    link_children.extend(bei_links.iter().map(|item| bei_link(item, source)));
    push_optional_elidable(
        &mut link_children,
        beho,
        Cmavo::Beho,
        source,
        with_free_word,
    );
    if preposed {
        link_children.push(base);
        sexpr::node(link_children)
    } else {
        let mut children = vec![base];
        children.extend(link_children);
        sexpr::node(children)
    }
}

#[requires(true)]
#[ensures(true)]
fn bei_link(value: &AdditionalLinkedSumtiSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![with_free_word(&value.bei, source)];
    if let Some(fa) = &value.fa {
        children.push(with_free_word(fa, source));
    }
    if let Some(sumti) = &value.sumti {
        children.push(argument_syntax(sumti, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn abstraction_syntax(value: &AbstractionSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![with_free_word(&value.nu, source)];
    if let Some(nai) = &value.nai {
        children.push(with_free_word(nai, source));
    }
    children.extend(value.abstractor_connections.iter().map(|item| {
        let mut items = vec![
            connective_syntax(&item.connective, source),
            with_free_word(&item.nu, source),
        ];
        if let Some(nai) = &item.nai {
            items.push(with_free_word(nai, source));
        }
        sexpr::node(items)
    }));
    children.push(subbridi(&value.subbridi, source));
    push_optional_elidable(
        &mut children,
        value.kei.as_ref(),
        Cmavo::Kei,
        source,
        with_free_word,
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_syntax(value: &TenseModalSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(TenseModalSyntax::AdHocModal { fiho, selbri, fehu }) => {
            let mut children = vec![
                with_free_word(fiho, source),
                relation_syntax(selbri, source),
            ];
            push_optional_elidable(
                &mut children,
                fehu.as_ref(),
                Cmavo::Fehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(TenseModalSyntax::Modal {
            nahe,
            se,
            bai,
            nai,
            ki,
        }) => {
            let mut children = Vec::new();
            if let Some(nahe) = nahe {
                children.push(with_free_word(nahe, source));
            }
            if let Some(se) = se {
                children.push(with_free_word(se, source));
            }
            children.push(with_free_word(bai, source));
            if let Some(nai) = nai {
                children.push(with_free_word(nai, source));
            }
            if let Some(ki) = ki {
                children.push(with_free_word(ki, source));
            }
            sexpr::node(children)
        }
        data!(TenseModalSyntax::TimeDirection(word))
        | data!(TenseModalSyntax::TimeInterval(word))
        | data!(TenseModalSyntax::SpaceDistance(word))
        | data!(TenseModalSyntax::SpaceDirection(word))
        | data!(TenseModalSyntax::Sticky(word))
        | data!(TenseModalSyntax::Actuality(word)) => with_free_word(word, source),
        data!(TenseModalSyntax::TimeDirectionDistance { pu, distance }) => {
            sexpr::node(vec![word(pu, source), with_free_word(distance, source)])
        }
        data!(TenseModalSyntax::TimeDirectionActuality { pu, caha }) => {
            sexpr::node(vec![word(pu, source), with_free_word(caha, source)])
        }
        data!(TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
        }) => {
            let mut children = vec![word(mohi, source), with_free_word(direction, source)];
            if let Some(distance) = distance {
                children.push(with_free_word(distance, source));
            }
            sexpr::node(children)
        }
        data!(TenseModalSyntax::EventContour(words)) => with_free_words(words, source),
        data!(TenseModalSyntax::IntervalProperty {
            number,
            roi_or_tahe,
            nai,
        }) => {
            let mut children = number
                .as_ref()
                .map_or_else(Vec::new, |number| words(number, source));
            children.push(with_free_word(roi_or_tahe, source));
            if let Some(nai) = nai {
                children.push(with_free_word(nai, source));
            }
            sexpr::node(children)
        }
        data!(TenseModalSyntax::Composite { parts }) => {
            let mut children = parts
                .value
                .iter()
                .map(|part| composite_tense_modal_part(part, source))
                .collect::<Vec<_>>();
            children.extend(
                parts
                    .free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal_part(
    value: &CompositeTenseModalPartSyntax,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    match value.as_data() {
        data!(CompositeTenseModalPartSyntax::Cmavo(part_word)) => word(part_word, source),
        data!(CompositeTenseModalPartSyntax::AdHocModal(fiho)) => fiho_modal(fiho, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn fiho_modal(value: &AdHocModalSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(nahe) = &value.nahe {
        children.push(word(nahe, source));
    }
    children.push(with_free_word(&value.fiho, source));
    children.push(relation_syntax(&value.selbri, source));
    push_optional_elidable(
        &mut children,
        value.fehu.as_ref(),
        Cmavo::Fehu,
        source,
        with_free_word,
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn connective_syntax(value: &ConnectiveSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let (se, nahe, na, cmavo, nai) = connective_parts(value);
    let mut children = Vec::new();
    if let Some(se) = se {
        children.push(word_no_leading_pause(se, source));
    }
    if let Some(nahe) = nahe {
        children.push(word_no_leading_pause(nahe, source));
    }
    if let Some(na) = na {
        children.push(word_no_leading_pause(na, source));
    }
    children.push(with_free_words(cmavo, source));
    if let Some(nai) = nai {
        children.push(with_free_word_no_leading_pause(nai, source));
    }
    sexpr::node(children)
}

#[requires(value.kind() == ConnectiveKind::Forethought)]
#[ensures(true)]
fn connective_prefix(value: &ConnectiveSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    let (se, nahe, na, cmavo, nai) = connective_parts(value);
    let mut children = Vec::new();
    if let Some(se) = se {
        children.push(word_no_leading_pause(se, source));
    }
    if let Some(nahe) = nahe {
        children.push(word_no_leading_pause(nahe, source));
    }
    if let Some(na) = na {
        children.push(word_no_leading_pause(na, source));
    }
    children.extend(
        cmavo
            .value
            .iter()
            .take(cmavo.value.len().saturating_sub(1))
            .map(|item| word_no_leading_pause(item, source)),
    );
    children.extend(
        cmavo
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    if let Some(nai) = nai {
        children.push(with_free_word_no_leading_pause(nai, source));
    }
    sexpr::node(children)
}

type ConnectivePartsRef<'a> = (
    Option<&'a Token>,
    Option<&'a Token>,
    Option<&'a Token>,
    &'a WithFreeModifiers<Vec<Token>>,
    Option<&'a WithFreeModifiers<Token>>,
);

#[requires(true)]
#[ensures(true)]
fn connective_parts(value: &ConnectiveSyntax) -> ConnectivePartsRef<'_> {
    match value.as_data() {
        data!(ConnectiveSyntax::Afterthought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Selbri {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::BridiTail {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Forethought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::NonLogical {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Interval {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }) => (
            se.as_ref(),
            nahe.as_ref(),
            na.as_ref(),
            cmavo.as_ref(),
            nai.as_deref(),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn free_modifier(value: &FreeModifierSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    match value.as_data() {
        data!(FreeModifierSyntax::MetalinguisticBridi {
            sei,
            terms,
            cu,
            selbri,
            sehu,
        }) => {
            let mut children = vec![
                with_free_word(sei, source),
                list_node(terms.iter().map(|item| term(item, source)).collect()),
            ];
            if let Some(cu) = cu {
                children.push(with_free_word(cu, source));
            }
            children.push(relation_syntax(selbri, source));
            push_optional_elidable(
                &mut children,
                sehu.as_ref(),
                Cmavo::Sehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FreeModifierSyntax::ParentheticalText { to, text, toi }) => {
            let mut children = vec![with_free_word(to, source), self::text(text, source)];
            push_optional_elidable(
                &mut children,
                toi.as_ref(),
                Cmavo::Toi,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FreeModifierSyntax::Subscript { xi, expression }) => {
            sexpr::node(vec![with_free_word(xi, source), mekso(expression, source)])
        }
        data!(FreeModifierSyntax::UtteranceOrdinal { number, mai }) => {
            let mut children = vec![list_node(words(number, source))];
            children.push(with_free_word(mai, source));
            sexpr::node(children)
        }
        data!(FreeModifierSyntax::ReciprocalSumti {
            soi,
            leading_sumti,
            trailing_sumti,
            sehu,
        }) => {
            let mut children = vec![
                with_free_word(soi, source),
                argument_syntax(leading_sumti, source),
            ];
            if let Some(sumti) = trailing_sumti {
                children.push(argument_syntax(sumti, source));
            }
            push_optional_elidable(
                &mut children,
                sehu.as_ref(),
                Cmavo::Sehu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FreeModifierSyntax::Vocative {
            vocative_markers,
            sumti,
            dohu,
        }) => {
            let mut children = vec![with_free_words(vocative_markers, source)];
            if let Some(sumti) = sumti {
                children.push(argument_syntax(sumti, source));
            }
            push_optional_elidable(
                &mut children,
                dohu.as_ref(),
                Cmavo::Dohu,
                source,
                with_free_word,
            );
            sexpr::node(children)
        }
        data!(FreeModifierSyntax::TextReplacement {
            lohai,
            old_words,
            sahai,
            new_words,
            lehai,
        }) => {
            let mut children = Vec::new();
            if let Some(lohai) = lohai {
                children.push(word(lohai, source));
            }
            children.extend(words(old_words, source));
            if let Some(sahai) = sahai {
                children.push(word(sahai, source));
            }
            children.extend(words(new_words, source));
            children.push(with_free_word(lehai, source));
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator(value: &Indicator, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut rendered = surface::format_with_indicators_with_options(
        &value.indicator,
        source.source,
        source.options.phonemes,
    );
    if let Some(nai) = &value.nai {
        rendered.push('-');
        rendered.push_str(&normalize_attached_surface(
            surface::format_with_indicators_with_options(
                &Token::bare(WordLike::bare(nai.clone())),
                source.source,
                source.options.phonemes,
            ),
        ));
    }
    sexpr::leaf(rendered)
}

#[requires(true)]
#[ensures(true)]
fn indicators(values: &[Indicator], source: &BracketContext<'_>) -> sexpr::SExpr {
    let rendered = values
        .iter()
        .map(|value| match indicator(value, source) {
            sexpr::SExpr::Leaf { text, .. } => text,
            _ => String::new(),
        })
        .collect::<Vec<_>>();
    let Some((first, rest)) = rendered.split_first() else {
        return sexpr::empty_node();
    };
    let mut text = first.clone();
    for (index, rendered) in rest.iter().enumerate() {
        let next_index = index + 1;
        text.push_str(indicator_separator(
            &values[index],
            &values[next_index],
            source,
        ));
        text.push_str(rendered);
    }
    sexpr::leaf(text)
}

#[requires(true)]
#[ensures(ret == "." || ret == "-")]
fn indicator_separator(
    previous: &Indicator,
    next: &Indicator,
    source: &BracketContext<'_>,
) -> &'static str {
    let Some(previous_end) = previous
        .words()
        .last()
        .and_then(|word| word.source_spans().last().map(|span| span.byte_end))
    else {
        return "-";
    };
    let Some(next_start) = next
        .words()
        .first()
        .and_then(|word| word.source_spans().first().map(|span| span.byte_start))
    else {
        return "-";
    };
    if previous_end <= next_start
        && source
            .source
            .get(previous_end..next_start)
            .is_some_and(|text| text.contains('.'))
    {
        "."
    } else {
        "-"
    }
}

#[requires(true)]
#[ensures(true)]
fn with_free_word(value: &WithFreeModifiers<Token>, source: &BracketContext<'_>) -> sexpr::SExpr {
    let mut children = vec![word(&value.value, source)];
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn with_free_words(
    value: &WithFreeModifiers<impl AsRef<[Token]>>,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = words(value.value.as_ref(), source);
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn with_free_word_no_leading_pause(
    value: &WithFreeModifiers<Token>,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = vec![word_no_leading_pause(&value.value, source)];
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn with_free_words_no_leading_pause(
    value: &WithFreeModifiers<Vec<Token>>,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = value
        .value
        .iter()
        .map(|item| word_no_leading_pause(item, source))
        .collect::<Vec<_>>();
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(|item| free_modifier(item, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn words(words: &[Token], source: &BracketContext<'_>) -> Vec<sexpr::SExpr> {
    words.iter().map(|item| word(item, source)).collect()
}

#[requires(true)]
#[ensures(true)]
fn word(word: &Token, source: &BracketContext<'_>) -> sexpr::SExpr {
    with_indicators_brackets(word.as_indicators(), source)
}

#[requires(true)]
#[ensures(true)]
fn word_no_leading_pause(word: &Token, source: &BracketContext<'_>) -> sexpr::SExpr {
    sexpr::leaf(normalize_attached_surface(
        surface::format_with_indicators_with_options(word, source.source, source.options.phonemes),
    ))
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_brackets(
    word: &WithIndicators<WordLike>,
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    match word {
        WithIndicators::Plain(word_like) => word_like_brackets(word_like, source),
        WithIndicators::Emphasized { bahe, word_like } => sexpr::node(vec![
            word_leaf(bahe, source),
            word_like_brackets(word_like, source),
        ]),
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            let mut children = vec![
                with_indicators_brackets(base, source),
                word_leaf(indicator, source),
            ];
            if let Some(nai) = nai {
                children.push(word_leaf(nai, source));
            }
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn word_like_brackets(word_like: &WordLike, source: &BracketContext<'_>) -> sexpr::SExpr {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word_leaf(word, source),
        data!(WordLike::QuotedWord { zo, word }) => {
            sexpr::node(vec![word_leaf(zo, source), word_leaf(word, source)])
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => sexpr::node(vec![
            word_leaf(zoi, source),
            word_leaf(opening_delimiter, source),
            quoted_text_leaf(quoted_text),
            word_leaf(closing_delimiter, source),
        ]),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut children = vec![word_leaf(lohu, source)];
            children.extend(quoted_words.iter().map(|word| word_leaf(word, source)));
            children.push(word_leaf(lehu, source));
            sexpr::node(children)
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => sexpr::node(vec![
            word_leaf(marker, source),
            quoted_text_leaf(quoted_text),
        ]),
        data!(WordLike::LerfuWord { base, bu }) => sexpr::node(vec![
            word_like_brackets(base, source),
            word_leaf(bu, source),
        ]),
        data!(WordLike::ZeiCompound { left, zei, right }) => sexpr::node(vec![
            word_like_brackets(left, source),
            word_leaf(zei, source),
            word_leaf(right, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn word_leaf(word: &Word, source: &BracketContext<'_>) -> sexpr::SExpr {
    if source.options.decompose_lujvo
        && let Some(parts) = word.lujvo_parts()
    {
        return sexpr::leaf_with_range(
            parts
                .iter()
                .map(|part| part.phonemes().render(source.options.phonemes))
                .collect::<Vec<_>>()
                .join(source.options.glyphs.lujvo_separator()),
            Some(word_bracket_source_range(word)),
        );
    }
    sexpr::leaf_with_range(
        surface::format_with_indicators_with_options(
            &Token::bare(WordLike::bare(word.clone())),
            source.source,
            source.options.phonemes,
        ),
        Some(word_bracket_source_range(word)),
    )
}

#[requires(true)]
#[ensures(true)]
fn push_optional_elidable<T>(
    children: &mut Vec<sexpr::SExpr>,
    value: Option<&T>,
    cmavo: Cmavo,
    source: &BracketContext<'_>,
    render: impl FnOnce(&T, &BracketContext<'_>) -> sexpr::SExpr,
) {
    if let Some(value) = value {
        children.push(render(value, source));
    } else if source.options.show_elided {
        let terminator = elided_terminator_leaf(cmavo, children, source);
        children.push(terminator);
    }
}

#[requires(true)]
#[ensures(true)]
fn elided_terminator_leaf(
    cmavo: Cmavo,
    previous_siblings: &[sexpr::SExpr],
    source: &BracketContext<'_>,
) -> sexpr::SExpr {
    sexpr::elided_leaf_with_range(
        elided_cmavo_text(cmavo, source.options.phonemes),
        last_child_end_range(previous_siblings),
    )
}

#[requires(true)]
#[ensures(true)]
fn last_child_end_range(children: &[sexpr::SExpr]) -> Option<BracketSourceRange> {
    children.iter().rev().find_map(expr_end_range)
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start == range.byte_end))]
fn expr_end_range(expr: &sexpr::SExpr) -> Option<BracketSourceRange> {
    let range = match expr {
        sexpr::SExpr::Leaf { range, .. } | sexpr::SExpr::Node { range, .. } => *range,
    }?;
    Some(BracketSourceRange {
        byte_start: range.byte_end,
        byte_end: range.byte_end,
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn elided_cmavo_text(cmavo: Cmavo, options: jbotci_morphology::PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text")
        .render(options)
}

#[requires(true)]
#[ensures(true)]
fn quoted_text_leaf(verbatim: &jbotci_morphology::Verbatim) -> sexpr::SExpr {
    sexpr::leaf_with_range(
        verbatim.text.trim().to_owned(),
        Some(BracketSourceRange {
            byte_start: verbatim.span.byte_start,
            byte_end: verbatim.span.byte_end,
        }),
    )
}

#[requires(word.span().byte_start <= word.span().byte_end)]
#[ensures(ret.byte_start == word.span().byte_start)]
fn word_bracket_source_range(word: &Word) -> BracketSourceRange {
    BracketSourceRange {
        byte_start: word.span().byte_start,
        byte_end: word.span().byte_end,
    }
}

#[requires(true)]
#[ensures(!ret.starts_with('.'))]
fn normalize_attached_surface(text: String) -> String {
    text.trim_start_matches('.').replace('.', "-")
}

#[requires(true)]
#[ensures(true)]
fn prenex(terms: Vec<sexpr::SExpr>, zohu: sexpr::SExpr, inner: sexpr::SExpr) -> sexpr::SExpr {
    sexpr::node(vec![list_node(terms), zohu, inner])
}

#[requires(true)]
#[ensures(true)]
fn list_node(children: Vec<sexpr::SExpr>) -> sexpr::SExpr {
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn source_words_node<T>(value: &T, source: &BracketContext<'_>) -> sexpr::SExpr
where
    T: TreeNode,
{
    let mut visitor = SourceWordBracketVisitor {
        source,
        children: Vec::new(),
    };
    value.visit_in_order(&mut visitor);
    sexpr::node(visitor.children)
}

#[derive(Debug)]
#[invariant(true)]
struct SourceWordBracketVisitor<'source> {
    source: &'source BracketContext<'source>,
    children: Vec<sexpr::SExpr>,
}

impl<'tree> TreeVisitor<'tree> for SourceWordBracketVisitor<'_> {
    type Node = NodeRef<'tree>;
    type Atom = AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            AtomRef::Token(word) => self.children.push(self::word(word, self.source)),
            AtomRef::Word(word) => self.children.push(word_leaf(word, self.source)),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn math_expression_syntax(value: &MeksoSyntax, source: &BracketContext<'_>) -> sexpr::SExpr {
    mekso(value, source)
}
