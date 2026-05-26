use crate::Token;
use bityzba::{data, invariant, new, requires};
use jbotci_morphology::{Cmavo, Selmaho};

use super::ast::{
    CompositeTenseModalPartSyntax, CompositeTenseModalPartSyntaxData, FihoModalSyntax,
    FreeModifierSyntax, IntervalTenseSyntax, SimpleTenseModalSyntax, SpaceTenseSyntax,
    TenseModalSyntax, TenseModalSyntaxData, TimeTenseSyntax, WithFreeModifiers,
};
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
fn parts_from_leaves(leaves: Vec<Token>) -> Vec<CompositeTenseModalPartSyntax> {
    leaves
        .into_iter()
        .map(|leaf| new!(CompositeTenseModalPartSyntax::Word(leaf)))
        .collect()
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(TenseModalSyntax::Composite { .. })))]
#[ensures(composite_leaf_count(&ret) == old(leaves.len()))]
pub(super) fn tense_modal_from_leaves(
    leaves: Vec<Token>,
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
                vec![new!(CompositeTenseModalPartSyntax::Fiho(Box::new(new!(
                    FihoModalSyntax {
                        nahe: None,
                        fiho,
                        relation,
                        fehu,
                    }
                ))))],
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
pub(super) fn connective_tense_modal_from_leaves(leaves: Vec<Token>) -> TenseModalSyntax {
    new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(parts_from_leaves(leaves), Vec::new()),
    })
}

impl TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    pub fn composite_time(&self) -> Option<TimeTenseSyntax> {
        let classification = classify_composite(self)?;
        (!classification.time_direction.is_empty()
            || classification.time_distance.is_some()
            || classification.time_interval.is_some())
        .then_some(new!(TimeTenseSyntax {
            direction: classification.time_direction,
            distance: classification.time_distance,
            interval: classification.time_interval,
            nai: classification.time_nai,
        }))
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
        .then_some(new!(SpaceTenseSyntax {
            direction: classification.space_direction,
            distance: classification.space_distance,
            interval: classification.space_interval,
            dimensions: classification.space_dimensions,
            mohi: classification.space_mohi,
            fehe: classification.space_fehe,
        }))
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
    pub fn composite_zaho(&self) -> Vec<Token> {
        classify_composite(self).map_or_else(Vec::new, |classification| classification.zaho)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_caha(&self) -> Option<Token> {
        classify_composite(self)?.caha
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_ki(&self) -> Option<Token> {
        classify_composite(self)?.ki
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_cuhe(&self) -> Option<Token> {
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
                    data!(CompositeTenseModalPartSyntax::Fiho(fiho)) => Some((**fiho).clone()),
                    data!(CompositeTenseModalPartSyntax::Word(_)) => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn composite_connectives(&self) -> Vec<Token> {
        classify_composite(self).map_or_else(Vec::new, |classification| classification.connectives)
    }
}

#[derive(Default)]
#[invariant(true)]
struct CompositeTenseModalClassification {
    time_direction: Vec<Token>,
    time_distance: Option<Token>,
    time_interval: Option<Token>,
    time_nai: Option<Token>,
    space_direction: Vec<Token>,
    space_distance: Vec<Token>,
    space_interval: Vec<Token>,
    space_dimensions: Vec<Token>,
    space_mohi: Option<Token>,
    space_fehe: Option<Token>,
    simple: Option<SimpleTenseModalSyntax>,
    interval: Option<IntervalTenseSyntax>,
    zaho: Vec<Token>,
    caha: Option<Token>,
    ki: Option<Token>,
    cuhe: Option<Token>,
    connectives: Vec<Token>,
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
fn classify_composite_leaf(leaf: &Token, classification: &mut CompositeTenseModalClassification) {
    if leaf.is_cmavo(Cmavo::Ki) {
        classification.ki = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Cuhe) {
        classification.cuhe = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Caha) {
        classification.caha = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Zaho) {
        classification.zaho.push(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Pu) {
        classification.time_direction.push(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Zi) {
        classification.time_distance = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Zeha) {
        classification.time_interval = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Faha) {
        classification.space_direction.push(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Va) {
        classification.space_distance.push(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Veha) {
        classification.space_interval.push(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Viha) {
        classification.space_dimensions.push(leaf.clone());
    } else if leaf.is_cmavo(Cmavo::Mohi) {
        classification.space_mohi = Some(leaf.clone());
    } else if leaf.is_cmavo(Cmavo::Fehe) {
        classification.space_fehe = Some(leaf.clone());
    } else if leaf.is_selmaho(Selmaho::Bai) {
        classification.simple = Some(set_simple_bai(classification.simple.take(), leaf.clone()));
    } else if leaf.is_selmaho(Selmaho::Nahe) {
        classification.simple = Some(set_simple_nahe(classification.simple.take(), leaf.clone()));
    } else if leaf.is_selmaho(Selmaho::Se) {
        classification.simple = Some(set_simple_se(classification.simple.take(), leaf.clone()));
    } else if leaf.is_cmavo(Cmavo::Nai) {
        if let Some(existing_simple) = classification.simple.take() {
            let mut simple_data = existing_simple.into_data();
            simple_data.nai = Some(leaf.clone());
            classification.simple = Some(SimpleTenseModalSyntax::from_data(simple_data));
        } else if classification.interval.is_some() {
            let existing_interval = classification
                .interval
                .take()
                .expect("interval was checked as present");
            let mut interval_data = existing_interval.into_data();
            interval_data.nai = Some(leaf.clone());
            classification.interval = Some(IntervalTenseSyntax::from_data(interval_data));
        } else {
            classification.time_nai = Some(leaf.clone());
        }
    } else if leaf.is_selmaho(Selmaho::Roi) || leaf.is_selmaho(Selmaho::Tahe) {
        classification.interval = Some(new!(IntervalTenseSyntax {
            number: None,
            roi_or_tahe: leaf.clone(),
            nai: None,
        }));
    } else if leaf.is_selmaho(Selmaho::Ja)
        || leaf.is_selmaho(Selmaho::Joi)
        || leaf.is_selmaho(Selmaho::Bihi)
        || leaf.is_selmaho(Selmaho::Gaho)
        || leaf.is_selmaho(Selmaho::Fa)
    {
        classification.connectives.push(leaf.clone());
    }
}

#[requires(true)]
#[ensures(ret.bai.is_some())]
fn set_simple_bai(simple: Option<SimpleTenseModalSyntax>, bai: Token) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => {
            let mut data = existing.into_data();
            data.bai = Some(bai);
            SimpleTenseModalSyntax::from_data(data)
        }
        None => new!(SimpleTenseModalSyntax {
            nahe: None,
            se: None,
            bai: Some(bai),
            nai: None,
        }),
    }
}

#[requires(true)]
#[ensures(ret.nahe.is_some())]
fn set_simple_nahe(simple: Option<SimpleTenseModalSyntax>, nahe: Token) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => {
            let mut data = existing.into_data();
            data.nahe = Some(nahe);
            SimpleTenseModalSyntax::from_data(data)
        }
        None => new!(SimpleTenseModalSyntax {
            nahe: Some(nahe),
            se: None,
            bai: None,
            nai: None,
        }),
    }
}

#[requires(true)]
#[ensures(ret.se.is_some())]
fn set_simple_se(simple: Option<SimpleTenseModalSyntax>, se: Token) -> SimpleTenseModalSyntax {
    match simple {
        Some(existing) => {
            let mut data = existing.into_data();
            data.se = Some(se);
            SimpleTenseModalSyntax::from_data(data)
        }
        None => new!(SimpleTenseModalSyntax {
            nahe: None,
            se: Some(se),
            bai: None,
            nai: None,
        }),
    }
}
