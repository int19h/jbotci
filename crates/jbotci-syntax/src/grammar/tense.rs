use crate::WithIndicators;
use bityzba::{data, invariant, new, requires};
use jbotci_morphology::WordLike;

use super::ast::{
    CompositeTenseModalPartSyntax, CompositeTenseModalPartSyntaxData, FihoModalSyntax,
    FreeModifierSyntax, IntervalTenseSyntax, SimpleTenseModalSyntax, SpaceTenseSyntax,
    TenseModalSyntax, TenseModalSyntaxData, TimeTenseSyntax, WithFreeModifiers,
};
use super::tokens::{BAI_WORDS, CAHA_WORDS, FA_WORDS, ROI_WORDS, ZAHO_WORDS, cmavo_text_matches};

#[requires(true)]
#[ensures(true)]
fn composite_leaf_count(tense_modal: &TenseModalSyntax) -> usize {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts
            .value
            .iter()
            .filter(|part| {
                matches!(
                    part.as_data(),
                    data!(CompositeTenseModalPartSyntax::Word(_))
                )
            })
            .count(),
        _ => 0,
    }
}

#[requires(true)]
#[ensures(ret.len() == old(leaves.len()))]
fn parts_from_leaves(leaves: Vec<WithIndicators<WordLike>>) -> Vec<CompositeTenseModalPartSyntax> {
    leaves
        .into_iter()
        .map(|leaf| new!(CompositeTenseModalPartSyntax::Word(leaf)))
        .collect()
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(TenseModalSyntax::Composite { .. })))]
#[ensures(composite_leaf_count(&ret) == old(leaves.len()))]
pub(super) fn tense_modal_from_leaves(
    leaves: Vec<WithIndicators<WordLike>>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
    new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(parts_from_leaves(leaves), free_modifiers),
    })
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(TenseModalSyntax::Composite { .. })))]
pub(super) fn tense_modal_as_composite(tense_modal: TenseModalSyntax) -> TenseModalSyntax {
    match tense_modal.into_data() {
        data!(TenseModalSyntax::Composite { parts }) => new!(TenseModalSyntax::Composite { parts }),
        data!(TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
        }) => new!(TenseModalSyntax::Composite {
            parts: WithFreeModifiers::new(
                vec![new!(CompositeTenseModalPartSyntax::Fiho(FihoModalSyntax {
                    nahe: None,
                    fiho,
                    relation: *relation,
                    fehu,
                }))],
                Vec::new(),
            ),
        }),
        other => {
            let other = TenseModalSyntax::from_data(other);
            let (leaves, free_modifiers) = other.leaf_words_and_free_modifiers();
            tense_modal_from_leaves(leaves, free_modifiers)
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(TenseModalSyntax::Composite { .. })))]
#[ensures(composite_leaf_count(&ret) == old(leaves.len()))]
pub(super) fn connective_tense_modal_from_leaves(
    leaves: Vec<WithIndicators<WordLike>>,
) -> TenseModalSyntax {
    new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(parts_from_leaves(leaves), Vec::new()),
    })
}

#[requires(!texts.is_empty())]
#[ensures(ret == texts.iter().any(|text| cmavo_text_matches(word, text)))]
fn cmavo_matches_any(word: &WithIndicators<WordLike>, texts: &[&str]) -> bool {
    texts.iter().any(|text| cmavo_text_matches(word, text))
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn composite_time(&self) -> Option<TimeTenseSyntax> {
        let classification = classify_composite(self)?;
        (!classification.time_direction.is_empty()
            || classification.time_distance.is_some()
            || classification.time_interval.is_some())
        .then_some(TimeTenseSyntax {
            direction: classification.time_direction,
            distance: classification.time_distance,
            interval: classification.time_interval,
            nai: classification.time_nai,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_space(&self) -> Option<SpaceTenseSyntax> {
        let classification = classify_composite(self)?;
        (!classification.space_direction.is_empty()
            || !classification.space_distance.is_empty()
            || !classification.space_interval.is_empty()
            || !classification.space_dimensions.is_empty()
            || classification.space_mohi.is_some()
            || classification.space_fehe.is_some())
        .then_some(SpaceTenseSyntax {
            direction: classification.space_direction,
            distance: classification.space_distance,
            interval: classification.space_interval,
            dimensions: classification.space_dimensions,
            mohi: classification.space_mohi,
            fehe: classification.space_fehe,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_simple(&self) -> Option<SimpleTenseModalSyntax> {
        classify_composite(self)?.simple
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_interval(&self) -> Option<IntervalTenseSyntax> {
        classify_composite(self)?.interval
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_zaho(&self) -> Vec<WithIndicators<WordLike>> {
        classify_composite(self).map_or_else(Vec::new, |classification| classification.zaho)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_caha(&self) -> Option<WithIndicators<WordLike>> {
        classify_composite(self)?.caha
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_ki(&self) -> Option<WithIndicators<WordLike>> {
        classify_composite(self)?.ki
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_cuhe(&self) -> Option<WithIndicators<WordLike>> {
        classify_composite(self)?.cuhe
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_fiho(&self) -> Vec<FihoModalSyntax> {
        match self.as_data() {
            data!(TenseModalSyntax::Composite { parts }) => parts
                .value
                .iter()
                .filter_map(|part| match part.as_data() {
                    data!(CompositeTenseModalPartSyntax::Fiho(fiho)) => Some(fiho.clone()),
                    data!(CompositeTenseModalPartSyntax::Word(_)) => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_connectives(&self) -> Vec<WithIndicators<WordLike>> {
        classify_composite(self).map_or_else(Vec::new, |classification| classification.connectives)
    }
}

#[derive(Default)]
#[invariant(true)]
struct CompositeTenseModalClassification {
    time_direction: Vec<WithIndicators<WordLike>>,
    time_distance: Option<WithIndicators<WordLike>>,
    time_interval: Option<WithIndicators<WordLike>>,
    time_nai: Option<WithIndicators<WordLike>>,
    space_direction: Vec<WithIndicators<WordLike>>,
    space_distance: Vec<WithIndicators<WordLike>>,
    space_interval: Vec<WithIndicators<WordLike>>,
    space_dimensions: Vec<WithIndicators<WordLike>>,
    space_mohi: Option<WithIndicators<WordLike>>,
    space_fehe: Option<WithIndicators<WordLike>>,
    simple: Option<SimpleTenseModalSyntax>,
    interval: Option<IntervalTenseSyntax>,
    zaho: Vec<WithIndicators<WordLike>>,
    caha: Option<WithIndicators<WordLike>>,
    ki: Option<WithIndicators<WordLike>>,
    cuhe: Option<WithIndicators<WordLike>>,
    connectives: Vec<WithIndicators<WordLike>>,
}

#[requires(true)]
#[ensures(true)]
fn classify_composite(tense_modal: &TenseModalSyntax) -> Option<CompositeTenseModalClassification> {
    let data!(TenseModalSyntax::Composite { parts }) = tense_modal.as_data() else {
        return None;
    };
    let mut classification = CompositeTenseModalClassification::default();
    for part in &parts.value {
        let data!(CompositeTenseModalPartSyntax::Word(leaf)) = part.as_data() else {
            continue;
        };
        classify_composite_leaf(leaf, &mut classification);
    }
    Some(classification)
}

#[requires(true)]
#[ensures(true)]
fn classify_composite_leaf(
    leaf: &WithIndicators<WordLike>,
    classification: &mut CompositeTenseModalClassification,
) {
    if cmavo_text_matches(leaf, "ki") {
        classification.ki = Some(leaf.clone());
    } else if cmavo_matches_any(leaf, &["cu'e", "nau"]) {
        classification.cuhe = Some(leaf.clone());
    } else if cmavo_matches_any(leaf, CAHA_WORDS) {
        classification.caha = Some(leaf.clone());
    } else if cmavo_matches_any(leaf, ZAHO_WORDS) {
        classification.zaho.push(leaf.clone());
    } else if cmavo_matches_any(leaf, &["pu", "ca", "ba"]) {
        classification.time_direction.push(leaf.clone());
    } else if cmavo_matches_any(leaf, &["zi", "za", "zu"]) {
        classification.time_distance = Some(leaf.clone());
    } else if cmavo_matches_any(leaf, &["ze'i", "ze'a", "ze'u", "ze'e"]) {
        classification.time_interval = Some(leaf.clone());
    } else if cmavo_matches_any(
        leaf,
        &[
            "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a", "ru'u",
            "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a", "zo'i", "ze'o",
        ],
    ) {
        classification.space_direction.push(leaf.clone());
    } else if cmavo_matches_any(leaf, &["vi", "va", "vu"]) {
        classification.space_distance.push(leaf.clone());
    } else if cmavo_matches_any(leaf, &["ve'i", "ve'a", "ve'u", "ve'e"]) {
        classification.space_interval.push(leaf.clone());
    } else if cmavo_matches_any(leaf, &["vi'i", "vi'a", "vi'u", "vi'e"]) {
        classification.space_dimensions.push(leaf.clone());
    } else if cmavo_text_matches(leaf, "mo'i") {
        classification.space_mohi = Some(leaf.clone());
    } else if cmavo_text_matches(leaf, "fe'e") {
        classification.space_fehe = Some(leaf.clone());
    } else if cmavo_matches_any(leaf, BAI_WORDS) {
        classification.simple = Some(set_simple_bai(classification.simple.take(), leaf.clone()));
    } else if cmavo_matches_any(leaf, &["na'e", "to'e", "no'e", "je'a"]) {
        classification.simple = Some(set_simple_nahe(classification.simple.take(), leaf.clone()));
    } else if cmavo_matches_any(leaf, &["se", "te", "ve", "xe"]) {
        classification.simple = Some(set_simple_se(classification.simple.take(), leaf.clone()));
    } else if cmavo_text_matches(leaf, "nai") {
        if let Some(existing_simple) = classification.simple.take() {
            classification.simple = Some(SimpleTenseModalSyntax {
                nai: Some(leaf.clone()),
                ..existing_simple
            });
        } else if classification.interval.is_some() {
            let existing_interval = classification
                .interval
                .take()
                .expect("interval was checked as present");
            classification.interval = Some(IntervalTenseSyntax {
                nai: Some(leaf.clone()),
                ..existing_interval
            });
        } else {
            classification.time_nai = Some(leaf.clone());
        }
    } else if cmavo_matches_any(leaf, ROI_WORDS)
        || cmavo_matches_any(leaf, &["di'i", "na'o", "ru'i", "ta'e"])
    {
        classification.interval = Some(IntervalTenseSyntax {
            number: None,
            roi_or_tahe: leaf.clone(),
            nai: None,
        });
    } else if cmavo_matches_any(leaf, &["je'i", "ja", "je", "jo", "ju"])
        || cmavo_matches_any(
            leaf,
            &[
                "ce", "ce'o", "jo'u", "jo'e", "fa'u", "ku'a", "pi'u", "joi", "bi'i", "bi'o", "mi'i",
            ],
        )
        || cmavo_matches_any(leaf, &["ga'o", "ke'i"])
        || cmavo_matches_any(leaf, FA_WORDS)
    {
        classification.connectives.push(leaf.clone());
    }
}

#[requires(true)]
#[ensures(ret.bai.is_some())]
fn set_simple_bai(
    simple: Option<SimpleTenseModalSyntax>,
    bai: WithIndicators<WordLike>,
) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => SimpleTenseModalSyntax {
            bai: Some(bai),
            ..existing
        },
        None => SimpleTenseModalSyntax {
            nahe: None,
            se: None,
            bai: Some(bai),
            nai: None,
        },
    }
}

#[requires(true)]
#[ensures(ret.nahe.is_some())]
fn set_simple_nahe(
    simple: Option<SimpleTenseModalSyntax>,
    nahe: WithIndicators<WordLike>,
) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => SimpleTenseModalSyntax {
            nahe: Some(nahe),
            ..existing
        },
        None => SimpleTenseModalSyntax {
            nahe: Some(nahe),
            se: None,
            bai: None,
            nai: None,
        },
    }
}

#[requires(true)]
#[ensures(ret.se.is_some())]
fn set_simple_se(
    simple: Option<SimpleTenseModalSyntax>,
    se: WithIndicators<WordLike>,
) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => SimpleTenseModalSyntax {
            se: Some(se),
            ..existing
        },
        None => SimpleTenseModalSyntax {
            nahe: None,
            se: Some(se),
            bai: None,
            nai: None,
        },
    }
}
