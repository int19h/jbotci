use bityzba::data;
#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::{invariant, requires};
use jbotci_morphology::{Word, WordLike, WordLikeData};
use jbotci_syntax::ast::*;
use jbotci_syntax::{Indicator, WithIndicators};
use jbotci_tree::TreeVisitor;

use crate::{BracketRenderOptions, OutputError, sexpr, surface};

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_brackets_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let sexpr = text(tree, source);
    Ok(sexpr::render_bracketed_with_options(
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
    let sexpr = sexpr::node(
        words
            .iter()
            .map(|word_like| word_like_brackets(word_like, source))
            .collect(),
    );
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(true)]
fn text(tree: &TextSyntax, source: &str) -> sexpr::SExpr {
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
fn paragraph(value: &ParagraphSyntax, source: &str) -> sexpr::SExpr {
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
fn paragraph_statement(value: &ParagraphStatementSyntax, source: &str) -> sexpr::SExpr {
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
fn statement_syntax(value: &StatementSyntax, source: &str) -> sexpr::SExpr {
    match value {
        StatementSyntax::Tuhe {
            tense_modal,
            tuhe,
            text: inner,
            tuhu,
        } => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(tuhe, source));
            children.push(text(inner, source));
            if let Some(tuhu) = tuhu {
                children.push(with_free_word(tuhu, source));
            }
            sexpr::node(children)
        }
        StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_statement,
        } => prenex(
            prenex_terms.iter().map(|item| term(item, source)).collect(),
            with_free_word(zohu, source),
            statement_syntax(inner_statement, source),
        ),
        StatementSyntax::Predicate(predicate) => predicate_syntax(predicate, source),
        StatementSyntax::Connected {
            i,
            connective,
            leading_statement,
            trailing_statement,
        } => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            word(i, source),
            connective_syntax(connective, source),
            statement_syntax(trailing_statement, source),
        ]),
        StatementSyntax::PreIConnected {
            connective,
            i,
            leading_statement,
            trailing_statement,
        } => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            connective_syntax(connective, source),
            word(i, source),
            statement_syntax(trailing_statement, source),
        ]),
        StatementSyntax::Iau {
            inner_statement,
            iau,
            reset_terms,
        } => {
            let mut children = vec![
                statement_syntax(inner_statement, source),
                with_free_word(iau, source),
            ];
            children.extend(reset_terms.iter().map(|item| term(item, source)));
            sexpr::node(children)
        }
        StatementSyntax::ExperimentalPredicateContinuation {
            leading_statement,
            continuation,
        } => sexpr::node(vec![
            statement_syntax(leading_statement, source),
            source_words_node(continuation, source),
        ]),
        StatementSyntax::Fragment(fragment) => fragment_syntax(fragment, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_syntax(value: &PredicateSyntax, source: &str) -> sexpr::SExpr {
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
    children.push(predicate_tail(&value.predicate_tail, source));
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
fn predicate_tail(value: &PredicateTailSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![predicate_tail1(&value.first, source)];
    if let Some(continuation) = &value.ke_continuation {
        children.push(ke_predicate_tail(continuation, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail1(value: &PredicateTail1Syntax, source: &str) -> sexpr::SExpr {
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
fn predicate_tail2(value: &PredicateTail2Syntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![predicate_tail3(&value.first, source)];
    if let Some(continuation) = &value.bo_continuation {
        children.push(bo_predicate_tail(continuation, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail3(value: &PredicateTail3Syntax, source: &str) -> sexpr::SExpr {
    match value {
        PredicateTail3Syntax::Relation {
            relation,
            terms,
            vau,
            free_modifiers,
        } => {
            let mut children = vec![relation_syntax(relation, source)];
            children.push(list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            ));
            if let Some(vau) = vau {
                children.push(with_free_word(vau, source));
            }
            children.push(list_node(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source))
                    .collect(),
            ));
            sexpr::node(children)
        }
        PredicateTail3Syntax::GekSentence(gek) => gek_sentence(gek, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn ke_predicate_tail(value: &KePredicateTailSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    children.push(with_free_word(&value.ke, source));
    children.push(predicate_tail(&value.predicate_tail, source));
    if let Some(kehe) = &value.kehe {
        children.push(with_free_word(kehe, source));
    }
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    if let Some(vau) = &value.vau {
        children.push(with_free_word(vau, source));
    }
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
    value: &PredicateTailContinuationSyntax,
    source: &str,
) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    if let Some(cu) = &value.cu {
        children.push(with_free_word(cu, source));
    }
    children.push(predicate_tail2(&value.predicate_tail, source));
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    if let Some(vau) = &value.vau {
        children.push(with_free_word(vau, source));
    }
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
fn bo_predicate_tail(value: &BoPredicateTailSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![connective_syntax(&value.connective, source)];
    if let Some(tense_modal) = &value.tense_modal {
        children.push(tense_modal_syntax(tense_modal, source));
    }
    children.push(with_free_word(&value.bo, source));
    if let Some(cu) = &value.cu {
        children.push(with_free_word(cu, source));
    }
    children.push(predicate_tail2(&value.predicate_tail, source));
    children.extend(value.tail_terms.iter().map(|item| term(item, source)));
    if let Some(vau) = &value.vau {
        children.push(with_free_word(vau, source));
    }
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
fn gek_sentence(value: &GekSentenceSyntax, source: &str) -> sexpr::SExpr {
    match value {
        GekSentenceSyntax::Pair {
            gek,
            first,
            gik,
            second,
            gihi,
            tail_terms,
            vau,
            free_modifiers,
        } => {
            let mut children = vec![
                connective_syntax(gek, source),
                subsentence(first, source),
                connective_syntax(gik, source),
                subsentence(second, source),
            ];
            if let Some(gihi) = gihi {
                children.push(word(gihi, source));
            }
            children.push(list_node(
                tail_terms.iter().map(|item| term(item, source)).collect(),
            ));
            if let Some(vau) = vau {
                children.push(with_free_word(vau, source));
            }
            children.extend(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            sexpr::node(children)
        }
        GekSentenceSyntax::Ke {
            tense_modal,
            ke,
            inner,
            kehe,
        } => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(gek_sentence(inner, source));
            if let Some(kehe) = kehe {
                children.push(with_free_word(kehe, source));
            }
            sexpr::node(children)
        }
        GekSentenceSyntax::Na { na, inner } => sexpr::node(vec![
            with_free_word(na, source),
            gek_sentence(inner, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn subsentence(value: &SubsentenceSyntax, source: &str) -> sexpr::SExpr {
    match value {
        SubsentenceSyntax::Plain(predicate) => predicate_syntax(predicate, source),
        SubsentenceSyntax::Prenex {
            prenex_terms,
            zohu,
            inner_subsentence,
        } => sexpr::node(vec![
            sexpr::node(vec![
                list_node(prenex_terms.iter().map(|item| term(item, source)).collect()),
                with_free_word(zohu, source),
            ]),
            subsentence(inner_subsentence, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn fragment_syntax(value: &FragmentSyntax, source: &str) -> sexpr::SExpr {
    match value {
        FragmentSyntax::Ek(connective) | FragmentSyntax::Gihek(connective) => {
            connective_syntax(connective, source)
        }
        FragmentSyntax::Other(words) => with_free_words(words, source),
        FragmentSyntax::Ijek { i, connective } => {
            sexpr::node(vec![word(i, source), connective_syntax(connective, source)])
        }
        FragmentSyntax::Prenex { terms, zohu } => {
            let header = terms
                .iter()
                .map(|item| term(item, source))
                .collect::<Vec<_>>();
            sexpr::node(vec![sexpr::node(vec![
                sexpr::node(header),
                with_free_word(zohu, source),
            ])])
        }
        FragmentSyntax::BeLink {
            be,
            fa,
            first_argument,
            bei_links,
            beho,
        } => {
            let mut children = vec![with_free_word(be, source)];
            if let Some(fa) = fa {
                children.push(with_free_word(fa, source));
            }
            if let Some(argument) = first_argument {
                children.push(argument_syntax(argument, source));
            }
            children.push(list_node(
                bei_links
                    .iter()
                    .map(|item| bei_link(item, source))
                    .collect(),
            ));
            if let Some(beho) = beho {
                children.push(with_free_word(beho, source));
            }
            sexpr::node(children)
        }
        FragmentSyntax::BeiLink(links) => {
            list_node(links.iter().map(|item| bei_link(item, source)).collect())
        }
        FragmentSyntax::RelativeClause(relative_clauses) => list_node(
            relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect(),
        ),
        FragmentSyntax::MathExpression(expression) => math_expression(expression, source),
        FragmentSyntax::Term { terms, vau } => {
            let mut children = vec![list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            )];
            if let Some(vau) = vau {
                children.push(with_free_word(vau, source));
            }
            sexpr::node(children)
        }
        FragmentSyntax::Relation(relation) => relation_syntax(relation, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn term(value: &TermSyntax, source: &str) -> sexpr::SExpr {
    match value {
        TermSyntax::Argument(argument) => argument_syntax(argument, source),
        TermSyntax::Tagged {
            tense_modal,
            argument,
        } => {
            let mut children = Vec::new();
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(argument_syntax(argument, source));
            sexpr::node(children)
        }
        TermSyntax::JaiTagged { jai, tag, argument } => {
            let mut children = vec![with_free_word(jai, source)];
            if let Some(tag) = tag {
                children.push(tense_modal_syntax(tag, source));
            }
            children.push(argument_syntax(argument, source));
            sexpr::node(children)
        }
        TermSyntax::Fa { fa, argument, ku } => {
            let mut children = vec![
                with_free_word(fa, source),
                argument_syntax(argument, source),
            ];
            if let Some(ku) = ku {
                children.push(with_free_word(ku, source));
            }
            sexpr::node(children)
        }
        TermSyntax::NaKu { na, na_ku } => {
            sexpr::node(vec![word(na, source), with_free_word(na_ku, source)])
        }
        TermSyntax::BareNa(na) => with_free_word(na, source),
        TermSyntax::NuhiTermset {
            nuhi,
            termset,
            nuhu,
        } => {
            let mut children = vec![
                with_free_word(nuhi, source),
                list_node(termset.iter().map(|item| term(item, source)).collect()),
            ];
            if let Some(nuhu) = nuhu {
                children.push(with_free_word(nuhu, source));
            }
            sexpr::node(children)
        }
        TermSyntax::GekNuhiTermset {
            m_nuhi,
            gek,
            terms,
            nuhu,
            gik,
            gik_terms,
            gihi,
            gik_nuhu,
        } => {
            let mut children = Vec::new();
            if let Some(nuhi) = m_nuhi {
                children.push(with_free_word(nuhi, source));
            }
            children.push(connective_syntax(gek, source));
            children.push(list_node(
                terms.iter().map(|item| term(item, source)).collect(),
            ));
            if let Some(nuhu) = nuhu {
                children.push(with_free_word(nuhu, source));
            }
            children.push(connective_syntax(gik, source));
            children.push(list_node(
                gik_terms.iter().map(|item| term(item, source)).collect(),
            ));
            if let Some(gihi) = gihi {
                children.push(word(gihi, source));
            }
            if let Some(nuhu) = gik_nuhu {
                children.push(with_free_word(nuhu, source));
            }
            sexpr::node(children)
        }
        TermSyntax::Cehe {
            leading_terms,
            cehe,
            trailing_terms,
        } => {
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
        TermSyntax::Pehe {
            leading_terms,
            pehe,
            connective,
            trailing_terms,
        } => {
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
        TermSyntax::Connected {
            leading_terms,
            connective,
            trailing_terms,
        } => {
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
        TermSyntax::BoConnected {
            leading_terms,
            bo_connective,
            tense_modal,
            bo,
            trailing_term,
        } => {
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
        TermSyntax::NoihaAdverbial {
            noiha,
            tail_elements,
            relation,
            relative_clauses,
            fehu,
        } => {
            let mut children = vec![with_free_word(noiha, source)];
            children.push(list_node(
                tail_elements
                    .iter()
                    .map(|item| argument_tail_element(item, source))
                    .collect(),
            ));
            if let Some(relation) = relation {
                children.push(relation_syntax(relation, source));
            }
            children.push(list_node(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source))
                    .collect(),
            ));
            if let Some(fehu) = fehu {
                children.push(with_free_word(fehu, source));
            }
            sexpr::node(children)
        }
        TermSyntax::PoihaBrigahi {
            poiha,
            tail_elements,
            relation,
            relative_clauses,
            brigahi_ku,
        } => {
            let mut children = vec![with_free_word(poiha, source)];
            children.push(list_node(
                tail_elements
                    .iter()
                    .map(|item| argument_tail_element(item, source))
                    .collect(),
            ));
            if let Some(relation) = relation {
                children.push(relation_syntax(relation, source));
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
        TermSyntax::FihoiAdverbial {
            fihoi,
            subsentence: inner,
            fihau,
        } => {
            let mut children = vec![with_free_word(fihoi, source), subsentence(inner, source)];
            if let Some(fihau) = fihau {
                children.push(with_free_word(fihau, source));
            }
            sexpr::node(children)
        }
        TermSyntax::SoiAdverbial {
            soi,
            subsentence: inner,
            sehu,
        } => {
            let mut children = vec![with_free_word(soi, source), subsentence(inner, source)];
            if let Some(sehu) = sehu {
                children.push(with_free_word(sehu, source));
            }
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_syntax(value: &ArgumentSyntax, source: &str) -> sexpr::SExpr {
    match value {
        ArgumentSyntax::Quote(quote) => quote_syntax(quote, source),
        ArgumentSyntax::MathExpression {
            li,
            expression,
            loho,
        } => {
            let mut children = vec![
                with_free_word(li, source),
                math_expression(expression, source),
            ];
            if let Some(loho) = loho {
                children.push(with_free_word(loho, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Letter { letter, boi } => {
            let mut children = vec![with_free_words(letter, source)];
            if let Some(boi) = boi {
                children.push(with_free_word(boi, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Quantified {
            quantifier,
            inner_argument,
        } => sexpr::node(vec![
            quantifier_syntax(quantifier, source),
            argument_syntax(inner_argument, source),
        ]),
        ArgumentSyntax::Connected {
            leading_argument,
            connective,
            trailing_argument,
        } => sexpr::node(vec![
            argument_syntax(leading_argument, source),
            connective_syntax(connective, source),
            argument_syntax(trailing_argument, source),
        ]),
        ArgumentSyntax::Descriptor(descriptor) => descriptor_syntax(descriptor, source),
        ArgumentSyntax::ConnectedDescriptor(descriptor) => connected_descriptor(descriptor, source),
        ArgumentSyntax::Name { la, names } => sexpr::node(vec![
            with_free_word(la, source),
            with_free_words(names, source),
        ]),
        ArgumentSyntax::Cmevla(words) => with_free_words(words, source),
        ArgumentSyntax::RelativeClause {
            base_argument,
            vuho,
            relative_clauses,
        } => {
            let mut children = vec![argument_syntax(base_argument, source)];
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
        ArgumentSyntax::Vuho {
            base_argument,
            vuho_marker,
            relative_clauses,
            connected_argument,
        } => {
            let mut children = vec![
                argument_syntax(base_argument, source),
                with_free_word(vuho_marker, source),
            ];
            children.extend(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            if let Some(connection) = connected_argument {
                children.push(sexpr::node(vec![
                    connective_syntax(&connection.connective, source),
                    argument_syntax(&connection.argument, source),
                ]));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::BridiDescription {
            lohoi,
            subsentence: inner,
            kuhau,
        } => {
            let mut children = vec![with_free_word(lohoi, source), subsentence(inner, source)];
            if let Some(kuhau) = kuhau {
                children.push(with_free_word(kuhau, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::NaKu { na, ku } => {
            sexpr::node(vec![word(na, source), with_free_word(ku, source)])
        }
        ArgumentSyntax::Tagged {
            tag,
            inner_argument,
        } => sexpr::node(vec![
            argument_tag(tag, source),
            argument_syntax(inner_argument, source),
        ]),
        ArgumentSyntax::NaheBo {
            nahe,
            bo,
            inner_argument,
            luhu,
        } => {
            let mut children = vec![
                word(nahe, source),
                with_free_word(bo, source),
                argument_syntax(inner_argument, source),
            ];
            if let Some(luhu) = luhu {
                children.push(with_free_word(luhu, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Nahe {
            nahe,
            inner_argument,
            luhu,
        } => {
            let mut children = vec![
                with_free_word(nahe, source),
                argument_syntax(inner_argument, source),
            ];
            if let Some(luhu) = luhu {
                children.push(with_free_word(luhu, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::TermWrapped {
            wrapper,
            wrapper_bo,
            inner_term,
            luhu,
            ..
        } => {
            let mut children = vec![with_free_word(wrapper, source)];
            if let Some(wrapper_bo) = wrapper_bo {
                children.push(with_free_word(wrapper_bo, source));
            }
            children.push(term(inner_term, source));
            if let Some(luhu) = luhu {
                children.push(with_free_word(luhu, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Koha(koha) => with_free_word(koha, source),
        ArgumentSyntax::Zohe {
            tag,
            maybe_ku,
            free_modifiers,
        } => {
            let mut children = Vec::new();
            if let Some(tag) = tag {
                children.push(argument_tag(tag, source));
            }
            if let Some(ku) = maybe_ku {
                children.push(with_free_word(ku, source));
            }
            children.extend(
                free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            sexpr::node(children)
        }
        ArgumentSyntax::Lahe {
            lahe,
            relative_clauses,
            inner_argument,
            luhu,
        } => {
            let mut children = vec![with_free_word(lahe, source)];
            children.extend(
                relative_clauses
                    .iter()
                    .map(|item| relative_clause(item, source)),
            );
            children.push(argument_syntax(inner_argument, source));
            if let Some(luhu) = luhu {
                children.push(with_free_word(luhu, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Ke {
            ke,
            inner_argument,
            kehe,
        } => {
            let mut children = vec![
                with_free_word(ke, source),
                argument_syntax(inner_argument, source),
            ];
            if let Some(kehe) = kehe {
                children.push(with_free_word(kehe, source));
            }
            sexpr::node(children)
        }
        ArgumentSyntax::Bo {
            leading_argument,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_argument,
        } => {
            let mut children = vec![argument_syntax(leading_argument, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(argument_syntax(trailing_argument, source));
            sexpr::node(children)
        }
        ArgumentSyntax::Gek {
            gek,
            leading_argument,
            gik,
            trailing_argument,
            gihi,
        } => sexpr::node(
            vec![
                connective_syntax(gek, source),
                argument_syntax(leading_argument, source),
                connective_syntax(gik, source),
                argument_syntax(trailing_argument, source),
            ]
            .into_iter()
            .chain(gihi.iter().map(|gihi| word(gihi, source)))
            .collect(),
        ),
        ArgumentSyntax::RelationVocative {
            leading_relative_clauses,
            relation,
            trailing_relative_clauses,
        } => {
            let mut children = leading_relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect::<Vec<_>>();
            children.push(relation_syntax(relation, source));
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
fn argument_tag(value: &ArgumentTagSyntax, source: &str) -> sexpr::SExpr {
    match value {
        ArgumentTagSyntax::TenseModal(tense_modal) => tense_modal_syntax(tense_modal, source),
        ArgumentTagSyntax::Fa(fa) => with_free_word(fa, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn descriptor_syntax(value: &DescriptorSyntax, source: &str) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(outer_quantifier) = &value.outer_quantifier {
        children.push(quantifier_syntax(outer_quantifier, source));
    }
    if let Some(descriptor) = &value.descriptor {
        children.push(with_free_word(descriptor, source));
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
    if let Some(relation) = &value.relation {
        children.push(relation_syntax(relation, source));
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
    if let Some(ku) = &value.ku {
        children.push(with_free_word(ku, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn connected_descriptor(value: &ConnectedDescriptorSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![
        descriptor_head(&value.leading_descriptor_head, source),
        connective_syntax(&value.connective, source),
        descriptor_head(&value.trailing_descriptor_head, source),
    ];
    children.extend(
        value
            .tail_elements
            .iter()
            .map(|item| argument_tail_element(item, source)),
    );
    if let Some(relation) = &value.relation {
        children.push(relation_syntax(relation, source));
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
    if let Some(ku) = &value.ku {
        children.push(with_free_word(ku, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn descriptor_head(value: &DescriptorHeadSyntax, source: &str) -> sexpr::SExpr {
    with_free_word(&value.descriptor, source)
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_element(value: &ArgumentTailElementSyntax, source: &str) -> sexpr::SExpr {
    match value {
        ArgumentTailElementSyntax::Argument(argument) => argument_syntax(argument, source),
        ArgumentTailElementSyntax::RelativeClauses(relative_clauses) => sexpr::node(
            relative_clauses
                .iter()
                .map(|item| relative_clause(item, source))
                .collect(),
        ),
        ArgumentTailElementSyntax::Quantifier(quantifier) => quantifier_syntax(quantifier, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clause(value: &RelativeClauseSyntax, source: &str) -> sexpr::SExpr {
    match value {
        RelativeClauseSyntax::Goi(goi) => {
            let mut children = vec![
                with_free_word(&goi.goi, source),
                argument_syntax(&goi.argument, source),
            ];
            if let Some(gehu) = &goi.gehu {
                children.push(with_free_word(gehu, source));
            }
            sexpr::node(children)
        }
        RelativeClauseSyntax::Noi {
            noi,
            subsentence: inner,
            kuho,
        }
        | RelativeClauseSyntax::Poi {
            poi: noi,
            subsentence: inner,
            kuho,
        } => {
            let mut children = vec![with_free_word(noi, source), subsentence(inner, source)];
            if let Some(kuho) = kuho {
                children.push(with_free_word(kuho, source));
            }
            sexpr::node(children)
        }
        RelativeClauseSyntax::Zihe { zihe, inner } => sexpr::node(vec![
            with_free_word(zihe, source),
            relative_clause(inner, source),
        ]),
        RelativeClauseSyntax::Connected { connective, inner } => sexpr::node(vec![
            connective_syntax(connective, source),
            relative_clause(inner, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_syntax(value: &QuoteSyntax, source: &str) -> sexpr::SExpr {
    match value {
        QuoteSyntax::Lu { lu, text, lihu } => {
            let mut children = vec![word(&lu.value, source)];
            children.extend(
                lu.free_modifiers
                    .iter()
                    .map(|item| free_modifier(item, source)),
            );
            children.push(self::text(text, source));
            if let Some(lihu) = lihu {
                children.push(with_free_word(lihu, source));
            }
            sexpr::node(children)
        }
        QuoteSyntax::Zo(zo)
        | QuoteSyntax::ZohOi(zo)
        | QuoteSyntax::Zoi(zo)
        | QuoteSyntax::Lohu(zo) => with_free_word(zo, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_syntax(value: &QuantifierSyntax, source: &str) -> sexpr::SExpr {
    match value {
        QuantifierSyntax::Number { number, boi } => {
            let mut children = vec![with_free_words(number, source)];
            if let Some(boi) = boi {
                children.push(with_free_word(boi, source));
            }
            sexpr::node(children)
        }
        QuantifierSyntax::Vei {
            vei,
            math_expression,
            veho,
        } => {
            let mut children = vec![
                with_free_word(vei, source),
                self::math_expression(math_expression, source),
            ];
            if let Some(veho) = veho {
                children.push(with_free_word(veho, source));
            }
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn math_expression(value: &MathExpressionSyntax, source: &str) -> sexpr::SExpr {
    match value {
        MathExpressionSyntax::Number(number) => quantifier_syntax(number, source),
        MathExpressionSyntax::Letter { letter, boi } => {
            let mut children = vec![with_free_words(letter, source)];
            if let Some(boi) = boi {
                children.push(with_free_word(boi, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Binary {
            operator,
            left_expression,
            right_expression,
        } => sexpr::node(vec![
            self::math_expression(left_expression, source),
            math_operator(operator, source),
            self::math_expression(right_expression, source),
        ]),
        MathExpressionSyntax::Connected {
            left_expression,
            connective,
            right_expression,
        } => sexpr::node(vec![
            self::math_expression(left_expression, source),
            connective_syntax(connective, source),
            self::math_expression(right_expression, source),
        ]),
        MathExpressionSyntax::Vei {
            vei,
            inner_expression,
            veho,
        } => {
            let mut children = vec![
                with_free_word_no_leading_pause(vei, source),
                self::math_expression(inner_expression, source),
            ];
            if let Some(veho) = veho {
                children.push(with_free_word_no_leading_pause(veho, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Gek {
            gek,
            left_expression,
            gik,
            right_expression,
        } => sexpr::node(vec![
            connective_syntax(gek, source),
            self::math_expression(left_expression, source),
            connective_syntax(gik, source),
            self::math_expression(right_expression, source),
        ]),
        MathExpressionSyntax::Forethought {
            peho,
            operator,
            operands,
            kuhe,
        } => {
            let mut children = Vec::new();
            if let Some(peho) = peho {
                children.push(with_free_word(peho, source));
            }
            children.push(math_operator(operator, source));
            children.push(list_node(
                operands
                    .iter()
                    .map(|item| self::math_expression(item, source))
                    .collect(),
            ));
            if let Some(kuhe) = kuhe {
                children.push(with_free_word(kuhe, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::ReversePolish {
            fuha,
            operands,
            operators,
        } => {
            let mut children = vec![with_free_word(fuha, source)];
            children.extend(
                operands
                    .iter()
                    .map(|item| self::math_expression(item, source)),
            );
            children.extend(operators.iter().map(|item| math_operator(item, source)));
            sexpr::node(children)
        }
        MathExpressionSyntax::Nihe {
            nihe,
            relation,
            tehu,
        } => {
            let mut children = vec![
                with_free_word(nihe, source),
                relation_syntax(relation, source),
            ];
            if let Some(tehu) = tehu {
                children.push(with_free_word(tehu, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Mohe {
            mohe,
            argument,
            tehu,
        } => {
            let mut children = vec![
                with_free_word(mohe, source),
                argument_syntax(argument, source),
            ];
            if let Some(tehu) = tehu {
                children.push(with_free_word(tehu, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Johi {
            johi,
            expressions,
            tehu,
        } => {
            let mut children = vec![with_free_word(johi, source)];
            children.push(list_node(
                expressions
                    .iter()
                    .map(|item| self::math_expression(item, source))
                    .collect(),
            ));
            if let Some(tehu) = tehu {
                children.push(with_free_word(tehu, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Lahe {
            markers,
            inner_expression,
            luhu,
        } => {
            let mut children = vec![
                with_free_words(markers, source),
                self::math_expression(inner_expression, source),
            ];
            if let Some(luhu) = luhu {
                children.push(with_free_word(luhu, source));
            }
            sexpr::node(children)
        }
        MathExpressionSyntax::Bihe {
            left_expression,
            bihe,
            operator,
            right_expression,
        } => sexpr::node(vec![
            self::math_expression(left_expression, source),
            with_free_word_no_leading_pause(bihe, source),
            math_operator(operator, source),
            self::math_expression(right_expression, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator(value: &MathOperatorSyntax, source: &str) -> sexpr::SExpr {
    match value {
        MathOperatorSyntax::Vuhu(word) => with_free_word(word, source),
        MathOperatorSyntax::Connected {
            left_operator,
            connective,
            right_operator,
        } => {
            if connective.kind() == ConnectiveKind::Forethought
                && connective.cmavo().value.len() >= 2
            {
                let mut children = vec![
                    connective_prefix(connective, source),
                    math_operator(left_operator, source),
                ];
                if let Some(gi) = connective.cmavo().value.last() {
                    children.push(word_no_leading_pause(gi, source));
                }
                children.push(math_operator(right_operator, source));
                sexpr::node(children)
            } else {
                sexpr::node(vec![
                    math_operator(left_operator, source),
                    connective_syntax(connective, source),
                    math_operator(right_operator, source),
                ])
            }
        }
        MathOperatorSyntax::Maho {
            maho,
            math_expression,
            tehu,
        } => {
            let mut children = vec![
                with_free_word(maho, source),
                self::math_expression(math_expression, source),
            ];
            if let Some(tehu) = tehu {
                children.push(with_free_word(tehu, source));
            }
            sexpr::node(children)
        }
        MathOperatorSyntax::Se { se, inner_operator }
        | MathOperatorSyntax::Nahe {
            nahe: se,
            inner_operator,
        } => sexpr::node(vec![
            with_free_word(se, source),
            math_operator(inner_operator, source),
        ]),
        MathOperatorSyntax::Nahu {
            nahu,
            relation,
            tehu,
        } => {
            let mut children = vec![
                with_free_word(nahu, source),
                relation_syntax(relation, source),
            ];
            if let Some(tehu) = tehu {
                children.push(with_free_word(tehu, source));
            }
            sexpr::node(children)
        }
        MathOperatorSyntax::Ke {
            ke,
            inner_operator,
            kehe,
        } => {
            let mut children = vec![
                with_free_word(ke, source),
                math_operator(inner_operator, source),
            ];
            if let Some(kehe) = kehe {
                children.push(with_free_word(kehe, source));
            }
            sexpr::node(children)
        }
        MathOperatorSyntax::Bo {
            left_operator,
            connective,
            bo,
            right_operator,
        } => sexpr::node(vec![
            math_operator(left_operator, source),
            connective_syntax(connective, source),
            with_free_word(bo, source),
            math_operator(right_operator, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_syntax(value: &RelationSyntax, source: &str) -> sexpr::SExpr {
    match value {
        RelationSyntax::Base(value) => word(value, source),
        RelationSyntax::Compound(units) => sexpr::node(
            units
                .iter()
                .map(|unit| relation_unit(unit, source))
                .collect(),
        ),
        RelationSyntax::Connected {
            connective,
            leading_relation,
            trailing_relation,
        } => sexpr::node(vec![
            relation_syntax(leading_relation, source),
            connective_syntax(connective, source),
            relation_syntax(trailing_relation, source),
        ]),
        RelationSyntax::Co {
            leading_relation,
            co,
            trailing_relation,
        } => sexpr::node(vec![
            relation_syntax(leading_relation, source),
            with_free_word(co, source),
            relation_syntax(trailing_relation, source),
        ]),
        RelationSyntax::Bo {
            leading_relation,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_relation,
        } => {
            let mut children = vec![relation_syntax(leading_relation, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(relation_syntax(trailing_relation, source));
            sexpr::node(children)
        }
        RelationSyntax::Na { na, inner_relation } => sexpr::node(vec![
            with_free_word(na, source),
            relation_syntax(inner_relation, source),
        ]),
        RelationSyntax::Se { se, inner_relation } => sexpr::node(vec![
            with_free_word(se, source),
            relation_syntax(inner_relation, source),
        ]),
        RelationSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => {
            let mut children = Vec::new();
            if let Some(tense_modal) = ke_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(relation_syntax(relation, source));
            if let Some(kehe) = kehe {
                children.push(with_free_word(kehe, source));
            }
            sexpr::node(children)
        }
        RelationSyntax::TenseModal {
            tense_modal,
            inner_relation,
        } => sexpr::node(vec![
            tense_modal_syntax(tense_modal, source),
            relation_syntax(inner_relation, source),
        ]),
        RelationSyntax::Guha {
            guhek,
            leading_predicate,
            gik,
            trailing_predicate,
            gihi,
        } => sexpr::node(
            vec![
                connective_syntax(guhek, source),
                predicate_syntax(leading_predicate, source),
                connective_syntax(gik, source),
                predicate_syntax(trailing_predicate, source),
            ]
            .into_iter()
            .chain(gihi.iter().map(|gihi| word(gihi, source)))
            .collect(),
        ),
        RelationSyntax::Abstraction(abstraction) => abstraction_syntax(abstraction, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_unit(value: &RelationUnitSyntax, source: &str) -> sexpr::SExpr {
    match value {
        RelationUnitSyntax::Word(word) => with_free_word(word, source),
        RelationUnitSyntax::Goha { goha, raho } => {
            let mut children = vec![with_free_word(goha, source)];
            if let Some(raho) = raho {
                children.push(with_free_word(raho, source));
            }
            sexpr::node(children)
        }
        RelationUnitSyntax::Se { se, inner_unit } => sexpr::node(vec![
            with_free_word(se, source),
            relation_unit(inner_unit, source),
        ]),
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => {
            let mut children = Vec::new();
            if let Some(tense_modal) = ke_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(ke, source));
            children.push(relation_syntax(relation, source));
            if let Some(kehe) = kehe {
                children.push(with_free_word(kehe, source));
            }
            sexpr::node(children)
        }
        RelationUnitSyntax::Nahe { nahe, inner_unit } => sexpr::node(vec![
            with_free_word(nahe, source),
            relation_unit(inner_unit, source),
        ]),
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        } => {
            let mut children = vec![relation_unit(leading_unit, source)];
            if let Some(connective) = bo_connective {
                children.push(connective_syntax(connective, source));
            }
            if let Some(tense_modal) = bo_tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(with_free_word(bo, source));
            children.push(relation_unit(trailing_unit, source));
            sexpr::node(children)
        }
        RelationUnitSyntax::Connected {
            leading_unit,
            connective,
            trailing_unit,
        } => sexpr::node(vec![
            relation_unit(leading_unit, source),
            connective_syntax(connective, source),
            relation_unit(trailing_unit, source),
        ]),
        RelationUnitSyntax::Wrapped(relation) => relation_syntax(relation, source),
        RelationUnitSyntax::Jai {
            jai,
            tense_modal,
            inner_unit,
        } => {
            let mut children = vec![with_free_word(jai, source)];
            if let Some(tense_modal) = tense_modal {
                children.push(tense_modal_syntax(tense_modal, source));
            }
            children.push(relation_unit(inner_unit, source));
            sexpr::node(children)
        }
        RelationUnitSyntax::Be {
            base,
            be,
            fa,
            first_argument,
            bei_links,
            beho,
        } => be_link_node(
            relation_unit(base, source),
            be,
            fa.as_ref(),
            first_argument.as_ref(),
            bei_links,
            beho.as_ref(),
            source,
            false,
        ),
        RelationUnitSyntax::PreposedBe {
            be,
            fa,
            first_argument,
            bei_links,
            beho,
            base,
        } => be_link_node(
            relation_unit(base, source),
            be,
            fa.as_ref(),
            first_argument.as_ref(),
            bei_links,
            beho.as_ref(),
            source,
            true,
        ),
        RelationUnitSyntax::Abstraction(abstraction) => abstraction_syntax(abstraction, source),
        RelationUnitSyntax::Me {
            me,
            argument,
            mehu,
            moi_marker,
        } => {
            let mut children = vec![
                with_free_word(me, source),
                argument_syntax(argument, source),
            ];
            if let Some(mehu) = mehu {
                children.push(with_free_word(mehu, source));
            }
            if let Some(moi) = moi_marker {
                children.push(with_free_word(moi, source));
            }
            sexpr::node(children)
        }
        RelationUnitSyntax::Mehoi(mehoi) => with_free_word(mehoi, source),
        RelationUnitSyntax::Gohoi(gohoi) => with_free_word(gohoi, source),
        RelationUnitSyntax::Muhoi(muhoi) => with_free_word(muhoi, source),
        RelationUnitSyntax::Luhei { luhei, text, liau } => {
            let mut children = vec![with_free_word(luhei, source), self::text(text, source)];
            if let Some(liau) = liau {
                children.push(with_free_word(liau, source));
            }
            sexpr::node(children)
        }
        RelationUnitSyntax::Moi { number, moi } => {
            let mut children = vec![list_node(words(number, source))];
            children.push(with_free_word(moi, source));
            sexpr::node(children)
        }
        RelationUnitSyntax::Nuha {
            nuha,
            math_operator: operator,
        } => sexpr::node(vec![
            with_free_word(nuha, source),
            math_operator(operator, source),
        ]),
        RelationUnitSyntax::Xohi { xohi, tag } => sexpr::node(vec![
            with_free_word(xohi, source),
            tense_modal_syntax(tag, source),
        ]),
        RelationUnitSyntax::Cei { base, assignments } => {
            let mut children = vec![relation_unit(base, source)];
            children.extend(assignments.iter().map(|assignment| {
                sexpr::node(vec![
                    with_free_word(&assignment.cei, source),
                    relation_unit(&assignment.relation_unit, source),
                ])
            }));
            sexpr::node(children)
        }
        RelationUnitSyntax::SelbriRelativeClause {
            base,
            selbri_relative_clauses,
        } => {
            let mut children = vec![relation_unit(base, source)];
            children.extend(selbri_relative_clauses.iter().map(|item| {
                let mut items = vec![
                    with_free_word(&item.nohoi, source),
                    relation_syntax(&item.relation, source),
                ];
                if let Some(kuhoi) = &item.kuhoi {
                    items.push(with_free_word(kuhoi, source));
                }
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
    be: &WithFreeModifiers<WithIndicators<WordLike>>,
    fa: Option<&WithFreeModifiers<WithIndicators<WordLike>>>,
    first_argument: Option<&ArgumentSyntax>,
    bei_links: &[BeiLinkSyntax],
    beho: Option<&WithFreeModifiers<WithIndicators<WordLike>>>,
    source: &str,
    preposed: bool,
) -> sexpr::SExpr {
    let mut link_children = vec![with_free_word(be, source)];
    if let Some(fa) = fa {
        link_children.push(with_free_word(fa, source));
    }
    if let Some(argument) = first_argument {
        link_children.push(argument_syntax(argument, source));
    }
    link_children.extend(bei_links.iter().map(|item| bei_link(item, source)));
    if let Some(beho) = beho {
        link_children.push(with_free_word(beho, source));
    }
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
fn bei_link(value: &BeiLinkSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![with_free_word(&value.bei, source)];
    if let Some(fa) = &value.fa {
        children.push(with_free_word(fa, source));
    }
    if let Some(argument) = &value.argument {
        children.push(argument_syntax(argument, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn abstraction_syntax(value: &AbstractionSyntax, source: &str) -> sexpr::SExpr {
    let mut children = vec![with_free_word(&value.nu, source)];
    if let Some(nai) = &value.nai {
        children.push(with_free_word(nai, source));
    }
    children.extend(value.additional_nu.iter().map(|item| {
        let mut items = vec![
            connective_syntax(&item.connective, source),
            with_free_word(&item.nu, source),
        ];
        if let Some(nai) = &item.nai {
            items.push(with_free_word(nai, source));
        }
        sexpr::node(items)
    }));
    children.push(subsentence(&value.subsentence, source));
    if let Some(kei) = &value.kei {
        children.push(with_free_word(kei, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_syntax(value: &TenseModalSyntax, source: &str) -> sexpr::SExpr {
    match value {
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
        } => {
            let mut children = vec![
                with_free_word(fiho, source),
                relation_syntax(relation, source),
            ];
            if let Some(fehu) = fehu {
                children.push(with_free_word(fehu, source));
            }
            sexpr::node(children)
        }
        TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki,
        } => {
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
        TenseModalSyntax::Pu(word)
        | TenseModalSyntax::TimeInterval(word)
        | TenseModalSyntax::SpaceDistance(word)
        | TenseModalSyntax::SpaceDirection(word)
        | TenseModalSyntax::Ki(word)
        | TenseModalSyntax::Caha(word) => with_free_word(word, source),
        TenseModalSyntax::PuDistance { pu, distance } => {
            sexpr::node(vec![word(pu, source), with_free_word(distance, source)])
        }
        TenseModalSyntax::PuCaha { pu, caha } => {
            sexpr::node(vec![word(pu, source), with_free_word(caha, source)])
        }
        TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
        } => {
            let mut children = vec![word(mohi, source), with_free_word(direction, source)];
            if let Some(distance) = distance {
                children.push(with_free_word(distance, source));
            }
            sexpr::node(children)
        }
        TenseModalSyntax::Zaho(words) => with_free_words(words, source),
        TenseModalSyntax::Interval {
            number,
            roi_or_tahe,
            nai,
        } => {
            let mut children = number
                .as_ref()
                .map_or_else(Vec::new, |number| words(number, source));
            children.push(with_free_word(roi_or_tahe, source));
            if let Some(nai) = nai {
                children.push(with_free_word(nai, source));
            }
            sexpr::node(children)
        }
        TenseModalSyntax::Composite { parts } => {
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
fn composite_tense_modal_part(value: &CompositeTenseModalPartSyntax, source: &str) -> sexpr::SExpr {
    match value {
        CompositeTenseModalPartSyntax::Word(part_word) => word(part_word, source),
        CompositeTenseModalPartSyntax::Fiho(fiho) => fiho_modal(fiho, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn fiho_modal(value: &FihoModalSyntax, source: &str) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(nahe) = &value.nahe {
        children.push(word(nahe, source));
    }
    children.push(with_free_word(&value.fiho, source));
    children.push(relation_syntax(&value.relation, source));
    if let Some(fehu) = &value.fehu {
        children.push(with_free_word(fehu, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn connective_syntax(value: &ConnectiveSyntax, source: &str) -> sexpr::SExpr {
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
fn connective_prefix(value: &ConnectiveSyntax, source: &str) -> sexpr::SExpr {
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
    Option<&'a WithIndicators<WordLike>>,
    Option<&'a WithIndicators<WordLike>>,
    Option<&'a WithIndicators<WordLike>>,
    &'a WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    Option<&'a WithFreeModifiers<WithIndicators<WordLike>>>,
);

#[requires(true)]
#[ensures(true)]
fn connective_parts(value: &ConnectiveSyntax) -> ConnectivePartsRef<'_> {
    match value {
        ConnectiveSyntax::Afterthought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }
        | ConnectiveSyntax::Relation {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }
        | ConnectiveSyntax::PredicateTail {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }
        | ConnectiveSyntax::Forethought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }
        | ConnectiveSyntax::NonLogical {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }
        | ConnectiveSyntax::Interval {
            se,
            nahe,
            na,
            cmavo,
            nai,
        } => (se.as_ref(), nahe.as_ref(), na.as_ref(), cmavo, nai.as_ref()),
    }
}

#[requires(true)]
#[ensures(true)]
fn free_modifier(value: &FreeModifierSyntax, source: &str) -> sexpr::SExpr {
    match value {
        FreeModifierSyntax::Sei {
            sei,
            terms,
            cu,
            relation,
            sehu,
        } => {
            let mut children = vec![
                with_free_word(sei, source),
                list_node(terms.iter().map(|item| term(item, source)).collect()),
            ];
            if let Some(cu) = cu {
                children.push(with_free_word(cu, source));
            }
            children.push(relation_syntax(relation, source));
            if let Some(sehu) = sehu {
                children.push(with_free_word(sehu, source));
            }
            sexpr::node(children)
        }
        FreeModifierSyntax::To { to, text, toi } => {
            let mut children = vec![with_free_word(to, source), self::text(text, source)];
            if let Some(toi) = toi {
                children.push(with_free_word(toi, source));
            }
            sexpr::node(children)
        }
        FreeModifierSyntax::Xi { xi, expression } => sexpr::node(vec![
            with_free_word(xi, source),
            math_expression(expression, source),
        ]),
        FreeModifierSyntax::Mai { number, mai } => {
            let mut children = vec![list_node(words(number, source))];
            children.push(with_free_word(mai, source));
            sexpr::node(children)
        }
        FreeModifierSyntax::Soi {
            soi,
            leading_argument,
            trailing_argument,
            sehu,
        } => {
            let mut children = vec![
                with_free_word(soi, source),
                argument_syntax(leading_argument, source),
            ];
            if let Some(argument) = trailing_argument {
                children.push(argument_syntax(argument, source));
            }
            if let Some(sehu) = sehu {
                children.push(with_free_word(sehu, source));
            }
            sexpr::node(children)
        }
        FreeModifierSyntax::Vocative {
            vocative_markers,
            argument,
            dohu,
        } => {
            let mut children = vec![with_free_words(vocative_markers, source)];
            if let Some(argument) = argument {
                children.push(argument_syntax(argument, source));
            }
            if let Some(dohu) = dohu {
                children.push(with_free_word(dohu, source));
            }
            sexpr::node(children)
        }
        FreeModifierSyntax::Replacement {
            lohai,
            old_words,
            sahai,
            new_words,
            lehai,
        } => {
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
fn indicator(value: &Indicator, source: &str) -> sexpr::SExpr {
    let mut rendered = surface::format_with_indicators(&value.indicator, source);
    if let Some(nai) = &value.nai {
        rendered.push('-');
        rendered.push_str(&normalize_attached_surface(
            surface::format_with_indicators(
                &WithIndicators::bare(WordLike::bare((**nai).clone())),
                source,
            ),
        ));
    }
    sexpr::leaf(rendered)
}

#[requires(true)]
#[ensures(true)]
fn indicators(values: &[Indicator], source: &str) -> sexpr::SExpr {
    let rendered = values
        .iter()
        .map(|value| match indicator(value, source) {
            sexpr::SExpr::Leaf(text) => text,
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
fn indicator_separator(previous: &Indicator, next: &Indicator, source: &str) -> &'static str {
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
fn with_free_word(
    value: &WithFreeModifiers<WithIndicators<WordLike>>,
    source: &str,
) -> sexpr::SExpr {
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
    value: &WithFreeModifiers<impl AsRef<[WithIndicators<WordLike>]>>,
    source: &str,
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
    value: &WithFreeModifiers<WithIndicators<WordLike>>,
    source: &str,
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
    value: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    source: &str,
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
fn words(words: &[WithIndicators<WordLike>], source: &str) -> Vec<sexpr::SExpr> {
    words.iter().map(|item| word(item, source)).collect()
}

#[requires(true)]
#[ensures(true)]
fn word(word: &WithIndicators<WordLike>, source: &str) -> sexpr::SExpr {
    with_indicators_brackets(word, source)
}

#[requires(true)]
#[ensures(true)]
fn word_no_leading_pause(word: &WithIndicators<WordLike>, source: &str) -> sexpr::SExpr {
    sexpr::leaf(normalize_attached_surface(surface::format_with_indicators(
        word, source,
    )))
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_brackets(word: &WithIndicators<WordLike>, source: &str) -> sexpr::SExpr {
    match word {
        WithIndicators::Bare(word_like) => word_like_brackets(word_like, source),
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
fn word_like_brackets(word_like: &WordLike, source: &str) -> sexpr::SExpr {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => word_leaf(word, source),
        data!(WordLike::ZoQuote { zo, word }) => {
            sexpr::node(vec![word_leaf(zo, source), word_leaf(word, source)])
        }
        data!(WordLike::ZoiQuote {
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
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut children = vec![word_leaf(lohu, source)];
            children.extend(quoted_words.iter().map(|word| word_leaf(word, source)));
            children.push(word_leaf(lehu, source));
            sexpr::node(children)
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => sexpr::node(vec![
            word_leaf(marker, source),
            quoted_text_leaf(quoted_text),
        ]),
        data!(WordLike::Letter { base, bu }) => sexpr::node(vec![
            word_like_brackets(base, source),
            word_leaf(bu, source),
        ]),
        data!(WordLike::ZeiLujvo { left, zei, right }) => sexpr::node(vec![
            word_like_brackets(left, source),
            word_leaf(zei, source),
            word_leaf(right, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn word_leaf(word: &Word, source: &str) -> sexpr::SExpr {
    sexpr::leaf(surface::format_with_indicators(
        &WithIndicators::bare(WordLike::bare(word.clone())),
        source,
    ))
}

#[requires(true)]
#[ensures(true)]
fn quoted_text_leaf(verbatim: &jbotci_morphology::Verbatim) -> sexpr::SExpr {
    sexpr::leaf(verbatim.text.trim().to_owned())
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
fn source_words_node<T>(value: &T, source: &str) -> sexpr::SExpr
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
    source: &'source str,
    children: Vec<sexpr::SExpr>,
}

impl<'tree> TreeVisitor<'tree> for SourceWordBracketVisitor<'_> {
    type Node = NodeRef<'tree>;
    type Atom = AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            AtomRef::WithIndicatorsWordLike(word) => {
                self.children.push(self::word(word, self.source))
            }
            AtomRef::Word(word) => self.children.push(word_leaf(word, self.source)),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn math_expression_syntax(value: &MathExpressionSyntax, source: &str) -> sexpr::SExpr {
    math_expression(value, source)
}
