//! A type-directed resolver over the model type graph, built from the source
//! scan. It maps a JSON coordinate (`"<NodeType>:<dotted.path>"`) to the Rust
//! `(owner-type, leaf-field, leaf-type)` it lands on, so a witness pointer can be
//! tied to the surface/field it *claims* — not merely to some field carrying an
//! equal string.

#![allow(dead_code)]

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use std::collections::{BTreeMap, BTreeSet};

use jbotci_semantics::completeness::model::SurfaceCategory;

use crate::schema_scan::Model;

/// Marker child-types for the two polymorphic wrappers, resolved by peeking at
/// the next path segment.
const ASPECT_OR_RECURRENCE: &str = "__ASPECT_OR_RECURRENCE__";
const MATH_LITERAL_VALUE: &str = "__MATHLITERALVALUE__";

/// The Rust node types (JSON object surfaces).
const NODE_TYPES: &[(&str, &str)] = &[
    ("Utterance", "UtteranceNode"),
    ("Sequence", "SequenceNode"),
    ("Eventuality", "EventualityNode"),
    ("Referent", "ReferentNode"),
    ("Parameter", "ParameterNode"),
    ("Predication", "PredicationNode"),
    ("Sign", "SignNode"),
    ("DisplayedContent", "DisplayedContentNode"),
    ("MathExpression", "MathExpressionNode"),
    ("Quantity", "QuantityNode"),
    ("RelationMetadata", "RelationMetadataNode"),
    ("Question", "QuestionNode"),
];

/// `(owner, key)` edges the manual node serializers introduce that are not
/// direct `*Node` struct fields (derived values / computed keys). Mirrors the
/// `serialize_*` expressions in `model/semantic_object.rs`.
const DERIVED_EDGES: &[(&str, &str, &str)] = &[
    ("Eventuality", "category", "ReferentCategory"),
    ("Eventuality", "scopeDependence", "ScopeDependence"),
    ("Eventuality", "sort", "SemanticSort"),
    ("Eventuality", "intervalModifiers", "IntervalModifier"),
    ("Eventuality", "spatialIntervalModifiers", "IntervalModifier"),
    ("Referent", "sort", "SemanticSort"),
    ("Referent", "scopeDependence", "ScopeDependence"),
    ("Sign", "sort", "SemanticSort"),
    ("Sign", "kind", "SignKind"),
    ("Predication", "relation", "RelationLabel"),
    ("MathExpression", "operator", "MathOperator"),
    ("MathExpression", "literal", "MathLiteral"),
    ("MathExpression", "endpointInclusion", "IntervalEndpointInclusion"),
    ("MathExpression", "scalarNegation", "ScalarNegation"),
    ("Formula", "operator", "FormulaOperator"),
    ("Formula", "domainImport", "DomainImport"),
    ("Formula", "connector", "Connector"),
    ("Formula", "selectionSource", "SelectionSource"),
    ("Formula", "bindings", "QuantifierBinding"),
    ("Formula", "streams", "RespectivelyStream"),
    ("Formula", "restriction", "__ID__"),
    ("Formula", "body", "__ID__"),
    // Enums that serialize as objects: `kind` carries the enum value itself.
    ("ScopeDependence", "kind", "ScopeDependence"),
    ("ScopeDependence", "mayDependOn", "__ID__"),
    ("IntervalModifier", "kind", "IntervalModifier"),
    ("IntervalModifier", "value", ASPECT_OR_RECURRENCE),
    ("MathLiteral", "value", MATH_LITERAL_VALUE),
];

/// Where a resolved coordinate lands in the type graph.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The Rust type owning the leaf field.
    pub owner: String,
    /// The leaf JSON key.
    pub key: String,
    /// The declared type of the leaf field, if the graph knows it.
    pub leaf_type: Option<String>,
}

/// The resolver.
#[invariant(true)]
pub struct TypeGraph {
    edges: BTreeMap<(String, String), String>,
    struct_fields: BTreeMap<String, BTreeSet<String>>,
}

impl TypeGraph {
    #[requires(true)]
    #[ensures(true)]
    pub fn from_source() -> Self {
        let model = Model::from_source();
        let mut edges = BTreeMap::new();
        let mut struct_fields: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for (name, (fields, _serialize)) in &model.structs {
            for field in fields {
                edges.insert((name.clone(), field.key.clone()), field.base_type.clone());
                struct_fields
                    .entry(name.clone())
                    .or_default()
                    .insert(field.key.clone());
            }
        }
        // Node surfaces borrow their `*Node` struct's field types, plus the two
        // common fields every object carries via `SemanticObjectCommon`.
        for (node, node_struct) in NODE_TYPES {
            if let Some((fields, _)) = model.structs.get(*node_struct) {
                for field in fields {
                    edges.insert(((*node).to_owned(), field.key.clone()), field.base_type.clone());
                }
            }
            edges.insert(((*node).to_owned(), "source".to_owned()), "SemanticSource".to_owned());
            edges.insert(
                ((*node).to_owned(), "diagnostics".to_owned()),
                "SemanticDiagnostic".to_owned(),
            );
        }
        // Formula also carries source/diagnostics (its `*Node` structs vary).
        edges.insert(("Formula".to_owned(), "source".to_owned()), "SemanticSource".to_owned());
        edges.insert(
            ("Formula".to_owned(), "diagnostics".to_owned()),
            "SemanticDiagnostic".to_owned(),
        );
        // QuestionSlot (untagged) union of variant fields.
        if let Some(question_slot) = model.enums.get("QuestionSlot") {
            for variant in &question_slot.variants {
                for field in &variant.fields {
                    edges.insert(
                        ("QuestionSlot".to_owned(), field.key.clone()),
                        field.base_type.clone(),
                    );
                }
            }
        }
        for (owner, key, ty) in DERIVED_EDGES {
            edges.insert(((*owner).to_owned(), (*key).to_owned()), (*ty).to_owned());
        }

        TypeGraph {
            edges,
            struct_fields,
        }
    }

    #[requires(!owner.is_empty() && !key.is_empty())]
    #[ensures(true)]
    fn edge(&self, owner: &str, key: &str) -> Option<&str> {
        self.edges.get(&(owner.to_owned(), key.to_owned())).map(String::as_str)
    }

    /// The declared base type of `owner.key`, if known. Public so a test can
    /// derive provenance (`SemanticSource`-typed fields) from the type graph and
    /// cross-check the disposition policy's hard-coded source-link set.
    #[requires(!owner.is_empty() && !key.is_empty())]
    #[ensures(true)]
    pub fn field_type(&self, owner: &str, key: &str) -> Option<&str> {
        self.edge(owner, key)
    }

    /// The concrete child-owner reached from `owner` through `key`, given the
    /// following segment (needed to disambiguate the polymorphic wrappers).
    #[requires(!owner.is_empty() && !key.is_empty())]
    #[ensures(true)]
    fn child_owner(&self, owner: &str, key: &str, next_segment: Option<&str>) -> Option<String> {
        match self.edge(owner, key)? {
            ASPECT_OR_RECURRENCE => {
                let next = next_segment?;
                if self.struct_fields.get("Aspect").is_some_and(|fields| fields.contains(next)) {
                    Some("Aspect".to_owned())
                } else {
                    Some("Recurrence".to_owned())
                }
            }
            MATH_LITERAL_VALUE => {
                let next = next_segment?;
                self.struct_fields
                    .get("MixedRadixLiteral")
                    .filter(|fields| fields.contains(next))
                    .map(|_| "MixedRadixLiteral".to_owned())
            }
            "__ID__" => None,
            concrete => Some(concrete.to_owned()),
        }
    }

    /// Resolve a `"<NodeType>:<dotted.path>"` coordinate to its `(owner, key,
    /// leaf_type)`. `x*` place-map segments are transparent (the map key, not a
    /// field). Returns `None` if the graph cannot follow the path.
    #[requires(true)]
    #[ensures(true)]
    pub fn resolve(&self, coord: &str) -> Option<Resolved> {
        let (root, path) = coord.split_once(':')?;
        let segments: Vec<&str> = path.split('.').filter(|s| *s != "x*").collect();
        if segments.is_empty() {
            return None;
        }
        let mut owner = root.to_owned();
        for window in 0..segments.len() - 1 {
            let key = segments[window];
            let next = segments.get(window + 1).copied();
            owner = self.child_owner(&owner, key, next)?;
        }
        let key = segments[segments.len() - 1].to_owned();
        let leaf_type = self.edge(&owner, &key).map(|ty| ty.to_owned()).filter(|ty| {
            ty != ASPECT_OR_RECURRENCE && ty != MATH_LITERAL_VALUE && ty != "__ID__"
        });
        Some(Resolved {
            owner,
            key,
            leaf_type,
        })
    }

    /// Whether a witness `path` resolves to the claimed `(category, surface,
    /// field)`. For a `Field` entry the path's leaf must be `surface::field`; for
    /// a variant (`Enum`) the leaf's *type* must be the claimed enum. `Document`
    /// facts have no object path and are handled by the caller.
    #[requires(!path.is_empty() && !surface.is_empty() && !field.is_empty())]
    #[ensures(true)]
    pub fn path_matches_entry(
        &self,
        path: &str,
        category: SurfaceCategory,
        surface: &str,
        field: &str,
    ) -> bool {
        let Some(resolved) = self.resolve(path) else {
            return false;
        };
        match category {
            SurfaceCategory::Enum => resolved.leaf_type.as_deref() == Some(surface),
            SurfaceCategory::Object | SurfaceCategory::ValueStruct => {
                resolved.owner == surface && resolved.key == field
            }
            SurfaceCategory::Document => false,
        }
    }
}
