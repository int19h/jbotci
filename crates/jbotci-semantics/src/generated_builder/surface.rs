use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_quantifier_formula_operator(
    quantifier: &QuantifierSyntax,
) -> FormulaOperator {
    let mut visitor = GeneratedSpanCollector::default();
    quantifier.visit_in_order(&mut visitor);
    match token_list_text(visitor.tokens.iter().copied()).as_str() {
        "ro" => FormulaOperator::Forall,
        "no" => FormulaOperator::None,
        _ => FormulaOperator::Cardinality,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn quantifier_tokens(quantifier: &QuantifierSyntax) -> Vec<Token> {
    let mut visitor = GeneratedSpanCollector::default();
    quantifier.visit_in_order(&mut visitor);
    visitor.tokens.into_iter().cloned().collect()
}

#[requires(true)]
#[ensures(source_text.is_none() -> ret.as_slice() == spans)]
pub(super) fn source_spans_with_following_cmevla_period(
    spans: &[SourceSpan],
    source_text: Option<&str>,
) -> Vec<SourceSpan> {
    let Some(source_text) = source_text else {
        return spans.to_vec();
    };
    let Some((last_index, last_span)) = spans
        .iter()
        .enumerate()
        .max_by_key(|(_, span)| span.byte_end)
    else {
        return Vec::new();
    };
    let Some(period) = source_text
        .get(last_span.byte_end..)
        .and_then(|tail| tail.chars().next())
        .filter(|period| jbotci_morphology::is_period_character(*period))
    else {
        return spans.to_vec();
    };
    let Ok(expanded) = SourceSpan::new(
        last_span.source_id.clone(),
        last_span.byte_start,
        last_span.byte_end + period.len_utf8(),
        last_span.char_start,
        last_span.char_end + 1,
    ) else {
        return spans.to_vec();
    };
    let mut expanded_spans = spans.to_vec();
    expanded_spans[last_index] = expanded;
    expanded_spans
}

#[requires(true)]
#[ensures(true)]
pub(super) fn token_list_text<'a>(tokens: impl Iterator<Item = &'a Token>) -> String {
    let mut text = String::new();
    push_token_list_text(&mut text, tokens);
    text
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
pub(super) fn push_token_list_text<'a>(
    output: &mut String,
    tokens: impl Iterator<Item = &'a Token>,
) {
    let mut first = true;
    for token in tokens {
        if first {
            first = false;
        } else {
            output.push(' ');
        }
        push_token_text(output, token);
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn non_empty_token_list_text<'a>(
    tokens: impl Iterator<Item = &'a Token>,
) -> Option<String> {
    let text = token_list_text(tokens);
    if text.is_empty() { None } else { Some(text) }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_selbri_surface_text(
    selbri: &SelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    selbri.visit_in_order(&mut visitor);
    non_empty_token_list_text(visitor.tokens.iter().copied()).map_or_else(
        || relation_label_from_selbri(selbri).map(|label| label.display_text()),
        Ok,
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_vocative_kind_for_markers(
    markers: &WithFreeModifiers<VocativeMarkerWordsSyntax, FreeModifierSyntax>,
) -> String {
    let first = generated_vocative_marker_first_token(&markers.value);
    match first.cmavo() {
        Some(Cmavo::Coi) => "greeting".to_owned(),
        Some(Cmavo::Jehe) => "acknowledgement".to_owned(),
        Some(Cmavo::Coho) => "farewell".to_owned(),
        Some(Cmavo::Fihi) => "welcome".to_owned(),
        Some(Cmavo::Mihe) => "selfIdentification".to_owned(),
        Some(Cmavo::Doi) => "address".to_owned(),
        _ => token_text(first),
    }
}

#[requires(true)]
#[ensures(!token_text(ret).is_empty())]
pub(super) fn generated_vocative_marker_first_token(markers: &VocativeMarkerWordsSyntax) -> &Token {
    match markers {
        VocativeMarkerWordsSyntax::CoiVocativeMarkerWords(markers) => &markers.first_coi,
        VocativeMarkerWordsSyntax::DoiVocativeMarkerWords(markers) => &markers.0,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|definition| crate::model::argument_object_kind_can_fill(definition.value.object_kind())))]
pub(super) fn scalar_scale_definition_for_modal_argument(
    modal_argument: &ModalArgument,
) -> Option<GeneratedScalarScaleDefinition> {
    modal_argument.relation.as_ref()?;
    if modal_argument.introduced_by != "ci'u" {
        return None;
    }
    let value = modal_argument.arguments.get(&argument_key(1))?.value?;
    Some(GeneratedScalarScaleDefinition::from_data(data!(
        GeneratedScalarScaleDefinition {
            value,
            introduced_by: modal_argument.introduced_by.clone(),
            source: modal_argument.source.clone(),
        }
    )))
}

#[requires(true)]
#[ensures(ret.construct.as_deref() == Some("scalar-scale"))]
pub(super) fn source_as_scalar_scale(
    source: crate::model::SemanticSource,
) -> crate::model::SemanticSource {
    crate::model::SemanticSource {
        construct: Some("scalar-scale".to_owned()),
        ..source
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|source| source.construct.as_deref() == Some("name-denotation")))]
pub(super) fn source_as_name_denotation(
    source: Option<crate::model::SemanticSource>,
) -> Option<crate::model::SemanticSource> {
    source.map(|source| crate::model::SemanticSource {
        construct: Some("name-denotation".to_owned()),
        ..source
    })
}

#[requires(!construct.is_empty())]
#[ensures(ret.as_ref().is_none_or(|source| source.construct.as_deref() == Some(construct)))]
pub(super) fn source_with_construct(
    source: Option<crate::model::SemanticSource>,
    construct: &str,
) -> Option<crate::model::SemanticSource> {
    source.map(|source| crate::model::SemanticSource {
        construct: Some(construct.to_owned()),
        ..source
    })
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
pub(super) fn scalar_negation_for_marker<F>(
    marker: &WithFreeModifiers<Token, F>,
) -> ScalarNegation {
    scalar_negation_for_token(&marker.value)
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
pub(super) fn scalar_negation_for_token(token: &Token) -> ScalarNegation {
    ScalarNegation::new(
        scalar_negation_kind_for_cmavo(token.cmavo()),
        token_text(token),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn scalar_negation_kind_for_cmavo(cmavo: Option<Cmavo>) -> ScalarNegationKind {
    match cmavo {
        Some(Cmavo::Tohe) => ScalarNegationKind::Opposite,
        Some(Cmavo::Nohe) => ScalarNegationKind::Neutral,
        Some(Cmavo::Jeha) => ScalarNegationKind::Affirmed,
        _ => ScalarNegationKind::OtherThan,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn scalar_negated_sumti_qualifier_kind(cmavo: Option<Cmavo>) -> DescriptorKind {
    match cmavo {
        Some(Cmavo::Tohe) => DescriptorKind::OppositeOf,
        Some(Cmavo::Nohe) => DescriptorKind::NeutralOf,
        Some(Cmavo::Jeha) => DescriptorKind::AffirmedAs,
        _ => DescriptorKind::OtherThan,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn descriptor_definiteness_for_scalar_negated_sumti(
    cmavo: Option<Cmavo>,
) -> Option<DescriptorDefiniteness> {
    match cmavo {
        Some(Cmavo::Tohe) => Some(DescriptorDefiniteness::UniqueExtreme),
        Some(Cmavo::Nohe) => Some(DescriptorDefiniteness::NeutralPoint),
        Some(Cmavo::Jeha) => Some(DescriptorDefiniteness::AffirmedPoint),
        _ => Some(DescriptorDefiniteness::IndefiniteAlternative),
    }
}

#[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(true)]
pub(super) fn bind_generated_modal_argument_to_host_event(
    modal_argument: &mut ModalArgument,
    eventuality: SemanticObjectId,
) {
    let _ =
        bind_generated_modal_argument_to_host_event_preserving_elision(modal_argument, eventuality);
}

#[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(ret.as_ref().is_none_or(|elision| elision.argument.kind == ArgumentValueKind::Elided))]
pub(super) fn bind_generated_modal_argument_to_host_event_preserving_elision(
    modal_argument: &mut ModalArgument,
    eventuality: SemanticObjectId,
) -> Option<GeneratedHostEventModalElision> {
    if modal_argument.relation.is_none() {
        return None;
    }
    let relation = modal_argument.relation.as_ref()?.clone();
    let Some(place) = generated_modal_relation_host_event_place_for_argument(modal_argument) else {
        return None;
    };
    let key = argument_key(place);
    if modal_argument
        .arguments
        .get(&key)
        .is_some_and(|argument| argument.kind != ArgumentValueKind::Elided)
    {
        return None;
    }
    let original_elision = modal_argument.arguments.get(&key).cloned();
    let mut data = modal_argument.clone().into_data();
    data.arguments
        .insert(key.clone(), ArgumentValue::filled(eventuality, None));
    *modal_argument = ModalArgument::from_data(data);
    original_elision.map(|argument| {
        new!(GeneratedHostEventModalElision {
            relation,
            introduced_by: modal_argument.introduced_by.clone(),
            source: modal_argument.source.clone(),
            place,
            argument,
        })
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
pub(super) fn generated_modal_relation_host_event_place_for_argument(
    modal_argument: &ModalArgument,
) -> Option<usize> {
    let relation = modal_argument.relation.as_deref()?;
    if generated_modal_relation_has_complementary_event_places(relation)
        && generated_modal_argument_place_is_filled(modal_argument, 2)
        && !generated_modal_argument_place_is_filled(modal_argument, 1)
    {
        return Some(1);
    }
    generated_modal_relation_host_event_place(relation)
}

#[requires(place > 0)]
#[ensures(true)]
pub(super) fn generated_modal_argument_place_is_filled(
    modal_argument: &ModalArgument,
    place: usize,
) -> bool {
    modal_argument
        .arguments
        .get(&argument_key(place))
        .is_some_and(|argument| argument.kind != ArgumentValueKind::Elided)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_modal_relation_has_complementary_event_places(relation: &str) -> bool {
    matches!(relation, "krinu" | "mukti" | "nibli" | "rinka")
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
pub(super) fn generated_modal_relation_host_event_place(relation: &str) -> Option<usize> {
    match relation {
        "bapli" | "gasnu" | "krinu" | "mukti" | "nibli" | "rinka" => Some(2),
        "pilno" => Some(3),
        _ => None,
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
pub(super) fn invalid_graph(message: String) -> SemanticsError {
    SemanticsError {
        kind: SemanticsErrorKind::InvalidGraph,
        message: format!("semantic graph invariant failed: {message}"),
    }
}

#[requires(true)]
#[ensures(ret.is_displayable())]
pub(super) fn relation_label_from_token(token: &Token) -> RelationLabel {
    let text = token_text(token);
    match token.core_word().as_data() {
        data!(WordLike::ZeiCompound { .. }) => RelationLabel::zei_compound(text),
        data!(WordLike::PlainWord(word)) => match word.cmavo() {
            Some(Cmavo::Du) => RelationLabel::du(),
            Some(cmavo) if generated_relation_is_pro_bridi_label(cmavo.canonical_text()) => {
                RelationLabel::pro_bridi(text)
            }
            _ => match word.kind() {
                jbotci_morphology::WordKind::Gismu
                | jbotci_morphology::WordKind::Lujvo
                | jbotci_morphology::WordKind::Fuhivla => RelationLabel::brivla(text),
                _ => RelationLabel::constructed(text),
            },
        },
        _ => RelationLabel::constructed(text),
    }
}

#[requires(relation.is_displayable())]
#[ensures(ret.is_displayable())]
pub(super) fn semantic_relation_label(relation: RelationLabel) -> RelationLabel {
    if matches!(relation.as_data(), data!(RelationLabel::Du)) {
        RelationLabel::identity()
    } else {
        relation
    }
}

#[requires(place > 0)]
#[ensures(ret.get() == place)]
pub(super) fn argument_key(place: usize) -> PlaceIndex {
    PlaceIndex::new(place)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn token_text(token: &Token) -> String {
    let mut text = String::new();
    push_token_text(&mut text, token);
    text
}

#[requires(true)]
#[ensures(output.len() > old(output.len()))]
pub(super) fn push_token_text(output: &mut String, token: &Token) {
    if let Some(word) = token.core_word().bare_word() {
        push_word_text(output, word);
    } else {
        output.push_str(&token.to_string());
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn quote_delimiter_text(token: &Token) -> String {
    let mut text = String::new();
    push_quote_delimiter_text(&mut text, token);
    text
}

#[requires(true)]
#[ensures(output.len() > old(output.len()))]
pub(super) fn push_quote_delimiter_text(output: &mut String, token: &Token) {
    match token.core_word().as_data() {
        data!(WordLike::DelimitedNonLojbanQuote {
            opening_delimiter,
            ..
        }) => push_word_text(output, opening_delimiter),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => push_word_text(output, marker),
        _ => push_token_text(output, token),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn word_text(word: &Word) -> String {
    let mut text = String::new();
    push_word_text(&mut text, word);
    text
}

#[requires(true)]
#[ensures(output.len() > old(output.len()))]
pub(super) fn push_word_text(output: &mut String, word: &Word) {
    push_stripped_diacritics_to(word.phonemes().as_str(), output);
}

#[requires(!what.is_empty())]
#[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
pub(super) fn unsupported(what: &str) -> SemanticsError {
    SemanticsError {
        kind: SemanticsErrorKind::InvalidGraph,
        message: format!("generated semantic builder does not yet support {what}"),
    }
}

#[requires(!what.is_empty())]
#[ensures(ret.kind == SemanticsErrorKind::RequiresDiscourseContext)]
pub(super) fn requires_discourse_context(what: &str) -> SemanticsError {
    SemanticsError::requires_discourse_context(what)
}
