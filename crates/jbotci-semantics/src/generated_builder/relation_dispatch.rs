use super::*;

#[contract_trait]
pub(super) trait RelationDispatch {
    #[requires(true)]
    #[ensures(true)]
    fn relation_place_count(&self, dictionary: &Dictionary<'_>) -> Option<usize>;

    #[requires(true)]
    #[ensures(true)]
    fn has_open_place_structure(&self) -> bool;

    #[requires(true)]
    #[ensures(true)]
    fn asserted_predication_mode(&self) -> PredicationMode;
}

#[contract_trait]
impl RelationDispatch for RelationLabel {
    #[requires(self.is_displayable())]
    #[ensures(true)]
    fn relation_place_count(&self, dictionary: &Dictionary<'_>) -> Option<usize> {
        if self.has_open_place_structure() {
            return None;
        }
        if let Some(place_count) = constructed_relation_place_count(self) {
            return Some(place_count);
        }
        match self.as_data() {
            data!(RelationLabel::Brivla { word }) => {
                dictionary_relation_place_count(dictionary, word)
            }
            _ => None,
        }
    }

    #[requires(self.is_displayable())]
    #[ensures(true)]
    fn has_open_place_structure(&self) -> bool {
        matches!(
            self.as_data(),
            data!(RelationLabel::Identity)
                | data!(RelationLabel::Du)
                | data!(RelationLabel::NuhaOperator { .. })
        )
    }

    #[requires(self.is_displayable())]
    #[ensures(true)]
    fn asserted_predication_mode(&self) -> PredicationMode {
        if matches!(
            self.as_data(),
            data!(RelationLabel::Identity) | data!(RelationLabel::Du)
        ) {
            PredicationMode::Definitional
        } else {
            PredicationMode::Asserted
        }
    }
}

#[contract_trait]
impl RelationDispatch for str {
    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn relation_place_count(&self, dictionary: &Dictionary<'_>) -> Option<usize> {
        if self.has_open_place_structure() {
            return None;
        }
        constructed_relation_text_place_count(self)
            .or_else(|| dictionary_relation_place_count(dictionary, self))
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn has_open_place_structure(&self) -> bool {
        self == "identity"
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn asserted_predication_mode(&self) -> PredicationMode {
        if self == "identity" {
            PredicationMode::Definitional
        } else {
            PredicationMode::Asserted
        }
    }
}

#[contract_trait]
impl RelationDispatch for String {
    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn relation_place_count(&self, dictionary: &Dictionary<'_>) -> Option<usize> {
        self.as_str().relation_place_count(dictionary)
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn has_open_place_structure(&self) -> bool {
        self.as_str().has_open_place_structure()
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn asserted_predication_mode(&self) -> PredicationMode {
        self.as_str().asserted_predication_mode()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_place_count<R: RelationDispatch + ?Sized>(
    dictionary: &Dictionary<'_>,
    relation: &R,
) -> Option<usize> {
    relation.relation_place_count(dictionary)
}

#[requires(relation.is_displayable())]
#[ensures(ret.is_some() -> *ret.as_ref().unwrap() > 0)]
pub(super) fn constructed_relation_place_count(relation: &RelationLabel) -> Option<usize> {
    match relation.as_data() {
        data!(RelationLabel::Constructed { text }) => match text.as_str() {
            "referentOf" => Some(2),
            "eventOf"
            | "propertyOf"
            | "amountOf"
            | "truthValueOf"
            | "propositionOf"
            | "associatedWith"
            | "involves"
            | "memberOf"
            | "specificallyAssociatedWith"
            | "intrinsicallyPossessedBy" => Some(2),
            "conceptOf" | "experienceOf" | "abstractionOf" | "describedAs" => Some(3),
            _ => None,
        },
        data!(RelationLabel::Abstraction { kind, .. }) if *kind == AbstractionKind::Event => {
            Some(1)
        }
        data!(RelationLabel::MeksoMoi { .. }) => Some(3),
        data!(RelationLabel::Brivla { .. })
        | data!(RelationLabel::Identity)
        | data!(RelationLabel::Du)
        | data!(RelationLabel::ProBridi { .. })
        | data!(RelationLabel::Abstraction { .. })
        | data!(RelationLabel::NuhaOperator { .. })
        | data!(RelationLabel::ZeiCompound { .. })
        | data!(RelationLabel::StatementConnection { .. })
        | data!(RelationLabel::TextGroup { .. }) => None,
    }
}

#[requires(!relation.is_empty())]
#[ensures(ret.is_some() -> *ret.as_ref().unwrap() > 0)]
pub(super) fn constructed_relation_text_place_count(relation: &str) -> Option<usize> {
    match relation {
        "referentOf" => Some(2),
        "eventOf"
        | "propertyOf"
        | "amountOf"
        | "truthValueOf"
        | "propositionOf"
        | "associatedWith"
        | "involves"
        | "memberOf"
        | "specificallyAssociatedWith"
        | "intrinsicallyPossessedBy" => Some(2),
        "conceptOf" | "experienceOf" | "abstractionOf" | "describedAs" => Some(3),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_has_open_place_structure<R: RelationDispatch + ?Sized>(
    relation: &R,
) -> bool {
    relation.has_open_place_structure()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn asserted_predication_mode_for_relation<R: RelationDispatch + ?Sized>(
    relation: &R,
) -> PredicationMode {
    relation.asserted_predication_mode()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn predication_mode_for_relation<R: RelationDispatch + ?Sized>(
    relation: &R,
    mode: PredicationMode,
) -> PredicationMode {
    if mode == PredicationMode::Asserted {
        asserted_predication_mode_for_relation(relation)
    } else {
        mode
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn abstraction_kind_for_nu(abstraction: &AbstractionTanruUnitSyntax) -> AbstractionKind {
    abstraction_kind_for_cmavo(abstraction.nu.value.cmavo())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn statement_connective_from_standard(
    connective: &StandardStatementConnectiveSyntax,
) -> StatementConnectiveSyntax {
    match connective {
        StandardStatementConnectiveSyntax::JoikConnective(connective) => {
            StatementConnectiveSyntax::JoikConnective(connective.clone())
        }
        StandardStatementConnectiveSyntax::JekConnective(connective) => {
            StatementConnectiveSyntax::JekConnective(connective.clone())
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn statement_connective_from_paragraph_standard(
    connective: &ParagraphStandardStatementConnectiveSyntax,
) -> StatementConnectiveSyntax {
    match connective {
        ParagraphStandardStatementConnectiveSyntax::ParagraphJekConnective(connective) => {
            StatementConnectiveSyntax::JekConnective(JekConnectiveSyntax {
                na: connective.na.clone(),
                se: connective.se.clone(),
                ja: WithFreeModifiers::new(connective.ja.clone(), Vec::new()),
                nai: connective
                    .nai
                    .clone()
                    .map(|nai| WithFreeModifiers::new(nai, Vec::new())),
            })
        }
        ParagraphStandardStatementConnectiveSyntax::ParagraphJoiConnective(connective) => {
            StatementConnectiveSyntax::JoikConnective(JoikConnectiveSyntax::JoiConnective(
                JoiConnectiveSyntax {
                    se: connective.se.clone(),
                    joi: WithFreeModifiers::new(connective.joi.clone(), Vec::new()),
                    nai: connective
                        .nai
                        .clone()
                        .map(|nai| WithFreeModifiers::new(nai, Vec::new())),
                },
            ))
        }
        ParagraphStandardStatementConnectiveSyntax::ParagraphSimpleIntervalConnective(
            connective,
        ) => StatementConnectiveSyntax::JoikConnective(
            JoikConnectiveSyntax::SimpleIntervalConnective(SimpleIntervalConnectiveSyntax {
                se: connective.se.clone(),
                bihi: WithFreeModifiers::new(connective.bihi.clone(), Vec::new()),
                nai: connective
                    .nai
                    .clone()
                    .map(|nai| WithFreeModifiers::new(nai, Vec::new())),
            }),
        ),
        ParagraphStandardStatementConnectiveSyntax::ParagraphClosedIntervalConnective(
            connective,
        ) => StatementConnectiveSyntax::JoikConnective(
            JoikConnectiveSyntax::ClosedIntervalConnective(ClosedIntervalConnectiveSyntax {
                left_interval: connective.left_interval.clone(),
                se: connective.se.clone(),
                bihi: connective.bihi.clone(),
                nai: connective.nai.clone(),
                right_interval: WithFreeModifiers::new(
                    connective.right_interval.clone(),
                    Vec::new(),
                ),
            }),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn abstraction_kind_for_cmavo(cmavo: Option<Cmavo>) -> AbstractionKind {
    match cmavo {
        Some(Cmavo::Nu) => AbstractionKind::Event,
        Some(Cmavo::Muhe) => AbstractionKind::Achievement,
        Some(Cmavo::Puhu) => AbstractionKind::Process,
        Some(Cmavo::Zuho) => AbstractionKind::Activity,
        Some(Cmavo::Zahi) => AbstractionKind::State,
        Some(Cmavo::Ka) => AbstractionKind::Property,
        Some(Cmavo::Ni) => AbstractionKind::Amount,
        Some(Cmavo::Jei) => AbstractionKind::TruthValue,
        Some(Cmavo::Duhu) => AbstractionKind::Proposition,
        Some(Cmavo::Siho) => AbstractionKind::Concept,
        Some(Cmavo::Lihi) => AbstractionKind::Experience,
        _ => AbstractionKind::Unspecified,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn abstraction_body_mode(kind: AbstractionKind) -> PredicationMode {
    if kind == AbstractionKind::Property {
        PredicationMode::Restrictive
    } else {
        PredicationMode::Inert
    }
}

#[requires(true)]
#[ensures(ret.is_none() || matches!(kind, AbstractionKind::Event | AbstractionKind::Achievement | AbstractionKind::Process | AbstractionKind::Activity | AbstractionKind::State | AbstractionKind::Experience))]
pub(super) fn abstraction_eventuality_class(kind: AbstractionKind) -> Option<EventualityClass> {
    match kind {
        AbstractionKind::Event | AbstractionKind::Experience => Some(EventualityClass::Event),
        AbstractionKind::Achievement => Some(EventualityClass::Achievement),
        AbstractionKind::Process => Some(EventualityClass::Process),
        AbstractionKind::Activity => Some(EventualityClass::Activity),
        AbstractionKind::State => Some(EventualityClass::State),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn abstraction_output_sort(kind: AbstractionKind) -> SemanticSort {
    kind.output_sort()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn abstraction_link_relation(kind: AbstractionKind) -> &'static str {
    match kind {
        AbstractionKind::Event => "eventOf",
        AbstractionKind::Achievement => "achievementOf",
        AbstractionKind::Process => "processOf",
        AbstractionKind::Activity => "activityOf",
        AbstractionKind::State => "stateOf",
        AbstractionKind::Property => "propertyOf",
        AbstractionKind::Amount => "amountOf",
        AbstractionKind::TruthValue => "truthValueOf",
        AbstractionKind::Proposition => "propositionOf",
        AbstractionKind::Concept => "conceptOf",
        AbstractionKind::Experience => "experienceOf",
        AbstractionKind::SentenceSign | AbstractionKind::Unspecified => "abstractionOf",
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
pub(super) fn abstraction_extra_surface_place(kind: AbstractionKind) -> Option<u8> {
    match kind {
        AbstractionKind::Process
        | AbstractionKind::Activity
        | AbstractionKind::Amount
        | AbstractionKind::Concept
        | AbstractionKind::Experience
        | AbstractionKind::Unspecified => Some(2),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum DescriptionCharacterization {
    SpeakerDescribed,
    Named,
    Veridical,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn description_characterization_for_cmavo(
    cmavo: Option<Cmavo>,
) -> DescriptionCharacterization {
    match cmavo {
        Some(Cmavo::Le) => DescriptionCharacterization::SpeakerDescribed,
        Some(Cmavo::La) => DescriptionCharacterization::Named,
        _ => DescriptionCharacterization::Veridical,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn description_kind_for_cmavo(cmavo: Option<Cmavo>) -> DescriptorKind {
    match cmavo {
        Some(Cmavo::Lo) => DescriptorKind::VeridicalDescription,
        Some(Cmavo::Loi) => DescriptorKind::VeridicalMassDescription,
        Some(Cmavo::Lohi) => DescriptorKind::VeridicalSetDescription,
        Some(Cmavo::Le) => DescriptorKind::SpeakerDescription,
        Some(Cmavo::Lei) => DescriptorKind::SpeakerMassDescription,
        Some(Cmavo::Lehi) => DescriptorKind::SpeakerSetDescription,
        Some(Cmavo::Lehe) => DescriptorKind::SpeakerStereotypeDescription,
        Some(Cmavo::La) => DescriptorKind::Name,
        Some(Cmavo::Lai) => DescriptorKind::MassNameDescription,
        Some(Cmavo::Lahi) => DescriptorKind::SetNameDescription,
        Some(Cmavo::Lohe) => DescriptorKind::TypicalDescription,
        _ => DescriptorKind::Description,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn name_description_kind_for_cmavo(cmavo: Option<Cmavo>) -> DescriptorKind {
    match cmavo {
        Some(Cmavo::Lai) => DescriptorKind::MassName,
        Some(Cmavo::Lahi) => DescriptorKind::SetName,
        _ => DescriptorKind::Name,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gadri_name_sort(cmavo: Option<Cmavo>) -> SemanticSort {
    match cmavo {
        Some(Cmavo::Lai) => SemanticSort::Mass,
        Some(Cmavo::Lahi) => SemanticSort::Set,
        _ => SemanticSort::Entity,
    }
}
