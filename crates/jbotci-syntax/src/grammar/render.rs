use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn lojban_text_tree(text: TextSyntax) -> SyntaxValue {
    let paragraphs = paragraphs_tree(text.clone());
    node(
        "LojbanText",
        vec![
            field(
                "leadingNai",
                list(text.leading_nai.into_iter().map(word_value).collect()),
            ),
            field(
                "leadingCmevla",
                list(
                    text.leading_cmevla
                        .into_iter()
                        .map(name_word_value)
                        .collect(),
                ),
            ),
            field(
                "leadingIndicators",
                list(
                    text.leading_indicators
                        .into_iter()
                        .map(word_value)
                        .collect(),
                ),
            ),
            field(
                "leadingFreeModifiers",
                list(
                    text.leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "leadingConnective",
                text.leading_connective
                    .map_or_else(nothing, |connective| just(connective_tree(connective))),
            ),
            field("paragraphs", paragraphs),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn paragraphs_tree(text: TextSyntax) -> SyntaxValue {
    list(text.paragraphs.into_iter().map(paragraph_tree).collect())
}

#[requires(true)]
#[ensures(true)]
fn paragraph_tree(paragraph: ParagraphSyntax) -> SyntaxValue {
    node(
        "Paragraph",
        vec![
            field("i", maybe_word(paragraph.i)),
            field(
                "niho",
                list(paragraph.niho.into_iter().map(word_value).collect()),
            ),
            field(
                "freeModifiers",
                list(
                    paragraph
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "statements",
                list(
                    paragraph
                        .statements
                        .into_iter()
                        .map(paragraph_statement_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn paragraph_statement_tree(statement: ParagraphStatementSyntax) -> SyntaxValue {
    node(
        "ParagraphStatement",
        vec![
            field("i", maybe_word(statement.i)),
            field(
                "connective",
                statement
                    .connective
                    .map_or_else(nothing, |connective| just(connective_tree(connective))),
            ),
            field(
                "freeModifiers",
                list(
                    statement
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "statement",
                statement
                    .statement
                    .map_or_else(nothing, |statement| just(statement_tree(statement))),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn free_modifier_tree(free_modifier: FreeModifierSyntax) -> SyntaxValue {
    match free_modifier {
        FreeModifierSyntax::Sei {
            sei,
            leading_free_modifiers,
            terms,
            cu,
            cu_free_modifiers,
            relation,
            sehu,
            sehu_free_modifiers,
        } => node(
            "SeiFree",
            vec![
                field("sei", word_value(sei)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "terms",
                    if terms.is_empty() {
                        nothing()
                    } else {
                        just(list(terms.into_iter().map(term_tree).collect()))
                    },
                ),
                field("cu", maybe_word(cu)),
                field(
                    "cuFreeModifiers",
                    list(
                        cu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field("sehu", maybe_word(sehu)),
                field(
                    "sehuFreeModifiers",
                    list(
                        sehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::To {
            to,
            free_modifiers,
            text,
            toi,
            toi_free_modifiers,
        } => node(
            "ToFree",
            vec![
                field("to", word_value(to)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("text", lojban_text_tree(*text)),
                field("toi", maybe_word(toi)),
                field(
                    "toiFreeModifiers",
                    list(
                        toi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::Xi {
            xi,
            free_modifiers,
            expression,
        } => node(
            "XiFree",
            vec![
                field("xi", word_value(xi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("mathExpression", math_expression_tree(expression)),
            ],
        ),
        FreeModifierSyntax::Mai {
            number,
            mai,
            free_modifiers,
        } => node(
            "MaiFree",
            vec![
                field("number", nonempty_number_words(number)),
                field("mai", word_value(mai)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FreeModifierSyntax::Soi {
            soi,
            free_modifiers,
            leading_argument,
            trailing_argument,
            sehu,
            sehu_free_modifiers,
        } => node(
            "SoiFree",
            vec![
                field("soi", word_value(soi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("leadingArgument", argument_tree(*leading_argument)),
                field(
                    "trailingArgument",
                    trailing_argument
                        .map_or_else(nothing, |argument| just(argument_tree(*argument))),
                ),
                field("sehu", maybe_word(sehu)),
                field(
                    "sehuFreeModifiers",
                    list(
                        sehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FreeModifierSyntax::Vocative {
            vocative_markers,
            free_modifiers,
            argument,
            dohu,
            dohu_free_modifiers,
        } => node(
            "VocativeFree",
            vec![
                field(
                    "vocativeMarkers",
                    list(
                        vocative_markers
                            .into_iter()
                            .map(vocative_marker_value)
                            .collect(),
                    ),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "argument",
                    argument.map_or_else(nothing, |argument| just(argument_tree(argument))),
                ),
                field("dohu", maybe_word(dohu)),
                field(
                    "dohuFreeModifiers",
                    list(
                        dohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn statement_tree(statement: StatementSyntax) -> SyntaxValue {
    match statement {
        StatementSyntax::Tuhe {
            tense_modal,
            tuhe,
            tuhe_free_modifiers,
            text,
            tuhu,
            tuhu_free_modifiers,
        } => node(
            "TuheStatement",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("tuhe", word_value(tuhe)),
                field(
                    "tuheFreeModifiers",
                    list(
                        tuhe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("paragraphs", paragraphs_tree(*text)),
                field("tuhu", maybe_word(tuhu)),
                field(
                    "tuhuFreeModifiers",
                    list(
                        tuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        StatementSyntax::Prenex {
            prenex_terms,
            zohu,
            zohu_free_modifiers,
            inner_statement,
        } => node(
            "PrenexStatement",
            vec![
                field(
                    "prenexTerms",
                    list(prenex_terms.into_iter().map(term_tree).collect()),
                ),
                field("zohu", word_value(zohu)),
                field(
                    "zohuFreeModifiers",
                    list(
                        zohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerStatement", statement_tree(*inner_statement)),
            ],
        ),
        StatementSyntax::Predicate(predicate) => node(
            "StatementPredicate",
            vec![field("predicate", predicate_tree(predicate))],
        ),
        StatementSyntax::Connected {
            i,
            connective,
            leading_statement,
            trailing_statement,
        } => node(
            "ConnectedStatement",
            vec![
                field("i", word_value(i)),
                field("connective", connective_tree(connective)),
                field("leadingStatement", statement_tree(*leading_statement)),
                field("trailingStatement", statement_tree(*trailing_statement)),
            ],
        ),
        StatementSyntax::PreIConnected {
            connective,
            i,
            leading_statement,
            trailing_statement,
        } => node(
            "PreIConnectedStatement",
            vec![
                field("connective", connective_tree(connective)),
                field("i", word_value(i)),
                field("leadingStatement", statement_tree(*leading_statement)),
                field("trailingStatement", statement_tree(*trailing_statement)),
            ],
        ),
        StatementSyntax::Iau {
            inner_statement,
            iau,
            iau_free_modifiers,
            reset_terms,
        } => node(
            "IauStatement",
            vec![
                field("innerStatement", statement_tree(*inner_statement)),
                field("iau", word_value(iau)),
                field(
                    "iauFreeModifiers",
                    list(
                        iau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "resetTerms",
                    list(reset_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        StatementSyntax::ExperimentalPredicateContinuation {
            leading_statement,
            continuation,
        } => node(
            "ExperimentalPredicateContinuationStatement",
            vec![
                field("leadingStatement", statement_tree(*leading_statement)),
                field(
                    "continuation",
                    predicate_statement_continuation_tree(continuation),
                ),
            ],
        ),
        StatementSyntax::Fragment(fragment) => node(
            "StatementFragment",
            vec![field("fragment", fragment_tree(fragment))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_continuation_tree(
    continuation: PredicateStatementContinuationSyntax,
) -> SyntaxValue {
    node(
        "PredicateStatementContinuation",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field(
                "marker",
                predicate_statement_continuation_marker_tree(continuation.marker),
            ),
            field(
                "trailingSubsentence",
                subsentence_tree(continuation.trailing_subsentence),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_statement_continuation_marker_tree(
    marker: PredicateStatementContinuationMarkerSyntax,
) -> SyntaxValue {
    match marker {
        PredicateStatementContinuationMarkerSyntax::Bo { bo, free_modifiers } => node(
            "PredicateStatementBo",
            vec![
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        PredicateStatementContinuationMarkerSyntax::Ke {
            ke,
            ke_free_modifiers,
            kehe,
            kehe_free_modifiers,
        } => node(
            "PredicateStatementKe",
            vec![
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn fragment_tree(fragment: FragmentSyntax) -> SyntaxValue {
    match fragment {
        FragmentSyntax::Argument { argument } => node(
            "ArgumentFragment",
            vec![field("argument", argument_tree(argument))],
        ),
        FragmentSyntax::Ek {
            connective,
            free_modifiers,
        } => node(
            "EkFragment",
            vec![
                field("connective", connective_tree(connective)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Gihek {
            connective,
            free_modifiers,
        } => node(
            "GihekFragment",
            vec![
                field("connective", connective_tree(connective)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Other {
            words,
            free_modifiers,
        } => node(
            "OtherFragment",
            vec![
                field(
                    "otherWords",
                    list(words.into_iter().map(word_value).collect()),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        FragmentSyntax::Vocative {
            vocative_markers,
            free_modifiers,
            vocative_argument,
            dohu,
            dohu_free_modifiers,
        } => node(
            "VocativeFragment",
            vec![
                field(
                    "vocativeMarkers",
                    list(
                        vocative_markers
                            .into_iter()
                            .map(vocative_marker_value)
                            .collect(),
                    ),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "vocativeArgument",
                    vocative_argument
                        .map_or_else(nothing, |argument| just(argument_tree(argument))),
                ),
                field("dohu", maybe_word(dohu)),
                field(
                    "dohuFreeModifiers",
                    list(
                        dohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::Ijek { i, connective } => node(
            "IjekFragment",
            vec![
                field("i", word_value(i)),
                field("connective", connective_tree(connective)),
            ],
        ),
        FragmentSyntax::Prenex {
            terms,
            zohu,
            zohu_free_modifiers,
        } => node(
            "PrenexFragment",
            vec![
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("zohu", word_value(zohu)),
                field(
                    "zohuFreeModifiers",
                    list(
                        zohu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::BeLink {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
        } => node(
            "BeLinkFragment",
            vec![
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::BeiLink { bei_only_links } => node(
            "BeiLinkFragment",
            vec![field(
                "beiOnlyLinks",
                list(bei_only_links.into_iter().map(bei_link_tree).collect()),
            )],
        ),
        FragmentSyntax::RelativeClause { relative_clauses } => node(
            "RelativeClauseFragment",
            vec![field(
                "relativeClauses",
                list(
                    relative_clauses
                        .into_iter()
                        .map(relative_clause_tree)
                        .collect(),
                ),
            )],
        ),
        FragmentSyntax::MathExpression { math_expression } => node(
            "MathExpressionFragment",
            vec![field(
                "mathExpression",
                math_expression_tree(math_expression),
            )],
        ),
        FragmentSyntax::Term {
            terms,
            vau,
            vau_free_modifiers,
        } => node(
            "TermFragment",
            vec![
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("vau", maybe_word(vau)),
                field(
                    "vauFreeModifiers",
                    list(
                        vau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        FragmentSyntax::Relation { relation } => node(
            "RelationFragment",
            vec![field("relation", relation_tree(relation))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause_tree(relative_clause: GoiRelativeClauseSyntax) -> SyntaxValue {
    node(
        "GoiRelativeClause",
        vec![
            field("goi", word_value(relative_clause.goi)),
            field(
                "leadingFreeModifiers",
                list(
                    relative_clause
                        .leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("argument", argument_tree(relative_clause.argument)),
            field("gehu", maybe_word(relative_clause.gehu)),
            field(
                "trailingFreeModifiers",
                list(
                    relative_clause
                        .trailing_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tree(predicate: BasicPredicate) -> SyntaxValue {
    let predicate_tail = predicate_tail_tree(predicate.clone());
    node(
        "Predicate",
        vec![
            field(
                "leadingTerms",
                list(predicate.leading_terms.into_iter().map(term_tree).collect()),
            ),
            field("cu", maybe_word(predicate.cu)),
            field(
                "cuFreeModifiers",
                list(
                    predicate
                        .cu_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("predicateTail", predicate_tail),
            field(
                "freeModifiers",
                list(
                    predicate
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_tree(predicate: BasicPredicate) -> SyntaxValue {
    let ke_continuation = predicate.ke_continuation.clone();
    node(
        "PredicateTail",
        vec![
            field(
                "first",
                node(
                    "PredicateTail1",
                    vec![
                        field("first", predicate_tail2_tree(predicate.clone())),
                        field(
                            "continuations",
                            list(
                                predicate
                                    .continuations
                                    .into_iter()
                                    .map(predicate_tail_continuation_tree)
                                    .collect(),
                            ),
                        ),
                    ],
                ),
            ),
            field(
                "keContinuation",
                ke_continuation.map_or_else(nothing, |ke_continuation| {
                    just(predicate_tail_ke_continuation_tree(ke_continuation))
                }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2_tree(predicate: BasicPredicate) -> SyntaxValue {
    let tail3 = predicate.gek_sentence.map_or_else(
        || {
            node(
                "RelationPredicateTail3",
                vec![
                    field("relation", relation_tree(predicate.relation)),
                    field(
                        "terms",
                        list(predicate.tail_terms.into_iter().map(term_tree).collect()),
                    ),
                    field("vau", maybe_word(predicate.vau)),
                    field(
                        "freeModifiers",
                        list(
                            predicate
                                .tail_free_modifiers
                                .into_iter()
                                .map(free_modifier_tree)
                                .collect(),
                        ),
                    ),
                ],
            )
        },
        |gek_sentence| {
            node(
                "GekSentencePredicateTail3",
                vec![field("gekSentence", gek_sentence_tree(gek_sentence))],
            )
        },
    );
    node(
        "PredicateTail2",
        vec![
            field("first", tail3),
            field(
                "boContinuation",
                predicate
                    .bo_continuation
                    .map_or_else(nothing, |bo_continuation| {
                        just(predicate_tail_bo_continuation_tree(bo_continuation))
                    }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_bo_continuation_tree(
    continuation: PredicateTailBoContinuationSyntax,
) -> SyntaxValue {
    node(
        "BoPredicateTail",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field("bo", word_value(continuation.bo)),
            field(
                "freeModifiers",
                list(
                    continuation
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("cu", maybe_word(continuation.cu)),
            field(
                "cuFreeModifiers",
                list(
                    continuation
                        .cu_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "predicateTail",
                predicate_tail2_tree(*continuation.predicate_tail),
            ),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.vau)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_ke_continuation_tree(
    continuation: PredicateTailKeContinuationSyntax,
) -> SyntaxValue {
    node(
        "KePredicateTail",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field("ke", word_value(continuation.ke)),
            field(
                "keFreeModifiers",
                list(
                    continuation
                        .ke_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "predicateTail",
                predicate_tail_tree(*continuation.predicate_tail),
            ),
            field("kehe", maybe_word(continuation.kehe)),
            field(
                "keheFreeModifiers",
                list(
                    continuation
                        .kehe_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.vau)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn gek_sentence_tree(gek_sentence: GekSentenceSyntax) -> SyntaxValue {
    match gek_sentence {
        GekSentenceSyntax::Pair {
            gek,
            first,
            gik,
            second,
            tail_terms,
            vau,
            free_modifiers,
        } => node(
            "GekSentencePair",
            vec![
                field("gek", connective_tree(gek)),
                field("first", subsentence_tree(*first)),
                field("gik", connective_tree(gik)),
                field("second", subsentence_tree(*second)),
                field(
                    "tailTerms",
                    list(tail_terms.into_iter().map(term_tree).collect()),
                ),
                field("vau", maybe_word(vau)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        GekSentenceSyntax::Ke {
            tense_modal,
            ke,
            ke_free_modifiers,
            inner,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeGekSentence",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("inner", gek_sentence_tree(*inner)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        GekSentenceSyntax::Na {
            na,
            free_modifiers,
            inner,
        } => node(
            "NaGekSentence",
            vec![
                field("na", word_value(na)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("inner", gek_sentence_tree(*inner)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_continuation_tree(continuation: PredicateTailContinuationSyntax) -> SyntaxValue {
    node(
        "PredicateTailContinuation",
        vec![
            field("connective", connective_tree(continuation.connective)),
            field(
                "tenseModal",
                continuation
                    .tense_modal
                    .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
            ),
            field("cu", maybe_word(continuation.cu)),
            field(
                "cuFreeModifiers",
                list(
                    continuation
                        .cu_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "predicateTail",
                node(
                    "PredicateTail2",
                    vec![
                        field(
                            "first",
                            node(
                                "RelationPredicateTail3",
                                vec![
                                    field("relation", relation_tree(continuation.relation)),
                                    field(
                                        "terms",
                                        list(
                                            continuation.terms.into_iter().map(term_tree).collect(),
                                        ),
                                    ),
                                    field("vau", maybe_word(continuation.vau)),
                                    field(
                                        "freeModifiers",
                                        list(
                                            continuation
                                                .free_modifiers
                                                .into_iter()
                                                .map(free_modifier_tree)
                                                .collect(),
                                        ),
                                    ),
                                ],
                            ),
                        ),
                        field(
                            "boContinuation",
                            continuation
                                .bo_continuation
                                .map_or_else(nothing, |bo_continuation| {
                                    just(predicate_tail_bo_continuation_tree(bo_continuation))
                                }),
                        ),
                    ],
                ),
            ),
            field(
                "tailTerms",
                list(continuation.tail_terms.into_iter().map(term_tree).collect()),
            ),
            field("vau", maybe_word(continuation.tail_vau)),
            field("freeModifiers", nil()),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn term_tree(term: TermSyntax) -> SyntaxValue {
    match term {
        TermSyntax::NuhiTermset {
            nuhi,
            nuhi_free_modifiers,
            termset,
            nuhu,
            nuhu_free_modifiers,
        } => node(
            "NuhiTermset",
            vec![
                field("nuhi", word_value(nuhi)),
                field(
                    "nuhiFreeModifiers",
                    list(
                        nuhi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "termset",
                    list(termset.into_iter().map(term_tree).collect()),
                ),
                field("nuhu", maybe_word(nuhu)),
                field(
                    "nuhuFreeModifiers",
                    list(
                        nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::GekNuhiTermset {
            m_nuhi,
            nuhi_free_modifiers,
            gek,
            terms,
            nuhu,
            nuhu_free_modifiers,
            gik,
            gik_terms,
            gik_nuhu,
            gik_nuhu_free_modifiers,
        } => node(
            "GekNuhiTermset",
            vec![
                field("mNuhi", maybe_word(m_nuhi)),
                field(
                    "nuhiFreeModifiers",
                    list(
                        nuhi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("gek", connective_tree(gek)),
                field("terms", list(terms.into_iter().map(term_tree).collect())),
                field("nuhu", maybe_word(nuhu)),
                field(
                    "nuhuFreeModifiers",
                    list(
                        nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("gik", connective_tree(gik)),
                field(
                    "gikTerms",
                    list(gik_terms.into_iter().map(term_tree).collect()),
                ),
                field("gikNuhu", maybe_word(gik_nuhu)),
                field(
                    "gikNuhuFreeModifiers",
                    list(
                        gik_nuhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::Cehe {
            leading_terms,
            cehe,
            free_modifiers,
            trailing_terms,
        } => node(
            "CeheTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("cehe", word_value(cehe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::Pehe {
            leading_terms,
            pehe,
            free_modifiers,
            connective,
            trailing_terms,
        } => node(
            "PeheTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("pehe", word_value(pehe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("connective", connective_tree(connective)),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::Argument(argument) => node(
            "ArgumentTerm",
            vec![field("argument", argument_tree(argument))],
        ),
        TermSyntax::Fa {
            fa,
            free_modifiers,
            argument,
            ku,
            ku_free_modifiers,
        } => node(
            "FaTerm",
            vec![
                field("fa", word_value(fa)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("argument", argument_tree(argument)),
                field("ku", maybe_word(ku)),
                field(
                    "kuFreeModifiers",
                    list(
                        ku_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::NaKu {
            na,
            na_ku,
            free_modifiers,
        } => node(
            "NaKuTerm",
            vec![
                field("na", word_value(na)),
                field("naKu", word_value(na_ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        TermSyntax::BareNa { na, free_modifiers } => node(
            "BareNaTerm",
            vec![
                field("na", word_value(na)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        TermSyntax::NoihaAdverbial {
            noiha,
            leading_free_modifiers,
            tail_elements,
            relation,
            relative_clauses,
            fehu,
            trailing_free_modifiers,
        } => node(
            "NoihaAdverbialTerm",
            vec![
                field("noiha", word_value(noiha)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "tailElements",
                    list(
                        tail_elements
                            .into_iter()
                            .map(argument_tail_element_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relation",
                    relation.map_or_else(nothing, |relation| just(relation_tree(relation))),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("fehu", maybe_word(fehu)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::PoihaBrigahi {
            poiha,
            leading_free_modifiers,
            tail_elements,
            relation,
            relative_clauses,
            brigahi_ku,
            trailing_free_modifiers,
        } => node(
            "PoihaBrigahiTerm",
            vec![
                field("poiha", word_value(poiha)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "tailElements",
                    list(
                        tail_elements
                            .into_iter()
                            .map(argument_tail_element_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relation",
                    relation.map_or_else(nothing, |relation| just(relation_tree(relation))),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("brigahiKu", word_value(brigahi_ku)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::FihoiAdverbial {
            fihoi,
            leading_free_modifiers,
            subsentence,
            fihau,
            trailing_free_modifiers,
        } => node(
            "FihoiAdverbialTerm",
            vec![
                field("fihoi", word_value(fihoi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("fihau", maybe_word(fihau)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::SoiAdverbial {
            soi,
            leading_free_modifiers,
            subsentence,
            sehu,
            trailing_free_modifiers,
        } => node(
            "SoiAdverbialTerm",
            vec![
                field("soi", word_value(soi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("sehu", maybe_word(sehu)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        TermSyntax::Tagged {
            tense_modal,
            free_modifiers,
            argument,
        } => node(
            "TaggedTerm",
            vec![
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("argument", argument_tree(argument)),
            ],
        ),
        TermSyntax::Connected {
            leading_terms,
            connective,
            trailing_terms,
        } => node(
            "ConnectedTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field("connective", connective_tree(connective)),
                field(
                    "trailingTerms",
                    list(trailing_terms.into_iter().map(term_tree).collect()),
                ),
            ],
        ),
        TermSyntax::BoConnected {
            leading_terms,
            bo_connective,
            tense_modal,
            bo,
            free_modifiers,
            trailing_term,
        } => node(
            "BoConnectedTerm",
            vec![
                field(
                    "leadingTerms",
                    list(leading_terms.into_iter().map(term_tree).collect()),
                ),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingTerm", term_tree(*trailing_term)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn term_wrapper_kind_tree(kind: TermWrapperKindSyntax) -> SyntaxValue {
    match kind {
        TermWrapperKindSyntax::Lahe => node("LaheTermWrapper", Vec::new()),
        TermWrapperKindSyntax::NaheBo => node("NaheBoTermWrapper", Vec::new()),
        TermWrapperKindSyntax::Nahe => node("NaheTermWrapper", Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_tree(argument: ArgumentSyntax) -> SyntaxValue {
    match argument {
        ArgumentSyntax::Quote { quote } => node(
            "QuoteArgument",
            vec![
                field("quote", quote_tree(quote)),
                field("freeModifiers", nil()),
            ],
        ),
        ArgumentSyntax::MathExpression {
            li,
            li_free_modifiers,
            expression,
            loho,
            loho_free_modifiers,
        } => node(
            "MathExpressionArgument",
            vec![
                field("li", word_value(li)),
                field(
                    "liFreeModifiers",
                    list(
                        li_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("mathExpression", math_expression_tree(expression)),
                field("loho", maybe_word(loho)),
                field(
                    "lohoFreeModifiers",
                    list(
                        loho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Letter {
            letter,
            boi,
            boi_free_modifiers,
        } => node(
            "LetterArgument",
            vec![
                field("letter", nonempty_letter_words(letter)),
                field("boi", maybe_word(boi)),
                field(
                    "boiFreeModifiers",
                    list(
                        boi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Quantified {
            quantifier,
            inner_argument,
        } => node(
            "QuantifiedArgument",
            vec![
                field("quantifier", quantifier_expression_tree(quantifier)),
                field("innerArgument", argument_tree(*inner_argument)),
            ],
        ),
        ArgumentSyntax::RelativeClause {
            base_argument,
            vuho,
            vuho_free_modifiers,
            relative_clauses,
        } => node(
            "RelativeClauseArgument",
            vec![
                field("baseArgument", argument_tree(*base_argument)),
                field("vuho", maybe_word(vuho)),
                field(
                    "vuhoFreeModifiers",
                    list(
                        vuho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Vuho {
            base_argument,
            vuho_marker,
            vuho_free_modifiers,
            relative_clauses,
            connected_argument,
        } => node(
            "VuhoArgument",
            vec![
                field("baseArgument", argument_tree(*base_argument)),
                field("vuhoMarker", word_value(vuho_marker)),
                field(
                    "vuhoFreeModifiers",
                    list(
                        vuho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field(
                    "relativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field(
                    "connectedArgument",
                    connected_argument.map_or_else(nothing, |connected_argument| {
                        just(node(
                            "(,)",
                            vec![
                                unnamed_field(connective_tree(connected_argument.connective)),
                                unnamed_field(argument_tree(*connected_argument.argument)),
                            ],
                        ))
                    }),
                ),
            ],
        ),
        ArgumentSyntax::BridiDescription {
            lohoi,
            lohoi_free_modifiers,
            subsentence,
            kuhau,
            kuhau_free_modifiers,
        } => node(
            "BridiDescriptionArgument",
            vec![
                field("lohoi", word_value(lohoi)),
                field(
                    "lohoiFreeModifiers",
                    list(
                        lohoi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(*subsentence)),
                field("kuhau", maybe_word(kuhau)),
                field(
                    "kuhauFreeModifiers",
                    list(
                        kuhau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::NaKu {
            na,
            ku,
            free_modifiers,
        } => node(
            "NaKuArgument",
            vec![
                field("na", word_value(na)),
                field("ku", word_value(ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Tagged {
            tag_words,
            tag_tense_modal,
            tag_fa,
            free_modifiers,
            inner_argument,
        } => node(
            "TaggedArgument",
            vec![
                field(
                    "tagWords",
                    list(tag_words.into_iter().map(word_value).collect()),
                ),
                field(
                    "tagTenseModal",
                    tag_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("tagFa", maybe_word(tag_fa)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
            ],
        ),
        ArgumentSyntax::NaheBo {
            nahe,
            bo,
            free_modifiers,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "NaheBoArgument",
            vec![
                field("nahe", word_value(nahe)),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Nahe {
            nahe,
            free_modifiers,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "NaheArgument",
            vec![
                field("nahe", word_value(nahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::TermWrapped {
            term_wrapper_kind,
            wrapper,
            wrapper_bo,
            free_modifiers,
            inner_term,
            luhu,
            luhu_free_modifiers,
        } => node(
            "TermWrappedArgument",
            vec![
                field("termWrapperKind", term_wrapper_kind_tree(term_wrapper_kind)),
                field("wrapper", word_value(wrapper)),
                field("wrapperBo", maybe_word(wrapper_bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerTerm", term_tree(*inner_term)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Koha {
            koha,
            free_modifiers,
        } => node(
            "KohaArgument",
            vec![
                field("koha", word_value(koha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Zohe {
            tag_words,
            maybe_ku,
            free_modifiers,
        } => node(
            "ZoheArgument",
            vec![
                field(
                    "tagWords",
                    list(tag_words.into_iter().map(word_value).collect()),
                ),
                field("maybeKu", maybe_word(maybe_ku)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::Lahe {
            lahe,
            free_modifiers,
            relative_clauses,
            inner_argument,
            luhu,
            luhu_free_modifiers,
        } => node(
            "LaheArgument",
            vec![
                field("lahe", word_value(lahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "laheRelativeClauses",
                    list(
                        relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("luhu", maybe_word(luhu)),
                field(
                    "luhuFreeModifiers",
                    list(
                        luhu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Connected {
            leading_argument,
            connective,
            trailing_argument,
        } => node(
            "ConnectedArgument",
            vec![
                field("leadingArgument", argument_tree(*leading_argument)),
                field("connective", connective_tree(connective)),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Ke {
            ke,
            ke_free_modifiers,
            inner_argument,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeArgument",
            vec![
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerArgument", argument_tree(*inner_argument)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Bo {
            leading_argument,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_argument,
        } => node(
            "BoArgument",
            vec![
                field("leadingArgument", argument_tree(*leading_argument)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Gek {
            gek,
            leading_argument,
            gik,
            trailing_argument,
        } => node(
            "GekArgument",
            vec![
                field("gek", connective_tree(gek)),
                field("leadingArgument", argument_tree(*leading_argument)),
                field("gik", connective_tree(gik)),
                field("trailingArgument", argument_tree(*trailing_argument)),
            ],
        ),
        ArgumentSyntax::Descriptor { descriptor } => node(
            "DescriptorArgument",
            vec![field("descriptor", descriptor_tree(descriptor))],
        ),
        ArgumentSyntax::Name {
            la,
            la_free_modifiers,
            names,
            name_free_modifiers,
        } => node(
            "NameArgument",
            vec![
                field("la", gadri_word_value(la)),
                field(
                    "laFreeModifiers",
                    list(
                        la_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("names", nonempty_name_words(names)),
                field(
                    "nameFreeModifiers",
                    list(
                        name_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        ArgumentSyntax::Cmevla {
            cmevla,
            free_modifiers,
        } => node(
            "CmevlaArgument",
            vec![
                field("cmevla", nonempty_name_words(cmevla)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        ArgumentSyntax::RelationVocative {
            leading_relative_clauses,
            relation,
            trailing_relative_clauses,
        } => node(
            "RelationVocativeArgument",
            vec![
                field(
                    "leadingRelativeClauses",
                    list(
                        leading_relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field(
                    "trailingRelativeClauses",
                    list(
                        trailing_relative_clauses
                            .into_iter()
                            .map(relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn subsentence_tree(subsentence: SubsentenceSyntax) -> SyntaxValue {
    match subsentence {
        SubsentenceSyntax::Plain(predicate) => node(
            "PlainSubsentence",
            vec![unnamed_field(predicate_tree(predicate))],
        ),
        SubsentenceSyntax::Prenex {
            prenex_terms,
            zohu,
            zohu_free_modifiers,
            inner_subsentence,
        } => node(
            "PrenexSubsentence",
            vec![
                unnamed_field(node(
                    "Prenex",
                    vec![
                        field(
                            "terms",
                            list(prenex_terms.into_iter().map(term_tree).collect()),
                        ),
                        field("zohu", word_value(zohu)),
                        field(
                            "zohuFreeModifiers",
                            list(
                                zohu_free_modifiers
                                    .into_iter()
                                    .map(free_modifier_tree)
                                    .collect(),
                            ),
                        ),
                    ],
                )),
                unnamed_field(subsentence_tree(*inner_subsentence)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_tree(relative_clause: RelativeClauseSyntax) -> SyntaxValue {
    match relative_clause {
        RelativeClauseSyntax::Goi(relative_clause) => goi_relative_clause_tree(relative_clause),
        RelativeClauseSyntax::Noi {
            noi,
            leading_free_modifiers,
            subsentence,
            kuho,
            trailing_free_modifiers,
        } => node(
            "NoiRelativeClause",
            vec![
                field("noi", word_value(noi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(subsentence)),
                field("kuho", maybe_word(kuho)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelativeClauseSyntax::Poi {
            poi,
            leading_free_modifiers,
            subsentence,
            kuho,
            trailing_free_modifiers,
        } => node(
            "PoiRelativeClause",
            vec![
                field("poi", word_value(poi)),
                field(
                    "leadingFreeModifiers",
                    list(
                        leading_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("subsentence", subsentence_tree(subsentence)),
                field("kuho", maybe_word(kuho)),
                field(
                    "trailingFreeModifiers",
                    list(
                        trailing_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelativeClauseSyntax::Zihe {
            zihe,
            free_modifiers,
            inner,
        } => node(
            "ZiheRelativeClause",
            vec![
                field("zihe", word_value(zihe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("inner", relative_clause_tree(*inner)),
            ],
        ),
        RelativeClauseSyntax::Connected { connective, inner } => node(
            "ConnectedRelativeClause",
            vec![
                field("connective", connective_tree(connective)),
                field("inner", relative_clause_tree(*inner)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_relative_clause_tree(relative_clause: SelbriRelativeClauseSyntax) -> SyntaxValue {
    node(
        "SelbriRelativeClause",
        vec![
            field("nohoi", word_value(relative_clause.nohoi)),
            field(
                "leadingFreeModifiers",
                list(
                    relative_clause
                        .leading_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("relation", relation_tree(relative_clause.relation)),
            field("kuhoi", maybe_word(relative_clause.kuhoi)),
            field(
                "trailingFreeModifiers",
                list(
                    relative_clause
                        .trailing_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn quote_tree(quote: QuoteSyntax) -> SyntaxValue {
    match quote {
        QuoteSyntax::Lu {
            lu,
            free_modifiers,
            text,
            lihu,
            lihu_free_modifiers,
        } => node(
            "LuQuote",
            vec![
                field("lu", word_value(lu)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("text", lojban_text_tree(text)),
                field("lihu", maybe_word(lihu)),
                field(
                    "lihuFreeModifiers",
                    list(
                        lihu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        QuoteSyntax::Zo {
            zo,
            word,
            free_modifiers,
        } => node(
            "ZoQuote",
            vec![
                field("zo", word_value(zo)),
                field("word", word_value(word)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::ZohOi {
            zohoi,
            quoted_text,
            free_modifiers,
        } => node(
            "ZohOiQuote",
            vec![
                field("zohoi", word_value(zohoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "ZoiQuote",
            vec![
                field("zoi", word_value(zoi)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Laho {
            laho,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "LahoQuote",
            vec![
                field("laho", word_value(laho)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            lehu,
            lehu_free_modifiers,
        } => node(
            "LohuQuote",
            vec![
                field("lohu", word_value(lohu)),
                field(
                    "quotedWords",
                    list(quoted_words.into_iter().map(word_value).collect()),
                ),
                field("lehu", word_value(lehu)),
                field(
                    "lehuFreeModifiers",
                    list(
                        lehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn descriptor_tree(descriptor: DescriptorSyntax) -> SyntaxValue {
    node(
        "Descriptor",
        vec![
            field(
                "descriptor",
                descriptor
                    .descriptor
                    .map_or_else(nothing, |descriptor| just(word_value(descriptor))),
            ),
            field(
                "descriptorFreeModifiers",
                list(
                    descriptor
                        .descriptor_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "outerQuantifier",
                descriptor
                    .outer_quantifier
                    .map_or_else(nothing, |quantifier| just(quantifier_tree(quantifier))),
            ),
            field(
                "tailElements",
                list(
                    descriptor
                        .tail_elements
                        .into_iter()
                        .map(argument_tail_element_tree)
                        .collect(),
                ),
            ),
            field(
                "relation",
                descriptor
                    .relation
                    .map_or_else(nothing, |relation| just(relation_tree(relation))),
            ),
            field(
                "relativeClauses",
                list(
                    descriptor
                        .relative_clauses
                        .into_iter()
                        .map(relative_clause_tree)
                        .collect(),
                ),
            ),
            field("ku", maybe_word(descriptor.ku)),
            field(
                "kuFreeModifiers",
                list(
                    descriptor
                        .ku_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn connective_tree(connective: ConnectiveSyntax) -> SyntaxValue {
    let kind = match connective.kind {
        ConnectiveKind::Afterthought => "AfterthoughtConnective",
        ConnectiveKind::Relation => "RelationConnective",
        ConnectiveKind::PredicateTail => "PredicateTailConnective",
        ConnectiveKind::Forethought => "ForethoughtConnective",
        ConnectiveKind::NonLogical => "NonLogicalConnective",
        ConnectiveKind::Interval => "IntervalConnective",
    };

    node(
        "Connective",
        vec![
            field("kind", node(kind, Vec::new())),
            field("se", maybe_word(connective.se)),
            field("nahe", maybe_word(connective.nahe)),
            field("na", maybe_word(connective.na)),
            field(
                "cmavo",
                list(connective.cmavo.into_iter().map(word_value).collect()),
            ),
            field("nai", maybe_word(connective.nai)),
            field(
                "freeModifiers",
                list(
                    connective
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_element_tree(element: ArgumentTailElementSyntax) -> SyntaxValue {
    match element {
        ArgumentTailElementSyntax::Argument(argument) => node(
            "ArgumentTailArgument",
            vec![unnamed_field(argument_tree(*argument))],
        ),
        ArgumentTailElementSyntax::RelativeClauses(relative_clauses) => node(
            "ArgumentTailRelativeClauses",
            vec![unnamed_field(list(
                relative_clauses
                    .into_iter()
                    .map(relative_clause_tree)
                    .collect(),
            ))],
        ),
        ArgumentTailElementSyntax::Quantifier(quantifier) => node(
            "ArgumentTailQuantifier",
            vec![unnamed_field(quantifier_tree(quantifier))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_tree(quantifier: QuantifierSyntax) -> SyntaxValue {
    match quantifier {
        QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers,
        } => node(
            "NumberQuantifier",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field(
                    "boiFreeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuantifierSyntax::Vei {
            vei,
            math_expression,
            veho,
        } => node(
            "VeiQuantifier",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("mathExpression", math_expression_tree(*math_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_expression_tree(quantifier: QuantifierSyntax) -> SyntaxValue {
    match quantifier {
        QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers,
        } => node(
            "NumberExpression",
            vec![
                field("number", nonempty_number_words(number)),
                field("boi", maybe_word(boi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        QuantifierSyntax::Vei {
            vei,
            math_expression,
            veho,
        } => node(
            "VeiExpression",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*math_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn math_expression_tree(expression: MathExpressionSyntax) -> SyntaxValue {
    match expression {
        MathExpressionSyntax::Number(quantifier) => quantifier_expression_tree(quantifier),
        MathExpressionSyntax::Letter { letter, boi } => node(
            "LetterExpression",
            vec![
                field("letter", nonempty_letter_words(letter)),
                field("boi", maybe_word(boi)),
                field("freeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Vei {
            vei,
            inner_expression,
            veho,
        } => node(
            "VeiExpression",
            vec![
                field("vei", word_value(vei)),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*inner_expression)),
                field("veho", maybe_word(veho)),
                field("vehoFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Gek {
            gek,
            left_expression,
            gik,
            right_expression,
        } => node(
            "GekExpression",
            vec![
                field("gek", connective_tree(gek)),
                field("leftExpression", math_expression_tree(*left_expression)),
                field("gik", connective_tree(gik)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Forethought {
            peho,
            operator,
            operands,
            kuhe,
        } => node(
            "ForethoughtExpression",
            vec![
                field("peho", maybe_word(peho)),
                field("freeModifiers", nil()),
                field("operator", math_operator_tree(operator)),
                field(
                    "operands",
                    list(operands.into_iter().map(math_expression_tree).collect()),
                ),
                field("kuhe", maybe_word(kuhe)),
                field("kuheFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::ReversePolish {
            fuha,
            free_modifiers,
            operands,
            operators,
        } => node(
            "ReversePolishExpression",
            vec![
                field("fuha", word_value(fuha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "operands",
                    list(operands.into_iter().map(math_expression_tree).collect()),
                ),
                field(
                    "operators",
                    list(operators.into_iter().map(math_operator_tree).collect()),
                ),
            ],
        ),
        MathExpressionSyntax::Nihe {
            nihe,
            relation,
            tehu,
        } => node(
            "NiheExpression",
            vec![
                field("nihe", word_value(nihe)),
                field("freeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Mohe {
            mohe,
            argument,
            tehu,
        } => node(
            "MoheExpression",
            vec![
                field("mohe", word_value(mohe)),
                field("freeModifiers", nil()),
                field("argument", argument_tree(*argument)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Johi {
            johi,
            free_modifiers,
            expressions,
            tehu,
            tehu_free_modifiers,
        } => node(
            "JohiExpression",
            vec![
                field("johi", word_value(johi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("expressions", nonempty_math_expressions(expressions)),
                field("tehu", maybe_word(tehu)),
                field(
                    "tehuFreeModifiers",
                    list(
                        tehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        MathExpressionSyntax::Lahe {
            markers,
            inner_expression,
            luhu,
        } => node(
            "LaheExpression",
            vec![
                field(
                    "markers",
                    list(markers.into_iter().map(word_value).collect()),
                ),
                field("freeModifiers", nil()),
                field("innerExpression", math_expression_tree(*inner_expression)),
                field("luhu", maybe_word(luhu)),
                field("luhuFreeModifiers", nil()),
            ],
        ),
        MathExpressionSyntax::Connected {
            left_expression,
            connective,
            right_expression,
        } => node(
            "ConnectedExpression",
            vec![
                field("leftExpression", math_expression_tree(*left_expression)),
                field("connective", connective_tree(connective)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Binary {
            operator,
            left_expression,
            right_expression,
        } => node(
            "BinaryExpression",
            vec![
                field("operator", math_operator_tree(operator)),
                field("leftExpression", math_expression_tree(*left_expression)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
        MathExpressionSyntax::Bihe {
            left_expression,
            bihe,
            operator,
            right_expression,
        } => node(
            "BiheExpression",
            vec![
                field("leftExpression", math_expression_tree(*left_expression)),
                field("bihe", word_value(bihe)),
                field("freeModifiers", nil()),
                field("operator", math_operator_tree(operator)),
                field("rightExpression", math_expression_tree(*right_expression)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator_tree(operator: MathOperatorSyntax) -> SyntaxValue {
    match operator {
        MathOperatorSyntax::Vuhu { vuhu } => node(
            "VuhuOperator",
            vec![
                field("vuhu", word_value(vuhu)),
                field("freeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Maho {
            maho,
            math_expression,
            tehu,
        } => node(
            "MahoOperator",
            vec![
                field("maho", word_value(maho)),
                field("freeModifiers", nil()),
                field("mathExpression", math_expression_tree(*math_expression)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Se { se, inner_operator } => node(
            "SeOperator",
            vec![
                field("se", word_value(se)),
                field("freeModifiers", nil()),
                field("innerOperator", math_operator_tree(*inner_operator)),
            ],
        ),
        MathOperatorSyntax::Nahe {
            nahe,
            inner_operator,
        } => node(
            "NaheOperator",
            vec![
                field("nahe", word_value(nahe)),
                field("freeModifiers", nil()),
                field("innerOperator", math_operator_tree(*inner_operator)),
            ],
        ),
        MathOperatorSyntax::Nahu {
            nahu,
            relation,
            tehu,
        } => node(
            "NahuOperator",
            vec![
                field("nahu", word_value(nahu)),
                field("freeModifiers", nil()),
                field("relation", relation_tree(relation)),
                field("tehu", maybe_word(tehu)),
                field("tehuFreeModifiers", nil()),
            ],
        ),
        MathOperatorSyntax::Connected {
            left_operator,
            connective,
            right_operator,
        } => node(
            "ConnectedOperator",
            vec![
                field("leftOperator", math_operator_tree(*left_operator)),
                field("connective", connective_tree(connective)),
                field("rightOperator", math_operator_tree(*right_operator)),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_tree(relation: RelationSyntax) -> SyntaxValue {
    match relation {
        RelationSyntax::Connected {
            connective,
            leading_relation,
            trailing_relation,
        } => node(
            "ConnectedRelation",
            vec![
                field("connective", connective_tree(connective)),
                field("leadingRelation", relation_tree(*leading_relation)),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Co {
            leading_relation,
            co,
            free_modifiers,
            trailing_relation,
        } => node(
            "CoRelation",
            vec![
                field("leadingRelation", relation_tree(*leading_relation)),
                field("co", word_value(co)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Bo {
            leading_relation,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_relation,
        } => node(
            "BoRelation",
            vec![
                field("leadingRelation", relation_tree(*leading_relation)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingRelation", relation_tree(*trailing_relation)),
            ],
        ),
        RelationSyntax::Na {
            na,
            free_modifiers,
            inner_relation,
        } => node(
            "NaRelation",
            vec![
                field("na", word_value(na)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Base { word } => {
            node("BaseRelation", vec![field("word", word_value(word))])
        }
        RelationSyntax::Se {
            se,
            free_modifiers,
            inner_relation,
        } => node(
            "SeRelation",
            vec![
                field("se", word_value(se)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Ke {
            ke_tense_modal,
            ke,
            ke_free_modifiers,
            relation,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeRelation",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("innerRelation", relation_tree(*relation)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationSyntax::TenseModal {
            tense_modal,
            inner_relation,
        } => node(
            "TenseModalRelation",
            vec![
                field("tenseModal", tense_modal_tree(tense_modal)),
                field("innerRelation", relation_tree(*inner_relation)),
            ],
        ),
        RelationSyntax::Guha {
            guhek,
            leading_predicate,
            gik,
            trailing_predicate,
        } => node(
            "GuhaRelation",
            vec![
                field("guhek", connective_tree(guhek)),
                field("leadingPredicate", predicate_tree(*leading_predicate)),
                field("gik", connective_tree(gik)),
                field("trailingPredicate", predicate_tree(*trailing_predicate)),
            ],
        ),
        RelationSyntax::Abstraction { abstraction } => node(
            "AbstractionRelation",
            vec![field("abstraction", abstraction_tree(abstraction))],
        ),
        RelationSyntax::Compound { units } => node(
            "CompoundRelation",
            vec![field("relationUnits", nonempty_relation_units(units))],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_tree(abstraction: AbstractionSyntax) -> SyntaxValue {
    node(
        "Abstraction",
        vec![
            field("nu", word_value(abstraction.nu)),
            field("nai", maybe_word(abstraction.nai)),
            field(
                "freeModifiers",
                list(
                    abstraction
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field(
                "additionalNu",
                list(
                    abstraction
                        .additional_nu
                        .into_iter()
                        .map(additional_nu_tree)
                        .collect(),
                ),
            ),
            field("subsentence", subsentence_tree(*abstraction.subsentence)),
            field("kei", maybe_word(abstraction.kei)),
            field(
                "keiFreeModifiers",
                list(
                    abstraction
                        .kei_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn additional_nu_tree(additional_nu: AdditionalNuSyntax) -> SyntaxValue {
    node(
        "AdditionalNu",
        vec![
            field("connective", connective_tree(additional_nu.connective)),
            field("nu", word_value(additional_nu.nu)),
            field("nai", maybe_word(additional_nu.nai)),
            field(
                "freeModifiers",
                list(
                    additional_nu
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_tree(tense_modal: TenseModalSyntax) -> SyntaxValue {
    let free_modifiers = tense_modal.clone().free_modifiers();
    let leaves = match &tense_modal {
        TenseModalSyntax::Fiho { .. } => Vec::new(),
        _ => tense_modal.clone().leaf_words(),
    };
    let ki_field = match &tense_modal {
        TenseModalSyntax::Simple { ki: Some(ki), .. } | TenseModalSyntax::Ki { ki, .. } => {
            just(word_value(ki.clone()))
        }
        TenseModalSyntax::Composite { ki: Some(ki), .. } => just(word_value(ki.clone())),
        _ => nothing(),
    };
    let cuhe_field = match &tense_modal {
        TenseModalSyntax::Composite {
            cuhe: Some(cuhe), ..
        } => just(word_value(cuhe.clone())),
        _ => nothing(),
    };
    let connectives_field = match &tense_modal {
        TenseModalSyntax::Simple { connectives, .. } => {
            list(connectives.iter().cloned().map(word_value).collect())
        }
        TenseModalSyntax::Composite { connectives, .. } => {
            list(connectives.iter().cloned().map(word_value).collect())
        }
        _ => nil(),
    };
    let (time, space, simple, interval, zaho, caha, fiho) = match tense_modal {
        TenseModalSyntax::Composite {
            leaves: _,
            time,
            space,
            nahe,
            interval,
            zaho,
            caha,
            connectives: _,
            ..
        } => (
            time.map_or_else(nothing, |time| just(time_tense_tree(time))),
            space.map_or_else(nothing, |space| just(space_tense_tree(space))),
            nahe.map_or_else(nothing, |nahe| {
                just(node(
                    "SimpleTenseModal",
                    vec![
                        field("nahe", just(word_value(nahe))),
                        field("se", nothing()),
                        field("bai", nothing()),
                        field("nai", nothing()),
                    ],
                ))
            }),
            interval.map_or_else(nothing, |interval| {
                just(node(
                    "Interval",
                    vec![
                        field(
                            "number",
                            if interval.number.is_empty() {
                                nothing()
                            } else {
                                just(nonempty_number_words(interval.number))
                            },
                        ),
                        field("roiOrTahe", word_value(interval.roi_or_tahe)),
                        field("nai", maybe_word(interval.nai)),
                    ],
                ))
            }),
            list(zaho.into_iter().map(word_value).collect()),
            caha.map_or_else(nothing, |caha| just(word_value(caha))),
            nil(),
        ),
        TenseModalSyntax::Pu { word, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(word)])),
                    field("distance", nothing()),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::PuDistance { pu, distance, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(pu)])),
                    field("distance", just(word_value(distance))),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::TimeInterval { word, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", nil()),
                    field("distance", nothing()),
                    field("interval", just(word_value(word))),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::PuCaha { pu, caha, .. } => (
            just(node(
                "Time",
                vec![
                    field("direction", list(vec![word_value(pu)])),
                    field("distance", nothing()),
                    field("interval", nothing()),
                    field("nai", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            just(word_value(caha)),
            nil(),
        ),
        TenseModalSyntax::SpaceDistance { word, .. } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", nil()),
                    field("distance", list(vec![word_value(word)])),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", nothing()),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::SpaceDirection { word, .. } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", list(vec![word_value(word)])),
                    field("distance", nil()),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", nothing()),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
            ..
        } => (
            nothing(),
            just(node(
                "Space",
                vec![
                    field("direction", list(vec![word_value(direction)])),
                    field(
                        "distance",
                        list(distance.into_iter().map(word_value).collect()),
                    ),
                    field("interval", nil()),
                    field("dimensions", nil()),
                    field("mohi", just(word_value(mohi))),
                    field("fehe", nothing()),
                ],
            )),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki: _,
            connectives: _,
            ..
        } => (
            nothing(),
            nothing(),
            just(node(
                "SimpleTenseModal",
                vec![
                    field("nahe", maybe_word(nahe)),
                    field("se", maybe_word(se)),
                    field("bai", just(word_value(bai))),
                    field("nai", maybe_word(nai)),
                ],
            )),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Ki { ki: _, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
            ..
        } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            nothing(),
            list(vec![node(
                "FihoModal",
                vec![
                    field("nahe", nothing()),
                    field("fiho", word_value(fiho)),
                    field("fihoFreeModifiers", nil()),
                    field("relation", relation_tree(*relation)),
                    field("fehu", maybe_word(fehu)),
                    field("fehuFreeModifiers", nil()),
                ],
            )]),
        ),
        TenseModalSyntax::Caha { word, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            nil(),
            just(word_value(word)),
            nil(),
        ),
        TenseModalSyntax::Zaho { words, .. } => (
            nothing(),
            nothing(),
            nothing(),
            nothing(),
            list(words.into_iter().map(word_value).collect()),
            nothing(),
            nil(),
        ),
        TenseModalSyntax::Interval {
            number,
            roi_or_tahe,
            nai,
            ..
        } => (
            nothing(),
            nothing(),
            nothing(),
            just(node(
                "Interval",
                vec![
                    field(
                        "number",
                        if number.is_empty() {
                            nothing()
                        } else {
                            just(nonempty_number_words(number))
                        },
                    ),
                    field("roiOrTahe", word_value(roi_or_tahe)),
                    field("nai", maybe_word(nai)),
                ],
            )),
            nil(),
            nothing(),
            nil(),
        ),
    };

    node(
        "TenseModal",
        vec![
            field("leaves", list(leaves.into_iter().map(word_value).collect())),
            field("time", time),
            field("space", space),
            field("simple", simple),
            field("interval", interval),
            field("zaho", zaho),
            field("caha", caha),
            field("ki", ki_field),
            field("cuhe", cuhe_field),
            field("fiho", fiho),
            field("connectives", connectives_field),
            field(
                "freeModifiers",
                list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn time_tense_tree(time: TimeTenseSyntax) -> SyntaxValue {
    node(
        "Time",
        vec![
            field(
                "direction",
                list(time.direction.into_iter().map(word_value).collect()),
            ),
            field("distance", maybe_word(time.distance)),
            field("interval", maybe_word(time.interval)),
            field("nai", maybe_word(time.nai)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn space_tense_tree(space: SpaceTenseSyntax) -> SyntaxValue {
    node(
        "Space",
        vec![
            field(
                "direction",
                list(space.direction.into_iter().map(word_value).collect()),
            ),
            field(
                "distance",
                list(space.distance.into_iter().map(word_value).collect()),
            ),
            field(
                "interval",
                list(space.interval.into_iter().map(word_value).collect()),
            ),
            field(
                "dimensions",
                list(space.dimensions.into_iter().map(word_value).collect()),
            ),
            field("mohi", maybe_word(space.mohi)),
            field("fehe", maybe_word(space.fehe)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn nonempty_relation_units(units: Vec<RelationUnitSyntax>) -> SyntaxValue {
    let mut rendered = units
        .into_iter()
        .map(relation_unit_tree)
        .collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_letter_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(letter_word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_number_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_math_expressions(expressions: Vec<MathExpressionSyntax>) -> SyntaxValue {
    let mut rendered = expressions
        .into_iter()
        .map(math_expression_tree)
        .collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn nonempty_name_words(words: Vec<WordWithModifiers>) -> SyntaxValue {
    let mut rendered = words.into_iter().map(name_word_value).collect::<Vec<_>>();
    if rendered.len() <= 1 {
        return plain_list(rendered);
    }

    let tail = rendered.split_off(1);
    plain_list(vec![rendered.remove(0), list(tail)])
}

#[requires(true)]
#[ensures(true)]
fn letter_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_cmavo_i(normalize_syntax_word(word)))
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_tree(unit: RelationUnitSyntax) -> SyntaxValue {
    match unit {
        RelationUnitSyntax::Word {
            word,
            free_modifiers,
        } => node(
            "WordRelationUnit",
            vec![
                field("word", word_value(word)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Goha {
            goha,
            raho,
            free_modifiers,
        } => node(
            "GohaRelationUnit",
            vec![
                field("goha", word_value(goha)),
                field("raho", maybe_word(raho)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Se {
            se,
            free_modifiers,
            inner_unit,
        } => node(
            "SeRelationUnit",
            vec![
                field("se", word_value(se)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            ke_free_modifiers,
            relation,
            kehe,
            kehe_free_modifiers,
        } => node(
            "KeRelationUnit",
            vec![
                field(
                    "keTenseModal",
                    ke_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("ke", word_value(ke)),
                field(
                    "keFreeModifiers",
                    list(
                        ke_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("relation", relation_tree(relation)),
                field("kehe", maybe_word(kehe)),
                field(
                    "keheFreeModifiers",
                    list(
                        kehe_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Nahe {
            nahe,
            free_modifiers,
            inner_unit,
        } => node(
            "NaheRelationUnit",
            vec![
                field("nahe", word_value(nahe)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            free_modifiers,
            trailing_unit,
        } => node(
            "BoRelationUnit",
            vec![
                field("leadingUnit", relation_unit_tree(*leading_unit)),
                field(
                    "boConnective",
                    bo_connective
                        .map_or_else(nothing, |connective| just(connective_tree(connective))),
                ),
                field(
                    "boTenseModal",
                    bo_tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("bo", word_value(bo)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("trailingUnit", relation_unit_tree(*trailing_unit)),
            ],
        ),
        RelationUnitSyntax::Connected {
            leading_unit,
            connective,
            trailing_unit,
        } => node(
            "ConnectedRelationUnit",
            vec![
                field("leadingUnit", relation_unit_tree(*leading_unit)),
                field("connective", connective_tree(connective)),
                field("trailingUnit", relation_unit_tree(*trailing_unit)),
            ],
        ),
        RelationUnitSyntax::SelbriRelativeClause {
            base,
            selbri_relative_clauses,
        } => node(
            "SelbriRelativeClauseRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field(
                    "selbriRelativeClauses",
                    list(
                        selbri_relative_clauses
                            .into_iter()
                            .map(selbri_relative_clause_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Wrapped { relation } => node(
            "WrappedRelationUnit",
            vec![field("relation", relation_tree(relation))],
        ),
        RelationUnitSyntax::Jai {
            jai,
            free_modifiers,
            tense_modal,
            inner_unit,
        } => node(
            "JaiRelationUnit",
            vec![
                field("jai", word_value(jai)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field(
                    "tenseModal",
                    tense_modal
                        .map_or_else(nothing, |tense_modal| just(tense_modal_tree(tense_modal))),
                ),
                field("innerUnit", relation_unit_tree(*inner_unit)),
            ],
        ),
        RelationUnitSyntax::Be {
            base,
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
        } => node(
            "BeRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::PreposedBe {
            be,
            free_modifiers,
            fa,
            fa_free_modifiers,
            first_argument,
            bei_links,
            beho,
            beho_free_modifiers,
            base,
        } => node(
            "PreposedBeRelationUnit",
            vec![
                field("be", word_value(be)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("fa", maybe_word(fa)),
                field(
                    "faFreeModifiers",
                    list(
                        fa_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("firstArgument", maybe_argument(first_argument)),
                field(
                    "beiLinks",
                    list(bei_links.into_iter().map(bei_link_tree).collect()),
                ),
                field("beho", maybe_word(beho)),
                field(
                    "behoFreeModifiers",
                    list(
                        beho_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("base", relation_unit_tree(*base)),
            ],
        ),
        RelationUnitSyntax::Abstraction { abstraction } => node(
            "AbstractionRelationUnit",
            vec![field("abstraction", abstraction_tree(abstraction))],
        ),
        RelationUnitSyntax::Me {
            me,
            me_free_modifiers,
            argument,
            mehu,
            mehu_free_modifiers,
            moi_marker,
            moi_free_modifiers,
        } => node(
            "MeRelationUnit",
            vec![
                field("me", word_value(me)),
                field(
                    "meFreeModifiers",
                    list(
                        me_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("argument", argument_tree(argument)),
                field("mehu", maybe_word(mehu)),
                field(
                    "mehuFreeModifiers",
                    list(
                        mehu_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("moiMarker", maybe_word(moi_marker)),
                field(
                    "moiFreeModifiers",
                    list(
                        moi_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Mehoi {
            mehoi,
            quoted_text,
            free_modifiers,
        } => node(
            "MehoiRelationUnit",
            vec![
                field("mehoi", word_value(mehoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Gohoi {
            gohoi,
            quoted_text,
            free_modifiers,
        } => node(
            "GohoiRelationUnit",
            vec![
                field("gohoi", word_value(gohoi)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Muhoi {
            muhoi,
            opening_delimiter,
            closing_delimiter,
            quoted_text,
            free_modifiers,
        } => node(
            "MuhoiRelationUnit",
            vec![
                field("muhoi", word_value(muhoi)),
                field("openingDelimiter", word_value(opening_delimiter)),
                field("closingDelimiter", word_value(closing_delimiter)),
                field("quotedText", SyntaxValue::text(quoted_text)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Luhei {
            luhei,
            luhei_free_modifiers,
            text,
            liau,
            liau_free_modifiers,
        } => node(
            "LuheiRelationUnit",
            vec![
                field("luhei", word_value(luhei)),
                field(
                    "luheiFreeModifiers",
                    list(
                        luhei_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
                field("text", lojban_text_tree(text)),
                field("liau", maybe_word(liau)),
                field(
                    "liauFreeModifiers",
                    list(
                        liau_free_modifiers
                            .into_iter()
                            .map(free_modifier_tree)
                            .collect(),
                    ),
                ),
            ],
        ),
        RelationUnitSyntax::Moi {
            number,
            moi,
            free_modifiers,
        } => node(
            "MoiRelationUnit",
            vec![
                field("number", nonempty_number_words(number)),
                field("moi", word_value(moi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
            ],
        ),
        RelationUnitSyntax::Nuha {
            nuha,
            free_modifiers,
            math_operator,
        } => node(
            "NuhaRelationUnit",
            vec![
                field("nuha", word_value(nuha)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("mathOperator", math_operator_tree(math_operator)),
            ],
        ),
        RelationUnitSyntax::Xohi {
            xohi,
            free_modifiers,
            tag,
        } => node(
            "XohiRelationUnit",
            vec![
                field("xohi", word_value(xohi)),
                field(
                    "freeModifiers",
                    list(free_modifiers.into_iter().map(free_modifier_tree).collect()),
                ),
                field("tag", tense_modal_tree(tag)),
            ],
        ),
        RelationUnitSyntax::Cei { base, assignments } => node(
            "CeiRelationUnit",
            vec![
                field("base", relation_unit_tree(*base)),
                field(
                    "assignments",
                    list(assignments.into_iter().map(cei_assignment_tree).collect()),
                ),
            ],
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn cei_assignment_tree(assignment: CeiAssignmentSyntax) -> SyntaxValue {
    node(
        "CeiAssignment",
        vec![
            field("cei", word_value(assignment.cei)),
            field(
                "freeModifiers",
                list(
                    assignment
                        .free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("relationUnit", relation_unit_tree(assignment.relation_unit)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn maybe_word(word: Option<WordWithModifiers>) -> SyntaxValue {
    word.map_or_else(nothing, |word| just(word_value(word)))
}

#[requires(true)]
#[ensures(true)]
fn maybe_argument(argument: Option<ArgumentSyntax>) -> SyntaxValue {
    argument.map_or_else(nothing, |argument| just(argument_tree(argument)))
}

#[requires(true)]
#[ensures(true)]
fn bei_link_tree(link: BeiLinkSyntax) -> SyntaxValue {
    node(
        "BeiLink",
        vec![
            field("bei", word_value(link.bei)),
            field(
                "beiFreeModifiers",
                list(
                    link.bei_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("fa", maybe_word(link.fa)),
            field(
                "faFreeModifiers",
                list(
                    link.fa_free_modifiers
                        .into_iter()
                        .map(free_modifier_tree)
                        .collect(),
                ),
            ),
            field("argument", maybe_argument(link.argument)),
        ],
    )
}

#[requires(true)]
#[ensures(true)]
fn word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn gadri_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_cmavo_i(normalize_syntax_word(word)))
}

#[requires(true)]
#[ensures(true)]
fn vocative_marker_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn name_word_value(word: WordWithModifiers) -> SyntaxValue {
    syntax_word_value(normalize_syntax_word(word))
}

#[requires(true)]
#[ensures(true)]
fn syntax_word_value(word: WordWithModifiers) -> SyntaxValue {
    SyntaxValue::word(word)
}

#[requires(true)]
#[ensures(true)]
fn normalize_cmavo_i(word: WordWithModifiers) -> WordWithModifiers {
    match word.into_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            WordWithModifiers::base_word(normalize_word_like_cmavo_i(*word_like))
        }
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => {
            WordWithModifiers::emphasized(*bahe, normalize_word_like_cmavo_i(*word_like))
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            WordWithModifiers::standalone_indicator(
                normalize_word_record_cmavo_i(*indicator),
                nai.map(|nai| normalize_word_record_cmavo_i(*nai)),
            )
        }
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => WordWithModifiers::with_indicator(
            normalize_cmavo_i(*base),
            normalize_word_record_cmavo_i(*indicator),
            nai.map(|nai| normalize_word_record_cmavo_i(*nai)),
        ),
        data!(WordWithModifiers::NotEof) => WordWithModifiers::not_eof(),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_word_like_cmavo_i(word_like: WordLike) -> WordLike {
    match word_like.into_data() {
        data!(WordLike::Bare { word }) => WordLike::bare(normalize_word_record_cmavo_i(*word)),
        other => WordLike::from_data(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_word_record_cmavo_i(word: jbotci_morphology::Word) -> jbotci_morphology::Word {
    if word.kind == WordKind::Cmavo {
        let phonemes = word
            .phonemes
            .chars()
            .map(|ch| match ch {
                'ĭ' => 'i',
                'ŭ' => 'u',
                ch => ch,
            })
            .collect();
        word.with_data(data! {
            phonemes: phonemes,
        })
    } else {
        word
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word(word: WordWithModifiers) -> WordWithModifiers {
    match word.into_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            WordWithModifiers::base_word(normalize_syntax_word_like(*word_like))
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            WordWithModifiers::standalone_indicator(*indicator, nai.map(|nai| *nai))
        }
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => WordWithModifiers::emphasized(
            normalize_syntax_word_record(*bahe),
            normalize_syntax_word_like(*word_like),
        ),
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => WordWithModifiers::with_indicator(
            normalize_syntax_word(*base),
            *indicator,
            nai.map(|nai| *nai),
        ),
        data!(WordWithModifiers::NotEof) => WordWithModifiers::not_eof(),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word_like(word_like: WordLike) -> WordLike {
    match word_like.into_data() {
        data!(WordLike::Bare { word }) => WordLike::bare(normalize_syntax_word_record(*word)),
        data!(WordLike::ZoQuote { zo, word }) => WordLike::zo_quote(
            normalize_syntax_word_record(*zo),
            normalize_syntax_word_record(*word),
        ),
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => WordLike::zoi_quote(
            normalize_syntax_word_record(*zoi),
            normalize_syntax_word_record(*opening_delimiter),
            quoted_text,
            normalize_syntax_word_record(*closing_delimiter),
        ),
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => WordLike::lohu_quote(
            normalize_syntax_word_record(*lohu),
            quoted_words
                .into_iter()
                .map(normalize_syntax_word_record)
                .collect(),
            normalize_syntax_word_record(*lehu),
        ),
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => WordLike::single_word_quote(normalize_syntax_word_record(*marker), quoted_text),
        data!(WordLike::Letter { base, bu }) => WordLike::letter(
            normalize_syntax_word_like(*base),
            normalize_syntax_word_record(*bu),
        ),
        data!(WordLike::ZeiLujvo { left, zei, right }) => WordLike::zei_lujvo(
            normalize_syntax_word_like(*left),
            normalize_syntax_word_record(*zei),
            normalize_syntax_word_record(*right),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_syntax_word_record(word: jbotci_morphology::Word) -> jbotci_morphology::Word {
    word
}

#[requires(true)]
#[ensures(true)]
fn node(constructor: impl AsRef<str>, fields: Vec<SyntaxField>) -> SyntaxValue {
    SyntaxValue::node(constructor.as_ref().to_owned(), fields)
}

#[requires(true)]
#[ensures(true)]
fn field(name: impl AsRef<str>, value: SyntaxValue) -> SyntaxField {
    new!(SyntaxField {
        name: Some(name.as_ref().to_owned()),
        value: value,
    })
}

#[requires(true)]
#[ensures(true)]
fn unnamed_field(value: SyntaxValue) -> SyntaxField {
    new!(SyntaxField {
        name: None,
        value: value,
    })
}

#[requires(true)]
#[ensures(true)]
fn just(value: SyntaxValue) -> SyntaxValue {
    node("Just", vec![unnamed_field(value)])
}

#[requires(true)]
#[ensures(true)]
fn nothing() -> SyntaxValue {
    node("Nothing", Vec::new())
}

#[requires(true)]
#[ensures(true)]
fn nil() -> SyntaxValue {
    node("[]", Vec::new())
}

#[requires(true)]
#[ensures(true)]
fn plain_list(items: Vec<SyntaxValue>) -> SyntaxValue {
    SyntaxValue::list(items)
}

#[requires(true)]
#[ensures(true)]
fn list(items: Vec<SyntaxValue>) -> SyntaxValue {
    items.into_iter().rfold(nil(), |tail, head| {
        node("(:)", vec![unnamed_field(head), unnamed_field(tail)])
    })
}
