//! Canonical SFN-XML rendering for `lojban-semantics-json-1`.
//!
//! This is a faithful Rust port of `render_xml.py` at research commit
//! `e25eeaf09bab4f14eea98e73cd1244ac464346da`.  Like the frozen `smusni`
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

const SCOPE_DEPENDENCE_TEACHING: &str = "A referent mentioned inside a quantifier may either be one shared thing for all values of the bound variable, or a possibly different thing per value — the text does not decide, and nothing is marked in that default case. SAME-FOR-ALL marks the exceptions known to be one and the same across all values.";

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
        "some",
        "REF=\"SOME\" denotes a distinct elided node per occurrence; distinct nodes assert neither identity (=) nor non-identity (≠).",
    ),
    (
        "ground",
        "DEICTIC-GROUND is the shared speech-situation unit selected by UTTERANCE GROUND=. Its g-prefixed ID is a notation-level rendering id because the JSON has no ground object id; if the graph gains context ids, the rendering must align. Ground units share one definition ⇔ their SPEAKER-REF/AUDIENCE-REF/TIME-REF/PLACE-REF graph referents are pairwise identical.",
    ),
    (
        "quantifiers",
        "EXISTS, FORALL, and CARDINALITY are binder elements; VARIABLE defines its variable ID=/SORT= at the binder site; use sites carry REF=. RESTRICTION and BODY are loud sibling elements; EXISTS has no RESTRICTION; FORALL and CARDINALITY always write RESTRICTION explicitly, empty as RESTRICTION/.",
    ),
    ("scope-dependence", SCOPE_DEPENDENCE_TEACHING),
    (
        "scope-dependence-subsets",
        "POSSIBLY-DIFFERENT-PER= is a space-separated list of the enclosing bound-variable ids on which a referent may depend when that list is a strict subset of all enclosing binders.",
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

#[requires(true)]
#[ensures(true)]
fn walk_pointer_values(value: &Value, object_keys: &HashSet<String>, output: &mut Vec<String>) {
    match value {
        Value::String(value) if object_keys.contains(value) => output.push(value.clone()),
        Value::Array(items) => {
            for item in items {
                walk_pointer_values(item, object_keys, output);
            }
        }
        Value::Object(object) => {
            for (field, item) in object {
                if field == "source" && is_source_record(item) {
                    continue;
                }
                walk_pointer_values(item, object_keys, output);
            }
        }
        _ => {}
    }
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

// Validated once in `from_value`; fields are private and never mutated.
#[invariant(objects.contains_key(root), "the root must name a graph object")]
#[invariant(
    object_keys.len() == objects.len()
        && order.len() == objects.len()
        && ids.len() == objects.len(),
    "all object-keyed indexes must cover the graph"
)]
#[expensive_invariant(
    object_keys.iter().all(|key| objects.contains_key(key))
        && objects.keys().all(|key| object_keys.contains(key))
        && order.keys().all(|key| object_keys.contains(key))
        && ids.keys().all(|key| object_keys.contains(key)),
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

        let mut uses: HashMap<String, usize> =
            object_keys.iter().map(|key| (key.clone(), 0)).collect();
        *uses.get_mut(&root).expect("root belongs to objects") += 1;
        for object in objects.values() {
            let mut pointers = Vec::new();
            walk_pointer_values(object, &object_keys, &mut pointers);
            for pointer in pointers {
                *uses
                    .get_mut(&pointer)
                    .expect("walk only yields object keys") += 1;
            }
        }

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

        let mut event_binding_owners = HashMap::new();
        let mut quantifier_owners = HashMap::new();
        let mut quantifier_restrictions = HashSet::new();
        for (owner, value) in &objects {
            let object = json_object(value);
            if let Some(bound) = object.get("boundEventualities").and_then(Value::as_array) {
                for event in bound {
                    let event = event
                        .as_str()
                        .unwrap_or_else(|| panic!("bound eventuality must be an id"));
                    assert!(
                        event_binding_owners
                            .insert(event.to_owned(), owner.clone())
                            .is_none(),
                        "eventuality has multiple binders: {event:?}"
                    );
                }
            }
            if optional_string(object, "type") == Some("formula")
                && matches!(
                    optional_string(object, "operator"),
                    Some("exists" | "forall" | "cardinality")
                )
                && let Some(variable) = optional_string(object, "variable")
            {
                assert!(
                    quantifier_owners
                        .insert(variable.to_owned(), owner.clone())
                        .is_none(),
                    "variable has multiple quantifier binders: {variable:?}"
                );
                if matches!(
                    optional_string(object, "operator"),
                    Some("forall" | "cardinality")
                ) && let Some(restriction) = optional_string(object, "restriction")
                {
                    quantifier_restrictions.insert((variable.to_owned(), restriction.to_owned()));
                }
            }
        }
        let special_definition_keys: HashSet<String> = context_sites
            .keys()
            .chain(event_binding_owners.keys())
            .chain(quantifier_owners.keys())
            .cloned()
            .collect();
        let id_keys: HashSet<String> = objects
            .keys()
            .filter(|key| uses.get(*key).copied().unwrap_or_default() > 1)
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
            self.unaccounted_surfaces.remove(&surface),
            "omitted semantic surface was already accounted: {}",
            surface.path()
        );
        let descendant_prefix = format!("{}/", surface.path());
        self.unaccounted_surfaces.retain(|candidate| {
            candidate.path() != surface.path() && !candidate.path().starts_with(&descendant_prefix)
        });
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
        self.planning = true;
        self.initial_planning_pass = initial;
        self.scope_parent.clear();
        self.ground_scope_parent.clear();
        self.pointer_use_scopes.clear();
        self.use_counter = 0;
    }

    #[requires(self.planning)]
    #[ensures(!self.planning && !self.initial_planning_pass)]
    fn finish_planning_pass(&mut self) {
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

    #[requires(true)]
    #[ensures(graph.ordinary_definition_keys.iter().all(|key| self.declaration_scopes.contains_key(key)))]
    fn plan_declaration_scopes(&mut self, graph: &GraphData) {
        if graph.ordinary_definition_keys.is_empty() {
            return;
        }
        self.start_planning_pass(true);
        let _ = self.render_pointer(graph, &graph.root);
        for key in &graph.ordinary_definition_keys {
            let scopes = self
                .pointer_use_scopes
                .get(key)
                .unwrap_or_else(|| panic!("shared node has no reachable use site: {key:?}"));
            self.declaration_scopes
                .insert(key.clone(), self.least_common_scope(scopes));
        }
        self.finish_planning_pass();
        self.rebuild_scope_declarations(graph);

        for _ in 0..=graph.objects.len() {
            let previous = self.declaration_scopes.clone();
            self.start_planning_pass(false);
            let _ = self.render_pointer(graph, &graph.root);
            let planned: HashMap<String, Scope> = graph
                .ordinary_definition_keys
                .iter()
                .map(|key| {
                    let scopes = self.pointer_use_scopes.get(key).unwrap_or_else(|| {
                        panic!("shared node has no reachable use site: {key:?}")
                    });
                    (key.clone(), self.least_common_scope(scopes))
                })
                .collect();
            self.finish_planning_pass();
            self.declaration_scopes = planned;
            self.rebuild_scope_declarations(graph);
            if self.declaration_scopes == previous {
                return;
            }
        }
        panic!("shared-node scope planning did not converge");
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
        let _ = self.render_pointer(graph, &graph.root);
        self.finish_planning_pass();
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
            self.apply_scope_dependence(graph, &mut result, scope);
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
            let parameters = object
                .get("parameters")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            for parameter in parameters {
                self.bound_variable_stack.push(
                    parameter
                        .as_str()
                        .unwrap_or_else(|| panic!("referent parameter must be an id"))
                        .to_owned(),
                );
            }
            let body = self.scoped_pointer(
                graph,
                "BODY",
                body_key,
                vec!["description-body".to_owned(), key.to_owned()],
            );
            self.bound_variable_stack
                .truncate(self.bound_variable_stack.len() - parameters.len());
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
            result.push(self.scoped_pointer(
                graph,
                "CONTENT",
                content,
                vec!["description-content".to_owned(), key.to_owned()],
            ));
        }
        if let Some(parameters) = object.get("parameters").and_then(Value::as_array) {
            self.account_field(graph, object, "parameters");
            result.set(
                "PARAMETERS",
                self.pointer_list(graph, parameters, "PARAMETERS"),
            );
        }
        if let Some(arity) = object.get("arity") {
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
                return XmlElement::with_attributes(
                    "INTEGER",
                    [("VALUE", scalar_string(&literal["value"]))],
                );
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

impl RenderState {
    #[requires(true)]
    #[ensures(ret.ends_with('\n'))]
    fn render_document(&mut self, graph: &GraphData, document_name: &str) -> String {
        let document_scope = vec!["document".to_owned()];
        self.enter_ground_scope(document_scope.clone());
        let document_declarations = self.ground_declarations(graph, &document_scope);
        let graph_root = self.render_pointer(graph, &graph.root);
        self.leave_ground_scope(&document_scope);

        let mut unreachable: Vec<String> = graph
            .objects
            .keys()
            .filter(|key| !self.emitted.contains(*key))
            .cloned()
            .collect();
        unreachable.sort_by_key(|key| graph.id(key).to_owned());
        let unreachable = if unreachable.is_empty() {
            None
        } else {
            let mut rendered = XmlElement::new("UNREACHABLE");
            for key in unreachable {
                rendered.push(self.render_pointer(graph, &key));
            }
            Some(rendered)
        };
        assert_eq!(
            self.emitted, graph.object_keys,
            "some graph objects were not rendered"
        );
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
        let mut waivers = XmlElement::new("WAIVERS");
        for message in waiver_messages(&self.omissions) {
            let mut waiver = XmlElement::new("WAIVER");
            waiver.text = Some(message);
            waivers.push(waiver);
        }
        root.push(waivers);
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
fn render_xml_value(graph: Value, document_name: &str) -> XmlRender {
    let graph = GraphData::from_value(graph);
    let mut state = RenderState::new();
    state.plan_declaration_scopes(&graph);
    state.plan_ground_scopes(&graph);
    state.reset_traversal_state();
    state.start_omission_accounting(&graph);
    let output = state.render_document(&graph, document_name);
    let omissions = state.omissions;
    new!(XmlRender { output, omissions })
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

    #[requires(true)]
    #[ensures(output.len() >= old(output.len()))]
    fn collect_declared_waiver_occurrences(
        value: &Value,
        path: &str,
        descriptor: bool,
        output: &mut BTreeSet<XmlOmission>,
    ) {
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    collect_declared_waiver_occurrences(
                        item,
                        &format!("{path}/{index}"),
                        false,
                        output,
                    );
                }
            }
            Value::Object(object) => {
                if descriptor
                    && !matches!(optional_string(object, "kind"), Some("elided" | "proSumti"))
                    && object.get("word").is_some_and(Value::is_string)
                {
                    output.insert(new!(XmlOmission {
                        waiver: Some(XmlWaiverFamily::DescriptorWord),
                        surface: field_surface(format!("{path}/word")),
                    }));
                }
                if optional_string(object, "type") == Some("referent")
                    && optional_string(object, "category") == Some("variable")
                    && let Some(descriptor) = object.get("descriptor").and_then(Value::as_object)
                    && optional_string(descriptor, "kind") == Some("proSumti")
                    && descriptor.get("word").is_some_and(Value::is_string)
                {
                    output.insert(new!(XmlOmission {
                        waiver: Some(XmlWaiverFamily::BoundVariableWord),
                        surface: field_surface(format!("{path}/descriptor/word")),
                    }));
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
                        field == "descriptor",
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
        collect_declared_waiver_occurrences(graph, "", false, &mut occurrences);
        occurrences
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn frozen_xml_corpus_is_exact_and_pinned() {
        assert_eq!(XML_CORPUS_DOCS.len(), 46);
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
            "f01d67efab5d2d90473481d80702dfa593d47747447546c63b1bb821bd0cf102"
        );
        assert_eq!(
            aggregate_hash("xml.txt"),
            "f1bb35a70f818bf15850bfb74f3986fa69900431b88a59d8dcb1919bd595d66d"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xml_matches_the_frozen_prototype_on_all_46_documents() {
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
                (XmlWaiverFamily::SourceRecord, 605),
                (XmlWaiverFamily::AssignedNameRecord, 3),
                (XmlWaiverFamily::DescriptorWord, 54),
                (XmlWaiverFamily::IntroducedBy, 232),
                (XmlWaiverFamily::QuantityText, 11),
                (XmlWaiverFamily::BoundVariableWord, 8),
            ])
        );
        assert_eq!(counts.values().sum::<usize>(), 913);
        assert_eq!(
            documents
                .into_iter()
                .map(|(family, documents)| (family, documents.len()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                (XmlWaiverFamily::SourceRecord, 46),
                (XmlWaiverFamily::AssignedNameRecord, 2),
                (XmlWaiverFamily::DescriptorWord, 34),
                (XmlWaiverFamily::IntroducedBy, 44),
                (XmlWaiverFamily::QuantityText, 7),
                (XmlWaiverFamily::BoundVariableWord, 5),
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
    fn xml_rendering_is_deterministic_on_all_46_documents() {
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
                    } else if matches!(
                        field.as_str(),
                        "type" | "sort" | "denotation" | "category" | "indexical"
                    ) {
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
            panic!("node used before its semantic definition site: {key:?}");
        }
        assert!(
            !self.emitted.contains(key),
            "single-use node emitted more than once: {key:?}"
        );
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
                assert!(
                    dependency_set.is_subset(&active_set),
                    "scopeDependence mayDependOn contains a non-enclosing binder"
                );
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
        if kind == "elided" && value.contains_key("word") {
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
            self.apply_scope_dependence(graph, &mut variable, scope);
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
    (candidates.len() == 1).then_some(candidates[0])
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
            walk_pointer_values(value, &graph.object_keys, &mut pointers);
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

impl RenderState {
    #[requires(graph.objects.contains_key(key))]
    #[ensures(!ret.name.is_empty())]
    fn render_object(&mut self, graph: &GraphData, key: &str) -> XmlElement {
        let object = graph.object(key);
        self.account_object(graph, object);
        if object.contains_key("type") {
            self.account_field(graph, object, "type");
        }
        match optional_string(object, "type") {
            Some("utterance") => self.render_utterance(graph, key, object),
            Some("predication") => self.render_predication(graph, key, object),
            Some("formula") => self.render_formula(graph, key, object),
            Some("referent") => self.render_referent(graph, key, object),
            Some("quantity") => self.render_quantity(graph, object),
            Some("parameter") => self.render_parameter(graph, object),
            Some("sequence") => self.render_sequence(graph, key, object),
            Some("displayedContent") => self.render_displayed_content(graph, object),
            Some("mathExpression") => self.render_math_expression(graph, object),
            _ => self.render_unknown_object(graph, object),
        }
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
        if let Some(adjuncts) = object.get("adjuncts").and_then(Value::as_array) {
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
            assert!(
                !content.contains_key("restriction"),
                "EXISTS cannot carry a RESTRICTION in SFN-XML"
            );
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
                    "boundEventualities",
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
