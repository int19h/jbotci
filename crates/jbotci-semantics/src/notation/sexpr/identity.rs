//! Lexical spelling of typed semantic-graph identities.
//!
//! A version-0 variable is `$` followed by a bare symbol, so an identity's
//! `Display` text is not an admissible spelling: eventuality subtype labels
//! contain `/`, which the document grammar reserves for exact rationals. Names
//! are therefore composed from the typed identity components — the namespace
//! carried by [`SemanticIdPrefix`] and the identity index — through the closed
//! token tables below, never by rewriting display text.
//!
//! Every token begins with an ASCII lowercase letter. Specification section 2.1
//! gives PascalCase its own closed namespace of registered primitives, prelude
//! names, types, and literals, so a generated `$Formula_1` would read as a
//! reserved spelling wearing a variable marker even though the grammar's
//! `symbol-name` production admits it. The renderer therefore keeps generated
//! variables inside the lowercase namespace that section 15.3's `$x`/`$e`/`$p`
//! stems already occupy.
//!
//! The tables deliberately duplicate no label from `model`: the display labels
//! belong to the identity module's own spelling, while these tokens belong to
//! this notation. Keeping them separate is what lets this notation guarantee
//! its own lexical grammar when a new sort or kind is added, because both
//! matches are exhaustive.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use super::super::kernel::intrinsic::Intrinsic;
use super::super::kernel::types::Variable;
use super::datum::Datum;
use super::type_syntax::variable_to_datum;
use crate::model::{
    EventualitySort, SemanticIdPrefix, SemanticObjectId, SemanticObjectKind, SemanticSort,
};

/// Stable lexical variable for one typed graph identity.
#[requires(true)]
#[ensures(ret.as_str().starts_with('$'))]
#[ensures(
    ret.as_str()[1..].starts_with(|first: char| first.is_ascii_lowercase()),
    "a generated variable never enters the reserved PascalCase namespace"
)]
pub(super) fn object_variable(id: SemanticObjectId) -> Variable {
    Variable::from_token_and_index(identity_namespace_token(id.prefix()), id.index())
}

/// Fresh local binder for one omitted section-11.3 crossing operand.
///
/// The crossing name is part of the namespace because two different omitted
/// operands on one graph identity are distinct contextual computations. Every
/// token ends in `Context`, which no graph-identity namespace does, so these
/// locals cannot collide with [`object_variable`].
#[requires(matches!(
    intrinsic,
    Intrinsic::Measure
        | Intrinsic::TruthValue
        | Intrinsic::ExperienceOf
        | Intrinsic::ProcessOf
        | Intrinsic::ActivityOf
        | Intrinsic::Concept
        | Intrinsic::Abstract
))]
#[ensures(ret.as_str().starts_with('$'))]
pub(super) fn crossing_default_variable(id: SemanticObjectId, intrinsic: Intrinsic) -> Variable {
    let token = match intrinsic {
        Intrinsic::Measure => "scaleContext",
        Intrinsic::TruthValue => "epistemologyContext",
        Intrinsic::ExperienceOf => "experiencerContext",
        Intrinsic::ProcessOf => "stagesContext",
        Intrinsic::ActivityOf => "actionsContext",
        Intrinsic::Concept => "mindContext",
        Intrinsic::Abstract => "categorizerContext",
        _ => unreachable!("the contract admits exactly the section-11.3 crossings"),
    };
    Variable::from_token_and_index(token, id.index())
}

/// Stable lexical variable datum for one typed graph identity.
#[requires(true)]
#[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('$')))]
#[ensures(ret.as_atom().is_some_and(|atom| atom[1..].starts_with(|first: char| first.is_ascii_lowercase())))]
pub(super) fn variable_datum(id: SemanticObjectId) -> Datum {
    variable_to_datum(&object_variable(id))
}

/// Closed lexical token for one typed identity namespace.
///
/// `SemanticObjectKind` and `SemanticSort` share five display labels
/// (`sequence`, `eventuality`, `predication`, `quantity`, and `sign`), so a
/// spelling that merely reused those labels would map two distinct identities
/// onto one variable. Both namespaces must nevertheless stay lowercase, so the
/// leading character cannot be the separator. Structural kinds instead carry
/// the model's own `…Node` stem for the graph object they identify, and no
/// referent sort's stem ends that way. Each table is injective, no token
/// contains `_`, and the `Node` suffix separates the tables, so
/// `$<token>_<index>` is injective over every `SemanticObjectId`.
#[requires(true)]
#[ensures(!ret.is_empty() && !ret.contains('_'))]
#[ensures(ret.starts_with(|first: char| first.is_ascii_lowercase()))]
fn identity_namespace_token(prefix: SemanticIdPrefix) -> &'static str {
    match prefix {
        SemanticIdPrefix::Structural(kind) => structural_namespace_token(kind),
        SemanticIdPrefix::Referent(sort) => referent_namespace_token(sort),
    }
}

/// camelCase `…Node` token for one structural object kind.
///
/// The suffix is what keeps this table disjoint from the referent-sort table
/// now that neither may use the leading character as its namespace marker. It
/// is also the model's own naming for these payloads (`UtteranceNode`,
/// `SequenceNode`, `ReferentNode`, and so on), so the spelling stays legible.
#[requires(true)]
#[ensures(ret.starts_with(|first: char| first.is_ascii_lowercase()) && !ret.contains('_'))]
#[ensures(ret.ends_with("Node"), "the structural namespace is marked by its stem, not its case")]
fn structural_namespace_token(kind: SemanticObjectKind) -> &'static str {
    match kind {
        SemanticObjectKind::Utterance => "utteranceNode",
        SemanticObjectKind::Sequence => "sequenceNode",
        SemanticObjectKind::Eventuality => "eventualityNode",
        SemanticObjectKind::Referent => "referentNode",
        SemanticObjectKind::Parameter => "parameterNode",
        SemanticObjectKind::Predication => "predicationNode",
        SemanticObjectKind::Formula => "formulaNode",
        SemanticObjectKind::Abstraction => "abstractionNode",
        SemanticObjectKind::Sign => "signNode",
        SemanticObjectKind::DisplayedContent => "displayNode",
        SemanticObjectKind::MathExpression => "mathNode",
        SemanticObjectKind::Quantity => "quantityNode",
        SemanticObjectKind::RelationMetadata => "relationMetadataNode",
        SemanticObjectKind::Question => "questionNode",
    }
}

/// camelCase token for one referent sort.
#[requires(true)]
#[ensures(ret.starts_with(|first: char| first.is_ascii_lowercase()) && !ret.contains('_'))]
#[ensures(!ret.ends_with("Node"), "the `…Node` stem belongs to the structural namespace")]
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
        SemanticSort::Epistemology => "epistemology",
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
    fn every_namespace_stays_out_of_the_reserved_pascal_case_namespace() {
        for prefix in all_prefixes() {
            let token = identity_namespace_token(prefix);
            assert!(
                token.starts_with(|first: char| first.is_ascii_lowercase()),
                "{prefix} spells the reserved-looking token {token:?}",
            );
            for index in [1usize, 9, 10, usize::MAX] {
                // The model keeps `SemanticObjectId`'s prefix constructor
                // private, so this composes the same spelling `object_variable`
                // composes rather than only the ids that have public
                // constructors; `distinct_identities_have_distinct_variables`
                // covers the constructible ids through `object_variable`.
                let variable = Variable::from_token_and_index(token, index);
                let after_marker = variable
                    .as_str()
                    .strip_prefix('$')
                    .expect("a variable always carries its `$` marker");
                assert!(
                    after_marker.starts_with(|first: char| first.is_ascii_lowercase()),
                    "{variable:?} puts a reserved PascalCase spelling behind `$`",
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
        // Both tables are lowercase, so the `…Node` stem — not the leading
        // character — is what keeps the two namespaces apart.
        for kind in ALL_KINDS {
            assert!(structural_namespace_token(kind).ends_with("Node"));
        }
        for sort in all_sorts() {
            assert!(!referent_namespace_token(sort).ends_with("Node"));
        }
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
            assert_eq!(
                variable_to_datum(&variable).as_atom(),
                Some(variable.as_str())
            );
        }
    }
}
