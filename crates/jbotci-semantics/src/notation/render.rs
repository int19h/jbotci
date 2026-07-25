//! The `lean3` graph walk: a faithful Rust port of the Python oracle's `lean3`
//! rendering path (`render_v5.py` at commit `cab176bcce`).
//!
//! # Why this reads the canonical JSON
//!
//! The notation is *defined* — by the research repo's `DESIGN-RECORD.md` /
//! `FREEZE-PHASE-B.md` and by the byte-parity oracle — as a rendering of the
//! `lojban-semantics-json-1` interchange graph (the exact bytes `tersmu
//! --format json` emits). The renderer therefore consumes the model's own
//! canonical serialization ([`serde_json::to_value`]) of each object and walks
//! it with the same field-shape logic the oracle uses, so byte parity is
//! correct by construction rather than reconstructed from the typed model's
//! accessors (which would re-derive the JSON shape anyway, with more room to
//! drift). The public entry ([`super::render_notation`]) is fully typed; the
//! `serde_json::Value` walk is an encapsulated implementation detail justified
//! by the byte-parity contract. Object *iteration order* — which the oracle
//! reads from the JSON's own object insertion order — is taken from the typed
//! [`SemanticGraph::objects`] `BTreeMap`, whose `SemanticObjectId` ordering is
//! byte-for-byte the frozen graph's object order.
//!
//! Only the `lean3` profile is realised here: the frozen `lean3` option set
//! (`opt_compact_dimensions`, `opt_bracket_keys`, `opt_collapse_notcomputed`,
//! `opt_terse_labels`, `opt_glyph_formulas`, `opt_braces`, `opt_short_ids`,
//! `opt_dense_decls`, all on; `opt_provenance_off` following the runtime
//! `provenance` toggle). The experiment-only options that `lean3` leaves off
//! (nav-index, scope-paths, inline-lambda, colocated-defs, discourse-order,
//! infix-implication, inline-introductions, content-ids) are intentionally not
//! ported — they are not part of `lean3` and are unreachable through this API.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde_json::Value;

use crate::model::SemanticGraph;

use super::writer::Writer;

/// `lean3` render configuration. `provenance` is the one runtime toggle
/// (`--provenance` / `opt_provenance_off` off): when set, source spans/text are
/// rendered; otherwise the profile renders semantic content only.
// `#[invariant(true)]`: an audited no-op — a single `bool` toggle, so every
// value is a valid configuration; the field type already expresses the whole
// domain.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lean3Config {
    pub provenance: bool,
}

/// The frozen `SHORT_ID_PREFIX` map (FREEZE-PHASE-B.md section (c)): one prefix
/// per `id_kind_for` kind, in the exact dict insertion order the `ID PREFIXES:`
/// legend reproduces. This is the single source of truth for both the generated
/// IDs and the legend line.
const SHORT_ID_PREFIX: &[(&str, &str)] = &[
    ("reference", "r"),
    ("predication", "p"),
    ("formula", "f"),
    ("quantity", "q"),
    ("utterance", "u"),
    ("sequence", "s"),
    ("mathExpression", "m"),
    ("parameter", "x"),
    ("relation_expression", "l"),
    ("displayed_content", "d"),
];

/// The short prefix for a kind, or `None` for a kind this renderer has never
/// seen (kept as `<kind>_<n>` rather than inventing a prefix — the oracle's
/// `.get`-not-`[...]` "never guess" rule).
#[requires(true)]
#[ensures(true)]
fn short_prefix(kind: &str) -> Option<&'static str> {
    SHORT_ID_PREFIX
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, prefix)| *prefix)
}

/// The `lean3` declaration kind label for a JSON object `type`, mirroring the
/// oracle's `KIND_LABEL` keyed through `id_kind_for`.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn kind_label(id_kind: &str) -> &'static str {
    match id_kind {
        "utterance" => "UTTERANCE",
        "predication" => "PREDICATION",
        "formula" => "FORMULA",
        "reference" => "REFERENCE",
        "relation_expression" => "RELATION EXPRESSION",
        "quantity" => "QUANTITY",
        "parameter" => "PARAMETER",
        "sequence" => "SEQUENCE",
        "displayed_content" => "DISPLAYED CONTENT",
        "mathExpression" => "MATH EXPRESSION",
        _ => "UNKNOWN",
    }
}

/// `id_kind_for`: the ID/declaration kind for an object. A `referent` collapses
/// onto `reference` except when its `sort` is `relation` (a ka-style
/// characteristic function → `relation_expression`); `displayedContent` maps to
/// `displayed_content`; every other `type` is used as-is.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn id_kind_for(obj: &Value) -> &str {
    let t = obj.get("type").and_then(Value::as_str).unwrap_or("");
    if t == "referent" {
        if obj.get("sort").and_then(Value::as_str) == Some("relation") {
            return "relation_expression";
        }
        return "reference";
    }
    if t == "displayedContent" {
        return "displayed_content";
    }
    t
}

/// The trailing decimal run of a graph key (`"eventuality/locution:13"` → `13`),
/// reused as the digit-bearing part of the generated ID. When a key has no
/// trailing digits, the oracle substitutes a `\W+`→`_` sanitisation; that path
/// is never hit by real graphs (their counters always end in a digit) but is
/// reproduced for faithfulness.
#[requires(true)]
#[ensures(!ret.is_empty() || key.is_empty())]
fn key_number(key: &str) -> String {
    let digits: String = key
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if !digits.is_empty() {
        return digits;
    }
    sanitize_non_word(key)
}

/// Replace every maximal run of non-word characters with a single `_` (Python
/// `re.sub(r"\W+", "_", s)`; word = `[A-Za-z0-9_]`).
#[requires(true)]
#[ensures(true)]
fn sanitize_non_word(s: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    out
}

/// Build the `graph key -> generated id` map, in object iteration order, using
/// the `opt_short_ids` (N8c) scheme with the frozen collision fallback. Every ID
/// is verified unique across the whole document; a genuine collision falls back
/// to appending the disambiguated source key so uniqueness is never silently
/// violated.
#[requires(true)]
#[ensures(ret.len() == order.len())]
fn build_id_map(order: &[String], objects: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    let mut id_map = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for key in order {
        let obj = &objects[key];
        let kind = id_kind_for(obj);
        let num = key_number(key);
        let mut vid = match short_prefix(kind) {
            Some(prefix) => format!("{prefix}{num}"),
            None => format!("{kind}_{num}"),
        };
        if used.contains(&vid) {
            vid = format!("{vid}_{}", sanitize_colon_slash(key));
        }
        used.insert(vid.clone());
        id_map.insert(key.clone(), vid);
    }
    id_map
}

/// The oracle's collision-fallback key sanitiser: `re.sub(r'[:/]', '_', key)`.
#[requires(true)]
#[ensures(true)]
fn sanitize_colon_slash(key: &str) -> String {
    key.chars()
        .map(|c| if c == ':' || c == '/' { '_' } else { c })
        .collect()
}

// ---------------------------------------------------------------------------
// Lexical helpers (ENUM / LEXICAL / QUOTE / title_sort), ported from the oracle.
// ---------------------------------------------------------------------------

/// Split camelCase/kebab-case/snake_case into words (Python
/// `re.split(r"[-_]|(?=[A-Z])", s)` with empties dropped).
#[requires(true)]
#[ensures(true)]
fn split_words(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch == '-' || ch == '_' {
            if !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if ch.is_ascii_uppercase() && !cur.is_empty() {
            parts.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    parts
}

/// Render a closed schema enum tag as an uppercase English keyword
/// (`atLeast` → `AT LEAST`, `same-topic-continuation` → `SAME TOPIC
/// CONTINUATION`).
#[requires(true)]
#[ensures(true)]
fn enum_render(s: &str) -> String {
    let words = split_words(s);
    if words.is_empty() {
        return s.to_uppercase();
    }
    words
        .iter()
        .map(|w| w.to_uppercase())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a Lojban-vocabulary field: bare lowercase, unless the value cannot be
/// Lojban (contains an uppercase letter → a synthesized internal name like
/// `memberOf`), in which case it falls back to [`enum_render`].
#[requires(true)]
#[ensures(true)]
fn lexical(s: &str) -> String {
    if s.chars().any(|c| c.is_ascii_uppercase()) {
        enum_render(s)
    } else {
        s.to_string()
    }
}

/// Quote witness/content text as a JSON string with non-ASCII preserved (Python
/// `json.dumps(s, ensure_ascii=False)`; `serde_json::to_string` matches this,
/// escaping only `"`, `\`, and control characters).
#[requires(true)]
#[ensures(ret.starts_with('"') && ret.ends_with('"'))]
fn quote(s: &str) -> String {
    serde_json::to_string(s).expect("a string always serializes to JSON")
}

/// `entity` → `Entity`, `eventuality/locution` → `Eventuality/Locution`
/// (Python `str.capitalize` per `/`-split part: first char upper, rest lower).
#[requires(true)]
#[ensures(true)]
fn title_sort(sort: &str) -> String {
    sort.split('/')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join("/")
}

/// Python `str.capitalize`: first character uppercased, the rest lowercased.
#[requires(true)]
#[ensures(true)]
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.extend(chars.flat_map(|c| c.to_lowercase()));
            out
        }
    }
}

/// The glyph substitution for the five operators `opt_glyph_formulas` (N1a)
/// names; every other operator stays worded via [`enum_render`].
#[requires(true)]
#[ensures(true)]
fn glyph_operator(op: &str) -> Option<&'static str> {
    match op {
        "not" => Some("¬"),
        "and" => Some("∧"),
        "or" => Some("∨"),
        "exists" => Some("∃"),
        "forall" => Some("∀"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The render context and per-kind renderers.
// ---------------------------------------------------------------------------

/// Threads the object lookup, id map, and the one `lean3` runtime toggle through
/// the per-kind render functions.
// `#[invariant(true)]`: an audited no-op — `objects` and `id_map` are borrowed
// views built together over the same graph, and every field combination is a
// valid rendering context (missing pointers fall back gracefully in `Ctx::id`).
#[invariant(true)]
struct Ctx<'a> {
    objects: &'a BTreeMap<String, Value>,
    id_map: &'a BTreeMap<String, String>,
    provenance: bool,
}

impl Ctx<'_> {
    /// The generated ID for a graph key (`id_map[key]`); the pointed-to object
    /// is guaranteed present in a well-formed graph.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn id(&self, key: &str) -> &str {
        self.id_map
            .get(key)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("graph pointer `{key}` has no generated id (malformed graph)"))
    }

    /// The generated ID for a graph key held in a JSON string value.
    #[requires(true)]
    #[ensures(true)]
    fn id_of(&self, value: &Value) -> String {
        self.id(value.as_str().unwrap_or("")).to_string()
    }
}

/// Read a string field.
#[requires(true)]
#[ensures(true)]
fn field_str<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// A JSON number rendered bare (integers as `190`, no quotes), for VALUE/ARITY.
#[requires(true)]
#[ensures(true)]
fn number_str(value: &Value) -> String {
    value.to_string()
}

/// The public render entry: walk `graph` and produce its `lean3` notation.
#[requires(true)]
#[ensures(ret.ends_with('\n'))]
pub fn render_lean3(graph: &SemanticGraph, config: Lean3Config) -> String {
    let order: Vec<String> = graph.objects.keys().map(|id| id.to_string()).collect();
    let objects: BTreeMap<String, Value> = graph
        .objects
        .iter()
        .map(|(id, object)| {
            (
                id.to_string(),
                serde_json::to_value(object).expect("semantic objects serialize to JSON"),
            )
        })
        .collect();
    let id_map = build_id_map(&order, &objects);
    let root = graph.root.to_string();

    let ctx = Ctx {
        objects: &objects,
        id_map: &id_map,
        provenance: config.provenance,
    };

    let mut w = Writer::new(/* opt_braces */ true, /* opt_dense_decls */ true);
    w.declaration("SEMANTIC DOCUMENT", "document_1", None, false, |w| {
        w.field("ROOT", ctx.id(&root));
        // N8c: the ID-prefix legend, generated from the same SHORT_ID_PREFIX
        // table the IDs are built from, so it can never drift.
        let legend = SHORT_ID_PREFIX
            .iter()
            .map(|(kind, prefix)| format!("{prefix}={kind}"))
            .collect::<Vec<_>>()
            .join(" ");
        w.field("ID PREFIXES", &legend);
        // opt_collapse_notcomputed: the single document-level NOT COMPUTED note.
        w.collection("NOT COMPUTED", |w| {
            w.entry("denotation-multiplicity");
        });
        w.collection("DECLARATIONS", |w| {
            for key in &order {
                render_one(w, &ctx, key, &objects[key]);
            }
        });
    });
    w.finish()
}

/// Dispatch one object to its per-kind renderer (the oracle's `render_one`).
#[requires(true)]
#[ensures(true)]
fn render_one(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    match obj.get("type").and_then(Value::as_str).unwrap_or("") {
        "utterance" => render_utterance(w, ctx, key, obj),
        "predication" => render_predication(w, ctx, key, obj),
        "formula" => render_formula(w, ctx, key, obj),
        "referent" => {
            if obj.get("sort").and_then(Value::as_str) == Some("relation") {
                render_relation_expression(w, ctx, key, obj);
            } else {
                render_reference(w, ctx, key, obj);
            }
        }
        "quantity" => render_quantity(w, ctx, key, obj),
        "parameter" => render_parameter(w, ctx, key, obj),
        "sequence" => render_sequence(w, ctx, key, obj),
        "displayedContent" => render_displayed_content(w, ctx, key, obj),
        "mathExpression" => render_math_expression(w, ctx, key, obj),
        other => {
            let vid = ctx.id(key).to_string();
            let other = other.to_string();
            w.declaration("UNKNOWN", &vid, None, true, |w| {
                w.field("NOT COMPUTED", &format!("renderer-support({other})"));
            });
        }
    }
}

/// Optional provenance block (§5), rendered only with `--provenance`.
#[requires(true)]
#[ensures(true)]
fn render_source(w: &mut Writer, ctx: &Ctx, obj: &Value) {
    if !ctx.provenance {
        return;
    }
    let Some(src) = obj.get("source").filter(|s| !s.is_null()) else {
        return;
    };
    w.heading("PROVENANCE", |w| {
        if let Some(span) = src.get("span").filter(|s| !s.is_null()) {
            if let (Some(start), Some(end)) = (span.get("byteStart"), span.get("byteEnd")) {
                w.field("BYTE SPAN", &format!("{}..{}", number_str(start), number_str(end)));
            }
        }
        if let Some(text) = field_str(src, "text") {
            w.field("TEXT", &quote(text));
        }
        if let Some(construct) = field_str(src, "construct") {
            w.field("CONSTRUCT", &quote(construct));
        }
    });
}

/// `scopeDependence`: a `SCOPE DEPENDENCE: <kind>;` scalar, or a heading with a
/// `MAY DEPEND ON { ... }` collection when the referent depends on binders.
#[requires(true)]
#[ensures(true)]
fn render_scope_dependence(w: &mut Writer, ctx: &Ctx, sd: &Value) {
    let kind = field_str(sd, "kind").unwrap_or("");
    let deps = sd.get("mayDependOn").and_then(Value::as_array);
    match deps.filter(|d| !d.is_empty()) {
        Some(deps) => {
            let heading = format!("SCOPE DEPENDENCE IS {}", enum_render(kind));
            w.heading(&heading, |w| {
                w.collection("MAY DEPEND ON", |w| {
                    for d in deps {
                        w.entry(&ctx.id_of(d));
                    }
                });
            });
        }
        None => w.field("SCOPE DEPENDENCE", &enum_render(kind)),
    }
}

/// The graph's own JSON field names for the four dimensions no sample graph has
/// ever serialized; a future graph that starts emitting one is flagged rather
/// than guessed at.
const UNOBSERVED_DIMENSION_FIELDS: &[(&str, &str)] = &[
    ("SPACE", "space"),
    ("SPATIAL ASPECT", "spatialAspect"),
    ("SPATIAL RECURRENCE", "spatialRecurrence"),
    ("DETAILS", "details"),
];

/// One `recurrence` list entry's rendered text (`<KIND> INTRODUCED BY <cmavo>`
/// plus, for occurrence/ordinal kinds, ` QUANTITY <id>`).
#[requires(true)]
#[ensures(true)]
fn recurrence_item_text(ctx: &Ctx, item: &Value) -> String {
    let kind = field_str(item, "kind").unwrap_or("");
    let introduced_by = field_str(item, "introducedBy").unwrap_or("");
    let mut text = format!("{} INTRODUCED BY {}", enum_render(kind), lexical(introduced_by));
    if kind == "occurrenceCount" || kind == "ordinalOccurrence" {
        match item.get("quantity") {
            Some(quantity) => text.push_str(&format!(" QUANTITY {}", ctx.id_of(quantity))),
            None => return format!("NOT COMPUTED: recurrence-item-shape({kind})"),
        }
    } else if kind != "habitually" {
        return format!("NOT COMPUTED: recurrence-item-shape({kind})");
    }
    text
}

/// The eight shipped eventuality dimensions as one compact, COMPLETE record
/// (`opt_compact_dimensions`), in fixed order. `UNSPECIFIED` means the dimension
/// is absent from the JSON (explicit-over-implicit). Eventuality-only structure;
/// non-eventuality references get no dimension block.
#[requires(true)]
#[ensures(true)]
fn render_eventuality_dimensions(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let sort = obj.get("sort").and_then(Value::as_str);
    if sort != Some("eventuality") && sort != Some("eventuality/locution") {
        return;
    }
    let mut pairs: Vec<(String, String)> = Vec::new();

    match obj.get("time").filter(|t| !t.is_null()) {
        Some(t) => {
            let relation = enum_render(field_str(t, "relation").unwrap_or(""));
            let anchor = t.get("anchor").map(|a| ctx.id_of(a)).unwrap_or_default();
            pairs.push((
                "time".to_string(),
                format!("{{ relation = {relation}, anchor = {anchor} }}"),
            ));
        }
        None => pairs.push(("time".to_string(), "UNSPECIFIED".to_string())),
    }

    match obj.get("actuality").filter(|a| !a.is_null()) {
        Some(a) => pairs.push((
            "actuality".to_string(),
            enum_render(field_str(a, "kind").unwrap_or("")),
        )),
        None => pairs.push(("actuality".to_string(), "UNSPECIFIED".to_string())),
    }

    match obj.get("aspect").filter(|a| !a.is_null()) {
        Some(a) => {
            if let Some(contour) = field_str(a, "contour") {
                pairs.push(("aspect".to_string(), enum_render(contour)));
            } else {
                pairs.push((
                    "aspect".to_string(),
                    "NOT_COMPUTED(dimension-shape(aspect))".to_string(),
                ));
            }
        }
        None => pairs.push(("aspect".to_string(), "UNSPECIFIED".to_string())),
    }

    match obj.get("recurrence").and_then(Value::as_array).filter(|r| !r.is_empty()) {
        Some(recurrence) => {
            let items = recurrence
                .iter()
                .map(|item| recurrence_item_text(ctx, item))
                .collect::<Vec<_>>()
                .join(", ");
            pairs.push(("recurrence".to_string(), format!("[ {items} ]")));
        }
        None => pairs.push(("recurrence".to_string(), "UNSPECIFIED".to_string())),
    }

    for (label, json_key) in UNOBSERVED_DIMENSION_FIELDS {
        let lower = label.to_lowercase().replace(' ', "_");
        if obj.get(*json_key).is_some_and(|v| !v.is_null()) {
            pairs.push((lower, format!("NOT_COMPUTED(dimension-shape({json_key}))")));
        } else {
            pairs.push((lower, "UNSPECIFIED".to_string()));
        }
    }

    w.dimension_record(ctx.id(key), &pairs);
}

/// A `descriptor` block: `DESCRIPTOR IS <kind>` heading with its populated
/// fields (word, speaker, body, quantity, name, relative clauses) in that order.
#[requires(true)]
#[ensures(true)]
fn render_descriptor(w: &mut Writer, ctx: &Ctx, d: &Value) {
    let kind = field_str(d, "kind").unwrap_or("");
    let heading = format!("DESCRIPTOR IS {}", enum_render(kind));
    w.heading(&heading, |w| {
        if let Some(word) = field_str(d, "word") {
            w.field("WORD", &lexical(word));
        }
        if let Some(speaker) = d.get("speaker") {
            w.field("SPEAKER", &ctx.id_of(speaker));
        }
        if let Some(body) = d.get("body") {
            w.field("BODY", &ctx.id_of(body));
        }
        if let Some(quantity) = d.get("quantity") {
            w.field("QUANTITY", &ctx.id_of(quantity));
        }
        if let Some(name) = field_str(d, "name") {
            w.field("NAME", &quote(name));
        }
        if let Some(clauses) = d.get("relativeClauses").and_then(Value::as_array).filter(|c| !c.is_empty()) {
            w.collection("RELATIVE CLAUSES", |w| {
                for clause in clauses {
                    let text = format!(
                        "{} {}",
                        enum_render(field_str(clause, "kind").unwrap_or("")),
                        clause.get("body").map(|b| ctx.id_of(b)).unwrap_or_default()
                    );
                    w.entry(&text);
                }
            });
        }
    });
}

/// §6.3 operand mode: `VALUE <id>` for a bound singular value (`parameter`
/// target), `REFERENCE DENOTATION <id>` for a full reference denotation.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn operand_ref(ctx: &Ctx, value: &Value) -> String {
    let val = value.as_str().unwrap_or("");
    let is_parameter = ctx
        .objects
        .get(val)
        .and_then(|target| target.get("type"))
        .and_then(Value::as_str)
        == Some("parameter");
    if is_parameter {
        format!("VALUE {}", ctx.id(val))
    } else {
        format!("REFERENCE DENOTATION {}", ctx.id(val))
    }
}

#[requires(true)]
#[ensures(true)]
fn render_utterance(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("UTTERANCE", &vid, None, true, |w| {
        w.field("FORCE", &enum_render(field_str(obj, "force").unwrap_or("")));
        w.field("SPEAKER", &ctx.id_of(&obj["speaker"]));
        w.field("AUDIENCE", &ctx.id_of(&obj["audience"]));
        w.field("EVENTUALITY", &ctx.id_of(&obj["eventuality"]));
        if let Some(content) = obj.get("content") {
            w.field("CONTENT", &ctx.id_of(content));
        }
        if let Some(vocative_kind) = field_str(obj, "vocativeKind") {
            w.field("VOCATIVE KIND", &enum_render(vocative_kind));
        }
        if let Some(dg) = obj.get("deicticGround").filter(|d| !d.is_null()) {
            w.heading("DEICTIC GROUND", |w| {
                w.field("TIME", &ctx.id_of(&dg["time"]));
                w.field("PLACE", &ctx.id_of(&dg["place"]));
            });
        }
        if let Some(asides) = obj.get("asides").and_then(Value::as_array).filter(|a| !a.is_empty()) {
            w.collection("ASIDES", |w| {
                for a in asides {
                    w.entry(&ctx.id_of(a));
                }
            });
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_predication(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("PREDICATION", &vid, None, true, |w| {
        w.field("RELATION", &lexical(field_str(obj, "relation").unwrap_or("")));
        if let Some(eventuality) = obj.get("eventuality") {
            w.field("EVENTUALITY", &ctx.id_of(eventuality));
        }
        if let Some(args) = obj.get("arguments").and_then(Value::as_object).filter(|a| !a.is_empty()) {
            // opt_terse_labels: ARGS; opt_bracket_keys: [N]:. Canonical x1..xn
            // emission order.
            let mut keys: Vec<&String> = args.keys().collect();
            keys.sort_by_key(|k| place_number(k));
            w.ordered("ARGS", |w| {
                for xk in keys {
                    let n = place_number(xk);
                    let value = &args[xk]["value"];
                    let operand = operand_ref(ctx, value);
                    w.entry(&format!("[{n}]: {operand}"));
                }
            });
        }
        w.field("MODE", &enum_render(field_str(obj, "mode").unwrap_or("")));
        render_source(w, ctx, obj);
    });
}

/// The 1-based place number embedded in an argument key (`x2` → 2).
#[requires(true)]
#[ensures(true)]
fn place_number(place: &str) -> usize {
    place
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// A formula's own `connector` (`full = false`): CONNECTIVE SOURCE plus, when
/// present, TRUTH TABLE. `connector.parameter`, never observed non-null, is
/// flagged rather than dropped.
#[requires(true)]
#[ensures(true)]
fn render_connector(w: &mut Writer, connector: Option<&Value>, full: bool) {
    let Some(connector) = connector.filter(|c| !c.is_null()) else {
        return;
    };
    w.field("CONNECTIVE SOURCE", &lexical(field_str(connector, "source").unwrap_or("")));
    if full {
        w.field("LOCUS", &enum_render(field_str(connector, "locus").unwrap_or("")));
    }
    if let Some(truth_table) = field_str(connector, "truthTable") {
        w.field("TRUTH TABLE", &enum_render(truth_table));
    }
    if connector.get("parameter").is_some_and(|p| !p.is_null()) {
        w.field("NOT COMPUTED", "connector-parameter");
    }
}

#[requires(true)]
#[ensures(true)]
fn render_formula(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    let op = field_str(obj, "operator").unwrap_or("");
    // opt_glyph_formulas: glyph for the five named operators, else worded.
    let variant = glyph_operator(op)
        .map(str::to_string)
        .unwrap_or_else(|| enum_render(op));
    let op = op.to_string();
    w.declaration("FORMULA", &vid, Some(&variant), true, |w| {
        let mut rendered_body = false;
        if let Some(predication) = obj.get("predication") {
            w.field("PREDICATION", &ctx.id_of(predication));
            rendered_body = true;
        }
        if let Some(children) = obj.get("children").and_then(Value::as_array) {
            w.ordered("OPERANDS IN ORDER", |w| {
                for c in children {
                    w.entry(&ctx.id_of(c));
                }
            });
            render_connector(w, obj.get("connector"), false);
            rendered_body = true;
        }
        if let Some(variable) = obj.get("variable") {
            w.field("VARIABLE", &ctx.id_of(variable));
            if let Some(restriction) = obj.get("restriction") {
                w.field("RESTRICTION", &ctx.id_of(restriction));
            }
            if let Some(domain_import) = field_str(obj, "domainImport") {
                w.field("DOMAIN IMPORT", &enum_render(domain_import));
            }
            if let Some(body) = obj.get("body") {
                w.field("BODY", &ctx.id_of(body));
            }
            if let Some(quantity) = obj.get("quantity") {
                w.field("QUANTITY", &ctx.id_of(quantity));
            }
            rendered_body = true;
        }
        if !rendered_body {
            w.field("NOT COMPUTED", &format!("formula-shape({op})"));
        }
        if let Some(be) = obj.get("boundEventualities").and_then(Value::as_array).filter(|b| !b.is_empty()) {
            w.collection("BOUND EVENTUALITIES", |w| {
                for e in be {
                    w.entry(&ctx.id_of(e));
                }
            });
        }
        render_source(w, ctx, obj);
    });
}

/// A `sort: sign` referent's own semantic content (KIND plus mode-specific
/// quotation/word fields). Renders in every profile (it is denotation, not
/// provenance).
#[requires(true)]
#[ensures(true)]
fn render_sign_content(w: &mut Writer, ctx: &Ctx, obj: &Value) {
    let Some(kind) = field_str(obj, "kind") else {
        return;
    };
    w.field("KIND", &enum_render(kind));
    match kind {
        "quotation" => {
            let q = obj.get("quotation").cloned().unwrap_or(Value::Null);
            let mode = field_str(&q, "mode");
            w.field("MODE", &enum_render(mode.unwrap_or("")));
            if mode == Some("parsed") && q.get("utterance").is_some() {
                w.field("QUOTED UTTERANCE", &ctx.id_of(&q["utterance"]));
            } else if mode == Some("opaque") && q.get("text").is_some() {
                w.field("QUOTED TEXT", &quote(field_str(&q, "text").unwrap_or("")));
                if let Some(delimiter) = field_str(&q, "delimiter") {
                    w.field("DELIMITER", &quote(delimiter));
                }
            } else {
                w.field("NOT COMPUTED", &format!("quotation-shape({})", mode.unwrap_or("")));
            }
        }
        "word" => {
            if let Some(text) = field_str(obj, "text") {
                w.field("TEXT", &quote(text));
            }
            if let Some(denotes) = obj.get("denotes") {
                w.field("DENOTES", &ctx.id_of(denotes));
            }
        }
        other => {
            w.field("NOT COMPUTED", &format!("sign-kind-shape({other})"));
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    let key = key.to_string();
    w.declaration("REFERENCE", &vid, None, true, |w| {
        // opt_terse_labels: the tested-winner reference-sort header.
        w.annotate(
            "DENOTES VALUES OF SORT",
            &title_sort(field_str(obj, "sort").unwrap_or("")),
        );
        // opt_collapse_notcomputed: the per-reference denotation-multiplicity
        // note is collapsed to the one document-level NOT COMPUTED block, so no
        // per-reference marker here.
        if let Some(category) = field_str(obj, "category") {
            // N8e: BINDING CATEGORY -> BINDING (the one compressed label).
            w.field("BINDING", &enum_render(category));
        }
        if let Some(indexical) = field_str(obj, "indexical") {
            w.field("INDEXICAL", &enum_render(indexical));
        }
        if let Some(denotation) = field_str(obj, "denotation") {
            w.field("DENOTATION", &enum_render(denotation));
        }
        if let Some(sd) = obj.get("scopeDependence").filter(|s| !s.is_null()) {
            render_scope_dependence(w, ctx, sd);
        }
        render_eventuality_dimensions(w, ctx, &key, obj);
        render_sign_content(w, ctx, obj);
        if let Some(descriptor) = obj.get("descriptor").filter(|d| !d.is_null()) {
            render_descriptor(w, ctx, descriptor);
        }
        if let Some(body) = obj.get("body") {
            w.field("BODY", &ctx.id_of(body));
        }
        if let Some(content) = obj.get("content") {
            w.field("CONTENT", &ctx.id_of(content));
        }
        if let Some(target) = obj.get("target") {
            w.field("TARGET", &ctx.id_of(target));
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_relation_expression(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("RELATION EXPRESSION", &vid, None, true, |w| {
        if let Some(params) = obj.get("parameters").and_then(Value::as_array).filter(|p| !p.is_empty()) {
            // opt_terse_labels: PARAMETERS IN CALLABLE ORDER -> PARAMS.
            w.ordered("PARAMS", |w| {
                for p in params {
                    let param_key = p.as_str().unwrap_or("");
                    let param_sort = ctx
                        .objects
                        .get(param_key)
                        .and_then(|o| field_str(o, "sort"))
                        .map(title_sort)
                        .unwrap_or_default();
                    w.entry(&format!("{} AS {param_sort}", ctx.id(param_key)));
                }
            });
        }
        if let Some(arity) = obj.get("arity") {
            w.field("ARITY", &number_str(arity));
        }
        if let Some(body) = obj.get("body") {
            let body_key = body.as_str().unwrap_or("");
            let body_type = ctx
                .objects
                .get(body_key)
                .and_then(|o| field_str(o, "type"))
                .unwrap_or("");
            w.field("OUTPUT TYPE", &title_sort(body_type));
            w.field("BODY", &ctx.id(body_key).to_string());
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_quantity(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("QUANTITY", &vid, None, true, |w| {
        w.field("FORM", &enum_render(field_str(obj, "form").unwrap_or("")));
        let empty = serde_json::Map::new();
        let val = obj.get("value").and_then(Value::as_object).unwrap_or(&empty);
        let mut handled = std::collections::BTreeSet::new();
        if let Some(integer) = val.get("integer") {
            w.field("VALUE", &number_str(integer));
            handled.insert("integer");
        }
        if let Some(text) = val.get("text").and_then(Value::as_str) {
            w.field("VALUE TEXT", &lexical(text));
            handled.insert("text");
        }
        if let Some(math_expression) = val.get("mathExpression") {
            w.field("VALUE", &ctx.id_of(math_expression));
            handled.insert("mathExpression");
        }
        let unhandled: Vec<&str> = val
            .keys()
            .map(String::as_str)
            .filter(|k| !handled.contains(k))
            .collect();
        if !unhandled.is_empty() {
            w.field(
                "NOT COMPUTED",
                &format!("quantity-value-shape({})", unhandled.join(", ")),
            );
        }
        if let Some(scale) = field_str(obj, "scale") {
            w.field("SCALE", &enum_render(scale));
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_parameter(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("PARAMETER", &vid, None, true, |w| {
        w.field("SORT", &title_sort(field_str(obj, "sort").unwrap_or("")));
        w.field("ROLE", &enum_render(field_str(obj, "role").unwrap_or("")));
        w.field("INTRODUCED BY", &lexical(field_str(obj, "introducedBy").unwrap_or("")));
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_sequence(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("SEQUENCE", &vid, None, true, |w| {
        let empty = Vec::new();
        let items = obj.get("items").and_then(Value::as_array).unwrap_or(&empty);
        w.ordered("ITEMS IN ORDER", |w| {
            for it in items {
                w.entry(&ctx.id_of(it));
            }
        });
        if let Some(cc) = obj.get("connectionClaims").and_then(Value::as_array).filter(|c| !c.is_empty()) {
            w.collection("CONNECTION CLAIMS", |w| {
                for c in cc {
                    w.entry(&ctx.id_of(c));
                }
            });
        }
        if let Some(be) = obj.get("boundEventualities").and_then(Value::as_array).filter(|b| !b.is_empty()) {
            w.collection("BOUND EVENTUALITIES", |w| {
                for e in be {
                    w.entry(&ctx.id_of(e));
                }
            });
        }
        w.field("RELATION", &enum_render(field_str(obj, "relation").unwrap_or("")));
        if let Some(nc) = obj.get("nonlogicalConnection").filter(|n| !n.is_null()) {
            w.heading("NONLOGICAL CONNECTION", |w| {
                let operator = field_str(nc, "operator").unwrap_or("");
                if let Some(rest) = operator.strip_prefix("nonlogical:") {
                    w.field("OPERATOR", &format!("{} {}", enum_render("nonlogical"), lexical(rest)));
                } else {
                    w.field("OPERATOR", &enum_render(operator));
                }
                w.heading("CONNECTOR", |w| {
                    render_connector(w, nc.get("connector"), true);
                });
            });
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_displayed_content(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    w.declaration("DISPLAYED CONTENT", &vid, None, true, |w| {
        w.field("RELATION", &lexical(field_str(obj, "relation").unwrap_or("")));
        if let Some(family) = field_str(obj, "family") {
            w.field("FAMILY", &enum_render(family));
        }
        if let Some(polarity) = field_str(obj, "polarity") {
            w.field("POLARITY", &enum_render(polarity));
        }
        if let Some(assertion_effect) = field_str(obj, "assertionEffect") {
            w.field("ASSERTION EFFECT", &enum_render(assertion_effect));
        }
        if let Some(experiencer) = obj.get("experiencer") {
            w.field("EXPERIENCER", &ctx.id_of(experiencer));
        }
        if let Some(target) = obj.get("target") {
            w.field("TARGET", &ctx.id_of(target));
        }
        if let Some(target_focus) = field_str(obj, "targetFocus") {
            w.field("TARGET FOCUS", &enum_render(target_focus));
        }
        if let Some(anchor) = obj.get("anchor") {
            w.field("ANCHOR", &ctx.id_of(anchor));
        }
        render_source(w, ctx, obj);
    });
}

#[requires(true)]
#[ensures(true)]
fn render_math_expression(w: &mut Writer, ctx: &Ctx, key: &str, obj: &Value) {
    let vid = ctx.id(key).to_string();
    let literal = obj.get("literal").filter(|l| !l.is_null());
    let operator = field_str(obj, "operator");
    let variant = match literal {
        Some(literal) => enum_render(field_str(literal, "kind").unwrap_or("")),
        None => enum_render(operator.unwrap_or("")),
    };
    w.declaration("MATH EXPRESSION", &vid, Some(&variant), true, |w| {
        if let Some(literal) = literal {
            let kind = field_str(literal, "kind").unwrap_or("");
            if kind == "integer" && literal.get("value").is_some() {
                w.field("VALUE", &number_str(&literal["value"]));
            } else {
                w.field("NOT COMPUTED", &format!("math-literal-shape({kind})"));
            }
        } else if operator.is_some() {
            let empty = Vec::new();
            let operands = obj.get("operands").and_then(Value::as_array).unwrap_or(&empty);
            w.ordered("OPERANDS IN ORDER", |w| {
                for (index, operand) in operands.iter().enumerate() {
                    w.entry(&format!("[{}]: {}", index + 1, ctx.id_of(operand)));
                }
            });
        } else {
            w.field("NOT COMPUTED", "math-expression-shape");
        }
        render_source(w, ctx, obj);
    });
}
