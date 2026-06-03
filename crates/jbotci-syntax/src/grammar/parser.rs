use super::tense::*;
use super::tokens::*;
use super::*;
use crate::SyntaxExpectedTokenData;
use chumsky::input::MapExtra;
use chumsky::primitive::custom;
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Selmaho};
use std::{cell::Cell, sync::Arc};

type OptionalWordWithFreeModifiers = Option<WithFreeModifiers<Token>>;
type OptionalWordFreeModifierSplit = (OptionalWordWithFreeModifiers, Vec<FreeModifierSyntax>);
type BoxedSumtiSyntax = Box<SumtiSyntax>;
type BoxedTermSyntax = Box<TermSyntax>;
type BoxedQuantifierSyntax = Box<QuantifierSyntax>;
type BoxedTenseModalSyntax = Box<TenseModalSyntax>;
type BoxedSelbriSyntax = Box<SelbriSyntax>;
type BoxedTanruUnitSyntax = Box<TanruUnitSyntax>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BoSumtiTailSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<BoxedTenseModalSyntax>,
    bo: Token,
    free_modifiers: Vec<FreeModifierSyntax>,
    trailing_sumti: BoxedSumtiSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct KeSumtiTailSyntax {
    connective: ConnectiveSyntax,
    tense_modal: Option<BoxedTenseModalSyntax>,
    ke: Token,
    free_modifiers: Vec<FreeModifierSyntax>,
    inner_sumti: BoxedSumtiSyntax,
    kehe: Option<WithFreeModifiers<Token>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BoTanruUnitTailSyntax {
    connective: Option<Box<ConnectiveSyntax>>,
    tense_modal: Option<BoxedTenseModalSyntax>,
    bo: Token,
    free_modifiers: Vec<FreeModifierSyntax>,
    trailing_unit: BoxedTanruUnitSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GekNuhiTermsetHeadSyntax {
    m_nuhi: Option<(Token, Vec<FreeModifierSyntax>)>,
    gek: ConnectiveSyntax,
    terms: Vec<BoxedTermSyntax>,
    nuhu: Option<(Token, Vec<FreeModifierSyntax>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct LeadingIStatementSyntax {
    i: Token,
    connective: Option<ConnectiveSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::Pehe => !tails.is_empty())]
#[invariant(::TermConnection => !tails.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TermContinuationSyntax {
    Pehe {
        tails: Vec<(
            Token,
            Vec<FreeModifierSyntax>,
            ConnectiveSyntax,
            BoxedTermSyntax,
        )>,
    },
    TermConnection {
        tails: Vec<(ConnectiveSyntax, BoxedTermSyntax)>,
    },
    None,
}

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct ParserDialectConfig {
    term_hierarchy_enabled: bool,
    cbm_enabled: bool,
    soi_adverbials_enabled: bool,
    zantufa_adverbials_enabled: bool,
    zantufa_connectives_enabled: bool,
    zantufa_quotes_enabled: bool,
    zantufa_tags_enabled: bool,
}

impl ParserDialectConfig {
    #[requires(true)]
    #[ensures(!ret.term_hierarchy_enabled)]
    const fn empty() -> Self {
        Self {
            term_hierarchy_enabled: false,
            cbm_enabled: false,
            soi_adverbials_enabled: false,
            zantufa_adverbials_enabled: false,
            zantufa_connectives_enabled: false,
            zantufa_quotes_enabled: false,
            zantufa_tags_enabled: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_options(options: &ParseOptions) -> Self {
        let features = &options.dialect.features;
        Self {
            term_hierarchy_enabled: features.contains(&DialectFeature::TermHierarchy),
            cbm_enabled: features.contains(&DialectFeature::Cbm),
            soi_adverbials_enabled: features.contains(&DialectFeature::SoiAdverbials),
            zantufa_adverbials_enabled: features.contains(&DialectFeature::ZantufaAdverbials),
            zantufa_connectives_enabled: features.contains(&DialectFeature::ZantufaConnectives),
            zantufa_quotes_enabled: features.contains(&DialectFeature::ZantufaQuotes),
            zantufa_tags_enabled: features.contains(&DialectFeature::ZantufaTags),
        }
    }
}

thread_local! {
    static PARSER_DIALECT_CONFIG: Cell<ParserDialectConfig> =
        const { Cell::new(ParserDialectConfig::empty()) };
}

#[derive(Debug)]
#[invariant(true)]
struct ParserDialectConfigScope {
    previous: ParserDialectConfig,
}

impl ParserDialectConfigScope {
    #[requires(true)]
    #[ensures(true)]
    fn enter(config: ParserDialectConfig) -> Self {
        let previous = PARSER_DIALECT_CONFIG.with(|current| current.replace(config));
        Self { previous }
    }
}

impl Drop for ParserDialectConfigScope {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        PARSER_DIALECT_CONFIG.with(|current| current.set(self.previous));
    }
}

#[requires(true)]
#[ensures(true)]
fn parser_dialect_config() -> ParserDialectConfig {
    PARSER_DIALECT_CONFIG.with(Cell::get)
}

#[requires(true)]
#[ensures(ret.free_modifier_count() >= old(free_modifiers.len()))]
fn attach_tense_modal_free_modifiers(
    tense_modal: TenseModalSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
    match tense_modal.into_data() {
        data!(TenseModalSyntax::Composite { mut parts }) => {
            parts.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::Composite { parts })
        }
        data!(TenseModalSyntax::TimeDirection(mut word)) => {
            word.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::TimeDirection(word))
        }
        data!(TenseModalSyntax::TimeDirectionDistance { pu, mut distance }) => {
            distance.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::TimeDirectionDistance { pu, distance })
        }
        data!(TenseModalSyntax::TimeInterval(mut word)) => {
            word.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::TimeInterval(word))
        }
        data!(TenseModalSyntax::TimeDirectionActuality { pu, mut caha }) => {
            caha.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::TimeDirectionActuality { pu, caha })
        }
        data!(TenseModalSyntax::SpaceDistance(mut word)) => {
            word.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::SpaceDistance(word))
        }
        data!(TenseModalSyntax::SpaceDirection(mut word)) => {
            word.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::SpaceDirection(word))
        }
        data!(TenseModalSyntax::SpaceMovement {
            mohi,
            mut direction,
            mut distance,
        }) => {
            if let Some(distance) = &mut distance {
                distance.free_modifiers.extend(free_modifiers);
            } else {
                direction.free_modifiers.extend(free_modifiers);
            }
            new!(TenseModalSyntax::SpaceMovement {
                mohi,
                direction,
                distance,
            })
        }
        data!(TenseModalSyntax::Modal {
            nahe,
            se,
            mut bai,
            mut nai,
            mut ki,
        }) => {
            if let Some(ki) = &mut ki {
                ki.free_modifiers.extend(free_modifiers);
            } else if let Some(nai) = &mut nai {
                nai.free_modifiers.extend(free_modifiers);
            } else {
                bai.free_modifiers.extend(free_modifiers);
            }
            new!(TenseModalSyntax::Modal {
                nahe,
                se,
                bai,
                nai,
                ki,
            })
        }
        data!(TenseModalSyntax::Sticky(mut ki)) => {
            ki.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::Sticky(ki))
        }
        data!(TenseModalSyntax::AdHocModal {
            mut fiho,
            selbri,
            mut fehu,
        }) => {
            if let Some(fehu) = &mut fehu {
                fehu.free_modifiers.extend(free_modifiers);
            } else {
                fiho.free_modifiers.extend(free_modifiers);
            }
            new!(TenseModalSyntax::AdHocModal { fiho, selbri, fehu })
        }
        data!(TenseModalSyntax::Actuality(mut word)) => {
            word.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::Actuality(word))
        }
        data!(TenseModalSyntax::EventContour(mut words)) => {
            words.free_modifiers.extend(free_modifiers);
            new!(TenseModalSyntax::EventContour(words))
        }
        data!(TenseModalSyntax::IntervalProperty {
            number,
            mut roi_or_tahe,
            mut nai,
        }) => {
            if let Some(nai) = &mut nai {
                nai.free_modifiers.extend(free_modifiers);
            } else {
                roi_or_tahe.free_modifiers.extend(free_modifiers);
            }
            new!(TenseModalSyntax::IntervalProperty {
                number,
                roi_or_tahe,
                nai,
            })
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().free_modifier_count() >= old(free_modifiers.len()))]
fn attach_boxed_tense_modal_free_modifiers(
    tense_modal: BoxedTenseModalSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> BoxedTenseModalSyntax {
    Box::new(attach_tense_modal_free_modifiers(
        *tense_modal,
        free_modifiers,
    ))
}

#[requires(true)]
#[ensures(matches!(
    ret.as_ref().as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
fn boxed_tense_modal_from_leaves(
    leaves: Vec<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> BoxedTenseModalSyntax {
    Box::new(tense_modal_from_leaves(leaves, free_modifiers))
}

#[requires(true)]
#[ensures(true)]
fn split_optional_word_free_modifiers(
    word: Option<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> OptionalWordFreeModifierSplit {
    match word {
        Some(word) => (
            Some(WithFreeModifiers::new(word, free_modifiers)),
            Vec::new(),
        ),
        None => (None, free_modifiers),
    }
}

#[requires(true)]
#[ensures(true)]
fn build_zohe_argument(
    tag: Option<SumtiTagSyntax>,
    maybe_ku: Option<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> SumtiSyntax {
    let (maybe_ku, free_modifiers) = split_optional_word_free_modifiers(maybe_ku, free_modifiers);
    new!(SumtiSyntax::ElidedSumti {
        tag: tag.map(Box::new),
        maybe_ku,
        free_modifiers,
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_statement(
    words: &[Token],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<ParsedStatement, SyntaxError> {
    parse_statement_attempt(words, source, options).result
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_statement_attempt(
    words: &[Token],
    source: Option<&str>,
    options: &ParseOptions,
) -> ParsedStatementAttempt {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);
    let mut state = ParserState::new(words, options);

    let result = statement_parser(source, options)
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

#[requires(true)]
#[ensures(true)]
fn statement_parser<'tokens>(
    source: Option<&'tokens str>,
    options: &ParseOptions,
) -> BoxedParser<'tokens, TextSyntax> {
    let _dialect_scope =
        ParserDialectConfigScope::enter(ParserDialectConfig::from_options(options));
    let dialect = parser_dialect_config();
    let mut text = Recursive::declare();
    let mut sumti = Recursive::declare();
    let mut selbri = Recursive::declare();
    let mut statement = Recursive::declare();
    let mut subbridi = Recursive::declare();
    let mut free_modifier = Recursive::declare();
    let mut term = Recursive::declare();
    sumti.define(syntax_context(
        "sumti",
        argument_parser_with(
            sumti.clone(),
            selbri.clone(),
            subbridi.clone(),
            term.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ),
    ));
    let tense_modal_with_free_modifiers = tense_modal_boxed()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(tense_modal, free_modifiers)| {
            attach_boxed_tense_modal_free_modifiers(tense_modal, free_modifiers)
        })
        .boxed();
    selbri.define(syntax_context(
        "selbri",
        relation_parser_with(
            sumti.clone(),
            selbri.clone(),
            subbridi.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ),
    ));

    let argument_term = sumti
        .clone()
        .map(|value| new!(TermSyntax::Sumti(Box::new(value))));
    let elided_argument = cmavo(Cmavo::Ku)
        .or_not()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(maybe_ku, free_modifiers)| build_zohe_argument(None, maybe_ku, free_modifiers));
    let fa_term = selmaho(Selmaho::Fa)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(sumti.clone().or(elided_argument))
        .map(|((fa, free_modifiers), sumti)| {
            new!(TermSyntax::PlaceTaggedSumti {
                fa: WithFreeModifiers::new(fa, free_modifiers),
                sumti: Box::new(sumti),
                ku: None,
            })
        });
    let zantufa_jai_tag_term = cmavo(Cmavo::Jai)
        .map_with(
            |jai, extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZantufaJaiTagTerm, &jai);
                jai
            },
        )
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_boxed().or_not())
        .then(
            sumti.clone().or(cmavo(Cmavo::Ku)
                .or_not()
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(|(maybe_ku, free_modifiers)| {
                    build_zohe_argument(None, maybe_ku, free_modifiers)
                })),
        )
        .map(|(((jai, free_modifiers), tag), sumti)| {
            new!(TermSyntax::JaiTaggedSumti {
                jai: WithFreeModifiers::new(jai, free_modifiers),
                tag,
                sumti: Box::new(sumti),
            })
        })
        .boxed();
    let na_ku_term = na_cmavo()
        .then(cmavo(Cmavo::Ku))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, na_ku), free_modifiers)| {
            new!(TermSyntax::BridiNegation {
                na,
                na_ku: WithFreeModifiers::new(na_ku, free_modifiers),
            })
        });
    let tagged_term_before_tag_start = leading_term_tag_tense_modal_boxed()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_boxed().rewind().ignored())
        .rewind()
        .ignored();
    let bare_na_term_blocker = choice((
        tagged_term_before_tag_start
            .not()
            .ignore_then(selbri.clone().ignored()),
        modal_forethought_connective().ignored(),
        selmaho(Selmaho::Ja).ignored(),
        selmaho(Selmaho::Se)
            .or_not()
            .then(selmaho(Selmaho::A))
            .ignored(),
        selmaho(Selmaho::Se)
            .or_not()
            .then(selmaho(Selmaho::Giha))
            .ignored(),
    ));
    let bare_na_term = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(bare_na_term_blocker.rewind().not())
        .map(|((na, free_modifiers), _)| {
            new!(TermSyntax::BareNegation(WithFreeModifiers::new(
                na,
                free_modifiers
            )))
        });
    let tagged_term_start = modal_forethought_connective()
        .rewind()
        .not()
        .ignore_then(leading_term_tag_tense_modal_boxed())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>());
    let tagged_term_before_tag = tagged_term_start
        .clone()
        .then(tense_modal_boxed().rewind().ignored())
        .map(|((tense_modal, free_modifiers), _)| {
            new!(TermSyntax::TaggedSumti {
                tense_modal: Some(attach_boxed_tense_modal_free_modifiers(
                    tense_modal,
                    free_modifiers,
                )),
                sumti: Box::new(implicit_zohe_argument()),
            })
        });
    let tagged_term_before_non_relation = tagged_term_start
        .then(selbri.clone().rewind().not())
        .then(
            sumti.clone().or(cmavo(Cmavo::Ku)
                .or_not()
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(|(maybe_ku, free_modifiers)| {
                    build_zohe_argument(None, maybe_ku, free_modifiers)
                })),
        )
        .map(|(((tense_modal, free_modifiers), _), sumti)| {
            new!(TermSyntax::TaggedSumti {
                tense_modal: Some(attach_boxed_tense_modal_free_modifiers(
                    tense_modal,
                    free_modifiers,
                )),
                sumti: Box::new(sumti),
            })
        });
    let tagged_term = choice((tagged_term_before_tag, tagged_term_before_non_relation));
    let noiha_terminator =
        if dialect.zantufa_adverbials_enabled {
            cmavo(Cmavo::Fehu)
                .map(Ok)
                .or(cmavo(Cmavo::Ku).map_with(
                    |ku,
                     extra: &mut MapExtra<
                        'tokens,
                        '_,
                        ParserInput<'tokens>,
                        ParseExtra<'tokens>,
                    >| {
                        extra
                            .state()
                            .warn(ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi, &ku);
                        Err(Box::new(ku))
                    },
                ))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not()
                .boxed()
        } else {
            cmavo(Cmavo::Fehu)
                .map(Ok)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not()
                .boxed()
        };
    let noiha_adverbial = selmaho(Selmaho::Noiha)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument_tail_with(
            sumti.clone(),
            sumti.clone(),
            selbri.clone(),
            subbridi.clone(),
            free_modifier.clone(),
        ))
        .then(noiha_terminator)
        .map(
            |(
                ((noiha, leading_free_modifiers), (tail_elements, selbri, relative_clauses)),
                terminator,
            )| {
                match terminator {
                    Some((Err(brigahi_ku), trailing_free_modifiers)) => {
                        new!(TermSyntax::BridiVariableAdverbialTerm {
                            poiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                            tail_elements,
                            selbri,
                            relative_clauses,
                            brigahi_ku: WithFreeModifiers::new(
                                *brigahi_ku,
                                trailing_free_modifiers,
                            ),
                        })
                    }
                    Some((Ok(fehu), trailing_free_modifiers)) => {
                        new!(TermSyntax::RelativeAdverbialTerm {
                            noiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                            tail_elements,
                            selbri,
                            relative_clauses,
                            fehu: Some(WithFreeModifiers::new(fehu, trailing_free_modifiers)),
                        })
                    }
                    None => new!(TermSyntax::RelativeAdverbialTerm {
                        noiha: WithFreeModifiers::new(noiha, leading_free_modifiers),
                        tail_elements,
                        selbri,
                        relative_clauses,
                        fehu: None,
                    }),
                }
            },
        )
        .boxed();
    let fihoi_adverbial = cmavo(Cmavo::Fihoi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .then(
            cmavo(Cmavo::Fihau)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((fihoi, leading_free_modifiers), subbridi), fihau)| {
            new!(TermSyntax::AdHocBridiAdverbialTerm {
                fihoi: WithFreeModifiers::new(fihoi, leading_free_modifiers),
                subbridi: Box::new(subbridi),
                fihau: fihau
                    .map(|(fihau, free_modifiers)| WithFreeModifiers::new(fihau, free_modifiers)),
            })
        })
        .boxed();
    let soi_adverbial = selmaho(Selmaho::Soi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .then(
            cmavo(Cmavo::Sehu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((soi, leading_free_modifiers), subbridi), sehu)| {
            new!(TermSyntax::ReciprocalBridiAdverbialTerm {
                soi: WithFreeModifiers::new(soi, leading_free_modifiers),
                subbridi: Box::new(subbridi),
                sehu: sehu
                    .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
            })
        })
        .boxed();
    let base_simple_term = if dialect.soi_adverbials_enabled {
        let non_jai_term = choice((
            fa_term.clone(),
            tagged_term.clone(),
            noiha_adverbial.clone(),
            fihoi_adverbial.clone(),
            soi_adverbial,
            na_ku_term.clone(),
            argument_term.clone(),
            bare_na_term.clone(),
        ));
        if dialect.zantufa_tags_enabled {
            zantufa_jai_tag_term.or(non_jai_term).boxed()
        } else {
            non_jai_term.boxed()
        }
    } else {
        let non_jai_term = choice((
            fa_term,
            tagged_term,
            noiha_adverbial,
            fihoi_adverbial,
            na_ku_term,
            argument_term,
            bare_na_term,
        ));
        if dialect.zantufa_tags_enabled {
            zantufa_jai_tag_term.or(non_jai_term).boxed()
        } else {
            non_jai_term.boxed()
        }
    };
    let term_body = {
        let term = term.clone();
        let boxed_term = term.clone().map(Box::new).boxed();
        let gek_nuhi_termset_head = cmavo(Cmavo::Nuhi)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .or_not()
            .then(modal_forethought_connective_with_free_modifiers(
                free_modifier.clone(),
            ))
            .then(
                boxed_term
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(
                cmavo(Cmavo::Nuhu)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((m_nuhi, gek), terms), nuhu)| {
                Box::new(GekNuhiTermsetHeadSyntax {
                    m_nuhi,
                    gek,
                    terms,
                    nuhu,
                })
            })
            .boxed();
        let gek_nuhi_termset = gek_nuhi_termset_head
            .then(gik_connective_with_free_modifiers(free_modifier.clone()))
            .then(
                boxed_term
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(optional_gihi_terminator())
            .then(
                cmavo(Cmavo::Nuhu)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|((((head, gik), gik_terms), gihi), gik_nuhu)| {
                let GekNuhiTermsetHeadSyntax {
                    m_nuhi,
                    gek,
                    terms,
                    nuhu,
                } = *head;
                new!(TermSyntax::ForethoughtTermsetConnection {
                    m_nuhi: m_nuhi.map(|(nuhi, free_modifiers)| {
                        WithFreeModifiers::new(nuhi, free_modifiers)
                    }),
                    gek,
                    terms: unbox_terms(terms),
                    nuhu: nuhu.map(|(nuhu, free_modifiers)| {
                        WithFreeModifiers::new(nuhu, free_modifiers)
                    }),
                    gik,
                    gik_terms: unbox_terms(gik_terms),
                    gihi,
                    gik_nuhu: gik_nuhu.map(|(nuhu, free_modifiers)| {
                        WithFreeModifiers::new(nuhu, free_modifiers)
                    }),
                })
            });
        let nuhi_termset = cmavo(Cmavo::Nuhi)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(
                boxed_term
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(
                cmavo(Cmavo::Nuhu)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((nuhi, nuhi_free_modifiers), termset), nuhu)| {
                new!(TermSyntax::Termset {
                    nuhi: WithFreeModifiers::new(nuhi, nuhi_free_modifiers),
                    termset: unbox_terms(termset),
                    nuhu: nuhu
                        .map(|(nuhu, free_modifiers)| WithFreeModifiers::new(nuhu, free_modifiers)),
                })
            });
        let ke_termset =
            cmavo(Cmavo::Ke)
                .map_with(
                    |ke,
                     extra: &mut MapExtra<
                        'tokens,
                        '_,
                        ParserInput<'tokens>,
                        ParseExtra<'tokens>,
                    >| {
                        extra
                            .state()
                            .warn(ExperimentalConstruct::ExperimentalKeTermset, &ke);
                        ke
                    },
                )
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(boxed_term.repeated().at_least(1).collect::<Vec<_>>())
                .then(
                    cmavo(Cmavo::Kehe)
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .or_not(),
                )
                .map(|(((ke, ke_free_modifiers), termset), kehe)| {
                    new!(TermSyntax::Termset {
                        nuhi: WithFreeModifiers::new(ke, ke_free_modifiers),
                        termset: unbox_terms(termset),
                        nuhu: kehe.map(|(kehe, free_modifiers)| WithFreeModifiers::new(
                            kehe,
                            free_modifiers
                        )),
                    })
                });
        let simple_term = choice((
            base_simple_term.clone().map(Box::new),
            gek_nuhi_termset.map(Box::new),
            nuhi_termset.map(Box::new),
            ke_termset.map(Box::new),
        ))
        .boxed();
        let cehe_term = simple_term
            .clone()
            .then(
                cmavo(Cmavo::Cehe)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(simple_term.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(leading_term, cehe_tails)| {
                cehe_tails.into_iter().fold(
                    leading_term,
                    |leading_term, ((cehe, free_modifiers), trailing_term)| {
                        Box::new(new!(TermSyntax::TermsetGroup {
                            leading_terms: vec![*leading_term],
                            cehe: WithFreeModifiers::new(cehe, free_modifiers),
                            trailing_terms: vec![*trailing_term],
                        }))
                    },
                )
            })
            .boxed();
        let post_bo_argument_gate = if dialect.term_hierarchy_enabled {
            empty().to(()).boxed()
        } else {
            sumti.clone().rewind().not().boxed()
        };
        let post_bo_trailing_argument_gate = if dialect.term_hierarchy_enabled {
            empty().to(()).boxed()
        } else {
            sumti.clone().rewind().not().boxed()
        };
        let bo_tail = connective_with_free_modifiers(joik_ek_connective(), free_modifier.clone())
            .then(cmavo(Cmavo::Bo))
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
                        Box::new(new!(TermSyntax::BoundTermConnection {
                            leading_terms: vec![*leading_term],
                            bo_connective: bo_connective.map(Box::new),
                            tense_modal,
                            bo: WithFreeModifiers::new(bo, free_modifiers),
                            trailing_term,
                        }))
                    },
                )
            })
            .boxed();
        let pehe_continuation = cmavo(Cmavo::Pehe)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(statement_connective())
            .then(term2.clone())
            .map(|(((pehe, free_modifiers), connective), trailing_term)| {
                (pehe, free_modifiers, connective, trailing_term)
            })
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|tails| new!(TermContinuationSyntax::Pehe { tails }));
        let term_connection_continuation =
            connective_with_free_modifiers(term_connective(), free_modifier.clone())
                .then(term2.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>()
                .map(|tails| new!(TermContinuationSyntax::TermConnection { tails }));
        term2
            .clone()
            .then(choice((
                pehe_continuation,
                term_connection_continuation,
                empty().to(new!(TermContinuationSyntax::None)),
            )))
            .map(
                |(leading_term, continuation)| match continuation.into_data() {
                    data!(TermContinuationSyntax::Pehe { tails }) => tails.into_iter().fold(
                        leading_term,
                        |leading_term, (pehe, free_modifiers, connective, trailing_term)| {
                            Box::new(new!(TermSyntax::TermsetConnection {
                                leading_terms: vec![*leading_term],
                                pehe: WithFreeModifiers::new(pehe, free_modifiers),
                                connective,
                                trailing_terms: vec![*trailing_term],
                            }))
                        },
                    ),
                    data!(TermContinuationSyntax::TermConnection { tails }) => tails
                        .into_iter()
                        .fold(leading_term, |leading_term, (connective, trailing_term)| {
                            Box::new(new!(TermSyntax::TermConnection {
                                leading_terms: vec![*leading_term],
                                connective,
                                trailing_terms: vec![*trailing_term],
                            }))
                        }),
                    data!(TermContinuationSyntax::None) => leading_term,
                },
            )
            .boxed()
    };
    term.define(syntax_context("term", term_body.map(|term| *term).boxed()));
    let tail_term = cmavo(Cmavo::I)
        .rewind()
        .not()
        .ignore_then(term.clone())
        .boxed();
    let cu = cmavo(Cmavo::Cu);
    let basic_predicate = recursive(|_basic_predicate| {
        let forethought_connection = recursive(|forethought_connection| {
            let pair = modal_forethought_connective_with_free_modifiers(free_modifier.clone())
                .then(subbridi.clone())
                .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                .then(subbridi.clone())
                .then(optional_gihi_terminator())
                .then(tail_term.clone().repeated().collect::<Vec<_>>())
                .then(cmavo(Cmavo::Vau).or_not())
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(
                    |(
                        ((((((gek, first), gik), second), gihi), tail_terms), vau),
                        free_modifiers,
                    )| {
                        let (vau, free_modifiers) =
                            split_optional_word_free_modifiers(vau, free_modifiers);
                        new!(ForethoughtBridiConnectionSyntax::BridiConnection {
                            gek,
                            first: Box::new(first),
                            gik,
                            second: Box::new(second),
                            gihi,
                            tail_terms,
                            vau: vau.map(Arc::new),
                            free_modifiers,
                        })
                    },
                );
            let ke = tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo(Cmavo::Ke))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(forethought_connection.clone())
                .then(
                    cmavo(Cmavo::Kehe)
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .or_not(),
                )
                .map(|((((tense_modal, ke), ke_free_modifiers), inner), kehe)| {
                    new!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
                        tense_modal,
                        ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                        inner: Box::new(inner),
                        kehe: kehe.map(|(kehe, free_modifiers)| {
                            Arc::new(WithFreeModifiers::new(kehe, free_modifiers))
                        }),
                    })
                });
            let na = na_cmavo()
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(forethought_connection.clone())
                .map(|((na, free_modifiers), inner)| {
                    new!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection {
                        na: WithFreeModifiers::new(na, free_modifiers),
                        inner: Box::new(inner),
                    })
                });
            choice((pair, ke, na)).boxed()
        });
        let implicit_tagged_term_before_grouped_gek = tense_modal_with_free_modifiers
            .clone()
            .then(cmavo(Cmavo::Ke).rewind())
            .map(|(tense_modal, _)| {
                new!(TermSyntax::TaggedSumti {
                    tense_modal: Some(tense_modal),
                    sumti: Box::new(implicit_zohe_argument()),
                })
            });
        let non_grouped_gek_term = cmavo(Cmavo::Ke).rewind().not().ignore_then(term.clone());
        let gek_leading_term = choice((
            implicit_tagged_term_before_grouped_gek,
            non_grouped_gek_term,
        ))
        .boxed();
        let predicate_tail_terms = tail_term
            .clone()
            .repeated()
            .collect::<Vec<_>>()
            .then(cmavo(Cmavo::Vau).or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((tail_terms, vau), free_modifiers)| {
                let (vau, free_modifiers) = split_optional_word_free_modifiers(vau, free_modifiers);
                (tail_terms, vau, free_modifiers)
            });
        let experimental_predicate_tail_cu = cu
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .or_not()
            .map(|cu| cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)));
        let bridi_tail = recursive(|bridi_tail| {
            let predicate_tail2 = recursive(|predicate_tail2| {
                let relation_tail3 = selbri.clone().then(predicate_tail_terms.clone()).map(
                    |(selbri, (terms, vau, free_modifiers))| {
                        new!(SimpleBridiTailSyntax::SelbriBridiTail {
                            selbri: Box::new(selbri),
                            terms,
                            vau: vau.map(Arc::new),
                            free_modifiers,
                        })
                    },
                );
                let gek_tail3 = forethought_connection.clone().map(|value| {
                    new!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
                        Box::new(value)
                    ))
                });
                let bo_continuation = predicate_tail_connective()
                    .then(tense_modal_with_free_modifiers.clone().or_not())
                    .then(cmavo(Cmavo::Bo))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(experimental_predicate_tail_cu.clone())
                    .then(predicate_tail2.clone())
                    .then(predicate_tail_terms.clone())
                    .map(
                        |(
                            (
                                ((((connective, tense_modal), bo), bo_free_modifiers), cu),
                                bridi_tail,
                            ),
                            (tail_terms, vau, tail_free_modifiers),
                        )| {
                            new!(BoundBridiTailConnectionSyntax {
                                connective,
                                tense_modal,
                                bo: WithFreeModifiers::new(bo, bo_free_modifiers),
                                cu: cu.map(Arc::new),
                                bridi_tail: Box::new(bridi_tail),
                                tail_terms,
                                vau: vau.map(Arc::new),
                                free_modifiers: tail_free_modifiers,
                            })
                        },
                    )
                    .boxed();
                choice((gek_tail3, relation_tail3))
                    .then(bo_continuation.or_not())
                    .map(|(first, bo_continuation)| BoGroupedBridiTailSyntax {
                        first: Box::new(first),
                        bo_continuation: bo_continuation.map(Box::new),
                    })
            });
            let bo_or_ke_continuation_start = predicate_tail_connective()
                .then(tense_modal_with_free_modifiers.clone().or_not())
                .then(choice((cmavo(Cmavo::Bo), cmavo(Cmavo::Ke))))
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
                        (((connective, free_modifiers), cu), bridi_tail),
                        (tail_terms, vau, tail_free_modifiers),
                    )| {
                        let connective =
                            append_connective_free_modifiers(connective, free_modifiers);
                        new!(BridiTailConnectionSyntax {
                            connective,
                            tense_modal: None,
                            cu: cu.map(Arc::new),
                            bridi_tail: Box::new(bridi_tail),
                            tail_terms,
                            vau: vau.map(Arc::new),
                            free_modifiers: tail_free_modifiers,
                        })
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
                .map(|(first, continuations)| AfterthoughtBridiTailSyntax {
                    first: Box::new(first),
                    continuations,
                });
            let ke_continuation = predicate_tail_connective()
                .then(tense_modal_with_free_modifiers.clone().or_not())
                .then(cmavo(Cmavo::Ke))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bridi_tail.clone())
                .then(
                    cmavo(Cmavo::Kehe)
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .or_not(),
                )
                .then(predicate_tail_terms.clone())
                .map(
                    |(
                        (((((connective, tense_modal), ke), ke_free_modifiers), bridi_tail), kehe),
                        (tail_terms, vau, free_modifiers),
                    )| {
                        new!(GroupedBridiTailConnectionSyntax {
                            connective,
                            tense_modal,
                            ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                            bridi_tail: Box::new(bridi_tail),
                            kehe: kehe.map(|(kehe, free_modifiers)| {
                                Arc::new(WithFreeModifiers::new(kehe, free_modifiers))
                            }),
                            tail_terms,
                            vau: vau.map(Arc::new),
                            free_modifiers,
                        })
                    },
                )
                .boxed();
            predicate_tail1.then(ke_continuation.or_not()).try_map(
                |(first, ke_continuation), span| {
                    if ke_continuation.as_ref().is_some_and(|ke_continuation| {
                        !predicate_tail_ke_continuation_allowed(&first, ke_continuation)
                    }) {
                        return Err(SyntaxParseError::custom(
                            span,
                            "bridi-tail KE continuation conflicts with trailing sumti connection"
                                .to_owned(),
                        ));
                    }
                    Ok(BridiTailSyntax {
                        first: Box::new(first),
                        ke_continuation: ke_continuation.map(Box::new),
                    })
                },
            )
        });
        let cu_terms_bridi_tail = term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(bridi_tail.clone())
            .map(|(terms, bridi_tail)| BridiTailSyntax {
                first: Box::new(AfterthoughtBridiTailSyntax {
                    first: Box::new(BoGroupedBridiTailSyntax {
                        first: Box::new(new!(SimpleBridiTailSyntax::TermPrefixedBridiTail {
                            terms,
                            bridi_tail: Box::new(bridi_tail),
                        })),
                        bo_continuation: None,
                    }),
                    continuations: Vec::new(),
                }),
                ke_continuation: None,
            })
            .boxed();
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
            .then(bridi_tail.clone())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(((leading_terms, cu), bridi_tail), free_modifiers)| {
                new!(BridiSyntax {
                    leading_terms,
                    cu: cu.map(|(cu, free_modifiers)| {
                        Arc::new(WithFreeModifiers::new(cu, free_modifiers))
                    }),
                    bridi_tail: Box::new(bridi_tail),
                    free_modifiers,
                })
            });

        let predicate_with_post_cu_terms = term
            .clone()
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(
                cu.clone()
                    .map_with(
                        |cu,
                         extra: &mut MapExtra<
                            'tokens,
                            '_,
                            ParserInput<'tokens>,
                            ParseExtra<'tokens>,
                        >| {
                            extra
                                .state()
                                .warn(ExperimentalConstruct::ExperimentalCuTermsSelbri, &cu);
                            cu
                        },
                    )
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>()),
            )
            .then(cu_terms_bridi_tail.clone())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(
                |(((leading_terms, (cu, cu_free_modifiers)), bridi_tail), free_modifiers)| {
                    new!(BridiSyntax {
                        leading_terms,
                        cu: Some(Arc::new(WithFreeModifiers::new(cu, cu_free_modifiers))),
                        bridi_tail: Box::new(bridi_tail),
                        free_modifiers,
                    })
                },
            );

        let relation_only = bridi_tail
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(bridi_tail, free_modifiers)| {
                new!(BridiSyntax {
                    leading_terms: Vec::new(),
                    cu: None,
                    bridi_tail: Box::new(bridi_tail),
                    free_modifiers,
                })
            });
        let bare_cu_predicate = cu
            .clone()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(bridi_tail.clone())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(((cu, cu_free_modifiers), bridi_tail), free_modifiers)| {
                new!(BridiSyntax {
                    leading_terms: Vec::new(),
                    cu: Some(Arc::new(WithFreeModifiers::new(cu, cu_free_modifiers))),
                    bridi_tail: Box::new(bridi_tail),
                    free_modifiers,
                })
            })
            .boxed();
        let bare_cu_terms_predicate =
            cu.clone()
                .map_with(
                    |cu,
                     extra: &mut MapExtra<
                        'tokens,
                        '_,
                        ParserInput<'tokens>,
                        ParseExtra<'tokens>,
                    >| {
                        extra
                            .state()
                            .warn(ExperimentalConstruct::ExperimentalCuTermsSelbri, &cu);
                        cu
                    },
                )
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(cu_terms_bridi_tail)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .map(|(((cu, cu_free_modifiers), bridi_tail), free_modifiers)| {
                    new!(BridiSyntax {
                        leading_terms: Vec::new(),
                        cu: Some(Arc::new(WithFreeModifiers::new(cu, cu_free_modifiers))),
                        bridi_tail: Box::new(bridi_tail),
                        free_modifiers,
                    })
                })
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
            .then(bridi_tail)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(((leading_terms, cu), bridi_tail), free_modifiers)| {
                new!(BridiSyntax {
                    leading_terms,
                    cu: cu.map(|(cu, free_modifiers)| {
                        Arc::new(WithFreeModifiers::new(cu, free_modifiers))
                    }),
                    bridi_tail: Box::new(bridi_tail),
                    free_modifiers,
                })
            });

        choice((
            forethought_predicate_with_leading_terms,
            predicate_with_leading_terms,
            predicate_with_post_cu_terms,
            bare_cu_predicate,
            bare_cu_terms_predicate,
            relation_only,
        ))
        .boxed()
    });
    let plain_subbridi = basic_predicate
        .clone()
        .map(|value| new!(SubbridiSyntax::Bridi(Box::new(value))));
    let prenex_subbridi = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo(Cmavo::Zohu))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_subbridi)| {
                new!(SubbridiSyntax::Prenex {
                    prenex_terms,
                    zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
                    inner_subbridi: Box::new(inner_subbridi),
                })
            },
        );
    subbridi.define(syntax_context(
        "subbridi",
        choice((prenex_subbridi, plain_subbridi)),
    ));
    let predicate_statement_bo_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo(Cmavo::Bo))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .map(
            |((((connective, tense_modal), bo), free_modifiers), trailing_subbridi)| {
                BridiStatementContinuationSyntax {
                    connective,
                    tense_modal,
                    marker: new!(BridiStatementContinuationMarkerSyntax::BoGrouped(
                        WithFreeModifiers::new(bo, free_modifiers,)
                    )),
                    trailing_subbridi: Box::new(trailing_subbridi),
                }
            },
        );
    let predicate_statement_ke_continuation = predicate_tail_connective()
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo(Cmavo::Ke))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .then(
            cmavo(Cmavo::Kehe)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((((connective, tense_modal), ke), ke_free_modifiers), trailing_subbridi), kehe)| {
                BridiStatementContinuationSyntax {
                    connective,
                    tense_modal,
                    marker: new!(BridiStatementContinuationMarkerSyntax::KeGrouped {
                        ke: WithFreeModifiers::new(ke, ke_free_modifiers),
                        kehe: kehe.map(|(kehe, free_modifiers)| {
                            WithFreeModifiers::new(kehe, free_modifiers)
                        }),
                    }),
                    trailing_subbridi: Box::new(trailing_subbridi),
                }
            },
        );
    let predicate_statement_continuation = choice((
        predicate_statement_bo_continuation,
        predicate_statement_ke_continuation,
    ));
    let bridi = basic_predicate
        .clone()
        .then(
            predicate_statement_continuation
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(bridi, continuations)| build_predicate_statement(bridi, continuations));

    let fragment_term = term.clone();

    let term_fragment = fragment_term
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(
            cmavo(Cmavo::Vau)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(terms, vau)| {
            statement_from_fragment(new!(FragmentSyntax::Terms {
                terms,
                vau: vau.map(|(vau, free_modifiers)| WithFreeModifiers::new(vau, free_modifiers)),
            }))
        });

    let relative_clause_fragment =
        relative_clauses(sumti.clone(), subbridi.clone(), free_modifier.clone()).map(
            |relative_clauses| {
                statement_from_fragment(new!(FragmentSyntax::RelativeClauses(relative_clauses)))
            },
        );
    let ek_fragment = ek_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            statement_from_fragment(new!(FragmentSyntax::Ek(append_connective_free_modifiers(
                connective,
                free_modifiers,
            ))))
        });
    let gihek_fragment = gihek_connective()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(connective, free_modifiers)| {
            statement_from_fragment(new!(FragmentSyntax::BridiTailConnective(
                append_connective_free_modifiers(connective, free_modifiers,)
            )))
        });

    let multiple_na_fragment = na_cmavo()
        .then(na_cmavo())
        .then(na_cmavo().repeated().collect::<Vec<_>>())
        .then(selmaho(Selmaho::Ja).rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((((first_na, second_na), rest_na), _), free_modifiers)| {
            let mut words = vec![first_na, second_na];
            words.extend(rest_na);
            statement_from_fragment(new!(FragmentSyntax::Other(WithFreeModifiers::new(
                words,
                free_modifiers,
            ))))
        });
    let single_na_fragment_blocker = choice((
        cmavo(Cmavo::Ku).ignored(),
        na_cmavo().ignored(),
        selmaho(Selmaho::Ja).ignored(),
        argument_connective().ignored(),
        predicate_tail_connective().ignored(),
    ));
    let single_na_fragment = na_cmavo()
        .then(single_na_fragment_blocker.rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((na, _), free_modifiers)| {
            statement_from_fragment(new!(FragmentSyntax::Other(WithFreeModifiers::new(
                vec![na],
                free_modifiers,
            ))))
        });

    let be_link_fragment = be_link_parser(sumti.clone(), free_modifier.clone()).map(|link| {
        let data!(LinkedSumtiListSyntax {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) = link.into_data();

        statement_from_fragment(new!(FragmentSyntax::LinkedSumti {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }))
    });
    let bei_link_fragment = bei_link_parser(sumti.clone(), free_modifier.clone())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|bei_only_links| {
            statement_from_fragment(new!(FragmentSyntax::LinkedSumtiContinuation(
                bei_only_links
            )))
        });

    let math_expression_fragment =
        quantifier_with_free_modifiers_boxed(quantifier_boxed(), free_modifier.clone()).map(
            |quantifier| {
                statement_from_fragment(new!(FragmentSyntax::Mekso(Box::new(new!(
                    MeksoSyntax::NumberMekso(quantifier)
                )))))
            },
        );

    let relation_fragment = selbri
        .clone()
        .map(|selbri| statement_from_fragment(new!(FragmentSyntax::Selbri(Box::new(selbri)))));

    let prenex_fragment = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo(Cmavo::Zohu))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((terms, zohu), zohu_free_modifiers)| {
            statement_from_fragment(new!(FragmentSyntax::Prenex {
                terms,
                zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
            }))
        });

    let prenex_statement = term
        .clone()
        .repeated()
        .collect::<Vec<_>>()
        .then(cmavo(Cmavo::Zohu))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(statement.clone())
        .map(
            |(((prenex_terms, zohu), zohu_free_modifiers), inner_statement)| {
                new!(StatementSyntax::Prenex {
                    prenex_terms,
                    zohu: WithFreeModifiers::new(zohu, zohu_free_modifiers),
                    inner_statement: Box::new(inner_statement),
                })
            },
        );
    let tuhe_statement = tense_modal_with_free_modifiers
        .clone()
        .or_not()
        .then(cmavo(Cmavo::Tuhe))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(
            cmavo(Cmavo::Tuhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((tense_modal, tuhe), tuhe_free_modifiers), text), tuhu)| {
                new!(StatementSyntax::TextGroup {
                    tense_modal,
                    tuhe: WithFreeModifiers::new(tuhe, tuhe_free_modifiers),
                    text: Box::new(text),
                    tuhu: tuhu
                        .map(|(tuhu, free_modifiers)| WithFreeModifiers::new(tuhu, free_modifiers)),
                })
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

    let simple_statement_after_i_connective = choice((bridi, tuhe_statement)).boxed();

    let simple_statement = choice((
        prenex_statement,
        simple_statement_after_i_connective.clone(),
    ));

    let pending_i_connective = cmavo(Cmavo::I)
        .then(statement_connective())
        .then(cmavo(Cmavo::I).rewind())
        .map(|((i, connective), _)| (i, connective))
        .boxed();
    let optional_statement_tag_bo = tense_modal_with_free_modifiers
        .clone()
        .or_not()
        .then(cmavo(Cmavo::Bo))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .or_not()
        .boxed();
    let chained_i_connective_statement_tail = pending_i_connective
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(
            cmavo(Cmavo::I)
                .then(statement_connective())
                .then(optional_statement_tag_bo.clone())
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
                        pending_words.extend(pending_connective.words());
                        (first_i, pending_words)
                    },
                );
                let connective = match tag_bo {
                    None => connective,
                    Some(((tense_modal, bo), free_modifiers)) => append_connective_free_modifiers(
                        append_optional_tense_modal_and_bo(connective, tense_modal, bo),
                        free_modifiers,
                    ),
                };
                let mut pending_words = pending_words;
                pending_words.push(i);
                let connective = prepend_connective_words(pending_words, connective);
                (
                    false,
                    first_i.expect("at least one pending i connective is parsed"),
                    connective,
                    trailing_statement,
                )
            },
        );
    let i_connective_statement_tail = cmavo(Cmavo::I)
        .then(statement_connective())
        .then(optional_statement_tag_bo.clone())
        .then(simple_statement_after_i_connective.clone())
        .map(|(((i, connective), tag_bo), trailing_statement)| {
            let connective = match tag_bo {
                None => connective,
                Some(((tense_modal, bo), free_modifiers)) => append_connective_free_modifiers(
                    append_optional_tense_modal_and_bo(connective, tense_modal, bo),
                    free_modifiers,
                ),
            };
            (false, i, connective, trailing_statement)
        });
    let i_bo_statement_tail = cmavo(Cmavo::I)
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(cmavo(Cmavo::Bo))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(simple_statement_after_i_connective.clone())
        .map(
            |((((i, tense_modal), bo), bo_free_modifiers), trailing_statement)| {
                let mut cmavo = Vec::new();
                if let Some(tense_modal) = tense_modal {
                    tense_modal.extend_words_into(&mut cmavo);
                }
                cmavo.push(bo);
                let connective =
                    connective_syntax(ConnectiveKind::Selbri, None, None, None, cmavo, None);
                (
                    false,
                    i,
                    append_connective_free_modifiers(connective, bo_free_modifiers),
                    trailing_statement,
                )
            },
        );
    let connected_statement_tail = choice((
        chained_i_connective_statement_tail,
        i_connective_statement_tail,
        i_bo_statement_tail,
        statement_connective()
            .then(optional_statement_tag_bo)
            .then(cmavo(Cmavo::I))
            .then(simple_statement_after_i_connective.clone())
            .map(|(((connective, tag_bo), i), trailing_statement)| {
                let connective = match tag_bo {
                    None => connective,
                    Some(((tense_modal, bo), free_modifiers)) => append_connective_free_modifiers(
                        append_optional_tense_modal_and_bo(connective, tense_modal, bo),
                        free_modifiers,
                    ),
                };
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
            cmavo(Cmavo::Iahu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(term.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(statement, iau_tail)| match iau_tail {
            Some(((iau, iau_free_modifiers), reset_terms)) => new!(StatementSyntax::Iau {
                inner_statement: Box::new(statement),
                iau: WithFreeModifiers::new(iau, iau_free_modifiers),
                reset_terms,
            }),
            None => statement,
        });

    statement.define(syntax_context("statement", iau_statement_body));
    free_modifier.define(syntax_context(
        "free modifier",
        choice((
            syntax_label(
                "replacement phrase",
                replacement_free(free_modifier.clone()),
            ),
            syntax_label("utterance ordinal", mai_free(free_modifier.clone())),
            syntax_label("subscript", xi_free(free_modifier.clone())),
            syntax_label(
                "metalinguistic comment",
                sei_free(term.clone(), selbri.clone(), free_modifier.clone()),
            ),
            syntax_label("reciprocal", soi_free(sumti.clone(), free_modifier.clone())),
            syntax_label(
                "parenthetical text",
                to_free(text.clone(), free_modifier.clone()),
            ),
            syntax_label(
                "vocative phrase",
                vocative_free(
                    sumti.clone(),
                    selbri.clone(),
                    subbridi.clone(),
                    free_modifier.clone(),
                ),
            ),
        )),
    ));

    let paragraph_statement_body = choice((statement.clone(), fragment_statement.clone())).boxed();
    let initial_statement = paragraph_statement_body.clone().map(|statement| {
        new!(ParagraphStatementSyntax {
            i: None,
            connective: None,
            free_modifiers: Vec::new(),
            statement: Some(Box::new(statement)),
        })
    });

    let i_connective_tag_bo = standard_statement_connective()
        .or_not()
        .then(
            tense_modal_with_free_modifiers
                .clone()
                .or_not()
                .then(cmavo(Cmavo::Bo))
                .or_not(),
        )
        .map(|(connective, tag_bo)| match (connective, tag_bo) {
            (None, None) => None,
            (Some(connective), None) => Some(connective),
            (connective, Some((tense_modal, bo))) => {
                let ConnectiveSyntaxParts {
                    kind,
                    se,
                    nahe,
                    na,
                    mut cmavo,
                    nai,
                } = connective.map_or(
                    ConnectiveSyntaxParts {
                        kind: ConnectiveKind::Selbri,
                        se: None,
                        nahe: None,
                        na: None,
                        cmavo: wrapped_words(Vec::new(), Vec::new()),
                        nai: None,
                    },
                    |connective| connective.into_parts(),
                );
                if let Some(tense_modal) = tense_modal {
                    tense_modal.extend_words_into(&mut cmavo.value);
                }
                cmavo.value.push(bo);
                Some(ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai))
            }
        });

    let leading_i_statement = cmavo(Cmavo::I)
        .then(i_connective_tag_bo.clone())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |((i, connective), free_modifiers)| LeadingIStatementSyntax {
                i,
                connective,
                free_modifiers,
            },
        );

    let following_statement = cmavo(Cmavo::I)
        .then_ignore(statement_connective().rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(paragraph_statement_body.or_not())
        .map(|((i, free_modifiers), statement)| {
            new!(ParagraphStatementSyntax {
                i: Some(i),
                connective: None,
                free_modifiers,
                statement: statement.map(Box::new),
            })
        });
    let trailing_ijek_statement =
        cmavo(Cmavo::I)
            .then(statement_connective())
            .map(|(i, connective)| {
                new!(ParagraphStatementSyntax {
                    i: None,
                    connective: None,
                    free_modifiers: Vec::new(),
                    statement: Some(Box::new(statement_from_fragment(new!(
                        FragmentSyntax::BridiConnective { i, connective }
                    )))),
                })
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
    let paragraph_with_niho = selmaho(Selmaho::Niho)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(paragraph.clone().or_not())
        .map(|((niho, free_modifiers), paragraph)| match paragraph {
            Some(paragraph) => {
                let mut paragraph_data = paragraph.into_data();
                if paragraph_data.niho.is_empty() {
                    paragraph_data.niho = niho;
                }
                if paragraph_data.free_modifiers.is_empty() {
                    paragraph_data.free_modifiers = free_modifiers;
                }
                ParagraphSyntax::from_data(paragraph_data)
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

    let leading_cmevla = if dialect.cbm_enabled {
        empty().map(|_| Vec::new()).boxed()
    } else {
        cmevla_word().repeated().collect::<Vec<_>>().boxed()
    };
    let text_body = cmavo(Cmavo::Nai)
        .repeated()
        .collect::<Vec<_>>()
        .then(leading_cmevla)
        .then(leading_indicator().repeated().collect::<Vec<_>>())
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(
            modal_forethought_connective()
                .rewind()
                .not()
                .ignore_then(text_leading_connective())
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
                let text = new!(TextSyntax {
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
                    .fold(text, |text, leading_i_statement| {
                        prepend_i_with_free_modifier(leading_i_statement, text)
                    })
            },
        );

    text.define(syntax_context("text", text_body));
    text.then_ignore(end()).boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_context<'tokens, O: 'tokens>(
    construct: &'static str,
    parser: impl Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
) -> BoxedParser<'tokens, O> {
    trace_enter(construct)
        .ignore_then(
            parser
                .labelled(construct)
                .as_context()
                .map_with(move |output, extra| {
                    let span: Span = extra.span();
                    let byte_start = span.start.min(span.end);
                    let byte_end = span.start.max(span.end);
                    extra.state().trace_exit_construct(
                        TraceLevel::Top,
                        TraceEventKind::ConstructSuccess,
                        construct,
                        byte_start,
                        byte_end,
                        || None,
                    );
                    output
                })
                .map_err_with_state(move |error, span: Span, state| {
                    let byte_start = span.start.min(span.end);
                    let byte_end = span.start.max(span.end);
                    state.trace_exit_construct(
                        TraceLevel::Top,
                        TraceEventKind::ConstructFailure,
                        construct,
                        byte_start,
                        byte_end,
                        || None,
                    );
                    error
                }),
        )
        .boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn syntax_label<'tokens, O: 'tokens>(
    construct: &'static str,
    parser: impl Parser<'tokens, ParserInput<'tokens>, O, ParseExtra<'tokens>> + 'tokens,
) -> BoxedParser<'tokens, O> {
    parser.labelled(construct).boxed()
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn trace_enter<'tokens>(construct: &'static str) -> BoxedParser<'tokens, ()> {
    custom::<_, ParserInput<'tokens>, (), ParseExtra<'tokens>>(move |input| {
        if input
            .state()
            .trace_should_record(TraceLevel::Top, construct)
        {
            input
                .state()
                .trace_enter_construct(TraceLevel::Top, construct, 0, 0);
        }
        Ok(())
    })
    .boxed()
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn syntax_grammar_ebnf(options: &ParseOptions) -> String {
    statement_parser(None, options).debug().to_ebnf()
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn syntax_grammar_svg(options: &ParseOptions) -> String {
    statement_parser(None, options)
        .debug()
        .to_railroad_svg()
        .to_string()
}

#[requires(true)]
#[ensures(true)]
fn prepend_i_with_free_modifier(marker: LeadingIStatementSyntax, text: TextSyntax) -> TextSyntax {
    let mut text_data = text.into_data();
    if text_data.paragraphs.is_empty() {
        text_data.paragraphs.push(new!(ParagraphSyntax {
            i: None,
            niho: Vec::new(),
            free_modifiers: Vec::new(),
            statements: vec![new!(ParagraphStatementSyntax {
                i: Some(marker.i),
                connective: marker.connective.map(Box::new),
                free_modifiers: marker.free_modifiers,
                statement: None,
            })],
        }));
        return TextSyntax::from_data(text_data);
    }

    let mut paragraph_data = text_data.paragraphs.remove(0).into_data();
    if paragraph_data.niho.is_empty() {
        paragraph_data.statements = prepend_i_to_niho_free_paragraph_statements(
            marker,
            std::mem::take(&mut paragraph_data.statements),
        );
    } else {
        paragraph_data.i = Some(marker.i);
        paragraph_data.statements = attach_i_connective_to_niho_paragraph_statements(
            marker.connective,
            marker.free_modifiers,
            std::mem::take(&mut paragraph_data.statements),
        );
    }
    text_data
        .paragraphs
        .insert(0, ParagraphSyntax::from_data(paragraph_data));
    TextSyntax::from_data(text_data)
}

#[requires(true)]
#[ensures(true)]
fn prepend_i_to_niho_free_paragraph_statements(
    marker: LeadingIStatementSyntax,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let LeadingIStatementSyntax {
        i,
        connective,
        free_modifiers,
    } = marker;
    if statements.is_empty() {
        return vec![new!(ParagraphStatementSyntax {
            i: Some(i),
            connective: connective.map(Box::new),
            free_modifiers,
            statement: None,
        })];
    };
    if statements.first().is_some_and(|first| first.i.is_some()) {
        let new_statement = new!(ParagraphStatementSyntax {
            i: Some(i),
            connective: connective.map(Box::new),
            free_modifiers,
            statement: None,
        });
        statements.insert(0, new_statement);
        return statements;
    }

    let mut first_data = statements.remove(0).into_data();
    first_data.i = Some(i);
    first_data.connective = connective.map(Box::new);
    first_data.free_modifiers = free_modifiers;
    statements.insert(0, ParagraphStatementSyntax::from_data(first_data));
    statements
}

#[requires(true)]
#[ensures(true)]
fn attach_i_connective_to_niho_paragraph_statements(
    connective: Option<ConnectiveSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    if statements.is_empty() {
        return vec![new!(ParagraphStatementSyntax {
            i: None,
            connective: connective.map(Box::new),
            free_modifiers,
            statement: None,
        })];
    };
    let mut first_data = statements.remove(0).into_data();
    first_data.connective = connective.map(Box::new);
    let mut combined_free_modifiers = free_modifiers;
    combined_free_modifiers.append(&mut first_data.free_modifiers);
    first_data.free_modifiers = combined_free_modifiers;
    statements.insert(0, ParagraphStatementSyntax::from_data(first_data));
    statements
}

#[requires(true)]
#[ensures(true)]
fn build_paragraph(
    i: Option<Token>,
    niho: Vec<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
    statements: Vec<ParagraphStatementSyntax>,
) -> ParagraphSyntax {
    new!(ParagraphSyntax {
        i,
        niho,
        free_modifiers,
        statements: normalize_trailing_ijek_fragment(statements),
    })
}

#[requires(true)]
#[ensures(true)]
fn normalize_trailing_ijek_fragment(
    mut statements: Vec<ParagraphStatementSyntax>,
) -> Vec<ParagraphStatementSyntax> {
    let Some(last) = statements.pop() else {
        return statements;
    };
    match last.into_data() {
        data!(ParagraphStatementSyntax {
            i: Some(i),
            connective: Some(connective),
            free_modifiers,
            statement: None,
        }) if free_modifiers.is_empty() => {
            statements.push(new!(ParagraphStatementSyntax {
                i: None,
                connective: None,
                free_modifiers: Vec::new(),
                statement: Some(Box::new(statement_from_fragment(new!(
                    FragmentSyntax::BridiConnective {
                        i,
                        connective: *connective
                    }
                )))),
            }));
            statements
        }
        other => {
            statements.push(ParagraphStatementSyntax::from_data(other));
            statements
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn build_predicate_statement(
    bridi: BridiSyntax,
    continuations: Vec<BridiStatementContinuationSyntax>,
) -> StatementSyntax {
    continuations.into_iter().fold(
        new!(StatementSyntax::Bridi(Box::new(bridi))),
        |leading_statement, continuation| {
            new!(StatementSyntax::ExperimentalBridiContinuation {
                leading_statement: Box::new(leading_statement),
                continuation,
            })
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn statement_from_fragment(fragment: FragmentSyntax) -> StatementSyntax {
    new!(StatementSyntax::Fragment(Box::new(fragment)))
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.word_count() >= old(leading_statement.word_count()))]
fn build_connected_statement(
    leading_statement: StatementSyntax,
    continuations: Vec<(bool, Token, ConnectiveSyntax, StatementSyntax)>,
) -> StatementSyntax {
    let mut statements = vec![leading_statement];
    let mut connectors = Vec::new();
    for (pre_i, i, connective, trailing_statement) in continuations {
        connectors.push((pre_i, i, connective));
        statements.push(trailing_statement);
    }

    let mut right_statement = statements
        .pop()
        .expect("there is always at least the leading statement");
    let mut pending_non_bo = Vec::new();
    while let Some((pre_i, i, connective)) = connectors.pop() {
        let left_statement = statements
            .pop()
            .expect("connectors are paired with a leading statement");
        if connective_has_bo(&connective) {
            right_statement =
                connected_statement_node(pre_i, i, connective, left_statement, right_statement);
        } else {
            pending_non_bo.push((pre_i, i, connective, right_statement));
            right_statement = left_statement;
        }
    }

    let mut connected_statement = right_statement;
    for (pre_i, i, connective, trailing_statement) in pending_non_bo.into_iter().rev() {
        connected_statement = connected_statement_node(
            pre_i,
            i,
            connective,
            connected_statement,
            trailing_statement,
        );
    }
    connected_statement
}

#[requires(true)]
#[ensures(true)]
fn connected_statement_node(
    pre_i: bool,
    i: Token,
    connective: ConnectiveSyntax,
    leading_statement: StatementSyntax,
    trailing_statement: StatementSyntax,
) -> StatementSyntax {
    if pre_i {
        new!(StatementSyntax::PreposedIStatementConnection {
            connective,
            i,
            leading_statement: Box::new(leading_statement),
            trailing_statement: Box::new(trailing_statement),
        })
    } else {
        new!(StatementSyntax::StatementConnection {
            i,
            connective,
            leading_statement: Box::new(leading_statement),
            trailing_statement: Box::new(trailing_statement),
        })
    }
}

#[requires(true)]
#[ensures(ret == connective.cmavo().value.iter().any(|word| word.is_cmavo(Cmavo::Bo)))]
fn connective_has_bo(connective: &ConnectiveSyntax) -> bool {
    connective
        .cmavo()
        .value
        .iter()
        .any(|word| word.is_cmavo(Cmavo::Bo))
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_ke_continuation_allowed(
    first: &AfterthoughtBridiTailSyntax,
    ke_continuation: &GroupedBridiTailConnectionSyntax,
) -> bool {
    !predicate_tail1_has_tail_terms(first) || connective_is_gihek(&ke_continuation.connective)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail1_has_tail_terms(bridi_tail: &AfterthoughtBridiTailSyntax) -> bool {
    predicate_tail2_has_tail_terms(&bridi_tail.first)
        || bridi_tail.continuations.iter().any(|continuation| {
            !continuation.tail_terms.is_empty()
                || predicate_tail2_has_tail_terms(&continuation.bridi_tail)
        })
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2_has_tail_terms(bridi_tail: &BoGroupedBridiTailSyntax) -> bool {
    predicate_tail3_has_tail_terms(&bridi_tail.first)
        || bridi_tail
            .bo_continuation
            .as_ref()
            .is_some_and(|continuation| {
                !continuation.tail_terms.is_empty()
                    || predicate_tail2_has_tail_terms(&continuation.bridi_tail)
            })
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail3_has_tail_terms(bridi_tail: &SimpleBridiTailSyntax) -> bool {
    match bridi_tail.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { terms, .. }) => !terms.is_empty(),
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
            forethought_connection
        )) => gek_sentence_has_tail_terms(forethought_connection),
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { .. }) => true,
    }
}

#[requires(true)]
#[ensures(true)]
fn gek_sentence_has_tail_terms(forethought_connection: &ForethoughtBridiConnectionSyntax) -> bool {
    match forethought_connection.as_data() {
        data!(ForethoughtBridiConnectionSyntax::BridiConnection { tail_terms, .. }) => {
            !tail_terms.is_empty()
        }
        data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection { inner, .. })
        | data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => {
            gek_sentence_has_tail_terms(inner)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_is_gihek(connective: &ConnectiveSyntax) -> bool {
    connective
        .cmavo()
        .value
        .iter()
        .any(|word| word.is_selmaho(Selmaho::Giha))
}

#[requires(true)]
#[ensures(true)]
fn empty_text() -> TextSyntax {
    new!(TextSyntax {
        leading_nai: Vec::new(),
        leading_cmevla: Vec::new(),
        leading_indicators: Vec::new(),
        leading_free_modifiers: Vec::new(),
        leading_connective: None,
        paragraphs: Vec::new(),
    })
}

#[requires(true)]
#[ensures(true)]
fn sei_free<'tokens, T, R, F>(
    term: T,
    selbri: R,
    free_modifier: F,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let prohibited_free_modifier = cll_prohibited_free_modifier(free_modifier.clone());
    selmaho(Selmaho::Sei)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(term.repeated().collect::<Vec<_>>())
        .then(
            cmavo(Cmavo::Cu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .then(selbri)
        .then(
            cmavo(Cmavo::Sehu)
                .then(
                    prohibited_free_modifier
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(
            |(((((sei, leading_free_modifiers), terms), cu), selbri), sehu)| {
                new!(FreeModifierSyntax::MetalinguisticBridi {
                    sei: WithFreeModifiers::new(sei, leading_free_modifiers),
                    terms,
                    cu: cu.map(|(cu, free_modifiers)| WithFreeModifiers::new(cu, free_modifiers)),
                    selbri: Box::new(selbri),
                    sehu: sehu
                        .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
                })
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
    let empty_parenthetical = selmaho(Selmaho::To)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(cmavo(Cmavo::Toi))
        .then(
            prohibited_free_modifier
                .clone()
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(move |(((to, free_modifiers), toi), toi_free_modifiers)| {
            new!(FreeModifierSyntax::ParentheticalText {
                to: WithFreeModifiers::new(to, free_modifiers),
                text: Box::new(empty_text()),
                toi: Some(WithFreeModifiers::new(toi, toi_free_modifiers)),
            })
        });

    let nonempty_parenthetical = selmaho(Selmaho::To)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(
            cmavo(Cmavo::Toi)
                .then(
                    prohibited_free_modifier
                        .clone()
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(|(((to, free_modifiers), text), toi)| {
            new!(FreeModifierSyntax::ParentheticalText {
                to: WithFreeModifiers::new(to, free_modifiers),
                text: Box::new(text),
                toi: toi.map(|(toi, free_modifiers)| WithFreeModifiers::new(toi, free_modifiers)),
            })
        });

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
    let full_replacement = cmavo(Cmavo::Lohai)
        .then(raw_words_until(&[Cmavo::Sahai, Cmavo::Lehai]))
        .then(cmavo(Cmavo::Sahai).or_not())
        .then(raw_words_until(&[Cmavo::Lehai]))
        .then(cmavo(Cmavo::Lehai))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(
            |(((((lohai, old_words), sahai), new_words), lehai), free_modifiers)| {
                new!(FreeModifierSyntax::TextReplacement {
                    lohai: Some(lohai),
                    old_words,
                    sahai,
                    new_words,
                    lehai: WithFreeModifiers::new(lehai, free_modifiers),
                })
            },
        );
    let new_only_replacement = cmavo(Cmavo::Sahai)
        .then(raw_words_until(&[Cmavo::Lehai]))
        .then(cmavo(Cmavo::Lehai))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(((sahai, new_words), lehai), free_modifiers)| {
            new!(FreeModifierSyntax::TextReplacement {
                lohai: None,
                old_words: Vec::new(),
                sahai: Some(sahai),
                new_words,
                lehai: WithFreeModifiers::new(lehai, free_modifiers),
            })
        });
    let close_only_replacement = cmavo(Cmavo::Lehai)
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(lehai, free_modifiers)| {
            new!(FreeModifierSyntax::TextReplacement {
                lohai: None,
                old_words: Vec::new(),
                sahai: None,
                new_words: Vec::new(),
                lehai: WithFreeModifiers::new(lehai, free_modifiers),
            })
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
fn raw_words_until<'tokens>(terminators: &'static [Cmavo]) -> BoxedParser<'tokens, Vec<Token>> {
    token_matching(
        "replacement word",
        "REPLACEMENT WORD",
        vec![new!(SyntaxExpectedToken::WordCategory(
            SyntaxWordCategory::ReplacementWord,
        ))],
        move |word| !word.is_one_of_cmavo(terminators),
    )
    .repeated()
    .collect::<Vec<_>>()
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body<'tokens>() -> BoxedParser<'tokens, MeksoSyntax> {
    math_parser_pair().0
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context<'tokens, A, R, F>(
    sumti: A,
    selbri: R,
    free_modifier: F,
) -> BoxedParser<'tokens, MeksoSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    syntax_label(
        "mex",
        math_parser_pair_with_context(sumti, selbri, free_modifier).0,
    )
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair_with_context<'tokens, A, R, F>(
    sumti: A,
    selbri: R,
    free_modifier: F,
) -> (
    BoxedParser<'tokens, MeksoSyntax>,
    BoxedParser<'tokens, MeksoOperatorSyntax>,
)
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let mut expression = Recursive::declare();
    let mut operator = Recursive::declare();
    expression.define(math_expression_body_with_context_inner(
        expression.clone(),
        operator.clone(),
        sumti.clone(),
        selbri.clone(),
        free_modifier,
    ));
    operator.define(math_operator_with_context(
        expression.clone(),
        operator.clone(),
        selbri,
    ));
    (expression.boxed(), operator.boxed())
}

#[requires(true)]
#[ensures(true)]
fn math_expression_body_with_context_inner<'tokens, E, O, A, R, F>(
    expression: E,
    operator: O,
    sumti: A,
    selbri: R,
    free_modifier: F,
) -> BoxedParser<'tokens, MeksoSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MeksoSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MeksoOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number =
        quantifier_with_free_modifiers_boxed(number_quantifier_boxed(), free_modifier.clone())
            .map(|value| new!(MeksoSyntax::NumberMekso(value)));
    let letter = letter_string()
        .then_ignore(selmaho(Selmaho::Moi).rewind().not())
        .then(cmavo(Cmavo::Boi).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((letter, boi), free_modifiers)| {
            new!(MeksoSyntax::LerfuStringMekso {
                letter: WithFreeModifiers::new(
                    word_run(letter),
                    if boi.is_some() {
                        Vec::new()
                    } else {
                        free_modifiers.clone()
                    },
                ),
                boi: boi.map(|boi| WithFreeModifiers::new(boi, free_modifiers)),
            })
        });
    let nihe = cmavo(Cmavo::Nihe)
        .then(selbri.clone())
        .then(cmavo(Cmavo::Tehu).or_not())
        .map(|((nihe, selbri), tehu)| {
            new!(MeksoSyntax::SelbriOperand {
                nihe: WithFreeModifiers::new(nihe, Vec::new()),
                selbri: Box::new(selbri),
                tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, Vec::new())),
            })
        });
    let mohe = cmavo(Cmavo::Mohe)
        .then(sumti)
        .then(cmavo(Cmavo::Tehu).or_not())
        .map(|((mohe, sumti), tehu)| {
            new!(MeksoSyntax::SumtiOperand {
                mohe: WithFreeModifiers::new(mohe, Vec::new()),
                sumti: Box::new(sumti),
                tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, Vec::new())),
            })
        });
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo(Cmavo::Johi)
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo(Cmavo::Tehu).or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                new!(MeksoSyntax::MeksoArray {
                    johi: WithFreeModifiers::new(johi, free_modifiers),
                    expressions: math_expression_vec(expressions),
                    tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, tehu_free_modifiers)),
                })
            },
        );
    let vei = cmavo(Cmavo::Vei)
        .then(expression.clone())
        .then(cmavo(Cmavo::Veho).or_not())
        .map(|((vei, inner_expression), veho)| {
            new!(MeksoSyntax::ParenthesizedMekso {
                vei: WithFreeModifiers::new(vei, Vec::new()),
                inner_expression: Box::new(inner_expression),
                veho: veho.map(|veho| WithFreeModifiers::new(veho, Vec::new())),
            })
        });
    let gek = modal_forethought_connective_with_free_modifiers(free_modifier.clone())
        .then(expression.clone())
        .then(gik_connective_with_free_modifiers(free_modifier.clone()))
        .then(expression)
        .map(|(((gek, left_expression), gik), right_expression)| {
            new!(MeksoSyntax::ForethoughtMeksoConnection {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            })
        });
    let math_operand_atom = choice((gek, vei, nihe, mohe, johi, number, letter)).boxed();
    let math_operand = recursive(|math_operand| {
        let math_operand2 = recursive(|math_operand2| {
            math_operand_atom
                .clone()
                .then(
                    operand_connective()
                        .then(tense_modal().or_not())
                        .then(cmavo(Cmavo::Bo))
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .then(math_operand2)
                        .or_not(),
                )
                .map(|(left_expression, bo_tail)| match bo_tail {
                    None => left_expression,
                    Some(((((connective, tense_modal), bo), free_modifiers), right_expression)) => {
                        let connective = match tense_modal {
                            None => connective,
                            Some(tag) => append_tense_modal_words(connective, tag),
                        };
                        let connective =
                            append_connective_free_modifiers(connective, free_modifiers);
                        let connective = append_connective_words(connective, vec![bo]);
                        new!(MeksoSyntax::MeksoConnection {
                            left_expression: Box::new(left_expression),
                            connective,
                            right_expression: Box::new(right_expression),
                        })
                    }
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
                        new!(MeksoSyntax::MeksoConnection {
                            left_expression: Box::new(left_expression),
                            connective,
                            right_expression: Box::new(right_expression),
                        })
                    },
                )
            });
        math_operand1
            .clone()
            .then(
                operand_connective()
                    .then(tense_modal().or_not())
                    .then(cmavo(Cmavo::Ke))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(math_operand)
                    .then(cmavo(Cmavo::Kehe).or_not())
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(left_expression, grouped_tail)| match grouped_tail {
                None => left_expression,
                Some((
                    (
                        ((((connective, tense_modal), ke), ke_free_modifiers), right_expression),
                        kehe,
                    ),
                    kehe_free_modifiers,
                )) => {
                    let connective = match tense_modal {
                        None => connective,
                        Some(tag) => append_tense_modal_words(connective, tag),
                    };
                    new!(MeksoSyntax::MeksoConnection {
                        left_expression: Box::new(left_expression),
                        connective,
                        right_expression: Box::new(new!(MeksoSyntax::ParenthesizedMekso {
                            vei: WithFreeModifiers::new(ke, ke_free_modifiers),
                            inner_expression: Box::new(right_expression),
                            veho: kehe
                                .map(|kehe| WithFreeModifiers::new(kehe, kehe_free_modifiers)),
                        })),
                    })
                }
            })
            .boxed()
    });
    let math_expression2 = recursive(|math_expression2| {
        let lahe = selmaho(Selmaho::Nahe)
            .then(cmavo(Cmavo::Bo))
            .then(math_expression2.clone())
            .then(cmavo(Cmavo::Luhu).or_not())
            .map(|(((nahe, bo), inner_expression), luhu)| {
                new!(MeksoSyntax::QualifiedOperand {
                    markers: WithFreeModifiers::new(vec![nahe, bo], Vec::new()),
                    inner_expression: Box::new(inner_expression),
                    luhu: luhu.map(|luhu| WithFreeModifiers::new(luhu, Vec::new())),
                })
            });
        let forethought = cmavo(Cmavo::Peho)
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo(Cmavo::Kuhe).or_not())
            .map(|(((peho, operator), operands), kuhe)| {
                new!(MeksoSyntax::ForethoughtCall {
                    peho: peho.map(|peho| WithFreeModifiers::new(peho, Vec::new())),
                    operator: Box::new(operator),
                    operands,
                    kuhe: kuhe.map(|kuhe| WithFreeModifiers::new(kuhe, Vec::new())),
                })
            });
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
        cmavo(Cmavo::Fuha)
            .then(reverse_polish_parts)
            .map(|(fuha, (operands, operators))| {
                new!(MeksoSyntax::ReversePolish {
                    fuha: WithFreeModifiers::new(fuha, Vec::new()),
                    operands,
                    operators,
                })
            });
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo(Cmavo::Bihe)
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => new!(MeksoSyntax::PrecedenceInfix {
                    left_expression: Box::new(left_expression),
                    bihe: WithFreeModifiers::new(bihe, Vec::new()),
                    operator: Box::new(operator),
                    right_expression: Box::new(right_expression),
                }),
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
                |left_expression, (operator, right_expression)| {
                    new!(MeksoSyntax::Infix {
                        operator: Box::new(operator),
                        left_expression: Box::new(left_expression),
                        right_expression: Box::new(right_expression),
                    })
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_parser_pair<'tokens>() -> (
    BoxedParser<'tokens, MeksoSyntax>,
    BoxedParser<'tokens, MeksoOperatorSyntax>,
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
) -> BoxedParser<'tokens, MeksoSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MeksoSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MeksoOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let number = number_quantifier_boxed().map(|value| new!(MeksoSyntax::NumberMekso(value)));
    let letter = letter_string()
        .then_ignore(selmaho(Selmaho::Moi).rewind().not())
        .then(cmavo(Cmavo::Boi).or_not())
        .map(|(letter, boi)| {
            new!(MeksoSyntax::LerfuStringMekso {
                letter: WithFreeModifiers::new(word_run(letter), Vec::new()),
                boi: boi.map(|boi| WithFreeModifiers::new(boi, Vec::new())),
            })
        });
    let vei = cmavo(Cmavo::Vei)
        .then(expression.clone())
        .then(cmavo(Cmavo::Veho).or_not())
        .map(|((vei, inner_expression), veho)| {
            new!(MeksoSyntax::ParenthesizedMekso {
                vei: WithFreeModifiers::new(vei, Vec::new()),
                inner_expression: Box::new(inner_expression),
                veho: veho.map(|veho| WithFreeModifiers::new(veho, Vec::new())),
            })
        });
    let no_free_modifiers = empty().to(Vec::<FreeModifierSyntax>::new());
    let johi = cmavo(Cmavo::Johi)
        .then(no_free_modifiers.clone())
        .then(
            expression
                .clone()
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(cmavo(Cmavo::Tehu).or_not())
        .then(no_free_modifiers)
        .map(
            |((((johi, free_modifiers), expressions), tehu), tehu_free_modifiers)| {
                new!(MeksoSyntax::MeksoArray {
                    johi: WithFreeModifiers::new(johi, free_modifiers),
                    expressions: math_expression_vec(expressions),
                    tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, tehu_free_modifiers)),
                })
            },
        );
    let gek = modal_forethought_connective()
        .then(expression.clone())
        .then(gik_connective())
        .then(expression)
        .map(|(((gek, left_expression), gik), right_expression)| {
            new!(MeksoSyntax::ForethoughtMeksoConnection {
                gek,
                left_expression: Box::new(left_expression),
                gik,
                right_expression: Box::new(right_expression),
            })
        });
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
                |left_expression, (connective, right_expression)| {
                    new!(MeksoSyntax::MeksoConnection {
                        left_expression: Box::new(left_expression),
                        connective,
                        right_expression: Box::new(right_expression),
                    })
                },
            )
        })
        .boxed();
    let math_expression2 = recursive(|math_expression2| {
        let forethought = cmavo(Cmavo::Peho)
            .or_not()
            .then(operator.clone())
            .then(
                math_expression2
                    .clone()
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(cmavo(Cmavo::Kuhe).or_not())
            .map(|(((peho, operator), operands), kuhe)| {
                new!(MeksoSyntax::ForethoughtCall {
                    peho: peho.map(|peho| WithFreeModifiers::new(peho, Vec::new())),
                    operator: Box::new(operator),
                    operands,
                    kuhe: kuhe.map(|kuhe| WithFreeModifiers::new(kuhe, Vec::new())),
                })
            });
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
        cmavo(Cmavo::Fuha)
            .then(reverse_polish_parts)
            .map(|(fuha, (operands, operators))| {
                new!(MeksoSyntax::ReversePolish {
                    fuha: WithFreeModifiers::new(fuha, Vec::new()),
                    operands,
                    operators,
                })
            });
    let math_expression1 = recursive(|math_expression1| {
        math_expression2
            .clone()
            .then(
                cmavo(Cmavo::Bihe)
                    .then(operator.clone())
                    .then(math_expression1)
                    .or_not(),
            )
            .map(|(left_expression, bihe_tail)| match bihe_tail {
                None => left_expression,
                Some(((bihe, operator), right_expression)) => new!(MeksoSyntax::PrecedenceInfix {
                    left_expression: Box::new(left_expression),
                    bihe: WithFreeModifiers::new(bihe, Vec::new()),
                    operator: Box::new(operator),
                    right_expression: Box::new(right_expression),
                }),
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
                |left_expression, (operator, right_expression)| {
                    new!(MeksoSyntax::Infix {
                        operator: Box::new(operator),
                        left_expression: Box::new(left_expression),
                        right_expression: Box::new(right_expression),
                    })
                },
            )
        })
        .boxed();

    choice((infix_expression, reverse_polish)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_tail_with<'tokens, B, A, R, S, F>(
    leading_sumti: B,
    sumti: A,
    selbri: R,
    subbridi: S,
    free_modifier: F,
) -> BoxedParser<
    'tokens,
    (
        Vec<DescriptionTailElementSyntax>,
        Option<Box<SelbriSyntax>>,
        Vec<RelativeClauseSyntax>,
    ),
>
where
    B: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let tail_argument =
        pa_word()
            .rewind()
            .not()
            .ignore_then(leading_sumti)
            .map(|sumti| match sumti.into_data() {
                data!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti,
                    vuho: _,
                    relative_clauses,
                }) => vec![
                    new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                        base_sumti
                    )),
                    new!(
                        DescriptionTailElementSyntax::DescriptionTailRelativeClauses(
                            relative_clauses
                        )
                    ),
                ],
                sumti => vec![new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                    Box::new(SumtiSyntax::from_data(sumti),)
                ))],
            });
    let contextual_quantifier = quantifier_with_free_modifiers_boxed(
        quantifier_with_context_boxed(sumti.clone(), selbri.clone(), free_modifier.clone()),
        free_modifier.clone(),
    );
    let descriptor_relative_clauses =
        relative_clauses(sumti.clone(), subbridi, free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);

    let leading_tail_elements = tail_argument
        .or_not()
        .then(descriptor_relative_clauses.clone())
        .map(|(sumti, relative_clauses)| {
            let mut tail_elements = sumti.into_iter().flatten().collect::<Vec<_>>();
            if !relative_clauses.is_empty() {
                tail_elements.push(new!(
                    DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
                ));
            }
            tail_elements
        });

    let relation_tail = selbri
        .clone()
        .then(descriptor_relative_clauses.clone())
        .map(|(selbri, relative_clauses)| (Vec::new(), Some(Box::new(selbri)), relative_clauses));
    let quantifier_relation_tail = contextual_quantifier
        .clone()
        .then(selmaho(Selmaho::Roi).rewind().not())
        .map(|(quantifier, _)| quantifier)
        .map(|quantifier| {
            new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                *quantifier
            ))
        })
        .then(selbri.clone())
        .then(descriptor_relative_clauses.clone())
        .map(|((quantifier, selbri), relative_clauses)| {
            (vec![quantifier], Some(Box::new(selbri)), relative_clauses)
        });
    let quantifier_argument_tail = contextual_quantifier
        .map(|quantifier| {
            new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                *quantifier
            ))
        })
        .then(sumti)
        .map(|(quantifier, sumti)| {
            (
                vec![
                    quantifier,
                    new!(DescriptionTailElementSyntax::DescriptionTailSumti(
                        Box::new(sumti)
                    )),
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
            |(mut leading_tail_elements, (tail_elements, selbri, relative_clauses))| {
                leading_tail_elements.extend(tail_elements);
                (leading_tail_elements, selbri, relative_clauses)
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_parser_with<'tokens, A, R, S, T, F>(
    sumti: A,
    selbri: R,
    subbridi: impl Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    single_term: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, SumtiSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, TermSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let quote = syntax_label("quote", quote_argument(source, text, free_modifier.clone()));

    let mekso = syntax_context(
        "number sumti",
        selmaho(Selmaho::Li)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(math_expression_body_with_context(
                sumti.clone(),
                selbri.clone(),
                free_modifier.clone(),
            ))
            .then(
                cmavo(Cmavo::Loho)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((li, li_free_modifiers), expression), loho)| {
                new!(SumtiSyntax::NumberSumti {
                    li: WithFreeModifiers::new(li, li_free_modifiers),
                    expression: Box::new(expression),
                    loho: loho.map(|(loho, free_modifiers)| {
                        WithFreeModifiers::new(loho, free_modifiers)
                    }),
                })
            }),
    );

    let letter = syntax_context(
        "lerfu string",
        letter_string()
            .then_ignore(selmaho(Selmaho::Moi).rewind().not())
            .then_ignore(selmaho(Selmaho::Mai).rewind().not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(
                cmavo(Cmavo::Boi)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|((letter, letter_free_modifiers), boi)| {
                new!(SumtiSyntax::LerfuStringSumti {
                    letter: WithFreeModifiers::new(word_run(letter), letter_free_modifiers),
                    boi: boi
                        .map(|(boi, free_modifiers)| WithFreeModifiers::new(boi, free_modifiers)),
                })
            }),
    );

    let koha = syntax_context(
        "pro-sumti",
        koha_argument()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(koha, free_modifiers)| {
                new!(SumtiSyntax::ProSumti(WithFreeModifiers::new(
                    koha,
                    free_modifiers
                )))
            }),
    );
    let lahe = syntax_context(
        "converted sumti",
        lahe_cmavo()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(
                relative_clauses(sumti.clone(), subbridi.clone(), free_modifier.clone())
                    .or_not()
                    .map(Option::unwrap_or_default),
            )
            .then(sumti.clone())
            .then(
                cmavo(Cmavo::Luhu)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |((((lahe, free_modifiers), relative_clauses), inner_sumti), luhu)| {
                    new!(SumtiSyntax::ReferentSumti {
                        lahe: WithFreeModifiers::new(lahe, free_modifiers),
                        relative_clauses,
                        inner_sumti: Box::new(inner_sumti),
                        luhu: luhu.map(|(luhu, free_modifiers)| {
                            WithFreeModifiers::new(luhu, free_modifiers)
                        }),
                    })
                },
            ),
    );
    let lahe_term_wrapper = lahe_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(
            cmavo(Cmavo::Luhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((wrapper, free_modifiers), inner_term), luhu)| {
            new!(SumtiSyntax::QualifiedTerm {
                term_wrapper_kind: SumtiWrapperKindSyntax::Referent,
                wrapper: WithFreeModifiers::new(wrapper, free_modifiers),
                wrapper_bo: None,
                inner_term: Box::new(inner_term),
                luhu: luhu
                    .map(|(luhu, free_modifiers)| WithFreeModifiers::new(luhu, free_modifiers)),
            })
        })
        .boxed();

    let name = syntax_context(
        "name",
        la_cmavo()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(((la, la_free_modifiers), names), name_free_modifiers)| {
                new!(SumtiSyntax::NameDescription {
                    la: WithFreeModifiers::new(la, la_free_modifiers),
                    names: WithFreeModifiers::new(word_run(names), name_free_modifiers),
                })
            }),
    );

    let contextual_quantifier = quantifier_with_free_modifiers_boxed(
        quantifier_with_context_boxed(sumti.clone(), selbri.clone(), free_modifier.clone()),
        free_modifier.clone(),
    );
    let mut argument6 = Recursive::declare();
    let descriptor_tail = argument_tail_with(
        argument6.clone(),
        sumti.clone(),
        selbri.clone(),
        subbridi.clone(),
        free_modifier.clone(),
    );
    let descriptor_head = le_cmavo()
        .or(la_cmavo())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(description, descriptor_free_modifiers)| {
            new!(DescriptionHeadSyntax {
                description: WithFreeModifiers::new(description, descriptor_free_modifiers),
            })
        });
    let descriptor_head_connective = jek_connective()
        .map(|connective| connective_with_kind(connective, ConnectiveKind::Afterthought));
    let description_connection = descriptor_head
        .clone()
        .then(descriptor_head_connective)
        .then(descriptor_head)
        .then(descriptor_tail.clone())
        .then(
            cmavo(Cmavo::Ku)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                (
                    ((leading_description_head, connective), trailing_description_head),
                    descriptor_tail,
                ),
                ku,
            )| {
                let (tail_elements, selbri, relative_clauses) = descriptor_tail;
                new!(SumtiSyntax::DescriptionConnection(Box::new(new!(
                    DescriptionConnectionSyntax {
                        leading_description_head: Box::new(leading_description_head),
                        connective,
                        trailing_description_head: Box::new(trailing_description_head),
                        tail_elements,
                        selbri,
                        relative_clauses,
                        ku: ku
                            .map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                    }
                ))))
            },
        );

    let descriptor_with_gadri = le_cmavo()
        .or(la_cmavo())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(descriptor_tail.clone())
        .then(
            cmavo(Cmavo::Ku)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((description, descriptor_free_modifiers), descriptor_tail), ku)| {
                let (tail_elements, selbri, relative_clauses) = descriptor_tail;
                new!(SumtiSyntax::Description(Box::new(new!(
                    DescriptionSyntax {
                        outer_quantifier: None,
                        description: Some(WithFreeModifiers::new(
                            description,
                            descriptor_free_modifiers,
                        )),
                        tail_elements,
                        selbri,
                        relative_clauses,
                        ku: ku
                            .map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                    }
                ))))
            },
        );
    let descriptor_with_outer_quantifier = contextual_quantifier
        .clone()
        .then(le_cmavo().or(la_cmavo()))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(descriptor_tail.clone())
        .then(
            cmavo(Cmavo::Ku)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(
                (((outer_quantifier, description), descriptor_free_modifiers), descriptor_tail),
                ku,
            )| {
                let (tail_elements, selbri, relative_clauses) = descriptor_tail;
                new!(SumtiSyntax::Description(Box::new(new!(
                    DescriptionSyntax {
                        outer_quantifier: Some(outer_quantifier),
                        description: Some(WithFreeModifiers::new(
                            description,
                            descriptor_free_modifiers,
                        )),
                        tail_elements,
                        selbri,
                        relative_clauses,
                        ku: ku
                            .map(|(ku, free_modifiers)| WithFreeModifiers::new(ku, free_modifiers)),
                    }
                ))))
            },
        );

    let descriptor_without_gadri = contextual_quantifier
        .clone()
        .then(selmaho(Selmaho::Roi).rewind().not())
        .map(|(quantifier, _)| quantifier)
        .map(|quantifier| {
            new!(DescriptionTailElementSyntax::DescriptionTailQuantifier(
                *quantifier
            ))
        })
        .then(selbri.clone())
        .then(
            relative_clauses(sumti.clone(), subbridi.clone(), free_modifier.clone())
                .or_not()
                .map(Option::unwrap_or_default),
        )
        .map(|((quantifier, selbri), relative_clauses)| {
            new!(SumtiSyntax::Description(Box::new(new!(
                DescriptionSyntax {
                    outer_quantifier: None,
                    description: None,
                    tail_elements: vec![quantifier],
                    selbri: Some(Box::new(selbri)),
                    relative_clauses,
                    ku: None,
                }
            ))))
        });

    let nahe_bo_argument = selmaho(Selmaho::Nahe)
        .then(cmavo(Cmavo::Bo))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(sumti.clone())
        .then(
            cmavo(Cmavo::Luhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|((((nahe, bo), free_modifiers), inner_sumti), luhu)| {
            new!(SumtiSyntax::ScalarNegatedSumtiWithBo {
                nahe,
                bo: WithFreeModifiers::new(bo, free_modifiers),
                inner_sumti: Box::new(inner_sumti),
                luhu: luhu
                    .map(|(luhu, free_modifiers)| WithFreeModifiers::new(luhu, free_modifiers)),
            })
        });
    let nahe_bo_term_wrapper = selmaho(Selmaho::Nahe)
        .then(cmavo(Cmavo::Bo))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(
            cmavo(Cmavo::Luhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((wrapper, wrapper_bo), free_modifiers), inner_term), luhu)| {
                new!(SumtiSyntax::QualifiedTerm {
                    term_wrapper_kind: SumtiWrapperKindSyntax::ScalarNegationWithBo,
                    wrapper: WithFreeModifiers::new(wrapper, Vec::new()),
                    wrapper_bo: Some(WithFreeModifiers::new(wrapper_bo, free_modifiers)),
                    inner_term: Box::new(inner_term),
                    luhu: luhu
                        .map(|(luhu, free_modifiers)| WithFreeModifiers::new(luhu, free_modifiers)),
                })
            },
        )
        .boxed();
    let nahe_argument = selmaho(Selmaho::Nahe)
        .then(cmavo(Cmavo::Bo).rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(sumti.clone())
        .then(
            cmavo(Cmavo::Luhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|((((nahe, _), free_modifiers), inner_sumti), luhu)| {
            new!(SumtiSyntax::ScalarNegatedSumti {
                nahe: WithFreeModifiers::new(nahe, free_modifiers),
                inner_sumti: Box::new(inner_sumti),
                luhu: luhu
                    .map(|(luhu, free_modifiers)| WithFreeModifiers::new(luhu, free_modifiers)),
            })
        })
        .boxed();
    let nahe_term_wrapper = selmaho(Selmaho::Nahe)
        .then(cmavo(Cmavo::Bo).rewind().not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(single_term.clone())
        .then(
            cmavo(Cmavo::Luhu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|((((wrapper, _), free_modifiers), inner_term), luhu)| {
            new!(SumtiSyntax::QualifiedTerm {
                term_wrapper_kind: SumtiWrapperKindSyntax::ScalarNegation,
                wrapper: WithFreeModifiers::new(wrapper, free_modifiers),
                wrapper_bo: None,
                inner_term: Box::new(inner_term),
                luhu: luhu
                    .map(|(luhu, free_modifiers)| WithFreeModifiers::new(luhu, free_modifiers)),
            })
        })
        .boxed();
    let bridi_description = selmaho(Selmaho::Lohoi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi.clone())
        .then(
            cmavo(Cmavo::Kuhau)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((lohoi, lohoi_free_modifiers), subbridi), kuhau)| {
            new!(SumtiSyntax::BridiDescription {
                lohoi: WithFreeModifiers::new(lohoi, lohoi_free_modifiers),
                subbridi: Box::new(subbridi),
                kuhau: kuhau
                    .map(|(kuhau, free_modifiers)| WithFreeModifiers::new(kuhau, free_modifiers)),
            })
        })
        .boxed();
    let quoted_or_simple_argument_core = choice((
        quote,
        mekso,
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
        syntax_label("description", description_connection),
        syntax_label("description", descriptor_with_outer_quantifier),
        syntax_label("description", descriptor_with_gadri),
        syntax_label("description", descriptor_without_gadri),
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
        relative_clauses(sumti.clone(), subbridi.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let unquantified_base_argument = unquantified_base_argument_core
        .clone()
        .map(Box::new)
        .then(base_relative_clauses.clone())
        .map(|(base_sumti, relative_clauses)| {
            if relative_clauses.is_empty() {
                *base_sumti
            } else {
                new!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti,
                    vuho: None,
                    relative_clauses,
                })
            }
        });
    let quantified_argument = contextual_quantifier
        .then(unquantified_base_argument_core.map(Box::new))
        .then(base_relative_clauses)
        .map(|((quantifier, inner_sumti), relative_clauses)| {
            let quantified = new!(SumtiSyntax::QuantifiedSumti {
                quantifier: *quantifier,
                inner_sumti,
            });
            if relative_clauses.is_empty() {
                quantified
            } else {
                new!(SumtiSyntax::SumtiWithRelativeClauses {
                    base_sumti: Box::new(quantified),
                    vuho: None,
                    relative_clauses,
                })
            }
        });
    let base_sumti = choice((unquantified_base_argument, quantified_argument)).boxed();

    let sumti4_boxed: BoxedParser<'tokens, BoxedSumtiSyntax> =
        recursive::<_, BoxedSumtiSyntax, _, _, _>(|argument4| {
            let gek_argument =
                modal_forethought_connective_with_free_modifiers(free_modifier.clone())
                    .then(sumti.clone().map(Box::new))
                    .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                    .then(argument4)
                    .then(optional_gihi_terminator())
                    .map(|((((gek, leading_sumti), gik), trailing_sumti), gihi)| {
                        Box::new(new!(SumtiSyntax::ForethoughtSumtiConnection {
                            gek,
                            leading_sumti,
                            gik,
                            trailing_sumti,
                            gihi,
                        }))
                    });

            choice((gek_argument, base_sumti.clone().map(Box::new))).boxed()
        })
        .boxed();
    let sumti3_boxed: BoxedParser<'tokens, BoxedSumtiSyntax> =
        recursive::<_, BoxedSumtiSyntax, _, _, _>(|argument3| {
            sumti4_boxed
                .clone()
                .then(
                    connective_with_free_modifiers(argument_connective(), free_modifier.clone())
                        .then(tense_modal_boxed().or_not())
                        .then(cmavo(Cmavo::Bo))
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .then(argument3)
                        .map(
                            |(
                                (((connective, tense_modal), bo), free_modifiers),
                                trailing_sumti,
                            )| {
                                Box::new(BoSumtiTailSyntax {
                                    connective,
                                    tense_modal,
                                    bo,
                                    free_modifiers,
                                    trailing_sumti,
                                })
                            },
                        )
                        .boxed()
                        .or_not(),
                )
                .map(|(leading_sumti, bo_tail)| match bo_tail {
                    None => leading_sumti,
                    Some(bo_tail) => {
                        let BoSumtiTailSyntax {
                            connective,
                            tense_modal,
                            bo,
                            free_modifiers,
                            trailing_sumti,
                        } = *bo_tail;
                        Box::new(new!(SumtiSyntax::BoundSumtiConnection {
                            leading_sumti,
                            bo_connective: Some(Box::new(connective)),
                            bo_tense_modal: tense_modal,
                            bo: WithFreeModifiers::new(bo, free_modifiers),
                            trailing_sumti,
                        }))
                    }
                })
                .boxed()
        })
        .boxed();
    let afterthought_argument_tail =
        connective_with_free_modifiers(argument_connective(), free_modifier.clone())
            .then(sumti3_boxed.clone())
            .boxed();
    let argument2_boxed = sumti3_boxed
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
            Box::new(continuations.into_iter().fold(
                *first,
                |leading_sumti, (connective, trailing_sumti)| {
                    new!(SumtiSyntax::SumtiConnection {
                        leading_sumti: Box::new(leading_sumti),
                        connective,
                        trailing_sumti,
                    })
                },
            ))
        })
        .boxed();

    let argument_ke_tail =
        connective_with_free_modifiers(argument_connective(), free_modifier.clone())
            .then(tense_modal_boxed().or_not())
            .then(cmavo(Cmavo::Ke))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(sumti.clone().map(Box::new))
            .then(
                cmavo(Cmavo::Kehe)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |(((((connective, tense_modal), ke), free_modifiers), inner_sumti), kehe)| {
                    Box::new(KeSumtiTailSyntax {
                        connective,
                        tense_modal,
                        ke,
                        free_modifiers,
                        inner_sumti,
                        kehe: kehe.map(|(kehe, free_modifiers)| {
                            WithFreeModifiers::new(kehe, free_modifiers)
                        }),
                    })
                },
            )
            .boxed();
    let argument1_boxed = argument2_boxed
        .clone()
        .then(
            argument_ke_tail
                .clone()
                .rewind()
                .ignore_then(argument_ke_tail)
                .or_not(),
        )
        .map(|(leading_sumti, ke_tail)| match ke_tail {
            None => leading_sumti,
            Some(ke_tail) => {
                let KeSumtiTailSyntax {
                    connective,
                    tense_modal,
                    ke,
                    free_modifiers,
                    inner_sumti,
                    kehe,
                } = *ke_tail;
                let connective = match tense_modal {
                    None => connective,
                    Some(tense_modal) => append_tense_modal_words(connective, *tense_modal),
                };
                Box::new(new!(SumtiSyntax::SumtiConnection {
                    leading_sumti,
                    connective,
                    trailing_sumti: Box::new(new!(SumtiSyntax::GroupedSumti {
                        ke: WithFreeModifiers::new(ke, free_modifiers),
                        inner_sumti,
                        kehe,
                    })),
                }))
            }
        })
        .boxed();

    argument1_boxed
        .then(
            cmavo(Cmavo::Vuho)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(
                    relative_clauses(sumti.clone(), subbridi, free_modifier.clone())
                        .or_not()
                        .map(Option::unwrap_or_default),
                )
                .then(
                    argument_connective()
                        .then(sumti.map(Box::new))
                        .map(|(connective, sumti)| SumtiConnectionSyntax { connective, sumti })
                        .or_not(),
                )
                .or_not(),
        )
        .map(|(base_sumti, vuho_attachment)| {
            if let Some((((vuho, vuho_free_modifiers), relative_clauses), sumti_connection)) =
                vuho_attachment
            {
                if !relative_clauses.is_empty() && sumti_connection.is_none() {
                    new!(SumtiSyntax::SumtiWithRelativeClauses {
                        base_sumti,
                        vuho: Some(WithFreeModifiers::new(vuho, vuho_free_modifiers)),
                        relative_clauses,
                    })
                } else {
                    new!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                        base_sumti,
                        vuho_marker: WithFreeModifiers::new(vuho, vuho_free_modifiers),
                        relative_clauses,
                        sumti_connection: sumti_connection.map(Box::new),
                    })
                }
            } else {
                *base_sumti
            }
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn implicit_zohe_argument() -> SumtiSyntax {
    new!(SumtiSyntax::ElidedSumti {
        tag: None,
        maybe_ku: None,
        free_modifiers: Vec::new(),
    })
}

#[requires(true)]
#[ensures(true)]
fn letter_string<'tokens>() -> BoxedParser<'tokens, Vec<Token>> {
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
fn number_words<'tokens>() -> BoxedParser<'tokens, Vec<Token>> {
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
fn number_or_letter_words<'tokens>() -> BoxedParser<'tokens, Vec<Token>> {
    choice((number_words(), letter_string())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn letter_word_tokens_from<'tokens, L>(letter_string: L) -> BoxedParser<'tokens, Vec<Token>>
where
    L: Parser<'tokens, ParserInput<'tokens>, Vec<Token>, ParseExtra<'tokens>> + Clone + 'tokens,
{
    recursive(|letter_tokens| {
        let by = letter_word().map(|word| vec![word]);
        let lau = selmaho(Selmaho::Lau)
            .then(letter_tokens.clone())
            .map(|(lau, mut rest)| {
                let mut words = vec![lau];
                words.append(&mut rest);
                words
            });
        let tei = cmavo(Cmavo::Tei)
            .then(letter_string.clone())
            .then(cmavo(Cmavo::Foi))
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
    number_quantifier_boxed()
        .map(|quantifier| *quantifier)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn number_quantifier_boxed<'tokens>() -> BoxedParser<'tokens, BoxedQuantifierSyntax> {
    number_words()
        .then(cmavo(Cmavo::Boi).or_not())
        .map(|(number, boi)| {
            Box::new(new!(QuantifierSyntax::NumberQuantifier {
                number: WithFreeModifiers::new(word_run(number), Vec::new()),
                boi: boi.map(|boi| WithFreeModifiers::new(boi, Vec::new())),
            }))
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier<'tokens>() -> BoxedParser<'tokens, QuantifierSyntax> {
    quantifier_boxed().map(|quantifier| *quantifier).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_boxed<'tokens>() -> BoxedParser<'tokens, BoxedQuantifierSyntax> {
    let vei_quantifier = cmavo(Cmavo::Vei)
        .then(math_expression_body().map(Box::new))
        .then(cmavo(Cmavo::Veho).or_not())
        .map(|((vei, mekso), veho)| {
            Box::new(new!(QuantifierSyntax::MeksoQuantifier {
                vei: WithFreeModifiers::new(vei, Vec::new()),
                mekso,
                veho: veho.map(|veho| WithFreeModifiers::new(veho, Vec::new())),
            }))
        });
    choice((vei_quantifier, number_quantifier_boxed())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_context<'tokens, A, R, F>(
    sumti: A,
    selbri: R,
    free_modifier: F,
) -> BoxedParser<'tokens, QuantifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    quantifier_with_context_boxed(sumti, selbri, free_modifier)
        .map(|quantifier| *quantifier)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_context_boxed<'tokens, A, R, F>(
    sumti: A,
    selbri: R,
    free_modifier: F,
) -> BoxedParser<'tokens, BoxedQuantifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let vei_quantifier = cmavo(Cmavo::Vei)
        .then(math_expression_body_with_context(sumti, selbri, free_modifier).map(Box::new))
        .then(cmavo(Cmavo::Veho).or_not())
        .map(|((vei, mekso), veho)| {
            Box::new(new!(QuantifierSyntax::MeksoQuantifier {
                vei: WithFreeModifiers::new(vei, Vec::new()),
                mekso,
                veho: veho.map(|veho| WithFreeModifiers::new(veho, Vec::new())),
            }))
        });
    choice((vei_quantifier, number_quantifier_boxed())).boxed()
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
    quantifier_with_free_modifiers_boxed(quantifier.map(Box::new), free_modifier)
        .map(|quantifier| *quantifier)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quantifier_with_free_modifiers_boxed<'tokens, Q, F>(
    quantifier: Q,
    free_modifier: F,
) -> BoxedParser<'tokens, BoxedQuantifierSyntax>
where
    Q: Parser<'tokens, ParserInput<'tokens>, BoxedQuantifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    quantifier
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|(quantifier, free_modifiers)| {
            Box::new(attach_quantifier_free_modifiers(
                *quantifier,
                free_modifiers,
            ))
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn attach_quantifier_free_modifiers(
    quantifier: QuantifierSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> QuantifierSyntax {
    match quantifier.into_data() {
        data!(QuantifierSyntax::NumberQuantifier { mut number, boi }) => {
            let boi = if let Some(mut boi) = boi {
                boi.free_modifiers.extend(free_modifiers);
                Some(boi)
            } else {
                number.free_modifiers.extend(free_modifiers);
                None
            };
            new!(QuantifierSyntax::NumberQuantifier { number, boi })
        }
        data!(QuantifierSyntax::MeksoQuantifier {
            mut vei,
            mekso,
            veho,
        }) => {
            let veho = if let Some(mut veho) = veho {
                veho.free_modifiers.extend(free_modifiers);
                Some(veho)
            } else {
                vei.free_modifiers.extend(free_modifiers);
                None
            };
            new!(QuantifierSyntax::MeksoQuantifier { vei, mekso, veho })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn quote_argument<'tokens, T, F>(
    _source: Option<&'tokens str>,
    text: T,
    free_modifier: F,
) -> BoxedParser<'tokens, SumtiSyntax>
where
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let compound_quote = any()
        .try_map(move |word: Token, span| {
            match word.core_word().as_data() {
                data!(WordLike::QuotedWord { .. }) => Ok(Box::new(new!(QuoteSyntax::WordQuote(
                    WithFreeModifiers::new(word.clone(), Vec::new()),
                )))),
                data!(WordLike::DelimitedNonLojbanQuote { .. }) => Ok(Box::new(new!(QuoteSyntax::DelimitedNonLojbanQuote(
                    WithFreeModifiers::new(word.clone(), Vec::new()),
                )))),
                data!(WordLike::QuotedWords { .. }) => Ok(Box::new(new!(QuoteSyntax::WordsQuote(
                    WithFreeModifiers::new(word.clone(), Vec::new()),
                )))),
                data!(WordLike::DelimitedWordQuote { .. }) => {
                    Ok(Box::new(new!(QuoteSyntax::DelimitedWordQuote(
                        WithFreeModifiers::new(word.clone(), Vec::new()),
                    ))))
                },
                _ => Err(SyntaxParseError::expected(
                    span,
                    vec![new!(SyntaxExpectedToken::WordCategory(
                        SyntaxWordCategory::Quote
                    ))],
                )),
            }
        })
        .labelled("QUOTE")
        .as_terminal()
        .map_with(
            |quote,
             extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            if let data!(QuoteSyntax::DelimitedWordQuote(zohoi)) = quote.as_data() {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZohOiQuote, &zohoi.value);
            }
            quote
        })
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(quote, free_modifiers)| attach_boxed_quote_free_modifiers(quote, free_modifiers));

    let lu_quote = cmavo(Cmavo::Lu)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text)
        .then(
            cmavo(Cmavo::Lihu)
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((lu, free_modifiers), text), lihu)| {
            Box::new(new!(QuoteSyntax::TextQuote {
                lu: WithFreeModifiers::new(lu, free_modifiers),
                text: Box::new(text),
                lihu: lihu
                    .map(|(lihu, free_modifiers)| WithFreeModifiers::new(lihu, free_modifiers)),
            }))
        });

    choice((compound_quote, lu_quote))
        .map(|quote| new!(SumtiSyntax::QuotedSumti(quote)))
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn attach_boxed_quote_free_modifiers(
    quote: Box<QuoteSyntax>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> Box<QuoteSyntax> {
    Box::new(quote_with_free_modifiers(*quote, free_modifiers))
}

#[requires(true)]
#[ensures(true)]
fn quote_with_free_modifiers(
    quote: QuoteSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> QuoteSyntax {
    match quote.into_data() {
        data!(QuoteSyntax::TextQuote { mut lu, text, lihu }) => {
            lu.free_modifiers.extend(free_modifiers);
            new!(QuoteSyntax::TextQuote { lu, text, lihu })
        }
        data!(QuoteSyntax::WordQuote(mut zo)) => {
            zo.free_modifiers.extend(free_modifiers);
            new!(QuoteSyntax::WordQuote(zo))
        }
        data!(QuoteSyntax::DelimitedWordQuote(mut zohoi)) => {
            zohoi.free_modifiers.extend(free_modifiers);
            new!(QuoteSyntax::DelimitedWordQuote(zohoi))
        }
        data!(QuoteSyntax::DelimitedNonLojbanQuote(mut zoi)) => {
            zoi.free_modifiers.extend(free_modifiers);
            new!(QuoteSyntax::DelimitedNonLojbanQuote(zoi))
        }
        data!(QuoteSyntax::WordsQuote(mut lohu)) => {
            lohu.free_modifiers.extend(free_modifiers);
            new!(QuoteSyntax::WordsQuote(lohu))
        }
    }
}
#[requires(true)]
#[ensures(true)]
fn relative_clauses<'tokens, A, S>(
    sumti: A,
    subbridi: S,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, Vec<RelativeClauseSyntax>>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let clause = relative_clause(sumti, subbridi, free_modifier.clone());
    syntax_context(
        "relative clauses",
        clause
            .clone()
            .then(
                choice((
                    cmavo(Cmavo::Zihe)
                        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                        .then(clause.clone())
                        .map(|((zihe, free_modifiers), inner)| {
                            new!(RelativeClauseSyntax::JoinedRelativeClauses {
                                zihe: WithFreeModifiers::new(zihe, free_modifiers),
                                inner: Box::new(inner),
                            })
                        }),
                    relative_clause_connective()
                        .then(clause)
                        .map(|(connective, inner)| {
                            new!(RelativeClauseSyntax::RelativeClauseConnection {
                                connective,
                                inner: Box::new(inner),
                            })
                        }),
                ))
                .repeated()
                .collect::<Vec<_>>(),
            )
            .map(|(first, rest)| std::iter::once(first).chain(rest).collect()),
    )
}

#[requires(true)]
#[ensures(true)]
fn relative_clause<'tokens, R>(
    sumti: impl Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    subbridi: R,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, RelativeClauseSyntax>
where
    R: Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let goi = syntax_label(
        "sumti association phrase",
        goi_relative_clause(sumti, free_modifier.clone()),
    )
    .map(|value| {
        new!(RelativeClauseSyntax::SumtiAssociationPhrase(Box::new(
            value
        )))
    });
    let noi = syntax_label(
        "relative bridi",
        cmavo_one_of(
            "NOI",
            &[
                Cmavo::Noi,
                Cmavo::Nohoi,
                Cmavo::Poi,
                Cmavo::Pohoi,
                Cmavo::Voi,
                Cmavo::Voihi,
            ],
        )
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(subbridi)
        .then(
            cmavo(Cmavo::Kuho)
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((marker, leading_free_modifiers), subbridi), kuho)| {
            if marker.is_one_of_cmavo(crate::tree::RESTRICTIVE_RELATIVE_CLAUSE_CMAVO) {
                new!(RelativeClauseSyntax::RestrictiveRelativeBridi {
                    poi: WithFreeModifiers::new(marker, leading_free_modifiers),
                    subbridi: Box::new(subbridi),
                    kuho: kuho.map(|(kuho, free_modifiers)| {
                        WithFreeModifiers::new(kuho, free_modifiers)
                    }),
                })
            } else {
                new!(RelativeClauseSyntax::IncidentalRelativeBridi {
                    noi: WithFreeModifiers::new(marker, leading_free_modifiers),
                    subbridi: Box::new(subbridi),
                    kuho: kuho.map(|(kuho, free_modifiers)| {
                        WithFreeModifiers::new(kuho, free_modifiers)
                    }),
                })
            }
        }),
    );
    syntax_label("relative clause", choice((goi, noi))).boxed()
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((joik_connective(), jek_connective())).boxed()
}

#[requires(true)]
#[ensures(true)]
fn goi_relative_clause<'tokens, A>(
    sumti: A,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, SumtiAssociationPhraseSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let tagged_tail = sumti
        .clone()
        .map(|sumti| (Some(sumti), None, Vec::new()))
        .or(cmavo(Cmavo::Ku)
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)))
        .boxed();
    let tense_tagged_argument = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail.clone())
        .map(
            |((tense_modal, tag_free_modifiers), (sumti, maybe_ku, trailing_free_modifiers))| {
                let tag = new!(SumtiTagSyntax::TenseModal(Box::new(
                    attach_tense_modal_free_modifiers(tense_modal, tag_free_modifiers,)
                )));
                if let Some(sumti) = sumti {
                    new!(SumtiSyntax::TaggedSumti {
                        tag,
                        inner_sumti: Box::new(sumti),
                    })
                } else {
                    build_zohe_argument(Some(tag), maybe_ku, trailing_free_modifiers)
                }
            },
        );
    let fa_tagged_argument = selmaho(Selmaho::Fa)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail)
        .map(
            |((fa, fa_free_modifiers), (sumti, maybe_ku, trailing_free_modifiers))| {
                let tag = new!(SumtiTagSyntax::PlaceTag(WithFreeModifiers::new(
                    fa,
                    fa_free_modifiers
                )));
                if let Some(sumti) = sumti {
                    new!(SumtiSyntax::TaggedSumti {
                        tag,
                        inner_sumti: Box::new(sumti),
                    })
                } else {
                    build_zohe_argument(Some(tag), maybe_ku, trailing_free_modifiers)
                }
            },
        );
    let argument_base = sumti
        .clone()
        .or(tense_tagged_argument)
        .or(fa_tagged_argument)
        .or(na_ku_argument_parser(free_modifier.clone()))
        .boxed();

    selmaho(Selmaho::Goi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(argument_base)
        .then(
            cmavo(Cmavo::Gehu)
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((goi, leading_free_modifiers), sumti), gehu)| {
            new!(SumtiAssociationPhraseSyntax {
                association_marker: WithFreeModifiers::new(goi, leading_free_modifiers),
                sumti: Box::new(sumti),
                gehu: gehu
                    .map(|(gehu, free_modifiers)| WithFreeModifiers::new(gehu, free_modifiers)),
            })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn na_ku_argument_parser<'tokens, F>(free_modifier: F) -> BoxedParser<'tokens, SumtiSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    na_cmavo()
        .then(cmavo(Cmavo::Ku))
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|((na, ku), free_modifiers)| {
            new!(SumtiSyntax::NegatedSumti {
                na,
                ku: WithFreeModifiers::new(ku, free_modifiers),
            })
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
        .then(cmavo(Cmavo::Boi).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, boi), free_modifiers)| {
            new!(MeksoSyntax::NumberMekso(Box::new(new!(
                QuantifierSyntax::NumberQuantifier {
                    number: WithFreeModifiers::new(
                        word_run(number),
                        if boi.is_some() {
                            Vec::new()
                        } else {
                            free_modifiers.clone()
                        },
                    ),
                    boi: boi.map(|boi| WithFreeModifiers::new(boi, free_modifiers)),
                }
            ))))
        });
    let xi_expression = choice((number_or_letter, math_expression_body()));

    selmaho(Selmaho::Xi)
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(xi_expression)
        .map(|((xi, free_modifiers), expression)| {
            new!(FreeModifierSyntax::Subscript {
                xi: WithFreeModifiers::new(xi, free_modifiers),
                expression: Box::new(expression),
            })
        })
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
        .then(selmaho(Selmaho::Mai))
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(|((number, mai), free_modifiers)| {
            new!(FreeModifierSyntax::UtteranceOrdinal {
                number: word_run(number),
                mai: WithFreeModifiers::new(mai, free_modifiers),
            })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn soi_free<'tokens, A, F>(sumti: A, free_modifier: F) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let prohibited_free_modifier = cll_prohibited_free_modifier(free_modifier.clone());
    cmavo(Cmavo::Soi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(sumti.clone())
        .then(sumti.or_not())
        .then(
            cmavo(Cmavo::Sehu)
                .then(prohibited_free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((soi, free_modifiers), leading_sumti), trailing_sumti), sehu)| {
                new!(FreeModifierSyntax::ReciprocalSumti {
                    soi: WithFreeModifiers::new(soi, free_modifiers),
                    leading_sumti: Box::new(leading_sumti),
                    trailing_sumti: trailing_sumti.map(Box::new),
                    sehu: sehu
                        .map(|(sehu, free_modifiers)| WithFreeModifiers::new(sehu, free_modifiers)),
                })
            },
        )
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vocative_free<'tokens, A, R>(
    sumti: A,
    selbri: R,
    subbridi: impl Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
    free_modifier: impl Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
    + Clone
    + 'tokens,
) -> BoxedParser<'tokens, FreeModifierSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let optional_relative_clauses =
        relative_clauses(sumti.clone(), subbridi.clone(), free_modifier.clone())
            .or_not()
            .map(Option::unwrap_or_default);
    let relation_vocative = optional_relative_clauses
        .clone()
        .then(selbri)
        .then(optional_relative_clauses.clone())
        .map(
            |((leading_relative_clauses, selbri), trailing_relative_clauses)| {
                new!(SumtiSyntax::SelbriVocative {
                    leading_relative_clauses,
                    selbri: Box::new(selbri),
                    trailing_relative_clauses,
                })
            },
        );
    let cmevla_vocative = optional_relative_clauses
        .clone()
        .then(cmevla_word().repeated().at_least(1).collect::<Vec<_>>())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(optional_relative_clauses)
        .map(
            |(((leading_relative_clauses, cmevla), free_modifiers), trailing_relative_clauses)| {
                let sumti = new!(SumtiSyntax::NameWords(WithFreeModifiers::new(
                    word_run(cmevla),
                    free_modifiers,
                )));
                let relative_clauses = leading_relative_clauses
                    .into_iter()
                    .chain(trailing_relative_clauses)
                    .collect::<Vec<_>>();
                if relative_clauses.is_empty() {
                    sumti
                } else {
                    new!(SumtiSyntax::SumtiWithRelativeClauses {
                        base_sumti: Box::new(sumti),
                        vuho: None,
                        relative_clauses,
                    })
                }
            },
        );
    let vocative_argument = choice((relation_vocative, cmevla_vocative, sumti));

    vocative_markers()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(vocative_argument.or_not())
        .then(
            cmavo(Cmavo::Dohu)
                .then(
                    cll_prohibited_free_modifier(free_modifier)
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map(|(((vocative_markers, free_modifiers), sumti), dohu)| {
            new!(FreeModifierSyntax::Vocative {
                vocative_markers: WithFreeModifiers::new(vocative_markers, free_modifiers),
                sumti: sumti.map(Box::new),
                dohu: dohu
                    .map(|(dohu, free_modifiers)| WithFreeModifiers::new(dohu, free_modifiers)),
            })
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
fn free_modifier_anchor(free_modifier: &FreeModifierSyntax) -> Option<Token> {
    free_modifier.first_word().cloned()
}

#[requires(true)]
#[ensures(true)]
fn vocative_markers<'tokens>() -> BoxedParser<'tokens, Vec<Token>> {
    let coi_marker = selmaho(Selmaho::Coi)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(coi, nai)| {
            let mut markers = vec![coi];
            if let Some(nai) = nai {
                markers.push(nai);
            }
            markers
        });

    choice((
        coi_marker
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(cmavo(Cmavo::Doi).or_not())
            .map(|(coi_markers, doi)| {
                let mut markers = coi_markers.into_iter().flatten().collect::<Vec<_>>();
                markers.extend(doi);
                markers
            }),
        cmavo(Cmavo::Doi).map(|doi| vec![doi]),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn argument_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let tagged_term_start = choice((tense_modal().ignored(), selmaho(Selmaho::Fa).ignored()));
    let cehe_connective = cmavo(Cmavo::Cehe)
        .then_ignore(tagged_term_start.rewind().not())
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(cmavo, nai)| {
            connective_syntax(
                ConnectiveKind::NonLogical,
                None,
                None,
                None,
                vec![cmavo],
                nai,
            )
        });
    choice((
        cehe_connective,
        na_cmavo()
            .or_not()
            .then(selmaho(Selmaho::Se).or_not())
            .then(selmaho(Selmaho::A))
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|(((na, se), cmavo), nai)| {
                connective_syntax(ConnectiveKind::Afterthought, se, None, na, vec![cmavo], nai)
            }),
        na_cmavo()
            .or_not()
            .then(selmaho(Selmaho::Se).or_not())
            .then(selmaho(Selmaho::Jehi))
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|(((na, se), cmavo), nai)| {
                connective_syntax(ConnectiveKind::Afterthought, se, None, na, vec![cmavo], nai)
            }),
        joik_connective(),
        selmaho(Selmaho::Joi)
            .or_not()
            .then(selmaho(Selmaho::Bihi))
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|((se, cmavo), nai)| {
                connective_syntax(ConnectiveKind::Interval, se, None, None, vec![cmavo], nai)
            }),
        selmaho(Selmaho::Gaho)
            .then(selmaho(Selmaho::Se).or_not())
            .then(selmaho(Selmaho::Bihi))
            .then(cmavo(Cmavo::Nai).or_not())
            .then(selmaho(Selmaho::Gaho))
            .map(|((((left_interval, se), cmavo), nai), right_interval)| {
                connective_syntax(
                    ConnectiveKind::Interval,
                    se,
                    None,
                    None,
                    vec![left_interval, cmavo, right_interval],
                    nai,
                )
            }),
        vuhu_nonlogical_connective(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn text_leading_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        standard_statement_connective(),
        cmavo(Cmavo::Cehe)
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|(cmavo, nai)| {
                connective_syntax(
                    ConnectiveKind::NonLogical,
                    None,
                    None,
                    None,
                    vec![cmavo],
                    nai,
                )
            }),
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
        .then(selmaho(Selmaho::Se).or_not())
        .then(selmaho(Selmaho::A))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(((na, se), cmavo), nai)| {
            connective_syntax(ConnectiveKind::Afterthought, se, None, na, vec![cmavo], nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn vuhu_nonlogical_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    selmaho(Selmaho::Vuhu)
        .map(|cmavo| {
            connective_syntax(
                ConnectiveKind::NonLogical,
                None,
                None,
                None,
                vec![cmavo],
                None,
            )
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
#[ensures(true)]
fn append_connective_free_modifiers(
    connective: ConnectiveSyntax,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind,
        se,
        nahe,
        na,
        mut cmavo,
        nai,
    } = connective.into_parts();
    let nai = if let Some(mut nai) = nai {
        nai.free_modifiers.extend(free_modifiers);
        Some(nai)
    } else {
        cmavo.free_modifiers.extend(free_modifiers);
        None
    };
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[requires(true)]
#[ensures(ret.cmavo().value.len() >= old(words.len()))]
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

#[requires(true)]
#[ensures(true)]
fn append_optional_tense_modal_and_bo(
    connective: ConnectiveSyntax,
    tense_modal: Option<BoxedTenseModalSyntax>,
    bo: Token,
) -> ConnectiveSyntax {
    let connective = if let Some(tense_modal) = tense_modal {
        append_tense_modal_words(connective, *tense_modal)
    } else {
        connective
    };
    append_connective_words(connective, vec![bo])
}

#[requires(true)]
#[ensures(ret.cmavo().value.len() >= old(connective.cmavo().value.len()))]
fn append_tense_modal_words(
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

#[requires(true)]
#[ensures(ret.cmavo().value.len() >= old(words.len()))]
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

#[requires(true)]
#[ensures(true)]
fn jek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(selmaho(Selmaho::Se).or_not())
        .then(selmaho(Selmaho::Ja))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(((na, se), cmavo), nai)| {
            connective_syntax(ConnectiveKind::Selbri, se, None, na, vec![cmavo], nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn joik_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    choice((
        selmaho(Selmaho::Se)
            .or_not()
            .then(selmaho(Selmaho::Joi))
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|((se, cmavo), nai)| {
                connective_syntax(ConnectiveKind::NonLogical, se, None, None, vec![cmavo], nai)
            }),
        selmaho(Selmaho::Se)
            .or_not()
            .then(selmaho(Selmaho::Bihi))
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|((se, cmavo), nai)| {
                connective_syntax(ConnectiveKind::Interval, se, None, None, vec![cmavo], nai)
            }),
        selmaho(Selmaho::Gaho)
            .then(selmaho(Selmaho::Se).or_not())
            .then(selmaho(Selmaho::Bihi))
            .then(cmavo(Cmavo::Nai).or_not())
            .then(selmaho(Selmaho::Gaho))
            .map(|((((left_interval, se), cmavo), nai), right_interval)| {
                connective_syntax(
                    ConnectiveKind::Interval,
                    se,
                    None,
                    None,
                    vec![left_interval, cmavo, right_interval],
                    nai,
                )
            }),
    ))
    .boxed()
}

#[requires(!connective.cmavo().value.is_empty())]
#[ensures(ret.len() >= old(connective.cmavo().value.len()))]
fn connective_tense_modal_leaves(connective: ConnectiveSyntax) -> Vec<Token> {
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
    selmaho(Selmaho::Nahe)
        .or_not()
        .then(selmaho(Selmaho::Se).or_not())
        .then(selmaho(Selmaho::Guha))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(((nahe, se), guha), nai)| {
            connective_syntax(ConnectiveKind::Forethought, se, nahe, None, vec![guha], nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn modal_forethought_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let dialect = parser_dialect_config();
    let ga = selmaho(Selmaho::Se)
        .or_not()
        .then(selmaho(Selmaho::Ga))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|((se, ga), nai)| {
            connective_syntax(ConnectiveKind::Forethought, se, None, None, vec![ga], nai)
        })
        .boxed();
    let modal_gi = tense_modal_boxed()
        .then(cmavo(Cmavo::Gi))
        .then(zantufa_gek_bo())
        .map(|((tense_modal, gi), bo)| {
            let mut cmavo = Vec::new();
            tense_modal.extend_words_into(&mut cmavo);
            cmavo.push(gi);
            cmavo.extend(bo);
            connective_syntax(ConnectiveKind::Forethought, None, None, None, cmavo, None)
        })
        .boxed();
    let jek_as_gek = jek_connective().map_with(
        |connective,
         extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            if let Some(anchor) = connective.cmavo().value.first() {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZantufaGek, anchor);
            }
            connective
        },
    )
    .boxed();
    let joik_jek_gi = choice((joik_connective(), jek_as_gek))
        .then(cmavo(Cmavo::Gi))
        .then(zantufa_gek_bo())
        .map(|((connective, gi), bo)| {
            let extra = [Some(gi), bo].into_iter().flatten().collect::<Vec<_>>();
            append_connective_words(connective, extra)
        })
        .boxed();
    let zantufa_initial_gi = cmavo(Cmavo::Gi)
        .map_with(
            |gi, extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZantufaGek, &gi);
                gi
            },
        )
        .then(
            choice((
                joik_connective().map(connective_tense_modal_leaves),
                jek_connective().map(connective_tense_modal_leaves),
                tense_modal_boxed().map(|tense_modal| {
                    let mut words = Vec::new();
                    tense_modal.extend_words_into(&mut words);
                    words
                }),
            ))
            .boxed(),
        )
        .then(cmavo(Cmavo::Bo).or_not())
        .map(|((gi, mut tail_words), bo)| {
            let mut cmavo = vec![gi];
            cmavo.append(&mut tail_words);
            cmavo.extend(bo);
            connective_syntax(ConnectiveKind::Forethought, None, None, None, cmavo, None)
        })
        .boxed();
    if dialect.zantufa_connectives_enabled {
        choice((ga, zantufa_initial_gi, joik_jek_gi, modal_gi)).boxed()
    } else {
        choice((ga, joik_jek_gi, modal_gi)).boxed()
    }
}

#[requires(true)]
#[ensures(true)]
fn zantufa_gek_bo<'tokens>() -> BoxedParser<'tokens, Option<Token>> {
    cmavo(Cmavo::Bo)
        .map_with(
            |bo, extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalZantufaGek, &bo);
                bo
            },
        )
        .or_not()
        .boxed()
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
    cmavo(Cmavo::Gi)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(gi, nai)| {
            connective_syntax(ConnectiveKind::Forethought, None, None, None, vec![gi], nai)
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
fn optional_gihi_terminator<'tokens>() -> BoxedParser<'tokens, Option<Token>> {
    if parser_dialect_config().zantufa_connectives_enabled {
        cmavo(Cmavo::Gihi)
            .map_with(
                |gihi,
                 extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
                    extra.state().warn(
                        ExperimentalConstruct::ExperimentalZantufaForethoughtGihi,
                        &gihi,
                    );
                    gihi
                },
            )
            .or_not()
            .boxed()
    } else {
        empty().map(|_| None).boxed()
    }
}

#[requires(true)]
#[ensures(true)]
fn gihek_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    na_cmavo()
        .or_not()
        .then(selmaho(Selmaho::Se).or_not())
        .then(selmaho(Selmaho::Giha))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(((na, se), cmavo), nai)| {
            connective_syntax(ConnectiveKind::BridiTail, se, None, na, vec![cmavo], nai)
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail_connective<'tokens>() -> BoxedParser<'tokens, ConnectiveSyntax> {
    let experimental = relation_afterthought_connective()
        .map(|connective| connective_with_kind(connective, ConnectiveKind::BridiTail));
    choice((gihek_connective(), experimental)).boxed()
}

#[requires(true)]
#[ensures(ret.kind() == old(kind.clone()))]
fn connective_with_kind(connective: ConnectiveSyntax, kind: ConnectiveKind) -> ConnectiveSyntax {
    let ConnectiveSyntaxParts {
        kind: _,
        se,
        nahe,
        na,
        cmavo,
        nai,
    } = connective.into_parts();
    ConnectiveSyntax::new(kind, se, nahe, na, cmavo, nai)
}

#[requires(true)]
#[ensures(true)]
fn mekso_operator<'tokens>() -> BoxedParser<'tokens, MeksoOperatorSyntax> {
    math_parser_pair().1
}

#[requires(true)]
#[ensures(true)]
fn wrapped_word(word: Token, free_modifiers: Vec<FreeModifierSyntax>) -> WithFreeModifiers<Token> {
    WithFreeModifiers::new(word, free_modifiers)
}

#[requires(true)]
#[ensures(true)]
fn wrapped_words(
    words: Vec<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> WithFreeModifiers<Vec<Token>> {
    WithFreeModifiers::new(words, free_modifiers)
}

#[requires(!words.is_empty(), "syntax word runs must be non-empty")]
#[ensures(!ret.is_empty())]
fn word_run(words: Vec<Token>) -> WordRun {
    WordRun::try_from_vec(words).expect("precondition guarantees non-empty words")
}

#[requires(!expressions.is_empty(), "math expression runs must be non-empty")]
#[ensures(!ret.is_empty())]
fn math_expression_vec(expressions: Vec<MeksoSyntax>) -> MeksoVec {
    MeksoVec::try_from_vec(expressions).expect("precondition guarantees non-empty expressions")
}

#[requires(true)]
#[ensures(true)]
fn word_run_leaves(words: &WordRun) -> Vec<Token> {
    words.iter().cloned().collect()
}

#[requires(true)]
#[ensures(true)]
fn connective_syntax(
    kind: ConnectiveKind,
    se: Option<Token>,
    nahe: Option<Token>,
    na: Option<Token>,
    cmavo: Vec<Token>,
    nai: Option<Token>,
) -> ConnectiveSyntax {
    ConnectiveSyntax::new(
        kind,
        se,
        nahe,
        na,
        wrapped_words(cmavo, Vec::new()),
        nai.map(|nai| wrapped_word(nai, Vec::new())),
    )
}

#[requires(true)]
#[ensures(true)]
fn goha_relation_unit(
    goha: Token,
    raho: Option<Token>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TanruUnitSyntax {
    if let Some(raho) = raho {
        new!(TanruUnitSyntax::ProBridi {
            goha: wrapped_word(goha, Vec::new()),
            raho: Some(wrapped_word(raho, free_modifiers)),
        })
    } else {
        new!(TanruUnitSyntax::ProBridi {
            goha: wrapped_word(goha, free_modifiers),
            raho: None,
        })
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with<'tokens, E, O>(
    expression: E,
    operator: O,
) -> BoxedParser<'tokens, MeksoOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MeksoSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MeksoOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let vuhu = selmaho(Selmaho::Vuhu).map(|vuhu| {
        new!(MeksoOperatorSyntax::Primitive(WithFreeModifiers::new(
            vuhu,
            Vec::new()
        )))
    });
    let maho = cmavo(Cmavo::Maho)
        .then(expression)
        .then(cmavo(Cmavo::Tehu).or_not())
        .map(|((maho, mekso), tehu)| {
            new!(MeksoOperatorSyntax::OperandAsOperator {
                maho: WithFreeModifiers::new(maho, Vec::new()),
                mekso: Box::new(mekso),
                tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, Vec::new())),
            })
        });
    let ke = cmavo(Cmavo::Ke)
        .then(operator.clone())
        .then(cmavo(Cmavo::Kehe).or_not())
        .map(|((ke, inner_operator), kehe)| {
            new!(MeksoOperatorSyntax::GroupedOperator {
                ke: WithFreeModifiers::new(ke, Vec::new()),
                inner_operator: Box::new(inner_operator),
                kehe: kehe.map(|kehe| WithFreeModifiers::new(kehe, Vec::new())),
            })
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator.clone())
        .map(|(((guhek, left_operator), gik), right_operator)| {
            new!(MeksoOperatorSyntax::OperatorConnection {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            })
        });
    let atom = choice((forethought, ke, maho, vuhu)).boxed();
    let bo_operator = atom
        .clone()
        .then(
            standard_statement_connective()
                .then(cmavo(Cmavo::Bo))
                .then(operator.clone())
                .or_not(),
        )
        .map(|(left_operator, bo_tail)| match bo_tail {
            Some(((connective, bo), right_operator)) => {
                new!(MeksoOperatorSyntax::BoundOperatorConnection {
                    left_operator: Box::new(left_operator),
                    connective,
                    bo: WithFreeModifiers::new(bo, Vec::new()),
                    right_operator: Box::new(right_operator),
                })
            }
            None => left_operator,
        });
    bo_operator
        .clone()
        .then(
            standard_statement_connective()
                .then(bo_operator)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    new!(MeksoOperatorSyntax::OperatorConnection {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    })
                })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn math_operator_with_context<'tokens, E, O, R>(
    expression: E,
    operator: O,
    selbri: R,
) -> BoxedParser<'tokens, MeksoOperatorSyntax>
where
    E: Parser<'tokens, ParserInput<'tokens>, MeksoSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    O: Parser<'tokens, ParserInput<'tokens>, MeksoOperatorSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
{
    let vuhu = selmaho(Selmaho::Vuhu).map(|vuhu| {
        new!(MeksoOperatorSyntax::Primitive(WithFreeModifiers::new(
            vuhu,
            Vec::new()
        )))
    });
    let maho = cmavo(Cmavo::Maho)
        .then(expression)
        .then(cmavo(Cmavo::Tehu).or_not())
        .map(|((maho, mekso), tehu)| {
            new!(MeksoOperatorSyntax::OperandAsOperator {
                maho: WithFreeModifiers::new(maho, Vec::new()),
                mekso: Box::new(mekso),
                tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, Vec::new())),
            })
        });
    let se = selmaho(Selmaho::Se)
        .then(operator.clone())
        .map(|(se, inner_operator)| {
            new!(MeksoOperatorSyntax::Converted {
                se: WithFreeModifiers::new(se, Vec::new()),
                inner_operator: Box::new(inner_operator),
            })
        });
    let nahe = selmaho(Selmaho::Nahe)
        .then(operator.clone())
        .map(|(nahe, inner_operator)| {
            new!(MeksoOperatorSyntax::ScalarNegated {
                nahe: WithFreeModifiers::new(nahe, Vec::new()),
                inner_operator: Box::new(inner_operator),
            })
        });
    let nahu = cmavo(Cmavo::Nahu)
        .then(selbri)
        .then(cmavo(Cmavo::Tehu).or_not())
        .map(|((nahu, selbri), tehu)| {
            new!(MeksoOperatorSyntax::SelbriAsOperator {
                nahu: WithFreeModifiers::new(nahu, Vec::new()),
                selbri: Box::new(selbri),
                tehu: tehu.map(|tehu| WithFreeModifiers::new(tehu, Vec::new())),
            })
        });
    let ke = cmavo(Cmavo::Ke)
        .then(operator.clone())
        .then(cmavo(Cmavo::Kehe).or_not())
        .map(|((ke, inner_operator), kehe)| {
            new!(MeksoOperatorSyntax::GroupedOperator {
                ke: WithFreeModifiers::new(ke, Vec::new()),
                inner_operator: Box::new(inner_operator),
                kehe: kehe.map(|kehe| WithFreeModifiers::new(kehe, Vec::new())),
            })
        });
    let forethought = guhek_connective()
        .then(operator.clone())
        .then(gik_connective())
        .then(operator.clone())
        .map(|(((guhek, left_operator), gik), right_operator)| {
            new!(MeksoOperatorSyntax::OperatorConnection {
                left_operator: Box::new(left_operator),
                connective: append_connective_words(guhek, gik.words()),
                right_operator: Box::new(right_operator),
            })
        });
    let atom = choice((se, nahe, forethought, ke, nahu, maho, vuhu)).boxed();
    let bo_operator = atom
        .clone()
        .then(
            standard_statement_connective()
                .then(cmavo(Cmavo::Bo))
                .then(operator.clone())
                .or_not(),
        )
        .map(|(left_operator, bo_tail)| match bo_tail {
            Some(((connective, bo), right_operator)) => {
                new!(MeksoOperatorSyntax::BoundOperatorConnection {
                    left_operator: Box::new(left_operator),
                    connective,
                    bo: WithFreeModifiers::new(bo, Vec::new()),
                    right_operator: Box::new(right_operator),
                })
            }
            None => left_operator,
        });
    bo_operator
        .clone()
        .then(
            standard_statement_connective()
                .then(bo_operator)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| {
            continuations
                .into_iter()
                .fold(first, |left_operator, (connective, right_operator)| {
                    new!(MeksoOperatorSyntax::OperatorConnection {
                        left_operator: Box::new(left_operator),
                        connective,
                        right_operator: Box::new(right_operator),
                    })
                })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn single_word_quoted_relation_unit<'tokens, F, B>(
    marker_cmavo: Cmavo,
    free_modifier: F,
    build: B,
) -> BoxedParser<'tokens, TanruUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    B: Fn(WithFreeModifiers<Token>) -> TanruUnitSyntax + Clone + 'tokens,
{
    any()
        .try_map(move |word: Token, span| {
            let data!(WordLike::DelimitedWordQuote {
                marker,
                ..
            }) = word.core_word().as_data()
            else {
                return Err(SyntaxParseError::expected(
                    span,
                    vec![new!(SyntaxExpectedToken::Cmavo(marker_cmavo))],
                ));
            };
            if marker.is_cmavo(marker_cmavo) {
                Ok(word.clone())
            } else {
                Err(SyntaxParseError::expected(
                    span,
                    vec![new!(SyntaxExpectedToken::Cmavo(marker_cmavo))],
                ))
            }
        })
        .labelled(marker_cmavo.canonical_text())
        .as_terminal()
        .map_with(
            move |word,
                  extra: &mut MapExtra<
                'tokens,
                '_,
                ParserInput<'tokens>,
                ParseExtra<'tokens>,
            >| {
                if let Some(construct) = quoted_relation_unit_warning(marker_cmavo) {
                    extra.state().warn(construct, &word);
                }
                word
            },
        )
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .map(move |(word, free_modifiers)| build(wrapped_word(word, free_modifiers)))
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn delimited_quoted_relation_unit<'tokens, F, B>(
    marker_cmavo: Cmavo,
    free_modifier: F,
    build: B,
) -> BoxedParser<'tokens, TanruUnitSyntax>
where
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
    B: Fn(WithFreeModifiers<Token>) -> TanruUnitSyntax + Clone + 'tokens,
{
    custom(move |input| {
        let checkpoint = input.save();
        let cursor = input.cursor();
        let Some(word): Option<Token> = input.next() else {
            let span = input.span_since(&cursor);
            return Err(SyntaxParseError::expected(
                span,
                vec![new!(SyntaxExpectedToken::Cmavo(marker_cmavo))],
            ));
        };
        let span = input.span_since(&cursor);
        let data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) = word.core_word().as_data()
        else {
            input.rewind(checkpoint);
            return Err(SyntaxParseError::expected(
                span,
                vec![new!(SyntaxExpectedToken::Cmavo(marker_cmavo))],
            ));
        };
        if !zoi.is_cmavo(marker_cmavo) {
            input.rewind(checkpoint);
            return Err(SyntaxParseError::expected(
                span,
                vec![new!(SyntaxExpectedToken::Cmavo(marker_cmavo))],
            ));
        }
        let state: &mut ParserState = input.state();
        if let Some(construct) = quoted_relation_unit_warning(marker_cmavo) {
            state.warn(construct, &word);
        }
        Ok(word)
    })
    .labelled(marker_cmavo.canonical_text())
    .as_terminal()
    .then(free_modifier.repeated().collect::<Vec<_>>())
    .map(move |(word, free_modifiers)| build(wrapped_word(word, free_modifiers)))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn quoted_relation_unit_warning(marker_cmavo: Cmavo) -> Option<ExperimentalConstruct> {
    match marker_cmavo {
        Cmavo::Mehoi => Some(ExperimentalConstruct::ExperimentalMehOiSelbriUnit),
        Cmavo::Gohoi => Some(ExperimentalConstruct::ExperimentalGohoiSelbriUnit),
        Cmavo::Muhoi => Some(ExperimentalConstruct::ExperimentalZantufaMuhoiSelbriUnit),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_parser_with<'tokens, P, R, S, T, F>(
    sumti: P,
    selbri: R,
    subbridi: S,
    text: T,
    free_modifier: F,
    source: Option<&'tokens str>,
) -> BoxedParser<'tokens, SelbriSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    R: Parser<'tokens, ParserInput<'tokens>, SelbriSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let zantufa_quotes_enabled = parser_dialect_config().zantufa_quotes_enabled;
    let tense_modal_with_free_modifiers = tense_modal_boxed()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(tense_modal, free_modifiers)| {
            attach_boxed_tense_modal_free_modifiers(tense_modal, free_modifiers)
        })
        .boxed();
    let me_argument = sumti.clone().or(letter_string().map(|letter| {
        new!(SumtiSyntax::LerfuStringSumti {
            letter: WithFreeModifiers::new(word_run(letter), Vec::new()),
            boi: None,
        })
    }));
    let me_unit = cmavo(Cmavo::Me)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(me_argument)
        .then(
            cmavo(Cmavo::Mehu)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .then(
            selmaho(Selmaho::Moi)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|((((me, me_free_modifiers), sumti), mehu), moi_marker)| {
            new!(TanruUnitSyntax::SumtiSelbri {
                me: wrapped_word(me, me_free_modifiers),
                sumti: Box::new(sumti),
                mehu: mehu.map(|(mehu, free_modifiers)| wrapped_word(mehu, free_modifiers)),
                moi_marker: moi_marker
                    .map(|(moi_marker, free_modifiers)| wrapped_word(moi_marker, free_modifiers)),
            })
        });
    let mehoi_unit =
        single_word_quoted_relation_unit(Cmavo::Mehoi, free_modifier.clone(), |word| {
            new!(TanruUnitSyntax::QuotedWordSelbri(word))
        });
    let gohoi_unit =
        single_word_quoted_relation_unit(Cmavo::Gohoi, free_modifier.clone(), |word| {
            new!(TanruUnitSyntax::QuotedBridiSelbri(word))
        });
    let muhoi_unit = delimited_quoted_relation_unit(Cmavo::Muhoi, free_modifier.clone(), |word| {
        new!(TanruUnitSyntax::QuotedTextSelbri(word))
    });
    let luhei_unit = cmavo(Cmavo::Luhei)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(text.clone())
        .then(
            cmavo(Cmavo::Lihau)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((luhei, luhei_free_modifiers), text), liau)| {
            new!(TanruUnitSyntax::TextSelbri {
                luhei: wrapped_word(luhei, luhei_free_modifiers),
                text: Box::new(text),
                liau: liau.map(|(liau, free_modifiers)| wrapped_word(liau, free_modifiers)),
            })
        })
        .boxed();

    let brivla_word_unit = brivla_relation_word(parser_dialect_config().cbm_enabled)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(word, free_modifiers)| {
            new!(TanruUnitSyntax::TanruUnitWord(wrapped_word(
                word,
                free_modifiers
            )))
        });
    let goha_word_unit = selmaho(Selmaho::Goha)
        .then_ignore(
            choice((
                cmavo(Cmavo::Raho).ignored(),
                cmavo(Cmavo::Be).ignored(),
                pa_word().ignored(),
                free_modifier.clone().ignored(),
            ))
            .rewind()
            .not(),
        )
        .map(|word| {
            new!(TanruUnitSyntax::TanruUnitWord(wrapped_word(
                word,
                Vec::new()
            )))
        });
    let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
    let goha_unit = selmaho(Selmaho::Goha)
        .then(cmavo(Cmavo::Raho).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, raho, free_modifiers));
    let goha_raho_unit = selmaho(Selmaho::Goha)
        .then(cmavo(Cmavo::Raho))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, Some(raho), free_modifiers));
    let moi_unit = number_or_letter_words()
        .then(selmaho(Selmaho::Moi))
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|((number, moi), free_modifiers)| {
            new!(TanruUnitSyntax::OrdinalSelbri {
                number: word_run(number),
                moi: wrapped_word(moi, free_modifiers),
            })
        });
    let contextual_math_operator =
        math_parser_pair_with_context(sumti.clone(), selbri.clone(), free_modifier.clone()).1;
    let nuha_unit = cmavo(Cmavo::Nuha)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(contextual_math_operator)
        .map(|((nuha, free_modifiers), mekso_operator)| {
            new!(TanruUnitSyntax::OperatorSelbri {
                nuha: wrapped_word(nuha, free_modifiers),
                mekso_operator: Box::new(mekso_operator),
            })
        });
    let xohi_unit = cmavo(Cmavo::Xohi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_with_free_modifiers.clone())
        .map(|((xohi, free_modifiers), tag)| {
            new!(TanruUnitSyntax::TagSelbri {
                xohi: wrapped_word(xohi, free_modifiers),
                tag,
            })
        });

    let ke_unit = cmavo(Cmavo::Ke)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(relation_units_inner(
            sumti.clone(),
            subbridi.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .then(
            cmavo(Cmavo::Kehe)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((ke, ke_free_modifiers), selbri), kehe)| {
            new!(TanruUnitSyntax::GroupedTanruUnit {
                ke_tense_modal: None,
                ke: wrapped_word(ke, ke_free_modifiers),
                selbri,
                kehe: kehe.map(|(kehe, free_modifiers)| wrapped_word(kehe, free_modifiers)),
            })
        });

    let se_unit = recursive(|se_unit| {
        let nahe_inner_choices = if zantufa_quotes_enabled {
            choice((
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
            ))
            .boxed()
        } else {
            choice((
                se_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                xohi_unit.clone(),
                nuha_unit.clone(),
                moi_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            ))
            .boxed()
        };
        let nahe_inner_unit = selmaho(Selmaho::Nahe)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(nahe_inner_choices)
            .map(|((nahe, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ScalarNegatedTanruUnit {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        selmaho(Selmaho::Se)
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
            .map(|((se, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            })
    })
    .boxed();

    let wrapped_tense_unit = tense_modal_with_free_modifiers
        .clone()
        .then(relation_units_inner(
            sumti.clone(),
            subbridi.clone(),
            text.clone(),
            free_modifier.clone(),
            source,
        ))
        .map(|(tense_modal, inner_selbri)| {
            new!(TanruUnitSyntax::SelbriGroupTanruUnit(Box::new(new!(
                SelbriSyntax::TaggedSelbri {
                    tense_modal,
                    inner_selbri,
                }
            ))))
        });

    let jai_inner_unit = recursive(|jai_inner_unit| {
        let se_inner_unit = selmaho(Selmaho::Se)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_inner_unit.clone())
            .map(|((se, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        let nahe_inner_unit = selmaho(Selmaho::Nahe)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_inner_unit.clone())
            .map(|((nahe, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ScalarNegatedTanruUnit {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        if zantufa_quotes_enabled {
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
            .boxed()
        } else {
            choice((
                se_inner_unit,
                nahe_inner_unit,
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                ke_unit.clone(),
                moi_unit.clone(),
                nuha_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            ))
            .boxed()
        }
    })
    .boxed();

    let jai_unit = cmavo(Cmavo::Jai)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tense_modal_with_free_modifiers.clone().or_not())
        .then(jai_inner_unit)
        .map(|(((jai, free_modifiers), tense_modal), inner_unit)| {
            new!(TanruUnitSyntax::ModalConversion {
                jai: wrapped_word(jai, free_modifiers),
                tense_modal,
                inner_unit: Box::new(inner_unit),
            })
        });
    let se_jai_unit = selmaho(Selmaho::Se)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(jai_unit.clone())
        .map(|((se, free_modifiers), inner_unit)| {
            new!(TanruUnitSyntax::ConvertedTanruUnit {
                se: wrapped_word(se, free_modifiers),
                inner_unit: Box::new(inner_unit),
            })
        });

    let nahe_unit = recursive(|nahe_unit| {
        selmaho(Selmaho::Nahe)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(if zantufa_quotes_enabled {
                choice((
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
                ))
                .boxed()
            } else {
                choice((
                    wrapped_tense_unit.clone(),
                    ke_unit.clone(),
                    me_unit.clone(),
                    mehoi_unit.clone(),
                    gohoi_unit.clone(),
                    xohi_unit.clone(),
                    nuha_unit.clone(),
                    moi_unit.clone(),
                    se_unit.clone(),
                    jai_unit.clone(),
                    nahe_unit,
                    goha_unit.clone(),
                    word_unit.clone(),
                ))
                .boxed()
            })
            .map(|((nahe, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ScalarNegatedTanruUnit {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            })
    })
    .boxed();

    let nu_cmavo = || selmaho(Selmaho::Nu);
    let abstractor_connections = statement_connective()
        .then(nu_cmavo())
        .then(cmavo(Cmavo::Nai).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .map(|(((connective, nu), nai), free_modifiers)| {
            new!(AbstractorConnectionSyntax {
                connective,
                nu: WithFreeModifiers::new(
                    nu,
                    if nai.is_some() {
                        Vec::new()
                    } else {
                        free_modifiers.clone()
                    },
                ),
                nai: nai.map(|nai| WithFreeModifiers::new(nai, free_modifiers)),
            })
        });
    let abstraction_subbridi_unit = nu_cmavo()
        .then(cmavo(Cmavo::Nai).or_not())
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(abstractor_connections.repeated().collect::<Vec<_>>())
        .then(subbridi)
        .then(
            cmavo(Cmavo::Kei)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |(((((nu, nai), free_modifiers), abstractor_connections), subbridi), kei)| {
                new!(TanruUnitSyntax::Abstraction(Box::new(new!(
                    AbstractionSyntax {
                        nu: WithFreeModifiers::new(
                            nu,
                            if nai.is_some() {
                                Vec::new()
                            } else {
                                free_modifiers.clone()
                            },
                        ),
                        nai: nai.map(|nai| WithFreeModifiers::new(nai, free_modifiers)),
                        abstractor_connections,
                        subbridi: Box::new(subbridi),
                        kei: kei.map(|(kei, free_modifiers)| WithFreeModifiers::new(
                            kei,
                            free_modifiers
                        )),
                    }
                ))))
            },
        )
        .boxed();

    let se_abstraction_unit = selmaho(Selmaho::Se)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(abstraction_subbridi_unit.clone())
        .map(|((se, free_modifiers), inner_unit)| {
            new!(TanruUnitSyntax::ConvertedTanruUnit {
                se: wrapped_word(se, free_modifiers),
                inner_unit: Box::new(inner_unit),
            })
        });

    let base_unit = if zantufa_quotes_enabled {
        choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subbridi_unit.clone(),
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
        .map(Box::new)
        .boxed()
    } else {
        choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subbridi_unit.clone(),
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
        .map(Box::new)
        .boxed()
    };
    let base_unit_for_cei = if zantufa_quotes_enabled {
        choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            muhoi_unit.clone(),
            luhei_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subbridi_unit.clone(),
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
        .map(Box::new)
        .boxed()
    } else {
        choice((
            goha_raho_unit.clone(),
            me_unit.clone(),
            mehoi_unit.clone(),
            gohoi_unit.clone(),
            se_abstraction_unit.clone(),
            abstraction_subbridi_unit.clone(),
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
        .map(Box::new)
        .boxed()
    };
    let be_link = be_link_parser(sumti.clone(), free_modifier.clone());
    let selbri_relative_clause = cmavo(Cmavo::Nohoi)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(selbri.clone().map(Box::new))
        .then(
            cmavo(Cmavo::Kuhoi)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(|(((nohoi, leading_free_modifiers), selbri), kuhoi)| {
            new!(SelbriRelativePhraseSyntax {
                nohoi: WithFreeModifiers::new(nohoi, leading_free_modifiers),
                selbri,
                kuhoi: kuhoi
                    .map(|(kuhoi, free_modifiers)| WithFreeModifiers::new(kuhoi, free_modifiers)),
            })
        })
        .boxed();

    let linked_unit_from = |base_unit: BoxedParser<'tokens, BoxedTanruUnitSyntax>| {
        base_unit
            .then(be_link.clone().or_not())
            .map(|(base, be_link)| match be_link {
                None => base,
                Some(link) => {
                    let data!(LinkedSumtiListSyntax {
                        be,
                        fa,
                        first_sumti,
                        bei_links,
                        beho,
                    }) = link.into_data();

                    Box::new(new!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                        base,
                        be,
                        fa,
                        first_sumti,
                        bei_links,
                        beho,
                    }))
                }
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
                    Box::new(new!(TanruUnitSyntax::RelativeClauses {
                        base: linked_unit,
                        selbri_relative_clauses,
                    }))
                }
            })
            .boxed()
    };
    let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
        let data!(LinkedSumtiListSyntax {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
        }) = link.into_data();

        Box::new(new!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            be,
            fa,
            first_sumti,
            bei_links,
            beho,
            base,
        }))
    });
    let linked_unit = linked_unit_from(base_unit);
    let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
    let cei_unit = linked_unit_for_cei
        .clone()
        .then(
            cmavo(Cmavo::Cei)
                .then(linked_unit_for_cei.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(base, be_link)| {
            Box::new(new!(TanruUnitSyntax::AssignedProBridi {
                base,
                assignments: be_link
                    .into_iter()
                    .map(|(cei, tanru_unit)| new!(ProBridiAssignmentSyntax {
                        cei: wrapped_word(cei, Vec::new()),
                        tanru_unit,
                    }))
                    .collect(),
            }))
        })
        .boxed();

    let bo_unit: BoxedParser<'tokens, BoxedTanruUnitSyntax> =
        recursive::<_, BoxedTanruUnitSyntax, _, _, _>(|bo_unit| {
            let guha_unit = guhek_connective()
                .then(selbri.clone().map(Box::new))
                .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                .then(bo_unit.clone())
                .then(optional_gihi_terminator())
                .map(|((((guhek, leading_selbri), gik), trailing_unit), gihi)| {
                    Box::new(new!(TanruUnitSyntax::SelbriGroupTanruUnit(Box::new(new!(
                        SelbriSyntax::ForethoughtSelbriConnection {
                            guhek,
                            leading_bridi: Box::new(relation_to_empty_predicate(*leading_selbri)),
                            gik,
                            trailing_bridi: Box::new(relation_to_empty_predicate(
                                relation_unit_to_relation(&trailing_unit),
                            )),
                            gihi,
                        }
                    )))))
                });
            let atom_unit = choice((
                guha_unit,
                preposed_unit.clone(),
                cei_unit.clone(),
                linked_unit.clone(),
            ))
            .boxed();
            let connected_bo_tail = statement_connective()
                .then(tense_modal_boxed().or_not())
                .then(cmavo(Cmavo::Bo))
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit.clone())
                .map(
                    |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                        Box::new(BoTanruUnitTailSyntax {
                            connective: Some(Box::new(connective)),
                            tense_modal: bo_tense_modal,
                            bo,
                            free_modifiers,
                            trailing_unit,
                        })
                    },
                );
            let bare_bo_tail = cmavo(Cmavo::Bo)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(bo_unit)
                .map(|((bo, free_modifiers), trailing_unit)| {
                    Box::new(BoTanruUnitTailSyntax {
                        connective: None,
                        tense_modal: None,
                        bo,
                        free_modifiers,
                        trailing_unit,
                    })
                });
            atom_unit
                .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
                .map(|(leading_unit, bo_tail)| match bo_tail {
                    None => leading_unit,
                    Some(bo_tail) => {
                        let BoTanruUnitTailSyntax {
                            connective,
                            tense_modal,
                            bo,
                            free_modifiers,
                            trailing_unit,
                        } = *bo_tail;
                        Box::new(new!(TanruUnitSyntax::BoundTanruUnitConnection {
                            leading_unit,
                            bo_connective: connective,
                            bo_tense_modal: tense_modal,
                            bo: wrapped_word(bo, free_modifiers),
                            trailing_unit,
                        }))
                    }
                })
        })
        .boxed();

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
                    Box::new(new!(TanruUnitSyntax::TanruUnitConnection {
                        leading_unit,
                        connective,
                        trailing_unit,
                    }))
                })
        });

    let tanru_units = connected_unit
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(boxed_selbri_from_boxed_units);

    let base_relation = tanru_units;
    let connected_relation = base_relation
        .clone()
        .then(
            relation_afterthought_connective()
                .then(base_relation.clone())
                .or_not(),
        )
        .map(|(leading_selbri, connected)| match connected {
            None => leading_selbri,
            Some((connective, trailing_selbri)) => Box::new(new!(SelbriSyntax::SelbriConnection {
                connective,
                leading_selbri,
                trailing_selbri,
            })),
        });
    let na_relation = na_cmavo()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(selbri.clone().map(Box::new))
        .map(|((na, free_modifiers), inner_selbri)| {
            Box::new(new!(SelbriSyntax::Negated {
                na: wrapped_word(na, free_modifiers),
                inner_selbri,
            }))
        });
    let co_selbri = recursive::<_, BoxedSelbriSyntax, _, _, _>(|co_selbri| {
        connected_relation
            .clone()
            .then(
                cmavo(Cmavo::Co)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(co_selbri)
                    .or_not(),
            )
            .map(|(leading_selbri, co_tail)| match co_tail {
                None => leading_selbri,
                Some(((co, free_modifiers), trailing_selbri)) => {
                    Box::new(new!(SelbriSyntax::InvertedTanru {
                        leading_selbri,
                        co: wrapped_word(co, free_modifiers),
                        trailing_selbri,
                    }))
                }
            })
    });

    let untagged_relation = choice((na_relation, co_selbri)).boxed();
    let tagged_relation = tense_modal_with_free_modifiers
        .then(untagged_relation.clone())
        .map(|(tense_modal, inner_selbri)| {
            Box::new(new!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }))
        });

    choice((tagged_relation, untagged_relation))
        .map(|selbri| *selbri)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn relation_units_inner<'tokens, P, S, T, F>(
    sumti: P,
    subbridi: S,
    text: T,
    free_modifier: F,
    _source: Option<&'tokens str>,
) -> BoxedParser<'tokens, BoxedSelbriSyntax>
where
    P: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    S: Parser<'tokens, ParserInput<'tokens>, SubbridiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    T: Parser<'tokens, ParserInput<'tokens>, TextSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    recursive::<_, BoxedSelbriSyntax, _, _, _>(|inner_selbri| {
        let zantufa_quotes_enabled = parser_dialect_config().zantufa_quotes_enabled;
        let me_argument = sumti.clone().or(letter_string().map(|letter| {
            new!(SumtiSyntax::LerfuStringSumti {
                letter: WithFreeModifiers::new(word_run(letter), Vec::new()),
                boi: None,
            })
        }));
        let me_unit = cmavo(Cmavo::Me)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(me_argument)
            .then(
                cmavo(Cmavo::Mehu)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .then(
                selmaho(Selmaho::Moi)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|((((me, me_free_modifiers), sumti), mehu), moi_marker)| {
                new!(TanruUnitSyntax::SumtiSelbri {
                    me: wrapped_word(me, me_free_modifiers),
                    sumti: Box::new(sumti),
                    mehu: mehu.map(|(mehu, free_modifiers)| wrapped_word(mehu, free_modifiers)),
                    moi_marker: moi_marker.map(|(moi_marker, free_modifiers)| {
                        wrapped_word(moi_marker, free_modifiers)
                    }),
                })
            });
        let mehoi_unit =
            single_word_quoted_relation_unit(Cmavo::Mehoi, free_modifier.clone(), |word| {
                new!(TanruUnitSyntax::QuotedWordSelbri(word))
            });
        let gohoi_unit =
            single_word_quoted_relation_unit(Cmavo::Gohoi, free_modifier.clone(), |word| {
                new!(TanruUnitSyntax::QuotedBridiSelbri(word))
            });
        let muhoi_unit =
            delimited_quoted_relation_unit(Cmavo::Muhoi, free_modifier.clone(), |word| {
                new!(TanruUnitSyntax::QuotedTextSelbri(word))
            });
        let luhei_unit = cmavo(Cmavo::Luhei)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(text.clone())
            .then(
                cmavo(Cmavo::Lihau)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((luhei, luhei_free_modifiers), text), liau)| {
                new!(TanruUnitSyntax::TextSelbri {
                    luhei: wrapped_word(luhei, luhei_free_modifiers),
                    text: Box::new(text),
                    liau: liau.map(|(liau, free_modifiers)| wrapped_word(liau, free_modifiers)),
                })
            })
            .boxed();
        let brivla_word_unit = brivla_relation_word(parser_dialect_config().cbm_enabled)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(word, free_modifiers)| {
                new!(TanruUnitSyntax::TanruUnitWord(wrapped_word(
                    word,
                    free_modifiers
                )))
            });
        let goha_word_unit = selmaho(Selmaho::Goha)
            .then_ignore(
                choice((
                    cmavo(Cmavo::Raho).ignored(),
                    cmavo(Cmavo::Be).ignored(),
                    pa_word().ignored(),
                    free_modifier.clone().ignored(),
                ))
                .rewind()
                .not(),
            )
            .map(|word| {
                new!(TanruUnitSyntax::TanruUnitWord(wrapped_word(
                    word,
                    Vec::new()
                )))
            });
        let word_unit = choice((brivla_word_unit, goha_word_unit)).boxed();
        let goha_unit = selmaho(Selmaho::Goha)
            .then(cmavo(Cmavo::Raho).or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| goha_relation_unit(goha, raho, free_modifiers));
        let goha_raho_unit = selmaho(Selmaho::Goha)
            .then(cmavo(Cmavo::Raho))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((goha, raho), free_modifiers)| {
                goha_relation_unit(goha, Some(raho), free_modifiers)
            });
        let moi_unit = number_or_letter_words()
            .then(selmaho(Selmaho::Moi))
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|((number, moi), free_modifiers)| {
                new!(TanruUnitSyntax::OrdinalSelbri {
                    number: word_run(number),
                    moi: wrapped_word(moi, free_modifiers),
                })
            });
        let nuha_unit = cmavo(Cmavo::Nuha)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(mekso_operator())
            .map(|((nuha, free_modifiers), mekso_operator)| {
                new!(TanruUnitSyntax::OperatorSelbri {
                    nuha: wrapped_word(nuha, free_modifiers),
                    mekso_operator: Box::new(mekso_operator),
                })
            });
        let xohi_unit = cmavo(Cmavo::Xohi)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(tense_modal_boxed())
            .map(|((xohi, free_modifiers), tag)| {
                new!(TanruUnitSyntax::TagSelbri {
                    xohi: wrapped_word(xohi, free_modifiers),
                    tag,
                })
            });
        let nu_cmavo = || selmaho(Selmaho::Nu);
        let abstractor_connections = statement_connective()
            .then(nu_cmavo())
            .then(cmavo(Cmavo::Nai).or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(((connective, nu), nai), free_modifiers)| {
                new!(AbstractorConnectionSyntax {
                    connective,
                    nu: WithFreeModifiers::new(
                        nu,
                        if nai.is_some() {
                            Vec::new()
                        } else {
                            free_modifiers.clone()
                        },
                    ),
                    nai: nai.map(|nai| WithFreeModifiers::new(nai, free_modifiers)),
                })
            });
        let abstraction_subbridi_unit = nu_cmavo()
            .then(cmavo(Cmavo::Nai).or_not())
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(abstractor_connections.repeated().collect::<Vec<_>>())
            .then(subbridi.clone())
            .then(
                cmavo(Cmavo::Kei)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(
                |(((((nu, nai), free_modifiers), abstractor_connections), subbridi), kei)| {
                    new!(TanruUnitSyntax::Abstraction(Box::new(new!(
                        AbstractionSyntax {
                            nu: WithFreeModifiers::new(
                                nu,
                                if nai.is_some() {
                                    Vec::new()
                                } else {
                                    free_modifiers.clone()
                                },
                            ),
                            nai: nai.map(|nai| WithFreeModifiers::new(nai, free_modifiers)),
                            abstractor_connections,
                            subbridi: Box::new(subbridi),
                            kei: kei.map(|(kei, free_modifiers)| {
                                WithFreeModifiers::new(kei, free_modifiers)
                            }),
                        }
                    ))))
                },
            )
            .boxed();
        let se_abstraction_unit = selmaho(Selmaho::Se)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(abstraction_subbridi_unit.clone())
            .map(|((se, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        let ke_unit = cmavo(Cmavo::Ke)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_selbri.clone())
            .then(
                cmavo(Cmavo::Kehe)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((ke, ke_free_modifiers), selbri), kehe)| {
                new!(TanruUnitSyntax::GroupedTanruUnit {
                    ke_tense_modal: None,
                    ke: wrapped_word(ke, ke_free_modifiers),
                    selbri,
                    kehe: kehe.map(|(kehe, free_modifiers)| wrapped_word(kehe, free_modifiers)),
                })
            });
        let se_unit = recursive(|se_unit| {
            selmaho(Selmaho::Se)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(choice((
                    ke_unit.clone(),
                    moi_unit.clone(),
                    nuha_unit.clone(),
                    se_unit,
                    word_unit.clone(),
                    goha_unit.clone(),
                )))
                .map(|((se, free_modifiers), inner_unit)| {
                    new!(TanruUnitSyntax::ConvertedTanruUnit {
                        se: wrapped_word(se, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    })
                })
        })
        .boxed();
        let jai_inner_unit = recursive(|jai_inner_unit| {
            let se_inner_unit = selmaho(Selmaho::Se)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(jai_inner_unit.clone())
                .map(|((se, free_modifiers), inner_unit)| {
                    new!(TanruUnitSyntax::ConvertedTanruUnit {
                        se: wrapped_word(se, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    })
                });
            let nahe_inner_unit = selmaho(Selmaho::Nahe)
                .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                .then(jai_inner_unit.clone())
                .map(|((nahe, free_modifiers), inner_unit)| {
                    new!(TanruUnitSyntax::ScalarNegatedTanruUnit {
                        nahe: wrapped_word(nahe, free_modifiers),
                        inner_unit: Box::new(inner_unit),
                    })
                });
            if zantufa_quotes_enabled {
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
                .boxed()
            } else {
                choice((
                    se_inner_unit,
                    nahe_inner_unit,
                    me_unit.clone(),
                    mehoi_unit.clone(),
                    gohoi_unit.clone(),
                    ke_unit.clone(),
                    moi_unit.clone(),
                    nuha_unit.clone(),
                    goha_unit.clone(),
                    word_unit.clone(),
                ))
                .boxed()
            }
        })
        .boxed();
        let jai_unit = cmavo(Cmavo::Jai)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(tense_modal_boxed().or_not())
            .then(jai_inner_unit)
            .map(|(((jai, free_modifiers), tense_modal), inner_unit)| {
                new!(TanruUnitSyntax::ModalConversion {
                    jai: wrapped_word(jai, free_modifiers),
                    tense_modal,
                    inner_unit: Box::new(inner_unit),
                })
            });
        let se_jai_unit = selmaho(Selmaho::Se)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(jai_unit.clone())
            .map(|((se, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se: wrapped_word(se, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        let nahe_unit = selmaho(Selmaho::Nahe)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(choice((
                ke_unit.clone(),
                moi_unit.clone(),
                jai_unit.clone(),
                se_unit.clone(),
                goha_unit.clone(),
                word_unit.clone(),
            )))
            .map(|((nahe, free_modifiers), inner_unit)| {
                new!(TanruUnitSyntax::ScalarNegatedTanruUnit {
                    nahe: wrapped_word(nahe, free_modifiers),
                    inner_unit: Box::new(inner_unit),
                })
            });
        let be_link = be_link_parser(sumti.clone(), free_modifier.clone());
        let selbri_relative_clause = cmavo(Cmavo::Nohoi)
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .then(inner_selbri.clone())
            .then(
                cmavo(Cmavo::Kuhoi)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .or_not(),
            )
            .map(|(((nohoi, leading_free_modifiers), selbri), kuhoi)| {
                new!(SelbriRelativePhraseSyntax {
                    nohoi: WithFreeModifiers::new(nohoi, leading_free_modifiers),
                    selbri,
                    kuhoi: kuhoi.map(|(kuhoi, free_modifiers)| {
                        WithFreeModifiers::new(kuhoi, free_modifiers)
                    }),
                })
            })
            .boxed();

        let base_unit = if zantufa_quotes_enabled {
            choice((
                goha_raho_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                muhoi_unit.clone(),
                luhei_unit.clone(),
                se_abstraction_unit.clone(),
                abstraction_subbridi_unit.clone(),
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
            .map(Box::new)
            .boxed()
        } else {
            choice((
                goha_raho_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                se_abstraction_unit.clone(),
                abstraction_subbridi_unit.clone(),
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
            .map(Box::new)
            .boxed()
        };
        let base_unit_for_cei = if zantufa_quotes_enabled {
            choice((
                goha_raho_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                muhoi_unit.clone(),
                luhei_unit.clone(),
                se_abstraction_unit,
                abstraction_subbridi_unit,
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
            .map(Box::new)
            .boxed()
        } else {
            choice((
                goha_raho_unit.clone(),
                me_unit.clone(),
                mehoi_unit.clone(),
                gohoi_unit.clone(),
                se_abstraction_unit,
                abstraction_subbridi_unit,
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
            .map(Box::new)
            .boxed()
        };
        let linked_unit_from = |base_unit: BoxedParser<'tokens, BoxedTanruUnitSyntax>| {
            base_unit
                .then(be_link.clone().or_not())
                .map(|(base, be_link)| match be_link {
                    None => base,
                    Some(link) => {
                        let data!(LinkedSumtiListSyntax {
                            be,
                            fa,
                            first_sumti,
                            bei_links,
                            beho,
                        }) = link.into_data();

                        Box::new(new!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                            base,
                            be,
                            fa,
                            first_sumti,
                            bei_links,
                            beho,
                        }))
                    }
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
                        Box::new(new!(TanruUnitSyntax::RelativeClauses {
                            base: linked_unit,
                            selbri_relative_clauses,
                        }))
                    }
                })
                .boxed()
        };
        let preposed_unit = be_link.clone().then(base_unit.clone()).map(|(link, base)| {
            let data!(LinkedSumtiListSyntax {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
            }) = link.into_data();

            Box::new(new!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                be,
                fa,
                first_sumti,
                bei_links,
                beho,
                base,
            }))
        });
        let linked_unit = linked_unit_from(base_unit);
        let linked_unit_for_cei = linked_unit_from(base_unit_for_cei);
        let cei_unit = linked_unit_for_cei
            .clone()
            .then(
                cmavo(Cmavo::Cei)
                    .then(linked_unit_for_cei.clone())
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(base, be_link)| {
                Box::new(new!(TanruUnitSyntax::AssignedProBridi {
                    base,
                    assignments: be_link
                        .into_iter()
                        .map(|(cei, tanru_unit)| new!(ProBridiAssignmentSyntax {
                            cei: wrapped_word(cei, Vec::new()),
                            tanru_unit,
                        }))
                        .collect(),
                }))
            })
            .boxed();
        let bo_unit: BoxedParser<'tokens, BoxedTanruUnitSyntax> =
            recursive::<_, BoxedTanruUnitSyntax, _, _, _>(|bo_unit| {
                let guha_unit = guhek_connective()
                    .then(inner_selbri.clone())
                    .then(gik_connective_with_free_modifiers(free_modifier.clone()))
                    .then(bo_unit.clone())
                    .then(optional_gihi_terminator())
                    .map(|((((guhek, leading_selbri), gik), trailing_unit), gihi)| {
                        Box::new(new!(TanruUnitSyntax::SelbriGroupTanruUnit(Box::new(new!(
                            SelbriSyntax::ForethoughtSelbriConnection {
                                guhek,
                                leading_bridi: Box::new(relation_to_empty_predicate(
                                    *leading_selbri,
                                )),
                                gik,
                                trailing_bridi: Box::new(relation_to_empty_predicate(
                                    relation_unit_to_relation(&trailing_unit),
                                )),
                                gihi,
                            }
                        )))))
                    });
                let atom_unit = choice((
                    guha_unit,
                    preposed_unit.clone(),
                    cei_unit.clone(),
                    linked_unit.clone(),
                ))
                .boxed();
                let connected_bo_tail = statement_connective()
                    .then(tense_modal_boxed().or_not())
                    .then(cmavo(Cmavo::Bo))
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(bo_unit.clone())
                    .map(
                        |((((connective, bo_tense_modal), bo), free_modifiers), trailing_unit)| {
                            Box::new(BoTanruUnitTailSyntax {
                                connective: Some(Box::new(connective)),
                                tense_modal: bo_tense_modal,
                                bo,
                                free_modifiers,
                                trailing_unit,
                            })
                        },
                    );
                let bare_bo_tail = cmavo(Cmavo::Bo)
                    .then(free_modifier.clone().repeated().collect::<Vec<_>>())
                    .then(bo_unit)
                    .map(|((bo, free_modifiers), trailing_unit)| {
                        Box::new(BoTanruUnitTailSyntax {
                            connective: None,
                            tense_modal: None,
                            bo,
                            free_modifiers,
                            trailing_unit,
                        })
                    });
                atom_unit
                    .then(choice((connected_bo_tail, bare_bo_tail)).or_not())
                    .map(|(leading_unit, bo_tail)| match bo_tail {
                        None => leading_unit,
                        Some(bo_tail) => {
                            let BoTanruUnitTailSyntax {
                                connective,
                                tense_modal,
                                bo,
                                free_modifiers,
                                trailing_unit,
                            } = *bo_tail;
                            Box::new(new!(TanruUnitSyntax::BoundTanruUnitConnection {
                                leading_unit,
                                bo_connective: connective,
                                bo_tense_modal: tense_modal,
                                bo: wrapped_word(bo, free_modifiers),
                                trailing_unit,
                            }))
                        }
                    })
            })
            .boxed();
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
                    |leading_unit, (connective, trailing_unit)| {
                        Box::new(new!(TanruUnitSyntax::TanruUnitConnection {
                            leading_unit,
                            connective,
                            trailing_unit,
                        }))
                    },
                )
            })
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(boxed_selbri_from_boxed_units)
    })
    .boxed()
}

#[requires(!units.is_empty(), "selbri unit sequences must be non-empty")]
#[ensures(true)]
fn boxed_selbri_from_boxed_units(units: Vec<BoxedTanruUnitSyntax>) -> BoxedSelbriSyntax {
    Box::new(selbri_from_boxed_units(units))
}

#[requires(true)]
#[ensures(ret.len() == old(terms.len()))]
fn unbox_terms(terms: Vec<BoxedTermSyntax>) -> Vec<TermSyntax> {
    terms.into_iter().map(|term| *term).collect()
}

#[requires(!units.is_empty(), "selbri unit sequences must be non-empty")]
#[ensures(true)]
fn selbri_from_boxed_units(units: Vec<BoxedTanruUnitSyntax>) -> SelbriSyntax {
    relation_from_units(units.into_iter().map(|unit| *unit).collect())
}

#[requires(!units.is_empty(), "selbri unit sequences must be non-empty")]
#[ensures(true)]
fn relation_from_units(units: Vec<TanruUnitSyntax>) -> SelbriSyntax {
    if let [unit] = units.as_slice() {
        match unit.as_data() {
            data!(TanruUnitSyntax::TanruUnitWord(word)) if word.free_modifiers.is_empty() => {
                return new!(SelbriSyntax::SelbriWord(word.value.clone()));
            }
            data!(TanruUnitSyntax::ProBridi { goha, raho: None })
                if goha.free_modifiers.is_empty() =>
            {
                return new!(SelbriSyntax::SelbriWord(goha.value.clone()));
            }
            data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
                return new!(SelbriSyntax::ConvertedSelbri {
                    se: se.clone(),
                    inner_selbri: Box::new(relation_unit_to_relation(inner_unit.as_ref())),
                });
            }
            data!(TanruUnitSyntax::GroupedTanruUnit {
                ke_tense_modal,
                ke,
                selbri,
                kehe,
            }) => {
                return new!(SelbriSyntax::GroupedSelbri {
                    ke_tense_modal: ke_tense_modal.clone(),
                    ke: ke.clone(),
                    selbri: selbri.clone(),
                    kehe: kehe.clone(),
                });
            }
            data!(TanruUnitSyntax::Abstraction(abstraction)) => {
                return new!(SelbriSyntax::Abstraction(abstraction.clone()));
            }
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                bo_connective,
                bo_tense_modal,
                bo,
                trailing_unit,
            }) => {
                return new!(SelbriSyntax::BoundSelbriConnection {
                    leading_selbri: Box::new(relation_unit_to_relation(leading_unit)),
                    bo_connective: bo_connective.clone(),
                    bo_tense_modal: bo_tense_modal.clone(),
                    bo: bo.clone(),
                    trailing_selbri: Box::new(relation_unit_to_relation(trailing_unit)),
                });
            }
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => {
                return new!(SelbriSyntax::SelbriConnection {
                    connective: connective.clone(),
                    leading_selbri: Box::new(relation_unit_to_relation(leading_unit)),
                    trailing_selbri: Box::new(relation_unit_to_relation(trailing_unit)),
                });
            }
            data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => return *selbri.clone(),
            _ => {}
        }
    }
    new!(SelbriSyntax::Tanru(Box::new(relation_unit_vec(units))))
}

#[requires(!units.is_empty())]
#[ensures(!ret.is_empty())]
fn relation_unit_vec(units: Vec<TanruUnitSyntax>) -> TanruUnitVec {
    TanruUnitVec::try_from_vec(units).expect("precondition guarantees non-empty units")
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_to_relation(unit: &TanruUnitSyntax) -> SelbriSyntax {
    match unit.as_data() {
        data!(TanruUnitSyntax::TanruUnitWord(word)) if word.free_modifiers.is_empty() => {
            new!(SelbriSyntax::SelbriWord(word.value.clone()))
        }
        data!(TanruUnitSyntax::ProBridi { goha, raho: None }) if goha.free_modifiers.is_empty() => {
            new!(SelbriSyntax::SelbriWord(goha.value.clone()))
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
            new!(SelbriSyntax::ConvertedSelbri {
                se: se.clone(),
                inner_selbri: Box::new(relation_unit_to_relation(inner_unit)),
            })
        }
        data!(TanruUnitSyntax::GroupedTanruUnit {
            ke_tense_modal,
            ke,
            selbri,
            kehe,
        }) => new!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal: ke_tense_modal.clone(),
            ke: ke.clone(),
            selbri: selbri.clone(),
            kehe: kehe.clone(),
        }),
        data!(TanruUnitSyntax::Abstraction(abstraction)) => {
            new!(SelbriSyntax::Abstraction(abstraction.clone()))
        }
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            bo_connective,
            bo_tense_modal,
            bo,
            trailing_unit,
        }) => new!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri: Box::new(relation_unit_to_relation(leading_unit)),
            bo_connective: bo_connective.clone(),
            bo_tense_modal: bo_tense_modal.clone(),
            bo: bo.clone(),
            trailing_selbri: Box::new(relation_unit_to_relation(trailing_unit)),
        }),
        data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            connective,
            trailing_unit,
        }) => new!(SelbriSyntax::SelbriConnection {
            connective: connective.clone(),
            leading_selbri: Box::new(relation_unit_to_relation(leading_unit)),
            trailing_selbri: Box::new(relation_unit_to_relation(trailing_unit)),
        }),
        data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => *selbri.clone(),
        unit => new!(SelbriSyntax::Tanru(Box::new(TanruUnitVec::new(
            TanruUnitSyntax::from_data(unit.clone())
        )))),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_to_empty_predicate(selbri: SelbriSyntax) -> BridiSyntax {
    new!(BridiSyntax {
        leading_terms: Vec::new(),
        cu: None,
        bridi_tail: Box::new(BridiTailSyntax {
            first: Box::new(AfterthoughtBridiTailSyntax {
                first: Box::new(BoGroupedBridiTailSyntax {
                    first: Box::new(new!(SimpleBridiTailSyntax::SelbriBridiTail {
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

#[requires(true)]
#[ensures(true)]
fn fiho_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let selbri = recursive(|selbri| {
        let word_unit = relation_word().map(|word| {
            new!(TanruUnitSyntax::TanruUnitWord(wrapped_word(
                word,
                Vec::new()
            )))
        });
        let se_unit = selmaho(Selmaho::Se)
            .then(word_unit.clone())
            .map(|(se, inner_unit)| {
                new!(TanruUnitSyntax::ConvertedTanruUnit {
                    se: wrapped_word(se, Vec::new()),
                    inner_unit: Box::new(inner_unit),
                })
            });
        let ke_unit = cmavo(Cmavo::Ke)
            .then(selbri.clone())
            .then(cmavo(Cmavo::Kehe).or_not())
            .map(|((ke, selbri), kehe)| {
                new!(TanruUnitSyntax::GroupedTanruUnit {
                    ke_tense_modal: None,
                    ke: wrapped_word(ke, Vec::new()),
                    selbri: Box::new(selbri),
                    kehe: kehe.map(|kehe| wrapped_word(kehe, Vec::new())),
                })
            });
        let simple_unit = choice((ke_unit, se_unit, word_unit)).boxed();
        let bo_unit = recursive(|bo_unit| {
            simple_unit
                .clone()
                .then(cmavo(Cmavo::Bo).then(bo_unit).or_not())
                .map(|(leading_unit, bo_tail)| match bo_tail {
                    None => leading_unit,
                    Some((bo, trailing_unit)) => new!(TanruUnitSyntax::BoundTanruUnitConnection {
                        leading_unit: Box::new(leading_unit),
                        bo_connective: None,
                        bo_tense_modal: None,
                        bo: wrapped_word(bo, Vec::new()),
                        trailing_unit: Box::new(trailing_unit),
                    }),
                })
        })
        .boxed();
        bo_unit
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(relation_from_units)
    });

    cmavo(Cmavo::Fiho)
        .then(selbri)
        .then(cmavo(Cmavo::Fehu).or_not())
        .map(|((fiho, selbri), fehu)| {
            new!(TenseModalSyntax::AdHocModal {
                fiho: WithFreeModifiers::new(fiho, Vec::new()),
                selbri: Box::new(selbri),
                fehu: fehu.map(|fehu| WithFreeModifiers::new(fehu, Vec::new())),
            })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn flat_tag_chunk_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    let dialect = parser_dialect_config();
    let prefixes = selmaho(Selmaho::Nahe)
        .then(selmaho(Selmaho::Se).or_not())
        .map(|(nahe, se)| {
            let mut leaves = vec![nahe];
            leaves.extend(se);
            leaves
        })
        .or(selmaho(Selmaho::Se).map(|se| vec![se]));
    let zantufa_prefix = choice((
        cmavo(Cmavo::Nahe),
        cmavo(Cmavo::Tohe),
        cmavo(Cmavo::Nohe),
        cmavo(Cmavo::Jeha),
        cmavo(Cmavo::Se),
        cmavo(Cmavo::Te),
        cmavo(Cmavo::Ve),
        cmavo(Cmavo::Xe),
    ));
    let atom = choice((
        selmaho(Selmaho::Fa).map(|fa| (vec![fa.clone()], Some(fa))),
        simple_tense_modal().map(|tense_modal| (tense_modal.leaf_words(), None)),
        composite_tense_modal().map(|tense_modal| (tense_modal.leaf_words(), None)),
    ));

    let prefixed = prefixes.then(atom.clone()).map_with(
        |(mut prefix_leaves, (atom_leaves, fa)),
         extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            let anchor = prefix_leaves
                .first()
                .expect("flat tag prefixes parser produces at least one word");
            extra
                .state()
                .warn(ExperimentalConstruct::ExperimentalFlattenedTag, anchor);
            if let Some(fa) = &fa {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalFaAsTag, fa);
            }
            prefix_leaves.extend(atom_leaves);
            tense_modal_from_leaves(prefix_leaves, Vec::new())
        },
    );
    let fa = selmaho(Selmaho::Fa).map_with(
        |fa, extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            extra
                .state()
                .warn(ExperimentalConstruct::ExperimentalFaAsTag, &fa);
            tense_modal_from_leaves(vec![fa], Vec::new())
        },
    );
    let zantufa_recursive = zantufa_prefix
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(atom)
        .map_with(|(mut prefix_leaves, (atom_leaves, fa)), extra: &mut MapExtra<'tokens, '_, ParserInput<'tokens>, ParseExtra<'tokens>>| {
            let anchor = prefix_leaves
                .first()
                .expect("Zantufa recursive tag prefixes parser produces at least one word");
            extra
                .state()
                .warn(ExperimentalConstruct::ExperimentalZantufaRecursiveTag, anchor);
            if let Some(fa) = &fa {
                extra
                    .state()
                    .warn(ExperimentalConstruct::ExperimentalFaAsTag, fa);
            }
            prefix_leaves.extend(atom_leaves);
            tense_modal_from_leaves(prefix_leaves, Vec::new())
        });

    if dialect.zantufa_tags_enabled {
        choice((prefixed, fa, zantufa_recursive)).boxed()
    } else {
        choice((prefixed, fa)).boxed()
    }
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    composite_tense_modal_boxed()
        .map(|tense_modal| *tense_modal)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal_boxed<'tokens>() -> BoxedParser<'tokens, BoxedTenseModalSyntax> {
    let pu = selmaho(Selmaho::Pu)
        .then(cmavo(Cmavo::Nai).or_not())
        .then(selmaho(Selmaho::Zi).or_not())
        .map(|((pu, nai), distance)| {
            let mut leaves = vec![pu];
            leaves.extend(nai);
            leaves.extend(distance);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let zi = selmaho(Selmaho::Zi)
        .map(|zi| boxed_tense_modal_from_leaves(vec![zi], Vec::new()))
        .boxed();
    let faha = selmaho(Selmaho::Faha)
        .then(cmavo(Cmavo::Nai).or_not())
        .then(selmaho(Selmaho::Va).or_not())
        .map(|((faha, nai), distance)| {
            let mut leaves = vec![faha];
            leaves.extend(nai);
            leaves.extend(distance);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let va = selmaho(Selmaho::Va)
        .map(|va| boxed_tense_modal_from_leaves(vec![va], Vec::new()))
        .boxed();
    let numbered_interval_start = number_words()
        .then(selmaho(Selmaho::Roi))
        .rewind()
        .ignored();
    let numbered_interval = numbered_interval_start
        .ignore_then(number_words())
        .then(selmaho(Selmaho::Roi))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let number = word_run(number);
            let mut leaves = word_run_leaves(&number);
            leaves.push(roi_or_tahe);
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let tahe_interval = selmaho(Selmaho::Tahe)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(roi_or_tahe, nai)| {
            let mut leaves = vec![roi_or_tahe];
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let caha = selmaho(Selmaho::Caha)
        .map(|caha| boxed_tense_modal_from_leaves(vec![caha], Vec::new()))
        .boxed();
    let zaho = selmaho(Selmaho::Zaho)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(zaho, nai)| {
            let mut leaves = vec![zaho];
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let ki = cmavo(Cmavo::Ki)
        .map(|ki| boxed_tense_modal_from_leaves(vec![ki], Vec::new()))
        .boxed();
    let cuhe = selmaho(Selmaho::Cuhe)
        .map(|cuhe| boxed_tense_modal_from_leaves(vec![cuhe], Vec::new()))
        .boxed();

    let zeha_clause = selmaho(Selmaho::Zeha)
        .then(
            selmaho(Selmaho::Pu)
                .then(cmavo(Cmavo::Nai).or_not())
                .or_not(),
        )
        .map(|(zeha, pu_nai)| {
            let mut leaves = vec![zeha];
            if let Some((pu, nai)) = pu_nai {
                leaves.push(pu);
                leaves.extend(nai);
            }
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let interval_property = choice((numbered_interval, tahe_interval, zaho)).boxed();
    let time_offset = pu;
    let time_tense_with_zi = zi
        .clone()
        .then(time_offset.clone().repeated().collect::<Vec<_>>())
        .then(zeha_clause.clone().or_not())
        .then(interval_property.clone().repeated().collect::<Vec<_>>())
        .map(|(((zi, offsets), zeha), props)| {
            let mut parts = vec![zi];
            parts.extend(offsets);
            parts.extend(zeha);
            parts.extend(props);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let time_tense_with_offset = zi
        .clone()
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
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let time_tense_with_interval = zi
        .clone()
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
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let time_tense_with_properties = zi
        .or_not()
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
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let time_tense = choice((
        time_tense_with_zi,
        time_tense_with_offset,
        time_tense_with_interval,
        time_tense_with_properties,
    ))
    .boxed();

    let space_offset = faha;
    let veha_viha = selmaho(Selmaho::Veha)
        .then(selmaho(Selmaho::Viha).or_not())
        .map(|(veha, viha)| {
            let mut leaves = vec![veha];
            leaves.extend(viha);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .or(selmaho(Selmaho::Viha)
            .map(|viha| boxed_tense_modal_from_leaves(vec![viha], Vec::new())))
        .boxed();
    let faha_nai = selmaho(Selmaho::Faha)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(faha, nai)| {
            let mut leaves = vec![faha];
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        })
        .boxed();
    let fehe_interval_property = cmavo(Cmavo::Fehe)
        .then(interval_property)
        .map(|(fehe, interval)| {
            combine_boxed_composite_tense_modals(vec![
                boxed_tense_modal_from_leaves(vec![fehe], Vec::new()),
                interval,
            ])
        })
        .boxed();
    let space_interval_properties = fehe_interval_property
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(combine_boxed_composite_tense_modals)
        .boxed();
    let space_interval_with_extent = veha_viha
        .then(faha_nai.or_not())
        .then(space_interval_properties.clone().or_not())
        .map(|((vv, faha), props)| {
            let mut parts = vec![vv];
            parts.extend(faha);
            parts.extend(props);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_interval = space_interval_with_extent
        .or(space_interval_properties)
        .boxed();
    let mohi_offset = selmaho(Selmaho::Mohi)
        .then(space_offset.clone())
        .map(|(mohi, offset)| {
            combine_boxed_composite_tense_modals(vec![
                boxed_tense_modal_from_leaves(vec![mohi], Vec::new()),
                offset,
            ])
        })
        .boxed();
    let space_tense_with_va = va
        .clone()
        .then(space_offset.clone().repeated().collect::<Vec<_>>())
        .then(space_interval.clone().or_not())
        .then(mohi_offset.clone().or_not())
        .map(|(((va, offsets), interval), mohi)| {
            let mut parts = vec![va];
            parts.extend(offsets);
            parts.extend(interval);
            parts.extend(mohi);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_tense_with_offset = va
        .clone()
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
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_tense_with_interval = va
        .clone()
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
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_tense_with_mohi = va
        .or_not()
        .then(space_offset.repeated().collect::<Vec<_>>())
        .then(space_interval.or_not())
        .then(mohi_offset)
        .map(|(((va, offsets), interval), mohi)| {
            let mut parts = Vec::new();
            parts.extend(va);
            parts.extend(offsets);
            parts.extend(interval);
            parts.push(mohi);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_tense = choice((
        space_tense_with_va,
        space_tense_with_offset,
        space_tense_with_interval,
        space_tense_with_mohi,
    ))
    .boxed();

    let time_then_space_caha = time_tense
        .clone()
        .then(space_tense.clone().or_not())
        .then(caha.clone().or_not())
        .map(|((time, space), caha)| {
            let mut parts = vec![time];
            parts.extend(space);
            parts.extend(caha);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let space_then_time_caha = space_tense
        .then(time_tense.or_not())
        .then(caha.or_not())
        .map(|((space, time), caha)| {
            let mut parts = vec![space];
            parts.extend(time);
            parts.extend(caha);
            combine_boxed_composite_tense_modals(parts)
        })
        .boxed();
    let bare_caha = selmaho(Selmaho::Caha)
        .map(|caha| boxed_tense_modal_from_leaves(vec![caha], Vec::new()))
        .boxed();
    let time_space_caha = choice((time_then_space_caha, space_then_time_caha, bare_caha)).boxed();
    let nahe_before_time_space_caha = selmaho(Selmaho::Nahe)
        .then(time_space_caha.clone().rewind().ignored())
        .rewind()
        .ignore_then(selmaho(Selmaho::Nahe));

    nahe_before_time_space_caha
        .or_not()
        .then(time_space_caha)
        .then(ki.or_not())
        .map(|((nahe, tense), ki)| {
            let tense = match nahe {
                Some(nahe) => prefix_boxed_tense_modal_nahe(nahe, tense),
                None => tense,
            };
            if let Some(ki) = ki {
                combine_boxed_composite_tense_modals(vec![tense, ki])
            } else {
                tense
            }
        })
        .or(cuhe)
        .boxed()
}

#[requires(matches!(
    modal.as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
#[ensures(matches!(
    ret.as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
fn prefix_tense_modal_nahe(nahe: Token, modal: TenseModalSyntax) -> TenseModalSyntax {
    let data!(TenseModalSyntax::Composite { mut parts }) = modal.into_data() else {
        unreachable!("prefix_tense_modal_nahe requires a composite tense modal")
    };
    parts
        .value
        .insert(0, new!(CompositeTenseModalPartSyntax::Cmavo(nahe)));
    new!(TenseModalSyntax::Composite { parts })
}

#[requires(matches!(
    modal.as_ref().as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
#[ensures(matches!(
    ret.as_ref().as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
fn prefix_boxed_tense_modal_nahe(
    nahe: Token,
    modal: BoxedTenseModalSyntax,
) -> BoxedTenseModalSyntax {
    Box::new(prefix_tense_modal_nahe(nahe, *modal))
}

#[requires(!parts.is_empty())]
#[ensures(matches!(
    ret.as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
fn combine_composite_tense_modals(parts: Vec<TenseModalSyntax>) -> TenseModalSyntax {
    let mut combined_parts = Vec::new();
    let mut free_modifiers = Vec::new();

    for part in parts {
        if let data!(TenseModalSyntax::Composite { parts }) = part.into_data() {
            combined_parts.extend(parts.value);
            free_modifiers.extend(parts.free_modifiers);
        }
    }

    new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(combined_parts, free_modifiers),
    })
}

#[requires(!parts.is_empty())]
#[ensures(matches!(
    ret.as_ref().as_data(),
    data!(TenseModalSyntax::Composite { .. })
))]
fn combine_boxed_composite_tense_modals(
    parts: Vec<BoxedTenseModalSyntax>,
) -> BoxedTenseModalSyntax {
    let mut combined_parts = Vec::new();
    let mut free_modifiers = Vec::new();

    for part in parts {
        if let data!(TenseModalSyntax::Composite { parts }) = (*part).into_data() {
            combined_parts.extend(parts.value);
            free_modifiers.extend(parts.free_modifiers);
        }
    }

    Box::new(new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(combined_parts, free_modifiers),
    }))
}

#[requires(true)]
#[ensures(true)]
fn leading_term_tag_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    leading_term_tag_tense_modal_boxed()
        .map(|tense_modal| *tense_modal)
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn leading_term_tag_tense_modal_boxed<'tokens>() -> BoxedParser<'tokens, BoxedTenseModalSyntax> {
    let pu_before_nahe = selmaho(Selmaho::Pu)
        .then(cmavo(Cmavo::Nai).or_not())
        .then(selmaho(Selmaho::Nahe).rewind().ignored())
        .map(|((pu, nai), _)| {
            let mut leaves = vec![pu];
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        });
    let pu_distance_before_tag = selmaho(Selmaho::Pu)
        .then(cmavo(Cmavo::Nai).or_not())
        .then(selmaho(Selmaho::Zi))
        .then(selmaho(Selmaho::Zi).rewind())
        .map(|(((pu, nai), distance), _)| {
            let mut leaves = vec![pu];
            leaves.extend(nai);
            leaves.push(distance);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        });
    let zi_before_zi = selmaho(Selmaho::Zi)
        .then(selmaho(Selmaho::Zi).rewind())
        .map(|(zi, _)| boxed_tense_modal_from_leaves(vec![zi], Vec::new()));
    let va_before_va = selmaho(Selmaho::Va)
        .then(selmaho(Selmaho::Va).rewind())
        .map(|(va, _)| boxed_tense_modal_from_leaves(vec![va], Vec::new()));
    let mohi_before_mohi = selmaho(Selmaho::Mohi)
        .then(selmaho(Selmaho::Faha))
        .then(cmavo(Cmavo::Nai).or_not())
        .then(selmaho(Selmaho::Va).or_not())
        .then(selmaho(Selmaho::Mohi).rewind())
        .map(|((((mohi, direction), nai), distance), _)| {
            let mut leaves = vec![mohi, direction];
            leaves.extend(nai);
            leaves.extend(distance);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        });
    let zaho_property =
        selmaho(Selmaho::Zaho)
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|(zaho, nai)| {
                let mut leaves = vec![zaho];
                leaves.extend(nai);
                boxed_tense_modal_from_leaves(leaves, Vec::new())
            });
    let numbered_interval_start = number_words()
        .then(selmaho(Selmaho::Roi))
        .rewind()
        .ignored();
    let numbered_interval = numbered_interval_start
        .ignore_then(number_words())
        .then(selmaho(Selmaho::Roi))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|((number, roi_or_tahe), nai)| {
            let number = word_run(number);
            let mut leaves = word_run_leaves(&number);
            leaves.push(roi_or_tahe);
            leaves.extend(nai);
            boxed_tense_modal_from_leaves(leaves, Vec::new())
        });
    let tahe_interval =
        selmaho(Selmaho::Tahe)
            .then(cmavo(Cmavo::Nai).or_not())
            .map(|(roi_or_tahe, nai)| {
                let mut leaves = vec![roi_or_tahe];
                leaves.extend(nai);
                boxed_tense_modal_from_leaves(leaves, Vec::new())
            });
    let caha_before_tag = selmaho(Selmaho::Caha)
        .then(tense_modal_boxed().rewind().ignored())
        .map(|(caha, _)| {
            Box::new(new!(TenseModalSyntax::Actuality(WithFreeModifiers::new(
                caha,
                Vec::new()
            ))))
        });
    let property_split_follower = choice((
        selmaho(Selmaho::Pu).ignored(),
        selmaho(Selmaho::Zi).ignored(),
        selmaho(Selmaho::Zeha).ignored(),
        selmaho(Selmaho::Nahe)
            .then(selmaho(Selmaho::Caha))
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
        tense_modal_boxed(),
    ))
    .boxed()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    tense_modal_boxed().map(|tense_modal| *tense_modal).boxed()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_boxed<'tokens>() -> BoxedParser<'tokens, BoxedTenseModalSyntax> {
    let atom = tense_modal_atom_boxed();
    atom.clone()
        .then(
            choice((joik_connective(), jek_connective()))
                .then(atom)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, continuations)| combine_connected_boxed_tense_modals(first, continuations))
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn combine_connected_boxed_tense_modals(
    first: BoxedTenseModalSyntax,
    continuations: Vec<(ConnectiveSyntax, BoxedTenseModalSyntax)>,
) -> BoxedTenseModalSyntax {
    if continuations.is_empty() {
        return first;
    }

    let mut parts = vec![Box::new(tense_modal_as_composite(*first))];
    for (connective, tense_modal) in continuations {
        parts.push(Box::new(connective_tense_modal_from_leaves(
            connective_tense_modal_leaves(connective),
        )));
        parts.push(Box::new(tense_modal_as_composite(*tense_modal)));
    }
    combine_boxed_composite_tense_modals(parts)
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_atom_boxed<'tokens>() -> BoxedParser<'tokens, BoxedTenseModalSyntax> {
    #[invariant(true)]
    #[invariant(::Distance(distance) => distance.is_selmaho(Selmaho::Zi))]
    #[invariant(::Caha(caha) => caha.is_selmaho(Selmaho::Caha))]
    #[derive(Clone)]
    enum PuTail {
        Distance(Token),
        Caha(Token),
    }

    let pu_tail = choice((
        selmaho(Selmaho::Zi).map(|distance| new!(PuTail::Distance(distance))),
        selmaho(Selmaho::Caha).map(|caha| new!(PuTail::Caha(caha))),
    ))
    .boxed();
    let pu = selmaho(Selmaho::Pu)
        .then(pu_tail.or_not())
        .map(|(pu, tail)| match tail.map(|tail| tail.into_data()) {
            Some(data!(PuTail::Distance(distance))) => {
                Box::new(new!(TenseModalSyntax::TimeDirectionDistance {
                    pu,
                    distance: WithFreeModifiers::new(distance, Vec::new()),
                }))
            }
            Some(data!(PuTail::Caha(caha))) => {
                Box::new(new!(TenseModalSyntax::TimeDirectionActuality {
                    pu,
                    caha: WithFreeModifiers::new(caha, Vec::new()),
                }))
            }
            None => Box::new(new!(TenseModalSyntax::TimeDirection(
                WithFreeModifiers::new(pu, Vec::new())
            ))),
        })
        .boxed();
    let va = selmaho(Selmaho::Va)
        .map(|word| {
            Box::new(new!(TenseModalSyntax::SpaceDistance(
                WithFreeModifiers::new(word, Vec::new())
            )))
        })
        .boxed();
    let zeha = selmaho(Selmaho::Zeha)
        .map(|word| {
            Box::new(new!(TenseModalSyntax::TimeInterval(
                WithFreeModifiers::new(word, Vec::new())
            )))
        })
        .boxed();
    let faha = selmaho(Selmaho::Faha)
        .map(|word| {
            Box::new(new!(TenseModalSyntax::SpaceDirection(
                WithFreeModifiers::new(word, Vec::new())
            )))
        })
        .boxed();
    let mohi = selmaho(Selmaho::Mohi)
        .then(selmaho(Selmaho::Faha))
        .then(selmaho(Selmaho::Va).or_not())
        .map(|((mohi, direction), distance)| {
            Box::new(new!(TenseModalSyntax::SpaceMovement {
                mohi,
                direction: WithFreeModifiers::new(direction, Vec::new()),
                distance: distance.map(|distance| WithFreeModifiers::new(distance, Vec::new())),
            }))
        })
        .boxed();
    let caha = selmaho(Selmaho::Caha)
        .map(|word| {
            Box::new(new!(TenseModalSyntax::Actuality(WithFreeModifiers::new(
                word,
                Vec::new()
            ))))
        })
        .boxed();
    let fiho = fiho_tense_modal().map(Box::new).boxed();
    let zaho = selmaho(Selmaho::Zaho)
        .map(|word| {
            Box::new(new!(TenseModalSyntax::EventContour(
                WithFreeModifiers::new(vec![word], Vec::new())
            )))
        })
        .boxed();
    let simple = simple_tense_modal().map(Box::new).boxed();
    let flat_tag_chunk = flat_tag_chunk_tense_modal().map(Box::new).boxed();
    let ki = cmavo(Cmavo::Ki)
        .map(|ki| {
            Box::new(new!(TenseModalSyntax::Sticky(WithFreeModifiers::new(
                ki,
                Vec::new()
            ))))
        })
        .boxed();
    let numbered_interval = pa_word()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(selmaho(Selmaho::Roi).or(selmaho(Selmaho::Tahe)))
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|((number, roi_or_tahe), nai)| {
            Box::new(new!(TenseModalSyntax::IntervalProperty {
                number: Some(word_run(number)),
                roi_or_tahe: WithFreeModifiers::new(roi_or_tahe, Vec::new()),
                nai: nai.map(|nai| WithFreeModifiers::new(nai, Vec::new())),
            }))
        })
        .boxed();
    let tahe = selmaho(Selmaho::Tahe)
        .then(cmavo(Cmavo::Nai).or_not())
        .map(|(roi_or_tahe, nai)| {
            Box::new(new!(TenseModalSyntax::IntervalProperty {
                number: None,
                roi_or_tahe: WithFreeModifiers::new(roi_or_tahe, Vec::new()),
                nai: nai.map(|nai| WithFreeModifiers::new(nai, Vec::new())),
            }))
        })
        .boxed();

    let structural_atoms =
        choice((composite_tense_modal_boxed(), pu, va, zeha, faha, mohi)).boxed();
    let tag_atoms = choice((caha, fiho, zaho, simple, flat_tag_chunk, ki)).boxed();
    let interval_atoms = choice((numbered_interval, tahe)).boxed();
    choice((structural_atoms, tag_atoms, interval_atoms)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn simple_tense_modal<'tokens>() -> BoxedParser<'tokens, TenseModalSyntax> {
    selmaho(Selmaho::Nahe)
        .or_not()
        .then(selmaho(Selmaho::Se).or_not())
        .then(selmaho(Selmaho::Bai))
        .then(cmavo(Cmavo::Nai).or_not())
        .then(cmavo(Cmavo::Ki).or_not())
        .map(|((((nahe, se), bai), nai), ki)| {
            new!(TenseModalSyntax::Modal {
                nahe: nahe.map(|nahe| WithFreeModifiers::new(nahe, Vec::new())),
                se: se.map(|se| WithFreeModifiers::new(se, Vec::new())),
                bai: WithFreeModifiers::new(bai, Vec::new()),
                nai: nai.map(|nai| WithFreeModifiers::new(nai, Vec::new())),
                ki: ki.map(|ki| WithFreeModifiers::new(ki, Vec::new())),
            })
        })
        .boxed()
}

#[requires(true)]
#[ensures(true)]
fn link_argument_parser<'tokens, A, F>(
    sumti: A,
    free_modifier: F,
) -> BoxedParser<'tokens, LinkedSumtiSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let argument_base = sumti
        .clone()
        .or(na_ku_argument_parser(free_modifier.clone()))
        .boxed();
    let fa_tail = argument_base
        .clone()
        .map(|sumti| (Some(sumti), None, Vec::new()))
        .or(cmavo(Cmavo::Ku)
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)));
    let fa_link_argument = selmaho(Selmaho::Fa)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(fa_tail)
        .map(
            |((fa, fa_free_modifiers), (sumti, maybe_ku, trailing_free_modifiers))| {
                if let Some(sumti) = sumti {
                    new!(LinkedSumtiSyntax {
                        fa: Some(WithFreeModifiers::new(fa, fa_free_modifiers)),
                        sumti: Some(Box::new(sumti)),
                    })
                } else {
                    let tag = new!(SumtiTagSyntax::PlaceTag(WithFreeModifiers::new(
                        fa,
                        fa_free_modifiers
                    )));
                    new!(LinkedSumtiSyntax {
                        fa: None,
                        sumti: Some(Box::new(build_zohe_argument(
                            Some(tag),
                            maybe_ku,
                            trailing_free_modifiers,
                        ))),
                    })
                }
            },
        );
    let tagged_tail = argument_base
        .clone()
        .map(|sumti| (Some(sumti), None, Vec::new()))
        .or(cmavo(Cmavo::Ku)
            .or_not()
            .then(free_modifier.clone().repeated().collect::<Vec<_>>())
            .map(|(maybe_ku, free_modifiers)| (None, maybe_ku, free_modifiers)));
    let tagged_link_argument = tense_modal()
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(tagged_tail)
        .map(
            |((tense_modal, tag_free_modifiers), (sumti, maybe_ku, trailing_free_modifiers))| {
                let tag = new!(SumtiTagSyntax::TenseModal(Box::new(
                    attach_tense_modal_free_modifiers(tense_modal, tag_free_modifiers,)
                )));
                if let Some(sumti) = sumti {
                    new!(LinkedSumtiSyntax {
                        fa: None,
                        sumti: Some(Box::new(new!(SumtiSyntax::TaggedSumti {
                            tag,
                            inner_sumti: Box::new(sumti),
                        }))),
                    })
                } else {
                    new!(LinkedSumtiSyntax {
                        fa: None,
                        sumti: Some(Box::new(build_zohe_argument(
                            Some(tag),
                            maybe_ku,
                            trailing_free_modifiers,
                        ))),
                    })
                }
            },
        );
    let plain_argument = argument_base.map(|sumti| {
        new!(LinkedSumtiSyntax {
            fa: None,
            sumti: Some(Box::new(sumti)),
        })
    });

    choice((fa_link_argument, tagged_link_argument, plain_argument)).boxed()
}

#[requires(true)]
#[ensures(true)]
fn empty_link_argument() -> LinkedSumtiSyntax {
    new!(LinkedSumtiSyntax {
        fa: None,
        sumti: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn be_link_parser<'tokens, A, F>(
    sumti: A,
    free_modifier: F,
) -> BoxedParser<'tokens, LinkedSumtiListSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(sumti.clone(), free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo(Cmavo::Be)
        .then(free_modifier.clone().repeated().collect::<Vec<_>>())
        .then(link_argument)
        .then(
            bei_link_parser(sumti, free_modifier.clone())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(
            cmavo(Cmavo::Beho)
                .then(free_modifier.repeated().collect::<Vec<_>>())
                .or_not(),
        )
        .map(
            |((((be, free_modifiers), link_argument), bei_links), beho)| {
                let data!(LinkedSumtiSyntax { fa, sumti }) = link_argument.into_data();

                new!(LinkedSumtiListSyntax {
                    be: WithFreeModifiers::new(be, free_modifiers),
                    fa,
                    first_sumti: sumti,
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
    sumti: A,
    free_modifier: F,
) -> BoxedParser<'tokens, AdditionalLinkedSumtiSyntax>
where
    A: Parser<'tokens, ParserInput<'tokens>, SumtiSyntax, ParseExtra<'tokens>> + Clone + 'tokens,
    F: Parser<'tokens, ParserInput<'tokens>, FreeModifierSyntax, ParseExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let link_argument = link_argument_parser(sumti, free_modifier.clone())
        .or_not()
        .map(|link_argument| link_argument.unwrap_or_else(empty_link_argument));

    cmavo(Cmavo::Bei)
        .then(free_modifier.repeated().collect::<Vec<_>>())
        .then(link_argument)
        .map(|((bei, bei_free_modifiers), link_argument)| {
            let data!(LinkedSumtiSyntax { fa, sumti }) = link_argument.into_data();

            new!(AdditionalLinkedSumtiSyntax {
                bei: WithFreeModifiers::new(bei, bei_free_modifiers),
                fa,
                sumti,
            })
        })
        .boxed()
}
