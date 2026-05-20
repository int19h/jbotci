use crate::WithIndicators;
use bityzba::requires;
use jbotci_morphology::WordLike;

use super::ast::{
    FihoModalSyntax, FreeModifierSyntax, IntervalTenseSyntax, SimpleTenseModalSyntax,
    SpaceTenseSyntax, TenseModalSyntax, TimeTenseSyntax, WithFreeModifiers,
};
use super::tokens::{BAI_WORDS, CAHA_WORDS, FA_WORDS, ROI_WORDS, ZAHO_WORDS, cmavo_text_matches};

#[requires(true)]
#[ensures(matches!(ret, TenseModalSyntax::Composite { .. }))]
#[ensures(ret.clone().leaf_words() == old(leaves.clone()))]
pub(super) fn tense_modal_from_leaves(
    leaves: Vec<WithIndicators<WordLike>>,
    free_modifiers: Vec<FreeModifierSyntax>,
) -> TenseModalSyntax {
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
    let mut interval = None;
    let mut zaho = Vec::new();
    let mut caha = None;
    let mut ki = None;
    let mut cuhe = None;
    let mut connectives = Vec::new();

    for leaf in &leaves {
        if cmavo_text_matches(leaf, "ki") {
            ki = Some(leaf.clone());
        } else if cmavo_matches_any(leaf, &["cu'e", "nau"]) {
            cuhe = Some(leaf.clone());
        } else if cmavo_matches_any(leaf, CAHA_WORDS) {
            caha = Some(leaf.clone());
        } else if cmavo_matches_any(leaf, ZAHO_WORDS) {
            zaho.push(leaf.clone());
        } else if cmavo_matches_any(leaf, &["pu", "ca", "ba"]) {
            time_direction.push(leaf.clone());
        } else if cmavo_matches_any(leaf, &["zi", "za", "zu"]) {
            time_distance = Some(leaf.clone());
        } else if cmavo_matches_any(leaf, &["ze'i", "ze'a", "ze'u", "ze'e"]) {
            time_interval = Some(leaf.clone());
        } else if cmavo_matches_any(
            leaf,
            &[
                "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a",
                "ru'u", "re'o", "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a",
                "zo'i", "ze'o",
            ],
        ) {
            space_direction.push(leaf.clone());
        } else if cmavo_matches_any(leaf, &["vi", "va", "vu"]) {
            space_distance.push(leaf.clone());
        } else if cmavo_matches_any(leaf, &["ve'i", "ve'a", "ve'u", "ve'e"]) {
            space_interval.push(leaf.clone());
        } else if cmavo_matches_any(leaf, &["vi'i", "vi'a", "vi'u", "vi'e"]) {
            space_dimensions.push(leaf.clone());
        } else if cmavo_text_matches(leaf, "mo'i") {
            space_mohi = Some(leaf.clone());
        } else if cmavo_text_matches(leaf, "fe'e") {
            space_fehe = Some(leaf.clone());
        } else if cmavo_matches_any(leaf, BAI_WORDS) {
            simple = Some(set_simple_bai(simple, leaf.clone()));
        } else if cmavo_matches_any(leaf, &["na'e", "to'e", "no'e", "je'a"]) {
            simple = Some(set_simple_nahe(simple, leaf.clone()));
        } else if cmavo_matches_any(leaf, &["se", "te", "ve", "xe"]) {
            simple = Some(set_simple_se(simple, leaf.clone()));
        } else if cmavo_text_matches(leaf, "nai") {
            if let Some(existing_simple) = simple.take() {
                simple = Some(SimpleTenseModalSyntax {
                    nai: Some(leaf.clone()),
                    ..existing_simple
                });
            } else if interval.is_some() {
                let existing_interval = interval.take().expect("interval was checked as present");
                interval = Some(IntervalTenseSyntax {
                    nai: Some(leaf.clone()),
                    ..existing_interval
                });
            } else {
                time_nai = Some(leaf.clone());
            }
        } else if cmavo_matches_any(leaf, ROI_WORDS)
            || cmavo_matches_any(leaf, &["di'i", "na'o", "ru'i", "ta'e"])
        {
            interval = Some(IntervalTenseSyntax {
                number: Vec::new(),
                roi_or_tahe: leaf.clone(),
                nai: None,
            });
        } else if cmavo_matches_any(leaf, &["je'i", "ja", "je", "jo", "ju"])
            || cmavo_matches_any(
                leaf,
                &[
                    "ce", "ce'o", "jo'u", "jo'e", "fa'u", "ku'a", "pi'u", "joi", "bi'i", "bi'o",
                    "mi'i",
                ],
            )
            || cmavo_matches_any(leaf, &["ga'o", "ke'i"])
            || cmavo_matches_any(leaf, FA_WORDS)
        {
            connectives.push(leaf.clone());
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
        leaves: WithFreeModifiers::new(leaves, free_modifiers),
        time,
        space,
        simple,
        interval,
        zaho,
        caha,
        ki,
        cuhe,
        fiho: Vec::new(),
        connectives,
    }
}

#[requires(true)]
#[ensures(matches!(ret, TenseModalSyntax::Composite { .. }))]
pub(super) fn tense_modal_as_composite(tense_modal: TenseModalSyntax) -> TenseModalSyntax {
    match tense_modal {
        composite @ TenseModalSyntax::Composite { .. } => composite,
        TenseModalSyntax::Fiho {
            fiho,
            relation,
            fehu,
        } => TenseModalSyntax::Composite {
            leaves: WithFreeModifiers::new(Vec::new(), Vec::new()),
            time: None,
            space: None,
            simple: None,
            interval: None,
            zaho: Vec::new(),
            caha: None,
            ki: None,
            cuhe: None,
            fiho: vec![FihoModalSyntax {
                nahe: None,
                fiho,
                relation: *relation,
                fehu,
            }],
            connectives: Vec::new(),
        },
        other => tense_modal_from_leaves(other.clone().leaf_words(), other.free_modifiers()),
    }
}

#[requires(true)]
#[ensures(matches!(ret, TenseModalSyntax::Composite { .. }))]
#[ensures(ret.clone().leaf_words() == old(leaves.clone()))]
pub(super) fn connective_tense_modal_from_leaves(
    leaves: Vec<WithIndicators<WordLike>>,
) -> TenseModalSyntax {
    let connectives = leaves
        .iter()
        .filter(|leaf| {
            cmavo_matches_any(leaf, &["je'i", "ja", "je", "jo", "ju"])
                || cmavo_matches_any(
                    leaf,
                    &[
                        "ce", "ce'e", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a",
                        "pi'u", "bi'i", "bi'o", "mi'i",
                    ],
                )
                || cmavo_matches_any(leaf, &["ga'o", "ke'i"])
                || cmavo_matches_any(leaf, FA_WORDS)
        })
        .cloned()
        .collect();
    TenseModalSyntax::Composite {
        leaves: WithFreeModifiers::new(leaves, Vec::new()),
        time: None,
        space: None,
        simple: None,
        interval: None,
        zaho: Vec::new(),
        caha: None,
        ki: None,
        cuhe: None,
        fiho: Vec::new(),
        connectives,
    }
}

#[requires(!texts.is_empty())]
#[ensures(ret == texts.iter().any(|text| cmavo_text_matches(word, text)))]
fn cmavo_matches_any(word: &WithIndicators<WordLike>, texts: &[&str]) -> bool {
    texts.iter().any(|text| cmavo_text_matches(word, text))
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
