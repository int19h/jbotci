//! Canonical SFN-XML rendering for `lojban-semantics-json-1`.
//!
//! This is a faithful Rust port of `render_xml.py` at research commit
//! `c5d369e98358bffe9026898bb9f21cb8885e4a9b`.  Like the frozen `smusni`
//! renderer, it deliberately walks [`SemanticGraph`]'s own canonical JSON
//! serialization: the notation is specified over that interchange surface, and
//! using it directly avoids a second, drift-prone reconstruction of the serde
//! shape.  The XML emitter and scope planner are independent of `smusni`; the
//! two renderers share only this justified canonical-JSON boundary.

use std::collections::{BTreeSet, HashMap, HashSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use serde_json::{Map, Value};

use crate::model::{SEMANTIC_JSON_VERSION, SemanticGraph};

const SCOPE_DEPENDENCE_TEACHING: &str = "A referent mentioned inside enclosing binders — quantifier variables or abstraction/question parameters — may either be one shared thing for all binder values, or a possibly different thing per combination of values; the text does not decide, and nothing is marked when it may depend on every enclosing binder. SAME-FOR-ALL marks the exceptions known to be one and the same across all enclosing binders.";

const KEY_RULES_BEFORE_SORTS: &[(&str, &str)] = &[
    (
        "ids",
        "ID= marks the definition of a shared graph node except a speech-situation referent; DEICTIC-GROUND SPEAKER-REF=/AUDIENCE-REF=/TIME-REF=/PLACE-REF= values are the sole definition sites for those referent ids. REF= and named *-REF= attributes point to discourse referents. GROUND= points to a deictic-ground unit and is the sole suffix exception. Later REF= occurrences are exact-node reuse. Graph-node ids are opaque and JSON-aligned; distinct ids assert neither identity (=) nor non-identity (≠).",
    ),
    (
        "lexical-categories",
        "UPPERCASE = structural keywords (element and attribute names); PascalCase = sorts; lowercase = content words as data values only; quoted attribute strings = names.",
    ),
];

const KEY_RULES_AFTER_SORTS: &[(&str, &str)] = &[
    (
        "embedded-questions",
        "EMBEDDED-QUESTIONS preserves question metadata attached to an abstraction formula. UNKNOWN TYPE=\"question\" is the typed escape hatch for that metadata: FIELD names are JSON field names, and RECORD/LIST/STRING preserve its interchange values without claiming a compact question notation.",
    ),
    (
        "some",
        "REF=\"SOME\" denotes a distinct elided node per occurrence; distinct nodes assert neither identity (=) nor non-identity (≠).",
    ),
    (
        "ground",
        "DEICTIC-GROUND is the shared speech-situation unit selected by UTTERANCE GROUND=. Its g-prefixed ID is a notation-level rendering id because the JSON has no ground object id; if the graph gains context ids, the rendering must align. Ground units share one definition ⇔ their SPEAKER-REF/AUDIENCE-REF/TIME-REF/PLACE-REF graph referents are pairwise identical.",
    ),
    (
        "quantifiers",
        "EXISTS, FORALL, and CARDINALITY are binder elements; VARIABLE defines its variable ID=/SORT= at the binder site; use sites carry REF=. RESTRICTION and BODY are loud sibling elements; EXISTS writes RESTRICTION exactly when the graph supplies one; FORALL and CARDINALITY always write RESTRICTION explicitly, empty as RESTRICTION/.",
    ),
    ("scope-dependence", SCOPE_DEPENDENCE_TEACHING),
    (
        "scope-dependence-subsets",
        "POSSIBLY-DIFFERENT-PER= is a space-separated list of enclosing quantifier-variable or abstraction/question-parameter ids on which a referent may depend when that list is a strict subset of all enclosing binders.",
    ),
    (
        "number-neutrality",
        "References are number-neutral: a reference may denote one or several individuals; the only number commitments are explicit quantities on descriptions, cardinality binders, or mass restrictions.",
    ),
    (
        "personal-reference",
        "PERSONAL-MASS-MEMBERSHIP states whether SPEAKER and AUDIENCE are INCLUDED or EXCLUDED and points to any additional included OTHERS. DEICTIC-REFERENCE states PROXIMITY to a discourse-referent GROUND-REF. These structures carry the semantics; no pro-sumti label is implied.",
    ),
    (
        "speaker-anchor",
        "A description is anchored to its enclosing utterance's speaker. SPEAKER-REF= on a description, or BY inside NAMED, appears only when the anchor differs from that enclosing speaker.",
    ),
    (
        "facet-silence",
        "Absent facet attribute ⇒ UNSPECIFIED (no commitment). Facet attributes: TIME, ACTUALITY, ASPECT, RECURRENCE, SPACE, SPATIAL-ASPECT, SPATIAL-RECURRENCE, DETAILS.",
    ),
    (
        "event-field",
        "EVENT is the reserved first child of a PREDICATION, its Eventuality referent, never a numbered ARG.",
    ),
    (
        "adjuncts",
        "ADJUNCT introduces a predicate-keyed optional participant of the host predication; PREDICATE= with flat ARG children is the compact single-lexical-predicate place map; without PREDICATE=, BODY carries the composite predicate subtree; ARG FILL=\"true\" marks the unique explicit non-host filled place; a non-unique graph stays complete and carries FILL-STATUS; APPLIES-TO links the host component.",
    ),
    (
        "pro-sumti",
        "UNRESOLVED-REFERENT WORD= is a word-only stopgap only for referents that remain unresolved after the jbotci#690 KOhA audit; the quoted WORD value is the stopgap's whole content. Bound-variable surface words are provenance-only.",
    ),
    (
        "mode",
        "MODE vocabulary: ASSERTED=main claim; RESTRICTIVE=restriction; INCIDENTAL=side claim; INERT=embedded nonclaim; DEFINITIONAL=identity definition; PERFORMATIVE=speech act. MODE is a required attribute on PREDICATION.",
    ),
    (
        "defs",
        "Every non-binder graph-node definition and every DEICTIC-GROUND definition sits in DEFS in the smallest graph scope strictly containing all uses; an attribute reference on element X is a use at X's position. Quantifier VARIABLE precedes its scope's DEFS; all graph-node uses outside their definition site are references.",
    ),
    (
        "atomic-lists",
        "A list of simple ids or numbers is a space-separated NMTOKENS attribute, never a sequence of child elements; semantic structure remains element-valued.",
    ),
    (
        "order",
        "Child order is fixed per element; CONNECTIVE child order is semantically significant; childless elements self-close.",
    ),
];

const FACET_FIELDS: &[&str] = &[
    "time",
    "actuality",
    "aspect",
    "recurrence",
    "space",
    "spatialAspect",
    "spatialRecurrence",
    "details",
];

const DESCRIPTOR_KINDS: &[&str] = &[
    "number",
    "name",
    "massName",
    "setName",
    "speakerDescription",
    "scale",
    "proSumti",
    "unloweredSumti",
    "description",
    "veridicalDescription",
    "veridicalMassDescription",
    "veridicalSetDescription",
    "speakerMassDescription",
    "speakerSetDescription",
    "speakerStereotypeDescription",
    "massNameDescription",
    "setNameDescription",
    "typicalDescription",
    "typicalPlaceValue",
    "utteranceReference",
    "elided",
    "abstractionAbout",
    "referentOfSymbol",
    "symbolForReferent",
    "memberOf",
    "setFrom",
    "massFrom",
    "sequenceFrom",
    "qualifiedSumti",
    "oppositeOf",
    "neutralOf",
    "affirmedAs",
    "otherThan",
];

/// The only omission families permitted by the owner-approved XML design.
///
/// Every family is provenance named by the prototype's `WAIVERS` block.  This
/// declaration is intentionally closed: adding a semantic omission requires an
/// explicit API and test change rather than silently widening the renderer.
#[invariant(::SourceRecord => true)]
#[invariant(::AssignedNameRecord => true)]
#[invariant(::DescriptorWord => true)]
#[invariant(::IntroducedBy => true)]
#[invariant(::QuantityText => true)]
#[invariant(::BoundVariableWord => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum XmlWaiverFamily {
    SourceRecord,
    AssignedNameRecord,
    DescriptorWord,
    IntroducedBy,
    QuantityText,
    BoundVariableWord,
}

/// One typed object or field occurrence in canonical semantic-graph JSON.
#[invariant(::Object { path } => path.starts_with("/objects/"))]
#[invariant(::Field { path } => path.starts_with("/objects/"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum XmlSurface {
    /// An object-valued JSON occurrence.
    Object { path: String },
    /// A named field occurrence on a JSON object.
    Field { path: String },
}

impl XmlSurface {
    /// Return this occurrence's canonical JSON Pointer.
    #[requires(true)]
    #[ensures(ret.starts_with("/objects/"))]
    pub fn path(&self) -> &str {
        match self.as_data() {
            data!(XmlSurface::Object { path }) | data!(XmlSurface::Field { path }) => path,
        }
    }
}

/// One concrete graph occurrence absent from ordinary SFN-XML.
#[invariant(
    surface.path().starts_with("/objects/"),
    "an omission must identify its concrete graph occurrence"
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XmlOmission {
    /// The declared waiver family, or `None` for an unwaived omission.
    pub waiver: Option<XmlWaiverFamily>,
    /// Whether the omitted occurrence is an object or a field, with its JSON Pointer.
    pub surface: XmlSurface,
}

/// The complete result of one XML render.
#[invariant(output.ends_with('\n'), "canonical SFN-XML has one trailing newline")]
#[invariant(omissions.iter().all(|omission| !omission.surface.path().is_empty()))]
#[expensive_invariant(
    omissions.iter().collect::<BTreeSet<_>>().len() == omissions.len(),
    "each omitted semantic occurrence must be reported at most once"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlRender {
    pub output: String,
    pub omissions: Vec<XmlOmission>,
}

/// The closed, owner-audited set of permitted XML omission families.
pub const XML_DECLARED_WAIVERS: &[XmlWaiverFamily] = &[
    XmlWaiverFamily::SourceRecord,
    XmlWaiverFamily::AssignedNameRecord,
    XmlWaiverFamily::DescriptorWord,
    XmlWaiverFamily::IntroducedBy,
    XmlWaiverFamily::QuantityText,
    XmlWaiverFamily::BoundVariableWord,
];

#[requires(path.starts_with("/objects/"))]
#[ensures(ret.path().starts_with("/objects/"))]
fn object_surface(path: String) -> XmlSurface {
    new!(XmlSurface::Object { path })
}

#[requires(path.starts_with("/objects/"))]
#[ensures(ret.path().starts_with("/objects/"))]
fn field_surface(path: String) -> XmlSurface {
    new!(XmlSurface::Field { path })
}

/// Remove one surface and every object/field occurrence nested below its JSON
/// Pointer without scanning unrelated inventory entries.
///
/// Object and field variants form separate ordered ranges. Starting each range
/// at `path + "/"` is significant: a bare `path` lower bound would also visit
/// lexicographic siblings such as `path-...` before reaching real descendants.
#[requires(surface.path().starts_with("/objects/"))]
#[ensures(!surfaces.contains(surface))]
fn remove_surface_subtree(surfaces: &mut BTreeSet<XmlSurface>, surface: &XmlSurface) -> bool {
    let path = surface.path().to_owned();
    let removed = surfaces.remove(surface);
    surfaces.remove(&object_surface(path.clone()));
    surfaces.remove(&field_surface(path.clone()));

    let descendant_prefix = format!("{path}/");
    let object_start = object_surface(descendant_prefix.clone());
    let object_descendants: Vec<XmlSurface> = surfaces
        .range(object_start..)
        .take_while(|candidate| {
            matches!(
                candidate.as_data(),
                data!(XmlSurface::Object { path }) if path.starts_with(&descendant_prefix)
            )
        })
        .cloned()
        .collect();
    let field_start = field_surface(descendant_prefix.clone());
    let field_descendants: Vec<XmlSurface> = surfaces
        .range(field_start..)
        .take_while(|candidate| {
            matches!(
                candidate.as_data(),
                data!(XmlSurface::Field { path }) if path.starts_with(&descendant_prefix)
            )
        })
        .cloned()
        .collect();
    for descendant in object_descendants.into_iter().chain(field_descendants) {
        surfaces.remove(&descendant);
    }
    removed
}

// Mutable XML construction state. Validity is established by the private
// constructors and canonical serializer rather than by a wrapper that would
// prohibit in-place tree assembly.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlElement {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Self>,
    text: Option<String>,
}

impl XmlElement {
    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        assert!(!name.is_empty(), "XML element names cannot be empty");
        Self {
            name,
            attributes: Vec::new(),
            children: Vec::new(),
            text: None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn with_attributes(
        name: impl Into<String>,
        attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        let mut result = Self::new(name);
        for (key, value) in attributes {
            result.set(key, value);
        }
        result
    }

    #[requires(true)]
    #[ensures(self.attributes.iter().all(|(name, _)| !name.is_empty()))]
    fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        assert!(!name.is_empty(), "XML attribute names cannot be empty");
        let value = value.into();
        if let Some((_, old)) = self.attributes.iter_mut().find(|(key, _)| key == &name) {
            *old = value;
        } else {
            self.attributes.push((name, value));
        }
    }

    #[requires(attributes.iter().all(|(name, _)| !name.is_empty()))]
    #[ensures(self.attributes.len() >= old(self.attributes.len()))]
    fn prepend_attributes(&mut self, attributes: Vec<(String, String)>) {
        assert!(
            attributes
                .iter()
                .all(|(name, _)| !self.attributes.iter().any(|(old, _)| old == name)),
            "{} already has an attribute being prepended",
            self.name
        );
        let mut existing = std::mem::take(&mut self.attributes);
        self.attributes = attributes;
        self.attributes.append(&mut existing);
    }

    #[requires(true)]
    #[ensures(self.children.len() == old(self.children.len()) + 1)]
    fn push(&mut self, child: Self) {
        self.children.push(child);
    }

    #[requires(true)]
    #[ensures(true)]
    fn extend(&mut self, children: Vec<Self>) {
        self.children.extend(children);
    }
}

#[requires(true)]
#[ensures(ret.contains("&amp;") == value.contains('&') || !value.contains('&'))]
fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[requires(true)]
#[ensures(!ret.contains('"'))]
fn escape_attribute(value: &str) -> String {
    escape_text(value)
        .replace('"', "&quot;")
        .replace('\t', "&#09;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn serialize_element(node: &XmlElement, depth: usize, output: &mut String) {
    output.push_str(&"  ".repeat(depth));
    output.push('<');
    output.push_str(&node.name);
    for (name, value) in &node.attributes {
        output.push(' ');
        output.push_str(name);
        output.push_str("=\"");
        output.push_str(&escape_attribute(value));
        output.push('"');
    }
    if node.children.is_empty() && node.text.is_none() {
        output.push_str("/>");
        return;
    }
    output.push('>');
    if let Some(text) = &node.text {
        output.push_str(&escape_text(text));
    }
    if !node.children.is_empty() {
        output.push('\n');
        for child in &node.children {
            serialize_element(child, depth + 1, output);
            output.push('\n');
        }
        output.push_str(&"  ".repeat(depth));
    }
    output.push_str("</");
    output.push_str(&node.name);
    output.push('>');
}

#[requires(true)]
#[ensures(ret.ends_with('\n'))]
fn serialize(root: &XmlElement) -> String {
    let mut output = String::new();
    serialize_element(root, 0, &mut output);
    output.push('\n');
    output
}

#[requires(true)]
#[ensures(true)]
fn json_object(value: &Value) -> &Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("expected JSON object, got {value:?}"))
}

#[requires(true)]
#[ensures(true)]
fn string_field<'a>(object: &'a Map<String, Value>, field: &str) -> &'a str {
    object
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("field {field:?} must be a string"))
}

#[requires(true)]
#[ensures(true)]
fn optional_string<'a>(object: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

#[requires(true)]
#[ensures(true)]
fn json_array(value: &Value) -> &[Value] {
    value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("expected JSON array, got {value:?}"))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn enum_token(value: &Value) -> String {
    let value = match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => {
            if *value {
                "True".to_owned()
            } else {
                "False".to_owned()
            }
        }
        Value::Null => "None".to_owned(),
        other => other.to_string(),
    };
    let mut expanded = String::with_capacity(value.len());
    let mut previous_lower_or_digit = false;
    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_lower_or_digit {
            expanded.push('_');
        }
        expanded.push(character);
        previous_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    let mut normalized = String::with_capacity(expanded.len());
    let mut separator = false;
    for character in expanded.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_uppercase());
            separator = false;
        } else if !separator && !normalized.is_empty() {
            normalized.push('_');
            separator = true;
        }
    }
    while normalized.ends_with('_') {
        normalized.pop();
    }
    if normalized.is_empty() {
        "EMPTY".to_owned()
    } else {
        normalized
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn enum_string(value: &str) -> String {
    enum_token(&Value::String(value.to_owned()))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn predicate_symbol(value: &str) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut previous_lower_or_digit = false;
    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_lower_or_digit {
            expanded.push('_');
        }
        expanded.push(character);
        previous_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    let mut normalized = String::with_capacity(expanded.len());
    let mut separator = false;
    for character in expanded.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '\'' | '_' | '-') {
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        } else if !separator && !normalized.is_empty() {
            normalized.push('_');
            separator = true;
        }
    }
    while normalized.ends_with('_') {
        normalized.pop();
    }
    if normalized.is_empty() {
        "unnamed_relation".to_owned()
    } else {
        normalized
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn sort_name(value: &Value) -> String {
    let value = value
        .as_str()
        .unwrap_or_else(|| panic!("sort must be a string"));
    value
        .split('/')
        .map(|part| {
            let mut rendered = String::new();
            for word in part.split(|character: char| !character.is_ascii_alphanumeric()) {
                if word.is_empty() {
                    continue;
                }
                let mut characters = word.chars();
                if let Some(first) = characters.next() {
                    rendered.push(first.to_ascii_uppercase());
                    rendered.extend(characters);
                }
            }
            if rendered.is_empty() {
                "Unknown".to_owned()
            } else {
                rendered
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn flat_sort_name(value: &Value) -> String {
    sort_name(value)
        .rsplit('/')
        .next()
        .expect("sort_name is nonempty")
        .to_owned()
}

#[requires(true)]
#[ensures(ret == (value.as_object().is_some_and(|object| {
    !object.is_empty()
        && ["span", "text", "construct"]
            .iter()
            .any(|field| object.contains_key(*field))
})))]
fn is_source_record(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        !object.is_empty()
            && ["span", "text", "construct"]
                .iter()
                .any(|field| object.contains_key(*field))
    })
}

#[requires(true)]
#[ensures(true)]
fn json_pointer_escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// Append the graph pointers counted by the e25eeaf prototype's ID policy.
///
/// This deliberately walks every canonical JSON field except source records.
/// It is *not* the compact renderer's traversal graph: provenance, generated
/// inverse content, and other fields that compact SFN derives or waives still
/// affect prototype-compatible ID/share counts. Compact safety evidence is
/// captured separately by the real planning traversal.
#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn append_prototype_non_source_pointers(
    value: &Value,
    object_keys: &HashSet<String>,
    output: &mut Vec<String>,
) {
    match value {
        Value::String(value) if object_keys.contains(value) => output.push(value.clone()),
        Value::Array(items) => {
            for item in items {
                append_prototype_non_source_pointers(item, object_keys, output);
            }
        }
        Value::Object(object) => {
            for (field, item) in object {
                if field == "source" && is_source_record(item) {
                    continue;
                }
                append_prototype_non_source_pointers(item, object_keys, output);
            }
        }
        _ => {}
    }
}

/// Reproduce e25eeaf's raw non-source reference multiplicities exactly.
///
/// The synthetic root occurrence and duplicate pointers are intentional. These
/// counts decide which compact nodes receive prototype-compatible IDs; they do
/// not claim that every counted field is traversed by the compact renderer.
#[requires(objects.contains_key(root))]
#[requires(objects.keys().all(|key| object_keys.contains(key)))]
#[ensures(
    ret.len() == objects.len()
        && ret.keys().all(|key| object_keys.contains(key))
        && ret.get(root).is_some_and(|count| *count > 0)
)]
fn prototype_non_source_reference_counts(
    root: &str,
    objects: &Map<String, Value>,
    object_keys: &HashSet<String>,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> =
        object_keys.iter().map(|key| (key.clone(), 0)).collect();
    *counts.get_mut(root).expect("root belongs to objects") += 1;
    for object in objects.values() {
        let mut pointers = Vec::new();
        append_prototype_non_source_pointers(object, object_keys, &mut pointers);
        for pointer in pointers {
            *counts
                .get_mut(&pointer)
                .expect("prototype pointer walk only yields object keys") += 1;
        }
    }
    counts
}

#[requires(arguments.keys().all(|place| place.starts_with('x')))]
#[ensures(ret.len() == arguments.len())]
fn sorted_places(arguments: &Map<String, Value>) -> Vec<&str> {
    let mut places: Vec<&str> = arguments.keys().map(String::as_str).collect();
    places.sort_by_key(|place| {
        place
            .strip_prefix('x')
            .and_then(|number| number.parse::<usize>().ok())
            .filter(|number| *number > 0)
            .unwrap_or_else(|| panic!("noncanonical argument place: {place:?}"))
    });
    places
}

#[requires(place.starts_with('x'))]
#[ensures(!ret.is_empty())]
fn place_label(place: &str) -> &str {
    let number = place
        .strip_prefix('x')
        .filter(|number| {
            !number.is_empty()
                && !number.starts_with('0')
                && number.chars().all(|character| character.is_ascii_digit())
        })
        .unwrap_or_else(|| panic!("noncanonical argument place: {place:?}"));
    number
}

type Scope = Vec<String>;
type Ground = [String; 4];

/// One structural fact that prevents truthful use of the compact SFN vocabulary.
///
/// These are properties of the semantic graph, not renderer failures.  Keeping
/// them typed makes the compact/graph-form boundary exhaustive and inspectable.
#[invariant(::NonCanonicalGround { object, role } => !object.is_empty() && !role.is_empty())]
#[invariant(::MultipleBinderOwners { referent } => !referent.is_empty())]
#[invariant(::BinderDoesNotEncloseUse { referent, owner, use_site } =>
    !referent.is_empty() && !owner.is_empty() && !use_site.is_empty()
)]
#[invariant(::ScopeDependencyWithoutEnclosingBinder { referent, dependency } =>
    !referent.is_empty() && !dependency.is_empty()
)]
#[invariant(::NonCompactReferent { referent, field } =>
    !referent.is_empty() && !field.is_empty()
)]
#[invariant(::NonCompactFieldShape { object, field } =>
    !object.is_empty() && !field.is_empty()
)]
#[invariant(::NonCompactNameDescriptor { referent } => !referent.is_empty())]
#[invariant(::NonDerivableGeneratedContent { referent, content } =>
    !referent.is_empty() && !content.is_empty()
)]
#[invariant(::UnrepresentableCycle { entry } => !entry.is_empty())]
#[invariant(::RepeatedSingleUseEmission { object } => !object.is_empty())]
#[invariant(::PrototypeIdWithoutCompactUse { object } => !object.is_empty())]
#[invariant(::DefinitionSiteDoesNotDominateUse { object } => !object.is_empty())]
#[invariant(::DeclarationPlanningDidNotConverge { iterations } => *iterations > 0)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CompactIncompatibility {
    NonCanonicalGround {
        object: String,
        role: String,
    },
    MultipleBinderOwners {
        referent: String,
    },
    BinderDoesNotEncloseUse {
        referent: String,
        owner: String,
        use_site: String,
    },
    ScopeDependencyWithoutEnclosingBinder {
        referent: String,
        dependency: String,
    },
    NonCompactReferent {
        referent: String,
        field: String,
    },
    NonCompactFieldShape {
        object: String,
        field: String,
    },
    NonCompactNameDescriptor {
        referent: String,
    },
    NonDerivableGeneratedContent {
        referent: String,
        content: String,
    },
    UnrepresentableCycle {
        entry: String,
    },
    RepeatedSingleUseEmission {
        object: String,
    },
    PrototypeIdWithoutCompactUse {
        object: String,
    },
    DefinitionSiteDoesNotDominateUse {
        object: String,
    },
    DeclarationPlanningDidNotConverge {
        iterations: usize,
    },
}

impl CompactIncompatibility {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn kind(&self) -> &'static str {
        match self.as_data() {
            data!(CompactIncompatibility::NonCanonicalGround { .. }) => "NON-CANONICAL-GROUND",
            data!(CompactIncompatibility::MultipleBinderOwners { .. }) => "MULTIPLE-BINDER-OWNERS",
            data!(CompactIncompatibility::BinderDoesNotEncloseUse { .. }) => {
                "BINDER-DOES-NOT-ENCLOSE-USE"
            }
            data!(CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder { .. }) => {
                "SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER"
            }
            data!(CompactIncompatibility::NonCompactReferent { .. }) => "NON-COMPACT-REFERENT",
            data!(CompactIncompatibility::NonCompactFieldShape { .. }) => "NON-COMPACT-FIELD-SHAPE",
            data!(CompactIncompatibility::NonCompactNameDescriptor { .. }) => {
                "NON-COMPACT-NAME-DESCRIPTOR"
            }
            data!(CompactIncompatibility::NonDerivableGeneratedContent { .. }) => {
                "NON-DERIVABLE-GENERATED-CONTENT"
            }
            data!(CompactIncompatibility::UnrepresentableCycle { .. }) => "UNREPRESENTABLE-CYCLE",
            data!(CompactIncompatibility::RepeatedSingleUseEmission { .. }) => {
                "REPEATED-SINGLE-USE-EMISSION"
            }
            data!(CompactIncompatibility::PrototypeIdWithoutCompactUse { .. }) => {
                "PROTOTYPE-ID-WITHOUT-COMPACT-USE"
            }
            data!(CompactIncompatibility::DefinitionSiteDoesNotDominateUse { .. }) => {
                "DEFINITION-SITE-DOES-NOT-DOMINATE-USE"
            }
            data!(CompactIncompatibility::DeclarationPlanningDidNotConverge { .. }) => {
                "DECLARATION-PLANNING-DID-NOT-CONVERGE"
            }
        }
    }
}

/// The exact representation selected before rendering starts.
#[invariant(::Compact => true)]
#[invariant(::TypedGraph { incompatibilities } => !incompatibilities.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum XmlRepresentationPlan {
    Compact,
    TypedGraph {
        incompatibilities: BTreeSet<CompactIncompatibility>,
    },
}

impl XmlRepresentationPlan {
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(XmlRepresentationPlan::Compact)))]
    fn is_compact(&self) -> bool {
        matches!(self.as_data(), data!(XmlRepresentationPlan::Compact))
    }
}

#[cfg(test)]
#[invariant(
    *predication_adjuncts != *referent_arity,
    "a hostile renderer drops exactly one independent branch"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestRenderSuppression {
    predication_adjuncts: bool,
    referent_arity: bool,
}

// Validated once in `from_value`; fields are private and never mutated.
#[invariant(objects.contains_key(root), "the root must name a graph object")]
#[invariant(
    object_keys.len() == objects.len()
        && order.len() == objects.len()
        && ids.len() == objects.len()
        && prototype_non_source_reference_counts.len() == objects.len(),
    "all object-keyed indexes must cover the graph"
)]
#[expensive_invariant(
    object_keys.iter().all(|key| objects.contains_key(key))
        && objects.keys().all(|key| object_keys.contains(key))
        && order.keys().all(|key| object_keys.contains(key))
        && ids.keys().all(|key| object_keys.contains(key))
        && prototype_non_source_reference_counts
            .keys()
            .all(|key| object_keys.contains(key)),
    "all object-keyed indexes must have the same key domain"
)]
#[expensive_invariant(
    ids.values().collect::<HashSet<_>>().len() == ids.len(),
    "rendered graph ids must be unique"
)]
#[expensive_invariant(
    surface_paths
        .iter()
        .all(|surface| surface.path().starts_with("/objects/")),
    "the occurrence inventory covers canonical graph-object surfaces"
)]
#[derive(Debug)]
struct GraphData {
    root: String,
    objects: Map<String, Value>,
    object_keys: HashSet<String>,
    order: HashMap<String, usize>,
    ids: HashMap<String, String>,
    context_sites: HashMap<String, Vec<(String, String)>>,
    event_binding_owners: HashMap<String, String>,
    semantic_definition_owners: HashSet<String>,
    prototype_non_source_reference_counts: HashMap<String, usize>,
    representation: XmlRepresentationPlan,
    special_definition_keys: HashSet<String>,
    ordinary_definition_keys: HashSet<String>,
    ground_by_utterance: HashMap<String, Ground>,
    quantifier_restrictions: HashSet<(String, String)>,
    subtype_pairs: Vec<(String, String)>,
    value_paths: HashMap<usize, String>,
    surface_paths: BTreeSet<XmlSurface>,
}

impl GraphData {
    #[requires(true)]
    #[ensures(ret.objects.contains_key(&ret.root))]
    fn from_value(mut graph: Value) -> Self {
        let graph_object = graph
            .as_object_mut()
            .unwrap_or_else(|| panic!("semantic graph must serialize as an object"));
        assert_eq!(
            graph_object.get("version").and_then(Value::as_str),
            Some(SEMANTIC_JSON_VERSION),
            "unsupported semantic graph version"
        );
        let root = string_field(graph_object, "root").to_owned();
        let objects = match graph_object.remove("objects") {
            Some(Value::Object(objects)) => objects,
            _ => panic!("graph must contain an object map"),
        };
        assert!(objects.contains_key(&root), "missing root object: {root:?}");

        let object_keys: HashSet<String> = objects.keys().cloned().collect();
        let order: HashMap<String, usize> = objects
            .keys()
            .enumerate()
            .map(|(index, key)| (key.clone(), index))
            .collect();
        let ids: HashMap<String, String> = objects
            .iter()
            .map(|(key, object)| (key.clone(), make_id(key, json_object(object))))
            .collect();
        assert_eq!(
            ids.values().collect::<HashSet<_>>().len(),
            ids.len(),
            "generated SFN ids are not unique"
        );

        let prototype_non_source_reference_counts =
            prototype_non_source_reference_counts(&root, &objects, &object_keys);

        let mut context_sites: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut ground_by_utterance = HashMap::new();
        for (key, value) in &objects {
            let object = json_object(value);
            if optional_string(object, "type") != Some("utterance") {
                continue;
            }
            let ground = utterance_ground(object);
            for (role, referent) in ["SPEAKER", "AUDIENCE", "TIME", "PLACE"]
                .into_iter()
                .zip(ground.iter())
            {
                context_sites
                    .entry(referent.clone())
                    .or_default()
                    .push((key.clone(), role.to_owned()));
            }
            ground_by_utterance.insert(key.clone(), ground);
        }

        let mut event_binding_owner_sets: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut binder_owner_sets: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut quantifier_owner_sets: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut quantifier_restrictions = HashSet::new();
        let mut embedding_abstractions: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (owner, value) in &objects {
            if let Some(questions) = json_object(value)
                .get("embeddedQuestions")
                .and_then(Value::as_array)
            {
                for question in questions {
                    let question = question
                        .as_str()
                        .unwrap_or_else(|| panic!("embedded question must be an id"));
                    embedding_abstractions
                        .entry(question.to_owned())
                        .or_default()
                        .insert(owner.clone());
                }
            }
        }
        for (owner, value) in &objects {
            let object = json_object(value);
            if let Some(bound) = object.get("boundEventualities").and_then(Value::as_array) {
                for event in bound {
                    let event = event
                        .as_str()
                        .unwrap_or_else(|| panic!("bound eventuality must be an id"));
                    event_binding_owner_sets
                        .entry(event.to_owned())
                        .or_default()
                        .insert(owner.clone());
                    binder_owner_sets
                        .entry(event.to_owned())
                        .or_default()
                        .insert(owner.clone());
                }
            }
            if optional_string(object, "type") == Some("formula")
                && matches!(
                    optional_string(object, "operator"),
                    Some("exists" | "forall" | "cardinality")
                )
                && let Some(variable) = optional_string(object, "variable")
            {
                binder_owner_sets
                    .entry(variable.to_owned())
                    .or_default()
                    .insert(owner.clone());
                quantifier_owner_sets
                    .entry(variable.to_owned())
                    .or_default()
                    .insert(owner.clone());
                if let Some(restriction) = optional_string(object, "restriction") {
                    quantifier_restrictions.insert((variable.to_owned(), restriction.to_owned()));
                }
            }
            if let Some(parameters) = object.get("parameters").and_then(Value::as_array) {
                for parameter in parameters {
                    let parameter = parameter
                        .as_str()
                        .unwrap_or_else(|| panic!("parameter binder must be an id"));
                    binder_owner_sets
                        .entry(parameter.to_owned())
                        .or_default()
                        .insert(owner.clone());
                }
            }
            if optional_string(object, "type") == Some("question") {
                let slots = object.get("slots").map(json_array).unwrap_or_default();
                for slot in slots {
                    let slot = json_object(slot);
                    if slot.contains_key("parameter") {
                        let parameter = string_field(slot, "parameter");
                        assert_eq!(
                            objects
                                .get(parameter)
                                .and_then(Value::as_object)
                                .and_then(|parameter| optional_string(parameter, "type")),
                            Some("parameter"),
                            "question slot parameter must reference a parameter object"
                        );
                        let owners = binder_owner_sets.entry(parameter.to_owned()).or_default();
                        if let Some(abstractions) = embedding_abstractions.get(owner) {
                            let question_body = string_field(object, "body");
                            let mut matched_abstraction = false;
                            for abstraction in abstractions {
                                let abstraction_object =
                                    json_object(objects.get(abstraction).unwrap_or_else(|| {
                                        panic!("missing embedding abstraction: {abstraction:?}")
                                    }));
                                if ["body", "content"].into_iter().any(|field| {
                                    optional_string(abstraction_object, field)
                                        == Some(question_body)
                                }) {
                                    owners.insert(abstraction.clone());
                                    matched_abstraction = true;
                                }
                            }
                            if !matched_abstraction {
                                owners.insert(owner.clone());
                            }
                        } else {
                            owners.insert(owner.clone());
                        }
                    }
                }
            }
        }
        let event_binding_owners: HashMap<String, String> = event_binding_owner_sets
            .iter()
            .filter_map(|(referent, owners)| {
                owners
                    .iter()
                    .next()
                    .filter(|_| owners.len() == 1)
                    .map(|owner| (referent.clone(), owner.clone()))
            })
            .collect();
        let quantifier_owners: HashMap<String, String> = quantifier_owner_sets
            .iter()
            .filter_map(|(referent, owners)| {
                owners
                    .iter()
                    .next()
                    .filter(|_| owners.len() == 1)
                    .map(|owner| (referent.clone(), owner.clone()))
            })
            .collect();
        let representation = representation_plan(
            &root,
            &objects,
            &object_keys,
            &context_sites,
            &binder_owner_sets,
            &quantifier_restrictions,
        );
        let semantic_definition_owners: HashSet<String> = event_binding_owners
            .values()
            .cloned()
            .chain(quantifier_owners.values().cloned())
            .collect();
        let special_definition_keys: HashSet<String> = context_sites
            .keys()
            .chain(event_binding_owners.keys())
            .chain(quantifier_owners.keys())
            .cloned()
            .collect();
        let id_keys: HashSet<String> = objects
            .keys()
            .filter(|key| {
                prototype_non_source_reference_counts
                    .get(*key)
                    .copied()
                    .unwrap_or_default()
                    > 1
            })
            .cloned()
            .chain(special_definition_keys.iter().cloned())
            .collect();
        let ordinary_definition_keys = id_keys
            .difference(&special_definition_keys)
            .cloned()
            .collect();

        let subtype_pairs = subtype_pairs(&objects);
        let mut value_paths = HashMap::new();
        let mut surface_paths = BTreeSet::new();
        for (key, value) in &objects {
            index_value_paths(
                value,
                &format!("/objects/{}", json_pointer_escape(key)),
                &mut value_paths,
                &mut surface_paths,
            );
        }
        Self::from_data(data!(GraphData {
            root,
            objects,
            object_keys,
            order,
            ids,
            context_sites,
            event_binding_owners,
            semantic_definition_owners,
            prototype_non_source_reference_counts,
            representation,
            special_definition_keys,
            ordinary_definition_keys,
            ground_by_utterance,
            quantifier_restrictions,
            subtype_pairs,
            value_paths,
            surface_paths,
        }))
    }

    #[requires(self.objects.contains_key(key))]
    #[ensures(true)]
    fn object(&self, key: &str) -> &Map<String, Value> {
        json_object(
            self.objects
                .get(key)
                .unwrap_or_else(|| panic!("dangling graph pointer: {key:?}")),
        )
    }

    #[requires(self.ids.contains_key(key))]
    #[ensures(!ret.is_empty())]
    fn id(&self, key: &str) -> &str {
        self.ids
            .get(key)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("missing rendered id for {key:?}"))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn value_path(&self, value: &Map<String, Value>) -> &str {
        self.value_paths
            .get(&(value as *const Map<String, Value> as usize))
            .map(String::as_str)
            .unwrap_or_else(|| panic!("semantic record is absent from the path index"))
    }
}

/// Build the semantic-reference adjacency used for binder enclosure.
///
/// Like the prototype count policy, this includes every non-source pointer.
/// Unlike the counts it deduplicates parallel edges because reachability and
/// dominance depend only on the graph topology.
#[requires(objects.keys().all(|key| object_keys.contains(key)))]
#[ensures(
    ret.len() == objects.len()
        && ret.keys().all(|key| object_keys.contains(key))
        && ret
            .values()
            .all(|targets| targets.iter().all(|target| object_keys.contains(target)))
)]
fn semantic_reference_adjacency(
    objects: &Map<String, Value>,
    object_keys: &HashSet<String>,
) -> HashMap<String, BTreeSet<String>> {
    objects
        .iter()
        .map(|(key, value)| {
            let mut pointers = Vec::new();
            append_prototype_non_source_pointers(value, object_keys, &mut pointers);
            (key.clone(), pointers.into_iter().collect())
        })
        .collect()
}

/// An immutable indexed graph shared by the linear SCC and dominator analyses.
#[invariant(
    keys.len() == indexes.len()
        && keys.len() == successors.len()
        && keys.len() == predecessors.len(),
    "every reference-graph node must have all four index entries"
)]
#[expensive_invariant(
    keys.iter()
        .enumerate()
        .all(|(index, key)| indexes.get(key) == Some(&index)),
    "key and numeric indexes must be mutual inverses"
)]
#[expensive_invariant(
    successors
        .iter()
        .chain(predecessors.iter())
        .flatten()
        .all(|target| *target < keys.len()),
    "all indexed edges must stay within the graph"
)]
#[derive(Debug)]
struct ReferenceGraph {
    keys: Vec<String>,
    indexes: HashMap<String, usize>,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
}

impl ReferenceGraph {
    #[requires(true)]
    #[ensures(ret.keys.len() == old(keys.len()))]
    fn from_adjacency(keys: Vec<String>, adjacency: &HashMap<String, BTreeSet<String>>) -> Self {
        let indexes: HashMap<String, usize> = keys
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, key)| (key, index))
            .collect();
        assert_eq!(
            indexes.len(),
            keys.len(),
            "reference-graph keys must be unique"
        );
        assert!(
            adjacency.keys().all(|key| indexes.contains_key(key))
                && adjacency
                    .values()
                    .flatten()
                    .all(|target| indexes.contains_key(target)),
            "reference adjacency contains an unknown object"
        );

        let mut successors = vec![Vec::new(); keys.len()];
        let mut predecessors = vec![Vec::new(); keys.len()];
        for (key, targets) in adjacency {
            let source = indexes[key];
            for target in targets {
                let target = indexes[target];
                successors[source].push(target);
                predecessors[target].push(source);
            }
        }
        Self::from_data(data!(ReferenceGraph {
            keys,
            indexes,
            successors,
            predecessors,
        }))
    }

    #[requires(self.indexes.contains_key(key))]
    #[ensures(ret < self.keys.len())]
    fn index(&self, key: &str) -> usize {
        self.indexes[key]
    }

    #[requires(root < self.keys.len())]
    #[ensures(ret.len() == self.keys.len() && ret[root])]
    fn reachable_from(&self, root: usize) -> Vec<bool> {
        let mut reachable = vec![false; self.keys.len()];
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if reachable[node] {
                continue;
            }
            reachable[node] = true;
            pending.extend(self.successors[node].iter().copied());
        }
        reachable
    }

    /// Compute SCCs with iterative Kosaraju passes in O(vertices + edges).
    #[requires(true)]
    #[ensures(ret.component_by_node.len() == self.keys.len())]
    fn strongly_connected_components(&self) -> StrongComponents {
        let finish_order = depth_first_finish_order(&self.successors);
        let mut component_by_node = vec![usize::MAX; self.keys.len()];
        let mut components = Vec::new();
        for start in finish_order.into_iter().rev() {
            if component_by_node[start] != usize::MAX {
                continue;
            }
            let component_index = components.len();
            let mut component = Vec::new();
            let mut pending = vec![start];
            component_by_node[start] = component_index;
            while let Some(node) = pending.pop() {
                component.push(node);
                for predecessor in &self.predecessors[node] {
                    if component_by_node[*predecessor] == usize::MAX {
                        component_by_node[*predecessor] = component_index;
                        pending.push(*predecessor);
                    }
                }
            }
            components.push(component);
        }
        StrongComponents::from_data(data!(StrongComponents {
            component_by_node,
            components,
        }))
    }

    /// Compute one dominance relation for all enclosure queries.
    ///
    /// A virtual entry points at every compact document component root. The
    /// Lengauer-Tarjan semidominator pass computes the dominator tree once;
    /// Euler intervals then answer every binder-encloses-use query in O(1).
    #[requires(!roots.is_empty())]
    #[requires(roots.iter().all(|root| *root < self.keys.len()))]
    #[ensures(ret.entry.len() == self.keys.len() && ret.exit.len() == self.keys.len())]
    fn dominator_intervals(&self, roots: &[usize]) -> DominatorIntervals {
        let node_count = self.keys.len();
        let virtual_root = node_count;
        let total_nodes = node_count + 1;
        let mut unique_roots = roots.to_vec();
        unique_roots.sort_unstable();
        unique_roots.dedup();

        // Iterative DFS assigns the 1-based numbering required by the standard
        // semidominator algorithm without risking call-stack exhaustion.
        let mut dfs_number = vec![0usize; total_nodes];
        let mut vertex = vec![usize::MAX; total_nodes + 1];
        let mut parent = vec![0usize; total_nodes + 1];
        dfs_number[virtual_root] = 1;
        vertex[1] = virtual_root;
        let mut next_number = 2usize;
        let mut stack = vec![(virtual_root, 0usize)];
        while let Some((node, next_successor)) = stack.last_mut() {
            let successors = if *node == virtual_root {
                unique_roots.as_slice()
            } else {
                self.successors[*node].as_slice()
            };
            if *next_successor == successors.len() {
                stack.pop();
                continue;
            }
            let successor = successors[*next_successor];
            *next_successor += 1;
            if dfs_number[successor] != 0 {
                continue;
            }
            let number = next_number;
            next_number += 1;
            dfs_number[successor] = number;
            vertex[number] = successor;
            parent[number] = dfs_number[*node];
            stack.push((successor, 0));
        }
        let visited = next_number - 1;
        assert_eq!(
            visited, total_nodes,
            "document component roots must make every graph node reachable"
        );

        let mut predecessors = vec![Vec::new(); total_nodes + 1];
        for (source, targets) in self.successors.iter().enumerate() {
            for target in targets {
                predecessors[dfs_number[*target]].push(dfs_number[source]);
            }
        }
        for root in unique_roots {
            predecessors[dfs_number[root]].push(1);
        }

        let mut semi: Vec<usize> = (0..=total_nodes).collect();
        let mut label: Vec<usize> = (0..=total_nodes).collect();
        let mut ancestor = vec![0usize; total_nodes + 1];
        let mut immediate_dominator = vec![0usize; total_nodes + 1];
        let mut buckets = vec![Vec::new(); total_nodes + 1];
        for node in (2..=total_nodes).rev() {
            for predecessor in &predecessors[node] {
                let candidate = semidominator_eval(*predecessor, &mut ancestor, &mut label, &semi);
                semi[node] = semi[node].min(semi[candidate]);
            }
            buckets[semi[node]].push(node);
            ancestor[node] = parent[node];
            for pending in std::mem::take(&mut buckets[parent[node]]) {
                let candidate = semidominator_eval(pending, &mut ancestor, &mut label, &semi);
                immediate_dominator[pending] = if semi[candidate] < semi[pending] {
                    candidate
                } else {
                    parent[node]
                };
            }
        }
        for node in 2..=total_nodes {
            if immediate_dominator[node] != semi[node] {
                immediate_dominator[node] = immediate_dominator[immediate_dominator[node]];
            }
        }
        immediate_dominator[1] = 1;

        let mut dominator_children = vec![Vec::new(); total_nodes];
        for node in 2..=total_nodes {
            let graph_node = vertex[node];
            let dominator = vertex[immediate_dominator[node]];
            dominator_children[dominator].push(graph_node);
        }
        let mut entry = vec![0usize; total_nodes];
        let mut exit = vec![0usize; total_nodes];
        let mut clock = 0usize;
        let mut traversal = vec![(virtual_root, 0usize)];
        entry[virtual_root] = clock;
        clock += 1;
        while let Some((node, next_child)) = traversal.last_mut() {
            if *next_child == dominator_children[*node].len() {
                exit[*node] = clock;
                traversal.pop();
                continue;
            }
            let child = dominator_children[*node][*next_child];
            *next_child += 1;
            entry[child] = clock;
            clock += 1;
            traversal.push((child, 0));
        }
        entry.pop();
        exit.pop();
        DominatorIntervals::from_data(data!(DominatorIntervals { entry, exit }))
    }
}

#[invariant(
    component_by_node.len() == components.iter().map(Vec::len).sum::<usize>(),
    "every graph node belongs to exactly one strong component"
)]
#[expensive_invariant(
    component_by_node
        .iter()
        .enumerate()
        .all(|(node, component)| components
            .get(*component)
            .is_some_and(|members| members.contains(&node))),
    "the component index must agree with component membership"
)]
#[derive(Debug)]
struct StrongComponents {
    component_by_node: Vec<usize>,
    components: Vec<Vec<usize>>,
}

impl StrongComponents {
    #[requires(node < self.component_by_node.len())]
    #[ensures(ret < self.components.len())]
    fn component(&self, node: usize) -> usize {
        self.component_by_node[node]
    }

    #[requires(node < graph.keys.len())]
    #[requires(self.component_by_node.len() == graph.keys.len())]
    #[ensures(true)]
    fn node_is_cyclic(&self, graph: &ReferenceGraph, node: usize) -> bool {
        let component = &self.components[self.component(node)];
        component.len() > 1 || graph.successors[node].contains(&node)
    }
}

#[invariant(entry.len() == exit.len())]
#[expensive_invariant(
    entry
        .iter()
        .zip(exit.iter())
        .all(|(entry, exit)| entry < exit),
    "every dominator interval must be nonempty"
)]
#[derive(Debug)]
struct DominatorIntervals {
    entry: Vec<usize>,
    exit: Vec<usize>,
}

impl DominatorIntervals {
    #[requires(dominator < self.entry.len() && node < self.entry.len())]
    #[ensures(true)]
    fn dominates(&self, dominator: usize, node: usize) -> bool {
        self.entry[dominator] <= self.entry[node] && self.exit[node] <= self.exit[dominator]
    }
}

#[requires(
    successors
        .iter()
        .flatten()
        .all(|target| *target < successors.len())
)]
#[ensures(
    ret.len() == successors.len()
        && ret.iter().copied().collect::<HashSet<_>>().len() == ret.len()
)]
fn depth_first_finish_order(successors: &[Vec<usize>]) -> Vec<usize> {
    let mut seen = vec![false; successors.len()];
    let mut finished = Vec::with_capacity(successors.len());
    for start in 0..successors.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, next_successor)) = stack.last_mut() {
            if *next_successor == successors[*node].len() {
                finished.push(*node);
                stack.pop();
                continue;
            }
            let successor = successors[*node][*next_successor];
            *next_successor += 1;
            if !seen[successor] {
                seen[successor] = true;
                stack.push((successor, 0));
            }
        }
    }
    finished
}

#[requires(node > 0 && node < ancestor.len())]
#[requires(ancestor.len() == label.len() && label.len() == semi.len())]
#[ensures(ret > 0 && ret < label.len())]
fn semidominator_eval(
    node: usize,
    ancestor: &mut [usize],
    label: &mut [usize],
    semi: &[usize],
) -> usize {
    if ancestor[node] == 0 {
        return label[node];
    }
    let mut path = Vec::new();
    let mut current = node;
    while ancestor[ancestor[current]] != 0 {
        path.push(current);
        current = ancestor[current];
    }
    for current in path.into_iter().rev() {
        let parent = ancestor[current];
        if semi[label[parent]] < semi[label[current]] {
            label[current] = label[parent];
        }
        ancestor[current] = ancestor[parent];
    }
    label[node]
}

#[requires(objects.contains_key(event_key))]
#[ensures(true)]
fn generated_event_content_is_derivable(
    objects: &Map<String, Value>,
    event_key: &str,
    content_key: &str,
) -> bool {
    let Some(formula) = objects.get(content_key).and_then(Value::as_object) else {
        return false;
    };
    let Some(predication_key) = optional_string(formula, "predication") else {
        return false;
    };
    let Some(predication) = objects.get(predication_key).and_then(Value::as_object) else {
        return false;
    };
    optional_string(formula, "type") == Some("formula")
        && optional_string(formula, "operator") == Some("atom")
        && optional_string(predication, "type") == Some("predication")
        && optional_string(predication, "eventuality") == Some(event_key)
}

#[requires(true)]
#[ensures(true)]
fn has_noncompact_elided_restriction(
    value: &Value,
    quantifier_restrictions: &HashSet<(String, String)>,
) -> bool {
    match value {
        Value::Array(items) => items
            .iter()
            .any(|item| has_noncompact_elided_restriction(item, quantifier_restrictions)),
        Value::Object(object) => {
            let noncompact_here = optional_string(object, "value").is_some_and(|referent| {
                object
                    .get("relativeClauses")
                    .and_then(Value::as_array)
                    .is_some_and(|clauses| {
                        clauses.iter().map(json_object).any(|clause| {
                            optional_string(clause, "body").is_some_and(|body| {
                                quantifier_restrictions
                                    .contains(&(referent.to_owned(), body.to_owned()))
                                    && clause.keys().any(|field| {
                                        !matches!(
                                            field.as_str(),
                                            "kind" | "body" | "source" | "introducedBy"
                                        )
                                    })
                            })
                        })
                    })
            });
            noncompact_here
                || object
                    .values()
                    .any(|item| has_noncompact_elided_restriction(item, quantifier_restrictions))
        }
        _ => false,
    }
}

#[requires(objects.contains_key(root))]
#[requires(objects.keys().all(|key| object_keys.contains(key)))]
#[ensures(
    ret.is_compact()
        || matches!(
            ret.as_data(),
            data!(XmlRepresentationPlan::TypedGraph { .. })
        )
)]
fn representation_plan(
    root: &str,
    objects: &Map<String, Value>,
    object_keys: &HashSet<String>,
    context_sites: &HashMap<String, Vec<(String, String)>>,
    binder_owner_sets: &HashMap<String, BTreeSet<String>>,
    quantifier_restrictions: &HashSet<(String, String)>,
) -> XmlRepresentationPlan {
    let semantic_adjacency = semantic_reference_adjacency(objects, object_keys);
    let semantic_graph =
        ReferenceGraph::from_adjacency(objects.keys().cloned().collect(), &semantic_adjacency);
    let root_index = semantic_graph.index(root);
    let semantic_root_reachable = semantic_graph.reachable_from(root_index);
    // `render_graph_components` seeds every object outside the semantic root
    // component as a possible top-level component. Giving those objects direct
    // virtual-entry edges is the exact dominance model of that behavior: no
    // binder in one separately seeded component can enclose another component.
    let document_root_indexes: Vec<usize> = std::iter::once(root_index)
        .chain(
            semantic_graph
                .keys
                .iter()
                .enumerate()
                .filter(|(index, _)| !semantic_root_reachable[*index])
                .map(|(index, _)| index),
        )
        .collect();
    let dominators = semantic_graph.dominator_intervals(&document_root_indexes);
    let mut uses: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for (owner, references) in &semantic_adjacency {
        for referent in references {
            uses.entry(referent).or_default().insert(owner);
        }
    }

    let mut incompatibilities = BTreeSet::new();

    for (key, sites) in context_sites {
        let object = json_object(
            objects
                .get(key)
                .unwrap_or_else(|| panic!("ground refers to missing object {key:?}")),
        );
        for (_, role) in sites {
            let expected = match role.as_str() {
                "SPEAKER" => "speaker",
                "AUDIENCE" => "audience",
                "TIME" => "now",
                "PLACE" => "here",
                _ => unreachable!("context sites use the closed ground role vocabulary"),
            };
            if optional_string(object, "indexical") != Some(expected) {
                incompatibilities.insert(new!(CompactIncompatibility::NonCanonicalGround {
                    object: key.clone(),
                    role: role.clone(),
                }));
            }
            if object
                .get("target")
                .is_some_and(|target| target.as_str() != Some(key))
            {
                incompatibilities.insert(new!(CompactIncompatibility::NonCanonicalGround {
                    object: key.clone(),
                    role: role.clone(),
                }));
            }
            if object.keys().any(|field| {
                !matches!(
                    field.as_str(),
                    "type"
                        | "sort"
                        | "denotation"
                        | "category"
                        | "indexical"
                        | "target"
                        | "source"
                        | "assignedNames"
                )
            }) {
                incompatibilities.insert(new!(CompactIncompatibility::NonCanonicalGround {
                    object: key.clone(),
                    role: role.clone(),
                }));
            }
        }
    }

    for (referent, owners) in binder_owner_sets {
        if owners.len() != 1 {
            incompatibilities.insert(new!(CompactIncompatibility::MultipleBinderOwners {
                referent: referent.clone(),
            }));
            continue;
        }
        let owner = owners
            .iter()
            .next()
            .expect("one binder owner was established");
        for use_site in uses
            .get(referent.as_str())
            .into_iter()
            .flat_map(|sites| sites.iter())
            .filter(|use_site| **use_site != owner)
        {
            if !dominators.dominates(semantic_graph.index(owner), semantic_graph.index(use_site)) {
                incompatibilities.insert(new!(CompactIncompatibility::BinderDoesNotEncloseUse {
                    referent: referent.clone(),
                    owner: owner.clone(),
                    use_site: (*use_site).to_owned(),
                }));
            }
        }
    }

    for (key, value) in objects {
        let object = json_object(value);
        if optional_string(object, "type") == Some("referent") {
            let has_unique_binder = binder_owner_sets
                .get(key)
                .is_some_and(|owners| owners.len() == 1);
            if !has_unique_binder
                && !matches!(
                    optional_string(object, "denotation"),
                    None | Some("referential")
                )
            {
                incompatibilities.insert(new!(CompactIncompatibility::NonCompactReferent {
                    referent: key.clone(),
                    field: "denotation".to_owned(),
                }));
            }
            if !has_unique_binder
                && !matches!(
                    optional_string(object, "category"),
                    None | Some("constant" | "indexical" | "composite")
                )
            {
                incompatibilities.insert(new!(CompactIncompatibility::NonCompactReferent {
                    referent: key.clone(),
                    field: "category".to_owned(),
                }));
            }
            if let Some(descriptor) = object.get("descriptor").and_then(Value::as_object)
                && optional_string(descriptor, "kind") == Some("name")
                && (optional_string(descriptor, "name").is_none()
                    || optional_string(descriptor, "speaker").is_none())
            {
                incompatibilities.insert(new!(CompactIncompatibility::NonCompactNameDescriptor {
                    referent: key.clone(),
                }));
            }
            if optional_string(object, "denotation") == Some("generated-bound")
                && let Some(content) = optional_string(object, "content")
                && !generated_event_content_is_derivable(objects, key, content)
            {
                incompatibilities.insert(new!(
                    CompactIncompatibility::NonDerivableGeneratedContent {
                        referent: key.clone(),
                        content: content.to_owned(),
                    }
                ));
            }
        }

        if optional_string(object, "type") == Some("sequence")
            && object
                .get("relation")
                .is_some_and(|relation| relation.is_object() || relation.is_array())
        {
            incompatibilities.insert(new!(CompactIncompatibility::NonCompactFieldShape {
                object: key.clone(),
                field: "relation".to_owned(),
            }));
        }
        if has_noncompact_elided_restriction(value, quantifier_restrictions) {
            incompatibilities.insert(new!(CompactIncompatibility::NonCompactFieldShape {
                object: key.clone(),
                field: "relativeClauses".to_owned(),
            }));
        }

        let Some(scope) = object.get("scopeDependence").and_then(Value::as_object) else {
            continue;
        };
        let Some(dependencies) = scope.get("mayDependOn").and_then(Value::as_array) else {
            continue;
        };
        if dependencies.is_empty() {
            incompatibilities.insert(new!(
                CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder {
                    referent: key.clone(),
                    dependency: "EMPTY".to_owned(),
                }
            ));
        }
        for dependency in dependencies {
            let dependency = dependency
                .as_str()
                .unwrap_or_else(|| panic!("scope dependency must be an object id"));
            let enclosing = binder_owner_sets
                .get(dependency)
                .filter(|owners| owners.len() == 1)
                .and_then(|owners| owners.iter().next())
                .is_some_and(|owner| {
                    dominators.dominates(semantic_graph.index(owner), semantic_graph.index(key))
                });
            if !enclosing {
                incompatibilities.insert(new!(
                    CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder {
                        referent: key.clone(),
                        dependency: dependency.to_owned(),
                    }
                ));
            }
        }
    }

    if incompatibilities.is_empty() {
        new!(XmlRepresentationPlan::Compact)
    } else {
        new!(XmlRepresentationPlan::TypedGraph { incompatibilities })
    }
}

#[requires(!path.is_empty())]
#[ensures(true)]
fn index_value_paths(
    value: &Value,
    path: &str,
    value_paths: &mut HashMap<usize, String>,
    surface_paths: &mut BTreeSet<XmlSurface>,
) {
    match value {
        Value::Object(object) => {
            assert!(
                surface_paths.insert(object_surface(path.to_owned())),
                "one semantic object received multiple JSON paths"
            );
            assert!(
                value_paths
                    .insert(
                        object as *const Map<String, Value> as usize,
                        path.to_owned()
                    )
                    .is_none(),
                "one semantic record received multiple JSON paths"
            );
            for (field, value) in object {
                let field_path = format!("{path}/{}", json_pointer_escape(field));
                assert!(
                    surface_paths.insert(field_surface(field_path.clone())),
                    "one semantic field received multiple JSON paths"
                );
                index_value_paths(value, &field_path, value_paths, surface_paths);
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                index_value_paths(
                    value,
                    &format!("{path}/{index}"),
                    value_paths,
                    surface_paths,
                );
            }
        }
        _ => {}
    }
}

#[requires(!key.is_empty())]
#[ensures(!ret.is_empty())]
fn make_id(key: &str, object: &Map<String, Value>) -> String {
    let prefix = if optional_string(object, "type") == Some("referent") {
        if optional_string(object, "sort").is_some_and(|sort| sort.starts_with("eventuality")) {
            "e"
        } else {
            "r"
        }
    } else {
        match optional_string(object, "type") {
            Some("utterance") => "u",
            Some("predication") => "p",
            Some("formula") => "f",
            Some("quantity") => "q",
            Some("parameter") => "v",
            Some("sequence") => "s",
            Some("displayedContent") => "d",
            Some("mathExpression") => "m",
            Some("question") => "x",
            _ => "o",
        }
    };
    let suffix = key
        .rsplit_once(':')
        .map(|(_, suffix)| suffix)
        .filter(|suffix| suffix.chars().all(|character| character.is_ascii_digit()))
        .unwrap_or_else(|| panic!("graph key lacks JSON-aligned suffix: {key:?}"));
    format!("{prefix}{suffix}")
}

#[requires(true)]
#[ensures(ret.iter().all(|(subtype, supertype)| !subtype.is_empty() && !supertype.is_empty()))]
fn subtype_pairs(objects: &Map<String, Value>) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut seen = HashSet::new();
    for value in objects.values() {
        let object = json_object(value);
        let Some(sort) = object.get("sort") else {
            continue;
        };
        let path = sort_name(sort);
        let parts: Vec<&str> = path.split('/').collect();
        for pair in parts.windows(2) {
            let pair = (pair[1].to_owned(), pair[0].to_owned());
            if seen.insert(pair.clone()) {
                pairs.push(pair);
            }
        }
    }
    pairs
}

#[requires(true)]
#[ensures(true)]
fn utterance_ground(object: &Map<String, Value>) -> Ground {
    let deictic = object
        .get("deicticGround")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("utterance lacks deicticGround record"));
    assert_eq!(
        deictic.keys().map(String::as_str).collect::<HashSet<_>>(),
        HashSet::from(["time", "place"]),
        "utterance deicticGround must contain exactly time and place"
    );
    [
        string_field(object, "speaker").to_owned(),
        string_field(object, "audience").to_owned(),
        string_field(deictic, "time").to_owned(),
        string_field(deictic, "place").to_owned(),
    ]
}

// Deliberately mutable render/planning state. Each transition has local
// contracts and balanced-stack assertions.
#[invariant(true)]
#[derive(Debug)]
struct RenderState {
    defined: HashSet<String>,
    definition_sites: HashMap<String, usize>,
    emitted: HashSet<String>,
    active_fill_site: Option<(String, String)>,
    fill_marks: usize,
    declaration_scopes: HashMap<String, Scope>,
    scope_declarations: HashMap<Scope, Vec<String>>,
    scope_parent: HashMap<Scope, Option<Scope>>,
    scope_stack: Vec<Scope>,
    pointer_use_scopes: HashMap<String, Vec<Scope>>,
    first_use_order: HashMap<String, usize>,
    use_counter: usize,
    planning: bool,
    initial_planning_pass: bool,
    rendering_declaration: bool,
    ground_ids: HashMap<Ground, String>,
    ground_declaration_scopes: HashMap<Ground, Scope>,
    ground_scope_declarations: HashMap<Scope, Vec<Ground>>,
    ground_scope_parent: HashMap<Scope, Option<Scope>>,
    ground_scope_stack: Vec<Scope>,
    ground_pointer_use_scopes: HashMap<Ground, Vec<Scope>>,
    ground_first_use_order: HashMap<Ground, usize>,
    ground_use_counter: usize,
    planning_grounds: bool,
    defined_grounds: HashSet<Ground>,
    ground_definition_sites: HashMap<Ground, usize>,
    speaker_stack: Vec<String>,
    bound_variable_stack: Vec<String>,
    omissions: Vec<XmlOmission>,
    unaccounted_surfaces: BTreeSet<XmlSurface>,
    planning_incompatibilities: BTreeSet<CompactIncompatibility>,
    planning_definition_incompatibilities: BTreeSet<CompactIncompatibility>,
    planning_object_stack: Vec<String>,
    planning_compact_adjacency: HashMap<String, BTreeSet<String>>,
    planning_repeated_single_use: BTreeSet<String>,
    #[cfg(test)]
    test_suppression: Option<TestRenderSuppression>,
}

impl RenderState {
    #[requires(true)]
    #[ensures(ret.scope_stack.is_empty() && ret.ground_scope_stack.is_empty())]
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            definition_sites: HashMap::new(),
            emitted: HashSet::new(),
            active_fill_site: None,
            fill_marks: 0,
            declaration_scopes: HashMap::new(),
            scope_declarations: HashMap::new(),
            scope_parent: HashMap::new(),
            scope_stack: Vec::new(),
            pointer_use_scopes: HashMap::new(),
            first_use_order: HashMap::new(),
            use_counter: 0,
            planning: false,
            initial_planning_pass: false,
            rendering_declaration: false,
            ground_ids: HashMap::new(),
            ground_declaration_scopes: HashMap::new(),
            ground_scope_declarations: HashMap::new(),
            ground_scope_parent: HashMap::new(),
            ground_scope_stack: Vec::new(),
            ground_pointer_use_scopes: HashMap::new(),
            ground_first_use_order: HashMap::new(),
            ground_use_counter: 0,
            planning_grounds: false,
            defined_grounds: HashSet::new(),
            ground_definition_sites: HashMap::new(),
            speaker_stack: Vec::new(),
            bound_variable_stack: Vec::new(),
            omissions: Vec::new(),
            unaccounted_surfaces: BTreeSet::new(),
            planning_incompatibilities: BTreeSet::new(),
            planning_definition_incompatibilities: BTreeSet::new(),
            planning_object_stack: Vec::new(),
            planning_compact_adjacency: HashMap::new(),
            planning_repeated_single_use: BTreeSet::new(),
            #[cfg(test)]
            test_suppression: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn suppress_predication_adjuncts(&self) -> bool {
        #[cfg(test)]
        {
            self.test_suppression
                .as_ref()
                .is_some_and(|suppression| suppression.predication_adjuncts)
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn suppress_referent_arity(&self) -> bool {
        #[cfg(test)]
        {
            self.test_suppression
                .as_ref()
                .is_some_and(|suppression| suppression.referent_arity)
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    #[requires(true)]
    #[ensures(self.scope_stack.is_empty() && self.ground_scope_stack.is_empty())]
    fn reset_traversal_state(&mut self) {
        self.defined.clear();
        self.definition_sites.clear();
        self.emitted.clear();
        self.active_fill_site = None;
        self.fill_marks = 0;
        self.scope_stack.clear();
        self.ground_scope_stack.clear();
        self.defined_grounds.clear();
        self.ground_definition_sites.clear();
        self.speaker_stack.clear();
        self.bound_variable_stack.clear();
        self.omissions.clear();
        self.unaccounted_surfaces.clear();
        self.planning_object_stack.clear();
        self.planning_compact_adjacency.clear();
        self.planning_repeated_single_use.clear();
    }

    #[requires(true)]
    #[ensures(self.unaccounted_surfaces == graph.surface_paths)]
    fn start_omission_accounting(&mut self, graph: &GraphData) {
        assert!(!self.planning, "accounting cannot begin during planning");
        self.unaccounted_surfaces.clone_from(&graph.surface_paths);
    }

    #[requires(true)]
    #[ensures(true)]
    fn account_object(&mut self, graph: &GraphData, object: &Map<String, Value>) {
        if self.planning {
            return;
        }
        self.unaccounted_surfaces
            .remove(&object_surface(graph.value_path(object).to_owned()));
    }

    #[requires(object.contains_key(field))]
    #[ensures(true)]
    fn account_field(&mut self, graph: &GraphData, object: &Map<String, Value>, field: &str) {
        if self.planning {
            return;
        }
        self.account_object(graph, object);
        let path = format!(
            "{}/{}",
            graph.value_path(object),
            json_pointer_escape(field)
        );
        assert!(
            self.unaccounted_surfaces
                .remove(&field_surface(path.clone())),
            "semantic field was accounted more than once: {path}"
        );
    }

    #[requires(XML_DECLARED_WAIVERS.contains(&waiver))]
    #[ensures(self.planning || self.omissions.len() == old(self.omissions.len()) + 1)]
    fn record_omission(&mut self, waiver: XmlWaiverFamily, surface: XmlSurface) {
        if self.planning {
            return;
        }
        assert!(
            remove_surface_subtree(&mut self.unaccounted_surfaces, &surface),
            "omitted semantic surface was already accounted: {}",
            surface.path()
        );
        self.omissions.push(new!(XmlOmission {
            waiver: Some(waiver),
            surface,
        }));
    }

    #[requires(object.contains_key(field))]
    #[requires(XML_DECLARED_WAIVERS.contains(&waiver))]
    #[ensures(true)]
    fn record_field_omission(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
        field: &str,
        waiver: XmlWaiverFamily,
    ) {
        self.record_omission(
            waiver,
            field_surface(format!(
                "{}/{}",
                graph.value_path(object),
                json_pointer_escape(field)
            )),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn account_value_fields(&mut self, graph: &GraphData, value: &Value) {
        match value {
            Value::Array(items) => {
                for item in items {
                    self.account_value_fields(graph, item);
                }
            }
            Value::Object(object) => {
                self.account_object(graph, object);
                for (field, value) in object {
                    self.account_field(graph, object, field);
                    self.account_value_fields(graph, value);
                }
            }
            _ => {}
        }
    }

    #[requires(object.contains_key(field))]
    #[ensures(true)]
    fn account_field_tree(&mut self, graph: &GraphData, object: &Map<String, Value>, field: &str) {
        self.account_field(graph, object, field);
        self.account_value_fields(graph, &object[field]);
    }

    #[requires(true)]
    #[ensures(true)]
    fn observe_nested_provenance_omissions(&mut self, graph: &GraphData, value: &Value) {
        match value {
            Value::Array(items) => {
                for item in items {
                    self.observe_nested_provenance_omissions(graph, item);
                }
            }
            Value::Object(object) => {
                for (field, value) in object {
                    if field == "source" && is_source_record(value) {
                        self.record_field_omission(
                            graph,
                            object,
                            field,
                            XmlWaiverFamily::SourceRecord,
                        );
                        continue;
                    }
                    if field == "introducedBy" {
                        self.record_field_omission(
                            graph,
                            object,
                            field,
                            XmlWaiverFamily::IntroducedBy,
                        );
                    }
                    self.observe_nested_provenance_omissions(graph, value);
                }
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn observe_assigned_name_omissions(&mut self, graph: &GraphData, value: &Value) {
        let records = json_array(value);
        for record in records {
            let record_object = json_object(record);
            self.observe_nested_provenance_omissions(graph, record);
            self.record_omission(
                XmlWaiverFamily::AssignedNameRecord,
                object_surface(graph.value_path(record_object).to_owned()),
            );
        }
    }

    #[requires(true)]
    #[ensures(self.planning)]
    fn start_planning_pass(&mut self, initial: bool) {
        self.reset_traversal_state();
        self.planning_incompatibilities.clear();
        self.planning = true;
        self.initial_planning_pass = initial;
        self.scope_parent.clear();
        self.ground_scope_parent.clear();
        self.pointer_use_scopes.clear();
        self.use_counter = 0;
    }

    #[requires(self.planning)]
    #[requires(self.planning_object_stack.is_empty())]
    #[ensures(!self.planning && !self.initial_planning_pass)]
    fn finish_planning_pass(&mut self, graph: &GraphData) {
        if !self.planning_repeated_single_use.is_empty() {
            // These edges are not a separately maintained approximation. They
            // were recorded by `render_pointer_inner` while the real compact
            // planning renderer traversed each object. SCCs therefore explain
            // exact repeated-emission evidence without preclassifying benign
            // cycles or changing any compact traversal decision.
            let reference_graph = ReferenceGraph::from_adjacency(
                graph.objects.keys().cloned().collect(),
                &self.planning_compact_adjacency,
            );
            let components = reference_graph.strongly_connected_components();
            for key in &self.planning_repeated_single_use {
                let node = reference_graph.index(key);
                let incompatibility = if components.node_is_cyclic(&reference_graph, node) {
                    new!(CompactIncompatibility::UnrepresentableCycle { entry: key.clone() })
                } else {
                    new!(CompactIncompatibility::RepeatedSingleUseEmission {
                        object: key.clone(),
                    })
                };
                self.planning_definition_incompatibilities
                    .insert(incompatibility);
            }
        }
        self.planning = false;
        self.initial_planning_pass = false;
    }

    #[requires(true)]
    #[ensures(ret.first().is_none_or(|ancestor| ancestor == scope || self.scope_parent.contains_key(scope)))]
    fn scope_ancestors(&self, scope: &Scope) -> Vec<Scope> {
        ancestors(scope, &self.scope_parent, "scope")
    }

    #[requires(!scopes.is_empty())]
    #[ensures(scopes.iter().all(|scope| self.scope_ancestors(scope).contains(&ret)))]
    fn least_common_scope(&self, scopes: &[Scope]) -> Scope {
        least_common_scope(scopes, &self.scope_parent, "shared-node")
    }

    #[requires(true)]
    #[ensures(true)]
    fn rebuild_scope_declarations(&mut self, graph: &GraphData) {
        let mut grouped: HashMap<Scope, Vec<String>> = HashMap::new();
        for (key, scope) in &self.declaration_scopes {
            grouped.entry(scope.clone()).or_default().push(key.clone());
        }
        for keys in grouped.values_mut() {
            keys.sort_by_key(|key| {
                (
                    self.first_use_order
                        .get(key)
                        .copied()
                        .unwrap_or(graph.objects.len()),
                    graph.order[key],
                )
            });
        }
        self.scope_declarations = grouped;
    }

    #[requires(
        graph.ordinary_definition_keys.iter().all(|key| {
            graph
                .prototype_non_source_reference_counts
                .get(key)
                .is_some_and(|count| *count > 1)
        })
    )]
    #[ensures(
        !ret.is_empty()
            || graph
                .ordinary_definition_keys
                .iter()
                .all(|key| self.declaration_scopes.contains_key(key))
    )]
    fn plan_declaration_scopes(&mut self, graph: &GraphData) -> BTreeSet<CompactIncompatibility> {
        self.planning_definition_incompatibilities.clear();
        self.declaration_scopes.clear();
        self.scope_declarations.clear();
        self.start_planning_pass(true);
        let document_scope = vec!["document".to_owned()];
        let _ = self.scoped_parts(graph, document_scope, |state, graph| {
            state.render_root_component(graph)
        });
        self.finish_planning_pass(graph);
        if graph.ordinary_definition_keys.is_empty() {
            return self.planning_definition_incompatibilities.clone();
        }
        for key in &graph.ordinary_definition_keys {
            match self.pointer_use_scopes.get(key) {
                Some(scopes) if !scopes.is_empty() => {
                    self.declaration_scopes
                        .insert(key.clone(), self.least_common_scope(scopes));
                }
                _ => {
                    // Raw prototype pointer counts intentionally include fields
                    // that compact SFN may derive or waive. Requiring one use
                    // observed by the real planning renderer proves that every
                    // raw-count ID has an actual compact declaration/emission
                    // site; otherwise the graph form is selected.
                    self.planning_definition_incompatibilities.insert(new!(
                        CompactIncompatibility::PrototypeIdWithoutCompactUse {
                            object: key.clone(),
                        }
                    ));
                }
            }
        }
        if !self.planning_definition_incompatibilities.is_empty() {
            return self.planning_definition_incompatibilities.clone();
        }
        self.rebuild_scope_declarations(graph);

        let iteration_limit = graph.objects.len() + 1;
        for _ in 0..iteration_limit {
            let previous = self.declaration_scopes.clone();
            self.start_planning_pass(false);
            let document_scope = vec!["document".to_owned()];
            let _ = self.scoped_parts(graph, document_scope, |state, graph| {
                state.render_root_component(graph)
            });
            self.finish_planning_pass(graph);
            let mut planned = HashMap::new();
            for key in &graph.ordinary_definition_keys {
                match self.pointer_use_scopes.get(key) {
                    Some(scopes) if !scopes.is_empty() => {
                        planned.insert(key.clone(), self.least_common_scope(scopes));
                    }
                    _ => {
                        self.planning_definition_incompatibilities.insert(new!(
                            CompactIncompatibility::PrototypeIdWithoutCompactUse {
                                object: key.clone(),
                            }
                        ));
                    }
                }
            }
            if !self.planning_definition_incompatibilities.is_empty() {
                return self.planning_definition_incompatibilities.clone();
            }
            self.declaration_scopes = planned;
            self.rebuild_scope_declarations(graph);
            if self.declaration_scopes == previous {
                return BTreeSet::new();
            }
        }
        self.planning_definition_incompatibilities.insert(new!(
            CompactIncompatibility::DeclarationPlanningDidNotConverge {
                iterations: iteration_limit,
            }
        ));
        self.planning_definition_incompatibilities.clone()
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_scope(&mut self, scope: Scope) {
        let parent = self.scope_stack.last().cloned();
        if let Some(old_parent) = self.scope_parent.get(&scope) {
            assert_eq!(
                old_parent, &parent,
                "scope {scope:?} has inconsistent parents"
            );
        }
        self.scope_parent.insert(scope.clone(), parent);
        self.scope_stack.push(scope.clone());
        self.enter_ground_scope(scope);
    }

    #[requires(self.scope_stack.last() == Some(scope))]
    #[ensures(self.scope_stack.len() == old(self.scope_stack.len()) - 1)]
    fn leave_scope(&mut self, scope: &Scope) {
        self.leave_ground_scope(scope);
        assert_eq!(self.scope_stack.pop().as_ref(), Some(scope));
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_ground_scope(&mut self, scope: Scope) {
        let parent = self.ground_scope_stack.last().cloned();
        if let Some(old_parent) = self.ground_scope_parent.get(&scope) {
            assert_eq!(
                old_parent, &parent,
                "ground scope {scope:?} has inconsistent parents"
            );
        }
        self.ground_scope_parent.insert(scope.clone(), parent);
        self.ground_scope_stack.push(scope);
    }

    #[requires(self.ground_scope_stack.last() == Some(scope))]
    #[ensures(self.ground_scope_stack.len() == old(self.ground_scope_stack.len()) - 1)]
    fn leave_ground_scope(&mut self, scope: &Scope) {
        assert_eq!(self.ground_scope_stack.pop().as_ref(), Some(scope));
    }

    #[requires(true)]
    #[ensures(true)]
    fn plan_ground_scopes(&mut self, graph: &GraphData) {
        self.ground_pointer_use_scopes.clear();
        self.ground_first_use_order.clear();
        self.ground_use_counter = 0;
        self.ground_declaration_scopes.clear();
        self.ground_scope_declarations.clear();
        self.planning_grounds = true;
        self.start_planning_pass(false);
        let document_scope = vec!["document".to_owned()];
        let _ = self.scoped_parts(graph, document_scope, |state, graph| {
            state.render_graph_components(graph)
        });
        self.finish_planning_pass(graph);
        self.planning_grounds = false;

        let mut grounds: Vec<Ground> = self.ground_pointer_use_scopes.keys().cloned().collect();
        grounds.sort_by_key(|ground| self.ground_first_use_order[ground]);
        self.ground_ids = grounds
            .iter()
            .enumerate()
            .map(|(index, ground)| (ground.clone(), format!("g{}", index + 1)))
            .collect();
        for ground in grounds {
            let scopes = self
                .ground_pointer_use_scopes
                .get(&ground)
                .expect("planned ground has use scopes");
            let scope = least_common_scope(scopes, &self.ground_scope_parent, "GROUND");
            self.ground_declaration_scopes
                .insert(ground.clone(), scope.clone());
            self.ground_scope_declarations
                .entry(scope)
                .or_default()
                .push(ground);
        }
    }

    #[requires(self.planning_grounds)]
    #[requires(!self.ground_scope_stack.is_empty())]
    #[ensures(self.ground_pointer_use_scopes.contains_key(ground))]
    fn observe_ground_use(&mut self, ground: &Ground) {
        let scope = self
            .ground_scope_stack
            .last()
            .expect("precondition ensures a ground scope")
            .clone();
        self.ground_pointer_use_scopes
            .entry(ground.clone())
            .or_default()
            .push(scope);
        if !self.ground_first_use_order.contains_key(ground) {
            self.ground_first_use_order
                .insert(ground.clone(), self.ground_use_counter);
        }
        self.ground_use_counter += 1;
    }

    /// Include ordinary compact pointers to a speech-situation referent in the
    /// placement of every DEICTIC-GROUND that can define it.
    ///
    /// Planning provisionally marks context referents defined before walking
    /// the graph, so merely watching `GROUND=` attributes would miss an anchor
    /// or other pointer reached through an earlier scoped declaration. Recording
    /// those real pointer sites makes the ground declaration dominate all four
    /// constituent referents' uses, not only the owning utterance's `GROUND=`.
    #[requires(graph.objects.contains_key(key))]
    #[ensures(true)]
    fn observe_context_referent_use(&mut self, graph: &GraphData, key: &str) {
        if !self.planning_grounds {
            return;
        }
        let grounds: BTreeSet<Ground> = graph
            .context_sites
            .get(key)
            .into_iter()
            .flatten()
            .map(|(utterance, _)| {
                graph
                    .ground_by_utterance
                    .get(utterance)
                    .unwrap_or_else(|| {
                        panic!("context site lacks an utterance ground: {utterance:?}")
                    })
                    .clone()
            })
            .collect();
        for ground in grounds {
            self.observe_ground_use(&ground);
        }
    }

    #[requires(true)]
    #[ensures(!self.planning)]
    fn compact_planning_incompatibilities(
        &mut self,
        graph: &GraphData,
    ) -> BTreeSet<CompactIncompatibility> {
        self.start_planning_pass(false);
        let document_scope = vec!["document".to_owned()];
        let _ = self.scoped_parts(graph, document_scope, |state, graph| {
            state.render_graph_components(graph)
        });
        self.finish_planning_pass(graph);
        let mut incompatibilities = self.planning_incompatibilities.clone();
        incompatibilities.extend(self.planning_definition_incompatibilities.iter().cloned());
        incompatibilities
    }
}

impl RenderState {
    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_referent(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        if is_bare_zohe(object) && self.bound_variable_stack.is_empty() {
            for (field, value) in object {
                if field == "type" {
                    continue;
                } else if field == "source" && is_source_record(value) {
                    self.record_field_omission(graph, object, field, XmlWaiverFamily::SourceRecord);
                } else {
                    self.account_field_tree(graph, object, field);
                }
            }
            return XmlElement::with_attributes("REFERENCE", [("REF", "SOME")]);
        }
        let elided_zohe = object
            .get("descriptor")
            .and_then(Value::as_object)
            .is_some_and(|descriptor| {
                optional_string(descriptor, "kind") == Some("elided")
                    && optional_string(descriptor, "word") == Some("zo'e")
            });
        let mut handled = Vec::from([
            "type",
            "sort",
            "denotation",
            "scopeDependence",
            "descriptor",
            "assignedNames",
            "body",
            "content",
            "parameters",
            "arity",
            "kind",
            "category",
            "quotation",
            "intervalModifiers",
            "adjuncts",
            "personalMassMembership",
            "deicticReference",
            "generatedReferent",
        ]);
        handled.extend(FACET_FIELDS.iter().copied());
        if let Some(assigned_names) = object.get("assignedNames") {
            self.account_field(graph, object, "assignedNames");
            self.observe_assigned_name_omissions(graph, assigned_names);
        }
        if object.contains_key("denotation") {
            self.account_field(graph, object, "denotation");
        }
        assert!(
            matches!(
                optional_string(object, "denotation"),
                None | Some("referential")
            ),
            "a non-binder referent has a non-derivable denotation"
        );
        if object.contains_key("category") {
            self.account_field(graph, object, "category");
        }
        assert!(
            matches!(
                optional_string(object, "category"),
                None | Some("constant" | "indexical" | "composite")
            ),
            "a non-binder referent has a non-derivable category"
        );
        let result_kind = optional_string(object, "kind")
            .filter(|kind| *kind != "quotation")
            .map(enum_string);
        if object.contains_key("kind") {
            self.account_field(graph, object, "kind");
        }
        let tag = if elided_zohe {
            "UNSPECIFIED-REFERENT".to_owned()
        } else {
            let sort = object
                .get("sort")
                .map(flat_sort_name)
                .unwrap_or_else(|| "Unknown".to_owned());
            enum_string(&sort)
        };
        if object.contains_key("sort") {
            self.account_field(graph, object, "sort");
        }
        let mut result = XmlElement::new(tag);
        if let Some(kind) = result_kind {
            result.set("KIND", kind);
        }
        self.apply_facets(graph, &mut result, object);
        if let Some(scope) = object.get("scopeDependence").and_then(Value::as_object) {
            self.account_field(graph, object, "scopeDependence");
            self.apply_scope_dependence(graph, key, &mut result, scope);
        }
        if let Some(modifiers) = object.get("intervalModifiers").and_then(Value::as_array) {
            self.account_field(graph, object, "intervalModifiers");
            let mut rendered = XmlElement::new("INTERVAL-MODIFIERS");
            for modifier in modifiers {
                rendered.push(self.render_interval_modifier(graph, json_object(modifier)));
            }
            result.push(rendered);
        }
        if let Some(membership) = object
            .get("personalMassMembership")
            .and_then(Value::as_object)
        {
            self.account_field(graph, object, "personalMassMembership");
            result.push(self.render_personal_mass_membership(graph, membership));
        }
        if let Some(reference) = object.get("deicticReference").and_then(Value::as_object) {
            self.account_field(graph, object, "deicticReference");
            result.push(self.render_deictic_reference(graph, reference));
        }
        if let Some(generated) = object.get("generatedReferent").and_then(Value::as_object) {
            self.account_field(graph, object, "generatedReferent");
            result.push(self.render_generated_referent(graph, generated));
        }
        let mut descriptor_index = None;
        if !elided_zohe
            && let Some(descriptor) = object.get("descriptor").and_then(Value::as_object)
        {
            self.account_field(graph, object, "descriptor");
            descriptor_index = Some(result.children.len());
            result.push(self.render_descriptor(graph, descriptor, Some(key)));
        } else if elided_zohe && object.contains_key("descriptor") {
            self.account_field(graph, object, "descriptor");
            let descriptor = json_object(&object["descriptor"]);
            self.account_field(graph, descriptor, "kind");
            self.account_field(graph, descriptor, "word");
        }
        if let Some(body_key) = optional_string(object, "body") {
            self.account_field(graph, object, "body");
            let parameters = abstraction_parameters(graph, object, body_key);
            self.bound_variable_stack.extend(parameters.iter().cloned());
            let embedded_questions = embedded_questions_for_formula(graph, object, body_key);
            if !embedded_questions.is_empty() {
                self.account_field(graph, object, "embeddedQuestions");
                handled.push("embeddedQuestions");
            }
            let (declarations, (rendered_body, rendered_questions)) = self.scoped_parts(
                graph,
                vec!["description-body".to_owned(), key.to_owned()],
                |state, graph| {
                    let rendered_body = state.render_pointer(graph, body_key);
                    let rendered_questions: Vec<XmlElement> = embedded_questions
                        .iter()
                        .map(|question| state.render_pointer(graph, question))
                        .collect();
                    (rendered_body, rendered_questions)
                },
            );
            self.bound_variable_stack
                .truncate(self.bound_variable_stack.len() - parameters.len());
            let mut body = XmlElement::new("BODY");
            Self::append_defs(&mut body, declarations);
            body.push(rendered_body);
            if !rendered_questions.is_empty() {
                let mut questions = XmlElement::new("EMBEDDED-QUESTIONS");
                questions.extend(rendered_questions);
                body.push(questions);
            }
            let proposition =
                object.get("sort").map(flat_sort_name).as_deref() == Some("Proposition");
            if proposition && let Some(index) = descriptor_index {
                result.children[index].push(body);
            } else {
                result.push(body);
            }
        }
        if let Some(content) = optional_string(object, "content") {
            self.account_field(graph, object, "content");
            let parameters = abstraction_parameters(graph, object, content);
            self.bound_variable_stack.extend(parameters.iter().cloned());
            let embedded_questions = embedded_questions_for_formula(graph, object, content);
            if !embedded_questions.is_empty() {
                self.account_field(graph, object, "embeddedQuestions");
                handled.push("embeddedQuestions");
            }
            let (declarations, (rendered_content, rendered_questions)) = self.scoped_parts(
                graph,
                vec!["description-content".to_owned(), key.to_owned()],
                |state, graph| {
                    let rendered_content = state.render_pointer(graph, content);
                    let rendered_questions: Vec<XmlElement> = embedded_questions
                        .iter()
                        .map(|question| state.render_pointer(graph, question))
                        .collect();
                    (rendered_content, rendered_questions)
                },
            );
            self.bound_variable_stack
                .truncate(self.bound_variable_stack.len() - parameters.len());
            let mut rendered = XmlElement::new("CONTENT");
            Self::append_defs(&mut rendered, declarations);
            rendered.push(rendered_content);
            if !rendered_questions.is_empty() {
                let mut questions = XmlElement::new("EMBEDDED-QUESTIONS");
                questions.extend(rendered_questions);
                rendered.push(questions);
            }
            result.push(rendered);
        }
        if let Some(parameters) = object.get("parameters").and_then(Value::as_array) {
            self.account_field(graph, object, "parameters");
            result.set(
                "PARAMETERS",
                self.pointer_list(graph, parameters, "PARAMETERS"),
            );
        }
        if !self.suppress_referent_arity()
            && let Some(arity) = object.get("arity")
        {
            self.account_field(graph, object, "arity");
            result.set("ARITY", scalar_string(arity));
        }
        if let Some(quotation) = object.get("quotation").and_then(Value::as_object) {
            self.account_field(graph, object, "quotation");
            let mut rendered = XmlElement::new("QUOTATION");
            let mut quotation_handled = Vec::new();
            if let Some(mode) = quotation.get("mode") {
                self.account_field(graph, quotation, "mode");
                rendered.set("MODE", enum_token(mode));
                quotation_handled.push("mode");
            }
            if let Some(text) = optional_string(quotation, "text") {
                self.account_field(graph, quotation, "text");
                rendered.push(XmlElement::with_attributes("TEXT", [("VALUE", text)]));
                quotation_handled.push("text");
            }
            if let Some(delimiter) = optional_string(quotation, "delimiter") {
                self.account_field(graph, quotation, "delimiter");
                rendered.push(XmlElement::with_attributes(
                    "DELIMITER",
                    [("VALUE", delimiter)],
                ));
                quotation_handled.push("delimiter");
            }
            if let Some(utterance) = optional_string(quotation, "utterance") {
                self.account_field(graph, quotation, "utterance");
                rendered.push(self.wrap_pointer(graph, "UTTERANCE", utterance, Vec::new()));
                quotation_handled.push("utterance");
            }
            rendered.extend(self.extras(graph, quotation, &quotation_handled));
            result.push(rendered);
        }
        if let Some(adjuncts) = object.get("adjuncts").and_then(Value::as_array) {
            self.account_field(graph, object, "adjuncts");
            for adjunct in adjuncts {
                result.push(self.render_added_place(graph, json_object(adjunct), Some(key)));
            }
        }
        result.extend(self.extras(graph, object, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "QUANTITY")]
    fn render_quantity(&mut self, graph: &GraphData, object: &Map<String, Value>) -> XmlElement {
        let mut result = XmlElement::new("QUANTITY");
        if let Some(form) = object.get("form") {
            self.account_field(graph, object, "form");
            result.set("FORM", enum_token(form));
        }
        if let Some(scale) = object.get("scale") {
            self.account_field(graph, object, "scale");
            result.set("SCALE", enum_token(scale));
        }
        if let Some(value) = object.get("value") {
            self.account_field(graph, object, "value");
            if let Some(value_object) = value.as_object() {
                self.account_object(graph, value_object);
            }
            let rendered = if let Some(value) = value.as_object()
                && value.len() == 1
                && value.get("text").is_some_and(Value::is_string)
            {
                assert!(
                    object.contains_key("form"),
                    "quantity text cannot be provenance-only without FORM"
                );
                self.record_field_omission(graph, value, "text", XmlWaiverFamily::QuantityText);
                None
            } else if let Some(value) = value.as_object()
                && value.len() == 1
                && let Some(integer) = value.get("integer")
            {
                self.account_field(graph, value, "integer");
                Some(XmlElement::with_attributes(
                    "INTEGER",
                    [("VALUE", scalar_string(integer))],
                ))
            } else if let Some(value) = value.as_object()
                && value.len() == 1
                && let Some(expression) = optional_string(value, "mathExpression")
            {
                self.account_field(graph, value, "mathExpression");
                Some(self.render_pointer(graph, expression))
            } else {
                Some(self.generic_value(graph, value))
            };
            if let Some(rendered) = rendered {
                let mut value = XmlElement::new("VALUE");
                value.push(rendered);
                result.push(value);
            }
        }
        result.extend(self.extras(graph, object, &["type", "form", "scale", "value"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "PARAMETER")]
    fn render_parameter(&mut self, graph: &GraphData, object: &Map<String, Value>) -> XmlElement {
        let mut result = XmlElement::new("PARAMETER");
        if let Some(sort) = object.get("sort") {
            self.account_field(graph, object, "sort");
            result.set("SORT", flat_sort_name(sort));
        }
        if let Some(role) = object.get("role") {
            self.account_field(graph, object, "role");
            result.set("ROLE", enum_token(role));
        }
        if object.contains_key("introducedBy") {
            self.record_field_omission(
                graph,
                object,
                "introducedBy",
                XmlWaiverFamily::IntroducedBy,
            );
        }
        result.extend(self.extras(graph, object, &["type", "sort", "role", "introducedBy"]));
        result
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_sequence(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        if object.contains_key("boundEventualities") {
            self.account_field(graph, object, "boundEventualities");
        }
        let bound = object
            .get("boundEventualities")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        self.render_bound_sequence(graph, key, object, bound, 0)
    }

    #[requires(index <= bound.len())]
    #[ensures(!ret.name.is_empty())]
    fn render_bound_sequence(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
        bound: &[Value],
        index: usize,
    ) -> XmlElement {
        if index < bound.len() {
            let event_key = bound[index]
                .as_str()
                .unwrap_or_else(|| panic!("bound eventuality must be an id"));
            assert_eq!(
                graph
                    .event_binding_owners
                    .get(event_key)
                    .map(String::as_str),
                Some(key),
                "eventuality is not owned by this sequence: {event_key:?}"
            );
            let binder = self.render_binder_definition(graph, event_key, "EXISTS binder");
            let scope = vec![
                "event-quantifier-body".to_owned(),
                key.to_owned(),
                event_key.to_owned(),
            ];
            let (declarations, body) = self.scoped_parts(graph, scope, |state, graph| {
                state.render_bound_sequence(graph, key, object, bound, index + 1)
            });
            let mut result = XmlElement::new("EXISTS");
            result.push(binder);
            let mut body_node = XmlElement::new("BODY");
            Self::append_defs(&mut body_node, declarations);
            body_node.push(body);
            result.push(body_node);
            return result;
        }
        let scope = vec!["sequence".to_owned(), key.to_owned()];
        let (declarations, parts) = self.scoped_parts(graph, scope, |state, graph| {
            let mut handled = Vec::from(["type", "items", "relation", "boundEventualities"]);
            let mut items = Vec::new();
            if let Some(values) = object.get("items").and_then(Value::as_array) {
                state.account_field(graph, object, "items");
                for item in values {
                    items.push(
                        state.render_pointer(
                            graph,
                            item.as_str()
                                .unwrap_or_else(|| panic!("sequence item must be an id")),
                        ),
                    );
                }
            }
            let mut metadata = Vec::new();
            for field in ["connectionClaims", "nonlogicalConnection"] {
                if let Some(value) = object.get(field) {
                    state.account_field(graph, object, field);
                    let mut rendered = XmlElement::new(enum_string(field).replace('_', "-"));
                    rendered.push(state.generic_value(graph, value));
                    metadata.push(rendered);
                    handled.push(field);
                }
            }
            metadata.extend(state.extras(graph, object, &handled));
            (items, metadata)
        });
        let (items, metadata) = parts;
        if object.contains_key("relation") {
            self.account_field(graph, object, "relation");
        }
        let mut result = XmlElement::with_attributes(
            "SEQUENCE",
            [(
                "RELATION",
                object
                    .get("relation")
                    .map(enum_token)
                    .unwrap_or_else(|| "SEQUENCE".to_owned()),
            )],
        );
        Self::append_defs(&mut result, declarations);
        result.extend(items);
        if !metadata.is_empty() {
            let mut rendered = XmlElement::new("META");
            rendered.extend(metadata);
            result.push(rendered);
        }
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "DISPLAYED-CONTENT")]
    fn render_displayed_content(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new("DISPLAYED-CONTENT");
        let mut handled = Vec::from(["type"]);
        if let Some(relation) = optional_string(object, "relation") {
            self.account_field(graph, object, "relation");
            result.set("RELATION", predicate_symbol(relation));
            handled.push("relation");
        }
        for field in ["family", "polarity", "assertionEffect", "targetFocus"] {
            if let Some(value) = object.get(field) {
                self.account_field(graph, object, field);
                result.set(enum_string(field).replace('_', "-"), enum_token(value));
                handled.push(field);
            }
        }
        for field in ["experiencer", "target", "anchor"] {
            if let Some(pointer) = optional_string(object, field) {
                self.account_field(graph, object, field);
                result.push(self.wrap_pointer(
                    graph,
                    &enum_string(field).replace('_', "-"),
                    pointer,
                    Vec::new(),
                ));
                handled.push(field);
            }
        }
        result.extend(self.extras(graph, object, &handled));
        result
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_math_expression(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> XmlElement {
        if let Some(literal) = object.get("literal") {
            self.account_field(graph, object, "literal");
            if let Some(literal) = literal.as_object()
                && optional_string(literal, "kind") == Some("integer")
                && literal.len() == 2
            {
                self.account_value_fields(graph, &object["literal"]);
                let mut result = XmlElement::with_attributes(
                    "INTEGER",
                    [("VALUE", scalar_string(&literal["value"]))],
                );
                result.extend(self.extras(graph, object, &["type", "literal"]));
                return result;
            }
            let mut result = XmlElement::new("MATH");
            let mut rendered = XmlElement::new("LITERAL");
            rendered.push(self.generic_value(graph, literal));
            result.push(rendered);
            result.extend(self.extras(graph, object, &["type", "literal"]));
            return result;
        }
        let mut result = XmlElement::with_attributes(
            "MATH",
            [(
                "OPERATOR",
                object
                    .get("operator")
                    .map(enum_token)
                    .unwrap_or_else(|| "MATH".to_owned()),
            )],
        );
        if object.contains_key("operator") {
            self.account_field(graph, object, "operator");
        }
        if let Some(operands) = object.get("operands").and_then(Value::as_array) {
            self.account_field(graph, object, "operands");
            for operand in operands {
                result.push(
                    self.render_pointer(
                        graph,
                        operand
                            .as_str()
                            .unwrap_or_else(|| panic!("math operand must be an id")),
                    ),
                );
            }
        }
        result.extend(self.extras(graph, object, &["type", "operator", "operands"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "UNKNOWN")]
    fn render_unknown_object(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::with_attributes(
            "UNKNOWN",
            [("TYPE", optional_string(object, "type").unwrap_or("missing"))],
        );
        for (key, value) in object {
            if key == "type" {
                continue;
            }
            if key == "source" && is_source_record(value) {
                self.record_field_omission(graph, object, key, XmlWaiverFamily::SourceRecord);
                continue;
            }
            self.account_field(graph, object, key);
            let mut field = XmlElement::with_attributes("FIELD", [("NAME", key.as_str())]);
            field.push(self.generic_value(graph, value));
            result.push(field);
        }
        result
    }
}

#[requires(true)]
#[ensures(true)]
fn counted(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

#[requires(true)]
#[ensures(ret.iter().all(|message| !message.is_empty()))]
fn waiver_messages(omissions: &[XmlOmission]) -> Vec<String> {
    let count = |kind| {
        omissions
            .iter()
            .filter(|omission| omission.waiver == Some(kind))
            .count()
    };
    let mut messages = Vec::new();
    let source = count(XmlWaiverFamily::SourceRecord);
    if source > 0 {
        messages.push(format!(
            "*.source provenance ({}: spans, witness text, construct labels)",
            counted(source, "record")
        ));
    }
    let assigned = count(XmlWaiverFamily::AssignedNameRecord);
    if assigned > 0 {
        messages.push(format!(
            "*.assignedNames provenance ({})",
            counted(assigned, "record")
        ));
    }
    let descriptor = count(XmlWaiverFamily::DescriptorWord);
    if descriptor > 0 {
        messages.push(format!(
            "descriptor *.word provenance ({})",
            counted(descriptor, "field")
        ));
    }
    let introduced = count(XmlWaiverFamily::IntroducedBy);
    if introduced > 0 {
        messages.push(format!(
            "*.introducedBy provenance ({})",
            counted(introduced, "field")
        ));
    }
    let quantity = count(XmlWaiverFamily::QuantityText);
    if quantity > 0 {
        messages.push(format!(
            "quantity value text provenance ({})",
            counted(quantity, "field")
        ));
    }
    let bound = count(XmlWaiverFamily::BoundVariableWord);
    if bound > 0 {
        messages.push(format!(
            "bound-variable surface word provenance ({})",
            counted(bound, "field")
        ));
    }
    messages
}

#[requires(true)]
#[ensures(ret.name == "INCOMPATIBILITY")]
fn render_compact_incompatibility(reason: &CompactIncompatibility) -> XmlElement {
    let mut result = XmlElement::with_attributes("INCOMPATIBILITY", [("KIND", reason.kind())]);
    match reason.as_data() {
        data!(CompactIncompatibility::NonCanonicalGround { object, role }) => {
            result.set("OBJECT", object);
            result.set("ROLE", role);
        }
        data!(CompactIncompatibility::MultipleBinderOwners { referent }) => {
            result.set("REFERENT", referent);
        }
        data!(CompactIncompatibility::BinderDoesNotEncloseUse {
            referent,
            owner,
            use_site,
        }) => {
            result.set("REFERENT", referent);
            result.set("OWNER", owner);
            result.set("USE-SITE", use_site);
        }
        data!(
            CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder {
                referent,
                dependency,
            }
        ) => {
            result.set("REFERENT", referent);
            result.set("DEPENDENCY", dependency);
        }
        data!(CompactIncompatibility::NonCompactReferent { referent, field }) => {
            result.set("REFERENT", referent);
            result.set("FIELD", field);
        }
        data!(CompactIncompatibility::NonCompactFieldShape { object, field }) => {
            result.set("OBJECT", object);
            result.set("FIELD", field);
        }
        data!(CompactIncompatibility::NonCompactNameDescriptor { referent }) => {
            result.set("REFERENT", referent);
        }
        data!(CompactIncompatibility::NonDerivableGeneratedContent { referent, content }) => {
            result.set("REFERENT", referent);
            result.set("CONTENT", content);
        }
        data!(CompactIncompatibility::UnrepresentableCycle { entry }) => {
            result.set("ENTRY", entry);
        }
        data!(CompactIncompatibility::DefinitionSiteDoesNotDominateUse { object }) => {
            result.set("OBJECT", object);
        }
        data!(CompactIncompatibility::RepeatedSingleUseEmission { object }) => {
            result.set("OBJECT", object);
        }
        data!(CompactIncompatibility::PrototypeIdWithoutCompactUse { object }) => {
            result.set("OBJECT", object);
        }
        data!(CompactIncompatibility::DeclarationPlanningDidNotConverge { iterations }) => {
            result.set("ITERATIONS", iterations.to_string());
        }
    }
    result
}

impl RenderState {
    /// Render only the semantic root component.
    ///
    /// Declaration-scope planning deliberately uses this exact compact
    /// traversal rather than the later `UNREACHABLE` sweep. The prototype
    /// rejects an ordinary shared node with no use reachable from the semantic
    /// root; allowing the sweep to manufacture such a use can assign a stale
    /// declaration scope and prevent a later planning pass from making
    /// progress.
    #[requires(true)]
    #[ensures(true)]
    fn render_root_component(&mut self, graph: &GraphData) -> XmlElement {
        if self.planning {
            let mut ground_referents: Vec<String> = graph
                .context_sites
                .keys()
                .filter(|referent| !self.defined.contains(*referent))
                .cloned()
                .collect();
            ground_referents.sort_by_key(|referent| graph.id(referent).to_owned());
            for referent in ground_referents {
                if !self.defined.contains(&referent) {
                    let _ = self.define_at_site(graph, &referent, "planning DEICTIC-GROUND");
                }
            }
        }
        self.render_pointer(graph, &graph.root)
    }

    #[requires(true)]
    #[ensures(self.emitted == graph.object_keys)]
    fn render_graph_components(&mut self, graph: &GraphData) -> (XmlElement, Option<XmlElement>) {
        let mut unreachable = XmlElement::new("UNREACHABLE");
        let graph_root = self.render_root_component(graph);
        loop {
            let definition_owner = graph
                .objects
                .keys()
                .filter(|key| {
                    !self.emitted.contains(*key)
                        && graph.semantic_definition_owners.contains(*key)
                        && !graph.special_definition_keys.contains(*key)
                })
                .min_by_key(|key| graph.id(key))
                .cloned();
            let next = definition_owner.or_else(|| {
                graph
                    .objects
                    .keys()
                    .filter(|key| {
                        !self.emitted.contains(*key)
                            && !graph.special_definition_keys.contains(*key)
                    })
                    .min_by_key(|key| graph.id(key))
                    .cloned()
            });
            let Some(key) = next else {
                assert_eq!(
                    self.emitted, graph.object_keys,
                    "unrendered special nodes have no remaining semantic definition owner"
                );
                break;
            };
            unreachable.push(self.render_pointer(graph, &key));
        }
        let unreachable = (!unreachable.children.is_empty()).then_some(unreachable);
        (graph_root, unreachable)
    }

    #[requires(true)]
    #[ensures(ret.name == "WAIVERS")]
    fn waivers_element(&self) -> XmlElement {
        let mut waivers = XmlElement::new("WAIVERS");
        for message in waiver_messages(&self.omissions) {
            let mut waiver = XmlElement::new("WAIVER");
            waiver.text = Some(message);
            waivers.push(waiver);
        }
        waivers
    }

    #[requires(true)]
    #[ensures(self.unaccounted_surfaces.is_empty())]
    fn finish_omission_accounting(&mut self) {
        self.omissions.extend(
            std::mem::take(&mut self.unaccounted_surfaces)
                .into_iter()
                .map(|surface| {
                    new!(XmlOmission {
                        waiver: None,
                        surface,
                    })
                }),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_typed_graph_value(
        &mut self,
        graph: &GraphData,
        value: &Value,
        descriptor_variable: Option<bool>,
        quantity_value: bool,
    ) -> XmlElement {
        match value {
            Value::String(value) if graph.object_keys.contains(value) => {
                XmlElement::with_attributes(
                    "REFERENCE",
                    [("REF", graph.id(value)), ("KEY", value.as_str())],
                )
            }
            Value::String(value) => {
                XmlElement::with_attributes("STRING", [("VALUE", value.as_str())])
            }
            Value::Null => XmlElement::new("NULL"),
            Value::Bool(value) => {
                XmlElement::with_attributes("BOOLEAN", [("VALUE", value.to_string())])
            }
            Value::Number(value) => {
                XmlElement::with_attributes("NUMBER", [("VALUE", value.to_string())])
            }
            Value::Array(items) => {
                let mut list = XmlElement::new("LIST");
                for item in items {
                    let mut element = XmlElement::new("ITEM");
                    element.push(self.render_typed_graph_value(
                        graph,
                        item,
                        descriptor_variable,
                        quantity_value,
                    ));
                    list.push(element);
                }
                list
            }
            Value::Object(object) => self.render_typed_graph_record(
                graph,
                object,
                descriptor_variable,
                quantity_value,
                false,
            ),
        }
    }

    #[requires(!skip_type || object.contains_key("type"))]
    #[ensures(ret.name == "RECORD")]
    fn render_typed_graph_record(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
        descriptor_variable: Option<bool>,
        quantity_value: bool,
        skip_type: bool,
    ) -> XmlElement {
        self.account_object(graph, object);
        let mut record = XmlElement::new("RECORD");
        for (field, value) in object {
            if skip_type && field == "type" {
                continue;
            }
            if field == "source" && is_source_record(value) {
                self.record_field_omission(graph, object, field, XmlWaiverFamily::SourceRecord);
                continue;
            }
            if field == "assignedNames" {
                self.account_field(graph, object, field);
                self.observe_assigned_name_omissions(graph, value);
                continue;
            }
            if field == "introducedBy" {
                self.record_field_omission(graph, object, field, XmlWaiverFamily::IntroducedBy);
                continue;
            }
            if field == "word"
                && let Some(variable) = descriptor_variable
            {
                let kind = optional_string(object, "kind");
                if kind == Some("elided") && value.as_str() == Some("zo'e") {
                    self.account_field(graph, object, field);
                    continue;
                }
                if variable || kind != Some("proSumti") {
                    self.record_field_omission(
                        graph,
                        object,
                        field,
                        if variable && kind == Some("proSumti") {
                            XmlWaiverFamily::BoundVariableWord
                        } else {
                            XmlWaiverFamily::DescriptorWord
                        },
                    );
                    continue;
                }
                // A non-variable pro-sumti word is the unresolved referent's
                // only identifying surface, so both compact and typed-graph
                // forms fall through to the ordinary field renderer.
            }
            if quantity_value && field == "text" {
                self.record_field_omission(graph, object, field, XmlWaiverFamily::QuantityText);
                continue;
            }

            self.account_field(graph, object, field);
            let child_descriptor_variable = (field == "descriptor")
                .then(|| optional_string(object, "category") == Some("variable"));
            let child_quantity_value =
                field == "value" && optional_string(object, "type") == Some("quantity");
            let mut rendered = XmlElement::with_attributes("FIELD", [("NAME", field.as_str())]);
            rendered.push(self.render_typed_graph_value(
                graph,
                value,
                child_descriptor_variable,
                child_quantity_value,
            ));
            record.push(rendered);
        }
        record
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(ret.name == "OBJECT")]
    fn render_typed_graph_object(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        let object = graph.object(key);
        self.emitted.insert(key.to_owned());
        self.account_field(graph, object, "type");
        let mut result = XmlElement::with_attributes(
            "OBJECT",
            [
                ("ID", graph.id(key)),
                ("KEY", key),
                ("TYPE", string_field(object, "type")),
            ],
        );
        let record = self.render_typed_graph_record(graph, object, None, false, true);
        result.extend(record.children);
        result
    }

    #[requires(!incompatibilities.is_empty())]
    #[ensures(ret.ends_with('\n'))]
    fn render_typed_graph_document(
        &mut self,
        graph: &GraphData,
        document_name: &str,
        incompatibilities: &BTreeSet<CompactIncompatibility>,
    ) -> String {
        let mut typed_graph = XmlElement::with_attributes(
            "TYPED-GRAPH",
            [
                ("ROOT-REF", graph.id(&graph.root)),
                ("ROOT-KEY", graph.root.as_str()),
            ],
        );
        let mut keys: Vec<&str> = graph.objects.keys().map(String::as_str).collect();
        keys.sort_by_key(|key| graph.order[*key]);
        for key in keys {
            typed_graph.push(self.render_typed_graph_object(graph, key));
        }
        assert_eq!(
            self.emitted, graph.object_keys,
            "typed graph form must render every semantic object"
        );
        self.finish_omission_accounting();

        let mut root = XmlElement::with_attributes(
            "SFN",
            [
                ("VERSION", "0"),
                ("DOC", document_name),
                ("FORM", "TYPED-GRAPH"),
            ],
        );
        let mut key = XmlElement::new("KEY");
        for (topic, prose) in [
            (
                "form",
                "FORM=TYPED-GRAPH is selected exactly when the semantic graph cannot be represented truthfully by the compact SFN prototype vocabulary. It is a typed XML projection of the semantic graph, not a reinterpretation.",
            ),
            (
                "objects",
                "Each OBJECT is defined once by its canonical graph KEY= and XML ID=. ROOT-REF=/ROOT-KEY= identify the graph root. REFERENCE points to the exact shared object and never clones it.",
            ),
            (
                "fields",
                "Every non-waived semantic object and field occurrence is represented as typed OBJECT, FIELD, RECORD, LIST, ITEM, REFERENCE, STRING, NUMBER, BOOLEAN, or NULL structure. Child order follows canonical semantic JSON order.",
            ),
            (
                "elided-zohe",
                "A descriptor word is mechanically omitted only when descriptor KIND is elided and the value is exactly zo'e; every other descriptor word omission is reported by the existing descriptor-word waiver family.",
            ),
        ] {
            let mut rule = XmlElement::with_attributes("RULE", [("TOPIC", topic)]);
            rule.text = Some(prose.to_owned());
            key.push(rule);
        }
        let mut reasons = XmlElement::new("COMPACT-INCOMPATIBILITIES");
        for reason in incompatibilities {
            reasons.push(render_compact_incompatibility(reason));
        }
        key.push(reasons);
        root.push(key);
        root.push(self.waivers_element());
        root.push(typed_graph);
        serialize(&root)
    }

    #[requires(true)]
    #[ensures(ret.ends_with('\n'))]
    fn render_document(&mut self, graph: &GraphData, document_name: &str) -> String {
        let document_scope = vec!["document".to_owned()];
        let (document_declarations, components) =
            self.scoped_parts(graph, document_scope, |state, graph| {
                state.render_graph_components(graph)
            });
        let (graph_root, unreachable) = components;
        assert_eq!(
            self.emitted, graph.object_keys,
            "some graph objects were not rendered"
        );
        self.finish_omission_accounting();

        let mut root =
            XmlElement::with_attributes("SFN", [("VERSION", "0"), ("DOC", document_name)]);
        let mut key = XmlElement::new("KEY");
        for (topic, prose) in KEY_RULES_BEFORE_SORTS {
            let mut rule = XmlElement::with_attributes("RULE", [("TOPIC", *topic)]);
            rule.text = Some((*prose).to_owned());
            key.push(rule);
        }
        let facts = if graph.subtype_pairs.is_empty() {
            "none".to_owned()
        } else {
            graph
                .subtype_pairs
                .iter()
                .map(|(subtype, supertype)| format!("{subtype} ⊂ {supertype}"))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let mut sorts = XmlElement::with_attributes("RULE", [("TOPIC", "sorts")]);
        sorts.text = Some(format!(
            "SORT= values are flat PascalCase sort names. Subtype facts derived from encountered JSON sort paths: {facts}. LOCUTION implies sort Locution and therefore omits SORT=."
        ));
        key.push(sorts);
        for (topic, prose) in KEY_RULES_AFTER_SORTS {
            let mut rule = XmlElement::with_attributes("RULE", [("TOPIC", *topic)]);
            rule.text = Some((*prose).to_owned());
            key.push(rule);
        }
        root.push(key);
        root.push(self.waivers_element());
        Self::append_defs(&mut root, document_declarations);
        root.push(graph_root);
        if let Some(rendered) = unreachable {
            root.push(rendered);
        }
        serialize(&root)
    }
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value_with_state(
    graph: Value,
    document_name: &str,
    mut state: RenderState,
) -> XmlRender {
    let graph = GraphData::from_value(graph);
    let preliminary_incompatibilities = match graph.representation.as_data() {
        data!(XmlRepresentationPlan::Compact) => BTreeSet::new(),
        data!(XmlRepresentationPlan::TypedGraph { incompatibilities }) => incompatibilities.clone(),
    };
    let output = if !preliminary_incompatibilities.is_empty() {
        state.start_omission_accounting(&graph);
        state.render_typed_graph_document(&graph, document_name, &preliminary_incompatibilities)
    } else {
        let mut planning_incompatibilities = state.plan_declaration_scopes(&graph);
        if planning_incompatibilities.is_empty() {
            state.plan_ground_scopes(&graph);
            planning_incompatibilities = state.compact_planning_incompatibilities(&graph);
        }
        state.reset_traversal_state();
        state.start_omission_accounting(&graph);
        if planning_incompatibilities.is_empty() {
            state.render_document(&graph, document_name)
        } else {
            state.render_typed_graph_document(&graph, document_name, &planning_incompatibilities)
        }
    };
    let omissions = state.omissions;
    new!(XmlRender { output, omissions })
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value(graph: Value, document_name: &str) -> XmlRender {
    render_xml_value_with_state(graph, document_name, RenderState::new())
}

#[cfg(test)]
#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value_with_test_suppression(
    graph: Value,
    document_name: &str,
    suppression: TestRenderSuppression,
) -> XmlRender {
    let mut state = RenderState::new();
    state.test_suppression = Some(suppression);
    render_xml_value_with_state(graph, document_name, state)
}

/// Render a semantic graph as canonical SFN-XML.
///
/// `document_name` becomes the root `DOC=` value. The returned omission list
/// names every typed graph occurrence absent from ordinary XML. A `waiver` of
/// `None` exposes an unwaived omission rather than silently discarding an
/// unaccounted semantic surface.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
pub fn render_xml(graph: &SemanticGraph, document_name: &str) -> XmlRender {
    let value =
        serde_json::to_value(graph).expect("SemanticGraph's canonical serialization cannot fail");
    render_xml_value(value, document_name)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
    use sha2::{Digest, Sha256};

    use super::*;

    const XML_CORPUS_DOCS: &[&str] = &[
        "b13",
        "b14",
        "b15",
        "b16",
        "b17",
        "b18",
        "b19",
        "b21",
        "b22",
        "b23",
        "b24",
        "b25",
        "b26",
        "b27",
        "b28",
        "b29",
        "b30",
        "b31",
        "b32",
        "b33",
        "b34",
        "b35",
        "b36",
        "b37",
        "b38",
        "b39",
        "b40",
        "b41",
        "b42",
        "b43",
        "b44",
        "b45",
        "b46",
        "b47",
        "b48",
        "b49",
        "b50",
        "b51",
        "b52",
        "b53",
        "b54",
        "b55",
        "b57",
        "b58",
        "medium-quantified",
        "numeral-price",
        "paragraph-narrative",
        "small-mi-klama",
    ];

    #[requires(!document.is_empty() && !suffix.is_empty())]
    #[ensures(ret.ends_with(format!("{document}.{suffix}")))]
    fn fixture(document: &str, suffix: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/xml_corpus")
            .join(format!("{document}.{suffix}"))
    }

    #[requires(!document.is_empty())]
    #[ensures(ret.is_object())]
    fn graph(document: &str) -> Value {
        let path = fixture(document, "frozen.json");
        serde_json::from_slice(
            &std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
    }

    #[requires(!document.is_empty())]
    #[ensures(ret.is_object())]
    fn phaseb_graph(document: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/phaseb_corpus")
            .join(format!("{document}.frozen.json"));
        serde_json::from_slice(
            &std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
    }

    #[requires(!suffix.is_empty())]
    #[ensures(ret.len() == 64)]
    fn aggregate_hash(suffix: &str) -> String {
        let mut hasher = Sha256::new();
        for document in XML_CORPUS_DOCS {
            hasher.update(document.as_bytes());
            hasher.update(b"\n");
            hasher.update(
                std::fs::read(fixture(document, suffix))
                    .unwrap_or_else(|error| panic!("read {document}.{suffix}: {error}")),
            );
        }
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordered_surface_subtree_removal_matches_full_scan_oracle() {
        let paths = [
            "/objects/entity:1/source",
            "/objects/entity:1/source/span",
            "/objects/entity:1/source/span/byteStart",
            "/objects/entity:1/source-lexicographic-sibling",
            "/objects/entity:1/source0",
            "/objects/entity:1/sourc",
            "/objects/entity:2/source/span",
        ];
        let inventory: BTreeSet<XmlSurface> = paths
            .into_iter()
            .flat_map(|path| {
                [
                    object_surface(path.to_owned()),
                    field_surface(path.to_owned()),
                ]
            })
            .collect();

        for omitted in [
            object_surface("/objects/entity:1/source".to_owned()),
            field_surface("/objects/entity:1/source".to_owned()),
        ] {
            let mut oracle = inventory.clone();
            assert!(oracle.remove(&omitted));
            let descendant_prefix = format!("{}/", omitted.path());
            oracle.retain(|candidate| {
                candidate.path() != omitted.path()
                    && !candidate.path().starts_with(&descendant_prefix)
            });

            let mut indexed = inventory.clone();
            assert!(remove_surface_subtree(&mut indexed, &omitted));
            assert_eq!(indexed, oracle);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_graph_scc_and_dominance_are_structural_and_exact() {
        let keys = [
            "entry", "left", "right", "join", "cycle-a", "cycle-b", "orphan",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let adjacency = HashMap::from([
            (
                "entry".to_owned(),
                BTreeSet::from(["left".to_owned(), "right".to_owned()]),
            ),
            ("left".to_owned(), BTreeSet::from(["join".to_owned()])),
            ("right".to_owned(), BTreeSet::from(["join".to_owned()])),
            ("join".to_owned(), BTreeSet::from(["cycle-a".to_owned()])),
            ("cycle-a".to_owned(), BTreeSet::from(["cycle-b".to_owned()])),
            ("cycle-b".to_owned(), BTreeSet::from(["cycle-a".to_owned()])),
            ("orphan".to_owned(), BTreeSet::new()),
        ]);
        let graph = ReferenceGraph::from_adjacency(keys, &adjacency);
        let components = graph.strongly_connected_components();
        assert_eq!(
            components.component(graph.index("cycle-a")),
            components.component(graph.index("cycle-b"))
        );
        assert!(components.node_is_cyclic(&graph, graph.index("cycle-a")));
        assert!(!components.node_is_cyclic(&graph, graph.index("join")));

        let dominators = graph.dominator_intervals(&[graph.index("entry"), graph.index("orphan")]);
        assert!(dominators.dominates(graph.index("entry"), graph.index("join")));
        assert!(dominators.dominates(graph.index("join"), graph.index("cycle-b")));
        assert!(dominators.dominates(graph.index("cycle-a"), graph.index("cycle-b")));
        assert!(!dominators.dominates(graph.index("left"), graph.index("join")));
        assert!(!dominators.dominates(graph.index("entry"), graph.index("orphan")));
        assert!(dominators.dominates(graph.index("orphan"), graph.index("orphan")));
    }

    #[requires(successors.iter().flatten().all(|node| *node < successors.len()))]
    #[requires(roots.iter().all(|root| *root < successors.len()))]
    #[ensures(ret.len() == successors.len())]
    fn oracle_reachable_without(
        successors: &[Vec<usize>],
        roots: &[usize],
        blocked: Option<usize>,
    ) -> Vec<bool> {
        let mut reachable = vec![false; successors.len()];
        let mut pending = roots.to_vec();
        while let Some(node) = pending.pop() {
            if blocked == Some(node) || reachable[node] {
                continue;
            }
            reachable[node] = true;
            pending.extend(successors[node].iter().copied());
        }
        reachable
    }

    #[requires(successors.iter().flatten().all(|node| *node < successors.len()))]
    #[ensures(ret.iter().flatten().all(|node| *node < successors.len()))]
    fn oracle_strong_components(successors: &[Vec<usize>]) -> BTreeSet<Vec<usize>> {
        let reachability: Vec<Vec<bool>> = (0..successors.len())
            .map(|root| oracle_reachable_without(successors, &[root], None))
            .collect();
        let mut remaining: BTreeSet<usize> = (0..successors.len()).collect();
        let mut components = BTreeSet::new();
        while let Some(start) = remaining.pop_first() {
            let mut component = vec![start];
            let peers: Vec<usize> = remaining
                .iter()
                .copied()
                .filter(|candidate| {
                    reachability[start][*candidate] && reachability[*candidate][start]
                })
                .collect();
            for peer in peers {
                remaining.remove(&peer);
                component.push(peer);
            }
            components.insert(component);
        }
        components
    }

    #[requires(components.component_by_node.len() == graph.keys.len())]
    #[ensures(ret.iter().flatten().all(|node| *node < graph.keys.len()))]
    fn normalized_production_components(
        graph: &ReferenceGraph,
        components: &StrongComponents,
    ) -> BTreeSet<Vec<usize>> {
        components
            .components
            .iter()
            .map(|component| {
                let mut component = component.clone();
                component.sort_unstable();
                component
            })
            .collect()
    }

    #[requires(!successors.is_empty())]
    #[requires(successors.iter().flatten().all(|node| *node < successors.len()))]
    #[requires(
        root_sets.iter().all(|roots| {
            !roots.is_empty() && roots.iter().all(|root| *root < successors.len())
        })
    )]
    #[ensures(true)]
    fn assert_graph_algorithms_match_oracles(
        successors: &[Vec<usize>],
        root_sets: &[Vec<usize>],
        label: &str,
    ) {
        let keys: Vec<String> = (0..successors.len())
            .map(|node| format!("node:{node}"))
            .collect();
        let adjacency: HashMap<String, BTreeSet<String>> = successors
            .iter()
            .enumerate()
            .map(|(source, targets)| {
                (
                    keys[source].clone(),
                    targets.iter().map(|target| keys[*target].clone()).collect(),
                )
            })
            .collect();
        let graph = ReferenceGraph::from_adjacency(keys, &adjacency);
        let components = graph.strongly_connected_components();
        assert_eq!(
            normalized_production_components(&graph, &components),
            oracle_strong_components(successors),
            "SCC mismatch for {label}"
        );

        for roots in root_sets {
            let production = graph.dominator_intervals(roots);
            for dominator in 0..successors.len() {
                let reachable_without =
                    oracle_reachable_without(successors, roots, Some(dominator));
                for node in 0..successors.len() {
                    let expected = dominator == node || !reachable_without[node];
                    assert_eq!(
                        production.dominates(dominator, node),
                        expected,
                        "dominance mismatch for {label}, roots={roots:?}, dominator={dominator}, node={node}"
                    );
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn named_graph_topologies_match_independent_oracles() {
        let cases = [
            ("self-loop", vec![vec![0]], vec![vec![0]]),
            (
                "diamond",
                vec![vec![1, 2], vec![3], vec![3], vec![]],
                vec![vec![0]],
            ),
            ("cycle", vec![vec![1], vec![2], vec![1]], vec![vec![0]]),
            (
                "disconnected-multi-root",
                vec![vec![1], vec![], vec![3], vec![]],
                vec![vec![0, 2]],
            ),
        ];
        for (label, successors, roots) in cases {
            assert_graph_algorithms_match_oracles(&successors, &roots, label);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn graph_algorithms_match_independent_bounded_oracles() {
        // Four nodes cover every directed adjacency matrix while remaining a
        // bounded deterministic test: 65,536 graphs at the largest size.
        for node_count in 1..=4usize {
            let edge_slots = node_count * node_count;
            for edge_mask in 0usize..(1usize << edge_slots) {
                let mut successors = vec![Vec::new(); node_count];
                for source in 0..node_count {
                    for target in 0..node_count {
                        let edge = source * node_count + target;
                        if edge_mask & (1usize << edge) != 0 {
                            successors[source].push(target);
                        }
                    }
                }

                let mut root_sets = Vec::new();
                for primary_root in 0..node_count {
                    let primary_reachable =
                        oracle_reachable_without(&successors, &[primary_root], None);
                    let roots: Vec<usize> = std::iter::once(primary_root)
                        .chain(
                            primary_reachable
                                .iter()
                                .enumerate()
                                .filter(|(_, reachable)| !**reachable)
                                .map(|(node, _)| node),
                        )
                        .collect();
                    root_sets.push(roots);
                }
                assert_graph_algorithms_match_oracles(
                    &successors,
                    &root_sets,
                    &format!("n={node_count}, mask={edge_mask:#x}"),
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn planning_preflight_covers_single_use_cycles_and_raw_only_id_uses() {
        let single_use_cycle = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "root:1",
            "objects": {
                "root:1": {"type": "unknown"},
                "cycle:2": {"type": "unknown", "next": "cycle:3"},
                "cycle:3": {"type": "unknown", "next": "cycle:2"}
            }
        });
        let rendered = render_xml_value(single_use_cycle, "<single-use-cycle>");
        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(rendered.output.contains("KIND=\"UNREPRESENTABLE-CYCLE\""));

        let raw_only_id_use = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "entity:1",
            "objects": {
                "entity:1": {
                    "type": "referent",
                    "sort": "entity",
                    "denotation": "referential",
                    "category": "constant",
                    "assignedNames": [{
                        "first": "entity:2",
                        "second": "entity:2"
                    }]
                },
                "entity:2": {"type": "unknown"}
            }
        });
        let rendered = render_xml_value(raw_only_id_use, "<raw-only-id-use>");
        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(
            rendered
                .output
                .contains("KIND=\"PROTOTYPE-ID-WITHOUT-COMPACT-USE\"")
        );

        // If semantic binding metadata evolves onto an object whose compact
        // renderer has no corresponding definition site, preliminary topology
        // alone cannot reject the owner-local use. The real planning traversal
        // must still select typed form before final compact emission.
        let missing_definition_site = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "owner:1",
            "objects": {
                "owner:1": {
                    "type": "unknown",
                    "boundEventualities": ["eventuality:2"]
                },
                "eventuality:2": {
                    "type": "referent",
                    "sort": "eventuality",
                    "denotation": "referential"
                }
            }
        });
        let rendered = render_xml_value(missing_definition_site, "<missing-definition-site>");
        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(
            rendered
                .output
                .contains("KIND=\"DEFINITION-SITE-DOES-NOT-DOMINATE-USE\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn hostile_adjunct_drop_routes_shared_unreachable_graph_to_typed_form() {
        let graph = graph("b44");
        let expected = declared_waiver_occurrences(&graph);
        let rendered = render_xml_value_with_test_suppression(
            graph,
            "<hostile-adjunct-drop>",
            new!(TestRenderSuppression {
                predication_adjuncts: true,
                referent_arity: false,
            }),
        );

        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(
            rendered
                .output
                .contains("KIND=\"PROTOTYPE-ID-WITHOUT-COMPACT-USE\"")
        );
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected,
            "typed fallback must preserve every non-waived adjunct surface"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn hostile_scalar_drop_is_an_unwaived_occurrence() {
        let graph = graph("b13");
        let mut expected = declared_waiver_occurrences(&graph);
        expected.insert(new!(XmlOmission {
            waiver: None,
            surface: field_surface("/objects/relation:20/arity".to_owned()),
        }));
        let rendered = render_xml_value_with_test_suppression(
            graph,
            "<hostile-scalar-drop>",
            new!(TestRenderSuppression {
                predication_adjuncts: false,
                referent_arity: true,
            }),
        );

        assert!(!rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(!rendered.output.contains("ARITY=\"1\""));
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn descriptor_word_oracle_covers_elided_and_nonvariable_prosumti_edges() {
        let elided_non_zohe = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "entity:1",
            "objects": {
                "entity:1": {
                    "type": "referent",
                    "category": "constant",
                    "sort": "entity",
                    "descriptor": {"kind": "elided", "word": "zi'o"}
                }
            }
        });
        let expected = declared_waiver_occurrences(&elided_non_zohe);
        let rendered = render_xml_value(elided_non_zohe, "<elided-non-zohe>");
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(expected.contains(&new!(XmlOmission {
            waiver: Some(XmlWaiverFamily::DescriptorWord),
            surface: field_surface("/objects/entity:1/descriptor/word".to_owned()),
        })));

        let nonvariable_prosumti = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "entity:1",
            "objects": {
                "entity:1": {
                    "type": "referent",
                    "denotation": "generated-bound",
                    "category": "constant",
                    "sort": "entity",
                    "descriptor": {"kind": "proSumti", "word": "ko'a"}
                }
            }
        });
        let expected = declared_waiver_occurrences(&nonvariable_prosumti);
        let rendered = render_xml_value(nonvariable_prosumti, "<nonvariable-prosumti>");
        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(rendered.output.contains("<STRING VALUE=\"ko'a\"/>"));
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(expected.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_golden_typed_branches_have_structural_witnesses() {
        let deictic = render_xml_value(phaseb_graph("ti-mo"), "<deictic-witness>");
        assert!(
            deictic
                .output
                .contains("<DEICTIC-REFERENCE PROXIMITY=\"PROXIMAL\"")
        );
        assert!(
            deictic
                .omissions
                .iter()
                .all(|omission| omission.waiver.is_some())
        );

        let personal_mass =
            render_xml_value(phaseb_graph("modal-fronted-vao"), "<personal-mass-witness>");
        assert!(personal_mass.output.contains("<PERSONAL-MASS-MEMBERSHIP>"));
        assert!(
            personal_mass
                .output
                .contains("<ADJUNCT PREDICATE=\"vanbi\">")
        );
        assert!(
            personal_mass
                .omissions
                .iter()
                .all(|omission| omission.waiver.is_some())
        );

        let synthetic = serde_json::json!({
            "version": SEMANTIC_JSON_VERSION,
            "root": "entity:1",
            "objects": {
                "entity:1": {
                    "type": "referent",
                    "category": "constant",
                    "sort": "entity",
                    "intervalModifiers": [{
                        "kind": "aspect",
                        "value": {"contour": "initiative"}
                    }],
                    "generatedReferent": {
                        "realization": "explicit",
                        "specificity": "specific"
                    },
                    "adjuncts": [{"witness": "referent-level"}]
                }
            }
        });
        let synthetic = render_xml_value(synthetic, "<synthetic-typed-branches>");
        assert!(synthetic.output.contains("<INTERVAL-MODIFIERS>"));
        assert!(
            synthetic.output.contains(
                "<GENERATED-REFERENT REALIZATION=\"EXPLICIT\" SPECIFICITY=\"SPECIFIC\"/>"
            )
        );
        assert!(synthetic.output.contains("<FIELD NAME=\"witness\">"));
        assert!(synthetic.omissions.is_empty());
    }

    #[requires(true)]
    #[ensures(output.len() >= old(output.len()))]
    fn collect_declared_waiver_occurrences(
        value: &Value,
        path: &str,
        descriptor_variable: Option<bool>,
        output: &mut BTreeSet<XmlOmission>,
    ) {
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    collect_declared_waiver_occurrences(
                        item,
                        &format!("{path}/{index}"),
                        None,
                        output,
                    );
                }
            }
            Value::Object(object) => {
                if let Some(variable) = descriptor_variable
                    && let Some(word) = object.get("word").and_then(Value::as_str)
                {
                    let kind = optional_string(object, "kind");
                    let waiver = if kind == Some("elided") && word == "zo'e" {
                        None
                    } else if !variable && kind == Some("proSumti") {
                        None
                    } else if variable && kind == Some("proSumti") {
                        Some(XmlWaiverFamily::BoundVariableWord)
                    } else {
                        Some(XmlWaiverFamily::DescriptorWord)
                    };
                    if let Some(waiver) = waiver {
                        output.insert(new!(XmlOmission {
                            waiver: Some(waiver),
                            surface: field_surface(format!("{path}/word")),
                        }));
                    }
                }
                if optional_string(object, "type") == Some("quantity")
                    && let Some(quantity) = object.get("value").and_then(Value::as_object)
                    && quantity.len() == 1
                    && quantity.get("text").is_some_and(Value::is_string)
                {
                    output.insert(new!(XmlOmission {
                        waiver: Some(XmlWaiverFamily::QuantityText),
                        surface: field_surface(format!("{path}/value/text")),
                    }));
                }
                for (field, item) in object {
                    let field_path = format!("{path}/{}", json_pointer_escape(field));
                    if field == "source" && is_source_record(item) {
                        output.insert(new!(XmlOmission {
                            waiver: Some(XmlWaiverFamily::SourceRecord),
                            surface: field_surface(field_path.clone()),
                        }));
                    }
                    if field == "introducedBy" {
                        output.insert(new!(XmlOmission {
                            waiver: Some(XmlWaiverFamily::IntroducedBy),
                            surface: field_surface(field_path.clone()),
                        }));
                    }
                    if field == "assignedNames"
                        && let Some(records) = item.as_array()
                    {
                        for index in 0..records.len() {
                            output.insert(new!(XmlOmission {
                                waiver: Some(XmlWaiverFamily::AssignedNameRecord),
                                surface: object_surface(format!("{field_path}/{index}")),
                            }));
                        }
                    }
                    collect_declared_waiver_occurrences(
                        item,
                        &field_path,
                        (field == "descriptor")
                            .then(|| optional_string(object, "category") == Some("variable")),
                        output,
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn declared_waiver_occurrences(graph: &Value) -> BTreeSet<XmlOmission> {
        let mut occurrences = BTreeSet::new();
        collect_declared_waiver_occurrences(graph, "", None, &mut occurrences);
        occurrences
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn frozen_xml_corpus_is_exact_and_pinned() {
        assert_eq!(XML_CORPUS_DOCS.len(), 48);
        let expected: BTreeSet<String> = XML_CORPUS_DOCS
            .iter()
            .flat_map(|document| {
                [
                    format!("{document}.frozen.json"),
                    format!("{document}.xml.txt"),
                ]
            })
            .collect();
        let actual: BTreeSet<String> =
            std::fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xml_corpus"))
                .expect("read XML corpus directory")
                .filter_map(|entry| {
                    let entry = entry.expect("read XML corpus entry");
                    entry
                        .file_type()
                        .expect("read XML corpus file type")
                        .is_file()
                        .then(|| entry.file_name().to_string_lossy().into_owned())
                })
                .filter(|name| name != "PROVENANCE.md")
                .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            aggregate_hash("frozen.json"),
            "69ea08a65aba19049f65070b9eb045361834ddfbd2773da972c047be325381b3"
        );
        assert_eq!(
            aggregate_hash("xml.txt"),
            "220c7b2e2d73ae4b0f98ba7e2927bc4108b351bc04267e040012eaa55e0ce3fd"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xml_matches_the_frozen_prototype_on_all_48_documents() {
        let mut mismatches = Vec::new();
        for document in XML_CORPUS_DOCS {
            let expected = std::fs::read_to_string(fixture(document, "xml.txt"))
                .unwrap_or_else(|error| panic!("read {document}.xml.txt: {error}"));
            let actual = render_xml_value(graph(document), document);
            if actual.output != expected {
                let first = expected
                    .lines()
                    .zip(actual.output.lines())
                    .position(|(expected, actual)| expected != actual)
                    .map_or(1, |index| index + 1);
                mismatches.push(format!("{document}: first differing line {first}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{}/{} SFN-XML documents diverged:\n{}",
            mismatches.len(),
            XML_CORPUS_DOCS.len(),
            mismatches.join("\n")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_parameters_share_scope_dependence_binder_semantics() {
        let default = render_xml_value(graph("b58"), "<parameter-default>");
        assert!(!default.output.contains("SAME-FOR-ALL=\"true\""));
        assert!(!default.output.contains("POSSIBLY-DIFFERENT-PER=\""));

        let mut fixed = graph("b58");
        fixed["objects"]["entity:8"]["scopeDependence"] = serde_json::json!({"kind": "fixed"});
        let fixed = render_xml_value(fixed, "<parameter-fixed>");
        assert!(
            fixed
                .output
                .contains("<UNSPECIFIED-REFERENT SAME-FOR-ALL=\"true\"/>")
        );

        let mut subset = graph("b58");
        subset["objects"]["parameter:99"] = serde_json::json!({
            "type": "parameter",
            "sort": "entity",
            "role": "argumentQuestion"
        });
        subset["objects"]["question:11"]["slots"]
            .as_array_mut()
            .expect("b58 question slots")
            .push(serde_json::json!({
                "parameter": "parameter:99",
                "role": "answer"
            }));
        let subset = render_xml_value(subset, "<parameter-subset>");
        assert!(subset.output.contains("POSSIBLY-DIFFERENT-PER=\"v7\""));
        assert!(!subset.output.contains("FORM=\"TYPED-GRAPH\""));

        let mut distinct_question_body = graph("b58");
        distinct_question_body["objects"]["formula:99"] =
            distinct_question_body["objects"]["formula:10"].clone();
        distinct_question_body["objects"]["formula:99"]
            .as_object_mut()
            .expect("cloned b58 formula")
            .remove("boundEventualities");
        distinct_question_body["objects"]["question:11"]["body"] =
            Value::String("formula:99".to_owned());
        let distinct_question_body =
            render_xml_value(distinct_question_body, "<distinct-question-body>");
        assert!(
            distinct_question_body
                .output
                .contains("FORM=\"TYPED-GRAPH\"")
        );
        assert!(
            distinct_question_body
                .output
                .contains("BINDER-DOES-NOT-ENCLOSE-USE")
        );

        let mut malformed = graph("b58");
        malformed["objects"]["question:11"]["slots"][0]["parameter"] = Value::Number(7.into());
        assert!(
            std::panic::catch_unwind(|| render_xml_value(malformed, "<malformed-slot>")).is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_content_and_direct_question_scopes_are_byte_pinned() {
        let mut content_abstraction = graph("b58");
        let formula = content_abstraction["objects"]["proposition:12"]
            .as_object_mut()
            .expect("b58 abstraction")
            .remove("body")
            .expect("b58 abstraction body");
        content_abstraction["objects"]["proposition:12"]["content"] = formula;
        content_abstraction["objects"]["proposition:12"]["sort"] =
            Value::String("eventuality".to_owned());
        let content_abstraction = render_xml_value(content_abstraction, "<nu-content-witness>")
            .into_data()
            .output;
        assert!(!content_abstraction.contains("FORM=\"TYPED-GRAPH\""));
        assert!(content_abstraction.contains("<CONTENT>"));
        assert!(content_abstraction.contains("<EMBEDDED-QUESTIONS>"));
        assert_eq!(
            format!("{:x}", Sha256::digest(content_abstraction.as_bytes())),
            "4615f37c9f6f54276bb40fc9a2aed07abff9c95de27a2fa0dc962410bdf0e082"
        );

        let mut direct_question = graph("b58");
        direct_question["objects"]["utterance:5"]["content"] =
            Value::String("question:11".to_owned());
        direct_question["objects"]
            .as_object_mut()
            .expect("b58 objects")
            .remove("proposition:12");
        let direct_question = render_xml_value(direct_question, "<direct-question-witness>")
            .into_data()
            .output;
        assert!(!direct_question.contains("FORM=\"TYPED-GRAPH\""));
        assert!(direct_question.contains("<UNKNOWN TYPE=\"question\">"));
        assert_eq!(
            format!("{:x}", Sha256::digest(direct_question.as_bytes())),
            "26d32fcbe81ce58d1f493ac4954e573b2ef9de1ddc071121d92374a5a72de274"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn omissions_are_exactly_the_declared_waiver_families() {
        assert_eq!(
            XML_DECLARED_WAIVERS,
            &[
                XmlWaiverFamily::SourceRecord,
                XmlWaiverFamily::AssignedNameRecord,
                XmlWaiverFamily::DescriptorWord,
                XmlWaiverFamily::IntroducedBy,
                XmlWaiverFamily::QuantityText,
                XmlWaiverFamily::BoundVariableWord,
            ]
        );
        let mut counts: BTreeMap<XmlWaiverFamily, usize> = BTreeMap::new();
        let mut documents: BTreeMap<XmlWaiverFamily, BTreeSet<&str>> = BTreeMap::new();
        for document in XML_CORPUS_DOCS {
            let graph = graph(document);
            let expected = declared_waiver_occurrences(&graph);
            let rendered = render_xml_value(graph, document);
            let actual: BTreeSet<XmlOmission> = rendered.omissions.iter().cloned().collect();
            assert_eq!(
                actual, expected,
                "{document}: observed omissions differ from independently expanded waivers"
            );
            assert_eq!(
                actual.len(),
                rendered.omissions.len(),
                "{document}: duplicate observed omission"
            );
            for omission in actual {
                let waiver = omission.waiver.unwrap_or_else(|| {
                    panic!(
                        "{document}: unwaived omission at {}",
                        omission.surface.path()
                    )
                });
                assert!(
                    XML_DECLARED_WAIVERS.contains(&waiver),
                    "{document}: unwaived omission at {}",
                    omission.surface.path()
                );
                *counts.entry(waiver).or_default() += 1;
                documents.entry(waiver).or_default().insert(document);
            }
        }
        assert_eq!(
            counts,
            BTreeMap::from([
                (XmlWaiverFamily::SourceRecord, 624),
                (XmlWaiverFamily::AssignedNameRecord, 3),
                (XmlWaiverFamily::DescriptorWord, 55),
                (XmlWaiverFamily::IntroducedBy, 234),
                (XmlWaiverFamily::QuantityText, 11),
                (XmlWaiverFamily::BoundVariableWord, 9),
            ])
        );
        assert_eq!(counts.values().sum::<usize>(), 936);
        assert_eq!(
            documents
                .into_iter()
                .map(|(family, documents)| (family, documents.len()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                (XmlWaiverFamily::SourceRecord, 48),
                (XmlWaiverFamily::AssignedNameRecord, 2),
                (XmlWaiverFamily::DescriptorWord, 35),
                (XmlWaiverFamily::IntroducedBy, 45),
                (XmlWaiverFamily::QuantityText, 7),
                (XmlWaiverFamily::BoundVariableWord, 6),
            ])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn occurrence_ledger_renders_unknown_fields_and_exposes_silent_drops() {
        let mut unknown = graph("b13");
        unknown["objects"]["utterance:5"]["futureSemanticField"] =
            Value::String("must-render".to_owned());
        let rendered = render_xml_value(unknown, "ledger-unknown");
        assert!(
            rendered
                .output
                .contains("<FIELD NAME=\"futureSemanticField\">")
        );
        assert!(
            rendered
                .omissions
                .iter()
                .all(|omission| omission.waiver.is_some())
        );

        let mut wrong_shape = graph("b13");
        wrong_shape["objects"]["utterance:5"]["eventuality"] = Value::Object(Map::new());
        let rendered = render_xml_value(wrong_shape, "ledger-unaccounted");
        assert!(rendered.omissions.contains(&new!(XmlOmission {
            waiver: None,
            surface: field_surface("/objects/utterance:5/eventuality".to_owned()),
        })));
        assert!(rendered.omissions.contains(&new!(XmlOmission {
            waiver: None,
            surface: object_surface("/objects/utterance:5/eventuality".to_owned()),
        })));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xml_rendering_is_deterministic_on_all_48_documents() {
        for document in XML_CORPUS_DOCS {
            let graph = graph(document);
            assert_eq!(
                render_xml_value(graph.clone(), document),
                render_xml_value(graph, document),
                "{document}"
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn ancestors(scope: &Scope, parents: &HashMap<Scope, Option<Scope>>, label: &str) -> Vec<Scope> {
    let mut ancestors = Vec::new();
    let mut current = Some(scope.clone());
    let mut seen = HashSet::new();
    while let Some(scope) = current {
        assert!(
            seen.insert(scope.clone()),
            "{label} parent cycle at {scope:?}"
        );
        ancestors.push(scope.clone());
        current = parents.get(&scope).cloned().flatten();
    }
    ancestors.reverse();
    ancestors
}

#[requires(!scopes.is_empty())]
#[ensures(scopes.iter().all(|scope| ancestors(scope, parents, label).contains(&ret)))]
fn least_common_scope(
    scopes: &[Scope],
    parents: &HashMap<Scope, Option<Scope>>,
    label: &str,
) -> Scope {
    let paths: Vec<Vec<Scope>> = scopes
        .iter()
        .map(|scope| ancestors(scope, parents, label))
        .collect();
    let shortest = paths.iter().map(Vec::len).min().expect("scopes nonempty");
    let mut common = None;
    for index in 0..shortest {
        let candidate = &paths[0][index];
        if paths.iter().all(|path| &path[index] == candidate) {
            common = Some(candidate.clone());
        } else {
            break;
        }
    }
    common.unwrap_or_else(|| panic!("{label} uses have no enclosing scope: {scopes:?}"))
}

impl RenderState {
    #[requires(graph.objects.contains_key(key))]
    #[ensures(graph.ids.contains_key(key))]
    fn define_at_site(&mut self, graph: &GraphData, key: &str, site: &str) -> String {
        assert!(
            !self.defined.contains(key) && !self.emitted.contains(key),
            "graph node was defined before its {site} site: {key:?}"
        );
        assert!(
            graph.special_definition_keys.contains(key)
                || graph.ordinary_definition_keys.contains(key),
            "definition site lacks an id: {key:?}"
        );
        self.defined.insert(key.to_owned());
        self.emitted.insert(key.to_owned());
        *self.definition_sites.entry(key.to_owned()).or_default() += 1;
        graph.id(key).to_owned()
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(true)]
    fn render_declaration(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        assert!(
            !self.defined.contains(key) && !self.emitted.contains(key),
            "duplicate scoped declaration: {key:?}"
        );
        self.defined.insert(key.to_owned());
        self.emitted.insert(key.to_owned());
        *self.definition_sites.entry(key.to_owned()).or_default() += 1;
        let old = self.rendering_declaration;
        self.rendering_declaration = true;
        let mut rendered = self.render_object(graph, key);
        self.rendering_declaration = old;
        rendered.prepend_attributes(vec![("ID".to_owned(), graph.id(key).to_owned())]);
        rendered
    }

    #[requires(true)]
    #[ensures(true)]
    fn scoped_parts<T>(
        &mut self,
        graph: &GraphData,
        scope: Scope,
        build_body: impl FnOnce(&mut Self, &GraphData) -> T,
    ) -> (Vec<XmlElement>, T) {
        self.enter_scope(scope.clone());
        let mut declarations = self.ground_declarations(graph, &scope);
        let graph_declarations = self
            .scope_declarations
            .get(&scope)
            .cloned()
            .unwrap_or_default();
        declarations.extend(
            graph_declarations
                .iter()
                .map(|key| self.render_declaration(graph, key)),
        );
        let body = build_body(self, graph);
        self.leave_scope(&scope);
        (declarations, body)
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_ground_declaration(&mut self, graph: &GraphData, ground: &Ground) -> XmlElement {
        assert!(
            !self.defined_grounds.contains(ground),
            "duplicate DEICTIC-GROUND declaration: {ground:?}"
        );
        let mut result = XmlElement::new("DEICTIC-GROUND");
        result.set(
            "ID",
            self.ground_ids
                .get(ground)
                .unwrap_or_else(|| panic!("ground lacks a rendered id: {ground:?}"))
                .clone(),
        );
        for ((role, expected), key) in [
            ("SPEAKER-REF", "speaker"),
            ("AUDIENCE-REF", "audience"),
            ("TIME-REF", "now"),
            ("PLACE-REF", "here"),
        ]
        .into_iter()
        .zip(ground)
        {
            let object = graph.object(key);
            assert_eq!(
                optional_string(object, "indexical"),
                Some(expected),
                "{role} ground role disagrees with graph binding: {key:?}"
            );
            result.set(role, graph.id(key));
            if !self.defined.contains(key) {
                for (field, value) in object {
                    if field == "source" && is_source_record(value) {
                        self.record_field_omission(
                            graph,
                            object,
                            field,
                            XmlWaiverFamily::SourceRecord,
                        );
                    } else if field == "assignedNames" {
                        self.account_field(graph, object, field);
                        self.observe_assigned_name_omissions(graph, value);
                    } else if matches!(
                        field.as_str(),
                        "type" | "sort" | "denotation" | "category" | "indexical"
                    ) {
                        self.account_field(graph, object, field);
                    } else if field == "target" && value.as_str() == Some(key) {
                        self.account_field(graph, object, field);
                    }
                }
                self.define_at_site(graph, key, &format!("{role} DEICTIC-GROUND attribute"));
            }
        }
        self.defined_grounds.insert(ground.clone());
        *self
            .ground_definition_sites
            .entry(ground.clone())
            .or_default() += 1;
        result
    }

    #[requires(true)]
    #[ensures(true)]
    fn ground_declarations(&mut self, graph: &GraphData, scope: &Scope) -> Vec<XmlElement> {
        self.ground_scope_declarations
            .get(scope)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|ground| self.render_ground_declaration(graph, ground))
            .collect()
    }

    #[requires(!self.ground_scope_stack.is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_ground_reference(&mut self, graph: &GraphData, ground: &Ground) -> String {
        if self.planning_grounds {
            self.observe_ground_use(ground);
        }

        for ((role, expected), key) in [
            ("SPEAKER-REF", "speaker"),
            ("AUDIENCE-REF", "audience"),
            ("TIME-REF", "now"),
            ("PLACE-REF", "here"),
        ]
        .into_iter()
        .zip(ground)
        {
            assert!(
                graph.objects.contains_key(key),
                "dangling {role} ground pointer: {key:?}"
            );
            assert!(
                graph.context_sites.contains_key(key),
                "{role} pointer lacks a ground definition: {key:?}"
            );
            assert_eq!(
                optional_string(graph.object(key), "indexical"),
                Some(expected),
                "{role} ground role disagrees with graph binding: {key:?}"
            );
            if !self.defined.contains(key) {
                assert!(
                    self.planning,
                    "{role} referent used before its DEICTIC-GROUND: {key:?}"
                );
                self.define_at_site(graph, key, &format!("{role} planning DEICTIC-GROUND"));
            }
        }
        if !self.planning {
            let declaration_scope = self
                .ground_declaration_scopes
                .get(ground)
                .unwrap_or_else(|| panic!("GROUND lacks declaration scope: {ground:?}"));
            assert!(
                self.ground_scope_stack.contains(declaration_scope),
                "GROUND escaped its declaration scope: {ground:?}"
            );
            assert!(
                self.defined_grounds.contains(ground),
                "GROUND used before its declaration: {ground:?}"
            );
        }
        self.ground_ids
            .get(ground)
            .cloned()
            .unwrap_or_else(|| "g0".to_owned())
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(true)]
    fn render_pointer(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        let document_scope = key == graph.root && self.ground_scope_stack.is_empty();
        let document = vec!["document".to_owned()];
        if document_scope {
            self.enter_ground_scope(document.clone());
        }
        let result = self.render_pointer_inner(graph, key);
        if document_scope {
            self.leave_ground_scope(&document);
        }
        result
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(true)]
    fn render_pointer_inner(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        // This observation must precede the `defined` fast path below. Context
        // referents are provisionally predefined during planning, but their
        // actual compact pointer sites still constrain DEICTIC-GROUND placement.
        self.observe_context_referent_use(graph, key);
        if self.planning
            && let Some(owner) = self.planning_object_stack.last()
        {
            self.planning_compact_adjacency
                .entry(owner.clone())
                .or_default()
                .insert(key.to_owned());
        }
        if graph.ordinary_definition_keys.contains(key) {
            let scope = self
                .scope_stack
                .last()
                .unwrap_or_else(|| panic!("shared node used outside an SFN scope: {key:?}"))
                .clone();
            self.pointer_use_scopes
                .entry(key.to_owned())
                .or_default()
                .push(scope);
            if self.initial_planning_pass && !self.first_use_order.contains_key(key) {
                self.first_use_order
                    .insert(key.to_owned(), self.use_counter);
            }
            self.use_counter += 1;

            if self.initial_planning_pass {
                if self.defined.contains(key) {
                    return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
                }
                self.defined.insert(key.to_owned());
                self.emitted.insert(key.to_owned());
                *self.definition_sites.entry(key.to_owned()).or_default() += 1;
                let mut rendered = self.render_object(graph, key);
                rendered.prepend_attributes(vec![("ID".to_owned(), graph.id(key).to_owned())]);
                return rendered;
            }

            let declaration_scope = self
                .declaration_scopes
                .get(key)
                .unwrap_or_else(|| panic!("shared node lacks a declaration scope: {key:?}"));
            if !self.planning {
                assert!(
                    self.scope_stack.contains(declaration_scope),
                    "shared node escaped its declaration scope: {key:?}"
                );
            }
            return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
        }

        if self.defined.contains(key) {
            return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
        }
        if graph.special_definition_keys.contains(key) {
            if graph.context_sites.contains_key(key) && self.rendering_declaration {
                return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
            }
            if self.planning {
                self.planning_definition_incompatibilities.insert(new!(
                    CompactIncompatibility::DefinitionSiteDoesNotDominateUse {
                        object: key.to_owned(),
                    }
                ));
                self.defined.insert(key.to_owned());
                self.emitted.insert(key.to_owned());
                *self.definition_sites.entry(key.to_owned()).or_default() += 1;
                return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
            }
            panic!("node used before its semantic definition site: {key:?}");
        }
        if self.emitted.contains(key) {
            if self.planning {
                self.planning_repeated_single_use.insert(key.to_owned());
                return XmlElement::with_attributes("REFERENCE", [("REF", graph.id(key))]);
            }
            panic!("single-use node emitted more than once: {key:?}");
        }
        self.emitted.insert(key.to_owned());
        self.render_object(graph, key)
    }

    #[requires(true)]
    #[ensures(true)]
    fn defs(declarations: Vec<XmlElement>) -> Option<XmlElement> {
        if declarations.is_empty() {
            None
        } else {
            let mut defs = XmlElement::new("DEFS");
            defs.extend(declarations);
            Some(defs)
        }
    }

    #[requires(true)]
    #[ensures(parent.children.len() >= old(parent.children.len()))]
    fn append_defs(parent: &mut XmlElement, declarations: Vec<XmlElement>) {
        if let Some(defs) = Self::defs(declarations) {
            parent.push(defs);
        }
    }

    #[requires(true)]
    #[ensures(ret == (node.name == "REFERENCE"
        && node.attributes.len() == 1
        && node.attributes[0].0 == "REF"
        && node.children.is_empty()
        && node.text.is_none()))]
    fn is_reference(node: &XmlElement) -> bool {
        node.name == "REFERENCE"
            && node.attributes.len() == 1
            && node.attributes[0].0 == "REF"
            && node.children.is_empty()
            && node.text.is_none()
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(ret.name == tag)]
    fn wrap_pointer(
        &mut self,
        graph: &GraphData,
        tag: &str,
        key: &str,
        attributes: Vec<(&str, String)>,
    ) -> XmlElement {
        let rendered = self.render_pointer(graph, key);
        let mut result = XmlElement::new(tag);
        for (name, value) in attributes {
            result.set(name, value);
        }
        if Self::is_reference(&rendered) {
            result.set("REF", rendered.attributes[0].1.clone());
        } else {
            result.push(rendered);
        }
        result
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(ret == graph.id(key))]
    fn pointer_id(&mut self, graph: &GraphData, key: &str, site: &str) -> String {
        let rendered = self.render_pointer(graph, key);
        assert!(
            self.planning || Self::is_reference(&rendered),
            "{site} attribute target is not a defined reference: {key:?}"
        );
        graph.id(key).to_owned()
    }

    #[requires(!keys.is_empty())]
    #[ensures(!ret.is_empty())]
    fn pointer_list(&mut self, graph: &GraphData, keys: &[Value], site: &str) -> String {
        keys.iter()
            .map(|key| {
                self.pointer_id(
                    graph,
                    key.as_str()
                        .unwrap_or_else(|| panic!("{site} must contain only ids")),
                    site,
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[requires(true)]
    #[ensures(true)]
    fn current_speaker(&self) -> Option<&str> {
        self.speaker_stack.last().map(String::as_str)
    }

    #[requires(graph.objects.contains_key(speaker))]
    #[ensures(true)]
    fn apply_speaker_anchor(
        &mut self,
        graph: &GraphData,
        node: &mut XmlElement,
        speaker: &str,
        named: bool,
    ) {
        let speaker_id = self.pointer_id(graph, speaker, "speaker anchor");
        if Some(speaker) == self.current_speaker() {
            return;
        }
        if named {
            node.push(XmlElement::with_attributes("BY", [("REF", speaker_id)]));
        } else {
            node.set("SPEAKER-REF", speaker_id);
        }
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(ret.name == tag)]
    fn scoped_pointer(
        &mut self,
        graph: &GraphData,
        tag: &str,
        key: &str,
        scope: Scope,
    ) -> XmlElement {
        let (declarations, rendered) = self.scoped_parts(graph, scope, |state, graph| {
            state.render_pointer(graph, key)
        });
        let mut field = XmlElement::new(tag);
        let has_declarations = !declarations.is_empty();
        Self::append_defs(&mut field, declarations);
        if !has_declarations && Self::is_reference(&rendered) {
            field.set("REF", rendered.attributes[0].1.clone());
        } else {
            field.push(rendered);
        }
        field
    }

    #[requires(true)]
    #[ensures(true)]
    fn generic_value(&mut self, graph: &GraphData, value: &Value) -> XmlElement {
        match value {
            Value::String(value) if graph.object_keys.contains(value) => {
                self.render_pointer(graph, value)
            }
            Value::String(value) => {
                XmlElement::with_attributes("STRING", [("VALUE", value.as_str())])
            }
            Value::Null => XmlElement::new("NULL"),
            Value::Bool(_) => {
                XmlElement::with_attributes("BOOLEAN", [("VALUE", enum_token(value))])
            }
            Value::Number(_) => {
                XmlElement::with_attributes("NUMBER", [("VALUE", value.to_string())])
            }
            Value::Array(items) => {
                let mut list = XmlElement::new("LIST");
                for item in items {
                    let mut element = XmlElement::new("ITEM");
                    element.push(self.generic_value(graph, item));
                    list.push(element);
                }
                list
            }
            Value::Object(object) => {
                self.account_object(graph, object);
                let mut record = XmlElement::new("RECORD");
                for (key, item) in object {
                    if key == "source" && is_source_record(item) {
                        self.record_field_omission(
                            graph,
                            object,
                            key,
                            XmlWaiverFamily::SourceRecord,
                        );
                        continue;
                    }
                    if key == "introducedBy" {
                        self.record_field_omission(
                            graph,
                            object,
                            key,
                            XmlWaiverFamily::IntroducedBy,
                        );
                        continue;
                    }
                    self.account_field(graph, object, key);
                    let mut field = XmlElement::with_attributes("FIELD", [("NAME", key.as_str())]);
                    field.push(self.generic_value(graph, item));
                    record.push(field);
                }
                record
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.len() <= 1)]
    fn extras(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
        handled: &[&str],
    ) -> Vec<XmlElement> {
        let mut fields = Vec::new();
        for (key, value) in object {
            if handled.contains(&key.as_str()) {
                continue;
            }
            if key == "source" && is_source_record(value) {
                self.record_field_omission(graph, object, key, XmlWaiverFamily::SourceRecord);
                continue;
            }
            if key == "introducedBy" {
                self.record_field_omission(graph, object, key, XmlWaiverFamily::IntroducedBy);
                continue;
            }
            self.account_field(graph, object, key);
            let mut field = XmlElement::with_attributes("FIELD", [("NAME", key.as_str())]);
            field.push(self.generic_value(graph, value));
            fields.push(field);
        }
        if fields.is_empty() {
            Vec::new()
        } else {
            let mut extra = XmlElement::new("EXTRA");
            extra.extend(fields);
            vec![extra]
        }
    }

    #[requires(!tag.is_empty())]
    #[ensures(ret.name == tag)]
    fn scalar(tag: &str, value: &Value) -> XmlElement {
        XmlElement::with_attributes(tag, [("VALUE", enum_token(value))])
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn facet_attribute_name(field: &str) -> &'static str {
    match field {
        "time" => "TIME",
        "actuality" => "ACTUALITY",
        "aspect" => "ASPECT",
        "recurrence" => "RECURRENCE",
        "space" => "SPACE",
        "spatialAspect" => "SPATIAL-ASPECT",
        "spatialRecurrence" => "SPATIAL-RECURRENCE",
        "details" => "DETAILS",
        _ => panic!("unknown facet field: {field:?}"),
    }
}

#[requires(true)]
#[ensures(true)]
fn facet_primary_field(field: &str) -> Option<&'static str> {
    match field {
        "time" | "space" => Some("relation"),
        "actuality" => Some("kind"),
        "aspect" | "spatialAspect" => Some("contour"),
        _ => None,
    }
}

impl RenderState {
    #[requires(true)]
    #[ensures(true)]
    fn apply_scope_dependence(
        &mut self,
        graph: &GraphData,
        referent_key: &str,
        node: &mut XmlElement,
        value: &Map<String, Value>,
    ) {
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        if value.contains_key("mayDependOn") {
            self.account_field(graph, value, "mayDependOn");
        }
        assert!(
            value
                .keys()
                .all(|field| matches!(field.as_str(), "kind" | "mayDependOn")),
            "scopeDependence has unsupported fields"
        );
        let active: Vec<String> =
            self.bound_variable_stack
                .iter()
                .cloned()
                .fold(Vec::new(), |mut unique, value| {
                    if !unique.contains(&value) {
                        unique.push(value);
                    }
                    unique
                });
        match optional_string(value, "kind") {
            Some("fixed") if !active.is_empty() => node.set("SAME-FOR-ALL", "true"),
            Some("fixed") => {}
            Some("underspecified") => {
                let dependencies: Vec<String> = value
                    .get("mayDependOn")
                    .map(|dependencies| {
                        json_array(dependencies)
                            .iter()
                            .map(|dependency| {
                                dependency
                                    .as_str()
                                    .unwrap_or_else(|| {
                                        panic!("scopeDependence dependency must be an id")
                                    })
                                    .to_owned()
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| active.clone());
                let active_set: HashSet<&str> = active.iter().map(String::as_str).collect();
                let dependency_set: HashSet<&str> =
                    dependencies.iter().map(String::as_str).collect();
                if !dependency_set.is_subset(&active_set) {
                    if self.planning {
                        for dependency in dependency_set.difference(&active_set) {
                            self.planning_incompatibilities.insert(new!(
                                CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder {
                                    referent: referent_key.to_owned(),
                                    dependency: (*dependency).to_owned(),
                                }
                            ));
                        }
                        return;
                    }
                    panic!("scopeDependence mayDependOn contains a non-enclosing binder");
                }
                let dependency_ids: Vec<String> = dependencies
                    .iter()
                    .map(|dependency| self.pointer_id(graph, dependency, "POSSIBLY-DIFFERENT-PER"))
                    .collect();
                if dependency_set.len() < active_set.len() {
                    assert!(
                        !dependencies.is_empty(),
                        "an empty strict may-depend subset has no NMTOKENS representation"
                    );
                    node.set("POSSIBLY-DIFFERENT-PER", dependency_ids.join(" "));
                }
            }
            kind => panic!("unknown scopeDependence kind: {kind:?}"),
        }
    }

    #[requires(true)]
    #[ensures(ret.name == "RELATIVE-CLAUSE")]
    fn render_relative_clause(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
        owner_key: Option<&str>,
        index: usize,
    ) -> XmlElement {
        let kind = value
            .get("kind")
            .map(enum_token)
            .unwrap_or_else(|| "UNKNOWN".to_owned());
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        let mut clause = XmlElement::with_attributes("RELATIVE-CLAUSE", [("KIND", kind)]);
        if let Some(body) = optional_string(value, "body") {
            self.account_field(graph, value, "body");
            let rendered = if let Some(owner) = owner_key {
                self.scoped_pointer(
                    graph,
                    "BODY",
                    body,
                    vec![
                        "description-relative".to_owned(),
                        owner.to_owned(),
                        index.to_string(),
                    ],
                )
            } else {
                self.wrap_pointer(graph, "BODY", body, Vec::new())
            };
            clause.push(rendered);
        }
        clause.extend(self.extras(graph, value, &["kind", "body"]));
        clause
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_descriptor(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
        owner_key: Option<&str>,
    ) -> XmlElement {
        let kind = optional_string(value, "kind").unwrap_or("unknown");
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        if kind == "proSumti" {
            let word = string_field(value, "word");
            self.account_field(graph, value, "word");
            let mut result = XmlElement::with_attributes("UNRESOLVED-REFERENT", [("WORD", word)]);
            result.extend(self.extras(graph, value, &["kind", "word"]));
            return result;
        }
        if kind == "name" {
            let name = string_field(value, "name");
            let speaker = string_field(value, "speaker");
            self.account_field(graph, value, "name");
            self.account_field(graph, value, "speaker");
            let mut result = XmlElement::with_attributes("NAMED", [("TEXT", name)]);
            self.apply_speaker_anchor(graph, &mut result, speaker, true);
            if value.contains_key("word") {
                self.record_field_omission(graph, value, "word", XmlWaiverFamily::DescriptorWord);
            }
            result.extend(self.extras(graph, value, &["kind", "name", "speaker", "word"]));
            return result;
        }

        let mut result = if DESCRIPTOR_KINDS.contains(&kind) {
            XmlElement::new(enum_string(kind).replace('_', "-"))
        } else {
            XmlElement::with_attributes("DESCRIPTOR", [("KIND", enum_string(kind))])
        };
        let mut handled = Vec::from(["kind", "word"]);
        if kind == "elided" && optional_string(value, "word") == Some("zo'e") {
            self.account_field(graph, value, "word");
        } else if value.contains_key("word") {
            self.record_field_omission(graph, value, "word", XmlWaiverFamily::DescriptorWord);
        }
        if let Some(name) = optional_string(value, "name") {
            self.account_field(graph, value, "name");
            result.push(XmlElement::with_attributes("NAME-VALUE", [("VALUE", name)]));
            handled.push("name");
        }
        if let Some(speaker) = optional_string(value, "speaker") {
            self.account_field(graph, value, "speaker");
            self.apply_speaker_anchor(graph, &mut result, speaker, false);
            handled.push("speaker");
        }
        for field in ["quantity", "operand", "denotes"] {
            if let Some(key) = optional_string(value, field) {
                self.account_field(graph, value, field);
                result.push(self.wrap_pointer(
                    graph,
                    &enum_string(field).replace('_', "-"),
                    key,
                    Vec::new(),
                ));
                handled.push(field);
            }
        }
        if let Some(body) = optional_string(value, "body") {
            self.account_field(graph, value, "body");
            let rendered = if let Some(owner) = owner_key {
                self.scoped_pointer(
                    graph,
                    "BODY",
                    body,
                    vec!["description-body".to_owned(), owner.to_owned()],
                )
            } else {
                self.wrap_pointer(graph, "BODY", body, Vec::new())
            };
            result.push(rendered);
            handled.push("body");
        }
        if let Some(clauses) = value.get("relativeClauses").and_then(Value::as_array) {
            self.account_field(graph, value, "relativeClauses");
            let mut rendered = XmlElement::new("RELATIVE-CLAUSES");
            for (index, clause) in clauses.iter().enumerate() {
                rendered.push(self.render_relative_clause(
                    graph,
                    json_object(clause),
                    owner_key,
                    index,
                ));
            }
            result.push(rendered);
            handled.push("relativeClauses");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "PERSONAL-MASS-MEMBERSHIP")]
    fn render_personal_mass_membership(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new("PERSONAL-MASS-MEMBERSHIP");
        let mut handled = Vec::from(["speaker", "audience"]);
        for (field, tag) in [("speaker", "SPEAKER"), ("audience", "AUDIENCE")] {
            self.account_field(graph, value, field);
            let participant = value
                .get(field)
                .and_then(Value::as_object)
                .unwrap_or_else(|| panic!("personal mass membership lacks {field}"));
            let referent = string_field(participant, "referent");
            self.account_field(graph, participant, "referent");
            let membership = participant
                .get("membership")
                .map(enum_token)
                .unwrap_or_else(|| "UNKNOWN".to_owned());
            if participant.contains_key("membership") {
                self.account_field(graph, participant, "membership");
            }
            let mut member =
                self.wrap_pointer(graph, tag, referent, vec![("MEMBERSHIP", membership)]);
            member.extend(self.extras(graph, participant, &["membership", "referent"]));
            result.push(member);
        }
        if let Some(others) = optional_string(value, "others") {
            self.account_field(graph, value, "others");
            result.push(self.wrap_pointer(graph, "OTHERS", others, Vec::new()));
            handled.push("others");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "DEICTIC-REFERENCE")]
    fn render_deictic_reference(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let ground = string_field(value, "ground");
        self.account_field(graph, value, "ground");
        if value.contains_key("proximity") {
            self.account_field(graph, value, "proximity");
        }
        let mut result = XmlElement::with_attributes(
            "DEICTIC-REFERENCE",
            [
                (
                    "PROXIMITY",
                    value
                        .get("proximity")
                        .map(enum_token)
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                ),
                (
                    "GROUND-REF",
                    self.pointer_id(graph, ground, "DEICTIC-REFERENCE GROUND-REF"),
                ),
            ],
        );
        result.extend(self.extras(graph, value, &["proximity", "ground"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "GENERATED-REFERENT")]
    fn render_generated_referent(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new("GENERATED-REFERENT");
        let mut handled = Vec::new();
        for field in ["realization", "specificity"] {
            if let Some(field_value) = value.get(field) {
                self.account_field(graph, value, field);
                result.set(
                    enum_string(field).replace('_', "-"),
                    enum_token(field_value),
                );
                handled.push(field);
            }
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(FACET_FIELDS.contains(&field))]
    #[ensures(ret.name == "FACET")]
    fn render_facet(&mut self, graph: &GraphData, field: &str, value: &Value) -> XmlElement {
        let mut result =
            XmlElement::with_attributes("FACET", [("NAME", facet_attribute_name(field))]);
        if matches!(field, "recurrence" | "spatialRecurrence")
            && let Some(items) = value.as_array()
        {
            for item in items {
                let Some(item) = item.as_object() else {
                    result.push(self.generic_value(graph, item));
                    continue;
                };
                let mut occurrence = XmlElement::new("OCCURRENCE");
                let mut handled = Vec::from(["introducedBy"]);
                if item.contains_key("introducedBy") {
                    self.record_field_omission(
                        graph,
                        item,
                        "introducedBy",
                        XmlWaiverFamily::IntroducedBy,
                    );
                }
                if let Some(kind) = item.get("kind") {
                    self.account_field(graph, item, "kind");
                    occurrence.set("KIND", enum_token(kind));
                    handled.push("kind");
                }
                if let Some(quantity) = optional_string(item, "quantity") {
                    self.account_field(graph, item, "quantity");
                    occurrence.push(self.wrap_pointer(graph, "QUANTITY", quantity, Vec::new()));
                    handled.push("quantity");
                }
                occurrence.extend(self.extras(graph, item, &handled));
                result.push(occurrence);
            }
            return result;
        }
        if let Some(value) = value.as_object() {
            let mut handled = Vec::new();
            if let Some(primary) = facet_primary_field(field)
                && let Some(primary_value) = value.get(primary)
            {
                self.account_field(graph, value, primary);
                result.set(
                    enum_string(primary).replace('_', "-"),
                    enum_token(primary_value),
                );
                handled.push(primary);
            }
            if let Some(anchor) = optional_string(value, "anchor") {
                self.account_field(graph, value, "anchor");
                result.push(self.wrap_pointer(graph, "ANCHOR", anchor, Vec::new()));
                handled.push("anchor");
            }
            result.extend(self.extras(graph, value, &handled));
            return result;
        }
        result.push(self.generic_value(graph, value));
        result
    }

    #[requires(true)]
    #[ensures(true)]
    fn apply_facets(
        &mut self,
        graph: &GraphData,
        node: &mut XmlElement,
        object: &Map<String, Value>,
    ) {
        for field in FACET_FIELDS {
            let Some(value) = object.get(*field) else {
                continue;
            };
            self.account_field(graph, object, field);
            let primary = facet_primary_field(field);
            if let (Some(primary), Some(record)) = (primary, value.as_object())
                && record.len() == 1
                && let Some(primary_value) = record.get(primary)
            {
                self.account_field(graph, record, primary);
                node.set(facet_attribute_name(field), enum_token(primary_value));
            } else if !value.is_object() && !value.is_array() {
                node.set(facet_attribute_name(field), enum_token(value));
            } else {
                node.push(self.render_facet(graph, field, value));
            }
        }
    }

    #[requires(graph.objects.contains_key(key))]
    #[ensures(ret.name == "VARIABLE")]
    fn render_binder_definition(&mut self, graph: &GraphData, key: &str, site: &str) -> XmlElement {
        let node_id = self.define_at_site(graph, key, site);
        let object = graph.object(key);
        self.account_field(graph, object, "type");
        assert!(
            matches!(
                optional_string(object, "type"),
                Some("referent" | "parameter")
            ),
            "quantifier variable is not a referent: {key:?}"
        );
        let sort = object
            .get("sort")
            .unwrap_or_else(|| panic!("quantifier variable lacks a sort: {key:?}"));
        self.account_field(graph, object, "sort");
        let mut variable = XmlElement::with_attributes(
            "VARIABLE",
            [("ID", node_id), ("SORT", flat_sort_name(sort))],
        );
        let mut handled = Vec::from(["type", "sort", "assignedNames"]);
        if let Some(assigned_names) = object.get("assignedNames") {
            self.account_field(graph, object, "assignedNames");
            self.observe_assigned_name_omissions(graph, assigned_names);
        }
        if optional_string(object, "denotation") == Some("generated-bound") {
            self.account_field(graph, object, "denotation");
            handled.push("denotation");
            if let Some(content) = optional_string(object, "content") {
                self.account_field(graph, object, "content");
                validate_generated_event_content_backlink(graph, key, content);
                handled.push("content");
            }
        } else if let Some(denotation) = object.get("denotation") {
            self.account_field(graph, object, "denotation");
            variable.push(Self::scalar("DENOTATION", denotation));
            handled.push("denotation");
        }
        if optional_string(object, "category") == Some("variable") {
            self.account_field(graph, object, "category");
            handled.push("category");
        } else if let Some(category) = object.get("category") {
            self.account_field(graph, object, "category");
            variable.push(Self::scalar("CATEGORY", category));
            handled.push("category");
        }
        if let Some(descriptor) = object.get("descriptor").and_then(Value::as_object) {
            self.account_field(graph, object, "descriptor");
            let bound_surface_word = optional_string(object, "category") == Some("variable")
                && optional_string(descriptor, "kind") == Some("proSumti");
            if bound_surface_word {
                self.account_field(graph, descriptor, "kind");
                if descriptor.contains_key("word") {
                    self.record_field_omission(
                        graph,
                        descriptor,
                        "word",
                        XmlWaiverFamily::BoundVariableWord,
                    );
                }
            } else {
                variable.push(self.render_descriptor(graph, descriptor, Some(key)));
            }
            handled.push("descriptor");
        }
        if let Some(scope) = object.get("scopeDependence").and_then(Value::as_object) {
            self.account_field(graph, object, "scopeDependence");
            self.apply_scope_dependence(graph, key, &mut variable, scope);
            handled.push("scopeDependence");
        }
        self.apply_facets(graph, &mut variable, object);
        handled.extend(FACET_FIELDS.iter().copied());
        variable.extend(self.extras(graph, object, &handled));
        variable
    }
}

#[requires(graph.objects.contains_key(event_key))]
#[requires(graph.objects.contains_key(content_key))]
#[ensures(true)]
fn validate_generated_event_content_backlink(
    graph: &GraphData,
    event_key: &str,
    content_key: &str,
) {
    let formula = graph.object(content_key);
    let predication_key = optional_string(formula, "predication")
        .unwrap_or_else(|| panic!("generated event content must point to a predication"));
    let predication = graph.object(predication_key);
    assert!(
        optional_string(formula, "type") == Some("formula")
            && optional_string(formula, "operator") == Some("atom")
            && optional_string(predication, "type") == Some("predication")
            && optional_string(predication, "eventuality") == Some(event_key),
        "generated-bound event content is not the inverse of its predication EVENT edge"
    );
}

impl RenderState {
    #[requires(true)]
    #[ensures(ret.name == "OCCURRENCE")]
    fn render_recurrence_item(&mut self, graph: &GraphData, value: &Value) -> XmlElement {
        let Some(value) = value.as_object() else {
            let mut result = XmlElement::new("OCCURRENCE");
            result.push(self.generic_value(graph, value));
            return result;
        };
        let mut result = XmlElement::new("OCCURRENCE");
        let mut handled = Vec::from(["introducedBy"]);
        if value.contains_key("introducedBy") {
            self.record_field_omission(graph, value, "introducedBy", XmlWaiverFamily::IntroducedBy);
        }
        if let Some(kind) = value.get("kind") {
            self.account_field(graph, value, "kind");
            result.set("KIND", enum_token(kind));
            handled.push("kind");
        }
        if let Some(quantity) = optional_string(value, "quantity") {
            self.account_field(graph, value, "quantity");
            result.push(self.wrap_pointer(graph, "QUANTITY", quantity, Vec::new()));
            handled.push("quantity");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "INTERVAL-MODIFIER")]
    fn render_interval_modifier(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let kind = value
            .get("kind")
            .map(enum_token)
            .unwrap_or_else(|| "INTERVAL".to_owned());
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        let mut result = XmlElement::with_attributes("INTERVAL-MODIFIER", [("KIND", kind)]);
        if let Some(field_value) = value.get("value") {
            self.account_field(graph, value, "value");
            let mut rendered = XmlElement::new("VALUE");
            rendered.push(self.generic_value(graph, field_value));
            result.push(rendered);
        }
        result.extend(self.extras(graph, value, &["kind", "value"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "CONNECTOR")]
    fn render_connector(&mut self, graph: &GraphData, value: &Map<String, Value>) -> XmlElement {
        let mut result = XmlElement::new("CONNECTOR");
        let mut handled = Vec::new();
        if let Some(source) = value.get("source") {
            self.account_field(graph, value, "source");
            result.set("SOURCE-WORD", scalar_string(source));
            handled.push("source");
        }
        for field in ["locus", "truthTable"] {
            if let Some(field_value) = value.get(field) {
                self.account_field(graph, value, field);
                result.push(Self::scalar(
                    &enum_string(field).replace('_', "-"),
                    field_value,
                ));
                handled.push(field);
            }
        }
        if let Some(parameter) = optional_string(value, "parameter") {
            self.account_field(graph, value, "parameter");
            result.push(self.wrap_pointer(graph, "PARAMETER", parameter, Vec::new()));
            handled.push("parameter");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "SCALAR-NEGATION")]
    fn render_scalar_negation(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new("SCALAR-NEGATION");
        let mut handled = Vec::from(["introducedBy"]);
        if value.contains_key("introducedBy") {
            self.record_field_omission(graph, value, "introducedBy", XmlWaiverFamily::IntroducedBy);
        }
        if let Some(kind) = value.get("kind") {
            self.account_field(graph, value, "kind");
            result.set("KIND", enum_token(kind));
            handled.push("kind");
        }
        if let Some(scale) = optional_string(value, "scale") {
            self.account_field(graph, value, "scale");
            result.push(self.wrap_pointer(graph, "SCALE", scale, Vec::new()));
            handled.push("scale");
        }
        if let Some(argument_scope) = value.get("argumentScope").and_then(Value::as_array) {
            self.account_field(graph, value, "argumentScope");
            let places: Vec<&str> = argument_scope
                .iter()
                .map(|place| {
                    place_label(
                        place
                            .as_str()
                            .unwrap_or_else(|| panic!("ARGUMENT-SCOPE place must be a string")),
                    )
                })
                .collect();
            assert!(!places.is_empty(), "ARGUMENT-SCOPE cannot be empty");
            result.set("ARGUMENT-SCOPE", places.join(" "));
            handled.push("argumentScope");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "DISPLAY-MODIFIER")]
    fn render_display_modifier(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new("DISPLAY-MODIFIER");
        let mut handled = Vec::new();
        if let Some(relation) = optional_string(value, "relation") {
            self.account_field(graph, value, "relation");
            result.set("RELATION", predicate_symbol(relation));
            handled.push("relation");
        }
        for field in ["family", "polarity", "assertionEffect"] {
            if let Some(field_value) = value.get(field) {
                self.account_field(graph, value, field);
                result.set(
                    enum_string(field).replace('_', "-"),
                    enum_token(field_value),
                );
                handled.push(field);
            }
        }
        if let Some(intensity) = optional_string(value, "intensity") {
            self.account_field(graph, value, "intensity");
            result.set("INTENSITY-WORD", intensity);
            handled.push("intensity");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "ARG")]
    fn render_argument(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
        index: Option<&str>,
        fill: bool,
    ) -> XmlElement {
        let kind = optional_string(value, "kind");
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        let mut handled = Vec::from(["kind"]);
        let rendered = if kind == Some("deleted") {
            XmlElement::new("DELETED")
        } else if let Some(pointer) = optional_string(value, "value") {
            self.account_field(graph, value, "value");
            handled.push("value");
            self.render_pointer(graph, pointer)
        } else {
            XmlElement::new("MISSING-VALUE")
        };

        let introduced = optional_string(value, "introducedBy");
        if introduced.is_some() {
            self.record_field_omission(graph, value, "introducedBy", XmlWaiverFamily::IntroducedBy);
        }
        let mut result = XmlElement::new("ARG");
        if let Some(index) = index {
            result.set("INDEX", index);
        }
        if fill {
            result.set("FILL", "true");
        }
        if kind == Some("elided") {
            let plain_zohe = introduced == Some("zo'e")
                && optional_string(value, "value")
                    .is_some_and(|key| is_elided_unspecified(graph.object(key)));
            if introduced.is_some() {
                handled.push("introducedBy");
            }
            if !plain_zohe && introduced.is_some() {
                result.set("STATUS", "ELIDED");
            }
        } else if introduced.is_some() {
            result.set("STATUS", "FILLED");
            handled.push("introducedBy");
        }

        if Self::is_reference(&rendered) {
            result.set("REF", rendered.attributes[0].1.clone());
        } else {
            result.push(rendered);
        }
        if let Some(clauses) = value.get("relativeClauses").and_then(Value::as_array) {
            self.account_field(graph, value, "relativeClauses");
            for clause_value in clauses {
                let clause = json_object(clause_value);
                let value = optional_string(value, "value");
                let body = optional_string(clause, "body");
                if matches!((value, body), (Some(value), Some(body))
                    if graph.quantifier_restrictions.contains(&(value.to_owned(), body.to_owned())))
                {
                    for (field, field_value) in clause {
                        if field == "source" && is_source_record(field_value) {
                            self.record_field_omission(
                                graph,
                                clause,
                                field,
                                XmlWaiverFamily::SourceRecord,
                            );
                        } else if field == "introducedBy" {
                            self.record_field_omission(
                                graph,
                                clause,
                                field,
                                XmlWaiverFamily::IntroducedBy,
                            );
                        } else if matches!(field.as_str(), "kind" | "body") {
                            self.account_field(graph, clause, field);
                        }
                    }
                }
            }
            let clauses: Vec<&Map<String, Value>> = clauses
                .iter()
                .map(json_object)
                .filter(|clause| {
                    let value = optional_string(value, "value");
                    let body = optional_string(clause, "body");
                    !matches!((value, body), (Some(value), Some(body))
                        if graph.quantifier_restrictions.contains(&(value.to_owned(), body.to_owned())))
                })
                .collect();
            if !clauses.is_empty() {
                let mut rendered = XmlElement::new("RELATIVE-CLAUSES");
                for clause in clauses {
                    rendered.push(self.render_relative_clause(graph, clause, None, 0));
                }
                result.push(rendered);
            }
            handled.push("relativeClauses");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "ADJUNCT")]
    fn render_added_place(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
        host_referent: Option<&str>,
    ) -> XmlElement {
        if let Some(relation) = optional_string(value, "relation") {
            self.account_field(graph, value, "relation");
            let symbol = predicate_symbol(relation);
            assert_eq!(
                symbol,
                symbol.to_lowercase(),
                "adjunct key is not lowercase"
            );
            let arguments = value
                .get("arguments")
                .and_then(Value::as_object)
                .filter(|arguments| !arguments.is_empty())
                .unwrap_or_else(|| panic!("relation-keyed adjunct needs a nonempty place map"));
            self.account_field(graph, value, "arguments");
            let fill_place = structural_adjunct_fill_place(value, host_referent);
            let mut result = XmlElement::with_attributes("ADJUNCT", [("PREDICATE", symbol)]);
            for place in sorted_places(arguments) {
                self.account_field(graph, arguments, place);
                result.push(self.render_argument(
                    graph,
                    json_object(&arguments[place]),
                    Some(place_label(place)),
                    fill_place == Some(place),
                ));
            }
            if fill_place.is_none() {
                result.push(XmlElement::with_attributes(
                    "FILL-STATUS",
                    [("VALUE", "AMBIGUOUS-IN-GRAPH")],
                ));
            }
            let mut handled = Vec::from(["relation", "arguments"]);
            result.extend(self.render_added_metadata(graph, value, &mut handled));
            return result;
        }

        let Some(body_key) = optional_string(value, "body") else {
            let mut result = XmlElement::new("ADJUNCT");
            let mut record = XmlElement::new("RECORD");
            for (field, item) in value {
                if field == "source" && is_source_record(item) {
                    self.record_field_omission(graph, value, field, XmlWaiverFamily::SourceRecord);
                    continue;
                }
                if field == "introducedBy" {
                    self.record_field_omission(graph, value, field, XmlWaiverFamily::IntroducedBy);
                    continue;
                }
                self.account_field(graph, value, field);
                let mut rendered = XmlElement::with_attributes("FIELD", [("NAME", field.as_str())]);
                rendered.push(self.generic_value(graph, item));
                record.push(rendered);
            }
            result.push(record);
            return result;
        };
        self.account_field(graph, value, "body");
        let fill_site = unique_explicit_body_argument(graph, body_key);
        let old_fill = self.active_fill_site.clone();
        let old_marks = self.fill_marks;
        self.active_fill_site = fill_site.clone();
        self.fill_marks = 0;
        let body = self.render_pointer(graph, body_key);
        let marks = self.fill_marks;
        self.active_fill_site = old_fill;
        self.fill_marks = old_marks;

        let mut result = XmlElement::new("ADJUNCT");
        let mut body_node = XmlElement::new("BODY");
        body_node.push(body);
        result.push(body_node);
        if fill_site.is_none() {
            result.push(XmlElement::with_attributes(
                "FILL-STATUS",
                [("VALUE", "AMBIGUOUS-IN-GRAPH")],
            ));
        } else {
            assert_eq!(
                marks, 1,
                "added body {body_key:?} expected one FILL, found {marks}"
            );
        }
        let mut handled = Vec::from(["body"]);
        result.extend(self.render_added_metadata(graph, value, &mut handled));
        result
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_added_metadata<'a>(
        &mut self,
        graph: &GraphData,
        value: &'a Map<String, Value>,
        handled: &mut Vec<&'a str>,
    ) -> Vec<XmlElement> {
        let mut entries = Vec::new();
        if value.contains_key("introducedBy") {
            self.record_field_omission(graph, value, "introducedBy", XmlWaiverFamily::IntroducedBy);
            handled.push("introducedBy");
        }
        if let Some(component) = optional_string(value, "component") {
            self.account_field(graph, value, "component");
            entries.push(self.wrap_pointer(graph, "APPLIES-TO", component, Vec::new()));
            handled.push("component");
        }
        if let Some(negation) = value.get("negation").and_then(Value::as_object) {
            self.account_field(graph, value, "negation");
            if negation.contains_key("introducedBy") {
                self.record_field_omission(
                    graph,
                    negation,
                    "introducedBy",
                    XmlWaiverFamily::IntroducedBy,
                );
            }
            let mut node = XmlElement::with_attributes(
                "NEGATION-METADATA",
                [(
                    "KIND",
                    negation
                        .get("kind")
                        .map(enum_token)
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                )],
            );
            if negation.contains_key("kind") {
                self.account_field(graph, negation, "kind");
            }
            node.extend(self.extras(graph, negation, &["kind", "introducedBy"]));
            entries.push(node);
            handled.push("negation");
        }
        if let Some(negation) = value.get("scalarNegation").and_then(Value::as_object) {
            self.account_field(graph, value, "scalarNegation");
            entries.push(self.render_scalar_negation(graph, negation));
            handled.push("scalarNegation");
        }
        if let Some(modifiers) = value.get("modifiers").and_then(Value::as_array) {
            self.account_field(graph, value, "modifiers");
            let mut rendered = XmlElement::new("MODIFIERS");
            for modifier in modifiers {
                rendered.push(self.render_display_modifier(graph, json_object(modifier)));
            }
            entries.push(rendered);
            handled.push("modifiers");
        }
        entries.extend(self.extras(graph, value, handled));
        entries
    }
}

#[requires(true)]
#[ensures(true)]
fn scalar_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

#[requires(true)]
#[ensures(true)]
fn structural_adjunct_fill_place<'a>(
    value: &'a Map<String, Value>,
    host_referent: Option<&str>,
) -> Option<&'a str> {
    let arguments = value.get("arguments")?.as_object()?;
    let candidates: Vec<&str> = arguments
        .iter()
        .filter_map(|(place, value)| {
            let argument = value.as_object()?;
            (optional_string(argument, "kind") == Some("filled")
                && optional_string(argument, "value") != host_referent)
                .then_some(place.as_str())
        })
        .collect();
    match candidates.as_slice() {
        [candidate] => Some(*candidate),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn unique_explicit_body_argument(graph: &GraphData, body_key: &str) -> Option<(String, String)> {
    let mut pending = vec![body_key.to_owned()];
    let mut seen_formulas = HashSet::new();
    let mut predications = Vec::new();
    while let Some(formula_key) = pending.pop() {
        if !seen_formulas.insert(formula_key.clone()) {
            continue;
        }
        let formula = graph.object(&formula_key);
        assert_eq!(
            optional_string(formula, "type"),
            Some("formula"),
            "added body is not a formula: {formula_key:?}"
        );
        let mut pointers = Vec::new();
        for (field, value) in formula {
            if field == "source" && is_source_record(value) {
                continue;
            }
            append_prototype_non_source_pointers(value, &graph.object_keys, &mut pointers);
        }
        for target in pointers {
            match optional_string(graph.object(&target), "type") {
                Some("formula") => pending.push(target),
                Some("predication") => predications.push(target),
                _ => {}
            }
        }
    }
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for predication_key in predications {
        if !seen.insert(predication_key.clone()) {
            continue;
        }
        let predication = graph.object(&predication_key);
        let Some(arguments) = predication.get("arguments").and_then(Value::as_object) else {
            continue;
        };
        for place in sorted_places(arguments) {
            let argument = json_object(&arguments[place]);
            if optional_string(argument, "kind") == Some("filled")
                && optional_string(argument, "value")
                    .is_some_and(|value| graph.objects.contains_key(value))
            {
                candidates.push((predication_key.clone(), place.to_owned()));
            }
        }
    }
    (candidates.len() == 1).then(|| candidates.remove(0))
}

#[requires(true)]
#[ensures(true)]
fn is_elided_unspecified(object: &Map<String, Value>) -> bool {
    optional_string(object, "type") == Some("referent")
        && object
            .get("descriptor")
            .and_then(Value::as_object)
            .is_some_and(|descriptor| {
                optional_string(descriptor, "kind") == Some("elided")
                    && optional_string(descriptor, "word") == Some("zo'e")
            })
}

#[requires(true)]
#[ensures(true)]
fn is_bare_zohe(object: &Map<String, Value>) -> bool {
    let descriptor = object.get("descriptor").and_then(Value::as_object);
    is_elided_unspecified(object)
        && optional_string(object, "sort") == Some("entity")
        && optional_string(object, "category") == Some("constant")
        && object
            .get("scopeDependence")
            .and_then(Value::as_object)
            .is_some_and(|scope| {
                scope.len() == 1 && optional_string(scope, "kind") == Some("fixed")
            })
        && descriptor.is_some_and(|descriptor| descriptor.len() == 2)
        && object.keys().all(|field| {
            matches!(
                field.as_str(),
                "type" | "sort" | "category" | "scopeDependence" | "descriptor" | "source"
            )
        })
}

#[requires(true)]
#[ensures(ret.iter().all(|parameter| graph.objects.contains_key(parameter)))]
fn question_parameters(graph: &GraphData, object: &Map<String, Value>) -> Vec<String> {
    let slots = object.get("slots").map(json_array).unwrap_or_default();
    let mut parameters = Vec::new();
    for slot in slots {
        let slot = json_object(slot);
        let Some(parameter) = slot.get("parameter") else {
            continue;
        };
        let parameter = parameter
            .as_str()
            .unwrap_or_else(|| panic!("question slot parameter must be an id"));
        assert_eq!(
            optional_string(graph.object(parameter), "type"),
            Some("parameter"),
            "question slot parameter must reference a parameter object"
        );
        parameters.push(parameter.to_owned());
    }
    parameters.sort();
    parameters.dedup();
    parameters
}

#[requires(true)]
#[ensures(ret.iter().all(|question| graph.objects.contains_key(*question)))]
fn embedded_questions_for_formula<'a>(
    graph: &'a GraphData,
    object: &'a Map<String, Value>,
    formula: &str,
) -> Vec<&'a str> {
    let mut questions = Vec::new();
    if let Some(embedded) = object.get("embeddedQuestions").and_then(Value::as_array) {
        for question in embedded {
            let question = question
                .as_str()
                .unwrap_or_else(|| panic!("embedded question must be an id"));
            if string_field(graph.object(question), "body") == formula {
                questions.push(question);
            }
        }
    }
    questions
}

#[requires(true)]
#[ensures(ret.iter().all(|parameter| graph.objects.contains_key(parameter)))]
fn abstraction_parameters(
    graph: &GraphData,
    object: &Map<String, Value>,
    formula: &str,
) -> Vec<String> {
    let mut parameters = Vec::new();
    if let Some(explicit) = object.get("parameters").and_then(Value::as_array) {
        parameters.extend(explicit.iter().map(|parameter| {
            parameter
                .as_str()
                .unwrap_or_else(|| panic!("abstraction parameter must be an id"))
                .to_owned()
        }));
    }
    if let Some(questions) = object.get("embeddedQuestions").and_then(Value::as_array) {
        for question in questions {
            let question = question
                .as_str()
                .unwrap_or_else(|| panic!("embedded question must be an id"));
            let question = graph.object(question);
            if string_field(question, "body") == formula {
                parameters.extend(question_parameters(graph, question));
            }
        }
    }
    parameters.sort();
    parameters.dedup();
    parameters
}

impl RenderState {
    #[requires(graph.objects.contains_key(key))]
    #[ensures(!ret.name.is_empty())]
    fn render_object(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        let object = graph.object(key);
        self.account_object(graph, object);
        if object.contains_key("type") {
            self.account_field(graph, object, "type");
        }
        if self.planning {
            self.planning_compact_adjacency
                .entry(key.to_owned())
                .or_default();
            self.planning_object_stack.push(key.to_owned());
        }
        let rendered = match optional_string(object, "type") {
            Some("utterance") => self.render_utterance(graph, key, object),
            Some("predication") => self.render_predication(graph, key, object),
            Some("formula") => self.render_formula(graph, key, object),
            Some("referent") => self.render_referent(graph, key, object),
            Some("quantity") => self.render_quantity(graph, object),
            Some("parameter") => self.render_parameter(graph, object),
            Some("sequence") => self.render_sequence(graph, key, object),
            Some("displayedContent") => self.render_displayed_content(graph, object),
            Some("mathExpression") => self.render_math_expression(graph, object),
            Some("question") => self.render_question(graph, key, object),
            _ => self.render_unknown_object(graph, object),
        };
        if self.planning {
            assert_eq!(
                self.planning_object_stack.pop().as_deref(),
                Some(key),
                "compact planning object stack became unbalanced"
            );
        }
        rendered
    }

    #[requires(true)]
    #[ensures(ret.name == "UNKNOWN")]
    fn render_question(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let parameters = question_parameters(graph, object);
        let mut result = XmlElement::with_attributes("UNKNOWN", [("TYPE", "question")]);
        for (field, value) in object {
            if field == "type" {
                continue;
            }
            if field == "source" && is_source_record(value) {
                self.record_field_omission(graph, object, field, XmlWaiverFamily::SourceRecord);
                continue;
            }
            self.account_field(graph, object, field);
            let mut rendered = XmlElement::with_attributes("FIELD", [("NAME", field.as_str())]);
            if field == "body" {
                let missing: Vec<String> = parameters
                    .iter()
                    .filter(|parameter| !self.bound_variable_stack.contains(parameter))
                    .cloned()
                    .collect();
                self.bound_variable_stack.extend(missing.iter().cloned());
                let (declarations, body) = self.scoped_parts(
                    graph,
                    vec!["question-body".to_owned(), key.to_owned()],
                    |state, graph| state.generic_value(graph, value),
                );
                self.bound_variable_stack
                    .truncate(self.bound_variable_stack.len() - missing.len());
                Self::append_defs(&mut rendered, declarations);
                rendered.push(body);
            } else {
                let mut removed = Vec::new();
                for index in (0..self.bound_variable_stack.len()).rev() {
                    if parameters.contains(&self.bound_variable_stack[index]) {
                        removed.push((index, self.bound_variable_stack.remove(index)));
                    }
                }
                rendered.push(self.generic_value(graph, value));
                for (index, binder) in removed.into_iter().rev() {
                    self.bound_variable_stack.insert(index, binder);
                }
            }
            result.push(rendered);
        }
        result
    }

    #[requires(graph.ground_by_utterance.contains_key(key))]
    #[ensures(ret.name == "UTTERANCE")]
    fn render_utterance(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let ground = graph.ground_by_utterance[key].clone();
        let ground_id = self.render_ground_reference(graph, &ground);
        let scope = vec!["utterance-ground".to_owned(), key.to_owned()];
        self.enter_ground_scope(scope.clone());
        let speaker = string_field(object, "speaker");
        self.account_field(graph, object, "speaker");
        self.account_field(graph, object, "audience");
        self.account_field(graph, object, "deicticGround");
        let deictic_ground = json_object(&object["deicticGround"]);
        self.account_field(graph, deictic_ground, "time");
        self.account_field(graph, deictic_ground, "place");
        self.speaker_stack.push(speaker.to_owned());
        let declarations = self.ground_declarations(graph, &scope);
        let force = object
            .get("force")
            .map(enum_token)
            .unwrap_or_else(|| panic!("utterance lacks force"));
        self.account_field(graph, object, "force");
        let mut result =
            XmlElement::with_attributes("UTTERANCE", [("FORCE", force), ("GROUND", ground_id)]);
        Self::append_defs(&mut result, declarations);
        let mut handled = Vec::from([
            "type",
            "force",
            "speaker",
            "audience",
            "eventuality",
            "deicticGround",
        ]);
        if let Some(locution_key) = optional_string(object, "eventuality") {
            self.account_field(graph, object, "eventuality");
            assert_eq!(
                graph
                    .object(locution_key)
                    .get("sort")
                    .map(flat_sort_name)
                    .as_deref(),
                Some("Locution"),
                "utterance eventuality is not Locution: {locution_key:?}"
            );
            let mut locution = self.render_pointer(graph, locution_key);
            if Self::is_reference(&locution) {
                result.push(XmlElement::with_attributes(
                    "LOCUTION",
                    [("REF", locution.attributes[0].1.clone())],
                ));
            } else {
                locution.name = "LOCUTION".to_owned();
                result.push(locution);
            }
        }
        if let Some(content) = optional_string(object, "content") {
            self.account_field(graph, object, "content");
            result.push(self.scoped_pointer(
                graph,
                "CONTENT",
                content,
                vec!["utterance-content".to_owned(), key.to_owned()],
            ));
            handled.push("content");
        }
        if let Some(kind) = object.get("vocativeKind") {
            self.account_field(graph, object, "vocativeKind");
            result.push(Self::scalar("VOCATIVE-KIND", kind));
            handled.push("vocativeKind");
        }
        if let Some(asides) = object.get("asides").and_then(Value::as_array) {
            self.account_field(graph, object, "asides");
            let mut rendered = XmlElement::new("ASIDES");
            for aside in asides {
                rendered.push(
                    self.wrap_pointer(
                        graph,
                        "ASIDE",
                        aside
                            .as_str()
                            .unwrap_or_else(|| panic!("aside must be an id")),
                        Vec::new(),
                    ),
                );
            }
            result.push(rendered);
            handled.push("asides");
        }
        result.extend(self.extras(graph, object, &handled));
        assert_eq!(
            self.speaker_stack.pop().as_deref(),
            Some(speaker),
            "unbalanced utterance speaker stack"
        );
        self.leave_ground_scope(&scope);
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "PREDICATION")]
    fn render_predication(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let mut handled = Vec::from([
            "type",
            "eventuality",
            "relation",
            "relationParameter",
            "arguments",
            "adjuncts",
            "mode",
        ]);
        let mut result = XmlElement::new("PREDICATION");
        let relation_value = if let Some(relation) = optional_string(object, "relation") {
            self.account_field(graph, object, "relation");
            result.set("PREDICATE", predicate_symbol(relation));
            None
        } else if let Some(parameter) = optional_string(object, "relationParameter") {
            self.account_field(graph, object, "relationParameter");
            Some(self.wrap_pointer(graph, "RELATION", parameter, Vec::new()))
        } else {
            Some(XmlElement::new("MISSING-RELATION"))
        };
        result.set(
            "MODE",
            object
                .get("mode")
                .map(enum_token)
                .unwrap_or_else(|| "UNKNOWN".to_owned()),
        );
        if object.contains_key("mode") {
            self.account_field(graph, object, "mode");
        }
        if let Some(relation) = relation_value {
            result.push(relation);
        }
        if let Some(eventuality) = optional_string(object, "eventuality") {
            self.account_field(graph, object, "eventuality");
            result.push(self.wrap_pointer(graph, "EVENT", eventuality, Vec::new()));
        }
        if let Some(arguments) = object.get("arguments").and_then(Value::as_object) {
            self.account_field(graph, object, "arguments");
            for place in sorted_places(arguments) {
                self.account_field(graph, arguments, place);
                let fill = self
                    .active_fill_site
                    .as_ref()
                    .is_some_and(|site| site.0 == key && site.1 == place);
                if fill {
                    self.fill_marks += 1;
                }
                result.push(self.render_argument(
                    graph,
                    json_object(&arguments[place]),
                    Some(place_label(place)),
                    fill,
                ));
            }
        }
        if !self.suppress_predication_adjuncts()
            && let Some(adjuncts) = object.get("adjuncts").and_then(Value::as_array)
        {
            self.account_field(graph, object, "adjuncts");
            for adjunct in adjuncts {
                result.push(self.render_added_place(
                    graph,
                    json_object(adjunct),
                    optional_string(object, "eventuality"),
                ));
            }
        }

        let mut metadata = XmlElement::new("META");
        if let Some(questions) = object.get("placeQuestions") {
            self.account_field(graph, object, "placeQuestions");
            let mut rendered = XmlElement::new("PLACE-QUESTIONS");
            rendered.push(self.generic_value(graph, questions));
            metadata.push(rendered);
            handled.push("placeQuestions");
        }
        if let Some(link) = object.get("tanruLink").and_then(Value::as_object) {
            self.account_field(graph, object, "tanruLink");
            let mut rendered = XmlElement::new("TANRU-LINK");
            if let Some(head) = optional_string(link, "head") {
                self.account_field(graph, link, "head");
                rendered.push(self.wrap_pointer(graph, "HEAD", head, Vec::new()));
            }
            if let Some(modifier) = optional_string(link, "modifier") {
                self.account_field(graph, link, "modifier");
                rendered.push(self.wrap_pointer(graph, "MODIFIER", modifier, Vec::new()));
            }
            if let Some(label) = optional_string(link, "relationLabel") {
                self.account_field(graph, link, "relationLabel");
                rendered.push(XmlElement::with_attributes(
                    "RELATION-LABEL",
                    [("VALUE", label)],
                ));
            }
            rendered.extend(self.extras(graph, link, &["head", "modifier", "relationLabel"]));
            metadata.push(rendered);
            handled.push("tanruLink");
        }
        if let Some(negation) = object.get("scalarNegation").and_then(Value::as_object) {
            self.account_field(graph, object, "scalarNegation");
            metadata.push(self.render_scalar_negation(graph, negation));
            handled.push("scalarNegation");
        }
        metadata.extend(self.extras(graph, object, &handled));
        if !metadata.children.is_empty() {
            result.push(metadata);
        }
        result
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_formula(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        if object.contains_key("boundEventualities") {
            self.account_field(graph, object, "boundEventualities");
        }
        let bound = object
            .get("boundEventualities")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        self.render_bound_formula(graph, key, object, bound, 0)
    }

    #[requires(index <= bound.len())]
    #[ensures(!ret.name.is_empty())]
    fn render_bound_formula(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
        bound: &[Value],
        index: usize,
    ) -> XmlElement {
        if index >= bound.len() {
            return self.render_formula_core(graph, key, object);
        }
        let event_key = bound[index]
            .as_str()
            .unwrap_or_else(|| panic!("bound eventuality must be an id"));
        assert_eq!(
            graph
                .event_binding_owners
                .get(event_key)
                .map(String::as_str),
            Some(key),
            "eventuality is not owned by this formula: {event_key:?}"
        );
        assert_eq!(
            optional_string(graph.object(event_key), "denotation"),
            Some("generated-bound"),
            "bound eventuality is not generated-bound: {event_key:?}"
        );
        let binder = self.render_binder_definition(graph, event_key, "EXISTS binder");
        let scope = vec![
            "event-quantifier-body".to_owned(),
            key.to_owned(),
            event_key.to_owned(),
        ];
        let (declarations, body) = self.scoped_parts(graph, scope, |state, graph| {
            state.render_bound_formula(graph, key, object, bound, index + 1)
        });
        let mut result = XmlElement::new("EXISTS");
        result.push(binder);
        let mut body_node = XmlElement::new("BODY");
        Self::append_defs(&mut body_node, declarations);
        body_node.push(body);
        result.push(body_node);
        result
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_formula_core(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let operator = optional_string(object, "operator").unwrap_or("unknown");
        if object.contains_key("operator") {
            self.account_field(graph, object, "operator");
        }
        if matches!(operator, "cardinality" | "forall" | "exists") {
            self.render_explicit_quantifier(graph, key, object, operator)
        } else {
            self.render_nonquantifier_formula(graph, key, object, operator)
        }
    }

    #[requires(matches!(operator, "cardinality" | "forall" | "exists"))]
    #[ensures(ret.name == enum_string(operator))]
    fn render_explicit_quantifier(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
        operator: &str,
    ) -> XmlElement {
        let variable = string_field(object, "variable");
        self.account_field(graph, object, "variable");
        let binder = self.render_binder_definition(
            graph,
            variable,
            &format!("{} binder", enum_string(operator)),
        );
        self.bound_variable_stack.push(variable.to_owned());
        let scope = vec!["quantifier-body".to_owned(), key.to_owned()];
        let (declarations, content) = self.scoped_parts(graph, scope, |state, graph| {
            let mut handled = Vec::from(["type", "operator", "boundEventualities", "variable"]);
            let mut content: HashMap<&str, XmlElement> = HashMap::new();
            for (field, tag) in [("restriction", "RESTRICTION"), ("body", "BODY")] {
                if let Some(pointer) = optional_string(object, field) {
                    state.account_field(graph, object, field);
                    content.insert(field, state.wrap_pointer(graph, tag, pointer, Vec::new()));
                    handled.push(field);
                }
            }
            if let Some(quantity_key) = optional_string(object, "quantity") {
                state.account_field(graph, object, "quantity");
                let quantity = state.render_pointer(graph, quantity_key);
                let rendered = if operator == "cardinality" {
                    if Self::is_reference(&quantity) {
                        XmlElement::with_attributes(
                            "CARD",
                            [("REF", quantity.attributes[0].1.clone())],
                        )
                    } else {
                        let mut card = XmlElement::new("CARD");
                        card.push(quantity);
                        card
                    }
                } else if Self::is_reference(&quantity) {
                    XmlElement::with_attributes(
                        "QUANTITY-VALUE",
                        [("REF", quantity.attributes[0].1.clone())],
                    )
                } else {
                    quantity
                };
                content.insert("quantity", rendered);
                handled.push("quantity");
            }
            if let Some(import) = object.get("domainImport") {
                state.account_field(graph, object, "domainImport");
                content.insert("domainImport", Self::scalar("DOMAIN-IMPORT", import));
                handled.push("domainImport");
            }
            if let Some(connector) = object.get("connector").and_then(Value::as_object) {
                state.account_field(graph, object, "connector");
                content.insert("connector", state.render_connector(graph, connector));
                handled.push("connector");
            }
            let mut extras = XmlElement::new("EXTRAS");
            extras.extend(state.extras(graph, object, &handled));
            content.insert("extras", extras);
            content
        });
        assert_eq!(
            self.bound_variable_stack.pop().as_deref(),
            Some(variable),
            "unbalanced quantifier variable stack"
        );
        let mut result = XmlElement::new(enum_string(operator));
        result.push(binder);
        Self::append_defs(&mut result, declarations);
        let mut content = content;
        if operator == "cardinality"
            && let Some(quantity) = content.remove("quantity")
        {
            result.push(quantity);
        }
        if operator == "exists" {
            if let Some(restriction) = content.remove("restriction") {
                result.push(restriction);
            }
        } else {
            result.push(
                content
                    .remove("restriction")
                    .unwrap_or_else(|| XmlElement::new("RESTRICTION")),
            );
        }
        result.push(
            content
                .remove("body")
                .unwrap_or_else(|| XmlElement::new("BODY")),
        );
        if operator != "cardinality"
            && let Some(quantity) = content.remove("quantity")
        {
            result.push(quantity);
        }
        for field in ["domainImport", "connector"] {
            if let Some(value) = content.remove(field) {
                result.push(value);
            }
        }
        if let Some(extras) = content.remove("extras") {
            result.extend(extras.children);
        }
        result
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    fn render_nonquantifier_formula(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
        operator: &str,
    ) -> XmlElement {
        let scope = vec!["formula-operands".to_owned(), key.to_owned()];
        let (declarations, parts) = self.scoped_parts(graph, scope, |state, graph| {
            let mut handled = Vec::from(["type", "operator", "boundEventualities"]);
            let mut operands = Vec::new();
            if operator == "atom" {
                if let Some(predication) = optional_string(object, "predication") {
                    state.account_field(graph, object, "predication");
                    operands.push(state.render_pointer(graph, predication));
                    handled.push("predication");
                } else {
                    operands.push(XmlElement::new("MISSING-PREDICATION"));
                }
            } else if matches!(operator, "and" | "or" | "not") {
                if let Some(children) = object.get("children").and_then(Value::as_array) {
                    state.account_field(graph, object, "children");
                    operands.extend(children.iter().map(|child| {
                        state.render_pointer(
                            graph,
                            child
                                .as_str()
                                .unwrap_or_else(|| panic!("formula child must be an id")),
                        )
                    }));
                }
                handled.push("children");
            } else {
                for field in [
                    "children",
                    "predication",
                    "variable",
                    "restriction",
                    "body",
                    "quantity",
                    "domainImport",
                ] {
                    if let Some(value) = object.get(field) {
                        state.account_field(graph, object, field);
                        let mut operand = XmlElement::with_attributes(
                            "OPERAND",
                            [("ROLE", enum_string(field).replace('_', "-"))],
                        );
                        operand.push(state.generic_value(graph, value));
                        operands.push(operand);
                        handled.push(field);
                    }
                }
            }
            let mut post = Vec::new();
            if let Some(connector) = object.get("connector").and_then(Value::as_object) {
                state.account_field(graph, object, "connector");
                post.push(state.render_connector(graph, connector));
                handled.push("connector");
            }
            let extras = state.extras(graph, object, &handled);
            (operands, post, extras)
        });
        let (operands, post, extras) = parts;
        if operator == "atom" && declarations.is_empty() && post.is_empty() && extras.is_empty() {
            assert_eq!(operands.len(), 1, "atom formula has invalid operand count");
            return operands.into_iter().next().expect("one operand");
        }
        let mut core = match operator {
            "and" | "or" => {
                XmlElement::with_attributes("CONNECTIVE", [("OPERATOR", enum_string(operator))])
            }
            "not" => XmlElement::new("NEGATION"),
            "atom" => XmlElement::new("FORMULA"),
            _ => XmlElement::with_attributes("FORMULA", [("OPERATOR", enum_string(operator))]),
        };
        Self::append_defs(&mut core, declarations);
        core.extend(operands);
        if post.is_empty() && extras.is_empty() {
            return core;
        }
        if core.name == "FORMULA" {
            core.extend(post);
            core.extend(extras);
            return core;
        }
        let mut wrapper = XmlElement::new("FORMULA");
        wrapper.push(core);
        wrapper.extend(post);
        wrapper.extend(extras);
        wrapper
    }
}
