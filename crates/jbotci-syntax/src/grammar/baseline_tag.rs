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
fn valid_wf<'tree, K: jbotci_tree::view::SourceKind, V: view::Inspect, S, R, P>(
    value: &view::WithFreeModifiersView<
        view::Slot<V>,
        jbotci_tree::view::Sequence<'tree, S, R, view::Slot<view::FreeModifierSyntaxRef<'tree>>, P>,
    >,
) -> bool
where
    P: jbotci_tree::view::Projection<
            'tree,
            S,
            R,
            View = view::Slot<view::FreeModifierSyntaxRef<'tree>>,
        >,
{
    if K::STRICT {
        return true;
    }
    let view::WithFreeModifiersView {
        value,
        free_modifiers,
    } = *value;
    value.valid().is_some()
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
            valid(atom).is_some_and(|view::ExpPaNumberAtomSyntaxView(pa)| pa.valid().is_some())
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
fn classify_atom<K: jbotci_tree::view::SourceKind>(
    atom: view::ExpPrefixedTagAtomSyntaxRef<'_>,
) -> Option<ClassifiedAtom> {
    // This pattern requires a decision for each field without projecting the product.
    let _ = |view::ExpPrefixedTagAtomSyntaxView {
                 nahe: _,
                 se: _,
                 atom: _,
             }: view::ExpPrefixedTagAtomSyntaxView<'_>| ();
    let nahe = atom.field_nahe();
    let se = atom.field_se();
    let atom = atom.field_atom();
    let nahe = match nahe {
        Some(nahe) if valid_wf::<K, _, _, _, _>(&nahe) => true,
        Some(_) => return None,
        None => false,
    };
    let se = match se {
        Some(se) if valid_wf::<K, _, _, _, _>(&se) => true,
        Some(_) => return None,
        None => false,
    };
    if !K::STRICT
        && !atom
            .free_modifiers
            .iter()
            .all(|modifier| modifier.valid().is_some())
    {
        return None;
    }
    let kind = match valid(&atom.value)? {
        view::ExpTagAtomSyntaxView::ExpBaiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpBaiTagAtomSyntaxView(bai)| {
                bai.valid().is_some().then_some(AtomKind::Bai)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpCahaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpCahaTagAtomSyntaxView(caha)| {
                caha.valid().is_some().then_some(AtomKind::Caha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpCuheTagAtom(atom) => {
            valid(atom).and_then(|view::ExpCuheTagAtomSyntaxView(cuhe)| {
                cuhe.valid().is_some().then_some(AtomKind::Cuhe)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpKiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpKiTagAtomSyntaxView(ki)| {
                ki.valid().is_some().then_some(AtomKind::Ki)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZiTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZiTagAtomSyntaxView(zi)| {
                zi.valid().is_some().then_some(AtomKind::Zi)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpPuTagAtom(atom) => {
            valid(atom).and_then(|view::ExpPuTagAtomSyntaxView(pu)| {
                pu.valid().is_some().then_some(AtomKind::Pu)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVaTagAtomSyntaxView(va)| {
                va.valid().is_some().then_some(AtomKind::Va)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFahaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFahaTagAtomSyntaxView { mohi, faha }| {
                (mohi.as_ref().is_none_or(|mohi| mohi.valid().is_some()) && faha.valid().is_some())
                    .then_some(AtomKind::Faha {
                        mohi: mohi.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZehaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZehaTagAtomSyntaxView(zeha)| {
                zeha.valid().is_some().then_some(AtomKind::Zeha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVehaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVehaTagAtomSyntaxView(veha)| {
                veha.valid().is_some().then_some(AtomKind::Veha)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpVihaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpVihaTagAtomSyntaxView(viha)| {
                viha.valid().is_some().then_some(AtomKind::Viha)
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
                        if !valid_wf::<K, _, _, _, _>(&vei)
                            || expression.valid().is_none()
                            || !veho.as_ref().is_none_or(valid_wf::<K, _, _, _, _>)
                        {
                            return None;
                        }
                        false
                    }
                    view::ExpRoiIntervalSyntaxView::ExpNumber(number) => {
                        valid(number).is_some_and(exp_number_is_baseline)
                    }
                };
                (fehe.as_ref().is_none_or(|fehe| fehe.valid().is_some()) && roi.valid().is_some())
                    .then_some(AtomKind::Roi {
                        fehe: fehe.is_some(),
                        baseline_number,
                    })
            },
        )?,
        view::ExpTagAtomSyntaxView::ExpTaheTagAtom(atom) => {
            valid(atom).and_then(|view::ExpTaheTagAtomSyntaxView { fehe, tahe }| {
                (fehe.as_ref().is_none_or(|fehe| fehe.valid().is_some()) && tahe.valid().is_some())
                    .then_some(AtomKind::Tahe {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpZahoTagAtom(atom) => {
            valid(atom).and_then(|view::ExpZahoTagAtomSyntaxView { fehe, zaho }| {
                (fehe.as_ref().is_none_or(|fehe| fehe.valid().is_some()) && zaho.valid().is_some())
                    .then_some(AtomKind::Zaho {
                        fehe: fehe.is_some(),
                    })
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFihoTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFihoTagAtomSyntaxView { fiho, selbri, fehu }| {
                (valid_wf::<K, _, _, _, _>(&fiho)
                    && selbri.valid().is_some()
                    && fehu.as_ref().is_none_or(valid_wf::<K, _, _, _, _>))
                .then_some(AtomKind::Fiho)
            })?
        }
        view::ExpTagAtomSyntaxView::ExpFaTagAtom(atom) => {
            valid(atom).and_then(|view::ExpFaTagAtomSyntaxView(fa)| {
                fa.valid().is_some().then_some(AtomKind::Fa)
            })?
        }
    };
    Some(ClassifiedAtom { nahe, se, kind })
}

#[requires(index <= run.additional.len())]
#[ensures(true)]
fn classified<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
    index: usize,
) -> Option<ClassifiedAtom> {
    let view::ExpTagAtomRunBodySyntaxView { first, additional } = run;
    if index == 0 {
        first.valid().and_then(classify_atom::<K>)
    } else {
        additional
            .get(index - 1)
            .expect("index is within the run")
            .valid()
            .and_then(classify_atom::<K>)
    }
}

#[requires(start <= end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn all_unprefixed(run: view::ExpTagAtomRunBodySyntaxView<'_>, start: usize, end: usize) -> bool {
    // The accepted composite already proves each atom. This pass only checks prefixes.
    (start..end).all(|index| {
        let atom = if index == 0 {
            run.first
        } else {
            run.additional
                .get(index - 1)
                .expect("index is within the run")
        };
        valid(atom).is_some_and(|view::ExpPrefixedTagAtomSyntaxView { nahe, se, atom: _ }| {
            nahe.is_none() && se.is_none()
        })
    })
}

/// Peek retains the current classification until the cursor consumes it.
#[requires(true)]
#[ensures(true)]
fn take_kind(
    atoms: &mut std::iter::Peekable<impl Iterator<Item = Option<ClassifiedAtom>>>,
    matches: impl FnOnce(AtomKind) -> bool,
) -> bool {
    if atoms
        .peek()
        .is_some_and(|atom| atom.is_some_and(|atom| matches(atom.kind)))
    {
        atoms.next();
        true
    } else {
        false
    }
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn time_slice<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
    start: usize,
    end: usize,
) -> bool {
    let mut atoms = (start..end)
        .map(|index| classified::<K>(run, index))
        .peekable();
    let leading_zi = take_kind(&mut atoms, |kind| kind == AtomKind::Zi);
    let mut has_offsets = false;
    while take_kind(&mut atoms, |kind| kind == AtomKind::Pu) {
        has_offsets = true;
        take_kind(&mut atoms, |kind| kind == AtomKind::Zi);
    }
    let interval = take_kind(&mut atoms, |kind| kind == AtomKind::Zeha);
    if interval {
        take_kind(&mut atoms, |kind| kind == AtomKind::Pu);
    }
    let mut has_properties = false;
    while take_kind(&mut atoms, is_time_property) {
        has_properties = true;
    }
    atoms.peek().is_none() && (leading_zi || has_offsets || interval || has_properties)
}

#[requires(start < end)]
#[requires(end <= run.additional.len() + 1)]
#[ensures(true)]
fn space_slice<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
    start: usize,
    end: usize,
) -> bool {
    let mut atoms = (start..end)
        .map(|index| classified::<K>(run, index))
        .peekable();
    let leading_va = take_kind(&mut atoms, |kind| kind == AtomKind::Va);
    let mut has_offsets = false;
    while take_kind(&mut atoms, |kind| {
        matches!(kind, AtomKind::Faha { mohi: false })
    }) {
        has_offsets = true;
        take_kind(&mut atoms, |kind| kind == AtomKind::Va);
    }
    let veha = take_kind(&mut atoms, |kind| kind == AtomKind::Veha);
    let viha = take_kind(&mut atoms, |kind| kind == AtomKind::Viha);
    let direction = (veha || viha)
        && take_kind(&mut atoms, |kind| {
            matches!(kind, AtomKind::Faha { mohi: false })
        });
    let mut has_properties = false;
    while take_kind(&mut atoms, is_space_property) {
        has_properties = true;
    }
    let interval = veha || viha || direction || has_properties;
    let mohi = take_kind(&mut atoms, |kind| {
        matches!(kind, AtomKind::Faha { mohi: true })
    });
    if mohi {
        take_kind(&mut atoms, |kind| kind == AtomKind::Va);
    }
    atoms.peek().is_none() && (leading_va || has_offsets || interval || mohi)
}

#[requires(true)]
#[ensures(true)]
fn composite_is_baseline<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
    start: usize,
    end: usize,
) -> bool {
    if start >= end {
        return false;
    }
    if end == start + 1
        && classified::<K>(run, start).is_some_and(|atom| atom.kind == AtomKind::Caha)
    {
        return true;
    }
    let without_caha =
        if classified::<K>(run, end - 1).is_some_and(|atom| atom.kind == AtomKind::Caha) {
            end - 1
        } else {
            end
        };
    if without_caha == start {
        return false;
    }
    (start + 1..=without_caha).any(|time_end| {
        time_slice::<K>(run, start, time_end)
            && (time_end == without_caha
                || time_end < without_caha && space_slice::<K>(run, time_end, without_caha))
    }) || (start + 1..=without_caha).any(|space_end| {
        space_slice::<K>(run, start, space_end)
            && (space_end == without_caha
                || space_end < without_caha && time_slice::<K>(run, space_end, without_caha))
    })
}

#[requires(true)]
#[ensures(true)]
fn is_single_composite(kind: AtomKind) -> bool {
    matches!(
        kind,
        AtomKind::Caha
            | AtomKind::Zi
            | AtomKind::Pu
            | AtomKind::Zeha
            | AtomKind::Va
            | AtomKind::Faha { .. }
            | AtomKind::Veha
            | AtomKind::Viha
    ) || is_time_property(kind)
        || is_space_property(kind)
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_tag<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
) -> bool {
    let len = run.additional.len() + 1;
    let Some(first) = classified::<K>(run, 0) else {
        return false;
    };
    if first.kind == AtomKind::Fiho {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.kind == AtomKind::Bai {
        return len == 1
            || len == 2
                && classified::<K>(run, 1)
                    .is_some_and(|atom| atom.kind == AtomKind::Ki && !atom.nahe && !atom.se);
    }
    if matches!(first.kind, AtomKind::Cuhe | AtomKind::Ki) {
        return len == 1 && !first.nahe && !first.se;
    }
    if first.se {
        return false;
    }
    if len == 1 {
        return is_single_composite(first.kind);
    }
    let composite_end =
        if len > 1 && classified::<K>(run, len - 1).is_some_and(|atom| atom.kind == AtomKind::Ki) {
            len - 1
        } else {
            len
        };
    composite_is_baseline::<K>(run, 0, composite_end) && all_unprefixed(run, 1, len)
}

#[requires(true)]
#[ensures(true)]
fn exp_run_starts_with_baseline_term<K: jbotci_tree::view::SourceKind>(
    run: view::ExpTagAtomRunBodySyntaxView<'_>,
) -> bool {
    let Some(first) = classified::<K>(run, 0) else {
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
    !first.se && is_single_composite(first.kind)
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
fn tense_modal_is_extension<K: jbotci_tree::view::SourceKind>(
    output: view::TenseModalSyntaxView<'_>,
) -> bool {
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
                        .is_some_and(exp_run_starts_with_baseline_term::<K>),
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
                    .is_some_and(exp_run_starts_with_baseline_term::<K>),
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
        .is_some_and(|nahe| nahe.value.valid().is_some() && nahe.free_modifiers.is_empty())
        && se.is_none()
        && atom.value.valid().is_some()
        && atom.free_modifiers.is_empty()
        && additional.is_empty()
        && fiho.value.valid().is_some()
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
        valid(value.as_view()).is_some_and(is_baseline_tag::<<T as AsView<'_>>::Kind>)
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
                valid(tense_modal).is_some_and(tense_modal_is_extension::<<T as AsView<'_>>::Kind>)
            }),
            view::SelbriSyntaxView::UntaggedSelbri(_) => false,
        })
    }
}
