//! Canonical SFN-XML rendering for `lojban-semantics-json-1`.
//!
//! This is a faithful Rust port of `render_xml.py` at research commit
//! `51c19cf18d1df2e880744fc9ceb2846d92338571`. Like the frozen `smusni`
//! renderer, it deliberately walks [`SemanticGraph`]'s own canonical JSON
//! serialization: the notation is specified over that interchange surface, and
//! using it directly avoids a second, drift-prone reconstruction of the serde
//! shape.  The XML emitter and scope planner are independent of `smusni`; the
//! two renderers share only this justified canonical-JSON boundary.

use std::collections::{BTreeSet, HashMap, HashSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use serde_json::{Map, Value};

use crate::model::{
    SEMANTIC_JSON_VERSION, SemanticGraph, semantic_scope_dependence_binder_universes,
};
use crate::notation::word_cards::WordCard;
use crate::notation::xml_words::words_section;

const SCOPE_DEPENDENCE_TEACHING: &str = "Scope-dependence markers compare a referent's scopeDependence with the semantic graph derivation's authoritative first-visit binder universe, which is not a lexical-enclosure set. Silence means that the referent may depend on the full first-visit universe. SAME-FOR-ALL means that it is fixed despite a nonempty first-visit universe.";

const KEY_RULES_BEFORE_SORTS: &[&str] = &[
    "ID= marks the definition of a shared graph node except a speech-situation referent; DEICTIC-GROUND SPEAKER-REF=/AUDIENCE-REF=/TIME-REF=/PLACE-REF= values are the sole definition sites for those referent ids. REF= and named *-REF= attributes point to discourse referents. GROUND= points to a deictic-ground unit and is the sole suffix exception. Later REF= occurrences are exact-node reuse. Graph-node ids are opaque and JSON-aligned; distinct ids assert neither identity (=) nor non-identity (≠).",
    "UPPERCASE = structural keywords (element and attribute names); PascalCase = sorts; lowercase = content words as data values only; quoted attribute strings = names.",
];

const KEY_RULES_AFTER_SORTS: &[&str] = &[
    "EMBEDDED-QUESTIONS preserves typed QUESTION metadata attached to the abstraction formula named by QUESTION/BODY. KIND, MODE, and DOMAIN classify the question; ASKER, RESPONDENT, SLOTS, FOCUS, and PRESUPPOSED-ANSWER preserve its participants. SLOT parameters bind only the question BODY.",
    "KIND-COMPOSITION in the relation slot of a PREDICATION is the predication's composed predicate, replacing PREDICATE= exactly when the predicate is composite (the same compact/composite dichotomy as ADJUNCT, one level up): MODIFIER A of KIND B denotes an A-modified kind of B while retaining B's place structure and eventuality. Predicating the composition entails the KIND predication of the enclosing PREDICATION's EVENT and ARGs together with an underspecified modification connection between KIND's exposed place-1 participant and the relation denoted by MODIFIER; it does not by itself entail that the participant satisfies MODIFIER. Silence is the underspecification, not intersection: when a speaker makes the stronger claim, it appears as its own conjunct. CONNECTION= is reserved for a future resolved-connection marker and is never emitted; its absence means UNSPECIFIED.",
    "PREDICATE= on KIND, MODIFIER, or a relation-expression RELATION leaf is the compact lexical form: PARTICIPANT-PLACE= marks the lexical place the composition participant fills and defaults to 1, each omitted unfilled lexical place elaborates to a distinct ordinary elided-place node, and the operand's own eventuality is fresh and locally existentially bound. BODY wraps a composite operand instead: a nested KIND-COMPOSITION, a CONNECTIVE conjoining co-modifiers of the same head participant, or a RELATION lambda or abstraction subtree.",
    "GROUPING= on KIND-COMPOSITION states the basis of the displayed composition tree; silence means ASSUMED-LEFT, the deterministic grammar default. EXPLICIT appears only where the text itself encodes the grouping: at the tree root when every edge is explicit, per node when one tree mixes explicit and default edges. CONNECTION= and GROUPING= defaults are semantic defaults of this notation, not a generic rule for arbitrary absent attributes.",
    "CONNECTIVE carries OPERATOR= and, only when the graph records a truth table that OPERATOR= does not already determine, TRUTH-TABLE=. A connective question's bound parameter appears as the PARAMETER= attribute of its connective formula. The surface connective word and its grammatical locus are derivational provenance and never appear in default output.",
    "Each semantic diagnostic is one repeatable WARNING text element; its character content is the diagnostic message.",
    "REF=\"SOME\" denotes a distinct elided node per occurrence; distinct nodes assert neither identity (=) nor non-identity (≠).",
    "DEICTIC-GROUND is the shared speech-situation unit selected by UTTERANCE GROUND=. Its g-prefixed ID is a notation-level rendering id because the JSON has no ground object id; if the graph gains context ids, the rendering must align. Ground units share one definition ⇔ their SPEAKER-REF/AUDIENCE-REF/TIME-REF/PLACE-REF graph referents are pairwise identical.",
    "EXISTS, FORALL, and CARDINALITY are binder elements; VARIABLE defines its variable ID=/SORT= at the binder site; use sites carry REF=. RESTRICTION and BODY are loud sibling elements; EXISTS writes RESTRICTION exactly when the graph supplies one; FORALL and CARDINALITY always write RESTRICTION explicitly, empty as RESTRICTION/.",
    SCOPE_DEPENDENCE_TEACHING,
    "POSSIBLY-DIFFERENT-PER= is a space-separated strict nonempty subset of the first-visit quantifier-variable or abstraction/question-parameter ids on which a referent may depend.",
    "References are number-neutral: a reference may denote one or several individuals; the only number commitments are explicit quantities on descriptions, cardinality binders, or mass restrictions.",
    "PERSONAL-MASS-MEMBERSHIP states whether SPEAKER and AUDIENCE are INCLUDED or EXCLUDED and points to any additional included OTHERS. DEICTIC-REFERENCE states PROXIMITY to a discourse-referent GROUND-REF. These structures carry the semantics; no pro-sumti label is implied.",
    "A description is anchored to its enclosing utterance's speaker. SPEAKER-REF= on a description, or BY inside NAMED, appears only when the anchor differs from that enclosing speaker.",
    "Absent facet attribute ⇒ UNSPECIFIED (no commitment). Facet attributes: TIME, ACTUALITY, ASPECT, RECURRENCE, SPACE, SPATIAL-ASPECT, SPATIAL-RECURRENCE, DETAILS.",
    "EVENT is the reserved first child of a PREDICATION, its Eventuality referent, never a numbered ARG.",
    "ADJUNCT introduces a predicate-keyed optional participant of the host predication; PREDICATE= with flat ARG children is the compact single-lexical-predicate place map; without PREDICATE=, BODY carries the composite predicate subtree; ARG FILL=\"true\" marks the unique explicit non-host filled place; a non-unique graph stays complete and carries FILL-STATUS; APPLIES-TO links the host component.",
    "UNRESOLVED-REFERENT WORD= is a word-only stopgap only for referents that remain unresolved after the jbotci#690 KOhA audit; the quoted WORD value is the stopgap's whole content. Bound-variable surface words are provenance-only.",
    "MODE vocabulary: ASSERTED=main claim; RESTRICTIVE=restriction; INCIDENTAL=side claim; INERT=embedded nonclaim; DEFINITIONAL=identity definition; PERFORMATIVE=speech act. MODE is a required attribute on PREDICATION.",
    "Every non-binder graph-node definition and every DEICTIC-GROUND definition sits in DEFS in the smallest graph scope strictly containing all uses; an attribute reference on element X is a use at X's position. Quantifier VARIABLE precedes its scope's DEFS; all graph-node uses outside their definition site are references.",
    "A list of simple ids or numbers is a space-separated NMTOKENS attribute, never a sequence of child elements; semantic structure remains element-valued.",
    "Child order is fixed per element; CONNECTIVE child order is semantically significant; childless elements self-close.",
];

/// KEY paragraphs appended exactly when the document carries a WORDS word-card
/// section (#709): documents without cards stay byte-identical.
const KEY_RULES_WORD_CARDS: &[&str] = &[
    "WORDS lists one WORD card per content word of the text; ID= is the card key. GLOSS, DEF, and NOTES are dictionary prose; inside DEF and NOTES, ARG INDEX=\"n\" marks the word's nth place — the same argument vocabulary as predications, so a place in a definition matches ARG INDEX on the word's predications. KNOWN=\"false\" marks a word with no dictionary definition; the default true is omitted.",
    "COMPOSITE-APPROX shows the mechanical composition of a dictionary-absent compound through the same KIND-COMPOSITION idiom as the body; it is suggestive, not definitional. COMPONENT WORD= references the component's own WORD card.",
    "PLACES=\"UNKNOWN\" means the composition tree determines no place structure. The actual meaning and places were chosen by the coiner. Places of COMPONENT cards are not inherited by the compound, and operators or grouping the coiner omitted leave no recoverable trace.",
    "On word cards, GROUPING= and SCOPE= describe the basis of the displayed tree, not the coiner's guaranteed intent: ASSUMED-LEFT = CLL default left grouping, ASSUMED-SHORT = CLL-12.12 narrow operator scope, EXPLICIT = the word itself encodes the boundary. The attributes are tree-level; they appear per-node only when one tree mixes explicit and assumed edges. Unlike the body scope, cards state ASSUMED-LEFT because a card tree is an approximation of an undefined word, not the deterministic grammar default.",
    "VARIABLE-CONTEXT denotes the abstract role an utterance context would supply for a context-dependent word used inside a definition. These are roles, not referents; they define no ids.",
    "WORD ID values are surface-spelling card keys: the canonical spelling for one-token words; multiword (zei) compounds join their parts with hyphens (mi-zei-do). Hyphens never occur inside a single Lojban word, so the two namespaces cannot collide.",
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
#[invariant(::CompositionRelationLabel => true)]
#[invariant(::ConnectorProvenance => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum XmlWaiverFamily {
    SourceRecord,
    AssignedNameRecord,
    DescriptorWord,
    IntroducedBy,
    QuantityText,
    BoundVariableWord,
    CompositionRelationLabel,
    /// Connector surface words and grammatical loci: provenance-class data that
    /// no semantic consumer reads (jbotci#719). Truth-conditional connector
    /// content (the operator, and a truth table the operator does not already
    /// determine) stays in default output.
    ConnectorProvenance,
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
    XmlWaiverFamily::CompositionRelationLabel,
    XmlWaiverFamily::ConnectorProvenance,
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

/// One item of an element's mixed content: interleaved text runs and child
/// elements on a single line (`<DEF>` place markup, #709).
// Per-variant `=> true` markers are audited no-ops: like `XmlElement` itself,
// this is mutable serializer construction state whose validity is established
// by the emission code and the canonical serializer, not by a wrapper.
#[invariant(::Text(_) => true)]
#[invariant(::Element(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MixedContent {
    Text(String),
    Element(XmlElement),
}

// Mutable XML construction state. Validity is established by the private
// constructors and canonical serializer rather than by a wrapper that would
// prohibit in-place tree assembly. `mixed` is the single-line mixed-content
// representation used by the WORDS section (#709): when it is non-empty,
// `text` and `children` must be empty (the serializer debug-asserts this) and
// the items are emitted inline between the open and close tags.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlElement {
    pub(crate) name: String,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) children: Vec<Self>,
    pub(crate) text: Option<String>,
    pub(crate) mixed: Vec<MixedContent>,
}

impl XmlElement {
    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    pub(crate) fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        assert!(!name.is_empty(), "XML element names cannot be empty");
        Self {
            name,
            attributes: Vec::new(),
            children: Vec::new(),
            text: None,
            mixed: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.name.is_empty())]
    pub(crate) fn with_attributes(
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
    pub(crate) fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
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
    pub(crate) fn push(&mut self, child: Self) {
        self.children.push(child);
    }

    #[requires(true)]
    #[ensures(true)]
    fn extend(&mut self, children: Vec<Self>) {
        self.children.extend(children);
    }

    /// Append one mixed-content item (a text run or an inline child element).
    /// Mixed content is mutually exclusive with `text`/`children` by
    /// construction discipline; the serializer debug-asserts it.
    #[requires(true)]
    #[ensures(self.mixed.len() == old(self.mixed.len()) + 1)]
    pub(crate) fn push_mixed(&mut self, item: MixedContent) {
        self.mixed.push(item);
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
    if !node.mixed.is_empty() {
        debug_assert!(
            node.children.is_empty() && node.text.is_none(),
            "mixed content cannot combine with text or children: {}",
            node.name
        );
        output.push('>');
        for item in &node.mixed {
            match item {
                MixedContent::Text(text) => output.push_str(&escape_text(text)),
                MixedContent::Element(child) => serialize_element_inline(child, output),
            }
        }
        output.push_str("</");
        output.push_str(&node.name);
        output.push('>');
        return;
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

/// Serialize one element without any indentation or newlines: the inline form
/// used inside mixed content (`<ARG INDEX="n"/>` inside `<DEF>`, #709).
#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn serialize_element_inline(node: &XmlElement, output: &mut String) {
    output.push('<');
    output.push_str(&node.name);
    for (name, value) in &node.attributes {
        output.push(' ');
        output.push_str(name);
        output.push_str("=\"");
        output.push_str(&escape_attribute(value));
        output.push('"');
    }
    if node.children.is_empty() && node.text.is_none() && node.mixed.is_empty() {
        output.push_str("/>");
        return;
    }
    output.push('>');
    if let Some(text) = &node.text {
        output.push_str(&escape_text(text));
    }
    for child in &node.children {
        serialize_element_inline(child, output);
    }
    for item in &node.mixed {
        match item {
            MixedContent::Text(text) => output.push_str(&escape_text(text)),
            MixedContent::Element(child) => serialize_element_inline(child, output),
        }
    }
    output.push_str("</");
    output.push_str(&node.name);
    output.push('>');
}

/// Serialize the document, optionally preceded by one `<!-- ... -->` prolog
/// comment. The comment carries the KEY teaching prose (jbotci#719: the former
/// structured `<KEY><RULE TOPIC=...>` block is a single comment now). The text
/// must be comment-safe: XML comments forbid `--` and a trailing `-`.
#[requires(comment.is_none_or(|comment| !comment.contains("--") && !comment.ends_with('-')))]
#[ensures(ret.ends_with('\n'))]
pub(crate) fn serialize(root: &XmlElement, comment: Option<&str>) -> String {
    let mut output = String::new();
    if let Some(comment) = comment {
        output.push_str("<!--\n");
        output.push_str(comment);
        output.push_str("\n-->\n");
    }
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
#[ensures(ret -> field == "tanruLink" && value.is_object())]
fn is_composition_link_field(parent: &Map<String, Value>, field: &str, value: &Value) -> bool {
    optional_string(parent, "type") == Some("predication")
        && field == "tanruLink"
        && value.is_object()
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

/// The truth table a binary logical formula operator already determines, in
/// the builder's row order (TT, TF, FT, FF), mirroring
/// `generated_truth_table_for_formula_operator`. `None` for operators with no
/// truth-functional reading: their recorded tables always render (jbotci#719).
#[requires(true)]
#[ensures(ret.is_none_or(|table| table.len() == 4))]
fn canonical_truth_table(operator: &str) -> Option<&'static str> {
    match operator {
        "and" => Some("TFFF"),
        "or" => Some("TTTF"),
        "iff" => Some("TFFT"),
        "whetherOrNot" => Some("TTFF"),
        _ => None,
    }
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
/// One declared reason a semantic graph cannot be represented truthfully by
/// the compact SFN prototype vocabulary. Records of this type are declared in
/// a TYPED-GRAPH document's `COMPACT-INCOMPATIBILITIES` section; the analysis
/// is format-independent, so tooling can also compute them without rendering
/// (jbotci#723, see [`analyze_compact_incompatibilities`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompactIncompatibility {
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
    /// The declared `KIND=` vocabulary value.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn kind(&self) -> &'static str {
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

    /// The exact `<INCOMPATIBILITY .../>` declaration line the renderer emits
    /// for this record in a TYPED-GRAPH document's `COMPACT-INCOMPATIBILITIES`
    /// section (jbotci#723).
    #[requires(true)]
    #[ensures(ret.starts_with("<INCOMPATIBILITY KIND=\"") && ret.ends_with("/>"))]
    pub fn declaration(&self) -> String {
        let mut output = String::new();
        serialize_element_inline(&render_compact_incompatibility(self), &mut output);
        output
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
    scope_dependence_binder_universes.len() <= objects.len(),
    "binder universes index only graph objects"
)]
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
    scope_dependence_binder_universes
        .keys()
        .all(|key| object_keys.contains(key))
        && objects.iter().all(|(key, object)| {
            json_object(object).contains_key("scopeDependence")
                == scope_dependence_binder_universes.contains_key(key)
        }),
    "binder universes have exactly the scope-dependence object domain"
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
    scope_dependence_binder_universes: HashMap<String, BTreeSet<String>>,
    subtype_pairs: Vec<(String, String)>,
    value_paths: HashMap<usize, String>,
    surface_paths: BTreeSet<XmlSurface>,
    /// The tanru-projection report (jbotci#719): which objects the recognition
    /// boundary consumed and their original JSON, so omission accounting can
    /// classify every removed surface (projected structure vs waived
    /// provenance). Empty when the graph has no tanru patterns.
    projection: crate::notation::relation_expression::TanruProjection,
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

        // jbotci#719: the recognition boundary. Proven tanru patterns become
        // typed relation expressions on their head predications; the consumed
        // head/link scaffolding is removed here so declaration planning,
        // reference counting, and rendering all see the projected graph. The
        // consumed objects' original JSON is kept for omission accounting.
        let (objects, projection) =
            crate::notation::relation_expression::project_tanru_compositions(&objects);

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
        let scope_dependence_binder_universes: HashMap<String, BTreeSet<String>> = graph_object
            .remove("scopeDependenceBinderUniverses")
            .map(|value| {
                serde_json::from_value::<HashMap<String, BTreeSet<String>>>(value)
                    .expect("scope-dependence binder universes must be string-set records")
            })
            .unwrap_or_default()
            .into_iter()
            // The universes are computed over the unprojected graph; tanru
            // projection consumes exactly the objects it reports, so pruning to
            // survivors keeps the exact-domain invariant below.
            .filter(|(key, _)| objects.contains_key(key))
            .collect();
        assert!(
            scope_dependence_binder_universes
                .keys()
                .all(|key| objects.contains_key(key))
                && objects.iter().all(|(key, object)| {
                    json_object(object).contains_key("scopeDependence")
                        == scope_dependence_binder_universes.contains_key(key)
                }),
            "scope-dependence binder universes must cover exactly every scopeDependence object"
        );
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
        // Consumed tanru-scaffolding objects keep their surfaces in the
        // inventory so omission accounting must classify each one explicitly
        // (waived provenance or rendered-by-projection) instead of silently
        // dropping them. Rewritten anchors keep only their vanished
        // children/connector subtrees: the surviving fields dedupe against
        // the rewritten formula's own surfaces.
        let mut consumed_value_paths = HashMap::new();
        for (key, value) in &projection.consumed_objects {
            index_value_paths(
                value,
                &format!("/objects/{}", json_pointer_escape(key)),
                &mut consumed_value_paths,
                &mut surface_paths,
            );
        }
        for (key, value) in &projection.rewritten_anchors {
            let object = json_object(value);
            for field in ["children", "connector"] {
                if let Some(field_value) = object.get(field) {
                    let field_path = format!("/objects/{}/{}", json_pointer_escape(key), field);
                    surface_paths.insert(field_surface(field_path.clone()));
                    index_value_paths(
                        field_value,
                        &field_path,
                        &mut consumed_value_paths,
                        &mut surface_paths,
                    );
                }
            }
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
            scope_dependence_binder_universes,
            subtype_pairs,
            value_paths,
            surface_paths,
            projection,
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
    /// Whether this render carries a WORDS word-card section (#709). When it
    /// does, predication `relationMetadata` subtrees dedupe into the nonce
    /// word's WORD card instead of a body `RELATION-METADATA` element. Set
    /// once by `render_indexed_graph_with_state` before planning so every
    /// pass (planning and final) makes the same emission decision.
    word_cards_present: bool,
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
            word_cards_present: false,
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
    #[ensures(graph.projection.consumed_objects.keys().all(|key| !self.unaccounted_surfaces.contains(&object_surface(format!("/objects/{key}")))))]
    fn start_omission_accounting(&mut self, graph: &GraphData) {
        assert!(!self.planning, "accounting cannot begin during planning");
        self.unaccounted_surfaces.clone_from(&graph.surface_paths);
        self.account_projected_consumption(graph);
    }

    /// Classify every surface the tanru projection consumed (jbotci#719):
    /// provenance fields keep their waiver families (the omissions API is
    /// unchanged — a consumed source record is still a source-record omission),
    /// and everything else is rendered-by-projection, so it leaves the
    /// unaccounted set with no omission at all.
    #[requires(true)]
    #[ensures(true)]
    fn account_projected_consumption(&mut self, graph: &GraphData) {
        if self.planning {
            return;
        }
        for (key, value) in &graph.projection.consumed_objects {
            let base = format!("/objects/{}", json_pointer_escape(key));
            self.waive_consumed_provenance(&base, json_object(value));
            assert!(
                remove_surface_subtree(&mut self.unaccounted_surfaces, &object_surface(base.clone())),
                "projected tanru object was already accounted: {base}"
            );
        }
        // Rewritten anchors survive as the head atom, so their own surfaces
        // render normally; only the vanished `children`/`connector` subtrees
        // need classification (connector source/locus is waived provenance,
        // the children list is rendered-by-projection).
        for (key, value) in &graph.projection.rewritten_anchors {
            let base = format!("/objects/{}", json_pointer_escape(key));
            let object = json_object(value);
            if let Some(connector) = object.get("connector").and_then(Value::as_object) {
                for connector_field in ["source", "locus"] {
                    if connector.contains_key(connector_field) {
                        self.record_omission(
                            XmlWaiverFamily::ConnectorProvenance,
                            field_surface(format!("{base}/connector/{connector_field}")),
                        );
                    }
                }
            }
            for field in ["children", "connector"] {
                if object.contains_key(field) {
                    let surface = field_surface(format!("{base}/{field}"));
                    assert!(
                        remove_surface_subtree(&mut self.unaccounted_surfaces, &surface),
                        "rewritten tanru anchor field was already accounted: {base}/{field}"
                    );
                }
            }
        }
    }

    /// Record waiver-family omissions for the provenance fields of one consumed
    /// object; the caller sweeps the remaining (projected) surfaces afterwards.
    #[requires(true)]
    #[ensures(true)]
    fn waive_consumed_provenance(&mut self, base: &str, object: &Map<String, Value>) {
        for (field_name, value) in object {
            let field_path = format!("{base}/{}", json_pointer_escape(field_name));
            match field_name.as_str() {
                "source" if is_source_record(value) => {
                    self.record_omission(XmlWaiverFamily::SourceRecord, field_surface(field_path));
                }
                "introducedBy" => {
                    self.record_omission(XmlWaiverFamily::IntroducedBy, field_surface(field_path));
                }
                "connector" => {
                    for connector_field in json_object(value).keys() {
                        if matches!(connector_field.as_str(), "source" | "locus") {
                            self.record_omission(
                                XmlWaiverFamily::ConnectorProvenance,
                                field_surface(format!(
                                    "{field_path}/{}",
                                    json_pointer_escape(connector_field)
                                )),
                            );
                        }
                    }
                }
                "tanruLink" => {
                    if json_object(value).contains_key("relationLabel") {
                        self.record_omission(
                            XmlWaiverFamily::CompositionRelationLabel,
                            field_surface(format!("{field_path}/relationLabel")),
                        );
                    }
                }
                "arguments" => {
                    for (place, argument) in json_object(value) {
                        if json_object(argument).contains_key("introducedBy") {
                            self.record_omission(
                                XmlWaiverFamily::IntroducedBy,
                                field_surface(format!(
                                    "{field_path}/{}/introducedBy",
                                    json_pointer_escape(place)
                                )),
                            );
                        }
                    }
                }
                "descriptor" => {
                    // Mirror the renderer's descriptor rule: only the exact
                    // elided-zo'e word is a mechanical omission; any other
                    // descriptor word stays in the descriptor-word family.
                    let descriptor = json_object(value);
                    let mechanical = optional_string(descriptor, "kind") == Some("elided")
                        && optional_string(descriptor, "word") == Some("zo'e");
                    if !mechanical && descriptor.contains_key("word") {
                        self.record_omission(
                            XmlWaiverFamily::DescriptorWord,
                            field_surface(format!("{field_path}/word")),
                        );
                    }
                }
                _ => {}
            }
        }
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

    /// #709 single-document dedup: with a WORDS word-card section present, a
    /// predication's `relationMetadata` subtree is rendered by the nonce
    /// word's WORD card, not by a body `RELATION-METADATA` element. Account
    /// the predication field, the referenced object, and every nested field
    /// as rendered — rendered-via-card is deliberately NOT a waiver, so no
    /// omission entries exist for any part of the subtree — and mark the
    /// object defined/emitted so the `emitted == object_keys` assertion holds
    /// and the `render_graph_components` unreachable-object sweep does not
    /// re-emit it.
    ///
    /// The object also never enters scoped DEFS declarations: declaration
    /// scopes are planned from pointer uses observed by the planning passes,
    /// and this path records none (a relationMetadata object is referenced
    /// only from its owning predication's `relationMetadata` field). A
    /// hypothetical multiply referenced relationMetadata would surface as
    /// `PrototypeIdWithoutCompactUse` during planning and select the
    /// TYPED-GRAPH form rather than silently dropping content.
    #[requires(optional_string(object, "relationMetadata") == Some(metadata_key))]
    #[requires(graph.objects.contains_key(metadata_key))]
    #[ensures(self.defined.contains(metadata_key) && self.emitted.contains(metadata_key))]
    fn account_relation_metadata_via_card(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
        metadata_key: &str,
    ) {
        self.account_field(graph, object, "relationMetadata");
        let metadata = graph.object(metadata_key);
        self.account_object(graph, metadata);
        for field in metadata.keys() {
            self.account_field_tree(graph, metadata, field);
        }
        self.defined.insert(metadata_key.to_owned());
        self.emitted.insert(metadata_key.to_owned());
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
            "tenseModal",
            "intervalModifiers",
            "spatialIntervalModifiers",
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
        if let Some(tense_modal) = optional_string(object, "tenseModal") {
            self.account_field(graph, object, "tenseModal");
            result.push(self.wrap_pointer(graph, "TENSE-MODAL", tense_modal, Vec::new()));
        }
        if let Some(modifiers) = object.get("intervalModifiers").and_then(Value::as_array) {
            self.account_field(graph, object, "intervalModifiers");
            let mut rendered = XmlElement::new("INTERVAL-MODIFIERS");
            for modifier in modifiers {
                rendered.push(self.render_interval_modifier(graph, json_object(modifier)));
            }
            result.push(rendered);
        }
        if let Some(modifiers) = object
            .get("spatialIntervalModifiers")
            .and_then(Value::as_array)
        {
            self.account_field(graph, object, "spatialIntervalModifiers");
            let mut rendered = XmlElement::new("SPATIAL-INTERVAL-MODIFIERS");
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
        let mut embedded_questions_accounted = false;
        let mut rendered_embedded_questions = HashSet::new();
        if let Some(body_key) = optional_string(object, "body") {
            self.account_field(graph, object, "body");
            let parameters = abstraction_body_parameters(graph, object, body_key);
            self.bound_variable_stack.extend(parameters.iter().cloned());
            let embedded_questions = embedded_questions_rendered_with_body(graph, object, body_key);
            if !embedded_questions.is_empty() {
                self.account_field(graph, object, "embeddedQuestions");
                embedded_questions_accounted = true;
                handled.push("embeddedQuestions");
                rendered_embedded_questions.extend(embedded_questions.iter().copied());
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
            let parameters = abstraction_content_parameters(graph, object, content);
            self.bound_variable_stack.extend(parameters.iter().cloned());
            let embedded_questions =
                embedded_questions_rendered_with_content(graph, object, content);
            if !embedded_questions.is_empty() {
                if !embedded_questions_accounted {
                    self.account_field(graph, object, "embeddedQuestions");
                    embedded_questions_accounted = true;
                    handled.push("embeddedQuestions");
                }
                rendered_embedded_questions.extend(embedded_questions.iter().copied());
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
        if let Some(questions) = object.get("embeddedQuestions").and_then(Value::as_array) {
            if !embedded_questions_accounted {
                self.account_field(graph, object, "embeddedQuestions");
                handled.push("embeddedQuestions");
            }
            let remaining = questions
                .iter()
                .map(|question| {
                    question
                        .as_str()
                        .unwrap_or_else(|| panic!("embedded question must be an id"))
                })
                .filter(|question| !rendered_embedded_questions.contains(question))
                .collect::<Vec<_>>();
            if !remaining.is_empty() {
                let mut rendered = XmlElement::new("EMBEDDED-QUESTIONS");
                for question in remaining {
                    rendered.push(self.render_pointer(graph, question));
                }
                result.push(rendered);
            }
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
            let Some(value) = value.as_object() else {
                let mut rendered = XmlElement::new("VALUE");
                rendered.push(self.generic_value(graph, value));
                result.push(rendered);
                result.extend(self.extras(graph, object, &["type", "form", "scale", "value"]));
                return result;
            };
            self.account_object(graph, value);
            let primary = ["integer", "text", "mathExpression"]
                .into_iter()
                .filter(|field| value.contains_key(*field))
                .collect::<Vec<_>>();
            assert_eq!(
                primary.len(),
                1,
                "quantity value must have one quantity representation"
            );
            let mut rendered = XmlElement::new("VALUE");
            if let Some(parameters) = value.get("questionParameters").and_then(Value::as_array) {
                self.account_field(graph, value, "questionParameters");
                rendered.set(
                    "QUESTION-PARAMETERS",
                    self.pointer_list(graph, parameters, "QUESTION-PARAMETERS"),
                );
            }
            match primary[0] {
                "integer" => {
                    self.account_field(graph, value, "integer");
                    rendered.push(XmlElement::with_attributes(
                        "INTEGER",
                        [("VALUE", scalar_string(&value["integer"]))],
                    ));
                }
                "mathExpression" => {
                    let expression = string_field(value, "mathExpression");
                    self.account_field(graph, value, "mathExpression");
                    rendered.push(self.render_pointer(graph, expression));
                }
                "text" => {
                    assert!(
                        object.contains_key("form"),
                        "quantity text cannot be provenance-only without FORM"
                    );
                    self.record_field_omission(graph, value, "text", XmlWaiverFamily::QuantityText);
                }
                _ => unreachable!("primary fields are closed"),
            }
            rendered.extend(self.extras(
                graph,
                value,
                &["integer", "text", "mathExpression", "questionParameters"],
            ));
            if primary[0] != "text"
                || !rendered.attributes.is_empty()
                || !rendered.children.is_empty()
            {
                result.push(rendered);
            }
        }
        result.extend(self.extras(graph, object, &["type", "form", "scale", "value"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "PLACE")]
    fn render_place_description(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        self.account_object(graph, value);
        self.account_field(graph, value, "place");
        self.account_field(graph, value, "description");
        let mut result = XmlElement::with_attributes(
            "PLACE",
            [
                ("INDEX", place_label(string_field(value, "place"))),
                ("DESCRIPTION", string_field(value, "description")),
            ],
        );
        result.extend(self.extras(graph, value, &["place", "description"]));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "RAFSI-BINDING")]
    fn render_rafsi_binding(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        self.account_object(graph, value);
        self.account_field(graph, value, "rafsi");
        let mut result =
            XmlElement::with_attributes("RAFSI-BINDING", [("RAFSI", string_field(value, "rafsi"))]);
        let mut handled = Vec::from(["rafsi"]);
        if let Some(source_word) = optional_string(value, "sourceWord") {
            self.account_field(graph, value, "sourceWord");
            result.set("SOURCE-WORD", source_word);
            handled.push("sourceWord");
        }
        if let Some(referent) = optional_string(value, "referent") {
            self.account_field(graph, value, "referent");
            result.push(self.wrap_pointer(graph, "REFERENT", referent, Vec::new()));
            handled.push("referent");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "EXPANSION")]
    fn render_relation_expansion(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        self.account_object(graph, value);
        self.account_field(graph, value, "kind");
        let mut result =
            XmlElement::with_attributes("EXPANSION", [("KIND", enum_token(&value["kind"]))]);
        let mut handled = Vec::from(["kind"]);
        if let Some(words) = value.get("sourceWords").and_then(Value::as_array) {
            self.account_field(graph, value, "sourceWords");
            let mut rendered_words = XmlElement::new("SOURCE-WORDS");
            for word in words {
                rendered_words.push(XmlElement::with_attributes(
                    "WORD",
                    [("VALUE", scalar_string(word))],
                ));
            }
            result.push(rendered_words);
            handled.push("sourceWords");
        }
        if let Some(bindings) = value.get("rafsiBindings").and_then(Value::as_array) {
            self.account_field(graph, value, "rafsiBindings");
            let mut rendered_bindings = XmlElement::new("RAFSI-BINDINGS");
            for binding in bindings {
                rendered_bindings.push(self.render_rafsi_binding(graph, json_object(binding)));
            }
            result.push(rendered_bindings);
            handled.push("rafsiBindings");
        }
        result.extend(self.extras(graph, value, &handled));
        result
    }

    #[requires(true)]
    #[ensures(ret.name == "RELATION-METADATA")]
    fn render_relation_metadata(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> XmlElement {
        self.account_field(graph, object, "relation");
        let mut result = XmlElement::with_attributes(
            "RELATION-METADATA",
            [("RELATION", string_field(object, "relation"))],
        );
        let mut handled = Vec::from(["type", "relation"]);
        if let Some(words) = object.get("sourceWords").and_then(Value::as_array) {
            self.account_field(graph, object, "sourceWords");
            let mut rendered_words = XmlElement::new("SOURCE-WORDS");
            for word in words {
                rendered_words.push(XmlElement::with_attributes(
                    "WORD",
                    [("VALUE", scalar_string(word))],
                ));
            }
            result.push(rendered_words);
            handled.push("sourceWords");
        }
        if let Some(places) = object.get("placeStructure").and_then(Value::as_array) {
            self.account_field(graph, object, "placeStructure");
            let mut rendered_places = XmlElement::new("PLACE-STRUCTURE");
            for place in places {
                rendered_places.push(self.render_place_description(graph, json_object(place)));
            }
            result.push(rendered_places);
            handled.push("placeStructure");
        }
        if let Some(expansion) = object.get("expansion").and_then(Value::as_object) {
            self.account_field(graph, object, "expansion");
            result.push(self.render_relation_expansion(graph, expansion));
            handled.push("expansion");
        }
        result.extend(self.extras(graph, object, &handled));
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
            let mut handled =
                Vec::from(["type", "items", "relation", "boundEventualities", "force"]);
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
            let mut semantic_entries = Vec::new();
            if let Some(content) = optional_string(object, "content") {
                state.account_field(graph, object, "content");
                semantic_entries.push(state.wrap_pointer(graph, "CONTENT", content, Vec::new()));
                handled.push("content");
            }
            if let Some(claims) = object.get("connectionClaims").and_then(Value::as_array) {
                state.account_field(graph, object, "connectionClaims");
                let mut rendered = XmlElement::new("CONNECTION-CLAIMS");
                for claim in claims {
                    rendered.push(
                        state.wrap_pointer(
                            graph,
                            "CONNECTION-CLAIM",
                            claim
                                .as_str()
                                .unwrap_or_else(|| panic!("connection claim must be an id")),
                            Vec::new(),
                        ),
                    );
                }
                semantic_entries.push(rendered);
                handled.push("connectionClaims");
            }
            if let Some(labels) = object.get("ordinalLabels").and_then(Value::as_array) {
                state.account_field(graph, object, "ordinalLabels");
                let mut rendered_labels = XmlElement::new("ORDINAL-LABELS");
                for label in labels {
                    let label = json_object(label);
                    state.account_object(graph, label);
                    state.account_field(graph, label, "level");
                    let mut rendered = XmlElement::with_attributes(
                        "ORDINAL-LABEL",
                        [("LEVEL", enum_token(&label["level"]))],
                    );
                    let mut label_handled = Vec::from(["level"]);
                    if let Some(target) = optional_string(label, "target") {
                        state.account_field(graph, label, "target");
                        rendered.push(state.wrap_pointer(graph, "TARGET", target, Vec::new()));
                        label_handled.push("target");
                    }
                    let value = string_field(label, "value");
                    state.account_field(graph, label, "value");
                    rendered.push(state.wrap_pointer(graph, "VALUE", value, Vec::new()));
                    label_handled.push("value");
                    if label.contains_key("introducedBy") {
                        state.record_field_omission(
                            graph,
                            label,
                            "introducedBy",
                            XmlWaiverFamily::IntroducedBy,
                        );
                        label_handled.push("introducedBy");
                    }
                    rendered.extend(state.extras(graph, label, &label_handled));
                    rendered_labels.push(rendered);
                }
                semantic_entries.push(rendered_labels);
                handled.push("ordinalLabels");
            }
            if let Some(connection) = object
                .get("nonlogicalConnection")
                .and_then(Value::as_object)
            {
                state.account_field(graph, object, "nonlogicalConnection");
                state.account_object(graph, connection);
                state.account_field(graph, connection, "operator");
                let mut rendered = XmlElement::with_attributes(
                    "NONLOGICAL-CONNECTION",
                    [("OPERATOR", string_field(connection, "operator"))],
                );
                let connector = connection
                    .get("connector")
                    .and_then(Value::as_object)
                    .unwrap_or_else(|| panic!("nonlogical connection lacks connector"));
                state.account_field(graph, connection, "connector");
                let operator = string_field(connection, "operator");
                let (attributes, children) = state.account_connector(graph, connector, operator);
                for (name, value) in attributes {
                    rendered.set(name, value);
                }
                rendered.extend(children);
                rendered.extend(state.extras(graph, connection, &["operator", "connector"]));
                semantic_entries.push(rendered);
                handled.push("nonlogicalConnection");
            }
            if let Some(operand) = object.get("elidedConnectionOperand") {
                state.account_field(graph, object, "elidedConnectionOperand");
                semantic_entries.push(Self::scalar("ELIDED-CONNECTION-OPERAND", operand));
                handled.push("elidedConnectionOperand");
            }
            semantic_entries.extend(state.extras(graph, object, &handled));
            (items, semantic_entries)
        });
        let (items, semantic_entries) = parts;
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
        if let Some(force) = object.get("force") {
            self.account_field(graph, object, "force");
            result.set("FORCE", enum_token(force));
        }
        Self::append_defs(&mut result, declarations);
        result.extend(items);
        result.extend(semantic_entries);
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
        let mut handled = Vec::from(["type"]);
        let mut result = if let Some(literal) = object.get("literal") {
            self.account_field(graph, object, "literal");
            handled.push("literal");
            let literal = literal
                .as_object()
                .unwrap_or_else(|| panic!("math literal must be a record"));
            self.account_object(graph, literal);
            let kind = string_field(literal, "kind");
            self.account_field(graph, literal, "kind");
            self.account_field(graph, literal, "value");
            let mut rendered = match kind {
                "integer" => XmlElement::with_attributes(
                    "INTEGER",
                    [("VALUE", scalar_string(&literal["value"]))],
                ),
                "mixedRadix" => {
                    let mixed = literal["value"]
                        .as_object()
                        .unwrap_or_else(|| panic!("mixed-radix literal must be a record"));
                    self.account_object(graph, mixed);
                    self.account_field(graph, mixed, "components");
                    let components = mixed["components"]
                        .as_array()
                        .unwrap_or_else(|| panic!("mixed-radix components must be a list"));
                    let mut mixed_radix = XmlElement::new("MIXED-RADIX");
                    for component in components {
                        let component = json_object(component);
                        self.account_object(graph, component);
                        self.account_field(graph, component, "text");
                        let mut rendered_component = XmlElement::with_attributes(
                            "COMPONENT",
                            [("TEXT", string_field(component, "text"))],
                        );
                        let mut component_handled = Vec::from(["text"]);
                        if let Some(integer) = component.get("integer") {
                            self.account_field(graph, component, "integer");
                            rendered_component.set("INTEGER", scalar_string(integer));
                            component_handled.push("integer");
                        }
                        rendered_component.extend(self.extras(
                            graph,
                            component,
                            &component_handled,
                        ));
                        mixed_radix.push(rendered_component);
                    }
                    mixed_radix.extend(self.extras(graph, mixed, &["components"]));
                    mixed_radix
                }
                _ => XmlElement::with_attributes(
                    "MATH-LITERAL",
                    [
                        ("KIND", enum_string(kind)),
                        ("VALUE", scalar_string(&literal["value"])),
                    ],
                ),
            };
            rendered.extend(self.extras(graph, literal, &["kind", "value"]));
            rendered
        } else {
            let mut rendered = XmlElement::new("MATH");
            if let Some(operator) = object.get("operator") {
                self.account_field(graph, object, "operator");
                rendered.set("OPERATOR", enum_token(operator));
                handled.push("operator");
            } else if let Some(parameter) = optional_string(object, "operatorParameter") {
                self.account_field(graph, object, "operatorParameter");
                rendered.push(self.wrap_pointer(
                    graph,
                    "OPERATOR-PARAMETER",
                    parameter,
                    Vec::new(),
                ));
                handled.push("operatorParameter");
            }
            rendered
        };
        if let Some(operands) = object.get("operands").and_then(Value::as_array) {
            self.account_field(graph, object, "operands");
            handled.push("operands");
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
        for (field, tag) in [
            ("denotes", "DENOTES"),
            ("operatorDenotes", "OPERATOR-DENOTES"),
        ] {
            if let Some(pointer) = optional_string(object, field) {
                self.account_field(graph, object, field);
                result.push(self.wrap_pointer(graph, tag, pointer, Vec::new()));
                handled.push(field);
            }
        }
        if let Some(inclusion) = object.get("endpointInclusion") {
            self.account_field(graph, object, "endpointInclusion");
            result.push(Self::scalar("ENDPOINT-INCLUSION", inclusion));
            handled.push("endpointInclusion");
        }
        if let Some(negation) = object.get("scalarNegation").and_then(Value::as_object) {
            self.account_field(graph, object, "scalarNegation");
            result.push(self.render_scalar_negation(graph, negation));
            handled.push("scalarNegation");
        }
        if let Some(subscript) = object.get("subscript").and_then(Value::as_object) {
            self.account_field(graph, object, "subscript");
            self.account_object(graph, subscript);
            let value = string_field(subscript, "value");
            self.account_field(graph, subscript, "value");
            let mut rendered = self.wrap_pointer(graph, "SUBSCRIPT", value, Vec::new());
            if subscript.contains_key("introducedBy") {
                self.record_field_omission(
                    graph,
                    subscript,
                    "introducedBy",
                    XmlWaiverFamily::IntroducedBy,
                );
            }
            rendered.extend(self.extras(graph, subscript, &["value", "introducedBy"]));
            result.push(rendered);
            handled.push("subscript");
        }
        result.extend(self.extras(graph, object, &handled));
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
            if key == "diagnostics" {
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
        composition_link: bool,
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
                        composition_link,
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
                composition_link,
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
        composition_link: bool,
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
            if composition_link && field == "relationLabel" {
                self.record_field_omission(
                    graph,
                    object,
                    field,
                    XmlWaiverFamily::CompositionRelationLabel,
                );
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
            let child_composition_link = is_composition_link_field(object, field, value);
            let mut rendered = XmlElement::with_attributes("FIELD", [("NAME", field.as_str())]);
            rendered.push(self.render_typed_graph_value(
                graph,
                value,
                child_descriptor_variable,
                child_quantity_value,
                child_composition_link,
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
        let record = self.render_typed_graph_record(graph, object, None, false, false, true);
        result.extend(record.children);
        result
    }

    /// Render the TYPED-GRAPH fallback document. This is a raw typed
    /// projection of the semantic graph for graphs the compact vocabulary
    /// cannot represent truthfully; it deliberately never carries a WORDS
    /// word-card section (#709), which is defined over the compact document
    /// shape (KEY → WORDS → WAIVERS → DEFS → body).
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
        let comment = [
            "SFN KEY (notation version 0): teaching text for this document. Defaults stated here are commitments, not omissions.",
            "FORM=TYPED-GRAPH is selected exactly when the semantic graph cannot be represented truthfully by the compact SFN prototype vocabulary. It is a typed XML projection of the semantic graph, not a reinterpretation.",
            "Each OBJECT is defined once by its canonical graph KEY= and XML ID=. ROOT-REF=/ROOT-KEY= identify the graph root. REFERENCE points to the exact shared object and never clones it.",
            "Every non-waived semantic object and field occurrence is represented as typed OBJECT, FIELD, RECORD, LIST, ITEM, REFERENCE, STRING, NUMBER, BOOLEAN, or NULL structure. Child order follows canonical semantic JSON order.",
            "A descriptor word is mechanically omitted only when descriptor KIND is elided and the value is exactly zo'e; every other descriptor word omission is reported by the existing descriptor-word waiver family.",
        ]
        .join("\n\n");
        let mut reasons = XmlElement::new("COMPACT-INCOMPATIBILITIES");
        for reason in incompatibilities {
            reasons.push(render_compact_incompatibility(reason));
        }
        root.push(reasons);
        root.push(typed_graph);
        serialize(&root, Some(&comment))
    }

    #[requires(true)]
    #[ensures(ret.ends_with('\n'))]
    fn render_document(
        &mut self,
        graph: &GraphData,
        document_name: &str,
        word_cards: Option<&[WordCard]>,
    ) -> String {
        // A WORDS section exists exactly for a present, non-empty card list;
        // the word-card KEY rules travel with it so documents without cards
        // stay byte-identical.
        let word_cards = word_cards.filter(|cards| !cards.is_empty());
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
        let sorts = format!(
            "SORT= values are flat PascalCase sort names. Subtype facts derived from encountered JSON sort paths: {facts}. LOCUTION implies sort Locution and therefore omits SORT=."
        );
        let mut paragraphs: Vec<&str> = vec![
            "SFN KEY (notation version 0): teaching text for this document. Defaults stated here are commitments, not omissions.",
        ];
        paragraphs.extend(KEY_RULES_BEFORE_SORTS);
        paragraphs.push(&sorts);
        paragraphs.extend(KEY_RULES_AFTER_SORTS);
        if word_cards.is_some() {
            paragraphs.extend(KEY_RULES_WORD_CARDS);
        }
        let comment = paragraphs.join("\n\n");
        if let Some(cards) = word_cards {
            root.push(words_section(cards));
        }
        Self::append_defs(&mut root, document_declarations);
        root.push(graph_root);
        if let Some(rendered) = unreachable {
            root.push(rendered);
        }
        serialize(&root, Some(&comment))
    }
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value_with_state(
    graph: Value,
    document_name: &str,
    state: RenderState,
    word_cards: Option<&[WordCard]>,
) -> XmlRender {
    let graph = GraphData::from_value(graph);
    render_indexed_graph_with_state(graph, document_name, state, word_cards)
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_indexed_graph_with_state(
    graph: GraphData,
    document_name: &str,
    mut state: RenderState,
    word_cards: Option<&[WordCard]>,
) -> XmlRender {
    // A WORDS section exists exactly for a present, non-empty card list (the
    // same filter `render_document` applies); every planning and render pass
    // must see the same card-presence decision (#709 relationMetadata dedup).
    state.word_cards_present = word_cards.is_some_and(|cards| !cards.is_empty());
    let preliminary_incompatibilities = match graph.representation.as_data() {
        data!(XmlRepresentationPlan::Compact) => BTreeSet::new(),
        data!(XmlRepresentationPlan::TypedGraph { incompatibilities }) => incompatibilities.clone(),
    };
    let output = if !preliminary_incompatibilities.is_empty() {
        state.start_omission_accounting(&graph);
        // The TYPED-GRAPH fallback is a raw-graph surface without a WORDS
        // section; word cards are silently inapplicable there.
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
            state.render_document(&graph, document_name, word_cards)
        } else {
            state.render_typed_graph_document(&graph, document_name, &planning_incompatibilities)
        }
    };
    let omissions = state.omissions;
    new!(XmlRender { output, omissions })
}

#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.objects.contains_key(&ret.root))]
fn graph_data_from_semantic_graph(graph: &SemanticGraph) -> GraphData {
    let binder_universes: HashMap<String, BTreeSet<String>> =
        semantic_scope_dependence_binder_universes(graph.root, &graph.objects)
            .into_iter()
            .map(|(referent, binders)| {
                (
                    referent.to_string(),
                    binders
                        .into_iter()
                        .map(|binder| binder.to_string())
                        .collect(),
                )
            })
            .collect();
    let mut value =
        serde_json::to_value(graph).expect("SemanticGraph's canonical serialization cannot fail");
    value["scopeDependenceBinderUniverses"] =
        serde_json::to_value(binder_universes).expect("binder universes serialize as an object");
    GraphData::from_value(value)
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_graph_with_state(
    graph: &SemanticGraph,
    document_name: &str,
    state: RenderState,
    word_cards: Option<&[WordCard]>,
) -> XmlRender {
    let indexed = graph_data_from_semantic_graph(graph);
    render_indexed_graph_with_state(indexed, document_name, state, word_cards)
}

/// Compute the compact-representation incompatibility records that the SFN-XML
/// renderer declares for `graph`, without serializing a document (jbotci#723).
///
/// The analysis is exactly the renderer's own: the graph-level representation
/// plan, and, when that plan is compact, the declaration-scope planning pass.
/// `word_cards_present` must be the card-presence decision of the render being
/// analyzed (a present, non-empty word-card list), because the planning pass
/// sees the same card context. The returned records are precisely the set the
/// corresponding render declares in its `COMPACT-INCOMPATIBILITIES` section —
/// empty when the document renders compact.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
pub fn analyze_compact_incompatibilities(
    graph: &SemanticGraph,
    word_cards_present: bool,
) -> Vec<CompactIncompatibility> {
    let indexed = graph_data_from_semantic_graph(graph);
    match indexed.representation.as_data() {
        data!(XmlRepresentationPlan::TypedGraph { incompatibilities }) => {
            incompatibilities.iter().cloned().collect()
        }
        data!(XmlRepresentationPlan::Compact) => {
            let mut state = RenderState::new();
            state.word_cards_present = word_cards_present;
            let mut incompatibilities = state.plan_declaration_scopes(&indexed);
            if incompatibilities.is_empty() {
                state.plan_ground_scopes(&indexed);
                incompatibilities = state.compact_planning_incompatibilities(&indexed);
            }
            incompatibilities.into_iter().collect()
        }
    }
}

#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value(graph: Value, document_name: &str) -> XmlRender {
    render_xml_value_with_state(graph, document_name, RenderState::new(), None)
}

#[cfg(test)]
#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
fn render_xml_value_with_binder_universes(
    mut graph: Value,
    document_name: &str,
    binder_universes: HashMap<String, BTreeSet<String>>,
) -> XmlRender {
    let injected = graph
        .as_object_mut()
        .expect("test graph must be an object")
        .entry("scopeDependenceBinderUniverses")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("test binder universes must be an object");
    for (key, universe) in binder_universes {
        injected.insert(
            key,
            Value::Array(universe.into_iter().map(Value::String).collect()),
        );
    }
    render_xml_value(graph, document_name)
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
    render_xml_value_with_state(graph, document_name, state, None)
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
    render_xml_graph_with_state(graph, document_name, RenderState::new(), None)
}

/// Tooling seam for corpus regeneration after an intentional output-shape
/// change (jbotci#719): render an already-serialized canonical graph `Value`
/// exactly as the in-crate corpus tests do (the corpus pins the canonical
/// JSON directly, so no `SemanticGraph` roundtrip exists for it). Not
/// product API.
#[doc(hidden)]
#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
pub fn render_xml_value_for_tooling(graph: Value, document_name: &str) -> XmlRender {
    render_xml_value(graph, document_name)
}

/// Render a semantic graph as canonical SFN-XML with a structured `<WORDS>`
/// word-card section (#709): the KEY gains the word-card rules and the WORDS
/// section follows it (before WAIVERS), ahead of the body. With cards present,
/// predication `relationMetadata` subtrees dedupe into the nonce word's WORD
/// card — body predications render no `RELATION-METADATA` and the subtree is
/// accounted rendered-via-card (no omission entries). An empty card list
/// renders exactly like [`render_xml`].
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!document_name.is_empty())]
#[ensures(ret.output.ends_with('\n'))]
pub fn render_xml_with_word_cards(
    graph: &SemanticGraph,
    document_name: &str,
    word_cards: &[WordCard],
) -> XmlRender {
    render_xml_graph_with_state(graph, document_name, RenderState::new(), Some(word_cards))
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
        let mut graph: Value = serde_json::from_slice(
            &std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
        let binder_universes: Value = serde_json::from_slice(include_bytes!(
            "../../tests/xml_corpus/BINDER_UNIVERSES.json"
        ))
        .expect("parse frozen binder-universe projections");
        graph["scopeDependenceBinderUniverses"] = binder_universes[document].clone();
        graph
    }

    #[requires(!document.is_empty())]
    #[ensures(ret.is_object())]
    fn phaseb_graph(document: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/phaseb_corpus")
            .join(format!("{document}.frozen.json"));
        let mut graph: Value = serde_json::from_slice(
            &std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
        let binder_universes: Value = serde_json::from_slice(include_bytes!(
            "../../tests/xml_corpus/BINDER_UNIVERSES.json"
        ))
        .expect("parse frozen binder-universe projections");
        graph["scopeDependenceBinderUniverses"] =
            binder_universes[format!("phaseb/{document}")].clone();
        graph
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

    const COMPACT_GENERIC_FALLBACK_MARKERS: &[&str] = &[
        "<EXTRA>", "<FIELD ", "<LIST>", "<ITEM>", "<RECORD>", "<UNKNOWN",
    ];

    #[requires(output.ends_with('\n'))]
    #[requires(!output.contains("FORM=\"TYPED-GRAPH\""))]
    #[ensures(true)]
    fn assert_no_compact_generic_fallback(output: &str, document: &str) {
        for marker in COMPACT_GENERIC_FALLBACK_MARKERS {
            assert!(
                !output.contains(marker),
                "{document}: known compact semantics reached generic {marker} scaffolding"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compact_incompatibility_declaration_is_the_exact_document_line() {
        // jbotci#723: the declaration line is the lossless record form quoted
        // into tooling, byte-identical to the document's own declaration.
        let record = new!(CompactIncompatibility::ScopeDependencyWithoutEnclosingBinder {
            referent: "entity:15".to_owned(),
            dependency: "entity:14".to_owned(),
        });
        assert_eq!(record.kind(), "SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER");
        assert_eq!(
            record.declaration(),
            "<INCOMPATIBILITY KIND=\"SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER\" REFERENT=\"entity:15\" DEPENDENCY=\"entity:14\"/>"
        );
        let planning = new!(CompactIncompatibility::DeclarationPlanningDidNotConverge {
            iterations: 4,
        });
        assert_eq!(
            planning.declaration(),
            "<INCOMPATIBILITY KIND=\"DECLARATION-PLANNING-DID-NOT-CONVERGE\" ITERATIONS=\"4\"/>"
        );
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn known_exceptional_quantity_and_relation_metadata_are_typed() {
        let mut graph = graph("b13");
        graph["objects"]["predication:18"]["relationMetadata"] =
            Value::String("relationMetadata:99".to_owned());
        graph["objects"]["relationMetadata:99"] = serde_json::json!({
            "type": "relationMetadata",
            "relation": "test-lujvo",
            "sourceWords": ["test", "lujvo"],
            "placeStructure": [{"place": "x1", "description": "the test participant"}],
            "expansion": {
                "kind": "lujvo",
                "sourceWords": ["tes", "luj"],
                "rafsiBindings": [{
                    "rafsi": "tes",
                    "sourceWord": "test",
                    "referent": "entity:17"
                }]
            }
        });
        graph["objects"]["math:97"] = serde_json::json!({
            "type": "mathExpression",
            "literal": {"kind": "integer", "value": 2}
        });
        graph["objects"]["quantity:98"] = serde_json::json!({
            "type": "quantity",
            "form": "exact",
            "scale": "count",
            "value": {
                "mathExpression": "math:97",
                "questionParameters": ["parameter:15"]
            }
        });
        graph["objects"]["predication:18"]["arguments"]["x2"]["value"] =
            Value::String("quantity:98".to_owned());
        graph["objects"]
            .as_object_mut()
            .expect("b13 objects")
            .remove("entity:16");
        graph["scopeDependenceBinderUniverses"]
            .as_object_mut()
            .expect("b13 binder universes")
            .remove("entity:16");

        let rendered = render_xml_value(graph, "<known-exceptional-fields>");
        assert!(rendered.output.contains("<RELATION-METADATA "));
        assert!(rendered.output.contains("RELATION=\"test-lujvo\""));
        assert!(rendered.output.contains("<WORD VALUE=\"test\"/>"));
        assert!(
            rendered
                .output
                .contains("<PLACE INDEX=\"1\" DESCRIPTION=\"the test participant\"/>")
        );
        assert!(rendered.output.contains("<EXPANSION KIND=\"LUJVO\">"));
        assert!(
            rendered
                .output
                .contains("<RAFSI-BINDING RAFSI=\"tes\" SOURCE-WORD=\"test\">")
        );
        assert!(rendered.output.contains("<REFERENT REF=\"r17\"/>"));
        assert!(
            rendered
                .output
                .contains("<VALUE QUESTION-PARAMETERS=\"v15\">")
        );
        assert_no_compact_generic_fallback(&rendered.output, "<known-exceptional-fields>");
    }

    /// #709 dedup: with a WORDS word-card section present, a nonce-lujvo
    /// predication's `relationMetadata` subtree renders via the lujvo's WORD
    /// card — no `RELATION-METADATA` element anywhere, no body mention, and no
    /// omission entries for any part of the subtree. Without cards the
    /// interim body form is preserved (pinned by
    /// `known_exceptional_quantity_and_relation_metadata_are_typed` and the
    /// frozen corpus). The generic `<UNKNOWN>` fallback never fires either way.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn relation_metadata_dedupes_into_word_cards_when_present() {
        use jbotci_morphology::segment_words_with_modifiers;

        use crate::notation::word_cards::build_xml_word_cards;

        let mut graph = graph("b13");
        graph["objects"]["predication:18"]["relationMetadata"] =
            Value::String("relationMetadata:99".to_owned());
        graph["objects"]["relationMetadata:99"] = serde_json::json!({
            "type": "relationMetadata",
            "relation": "skamymlatu",
            "sourceWords": ["skami", "mlatu"],
            "placeStructure": [{"place": "x1", "description": "the computer-feline participant"}],
            "expansion": {
                "kind": "lujvo",
                "sourceWords": ["skam", "mlatu"],
                "rafsiBindings": [{
                    "rafsi": "skam",
                    "sourceWord": "skami",
                    "referent": "entity:17"
                }]
            }
        });

        let words = segment_words_with_modifiers("skamymlatu").expect("skamymlatu segments");
        let cards = build_xml_word_cards(jbotci_dictionary_data::english(), &words);
        let with_cards = render_xml_value_with_state(
            graph.clone(),
            "b13",
            RenderState::new(),
            Some(&cards),
        );
        let without_cards = render_xml_value(graph, "b13");

        // Cards present: RELATION-METADATA never fires, not even in DEFS.
        assert!(
            !with_cards.output.contains("RELATION-METADATA"),
            "cards-present document must not emit RELATION-METADATA"
        );
        let with_body = with_cards
            .output
            .split_once("</WORDS>")
            .expect("WORDS section")
            .1;
        assert!(
            !with_body.contains("relationMetadata"),
            "cards-present body must not mention relationMetadata:\n{with_body}"
        );
        assert!(
            with_cards
                .omissions
                .iter()
                .all(|omission| !omission.surface.path().contains("relationMetadata")),
            "rendered-via-card accounting leaves no relationMetadata omissions: {:?}",
            with_cards.omissions
        );
        // The lujvo's WORD card carries the composition instead.
        assert!(
            with_cards.output.contains("<WORD ID=\"skamymlatu\""),
            "missing skamymlatu WORD card"
        );
        assert!(
            with_cards.output.contains("<COMPOSITE-APPROX"),
            "skamymlatu WORD card must carry the composition"
        );
        assert_no_compact_generic_fallback(&with_cards.output, "<relation-metadata-via-card>");

        // Cards absent: the interim body form and the accounting are unchanged.
        assert!(without_cards.output.contains("<RELATION-METADATA "));
        assert!(without_cards.output.contains("RELATION=\"skamymlatu\""));
        assert!(
            without_cards
                .omissions
                .iter()
                .all(|omission| !omission.surface.path().contains("relationMetadata")),
            "the typed interim renderer accounts every relationMetadata surface: {:?}",
            without_cards.omissions
        );
        assert_no_compact_generic_fallback(&without_cards.output, "<relation-metadata-interim>");
    }

    #[requires(true)]
    #[ensures(output.len() >= old(output.len()))]
    fn collect_declared_waiver_occurrences(
        value: &Value,
        path: &str,
        descriptor_variable: Option<bool>,
        composition_link: bool,
        output: &mut BTreeSet<XmlOmission>,
    ) {
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    collect_declared_waiver_occurrences(
                        item,
                        &format!("{path}/{index}"),
                        None,
                        false,
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
                    if field == "connector"
                        && let Some(connector) = item.as_object()
                    {
                        for connector_field in ["source", "locus"] {
                            if connector.contains_key(connector_field) {
                                output.insert(new!(XmlOmission {
                                    waiver: Some(XmlWaiverFamily::ConnectorProvenance),
                                    surface: field_surface(format!(
                                        "{field_path}/{connector_field}"
                                    )),
                                }));
                            }
                        }
                    }
                    if composition_link && field == "relationLabel" {
                        output.insert(new!(XmlOmission {
                            waiver: Some(XmlWaiverFamily::CompositionRelationLabel),
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
                        is_composition_link_field(object, field, item),
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
        collect_declared_waiver_occurrences(graph, "", None, false, &mut occurrences);
        occurrences
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn words_section_follows_key_and_carries_its_rules() {
        use jbotci_morphology::segment_words_with_modifiers;

        use crate::notation::word_cards::build_xml_word_cards;

        let words = segment_words_with_modifiers("barda").expect("barda segments");
        let cards = build_xml_word_cards(jbotci_dictionary_data::english(), &words);
        let with_cards =
            render_xml_value_with_state(graph("b13"), "b13", RenderState::new(), Some(&cards));
        let without_cards = render_xml_value(graph("b13"), "b13");
        let with_cards = with_cards.output.as_str();
        let without_cards = without_cards.output.as_str();

        // #719: the KEY is a single comment before the root element; WORDS is
        // the first child of the root when cards are present.
        assert!(
            with_cards.starts_with("<!--\n"),
            "document must open with the KEY comment"
        );
        assert!(
            with_cards.contains("\n<SFN VERSION=\"0\" DOC=\"b13\">\n  <WORDS>\n"),
            "WORDS must be the first child of the root"
        );
        assert!(!with_cards.contains("<KEY>") && !with_cards.contains("<WAIVERS>"));
        // The word-card KEY paragraphs exist exactly when the section does.
        for paragraph_marker in [
            "WORDS lists one WORD card per content word",
            "COMPOSITE-APPROX shows the mechanical composition",
            "PLACES=\"UNKNOWN\" means the composition tree",
            "cards state ASSUMED-LEFT",
            "VARIABLE-CONTEXT denotes the abstract role",
            "WORD ID values are surface-spelling card keys",
        ] {
            assert!(
                with_cards.contains(paragraph_marker),
                "missing KEY comment paragraph: {paragraph_marker}"
            );
            assert!(
                !without_cards.contains(paragraph_marker),
                "card-less document must not carry the paragraph: {paragraph_marker}"
            );
        }
        assert!(!without_cards.contains("<WORDS>"));
        // The body after the section is byte-identical to the card-less body.
        let with_body = with_cards
            .split_once("</WORDS>")
            .expect("WORDS section")
            .1;
        let without_body = without_cards
            .split_once("\n  <DEFS>")
            .expect("DEFS after comment");
        assert_eq!(with_body, format!("\n  <DEFS>{}", without_body.1));
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
                .filter(|name| !matches!(name.as_str(), "PROVENANCE.md" | "BINDER_UNIVERSES.json"))
                .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            aggregate_hash("frozen.json"),
            "3f5171701f45523f70a97faad8a8b86c6a96c4c98a22dffccc05d7e210d12218"
        );
        assert_eq!(
            aggregate_hash("xml.txt"),
            "38e85d26ba63780c33a90ea99c9d427e3e37d6364a0cb4813f4d21615645199c"
        );
        let binder_universe_bytes = include_bytes!("../../tests/xml_corpus/BINDER_UNIVERSES.json");
        assert_eq!(
            format!("{:x}", Sha256::digest(binder_universe_bytes)),
            "d672abb2849175b03a18d0c45b854a5094a702ddb11adde6d7075ccd419b5776"
        );
        let binder_universes: Map<String, Value> = serde_json::from_slice(binder_universe_bytes)
            .expect("parse frozen binder-universe projection");
        let mut expected_projection_documents: BTreeSet<String> = XML_CORPUS_DOCS
            .iter()
            .map(|document| (*document).to_owned())
            .collect();
        let phaseb_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/phaseb_corpus");
        expected_projection_documents.extend(
            std::fs::read_dir(phaseb_dir)
                .expect("read Phase B corpus")
                .map(|entry| entry.expect("read Phase B entry").path())
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "json")
                })
                .map(|path| {
                    format!(
                        "phaseb/{}",
                        path.file_stem()
                            .expect("Phase B JSON has a stem")
                            .to_string_lossy()
                            .trim_end_matches(".frozen")
                    )
                }),
        );
        assert_eq!(
            binder_universes.keys().cloned().collect::<BTreeSet<_>>(),
            expected_projection_documents
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
            assert_no_compact_generic_fallback(&actual.output, document);
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
        let fixed = render_xml_value_with_binder_universes(
            fixed,
            "<parameter-fixed>",
            HashMap::from([(
                "entity:8".to_owned(),
                BTreeSet::from(["parameter:7".to_owned()]),
            )]),
        );
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
        subset["objects"]["entity:8"]["scopeDependence"] = serde_json::json!({
            "kind": "underspecified",
            "mayDependOn": ["parameter:7"]
        });
        let subset = render_xml_value_with_binder_universes(
            subset,
            "<parameter-subset>",
            HashMap::from([(
                "entity:8".to_owned(),
                BTreeSet::from(["parameter:7".to_owned(), "parameter:99".to_owned()]),
            )]),
        );
        assert!(subset.output.contains("POSSIBLY-DIFFERENT-PER=\"v7\""));
        assert!(!subset.output.contains("FORM=\"TYPED-GRAPH\""));

        let mut distinct_question_body = graph("b58");
        distinct_question_body["objects"]["formula:99"] = serde_json::json!({
            "type": "formula",
            "operator": "atom",
            "predication": "predication:99",
            "boundEventualities": ["eventuality:99"]
        });
        distinct_question_body["objects"]["predication:99"] =
            distinct_question_body["objects"]["predication:9"].clone();
        distinct_question_body["objects"]["predication:99"]["eventuality"] =
            Value::String("eventuality:99".to_owned());
        distinct_question_body["objects"]["predication:99"]["arguments"]["x2"]["value"] =
            Value::String("entity:98".to_owned());
        distinct_question_body["objects"]["eventuality:99"] =
            distinct_question_body["objects"]["eventuality:6"].clone();
        distinct_question_body["objects"]["entity:98"] =
            distinct_question_body["objects"]["entity:8"].clone();
        distinct_question_body["objects"]["entity:98"]["scopeDependence"] =
            serde_json::json!({"kind": "underspecified", "mayDependOn": ["parameter:7"]});
        distinct_question_body["objects"]["entity:8"]["scopeDependence"] =
            serde_json::json!({"kind": "fixed"});
        distinct_question_body["objects"]["question:11"]["body"] =
            Value::String("formula:99".to_owned());
        let distinct_question_body = render_xml_value_with_binder_universes(
            distinct_question_body,
            "<distinct-question-body>",
            HashMap::from([
                ("entity:8".to_owned(), BTreeSet::new()),
                (
                    "entity:98".to_owned(),
                    BTreeSet::from(["parameter:7".to_owned()]),
                ),
            ]),
        );
        assert!(
            distinct_question_body
                .output
                .contains("FORM=\"TYPED-GRAPH\"")
        );
        assert!(
            distinct_question_body
                .output
                .contains("KIND=\"BINDER-DOES-NOT-ENCLOSE-USE\"")
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
        content_abstraction["objects"]["entity:8"]["scopeDependence"] =
            serde_json::json!({"kind": "fixed"});
        content_abstraction["scopeDependenceBinderUniverses"]["entity:8"] = serde_json::json!([]);
        let content_abstraction = render_xml_value(content_abstraction, "<nu-content-witness>")
            .into_data()
            .output;
        assert!(!content_abstraction.contains("FORM=\"TYPED-GRAPH\""));
        assert!(content_abstraction.contains("<CONTENT>"));
        assert!(content_abstraction.contains("<EMBEDDED-QUESTIONS>"));
        assert_eq!(
            format!("{:x}", Sha256::digest(content_abstraction.as_bytes())),
            "6080365adb356f04598f6ca705d67da4b2f8ae4d338de20abdb1f034cf5109e3"
        );

        let mut direct_question = graph("b58");
        direct_question["objects"]["utterance:5"]["content"] =
            Value::String("question:11".to_owned());
        direct_question["objects"]
            .as_object_mut()
            .expect("b58 objects")
            .remove("proposition:12");
        direct_question["scopeDependenceBinderUniverses"]
            .as_object_mut()
            .expect("b58 binder universes")
            .remove("proposition:12");
        let direct_question = render_xml_value(direct_question, "<direct-question-witness>")
            .into_data()
            .output;
        assert!(!direct_question.contains("FORM=\"TYPED-GRAPH\""));
        assert!(direct_question.contains("<QUESTION KIND=\"ARGUMENT\""));
        assert_no_compact_generic_fallback(&direct_question, "<direct-question-witness>");
        assert_eq!(
            format!("{:x}", Sha256::digest(direct_question.as_bytes())),
            "fb17cdc0c93972c82b1e641e528545af881ff5f3f9995a01bafecfa34f40d99c"
        );

        let mut shared = graph("b58");
        shared["objects"]["proposition:12"]["sort"] = Value::String("eventuality".to_owned());
        shared["objects"]["proposition:12"]["content"] =
            shared["objects"]["proposition:12"]["body"].clone();
        shared["objects"]["question:11"]["slots"] = serde_json::json!([]);
        shared["objects"]["question:11"]
            .as_object_mut()
            .expect("b58 question")
            .remove("focus");
        shared["objects"]["predication:9"]["arguments"]["x1"]["value"] =
            Value::String("entity:8".to_owned());
        shared["objects"]["entity:8"]["scopeDependence"] = serde_json::json!({"kind": "fixed"});
        let shared = render_xml_value_with_binder_universes(
            shared,
            "<shared-body-content>",
            HashMap::from([("entity:8".to_owned(), BTreeSet::new())]),
        )
        .into_data()
        .output;
        assert!(!shared.contains("FORM=\"TYPED-GRAPH\""));
        assert_eq!(shared.matches("<EMBEDDED-QUESTIONS>").count(), 1);
        assert_eq!(
            format!("{:x}", Sha256::digest(shared.as_bytes())),
            "5aeb6ad72a98c5cb2c6558b1a71e90a5bdc7306321e9886d9e81bc5bfd53a8e7"
        );
        let content = shared
            .split_once("<CONTENT>")
            .expect("shared content start")
            .1
            .split_once("</CONTENT>")
            .expect("shared content end")
            .0;
        let body = shared
            .split_once("<BODY>")
            .expect("shared body start")
            .1
            .split_once("</BODY>")
            .expect("shared body end")
            .0;
        assert!(content.contains("<EMBEDDED-QUESTIONS>"));
        assert!(!body.contains("<EMBEDDED-QUESTIONS>"));

        let focused_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/xml_focused_regressions/content-first-question-scope/b60.frozen.json");
        let mut shared_with_slots: Value = serde_json::from_slice(
            &std::fs::read(&focused_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", focused_path.display())),
        )
        .expect("parse focused b60 graph");
        shared_with_slots["scopeDependenceBinderUniverses"] = serde_json::json!({
            "entity:10": [],
            "entity:11": [],
            "entity:12": [],
            "entity:16": [],
            "entity:9": [],
            "eventuality/locution:19": [],
            "eventuality:7": []
        });
        shared_with_slots["objects"]["eventuality:7"]["body"] =
            shared_with_slots["objects"]["eventuality:7"]["content"].clone();
        shared_with_slots["objects"]["entity:10"]["scopeDependence"] = serde_json::json!({
            "kind": "underspecified",
            "mayDependOn": ["parameter:9"]
        });
        let shared_with_slots = render_xml_value(shared_with_slots, "<shared-real-slots>")
            .into_data()
            .output;
        assert!(shared_with_slots.contains("FORM=\"TYPED-GRAPH\""));
        assert!(
            shared_with_slots.contains("KIND=\"SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER\""),
            "{shared_with_slots}"
        );
        let mut orphan = graph("b58");
        orphan["objects"]["formula:99"] = serde_json::json!({
            "type": "formula",
            "operator": "atom",
            "predication": "predication:99",
            "boundEventualities": ["eventuality:99"]
        });
        orphan["objects"]["predication:99"] = orphan["objects"]["predication:9"].clone();
        orphan["objects"]["predication:99"]["eventuality"] =
            Value::String("eventuality:99".to_owned());
        orphan["objects"]["eventuality:99"] = orphan["objects"]["eventuality:6"].clone();
        orphan["objects"]["question:11"]["body"] = Value::String("formula:99".to_owned());
        orphan["objects"]["predication:9"]["arguments"]["x1"]["value"] =
            Value::String("entity:8".to_owned());
        orphan["objects"]["entity:8"]["scopeDependence"] = serde_json::json!({"kind": "fixed"});
        let orphan = render_xml_value_with_binder_universes(
            orphan,
            "<orphan>",
            HashMap::from([("entity:8".to_owned(), BTreeSet::new())]),
        )
        .into_data()
        .output;
        assert!(orphan.contains("<EMBEDDED-QUESTIONS>"));
        assert!(orphan.contains("<QUESTION KIND=\"ARGUMENT\""));
        assert_no_compact_generic_fallback(&orphan, "<orphan>");
        assert_eq!(
            format!("{:x}", Sha256::digest(orphan.as_bytes())),
            "ab0ba51a6930df1f4a7bf548471a241fdc9df4d36d29e4376d3690fd10f408fd"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn kind_composition_is_first_class_nested_and_provenance_free() {
        let rendered = render_xml_value(graph("b39"), "b39");
        assert!(!rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        // jbotci#719: the tanru projects as one predication with a nested
        // KIND-COMPOSITION relation expression in the relation slot.
        assert_eq!(rendered.output.matches("<KIND-COMPOSITION>").count(), 2);
        assert!(rendered.output.contains(
            "<KIND-COMPOSITION>\n                        <KIND PREDICATE=\"prenu\"/>\n                        <MODIFIER>"
        ));
        assert!(rendered.output.contains(
            "<KIND-COMPOSITION>\n                              <KIND PREDICATE=\"bajra\"/>\n                              <MODIFIER PREDICATE=\"sutra\"/>"
        ));
        assert!(!rendered.output.contains("PREDICATE=\"tanru\""));
        assert!(!rendered.output.contains("<CONNECTOR"));
        assert!(!rendered.output.contains("TANRU-LINK"));
        assert!(!rendered.output.contains("<RELATION-LABEL"));
        assert!(!rendered.output.contains("FIELD NAME=\"tanruLink\""));
        assert_eq!(
            rendered
                .omissions
                .iter()
                .filter(|omission| {
                    omission.waiver == Some(XmlWaiverFamily::CompositionRelationLabel)
                })
                .count(),
            2
        );
        assert_no_compact_generic_fallback(&rendered.output, "b39");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostics_are_repeatable_escaped_warning_text_nodes() {
        let mut warning_graph = graph("b57");
        for object in warning_graph["objects"]
            .as_object_mut()
            .expect("b57 objects")
            .values_mut()
        {
            if let Some(object) = object.as_object_mut() {
                object.remove("diagnostics");
            }
        }
        warning_graph["objects"]["predication:10"]["diagnostics"] = serde_json::json!([
            {"severity": "warning", "message": "one < two & three > zero"},
            {"severity": "warning", "message": "quoted \"warning\" remains text"}
        ]);

        let rendered = render_xml_value(warning_graph, "<warning-escaping>");
        assert_eq!(rendered.output.matches("<WARNING>").count(), 2);
        assert!(
            rendered
                .output
                .contains("<WARNING>one &lt; two &amp; three &gt; zero</WARNING>")
        );
        assert!(
            rendered
                .output
                .contains("<WARNING>quoted \"warning\" remains text</WARNING>")
        );
        for known_field in ["diagnostics", "severity", "message"] {
            assert!(
                !rendered
                    .output
                    .contains(&format!("<FIELD NAME=\"{known_field}\">"))
            );
        }
        assert_no_compact_generic_fallback(&rendered.output, "<warning-escaping>");
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
                XmlWaiverFamily::CompositionRelationLabel,
                XmlWaiverFamily::ConnectorProvenance,
            ]
        );
        let mut counts: BTreeMap<XmlWaiverFamily, usize> = BTreeMap::new();
        let mut documents: BTreeMap<XmlWaiverFamily, BTreeSet<&str>> = BTreeMap::new();
        for document in XML_CORPUS_DOCS {
            let graph = graph(document);
            let expected = declared_waiver_occurrences(&graph);
            let rendered = render_xml_value(graph, document);
            let actual: BTreeSet<XmlOmission> = rendered.omissions.iter().cloned().collect();
            if actual != expected {
                let missing: Vec<_> = expected.difference(&actual).collect();
                let extra: Vec<_> = actual.difference(&expected).collect();
                panic!(
                    "{document}: observed omissions differ from independently expanded waivers:\nmissing: {missing:#?}\nextra: {extra:#?}"
                );
            }
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
                (XmlWaiverFamily::CompositionRelationLabel, 5),
                (XmlWaiverFamily::ConnectorProvenance, 16),
            ])
        );
        assert_eq!(counts.values().sum::<usize>(), 957);
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
                (XmlWaiverFamily::CompositionRelationLabel, 3),
                (XmlWaiverFamily::ConnectorProvenance, 6),
            ])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn occurrence_ledger_renders_unknown_fields_and_exposes_silent_drops() {
        let mut unknown = graph("b13");
        unknown["objects"]["utterance:5"]["futureSemanticField"] = serde_json::json!({
            "nested": ["must-render"]
        });
        let rendered = render_xml_value(unknown, "ledger-unknown");
        assert!(
            rendered
                .output
                .contains("<FIELD NAME=\"futureSemanticField\">")
        );
        for preserved in [
            "<RECORD>",
            "<FIELD NAME=\"nested\">",
            "<LIST>",
            "<ITEM>",
            "<STRING VALUE=\"must-render\"/>",
        ] {
            assert!(rendered.output.contains(preserved), "missing {preserved}");
        }
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
    fn text_quantity_companions_are_accounted_and_unknown_children_survive() {
        let mut unknown = graph("b23");
        unknown["objects"]["quantity:20"]["value"]["novelNested"] = serde_json::json!({
            "inner": ["must-render"]
        });
        let expected = declared_waiver_occurrences(&unknown);
        assert!(expected.contains(&new!(XmlOmission {
            waiver: Some(XmlWaiverFamily::QuantityText),
            surface: field_surface("/objects/quantity:20/value/text".to_owned()),
        })));

        let rendered = render_xml_value(unknown, "<text-quantity-unknown-child>");
        assert!(!rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        for preserved in [
            "<VALUE>",
            "<EXTRA>",
            "<FIELD NAME=\"novelNested\">",
            "<FIELD NAME=\"inner\">",
            "<LIST>",
            "<ITEM>",
            "<STRING VALUE=\"must-render\"/>",
        ] {
            assert!(rendered.output.contains(preserved), "missing {preserved}");
        }
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected,
            "an emitted fallback child must not disappear from occurrence accounting"
        );

        let question_quantity = phaseb_graph("question-multiple-domains");
        let expected = declared_waiver_occurrences(&question_quantity);
        assert!(expected.contains(&new!(XmlOmission {
            waiver: Some(XmlWaiverFamily::QuantityText),
            surface: field_surface("/objects/quantity:15/value/text".to_owned()),
        })));
        let rendered = render_xml_value(question_quantity, "<text-quantity-question-parameter>");
        assert_eq!(
            rendered
                .into_data()
                .omissions
                .into_iter()
                .collect::<BTreeSet<_>>(),
            expected,
            "text plus questionParameters must use the same exact QuantityText waiver oracle"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_graph_waives_only_structural_composition_relation_labels() {
        let mut typed = graph("b39");
        let root = typed["root"].as_str().expect("b39 root").to_owned();
        typed["objects"][root.as_str()]["relationLabel"] =
            Value::String("unrelated-novel-label".to_owned());
        typed["objects"]["cycle:90"] = serde_json::json!({
            "type": "unknown",
            "next": "cycle:91"
        });
        typed["objects"]["cycle:91"] = serde_json::json!({
            "type": "unknown",
            "next": "cycle:90"
        });
        let expected = declared_waiver_occurrences(&typed);

        let rendered = render_xml_value(typed, "<typed-unrelated-relation-label>");
        assert!(rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert!(rendered.output.contains("<FIELD NAME=\"relationLabel\">"));
        assert!(
            rendered
                .output
                .contains("<STRING VALUE=\"unrelated-novel-label\"/>")
        );
        assert_eq!(
            rendered
                .omissions
                .iter()
                .filter(|omission| {
                    omission.waiver == Some(XmlWaiverFamily::CompositionRelationLabel)
                })
                .count(),
            2,
            "only b39's two structural composition links are provenance-only"
        );
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_phase_b_known_compact_field_avoids_generic_fallback() {
        let phaseb_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/phaseb_corpus");
        let mut documents: Vec<String> = std::fs::read_dir(&phaseb_dir)
            .expect("read Phase B corpus")
            .map(|entry| entry.expect("read Phase B entry").path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with(".frozen.json"))
            })
            .map(|path| {
                path.file_name()
                    .expect("Phase B JSON has a filename")
                    .to_string_lossy()
                    .trim_end_matches(".frozen.json")
                    .to_owned()
            })
            .collect();
        documents.sort();
        assert_eq!(documents.len(), 50, "Phase B inventory changed");

        let mut compact = 0usize;
        for document in documents {
            let rendered = render_xml_value(phaseb_graph(&document), &format!("phaseb/{document}"));
            if !rendered.output.contains("FORM=\"TYPED-GRAPH\"") {
                compact += 1;
                assert_no_compact_generic_fallback(&rendered.output, &document);
            }
        }
        assert_eq!(compact, 49, "Phase B compact population changed");
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
            if key == "diagnostics" {
                // Semantic-object diagnostics are rendered once, centrally,
                // after the object's typed semantic content.
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
        let active: Vec<String> = graph
            .scope_dependence_binder_universes
            .get(referent_key)
            .unwrap_or_else(|| {
                panic!("constant referent lacks a first-visit binder universe: {referent_key:?}")
            })
            .iter()
            .cloned()
            .collect();
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
        for (field, tag) in [
            ("intervalModifiers", "INTERVAL-MODIFIERS"),
            ("spatialIntervalModifiers", "SPATIAL-INTERVAL-MODIFIERS"),
        ] {
            if let Some(modifiers) = object.get(field).and_then(Value::as_array) {
                self.account_field(graph, object, field);
                let mut rendered = XmlElement::new(tag);
                for modifier in modifiers {
                    rendered.push(self.render_interval_modifier(graph, json_object(modifier)));
                }
                variable.push(rendered);
                handled.push(field);
            }
        }
        if let Some(tense_modal) = optional_string(object, "tenseModal") {
            self.account_field(graph, object, "tenseModal");
            variable.push(self.wrap_pointer(graph, "TENSE-MODAL", tense_modal, Vec::new()));
            handled.push("tenseModal");
        }
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
        if let Some(connection) = value.get("connection").and_then(Value::as_object) {
            self.account_field(graph, value, "connection");
            self.account_object(graph, connection);
            self.account_field(graph, connection, "kind");
            let mut rendered = XmlElement::with_attributes(
                "CONNECTION",
                [("KIND", enum_token(&connection["kind"]))],
            );
            if connection.contains_key("introducedBy") {
                self.record_field_omission(
                    graph,
                    connection,
                    "introducedBy",
                    XmlWaiverFamily::IntroducedBy,
                );
            }
            rendered.extend(self.extras(graph, connection, &["kind", "introducedBy"]));
            result.push(rendered);
            handled.push("connection");
        }
        if let Some(quantity_value) = value.get("value").and_then(Value::as_object) {
            self.account_field(graph, value, "value");
            self.account_object(graph, quantity_value);
            let primary = ["integer", "text", "mathExpression"]
                .into_iter()
                .filter(|field| quantity_value.contains_key(*field))
                .collect::<Vec<_>>();
            assert_eq!(
                primary.len(),
                1,
                "recurrence value must have one quantity representation"
            );
            let mut rendered = XmlElement::new("VALUE");
            match primary[0] {
                "integer" => {
                    self.account_field(graph, quantity_value, "integer");
                    rendered.push(XmlElement::with_attributes(
                        "INTEGER",
                        [("VALUE", scalar_string(&quantity_value["integer"]))],
                    ));
                }
                "mathExpression" => {
                    let expression = string_field(quantity_value, "mathExpression");
                    self.account_field(graph, quantity_value, "mathExpression");
                    rendered.push(self.wrap_pointer(
                        graph,
                        "MATH-EXPRESSION",
                        expression,
                        Vec::new(),
                    ));
                }
                "text" => {
                    self.record_field_omission(
                        graph,
                        quantity_value,
                        "text",
                        XmlWaiverFamily::QuantityText,
                    );
                }
                _ => unreachable!("primary fields are closed"),
            }
            if let Some(parameters) = quantity_value
                .get("questionParameters")
                .and_then(Value::as_array)
            {
                self.account_field(graph, quantity_value, "questionParameters");
                rendered.set(
                    "QUESTION-PARAMETERS",
                    self.pointer_list(graph, parameters, "QUESTION-PARAMETERS"),
                );
            }
            rendered.extend(self.extras(
                graph,
                quantity_value,
                &["integer", "text", "mathExpression", "questionParameters"],
            ));
            result.push(rendered);
            handled.push("value");
        }
        if let Some(interval) = optional_string(value, "interval") {
            self.account_field(graph, value, "interval");
            result.push(self.wrap_pointer(graph, "INTERVAL", interval, Vec::new()));
            handled.push("interval");
        }
        if let Some(negation) = value.get("negation").and_then(Value::as_object) {
            self.account_field(graph, value, "negation");
            self.account_object(graph, negation);
            self.account_field(graph, negation, "kind");
            let mut rendered =
                XmlElement::with_attributes("NEGATION", [("KIND", enum_token(&negation["kind"]))]);
            if negation.contains_key("introducedBy") {
                self.record_field_omission(
                    graph,
                    negation,
                    "introducedBy",
                    XmlWaiverFamily::IntroducedBy,
                );
            }
            rendered.extend(self.extras(graph, negation, &["kind", "introducedBy"]));
            result.push(rendered);
            handled.push("negation");
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
        let kind_value = optional_string(value, "kind").unwrap_or("interval");
        let kind = enum_string(kind_value);
        if value.contains_key("kind") {
            self.account_field(graph, value, "kind");
        }
        let mut result = XmlElement::with_attributes("INTERVAL-MODIFIER", [("KIND", kind)]);
        let nested_value = value
            .get("value")
            .unwrap_or_else(|| panic!("interval modifier lacks value"));
        let nested = nested_value
            .as_object()
            .unwrap_or_else(|| panic!("interval modifier value must be a record"));
        self.account_field(graph, value, "value");
        match kind_value {
            "aspect" => {
                self.account_object(graph, nested);
                self.account_field(graph, nested, "contour");
                let mut aspect = XmlElement::with_attributes(
                    "ASPECT",
                    [("CONTOUR", enum_token(&nested["contour"]))],
                );
                let mut handled = Vec::from(["contour"]);
                if let Some(anchor) = optional_string(nested, "anchor") {
                    self.account_field(graph, nested, "anchor");
                    aspect.push(self.wrap_pointer(graph, "ANCHOR", anchor, Vec::new()));
                    handled.push("anchor");
                }
                if let Some(negation) = nested.get("scalarNegation").and_then(Value::as_object) {
                    self.account_field(graph, nested, "scalarNegation");
                    aspect.push(self.render_scalar_negation(graph, negation));
                    handled.push("scalarNegation");
                }
                aspect.extend(self.extras(graph, nested, &handled));
                result.push(aspect);
            }
            "recurrence" => result.push(self.render_recurrence_item(graph, nested_value)),
            _ => panic!("unknown interval modifier kind: {kind_value:?}"),
        }
        result.extend(self.extras(graph, value, &["kind", "value"]));
        result
    }

    /// Account one connector record without rendering a CONNECTOR element
    /// (jbotci#719): the surface word and the grammatical locus are
    /// provenance-class and join the connector-provenance waiver family; a
    /// truth table the parent operator does not already determine renders as a
    /// TRUTH-TABLE= attribute; a connective question's bound parameter renders
    /// as a PARAMETER child. Returns the attributes and children to attach to
    /// the parent element where the CONNECTOR element used to sit.
    #[requires(true)]
    #[ensures(true)]
    fn account_connector(
        &mut self,
        graph: &GraphData,
        connector: &Map<String, Value>,
        operator: &str,
    ) -> (Vec<(String, String)>, Vec<XmlElement>) {
        self.account_object(graph, connector);
        for field in ["source", "locus"] {
            if connector.contains_key(field) {
                self.record_field_omission(
                    graph,
                    connector,
                    field,
                    XmlWaiverFamily::ConnectorProvenance,
                );
            }
        }
        let mut attributes = Vec::new();
        if let Some(truth_table) = optional_string(connector, "truthTable") {
            self.account_field(graph, connector, "truthTable");
            if canonical_truth_table(operator) != Some(truth_table) {
                attributes.push(("TRUTH-TABLE".to_owned(), truth_table.to_owned()));
            }
        }
        if let Some(parameter) = optional_string(connector, "parameter") {
            self.account_field(graph, connector, "parameter");
            // A connective question's bound parameter is an attribute use of
            // an already-defined parameter object (jbotci#719: the PARAMETER
            // element spelling would collide with the parameter object
            // element in the schema's content models).
            let id = self.pointer_id(graph, parameter, "connective question parameter");
            attributes.push(("PARAMETER".to_owned(), id));
        }
        let mut children = Vec::new();
        children.extend(self.extras(
            graph,
            connector,
            &["source", "locus", "truthTable", "parameter"],
        ));
        (attributes, children)
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
    #[ensures(ret.name == "PLACE-QUESTION")]
    fn render_place_question(
        &mut self,
        graph: &GraphData,
        value: &Map<String, Value>,
    ) -> XmlElement {
        self.account_object(graph, value);
        let candidates = value
            .get("candidatePlaces")
            .and_then(Value::as_array)
            .filter(|candidates| !candidates.is_empty())
            .unwrap_or_else(|| panic!("place question must have candidate place labels"));
        self.account_field(graph, value, "candidatePlaces");
        let candidate_places = candidates
            .iter()
            .map(|place| {
                place_label(
                    place
                        .as_str()
                        .unwrap_or_else(|| panic!("candidate place must be a string")),
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut result =
            XmlElement::with_attributes("PLACE-QUESTION", [("CANDIDATE-PLACES", candidate_places)]);
        let parameter = string_field(value, "parameter");
        self.account_field(graph, value, "parameter");
        result.push(self.wrap_pointer(graph, "PARAMETER", parameter, Vec::new()));
        let argument = value
            .get("argument")
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("place question argument must be a record"));
        self.account_field(graph, value, "argument");
        result.push(self.render_argument(graph, argument, None, false));
        result.extend(self.extras(graph, value, &["parameter", "argument", "candidatePlaces"]));
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
fn embedded_questions_rendered_with_content<'a>(
    graph: &'a GraphData,
    object: &'a Map<String, Value>,
    formula: &str,
) -> Vec<&'a str> {
    embedded_questions_for_required_body(graph, object, formula)
}

/// Returns questions rendered at the typed `ReferentNode.body` reference site.
///
/// `EventualityNode::references_into` visits `content` before `body`, then its
/// parameters and `embeddedQuestions`; `ReferentNode` has only `body`.  A shared
/// content/body formula is therefore represented at `CONTENT` exactly once.
#[requires(true)]
#[ensures(ret.iter().all(|question| graph.objects.contains_key(*question)))]
fn embedded_questions_rendered_with_body<'a>(
    graph: &'a GraphData,
    object: &'a Map<String, Value>,
    formula: &str,
) -> Vec<&'a str> {
    if optional_string(object, "content") == Some(formula) {
        Vec::new()
    } else {
        embedded_questions_for_required_body(graph, object, formula)
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|question| graph.objects.contains_key(*question)))]
fn embedded_questions_for_required_body<'a>(
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
fn embedded_question_parameters_for_formula(
    graph: &GraphData,
    object: &Map<String, Value>,
    formula: &str,
) -> Vec<String> {
    let mut parameters = Vec::new();
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

#[requires(true)]
#[ensures(ret.iter().all(|parameter| graph.objects.contains_key(parameter)))]
fn abstraction_body_parameters(
    graph: &GraphData,
    object: &Map<String, Value>,
    formula: &str,
) -> Vec<String> {
    let mut parameters = object
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| {
            parameter
                .as_str()
                .unwrap_or_else(|| panic!("abstraction parameter must be an id"))
                .to_owned()
        })
        .collect::<Vec<_>>();
    parameters.extend(embedded_question_parameters_for_formula(
        graph, object, formula,
    ));
    parameters.sort();
    parameters.dedup();
    parameters
}

#[requires(true)]
#[ensures(ret.iter().all(|parameter| graph.objects.contains_key(parameter)))]
fn abstraction_content_parameters(
    graph: &GraphData,
    object: &Map<String, Value>,
    formula: &str,
) -> Vec<String> {
    // visit_abstraction_scope traverses content in the current environment;
    // only a question whose required body is this formula adds binders here.
    embedded_question_parameters_for_formula(graph, object, formula)
}

impl RenderState {
    #[requires(true)]
    #[ensures(ret.iter().all(|warning| warning.name == "WARNING"))]
    fn render_diagnostics(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> Vec<XmlElement> {
        let Some(diagnostics) = object.get("diagnostics") else {
            return Vec::new();
        };
        self.account_field(graph, object, "diagnostics");
        diagnostics
            .as_array()
            .unwrap_or_else(|| panic!("semantic diagnostics must be a list"))
            .iter()
            .map(|diagnostic| {
                let diagnostic = diagnostic
                    .as_object()
                    .unwrap_or_else(|| panic!("semantic diagnostic must be a record"));
                self.account_object(graph, diagnostic);
                assert_eq!(
                    diagnostic.len(),
                    2,
                    "semantic diagnostic has an unknown field shape"
                );
                let severity = string_field(diagnostic, "severity");
                self.account_field(graph, diagnostic, "severity");
                assert_eq!(
                    severity, "warning",
                    "SFN-XML WARNING requires warning diagnostic severity"
                );
                let message = string_field(diagnostic, "message");
                self.account_field(graph, diagnostic, "message");
                assert!(!message.is_empty(), "semantic diagnostic message is empty");
                let mut warning = XmlElement::new("WARNING");
                warning.text = Some(message.to_owned());
                warning
            })
            .collect()
    }

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
        let mut rendered = match optional_string(object, "type") {
            Some("utterance") => self.render_utterance(graph, key, object),
            Some("predication") => self.render_predication(graph, key, object),
            Some("formula") => self.render_formula(graph, key, object),
            Some("referent") => self.render_referent(graph, key, object),
            Some("quantity") => self.render_quantity(graph, object),
            Some("parameter") => self.render_parameter(graph, object),
            Some("sequence") => self.render_sequence(graph, key, object),
            Some("displayedContent") => self.render_displayed_content(graph, object),
            Some("mathExpression") => self.render_math_expression(graph, object),
            Some("relationMetadata") => self.render_relation_metadata(graph, object),
            Some("question") => self.render_question(graph, key, object),
            _ => self.render_unknown_object(graph, object),
        };
        rendered.extend(self.render_diagnostics(graph, object));
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
    #[ensures(ret.name == "QUESTION")]
    fn render_question(
        &mut self,
        graph: &GraphData,
        key: &str,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let parameters = question_parameters(graph, object);
        self.account_field(graph, object, "kind");
        self.account_field(graph, object, "mode");
        self.account_field(graph, object, "domain");
        let mut result = XmlElement::with_attributes(
            "QUESTION",
            [
                ("KIND", enum_token(&object["kind"])),
                ("MODE", enum_token(&object["mode"])),
                ("DOMAIN", flat_sort_name(&object["domain"])),
            ],
        );
        let mut handled = Vec::from(["type", "kind", "mode", "domain", "body"]);

        let mut removed = Vec::new();
        for index in (0..self.bound_variable_stack.len()).rev() {
            if parameters.contains(&self.bound_variable_stack[index]) {
                removed.push((index, self.bound_variable_stack.remove(index)));
            }
        }
        for (field, tag) in [("asker", "ASKER"), ("respondent", "RESPONDENT")] {
            let pointer = string_field(object, field);
            self.account_field(graph, object, field);
            result.push(self.wrap_pointer(graph, tag, pointer, Vec::new()));
            handled.push(field);
        }
        if let Some(slots) = object.get("slots").and_then(Value::as_array) {
            self.account_field(graph, object, "slots");
            let mut rendered_slots = XmlElement::new("SLOTS");
            for slot in slots {
                let slot = json_object(slot);
                self.account_object(graph, slot);
                self.account_field(graph, slot, "role");
                let mut rendered_slot =
                    XmlElement::with_attributes("SLOT", [("ROLE", enum_token(&slot["role"]))]);
                let mut slot_handled = Vec::from(["role"]);
                if let Some(kind) = slot.get("kind") {
                    self.account_field(graph, slot, "kind");
                    rendered_slot.set("KIND", enum_token(kind));
                    slot_handled.push("kind");
                }
                if let Some(domain) = slot.get("domain") {
                    self.account_field(graph, slot, "domain");
                    rendered_slot.set("DOMAIN", flat_sort_name(domain));
                    slot_handled.push("domain");
                }
                if let Some(parameter) = optional_string(slot, "parameter") {
                    self.account_field(graph, slot, "parameter");
                    rendered_slot.push(self.wrap_pointer(
                        graph,
                        "PARAMETER",
                        parameter,
                        Vec::new(),
                    ));
                    slot_handled.push("parameter");
                }
                rendered_slot.extend(self.extras(graph, slot, &slot_handled));
                rendered_slots.push(rendered_slot);
            }
            result.push(rendered_slots);
            handled.push("slots");
        }
        for (field, tag) in [
            ("focus", "FOCUS"),
            ("presupposedAnswer", "PRESUPPOSED-ANSWER"),
        ] {
            if let Some(pointer) = optional_string(object, field) {
                self.account_field(graph, object, field);
                result.push(self.wrap_pointer(graph, tag, pointer, Vec::new()));
                handled.push(field);
            }
        }
        for (index, binder) in removed.into_iter().rev() {
            self.bound_variable_stack.insert(index, binder);
        }

        let body_key = string_field(object, "body");
        self.account_field(graph, object, "body");
        let missing: Vec<String> = parameters
            .iter()
            .filter(|parameter| !self.bound_variable_stack.contains(parameter))
            .cloned()
            .collect();
        self.bound_variable_stack.extend(missing.iter().cloned());
        let (declarations, rendered_body) = self.scoped_parts(
            graph,
            vec!["question-body".to_owned(), key.to_owned()],
            |state, graph| state.render_pointer(graph, body_key),
        );
        self.bound_variable_stack
            .truncate(self.bound_variable_stack.len() - missing.len());
        let mut body = XmlElement::new("BODY");
        Self::append_defs(&mut body, declarations);
        body.push(rendered_body);
        result.push(body);
        result.extend(self.extras(graph, object, &handled));
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

    /// Render the tanru-link sidecar of an unprojected tanru-link predication
    /// (the loud fallback form): a KIND-COMPOSITION of pointer wrappers to the
    /// head predication and the modifier relation, occupying the predication's
    /// relation slot (jbotci#719).
    #[requires(object.contains_key("tanruLink"))]
    #[ensures(ret.name == "KIND-COMPOSITION")]
    fn render_tanru_link_sidecar(
        &mut self,
        graph: &GraphData,
        object: &Map<String, Value>,
    ) -> XmlElement {
        let link = json_object(&object["tanruLink"]);
        self.account_field(graph, object, "tanruLink");
        let mut composition = XmlElement::new("KIND-COMPOSITION");
        let head = string_field(link, "head");
        self.account_field(graph, link, "head");
        composition.push(self.wrap_pointer(graph, "KIND", head, Vec::new()));
        let modifier = string_field(link, "modifier");
        self.account_field(graph, link, "modifier");
        composition.push(self.wrap_pointer(graph, "MODIFIER", modifier, Vec::new()));
        if link.contains_key("relationLabel") {
            self.record_field_omission(
                graph,
                link,
                "relationLabel",
                XmlWaiverFamily::CompositionRelationLabel,
            );
        }
        composition.extend(self.extras(graph, link, &["head", "modifier", "relationLabel"]));
        composition
    }

    /// Render the typed relation-expression view of a projected tanru
    /// predication (jbotci#719): a KIND-COMPOSITION in the relation slot.
    /// `host_predicate` is the enclosing predication's own relation, rendered
    /// as the PREDICATE= of `host` operands.
    #[requires(view.contains_key("kind") && view.contains_key("modifier"))]
    #[ensures(ret.name == "KIND-COMPOSITION")]
    fn render_relation_composition(
        &mut self,
        graph: &GraphData,
        view: &Map<String, Value>,
        host_predicate: &str,
    ) -> XmlElement {
        let mut result = XmlElement::new("KIND-COMPOSITION");
        if view.get("grouping").and_then(Value::as_str) == Some("explicit") {
            result.set("GROUPING", "EXPLICIT");
        }
        result.push(self.render_relation_operand(
            graph,
            "KIND",
            json_object(&view["kind"]),
            Some(host_predicate),
        ));
        result.push(self.render_relation_operand(
            graph,
            "MODIFIER",
            json_object(&view["modifier"]),
            None,
        ));
        result
    }

    /// Render one operand of a relation composition under `tag` (KIND,
    /// MODIFIER, or RELATION inside a relation-level CONNECTIVE).
    #[requires(true)]
    #[ensures(ret.name == tag)]
    fn render_relation_operand(
        &mut self,
        graph: &GraphData,
        tag: &str,
        operand: &Map<String, Value>,
        host_predicate: Option<&str>,
    ) -> XmlElement {
        match optional_string(operand, "type") {
            Some("host") => {
                let mut result = XmlElement::new(tag);
                result.set(
                    "PREDICATE",
                    predicate_symbol(
                        host_predicate.expect("host operand requires the predication relation"),
                    ),
                );
                Self::set_participant_place(&mut result, operand);
                result
            }
            Some("lexical") => self.render_relation_lexical(graph, tag, operand),
            Some("kindComposition") => {
                let mut result = XmlElement::new(tag);
                let mut body = XmlElement::new("BODY");
                body.push(self.render_relation_composition(
                    graph,
                    operand,
                    host_predicate.unwrap_or_default(),
                ));
                result.push(body);
                result
            }
            Some("connective") => {
                let operator = optional_string(operand, "operator")
                    .unwrap_or_else(|| panic!("relation connective lacks an operator"));
                let mut connective =
                    XmlElement::with_attributes("CONNECTIVE", [("OPERATOR", enum_string(operator))]);
                for leaf in operand
                    .get("operands")
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| panic!("relation connective lacks operands"))
                {
                    connective.push(self.render_relation_operand(
                        graph,
                        "RELATION",
                        json_object(leaf),
                        None,
                    ));
                }
                let mut result = XmlElement::new(tag);
                let mut body = XmlElement::new("BODY");
                body.push(connective);
                result.push(body);
                result
            }
            Some("reference") => {
                let relation = optional_string(operand, "relation")
                    .unwrap_or_else(|| panic!("relation reference operand lacks a target"));
                let rendered = self.render_pointer(graph, relation);
                let mut result = XmlElement::new(tag);
                if Self::is_reference(&rendered) {
                    result.set("REF", rendered.attributes[0].1.clone());
                } else {
                    let mut body = XmlElement::new("BODY");
                    body.push(rendered);
                    result.push(body);
                }
                result
            }
            other => panic!("unknown relation operand kind: {other:?}"),
        }
    }

    /// Render a compact lexical relation leaf: PREDICATE= plus an optional
    /// PARTICIPANT-PLACE= (default 1) and fixed non-participant ARGs.
    #[requires(true)]
    #[ensures(ret.name == tag)]
    fn render_relation_lexical(
        &mut self,
        graph: &GraphData,
        tag: &str,
        operand: &Map<String, Value>,
    ) -> XmlElement {
        let mut result = XmlElement::new(tag);
        result.set(
            "PREDICATE",
            predicate_symbol(
                optional_string(operand, "predicate")
                    .unwrap_or_else(|| panic!("lexical relation leaf lacks a predicate")),
            ),
        );
        Self::set_participant_place(&mut result, operand);
        if let Some(fixed) = operand.get("fixedArguments").and_then(Value::as_object) {
            let mut places: Vec<(&String, &str)> = fixed
                .iter()
                .map(|(place, target)| {
                    (
                        place,
                        target.as_str().unwrap_or_else(|| {
                            panic!("fixed relation argument target must be an id")
                        }),
                    )
                })
                .collect();
            places.sort_by_key(|(place, _)| {
                place
                    .parse::<usize>()
                    .unwrap_or_else(|_| panic!("fixed relation argument place must be numeric"))
            });
            for (place, target) in places {
                result.push(self.wrap_pointer(
                    graph,
                    "ARG",
                    target,
                    vec![("INDEX", place.clone())],
                ));
            }
        }
        result
    }

    /// PARTICIPANT-PLACE= is stated exactly when the composition participant
    /// fills a lexical place other than the first.
    #[requires(true)]
    #[ensures(true)]
    fn set_participant_place(result: &mut XmlElement, operand: &Map<String, Value>) {
        let place = operand
            .get("participantPlace")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        if place != 1 {
            result.set("PARTICIPANT-PLACE", place.to_string());
        }
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
        let relation_value = if let Some(view) =
            object.get("relationExpression").and_then(Value::as_object)
        {
            // A projected tanru (#719): the relation slot carries the composite
            // predicate expression; the predication's own `relation` renders as
            // the host KIND leaf's PREDICATE= rather than on the element.
            self.account_field_tree(graph, object, "relationExpression");
            handled.push("relationExpression");
            let host_predicate = optional_string(object, "relation")
                .unwrap_or_else(|| panic!("projected predication lacks its host relation"));
            self.account_field(graph, object, "relation");
            Some(self.render_relation_composition(graph, view, host_predicate))
        } else if let Some(relation) = optional_string(object, "relation") {
            self.account_field(graph, object, "relation");
            result.set("PREDICATE", predicate_symbol(relation));
            None
        } else if let Some(parameter) = optional_string(object, "relationParameter") {
            self.account_field(graph, object, "relationParameter");
            Some(self.wrap_pointer(graph, "RELATION", parameter, Vec::new()))
        } else if object.contains_key("tanruLink") {
            // An unprojected tanru-link predication (the recognition guards
            // rejected the compact form): no PREDICATE= — the KIND-COMPOSITION
            // sidecar occupies the relation slot and carries the meaning.
            handled.push("tanruLink");
            Some(self.render_tanru_link_sidecar(graph, object))
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

        if let Some(questions) = object.get("placeQuestions").and_then(Value::as_array) {
            self.account_field(graph, object, "placeQuestions");
            let mut rendered = XmlElement::new("PLACE-QUESTIONS");
            for question in questions {
                rendered.push(self.render_place_question(graph, json_object(question)));
            }
            result.push(rendered);
            handled.push("placeQuestions");
        }

        let mut metadata = XmlElement::new("META");
        if let Some(relation_metadata) = optional_string(object, "relationMetadata") {
            if self.word_cards_present {
                // #709 dedup: with a WORDS section, the nonce word's WORD card
                // carries the decomposition; the body predication renders no
                // RELATION-METADATA (the subtree is accounted rendered-via-card).
                self.account_relation_metadata_via_card(graph, object, relation_metadata);
            } else {
                // Without a WORDS section no card exists to carry the
                // decomposition, and the omissions/waiver discipline does not
                // allow silently dropping it, so the interim body
                // RELATION-METADATA preservation form stays (this also keeps
                // the frozen 48-document corpus byte pins green).
                self.account_field(graph, object, "relationMetadata");
                metadata.push(self.render_pointer(graph, relation_metadata));
            }
            handled.push("relationMetadata");
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
        let mut connector_parts = None;
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
                connector_parts = Some(state.account_connector(graph, connector, operator));
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
        if let Some(value) = content.remove("domainImport") {
            result.push(value);
        }
        if let Some((attributes, children)) = connector_parts {
            for (name, value) in attributes {
                result.set(name, value);
            }
            result.extend(children);
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
                    handled.push("children");
                }
                for field in [
                    "predication",
                    "variable",
                    "sourceVariable",
                    "restriction",
                    "body",
                    "quantity",
                    "eventuality",
                ] {
                    if let Some(pointer) = optional_string(object, field) {
                        state.account_field(graph, object, field);
                        operands.push(state.wrap_pointer(
                            graph,
                            &enum_string(field).replace('_', "-"),
                            pointer,
                            Vec::new(),
                        ));
                        handled.push(field);
                    }
                }
                if let Some(import) = object.get("domainImport") {
                    state.account_field(graph, object, "domainImport");
                    operands.push(Self::scalar("DOMAIN-IMPORT", import));
                    handled.push("domainImport");
                }
            }
            let mut connector_parts = None;
            if let Some(connector) = object.get("connector").and_then(Value::as_object) {
                state.account_field(graph, object, "connector");
                connector_parts = Some(state.account_connector(graph, connector, operator));
                handled.push("connector");
            }
            let extras = state.extras(graph, object, &handled);
            (operands, connector_parts, extras)
        });
        let (operands, connector_parts, extras) = parts;
        let (connector_attributes, connector_children) =
            connector_parts.unwrap_or_default();
        let connector_rendered = !connector_attributes.is_empty() || !connector_children.is_empty();
        if operator == "atom"
            && declarations.is_empty()
            && !connector_rendered
            && extras.is_empty()
        {
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
        // Connector content attaches to the connective element itself now that
        // the CONNECTOR wrapper is gone (#719): TRUTH-TABLE= and PARAMETER=
        // attributes; only unknown extra fields still force a FORMULA wrapper.
        for (name, value) in connector_attributes {
            core.set(name, value);
        }
        core.extend(connector_children);
        if extras.is_empty() {
            return core;
        }
        if core.name == "FORMULA" {
            core.extend(extras);
            return core;
        }
        let mut wrapper = XmlElement::new("FORMULA");
        wrapper.push(core);
        wrapper.extend(extras);
        wrapper
    }
}
