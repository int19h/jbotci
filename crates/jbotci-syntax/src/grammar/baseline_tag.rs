//! Typed ownership classification for corrected camxes-exp tag-atom runs.
//!
//! The extension route runs before the standard tag alternatives so it can
//! consume a complete adjacent atom run. Completed runs that spell exactly one
//! camxes-standard tense-modal are rejected here and reparsed by the standard
//! route. Every generated product and sum used by the proof is destructured
//! exhaustively and without `..`: any later field or alternative addition must
//! force this proof to be revisited. The accepted cases below mirror the
//! standard modal and composite productions atom for atom, including their
//! optional prefix and trailing-KI structure. Therefore a rejected run has the
//! same first and last token as the standard reparse and consumes the identical
//! extent; free modifiers at an atom-internal extension-only boundary prevent
//! rejection.

use std::sync::Arc;

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    BaselineTermConnectedTenseModalContinuationSyntax, BaselineTermConnectedTenseModalSyntax,
    BaselineTermTenseModalAtomSyntax, BaselineTermTenseModalSyntax,
    ConnectedTenseModalContinuationSyntax, ConnectedTenseModalSyntax, TenseModalAtomSyntax,
    TenseModalBodySyntax, TenseModalSyntax, recovered, view,
};
use super::generated_runtime::OutputRejection;
use view::AsView;
use view::valid;

#[invariant(true)]
#[invariant(::Faha => true)]
#[invariant(::Roi => true)]
#[invariant(::Tahe => true)]
#[invariant(::Zaho => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomKind {
    Bai,
    Caha,
    Cuhe,
    Ki,
    Zi,
    Pu,
    Va,
    Faha { mohi: bool },
    Zeha,
    Veha,
    Viha,
    Roi { fehe: bool, baseline_number: bool },
    Tahe { fehe: bool },
    Zaho { fehe: bool },
    Fiho,
    Fa,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct ClassifiedAtom {
    nahe: bool,
    se: bool,
    kind: AtomKind,
}

impl From<BaselineTermTenseModalAtomSyntax> for TenseModalAtomSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: BaselineTermTenseModalAtomSyntax) -> Self {
        match value {
            BaselineTermTenseModalAtomSyntax::CompositeTense(value) => Self::CompositeTense(value),
            BaselineTermTenseModalAtomSyntax::FihoTense(value) => Self::FihoTense(value),
            BaselineTermTenseModalAtomSyntax::ModalTense(value) => Self::ModalTense(value),
            BaselineTermTenseModalAtomSyntax::StickyTense(value) => Self::StickyTense(value),
        }
    }
}

impl From<BaselineTermConnectedTenseModalContinuationSyntax>
    for ConnectedTenseModalContinuationSyntax
{
    #[requires(true)]
    #[ensures(true)]
    fn from(value: BaselineTermConnectedTenseModalContinuationSyntax) -> Self {
        let BaselineTermConnectedTenseModalContinuationSyntax {
            connective,
            tense_modal,
        } = value;
        Self {
            connective,
            tense_modal: Arc::new(Arc::unwrap_or_clone(tense_modal).into()),
        }
    }
}

impl From<BaselineTermConnectedTenseModalSyntax> for ConnectedTenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: BaselineTermConnectedTenseModalSyntax) -> Self {
        let BaselineTermConnectedTenseModalSyntax {
            first,
            continuations,
        } = value;
        let continuations =
            continuations.mapped(|value| Arc::new(Arc::unwrap_or_clone(value).into()));
        Self {
            first: Arc::new(Arc::unwrap_or_clone(first).into()),
            continuations,
        }
    }
}

impl From<BaselineTermTenseModalSyntax> for TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: BaselineTermTenseModalSyntax) -> Self {
        Self(Arc::new(match value {
            BaselineTermTenseModalSyntax::BaselineTermConnectedTenseModal(value) => {
                TenseModalBodySyntax::ConnectedTenseModal(Arc::new(
                    Arc::unwrap_or_clone(value).into(),
                ))
            }
            BaselineTermTenseModalSyntax::BaselineTermTenseModalAtom(value) => {
                TenseModalBodySyntax::TenseModalAtom(Arc::new(Arc::unwrap_or_clone(value).into()))
            }
        }))
    }
}

#[requires(true)]
#[ensures(true)]
fn map_recovered<T, U>(
    value: recovered::Recovered<T>,
    convert: impl FnOnce(T) -> U,
) -> recovered::Recovered<U> {
    match value {
        recovered::Recovered::Valid(value) => recovered::Recovered::valid(convert(*value)),
        recovered::Recovered::Error(error) => recovered::Recovered::error(error),
        recovered::Recovered::Prefix(prefix) => recovered::Recovered::prefix_boxed(
            prefix.errors.into_vec(),
            Box::new(convert(*prefix.value)),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_atom_into_tense_modal_atom(
    value: recovered::BaselineTermTenseModalAtomSyntax,
) -> recovered::TenseModalAtomSyntax {
    match value {
        recovered::BaselineTermTenseModalAtomSyntax::CompositeTense(value) => {
            recovered::TenseModalAtomSyntax::CompositeTense(value)
        }
        recovered::BaselineTermTenseModalAtomSyntax::FihoTense(value) => {
            recovered::TenseModalAtomSyntax::FihoTense(value)
        }
        recovered::BaselineTermTenseModalAtomSyntax::ModalTense(value) => {
            recovered::TenseModalAtomSyntax::ModalTense(value)
        }
        recovered::BaselineTermTenseModalAtomSyntax::StickyTense(value) => {
            recovered::TenseModalAtomSyntax::StickyTense(value)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_continuation_into_continuation(
    value: recovered::BaselineTermConnectedTenseModalContinuationSyntax,
) -> recovered::ConnectedTenseModalContinuationSyntax {
    let recovered::BaselineTermConnectedTenseModalContinuationSyntax {
        connective,
        tense_modal,
    } = value;
    recovered::ConnectedTenseModalContinuationSyntax {
        connective,
        tense_modal: Arc::new(map_recovered(
            Arc::unwrap_or_clone(tense_modal),
            recovered_baseline_atom_into_tense_modal_atom,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_connected_into_connected(
    value: recovered::BaselineTermConnectedTenseModalSyntax,
) -> recovered::ConnectedTenseModalSyntax {
    let recovered::BaselineTermConnectedTenseModalSyntax {
        first,
        continuations,
    } = value;
    let continuations = continuations.mapped(|value| {
        Arc::new(map_recovered(
            Arc::unwrap_or_clone(value),
            recovered_baseline_continuation_into_continuation,
        ))
    });
    recovered::ConnectedTenseModalSyntax {
        first: Arc::new(map_recovered(
            Arc::unwrap_or_clone(first),
            recovered_baseline_atom_into_tense_modal_atom,
        )),
        continuations,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_body_into_body(
    value: recovered::BaselineTermTenseModalSyntax,
) -> recovered::TenseModalBodySyntax {
    match value {
        recovered::BaselineTermTenseModalSyntax::BaselineTermConnectedTenseModal(value) => {
            recovered::TenseModalBodySyntax::ConnectedTenseModal(Arc::new(map_recovered(
                Arc::unwrap_or_clone(value),
                recovered_baseline_connected_into_connected,
            )))
        }
        recovered::BaselineTermTenseModalSyntax::BaselineTermTenseModalAtom(value) => {
            recovered::TenseModalBodySyntax::TenseModalAtom(Arc::new(map_recovered(
                Arc::unwrap_or_clone(value),
                recovered_baseline_atom_into_tense_modal_atom,
            )))
        }
    }
}

impl From<recovered::Recovered<recovered::BaselineTermTenseModalSyntax>>
    for recovered::TenseModalSyntax
{
    #[requires(true)]
    #[ensures(true)]
    fn from(value: recovered::Recovered<recovered::BaselineTermTenseModalSyntax>) -> Self {
        Self(Arc::new(map_recovered(
            value,
            recovered_baseline_body_into_body,
        )))
    }
}

#[requires(true)]
#[ensures(true)]
fn is_time_property(kind: AtomKind) -> bool {
    matches!(
        kind,
        AtomKind::Roi {
            fehe: false,
            baseline_number: true
        } | AtomKind::Tahe { fehe: false }
            | AtomKind::Zaho { fehe: false }
    )
}

#[requires(true)]
#[ensures(true)]
fn is_space_property(kind: AtomKind) -> bool {
    matches!(
        kind,
        AtomKind::Roi {
            fehe: true,
            baseline_number: true
        } | AtomKind::Tahe { fehe: true }
            | AtomKind::Zaho { fehe: true }
    )
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineTagRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct PostNaExtensionTagRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct NonElidedNaheFihoTagTermRejection;

#[requires(true)]
#[ensures(true)]
fn valid_wf<'tree, V: view::Inspect, S, R>(
    value: &view::WithFreeModifiersView<
        view::Slot<V>,
        jbotci_tree::view::Sequence<'tree, S, R, view::Slot<view::FreeModifierSyntaxRef<'tree>>>,
    >,
) -> bool {
    let view::WithFreeModifiersView {
        value,
        free_modifiers,
    } = *value;
    valid(value).is_some()
        && free_modifiers
            .iter()
            .all(|modifier| modifier.valid().is_some())
}

#[requires(true)]
#[ensures(true)]
fn exp_number_is_baseline(number: view::ExpNumberSyntaxView<'_>) -> bool {
    let view::ExpNumberSyntaxView { first, additional } = number;
    valid(first).is_some_and(exp_number_atom_is_pa)
        && additional
            .iter()
            .all(|atom| valid(atom).is_some_and(exp_number_atom_is_pa))
}

#[requires(true)]
#[ensures(true)]
fn exp_number_atom_is_pa(atom: view::ExpNumberAtomSyntaxView<'_>) -> bool {
    match atom {
        view::ExpNumberAtomSyntaxView::ExpPaNumberAtom(atom) => {
            valid(atom).is_some_and(|view::ExpPaNumberAtomSyntaxView(pa)| valid(pa).is_some())
        }
        view::ExpNumberAtomSyntaxView::ExpNiheNumberAtom(atom) => valid(atom).is_some_and(
            |view::ExpNiheNumberAtomSyntaxView {
                 nihe: _,
                 selbri: _,
                 tehu: _,
             }| false,
        ),
        view::ExpNumberAtomSyntaxView::ExpMoheNumberAtom(atom) => valid(atom).is_some_and(
            |view::ExpMoheNumberAtomSyntaxView {
                 mohe: _,
                 sumti: _,
                 tehu: _,
             }| false,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_atom(atom: view::ExpPrefixedTagAtomSyntaxView<'_>) -> Option<ClassifiedAtom> {
    let view::ExpPrefixedTagAtomSyntaxView { nahe, se, atom } = atom;
    let nahe = match nahe {
        Some(nahe) if valid_wf(&nahe) => true,
        Some(_) => return None,
        None => false,
    };
    let se = match se {
        Some(se) if valid_wf(&se) => true,
        Some(_) => return None,
        None => false,
    };
    if !atom
        .free_modifiers
        .iter()
        .all(|modifier| valid(modifier).is_some())
    {
        return None;
    }
    let kind = match valid(&atom.value)? {
        view::ExpTagAtomSyntaxView::ExpBaiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpBaiTagAtomSyntaxView(bai)| {
                valid(bai).is_some().then_some(AtomKind::Bai)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpCahaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpCahaTagAtomSyntaxView(caha)| {
                valid(caha).is_some().then_some(AtomKind::Caha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpCuheTagAtom(atom) => {
            valid(atom).and_then(|view::ExpCuheTagAtomSyntaxView(cuhe)| {
                valid(cuhe).is_some().then_some(AtomKind::Cuhe)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpKiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpKiTagAtomSyntaxView(ki)| {
                valid(ki).is_some().then_some(AtomKind::Ki)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZiTagAtomSyntaxView(zi)| {
                valid(zi).is_some().then_some(AtomKind::Zi)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpPuTagAtom(atom) => {
            valid(atom).and_then(|view::ExpPuTagAtomSyntaxView(pu)| {
                valid(pu).is_some().then_some(AtomKind::Pu)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVaTagAtomSyntaxView(va)| {
                valid(va).is_some().then_some(AtomKind::Va)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFahaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFahaTagAtomSyntaxView { mohi, faha }| {
                (mohi.as_ref().is_none_or(|mohi| valid(mohi).is_some()) && valid(faha).is_some())
                    .then_some(AtomKind::Faha {
                        mohi: mohi.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZehaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZehaTagAtomSyntaxView(zeha)| {
                valid(zeha).is_some().then_some(AtomKind::Zeha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVehaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVehaTagAtomSyntaxView(veha)| {
                valid(veha).is_some().then_some(AtomKind::Veha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVihaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVihaTagAtomSyntaxView(viha)| {
                valid(viha).is_some().then_some(AtomKind::Viha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpRoiTagAtom(atom) => valid(atom).and_then(
            |view::ExpRoiTagAtomSyntaxView {
                 fehe,
                 interval,
                 roi,
             }| {
                let interval = valid(interval)?;
                let baseline_number = match interval {
                    view::ExpRoiIntervalSyntaxView::ExpParenthesizedRoiInterval(interval) => {
                        let view::ExpParenthesizedRoiIntervalSyntaxView {
                            vei,
                            expression,
                            veho,
                        } = valid(interval)?;
                        if !valid_wf(&vei)
                            || valid(expression).is_none()
                            || !veho.as_ref().is_none_or(valid_wf)
                        {
                            return None;
                        }
                        false
                    }
                    view::ExpRoiIntervalSyntaxView::ExpNumber(number) => {
                        valid(number).is_some_and(exp_number_is_baseline)
                    }
                };
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(roi).is_some())
                    .then_some(AtomKind::Roi {
                        fehe: fehe.is_some(),
                        baseline_number,
                    })
            },
        )?,
        view::ExpTagAtomSyntaxView::ExpTaheTagAtom(atom) => {
            valid(atom).and_then(|view::ExpTaheTagAtomSyntaxView { fehe, tahe }| {
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(tahe).is_some())
                    .then_some(AtomKind::Tahe {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZahoTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZahoTagAtomSyntaxView { fehe, zaho }| {
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(zaho).is_some())
                    .then_some(AtomKind::Zaho {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFihoTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFihoTagAtomSyntaxView { fiho, selbri, fehu }| {
                (valid_wf(&fiho) && valid(selbri).is_some() && fehu.as_ref().is_none_or(valid_wf))
                    .then_some(AtomKind::Fiho)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFaTagAtomSyntaxView(fa)| {
                valid(fa).is_some().then_some(AtomKind::Fa)
            })?
        }
    };
    Some(ClassifiedAtom { nahe, se, kind })
}

#[requires(index <= run.additional.len())]
#[ensures(true)]
fn classified(run: view::ExpTagAtomRunBodySyntaxView<'_>, index: usize) -> Option<ClassifiedAtom> {
    let view::ExpTagAtomRunBodySyntaxView { first, additional } = run;
    if index == 0 {
        valid(first).and_then(classify_atom)
    } else {
        valid(additional.get(index - 1).expect("index is within the run")).and_then(classify_atom)
    }
}

#[requires(start <= end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn all_unprefixed(run: view::ExpTagAtomRunBodySyntaxView<'_>, start: usize, end: usize) -> bool {
    (start..end).all(|index| classified(run, index).is_some_and(|atom| !atom.nahe && !atom.se))
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn time_slice(run: view::ExpTagAtomRunBodySyntaxView<'_>, start: usize, end: usize) -> bool {
    let Some(first) = classified(run, start) else {
        return false;
    };
    let mut index = start;
    let leading_zi = first.kind == AtomKind::Zi;
    if leading_zi {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Pu) {
        offsets += 1;
        index += 1;
        if index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Zi) {
            index += 1;
        }
    }
    let interval =
        index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Zeha);
    if interval {
        index += 1;
        if index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Pu) {
            index += 1;
        }
    }
    let property_start = index;
    while index < end && classified(run, index).is_some_and(|atom| is_time_property(atom.kind)) {
        index += 1;
    }
    index == end && (leading_zi || offsets > 0 || interval || property_start < end)
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn space_slice(run: view::ExpTagAtomRunBodySyntaxView<'_>, start: usize, end: usize) -> bool {
    let Some(first) = classified(run, start) else {
        return false;
    };
    let mut index = start;
    let leading_va = first.kind == AtomKind::Va;
    if leading_va {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end
        && classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: false }))
    {
        offsets += 1;
        index += 1;
        if index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Va) {
            index += 1;
        }
    }
    let interval_start = index;
    if index < end
        && classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Veha | AtomKind::Viha))
    {
        let was_veha = classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Veha);
        index += 1;
        if was_veha
            && index < end
            && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Viha)
        {
            index += 1;
        }
        if index < end
            && classified(run, index)
                .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: false }))
        {
            index += 1;
        }
        while index < end && classified(run, index).is_some_and(|atom| is_space_property(atom.kind))
        {
            index += 1;
        }
    } else {
        while index < end && classified(run, index).is_some_and(|atom| is_space_property(atom.kind))
        {
            index += 1;
        }
    }
    let has_interval = interval_start < index;
    let mohi_offset = index < end
        && classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: true }));
    if mohi_offset {
        index += 1;
        if index < end && classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Va) {
            index += 1;
        }
    }
    index == end && (leading_va || offsets > 0 || has_interval || mohi_offset)
}

#[requires(true)]
#[ensures(true)]
fn composite_is_baseline(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
    start: usize,
    end: usize,
) -> bool {
    if start >= end {
        return false;
    }
    if end == start + 1 && classified(run, start).is_some_and(|atom| atom.kind == AtomKind::Caha) {
        return true;
    }
    let without_caha = if classified(run, end - 1).is_some_and(|atom| atom.kind == AtomKind::Caha) {
        end - 1
    } else {
        end
    };
    if without_caha == start {
        return false;
    }
    (start + 1..=without_caha).any(|time_end| {
        time_slice(run, start, time_end)
            && (time_end == without_caha
                || time_end < without_caha && space_slice(run, time_end, without_caha))
    }) || (start + 1..=without_caha).any(|space_end| {
        space_slice(run, start, space_end)
            && (space_end == without_caha
                || space_end < without_caha && time_slice(run, space_end, without_caha))
    })
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_tag(run: view::ExpTagAtomRunBodySyntaxView<'_>) -> bool {
    let len = run.additional.len() + 1;
    let Some(first) = classified(run, 0) else {
        return false;
    };
    if first.kind == AtomKind::Fiho {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.kind == AtomKind::Bai {
        return len == 1
            || len == 2
                && classified(run, 1).is_some_and(|atom| atom.kind == AtomKind::Ki)
                && all_unprefixed(run, 1, len);
    }
    if matches!(first.kind, AtomKind::Cuhe | AtomKind::Ki) {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.se {
        return false;
    }
    let composite_end =
        if len > 1 && classified(run, len - 1).is_some_and(|atom| atom.kind == AtomKind::Ki) {
            len - 1
        } else {
            len
        };
    composite_is_baseline(run, 0, composite_end) && all_unprefixed(run, 1, len)
}

#[requires(true)]
#[ensures(true)]
fn exp_run_starts_with_baseline_term(run: view::ExpTagAtomRunBodySyntaxView<'_>) -> bool {
    let Some(first) = classified(run, 0) else {
        return false;
    };
    if first.kind == AtomKind::Fiho {
        return !first.nahe && !first.se;
    }
    if first.kind == AtomKind::Bai {
        return true;
    }
    if matches!(first.kind, AtomKind::Cuhe | AtomKind::Ki) {
        return !first.nahe && !first.se;
    }
    !first.se && composite_is_baseline(run, 0, 1)
}

#[requires(true)]
#[ensures(true)]
fn atom_is_extension(atom: view::TenseModalAtomSyntaxView<'_>) -> bool {
    match atom {
        view::TenseModalAtomSyntaxView::ExpTagAtomRun(_) => true,
        view::TenseModalAtomSyntaxView::CompositeTense(_)
        | view::TenseModalAtomSyntaxView::FihoTense(_)
        | view::TenseModalAtomSyntaxView::ModalTense(_)
        | view::TenseModalAtomSyntaxView::StickyTense(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_is_extension(output: view::TenseModalSyntaxView<'_>) -> bool {
    let view::TenseModalSyntaxView(body) = output;
    let Some(body) = valid(body) else {
        return false;
    };
    match body {
        view::TenseModalBodySyntaxView::ConnectedTenseModal(connected) => valid(connected)
            .is_some_and(|connected| {
                let view::ConnectedTenseModalSyntaxView {
                    first,
                    continuations,
                } = connected;
                let starts_with_baseline_term = valid(first).is_some_and(|first| match first {
                    view::TenseModalAtomSyntaxView::ExpTagAtomRun(run) => valid(run)
                        .and_then(|view::ExpTagAtomRunSyntaxView(body)| valid(body))
                        .is_some_and(exp_run_starts_with_baseline_term),
                    view::TenseModalAtomSyntaxView::CompositeTense(_)
                    | view::TenseModalAtomSyntaxView::FihoTense(_)
                    | view::TenseModalAtomSyntaxView::ModalTense(_)
                    | view::TenseModalAtomSyntaxView::StickyTense(_) => true,
                });
                starts_with_baseline_term
                    && (valid(first).is_some_and(atom_is_extension)
                        || continuations.iter().any(|continuation| {
                            valid(continuation).is_some_and(|continuation| {
                                valid({
                                    let view::ConnectedTenseModalContinuationSyntaxView {
                                        connective: _,
                                        tense_modal,
                                    } = continuation;
                                    tense_modal
                                })
                                .is_some_and(atom_is_extension)
                            })
                        }))
            }),
        view::TenseModalBodySyntaxView::TenseModalAtom(atom) => {
            valid(atom).is_some_and(|atom| match atom {
                view::TenseModalAtomSyntaxView::ExpTagAtomRun(run) => valid(run)
                    .and_then(|view::ExpTagAtomRunSyntaxView(body)| valid(body))
                    .is_some_and(exp_run_starts_with_baseline_term),
                view::TenseModalAtomSyntaxView::CompositeTense(_)
                | view::TenseModalAtomSyntaxView::FihoTense(_)
                | view::TenseModalAtomSyntaxView::ModalTense(_)
                | view::TenseModalAtomSyntaxView::StickyTense(_) => false,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_elided_nahe_fiho_tense_modal(tense_modal: view::Slot<view::TenseModalSyntaxRef<'_>>) -> bool {
    let Some(view::TenseModalSyntaxView(body)) = valid(tense_modal) else {
        return false;
    };
    let atom = match valid(body) {
        Some(view::TenseModalBodySyntaxView::TenseModalAtom(atom)) => atom,
        Some(view::TenseModalBodySyntaxView::ConnectedTenseModal(_)) | None => return false,
    };
    let run = match valid(atom) {
        Some(view::TenseModalAtomSyntaxView::ExpTagAtomRun(run)) => run,
        Some(
            view::TenseModalAtomSyntaxView::CompositeTense(_)
            | view::TenseModalAtomSyntaxView::FihoTense(_)
            | view::TenseModalAtomSyntaxView::ModalTense(_)
            | view::TenseModalAtomSyntaxView::StickyTense(_),
        )
        | None => return false,
    };
    let Some(run) = valid(run).and_then(|view::ExpTagAtomRunSyntaxView(body)| valid(body)) else {
        return false;
    };
    let view::ExpTagAtomRunBodySyntaxView { first, additional } = run;
    let Some(view::ExpPrefixedTagAtomSyntaxView { nahe, se, atom }) = valid(first) else {
        return false;
    };
    let fiho = match valid(atom.value) {
        Some(view::ExpTagAtomSyntaxView::ExpFihoTagAtom(fiho)) => fiho,
        Some(
            view::ExpTagAtomSyntaxView::ExpBaiTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpCahaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpCuheTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpKiTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpZiTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpPuTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpVaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpFahaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpZehaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpVehaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpVihaTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpRoiTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpTaheTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpZahoTagAtom(_)
            | view::ExpTagAtomSyntaxView::ExpFaTagAtom(_),
        )
        | None => return false,
    };
    let Some(view::ExpFihoTagAtomSyntaxView {
        fiho,
        selbri: _,
        fehu,
    }) = valid(fiho)
    else {
        return false;
    };
    nahe.as_ref()
        .is_some_and(|nahe| valid(&nahe.value).is_some() && nahe.free_modifiers.is_empty())
        && se.is_none()
        && valid(&atom.value).is_some()
        && atom.free_modifiers.is_empty()
        && additional.is_empty()
        && valid(&fiho.value).is_some()
        && fiho.free_modifiers.is_empty()
        && fehu.is_none()
}

#[contract_trait]
impl<T> OutputRejection<T> for BaselineTagRejection
where
    for<'tree> T: AsView<'tree, Ref = view::ExpTagAtomRunBodySyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "baseline tag surface"
    }
    fn rejects(&self, value: &T) -> bool {
        valid(value.as_view()).is_some_and(is_baseline_tag)
    }
}

#[contract_trait]
impl<T> OutputRejection<T> for NonElidedNaheFihoTagTermRejection
where
    for<'tree> T: AsView<'tree, Ref = view::TenseModalSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "non-elided-FEhU or non-NAhE/FIhO final tag term"
    }
    fn rejects(&self, value: &T) -> bool {
        !is_elided_nahe_fiho_tense_modal(value.as_view())
    }
}

#[contract_trait]
impl<T> OutputRejection<T> for PostNaExtensionTagRejection
where
    for<'tree> T: AsView<'tree, Ref = view::SelbriSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "extension tag immediately after selbri negation"
    }
    fn rejects(&self, value: &T) -> bool {
        valid(value.as_view()).is_some_and(|output| match output {
            view::SelbriSyntaxView::TaggedSelbri(tagged) => valid(tagged).is_some_and(|tagged| {
                let view::TaggedSelbriSyntaxView {
                    tense_modal,
                    inner_selbri: _,
                } = tagged;
                valid(tense_modal).is_some_and(tense_modal_is_extension)
            }),
            view::SelbriSyntaxView::UntaggedSelbri(_) => false,
        })
    }
}
