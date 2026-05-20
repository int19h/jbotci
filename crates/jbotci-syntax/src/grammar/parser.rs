use super::tense::*;
use super::tokens::*;
use super::*;
use chumsky::input::MapExtra;
use jbotci_dialect::DialectFeature;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct LeadingIStatementSyntax {
    i: WithIndicators<WordLike>,
    connective: Option<ConnectiveSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum TermContinuationSyntax {
    Pehe {
        tails: Vec<(
            WithIndicators<WordLike>,
            Vec<FreeModifierSyntax>,
            ConnectiveSyntax,
            TermSyntax,
        )>,
    },
    Connected {
        tails: Vec<(ConnectiveSyntax, TermSyntax)>,
    },
    None,
}

#[requires(true)]
#[ensures(ret.clone().free_modifiers().len() >= old(free_modifiers.len()))]
fn attach_tense_modal_free_modifiers(
    tense_modal: TenseModalSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
    match tense_modal {
        TenseModalSyntax::Composite {
            leaves,
            time,
            space,
            simple,
            interval,
            zaho,
            caha,
            ki,
            cuhe,
            fiho,
            connectives,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Composite {
                leaves,
                time,
                space,
                simple,
                interval,
                zaho,
                caha,
                ki,
                cuhe,
                fiho,
                connectives,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Pu {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Pu {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::PuDistance {
            pu,
            distance,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::PuDistance {
                pu,
                distance,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::TimeInterval {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::TimeInterval {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::PuCaha {
            pu,
            caha,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::PuCaha {
                pu,
                caha,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceDistance {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceDistance {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceDirection {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceDirection {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki,
            connectives,
            extra_leaves,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Simple {
                nahe,
                se,
                bai,
                nai,
                ki,
                connectives,
                extra_leaves,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Ki {
            ki,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Ki {
                ki,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Fiho {
                fiho,
                relation,
                fehu,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Caha {
            word,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Caha {
                word,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Zaho {
            words,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Zaho {
                words,
                free_modifiers: existing_free_modifiers,
            }
        }
        TenseModalSyntax::Interval {
            number,
            roi_or_tahe,
            nai,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
                free_modifiers: existing_free_modifiers,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_statement(
    words: &[WithIndicators<WordLike>],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<ParsedStatement, SyntaxError> {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
    let mut state = ParserState::new(words);

    let text = statement_parser(source, options)
        .then_ignore(end())
        .parse_with_state(
            tokens
                .as_slice()
                .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
            &mut state,
        )
        .into_result()
        .map_err(syntax_error)?;
    Ok(ParsedStatement {
        text,
        warnings: state.finish_warnings(),
    })
}

#[requires(true)]
#[ensures(true)]
fn statement_parser<'tokens>(
    source: Option<&'tokens str>,
    options: &ParseOptions,
) -> BoxedParser<'tokens, TextSyntax> {
    let mut text = Recursive::declare();
    let mut argument = Recursive::declare();
    let mut relation = Recursive::declare();
    let mut statement = Recursive::declare();
    let mut subsentence = Recursive::declare();
    let mut free_modifier = Recursive::declare();
    let mut term = Recursive::declare();
    let term_hierarchy_enabled = options
        .dialect
        .features
        .contains(&DialectFeature::TermHierarchy);
    argument.define(argument_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        term.clone(),
        text.clone(),
        free_modifier.clone(),
        source,
    ));
    let tense_modal_with_free_modifiers = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(tense_modal, free_modifiers)| {
            attach_tense_modal_free_modifiers(tense_modal, free_modifiers)
        })
        .boxed();
    relation.define(relation_parser_with(
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        text.clone(),
        free_modifier.clone(),
        source,
    ));

    let argument_term = argument.clone().map(TermSyntax::Argument);
    let elided_argument = cmavo("ku")
        .or_not()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(maybe_ku, free_modifiers)| ArgumentSyntax::Zohe {
            tag_words: Vec::new(),
            maybe_ku,
            free_modifiers,
        });
    let fa_term = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone().or(elided_argument))
        .map(|((fa, free_modifiers), argument)| TermSyntax::Fa {
            fa,
            free_modifiers,
            argument,
            ku: None,
        });
    let na_ku_term = na_cmavo()
        .then(cmavo("ku"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, na_ku), free_modifiers)| TermSyntax::NaKu {
            na,
            na_ku,
            free_modifiers,
        });
    let tagged_term_before_tag_start = leading_term_tag_tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal().rewind())
        .rewind()
        .ignored();
    let bare_na_term_blocker = choice((
        tagged_term_before_tag_start
            .not()
            .ignore_then(relation.clone().ignored()),
        modal_forethought_connective().ignored(),
        cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]).ignored(),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
            .ignored(),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("GIhA", &["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"]))
            .ignored(),
    ));
    let bare_na_term = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(bare_na_term_blocker.rewind().not())
        .map(|((na, free_modifiers), _)| TermSyntax::BareNa { na, free_modifiers });
    let tagged_term_start = modal_forethought_connective()
        .rewind()
        .not()
        .ignore_then(leading_term_tag_tense_modal())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>());
    let tagged_term_before_tag = tagged_term_start.clone().then(tense_modal().rewind()).map(
        |((tense_modal, free_modifiers), _)| TermSyntax::Tagged {
            tense_modal: Some(attach_tense_modal_free_modifiers(
                tense_modal,
                free_modifiers,
            )),
            free_modifiers: Vec::new(),
            argument: implicit_zohe_argument(),
        },
    );
    let tagged_term_before_non_relation = tagged_term_start
        .then(relation.clone().rewind().not())
        .then(
            argument.clone().or(cmavo("ku")
                .or_not()
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(|(maybe_ku, free_modifiers)| ArgumentSyntax::Zohe {
                    tag_words: Vec::new(),
                    maybe_ku,
                    free_modifiers,
                })),
        )
        .map(
            |(((tense_modal, free_modifiers), _), argument)| TermSyntax::Tagged {
                tense_modal: Some(attach_tense_modal_free_modifiers(
                    tense_modal,
                    free_modifiers,
                )),
                free_modifiers: Vec::new(),
                argument,
            },
        );
    let tagged_term = choice((tagged_term_before_tag, tagged_term_before_non_relation));
    let noiha_adverbial = cmavo_of("NOIhA", &["noi'a", "poi'a", "poi'o'a", "soi'a", "noi'o'a"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument_tail_with(
            argument.clone(),
            argument.clone(),
            relation.clone(),
            subsentence.clone(),
            free_modifier.clone(),
        ))
        .then(
            cmavo("fe'u")
                .map(Ok)
                .or(cmavo("ku").map(Err))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                ((noiha, leading_free_modifiers), (tail_elements, relation, relative_clauses)),
                terminator,
            )| {
                match terminator {
                    Some((Err(brigahi_ku), trailing_free_modifiers)) => TermSyntax::PoihaBrigahi {
                        poiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                        tail_elements,
                        relation,
                        relative_clauses,
                        brigahi_ku: WithFreeModifiers::new(brigahi_ku, trailing_free_modifiers),
                    },
                    Some((Ok(fehu), trailing_free_modifiers)) => TermSyntax::NoihaAdverbial {
                        noiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                        tail_elements,
                        relation,
                        relative_clauses,
                        fehu: Some(WithFreeModifiers::new(fehu, trailing_free_modifiers)),
                    },
                    None => TermSyntax::NoihaAdverbial {
                        noiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                        tail_elements,
                        relation,
                        relative_clauses,
                        fehu: None,
                    },
                }
            },
        )
        .boxed();
    let fihoi_adverbial = cmavo("fi'oi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(
            cmavo("fi'au")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((fihoi, leading_free_modifiers), subsentence), fihau)| TermSyntax::FihoiAdverbial {
                fihoi: WithFreeModifiers::new(fihoi, leading_free_modifiers),
                subsentence: Box::new(subsentence),
                fihau: fihau
                    .map(|(fihau, free_modifiers)| WithFreeModifiers::new(fihau, free_modifiers)),
            },
        )
        .boxed();
    let soi_adverbial = cmavo_of("SOI", &["soi", "xoi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(
            cmavo("se'u")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((soi, leading_free_modifiers), subsentence), sehu)| TermSyntax::SoiAdverbial {
                soi: WithFreeModifiers::new(soi, leading_free_modifiers),
                subsentence: Box::new(subsentence),
                sehu: sehu
                    .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
            },
        )
        .boxed();
    let soi_adverbials_enabled = options
        .dialect
        .features
        .contains(&DialectFeature::SoiAdverbials);
    let base_simple_term = if soi_adverbials_enabled {
        choice((
            fa_term.clone(),
            tagged_term.clone(),
            noiha_adverbial.clone(),
            fihoi_adverbial.clone(),
            soi_adverbial,
            na_ku_term.clone(),
            argument_term.clone(),
            bare_na_term.clone(),
        ))
        .boxed()
    } else {
        choice((
            fa_term,
            tagged_term,
            noiha_adverbial,
            fihoi_adverbial,
            na_ku_term,
            argument_term,
            bare_na_term,
        ))
        .boxed()
    };
    let term_body = {
        let term = term.clone();
        let gek_nuhi_termset = cmavo("nu'i")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .or_not()
            .then(modal_forethought_connective_with_free_modifiers(
                free_modifier.clone(),
            ))
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(
                cmavo("nu'u")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .then(gik_connective_with_free_modifiers(free_modifier.clone()))
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(
                cmavo("nu'u")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |((((((m_nuhi, gek), terms), nuhu), gik), gik_terms), gik_nuhu)| {
                    TermSyntax::GekNuhiTermset {
                        m_nuhi: m_nuhi.map(|(nuhi, free_modifiers)| {
                            WithFreeModifiers::new(nuhi, free_modifiers)
                        }),
                        gek,
                        terms,
                        nuhu: nuhu.map(|(nuhu, free_modifiers)| {
                            WithFreeModifiers::new(nuhu, free_modifiers)
                        }),
                        gik,
                        gik_terms,
                        gik_nuhu: gik_nuhu.map(|(nuhu, free_modifiers)| {
                            WithFreeModifiers::new(nuhu, free_modifiers)
                        }),
                    }
                },
            );
        let nuhi_termset = cmavo("nu'i")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(term.clone().repeated().at_least(1).collect::<Vec<_>>())
            .then(
                cmavo("nu'u")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |(((nuhi, nuhi_free_modifiers), termset), nuhu)| TermSyntax::NuhiTermset {
                    nuhi: WithFreeModifiers::new(nuhi, nuhi_free_modifiers),
                    termset,
                    nuhu: nuhu
                        .map(|(nuhu, free_modifiers)| WithFreeModifiers::new(nuhu, free_modifiers)),
                },
            );
        let simple_term =
            choice((base_simple_term.clone(), gek_nuhi_termset, nuhi_termset)).boxed();
        let cehe_term = simple_term
            .clone()
            .then(
                cmavo("ce'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(
                        simple_term
                            .clone()
                            .repeated()
                            .at_least(1)
                            .collect::<Vec<_>>(),
                    )
                    .or_not(),
            )
            .map(|(leading_term, cehe_tail)| {
                cehe_tail.map_or(
                    leading_term.clone(),
                    |((cehe, free_modifiers), trailing_terms)| TermSyntax::Cehe {
                        leading_terms: vec![leading_term],
                        cehe,
                        free_modifiers,
                        trailing_terms,
                    },
                )
            })
            .boxed();
        let post_bo_argument_gate = if term_hierarchy_enabled {
            empty().to(()).boxed()
        } else {
            argument.clone().rewind().not().boxed()
        };
        let post_bo_trailing_argument_gate = if term_hierarchy_enabled {
            empty().to(()).boxed()
        } else {
            argument.clone().rewind().not().boxed()
        };
        let bo_tail = connective_with_free_modifiers(joik_ek_connective(), free_modifier.clone())
            .then(cmavo("bo"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(post_bo_argument_gate)
            .then(simple_term.clone())
            .then(post_bo_trailing_argument_gate)
            .map(
                |(((((bo_connective, bo), free_modifiers), _), trailing_term), _)| {
                    (Some(bo_connective), None, bo, free_modifiers, trailing_term)
                },
            );
        let term2 = cehe_term
            .clone()
            .then(bo_tail.repeated().collect::<Vec<_>>())
            .map(|(first, tails)| {
                tails.into_iter().fold(
                    first,
                    |leading_term, (bo_connective, tense_modal, bo, free_modifiers, trailing_term)| {
                        TermSyntax::BoConnected {
                            leading_terms: vec![leading_term],
                            bo_connective,
                            tense_modal,
                            bo,
                            free_modifiers,
                            trailing_term: Box::new(trailing_term),
                        }
                    },
                )
            })
            .boxed();
        let pehe_continuation = cmavo("pe'e")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(statement_connective())
            .then(term2.clone())
            .map(|(((pehe, free_modifiers), connective), trailing_term)| {
                (pehe, free_modifiers, connective, trailing_term)
            })
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|tails| TermContinuationSyntax::Pehe { tails });
        let connected_continuation =
            connective_with_free_modifiers(term_connective(), free_modifier.clone())
                .then(term2.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>()
                .map(|tails| TermContinuationSyntax::Connected { tails });
        term2
            .clone()
            .then(choice((
                pehe_continuation,
                connected_continuation,
                empty().to(TermContinuationSyntax::None),
            )))
            .map(|(leading_term, continuation)| match continuation {
                TermContinuationSyntax::Pehe { tails } => tails.into_iter().fold(
                    leading_term,
                    |leading_term, (pehe, free_modifiers, connective, trailing_term)| {
                        TermSyntax::Pehe {
                            leading_terms: vec![leading_term],
                            pehe,
                            free_modifiers,
                            connective,
                            trailing_terms: vec![trailing_term],
                        }
                    },
                ),
                TermContinuationSyntax::Connected { tails } => tails.into_iter().fold(
                    leading_term,
                    |leading_term, (connective, trailing_term)| TermSyntax::Connected {
                        leading_terms: vec![leading_term],
                        connective,
                        trailing_terms: vec![trailing_term],
                    },
                ),
                TermContinuationSyntax::None => leading_term,
            })
            .boxed()
    };
    term.define(term_body.boxed());
    let tail_term = term.clone();
    let cu = cmavo("cu");
    let basic_predicate = recursive(|_basic_predicate| {
        let gek_sentence = recursive(|gek_sentence| {
            let pair = modal_forethought_connective_with_free_modifiers(free_modifier.clone())
                .then(subsentence.clone())
                .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                .then(subsentence.clone())
                .then(tail_term.clone().repeated().collect::<Vec<_>>())
                .then(cmavo("vau").or_not())
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(
                    |((((((gek, first), gik), second), tail_terms), vau), free_modifiers)| {
                        GekSentenceSyntax::Pair {
                            gek,
                            first: Box::new(first),
                            gik,
                            second: Box::new(second),
                            tail_terms,
                            vau,
                            free_modifiers,
                        }
                    },
                );
            let ke = tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("ke"))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(gek_sentence.clone())
                .then(
                    cmavo("ke'e")
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .or_not(),
                )
                .map(|((((tense_modal, ke), ke_free_modifiers), inner), kehe)| {
                    GekSentenceSyntax::Ke {
                        tense_modal,
                        ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                        inner: Box::new(inner),
                        kehe: kehe.map(|(kehe, free_modifiers)| {
                            WithFreeModifiers::new(kehe, free_modifiers)
                        }),
                    }
                });
            let na = na_cmavo()
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(gek_sentence.clone())
                .map(|((na, free_modifiers), inner)| GekSentenceSyntax::Na {
                    na,
                    free_modifiers,
                    inner: Box::new(inner),
                });
            choice((pair, ke, na)).boxed()
        });
        let implicit_tagged_term_before_grouped_gek = tense_modal_with_free_modifiers
            .clone()
            .then(cmavo("ke").rewind())
            .map(|(tense_modal, _)| TermSyntax::Tagged {
                tense_modal: Some(tense_modal),
                free_modifiers: Vec::new(),
                argument: implicit_zohe_argument(),
            });
        let non_grouped_gek_term = cmavo("ke").rewind().not().ignore_then(term.clone());
        let gek_leading_term = choice((
            implicit_tagged_term_before_grouped_gek,
            non_grouped_gek_term,
        ))
        .boxed();
        let predicate_tail_terms = tail_term
            .clone()
            .repeated()
            .collect::<Vec<_>>()
            .then(cmavo("vau").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>());
        let experimental_predicate_tail_cu = cu
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .or_not()
            .map(|cu| cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)));
        let predicate_tail = recursive(|predicate_tail| {
            let predicate_tail2 = recursive(|predicate_tail2| {
                let relation_tail3 = relation.clone().then(predicate_tail_terms.clone()).map(
                    |(relation, ((terms, vau), free_modifiers))| PredicateTail3Syntax::Relation {
                        relation,
                        terms,
                        vau,
                        free_modifiers,
                    },
                );
                let gek_tail3 = gek_sentence.clone().map(PredicateTail3Syntax::GekSentence);
                let bo_continuation = predicate_tail_connective()
                    .then(tense_modal_with_free_modifiers.clone().or_not())
                    .then(cmavo("bo"))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(experimental_predicate_tail_cu.clone())
                    .then(predicate_tail2.clone())
                    .then(predicate_tail_terms.clone())
                    .map(
                        |(
                            (
                                ((((connective, tense_modal), bo), free_modifiers), cu),
                                predicate_tail,
                            ),
                            ((tail_terms, vau), _tail_free_modifiers),
                        )| BoPredicateTailSyntax {
                            connective,
                            tense_modal,
                            bo,
                            free_modifiers,
                            cu,
                            predicate_tail: Box::new(predicate_tail),
                            tail_terms,
                            vau,
                        },
                    )
                    .boxed();
                choice((gek_tail3, relation_tail3))
                    .then(bo_continuation.or_not())
                    .map(|(first, bo_continuation)| PredicateTail2Syntax {
                        first,
                        bo_continuation,
                    })
            });
            let bo_or_ke_continuation_start = predicate_tail_connective()
                .then(tense_modal_with_free_modifiers.clone().or_not())
                .then(choice((cmavo("bo"), cmavo("ke"))))
                .rewind();
            let predicate_tail_continuation = bo_or_ke_continuation_start
                .not()
                .ignore_then(predicate_tail_connective())
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(experimental_predicate_tail_cu.clone())
                .then(predicate_tail2.clone())
                .then(predicate_tail_terms.clone())
                .map(
                    |(
                        (((connective, free_modifiers), cu), predicate_tail),
                        ((tail_terms, vau), _tail_free_modifiers),
                    )| {
                        let mut connective = connective;
                        connective.free_modifiers.extend(free_modifiers);
                        PredicateTailContinuationSyntax {
                            connective,
                            tense_modal: None,
                            cu,
                            predicate_tail,
                            tail_terms,
                            vau,
                            free_modifiers: Vec::new(),
                        }
                    },
                )
                .boxed();
            let predicate_tail1 = predicate_tail2
                .clone()
                .then(
                    predicate_tail_continuation
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .map(|(first, continuations)| PredicateTail1Syntax {
                    first,
                    continuations,
                });
            let ke_continuation = predicate_tail_connective()
                .then(tense_modal_with_free_modifiers.clone().or_not())
                .then(cmavo("ke"))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(predicate_tail.clone())
                .then(
                    cmavo("ke'e")
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .or_not(),
                )
                .then(predicate_tail_terms.clone())
                .map(
                    |(
                        (
                            ((((connective, tense_modal), ke), ke_free_modifiers), predicate_tail),
                            kehe,
                        ),
                        ((tail_terms, vau), _tail_free_modifiers),
                    )| {
                        KePredicateTailSyntax {
                            connective,
                            tense_modal,
                            ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                            predicate_tail: Box::new(predicate_tail),
                            kehe: kehe.map(|(kehe, free_modifiers)| {
                                WithFreeModifiers::new(kehe, free_modifiers)
                            }),
                            tail_terms,
                            vau,
                        }
                    },
                )
                .boxed();
            predicate_tail1
                .then(ke_continuation.or_not())
                .try_map(|(first, ke_continuation), span| {
                    if ke_continuation.as_ref().is_some_and(|ke_continuation| {
                        !predicate_tail_ke_continuation_allowed(&first, ke_continuation)
                    }) {
                        return Err(Rich::custom(
                            span,
                            "predicate-tail KE continuation conflicts with trailing argument connection",
                        ));
                    }
                    Ok(PredicateTailSyntax {
                        first,
                        ke_continuation,
                    })
                })
        });
        let predicate_with_leading_terms = term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cu.clone()
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .then(predicate_tail.clone())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((leading_terms, cu), predicate_tail), free_modifiers)| PredicateSyntax {
                    leading_terms,
                    cu: cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)),
                    predicate_tail,
                    free_modifiers,
                },
            );

        let relation_only = predicate_tail
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(predicate_tail, free_modifiers)| PredicateSyntax {
                leading_terms: Vec::new(),
                cu: None,
                predicate_tail,
                free_modifiers,
            });
        let bare_cu_predicate = cu
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(predicate_tail.clone())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((cu, cu_free_modifiers), predicate_tail), free_modifiers)| PredicateSyntax {
                    leading_terms: Vec::new(),
                    cu: Some(WithFreeModifiers::new(cu, cu_free_modifiers)),
                    predicate_tail,
                    free_modifiers,
                },
            )
            .boxed();
        let forethought_predicate_with_leading_terms = gek_leading_term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cu.clone()
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .then(predicate_tail)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((leading_terms, cu), predicate_tail), free_modifiers)| PredicateSyntax {
                    leading_terms,
                    cu: cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)),
                    predicate_tail,
                    free_modifiers,
                },
            );

        choice((
            forethought_predicate_with_leading_terms,
            predicate_with_leading_terms,
            bare_cu_predicate,
            relation_only,
        ))
        .boxed()
    });
    let plain_subsentence = basic_predicate.clone().map(SubsentenceSyntax::Plain);
    let prenex_subsentence = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_subsentence)| {
                SubsentenceSyntax::Prenex {
                    prenex_terms,
                    zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
                    inner_subsentence: Box::new(inner_subsentence),
                }
            },
        );
    subsentence.define(choice((prenex_subsentence, plain_subsentence)));
    let predicate_statement_bo_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .map(
            |((((connective, tense_modal), bo), free_modifiers), trailing_subsentence)| {
                PredicateStatementContinuationSyntax {
                    connective,
                    tense_modal,
                    marker: PredicateStatementContinuationMarkerSyntax::Bo {
                        bo: WithFreeModifiers::new(bo, free_modifiers),
                    },
                    trailing_subsentence,
                }
            },
        );
    let predicate_statement_ke_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("ke"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(
            cmavo("ke'e")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                ((((connective, tense_modal), ke), ke_free_modifiers), trailing_subsentence),
                kehe,
            )| {
                PredicateStatementContinuationSyntax {
                    connective,
                    tense_modal,
                    marker: PredicateStatementContinuationMarkerSyntax::Ke {
                        ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                        kehe: kehe.map(|(kehe, free_modifiers)| {
                            WithFreeModifiers::new(kehe, free_modifiers)
                        }),
                    },
                    trailing_subsentence,
                }
            },
        );
    let predicate_statement_continuation = choice((
        predicate_statement_bo_continuation,
        predicate_statement_ke_continuation,
    ));
    let predicate = basic_predicate
        .clone()
        .then(
            predicate_statement_continuation
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(predicate, continuations)| build_predicate_statement(predicate, continuations));

    let fragment_term = term.clone();

    let term_fragment = fragment_term
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(
            cmavo("vau")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(terms, vau)| {
            StatementSyntax::Fragment(FragmentSyntax::Term {
                terms,
                vau: vau.map(|(vau, free_modifiers)| WithFreeModifiers::new(vau, free_modifiers)),
            })
        });

    let relative_clause_fragment =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone()).map(
            |relative_clauses| {
                StatementSyntax::Fragment(FragmentSyntax::RelativeClause(relative_clauses))
            },
        );
    let ek_fragment = ek_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Ek {
                connective,
                free_modifiers,
            })
        });
    let gihek_fragment = gihek_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Gihek {
                connective,
                free_modifiers,
            })
        });

    let multiple_na_fragment = na_cmavo()
        .then(na_cmavo())
        .then(na_cmavo().repeated().collect::<Vec<_>>())
        .then(
            cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"])
                .rewind()
                .not(),
        )
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((((first_na, second_na), rest_na), _), free_modifiers)| {
            let mut words = vec![first_na, second_na];
            words.extend(rest_na);
            StatementSyntax::Fragment(FragmentSyntax::Other {
                words,
                free_modifiers,
            })
        });
    let single_na_fragment_blocker = choice((
        cmavo("ku").ignored(),
        na_cmavo().ignored(),
        cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]).ignored(),
        argument_connective().ignored(),
        predicate_tail_connective().ignored(),
    ));
    let single_na_fragment = na_cmavo()
        .then(single_na_fragment_blocker.rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, _), free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Other {
                words: vec![na],
                free_modifiers,
            })
        });

    let be_link_fragment = be_link_parser(argument.clone(), free_modifier.clone()).map(|link| {
        let data!(BeLinkSyntax {
            be,
            fa,
            first_argument,
            bei_links,
            beho,
        }) = link.into_data();

        {
            StatementSyntax::Fragment(FragmentSyntax::BeLink {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            })
        }
    });
    let bei_link_fragment = bei_link_parser(argument.clone(), free_modifier.clone())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|bei_only_links| StatementSyntax::Fragment(FragmentSyntax::BeiLink(bei_only_links)));

    let math_expression_fragment =
        quantifier_with_free_modifiers(quantifier(), free_modifier.clone()).map(|quantifier| {
            StatementSyntax::Fragment(FragmentSyntax::MathExpression(
                MathExpressionSyntax::Number(quantifier),
            ))
        });

    let relation_fragment = relation
        .clone()
        .map(|relation| StatementSyntax::Fragment(FragmentSyntax::Relation(relation)));

    let prenex_fragment = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((terms, zohu), zohu_free_modifiers)| {
            StatementSyntax::Fragment(FragmentSyntax::Prenex {
                terms,
                zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
            })
        });

    let prenex_statement = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo("zo'u"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(statement.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_statement)| {
                StatementSyntax::Prenex {
                    prenex_terms,
                    zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
                    inner_statement: Box::new(inner_statement),
                }
            },
        );
    let tuhe_statement = tense_modal_with_free_modifiers
        .clone()
        .or_not()
        .then(cmavo("tu'e"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(
            cmavo("tu'u")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((tense_modal, tuhe), tuhe_free_modifiers), text), tuhu)| StatementSyntax::Tuhe {
                tense_modal,
                tuhe: WithFreeModifiers::new(tuhe, tuhe_free_modifiers),
                text: Box::new(text),
                tuhu: tuhu
                    .map(|(tuhu, free_modifiers)| WithFreeModifiers::new(tuhu, free_modifiers)),
            },
        );

    let fragment_statement = choice((
        prenex_fragment,
        relation_fragment,
        multiple_na_fragment,
        single_na_fragment,
        term_fragment,
        ek_fragment,
        gihek_fragment,
        math_expression_fragment,
        relative_clause_fragment,
        bei_link_fragment,
        be_link_fragment,
    ))
    .boxed();

    let simple_statement_after_i_connective = choice((predicate, tuhe_statement)).boxed();

    let simple_statement = choice((
        prenex_statement,
        simple_statement_after_i_connective.clone(),
    ));

    let pending_i_connective = cmavo("i")
        .then(statement_connective())
        .then(cmavo("i").rewind())
        .map(|((i, connective), _)| (i, connective))
        .boxed();
    let chained_i_connective_statement_tail = pending_i_connective
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(
            cmavo("i")
                .then(statement_connective())
                .then(
                    tense_modal_with_free_modifiers
                        .clone()
                        .or_not()
                        .then(cmavo("bo"))
                        .or_not(),
                )
                .then(simple_statement_after_i_connective.clone()),
        )
        .map(
            |(pending, (((i, connective), tag_bo), trailing_statement))| {
                let (first_i, pending_words) = pending.into_iter().enumerate().fold(
                    (None, Vec::new()),
                    |(first_i, mut pending_words), (index, (pending_i, pending_connective))| {
                        let first_i = first_i.or_else(|| Some(pending_i.clone()));
                        if index > 0 {
                            pending_words.push(pending_i);
                        }
                        pending_words.extend(pending_connective.cmavo);
                        (first_i, pending_words)
                    },
                );
                let connective = tag_bo.map_or(connective.clone(), |(tense_modal, bo)| {
                    let mut cmavo = connective.cmavo;
                    if let Some(tense_modal) = tense_modal {
                        cmavo.extend(tense_modal.words());
                    }
                    cmavo.push(bo);
                    ConnectiveSyntax {
                        kind: connective.kind,
                        se: connective.se,
                        nahe: connective.nahe,
                        na: connective.na,
                        cmavo,
                        nai: connective.nai,
                        free_modifiers: connective.free_modifiers,
                    }
                });
                let connective =
                    prepend_connective_words([pending_words, vec![i]].concat(), connective);
                (
                    false,
                    first_i.expect("at least one pending i connective is parsed"),
                    connective,
                    trailing_statement,
                )
            },
        );
    let i_connective_statement_tail = cmavo("i")
        .then(statement_connective())
        .then(
            tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("bo"))
                .or_not(),
        )
        .then(simple_statement_after_i_connective.clone())
        .map(|(((i, connective), tag_bo), trailing_statement)| {
            let connective = tag_bo.map_or(connective.clone(), |(tense_modal, bo)| {
                let mut cmavo = connective.cmavo;
                if let Some(tense_modal) = tense_modal {
                    cmavo.extend(tense_modal.words());
                }
                cmavo.push(bo);
                ConnectiveSyntax {
                    kind: connective.kind,
                    se: connective.se,
                    nahe: connective.nahe,
                    na: connective.na,
                    cmavo,
                    nai: connective.nai,
                    free_modifiers: connective.free_modifiers,
                }
            });
            (false, i, connective, trailing_statement)
        });
    let i_bo_statement_tail = cmavo("i")
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo("bo"))
        .then(simple_statement_after_i_connective.clone())
        .map(|(((i, tense_modal), bo), trailing_statement)| {
            let mut cmavo = tense_modal.map_or_else(Vec::new, TenseModalSyntax::words);
            cmavo.push(bo);
            (
                false,
                i,
                ConnectiveSyntax {
                    kind: ConnectiveKind::Relation,
                    se: None,
                    nahe: None,
                    na: None,
                    cmavo,
                    nai: None,
                    free_modifiers: Vec::new(),
                },
                trailing_statement,
            )
        });
    let connected_statement_tail = choice((
        chained_i_connective_statement_tail,
        i_connective_statement_tail,
        i_bo_statement_tail,
        statement_connective()
            .then(
                tense_modal_with_free_modifiers
                    .clone()
                    .or_not()
                    .then(cmavo("bo"))
                    .or_not(),
            )
            .then(cmavo("i"))
            .then(simple_statement_after_i_connective.clone())
            .map(|(((connective, tag_bo), i), trailing_statement)| {
                let connective = tag_bo.map_or(connective.clone(), |(tense_modal, bo)| {
                    let mut cmavo = connective.cmavo;
                    if let Some(tense_modal) = tense_modal {
                        cmavo.extend(tense_modal.words());
                    }
                    cmavo.push(bo);
                    ConnectiveSyntax {
                        kind: connective.kind,
                        se: connective.se,
                        nahe: connective.nahe,
                        na: connective.na,
                        cmavo,
                        nai: connective.nai,
                        free_modifiers: connective.free_modifiers,
                    }
                });
                (true, i, connective, trailing_statement)
            }),
    ))
    .boxed();
    let statement_body = simple_statement
        .clone()
        .then(connected_statement_tail.repeated().collect::<Vec<_>>())
        .map(|(leading_statement, continuations)| {
            build_connected_statement(leading_statement, continuations)
        });

    let iau_statement_body = statement_body
        .then(
            cmavo("ia'u")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(term.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(statement, iau_tail)| match iau_tail {
            Some(((iau, iau_free_modifiers), reset_terms)) => StatementSyntax::Iau {
                inner_statement: Box::new(statement),
                iau: WithFreeModifiers::new(iau, iau_free_modifiers),
                reset_terms,
            },
            None => statement,
        });

    statement.define(iau_statement_body);
    free_modifier.define(choice((
        replacement_free(free_modifier.clone()),
        mai_free(free_modifier.clone()),
        xi_free(free_modifier.clone()),
        sei_free(term.clone(), relation.clone(), free_modifier.clone()),
        soi_free(argument.clone(), free_modifier.clone()),
        to_free(text.clone(), free_modifier.clone()),
        vocative_free(
            argument.clone(),
            relation.clone(),
            subsentence.clone(),
            free_modifier.clone(),
        ),
    )));

    let paragraph_statement_body = choice((statement.clone(), fragment_statement.clone())).boxed();
    let initial_statement =
        paragraph_statement_body
            .clone()
            .map(|statement| ParagraphStatementSyntax {
                i: None,
                connective: None,
                free_modifiers: Vec::new(),
                statement: Some(statement),
            });

    let i_connective_tag_bo = standard_statement_connective()
        .or_not()
        .then(
            tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo("bo"))
                .or_not(),
        )
        .map(|(connective, tag_bo)| match (connective, tag_bo) {
            (None, None) => None,
            (Some(connective), None) => Some(connective),
            (connective, Some((tense_modal, bo))) => {
                let (kind, se, nahe, na, nai, mut cmavo, free_modifiers) = connective.map_or(
                    (
                        ConnectiveKind::Relation,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                        Vec::new(),
                    ),
                    |connective| {
                        (
                            connective.kind,
                            connective.se,
                            connective.nahe,
                            connective.na,
                            connective.nai,
                            connective.cmavo,
                            connective.free_modifiers,
                        )
                    },
                );
                if let Some(tense_modal) = tense_modal {
                    cmavo.extend(tense_modal.words());
                }
                cmavo.push(bo);
                Some(ConnectiveSyntax {
                    kind,
                    se,
                    nahe,
                    na,
                    cmavo,
                    nai,
                    free_modifiers,
                })
            }
        });

    let leading_i_statement = cmavo("i")
        .then(i_connective_tag_bo.clone())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((i, connective), free_modifiers)| LeadingIStatementSyntax {
                i,
                connective,
                free_modifiers,
            },
        );

    let following_statement = cmavo("i")
        .then_ignore(statement_connective().rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(paragraph_statement_body.or_not())
        .map(
            |((i, free_modifiers), statement)| ParagraphStatementSyntax {
                i: Some(i),
                connective: None,
                free_modifiers,
                statement,
            },
        );
    let trailing_ijek_statement = cmavo("i")
        .then(statement_connective())
        .map(|(i, connective)| ParagraphStatementSyntax {
            i: None,
            connective: None,
            free_modifiers: Vec::new(),
            statement: Some(StatementSyntax::Fragment(FragmentSyntax::Ijek {
                i,
                connective,
            })),
        });

    let paragraph_without_niho = initial_statement
        .clone()
        .then(following_statement.clone().repeated().collect::<Vec<_>>())
        .then(trailing_ijek_statement.repeated().collect::<Vec<_>>())
        .map(|((initial, following), trailing_ijek)| {
            build_paragraph(
                None,
                Vec::new(),
                Vec::new(),
                std::iter::once(initial)
                    .chain(following)
                    .chain(trailing_ijek)
                    .collect(),
            )
        });
    let paragraph = paragraph_without_niho.boxed();
    let paragraph_with_niho = cmavo_of("NIhO", &["ni'o", "no'i"])
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(paragraph.clone().or_not())
        .map(|((niho, free_modifiers), paragraph)| match paragraph {
            Some(mut paragraph) => {
                if paragraph.niho.is_empty() {
                    paragraph.niho = niho;
                }
                if paragraph.free_modifiers.is_empty() {
                    paragraph.free_modifiers = free_modifiers;
                }
                paragraph
            }
            None => build_paragraph(None, niho, free_modifiers, Vec::new()),
        })
        .boxed();
    let paragraphs = choice((
        paragraph
            .clone()
            .then(paragraph_with_niho.clone().repeated().collect::<Vec<_>>())
            .map(|(first, rest)| std::iter::once(first).chain(rest).collect::<Vec<_>>()),
        paragraph_with_niho
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>(),
    ))
    .or_not()
    .map(Option::unwrap_or_default);

    let text_body = cmavo("nai")
        .repeated()
        .collect::<Vec<_>>()
        .then(cmevla_word().repeated().collect::<Vec<_>>())
        .then(leading_indicator().repeated().collect::<Vec<_>>())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(
            modal_forethought_connective()
                .rewind()
                .not()
                .ignore_then(standard_statement_connective())
                .or_not(),
        )
        .then(leading_i_statement.repeated().collect::<Vec<_>>())
        .then(paragraphs)
        .map(
            |(
                (
                    (
                        (
                            ((leading_nai, leading_cmevla), leading_indicators),
                            leading_free_modifiers,
                        ),
                        leading_connective,
                    ),
                    leading_i_statements,
                ),
                paragraphs,
            )| {
                let text = TextSyntax {
                    leading_nai,
                    leading_cmevla,
                    leading_indicators,
                    leading_free_modifiers,
                    leading_connective,
                    paragraphs,
                };
                leading_i_statements
                    .into_iter()
                    .rev()
                    .fold(text, |text, leading_i_statement| {
                        prepend_i_with_free_modifier(leading_i_statement, text)
                    })
            },
        );

    text.define(text_body);
    text.then_ignore(end()).boxed()
}

#[requires(true)]
#[ensures(true)]
fn prepend_i_with_free_modifier(
    marker: LeadingIStatementSyntax,
    mut text: TextSyntax,
) -> TextSyntax {
    if text.paragraphs.is_empty() {
        text.paragraphs.push(ParagraphSyntax {
            i: None,
            niho: Vec::new(),
            free_modifiers: Vec::new(),
            statements: vec![ParagraphStatementSyntax {
                i: Some(marker.i),
                connective: marker.connective,
                free_modifiers: marker.free_modifiers,
                statement: None,
            }],
        });
        return text;
    }

    let mut paragraph = text.paragraphs.remove(0);
    if paragraph.niho.is_empty() {
        paragraph.statements = prepend_i_to_niho_free_paragraph_statements(
            marker,
            std::mem::take(&mut paragraph.statements),
        );
    } else {
        paragraph.i = Some(marker.i);
        paragraph.statements = attach_i_connective_to_niho_paragraph_statements(
            marker.connective,
            marker.free_modifiers,
            std::mem::take(&mut paragraph.statements),
        );
    }
    text.paragraphs.insert(0, paragraph);
    text
}

#[requires(true)]
#[ensures(true)]
fn prepend_i_to_niho_free_paragraph_statements(
    marker: LeadingIStatementSyntax,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let new_statement = ParagraphStatementSyntax {
        i: Some(marker.i),
        connective: marker.connective,
        free_modifiers: marker.free_modifiers,
        statement: None,
    };
    let Some(first) = statements.first_mut() else {
        return vec![new_statement];
    };
    if first.i.is_some() {
        statements.insert(0, new_statement);
        return statements;
    }

    first.i = new_statement.i;
    first.connective = new_statement.connective;
    first.free_modifiers = new_statement.free_modifiers;
    statements
}

#[requires(true)]
#[ensures(true)]
fn attach_i_connective_to_niho_paragraph_statements(
    connective: Option<ConnectiveSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let Some(first) = statements.first_mut() else {
        return vec![ParagraphStatementSyntax {
            i: None,
            connective,
            free_modifiers,
            statement: None,
        }];
    };
    first.connective = connective;
    let mut combined_free_modifiers = free_modifiers;
    combined_free_modifiers.append(&mut first.free_modifiers);
    first.free_modifiers = combined_free_modifiers;
    statements
}

#[requires(true)]
#[ensures(true)]
fn build_paragraph(
    i: Option<WithIndicators<WordLike>>,
    niho: Vec<WithIndicators<WordLike>>,
    free_modifiers: Vec<FreeModifierSyntax>,
    statements: Vec<ParagraphStatementSyntax>,
) -> ParagraphSyntax {
    ParagraphSyntax {
        i,
        niho,
        free_modifiers,
        statements: normalize_trailing_ijek_fragment(statements),
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_trailing_ijek_fragment(
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let Some(last) = statements.pop() else {
        return statements;
    };
    match last {
        ParagraphStatementSyntax {
            i: Some(i),
            connective: Some(connective),
            free_modifiers,
            statement: None,
        } if free_modifiers.is_empty() => {
            statements.push(ParagraphStatementSyntax {
                i: None,
                connective: None,
                free_modifiers: Vec::new(),
                statement: Some(StatementSyntax::Fragment(FragmentSyntax::Ijek {
                    i,
                    connective,
                })),
            });
            statements
        }
        other => {
            statements.push(other);
            statements
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn build_predicate_statement(
    predicate: PredicateSyntax,
    continuations: Vec<PredicateStatementContinuationSyntax>,
) -> StatementSyntax {
    continuations.into_iter().fold(
        StatementSyntax::Predicate(predicate),
        |leading_statement, continuation| StatementSyntax::ExperimentalPredicateContinuation {
            leading_statement: Box::new(leading_statement),
            continuation,
        },
    )
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.clone().words().len() >= old(leading_statement.clone().words().len()))]
fn build_connected_statement(
    leading_statement: StatementSyntax,
    continuations: Vec<(
        bool,
        WithIndicators<WordLike>,
        ConnectiveSyntax,
        StatementSyntax,
    )>,
) -> StatementSyntax {
    let mut statements = vec![leading_statement];
    let mut connectors = Vec::new();
    for (pre_i, i, connective, trailing_statement) in continuations {
        connectors.push((pre_i, i, connective));
        statements.push(trailing_statement);
    }

    for index in (0..connectors.len()).rev() {
        if connective_has_bo(&connectors[index].2) {
            let trailing_statement = statements.remove(index + 1);
            let leading_statement = statements.remove(index);
            let (pre_i, i, connective) = connectors.remove(index);
            statements.insert(
                index,
                if pre_i {
                    StatementSyntax::PreIConnected {
                        connective,
                        i,
                        leading_statement: Box::new(leading_statement),
                        trailing_statement: Box::new(trailing_statement),
                    }
                } else {
                    StatementSyntax::Connected {
                        i,
                        connective,
                        leading_statement: Box::new(leading_statement),
                        trailing_statement: Box::new(trailing_statement),
                    }
                },
            );
        }
    }

    let mut statements = statements.into_iter();
    let mut connected_statement = statements
        .next()
        .expect("there is always at least the leading statement");
    for ((pre_i, i, connective), trailing_statement) in connectors.into_iter().zip(statements) {
        connected_statement = if pre_i {
            StatementSyntax::PreIConnected {
                connective,
                i,
                leading_statement: Box::new(connected_statement),
                trailing_statement: Box::new(trailing_statement),
            }
        } else {
            StatementSyntax::Connected {
                i,
                connective,
                leading_statement: Box::new(connected_statement),
                trailing_statement: Box::new(trailing_statement),
            }
        };
    }
    connected_statement
}

#[requires(true)]
#[ensures(ret == connective.cmavo.iter().any(|word| cmavo_text_matches(word, "bo")))]
fn connective_has_bo(connective: &ConnectiveSyntax) -> bool {
    connective
        .cmavo
        .iter()
        .any(|word| cmavo_text_matches(word, "bo"))
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_ke_continuation_allowed(
    first: &PredicateTail1Syntax,
    ke_continuation: &KePredicateTailSyntax,
) -> bool {
    !predicate_tail1_has_tail_terms(first) || connective_is_gihek(&ke_continuation.connective)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail1_has_tail_terms(predicate_tail: &PredicateTail1Syntax) -> bool {
    predicate_tail2_has_tail_terms(&predicate_tail.first)
        || predicate_tail.continuations.iter().any(|continuation| {
            !continuation.tail_terms.is_empty()
                || predicate_tail2_has_tail_terms(&continuation.predicate_tail)
        })
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2_has_tail_terms(predicate_tail: &PredicateTail2Syntax) -> bool {
    predicate_tail3_has_tail_terms(&predicate_tail.first)
        || predicate_tail
            .bo_continuation
            .as_ref()
            .is_some_and(|continuation| {
                !continuation.tail_terms.is_empty()
                    || predicate_tail2_has_tail_terms(&continuation.predicate_tail)
            })
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail3_has_tail_terms(predicate_tail: &PredicateTail3Syntax) -> bool {
    match predicate_tail {
        PredicateTail3Syntax::Relation { terms, .. } => !terms.is_empty(),
        PredicateTail3Syntax::GekSentence(gek_sentence) => {
            gek_sentence_has_tail_terms(gek_sentence)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn gek_sentence_has_tail_terms(gek_sentence: &GekSentenceSyntax) -> bool {
    match gek_sentence {
        GekSentenceSyntax::Pair { tail_terms, .. } => !tail_terms.is_empty(),
        GekSentenceSyntax::Ke { inner, .. } | GekSentenceSyntax::Na { inner, .. } => {
            gek_sentence_has_tail_terms(inner)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_is_gihek(connective: &ConnectiveSyntax) -> bool {
    connective.cmavo.iter().any(|word| {
        ["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"]
            .iter()
            .any(|text| cmavo_text_matches(word, text))
    })
}

#[requires(true)]
#[ensures(true)]
fn empty_text() -> TextSyntax {
    TextSyntax {
        leading_nai: Vec::new(),
        leading_cmevla: Vec::new(),
        leading_indicators: Vec::new(),
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn sei_free<'tokens, T, R, F>(
    term: T,
    relation: R,
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let prohibited_free_modifier = cll_prohibited_free_modifier(free_modifier.clone());
    cmavo_of("SEI", &["sei", "ti'o", "xoi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(term.repeated().collect::<Vec<_>>())
        .then(
            cmavo("cu")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .then(relation)
        .then(
            cmavo("se'u")
                .then(
                    prohibited_free_modifier
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(
            |(((((sei, leading_free_modifiers), terms), cu), relation), sehu)| {
                FreeModifierSyntax::Sei {
                    sei: WithFreeModifiers::new(sei, leading_free_modifiers),
                    terms,
                    cu: cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)),
                    relation,
                    sehu: sehu
                        .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
                }
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn to_free<'tokens, T, F>(text: T, free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let prohibited_free_modifier = cll_prohibited_free_modifier(free_modifier.clone());
    let empty_parenthetical = cmavo_of("TO", &["to'i", "to"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmavo("toi"))
        .then(
            prohibited_free_modifier
                .clone()
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(
            move |(((to, free_modifiers), toi), toi_free_modifiers)| FreeModifierSyntax::To {
                to: WithFreeModifiers::new(to, free_modifiers),
                text: Box::new(empty_text()),
                toi: Some(WithFreeModifiers::new(toi, toi_free_modifiers)),
            },
        );

    let nonempty_parenthetical = cmavo_of("TO", &["to'i", "to"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(
            cmavo("toi")
                .then(
                    prohibited_free_modifier
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(
            |(((to, free_modifiers), text), toi)| FreeModifierSyntax::To {
                to: WithFreeModifiers::new(to, free_modifiers),
                text: Box::new(text),
                toi: toi.map(|(toi, free_modifiers)| WithFreeModifiers::new(toi, free_modifiers)),
            },
        );

    choice((empty_parenthetical, nonempty_parenthetical)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn replacement_free<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let full_replacement = cmavo("lo'ai")
        .then(raw_words_until(&["sa'ai", "le'ai"]))
        .then(cmavo("sa'ai").or_not())
        .then(raw_words_until(&["le'ai"]))
        .then(cmavo("le'ai"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((lohai, old_words), sahai), new_words), lehai), free_modifiers)| {
                FreeModifierSyntax::Replacement {
                    lohai: Some(lohai),
                    old_words,
                    sahai,
                    new_words,
                    lehai: WithFreeModifiers::new(lehai, free_modifiers),
                }
            },
        );
    let new_only_replacement = cmavo("sa'ai")
        .then(raw_words_until(&["le'ai"]))
        .then(cmavo("le'ai"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((sahai, new_words), lehai), free_modifiers)| FreeModifierSyntax::Replacement {
                lohai: None,
                old_words: Vec::new(),
                sahai: Some(sahai),
                new_words,
                lehai: WithFreeModifiers::new(lehai, free_modifiers),
            },
        );
    let close_only_replacement = cmavo("le'ai")
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(lehai, free_modifiers)| FreeModifierSyntax::Replacement {
            lohai: None,
            old_words: Vec::new(),
            sahai: None,
            new_words: Vec::new(),
            lehai: WithFreeModifiers::new(lehai, free_modifiers),
        });

    choice((
        full_replacement,
        new_only_replacement,
        close_only_replacement,
    ))
    .boxed()
}

#[requires(!terminators.is_empty())]
#[ensures(true)]
fn raw_words_until<'tokens>(
    terminators: &'static [&'static str],
) -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>> {
    token_matching("replacement word", move |word| {
        !terminators
            .iter()
            .any(|terminator| cmavo_text_matches(word, terminator))
    })
    .repeated()
    .collect::<Vec<_>>()
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body<'tokens>() -> BoxedParser<'tokens, MathExpressionSyntax> {
    math_parser_pair().0
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context<'tokens, A, R, F>(
    argument: A,
    relation: R,
    free_modifier: F,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    math_parser_pair_with_context(argument, relation, free_modifier).0
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair_with_context<'tokens, A, R, F>(
    argument: A,
    relation: R,
    free_modifier: F,
) -> (
    BoxedParser<'tokens, MathExpressionSyntax>,
    BoxedParser<'tokens, MathOperatorSyntax>,
)
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let mut expression = Recursive::declare();
    let mut operator = Recursive::declare();
    expression.define(math_expression_body_with_context_inner(
        expression.clone(),
        operator.clone(),
        argument.clone(),
        relation.clone(),
        free_modifier,
    ));
    operator.define(math_operator_with_context(
        expression.clone(),
        operator.clone(),
        relation,
    ));
    (expression.boxed(), operator.boxed())
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context_inner<'tokens, E, O, A, R, F>(
    expression: E,
    operator: O,
    argument: A,
    relation: R,
    free_modifier: F,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number = quantifier_with_free_modifiers(number_quantifier(), free_modifier.clone())
        .map(MathExpressionSyntax::Number);
    let letter = letter_string()
        .then_ignore(cmavo_of("MOI", MOI_WORDS).rewind().not())
        .then(cmavo("boi").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((letter, boi), free_modifiers)| MathExpressionSyntax::Letter {
                letter,
                boi,
                free_modifiers,
            },
        );
    let nihe = cmavo("ni'e")
        .then(relation.clone())
        .then(cmavo("te'u").or_not())
        .map(|((nihe, relation), tehu)| MathExpressionSyntax::Nihe {
            nihe,
            free_modifiers: Vec::new(),
            relation,
            tehu,
            tehu_free_modifiers: Vec::new(),
        });
    let mohe = cmavo("mo'e")
        .then(argument)
        .then(cmavo("te'u").or_not())
        .map(|((mohe, argument), tehu)| MathExpressionSyntax::Mohe {
            mohe,
            free_modifiers: Vec::new(),
            argument: Box::new(argument),
            tehu,
            tehu_free_modifiers: Vec::new(),
        });
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo("jo'i")
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo("te'u").or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                MathExpressionSyntax::Johi {
                    johi,
                    free_modifiers,
                    expressions,
                    tehu,
                    tehu_free_modifiers,
                }
            },
        );
    let vei = cmavo("vei")
        .then(expression.clone())
        .then(cmavo("ve'o").or_not())
        .map(
            |((vei, inner_expression), veho)| MathExpressionSyntax::Vei {
                vei,
                free_modifiers: Vec::new(),
                inner_expression: Box::new(inner_expression),
                veho,
                veho_free_modifiers: Vec::new(),
            },
        );
    let gek = modal_forethought_connective_with_free_modifiers(free_modifier.clone())
        .then(expression.clone())
        .then(gik_connective_with_free_modifiers(free_modifier.clone()))
        .then(expression)
        .map(
            |(((gek, left_expression), gik), right_expression)| MathExpressionSyntax::Gek {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            },
        );
    let math_operand_atom = choice((gek, vei, nihe, mohe, johi, number, letter)).boxed();
    let math_operand = recursive(|math_operand| {
        let math_operand2 = recursive(|math_operand2| {
            math_operand_atom
                .clone()
                .then(
                    operand_connective()
                        .then(tense_modal().or_not())
                        .then(cmavo("bo"))
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .then(math_operand2)
                        .or_not(),
                )
                .map(|(left_expression, bo_tail)| {
                    bo_tail.map_or(
                        left_expression.clone(),
                        |((((connective, tense_modal), bo), free_modifiers), right_expression)| {
                            let connective = tense_modal.map_or(connective.clone(), |tag| {
                                append_connective_words(connective, tag.words())
                            });
                            let connective =
                                append_connective_free_modifiers(connective, free_modifiers);
                            let connective = append_connective_words(connective, vec![bo]);
                            MathExpressionSyntax::Connected {
                                left_expression: Box::new(left_expression),
                                connective,
                                right_expression: Box::new(right_expression),
                            }
                        },
                    )
                })
        });
        let math_operand1 = math_operand2
            .clone()
            .then(
                operand_connective()
                    .then(math_operand2)
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first, continuations)| {
                continuations.into_iter().fold(
                    first,
                    |left_expression, (connective, right_expression)| {
                        MathExpressionSyntax::Connected {
                            left_expression: Box::new(left_expression),
                            connective,
                            right_expression: Box::new(right_expression),
                        }
                    },
                )
            });
        math_operand1
            .clone()
            .then(
                operand_connective()
                    .then(tense_modal().or_not())
                    .then(cmavo("ke"))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(math_operand)
                    .then(cmavo("ke'e").or_not())
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(left_expression, grouped_tail)| {
                grouped_tail.map_or(
                    left_expression.clone(),
                    |(
                        (
                            (
                                (((connective, tense_modal), ke), ke_free_modifiers),
                                right_expression,
                            ),
                            kehe,
                        ),
                        kehe_free_modifiers,
                    )| {
                        let connective = tense_modal.map_or(connective.clone(), |tag| {
                            append_connective_words(connective, tag.words())
                        });
                        MathExpressionSyntax::Connected {
                            left_expression: Box::new(left_expression),
                            connective,
                            right_expression: Box::new(MathExpressionSyntax::Vei {
                                vei: ke,
                                free_modifiers: ke_free_modifiers,
                                inner_expression: Box::new(right_expression),
                                veho: kehe,
                                veho_free_modifiers: kehe_free_modifiers,
                            }),
                        }
                    },
                )
            })
            .boxed()
    });
    let math_expression2 = recursive(|math_expression2| {
        let lahe = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(cmavo("bo"))
            .then(math_expression2.clone())
            .then(cmavo("lu'u").or_not())
            .map(
                |(((nahe, bo), inner_expression), luhu)| MathExpressionSyntax::Lahe {
                    markers: vec![nahe, bo],
                    free_modifiers: Vec::new(),
                    inner_expression: Box::new(inner_expression),
                    luhu,
                    luhu_free_modifiers: Vec::new(),
                },
            );
        let forethought = cmavo("pe'o")
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo("ku'e").or_not())
            .map(
                |(((peho, operator), operands), kuhe)| MathExpressionSyntax::Forethought {
                    peho,
                    free_modifiers: Vec::new(),
                    operator,
                    operands,
                    kuhe,
                    kuhe_free_modifiers: Vec::new(),
                },
            );
        choice((math_operand.clone(), lahe, forethought)).boxed()
    });
    let reverse_polish_parts = recursive(|reverse_polish_parts| {
        math_operand
            .clone()
            .then(
                reverse_polish_parts
                    .clone()
                    .then(operator.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first_operand, tails)| {
                let mut operands = vec![first_operand];
                let mut operators = Vec::new();
                for ((mut tail_operands, mut tail_operators), operator) in tails {
                    operands.append(&mut tail_operands);
                    operators.append(&mut tail_operators);
                    operators.push(operator);
                }
                (operands, operators)
            })
    });
    let reverse_polish =
        cmavo("fu'a")
            .then(reverse_polish_parts)
            .map(
                |(fuha, (operands, operators))| MathExpressionSyntax::ReversePolish {
                    fuha,
                    free_modifiers: Vec::new(),
                    operands,
                    operators,
                },
            );
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo("bi'e")
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => MathExpressionSyntax::Bihe {
                    left_expression: Box::new(left_expression),
                    bihe,
                    free_modifiers: Vec::new(),
                    operator,
                    right_expression: Box::new(right_expression),
                },
            })
    });
    let infix_expression = math_expression1
        .clone()
        .then(
            operator
                .then(math_expression1)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (operator, right_expression)| MathExpressionSyntax::Binary {
                    operator,
                    left_expression: Box::new(left_expression),
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair<'tokens>() -> (
    BoxedParser<'tokens, MathExpressionSyntax>,
    BoxedParser<'tokens, MathOperatorSyntax>,
) {
    let mut expression = Recursive::declare();
    let mut operator = Recursive::declare();
    expression.define(math_expression_body_with(
        expression.clone(),
        operator.clone(),
    ));
    operator.define(math_operator_with(expression.clone(), operator.clone()));
    (expression.boxed(), operator.boxed())
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with<'tokens, E, O>(
    expression: E,
    operator: O,
) -> BoxedParser<'tokens, MathExpressionSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number = number_quantifier().map(MathExpressionSyntax::Number);
    let letter = letter_string()
        .then_ignore(cmavo_of("MOI", MOI_WORDS).rewind().not())
        .then(cmavo("boi").or_not())
        .map(|(letter, boi)| MathExpressionSyntax::Letter {
            letter,
            boi,
            free_modifiers: Vec::new(),
        });
    let vei = cmavo("vei")
        .then(expression.clone())
        .then(cmavo("ve'o").or_not())
        .map(
            |((vei, inner_expression), veho)| MathExpressionSyntax::Vei {
                vei,
                free_modifiers: Vec::new(),
                inner_expression: Box::new(inner_expression),
                veho,
                veho_free_modifiers: Vec::new(),
            },
        );
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo("jo'i")
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo("te'u").or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                MathExpressionSyntax::Johi {
                    johi,
                    free_modifiers,
                    expressions,
                    tehu,
                    tehu_free_modifiers,
                }
            },
        );
    let gek = modal_forethought_connective()
        .then(expression.clone())
        .then(gik_connective())
        .then(expression)
        .map(
            |(((gek, left_expression), gik), right_expression)| MathExpressionSyntax::Gek {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            },
        );
    let math_operand_atom = choice((gek, vei, johi, number, letter)).boxed();
    let math_operand = math_operand_atom
        .clone()
        .then(
            operand_connective()
                .then(math_operand_atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (connective, right_expression)| MathExpressionSyntax::Connected {
                    left_expression: Box::new(left_expression),
                    connective,
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();
    let math_expression2 = recursive(|math_expression2| {
        let forethought = cmavo("pe'o")
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo("ku'e").or_not())
            .map(
                |(((peho, operator), operands), kuhe)| MathExpressionSyntax::Forethought {
                    peho,
                    free_modifiers: Vec::new(),
                    operator,
                    operands,
                    kuhe,
                    kuhe_free_modifiers: Vec::new(),
                },
            );
        choice((math_operand.clone(), forethought)).boxed()
    });
    let reverse_polish_parts = recursive(|reverse_polish_parts| {
        math_operand
            .clone()
            .then(
                reverse_polish_parts
                    .clone()
                    .then(operator.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first_operand, tails)| {
                let mut operands = vec![first_operand];
                let mut operators = Vec::new();
                for ((mut tail_operands, mut tail_operators), operator) in tails {
                    operands.append(&mut tail_operands);
                    operators.append(&mut tail_operators);
                    operators.push(operator);
                }
                (operands, operators)
            })
    });
    let reverse_polish =
        cmavo("fu'a")
            .then(reverse_polish_parts)
            .map(
                |(fuha, (operands, operators))| MathExpressionSyntax::ReversePolish {
                    fuha,
                    free_modifiers: Vec::new(),
                    operands,
                    operators,
                },
            );
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo("bi'e")
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => MathExpressionSyntax::Bihe {
                    left_expression: Box::new(left_expression),
                    bihe,
                    free_modifiers: Vec::new(),
                    operator,
                    right_expression: Box::new(right_expression),
                },
            })
    });
    let infix_expression = math_expression1
        .clone()
        .then(
            operator
                .then(math_expression1)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |left_expression, (operator, right_expression)| MathExpressionSyntax::Binary {
                    operator,
                    left_expression: Box::new(left_expression),
                    right_expression: Box::new(right_expression),
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_with<'tokens, B, A, R, S, F>(
    leading_argument: B,
    argument: A,
    relation: R,
    subsentence: S,
    free_modifier: F,
) -> BoxedParser<
    'tokens,
    (
        Vec<ArgumentTailElementSyntax>,
        Option<RelationSyntax>,
        Vec<RelativeClauseSyntax>,
    ),
>
where
    B: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let tail_argument = pa_word()
        .rewind()
        .not()
        .ignore_then(leading_argument)
        .map(|argument| match argument {
            ArgumentSyntax::RelativeClause {
                base_argument,
                vuho: _,
                vuho_free_modifiers: _,
                relative_clauses,
            } => vec![
                ArgumentTailElementSyntax::Argument(base_argument),
                ArgumentTailElementSyntax::RelativeClauses(relative_clauses),
            ],
            argument => vec![ArgumentTailElementSyntax::Argument(Box::new(argument))],
        });
    let contextual_quantifier = quantifier_with_free_modifiers(
        quantifier_with_context(argument.clone(), relation.clone(), free_modifier.clone()),
        free_modifier.clone(),
    );
    let descriptor_relative_clauses =
        relative_clauses(argument.clone(), subsentence, free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);

    let leading_tail_elements = tail_argument
        .or_not()
        .then(descriptor_relative_clauses.clone())
        .map(|(argument, relative_clauses)| {
            let mut tail_elements = argument.into_iter().flatten().collect::<Vec<_>>();
            if !relative_clauses.is_empty() {
                tail_elements.push(ArgumentTailElementSyntax::RelativeClauses(relative_clauses));
            }
            tail_elements
        });

    let relation_tail = relation
        .clone()
        .then(descriptor_relative_clauses.clone())
        .map(|(relation, relative_clauses)| (Vec::new(), Some(relation), relative_clauses));
    let quantifier_relation_tail = contextual_quantifier
        .clone()
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(relation.clone())
        .then(descriptor_relative_clauses.clone())
        .map(|((quantifier, relation), relative_clauses)| {
            (vec![quantifier], Some(relation), relative_clauses)
        });
    let quantifier_argument_tail = contextual_quantifier
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(argument)
        .map(|(quantifier, argument)| {
            (
                vec![
                    quantifier,
                    ArgumentTailElementSyntax::Argument(Box::new(argument)),
                ],
                None,
                Vec::new(),
            )
        });

    leading_tail_elements
        .then(choice((
            quantifier_relation_tail,
            quantifier_argument_tail,
            relation_tail,
        )))
        .map(
            |(mut leading_tail_elements, (tail_elements, relation, relative_clauses))| {
                leading_tail_elements.extend(tail_elements);
                (leading_tail_elements, relation, relative_clauses)
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_parser_with<'tokens, A, R, S, T, F>(
    argument: A,
    relation: R,
    subsentence: impl Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    single_term: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let quote = quote_argument(source, text, free_modifier.clone());

    let math_expression = cmavo_of("LI", &["li", "me'o"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(math_expression_body_with_context(
            argument.clone(),
            relation.clone(),
            free_modifier.clone(),
        ))
        .then(cmavo("lo'o").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((li, li_free_modifiers), expression), loho), loho_free_modifiers)| {
                ArgumentSyntax::MathExpression {
                    li,
                    li_free_modifiers,
                    expression,
                    loho,
                    loho_free_modifiers,
                }
            },
        );

    let letter = letter_string()
        .then_ignore(cmavo_of("MOI", MOI_WORDS).rewind().not())
        .then_ignore(cmavo_of("MAI", MAI_WORDS).rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(
            cmavo("boi")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((letter, letter_free_modifiers), boi)| ArgumentSyntax::Letter {
                letter: WithFreeModifiers::new(letter, letter_free_modifiers),
                boi: boi.map(|(boi, free_modifiers)| WithFreeModifiers::new(boi, free_modifiers)),
            },
        );

    let koha = koha_argument()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(koha, free_modifiers)| ArgumentSyntax::Koha {
            koha,
            free_modifiers,
        });
    let lahe = lahe_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(
            relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                ((((lahe, free_modifiers), relative_clauses), inner_argument), luhu),
                luhu_free_modifiers,
            )| ArgumentSyntax::Lahe {
                lahe,
                free_modifiers,
                relative_clauses,
                inner_argument: Box::new(inner_argument),
                luhu,
                luhu_free_modifiers,
            },
        );
    let lahe_term_wrapper = lahe_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((wrapper, free_modifiers), inner_term), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::Lahe,
                    wrapper,
                    wrapper_bo: None,
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();

    let name = la_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((la, la_free_modifiers), names), name_free_modifiers)| ArgumentSyntax::Name {
                la,
                la_free_modifiers,
                names,
                name_free_modifiers,
            },
        );

    let contextual_quantifier = quantifier_with_free_modifiers(
        quantifier_with_context(argument.clone(), relation.clone(), free_modifier.clone()),
        free_modifier.clone(),
    );
    let mut argument6 = Recursive::declare();
    let descriptor_tail = argument_tail_with(
        argument6.clone(),
        argument.clone(),
        relation.clone(),
        subsentence.clone(),
        free_modifier.clone(),
    );
    let descriptor_head = le_cmavo()
        .or(la_cmavo())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(descriptor, descriptor_free_modifiers)| DescriptorHeadSyntax {
                descriptor: WithFreeModifiers::new(descriptor, descriptor_free_modifiers),
            },
        );
    let descriptor_head_connective = jek_connective().map(|connective| ConnectiveSyntax {
        kind: ConnectiveKind::Afterthought,
        se: connective.se,
        nahe: connective.nahe,
        na: connective.na,
        cmavo: connective.cmavo,
        nai: connective.nai,
        free_modifiers: connective.free_modifiers,
    });
    let connected_descriptor = descriptor_head
        .clone()
        .then(descriptor_head_connective)
        .then(descriptor_head)
        .then(descriptor_tail.clone())
        .then(
            cmavo("ku")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                (
                    ((leading_descriptor_head, connective), trailing_descriptor_head),
                    descriptor_tail,
                ),
                ku,
            )| {
                let (tail_elements, relation, relative_clauses) = descriptor_tail;
                ArgumentSyntax::ConnectedDescriptor(ConnectedDescriptorSyntax {
                    leading_descriptor_head,
                    connective,
                    trailing_descriptor_head,
                    tail_elements,
                    relation,
                    relative_clauses,
                    ku: ku.map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                })
            },
        );

    let descriptor_with_gadri = le_cmavo()
        .or(la_cmavo())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(descriptor_tail.clone())
        .then(
            cmavo("ku")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((descriptor, descriptor_free_modifiers), descriptor_tail), ku)| {
                let (tail_elements, relation, relative_clauses) = descriptor_tail;
                ArgumentSyntax::Descriptor(DescriptorSyntax {
                    descriptor: Some(WithFreeModifiers::new(
                        descriptor,
                        descriptor_free_modifiers,
                    )),
                    outer_quantifier: None,
                    tail_elements,
                    relation,
                    relative_clauses,
                    ku: ku.map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                })
            },
        );
    let descriptor_with_outer_quantifier = contextual_quantifier
        .clone()
        .then(le_cmavo().or(la_cmavo()))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(descriptor_tail.clone())
        .then(
            cmavo("ku")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                (((outer_quantifier, descriptor), descriptor_free_modifiers), descriptor_tail),
                ku,
            )| {
                let (tail_elements, relation, relative_clauses) = descriptor_tail;
                ArgumentSyntax::Descriptor(DescriptorSyntax {
                    descriptor: Some(WithFreeModifiers::new(
                        descriptor,
                        descriptor_free_modifiers,
                    )),
                    outer_quantifier: Some(outer_quantifier),
                    tail_elements,
                    relation,
                    relative_clauses,
                    ku: ku.map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                })
            },
        );

    let descriptor_without_gadri = contextual_quantifier
        .clone()
        .map(ArgumentTailElementSyntax::Quantifier)
        .then(relation.clone())
        .then(
            relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .map(|((quantifier, relation), relative_clauses)| {
            ArgumentSyntax::Descriptor(DescriptorSyntax {
                descriptor: None,
                outer_quantifier: None,
                tail_elements: vec![quantifier],
                relation: Some(relation),
                relative_clauses,
                ku: None,
            })
        });

    let nahe_bo_argument = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((nahe, bo), free_modifiers), inner_argument), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::NaheBo {
                    nahe,
                    bo,
                    free_modifiers,
                    inner_argument: Box::new(inner_argument),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        );
    let nahe_bo_term_wrapper = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                ((((wrapper, wrapper_bo), free_modifiers), inner_term), luhu),
                luhu_free_modifiers,
            )| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::NaheBo,
                    wrapper,
                    wrapper_bo: Some(wrapper_bo),
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let nahe_argument = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo").rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((nahe, _), free_modifiers), inner_argument), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::Nahe {
                    nahe,
                    free_modifiers,
                    inner_argument: Box::new(inner_argument),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let nahe_term_wrapper = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo("bo").rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(cmavo("lu'u").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((wrapper, _), free_modifiers), inner_term), luhu), luhu_free_modifiers)| {
                ArgumentSyntax::TermWrapped {
                    term_wrapper_kind: TermWrapperKindSyntax::Nahe,
                    wrapper,
                    wrapper_bo: None,
                    free_modifiers,
                    inner_term: Box::new(inner_term),
                    luhu,
                    luhu_free_modifiers,
                }
            },
        )
        .boxed();
    let bridi_description = cmavo_of("LOhOI", &["lo'oi", "mau'a", "xau'a"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence.clone())
        .then(cmavo("ku'au").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((((lohoi, lohoi_free_modifiers), subsentence), kuhau), kuhau_free_modifiers)| {
                ArgumentSyntax::BridiDescription {
                    lohoi,
                    lohoi_free_modifiers,
                    subsentence: Box::new(subsentence),
                    kuhau,
                    kuhau_free_modifiers,
                }
            },
        )
        .boxed();
    let quoted_or_simple_argument_core = choice((
        quote,
        math_expression,
        letter,
        lahe,
        lahe_term_wrapper,
        name,
        bridi_description,
    ))
    .boxed();
    let tagged_or_negated_argument_core = choice((
        nahe_bo_argument,
        nahe_bo_term_wrapper,
        nahe_argument,
        nahe_term_wrapper,
    ))
    .boxed();
    let descriptor_argument_core = choice((
        connected_descriptor,
        descriptor_with_outer_quantifier,
        descriptor_with_gadri,
        descriptor_without_gadri,
        koha,
    ))
    .boxed();
    let unquantified_base_argument_core = choice((
        quoted_or_simple_argument_core,
        tagged_or_negated_argument_core,
        descriptor_argument_core,
    ))
    .boxed();
    argument6.define(unquantified_base_argument_core.clone());
    let base_relative_clauses =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let unquantified_base_argument = unquantified_base_argument_core
        .clone()
        .then(base_relative_clauses.clone())
        .map(|(base_argument, relative_clauses)| {
            if relative_clauses.is_empty() {
                base_argument
            } else {
                ArgumentSyntax::RelativeClause {
                    base_argument: Box::new(base_argument),
                    vuho: None,
                    vuho_free_modifiers: Vec::new(),
                    relative_clauses,
                }
            }
        });
    let quantified_argument = contextual_quantifier
        .then(unquantified_base_argument_core)
        .then(base_relative_clauses)
        .map(|((quantifier, inner_argument), relative_clauses)| {
            let quantified = ArgumentSyntax::Quantified {
                quantifier,
                inner_argument: Box::new(inner_argument),
            };
            if relative_clauses.is_empty() {
                quantified
            } else {
                ArgumentSyntax::RelativeClause {
                    base_argument: Box::new(quantified),
                    vuho: None,
                    vuho_free_modifiers: Vec::new(),
                    relative_clauses,
                }
            }
        });
    let base_argument = choice((unquantified_base_argument, quantified_argument));

    let argument4 = recursive(|argument4| {
        let gek_argument = modal_forethought_connective_with_free_modifiers(free_modifier.clone())
            .then(argument.clone())
            .then(gik_connective_with_free_modifiers(free_modifier.clone()))
            .then(argument4)
            .map(
                |(((gek, leading_argument), gik), trailing_argument)| ArgumentSyntax::Gek {
                    gek,
                    leading_argument: Box::new(leading_argument),
                    gik,
                    trailing_argument: Box::new(trailing_argument),
                },
            );

        choice((gek_argument, base_argument.clone())).boxed()
    });
    let argument3 = recursive(|argument3| {
        argument4
            .clone()
            .then(
                connective_with_free_modifiers(argument_connective(), free_modifier.clone())
                    .then(tense_modal().or_not())
                    .then(cmavo("bo"))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(argument3)
                    .or_not(),
            )
            .map(|(leading_argument, bo_tail)| {
                bo_tail.map_or(
                    leading_argument.clone(),
                    |(
                        (((bo_connective, bo_tense_modal), bo), free_modifiers),
                        trailing_argument,
                    )| {
                        ArgumentSyntax::Bo {
                            leading_argument: Box::new(leading_argument),
                            bo_connective: Some(bo_connective),
                            bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_argument: Box::new(trailing_argument),
                        }
                    },
                )
            })
            .boxed()
    });
    let afterthought_argument_tail =
        connective_with_free_modifiers(argument_connective(), free_modifier.clone())
            .then(argument3.clone())
            .boxed();
    let argument2 = argument3
        .clone()
        .then(
            afterthought_argument_tail
                .clone()
                .rewind()
                .ignore_then(afterthought_argument_tail)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations.into_iter().fold(
                first,
                |leading_argument, (connective, trailing_argument)| ArgumentSyntax::Connected {
                    leading_argument: Box::new(leading_argument),
                    connective,
                    trailing_argument: Box::new(trailing_argument),
                },
            )
        })
        .boxed();

    let argument_ke_tail =
        connective_with_free_modifiers(argument_connective(), free_modifier.clone())
            .then(tense_modal().or_not())
            .then(cmavo("ke"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(argument.clone())
            .then(cmavo("ke'e").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .boxed();
    let argument1 = argument2
        .clone()
        .then(
            argument_ke_tail
                .clone()
                .rewind()
                .ignore_then(argument_ke_tail)
                .or_not(),
        )
        .map(|(leading_argument, ke_tail)| {
            ke_tail.map_or(
                leading_argument.clone(),
                |(
                    (((((connective, tense_modal), ke), ke_free_modifiers), inner_argument), kehe),
                    kehe_free_modifiers,
                )| {
                    let connective = tense_modal.map_or(connective.clone(), |tense_modal| {
                        append_connective_words(connective, tense_modal.words())
                    });
                    ArgumentSyntax::Connected {
                        leading_argument: Box::new(leading_argument),
                        connective,
                        trailing_argument: Box::new(ArgumentSyntax::Ke {
                            ke,
                            ke_free_modifiers,
                            inner_argument: Box::new(inner_argument),
                            kehe,
                            kehe_free_modifiers,
                        }),
                    }
                },
            )
        })
        .boxed();

    argument1
        .then(
            cmavo("vu'o")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(
                    relative_clauses(argument.clone(), subsentence, free_modifier.clone())
                        .or_not()
                        .map(Option::unwrap_or_default),
                )
                .then(
                    argument_connective()
                        .then(argument)
                        .map(|(connective, argument)| ArgumentConnectionSyntax {
                            connective,
                            argument: Box::new(argument),
                        })
                        .or_not(),
                )
                .or_not(),
        )
        .map(|(base_argument, vuho_attachment)| {
            if let Some((((vuho, vuho_free_modifiers), relative_clauses), connected_argument)) =
                vuho_attachment
            {
                if !relative_clauses.is_empty() && connected_argument.is_none() {
                    ArgumentSyntax::RelativeClause {
                        base_argument: Box::new(base_argument),
                        vuho: Some(vuho),
                        vuho_free_modifiers,
                        relative_clauses,
                    }
                } else {
                    ArgumentSyntax::Vuho {
                        base_argument: Box::new(base_argument),
                        vuho_marker: vuho,
                        vuho_free_modifiers,
                        relative_clauses,
                        connected_argument,
                    }
                }
            } else {
                base_argument
            }
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn implicit_zohe_argument() -> ArgumentSyntax {
    ArgumentSyntax::Zohe {
        tag_words: Vec::new(),
        maybe_ku: None,
        free_modifiers: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn letter_string<'tokens>() -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>> {
    recursive(|letter_string| {
        let letter_tokens = letter_word_tokens_from(letter_string.clone());
        let continuation = choice((pa_word().map(|word| vec![word]), letter_tokens.clone()))
            .repeated()
            .collect::<Vec<_>>();
        letter_tokens.then(continuation).map(|(mut first, rest)| {
            for mut group in rest {
                first.append(&mut group);
            }
            first
        })
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_words<'tokens>() -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>> {
    let letter_tokens = letter_word_tokens_from(letter_string());
    pa_word()
        .map(|word| vec![word])
        .then(
            choice((pa_word().map(|word| vec![word]), letter_tokens))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(mut first, rest)| {
            for mut group in rest {
                first.append(&mut group);
            }
            first
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_or_letter_words<'tokens>() -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>> {
    choice((number_words(), letter_string())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn letter_word_tokens_from<'tokens, L>(
    letter_string: L,
) -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>>
where
    L: Parser<'tokens, ParserInput<'tokens>, Vec<WithIndicators<WordLike>>, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive(|letter_tokens| {
        let by = letter_word().map(|word| vec![word]);
        let lau = cmavo_of("LAU", LAU_WORDS)
            .then(letter_tokens.clone())
            .map(|(lau, mut rest)| {
                let mut words = vec![lau];
                words.append(&mut rest);
                words
            });
        let tei = cmavo("tei")
            .then(letter_string.clone())
            .then(cmavo("foi"))
            .map(|((tei, mut inner), foi)| {
                let mut words = vec![tei];
                words.append(&mut inner);
                words.push(foi);
                words
            });
        choice((by, lau, tei)).boxed()
    })
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_quantifier<'tokens>() -> BoxedParser<'tokens, QuantifierSyntax> {
    number_words()
        .then(cmavo("boi").or_not())
        .map(|(number, boi)| QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier<'tokens>() -> BoxedParser<'tokens, QuantifierSyntax> {
    let vei_quantifier = cmavo("vei")
        .then(math_expression_body())
        .then(cmavo("ve'o").or_not())
        .map(|((vei, math_expression), veho)| QuantifierSyntax::Vei {
            vei,
            free_modifiers: Vec::new(),
            math_expression: Box::new(math_expression),
            veho,
            veho_free_modifiers: Vec::new(),
        });
    choice((vei_quantifier, number_quantifier())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_context<'tokens, A, R, F>(
    argument: A,
    relation: R,
    free_modifier: F,
) -> BoxedParser<'tokens, QuantifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let vei_quantifier = cmavo("vei")
        .then(math_expression_body_with_context(
            argument,
            relation,
            free_modifier,
        ))
        .then(cmavo("ve'o").or_not())
        .map(|((vei, math_expression), veho)| QuantifierSyntax::Vei {
            vei,
            free_modifiers: Vec::new(),
            math_expression: Box::new(math_expression),
            veho,
            veho_free_modifiers: Vec::new(),
        });
    choice((vei_quantifier, number_quantifier())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_free_modifiers<'tokens, Q, F>(
    quantifier: Q,
    free_modifier: F,
) -> BoxedParser<'tokens, QuantifierSyntax>
where
    Q: Parser<'tokens, ParserInput<'tokens>, QuantifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    quantifier
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(quantifier, free_modifiers)| {
            attach_quantifier_free_modifiers(quantifier, free_modifiers)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn attach_quantifier_free_modifiers(
    quantifier: QuantifierSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> QuantifierSyntax {
    match quantifier {
        QuantifierSyntax::Number {
            number,
            boi,
            free_modifiers: mut existing_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            QuantifierSyntax::Number {
                number,
                boi,
                free_modifiers: existing_free_modifiers,
            }
        }
        QuantifierSyntax::Vei {
            vei,
            free_modifiers: mut existing_free_modifiers,
            math_expression,
            veho,
            veho_free_modifiers,
        } => {
            existing_free_modifiers.extend(free_modifiers);
            QuantifierSyntax::Vei {
                vei,
                free_modifiers: existing_free_modifiers,
                math_expression,
                veho,
                veho_free_modifiers,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_argument<'tokens, T, F>(
    source: Option<&'tokens str>,
    text: T,
    free_modifier: F,
) -> BoxedParser<'tokens, ArgumentSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let compound_quote = any()
        .try_map(move |word: WithIndicators<WordLike>, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, "expected quote"));
            };

            match word_like.as_data() {
                data!(WordLike::ZoQuote { word: quoted, .. }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::Zo {
                        zo: word.clone(),
                        word: WithFreeModifiers::new(
                            base_word_from_record((**quoted).clone()),
                            Vec::new(),
                        ),
                    },
                    free_modifiers: Vec::new(),
                }),
                data!(WordLike::ZoiQuote {
                    zoi: _,
                    opening_delimiter,
                    quoted_text,
                    closing_delimiter,
                    ..
                }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::Zoi {
                        zoi: word.clone(),
                        opening_delimiter: base_word_from_record((**opening_delimiter).clone()),
                        closing_delimiter: WithFreeModifiers::new(
                            base_word_from_record((**closing_delimiter).clone()),
                            Vec::new(),
                        ),
                        quoted_text: source_text(source, quoted_text),
                    },
                    free_modifiers: Vec::new(),
                }),
                data!(WordLike::LohuQuote {
                    quoted_words,
                    lehu,
                    ..
                }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::Lohu {
                        lohu: word.clone(),
                        quoted_words: quoted_words
                            .iter()
                            .cloned()
                            .map(base_word_from_record)
                            .collect(),
                        lehu: WithFreeModifiers::new(
                            base_word_from_record((**lehu).clone()),
                            Vec::new(),
                        ),
                    },
                    free_modifiers: Vec::new(),
                }),
                data!(WordLike::SingleWordQuote {
                    marker: _,
                    quoted_text,
                }) => Ok(ArgumentSyntax::Quote {
                    quote: QuoteSyntax::ZohOi {
                        zohoi: WithFreeModifiers::new(word.clone(), Vec::new()),
                        quoted_text: source_text(source, quoted_text),
                    },
                    free_modifiers: Vec::new(),
                }),
                _ => Err(Rich::custom(span, "expected quote")),
            }
        })
        .map_with(
            |argument,
             extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            if let ArgumentSyntax::Quote {
                quote: QuoteSyntax::ZohOi { zohoi, .. },
                ..
            } = &argument
            {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZohOiQuote, &zohoi.value);
            }
            argument
        })
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(argument, free_modifiers)| attach_quote_free_modifiers(argument, free_modifiers));

    let lu_quote = cmavo("lu")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(
            cmavo("li'u")
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((lu, free_modifiers), text), lihu)| ArgumentSyntax::Quote {
                quote: QuoteSyntax::Lu {
                    lu: WithFreeModifiers::new(lu, free_modifiers),
                    text,
                    lihu: lihu
                        .map(|(lihu, free_modifiers)| WithFreeModifiers::new(lihu, free_modifiers)),
                },
                free_modifiers: Vec::new(),
            },
        );

    choice((compound_quote, lu_quote)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn attach_quote_free_modifiers(
    argument: ArgumentSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> ArgumentSyntax {
    match argument {
        ArgumentSyntax::Quote {
            quote,
            free_modifiers: argument_free_modifiers,
        } => ArgumentSyntax::Quote {
            quote: quote_with_free_modifiers(quote, free_modifiers),
            free_modifiers: argument_free_modifiers,
        },
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_with_free_modifiers(
    quote: QuoteSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> QuoteSyntax {
    match quote {
        QuoteSyntax::Lu { mut lu, text, lihu } => {
            lu.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Lu { lu, text, lihu }
        }
        QuoteSyntax::Zo { zo, mut word } => {
            word.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Zo { zo, word }
        }
        QuoteSyntax::ZohOi {
            mut zohoi,
            quoted_text,
        } => {
            zohoi.free_modifiers.extend(free_modifiers);
            QuoteSyntax::ZohOi { zohoi, quoted_text }
        }
        QuoteSyntax::Zoi {
            zoi,
            opening_delimiter,
            mut closing_delimiter,
            quoted_text,
        } => {
            closing_delimiter.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Zoi {
                zoi,
                opening_delimiter,
                closing_delimiter,
                quoted_text,
            }
        }
        QuoteSyntax::Laho {
            laho,
            opening_delimiter,
            mut closing_delimiter,
            quoted_text,
        } => {
            closing_delimiter.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Laho {
                laho,
                opening_delimiter,
                closing_delimiter,
                quoted_text,
            }
        }
        QuoteSyntax::Lohu {
            lohu,
            quoted_words,
            mut lehu,
        } => {
            lehu.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Lohu {
                lohu,
                quoted_words,
                lehu,
            }
        }
        QuoteSyntax::Meho {
            mut meho,
            math_expression,
        } => {
            meho.free_modifiers.extend(free_modifiers);
            QuoteSyntax::Meho {
                meho,
                math_expression,
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_word_like(word: &WithIndicators<WordLike>) -> Option<&WordLike> {
    match word {
        WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
            Some(word_like)
        }
        WithIndicators::WithIndicator { base, .. } => quote_word_like(base),
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_clauses<'tokens, A, S>(
    argument: A,
    subsentence: S,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, Vec<RelativeClauseSyntax>>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let clause = relative_clause(argument, subsentence, free_modifier.clone());
    clause
        .clone()
        .then(
            choice((
                cmavo("zi'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(clause.clone())
                    .map(
                        |((zihe, free_modifiers), inner)| RelativeClauseSyntax::Zihe {
                            zihe: WithFreeModifiers::new(zihe, free_modifiers),
                            inner: Box::new(inner),
                        },
                    ),
                relative_clause_connective()
                    .then(clause)
                    .map(|(connective, inner)| RelativeClauseSyntax::Connected {
                        connective,
                        inner: Box::new(inner),
                    }),
            ))
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn relative_clause<'tokens, R>(
    argument: impl Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    subsentence: R,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, RelativeClauseSyntax>
where
    R: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let goi = goi_relative_clause(argument, free_modifier.clone()).map(RelativeClauseSyntax::Goi);
    let noi = cmavo_of("NOI", &["poi", "noi", "voi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subsentence)
        .then(
            cmavo("ku'o")
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((marker, leading_free_modifiers), subsentence), kuho)| {
            if cmavo_text_matches(&marker, "poi") {
                RelativeClauseSyntax::Poi {
                    poi: WithFreeModifiers::new(marker, leading_free_modifiers),
                    subsentence,
                    kuho: kuho
                        .map(|(kuho, free_modifiers)| WithFreeModifiers::new(kuho, free_modifiers)),
                }
            } else {
                RelativeClauseSyntax::Noi {
                    noi: WithFreeModifiers::new(marker, leading_free_modifiers),
                    subsentence,
                    kuho: kuho
                        .map(|(kuho, free_modifiers)| WithFreeModifiers::new(kuho, free_modifiers)),
                }
            }
        });
    choice((goi, noi)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause<'tokens, A>(
    argument: A,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, GoiRelativeClauseSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let tagged_tail = argument
        .clone()
        .map(|argument| (Some(argument), None, Vec::new()))
        .or(cmavo("ku")
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)))
        .boxed();
    let tense_tagged_argument = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail.clone())
        .map(
            |(
                (tense_modal, mut tag_free_modifiers),
                (argument, maybe_ku, trailing_free_modifiers),
            )| {
                let tag_words = tense_modal.clone().leaf_words();
                tag_free_modifiers.extend(tense_modal.clone().free_modifiers());
                if let Some(argument) = argument {
                    ArgumentSyntax::Tagged {
                        tag_words,
                        tag_tense_modal: Some(tense_modal),
                        tag_fa: None,
                        free_modifiers: tag_free_modifiers,
                        inner_argument: Box::new(argument),
                    }
                } else {
                    tag_free_modifiers.extend(trailing_free_modifiers);
                    ArgumentSyntax::Zohe {
                        tag_words,
                        maybe_ku,
                        free_modifiers: tag_free_modifiers,
                    }
                }
            },
        );
    let fa_tagged_argument = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail)
        .map(
            |((fa, mut fa_free_modifiers), (argument, maybe_ku, trailing_free_modifiers))| {
                if let Some(argument) = argument {
                    ArgumentSyntax::Tagged {
                        tag_words: vec![fa.clone()],
                        tag_tense_modal: None,
                        tag_fa: Some(fa),
                        free_modifiers: fa_free_modifiers,
                        inner_argument: Box::new(argument),
                    }
                } else {
                    fa_free_modifiers.extend(trailing_free_modifiers);
                    ArgumentSyntax::Zohe {
                        tag_words: vec![fa],
                        maybe_ku,
                        free_modifiers: fa_free_modifiers,
                    }
                }
            },
        );
    let argument_base = argument
        .clone()
        .or(tense_tagged_argument)
        .or(fa_tagged_argument)
        .or(na_ku_argument_parser(free_modifier.clone()))
        .boxed();

    cmavo_of("GOI", &["pe", "ne", "po", "po'e", "po'u", "no'u", "goi"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument_base)
        .then(
            cmavo("ge'u")
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((goi, leading_free_modifiers), argument), gehu)| GoiRelativeClauseSyntax {
                goi: WithFreeModifiers::new(goi, leading_free_modifiers),
                argument,
                gehu: gehu
                    .map(|(gehu, free_modifiers)| WithFreeModifiers::new(gehu, free_modifiers)),
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn na_ku_argument_parser<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, ArgumentSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    na_cmavo()
        .then(cmavo("ku"))
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|((na, ku), free_modifiers)| ArgumentSyntax::NaKu {
            na,
            ku,
            free_modifiers,
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn xi_free<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number_or_letter = number_or_letter_words()
        .then(cmavo("boi").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, boi), free_modifiers)| {
            MathExpressionSyntax::Number(QuantifierSyntax::Number {
                number,
                boi,
                free_modifiers,
            })
        });
    let xi_expression = choice((number_or_letter, math_expression_body()));

    cmavo_of("XI", &["xi", "te'ai"])
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(xi_expression)
        .map(
            |((xi, free_modifiers), expression)| FreeModifierSyntax::Xi {
                xi: WithFreeModifiers::new(xi, free_modifiers),
                expression,
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn mai_free<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    number_or_letter_words()
        .then(cmavo_of("MAI", MAI_WORDS))
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|((number, mai), free_modifiers)| FreeModifierSyntax::Mai {
            number,
            mai: WithFreeModifiers::new(mai, free_modifiers),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn soi_free<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let prohibited_free_modifier = cll_prohibited_free_modifier(free_modifier.clone());
    cmavo("soi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument.clone())
        .then(argument.or_not())
        .then(
            cmavo("se'u")
                .then(prohibited_free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((soi, free_modifiers), leading_argument), trailing_argument), sehu)| {
                FreeModifierSyntax::Soi {
                    soi: WithFreeModifiers::new(soi, free_modifiers),
                    leading_argument: Box::new(leading_argument),
                    trailing_argument: trailing_argument.map(Box::new),
                    sehu: sehu
                        .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
                }
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vocative_free<'tokens, A, R>(
    argument: A,
    relation: R,
    subsentence: impl Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let optional_relative_clauses =
        relative_clauses(argument.clone(), subsentence.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let relation_vocative = optional_relative_clauses
        .clone()
        .then(relation)
        .then(optional_relative_clauses.clone())
        .map(
            |((leading_relative_clauses, relation), trailing_relative_clauses)| {
                ArgumentSyntax::RelationVocative {
                    leading_relative_clauses,
                    relation,
                    trailing_relative_clauses,
                }
            },
        );
    let cmevla_vocative = optional_relative_clauses
        .clone()
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(optional_relative_clauses)
        .map(
            |(((leading_relative_clauses, cmevla), free_modifiers), trailing_relative_clauses)| {
                let argument = ArgumentSyntax::Cmevla {
                    cmevla,
                    free_modifiers,
                };
                let relative_clauses = leading_relative_clauses
                    .into_iter()
                    .chain(trailing_relative_clauses)
                    .collect::<Vec<_>>();
                if relative_clauses.is_empty() {
                    argument
                } else {
                    ArgumentSyntax::RelativeClause {
                        base_argument: Box::new(argument),
                        vuho: None,
                        vuho_free_modifiers: Vec::new(),
                        relative_clauses,
                    }
                }
            },
        );
    let vocative_argument = choice((relation_vocative, cmevla_vocative, argument));

    vocative_markers()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(vocative_argument.or_not())
        .then(
            cmavo("do'u")
                .then(
                    cll_prohibited_free_modifier(free_modifier)
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(|(((vocative_markers, free_modifiers), argument), dohu)| {
            FreeModifierSyntax::Vocative {
                vocative_markers: WithFreeModifiers::new(vocative_markers, free_modifiers),
                argument,
                dohu: dohu
                    .map(|(dohu, free_modifiers)| WithFreeModifiers::new(dohu, free_modifiers)),
            }
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn cll_prohibited_free_modifier<'tokens, F>(
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    free_modifier
        .map_with(
            |free_modifier,
             extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                if let Some(anchor) = free_modifier_anchor(&free_modifier) {
                    extra.state().warn(
                        ExperimentalConstruct::CllProhibitedFreeModifierPlacement,
                        &anchor,
                    );
                }
                free_modifier
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn free_modifier_anchor(free_modifier: &FreeModifierSyntax) -> Option<WithIndicators<WordLike>> {
    free_modifier.clone().words().into_iter().next()
}

#[requires(true)]
#[ensures(true)]
fn vocative_markers<'tokens>() -> BoxedParser<'tokens, Vec<WithIndicators<WordLike>>> {
    let coi_marker = cmavo_of("COI", COI_WORDS)
        .then(cmavo("nai").or_not())
        .map(|(coi, nai)| [vec![coi], nai.into_iter().collect()].concat());

    choice((
        coi_marker
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(cmavo("doi").or_not())
            .map(|(coi_markers, doi)| {
                let mut markers = coi_markers.into_iter().flatten().collect::<Vec<_>>();
                markers.extend(doi);
                markers
            }),
        cmavo("doi").map(|doi| vec![doi]),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let tagged_term_start = choice((tense_modal().ignored(), cmavo_of("FA", FA_WORDS).ignored()));
    let cehe_connective = cmavo("ce'e")
        .then_ignore(tagged_term_start.rewind().not())
        .then(cmavo("nai").or_not())
        .map(|(cmavo, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::NonLogical,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        });
    choice((
        cehe_connective,
        na_cmavo()
            .or_not()
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
            .then(cmavo("nai").or_not())
            .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Afterthought,
                se,
                nahe: None,
                na,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        na_cmavo()
            .or_not()
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("JEhI", &["je'i", "ja", "je", "jo", "ju"]))
            .then(cmavo("nai").or_not())
            .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Afterthought,
                se,
                nahe: None,
                na,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of(
            "JOI",
            &[
                "ce", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a", "pi'u",
            ],
        )
        .then(cmavo("nai").or_not())
        .map(|(cmavo, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::NonLogical,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        }),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Interval,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("GAhO", &["ga'o", "ke'i"])
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .then(cmavo_of("GAhO", &["ga'o", "ke'i"]))
            .map(
                |((((left_interval, se), cmavo), nai), right_interval)| ConnectiveSyntax {
                    kind: ConnectiveKind::Interval,
                    se,
                    nahe: None,
                    na: None,
                    cmavo: vec![left_interval, cmavo, right_interval],
                    nai,
                    free_modifiers: Vec::new(),
                },
            ),
        vuhu_nonlogical_connective(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn joik_ek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), ek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn operand_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), ek_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn term_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        joik_connective(),
        jek_connective(),
        ek_connective(),
        vuhu_nonlogical_connective(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn ek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("A", &["a", "e", "o", "u", "ji"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Afterthought,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vuhu_nonlogical_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    cmavo_of("VUhU", VUHU_WORDS)
        .map(|cmavo| ConnectiveSyntax {
            kind: ConnectiveKind::NonLogical,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![cmavo],
            nai: None,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn connective_with_free_modifiers<'tokens, C, F>(
    connective: C,
    free_modifier: F,
) -> BoxedParser<'tokens, ConnectiveSyntax>
where
    C: Parser<'tokens, ParserInput<'tokens>, ConnectiveSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    connective
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            append_connective_free_modifiers(connective, free_modifiers)
        })
        .boxed()
}

#[requires(true)]
#[ensures(ret.free_modifiers.len() >= old(connective.free_modifiers.len()))]
fn append_connective_free_modifiers(
    connective: ConnectiveSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> ConnectiveSyntax {
    let mut existing_free_modifiers = connective.free_modifiers;
    existing_free_modifiers.extend(free_modifiers);
    ConnectiveSyntax {
        kind: connective.kind,
        se: connective.se,
        nahe: connective.nahe,
        na: connective.na,
        cmavo: connective.cmavo,
        nai: connective.nai,
        free_modifiers: existing_free_modifiers,
    }
}

#[requires(true)]
#[ensures(ret.cmavo.len() >= old(words.len()))]
fn append_connective_words(
    connective: ConnectiveSyntax,
    words: Vec<WithIndicators<WordLike>>,
) -> ConnectiveSyntax {
    let mut cmavo = connective.cmavo;
    cmavo.extend(words);
    ConnectiveSyntax {
        kind: connective.kind,
        se: connective.se,
        nahe: connective.nahe,
        na: connective.na,
        cmavo,
        nai: connective.nai,
        free_modifiers: connective.free_modifiers,
    }
}

#[requires(true)]
#[ensures(ret.cmavo.len() >= old(words.len()))]
fn prepend_connective_words(
    words: Vec<WithIndicators<WordLike>>,
    connective: ConnectiveSyntax,
) -> ConnectiveSyntax {
    let mut cmavo = words;
    cmavo.extend(connective.cmavo);
    ConnectiveSyntax {
        kind: connective.kind,
        se: connective.se,
        nahe: connective.nahe,
        na: connective.na,
        cmavo,
        nai: connective.nai,
        free_modifiers: connective.free_modifiers,
    }
}

#[requires(true)]
#[ensures(true)]
fn jek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("JA", &["je'i", "ja", "je", "jo", "ju"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Relation,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn joik_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of(
                "JOI",
                &[
                    "ce", "ce'e", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a", "pi'u",
                ],
            ))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::NonLogical,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .or_not()
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .map(|((se, cmavo), nai)| ConnectiveSyntax {
                kind: ConnectiveKind::Interval,
                se,
                nahe: None,
                na: None,
                cmavo: vec![cmavo],
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("GAhO", &["ga'o", "ke'i"])
            .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
            .then(cmavo_of("BIhI", &["mi'i", "bi'o", "bi'i"]))
            .then(cmavo("nai").or_not())
            .then(cmavo_of("GAhO", &["ga'o", "ke'i"]))
            .map(
                |((((left_interval, se), cmavo), nai), right_interval)| ConnectiveSyntax {
                    kind: ConnectiveKind::Interval,
                    se,
                    nahe: None,
                    na: None,
                    cmavo: vec![left_interval, cmavo, right_interval],
                    nai,
                    free_modifiers: Vec::new(),
                },
            ),
    ))
    .boxed()
}

#[requires(!connective.cmavo.is_empty())]
#[ensures(ret.len() >= old(connective.cmavo.len()))]
fn connective_tense_modal_leaves(connective: ConnectiveSyntax) -> Vec<WithIndicators<WordLike>> {
    let mut leaves = Vec::new();
    leaves.extend(connective.se);
    leaves.extend(connective.nahe);
    leaves.extend(connective.na);
    leaves.extend(connective.cmavo);
    leaves.extend(connective.nai);
    leaves
}

#[requires(true)]
#[ensures(true)]
fn standard_statement_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn statement_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        joik_connective(),
        jek_connective(),
        ek_connective(),
        vuhu_nonlogical_connective(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_afterthought_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        joik_connective(),
        jek_connective(),
        ek_connective(),
        vuhu_nonlogical_connective(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn guhek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("GUhA", &["gu'a", "gu'e", "gu'i", "gu'o", "gu'u"]))
        .then(cmavo("nai").or_not())
        .map(|(((nahe, se), guha), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se,
            nahe,
            na: None,
            cmavo: vec![guha],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn modal_forethought_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let ga = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .or_not()
        .then(cmavo_of("GA", &["ga", "ge", "ge'i", "go", "gu"]))
        .then(cmavo("nai").or_not())
        .map(|((se, ga), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se,
            nahe: None,
            na: None,
            cmavo: vec![ga],
            nai,
            free_modifiers: Vec::new(),
        });
    let modal_gi = tense_modal().then(cmavo("gi")).map(|(tense_modal, gi)| {
        let mut cmavo = tense_modal.words();
        cmavo.push(gi);
        ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se: None,
            nahe: None,
            na: None,
            cmavo,
            nai: None,
            free_modifiers: Vec::new(),
        }
    });
    let joik_gi = joik_connective()
        .then(cmavo("gi"))
        .then(cmavo("bo").or_not())
        .map(|((connective, gi), bo)| {
            let extra = [Some(gi), bo].into_iter().flatten().collect::<Vec<_>>();
            append_connective_words(connective, extra)
        });
    choice((ga, joik_gi, modal_gi)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn modal_forethought_connective_with_free_modifiers<'tokens, F>(
    free_modifier: F,
) -> BoxedParser<'tokens, ConnectiveSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    modal_forethought_connective()
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            append_connective_free_modifiers(connective, free_modifiers)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn gik_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    cmavo("gi")
        .then(cmavo("nai").or_not())
        .map(|(gi, nai)| ConnectiveSyntax {
            kind: ConnectiveKind::Forethought,
            se: None,
            nahe: None,
            na: None,
            cmavo: vec![gi],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn gik_connective_with_free_modifiers<'tokens, F>(
    free_modifier: F,
) -> BoxedParser<'tokens, ConnectiveSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    gik_connective()
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            append_connective_free_modifiers(connective, free_modifiers)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn gihek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("GIhA", &["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"]))
        .then(cmavo("nai").or_not())
        .map(|(((na, se), cmavo), nai)| ConnectiveSyntax {
            kind: ConnectiveKind::PredicateTail,
            se,
            nahe: None,
            na,
            cmavo: vec![cmavo],
            nai,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let experimental = relation_afterthought_connective()
        .map(|connective| connective_with_kind(connective, ConnectiveKind::PredicateTail));
    choice((gihek_connective(), experimental)).boxed()
}

#[requires(true)]
#[ensures(ret.kind == old(kind.clone()))]
fn connective_with_kind(connective: ConnectiveSyntax, kind: ConnectiveKind) -> ConnectiveSyntax {
    ConnectiveSyntax { kind, ..connective }
}

#[requires(true)]
#[ensures(true)]
fn math_operator<'tokens>() -> BoxedParser<'tokens, MathOperatorSyntax> {
    math_parser_pair().1
}

#[requires(true)]
#[ensures(true)]
fn wrapped_word(
    word: WithIndicators<WordLike>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> WithFreeModifiers<WithIndicators<WordLike>> {
    WithFreeModifiers::new(word, free_modifiers)
}

#[requires(true)]
#[ensures(true)]
fn goha_relation_unit(
    goha: WithIndicators<WordLike>,
    raho: Option<WithIndicators<WordLike>>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> RelationUnitSyntax {
    if let Some(raho) = raho {
        RelationUnitSyntax::Goha {
            goha: wrapped_word(goha, Vec::new()),
            raho: Some(wrapped_word(raho, free_modifiers)),
        }
    } else {
        RelationUnitSyntax::Goha {
            goha: wrapped_word(goha, free_modifiers),
            raho: None,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with<'tokens, E, O>(
    expression: E,
    operator: O,
) -> BoxedParser<'tokens, MathOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let vuhu = cmavo_of("VUhU", VUHU_WORDS).map(|vuhu| MathOperatorSyntax::Vuhu {
        vuhu,
        free_modifiers: Vec::new(),
    });
    let maho = cmavo("ma'o")
        .then(expression)
        .then(cmavo("te'u").or_not())
        .map(|((maho, math_expression), tehu)| MathOperatorSyntax::Maho {
            maho,
            free_modifiers: Vec::new(),
            math_expression: Box::new(math_expression),
            tehu,
            tehu_free_modifiers: Vec::new(),
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator)
        .map(
            |(((guhek, left_operator), gik), right_operator)| MathOperatorSyntax::Connected {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            },
        );
    let atom = choice((forethought, maho, vuhu)).boxed();
    atom.clone()
        .then(
            statement_connective()
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    MathOperatorSyntax::Connected {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    }
                })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with_context<'tokens, E, O, R>(
    expression: E,
    operator: O,
    relation: R,
) -> BoxedParser<'tokens, MathOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MathExpressionSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MathOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let vuhu = cmavo_of("VUhU", VUHU_WORDS).map(|vuhu| MathOperatorSyntax::Vuhu {
        vuhu,
        free_modifiers: Vec::new(),
    });
    let maho = cmavo("ma'o")
        .then(expression)
        .then(cmavo("te'u").or_not())
        .map(|((maho, math_expression), tehu)| MathOperatorSyntax::Maho {
            maho,
            free_modifiers: Vec::new(),
            math_expression: Box::new(math_expression),
            tehu,
            tehu_free_modifiers: Vec::new(),
        });
    let se = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(operator.clone())
        .map(|(se, inner_operator)| MathOperatorSyntax::Se {
            se,
            free_modifiers: Vec::new(),
            inner_operator: Box::new(inner_operator),
        });
    let nahe = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(operator.clone())
        .map(|(nahe, inner_operator)| MathOperatorSyntax::Nahe {
            nahe,
            free_modifiers: Vec::new(),
            inner_operator: Box::new(inner_operator),
        });
    let nahu = cmavo("na'u")
        .then(relation)
        .then(cmavo("te'u").or_not())
        .map(|((nahu, relation), tehu)| MathOperatorSyntax::Nahu {
            nahu,
            free_modifiers: Vec::new(),
            relation,
            tehu,
            tehu_free_modifiers: Vec::new(),
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator)
        .map(
            |(((guhek, left_operator), gik), right_operator)| MathOperatorSyntax::Connected {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            },
        );
    let atom = choice((se, nahe, forethought, nahu, maho, vuhu)).boxed();
    atom.clone()
        .then(
            statement_connective()
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    MathOperatorSyntax::Connected {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    }
                })
        })
        .boxed()
}

#[requires(!marker_text.is_empty())]
#[ensures(true)]
fn single_word_quoted_relation_unit<'tokens, F>(
    marker_text: &'static str,
    source: Option<&'tokens str>,
    free_modifier: F,
    build: fn(WithIndicators<WordLike>, String, Vec<FreeModifierSyntax>) -> RelationUnitSyntax,
) -> BoxedParser<'tokens, RelationUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    any()
        .try_map(move |word: WithIndicators<WordLike>, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            let data!(WordLike::SingleWordQuote {
                marker,
                quoted_text,
            }) = word_like.as_data()
            else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            if word_record_text_matches(marker, marker_text) {
                Ok((word.clone(), source_text(source, quoted_text)))
            } else {
                Err(Rich::custom(span, format!("expected {marker_text} quote")))
            }
        })
        .map_with(
            move |(word, quoted_text),
                  extra: &mut MapExtra<
                'tokens,
                '_,
                ParserInput<'tokens>,
                ParseExtra<'tokens>,
            >| {
                if let Some(construct) = quoted_relation_unit_warning(marker_text) {
                    extra.state().warn(construct, &word);
                }
                (word, quoted_text)
            },
        )
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(move |((word, quoted_text), free_modifiers)| build(word, quoted_text, free_modifiers))
        .boxed()
}

#[requires(!marker_text.is_empty())]
#[ensures(true)]
fn delimited_quoted_relation_unit<'tokens, F>(
    marker_text: &'static str,
    source: Option<&'tokens str>,
    free_modifier: F,
    build: fn(
        WithIndicators<WordLike>,
        WithIndicators<WordLike>,
        WithIndicators<WordLike>,
        String,
        Vec<FreeModifierSyntax>,
    ) -> RelationUnitSyntax,
) -> BoxedParser<'tokens, RelationUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    any()
        .try_map(move |word: WithIndicators<WordLike>, span| {
            let Some(word_like) = quote_word_like(&word) else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            let data!(WordLike::ZoiQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) = word_like.as_data()
            else {
                return Err(Rich::custom(span, format!("expected {marker_text} quote")));
            };
            if word_record_text_matches(zoi, marker_text) {
                Ok((
                    word.clone(),
                    base_word_from_record((**opening_delimiter).clone()),
                    base_word_from_record((**closing_delimiter).clone()),
                    source_text(source, quoted_text),
                ))
            } else {
                Err(Rich::custom(span, format!("expected {marker_text} quote")))
            }
        })
        .map_with(
            move |(word, opening_delimiter, closing_delimiter, quoted_text),
                  extra: &mut MapExtra<
                'tokens,
                '_,
                ParserInput<'tokens>,
                ParseExtra<'tokens>,
            >| {
                if let Some(construct) = quoted_relation_unit_warning(marker_text) {
                    extra.state().warn(construct, &word);
                }
                (word, opening_delimiter, closing_delimiter, quoted_text)
            },
        )
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(
            move |((word, opening_delimiter, closing_delimiter, quoted_text), free_modifiers)| {
                build(
                    word,
                    opening_delimiter,
                    closing_delimiter,
                    quoted_text,
                    free_modifiers,
                )
            },
        )
        .boxed()
}

#[requires(!marker_text.is_empty())]
#[ensures(true)]
fn quoted_relation_unit_warning(marker_text: &str) -> Option<ExperimentalConstruct> {
    match marker_text {
        "me'oi" => Some(ExperimentalConstruct::ExperimentalMehOiRelationUnit),
        "go'oi" => Some(ExperimentalConstruct::ExperimentalGohoiRelationUnit),
        "mu'oi" => Some(ExperimentalConstruct::ExperimentalZantufaMuhoiRelationUnit),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_parser_with<'tokens, P, R, S, T, F>(
    argument: P,
    relation: R,
    subsentence: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, RelationSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let tense_modal_with_free_modifiers = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(tense_modal, free_modifiers)| {
            attach_tense_modal_free_modifiers(tense_modal, free_modifiers)
        })
        .boxed();
    let me_argument = argument
        .clone()
        .or(letter_string().map(|letter| ArgumentSyntax::Letter {
            letter: WithFreeModifiers::new(letter, Vec::new()),
            boi: None,
        }));
    let me_unit = cmavo("me")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(me_argument)
        .then(
            cmavo("me'u")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .then(
            cmavo_of("MOI", MOI_WORDS)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((me, me_free_modifiers), argument), mehu), moi_marker)| RelationUnitSyntax::Me {
                me: wrapped_word(me, me_free_modifiers),
                argument,
                mehu: mehu.map(|(mehu, free_modifiers)| wrapped_word(mehu, free_modifiers)),
                moi_marker: moi_marker
                    .map(|(moi_marker, free_modifiers)| wrapped_word(moi_marker, free_modifiers)),
            },
        );
    let mehoi_unit = single_word_quoted_relation_unit(
        "me'oi",
        source,
        free_modifier.clone(),
        |mehoi, quoted_text, free_modifiers| RelationUnitSyntax::Mehoi {
            mehoi: wrapped_word(mehoi, free_modifiers),
            quoted_text,
        },
    );
    let gohoi_unit = single_word_quoted_relation_unit(
        "go'oi",
        source,
        free_modifier.clone(),
        |gohoi, quoted_text, free_modifiers| RelationUnitSyntax::Gohoi {
            gohoi: wrapped_word(gohoi, free_modifiers),
            quoted_text,
        },
    );
    let muhoi_unit = delimited_quoted_relation_unit(
        "mu'oi",
        source,
        free_modifier.clone(),
        |muhoi, opening_delimiter, closing_delimiter, quoted_text, free_modifiers| {
            RelationUnitSyntax::Muhoi {
                muhoi,
                opening_delimiter,
                closing_delimiter: wrapped_word(closing_delimiter, free_modifiers),
                quoted_text,
            }
        },
    );
    let luhei_unit = cmavo("lu'ei")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(
            cmavo("li'au")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((luhei, luhei_free_modifiers), text), liau)| RelationUnitSyntax::Luhei {
                luhei: wrapped_word(luhei, luhei_free_modifiers),
                text,
                liau: liau.map(|(liau, free_modifiers)| wrapped_word(liau, free_modifiers)),
            },
        )
        .boxed();

    let brivla_word_unit = brivla_relation_word()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(word, free_modifiers)| RelationUnitSyntax::Word {
            word: wrapped_word(word, free_modifiers),
        });
    let goha_word_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then_ignore(
            choice((
                cmavo("ra'o").ignored(),
                cmavo("be").ignored(),
                pa_word().ignored(),
                free_modifier.clone().ignored(),
            ))
            .rewind()
            .not(),
        )
        .map(|word| RelationUnitSyntax::Word {
            word: wrapped_word(word, Vec::new()),
        });
    let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
    let goha_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then(cmavo("ra'o").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, raho, free_modifiers));
    let goha_raho_unit = cmavo_of("GOhA", GOHA_WORDS)
        .then(cmavo("ra'o"))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, Some(raho), free_modifiers));
    let moi_unit = number_or_letter_words()
        .then(cmavo_of("MOI", MOI_WORDS))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, moi), free_modifiers)| RelationUnitSyntax::Moi {
            number,
            moi: wrapped_word(moi, free_modifiers),
        });
    let contextual_math_operator =
        math_parser_pair_with_context(argument.clone(), relation.clone(), free_modifier.clone()).1;
    let nuha_unit = cmavo("nu'a")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(contextual_math_operator)
        .map(
            |((nuha, free_modifiers), math_operator)| RelationUnitSyntax::Nuha {
                nuha: wrapped_word(nuha, free_modifiers),
                math_operator,
            },
        );
    let xohi_unit = cmavo("xo'i")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_with_free_modifiers.clone())
        .map(|((xohi, free_modifiers), tag)| RelationUnitSyntax::Xohi {
            xohi: wrapped_word(xohi, free_modifiers),
            tag,
        });

    let ke_unit = cmavo("ke")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .then(
            cmavo("ke'e")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((ke, ke_free_modifiers), relation), kehe)| RelationUnitSyntax::Ke {
                ke_tense_modal: None,
                ke: wrapped_word(ke, ke_free_modifiers),
                relation,
                kehe: kehe.map(|(kehe, free_modifiers)| wrapped_word(kehe, free_modifiers)),
            },
        );

    let se_unit = recursive(|se_unit| {
        let nahe_inner_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                se_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                muhoi_unit.clone(),
                luhei_unit.clone(),
                xohi_unit.clone(),
                nuha_unit.clone(),
                moi_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(
                |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                nuha_unit.clone(),
                nahe_inner_unit,
                se_unit,
                word_unit.clone(),
                goha_unit.clone(),
            )))
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            )
    })
    .boxed();

    let wrapped_tense_unit = tense_modal_with_free_modifiers
        .clone()
        .then(relation_units_inner(
            argument.clone(),
            subsentence.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .map(|(tense_modal, inner_relation)| {
            RelationUnitSyntax::Wrapped(RelationSyntax::TenseModal {
                tense_modal,
                inner_relation: Box::new(inner_relation),
            })
        });

    let jai_inner_unit = recursive(|jai_inner_unit| {
        let se_inner_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_inner_unit.clone())
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        let nahe_inner_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_inner_unit.clone())
            .map(
                |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        choice((
            se_inner_unit,
            nahe_inner_unit,
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            ke_unit.clone(),
            moi_unit.clone(),
            nuha_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        ))
    })
    .boxed();

    let jai_unit = cmavo("jai")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(jai_inner_unit)
        .map(
            |(((jai, free_modifiers), tense_modal), inner_unit)| RelationUnitSyntax::Jai {
                jai: wrapped_word(jai, free_modifiers),
                tense_modal,
                inner_unit: Box::new(inner_unit),
            },
        );
    let se_jai_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(jai_unit.clone())
        .map(
            |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                se: wrapped_word(se, free_modifiers),
                inner_unit: Box::new(inner_unit),
            },
        );

    let nahe_unit = recursive(|nahe_unit| {
        cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                wrapped_tense_unit.clone(),
                ke_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                muhoi_unit.clone(),
                luhei_unit.clone(),
                xohi_unit.clone(),
                nuha_unit.clone(),
                moi_unit.clone(),
                se_unit.clone(),
                jai_unit.clone(),
                nahe_unit,
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(
                |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            )
    })
    .boxed();

    let nu_cmavo = || cmavo_of("NU", NU_WORDS);
    let additional_nu = statement_connective()
        .then(nu_cmavo())
        .then(cmavo("nai").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((connective, nu), nai), free_modifiers)| AdditionalNuSyntax {
                connective,
                nu,
                nai,
                free_modifiers,
            },
        );
    let abstraction_subsentence_unit = nu_cmavo()
        .then(cmavo("nai").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(additional_nu.repeated().collect::<Vec<_>>())
        .then(subsentence)
        .then(cmavo("kei").or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(
                (((((nu, nai), free_modifiers), additional_nu), subsentence), kei),
                kei_free_modifiers,
            )| {
                RelationUnitSyntax::Abstraction(AbstractionSyntax {
                    nu,
                    nai,
                    free_modifiers,
                    additional_nu,
                    subsentence: Box::new(subsentence),
                    kei,
                    kei_free_modifiers,
                })
            },
        )
        .boxed();

    let se_abstraction_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(abstraction_subsentence_unit.clone())
        .map(
            |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                se: wrapped_word(se, free_modifiers),
                inner_unit: Box::new(inner_unit),
            },
        );

    let base_unit = choice((
        goha_raho_unit.clone(),
        me_unit.clone(),
        mehoi_unit.clone(),
        gohoi_unit.clone(),
        muhoi_unit.clone(),
        luhei_unit.clone(),
        se_abstraction_unit.clone(),
        abstraction_subsentence_unit.clone(),
        se_jai_unit.clone(),
        jai_unit.clone(),
        nahe_unit.clone(),
        se_unit.clone(),
        ke_unit.clone(),
        xohi_unit.clone(),
        nuha_unit.clone(),
        moi_unit.clone(),
        word_unit.clone(),
        goha_unit.clone(),
    ))
    .boxed();
    let base_unit_for_cei = choice((
        goha_raho_unit.clone(),
        me_unit.clone(),
        mehoi_unit.clone(),
        gohoi_unit.clone(),
        muhoi_unit.clone(),
        luhei_unit.clone(),
        se_abstraction_unit.clone(),
        abstraction_subsentence_unit.clone(),
        se_jai_unit,
        jai_unit.clone(),
        nahe_unit.clone(),
        se_unit.clone(),
        ke_unit.clone(),
        xohi_unit,
        nuha_unit.clone(),
        moi_unit.clone(),
        goha_unit.clone(),
        word_unit.clone(),
    ))
    .boxed();
    let be_link = be_link_parser(argument.clone(), free_modifier.clone());
    let selbri_relative_clause = cmavo("no'oi")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation.clone())
        .then(
            cmavo("ku'oi")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((nohoi, leading_free_modifiers), relation), kuhoi)| SelbriRelativeClauseSyntax {
                nohoi: WithFreeModifiers::new(nohoi, leading_free_modifiers),
                relation,
                kuhoi: kuhoi
                    .map(|(kuhoi, free_modifiers)| WithFreeModifiers::new(kuhoi, free_modifiers)),
            },
        )
        .boxed();

    let linked_unit_from = |base_unit: BoxedParser<'tokens, RelationUnitSyntax>| {
        base_unit
            .then(be_link.clone().or_not())
            .map(|(base, be_link)| {
                be_link.map_or(base.clone(), |link| {
                    let data!(BeLinkSyntax {
                        be,
                        fa,
                        first_argument,
                        bei_links,
                        beho,
                    }) = link.into_data();

                    RelationUnitSyntax::Be {
                        base: Box::new(base),
                        be,
                        fa,
                        first_argument,
                        bei_links,
                        beho,
                    }
                })
            })
            .then(
                selbri_relative_clause
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(linked_unit, selbri_relative_clauses)| {
                if selbri_relative_clauses.is_empty() {
                    linked_unit
                } else {
                    RelationUnitSyntax::SelbriRelativeClause {
                        base: Box::new(linked_unit),
                        selbri_relative_clauses,
                    }
                }
            })
            .boxed()
    };
    let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
        let data!(BeLinkSyntax {
            be,
            fa,
            first_argument,
            bei_links,
            beho,
        }) = link.into_data();

        RelationUnitSyntax::PreposedBe {
            be,
            fa,
            first_argument,
            bei_links,
            beho,
            base: Box::new(base),
        }
    });
    let linked_unit = linked_unit_from(base_unit);
    let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
    let cei_unit = linked_unit_for_cei
        .clone()
        .then(
            cmavo("cei")
                .then(linked_unit_for_cei.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(base, be_link)| RelationUnitSyntax::Cei {
            base: Box::new(base),
            assignments: be_link
                .into_iter()
                .map(|(cei, relation_unit)| CeiAssignmentSyntax {
                    cei: wrapped_word(cei, Vec::new()),
                    relation_unit,
                })
                .collect(),
        })
        .boxed();

    let bo_unit = recursive(|bo_unit| {
        let guha_unit = guhek_connective()
            .then(relation.clone())
            .then(gik_connective_with_free_modifiers(free_modifier.clone()))
            .then(bo_unit.clone())
            .map(|(((guhek, leading_relation), gik), trailing_unit)| {
                RelationUnitSyntax::Wrapped(RelationSyntax::Guha {
                    guhek,
                    leading_predicate: Box::new(relation_to_empty_predicate(leading_relation)),
                    gik,
                    trailing_predicate: Box::new(relation_to_empty_predicate(
                        relation_unit_to_relation(&trailing_unit),
                    )),
                })
            });
        let atom_unit = choice((
            guha_unit,
            preposed_unit.clone(),
            cei_unit.clone(),
            linked_unit.clone(),
        ))
        .boxed();
        let connected_bo_tail = statement_connective()
            .then(tense_modal().or_not())
            .then(cmavo("bo"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(bo_unit.clone())
            .map(
                |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                    (
                        Some(connective),
                        bo_tense_modal,
                        bo,
                        free_modifiers,
                        trailing_unit,
                    )
                },
            );
        let bare_bo_tail = cmavo("bo")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(bo_unit)
            .map(|((bo, free_modifiers), trailing_unit)| {
                (None, None, bo, free_modifiers, trailing_unit)
            });
        atom_unit
            .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
            .map(|(leading_unit, bo_tail)| {
                bo_tail.map_or(
                    leading_unit.clone(),
                    |(bo_connective, bo_tense_modal, bo, free_modifiers, trailing_unit)| {
                        RelationUnitSyntax::Bo {
                            leading_unit: Box::new(leading_unit),
                            bo_connective,
                            bo_tense_modal,
                            bo: wrapped_word(bo, free_modifiers),
                            trailing_unit: Box::new(trailing_unit),
                        }
                    },
                )
            })
    });

    let connected_unit = bo_unit
        .clone()
        .then(
            relation_afterthought_connective()
                .then(bo_unit)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |leading_unit, (connective, trailing_unit)| {
                    RelationUnitSyntax::Connected {
                        leading_unit: Box::new(leading_unit),
                        connective,
                        trailing_unit: Box::new(trailing_unit),
                    }
                })
        });

    let relation_units = connected_unit
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(relation_from_units);

    let base_relation = relation_units;
    let connected_relation = base_relation
        .clone()
        .then(
            relation_afterthought_connective()
                .then(base_relation.clone())
                .or_not(),
        )
        .map(|(leading_relation, connected)| {
            connected.map_or(
                leading_relation.clone(),
                |(connective, trailing_relation)| RelationSyntax::Connected {
                    connective,
                    leading_relation: Box::new(leading_relation),
                    trailing_relation: Box::new(trailing_relation),
                },
            )
        });
    let na_relation = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation)
        .map(
            |((na, free_modifiers), inner_relation)| RelationSyntax::Na {
                na: wrapped_word(na, free_modifiers),
                inner_relation: Box::new(inner_relation),
            },
        );
    let co_relation = recursive(|co_relation| {
        connected_relation
            .clone()
            .then(
                cmavo("co")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(co_relation)
                    .or_not(),
            )
            .map(|(leading_relation, co_tail)| {
                co_tail.map_or(
                    leading_relation.clone(),
                    |((co, free_modifiers), trailing_relation)| RelationSyntax::Co {
                        leading_relation: Box::new(leading_relation),
                        co: wrapped_word(co, free_modifiers),
                        trailing_relation: Box::new(trailing_relation),
                    },
                )
            })
    });

    let untagged_relation = choice((na_relation, co_relation)).boxed();
    let tagged_relation = tense_modal_with_free_modifiers
        .then(untagged_relation.clone())
        .map(|(tense_modal, inner_relation)| RelationSyntax::TenseModal {
            tense_modal,
            inner_relation: Box::new(inner_relation),
        });

    choice((tagged_relation, untagged_relation)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_units_inner<'tokens, P, S, T, F>(
    argument: P,
    subsentence: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, RelationSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubsentenceSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive(|inner_relation| {
        let me_argument =
            argument
                .clone()
                .or(letter_string().map(|letter| ArgumentSyntax::Letter {
                    letter: WithFreeModifiers::new(letter, Vec::new()),
                    boi: None,
                }));
        let me_unit = cmavo("me")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(me_argument)
            .then(
                cmavo("me'u")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .then(
                cmavo_of("MOI", MOI_WORDS)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |((((me, me_free_modifiers), argument), mehu), moi_marker)| {
                    RelationUnitSyntax::Me {
                        me: wrapped_word(me, me_free_modifiers),
                        argument,
                        mehu: mehu.map(|(mehu, free_modifiers)| wrapped_word(mehu, free_modifiers)),
                        moi_marker: moi_marker.map(|(moi_marker, free_modifiers)| {
                            wrapped_word(moi_marker, free_modifiers)
                        }),
                    }
                },
            );
        let mehoi_unit = single_word_quoted_relation_unit(
            "me'oi",
            source,
            free_modifier.clone(),
            |mehoi, quoted_text, free_modifiers| RelationUnitSyntax::Mehoi {
                mehoi: wrapped_word(mehoi, free_modifiers),
                quoted_text,
            },
        );
        let gohoi_unit = single_word_quoted_relation_unit(
            "go'oi",
            source,
            free_modifier.clone(),
            |gohoi, quoted_text, free_modifiers| RelationUnitSyntax::Gohoi {
                gohoi: wrapped_word(gohoi, free_modifiers),
                quoted_text,
            },
        );
        let muhoi_unit = delimited_quoted_relation_unit(
            "mu'oi",
            source,
            free_modifier.clone(),
            |muhoi, opening_delimiter, closing_delimiter, quoted_text, free_modifiers| {
                RelationUnitSyntax::Muhoi {
                    muhoi,
                    opening_delimiter,
                    closing_delimiter: wrapped_word(closing_delimiter, free_modifiers),
                    quoted_text,
                }
            },
        );
        let luhei_unit = cmavo("lu'ei")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(text.clone())
            .then(
                cmavo("li'au")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |(((luhei, luhei_free_modifiers), text), liau)| RelationUnitSyntax::Luhei {
                    luhei: wrapped_word(luhei, luhei_free_modifiers),
                    text,
                    liau: liau.map(|(liau, free_modifiers)| wrapped_word(liau, free_modifiers)),
                },
            )
            .boxed();
        let brivla_word_unit = brivla_relation_word()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(word, free_modifiers)| RelationUnitSyntax::Word {
                word: wrapped_word(word, free_modifiers),
            });
        let goha_word_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then_ignore(
                choice((
                    cmavo("ra'o").ignored(),
                    cmavo("be").ignored(),
                    pa_word().ignored(),
                    free_modifier.clone().ignored(),
                ))
                .rewind()
                .not(),
            )
            .map(|word| RelationUnitSyntax::Word {
                word: wrapped_word(word, Vec::new()),
            });
        let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
        let goha_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then(cmavo("ra'o").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, raho, free_modifiers));
        let goha_raho_unit = cmavo_of("GOhA", GOHA_WORDS)
            .then(cmavo("ra'o"))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| {
                goha_relation_unit(goha, Some(raho), free_modifiers)
            });
        let moi_unit = number_or_letter_words()
            .then(cmavo_of("MOI", MOI_WORDS))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((number, moi), free_modifiers)| RelationUnitSyntax::Moi {
                number,
                moi: wrapped_word(moi, free_modifiers),
            });
        let nuha_unit = cmavo("nu'a")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(math_operator())
            .map(
                |((nuha, free_modifiers), math_operator)| RelationUnitSyntax::Nuha {
                    nuha: wrapped_word(nuha, free_modifiers),
                    math_operator,
                },
            );
        let xohi_unit = cmavo("xo'i")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(tense_modal())
            .map(|((xohi, free_modifiers), tag)| RelationUnitSyntax::Xohi {
                xohi: wrapped_word(xohi, free_modifiers),
                tag,
            });
        let nu_cmavo = || cmavo_of("NU", NU_WORDS);
        let additional_nu = statement_connective()
            .then(nu_cmavo())
            .then(cmavo("nai").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((connective, nu), nai), free_modifiers)| AdditionalNuSyntax {
                    connective,
                    nu,
                    nai,
                    free_modifiers,
                },
            );
        let abstraction_subsentence_unit = nu_cmavo()
            .then(cmavo("nai").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(additional_nu.repeated().collect::<Vec<_>>())
            .then(subsentence.clone())
            .then(cmavo("kei").or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(
                    (((((nu, nai), free_modifiers), additional_nu), subsentence), kei),
                    kei_free_modifiers,
                )| {
                    RelationUnitSyntax::Abstraction(AbstractionSyntax {
                        nu,
                        nai,
                        free_modifiers,
                        additional_nu,
                        subsentence: Box::new(subsentence),
                        kei,
                        kei_free_modifiers,
                    })
                },
            )
            .boxed();
        let se_abstraction_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(abstraction_subsentence_unit.clone())
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        let ke_unit = cmavo("ke")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_relation.clone())
            .then(
                cmavo("ke'e")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |(((ke, ke_free_modifiers), relation), kehe)| RelationUnitSyntax::Ke {
                    ke_tense_modal: None,
                    ke: wrapped_word(ke, ke_free_modifiers),
                    relation,
                    kehe: kehe.map(|(kehe, free_modifiers)| wrapped_word(kehe, free_modifiers)),
                },
            );
        let se_unit = recursive(|se_unit| {
            cmavo_of("SE", &["se", "te", "ve", "xe"])
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(choice((
                    ke_unit.clone(),
                    moi_unit.clone(),
                    nuha_unit.clone(),
                    se_unit,
                    word_unit.clone(),
                    goha_unit.clone(),
                )))
                .map(
                    |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                        se: wrapped_word(se, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    },
                )
        })
        .boxed();
        let jai_inner_unit = recursive(|jai_inner_unit| {
            let se_inner_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(jai_inner_unit.clone())
                .map(
                    |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                        se: wrapped_word(se, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    },
                );
            let nahe_inner_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(jai_inner_unit.clone())
                .map(
                    |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                        nahe: wrapped_word(nahe, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    },
                );
            choice((
                se_inner_unit,
                nahe_inner_unit,
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                muhoi_unit.clone(),
                luhei_unit.clone(),
                ke_unit.clone(),
                moi_unit.clone(),
                nuha_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            ))
        })
        .boxed();
        let jai_unit = cmavo("jai")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(tense_modal().or_not())
            .then(jai_inner_unit)
            .map(
                |(((jai, free_modifiers), tense_modal), inner_unit)| RelationUnitSyntax::Jai {
                    jai: wrapped_word(jai, free_modifiers),
                    tense_modal,
                    inner_unit: Box::new(inner_unit),
                },
            );
        let se_jai_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_unit.clone())
            .map(
                |((se, free_modifiers), inner_unit)| RelationUnitSyntax::Se {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        let nahe_unit = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                jai_unit.clone(),
                se_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(
                |((nahe, free_modifiers), inner_unit)| RelationUnitSyntax::Nahe {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                },
            );
        let be_link = be_link_parser(argument.clone(), free_modifier.clone());
        let selbri_relative_clause = cmavo("no'oi")
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_relation.clone())
            .then(
                cmavo("ku'oi")
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((nohoi, leading_free_modifiers), relation), kuhoi)| {
                SelbriRelativeClauseSyntax {
                    nohoi: WithFreeModifiers::new(nohoi, leading_free_modifiers),
                    relation,
                    kuhoi: kuhoi.map(|(kuhoi, free_modifiers)| {
                        WithFreeModifiers::new(kuhoi, free_modifiers)
                    }),
                }
            })
            .boxed();

        let base_unit = choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subsentence_unit.clone(),
            se_jai_unit.clone(),
            jai_unit.clone(),
            nahe_unit.clone(),
            se_unit.clone(),
            ke_unit.clone(),
            xohi_unit.clone(),
            nuha_unit.clone(),
            moi_unit.clone(),
            word_unit.clone(),
            goha_unit.clone(),
        ))
        .boxed();
        let base_unit_for_cei = choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit,
            abstraction_subsentence_unit,
            se_jai_unit,
            jai_unit,
            nahe_unit.clone(),
            se_unit.clone(),
            ke_unit.clone(),
            xohi_unit,
            nuha_unit.clone(),
            moi_unit.clone(),
            goha_unit.clone(),
            word_unit.clone(),
        ))
        .boxed();
        let linked_unit_from = |base_unit: BoxedParser<'tokens, RelationUnitSyntax>| {
            base_unit
                .then(be_link.clone().or_not())
                .map(|(base, be_link)| {
                    be_link.map_or(base.clone(), |link| {
                        let data!(BeLinkSyntax {
                            be,
                            fa,
                            first_argument,
                            bei_links,
                            beho,
                        }) = link.into_data();

                        RelationUnitSyntax::Be {
                            base: Box::new(base),
                            be,
                            fa,
                            first_argument,
                            bei_links,
                            beho,
                        }
                    })
                })
                .then(
                    selbri_relative_clause
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .map(|(linked_unit, selbri_relative_clauses)| {
                    if selbri_relative_clauses.is_empty() {
                        linked_unit
                    } else {
                        RelationUnitSyntax::SelbriRelativeClause {
                            base: Box::new(linked_unit),
                            selbri_relative_clauses,
                        }
                    }
                })
                .boxed()
        };
        let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
            let data!(BeLinkSyntax {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
            }) = link.into_data();

            RelationUnitSyntax::PreposedBe {
                be,
                fa,
                first_argument,
                bei_links,
                beho,
                base: Box::new(base),
            }
        });
        let linked_unit = linked_unit_from(base_unit);
        let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
        let cei_unit = linked_unit_for_cei
            .clone()
            .then(
                cmavo("cei")
                    .then(linked_unit_for_cei.clone())
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(base, be_link)| RelationUnitSyntax::Cei {
                base: Box::new(base),
                assignments: be_link
                    .into_iter()
                    .map(|(cei, relation_unit)| CeiAssignmentSyntax {
                        cei: wrapped_word(cei, Vec::new()),
                        relation_unit,
                    })
                    .collect(),
            })
            .boxed();
        let bo_unit = recursive(|bo_unit| {
            let guha_unit = guhek_connective()
                .then(inner_relation.clone())
                .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                .then(bo_unit.clone())
                .map(|(((guhek, leading_relation), gik), trailing_unit)| {
                    RelationUnitSyntax::Wrapped(RelationSyntax::Guha {
                        guhek,
                        leading_predicate: Box::new(relation_to_empty_predicate(leading_relation)),
                        gik,
                        trailing_predicate: Box::new(relation_to_empty_predicate(
                            relation_unit_to_relation(&trailing_unit),
                        )),
                    })
                });
            let atom_unit = choice((
                guha_unit,
                preposed_unit.clone(),
                cei_unit.clone(),
                linked_unit.clone(),
            ))
            .boxed();
            let connected_bo_tail = statement_connective()
                .then(tense_modal().or_not())
                .then(cmavo("bo"))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit.clone())
                .map(
                    |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                        (
                            Some(connective),
                            bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_unit,
                        )
                    },
                );
            let bare_bo_tail = cmavo("bo")
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit)
                .map(|((bo, free_modifiers), trailing_unit)| {
                    (None, None, bo, free_modifiers, trailing_unit)
                });
            atom_unit
                .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
                .map(|(leading_unit, bo_tail)| {
                    bo_tail.map_or(
                        leading_unit.clone(),
                        |(bo_connective, bo_tense_modal, bo, free_modifiers, trailing_unit)| {
                            RelationUnitSyntax::Bo {
                                leading_unit: Box::new(leading_unit),
                                bo_connective,
                                bo_tense_modal,
                                bo: wrapped_word(bo, free_modifiers),
                                trailing_unit: Box::new(trailing_unit),
                            }
                        },
                    )
                })
        });
        bo_unit
            .clone()
            .then(
                statement_connective()
                    .then(bo_unit)
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first, continuations)| {
                continuations.into_iter().fold(
                    first,
                    |leading_unit, (connective, trailing_unit)| RelationUnitSyntax::Connected {
                        leading_unit: Box::new(leading_unit),
                        connective,
                        trailing_unit: Box::new(trailing_unit),
                    },
                )
            })
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(relation_from_units)
    })
    .boxed()
}

#[requires(!units.is_empty(), "relation unit sequences must be non-empty")]
#[ensures(true)]
fn relation_from_units(units: Vec<RelationUnitSyntax>) -> RelationSyntax {
    match units.as_slice() {
        [RelationUnitSyntax::Word { word }] if word.free_modifiers.is_empty() => {
            RelationSyntax::Base(word.value.clone())
        }
        [RelationUnitSyntax::Goha { goha, raho: None }] if goha.free_modifiers.is_empty() => {
            RelationSyntax::Base(goha.value.clone())
        }
        [RelationUnitSyntax::Word { .. } | RelationUnitSyntax::Goha { .. }] => {
            RelationSyntax::Compound(units)
        }
        [RelationUnitSyntax::Se { se, inner_unit }] => RelationSyntax::Se {
            se: se.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit.as_ref())),
        },
        [
            RelationUnitSyntax::Ke {
                ke_tense_modal,
                ke,
                relation,
                kehe,
            },
        ] => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
        },
        [RelationUnitSyntax::Abstraction(abstraction)] => {
            RelationSyntax::Abstraction(abstraction.clone())
        }
        [
            RelationUnitSyntax::Bo {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_unit,
            },
        ] => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        [
            RelationUnitSyntax::Connected {
                leading_unit,
                connective,
                trailing_unit,
            },
        ] => RelationSyntax::Connected {
            connective: connective.clone(),
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        [RelationUnitSyntax::Wrapped(relation)] => relation.clone(),
        _ => RelationSyntax::Compound(units),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_to_relation(unit: &RelationUnitSyntax) -> RelationSyntax {
    match unit {
        RelationUnitSyntax::Word { word } if word.free_modifiers.is_empty() => {
            RelationSyntax::Base(word.value.clone())
        }
        RelationUnitSyntax::Goha { goha, raho: None } if goha.free_modifiers.is_empty() => {
            RelationSyntax::Base(goha.value.clone())
        }
        RelationUnitSyntax::Se { se, inner_unit } => RelationSyntax::Se {
            se: se.clone(),
            inner_relation: Box::new(relation_unit_to_relation(inner_unit)),
        },
        RelationUnitSyntax::Ke {
            ke_tense_modal,
            ke,
            relation,
            kehe,
        } => RelationSyntax::Ke {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            relation: Box::new(relation.clone()),
            kehe: kehe.clone(),
        },
        RelationUnitSyntax::Abstraction(abstraction) => {
            RelationSyntax::Abstraction(abstraction.clone())
        }
        RelationUnitSyntax::Bo {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        } => RelationSyntax::Bo {
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        RelationUnitSyntax::Connected {
            leading_unit,
            connective,
            trailing_unit,
        } => RelationSyntax::Connected {
            connective: connective.clone(),
            leading_relation: Box::new(relation_unit_to_relation(leading_unit)),
            trailing_relation: Box::new(relation_unit_to_relation(trailing_unit)),
        },
        RelationUnitSyntax::Wrapped(relation) => relation.clone(),
        unit => RelationSyntax::Compound(vec![unit.clone()]),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_to_empty_predicate(relation: RelationSyntax) -> PredicateSyntax {
    PredicateSyntax {
        leading_terms: Vec::new(),
        cu: None,
        predicate_tail: PredicateTailSyntax {
            first: PredicateTail1Syntax {
                first: PredicateTail2Syntax {
                    first: PredicateTail3Syntax::Relation {
                        relation,
                        terms: Vec::new(),
                        vau: None,
                        free_modifiers: Vec::new(),
                    },
                    bo_continuation: None,
                },
                continuations: Vec::new(),
            },
            ke_continuation: None,
        },
        free_modifiers: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn fiho_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let relation = recursive(|relation| {
        let word_unit = relation_word().map(|word| RelationUnitSyntax::Word {
            word: wrapped_word(word, Vec::new()),
        });
        let se_unit = cmavo_of("SE", &["se", "te", "ve", "xe"])
            .then(word_unit.clone())
            .map(|(se, inner_unit)| RelationUnitSyntax::Se {
                se: wrapped_word(se, Vec::new()),
                inner_unit: Box::new(inner_unit),
            });
        let ke_unit = cmavo("ke")
            .then(relation.clone())
            .then(cmavo("ke'e").or_not())
            .map(|((ke, relation), kehe)| RelationUnitSyntax::Ke {
                ke_tense_modal: None,
                ke: wrapped_word(ke, Vec::new()),
                relation,
                kehe: kehe.map(|kehe| wrapped_word(kehe, Vec::new())),
            });
        let simple_unit = choice((ke_unit, se_unit, word_unit)).boxed();
        let bo_unit = recursive(|bo_unit| {
            simple_unit
                .clone()
                .then(cmavo("bo").then(bo_unit).or_not())
                .map(|(leading_unit, bo_tail)| {
                    bo_tail.map_or(leading_unit.clone(), |(bo, trailing_unit)| {
                        RelationUnitSyntax::Bo {
                            leading_unit: Box::new(leading_unit),
                            bo_connective: None,
                            bo_tense_modal: None,
                            bo: wrapped_word(bo, Vec::new()),
                            trailing_unit: Box::new(trailing_unit),
                        }
                    })
                })
        })
        .boxed();
        bo_unit
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(relation_from_units)
    });

    cmavo("fi'o")
        .then(relation)
        .then(cmavo("fe'u").or_not())
        .map(|((fiho, relation), fehu)| TenseModalSyntax::Fiho {
            fiho,
            relation: Box::new(relation),
            fehu,
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn flat_tag_chunk_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let prefixes = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .map(|(nahe, se)| {
            let mut leaves = vec![nahe];
            leaves.extend(se);
            leaves
        })
        .or(cmavo_of("SE", &["se", "te", "ve", "xe"]).map(|se| vec![se]));
    let atom = choice((
        cmavo_of("FA", FA_WORDS).map(|fa| vec![fa]),
        simple_tense_modal().map(|tense_modal| tense_modal.leaf_words()),
        composite_tense_modal().map(|tense_modal| tense_modal.leaf_words()),
    ));

    prefixes
        .then(atom)
        .map(|(mut prefix_leaves, atom_leaves)| {
            prefix_leaves.extend(atom_leaves);
            tense_modal_from_leaves(prefix_leaves, Vec::new())
        })
        .or(cmavo_of("FA", FA_WORDS).map(|fa| tense_modal_from_leaves(vec![fa], Vec::new())))
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let pu = cmavo_of("PU", &["pu", "ca", "ba"])
        .then(cmavo("nai").or_not())
        .then(cmavo_of("ZI", &["zi", "za", "zu"]).or_not())
        .map(|((pu, nai), distance)| {
            let mut leaves = vec![pu.clone()];
            leaves.extend(nai.clone());
            leaves.extend(distance.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: Some(TimeTenseSyntax {
                    direction: vec![pu],
                    distance,
                    interval: None,
                    nai,
                }),
                space: None,
                simple: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let zi = cmavo_of("ZI", &["zi", "za", "zu"]).map(|zi| TenseModalSyntax::Composite {
        leaves: vec![zi.clone()],
        time: Some(TimeTenseSyntax {
            direction: Vec::new(),
            distance: Some(zi),
            interval: None,
            nai: None,
        }),
        space: None,
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: None,
        fiho: Vec::new(),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let faha = cmavo_of(
        "FAhA",
        &[
            "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a", "ru'u",
            "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a", "zo'i", "ze'o",
        ],
    )
    .then(cmavo("nai").or_not())
    .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
    .map(|((faha, nai), distance)| {
        let mut leaves = vec![faha.clone()];
        leaves.extend(nai);
        leaves.extend(distance.clone());
        TenseModalSyntax::Composite {
            leaves,
            time: None,
            space: Some(SpaceTenseSyntax {
                direction: vec![faha],
                distance: distance.into_iter().collect(),
                interval: Vec::new(),
                dimensions: Vec::new(),
                mohi: None,
                fehe: None,
            }),
            simple: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            fiho: Vec::new(),
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        }
    });
    let va = cmavo_of("VA", &["vi", "va", "vu"]).map(|va| TenseModalSyntax::Composite {
        leaves: vec![va.clone()],
        time: None,
        space: Some(SpaceTenseSyntax {
            direction: Vec::new(),
            distance: vec![va],
            interval: Vec::new(),
            dimensions: Vec::new(),
            mohi: None,
            fehe: None,
        }),
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: None,
        fiho: Vec::new(),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let numbered_interval_start = number_words()
        .then(cmavo_of("ROI", ROI_WORDS))
        .rewind()
        .ignored();
    let numbered_interval = numbered_interval_start
        .ignore_then(number_words())
        .then(cmavo_of("ROI", ROI_WORDS))
        .then(cmavo("nai").or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let mut leaves = number.clone();
            leaves.push(roi_or_tahe.clone());
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: Some(IntervalTenseSyntax {
                    number,
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let tahe_interval = cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
        .then(cmavo("nai").or_not())
        .map(|(roi_or_tahe, nai)| {
            let mut leaves = vec![roi_or_tahe.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: Some(IntervalTenseSyntax {
                    number: Vec::new(),
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let caha = cmavo_of("CAhA", CAHA_WORDS).map(|caha| TenseModalSyntax::Composite {
        leaves: vec![caha.clone()],
        time: None,
        space: None,
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: Some(caha),
        ki: None,
        cuhe: None,
        fiho: Vec::new(),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let zaho = cmavo_of("ZAhO", ZAHO_WORDS)
        .then(cmavo("nai").or_not())
        .map(|(zaho, nai)| {
            let mut leaves = vec![zaho.clone()];
            leaves.extend(nai);
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: None,
                zaho: vec![zaho],
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let ki = cmavo("ki").map(|ki| TenseModalSyntax::Composite {
        leaves: vec![ki.clone()],
        time: None,
        space: None,
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: Some(ki),
        cuhe: None,
        fiho: Vec::new(),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });
    let cuhe = cmavo_of("CUhE", &["cu'e", "nau"]).map(|cuhe| TenseModalSyntax::Composite {
        leaves: vec![cuhe.clone()],
        time: None,
        space: None,
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: Some(cuhe),
        fiho: Vec::new(),
        connectives: Vec::new(),
        free_modifiers: Vec::new(),
    });

    let zeha_clause = cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"])
        .then(
            cmavo_of("PU", &["pu", "ca", "ba"])
                .then(cmavo("nai").or_not())
                .or_not(),
        )
        .map(|(zeha, pu_nai)| {
            let mut leaves = vec![zeha];
            if let Some((pu, nai)) = pu_nai {
                leaves.push(pu);
                leaves.extend(nai);
            }
            tense_modal_from_leaves(leaves, Vec::new())
        });
    let interval_property = choice((numbered_interval, tahe_interval, zaho)).boxed();
    let time_offset = pu;
    let time_tense = choice((
        zi.clone()
            .then(time_offset.clone().repeated().collect::<Vec<_>>())
            .then(zeha_clause.clone().or_not())
            .then(interval_property.clone().repeated().collect::<Vec<_>>())
            .map(|(((zi, offsets), zeha), props)| {
                let mut parts = vec![zi];
                parts.extend(offsets);
                parts.extend(zeha);
                parts.extend(props);
                combine_composite_tense_modals(parts)
            }),
        zi.clone()
            .or_not()
            .then(
                time_offset
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(zeha_clause.clone().or_not())
            .then(interval_property.clone().repeated().collect::<Vec<_>>())
            .map(|(((zi, offsets), zeha), props)| {
                let mut parts = Vec::new();
                parts.extend(zi);
                parts.extend(offsets);
                parts.extend(zeha);
                parts.extend(props);
                combine_composite_tense_modals(parts)
            }),
        zi.clone()
            .or_not()
            .then(time_offset.clone().repeated().collect::<Vec<_>>())
            .then(zeha_clause.clone())
            .then(interval_property.clone().repeated().collect::<Vec<_>>())
            .map(|(((zi, offsets), zeha), props)| {
                let mut parts = Vec::new();
                parts.extend(zi);
                parts.extend(offsets);
                parts.push(zeha);
                parts.extend(props);
                combine_composite_tense_modals(parts)
            }),
        zi.or_not()
            .then(time_offset.repeated().collect::<Vec<_>>())
            .then(zeha_clause.or_not())
            .then(
                interval_property
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(((zi, offsets), zeha), props)| {
                let mut parts = Vec::new();
                parts.extend(zi);
                parts.extend(offsets);
                parts.extend(zeha);
                parts.extend(props);
                combine_composite_tense_modals(parts)
            }),
    ))
    .boxed();

    let space_offset = faha;
    let veha_viha = cmavo_of("VEhA", &["ve'i", "ve'a", "ve'u", "ve'e"])
        .then(cmavo_of("VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"]).or_not())
        .map(|(veha, viha)| {
            let mut leaves = vec![veha];
            leaves.extend(viha);
            tense_modal_from_leaves(leaves, Vec::new())
        })
        .or(cmavo_of("VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"])
            .map(|viha| tense_modal_from_leaves(vec![viha], Vec::new())));
    let faha_nai = cmavo_of(
        "FAhA",
        &[
            "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a", "ru'u",
            "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a", "zo'i", "ze'o",
        ],
    )
    .then(cmavo("nai").or_not())
    .map(|(faha, nai)| {
        let mut leaves = vec![faha];
        leaves.extend(nai);
        tense_modal_from_leaves(leaves, Vec::new())
    });
    let fehe_interval_property = cmavo("fe'e")
        .then(interval_property)
        .map(|(fehe, interval)| {
            combine_composite_tense_modals(vec![
                tense_modal_from_leaves(vec![fehe], Vec::new()),
                interval,
            ])
        });
    let space_interval_properties = fehe_interval_property
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(combine_composite_tense_modals)
        .boxed();
    let space_interval = veha_viha
        .then(faha_nai.or_not())
        .then(space_interval_properties.clone().or_not())
        .map(|((vv, faha), props)| {
            let mut parts = vec![vv];
            parts.extend(faha);
            parts.extend(props);
            combine_composite_tense_modals(parts)
        })
        .or(space_interval_properties)
        .boxed();
    let mohi_offset = cmavo("mo'i")
        .then(space_offset.clone())
        .map(|(mohi, offset)| {
            combine_composite_tense_modals(vec![
                tense_modal_from_leaves(vec![mohi], Vec::new()),
                offset,
            ])
        });
    let space_tense = choice((
        va.clone()
            .then(space_offset.clone().repeated().collect::<Vec<_>>())
            .then(space_interval.clone().or_not())
            .then(mohi_offset.clone().or_not())
            .map(|(((va, offsets), interval), mohi)| {
                let mut parts = vec![va];
                parts.extend(offsets);
                parts.extend(interval);
                parts.extend(mohi);
                combine_composite_tense_modals(parts)
            }),
        va.clone()
            .or_not()
            .then(
                space_offset
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(space_interval.clone().or_not())
            .then(mohi_offset.clone().or_not())
            .map(|(((va, offsets), interval), mohi)| {
                let mut parts = Vec::new();
                parts.extend(va);
                parts.extend(offsets);
                parts.extend(interval);
                parts.extend(mohi);
                combine_composite_tense_modals(parts)
            }),
        va.clone()
            .or_not()
            .then(space_offset.clone().repeated().collect::<Vec<_>>())
            .then(space_interval.clone())
            .then(mohi_offset.clone().or_not())
            .map(|(((va, offsets), interval), mohi)| {
                let mut parts = Vec::new();
                parts.extend(va);
                parts.extend(offsets);
                parts.push(interval);
                parts.extend(mohi);
                combine_composite_tense_modals(parts)
            }),
        va.or_not()
            .then(space_offset.repeated().collect::<Vec<_>>())
            .then(space_interval.or_not())
            .then(mohi_offset)
            .map(|(((va, offsets), interval), mohi)| {
                let mut parts = Vec::new();
                parts.extend(va);
                parts.extend(offsets);
                parts.extend(interval);
                parts.push(mohi);
                combine_composite_tense_modals(parts)
            }),
    ))
    .boxed();

    let time_space_caha = choice((
        time_tense
            .clone()
            .then(space_tense.clone().or_not())
            .then(caha.clone().or_not())
            .map(|((time, space), caha)| {
                let mut parts = vec![time];
                parts.extend(space);
                parts.extend(caha);
                combine_composite_tense_modals(parts)
            }),
        space_tense
            .then(time_tense.or_not())
            .then(caha.or_not())
            .map(|((space, time), caha)| {
                let mut parts = vec![space];
                parts.extend(time);
                parts.extend(caha);
                combine_composite_tense_modals(parts)
            }),
        cmavo_of("CAhA", CAHA_WORDS).map(|caha| tense_modal_from_leaves(vec![caha], Vec::new())),
    ))
    .boxed();
    let nahe_before_time_space_caha = cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .then(time_space_caha.clone().rewind())
        .rewind()
        .ignore_then(cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"]));

    nahe_before_time_space_caha
        .or_not()
        .then(time_space_caha)
        .then(ki.or_not())
        .map(|((nahe, tense), ki)| {
            let tense = match nahe {
                Some(nahe) => prefix_tense_modal_nahe(nahe, tense),
                None => tense,
            };
            if let Some(ki) = ki {
                combine_composite_tense_modals(vec![tense, ki])
            } else {
                tense
            }
        })
        .or(cuhe)
        .boxed()
}

#[requires(matches!(modal, TenseModalSyntax::Composite { .. }))]
#[ensures(matches!(ret, TenseModalSyntax::Composite { simple: Some(SimpleTenseModalSyntax { nahe: Some(_), .. }), .. }))]
fn prefix_tense_modal_nahe(
    nahe: WithIndicators<WordLike>,
    modal: TenseModalSyntax,
) -> TenseModalSyntax {
    let TenseModalSyntax::Composite {
        mut leaves,
        time,
        space,
        simple,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        fiho,
        connectives,
        free_modifiers,
    } = modal
    else {
        unreachable!("prefix_tense_modal_nahe requires a composite tense modal")
    };
    leaves.insert(0, nahe.clone());
    let simple = Some(match simple {
        Some(simple) => SimpleTenseModalSyntax {
            nahe: Some(nahe),
            se: simple.se,
            bai: simple.bai,
            nai: simple.nai,
        },
        None => SimpleTenseModalSyntax {
            nahe: Some(nahe),
            se: None,
            bai: None,
            nai: None,
        },
    });
    TenseModalSyntax::Composite {
        leaves,
        time,
        space,
        simple,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        fiho,
        connectives,
        free_modifiers,
    }
}

#[requires(!parts.is_empty())]
#[ensures(matches!(ret, TenseModalSyntax::Composite { .. }))]
fn combine_composite_tense_modals(parts: Vec<TenseModalSyntax>) -> TenseModalSyntax {
    let mut leaves = Vec::new();
    let mut time_direction = Vec::new();
    let mut time_distance = None;
    let mut time_interval = None;
    let mut time_nai = None;
    let mut space_direction = Vec::new();
    let mut space_distance = Vec::new();
    let mut space_interval = Vec::new();
    let mut space_dimensions = Vec::new();
    let mut space_mohi = None;
    let mut space_fehe = None;
    let mut simple = None;
    let mut zaho = Vec::new();
    let mut caha = None;
    let mut ki = None;
    let mut cuhe = None;
    let mut fiho = Vec::new();
    let mut connectives = Vec::new();
    let mut interval = None;
    let mut free_modifiers = Vec::new();

    for part in parts {
        if let TenseModalSyntax::Composite {
            leaves: part_leaves,
            time,
            space,
            simple: part_simple,
            interval: part_interval,
            zaho: part_zaho,
            caha: part_caha,
            ki: part_ki,
            cuhe: part_cuhe,
            fiho: part_fiho,
            connectives: part_connectives,
            free_modifiers: part_free_modifiers,
        } = part
        {
            leaves.extend(part_leaves);
            if let Some(time) = time {
                time_direction.extend(time.direction);
                time_distance = time_distance.or(time.distance);
                time_interval = time_interval.or(time.interval);
                time_nai = time_nai.or(time.nai);
            }
            if let Some(space) = space {
                space_direction.extend(space.direction);
                space_distance.extend(space.distance);
                space_interval.extend(space.interval);
                space_dimensions.extend(space.dimensions);
                space_mohi = space_mohi.or(space.mohi);
                space_fehe = space_fehe.or(space.fehe);
            }
            simple = simple.or(part_simple);
            zaho.extend(part_zaho);
            caha = caha.or(part_caha);
            ki = ki.or(part_ki);
            cuhe = cuhe.or(part_cuhe);
            fiho.extend(part_fiho);
            interval = interval.or(part_interval);
            connectives.extend(part_connectives);
            free_modifiers.extend(part_free_modifiers);
        }
    }

    let time = (!time_direction.is_empty() || time_distance.is_some() || time_interval.is_some())
        .then_some(TimeTenseSyntax {
            direction: time_direction,
            distance: time_distance,
            interval: time_interval,
            nai: time_nai,
        });
    let space = (!space_direction.is_empty()
        || !space_distance.is_empty()
        || !space_interval.is_empty()
        || !space_dimensions.is_empty()
        || space_mohi.is_some()
        || space_fehe.is_some())
    .then_some(SpaceTenseSyntax {
        direction: space_direction,
        distance: space_distance,
        interval: space_interval,
        dimensions: space_dimensions,
        mohi: space_mohi,
        fehe: space_fehe,
    });

    TenseModalSyntax::Composite {
        leaves,
        time,
        space,
        simple,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        fiho,
        connectives,
        free_modifiers,
    }
}

#[requires(true)]
#[ensures(true)]
fn leading_term_tag_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let pu_before_nahe = cmavo_of("PU", &["pu", "ca", "ba"])
        .then(cmavo("nai").or_not())
        .then(
            cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
                .rewind()
                .ignored(),
        )
        .map(|((pu, nai), _)| {
            let mut leaves = vec![pu.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: Some(TimeTenseSyntax {
                    direction: vec![pu],
                    distance: None,
                    interval: None,
                    nai,
                }),
                space: None,
                simple: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let pu_distance_before_tag = cmavo_of("PU", &["pu", "ca", "ba"])
        .then(cmavo("nai").or_not())
        .then(cmavo_of("ZI", &["zi", "za", "zu"]))
        .then(cmavo_of("ZI", &["zi", "za", "zu"]).rewind())
        .map(|(((pu, nai), distance), _)| {
            let mut leaves = vec![pu.clone()];
            leaves.extend(nai.clone());
            leaves.push(distance.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: Some(TimeTenseSyntax {
                    direction: vec![pu],
                    distance: Some(distance),
                    interval: None,
                    nai,
                }),
                space: None,
                simple: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let zi_before_zi = cmavo_of("ZI", &["zi", "za", "zu"])
        .then(cmavo_of("ZI", &["zi", "za", "zu"]).rewind())
        .map(|(zi, _)| TenseModalSyntax::Composite {
            leaves: vec![zi.clone()],
            time: Some(TimeTenseSyntax {
                direction: Vec::new(),
                distance: Some(zi),
                interval: None,
                nai: None,
            }),
            space: None,
            simple: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            fiho: Vec::new(),
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        });
    let va_before_va = cmavo_of("VA", &["vi", "va", "vu"])
        .then(cmavo_of("VA", &["vi", "va", "vu"]).rewind())
        .map(|(va, _)| TenseModalSyntax::Composite {
            leaves: vec![va.clone()],
            time: None,
            space: Some(SpaceTenseSyntax {
                direction: Vec::new(),
                distance: vec![va],
                interval: Vec::new(),
                dimensions: Vec::new(),
                mohi: None,
                fehe: None,
            }),
            simple: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            fiho: Vec::new(),
            connectives: Vec::new(),
            free_modifiers: Vec::new(),
        });
    let mohi_before_mohi = cmavo("mo'i")
        .then(cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        ))
        .then(cmavo("nai").or_not())
        .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
        .then(cmavo("mo'i").rewind())
        .map(|((((mohi, direction), nai), distance), _)| {
            let mut leaves = vec![mohi.clone(), direction.clone()];
            leaves.extend(nai);
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: Some(SpaceTenseSyntax {
                    direction: vec![direction],
                    distance: distance.into_iter().collect(),
                    interval: Vec::new(),
                    dimensions: Vec::new(),
                    mohi: Some(mohi),
                    fehe: None,
                }),
                simple: None,
                interval: None,
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let zaho_property = cmavo_of("ZAhO", ZAHO_WORDS)
        .then(cmavo("nai").or_not())
        .map(|(zaho, nai)| {
            let mut leaves = vec![zaho.clone()];
            leaves.extend(nai);
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: None,
                zaho: vec![zaho],
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let numbered_interval_start = number_words()
        .then(cmavo_of("ROI", ROI_WORDS))
        .rewind()
        .ignored();
    let numbered_interval = numbered_interval_start
        .ignore_then(number_words())
        .then(cmavo_of("ROI", ROI_WORDS))
        .then(cmavo("nai").or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let mut leaves = number.clone();
            leaves.push(roi_or_tahe.clone());
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: Some(IntervalTenseSyntax {
                    number,
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let tahe_interval = cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
        .then(cmavo("nai").or_not())
        .map(|(roi_or_tahe, nai)| {
            let mut leaves = vec![roi_or_tahe.clone()];
            leaves.extend(nai.clone());
            TenseModalSyntax::Composite {
                leaves,
                time: None,
                space: None,
                simple: None,
                interval: Some(IntervalTenseSyntax {
                    number: Vec::new(),
                    roi_or_tahe,
                    nai,
                }),
                zaho: Vec::new(),
                caha: None,
                ki: None,
                cuhe: None,
                fiho: Vec::new(),
                connectives: Vec::new(),
                free_modifiers: Vec::new(),
            }
        });
    let caha_before_tag = cmavo_of("CAhA", CAHA_WORDS)
        .then(tense_modal().rewind())
        .map(|(caha, _)| TenseModalSyntax::Caha {
            word: caha,
            free_modifiers: Vec::new(),
        });
    let property_split_follower = choice((
        cmavo_of("PU", &["pu", "ca", "ba"]).ignored(),
        cmavo_of("ZI", &["zi", "za", "zu"]).ignored(),
        cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]).ignored(),
        cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
            .then(cmavo_of("CAhA", CAHA_WORDS))
            .ignored(),
        simple_tense_modal().ignored(),
        fiho_tense_modal().ignored(),
    ));
    let leading_interval_property = choice((zaho_property, numbered_interval, tahe_interval))
        .then(property_split_follower.rewind());

    choice((
        pu_before_nahe,
        pu_distance_before_tag,
        zi_before_zi,
        va_before_va,
        mohi_before_mohi,
        caha_before_tag,
        leading_interval_property.map(|(tense_modal, _)| tense_modal),
        tense_modal(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let atom = tense_modal_atom();
    atom.clone()
        .then(
            choice((joik_connective(), jek_connective()))
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| combine_connected_tense_modals(first, continuations))
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn combine_connected_tense_modals(
    first: TenseModalSyntax,
    continuations: Vec<(ConnectiveSyntax, TenseModalSyntax)>,
) -> TenseModalSyntax {
    if continuations.is_empty() {
        return first;
    }

    let mut parts = vec![tense_modal_as_composite(first)];
    for (connective, tense_modal) in continuations {
        parts.push(connective_tense_modal_from_leaves(
            connective_tense_modal_leaves(connective),
        ));
        parts.push(tense_modal_as_composite(tense_modal));
    }
    combine_composite_tense_modals(parts)
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_atom<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    #[derive(Clone)]
    #[invariant(true)]
    enum PuTail {
        Distance(WithIndicators<WordLike>),
        Caha(WithIndicators<WordLike>),
    }

    choice((
        composite_tense_modal(),
        cmavo_of("PU", &["pu", "ca", "ba"])
            .then(
                choice((
                    cmavo_of("ZI", &["zi", "za", "zu"]).map(PuTail::Distance),
                    cmavo_of("CAhA", CAHA_WORDS).map(PuTail::Caha),
                ))
                .or_not(),
            )
            .map(|(pu, tail)| match tail {
                Some(PuTail::Distance(distance)) => TenseModalSyntax::PuDistance {
                    pu,
                    distance,
                    free_modifiers: Vec::new(),
                },
                Some(PuTail::Caha(caha)) => TenseModalSyntax::PuCaha {
                    pu,
                    caha,
                    free_modifiers: Vec::new(),
                },
                None => TenseModalSyntax::Pu {
                    word: pu,
                    free_modifiers: Vec::new(),
                },
            }),
        cmavo_of("VA", &["vi", "va", "vu"]).map(|word| TenseModalSyntax::SpaceDistance {
            word,
            free_modifiers: Vec::new(),
        }),
        cmavo_of("ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]).map(|word| {
            TenseModalSyntax::TimeInterval {
                word,
                free_modifiers: Vec::new(),
            }
        }),
        cmavo_of(
            "FAhA",
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        )
        .map(|word| TenseModalSyntax::SpaceDirection {
            word,
            free_modifiers: Vec::new(),
        }),
        cmavo("mo'i")
            .then(cmavo_of(
                "FAhA",
                &[
                    "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                    "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                    "zo'i", "ze'o",
                ],
            ))
            .then(cmavo_of("VA", &["vi", "va", "vu"]).or_not())
            .map(
                |((mohi, direction), distance)| TenseModalSyntax::SpaceMovement {
                    mohi,
                    direction,
                    distance,
                    free_modifiers: Vec::new(),
                },
            ),
        cmavo_of("CAhA", CAHA_WORDS).map(|word| TenseModalSyntax::Caha {
            word,
            free_modifiers: Vec::new(),
        }),
        fiho_tense_modal(),
        cmavo_of("ZAhO", ZAHO_WORDS).map(|word| TenseModalSyntax::Zaho {
            words: vec![word],
            free_modifiers: Vec::new(),
        }),
        simple_tense_modal(),
        flat_tag_chunk_tense_modal(),
        cmavo("ki").map(|ki| TenseModalSyntax::Ki {
            ki,
            free_modifiers: Vec::new(),
        }),
        pa_word()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cmavo_of("ROI", ROI_WORDS).or(cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])),
            )
            .then(cmavo("nai").or_not())
            .map(|((number, roi_or_tahe), nai)| TenseModalSyntax::Interval {
                number,
                roi_or_tahe,
                nai,
                free_modifiers: Vec::new(),
            }),
        cmavo_of("TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
            .then(cmavo("nai").or_not())
            .map(|(roi_or_tahe, nai)| TenseModalSyntax::Interval {
                number: Vec::new(),
                roi_or_tahe,
                nai,
                free_modifiers: Vec::new(),
            }),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn simple_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    cmavo_of("NAhE", &["na'e", "to'e", "no'e", "je'a"])
        .or_not()
        .then(cmavo_of("SE", &["se", "te", "ve", "xe"]).or_not())
        .then(cmavo_of("BAI", BAI_WORDS))
        .then(cmavo("nai").or_not())
        .then(cmavo("ki").or_not())
        .map(|((((nahe, se), bai), nai), ki)| TenseModalSyntax::Simple {
            nahe,
            se,
            bai,
            nai,
            ki,
            connectives: Vec::new(),
            extra_leaves: Vec::new(),
            free_modifiers: Vec::new(),
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn link_argument_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, LinkArgumentSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let argument_base = argument
        .clone()
        .or(na_ku_argument_parser(free_modifier.clone()))
        .boxed();
    let fa_tail = argument_base
        .clone()
        .map(|argument| (Some(argument), None, Vec::new()))
        .or(cmavo("ku")
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)));
    let fa_link_argument = cmavo_of("FA", FA_WORDS)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(fa_tail)
        .map(
            |((fa, mut fa_free_modifiers), (argument, maybe_ku, trailing_free_modifiers))| {
                if let Some(argument) = argument {
                    new!(LinkArgumentSyntax {
                        fa: Some(WithFreeModifiers::new(fa, fa_free_modifiers)),
                        argument: Some(argument),
                    })
                } else {
                    fa_free_modifiers.extend(trailing_free_modifiers);
                    new!(LinkArgumentSyntax {
                        fa: None,
                        argument: Some(ArgumentSyntax::Zohe {
                            tag_words: vec![fa],
                            maybe_ku,
                            free_modifiers: fa_free_modifiers,
                        }),
                    })
                }
            },
        );
    let tagged_tail = argument_base
        .clone()
        .map(|argument| (Some(argument), None, Vec::new()))
        .or(cmavo("ku")
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)));
    let tagged_link_argument = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail)
        .map(
            |(
                (tense_modal, mut tag_free_modifiers),
                (argument, maybe_ku, trailing_free_modifiers),
            )| {
                let tag_words = tense_modal.clone().leaf_words();
                tag_free_modifiers.extend(tense_modal.clone().free_modifiers());
                if let Some(argument) = argument {
                    new!(LinkArgumentSyntax {
                        fa: None,
                        argument: Some(ArgumentSyntax::Tagged {
                            tag_words,
                            tag_tense_modal: Some(tense_modal),
                            tag_fa: None,
                            free_modifiers: tag_free_modifiers,
                            inner_argument: Box::new(argument),
                        }),
                    })
                } else {
                    tag_free_modifiers.extend(trailing_free_modifiers);
                    new!(LinkArgumentSyntax {
                        fa: None,
                        argument: Some(ArgumentSyntax::Zohe {
                            tag_words,
                            maybe_ku,
                            free_modifiers: tag_free_modifiers,
                        }),
                    })
                }
            },
        );
    let plain_argument = argument_base.map(|argument| {
        new!(LinkArgumentSyntax {
            fa: None,
            argument: Some(argument),
        })
    });

    choice((fa_link_argument, tagged_link_argument, plain_argument)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn empty_link_argument() -> LinkArgumentSyntax {
    new!(LinkArgumentSyntax {
        fa: None,
        argument: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn be_link_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, BeLinkSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(argument.clone(), free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo("be")
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(link_argument)
        .then(
            bei_link_parser(argument, free_modifier.clone())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(
            cmavo("be'o")
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((be, free_modifiers), link_argument), bei_links), beho)| {
                let data!(LinkArgumentSyntax { fa, argument }) = link_argument.into_data();

                new!(BeLinkSyntax {
                    be: WithFreeModifiers::new(be, free_modifiers),
                    fa,
                    first_argument: argument,
                    bei_links,
                    beho: beho.map(|(beho, free_modifiers)| {
                        WithFreeModifiers::new(beho, free_modifiers)
                    }),
                })
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn bei_link_parser<'tokens, A, F>(
    argument: A,
    free_modifier: F,
) -> BoxedParser<'tokens, BeiLinkSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, ArgumentSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(argument, free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo("bei")
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(link_argument)
        .map(|((bei, bei_free_modifiers), link_argument)| {
            let data!(LinkArgumentSyntax { fa, argument }) = link_argument.into_data();

            BeiLinkSyntax {
                bei: WithFreeModifiers::new(bei, bei_free_modifiers),
                fa,
                argument,
            }
        })
        .boxed()
}
