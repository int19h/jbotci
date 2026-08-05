//! Lexical spelling of typed semantic-graph identities.
//!
//! A version-0 variable is `$` followed by a bare symbol, so an identity's
//! `Display` text is not an admissible spelling: eventuality subtype labels
//! contain `/`, which the document grammar reserves for exact rationals. Names
//! are therefore composed from the typed identity components — the namespace
//! carried by [`SemanticIdPrefix`] and the identity index — through the closed
//! token tables below, never by rewriting display text.
//!
//! The tables deliberately duplicate no label from `model`: the display labels
//! belong to the identity module's own spelling, while these tokens belong to
//! this notation. Keeping them separate is what lets this notation guarantee
//! its own lexical grammar when a new sort or kind is added, because both
//! matches are exhaustive.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use super::datum::Datum;
use super::type_system::Variable;
use crate::model::{
    EventualitySort, SemanticIdPrefix, SemanticObjectId, SemanticObjectKind, SemanticSort,
};

/// Stable lexical variable for one typed graph identity.
#[requires(true)]
#[ensures(ret.as_str().starts_with('$'))]
pub(super) fn object_variable(id: SemanticObjectId) -> Variable {
    Variable::from_token_and_index(identity_namespace_token(id.prefix()), id.index())
}

/// Stable lexical variable datum for one typed graph identity.
#[requires(true)]
#[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('$')))]
pub(super) fn variable_datum(id: SemanticObjectId) -> Datum {
    object_variable(id).to_datum()
}

/// Closed lexical token for one typed identity namespace.
///
/// `SemanticObjectKind` and `SemanticSort` share five display labels
/// (`sequence`, `eventuality`, `predication`, `quantity`, and `sign`), so a
/// spelling that merely reused those labels would map two distinct identities
/// onto one variable. Structural kinds are therefore spelled PascalCase and
/// referent sorts camelCase: the leading character alone separates the two
/// namespaces, each table is injective, and no token contains `_`, so
/// `$<token>_<index>` is injective over every `SemanticObjectId`.
#[requires(true)]
#[ensures(!ret.is_empty() && !ret.contains('_'))]
fn identity_namespace_token(prefix: SemanticIdPrefix) -> &'static str {
    match prefix {
        SemanticIdPrefix::Structural(kind) => structural_namespace_token(kind),
        SemanticIdPrefix::Referent(sort) => referent_namespace_token(sort),
    }
}

/// PascalCase token for one structural object kind.
#[requires(true)]
#[ensures(ret.starts_with(|first: char| first.is_ascii_uppercase()) && !ret.contains('_'))]
fn structural_namespace_token(kind: SemanticObjectKind) -> &'static str {
    match kind {
        SemanticObjectKind::Utterance => "Utterance",
        SemanticObjectKind::Sequence => "Sequence",
        SemanticObjectKind::Eventuality => "Eventuality",
        SemanticObjectKind::Referent => "Referent",
        SemanticObjectKind::Parameter => "Parameter",
        SemanticObjectKind::Predication => "Predication",
        SemanticObjectKind::Formula => "Formula",
        SemanticObjectKind::Abstraction => "Abstraction",
        SemanticObjectKind::Sign => "Sign",
        SemanticObjectKind::DisplayedContent => "Display",
        SemanticObjectKind::MathExpression => "Math",
        SemanticObjectKind::Quantity => "Quantity",
        SemanticObjectKind::RelationMetadata => "RelationMetadata",
        SemanticObjectKind::Question => "Question",
    }
}

/// camelCase token for one referent sort.
#[requires(true)]
#[ensures(ret.starts_with(|first: char| first.is_ascii_lowercase()) && !ret.contains('_'))]
fn referent_namespace_token(sort: SemanticSort) -> &'static str {
    match sort {
        SemanticSort::Entity => "entity",
        SemanticSort::Mass => "mass",
        SemanticSort::Set => "set",
        SemanticSort::Sequence => "sequence",
        SemanticSort::Time => "time",
        SemanticSort::Eventuality(sort) => eventuality_namespace_token(sort),
        SemanticSort::Predication => "predication",
        SemanticSort::TruthValue => "truthValue",
        SemanticSort::Proposition => "proposition",
        SemanticSort::Concept => "concept",
        SemanticSort::Amount => "amount",
        SemanticSort::Quantity => "quantity",
        SemanticSort::Number => "number",
        SemanticSort::Scale => "scale",
        SemanticSort::Text => "text",
        SemanticSort::Sign => "sign",
        SemanticSort::Relation => "relation",
        SemanticSort::Place => "place",
        SemanticSort::Connective => "connective",
        SemanticSort::TenseModal => "tenseModal",
        SemanticSort::MathOperator => "mathOperator",
        SemanticSort::ArgumentBundle => "argumentBundle",
        SemanticSort::AbstractNature => "abstractNature",
    }
}

/// camelCase token for one eventuality subtype.
///
/// The model spells these subtypes with `/`; this notation cannot, so each
/// subtype gets its own token rather than a rewritten label.
#[requires(true)]
#[ensures(ret.starts_with("eventuality") && !ret.contains('_'))]
fn eventuality_namespace_token(sort: EventualitySort) -> &'static str {
    match sort {
        EventualitySort::General => "eventuality",
        EventualitySort::State => "eventualityState",
        EventualitySort::Process => "eventualityProcess",
        EventualitySort::Activity => "eventualityActivity",
        EventualitySort::Achievement => "eventualityAchievement",
        EventualitySort::Experience => "eventualityExperience",
        EventualitySort::Locution => "eventualityLocution",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::super::datum::Atom;
    use super::*;

    /// Every structural object kind in the model.
    const ALL_KINDS: [SemanticObjectKind; 14] = [
        SemanticObjectKind::Utterance,
        SemanticObjectKind::Sequence,
        SemanticObjectKind::Eventuality,
        SemanticObjectKind::Referent,
        SemanticObjectKind::Parameter,
        SemanticObjectKind::Predication,
        SemanticObjectKind::Formula,
        SemanticObjectKind::Abstraction,
        SemanticObjectKind::Sign,
        SemanticObjectKind::DisplayedContent,
        SemanticObjectKind::MathExpression,
        SemanticObjectKind::Quantity,
        SemanticObjectKind::RelationMetadata,
        SemanticObjectKind::Question,
    ];

    /// Every eventuality subtype in the model.
    const ALL_EVENTUALITY_SORTS: [EventualitySort; 7] = [
        EventualitySort::General,
        EventualitySort::State,
        EventualitySort::Process,
        EventualitySort::Activity,
        EventualitySort::Achievement,
        EventualitySort::Experience,
        EventualitySort::Locution,
    ];

    /// Every referent sort in the model, including all eventuality subtypes.
    #[requires(true)]
    #[ensures(ret.len() == 29)]
    fn all_sorts() -> Vec<SemanticSort> {
        let mut sorts = vec![
            SemanticSort::Entity,
            SemanticSort::Mass,
            SemanticSort::Set,
            SemanticSort::Sequence,
            SemanticSort::Time,
            SemanticSort::Predication,
            SemanticSort::TruthValue,
            SemanticSort::Proposition,
            SemanticSort::Concept,
            SemanticSort::Amount,
            SemanticSort::Quantity,
            SemanticSort::Number,
            SemanticSort::Scale,
            SemanticSort::Text,
            SemanticSort::Sign,
            SemanticSort::Relation,
            SemanticSort::Place,
            SemanticSort::Connective,
            SemanticSort::TenseModal,
            SemanticSort::MathOperator,
            SemanticSort::ArgumentBundle,
            SemanticSort::AbstractNature,
        ];
        sorts.extend(ALL_EVENTUALITY_SORTS.map(SemanticSort::Eventuality));
        sorts
    }

    /// Every identity namespace the model can mint.
    #[requires(true)]
    #[ensures(ret.len() == 43)]
    fn all_prefixes() -> Vec<SemanticIdPrefix> {
        ALL_KINDS
            .into_iter()
            .map(SemanticIdPrefix::Structural)
            .chain(all_sorts().into_iter().map(SemanticIdPrefix::Referent))
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_namespace_spells_a_valid_variable_atom() {
        for prefix in all_prefixes() {
            for index in [1usize, 9, 10, usize::MAX] {
                let variable =
                    Variable::from_token_and_index(identity_namespace_token(prefix), index);
                assert!(
                    Variable::try_new(variable.as_str()).is_ok(),
                    "{variable:?} must re-parse as a variable",
                );
                assert!(
                    Atom::try_new(variable.as_str()).is_ok(),
                    "{variable:?} must print as a bare atom",
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn namespaces_never_collide() {
        let prefixes = all_prefixes();
        let tokens = prefixes
            .iter()
            .map(|prefix| identity_namespace_token(*prefix))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            tokens.len(),
            prefixes.len(),
            "identity namespace tokens must be pairwise distinct",
        );
        // Five kind labels and sort labels coincide in the model's own display
        // spelling; the namespaces must nevertheless stay separated here.
        for shared in [
            SemanticObjectKind::Sequence,
            SemanticObjectKind::Eventuality,
            SemanticObjectKind::Predication,
            SemanticObjectKind::Quantity,
            SemanticObjectKind::Sign,
        ] {
            assert_ne!(
                structural_namespace_token(shared),
                referent_namespace_token(match shared {
                    SemanticObjectKind::Sequence => SemanticSort::Sequence,
                    SemanticObjectKind::Eventuality => SemanticSort::eventuality(),
                    SemanticObjectKind::Predication => SemanticSort::Predication,
                    SemanticObjectKind::Quantity => SemanticSort::Quantity,
                    _ => SemanticSort::Sign,
                }),
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn distinct_identities_have_distinct_variables() {
        let mut spellings = BTreeSet::new();
        for sort in all_sorts() {
            for index in 1..=3 {
                assert!(
                    spellings.insert(
                        object_variable(SemanticObjectId::referent_with_sort(sort, index))
                            .as_str()
                            .to_owned()
                    ),
                    "referent {sort:?}:{index} reused a variable spelling",
                );
            }
        }
        for index in 1..=3 {
            for id in [
                SemanticObjectId::utterance(index),
                SemanticObjectId::sequence(index),
                SemanticObjectId::parameter(index),
                SemanticObjectId::predication(index),
                SemanticObjectId::formula(index),
                SemanticObjectId::displayed_content(index),
                SemanticObjectId::math_expression(index),
                SemanticObjectId::quantity(index),
                SemanticObjectId::relation_metadata(index),
                SemanticObjectId::question(index),
            ] {
                assert!(
                    spellings.insert(object_variable(id).as_str().to_owned()),
                    "{id} reused a variable spelling",
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn eventuality_subtypes_do_not_leak_the_model_separator() {
        for sort in ALL_EVENTUALITY_SORTS {
            let id = SemanticObjectId::referent_with_sort(SemanticSort::Eventuality(sort), 17);
            let variable = object_variable(id);
            assert!(
                !variable.as_str().contains('/'),
                "{variable:?} must not carry the model's subtype separator",
            );
            assert_eq!(variable.to_datum().as_atom(), Some(variable.as_str()));
        }
    }
}
