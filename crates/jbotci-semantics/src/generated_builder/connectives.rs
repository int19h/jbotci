use super::*;

/// Grammar-directed guard for Zantufa JOIK shapes whose syntax is now typed but
/// whose negation or one-sided endpoint semantics are not yet representable.
#[invariant(
    unsupported_shape
        .as_ref()
        .is_none_or(|description| !description.is_empty()),
    "captured unsupported JOIK descriptions must be non-empty"
)]
#[derive(Clone, Default)]
struct GeneratedZantufaJoikSupportValidator {
    unsupported_shape: Option<String>,
}

impl GeneratedZantufaJoikSupportValidator {
    #[requires(!description.is_empty())]
    #[ensures(self.unsupported_shape.is_some())]
    fn reject(&mut self, description: &str) {
        if self.unsupported_shape.is_none() {
            *self = self.clone().with_data(data! {
                unsupported_shape: Some(description.to_owned())
            });
        }
    }
}

impl<'tree> TreeWalker<'tree> for GeneratedZantufaJoikSupportValidator {
    #[requires(true)]
    #[ensures(true)]
    fn walk_joik_connective(&mut self, node: &'tree JoikConnectiveSyntax) {
        match node {
            JoikConnectiveSyntax::ZantufaNaJoikConnective(_) => {
                self.reject("a Zantufa NA-led JOIK connective")
            }
            JoikConnectiveSyntax::ZantufaGahoJoikConnective(connective) => {
                if connective.na.is_some() {
                    self.reject("a Zantufa GAhO-led JOIK connective with NA")
                } else if connective.right_gaho.is_none() {
                    self.reject("a Zantufa JOIK connective with only a left GAhO endpoint")
                }
            }
            JoikConnectiveSyntax::ZantufaRightGahoJoikConnective(_) => {
                self.reject("a Zantufa JOIK connective with only a right GAhO endpoint")
            }
            _ => {}
        }
        jbotci_syntax::generated_model::walk::joik_connective(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_paragraph_standard_statement_connective(
        &mut self,
        node: &'tree ParagraphStandardStatementConnectiveSyntax,
    ) {
        match node {
            ParagraphStandardStatementConnectiveSyntax::ParagraphZantufaNaJoikConnective(_) => {
                self.reject("a paragraph Zantufa NA-led JOIK connective")
            }
            ParagraphStandardStatementConnectiveSyntax::ParagraphZantufaGahoJoikConnective(
                connective,
            ) => {
                if connective.na.is_some() {
                    self.reject("a paragraph Zantufa GAhO-led JOIK connective with NA")
                } else if connective.right_gaho.is_none() {
                    self.reject(
                        "a paragraph Zantufa JOIK connective with only a left GAhO endpoint",
                    )
                }
            }
            ParagraphStandardStatementConnectiveSyntax::ParagraphZantufaRightGahoJoikConnective(
                _,
            ) => self.reject("a paragraph Zantufa JOIK connective with only a right GAhO endpoint"),
            _ => {}
        }
        jbotci_syntax::generated_model::walk::paragraph_standard_statement_connective(self, node);
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn validate_supported_zantufa_joik_semantics(
    syntax: &TextSyntax,
) -> Result<(), SemanticsError> {
    let mut validator = GeneratedZantufaJoikSupportValidator::default();
    TreeWalkable::walk_with(syntax, &mut validator);
    if let Some(description) = validator.into_data().unsupported_shape {
        return Err(undefined_semantics(&description));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn generated_sumti_connective_operator(
    connective: &SumtiConnectiveSyntax,
) -> Result<CompositionOperator, SemanticsError> {
    if generated_sumti_connective_question_token(connective).is_some() {
        return Ok(CompositionOperator::ConnectiveQuestion);
    }
    if generated_sumti_connective_is_logical(connective) {
        Ok(CompositionOperator::Joint)
    } else {
        generated_nonlogical_sumti_composition_operator(connective)
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_primary_cmavo(
    connective: &SumtiConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        SumtiConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_primary_cmavo(connective)
        }
        SumtiConnectiveSyntax::EkConnective(connective) => connective.a.value.cmavo(),
        SumtiConnectiveSyntax::JehiConnective(connective) => connective.jehi.value.cmavo(),
        SumtiConnectiveSyntax::ExperimentalVuhuSumtiConnective(connective) => {
            connective.0.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_is_logical(connective: &SumtiConnectiveSyntax) -> bool {
    if generated_sumti_connective_question_token(connective).is_some() {
        return true;
    }
    matches!(
        generated_sumti_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
                | Cmavo::Ge
                | Cmavo::Ga
                | Cmavo::Go
                | Cmavo::Gu
                | Cmavo::Jehi
                | Cmavo::Gehi
        )
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_is_interval(connective: &SumtiConnectiveSyntax) -> bool {
    matches!(
        generated_sumti_connective_primary_cmavo(connective),
        Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)
    )
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn generated_nonlogical_sumti_composition_operator(
    connective: &SumtiConnectiveSyntax,
) -> Result<CompositionOperator, SemanticsError> {
    if matches!(
        connective,
        SumtiConnectiveSyntax::ExperimentalVuhuSumtiConnective(_)
    ) {
        return Err(undefined_semantics(&format!(
            "the experimental VUhU sumti connective `{}` outside a mekso expression",
            generated_sumti_connective_source(connective)?
        )));
    }
    match generated_sumti_connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => Ok(CompositionOperator::Joint),
        Some(Cmavo::Joi) => Ok(CompositionOperator::Mass),
        Some(Cmavo::Ce) => Ok(CompositionOperator::Set),
        Some(Cmavo::Ceho) => Ok(CompositionOperator::Sequence),
        Some(Cmavo::Fahu) => Ok(CompositionOperator::Respectively),
        Some(Cmavo::Johe) => Ok(CompositionOperator::Union),
        Some(Cmavo::Kuha) => Ok(CompositionOperator::Intersection),
        Some(Cmavo::Pihu) => Ok(CompositionOperator::CrossProduct),
        Some(Cmavo::Bihi) => Ok(CompositionOperator::UnorderedInterval),
        Some(Cmavo::Biho) => Ok(CompositionOperator::OrderedInterval),
        Some(Cmavo::Mihi) => Ok(CompositionOperator::CenteredInterval),
        _ => Err(invalid_graph(format!(
            "generated nonlogical sumti connective `{}` has no composition operator",
            generated_sumti_connective_source(connective)?
        ))),
    }
}

#[requires(true)]
#[ensures(!ret || generated_sumti_connective_has_se(connective))]
pub(super) fn generated_sumti_connective_reverses_composition_members(
    connective: &SumtiConnectiveSyntax,
) -> bool {
    generated_sumti_connective_has_se(connective)
        && matches!(
            generated_sumti_connective_primary_cmavo(connective),
            Some(Cmavo::Ceho | Cmavo::Fahu | Cmavo::Pihu | Cmavo::Biho | Cmavo::Mihi)
        )
}

#[requires(true)]
#[ensures(ret.is_none() || generated_sumti_connective_is_interval(connective))]
pub(super) fn generated_sumti_connective_endpoint_inclusion(
    connective: &SumtiConnectiveSyntax,
    reverse_members: bool,
) -> Option<IntervalEndpointInclusion> {
    let SumtiConnectiveSyntax::JoikConnective(JoikConnectiveSyntax::ClosedIntervalConnective(
        connective,
    )) = connective
    else {
        return None;
    };
    let left = endpoint_inclusion_for_generated_cmavo(connective.left_interval.cmavo()?)?;
    let right = endpoint_inclusion_for_generated_cmavo(connective.right_interval.value.cmavo()?)?;
    if reverse_members {
        Some(IntervalEndpointInclusion {
            left: right,
            right: left,
        })
    } else {
        Some(IntervalEndpointInclusion { left, right })
    }
}

#[requires(true)]
#[ensures(matches!(ret, Some(crate::model::EndpointInclusion::Inclusive)) == (cmavo == Cmavo::Gaho))]
pub(super) fn endpoint_inclusion_for_generated_cmavo(
    cmavo: Cmavo,
) -> Option<crate::model::EndpointInclusion> {
    match cmavo {
        Cmavo::Gaho => Some(crate::model::EndpointInclusion::Inclusive),
        Cmavo::Kehi => Some(crate::model::EndpointInclusion::Exclusive),
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_sumti_connective_tokens(connective: &SumtiConnectiveSyntax) -> Vec<Token> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector.tokens.into_iter().cloned().collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_head_indicator_parts(
    connective: &SumtiConnectiveSyntax,
) -> Vec<IndicatorPart> {
    generated_sumti_connective_tokens(connective)
        .into_iter()
        .filter(generated_sumti_connective_token_is_head)
        .flat_map(|token| indicator_parts_for_token(&token))
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_modifier_indicator_parts(
    connective: &SumtiConnectiveSyntax,
) -> Vec<IndicatorPart> {
    generated_sumti_connective_tokens(connective)
        .into_iter()
        .filter(|token| !generated_sumti_connective_token_is_head(token))
        .flat_map(|token| indicator_parts_for_token(&token))
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_token_is_head(token: &Token) -> bool {
    token.is_selmaho(Selmaho::A)
        || token.is_selmaho(Selmaho::Joi)
        || token.is_selmaho(Selmaho::Bihi)
        || token.is_selmaho(Selmaho::Vuhu)
        || token.is_selmaho(Selmaho::Jehi)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi))))]
pub(super) fn generated_sumti_connective_question_token(
    connective: &SumtiConnectiveSyntax,
) -> Option<Token> {
    generated_sumti_connective_tokens(connective)
        .into_iter()
        .find(|token| {
            matches!(
                token.cmavo(),
                Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi)
            )
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_formula_operator(
    connective: &SumtiConnectiveSyntax,
) -> FormulaOperator {
    if generated_sumti_connective_question_token(connective).is_some() {
        return FormulaOperator::ConnectiveQuestion;
    }
    let tokens = generated_sumti_connective_tokens(connective);
    if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::A | Cmavo::Ja | Cmavo::Ga)))
    {
        FormulaOperator::Or
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::E | Cmavo::Je | Cmavo::Ge)))
    {
        FormulaOperator::And
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::O | Cmavo::Jo | Cmavo::Go)))
    {
        FormulaOperator::Iff
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::U | Cmavo::Ju | Cmavo::Gu)))
    {
        FormulaOperator::WhetherOrNot
    } else {
        FormulaOperator::And
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_sumti_connective_source(
    connective: &SumtiConnectiveSyntax,
) -> Result<String, SemanticsError> {
    if let Some(token) = generated_sumti_connective_question_token(connective) {
        return Ok(token_text(&token));
    }
    let tokens = generated_sumti_connective_tokens(connective);
    if tokens.is_empty() {
        return Err(invalid_graph(
            "generated sumti connective has no tokens".to_owned(),
        ));
    }
    Ok(connective_source_from_tokens(tokens.iter().collect()))
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_sumti_connective_truth_table(
    connective: &SumtiConnectiveSyntax,
) -> Option<String> {
    if generated_sumti_connective_question_token(connective).is_some() {
        return None;
    }
    let tokens = generated_sumti_connective_tokens(connective);
    let base = if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::A | Cmavo::Ja | Cmavo::Ga)))
    {
        Some(Cmavo::A)
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::E | Cmavo::Je | Cmavo::Ge)))
    {
        Some(Cmavo::E)
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::O | Cmavo::Jo | Cmavo::Go)))
    {
        Some(Cmavo::O)
    } else if tokens
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::U | Cmavo::Ju | Cmavo::Gu)))
    {
        Some(Cmavo::U)
    } else {
        None
    }?;
    let left_negated = generated_sumti_connective_negates_left(connective);
    let right_negated = generated_sumti_connective_negates_right(connective);
    let se = generated_sumti_connective_has_se(connective);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    generated_connective_truth_value(base, right, left)
                } else {
                    generated_connective_truth_value(base, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_negates_left(connective: &SumtiConnectiveSyntax) -> bool {
    generated_sumti_connective_tokens(connective)
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Na))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_negates_right(connective: &SumtiConnectiveSyntax) -> bool {
    generated_sumti_connective_tokens(connective)
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Nai))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connective_has_se(connective: &SumtiConnectiveSyntax) -> bool {
    generated_sumti_connective_tokens(connective)
        .iter()
        .any(|token| token.is_selmaho(Selmaho::Se))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_is_logical(
    connective: GeneratedDirectTermConnective<'_>,
) -> bool {
    matches!(
        generated_direct_term_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
                | Cmavo::Ji
        )
    )
}

/// Classify a direct term connection among `terms` (if any) for lowering paths that cannot build
/// it — e.g. paths carrying preassigned/shared arguments (`gi'e`-shared terms). Returns the
/// graceful unsupported-construct error so such a connection never reaches simple-term assignment
/// lowering and trips a graph invariant: a nonlogical connection is unsupported everywhere, and a
/// logical connection is unsupported specifically when combined with shared arguments (threading
/// preassigned arguments through the connection is a separate feature). Returns `None` when the
/// terms contain no direct term connection.
#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connection_unsupported_error(
    terms: &[&TermSyntax],
) -> Option<SemanticsError> {
    let connection = terms.iter().copied().find_map(|term| match term {
        TermSyntax::ConnectedTerm(connection) if !connection.continuations.is_empty() => Some(term),
        TermSyntax::BoundTermConnection(_) => Some(term),
        _ => None,
    })?;
    let all_logical = match connection {
        TermSyntax::ConnectedTerm(connection) => {
            connection.continuations.iter().all(|continuation| {
                generated_direct_term_connective_is_logical(
                    GeneratedDirectTermConnective::Connected(&continuation.connective),
                )
            })
        }
        TermSyntax::BoundTermConnection(connection) => generated_direct_term_connective_is_logical(
            GeneratedDirectTermConnective::Bound(&connection.connective),
        ),
        _ => unreachable!("the direct term connection search returned another term kind"),
    };
    Some(if all_logical {
        undefined_semantics("a direct term connection that shares terms with a connected bridi")
    } else {
        undefined_semantics("an experimental nonlogical direct term connection")
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_formula_operator(
    connective: GeneratedDirectTermConnective<'_>,
) -> FormulaOperator {
    match generated_direct_term_connective_primary_cmavo(connective) {
        Some(Cmavo::Ji) => FormulaOperator::ConnectiveQuestion,
        Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_primary_cmavo(
    connective: GeneratedDirectTermConnective<'_>,
) -> Option<Cmavo> {
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => match connective {
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_primary_cmavo(connective),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JekConnective(
                connective,
            ) => connective.ja.value.cmavo(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.a.value.cmavo(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::VuhuNonlogicalConnective(
                connective,
            ) => connective.0.value.cmavo(),
        },
        GeneratedDirectTermConnective::Bound(connective) => match connective {
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_primary_cmavo(connective),
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.a.value.cmavo(),
        },
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| token.cmavo() == Some(Cmavo::Ji)))]
pub(super) fn generated_direct_term_connective_question_token(
    connective: GeneratedDirectTermConnective<'_>,
) -> Option<Token> {
    let mut collector = GeneratedSpanCollector::default();
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => {
            connective.visit_in_order(&mut collector);
        }
        GeneratedDirectTermConnective::Bound(connective) => {
            connective.visit_in_order(&mut collector);
        }
    }
    collector
        .tokens
        .into_iter()
        .find(|token| token.cmavo() == Some(Cmavo::Ji))
        .cloned()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_direct_term_connective_source(
    connective: GeneratedDirectTermConnective<'_>,
) -> Result<String, SemanticsError> {
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => match connective {
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JoikConnective(
                connective,
            ) => Ok(generated_joik_connective_source(connective)),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JekConnective(
                connective,
            ) => {
                let mut tokens = Vec::new();
                if let Some(token) = &connective.na {
                    tokens.push(token);
                }
                if let Some(token) = &connective.se {
                    tokens.push(token);
                }
                tokens.push(&connective.ja.value);
                if let Some(token) = &connective.nai {
                    tokens.push(&token.value);
                }
                Ok(connective_source_from_tokens(tokens))
            }
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::EkConnective(
                connective,
            ) => {
                let mut tokens = Vec::new();
                if let Some(token) = &connective.na {
                    tokens.push(token);
                }
                if let Some(token) = &connective.se {
                    tokens.push(token);
                }
                tokens.push(&connective.a.value);
                if let Some(token) = &connective.nai {
                    tokens.push(&token.value);
                }
                Ok(connective_source_from_tokens(tokens))
            }
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::VuhuNonlogicalConnective(
                connective,
            ) => Ok(token_text(&connective.0.value)),
        },
        GeneratedDirectTermConnective::Bound(connective) => match connective {
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::JoikConnective(
                connective,
            ) => Ok(generated_joik_connective_source(connective)),
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::EkConnective(
                connective,
            ) => {
                let mut tokens = Vec::new();
                if let Some(token) = &connective.na {
                    tokens.push(token);
                }
                if let Some(token) = &connective.se {
                    tokens.push(token);
                }
                tokens.push(&connective.a.value);
                if let Some(token) = &connective.nai {
                    tokens.push(&token.value);
                }
                Ok(connective_source_from_tokens(tokens))
            }
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_has_se(
    connective: GeneratedDirectTermConnective<'_>,
) -> bool {
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => match connective {
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_has_se(connective),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JekConnective(
                connective,
            ) => connective.se.is_some(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.se.is_some(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::VuhuNonlogicalConnective(
                _,
            ) => false,
        },
        GeneratedDirectTermConnective::Bound(connective) => match connective {
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_has_se(connective),
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.se.is_some(),
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_negates_left(
    connective: GeneratedDirectTermConnective<'_>,
) -> bool {
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => match connective {
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JekConnective(
                connective,
            ) => connective.na.is_some(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.na.is_some(),
            _ => false,
        },
        GeneratedDirectTermConnective::Bound(connective) => match connective {
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::EkConnective(connective) => {
                connective.na.is_some()
            }
            _ => false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_direct_term_connective_negates_right(
    connective: GeneratedDirectTermConnective<'_>,
) -> bool {
    match connective {
        GeneratedDirectTermConnective::Connected(connective) => match connective {
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_negates_right(connective),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::JekConnective(
                connective,
            ) => connective.nai.is_some(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.nai.is_some(),
            jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax::VuhuNonlogicalConnective(
                _,
            ) => false,
        },
        GeneratedDirectTermConnective::Bound(connective) => match connective {
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::JoikConnective(
                connective,
            ) => generated_joik_connective_negates_right(connective),
            jbotci_syntax::generated_model::BoundTermConnectiveSyntax::EkConnective(
                connective,
            ) => connective.nai.is_some(),
        },
    }
}

#[requires(generated_direct_term_connective_is_logical(connective))]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_direct_term_connective_truth_table(
    connective: GeneratedDirectTermConnective<'_>,
) -> Option<String> {
    let operator = generated_direct_term_connective_formula_operator(connective);
    if operator == FormulaOperator::ConnectiveQuestion {
        return None;
    }
    let left_negated = generated_direct_term_connective_negates_left(connective);
    let right_negated = generated_direct_term_connective_negates_right(connective);
    let se = generated_direct_term_connective_has_se(connective);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_connective_truth_value(kind: Cmavo, left: bool, right: bool) -> bool {
    match kind {
        Cmavo::A => left || right,
        Cmavo::E => left && right,
        Cmavo::O => left == right,
        Cmavo::U => left,
        _ => left && right,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_relation_afterthought_connective_source(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.a.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.ja.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            Ok(generated_joik_connective_source(connective))
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            Ok(token_text(&connective.0.value))
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_joik_connective_source(connective: &JoikConnectiveSyntax) -> String {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.joi.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.bihi.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => {
            let mut tokens = Vec::new();
            tokens.push(&connective.left_interval);
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.bihi);
            if let Some(token) = &connective.nai {
                tokens.push(token);
            }
            tokens.push(&connective.right_interval.value);
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::ZantufaGahoJoikConnective(connective) => {
            let mut tokens = vec![&connective.left_gaho.value];
            if let Some(token) = &connective.na {
                tokens.push(&token.value);
            }
            if let Some(token) = &connective.se {
                tokens.push(&token.value);
            }
            tokens.push(&connective.joiz.value);
            if let Some(token) = &connective.right_gaho {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::ZantufaNaJoikConnective(connective) => {
            let mut tokens = vec![&connective.na.value];
            if let Some(token) = &connective.se {
                tokens.push(&token.value);
            }
            tokens.push(&connective.joiz.value);
            if let Some(token) = &connective.right_gaho {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::ZantufaRightGahoJoikConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.se {
                tokens.push(&token.value);
            }
            tokens.push(&connective.joiz.value);
            tokens.push(&connective.right_gaho.value);
            connective_source_from_tokens(tokens)
        }
    }
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn connective_source_from_tokens(tokens: Vec<&Token>) -> String {
    tokens
        .into_iter()
        .map(token_text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_parts_for_leading_indicator(
    indicator: &LeadingIndicatorSyntax,
) -> Vec<IndicatorPart> {
    let mut parts = if let Some(cmavo) = indicator.indicator.core_word().cmavo() {
        vec![IndicatorPart {
            cmavo,
            nai: false,
            tokens: vec![Token::bare(indicator.indicator.core_word().clone())],
        }]
    } else {
        Vec::new()
    };
    parts.extend(indicator_parts_for_token(&indicator.indicator));
    if let Some(nai) = &indicator.nai
        && let Some(last) = parts.last_mut()
    {
        last.nai = true;
        last.tokens.push(nai.clone());
    }
    parts
}

#[requires(true)]
#[ensures(!truth_question_consumed || ret.iter().all(|part| part.cmavo != Cmavo::Xu))]
pub(super) fn leading_indicator_parts(
    indicators: &[LeadingIndicatorSyntax],
    truth_question_consumed: bool,
) -> Vec<IndicatorPart> {
    indicators
        .iter()
        .flat_map(indicator_parts_for_leading_indicator)
        .filter(|part| !truth_question_consumed || part.cmavo != Cmavo::Xu)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_parts_for_token(token: &Token) -> Vec<IndicatorPart> {
    let mut parts = Vec::new();
    indicator_parts_for_with_indicators(token.as_indicators(), &mut parts);
    parts
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_parts_for_generated_node<N: TreeNode>(node: &N) -> Vec<IndicatorPart> {
    let mut collector = GeneratedSpanCollector::default();
    node.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .copied()
        .flat_map(indicator_parts_for_token)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_connection_has_branch_indicator_attachment(
    sumti: &SumtiSyntax,
) -> bool {
    let grouped = &sumti.base_sumti;
    grouped.grouped_tail.is_some()
        || !grouped.leading_sumti.continuations.is_empty()
        || grouped.leading_sumti.leading_sumti.bound_tail.is_some()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_subbridi_is_connected_bridi_tail(subbridi: &SubbridiSyntax) -> bool {
    match subbridi {
        SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
            generated_bridi_is_connected_bridi_tail(bridi)
        }
        SubbridiSyntax::PrenexSubbridi(prenex) => {
            generated_subbridi_is_connected_bridi_tail(&prenex.inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_is_connected_bridi_tail(bridi: &BridiSyntax) -> bool {
    match bridi {
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(tail)) => {
            generated_bridi_tail_is_connected(tail)
        }
        BridiSyntax::BridiWithLeadingTerms(bridi) => {
            generated_bridi_tail_is_connected(&bridi.bridi_tail)
        }
        BridiSyntax::BareCuBridi(bridi) => generated_bridi_tail_is_connected(&bridi.bridi_tail),
        BridiSyntax::BridiWithPostCuTerms(_) | BridiSyntax::BareCuTermsBridi(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_tail_is_connected(tail: &BridiTailSyntax) -> bool {
    match tail {
        BridiTailSyntax::ZantufaGroupedBridiTail(tail) => {
            generated_bridi_tail_is_connected(&tail.bridi_tail)
        }
        BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
            !tail.first.0.links.is_empty()
                || tail.first.0.first.bo_continuation.is_some()
                || tail.ke_continuation.is_some()
        }
        BridiTailSyntax::BridiTailWithoutTailTerms(tail) => {
            !tail.first.0.links.is_empty()
                || tail.first.0.first.bo_continuation.is_some()
                || tail.ke_continuation.is_some()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_parts_for_with_indicators(
    indicators: &WithIndicators<WordLike>,
    out: &mut Vec<IndicatorPart>,
) {
    match indicators.as_data() {
        data!(WithIndicators::Plain(_)) | data!(WithIndicators::Emphasized { .. }) => {}
        data!(WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }) => {
            indicator_parts_for_with_indicators(base, out);
            let Some(cmavo) = indicator.cmavo() else {
                return;
            };
            let mut tokens = vec![token_with_bahe_prefix(indicator_bahe, indicator)];
            if let Some(nai) = nai {
                tokens.push(token_with_bahe_prefix(nai_bahe, nai));
            }
            out.push(IndicatorPart {
                cmavo,
                nai: nai.is_some(),
                tokens,
            });
        }
    }
}

#[requires(bahe.iter().all(|word| word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
#[ensures(true)]
pub(super) fn token_with_bahe_prefix(bahe: &[Word], word: &Word) -> Token {
    if let Some((first_bahe, extra_bahe)) = bahe.split_first() {
        Token::from_indicators(WithIndicators::emphasized_with_extra_bahe(
            first_bahe.clone(),
            extra_bahe.to_vec(),
            WordLike::bare(word.clone()),
        ))
    } else {
        Token::bare(WordLike::bare(word.clone()))
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_display_drafts(parts: Vec<IndicatorPart>) -> Vec<IndicatorDisplayDraft> {
    let mut drafts = Vec::new();
    let mut current: Option<IndicatorDisplayDraft> = None;
    let mut pending_question_tokens = Vec::new();
    for part in parts {
        if part.cmavo == Cmavo::Kau {
            continue;
        }
        if part.cmavo == Cmavo::Pei {
            if let Some(draft) = &mut current {
                draft.question = true;
                draft.source_tokens.extend(part.tokens);
            } else {
                pending_question_tokens.extend(part.tokens);
            }
            continue;
        }
        if let Some(draft) = current.as_mut()
            && apply_indicator_modifier_to_draft(draft, &part)
        {
            continue;
        }
        if current.is_none()
            && let Some(relation) = indicator_modifier_relation(part.cmavo)
        {
            current = Some(IndicatorDisplayDraft {
                family: DisplayedContentFamily::AttitudeModifier,
                relation: relation.to_owned(),
                polarity: if part.nai {
                    DisplayedContentPolarity::Negative
                } else {
                    DisplayedContentPolarity::Positive
                },
                assertion_effect: DisplayedContentAssertionEffect::None,
                intensity: None,
                phase: None,
                modifiers: Vec::new(),
                question: false,
                empathy: false,
                source_tokens: part.tokens,
            });
            continue;
        }
        if let Some(spec) = indicator_base_spec(part.cmavo) {
            if let Some(draft) = current.take() {
                drafts.push(draft);
            }
            let mut source_tokens = std::mem::take(&mut pending_question_tokens);
            source_tokens.extend(part.tokens);
            let (relation, polarity) =
                indicator_base_relation_and_polarity(part.cmavo, spec.relation, part.nai);
            current = Some(IndicatorDisplayDraft {
                family: spec.family,
                relation,
                polarity,
                assertion_effect: spec.assertion_effect,
                intensity: None,
                phase: None,
                modifiers: Vec::new(),
                question: !source_tokens.is_empty() && source_tokens[0].cmavo() == Some(Cmavo::Pei),
                empathy: false,
                source_tokens,
            });
            continue;
        }
        let Some(draft) = current.as_mut() else {
            continue;
        };
        draft.source_tokens.extend(part.tokens.clone());
    }
    if let Some(draft) = current {
        drafts.push(draft);
    } else if !pending_question_tokens.is_empty() {
        drafts.push(IndicatorDisplayDraft {
            family: DisplayedContentFamily::QuestionPrompt,
            relation: "attitudeQuestion".to_owned(),
            polarity: DisplayedContentPolarity::Neutral,
            assertion_effect: DisplayedContentAssertionEffect::None,
            intensity: None,
            phase: None,
            modifiers: Vec::new(),
            question: false,
            empathy: false,
            source_tokens: pending_question_tokens,
        });
    }
    drafts
}

#[requires(true)]
#[ensures(true)]
pub(super) fn apply_indicator_modifier_to_draft(
    draft: &mut IndicatorDisplayDraft,
    part: &IndicatorPart,
) -> bool {
    if draft.family == DisplayedContentFamily::Evidential
        && draft.relation == "expectation"
        && part.cmavo == Cmavo::Cuhi
    {
        draft.source_tokens.extend(part.tokens.clone());
        draft.relation = if part.nai { "memory" } else { "experience" }.to_owned();
        draft.polarity = DisplayedContentPolarity::Positive;
        return true;
    }
    if let Some(intensity) = indicator_intensity(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.intensity = Some(intensity.to_owned());
        return true;
    }
    if let Some(polarity) = indicator_polarity_modifier(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.polarity = polarity;
        return true;
    }
    if let Some(phase) = indicator_phase(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.phase = Some(phase.to_owned());
        return true;
    }
    if part.cmavo == Cmavo::Dai {
        draft.source_tokens.extend(part.tokens.clone());
        draft.empathy = true;
        return true;
    }
    if let Some(relation) = indicator_modifier_relation(part.cmavo) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.modifiers.push(new!(DisplayedContentModifier {
            relation: relation.to_owned(),
            family: None,
            polarity: Some(if part.nai {
                DisplayedContentPolarity::Negative
            } else {
                DisplayedContentPolarity::Positive
            }),
            intensity: None,
            assertion_effect: None,
            source: None,
        }));
        return true;
    }
    false
}

#[requires(!relation.is_empty())]
#[ensures(!ret.0.is_empty())]
pub(super) fn indicator_base_relation_and_polarity(
    cmavo: Cmavo,
    relation: &'static str,
    nai: bool,
) -> (String, DisplayedContentPolarity) {
    match (cmavo, nai) {
        (Cmavo::Baha, true) => ("memory".to_owned(), DisplayedContentPolarity::Positive),
        _ => (
            relation.to_owned(),
            if nai {
                DisplayedContentPolarity::Negative
            } else {
                DisplayedContentPolarity::Positive
            },
        ),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn displayed_assertion_effect_for_target(
    effect: DisplayedContentAssertionEffect,
    target_kind: crate::model::SemanticObjectKind,
    force_none: bool,
) -> DisplayedContentAssertionEffect {
    if force_none {
        return DisplayedContentAssertionEffect::None;
    }
    match effect {
        DisplayedContentAssertionEffect::None => DisplayedContentAssertionEffect::None,
        DisplayedContentAssertionEffect::HostAsserted
        | DisplayedContentAssertionEffect::HostSubordinated
        | DisplayedContentAssertionEffect::MetalinguisticallyVoided
        | DisplayedContentAssertionEffect::Performative
            if target_kind == crate::model::SemanticObjectKind::Formula =>
        {
            effect
        }
        DisplayedContentAssertionEffect::HostAsserted
        | DisplayedContentAssertionEffect::HostSubordinated
        | DisplayedContentAssertionEffect::MetalinguisticallyVoided
        | DisplayedContentAssertionEffect::Performative => DisplayedContentAssertionEffect::None,
    }
}

#[requires(!relation.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn attitude_question_relation(relation: &str) -> String {
    if relation.ends_with("Question") {
        relation.to_owned()
    } else {
        format!("{relation}Question")
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|spec| !spec.relation.is_empty()))]
pub(super) fn indicator_base_spec(cmavo: Cmavo) -> Option<IndicatorBaseSpec> {
    let attitude = DisplayedContentAssertionEffect::HostSubordinated;
    let none = DisplayedContentAssertionEffect::None;
    let host = DisplayedContentAssertionEffect::HostAsserted;
    let performative = DisplayedContentAssertionEffect::Performative;
    let metalinguistically_voided = DisplayedContentAssertionEffect::MetalinguisticallyVoided;
    let spec = match cmavo {
        Cmavo::Ua => (DisplayedContentFamily::Emotion, "discovery", none),
        Cmavo::Uha => (DisplayedContentFamily::Emotion, "gain", none),
        Cmavo::Ue => (DisplayedContentFamily::Emotion, "surprise", none),
        Cmavo::Ui => (DisplayedContentFamily::Emotion, "happiness", none),
        Cmavo::Uo => (DisplayedContentFamily::Emotion, "completion", none),
        Cmavo::Uu => (DisplayedContentFamily::Emotion, "pity", none),
        Cmavo::Uhu => (DisplayedContentFamily::Emotion, "repentance", none),
        Cmavo::Ii => (DisplayedContentFamily::Emotion, "fear", none),
        Cmavo::Iu => (DisplayedContentFamily::Emotion, "love", none),
        Cmavo::Io => (DisplayedContentFamily::Emotion, "respect", none),
        Cmavo::Oi => (DisplayedContentFamily::Emotion, "complaint", none),
        Cmavo::Ohi => (DisplayedContentFamily::Emotion, "caution", none),
        Cmavo::Ohe => (DisplayedContentFamily::Emotion, "detachment", none),
        Cmavo::Oho => (DisplayedContentFamily::Emotion, "patience", none),
        Cmavo::Ohu => (DisplayedContentFamily::Emotion, "relaxation", none),
        Cmavo::Aha => (
            DisplayedContentFamily::PropositionalAttitude,
            "attention",
            attitude,
        ),
        Cmavo::Ahe => (
            DisplayedContentFamily::PropositionalAttitude,
            "alertness",
            attitude,
        ),
        Cmavo::Ai => (
            DisplayedContentFamily::PropositionalAttitude,
            "intent",
            attitude,
        ),
        Cmavo::Ahi => (
            DisplayedContentFamily::PropositionalAttitude,
            "effort",
            attitude,
        ),
        Cmavo::Aho => (
            DisplayedContentFamily::PropositionalAttitude,
            "hope",
            attitude,
        ),
        Cmavo::Au => (
            DisplayedContentFamily::PropositionalAttitude,
            "desire",
            attitude,
        ),
        Cmavo::Ahu => (
            DisplayedContentFamily::PropositionalAttitude,
            "interest",
            attitude,
        ),
        Cmavo::Eha => (
            DisplayedContentFamily::PropositionalAttitude,
            "permission",
            attitude,
        ),
        Cmavo::Ehe => (
            DisplayedContentFamily::PropositionalAttitude,
            "competence",
            attitude,
        ),
        Cmavo::Ei => (
            DisplayedContentFamily::PropositionalAttitude,
            "obligation",
            attitude,
        ),
        Cmavo::Eho => (
            DisplayedContentFamily::PropositionalAttitude,
            "request",
            attitude,
        ),
        Cmavo::Ehu => (
            DisplayedContentFamily::PropositionalAttitude,
            "suggestion",
            attitude,
        ),
        Cmavo::Ia => (
            DisplayedContentFamily::PropositionalAttitude,
            "belief",
            attitude,
        ),
        Cmavo::Iha => (
            DisplayedContentFamily::PropositionalAttitude,
            "acceptance",
            attitude,
        ),
        Cmavo::Ie => (
            DisplayedContentFamily::PropositionalAttitude,
            "agreement",
            attitude,
        ),
        Cmavo::Ihe => (
            DisplayedContentFamily::PropositionalAttitude,
            "approval",
            attitude,
        ),
        Cmavo::Cahe => (
            DisplayedContentFamily::Evidential,
            "definition",
            performative,
        ),
        Cmavo::Baha => (DisplayedContentFamily::Evidential, "expectation", host),
        Cmavo::Tihe => (DisplayedContentFamily::Evidential, "hearsay", host),
        Cmavo::Zaha => (DisplayedContentFamily::Evidential, "observation", host),
        Cmavo::Pehi => (DisplayedContentFamily::Evidential, "opinion", host),
        Cmavo::Ruha => (DisplayedContentFamily::Evidential, "presumption", host),
        Cmavo::Juho => (DisplayedContentFamily::Discursive, "certainty", attitude),
        Cmavo::Dahi => (DisplayedContentFamily::Discursive, "hypothetical", attitude),
        Cmavo::Poho => (DisplayedContentFamily::Discursive, "onlyRelevantCase", none),
        Cmavo::Kiha => (DisplayedContentFamily::Metalinguistic, "confusion", none),
        Cmavo::Peha => (DisplayedContentFamily::Metalinguistic, "figurative", none),
        Cmavo::Nahi => (
            DisplayedContentFamily::Metalinguistic,
            "metalinguisticNegation",
            metalinguistically_voided,
        ),
        Cmavo::Pau => (
            DisplayedContentFamily::QuestionPrompt,
            "questionPrompt",
            none,
        ),
        Cmavo::Xu => (
            DisplayedContentFamily::QuestionPrompt,
            "truthQuestionPrompt",
            none,
        ),
        Cmavo::Gehe => (
            DisplayedContentFamily::Metalinguistic,
            "unspecifiedAttitude",
            none,
        ),
        _ if cmavo.is_selmaho(jbotci_morphology::Selmaho::Ui) => (
            DisplayedContentFamily::Metalinguistic,
            cmavo.canonical_text(),
            none,
        ),
        _ => return None,
    };
    Some(IndicatorBaseSpec {
        family: spec.0,
        relation: spec.1,
        assertion_effect: spec.2,
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|intensity| !intensity.is_empty()))]
pub(super) fn indicator_intensity(cmavo: Cmavo, nai: bool) -> Option<&'static str> {
    match (cmavo, nai) {
        (Cmavo::Cai, false) => Some("maximal"),
        (Cmavo::Sai, false) => Some("strong"),
        (Cmavo::Ruhe, false) => Some("weak"),
        (Cmavo::Cai, true) => Some("negativeMaximal"),
        (Cmavo::Sai, true) => Some("negativeStrong"),
        (Cmavo::Ruhe, true) => Some("negativeWeak"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicator_polarity_modifier(
    cmavo: Cmavo,
    nai: bool,
) -> Option<DisplayedContentPolarity> {
    match (cmavo, nai) {
        (Cmavo::Cuhi, false) => Some(DisplayedContentPolarity::Neutral),
        (Cmavo::Cuhi, true) => Some(DisplayedContentPolarity::Negative),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|phase| !phase.is_empty()))]
pub(super) fn indicator_phase(cmavo: Cmavo, nai: bool) -> Option<&'static str> {
    match (cmavo, nai) {
        (Cmavo::Buho, false) => Some("starting"),
        (Cmavo::Buho, true) => Some("ending"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|relation| !relation.is_empty()))]
pub(super) fn indicator_modifier_relation(cmavo: Cmavo) -> Option<&'static str> {
    match cmavo {
        Cmavo::Gahi => Some("rank"),
        Cmavo::Sehi => Some("selfOrientation"),
        Cmavo::Rihe => Some("emotionalRelease"),
        Cmavo::Behu => Some("need"),
        Cmavo::Seha => Some("selfSufficiency"),
        Cmavo::Roho => Some("physical"),
        Cmavo::Rehe => Some("spiritual"),
        _ => None,
    }
}

#[invariant(true)]
pub(super) struct GeneratedIndicatorCmavoVisitor {
    cmavo: Cmavo,
    found: bool,
}

impl<'tree> TreeVisitor<'tree> for GeneratedIndicatorCmavoVisitor {
    type Node = GeneratedNodeRef<'tree>;
    type Atom = GeneratedAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let GeneratedAtomRef::Token(token) = atom;
        self.found |=
            token.cmavo() == Some(self.cmavo) || token_has_indicator_cmavo(token, self.cmavo);
    }
}

#[requires(true)]
#[ensures(ret == indicators_have_indicator_cmavo(token.as_indicators(), cmavo))]
pub(super) fn token_has_indicator_cmavo(token: &Token, cmavo: Cmavo) -> bool {
    indicators_have_indicator_cmavo(token.as_indicators(), cmavo)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn indicators_have_indicator_cmavo(
    indicators: &WithIndicators<WordLike>,
    cmavo: Cmavo,
) -> bool {
    match indicators.as_data() {
        data!(WithIndicators::Plain(_)) | data!(WithIndicators::Emphasized { .. }) => false,
        data!(WithIndicators::WithIndicator {
            base,
            indicator,
            ..
        }) => indicator.cmavo() == Some(cmavo) || indicators_have_indicator_cmavo(base, cmavo),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn with_free_modifiers_has_indicator_cmavo<F>(
    token: &WithFreeModifiers<Token, F>,
    cmavo: Cmavo,
) -> bool
where
    F: TreeNode,
{
    token_has_indicator_cmavo(&token.value, cmavo)
        || generated_free_modifiers_have_indicator_cmavo(&token.free_modifiers, cmavo)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_word_run_has_indicator_cmavo<T, F>(
    words: &WithFreeModifiers<T, F>,
    cmavo: Cmavo,
) -> bool
where
    T: AsRef<[Token]>,
    F: TreeNode,
{
    words
        .value
        .as_ref()
        .iter()
        .any(|token| token_has_indicator_cmavo(token, cmavo))
        || generated_free_modifiers_have_indicator_cmavo(&words.free_modifiers, cmavo)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_free_modifiers_have_indicator_cmavo<F>(
    free_modifiers: &[F],
    cmavo: Cmavo,
) -> bool
where
    F: TreeNode,
{
    free_modifiers
        .iter()
        .any(|free_modifier| generated_node_has_indicator_cmavo(free_modifier, cmavo))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_node_has_indicator_cmavo<N>(node: &N, cmavo: Cmavo) -> bool
where
    N: TreeNode,
{
    let mut visitor = GeneratedIndicatorCmavoVisitor {
        cmavo,
        found: false,
    };
    node.visit_in_order(&mut visitor);
    visitor.found
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_has_current_kau_focus(sumti: &SumtiSyntax) -> bool {
    if sumti.vuho_attachment.is_some() {
        return false;
    }
    generated_sumti_grouped_has_current_kau_focus(&sumti.base_sumti)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_grouped_has_current_kau_focus(sumti: &SumtiGroupedSyntax) -> bool {
    if sumti.grouped_tail.is_some() {
        return false;
    }
    generated_sumti_afterthought_has_current_kau_focus(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_afterthought_has_current_kau_focus(
    sumti: &SumtiAfterthoughtSyntax,
) -> bool {
    if !sumti.continuations.is_empty() {
        return false;
    }
    generated_sumti_bound_has_current_kau_focus(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_bound_has_current_kau_focus(sumti: &SumtiBoundSyntax) -> bool {
    if sumti.bound_tail.is_some() {
        return false;
    }
    generated_sumti_forethought_has_current_kau_focus(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_forethought_has_current_kau_focus(
    sumti: &SumtiForethoughtSyntax,
) -> bool {
    match sumti {
        SumtiForethoughtSyntax::SimpleSumti(sumti) => {
            generated_simple_sumti_has_current_kau_focus(sumti)
        }
        SumtiForethoughtSyntax::ForethoughtSumti(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_sumti_has_current_kau_focus(sumti: &SimpleSumtiSyntax) -> bool {
    match sumti.base_sumti.as_ref() {
        SumtiAtomSyntax::SumtiBase(sumti) => generated_sumti_base_has_current_kau_focus(sumti),
        SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_quantified_sumti_has_current_kau_focus(sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_quantified_sumti_has_current_kau_focus(
    sumti: &QuantifiedSumtiSyntax,
) -> bool {
    generated_sumti_base_has_current_kau_focus(&sumti.inner_sumti)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_sumti_base_has_current_kau_focus(sumti: &SumtiBaseSyntax) -> bool {
    match sumti {
        SumtiBaseSyntax::ProSumti(pro_sumti) => {
            with_free_modifiers_has_indicator_cmavo(&pro_sumti.0, Cmavo::Kau)
        }
        SumtiBaseSyntax::NameSumti(name) => {
            generated_word_run_has_indicator_cmavo(&name.names, Cmavo::Kau)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|operator| matches!(operator, FormulaOperator::And | FormulaOperator::Or | FormulaOperator::Iff | FormulaOperator::WhetherOrNot | FormulaOperator::ConnectiveQuestion)) || ret.is_err())]
pub(super) fn generated_statement_connective_formula_operator(
    connective: &IStatementConnectiveSyntax,
) -> Result<FormulaOperator, SemanticsError> {
    let Some(connective) = generated_i_statement_connective_core(connective)? else {
        return Ok(FormulaOperator::And);
    };
    Ok(
        match generated_statement_connective_primary_cmavo(connective) {
            Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
            Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
            Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
            Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
            Some(Cmavo::Ji | Cmavo::Jehi) => FormulaOperator::ConnectiveQuestion,
            _ => FormulaOperator::And,
        },
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_statement_connective_source(
    connective: &IStatementConnectiveSyntax,
) -> Result<String, SemanticsError> {
    Ok(generated_i_statement_connective_token_source(connective))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|table| table.is_none() || table.as_ref().is_some_and(|table| table.len() == 4)) || ret.is_err())]
pub(super) fn generated_statement_connective_truth_table(
    connective: &IStatementConnectiveSyntax,
) -> Result<Option<String>, SemanticsError> {
    Ok(
        if let Some(connective) = generated_i_statement_connective_core(connective)? {
            generated_statement_connective_core_truth_table(connective)
        } else {
            Some(generated_truth_table_for_formula_operator(
                FormulaOperator::And,
            ))
        },
    )
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_statement_connective_core_truth_table(
    connective: &StatementConnectiveSyntax,
) -> Option<String> {
    if !generated_statement_connective_is_logical(connective) {
        return None;
    }
    let operator = generated_statement_connective_formula_operator_for_core(connective);
    if operator == FormulaOperator::ConnectiveQuestion {
        return None;
    }
    let left_negated = generated_statement_connective_negates_left(connective);
    let right_negated = generated_statement_connective_negates_right(connective);
    let se = generated_statement_connective_has_se(connective);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|_| true) || ret.is_err())]
pub(super) fn generated_i_statement_connective_core(
    connective: &IStatementConnectiveSyntax,
) -> Result<Option<&StatementConnectiveSyntax>, SemanticsError> {
    match connective {
        IStatementConnectiveSyntax::IStandardStatementConnective(connective) => {
            Ok(Some(&connective.connective))
        }
        IStatementConnectiveSyntax::ITagBoStatementConnective(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_i_statement_connective_has_bo(
    connective: &IStatementConnectiveSyntax,
) -> bool {
    match connective {
        IStatementConnectiveSyntax::IStandardStatementConnective(connective) => {
            connective.tag_bo.is_some()
        }
        IStatementConnectiveSyntax::ITagBoStatementConnective(_) => true,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
pub(super) fn generated_modal_statement_connection_spec(
    connective: &IStatementConnectiveSyntax,
) -> Option<GeneratedModalStatementConnectionSpec> {
    match connective {
        IStatementConnectiveSyntax::IStandardStatementConnective(connective) => connective
            .tag_bo
            .as_ref()
            .and_then(|(tense_modal, _bo)| tense_modal.as_deref())
            .and_then(generated_modal_statement_connection_spec_for_tense_modal),
        IStatementConnectiveSyntax::ITagBoStatementConnective(connective) => connective
            .tense_modal
            .as_deref()
            .and_then(generated_modal_statement_connection_spec_for_tense_modal),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
pub(super) fn generated_paragraph_modal_statement_connection_spec(
    connective: &IParagraphStatementConnectiveSyntax,
) -> Option<GeneratedModalStatementConnectionSpec> {
    match connective {
        IParagraphStatementConnectiveSyntax::IStandardParagraphStatementConnective(connective) => {
            connective
                .tag_bo
                .as_ref()
                .and_then(|(tense_modal, _bo)| tense_modal.as_deref())
                .and_then(generated_modal_statement_connection_spec_for_tense_modal)
        }
        IParagraphStatementConnectiveSyntax::ITagBoParagraphStatementConnective(connective) => {
            connective
                .tense_modal
                .as_deref()
                .and_then(generated_modal_statement_connection_spec_for_tense_modal)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
pub(super) fn generated_modal_statement_connection_spec_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<GeneratedModalStatementConnectionSpec> {
    let (introduced_by, relation, visible_place) =
        generated_modal_relation_spec_for_tense_modal(tense_modal)?;
    Some(new!(GeneratedModalStatementConnectionSpec {
        argument_kind: generated_modal_connection_argument_kind_for_tense_modal(tense_modal),
        introduced_by,
        relation,
        visible_place,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
pub(super) fn generated_modal_statement_connection_spec_for_optional_tense_modal(
    tense_modal: Option<&TenseModalSyntax>,
) -> Option<GeneratedModalStatementConnectionSpec> {
    tense_modal.and_then(generated_modal_statement_connection_spec_for_tense_modal)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_modal_connection_argument_kind_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> GeneratedModalConnectionArgumentKind {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .find(|token| token.is_selmaho(Selmaho::Bai))
        .map(|token| generated_modal_connection_argument_kind_for_marker(&token_text(token)))
        .unwrap_or(GeneratedModalConnectionArgumentKind::Eventuality)
}

#[requires(!marker.is_empty())]
#[ensures(true)]
pub(super) fn generated_modal_connection_argument_kind_for_marker(
    marker: &str,
) -> GeneratedModalConnectionArgumentKind {
    match marker {
        "ni'i" => GeneratedModalConnectionArgumentKind::Formula,
        _ => GeneratedModalConnectionArgumentKind::Eventuality,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_i_statement_connective_has_logical_component(
    connective: &IStatementConnectiveSyntax,
) -> bool {
    match connective {
        IStatementConnectiveSyntax::IStandardStatementConnective(connective) => {
            generated_statement_connective_core_has_logical_component(&connective.connective)
        }
        IStatementConnectiveSyntax::ITagBoStatementConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_core_has_logical_component(
    connective: &StatementConnectiveSyntax,
) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector.tokens.iter().any(|token| {
        matches!(
            token.cmavo(),
            Some(
                Cmavo::A
                    | Cmavo::E
                    | Cmavo::O
                    | Cmavo::U
                    | Cmavo::Ga
                    | Cmavo::Ge
                    | Cmavo::Go
                    | Cmavo::Gu
                    | Cmavo::Giha
                    | Cmavo::Gihe
                    | Cmavo::Giho
                    | Cmavo::Gihu
                    | Cmavo::Ja
                    | Cmavo::Je
                    | Cmavo::Jo
                    | Cmavo::Ju
                    | Cmavo::Ji
                    | Cmavo::Jehi
            )
        )
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|connection| !connection.operator.is_empty() && connection.connector.source.as_surface_word().is_some()) || ret.is_err())]
pub(super) fn generated_i_statement_nonlogical_connection(
    connective: &IStatementConnectiveSyntax,
) -> Result<NonlogicalConnection, SemanticsError> {
    let source = generated_i_statement_connective_token_source(connective);
    let operator = match generated_i_statement_connective_core(connective)? {
        Some(core) => generated_nonlogical_statement_composition_operator(core)?
            .label()
            .to_owned(),
        None => format!("nonlogical:{source}"),
    };
    let truth_table = generated_statement_connective_truth_table(connective)?;
    Ok(NonlogicalConnection::new(
        operator,
        new!(Connector {
            source: ConnectorSource::surface_word(source),
            locus: ConnectorLocus::Statement,
            truth_table,
            parameter: None,
        }),
    ))
}

#[requires(!generated_statement_connective_is_logical(connective))]
#[ensures(ret.as_ref().is_ok_and(|connection| !connection.operator.is_empty() && connection.connector.source.as_surface_word().is_some()) || ret.is_err())]
pub(super) fn generated_statement_core_nonlogical_connection(
    connective: &StatementConnectiveSyntax,
) -> Result<NonlogicalConnection, SemanticsError> {
    let source = generated_statement_connective_core_source(connective)?;
    let operator = generated_nonlogical_statement_composition_operator(connective)?
        .label()
        .to_owned();
    Ok(NonlogicalConnection::new(
        operator,
        new!(Connector {
            source: ConnectorSource::surface_word(source),
            locus: ConnectorLocus::Statement,
            truth_table: None,
            parameter: None,
        }),
    ))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_i_statement_connective_token_source(
    connective: &IStatementConnectiveSyntax,
) -> String {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    token_list_text(collector.tokens.iter().copied())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_formula_operator_for_core(
    connective: &StatementConnectiveSyntax,
) -> FormulaOperator {
    match generated_statement_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
        Some(Cmavo::Ji | Cmavo::Jehi) => FormulaOperator::ConnectiveQuestion,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_primary_cmavo(
    connective: &StatementConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.a.value.cmavo(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.ja.value.cmavo(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_primary_cmavo(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            connective.0.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_statement_connective_core_source(
    connective: &StatementConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.a.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        StatementConnectiveSyntax::JekConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.ja.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        StatementConnectiveSyntax::JoikConnective(connective) => {
            Ok(generated_joik_connective_source(connective))
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            Ok(token_text(&connective.0.value))
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_has_se(
    connective: &StatementConnectiveSyntax,
) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.se.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.se.is_some(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_has_se(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_negates_left(
    connective: &StatementConnectiveSyntax,
) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.na.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.na.is_some(),
        StatementConnectiveSyntax::JoikConnective(_) => false,
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_negates_right(
    connective: &StatementConnectiveSyntax,
) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.nai.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.nai.is_some(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_negates_right(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_connective_is_logical(
    connective: &StatementConnectiveSyntax,
) -> bool {
    matches!(
        generated_statement_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
                | Cmavo::Ji
                | Cmavo::Jehi
        )
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
pub(super) fn build_generated_connective_question_parameter_for_statement_connective(
    builder: &mut GeneratedGraphBuilder<'_, '_, '_>,
    connective: &StatementConnectiveSyntax,
) -> Result<Option<SemanticObjectId>, SemanticsError> {
    let Some(token) = generated_statement_connective_question_token(connective) else {
        return Ok(None);
    };
    builder.build_generated_connective_question_parameter_for_token(&token)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Jehi))))]
pub(super) fn generated_statement_connective_question_token(
    connective: &StatementConnectiveSyntax,
) -> Option<Token> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .into_iter()
        .find(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Jehi)))
        .cloned()
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn generated_nonlogical_statement_composition_operator(
    connective: &StatementConnectiveSyntax,
) -> Result<CompositionOperator, SemanticsError> {
    if matches!(
        connective,
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_)
    ) {
        return Err(undefined_semantics(&format!(
            "the experimental VUhU statement connective `{}` outside a mekso expression",
            generated_statement_connective_core_source(connective)?
        )));
    }
    match generated_statement_connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => Ok(CompositionOperator::Joint),
        Some(Cmavo::Joi) => Ok(CompositionOperator::Mass),
        Some(Cmavo::Ce) => Ok(CompositionOperator::Set),
        Some(Cmavo::Ceho) => Ok(CompositionOperator::Sequence),
        Some(Cmavo::Fahu) => Ok(CompositionOperator::Respectively),
        Some(Cmavo::Johe) => Ok(CompositionOperator::Union),
        Some(Cmavo::Kuha) => Ok(CompositionOperator::Intersection),
        Some(Cmavo::Pihu) => Ok(CompositionOperator::CrossProduct),
        Some(Cmavo::Bihi) => Ok(CompositionOperator::UnorderedInterval),
        Some(Cmavo::Biho) => Ok(CompositionOperator::OrderedInterval),
        Some(Cmavo::Mihi) => Ok(CompositionOperator::CenteredInterval),
        _ => Err(invalid_graph(format!(
            "generated nonlogical statement connective `{}` has no composition operator",
            generated_statement_connective_core_source(connective)?
        ))),
    }
}

#[requires(true)]
#[ensures(!ret || generated_statement_connective_has_se(connective))]
pub(super) fn generated_statement_connective_reverses_composition_members(
    connective: &StatementConnectiveSyntax,
) -> bool {
    generated_statement_connective_has_se(connective)
        && matches!(
            generated_statement_connective_primary_cmavo(connective),
            Some(Cmavo::Ceho | Cmavo::Fahu | Cmavo::Pihu | Cmavo::Biho | Cmavo::Mihi)
        )
}

#[requires(true)]
#[ensures(ret.is_none() || matches!(generated_statement_connective_primary_cmavo(connective), Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)))]
pub(super) fn generated_statement_connective_endpoint_inclusion(
    connective: &StatementConnectiveSyntax,
    reverse_members: bool,
) -> Option<IntervalEndpointInclusion> {
    let StatementConnectiveSyntax::JoikConnective(JoikConnectiveSyntax::ClosedIntervalConnective(
        connective,
    )) = connective
    else {
        return None;
    };
    let left = endpoint_inclusion_for_generated_cmavo(connective.left_interval.cmavo()?)?;
    let right = endpoint_inclusion_for_generated_cmavo(connective.right_interval.value.cmavo()?)?;
    if reverse_members {
        Some(IntervalEndpointInclusion {
            left: right,
            right: left,
        })
    } else {
        Some(IntervalEndpointInclusion { left, right })
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_tail_connective_formula_operator(
    connective: &BridiTailConnectiveSyntax,
) -> FormulaOperator {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => {
            generated_gihek_connective_formula_operator(connective)
        }
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_formula_operator(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_bridi_tail_connective_source(
    connective: &BridiTailConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => {
            Ok(generated_gihek_connective_source(connective))
        }
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_source(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
pub(super) fn generated_bridi_tail_connective_source_with_tense_modal(
    connective: &BridiTailConnectiveSyntax,
    tense_modal: Option<&TenseModalSyntax>,
) -> Result<String, SemanticsError> {
    let connective_source = generated_bridi_tail_connective_source(connective)?;
    let Some(tense_modal) = tense_modal else {
        return Ok(connective_source);
    };
    let Some((introduced_by, _relation, _visible_place)) =
        generated_modal_relation_spec_for_tense_modal(tense_modal)
    else {
        return Ok(connective_source);
    };
    Ok(format!("{connective_source} {introduced_by} bo"))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_tail_connective_has_se(
    connective: &BridiTailConnectiveSyntax,
) -> bool {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => connective.se.is_some(),
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_has_se(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_tail_connective_negates_left(
    connective: &BridiTailConnectiveSyntax,
) -> bool {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => connective.na.is_some(),
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_negates_left(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_tail_connective_negates_right(
    connective: &BridiTailConnectiveSyntax,
) -> bool {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => connective.nai.is_some(),
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_negates_right(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_bridi_tail_connective_truth_table(
    connective: &BridiTailConnectiveSyntax,
) -> Option<String> {
    match connective {
        BridiTailConnectiveSyntax::GihekConnective(connective) => {
            generated_gihek_connective_truth_table(connective)
        }
        BridiTailConnectiveSyntax::RelationConnectiveAsBridiTail(connective) => {
            generated_relation_afterthought_connective_truth_table(&connective.0)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi))))]
pub(super) fn generated_bridi_tail_connective_question_token(
    connective: &BridiTailConnectiveSyntax,
) -> Option<Token> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .into_iter()
        .find(|token| {
            matches!(
                token.cmavo(),
                Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi)
            )
        })
        .cloned()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_gihek_connective_source(connective: &GihekConnectiveSyntax) -> String {
    let mut tokens = Vec::new();
    if let Some(token) = &connective.na {
        tokens.push(token);
    }
    if let Some(token) = &connective.se {
        tokens.push(token);
    }
    tokens.push(&connective.giha.value);
    if let Some(token) = &connective.nai {
        tokens.push(&token.value);
    }
    connective_source_from_tokens(tokens)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_gihek_connective_formula_operator(
    connective: &GihekConnectiveSyntax,
) -> FormulaOperator {
    match connective.giha.value.cmavo() {
        Some(Cmavo::Giha) => FormulaOperator::Or,
        Some(Cmavo::Gihe) => FormulaOperator::And,
        Some(Cmavo::Giho) => FormulaOperator::Iff,
        Some(Cmavo::Gihu) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_gihek_connective_truth_table(
    connective: &GihekConnectiveSyntax,
) -> Option<String> {
    let operator = generated_gihek_connective_formula_operator(connective);
    let left_negated = connective.na.is_some();
    let right_negated = connective.nai.is_some();
    let se = connective.se.is_some();
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_formula_operator(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> FormulaOperator {
    match generated_relation_afterthought_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
        Some(Cmavo::Ji | Cmavo::Jehi) => FormulaOperator::ConnectiveQuestion,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(matches!(ret, RelationAfterthoughtConnectiveSyntax::JekConnective(_) | RelationAfterthoughtConnectiveSyntax::JoikConnective(_)))]
pub(super) fn relation_afterthought_connective_from_selbri(
    connective: &SelbriAfterthoughtConnectiveSyntax,
) -> RelationAfterthoughtConnectiveSyntax {
    match connective {
        SelbriAfterthoughtConnectiveSyntax::JekConnective(connective) => {
            RelationAfterthoughtConnectiveSyntax::JekConnective(connective.clone())
        }
        SelbriAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            RelationAfterthoughtConnectiveSyntax::JoikConnective(connective.clone())
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Jehi))))]
pub(super) fn generated_relation_afterthought_connective_question_token(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Option<Token> {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective)
            if connective.a.value.cmavo() == Some(Cmavo::Ji) =>
        {
            Some(connective.a.value.clone())
        }
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective)
            if connective.ja.value.cmavo() == Some(Cmavo::Jehi) =>
        {
            Some(connective.ja.value.clone())
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_primary_cmavo(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => {
            connective.a.value.cmavo()
        }
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => {
            connective.ja.value.cmavo()
        }
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_primary_cmavo(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            connective.0.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_joik_connective_primary_cmavo(
    connective: &JoikConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.joi.value.cmavo(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.bihi.value.cmavo(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.bihi.cmavo(),
        JoikConnectiveSyntax::ZantufaGahoJoikConnective(connective) => {
            connective.joiz.value.cmavo()
        }
        JoikConnectiveSyntax::ZantufaNaJoikConnective(connective) => connective.joiz.value.cmavo(),
        JoikConnectiveSyntax::ZantufaRightGahoJoikConnective(connective) => {
            connective.joiz.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_has_se(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.se.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.se.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_has_se(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_joik_connective_has_se(connective: &JoikConnectiveSyntax) -> bool {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::ZantufaGahoJoikConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::ZantufaNaJoikConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::ZantufaRightGahoJoikConnective(connective) => connective.se.is_some(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_negates_left(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.na.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.na.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(_) => false,
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_negates_right(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.nai.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.nai.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_negates_right(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_joik_connective_negates_right(connective: &JoikConnectiveSyntax) -> bool {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.nai.is_some(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.nai.is_some(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.nai.is_some(),
        JoikConnectiveSyntax::ZantufaGahoJoikConnective(_)
        | JoikConnectiveSyntax::ZantufaNaJoikConnective(_)
        | JoikConnectiveSyntax::ZantufaRightGahoJoikConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_relation_afterthought_connective_truth_table(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Option<String> {
    if generated_relation_afterthought_connective_question_token(connective).is_some() {
        return None;
    }
    if !generated_relation_afterthought_connective_is_logical(connective) {
        return None;
    }
    let operator = generated_relation_afterthought_connective_formula_operator(connective);
    let left_negated = generated_relation_afterthought_connective_negates_left(connective);
    let right_negated = generated_relation_afterthought_connective_negates_right(connective);
    let se = generated_relation_afterthought_connective_has_se(connective);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_afterthought_connective_is_logical(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    if generated_relation_afterthought_connective_question_token(connective).is_some() {
        return true;
    }
    matches!(
        generated_relation_afterthought_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
        )
    )
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn generated_nonlogical_composition_operator(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<CompositionOperator, SemanticsError> {
    if matches!(
        connective,
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_)
    ) {
        return Err(undefined_semantics(&format!(
            "the experimental VUhU relation connective `{}` outside a mekso expression",
            generated_relation_afterthought_connective_source(connective)?
        )));
    }
    match generated_relation_afterthought_connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => Ok(CompositionOperator::Joint),
        Some(Cmavo::Joi) => Ok(CompositionOperator::Mass),
        Some(Cmavo::Ce) => Ok(CompositionOperator::Set),
        Some(Cmavo::Ceho) => Ok(CompositionOperator::Sequence),
        Some(Cmavo::Fahu) => Ok(CompositionOperator::Respectively),
        Some(Cmavo::Johe) => Ok(CompositionOperator::Union),
        Some(Cmavo::Kuha) => Ok(CompositionOperator::Intersection),
        Some(Cmavo::Pihu) => Ok(CompositionOperator::CrossProduct),
        Some(Cmavo::Bihi) => Ok(CompositionOperator::UnorderedInterval),
        Some(Cmavo::Biho) => Ok(CompositionOperator::OrderedInterval),
        Some(Cmavo::Mihi) => Ok(CompositionOperator::CenteredInterval),
        _ => Err(invalid_graph(format!(
            "generated nonlogical relation connective `{}` has no composition operator",
            generated_relation_afterthought_connective_source(connective)?
        ))),
    }
}

#[requires(true)]
#[ensures(!ret || generated_relation_afterthought_connective_has_se(connective))]
pub(super) fn generated_relation_afterthought_connective_reverses_composition_members(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    generated_relation_afterthought_connective_has_se(connective)
        && matches!(
            generated_relation_afterthought_connective_primary_cmavo(connective),
            Some(Cmavo::Ceho | Cmavo::Fahu | Cmavo::Pihu | Cmavo::Biho | Cmavo::Mihi)
        )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_guhek_connective_source(connective: &GuhekConnectiveSyntax) -> String {
    let mut tokens = Vec::new();
    if let Some(token) = &connective.nahe {
        tokens.push(token);
    }
    if let Some(token) = &connective.se {
        tokens.push(token);
    }
    tokens.push(&connective.guha.value);
    if let Some(token) = &connective.nai {
        tokens.push(&token.value);
    }
    connective_source_from_tokens(tokens)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_guhek_gik_connective_source(
    guhek: &GuhekConnectiveSyntax,
    gik: &GikConnectiveSyntax,
) -> String {
    let mut parts = vec![
        generated_guhek_connective_source(guhek),
        token_text(&gik.gi.value),
    ];
    if let Some(nai) = &gik.nai {
        parts.push(token_text(&nai.value));
    }
    parts.join(" ")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_guhek_connective_formula_operator(
    connective: &GuhekConnectiveSyntax,
) -> FormulaOperator {
    match connective.guha.value.cmavo() {
        Some(Cmavo::Guha) => FormulaOperator::Or,
        Some(Cmavo::Guhe) => FormulaOperator::And,
        Some(Cmavo::Guho) => FormulaOperator::Iff,
        Some(Cmavo::Guhu) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_guhek_connective_has_se(connective: &GuhekConnectiveSyntax) -> bool {
    connective.se.is_some()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_guhek_connective_negates_left(connective: &GuhekConnectiveSyntax) -> bool {
    connective.nai.is_some()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_gik_connective_negates_right(connective: &GikConnectiveSyntax) -> bool {
    connective.nai.is_some()
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_guhek_gik_connective_truth_table(
    guhek: &GuhekConnectiveSyntax,
    gik: &GikConnectiveSyntax,
) -> Option<String> {
    generated_guhek_connective_truth_table_with_negations(
        guhek,
        generated_guhek_connective_negates_left(guhek),
        generated_gik_connective_negates_right(gik),
    )
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
pub(super) fn generated_guhek_connective_truth_table_with_negations(
    guhek: &GuhekConnectiveSyntax,
    left_negated: bool,
    right_negated: bool,
) -> Option<String> {
    let operator = generated_guhek_connective_formula_operator(guhek);
    let se = generated_guhek_connective_has_se(guhek);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(matches!(
    operator,
    FormulaOperator::And
        | FormulaOperator::Or
        | FormulaOperator::Iff
        | FormulaOperator::WhetherOrNot
))]
#[ensures(true)]
pub(super) fn connective_truth_value_for_operator(
    operator: FormulaOperator,
    left: bool,
    right: bool,
) -> bool {
    match operator {
        FormulaOperator::And => left && right,
        FormulaOperator::Or => left || right,
        FormulaOperator::Iff => left == right,
        FormulaOperator::WhetherOrNot => left,
        _ => unreachable!("precondition restricts connective truth operators"),
    }
}

#[requires(matches!(
    operator,
    FormulaOperator::And
        | FormulaOperator::Or
        | FormulaOperator::Iff
        | FormulaOperator::WhetherOrNot
))]
#[ensures(ret.len() == 4)]
pub(super) fn generated_truth_table_for_formula_operator(operator: FormulaOperator) -> String {
    [(true, true), (true, false), (false, true), (false, false)]
        .into_iter()
        .map(|(left, right)| {
            if connective_truth_value_for_operator(operator, left, right) {
                'T'
            } else {
                'F'
            }
        })
        .collect()
}
