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
    ConnectedTenseModalContinuationSyntax, ConnectedTenseModalSyntax, ExpBaiTagAtomSyntax,
    ExpCahaTagAtomSyntax, ExpCuheTagAtomSyntax, ExpFaTagAtomSyntax, ExpFahaTagAtomSyntax,
    ExpFihoTagAtomSyntax, ExpKiTagAtomSyntax, ExpMoheNumberAtomSyntax, ExpNiheNumberAtomSyntax,
    ExpNumberAtomSyntax, ExpNumberSyntax, ExpPaNumberAtomSyntax, ExpParenthesizedRoiIntervalSyntax,
    ExpPrefixedTagAtomSyntax, ExpPuTagAtomSyntax, ExpRoiIntervalSyntax, ExpRoiTagAtomSyntax,
    ExpTagAtomRunBodySyntax, ExpTagAtomSyntax, ExpTaheTagAtomSyntax, ExpVaTagAtomSyntax,
    ExpVehaTagAtomSyntax, ExpVihaTagAtomSyntax, ExpZahoTagAtomSyntax, ExpZehaTagAtomSyntax,
    ExpZiTagAtomSyntax, TenseModalAtomSyntax, TenseModalBodySyntax, TenseModalSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

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
        let continuations = continuations
            .into_vec()
            .into_iter()
            .map(Into::into)
            .collect();
        Self {
            first: Arc::new(Arc::unwrap_or_clone(first).into()),
            continuations: vec1::Vec1::try_from_vec(continuations)
                .expect("a mapped non-empty continuation sequence remains non-empty"),
        }
    }
}

impl From<BaselineTermTenseModalSyntax> for TenseModalSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: BaselineTermTenseModalSyntax) -> Self {
        Self(match value {
            BaselineTermTenseModalSyntax::BaselineTermConnectedTenseModal(value) => {
                TenseModalBodySyntax::ConnectedTenseModal(value.into())
            }
            BaselineTermTenseModalSyntax::BaselineTermTenseModalAtom(value) => {
                TenseModalBodySyntax::TenseModalAtom(value.into())
            }
        })
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
    let continuations = continuations
        .into_vec()
        .into_iter()
        .map(|value| map_recovered(value, recovered_baseline_continuation_into_continuation))
        .collect();
    recovered::ConnectedTenseModalSyntax {
        first: Arc::new(map_recovered(
            Arc::unwrap_or_clone(first),
            recovered_baseline_atom_into_tense_modal_atom,
        )),
        continuations: vec1::Vec1::try_from_vec(continuations)
            .expect("a mapped non-empty recovered continuation sequence remains non-empty"),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_body_into_body(
    value: recovered::BaselineTermTenseModalSyntax,
) -> recovered::TenseModalBodySyntax {
    match value {
        recovered::BaselineTermTenseModalSyntax::BaselineTermConnectedTenseModal(value) => {
            recovered::TenseModalBodySyntax::ConnectedTenseModal(map_recovered(
                value,
                recovered_baseline_connected_into_connected,
            ))
        }
        recovered::BaselineTermTenseModalSyntax::BaselineTermTenseModalAtom(value) => {
            recovered::TenseModalBodySyntax::TenseModalAtom(map_recovered(
                value,
                recovered_baseline_atom_into_tense_modal_atom,
            ))
        }
    }
}

impl From<recovered::Recovered<recovered::BaselineTermTenseModalSyntax>>
    for recovered::TenseModalSyntax
{
    #[requires(true)]
    #[ensures(true)]
    fn from(value: recovered::Recovered<recovered::BaselineTermTenseModalSyntax>) -> Self {
        Self(map_recovered(value, recovered_baseline_body_into_body))
    }
}

#[requires(index <= run.additional.len())]
#[ensures(true)]
fn run_atom(run: &ExpTagAtomRunBodySyntax, index: usize) -> &ExpPrefixedTagAtomSyntax {
    let ExpTagAtomRunBodySyntax { first, additional } = run;
    if index == 0 {
        first.as_ref()
    } else {
        additional[index - 1].as_ref()
    }
}

#[requires(true)]
#[ensures(true)]
fn exp_number_is_baseline(number: &ExpNumberSyntax) -> bool {
    let ExpNumberSyntax { first, additional } = number;
    exp_number_atom_is_pa(first.as_ref())
        && additional
            .iter()
            .all(|atom| exp_number_atom_is_pa(atom.as_ref()))
}

#[requires(true)]
#[ensures(true)]
fn exp_number_atom_is_pa(atom: &ExpNumberAtomSyntax) -> bool {
    match atom {
        ExpNumberAtomSyntax::ExpPaNumberAtom(atom) => {
            let ExpPaNumberAtomSyntax(_pa) = atom;
            true
        }
        ExpNumberAtomSyntax::ExpNiheNumberAtom(atom) => {
            let ExpNiheNumberAtomSyntax {
                nihe: _,
                selbri: _,
                tehu: _,
            } = atom;
            false
        }
        ExpNumberAtomSyntax::ExpMoheNumberAtom(atom) => {
            let ExpMoheNumberAtomSyntax {
                mohe: _,
                sumti: _,
                tehu: _,
            } = atom;
            false
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn classify_atom(atom: &ExpPrefixedTagAtomSyntax) -> ClassifiedAtom {
    let ExpPrefixedTagAtomSyntax { nahe, se, atom } = atom;
    let kind = match atom.value.as_ref() {
        ExpTagAtomSyntax::ExpBaiTagAtom(atom) => {
            let ExpBaiTagAtomSyntax(_bai) = atom;
            AtomKind::Bai
        }
        ExpTagAtomSyntax::ExpCahaTagAtom(atom) => {
            let ExpCahaTagAtomSyntax(_caha) = atom;
            AtomKind::Caha
        }
        ExpTagAtomSyntax::ExpCuheTagAtom(atom) => {
            let ExpCuheTagAtomSyntax(_cuhe) = atom;
            AtomKind::Cuhe
        }
        ExpTagAtomSyntax::ExpKiTagAtom(atom) => {
            let ExpKiTagAtomSyntax(_ki) = atom;
            AtomKind::Ki
        }
        ExpTagAtomSyntax::ExpZiTagAtom(atom) => {
            let ExpZiTagAtomSyntax(_zi) = atom;
            AtomKind::Zi
        }
        ExpTagAtomSyntax::ExpPuTagAtom(atom) => {
            let ExpPuTagAtomSyntax(_pu) = atom;
            AtomKind::Pu
        }
        ExpTagAtomSyntax::ExpVaTagAtom(atom) => {
            let ExpVaTagAtomSyntax(_va) = atom;
            AtomKind::Va
        }
        ExpTagAtomSyntax::ExpFahaTagAtom(atom) => {
            let ExpFahaTagAtomSyntax { mohi, faha: _ } = atom;
            AtomKind::Faha {
                mohi: mohi.is_some(),
            }
        }
        ExpTagAtomSyntax::ExpZehaTagAtom(atom) => {
            let ExpZehaTagAtomSyntax(_zeha) = atom;
            AtomKind::Zeha
        }
        ExpTagAtomSyntax::ExpVehaTagAtom(atom) => {
            let ExpVehaTagAtomSyntax(_veha) = atom;
            AtomKind::Veha
        }
        ExpTagAtomSyntax::ExpVihaTagAtom(atom) => {
            let ExpVihaTagAtomSyntax(_viha) = atom;
            AtomKind::Viha
        }
        ExpTagAtomSyntax::ExpRoiTagAtom(atom) => {
            let ExpRoiTagAtomSyntax {
                fehe,
                interval,
                roi: _,
            } = atom;
            let baseline_number = match interval {
                ExpRoiIntervalSyntax::ExpParenthesizedRoiInterval(interval) => {
                    let ExpParenthesizedRoiIntervalSyntax {
                        vei: _,
                        expression: _,
                        veho: _,
                    } = interval;
                    false
                }
                ExpRoiIntervalSyntax::ExpNumber(number) => exp_number_is_baseline(number),
            };
            AtomKind::Roi {
                fehe: fehe.is_some(),
                baseline_number,
            }
        }
        ExpTagAtomSyntax::ExpTaheTagAtom(atom) => {
            let ExpTaheTagAtomSyntax { fehe, tahe: _ } = atom;
            AtomKind::Tahe {
                fehe: fehe.is_some(),
            }
        }
        ExpTagAtomSyntax::ExpZahoTagAtom(atom) => {
            let ExpZahoTagAtomSyntax { fehe, zaho: _ } = atom;
            AtomKind::Zaho {
                fehe: fehe.is_some(),
            }
        }
        ExpTagAtomSyntax::ExpFihoTagAtom(atom) => {
            let ExpFihoTagAtomSyntax {
                fiho: _,
                selbri: _,
                fehu: _,
            } = atom;
            AtomKind::Fiho
        }
        ExpTagAtomSyntax::ExpFaTagAtom(atom) => {
            let ExpFaTagAtomSyntax(_fa) = atom;
            AtomKind::Fa
        }
    };
    ClassifiedAtom {
        nahe: nahe.is_some(),
        se: se.is_some(),
        kind,
    }
}

#[requires(index <= run.additional.len())]
#[ensures(true)]
fn classified(run: &ExpTagAtomRunBodySyntax, index: usize) -> ClassifiedAtom {
    classify_atom(run_atom(run, index))
}

#[requires(start <= end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn all_unprefixed(run: &ExpTagAtomRunBodySyntax, start: usize, end: usize) -> bool {
    (start..end).all(|index| {
        let atom = classified(run, index);
        !atom.nahe && !atom.se
    })
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

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn time_slice(run: &ExpTagAtomRunBodySyntax, start: usize, end: usize) -> bool {
    let mut index = start;
    let leading_zi = classified(run, index).kind == AtomKind::Zi;
    if leading_zi {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end && classified(run, index).kind == AtomKind::Pu {
        offsets += 1;
        index += 1;
        if index < end && classified(run, index).kind == AtomKind::Zi {
            index += 1;
        }
    }
    let interval = index < end && classified(run, index).kind == AtomKind::Zeha;
    if interval {
        index += 1;
        if index < end && classified(run, index).kind == AtomKind::Pu {
            index += 1;
        }
    }
    let property_start = index;
    while index < end && is_time_property(classified(run, index).kind) {
        index += 1;
    }
    index == end && (leading_zi || offsets > 0 || interval || property_start < end)
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn space_slice(run: &ExpTagAtomRunBodySyntax, start: usize, end: usize) -> bool {
    let mut index = start;
    let leading_va = classified(run, index).kind == AtomKind::Va;
    if leading_va {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end && matches!(classified(run, index).kind, AtomKind::Faha { mohi: false }) {
        offsets += 1;
        index += 1;
        if index < end && classified(run, index).kind == AtomKind::Va {
            index += 1;
        }
    }
    let interval_start = index;
    if index < end && matches!(classified(run, index).kind, AtomKind::Veha | AtomKind::Viha) {
        let was_veha = classified(run, index).kind == AtomKind::Veha;
        index += 1;
        if was_veha && index < end && classified(run, index).kind == AtomKind::Viha {
            index += 1;
        }
        if index < end && matches!(classified(run, index).kind, AtomKind::Faha { mohi: false }) {
            index += 1;
        }
        while index < end && is_space_property(classified(run, index).kind) {
            index += 1;
        }
    } else {
        while index < end && is_space_property(classified(run, index).kind) {
            index += 1;
        }
    }
    let has_interval = interval_start < index;
    let mohi_offset =
        index < end && matches!(classified(run, index).kind, AtomKind::Faha { mohi: true });
    if mohi_offset {
        index += 1;
        if index < end && classified(run, index).kind == AtomKind::Va {
            index += 1;
        }
    }
    index == end && (leading_va || offsets > 0 || has_interval || mohi_offset)
}

#[requires(true)]
#[ensures(true)]
fn composite_is_baseline(run: &ExpTagAtomRunBodySyntax, start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    if end == start + 1 && classified(run, start).kind == AtomKind::Caha {
        return true;
    }
    let without_caha = if classified(run, end - 1).kind == AtomKind::Caha {
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
fn is_baseline_tag(run: &ExpTagAtomRunBodySyntax) -> bool {
    let len = run.additional.len() + 1;
    let first = classified(run, 0);
    if first.kind == AtomKind::Fiho {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.kind == AtomKind::Bai {
        return len == 1
            || len == 2 && classified(run, 1).kind == AtomKind::Ki && all_unprefixed(run, 1, len);
    }
    if matches!(first.kind, AtomKind::Cuhe | AtomKind::Ki) {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.se {
        return false;
    }
    let composite_end = if len > 1 && classified(run, len - 1).kind == AtomKind::Ki {
        len - 1
    } else {
        len
    };
    composite_is_baseline(run, 0, composite_end) && all_unprefixed(run, 1, len)
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineTagRejection;

/// Rejects only the whole rolling-Zantufa tag arm at source positions that are
/// camxes `stag` consumers but have no rolling `tag` counterpart. This match is
/// deliberately exhaustive and contains no catch-all arm: extending the shared
/// body enum must force every context guard to be audited again.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ZantufaTagRejection;

#[contract_trait]
impl OutputRejection<TenseModalSyntax> for ZantufaTagRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa tag at a camxes-only stag position"
    }

    fn rejects(&self, output: &TenseModalSyntax) -> bool {
        let TenseModalSyntax(body) = output;
        match body {
            TenseModalBodySyntax::ConnectedTenseModal(_) => false,
            TenseModalBodySyntax::TenseModalAtom(_) => false,
            TenseModalBodySyntax::ZantufaTag(_) => true,
        }
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::TenseModalSyntax>> for ZantufaTagRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa tag at a camxes-only stag position"
    }

    fn rejects(&self, output: &recovered::Recovered<recovered::TenseModalSyntax>) -> bool {
        let Some(output) = valid(output) else {
            return false;
        };
        let recovered::TenseModalSyntax(body) = output;
        let Some(body) = valid(body) else {
            return false;
        };
        match body {
            recovered::TenseModalBodySyntax::ConnectedTenseModal(_) => false,
            recovered::TenseModalBodySyntax::TenseModalAtom(_) => false,
            recovered::TenseModalBodySyntax::ZantufaTag(_) => true,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn valid<T>(value: &recovered::Recovered<T>) -> Option<&T> {
    match value {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn valid_wf<T>(value: &recovered::WithFreeModifiers<recovered::Recovered<T>>) -> bool {
    valid(&value.value).is_some()
        && value
            .free_modifiers
            .iter()
            .all(|modifier| valid(modifier).is_some())
}

#[requires(true)]
#[ensures(true)]
fn recovered_exp_number_is_baseline(number: &recovered::ExpNumberSyntax) -> bool {
    let recovered::ExpNumberSyntax { first, additional } = number;
    valid(first).is_some_and(recovered_exp_number_atom_is_pa)
        && additional
            .iter()
            .all(|atom| valid(atom).is_some_and(recovered_exp_number_atom_is_pa))
}

#[requires(true)]
#[ensures(true)]
fn recovered_exp_number_atom_is_pa(atom: &recovered::ExpNumberAtomSyntax) -> bool {
    match atom {
        recovered::ExpNumberAtomSyntax::ExpPaNumberAtom(atom) => {
            valid(atom).is_some_and(|recovered::ExpPaNumberAtomSyntax(pa)| valid(pa).is_some())
        }
        recovered::ExpNumberAtomSyntax::ExpNiheNumberAtom(atom) => valid(atom).is_some_and(
            |recovered::ExpNiheNumberAtomSyntax { nihe, selbri, tehu }| {
                let _ =
                    valid_wf(nihe) && valid(selbri).is_some() && tehu.as_ref().is_none_or(valid_wf);
                false
            },
        ),
        recovered::ExpNumberAtomSyntax::ExpMoheNumberAtom(atom) => {
            valid(atom).is_some_and(|recovered::ExpMoheNumberAtomSyntax { mohe, sumti, tehu }| {
                let _ =
                    valid_wf(mohe) && valid(sumti).is_some() && tehu.as_ref().is_none_or(valid_wf);
                false
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_classify_atom(atom: &recovered::ExpPrefixedTagAtomSyntax) -> Option<ClassifiedAtom> {
    let recovered::ExpPrefixedTagAtomSyntax { nahe, se, atom } = atom;
    let nahe = match nahe {
        Some(nahe) if valid(nahe).is_some() => true,
        Some(_) => return None,
        None => false,
    };
    let se = match se {
        Some(se) if valid(se).is_some() => true,
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
        recovered::ExpTagAtomSyntax::ExpBaiTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpBaiTagAtomSyntax(bai)| {
                valid(bai).is_some().then_some(AtomKind::Bai)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpCahaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpCahaTagAtomSyntax(caha)| {
                valid(caha).is_some().then_some(AtomKind::Caha)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpCuheTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpCuheTagAtomSyntax(cuhe)| {
                valid(cuhe).is_some().then_some(AtomKind::Cuhe)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpKiTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpKiTagAtomSyntax(ki)| {
                valid(ki).is_some().then_some(AtomKind::Ki)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpZiTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpZiTagAtomSyntax(zi)| {
                valid(zi).is_some().then_some(AtomKind::Zi)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpPuTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpPuTagAtomSyntax(pu)| {
                valid(pu).is_some().then_some(AtomKind::Pu)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpVaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpVaTagAtomSyntax(va)| {
                valid(va).is_some().then_some(AtomKind::Va)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpFahaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpFahaTagAtomSyntax { mohi, faha }| {
                (mohi.as_ref().is_none_or(|mohi| valid(mohi).is_some()) && valid(faha).is_some())
                    .then_some(AtomKind::Faha {
                        mohi: mohi.is_some(),
                    })
            })?
        }
        recovered::ExpTagAtomSyntax::ExpZehaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpZehaTagAtomSyntax(zeha)| {
                valid(zeha).is_some().then_some(AtomKind::Zeha)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpVehaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpVehaTagAtomSyntax(veha)| {
                valid(veha).is_some().then_some(AtomKind::Veha)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpVihaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpVihaTagAtomSyntax(viha)| {
                valid(viha).is_some().then_some(AtomKind::Viha)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpRoiTagAtom(atom) => valid(atom).and_then(
            |recovered::ExpRoiTagAtomSyntax {
                 fehe,
                 interval,
                 roi,
             }| {
                let interval = valid(interval)?;
                let baseline_number = match interval {
                    recovered::ExpRoiIntervalSyntax::ExpParenthesizedRoiInterval(interval) => {
                        let recovered::ExpParenthesizedRoiIntervalSyntax {
                            vei,
                            expression,
                            veho,
                        } = valid(interval)?;
                        if !valid_wf(vei)
                            || valid(expression).is_none()
                            || !veho.as_ref().is_none_or(valid_wf)
                        {
                            return None;
                        }
                        false
                    }
                    recovered::ExpRoiIntervalSyntax::ExpNumber(number) => {
                        valid(number).is_some_and(recovered_exp_number_is_baseline)
                    }
                };
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(roi).is_some())
                    .then_some(AtomKind::Roi {
                        fehe: fehe.is_some(),
                        baseline_number,
                    })
            },
        )?,
        recovered::ExpTagAtomSyntax::ExpTaheTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpTaheTagAtomSyntax { fehe, tahe }| {
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(tahe).is_some())
                    .then_some(AtomKind::Tahe {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        recovered::ExpTagAtomSyntax::ExpZahoTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpZahoTagAtomSyntax { fehe, zaho }| {
                (fehe.as_ref().is_none_or(|fehe| valid(fehe).is_some()) && valid(zaho).is_some())
                    .then_some(AtomKind::Zaho {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        recovered::ExpTagAtomSyntax::ExpFihoTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpFihoTagAtomSyntax { fiho, selbri, fehu }| {
                (valid_wf(fiho) && valid(selbri).is_some() && fehu.as_ref().is_none_or(valid_wf))
                    .then_some(AtomKind::Fiho)
            })?
        }
        recovered::ExpTagAtomSyntax::ExpFaTagAtom(atom) => {
            valid(atom).and_then(|recovered::ExpFaTagAtomSyntax(fa)| {
                valid(fa).is_some().then_some(AtomKind::Fa)
            })?
        }
    };
    Some(ClassifiedAtom { nahe, se, kind })
}

#[requires(index <= run.additional.len())]
#[ensures(true)]
fn recovered_classified(
    run: &recovered::ExpTagAtomRunBodySyntax,
    index: usize,
) -> Option<ClassifiedAtom> {
    let recovered::ExpTagAtomRunBodySyntax { first, additional } = run;
    if index == 0 {
        valid(first).and_then(recovered_classify_atom)
    } else {
        valid(&additional[index - 1]).and_then(recovered_classify_atom)
    }
}

#[requires(start <= end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn recovered_all_unprefixed(
    run: &recovered::ExpTagAtomRunBodySyntax,
    start: usize,
    end: usize,
) -> bool {
    (start..end)
        .all(|index| recovered_classified(run, index).is_some_and(|atom| !atom.nahe && !atom.se))
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn recovered_time_slice(
    run: &recovered::ExpTagAtomRunBodySyntax,
    start: usize,
    end: usize,
) -> bool {
    let Some(first) = recovered_classified(run, start) else {
        return false;
    };
    let mut index = start;
    let leading_zi = first.kind == AtomKind::Zi;
    if leading_zi {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end
        && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Pu)
    {
        offsets += 1;
        index += 1;
        if index < end
            && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Zi)
        {
            index += 1;
        }
    }
    let interval = index < end
        && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Zeha);
    if interval {
        index += 1;
        if index < end
            && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Pu)
        {
            index += 1;
        }
    }
    let property_start = index;
    while index < end
        && recovered_classified(run, index).is_some_and(|atom| is_time_property(atom.kind))
    {
        index += 1;
    }
    index == end && (leading_zi || offsets > 0 || interval || property_start < end)
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn recovered_space_slice(
    run: &recovered::ExpTagAtomRunBodySyntax,
    start: usize,
    end: usize,
) -> bool {
    let Some(first) = recovered_classified(run, start) else {
        return false;
    };
    let mut index = start;
    let leading_va = first.kind == AtomKind::Va;
    if leading_va {
        index += 1;
    }
    let mut offsets = 0usize;
    while index < end
        && recovered_classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: false }))
    {
        offsets += 1;
        index += 1;
        if index < end
            && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Va)
        {
            index += 1;
        }
    }
    let interval_start = index;
    if index < end
        && recovered_classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Veha | AtomKind::Viha))
    {
        let was_veha =
            recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Veha);
        index += 1;
        if was_veha
            && index < end
            && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Viha)
        {
            index += 1;
        }
        if index < end
            && recovered_classified(run, index)
                .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: false }))
        {
            index += 1;
        }
        while index < end
            && recovered_classified(run, index).is_some_and(|atom| is_space_property(atom.kind))
        {
            index += 1;
        }
    } else {
        while index < end
            && recovered_classified(run, index).is_some_and(|atom| is_space_property(atom.kind))
        {
            index += 1;
        }
    }
    let has_interval = interval_start < index;
    let mohi_offset = index < end
        && recovered_classified(run, index)
            .is_some_and(|atom| matches!(atom.kind, AtomKind::Faha { mohi: true }));
    if mohi_offset {
        index += 1;
        if index < end
            && recovered_classified(run, index).is_some_and(|atom| atom.kind == AtomKind::Va)
        {
            index += 1;
        }
    }
    index == end && (leading_va || offsets > 0 || has_interval || mohi_offset)
}

#[requires(true)]
#[ensures(true)]
fn recovered_composite_is_baseline(
    run: &recovered::ExpTagAtomRunBodySyntax,
    start: usize,
    end: usize,
) -> bool {
    if start >= end {
        return false;
    }
    if end == start + 1
        && recovered_classified(run, start).is_some_and(|atom| atom.kind == AtomKind::Caha)
    {
        return true;
    }
    let without_caha =
        if recovered_classified(run, end - 1).is_some_and(|atom| atom.kind == AtomKind::Caha) {
            end - 1
        } else {
            end
        };
    if without_caha == start {
        return false;
    }
    (start + 1..=without_caha).any(|time_end| {
        recovered_time_slice(run, start, time_end)
            && (time_end == without_caha
                || time_end < without_caha && recovered_space_slice(run, time_end, without_caha))
    }) || (start + 1..=without_caha).any(|space_end| {
        recovered_space_slice(run, start, space_end)
            && (space_end == without_caha
                || space_end < without_caha && recovered_time_slice(run, space_end, without_caha))
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_tag(run: &recovered::ExpTagAtomRunBodySyntax) -> bool {
    let len = run.additional.len() + 1;
    let Some(first) = recovered_classified(run, 0) else {
        return false;
    };
    if first.kind == AtomKind::Fiho {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.kind == AtomKind::Bai {
        return len == 1
            || len == 2
                && recovered_classified(run, 1).is_some_and(|atom| atom.kind == AtomKind::Ki)
                && recovered_all_unprefixed(run, 1, len);
    }
    if matches!(first.kind, AtomKind::Cuhe | AtomKind::Ki) {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.se {
        return false;
    }
    let composite_end = if len > 1
        && recovered_classified(run, len - 1).is_some_and(|atom| atom.kind == AtomKind::Ki)
    {
        len - 1
    } else {
        len
    };
    recovered_composite_is_baseline(run, 0, composite_end) && recovered_all_unprefixed(run, 1, len)
}

#[contract_trait]
impl OutputRejection<ExpTagAtomRunBodySyntax> for BaselineTagRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline tag surface"
    }

    fn rejects(&self, value: &ExpTagAtomRunBodySyntax) -> bool {
        is_baseline_tag(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpTagAtomRunBodySyntax>>
    for BaselineTagRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline tag surface"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::ExpTagAtomRunBodySyntax>) -> bool {
        valid(value).is_some_and(recovered_is_baseline_tag)
    }
}
