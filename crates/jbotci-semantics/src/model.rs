//! Public semantic object graph model serialized by `tersmu --format json`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroUsize;
use std::str::FromStr;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use jbotci_source::SourceSpan;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

pub const SEMANTIC_JSON_VERSION: &str = "lojban-semantics-json-1";

/// One-based numbered argument place such as `x1`.
///
/// `Ord` follows the numeric index, not the serialized label text. This
/// deliberately replaces the old string-key lexicographic JSON map order, so
/// maps containing `x2` and `x10` serialize in numeric place order.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceIndex(NonZeroUsize);

impl PlaceIndex {
    #[requires(index > 0)]
    #[ensures(ret.get() == index)]
    pub fn new(index: usize) -> Self {
        Self(NonZeroUsize::new(index).expect("place indices are one-based"))
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn get(self) -> usize {
        self.0.get()
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|place| place.get() > 0))]
    pub fn from_numbered_label(place: &str) -> Option<Self> {
        let digits = place.strip_prefix('x')?;
        if digits.is_empty() || digits.starts_with('0') {
            return None;
        }
        digits
            .parse::<usize>()
            .ok()
            .and_then(NonZeroUsize::new)
            .map(Self)
    }
}

impl fmt::Display for PlaceIndex {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "x{}", self.get())
    }
}

impl FromStr for PlaceIndex {
    type Err = ();

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|place| place.get() > 0) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_numbered_label(value).ok_or(())
    }
}

impl Serialize for PlaceIndex {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

const RELATION_LABEL_IDENTITY_TEXT: &str = "identity";
const RELATION_LABEL_DU_TEXT: &str = "du";

#[invariant(::Brivla { word } => !word.is_empty())]
#[invariant(::Identity => !RELATION_LABEL_IDENTITY_TEXT.is_empty())]
#[invariant(::Du => !RELATION_LABEL_DU_TEXT.is_empty())]
#[invariant(::ProBridi { word } => !word.is_empty())]
#[invariant(::Abstraction { abstractor, relation, .. } =>
    !abstractor.is_empty() && relation.is_displayable())]
#[invariant(::NuhaOperator { operator } => !operator.is_empty())]
#[invariant(::MeksoMoi { expression, moi } => !expression.is_empty() && !moi.is_empty())]
#[invariant(::ZeiCompound { text } => !text.is_empty())]
#[invariant(::StatementConnection { left, connector, right } =>
    left.is_displayable() && !connector.is_empty() && right.is_displayable())]
#[invariant(::TextGroup { modifier, opener, relation, closer } =>
    modifier.as_ref().is_none_or(|modifier| !modifier.is_empty())
        && !opener.is_empty()
        && relation.is_displayable()
        && closer.as_ref().is_none_or(|closer| !closer.is_empty()))]
#[invariant(::Prenex { terms, separator, relation } =>
    terms.iter().all(|term| !term.is_empty())
        && !separator.is_empty()
        && relation.is_displayable())]
#[invariant(::ForethoughtStatementConnection { opener, first, branches, closer } =>
    !opener.is_empty()
        && first.is_displayable()
        && !branches.is_empty()
        && branches.iter().all(ForethoughtRelationBranch::is_displayable)
        && closer.as_ref().is_none_or(|closer| !closer.is_empty()))]
#[invariant(::Constructed { text } => !text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationLabel {
    Brivla {
        word: String,
    },
    Identity,
    Du,
    ProBridi {
        word: String,
    },
    Abstraction {
        kind: AbstractionKind,
        abstractor: String,
        relation: Box<RelationLabel>,
    },
    NuhaOperator {
        operator: String,
    },
    MeksoMoi {
        expression: String,
        moi: String,
    },
    ZeiCompound {
        text: String,
    },
    StatementConnection {
        left: Box<RelationLabel>,
        connector: String,
        right: Box<RelationLabel>,
    },
    TextGroup {
        modifier: Option<String>,
        opener: String,
        relation: Box<RelationLabel>,
        closer: Option<String>,
    },
    Prenex {
        terms: Vec<String>,
        separator: String,
        relation: Box<RelationLabel>,
    },
    ForethoughtStatementConnection {
        opener: String,
        first: Box<RelationLabel>,
        branches: Vec<ForethoughtRelationBranch>,
        closer: Option<String>,
    },
    Constructed {
        text: String,
    },
}

impl RelationLabel {
    #[requires(!word.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn brivla(word: String) -> Self {
        new!(RelationLabel::Brivla { word })
    }

    #[requires(true)]
    #[ensures(ret.is_displayable())]
    pub fn identity() -> Self {
        new!(RelationLabel::Identity)
    }

    #[requires(true)]
    #[ensures(ret.is_displayable())]
    pub fn du() -> Self {
        new!(RelationLabel::Du)
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn pro_bridi(word: String) -> Self {
        new!(RelationLabel::ProBridi { word })
    }

    #[requires(!abstractor.is_empty())]
    #[requires(relation.is_displayable())]
    #[ensures(ret.is_displayable())]
    pub fn abstraction(kind: AbstractionKind, abstractor: String, relation: Self) -> Self {
        new!(RelationLabel::Abstraction {
            kind,
            abstractor,
            relation: Box::new(relation),
        })
    }

    #[requires(!operator.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn nuha_operator(operator: String) -> Self {
        new!(RelationLabel::NuhaOperator { operator })
    }

    #[requires(!expression.is_empty())]
    #[requires(!moi.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn mekso_moi(expression: String, moi: String) -> Self {
        new!(RelationLabel::MeksoMoi { expression, moi })
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn zei_compound(text: String) -> Self {
        new!(RelationLabel::ZeiCompound { text })
    }

    #[requires(left.is_displayable())]
    #[requires(!connector.is_empty())]
    #[requires(right.is_displayable())]
    #[ensures(ret.is_displayable())]
    pub fn statement_connection(left: Self, connector: String, right: Self) -> Self {
        new!(RelationLabel::StatementConnection {
            left: Box::new(left),
            connector,
            right: Box::new(right),
        })
    }

    #[requires(modifier.as_ref().is_none_or(|modifier| !modifier.is_empty()))]
    #[requires(!opener.is_empty())]
    #[requires(relation.is_displayable())]
    #[requires(closer.as_ref().is_none_or(|closer| !closer.is_empty()))]
    #[ensures(ret.is_displayable())]
    pub fn text_group(
        modifier: Option<String>,
        opener: String,
        relation: Self,
        closer: Option<String>,
    ) -> Self {
        new!(RelationLabel::TextGroup {
            modifier,
            opener,
            relation: Box::new(relation),
            closer,
        })
    }

    #[requires(terms.iter().all(|term| !term.is_empty()))]
    #[requires(!separator.is_empty())]
    #[requires(relation.is_displayable())]
    #[ensures(ret.is_displayable())]
    pub fn prenex(terms: Vec<String>, separator: String, relation: Self) -> Self {
        new!(RelationLabel::Prenex {
            terms,
            separator,
            relation: Box::new(relation),
        })
    }

    #[requires(!opener.is_empty())]
    #[requires(first.is_displayable())]
    #[requires(!branches.is_empty())]
    #[requires(branches.iter().all(ForethoughtRelationBranch::is_displayable))]
    #[requires(closer.as_ref().is_none_or(|closer| !closer.is_empty()))]
    #[ensures(ret.is_displayable())]
    pub fn forethought_statement_connection(
        opener: String,
        first: Self,
        branches: Vec<ForethoughtRelationBranch>,
        closer: Option<String>,
    ) -> Self {
        new!(RelationLabel::ForethoughtStatementConnection {
            opener,
            first: Box::new(first),
            branches,
            closer,
        })
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.is_displayable())]
    pub fn constructed(text: String) -> Self {
        new!(RelationLabel::Constructed { text })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_displayable(&self) -> bool {
        match self.as_data() {
            data!(RelationLabel::Brivla { word }) | data!(RelationLabel::ProBridi { word }) => {
                !word.is_empty()
            }
            data!(RelationLabel::Identity) | data!(RelationLabel::Du) => true,
            data!(RelationLabel::Abstraction {
                abstractor,
                relation,
                ..
            }) => !abstractor.is_empty() && relation.is_displayable(),
            data!(RelationLabel::NuhaOperator { operator }) => !operator.is_empty(),
            data!(RelationLabel::MeksoMoi { expression, moi }) => {
                !expression.is_empty() && !moi.is_empty()
            }
            data!(RelationLabel::StatementConnection {
                left,
                connector,
                right,
            }) => left.is_displayable() && !connector.is_empty() && right.is_displayable(),
            data!(RelationLabel::TextGroup {
                modifier,
                opener,
                relation,
                closer,
            }) => {
                modifier
                    .as_ref()
                    .is_none_or(|modifier| !modifier.is_empty())
                    && !opener.is_empty()
                    && relation.is_displayable()
                    && closer.as_ref().is_none_or(|closer| !closer.is_empty())
            }
            data!(RelationLabel::Prenex {
                terms,
                separator,
                relation,
            }) => {
                terms.iter().all(|term| !term.is_empty())
                    && !separator.is_empty()
                    && relation.is_displayable()
            }
            data!(RelationLabel::ForethoughtStatementConnection {
                opener,
                first,
                branches,
                closer,
            }) => {
                !opener.is_empty()
                    && first.is_displayable()
                    && !branches.is_empty()
                    && branches
                        .iter()
                        .all(ForethoughtRelationBranch::is_displayable)
                    && closer.as_ref().is_none_or(|closer| !closer.is_empty())
            }
            data!(RelationLabel::ZeiCompound { text })
            | data!(RelationLabel::Constructed { text }) => !text.is_empty(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn display_text(&self) -> String {
        match self.as_data() {
            data!(RelationLabel::Brivla { word })
            | data!(RelationLabel::ProBridi { word })
            | data!(RelationLabel::ZeiCompound { text: word })
            | data!(RelationLabel::Constructed { text: word }) => word.clone(),
            data!(RelationLabel::Identity) => RELATION_LABEL_IDENTITY_TEXT.to_owned(),
            data!(RelationLabel::Du) => RELATION_LABEL_DU_TEXT.to_owned(),
            data!(RelationLabel::Abstraction {
                abstractor,
                relation,
                ..
            }) => format!("{abstractor} {relation}"),
            data!(RelationLabel::NuhaOperator { operator }) => format!("nu'a {operator}"),
            data!(RelationLabel::MeksoMoi { expression, moi }) => {
                format!("{expression} {moi}")
            }
            data!(RelationLabel::StatementConnection {
                left,
                connector,
                right,
            }) => format!(
                "({}) {connector} ({})",
                left.display_text(),
                right.display_text()
            ),
            data!(RelationLabel::TextGroup {
                modifier,
                opener,
                relation,
                closer,
            }) => {
                let modifier = modifier
                    .as_ref()
                    .map_or_else(String::new, |modifier| format!("{modifier} "));
                let closer = closer
                    .as_ref()
                    .map_or_else(String::new, |closer| format!(" {closer}"));
                format!("{modifier}{opener} {}{closer}", relation.display_text())
            }
            data!(RelationLabel::Prenex {
                terms,
                separator,
                relation,
            }) => {
                let terms = terms.join(" ");
                let separator = if terms.is_empty() {
                    separator.clone()
                } else {
                    format!("{terms} {separator}")
                };
                format!("{separator} {}", relation.display_text())
            }
            data!(RelationLabel::ForethoughtStatementConnection {
                opener,
                first,
                branches,
                closer,
            }) => {
                let mut text = format!("{opener} ({})", first.display_text());
                for branch in branches {
                    text.push_str(&format!(
                        " {} ({})",
                        branch.separator,
                        branch.relation.display_text()
                    ));
                }
                if let Some(closer) = closer {
                    text.push(' ');
                    text.push_str(closer);
                }
                text
            }
        }
    }
}

#[invariant(!separator.is_empty())]
#[invariant(relation.is_displayable())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForethoughtRelationBranch {
    pub separator: String,
    pub relation: RelationLabel,
}

impl ForethoughtRelationBranch {
    #[requires(!separator.is_empty())]
    #[requires(relation.is_displayable())]
    #[ensures(ret.is_displayable())]
    pub fn new(separator: String, relation: RelationLabel) -> Self {
        Self::from_data(data!(ForethoughtRelationBranch {
            separator,
            relation,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_displayable(&self) -> bool {
        !self.separator.is_empty() && self.relation.is_displayable()
    }
}

impl fmt::Display for RelationLabel {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.display_text())
    }
}

impl Serialize for RelationLabel {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[invariant(*index > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticObjectId {
    prefix: SemanticIdPrefix,
    index: usize,
}

impl SemanticObjectId {
    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Utterance)]
    pub fn utterance(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Utterance),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Sequence),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn eventuality(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::eventuality(), index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn referent(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::Entity, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(sort))]
    pub fn referent_with_sort(sort: SemanticSort, index: usize) -> Self {
        Self::numbered(SemanticIdPrefix::Referent(sort), index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Parameter)]
    pub fn parameter(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Parameter),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn predication(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Predication),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn formula(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Formula),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::AbstractNature))]
    pub fn abstraction(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::AbstractNature, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::Sign))]
    pub fn sign(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::Sign, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::DisplayedContent)]
    pub fn displayed_content(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::DisplayedContent),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::MathExpression),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Quantity)]
    pub fn quantity(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Quantity),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::RelationMetadata)]
    pub fn relation_metadata(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::RelationMetadata),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    pub fn question(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Question),
            index,
        )
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn speaker() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 1)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn addressee() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 2)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn now() -> Self {
        Self::referent_with_sort(SemanticSort::eventuality(), 3)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn speech_time() -> Self {
        Self::now()
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn here() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 4)
    }

    #[requires(index > 0)]
    #[ensures(ret.prefix == prefix)]
    fn numbered(prefix: SemanticIdPrefix, index: usize) -> Self {
        new!(SemanticObjectId {
            prefix: prefix,
            index: index,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn object_kind(self) -> SemanticObjectKind {
        self.prefix.object_kind()
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (self.object_kind() == SemanticObjectKind::Referent))]
    pub fn referent_sort(self) -> Option<SemanticSort> {
        match self.prefix {
            SemanticIdPrefix::Referent(sort) => Some(sort),
            SemanticIdPrefix::Structural(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for SemanticObjectId {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.prefix, self.index)
    }
}

impl Serialize for SemanticObjectId {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// An eventuality whose existential force is supplied by a typed scope-owner edge.
///
/// The wrapped ID is intentionally not constructible outside the model implementation.
/// Callers can therefore inspect a binding without manufacturing one from a referential
/// eventuality ID.
#[invariant(id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneratedEventualityId {
    id: SemanticObjectId,
}

impl GeneratedEventualityId {
    #[requires(id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(ret.id == id)]
    pub(crate) fn new(id: SemanticObjectId) -> Self {
        new!(GeneratedEventualityId { id })
    }

    #[requires(true)]
    #[ensures(ret == self.id)]
    pub fn object_id(self) -> SemanticObjectId {
        self.id
    }
}

impl fmt::Display for GeneratedEventualityId {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(formatter)
    }
}

impl Serialize for GeneratedEventualityId {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.id.serialize(serializer)
    }
}

/// The two semantic object kinds that can own a generated-event binding.
#[invariant(::Formula { formula } => formula.object_kind() == SemanticObjectKind::Formula)]
#[invariant(::Sequence { sequence } => sequence.object_kind() == SemanticObjectKind::Sequence)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventBindingScope {
    Formula { formula: SemanticObjectId },
    Sequence { sequence: SemanticObjectId },
}

impl EventBindingScope {
    #[requires(formula.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.owner() == formula)]
    pub fn formula(formula: SemanticObjectId) -> Self {
        new!(EventBindingScope::Formula { formula })
    }

    #[requires(sequence.object_kind() == SemanticObjectKind::Sequence)]
    #[ensures(ret.owner() == sequence)]
    pub fn sequence(sequence: SemanticObjectId) -> Self {
        new!(EventBindingScope::Sequence { sequence })
    }

    #[requires(true)]
    #[ensures(matches!(ret.object_kind(), SemanticObjectKind::Formula | SemanticObjectKind::Sequence))]
    pub fn owner(self) -> SemanticObjectId {
        match self.as_data() {
            data!(EventBindingScope::Formula { formula }) => *formula,
            data!(EventBindingScope::Sequence { sequence }) => *sequence,
        }
    }
}

#[invariant(::Structural(_) => true)]
#[invariant(::Referent(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticIdPrefix {
    Structural(SemanticObjectKind),
    Referent(SemanticSort),
}

pub type SemanticReferentId = SemanticObjectId;

impl SemanticIdPrefix {
    #[requires(true)]
    #[ensures(true)]
    fn object_kind(self) -> SemanticObjectKind {
        match self {
            Self::Structural(kind) => kind,
            Self::Referent(_) => SemanticObjectKind::Referent,
        }
    }
}

impl fmt::Display for SemanticIdPrefix {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structural(kind) => formatter.write_str(kind.id_prefix_label()),
            Self::Referent(sort) => formatter.write_str(sort.label()),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticObjectKind {
    Utterance,
    Sequence,
    Eventuality,
    Referent,
    Parameter,
    Predication,
    Formula,
    Abstraction,
    Sign,
    DisplayedContent,
    MathExpression,
    Quantity,
    RelationMetadata,
    Question,
}

impl SemanticObjectKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn id_prefix_label(self) -> &'static str {
        match self {
            Self::Utterance => "utterance",
            Self::Sequence => "sequence",
            Self::Eventuality => "eventuality",
            Self::Referent => "referent",
            Self::Parameter => "parameter",
            Self::Predication => "predication",
            Self::Formula => "formula",
            Self::Abstraction => "abstraction",
            Self::Sign => "sign",
            Self::DisplayedContent => "display",
            Self::MathExpression => "math",
            Self::Quantity => "quantity",
            Self::RelationMetadata => "relationMetadata",
            Self::Question => "question",
        }
    }
}

#[invariant(*version == SEMANTIC_JSON_VERSION)]
#[invariant(objects.contains_key(root))]
#[expensive_invariant(semantic_object_ids_match_types(objects))]
#[expensive_invariant(semantic_object_references_are_defined(objects))]
#[expensive_invariant(semantic_object_references_match_roles(objects))]
#[expensive_invariant(semantic_object_arguments_are_valid(objects))]
#[expensive_invariant(semantic_object_compositions_are_valid(objects))]
#[expensive_invariant(semantic_object_question_slots_are_valid(objects))]
#[expensive_invariant(semantic_object_domain_imports_are_valid(objects))]
#[expensive_invariant(semantic_event_bindings_are_derived(*root, objects))]
#[expensive_invariant(semantic_object_scope_dependences_are_derived(*root, objects))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGraph {
    pub version: &'static str,
    pub root: SemanticObjectId,
    #[serde(serialize_with = "serialize_objects")]
    pub objects: BTreeMap<SemanticObjectId, SemanticObject>,
}

#[invariant(::ObjectIdTypeMismatch(message) => !message.is_empty())]
#[invariant(::UndefinedReference { source, missing } => source != missing)]
#[invariant(::InvalidEventBindings(message) => !message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticGraphError {
    ObjectIdTypeMismatch(String),
    UndefinedReference {
        source: SemanticObjectId,
        missing: SemanticObjectId,
    },
    ReferenceRoleMismatch,
    InvalidArguments,
    InvalidCompositions,
    InvalidQuestionSlots,
    InvalidEventBindings(String),
}

impl fmt::Display for SemanticGraphError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(SemanticGraphError::ObjectIdTypeMismatch(message)) => {
                write!(
                    formatter,
                    "semantic object ID prefixes must match object types: {message}"
                )
            }
            data!(SemanticGraphError::UndefinedReference { source, missing }) => {
                write!(
                    formatter,
                    "semantic object references must not dangle: {source} references missing {missing}"
                )
            }
            data!(SemanticGraphError::ReferenceRoleMismatch) => {
                formatter.write_str("semantic object references must match semantic roles")
            }
            data!(SemanticGraphError::InvalidArguments) => formatter.write_str(
                "semantic arguments must use valid numbered places and argument fillers",
            ),
            data!(SemanticGraphError::InvalidCompositions) => {
                formatter.write_str("semantic compositions must use coherent parameters")
            }
            data!(SemanticGraphError::InvalidQuestionSlots) => {
                formatter.write_str("semantic question slots must use coherent parameters")
            }
            data!(SemanticGraphError::InvalidEventBindings(message)) => {
                write!(
                    formatter,
                    "generated eventualities require valid scope bindings: {message}"
                )
            }
        }
    }
}

impl std::error::Error for SemanticGraphError {}

impl SemanticGraph {
    #[requires(objects.contains_key(&root))]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| graph.root == root))]
    #[expensive_ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| semantic_event_bindings_are_derived(graph.root, &graph.objects)))]
    #[expensive_ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| semantic_object_scope_dependences_are_derived(graph.root, &graph.objects)))]
    pub fn new(
        root: SemanticObjectId,
        mut objects: BTreeMap<SemanticObjectId, SemanticObject>,
    ) -> Result<Self, SemanticGraphError> {
        if let Some(mismatch) = first_semantic_object_id_type_mismatch(&objects) {
            return Err(new!(SemanticGraphError::ObjectIdTypeMismatch(mismatch)));
        }
        if let Some((source, missing)) = first_undefined_semantic_reference(&objects) {
            return Err(new!(SemanticGraphError::UndefinedReference {
                source,
                missing,
            }));
        }
        if !semantic_object_references_match_roles(&objects) {
            return Err(new!(SemanticGraphError::ReferenceRoleMismatch));
        }
        if !semantic_object_arguments_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidArguments));
        }
        if !semantic_object_compositions_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidCompositions));
        }
        if !semantic_object_question_slots_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidQuestionSlots));
        }
        apply_semantic_event_bindings(root, &mut objects)
            .map_err(|message| new!(SemanticGraphError::InvalidEventBindings(message)))?;
        apply_semantic_scope_dependence(root, &mut objects);
        Ok(new!(SemanticGraph {
            version: SEMANTIC_JSON_VERSION,
            root: root,
            objects: objects,
        }))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
    pub fn to_json_string(&self, indent: usize) -> Result<String, serde_json::Error> {
        if indent == 0 {
            serde_json::to_string(self)
        } else {
            let mut buffer = Vec::new();
            let indent_bytes = vec![b' '; indent];
            let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
            let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
            self.serialize(&mut serializer)?;
            String::from_utf8(buffer)
                .map_err(|error| serde_json::Error::io(std::io::Error::other(error)))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn serialize_objects<S>(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(objects.len()))?;
    for (id, object) in objects {
        map.serialize_entry(&id.to_string(), object)?;
    }
    map.end()
}

#[requires(true)]
#[ensures(ret == !*value)]
fn bool_is_false(value: &bool) -> bool {
    !*value
}

mod event_binding;
mod scope_dependence;
mod semantic_object;

pub(crate) use event_binding::apply_semantic_event_bindings;
pub use event_binding::semantic_event_bindings_are_derived;
pub(crate) use scope_dependence::apply_semantic_scope_dependence;
pub use scope_dependence::semantic_object_scope_dependences_are_derived;
pub use semantic_object::*;

#[requires(true)]
#[ensures(true)]
fn extend_optional(out: &mut Vec<SemanticObjectId>, value: Option<SemanticObjectId>) {
    if let Some(value) = value {
        out.push(value);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSource {
    pub span: SourceByteSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construct: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceByteSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

impl SourceByteSpan {
    #[requires(span.byte_start <= span.byte_end)]
    #[ensures(ret.byte_start == span.byte_start)]
    pub fn from_source_span(span: &SourceSpan) -> Self {
        Self {
            byte_start: span.byte_start,
            byte_end: span.byte_end,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

impl SemanticDiagnostic {
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    pub fn warning(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            severity: DiagnosticSeverity::Warning,
            message,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UtteranceForce {
    Assert,
    Ask,
    Command,
    Mention,
    Quote,
    Parenthetical,
    Subordinated,
    Vocative,
}

#[invariant(::SameTopicContinuation => true)]
#[invariant(::ParagraphBoundary { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceRelation {
    SameTopicContinuation,
    ParagraphBoundary {
        transition: ParagraphTransition,
        additional: Vec<ParagraphTransition>,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParagraphTransition {
    NewTopic,
    ResumePriorTopic,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElidedConnectionOperand {
    PriorDiscourse,
    FollowingDiscourse,
}

#[invariant(!operator.is_empty(), "nonlogical sequence operator must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonlogicalConnection {
    pub operator: String,
    pub connector: Connector,
}

impl NonlogicalConnection {
    #[requires(!operator.is_empty())]
    #[ensures(ret.operator == old(operator.clone()))]
    pub fn new(operator: String, connector: Connector) -> Self {
        Self::from_data(data!(NonlogicalConnection {
            operator,
            connector,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.connector.references_into(out);
    }
}

#[invariant(value.object_kind() == SemanticObjectKind::MathExpression, "ordinal label value must be a math expression")]
#[invariant(target.is_none_or(|target| {
    matches!(
        target.object_kind(),
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Formula
            | SemanticObjectKind::Referent
            | SemanticObjectKind::DisplayedContent
    )
}), "ordinal labels target discourse-visible objects")]
#[invariant(!introduced_by.is_empty(), "ordinal label source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrdinalLabel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SemanticObjectId>,
    pub level: OrdinalLabelLevel,
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl OrdinalLabel {
    #[requires(value.object_kind() == SemanticObjectKind::MathExpression)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        target: Option<SemanticObjectId>,
        level: OrdinalLabelLevel,
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(OrdinalLabel {
            target,
            level,
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.target);
        out.push(self.value);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrdinalLabelLevel {
    Item,
    Division,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventualityClass {
    Locution,
    Event,
    State,
    Process,
    Activity,
    Achievement,
}

impl EventualityClass {
    #[requires(true)]
    #[ensures(ret.is_subsort_of(SemanticSort::eventuality()))]
    pub fn sort(self) -> SemanticSort {
        SemanticSort::Eventuality(match self {
            Self::Locution => EventualitySort::Locution,
            Self::Event => EventualitySort::General,
            Self::State => EventualitySort::State,
            Self::Process => EventualitySort::Process,
            Self::Activity => EventualitySort::Activity,
            Self::Achievement => EventualitySort::Achievement,
        })
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Actuality {
    pub kind: ActualityKind,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActualityKind {
    Actual,
    Capable,
    Potential,
    Demonstrated,
}

#[invariant(!relation.is_empty(), "anchor relation must be named")]
#[invariant(argument_object_kind_can_fill(anchor.object_kind()), "anchor must be referent-like")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "anchor relation distance must be named when present")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRelation {
    pub relation: String,
    pub anchor: SemanticObjectId,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub sticky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<AnchorMagnitude>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<SpatialMotion>,
}

impl AnchorRelation {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.anchor);
        if let Some(magnitude) = &self.magnitude {
            magnitude.references_into(out);
        }
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(argument_object_kind_can_fill(value.object_kind()), "anchor magnitude value must be referent-like")]
#[invariant(!introduced_by.is_empty(), "anchor magnitude source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorMagnitude {
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl AnchorMagnitude {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(AnchorMagnitude {
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.value);
    }
}

#[invariant(!introduced_by.is_empty(), "spatial motion source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialMotion {
    pub kind: SpatialMotionKind,
    pub introduced_by: String,
}

impl SpatialMotion {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: SpatialMotionKind, introduced_by: String) -> Self {
        Self::from_data(data!(SpatialMotion {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpatialMotionKind {
    Toward,
}

#[invariant(!relation.is_empty(), "temporal path relation must be named")]
#[invariant(!introduced_by.is_empty(), "temporal path source marker must be named")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "temporal path distance must be named when present")]
#[invariant(anchor.object_id().is_none_or(|id| argument_object_kind_can_fill(id.object_kind())), "temporal path object anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalPathStep {
    pub relation: String,
    pub anchor: TemporalPathAnchor,
    pub introduced_by: String,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub sticky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<AnchorMagnitude>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<SpatialMotion>,
}

impl TemporalPathStep {
    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
    #[requires(anchor.object_id().is_none_or(|id| argument_object_kind_can_fill(id.object_kind())))]
    #[ensures(ret.relation == old(relation.clone()))]
    pub fn new(
        relation: String,
        anchor: TemporalPathAnchor,
        introduced_by: String,
        distance: Option<String>,
        magnitude: Option<AnchorMagnitude>,
        scalar_negation: Option<ScalarNegation>,
        motion: Option<SpatialMotion>,
    ) -> Self {
        Self::from_data(data!(TemporalPathStep {
            relation,
            anchor,
            introduced_by,
            sticky: false,
            inherited: None,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        if let Some(anchor) = self.anchor.object_id() {
            out.push(anchor);
        }
        if let Some(magnitude) = &self.magnitude {
            magnitude.references_into(out);
        }
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant((*kind == TemporalPathAnchorKind::Object) == value.is_some(), "object anchors carry a value and non-object anchors do not")]
#[invariant(value.is_none_or(|value| argument_object_kind_can_fill(value.object_kind())), "temporal path object anchor must be referent-like")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalPathAnchor {
    pub kind: TemporalPathAnchorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<SemanticObjectId>,
}

impl TemporalPathAnchor {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[ensures(ret.object_id() == Some(value))]
    pub fn object(value: SemanticObjectId) -> Self {
        Self::from_data(data!(TemporalPathAnchor {
            kind: TemporalPathAnchorKind::Object,
            value: Some(value),
        }))
    }

    #[requires(true)]
    #[ensures(ret.object_id().is_none())]
    pub fn previous() -> Self {
        Self::from_data(data!(TemporalPathAnchor {
            kind: TemporalPathAnchorKind::Previous,
            value: None,
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| argument_object_kind_can_fill(id.object_kind())))]
    pub fn object_id(&self) -> Option<SemanticObjectId> {
        self.value
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TemporalPathAnchorKind {
    Object,
    Previous,
}

#[invariant(!extent.is_empty(), "time interval extent must be named")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "time interval anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeInterval {
    pub extent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
}

impl TimeInterval {
    #[requires(!extent.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.extent == old(extent.clone()))]
    pub fn new(extent: String, anchor: Option<SemanticObjectId>) -> Self {
        Self::from_data(data!(TimeInterval { extent, anchor }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(!introduced_by.is_empty(), "time span introducer must be recorded")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSpan {
    pub start: TimeSpanEndpoint,
    pub end: TimeSpanEndpoint,
    pub introduced_by: String,
}

impl TimeSpan {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(start: TimeSpanEndpoint, end: TimeSpanEndpoint, introduced_by: String) -> Self {
        Self::from_data(data!(TimeSpan {
            start,
            end,
            introduced_by,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.start.references_into(out);
        self.end.references_into(out);
    }
}

#[invariant(!relation.is_empty(), "time span endpoint relation must be named")]
#[invariant(!introduced_by.is_empty(), "time span endpoint introducer must be recorded")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "time span endpoint anchor must be referent-like")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "time span endpoint distance must be named when present")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSpanEndpoint {
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
}

impl TimeSpanEndpoint {
    #[requires(!relation.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[requires(!introduced_by.is_empty())]
    #[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
    #[ensures(ret.relation == old(relation.clone()))]
    pub fn new(
        relation: String,
        anchor: Option<SemanticObjectId>,
        introduced_by: String,
        distance: Option<String>,
        scalar_negation: Option<ScalarNegation>,
    ) -> Self {
        Self::from_data(data!(TimeSpanEndpoint {
            relation,
            anchor,
            introduced_by,
            distance,
            scalar_negation,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(extent.as_ref().is_none_or(|extent| !extent.is_empty()), "space interval extent must be named when present")]
#[invariant(directions.iter().all(|direction| !direction.is_empty()), "space interval directions must be named")]
#[invariant(dimensions.iter().all(|dimension| !dimension.is_empty()), "space interval dimensions must be named")]
#[invariant(extent.is_some() || !directions.is_empty() || !dimensions.is_empty(), "space interval must carry at least one spatial attribute")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "space interval anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInterval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
}

impl SpaceInterval {
    #[requires(extent.as_ref().is_none_or(|extent| !extent.is_empty()))]
    #[requires(directions.iter().all(|direction| !direction.is_empty()))]
    #[requires(dimensions.iter().all(|dimension| !dimension.is_empty()))]
    #[requires(extent.is_some() || !directions.is_empty() || !dimensions.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.extent == old(extent.clone()))]
    pub fn new(
        extent: Option<String>,
        directions: Vec<String>,
        dimensions: Vec<String>,
        anchor: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(SpaceInterval {
            extent,
            directions,
            dimensions,
            anchor,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(!contour.is_empty(), "aspect contour must be named")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "aspect anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aspect {
    pub contour: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
}

impl Aspect {
    #[requires(!contour.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.contour == old(contour.clone()))]
    pub fn new(contour: String, anchor: Option<SemanticObjectId>) -> Self {
        Self::new_with_polarity(contour, anchor, None)
    }

    #[requires(!contour.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.contour == old(contour.clone()))]
    pub fn new_with_polarity(
        contour: String,
        anchor: Option<SemanticObjectId>,
        scalar_negation: Option<ScalarNegation>,
    ) -> Self {
        Self::from_data(data!(Aspect {
            contour,
            anchor,
            scalar_negation,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(!introduced_by.is_empty(), "recurrence marker must be named")]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity), "recurrence quantity must be a quantity object")]
#[invariant(interval.is_none_or(|interval| argument_object_kind_can_fill(interval.object_kind())), "recurrence interval must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    pub kind: RecurrenceKind,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<RecurrenceConnection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<QuantityValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negation: Option<ModalNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl Recurrence {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(
        kind: RecurrenceKind,
        introduced_by: String,
        connection: Option<RecurrenceConnection>,
        value: Option<QuantityValue>,
        interval: Option<SemanticObjectId>,
        negation: Option<ModalNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Recurrence {
            kind,
            introduced_by,
            connection,
            quantity: None,
            value,
            interval,
            negation,
            source,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(ret.quantity == Some(quantity))]
    pub fn new_with_quantity(
        kind: RecurrenceKind,
        introduced_by: String,
        connection: Option<RecurrenceConnection>,
        quantity: SemanticObjectId,
        interval: Option<SemanticObjectId>,
        negation: Option<ModalNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Recurrence {
            kind,
            introduced_by,
            connection,
            quantity: Some(quantity),
            value: None,
            interval,
            negation,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.quantity);
        if let Some(value) = &self.value {
            value.references_into(out);
        }
        extend_optional(out, self.interval);
    }
}

#[invariant(::Aspect(aspect) => !aspect.contour.is_empty(), "aspect interval modifier must carry a named contour")]
#[invariant(::Recurrence(recurrence) => !recurrence.introduced_by.is_empty(), "recurrence interval modifier must carry its source marker")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum IntervalModifier {
    Aspect(Aspect),
    Recurrence(Recurrence),
}

impl IntervalModifier {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        match self.as_data() {
            data!(IntervalModifier::Aspect(aspect)) => aspect.references_into(out),
            data!(IntervalModifier::Recurrence(recurrence)) => recurrence.references_into(out),
        }
    }
}

#[invariant(!introduced_by.is_empty(), "recurrence connection source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceConnection {
    pub kind: RecurrenceConnectionKind,
    pub introduced_by: String,
}

impl RecurrenceConnection {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: RecurrenceConnectionKind, introduced_by: String) -> Self {
        Self::from_data(data!(RecurrenceConnection {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceConnectionKind {
    Product,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceKind {
    OccurrenceCount,
    OrdinalOccurrence,
    Regular,
    Typically,
    Continuously,
    Habitually,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeicticGround {
    pub time: SemanticObjectId,
    pub place: SemanticObjectId,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferentCategory {
    Constant,
    Variable,
    Indexical,
    Composite,
}

/// Whether a constant referent's denotation can co-vary with enclosing binders.
///
/// `Underspecified` records only the binders that the denotation may depend on.
/// It does not assert that any such dependence actually exists.
#[invariant(::Fixed => true, "the unit fixed state has no invalid representation")]
#[invariant(::Underspecified { may_depend_on } => !may_depend_on.is_empty() && may_depend_on.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())), "underspecified dependence names one or more binder-capable objects")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScopeDependence {
    Fixed,
    Underspecified {
        #[serde(rename = "mayDependOn")]
        may_depend_on: BTreeSet<SemanticObjectId>,
    },
}

impl ScopeDependence {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(ScopeDependence::Fixed)))]
    pub fn fixed() -> Self {
        new!(ScopeDependence::Fixed)
    }

    #[requires(!may_depend_on.is_empty())]
    #[requires(may_depend_on.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
    #[ensures(ret.may_depend_on().is_some_and(|derived| derived == &old(may_depend_on.clone())))]
    pub fn underspecified(may_depend_on: BTreeSet<SemanticObjectId>) -> Self {
        new!(ScopeDependence::Underspecified { may_depend_on })
    }

    #[requires(true)]
    #[ensures(ret.is_none() == matches!(self.as_data(), data!(ScopeDependence::Fixed)))]
    pub fn may_depend_on(&self) -> Option<&BTreeSet<SemanticObjectId>> {
        match self.as_data() {
            data!(ScopeDependence::Fixed) => None,
            data!(ScopeDependence::Underspecified { may_depend_on }) => Some(may_depend_on),
        }
    }
}

/// Whether an eventuality is introduced by generated predication semantics or denotes a
/// referential Lojban sumti/discourse object.
///
/// Generated eventualities are bound structurally and therefore carry neither a referent
/// category nor `ScopeDependence`. Referential eventualities retain the same category and
/// constant-dependence model as other referents.
#[invariant(::GeneratedBound => true, "the unit generated-bound identity has no invalid payload")]
#[invariant(::Referential { category, scope_dependence } => (*category == ReferentCategory::Constant) == scope_dependence.is_some())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventualityDenotation {
    GeneratedBound,
    Referential {
        category: ReferentCategory,
        scope_dependence: Option<ScopeDependence>,
    },
}

impl EventualityDenotation {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(EventualityDenotation::GeneratedBound)))]
    pub fn generated_bound() -> Self {
        new!(EventualityDenotation::GeneratedBound)
    }

    #[requires(true)]
    #[ensures(ret.category() == Some(category))]
    pub fn referential(category: ReferentCategory) -> Self {
        let scope_dependence =
            (category == ReferentCategory::Constant).then(ScopeDependence::fixed);
        new!(EventualityDenotation::Referential {
            category,
            scope_dependence,
        })
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(EventualityDenotation::GeneratedBound)))]
    pub fn is_generated_bound(&self) -> bool {
        matches!(self.as_data(), data!(EventualityDenotation::GeneratedBound))
    }

    #[requires(true)]
    #[ensures(ret.is_none() == self.is_generated_bound())]
    pub fn category(&self) -> Option<ReferentCategory> {
        match self.as_data() {
            data!(EventualityDenotation::GeneratedBound) => None,
            data!(EventualityDenotation::Referential { category, .. }) => Some(*category),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (self.category() == Some(ReferentCategory::Constant)))]
    pub fn scope_dependence(&self) -> Option<&ScopeDependence> {
        match self.as_data() {
            data!(EventualityDenotation::GeneratedBound) => None,
            data!(EventualityDenotation::Referential {
                scope_dependence,
                ..
            }) => scope_dependence.as_ref(),
        }
    }

    #[requires(self.category() == Some(ReferentCategory::Constant))]
    #[ensures(ret.scope_dependence().is_some_and(|stored| stored == &old(scope_dependence.clone())))]
    pub(crate) fn with_scope_dependence(self, scope_dependence: ScopeDependence) -> Self {
        match self.into_data() {
            data!(EventualityDenotation::Referential { category, .. }) => {
                new!(EventualityDenotation::Referential {
                    category,
                    scope_dependence: Some(scope_dependence),
                })
            }
            data!(EventualityDenotation::GeneratedBound) => {
                unreachable!("precondition excludes generated-bound eventualities")
            }
        }
    }
}

impl Serialize for EventualityDenotation {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(if self.is_generated_bound() {
            "generated-bound"
        } else {
            "referential"
        })
    }
}

#[invariant(::Eventuality(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticSort {
    Entity,
    Mass,
    Set,
    Sequence,
    Time,
    Eventuality(EventualitySort),
    Predication,
    TruthValue,
    Proposition,
    Concept,
    Amount,
    Quantity,
    Number,
    Scale,
    Text,
    Sign,
    Relation,
    Place,
    Connective,
    TenseModal,
    MathOperator,
    ArgumentBundle,
    AbstractNature,
}

impl SemanticSort {
    #[requires(true)]
    #[ensures(ret == SemanticSort::Eventuality(EventualitySort::General))]
    pub fn eventuality() -> Self {
        Self::Eventuality(EventualitySort::General)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Mass => "mass",
            Self::Set => "set",
            Self::Sequence => "sequence",
            Self::Time => "time",
            Self::Eventuality(sort) => sort.label(),
            Self::Predication => "predication",
            Self::TruthValue => "truthValue",
            Self::Proposition => "proposition",
            Self::Concept => "concept",
            Self::Amount => "amount",
            Self::Quantity => "quantity",
            Self::Number => "number",
            Self::Scale => "scale",
            Self::Text => "text",
            Self::Sign => "sign",
            Self::Relation => "relation",
            Self::Place => "place",
            Self::Connective => "connective",
            Self::TenseModal => "tenseModal",
            Self::MathOperator => "mathOperator",
            Self::ArgumentBundle => "argumentBundle",
            Self::AbstractNature => "abstractNature",
        }
    }

    #[requires(true)]
    #[ensures(ret || self != required)]
    pub fn is_subsort_of(self, required: Self) -> bool {
        self == required
            || matches!(
                (self, required),
                (
                    Self::Eventuality(_),
                    Self::Eventuality(EventualitySort::General)
                )
            )
    }
}

impl Serialize for SemanticSort {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

impl fmt::Display for SemanticSort {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventualitySort {
    General,
    State,
    Process,
    Activity,
    Achievement,
    Experience,
    Locution,
}

impl EventualitySort {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "eventuality",
            Self::State => "eventuality/state",
            Self::Process => "eventuality/process",
            Self::Activity => "eventuality/activity",
            Self::Achievement => "eventuality/achievement",
            Self::Experience => "eventuality/experience",
            Self::Locution => "eventuality/locution",
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexicalKind {
    Speaker,
    Audience,
    Now,
    Here,
    ProximalDemonstrative,
    MedialDemonstrative,
    DistalDemonstrative,
}

#[invariant(!word.is_empty() || *kind == DescriptorKind::Description, "only bare descriptions may omit a descriptor word")]
#[invariant(speaker.is_none_or(|speaker| speaker.object_kind() == SemanticObjectKind::Referent))]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[invariant(scale.is_none_or(|scale| scale.object_kind() == SemanticObjectKind::Referent))]
#[invariant(operand.is_none_or(|operand| argument_object_kind_can_fill(operand.object_kind())))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub kind: DescriptorKind,
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veridical: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definiteness: Option<DescriptorDefiniteness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<SemanticObjectId>,
}

impl Descriptor {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.speaker);
        extend_optional(out, self.body);
        out.extend(self.relative_clauses.iter().map(|clause| clause.body));
        extend_optional(out, self.quantity);
        extend_optional(out, self.scale);
        extend_optional(out, self.operand);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DescriptorKind {
    Number,
    Name,
    MassName,
    SetName,
    SpeakerDescription,
    Scale,
    ProSumti,
    UnloweredSumti,
    Description,
    VeridicalDescription,
    VeridicalMassDescription,
    VeridicalSetDescription,
    SpeakerMassDescription,
    SpeakerSetDescription,
    SpeakerStereotypeDescription,
    MassNameDescription,
    SetNameDescription,
    TypicalDescription,
    TypicalPlaceValue,
    UtteranceReference,
    Elided,
    AbstractionAbout,
    ReferentOfSymbol,
    SymbolForReferent,
    MemberOf,
    SetFrom,
    MassFrom,
    SequenceFrom,
    QualifiedSumti,
    OppositeOf,
    NeutralOf,
    AffirmedAs,
    OtherThan,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DescriptorDefiniteness {
    AffirmedPoint,
    IndefiniteAlternative,
    NeutralPoint,
    UniqueExtreme,
}

#[invariant(members.iter().all(|member| argument_object_kind_can_fill(member.object_kind())), "composition members must be semantic objects that can fill an argument")]
#[invariant(excluded_members.iter().all(|member| argument_object_kind_can_fill(member.object_kind())), "excluded composition members must be semantic objects that can fill an argument")]
#[invariant(endpoint_inclusion.is_none() || operator.is_interval(), "endpoint inclusion only applies to interval compositions")]
#[invariant(*complement != Some(true) || operator.is_interval(), "composition complements are interval complements")]
#[invariant((*operator == CompositionOperator::ConnectiveQuestion) == operator_parameter.is_some(), "connective-question compositions must carry exactly one operator parameter")]
#[invariant(operator_parameter.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter), "composition operator parameter must be a parameter object")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    pub operator: CompositionOperator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_parameter: Option<SemanticObjectId>,
    pub members: Vec<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_members: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_inclusion: Option<IntervalEndpointInclusion>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompositionOperator {
    ConnectiveQuestion,
    Joint,
    Mass,
    Set,
    Sequence,
    Respectively,
    Union,
    Intersection,
    CrossProduct,
    UnorderedInterval,
    OrderedInterval,
    CenteredInterval,
}

impl CompositionOperator {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::ConnectiveQuestion => "connectiveQuestion",
            Self::Joint => "joint",
            Self::Mass => "mass",
            Self::Set => "set",
            Self::Sequence => "sequence",
            Self::Respectively => "respectively",
            Self::Union => "union",
            Self::Intersection => "intersection",
            Self::CrossProduct => "crossProduct",
            Self::UnorderedInterval => "unorderedInterval",
            Self::OrderedInterval => "orderedInterval",
            Self::CenteredInterval => "centeredInterval",
        }
    }

    #[requires(true)]
    #[ensures(ret == matches!(self, Self::UnorderedInterval | Self::OrderedInterval | Self::CenteredInterval))]
    pub fn is_interval(self) -> bool {
        matches!(
            self,
            Self::UnorderedInterval | Self::OrderedInterval | Self::CenteredInterval
        )
    }

    #[requires(true)]
    #[ensures(ret == (self == Self::Mass))]
    pub fn is_mass(self) -> bool {
        self == Self::Mass
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntervalEndpointInclusion {
    pub left: EndpointInclusion,
    pub right: EndpointInclusion,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointInclusion {
    Inclusive,
    Exclusive,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterRole {
    PropertySlot,
    RelativeClauseHead,
    ArgumentQuestion,
    RelationQuestion,
    RelationVariable,
    UnspecifiedRelation,
    PlaceQuestion,
    ConnectiveQuestion,
    TenseQuestion,
    MathOperatorQuestion,
    QuantityQuestion,
    AttitudeQuestion,
    RespectiveSlot,
}

#[invariant(variable.object_kind() == SemanticObjectKind::Referent)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionSource {
    pub kind: SelectionSourceKind,
    pub variable: SemanticObjectId,
}

impl SelectionSource {
    #[requires(variable.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.variable == variable)]
    pub fn witness_set(variable: SemanticObjectId) -> Self {
        Self::from_data(data!(SelectionSource {
            kind: SelectionSourceKind::WitnessSet,
            variable,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.variable);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionSourceKind {
    WitnessSet,
}

#[invariant(argument_value_shape_is_valid(*kind, *value, introduced_by.as_deref()))]
#[invariant(*kind != ArgumentValueKind::Deleted || relative_clauses.is_empty())]
#[invariant(*kind != ArgumentValueKind::Deleted || quantity.is_none())]
#[invariant(*kind != ArgumentValueKind::Deleted || command_target.is_none())]
#[invariant((*quantity).is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentValue {
    pub kind: ArgumentValueKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_target: Option<CommandTarget>,
}

impl ArgumentValue {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[ensures(true)]
    pub fn filled(value: SemanticObjectId, source: Option<SemanticSource>) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Filled,
            value: Some(value),
            quantity: None,
            introduced_by: None,
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[requires(!introduced_by.is_empty())]
    #[ensures(true)]
    pub fn elided(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Elided,
            value: Some(value),
            quantity: None,
            introduced_by: Some(introduced_by),
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(true)]
    pub fn deleted(introduced_by: String, source: Option<SemanticSource>) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: None,
            quantity: None,
            introduced_by: Some(introduced_by),
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[requires(!relative_clauses.is_empty())]
    #[ensures(!ret.relative_clauses.is_empty())]
    pub fn with_relative_clauses(self, relative_clauses: Vec<RelativeClause>) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            relative_clauses,
            ..data
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(ret.quantity == Some(quantity))]
    pub fn with_quantity(self, quantity: SemanticObjectId) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            quantity: Some(quantity),
            ..data
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[ensures(ret.command_target.is_some())]
    pub fn with_command_target(self, command_target: CommandTarget) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            command_target: Some(command_target),
            ..data
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        if let Some(value) = self.value {
            out.push(value);
        }
        extend_optional(out, self.quantity);
        out.extend(self.relative_clauses.iter().map(|clause| clause.body));
    }
}

#[invariant(!introduced_by.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTarget {
    pub introduced_by: String,
}

impl CommandTarget {
    #[requires(!introduced_by.is_empty())]
    #[ensures(!ret.introduced_by.is_empty())]
    pub fn new(introduced_by: String) -> Self {
        Self::from_data(data!(CommandTarget { introduced_by }))
    }
}

#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(introduced_by.as_ref().is_none_or(|introduced_by| !introduced_by.is_empty()))]
#[invariant(!matches!(*veridical, Some(true)), "true veridicality is omitted")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeClause {
    pub kind: RelativeClauseKind,
    pub body: SemanticObjectId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veridical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl RelativeClause {
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.body == body)]
    pub fn new(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: None,
            veridical: None,
            source
        }))
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.body == body)]
    pub fn with_introducer(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: Some(introduced_by),
            veridical: None,
            source
        }))
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.body == body)]
    pub fn nonveridical(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: Some(introduced_by),
            veridical: Some(false),
            source
        }))
    }
}

#[invariant(!name.is_empty(), "assigned names must preserve the assigned cmevla")]
#[invariant(!word.is_empty(), "assigned names must record the naming word")]
#[invariant(!introduced_by.is_empty(), "assigned names must record the assignment marker")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignedName {
    pub name: String,
    pub word: String,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RelativeClauseKind {
    Incidental,
    Restrictive,
}

#[invariant(parameter.object_kind() == SemanticObjectKind::Parameter)]
#[invariant(!candidate_places.is_empty(), "place questions must enumerate candidate places")]
#[invariant(candidate_places.iter().all(|place| place.get() > 0))]
#[invariant(candidate_places.iter().enumerate().all(|(index, place)| !candidate_places[..index].contains(place)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceQuestionBinding {
    pub parameter: SemanticObjectId,
    pub argument: ArgumentValue,
    pub candidate_places: Vec<PlaceIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl PlaceQuestionBinding {
    #[requires(parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!candidate_places.is_empty())]
    #[requires(candidate_places.iter().all(|place| place.get() > 0))]
    #[ensures(ret.parameter == parameter)]
    pub fn new(
        parameter: SemanticObjectId,
        argument: ArgumentValue,
        candidate_places: Vec<PlaceIndex>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(PlaceQuestionBinding {
            parameter,
            argument,
            candidate_places,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.parameter);
        self.argument.references_into(out);
    }
}

#[invariant(!introduced_by.is_empty(), "modal negation source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalNegation {
    pub kind: ModalNegationKind,
    pub introduced_by: String,
}

impl ModalNegation {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: ModalNegationKind, introduced_by: String) -> Self {
        Self::from_data(data!(ModalNegation {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModalNegationKind {
    Contradictory,
    OtherThan,
}

#[invariant(!introduced_by.is_empty(), "modal source marker must be named")]
#[invariant(relation.as_ref().is_none_or(|relation| !relation.is_empty()), "modal relation must be named when present")]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula), "modal body must be a formula")]
#[invariant(relation.is_some() != body.is_some(), "modal argument must use either relation arguments or a body formula")]
#[invariant(body.is_some() || !arguments.is_empty(), "modal relation must have at least one explicit place")]
#[invariant(body.is_none() || arguments.is_empty(), "modal body arguments are represented inside the body formula")]
#[invariant(arguments.keys().all(|place| place.get() > 0))]
#[invariant(component.is_none_or(|component| argument_object_kind_can_fill(component.object_kind())))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalArgument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    pub introduced_by: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<PlaceIndex, ArgumentValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negation: Option<ModalNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<DisplayedContentModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl ModalArgument {
    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(true)]
    pub fn new(
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::new_with_polarity(relation, introduced_by, arguments, None, None, source)
    }

    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(true)]
    pub fn new_with_polarity(
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        negation: Option<ModalNegation>,
        scalar_negation: Option<ScalarNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ModalArgument {
            relation: Some(relation),
            introduced_by,
            arguments,
            body: None,
            component: None,
            negation,
            scalar_negation,
            modifiers: Vec::new(),
            source,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.body == Some(body))]
    pub fn body(
        introduced_by: String,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ModalArgument {
            relation: None,
            introduced_by,
            arguments: BTreeMap::new(),
            body: Some(body),
            component: None,
            negation: None,
            scalar_negation: None,
            modifiers: Vec::new(),
            source,
        }))
    }

    #[requires(argument_object_kind_can_fill(component.object_kind()))]
    #[ensures(ret.component == Some(component))]
    pub fn with_component(self, component: SemanticObjectId) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ModalArgument {
            component: Some(component),
            ..data
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        for argument in self.arguments.values() {
            argument.references_into(out);
        }
        extend_optional(out, self.body);
        extend_optional(out, self.component);
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(!introduced_by.is_empty(), "reciprocity source marker must be named")]
#[invariant(left.kind != ArgumentValueKind::Deleted, "reciprocity participants must exist")]
#[invariant(right.kind != ArgumentValueKind::Deleted, "reciprocity participants must exist")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReciprocalExchange {
    pub left: ArgumentValue,
    pub right: ArgumentValue,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl ReciprocalExchange {
    #[requires(!introduced_by.is_empty())]
    #[requires(left.kind != ArgumentValueKind::Deleted)]
    #[requires(right.kind != ArgumentValueKind::Deleted)]
    #[ensures(true)]
    pub fn new(
        left: ArgumentValue,
        right: ArgumentValue,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ReciprocalExchange {
            left,
            right,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.left.references_into(out);
        self.right.references_into(out);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArgumentValueKind {
    Filled,
    Elided,
    Deleted,
}

#[requires(true)]
#[ensures(true)]
pub fn argument_object_kind_can_fill(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Referent | SemanticObjectKind::Parameter | SemanticObjectKind::Formula
    )
}

#[requires(true)]
#[ensures(true)]
pub fn displayed_content_target_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Formula
            | SemanticObjectKind::Question
            | SemanticObjectKind::Parameter
            | SemanticObjectKind::Referent
            | SemanticObjectKind::DisplayedContent
    )
}

#[requires(true)]
#[ensures(true)]
fn argument_value_shape_is_valid(
    kind: ArgumentValueKind,
    value: Option<SemanticObjectId>,
    introduced_by: Option<&str>,
) -> bool {
    let value_allowed =
        value.is_none_or(|value| argument_object_kind_can_fill(value.object_kind()));
    match kind {
        ArgumentValueKind::Filled => value_allowed && value.is_some() && introduced_by.is_none(),
        ArgumentValueKind::Elided => {
            value_allowed
                && value.is_some()
                && introduced_by.is_some_and(|introduced_by| !introduced_by.is_empty())
        }
        ArgumentValueKind::Deleted => {
            value.is_none() && introduced_by.is_some_and(|introduced_by| !introduced_by.is_empty())
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PredicationMode {
    Asserted,
    Definitional,
    Restrictive,
    Incidental,
    Displayed,
    Inert,
    Performative,
}

#[invariant(!introduced_by.is_empty(), "scalar negation source marker must be named")]
#[invariant(scale.is_none_or(|scale| scale.object_kind() == SemanticObjectKind::Referent), "scalar negation scale must be a referent")]
#[invariant(argument_scope.iter().all(|place| place.get() > 0), "scalar negation argument scope must use numbered argument places")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalarNegation {
    pub kind: ScalarNegationKind,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argument_scope: Vec<PlaceIndex>,
}

impl ScalarNegation {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: ScalarNegationKind, introduced_by: String) -> Self {
        Self::from_data(data!(ScalarNegation {
            kind,
            introduced_by,
            scale: None,
            argument_scope: Vec::new(),
        }))
    }

    #[requires(scale.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.scale == Some(scale))]
    pub fn with_scale(self, scale: SemanticObjectId) -> Self {
        self.with_data(data! { scale: Some(scale) })
    }

    #[requires(argument_scope.iter().all(|place| place.get() > 0))]
    #[ensures(ret.argument_scope == argument_scope)]
    pub fn with_argument_scope(self, argument_scope: Vec<PlaceIndex>) -> Self {
        self.with_data(data! { argument_scope: argument_scope.clone() })
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.scale);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScalarNegationKind {
    OtherThan,
    Opposite,
    Neutral,
    Affirmed,
}

#[invariant(::Formula(_) => true)]
#[invariant(::Math(operator) => !operator.label().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticOperator {
    Formula(FormulaOperator),
    Math(MathOperator),
}

impl SemanticOperator {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(SemanticOperator::Formula(_))))]
    fn formula(operator: FormulaOperator) -> Self {
        Self::from_data(data!(SemanticOperator::Formula(operator)))
    }

    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(SemanticOperator::Math(_))))]
    fn math(operator: MathOperator) -> Self {
        Self::from_data(data!(SemanticOperator::Math(operator)))
    }
}

impl Serialize for SemanticOperator {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.as_data() {
            data!(SemanticOperator::Formula(operator)) => operator.serialize(serializer),
            data!(SemanticOperator::Math(operator)) => operator.serialize(serializer),
        }
    }
}

#[invariant(::Named(label) => !label.is_empty() && MathOperator::known_label(label).is_none())]
#[invariant(::Add => self.label() == "add")]
#[invariant(::Multiply => self.label() == "multiply")]
#[invariant(::Power => self.label() == "power")]
#[invariant(::Subtract => self.label() == "subtract")]
#[invariant(::Divide => self.label() == "divide")]
#[invariant(::Base => self.label() == "base")]
#[invariant(::BoGroup => self.label() == "boGroup")]
#[invariant(::OperandGroup => self.label() == "operandGroup")]
#[invariant(::Array => self.label() == "array")]
#[invariant(::UnorderedInterval => self.label() == "unorderedInterval" && self.is_interval())]
#[invariant(::OrderedInterval => self.label() == "orderedInterval" && self.is_interval())]
#[invariant(::CenteredInterval => self.label() == "centeredInterval" && self.is_interval())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MathOperator {
    Add,
    Multiply,
    Power,
    Subtract,
    Divide,
    Base,
    BoGroup,
    OperandGroup,
    Array,
    UnorderedInterval,
    OrderedInterval,
    CenteredInterval,
    Named(String),
}

impl MathOperator {
    #[requires(!label.is_empty())]
    #[ensures(true)]
    pub fn from_label(label: String) -> Self {
        Self::known_label(&label)
            .unwrap_or_else(|| Self::from_data(data!(MathOperator::Named(label))))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn known_label(label: &str) -> Option<Self> {
        match label {
            "add" => Some(new!(MathOperator::Add)),
            "multiply" => Some(new!(MathOperator::Multiply)),
            "power" => Some(new!(MathOperator::Power)),
            "subtract" => Some(new!(MathOperator::Subtract)),
            "divide" => Some(new!(MathOperator::Divide)),
            "base" => Some(new!(MathOperator::Base)),
            "boGroup" => Some(new!(MathOperator::BoGroup)),
            "operandGroup" => Some(new!(MathOperator::OperandGroup)),
            "array" => Some(new!(MathOperator::Array)),
            "unorderedInterval" => Some(new!(MathOperator::UnorderedInterval)),
            "orderedInterval" => Some(new!(MathOperator::OrderedInterval)),
            "centeredInterval" => Some(new!(MathOperator::CenteredInterval)),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(&self) -> &str {
        match self.as_data() {
            data!(MathOperator::Add) => "add",
            data!(MathOperator::Multiply) => "multiply",
            data!(MathOperator::Power) => "power",
            data!(MathOperator::Subtract) => "subtract",
            data!(MathOperator::Divide) => "divide",
            data!(MathOperator::Base) => "base",
            data!(MathOperator::BoGroup) => "boGroup",
            data!(MathOperator::OperandGroup) => "operandGroup",
            data!(MathOperator::Array) => "array",
            data!(MathOperator::UnorderedInterval) => "unorderedInterval",
            data!(MathOperator::OrderedInterval) => "orderedInterval",
            data!(MathOperator::CenteredInterval) => "centeredInterval",
            data!(MathOperator::Named(label)) => label,
        }
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(MathOperator::UnorderedInterval) | data!(MathOperator::OrderedInterval) | data!(MathOperator::CenteredInterval)))]
    pub fn is_interval(&self) -> bool {
        matches!(
            self.as_data(),
            data!(MathOperator::UnorderedInterval)
                | data!(MathOperator::OrderedInterval)
                | data!(MathOperator::CenteredInterval)
        )
    }
}

impl Serialize for MathOperator {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FormulaOperator {
    Atom,
    Affirmed,
    Not,
    Scoped,
    And,
    Or,
    Implies,
    Iff,
    ExclusiveOr,
    WhetherOrNot,
    ConnectiveQuestion,
    Exists,
    Forall,
    None,
    Cardinality,
    PluralExists,
    PluralForall,
    QuantifierBundle,
    RespectivelyDistribution,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainImport {
    Projective,
}

#[invariant(quantifier_formula_operator_is_allowed(*operator))]
#[invariant(quantifier_variable_kind_is_allowed(variable.object_kind()))]
#[invariant(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantifierBinding {
    pub operator: FormulaOperator,
    pub variable: SemanticObjectId,
    #[serde(rename = "sourceVariable", skip_serializing_if = "Option::is_none")]
    pub source_variable: Option<SemanticObjectId>,
    #[serde(rename = "selectionSource", skip_serializing_if = "Option::is_none")]
    pub selection_source: Option<SelectionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl QuantifierBinding {
    #[requires(quantifier_formula_operator_is_allowed(operator))]
    #[requires(quantifier_variable_kind_is_allowed(variable.object_kind()))]
    #[requires(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
    #[ensures(ret.variable == variable)]
    pub fn new(
        operator: FormulaOperator,
        variable: SemanticObjectId,
        source_variable: Option<SemanticObjectId>,
        selection_source: Option<SelectionSource>,
        restriction: Option<SemanticObjectId>,
        quantity: Option<SemanticObjectId>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(QuantifierBinding {
            operator,
            variable,
            source_variable,
            selection_source,
            restriction,
            quantity,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.variable);
        extend_optional(out, self.source_variable);
        if let Some(selection_source) = &self.selection_source {
            selection_source.references_into(out);
        }
        extend_optional(out, self.restriction);
        extend_optional(out, self.quantity);
    }
}

#[invariant(!source.is_empty())]
#[invariant(!locus.is_empty())]
#[invariant(parameter.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    pub source: String,
    pub locus: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth_table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<SemanticObjectId>,
}

impl Connector {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.parameter);
    }
}

#[invariant(head.object_kind() == SemanticObjectKind::Predication)]
#[invariant(argument_object_kind_can_fill(modifier.object_kind()), "tanru modifier must be a semantic argument value")]
#[invariant(relation_label.is_displayable(), "tanru relation label must be displayable")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TanruLink {
    pub head: SemanticObjectId,
    pub modifier: SemanticObjectId,
    pub relation_label: RelationLabel,
}

impl TanruLink {
    #[requires(head.object_kind() == SemanticObjectKind::Predication)]
    #[requires(argument_object_kind_can_fill(modifier.object_kind()))]
    #[requires(relation_label.is_displayable())]
    #[ensures(ret.head == head)]
    pub fn new(
        head: SemanticObjectId,
        modifier: SemanticObjectId,
        relation_label: RelationLabel,
    ) -> Self {
        Self::from_data(data!(TanruLink {
            head,
            modifier,
            relation_label,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.head);
        out.push(self.modifier);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AbstractionKind {
    Event,
    Achievement,
    Process,
    Activity,
    State,
    Property,
    Amount,
    TruthValue,
    Proposition,
    SentenceSign,
    Concept,
    Experience,
    Unspecified,
}

impl AbstractionKind {
    #[requires(true)]
    #[ensures(true)]
    pub fn output_sort(self) -> SemanticSort {
        match self {
            Self::Event => SemanticSort::eventuality(),
            Self::Achievement => SemanticSort::Eventuality(EventualitySort::Achievement),
            Self::Process => SemanticSort::Eventuality(EventualitySort::Process),
            Self::Activity => SemanticSort::Eventuality(EventualitySort::Activity),
            Self::State => SemanticSort::Eventuality(EventualitySort::State),
            Self::Experience => SemanticSort::Eventuality(EventualitySort::Experience),
            Self::Property => SemanticSort::Relation,
            Self::Amount => SemanticSort::Amount,
            Self::TruthValue => SemanticSort::TruthValue,
            Self::Proposition => SemanticSort::Proposition,
            Self::SentenceSign => SemanticSort::Sign,
            Self::Concept => SemanticSort::Concept,
            Self::Unspecified => SemanticSort::AbstractNature,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignKind {
    Quotation,
    Letteral,
    MathExpression,
    Connective,
    Word,
    Text,
}

#[invariant(!source_words.is_empty(), "letteral units must preserve their source words")]
#[invariant(text.as_ref().is_none_or(|text| !text.is_empty()), "letteral unit display text must not be empty when present")]
#[invariant(value.as_ref().is_none_or(|value| !value.is_empty()), "letteral unit value must not be empty when present")]
#[invariant(modifier.as_ref().is_none_or(|modifier| !modifier.is_empty()), "letteral modifier must not be empty when present")]
#[invariant(bu_depth.is_none_or(|depth| depth > 0), "BU depth is recorded only when positive")]
#[invariant((*kind == LetteralUnitKind::Compound) == !parts.is_empty(), "only compound letterals have parts")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LetteralUnit {
    pub kind: LetteralUnitKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_words: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bu_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<LetteralUnit>,
}

impl LetteralUnit {
    #[requires(!source_words.is_empty())]
    #[requires(text.as_ref().is_none_or(|text| !text.is_empty()))]
    #[requires(value.as_ref().is_none_or(|value| !value.is_empty()))]
    #[requires(modifier.as_ref().is_none_or(|modifier| !modifier.is_empty()))]
    #[requires(bu_depth.is_none_or(|depth| depth > 0))]
    #[ensures(ret.kind == old(kind))]
    pub fn simple(
        kind: LetteralUnitKind,
        source_words: Vec<String>,
        text: Option<String>,
        value: Option<String>,
        modifier: Option<String>,
        bu_depth: Option<usize>,
    ) -> Self {
        Self::from_data(data!(LetteralUnit {
            kind,
            source_words,
            text,
            value,
            modifier,
            bu_depth,
            parts: Vec::new(),
        }))
    }

    #[requires(!source_words.is_empty())]
    #[requires(!parts.is_empty())]
    #[requires(value.as_ref().is_none_or(|value| !value.is_empty()))]
    #[ensures(ret.kind == LetteralUnitKind::Compound)]
    pub fn compound(
        source_words: Vec<String>,
        value: Option<String>,
        parts: Vec<LetteralUnit>,
    ) -> Self {
        Self::from_data(data!(LetteralUnit {
            kind: LetteralUnitKind::Compound,
            source_words,
            text: None,
            value,
            modifier: None,
            bu_depth: None,
            parts,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LetteralUnitKind {
    Glyph,
    Digit,
    Shift,
    CharacterCode,
    Compound,
}

#[invariant(!mode.is_empty())]
#[invariant(utterance.is_none_or(|utterance| utterance.object_kind() == SemanticObjectKind::Utterance))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quotation {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utterance: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Quotation {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.utterance);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentFamily {
    Emotion,
    AttitudeModifier,
    PropositionalAttitude,
    Evidential,
    Discursive,
    Metalinguistic,
    Emphasis,
    QuestionPrompt,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentPolarity {
    Positive,
    Neutral,
    Negative,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentTargetFocus {
    Bridi,
    Selbri,
}

#[invariant(!relation.is_empty(), "displayed-content modifier relation must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayedContentModifier {
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<DisplayedContentFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polarity: Option<DisplayedContentPolarity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,
    #[serde(rename = "assertionEffect", skip_serializing_if = "Option::is_none")]
    pub assertion_effect: Option<DisplayedContentAssertionEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentAssertionEffect {
    None,
    HostAsserted,
    HostSubordinated,
    MetalinguisticallyVoided,
    Performative,
}

#[invariant(value.object_kind() == SemanticObjectKind::MathExpression, "subscript value must be a math expression")]
#[invariant(!introduced_by.is_empty(), "subscript source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscript {
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl Subscript {
    #[requires(value.object_kind() == SemanticObjectKind::MathExpression)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Subscript {
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.value);
    }
}

#[invariant((*kind == MathLiteralKind::Integer) == matches!(value.as_data(), data!(MathLiteralValue::Integer(_))), "integer math literals must carry an integer value")]
#[invariant((*kind == MathLiteralKind::MixedRadix) == matches!(value.as_data(), data!(MathLiteralValue::MixedRadix(_))), "mixed-radix math literals must carry mixed-radix values")]
#[invariant(kind.is_text() == matches!(value.as_data(), data!(MathLiteralValue::Text(_))), "text math literal kinds must carry text values")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MathLiteral {
    pub kind: MathLiteralKind,
    pub value: MathLiteralValue,
}

impl MathLiteral {
    #[requires(true)]
    #[ensures(ret.kind == MathLiteralKind::Integer)]
    pub fn integer(value: i64) -> Self {
        Self::from_data(data!(MathLiteral {
            kind: MathLiteralKind::Integer,
            value: MathLiteralValue::from_data(data!(MathLiteralValue::Integer(value))),
        }))
    }

    #[requires(kind.is_text())]
    #[requires(!value.is_empty())]
    #[ensures(ret.kind == old(kind.clone()))]
    pub fn text(kind: MathLiteralKind, value: String) -> Self {
        Self::from_data(data!(MathLiteral {
            kind,
            value: MathLiteralValue::from_data(data!(MathLiteralValue::Text(value))),
        }))
    }

    #[requires(components.len() >= 2)]
    #[ensures(ret.kind == MathLiteralKind::MixedRadix)]
    pub fn mixed_radix(components: Vec<MixedRadixComponent>) -> Self {
        Self::from_data(data!(MathLiteral {
            kind: MathLiteralKind::MixedRadix,
            value: MathLiteralValue::from_data(data!(MathLiteralValue::MixedRadix(
                MixedRadixLiteral::from_data(data!(MixedRadixLiteral { components }))
            ))),
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MathLiteralKind {
    Integer,
    Decimal,
    Number,
    SumtiOperand,
    SelbriOperand,
    Expression,
    Variable,
    MixedRadix,
}

impl MathLiteralKind {
    #[requires(true)]
    #[ensures(ret == !matches!(self, Self::Integer | Self::MixedRadix))]
    pub fn is_text(self) -> bool {
        !matches!(self, Self::Integer | Self::MixedRadix)
    }
}

#[invariant(::Integer(_) => true)]
#[invariant(::Text(value) => !value.is_empty())]
#[invariant(::MixedRadix(value) => value.components.len() >= 2)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MathLiteralValue {
    Integer(i64),
    Text(String),
    MixedRadix(MixedRadixLiteral),
}

#[invariant(components.len() >= 2, "mixed-radix literals need at least two components")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixedRadixLiteral {
    pub components: Vec<MixedRadixComponent>,
}

#[invariant(!text.is_empty(), "mixed-radix component text must be preserved")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixedRadixComponent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer: Option<i64>,
}

impl MixedRadixComponent {
    #[requires(!text.is_empty())]
    #[ensures(ret.text == old(text.clone()))]
    pub fn new(text: String, integer: Option<i64>) -> Self {
        Self::from_data(data!(MixedRadixComponent { text, integer }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuantityForm {
    Exact,
    All,
    AtLeast,
    AtMost,
    MoreThan,
    LessThan,
    Approximate,
    Indefinite,
    Enough,
    TooMany,
    TooFew,
}

#[invariant((integer.is_some() as usize + text.is_some() as usize + math_expression.is_some() as usize) == 1)]
#[invariant(question_parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantityValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub math_expression: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub question_parameters: Vec<SemanticObjectId>,
}

impl QuantityValue {
    #[requires(true)]
    #[ensures(ret.integer == Some(integer))]
    pub fn integer(integer: i64) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: Some(integer),
            text: None,
            math_expression: None,
            question_parameters: Vec::new(),
        }))
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.text.is_some())]
    pub fn text(text: String) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: None,
            text: Some(text),
            math_expression: None,
            question_parameters: Vec::new(),
        }))
    }

    #[requires(math_expression.object_kind() == SemanticObjectKind::MathExpression)]
    #[ensures(ret.math_expression == Some(math_expression))]
    pub fn math_expression(math_expression: SemanticObjectId) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: None,
            text: None,
            math_expression: Some(math_expression),
            question_parameters: Vec::new(),
        }))
    }

    #[requires(!question_parameters.is_empty())]
    #[requires(question_parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[ensures(ret.question_parameters == old(question_parameters.clone()))]
    pub fn with_question_parameters(self, question_parameters: Vec<SemanticObjectId>) -> Self {
        self.with_data(data! { question_parameters: question_parameters })
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.math_expression);
        out.extend(self.question_parameters.iter().copied());
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuantityScale {
    Count,
    Fraction,
    Ordinal,
    Amount,
    Extent,
    Frequency,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceDescription {
    pub place: String,
    pub description: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationExpansion {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_words: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rafsi_bindings: Vec<RafsiBinding>,
}

impl RelationExpansion {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        for binding in &self.rafsi_bindings {
            binding.references_into(out);
        }
    }
}

#[invariant(!rafsi.is_empty(), "rafsi binding must preserve the source rafsi")]
#[invariant(source_word.as_ref().is_none_or(|word| !word.is_empty()))]
#[invariant(referent.is_none_or(|referent| referent.object_kind() == SemanticObjectKind::Referent))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RafsiBinding {
    pub rafsi: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referent: Option<SemanticObjectId>,
}

impl RafsiBinding {
    #[requires(!rafsi.is_empty())]
    #[requires(source_word.as_ref().is_none_or(|word| !word.is_empty()))]
    #[requires(referent.is_none_or(|referent| referent.object_kind() == SemanticObjectKind::Referent))]
    #[ensures(!ret.rafsi.is_empty())]
    pub fn new(
        rafsi: String,
        source_word: Option<String>,
        referent: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(RafsiBinding {
            rafsi,
            source_word,
            referent,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.referent);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionKind {
    Truth,
    Argument,
    Relation,
    Place,
    Connective,
    Tense,
    MathOperator,
    Attitude,
    Quantity,
    Multiple,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionMode {
    Direct,
    Indirect,
}

#[invariant(::Homogeneous { parameter, .. } => parameter.object_kind() == SemanticObjectKind::Parameter)]
#[invariant(::Typed { parameter, kind, domain, .. } => *kind != QuestionKind::Multiple && question_kind_domain_are_coherent(*kind, *domain) && (*kind == QuestionKind::Truth) == parameter.is_none() && parameter.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum QuestionSlot {
    /// A slot whose kind and domain are inherited from a homogeneous question.
    Homogeneous {
        parameter: SemanticObjectId,
        role: QuestionSlotRole,
    },
    /// A self-describing slot in a question with multiple answer domains.
    Typed {
        #[serde(skip_serializing_if = "Option::is_none")]
        parameter: Option<SemanticObjectId>,
        role: QuestionSlotRole,
        kind: QuestionKind,
        domain: SemanticSort,
    },
}

impl QuestionSlot {
    #[requires(parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[ensures(ret.parameter() == Some(parameter))]
    pub fn homogeneous(parameter: SemanticObjectId, role: QuestionSlotRole) -> Self {
        new!(QuestionSlot::Homogeneous { parameter, role })
    }

    #[requires(question_kind_domain_are_coherent(kind, domain))]
    #[requires(kind != QuestionKind::Multiple)]
    #[requires((kind == QuestionKind::Truth) == parameter.is_none())]
    #[requires(parameter.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[ensures(ret.kind_and_domain() == Some((kind, domain)))]
    pub fn typed(
        parameter: Option<SemanticObjectId>,
        role: QuestionSlotRole,
        kind: QuestionKind,
        domain: SemanticSort,
    ) -> Self {
        new!(QuestionSlot::Typed {
            parameter,
            role,
            kind,
            domain,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_none() == matches!(self.as_data(), data!(QuestionSlot::Typed { parameter: None, .. })))]
    pub fn parameter(&self) -> Option<SemanticObjectId> {
        match self.as_data() {
            data!(QuestionSlot::Homogeneous { parameter, .. }) => Some(*parameter),
            data!(QuestionSlot::Typed { parameter, .. }) => *parameter,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn role(&self) -> QuestionSlotRole {
        match self.as_data() {
            data!(QuestionSlot::Homogeneous { role, .. })
            | data!(QuestionSlot::Typed { role, .. }) => *role,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(QuestionSlot::Typed { .. })))]
    pub fn kind_and_domain(&self) -> Option<(QuestionKind, SemanticSort)> {
        match self.as_data() {
            data!(QuestionSlot::Homogeneous { .. }) => None,
            data!(QuestionSlot::Typed { kind, domain, .. }) => Some((*kind, *domain)),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionSlotRole {
    Answer,
    RespectiveSlot,
}

#[invariant(slot.object_kind() == SemanticObjectKind::Parameter)]
#[invariant(!items.is_empty(), "respectively stream cannot be empty")]
#[invariant(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespectivelyStream {
    pub slot: SemanticObjectId,
    pub items: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
}

impl RespectivelyStream {
    #[requires(slot.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!items.is_empty())]
    #[requires(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
    #[ensures(ret.items == old(items.clone()))]
    pub fn new(slot: SemanticObjectId, items: Vec<SemanticObjectId>) -> Self {
        Self::new_with_details(slot, items, None, None)
    }

    #[requires(slot.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!items.is_empty())]
    #[requires(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
    #[ensures(ret.slot == old(slot))]
    pub fn new_with_details(
        slot: SemanticObjectId,
        items: Vec<SemanticObjectId>,
        restriction: Option<SemanticObjectId>,
        quantity: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(RespectivelyStream {
            slot,
            items,
            restriction,
            quantity,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.slot);
        out.extend(self.items.iter().copied());
        extend_optional(out, self.restriction);
        extend_optional(out, self.quantity);
    }
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_ids_match_types(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    first_semantic_object_id_type_mismatch(objects).is_none()
}

#[requires(true)]
#[ensures(true)]
fn first_semantic_object_id_type_mismatch(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Option<String> {
    let mut seen_indices = BTreeMap::new();
    for (id, object) in objects {
        if let Some(previous) = seen_indices.insert(id.index(), *id) {
            return Some(format!("{id} reuses numeric ID already used by {previous}"));
        }
        if id.object_kind() != object.object_kind() {
            return Some(format!(
                "{id} has ID kind {:?}, but object type is {:?}",
                id.object_kind(),
                object.object_kind()
            ));
        }
        if id.object_kind() == SemanticObjectKind::Referent {
            if id.referent_sort() != object.sort() {
                return Some(format!(
                    "{id} has ID sort {:?}, but object sort is {:?}",
                    id.referent_sort(),
                    object.sort()
                ));
            }
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_references_are_defined(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    first_undefined_semantic_reference(objects).is_none()
}

#[requires(true)]
#[ensures(true)]
fn first_undefined_semantic_reference(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Option<(SemanticObjectId, SemanticObjectId)> {
    let mut references = Vec::new();
    for (source, object) in objects {
        references.clear();
        object.references_into(&mut references);
        if let Some(missing) = references
            .iter()
            .copied()
            .find(|reference| !objects.contains_key(reference))
        {
            return Some((*source, missing));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_references_match_roles(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects
        .values()
        .all(semantic_object_references_match_roles_for_object)
}
#[requires(true)]
#[ensures(ret)]
fn semantic_object_references_match_roles_for_object(_object: &SemanticObject) -> bool {
    // All per-object reference roles are enforced by the validated variant
    // nodes and their validated child values.
    true
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_question_slots_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    semantic_object_question_slots_validation_error(objects).is_none()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn semantic_object_question_slots_validation_error(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Option<String> {
    for (id, object) in objects {
        let valid = match object.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.tense_modal.is_none_or(|parameter| {
                parameter_has_sort_and_role(
                    objects,
                    parameter,
                    SemanticSort::TenseModal,
                    ParameterRole::TenseQuestion,
                )
            }),
            data!(SemanticObject::MathExpression(node)) => match node.kind.as_data() {
                data!(MathExpressionNodeKind::QuestionedOperator {
                    operator_parameter,
                    ..
                }) => parameter_has_sort_and_role(
                    objects,
                    *operator_parameter,
                    SemanticSort::MathOperator,
                    ParameterRole::MathOperatorQuestion,
                ),
                _ => true,
            },
            data!(SemanticObject::Quantity(node)) => {
                node.value.question_parameters.iter().all(|parameter| {
                    parameter_has_sort_and_role(
                        objects,
                        *parameter,
                        SemanticSort::Number,
                        ParameterRole::QuantityQuestion,
                    )
                })
            }
            data!(SemanticObject::Formula(node)) => formula_question_slots_are_valid(objects, node),
            data!(SemanticObject::Question(node)) => {
                for (slot_index, slot) in node.slots.iter().enumerate() {
                    if !question_slot_parameter_is_valid(objects, node, slot) {
                        return Some(format!(
                            "{id} answer slot {slot_index} has kind/domain {:?} but parameter {:?} has an incompatible sort or role",
                            slot.kind_and_domain().unwrap_or((node.kind, node.domain)),
                            slot.parameter(),
                        ));
                    }
                    if let Some(parameter) = slot.parameter()
                        && !semantic_object_reaches(objects, node.body, parameter)
                    {
                        let question_text = node
                            .common
                            .source
                            .as_ref()
                            .and_then(|source| source.text.as_deref())
                            .unwrap_or("<unknown source>");
                        let parameter_text = objects
                            .get(&parameter)
                            .and_then(SemanticObject::source)
                            .and_then(|source| source.text.as_deref())
                            .unwrap_or("<unknown source>");
                        return Some(format!(
                            "{id} for `{question_text}` answer slot {slot_index} parameter {parameter} from `{parameter_text}` is not reachable from question body {}",
                            node.body,
                        ));
                    }
                }
                true
            }
            _ => true,
        };
        if !valid {
            return Some(format!(
                "{id} contains a question parameter with an incompatible sort, role, or structural host"
            ));
        }
    }
    None
}

#[requires(objects.contains_key(&root))]
#[ensures(true)]
pub(crate) fn semantic_object_reaches(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    root: SemanticObjectId,
    target: SemanticObjectId,
) -> bool {
    let mut pending = vec![root];
    let mut visited = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if current == target {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(object) = objects.get(&current) {
            object.references_into(&mut pending);
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn question_slot_parameter_is_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    question: &QuestionNode,
    slot: &QuestionSlot,
) -> bool {
    let (kind, domain) = slot
        .kind_and_domain()
        .unwrap_or((question.kind, question.domain));
    if kind == QuestionKind::Truth {
        return slot.parameter().is_none();
    }
    let Some(parameter) = slot.parameter() else {
        return false;
    };
    let role = match kind {
        QuestionKind::Argument => ParameterRole::ArgumentQuestion,
        QuestionKind::Relation => ParameterRole::RelationQuestion,
        QuestionKind::Place => ParameterRole::PlaceQuestion,
        QuestionKind::Connective => ParameterRole::ConnectiveQuestion,
        QuestionKind::Tense => ParameterRole::TenseQuestion,
        QuestionKind::MathOperator => ParameterRole::MathOperatorQuestion,
        QuestionKind::Attitude => ParameterRole::AttitudeQuestion,
        QuestionKind::Quantity => ParameterRole::QuantityQuestion,
        QuestionKind::Truth | QuestionKind::Multiple => return false,
    };
    parameter_has_sort_and_role(objects, parameter, domain, role)
}

#[requires(true)]
#[ensures(true)]
fn formula_question_slots_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    formula: &FormulaNode,
) -> bool {
    match formula.as_data() {
        data!(FormulaNode::Connective(node)) => {
            connector_question_slot_is_valid(objects, node.operator, node.connector.as_ref())
        }
        data!(FormulaNode::Quantified(node)) => {
            node.variable.object_kind() != SemanticObjectKind::Parameter
                || parameter_has_sort_and_role(
                    objects,
                    node.variable,
                    SemanticSort::Relation,
                    ParameterRole::RelationVariable,
                )
        }
        data!(FormulaNode::QuantifierBundle(node)) => node.bindings.iter().all(|binding| {
            binding.variable.object_kind() != SemanticObjectKind::Parameter
                || parameter_has_sort_and_role(
                    objects,
                    binding.variable,
                    SemanticSort::Relation,
                    ParameterRole::RelationVariable,
                )
        }),
        data!(FormulaNode::Atom(_)) | data!(FormulaNode::RespectivelyDistribution(_)) => true,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_compositions_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        let composition = match object.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.composition.as_ref(),
            data!(SemanticObject::Referent(node)) => node.composition.as_ref(),
            _ => None,
        };
        composition
            .is_none_or(|composition| composition_operator_parameter_is_valid(objects, composition))
    })
}

#[requires(true)]
#[ensures(true)]
fn composition_operator_parameter_is_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    composition: &Composition,
) -> bool {
    composition.operator_parameter.is_none_or(|parameter| {
        parameter_has_sort_and_role(
            objects,
            parameter,
            SemanticSort::Connective,
            ParameterRole::ConnectiveQuestion,
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn connector_question_slot_is_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    operator: FormulaOperator,
    connector: Option<&Connector>,
) -> bool {
    if operator == FormulaOperator::ConnectiveQuestion {
        return connector
            .and_then(|connector| connector.parameter)
            .is_some_and(|parameter| {
                parameter_has_sort_and_role(
                    objects,
                    parameter,
                    SemanticSort::Connective,
                    ParameterRole::ConnectiveQuestion,
                )
            });
    }
    true
}

#[requires(true)]
#[ensures(true)]
fn parameter_has_sort_and_role(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    id: SemanticObjectId,
    sort: SemanticSort,
    role: ParameterRole,
) -> bool {
    objects.get(&id).is_some_and(|object| {
        matches!(
            object.as_data(),
            data!(SemanticObject::Parameter(node))
                if node.sort == sort && node.role == role
        )
    })
}

#[requires(true)]
#[ensures(ret == matches!((kind, domain),
    (QuestionKind::Truth, SemanticSort::TruthValue)
        | (QuestionKind::Argument, SemanticSort::Entity)
        | (QuestionKind::Relation, SemanticSort::Relation)
        | (QuestionKind::Place, SemanticSort::Place)
        | (QuestionKind::Connective, SemanticSort::Connective)
        | (QuestionKind::Tense, SemanticSort::TenseModal)
        | (QuestionKind::MathOperator, SemanticSort::MathOperator)
        | (QuestionKind::Attitude, SemanticSort::Entity)
        | (QuestionKind::Quantity, SemanticSort::Number)
        | (QuestionKind::Multiple, SemanticSort::ArgumentBundle)
))]
pub(crate) fn question_kind_domain_are_coherent(kind: QuestionKind, domain: SemanticSort) -> bool {
    matches!(
        (kind, domain),
        (QuestionKind::Truth, SemanticSort::TruthValue)
            | (QuestionKind::Argument, SemanticSort::Entity)
            | (QuestionKind::Relation, SemanticSort::Relation)
            | (QuestionKind::Place, SemanticSort::Place)
            | (QuestionKind::Connective, SemanticSort::Connective)
            | (QuestionKind::Tense, SemanticSort::TenseModal)
            | (QuestionKind::MathOperator, SemanticSort::MathOperator)
            | (QuestionKind::Attitude, SemanticSort::Entity)
            | (QuestionKind::Quantity, SemanticSort::Number)
            | (QuestionKind::Multiple, SemanticSort::ArgumentBundle)
    )
}

#[requires(true)]
#[ensures(true)]
fn parameter_role_matches_sort(sort: Option<SemanticSort>, role: Option<ParameterRole>) -> bool {
    match role {
        Some(ParameterRole::PropertySlot) => sort.is_some(),
        Some(ParameterRole::RelativeClauseHead)
        | Some(ParameterRole::ArgumentQuestion)
        | Some(ParameterRole::AttitudeQuestion) => sort == Some(SemanticSort::Entity),
        Some(ParameterRole::RelationQuestion)
        | Some(ParameterRole::RelationVariable)
        | Some(ParameterRole::UnspecifiedRelation) => sort == Some(SemanticSort::Relation),
        Some(ParameterRole::PlaceQuestion) => sort == Some(SemanticSort::Place),
        Some(ParameterRole::ConnectiveQuestion) => sort == Some(SemanticSort::Connective),
        Some(ParameterRole::TenseQuestion) => sort == Some(SemanticSort::TenseModal),
        Some(ParameterRole::MathOperatorQuestion) => sort == Some(SemanticSort::MathOperator),
        Some(ParameterRole::QuantityQuestion) => sort == Some(SemanticSort::Number),
        Some(ParameterRole::RespectiveSlot) => sort.is_some(),
        None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_variable_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    kind == SemanticObjectKind::Referent || kind == SemanticObjectKind::Parameter
}

#[requires(true)]
#[ensures(true)]
fn quantifier_formula_operator_is_allowed(operator: FormulaOperator) -> bool {
    matches!(
        operator,
        FormulaOperator::Exists
            | FormulaOperator::Forall
            | FormulaOperator::None
            | FormulaOperator::Cardinality
            | FormulaOperator::PluralExists
            | FormulaOperator::PluralForall
    )
}

#[requires(quantifier_formula_operator_is_allowed(operator))]
#[ensures(ret == ((matches!(operator, FormulaOperator::Forall | FormulaOperator::PluralForall) && restriction.is_some()).then_some(DomainImport::Projective)))]
fn quantified_formula_domain_import(
    operator: FormulaOperator,
    restriction: Option<SemanticObjectId>,
) -> Option<DomainImport> {
    (matches!(
        operator,
        FormulaOperator::Forall | FormulaOperator::PluralForall
    ) && restriction.is_some())
    .then_some(DomainImport::Projective)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_domain_imports_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        let Ok(serde_json::Value::Object(serialized)) = serde_json::to_value(object) else {
            return false;
        };
        match object.formula_domain_import() {
            Some(domain_import) => serde_json::to_value(domain_import)
                .is_ok_and(|expected| serialized.get("domainImport") == Some(&expected)),
            None => !serialized.contains_key("domainImport"),
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn quantifier_binding_matches_role(binding: &QuantifierBinding) -> bool {
    quantifier_formula_operator_is_allowed(binding.operator)
        && quantifier_variable_kind_is_allowed(binding.variable.object_kind())
        && binding
            .source_variable
            .is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent)
        && binding.selection_source.as_ref().is_none_or(|source| {
            source.variable.object_kind() == SemanticObjectKind::Referent
                && binding
                    .source_variable
                    .is_none_or(|variable| variable == source.variable)
        })
        && binding
            .restriction
            .is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula)
        && binding
            .quantity
            .is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity)
}

#[requires(true)]
#[ensures(true)]
fn utterance_content_reference_matches_force(
    force: Option<UtteranceForce>,
    content: SemanticObjectId,
) -> bool {
    let ordinary_content = matches!(
        content.object_kind(),
        SemanticObjectKind::Formula
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Question
            | SemanticObjectKind::DisplayedContent
    );
    if ordinary_content {
        return true;
    }
    matches!(
        force,
        Some(UtteranceForce::Mention | UtteranceForce::Vocative)
    ) && argument_object_kind_can_fill(content.object_kind())
}

#[requires(true)]
#[ensures(true)]
fn sequence_item_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::DisplayedContent
    )
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_arguments_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        let data!(SemanticObject::Predication(object)) = object.as_data() else {
            return true;
        };
        let modal_arguments_valid = object.modal_arguments.iter().all(|argument| {
            argument.arguments.iter().all(|(place, value)| {
                place.get() > 0 && argument_value_references_allowed_objects(value, objects)
            })
        });
        object.arguments.iter().all(|(place, value)| {
            place.get() > 0 && argument_value_references_allowed_objects(value, objects)
        }) && object.place_questions.iter().all(|question| {
            objects
                .get(&question.parameter)
                .is_some_and(|object| object.object_kind() == SemanticObjectKind::Parameter)
                && argument_value_references_allowed_objects(&question.argument, objects)
                && question
                    .candidate_places
                    .iter()
                    .all(|place| place.get() > 0)
        }) && modal_arguments_valid
            && object.reciprocity.iter().all(|exchange| {
                argument_value_references_allowed_objects(&exchange.left, objects)
                    && argument_value_references_allowed_objects(&exchange.right, objects)
            })
    })
}

#[requires(true)]
#[ensures(true)]
fn argument_value_references_allowed_objects(
    value: &ArgumentValue,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let value_is_valid = match value.value {
        Some(value) => objects
            .get(&value)
            .is_some_and(|object| argument_object_kind_can_fill(object.object_kind())),
        None => value.kind == ArgumentValueKind::Deleted,
    };
    value_is_valid
        && value.quantity.is_none_or(|quantity| {
            objects
                .get(&quantity)
                .is_some_and(|object| object.object_kind() == SemanticObjectKind::Quantity)
        })
        && value.relative_clauses.iter().all(|clause| {
            objects
                .get(&clause.body)
                .is_some_and(|object| object.object_kind() == SemanticObjectKind::Formula)
        })
}

#[requires(true)]
#[ensures(true)]
pub fn is_numbered_argument_place(place: &str) -> bool {
    PlaceIndex::from_numbered_label(place).is_some()
}

#[requires(true)]
#[ensures(true)]
pub fn source_from_spans(
    spans: &[SourceSpan],
    source_text: Option<&str>,
    construct: Option<&str>,
) -> Option<SemanticSource> {
    let first = spans.first()?;
    let byte_start = spans
        .iter()
        .map(|span| span.byte_start)
        .min()
        .unwrap_or(first.byte_start);
    let byte_end = spans
        .iter()
        .map(|span| span.byte_end)
        .max()
        .unwrap_or(first.byte_end);
    let text = source_text
        .and_then(|text| text.get(byte_start..byte_end))
        .map(str::to_owned);
    Some(SemanticSource {
        span: SourceByteSpan {
            byte_start,
            byte_end,
        },
        text,
        construct: construct.map(str::to_owned),
    })
}

#[requires(true)]
#[ensures(true)]
pub fn diagnostic(message: impl Into<String>) -> SemanticDiagnostic {
    SemanticDiagnostic::warning(message)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_graph_object_ids_match_types(graph: &SemanticGraph) -> bool {
    semantic_object_ids_match_types(&graph.objects)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_graph_references_are_defined(graph: &SemanticGraph) -> bool {
    semantic_object_references_are_defined(&graph.objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(semantic_event_bindings_are_derived(ret.root, &ret.objects))]
    fn graph_with_generated_atom() -> SemanticGraph {
        let root = SemanticObjectId::formula(1);
        let atom = SemanticObjectId::formula(2);
        let predication = SemanticObjectId::predication(3);
        let eventuality = SemanticObjectId::eventuality(4);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![atom],
                None,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            atom,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(
            predication,
            SemanticObject::predication(
                "klama".to_owned(),
                Some(eventuality),
                BTreeMap::new(),
                PredicationMode::Asserted,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            eventuality,
            SemanticObject::generated_eventuality(EventualityClass::Event, None, None),
        );
        SemanticGraph::new(root, objects).expect("generated atom graph is valid")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn eventuality_subsorts_satisfy_general_eventuality() {
        let general = SemanticSort::eventuality();
        let process = SemanticSort::Eventuality(EventualitySort::Process);
        assert!(process.is_subsort_of(general));
        assert!(general.is_subsort_of(general));
        assert!(!process.is_subsort_of(SemanticSort::Entity));
        assert!(!SemanticSort::Entity.is_subsort_of(general));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_object_id_rejects_zero_index_data() {
        let invalid = SemanticObjectId::try_from_data(data!(SemanticObjectId {
            prefix: SemanticIdPrefix::Structural(SemanticObjectKind::Formula),
            index: 0,
        }));

        assert!(invalid.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_dangling_object_references() {
        let root = SemanticObjectId::formula(1);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(SemanticObjectId::predication(2), None, Vec::new()),
        );

        let error = SemanticGraph::new(root, objects).expect_err("dangling reference");
        assert!(error.to_string().contains("must not dangle"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_unbound_generated_eventualities() {
        let root = SemanticObjectId::formula(1);
        let predication = SemanticObjectId::predication(2);
        let eventuality = SemanticObjectId::eventuality(3);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(
            predication,
            SemanticObject::predication(
                "klama".to_owned(),
                None,
                BTreeMap::new(),
                PredicationMode::Asserted,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            eventuality,
            SemanticObject::generated_eventuality(EventualityClass::Event, None, None),
        );

        let error = SemanticGraph::new(root, objects).expect_err("generated event has no use");
        assert!(error.to_string().contains("no bindable semantic use"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_event_binding_contract_checks_identity_uniqueness_and_lowest_scope() {
        let graph = graph_with_generated_atom();
        let root = graph.root;
        let atom = SemanticObjectId::formula(2);
        let eventuality = GeneratedEventualityId::new(SemanticObjectId::eventuality(4));
        assert_eq!(
            graph
                .objects
                .get(&atom)
                .expect("atom exists")
                .bound_eventualities(),
            &[eventuality]
        );

        let mut unbound = graph.clone().into_data();
        unbound
            .objects
            .get_mut(&atom)
            .expect("atom exists")
            .set_bound_eventualities(Vec::new());
        assert!(!semantic_event_bindings_are_derived(
            unbound.root,
            &unbound.objects
        ));

        let mut duplicated = graph.clone().into_data();
        duplicated
            .objects
            .get_mut(&root)
            .expect("root exists")
            .set_bound_eventualities(vec![eventuality]);
        assert!(!semantic_event_bindings_are_derived(
            duplicated.root,
            &duplicated.objects
        ));

        let mut too_high = graph.clone().into_data();
        too_high
            .objects
            .get_mut(&atom)
            .expect("atom exists")
            .set_bound_eventualities(Vec::new());
        too_high
            .objects
            .get_mut(&root)
            .expect("root exists")
            .set_bound_eventualities(vec![eventuality]);
        assert!(!semantic_event_bindings_are_derived(
            too_high.root,
            &too_high.objects
        ));

        let referential_id = SemanticObjectId::eventuality(5);
        let mut referential = graph.into_data();
        referential.objects.insert(
            referential_id,
            SemanticObject::referential_eventuality(EventualityClass::Event, None, None),
        );
        referential
            .objects
            .get_mut(&root)
            .expect("root exists")
            .set_bound_eventualities(vec![GeneratedEventualityId::new(referential_id)]);
        assert!(!semantic_event_bindings_are_derived(
            referential.root,
            &referential.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_dangling_scalar_negation_scale() {
        let root = SemanticObjectId::formula(1);
        let predication = SemanticObjectId::predication(2);
        let mut object = SemanticObject::predication(
            "klama".to_owned(),
            None,
            BTreeMap::new(),
            PredicationMode::Asserted,
            None,
            Vec::new(),
        );
        object.set_predication_scalar_negation(
            ScalarNegation::new(ScalarNegationKind::OtherThan, "na'e".to_owned())
                .with_scale(SemanticObjectId::referent(3)),
        );

        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(predication, object);

        let error = SemanticGraph::new(root, objects).expect_err("dangling scale reference");
        assert!(error.to_string().contains("must not dangle"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_id_object_kind_mismatches() {
        let root = SemanticObjectId::formula(1);
        let child = SemanticObjectId::formula(2);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![child],
                None,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            child,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                None,
                None,
                None,
                Vec::new(),
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("wrong object kind for ID");
        assert!(
            error
                .to_string()
                .contains("ID prefixes must match object types")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn place_index_rejects_malformed_argument_places() {
        assert!(PlaceIndex::from_numbered_label("01").is_none());
        assert!(PlaceIndex::from_numbered_label("x0").is_none());
        assert!(PlaceIndex::from_numbered_label("x01").is_none());
        assert_eq!(
            serde_json::to_string(&PlaceIndex::new(10)).expect("place index serializes"),
            "\"x10\""
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn place_index_serializes_argument_maps_in_numeric_order() {
        let mut arguments = BTreeMap::new();
        arguments.insert(
            PlaceIndex::new(10),
            ArgumentValue::filled(SemanticObjectId::referent(10), None),
        );
        arguments.insert(
            PlaceIndex::new(2),
            ArgumentValue::filled(SemanticObjectId::referent(2), None),
        );

        let predication = SemanticObject::predication(
            "broda".to_owned(),
            None,
            arguments,
            PredicationMode::Restrictive,
            None,
            Vec::new(),
        );
        let json = serde_json::to_string(&predication).expect("predication serializes");

        let x2_position = json.find(r#""x2""#).expect("x2 key is serialized");
        let x10_position = json.find(r#""x10""#).expect("x10 key is serialized");
        assert!(
            x2_position < x10_position,
            "argument map must serialize in numeric place order: {json}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_incoherent_parameter_sort() {
        let root = SemanticObjectId::eventuality(1);
        let parameter = SemanticObjectId::parameter(2);
        let mut eventuality =
            SemanticObject::referential_eventuality(EventualityClass::Event, None, None);
        eventuality
            .update_eventuality(|node| node.with_data(data! { tense_modal: Some(parameter) }));

        let mut objects = BTreeMap::new();
        objects.insert(root, eventuality);
        objects.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::ArgumentQuestion,
                "ma".to_owned(),
                None,
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("wrong parameter sort");
        assert!(error.to_string().contains("coherent parameters"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_incoherent_connector_question_parameter() {
        let root = SemanticObjectId::formula(1);
        let child = SemanticObjectId::formula(2);
        let predication = SemanticObjectId::predication(3);
        let parameter = SemanticObjectId::parameter(4);

        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::connective_formula(
                FormulaOperator::ConnectiveQuestion,
                vec![child],
                Some(new!(Connector {
                    source: "je'i".to_owned(),
                    locus: "tense".to_owned(),
                    truth_table: None,
                    parameter: Some(parameter),
                })),
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            child,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(
            predication,
            SemanticObject::predication(
                "king".to_owned(),
                None,
                BTreeMap::new(),
                PredicationMode::Asserted,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::TenseModal,
                ParameterRole::TenseQuestion,
                "cu'e".to_owned(),
                None,
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("impossible connector");
        assert!(error.to_string().contains("coherent parameters"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn argument_value_invariant_rejects_deleted_values() {
        let invalid = ArgumentValue::try_from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: Some(SemanticObjectId::referent(1)),
            quantity: None,
            introduced_by: Some("zi'o".to_owned()),
            source: None,
            relative_clauses: Vec::new(),
            command_target: None,
        }));

        assert!(invalid.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn argument_value_invariant_rejects_deleted_quantities() {
        let invalid = ArgumentValue::try_from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: None,
            quantity: Some(SemanticObjectId::quantity(1)),
            introduced_by: Some("zi'o".to_owned()),
            source: None,
            relative_clauses: Vec::new(),
            command_target: None,
        }));

        assert!(invalid.is_err());
    }
}
