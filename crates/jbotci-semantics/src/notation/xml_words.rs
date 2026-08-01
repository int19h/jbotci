//! `<WORDS>` word-card section emission for the SFN-XML renderer (#709).
//!
//! This is the XML emission layer for the word-card model built by
//! [`super::word_cards`]: one `<WORD>` card per content word, dictionary
//! prose (`GLOSS`/`DEF`/`NOTES`) for known words with `<ARG INDEX="n"/>` place
//! markup inside `DEF`/`NOTES`, and the `COMPOSITE-APPROX` composition tree
//! plus nonce `WARNING`s for dictionary-absent compounds. The tree vocabulary
//! deliberately mirrors the body notation's idioms (`KIND-COMPOSITION` with
//! `KIND` first, the `QUANTITY` `FORM`/`SCALE`/`VALUE` shape, `CONNECTIVE
//! OPERATOR=` underscore tokens) so the card section reads as the same
//! notation, not a parallel one.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_dictionary::places::{
    DefinitionPlaceMap, DefinitionPlaceSegmentData, definition_place_segments_for_definition_line,
    definition_place_segments_for_notes_line,
};

use super::word_cards::{
    ApproxConnective, ApproxExpr, ApproxExprData, ApproxQuantityForm, ApproxReferent,
    ApproxReferentData, CompositeApprox, GroupingBasis, Inclusion, LogicalVariableSort,
    ParameterRole, Proximity, ScalarNegationPolarity, ScopeBasis, VariableContextRole, WordCard,
};
use super::xml::{MixedContent, XmlElement};
use crate::model::{AbstractionKind, ActualityKind};

/// Which definition-place segmentation rule applies to the line.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionLineKind {
    Definition,
    Notes,
}

/// The `<WORDS>` section: one `<WORD>` card per card, in card order.
#[requires(!cards.is_empty())]
#[ensures(ret.name == "WORDS")]
#[ensures(ret.children.len() == cards.len())]
pub(crate) fn words_section(cards: &[WordCard]) -> XmlElement {
    let mut section = XmlElement::new("WORDS");
    for card in cards {
        section.push(word_card_element(card));
    }
    section
}

/// One `<WORD ID="...">` card: `KNOWN="false"` only for dictionary-absent
/// words; then `GLOSS`*, `DEF`?, `NOTES`?, `COMPOSITE-APPROX`?, `WARNING`* in
/// fixed child order (warnings always follow the composition approximation).
#[requires(true)]
#[ensures(ret.name == "WORD")]
fn word_card_element(card: &WordCard) -> XmlElement {
    let mut element = XmlElement::with_attributes("WORD", [("ID", card.id.clone())]);
    if !card.known {
        element.set("KNOWN", "false");
    }
    for gloss in &card.glosses {
        element.push(text_element("GLOSS", gloss));
    }
    if let Some(definition) = &card.definition {
        let place_map = DefinitionPlaceMap::from_definition(definition);
        element.push(place_segmented_element(
            "DEF",
            definition,
            &place_map,
            DefinitionLineKind::Definition,
        ));
        if let Some(notes) = &card.notes {
            element.push(place_segmented_element(
                "NOTES",
                notes,
                &place_map,
                DefinitionLineKind::Notes,
            ));
        }
    }
    if let Some(composition) = &card.composition {
        element.push(composite_approx_element(composition));
    }
    for warning in &card.warnings {
        element.push(text_element("WARNING", warning));
    }
    element
}

/// An element whose whole content is one text run (`GLOSS`, `WARNING`).
#[requires(!name.is_empty())]
#[requires(!text.is_empty())]
#[ensures(ret.name == name)]
fn text_element(name: &str, text: &str) -> XmlElement {
    let mut element = XmlElement::new(name);
    element.text = Some(text.to_owned());
    element
}

/// A `DEF` or `NOTES` element as mixed content: text runs interleaved with
/// inline `<ARG INDEX="n"/>` place markers. Notes-line variables the entry's
/// definition blocks never mapped render verbatim in the jbovlaste
/// `$letter_{index}$` surface form, matching the definition-line convention
/// of preserving unmapped variables as `$...$` text.
#[requires(!name.is_empty())]
#[requires(!input.is_empty())]
#[ensures(ret.name == name)]
fn place_segmented_element(
    name: &str,
    input: &str,
    place_map: &DefinitionPlaceMap,
    line_kind: DefinitionLineKind,
) -> XmlElement {
    let segments = match line_kind {
        DefinitionLineKind::Definition => {
            definition_place_segments_for_definition_line(input, place_map)
        }
        DefinitionLineKind::Notes => definition_place_segments_for_notes_line(input, place_map),
    };
    let mut element = XmlElement::new(name);
    for segment in segments {
        match segment.as_data() {
            data!(DefinitionPlaceSegment::Text(text)) => {
                element.push_mixed(MixedContent::Text(text.clone()));
            }
            data!(DefinitionPlaceSegment::Place(place)) => {
                element.push_mixed(MixedContent::Element(XmlElement::with_attributes(
                    "ARG",
                    [("INDEX", place.to_string())],
                )));
            }
            data!(DefinitionPlaceSegment::UnmappedVariable { letter, index }) => {
                element.push_mixed(MixedContent::Text(format!("${letter}_{{{index}}}")));
            }
        }
    }
    element
}

/// `<COMPOSITE-APPROX PLACES="UNKNOWN" [GROUPING=] [SCOPE=]>` wrapping the
/// composition tree. `PLACES="UNKNOWN"` is a constant: the tree determines no
/// place structure (see the `compound-places` KEY rule).
#[requires(true)]
#[ensures(ret.name == "COMPOSITE-APPROX")]
fn composite_approx_element(composition: &CompositeApprox) -> XmlElement {
    let mut element = XmlElement::with_attributes("COMPOSITE-APPROX", [("PLACES", "UNKNOWN")]);
    if let Some(grouping) = composition.grouping {
        element.set("GROUPING", grouping_basis_token(grouping));
    }
    if let Some(scope) = composition.scope {
        element.set("SCOPE", scope_basis_token(scope));
    }
    element.push(approx_expr_element(&composition.root));
    element
}

/// One node of the composition approximation tree, in fixed child order.
#[requires(true)]
#[ensures(!ret.name.is_empty())]
fn approx_expr_element(expr: &ApproxExpr) -> XmlElement {
    match expr.as_data() {
        data!(ApproxExpr::Component { word }) => {
            XmlElement::with_attributes("COMPONENT", [("WORD", word.clone())])
        }
        data!(ApproxExpr::KindComposition {
            kind,
            modifier,
            grouping,
        }) => {
            // KIND (the place-structure-bearing head) precedes MODIFIER,
            // mirroring the body's tanruLink emission.
            let mut element = XmlElement::new("KIND-COMPOSITION");
            if let Some(grouping) = grouping {
                element.set("GROUPING", grouping_basis_token(*grouping));
            }
            let mut kind_element = XmlElement::new("KIND");
            kind_element.push(approx_expr_element(kind));
            element.push(kind_element);
            let mut modifier_element = XmlElement::new("MODIFIER");
            modifier_element.push(approx_expr_element(modifier));
            element.push(modifier_element);
            element
        }
        data!(ApproxExpr::SwappedPlaces {
            first,
            second,
            inner,
            scope,
        }) => {
            let mut element = XmlElement::with_attributes(
                "SWAPPED-PLACES",
                [("PLACES", format!("{first} {second}"))],
            );
            if let Some(scope) = scope {
                element.set("SCOPE", scope_basis_token(*scope));
            }
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::ScalarNegation {
            polarity,
            inner,
            scope,
        }) => {
            let mut element = XmlElement::with_attributes(
                "SCALAR-NEGATION",
                [("POLARITY", scalar_negation_polarity_token(*polarity))],
            );
            if let Some(scope) = scope {
                element.set("SCOPE", scope_basis_token(*scope));
            }
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::PredicationNegation { inner }) => {
            let mut element = XmlElement::new("PREDICATION-NEGATION");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Abstraction { kind, inner, scope }) => {
            let mut element = XmlElement::with_attributes(
                "ABSTRACTION",
                [("KIND", abstraction_kind_token(*kind))],
            );
            if let Some(scope) = scope {
                element.set("SCOPE", scope_basis_token(*scope));
            }
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Connective {
            operator,
            left,
            right,
        }) => {
            let mut element = XmlElement::with_attributes(
                "CONNECTIVE",
                [("OPERATOR", connective_token(*operator))],
            );
            element.push(approx_expr_element(left));
            element.push(approx_expr_element(right));
            element
        }
        data!(ApproxExpr::TaggedPlace { inner }) => {
            let mut element = XmlElement::new("TAGGED-PLACE");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::PlaceDeletion { index, inner }) => {
            let mut element =
                XmlElement::with_attributes("PLACE-DELETION", [("INDEX", index.to_string())]);
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Figurative { inner }) => {
            let mut element = XmlElement::new("FIGURATIVE");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Identity) => XmlElement::new("IDENTITY"),
        data!(ApproxExpr::Quantity { form, value, text }) => {
            // The body's count-context quantities all carry SCALE="COUNT";
            // card quantities are counts of components by construction.
            let mut element = XmlElement::with_attributes(
                "QUANTITY",
                [("FORM", quantity_form_token(*form)), ("SCALE", "COUNT")],
            );
            if let Some(value) = value {
                let mut value_element = XmlElement::new("VALUE");
                value_element.push(XmlElement::with_attributes(
                    "INTEGER",
                    [("VALUE", value.to_string())],
                ));
                element.push(value_element);
            } else if let Some(text) = text {
                // The body omits non-integer quantity text behind a waiver
                // (XmlWaiverFamily::QuantityText); cards must not lose the
                // value, so they carry it as a TEXT leaf in the same VALUE
                // slot the body's INTEGER occupies.
                let mut value_element = XmlElement::new("VALUE");
                value_element.push(XmlElement::with_attributes(
                    "TEXT",
                    [("VALUE", text.clone())],
                ));
                element.push(value_element);
            }
            element
        }
        data!(ApproxExpr::Ordinal { inner }) => {
            let mut element = XmlElement::new("ORDINAL");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Cardinal { inner }) => {
            let mut element = XmlElement::new("CARDINAL");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Recurrence { inner }) => {
            let mut element = XmlElement::new("RECURRENCE");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::LetterOf { inner }) => {
            let mut element = XmlElement::new("LETTER");
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::Letter { text }) => {
            XmlElement::with_attributes("LETTER", [("TEXT", text.clone())])
        }
        data!(ApproxExpr::TenseModal {
            actuality,
            aspect,
            space_whole,
            time_whole,
            inner,
        }) => {
            let mut element = XmlElement::new("TENSE-MODAL");
            if let Some(actuality) = actuality {
                element.set("ACTUALITY", actuality_kind_token(*actuality));
            }
            if let Some(aspect) = aspect {
                element.set("ASPECT", aspect.model_contour().to_uppercase());
            }
            if *space_whole {
                element.set("SPACE", "WHOLE");
            }
            if *time_whole {
                element.set("TIME", "WHOLE");
            }
            element.push(approx_expr_element(inner));
            element
        }
        data!(ApproxExpr::ReferentOf { referent }) => {
            let mut element = XmlElement::new("REFERENT-OF");
            element.push(approx_referent_element(referent));
            element
        }
        data!(ApproxExpr::VariableContext {
            role,
            proximity,
            slot,
        }) => variable_context_element(*role, *proximity, *slot),
    }
}

/// One sumti-like referent inside `REFERENT-OF` (or the namer inside `BY`).
#[requires(true)]
#[ensures(!ret.name.is_empty())]
fn approx_referent_element(referent: &ApproxReferent) -> XmlElement {
    match referent.as_data() {
        data!(ApproxReferent::Context {
            role,
            proximity,
            slot,
        }) => variable_context_element(*role, *proximity, *slot),
        data!(ApproxReferent::Named { text, by }) => {
            // The body's NAMED carries BY only when the namer anchor differs
            // from the enclosing speaker; card scope has no anchors, so BY
            // appears exactly when the model supplies a namer and wraps the
            // namer structure instead of a REF.
            let mut element = XmlElement::with_attributes("NAMED", [("TEXT", text.clone())]);
            if let Some(by) = by {
                let mut by_element = XmlElement::new("BY");
                by_element.push(approx_referent_element(by));
                element.push(by_element);
            }
            element
        }
        data!(ApproxReferent::Unspecified) => XmlElement::new("UNSPECIFIED-REFERENT"),
        data!(ApproxReferent::PersonalMass {
            speaker,
            audience,
            others,
        }) => {
            // The body points SPEAKER/AUDIENCE/OTHERS children at referent
            // ids; card scope defines no referents, so membership is carried
            // as attributes with OTHERS="true" presence semantics.
            let mut element = XmlElement::with_attributes(
                "PERSONAL-MASS-MEMBERSHIP",
                [
                    ("SPEAKER", inclusion_token(*speaker)),
                    ("AUDIENCE", inclusion_token(*audience)),
                ],
            );
            if *others {
                element.set("OTHERS", "true");
            }
            element
        }
        data!(ApproxReferent::LogicalVariable { sort, series }) => {
            let mut element = XmlElement::with_attributes(
                "LOGICAL-VARIABLE",
                [("SORT", logical_variable_sort_token(*sort))],
            );
            element.set("SERIES", series.to_string());
            element.set("BINDING", "IMPLICIT-EXISTENTIAL");
            element
        }
        data!(ApproxReferent::Parameter { role }) => {
            XmlElement::with_attributes("PARAMETER", [("ROLE", parameter_role_token(*role))])
        }
    }
}

/// `<VARIABLE-CONTEXT ROLE=...>`: the abstract context role a pro-word
/// denotes in discourse-free card scope. These are roles, not referents.
#[requires(true)]
#[ensures(ret.name == "VARIABLE-CONTEXT")]
fn variable_context_element(
    role: VariableContextRole,
    proximity: Option<Proximity>,
    slot: Option<u8>,
) -> XmlElement {
    let mut element =
        XmlElement::with_attributes("VARIABLE-CONTEXT", [("ROLE", context_role_token(role))]);
    if let Some(proximity) = proximity {
        element.set("PROXIMITY", proximity_token(proximity));
    }
    if let Some(slot) = slot {
        element.set("SLOT", slot.to_string());
    }
    element
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn grouping_basis_token(basis: GroupingBasis) -> &'static str {
    match basis {
        GroupingBasis::Explicit => "EXPLICIT",
        GroupingBasis::AssumedLeft => "ASSUMED-LEFT",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn scope_basis_token(basis: ScopeBasis) -> &'static str {
    match basis {
        ScopeBasis::Explicit => "EXPLICIT",
        ScopeBasis::AssumedShort => "ASSUMED-SHORT",
    }
}

/// Underscore enum tokens, matching the body's `enum_token` convention.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn connective_token(operator: ApproxConnective) -> &'static str {
    match operator {
        ApproxConnective::Or => "OR",
        ApproxConnective::And => "AND",
        ApproxConnective::Iff => "IFF",
        ApproxConnective::WhetherOr => "WHETHER_OR",
        ApproxConnective::Mass => "MASS",
        ApproxConnective::Set => "SET",
        ApproxConnective::Sequence => "SEQUENCE",
        ApproxConnective::Union => "UNION",
        ApproxConnective::Joint => "JOINT",
        ApproxConnective::Intersection => "INTERSECTION",
        ApproxConnective::CartesianProduct => "CARTESIAN_PRODUCT",
        ApproxConnective::Interval => "INTERVAL",
    }
}

/// Underscore enum tokens, matching the body's `QUANTITY FORM=` convention.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn quantity_form_token(form: ApproxQuantityForm) -> &'static str {
    match form {
        ApproxQuantityForm::Exact => "EXACT",
        ApproxQuantityForm::All => "ALL",
        ApproxQuantityForm::AllBut => "ALL_BUT",
        ApproxQuantityForm::AtLeast => "AT_LEAST",
        ApproxQuantityForm::AtMost => "AT_MOST",
        ApproxQuantityForm::TooFew => "TOO_FEW",
        ApproxQuantityForm::AlmostAll => "ALMOST_ALL",
        ApproxQuantityForm::Most => "MOST",
        ApproxQuantityForm::Many => "MANY",
        ApproxQuantityForm::Few => "FEW",
    }
}

/// PascalCase kind tokens: abstraction kinds are sort-like, and the body
/// renders sorts as flat PascalCase names.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn abstraction_kind_token(kind: AbstractionKind) -> &'static str {
    match kind {
        AbstractionKind::Event => "Event",
        AbstractionKind::Achievement => "Achievement",
        AbstractionKind::Process => "Process",
        AbstractionKind::State => "State",
        AbstractionKind::Activity => "Activity",
        AbstractionKind::Property => "Property",
        AbstractionKind::Amount => "Amount",
        AbstractionKind::TruthValue => "TruthValue",
        AbstractionKind::Concept => "Concept",
        AbstractionKind::Proposition => "Proposition",
        AbstractionKind::Experience => "Experience",
        AbstractionKind::Unspecified => "Unspecified",
        // The closed operator table never classifies a sentence-sign
        // abstraction, so card trees cannot contain one.
        AbstractionKind::SentenceSign => {
            unreachable!("sentence-sign abstractions are not renderable in word cards")
        }
    }
}

/// Uppercase tokens of the model's CAhA actuality facets.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn actuality_kind_token(actuality: ActualityKind) -> &'static str {
    match actuality {
        ActualityKind::Actual => "ACTUAL",
        ActualityKind::Capable => "CAPABLE",
        ActualityKind::Potential => "POTENTIAL",
        ActualityKind::Demonstrated => "DEMONSTRATED",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn scalar_negation_polarity_token(polarity: ScalarNegationPolarity) -> &'static str {
    match polarity {
        ScalarNegationPolarity::Other => "OTHER",
        ScalarNegationPolarity::Neutral => "NEUTRAL",
        ScalarNegationPolarity::Opposite => "OPPOSITE",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn context_role_token(role: VariableContextRole) -> &'static str {
    match role {
        VariableContextRole::Speaker => "SPEAKER",
        VariableContextRole::Audience => "AUDIENCE",
        VariableContextRole::Demonstrated => "DEMONSTRATED",
        VariableContextRole::Assigned => "ASSIGNED",
        VariableContextRole::EllipticalPredicate => "ELLIPTICAL-PREDICATE",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn proximity_token(proximity: Proximity) -> &'static str {
    match proximity {
        Proximity::Proximal => "PROXIMAL",
        Proximity::Medial => "MEDIAL",
        Proximity::Distal => "DISTAL",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn inclusion_token(inclusion: Inclusion) -> &'static str {
    match inclusion {
        Inclusion::Included => "INCLUDED",
        Inclusion::Excluded => "EXCLUDED",
    }
}

/// PascalCase sort names, matching the body's `SORT=` convention.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn logical_variable_sort_token(sort: LogicalVariableSort) -> &'static str {
    match sort {
        LogicalVariableSort::Entity => "Entity",
        LogicalVariableSort::Predicate => "Predicate",
    }
}

/// Underscore enum tokens, matching the body's `PARAMETER ROLE=` convention.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn parameter_role_token(role: ParameterRole) -> &'static str {
    match role {
        ParameterRole::PropertySlot => "PROPERTY_SLOT",
        ParameterRole::Argument => "ARGUMENT",
    }
}

#[cfg(test)]
mod tests {
    use jbotci_dictionary::Dictionary;
    use jbotci_morphology::segment_words_with_modifiers;

    use super::*;
    use crate::notation::word_cards::build_xml_word_cards;
    use crate::notation::xml::serialize;

    #[requires(true)]
    #[ensures(true)]
    fn dictionary() -> &'static Dictionary<'static> {
        jbotci_dictionary_data::english()
    }

    /// Parse `text` with the real morphology segmenter and build its cards.
    #[requires(true)]
    #[ensures(true)]
    fn cards_for(text: &str) -> Vec<WordCard> {
        let words = segment_words_with_modifiers(text)
            .unwrap_or_else(|error| panic!("test input `{text}` must segment: {error:?}"));
        build_xml_word_cards(dictionary(), &words)
    }

    /// Serialize the `<WORDS>` section for the cards of `text`.
    #[requires(true)]
    #[ensures(ret.starts_with("<WORDS>\n"))]
    #[ensures(ret.ends_with("</WORDS>\n"))]
    fn words_xml_for(text: &str) -> String {
        serialize(&words_section(&cards_for(text)), None)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn known_gismu_card_xml_shape() {
        let xml = words_xml_for("barda");
        assert_eq!(
            xml,
            "<WORDS>\n  <WORD ID=\"barda\">\n    <GLOSS>big</GLOSS>\n    <GLOSS>large</GLOSS>\n    <DEF><ARG INDEX=\"1\"/> is big/large in property/dimension(s) <ARG INDEX=\"2\"/> (ka) as compared with standard/norm <ARG INDEX=\"3\"/>.</DEF>\n    <NOTES>See also {banli}, {clani}, {ganra}, {condi}, {plana}, {cmalu}, {rotsu}, {banro}, {xanto}.</NOTES>\n  </WORD>\n</WORDS>\n"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chained_alias_variables_resolve_to_the_leading_place() {
        let xml = words_xml_for("bavlamdei");
        assert!(
            xml.contains(
                "<DEF><ARG INDEX=\"1\"/> is tomorrow; <ARG INDEX=\"1\"/> is the day following <ARG INDEX=\"2\"/>, day standard <ARG INDEX=\"3\"/>.</DEF>"
            ),
            "$d_1=b_1=l_1$ must resolve to ARG INDEX=\"1\": {xml}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_gismu_card_is_bare_known_false() {
        let xml = words_xml_for("sfoto");
        assert_eq!(xml, "<WORDS>\n  <WORD ID=\"sfoto\" KNOWN=\"false\"/>\n</WORDS>\n");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_lujvo_card_carries_composite_approx_and_warning() {
        let xml = words_xml_for("skamymlatu");
        assert!(
            xml.contains(
                "<WORD ID=\"skamymlatu\" KNOWN=\"false\">\n    <COMPOSITE-APPROX PLACES=\"UNKNOWN\">\n      <KIND-COMPOSITION>\n        <KIND>\n          <COMPONENT WORD=\"mlatu\"/>\n        </KIND>\n        <MODIFIER>\n          <COMPONENT WORD=\"skami\"/>\n        </MODIFIER>\n      </KIND-COMPOSITION>\n    </COMPOSITE-APPROX>\n    <WARNING>"
            ),
            "skamymlatu card must carry the kind-composition tree: {xml}"
        );
        // Two components: no GROUPING/SCOPE basis attributes.
        let card = xml.split("<WORD ID=\"skamymlatu\"").nth(1).expect("card");
        let card = card.split("</WORD>").next().expect("card end");
        assert!(!card.contains("GROUPING="), "{card}");
        assert!(!card.contains("SCOPE="), "{card}");
        // Component cards follow in stream order.
        assert!(xml.contains("<WORD ID=\"skami\">"), "{xml}");
        assert!(xml.contains("<WORD ID=\"mlatu\">"), "{xml}");
        assert!(
            xml.find("<WORD ID=\"skami\"").expect("skami card")
                < xml.find("<WORD ID=\"mlatu\"").expect("mlatu card"),
            "component cards must follow stream order: {xml}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zei_compound_context_roles_are_variable_context_placeholders() {
        let xml = words_xml_for("mi zei do");
        assert!(
            xml.contains("<WORD ID=\"mi-zei-do\" KNOWN=\"false\">"),
            "{xml}"
        );
        assert!(
            xml.contains(
                "<KIND>\n          <REFERENT-OF>\n            <VARIABLE-CONTEXT ROLE=\"AUDIENCE\"/>\n          </REFERENT-OF>\n        </KIND>"
            ),
            "{xml}"
        );
        assert!(
            xml.contains(
                "<MODIFIER>\n          <REFERENT-OF>\n            <VARIABLE-CONTEXT ROLE=\"SPEAKER\"/>\n          </REFERENT-OF>\n        </MODIFIER>"
            ),
            "{xml}"
        );
    }
}
